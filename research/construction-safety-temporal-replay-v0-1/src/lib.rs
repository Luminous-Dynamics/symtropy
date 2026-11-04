// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic event-time reduction for construction-robot safety evidence.
//!
//! This crate consumes a bounded event batch and emits an advisory disposition
//! at an explicit cutoff. It is not a stream processor, event store, transport,
//! safety controller, PLC, or actuator interface.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_EVENTS: usize = 8192;
const MAX_SOURCES: usize = 128;
const MAX_ZONE_CELLS: usize = 4096;
const MAX_ID_BYTES: usize = 128;
const MAX_CONFIDENCE_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridCell {
    pub column: u32,
    pub row: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorModality {
    Camera,
    Lidar,
    Radar,
    Ultrasonic,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationState {
    HumanPresent,
    Clear,
    Unknown,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    WithinSyntheticEnvelope,
    StopRequiredBySyntheticPolicy,
    EvidenceInconsistent,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    PolicyMalformed,
    EventSetTooLarge,
    EventIdentityMalformed,
    EventIdentityConflict,
    RunBindingMismatch,
    UnknownSensorSource,
    SourceEpochMismatch,
    CalibrationEpochMismatch,
    CoverageGridMismatch,
    SourceDigestMalformed,
    EventSequenceInvalid,
    SourceSequenceConflict,
    SourceTimeRegression,
    SourceClockUncertaintyExcessive,
    SourceTimestampAfterReceiveWindow,
    SourceEventAfterEvaluationCutoff,
    EventCellOutsidePolicyZone,
    EventConfidenceMalformed,
    HumanPresent,
    ConflictingHumanEvidence,
    CellEvidenceMissing,
    CellEvidenceUnknown,
    CellEvidenceStale,
    CellConfidenceInsufficient,
    ModalityCoverageIncomplete,
    FaultDomainCoverageIncomplete,
    InterSensorTimeSkewExceeded,
    EvidenceWindowIncomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorSourceProfile {
    pub sensor_id: String,
    pub modality: SensorModality,
    pub fault_domain: String,
    pub source_epoch: u64,
    pub calibration_epoch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracePolicy {
    pub run_id: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
    pub coverage_grid_sha256: String,
    pub evaluated_at_ms: u64,
    pub maximum_event_age_ms: u64,
    pub required_evidence_window_ms: u64,
    pub maximum_inter_sample_gap_ms: u64,
    pub maximum_clock_uncertainty_ms: u64,
    pub maximum_inter_sensor_skew_ms: u64,
    pub minimum_confidence_ppm: u32,
    pub minimum_coverage_ppm: u32,
    pub minimum_independent_modalities: usize,
    pub minimum_independent_fault_domains: usize,
    pub required_cells: Vec<GridCell>,
    pub sources: Vec<SensorSourceProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyEvidenceEvent {
    pub event_id: String,
    pub run_id: String,
    pub source_id: String,
    pub source_epoch: u64,
    pub source_sequence: u64,
    pub source_time_ms: u64,
    pub received_at_ms: u64,
    pub clock_uncertainty_ms: u64,
    pub calibration_epoch: String,
    pub coverage_grid_sha256: String,
    pub cell: GridCell,
    pub state: ObservationState,
    pub confidence_ppm: u32,
    pub coverage_ppm: u32,
    pub source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventReference {
    pub event_id: String,
    pub source_id: String,
    pub source_epoch: u64,
    pub source_sequence: u64,
    pub source_time_ms: u64,
    pub received_at_ms: u64,
    pub source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellResult {
    pub cell: GridCell,
    pub clear_sources: Vec<String>,
    pub complete_modalities: Vec<SensorModality>,
    pub complete_fault_domains: Vec<String>,
    pub newest_source_time_ms: u64,
    pub oldest_source_time_ms: u64,
    pub maximum_event_age_ms: u64,
    pub evidence_window_start_ms: u64,
    pub maximum_observation_gap_ms: u64,
    pub temporal_sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalDecision {
    pub disposition: Disposition,
    pub reason: Option<Reason>,
    pub run_id: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
    pub coverage_grid_sha256: String,
    pub evaluated_at_ms: u64,
    pub cells: Vec<CellResult>,
    pub event_references: Vec<EventReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceCase {
    pub id: String,
    pub policy: TracePolicy,
    pub events: Vec<SafetyEvidenceEvent>,
    pub expected: Disposition,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

/// Validates digest syntax only; it does not authenticate source bytes.
pub fn is_sha256_reference(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn policy_is_valid(policy: &TracePolicy) -> bool {
    if !non_empty(&policy.run_id)
        || !non_empty(&policy.policy_id)
        || !non_empty(&policy.policy_revision)
        || !is_sha256_reference(&policy.policy_sha256)
        || !is_sha256_reference(&policy.coverage_grid_sha256)
        || policy.maximum_event_age_ms == 0
        || (policy.required_evidence_window_ms > 0 && policy.maximum_inter_sample_gap_ms == 0)
        || policy.minimum_confidence_ppm > MAX_CONFIDENCE_PPM
        || policy.minimum_coverage_ppm > MAX_CONFIDENCE_PPM
        || policy.minimum_independent_modalities == 0
        || policy.minimum_independent_fault_domains == 0
        || policy.required_cells.is_empty()
        || policy.required_cells.len() > MAX_ZONE_CELLS
        || policy.sources.is_empty()
        || policy.sources.len() > MAX_SOURCES
    {
        return false;
    }

    let cells = policy
        .required_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if cells.len() != policy.required_cells.len() {
        return false;
    }

    let mut source_ids = BTreeSet::new();
    let mut domains = BTreeSet::new();
    let mut modalities = BTreeSet::new();
    for source in &policy.sources {
        if !non_empty(&source.sensor_id)
            || !non_empty(&source.fault_domain)
            || !non_empty(&source.calibration_epoch)
            || source.source_epoch == 0
            || !source_ids.insert(source.sensor_id.as_str())
        {
            return false;
        }
        domains.insert(source.fault_domain.as_str());
        modalities.insert(source.modality);
    }

    policy.minimum_independent_modalities <= modalities.len()
        && policy.minimum_independent_fault_domains <= domains.len()
}

fn event_payload_matches(left: &SafetyEvidenceEvent, right: &SafetyEvidenceEvent) -> bool {
    left.run_id == right.run_id
        && left.source_id == right.source_id
        && left.source_epoch == right.source_epoch
        && left.source_sequence == right.source_sequence
        && left.source_time_ms == right.source_time_ms
        && left.clock_uncertainty_ms == right.clock_uncertainty_ms
        && left.calibration_epoch == right.calibration_epoch
        && left.coverage_grid_sha256 == right.coverage_grid_sha256
        && left.cell == right.cell
        && left.state == right.state
        && left.confidence_ppm == right.confidence_ppm
        && left.coverage_ppm == right.coverage_ppm
        && left.source_sha256 == right.source_sha256
}

fn make_decision(
    policy: &TracePolicy,
    disposition: Disposition,
    reason: Option<Reason>,
    cells: Vec<CellResult>,
    events: &BTreeMap<String, SafetyEvidenceEvent>,
) -> TemporalDecision {
    let mut event_references = events
        .values()
        .map(|event| EventReference {
            event_id: event.event_id.clone(),
            source_id: event.source_id.clone(),
            source_epoch: event.source_epoch,
            source_sequence: event.source_sequence,
            source_time_ms: event.source_time_ms,
            received_at_ms: event.received_at_ms,
            source_sha256: event.source_sha256.clone(),
        })
        .collect::<Vec<_>>();
    event_references.sort();

    TemporalDecision {
        disposition,
        reason,
        run_id: policy.run_id.clone(),
        policy_id: policy.policy_id.clone(),
        policy_revision: policy.policy_revision.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        coverage_grid_sha256: policy.coverage_grid_sha256.clone(),
        evaluated_at_ms: policy.evaluated_at_ms,
        cells,
        event_references,
    }
}

fn inconsistent(
    policy: &TracePolicy,
    reason: Reason,
    events: &BTreeMap<String, SafetyEvidenceEvent>,
) -> TemporalDecision {
    make_decision(
        policy,
        Disposition::EvidenceInconsistent,
        Some(reason),
        Vec::new(),
        events,
    )
}

fn stop(
    policy: &TracePolicy,
    reason: Reason,
    cells: Vec<CellResult>,
    events: &BTreeMap<String, SafetyEvidenceEvent>,
) -> TemporalDecision {
    make_decision(
        policy,
        Disposition::StopRequiredBySyntheticPolicy,
        Some(reason),
        cells,
        events,
    )
}

/// Reduces a bounded batch at one explicit event-time cutoff.
///
/// Events received after the cutoff are not available to this decision. Within
/// the available set, source epochs and sequence numbers—not receive order—
/// determine each source/cell's latest state. The same event set and cutoff
/// produce the same normalized disposition regardless of arrival permutation.
///
/// This function is advisory evidence evaluation only. It does not issue
/// commands, stop equipment, reset an emergency stop or authorize work.
pub fn evaluate(policy: &TracePolicy, input_events: &[SafetyEvidenceEvent]) -> TemporalDecision {
    let empty = BTreeMap::new();
    if !policy_is_valid(policy) {
        return make_decision(
            policy,
            Disposition::Unsupported,
            Some(Reason::PolicyMalformed),
            Vec::new(),
            &empty,
        );
    }
    if input_events.len() > MAX_EVENTS {
        return inconsistent(policy, Reason::EventSetTooLarge, &empty);
    }

    let required_cells = policy
        .required_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let sources = policy
        .sources
        .iter()
        .map(|source| (source.sensor_id.as_str(), source))
        .collect::<BTreeMap<_, _>>();

    // Sorting makes error selection deterministic even for malformed batches.
    let mut ordered_events = input_events
        .iter()
        .filter(|event| event.received_at_ms <= policy.evaluated_at_ms)
        .collect::<Vec<_>>();
    ordered_events.sort_by(|left, right| {
        left.event_id
            .cmp(&right.event_id)
            .then_with(|| left.run_id.cmp(&right.run_id))
            .then_with(|| left.source_id.cmp(&right.source_id))
            .then_with(|| left.source_epoch.cmp(&right.source_epoch))
            .then_with(|| left.source_sequence.cmp(&right.source_sequence))
            .then_with(|| left.source_time_ms.cmp(&right.source_time_ms))
            .then_with(|| left.received_at_ms.cmp(&right.received_at_ms))
            .then_with(|| left.clock_uncertainty_ms.cmp(&right.clock_uncertainty_ms))
            .then_with(|| left.calibration_epoch.cmp(&right.calibration_epoch))
            .then_with(|| left.coverage_grid_sha256.cmp(&right.coverage_grid_sha256))
            .then_with(|| left.cell.cmp(&right.cell))
            .then_with(|| left.state.cmp(&right.state))
            .then_with(|| left.confidence_ppm.cmp(&right.confidence_ppm))
            .then_with(|| left.coverage_ppm.cmp(&right.coverage_ppm))
            .then_with(|| left.source_sha256.cmp(&right.source_sha256))
    });

    // Ignore records that had not arrived by the decision cutoff. A later
    // evaluation is a new decision; it never mutates this cutoff's result.
    let mut events_by_id = BTreeMap::<String, SafetyEvidenceEvent>::new();
    for event in ordered_events {
        if !non_empty(&event.event_id)
            || !non_empty(&event.run_id)
            || !non_empty(&event.source_id)
            || !non_empty(&event.calibration_epoch)
        {
            return inconsistent(policy, Reason::EventIdentityMalformed, &events_by_id);
        }
        if event.run_id != policy.run_id {
            return inconsistent(policy, Reason::RunBindingMismatch, &events_by_id);
        }
        let Some(source) = sources.get(event.source_id.as_str()) else {
            return inconsistent(policy, Reason::UnknownSensorSource, &events_by_id);
        };
        if event.source_epoch != source.source_epoch || event.source_epoch == 0 {
            return inconsistent(policy, Reason::SourceEpochMismatch, &events_by_id);
        }
        if event.calibration_epoch != source.calibration_epoch {
            return inconsistent(policy, Reason::CalibrationEpochMismatch, &events_by_id);
        }
        if event.coverage_grid_sha256 != policy.coverage_grid_sha256 {
            return inconsistent(policy, Reason::CoverageGridMismatch, &events_by_id);
        }
        if !is_sha256_reference(&event.source_sha256) {
            return inconsistent(policy, Reason::SourceDigestMalformed, &events_by_id);
        }
        if event.source_sequence == 0 {
            return inconsistent(policy, Reason::EventSequenceInvalid, &events_by_id);
        }
        if event.clock_uncertainty_ms > policy.maximum_clock_uncertainty_ms {
            return inconsistent(
                policy,
                Reason::SourceClockUncertaintyExcessive,
                &events_by_id,
            );
        }
        if event.source_time_ms
            > event
                .received_at_ms
                .saturating_add(event.clock_uncertainty_ms)
        {
            return inconsistent(
                policy,
                Reason::SourceTimestampAfterReceiveWindow,
                &events_by_id,
            );
        }
        if event.confidence_ppm > MAX_CONFIDENCE_PPM || event.coverage_ppm > MAX_CONFIDENCE_PPM {
            return inconsistent(policy, Reason::EventConfidenceMalformed, &events_by_id);
        }
        if !required_cells.contains(&event.cell) {
            return inconsistent(policy, Reason::EventCellOutsidePolicyZone, &events_by_id);
        }

        if let Some(existing) = events_by_id.get(&event.event_id) {
            if !event_payload_matches(existing, event) {
                return inconsistent(policy, Reason::EventIdentityConflict, &events_by_id);
            }
            let mut updated = existing.clone();
            updated.received_at_ms = existing.received_at_ms.min(event.received_at_ms);
            events_by_id.insert(event.event_id.clone(), updated);
        } else {
            events_by_id.insert(event.event_id.clone(), event.clone());
        }
    }

    // A source sequence identifies one acquisition frame. All cell records in
    // that frame share one source timestamp; source time cannot regress as the
    // sequence increases within the pinned source epoch.
    let mut source_sequence_times = BTreeMap::<(String, u64), BTreeMap<u64, u64>>::new();
    let mut source_sequence_cells =
        BTreeMap::<(String, u64, u64, GridCell), SafetyEvidenceEvent>::new();

    for event in events_by_id.values() {
        let sequence_times = source_sequence_times
            .entry((event.source_id.clone(), event.source_epoch))
            .or_default();
        if let Some(existing_time) = sequence_times.get(&event.source_sequence) {
            if *existing_time != event.source_time_ms {
                return inconsistent(policy, Reason::SourceSequenceConflict, &events_by_id);
            }
        } else {
            sequence_times.insert(event.source_sequence, event.source_time_ms);
        }

        let key = (
            event.source_id.clone(),
            event.source_epoch,
            event.source_sequence,
            event.cell,
        );
        if let Some(existing) = source_sequence_cells.get(&key) {
            if !event_payload_matches(existing, event) {
                return inconsistent(policy, Reason::SourceSequenceConflict, &events_by_id);
            }
            return inconsistent(policy, Reason::EventIdentityConflict, &events_by_id);
        }
        source_sequence_cells.insert(key, event.clone());
    }

    for times in source_sequence_times.values() {
        let mut previous_time = None;
        for source_time in times.values() {
            if previous_time.is_some_and(|previous| *source_time < previous) {
                return inconsistent(policy, Reason::SourceTimeRegression, &events_by_id);
            }
            previous_time = Some(*source_time);
        }
    }

    // Reduce to the highest source sequence for each source/cell, independently
    // of the order in which records were delivered.
    let mut latest = BTreeMap::<(String, GridCell), SafetyEvidenceEvent>::new();
    for event in events_by_id.values() {
        let key = (event.source_id.clone(), event.cell);
        let replace = latest
            .get(&key)
            .is_none_or(|prior| event.source_sequence > prior.source_sequence);
        if replace {
            latest.insert(key, event.clone());
        }
    }

    let mut results = Vec::new();
    let mut modality_cells = BTreeMap::<SensorModality, BTreeSet<GridCell>>::new();
    let mut fault_domain_cells = BTreeMap::<String, BTreeSet<GridCell>>::new();

    let mut ordered_sources = policy.sources.iter().collect::<Vec<_>>();
    ordered_sources.sort_by(|left, right| left.sensor_id.cmp(&right.sensor_id));

    let evidence_window_start_ms = policy
        .evaluated_at_ms
        .saturating_sub(policy.required_evidence_window_ms);

    for cell in &required_cells {
        let mut clear_sources = Vec::new();
        let mut clear_times = Vec::new();
        let mut cell_modalities = BTreeSet::new();
        let mut cell_fault_domains = BTreeSet::new();
        let mut maximum_age_ms = 0u64;
        let mut maximum_observation_gap_ms = 0u64;
        let mut temporal_sample_count = 0usize;
        let mut saw_stale = false;
        let mut saw_low_confidence = false;
        let mut saw_unknown = false;

        for source in &ordered_sources {
            let key = (source.sensor_id.clone(), *cell);
            let Some(latest_event) = latest.get(&key) else {
                continue;
            };
            if latest_event.source_time_ms > policy.evaluated_at_ms {
                return stop(
                    policy,
                    Reason::SourceEventAfterEvaluationCutoff,
                    results,
                    &events_by_id,
                );
            }

            let history = if policy.required_evidence_window_ms == 0 {
                vec![latest_event]
            } else {
                let mut samples = events_by_id
                    .values()
                    .filter(|event| {
                        event.source_id == source.sensor_id
                            && event.cell == *cell
                            && event.source_time_ms >= evidence_window_start_ms
                            && event.source_time_ms <= policy.evaluated_at_ms
                    })
                    .collect::<Vec<_>>();
                samples.sort_by_key(|event| event.source_sequence);
                samples
            };

            if history.is_empty() {
                saw_stale = true;
                continue;
            }

            let first_source_time_ms = history[0].source_time_ms;
            let last_source_time_ms = history[history.len() - 1].source_time_ms;
            let latest_age_ms = policy.evaluated_at_ms.saturating_sub(last_source_time_ms);
            maximum_age_ms = maximum_age_ms.max(latest_age_ms);

            // A positive or conflicting observation inside the declared
            // evidence window remains material even if the newest frame is stale.
            for sample in &history {
                if sample.state == ObservationState::HumanPresent {
                    return stop(policy, Reason::HumanPresent, results, &events_by_id);
                }
                if sample.state == ObservationState::Conflicting {
                    return inconsistent(policy, Reason::ConflictingHumanEvidence, &events_by_id);
                }
            }

            if latest_age_ms > policy.maximum_event_age_ms
                || (policy.required_evidence_window_ms > 0
                    && latest_age_ms > policy.maximum_inter_sample_gap_ms)
            {
                saw_stale = true;
                continue;
            }

            if policy.required_evidence_window_ms > 0 {
                if first_source_time_ms.saturating_sub(evidence_window_start_ms)
                    > policy.maximum_inter_sample_gap_ms
                {
                    saw_stale = true;
                    continue;
                }

                let mut sample_gap_exceeded = false;
                for pair in history.windows(2) {
                    let gap_ms = pair[1]
                        .source_time_ms
                        .saturating_sub(pair[0].source_time_ms);
                    maximum_observation_gap_ms = maximum_observation_gap_ms.max(gap_ms);
                    if gap_ms > policy.maximum_inter_sample_gap_ms {
                        sample_gap_exceeded = true;
                        break;
                    }
                }
                if sample_gap_exceeded {
                    saw_stale = true;
                    continue;
                }
            }

            let mut source_has_complete_clear_window = true;
            for event in &history {
                if event.state == ObservationState::Unknown {
                    saw_unknown = true;
                    source_has_complete_clear_window = false;
                    continue;
                }
                if event.confidence_ppm < policy.minimum_confidence_ppm
                    || event.coverage_ppm < policy.minimum_coverage_ppm
                {
                    saw_low_confidence = true;
                    source_has_complete_clear_window = false;
                }
            }

            if !source_has_complete_clear_window {
                continue;
            }

            clear_sources.push(source.sensor_id.clone());
            clear_times.push(last_source_time_ms);
            temporal_sample_count += history.len();
            cell_modalities.insert(source.modality);
            cell_fault_domains.insert(source.fault_domain.clone());
        }

        if clear_sources.is_empty() {
            let reason = if saw_stale {
                Reason::CellEvidenceStale
            } else if saw_low_confidence {
                Reason::CellConfidenceInsufficient
            } else if saw_unknown {
                Reason::CellEvidenceUnknown
            } else {
                Reason::CellEvidenceMissing
            };
            return stop(policy, reason, results, &events_by_id);
        }
        if cell_modalities.len() < policy.minimum_independent_modalities
            || cell_fault_domains.len() < policy.minimum_independent_fault_domains
        {
            let reason = if saw_stale {
                Reason::CellEvidenceStale
            } else if saw_low_confidence {
                Reason::CellConfidenceInsufficient
            } else if saw_unknown {
                Reason::CellEvidenceUnknown
            } else {
                Reason::CellEvidenceMissing
            };
            return stop(policy, reason, results, &events_by_id);
        }

        let newest_source_time_ms = clear_times.iter().copied().max().unwrap_or(0);
        let oldest_source_time_ms = clear_times.iter().copied().min().unwrap_or(0);
        let source_time_spread_ms = newest_source_time_ms - oldest_source_time_ms;
        if source_time_spread_ms > policy.maximum_inter_sensor_skew_ms {
            return stop(
                policy,
                Reason::InterSensorTimeSkewExceeded,
                results,
                &events_by_id,
            );
        }

        for modality in &cell_modalities {
            modality_cells.entry(*modality).or_default().insert(*cell);
        }
        for fault_domain in &cell_fault_domains {
            fault_domain_cells
                .entry(fault_domain.clone())
                .or_default()
                .insert(*cell);
        }

        clear_sources.sort();
        results.push(CellResult {
            cell: *cell,
            clear_sources,
            complete_modalities: cell_modalities.into_iter().collect(),
            complete_fault_domains: cell_fault_domains.into_iter().collect(),
            newest_source_time_ms,
            oldest_source_time_ms,
            maximum_event_age_ms: maximum_age_ms,
            evidence_window_start_ms,
            maximum_observation_gap_ms,
            temporal_sample_count,
        });
    }

    let complete_modalities = modality_cells
        .iter()
        .filter(|(_, cells)| required_cells.is_subset(*cells))
        .map(|(modality, _)| *modality)
        .collect::<Vec<_>>();
    if complete_modalities.len() < policy.minimum_independent_modalities {
        return stop(
            policy,
            Reason::ModalityCoverageIncomplete,
            results,
            &events_by_id,
        );
    }

    let complete_fault_domains = fault_domain_cells
        .iter()
        .filter(|(_, cells)| required_cells.is_subset(*cells))
        .map(|(domain, _)| domain.clone())
        .collect::<Vec<_>>();
    if complete_fault_domains.len() < policy.minimum_independent_fault_domains {
        return stop(
            policy,
            Reason::FaultDomainCoverageIncomplete,
            results,
            &events_by_id,
        );
    }

    make_decision(
        policy,
        Disposition::WithinSyntheticEnvelope,
        None,
        results,
        &events_by_id,
    )
}

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn base_policy() -> TracePolicy {
    TracePolicy {
        run_id: "site-run-001".to_owned(),
        policy_id: "safety-policy-01".to_owned(),
        policy_revision: "revision-3".to_owned(),
        policy_sha256: digest('a'),
        coverage_grid_sha256: digest('b'),
        evaluated_at_ms: 5_000,
        maximum_event_age_ms: 500,
        required_evidence_window_ms: 0,
        maximum_inter_sample_gap_ms: 0,
        maximum_clock_uncertainty_ms: 20,
        maximum_inter_sensor_skew_ms: 100,
        minimum_confidence_ppm: 900_000,
        minimum_coverage_ppm: 900_000,
        minimum_independent_modalities: 2,
        minimum_independent_fault_domains: 2,
        required_cells: vec![
            GridCell { column: 0, row: 0 },
            GridCell { column: 1, row: 0 },
        ],
        sources: vec![
            SensorSourceProfile {
                sensor_id: "lidar-front".to_owned(),
                modality: SensorModality::Lidar,
                fault_domain: "range-power-a".to_owned(),
                source_epoch: 1,
                calibration_epoch: "calibration-7".to_owned(),
            },
            SensorSourceProfile {
                sensor_id: "camera-front".to_owned(),
                modality: SensorModality::Camera,
                fault_domain: "perception-power-b".to_owned(),
                source_epoch: 1,
                calibration_epoch: "calibration-4".to_owned(),
            },
        ],
    }
}

fn event(
    source_id: &str,
    source_sequence: u64,
    source_time_ms: u64,
    cell: GridCell,
    state: ObservationState,
) -> SafetyEvidenceEvent {
    let calibration_epoch = if source_id == "lidar-front" {
        "calibration-7"
    } else {
        "calibration-4"
    };
    let source_tag = if source_id == "lidar-front" { 'c' } else { 'd' };
    SafetyEvidenceEvent {
        event_id: format!(
            "{}-epoch1-seq{}-cell{}-{}",
            source_id, source_sequence, cell.column, cell.row
        ),
        run_id: "site-run-001".to_owned(),
        source_id: source_id.to_owned(),
        source_epoch: 1,
        source_sequence,
        source_time_ms,
        received_at_ms: source_time_ms + 5,
        clock_uncertainty_ms: 5,
        calibration_epoch: calibration_epoch.to_owned(),
        coverage_grid_sha256: digest('b'),
        cell,
        state,
        confidence_ppm: 990_000,
        coverage_ppm: 980_000,
        source_sha256: digest(source_tag),
    }
}

fn baseline_events() -> Vec<SafetyEvidenceEvent> {
    let mut events = Vec::new();
    for sensor in ["lidar-front", "camera-front"] {
        for cell in [
            GridCell { column: 0, row: 0 },
            GridCell { column: 1, row: 0 },
        ] {
            events.push(event(sensor, 1, 4_900, cell, ObservationState::Clear));
        }
    }
    events
}

fn window_policy() -> TracePolicy {
    let mut policy = base_policy();
    policy.required_evidence_window_ms = 200;
    policy.maximum_inter_sample_gap_ms = 100;
    policy.maximum_event_age_ms = 150;
    policy
}

fn window_events() -> Vec<SafetyEvidenceEvent> {
    let mut events = Vec::new();
    for sensor in ["lidar-front", "camera-front"] {
        for cell in [
            GridCell { column: 0, row: 0 },
            GridCell { column: 1, row: 0 },
        ] {
            for (sequence, source_time_ms) in [(1, 4_800), (2, 4_900), (3, 4_990)] {
                events.push(event(
                    sensor,
                    sequence,
                    source_time_ms,
                    cell,
                    ObservationState::Clear,
                ));
            }
        }
    }
    events
}

fn add_case(
    cases: &mut Vec<TraceCase>,
    id: String,
    policy: TracePolicy,
    events: Vec<SafetyEvidenceEvent>,
    expected: Disposition,
) {
    cases.push(TraceCase {
        id,
        policy,
        events,
        expected,
    });
}

/// Frozen event-time synthetic trace corpus. Expected outcomes are pinned here
/// and are not generated by the reducer under test.
pub fn frozen_trace_cases() -> Vec<TraceCase> {
    let base_policy = base_policy();
    let base = baseline_events();
    let mut cases = Vec::new();

    add_case(
        &mut cases,
        "TIME-001-synchronized-clear-baseline".to_owned(),
        base_policy.clone(),
        base.clone(),
        Disposition::WithinSyntheticEnvelope,
    );

    let mut events = base.clone();
    events.reverse();
    add_case(
        &mut cases,
        "TIME-002-arrival-order-permutation".to_owned(),
        base_policy.clone(),
        events,
        Disposition::WithinSyntheticEnvelope,
    );

    let mut events = base.clone();
    events.push(base[0].clone());
    add_case(
        &mut cases,
        "TIME-003-identical-event-duplicate".to_owned(),
        base_policy.clone(),
        events,
        Disposition::WithinSyntheticEnvelope,
    );

    let sources = ["lidar-front", "camera-front"];
    let cells = [
        GridCell { column: 0, row: 0 },
        GridCell { column: 1, row: 0 },
    ];
    let mut case_number = 4usize;

    for source in sources {
        for cell in cells {
            let name = format!(
                "TIME-{case_number:03}-human-present-{source}-{}",
                cell.column
            );
            let mut events = base.clone();
            events.push(event(
                source,
                2,
                4_950,
                cell,
                ObservationState::HumanPresent,
            ));
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::StopRequiredBySyntheticPolicy,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!("TIME-{case_number:03}-unknown-{source}-{}", cell.column);
            let mut events = base.clone();
            events.push(event(source, 2, 4_950, cell, ObservationState::Unknown));
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::StopRequiredBySyntheticPolicy,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!("TIME-{case_number:03}-missing-{source}-{}", cell.column);
            let mut events = base.clone();
            events.retain(|item| !(item.source_id == source && item.cell == cell));
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::StopRequiredBySyntheticPolicy,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!("TIME-{case_number:03}-stale-{source}-{}", cell.column);
            let mut events = base.clone();
            for source_event in &mut events {
                if source_event.source_id == source {
                    source_event.source_time_ms = 4_000;
                    source_event.received_at_ms = 4_005;
                }
            }
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::StopRequiredBySyntheticPolicy,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!(
                "TIME-{case_number:03}-low-confidence-{source}-{}",
                cell.column
            );
            let mut events = base.clone();
            if let Some(latest) = events
                .iter_mut()
                .find(|item| item.source_id == source && item.cell == cell)
            {
                latest.confidence_ppm = 100_000;
            }
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::StopRequiredBySyntheticPolicy,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!(
                "TIME-{case_number:03}-grid-mismatch-{source}-{}",
                cell.column
            );
            let mut events = base.clone();
            if let Some(latest) = events
                .iter_mut()
                .find(|item| item.source_id == source && item.cell == cell)
            {
                latest.coverage_grid_sha256 = digest('9');
            }
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::EvidenceInconsistent,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!(
                "TIME-{case_number:03}-source-epoch-mismatch-{source}-{}",
                cell.column
            );
            let mut events = base.clone();
            if let Some(latest) = events
                .iter_mut()
                .find(|item| item.source_id == source && item.cell == cell)
            {
                latest.source_epoch = 2;
            }
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::EvidenceInconsistent,
            );
            case_number += 1;
        }
    }

    for source in sources {
        for cell in cells {
            let name = format!(
                "TIME-{case_number:03}-calibration-mismatch-{source}-{}",
                cell.column
            );
            let mut events = base.clone();
            if let Some(latest) = events
                .iter_mut()
                .find(|item| item.source_id == source && item.cell == cell)
            {
                latest.calibration_epoch = "expired-calibration".to_owned();
            }
            add_case(
                &mut cases,
                name,
                base_policy.clone(),
                events,
                Disposition::EvidenceInconsistent,
            );
            case_number += 1;
        }
    }

    let mut events = base.clone();
    events[0].source_sha256 = "bad-digest".to_owned();
    add_case(
        &mut cases,
        "TIME-036-malformed-source-digest".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].clock_uncertainty_ms = 100;
    add_case(
        &mut cases,
        "TIME-037-source-clock-uncertainty-exceeded".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_time_ms = events[0].received_at_ms + 100;
    add_case(
        &mut cases,
        "TIME-038-source-timestamp-after-receive-window".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_time_ms = 5_005;
    events[0].received_at_ms = 5_000;
    events[0].clock_uncertainty_ms = 10;
    add_case(
        &mut cases,
        "TIME-039-source-event-after-evaluation-cutoff".to_owned(),
        base_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = base.clone();
    events[0].run_id = "other-run".to_owned();
    add_case(
        &mut cases,
        "TIME-040-run-binding-mismatch".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_id = "unregistered-sensor".to_owned();
    add_case(
        &mut cases,
        "TIME-041-unknown-sensor-source".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_sequence = 0;
    add_case(
        &mut cases,
        "TIME-042-zero-source-sequence".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].cell = GridCell { column: 9, row: 9 };
    add_case(
        &mut cases,
        "TIME-043-event-outside-declared-hazard-zone".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].event_id = events[1].event_id.clone();
    add_case(
        &mut cases,
        "TIME-044-event-id-conflict".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_sequence = 2;
    events[0].source_time_ms = 4_800;
    events[0].received_at_ms = 4_805;
    events[1].source_sequence = 1;
    events[1].source_time_ms = 4_900;
    events[1].received_at_ms = 4_905;
    add_case(
        &mut cases,
        "TIME-045-source-time-regresses-with-sequence".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_sequence = 2;
    events[0].source_time_ms = 4_980;
    events[0].received_at_ms = 4_985;
    events[1].source_sequence = 2;
    events[1].source_time_ms = 4_981;
    events[1].received_at_ms = 4_986;
    add_case(
        &mut cases,
        "TIME-046-one-source-sequence-has-multiple-times".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].source_sequence = 2;
    events[0].source_time_ms = 4_980;
    events[0].received_at_ms = 4_985;
    events[0].state = ObservationState::HumanPresent;
    let mut old = events[0].clone();
    old.event_id = "old-sequence-late-arrival".to_owned();
    old.source_sequence = 1;
    old.source_time_ms = 4_900;
    old.received_at_ms = 4_990;
    old.state = ObservationState::Clear;
    events.push(old);
    add_case(
        &mut cases,
        "TIME-047-newer-source-sequence-wins".to_owned(),
        base_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = base.clone();
    for item in &mut events {
        if item.source_id == "lidar-front" {
            item.source_time_ms = 4_800;
            item.received_at_ms = 4_805;
        }
    }
    events[0].state = ObservationState::HumanPresent;
    let mut late_clear = events[0].clone();
    late_clear.event_id = "late-clear-after-cutoff".to_owned();
    late_clear.source_sequence = 2;
    late_clear.source_time_ms = 4_990;
    late_clear.received_at_ms = 5_100;
    late_clear.state = ObservationState::Clear;
    events.push(late_clear);
    add_case(
        &mut cases,
        "TIME-048-late-clear-cannot-rewrite-prior-cutoff".to_owned(),
        base_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut later_policy = base_policy.clone();
    later_policy.evaluated_at_ms = 5_200;
    let mut events = base.clone();
    for item in &mut events {
        item.source_time_ms = 4_800;
        item.received_at_ms = 4_805;
    }
    // Sequence 1 records a person present; the later sequence-2 complete frame
    // clears every required cell for both modalities.
    events[0].state = ObservationState::HumanPresent;
    let mut later_frame = Vec::new();
    for source in ["lidar-front", "camera-front"] {
        for cell in [
            GridCell { column: 0, row: 0 },
            GridCell { column: 1, row: 0 },
        ] {
            later_frame.push(event(source, 2, 5_100, cell, ObservationState::Clear));
        }
    }
    events.extend(later_frame);
    add_case(
        &mut cases,
        "TIME-049-new-cutoff-consumes-new-evidence".to_owned(),
        later_policy,
        events,
        Disposition::WithinSyntheticEnvelope,
    );

    let mut skew_policy = base_policy.clone();
    skew_policy.maximum_inter_sensor_skew_ms = 10;
    let mut events = base.clone();
    for item in &mut events {
        if item.source_id == "camera-front" {
            item.source_time_ms = 4_790;
            item.received_at_ms = 4_795;
        }
    }
    add_case(
        &mut cases,
        "TIME-050-independent-sensors-exceed-time-skew".to_owned(),
        skew_policy,
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = base.clone();
    for item in &mut events {
        if item.source_id == "lidar-front" {
            item.source_time_ms = 4_499;
            item.received_at_ms = 4_999;
        }
    }
    add_case(
        &mut cases,
        "TIME-051-stale-source-time-despite-recent-arrival".to_owned(),
        base_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = base.clone();
    events[0].received_at_ms = 6_000;
    events[2].received_at_ms = 6_000;
    add_case(
        &mut cases,
        "TIME-052-newer-received-frames-not-yet-at-cutoff".to_owned(),
        base_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = base.clone();
    events[0].confidence_ppm = 1_000_001;
    add_case(
        &mut cases,
        "TIME-053-confidence-overflow".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = base.clone();
    events[0].state = ObservationState::Conflicting;
    add_case(
        &mut cases,
        "TIME-054-conflicting-human-evidence".to_owned(),
        base_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut single_domain_policy = base_policy.clone();
    single_domain_policy.sources[1].fault_domain =
        single_domain_policy.sources[0].fault_domain.clone();
    add_case(
        &mut cases,
        "TIME-055-policy-has-common-mode-only-sources".to_owned(),
        single_domain_policy,
        base.clone(),
        Disposition::Unsupported,
    );

    let mut single_modality_policy = base_policy.clone();
    single_modality_policy.sources[1].modality = single_modality_policy.sources[0].modality;
    add_case(
        &mut cases,
        "TIME-056-policy-has-one-modality-only".to_owned(),
        single_modality_policy,
        base.clone(),
        Disposition::Unsupported,
    );

    add_case(
        &mut cases,
        "TIME-057-empty-event-batch".to_owned(),
        base_policy.clone(),
        Vec::new(),
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut duplicate_policy = base_policy.clone();
    duplicate_policy.sources[1].sensor_id = duplicate_policy.sources[0].sensor_id.clone();
    add_case(
        &mut cases,
        "TIME-058-duplicate-policy-sensor-identity".to_owned(),
        duplicate_policy,
        base.clone(),
        Disposition::Unsupported,
    );

    let mut events = base.clone();
    for item in &mut events {
        if item.source_id == "lidar-front" {
            item.source_time_ms = 4_800;
            item.received_at_ms = 4_805;
        }
    }
    // The lower sequence arrives after the higher sequence but cannot replace it.
    events[0].received_at_ms = 4_990;
    events[0].state = ObservationState::HumanPresent;
    let mut newer_clear = events[0].clone();
    newer_clear.event_id = "newer-clear-arrived-first".to_owned();
    newer_clear.source_sequence = 2;
    newer_clear.source_time_ms = 4_980;
    newer_clear.received_at_ms = 4_985;
    newer_clear.state = ObservationState::Clear;
    events.push(newer_clear);
    add_case(
        &mut cases,
        "TIME-059-delayed-older-frame-does-not-overwrite-newer-clear".to_owned(),
        base_policy,
        events,
        Disposition::WithinSyntheticEnvelope,
    );

    let window_policy = window_policy();
    let window_baseline = window_events();
    add_case(
        &mut cases,
        "TIME-060-complete-clear-evidence-window".to_owned(),
        window_policy.clone(),
        window_baseline.clone(),
        Disposition::WithinSyntheticEnvelope,
    );

    let mut events = window_baseline.clone();
    events.retain(|item| !(item.source_id == "lidar-front" && item.source_sequence == 1));
    for item in &mut events {
        if item.source_id == "lidar-front" && item.source_sequence == 2 {
            item.source_time_ms = 4_901;
            item.received_at_ms = 4_906;
        }
    }
    add_case(
        &mut cases,
        "TIME-061-leading-unobserved-window-gap".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline.clone();
    for item in &mut events {
        if item.source_id == "lidar-front" && item.source_sequence == 2 {
            item.source_time_ms = 4_901;
            item.received_at_ms = 4_906;
        }
    }
    add_case(
        &mut cases,
        "TIME-062-inter-sample-gap-exceeds-limit".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline.clone();
    if let Some(sample) = events.iter_mut().find(|item| {
        item.source_id == "lidar-front"
            && item.source_sequence == 2
            && item.cell == (GridCell { column: 0, row: 0 })
    }) {
        sample.state = ObservationState::Unknown;
    }
    add_case(
        &mut cases,
        "TIME-063-unknown-state-inside-clear-window".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline.clone();
    if let Some(sample) = events.iter_mut().find(|item| {
        item.source_id == "lidar-front"
            && item.source_sequence == 2
            && item.cell == (GridCell { column: 0, row: 0 })
    }) {
        sample.state = ObservationState::HumanPresent;
    }
    add_case(
        &mut cases,
        "TIME-064-human-present-cannot-be-erased-by-later-clear".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline.clone();
    events.retain(|item| {
        !(item.source_id == "camera-front"
            && item.source_sequence == 2
            && item.cell == (GridCell { column: 1, row: 0 }))
    });
    add_case(
        &mut cases,
        "TIME-065-missing-middle-frame-for-one-hazard-cell".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    add_case(
        &mut cases,
        "TIME-066-exact-maximum-inter-sample-gap".to_owned(),
        window_policy.clone(),
        window_baseline.clone(),
        Disposition::WithinSyntheticEnvelope,
    );

    let mut events = window_baseline.clone();
    if let Some(sample) = events.iter_mut().find(|item| {
        item.source_id == "camera-front"
            && item.source_sequence == 2
            && item.cell == (GridCell { column: 0, row: 0 })
    }) {
        sample.confidence_ppm = 100_000;
    }
    add_case(
        &mut cases,
        "TIME-067-low-confidence-sample-breaks-clear-window".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline.clone();
    for item in &mut events {
        if item.source_id == "camera-front" && item.source_sequence == 2 {
            item.source_time_ms = 4_700;
            item.received_at_ms = 4_705;
        }
    }
    add_case(
        &mut cases,
        "TIME-068-source-time-regression-inside-evidence-window".to_owned(),
        window_policy.clone(),
        events,
        Disposition::EvidenceInconsistent,
    );

    let mut events = window_baseline;
    for item in &mut events {
        if item.source_id == "lidar-front" {
            item.source_time_ms = match item.source_sequence {
                1 => 4_600,
                2 => 4_700,
                _ => 4_800,
            };
            item.received_at_ms = item.source_time_ms + 5;
        }
    }
    add_case(
        &mut cases,
        "TIME-069-last-observation-too-old-for-cutoff".to_owned(),
        window_policy,
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline.clone();
    for item in &mut events {
        if item.source_id == "lidar-front" {
            item.source_time_ms = match item.source_sequence {
                1 => 4_700,
                2 => 4_800,
                _ => 4_890,
            };
            item.received_at_ms = item.source_time_ms + 5;
        }
    }
    add_case(
        &mut cases,
        "TIME-070-trailing-observation-gap-exceeds-limit".to_owned(),
        window_policy.clone(),
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut events = window_baseline;
    for item in &mut events {
        if item.source_id == "lidar-front" {
            item.source_time_ms = match item.source_sequence {
                1 => 4_600,
                2 => 4_700,
                _ => 4_800,
            };
            item.received_at_ms = item.source_time_ms + 5;
            if item.source_sequence == 3 && item.cell == (GridCell { column: 0, row: 0 }) {
                item.state = ObservationState::HumanPresent;
            }
        }
    }
    add_case(
        &mut cases,
        "TIME-071-stale-latest-frame-cannot-erase-human-presence-in-window".to_owned(),
        window_policy,
        events,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_trace_corpus_has_at_least_fifty_unique_cases() {
        let cases = frozen_trace_cases();
        let ids = cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(cases.len() >= 50);
        assert_eq!(ids.len(), cases.len());
    }

    #[test]
    fn every_trace_matches_its_pinned_expected_disposition() {
        for case in frozen_trace_cases() {
            let actual = evaluate(&case.policy, &case.events);
            assert_eq!(
                actual.disposition, case.expected,
                "case {}: reason={:?}",
                case.id, actual.reason
            );
        }
    }

    #[test]
    fn no_negative_trace_is_promoted_to_within_envelope() {
        for case in frozen_trace_cases() {
            let actual = evaluate(&case.policy, &case.events);
            if case.expected != Disposition::WithinSyntheticEnvelope {
                assert_ne!(
                    actual.disposition,
                    Disposition::WithinSyntheticEnvelope,
                    "false clear in {}",
                    case.id
                );
            }
        }
    }

    #[test]
    fn identical_event_retransmission_is_idempotent() {
        let policy = base_policy();
        let mut events = baseline_events();
        events.push(events[0].clone());
        let one = evaluate(&policy, &baseline_events());
        let two = evaluate(&policy, &events);
        assert_eq!(one, two);
    }

    #[test]
    fn arrival_permutation_does_not_change_normalized_decision() {
        let policy = base_policy();
        let mut events = baseline_events();
        let expected = evaluate(&policy, &events);
        events.reverse();
        let reversed = evaluate(&policy, &events);
        assert_eq!(expected, reversed);
    }

    #[test]
    fn identical_inputs_are_deterministic() {
        let policy = base_policy();
        let events = baseline_events();
        assert_eq!(evaluate(&policy, &events), evaluate(&policy, &events));
    }

    #[test]
    fn delayed_old_source_sequence_cannot_overwrite_newer_state() {
        let policy = base_policy();
        let mut events = baseline_events();
        for item in &mut events {
            if item.source_id == "lidar-front" {
                item.source_time_ms = 4_800;
                item.received_at_ms = 4_805;
            }
        }
        // The source sequence 1 clear record arrives after sequence 2.
        events[0].received_at_ms = 4_995;
        let mut newer = events[0].clone();
        newer.event_id = "newer-human-present".to_owned();
        newer.source_sequence = 2;
        newer.source_time_ms = 4_980;
        newer.received_at_ms = 4_990;
        newer.state = ObservationState::HumanPresent;
        events.push(newer);
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::HumanPresent));
    }

    #[test]
    fn future_received_event_cannot_rewrite_prior_cutoff() {
        let mut policy = base_policy();
        let mut events = baseline_events();
        events[0].state = ObservationState::HumanPresent;
        let mut late_clear = events[0].clone();
        late_clear.event_id = "not-yet-arrived-clear".to_owned();
        late_clear.source_sequence = 2;
        late_clear.source_time_ms = 4_990;
        late_clear.received_at_ms = policy.evaluated_at_ms + 1;
        late_clear.state = ObservationState::Clear;
        events.push(late_clear);
        let prior = evaluate(&policy, &events);
        assert_eq!(
            prior.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        policy.evaluated_at_ms += 200;
        let later = evaluate(&policy, &events);
        assert_eq!(later.disposition, Disposition::WithinSyntheticEnvelope);
    }

    #[test]
    fn conflicting_duplicate_identity_is_arrival_order_invariant() {
        let policy = base_policy();
        let mut events = baseline_events();
        let mut conflict = events[0].clone();
        conflict.state = ObservationState::HumanPresent;
        conflict.source_sha256 = digest('e');
        events.push(conflict);

        let first = evaluate(&policy, &events);
        events.reverse();
        let second = evaluate(&policy, &events);
        assert_eq!(first, second);
        assert_eq!(first.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(first.reason, Some(Reason::EventIdentityConflict));
    }

    #[test]
    fn source_sequence_is_not_rewritten_by_duplicate_arrival_time() {
        let policy = base_policy();
        let mut events = baseline_events();
        let mut replay = events[0].clone();
        replay.received_at_ms = 4_903;
        events.push(replay);

        let decision = evaluate(&policy, &events);
        assert_eq!(decision.disposition, Disposition::WithinSyntheticEnvelope);
        assert_eq!(
            decision
                .event_references
                .iter()
                .find(|reference| reference.event_id == events[0].event_id)
                .map(|reference| reference.received_at_ms),
            Some(4_903)
        );
    }

    #[test]
    fn source_epoch_and_calibration_are_exactly_bound() {
        let policy = base_policy();
        let mut events = baseline_events();
        events[0].source_epoch = 2;
        let epoch = evaluate(&policy, &events);
        assert_eq!(epoch.reason, Some(Reason::SourceEpochMismatch));

        let mut events = baseline_events();
        events[0].calibration_epoch = "changed".to_owned();
        let calibration = evaluate(&policy, &events);
        assert_eq!(calibration.reason, Some(Reason::CalibrationEpochMismatch));
    }

    #[test]
    fn complete_clear_window_reports_per_cell_temporal_depth() {
        let policy = window_policy();
        let decision = evaluate(&policy, &window_events());
        assert_eq!(decision.disposition, Disposition::WithinSyntheticEnvelope);
        assert_eq!(decision.cells.len(), policy.required_cells.len());
        for cell in decision.cells {
            assert_eq!(cell.evidence_window_start_ms, 4_800);
            assert_eq!(cell.maximum_observation_gap_ms, 100);
            assert_eq!(cell.temporal_sample_count, 6);
        }
    }

    #[test]
    fn leading_gap_cannot_be_hidden_by_a_fresh_latest_snapshot() {
        let policy = window_policy();
        let mut events = window_events();
        events.retain(|item| !(item.source_id == "lidar-front" && item.source_sequence == 1));
        for item in &mut events {
            if item.source_id == "lidar-front" && item.source_sequence == 2 {
                item.source_time_ms = 4_901;
                item.received_at_ms = 4_906;
            }
        }
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::CellEvidenceStale));
    }

    #[test]
    fn human_present_inside_the_window_cannot_be_erased_by_later_clear() {
        let policy = window_policy();
        let mut events = window_events();
        if let Some(sample) = events.iter_mut().find(|item| {
            item.source_id == "lidar-front"
                && item.source_sequence == 2
                && item.cell == (GridCell { column: 0, row: 0 })
        }) {
            sample.state = ObservationState::HumanPresent;
        }
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::HumanPresent));
    }

    #[test]
    fn unknown_sample_inside_the_window_breaks_clear_evidence() {
        let policy = window_policy();
        let mut events = window_events();
        if let Some(sample) = events.iter_mut().find(|item| {
            item.source_id == "lidar-front"
                && item.source_sequence == 2
                && item.cell == (GridCell { column: 0, row: 0 })
        }) {
            sample.state = ObservationState::Unknown;
        }
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::CellEvidenceUnknown));
    }

    #[test]
    fn exactly_maximum_inter_sample_gap_is_admissible() {
        let policy = window_policy();
        let decision = evaluate(&policy, &window_events());
        assert_eq!(decision.disposition, Disposition::WithinSyntheticEnvelope);
    }

    #[test]
    fn trailing_gap_is_bounded_by_inter_sample_limit() {
        let policy = window_policy();
        let mut events = window_events();
        for item in &mut events {
            if item.source_id == "lidar-front" {
                item.source_time_ms = match item.source_sequence {
                    1 => 4_700,
                    2 => 4_800,
                    _ => 4_890,
                };
                item.received_at_ms = item.source_time_ms + 5;
            }
        }
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::CellEvidenceStale));
    }

    #[test]
    fn human_present_inside_window_survives_stale_latest_frame() {
        let policy = window_policy();
        let mut events = window_events();
        for item in &mut events {
            if item.source_id == "lidar-front" {
                item.source_time_ms = match item.source_sequence {
                    1 => 4_600,
                    2 => 4_700,
                    _ => 4_800,
                };
                item.received_at_ms = item.source_time_ms + 5;
                if item.source_sequence == 3 && item.cell == (GridCell { column: 0, row: 0 }) {
                    item.state = ObservationState::HumanPresent;
                }
            }
        }
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::HumanPresent));
    }

    #[test]
    fn source_time_and_receive_time_are_not_interchangeable() {
        let policy = base_policy();
        let mut events = baseline_events();
        for item in &mut events {
            if item.source_id == "lidar-front" {
                item.source_time_ms = 4_000;
                item.received_at_ms = 4_999;
            }
        }
        let decision = evaluate(&policy, &events);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reason, Some(Reason::CellEvidenceStale));
    }
}
