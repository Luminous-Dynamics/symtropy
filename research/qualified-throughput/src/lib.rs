//! Campaign-level simulation benchmark normalization.
//!
//! The evaluator consumes existing campaign receipts. It does not run a
//! simulator, authenticate external instruments, or certify physical truth.
//! Raw work is reported separately from work that passed every case-specific
//! semantic, horizon, numerical, invariant, replay and evidence gate.
//!
//! No missing case may disappear from the denominator. Every campaign must
//! supply exactly one explicit result per case in the frozen suite.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: u32 = 1;
const CLAIM_CEILING: &str =
    "finite-campaign-receipt-normalization-only;does-not-run-simulations-or-authenticate-measurements";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkInputV1 {
    pub schema_version: u32,
    pub benchmark_id: String,
    pub suite_sha256: String,
    pub comparison_group_id: String,
    pub hardware_profile_sha256: String,
    pub workload_family: WorkloadFamily,
    pub max_energy_sample_gap_us: Option<u64>,
    pub cases: Vec<CaseRequirement>,
    pub campaigns: Vec<EngineCampaign>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadFamily {
    SingleWorldRigidBody,
    IndependentEnvironmentBatch,
    MaterialPointMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaseRequirement {
    pub case_id: String,
    pub required_simulated_ticks: u64,
    /// Absolute upper bound in the metric unit defined by the suite.
    pub max_error_upper_bound: f64,
    pub max_hard_constraint_violations: u64,
    pub require_replay_conformance: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EngineCampaign {
    pub engine_id: String,
    pub engine_version: String,
    pub executable_sha256: String,
    pub configuration_sha256: String,
    pub hardware_profile_sha256: String,
    pub campaign_wall_time_us: u64,
    pub peak_memory_bytes: u64,
    pub step_latency_p50_us: u64,
    pub step_latency_p95_us: u64,
    pub step_latency_p99_us: u64,
    pub energy_measurement: Option<EnergyMeasurement>,
    pub results: Vec<CaseResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnergyMeasurement {
    pub scope: EnergyScope,
    pub source: EnergySource,
    pub source_api_version: String,
    pub sample_series_sha256: String,
    pub integrated_energy_uj: f64,
    pub measurement_window_us: u64,
    pub sample_count: u32,
    pub max_sample_gap_us: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnergyScope {
    GpuBoard,
    CpuPackage,
    WholeSystem,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnergySource {
    NvmlPowerSamples,
    DcgmProfiling,
    RaplEnergyCounter,
    ExternalPowerMeter,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaseResult {
    pub case_id: String,
    pub outcome: CaseOutcome,
    pub semantic_equivalence: SemanticEquivalence,
    pub completed_simulated_ticks: u64,
    pub error_upper_bound: Option<f64>,
    pub hard_constraint_violations: Option<u64>,
    pub replay_conformant: Option<bool>,
    pub evidence_receipt_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseOutcome {
    Completed,
    TimedOut,
    Failed,
    Abstained,
    NotRun,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticEquivalence {
    Matched,
    Mismatch,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    NotCompleted,
    SemanticMismatch,
    SemanticEquivalenceUnknown,
    HorizonIncomplete,
    ErrorEvidenceMissing,
    ErrorBoundExceeded,
    HardConstraintEvidenceMissing,
    HardConstraintExceeded,
    ReplayEvidenceMissing,
    ReplayNonConformant,
    EvidenceReceiptMissing,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaseEvaluation {
    pub case_id: String,
    pub outcome: CaseOutcome,
    pub qualified: bool,
    pub rejection_reasons: Vec<RejectionReason>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CampaignReport {
    pub engine_id: String,
    pub engine_version: String,
    pub executable_sha256: String,
    pub configuration_sha256: String,
    pub hardware_profile_sha256: String,
    pub requested_cases: u64,
    pub requested_simulated_ticks: u64,
    pub completed_cases: u64,
    pub completed_unqualified_cases: u64,
    pub timed_out_cases: u64,
    pub failed_cases: u64,
    pub abstained_cases: u64,
    pub not_run_cases: u64,
    pub qualified_cases: u64,
    pub raw_simulated_ticks: u64,
    pub qualified_simulated_ticks: u64,
    pub case_coverage: f64,
    pub horizon_coverage: f64,
    pub raw_simulated_ticks_per_second: f64,
    pub qualified_simulated_ticks_per_second: f64,
    pub max_qualified_error_ratio: Option<f64>,
    pub step_latency_p50_us: u64,
    pub step_latency_p95_us: u64,
    pub step_latency_p99_us: u64,
    pub peak_memory_bytes: u64,
    pub energy_scope: Option<EnergyScope>,
    pub energy_per_qualified_tick_uj: Option<f64>,
    pub evaluations: Vec<CaseEvaluation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkReportV1 {
    pub schema_version: u32,
    pub input_sha256: String,
    pub benchmark_id: String,
    pub suite_sha256: String,
    pub comparison_group_id: String,
    pub hardware_profile_sha256: String,
    pub workload_family: WorkloadFamily,
    pub energy_included_in_frontier: bool,
    pub campaigns: Vec<CampaignReport>,
    pub pareto_frontier_engine_ids: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    UnsupportedSchema(u32),
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    NonFiniteOrNegative(&'static str),
    ZeroValue(&'static str),
    DuplicateCaseId(String),
    DuplicateEngineId(String),
    DuplicateResult(String),
    MissingCaseResult(String),
    UnexpectedCaseResult(String),
    HardwareProfileMismatch(String),
    SimulatedTicksExceedCaseHorizon(String),
    EnergySamplingPolicyMissing(String),
    EnergySampleGapExceeded(String),
    InvalidLatencyOrder(String),
    Serialization(String),
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => write!(f, "unsupported schema version: {v}"),
            Self::EmptyField(name) => write!(f, "required field is empty: {name}"),
            Self::InvalidSha256(name) => write!(f, "invalid SHA-256 identity: {name}"),
            Self::NonFiniteOrNegative(name) => {
                write!(f, "value must be finite and non-negative: {name}")
            }
            Self::ZeroValue(name) => write!(f, "value must be non-zero: {name}"),
            Self::DuplicateCaseId(id) => write!(f, "duplicate case ID: {id}"),
            Self::DuplicateEngineId(id) => write!(f, "duplicate engine ID: {id}"),
            Self::DuplicateResult(id) => write!(f, "duplicate result for case: {id}"),
            Self::MissingCaseResult(id) => write!(f, "campaign omitted required case: {id}"),
            Self::UnexpectedCaseResult(id) => write!(f, "campaign contains an unknown case: {id}"),
            Self::HardwareProfileMismatch(id) => {
                write!(f, "campaign uses a different comparison hardware profile: {id}")
            }
            Self::SimulatedTicksExceedCaseHorizon(id) => {
                write!(f, "campaign reports more simulated ticks than requested for case: {id}")
            }
            Self::EnergySamplingPolicyMissing(id) => {
                write!(f, "energy receipt has no preregistered sample-gap policy: {id}")
            }
            Self::EnergySampleGapExceeded(id) => {
                write!(f, "energy receipt exceeds sample-gap policy: {id}")
            }
            Self::InvalidLatencyOrder(id) => {
                write!(f, "latency percentiles are not ordered for campaign: {id}")
            }
            Self::Serialization(message) => write!(f, "serialization failure: {message}"),
        }
    }
}

impl std::error::Error for EvaluationError {}

/// Evaluate a complete benchmark comparison group.
pub fn evaluate(input: &BenchmarkInputV1) -> Result<BenchmarkReportV1, EvaluationError> {
    validate_input(input)?;
    let canonical = canonicalize(input);
    let input_sha256 = canonical_sha256(&canonical)?;
    let energy_included = energy_is_comparable(&canonical.campaigns);
    let requested_cases = canonical.cases.len() as u64;
    let requested_ticks = canonical
        .cases
        .iter()
        .map(|case| case.required_simulated_ticks)
        .sum::<u64>();

    let mut reports = canonical
        .campaigns
        .iter()
        .map(|campaign| evaluate_campaign(&canonical, campaign, requested_cases, requested_ticks))
        .collect::<Vec<_>>();

    let frontier = pareto_frontier(&reports, energy_included);
    reports.sort_by(|a, b| a.engine_id.cmp(&b.engine_id));

    Ok(BenchmarkReportV1 {
        schema_version: SCHEMA_VERSION,
        input_sha256,
        benchmark_id: canonical.benchmark_id,
        suite_sha256: canonical.suite_sha256,
        comparison_group_id: canonical.comparison_group_id,
        hardware_profile_sha256: canonical.hardware_profile_sha256,
        workload_family: canonical.workload_family,
        energy_included_in_frontier: energy_included,
        campaigns: reports,
        pareto_frontier_engine_ids: frontier,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

fn evaluate_campaign(
    input: &BenchmarkInputV1,
    campaign: &EngineCampaign,
    requested_cases: u64,
    requested_ticks: u64,
) -> CampaignReport {
    let requirements = input
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut evaluations = Vec::with_capacity(campaign.results.len());
    let mut completed_cases = 0_u64;
    let mut completed_unqualified_cases = 0_u64;
    let mut timed_out_cases = 0_u64;
    let mut failed_cases = 0_u64;
    let mut abstained_cases = 0_u64;
    let mut not_run_cases = 0_u64;
    let mut raw_ticks = 0_u64;
    let mut qualified_ticks = 0_u64;
    let mut qualified_cases = 0_u64;
    let mut max_error_ratio: Option<f64> = None;

    for result in &campaign.results {
        let requirement = requirements[result.case_id.as_str()];
        raw_ticks += result.completed_simulated_ticks;
        let reasons = case_rejection_reasons(requirement, result);

        match result.outcome {
            CaseOutcome::Completed => completed_cases += 1,
            CaseOutcome::TimedOut => timed_out_cases += 1,
            CaseOutcome::Failed => failed_cases += 1,
            CaseOutcome::Abstained => abstained_cases += 1,
            CaseOutcome::NotRun => not_run_cases += 1,
        }

        let qualified = reasons.is_empty();
        if qualified {
            qualified_cases += 1;
            qualified_ticks += requirement.required_simulated_ticks;
            if let Some(error) = result.error_upper_bound {
                let ratio = error / requirement.max_error_upper_bound;
                max_error_ratio = Some(max_error_ratio.map_or(ratio, |old| old.max(ratio)));
            }
        } else if result.outcome == CaseOutcome::Completed {
            completed_unqualified_cases += 1;
        }

        evaluations.push(CaseEvaluation {
            case_id: result.case_id.clone(),
            outcome: result.outcome,
            qualified,
            rejection_reasons: reasons,
        });
    }

    let wall_seconds = campaign.campaign_wall_time_us as f64 / 1_000_000.0;
    let case_coverage = ratio_or_zero(qualified_cases, requested_cases);
    let horizon_coverage = ratio_or_zero(qualified_ticks, requested_ticks);
    let raw_rate = raw_ticks as f64 / wall_seconds;
    let qualified_rate = qualified_ticks as f64 / wall_seconds;
    let (energy_scope, energy_per_tick) = match (
        &campaign.energy_measurement,
        qualified_ticks > 0,
    ) {
        (Some(energy), true) => (
            Some(energy.scope),
            Some(energy.integrated_energy_uj / qualified_ticks as f64),
        ),
        (Some(energy), false) => (Some(energy.scope), None),
        (None, _) => (None, None),
    };

    CampaignReport {
        engine_id: campaign.engine_id.clone(),
        engine_version: campaign.engine_version.clone(),
        executable_sha256: campaign.executable_sha256.clone(),
        configuration_sha256: campaign.configuration_sha256.clone(),
        hardware_profile_sha256: campaign.hardware_profile_sha256.clone(),
        requested_cases,
        requested_simulated_ticks: requested_ticks,
        completed_cases,
        completed_unqualified_cases,
        timed_out_cases,
        failed_cases,
        abstained_cases,
        not_run_cases,
        qualified_cases,
        raw_simulated_ticks: raw_ticks,
        qualified_simulated_ticks: qualified_ticks,
        case_coverage,
        horizon_coverage,
        raw_simulated_ticks_per_second: raw_rate,
        qualified_simulated_ticks_per_second: qualified_rate,
        max_qualified_error_ratio: max_error_ratio,
        step_latency_p50_us: campaign.step_latency_p50_us,
        step_latency_p95_us: campaign.step_latency_p95_us,
        step_latency_p99_us: campaign.step_latency_p99_us,
        peak_memory_bytes: campaign.peak_memory_bytes,
        energy_scope,
        energy_per_qualified_tick_uj: energy_per_tick,
        evaluations,
    }
}

fn case_rejection_reasons(
    requirement: &CaseRequirement,
    result: &CaseResult,
) -> Vec<RejectionReason> {
    let mut reasons = Vec::new();
    if result.outcome != CaseOutcome::Completed {
        reasons.push(RejectionReason::NotCompleted);
        return reasons;
    }
    match result.semantic_equivalence {
        SemanticEquivalence::Matched => {}
        SemanticEquivalence::Mismatch => reasons.push(RejectionReason::SemanticMismatch),
        SemanticEquivalence::Unknown => {
            reasons.push(RejectionReason::SemanticEquivalenceUnknown)
        }
    }
    if result.completed_simulated_ticks < requirement.required_simulated_ticks {
        reasons.push(RejectionReason::HorizonIncomplete);
    }
    match result.error_upper_bound {
        None => reasons.push(RejectionReason::ErrorEvidenceMissing),
        Some(error) if error > requirement.max_error_upper_bound => {
            reasons.push(RejectionReason::ErrorBoundExceeded)
        }
        Some(_) => {}
    }
    match result.hard_constraint_violations {
        None => reasons.push(RejectionReason::HardConstraintEvidenceMissing),
        Some(count) if count > requirement.max_hard_constraint_violations => {
            reasons.push(RejectionReason::HardConstraintExceeded)
        }
        Some(_) => {}
    }
    if requirement.require_replay_conformance {
        match result.replay_conformant {
            None => reasons.push(RejectionReason::ReplayEvidenceMissing),
            Some(false) => reasons.push(RejectionReason::ReplayNonConformant),
            Some(true) => {}
        }
    }
    if result.evidence_receipt_sha256.is_none() {
        reasons.push(RejectionReason::EvidenceReceiptMissing);
    }
    reasons
}

fn validate_input(input: &BenchmarkInputV1) -> Result<(), EvaluationError> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(EvaluationError::UnsupportedSchema(input.schema_version));
    }
    validate_nonempty(&input.benchmark_id, "benchmark_id")?;
    validate_nonempty(&input.comparison_group_id, "comparison_group_id")?;
    validate_sha256(&input.suite_sha256, "suite_sha256")?;
    validate_sha256(&input.hardware_profile_sha256, "hardware_profile_sha256")?;
    if let Some(max_gap) = input.max_energy_sample_gap_us {
        if max_gap == 0 {
            return Err(EvaluationError::ZeroValue("max_energy_sample_gap_us"));
        }
    }
    if input.cases.is_empty() {
        return Err(EvaluationError::EmptyField("cases"));
    }
    if input.campaigns.is_empty() {
        return Err(EvaluationError::EmptyField("campaigns"));
    }

    let mut case_ids = BTreeSet::new();
    for case in &input.cases {
        validate_nonempty(&case.case_id, "case.case_id")?;
        if !case_ids.insert(case.case_id.clone()) {
            return Err(EvaluationError::DuplicateCaseId(case.case_id.clone()));
        }
        if case.required_simulated_ticks == 0 {
            return Err(EvaluationError::ZeroValue("case.required_simulated_ticks"));
        }
        validate_positive_finite(case.max_error_upper_bound, "case.max_error_upper_bound")?;
    }

    let mut engine_ids = BTreeSet::new();
    for campaign in &input.campaigns {
        validate_nonempty(&campaign.engine_id, "campaign.engine_id")?;
        validate_nonempty(&campaign.engine_version, "campaign.engine_version")?;
        validate_sha256(&campaign.executable_sha256, "campaign.executable_sha256")?;
        validate_sha256(&campaign.configuration_sha256, "campaign.configuration_sha256")?;
        validate_sha256(
            &campaign.hardware_profile_sha256,
            "campaign.hardware_profile_sha256",
        )?;
        if campaign.hardware_profile_sha256 != input.hardware_profile_sha256 {
            return Err(EvaluationError::HardwareProfileMismatch(
                campaign.engine_id.clone(),
            ));
        }
        if !engine_ids.insert(campaign.engine_id.clone()) {
            return Err(EvaluationError::DuplicateEngineId(campaign.engine_id.clone()));
        }
        if campaign.campaign_wall_time_us == 0 {
            return Err(EvaluationError::ZeroValue("campaign.campaign_wall_time_us"));
        }
        if campaign.peak_memory_bytes == 0 {
            return Err(EvaluationError::ZeroValue("campaign.peak_memory_bytes"));
        }
        validate_nonzero_latencies(campaign)?;
        validate_energy_measurement(input, campaign)?;
        validate_campaign_cases(input, campaign)?;
    }
    Ok(())
}

fn validate_campaign_cases(
    input: &BenchmarkInputV1,
    campaign: &EngineCampaign,
) -> Result<(), EvaluationError> {
    let required = input.cases.iter().map(|case| case.case_id.as_str()).collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for result in &campaign.results {
        validate_nonempty(&result.case_id, "result.case_id")?;
        if !required.contains(result.case_id.as_str()) {
            return Err(EvaluationError::UnexpectedCaseResult(result.case_id.clone()));
        }
        if !seen.insert(result.case_id.as_str()) {
            return Err(EvaluationError::DuplicateResult(result.case_id.clone()));
        }
        let case_requirement = input
            .cases
            .iter()
            .find(|case| case.case_id == result.case_id)
            .expect("required case validated above");
        if result.completed_simulated_ticks > case_requirement.required_simulated_ticks {
            return Err(EvaluationError::SimulatedTicksExceedCaseHorizon(
                result.case_id.clone(),
            ));
        }
        if let Some(error) = result.error_upper_bound {
            validate_nonnegative_finite(error, "result.error_upper_bound")?;
        }
        if let Some(digest) = &result.evidence_receipt_sha256 {
            validate_sha256(digest, "result.evidence_receipt_sha256")?;
        }
    }
    for case in &input.cases {
        if !seen.contains(case.case_id.as_str()) {
            return Err(EvaluationError::MissingCaseResult(case.case_id.clone()));
        }
    }
    Ok(())
}

fn validate_energy_measurement(
    input: &BenchmarkInputV1,
    campaign: &EngineCampaign,
) -> Result<(), EvaluationError> {
    let Some(energy) = &campaign.energy_measurement else {
        return Ok(());
    };
    let Some(max_gap) = input.max_energy_sample_gap_us else {
        return Err(EvaluationError::EnergySamplingPolicyMissing(
            campaign.engine_id.clone(),
        ));
    };
    validate_nonempty(&energy.source_api_version, "energy.source_api_version")?;
    validate_sha256(&energy.sample_series_sha256, "energy.sample_series_sha256")?;
    validate_nonnegative_finite(energy.integrated_energy_uj, "energy.integrated_energy_uj")?;
    if energy.integrated_energy_uj == 0.0
        || energy.measurement_window_us == 0
        || energy.sample_count < 2
        || energy.max_sample_gap_us == 0
    {
        return Err(EvaluationError::ZeroValue("energy measurement coverage"));
    }
    if energy.max_sample_gap_us > max_gap {
        return Err(EvaluationError::EnergySampleGapExceeded(
            campaign.engine_id.clone(),
        ));
    }
    Ok(())
}

fn validate_nonzero_latencies(campaign: &EngineCampaign) -> Result<(), EvaluationError> {
    if campaign.step_latency_p50_us == 0
        || campaign.step_latency_p95_us == 0
        || campaign.step_latency_p99_us == 0
    {
        return Err(EvaluationError::ZeroValue("campaign.step_latency"));
    }
    if campaign.step_latency_p50_us > campaign.step_latency_p95_us
        || campaign.step_latency_p95_us > campaign.step_latency_p99_us
    {
        return Err(EvaluationError::InvalidLatencyOrder(
            campaign.engine_id.clone(),
        ));
    }
    Ok(())
}

fn canonicalize(input: &BenchmarkInputV1) -> BenchmarkInputV1 {
    let mut canonical = input.clone();
    canonical.cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    canonical.campaigns.sort_by(|a, b| a.engine_id.cmp(&b.engine_id));
    for campaign in &mut canonical.campaigns {
        campaign.results.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    }
    canonical
}

fn canonical_sha256(input: &BenchmarkInputV1) -> Result<String, EvaluationError> {
    let bytes = serde_json::to_vec(input)
        .map_err(|error| EvaluationError::Serialization(error.to_string()))?;
    Ok(hex_sha256(&bytes))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn pareto_frontier(reports: &[CampaignReport], include_energy: bool) -> Vec<String> {
    let mut frontier = Vec::new();
    for candidate in reports {
        let dominated = reports.iter().any(|other| {
            if candidate.engine_id == other.engine_id {
                return false;
            }
            dominates(other, candidate, include_energy)
        });
        if !dominated {
            frontier.push(candidate.engine_id.clone());
        }
    }
    frontier.sort();
    frontier
}

fn dominates(a: &CampaignReport, b: &CampaignReport, include_energy: bool) -> bool {
    let mut no_worse = a.qualified_simulated_ticks_per_second
        >= b.qualified_simulated_ticks_per_second
        && a.case_coverage >= b.case_coverage
        && a.step_latency_p95_us <= b.step_latency_p95_us
        && a.step_latency_p99_us <= b.step_latency_p99_us
        && a.peak_memory_bytes <= b.peak_memory_bytes;
    if let (Some(a_error), Some(b_error)) =
        (a.max_qualified_error_ratio, b.max_qualified_error_ratio)
    {
        no_worse &= a_error <= b_error;
    } else {
        return false;
    }
    let mut strictly_better = a.qualified_simulated_ticks_per_second
        > b.qualified_simulated_ticks_per_second
        || a.case_coverage > b.case_coverage
        || a.step_latency_p95_us < b.step_latency_p95_us
        || a.step_latency_p99_us < b.step_latency_p99_us
        || a.peak_memory_bytes < b.peak_memory_bytes
        || a.max_qualified_error_ratio < b.max_qualified_error_ratio;

    if include_energy {
        match (
            a.energy_per_qualified_tick_uj,
            b.energy_per_qualified_tick_uj,
        ) {
            (Some(a_energy), Some(b_energy)) => {
                no_worse &= a_energy <= b_energy;
                strictly_better |= a_energy < b_energy;
            }
            _ => return false,
        }
    }
    no_worse && strictly_better
}

fn energy_is_comparable(campaigns: &[EngineCampaign]) -> bool {
    let Some(first_scope) = campaigns
        .first()
        .and_then(|campaign| campaign.energy_measurement.as_ref().map(|energy| energy.scope))
    else {
        return false;
    };
    campaigns.iter().all(|campaign| {
        campaign
            .energy_measurement
            .as_ref()
            .is_some_and(|energy| energy.scope == first_scope)
    })
}

fn ratio_or_zero(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn validate_nonempty(value: &str, field: &'static str) -> Result<(), EvaluationError> {
    if value.trim().is_empty() {
        Err(EvaluationError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_sha256(value: &str, field: &'static str) -> Result<(), EvaluationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Err(EvaluationError::InvalidSha256(field))
    } else {
        Ok(())
    }
}

fn validate_nonnegative_finite(value: f64, field: &'static str) -> Result<(), EvaluationError> {
    if !value.is_finite() || value < 0.0 {
        Err(EvaluationError::NonFiniteOrNegative(field))
    } else {
        Ok(())
    }
}

fn validate_positive_finite(value: f64, field: &'static str) -> Result<(), EvaluationError> {
    validate_nonnegative_finite(value, field)?;
    if value == 0.0 {
        Err(EvaluationError::ZeroValue(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const H2: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const H3: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const H4: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn case(id: &str) -> CaseRequirement {
        CaseRequirement {
            case_id: id.to_owned(),
            required_simulated_ticks: 100,
            max_error_upper_bound: 0.01,
            max_hard_constraint_violations: 0,
            require_replay_conformance: true,
        }
    }

    fn result(id: &str) -> CaseResult {
        CaseResult {
            case_id: id.to_owned(),
            outcome: CaseOutcome::Completed,
            semantic_equivalence: SemanticEquivalence::Matched,
            completed_simulated_ticks: 100,
            error_upper_bound: Some(0.005),
            hard_constraint_violations: Some(0),
            replay_conformant: Some(true),
            evidence_receipt_sha256: Some(H3.to_owned()),
        }
    }

    fn campaign(id: &str, wall_time: u64, results: Vec<CaseResult>) -> EngineCampaign {
        EngineCampaign {
            engine_id: id.to_owned(),
            engine_version: "test-version".to_owned(),
            executable_sha256: H1.to_owned(),
            configuration_sha256: H2.to_owned(),
            hardware_profile_sha256: H4.to_owned(),
            campaign_wall_time_us: wall_time,
            peak_memory_bytes: 100_000_000,
            step_latency_p50_us: 500,
            step_latency_p95_us: 900,
            step_latency_p99_us: 1_200,
            energy_measurement: None,
            results,
        }
    }

    fn input() -> BenchmarkInputV1 {
        BenchmarkInputV1 {
            schema_version: SCHEMA_VERSION,
            benchmark_id: "qualified-throughput-known-answer".to_owned(),
            suite_sha256: H1.to_owned(),
            comparison_group_id: "same-hardware-debug".to_owned(),
            hardware_profile_sha256: H4.to_owned(),
            workload_family: WorkloadFamily::IndependentEnvironmentBatch,
            max_energy_sample_gap_us: None,
            cases: vec![case("case-a"), case("case-b")],
            campaigns: vec![
                campaign("engine-fast-low-coverage", 2_000, vec![result("case-a"), {
                    let mut invalid = result("case-b");
                    invalid.semantic_equivalence = SemanticEquivalence::Mismatch;
                    invalid
                }]),
                campaign("engine-slower-qualified", 8_000, vec![result("case-a"), result("case-b")]),
            ],
        }
    }

    #[test]
    fn raw_throughput_is_not_confused_with_qualified_throughput() {
        let report = evaluate(&input()).expect("valid campaigns");
        let fast = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").expect("fast");
        let qualified = report.campaigns.iter().find(|c| c.engine_id == "engine-slower-qualified").expect("qualified");
        assert!(fast.raw_simulated_ticks_per_second > qualified.raw_simulated_ticks_per_second);
        assert!(fast.qualified_simulated_ticks_per_second > qualified.qualified_simulated_ticks_per_second);
        assert!(fast.max_qualified_error_ratio.unwrap() == 0.5);
        assert!(qualified.max_qualified_error_ratio.unwrap() == 0.5);
        assert!(fast.case_coverage < qualified.case_coverage);
        assert_eq!(fast.qualified_simulated_ticks, 100);
        assert_eq!(qualified.qualified_simulated_ticks, 200);
        assert_eq!(report.pareto_frontier_engine_ids.len(), 2);
    }

    #[test]
    fn omitted_case_is_rejected_not_removed_from_denominator() {
        let mut data = input();
        data.campaigns[0].results.pop();
        assert_eq!(
            evaluate(&data).unwrap_err(),
            EvaluationError::MissingCaseResult("case-b".to_owned())
        );
    }

    #[test]
    fn timeout_keeps_case_in_requested_denominator() {
        let mut data = input();
        data.campaigns[0].results[1].outcome = CaseOutcome::TimedOut;
        let report = evaluate(&data).expect("timeout is a recorded outcome");
        let campaign = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").expect("fast");
        assert_eq!(campaign.requested_cases, 2);
        assert_eq!(campaign.timed_out_cases, 1);
        assert_eq!(campaign.case_coverage, 0.5);
    }

    #[test]
    fn partial_horizon_produces_raw_but_not_qualified_ticks() {
        let mut data = input();
        data.campaigns[0].results[0].completed_simulated_ticks = 50;
        let report = evaluate(&data).expect("partial result");
        let campaign = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").expect("fast");
        assert_eq!(campaign.raw_simulated_ticks, 50);
        assert_eq!(campaign.qualified_simulated_ticks, 0);
        assert_eq!(
            campaign.evaluations.iter().find(|e| e.case_id == "case-a").expect("case").rejection_reasons,
            vec![RejectionReason::HorizonIncomplete]
        );
    }

    #[test]
    fn semantic_mismatch_has_zero_qualified_work() {
        let mut data = input();
        data.campaigns[0].results[0].semantic_equivalence = SemanticEquivalence::Mismatch;
        let report = evaluate(&data).expect("mismatch must remain visible");
        let campaign = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").expect("fast");
        assert_eq!(campaign.qualified_cases, 0);
    }

    #[test]
    fn missing_numerical_or_invariant_evidence_is_not_a_pass() {
        let mut data = input();
        data.campaigns[0].results[0].error_upper_bound = None;
        data.campaigns[0].results[0].hard_constraint_violations = None;
        let report = evaluate(&data).expect("missing evidence is reportable");
        let item = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").unwrap()
            .evaluations.iter().find(|e| e.case_id == "case-a").unwrap();
        assert!(item.rejection_reasons.contains(&RejectionReason::ErrorEvidenceMissing));
        assert!(item.rejection_reasons.contains(&RejectionReason::HardConstraintEvidenceMissing));
    }

    #[test]
    fn error_above_case_bound_is_not_qualified() {
        let mut data = input();
        data.campaigns[0].results[0].error_upper_bound = Some(0.02);
        let report = evaluate(&data).expect("over-bound result");
        let item = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").unwrap()
            .evaluations.iter().find(|e| e.case_id == "case-a").unwrap();
        assert!(item.rejection_reasons.contains(&RejectionReason::ErrorBoundExceeded));
    }

    #[test]
    fn replay_nonconformance_is_not_qualified_when_required() {
        let mut data = input();
        data.campaigns[0].results[0].replay_conformant = Some(false);
        let report = evaluate(&data).expect("replay mismatch");
        let item = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").unwrap()
            .evaluations.iter().find(|e| e.case_id == "case-a").unwrap();
        assert!(item.rejection_reasons.contains(&RejectionReason::ReplayNonConformant));
    }

    #[test]
    fn campaign_order_and_case_order_do_not_change_report() {
        let mut forward = input();
        let mut reverse = forward.clone();
        reverse.campaigns.reverse();
        reverse.cases.reverse();
        for campaign in &mut reverse.campaigns {
            campaign.results.reverse();
        }
        let a = evaluate(&forward).expect("forward");
        let b = evaluate(&reverse).expect("reverse");
        assert_eq!(a.input_sha256, b.input_sha256);
        assert_eq!(a.pareto_frontier_engine_ids, b.pareto_frontier_engine_ids);
        assert_eq!(a.campaigns, b.campaigns);
    }

    #[test]
    fn energy_is_excluded_when_not_measured_for_all_campaigns() {
        let mut data = input();
        data.max_energy_sample_gap_us = Some(1_000);
        data.campaigns[0].energy_measurement = Some(EnergyMeasurement {
            scope: EnergyScope::GpuBoard,
            source: EnergySource::NvmlPowerSamples,
            source_api_version: "nvml".to_owned(),
            sample_series_sha256: H3.to_owned(),
            integrated_energy_uj: 10_000.0,
            measurement_window_us: 10_000,
            sample_count: 4,
            max_sample_gap_us: 500,
        });
        let report = evaluate(&data).expect("mixed energy availability");
        assert!(!report.energy_included_in_frontier);
    }

    #[test]
    fn energy_per_qualified_tick_is_derived_from_campaign_total() {
        let mut data = input();
        data.max_energy_sample_gap_us = Some(1_000);
        for campaign in &mut data.campaigns {
            campaign.energy_measurement = Some(EnergyMeasurement {
                scope: EnergyScope::GpuBoard,
                source: EnergySource::NvmlPowerSamples,
                source_api_version: "nvml".to_owned(),
                sample_series_sha256: H3.to_owned(),
                integrated_energy_uj: 20_000.0,
                measurement_window_us: 10_000,
                sample_count: 4,
                max_sample_gap_us: 500,
            });
        }
        let report = evaluate(&data).expect("energy measured for both campaigns");
        assert!(report.energy_included_in_frontier);
        let slower = report.campaigns.iter().find(|c| c.engine_id == "engine-slower-qualified").unwrap();
        assert_eq!(slower.energy_per_qualified_tick_uj, Some(100.0));
    }

    #[test]
    fn duplicate_case_result_is_rejected() {
        let mut data = input();
        data.campaigns[0].results[1].case_id = "case-a".to_owned();
        assert_eq!(
            evaluate(&data).unwrap_err(),
            EvaluationError::DuplicateResult("case-a".to_owned())
        );
    }

    #[test]
    fn failed_abstained_and_not_run_cases_remain_distinct() {
        let mut data = input();
        let results = &mut data.campaigns[0].results;
        results[0].outcome = CaseOutcome::Failed;
        results[1].outcome = CaseOutcome::Abstained;
        let mut third = result("case-c");
        third.outcome = CaseOutcome::NotRun;
        data.cases.push(case("case-c"));
        for campaign in &mut data.campaigns {
            if campaign.engine_id == "engine-fast-low-coverage" {
                campaign.results.push(third.clone());
            } else {
                campaign.results.push(result("case-c"));
            }
        }
        let report = evaluate(&data).expect("all statuses have explicit outcomes");
        let fast = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").unwrap();
        assert_eq!(fast.failed_cases, 1);
        assert_eq!(fast.abstained_cases, 1);
        assert_eq!(fast.not_run_cases, 1);
        assert_eq!(fast.requested_cases, 3);
    }

    #[test]
    fn hard_constraint_violation_is_not_qualified() {
        let mut data = input();
        data.campaigns[0].results[0].hard_constraint_violations = Some(1);
        let report = evaluate(&data).expect("reported invariant violation");
        let item = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").unwrap()
            .evaluations.iter().find(|e| e.case_id == "case-a").unwrap();
        assert!(item.rejection_reasons.contains(&RejectionReason::HardConstraintExceeded));
    }

    #[test]
    fn completed_run_cannot_claim_more_ticks_than_case_horizon() {
        let mut data = input();
        data.campaigns[0].results[0].completed_simulated_ticks = 101;
        assert_eq!(
            evaluate(&data).unwrap_err(),
            EvaluationError::SimulatedTicksExceedCaseHorizon("case-a".to_owned())
        );
    }

    #[test]
    fn invalid_energy_gap_does_not_enter_comparison() {
        let mut data = input();
        data.max_energy_sample_gap_us = Some(100);
        data.campaigns[0].energy_measurement = Some(EnergyMeasurement {
            scope: EnergyScope::GpuBoard,
            source: EnergySource::NvmlPowerSamples,
            source_api_version: "nvml".to_owned(),
            sample_series_sha256: H3.to_owned(),
            integrated_energy_uj: 10_000.0,
            measurement_window_us: 10_000,
            sample_count: 4,
            max_sample_gap_us: 500,
        });
        assert_eq!(
            evaluate(&data).unwrap_err(),
            EvaluationError::EnergySampleGapExceeded("engine-fast-low-coverage".to_owned())
        );
    }

    #[test]
    fn hardware_mismatch_is_rejected_before_comparison() {
        let mut data = input();
        data.campaigns[0].hardware_profile_sha256 = H3.to_owned();
        assert_eq!(
            evaluate(&data).unwrap_err(),
            EvaluationError::HardwareProfileMismatch("engine-fast-low-coverage".to_owned())
        );
    }

    #[test]
    fn malformed_evidence_digest_is_rejected_at_the_campaign_boundary() {
        let mut data = input();
        data.campaigns[0].results[0].evidence_receipt_sha256 = Some("not-a-digest".to_owned());
        assert_eq!(
            evaluate(&data).unwrap_err(),
            EvaluationError::InvalidSha256("result.evidence_receipt_sha256")
        );
    }

    #[test]
    fn zero_qualified_work_has_zero_throughput_not_nan() {
        let mut data = input();
        for result in &mut data.campaigns[0].results {
            result.semantic_equivalence = SemanticEquivalence::Unknown;
        }
        let report = evaluate(&data).expect("unknown semantics are reportable");
        let fast = report.campaigns.iter().find(|c| c.engine_id == "engine-fast-low-coverage").unwrap();
        assert_eq!(fast.qualified_simulated_ticks_per_second, 0.0);
        assert_eq!(fast.case_coverage, 0.0);
    }
}
