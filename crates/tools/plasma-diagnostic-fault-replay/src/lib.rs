use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

pub const SCHEMA_VERSION: &str = "luminous.plasma-diagnostic-fault-replay.v0.1";
pub const BENCHMARK_SCHEMA_VERSION: &str = "luminous.plasma-diagnostic-fault-replay-benchmark.v0.1";
pub const EVIDENCE_CLASS: &str = "SYNTHETIC_FAULT_REPLAY";
pub const CLAIM_CEILING: &str =
    "FAULT_PATTERN_ONLY_NO_DIAGNOSTIC_ACCURACY_OR_REAL_FAILURE_RATE_CLAIM";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayError {
    pub message: String,
}

impl ReplayError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ReplayError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRequest {
    pub schema_version: String,
    pub scenario: ScenarioSpec,
    pub schedule: FaultSchedule,
    pub points: Vec<EvaluationPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSpec {
    pub scenario_id: String,
    pub shot_id: String,
    pub regime_profile_id: String,
    pub clock_domain: String,
    pub duration_ns: u64,
    pub channels: Vec<ChannelSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelSpec {
    pub channel_id: String,
    pub diagnostic_system_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultSchedule {
    pub schedule_id: String,
    pub faults: Vec<FaultEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvent {
    pub fault_id: String,
    pub target: FaultTarget,
    pub mode: FaultMode,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum FaultTarget {
    Channel { channel_id: String },
    DiagnosticSystem { system_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultMode {
    Dropout,
    IntegrityFailure,
    Delay,
    Saturation,
    Bias,
    GainDrift,
    StaleCalibration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationPoint {
    pub point_id: String,
    pub time_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    SyntheticNominal,
    Unavailable,
    Invalid,
    Delayed,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelResult {
    pub channel_id: String,
    pub state: ChannelState,
    pub active_fault_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRow {
    pub point_id: String,
    pub time_ns: u64,
    pub channels: Vec<ChannelResult>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaySummary {
    pub synthetic_nominal: u64,
    pub unavailable: u64,
    pub invalid: u64,
    pub delayed: u64,
    pub degraded: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayReceiptBody {
    pub schema_version: String,
    pub scenario_id: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub claim_ceiling: String,
    pub scenario_sha256: String,
    pub schedule_sha256: String,
    pub evaluation_points_sha256: String,
    pub channel_ids: Vec<String>,
    pub rows: Vec<ReplayRow>,
    pub summary: ReplaySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayReceipt {
    pub body: ReplayReceiptBody,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkManifest {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    physical_measurements: bool,
    claim_ceiling: String,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    case_id: String,
    request: ReplayRequest,
    expected_disposition: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseAudit {
    pub case_id: String,
    pub actual_disposition: String,
    pub expected_disposition: String,
    pub matched_expectation: bool,
    pub error: Option<String>,
    pub receipt_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub benchmark_id: String,
    pub evidence_class: String,
    pub case_count: usize,
    pub matched_expectations: usize,
    pub mismatched_expectations: usize,
    pub results: Vec<CaseAudit>,
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn digest_serialized<T: Serialize>(value: &T) -> Result<String, ReplayError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ReplayError::new(format!("serialization failed: {error}")))?;
    Ok(digest_bytes(&bytes))
}

fn non_empty(value: &str, field: &str) -> Result<(), ReplayError> {
    if value.trim().is_empty() {
        return Err(ReplayError::new(format!("{field} must be non-empty")));
    }
    Ok(())
}

fn target_matches(target: &FaultTarget, channel: &ChannelSpec) -> bool {
    match target {
        FaultTarget::Channel { channel_id } => channel.channel_id == *channel_id,
        FaultTarget::DiagnosticSystem { system_id } => {
            channel.diagnostic_system_id == *system_id
        }
    }
}

fn validate_request(request: &ReplayRequest) -> Result<(), ReplayError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(ReplayError::new("unsupported replay schema version"));
    }

    non_empty(&request.scenario.scenario_id, "scenario_id")?;
    non_empty(&request.scenario.shot_id, "shot_id")?;
    non_empty(&request.scenario.regime_profile_id, "regime_profile_id")?;
    non_empty(&request.scenario.clock_domain, "clock_domain")?;
    non_empty(&request.schedule.schedule_id, "schedule_id")?;

    if request.scenario.duration_ns == 0 {
        return Err(ReplayError::new("scenario duration must be positive"));
    }
    if request.scenario.channels.is_empty() {
        return Err(ReplayError::new("scenario must declare at least one diagnostic channel"));
    }
    if request.points.is_empty() {
        return Err(ReplayError::new("at least one evaluation point is required"));
    }

    let mut channel_ids = BTreeSet::new();
    let mut system_ids = BTreeSet::new();
    for channel in &request.scenario.channels {
        non_empty(&channel.channel_id, "channel_id")?;
        non_empty(&channel.diagnostic_system_id, "diagnostic_system_id")?;
        if !channel_ids.insert(channel.channel_id.clone()) {
            return Err(ReplayError::new(format!(
                "duplicate channel_id: {}",
                channel.channel_id
            )));
        }
        system_ids.insert(channel.diagnostic_system_id.clone());
    }

    let mut point_ids = BTreeSet::new();
    let mut previous_time = None;
    for point in &request.points {
        non_empty(&point.point_id, "point_id")?;
        if !point_ids.insert(point.point_id.clone()) {
            return Err(ReplayError::new(format!("duplicate point_id: {}", point.point_id)));
        }
        if point.time_ns >= request.scenario.duration_ns {
            return Err(ReplayError::new(format!(
                "evaluation point {} lies outside the half-open scenario duration",
                point.point_id
            )));
        }
        if let Some(previous) = previous_time {
            if point.time_ns <= previous {
                return Err(ReplayError::new(
                    "evaluation points must be strictly increasing in time",
                ));
            }
        }
        previous_time = Some(point.time_ns);
    }

    let mut fault_ids = BTreeSet::new();
    for fault in &request.schedule.faults {
        non_empty(&fault.fault_id, "fault_id")?;
        if !fault_ids.insert(fault.fault_id.clone()) {
            return Err(ReplayError::new(format!("duplicate fault_id: {}", fault.fault_id)));
        }
        if fault.start_time_ns >= fault.end_time_ns {
            return Err(ReplayError::new(format!(
                "fault {} has an empty or reversed interval",
                fault.fault_id
            )));
        }
        if fault.end_time_ns > request.scenario.duration_ns {
            return Err(ReplayError::new(format!(
                "fault {} extends beyond the scenario duration",
                fault.fault_id
            )));
        }
        match &fault.target {
            FaultTarget::Channel { channel_id } => {
                if !channel_ids.contains(channel_id) {
                    return Err(ReplayError::new(format!(
                        "fault {} targets unknown channel {}",
                        fault.fault_id, channel_id
                    )));
                }
            }
            FaultTarget::DiagnosticSystem { system_id } => {
                if !system_ids.contains(system_id) {
                    return Err(ReplayError::new(format!(
                        "fault {} targets unknown diagnostic system {}",
                        fault.fault_id, system_id
                    )));
                }
            }
        }
    }

    // S0 uses unambiguous fault composition: two overlapping events cannot
    // affect the same channel, including channel-plus-system intersections.
    for left_index in 0..request.schedule.faults.len() {
        for right_index in (left_index + 1)..request.schedule.faults.len() {
            let left = &request.schedule.faults[left_index];
            let right = &request.schedule.faults[right_index];
            let intervals_overlap =
                left.start_time_ns < right.end_time_ns && right.start_time_ns < left.end_time_ns;
            if !intervals_overlap {
                continue;
            }
            let shared_channel = request.scenario.channels.iter().any(|channel| {
                target_matches(&left.target, channel) && target_matches(&right.target, channel)
            });
            if shared_channel {
                return Err(ReplayError::new(format!(
                    "overlapping faults {} and {} affect the same channel",
                    left.fault_id, right.fault_id
                )));
            }
        }
    }

    Ok(())
}

fn canonical_scenario(scenario: &ScenarioSpec) -> ScenarioSpec {
    let mut canonical = scenario.clone();
    canonical
        .channels
        .sort_by(|left, right| left.channel_id.cmp(&right.channel_id));
    canonical
}

fn canonical_schedule(schedule: &FaultSchedule) -> FaultSchedule {
    let mut canonical = schedule.clone();
    canonical.faults.sort_by(|left, right| left.fault_id.cmp(&right.fault_id));
    canonical
}

fn state_for_mode(mode: FaultMode) -> ChannelState {
    match mode {
        FaultMode::Dropout => ChannelState::Unavailable,
        FaultMode::IntegrityFailure | FaultMode::StaleCalibration => ChannelState::Invalid,
        FaultMode::Delay => ChannelState::Delayed,
        FaultMode::Saturation | FaultMode::Bias | FaultMode::GainDrift => ChannelState::Degraded,
    }
}

fn add_state(summary: &mut ReplaySummary, state: ChannelState) {
    match state {
        ChannelState::SyntheticNominal => summary.synthetic_nominal += 1,
        ChannelState::Unavailable => summary.unavailable += 1,
        ChannelState::Invalid => summary.invalid += 1,
        ChannelState::Delayed => summary.delayed += 1,
        ChannelState::Degraded => summary.degraded += 1,
    }
}

pub fn run_replay(request: &ReplayRequest) -> Result<ReplayReceipt, ReplayError> {
    validate_request(request)?;

    let scenario = canonical_scenario(&request.scenario);
    let schedule = canonical_schedule(&request.schedule);
    let scenario_sha256 = digest_serialized(&scenario)?;
    let schedule_sha256 = digest_serialized(&schedule)?;
    let evaluation_points_sha256 = digest_serialized(&request.points)?;
    let channel_ids: Vec<String> = scenario.channels.iter().map(|c| c.channel_id.clone()).collect();

    let mut summary = ReplaySummary::default();
    let mut rows = Vec::with_capacity(request.points.len());

    for point in &request.points {
        let mut channel_results = Vec::with_capacity(scenario.channels.len());
        for channel in &scenario.channels {
            let mut active: Vec<&FaultEvent> = schedule
                .faults
                .iter()
                .filter(|fault| {
                    fault.start_time_ns <= point.time_ns
                        && point.time_ns < fault.end_time_ns
                        && target_matches(&fault.target, channel)
                })
                .collect();

            active.sort_by(|left, right| left.fault_id.cmp(&right.fault_id));
            if active.len() > 1 {
                return Err(ReplayError::new(format!(
                    "ambiguous active fault composition for channel {} at {} ns",
                    channel.channel_id, point.time_ns
                )));
            }

            let (state, active_fault_ids) = match active.first() {
                Some(fault) => (state_for_mode(fault.mode), vec![fault.fault_id.clone()]),
                None => (ChannelState::SyntheticNominal, Vec::new()),
            };
            add_state(&mut summary, state);
            channel_results.push(ChannelResult {
                channel_id: channel.channel_id.clone(),
                state,
                active_fault_ids,
            });
        }
        rows.push(ReplayRow {
            point_id: point.point_id.clone(),
            time_ns: point.time_ns,
            channels: channel_results,
        });
    }

    let body = ReplayReceiptBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        scenario_id: scenario.scenario_id,
        evidence_class: EVIDENCE_CLASS.to_owned(),
        physical_measurements: false,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        scenario_sha256,
        schedule_sha256,
        evaluation_points_sha256,
        channel_ids,
        rows,
        summary,
    };
    let receipt_sha256 = digest_serialized(&body)?;

    Ok(ReplayReceipt { body, receipt_sha256 })
}

pub fn verify_receipt(receipt: &ReplayReceipt) -> Result<(), ReplayError> {
    if receipt.body.schema_version != SCHEMA_VERSION {
        return Err(ReplayError::new("receipt schema version is unsupported"));
    }
    if receipt.body.evidence_class != EVIDENCE_CLASS
        || receipt.body.physical_measurements
        || receipt.body.claim_ceiling != CLAIM_CEILING
    {
        return Err(ReplayError::new(
            "receipt evidence class or claim ceiling was changed",
        ));
    }
    if receipt.receipt_sha256 != digest_serialized(&receipt.body)? {
        return Err(ReplayError::new("receipt digest mismatch"));
    }

    let mut recomputed = ReplaySummary::default();
    let mut previous_time = None;
    for row in &receipt.body.rows {
        if let Some(previous) = previous_time {
            if row.time_ns <= previous {
                return Err(ReplayError::new("receipt rows are not strictly time ordered"));
            }
        }
        previous_time = Some(row.time_ns);

        let row_channel_ids: Vec<String> =
            row.channels.iter().map(|channel| channel.channel_id.clone()).collect();
        if row_channel_ids != receipt.body.channel_ids {
            return Err(ReplayError::new(
                "receipt channel inventory differs from its declared channel set",
            ));
        }
        for channel in &row.channels {
            add_state(&mut recomputed, channel.state);
        }
    }
    if recomputed != receipt.body.summary {
        return Err(ReplayError::new("receipt summary does not match row contents"));
    }
    Ok(())
}

pub fn audit_manifest_json(json: &str) -> Result<AuditReport, ReplayError> {
    let manifest: BenchmarkManifest = serde_json::from_str(json)
        .map_err(|error| ReplayError::new(format!("manifest parse failed: {error}")))?;

    if manifest.schema_version != BENCHMARK_SCHEMA_VERSION {
        return Err(ReplayError::new("unsupported benchmark manifest schema version"));
    }
    non_empty(&manifest.benchmark_id, "benchmark_id")?;
    if manifest.evidence_class != EVIDENCE_CLASS || manifest.physical_measurements {
        return Err(ReplayError::new(
            "benchmark manifest must remain explicitly synthetic",
        ));
    }
    if manifest.claim_ceiling != CLAIM_CEILING {
        return Err(ReplayError::new("benchmark claim ceiling was changed"));
    }
    if manifest.cases.is_empty() {
        return Err(ReplayError::new("benchmark manifest contains no cases"));
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(manifest.cases.len());
    for case in &manifest.cases {
        non_empty(&case.case_id, "case_id")?;
        if !case_ids.insert(case.case_id.clone()) {
            return Err(ReplayError::new(format!("duplicate case_id: {}", case.case_id)));
        }
        if case.expected_disposition != "COMPLETED"
            && case.expected_disposition != "REFUSED"
        {
            return Err(ReplayError::new(format!(
                "case {} has an unknown expected disposition",
                case.case_id
            )));
        }

        let (actual_disposition, error, receipt_sha256) = match run_replay(&case.request) {
            Ok(receipt) => {
                verify_receipt(&receipt)?;
                ("COMPLETED".to_owned(), None, Some(receipt.receipt_sha256))
            }
            Err(error) => ("REFUSED".to_owned(), Some(error.message), None),
        };
        let matched_expectation = actual_disposition == case.expected_disposition;
        results.push(CaseAudit {
            case_id: case.case_id.clone(),
            actual_disposition,
            expected_disposition: case.expected_disposition.clone(),
            matched_expectation,
            error,
            receipt_sha256,
        });
    }

    let matched_expectations = results.iter().filter(|result| result.matched_expectation).count();
    let case_count = results.len();
    Ok(AuditReport {
        benchmark_id: manifest.benchmark_id,
        evidence_class: manifest.evidence_class,
        case_count,
        matched_expectations,
        mismatched_expectations: case_count - matched_expectations,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> ReplayRequest {
        ReplayRequest {
            schema_version: SCHEMA_VERSION.to_owned(),
            scenario: ScenarioSpec {
                scenario_id: "scenario-001".to_owned(),
                shot_id: "synthetic-shot-001".to_owned(),
                regime_profile_id: "synthetic-diagnostic-profile-v0".to_owned(),
                clock_domain: "shot-relative-ns".to_owned(),
                duration_ns: 100,
                channels: vec![
                    ChannelSpec {
                        channel_id: "density".to_owned(),
                        diagnostic_system_id: "interferometer".to_owned(),
                    },
                    ChannelSpec {
                        channel_id: "temperature".to_owned(),
                        diagnostic_system_id: "thomson".to_owned(),
                    },
                ],
            },
            schedule: FaultSchedule {
                schedule_id: "schedule-001".to_owned(),
                faults: vec![FaultEvent {
                    fault_id: "drop-density".to_owned(),
                    target: FaultTarget::Channel {
                        channel_id: "density".to_owned(),
                    },
                    mode: FaultMode::Dropout,
                    start_time_ns: 20,
                    end_time_ns: 40,
                }],
            },
            points: vec![
                EvaluationPoint {
                    point_id: "p0".to_owned(),
                    time_ns: 0,
                },
                EvaluationPoint {
                    point_id: "p1".to_owned(),
                    time_ns: 20,
                },
                EvaluationPoint {
                    point_id: "p2".to_owned(),
                    time_ns: 40,
                },
            ],
        }
    }

    #[test]
    fn replay_is_deterministic_and_receipt_verifies() {
        let request = base_request();
        let receipt_a = run_replay(&request).expect("replay should complete");
        let receipt_b = run_replay(&request).expect("replay should complete");
        assert_eq!(receipt_a.receipt_sha256, receipt_b.receipt_sha256);
        verify_receipt(&receipt_a).expect("receipt should verify");
    }

    #[test]
    fn canonical_digest_ignores_channel_and_fault_input_order() {
        let request = base_request();
        let receipt_a = run_replay(&request).expect("first replay should complete");
        let mut reordered = request;
        reordered.scenario.channels.reverse();
        reordered.schedule.faults.reverse();
        let receipt_b = run_replay(&reordered).expect("reordered replay should complete");
        assert_eq!(receipt_a.body.scenario_sha256, receipt_b.body.scenario_sha256);
        assert_eq!(receipt_a.body.schedule_sha256, receipt_b.body.schedule_sha256);
        assert_eq!(receipt_a.receipt_sha256, receipt_b.receipt_sha256);
    }

    #[test]
    fn fault_intervals_are_half_open() {
        let receipt = run_replay(&base_request()).expect("replay should complete");
        assert_eq!(receipt.body.rows[0].channels[0].state, ChannelState::SyntheticNominal);
        assert_eq!(receipt.body.rows[1].channels[0].state, ChannelState::Unavailable);
        assert_eq!(receipt.body.rows[2].channels[0].state, ChannelState::SyntheticNominal);
    }

    #[test]
    fn diagnostic_system_fault_applies_to_every_member_channel() {
        let mut request = base_request();
        request.schedule.faults = vec![FaultEvent {
            fault_id: "thomson-system-loss".to_owned(),
            target: FaultTarget::DiagnosticSystem {
                system_id: "thomson".to_owned(),
            },
            mode: FaultMode::Dropout,
            start_time_ns: 0,
            end_time_ns: 50,
        }];
        let receipt = run_replay(&request).expect("group fault should replay");
        for row in &receipt.body.rows {
            assert_eq!(row.channels[1].state, ChannelState::Unavailable);
            assert_eq!(row.channels[1].active_fault_ids, vec!["thomson-system-loss"]);
        }
    }

    #[test]
    fn unknown_channel_target_is_refused() {
        let mut request = base_request();
        request.schedule.faults[0].target = FaultTarget::Channel {
            channel_id: "missing-channel".to_owned(),
        };
        assert!(run_replay(&request).is_err());
    }

    #[test]
    fn overlapping_faults_on_a_shared_channel_are_refused() {
        let mut request = base_request();
        request.schedule.faults.push(FaultEvent {
            fault_id: "interferometer-system-wide".to_owned(),
            target: FaultTarget::DiagnosticSystem {
                system_id: "interferometer".to_owned(),
            },
            mode: FaultMode::Degraded,
            start_time_ns: 30,
            end_time_ns: 60,
        });
        assert!(run_replay(&request).is_err());
    }

    #[test]
    fn duplicate_evaluation_points_are_refused() {
        let mut request = base_request();
        request.points.push(request.points[0].clone());
        assert!(run_replay(&request).is_err());
    }

    #[test]
    fn receipt_tampering_is_detected() {
        let mut receipt = run_replay(&base_request()).expect("replay should complete");
        receipt.body.rows[0].channels[0].state = ChannelState::Invalid;
        assert!(verify_receipt(&receipt).is_err());
    }

    #[test]
    fn receipt_cannot_be_promoted_to_measured_evidence() {
        let mut receipt = run_replay(&base_request()).expect("replay should complete");
        receipt.body.physical_measurements = true;
        receipt.receipt_sha256 = digest_serialized(&receipt.body).expect("digest should calculate");
        assert!(verify_receipt(&receipt).is_err());
    }

    #[test]
    fn repository_fixture_matches_all_expected_dispositions() {
        let manifest =
            include_str!("../../../../benchmarks/plasma/diagnostic-fault-replay-v0.json");
        let report = audit_manifest_json(manifest).expect("fixture should parse and audit");
        assert_eq!(report.case_count, 6);
        assert_eq!(report.matched_expectations, 6);
        assert_eq!(report.mismatched_expectations, 0);
    }
}
