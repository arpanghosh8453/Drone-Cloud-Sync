use crate::{
    cloud::models::objects,
    errors::{Error, Result},
};
use serde_json::Value;
use std::collections::HashSet;
pub const PAGE_SIZE: usize = 99;
pub fn record_list(payload: &Value) -> Result<&Vec<Value>> {
    let mut nested = vec![];
    objects(payload, &mut nested);
    for item in nested {
        for key in ["list", "List", "FileList", "fileList", "records", "Records"] {
            if let Some(Value::Array(a)) = item.get(key) {
                if a.iter().all(Value::is_object) {
                    return Ok(a);
                }
            }
        }
    }
    Err(Error::Schema(
        "Cannot locate flight records in response".into(),
    ))
}
#[derive(Default)]
pub struct Pagination {
    pub seen: HashSet<String>,
    pub pages: usize,
    pub total: usize,
}
impl Pagination {
    pub fn finished(&self, payload: &Value, batch: usize, page: usize) -> bool {
        let mut nested = vec![];
        objects(payload, &mut nested);
        for item in &nested {
            for key in ["has_more", "hasMore"] {
                if let Some(v) = item.get(key).and_then(Value::as_bool) {
                    return !v;
                }
            }
            for key in ["total_pages", "totalPages", "page_count", "pageCount"] {
                if let Some(n) = item.get(key).and_then(number) {
                    return page >= n as usize;
                }
            }
        }
        for item in nested {
            for key in ["total", "total_count", "totalCount"] {
                if let Some(n) = item.get(key).and_then(number) {
                    return self.total >= n as usize;
                }
            }
        }
        batch < PAGE_SIZE
    }
}
fn number(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}
