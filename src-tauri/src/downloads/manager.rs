use crate::{
    auth::session::SessionService,
    cloud::models::FlightRecord,
    database::Database,
    downloads::{extractor, hashing, progress::DownloadProgress},
    errors::{Error, Result},
    events::{emit, EventSink},
};
use futures_util::StreamExt;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncWriteExt,
    sync::{Notify, Semaphore},
};
use tokio_util::sync::CancellationToken;

struct Job {
    progress: DownloadProgress,
    cancel: CancellationToken,
}
pub struct DownloadManager {
    pub session: Arc<SessionService>,
    pub db: Arc<Database>,
    sink: Arc<dyn EventSink>,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    semaphore: RwLock<Arc<Semaphore>>,
    changed: Notify,
}
impl DownloadManager {
    pub fn new(
        session: Arc<SessionService>,
        db: Arc<Database>,
        sink: Arc<dyn EventSink>,
        concurrency: usize,
    ) -> Self {
        Self {
            session,
            db,
            sink,
            jobs: Arc::new(Mutex::new(HashMap::new())),
            semaphore: RwLock::new(Arc::new(Semaphore::new(concurrency.clamp(1, 4)))),
            changed: Notify::new(),
        }
    }
    pub fn active(&self) -> bool {
        self.jobs
            .lock()
            .unwrap()
            .values()
            .any(|j| !j.progress.terminal())
    }
    pub fn active_batches(&self) -> Vec<DownloadProgress> {
        let jobs = self.jobs.lock().unwrap();
        let batches: std::collections::HashSet<_> = jobs
            .values()
            .filter(|j| !j.progress.terminal())
            .map(|j| j.progress.batch_id.as_str())
            .collect();
        jobs.values()
            .filter(|j| batches.contains(j.progress.batch_id.as_str()))
            .map(|j| j.progress.clone())
            .collect()
    }
    pub fn configure(&self, concurrency: usize) -> Result<()> {
        if self.active() {
            return Err(Error::Invalid(
                "Wait for downloads to finish before changing concurrency.".into(),
            ));
        }
        *self.semaphore.write().unwrap() = Arc::new(Semaphore::new(concurrency.clamp(1, 4)));
        Ok(())
    }
    fn publish(&self, p: &DownloadProgress, event: &str) -> Result<()> {
        self.db.attempt(p)?;
        if let Some(j) = self.jobs.lock().unwrap().get_mut(&p.id) {
            j.progress = p.clone();
        }
        emit(self.sink.as_ref(), event, p);
        self.changed.notify_waiters();
        Ok(())
    }
    pub fn enqueue(
        self: &Arc<Self>,
        records: Vec<FlightRecord>,
        folder: PathBuf,
        sync_run: Option<String>,
    ) -> Result<Vec<String>> {
        hashing::validate_folder(&folder)?;
        let mut ids = vec![];
        let batch_id = sync_run
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        for record in records {
            let p = DownloadProgress {
                id: uuid::Uuid::new_v4().to_string(),
                batch_id: batch_id.clone(),
                record_id: record.id.clone(),
                sync_run_id: sync_run.clone(),
                filename: hashing::safe_filename(&record.original_filename),
                state: "Queued".into(),
                ..Default::default()
            };
            let cancel = CancellationToken::new();
            {
                let mut jobs = self.jobs.lock().unwrap();
                if let Some(job) = jobs
                    .values()
                    .find(|j| j.progress.record_id == record.id && !j.progress.terminal())
                {
                    ids.push(job.progress.id.clone());
                    continue;
                }
                self.db.attempt(&p)?;
                jobs.insert(
                    p.id.clone(),
                    Job {
                        progress: p.clone(),
                        cancel: cancel.clone(),
                    },
                );
            }
            ids.push(p.id.clone());
            emit(self.sink.as_ref(), "download:queued", &p);
            let manager = self.clone();
            let folder = folder.clone();
            tokio::spawn(async move {
                manager.run(record, folder, p, cancel).await;
            });
        }
        Ok(ids)
    }
    async fn run(
        self: Arc<Self>,
        record: FlightRecord,
        folder: PathBuf,
        mut p: DownloadProgress,
        cancel: CancellationToken,
    ) {
        let semaphore = self.semaphore.read().unwrap().clone();
        let permit =
            tokio::select! {_=cancel.cancelled()=>None,p=semaphore.acquire_owned()=>p.ok()};
        let outcome = if let Some(_permit) = permit {
            let mut retries = 0;
            loop {
                if cancel.is_cancelled() {
                    break Err(Error::Cancelled);
                }
                p.attempt = retries + 1;
                p.state = "Connecting".into();
                p.error = None;
                p.downloaded_bytes = 0;
                p.total_bytes = None;
                p.percentage = None;
                p.speed = 0.;
                if let Err(e) = self.publish(&p, "download:started") {
                    break Err(e);
                }
                match self.transfer(&record, &folder, &mut p, &cancel).await {
                    Ok(()) => break Ok(()),
                    Err(Error::Expired) => {
                        if self.session.snapshot().state
                            == crate::auth::session::SessionState::Connected
                        {
                            continue;
                        }
                        p.state = "Paused".into();
                        p.speed = 0.;
                        p.error = Some(Error::Expired.to_string());
                        let _ = self.publish(&p, "download:progress");
                        if let Err(e) = self.session.wait_connected(&cancel).await {
                            break Err(e);
                        }
                        continue;
                    }
                    Err(e @ (Error::Offline | Error::RateLimited)) if retries < 2 => {
                        retries += 1;
                        p.state = "Retrying".into();
                        p.speed = 0.;
                        p.error = Some(e.to_string());
                        let _ = self.publish(&p, "download:progress");
                        let delay = if matches!(e, Error::RateLimited) {
                            5u64
                        } else {
                            1
                        } * 2u64.pow(retries as u32 - 1);
                        tokio::select! {_=cancel.cancelled()=>break Err(Error::Cancelled),_=tokio::time::sleep(Duration::from_secs(delay))=>{}}
                    }
                    Err(e) => break Err(e),
                }
            }
        } else {
            Err(Error::Cancelled)
        };
        p.speed = 0.;
        p.eta_seconds = None;
        let event = match outcome {
            Ok(()) => {
                p.state = "Completed".into();
                p.percentage = Some(100.);
                p.error = None;
                "download:completed"
            }
            Err(Error::Cancelled) => {
                p.state = "Cancelled".into();
                p.error = None;
                "download:cancelled"
            }
            Err(e) => {
                p.state = "Failed".into();
                p.error = Some(e.to_string());
                "download:failed"
            }
        };
        // If persistence fails, never announce a completed record.
        if let Err(e) = self.publish(&p, event) {
            p.state = "Failed".into();
            p.error = Some(e.to_string());
            if let Some(j) = self.jobs.lock().unwrap().get_mut(&p.id) {
                j.progress = p.clone();
            }
            emit(self.sink.as_ref(), "download:failed", &p);
            self.changed.notify_waiters();
        }
    }
    async fn transfer(
        &self,
        record: &FlightRecord,
        folder: &std::path::Path,
        p: &mut DownloadProgress,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let (_, token) = self.session.credentials()?;
        let response = tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),r=self.session.cloud.download(&token,&record.original_filename)=>match r {Err(Error::Expired)=>{self.session.reject_token(&token).await?;return Err(Error::Expired);},other=>other?}};
        let total = response.content_length();
        if total.is_some_and(|n| n > extractor::MAX_FILE_BYTES) {
            return Err(Error::Invalid("Download exceeds the 512 MiB limit.".into()));
        }
        let part = tempfile::Builder::new()
            .prefix(".drone-cloud-sync-")
            .suffix(".part")
            .tempfile_in(folder)?;
        let mut file = tokio::fs::File::from_std(part.reopen()?);
        let mut stream = response.bytes_stream();
        let started = Instant::now();
        let mut last = Instant::now();
        let mut bytes = 0;
        p.state = "Downloading".into();
        p.update_bytes(0, total, 0.);
        self.publish(p, "download:progress")?;
        loop {
            let chunk = tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),c=stream.next()=>c};
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = chunk?;
            bytes += chunk.len() as u64;
            if bytes > extractor::MAX_FILE_BYTES {
                return Err(Error::Invalid("Download exceeds the 512 MiB limit.".into()));
            }
            file.write_all(&chunk).await?;
            if last.elapsed() >= Duration::from_millis(125) {
                p.update_bytes(bytes, total, started.elapsed().as_secs_f64());
                self.publish(p, "download:progress")?;
                last = Instant::now();
            }
        }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        if total.is_some_and(|n| n != bytes) {
            return Err(Error::Offline);
        }
        p.update_bytes(bytes, total, started.elapsed().as_secs_f64());
        p.state = "Extracting".into();
        p.speed = 0.;
        p.eta_seconds = None;
        self.publish(p, "download:extracting")?;
        let path = part.path().to_owned();
        let folder = folder.to_owned();
        let cancel = cancel.clone();
        let record = record.clone();
        let db = self.db.clone();
        let sink = self.sink.clone();
        let mut verifying = p.clone();
        let jobs = self.jobs.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let (output, format) = extractor::extract(&path, &folder, &cancel)?;
            verifying.state = "Verifying".into();
            if let Some(j) = jobs.lock().unwrap().get_mut(&verifying.id) {
                j.progress = verifying.clone();
            }
            emit(sink.as_ref(), "download:verifying", &verifying);
            db.attempt(&verifying)?;
            let mut local = hashing::inspect(output.path(), &cancel, &format)?;
            let (_, payload_md5, _) = hashing::hashes(&path, &cancel)?;
            let kind = match &record.server_md5 {
                Some(md5) if *md5 == local.md5 => "content",
                Some(md5) if *md5 == payload_md5 => "payload",
                _ => "unknown",
            };
            if record.server_hash_kind == "content" && kind != "content" {
                return Err(Error::Invalid(
                    "Flight-log content did not match its verified server MD5.".into(),
                ));
            }
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let final_path =
                extractor::commit(output, &folder, &record.original_filename, &record.id)?;
            local.absolute_path = final_path.canonicalize()?.to_string_lossy().into();
            local.filename = final_path.file_name().unwrap().to_string_lossy().into();
            let local_id = db.index_file(&local)?;
            db.link(&record.id, &local_id, &local.sha256, "download-verified")?;
            db.hash_kind(&record.id, kind)?;
            crate::sync::manifest::write(&db, &folder)?;
            Ok(())
        })
        .await
        .map_err(|_| Error::Invalid("Extraction worker stopped unexpectedly".into()))??;
        Ok(())
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        if let Some(j) = self.jobs.lock().unwrap().get(id) {
            j.cancel.cancel();
            Ok(())
        } else {
            Err(Error::Invalid("Download is no longer active".into()))
        }
    }
    pub fn cancel_all(&self) {
        for j in self.jobs.lock().unwrap().values() {
            if !j.progress.terminal() {
                j.cancel.cancel();
            }
        }
    }
    pub fn progress(&self, ids: &[String]) -> Vec<DownloadProgress> {
        let jobs = self.jobs.lock().unwrap();
        ids.iter()
            .filter_map(|id| jobs.get(id).map(|j| j.progress.clone()))
            .collect()
    }
    pub async fn wait(
        &self,
        ids: &[String],
        cancel: &CancellationToken,
    ) -> Result<Vec<DownloadProgress>> {
        loop {
            let notified = self.changed.notified();
            let jobs = self.progress(ids);
            if jobs.len() == ids.len() && jobs.iter().all(DownloadProgress::terminal) {
                return Ok(jobs);
            }
            tokio::select! {_=cancel.cancelled()=>{for id in ids{let _=self.cancel(id);}return Err(Error::Cancelled);},_=notified=>{}}
        }
    }
}
