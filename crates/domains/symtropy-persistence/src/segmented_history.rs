// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Immutable segmented event journals for long-lived causal history.
//!
//! Segmentation is a storage concern only. Loaded segments are reassembled into
//! one `EventChain` and verified through `symtropy-game-state`, so splitting or
//! archiving files cannot silently create a second historical identity model.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};
use symtropy_game_state::{EventChain, EventEnvelope, StableId, StateError};

/// On-disk schema for immutable history-segment manifests.
pub const SEGMENTED_HISTORY_SCHEMA_VERSION: u32 = 1;

/// Immutable metadata binding one storage segment to the global event chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalSegmentManifest {
    /// Manifest schema used by this segment.
    pub schema_version: u32,
    /// Zero-based contiguous segment index.
    pub segment_index: u64,
    /// Number of complete events stored in the segment.
    pub event_count: u64,
    /// First event identity in the segment.
    pub first_event_id: StableId,
    /// Last event identity in the segment.
    pub last_event_id: StableId,
    /// First canonical tick represented by the segment.
    pub first_tick: u64,
    /// Last canonical tick represented by the segment.
    pub last_tick: u64,
    /// Event-chain head immediately before this segment begins.
    pub previous_segment_head_hash: String,
    /// Event-chain head after this segment is complete.
    pub segment_head_hash: String,
}

/// Result of loading and verifying all immutable history segments.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentedJournalLoad<T> {
    /// Reconstructed single authoritative event chain.
    pub chain: EventChain<T>,
    /// Verified storage manifests in segment order.
    pub segments: Vec<JournalSegmentManifest>,
}

/// Directory-backed immutable segmented history store.
#[derive(Debug, Clone)]
pub struct SegmentedJournalStore {
    root: PathBuf,
}

