//! Aviation-specific calibration for Track V utility-estimate bounds.
//!
//! This crate evaluates precommitted utility estimates against sealed, independent
//! challenge outcomes. It does not fit the estimator, authenticate the source data,
//! establish independence, prove the physical model, authorize experiments, or certify
//! aviation safety. Its strongest positive status is limited to the declared challenge set.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Utility dimension whose conservative lower bound is being calibrated.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UtilityDimension {
    /// Decision-relevant predictive uncertainty reduction.
    UncertaintyReduction,
    /// Discrimination between independently defined model families.
    ModelDiscrimination,
    /// Reduction under a frozen robust/model-ambiguity set.
    WorstCaseRobustReduction,
    /// Relevance to the declared downstream engineering decision.
    DecisionRelevance,
}

/// Independent sampling unit represented by one challenge observation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IndependentUnit {
    /// Distinct flight.
    Flight,
    /// Distinct encounter family/event.
    Encounter,
    /// Distinct trajectory segment, only when segments are independently sampled.
    TrajectorySegment,
    /// Distinct independently measured experiment window.
    MeasurementWindow,
    /// Distinct model-family pair.
    ModelFamilyPair,
}

/// Evidence partition from which a utility observation originated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidencePartition {
    /// Used to construct or tune the estimator.
    AdaptiveDevelopment,
    /// Used to calibrate the estimator or its thresholds.
    Calibration,
    /// Used for non-confirmatory development.
    Development,
    /// Confirmatory partition; sealed from adaptive selection.
    ConfirmatorySealed,
    /// Untouched challenge partition reserved for final calibration assessment.
    ChallengeSealed,
}

/// Conservative utility lower bounds and uninformative-probability upper bound
/// copied from the Track V estimate receipt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PredictedUtility {
    /// Lower bound on uncertainty reduction.
    pub uncertainty_reduction_lower: f64,
    /// Lower bound on model discrimination.
    pub model_discrimination_lower: f64,
    /// Lower bound on robust reduction.
    pub worst_case_robust_reduction_lower: f64,
    /// Lower bound on decision relevance.
    pub decision_relevance_lower: f64,
    /// Upper bound on the probability of an uninformative outcome.
    pub uninformative_probability_upper: f64,
}

impl PredictedUtility {
    fn value(self, dimension: UtilityDimension) -> f64 {
        match dimension {
            UtilityDimension::UncertaintyReduction => self.uncertainty_reduction_lower,
            UtilityDimension::ModelDiscrimination => self.model_discrimination_lower,
            UtilityDimension::WorstCaseRobustReduction => self.worst_case_robust_reduction_lower,
            UtilityDimension::DecisionRelevance => self.decision_relevance_lower,
        }
    }

    fn is_valid(self) -> bool {
        bounded(self.uncertainty_reduction_lower)
            && bounded(self.model_discrimination_lower)
            && bounded(self.worst_case_robust_reduction_lower)
            && bounded(self.decision_relevance_lower)
            && bounded(self.uninformative_probability_upper)
    }
}

/// Exact Track V candidate and utility-estimate receipt identities being evaluated.
#[derive(Clone, Debug, PartialEq)]
pub struct UtilityClaim {
    /// Stable Track V candidate identity.
    pub candidate_id: String,
    /// Exact Track V utility-estimate receipt ID.
    pub utility_receipt_id: String,
    /// Exact payload digest of the utility-estimate receipt.
    pub utility_receipt_payload_sha256: String,
    /// Exact campaign identity.
    pub campaign_sha256: String,
    /// Exact model/sensor/control applicability identity.
    pub model_scope_sha256: String,
    /// Exact intended-use / risk-profile identity.
    pub intended_use_profile_sha256: String,
    /// Exact decision question.
    pub decision_question_id: String,
    /// Exact decision boundary / loss-function identity.
    pub decision_boundary_sha256: String,
    /// Exact model-hypothesis manifest.
    pub hypothesis_manifest_sha256: String,
    /// Exact robust/model-ambiguity-set manifest.
    pub robust_uncertainty_set_sha256: String,
    /// Exact adaptive-development data manifest used by the estimator.
    pub adaptive_data_manifest_sha256: String,
    /// Exact estimator build.
    pub estimator_build_sha256: String,
    /// Exact estimation-method identity.
    pub estimate_method_sha256: String,
    /// Claimed confidence level of the utility lower/upper bounds.
    pub estimate_confidence_level: f64,
    /// Sampling unit represented by the estimate's independent sample count.
    pub independent_unit: IndependentUnit,
    /// Declared number of independent samples used by the utility estimator.
    pub estimator_independent_sample_count: u64,
    /// Conservative utility bounds from the Track V receipt.
    pub predicted: PredictedUtility,
}

