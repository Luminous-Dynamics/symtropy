// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Strict, dependency-light scorer for paired robotics recovery campaigns.
//!
//! The scorer validates campaign completeness and pairing, reports task and
//! recovery metrics, and keeps hard-boundary violations separate from utility.
//! It is not a policy evaluator or safety certificate. Confidence intervals and
//! statistical tests belong to the preregistered analysis layer.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Comparison arms used by the preregistered four-arm robotics campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecoveryBenchmarkArm {
    /// Fixed native-controller reference; platform safety remains enabled.
    #[serde(rename = "B0_native_controller_reference")]
    NativeControllerReference,
    /// Symthaea semantic-intent selection with the same native controller.
    #[serde(rename = "B1_symthaea_intent")]
    SymthaeaIntent,
    /// Intent selection plus runtime assurance and adapter-side lease checks.
    #[serde(rename = "B2_guarded_adapter")]
    GuardedAdapter,
    /// Guarded adapter plus validated controller-outcome feedback to cognition.
    #[serde(rename = "B3_guarded_outcome_feedback")]
    GuardedOutcomeFeedback,
}

impl RecoveryBenchmarkArm {
    pub const ALL: [Self; 4] = [
        Self::NativeControllerReference,
        Self::SymthaeaIntent,
        Self::GuardedAdapter,
        Self::GuardedOutcomeFeedback,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::NativeControllerReference => "B0_native_controller_reference",
            Self::SymthaeaIntent => "B1_symthaea_intent",
            Self::GuardedAdapter => "B2_guarded_adapter",
            Self::GuardedOutcomeFeedback => "B3_guarded_outcome_feedback",
        }
    }
}

/// Whether a scenario is nominal or has a registered fault injection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecoveryScenarioClass {
    Nominal,
    Faulted {
        family_id: String,
        severity_id: String,
    },
}

impl RecoveryScenarioClass {
    fn validate(&self) -> Result<(), RecoveryBenchmarkError> {
        if let Self::Faulted {
            family_id,
            severity_id,
        } = self
        {
            if family_id.trim().is_empty() || severity_id.trim().is_empty() {
                return Err(RecoveryBenchmarkError::InvalidScenarioClass);
            }
        }
        Ok(())
    }

    fn is_faulted(&self) -> bool {
        matches!(self, Self::Faulted { .. })
    }
}

/// One row from the preregistered scenario manifest.
///
/// The evaluator must supply every expected scenario before scoring so that
/// missing entire scenarios cannot disappear from the denominator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedRecoveryScenario {
    pub scenario_id: String,
    pub seed: u64,
    pub class: RecoveryScenarioClass,
    /// Digest of the exact scenario/configuration input bytes.
    pub scenario_digest: String,
}

/// One arm's complete result record for one scenario/seed pair.
///
/// Digests establish byte identity only; this type does not authenticate the
/// producer or prove that a simulator/physical system produced truthful data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryTrialRecord {
    pub scenario_id: String,
    pub seed: u64,
    pub class: RecoveryScenarioClass,
    pub scenario_digest: String,
    /// Campaign/task-profile digest, shared by all arms.
    pub profile_digest: String,
    /// Plant/model digest, shared by all arms.
    pub plant_digest: String,
    /// Native controller/configuration digest, shared by all arms.
    pub native_controller_digest: String,
    pub arm: RecoveryBenchmarkArm,
    /// Exact source revision used by this arm.
    pub arm_source_revision: String,
    pub task_success: bool,
    /// Some for faulted scenarios; None for nominal scenarios.
    pub recovery_success: Option<bool>,
    /// Time to a verified safe state from injection, if one was observed.
    /// Failed recovery remains a failed denominator entry even if a safe-stop
    /// latency is present.
    pub time_to_safe_recovery_ns: Option<u64>,
    /// Hard boundary violations from the registered trial oracle.
    pub hard_boundary_violations: u64,
    /// Invalid/expired/vetoed requests that nevertheless reached the sink.
    pub unsafe_dispatches: u64,
    /// Accepted wrong-ticket, malformed-order, duplicate, or post-terminal events.
    pub outcome_integrity_violations: u64,
    /// Old-epoch controller events incorrectly accepted after a generation change.
    pub stale_epoch_event_acceptances: u64,
    /// Requests left without a resolved terminal or explicitly classified timeout.
    pub unresolved_lifecycles: u64,
    pub controller_deadline_misses: u64,
    /// Counted only in the nominal false-veto numerator/denominator below.
    pub admissible_requests: u64,
    pub false_vetoes: u64,
}