impl SegmentedJournalStore {
    /// Creates or opens the segmented-history directory rooted under `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, SegmentedHistoryError> {
        let root = root.into();
        fs::create_dir_all(root.join("segments")).map_err(SegmentedHistoryError::Io)?;
        Ok(Self { root })
    }

    /// Returns the save/history root containing the `segments/` directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Writes one immutable contiguous segment.
    ///
    /// Segment zero must begin at `GENESIS`. Every later segment must reference
    /// the exact verified head of the immediately preceding manifest. Existing
    /// segment indices are immutable and cannot be overwritten.
    pub fn write_segment<T: Serialize>(
        &self,
        segment_index: u64,
        previous_segment_head_hash: &str,
        events: &[EventEnvelope<T>],
    ) -> Result<JournalSegmentManifest, SegmentedHistoryError> {
        if events.is_empty() {
            return Err(SegmentedHistoryError::EmptySegment(segment_index));
        }

        let (data_path, manifest_path) = self.segment_paths(segment_index);
        if data_path.exists() || manifest_path.exists() {
            return Err(SegmentedHistoryError::SegmentAlreadyExists(segment_index));
        }

        let expected_previous_head = if segment_index == 0 {
            "GENESIS".to_owned()
        } else {
            let previous = self.read_manifest(segment_index - 1)?;
            previous.segment_head_hash
        };
        if previous_segment_head_hash != expected_previous_head {
            return Err(SegmentedHistoryError::PreviousHeadMismatch {
                segment_index,
                expected: expected_previous_head,
                actual: previous_segment_head_hash.to_owned(),
            });
        }

        self.validate_segment_links(segment_index, previous_segment_head_hash, events)?;

        let event_count = u64::try_from(events.len()).map_err(|_| {
            SegmentedHistoryError::InvalidSegment {
                segment_index,
                reason: "event count exceeds u64".into(),
            }
        })?;
        let first = &events[0];
        let last = &events[events.len() - 1];
        let manifest = JournalSegmentManifest {
            schema_version: SEGMENTED_HISTORY_SCHEMA_VERSION,
            segment_index,
            event_count,
            first_event_id: first.event_id.clone(),
            last_event_id: last.event_id.clone(),
            first_tick: first.simulation_tick,
            last_tick: last.simulation_tick,
            previous_segment_head_hash: previous_segment_head_hash.to_owned(),
            segment_head_hash: last.event_hash.clone(),
        };

        let segments_dir = self.root.join("segments");
        let data_partial = data_path.with_extension("jsonl.partial");
        let manifest_partial = manifest_path.with_extension("manifest.json.partial");

        {
            let file = File::create(&data_partial).map_err(SegmentedHistoryError::Io)?;
            let mut writer = BufWriter::new(file);
            for event in events {
                serde_json::to_writer(&mut writer, event).map_err(SegmentedHistoryError::Json)?;
                writer.write_all(b"\n").map_err(SegmentedHistoryError::Io)?;
            }
            writer.flush().map_err(SegmentedHistoryError::Io)?;
            writer.get_ref().sync_all().map_err(SegmentedHistoryError::Io)?;
        }

        {
            let file = File::create(&manifest_partial).map_err(SegmentedHistoryError::Io)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &manifest)
                .map_err(SegmentedHistoryError::Json)?;
            writer.write_all(b"\n").map_err(SegmentedHistoryError::Io)?;
            writer.flush().map_err(SegmentedHistoryError::Io)?;
            writer.get_ref().sync_all().map_err(SegmentedHistoryError::Io)?;
        }

        // Data becomes visible first; the manifest is the commit record used by
        // readers. A crash between renames may leave an unreferenced data file,
        // but cannot make an incomplete segment authoritative.
        fs::rename(&data_partial, &data_path).map_err(SegmentedHistoryError::Io)?;
        fs::rename(&manifest_partial, &manifest_path).map_err(SegmentedHistoryError::Io)?;
        sync_directory(&segments_dir)?;

        Ok(manifest)
    }

    /// Loads every committed segment and reconstructs one verified event chain.
    pub fn load<T: DeserializeOwned + Serialize>(
        &self,
        namespace: impl Into<String>,
        seed: u64,
    ) -> Result<SegmentedJournalLoad<T>, SegmentedHistoryError> {
        let mut manifests = self.list_manifests()?;
        manifests.sort_by_key(|manifest| manifest.segment_index);

        for (expected_index, manifest) in manifests.iter().enumerate() {
            let expected_index = u64::try_from(expected_index).map_err(|_| {
                SegmentedHistoryError::InvalidSegment {
                    segment_index: manifest.segment_index,
                    reason: "segment index exceeds u64".into(),
                }
            })?;
            if manifest.segment_index != expected_index {
                return Err(SegmentedHistoryError::NonContiguousSegments {
                    expected: expected_index,
                    actual: manifest.segment_index,
                });
            }
        }

        let mut all_events = Vec::new();
        let mut expected_previous_head = "GENESIS".to_owned();
        for manifest in &manifests {
            if manifest.schema_version != SEGMENTED_HISTORY_SCHEMA_VERSION {
                return Err(SegmentedHistoryError::UnsupportedSchema(
                    manifest.schema_version,
                ));
            }
            if manifest.previous_segment_head_hash != expected_previous_head {
                return Err(SegmentedHistoryError::PreviousHeadMismatch {
                    segment_index: manifest.segment_index,
                    expected: expected_previous_head,
                    actual: manifest.previous_segment_head_hash.clone(),
                });
            }

            let events = self.read_segment::<T>(manifest.segment_index)?;
            self.validate_manifest(manifest, &events)?;
            self.validate_segment_links(
                manifest.segment_index,
                &manifest.previous_segment_head_hash,
                &events,
            )?;
            expected_previous_head = manifest.segment_head_hash.clone();
            all_events.extend(events);
        }

        let chain = EventChain::from_events(namespace, seed, all_events);
        chain.verify().map_err(SegmentedHistoryError::State)?;
        Ok(SegmentedJournalLoad {
            chain,
            segments: manifests,
        })
    }

    fn validate_segment_links<T: Serialize>(
        &self,
        segment_index: u64,
        previous_segment_head_hash: &str,
        events: &[EventEnvelope<T>],
    ) -> Result<(), SegmentedHistoryError> {
        let Some(first) = events.first() else {
            return Err(SegmentedHistoryError::EmptySegment(segment_index));
        };
        if first.previous_hash != previous_segment_head_hash {
            return Err(SegmentedHistoryError::InvalidSegment {
                segment_index,
                reason: format!(
                    "first event previous hash {} does not match segment anchor {}",
                    first.previous_hash, previous_segment_head_hash
                ),
            });
        }

        let mut previous_tick = first.simulation_tick;
        for (offset, event) in events.iter().enumerate() {
            event.verify_hash().map_err(SegmentedHistoryError::State)?;
            if offset > 0 && event.simulation_tick < previous_tick {
                return Err(SegmentedHistoryError::InvalidSegment {
                    segment_index,
                    reason: format!(
                        "event {} moves backward from tick {} to {}",
                        event.event_id, previous_tick, event.simulation_tick
                    ),
                });
            }
            if offset > 0 {
                let prior = &events[offset - 1];
                if event.previous_hash != prior.event_hash {
                    return Err(SegmentedHistoryError::InvalidSegment {
                        segment_index,
                        reason: format!(
                            "event {} does not link to prior event {}",
                            event.event_id, prior.event_id
                        ),
                    });
                }
            }
            previous_tick = event.simulation_tick;
        }
        Ok(())
    }

    fn validate_manifest<T>(
        &self,
        manifest: &JournalSegmentManifest,
        events: &[EventEnvelope<T>],
    ) -> Result<(), SegmentedHistoryError> {
        if events.is_empty() {
            return Err(SegmentedHistoryError::EmptySegment(manifest.segment_index));
        }
        let actual_count = u64::try_from(events.len()).map_err(|_| {
            SegmentedHistoryError::InvalidSegment {
                segment_index: manifest.segment_index,
                reason: "event count exceeds u64".into(),
            }
        })?;
        let first = &events[0];
        let last = &events[events.len() - 1];
        let matches = manifest.event_count == actual_count
            && manifest.first_event_id == first.event_id
            && manifest.last_event_id == last.event_id
            && manifest.first_tick == first.simulation_tick
            && manifest.last_tick == last.simulation_tick
            && manifest.segment_head_hash == last.event_hash;
        if matches {
            Ok(())
        } else {
            Err(SegmentedHistoryError::InvalidSegment {
                segment_index: manifest.segment_index,
                reason: "manifest does not match immutable segment contents".into(),
            })
        }
    }

    fn list_manifests(&self) -> Result<Vec<JournalSegmentManifest>, SegmentedHistoryError> {
        let segments_dir = self.root.join("segments");
        let mut manifests = Vec::new();
        for entry in fs::read_dir(segments_dir).map_err(SegmentedHistoryError::Io)? {
            let entry = entry.map_err(SegmentedHistoryError::Io)?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.ends_with(".manifest.json") {
                continue;
            }
            let file = File::open(path).map_err(SegmentedHistoryError::Io)?;
            let manifest: JournalSegmentManifest =
                serde_json::from_reader(BufReader::new(file)).map_err(SegmentedHistoryError::Json)?;
            manifests.push(manifest);
        }
        Ok(manifests)
    }

    fn read_manifest(
        &self,
        segment_index: u64,
    ) -> Result<JournalSegmentManifest, SegmentedHistoryError> {
        let (_, manifest_path) = self.segment_paths(segment_index);
        if !manifest_path.exists() {
            return Err(SegmentedHistoryError::MissingPreviousSegment(segment_index));
        }
        let file = File::open(manifest_path).map_err(SegmentedHistoryError::Io)?;
        let manifest: JournalSegmentManifest =
            serde_json::from_reader(BufReader::new(file)).map_err(SegmentedHistoryError::Json)?;
        if manifest.schema_version != SEGMENTED_HISTORY_SCHEMA_VERSION {
            return Err(SegmentedHistoryError::UnsupportedSchema(
                manifest.schema_version,
            ));
        }
        if manifest.segment_index != segment_index {
            return Err(SegmentedHistoryError::InvalidSegment {
                segment_index,
                reason: "manifest index does not match filename/index requested".into(),
            });
        }
        Ok(manifest)
    }

    fn read_segment<T: DeserializeOwned>(
        &self,
        segment_index: u64,
    ) -> Result<Vec<EventEnvelope<T>>, SegmentedHistoryError> {
        let (data_path, _) = self.segment_paths(segment_index);
        let file = File::open(data_path).map_err(SegmentedHistoryError::Io)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (line_number, line) in reader.lines().enumerate() {
            let line = line.map_err(SegmentedHistoryError::Io)?;
            if line.is_empty() {
                continue;
            }
            let event = serde_json::from_str(&line).map_err(|source| {
                SegmentedHistoryError::InvalidSegmentRecord {
                    segment_index,
                    line_number: line_number + 1,
                    source,
                }
            })?;
            events.push(event);
        }
        Ok(events)
    }

    fn segment_paths(&self, segment_index: u64) -> (PathBuf, PathBuf) {
        let stem = format!("{segment_index:020}");
        let directory = self.root.join("segments");
        (
            directory.join(format!("{stem}.jsonl")),
            directory.join(format!("{stem}.manifest.json")),
        )
    }
}

