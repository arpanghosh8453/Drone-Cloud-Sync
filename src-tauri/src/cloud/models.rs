use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightRecord {
    pub id: String,
    pub remote_record_id: String,
    pub original_filename: String,
    pub flight_timestamp: Option<String>,
    pub sort_timestamp: i64,
    pub aircraft_name: Option<String>,
    pub aircraft_serial: Option<String>,
    pub duration: Option<Value>,
    pub distance: Option<Value>,
    pub max_altitude: Option<Value>,
    pub server_md5: Option<String>,
    pub server_hash_kind: String,
    pub raw_metadata: Value,
    pub status: String,
    pub local_path: Option<String>,
    pub local_md5: Option<String>,
    pub local_sha256: Option<String>,
}

pub fn objects<'a>(v: &'a Value, out: &mut Vec<&'a Map<String, Value>>) {
    match v {
        Value::Object(m) => {
            out.push(m);
            for v in m.values() {
                objects(v, out);
            }
        }
        Value::Array(a) => {
            for v in a {
                objects(v, out);
            }
        }
        _ => {}
    }
}
pub fn first<'a>(m: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .filter_map(|k| m.get(*k))
        .find(|v| !v.is_null() && **v != "")
}
pub fn string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
pub fn flatten(v: &Value) -> Map<String, Value> {
    let mut nested = vec![];
    objects(v, &mut nested);
    let mut flat = Map::new();
    for m in nested {
        for (k, v) in m {
            if !v.is_object() && !v.is_array() {
                flat.entry(k.clone()).or_insert(v.clone());
            }
        }
    }
    flat
}
pub fn normalize(entry: Value, account: &str) -> Option<FlightRecord> {
    let f = flatten(&entry);
    let get = |keys: &[&str]| first(&f, keys).and_then(string);
    let filename = get(&["file_name", "filename", "FileName", "fileName", "name"])?;
    let time = get(&[
        "start_time",
        "flight_start_time",
        "flightTime",
        "startTime",
        "create_time",
        "created_at",
        "time",
    ]);
    let serial = get(&["aircraft_sn", "aircraftSn", "drone_sn", "droneSn", "sn"]);
    // Never deduplicate across accounts. A filename/time/serial tuple is used only when no remote ID exists.
    let remote = get(&["record_id", "recordId"])
        .or_else(|| {
            entry
                .as_object()
                .and_then(|m| first(m, &["id", "uuid"]))
                .and_then(string)
        })
        .unwrap_or_else(|| {
            format!(
                "composite:{}",
                hex::encode(Sha256::digest(
                    serde_json::to_vec(&(&filename, &time, &serial)).unwrap()
                ))
            )
        });
    let id = hex::encode(Sha256::digest(format!("{account}\0{remote}")));
    let sort_timestamp = time
        .as_ref()
        .and_then(|s| {
            s.parse::<f64>()
                .ok()
                .map(|n| {
                    if n > 1e12 {
                        (n / 1000.) as i64
                    } else {
                        n as i64
                    }
                })
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|t| t.timestamp())
                })
        })
        .unwrap_or_else(|| filename_timestamp(&filename));
    Some(FlightRecord {
        id,
        remote_record_id: remote,
        original_filename: filename,
        flight_timestamp: time,
        sort_timestamp,
        aircraft_name: get(&[
            "aircraft_name",
            "aircraftName",
            "drone_name",
            "droneName",
            "product_name",
            "productName",
        ]),
        aircraft_serial: serial,
        duration: first(
            &f,
            &[
                "total_time",
                "flight_time",
                "flightTimeSeconds",
                "duration",
                "totalTime",
            ],
        )
        .cloned(),
        distance: first(&f, &["total_distance", "totalDistance", "distance"]).cloned(),
        max_altitude: first(
            &f,
            &[
                "max_height",
                "maxHeight",
                "max_altitude",
                "maxAltitude",
                "altitude",
            ],
        )
        .cloned(),
        server_md5: get(&["md5", "Md5", "MD5"])
            .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|s| s.to_lowercase()),
        server_hash_kind: "unknown".into(),
        raw_metadata: entry,
        status: "Missing".into(),
        local_path: None,
        local_md5: None,
        local_sha256: None,
    })
}
fn filename_timestamp(name: &str) -> i64 {
    // Reference fallback: YYYY-MM-DD...HH-MM-SS, including underscore and bracket separators.
    let digits: String = name
        .chars()
        .map(|c| if c.is_ascii_digit() { c } else { ' ' })
        .collect();
    let parts: Vec<&str> = digits.split_whitespace().collect();
    for p in parts.windows(6) {
        if p[0].len() != 4 || !p[0].starts_with("20") {
            continue;
        }
        let value = format!("{}-{}-{} {}:{}:{}", p[0], p[1], p[2], p[3], p[4], p[5]);
        if let Ok(t) = chrono::NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S") {
            return t.and_utc().timestamp();
        }
    }
    i64::MIN
}