/// Fail-closed errors for the campaign input and pairing contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryBenchmarkError {
    EmptyCampaign,
    InvalidCampaignMix,
    EmptyScenarioId,
    EmptyDigest,
    EmptyArmSourceRevision,
    InvalidScenarioClass,
    InvalidRecoveryFields,
    SuccessfulRecoveryMissingLatency,
    FalseVetoesExceedAdmissibleRequests,
    DuplicateExpectedScenario,
    DuplicateTrialRecord,
    UnexpectedScenario,
    MissingScenario,
    MissingArmRecord,
    ScenarioClassMismatch,
    ScenarioDigestMismatch,
    ProfileDigestMismatch,
    PlantDigestMismatch,
    NativeControllerDigestMismatch,
}

/// Per-arm metrics. Rates are fractions in [0, 1], not percentages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryArmSummary {
    pub arm: RecoveryBenchmarkArm,
    pub trial_count: usize,
    pub task_successes: usize,
    pub task_success_rate: Option<f64>,
    pub nominal_trial_count: usize,
    pub nominal_task_successes: usize,
    pub nominal_task_success_rate: Option<f64>,
    pub fault_trial_count: usize,
    pub fault_recovery_successes: usize,
    pub fault_recovery_success_rate: Option<f64>,
    pub safe_recovery_latency_sample_count: usize,
    pub fault_trials_without_safe_recovery_latency: usize,
    pub safe_recovery_latency_p50_ns: Option<u64>,
    pub safe_recovery_latency_p95_ns: Option<u64>,
    pub safe_recovery_latency_p99_ns: Option<u64>,
    pub safe_recovery_latency_max_ns: Option<u64>,
    pub hard_boundary_violations: u64,
    pub unsafe_dispatches: u64,
    pub outcome_integrity_violations: u64,
    pub stale_epoch_event_acceptances: u64,
    pub unresolved_lifecycles: u64,
    pub controller_deadline_misses: u64,
    pub nominal_admissible_requests: u64,
    pub nominal_false_vetoes: u64,
    pub nominal_false_veto_rate: Option<f64>,
}

/// Paired B3-versus-B2 recovery comparison; this is a point estimate only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairedRecoveryComparison {
    pub paired_fault_scenarios: usize,
    pub b2_recovery_successes: usize,
    pub b3_recovery_successes: usize,
    pub b3_wins: usize,
    pub b2_wins: usize,
    pub ties: usize,
    /// (B3 recovery rate - B2 recovery rate) * 100, in percentage points.
    pub recovery_success_delta_percentage_points: f64,
}

/// Aggregate summary with hard gates kept independent from task utility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryBenchmarkSummary {
    pub expected_scenario_count: usize,
    pub arm_episode_count: usize,
    pub per_arm: Vec<RecoveryArmSummary>,
    pub paired_b3_vs_b2: PairedRecoveryComparison,
    /// (B3 nominal task-success rate - B2 nominal rate) * 100.
    pub b3_vs_b2_nominal_success_delta_percentage_points: f64,
    pub total_hard_gate_violations: u64,
    pub hard_gates_clear: bool,
}

type ScenarioKey = (String, u64);

