// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Atomic snapshots, append-only event journals, and crash-tail recovery.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use symtropy_game_state::{EventChain, EventEnvelope, StableId, StateError};

/// Current on-disk save schema.
pub const SAVE_SCHEMA_VERSION: u32 = 1;

/// Atomic world-state checkpoint anchored to an event-chain hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveSnapshot<S> {
    /// Persistence schema used by this file.
    pub schema_version: u32,
    /// Stable save-slot identity.
    pub save_id: StableId,
    /// Scenario or content version required to interpret the state.
    pub content_version: String,
    /// Authoritative simulation tick represented by the snapshot.
    pub simulation_tick: u64,
    /// Event-chain head included in the state, or `GENESIS`.
    pub event_head_hash: String,
    /// Typed authoritative world state.
    pub state: S,
}

impl<S> SaveSnapshot<S> {
    /// Creates a snapshot at the current event-chain head.
    pub fn new(
        save_id: StableId,
        content_version: impl Into<String>,
        simulation_tick: u64,
        event_head_hash: impl Into<String>,
        state: S,
    ) -> Self {
        Self {
            schema_version: SAVE_SCHEMA_VERSION,
            save_id,
            content_version: content_version.into(),
            simulation_tick,
            event_head_hash: event_head_hash.into(),
            state,
        }
    }
}

/// Result of reading an append-only journal after a possible process crash.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalLoad<T> {
    /// Verified chain reconstructed from complete records.
    pub chain: EventChain<T>,
    /// Bytes ignored from an incomplete final record.
    pub discarded_tail_bytes: usize,
}

/// Directory-backed save slot.
#[derive(Debug, Clone)]
pub struct SaveStore {
    root: PathBuf,
}

impl SaveStore {
    /// Creates or opens a save directory.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, PersistenceError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(PersistenceError::Io)?;
        Ok(Self { root })
    }

    /// Returns the directory containing this save slot.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Writes a complete snapshot through a synchronized temporary file and rename.
    pub fn write_snapshot<S: Serialize>(
        &self,
        snapshot: &SaveSnapshot<S>,
    ) -> Result<(), PersistenceError> {
        if snapshot.schema_version != SAVE_SCHEMA_VERSION {
            return Err(PersistenceError::UnsupportedSchema(snapshot.schema_version));
        }
        let final_path = self.root.join("snapshot.json");
        let temporary_path = self.root.join("snapshot.json.partial");
        let file = File::create(&temporary_path).map_err(PersistenceError::Io)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, snapshot).map_err(PersistenceError::Json)?;
        writer.write_all(b"\n").map_err(PersistenceError::Io)?;
        writer.flush().map_err(PersistenceError::Io)?;
        writer.get_ref().sync_all().map_err(PersistenceError::Io)?;
        fs::rename(&temporary_path, &final_path).map_err(PersistenceError::Io)?;
        sync_directory(&self.root)?;
        Ok(())
    }

    /// Reads and schema-checks the latest complete snapshot.
    pub fn read_snapshot<S: DeserializeOwned>(&self) -> Result<SaveSnapshot<S>, PersistenceError> {
        let file = File::open(self.root.join("snapshot.json")).map_err(PersistenceError::Io)?;
        let snapshot: SaveSnapshot<S> =
            serde_json::from_reader(BufReader::new(file)).map_err(PersistenceError::Json)?;
        if snapshot.schema_version != SAVE_SCHEMA_VERSION {
            return Err(PersistenceError::UnsupportedSchema(snapshot.schema_version));
        }
        Ok(snapshot)
    }

    /// Appends one already-hashed event and synchronizes it before returning.
    pub fn append_event<T: Serialize>(
        &self,
        event: &EventEnvelope<T>,
    ) -> Result<(), PersistenceError> {
        event.verify_hash().map_err(PersistenceError::State)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("journal.jsonl"))
            .map_err(PersistenceError::Io)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, event).map_err(PersistenceError::Json)?;
        writer.write_all(b"\n").map_err(PersistenceError::Io)?;
        writer.flush().map_err(PersistenceError::Io)?;
        writer.get_ref().sync_data().map_err(PersistenceError::Io)?;
        Ok(())
    }

    /// Loads complete journal records and discards only a malformed final partial line.
    pub fn load_journal<T: DeserializeOwned + Serialize>(
        &self,
        namespace: impl Into<String>,
        seed: u64,
    ) -> Result<JournalLoad<T>, PersistenceError> {
        let path = self.root.join("journal.jsonl");
        if !path.exists() {
            return Ok(JournalLoad {
                chain: EventChain::new(namespace, seed),
                discarded_tail_bytes: 0,
            });
        }
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(PersistenceError::Io)?
            .read_to_end(&mut bytes)
            .map_err(PersistenceError::Io)?;

        let ends_with_newline = bytes.last().is_none_or(|byte| *byte == b'\n');
        let mut events = Vec::new();
        let mut offset = 0usize;
        let mut discarded_tail_bytes = 0usize;
        for segment in bytes.split_inclusive(|byte| *byte == b'\n') {
            let line = segment.strip_suffix(b"\n").unwrap_or(segment);
            if line.is_empty() {
                offset += segment.len();
                continue;
            }
            match serde_json::from_slice::<EventEnvelope<T>>(line) {
                Ok(event) => events.push(event),
                Err(error) => {
                    let is_final_segment = offset + segment.len() == bytes.len();
                    if is_final_segment && !ends_with_newline {
                        discarded_tail_bytes = segment.len();
                        break;
                    }
                    return Err(PersistenceError::InvalidJournalRecord {
                        byte_offset: offset,
                        source: error,
                    });
                }
            }
            offset += segment.len();
        }

        let chain = EventChain::from_events(namespace, seed, events);
        chain.verify().map_err(PersistenceError::State)?;
        Ok(JournalLoad {
            chain,
            discarded_tail_bytes,
        })
    }

    /// Confirms that a snapshot's event anchor is present in the loaded journal.
    pub fn verify_snapshot_anchor<S, T>(
        &self,
        snapshot: &SaveSnapshot<S>,
        journal: &JournalLoad<T>,
    ) -> Result<(), PersistenceError> {
        if snapshot.event_head_hash == "GENESIS"
            || journal
                .chain
                .events()
                .iter()
                .any(|event| event.event_hash == snapshot.event_head_hash)
        {
            Ok(())
        } else {
            Err(PersistenceError::MissingSnapshotAnchor(
                snapshot.event_head_hash.clone(),
            ))
        }
    }
}

