//! Evidence-gated digital-twin commissioning evaluator.
//!
//! V0 accepts synthetic telemetry only. It verifies declared frequency,
//! jitter, latency, freshness, clock, calibration, fidelity and provenance
//! envelopes. It does not certify physical twin accuracy or ISO conformance.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Simulated,
    Measured,
    Inspected,
    Accepted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StreamRequirement {
    pub stream_id: String,
    pub owner_twin_id: String,
    pub schema_id: String,
    pub schema_version: String,
    pub clock_id: String,
    pub expected_interval_ticks: u64,
    pub maximum_jitter_ticks: u64,
    pub maximum_latency_ticks: u64,
    pub maximum_age_ticks: u64,
    pub maximum_clock_offset_ticks: u64,
    pub maximum_clock_drift_ppm: u64,
    pub maximum_fidelity_residual_units: u64,
    pub minimum_coverage_permille: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StreamObservation {
    pub stream_id: String,
    pub owner_twin_id: String,
    pub schema_id: String,
    pub schema_version: String,
    pub clock_id: String,
    pub evidence_class: EvidenceClass,
    pub source_timestamps_ticks: Vec<u64>,
    pub ingestion_timestamps_ticks: Vec<u64>,
    pub measured_clock_offset_ticks: u64,
    pub measured_clock_drift_ppm: u64,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub calibration_sha256: [u8; 32],
    pub calibration_expires_at_tick: u64,
    pub source_sha256: [u8; 32],
    pub transformation_receipt_sha256: [u8; 32],
    pub fidelity_residual_units: u64,
    pub coverage_permille: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommissioningInput {
    pub twin_id: String,
    pub twin_revision: String,
    pub declared_operational_purpose: String,
    pub configuration_sha256: [u8; 32],
    pub now_tick: u64,
    pub requested_capabilities: Vec<String>,
    pub permitted_capabilities: Vec<String>,
    pub required_streams: Vec<StreamRequirement>,
    pub observations: Vec<StreamObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TwinViolationCode {
    EmptyTwinIdentity,
    EmptyOperationalPurpose,
    ConfigurationDigestMissing,
    DuplicateRequiredStream,
    DuplicateObservation,
    MissingObservation,
    UnexpectedObservation,
    AuthorityEscalation,
    InvalidRequirement,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamViolationCode {
    IdentityMismatch,
    SchemaMismatch,
    ClockMismatch,
    EvidenceClassNotAdmitted,
    InsufficientSamples,
    TimestampCountMismatch,
    DuplicateTimestamp,
    OutOfOrderTimestamp,
    SampleJitterExceeded,
    IngestionBeforeSource,
    FutureSourceTimestamp,
    FutureIngestionTimestamp,
    LatencyExceeded,
    ObservationStale,
    ClockOffsetExceeded,
    ClockDriftExceeded,
    CalibrationIdentityMissing,
    CalibrationDigestMissing,
    CalibrationExpired,
    SourceDigestMissing,
    TransformationReceiptMissing,
    FidelityResidualExceeded,
    CoverageBelowMinimum,
    NumericOverflow,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StreamMetrics {
    pub sample_count: usize,
    pub mean_interval_ticks: Option<u64>,
    pub maximum_interval_ticks: Option<u64>,
    pub maximum_jitter_ticks: Option<u64>,
    pub p50_latency_ticks: Option<u64>,
    pub p95_latency_ticks: Option<u64>,
    pub p99_latency_ticks: Option<u64>,
    pub maximum_latency_ticks: Option<u64>,
    pub latest_data_age_ticks: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StreamReport {
    pub stream_id: String,
    pub commissioned: bool,
    pub metrics: StreamMetrics,
    pub violations: Vec<StreamViolationCode>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommissioningDecision {
    Ready,
    NotReady,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommissioningReport {
    pub report_version: String,
    pub twin_id: String,
    pub twin_revision: String,
    pub declared_operational_purpose: String,
    pub decision: CommissioningDecision,
    pub twin_violations: Vec<TwinViolationCode>,
    pub streams: Vec<StreamReport>,
    pub maximum_authority: String,
    pub iso_23247_7_conformance_claim: bool,
    pub claim_ceiling: String,
}

fn digest_present(digest: &[u8; 32]) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn percentile(values: &[u64], percent: usize) -> Option<u64> {
    if values.is_empty() || !(1..=100).contains(&percent) {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let rank = (percent * ordered.len()).div_ceil(100);
    ordered.get(rank.saturating_sub(1)).copied()
}

fn empty_metrics(sample_count: usize) -> StreamMetrics {
    StreamMetrics {
        sample_count,
        mean_interval_ticks: None,
        maximum_interval_ticks: None,
        maximum_jitter_ticks: None,
        p50_latency_ticks: None,
        p95_latency_ticks: None,
        p99_latency_ticks: None,
        maximum_latency_ticks: None,
        latest_data_age_ticks: None,
    }
}

fn assess_stream(
    input: &CommissioningInput,
    requirement: &StreamRequirement,
    observation: &StreamObservation,
) -> StreamReport {
    let mut violations = Vec::new();
    let add = |items: &mut Vec<StreamViolationCode>, item| {
        if !items.contains(&item) {
            items.push(item);
        }
    };

    if requirement.stream_id.trim().is_empty()
        || requirement.owner_twin_id.trim().is_empty()
        || requirement.schema_id.trim().is_empty()
        || requirement.schema_version.trim().is_empty()
        || requirement.clock_id.trim().is_empty()
        || requirement.expected_interval_ticks == 0
        || requirement.minimum_coverage_permille > 1_000
    {
        add(&mut violations, StreamViolationCode::NumericOverflow);
    }

    if observation.stream_id != requirement.stream_id
        || observation.owner_twin_id != requirement.owner_twin_id
    {
        add(&mut violations, StreamViolationCode::IdentityMismatch);
    }
    if observation.schema_id != requirement.schema_id
        || observation.schema_version != requirement.schema_version
    {
        add(&mut violations, StreamViolationCode::SchemaMismatch);
    }
    if observation.clock_id != requirement.clock_id {
        add(&mut violations, StreamViolationCode::ClockMismatch);
    }
    if observation.evidence_class != EvidenceClass::Synthetic {
        add(&mut violations, StreamViolationCode::EvidenceClassNotAdmitted);
    }

    if observation.calibration_id.trim().is_empty()
        || observation.calibration_revision.trim().is_empty()
    {
        add(&mut violations, StreamViolationCode::CalibrationIdentityMissing);
    }
    if !digest_present(&observation.calibration_sha256) {
        add(&mut violations, StreamViolationCode::CalibrationDigestMissing);
    }
    if observation.calibration_expires_at_tick <= input.now_tick {
        add(&mut violations, StreamViolationCode::CalibrationExpired);
    }
    if !digest_present(&observation.source_sha256) {
        add(&mut violations, StreamViolationCode::SourceDigestMissing);
    }
    if !digest_present(&observation.transformation_receipt_sha256) {
        add(&mut violations, StreamViolationCode::TransformationReceiptMissing);
    }
    if observation.fidelity_residual_units > requirement.maximum_fidelity_residual_units {
        add(&mut violations, StreamViolationCode::FidelityResidualExceeded);
    }
    if observation.coverage_permille < requirement.minimum_coverage_permille
        || observation.coverage_permille > 1_000
    {
        add(&mut violations, StreamViolationCode::CoverageBelowMinimum);
    }
    if observation.measured_clock_offset_ticks > requirement.maximum_clock_offset_ticks {
        add(&mut violations, StreamViolationCode::ClockOffsetExceeded);
    }
    if observation.measured_clock_drift_ppm > requirement.maximum_clock_drift_ppm {
        add(&mut violations, StreamViolationCode::ClockDriftExceeded);
    }

    let source = &observation.source_timestamps_ticks;
    let ingestion = &observation.ingestion_timestamps_ticks;
    let mut metrics = empty_metrics(source.len());

    if source.len() < 2 {
        add(&mut violations, StreamViolationCode::InsufficientSamples);
    }
    if source.len() != ingestion.len() {
        add(&mut violations, StreamViolationCode::TimestampCountMismatch);
    }

    let paired = source.len().min(ingestion.len());
    let mut latencies = Vec::with_capacity(paired);
    for index in 0..paired {
        let source_tick = source[index];
        let ingest_tick = ingestion[index];
        if source_tick > input.now_tick {
            add(&mut violations, StreamViolationCode::FutureSourceTimestamp);
        }
        if ingest_tick > input.now_tick {
            add(&mut violations, StreamViolationCode::FutureIngestionTimestamp);
        }
        match ingest_tick.checked_sub(source_tick) {
            Some(latency) => {
                if latency > requirement.maximum_latency_ticks {
                    add(&mut violations, StreamViolationCode::LatencyExceeded);
                }
                latencies.push(latency);
            }
            None => add(&mut violations, StreamViolationCode::IngestionBeforeSource),
        }
    }

    let mut intervals = Vec::new();
    let mut maximum_jitter = 0u64;
    for pair in source.windows(2) {
        match pair[1].checked_sub(pair[0]) {
            Some(0) => add(&mut violations, StreamViolationCode::DuplicateTimestamp),
            Some(interval) => {
                let jitter = interval.abs_diff(requirement.expected_interval_ticks);
                maximum_jitter = maximum_jitter.max(jitter);
                if jitter > requirement.maximum_jitter_ticks {
                    add(&mut violations, StreamViolationCode::SampleJitterExceeded);
                }
                intervals.push(interval);
            }
            None => add(&mut violations, StreamViolationCode::OutOfOrderTimestamp),
        }
    }

    let maximum_interval = intervals.iter().copied().max();
    let mean_interval = if intervals.is_empty() {
        None
    } else {
        let sum = intervals
            .iter()
            .try_fold(0u64, |acc, value| acc.checked_add(*value));
        match sum {
            Some(total) => Some(total / intervals.len() as u64),
            None => {
                add(&mut violations, StreamViolationCode::NumericOverflow);
                None
            }
        }
    };

    let latest_age = source.last().and_then(|last| {
        if *last > input.now_tick {
            None
        } else {
            Some(input.now_tick - *last)
        }
    });
    if latest_age.is_some_and(|age| age > requirement.maximum_age_ticks) {
        add(&mut violations, StreamViolationCode::ObservationStale);
    }

    metrics = StreamMetrics {
        sample_count: source.len(),
        mean_interval_ticks: mean_interval,
        maximum_interval_ticks: maximum_interval,
        maximum_jitter_ticks: if intervals.is_empty() {
            None
        } else {
            Some(maximum_jitter)
        },
        p50_latency_ticks: percentile(&latencies, 50),
        p95_latency_ticks: percentile(&latencies, 95),
        p99_latency_ticks: percentile(&latencies, 99),
        maximum_latency_ticks: latencies.iter().copied().max(),
        latest_data_age_ticks: latest_age,
    };

    StreamReport {
        stream_id: requirement.stream_id.clone(),
        commissioned: violations.is_empty(),
        metrics,
        violations,
    }
}

fn missing_stream_report(stream_id: &str) -> StreamReport {
    StreamReport {
        stream_id: stream_id.to_owned(),
        commissioned: false,
        metrics: empty_metrics(0),
        violations: vec![StreamViolationCode::InsufficientSamples],
    }
}

pub fn commission_twin(input: &CommissioningInput) -> CommissioningReport {
    let mut twin_violations = Vec::new();
    let add_twin = |items: &mut Vec<TwinViolationCode>, item| {
        if !items.contains(&item) {
            items.push(item);
        }
    };

    if input.twin_id.trim().is_empty() || input.twin_revision.trim().is_empty() {
        add_twin(&mut twin_violations, TwinViolationCode::EmptyTwinIdentity);
    }
    if input.declared_operational_purpose.trim().is_empty() {
        add_twin(&mut twin_violations, TwinViolationCode::EmptyOperationalPurpose);
    }
    if !digest_present(&input.configuration_sha256) {
        add_twin(&mut twin_violations, TwinViolationCode::ConfigurationDigestMissing);
    }

    let permitted: HashSet<&str> = input
        .permitted_capabilities
        .iter()
        .map(String::as_str)
        .collect();
    if input
        .requested_capabilities
        .iter()
        .any(|capability| !permitted.contains(capability.as_str()))
    {
        add_twin(&mut twin_violations, TwinViolationCode::AuthorityEscalation);
    }

    let mut required_ids = HashSet::new();
    for required in &input.required_streams {
        if !required_ids.insert(required.stream_id.as_str()) {
            add_twin(&mut twin_violations, TwinViolationCode::DuplicateRequiredStream);
        }
    }

    let mut observation_map: HashMap<&str, &StreamObservation> = HashMap::new();
    for observation in &input.observations {
        if observation_map
            .insert(observation.stream_id.as_str(), observation)
            .is_some()
        {
            add_twin(&mut twin_violations, TwinViolationCode::DuplicateObservation);
        }
        if !required_ids.contains(observation.stream_id.as_str()) {
            add_twin(&mut twin_violations, TwinViolationCode::UnexpectedObservation);
        }
    }

    let mut required = input.required_streams.clone();
    required.sort_by(|left, right| left.stream_id.cmp(&right.stream_id));
    let mut streams = Vec::with_capacity(required.len());
    for requirement in &required {
        match observation_map.get(requirement.stream_id.as_str()) {
            Some(observation) => {
                streams.push(assess_stream(input, requirement, observation));
            }
            None => {
                add_twin(&mut twin_violations, TwinViolationCode::MissingObservation);
                streams.push(missing_stream_report(&requirement.stream_id));
            }
        }
    }

    let ready = twin_violations.is_empty() && streams.iter().all(|stream| stream.commissioned);
    CommissioningReport {
        report_version: "0.1.0".to_owned(),
        twin_id: input.twin_id.clone(),
        twin_revision: input.twin_revision.clone(),
        declared_operational_purpose: input.declared_operational_purpose.clone(),
        decision: if ready {
            CommissioningDecision::Ready
        } else {
            CommissioningDecision::NotReady
        },
        twin_violations,
        streams,
        maximum_authority: "READ_ONLY_TELEMETRY".to_owned(),
        iso_23247_7_conformance_claim: false,
        claim_ceiling: "SYNTHETIC_TWIN_COMMISSIONING_CONTRACT_ONLY".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hashes(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn stream_fixture(
        stream_id: &str,
        expected_interval: u64,
        source_ticks: Vec<u64>,
        latency: u64,
        max_latency: u64,
        max_age: u64,
    ) -> (StreamRequirement, StreamObservation) {
        let requirement = StreamRequirement {
            stream_id: stream_id.to_owned(),
            owner_twin_id: format!("OWNER-{stream_id}"),
            schema_id: format!("urn:luminous:{stream_id}"),
            schema_version: "1.0.0".to_owned(),
            clock_id: "CELL-MASTER".to_owned(),
            expected_interval_ticks: expected_interval,
            maximum_jitter_ticks: 0,
            maximum_latency_ticks: max_latency,
            maximum_age_ticks: max_age,
            maximum_clock_offset_ticks: 5,
            maximum_clock_drift_ppm: 10,
            maximum_fidelity_residual_units: 5,
            minimum_coverage_permille: 990,
        };
        let observation = StreamObservation {
            stream_id: stream_id.to_owned(),
            owner_twin_id: format!("OWNER-{stream_id}"),
            schema_id: format!("urn:luminous:{stream_id}"),
            schema_version: "1.0.0".to_owned(),
            clock_id: "CELL-MASTER".to_owned(),
            evidence_class: EvidenceClass::Synthetic,
            ingestion_timestamps_ticks: source_ticks
                .iter()
                .map(|tick| tick + latency)
                .collect(),
            source_timestamps_ticks: source_ticks,
            measured_clock_offset_ticks: 2,
            measured_clock_drift_ppm: 2,
            calibration_id: format!("CAL-{stream_id}"),
            calibration_revision: "R1".to_owned(),
            calibration_sha256: hashes(3),
            calibration_expires_at_tick: 5_000,
            source_sha256: hashes(1),
            transformation_receipt_sha256: hashes(2),
            fidelity_residual_units: 3,
            coverage_permille: 995,
        };
        (requirement, observation)
    }

    pub fn base_input() -> CommissioningInput {
        let fixtures = vec![
            stream_fixture("process-current-voltage", 1, vec![4_196, 4_197, 4_198, 4_199, 4_200], 1, 5, 10),
            stream_fixture("robot-tcp", 80, vec![3_880, 3_960, 4_040, 4_120, 4_200], 2, 5, 100),
            stream_fixture("power-source-opcua", 1_000, vec![200, 1_200, 2_200, 3_200, 4_200], 5, 10, 1_100),
            stream_fixture("scan-profiles", 200, vec![3_400, 3_600, 3_800, 4_000, 4_200], 3, 10, 250),
            stream_fixture("video-triggers", 250, vec![3_200, 3_450, 3_700, 3_950, 4_200], 10, 20, 300),
        ];
        let (required_streams, observations): (Vec<_>, Vec<_>) = fixtures.into_iter().unzip();
        CommissioningInput {
            twin_id: "SHIPYARD-WELD-CELL-01".to_owned(),
            twin_revision: "CELL-R7".to_owned(),
            declared_operational_purpose: "offline_state_visualization_and_recommendation".to_owned(),
            configuration_sha256: hashes(8),
            now_tick: 4_205,
            requested_capabilities: vec!["read_only_telemetry".to_owned()],
            permitted_capabilities: vec!["read_only_telemetry".to_owned()],
            required_streams,
            observations,
        }
    }

    #[test]
    fn heterogeneous_valid_streams_are_commissioned() {
        let report = commission_twin(&base_input());
        assert_eq!(report.decision, CommissioningDecision::Ready);
        assert_eq!(report.streams.len(), 5);
        assert!(report.streams.iter().all(|stream| stream.commissioned));
        assert!(!report.iso_23247_7_conformance_claim);
        assert_eq!(report.maximum_authority, "READ_ONLY_TELEMETRY");
    }

    #[test]
    fn process_and_robot_cadences_remain_distinct() {
        let report = commission_twin(&base_input());
        let process = report.streams.iter().find(|s| s.stream_id == "process-current-voltage").unwrap();
        let robot = report.streams.iter().find(|s| s.stream_id == "robot-tcp").unwrap();
        assert_eq!(process.metrics.mean_interval_ticks, Some(1));
        assert_eq!(robot.metrics.mean_interval_ticks, Some(80));
    }

    #[test]
    fn reports_latency_percentiles_and_maximum() {
        let mut input = base_input();
        let observation = input.observations.iter_mut()
            .find(|s| s.stream_id == "robot-tcp").unwrap();
        observation.ingestion_timestamps_ticks = vec![3_883, 3_966, 4_047, 4_128, 4_210];
        let report = commission_twin(&input);
        let robot = report.streams.iter().find(|s| s.stream_id == "robot-tcp").unwrap();
        assert_eq!(robot.metrics.p50_latency_ticks, Some(8));
        assert_eq!(robot.metrics.p95_latency_ticks, Some(10));
        assert_eq!(robot.metrics.p99_latency_ticks, Some(10));
        assert_eq!(robot.metrics.maximum_latency_ticks, Some(10));
    }

    #[test]
    fn missing_required_stream_blocks_commissioning() {
        let mut input = base_input();
        input.observations.pop();
        let report = commission_twin(&input);
        assert_eq!(report.decision, CommissioningDecision::NotReady);
        assert!(report.twin_violations.contains(&TwinViolationCode::MissingObservation));
    }

    #[test]
    fn unexpected_stream_blocks_commissioning() {
        let mut input = base_input();
        input.observations[0].stream_id = "unregistered-stream".to_owned();
        let report = commission_twin(&input);
        assert!(report.twin_violations.contains(&TwinViolationCode::UnexpectedObservation));
    }

    #[test]
    fn duplicate_observation_blocks_commissioning() {
        let mut input = base_input();
        input.observations.push(input.observations[0].clone());
        let report = commission_twin(&input);
        assert!(report.twin_violations.contains(&TwinViolationCode::DuplicateObservation));
    }

    #[test]
    fn duplicate_required_stream_blocks_commissioning() {
        let mut input = base_input();
        input.required_streams.push(input.required_streams[0].clone());
        let report = commission_twin(&input);
        assert!(report.twin_violations.contains(&TwinViolationCode::DuplicateRequiredStream));
    }

    #[test]
    fn wrong_owner_blocks_stream_commissioning() {
        let mut input = base_input();
        input.observations[0].owner_twin_id = "OTHER-OWNER".to_owned();
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::IdentityMismatch)));
    }

    #[test]
    fn schema_version_mismatch_blocks_commissioning() {
        let mut input = base_input();
        input.observations[0].schema_version = "2.0.0".to_owned();
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::SchemaMismatch)));
    }

    #[test]
    fn wrong_clock_id_blocks_commissioning() {
        let mut input = base_input();
        input.observations[0].clock_id = "UNRELATED-CLOCK".to_owned();
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::ClockMismatch)));
    }

    #[test]
    fn duplicate_timestamps_are_detected() {
        let mut input = base_input();
        input.observations[0].source_timestamps_ticks[2] = input.observations[0].source_timestamps_ticks[1];
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::DuplicateTimestamp)));
    }

    #[test]
    fn out_of_order_timestamps_are_detected() {
        let mut input = base_input();
        input.observations[0].source_timestamps_ticks.swap(2, 3);
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::OutOfOrderTimestamp)));
    }

    #[test]
    fn dropped_sample_gap_exceeds_cadence_budget() {
        let mut input = base_input();
        input.observations[1].source_timestamps_ticks[3] += 160;
        input.observations[1].ingestion_timestamps_ticks[3] += 160;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::SampleJitterExceeded)));
    }

    #[test]
    fn latency_budget_violation_is_detected() {
        let mut input = base_input();
        input.observations[0].ingestion_timestamps_ticks[2] += 20;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::LatencyExceeded)));
    }

    #[test]
    fn stale_observation_is_rejected() {
        let mut input = base_input();
        input.observations[4].source_timestamps_ticks[4] = 3_000;
        input.observations[4].ingestion_timestamps_ticks[4] = 3_010;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::ObservationStale)));
    }

    #[test]
    fn future_source_timestamp_is_rejected() {
        let mut input = base_input();
        input.observations[0].source_timestamps_ticks[4] = 4_300;
        input.observations[0].ingestion_timestamps_ticks[4] = 4_301;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::FutureSourceTimestamp)));
    }

    #[test]
    fn ingestion_before_source_is_rejected() {
        let mut input = base_input();
        input.observations[0].ingestion_timestamps_ticks[0] = 4_190;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::IngestionBeforeSource)));
    }

    #[test]
    fn clock_offset_budget_is_enforced() {
        let mut input = base_input();
        input.observations[0].measured_clock_offset_ticks = 6;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::ClockOffsetExceeded)));
    }

    #[test]
    fn clock_drift_budget_is_enforced() {
        let mut input = base_input();
        input.observations[0].measured_clock_drift_ppm = 11;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::ClockDriftExceeded)));
    }

    #[test]
    fn expired_calibration_is_rejected() {
        let mut input = base_input();
        input.observations[0].calibration_expires_at_tick = input.now_tick;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|stream| stream.violations.contains(&StreamViolationCode::CalibrationExpired)));
    }

    #[test]
    fn missing_provenance_digests_are_rejected() {
        let mut input = base_input();
        input.observations[0].source_sha256 = [0; 32];
        input.observations[1].transformation_receipt_sha256 = [0; 32];
        input.observations[2].calibration_sha256 = [0; 32];
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|s| s.violations.contains(&StreamViolationCode::SourceDigestMissing)));
        assert!(report.streams.iter().any(|s| s.violations.contains(&StreamViolationCode::TransformationReceiptMissing)));
        assert!(report.streams.iter().any(|s| s.violations.contains(&StreamViolationCode::CalibrationDigestMissing)));
    }

    #[test]
    fn fidelity_residual_budget_is_enforced() {
        let mut input = base_input();
        input.observations[0].fidelity_residual_units = 6;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|s| s.violations.contains(&StreamViolationCode::FidelityResidualExceeded)));
    }

    #[test]
    fn coverage_budget_is_enforced() {
        let mut input = base_input();
        input.observations[0].coverage_permille = 989;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|s| s.violations.contains(&StreamViolationCode::CoverageBelowMinimum)));
    }

    #[test]
    fn non_synthetic_evidence_is_not_admitted_in_v0() {
        let mut input = base_input();
        input.observations[0].evidence_class = EvidenceClass::Measured;
        let report = commission_twin(&input);
        assert!(report.streams.iter().any(|s| s.violations.contains(&StreamViolationCode::EvidenceClassNotAdmitted)));
    }

    #[test]
    fn authority_escalation_is_rejected() {
        let mut input = base_input();
        input.requested_capabilities.push("robot_motion_command".to_owned());
        let report = commission_twin(&input);
        assert!(report.twin_violations.contains(&TwinViolationCode::AuthorityEscalation));
    }

    #[test]
    fn missing_configuration_digest_blocks_commissioning() {
        let mut input = base_input();
        input.configuration_sha256 = [0; 32];
        let report = commission_twin(&input);
        assert!(report.twin_violations.contains(&TwinViolationCode::ConfigurationDigestMissing));
    }
}
