// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-018: metamorphic verification of bounded memory semantics.
//!
//! Transforms the same event stream to expose duplicate-delivery, event-order,
//! stale-revival, unrelated-history, chunking, same-revision-conflict, and
//! capacity behavior. Query keys are introduced only after writes complete.
//! This is a deterministic synthetic diagnostic, not native robot performance.
//!
//! rustc --edition=2024 --test tools/vision-memory-metamorphic.rs -o /tmp/vis-mem-meta-tests
//! /tmp/vis-mem-meta-tests
//! rustc --edition=2024 -O tools/vision-memory-metamorphic.rs -o /tmp/vis-mem-meta
//! /tmp/vis-mem-meta > vis-mem-metamorphic.tsv

use std::collections::BTreeMap;

const BUDGETS: [usize; 3] = [2, 4, 8];
const MAX_DISTRACTORS: u16 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvidenceKind {
    Outcome,
    Observation,
    Inference,
}

impl EvidenceKind {
    fn label(self) -> &'static str {
        match self {
            Self::Outcome => "outcome",
            Self::Observation => "observation",
            Self::Inference => "inference",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    entity: u16,
    source_revision: u16,
    value: u32,
    kind: EvidenceKind,
}

fn event(entity: u16, source_revision: u16, value: u32, kind: EvidenceKind) -> Event {
    Event { entity, source_revision, value, kind }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    LatestArrival,
    SourceRevisionUpsert,
    RevisionWatermark,
    ConflictAwareWatermark,
}

impl Policy {
    const ALL: [Self; 4] = [
        Self::LatestArrival,
        Self::SourceRevisionUpsert,
        Self::RevisionWatermark,
        Self::ConflictAwareWatermark,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::LatestArrival => "latest_arrival",
            Self::SourceRevisionUpsert => "source_revision_upsert",
            Self::RevisionWatermark => "revision_watermark",
            Self::ConflictAwareWatermark => "conflict_aware_watermark",
        }
    }

    fn has_watermark(self) -> bool {
        matches!(self, Self::RevisionWatermark | Self::ConflictAwareWatermark)
    }

