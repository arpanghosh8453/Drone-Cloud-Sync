use serde::Serialize;
use serde_json::Value;
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &str, payload: Value);
}
pub fn emit(sink: &dyn EventSink, name: &str, payload: &impl Serialize) {
    if let Ok(value) = serde_json::to_value(payload) {
        sink.emit(name, value);
    }
}
#[derive(Default)]
pub struct NoEvents;
impl EventSink for NoEvents {
    fn emit(&self, _: &str, _: Value) {}
}