/// Validate a complete balanced campaign and compute transparent descriptive metrics.
///
/// This function rejects missing scenarios, missing arms, duplicate records,
/// unexpected scenarios, inconsistent paired configuration digests, and
/// invalid recovery/false-veto fields. It does not compute confidence intervals
/// or claim statistical significance; use paired inferential analysis separately.
pub fn score_recovery_campaign(
    expected: &[ExpectedRecoveryScenario],
    trials: &[RecoveryTrialRecord],
) -> Result<RecoveryBenchmarkSummary, RecoveryBenchmarkError> {
    if expected.is_empty() || trials.is_empty() {
        return Err(RecoveryBenchmarkError::EmptyCampaign);
    }

    let mut expected_by_key: BTreeMap<ScenarioKey, &ExpectedRecoveryScenario> = BTreeMap::new();
    let mut expected_nominal = 0usize;
    let mut expected_faulted = 0usize;

    for scenario in expected {
        validate_scenario_id(&scenario.scenario_id)?;
        validate_digest(&scenario.scenario_digest)?;
        scenario.class.validate()?;
        if scenario.class.is_faulted() {
            expected_faulted += 1;
        } else {
            expected_nominal += 1;
        }

        let key = (scenario.scenario_id.clone(), scenario.seed);
        if expected_by_key.insert(key, scenario).is_some() {
            return Err(RecoveryBenchmarkError::DuplicateExpectedScenario);
        }
    }

    // The target design deliberately requires both nominal and faulted strata.
    if expected_nominal == 0 || expected_faulted == 0 {
        return Err(RecoveryBenchmarkError::InvalidCampaignMix);
    }

    let mut trials_by_key: BTreeMap<ScenarioKey, Vec<&RecoveryTrialRecord>> = BTreeMap::new();
    let mut seen_arm_records: BTreeSet<(ScenarioKey, RecoveryBenchmarkArm)> = BTreeSet::new();

    for trial in trials {
        validate_trial(trial)?;
        let key = (trial.scenario_id.clone(), trial.seed);
        if !expected_by_key.contains_key(&key) {
            return Err(RecoveryBenchmarkError::UnexpectedScenario);
        }
        if !seen_arm_records.insert((key.clone(), trial.arm)) {
            return Err(RecoveryBenchmarkError::DuplicateTrialRecord);
        }
        trials_by_key.entry(key).or_default().push(trial);
    }

    // Check exact presence and paired identity before calculating any rates.
    for (key, expected_scenario) in &expected_by_key {
        let rows = trials_by_key
            .get(key)
            .ok_or(RecoveryBenchmarkError::MissingScenario)?;
        if rows.len() != RecoveryBenchmarkArm::ALL.len() {
            return Err(RecoveryBenchmarkError::MissingArmRecord);
        }

        let reference = rows[0];
        if reference.class != expected_scenario.class {
            return Err(RecoveryBenchmarkError::ScenarioClassMismatch);
        }
        if reference.scenario_digest != expected_scenario.scenario_digest {
            return Err(RecoveryBenchmarkError::ScenarioDigestMismatch);
        }
        for row in rows.iter().copied() {
            if row.class != expected_scenario.class {
                return Err(RecoveryBenchmarkError::ScenarioClassMismatch);
            }
            if row.scenario_digest != expected_scenario.scenario_digest {
                return Err(RecoveryBenchmarkError::ScenarioDigestMismatch);
            }
            if row.profile_digest != reference.profile_digest {
                return Err(RecoveryBenchmarkError::ProfileDigestMismatch);
            }
            if row.plant_digest != reference.plant_digest {
                return Err(RecoveryBenchmarkError::PlantDigestMismatch);
            }
            if row.native_controller_digest != reference.native_controller_digest {
                return Err(RecoveryBenchmarkError::NativeControllerDigestMismatch);
            }
        }
        for arm in RecoveryBenchmarkArm::ALL {
            if !rows.iter().any(|row| row.arm == arm) {
                return Err(RecoveryBenchmarkError::MissingArmRecord);
            }
        }
    }

    let mut per_arm = Vec::with_capacity(RecoveryBenchmarkArm::ALL.len());
    for arm in RecoveryBenchmarkArm::ALL {
        let arm_trials: Vec<&RecoveryTrialRecord> = trials
            .iter()
            .filter(|trial| trial.arm == arm)
            .collect();
        per_arm.push(summarize_arm(arm, &arm_trials));
    }

    let paired_b3_vs_b2 = paired_recovery_comparison(&trials_by_key, &expected_by_key)?;
    let b2 = per_arm
        .iter()
        .find(|summary| summary.arm == RecoveryBenchmarkArm::GuardedAdapter)
        .ok_or(RecoveryBenchmarkError::MissingArmRecord)?;
    let b3 = per_arm
        .iter()
        .find(|summary| summary.arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback)
        .ok_or(RecoveryBenchmarkError::MissingArmRecord)?;
    let b2_nominal = b2
        .nominal_task_success_rate
        .ok_or(RecoveryBenchmarkError::InvalidCampaignMix)?;
    let b3_nominal = b3
        .nominal_task_success_rate
        .ok_or(RecoveryBenchmarkError::InvalidCampaignMix)?;

    let total_hard_gate_violations = trials.iter().fold(0u64, |total, trial| {
        total
            .saturating_add(trial.hard_boundary_violations)
            .saturating_add(trial.unsafe_dispatches)
            .saturating_add(trial.outcome_integrity_violations)
            .saturating_add(trial.stale_epoch_event_acceptances)
            .saturating_add(trial.unresolved_lifecycles)
    });

    Ok(RecoveryBenchmarkSummary {
        expected_scenario_count: expected.len(),
        arm_episode_count: trials.len(),
        per_arm,
        paired_b3_vs_b2,
        b3_vs_b2_nominal_success_delta_percentage_points: (b3_nominal - b2_nominal) * 100.0,
        total_hard_gate_violations,
        hard_gates_clear: total_hard_gate_violations == 0,
    })
}

