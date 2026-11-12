use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const MAX_INPUT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SEQUENCES: usize = 256;
const MAX_STEPS_PER_SEQUENCE: usize = 256;
const MAX_OBSERVATIONS_PER_SEQUENCE: usize = 20_000;
const MAX_IDENTITY_PARTITION: usize = 16;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum EventKind {
    Unchanged,
    ViewpointOnly,
    Occlusion,
    RegistrationDrift,
    PhysicalChange,
    Replacement,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DecisionDisposition {
    NoChangeSupported,
    PartialWithUnknown,
    RequestAdditionalObservation,
    AdmitCandidateDelta,
    ReplacementCandidate,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DecisionPurpose {
    MediumBuildingUpdate,
    SlowQualityRequalification,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceKind {
    Calibration,
    Registration,
    Survey,
    Ndt,
    Repair,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateCorpus {
    schema_version: String,
    sequences: Vec<CandidateSequence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateSequence {
    sequence_id: String,
    steps: Vec<CandidateStep>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateStep {
    step_id: String,
    measurement_tick: u64,
    measurement_clock_id: String,
    decision_cutoff_tick: u64,
    decision_clock_id: String,
    decision_clock_precision_ticks: u64,
    decision_purpose: DecisionPurpose,
    reported_event: EventKind,
    disposition: DecisionDisposition,
    decision_evidence_event_ids: Vec<String>,
    observations: Vec<CandidateObservation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateObservation {
    observation_id: String,
    candidate_track_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceCorpus {
    schema_version: String,
    sequences: Vec<ReferenceSequence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceSequence {
    sequence_id: String,
    building_group_id: String,
    steps: Vec<ReferenceStep>,
    observations: Vec<ReferenceObservation>,
    evidence_events: Vec<ReferenceEvidenceEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceStep {
    step_id: String,
    measurement_tick: u64,
    measurement_clock_id: String,
    ground_truth_event: EventKind,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceObservation {
    step_id: String,
    observation_id: String,
    ground_truth_entity_id: String,
    visible: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceEvidenceEvent {
    event_id: String,
    kind: EvidenceKind,
    measurement_tick: u64,
    first_received_tick: u64,
    clock_id: String,
    timestamp_precision_ticks: u64,
    valid_from_tick: u64,
    valid_until_tick: Option<u64>,
    revoked_known_at_tick: Option<u64>,
    allowed_purposes: Vec<DecisionPurpose>,
}

#[derive(Debug, Serialize)]
struct SequenceResult {
    sequence_id: String,
    building_group_id: String,
    step_count: usize,
    ground_truth_visible_detections: usize,
    candidate_detections: usize,
    id_true_positives: usize,
    id_false_positives: usize,
    id_false_negatives: usize,
    idf1: f64,
    identity_switches: usize,
    fragmentations: usize,
    event_matches: usize,
    event_mismatches: usize,
    false_physical_change_events: usize,
    ground_truth_physical_change_events: usize,
    detected_physical_change_events: usize,
    temporal_evidence_violations: usize,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct SequenceReport {
    schema_version: String,
    candidate_sha256: String,
    evaluator_reference_sha256: String,
    sequence_count: usize,
    building_group_count: usize,
    ground_truth_visible_detections: usize,
    candidate_detections: usize,
    id_true_positives: usize,
    id_false_positives: usize,
    id_false_negatives: usize,
    idf1: f64,
    identity_switches: usize,
    fragmentations: usize,
    event_matches: usize,
    event_mismatches: usize,
    false_physical_change_events: usize,
    ground_truth_physical_change_events: usize,
    detected_physical_change_events: usize,
    temporal_evidence_violations: usize,
    passed: bool,
    claim_ceiling: String,
    sequences: Vec<SequenceResult>,
}

fn expected_disposition(event: EventKind) -> DecisionDisposition {
    match event {
        EventKind::Unchanged | EventKind::ViewpointOnly => {
            DecisionDisposition::NoChangeSupported
        }
        EventKind::Occlusion => DecisionDisposition::PartialWithUnknown,
        EventKind::RegistrationDrift | EventKind::Unresolved => {
            DecisionDisposition::RequestAdditionalObservation
        }
        EventKind::PhysicalChange => DecisionDisposition::AdmitCandidateDelta,
        EventKind::Replacement => DecisionDisposition::ReplacementCandidate,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn max_identity_assignment(matrix: &[Vec<usize>]) -> Result<usize, String> {
    if matrix.is_empty() || matrix[0].is_empty() {
        return Ok(0);
    }
    let rows = matrix.len();
    let columns = matrix[0].len();
    if rows > MAX_IDENTITY_PARTITION || columns > MAX_IDENTITY_PARTITION {
        return Err(format!(
            "identity partition exceeds bounded exact oracle limit ({MAX_IDENTITY_PARTITION})"
        ));
    }
    if matrix.iter().any(|row| row.len() != columns) {
        return Err("identity overlap matrix is ragged".to_owned());
    }

    let states = 1usize << columns;
    let mut dp = vec![None; states];
    dp[0] = Some(0usize);
    for row in matrix {
        let mut next = dp.clone();
        for mask in 0..states {
            if let Some(score) = dp[mask] {
                for column in 0..columns {
                    let bit = 1usize << column;
                    if mask & bit == 0 {
                        let candidate = score + row[column];
                        let slot = &mut next[mask | bit];
                        let previous = (*slot).unwrap_or(0);
                        *slot = Some(previous.max(candidate));
                    }
                }
            }
        }
        dp = next;
    }
    Ok(dp.into_iter().flatten().max().unwrap_or(0))
}

fn evaluate_sequence(
    candidate: &CandidateSequence,
    reference: &ReferenceSequence,
) -> Result<SequenceResult, String> {
    if candidate.sequence_id != reference.sequence_id {
        return Err("candidate/reference sequence identity mismatch".to_owned());
    }
    if candidate.sequence_id.trim().is_empty() || reference.building_group_id.trim().is_empty() {
        return Err("sequence and building-group IDs must be non-empty".to_owned());
    }
    if candidate.steps.is_empty() || candidate.steps.len() > MAX_STEPS_PER_SEQUENCE {
        return Err(format!(
            "{} must contain 1..={MAX_STEPS_PER_SEQUENCE} steps",
            candidate.sequence_id
        ));
    }
    if reference.observations.len() > MAX_OBSERVATIONS_PER_SEQUENCE {
        return Err(format!(
            "{} exceeds observation bound",
            candidate.sequence_id
        ));
    }

    let mut reference_steps = BTreeMap::new();
    for step in &reference.steps {
        if step.step_id.trim().is_empty()
            || reference_steps.insert(step.step_id.as_str(), step).is_some()
        {
            return Err(format!(
                "{} has an empty or duplicate reference step ID",
                candidate.sequence_id
            ));
        }
    }
    let mut evidence_events = BTreeMap::new();
    for event in &reference.evidence_events {
        if event.event_id.trim().is_empty()
            || event
                .valid_until_tick
                .is_some_and(|until| until <= event.valid_from_tick)
            || evidence_events.insert(event.event_id.as_str(), event).is_some()
        {
            return Err(format!(
                "{} has an invalid or duplicate evidence-event identity/validity interval",
                candidate.sequence_id
            ));
        }
    }

    let mut reference_observations: BTreeMap<(&str, &str), &ReferenceObservation> =
        BTreeMap::new();
    let mut entity_ids = BTreeSet::new();
    let mut total_ground_truth_visible = 0usize;
    for observation in &reference.observations {
        if observation.step_id.trim().is_empty()
            || observation.observation_id.trim().is_empty()
            || observation.ground_truth_entity_id.trim().is_empty()
            || reference_observations
                .insert(
                    (observation.step_id.as_str(), observation.observation_id.as_str()),
                    observation,
                )
                .is_some()
        {
            return Err(format!(
                "{} has an empty or duplicate reference observation identity",
                candidate.sequence_id
            ));
        }
        if !reference_steps.contains_key(observation.step_id.as_str()) {
            return Err(format!(
                "{} observation references an unknown reference step",
                candidate.sequence_id
            ));
        }
        if observation.visible {
            total_ground_truth_visible += 1;
            entity_ids.insert(observation.ground_truth_entity_id.as_str());
        }
    }

    let mut candidate_steps = candidate.steps.iter().collect::<Vec<_>>();
    candidate_steps.sort_by(|left, right| {
        left.measurement_tick
            .cmp(&right.measurement_tick)
            .then_with(|| left.step_id.cmp(&right.step_id))
    });
    let mut candidate_step_ids = BTreeSet::new();
    let mut candidate_assignment: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    let mut candidate_track_ids = BTreeSet::new();
    let mut event_matches = 0usize;
    let mut event_mismatches = 0usize;
    let mut false_physical_change_events = 0usize;
    let mut ground_truth_physical_change_events = 0usize;
    let mut detected_physical_change_events = 0usize;
    let mut temporal_evidence_violations = 0usize;

    for step in &candidate_steps {
        if step.step_id.trim().is_empty() || !candidate_step_ids.insert(step.step_id.as_str()) {
            return Err(format!(
                "{} has an empty or duplicate candidate step ID",
                candidate.sequence_id
            ));
        }
        let reference_step = reference_steps
            .get(step.step_id.as_str())
            .ok_or_else(|| format!("{} has an unknown candidate step", candidate.sequence_id))?;
        if step.measurement_tick != reference_step.measurement_tick
            || step.measurement_clock_id != reference_step.measurement_clock_id
        {
            return Err(format!(
                "{} measurement time/clock differs from the evaluator sequence",
                step.step_id
            ));
        }
        if step.measurement_clock_id.trim().is_empty()
            || step.decision_clock_id.trim().is_empty()
            || step.decision_clock_precision_ticks == 0
            || step.measurement_clock_id != step.decision_clock_id
            || step.decision_cutoff_tick < step.measurement_tick
        {
            temporal_evidence_violations += 1;
        }

        let expected = expected_disposition(reference_step.ground_truth_event);
        let matched = step.reported_event == reference_step.ground_truth_event
            && step.disposition == expected;
        if matched {
            event_matches += 1;
        } else {
            event_mismatches += 1;
        }

        if reference_step.ground_truth_event == EventKind::PhysicalChange {
            ground_truth_physical_change_events += 1;
            if step.reported_event == EventKind::PhysicalChange
                && step.disposition == DecisionDisposition::AdmitCandidateDelta
            {
                detected_physical_change_events += 1;
            }
        } else if step.reported_event == EventKind::PhysicalChange
            || step.disposition == DecisionDisposition::AdmitCandidateDelta
        {
            false_physical_change_events += 1;
        }

        let mut used_evidence_ids = BTreeSet::new();
        for evidence_id in &step.decision_evidence_event_ids {
            if evidence_id.trim().is_empty() || !used_evidence_ids.insert(evidence_id.as_str()) {
                temporal_evidence_violations += 1;
                continue;
            }
            let Some(event) = evidence_events.get(evidence_id.as_str()) else {
                temporal_evidence_violations += 1;
                continue;
            };
            let cutoff = step.decision_cutoff_tick;
            let event_is_known = event.clock_id == step.decision_clock_id
                && event.timestamp_precision_ticks > 0
                && event.timestamp_precision_ticks <= step.decision_clock_precision_ticks
                && event.first_received_tick >= event.measurement_tick
                && event.measurement_tick <= cutoff
                && event.first_received_tick <= cutoff
                && event.valid_from_tick <= cutoff
                && event
                    .valid_until_tick
                    .is_none_or(|valid_until| cutoff < valid_until)
                && event
                    .revoked_known_at_tick
                    .is_none_or(|revoked_at| cutoff < revoked_at)
                && event.allowed_purposes.contains(&step.decision_purpose)
                && !(matches!(event.kind, EvidenceKind::Ndt | EvidenceKind::Repair)
                    && step.decision_purpose != DecisionPurpose::SlowQualityRequalification);
            if !event_is_known {
                temporal_evidence_violations += 1;
            }
        }

        let mut step_observation_ids = BTreeSet::new();
        for observation in &step.observations {
            if observation.observation_id.trim().is_empty()
                || observation.candidate_track_id.trim().is_empty()
                || !step_observation_ids.insert(observation.observation_id.as_str())
            {
                return Err(format!(
                    "{} has an empty or duplicate candidate observation/track ID",
                    step.step_id
                ));
            }
            candidate_track_ids.insert(observation.candidate_track_id.as_str());
            if candidate_assignment
                .insert(
                    (step.step_id.as_str(), observation.observation_id.as_str()),
                    observation.candidate_track_id.as_str(),
                )
                .is_some()
            {
                return Err(format!(
                    "{} repeats a candidate observation identity",
                    candidate.sequence_id
                ));
            }
        }
    }

    if candidate_step_ids.len() != reference_steps.len()
        || reference_steps
            .keys()
            .any(|step_id| !candidate_step_ids.contains(step_id))
    {
        return Err(format!(
            "{} candidate output omits one or more reference acquisition steps",
            candidate.sequence_id
        ));
    }

    let entity_ids = entity_ids.into_iter().collect::<Vec<_>>();
    let candidate_track_ids = candidate_track_ids.into_iter().collect::<Vec<_>>();
    let mut overlap = vec![vec![0usize; candidate_track_ids.len()]; entity_ids.len()];
    let entity_index = entity_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect::<BTreeMap<_, _>>();
    let track_index = candidate_track_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect::<BTreeMap<_, _>>();

    let mut candidate_detection_count = 0usize;
    for ((step_id, observation_id), track_id) in &candidate_assignment {
        candidate_detection_count += 1;
        if let Some(reference_observation) =
            reference_observations.get(&(*step_id, *observation_id))
        {
            if reference_observation.visible {
                let row = *entity_index
                    .get(reference_observation.ground_truth_entity_id.as_str())
                    .ok_or_else(|| "visible entity index missing".to_owned())?;
                let column = *track_index
                    .get(track_id)
                    .ok_or_else(|| "candidate track index missing".to_owned())?;
                overlap[row][column] += 1;
            }
        }
    }
    let id_true_positives = max_identity_assignment(&overlap)?;
    let id_false_positives = candidate_detection_count.saturating_sub(id_true_positives);
    let id_false_negatives = total_ground_truth_visible.saturating_sub(id_true_positives);
    let idf1_denominator =
        2 * id_true_positives + id_false_positives + id_false_negatives;
    let idf1 = if idf1_denominator == 0 {
        1.0
    } else {
        (2 * id_true_positives) as f64 / idf1_denominator as f64
    };

    let mut observations_by_entity: BTreeMap<&str, Vec<&ReferenceObservation>> = BTreeMap::new();
    for observation in reference.observations.iter().filter(|item| item.visible) {
        observations_by_entity
            .entry(observation.ground_truth_entity_id.as_str())
            .or_default()
            .push(observation);
    }
    let step_ticks = reference_steps
        .iter()
        .map(|(id, step)| (*id, step.measurement_tick))
        .collect::<BTreeMap<_, _>>();
    let mut identity_switches = 0usize;
    let mut fragmentations = 0usize;
    for entity_observations in observations_by_entity.values_mut() {
        entity_observations.sort_by(|left, right| {
            step_ticks
                .get(left.step_id.as_str())
                .cmp(&step_ticks.get(right.step_id.as_str()))
                .then_with(|| left.step_id.cmp(&right.step_id))
        });
        let mut previous_track: Option<&str> = None;
        let mut had_detection = false;
        let mut missed_after_detection = false;
        for observation in entity_observations.iter() {
            let current_track = candidate_assignment
                .get(&(observation.step_id.as_str(), observation.observation_id.as_str()))
                .copied();
            if let Some(track_id) = current_track {
                if let Some(previous) = previous_track {
                    if previous != track_id {
                        identity_switches += 1;
                    }
                }
                if had_detection && missed_after_detection {
                    fragmentations += 1;
                }
                previous_track = Some(track_id);
                had_detection = true;
                missed_after_detection = false;
            } else if had_detection {
                missed_after_detection = true;
            }
        }
    }

    let passed = event_mismatches == 0
        && false_physical_change_events == 0
        && detected_physical_change_events == ground_truth_physical_change_events
        && temporal_evidence_violations == 0
        && id_false_positives == 0
        && id_false_negatives == 0
        && identity_switches == 0
        && fragmentations == 0;

    Ok(SequenceResult {
        sequence_id: candidate.sequence_id.clone(),
        building_group_id: reference.building_group_id.clone(),
        step_count: candidate.steps.len(),
        ground_truth_visible_detections: total_ground_truth_visible,
        candidate_detections: candidate_detection_count,
        id_true_positives,
        id_false_positives,
        id_false_negatives,
        idf1,
        identity_switches,
        fragmentations,
        event_matches,
        event_mismatches,
        false_physical_change_events,
        ground_truth_physical_change_events,
        detected_physical_change_events,
        temporal_evidence_violations,
        passed,
    })
}

fn evaluate_bytes(
    candidate_bytes: &[u8],
    reference_bytes: &[u8],
) -> Result<SequenceReport, String> {
    if candidate_bytes.len() as u64 > MAX_INPUT_BYTES
        || reference_bytes.len() as u64 > MAX_INPUT_BYTES
    {
        return Err(format!("sequence inputs exceed {MAX_INPUT_BYTES} bytes"));
    }
    let candidate: CandidateCorpus = serde_json::from_slice(candidate_bytes)
        .map_err(|error| format!("candidate corpus parse failed: {error}"))?;
    let reference: ReferenceCorpus = serde_json::from_slice(reference_bytes)
        .map_err(|error| format!("evaluator reference parse failed: {error}"))?;
    if candidate.schema_version != "0.1" || reference.schema_version != "0.1" {
        return Err("unsupported sequence corpus schema version".to_owned());
    }
    if candidate.sequences.is_empty()
        || candidate.sequences.len() > MAX_SEQUENCES
        || reference.sequences.len() > MAX_SEQUENCES
    {
        return Err(format!("sequence corpus must contain 1..={MAX_SEQUENCES} sequences"));
    }

    let mut reference_sequences = BTreeMap::new();
    for sequence in &reference.sequences {
        if sequence.sequence_id.trim().is_empty()
            || reference_sequences
                .insert(sequence.sequence_id.as_str(), sequence)
                .is_some()
        {
            return Err("reference corpus has an empty or duplicate sequence ID".to_owned());
        }
    }
    let mut candidate_ids = BTreeSet::new();
    let mut candidate_sequences = candidate.sequences.iter().collect::<Vec<_>>();
    candidate_sequences.sort_by(|left, right| left.sequence_id.cmp(&right.sequence_id));
    if candidate_sequences
        .iter()
        .any(|sequence| !candidate_ids.insert(sequence.sequence_id.as_str()))
    {
        return Err("candidate corpus has a duplicate sequence ID".to_owned());
    }
    if candidate_ids.len() != reference_sequences.len()
        || reference_sequences
            .keys()
            .any(|sequence_id| !candidate_ids.contains(sequence_id))
    {
        return Err("candidate and evaluator reference sequence sets differ".to_owned());
    }

    let mut sequences = Vec::with_capacity(candidate_sequences.len());
    for candidate_sequence in candidate_sequences {
        let reference_sequence = reference_sequences
            .get(candidate_sequence.sequence_id.as_str())
            .ok_or_else(|| "reference sequence lookup failed".to_owned())?;
        sequences.push(evaluate_sequence(candidate_sequence, reference_sequence)?);
    }

    let sequence_count = sequences.len();
    let building_groups = sequences
        .iter()
        .map(|sequence| sequence.building_group_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let sum = |select: fn(&SequenceResult) -> usize| {
        sequences.iter().map(select).sum::<usize>()
    };
    let ground_truth_visible_detections = sum(|sequence| sequence.ground_truth_visible_detections);
    let candidate_detections = sum(|sequence| sequence.candidate_detections);
    let id_true_positives = sum(|sequence| sequence.id_true_positives);
    let id_false_positives = sum(|sequence| sequence.id_false_positives);
    let id_false_negatives = sum(|sequence| sequence.id_false_negatives);
    let idf1_denominator =
        2 * id_true_positives + id_false_positives + id_false_negatives;
    let idf1 = if idf1_denominator == 0 {
        1.0
    } else {
        (2 * id_true_positives) as f64 / idf1_denominator as f64
    };
    let event_matches = sum(|sequence| sequence.event_matches);
    let event_mismatches = sum(|sequence| sequence.event_mismatches);
    let false_physical_change_events = sum(|sequence| sequence.false_physical_change_events);
    let ground_truth_physical_change_events =
        sum(|sequence| sequence.ground_truth_physical_change_events);
    let detected_physical_change_events =
        sum(|sequence| sequence.detected_physical_change_events);
    let temporal_evidence_violations =
        sum(|sequence| sequence.temporal_evidence_violations);
    let identity_switches = sum(|sequence| sequence.identity_switches);
    let fragmentations = sum(|sequence| sequence.fragmentations);
    let passed = sequences.iter().all(|sequence| sequence.passed);

    Ok(SequenceReport {
        schema_version: "0.1".to_owned(),
        candidate_sha256: sha256_hex(candidate_bytes),
        evaluator_reference_sha256: sha256_hex(reference_bytes),
        sequence_count,
        building_group_count: building_groups,
        ground_truth_visible_detections,
        candidate_detections,
        id_true_positives,
        id_false_positives,
        id_false_negatives,
        idf1,
        identity_switches,
        fragmentations,
        event_matches,
        event_mismatches,
        false_physical_change_events,
        ground_truth_physical_change_events,
        detected_physical_change_events,
        temporal_evidence_violations,
        passed,
        claim_ceiling: "Synthetic sequence evaluation only; no physical identity, survey accuracy or real-world SOTA claim.".to_owned(),
        sequences,
    })
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut args = env::args().skip(1);
    let mut candidate = PathBuf::from(
        "tools/scan-bim-temporal-sequence-benchmark/fixtures/candidate-sequences.json",
    );
    let mut reference = PathBuf::from(
        "tools/scan-bim-temporal-sequence-benchmark/fixtures/evaluator-reference.json",
    );
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--candidate" => {
                candidate = args
                    .next()
                    .ok_or_else(|| "--candidate requires a path".to_owned())?
                    .into();
            }
            "--reference" => {
                reference = args
                    .next()
                    .ok_or_else(|| "--reference requires a path".to_owned())?
                    .into();
            }
            "--help" | "-h" => {
                println!(
                    "scan-bim-temporal-sequence-benchmark [--candidate PATH] [--reference PATH]"
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    Ok((candidate, reference))
}

fn run() -> Result<bool, String> {
    let (candidate_path, reference_path) = parse_args()?;
    let candidate_bytes = fs::read(candidate_path).map_err(|error| error.to_string())?;
    let reference_bytes = fs::read(reference_path).map_err(|error| error.to_string())?;
    let report = evaluate_bytes(&candidate_bytes, &reference_bytes)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(report.passed)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const CANDIDATE: &str =
        include_str!("../fixtures/candidate-sequences.json");
    const REFERENCE: &str =
        include_str!("../fixtures/evaluator-reference.json");

    fn parsed_fixture() -> (Value, Value) {
        (
            serde_json::from_str(CANDIDATE).expect("candidate fixture parses"),
            serde_json::from_str(REFERENCE).expect("reference fixture parses"),
        )
    }

    #[test]
    fn frozen_sequence_corpus_has_expected_metrics() {
        let report = evaluate_bytes(CANDIDATE.as_bytes(), REFERENCE.as_bytes())
            .expect("frozen sequence corpus evaluates");
        assert_eq!(report.sequence_count, 7);
        assert_eq!(report.building_group_count, 7);
        assert_eq!(report.id_true_positives, report.ground_truth_visible_detections);
        assert_eq!(report.id_false_positives, 0);
        assert_eq!(report.id_false_negatives, 0);
        assert_eq!(report.idf1, 1.0);
        assert_eq!(report.identity_switches, 0);
        assert_eq!(report.fragmentations, 0);
        assert_eq!(report.event_mismatches, 0);
        assert_eq!(report.false_physical_change_events, 0);
        assert_eq!(report.temporal_evidence_violations, 0);
        assert_eq!(report.detected_physical_change_events, 1);
        assert!(report.passed);
    }

    #[test]
    fn candidate_and_reference_input_order_do_not_change_semantic_metrics() {
        let baseline = evaluate_bytes(CANDIDATE.as_bytes(), REFERENCE.as_bytes())
            .expect("baseline sequence corpus evaluates");
        let (mut candidate, mut reference) = parsed_fixture();
        candidate["sequences"].as_array_mut().unwrap().reverse();
        for sequence in candidate["sequences"].as_array_mut().unwrap() {
            sequence["steps"].as_array_mut().unwrap().reverse();
            for step in sequence["steps"].as_array_mut().unwrap() {
                step["observations"].as_array_mut().unwrap().reverse();
            }
        }
        reference["sequences"].as_array_mut().unwrap().reverse();
        for sequence in reference["sequences"].as_array_mut().unwrap() {
            sequence["steps"].as_array_mut().unwrap().reverse();
            sequence["observations"].as_array_mut().unwrap().reverse();
            sequence["evidence_events"].as_array_mut().unwrap().reverse();
        }
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let reference_bytes = serde_json::to_vec(&reference).unwrap();
        let reordered = evaluate_bytes(&candidate_bytes, &reference_bytes)
            .expect("reordered sequence corpus evaluates");
        assert_eq!(
            serde_json::to_value(baseline.sequences).unwrap(),
            serde_json::to_value(reordered.sequences).unwrap()
        );
        assert_eq!(baseline.idf1, reordered.idf1);
        assert_eq!(baseline.identity_switches, reordered.identity_switches);
        assert_eq!(baseline.event_matches, reordered.event_matches);
        assert_ne!(baseline.candidate_sha256, reordered.candidate_sha256);
        assert_ne!(
            baseline.evaluator_reference_sha256,
            reordered.evaluator_reference_sha256
        );
    }

    #[test]
    fn late_arriving_scan_cannot_be_used_for_earlier_decision() {
        let (mut candidate, _) = parsed_fixture();
        let step = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap()["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "f-02")
            .unwrap();
        step["decision_evidence_event_ids"]
            .as_array_mut()
            .unwrap()
            .push(Value::String("ev-late".to_owned()));
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let report = evaluate_bytes(&candidate_bytes, REFERENCE.as_bytes())
            .expect("late evidence is reported, not hidden");
        assert_eq!(report.temporal_evidence_violations, 1);
        assert!(!report.passed);
    }

    #[test]
    fn ndt_outcome_cannot_leak_into_medium_building_update() {
        let (mut candidate, mut reference) = parsed_fixture();
        let sequence = reference["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap();
        sequence["evidence_events"].as_array_mut().unwrap().push(json!({
            "event_id":"ev-ndt",
            "kind":"ndt",
            "measurement_tick":19,
            "first_received_tick":19,
            "valid_from_tick":0,
            "valid_until_tick":100,
            "revoked_known_at_tick":null,
            "allowed_purposes":["slow_quality_requalification"]
        }));
        let step = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap()["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "f-02")
            .unwrap();
        step["decision_evidence_event_ids"]
            .as_array_mut()
            .unwrap()
            .push(Value::String("ev-ndt".to_owned()));
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let reference_bytes = serde_json::to_vec(&reference).unwrap();
        let report = evaluate_bytes(&candidate_bytes, &reference_bytes)
            .expect("purpose leakage is reported");
        assert_eq!(report.temporal_evidence_violations, 1);
        assert!(!report.passed);
    }

    #[test]
    fn unknown_clock_domain_is_not_silently_ordered() {
        let (mut candidate, _) = parsed_fixture();
        let step = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-UNCHANGED")
            .unwrap()["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "a-02")
            .unwrap();
        step["decision_clock_id"] = json!("unmapped-device-clock");
        let bytes = serde_json::to_vec(&candidate).unwrap();
        let report = evaluate_bytes(&bytes, REFERENCE.as_bytes())
            .expect("unknown clock is reported as a qualification violation");
        assert_eq!(report.temporal_evidence_violations, 1);
        assert!(!report.passed);
    }

    #[test]
    fn timestamp_precision_coarser_than_decision_resolution_is_rejected() {
        let (candidate, mut reference) = parsed_fixture();
        let sequence = reference["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-UNCHANGED")
            .unwrap();
        let event = sequence["evidence_events"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|event| event["event_id"] == "ev-scan")
            .unwrap();
        event["timestamp_precision_ticks"] = json!(2);
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let reference_bytes = serde_json::to_vec(&reference).unwrap();
        let report = evaluate_bytes(&candidate_bytes, &reference_bytes)
            .expect("coarse time precision is reported");
        assert_eq!(report.temporal_evidence_violations, 3);
        assert!(!report.passed);
    }

    #[test]
    fn revocation_known_by_cutoff_invalidates_historical_evidence() {
        let (mut candidate, mut reference) = parsed_fixture();
        let sequence = reference["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap();
        let revoked = sequence["evidence_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event_id"] == "ev-scan")
            .unwrap()
            .clone();
        let mut revoked = revoked;
        revoked["event_id"] = json!("ev-revoked-before-cutoff");
        revoked["revoked_known_at_tick"] = json!(20);
        sequence["evidence_events"].as_array_mut().unwrap().push(revoked);
        let step = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap()["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "f-02")
            .unwrap();
        step["decision_evidence_event_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!("ev-revoked-before-cutoff"));
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let reference_bytes = serde_json::to_vec(&reference).unwrap();
        let report = evaluate_bytes(&candidate_bytes, &reference_bytes)
            .expect("known revocation is scored");
        assert_eq!(report.temporal_evidence_violations, 1);
        assert!(!report.passed);
    }

    #[test]
    fn revocation_learned_after_cutoff_does_not_rewrite_past_snapshot() {
        let (mut candidate, mut reference) = parsed_fixture();
        let sequence = reference["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap();
        let mut revoked = sequence["evidence_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event_id"] == "ev-scan")
            .unwrap()
            .clone();
        revoked["event_id"] = json!("ev-revoked-after-cutoff");
        revoked["revoked_known_at_tick"] = json!(25);
        sequence["evidence_events"].as_array_mut().unwrap().push(revoked);
        let step = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap()["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "f-02")
            .unwrap();
        step["decision_evidence_event_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!("ev-revoked-after-cutoff"));
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let reference_bytes = serde_json::to_vec(&reference).unwrap();
        let report = evaluate_bytes(&candidate_bytes, &reference_bytes)
            .expect("later revocation preserves the earlier information set");
        assert_eq!(report.temporal_evidence_violations, 0);
        assert!(report.passed);
    }

    #[test]
    fn identity_switch_is_counted_and_reduces_idf1() {
        let (mut candidate, _) = parsed_fixture();
        let sequence = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-UNCHANGED")
            .unwrap();
        let step = sequence["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "a-02")
            .unwrap();
        step["observations"][0]["candidate_track_id"] =
            Value::String("track-wall-new".to_owned());
        let bytes = serde_json::to_vec(&candidate).unwrap();
        let report =
            evaluate_bytes(&bytes, REFERENCE.as_bytes()).expect("identity switch is scored");
        let unchanged = report
            .sequences
            .iter()
            .find(|sequence| sequence.sequence_id == "SEQ-UNCHANGED")
            .unwrap();
        assert_eq!(unchanged.identity_switches, 1);
        assert!(unchanged.idf1 < 1.0);
        assert!(!report.passed);
    }

    #[test]
    fn nuisance_attributed_as_physical_change_is_a_false_event() {
        let (mut candidate, _) = parsed_fixture();
        let sequence = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-VIEWPOINT")
            .unwrap();
        let step = sequence["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "c-02")
            .unwrap();
        step["reported_event"] = json!("physical_change");
        step["disposition"] = json!("admit_candidate_delta");
        let bytes = serde_json::to_vec(&candidate).unwrap();
        let report =
            evaluate_bytes(&bytes, REFERENCE.as_bytes()).expect("false event is scored");
        assert_eq!(report.false_physical_change_events, 1);
        assert!(!report.passed);
    }

    #[test]
    fn missing_visible_observation_reduces_idf1_and_fails() {
        let (mut candidate, _) = parsed_fixture();
        let sequence = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-UNCHANGED")
            .unwrap();
        let step = sequence["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "a-03")
            .unwrap();
        step["observations"].as_array_mut().unwrap().clear();
        let bytes = serde_json::to_vec(&candidate).unwrap();
        let report =
            evaluate_bytes(&bytes, REFERENCE.as_bytes()).expect("missed observation is scored");
        let unchanged = report
            .sequences
            .iter()
            .find(|sequence| sequence.sequence_id == "SEQ-UNCHANGED")
            .unwrap();
        assert_eq!(unchanged.id_false_negatives, 1);
        assert!(unchanged.idf1 < 1.0);
        assert!(!unchanged.passed);
        assert!(!report.passed);
    }

    #[test]
    fn replacement_uses_new_entity_and_does_not_count_as_id_switch() {
        let report = evaluate_bytes(CANDIDATE.as_bytes(), REFERENCE.as_bytes())
            .expect("replacement fixture evaluates");
        let sequence = report
            .sequences
            .iter()
            .find(|sequence| sequence.sequence_id == "SEQ-REPLACEMENT")
            .unwrap();
        assert_eq!(sequence.identity_switches, 0);
        assert_eq!(sequence.idf1, 1.0);
        assert!(sequence.passed);
    }

    #[test]
    fn cutoff_before_measurement_is_a_temporal_violation() {
        let (mut candidate, _) = parsed_fixture();
        let step = candidate["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-LATE-ARRIVAL")
            .unwrap()["steps"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|step| step["step_id"] == "f-02")
            .unwrap();
        step["decision_cutoff_tick"] = json!(19);
        let bytes = serde_json::to_vec(&candidate).unwrap();
        let report =
            evaluate_bytes(&bytes, REFERENCE.as_bytes()).expect("early cutoff is scored");
        assert_eq!(report.temporal_evidence_violations, 1);
        assert!(!report.passed);
    }

    #[test]
    fn duplicate_reference_evidence_event_ids_are_rejected() {
        let (candidate, mut reference) = parsed_fixture();
        let sequence = reference["sequences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|sequence| sequence["sequence_id"] == "SEQ-UNCHANGED")
            .unwrap();
        let duplicate = sequence["evidence_events"][0].clone();
        sequence["evidence_events"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        let candidate_bytes = serde_json::to_vec(&candidate).unwrap();
        let reference_bytes = serde_json::to_vec(&reference).unwrap();
        assert!(evaluate_bytes(&candidate_bytes, &reference_bytes).is_err());
    }

    #[test]
    fn reference_identity_reuse_across_replacement_is_explicitly_distinct() {
        let reference: ReferenceCorpus =
            serde_json::from_str(REFERENCE).expect("reference fixture parses");
        let replacement = reference
            .sequences
            .iter()
            .find(|sequence| sequence.sequence_id == "SEQ-REPLACEMENT")
            .unwrap();
        let old = replacement
            .observations
            .iter()
            .find(|observation| observation.step_id == "g-01")
            .unwrap();
        let new = replacement
            .observations
            .iter()
            .find(|observation| observation.step_id == "g-02")
            .unwrap();
        assert_ne!(old.ground_truth_entity_id, new.ground_truth_entity_id);
    }
}
