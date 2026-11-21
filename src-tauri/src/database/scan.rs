use super::{Database, LocalFile};
use crate::errors::{Error, Result};
use rusqlite::params;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use tokio_util::sync::CancellationToken;

pub struct ScannedFile {
    pub file: LocalFile,
    pub rehashed: bool,
}
pub struct ScanLink {
    pub record_id: String,
    pub file_index: usize,
    pub method: &'static str,
}

impl Database {
    /// Only reuse files verified in the active folder. Invalidated entries must be read again.
    pub fn cached_files(&self, folder: &Path) -> Result<HashMap<String, LocalFile>> {
        self.with(|c| {
            let mut q = c.prepare("SELECT id,absolute_path,filename,size_bytes,modified_at,sha256,md5,content_format FROM local_files WHERE exists_status=1")?;
            let rows = q.query_map([], |r| {
                Ok(LocalFile {
                    id: r.get(0)?,
                    absolute_path: r.get(1)?,
                    filename: r.get(2)?,
                    size_bytes: r.get(3)?,
                    modified_at: r.get(4)?,
                    sha256: r.get(5)?,
                    md5: r.get(6)?,
                    content_format: r.get(7)?,
                })
            })?;
            let mut files = HashMap::new();
            for row in rows {
                let file = row?;
                if Path::new(&file.absolute_path).parent() == Some(folder) {
                    files.insert(file.absolute_path.clone(), file);
                }
            }
            Ok(files)
        })
    }

    /// Publish one complete scan atomically; cancellation or an error rolls everything back.
    pub fn commit_scan(
        &self,
        cached: &HashMap<String, LocalFile>,
        files: &mut [ScannedFile],
        links: &[ScanLink],
        kinds: &[(String, String)],
        cancel: &CancellationToken,
    ) -> Result<()> {
        self.with(|c| {
            let tx = c.transaction()?;
            {
                let present: HashSet<_> = files.iter().map(|f| f.file.absolute_path.as_str()).collect();
                let mut invalidate = tx.prepare("UPDATE local_files SET exists_status=0 WHERE id=?")?;
                for old in cached.values() {
                    if cancel.is_cancelled() {
                        return Err(Error::Cancelled);
                    }
                    if !present.contains(old.absolute_path.as_str()) {
                        invalidate.execute([&old.id])?;
                    }
                }
            }
            {
                let mut index = tx.prepare("INSERT INTO local_files(id,absolute_path,filename,size_bytes,modified_at,sha256,md5,content_format) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(absolute_path) DO UPDATE SET filename=excluded.filename,size_bytes=excluded.size_bytes,modified_at=excluded.modified_at,sha256=excluded.sha256,md5=excluded.md5,content_format=excluded.content_format,last_verified_at=CURRENT_TIMESTAMP,exists_status=1 RETURNING id")?;
                for scanned in files.iter_mut() {
                    if cancel.is_cancelled() {
                        return Err(Error::Cancelled);
                    }
                    if !scanned.rehashed {
                        continue;
                    }
                    let f = &scanned.file;
                    scanned.file.id = index.query_row(params![f.id,f.absolute_path,f.filename,f.size_bytes,f.modified_at,f.sha256,f.md5,f.content_format], |r| r.get(0))?;
                }
                let mut link = tx.prepare("INSERT INTO flight_file_links(flight_record_id,local_file_id,expected_sha256,match_method) VALUES (?,?,?,?) ON CONFLICT(flight_record_id) DO UPDATE SET local_file_id=excluded.local_file_id,expected_sha256=excluded.expected_sha256,match_method=excluded.match_method,match_verified_at=CURRENT_TIMESTAMP WHERE local_file_id!=excluded.local_file_id OR expected_sha256!=excluded.expected_sha256 OR match_method!=excluded.match_method OR ?")?;
                for l in links {
                    if cancel.is_cancelled() {
                        return Err(Error::Cancelled);
                    }
                    let f = &files[l.file_index];
                    link.execute(params![l.record_id,f.file.id,f.file.sha256,l.method,f.rehashed])?;
                }
                let mut kind = tx.prepare("UPDATE flight_records SET server_hash_kind=? WHERE id=? AND server_hash_kind!=?")?;
                for (id, value) in kinds {
                    if cancel.is_cancelled() {
                        return Err(Error::Cancelled);
                    }
                    kind.execute(params![value, id, value])?;
                }
            }
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            tx.commit()?;
            Ok(())
        })
    }
}