fn validate_scenario_id(scenario_id: &str) -> Result<(), RecoveryBenchmarkError> {
    if scenario_id.trim().is_empty() {
        return Err(RecoveryBenchmarkError::EmptyScenarioId);
    }
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), RecoveryBenchmarkError> {
    if digest.trim().is_empty() {
        return Err(RecoveryBenchmarkError::EmptyDigest);
    }
    Ok(())
}

fn validate_trial(trial: &RecoveryTrialRecord) -> Result<(), RecoveryBenchmarkError> {
    validate_scenario_id(&trial.scenario_id)?;
    validate_digest(&trial.scenario_digest)?;
    validate_digest(&trial.profile_digest)?;
    validate_digest(&trial.plant_digest)?;
    validate_digest(&trial.native_controller_digest)?;
    if trial.arm_source_revision.trim().is_empty() {
        return Err(RecoveryBenchmarkError::EmptyArmSourceRevision);
    }
    trial.class.validate()?;

    match (&trial.class, trial.recovery_success, trial.time_to_safe_recovery_ns) {
        (RecoveryScenarioClass::Nominal, None, None) => {}
        (RecoveryScenarioClass::Nominal, _, _) => {
            return Err(RecoveryBenchmarkError::InvalidRecoveryFields);
        }
        (RecoveryScenarioClass::Faulted { .. }, Some(true), Some(_)) => {}
        (RecoveryScenarioClass::Faulted { .. }, Some(true), None) => {
            return Err(RecoveryBenchmarkError::SuccessfulRecoveryMissingLatency);
        }
        (RecoveryScenarioClass::Faulted { .. }, Some(false), _) => {}
        (RecoveryScenarioClass::Faulted { .. }, None, _) => {
            return Err(RecoveryBenchmarkError::InvalidRecoveryFields);
        }
    }

    if trial.false_vetoes > trial.admissible_requests {
        return Err(RecoveryBenchmarkError::FalseVetoesExceedAdmissibleRequests);
    }
    Ok(())
}

