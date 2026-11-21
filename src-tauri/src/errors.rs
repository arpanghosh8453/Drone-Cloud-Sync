use serde::Serialize;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Your cloud session has expired. Please sign in again.")]
    Expired,
    #[error("Cloud is temporarily unreachable. Your saved session has been retained.")]
    Offline,
    #[error("Cloud request was rejected (code {0}). Check your credentials or verification code.")]
    Api(String),
    #[error("Cloud response structure changed: {0}")]
    Schema(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("{0}")]
    Invalid(String),
    #[error(
        "Secure credential store is unavailable. Unlock your system credential vault and retry."
    )]
    Vault,
    #[error("Local database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("Local file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Cloud is rate limited. Try again later.")]
    RateLimited,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandError {
    kind: &'static str,
    message: String,
}
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let kind = match self {
            Self::Expired => "expired",
            Self::Offline => "offline",
            Self::Cancelled => "cancelled",
            Self::Vault => "vault",
            Self::RateLimited => "rateLimited",
            _ => "error",
        };
        CommandError {
            kind,
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}
impl From<reqwest::Error> for Error {
    // reqwest errors can include the token-bearing URL. Never forward or log them.
    fn from(_: reqwest::Error) -> Self {
        Self::Offline
    }
}
