use crate::{
    cloud::client::CloudClient,
    database::Database,
    errors::{Error, Result},
    events::{emit, EventSink},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

pub trait TokenVault: Send + Sync {
    fn read(&self) -> Result<Option<Zeroizing<String>>>;
    fn write(&self, token: &str) -> Result<()>;
    fn delete(&self) -> Result<()>;
}
#[cfg(feature = "native-vault")]
pub struct NativeVault;
#[cfg(feature = "native-vault")]
impl NativeVault {
    fn entry(&self) -> Result<keyring::Entry> {
        keyring::Entry::new("drone-cloud-sync", "cloud-session").map_err(|_| Error::Vault)
    }
}
#[cfg(feature = "native-vault")]
impl TokenVault for NativeVault {
    fn read(&self) -> Result<Option<Zeroizing<String>>> {
        match self.entry()?.get_password() {
            Ok(s) => Ok(Some(Zeroizing::new(s))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(Error::Vault),
        }
    }
    fn write(&self, token: &str) -> Result<()> {
        self.entry()?.set_password(token).map_err(|_| Error::Vault)
    }
    fn delete(&self) -> Result<()> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(Error::Vault),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Checking,
    Connected,
    Disconnected,
    Expired,
    Offline,
    Authenticating,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub state: SessionState,
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub message: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct VaultSession {
    email: String,
    token: String,
}
impl Drop for VaultSession {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.token.zeroize();
    }
}
struct Session {
    snapshot: SessionSnapshot,
    token: Option<Zeroizing<String>>,
}
pub struct SessionService {
    pub cloud: Arc<CloudClient>,
    pub db: Arc<Database>,
    vault: Arc<dyn TokenVault>,
    sink: Arc<dyn EventSink>,
    state: Mutex<Session>,
    gate: AsyncMutex<()>,
    changed: Notify,
}
impl SessionService {
    pub fn new(
        cloud: Arc<CloudClient>,
        db: Arc<Database>,
        vault: Arc<dyn TokenVault>,
        sink: Arc<dyn EventSink>,
    ) -> Result<Self> {
        let email = db.setting("email")?;
        let account_id = db.setting("accountId")?;
        Ok(Self {
            cloud,
            db,
            vault,
            sink,
            gate: AsyncMutex::new(()),
            changed: Notify::new(),
            state: Mutex::new(Session {
                snapshot: SessionSnapshot {
                    state: SessionState::Checking,
                    email,
                    account_id,
                    message: None,
                },
                token: None,
            }),
        })
    }
    pub fn snapshot(&self) -> SessionSnapshot {
        self.state.lock().expect("session mutex").snapshot.clone()
    }
    fn set(&self, state: SessionState, message: Option<String>) {
        let snapshot = {
            let mut s = self.state.lock().expect("session mutex");
            s.snapshot.state = state;
            s.snapshot.message = message;
            s.snapshot.clone()
        };
        emit(self.sink.as_ref(), "session:changed", &snapshot);
        self.changed.notify_waiters();
    }
    pub fn credentials(&self) -> Result<(String, Zeroizing<String>)> {
        let s = self.state.lock().expect("session mutex");
        if s.snapshot.state != SessionState::Connected {
            return Err(
                if s.snapshot.state == SessionState::Expired
                    || s.snapshot.state == SessionState::Disconnected
                {
                    Error::Expired
                } else {
                    Error::Offline
                },
            );
        }
        Ok((
            s.snapshot.account_id.clone().ok_or(Error::Expired)?,
            s.token.clone().ok_or(Error::Expired)?,
        ))
    }
    pub async fn wait_connected(&self, cancel: &CancellationToken) -> Result<()> {
        loop {
            let notified = self.changed.notified();
            if self.snapshot().state == SessionState::Connected {
                return Ok(());
            }
            tokio::select! { _=cancel.cancelled()=>return Err(Error::Cancelled), _=notified=>{} }
        }
    }
    pub async fn validate_stored(&self) -> Result<SessionSnapshot> {
        let _gate = self.gate.lock().await;
        self.set(SessionState::Checking, None);
        let vault = self.vault.clone();
        let stored = tokio::task::spawn_blocking(move || vault.read())
            .await
            .map_err(|_| Error::Vault)?;
        let mut stored = match stored {
            Ok(s) => s,
            Err(e) => {
                self.set(SessionState::Disconnected, Some(e.to_string()));
                return Err(e);
            }
        };
        if let Some(secret) = stored.as_ref() {
            if let Ok(bundle) = serde_json::from_str::<VaultSession>(secret) {
                let account = self.db.account(&bundle.email)?;
                {
                    let mut s = self.state.lock().expect("session mutex");
                    s.snapshot.email = Some(bundle.email.clone());
                    s.snapshot.account_id = Some(account);
                }
                stored = Some(Zeroizing::new(bundle.token.clone()));
            }
        }
        if self.state.lock().expect("session mutex").token.is_none() {
            self.state.lock().expect("session mutex").token = stored;
        }
        let token = self.state.lock().expect("session mutex").token.clone();
        match token {
            None => self.set(SessionState::Disconnected, None),
            Some(token) => match self.cloud.validate(&token).await {
                Ok(()) => {
                    let account = self.snapshot().account_id;
                    if let Some(id) = account {
                        self.db.validated(&id)?;
                        self.set(SessionState::Connected, None);
                    } else {
                        self.expire_inner().await?;
                    }
                }
                Err(Error::Expired) => self.expire_inner().await?,
                Err(e) => self.set(SessionState::Offline, Some(e.to_string())),
            },
        }
        Ok(self.snapshot())
    }
    pub async fn validate_current(&self) -> Result<()> {
        let (_, token) = self.credentials()?;
        match self.cloud.validate(&token).await {
            Err(Error::Expired) => {
                self.reject_token(&token).await?;
                Err(Error::Expired)
            }
            result => result,
        }
    }
    pub async fn login(
        &self,
        email: String,
        password: Zeroizing<String>,
        challenge_id: &str,
        answer: &str,
        remember: bool,
    ) -> Result<SessionSnapshot> {
        let _gate = self.gate.lock().await;
        self.set(SessionState::Authenticating, None);
        let result = self
            .cloud
            .login(email.trim(), &password, challenge_id, answer)
            .await;
        drop(password);
        let token = match result {
            Ok(token) => token,
            Err(e) => {
                self.set(SessionState::Disconnected, Some(e.to_string()));
                return Err(e);
            }
        };
        if let Err(e) = self.cloud.validate(&token).await {
            self.set(SessionState::Disconnected, Some(e.to_string()));
            return Err(e);
        }
        let vault = self.vault.clone();
        let save = Zeroizing::new(
            serde_json::to_string(&VaultSession {
                email: email.trim().to_lowercase(),
                token: token.to_string(),
            })
            .map_err(|_| Error::Vault)?,
        );
        let persisted = tokio::task::spawn_blocking(move || {
            if remember {
                vault.write(&save)
            } else {
                vault.delete()
            }
        })
        .await
        .map_err(|_| Error::Vault)?;
        if let Err(e) = persisted {
            self.set(SessionState::Disconnected, Some(e.to_string()));
            return Err(e);
        }
        let account = self.db.account(&email)?;
        self.db.validated(&account)?;
        {
            let mut s = self.state.lock().expect("session mutex");
            s.token = Some(token);
            s.snapshot.email = Some(email.trim().to_lowercase());
            s.snapshot.account_id = Some(account);
        }
        self.set(SessionState::Connected, None);
        Ok(self.snapshot())
    }
    async fn expire_inner(&self) -> Result<()> {
        self.state.lock().expect("session mutex").token = None;
        self.set(SessionState::Expired, Some(Error::Expired.to_string()));
        let vault = self.vault.clone();
        tokio::task::spawn_blocking(move || vault.delete())
            .await
            .map_err(|_| Error::Vault)??;
        Ok(())
    }
    pub async fn reject_token(&self, rejected: &str) -> Result<()> {
        let _gate = self.gate.lock().await;
        let current = self.state.lock().expect("session mutex").token.clone();
        if current.as_deref().map(|s| s.as_str()) == Some(rejected) {
            self.expire_inner().await?;
        }
        Ok(())
    }
    pub async fn expire(&self) -> Result<()> {
        let _gate = self.gate.lock().await;
        self.expire_inner().await
    }
    pub async fn logout(&self) -> Result<()> {
        let _gate = self.gate.lock().await;
        self.state.lock().expect("session mutex").token = None;
        self.set(SessionState::Disconnected, None);
        let vault = self.vault.clone();
        tokio::task::spawn_blocking(move || vault.delete())
            .await
            .map_err(|_| Error::Vault)??;
        Ok(())
    }
}