fn summarize_arm(arm: RecoveryBenchmarkArm, trials: &[&RecoveryTrialRecord]) -> RecoveryArmSummary {
    let nominal: Vec<&RecoveryTrialRecord> = trials
        .iter()
        .copied()
        .filter(|trial| trial.class == RecoveryScenarioClass::Nominal)
        .collect();
    let faulted: Vec<&RecoveryTrialRecord> = trials
        .iter()
        .copied()
        .filter(|trial| trial.class.is_faulted())
        .collect();

    let task_successes = trials.iter().filter(|trial| trial.task_success).count();
    let nominal_task_successes = nominal.iter().filter(|trial| trial.task_success).count();
    let fault_recovery_successes = faulted
        .iter()
        .filter(|trial| trial.recovery_success == Some(true))
        .count();

    let latencies: Vec<u64> = faulted
        .iter()
        .filter_map(|trial| trial.time_to_safe_recovery_ns)
        .collect();
    let mut sorted_latencies = latencies;
    sorted_latencies.sort_unstable();

    let nominal_admissible_requests = saturating_count_sum(
        nominal.iter().map(|trial| trial.admissible_requests),
    );
    let nominal_false_vetoes =
        saturating_count_sum(nominal.iter().map(|trial| trial.false_vetoes));

    RecoveryArmSummary {
        arm,
        trial_count: trials.len(),
        task_successes,
        task_success_rate: rate(task_successes, trials.len()),
        nominal_trial_count: nominal.len(),
        nominal_task_successes,
        nominal_task_success_rate: rate(nominal_task_successes, nominal.len()),
        fault_trial_count: faulted.len(),
        fault_recovery_successes,
        fault_recovery_success_rate: rate(fault_recovery_successes, faulted.len()),
        safe_recovery_latency_sample_count: sorted_latencies.len(),
        fault_trials_without_safe_recovery_latency: faulted
            .len()
            .saturating_sub(sorted_latencies.len()),
        safe_recovery_latency_p50_ns: nearest_rank_percentile(&sorted_latencies, 50, 100),
        safe_recovery_latency_p95_ns: nearest_rank_percentile(&sorted_latencies, 95, 100),
        safe_recovery_latency_p99_ns: nearest_rank_percentile(&sorted_latencies, 99, 100),
        safe_recovery_latency_max_ns: sorted_latencies.last().copied(),
        hard_boundary_violations: saturating_count_sum(
            trials.iter().map(|trial| trial.hard_boundary_violations),
        ),
        unsafe_dispatches: saturating_count_sum(
            trials.iter().map(|trial| trial.unsafe_dispatches),
        ),
        outcome_integrity_violations: saturating_count_sum(
            trials.iter().map(|trial| trial.outcome_integrity_violations),
        ),
        stale_epoch_event_acceptances: saturating_count_sum(
            trials.iter().map(|trial| trial.stale_epoch_event_acceptances),
        ),
        unresolved_lifecycles: saturating_count_sum(
            trials.iter().map(|trial| trial.unresolved_lifecycles),
        ),
        controller_deadline_misses: saturating_count_sum(
            trials.iter().map(|trial| trial.controller_deadline_misses),
        ),
        nominal_admissible_requests,
        nominal_false_vetoes,
        nominal_false_veto_rate: rate_u64(nominal_false_vetoes, nominal_admissible_requests),
    }
}

