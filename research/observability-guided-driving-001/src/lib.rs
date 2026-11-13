//! Risk-constrained observability and value-of-information auditor.
//!
//! This evaluates paired passive and active-observation branches at the same
//! frozen decision boundary. It reports proper scores, information changes,
//! action costs and safety outcomes separately; it never creates a composite.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    TrainedPolicy,
    UntrainedInputBridge,
    SyntheticReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchRole {
    PassiveControl,
    ActiveObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationActionKind {
    None,
    Wait,
    SensorRequest,
    SafeReposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DrivingDecision {
    Yield,
    Wait,
    Proceed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryBelief {
    /// Probability that the occluded actor exists, in parts per million.
    pub actor_present_ppm: u32,
    /// Probability that the actor enters the conflict/crossing area within the
    /// predeclared horizon, in parts per million.
    pub crosses_within_horizon_ppm: u32,
    /// Epistemic component, normalized to [0, 1_000_000].
    pub epistemic_uncertainty_ppm: u32,
    /// Aleatoric component, normalized to [0, 1_000_000].
    pub aleatoric_uncertainty_ppm: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCost {
    pub duration_us: u64,
    pub energy_uj: u64,
    pub maneuver_distance_mm: u64,
    pub estimated_residual_risk_ppm: u32,
    pub max_admissible_risk_ppm: u32,
    /// Policy-side prediction, not an evaluator truth label.
    pub expected_information_gain_milli_nats: u64,
    pub admissible_under_policy_constraints: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorOutcome {
    /// Hidden evaluator labels for the same root actor/intent at the paired
    /// decision boundary. These fields must never enter policy-visible inputs.
    pub actor_present: bool,
    pub crosses_within_horizon: bool,
    pub conflict_boundary_encroached: bool,
    pub contact_or_collision: bool,
    pub traffic_rule_violation: bool,
    pub minimum_clearance_mm: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchTrace {
    pub branch_id: String,
    pub role: BranchRole,
    pub information_action: InformationActionKind,
    pub decision_before: DrivingDecision,
    pub decision_after: DrivingDecision,
    pub belief_before: BinaryBelief,
    pub belief_after: BinaryBelief,
    pub action_cost: ActionCost,
    pub evaluator_outcome: EvaluatorOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PairedDecision {
    pub pair_id: String,
    pub scenario_id: String,
    pub decision_tick: u64,
    pub initial_world_snapshot_sha256: String,
    pub scenario_seed_sha256: String,
    pub exogenous_randomness_sha256: String,
    /// Link to the independent #4556 branch auditor's report/receipt.
    pub causal_branch_audit_sha256: String,
    pub no_safe_information_action_available: bool,
    pub passive_control: BranchTrace,
    pub active_observation: BranchTrace,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionBudget {
    pub maximum_duration_us: u64,
    pub maximum_energy_uj: u64,
    pub maximum_maneuver_distance_mm: u64,
    pub maximum_residual_risk_ppm: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub schema_version: u32,
    pub campaign_id: String,
    pub benchmark_revision: String,
    pub evaluator_revision: String,
    pub model_id: String,
    pub model_status: ModelStatus,
    pub model_revision: String,
    pub checkpoint_sha256: Option<String>,
    pub hardware_id: String,
    pub inference_precision: String,
    pub power_profile: String,
    pub energy_measurement_method: String,
    pub policy_visible_field_ids: Vec<String>,
    pub evaluator_only_field_ids: Vec<String>,
    pub expected_pair_count: u64,
    pub action_budget: ActionBudget,
    pub pairs: Vec<PairedDecision>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationBin {
    pub lower_bound_ppm: u32,
    pub upper_bound_ppm: u32,
    pub sample_count: usize,
    pub mean_confidence: Option<f64>,
    pub observed_frequency: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationMetrics {
    pub sample_count: usize,
    pub brier_score: Option<f64>,
    pub log_loss: Option<f64>,
    pub expected_calibration_error: Option<f64>,
    pub mean_confidence: Option<f64>,
    pub observed_frequency: Option<f64>,
    pub reliability_bins: Vec<CalibrationBin>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BranchCalibration {
    pub role: BranchRole,
    pub actor_presence_before: CalibrationMetrics,
    pub actor_presence_after: CalibrationMetrics,
    pub crossing_before: CalibrationMetrics,
    pub crossing_after: CalibrationMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostSummary {
    pub action_count: usize,
    pub total_duration_us: u128,
    pub total_energy_uj: u128,
    pub total_maneuver_distance_mm: u128,
    pub maximum_estimated_residual_risk_ppm: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairValueMetrics {
    pub pair_id: String,
    pub passive_crossing_brier: f64,
    pub active_crossing_brier: f64,
    /// Negative means active branch scored better than passive control.
    pub active_minus_passive_brier: f64,
    pub passive_entropy_reduction_nats: f64,
    pub active_entropy_reduction_nats: f64,
    pub additional_entropy_reduction_nats: f64,
    pub expected_information_gain_nats: f64,
    pub decision_changed_from_root: bool,
    pub active_action: InformationActionKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditReport {
    pub schema_version: u32,
    pub campaign_id: String,
    pub disposition: String,
    pub claim_ceiling: String,
    pub expected_pair_count: u64,
    pub observed_pair_count: usize,
    pub active_information_action_count: usize,
    pub decision_change_count: usize,
    pub unsafe_information_action_count: u64,
    pub no_safe_action_violations: u64,
    pub collision_or_contact_count: u64,
    pub conflict_boundary_encroachment_count: u64,
    pub traffic_rule_violation_count: u64,
    pub passive_control: BranchCalibration,
    pub active_observation: BranchCalibration,
    pub active_minus_passive_crossing_brier: Option<f64>,
    pub active_minus_passive_crossing_log_loss: Option<f64>,
    pub passive_cost: CostSummary,
    pub active_cost: CostSummary,
    pub pair_metrics: Vec<PairValueMetrics>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Finding {
    SchemaVersionMismatch,
    EmptyIdentity,
    InvalidCheckpointDigest,
    MissingCheckpointForTrainedPolicy,
    InvalidProbability,
    InvalidUncertainty,
    DuplicatePairId,
    PairDenominatorMismatch,
    EmptyPolicyVisibleFields,
    DuplicateFieldId,
    HiddenFieldLeak,
    InvalidDigest,
    InvalidDecisionTick,
    BranchIdentityMismatch,
    RootDecisionMismatch,
    HiddenOutcomeMismatch,
    PassiveBranchNotPassive,
    ActiveBranchWrongRole,
    PassiveBranchHasAction,
    ActionNotAdmissible,
    RiskBudgetExceeded,
    ActionTimeBudgetExceeded,
    ActionEnergyBudgetExceeded,
    ActionDistanceBudgetExceeded,
    UnsafeInformationSeeking,
    NoSafeActionViolation,
    UnsafeRepositionOutcome,
    DuplicateBranchId,
}

impl Finding {
    fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersionMismatch => "schema_version_mismatch",
            Self::EmptyIdentity => "empty_identity",
            Self::InvalidCheckpointDigest => "invalid_checkpoint_digest",
            Self::MissingCheckpointForTrainedPolicy => "missing_checkpoint_for_trained_policy",
            Self::InvalidProbability => "invalid_probability",
            Self::InvalidUncertainty => "invalid_uncertainty",
            Self::DuplicatePairId => "duplicate_pair_id",
            Self::PairDenominatorMismatch => "pair_denominator_mismatch",
            Self::EmptyPolicyVisibleFields => "empty_policy_visible_fields",
            Self::DuplicateFieldId => "duplicate_field_id",
            Self::HiddenFieldLeak => "hidden_field_leak",
            Self::InvalidDigest => "invalid_digest",
            Self::InvalidDecisionTick => "invalid_decision_tick",
            Self::BranchIdentityMismatch => "branch_identity_mismatch",
            Self::RootDecisionMismatch => "root_decision_mismatch",
            Self::HiddenOutcomeMismatch => "hidden_outcome_mismatch",
            Self::PassiveBranchNotPassive => "passive_branch_not_passive",
            Self::ActiveBranchWrongRole => "active_branch_wrong_role",
            Self::PassiveBranchHasAction => "passive_branch_has_action",
            Self::ActionNotAdmissible => "action_not_admissible",
            Self::RiskBudgetExceeded => "risk_budget_exceeded",
            Self::ActionTimeBudgetExceeded => "action_time_budget_exceeded",
            Self::ActionEnergyBudgetExceeded => "action_energy_budget_exceeded",
            Self::ActionDistanceBudgetExceeded => "action_distance_budget_exceeded",
            Self::UnsafeInformationSeeking => "unsafe_information_seeking",
            Self::NoSafeActionViolation => "no_safe_action_violation",
            Self::UnsafeRepositionOutcome => "unsafe_reposition_outcome",
            Self::DuplicateBranchId => "duplicate_branch_id",
        }
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_belief(belief: &BinaryBelief, findings: &mut BTreeSet<Finding>) {
    for probability in [belief.actor_present_ppm, belief.crosses_within_horizon_ppm] {
        if probability > 1_000_000 {
            findings.insert(Finding::InvalidProbability);
        }
    }
    if belief.epistemic_uncertainty_ppm > 1_000_000 || belief.aleatoric_uncertainty_ppm > 1_000_000
    {
        findings.insert(Finding::InvalidUncertainty);
    }
}

fn entropy_nats(probability_ppm: u32) -> f64 {
    let p = (probability_ppm as f64 / 1_000_000.0).clamp(1e-12, 1.0 - 1e-12);
    -p * p.ln() - (1.0 - p) * (1.0 - p).ln()
}

fn brier(probability_ppm: u32, outcome: bool) -> f64 {
    let p = probability_ppm as f64 / 1_000_000.0;
    let y = if outcome { 1.0 } else { 0.0 };
    (p - y).powi(2)
}

fn log_loss(probability_ppm: u32, outcome: bool) -> f64 {
    let p = (probability_ppm as f64 / 1_000_000.0).clamp(1e-6, 1.0 - 1e-6);
    let y = if outcome { 1.0 } else { 0.0 };
    -(y * p.ln() + (1.0 - y) * (1.0 - p).ln())
}

#[derive(Debug, Clone, Copy)]
struct CalibrationSample {
    probability_ppm: u32,
    outcome: bool,
}

fn calibration_metrics(samples: &[CalibrationSample]) -> CalibrationMetrics {
    const BIN_COUNT: usize = 10;
    let mut bins: Vec<Vec<CalibrationSample>> = vec![Vec::new(); BIN_COUNT];
    for sample in samples {
        let index = ((sample.probability_ppm as usize) / 100_000).min(BIN_COUNT - 1);
        bins[index].push(*sample);
    }

    let mut reliability_bins = Vec::with_capacity(BIN_COUNT);
    let mut calibration_error = 0.0;
    for (index, values) in bins.iter().enumerate() {
        let count = values.len();
        let lower = (index as u32) * 100_000;
        let upper = if index == BIN_COUNT - 1 {
            1_000_000
        } else {
            (index as u32 + 1) * 100_000 - 1
        };
        let mean_confidence = if count == 0 {
            None
        } else {
            Some(
                values
                    .iter()
                    .map(|item| item.probability_ppm as f64 / 1_000_000.0)
                    .sum::<f64>()
                    / count as f64,
            )
        };
        let observed_frequency = if count == 0 {
            None
        } else {
            Some(values.iter().filter(|item| item.outcome).count() as f64 / count as f64)
        };
        if let (Some(confidence), Some(frequency)) = (mean_confidence, observed_frequency) {
            calibration_error +=
                count as f64 / samples.len().max(1) as f64 * (confidence - frequency).abs();
        }
        reliability_bins.push(CalibrationBin {
            lower_bound_ppm: lower,
            upper_bound_ppm: upper,
            sample_count: count,
            mean_confidence,
            observed_frequency,
        });
    }

    let sample_count = samples.len();
    let score = if sample_count == 0 {
        None
    } else {
        Some(
            samples
                .iter()
                .map(|sample| brier(sample.probability_ppm, sample.outcome))
                .sum::<f64>()
                / sample_count as f64,
        )
    };
    let loss = if sample_count == 0 {
        None
    } else {
        Some(
            samples
                .iter()
                .map(|sample| log_loss(sample.probability_ppm, sample.outcome))
                .sum::<f64>()
                / sample_count as f64,
        )
    };
    let mean_confidence = if sample_count == 0 {
        None
    } else {
        Some(
            samples
                .iter()
                .map(|sample| sample.probability_ppm as f64 / 1_000_000.0)
                .sum::<f64>()
                / sample_count as f64,
        )
    };
    let observed_frequency = if sample_count == 0 {
        None
    } else {
        Some(samples.iter().filter(|sample| sample.outcome).count() as f64 / sample_count as f64)
    };

    CalibrationMetrics {
        sample_count,
        brier_score: score,
        log_loss: loss,
        expected_calibration_error: if sample_count == 0 {
            None
        } else {
            Some(calibration_error)
        },
        mean_confidence,
        observed_frequency,
        reliability_bins,
    }
}

fn collect_calibration<'a>(
    pairs: &'a [PairedDecision],
    role: BranchRole,
    after: bool,
    actor_presence: bool,
) -> Vec<CalibrationSample> {
    pairs
        .iter()
        .map(|pair| {
            let branch = match role {
                BranchRole::PassiveControl => &pair.passive_control,
                BranchRole::ActiveObservation => &pair.active_observation,
            };
            let belief = if after {
                &branch.belief_after
            } else {
                &branch.belief_before
            };
            let probability = if actor_presence {
                belief.actor_present_ppm
            } else {
                belief.crosses_within_horizon_ppm
            };
            let outcome = if actor_presence {
                branch.evaluator_outcome.actor_present
            } else {
                branch.evaluator_outcome.crosses_within_horizon
            };
            CalibrationSample {
                probability_ppm: probability,
                outcome,
            }
        })
        .collect()
}

fn branch_calibration(pairs: &[PairedDecision], role: BranchRole) -> BranchCalibration {
    BranchCalibration {
        role,
        actor_presence_before: calibration_metrics(&collect_calibration(pairs, role, false, true)),
        actor_presence_after: calibration_metrics(&collect_calibration(pairs, role, true, true)),
        crossing_before: calibration_metrics(&collect_calibration(pairs, role, false, false)),
        crossing_after: calibration_metrics(&collect_calibration(pairs, role, true, false)),
    }
}

fn summarize_cost(pairs: &[PairedDecision], role: BranchRole) -> CostSummary {
    let mut action_count = 0_usize;
    let mut duration = 0_u128;
    let mut energy = 0_u128;
    let mut distance = 0_u128;
    let mut max_risk: Option<u32> = None;
    for pair in pairs {
        let branch = match role {
            BranchRole::PassiveControl => &pair.passive_control,
            BranchRole::ActiveObservation => &pair.active_observation,
        };
        if branch.information_action != InformationActionKind::None {
            action_count += 1;
        }
        duration += u128::from(branch.action_cost.duration_us);
        energy += u128::from(branch.action_cost.energy_uj);
        distance += u128::from(branch.action_cost.maneuver_distance_mm);
        max_risk = Some(
            max_risk
                .unwrap_or(0)
                .max(branch.action_cost.estimated_residual_risk_ppm),
        );
    }
    CostSummary {
        action_count,
        total_duration_us: duration,
        total_energy_uj: energy,
        total_maneuver_distance_mm: distance,
        maximum_estimated_residual_risk_ppm: max_risk,
    }
}

fn validate_pair(
    pair: &PairedDecision,
    campaign: &Campaign,
    pair_ids: &mut BTreeSet<String>,
    branch_ids: &mut BTreeSet<String>,
    findings: &mut BTreeSet<Finding>,
) {
    if pair.pair_id.trim().is_empty() || pair.scenario_id.trim().is_empty() {
        findings.insert(Finding::EmptyIdentity);
    }
    if !pair_ids.insert(pair.pair_id.as_str()) {
        findings.insert(Finding::DuplicatePairId);
    }
    if pair.decision_tick == 0 {
        findings.insert(Finding::InvalidDecisionTick);
    }
    for digest in [
        pair.initial_world_snapshot_sha256.as_str(),
        pair.scenario_seed_sha256.as_str(),
        pair.exogenous_randomness_sha256.as_str(),
        pair.causal_branch_audit_sha256.as_str(),
    ] {
        if !is_sha256_hex(digest) {
            findings.insert(Finding::InvalidDigest);
        }
    }
    if pair.passive_control.branch_id.trim().is_empty()
        || pair.active_observation.branch_id.trim().is_empty()
    {
        findings.insert(Finding::EmptyIdentity);
    }
    if !branch_ids.insert(pair.passive_control.branch_id.as_str())
        || !branch_ids.insert(pair.active_observation.branch_id.as_str())
    {
        findings.insert(Finding::DuplicateBranchId);
    }
    if pair.passive_control.branch_id == pair.active_observation.branch_id {
        findings.insert(Finding::BranchIdentityMismatch);
    }
    if pair.passive_control.role != BranchRole::PassiveControl {
        findings.insert(Finding::PassiveBranchNotPassive);
    }
    if pair.active_observation.role != BranchRole::ActiveObservation {
        findings.insert(Finding::ActiveBranchWrongRole);
    }
    if pair.passive_control.information_action != InformationActionKind::None {
        findings.insert(Finding::PassiveBranchHasAction);
    }
    if pair.passive_control.decision_before != pair.active_observation.decision_before {
        findings.insert(Finding::RootDecisionMismatch);
    }
    let passive_outcome = &pair.passive_control.evaluator_outcome;
    let active_outcome = &pair.active_observation.evaluator_outcome;
    if passive_outcome.actor_present != active_outcome.actor_present
        || passive_outcome.crosses_within_horizon != active_outcome.crosses_within_horizon
    {
        findings.insert(Finding::HiddenOutcomeMismatch);
    }

    let action = &pair.active_observation;
    let cost = &action.action_cost;
    if action.information_action != InformationActionKind::None
        && !cost.admissible_under_policy_constraints
    {
        findings.insert(Finding::ActionNotAdmissible);
    }
    if cost.estimated_residual_risk_ppm > cost.max_admissible_risk_ppm
        || cost.max_admissible_risk_ppm > campaign.action_budget.maximum_residual_risk_ppm
    {
        findings.insert(Finding::RiskBudgetExceeded);
    }
    if cost.duration_us > campaign.action_budget.maximum_duration_us {
        findings.insert(Finding::ActionTimeBudgetExceeded);
    }
    if cost.energy_uj > campaign.action_budget.maximum_energy_uj {
        findings.insert(Finding::ActionEnergyBudgetExceeded);
    }
    if cost.maneuver_distance_mm > campaign.action_budget.maximum_maneuver_distance_mm {
        findings.insert(Finding::ActionDistanceBudgetExceeded);
    }
    if pair.no_safe_information_action_available {
        if action.information_action != InformationActionKind::None
            || action.decision_after == DrivingDecision::Proceed
            || active_outcome.conflict_boundary_encroached
        {
            findings.insert(Finding::NoSafeActionViolation);
        }
    }
    if matches!(
        action.information_action,
        InformationActionKind::SensorRequest | InformationActionKind::SafeReposition
    ) && (cost.estimated_residual_risk_ppm > cost.max_admissible_risk_ppm
        || !cost.admissible_under_policy_constraints)
    {
        findings.insert(Finding::UnsafeInformationSeeking);
    }
    if action.information_action == InformationActionKind::SafeReposition
        && (active_outcome.conflict_boundary_encroached
            || active_outcome.contact_or_collision
            || active_outcome.traffic_rule_violation)
    {
        findings.insert(Finding::UnsafeRepositionOutcome);
    }

    for belief in [
        &pair.passive_control.belief_before,
        &pair.passive_control.belief_after,
        &pair.active_observation.belief_before,
        &pair.active_observation.belief_after,
    ] {
        validate_belief(belief, findings);
    }
}

fn entropy_reduction(branch: &BranchTrace) -> f64 {
    entropy_nats(branch.belief_before.crosses_within_horizon_ppm)
        - entropy_nats(branch.belief_after.crosses_within_horizon_ppm)
}

fn pair_value_metrics(pair: &PairedDecision) -> PairValueMetrics {
    let y = pair
        .active_observation
        .evaluator_outcome
        .crosses_within_horizon;
    let passive = brier(
        pair.passive_control.belief_after.crosses_within_horizon_ppm,
        y,
    );
    let active = brier(
        pair.active_observation
            .belief_after
            .crosses_within_horizon_ppm,
        y,
    );
    PairValueMetrics {
        pair_id: pair.pair_id.clone(),
        passive_crossing_brier: passive,
        active_crossing_brier: active,
        active_minus_passive_brier: active - passive,
        passive_entropy_reduction_nats: entropy_reduction(&pair.passive_control),
        active_entropy_reduction_nats: entropy_reduction(&pair.active_observation),
        additional_entropy_reduction_nats: entropy_reduction(&pair.active_observation)
            - entropy_reduction(&pair.passive_control),
        expected_information_gain_nats: pair
            .active_observation
            .action_cost
            .expected_information_gain_milli_nats as f64
            / 1_000.0,
        decision_changed_from_root: pair.active_observation.decision_after
            != pair.active_observation.decision_before,
        active_action: pair.active_observation.information_action,
    }
}

pub fn audit_campaign(campaign: &Campaign) -> AuditReport {
    let mut findings = BTreeSet::new();
    if campaign.schema_version != 1 {
        findings.insert(Finding::SchemaVersionMismatch);
    }
    for identity in [
        campaign.campaign_id.as_str(),
        campaign.benchmark_revision.as_str(),
        campaign.evaluator_revision.as_str(),
        campaign.model_id.as_str(),
        campaign.model_revision.as_str(),
        campaign.hardware_id.as_str(),
        campaign.inference_precision.as_str(),
        campaign.power_profile.as_str(),
        campaign.energy_measurement_method.as_str(),
    ] {
        if identity.trim().is_empty() {
            findings.insert(Finding::EmptyIdentity);
        }
    }
    if campaign.model_status == ModelStatus::TrainedPolicy && campaign.checkpoint_sha256.is_none() {
        findings.insert(Finding::MissingCheckpointForTrainedPolicy);
    }
    if campaign
        .checkpoint_sha256
        .as_deref()
        .is_some_and(|value| !is_sha256_hex(value))
    {
        findings.insert(Finding::InvalidCheckpointDigest);
    }
    if campaign.policy_visible_field_ids.is_empty() {
        findings.insert(Finding::EmptyPolicyVisibleFields);
    }
    let mut fields = BTreeSet::new();
    for field in campaign
        .policy_visible_field_ids
        .iter()
        .chain(campaign.evaluator_only_field_ids.iter())
    {
        if field.trim().is_empty() || !fields.insert(field.as_str()) {
            findings.insert(Finding::DuplicateFieldId);
        }
    }
    let policy_fields = campaign
        .policy_visible_field_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if campaign
        .evaluator_only_field_ids
        .iter()
        .any(|field| policy_fields.contains(field.as_str()))
    {
        findings.insert(Finding::HiddenFieldLeak);
    }
    if campaign.pairs.len() as u64 != campaign.expected_pair_count {
        findings.insert(Finding::PairDenominatorMismatch);
    }

    let mut pair_ids = BTreeSet::new();
    let mut branch_ids = BTreeSet::new();
    for pair in &campaign.pairs {
        validate_pair(
            pair,
            campaign,
            &mut pair_ids,
            &mut branch_ids,
            &mut findings,
        );
    }

    let active_action_count = campaign
        .pairs
        .iter()
        .filter(|pair| pair.active_observation.information_action != InformationActionKind::None)
        .count();
    let decision_change_count = campaign
        .pairs
        .iter()
        .filter(|pair| {
            pair.active_observation.decision_before != pair.active_observation.decision_after
        })
        .count();
    let unsafe_information_actions = campaign
        .pairs
        .iter()
        .filter(|pair| {
            matches!(
                pair.active_observation.information_action,
                InformationActionKind::SensorRequest | InformationActionKind::SafeReposition
            ) && (pair
                .active_observation
                .action_cost
                .estimated_residual_risk_ppm
                > pair.active_observation.action_cost.max_admissible_risk_ppm
                || !pair
                    .active_observation
                    .action_cost
                    .admissible_under_policy_constraints)
        })
        .count() as u64;
    let no_safe_action_violations = campaign
        .pairs
        .iter()
        .filter(|pair| {
            pair.no_safe_information_action_available
                && (pair.active_observation.information_action != InformationActionKind::None
                    || pair.active_observation.decision_after == DrivingDecision::Proceed
                    || pair
                        .active_observation
                        .evaluator_outcome
                        .conflict_boundary_encroached)
        })
        .count() as u64;
    let collision_count = campaign
        .pairs
        .iter()
        .filter(|pair| {
            pair.active_observation
                .evaluator_outcome
                .contact_or_collision
                || pair.passive_control.evaluator_outcome.contact_or_collision
        })
        .count() as u64;
    let encroachment_count = campaign
        .pairs
        .iter()
        .filter(|pair| {
            pair.active_observation
                .evaluator_outcome
                .conflict_boundary_encroached
                || pair
                    .passive_control
                    .evaluator_outcome
                    .conflict_boundary_encroached
        })
        .count() as u64;
    let rule_violation_count = campaign
        .pairs
        .iter()
        .filter(|pair| {
            pair.active_observation
                .evaluator_outcome
                .traffic_rule_violation
                || pair
                    .passive_control
                    .evaluator_outcome
                    .traffic_rule_violation
        })
        .count() as u64;
    let pair_metrics = campaign
        .pairs
        .iter()
        .map(pair_value_metrics)
        .collect::<Vec<_>>();

    let passive_control = branch_calibration(&campaign.pairs, BranchRole::PassiveControl);
    let active_observation = branch_calibration(&campaign.pairs, BranchRole::ActiveObservation);
    let brier_delta = match (
        passive_control.crossing_after.brier_score,
        active_observation.crossing_after.brier_score,
    ) {
        (Some(passive), Some(active)) => Some(active - passive),
        _ => None,
    };
    let log_loss_delta = match (
        passive_control.crossing_after.log_loss,
        active_observation.crossing_after.log_loss,
    ) {
        (Some(passive), Some(active)) => Some(active - passive),
        _ => None,
    };

    AuditReport {
        schema_version: 1,
        campaign_id: campaign.campaign_id.clone(),
        disposition: if findings.is_empty() { "valid" } else { "rejected" }.to_string(),
        claim_ceiling: "Finite paired-branch record consistency and proper-score accounting only. This report does not authenticate simulator state, prove physical causality, establish trained semantic perception, or demonstrate public-road safety.".to_string(),
        expected_pair_count: campaign.expected_pair_count,
        observed_pair_count: campaign.pairs.len(),
        active_information_action_count: active_action_count,
        decision_change_count: decision_change_count,
        unsafe_information_action_count: unsafe_information_actions,
        no_safe_action_violations,
        collision_or_contact_count: collision_count,
        conflict_boundary_encroachment_count: encroachment_count,
        traffic_rule_violation_count: rule_violation_count,
        passive_control,
        active_observation,
        active_minus_passive_crossing_brier: brier_delta,
        active_minus_passive_crossing_log_loss: log_loss_delta,
        passive_cost: summarize_cost(&campaign.pairs, BranchRole::PassiveControl),
        active_cost: summarize_cost(&campaign.pairs, BranchRole::ActiveObservation),
        pair_metrics,
        findings: findings.into_iter().map(Finding::as_str).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn belief(actor: u32, crossing: u32, epistemic: u32, aleatoric: u32) -> BinaryBelief {
        BinaryBelief {
            actor_present_ppm: actor,
            crosses_within_horizon_ppm: crossing,
            epistemic_uncertainty_ppm: epistemic,
            aleatoric_uncertainty_ppm: aleatoric,
        }
    }

    fn cost(action: InformationActionKind) -> ActionCost {
        if action == InformationActionKind::None {
            ActionCost {
                duration_us: 0,
                energy_uj: 0,
                maneuver_distance_mm: 0,
                estimated_residual_risk_ppm: 0,
                max_admissible_risk_ppm: 100_000,
                expected_information_gain_milli_nats: 0,
                admissible_under_policy_constraints: true,
            }
        } else {
            ActionCost {
                duration_us: 500_000,
                energy_uj: 120,
                maneuver_distance_mm: 2_000,
                estimated_residual_risk_ppm: 50_000,
                max_admissible_risk_ppm: 100_000,
                expected_information_gain_milli_nats: 400,
                admissible_under_policy_constraints: true,
            }
        }
    }

    fn branch(
        id: &str,
        role: BranchRole,
        action: InformationActionKind,
        before: u32,
        after: u32,
        decision_before: DrivingDecision,
        decision_after: DrivingDecision,
        actor_present: bool,
        crosses: bool,
    ) -> BranchTrace {
        BranchTrace {
            branch_id: id.to_string(),
            role,
            information_action: action,
            decision_before,
            decision_after,
            belief_before: belief(500_000, before, 500_000, 100_000),
            belief_after: belief(
                if actor_present {
                    after
                } else {
                    1_000_000 - after
                },
                if crosses { after } else { 1_000_000 - after },
                100_000,
                100_000,
            ),
            action_cost: cost(action),
            evaluator_outcome: EvaluatorOutcome {
                actor_present,
                crosses_within_horizon: crosses,
                conflict_boundary_encroached: false,
                contact_or_collision: false,
                traffic_rule_violation: false,
                minimum_clearance_mm: Some(2_000),
            },
        }
    }

    fn pair(
        id: &str,
        action: InformationActionKind,
        no_safe_action: bool,
        passive_after: u32,
        active_after: u32,
        actor_present: bool,
        crosses: bool,
    ) -> PairedDecision {
        PairedDecision {
            pair_id: id.to_string(),
            scenario_id: "occluded-crosswalk-van-v1".to_string(),
            decision_tick: 12,
            initial_world_snapshot_sha256: "a".repeat(64),
            scenario_seed_sha256: "b".repeat(64),
            exogenous_randomness_sha256: "c".repeat(64),
            causal_branch_audit_sha256: "d".repeat(64),
            no_safe_information_action_available: no_safe_action,
            passive_control: branch(
                &format!("{id}-passive"),
                BranchRole::PassiveControl,
                InformationActionKind::None,
                450_000,
                passive_after,
                DrivingDecision::Wait,
                DrivingDecision::Wait,
                actor_present,
                crosses,
            ),
            active_observation: branch(
                &format!("{id}-active"),
                BranchRole::ActiveObservation,
                action,
                450_000,
                active_after,
                DrivingDecision::Wait,
                DrivingDecision::Yield,
                actor_present,
                crosses,
            ),
        }
    }

    fn campaign() -> Campaign {
        Campaign {
            schema_version: 1,
            campaign_id: "observability-occluded-crosswalk-fixture-v1".into(),
            benchmark_revision: "observability-guided-driving-v0.1".into(),
            evaluator_revision: "calibration-auditor-v1".into(),
            model_id: "synthetic-visibility-belief-reference".into(),
            model_status: ModelStatus::SyntheticReference,
            model_revision: "reference-v1".into(),
            checkpoint_sha256: None,
            hardware_id: "synthetic-fixture-only".into(),
            inference_precision: "not-measured".into(),
            power_profile: "not-measured".into(),
            energy_measurement_method: "synthetic-fixture-values".into(),
            policy_visible_field_ids: vec!["front-camera-rgb".into(), "ego-proprioception".into()],
            evaluator_only_field_ids: vec![
                "hidden-pedestrian-presence".into(),
                "hidden-pedestrian-intent".into(),
            ],
            expected_pair_count: 2,
            action_budget: ActionBudget {
                maximum_duration_us: 2_000_000,
                maximum_energy_uj: 500,
                maximum_maneuver_distance_mm: 5_000,
                maximum_residual_risk_ppm: 100_000,
            },
            pairs: vec![
                pair(
                    "occluded-pedestrian-present",
                    InformationActionKind::SafeReposition,
                    false,
                    600_000,
                    900_000,
                    true,
                    true,
                ),
                pair(
                    "no-safe-information-action",
                    InformationActionKind::None,
                    true,
                    500_000,
                    500_000,
                    true,
                    true,
                ),
            ],
        }
    }

    fn has(report: &AuditReport, expected: &str) -> bool {
        report.findings.iter().any(|finding| finding == expected)
    }

    #[test]
    fn accepts_paired_passive_and_safe_active_observation_fixture() {
        let report = audit_campaign(&campaign());
        assert_eq!(report.disposition, "valid");
        assert!(report.findings.is_empty());
        assert_eq!(report.expected_pair_count, 2);
        assert_eq!(report.observed_pair_count, 2);
        assert_eq!(report.active_information_action_count, 1);
        assert_eq!(report.passive_cost.total_duration_us, 0);
        assert_eq!(report.active_cost.total_duration_us, 500_000);
    }

    #[test]
    fn reports_proper_scoring_improvement_without_composite_score() {
        let report = audit_campaign(&campaign());
        assert!(report.active_minus_passive_crossing_brier.unwrap() < 0.0);
        assert!(report.active_minus_passive_crossing_log_loss.unwrap() < 0.0);
        assert!(report.pair_metrics[0].additional_entropy_reduction_nats > 0.0);
    }

    #[test]
    fn no_safe_information_action_must_not_be_replaced_by_proceed() {
        let mut input = campaign();
        input.pairs[1].active_observation.decision_after = DrivingDecision::Proceed;
        let report = audit_campaign(&input);
        assert!(has(&report, "no_safe_action_violation"));
    }

    #[test]
    fn no_safe_information_action_must_not_reposition_or_request_sensor() {
        let mut input = campaign();
        input.pairs[1].active_observation.information_action = InformationActionKind::SensorRequest;
        input.pairs[1].active_observation.action_cost = cost(InformationActionKind::SensorRequest);
        let report = audit_campaign(&input);
        assert!(has(&report, "no_safe_action_violation"));
    }

    #[test]
    fn hidden_evaluator_fields_must_not_leak_to_policy() {
        let mut input = campaign();
        input
            .policy_visible_field_ids
            .push("hidden-pedestrian-intent".into());
        let report = audit_campaign(&input);
        assert!(has(&report, "hidden_field_leak"));
    }

    #[test]
    fn a_safe_reposition_must_respect_risk_and_conflict_boundary() {
        let mut input = campaign();
        input.pairs[0]
            .active_observation
            .action_cost
            .estimated_residual_risk_ppm = 200_000;
        input.pairs[0]
            .active_observation
            .evaluator_outcome
            .conflict_boundary_encroached = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "risk_budget_exceeded"));
        assert!(has(&report, "unsafe_information_seeking"));
        assert!(has(&report, "unsafe_reposition_outcome"));
    }

    #[test]
    fn action_costs_are_budgeted_on_each_independent_axis() {
        let mut input = campaign();
        input.action_budget.maximum_duration_us = 100_000;
        input.action_budget.maximum_energy_uj = 10;
        input.action_budget.maximum_maneuver_distance_mm = 100;
        let report = audit_campaign(&input);
        assert!(has(&report, "action_time_budget_exceeded"));
        assert!(has(&report, "action_energy_budget_exceeded"));
        assert!(has(&report, "action_distance_budget_exceeded"));
    }

    #[test]
    fn hidden_outcome_labels_must_match_across_paired_branches() {
        let mut input = campaign();
        input.pairs[0]
            .active_observation
            .evaluator_outcome
            .crosses_within_horizon = false;
        let report = audit_campaign(&input);
        assert!(has(&report, "hidden_outcome_mismatch"));
    }

    #[test]
    fn denominator_and_checkpoint_provenance_are_fail_closed() {
        let mut input = campaign();
        input.expected_pair_count = 3;
        input.model_status = ModelStatus::TrainedPolicy;
        input.checkpoint_sha256 = None;
        let report = audit_campaign(&input);
        assert!(has(&report, "pair_denominator_mismatch"));
        assert!(has(&report, "missing_checkpoint_for_trained_policy"));
    }

    #[test]
    fn duplicate_branch_ids_and_pair_ids_are_rejected() {
        let mut input = campaign();
        input.pairs[1].pair_id = input.pairs[0].pair_id.clone();
        input.pairs[1].active_observation.branch_id =
            input.pairs[0].active_observation.branch_id.clone();
        let report = audit_campaign(&input);
        assert!(has(&report, "duplicate_pair_id"));
        assert!(has(&report, "duplicate_branch_id"));
    }

    #[test]
    fn proper_scores_match_known_binary_probability_examples() {
        assert!((brier(800_000, true) - 0.04).abs() < 1e-12);
        assert!((brier(200_000, false) - 0.04).abs() < 1e-12);
        assert!((log_loss(800_000, true) - -0.8_f64.ln()).abs() < 1e-12);
        assert!((log_loss(200_000, false) - -0.8_f64.ln()).abs() < 1e-12);
    }

    #[test]
    fn reliability_bins_include_probability_one_in_the_final_bin() {
        let samples = vec![
            CalibrationSample {
                probability_ppm: 0,
                outcome: false,
            },
            CalibrationSample {
                probability_ppm: 100_000,
                outcome: false,
            },
            CalibrationSample {
                probability_ppm: 999_999,
                outcome: true,
            },
            CalibrationSample {
                probability_ppm: 1_000_000,
                outcome: true,
            },
        ];
        let metrics = calibration_metrics(&samples);
        assert_eq!(metrics.reliability_bins[0].sample_count, 1);
        assert_eq!(metrics.reliability_bins[1].sample_count, 1);
        assert_eq!(metrics.reliability_bins[9].sample_count, 2);
        assert_eq!(
            metrics
                .reliability_bins
                .iter()
                .map(|bin| bin.sample_count)
                .sum::<usize>(),
            4
        );
    }

    #[test]
    fn report_keeps_uncertainty_information_value_cost_and_safety_as_separate_axes() {
        let report = audit_campaign(&campaign());
        let value = serde_json::to_value(report).unwrap();
        assert!(value.get("active_minus_passive_crossing_brier").is_some());
        assert!(value.get("active_cost").is_some());
        assert!(value.get("unsafe_information_action_count").is_some());
        assert!(value.get("collision_or_contact_count").is_some());
        assert!(value.get("composite_score").is_none());
        assert!(value.get("observability_score").is_none());
    }

    #[test]
    fn ece_reports_ten_fixed_reliability_bins() {
        let report = audit_campaign(&campaign());
        let bins = &report.active_observation.crossing_after.reliability_bins;
        assert_eq!(bins.len(), 10);
        assert_eq!(bins.iter().map(|bin| bin.sample_count).sum::<usize>(), 2);
    }

    #[test]
    fn uncertainty_components_are_bounded_independently() {
        let mut input = campaign();
        input.pairs[0]
            .active_observation
            .belief_after
            .epistemic_uncertainty_ppm = 1_000_001;
        let report = audit_campaign(&input);
        assert!(has(&report, "invalid_uncertainty"));
    }

    #[test]
    fn invalid_probabilities_are_rejected() {
        let mut input = campaign();
        input.pairs[0]
            .active_observation
            .belief_after
            .crosses_within_horizon_ppm = 1_000_001;
        let report = audit_campaign(&input);
        assert!(has(&report, "invalid_probability"));
    }

    #[test]
    fn passive_control_cannot_silently_take_an_information_action() {
        let mut input = campaign();
        input.pairs[0].passive_control.information_action = InformationActionKind::Wait;
        let report = audit_campaign(&input);
        assert!(has(&report, "passive_branch_not_passive"));
        assert!(has(&report, "passive_branch_has_action"));
    }

    #[test]
    fn repeated_policy_and_evaluator_field_ids_are_rejected() {
        let mut input = campaign();
        input
            .evaluator_only_field_ids
            .push("ego-proprioception".into());
        let report = audit_campaign(&input);
        assert!(has(&report, "duplicate_field_id"));
    }
}
