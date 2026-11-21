use crate::errors::Result;
use rusqlite::Connection;
pub fn migrate(conn: &mut Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > 1 {
        return Err(crate::errors::Error::Invalid(
            "Database was created by a newer version of drone-cloud-sync.".into(),
        ));
    }
    if version == 0 {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
        tx.commit()?;
    }
    // Closing the process can never leave an in-flight attempt marked completed.
    conn.execute("UPDATE download_attempts SET status='Cancelled', completed_at=CURRENT_TIMESTAMP, error_message='Application stopped before completion' WHERE status NOT IN ('Completed','Failed','Cancelled')", [])?;
    conn.execute("UPDATE sync_runs SET status='Interrupted', finished_at=CURRENT_TIMESTAMP WHERE status IN ('Running','Paused')", [])?;
    Ok(())
}