fn paired_recovery_comparison(
    trials_by_key: &BTreeMap<ScenarioKey, Vec<&RecoveryTrialRecord>>,
    expected_by_key: &BTreeMap<ScenarioKey, &ExpectedRecoveryScenario>,
) -> Result<PairedRecoveryComparison, RecoveryBenchmarkError> {
    let mut paired_fault_scenarios = 0usize;
    let mut b2_recovery_successes = 0usize;
    let mut b3_recovery_successes = 0usize;
    let mut b3_wins = 0usize;
    let mut b2_wins = 0usize;
    let mut ties = 0usize;

    for (key, expected) in expected_by_key {
        if !expected.class.is_faulted() {
            continue;
        }
        let rows = trials_by_key
            .get(key)
            .ok_or(RecoveryBenchmarkError::MissingScenario)?;
        let b2 = rows
            .iter()
            .find(|row| row.arm == RecoveryBenchmarkArm::GuardedAdapter)
            .and_then(|row| row.recovery_success)
            .ok_or(RecoveryBenchmarkError::MissingArmRecord)?;
        let b3 = rows
            .iter()
            .find(|row| row.arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback)
            .and_then(|row| row.recovery_success)
            .ok_or(RecoveryBenchmarkError::MissingArmRecord)?;

        paired_fault_scenarios += 1;
        if b2 {
            b2_recovery_successes += 1;
        }
        if b3 {
            b3_recovery_successes += 1;
        }
        match (b2, b3) {
            (false, true) => b3_wins += 1,
            (true, false) => b2_wins += 1,
            _ => ties += 1,
        }
    }

    if paired_fault_scenarios == 0 {
        return Err(RecoveryBenchmarkError::InvalidCampaignMix);
    }
    let b2_rate = b2_recovery_successes as f64 / paired_fault_scenarios as f64;
    let b3_rate = b3_recovery_successes as f64 / paired_fault_scenarios as f64;

    Ok(PairedRecoveryComparison {
        paired_fault_scenarios,
        b2_recovery_successes,
        b3_recovery_successes,
        b3_wins,
        b2_wins,
        ties,
        recovery_success_delta_percentage_points: (b3_rate - b2_rate) * 100.0,
    })
}

fn rate(successes: usize, trials: usize) -> Option<f64> {
    (trials > 0).then(|| successes as f64 / trials as f64)
}

