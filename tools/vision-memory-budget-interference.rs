// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-016: fixed-budget memory under distractors, updates, and stale arrivals.
//!
//! Dependency-free synthetic diagnostic. Storage runs without access to the
//! future query entity. Queries are issued only after the stream is complete;
//! ground-truth current values remain in the evaluator. This is not a native
//! robotics benchmark and makes no SOTA claim.
//!
//! rustc --edition=2024 --test tools/vision-memory-budget-interference.rs -o /tmp/vis-mem-budget-tests
//! /tmp/vis-mem-budget-tests
//! rustc --edition=2024 -O tools/vision-memory-budget-interference.rs -o /tmp/vis-mem-budget
//! /tmp/vis-mem-budget > vis-mem-budget.tsv

const CORE_ENTITIES: u16 = 12;
const DISTRACTOR_LEVELS: [usize; 4] = [0, 12, 36, 72];
const BUDGETS: [usize; 3] = [4, 8, 12];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvidenceKind {
    Outcome,
    Observation,
    Inference,
}

impl EvidenceKind {
    fn label(self) -> &'static str {
        match self {
            Self::Outcome => "action_outcome",
            Self::Observation => "grounded_observation",
            Self::Inference => "inference",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Outcome => 3,
            Self::Observation => 2,
            Self::Inference => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Record {
    entity: u16,
    source_revision: u16,
    value: u32,
    kind: EvidenceKind,
    arrival: u32,
}

fn entity_kind(entity: u16) -> EvidenceKind {
    if entity >= 100 {
        EvidenceKind::Inference
    } else if entity % 2 == 0 {
        EvidenceKind::Outcome
    } else {
        EvidenceKind::Observation
    }
}

fn append_record(stream: &mut Vec<Record>, entity: u16, source_revision: u16) {
    let arrival = stream.len() as u32;
    stream.push(Record {
        entity,
        source_revision,
        value: u32::from(entity) * 100 + u32::from(source_revision),
        kind: entity_kind(entity),
        arrival,
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Regime {
    Stable,
    MonotonicUpdates,
    UpdateChurn,
    ArrivalReversal,
}

impl Regime {
    const ALL: [Self; 4] = [
        Self::Stable,
        Self::MonotonicUpdates,
        Self::UpdateChurn,
        Self::ArrivalReversal,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::MonotonicUpdates => "monotonic_updates",
            Self::UpdateChurn => "update_churn",
            Self::ArrivalReversal => "arrival_reversal",
        }
    }
}

fn build_stream(regime: Regime, distractors: usize) -> (Vec<Record>, [Record; CORE_ENTITIES as usize]) {
    let mut stream = Vec::new();

    for entity in 0..CORE_ENTITIES {
        append_record(&mut stream, entity, 1);
    }

    match regime {
        Regime::Stable => {}
        Regime::MonotonicUpdates => {
            for entity in 0..CORE_ENTITIES {
                append_record(&mut stream, entity, 2);
            }
        }
        Regime::UpdateChurn => {
            for revision in [2, 3] {
                for entity in 0..CORE_ENTITIES {
                    append_record(&mut stream, entity, revision);
                }
            }
        }
        Regime::ArrivalReversal => {
            for entity in 0..CORE_ENTITIES {
                append_record(&mut stream, entity, 2);
            }
            append_distractors(&mut stream, distractors);
            // Older source revisions arrive after newer revisions and the
            // distractor burst. They must not overwrite newer world state.
            for entity in 0..CORE_ENTITIES {
                append_record(&mut stream, entity, 1);
            }
        }
    }

    if regime != Regime::ArrivalReversal {
        append_distractors(&mut stream, distractors);
    }

    let truth = std::array::from_fn(|idx| {
        let entity = idx as u16;
        stream.iter().copied()
            .filter(|record| record.entity == entity)
            .max_by_key(|record| record.source_revision)
            .expect("every core entity has an initial fact")
    });

    (stream, truth)
}

fn append_distractors(stream: &mut Vec<Record>, count: usize) {
    for index in 0..count {
        append_record(stream, 100 + index as u16, 1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    FifoRecords,
    ArrivalUpsertFifo,
    SourceRevisionUpsertFifo,
    EvidenceTierBounded,
    RevisionWatermarkEvidenceTier,
}

impl Policy {
    const ALL: [Self; 4] = [
        Self::FifoRecords,
        Self::ArrivalUpsertFifo,
        Self::SourceRevisionUpsertFifo,
        Self::EvidenceTierBounded,
        Self::RevisionWatermarkEvidenceTier,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::FifoRecords => "record_fifo",
            Self::ArrivalUpsertFifo => "arrival_upsert_fifo",
            Self::SourceRevisionUpsertFifo => "source_revision_upsert_fifo",
            Self::EvidenceTierBounded => "evidence_tier_bounded",
            Self::RevisionWatermarkEvidenceTier => "revision_watermark_evidence_tier",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct WriteEvent {
    result: &'static str,
    evicted: Option<Record>,
    store_size: usize,
    update_ops: usize,
    rejected_stale_writes: usize,
    watermark_entries: usize,
}

#[derive(Clone, Debug)]
struct Store {
    policy: Policy,
    budget: usize,
    records: Vec<Record>,
    write_ops: usize,
    update_ops: usize,
    rejected_stale_writes: usize,
    // Source-revision high-water marks survive payload eviction. Logical
    // payload cost is reported separately from fixed record slots.
    watermarks: Vec<(u16, u16)>,
}

impl Store {
    fn new(policy: Policy, budget: usize) -> Self {
        Self {
            policy,
            budget,
            records: Vec::new(),
            write_ops: 0,
            update_ops: 0,
            rejected_stale_writes: 0,
            watermarks: Vec::new(),
        }
    }

    // Intentionally has no future query/target argument.
    fn insert(&mut self, record: Record) -> WriteEvent {
        self.write_ops += 1;
        let existing = self.records.iter().position(|stored| stored.entity == record.entity);
        if existing.is_some() {
            self.update_ops += 1;
        }

        if self.policy == Policy::RevisionWatermarkEvidenceTier {
            match self.watermarks.iter().position(|(entity, _)| *entity == record.entity) {
                Some(index) if record.source_revision < self.watermarks[index].1 => {
                    self.rejected_stale_writes += 1;
                    return WriteEvent {
                        result: "reject_stale_against_watermark",
                        evicted: None,
                        store_size: self.records.len(),
                        update_ops: self.update_ops,
                        rejected_stale_writes: self.rejected_stale_writes,
                        watermark_entries: self.watermarks.len(),
                    };
                }
                Some(index) => {
                    self.watermarks[index].1 = self.watermarks[index].1.max(record.source_revision);
                }
                None => self.watermarks.push((record.entity, record.source_revision)),
            }
        }

        if self.policy == Policy::FifoRecords {
            self.records.push(record);
        } else if let Some(index) = existing {
            let previous = self.records[index];
            if matches!(
                self.policy,
                Policy::SourceRevisionUpsertFifo
                    | Policy::EvidenceTierBounded
                    | Policy::RevisionWatermarkEvidenceTier
            ) && record.source_revision < previous.source_revision
            {
                self.rejected_stale_writes += 1;
                return WriteEvent {
                    result: "reject_older_source_revision",
                    evicted: None,
                    store_size: self.records.len(),
                    update_ops: self.update_ops,
                    rejected_stale_writes: self.rejected_stale_writes,
                    watermark_entries: self.watermarks.len(),
                };
            }
            self.records.remove(index);
            self.records.push(record);
        } else {
            self.records.push(record);
        }

        let mut evicted = None;
        if self.records.len() > self.budget {
            let index = if matches!(self.policy, Policy::EvidenceTierBounded | Policy::RevisionWatermarkEvidenceTier) {
                self.evidence_tier_eviction_index()
            } else {
                0
            };
            evicted = Some(self.records.remove(index));
        }

        let result = match (existing, self.policy) {
            (Some(_), Policy::ArrivalUpsertFifo) => "replace_by_arrival",
            (Some(_), Policy::SourceRevisionUpsertFifo | Policy::EvidenceTierBounded | Policy::RevisionWatermarkEvidenceTier) => {
                "replace_by_source_revision"
            }
            (Some(_), Policy::FifoRecords) => "append_duplicate_version",
            (None, _) => "insert_new_entity",
        };

        WriteEvent {
            result,
            evicted,
            store_size: self.records.len(),
            update_ops: self.update_ops,
            rejected_stale_writes: self.rejected_stale_writes,
            watermark_entries: self.watermarks.len(),
        }
    }

    fn evidence_tier_eviction_index(&self) -> usize {
        let outcome_count = self.records.iter()
            .filter(|record| record.kind == EvidenceKind::Outcome)
            .count();
        let other_count = self.records.len() - outcome_count;
        let outcome_quota = (self.budget + 1) / 2;
        let other_quota = self.budget / 2;

        // Reserve a balanced two-tier budget for interaction-confirmed
        // outcomes and everything else. Within the second tier, evict weak
        // inferences before grounded observations. Quotas are soft when one
        // tier is under-filled, so available capacity is not left unused.
        if outcome_count > outcome_quota && other_count > other_quota {
            return self.lowest_ranked_other_index()
                .expect("non-outcome tier is nonempty");
        }
        if outcome_count > self.budget.saturating_sub(other_count) {
            return self.oldest_outcome_index()
                .unwrap_or_else(|| self.lowest_ranked_record_index());
        }
        if other_count > self.budget.saturating_sub(outcome_count) {
            return self.lowest_ranked_other_index()
                .unwrap_or_else(|| self.oldest_outcome_index()
                    .unwrap_or_else(|| self.lowest_ranked_record_index()));
        }

        self.lowest_ranked_record_index()
    }

    fn oldest_outcome_index(&self) -> Option<usize> {
        self.records.iter().enumerate()
            .filter(|(_, record)| record.kind == EvidenceKind::Outcome)
            .min_by_key(|(_, record)| record.arrival)
            .map(|(index, _)| index)
    }

    fn lowest_ranked_other_index(&self) -> Option<usize> {
        self.records.iter().enumerate()
            .filter(|(_, record)| record.kind != EvidenceKind::Outcome)
            .min_by_key(|(_, record)| (record.kind.rank(), record.arrival))
            .map(|(index, _)| index)
    }

    fn lowest_ranked_record_index(&self) -> usize {
        self.records.iter().enumerate()
            .min_by_key(|(_, record)| (record.kind.rank(), record.arrival))
            .map(|(index, _)| index)
            .expect("eviction only occurs when store is over budget")
    }

    fn retrieve(&self, entity: u16) -> (Option<Record>, usize) {
        let matches: Vec<Record> = self.records.iter().copied()
            .filter(|record| record.entity == entity)
            .collect();
        let duplicate_versions = matches.len().saturating_sub(1);
        let found = match self.policy {
            Policy::FifoRecords | Policy::ArrivalUpsertFifo => {
                matches.into_iter().max_by_key(|record| record.arrival)
            }
            Policy::SourceRevisionUpsertFifo | Policy::EvidenceTierBounded | Policy::RevisionWatermarkEvidenceTier => {
                matches.into_iter().max_by_key(|record| record.source_revision)
            }
        };
        (found, duplicate_versions)
    }
}

#[derive(Clone, Debug)]
struct QueryResult {
    regime: Regime,
    distractors: usize,
    budget: usize,
    policy: Policy,
    query_entity: u16,
    query_kind: EvidenceKind,
    expected_revision: u16,
    found_revision: Option<u16>,
    expected_value: u32,
    returned_value: Option<u32>,
    success: bool,
    stale_answer: bool,
    missing_answer: bool,
    duplicate_versions: usize,
    store_size: usize,
    write_ops: usize,
    update_ops: usize,
    rejected_stale_writes: usize,
    watermark_entries: usize,
    watermark_payload_bytes: usize,
    capacity_overrun: bool,
}

#[derive(Clone, Debug)]
struct LedgerRow {
    regime: Regime,
    distractors: usize,
    budget: usize,
    policy: Policy,
    step: usize,
    input: Record,
    event: WriteEvent,
}

fn flag(value: bool) -> String {
    if value { "1" } else { "0" }.to_owned()
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 { 0.0 } else { numerator as f64 / denominator as f64 }
}

fn line(out: &mut String, fields: &[String]) {
    out.push_str(&fields.join("\t"));
    out.push('\n');
}

fn opt_u16(value: Option<u16>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_owned())
}

fn opt_u32(value: Option<u32>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_owned())
}

fn make_report() -> String {
    let mut ledger = Vec::<LedgerRow>::new();
    let mut query_rows = Vec::<QueryResult>::new();

    for regime in Regime::ALL {
        for distractors in DISTRACTOR_LEVELS {
            let (stream, truth) = build_stream(regime, distractors);
            for budget in BUDGETS {
                for policy in Policy::ALL {
                    let mut store = Store::new(policy, budget);

                    for (step, input) in stream.iter().copied().enumerate() {
                        let event = store.insert(input);
                        ledger.push(LedgerRow {
                            regime,
                            distractors,
                            budget,
                            policy,
                            step: step + 1,
                            input,
                            event,
                        });
                    }

                    assert!(store.records.len() <= budget);
                    for query_entity in 0..CORE_ENTITIES {
                        let expected = truth[usize::from(query_entity)];
                        let (found, duplicate_versions) = store.retrieve(query_entity);
                        let found_revision = found.map(|record| record.source_revision);
                        let returned_value = found.map(|record| record.value);
                        let stale_answer = found.is_some_and(|record| {
                            record.source_revision < expected.source_revision
                        });
                        let success = found.is_some_and(|record| {
                            record.source_revision == expected.source_revision
                                && record.value == expected.value
                        });

                        query_rows.push(QueryResult {
                            regime,
                            distractors,
                            budget,
                            policy,
                            query_entity,
                            query_kind: expected.kind,
                            expected_revision: expected.source_revision,
                            found_revision,
                            expected_value: expected.value,
                            returned_value,
                            success,
                            stale_answer,
                            missing_answer: found.is_none(),
                            duplicate_versions,
                            store_size: store.records.len(),
                            write_ops: store.write_ops,
                            update_ops: store.update_ops,
                            rejected_stale_writes: store.rejected_stale_writes,
                            watermark_entries: store.watermarks.len(),
                            watermark_payload_bytes: store.watermarks.len() * 4,
                            capacity_overrun: store.records.len() > budget,
                        });
                    }
                }
            }
        }
    }

    let mut out = String::new();
    line(&mut out, &["META".into(), "format".into(), "VIS-MEM-016-v0.1".into()]);
    line(&mut out, &["META".into(), "scope".into(), "synthetic fixed-budget storage/update/interference diagnostic; not native task success".into()]);
    line(&mut out, &["META".into(), "query_boundary".into(), "future query is issued only after the storage stream completes".into()]);
    line(&mut out, &["META".into(), "watermark_cost".into(), "logical payload bytes count 4 bytes per entity/revision pair; allocator overhead is not included".into()]);
    line(&mut out, &[
        "LEDGER_HEADER".into(), "regime".into(), "distractors".into(), "budget".into(),
        "policy".into(), "step".into(), "input_entity".into(), "input_revision".into(),
        "input_value".into(), "input_kind".into(), "write_result".into(), "evicted_entity".into(),
        "evicted_revision".into(), "store_size".into(), "update_ops".into(),
        "rejected_stale_writes".into(), "watermark_entries".into(),
    ]);
    line(&mut out, &[
        "ROW_HEADER".into(), "regime".into(), "distractors".into(), "budget".into(),
        "policy".into(), "query_entity".into(), "query_kind".into(), "expected_revision".into(),
        "found_revision".into(), "expected_value".into(), "returned_value".into(), "success".into(),
        "stale_answer".into(), "missing_answer".into(), "duplicate_versions".into(),
        "store_size".into(), "write_ops".into(), "update_ops".into(),
        "rejected_stale_writes".into(), "watermark_entries".into(),
        "watermark_payload_bytes".into(), "capacity_overrun".into(),
    ]);
    line(&mut out, &[
        "SUMMARY_HEADER".into(), "policy".into(), "regime".into(), "distractors".into(),
        "budget".into(), "queries".into(), "success_rate".into(), "stale_rate".into(),
        "missing_rate".into(), "duplicate_query_rate".into(), "outcome_success_rate".into(),
        "observation_success_rate".into(), "capacity_overrun_rate".into(), "mean_store_size".into(),
        "mean_write_ops".into(), "mean_update_ops".into(), "mean_rejected_stale_writes".into(),
        "mean_watermark_entries".into(), "mean_watermark_payload_bytes".into(),
    ]);

    for entry in &ledger {
        line(&mut out, &[
            "LEDGER".into(), entry.regime.label().into(), entry.distractors.to_string(),
            entry.budget.to_string(), entry.policy.label().into(), entry.step.to_string(),
            entry.input.entity.to_string(), entry.input.source_revision.to_string(),
            entry.input.value.to_string(), entry.input.kind.label().into(),
            entry.event.result.into(),
            entry.event.evicted.map(|record| record.entity.to_string()).unwrap_or_else(|| "NA".into()),
            entry.event.evicted.map(|record| record.source_revision.to_string()).unwrap_or_else(|| "NA".into()),
            entry.event.store_size.to_string(), entry.event.update_ops.to_string(),
            entry.event.rejected_stale_writes.to_string(), entry.event.watermark_entries.to_string(),
        ]);
    }

    for result in &query_rows {
        line(&mut out, &[
            "ROW".into(), result.regime.label().into(), result.distractors.to_string(),
            result.budget.to_string(), result.policy.label().into(), result.query_entity.to_string(),
            result.query_kind.label().into(), result.expected_revision.to_string(),
            opt_u16(result.found_revision), result.expected_value.to_string(),
            opt_u32(result.returned_value), flag(result.success), flag(result.stale_answer),
            flag(result.missing_answer), result.duplicate_versions.to_string(),
            result.store_size.to_string(), result.write_ops.to_string(), result.update_ops.to_string(),
            result.rejected_stale_writes.to_string(), result.watermark_entries.to_string(),
            result.watermark_payload_bytes.to_string(), flag(result.capacity_overrun),
        ]);
    }

    for regime in Regime::ALL {
        for distractors in DISTRACTOR_LEVELS {
            for budget in BUDGETS {
                for policy in Policy::ALL {
                    let rows: Vec<&QueryResult> = query_rows.iter().filter(|r| {
                        r.regime == regime && r.distractors == distractors
                            && r.budget == budget && r.policy == policy
                    }).collect();
                    let successful = rows.iter().filter(|r| r.success).count();
                    let stale = rows.iter().filter(|r| r.stale_answer).count();
                    let missing = rows.iter().filter(|r| r.missing_answer).count();
                    let duplicate = rows.iter().filter(|r| r.duplicate_versions > 0).count();
                    let outcome: Vec<&QueryResult> = rows.iter()
                        .copied().filter(|r| r.query_kind == EvidenceKind::Outcome).collect();
                    let observation: Vec<&QueryResult> = rows.iter()
                        .copied().filter(|r| r.query_kind == EvidenceKind::Observation).collect();
                    let overruns = rows.iter().filter(|r| r.capacity_overrun).count();
                    let mean_store_size = rows.iter().map(|r| r.store_size as f64).sum::<f64>()
                        / rows.len().max(1) as f64;
                    let mean_write_ops = rows.iter().map(|r| r.write_ops as f64).sum::<f64>()
                        / rows.len().max(1) as f64;
                    let mean_update_ops = rows.iter().map(|r| r.update_ops as f64).sum::<f64>()
                        / rows.len().max(1) as f64;
                    let mean_rejected = rows.iter().map(|r| r.rejected_stale_writes as f64)
                        .sum::<f64>() / rows.len().max(1) as f64;

                    line(&mut out, &[
                        "SUMMARY".into(), policy.label().into(), regime.label().into(),
                        distractors.to_string(), budget.to_string(), rows.len().to_string(),
                        format!("{:.3}", rate(successful, rows.len())),
                        format!("{:.3}", rate(stale, rows.len())),
                        format!("{:.3}", rate(missing, rows.len())),
                        format!("{:.3}", rate(duplicate, rows.len())),
                        format!("{:.3}", rate(outcome.iter().filter(|r| r.success).count(), outcome.len())),
                        format!("{:.3}", rate(observation.iter().filter(|r| r.success).count(), observation.len())),
                        format!("{:.3}", rate(overruns, rows.len())),
                        format!("{:.3}", mean_store_size),
                        format!("{:.3}", mean_write_ops),
                        format!("{:.3}", mean_update_ops),
                        format!("{:.3}", mean_rejected),
                        format!("{:.3}", rows.iter().map(|r| r.watermark_entries as f64).sum::<f64>() / rows.len().max(1) as f64),
                        format!("{:.3}", rows.iter().map(|r| r.watermark_payload_bytes as f64).sum::<f64>() / rows.len().max(1) as f64),
                    ]);
                }
            }
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
    fn source_revision_upsert_rejects_a_late_stale_write() {
        let (stream, _) = build_stream(Regime::ArrivalReversal, 0);
        let mut arrival = Store::new(Policy::ArrivalUpsertFifo, 12);
        let mut source = Store::new(Policy::SourceRevisionUpsertFifo, 12);
        for record in stream.iter().copied() {
            arrival.insert(record);
            source.insert(record);
        }
        assert_eq!(arrival.retrieve(0).0.unwrap().source_revision, 1);
        assert_eq!(source.retrieve(0).0.unwrap().source_revision, 2);
        assert!(source.rejected_stale_writes >= usize::from(CORE_ENTITIES));
    }

    #[test]
    fn fifo_records_exposes_duplicate_version_occupancy_after_updates() {
        let (stream, _) = build_stream(Regime::UpdateChurn, 0);
        let mut fifo = Store::new(Policy::FifoRecords, 48);
        let mut keyed = Store::new(Policy::SourceRevisionUpsertFifo, 48);
        for record in stream.iter().copied() {
            fifo.insert(record);
            keyed.insert(record);
        }
        assert!(fifo.retrieve(0).1 > 0);
        assert_eq!(keyed.retrieve(0).1, 0);
    }

    #[test]
    fn persistent_watermark_rejects_stale_arrival_even_after_payload_eviction() {
        let (stream, truth) = build_stream(Regime::ArrivalReversal, 72);
        let mut unwatermarked = Store::new(Policy::SourceRevisionUpsertFifo, 4);
        let mut watermarked = Store::new(Policy::RevisionWatermarkEvidenceTier, 4);
        for record in stream.iter().copied() {
            unwatermarked.insert(record);
            watermarked.insert(record);
        }
        assert!(unwatermarked.retrieve(8).0.is_some_and(|r| r.source_revision < truth[8].source_revision));
        assert!(watermarked.rejected_stale_writes >= usize::from(CORE_ENTITIES));
        for entity in 0..CORE_ENTITIES {
            assert!(!watermarked.retrieve(entity).0.is_some_and(|r| r.source_revision < truth[usize::from(entity)].source_revision));
        }
        assert!(watermarked.records.len() <= 4);
        assert!(watermarked.watermarks.len() > watermarked.records.len());
    }

    #[test]
    fn all_policies_fail_closed_at_the_declared_capacity() {
        for regime in Regime::ALL {
            let (stream, _) = build_stream(regime, 72);
            for budget in BUDGETS {
                for policy in Policy::ALL {
                    let mut store = Store::new(policy, budget);
                    for record in stream.iter().copied() {
                        store.insert(record);
                        assert!(store.records.len() <= budget);
                    }
                }
            }
        }
    }

    #[test]
    fn evidence_tier_never_evicts_a_grounded_record_before_an_inference_when_overfull() {
        let mut store = Store::new(Policy::EvidenceTierBounded, 4);
        for entity in 0..4 {
            store.insert(Record {
                entity: 200 + entity,
                source_revision: 1,
                value: u32::from(entity),
                kind: EvidenceKind::Inference,
                arrival: entity as u32,
            });
        }
        let grounded = Record {
            entity: 1,
            source_revision: 1,
            value: 101,
            kind: EvidenceKind::Observation,
            arrival: 4,
        };
        let event = store.insert(grounded);
        assert_eq!(event.evicted.unwrap().kind, EvidenceKind::Inference);
        assert!(store.records.iter().any(|r| r.entity == 1));
        assert!(store.records.len() <= 4);
    }

    #[test]
    fn query_entity_is_not_an_input_to_storage_or_eviction_policy() {
        let source = include_str!("vision-memory-budget-interference.rs");
        let start = source.find("fn insert(&mut self, record: Record)").unwrap();
        let end = source[start..].find("\n    fn evidence_tier_eviction_index(").unwrap() + start;
        let writer = &source[start..end];
        assert!(!writer.contains("query_entity"));
        assert!(!writer.contains("expected_value"));
        assert!(!writer.contains("truth["));
        assert!(!writer.contains("Scenario"));
    }

    #[test]
    fn report_is_deterministic_and_every_schema_has_exact_width() {
        let first = make_report();
        let second = make_report();
        assert_eq!(first, second);
        assert_eq!(first.lines().filter(|line| line.starts_with("LEDGER_HEADER\t")).count(), 1);
        assert_eq!(first.lines().filter(|line| line.starts_with("ROW_HEADER\t")).count(), 1);
        assert_eq!(first.lines().filter(|line| line.starts_with("SUMMARY_HEADER\t")).count(), 1);
        assert_eq!(first.lines().filter(|line| line.starts_with("ROW\t")).count(), 4 * 4 * 3 * 5 * usize::from(CORE_ENTITIES));
        assert_eq!(first.lines().filter(|line| line.starts_with("SUMMARY\t")).count(), 4 * 4 * 3 * 5);
        for record in first.lines() {
            if record.starts_with("LEDGER_HEADER\t") || record.starts_with("LEDGER\t") {
                assert_eq!(record.split('\t').count(), 17, "ledger schema: {record}");
            } else if record.starts_with("ROW_HEADER\t") || record.starts_with("ROW\t") {
                assert_eq!(record.split('\t').count(), 22, "row schema: {record}");
            } else if record.starts_with("SUMMARY_HEADER\t") || record.starts_with("SUMMARY\t") {
                assert_eq!(record.split('\t').count(), 19, "summary schema: {record}");
            }
        }
    }
}
