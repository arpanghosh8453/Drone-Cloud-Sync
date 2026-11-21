use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::BTreeMap;
use zeroize::Zeroizing;
const AES_KEY: &[u8; 16] = b"221edf0e-edf7-42";
const AES_IV: &[u8; 16] = b"1wdv$ESZ@QAX3rfc";
const MAC_KEY: &[u8] = b"221edf0e-edf7-42b5-a451-776a5872ae4c";
pub const SIGNED_FIELDS: [&str; 5] = [
    "AppId-Mc",
    "ClientName-Mc",
    "DeviceId-Mc",
    "InvokeId-Mc",
    "Timestamp-Mc",
];
pub fn encrypt_password(password: &str) -> Zeroizing<String> {
    let encrypted = Zeroizing::new(
        cbc::Encryptor::<aes::Aes128>::new(AES_KEY.into(), AES_IV.into())
            .encrypt_padded_vec_mut::<Pkcs7>(password.as_bytes()),
    );
    Zeroizing::new(hex::encode(&*encrypted))
}
pub fn signature(headers: &BTreeMap<String, String>) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(MAC_KEY).expect("constant HMAC key");
    for field in SIGNED_FIELDS {
        mac.update(field.as_bytes());
        mac.update(headers[field].as_bytes());
    }
    STANDARD.encode(mac.finalize().into_bytes())
}
pub fn signed_headers() -> BTreeMap<String, String> {
    let mut h = BTreeMap::from([
        ("InvokeId-Mc".into(), uuid::Uuid::new_v4().to_string()),
        ("DeviceId-Mc".into(), uuid::Uuid::new_v4().to_string()),
        ("Encrypted-Mc".into(), "1".into()),
        ("AppId-Mc".into(), "djigo".into()),
        ("ClientName-Mc".into(), "android-9-4.3.61".into()),
        (
            "Timestamp-Mc".into(),
            chrono::Utc::now().timestamp().to_string(),
        ),
        (
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into(),
        ),
    ]);
    h.insert("Sign-Mc".into(), signature(&h));
    h
}
