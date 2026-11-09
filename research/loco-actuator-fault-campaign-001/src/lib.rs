//! Fault-adaptation benchmark identity, coverage and trial-evidence auditor.
//!
//! Consumes externally classified fault trials and independent viability/event
//! receipts. It does not diagnose raw telemetry or certify physical safety.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_SCENARIOS: usize = 20_000;
pub const MAX_POLICIES: usize = 64;
pub const MAX_PAIRED_BLOCKS: usize = 10_000;
pub const MAX_TRIALS: usize = 2_000_000;
pub const PPM_SCALE: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultFamily {
    TorqueDerating,
    CompletePowerLoss,
    JointLocked,
    FreeSwinging,
    IntermittentActuation,
    ThermalDerating,
    EncoderDrift,
    ImuDelay,
    ContactSensorDropout,
    CommandDelay,
    CommonPowerRail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultPhase {
    Stance,
    Swing,
    PushOff,
    ContactTransition,
    Flight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioSplit {
    Training,
    Validation,
    HeldOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultOnsetProfile {
    Abrupt,
    Ramped,
    Intermittent,
    Persistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    Simulation,
    SupportedHardware,
    TetheredHardware,
    UntetheredHardware,
    Biological,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultScenario {
    pub scenario_id: String,
    pub split: ScenarioSplit,
    pub fault_family: FaultFamily,
    pub affected_component_ids: Vec<String>,
    pub severity_lower_ppm: u32,
    pub severity_upper_ppm: u32,
    pub onset_phase: FaultPhase,
    pub onset_profile: FaultOnsetProfile,
    pub initial_state_digest: String,
    pub payload_digest: String,
    pub fault_profile_digest: String,
    pub task_digest: String,
    pub command_schedule_digest: String,
    pub terrain_stratum_digest: String,
    pub morphology_digest: String,
    pub actuator_model_digest: String,
    pub sensor_model_digest: String,
    pub physics_digest: String,
    pub fault_injection_digest: String,
    pub detection_deadline_ms: u64,
    pub adaptation_deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedBenchmarkContext {
    pub morphology_digest: String,
    pub actuator_model_digest: String,
    pub actuator_response_digest: String,
    pub sensor_model_digest: String,
    pub terrain_suite_digest: String,
    pub task_digest: String,
    pub command_schedule_digest: String,
    pub physics_digest: String,
    pub event_taxonomy_digest: String,
    pub event_detector_digest: String,
    pub energy_boundary_digest: String,
    pub interaction_budget_steps: u64,
    pub compute_budget_micros: u64,
    pub test_exposure_units: u64,
    pub test_exposure_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDescriptor {
    pub policy_id: String,
    pub architecture_digest: String,
    pub context: MatchedBenchmarkContext,
    /// Privileged fault labels may be available to a training critic only when
    /// the method declares this. Deployment must not receive ground-truth fault
    /// identity unless explicitly being scored as an oracle upper bound.
    pub training_fault_truth_visible: bool,
    pub deployment_fault_truth_visible: bool,
    pub oracle_upper_bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkManifest {
    pub manifest_digest: String,
    pub evidence_tier: EvidenceTier,
    pub baseline_policy_id: String,
    pub policies: Vec<PolicyDescriptor>,
    pub training_scenario_ids: Vec<String>,
    pub validation_scenario_ids: Vec<String>,
    pub heldout_scenarios: Vec<FaultScenario>,
    pub paired_block_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionResult {
    Detected {
        inferred_family: FaultFamily,
        plausible_families: Vec<FaultFamily>,
        latency_ms: u64,
        confidence_ppm: u32,
        severity_estimate_ppm: Option<u32>,
        severity_interval_ppm: Option<(u32, u32)>,
    },
    Unknown {
        reason: String,
        plausible_families: Vec<FaultFamily>,
    },
    Missed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdaptationResult {
    Stable { latency_ms: u64 },
    Unstable,
    Abstained { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOutcome {
    TaskCompleted,
    ControlledStop,
    FallWithRecovery,
    UnrecoveredFall,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultTrial {
    pub trial_id: String,
    pub policy_id: String,
    pub scenario_id: String,
    pub paired_block_id: String,
    pub realized_severity_ppm: u32,
    pub fault_injection_receipt_digest: String,
    pub run_trace_digest: String,
    pub detected: DetectionResult,
    pub adaptation: AdaptationResult,
    pub terminal_outcome: TerminalOutcome,
    pub recovery_succeeded: bool,
    pub recovery_latency_ms: Option<u64>,
    pub task_progress_ppm: u32,
    pub energy_mj: u64,
    pub event_vector_receipt_digest: String,
    pub viability_receipt_digest: String,
    pub action_applied: bool,
    pub certificate_current_at_action: bool,
    pub applied_action_admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultBenchmarkProblem {
    pub manifest: BenchmarkManifest,
    pub trials: Vec<FaultTrial>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyCoverage {
    pub family: FaultFamily,
    pub scenario_count: usize,
    pub matched_trial_count: usize,
    pub detection_count: usize,
    pub unknown_diagnosis_count: usize,
    pub missed_diagnosis_count: usize,
    pub misclassified_count: usize,
    pub fault_set_covered_count: usize,
    pub fault_set_uncovered_count: usize,
    pub hypothesis_set_size_total: usize,
    pub severity_estimate_count: usize,
    pub severity_absolute_error_sum_ppm: u64,
    pub severity_interval_coverage_count: usize,
    pub late_detection_count: usize,
    pub stable_adaptation_count: usize,
    pub late_adaptation_count: usize,
    pub controlled_stop_count: usize,
    pub fall_count: usize,
    pub unrecovered_fall_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyCoverage {
    pub policy_id: String,
    pub training_fault_truth_visible: bool,
    pub oracle_upper_bound: bool,
    pub trial_count: usize,
    pub task_completion_count: usize,
    pub controlled_stop_count: usize,
    pub fall_count: usize,
    pub unrecovered_fall_count: usize,
    pub unknown_outcome_count: usize,
    pub recovery_success_count: usize,
    pub stale_certificate_count: usize,
    pub unadmitted_action_count: usize,
    pub action_abstention_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultBenchmarkReport {
    pub manifest_digest: String,
    pub evidence_tier: EvidenceTier,
    pub baseline_policy_id: String,
    pub matched_policy_ids: Vec<String>,
    pub heldout_scenario_count: usize,
    pub paired_block_count: usize,
    pub matched_trial_count: usize,
    pub family_coverage: Vec<FamilyCoverage>,
    pub policy_coverage: Vec<PolicyCoverage>,
    // The report intentionally preserves no single aggregate fault-tolerance
    // score. Per-trial records remain the source of detailed distributions.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultBenchmarkError {
    EmptyIdentifier,
    NoPolicies,
    TooManyPolicies,
    TooManyScenarios,
    TooManyBlocks,
    TooManyTrials,
    DuplicatePolicy,
    DuplicateScenario,
    DuplicateComponent,
    EmptyAffectedComponentSet,
    InvalidSeverityRange,
    InvalidPpmValue,
    InvalidSeverityEstimate,
    InvalidSeverityInterval,
    InvalidDeadline,
    InvalidMatchedContext,
    PolicyContextMismatch,
    UnknownBaselinePolicy,
    NoCandidatePolicy,
    TrainingValidationLeakage,
    TrainingHeldoutLeakage,
    ValidationHeldoutLeakage,
    DuplicatePairedBlock,
    NoPairedBlocks,
    NoHeldoutScenarios,
    NoTrials,
    UnknownTrialPolicy,
    UnknownTrialScenario,
    UnknownTrialBlock,
    DuplicateTrial,
    MissingMatchedTrial,
    TrialIdentityInvalid,
    EmptyReceiptDigest,
    StaleCertificateActionApplied,
    UnadmittedActionApplied,
    InvalidDetectionResult,
    InvalidAdaptationResult,
    InvalidRecoveryResult,
    ScenarioContextMismatch,
    TrialOutsideHeldoutSet,
    OraclePolicyTruthMismatch,
    ArithmeticOverflow,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_family_set(families: &[FaultFamily]) -> Result<(), FaultBenchmarkError> {
    let mut unique = BTreeSet::new();
    for family in families {
        if !unique.insert(*family) {
            return Err(FaultBenchmarkError::InvalidDetectionResult);
        }
    }
    Ok(())
}

fn validate_unique_ids<'a, I>(values: I) -> Result<BTreeSet<&'a str>, FaultBenchmarkError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut ids = BTreeSet::new();
    for value in values {
        if !nonempty(value) {
            return Err(FaultBenchmarkError::EmptyIdentifier);
        }
        if !ids.insert(value) {
            return Err(FaultBenchmarkError::DuplicateScenario);
        }
    }
    Ok(ids)
}

fn validate_context(context: &MatchedBenchmarkContext) -> Result<(), FaultBenchmarkError> {
    let ids = [
        context.morphology_digest.as_str(),
        context.actuator_model_digest.as_str(),
        context.actuator_response_digest.as_str(),
        context.sensor_model_digest.as_str(),
        context.terrain_suite_digest.as_str(),
        context.task_digest.as_str(),
        context.command_schedule_digest.as_str(),
        context.physics_digest.as_str(),
        context.event_taxonomy_digest.as_str(),
        context.event_detector_digest.as_str(),
        context.energy_boundary_digest.as_str(),
        context.test_exposure_unit_id.as_str(),
    ];
    if ids.iter().any(|id| !nonempty(id)) {
        return Err(FaultBenchmarkError::InvalidMatchedContext);
    }
    if context.interaction_budget_steps == 0
        || context.compute_budget_micros == 0
        || context.test_exposure_units == 0
    {
        return Err(FaultBenchmarkError::InvalidMatchedContext);
    }
    Ok(())
}

fn validate_scenario(
    scenario: &FaultScenario,
    context: &MatchedBenchmarkContext,
) -> Result<(), FaultBenchmarkError> {
    let ids = [
        scenario.scenario_id.as_str(),
        scenario.initial_state_digest.as_str(),
        scenario.payload_digest.as_str(),
        scenario.fault_profile_digest.as_str(),
        scenario.task_digest.as_str(),
        scenario.command_schedule_digest.as_str(),
        scenario.terrain_stratum_digest.as_str(),
        scenario.morphology_digest.as_str(),
        scenario.actuator_model_digest.as_str(),
        scenario.sensor_model_digest.as_str(),
        scenario.physics_digest.as_str(),
        scenario.fault_injection_digest.as_str(),
    ];
    if ids.iter().any(|id| !nonempty(id)) {
        return Err(FaultBenchmarkError::EmptyIdentifier);
    }
    if scenario.affected_component_ids.is_empty() {
        return Err(FaultBenchmarkError::EmptyAffectedComponentSet);
    }
    validate_unique_ids(scenario.affected_component_ids.iter().map(String::as_str))
        .map_err(|_| FaultBenchmarkError::DuplicateComponent)?;
    if scenario.severity_lower_ppm > scenario.severity_upper_ppm
        || scenario.severity_upper_ppm > PPM_SCALE
    {
        return Err(FaultBenchmarkError::InvalidSeverityRange);
    }
    if scenario.detection_deadline_ms == 0 || scenario.adaptation_deadline_ms == 0 {
        return Err(FaultBenchmarkError::InvalidDeadline);
    }
    if scenario.morphology_digest != context.morphology_digest
        || scenario.actuator_model_digest != context.actuator_model_digest
        || scenario.sensor_model_digest != context.sensor_model_digest
        || scenario.physics_digest != context.physics_digest
        || scenario.task_digest != context.task_digest
        || scenario.command_schedule_digest != context.command_schedule_digest
    {
        return Err(FaultBenchmarkError::ScenarioContextMismatch);
    }
    Ok(())
}

fn policy_context_matches(left: &MatchedBenchmarkContext, right: &MatchedBenchmarkContext) -> bool {
    left == right
}

fn validate_trial(
    trial: &FaultTrial,
    policy: &PolicyDescriptor,
    scenario: &FaultScenario,
    block_ids: &BTreeSet<&str>,
) -> Result<(), FaultBenchmarkError> {
    if !nonempty(&trial.trial_id)
        || !nonempty(&trial.policy_id)
        || !nonempty(&trial.scenario_id)
        || !nonempty(&trial.paired_block_id)
    {
        return Err(FaultBenchmarkError::TrialIdentityInvalid);
    }
    if trial.policy_id != policy.policy_id || trial.scenario_id != scenario.scenario_id {
        return Err(FaultBenchmarkError::TrialIdentityInvalid);
    }
    if !block_ids.contains(trial.paired_block_id.as_str()) {
        return Err(FaultBenchmarkError::UnknownTrialBlock);
    }
    if trial.task_progress_ppm > PPM_SCALE || trial.realized_severity_ppm > PPM_SCALE {
        return Err(FaultBenchmarkError::InvalidPpmValue);
    }
    if trial.realized_severity_ppm < scenario.severity_lower_ppm
        || trial.realized_severity_ppm > scenario.severity_upper_ppm
    {
        return Err(FaultBenchmarkError::InvalidSeverityRange);
    }
    if !nonempty(&trial.fault_injection_receipt_digest)
        || !nonempty(&trial.run_trace_digest)
        || !nonempty(&trial.event_vector_receipt_digest)
        || !nonempty(&trial.viability_receipt_digest)
    {
        return Err(FaultBenchmarkError::EmptyReceiptDigest);
    }
    if trial.action_applied && !trial.certificate_current_at_action {
        return Err(FaultBenchmarkError::StaleCertificateActionApplied);
    }
    if trial.action_applied && !trial.applied_action_admitted {
        return Err(FaultBenchmarkError::UnadmittedActionApplied);
    }

    match &trial.detected {
        DetectionResult::Detected {
            inferred_family,
            plausible_families,
            confidence_ppm,
            severity_estimate_ppm,
            severity_interval_ppm,
            ..
        } => {
            if *confidence_ppm > PPM_SCALE {
                return Err(FaultBenchmarkError::InvalidPpmValue);
            }
            validate_family_set(plausible_families)?;
            if !plausible_families.contains(inferred_family) {
                return Err(FaultBenchmarkError::InvalidDetectionResult);
            }
            if severity_estimate_ppm.is_some_and(|estimate| estimate > PPM_SCALE) {
                return Err(FaultBenchmarkError::InvalidSeverityEstimate);
            }
            if let Some((lower, upper)) = severity_interval_ppm {
                if lower > upper
                    || *upper > PPM_SCALE
                    || severity_estimate_ppm.is_some_and(|estimate| estimate < *lower || estimate > *upper)
                {
                    return Err(FaultBenchmarkError::InvalidSeverityInterval);
                }
            }
        }
        DetectionResult::Unknown {
            reason,
            plausible_families,
        } => {
            if !nonempty(reason) {
                return Err(FaultBenchmarkError::InvalidDetectionResult);
            }
            validate_family_set(plausible_families)?;
        }
        DetectionResult::Missed => {}
    }
    match &trial.adaptation {
        AdaptationResult::Abstained { reason } if !nonempty(reason) => {
            return Err(FaultBenchmarkError::InvalidAdaptationResult);
        }
        _ => {}
    }

    if trial.recovery_succeeded && trial.recovery_latency_ms.is_none() {
        return Err(FaultBenchmarkError::InvalidRecoveryResult);
    }
    if let Some(latency) = trial.recovery_latency_ms {
        if latency == 0 || !trial.recovery_succeeded {
            return Err(FaultBenchmarkError::InvalidRecoveryResult);
        }
    }
    if matches!(
        trial.terminal_outcome,
        TerminalOutcome::ControlledStop | TerminalOutcome::FallWithRecovery
    ) && !trial.recovery_succeeded
    {
        return Err(FaultBenchmarkError::InvalidRecoveryResult);
    }
    if trial.terminal_outcome == TerminalOutcome::UnrecoveredFall && trial.recovery_succeeded {
        return Err(FaultBenchmarkError::InvalidRecoveryResult);
    }

    let _ = policy;
    Ok(())
}

pub fn audit_fault_benchmark(
    problem: &FaultBenchmarkProblem,
) -> Result<FaultBenchmarkReport, FaultBenchmarkError> {
    let manifest = &problem.manifest;
    if !nonempty(&manifest.manifest_digest) {
        return Err(FaultBenchmarkError::EmptyIdentifier);
    }
    if manifest.policies.is_empty() {
        return Err(FaultBenchmarkError::NoPolicies);
    }
    if manifest.policies.len() > MAX_POLICIES {
        return Err(FaultBenchmarkError::TooManyPolicies);
    }
    if manifest.heldout_scenarios.is_empty() {
        return Err(FaultBenchmarkError::NoHeldoutScenarios);
    }
    if manifest.heldout_scenarios.len() > MAX_SCENARIOS {
        return Err(FaultBenchmarkError::TooManyScenarios);
    }
    if manifest.paired_block_ids.is_empty() {
        return Err(FaultBenchmarkError::NoPairedBlocks);
    }
    if manifest.paired_block_ids.len() > MAX_PAIRED_BLOCKS {
        return Err(FaultBenchmarkError::TooManyBlocks);
    }
    if problem.trials.is_empty() {
        return Err(FaultBenchmarkError::NoTrials);
    }
    if problem.trials.len() > MAX_TRIALS {
        return Err(FaultBenchmarkError::TooManyTrials);
    }

    let training_ids =
        validate_unique_ids(manifest.training_scenario_ids.iter().map(String::as_str))?;
    let validation_ids = if manifest.validation_scenario_ids.is_empty() {
        BTreeSet::new()
    } else {
        validate_unique_ids(manifest.validation_scenario_ids.iter().map(String::as_str))?
    };
    let heldout_ids = validate_unique_ids(
        manifest
            .heldout_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.as_str()),
    )?;
    if !training_ids.is_disjoint(&validation_ids) {
        return Err(FaultBenchmarkError::TrainingValidationLeakage);
    }
    if !training_ids.is_disjoint(&heldout_ids) {
        return Err(FaultBenchmarkError::TrainingHeldoutLeakage);
    }
    if !validation_ids.is_disjoint(&heldout_ids) {
        return Err(FaultBenchmarkError::ValidationHeldoutLeakage);
    }

    let block_ids = validate_unique_ids(manifest.paired_block_ids.iter().map(String::as_str))
        .map_err(|_| FaultBenchmarkError::DuplicatePairedBlock)?;

    let mut policies = BTreeMap::new();
    for policy in &manifest.policies {
        if !nonempty(&policy.policy_id) || !nonempty(&policy.architecture_digest) {
            return Err(FaultBenchmarkError::EmptyIdentifier);
        }
        validate_context(&policy.context)?;
        if policy.deployment_fault_truth_visible && !policy.oracle_upper_bound {
            return Err(FaultBenchmarkError::OraclePolicyTruthMismatch);
        }
        if policies.insert(policy.policy_id.as_str(), policy).is_some() {
            return Err(FaultBenchmarkError::DuplicatePolicy);
        }
    }
    let baseline = policies
        .get(manifest.baseline_policy_id.as_str())
        .ok_or(FaultBenchmarkError::UnknownBaselinePolicy)?;
    if policies.len() < 2 {
        return Err(FaultBenchmarkError::NoCandidatePolicy);
    }
    for policy in manifest
        .policies
        .iter()
        .filter(|item| item.policy_id != manifest.baseline_policy_id)
    {
        if !policy_context_matches(&baseline.context, &policy.context) {
            return Err(FaultBenchmarkError::PolicyContextMismatch);
        }
    }

    let mut scenarios = BTreeMap::new();
    let mut family_scenario_counts = BTreeMap::<FaultFamily, usize>::new();
    for scenario in &manifest.heldout_scenarios {
        if scenario.split != ScenarioSplit::HeldOut {
            return Err(FaultBenchmarkError::TrialOutsideHeldoutSet);
        }
        validate_scenario(scenario, &baseline.context)?;
        *family_scenario_counts
            .entry(scenario.fault_family)
            .or_default() += 1;
        if scenarios
            .insert(scenario.scenario_id.as_str(), scenario)
            .is_some()
        {
            return Err(FaultBenchmarkError::DuplicateScenario);
        }
    }

    let expected_trials = policies
        .len()
        .checked_mul(scenarios.len())
        .and_then(|count| count.checked_mul(block_ids.len()))
        .ok_or(FaultBenchmarkError::ArithmeticOverflow)?;
    if expected_trials > MAX_TRIALS {
        return Err(FaultBenchmarkError::TooManyTrials);
    }

    let mut observed = BTreeMap::new();
    for trial in &problem.trials {
        let policy = policies
            .get(trial.policy_id.as_str())
            .ok_or(FaultBenchmarkError::UnknownTrialPolicy)?;
        let scenario = scenarios
            .get(trial.scenario_id.as_str())
            .ok_or(FaultBenchmarkError::UnknownTrialScenario)?;
        validate_trial(trial, policy, scenario, &block_ids)?;
        let key = (
            trial.policy_id.as_str(),
            trial.scenario_id.as_str(),
            trial.paired_block_id.as_str(),
        );
        if observed.insert(key, trial).is_some() {
            return Err(FaultBenchmarkError::DuplicateTrial);
        }
    }
    if observed.len() != expected_trials {
        return Err(FaultBenchmarkError::MissingMatchedTrial);
    }

    let mut family_rows = Vec::with_capacity(family_scenario_counts.len());
    for (family, scenario_count) in family_scenario_counts {
        let family_scenarios: BTreeSet<&str> = scenarios
            .values()
            .filter(|scenario| scenario.fault_family == family)
            .map(|scenario| scenario.scenario_id.as_str())
            .collect();
        let mut row = FamilyCoverage {
            family,
            scenario_count,
            matched_trial_count: 0,
            detection_count: 0,
            unknown_diagnosis_count: 0,
            missed_diagnosis_count: 0,
            misclassified_count: 0,
            fault_set_covered_count: 0,
            fault_set_uncovered_count: 0,
            hypothesis_set_size_total: 0,
            severity_estimate_count: 0,
            severity_absolute_error_sum_ppm: 0,
            severity_interval_coverage_count: 0,
            late_detection_count: 0,
            stable_adaptation_count: 0,
            late_adaptation_count: 0,
            controlled_stop_count: 0,
            fall_count: 0,
            unrecovered_fall_count: 0,
        };
        for scenario_id in &family_scenarios {
            let scenario = scenarios
                .get(scenario_id)
                .ok_or(FaultBenchmarkError::UnknownTrialScenario)?;
            for policy in &manifest.policies {
                for block_id in &block_ids {
                    let trial = observed
                        .get(&(policy.policy_id.as_str(), *scenario_id, *block_id))
                        .ok_or(FaultBenchmarkError::MissingMatchedTrial)?;
                    row.matched_trial_count += 1;
                    match &trial.detected {
                        DetectionResult::Detected {
                            inferred_family,
                            plausible_families,
                            latency_ms,
                            severity_estimate_ppm,
                            severity_interval_ppm,
                            ..
                        } => {
                            row.detection_count += 1;
                            if *inferred_family != scenario.fault_family {
                                row.misclassified_count += 1;
                            }
                            if plausible_families.contains(&scenario.fault_family) {
                                row.fault_set_covered_count += 1;
                            } else {
                                row.fault_set_uncovered_count += 1;
                            }
                            row.hypothesis_set_size_total += plausible_families.len();
                            if *latency_ms > scenario.detection_deadline_ms {
                                row.late_detection_count += 1;
                            }
                            if let Some(estimate) = severity_estimate_ppm {
                                row.severity_estimate_count += 1;
                                row.severity_absolute_error_sum_ppm = row
                                    .severity_absolute_error_sum_ppm
                                    .checked_add(estimate.abs_diff(trial.realized_severity_ppm))
                                    .ok_or(FaultBenchmarkError::ArithmeticOverflow)?;
                            }
                            if let Some((lower, upper)) = severity_interval_ppm {
                                if *lower <= trial.realized_severity_ppm
                                    && trial.realized_severity_ppm <= *upper
                                {
                                    row.severity_interval_coverage_count += 1;
                                }
                            }
                        }
                        DetectionResult::Unknown {
                            plausible_families,
                            ..
                        } => {
                            row.unknown_diagnosis_count += 1;
                            if plausible_families.contains(&scenario.fault_family) {
                                row.fault_set_covered_count += 1;
                            } else {
                                row.fault_set_uncovered_count += 1;
                            }
                            row.hypothesis_set_size_total += plausible_families.len();
                        }
                        DetectionResult::Missed => {
                            row.missed_diagnosis_count += 1;
                            row.fault_set_uncovered_count += 1;
                        }
                    }
                    if let AdaptationResult::Stable { latency_ms } = &trial.adaptation {
                        row.stable_adaptation_count += 1;
                        if *latency_ms > scenario.adaptation_deadline_ms {
                            row.late_adaptation_count += 1;
                        }
                    }
                    match trial.terminal_outcome {
                        TerminalOutcome::ControlledStop => row.controlled_stop_count += 1,
                        TerminalOutcome::FallWithRecovery => row.fall_count += 1,
                        TerminalOutcome::UnrecoveredFall => {
                            row.fall_count += 1;
                            row.unrecovered_fall_count += 1;
                        }
                        TerminalOutcome::TaskCompleted | TerminalOutcome::Unknown => {}
                    }
                }
            }
        }
        family_rows.push(row);
    }
    family_rows.sort_by_key(|row| row.family);

    let mut policy_rows = Vec::with_capacity(manifest.policies.len());
    for policy in &manifest.policies {
        let mut row = PolicyCoverage {
            policy_id: policy.policy_id.clone(),
            training_fault_truth_visible: policy.training_fault_truth_visible,
            oracle_upper_bound: policy.oracle_upper_bound,
            trial_count: 0,
            task_completion_count: 0,
            controlled_stop_count: 0,
            fall_count: 0,
            unrecovered_fall_count: 0,
            unknown_outcome_count: 0,
            recovery_success_count: 0,
            stale_certificate_count: 0,
            unadmitted_action_count: 0,
            action_abstention_count: 0,
        };
        for scenario_id in scenarios.keys() {
            for block_id in &block_ids {
                let trial = observed
                    .get(&(policy.policy_id.as_str(), *scenario_id, *block_id))
                    .ok_or(FaultBenchmarkError::MissingMatchedTrial)?;
                row.trial_count += 1;
                match trial.terminal_outcome {
                    TerminalOutcome::TaskCompleted => row.task_completion_count += 1,
                    TerminalOutcome::ControlledStop => row.controlled_stop_count += 1,
                    TerminalOutcome::FallWithRecovery => row.fall_count += 1,
                    TerminalOutcome::UnrecoveredFall => {
                        row.fall_count += 1;
                        row.unrecovered_fall_count += 1;
                    }
                    TerminalOutcome::Unknown => row.unknown_outcome_count += 1,
                }
                if trial.recovery_succeeded {
                    row.recovery_success_count += 1;
                }
                if trial.action_applied && !trial.certificate_current_at_action {
                    row.stale_certificate_count += 1;
                }
                if trial.action_applied && !trial.applied_action_admitted {
                    row.unadmitted_action_count += 1;
                }
                if !trial.action_applied {
                    row.action_abstention_count += 1;
                }
            }
        }
        policy_rows.push(row);
    }
    policy_rows.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));

    let matched_policy_ids = policies.keys().map(|id| (*id).to_owned()).collect();
    Ok(FaultBenchmarkReport {
        manifest_digest: manifest.manifest_digest.clone(),
        evidence_tier: manifest.evidence_tier,
        baseline_policy_id: manifest.baseline_policy_id.clone(),
        matched_policy_ids,
        heldout_scenario_count: scenarios.len(),
        paired_block_count: block_ids.len(),
        matched_trial_count: observed.len(),
        family_coverage: family_rows,
        policy_coverage: policy_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> MatchedBenchmarkContext {
        MatchedBenchmarkContext {
            morphology_digest: "morphology-v1".to_owned(),
            actuator_model_digest: "actuator-v1".to_owned(),
            actuator_response_digest: "actuator-response-v1".to_owned(),
            sensor_model_digest: "sensors-v1".to_owned(),
            terrain_suite_digest: "terrain-v1".to_owned(),
            task_digest: "task-v1".to_owned(),
            command_schedule_digest: "commands-v1".to_owned(),
            physics_digest: "physics-v1".to_owned(),
            event_taxonomy_digest: "taxonomy-v1".to_owned(),
            event_detector_digest: "detector-v1".to_owned(),
            energy_boundary_digest: "energy-boundary-v1".to_owned(),
            interaction_budget_steps: 1_000_000,
            compute_budget_micros: 10_000_000,
            test_exposure_units: 100,
            test_exposure_unit_id: "fault_trials".to_owned(),
        }
    }

    fn policy(id: &str, oracle: bool, deployment_truth: bool) -> PolicyDescriptor {
        PolicyDescriptor {
            policy_id: id.to_owned(),
            architecture_digest: format!("architecture-{id}"),
            context: context(),
            training_fault_truth_visible: true,
            deployment_fault_truth_visible: deployment_truth,
            oracle_upper_bound: oracle,
        }
    }

    fn scenario(id: &str, family: FaultFamily) -> FaultScenario {
        FaultScenario {
            scenario_id: id.to_owned(),
            split: ScenarioSplit::HeldOut,
            fault_family: family,
            affected_component_ids: vec!["hip-left".to_owned()],
            severity_lower_ppm: 250_000,
            severity_upper_ppm: 750_000,
            onset_phase: FaultPhase::Stance,
            onset_profile: FaultOnsetProfile::Abrupt,
            initial_state_digest: "initial-state-v1".to_owned(),
            payload_digest: "payload-v1".to_owned(),
            fault_profile_digest: format!("profile-{id}"),
            task_digest: "task-v1".to_owned(),
            command_schedule_digest: "commands-v1".to_owned(),
            terrain_stratum_digest: "terrain-rough-v1".to_owned(),
            morphology_digest: "morphology-v1".to_owned(),
            actuator_model_digest: "actuator-v1".to_owned(),
            sensor_model_digest: "sensors-v1".to_owned(),
            physics_digest: "physics-v1".to_owned(),
            fault_injection_digest: format!("fault-inject-{id}"),
            detection_deadline_ms: 20,
            adaptation_deadline_ms: 100,
        }
    }

    fn trial(
        policy_id: &str,
        scenario_id: &str,
        block_id: &str,
        family: FaultFamily,
    ) -> FaultTrial {
        FaultTrial {
            trial_id: format!("{policy_id}-{scenario_id}-{block_id}"),
            policy_id: policy_id.to_owned(),
            scenario_id: scenario_id.to_owned(),
            paired_block_id: block_id.to_owned(),
            realized_severity_ppm: 500_000,
            fault_injection_receipt_digest: "fault-injection-receipt".to_owned(),
            run_trace_digest: "run-trace-digest".to_owned(),
            detected: DetectionResult::Detected {
                inferred_family: family,
                plausible_families: vec![family],
                latency_ms: 10,
                confidence_ppm: 900_000,
                severity_estimate_ppm: Some(500_000),
                severity_interval_ppm: Some((450_000, 550_000)),
            },
            adaptation: AdaptationResult::Stable { latency_ms: 80 },
            terminal_outcome: TerminalOutcome::TaskCompleted,
            recovery_succeeded: false,
            recovery_latency_ms: None,
            task_progress_ppm: PPM_SCALE,
            energy_mj: 500,
            event_vector_receipt_digest: "event-vector-receipt".to_owned(),
            viability_receipt_digest: "viability-receipt".to_owned(),
            action_applied: true,
            certificate_current_at_action: true,
            applied_action_admitted: true,
        }
    }

    fn problem() -> FaultBenchmarkProblem {
        let scenarios = vec![
            scenario("scenario-locked", FaultFamily::JointLocked),
            scenario("scenario-power", FaultFamily::CompletePowerLoss),
        ];
        let mut trials = Vec::new();
        for scenario in &scenarios {
            for policy_id in ["nominal", "adaptive"] {
                for block in ["paired-1", "paired-2"] {
                    trials.push(trial(
                        policy_id,
                        &scenario.scenario_id,
                        block,
                        scenario.fault_family,
                    ));
                }
            }
        }
        FaultBenchmarkProblem {
            manifest: BenchmarkManifest {
                manifest_digest: "fault-bench-v1".to_owned(),
                evidence_tier: EvidenceTier::Simulation,
                baseline_policy_id: "nominal".to_owned(),
                policies: vec![
                    policy("nominal", false, false),
                    policy("adaptive", false, false),
                ],
                training_scenario_ids: vec!["train-locked".to_owned()],
                validation_scenario_ids: vec!["validation-power".to_owned()],
                heldout_scenarios: scenarios,
                paired_block_ids: vec!["paired-1".to_owned(), "paired-2".to_owned()],
            },
            trials,
        }
    }

    #[test]
    fn complete_matched_fault_suite_is_audited() {
        let report = audit_fault_benchmark(&problem()).expect("matched benchmark");
        assert_eq!(report.heldout_scenario_count, 2);
        assert_eq!(report.paired_block_count, 2);
        assert_eq!(report.matched_trial_count, 8);
        assert_eq!(report.policy_coverage.len(), 2);
        assert_eq!(report.family_coverage.len(), 2);
    }

    #[test]
    fn missing_policy_scenario_block_trial_is_rejected() {
        let mut input = problem();
        input.trials.pop();
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::MissingMatchedTrial)
        );
    }

    #[test]
    fn duplicate_trial_key_is_rejected() {
        let mut input = problem();
        input.trials.push(input.trials[0].clone());
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::DuplicateTrial)
        );
    }

    #[test]
    fn training_validation_scenario_leakage_is_rejected() {
        let mut input = problem();
        input
            .manifest
            .training_scenario_ids
            .push("validation-power".to_owned());
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::TrainingValidationLeakage)
        );
    }

    #[test]
    fn training_heldout_scenario_leakage_is_rejected() {
        let mut input = problem();
        input
            .manifest
            .training_scenario_ids
            .push("scenario-locked".to_owned());
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::TrainingHeldoutLeakage)
        );
    }

    #[test]
    fn policy_actuator_or_morphology_context_must_match() {
        let mut input = problem();
        input.manifest.policies[1].context.actuator_model_digest =
            "different-actuator-model".to_owned();
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::PolicyContextMismatch)
        );
    }

    #[test]
    fn deployment_fault_truth_is_only_permitted_for_oracle_upper_bound() {
        let mut input = problem();
        input.manifest.policies[1].deployment_fault_truth_visible = true;
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::OraclePolicyTruthMismatch)
        );
    }

    #[test]
    fn safe_abstention_does_not_count_as_an_unadmitted_applied_command() {
        let mut input = problem();
        input.trials[0].action_applied = false;
        input.trials[0].certificate_current_at_action = false;
        input.trials[0].applied_action_admitted = false;
        let report = audit_fault_benchmark(&input).expect("safe abstention is measurable");
        let policy = report
            .policy_coverage
            .iter()
            .find(|row| row.policy_id == "nominal")
            .expect("nominal policy");
        assert_eq!(policy.action_abstention_count, 1);
        assert_eq!(policy.stale_certificate_count, 0);
        assert_eq!(policy.unadmitted_action_count, 0);
    }

    #[test]
    fn stale_certificate_cannot_authorize_an_applied_action() {
        let mut input = problem();
        input.trials[0].action_applied = true;
        input.trials[0].certificate_current_at_action = false;
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::StaleCertificateActionApplied)
        );
    }

    #[test]
    fn unadmitted_action_cannot_be_marked_as_applied() {
        let mut input = problem();
        input.trials[0].action_applied = true;
        input.trials[0].applied_action_admitted = false;
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::UnadmittedActionApplied)
        );
    }

    #[test]
    fn unknown_fault_diagnosis_is_counted_not_treated_as_zero_latency() {
        let mut input = problem();
        input.trials[0].detected = DetectionResult::Unknown {
            reason: "slip and torque-loss hypotheses overlap".to_owned(),
            plausible_families: vec![FaultFamily::JointLocked],
        };
        let report = audit_fault_benchmark(&input).expect("unknown diagnosis preserved");
        let locked = report
            .family_coverage
            .iter()
            .find(|row| row.family == FaultFamily::JointLocked)
            .expect("locked coverage");
        assert_eq!(locked.unknown_diagnosis_count, 1);
        assert_eq!(locked.detection_count, 3);
        assert_eq!(locked.fault_set_covered_count, 4);
        assert_eq!(locked.fault_set_uncovered_count, 0);
    }

    #[test]
    fn fault_family_misclassification_is_counted_not_silently_relabelled() {
        let mut input = problem();
        input.trials[0].detected = DetectionResult::Detected {
            inferred_family: FaultFamily::TorqueDerating,
            plausible_families: vec![
                FaultFamily::JointLocked,
                FaultFamily::TorqueDerating,
            ],
            latency_ms: 10,
            confidence_ppm: 800_000,
            severity_estimate_ppm: None,
            severity_interval_ppm: None,
        };
        let report = audit_fault_benchmark(&input).expect("misclassification measured");
        let row = report
            .family_coverage
            .iter()
            .find(|row| row.family == FaultFamily::JointLocked)
            .expect("locked row");
        assert_eq!(row.misclassified_count, 1);
    }

    #[test]
    fn fault_hypothesis_set_coverage_is_distinct_from_top_one_accuracy() {
        let mut input = problem();
        input.trials[0].detected = DetectionResult::Detected {
            inferred_family: FaultFamily::TorqueDerating,
            plausible_families: vec![
                FaultFamily::JointLocked,
                FaultFamily::TorqueDerating,
            ],
            latency_ms: 10,
            confidence_ppm: 800_000,
            severity_estimate_ppm: None,
            severity_interval_ppm: None,
        };
        let report = audit_fault_benchmark(&input).expect("set-valued diagnosis");
        let row = report
            .family_coverage
            .iter()
            .find(|row| row.family == FaultFamily::JointLocked)
            .expect("locked family");
        assert_eq!(row.misclassified_count, 1);
        assert_eq!(row.fault_set_covered_count, 4);
        assert_eq!(row.fault_set_uncovered_count, 0);
    }

    #[test]
    fn fault_severity_estimate_and_interval_coverage_are_reported() {
        let report = audit_fault_benchmark(&problem()).expect("severity metrics");
        let row = report
            .family_coverage
            .iter()
            .find(|row| row.family == FaultFamily::JointLocked)
            .expect("locked family");
        assert_eq!(row.severity_estimate_count, 4);
        assert_eq!(row.severity_absolute_error_sum_ppm, 0);
        assert_eq!(row.severity_interval_coverage_count, 4);
    }

    #[test]
    fn duplicate_fault_hypothesis_is_rejected() {
        let mut input = problem();
        input.trials[0].detected = DetectionResult::Detected {
            inferred_family: FaultFamily::JointLocked,
            plausible_families: vec![
                FaultFamily::JointLocked,
                FaultFamily::JointLocked,
            ],
            latency_ms: 10,
            confidence_ppm: 900_000,
            severity_estimate_ppm: None,
            severity_interval_ppm: None,
        };
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::InvalidDetectionResult)
        );
    }

    #[test]
    fn late_detection_and_adaptation_are_reported_separately() {
        let mut input = problem();
        input.trials[0].detected = DetectionResult::Detected {
            inferred_family: FaultFamily::JointLocked,
            plausible_families: vec![FaultFamily::JointLocked],
            latency_ms: 25,
            confidence_ppm: 900_000,
            severity_estimate_ppm: Some(500_000),
            severity_interval_ppm: Some((450_000, 550_000)),
        };
        input.trials[0].adaptation = AdaptationResult::Stable { latency_ms: 125 };
        let report = audit_fault_benchmark(&input).expect("late response measured");
        let row = report
            .family_coverage
            .iter()
            .find(|row| row.family == FaultFamily::JointLocked)
            .expect("locked row");
        assert_eq!(row.late_detection_count, 1);
        assert_eq!(row.late_adaptation_count, 1);
    }

    #[test]
    fn recoverable_stop_requires_recovery_evidence() {
        let mut input = problem();
        input.trials[0].terminal_outcome = TerminalOutcome::ControlledStop;
        input.trials[0].recovery_succeeded = false;
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::InvalidRecoveryResult)
        );
    }

    #[test]
    fn scenario_model_identity_must_match_benchmark_context() {
        let mut input = problem();
        input.manifest.heldout_scenarios[0].sensor_model_digest = "other-sensors".to_owned();
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::ScenarioContextMismatch)
        );
    }

    #[test]
    fn fault_task_and_command_schedule_must_match_campaign_context() {
        let mut input = problem();
        input.manifest.heldout_scenarios[0].task_digest = "different-task".to_owned();
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::ScenarioContextMismatch)
        );
    }

    #[test]
    fn safe_abstention_does_not_count_as_an_unadmitted_applied_command() {
        let mut input = problem();
        input.trials[0].action_applied = false;
        input.trials[0].certificate_current_at_action = false;
        input.trials[0].applied_action_admitted = false;
        let report = audit_fault_benchmark(&input).expect("safe abstention is measurable");
        let policy = report
            .policy_coverage
            .iter()
            .find(|row| row.policy_id == "nominal")
            .expect("nominal policy");
        assert_eq!(policy.action_abstention_count, 1);
        assert_eq!(policy.stale_certificate_count, 0);
        assert_eq!(policy.unadmitted_action_count, 0);
    }

    #[test]
    fn severity_estimate_outside_its_declared_interval_is_rejected() {
        let mut input = problem();
        input.trials[0].detected = DetectionResult::Detected {
            inferred_family: FaultFamily::JointLocked,
            plausible_families: vec![FaultFamily::JointLocked],
            latency_ms: 10,
            confidence_ppm: 900_000,
            severity_estimate_ppm: Some(800_000),
            severity_interval_ppm: Some((400_000, 600_000)),
        };
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::InvalidSeverityInterval)
        );
    }

    #[test]
    fn zero_exposure_budget_and_empty_receipts_fail_closed() {
        let mut input = problem();
        input.manifest.policies[0].context.test_exposure_units = 0;
        assert_eq!(
            audit_fault_benchmark(&input),
            Err(FaultBenchmarkError::InvalidMatchedContext)
        );
    }
}
