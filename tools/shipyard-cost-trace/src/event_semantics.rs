//! Explicit quantity semantics for bitemporal shipyard evidence intake.
//!
//! This module is a normalization boundary for the V0 accounting kernel. Pass only the
//! normalized quantity to `audit`; never pass cumulative snapshots directly as if each were
//! an additive contribution. The source records remain immutable and their IDs remain attached
//! to the normalized result.
//!
//! This is intentionally not an ERP/MES connector. Source authentication, permissions, units,
//! measurement quality and configuration applicability still require separate evidence checks.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::Stage;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct QuantityIdentity {
    pub build_id: String,
    pub configuration_sha256: String,
    pub task_id: String,
    pub quantity_key: String,
    pub unit: String,
    pub stage: Stage,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuantityRecordKind {
    /// A new, non-negative contribution that can be added to other deltas.
    Delta { quantity: f64 },
    /// A cumulative value for the entire stream as of this sequence, not an increment.
    CumulativeSnapshot { quantity: f64 },
    /// Replace an earlier delta's value. Corrections target the original delta, not another
    /// correction; repeated corrections to one target are rejected as ambiguous.
    Correction {
        target_record_id: String,
        replacement_quantity: f64,
    },
    /// Remove an earlier delta's effect without deleting its immutable source record.
    Reversal { target_record_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantityRecord {
    /// Stable, source-assigned identity. Retries must reuse the same ID.
    pub record_id: String,
    /// Stable identity for one independently ordered quantity stream.
    pub stream_id: String,
    /// Strictly unique within the stream; snapshots must also be time-monotonic by sequence.
    pub sequence: u64,
    /// Source event time in Unix milliseconds, not the ingestion time.
    pub event_time_ms: i64,
    /// Time this system first recorded the immutable source record, Unix milliseconds.
    pub recorded_at_ms: i64,
    pub identity: QuantityIdentity,
    pub kind: QuantityRecordKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsOfCutoff {
    pub event_time_ms: i64,
    pub recorded_at_ms: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormalizedSemantics {
    /// Effective delta contributions, including explicit correction/reversal effects.
    AdditiveDeltas,
    /// Latest sequence-visible cumulative value from one stream.
    LatestCumulativeSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedQuantity {
    pub identity: QuantityIdentity,
    pub quantity: f64,
    pub semantics: NormalizedSemantics,
    pub as_of: AsOfCutoff,
    /// All eligible source records used to establish the normalized value, sorted by ID.
    pub evidence_record_ids: Vec<String>,
    /// Stable source streams represented by this normalized value, sorted and unique.
    pub source_stream_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum StreamMode {
    Delta,
    Snapshot,
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn mode(kind: &QuantityRecordKind) -> StreamMode {
    match kind {
        QuantityRecordKind::CumulativeSnapshot { .. } => StreamMode::Snapshot,
        QuantityRecordKind::Delta { .. }
        | QuantityRecordKind::Correction { .. }
        | QuantityRecordKind::Reversal { .. } => StreamMode::Delta,
    }
}

fn validate_quantity(value: f64, record_id: &str, errors: &mut BTreeSet<String>) {
    if !value.is_finite() || value < 0.0 {
        errors.insert(format!(
            "{record_id}: quantity must be finite and non-negative"
        ));
    }
}

/// Normalize an append-only source corpus into one quantity per explicit build/configuration/task
/// axis/unit/stage key at a bitemporal cutoff.
///
/// A record is visible only if *both* its source event time and system-recorded time are at or
/// before the respective cutoff. Therefore a late-arriving event cannot change a report made at
/// an earlier recorded-time cutoff. Empty output means no eligible quantity evidence; it does not
/// mean zero.
///
/// Delta streams sum effective deltas across independently identified streams. A correction
/// replaces one earlier delta; a reversal cancels one earlier delta. Each target may have at most
/// one applicable correction or reversal. Snapshot streams select the greatest eligible sequence,
/// never sum snapshots, and require strictly increasing event times with increasing sequence.
///
/// This function fails closed on mixed snapshot/delta semantics for one identity, multiple snapshot
/// streams for one identity, reused IDs/sequences, causal or lineage errors, and ambiguous corrections.
/// The existing V0 `audit` function remains a delta aggregator and does not infer this policy.
pub fn normalize_quantities(
    records: &[QuantityRecord],
    as_of: AsOfCutoff,
) -> Result<Vec<NormalizedQuantity>, Vec<String>> {
    let eligible: Vec<&QuantityRecord> = records
        .iter()
        .filter(|record| {
            record.event_time_ms <= as_of.event_time_ms
                && record.recorded_at_ms <= as_of.recorded_at_ms
        })
        .collect();

    if eligible.is_empty() {
        return Ok(Vec::new());
    }

    let mut errors = BTreeSet::new();
    let mut ids: HashSet<&str> = HashSet::new();
    let mut sequences: HashSet<(&str, u64)> = HashSet::new();

    for record in &eligible {
        if !is_nonempty(&record.record_id) {
            errors.insert("record has a blank record_id".to_owned());
        }
        if !is_nonempty(&record.stream_id) {
            errors.insert(format!("{}: stream_id is missing", record.record_id));
        }
        if !ids.insert(record.record_id.as_str()) {
            errors.insert(format!("{}: duplicate record_id", record.record_id));
        }
        if !sequences.insert((record.stream_id.as_str(), record.sequence)) {
            errors.insert(format!(
                "{}: duplicate sequence {} in stream {}",
                record.record_id, record.sequence, record.stream_id
            ));
        }

        let identity = &record.identity;
        for (name, value) in [
            ("build_id", identity.build_id.as_str()),
            ("task_id", identity.task_id.as_str()),
            ("quantity_key", identity.quantity_key.as_str()),
            ("unit", identity.unit.as_str()),
        ] {
            if !is_nonempty(value) {
                errors.insert(format!("{}: {name} is missing", record.record_id));
            }
        }
        if !is_sha256(&identity.configuration_sha256) {
            errors.insert(format!(
                "{}: configuration_sha256 must be 64 hexadecimal characters",
                record.record_id
            ));
        }

        match &record.kind {
            QuantityRecordKind::Delta { quantity }
            | QuantityRecordKind::CumulativeSnapshot { quantity } => {
                validate_quantity(*quantity, &record.record_id, &mut errors);
            }
            QuantityRecordKind::Correction {
                replacement_quantity,
                target_record_id,
            } => {
                validate_quantity(*replacement_quantity, &record.record_id, &mut errors);
                if !is_nonempty(target_record_id) {
                    errors.insert(format!(
                        "{}: correction target_record_id is missing",
                        record.record_id
                    ));
                }
            }
            QuantityRecordKind::Reversal { target_record_id } => {
                if !is_nonempty(target_record_id) {
                    errors.insert(format!(
                        "{}: reversal target_record_id is missing",
                        record.record_id
                    ));
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors.into_iter().collect());
    }

    let by_id: HashMap<&str, &QuantityRecord> = eligible
        .iter()
        .map(|record| (record.record_id.as_str(), *record))
        .collect();

    let mut stream_meta: BTreeMap<String, (QuantityIdentity, StreamMode)> = BTreeMap::new();
    let mut identity_modes: BTreeMap<QuantityIdentity, StreamMode> = BTreeMap::new();
    let mut snapshot_streams: BTreeMap<QuantityIdentity, BTreeSet<String>> = BTreeMap::new();
    let mut grouped: BTreeMap<QuantityIdentity, Vec<&QuantityRecord>> = BTreeMap::new();

    for record in &eligible {
        let current_mode = mode(&record.kind);
        if let Some((previous_identity, previous_mode)) = stream_meta.get(&record.stream_id) {
            if previous_identity != &record.identity {
                errors.insert(format!(
                    "{}: stream {} changes quantity identity",
                    record.record_id, record.stream_id
                ));
            }
            if *previous_mode != current_mode {
                errors.insert(format!(
                    "{}: stream {} mixes delta and snapshot semantics",
                    record.record_id, record.stream_id
                ));
            }
        } else {
            stream_meta.insert(
                record.stream_id.clone(),
                (record.identity.clone(), current_mode),
            );
        }

        if let Some(previous_mode) = identity_modes.get(&record.identity) {
            if *previous_mode != current_mode {
                errors.insert(format!(
                    "{}: identity mixes snapshot and delta semantics",
                    record.record_id
                ));
            }
        } else {
            identity_modes.insert(record.identity.clone(), current_mode);
        }

        if current_mode == StreamMode::Snapshot {
            snapshot_streams
                .entry(record.identity.clone())
                .or_default()
                .insert(record.stream_id.clone());
        }
        grouped
            .entry(record.identity.clone())
            .or_default()
            .push(*record);
    }

    for (identity, streams) in &snapshot_streams {
        if streams.len() > 1 {
            errors.insert(format!(
                "{} / {}: cumulative identity spans multiple snapshot streams",
                identity.build_id, identity.quantity_key
            ));
        }
    }

    let mut corrections: BTreeMap<&str, Vec<&QuantityRecord>> = BTreeMap::new();
    for record in &eligible {
        let target_id = match &record.kind {
            QuantityRecordKind::Correction {
                target_record_id, ..
            }
            | QuantityRecordKind::Reversal { target_record_id } => Some(target_record_id.as_str()),
            QuantityRecordKind::Delta { .. } | QuantityRecordKind::CumulativeSnapshot { .. } => {
                None
            }
        };
        let Some(target_id) = target_id else {
            continue;
        };

        match by_id.get(target_id) {
            None => {
                errors.insert(format!(
                    "{}: correction/reversal target {} is not available at this as-of cutoff",
                    record.record_id, target_id
                ));
            }
            Some(target) => {
                if !matches!(&target.kind, QuantityRecordKind::Delta { .. }) {
                    errors.insert(format!(
                        "{}: correction/reversal target {} is not an original delta",
                        record.record_id, target_id
                    ));
                }
                if target.stream_id != record.stream_id || target.identity != record.identity {
                    errors.insert(format!(
                        "{}: correction/reversal target {} crosses stream or quantity identity",
                        record.record_id, target_id
                    ));
                }
                if target.sequence >= record.sequence
                    || target.event_time_ms > record.event_time_ms
                    || target.recorded_at_ms > record.recorded_at_ms
                {
                    errors.insert(format!(
                        "{}: correction/reversal target {} is not causally prior",
                        record.record_id, target_id
                    ));
                }
                corrections.entry(target_id).or_default().push(*record);
            }
        }
    }

    for (target_id, actions) in &corrections {
        if actions.len() > 1 {
            errors.insert(format!(
                "{target_id}: multiple applicable corrections/reversals are ambiguous"
            ));
        }
    }

    for records_for_identity in grouped.values_mut() {
        records_for_identity.sort_by(|left, right| {
            left.stream_id
                .cmp(&right.stream_id)
                .then_with(|| left.sequence.cmp(&right.sequence))
                .then_with(|| left.record_id.cmp(&right.record_id))
        });

        if identity_modes.get(&records_for_identity[0].identity) == Some(&StreamMode::Snapshot) {
            let mut previous_event_time: Option<i64> = None;
            for record in records_for_identity.iter() {
                if previous_event_time.is_some_and(|previous| record.event_time_ms <= previous) {
                    errors.insert(format!(
                        "{}: snapshot event time does not increase with sequence",
                        record.record_id
                    ));
                }
                previous_event_time = Some(record.event_time_ms);
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors.into_iter().collect());
    }

    let mut normalized = Vec::with_capacity(grouped.len());
    for (identity, records_for_identity) in grouped {
        let semantics = identity_modes[&identity];
        let mut source_stream_ids = BTreeSet::new();
        let mut evidence_record_ids = BTreeSet::new();

        let quantity = match semantics {
            StreamMode::Snapshot => {
                let latest = records_for_identity
                    .iter()
                    .filter(|record| {
                        matches!(&record.kind, QuantityRecordKind::CumulativeSnapshot { .. })
                    })
                    .max_by_key(|record| record.sequence)
                    .expect("snapshot identity has at least one eligible snapshot");
                for record in &records_for_identity {
                    source_stream_ids.insert(record.stream_id.clone());
                    evidence_record_ids.insert(record.record_id.clone());
                }
                match &latest.kind {
                    QuantityRecordKind::CumulativeSnapshot { quantity } => *quantity,
                    _ => unreachable!("filtered to cumulative snapshots"),
                }
            }
            StreamMode::Delta => {
                let mut total = 0.0_f64;
                for record in &records_for_identity {
                    source_stream_ids.insert(record.stream_id.clone());
                    evidence_record_ids.insert(record.record_id.clone());
                    if let Some(actions) = corrections.get(record.record_id.as_str()) {
                        for action in actions {
                            evidence_record_ids.insert(action.record_id.clone());
                        }
                    }

                    let contribution = match &record.kind {
                        QuantityRecordKind::Delta { quantity } => {
                            match corrections.get(record.record_id.as_str()).map(|v| v[0]) {
                                Some(action) => match &action.kind {
                                    QuantityRecordKind::Correction {
                                        replacement_quantity,
                                        ..
                                    } => *replacement_quantity,
                                    QuantityRecordKind::Reversal { .. } => 0.0,
                                    _ => unreachable!("only corrective records target a delta"),
                                },
                                None => *quantity,
                            }
                        }
                        QuantityRecordKind::CumulativeSnapshot { .. } => {
                            unreachable!("delta identity has no snapshot records")
                        }
                        QuantityRecordKind::Correction { .. }
                        | QuantityRecordKind::Reversal { .. } => 0.0,
                    };
                    total += contribution;
                    if !total.is_finite() {
                        return Err(vec![format!(
                            "{}: normalized quantity aggregation overflow",
                            record.record_id
                        )]);
                    }
                }
                total
            }
        };

        normalized.push(NormalizedQuantity {
            identity,
            quantity,
            semantics: match semantics {
                StreamMode::Delta => NormalizedSemantics::AdditiveDeltas,
                StreamMode::Snapshot => NormalizedSemantics::LatestCumulativeSnapshot,
            },
            as_of,
            evidence_record_ids: evidence_record_ids.into_iter().collect(),
            source_stream_ids: source_stream_ids.into_iter().collect(),
        });
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn record(
        record_id: &str,
        sequence: u64,
        event_time_ms: i64,
        recorded_at_ms: i64,
        kind: QuantityRecordKind,
    ) -> QuantityRecord {
        QuantityRecord {
            record_id: record_id.to_owned(),
            stream_id: "crew-clock-1".to_owned(),
            sequence,
            event_time_ms,
            recorded_at_ms,
            identity: QuantityIdentity {
                build_id: "build-001".to_owned(),
                configuration_sha256: CONFIG.to_owned(),
                task_id: "weld-001".to_owned(),
                quantity_key: "labor.welder.hours".to_owned(),
                unit: "hour".to_owned(),
                stage: Stage::Observed,
            },
            kind,
        }
    }

    fn cutoff() -> AsOfCutoff {
        AsOfCutoff {
            event_time_ms: 1_000,
            recorded_at_ms: 1_000,
        }
    }

    #[test]
    fn cumulative_snapshots_select_latest_value_instead_of_summing() {
        let records = vec![
            record(
                "snapshot-1",
                1,
                100,
                100,
                QuantityRecordKind::CumulativeSnapshot { quantity: 12.0 },
            ),
            record(
                "snapshot-2",
                2,
                200,
                200,
                QuantityRecordKind::CumulativeSnapshot { quantity: 14.0 },
            ),
        ];
        let normalized = normalize_quantities(&records, cutoff()).unwrap();
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].quantity, 14.0);
        assert_eq!(
            normalized[0].semantics,
            NormalizedSemantics::LatestCumulativeSnapshot
        );
        assert_eq!(
            normalized[0].evidence_record_ids,
            vec!["snapshot-1", "snapshot-2"]
        );
    }

    #[test]
    fn additive_deltas_are_summed() {
        let records = vec![
            record(
                "delta-1",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 12.0 },
            ),
            record(
                "delta-2",
                2,
                200,
                200,
                QuantityRecordKind::Delta { quantity: 14.0 },
            ),
        ];
        let normalized = normalize_quantities(&records, cutoff()).unwrap();
        assert_eq!(normalized[0].quantity, 26.0);
        assert_eq!(normalized[0].semantics, NormalizedSemantics::AdditiveDeltas);
    }

    #[test]
    fn correction_replaces_target_delta_without_mutating_source_record() {
        let records = vec![
            record(
                "delta-1",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 12.0 },
            ),
            record(
                "delta-2",
                2,
                200,
                200,
                QuantityRecordKind::Delta { quantity: 14.0 },
            ),
            record(
                "correct-1",
                3,
                300,
                300,
                QuantityRecordKind::Correction {
                    target_record_id: "delta-1".to_owned(),
                    replacement_quantity: 10.0,
                },
            ),
        ];
        let normalized = normalize_quantities(&records, cutoff()).unwrap();
        assert_eq!(normalized[0].quantity, 24.0);
        assert_eq!(
            normalized[0].evidence_record_ids,
            vec!["correct-1", "delta-1", "delta-2"]
        );
    }

    #[test]
    fn reversal_cancels_target_delta_without_deleting_history() {
        let records = vec![
            record(
                "delta-1",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 12.0 },
            ),
            record(
                "delta-2",
                2,
                200,
                200,
                QuantityRecordKind::Delta { quantity: 14.0 },
            ),
            record(
                "reverse-1",
                3,
                300,
                300,
                QuantityRecordKind::Reversal {
                    target_record_id: "delta-1".to_owned(),
                },
            ),
        ];
        let normalized = normalize_quantities(&records, cutoff()).unwrap();
        assert_eq!(normalized[0].quantity, 14.0);
        assert!(
            normalized[0]
                .evidence_record_ids
                .contains(&"reverse-1".to_owned())
        );
        assert!(
            normalized[0]
                .evidence_record_ids
                .contains(&"delta-1".to_owned())
        );
    }

    #[test]
    fn late_arrival_does_not_change_earlier_recorded_as_of_report() {
        let records = vec![
            record(
                "delta-1",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 12.0 },
            ),
            record(
                "late-delta",
                2,
                200,
                1_500,
                QuantityRecordKind::Delta { quantity: 14.0 },
            ),
        ];
        let earlier = normalize_quantities(
            &records,
            AsOfCutoff {
                event_time_ms: 1_000,
                recorded_at_ms: 1_000,
            },
        )
        .unwrap();
        assert_eq!(earlier[0].quantity, 12.0);

        let later = normalize_quantities(
            &records,
            AsOfCutoff {
                event_time_ms: 1_000,
                recorded_at_ms: 2_000,
            },
        )
        .unwrap();
        assert_eq!(later[0].quantity, 26.0);
    }

    #[test]
    fn mixed_snapshot_and_delta_semantics_fail_closed() {
        let records = vec![
            record(
                "snapshot",
                1,
                100,
                100,
                QuantityRecordKind::CumulativeSnapshot { quantity: 12.0 },
            ),
            record(
                "delta",
                2,
                200,
                200,
                QuantityRecordKind::Delta { quantity: 2.0 },
            ),
        ];
        let errors = normalize_quantities(&records, cutoff()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("mixes delta and snapshot semantics")
                    || error.contains("identity mixes snapshot and delta semantics"))
        );
    }

    #[test]
    fn snapshot_sequence_time_contradiction_fails_closed() {
        let records = vec![
            record(
                "snapshot-1",
                1,
                200,
                200,
                QuantityRecordKind::CumulativeSnapshot { quantity: 12.0 },
            ),
            record(
                "snapshot-2",
                2,
                100,
                300,
                QuantityRecordKind::CumulativeSnapshot { quantity: 14.0 },
            ),
        ];
        let errors = normalize_quantities(&records, cutoff()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("snapshot event time does not increase"))
        );
    }

    #[test]
    fn two_corrections_of_one_delta_are_rejected_as_ambiguous() {
        let records = vec![
            record(
                "delta-1",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 12.0 },
            ),
            record(
                "correct-1",
                2,
                200,
                200,
                QuantityRecordKind::Correction {
                    target_record_id: "delta-1".to_owned(),
                    replacement_quantity: 10.0,
                },
            ),
            record(
                "correct-2",
                3,
                300,
                300,
                QuantityRecordKind::Correction {
                    target_record_id: "delta-1".to_owned(),
                    replacement_quantity: 9.0,
                },
            ),
        ];
        let errors = normalize_quantities(&records, cutoff()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("multiple applicable corrections/reversals"))
        );
    }

    #[test]
    fn duplicate_record_ids_and_stream_sequences_are_rejected() {
        let duplicate_id = vec![
            record(
                "same",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 1.0 },
            ),
            record(
                "same",
                2,
                200,
                200,
                QuantityRecordKind::Delta { quantity: 2.0 },
            ),
        ];
        assert!(
            normalize_quantities(&duplicate_id, cutoff())
                .unwrap_err()
                .iter()
                .any(|error| error.contains("duplicate record_id"))
        );

        let duplicate_sequence = vec![
            record(
                "first",
                1,
                100,
                100,
                QuantityRecordKind::Delta { quantity: 1.0 },
            ),
            record(
                "second",
                1,
                200,
                200,
                QuantityRecordKind::Delta { quantity: 2.0 },
            ),
        ];
        assert!(
            normalize_quantities(&duplicate_sequence, cutoff())
                .unwrap_err()
                .iter()
                .any(|error| error.contains("duplicate sequence"))
        );
    }

    #[test]
    fn correction_cannot_cross_source_stream() {
        let mut target = record(
            "delta-1",
            1,
            100,
            100,
            QuantityRecordKind::Delta { quantity: 12.0 },
        );
        let mut correction = record(
            "correct-1",
            2,
            200,
            200,
            QuantityRecordKind::Correction {
                target_record_id: "delta-1".to_owned(),
                replacement_quantity: 10.0,
            },
        );
        correction.stream_id = "other-stream".to_owned();
        // The target is not in the same stream as the correction; this must not rewrite it.
        let errors = normalize_quantities(&[target.clone(), correction], cutoff()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("crosses stream or quantity identity"))
        );

        target.identity.task_id = "other-task".to_owned();
        let cross_identity = vec![
            target,
            record(
                "correction-2",
                2,
                250,
                250,
                QuantityRecordKind::Correction {
                    target_record_id: "delta-1".to_owned(),
                    replacement_quantity: 10.0,
                },
            ),
        ];
        assert!(
            normalize_quantities(&cross_identity, cutoff())
                .unwrap_err()
                .iter()
                .any(|error| error.contains("crosses stream or quantity identity"))
        );
    }

    #[test]
    fn independent_delta_streams_sum_in_stable_stream_order() {
        let first = record(
            "crew-a",
            1,
            100,
            100,
            QuantityRecordKind::Delta { quantity: 12.0 },
        );
        let mut second = record(
            "crew-b",
            1,
            110,
            110,
            QuantityRecordKind::Delta { quantity: 14.0 },
        );
        second.stream_id = "crew-clock-2".to_owned();

        let normalized = normalize_quantities(&[second, first], cutoff()).unwrap();
        assert_eq!(normalized[0].quantity, 26.0);
        assert_eq!(
            normalized[0].source_stream_ids,
            vec!["crew-clock-1", "crew-clock-2"]
        );
    }

    #[test]
    fn normalization_is_invariant_to_source_input_order() {
        let delta = record(
            "delta-1",
            1,
            100,
            100,
            QuantityRecordKind::Delta { quantity: 12.0 },
        );
        let next = record(
            "delta-2",
            2,
            200,
            200,
            QuantityRecordKind::Delta { quantity: 14.0 },
        );
        let correction = record(
            "correct-1",
            3,
            300,
            300,
            QuantityRecordKind::Correction {
                target_record_id: "delta-1".to_owned(),
                replacement_quantity: 10.0,
            },
        );
        let forward =
            normalize_quantities(&[delta.clone(), next.clone(), correction.clone()], cutoff())
                .unwrap();
        let reverse = normalize_quantities(&[correction, next, delta], cutoff()).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward[0].quantity, 24.0);
    }

    #[test]
    fn multiple_snapshot_streams_for_one_identity_fail_closed() {
        let first = record(
            "snapshot-a",
            1,
            100,
            100,
            QuantityRecordKind::CumulativeSnapshot { quantity: 12.0 },
        );
        let mut second = record(
            "snapshot-b",
            1,
            100,
            100,
            QuantityRecordKind::CumulativeSnapshot { quantity: 14.0 },
        );
        second.stream_id = "snapshot-stream-2".to_owned();

        let errors = normalize_quantities(&[first, second], cutoff()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("cumulative identity spans multiple snapshot streams"))
        );
    }

    #[test]
    fn no_records_before_bitemporal_cutoff_means_no_evidence_not_zero() {
        let records = vec![record(
            "later",
            1,
            2_000,
            2_000,
            QuantityRecordKind::Delta { quantity: 12.0 },
        )];
        assert!(normalize_quantities(&records, cutoff()).unwrap().is_empty());
    }
}
