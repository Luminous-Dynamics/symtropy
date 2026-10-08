// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SQLite implementation of the external transparency witness-state contract.
//!
//! The backend uses one SQLite row per policy/log namespace and stores the
//! complete versioned witness snapshot as a single JSON value. Each CAS uses
//! BEGIN IMMEDIATE so competing writers serialize against the same stored
//! predecessor. WAL + synchronous=FULL are selected deliberately so SQLite
//! commits include the durability sync needed for the backend's crash-resilience
//! contract. Actual production durability still depends on the operating system
//! and storage device honoring sync requests.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};

use super::transparency::TransparencyWitnessStateSnapshotV1;
use super::transparency_store::{
    validate_cas_result, TransparencyWitnessStateStore, TransparencyWitnessStoreError,
    TransparencyWitnessStoreKeyV1, TransparencyWitnessStoredStateV1,
};

const TABLE: &str = "symtropy_transparency_witness_state";

/// Transactional SQLite backend for retained transparency witness memory.
///
/// The database is intentionally scoped to this one store contract. The caller
/// remains responsible for independently operating the backend if the threat
/// model requires separation from the execution journal.
#[derive(Debug, Clone)]
pub struct SqliteTransparencyWitnessStateStore {
    path: PathBuf,
}

impl SqliteTransparencyWitnessStateStore {
    /// Open (and initialize) a SQLite witness-state database.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, TransparencyWitnessStoreError> {
        let path = path.into();
        let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty());
        if let Some(parent) = parent {
            std::fs::create_dir_all(parent).map_err(|error| {
                TransparencyWitnessStoreError::Backend(format!(
                    "cannot create SQLite parent directory {}: {error}",
                    parent.display()
                ))
            })?;
        }

        let store = Self { path };
        let _ = store.connection()?;
        Ok(store)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connection(&self) -> Result<Connection, TransparencyWitnessStoreError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = {
            #[cfg(test)]
            if let Some(vfs) = sqlite_fault_vfs::active_name() {
                Connection::open_with_flags_and_vfs(&self.path, flags, vfs)
                    .map_err(sqlite_error)?
            } else {
                Connection::open_with_flags(&self.path, flags).map_err(sqlite_error)?
            }

            #[cfg(not(test))]
            {
                Connection::open_with_flags(&self.path, flags).map_err(sqlite_error)?
            }
        };
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(sqlite_error)?;

        // WAL gives crash recovery for each database file; FULL adds the WAL
        // sync at transaction commit needed for durability across power loss.
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(sqlite_error)?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(sqlite_error)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(sqlite_error)?;

        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(TransparencyWitnessStoreError::Backend(format!(
                "SQLite did not retain required WAL journal mode: {journal_mode}"
            )));
        }

        let synchronous = connection
            .query_row("PRAGMA synchronous", [], |row| row.get::<_, i64>(0))
            .map_err(sqlite_error)?;
        if synchronous != 2 {
            return Err(TransparencyWitnessStoreError::Backend(format!(
                "SQLite did not retain required synchronous=FULL mode: {synchronous}"
            )));
        }

        let foreign_keys = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .map_err(sqlite_error)?;
        if foreign_keys != 1 {
            return Err(TransparencyWitnessStoreError::Backend(
                "SQLite did not retain required foreign_keys=ON mode".to_string(),
            ));
        }

        connection
            .execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {TABLE} (
                    policy_commitment TEXT NOT NULL,
                    log_authority_commitment TEXT NOT NULL,
                    state_json TEXT NOT NULL,
                    PRIMARY KEY (policy_commitment, log_authority_commitment)
                );"
            ))
            .map_err(sqlite_error)?;
        Self::validate_schema(&connection)?;
        Self::validate_schema_objects(&connection)?;
        Self::validate_integrity(&connection)?;

        Ok(connection)
    }

    fn validate_integrity(connection: &Connection) -> Result<(), TransparencyWitnessStoreError> {
        let result = connection
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        if result != "ok" {
            return Err(TransparencyWitnessStoreError::Invalid(format!(
                "SQLite integrity_check failed: {result}"
            )));
        }
        Ok(())
    }

    fn validate_schema(connection: &Connection) -> Result<(), TransparencyWitnessStoreError> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_xinfo({TABLE})"))
            .map_err(sqlite_error)?;
        let mut rows = statement.query([]).map_err(sqlite_error)?;
        let mut columns = Vec::new();

        while let Some(row) = rows.next().map_err(sqlite_error)? {
            let cid = row.get::<_, i64>(0).map_err(sqlite_error)?;
            let name = row.get::<_, String>(1).map_err(sqlite_error)?;
            let sql_type = row.get::<_, String>(2).map_err(sqlite_error)?;
            let not_null = row.get::<_, i64>(3).map_err(sqlite_error)?;
            let primary_key_position = row.get::<_, i64>(5).map_err(sqlite_error)?;
            columns.push((cid, name, sql_type, not_null, primary_key_position));
        }

        let expected = vec![
            (0_i64, "policy_commitment".to_string(), "TEXT".to_string(), 1_i64, 1_i64),
            (
                1_i64,
                "log_authority_commitment".to_string(),
                "TEXT".to_string(),
                1_i64,
                2_i64,
            ),
            (2_i64, "state_json".to_string(), "TEXT".to_string(), 1_i64, 0_i64),
        ];

        if columns != expected {
            return Err(TransparencyWitnessStoreError::Invalid(
                "SQLite witness-state table schema does not match the required atomic whole-snapshot layout"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn validate_schema_objects(
        connection: &Connection,
    ) -> Result<(), TransparencyWitnessStoreError> {
        let mut statement = connection
            .prepare(
                "SELECT type, name, tbl_name
                 FROM sqlite_master
                 WHERE tbl_name = ?1
                 ORDER BY type, name",
            )
            .map_err(sqlite_error)?;
        let objects = statement
            .query_map([TABLE], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        let expected = vec![(
            "table".to_string(),
            TABLE.to_string(),
            TABLE.to_string(),
        )];

        if objects != expected {
            return Err(TransparencyWitnessStoreError::Invalid(
                "SQLite witness-state schema contains unexpected database objects"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn decode_state(
        key: &TransparencyWitnessStoreKeyV1,
        json: &str,
    ) -> Result<TransparencyWitnessStoredStateV1, TransparencyWitnessStoreError> {
        let state = serde_json::from_str::<TransparencyWitnessStoredStateV1>(json).map_err(
            |error| {
                TransparencyWitnessStoreError::Invalid(format!(
                    "SQLite witness state JSON is invalid: {error}"
                ))
            },
        )?;
        state.validate_basic()?;

        if state.snapshot().policy_commitment() != key.policy_commitment()
            || state.snapshot().log_authority_commitment() != key.log_authority_commitment()
        {
            return Err(TransparencyWitnessStoreError::IdentityMismatch);
        }

        Ok(state)
    }

    fn load_in_transaction(
        transaction: &rusqlite::Transaction<'_>,
        key: &TransparencyWitnessStoreKeyV1,
    ) -> Result<Option<TransparencyWitnessStoredStateV1>, TransparencyWitnessStoreError> {
        let json = transaction
            .query_row(
                &format!(
                    "SELECT state_json FROM {TABLE}
                     WHERE policy_commitment = ?1 AND log_authority_commitment = ?2"
                ),
                params![key.policy_commitment(), key.log_authority_commitment()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)?;

        let Some(json) = json else {
            return Ok(None);
        };

        Ok(Some(Self::decode_state(key, &json)?))
    }

    fn encode_state(
        state: &TransparencyWitnessStoredStateV1,
    ) -> Result<String, TransparencyWitnessStoreError> {
        serde_json::to_string(state).map_err(|error| {
            TransparencyWitnessStoreError::Invalid(format!(
                "cannot serialize SQLite witness state: {error}"
            ))
        })
    }
}

impl TransparencyWitnessStateStore for SqliteTransparencyWitnessStateStore {
    fn load(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
    ) -> Result<Option<TransparencyWitnessStoredStateV1>, TransparencyWitnessStoreError> {
        key.validate_basic()?;
        let connection = self.connection()?;
        let json = connection
            .query_row(
                &format!(
                    "SELECT state_json FROM {TABLE}
                     WHERE policy_commitment = ?1 AND log_authority_commitment = ?2"
                ),
                params![key.policy_commitment(), key.log_authority_commitment()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)?;

        let Some(json) = json else {
            return Ok(None);
        };

        Ok(Some(Self::decode_state(key, &json)?))
    }

    fn compare_and_swap(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
        expected: Option<&TransparencyWitnessStoredStateV1>,
        replacement: TransparencyWitnessStateSnapshotV1,
    ) -> Result<TransparencyWitnessStoredStateV1, TransparencyWitnessStoreError> {
        key.validate_basic()?;
        replacement.validate_basic()?;

        if replacement.policy_commitment() != key.policy_commitment()
            || replacement.log_authority_commitment() != key.log_authority_commitment()
        {
            return Err(TransparencyWitnessStoreError::IdentityMismatch);
        }
        if let Some(expected) = expected {
            expected.validate_basic()?;
            if expected.snapshot().policy_commitment() != key.policy_commitment()
                || expected.snapshot().log_authority_commitment() != key.log_authority_commitment()
            {
                return Err(TransparencyWitnessStoreError::IdentityMismatch);
            }
        }

        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;

        let current = Self::load_in_transaction(&transaction, key)?;

        match (current.as_ref(), expected) {
            (None, None) => {}
            (Some(_), None) => {
                return Err(TransparencyWitnessStoreError::GenerationMismatch);
            }
            (None, Some(_)) => {
                return Err(TransparencyWitnessStoreError::GenerationMismatch);
            }
            (Some(current), Some(expected)) if current != expected => {
                return Err(TransparencyWitnessStoreError::GenerationMismatch);
            }
            (Some(_), Some(_)) => {}
        }

        let next_generation = expected
            .map(|state| {
                state
                    .generation()
                    .checked_add(1)
                    .ok_or(TransparencyWitnessStoreError::GenerationExhausted)
            })
            .transpose()?
            .unwrap_or(0);

        let stored = TransparencyWitnessStoredStateV1::new(next_generation, replacement.clone())?;
        let state_json = Self::encode_state(&stored)?;

        transaction
            .execute(
                &format!(
                    "INSERT INTO {TABLE} (
                        policy_commitment, log_authority_commitment, state_json
                     ) VALUES (?1, ?2, ?3)
                     ON CONFLICT(policy_commitment, log_authority_commitment)
                     DO UPDATE SET state_json = excluded.state_json"
                ),
                params![
                    key.policy_commitment(),
                    key.log_authority_commitment(),
                    state_json
                ],
            )
            .map_err(sqlite_error)?;

        if expected.is_some() {
            abort_for_test("after-write-before-commit");
        }

        let persisted_json = transaction
            .query_row(
                &format!(
                    "SELECT state_json FROM {TABLE}
                     WHERE policy_commitment = ?1 AND log_authority_commitment = ?2"
                ),
                params![key.policy_commitment(), key.log_authority_commitment()],
                |row| row.get::<_, String>(0),
            )
            .map_err(sqlite_error)?;
        let persisted = Self::decode_state(key, &persisted_json)?;
        if persisted != stored {
            return Err(TransparencyWitnessStoreError::Invalid(
                "SQLite witness-state postcondition mismatch before commit".to_string(),
            ));
        }

        transaction.commit().map_err(sqlite_error)?;

        validate_cas_result(key, expected, &replacement, &stored)?;
        Ok(stored)
    }
}

fn sqlite_error(error: rusqlite::Error) -> TransparencyWitnessStoreError {
    TransparencyWitnessStoreError::Backend(format!("SQLite error: {error}"))
}

fn abort_for_test(point: &str) {
    if cfg!(test)
        && std::env::var("SYMTROPY_SQLITE_ABORT_AT").ok().as_deref() == Some(point)
    {
        std::process::abort();
    }
}

#[cfg(test)]
mod sqlite_fault_vfs;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transparency::{TransparencyVdsTreeHeadV1, TransparencyWitnessRecordV1, TransparencyWitnessStateSnapshotV1};
    use crate::transparency_store::TransparencyWitnessStateStore;
    use std::process::Command;
    use std::sync::{Arc, Barrier};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_database_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "symtropy-{label}-{}-{nonce}.sqlite3",
            std::process::id()
        ))
    }

    fn fixture() -> (TransparencyWitnessStoreKeyV1, TransparencyWitnessStateSnapshotV1) {
        let policy_commitment = "11".repeat(32);
        let log_commitment = "22".repeat(32);
        let witness = TransparencyWitnessRecordV1::new(
            "w1",
            0,
            &"33".repeat(32),
            0,
            "GENESIS",
            TransparencyVdsTreeHeadV1::empty(),
        )
        .expect("witness");
        let snapshot = TransparencyWitnessStateSnapshotV1::new(
            policy_commitment.clone(),
            log_commitment.clone(),
            vec![witness],
        )
        .expect("snapshot");
        (
            TransparencyWitnessStoreKeyV1::new(policy_commitment, log_commitment).expect("key"),
            snapshot,
        )
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_database_path_is_rejected() {
        use std::os::unix::fs::symlink;

        let target = temp_database_path("nofollow-target");
        let link = temp_database_path("nofollow-link");
        let store = SqliteTransparencyWitnessStateStore::open(&target).expect("target open");
        drop(store);

        symlink(&target, &link).expect("symlink");
        let error = SqliteTransparencyWitnessStateStore::open(&link)
            .expect_err("symlinked database path must be rejected");
        assert!(matches!(
            error,
            TransparencyWitnessStoreError::Backend(message)
                if message.contains("SQLite error")
        ));

        cleanup(&link);
        cleanup(&target);
    }

    #[test]
    fn required_sqlite_durability_pragmas_are_retained() {
        let path = temp_database_path("pragma-contract");
        let store = SqliteTransparencyWitnessStateStore::open(&path).expect("open");
        let connection = store.connection().expect("reopen connection");

        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .expect("journal mode"),
            "wal"
        );
        assert_eq!(
            connection
                .query_row("PRAGMA synchronous", [], |row| row.get::<_, i64>(0))
                .expect("synchronous mode"),
            2
        );
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("foreign keys"),
            1
        );

        drop(connection);
        drop(store);
        cleanup(&path);
    }

    #[test]
    fn nonconforming_preexisting_schema_is_rejected() {
        let path = temp_database_path("schema-reject");
        {
            let connection = Connection::open(&path).expect("raw sqlite");
            connection
                .execute_batch(&format!(
                    "CREATE TABLE {TABLE} (
                        policy_commitment TEXT NOT NULL,
                        log_authority_commitment TEXT NOT NULL,
                        state_json TEXT NOT NULL
                    );"
                ))
                .expect("bad schema");
        }

        let error =
            SqliteTransparencyWitnessStateStore::open(&path).expect_err("bad schema must fail");
        assert!(matches!(
            error,
            TransparencyWitnessStoreError::Invalid(message)
                if message.contains("schema does not match")
        ));

        cleanup(&path);
    }

    #[test]
    fn unexpected_schema_objects_are_rejected_before_witness_state_is_trusted() {
        let path = temp_database_path("schema-object-reject");
        {
            let connection = Connection::open(&path).expect("raw sqlite");
            connection
                .execute_batch(&format!(
                    "CREATE TABLE {TABLE} (
                        policy_commitment TEXT NOT NULL,
                        log_authority_commitment TEXT NOT NULL,
                        state_json TEXT NOT NULL,
                        PRIMARY KEY (policy_commitment, log_authority_commitment)
                    );
                    CREATE TRIGGER witness_mutation
                    AFTER INSERT ON {TABLE}
                    BEGIN
                        UPDATE {TABLE}
                        SET state_json = '{"generation":999}'
                        WHERE rowid = NEW.rowid;
                    END;"
                ))
                .expect("hostile schema");
        }

        let error =
            SqliteTransparencyWitnessStateStore::open(&path).expect_err("trigger must be rejected");
        assert!(matches!(
            error,
            TransparencyWitnessStoreError::Invalid(message)
                if message.contains("unexpected database objects")
        ));

        cleanup(&path);
    }

    #[test]
    fn reopen_preserves_committed_witness_state() {
        let path = temp_database_path("reopen");
        let (key, snapshot) = fixture();

        {
            let store = SqliteTransparencyWitnessStateStore::open(&path).expect("open");
            let stored = store
                .compare_and_swap(&key, None, snapshot.clone())
                .expect("initial put");
            assert_eq!(stored.generation(), 0);
        }

        {
            let reopened = SqliteTransparencyWitnessStateStore::open(&path).expect("reopen");
            let stored = reopened
                .load(&key)
                .expect("load")
                .expect("persistent state");
            assert_eq!(stored.generation(), 0);
            assert_eq!(stored.snapshot(), &snapshot);
        }

        cleanup(&path);
    }

    #[test]
    fn sqlite_commit_survives_abrupt_process_exit() {
        const WORKER_ENV: &str = "SYMTROPY_SQLITE_CRASH_WORKER";

        if let Ok(path) = std::env::var(WORKER_ENV) {
            let path = PathBuf::from(path);
            let (key, snapshot) = fixture();
            let store = SqliteTransparencyWitnessStateStore::open(&path).expect("worker open");
            store
                .compare_and_swap(&key, None, snapshot)
                .expect("worker commit");
            std::process::exit(0);
        }

        let path = temp_database_path("process-exit");
        let status = Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "transparency_store_sqlite::tests::sqlite_commit_survives_abrupt_process_exit",
                "--nocapture",
            ])
            .env(WORKER_ENV, &path)
            .status()
            .expect("spawn worker");

        assert!(status.success(), "crash worker must exit successfully");

        let (key, snapshot) = fixture();
        let reopened =
            SqliteTransparencyWitnessStateStore::open(&path).expect("reopen after worker");
        let stored = reopened
            .load(&key)
            .expect("load after process crash")
            .expect("committed state");
        assert_eq!(stored.generation(), 0);
        assert_eq!(stored.snapshot(), &snapshot);

        cleanup(&path);
    }

    #[test]
    fn coherent_database_snapshot_restore_produces_valid_older_frontier() {
        let path = temp_database_path("rollback-source");
        let snapshot_path = temp_database_path("rollback-snapshot");
        let restored_path = temp_database_path("rollback-restored");
        let (key, initial) = fixture();

        let store = SqliteTransparencyWitnessStateStore::open(&path).expect("open");
        let committed = store
            .compare_and_swap(&key, None, initial.clone())
            .expect("initial commit");
        assert_eq!(committed.generation(), 0);
        drop(store);

        {
            let connection = Connection::open(&path).expect("snapshot connection");
            connection
                .execute("VACUUM INTO ?1", params![snapshot_path.to_string_lossy().as_ref()])
                .expect("coherent snapshot");
        }

        {
            let store = SqliteTransparencyWitnessStateStore::open(&path).expect("reopen");
            let advanced = store
                .compare_and_swap(&key, Some(&committed), initial.clone())
                .expect("advance after snapshot");
            assert_eq!(advanced.generation(), 1);
        }

        std::fs::copy(&snapshot_path, &restored_path).expect("restore coherent snapshot");

        let restored = SqliteTransparencyWitnessStateStore::open(&restored_path)
            .expect("open restored snapshot");
        let recovered = restored
            .load(&key)
            .expect("load restored snapshot")
            .expect("restored witness state");

        assert_eq!(recovered, committed);
        assert_eq!(recovered.generation(), 0);

        cleanup(&path);
        cleanup(&snapshot_path);
        cleanup(&restored_path);
    }

    #[test]
    fn injected_wal_write_failure_recovers_to_a_valid_frontier() {
        use crate::sqlite_fault_vfs::{arm_wal, activate_current_thread, FaultOperation};

        let path = temp_database_path("vfs-write-failure");
        let (key, initial) = fixture();
        let store = SqliteTransparencyWitnessStateStore::open(&path).expect("open");
        let committed = store
            .compare_and_swap(&key, None, initial.clone())
            .expect("initial commit");

        let _activation = activate_current_thread();
        let _fault = arm_wal(FaultOperation::Write, 1);

        let candidate = store.compare_and_swap(&key, Some(&committed), initial.clone());
        assert!(candidate.is_err(), "injected WAL write must surface as an error");
        assert!(crate::sqlite_fault_vfs::fired(), "WAL write fault must fire");

        drop(_fault);
        drop(_activation);

        let recovered = SqliteTransparencyWitnessStateStore::open(&path)
            .expect("reopen after injected write failure")
            .load(&key)
            .expect("load after injected write failure")
            .expect("committed frontier");
        assert!(matches!(recovered.generation(), 0 | 1));
        assert_eq!(recovered.snapshot().policy_commitment(), key.policy_commitment());
        assert_eq!(
            recovered.snapshot().log_authority_commitment(),
            key.log_authority_commitment()
        );

        cleanup(&path);
    }

    #[test]
    fn injected_wal_sync_failure_recovers_to_a_valid_frontier() {
        use crate::sqlite_fault_vfs::{arm_wal, activate_current_thread, FaultOperation};

        let path = temp_database_path("vfs-sync-failure");
        let (key, initial) = fixture();
        let store = SqliteTransparencyWitnessStateStore::open(&path).expect("open");
        let committed = store
            .compare_and_swap(&key, None, initial.clone())
            .expect("initial commit");

        let _activation = activate_current_thread();
        let _fault = arm_wal(FaultOperation::Sync, 1);

        let candidate = store.compare_and_swap(&key, Some(&committed), initial.clone());
        assert!(crate::sqlite_fault_vfs::fired(), "WAL sync fault must fire");

        drop(_fault);
        drop(_activation);

        let recovered = SqliteTransparencyWitnessStateStore::open(&path)
            .expect("reopen after injected sync failure")
            .load(&key)
            .expect("load after injected sync failure")
            .expect("recoverable frontier");
        assert!(matches!(recovered.generation(), 0 | 1));
        assert_eq!(recovered.snapshot(), &initial);

        if candidate.is_ok() {
            assert_eq!(recovered.generation(), 1);
        } else {
            assert!(recovered.generation() <= 1);
        }

        cleanup(&path);
    }

    #[test]
    fn abort_after_write_before_commit_recovers_previous_frontier() {
        const WORKER_ENV: &str = "SYMTROPY_SQLITE_ABORT_WORKER";
        const ABORT_POINT_ENV: &str = "SYMTROPY_SQLITE_ABORT_AT";

        if let Ok(path) = std::env::var(WORKER_ENV) {
            let path = PathBuf::from(path);
            let (key, initial) = fixture();
            let store = SqliteTransparencyWitnessStateStore::open(&path).expect("worker open");
            store
                .compare_and_swap(&key, None, initial)
                .expect("initial commit");

            let next = TransparencyWitnessStateSnapshotV1::new(
                key.policy_commitment().to_string(),
                key.log_authority_commitment().to_string(),
                vec![initial.witnesses()[0].clone()],
            )
            .expect("replacement");
            let _ = store.compare_and_swap(
                &key,
                Some(
                    &store
                        .load(&key)
                        .expect("load")
                        .expect("initial state"),
                ),
                next,
            );
            std::process::exit(0);
        }

        let path = temp_database_path("abort-before-commit");
        let (key, initial) = fixture();
        let store = SqliteTransparencyWitnessStateStore::open(&path).expect("open");
        let committed = store
            .compare_and_swap(&key, None, initial.clone())
            .expect("initial commit");
        assert_eq!(committed.generation(), 0);

        let status = Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "transparency_store_sqlite::tests::abort_after_write_before_commit_recovers_previous_frontier",
                "--nocapture",
            ])
            .env(WORKER_ENV, &path)
            .env(ABORT_POINT_ENV, "after-write-before-commit")
            .status()
            .expect("spawn abort worker");

        assert!(
            !status.success(),
            "abort worker must terminate before transaction commit"
        );

        let reopened =
            SqliteTransparencyWitnessStateStore::open(&path).expect("reopen after abort");
        let recovered = reopened
            .load(&key)
            .expect("load after abort")
            .expect("previous committed state");
        assert_eq!(recovered, committed);

        cleanup(&path);
    }

    #[test]
    fn stale_writer_is_rejected_across_independent_store_instances() {    #[test]
    fn stale_writer_is_rejected_across_independent_store_instances() {
        let path = temp_database_path("stale");
        let (key, initial) = fixture();
        let first = SqliteTransparencyWitnessStateStore::open(&path).expect("first");
        let second = SqliteTransparencyWitnessStateStore::open(&path).expect("second");

        let initial_state = first
            .compare_and_swap(&key, None, initial.clone())
            .expect("initial put");
        let second_view = second.load(&key).expect("load").expect("state");

        let replacement = TransparencyWitnessStateSnapshotV1::new(
            initial.policy_commitment().to_string(),
            initial.log_authority_commitment().to_string(),
            vec![initial.witnesses()[0].clone()],
        )
        .expect("replacement");

        let first_next = first
            .compare_and_swap(&key, Some(&initial_state), replacement.clone())
            .expect("advance");
        assert_eq!(first_next.generation(), 1);

        let stale = second.compare_and_swap(&key, Some(&second_view), initial);
        assert!(matches!(
            stale,
            Err(TransparencyWitnessStoreError::GenerationMismatch)
        ));

        cleanup(&path);
    }

    #[test]
    fn concurrent_creates_have_exactly_one_winner() {
        let path = temp_database_path("concurrent-create");
        let (key, snapshot) = fixture();
        let left = Arc::new(
            SqliteTransparencyWitnessStateStore::open(&path).expect("left store"),
        );
        let right = Arc::new(
            SqliteTransparencyWitnessStateStore::open(&path).expect("right store"),
        );
        let barrier = Arc::new(Barrier::new(3));

        let left_store = Arc::clone(&left);
        let left_key = key.clone();
        let left_snapshot = snapshot.clone();
        let left_barrier = Arc::clone(&barrier);
        let left_thread = std::thread::spawn(move || {
            left_barrier.wait();
            left_store.compare_and_swap(&left_key, None, left_snapshot)
        });

        let right_store = Arc::clone(&right);
        let right_key = key;
        let right_snapshot = snapshot;
        let right_barrier = Arc::clone(&barrier);
        let right_thread = std::thread::spawn(move || {
            right_barrier.wait();
            right_store.compare_and_swap(&right_key, None, right_snapshot)
        });

        barrier.wait();

        let results = [
            left_thread.join().expect("left join"),
            right_thread.join().expect("right join"),
        ];
        assert_eq!(
            results.iter().filter(|result| result.is_ok()).count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(TransparencyWitnessStoreError::GenerationMismatch)))
                .count(),
            1
        );

        cleanup(&path);
    }
}
