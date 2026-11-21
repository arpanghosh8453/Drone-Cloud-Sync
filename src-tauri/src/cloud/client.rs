use crate::{
    auth::{
        captcha::{Captcha, Challenge},
        crypto,
    },
    errors::{Error, Result},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{Client, StatusCode, Url};
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

pub struct CloudClient {
    pub http: Client,
    account: String,
    flight: String,
    challenge: Mutex<Option<Challenge>>,
}
impl CloudClient {
    pub fn production(timeout: u64) -> Result<Self> {
        Self::new(
            "https://account-api.dji.com/apis/apprest/v1",
            "https://mydjiflight.dji.com/flight",
            timeout,
        )
    }
    pub fn new(account: &str, flight: &str, timeout: u64) -> Result<Self> {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        let http = Client::builder()
            .cookie_provider(jar)
            .user_agent("DJICloudResearchClient/1.0")
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(15))
            .referer(false)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            http,
            account: account.into(),
            flight: flight.into(),
            challenge: Mutex::new(None),
        })
    }
    pub async fn request_captcha(&self) -> Result<Captcha> {
        let mut slot = self.challenge.lock().await;
        let id = crate::auth::captcha::random_id();
        let response = self
            .http
            .get(format!("{}/vcode", self.account))
            .query(&[("srandom", &id)])
            .send()
            .await?;
        check_status(response.status())?;
        let mime = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .split(';')
            .next()
            .unwrap_or("image/png")
            .to_string();
        if !["image/png", "image/jpeg", "image/gif", "image/webp"].contains(&mime.as_str()) {
            return Err(Error::Schema("CAPTCHA did not contain an image".into()));
        }
        let bytes = response.bytes().await?;
        if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
            return Err(Error::Schema("CAPTCHA image size is invalid".into()));
        }
        *slot = Some(Challenge {
            id: id.clone(),
            created: Instant::now(),
        });
        Ok(Captcha {
            challenge_id: id,
            image_data_url: format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
        })
    }
    pub async fn login(
        &self,
        email: &str,
        password: &str,
        challenge_id: &str,
        answer: &str,
    ) -> Result<Zeroizing<String>> {
        let code = answer.trim().to_ascii_uppercase();
        if code.len() != 4
            || !code
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Err(Error::Invalid(
                "Enter the four-character verification code.".into(),
            ));
        }
        let mut slot = self.challenge.lock().await;
        let challenge = slot
            .as_ref()
            .ok_or_else(|| Error::Invalid("Request a fresh verification image.".into()))?;
        if challenge.id != challenge_id || challenge.created.elapsed() > Duration::from_secs(300) {
            return Err(Error::Invalid(
                "Verification image expired. Refresh it and try again.".into(),
            ));
        }
        let challenge = slot.take().expect("checked current challenge");
        let encrypted = crypto::encrypt_password(password);
        let mut request = self.http.post(format!("{}/email_login", self.account));
        for (k, v) in crypto::signed_headers() {
            request = request.header(k, v);
        }
        let response = request
            .form(&[
                ("email", email),
                ("password", encrypted.as_str()),
                ("srandom", &challenge.id),
                ("verificationCode", &code),
            ])
            .send()
            .await?;
        let payload = json_response(response).await?;
        check_api_error(&payload)?;
        extract_token(&payload).map(Zeroizing::new).ok_or_else(|| {
            Error::Schema("Login succeeded but no supported token field was returned.".into())
        })
    }
    pub async fn page(&self, token: &str, page: usize, size: usize) -> Result<Value> {
        let response = self
            .http
            .get(format!("{}/record_info_list", self.flight))
            .query(&[
                ("token", token.to_string()),
                ("page", page.to_string()),
                ("page_size", size.to_string()),
            ])
            .send()
            .await?;
        let payload = json_response(response).await?;
        check_api_error(&payload)?;
        Ok(payload)
    }
    pub async fn validate(&self, token: &str) -> Result<()> {
        let data = self.page(token, 1, 1).await?;
        crate::cloud::pagination::record_list(&data)?;
        Ok(())
    }
    pub async fn download(&self, token: &str, filename: &str) -> Result<reqwest::Response> {
        let mut url = Url::parse(&format!("{}/zipdownload", self.flight))
            .map_err(|_| Error::Invalid("Invalid cloud URL".into()))?;
        url.query_pairs_mut()
            .append_pair("token", token)
            .append_pair("filename", filename);
        for hop in 0..=10 {
            let response = self.http.get(url.clone()).send().await?;
            if response.status().is_redirection() {
                if hop == 10 {
                    return Err(Error::Invalid("Too many cloud redirects".into()));
                }
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| Error::Schema("Download redirect has no location".into()))?;
                let next = url
                    .join(location)
                    .map_err(|_| Error::Invalid("Invalid redirect location".into()))?;
                if !allowed_redirect(&next) {
                    return Err(Error::Invalid("Download redirect was rejected: destination is outside the trusted HTTPS hosts.".into()));
                }
                url = next;
                continue;
            }
            check_status(response.status())?;
            let mime = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if mime.contains("json") {
                let payload = json_response(response).await?;
                check_api_error(&payload)?;
                return Err(Error::Schema(
                    "Download returned JSON instead of a flight log".into(),
                ));
            }
            return Ok(response);
        }
        Err(Error::Invalid("Too many redirects".into()))
    }
}
pub fn allowed_redirect(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|host| {
            ["dji.com", "djicdn.com", "djistatic.com", "amazonaws.com"]
                .iter()
                .any(|root| host == *root || host.ends_with(&format!(".{root}")))
        })
}
pub fn check_status(status: StatusCode) -> Result<()> {
    match status.as_u16() {
        401 | 403 => Err(Error::Expired),
        429 => Err(Error::RateLimited),
        500..=599 => Err(Error::Offline),
        200..=299 => Ok(()),
        _ => Err(Error::Api(status.as_u16().to_string())),
    }
}
pub async fn json_response(response: reqwest::Response) -> Result<Value> {
    check_status(response.status())?;
    response
        .json()
        .await
        .map_err(|_| Error::Schema("Expected a JSON object".into()))
}
pub fn check_api_error(data: &Value) -> Result<()> {
    let m = data
        .as_object()
        .ok_or_else(|| Error::Schema("Expected a JSON object".into()))?;
    for key in ["code", "Code"] {
        if let Some(v) = m.get(key) {
            let code = crate::cloud::models::string(v).unwrap_or_default();
            if code != "0" && code != "200" {
                let message =
                    crate::cloud::models::first(m, &["message", "Message", "ErrMsg", "msg"])
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_ascii_lowercase();
                // Only explicit rejection clears a stored session. Do not guess undocumented numeric codes.
                let auth_rejected = (message.contains("token") || message.contains("session"))
                    && ["invalid", "expired", "expire", "unauthoriz", "not valid"]
                        .iter()
                        .any(|s| message.contains(s));
                if code == "401" || code == "403" || auth_rejected {
                    return Err(Error::Expired);
                }
                if code == "429" {
                    return Err(Error::RateLimited);
                }
                let safe_code: String = code
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                    .take(16)
                    .collect();
                return Err(Error::Api(safe_code));
            }
        }
    }
    Ok(())
}
pub fn extract_token(payload: &Value) -> Option<String> {
    let mut nested = vec![];
    crate::cloud::models::objects(payload, &mut nested);
    for item in nested {
        for k in ["token", "Token", "access_token", "accessToken"] {
            if let Some(s) = item.get(k).and_then(Value::as_str) {
                if s.len() > 15 {
                    return Some(s.into());
                }
            }
        }
    }
    None
}
