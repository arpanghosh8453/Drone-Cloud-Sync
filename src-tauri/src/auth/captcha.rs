use serde::Serialize;
use std::time::Instant;
#[derive(Clone)]
pub struct Challenge {
    pub id: String,
    pub created: Instant,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Captcha {
    pub challenge_id: String,
    pub image_data_url: String,
}
pub fn random_id() -> String {
    use rand::Rng;
    let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| chars[rng.gen_range(0..chars.len())] as char)
        .collect()
}
