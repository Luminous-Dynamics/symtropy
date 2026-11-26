// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only source/partition watermark and causal-cut validator.
//!
//! This module validates the internal consistency of a declared evidence cut.
//! It does not prove that the source roster is complete, authenticate source
//! checkpoints, verify receiver receipts, or qualify physical observations.
//! Those facts must come from owner-native and independently qualified inputs.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCut {
    pub watermarks: Vec<PartitionWatermark>,
    pub events: Vec<CutEvent>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PartitionWatermark {
    pub source_id: String,
    pub partition_id: String,
    pub stream_epoch: String,
    pub last_contiguous_sequence: u64,
    pub receiver_inclusion_offset: u64,
    pub checkpoint_digest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CutEvent {
    pub event_ref: String,
    pub source_id: String,
    pub partition_id: String,
    pub stream_epoch: String,
    pub sequence: u64,
    pub receiver_inclusion_offset: u64,
    #[serde(default)]
    pub dependencies: Vec<EventDependency>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EventDependency {
    pub event_ref: String,
    pub source_id: String,
    pub partition_id: String,
    pub stream_epoch: String,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CutStatus {
    Closed,
    Incomplete,
    Unverified,
    Invalid,
}

impl CutStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "CAUSALLY_CLOSED",
            Self::Incomplete => "INCOMPLETE",
            Self::Unverified => "UNVERIFIED",
            Self::Invalid => "INVALID",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CutAssessment {
    pub status: CutStatus,
    pub reason_codes: Vec<String>,
    pub watermark_count: usize,
    pub event_count: usize,
    pub dependency_count: usize,
}

impl CutAssessment {
    fn new(
        status: CutStatus,
        reasons: impl IntoIterator<Item = String>,
        watermark_count: usize,
        event_count: usize,
        dependency_count: usize,
    ) -> Self {
        Self {
            status,
            reason_codes: reasons.into_iter().collect::<BTreeSet<_>>().into_iter().collect(),
            watermark_count,
            event_count,
            dependency_count,
        }
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn partition_key(source_id: &str, partition_id: &str) -> (String, String) {
    (source_id.to_owned(), partition_id.to_owned())
}

fn event_coordinate_matches(
    event: &CutEvent,
    dependency: &EventDependency,
) -> bool {
    event.event_ref == dependency.event_ref
        && event.source_id == dependency.source_id
        && event.partition_id == dependency.partition_id
        && event.stream_epoch == dependency.stream_epoch
        && event.sequence == dependency.sequence
}

fn invalid_assessment(cut: &EvidenceCut, reasons: Vec<String>, dependencies: usize) -> CutAssessment {
    CutAssessment::new(
        CutStatus::Invalid,
        reasons,
        cut.watermarks.len(),
        cut.events.len(),
        dependencies,
    )
}

/// Validate that all declared events and their causal predecessors fit inside
/// one source/partition-scoped evidence cut.
///
/// Status precedence is fail-closed:
/// - Invalid: contradictory identifiers, malformed checkpoint digest, cycle,
///   or an event asserted as included beyond its own watermark.
/// - Unverified: no watermark or a different stream epoch for a dependency.
/// - Incomplete: a required predecessor is absent or its sequence is beyond
///   the available contiguous watermark.
/// - Closed: every event and dependency is covered by the declared vector.
pub fn validate_causal_cut(cut: &EvidenceCut) -> CutAssessment {
    let dependency_count = cut
        .events
        .iter()
        .map(|event| event.dependencies.len())
        .sum::<usize>();

    if cut.watermarks.is_empty() {
        return CutAssessment::new(
            CutStatus::Unverified,
            ["EVIDENCE_WATERMARK_VECTOR_EMPTY".to_owned()],
            0,
            cut.events.len(),
            dependency_count,
        );
    }

    let mut invalid = Vec::new();
    let mut unverified = Vec::new();
    let mut incomplete = Vec::new();

    let mut watermarks = BTreeMap::new();
    for watermark in &cut.watermarks {
        if !nonempty(&watermark.source_id)
            || !nonempty(&watermark.partition_id)
            || !nonempty(&watermark.stream_epoch)
        {
            invalid.push("WATERMARK_IDENTITY_MISSING_OR_NONCANONICAL".to_owned());
            continue;
        }
        if !valid_sha256(&watermark.checkpoint_digest_sha256) {
            invalid.push("WATERMARK_CHECKPOINT_DIGEST_INVALID".to_owned());
            continue;
        }
        let key = partition_key(&watermark.source_id, &watermark.partition_id);
        if watermarks.insert(key, watermark).is_some() {
            invalid.push("DUPLICATE_PARTITION_WATERMARK".to_owned());
        }
    }

    let mut events_by_ref = BTreeMap::new();
    let mut sequence_owners = BTreeMap::new();

    for event in &cut.events {
        if !nonempty(&event.event_ref)
            || !nonempty(&event.source_id)
            || !nonempty(&event.partition_id)
            || !nonempty(&event.stream_epoch)
            || event.sequence == 0
        {
            invalid.push("EVENT_IDENTITY_MISSING_OR_NONCANONICAL".to_owned());
            continue;
        }

        if events_by_ref.insert(event.event_ref.clone(), event).is_some() {
            invalid.push("DUPLICATE_EVENT_REFERENCE".to_owned());
            continue;
        }

        let key = (
            event.source_id.clone(),
            event.partition_id.clone(),
            event.stream_epoch.clone(),
            event.sequence,
        );
        if sequence_owners.insert(key, event.event_ref.clone()).is_some() {
            invalid.push("SOURCE_SEQUENCE_FORK".to_owned());
        }
    }

    if !invalid.is_empty() {
        return invalid_assessment(cut, invalid, dependency_count);
    }

    for event in &cut.events {
        let key = partition_key(&event.source_id, &event.partition_id);
        let Some(watermark) = watermarks.get(&key) else {
            unverified.push("EVENT_PARTITION_WATERMARK_MISSING".to_owned());
            continue;
        };

        if watermark.stream_epoch != event.stream_epoch {
            unverified.push("EVENT_STREAM_EPOCH_MISMATCH".to_owned());
            continue;
        }

        if event.sequence > watermark.last_contiguous_sequence
            || event.receiver_inclusion_offset > watermark.receiver_inclusion_offset
        {
            invalid.push("EVENT_ASSERTED_INCLUDED_BEYOND_WATERMARK".to_owned());
        }
    }

    let mut indegree = BTreeMap::<String, usize>::new();
    let mut dependents = BTreeMap::<String, Vec<String>>::new();

    for event in &cut.events {
        indegree.insert(event.event_ref.clone(), 0);
    }

    for event in &cut.events {
        for dependency in &event.dependencies {
            let Some(parent) = events_by_ref.get(&dependency.event_ref) else {
                incomplete.push("CAUSAL_PREDECESSOR_MISSING_FROM_EVIDENCE_VIEW".to_owned());
                continue;
            };

            if !event_coordinate_matches(parent, dependency) {
                invalid.push("CAUSAL_PREDECESSOR_IDENTITY_MISMATCH".to_owned());
                continue;
            }

            let key = partition_key(&dependency.source_id, &dependency.partition_id);
            let Some(watermark) = watermarks.get(&key) else {
                unverified.push("DEPENDENCY_PARTITION_WATERMARK_MISSING".to_owned());
                continue;
            };

            if watermark.stream_epoch != dependency.stream_epoch {
                unverified.push("DEPENDENCY_STREAM_EPOCH_MISMATCH".to_owned());
                continue;
            }

            if dependency.sequence > watermark.last_contiguous_sequence {
                incomplete.push("CAUSAL_PREDECESSOR_AFTER_CONTIGUOUS_WATERMARK".to_owned());
                continue;
            }

            if let Some(degree) = indegree.get_mut(&event.event_ref) {
                *degree += 1;
            }
            dependents
                .entry(dependency.event_ref.clone())
                .or_default()
                .push(event.event_ref.clone());
        }
    }

    if !invalid.is_empty() {
        return invalid_assessment(cut, invalid, dependency_count);
    }

    if !unverified.is_empty() {
        return CutAssessment::new(
            CutStatus::Unverified,
            unverified,
            cut.watermarks.len(),
            cut.events.len(),
            dependency_count,
        );
    }

    if !incomplete.is_empty() {
        return CutAssessment::new(
            CutStatus::Incomplete,
            incomplete,
            cut.watermarks.len(),
            cut.events.len(),
            dependency_count,
        );
    }

    // Kahn's algorithm detects cycles without recursion or input-order
    // dependence. The graph is small enough to keep an explicit adjacency map.
    let mut ready = VecDeque::new();
    for (event_ref, degree) in &indegree {
        if *degree == 0 {
            ready.push_back(event_ref.clone());
        }
    }

    let mut visited = 0usize;
    while let Some(event_ref) = ready.pop_front() {
        visited += 1;
        if let Some(children) = dependents.get(&event_ref) {
            for child in children {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push_back(child.clone());
                    }
                }
            }
        }
    }

    if visited != cut.events.len() {
        return invalid_assessment(
            cut,
            vec!["CAUSAL_DEPENDENCY_CYCLE".to_owned()],
            dependency_count,
        );
    }

    CutAssessment::new(
        CutStatus::Closed,
        ["CAUSAL_CUT_CLOSED".to_owned()],
        cut.watermarks.len(),
        cut.events.len(),
        dependency_count,
    )
}

/// Compare two successive watermark vectors. Removing a previously observed
/// partition, changing its epoch without an explicit transition contract, or
/// moving either sequence or receiver offset backwards is not silently allowed.
pub fn compare_watermark_progress(
    previous: &[PartitionWatermark],
    current: &[PartitionWatermark],
) -> CutAssessment {
    let mut old = BTreeMap::new();
    let mut new = BTreeMap::new();
    let mut invalid = Vec::new();
    let mut unverified = Vec::new();
    let mut incomplete = Vec::new();

    for watermark in previous {
        let key = partition_key(&watermark.source_id, &watermark.partition_id);
        if old.insert(key, watermark).is_some() {
            invalid.push("PREVIOUS_VECTOR_HAS_DUPLICATE_PARTITION".to_owned());
        }
    }
    for watermark in current {
        let key = partition_key(&watermark.source_id, &watermark.partition_id);
        if new.insert(key, watermark).is_some() {
            invalid.push("CURRENT_VECTOR_HAS_DUPLICATE_PARTITION".to_owned());
        }
    }

    for (key, prior) in &old {
        let Some(now) = new.get(key) else {
            incomplete.push("PREVIOUS_PARTITION_MISSING_FROM_CURRENT_VECTOR".to_owned());
            continue;
        };
        if prior.stream_epoch != now.stream_epoch {
            unverified.push("STREAM_EPOCH_CHANGED_WITHOUT_TRANSITION_PROOF".to_owned());
            continue;
        }
        if now.last_contiguous_sequence < prior.last_contiguous_sequence
            || now.receiver_inclusion_offset < prior.receiver_inclusion_offset
        {
            invalid.push("WATERMARK_REGRESSION".to_owned());
        }
    }

    let status = if !invalid.is_empty() {
        CutStatus::Invalid
    } else if !unverified.is_empty() {
        CutStatus::Unverified
    } else if !incomplete.is_empty() {
        CutStatus::Incomplete
    } else {
        CutStatus::Closed
    };
    let reasons = invalid
        .into_iter()
        .chain(unverified)
        .chain(incomplete)
        .collect::<Vec<_>>();

    CutAssessment::new(status, reasons, current.len(), 0, 0)
}

#[cfg(test)]
mod tests {
    use super::{
        CutStatus, EvidenceCut, PartitionWatermark, compare_watermark_progress,
        validate_causal_cut,
    };
    use serde_json::{from_str, from_value};

    const FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/aero_vector_evidence_cut_v0_1.json"
    );

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn watermark(source: &str, partition: &str, epoch: &str, seq: u64, offset: u64) -> PartitionWatermark {
        PartitionWatermark {
            source_id: source.to_owned(),
            partition_id: partition.to_owned(),
            stream_epoch: epoch.to_owned(),
            last_contiguous_sequence: seq,
            receiver_inclusion_offset: offset,
            checkpoint_digest_sha256: DIGEST.to_owned(),
        }
    }

    fn cut(value: serde_json::Value) -> EvidenceCut {
        from_value(value).expect("evidence cut fixture parses")
    }

    #[test]
    fn independent_concurrent_partitions_form_a_closed_cut() {
        let evidence = cut(serde_json::json!({
            "watermarks": [
                {
                    "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-1",
                    "last_contiguous_sequence": 1, "receiver_inclusion_offset": 10,
                    "checkpoint_digest_sha256": DIGEST
                },
                {
                    "source_id": "sensor-B", "partition_id": "0", "stream_epoch": "epoch-1",
                    "last_contiguous_sequence": 1, "receiver_inclusion_offset": 8,
                    "checkpoint_digest_sha256": DIGEST
                }
            ],
            "events": [
                {
                    "event_ref": "event-A-1", "source_id": "sensor-A", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 1, "receiver_inclusion_offset": 10,
                    "dependencies": []
                },
                {
                    "event_ref": "event-B-1", "source_id": "sensor-B", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 1, "receiver_inclusion_offset": 8,
                    "dependencies": []
                }
            ]
        }));
        let result = validate_causal_cut(&evidence);
        assert_eq!(result.status, CutStatus::Closed);
        assert_eq!(result.event_count, 2);
    }

    #[test]
    fn missing_causal_predecessor_makes_cut_incomplete() {
        let evidence = cut(serde_json::json!({
            "watermarks": [{
                "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-1",
                "last_contiguous_sequence": 2, "receiver_inclusion_offset": 20,
                "checkpoint_digest_sha256": DIGEST
            }],
            "events": [{
                "event_ref": "event-A-2", "source_id": "sensor-A", "partition_id": "0",
                "stream_epoch": "epoch-1", "sequence": 2, "receiver_inclusion_offset": 20,
                "dependencies": [{
                    "event_ref": "event-A-1", "source_id": "sensor-A", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 1
                }]
            }]
        }));
        assert_eq!(
            validate_causal_cut(&evidence).status,
            CutStatus::Incomplete
        );
    }

    #[test]
    fn different_stream_epoch_is_unverified_not_silently_ordered() {
        let evidence = cut(serde_json::json!({
            "watermarks": [{
                "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-2",
                "last_contiguous_sequence": 4, "receiver_inclusion_offset": 40,
                "checkpoint_digest_sha256": DIGEST
            }],
            "events": [{
                "event_ref": "event-A-4", "source_id": "sensor-A", "partition_id": "0",
                "stream_epoch": "epoch-1", "sequence": 4, "receiver_inclusion_offset": 40,
                "dependencies": []
            }]
        }));
        assert_eq!(
            validate_causal_cut(&evidence).status,
            CutStatus::Unverified
        );
    }

    #[test]
    fn event_claimed_included_beyond_watermark_is_invalid() {
        let evidence = cut(serde_json::json!({
            "watermarks": [{
                "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-1",
                "last_contiguous_sequence": 3, "receiver_inclusion_offset": 30,
                "checkpoint_digest_sha256": DIGEST
            }],
            "events": [{
                "event_ref": "event-A-4", "source_id": "sensor-A", "partition_id": "0",
                "stream_epoch": "epoch-1", "sequence": 4, "receiver_inclusion_offset": 40,
                "dependencies": []
            }]
        }));
        assert_eq!(validate_causal_cut(&evidence).status, CutStatus::Invalid);
    }

    #[test]
    fn duplicated_sequence_is_detected_as_a_fork() {
        let evidence = cut(serde_json::json!({
            "watermarks": [{
                "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-1",
                "last_contiguous_sequence": 2, "receiver_inclusion_offset": 20,
                "checkpoint_digest_sha256": DIGEST
            }],
            "events": [
                {
                    "event_ref": "event-A-1a", "source_id": "sensor-A", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 1, "receiver_inclusion_offset": 10,
                    "dependencies": []
                },
                {
                    "event_ref": "event-A-1b", "source_id": "sensor-A", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 1, "receiver_inclusion_offset": 11,
                    "dependencies": []
                }
            ]
        }));
        assert_eq!(validate_causal_cut(&evidence).status, CutStatus::Invalid);
    }

    #[test]
    fn causal_dependency_cycle_is_invalid() {
        let evidence = cut(serde_json::json!({
            "watermarks": [{
                "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-1",
                "last_contiguous_sequence": 2, "receiver_inclusion_offset": 20,
                "checkpoint_digest_sha256": DIGEST
            }],
            "events": [
                {
                    "event_ref": "event-A-1", "source_id": "sensor-A", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 1, "receiver_inclusion_offset": 10,
                    "dependencies": [{
                        "event_ref": "event-A-2", "source_id": "sensor-A", "partition_id": "0",
                        "stream_epoch": "epoch-1", "sequence": 2
                    }]
                },
                {
                    "event_ref": "event-A-2", "source_id": "sensor-A", "partition_id": "0",
                    "stream_epoch": "epoch-1", "sequence": 2, "receiver_inclusion_offset": 20,
                    "dependencies": [{
                        "event_ref": "event-A-1", "source_id": "sensor-A", "partition_id": "0",
                        "stream_epoch": "epoch-1", "sequence": 1
                    }]
                }
            ]
        }));
        assert_eq!(validate_causal_cut(&evidence).status, CutStatus::Invalid);
    }

    #[test]
    fn watermark_regression_is_invalid() {
        let previous = [watermark("sensor-A", "0", "epoch-1", 20, 200)];
        let current = [watermark("sensor-A", "0", "epoch-1", 19, 205)];
        assert_eq!(
            compare_watermark_progress(&previous, &current).status,
            CutStatus::Invalid
        );
    }

    #[test]
    fn stream_epoch_change_requires_explicit_transition_evidence() {
        let previous = [watermark("sensor-A", "0", "epoch-1", 20, 200)];
        let current = [watermark("sensor-A", "0", "epoch-2", 1, 5)];
        assert_eq!(
            compare_watermark_progress(&previous, &current).status,
            CutStatus::Unverified
        );
    }

    #[test]
    fn checked_in_vector_cut_fixture_is_causally_closed() {
        let evidence = from_str::<EvidenceCut>(FIXTURE).expect("vector cut fixture parses");
        let report = validate_causal_cut(&evidence);
        assert_eq!(report.status, CutStatus::Closed);
        assert_eq!(report.watermark_count, 2);
        assert_eq!(report.event_count, 3);
        assert_eq!(report.dependency_count, 2);
    }

    #[test]
    fn empty_watermark_vector_is_unverified() {
        let evidence = EvidenceCut {
            watermarks: Vec::new(),
            events: Vec::new(),
        };
        assert_eq!(
            validate_causal_cut(&evidence).status,
            CutStatus::Unverified
        );
    }

    #[test]
    fn malformed_digest_rejects_the_cut() {
        let evidence = cut(serde_json::json!({
            "watermarks": [{
                "source_id": "sensor-A", "partition_id": "0", "stream_epoch": "epoch-1",
                "last_contiguous_sequence": 1, "receiver_inclusion_offset": 10,
                "checkpoint_digest_sha256": "not-a-digest"
            }],
            "events": []
        }));
        assert_eq!(validate_causal_cut(&evidence).status, CutStatus::Invalid);
    }
}