/// Failures produced by immutable segmented history storage.
#[derive(Debug)]
pub enum SegmentedHistoryError {
    /// Filesystem operation failed.
    Io(std::io::Error),
    /// Manifest or event JSON encoding/decoding failed.
    Json(serde_json::Error),
    /// Reconstructed game-state chain failed verification.
    State(StateError),
    /// Manifest uses an unsupported schema.
    UnsupportedSchema(u32),
    /// Empty segments are never committed.
    EmptySegment(u64),
    /// Segment indices are immutable once committed.
    SegmentAlreadyExists(u64),
    /// A later segment was attempted before its predecessor was committed.
    MissingPreviousSegment(u64),
    /// A segment did not anchor to the exact preceding segment head.
    PreviousHeadMismatch {
        segment_index: u64,
        expected: String,
        actual: String,
    },
    /// Stored manifest indices contained a gap or started after zero.
    NonContiguousSegments { expected: u64, actual: u64 },
    /// Segment content and/or link semantics were invalid.
    InvalidSegment { segment_index: u64, reason: String },
    /// A complete immutable segment contained malformed JSON.
    InvalidSegmentRecord {
        segment_index: u64,
        line_number: usize,
        source: serde_json::Error,
    },
}

impl fmt::Display for SegmentedHistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "segmented-history I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "segmented-history JSON failed: {error}"),
            Self::State(error) => write!(formatter, "segmented history failed state verification: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported segmented-history schema {version}")
            }
            Self::EmptySegment(index) => write!(formatter, "history segment {index} is empty"),
            Self::SegmentAlreadyExists(index) => {
                write!(formatter, "history segment {index} already exists and is immutable")
            }
            Self::MissingPreviousSegment(index) => {
                write!(formatter, "required previous history segment {index} is missing")
            }
            Self::PreviousHeadMismatch {
                segment_index,
                expected,
                actual,
            } => write!(
                formatter,
                "history segment {segment_index} previous head mismatch: expected {expected}, got {actual}"
            ),
            Self::NonContiguousSegments { expected, actual } => write!(
                formatter,
                "history segments are not contiguous: expected {expected}, found {actual}"
            ),
            Self::InvalidSegment {
                segment_index,
                reason,
            } => write!(formatter, "history segment {segment_index} is invalid: {reason}"),
            Self::InvalidSegmentRecord {
                segment_index,
                line_number,
                source,
            } => write!(
                formatter,
                "history segment {segment_index} contains invalid record at line {line_number}: {source}"
            ),
        }
    }
}

