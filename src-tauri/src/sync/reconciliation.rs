use crate::{
    cloud::models::FlightRecord,
    database::{Database, LocalFile},
    downloads::hashing,
    errors::{Error, Result},
};
use serde::Serialize;
use std::path::Path;
use tokio_util::sync::CancellationToken;
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scanned: usize,
    pub present: usize,
    pub missing: usize,
    pub ambiguous: usize,
    pub duplicate_contents: usize,
    pub errors: Vec<String>,
}
pub fn scan(
    db: &Database,
    account: &str,
    folder: &Path,
    cancel: &CancellationToken,
    mut progress: impl FnMut(usize),
) -> Result<ScanSummary> {
    let folder = hashing::validate_folder(folder)?;
    let records = db.records(account)?;
    let associations = super::manifest::read(&folder)?;
    let mut files = vec![];
    let mut summary = ScanSummary::default();
    // No symlinks, recursion or unrelated file mutation. Old hashes remain available to recognize renamed files.
    for entry in std::fs::read_dir(&folder)? {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
        {
            continue;
        }
        match hashing::inspect(&path, cancel, "raw") {
            Ok(file) => {
                files.push(file);
                summary.scanned += 1;
                progress(summary.scanned);
            }
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(_) => summary.errors.push(format!(
                "Could not verify {}",
                entry.file_name().to_string_lossy()
            )),
        }
    }
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    db.invalidate_files()?;
    for file in &mut files {
        file.id = db.index_file(file)?;
    }
    let mut hashes = std::collections::HashSet::new();
    for f in &files {
        if !hashes.insert(&f.sha256) {
            summary.duplicate_contents += 1;
        }
    }
    for mut record in records {
        if record.local_sha256.is_none() {
            if let Some(a) = associations
                .iter()
                .find(|a| a.account_id == account && a.record_id == record.id)
            {
                record.local_sha256 = Some(a.sha256.clone());
                if a.server_md5 == record.server_md5
                    && ["content", "payload"].contains(&a.server_hash_kind.as_str())
                {
                    record.server_hash_kind = a.server_hash_kind.clone();
                    db.hash_kind(&record.id, &a.server_hash_kind)?;
                }
            }
        }
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        if let Some((file, method)) = match_file(&record, &files) {
            db.link(&record.id, &file.id, &file.sha256, method)?;
            summary.present += 1;
        } else {
            summary.missing += 1;
            if files.iter().any(|f| {
                f.filename == hashing::safe_filename(&record.original_filename)
                    || record.server_md5.as_ref() == Some(&f.md5)
            }) {
                summary.ambiguous += 1;
            }
        }
    }
    super::manifest::write(db, &folder)?;
    Ok(summary)
}
pub fn match_file<'a>(
    record: &FlightRecord,
    files: &'a [LocalFile],
) -> Option<(&'a LocalFile, &'static str)> {
    if record.server_hash_kind == "content" {
        if let Some(f) = files
            .iter()
            .find(|f| Some(&f.md5) == record.server_md5.as_ref())
        {
            return Some((f, "verified-server-md5"));
        }
    }
    if let Some(sha) = &record.local_sha256 {
        if let Some(f) = files
            .iter()
            .find(|f| Some(&f.absolute_path) == record.local_path.as_ref() && &f.sha256 == sha)
        {
            return Some((f, "verified-association"));
        }
        if let Some(f) = files.iter().find(|f| &f.sha256 == sha) {
            return Some((f, "indexed-sha256"));
        }
    }
    // The reference provides no independent metadata proving content identity. Filename-only candidates are ambiguous.
    None
}
