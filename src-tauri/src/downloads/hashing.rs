use crate::{
    database::LocalFile,
    errors::{Error, Result},
};
use md5::Md5;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
pub fn hashes(path: &Path, cancel: &CancellationToken) -> Result<(String, String, u64)> {
    let mut file = File::open(path)?;
    let mut sha = Sha256::new();
    let mut md5 = Md5::new();
    let mut size = 0;
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        sha.update(&buffer[..n]);
        md5.update(&buffer[..n]);
        size += n as u64;
    }
    Ok((
        hex::encode(sha.finalize()),
        hex::encode(md5.finalize()),
        size,
    ))
}
pub fn inspect(path: &Path, cancel: &CancellationToken, format: &str) -> Result<LocalFile> {
    let before = std::fs::metadata(path)?;
    let (sha256, md5, size_bytes) = hashes(path, cancel)?;
    let after = std::fs::metadata(path)?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(Error::Invalid(
            "A local file changed while it was being verified. Rescan the folder.".into(),
        ));
    }
    Ok(LocalFile {
        id: uuid::Uuid::new_v4().to_string(),
        absolute_path: path.canonicalize()?.to_string_lossy().into(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        size_bytes,
        modified_at: format!("{:?}", after.modified()?),
        sha256,
        md5,
        content_format: format.into(),
    })
}
pub fn safe_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("flight-log");
    let mut clean = String::new();
    for c in base.chars().map(|c| {
        if c.is_alphanumeric() || ".()[] -_".contains(c) {
            c
        } else {
            '_'
        }
    }) {
        if clean.len() + c.len_utf8() > 180 {
            break;
        }
        clean.push(c);
    }
    let stem = Path::new(&clean)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let stem = stem.trim_matches(['.', ' ']);
    let stem = if stem.is_empty() { "flight-log" } else { stem };
    let reserved = stem.split('.').next().unwrap_or("").to_ascii_uppercase();
    let forbidden = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&reserved.as_str());
    format!("{}{stem}.txt", if forbidden { "_" } else { "" })
}
pub fn validate_folder(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        return Err(Error::Invalid(
            "The output folder is unavailable. Select a valid directory in Settings.".into(),
        ));
    }
    Ok(path.canonicalize()?)
}
