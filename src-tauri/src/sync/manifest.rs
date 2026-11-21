//! Portable, non-secret hash associations allow safe recovery after SQLite is rebuilt.
use crate::{
    database::Database,
    errors::{Error, Result},
};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};
pub const INDEX_NAME: &str = ".drone-cloud-sync-index.json";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Association {
    pub account_id: String,
    pub record_id: String,
    pub sha256: String,
    pub server_md5: Option<String>,
    pub server_hash_kind: String,
}
#[derive(Serialize, Deserialize)]
struct Index {
    version: u32,
    associations: Vec<Association>,
}
pub fn read(folder: &Path) -> Result<Vec<Association>> {
    let path = folder.join(INDEX_NAME);
    if !path.exists() {
        return Ok(vec![]);
    }
    if path.symlink_metadata()?.file_type().is_symlink()
        || path.metadata()?.len() > 16 * 1024 * 1024
    {
        return Err(Error::Invalid(
            "Local hash index is unsafe or too large. Inspect the output folder.".into(),
        ));
    }
    let index: Index = serde_json::from_reader(std::fs::File::open(path)?).map_err(|_| {
        Error::Invalid(
            "The local hash index is damaged. Restore it or move it aside before scanning.".into(),
        )
    })?;
    if index.version != 1 {
        return Err(Error::Invalid(
            "Local hash index was written by an unsupported version.".into(),
        ));
    }
    Ok(index
        .associations
        .into_iter()
        .filter(|a| a.sha256.len() == 64 && a.sha256.bytes().all(|c| c.is_ascii_hexdigit()))
        .collect())
}
pub fn write(db: &Database, folder: &Path) -> Result<()> {
    // Serialize read/merge/atomic replacement under the same lock as SQLite links.
    db.with(|conn|{
        let mut entries=read(folder)?;
        let mut q=conn.prepare("SELECT r.account_id,r.id,l.expected_sha256,r.server_md5,r.server_hash_kind,f.absolute_path FROM flight_file_links l JOIN flight_records r ON r.id=l.flight_record_id JOIN local_files f ON f.id=l.local_file_id")?;
        let rows=q.query_map([],|r|Ok((Association{account_id:r.get(0)?,record_id:r.get(1)?,sha256:r.get(2)?,server_md5:r.get(3)?,server_hash_kind:r.get(4)?},r.get::<_,String>(5)?)))?;
        for row in rows{let (association,path)=row?;if Path::new(&path).parent()!=Some(folder){continue;}
            entries.retain(|a|a.account_id!=association.account_id||a.record_id!=association.record_id);entries.push(association);
        }
        let index=Index{version:1,associations:entries};
        let mut file=tempfile::Builder::new().prefix(".drone-cloud-sync-index-").suffix(".part").tempfile_in(folder)?;
        serde_json::to_writer(&mut file,&index).map_err(|_|Error::Invalid("Could not encode the local hash index".into()))?;file.flush()?;file.as_file().sync_all()?;
        file.persist(folder.join(INDEX_NAME)).map_err(|e|Error::Io(e.error))?;Ok(())
    })
}