/// Realized utility metrics for one independent sealed challenge group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RealizedUtility {
    /// Realized uncertainty reduction.
    pub uncertainty_reduction: f64,
    /// Realized discrimination between the declared model families.
    pub model_discrimination: f64,
    /// Realized reduction under the frozen ambiguity set.
    pub worst_case_robust_reduction: f64,
    /// Realized decision relevance.
    pub decision_relevance: f64,
    /// Whether the experiment result was uninformative under the preregistered rule.
    pub uninformative: bool,
}

impl RealizedUtility {
    fn value(self, dimension: UtilityDimension) -> f64 {
        match dimension {
            UtilityDimension::UncertaintyReduction => self.uncertainty_reduction,
            UtilityDimension::ModelDiscrimination => self.model_discrimination,
            UtilityDimension::WorstCaseRobustReduction => self.worst_case_robust_reduction,
            UtilityDimension::DecisionRelevance => self.decision_relevance,
        }
    }

    fn is_valid(self) -> bool {
        bounded(self.uncertainty_reduction)
            && bounded(self.model_discrimination)
            && bounded(self.worst_case_robust_reduction)
            && bounded(self.decision_relevance)
    }
}

/// One outcome from one independent challenge group.
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeObservation {
    /// Unique observation record identity.
    pub observation_id: String,
    /// Candidate evaluated.
    pub candidate_id: String,
    /// Exact Track V utility-estimate receipt payload digest this outcome evaluates.
    pub utility_receipt_payload_sha256: String,
    /// Authenticated-source evidence receipt digest; signature and authority are external.
    pub measurement_evidence_receipt_sha256: String,
    /// Exact frozen challenge dataset manifest.
    pub challenge_manifest_sha256: String,
    /// Partition identity; only ChallengeSealed is eligible for final assessment.
    pub partition: EvidencePartition,
    /// Independent group identity, e.g. flight, encounter family or experiment window.
    pub independent_group_id: String,
    /// Declared aviation operating stratum.
    pub stratum_id: String,
    /// Realized outcome metrics.
    pub realized: RealizedUtility,
}

/// Frozen policy for the sealed calibration evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationPolicy {
    /// Digest of the calibration policy.
    pub policy_sha256: String,
    /// Exact campaign being assessed.
    pub expected_campaign_sha256: String,
    /// Exact model/sensor/control scope.
    pub expected_model_scope_sha256: String,
    /// Exact intended-use/risk profile.
    pub expected_intended_use_profile_sha256: String,
    /// Exact decision boundary.
    pub expected_decision_boundary_sha256: String,
    /// Exact model-hypothesis manifest.
    pub expected_hypothesis_manifest_sha256: String,
    /// Exact robust/model-ambiguity-set manifest.
    pub expected_robust_uncertainty_set_sha256: String,
    /// Exact estimator build.
    pub expected_estimator_build_sha256: String,
    /// Exact estimation-method identity.
    pub expected_estimate_method_sha256: String,
    /// Exact sealed challenge dataset manifest.
    pub challenge_manifest_sha256: String,
    /// Exact calibration-evaluator build identity recorded in the report.
    pub evaluator_build_sha256: String,
    /// Exact Wilson interval-method identity used for this assessment.
    pub interval_method_sha256: String,
    /// Required sampling unit for independent challenge groups.
    pub required_independent_unit: IndependentUnit,
    /// Metrics whose lower bounds must be calibrated.
    pub required_dimensions: Vec<UtilityDimension>,
    /// Operating strata assessed independently; they are never pooled to hide failure.
    pub required_strata: Vec<String>,
    /// Minimum number of independent groups required in each stratum.
    pub minimum_groups_per_stratum: u64,
    /// One-sided confidence level for Wilson score bounds (0.90, 0.95 or 0.99).
    pub one_sided_confidence_level: f64,
    /// Minimum Wilson lower confidence bound on empirical coverage of each predicted utility bound.
    pub minimum_coverage_lower_bound: f64,
}