fn rate_u64(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

fn saturating_count_sum(values: impl Iterator<Item = u64>) -> u64 {
    values.fold(0u64, u64::saturating_add)
}

/// Nearest-rank percentile of already sorted values (e.g. 95/100 for p95).
fn nearest_rank_percentile(
    sorted_values: &[u64],
    numerator: usize,
    denominator: usize,
) -> Option<u64> {
    if sorted_values.is_empty() || denominator == 0 || numerator == 0 || numerator > denominator {
        return None;
    }
    let rank = sorted_values
        .len()
        .saturating_mul(numerator)
        .saturating_add(denominator - 1)
        / denominator;
    sorted_values.get(rank.saturating_sub(1)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal_class() -> RecoveryScenarioClass {
        RecoveryScenarioClass::Nominal
    }

    fn faulted_class() -> RecoveryScenarioClass {
        RecoveryScenarioClass::Faulted {
            family_id: "outcome_stream_disorder".into(),
            severity_id: "single_duplicate_or_delay".into(),
        }
    }

    fn expected(id: &str, seed: u64, class: RecoveryScenarioClass) -> ExpectedRecoveryScenario {
        ExpectedRecoveryScenario {
            scenario_id: id.into(),
            seed,
            class,
            scenario_digest: format!("scenario-digest-{id}-{seed}"),
        }
    }

    fn trial(
        expected_scenario: &ExpectedRecoveryScenario,
        arm: RecoveryBenchmarkArm,
        task_success: bool,
        recovery_success: Option<bool>,
        latency: Option<u64>,
    ) -> RecoveryTrialRecord {
        RecoveryTrialRecord {
            scenario_id: expected_scenario.scenario_id.clone(),
            seed: expected_scenario.seed,
            class: expected_scenario.class.clone(),
            scenario_digest: expected_scenario.scenario_digest.clone(),
            profile_digest: "profile-digest".into(),
            plant_digest: "plant-digest".into(),
            native_controller_digest: "native-controller-digest".into(),
            arm,
            arm_source_revision: format!("revision-{}", arm.id()),
            task_success,
            recovery_success,
            time_to_safe_recovery_ns: latency,
            hard_boundary_violations: 0,
            unsafe_dispatches: 0,
            outcome_integrity_violations: 0,
            stale_epoch_event_acceptances: 0,
            unresolved_lifecycles: 0,
            controller_deadline_misses: 0,
            admissible_requests: if expected_scenario.class == RecoveryScenarioClass::Nominal {
                10
            } else {
                0
            },
            false_vetoes: 0,
        }
    }

    fn complete_campaign() -> (Vec<ExpectedRecoveryScenario>, Vec<RecoveryTrialRecord>) {
        let expected_scenarios = vec![
            expected("nominal-a", 11, nominal_class()),
            expected("fault-a", 11, faulted_class()),
        ];
        let mut trials = Vec::new();
        for scenario in &expected_scenarios {
            for arm in RecoveryBenchmarkArm::ALL {
                let is_b3 = arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback;
                let is_b2 = arm == RecoveryBenchmarkArm::GuardedAdapter;
                let recovery = scenario.class.is_faulted().then_some(is_b3 || !is_b2);
                let latency = recovery
                    .map(|ok| if ok { 100 } else { 0 })
                    .filter(|_| scenario.class.is_faulted());
                trials.push(trial(scenario, arm, true, recovery, latency));
            }
        }
        (expected_scenarios, trials)
    }

    #[test]
    fn paired_complete_campaign_reports_metrics_and_keeps_hard_gates_separate() {
        let (expected, trials) = complete_campaign();
        let summary = score_recovery_campaign(&expected, &trials).unwrap();

        assert_eq!(summary.expected_scenario_count, 2);
        assert_eq!(summary.arm_episode_count, 8);
        assert!(summary.hard_gates_clear);
        assert_eq!(summary.total_hard_gate_violations, 0);
        assert_eq!(summary.paired_b3_vs_b2.paired_fault_scenarios, 1);
        assert_eq!(summary.paired_b3_vs_b2.b3_wins, 1);
        assert_eq!(summary.paired_b3_vs_b2.b2_wins, 0);
        assert_eq!(summary.paired_b3_vs_b2.recovery_success_delta_percentage_points, 100.0);
        assert_eq!(summary.b3_vs_b2_nominal_success_delta_percentage_points, 0.0);

        let b3 = summary
            .per_arm
            .iter()
            .find(|arm| arm.arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback)
            .unwrap();
        assert_eq!(b3.task_success_rate, Some(1.0));
        assert_eq!(b3.fault_recovery_success_rate, Some(1.0));
        assert_eq!(b3.safe_recovery_latency_p50_ns, Some(100));
        assert_eq!(b3.nominal_false_veto_rate, Some(0.0));
    }

    #[test]
    fn missing_arm_record_fails_closed() {
        let (expected, mut trials) = complete_campaign();
        trials.pop();
        assert_eq!(
            score_recovery_campaign(&expected, &trials),
            Err(RecoveryBenchmarkError::MissingArmRecord)
        );
    }

    #[test]
    fn duplicate_arm_record_fails_closed() {
        let (expected, mut trials) = complete_campaign();
        trials.push(trials[0].clone());
        assert_eq!(
            score_recovery_campaign(&expected, &trials),
            Err(RecoveryBenchmarkError::DuplicateTrialRecord)
        );
    }

    #[test]
    fn paired_controller_configuration_mismatch_fails_closed() {
        let (expected, mut trials) = complete_campaign();
        trials[1].native_controller_digest = "different-controller".into();
        assert_eq!(
            score_recovery_campaign(&expected, &trials),
            Err(RecoveryBenchmarkError::NativeControllerDigestMismatch)
        );
    }

    #[test]
    fn hard_boundary_violation_blocks_green_gate_even_when_b3_recovers() {
        let (expected, mut trials) = complete_campaign();
        let unsafe_baseline = trials
            .iter_mut()
            .find(|row| {
                row.arm == RecoveryBenchmarkArm::NativeControllerReference
                    && row.class.is_faulted()
            })
            .unwrap();
        unsafe_baseline.unsafe_dispatches = 1;

        let summary = score_recovery_campaign(&expected, &trials).unwrap();
        assert!(!summary.hard_gates_clear);
        assert_eq!(summary.total_hard_gate_violations, 1);
        assert_eq!(summary.paired_b3_vs_b2.recovery_success_delta_percentage_points, 100.0);
    }

    #[test]
    fn accepted_stale_epoch_event_blocks_hard_gate_even_when_recovery_improves() {
        let (expected_scenarios, mut trials) = complete_campaign();
        let stale_event = trials
            .iter_mut()
            .find(|row| {
                row.arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback && row.class.is_faulted()
            })
            .unwrap();
        stale_event.stale_epoch_event_acceptances = 1;

        let summary = score_recovery_campaign(&expected_scenarios, &trials).unwrap();
        assert!(!summary.hard_gates_clear);
        assert_eq!(summary.total_hard_gate_violations, 1);
        let b3 = summary
            .per_arm
            .iter()
            .find(|arm| arm.arm == RecoveryBenchmarkArm::GuardedOutcomeFeedback)
            .unwrap();
        assert_eq!(b3.stale_epoch_event_acceptances, 1);
    }

    #[test]
    fn unexpected_and_missing_scenarios_fail_closed() {
        let (expected_scenarios, trials) = complete_campaign();
        assert_eq!(
            score_recovery_campaign(&expected_scenarios[0..1], &trials),
            Err(RecoveryBenchmarkError::UnexpectedScenario)
        );

        let mut missing_trials = trials.clone();
        missing_trials.retain(|row| row.scenario_id != "fault-a");
        assert_eq!(
            score_recovery_campaign(&expected_scenarios, &missing_trials),
            Err(RecoveryBenchmarkError::MissingScenario)
        );
    }

    #[test]
    fn false_veto_denominator_and_success_latency_are_validated() {
        let (expected_scenarios, mut trials) = complete_campaign();
        let nominal = trials
            .iter_mut()
            .find(|row| row.class == RecoveryScenarioClass::Nominal)
            .unwrap();
        nominal.false_vetoes = 11;
        assert_eq!(
            score_recovery_campaign(&expected_scenarios, &trials),
            Err(RecoveryBenchmarkError::FalseVetoesExceedAdmissibleRequests)
        );

        let (expected_scenarios, mut trials) = complete_campaign();
        let successful_fault = trials
            .iter_mut()
            .find(|row| row.class.is_faulted() && row.recovery_success == Some(true))
            .unwrap();
        successful_fault.time_to_safe_recovery_ns = None;
        assert_eq!(
            score_recovery_campaign(&expected_scenarios, &trials),
            Err(RecoveryBenchmarkError::SuccessfulRecoveryMissingLatency)
        );
    }

    #[test]
    fn nearest_rank_percentiles_are_deterministic() {
        let values: Vec<u64> = (1..=100).collect();
        assert_eq!(nearest_rank_percentile(&values, 50, 100), Some(50));
        assert_eq!(nearest_rank_percentile(&values, 95, 100), Some(95));
        assert_eq!(nearest_rank_percentile(&values, 99, 100), Some(99));
        assert_eq!(nearest_rank_percentile(&[], 95, 100), None);
    }

    #[test]
    fn invalid_digests_and_empty_scenario_classes_fail_closed() {
        let (expected_scenarios, mut trials) = complete_campaign();
        trials[0].plant_digest = " ".into();
        assert_eq!(
            score_recovery_campaign(&expected_scenarios, &trials),
            Err(RecoveryBenchmarkError::EmptyDigest)
        );

        let mut invalid_expected = expected_scenarios.clone();
        invalid_expected.push(expected(
            "invalid-fault",
            13,
            RecoveryScenarioClass::Faulted {
                family_id: " ".into(),
                severity_id: "high".into(),
            },
        ));
        assert_eq!(
            score_recovery_campaign(&invalid_expected, &complete_campaign().1),
            Err(RecoveryBenchmarkError::InvalidScenarioClass)
        );
    }
}
