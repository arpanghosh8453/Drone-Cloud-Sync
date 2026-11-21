pub mod migrations;
mod scan;
pub use scan::{ScanLink, ScannedFile};
use crate::{
    cloud::models::FlightRecord,
    downloads::progress::DownloadProgress,
    errors::{Error, Result},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path, sync::Mutex};

pub struct Database(Mutex<Connection>);
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFile {
    pub id: String,
    pub absolute_path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub sha256: String,
    pub md5: String,
    pub content_format: String,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        migrations::migrate(&mut conn)?;
        Ok(Self(Mutex::new(conn)))
    }
    pub fn with<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut conn = self
            .0
            .lock()
            .map_err(|_| Error::Invalid("Database lock unavailable".into()))?;
        f(&mut conn)
    }
    pub fn settings(&self) -> Result<BTreeMap<String, String>> {
        self.with(|c| {
            let mut q = c.prepare("SELECT key,value FROM settings")?;
            let rows = q.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        self.with(|c| {
            Ok(
                c.query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
                    r.get(0)
                })
                .optional()?,
            )
        })
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.with(|c| {c.execute("INSERT INTO settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value])?; Ok(())})
    }
    pub fn account(&self, email: &str) -> Result<String> {
        let email = email.trim().to_lowercase();
        let id = {
            use sha2::{Digest, Sha256};
            hex::encode(Sha256::digest(email.as_bytes()))
        };
        self.with(|c| { c.execute("INSERT INTO accounts(id,email,last_login_at) VALUES (?,?,CURRENT_TIMESTAMP) ON CONFLICT(email) DO UPDATE SET last_login_at=CURRENT_TIMESTAMP",params![id,email])?; Ok(()) })?;
        self.set_setting("email", &email)?;
        self.set_setting("accountId", &id)?;
        Ok(id)
    }
    pub fn validated(&self, account: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE accounts SET last_validated_at=CURRENT_TIMESTAMP WHERE id=?",
                [account],
            )?;
            Ok(())
        })
    }
    pub fn upsert_records(&self, account: &str, records: &[FlightRecord]) -> Result<()> {
        self.with(|c| {
        let tx = c.transaction()?;
        for r in records { tx.execute("INSERT INTO flight_records (id,account_id,remote_record_id,original_filename,flight_timestamp,sort_timestamp,aircraft_name,aircraft_serial,duration,distance,max_altitude,server_md5,raw_metadata_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13) ON CONFLICT(id) DO UPDATE SET original_filename=excluded.original_filename,flight_timestamp=excluded.flight_timestamp,sort_timestamp=excluded.sort_timestamp,aircraft_name=excluded.aircraft_name,aircraft_serial=excluded.aircraft_serial,duration=excluded.duration,distance=excluded.distance,max_altitude=excluded.max_altitude,server_hash_kind=CASE WHEN server_md5=excluded.server_md5 THEN server_hash_kind ELSE 'unknown' END,server_md5=excluded.server_md5,raw_metadata_json=excluded.raw_metadata_json,last_seen_at=CURRENT_TIMESTAMP", params![r.id,account,r.remote_record_id,r.original_filename,r.flight_timestamp,r.sort_timestamp,r.aircraft_name,r.aircraft_serial,r.duration.as_ref().map(Value::to_string),r.distance.as_ref().map(Value::to_string),r.max_altitude.as_ref().map(Value::to_string),r.server_md5,r.raw_metadata.to_string()])?; }
        tx.commit()?; Ok(())
    })
    }
    pub fn records(&self, account: &str) -> Result<Vec<FlightRecord>> {
        self.with(|c| {
        let mut q = c.prepare("SELECT r.id,r.remote_record_id,r.original_filename,r.flight_timestamp,r.sort_timestamp,r.aircraft_name,r.aircraft_serial,r.duration,r.distance,r.max_altitude,r.server_md5,r.server_hash_kind,r.raw_metadata_json,f.absolute_path,f.md5,f.sha256,COALESCE(f.exists_status,0),l.expected_sha256 FROM flight_records r LEFT JOIN flight_file_links l ON l.flight_record_id=r.id LEFT JOIN local_files f ON f.id=l.local_file_id WHERE r.account_id=? ORDER BY r.sort_timestamp DESC,r.original_filename DESC")?;
        let rows=q.query_map([account], |r| {
            let json = |idx| -> rusqlite::Result<Option<Value>> { Ok(r.get::<_,Option<String>>(idx)?.and_then(|s|serde_json::from_str(&s).ok())) };
            let exists: i32=r.get(16)?; let sha: Option<String>=r.get(15)?; let expected: Option<String>=r.get(17)?;
            Ok(FlightRecord { id:r.get(0)?,remote_record_id:r.get(1)?,original_filename:r.get(2)?,flight_timestamp:r.get(3)?,sort_timestamp:r.get(4)?,aircraft_name:r.get(5)?,aircraft_serial:r.get(6)?,duration:json(7)?,distance:json(8)?,max_altitude:json(9)?,server_md5:r.get(10)?,server_hash_kind:r.get(11)?,raw_metadata:json(12)?.unwrap_or(Value::Null),status:if exists==1 && sha==expected {"Downloaded"}else{"Missing"}.into(),local_path:r.get(13)?,local_md5:r.get(14)?,local_sha256:expected })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>,_>>()?)
    })
    }
    pub fn record(&self, account: &str, id: &str) -> Result<FlightRecord> {
        self.records(account)?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| {
                Error::Invalid("Flight record does not belong to the current account.".into())
            })
    }
    pub fn index_file(&self, file: &LocalFile) -> Result<String> {
        self.with(|c| {
        c.execute("INSERT INTO local_files(id,absolute_path,filename,size_bytes,modified_at,sha256,md5,content_format) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(absolute_path) DO UPDATE SET filename=excluded.filename,size_bytes=excluded.size_bytes,modified_at=excluded.modified_at,sha256=excluded.sha256,md5=excluded.md5,content_format=excluded.content_format,last_verified_at=CURRENT_TIMESTAMP,exists_status=1",params![file.id,file.absolute_path,file.filename,file.size_bytes,file.modified_at,file.sha256,file.md5,file.content_format])?;
        Ok(c.query_row("SELECT id FROM local_files WHERE absolute_path=?",[&file.absolute_path],|r|r.get(0))?)
    })
    }
    pub fn link(&self, record: &str, file: &str, sha: &str, method: &str) -> Result<()> {
        self.with(|c| { c.execute("INSERT INTO flight_file_links(flight_record_id,local_file_id,expected_sha256,match_method) VALUES (?,?,?,?) ON CONFLICT(flight_record_id) DO UPDATE SET local_file_id=excluded.local_file_id,expected_sha256=excluded.expected_sha256,match_method=excluded.match_method,match_verified_at=CURRENT_TIMESTAMP",params![record,file,sha,method])?; Ok(()) })
    }
    pub fn hash_kind(&self, id: &str, kind: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE flight_records SET server_hash_kind=? WHERE id=?",
                params![kind, id],
            )?;
            Ok(())
        })
    }
    pub fn invalidate_files(&self) -> Result<()> {
        self.with(|c| {
            c.execute("UPDATE local_files SET exists_status=0", [])?;
            Ok(())
        })
    }
    pub fn attempt(&self, p: &DownloadProgress) -> Result<()> {
        self.with(|c| {
        let finished=matches!(p.state.as_str(),"Completed"|"Failed"|"Cancelled");
        c.execute("INSERT INTO download_attempts(id,flight_record_id,sync_run_id,status,bytes_downloaded,error_message,progress_json,completed_at) VALUES (?,?,?,?,?,?,?,CASE WHEN ? THEN CURRENT_TIMESTAMP END) ON CONFLICT(id) DO UPDATE SET status=excluded.status,bytes_downloaded=excluded.bytes_downloaded,error_message=excluded.error_message,progress_json=excluded.progress_json,completed_at=excluded.completed_at",params![p.id,p.record_id,p.sync_run_id,p.state,p.downloaded_bytes,p.error,serde_json::to_string(p).unwrap(),finished])?;Ok(())
    })
    }
    pub fn history(&self, account: &str, record: Option<&str>) -> Result<Vec<DownloadProgress>> {
        self.with(|c| {
        let mut q=c.prepare("SELECT a.id,a.flight_record_id,a.status,a.bytes_downloaded,a.error_message,a.progress_json FROM download_attempts a JOIN flight_records r ON r.id=a.flight_record_id WHERE r.account_id=?1 AND (?2 IS NULL OR a.flight_record_id=?2) ORDER BY a.started_at DESC,a.rowid DESC LIMIT 500")?;
        let rows=q.query_map(params![account,record],|r| {
            let p: Option<String>=r.get(5)?;
            let mut p: DownloadProgress=p.and_then(|p|serde_json::from_str(&p).ok()).unwrap_or_default();
            p.id=r.get(0)?;p.record_id=r.get(1)?;p.state=r.get(2)?;p.downloaded_bytes=r.get(3)?;p.error=r.get(4)?;
            if ["Completed","Cancelled","Failed"].contains(&p.state.as_str()) {p.speed=0.;p.eta_seconds=None;}
            Ok(p)
        })?;Ok(rows.collect::<std::result::Result<_,_>>()?)
    })
    }
    pub fn clear_completed(&self, account: &str) -> Result<()> {
        self.with(|c|{c.execute("DELETE FROM download_attempts WHERE status='Completed' AND flight_record_id IN (SELECT id FROM flight_records WHERE account_id=?)",[account])?;Ok(())})
    }
    pub fn sync_history(&self, account: &str) -> Result<Vec<Value>> {
        self.with(|c| {
        let mut q=c.prepare("SELECT id,started_at,finished_at,status,total_records,already_present,downloaded,failed,error_summary FROM sync_runs WHERE account_id=? ORDER BY started_at DESC LIMIT 50")?;
        let rows=q.query_map([account],|r|Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"startedAt":r.get::<_,String>(1)?,"finishedAt":r.get::<_,Option<String>>(2)?,"status":r.get::<_,String>(3)?,"totalRecords":r.get::<_,u64>(4)?,"alreadyPresent":r.get::<_,u64>(5)?,"downloaded":r.get::<_,u64>(6)?,"failed":r.get::<_,u64>(7)?,"error":r.get::<_,Option<String>>(8)?})))?; Ok(rows.collect::<std::result::Result<_,_>>()?)
    })
    }
}
