use crate::{
    auth::{
        captcha::Captcha,
        session::{NativeVault, SessionService, SessionSnapshot},
    },
    cloud::{client::CloudClient, models::FlightRecord},
    database::Database,
    downloads::{hashing, manager::DownloadManager, progress::DownloadProgress},
    errors::{Error, Result},
    events::EventSink,
    sync::{
        engine::{SyncEngine, SyncProgress},
        reconciliation::{self, ScanSummary},
    },
};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tauri::{Emitter, Manager, State};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;
struct DesktopEvents(tauri::AppHandle);
impl EventSink for DesktopEvents {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit_to("main", event, payload);
    }
}
pub struct AppState {
    _instance_lock: std::fs::File,
    folder_gate: tokio::sync::Mutex<()>,
    db: Arc<Database>,
    session: Arc<SessionService>,
    downloads: Arc<DownloadManager>,
    engine: Arc<SyncEngine>,
}
impl AppState {
    fn account(&self) -> Result<String> {
        self.session
            .snapshot()
            .account_id
            .ok_or_else(|| Error::Invalid("Connect your cloud account first.".into()))
    }
    fn folder(&self) -> Result<PathBuf> {
        let path = self.db.setting("outputFolder")?.ok_or_else(|| {
            Error::Invalid("Choose an output folder in Settings before downloading.".into())
        })?;
        hashing::validate_folder(&PathBuf::from(path))
    }
    fn ensure_idle(&self) -> Result<()> {
        if self.downloads.active() || self.engine.active() {
            Err(Error::Invalid(
                "Wait for the current operation to finish, or cancel it first.".into(),
            ))
        } else {
            Ok(())
        }
    }
}
#[tauri::command]
async fn request_captcha(state: State<'_, AppState>) -> Result<Captcha> {
    state.session.cloud.request_captcha().await
}
#[tauri::command]
async fn login(
    state: State<'_, AppState>,
    email: String,
    password: String,
    challenge_id: String,
    captcha: String,
    remember: bool,
) -> Result<SessionSnapshot> {
    let password = Zeroizing::new(password);
    if (state.downloads.active() || state.engine.active())
        && state.session.snapshot().email.as_deref() != Some(&email.trim().to_lowercase())
    {
        return Err(Error::Invalid("Sign in to the same account to resume, or cancel the pending operation before changing accounts.".into()));
    }
    state
        .session
        .login(email, password, &challenge_id, &captcha, remember)
        .await
}
#[tauri::command]
async fn validate_stored_session(state: State<'_, AppState>) -> Result<SessionSnapshot> {
    state.session.validate_stored().await
}
#[tauri::command]
async fn restore_session(state: State<'_, AppState>) -> Result<SessionSnapshot> {
    state.session.validate_stored().await
}
#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<()> {
    state.engine.cancel();
    state.engine.cancel_inventory();
    state.downloads.cancel_all();
    while state.downloads.active() || state.engine.active() {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    state.session.logout().await
}
#[tauri::command]
async fn fetch_flight_records(state: State<'_, AppState>) -> Result<usize> {
    if state.engine.active() {
        return Err(Error::Invalid(
            "Cloud inventory is managed by the active synchronization.".into(),
        ));
    }
    state
        .engine
        .fetch_inventory(CancellationToken::new(), false)
        .await
}
#[tauri::command]
fn cancel_inventory(state: State<'_, AppState>) {
    state.engine.cancel_inventory();
}
#[tauri::command]
async fn get_cached_flight_records(state: State<'_, AppState>) -> Result<Vec<FlightRecord>> {
    let db = state.db.clone();
    let account = state.account()?;
    tokio::task::spawn_blocking(move || db.records(&account))
        .await
        .map_err(|_| Error::Invalid("Database worker stopped".into()))?
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FlightDetails {
    record: FlightRecord,
    history: Vec<DownloadProgress>,
}
#[tauri::command]
async fn get_flight_details(
    state: State<'_, AppState>,
    record_id: String,
) -> Result<FlightDetails> {
    let account = state.account()?;
    Ok(FlightDetails {
        record: state.db.record(&account, &record_id)?,
        history: state.db.history(&account, Some(&record_id))?,
    })
}
#[tauri::command]
async fn select_output_folder(state: State<'_, AppState>) -> Result<Option<String>> {
    let _folder_guard = state.folder_gate.lock().await;
    state.ensure_idle()?;
    let mut picker =
        rfd::AsyncFileDialog::new().set_title("drone-cloud-sync — Choose output folder");
    if let Some(folder) = state.db.setting("outputFolder")? {
        picker = picker.set_directory(folder);
    }
    if let Some(folder) = picker.pick_folder().await {
        state.ensure_idle()?;
        let folder = hashing::validate_folder(folder.path())?;
        let path = folder.to_string_lossy().to_string();
        state.db.set_setting("outputFolder", &path)?;
        state.db.invalidate_files()?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}
#[tauri::command]
fn get_output_folder(state: State<'_, AppState>) -> Result<Option<String>> {
    state.db.setting("outputFolder")
}
#[tauri::command]
async fn scan_output_folder(state: State<'_, AppState>) -> Result<ScanSummary> {
    let _folder_guard = state.folder_gate.lock().await;
    state.ensure_idle()?;
    let db = state.db.clone();
    let account = state.account()?;
    let folder = state.folder()?;
    tokio::task::spawn_blocking(move || {
        reconciliation::scan(&db, &account, &folder, &CancellationToken::new(), |_| {})
    })
    .await
    .map_err(|_| Error::Invalid("Folder scan stopped".into()))?
}
#[tauri::command]
async fn download_selected(
    state: State<'_, AppState>,
    record_ids: Vec<String>,
    force: Option<bool>,
) -> Result<Vec<String>> {
    let _folder_guard = state.folder_gate.lock().await;
    if state.engine.active() {
        return Err(Error::Invalid(
            "Wait for synchronization to finish before starting a manual download.".into(),
        ));
    }
    state.session.credentials()?;
    let folder = state.folder()?;
    let account = state.account()?;
    // Verify existing associations immediately before skipping a requested record.
    let db = state.db.clone();
    let scan_folder = folder.clone();
    let scan_account = account.clone();
    tokio::task::spawn_blocking(move || {
        reconciliation::scan(
            &db,
            &scan_account,
            &scan_folder,
            &CancellationToken::new(),
            |_| {},
        )
    })
    .await
    .map_err(|_| Error::Invalid("Folder scan stopped".into()))??;
    let records = record_ids
        .iter()
        .map(|id| state.db.record(&account, id))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|r| force == Some(true) || r.status != "Downloaded")
        .collect();
    state.downloads.enqueue(records, folder, None)
}
#[tauri::command]
fn cancel_download(state: State<'_, AppState>, download_id: String) -> Result<()> {
    state.downloads.cancel(&download_id)
}
#[tauri::command]
fn cancel_all_downloads(state: State<'_, AppState>) {
    state.engine.cancel();
    state.downloads.cancel_all();
}
#[tauri::command]
async fn retry_failed_downloads(state: State<'_, AppState>) -> Result<Vec<String>> {
    let account = state.account()?;
    let records = state
        .db
        .history(&account, None)?
        .into_iter()
        .filter(|p| p.state == "Failed" || p.state == "Cancelled")
        .map(|p| p.record_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    download_selected(state, records, Some(false)).await
}
#[tauri::command]
fn get_download_history(state: State<'_, AppState>) -> Result<Vec<DownloadProgress>> {
    if let Some(account) = state.session.snapshot().account_id {
        let mut history = state.db.history(&account, None)?;
        for current in state.downloads.active_batches() {
            if let Some(previous) = history.iter_mut().find(|p| p.id == current.id) {
                *previous = current;
            } else {
                history.push(current);
            }
        }
        Ok(history)
    } else {
        Ok(vec![])
    }
}
#[tauri::command]
fn clear_completed_history(state: State<'_, AppState>) -> Result<()> {
    state.ensure_idle()?;
    state.db.clear_completed(&state.account()?)
}
#[tauri::command]
async fn start_sync(state: State<'_, AppState>) -> Result<String> {
    let _folder_guard = state.folder_gate.lock().await;
    state.engine.start(state.folder()?)
}
#[tauri::command]
fn cancel_sync(state: State<'_, AppState>) {
    state.engine.cancel();
}
#[tauri::command]
fn get_sync_status(state: State<'_, AppState>) -> SyncProgress {
    state.engine.status()
}
#[tauri::command]
fn get_sync_history(state: State<'_, AppState>) -> Result<Vec<Value>> {
    state.db.sync_history(&state.account()?)
}
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<BTreeMap<String, String>> {
    let mut settings = state.db.settings()?;
    if settings
        .get("outputFolder")
        .is_some_and(|folder| !std::path::Path::new(folder).is_dir())
    {
        settings.insert(
            "outputFolderWarning".into(),
            "Your saved output folder is unavailable. Select a valid directory in Settings.".into(),
        );
    }
    Ok(settings)
}
#[tauri::command]
fn update_setting(state: State<'_, AppState>, key: String, value: String) -> Result<()> {
    match key.as_str() {
        "theme" if ["light", "dark", "system"].contains(&value.as_str()) => {}
        "downloadConcurrency" => {
            let n = value
                .parse::<usize>()
                .map_err(|_| Error::Invalid("Choose 1–4 concurrent downloads".into()))?;
            if !(1..=4).contains(&n) {
                return Err(Error::Invalid("Choose 1–4 concurrent downloads".into()));
            }
            state.downloads.configure(n)?;
        }
        "networkTimeout" if value.parse::<u64>().is_ok_and(|n| (10..=300).contains(&n)) => {}
        "tablePreferences"
            if value.len() < 16_384 && serde_json::from_str::<Value>(&value).is_ok() => {}
        _ => return Err(Error::Invalid("Unsupported setting or value".into())),
    }
    state.db.set_setting(&key, &value)
}
#[tauri::command]
async fn open_output_folder(state: State<'_, AppState>, record_id: Option<String>) -> Result<()> {
    let folder = if let Some(id) = record_id {
        let record = state.db.record(&state.account()?, &id)?;
        let path = record
            .local_path
            .ok_or_else(|| Error::Invalid("This record has no local file".into()))?;
        let path = PathBuf::from(path);
        path.parent()
            .ok_or_else(|| Error::Invalid("Local folder is unavailable".into()))?
            .to_path_buf()
    } else {
        state.folder()?
    };
    let folder = hashing::validate_folder(&folder)?;
    tokio::task::spawn_blocking(move || open::that(folder).map_err(Error::Io))
        .await
        .map_err(|_| Error::Invalid("Could not open folder".into()))?
}
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let instance_lock = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(dir.join(".drone-cloud-sync.lock"))?;
            if instance_lock.try_lock().is_err() {
                rfd::MessageDialog::new()
                    .set_title("drone-cloud-sync")
                    .set_description(
                        "drone-cloud-sync is already running. Open its existing window.",
                    )
                    .show();
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "drone-cloud-sync is already running",
                )
                .into());
            }
            let db = Arc::new(Database::open(&dir.join("drone-cloud-sync.sqlite"))?);
            let timeout = db
                .setting("networkTimeout")?
                .and_then(|n| n.parse().ok())
                .unwrap_or(30);
            let concurrency = db
                .setting("downloadConcurrency")?
                .and_then(|n| n.parse().ok())
                .unwrap_or(2);
            let sink: Arc<dyn EventSink> = Arc::new(DesktopEvents(app.handle().clone()));
            let cloud = Arc::new(CloudClient::production(timeout)?);
            let session = Arc::new(SessionService::new(
                cloud,
                db.clone(),
                Arc::new(NativeVault),
                sink.clone(),
            )?);
            let downloads = Arc::new(DownloadManager::new(
                session.clone(),
                db.clone(),
                sink.clone(),
                concurrency,
            ));
            let engine = Arc::new(SyncEngine::new(
                session.clone(),
                db.clone(),
                downloads.clone(),
                sink,
            ));
            app.manage(AppState {
                _instance_lock: instance_lock,
                folder_gate: tokio::sync::Mutex::new(()),
                db,
                session,
                downloads,
                engine,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            request_captcha,
            login,
            validate_stored_session,
            restore_session,
            logout,
            fetch_flight_records,
            cancel_inventory,
            get_cached_flight_records,
            get_flight_details,
            select_output_folder,
            get_output_folder,
            scan_output_folder,
            download_selected,
            cancel_download,
            cancel_all_downloads,
            retry_failed_downloads,
            get_download_history,
            clear_completed_history,
            start_sync,
            cancel_sync,
            get_sync_status,
            get_sync_history,
            get_settings,
            update_setting,
            open_output_folder
        ])
        .build(tauri::generate_context!())
        .expect("Unable to initialize drone-cloud-sync")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let state = app.state::<AppState>();
                state.engine.cancel();
                state.engine.cancel_inventory();
                state.downloads.cancel_all();
            }
        });
}