/// Reasons a utility calibration package is structurally malformed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationError {
    /// Policy has missing identities, invalid limits or duplicate declarations.
    InvalidPolicy,
    /// Utility claim identity or estimate metadata is malformed.
    InvalidClaim,
    /// Claim scope differs from the frozen calibration policy.
    ClaimPolicyMismatch,
    /// Claim uses an estimator build/method not accepted by policy.
    EstimatorMismatch,
    /// Claim's sampling unit differs from policy.
    SamplingUnitMismatch,
    /// Observation has malformed identity, scope, partition or numeric value.
    InvalidObservation,
    /// Observation refers to another candidate or estimate receipt.
    ObservationClaimMismatch,
    /// Observation uses a manifest other than the exact sealed challenge manifest.
    ChallengeManifestMismatch,
    /// Non-challenge data was supplied to the sealed evaluator.
    SealedPartitionContamination,
    /// Observation record identity is duplicated.
    DuplicateObservationId,
    /// An independent group is reused, causing pseudoreplication.
    DuplicateIndependentGroup,
    /// Observation stratum is not part of the frozen policy.
    UnexpectedStratum,
    /// Duplicate or unknown required strata/dimensions in policy.
    InvalidStrataOrDimensions,
}

/// Disposition of the calibration evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationDisposition {
    /// Bounds met their preregistered coverage gates on the sealed challenge set only.
    CalibratedForDeclaredChallengeSetOnly,
    /// At least one required utility lower bound failed its coverage gate.
    NotCalibrated,
    /// At least one stratum lacks sufficient independent challenge groups.
    IndeterminateInsufficientData,
}

/// Empirical interval-coverage outcome for one metric within one stratum.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundCoverage {
    /// Metric calibrated.
    pub dimension: UtilityDimension,
    /// Independent challenge groups in the denominator.
    pub independent_group_count: u64,
    /// Groups where realized utility met or exceeded the claimed lower bound.
    pub covered_group_count: u64,
    /// Observed fraction above the claimed lower bound.
    pub empirical_coverage: f64,
    /// One-sided Wilson lower confidence bound on empirical coverage.
    pub wilson_lower_bound: f64,
    /// Whether the preregistered lower-bound coverage requirement was met.
    pub passed: bool,
}

/// Empirical check of the Track V upper bound on probability of an uninformative result.
#[derive(Clone, Debug, PartialEq)]
pub struct UninformativeRateAssessment {
    /// Independent challenge groups in the denominator.
    pub independent_group_count: u64,
    /// Number of uninformative outcomes.
    pub uninformative_group_count: u64,
    /// Empirical uninformative rate.
    pub empirical_rate: f64,
    /// One-sided Wilson upper confidence bound on the uninformative rate.
    pub wilson_upper_bound: f64,
    /// Track V claimed upper bound.
    pub claimed_upper_bound: f64,
    /// Whether the empirical upper confidence bound is within the Track V claim.
    pub passed: bool,
}

/// Per-stratum calibration result; no cross-stratum pooling occurs.
#[derive(Clone, Debug, PartialEq)]
pub struct StratumAssessment {
    /// Operating stratum.
    pub stratum_id: String,
    /// Independent group count.
    pub independent_group_count: u64,
    /// Per-dimension bound coverage.
    pub bound_coverage: Vec<BoundCoverage>,
    /// Uninformative event-rate result.
    pub uninformative_rate: UninformativeRateAssessment,
    /// Whether the stratum passed all calibration gates.
    pub passed: bool,
    /// Whether sample volume is sufficient for this stratum.
    pub sufficient_data: bool,
}