fn sync_directory(path: &Path) -> Result<(), PersistenceError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(PersistenceError::Io)?;
    }
    Ok(())
}

/// Persistence and recovery failures.
#[derive(Debug)]
pub enum PersistenceError {
    /// Filesystem operation failed.
    Io(io::Error),
    /// JSON encoding or decoding failed.
    Json(serde_json::Error),
    /// State-chain validation failed.
    State(StateError),
    /// Snapshot uses an unsupported schema version.
    UnsupportedSchema(u32),
    /// A complete journal record was malformed.
    InvalidJournalRecord {
        byte_offset: usize,
        source: serde_json::Error,
    },
    /// Snapshot refers to an event not present in the recovered journal.
    MissingSnapshotAnchor(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "persistence I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "persistence JSON failed: {error}"),
            Self::State(error) => write!(formatter, "persisted state failed verification: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported save schema {version}")
            }
            Self::InvalidJournalRecord {
                byte_offset,
                source,
            } => write!(
                formatter,
                "invalid complete journal record at byte {byte_offset}: {source}"
            ),
            Self::MissingSnapshotAnchor(hash) => {
                write!(formatter, "snapshot anchor is absent from journal: {hash}")
            }
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::State(error) => Some(error),
            Self::InvalidJournalRecord { source, .. } => Some(source),
            Self::UnsupportedSchema(_) | Self::MissingSnapshotAnchor(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestState {
        gate_open: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestEvent {
        action: String,
    }

    fn temporary_store(name: &str) -> SaveStore {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("symtropy-{name}-{}-{nonce}", std::process::id()));
        SaveStore::open(root).expect("create temporary save store")
    }

    #[test]
    fn snapshot_round_trips_atomically() {
        let store = temporary_store("snapshot");
        let snapshot = SaveSnapshot::new(
            StableId::parse("save:firstlight:test").expect("valid id"),
            "firstlight-v0.1",
            42,
            "GENESIS",
            TestState { gate_open: true },
        );
        store.write_snapshot(&snapshot).expect("write snapshot");
        let loaded: SaveSnapshot<TestState> = store.read_snapshot().expect("read snapshot");
        assert_eq!(loaded, snapshot);
        assert!(!store.root().join("snapshot.json.partial").exists());
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }

    #[test]
    fn incomplete_final_record_is_discarded() {
        let store = temporary_store("tail");
        let mut chain = EventChain::new("journal", 7);
        chain
            .append(
                1,
                "repair",
                None,
                None,
                Vec::new(),
                TestEvent {
                    action: "isolate".into(),
                },
            )
            .expect("append event");
        store
            .append_event(&chain.events()[0])
            .expect("persist event");
        let mut file = OpenOptions::new()
            .append(true)
            .open(store.root().join("journal.jsonl"))
            .expect("open journal");
        file.write_all(b"{\"partial\":").expect("write crash tail");
        file.sync_all().expect("sync crash tail");

        let loaded: JournalLoad<TestEvent> =
            store.load_journal("journal", 7).expect("recover journal");
        assert_eq!(loaded.chain.events().len(), 1);
        assert!(loaded.discarded_tail_bytes > 0);
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }

    #[test]
    fn snapshot_anchor_must_exist() {
        let store = temporary_store("anchor");
        let snapshot = SaveSnapshot::new(
            StableId::parse("save:firstlight:anchor").expect("valid id"),
            "firstlight-v0.1",
            8,
            "missing",
            TestState { gate_open: false },
        );
        let journal: JournalLoad<TestEvent> = JournalLoad {
            chain: EventChain::new("journal", 1),
            discarded_tail_bytes: 0,
        };
        assert!(matches!(
            store.verify_snapshot_anchor(&snapshot, &journal),
            Err(PersistenceError::MissingSnapshotAnchor(_))
        ));
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }
}
