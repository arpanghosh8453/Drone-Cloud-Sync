use crate::{
    database::{Database, ScanLink, ScannedFile},
    downloads::hashing,
    errors::{Error, Result},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use tokio_util::sync::CancellationToken;
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scanned: usize,
    pub hashed: usize,
    pub reused: usize,
    pub check_ms: u64,
    pub compare_ms: u64,
    pub save_ms: u64,
    pub present: usize,
    pub missing: usize,
    pub ambiguous: usize,
    pub duplicate_contents: usize,
    pub errors: Vec<String>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Incremental,
    Full,
}

#[derive(Clone, Copy, Default)]
pub struct ScanProgress {
    pub stage: &'static str,
    pub checked: usize,
    pub total: usize,
    pub hashed: usize,
    pub reused: usize,
}

pub fn scan(
    db: &Database,
    account: &str,
    folder: &Path,
    mode: ScanMode,
    cancel: &CancellationToken,
    mut progress: impl FnMut(ScanProgress),
) -> Result<ScanSummary> {
    let started = std::time::Instant::now();
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let folder = hashing::validate_folder(folder)?;
    let records = db.records(account)?;
    let associations = super::manifest::read(&folder)?;
    let associations: HashMap<_, _> = associations
        .iter()
        .filter(|a| a.account_id == account)
        .map(|a| (a.record_id.as_str(), a))
        .collect();
    let cached = db.cached_files(&folder)?;
    let mut paths = vec![];
    // No symlinks, recursion or unrelated file mutation.
    for entry in std::fs::read_dir(&folder)? {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
        {
            paths.push(path);
        }
    }
    paths.sort_unstable();
    let mut state = ScanProgress {
        stage: "Scanning local files",
        total: paths.len(),
        ..Default::default()
    };
    progress(state);
    let mut files = vec![];
    let mut summary = ScanSummary::default();
    for path in paths {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let inspected = (|| {
            let metadata = path.symlink_metadata()?;
            if !metadata.is_file() {
                return Err(Error::Invalid(
                    "Local file is no longer a regular file".into(),
                ));
            }
            let key = path.to_string_lossy();
            if mode == ScanMode::Incremental {
                if let Some(old) = cached.get(key.as_ref()) {
                    if old.size_bytes == metadata.len()
                        && old.modified_at == format!("{:?}", metadata.modified()?)
                    {
                        return Ok(ScannedFile {
                            file: old.clone(),
                            rehashed: false,
                        });
                    }
                }
            }
            Ok(ScannedFile {
                file: hashing::inspect(&path, cancel, "raw")?,
                rehashed: true,
            })
        })();
        match inspected {
            Ok(file) => {
                if file.rehashed {
                    state.hashed += 1;
                } else {
                    state.reused += 1;
                }
                files.push(file);
                summary.scanned += 1;
            }
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(_) => summary.errors.push(format!(
                "Could not verify {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
        }
        state.checked += 1;
        progress(state);
    }
    summary.hashed = state.hashed;
    summary.reused = state.reused;
    summary.check_ms = started.elapsed().as_millis() as u64;
    state.stage = "Comparing records";
    progress(state);
    let comparing = std::time::Instant::now();
    let mut md5 = HashMap::new();
    let mut sha = HashMap::new();
    let mut paths = HashMap::new();
    let mut names = HashSet::new();
    for (i, scanned) in files.iter().enumerate() {
        let f = &scanned.file;
        md5.entry(f.md5.as_str()).or_insert(i);
        if let std::collections::hash_map::Entry::Vacant(e) = sha.entry(f.sha256.as_str()) {
            e.insert(i);
        } else {
            summary.duplicate_contents += 1;
        }
        paths.insert(f.absolute_path.as_str(), i);
        names.insert(f.filename.as_str());
    }
    let mut links = vec![];
    let mut kinds = vec![];
    for mut record in records {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        if record.local_sha256.is_none() {
            if let Some(a) = associations.get(record.id.as_str()) {
                record.local_sha256 = Some(a.sha256.clone());
                if a.server_md5 == record.server_md5
                    && ["content", "payload"].contains(&a.server_hash_kind.as_str())
                {
                    record.server_hash_kind = a.server_hash_kind.clone();
                    kinds.push((record.id.clone(), a.server_hash_kind.clone()));
                }
            }
        }
        let matched = if record.server_hash_kind == "content" {
            record
                .server_md5
                .as_deref()
                .and_then(|h| md5.get(h))
                .map(|&i| (i, "verified-server-md5"))
        } else {
            None
        }
        .or_else(|| {
            let expected = record.local_sha256.as_deref()?;
            if let Some(&i) = record.local_path.as_deref().and_then(|p| paths.get(p)) {
                if files[i].file.sha256 == expected {
                    return Some((i, "verified-association"));
                }
            }
            sha.get(expected).map(|&i| (i, "indexed-sha256"))
        });
        if let Some((file_index, method)) = matched {
            links.push(ScanLink {
                record_id: record.id,
                file_index,
                method,
            });
            summary.present += 1;
        } else {
            summary.missing += 1;
            // Filename-only matches and unproven server hashes never establish identity.
            if names.contains(hashing::safe_filename(&record.original_filename).as_str())
                || record
                    .server_md5
                    .as_deref()
                    .is_some_and(|h| md5.contains_key(h))
            {
                summary.ambiguous += 1;
            }
        }
    }
    summary.compare_ms = comparing.elapsed().as_millis() as u64;
    state.stage = "Saving scan results";
    progress(state);
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let saving = std::time::Instant::now();
    db.commit_scan(&cached, &mut files, &links, &kinds, cancel)?;
    super::manifest::write(db, &folder)?;
    summary.save_ms = saving.elapsed().as_millis() as u64;
    Ok(summary)
}