/// Full calibration report for one candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationReport {
    /// Candidate evaluated.
    pub candidate_id: String,
    /// Exact estimate receipt being assessed.
    pub utility_receipt_id: String,
    /// Exact sealed challenge dataset.
    pub challenge_manifest_sha256: String,
    /// Calibration disposition, bounded to the named challenge set.
    pub disposition: CalibrationDisposition,
    /// Independent observation-group total.
    pub independent_group_count: u64,
    /// Per-stratum reports.
    pub strata: Vec<StratumAssessment>,
    /// Exact policy used.
    pub policy_sha256: String,
    /// Exact calibration-evaluator build identity.
    pub evaluator_build_sha256: String,
    /// Exact interval-method identity.
    pub interval_method_sha256: String,
}

/// Independent resource-use dimensions for one realized decision alternative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionResourceUse {
    /// Setup/preparation minutes.
    pub setup_minutes: u64,
    /// Facility/operator minutes.
    pub facility_minutes: u64,
    /// CPU compute seconds.
    pub cpu_seconds: u64,
    /// Accelerator compute seconds.
    pub accelerator_seconds: u64,
    /// Energy joules.
    pub energy_joules: u64,
    /// Adjudication/review person-minutes.
    pub adjudication_person_minutes: u64,
}

/// Non-substitutable resource budget for one held-out decision case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionResourceBudget {
    /// Maximum setup/preparation minutes.
    pub setup_minutes: u64,
    /// Maximum facility/operator minutes.
    pub facility_minutes: u64,
    /// Maximum CPU compute seconds.
    pub cpu_seconds: u64,
    /// Maximum accelerator compute seconds.
    pub accelerator_seconds: u64,
    /// Maximum energy joules.
    pub energy_joules: u64,
    /// Maximum adjudication/review person-minutes.
    pub adjudication_person_minutes: u64,
}

impl DecisionResourceUse {
    fn within(self, budget: DecisionResourceBudget) -> bool {
        self.setup_minutes <= budget.setup_minutes
            && self.facility_minutes <= budget.facility_minutes
            && self.cpu_seconds <= budget.cpu_seconds
            && self.accelerator_seconds <= budget.accelerator_seconds
            && self.energy_joules <= budget.energy_joules
            && self.adjudication_person_minutes <= budget.adjudication_person_minutes
    }
}

/// Realized held-out outcome for one alternative in a decision case.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionAlternativeOutcome {
    /// Exact candidate identity.
    pub candidate_id: String,
    /// Whether the alternative satisfies the frozen engineering feasibility predicate.
    pub feasible: bool,
    /// Realized decision utility in [0, 1]; the no-action baseline is zero.
    pub decision_utility: f64,
    /// Realized, non-substitutable resource consumption.
    pub resource_use: DecisionResourceUse,
}

/// One independently adjudicated held-out decision case.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionCase {
    /// Stable independent case identity.
    pub case_id: String,
    /// Frozen non-substitutable resource budget.
    pub resource_budget: DecisionResourceBudget,
    /// Candidate selected by the evaluated policy; None means abstention/no action.
    pub selected_candidate_id: Option<String>,
    /// All alternatives evaluated against the same case and budget.
    pub alternatives: Vec<DecisionAlternativeOutcome>,
}

/// Per-case decision regret against the feasible, budget-admissible oracle.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionCaseResult {
    /// Independent case identity.
    pub case_id: String,
    /// Best feasible candidate under the same resource budget, if any.
    pub oracle_candidate_id: Option<String>,
    /// Candidate selected by the evaluated policy.
    pub selected_candidate_id: Option<String>,
    /// Best realized utility available under the frozen budget.
    pub oracle_utility: f64,
    /// Realized utility credited to the selected candidate; zero for abstention or invalid selection.
    pub selected_utility: f64,
    /// Non-negative regret relative to the feasible oracle.
    pub regret: f64,
    /// Whether the selected alternative was infeasible or over budget.
    pub false_feasible_selection: bool,
}

