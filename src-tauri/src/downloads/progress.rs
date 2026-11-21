use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub id: String,
    #[serde(default)]
    pub batch_id: String,
    pub record_id: String,
    pub sync_run_id: Option<String>,
    pub filename: String,
    pub state: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percentage: Option<f64>,
    pub speed: f64,
    pub eta_seconds: Option<f64>,
    pub error: Option<String>,
    pub attempt: u8,
}
impl DownloadProgress {
    pub fn terminal(&self) -> bool {
        matches!(self.state.as_str(), "Completed" | "Failed" | "Cancelled")
    }
    pub fn update_bytes(&mut self, bytes: u64, total: Option<u64>, seconds: f64) {
        self.downloaded_bytes = bytes;
        self.total_bytes = total;
        self.percentage = total
            .filter(|t| *t > 0)
            .map(|t| (bytes as f64 / t as f64 * 100.).min(100.));
        self.speed = if seconds > 0. {
            bytes as f64 / seconds
        } else {
            0.
        };
        self.eta_seconds = total.filter(|t| *t >= bytes).and_then(|t| {
            if self.speed > 0. {
                Some((t - bytes) as f64 / self.speed)
            } else {
                None
            }
        });
    }
}
