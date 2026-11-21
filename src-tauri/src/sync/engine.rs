use crate::{
    auth::session::{SessionService, SessionState},
    cloud::{
        models::normalize,
        pagination::{Pagination, PAGE_SIZE},
    },
    database::Database,
    downloads::manager::DownloadManager,
    errors::{Error, Result},
    events::{emit, EventSink},
    sync::reconciliation,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryProgress {
    pub page: usize,
    pub records: usize,
    pub state: String,
    pub error: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub id: String,
    pub stage: String,
    pub total_records: usize,
    pub already_present: usize,
    pub downloaded: usize,
    pub failed: usize,
    pub missing: usize,
    pub scanned: usize,
    pub error: Option<String>,
}
pub struct SyncEngine {
    pub session: Arc<SessionService>,
    pub db: Arc<Database>,
    pub downloads: Arc<DownloadManager>,
    pub sink: Arc<dyn EventSink>,
    state: Mutex<SyncProgress>,
    cancel: Mutex<Option<CancellationToken>>,
    inventory: tokio::sync::Mutex<()>,
    inventory_cancel: Mutex<Option<CancellationToken>>,
}
impl SyncEngine {
    pub fn new(
        session: Arc<SessionService>,
        db: Arc<Database>,
        downloads: Arc<DownloadManager>,
        sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            session,
            db,
            downloads,
            sink,
            state: Mutex::new(SyncProgress::default()),
            cancel: Mutex::new(None),
            inventory: tokio::sync::Mutex::new(()),
            inventory_cancel: Mutex::new(None),
        }
    }
    pub fn status(&self) -> SyncProgress {
        self.state.lock().unwrap().clone()
    }
    pub fn active(&self) -> bool {
        self.cancel.lock().unwrap().is_some()
    }
    fn publish(&self, p: &SyncProgress, event: &str) {
        *self.state.lock().unwrap() = p.clone();
        emit(self.sink.as_ref(), event, p);
    }
    pub fn cancel(&self) {
        if let Some(c) = self.cancel.lock().unwrap().as_ref() {
            c.cancel();
        }
    }
    pub fn cancel_inventory(&self) {
        if let Some(c) = self.inventory_cancel.lock().unwrap().as_ref() {
            c.cancel();
        }
    }
    pub async fn fetch_inventory(
        &self,
        cancel: CancellationToken,
        resume_auth: bool,
    ) -> Result<usize> {
        let _lock = tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),l=self.inventory.lock()=>l};
        *self.inventory_cancel.lock().unwrap() = Some(cancel.clone());
        let result = self.fetch_pages(&cancel, resume_auth).await;
        *self.inventory_cancel.lock().unwrap() = None;
        if let Err(e) = &result {
            emit(
                self.sink.as_ref(),
                "inventory:progress",
                &InventoryProgress {
                    state: if matches!(e, Error::Cancelled) {
                        "Cancelled"
                    } else {
                        "Partial failure"
                    }
                    .into(),
                    error: Some(e.to_string()),
                    ..Default::default()
                },
            );
        }
        result
    }
    async fn fetch_pages(&self, cancel: &CancellationToken, resume_auth: bool) -> Result<usize> {
        let (account, _) = self.session.credentials()?;
        let mut paging = Pagination::default();
        for page in 1..=1000 {
            let mut retries = 0;
            let payload = loop {
                let (_, token) = self.session.credentials()?;
                let result = tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),r=self.session.cloud.page(&token,page,PAGE_SIZE)=>r};
                match result {
                    Ok(payload) => break payload,
                    Err(Error::Expired) => {
                        self.session.reject_token(&token).await?;
                        if !resume_auth {
                            return Err(Error::Expired);
                        }
                        self.session.wait_connected(cancel).await?;
                    }
                    Err(e @ (Error::Offline | Error::RateLimited)) if retries < 2 => {
                        retries += 1;
                        let delay = if matches!(e, Error::RateLimited) {
                            5
                        } else {
                            1
                        } * 2u64.pow(retries - 1);
                        tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),_=tokio::time::sleep(Duration::from_secs(delay))=>{}}
                    }
                    Err(e) => return Err(e),
                }
            };
            let batch = crate::cloud::pagination::record_list(&payload)?;
            let mut normalized = vec![];
            for entry in batch {
                let r = normalize(entry.clone(), &account).ok_or_else(|| {
                    Error::Schema("A record has no filename; inventory is incomplete".into())
                })?;
                if paging.seen.insert(r.id.clone()) {
                    normalized.push(r);
                }
            }
            self.db.upsert_records(&account, &normalized)?;
            paging.total += normalized.len();
            paging.pages = page;
            emit(
                self.sink.as_ref(),
                "inventory:progress",
                &InventoryProgress {
                    page,
                    records: paging.total,
                    state: "Fetching".into(),
                    error: None,
                },
            );
            if paging.finished(&payload, batch.len(), page) {
                emit(
                    self.sink.as_ref(),
                    "inventory:progress",
                    &InventoryProgress {
                        page,
                        records: paging.total,
                        state: "Completed".into(),
                        error: None,
                    },
                );
                return Ok(paging.total);
            }
            if normalized.is_empty() {
                return Err(Error::Schema("Pagination repeated a page or returned an empty page before its declared end. Partial inventory was saved.".into()));
            }
        }
        Err(Error::Schema(
            "Inventory reached its 1000-page safety limit. Partial records were saved.".into(),
        ))
    }
    pub fn start(self: &Arc<Self>, folder: PathBuf) -> Result<String> {
        let (account, _) = self.session.credentials()?;
        crate::downloads::hashing::validate_folder(&folder)?;
        if self.downloads.active() {
            return Err(Error::Invalid(
                "Wait for manual downloads to finish before starting synchronization.".into(),
            ));
        }
        let cancel = CancellationToken::new();
        {
            let mut active = self.cancel.lock().unwrap();
            if active.is_some() {
                return Err(Error::Invalid("Synchronization is already running".into()));
            }
            *active = Some(cancel.clone());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let stored = self.db.with(|c| {
            c.execute(
                "INSERT INTO sync_runs(id,account_id,status) VALUES (?,?,'Running')",
                rusqlite::params![id, account],
            )?;
            Ok(())
        });
        if let Err(e) = stored {
            *self.cancel.lock().unwrap() = None;
            return Err(e);
        }
        let p = SyncProgress {
            id: id.clone(),
            stage: "Validating session".into(),
            ..Default::default()
        };
        self.publish(&p, "sync:started");
        let engine = self.clone();
        tokio::spawn(async move {
            engine.run(account, folder, p, cancel).await;
        });
        Ok(id)
    }
    async fn run(
        self: Arc<Self>,
        account: String,
        folder: PathBuf,
        mut p: SyncProgress,
        cancel: CancellationToken,
    ) {
        let result = self.work(&account, folder, &mut p, &cancel).await;
        let status = match &result {
            Ok(()) if p.failed == 0 => "Completed",
            Ok(()) => "Completed with failures",
            Err(Error::Cancelled) => "Cancelled",
            Err(_) => "Failed",
        };
        p.stage = status.into();
        if let Err(e) = result {
            p.error = Some(e.to_string());
        }
        let dbresult=self.db.with(|c|{c.execute("UPDATE sync_runs SET finished_at=CURRENT_TIMESTAMP,status=?,total_records=?,already_present=?,downloaded=?,failed=?,error_summary=? WHERE id=?",rusqlite::params![status,p.total_records,p.already_present,p.downloaded,p.failed,p.error,p.id])?;Ok(())});
        if let Err(e) = dbresult {
            p.stage = "Failed".into();
            p.error = Some(e.to_string());
        }
        if p.stage == "Completed" {
            let _ = self
                .db
                .set_setting("lastSuccessfulSync", &chrono::Utc::now().to_rfc3339());
        }
        let event = if p.stage.starts_with("Completed") {
            "sync:completed"
        } else if p.stage == "Cancelled" {
            "sync:cancelled"
        } else {
            "sync:failed"
        };
        self.publish(&p, event);
        *self.cancel.lock().unwrap() = None;
    }
    async fn work(
        &self,
        account: &str,
        folder: PathBuf,
        p: &mut SyncProgress,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let validation = tokio::select! {_=cancel.cancelled()=>return Err(Error::Cancelled),r=self.session.validate_current()=>r};
        if let Err(e) = validation {
            if matches!(e, Error::Expired) {
                p.stage = "Paused — sign in to resume".into();
                self.publish(p, "sync:progress");
                self.session.wait_connected(cancel).await?;
            } else {
                return Err(e);
            }
        }
        p.stage = "Fetching cloud inventory".into();
        self.publish(p, "sync:progress");
        p.total_records = self.fetch_inventory(cancel.clone(), true).await?;
        p.stage = "Scanning local files".into();
        self.publish(p, "sync:progress");
        let db = self.db.clone();
        let account_owned = account.to_owned();
        let scan_cancel = cancel.clone();
        let sink = self.sink.clone();
        let scan_folder = folder.clone();
        let mut scanning = p.clone();
        let scan = tokio::task::spawn_blocking(move || {
            reconciliation::scan(&db, &account_owned, &scan_folder, &scan_cancel, |n| {
                if n % 10 == 0 {
                    scanning.scanned = n;
                    emit(sink.as_ref(), "sync:progress", &scanning);
                }
            })
        })
        .await
        .map_err(|_| Error::Invalid("Folder scanning worker stopped unexpectedly".into()))??;
        p.scanned = scan.scanned;
        p.already_present = scan.present;
        p.missing = scan.missing;
        p.stage = "Comparing records".into();
        self.publish(p, "sync:progress");
        if !scan.errors.is_empty() {
            return Err(Error::Invalid(format!("{} local files could not be verified. Check folder permissions and rescan before syncing.",scan.errors.len())));
        }
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let missing = self
            .db
            .records(account)?
            .into_iter()
            .filter(|r| r.status != "Downloaded")
            .collect();
        let ids = self
            .downloads
            .enqueue(missing, folder, Some(p.id.clone()))?;
        p.stage = "Downloading missing records".into();
        self.publish(p, "sync:progress");
        loop {
            let jobs = self.downloads.progress(&ids);
            p.downloaded = jobs.iter().filter(|j| j.state == "Completed").count();
            p.failed = jobs
                .iter()
                .filter(|j| j.state == "Failed" || j.state == "Cancelled")
                .count();
            if jobs.iter().all(|j| j.terminal()) {
                break;
            }
            p.stage = if self.session.snapshot().state == SessionState::Expired {
                "Paused — sign in to resume"
            } else if jobs.iter().any(|j| j.state == "Verifying") {
                "Verifying downloaded files"
            } else {
                "Downloading missing records"
            }
            .into();
            self.publish(p, "sync:progress");
            tokio::select! {_=cancel.cancelled()=>{for id in &ids{let _=self.downloads.cancel(id);}let settle=CancellationToken::new();let settled=self.downloads.wait(&ids,&settle).await?;p.downloaded=settled.iter().filter(|j|j.state=="Completed").count();p.failed=settled.iter().filter(|j|j.state=="Failed").count();return Err(Error::Cancelled);},_=tokio::time::sleep(Duration::from_millis(200))=>{}}
        }
        Ok(())
    }
}