/// Aggregate held-out decision-regret report; no cross-case safety averaging is implied.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionRegretReport {
    /// Number of independent decision cases.
    pub case_count: u64,
    /// Number of cases where the policy abstained.
    pub abstention_count: u64,
    /// Number of selected alternatives that were infeasible or over budget.
    pub false_feasible_selection_count: u64,
    /// Mean regret across independent cases.
    pub mean_regret: f64,
    /// Maximum regret across independent cases.
    pub maximum_regret: f64,
    /// Mean utility of the feasible oracle.
    pub mean_oracle_utility: f64,
    /// Mean utility actually achieved by the evaluated policy.
    pub mean_selected_utility: f64,
    /// Per-case adjudication.
    pub cases: Vec<DecisionCaseResult>,
}

/// Structural errors in held-out decision-regret evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionRegretError {
    /// No cases were supplied or a case identity is empty.
    InvalidCase,
    /// A case identity is duplicated.
    DuplicateCase,
    /// A case has no alternatives or duplicate/empty candidate identities.
    InvalidAlternatives,
    /// A selected candidate is not present in the case's alternative set.
    SelectedCandidateMissing,
    /// A realized utility is non-finite or outside [0, 1].
    InvalidUtility,
}

/// Evaluate held-out decision regret against the best feasible alternative under the same
/// case-specific resource budget. Infeasible or over-budget selections receive zero utility
/// and are counted as false-feasible selections; they cannot be hidden by good average regret.
pub fn evaluate_decision_regret(
    cases: &[DecisionCase],
) -> Result<DecisionRegretReport, DecisionRegretError> {
    if cases.is_empty() {
        return Err(DecisionRegretError::InvalidCase);
    }
    let mut seen_cases = BTreeSet::new();
    let mut results = Vec::with_capacity(cases.len());
    let mut abstention_count = 0_u64;
    let mut false_feasible_selection_count = 0_u64;
    let mut regret_sum = 0.0;
    let mut oracle_sum = 0.0;
    let mut selected_sum = 0.0;
    let mut maximum_regret: f64 = 0.0;

    for case in cases {
        if case.case_id.trim().is_empty() {
            return Err(DecisionRegretError::InvalidCase);
        }
        if !seen_cases.insert(case.case_id.as_str()) {
            return Err(DecisionRegretError::DuplicateCase);
        }
        if case.alternatives.is_empty() {
            return Err(DecisionRegretError::InvalidAlternatives);
        }
        let mut candidate_ids = BTreeSet::new();
        for alternative in &case.alternatives {
            if alternative.candidate_id.trim().is_empty()
                || !candidate_ids.insert(alternative.candidate_id.as_str())
            {
                return Err(DecisionRegretError::InvalidAlternatives);
            }
            if !bounded(alternative.decision_utility) {
                return Err(DecisionRegretError::InvalidUtility);
            }
        }
        if case
            .selected_candidate_id
            .as_deref()
            .is_some_and(|selected| !candidate_ids.contains(selected))
        {
            return Err(DecisionRegretError::SelectedCandidateMissing);
        }

        let oracle = case
            .alternatives
            .iter()
            .filter(|alternative| {
                alternative.feasible && alternative.resource_use.within(case.resource_budget)
            })
            .max_by(|left, right| left.decision_utility.total_cmp(&right.decision_utility));
        let oracle_utility = oracle.map_or(0.0, |alternative| alternative.decision_utility);
        let oracle_candidate_id = oracle.map(|alternative| alternative.candidate_id.clone());

        let selected = case.selected_candidate_id.as_deref().and_then(|selected_id| {
            case.alternatives
                .iter()
                .find(|alternative| alternative.candidate_id == selected_id)
        });
        if selected.is_none() {
            abstention_count = abstention_count.saturating_add(1);
        }
        let false_feasible = selected.is_some_and(|alternative| {
            !alternative.feasible || !alternative.resource_use.within(case.resource_budget)
        });
        if false_feasible {
            false_feasible_selection_count = false_feasible_selection_count.saturating_add(1);
        }
        let selected_utility = selected
            .filter(|alternative| {
                alternative.feasible && alternative.resource_use.within(case.resource_budget)
            })
            .map_or(0.0, |alternative| alternative.decision_utility);
        let regret = (oracle_utility - selected_utility).max(0.0);
        regret_sum += regret;
        oracle_sum += oracle_utility;
        selected_sum += selected_utility;
        maximum_regret = maximum_regret.max(regret);
        results.push(DecisionCaseResult {
            case_id: case.case_id.clone(),
            oracle_candidate_id,
            selected_candidate_id: case.selected_candidate_id.clone(),
            oracle_utility,
            selected_utility,
            regret,
            false_feasible_selection: false_feasible,
        });
    }

    let count = cases.len() as f64;
    Ok(DecisionRegretReport {
        case_count: cases.len() as u64,
        abstention_count,
        false_feasible_selection_count,
        mean_regret: regret_sum / count,
        maximum_regret,
        mean_oracle_utility: oracle_sum / count,
        mean_selected_utility: selected_sum / count,
        cases: results,
    })
}