impl Error for SegmentedHistoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::State(error) => Some(error),
            Self::InvalidSegmentRecord { source, .. } => Some(source),
            Self::UnsupportedSchema(_)
            | Self::EmptySegment(_)
            | Self::SegmentAlreadyExists(_)
            | Self::MissingPreviousSegment(_)
            | Self::PreviousHeadMismatch { .. }
            | Self::NonContiguousSegments { .. }
            | Self::InvalidSegment { .. } => None,
        }
    }
}

#[cfg_attr(not(unix), allow(unused_variables))]
fn sync_directory(path: &Path) -> Result<(), SegmentedHistoryError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(SegmentedHistoryError::Io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestEvent {
        value: u32,
    }

    fn temporary_store(name: &str) -> SegmentedJournalStore {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "symtropy-segmented-{name}-{}-{nonce}",
            std::process::id()
        ));
        SegmentedJournalStore::open(root).expect("create segmented history store")
    }

    fn four_event_chain() -> EventChain<TestEvent> {
        let mut chain = EventChain::new("history", 23);
        let first = chain
            .append(1, "one", None, None, Vec::new(), TestEvent { value: 1 })
            .expect("append first");
        let second = chain
            .append(
                2,
                "two",
                None,
                None,
                vec![first.clone()],
                TestEvent { value: 2 },
            )
            .expect("append second");
        chain
            .append(
                3,
                "three",
                None,
                None,
                vec![second.clone()],
                TestEvent { value: 3 },
            )
            .expect("append third");
        chain
            .append(
                4,
                "four",
                None,
                None,
                vec![first, second],
                TestEvent { value: 4 },
            )
            .expect("append fourth");
        chain
    }

    #[test]
    fn two_segments_reconstruct_one_verified_history() {
        let store = temporary_store("roundtrip");
        let chain = four_event_chain();
        let first_manifest = store
            .write_segment(0, "GENESIS", &chain.events()[..2])
            .expect("write first segment");
        store
            .write_segment(
                1,
                &first_manifest.segment_head_hash,
                &chain.events()[2..],
            )
            .expect("write second segment");

        let loaded: SegmentedJournalLoad<TestEvent> =
            store.load("history", 23).expect("load segmented history");
        assert_eq!(loaded.chain, chain);
        assert_eq!(loaded.segments.len(), 2);
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }

    #[test]
    fn later_segment_requires_exact_previous_head() {
        let store = temporary_store("anchor");
        let chain = four_event_chain();
        store
            .write_segment(0, "GENESIS", &chain.events()[..2])
            .expect("write first segment");
        assert!(matches!(
            store.write_segment(1, "wrong-head", &chain.events()[2..]),
            Err(SegmentedHistoryError::PreviousHeadMismatch { .. })
        ));
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }

    #[test]
    fn committed_segment_index_is_immutable() {
        let store = temporary_store("immutable");
        let chain = four_event_chain();
        store
            .write_segment(0, "GENESIS", &chain.events()[..2])
            .expect("write first segment");
        assert!(matches!(
            store.write_segment(0, "GENESIS", &chain.events()[..2]),
            Err(SegmentedHistoryError::SegmentAlreadyExists(0))
        ));
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }

    #[test]
    fn tampered_manifest_fails_closed() {
        let store = temporary_store("tamper");
        let chain = four_event_chain();
        store
            .write_segment(0, "GENESIS", &chain.events()[..2])
            .expect("write first segment");
        let (_, manifest_path) = store.segment_paths(0);
        let mut manifest = store.read_manifest(0).expect("read manifest");
        manifest.segment_head_hash = "tampered".into();
        let file = File::create(&manifest_path).expect("open manifest for tamper fixture");
        serde_json::to_writer_pretty(BufWriter::new(file), &manifest)
            .expect("rewrite tampered manifest");

        let result: Result<SegmentedJournalLoad<TestEvent>, _> = store.load("history", 23);
        assert!(matches!(
            result,
            Err(SegmentedHistoryError::InvalidSegment { .. })
        ));
        fs::remove_dir_all(store.root()).expect("remove temporary store");
    }
}
