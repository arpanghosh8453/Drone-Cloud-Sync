use crate::{
    downloads::hashing::safe_filename,
    errors::{Error, Result},
};
use flate2::read::{MultiGzDecoder, ZlibDecoder};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use tokio_util::sync::CancellationToken;
pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
fn copy_checked(
    mut input: impl Read,
    output: &mut impl Write,
    cancel: &CancellationToken,
) -> Result<u64> {
    let mut buf = vec![0; 128 * 1024];
    let mut bytes = 0;
    loop {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let n = input.read(&mut buf).map_err(|_| {
            Error::Invalid("Flight-log decompression failed. The payload may be damaged.".into())
        })?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > MAX_FILE_BYTES {
            return Err(Error::Invalid(
                "Flight log exceeds the 512 MiB extraction limit.".into(),
            ));
        }
        output.write_all(&buf[..n])?;
    }
    if bytes == 0 {
        return Err(Error::Invalid("Downloaded flight log was empty.".into()));
    }
    Ok(bytes)
}
pub fn extract(
    part: &Path,
    folder: &Path,
    cancel: &CancellationToken,
) -> Result<(NamedTempFile, String)> {
    let mut input = File::open(part)?;
    let mut header = [0; 512];
    let n = input.read(&mut header)?;
    input.seek(SeekFrom::Start(0))?;
    let mut output = tempfile::Builder::new()
        .prefix(".drone-cloud-sync-")
        .suffix(".part")
        .tempfile_in(folder)?;
    let format;
    if header.starts_with(b"PK\x03\x04") || header.starts_with(b"PK\x05\x06") {
        format = "zip";
        let mut archive = zip::ZipArchive::new(input)
            .map_err(|_| Error::Invalid("Invalid ZIP flight log".into()))?;
        let mut files = vec![];
        let mut txt = vec![];
        for i in 0..archive.len() {
            let entry = archive
                .by_index(i)
                .map_err(|_| Error::Invalid("Cannot inspect ZIP entry".into()))?;
            if !entry.is_dir() {
                if entry.enclosed_name().is_none() || entry.is_symlink() {
                    return Err(Error::Invalid("Unsafe ZIP entry was rejected".into()));
                }
                files.push(i);
                if Path::new(entry.name())
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
                {
                    txt.push(i);
                }
            }
        }
        let index = if txt.len() == 1 {
            txt[0]
        } else if txt.is_empty() && files.len() == 1 {
            files[0]
        } else {
            return Err(Error::Invalid(
                "Archive contains multiple possible flight logs. Extraction is ambiguous.".into(),
            ));
        };
        let entry = archive
            .by_index(index)
            .map_err(|_| Error::Invalid("Cannot open ZIP entry".into()))?;
        if entry.size() > MAX_FILE_BYTES {
            return Err(Error::Invalid(
                "ZIP flight log exceeds the extraction limit".into(),
            ));
        }
        copy_checked(entry, &mut output, cancel)?;
    } else if header.starts_with(&[0x1f, 0x8b]) {
        format = "gzip";
        copy_checked(MultiGzDecoder::new(input), &mut output, cancel)?;
    } else if n >= 2
        && header[0] & 0x0f == 8
        && header[0] >> 4 <= 7
        && u16::from_be_bytes([header[0], header[1]]) % 31 == 0
    {
        format = "zlib";
        copy_checked(ZlibDecoder::new(input), &mut output, cancel)?;
    } else {
        let prefix = String::from_utf8_lossy(&header[..n])
            .trim_start()
            .to_ascii_lowercase();
        if [
            b"BZh".as_slice(),
            b"\xfd7zXZ\0",
            b"7z\xbc\xaf\x27\x1c",
            b"Rar!",
        ]
        .iter()
        .any(|magic| header.starts_with(magic))
            || prefix.starts_with("<!doctype html")
            || prefix.starts_with("<html")
            || prefix.starts_with("{\"code\"")
        {
            return Err(Error::Invalid(
                "Unsupported archive or cloud error document; no TXT file was created.".into(),
            ));
        }
        format = "raw";
        copy_checked(input, &mut output, cancel)?;
    }
    output.as_file().sync_all()?;
    Ok((output, format.into()))
}
pub fn commit(
    output: NamedTempFile,
    folder: &Path,
    filename: &str,
    record_id: &str,
) -> Result<PathBuf> {
    let name = safe_filename(filename);
    let first = folder.join(&name);
    let stem = Path::new(&name).file_stem().unwrap().to_string_lossy();
    let mut temp = output;
    for n in 0..100 {
        let path = if n == 0 {
            first.clone()
        } else if n == 1 {
            folder.join(format!(
                "{stem}-{}.txt",
                &record_id[..record_id.len().min(8)]
            ))
        } else {
            folder.join(format!("{stem}-{}.txt", uuid::Uuid::new_v4()))
        };
        match temp.persist_noclobber(&path) {
            Ok(file) => {
                file.sync_all()?;
                return Ok(path);
            }
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temp = e.file;
            }
            Err(e) => return Err(Error::Io(e.error)),
        }
    }
    Err(Error::Invalid(
        "Unable to choose a non-conflicting filename".into(),
    ))
}
