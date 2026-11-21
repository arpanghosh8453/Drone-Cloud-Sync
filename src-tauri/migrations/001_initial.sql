PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS accounts (
 id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 last_login_at TEXT, last_validated_at TEXT
);
CREATE TABLE IF NOT EXISTS flight_records (
 id TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id), remote_record_id TEXT NOT NULL,
 original_filename TEXT NOT NULL, flight_timestamp TEXT, sort_timestamp INTEGER NOT NULL,
 aircraft_name TEXT, aircraft_serial TEXT, duration TEXT, distance TEXT, max_altitude TEXT,
 server_md5 TEXT, server_hash_kind TEXT NOT NULL DEFAULT 'unknown', raw_metadata_json TEXT NOT NULL,
 first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE(account_id, remote_record_id)
);
CREATE INDEX IF NOT EXISTS records_account_date ON flight_records(account_id, sort_timestamp DESC);
CREATE INDEX IF NOT EXISTS records_server_md5 ON flight_records(server_md5);
CREATE TABLE IF NOT EXISTS local_files (
 id TEXT PRIMARY KEY, absolute_path TEXT NOT NULL UNIQUE, filename TEXT NOT NULL, size_bytes INTEGER NOT NULL,
 modified_at TEXT NOT NULL, sha256 TEXT NOT NULL, md5 TEXT NOT NULL, content_format TEXT NOT NULL,
 last_verified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, exists_status INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS local_sha256 ON local_files(sha256);
CREATE INDEX IF NOT EXISTS local_md5 ON local_files(md5);
CREATE TABLE IF NOT EXISTS flight_file_links (
 flight_record_id TEXT PRIMARY KEY REFERENCES flight_records(id), local_file_id TEXT NOT NULL REFERENCES local_files(id),
 match_method TEXT NOT NULL, expected_sha256 TEXT NOT NULL, match_verified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS sync_runs (
 id TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id), started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 finished_at TEXT, status TEXT NOT NULL, total_records INTEGER NOT NULL DEFAULT 0,
 already_present INTEGER NOT NULL DEFAULT 0, downloaded INTEGER NOT NULL DEFAULT 0, failed INTEGER NOT NULL DEFAULT 0, error_summary TEXT
);
CREATE TABLE IF NOT EXISTS download_attempts (
 id TEXT PRIMARY KEY, flight_record_id TEXT NOT NULL REFERENCES flight_records(id), sync_run_id TEXT REFERENCES sync_runs(id),
 started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, completed_at TEXT, status TEXT NOT NULL,
 bytes_downloaded INTEGER NOT NULL DEFAULT 0, error_message TEXT, progress_json TEXT
);
CREATE INDEX IF NOT EXISTS attempts_record ON download_attempts(flight_record_id, started_at DESC);
CREATE TABLE IF NOT EXISTS settings ( key TEXT PRIMARY KEY, value TEXT NOT NULL );
PRAGMA user_version = 1;