/// Wilson score interval one-sided critical value for supported confidence levels.
fn one_sided_z(confidence_level: f64) -> Option<f64> {
    match confidence_level {
        0.90 => Some(1.281_551_565_544_600_4),
        0.95 => Some(1.644_853_626_951_472_2),
        0.99 => Some(2.326_347_874_040_840_8),
        _ => None,
    }
}

fn bounded(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn wilson_lower_bound(successes: u64, total: u64, z: f64) -> f64 {
    let n = total as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let radius = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((center - radius) / denominator).max(0.0)
}

fn wilson_upper_bound(successes: u64, total: u64, z: f64) -> f64 {
    let n = total as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let radius = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((center + radius) / denominator).min(1.0)
}

fn validate_policy(policy: &CalibrationPolicy) -> Result<f64, CalibrationError> {
    if !valid_sha256(&policy.policy_sha256)
        || !valid_sha256(&policy.expected_campaign_sha256)
        || !valid_sha256(&policy.expected_model_scope_sha256)
        || !valid_sha256(&policy.expected_intended_use_profile_sha256)
        || !valid_sha256(&policy.expected_decision_boundary_sha256)
        || !valid_sha256(&policy.expected_hypothesis_manifest_sha256)
        || !valid_sha256(&policy.expected_robust_uncertainty_set_sha256)
        || !valid_sha256(&policy.expected_estimator_build_sha256)
        || !valid_sha256(&policy.expected_estimate_method_sha256)
        || !valid_sha256(&policy.challenge_manifest_sha256)
        || !valid_sha256(&policy.evaluator_build_sha256)
        || !valid_sha256(&policy.interval_method_sha256)
        || policy.minimum_groups_per_stratum == 0
        || !bounded(policy.minimum_coverage_lower_bound)
        || policy.required_strata.is_empty()
        || policy.required_dimensions.is_empty()
    {
        return Err(CalibrationError::InvalidPolicy);
    }

    let mut strata = BTreeSet::new();
    if policy.required_strata.iter().any(|value| {
        value.trim().is_empty() || !strata.insert(value.as_str())
    }) {
        return Err(CalibrationError::InvalidStrataOrDimensions);
    }
    let mut dimensions = BTreeSet::new();
    if policy.required_dimensions.iter().any(|value| !dimensions.insert(*value)) {
        return Err(CalibrationError::InvalidStrataOrDimensions);
    }

    one_sided_z(policy.one_sided_confidence_level).ok_or(CalibrationError::InvalidPolicy)
}

fn validate_claim(
    claim: &UtilityClaim,
    policy: &CalibrationPolicy,
) -> Result<(), CalibrationError> {
    if claim.candidate_id.trim().is_empty()
        || claim.utility_receipt_id.trim().is_empty()
        || !valid_sha256(&claim.utility_receipt_payload_sha256)
        || !valid_sha256(&claim.campaign_sha256)
        || !valid_sha256(&claim.model_scope_sha256)
        || !valid_sha256(&claim.intended_use_profile_sha256)
        || claim.decision_question_id.trim().is_empty()
        || !valid_sha256(&claim.decision_boundary_sha256)
        || !valid_sha256(&claim.hypothesis_manifest_sha256)
        || !valid_sha256(&claim.robust_uncertainty_set_sha256)
        || !valid_sha256(&claim.adaptive_data_manifest_sha256)
        || !valid_sha256(&claim.estimator_build_sha256)
        || !valid_sha256(&claim.estimate_method_sha256)
        || !bounded(claim.estimate_confidence_level)
        || claim.estimate_confidence_level == 0.0
        || claim.estimate_confidence_level == 1.0
        || claim.estimator_independent_sample_count == 0
        || !claim.predicted.is_valid()
    {
        return Err(CalibrationError::InvalidClaim);
    }

    if claim.campaign_sha256 != policy.expected_campaign_sha256
        || claim.model_scope_sha256 != policy.expected_model_scope_sha256
        || claim.intended_use_profile_sha256 != policy.expected_intended_use_profile_sha256
        || claim.decision_boundary_sha256 != policy.expected_decision_boundary_sha256
        || claim.hypothesis_manifest_sha256 != policy.expected_hypothesis_manifest_sha256
        || claim.robust_uncertainty_set_sha256 != policy.expected_robust_uncertainty_set_sha256
    {
        return Err(CalibrationError::ClaimPolicyMismatch);
    }
    if claim.estimator_build_sha256 != policy.expected_estimator_build_sha256
        || claim.estimate_method_sha256 != policy.expected_estimate_method_sha256
    {
        return Err(CalibrationError::EstimatorMismatch);
    }
    if claim.independent_unit != policy.required_independent_unit {
        return Err(CalibrationError::SamplingUnitMismatch);
    }
    if claim.adaptive_data_manifest_sha256 == policy.challenge_manifest_sha256
        || policy.minimum_coverage_lower_bound < claim.estimate_confidence_level
    {
        return Err(CalibrationError::ClaimPolicyMismatch);
    }
    Ok(())
}

/// Evaluate one Track V utility estimate against a sealed, independently grouped challenge set.
///
/// The Wilson interval is an approximate one-sided score interval over the supplied groups.
/// Independence and correct grouping remain source-owner assumptions. The evaluator does not
/// authenticate observation bytes or recompute the estimator's predicted utility.
pub fn evaluate_utility_claim(
    claim: &UtilityClaim,
    observations: &[ChallengeObservation],
    policy: &CalibrationPolicy,
) -> Result<CalibrationReport, CalibrationError> {
    let z = validate_policy(policy)?;
    validate_claim(claim, policy)?;

    let mut observation_ids = BTreeSet::new();
    let mut group_ids = BTreeSet::new();
    let required_strata: BTreeSet<&str> =
        policy.required_strata.iter().map(String::as_str).collect();
    let mut grouped: std::collections::BTreeMap<&str, Vec<&ChallengeObservation>> =
        std::collections::BTreeMap::new();

    for observation in observations {
        if observation.observation_id.trim().is_empty()
            || observation.candidate_id.trim().is_empty()
            || observation.independent_group_id.trim().is_empty()
            || observation.stratum_id.trim().is_empty()
            || !valid_sha256(&observation.utility_receipt_payload_sha256)
            || !valid_sha256(&observation.measurement_evidence_receipt_sha256)
            || !valid_sha256(&observation.challenge_manifest_sha256)
            || !observation.realized.is_valid()
        {
            return Err(CalibrationError::InvalidObservation);
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            return Err(CalibrationError::DuplicateObservationId);
        }
        if !group_ids.insert(observation.independent_group_id.as_str()) {
            return Err(CalibrationError::DuplicateIndependentGroup);
        }
        if observation.candidate_id != claim.candidate_id
            || observation.utility_receipt_payload_sha256 != claim.utility_receipt_payload_sha256
        {
            return Err(CalibrationError::ObservationClaimMismatch);
        }
        if observation.challenge_manifest_sha256 != policy.challenge_manifest_sha256 {
            return Err(CalibrationError::ChallengeManifestMismatch);
        }
        if observation.partition != EvidencePartition::ChallengeSealed {
            return Err(CalibrationError::SealedPartitionContamination);
        }
        if !required_strata.contains(observation.stratum_id.as_str()) {
            return Err(CalibrationError::UnexpectedStratum);
        }
        grouped
            .entry(observation.stratum_id.as_str())
            .or_default()
            .push(observation);
    }

    for stratum in &policy.required_strata {
        if !grouped.contains_key(stratum.as_str()) {
            return Err(CalibrationError::InvalidStrataOrDimensions);
        }
    }

    let mut strata = Vec::with_capacity(policy.required_strata.len());
    let mut any_insufficient = false;
    let mut any_failed_gate = false;
    let mut total_groups = 0_u64;

    for stratum in &policy.required_strata {
        let records = grouped
            .get(stratum.as_str())
            .ok_or(CalibrationError::InvalidStrataOrDimensions)?;
        let n = u64::try_from(records.len()).map_err(|_| CalibrationError::InvalidObservation)?;
        total_groups = total_groups.saturating_add(n);
        let sufficient_data = n >= policy.minimum_groups_per_stratum;
        if !sufficient_data {
            any_insufficient = true;
        }

        let mut bound_coverage = Vec::with_capacity(policy.required_dimensions.len());
        let mut stratum_passed = sufficient_data;

        for dimension in &policy.required_dimensions {
            let successes = records
                .iter()
                .filter(|record| record.realized.value(*dimension) >= claim.predicted.value(*dimension))
                .count() as u64;
            let empirical = successes as f64 / n as f64;
            let lower = wilson_lower_bound(successes, n, z);
            let passed = sufficient_data && lower >= policy.minimum_coverage_lower_bound;
            if sufficient_data && !passed {
                any_failed_gate = true;
            }
            stratum_passed &= passed;
            bound_coverage.push(BoundCoverage {
                dimension: *dimension,
                independent_group_count: n,
                covered_group_count: successes,
                empirical_coverage: empirical,
                wilson_lower_bound: lower,
                passed,
            });
        }

        let uninformative_count = records
            .iter()
            .filter(|record| record.realized.uninformative)
            .count() as u64;
        let empirical_uninformative = uninformative_count as f64 / n as f64;
        let uninformative_upper = wilson_upper_bound(uninformative_count, n, z);
        let uninformative_passed =
            sufficient_data && uninformative_upper <= claim.predicted.uninformative_probability_upper;
        if sufficient_data && !uninformative_passed {
            any_failed_gate = true;
        }
        stratum_passed &= uninformative_passed;

        strata.push(StratumAssessment {
            stratum_id: stratum.clone(),
            independent_group_count: n,
            bound_coverage,
            uninformative_rate: UninformativeRateAssessment {
                independent_group_count: n,
                uninformative_group_count: uninformative_count,
                empirical_rate: empirical_uninformative,
                wilson_upper_bound: uninformative_upper,
                claimed_upper_bound: claim.predicted.uninformative_probability_upper,
                passed: uninformative_passed,
            },
            passed: stratum_passed,
            sufficient_data,
        });
    }

    // A known failure in any sufficiently sampled stratum must not be hidden by
    // insufficient evidence in a different stratum.
    let disposition = if any_failed_gate {
        CalibrationDisposition::NotCalibrated
    } else if any_insufficient {
        CalibrationDisposition::IndeterminateInsufficientData
    } else {
        CalibrationDisposition::CalibratedForDeclaredChallengeSetOnly
    };

    Ok(CalibrationReport {
        candidate_id: claim.candidate_id.clone(),
        utility_receipt_id: claim.utility_receipt_id.clone(),
        challenge_manifest_sha256: policy.challenge_manifest_sha256.clone(),
        disposition,
        independent_group_count: total_groups,
        strata,
        policy_sha256: policy.policy_sha256.clone(),
        evaluator_build_sha256: policy.evaluator_build_sha256.clone(),
        interval_method_sha256: policy.interval_method_sha256.clone(),
    })
}
