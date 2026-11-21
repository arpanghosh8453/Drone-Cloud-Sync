use drone_cloud_sync::{
    cloud::models::{normalize, FlightRecord},
    database::Database,
    downloads::hashing,
    errors::Error,
    sync::{
        manifest,
        reconciliation::{scan, ScanMode, ScanProgress, ScanSummary},
    },
};
use std::{fs, time::Instant};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

struct Fixture {
    root: TempDir,
    db: Database,
    account: String,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let db = Database::open(&root.path().join("metadata.sqlite")).unwrap();
        let account = db.account("pilot@example.com").unwrap();
        fs::create_dir(root.path().join("logs")).unwrap();
        Self { root, db, account }
    }
    fn folder(&self) -> std::path::PathBuf {
        self.root.path().join("logs").canonicalize().unwrap()
    }
    fn file(&self, name: &str, contents: &[u8]) {
        fs::write(self.folder().join(name), contents).unwrap();
    }
    fn record(&self, name: &str, known_content: bool) -> FlightRecord {
        let file =
            hashing::inspect(&self.folder().join(name), &CancellationToken::new(), "raw").unwrap();
        let record = normalize(
            serde_json::json!({"id":name,"filename":name,"md5":file.md5}),
            &self.account,
        )
        .unwrap();
        self.db
            .upsert_records(&self.account, &[record.clone()])
            .unwrap();
        if known_content {
            self.db.hash_kind(&record.id, "content").unwrap();
        }
        record
    }
    fn scan(&self, mode: ScanMode) -> ScanSummary {
        scan(
            &self.db,
            &self.account,
            &self.folder(),
            mode,
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap()
    }
    fn get(&self, record: &FlightRecord) -> FlightRecord {
        self.db.record(&self.account, &record.id).unwrap()
    }
}