    fn detects_equal_revision_conflict(self) -> bool {
        self == Self::ConflictAwareWatermark
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredEvent {
    event: Event,
    arrival: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Watermark {
    source_revision: u16,
    value_fingerprint: u32,
    conflicted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lookup {
    Value { source_revision: u16, value: u32 },
    Conflict { source_revision: u16 },
    Missing,
}

impl Lookup {
    fn label(self) -> String {
        match self {
            Self::Value { source_revision, value } => format!("value:{source_revision}:{value}"),
            Self::Conflict { source_revision } => format!("conflict:{source_revision}"),
            Self::Missing => "missing".to_owned(),
        }
    }

    fn revision(self) -> Option<u16> {
        match self {
            Self::Value { source_revision, .. } | Self::Conflict { source_revision } => {
                Some(source_revision)
            }
            Self::Missing => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WriteOutcome {
    result: &'static str,
    evicted: Option<StoredEvent>,
}

#[derive(Clone, Debug)]
struct Store {
    policy: Policy,
    budget: usize,
    records: Vec<StoredEvent>,
    watermarks: BTreeMap<u16, Watermark>,
    next_arrival: u64,
    writes: usize,
    stale_rejections: usize,
    equal_revision_conflicts: usize,
}

impl Store {
    fn new(policy: Policy, budget: usize) -> Self {
        Self {
            policy,
            budget,
            records: Vec::new(),
            watermarks: BTreeMap::new(),
            next_arrival: 0,
            writes: 0,
            stale_rejections: 0,
            equal_revision_conflicts: 0,
        }
    }

    // The writer is structurally unable to inspect a future query/target key.
    fn insert(&mut self, input: Event) -> WriteOutcome {
        let arrival = self.next_arrival;
        self.next_arrival = self.next_arrival.saturating_add(1);
        self.writes = self.writes.saturating_add(1);

        if self.policy.has_watermark() {
            match self.watermarks.get_mut(&input.entity) {
                Some(watermark) if input.source_revision < watermark.source_revision => {
                    self.stale_rejections = self.stale_rejections.saturating_add(1);
                    return WriteOutcome { result: "reject_below_watermark", evicted: None };
                }
                Some(watermark) if input.source_revision > watermark.source_revision => {
                    *watermark = Watermark {
                        source_revision: input.source_revision,
                        value_fingerprint: fingerprint(input.value),
                        conflicted: false,
                    };
                }
                Some(watermark) => {
                    if watermark.value_fingerprint != fingerprint(input.value) {
                        self.equal_revision_conflicts = self.equal_revision_conflicts.saturating_add(1);
                        if self.policy.detects_equal_revision_conflict() {
                            watermark.conflicted = true;
                            return WriteOutcome { result: "flag_equal_revision_conflict", evicted: None };
                        } else {
                            // Deliberately weak ablation: same-revision values are
                            // resolved by arrival order rather than evidence.
                            watermark.value_fingerprint = fingerprint(input.value);
                        }
                    }
                }
                None => {
                    self.watermarks.insert(input.entity, Watermark {
                        source_revision: input.source_revision,
                        value_fingerprint: fingerprint(input.value),
                        conflicted: false,
                    });
                }
            }
        }

        let existing = self.records.iter().position(|record| record.event.entity == input.entity);
        if let Some(index) = existing {
            let previous = self.records[index];
            if self.policy == Policy::SourceRevisionUpsert && input.source_revision < previous.event.source_revision {
                self.stale_rejections = self.stale_rejections.saturating_add(1);
                return WriteOutcome { result: "reject_older_visible_revision", evicted: None };
            }
            if self.policy.has_watermark() && input.source_revision < previous.event.source_revision {
                self.stale_rejections = self.stale_rejections.saturating_add(1);
                return WriteOutcome { result: "reject_older_visible_revision", evicted: None };
            }
            self.records.remove(index);
        }

        self.records.push(StoredEvent { event: input, arrival });
        let evicted = if self.records.len() > self.budget {
            Some(self.records.remove(0))
        } else {
            None
        };
        WriteOutcome {
            result: if existing.is_some() { "replace_entity" } else { "insert_entity" },
            evicted,
        }
    }

    fn insert_all(&mut self, events: &[Event]) {
        for input in events.iter().copied() {
            self.insert(input);
        }
    }

    fn insert_chunked(&mut self, events: &[Event], chunk_sizes: &[usize]) {
        let mut offset = 0;
        for size in chunk_sizes.iter().copied() {
            if offset >= events.len() {
                break;
            }
            let end = offset.saturating_add(size).min(events.len());
            self.insert_all(&events[offset..end]);
            offset = end;
        }
        if offset < events.len() {
            self.insert_all(&events[offset..]);
        }
    }

    fn lookup(&self, query_entity: u16) -> Lookup {
        if let Some(watermark) = self.watermarks.get(&query_entity) {
            if watermark.conflicted {
                return Lookup::Conflict { source_revision: watermark.source_revision };
            }
        }
        self.records.iter()
            .filter(|record| record.event.entity == query_entity)
            .max_by_key(|record| record.arrival)
            .map(|record| Lookup::Value {
                source_revision: record.event.source_revision,
                value: record.event.value,
            })
            .unwrap_or(Lookup::Missing)
    }

    fn watermark_logical_bytes(&self) -> usize {
        // Conceptual packed encoding: entity key (2), source revision (2),
        // fingerprint (4), conflict flag (1). Excludes map/allocator overhead.
        self.watermarks.len() * 9
    }
}

fn fingerprint(value: u32) -> u32 {
    let mut x = value.wrapping_add(0x9E37_79B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^ (x >> 16)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Transform {
    DuplicateDelivery,
    SourceRevisionPermutation,
    StaleAfterPayloadEviction,
    UnrelatedEntityInsertion,
    ChunkPartition,
    EqualRevisionConflict,
    DistractorPressure,
}

impl Transform {
    const ALL: [Self; 7] = [
        Self::DuplicateDelivery,
        Self::SourceRevisionPermutation,
        Self::StaleAfterPayloadEviction,
        Self::UnrelatedEntityInsertion,
        Self::ChunkPartition,
        Self::EqualRevisionConflict,
        Self::DistractorPressure,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::DuplicateDelivery => "duplicate_delivery",
            Self::SourceRevisionPermutation => "source_revision_permutation",
            Self::StaleAfterPayloadEviction => "stale_after_payload_eviction",
            Self::UnrelatedEntityInsertion => "unrelated_entity_insertion",
            Self::ChunkPartition => "chunk_partition",
            Self::EqualRevisionConflict => "equal_revision_conflict",
            Self::DistractorPressure => "distractor_pressure",
        }
    }
}

#[derive(Clone, Debug)]
struct ResultRow {
    transform: Transform,
    policy: Policy,
    budget: usize,
    query_entity: u16,
    expectation: &'static str,
    required: bool,
    passed: bool,
    before: String,
    after: String,
    capacity_used: usize,
    watermark_entries: usize,
    watermark_bytes: usize,
    stale_rejections: usize,
    conflicts: usize,
}

fn base_events() -> Vec<Event> {
    vec![
        event(1, 1, 101, EvidenceKind::Observation),
        event(2, 1, 201, EvidenceKind::Outcome),
        event(1, 2, 102, EvidenceKind::Observation),
    ]
}

fn transform_result(transform: Transform, policy: Policy, budget: usize) -> ResultRow {
    let mut expectation = "invariant";
    let mut required = true;
    let mut query_entity = 1;
    let mut before = Lookup::Missing;
    let mut after = Lookup::Missing;
    let mut final_store = Store::new(policy, budget);

    match transform {
        Transform::DuplicateDelivery => {
            let events = base_events();
            final_store.insert_all(&events);
            before = final_store.lookup(query_entity);
            let duplicate = *events.last().expect("base events nonempty");
            final_store.insert(duplicate);
            after = final_store.lookup(query_entity);
            if budget >= 2 {
                expectation = "duplicate_idempotence";
            } else {
                required = false;
                expectation = "budget_exempt";
            }
        }
        Transform::SourceRevisionPermutation => {
            let forward = vec![
                event(1, 1, 101, EvidenceKind::Observation),
                event(2, 1, 201, EvidenceKind::Outcome),
                event(1, 2, 102, EvidenceKind::Observation),
            ];
            let reverse = vec![
                event(1, 2, 102, EvidenceKind::Observation),
                event(2, 1, 201, EvidenceKind::Outcome),
                event(1, 1, 101, EvidenceKind::Observation),
            ];
            let mut left = Store::new(policy, budget);
            let mut right = Store::new(policy, budget);
            left.insert_all(&forward);
            right.insert_all(&reverse);
            before = left.lookup(query_entity);
            after = right.lookup(query_entity);
            final_store = right;
            expectation = if policy == Policy::LatestArrival {
                "arrival_order_counterexample_expected"
            } else {
                "source_revision_order_invariant"
            };
        }
        Transform::StaleAfterPayloadEviction => {
            query_entity = 1;
            let mut events = vec![event(1, 2, 102, EvidenceKind::Observation)];
            for distractor in 0..budget {
                events.push(event(2 + distractor as u16, 1, 200 + distractor as u32, EvidenceKind::Inference));
            }
            final_store = Store::new(policy, budget);
            final_store.insert_all(&events);
            before = final_store.lookup(query_entity);
            final_store.insert(event(1, 1, 101, EvidenceKind::Observation));
            after = final_store.lookup(query_entity);
            expectation = if policy.has_watermark() {
                "stale_revival_rejected"
            } else {
                "stale_revival_counterexample_expected"
            };
        }
        Transform::UnrelatedEntityInsertion => {
            query_entity = 2;
            let events = base_events();
            let mut baseline = Store::new(policy, budget);
            final_store = Store::new(policy, budget);
            baseline.insert_all(&events);
            final_store.insert_all(&events);
            before = baseline.lookup(query_entity);
            final_store.insert(event(99, 1, 9901, EvidenceKind::Inference));
            after = final_store.lookup(query_entity);
            if budget >= 4 {
                expectation = "unrelated_history_noninterference";
            } else {
                expectation = "capacity_pressure_exemption";
                required = false;
            }
        }
        Transform::ChunkPartition => {
            let events = vec![
                event(1, 1, 101, EvidenceKind::Observation),
                event(2, 1, 201, EvidenceKind::Outcome),
                event(1, 2, 102, EvidenceKind::Observation),
                event(3, 1, 301, EvidenceKind::Inference),
            ];
            let mut ordinary = Store::new(policy, budget);
            let mut chunked = Store::new(policy, budget);
            ordinary.insert_all(&events);
            chunked.insert_chunked(&events, &[1, 2]);
            before = ordinary.lookup(1);
            after = chunked.lookup(1);
            final_store = chunked;
            expectation = "stream_chunking_invariant";
        }
        Transform::EqualRevisionConflict => {
            query_entity = 1;
            let events = [
                event(1, 2, 102, EvidenceKind::Observation),
                event(2, 1, 201, EvidenceKind::Outcome),
            ];
            final_store = Store::new(policy, budget);
            final_store.insert_all(&events);
            before = final_store.lookup(query_entity);
            final_store.insert(event(1, 2, 777, EvidenceKind::Observation));
            after = final_store.lookup(query_entity);
            expectation = if policy.detects_equal_revision_conflict() {
                "conflict_must_be_visible"
            } else {
                "silent_resolution_control"
            };
        }
        Transform::DistractorPressure => {
            let stream: Vec<Event> = (0..MAX_DISTRACTORS)
                .map(|entity| event(100 + entity, 1, u32::from(entity) * 17, EvidenceKind::Inference))
                .collect();
            final_store = Store::new(policy, budget);
            final_store.insert_all(&stream);
            query_entity = 100;
            after = final_store.lookup(query_entity);
            before = Lookup::Missing;
            expectation = "capacity_bound_under_distractors";
        }
    }

    let observed = match expectation {
        "arrival_order_counterexample_expected" => before != after,
        "stale_revival_rejected" => !matches!(after, Lookup::Value { source_revision: 1, .. }),
        "stale_revival_counterexample_expected" => matches!(after, Lookup::Value { source_revision: 1, .. }),
        "conflict_must_be_visible" => matches!(after, Lookup::Conflict { .. }),
        "silent_resolution_control" => matches!(after, Lookup::Value { .. }),
        "capacity_bound_under_distractors" => final_store.records.len() <= budget,
        _ if !required => true,
        _ => before == after,
    };

    ResultRow {
        transform,
        policy,
        budget,
        query_entity,
        expectation,
        required,
        passed: observed,
        before: before.label(),
        after: after.label(),
        capacity_used: final_store.records.len(),
        watermark_entries: final_store.watermarks.len(),
        watermark_bytes: final_store.watermark_logical_bytes(),
        stale_rejections: final_store.stale_rejections,
        conflicts: final_store.equal_revision_conflicts,
    }
}

fn flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn line(out: &mut String, fields: &[String]) {
    out.push_str(&fields.join("\t"));
    out.push('\n');
}

fn make_report() -> String {
    // Build rows with owned array iteration; no references into temporary
    // associated-const arrays escape nested iterator closures.
    let mut rows: Vec<ResultRow> = Vec::new();
    for transform in Transform::ALL {
        for policy in Policy::ALL {
            for budget in BUDGETS {
                rows.push(transform_result(transform, policy, budget));
            }
        }
    }

    let mut out = String::new();
    line(&mut out, &["META".into(), "format".into(), "VIS-MEM-018-v0.1".into()]);
    line(&mut out, &["META".into(), "scope".into(), "synthetic metamorphic semantics; not native task performance".into()]);
    line(&mut out, &["META".into(), "query_boundary".into(), "query key enters only after stream insertion for each case".into()]);
    line(&mut out, &["META".into(), "watermark_cost".into(), "9 conceptual packed payload bytes per entity watermark; map/allocator overhead excluded".into()]);
    line(&mut out, &[
        "ROW_HEADER".into(), "transformation".into(), "policy".into(), "budget".into(),
        "query_entity".into(), "expectation".into(), "required".into(), "contract_check_passed".into(),
        "before_lookup".into(), "after_lookup".into(), "capacity_used".into(),
        "watermark_entries".into(), "watermark_logical_bytes".into(), "stale_rejections".into(),
        "equal_revision_conflicts".into(),
    ]);
    for r in &rows {
        line(&mut out, &[
            "ROW".into(), r.transform.label().into(), r.policy.label().into(),
            r.budget.to_string(), r.query_entity.to_string(), r.expectation.into(),
            flag(r.required).into(), flag(r.passed).into(), r.before.clone(), r.after.clone(),
            r.capacity_used.to_string(), r.watermark_entries.to_string(),
            r.watermark_bytes.to_string(), r.stale_rejections.to_string(), r.conflicts.to_string(),
        ]);
    }

    line(&mut out, &[
        "SUMMARY_HEADER".into(), "transformation".into(), "policy".into(), "cases".into(),
        "required_cases".into(), "passed_required".into(), "pass_rate_required".into(),
        "counterexamples".into(), "max_capacity_overrun".into(), "max_watermark_bytes".into(),
    ]);
    for transform in Transform::ALL {
        for policy in Policy::ALL {
            let group: Vec<&ResultRow> = rows.iter()
                .filter(|r| r.transform == transform && r.policy == policy)
                .collect();
            let required: Vec<&ResultRow> = group.iter().copied().filter(|r| r.required).collect();
            let passed = required.iter().filter(|r| r.passed).count();
            let counterexamples = required.iter().filter(|r| !r.passed).count();
            let max_capacity_overrun = group.iter().any(|r| r.capacity_used > r.budget);
            let max_watermark_bytes = group.iter().map(|r| r.watermark_bytes).max().unwrap_or(0);
            line(&mut out, &[
                "SUMMARY".into(), transform.label().into(), policy.label().into(),
                group.len().to_string(), required.len().to_string(), passed.to_string(),
                format!("{:.3}", if required.is_empty() { 1.0 } else { passed as f64 / required.len() as f64 }),
                counterexamples.to_string(), flag(max_capacity_overrun).into(),
                max_watermark_bytes.to_string(),
            ]);
        }
    }
    out
}

fn main() {
    print!("{}", make_report());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_duplicate_delivery_is_idempotent_for_keyed_memory() {
        for policy in Policy::ALL {
            let row = transform_result(Transform::DuplicateDelivery, policy, 4);
            assert!(row.passed, "{}: {} != {}", policy.label(), row.before, row.after);
        }
    }

    #[test]
    fn source_revision_order_reversal_exposes_last_arrival_baseline() {
        let arrival = transform_result(Transform::SourceRevisionPermutation, Policy::LatestArrival, 8);
        assert!(arrival.passed);
        let maxrev = transform_result(Transform::SourceRevisionPermutation, Policy::SourceRevisionUpsert, 8);
        assert!(maxrev.passed);
        let watermark = transform_result(Transform::SourceRevisionPermutation, Policy::RevisionWatermark, 8);
        assert!(watermark.passed);
    }

    #[test]
    fn source_revision_upsert_can_revive_stale_state_after_payload_eviction() {
        let r = transform_result(Transform::StaleAfterPayloadEviction, Policy::SourceRevisionUpsert, 2);
        assert!(r.passed);
        assert!(r.after.contains(":101"));
        let watermark = transform_result(Transform::StaleAfterPayloadEviction, Policy::RevisionWatermark, 2);
        assert!(watermark.passed);
        assert!(watermark.after == "missing");
    }

    #[test]
    fn persistent_watermark_rejects_stale_arrival_after_payload_eviction() {
        for policy in [Policy::RevisionWatermark, Policy::ConflictAwareWatermark] {
            let r = transform_result(Transform::StaleAfterPayloadEviction, policy, 2);
            assert!(r.passed, "{} failed stale-revival check", policy.label());
            assert_ne!(r.after, "value:1:101");
        }
    }

    #[test]
    fn same_revision_conflict_is_visible_only_with_conflict_aware_watermark() {
        let robust = transform_result(Transform::EqualRevisionConflict, Policy::ConflictAwareWatermark, 4);
        assert!(robust.passed);
        assert!(robust.after.starts_with("conflict:"));
        for policy in [Policy::LatestArrival, Policy::SourceRevisionUpsert, Policy::RevisionWatermark] {
            let control = transform_result(Transform::EqualRevisionConflict, policy, 4);
            assert!(control.passed);
            assert!(control.after.starts_with("value:"));
        }
    }

    #[test]
    fn unrelated_history_does_not_change_queried_fact_when_no_eviction_is_needed() {
        for policy in Policy::ALL {
            let r = transform_result(Transform::UnrelatedEntityInsertion, policy, 8);
            assert!(r.passed, "{}: {} != {}", policy.label(), r.before, r.after);
        }
    }

    #[test]
    fn chunk_partition_and_capacity_are_invariant() {
        for policy in Policy::ALL {
            assert!(transform_result(Transform::ChunkPartition, policy, 4).passed);
            for budget in BUDGETS {
                let r = transform_result(Transform::DistractorPressure, policy, budget);
                assert!(r.passed);
                assert!(r.capacity_used <= budget);
            }
        }
    }

    #[test]
    fn insertion_never_receives_a_future_query_key() {
        let source = include_str!("vision-memory-metamorphic.rs");
        let start = source.find("fn insert(&mut self, input: Event)").unwrap();
        let end = source[start..].find("\n    fn insert_all(").unwrap() + start;
        let writer = &source[start..end];
        assert!(!writer.contains("query_entity"));
        assert!(!writer.contains("query_key"));
        assert!(!writer.contains("expected"));
    }

    #[test]
    fn report_replays_and_has_exact_row_and_summary_schemas() {
        let a = make_report();
        assert_eq!(a, make_report());
        assert_eq!(a.lines().filter(|l| l.starts_with("ROW_HEADER\t")).count(), 1);
        assert_eq!(a.lines().filter(|l| l.starts_with("ROW\t")).count(), 7 * 4 * 3);
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY_HEADER\t")).count(), 1);
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY\t")).count(), 7 * 4);
        for record in a.lines() {
            if record.starts_with("ROW_HEADER\t") || record.starts_with("ROW\t") {
                assert_eq!(record.split('\t').count(), 15, "row schema: {record}");
            }
            if record.starts_with("SUMMARY_HEADER\t") || record.starts_with("SUMMARY\t") {
                assert_eq!(record.split('\t').count(), 10, "summary schema: {record}");
            }
        }
    }
}