#[test]
fn unchanged_cache_survives_restart_and_skips_database_and_manifest_writes() {
    let f = Fixture::new();
    f.file("a.txt", b"first flight");
    f.file("b.TXT", b"second flight");
    let record = f.record("a.txt", true);
    let cold = f.scan(ScanMode::Incremental);
    assert_eq!(
        (cold.scanned, cold.hashed, cold.reused, cold.present),
        (2, 2, 0, 1)
    );
    let index_time = fs::metadata(f.folder().join(manifest::INDEX_NAME))
        .unwrap()
        .modified()
        .unwrap();
    let db = Database::open(&f.root.path().join("metadata.sqlite")).unwrap();
    let changes = db.with(|c| Ok(c.total_changes())).unwrap();
    let warm = scan(
        &db,
        &f.account,
        &f.folder(),
        ScanMode::Incremental,
        &CancellationToken::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!((warm.hashed, warm.reused, warm.present), (0, 2, 1));
    assert_eq!(changes, db.with(|c| Ok(c.total_changes())).unwrap());
    assert_eq!(
        index_time,
        fs::metadata(f.folder().join(manifest::INDEX_NAME))
            .unwrap()
            .modified()
            .unwrap()
    );
    assert_eq!(
        db.record(&f.account, &record.id).unwrap().status,
        "Downloaded"
    );
}

#[test]
fn changes_deletions_and_recreation_preserve_identity_and_recheck_only_changed_files() {
    let f = Fixture::new();
    f.file("a.txt", b"first flight");
    f.file("b.txt", b"second flight");
    let a = f.record("a.txt", true);
    let b = f.record("b.txt", true);
    f.scan(ScanMode::Incremental);
    let original_id: String =
        f.db.with(|c| {
            Ok(c.query_row(
                "SELECT id FROM local_files WHERE filename='a.txt'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    f.file("a.txt", b"changed flight contents");
    let changed = f.scan(ScanMode::Incremental);
    assert_eq!((changed.hashed, changed.reused, changed.missing), (1, 1, 1));
    assert_eq!(f.get(&a).status, "Missing");
    assert_eq!(f.get(&b).status, "Downloaded");
    fs::remove_file(f.folder().join("a.txt")).unwrap();
    f.scan(ScanMode::Incremental);
    f.file("a.txt", b"first flight");
    let recreated = f.scan(ScanMode::Incremental);
    assert_eq!(
        (recreated.hashed, recreated.reused, recreated.present),
        (1, 1, 2)
    );
    let id: String =
        f.db.with(|c| {
            Ok(c.query_row(
                "SELECT id FROM local_files WHERE filename='a.txt'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(id, original_id);
    fs::remove_file(f.folder().join("b.txt")).unwrap();
    assert_eq!(f.scan(ScanMode::Incremental).missing, 1);
    assert_eq!(f.get(&b).status, "Missing");
}

#[test]
fn full_rescan_detects_content_changes_with_preserved_size_and_timestamp() {
    let f = Fixture::new();
    f.file("a.txt", b"aaaa");
    let record = f.record("a.txt", true);
    f.scan(ScanMode::Incremental);
    let path = f.folder().join("a.txt");
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    fs::write(&path, b"bbbb").unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(f.scan(ScanMode::Incremental).reused, 1);
    let full = f.scan(ScanMode::Full);
    assert_eq!((full.hashed, full.reused, full.missing), (1, 0, 1));
    assert_eq!(f.get(&record).status, "Missing");
}

#[test]
fn timestamp_only_change_requires_hashing() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    f.scan(ScanMode::Incremental);
    let path = f.folder().join("a.txt");
    let modified =
        fs::metadata(&path).unwrap().modified().unwrap() + std::time::Duration::from_secs(2);
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(f.scan(ScanMode::Incremental).hashed, 1);
}

#[test]
fn renamed_files_and_duplicate_contents_keep_verified_associations() {
    let f = Fixture::new();
    f.file("original.txt", b"flight");
    let record = f.record("original.txt", false);
    let file = hashing::inspect(
        &f.folder().join("original.txt"),
        &CancellationToken::new(),
        "raw",
    )
    .unwrap();
    let id = f.db.index_file(&file).unwrap();
    f.db.link(&record.id, &id, &file.sha256, "downloaded")
        .unwrap();
    f.scan(ScanMode::Incremental);
    fs::rename(
        f.folder().join("original.txt"),
        f.folder().join("z-renamed.txt"),
    )
    .unwrap();
    assert_eq!(f.scan(ScanMode::Incremental).present, 1);
    f.file("a-duplicate.txt", b"flight");
    let result = f.scan(ScanMode::Incremental);
    assert_eq!(
        (
            result.hashed,
            result.reused,
            result.duplicate_contents,
            result.present
        ),
        (1, 1, 1, 1)
    );
    assert!(f
        .get(&record)
        .local_path
        .unwrap()
        .ends_with("z-renamed.txt"));
}

#[test]
fn filename_and_unproven_server_hash_do_not_establish_identity() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    let r = f.record("a.txt", false);
    let result = f.scan(ScanMode::Incremental);
    assert_eq!(
        (result.present, result.missing, result.ambiguous),
        (0, 1, 1)
    );
    assert_eq!(f.get(&r).status, "Missing");
    f.db.hash_kind(&r.id, "payload").unwrap();
    assert_eq!(f.scan(ScanMode::Incremental).present, 0);
}

#[test]
fn manifest_recovers_renamed_files_after_database_rebuild() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    let record = f.record("a.txt", true);
    f.scan(ScanMode::Incremental);
    fs::rename(f.folder().join("a.txt"), f.folder().join("renamed.txt")).unwrap();
    let db = Database::open(&f.root.path().join("rebuilt.sqlite")).unwrap();
    let account = db.account("pilot@example.com").unwrap();
    db.upsert_records(&account, &[record.clone()]).unwrap();
    let result = scan(
        &db,
        &account,
        &f.folder(),
        ScanMode::Incremental,
        &CancellationToken::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!((result.hashed, result.present), (1, 1));
    assert_eq!(
        db.record(&account, &record.id).unwrap().status,
        "Downloaded"
    );
}

#[test]
fn cancellation_before_or_during_scan_does_not_publish_partial_results() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    let r = f.record("a.txt", true);
    f.scan(ScanMode::Incremental);
    f.file("a.txt", b"changed");
    for phase in [
        "before",
        "Scanning local files",
        "Comparing records",
        "Saving scan results",
    ] {
        let cancel = CancellationToken::new();
        if phase == "before" {
            cancel.cancel();
        }
        let result = scan(
            &f.db,
            &f.account,
            &f.folder(),
            ScanMode::Incremental,
            &cancel,
            |p| {
                if p.stage == phase && (p.checked == p.total || p.stage != "Scanning local files") {
                    cancel.cancel();
                }
            },
        );
        assert!(matches!(result, Err(Error::Cancelled)), "{phase}");
        assert_eq!(f.get(&r).status, "Downloaded");
    }
}

#[test]
fn database_failure_rolls_back_file_updates_and_links_together() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    let r = f.record("a.txt", true);
    f.scan(ScanMode::Incremental);
    let original = f.get(&r).local_md5;
    f.file("a.txt", b"different");
    f.file("b.txt", b"second");
    f.record("b.txt", true);
    f.db.with(|c| { c.execute_batch("CREATE TRIGGER fail_link BEFORE INSERT ON flight_file_links BEGIN SELECT RAISE(ABORT, 'test failure'); END;")?; Ok(()) }).unwrap();
    assert!(scan(
        &f.db,
        &f.account,
        &f.folder(),
        ScanMode::Full,
        &CancellationToken::new(),
        |_| {}
    )
    .is_err());
    assert_eq!(f.get(&r).local_md5, original);
    assert_eq!(f.get(&r).status, "Downloaded");
}

#[test]
fn folder_scan_ignores_nested_unrelated_and_symlink_files_and_reports_phases() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    f.file("image.jpg", b"image");
    fs::create_dir(f.folder().join("nested")).unwrap();
    fs::write(f.folder().join("nested/b.txt"), b"nested").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(f.folder().join("a.txt"), f.folder().join("link.txt")).unwrap();
    let mut updates: Vec<ScanProgress> = vec![];
    let result = scan(
        &f.db,
        &f.account,
        &f.folder(),
        ScanMode::Incremental,
        &CancellationToken::new(),
        |p| updates.push(p),
    )
    .unwrap();
    assert_eq!(result.scanned, 1);
    assert_eq!(updates[0].total, 1);
    assert_eq!(updates[0].checked, 0);
    assert_eq!(updates.last().unwrap().checked, 1);
    assert_eq!(
        updates.iter().map(|p| p.stage).collect::<Vec<_>>(),
        vec![
            "Scanning local files",
            "Scanning local files",
            "Comparing records",
            "Saving scan results"
        ]
    );
    let other = tempfile::tempdir().unwrap();
    fs::write(other.path().join("other.txt"), b"other").unwrap();
    let file = hashing::inspect(
        &other.path().join("other.txt"),
        &CancellationToken::new(),
        "raw",
    )
    .unwrap();
    f.db.index_file(&file).unwrap();
    f.scan(ScanMode::Incremental);
    assert_eq!(
        f.db.cached_files(&other.path().canonicalize().unwrap())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn invalidated_files_are_read_again_and_other_account_associations_survive() {
    let f = Fixture::new();
    f.file("a.txt", b"flight");
    f.record("a.txt", true);
    f.scan(ScanMode::Incremental);
    let second_account = f.db.account("second@example.com").unwrap();
    let file =
        hashing::inspect(&f.folder().join("a.txt"), &CancellationToken::new(), "raw").unwrap();
    let second_record = normalize(
        serde_json::json!({"id":"second-flight", "filename":"a.txt", "md5":file.md5}),
        &second_account,
    )
    .unwrap();
    f.db.upsert_records(&second_account, &[second_record.clone()])
        .unwrap();
    f.db.hash_kind(&second_record.id, "content").unwrap();
    scan(
        &f.db,
        &second_account,
        &f.folder(),
        ScanMode::Incremental,
        &CancellationToken::new(),
        |_| {},
    )
    .unwrap();
    f.scan(ScanMode::Incremental);
    let entries = manifest::read(&f.folder()).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|a| a.account_id == f.account));
    assert!(entries.iter().any(|a| a.account_id == second_account));
    f.db.invalidate_files().unwrap();
    assert_eq!(f.scan(ScanMode::Incremental).hashed, 1);
}

#[test]
#[ignore = "manual throughput comparison; run with --ignored --nocapture"]
fn cold_and_warm_scan_timings() {
    let f = Fixture::new();
    let contents = vec![b'x'; 1024 * 1024];
    for i in 0..32 {
        f.file(&format!("flight-{i}.txt"), &contents);
    }
    let (_, md5, _) =
        hashing::hashes(&f.folder().join("flight-0.txt"), &CancellationToken::new()).unwrap();
    let records: Vec<_> = (0..1000).map(|i| normalize(serde_json::json!({"id":format!("record-{i}"),"filename":format!("flight-{}.txt", i % 32),"md5":md5}), &f.account).unwrap()).collect();
    f.db.upsert_records(&f.account, &records).unwrap();
    f.db.with(|c| {
        c.execute("UPDATE flight_records SET server_hash_kind='content'", [])?;
        Ok(())
    })
    .unwrap();
    let before = Instant::now();
    let cold = f.scan(ScanMode::Incremental);
    let cold_elapsed = before.elapsed();
    let before = Instant::now();
    let warm = f.scan(ScanMode::Incremental);
    let warm_elapsed = before.elapsed();
    assert_eq!((cold.hashed, warm.hashed, warm.reused), (32, 0, 32));
    assert_eq!((cold.present, warm.present), (1000, 1000));
    println!("32 MiB / 32 files / 1000 records: cold {cold_elapsed:?}, warm {warm_elapsed:?}; cold check/compare/save {} / {} / {} ms; warm {} / {} / {} ms", cold.check_ms,cold.compare_ms,cold.save_ms,warm.check_ms,warm.compare_ms,warm.save_ms);
}
