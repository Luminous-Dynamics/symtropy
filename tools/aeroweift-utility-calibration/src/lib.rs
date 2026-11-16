//! Calibration and held-out decision-regret evaluation for Track V utility receipts.
//!
//! Consumes UtilityEstimateReceipt from Track V. It evaluates conservative lower
//! utility-bound coverage, upper uninformative-probability calibration, sharpness,
//! sampling support, and paired decision regret against frozen baselines.
//!
//! It does not compute EIG, posteriors, acquisition functions, sequential e-processes,
//! physical model validity, cryptographic trust, flight safety or operational authority.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use aeroweift_experiment_design::{EstimateSampleUnit, UtilityEstimateReceipt};

/// Role of one frozen data partition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatasetRole {
    /// Estimator fitting/calibration data.
    EstimatorCalibration,
    /// Adaptive development data that may inform candidate/estimator construction.
    AdaptiveDevelopment,
    /// Untouched confirmatory evaluation data.
    Confirmatory,
    /// Separately sealed challenge data.
    Challenge,
}

/// A frozen manifest of independent evaluation units and their strata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatasetManifest {
    /// Exact manifest digest; syntax is checked, bytes are not authenticated here.
    pub manifest_sha256: String,
    /// Declared role for the partition.
    pub role: DatasetRole,
    /// Independent sampling-unit identifiers, e.g. flights or encounters.
    pub independent_unit_ids: Vec<String>,
    /// Declared environment/configuration strata available in this partition.
    pub strata: Vec<String>,
}

/// Sampling design used to obtain held-out outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SamplingDesign {
    /// Prospective randomized sampling with known positive inclusion probabilities.
    ProspectiveRandomized,
    /// Adaptive selection with recorded, known positive inclusion propensities.
    AdaptiveWithKnownPropensity,
    /// Adaptive selection with unknown or unrecorded inclusion propensities.
    AdaptiveWithUnknownPropensity,
}

/// Utility dimension whose conservative forecast is being calibrated.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UtilityMetric {
    /// Decision-relevant predictive-uncertainty reduction.
    UncertaintyReduction,
    /// Discrimination among independently specified model hypotheses.
    ModelDiscrimination,
    /// Robust reduction under the frozen ambiguity set.
    WorstCaseRobustReduction,
    /// Relevance to the frozen engineering decision.
    DecisionRelevance,
}

/// One realized utility vector for one candidate and one independent unit.
#[derive(Clone, Debug, PartialEq)]
pub struct RealizedUtility {
    /// Realized predictive-uncertainty reduction, normalized to [0, 1].
    pub uncertainty_reduction: f64,
    /// Realized model-family discrimination, normalized to [0, 1].
    pub model_discrimination: f64,
    /// Realized worst-case reduction, normalized to [0, 1].
    pub worst_case_robust_reduction: f64,
    /// Realized decision relevance, normalized to [0, 1].
    pub decision_relevance: f64,
    /// Whether this outcome was uninformative under the preregistered rule.
    pub uninformative: bool,
}

impl RealizedUtility {
    fn value(&self, metric: UtilityMetric) -> f64 {
        match metric {
            UtilityMetric::UncertaintyReduction => self.uncertainty_reduction,
            UtilityMetric::ModelDiscrimination => self.model_discrimination,
            UtilityMetric::WorstCaseRobustReduction => self.worst_case_robust_reduction,
            UtilityMetric::DecisionRelevance => self.decision_relevance,
        }
    }

    fn is_valid(&self) -> bool {
        [
            self.uncertainty_reduction,
            self.model_discrimination,
            self.worst_case_robust_reduction,
            self.decision_relevance,
        ]
        .into_iter()
        .all(bounded_unit)
    }
}

/// Realized outcome attached to one candidate and one independent unit.
#[derive(Clone, Debug, PartialEq)]
pub struct UtilityOutcome {
    /// Candidate identity used by Track V.
    pub candidate_id: String,
    /// Independent flight/encounter/window/model-pair identifier.
    pub independent_unit_id: String,
    /// Exact required environment/configuration stratum.
    pub stratum_id: String,
    /// Exact evaluation dataset manifest.
    pub dataset_manifest_sha256: String,
    /// Dataset role; must match the frozen evaluation manifest.
    pub dataset_role: DatasetRole,
    /// Independent sampling-unit type.
    pub sample_unit: EstimateSampleUnit,
    /// Inclusion/selection propensity known before observing this outcome.
    pub selection_propensity: Option<f64>,
    /// Realized utility outcome.
    pub realized: RealizedUtility,
}

/// A selector's chosen candidate for one independent unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionDecision {
    /// Selector/baseline identity, e.g. Track V, random-feasible, parameter-EIG or expert-plan.
    pub selector_id: String,
    /// Independent unit for which the candidate was selected.
    pub independent_unit_id: String,
    /// Candidate selected by this policy.
    pub candidate_id: String,
    /// Exact shared non-substitutable resource-budget manifest.
    pub resource_budget_sha256: String,
}

/// Frozen calibration/evaluation policy, independent of submitted forecasts/outcomes.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationPolicy {
    /// Exact campaign identity.
    pub campaign_sha256: String,
    /// Exact aircraft/model/sensor/control scope.
    pub model_scope_sha256: String,
    /// Exact intended-use/risk profile.
    pub intended_use_profile_sha256: String,
    /// Exact decision question.
    pub decision_question_id: String,
    /// Exact decision boundary/loss identity.
    pub decision_boundary_sha256: String,
    /// Exact model-hypothesis manifest.
    pub hypothesis_manifest_sha256: String,
    /// Exact robust ambiguity-set manifest.
    pub robust_uncertainty_set_sha256: String,
    /// Exact adaptive-development manifest expected in Track V receipts.
    pub adaptive_data_manifest_sha256: String,
    /// Exact estimator build required by the campaign.
    pub estimator_build_sha256: String,
    /// Exact expected utility-estimation method.
    pub estimate_method_sha256: String,
    /// Frozen calibration/interval method identity for this evaluator.
    pub calibration_method_sha256: String,
    /// Independent sampling unit expected from Track V and held-out outcomes.
    pub sample_unit: EstimateSampleUnit,
    /// Confidence level required for one-sided calibration bounds.
    pub confidence_level: f64,
    /// Minimum Wilson lower confidence bound for each conservative utility lower bound.
    pub minimum_lower_bound_coverage: f64,
    /// Minimum independent outcome count per candidate and stratum.
    pub minimum_units_per_stratum: u64,
    /// Minimum Kish effective sample size per candidate and stratum.
    pub minimum_effective_sample_size: f64,
    /// Minimum permissible inclusion propensity for adaptive/randomized observations.
    pub minimum_selection_propensity: f64,
    /// Minimum average conservative utility lower bound; prevents vacuous forecasts.
    pub minimum_mean_lower_bound_sharpness: f64,
    /// Maximum mean upper bound on uninformative probability; prevents vacuous forecasts.
    pub maximum_mean_uninformative_upper_bound: f64,
    /// Maximum permissible mean decision regret for the primary selector.
    pub maximum_primary_mean_regret: f64,
    /// Minimum regret improvement over random-feasible selection.
    pub minimum_regret_improvement_vs_random: f64,
    /// The selector under evaluation.
    pub primary_selector_id: String,
    /// Required random-feasible baseline.
    pub random_baseline_selector_id: String,
    /// All selector/baseline IDs that must be evaluated on the paired campaign.
    pub required_selector_ids: Vec<String>,
    /// Exact resource budget identity shared by all compared selector decisions.
    pub resource_budget_sha256: String,
    /// Sampling/selection mechanism for the held-out outcomes.
    pub sampling_design: SamplingDesign,
    /// Frozen estimator-calibration data partition.
    pub estimator_calibration_manifest: DatasetManifest,
    /// Frozen adaptive-development partition.
    pub adaptive_development_manifest: DatasetManifest,
    /// Frozen untouched confirmatory or challenge partition.
    pub evaluation_manifest: DatasetManifest,
    /// Exact strata requiring calibration.
    pub required_strata: Vec<String>,
}

/// Reasons the supplied evidence is insufficient to make a calibration claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotCalibratableReason {
    /// Adaptive selection was performed without recorded propensities.
    UnknownSelectionPropensity,
    /// An outcome lacks a required inclusion probability.
    MissingSelectionPropensity,
    /// One or more inclusion probabilities fall below the frozen positivity floor.
    InsufficientSelectionOverlap,
    /// The effective independent sample size is below the frozen floor.
    InsufficientEffectiveSampleSize,
    /// One or more required candidate/stratum populations are under-sampled.
    InsufficientIndependentUnits,
    /// One or more evaluation units lack paired outcomes for candidate alternatives.
    IncompletePairedCampaign,
    /// One or more selector decisions are missing for an evaluation unit.
    MissingSelectorDecision,
}

/// Calibration gate that did not satisfy the frozen acceptance contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationFailure {
    /// Empirical lower-bound coverage is below the preregistered floor.
    LowerBoundUndercoverage,
    /// Empirical uninformative rate exceeds its submitted upper probability bound.
    UninformativeUpperBoundUndercoverage,
    /// Conservative lower utility bounds are too low to be decision-informative.
    BoundsTooVacuous,
    /// Submitted upper bound on uninformative probability is too wide.
    UninformativeUpperBoundTooWide,
    /// Primary selector exceeds its maximum paired mean regret.
    PrimaryDecisionRegretExceeded,
    /// Primary selector does not improve on random selection by the preregistered margin.
    InsufficientImprovementOverRandom,
}

/// Overall evaluation disposition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationDisposition {
    /// Calibration and decision-regret gates pass for the named population and policy only.
    CalibratedForDeclaredCampaign,
    /// Evidence support/positivity/sample-size does not permit a calibration claim.
    NotCalibratable,
    /// Evidence is sufficient, but one or more preregistered gates fail.
    AcceptanceCriteriaFailed,
}

/// One-sided confidence interval for a weighted Bernoulli proportion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProportionInterval {
    /// Weighted empirical success proportion.
    pub estimate: f64,
    /// Effective independent sample size.
    pub effective_sample_size: f64,
    /// One-sided Wilson lower confidence bound.
    pub lower: f64,
    /// One-sided Wilson upper confidence bound.
    pub upper: f64,
}

/// Coverage for one metric in one candidate/stratum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricCoverage {
    /// Metric being calibrated.
    pub metric: UtilityMetric,
    /// Number of observed independent units.
    pub independent_unit_count: usize,
    /// Conservative lower-bound coverage interval.
    pub coverage: ProportionInterval,
}

/// Calibration result for one candidate and one stratum.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateStratumCalibration {
    /// Candidate identity.
    pub candidate_id: String,
    /// Named stratum.
    pub stratum_id: String,
    /// Independent outcome count.
    pub independent_unit_count: usize,
    /// Kish effective sample size after propensity weighting.
    pub effective_sample_size: f64,
    /// Coverage of all four conservative utility lower bounds.
    pub metric_coverage: Vec<MetricCoverage>,
    /// Coverage test for the submitted upper probability of an uninformative result.
    pub uninformative_rate: ProportionInterval,
    /// Mean submitted conservative lower utility bound; higher is sharper.
    pub mean_lower_bound_sharpness: f64,
    /// Submitted upper bound on the uninformative probability.
    pub submitted_uninformative_upper_bound: f64,
}

/// Paired realized decision regret for one selector/baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectorRegret {
    /// Selector identity.
    pub selector_id: String,
    /// Number of paired independent evaluation units.
    pub paired_unit_count: usize,
    /// Mean regret versus the best realized candidate in the finite paired candidate set.
    pub mean_regret: f64,
    /// Maximum regret observed across the paired units.
    pub maximum_regret: f64,
}

/// Result for the exact estimator/policy/held-out population.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationResult {
    /// Overall calibration disposition.
    pub disposition: CalibrationDisposition,
    /// Per-candidate/stratum calibration results.
    pub candidate_strata: Vec<CandidateStratumCalibration>,
    /// Paired decision regret for all required selectors.
    pub selector_regrets: Vec<SelectorRegret>,
    /// Primary selector mean regret, when calibratable.
    pub primary_mean_regret: Option<f64>,
    /// Random-baseline mean regret, when calibratable.
    pub random_mean_regret: Option<f64>,
    /// Random regret minus primary regret; positive means the primary selector is better.
    pub improvement_over_random: Option<f64>,
    /// Explicit reasons no calibration claim can be made.
    pub not_calibratable_reasons: Vec<NotCalibratableReason>,
    /// Acceptance gates that failed despite sufficient evidence.
    pub failures: Vec<CalibrationFailure>,
    /// Exact policy and method identities used.
    pub policy_sha256: String,
    /// Exact evaluation dataset manifest.
    pub evaluation_manifest_sha256: String,
}

/// Structural input or policy error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationError {
    /// Policy fields, thresholds or required identities are invalid.
    InvalidPolicy,
    /// Dataset manifest identity, role or membership is invalid.
    InvalidDatasetManifest,
    /// Calibration, adaptive-development and evaluation partitions overlap.
    DatasetLeakage,
    /// Forecast receipt identity, scope, bounds or estimator metadata is malformed.
    InvalidForecastReceipt,
    /// Duplicate candidate forecast.
    DuplicateCandidateForecast,
    /// Outcome references an unknown candidate.
    UnknownCandidate,
    /// Outcome scope, manifest, partition, sample unit or numeric values are invalid.
    InvalidOutcome,
    /// Duplicate independent candidate/stratum unit; pseudoreplication is not admitted.
    DuplicateIndependentOutcome,
    /// Selection decision is duplicated or does not match the frozen selector/budget.
    InvalidSelectionDecision,
}

/// Check digest syntax only. Cryptographic verification remains a Track Q/Mycelix responsibility.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn bounded_unit(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn supported_z(confidence_level: f64) -> Option<f64> {
    if (confidence_level - 0.90).abs() < 1e-9 {
        Some(1.281_551_565_544_600_4)
    } else if (confidence_level - 0.95).abs() < 1e-9 {
        Some(1.644_853_626_951_472_2)
    } else if (confidence_level - 0.99).abs() < 1e-9 {
        Some(2.326_347_874_040_840_8)
    } else {
        None
    }
}

fn wilson_one_sided(proportion: f64, effective_n: f64, z: f64) -> ProportionInterval {
    let n = effective_n.max(1.0);
    let p = proportion.clamp(0.0, 1.0);
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half = z / denominator * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ProportionInterval {
        estimate: p,
        effective_sample_size: effective_n,
        lower: (center - half).clamp(0.0, 1.0),
        upper: (center + half).clamp(0.0, 1.0),
    }
}

fn validate_manifest(
    manifest: &DatasetManifest,
    expected_role: DatasetRole,
) -> Result<(), CalibrationError> {
    let unique_units: BTreeSet<&str> = manifest
        .independent_unit_ids
        .iter()
        .map(String::as_str)
        .collect();
    let unique_strata: BTreeSet<&str> =
        manifest.strata.iter().map(String::as_str).collect();
    if !is_sha256(&manifest.manifest_sha256)
        || manifest.role != expected_role
        || manifest.independent_unit_ids.is_empty()
        || unique_units.len() != manifest.independent_unit_ids.len()
        || manifest
            .independent_unit_ids
            .iter()
            .any(|id| id.trim().is_empty())
        || manifest.strata.is_empty()
        || unique_strata.len() != manifest.strata.len()
        || manifest.strata.iter().any(|stratum| stratum.trim().is_empty())
    {
        return Err(CalibrationError::InvalidDatasetManifest);
    }
    Ok(())
}

fn manifests_overlap(left: &DatasetManifest, right: &DatasetManifest) -> bool {
    let right_ids: BTreeSet<&str> = right
        .independent_unit_ids
        .iter()
        .map(String::as_str)
        .collect();
    left.independent_unit_ids
        .iter()
        .any(|id| right_ids.contains(id.as_str()))
}

fn validate_policy(policy: &CalibrationPolicy) -> Result<f64, CalibrationError> {
    let valid_ids = [
        &policy.campaign_sha256,
        &policy.model_scope_sha256,
        &policy.intended_use_profile_sha256,
        &policy.decision_boundary_sha256,
        &policy.hypothesis_manifest_sha256,
        &policy.robust_uncertainty_set_sha256,
        &policy.adaptive_data_manifest_sha256,
        &policy.estimator_build_sha256,
        &policy.estimate_method_sha256,
        &policy.calibration_method_sha256,
        &policy.resource_budget_sha256,
        &policy.policy_sha256,
    ];
    if valid_ids.iter().any(|value| !is_sha256(value))
        || policy.decision_question_id.trim().is_empty()
        || policy.primary_selector_id.trim().is_empty()
        || policy.random_baseline_selector_id.trim().is_empty()
        || policy.primary_selector_id == policy.random_baseline_selector_id
        || policy.required_selector_ids.len() < 2
        || policy.required_selector_ids.iter().any(|id| id.trim().is_empty())
        || policy.required_selector_ids.iter().collect::<BTreeSet<_>>().len()
            != policy.required_selector_ids.len()
        || !policy.required_selector_ids.contains(&policy.primary_selector_id)
        || !policy
            .required_selector_ids
            .contains(&policy.random_baseline_selector_id)
        || !bounded_unit(policy.minimum_lower_bound_coverage)
        || policy.minimum_lower_bound_coverage == 0.0
        || !bounded_unit(policy.minimum_selection_propensity)
        || policy.minimum_selection_propensity == 0.0
        || !bounded_unit(policy.minimum_mean_lower_bound_sharpness)
        || !bounded_unit(policy.maximum_mean_uninformative_upper_bound)
        || !bounded_unit(policy.maximum_primary_mean_regret)
        || !bounded_unit(policy.minimum_regret_improvement_vs_random)
        || policy.minimum_units_per_stratum == 0
        || !policy.minimum_effective_sample_size.is_finite()
        || policy.minimum_effective_sample_size < 1.0
    {
        return Err(CalibrationError::InvalidPolicy);
    }
    let z = supported_z(policy.confidence_level).ok_or(CalibrationError::InvalidPolicy)?;
    if policy.required_strata.is_empty()
        || policy.required_strata.iter().collect::<BTreeSet<_>>().len()
            != policy.required_strata.len()
        || !policy.required_strata.iter().all(|stratum| {
            policy.evaluation_manifest.strata.contains(stratum)
        })
    {
        return Err(CalibrationError::InvalidPolicy);
    }
    Ok(z)
}

fn not_calibratable(
    policy: &CalibrationPolicy,
    reasons: Vec<NotCalibratableReason>,
) -> CalibrationResult {
    CalibrationResult {
        disposition: CalibrationDisposition::NotCalibratable,
        candidate_strata: Vec::new(),
        selector_regrets: Vec::new(),
        primary_mean_regret: None,
        random_mean_regret: None,
        improvement_over_random: None,
        not_calibratable_reasons: reasons,
        failures: Vec::new(),
        policy_sha256: policy.policy_sha256.clone(),
        evaluation_manifest_sha256: policy.evaluation_manifest.manifest_sha256.clone(),
    }
}

fn forecast_lower(receipt: &UtilityEstimateReceipt, metric: UtilityMetric) -> f64 {
    match metric {
        UtilityMetric::UncertaintyReduction => receipt.expected_uncertainty_reduction,
        UtilityMetric::ModelDiscrimination => receipt.expected_model_discrimination,
        UtilityMetric::WorstCaseRobustReduction => receipt.worst_case_robust_reduction,
        UtilityMetric::DecisionRelevance => receipt.decision_relevance,
    }
}

fn validate_forecast(
    forecast: &UtilityEstimateReceipt,
    candidate_id: &str,
    policy: &CalibrationPolicy,
) -> Result<(), CalibrationError> {
    if forecast.candidate_id != candidate_id
        || forecast.campaign_sha256 != policy.campaign_sha256
        || forecast.model_scope_sha256 != policy.model_scope_sha256
        || forecast.intended_use_profile_sha256 != policy.intended_use_profile_sha256
        || forecast.decision_question_id != policy.decision_question_id
        || forecast.decision_boundary_sha256 != policy.decision_boundary_sha256
        || forecast.hypothesis_manifest_sha256 != policy.hypothesis_manifest_sha256
        || forecast.robust_uncertainty_set_sha256 != policy.robust_uncertainty_set_sha256
        || forecast.adaptive_data_manifest_sha256 != policy.adaptive_data_manifest_sha256
        || forecast.estimator_build_sha256 != policy.estimator_build_sha256
        || forecast.estimate_method_sha256 != policy.estimate_method_sha256
        || forecast.sample_unit != policy.sample_unit
        || (forecast.confidence_level - policy.confidence_level).abs() > 1e-9
        || forecast.independent_unit_count < policy.minimum_units_per_stratum
        || forecast.receipt_id.trim().is_empty()
        || forecast.issuer_id.trim().is_empty()
        || !is_sha256(&forecast.payload_sha256)
        || !bounded_unit(forecast.probability_uninformative)
        || ![
            forecast.expected_uncertainty_reduction,
            forecast.expected_model_discrimination,
            forecast.worst_case_robust_reduction,
            forecast.decision_relevance,
        ]
        .into_iter()
        .all(bounded_unit)
    {
        return Err(CalibrationError::InvalidForecastReceipt);
    }
    Ok(())
}

fn weighted_proportion<F>(
    outcomes: &[&UtilityOutcome],
    policy: &CalibrationPolicy,
    predicate: F,
) -> Result<(f64, f64), NotCalibratableReason>
where
    F: Fn(&UtilityOutcome) -> bool,
{
    let mut sum_weights = 0.0;
    let mut sum_squared_weights = 0.0;
    let mut weighted_success = 0.0;
    for outcome in outcomes {
        let Some(propensity) = outcome.selection_propensity else {
            return Err(NotCalibratableReason::MissingSelectionPropensity);
        };
        if !propensity.is_finite() || propensity <= 0.0 || propensity > 1.0 {
            return Err(NotCalibratableReason::InsufficientSelectionOverlap);
        }
        if propensity < policy.minimum_selection_propensity {
            return Err(NotCalibratableReason::InsufficientSelectionOverlap);
        }
        let weight = 1.0 / propensity;
        sum_weights += weight;
        sum_squared_weights += weight * weight;
        if predicate(outcome) {
            weighted_success += weight;
        }
    }
    if sum_weights <= 0.0 || sum_squared_weights <= 0.0 {
        return Err(NotCalibratableReason::InsufficientEffectiveSampleSize);
    }
    let effective_n = sum_weights * sum_weights / sum_squared_weights;
    if effective_n < policy.minimum_effective_sample_size {
        return Err(NotCalibratableReason::InsufficientEffectiveSampleSize);
    }
    Ok(((weighted_success / sum_weights).clamp(0.0, 1.0), effective_n))
}

/// Evaluate Track V utility forecasts against independent held-out outcomes.
///
/// For lower-bound claims, success means realized utility is at least the submitted
/// conservative lower bound. For the uninformative-probability claim, the one-sided
/// upper confidence bound on the realized uninformative rate must not exceed the
/// submitted probability upper bound.
///
/// Adaptive outcomes require known positive propensities. Weighted intervals use
/// inverse-propensity weighting with a Kish effective-sample-size Wilson approximation.
pub fn evaluate_calibration(
    forecasts: &[(String, UtilityEstimateReceipt)],
    outcomes: &[UtilityOutcome],
    decisions: &[SelectionDecision],
    policy: &CalibrationPolicy,
) -> Result<CalibrationResult, CalibrationError> {
    let z = validate_policy(policy)?;

    validate_manifest(
        &policy.estimator_calibration_manifest,
        DatasetRole::EstimatorCalibration,
    )?;
    validate_manifest(
        &policy.adaptive_development_manifest,
        DatasetRole::AdaptiveDevelopment,
    )?;
    validate_manifest(
        &policy.evaluation_manifest,
        policy.evaluation_manifest.role,
    )?;
    if !matches!(
        policy.evaluation_manifest.role,
        DatasetRole::Confirmatory | DatasetRole::Challenge
    ) {
        return Err(CalibrationError::InvalidDatasetManifest);
    }
    if policy.estimator_calibration_manifest.manifest_sha256
        == policy.adaptive_development_manifest.manifest_sha256
        || policy.estimator_calibration_manifest.manifest_sha256
            == policy.evaluation_manifest.manifest_sha256
        || policy.adaptive_development_manifest.manifest_sha256
            == policy.evaluation_manifest.manifest_sha256
        || manifests_overlap(
            &policy.estimator_calibration_manifest,
            &policy.adaptive_development_manifest,
        )
        || manifests_overlap(
            &policy.estimator_calibration_manifest,
            &policy.evaluation_manifest,
        )
        || manifests_overlap(
            &policy.adaptive_development_manifest,
            &policy.evaluation_manifest,
        )
        || policy.adaptive_data_manifest_sha256
            != policy.adaptive_development_manifest.manifest_sha256
    {
        return Err(CalibrationError::DatasetLeakage);
    }

    if forecasts.len() < 2 || outcomes.is_empty() || decisions.is_empty() {
        return Err(CalibrationError::InvalidPolicy);
    }
    let mut forecast_map: BTreeMap<&str, &UtilityEstimateReceipt> = BTreeMap::new();
    for (candidate_id, receipt) in forecasts {
        if candidate_id.trim().is_empty()
            || forecast_map.insert(candidate_id.as_str(), receipt).is_some()
        {
            return Err(CalibrationError::DuplicateCandidateForecast);
        }
        validate_forecast(receipt, candidate_id, policy)?;
    }

    let mut outcome_keys = BTreeSet::new();
    let mut by_candidate_stratum: BTreeMap<(&str, &str), Vec<&UtilityOutcome>> = BTreeMap::new();
    let mut by_unit: BTreeMap<&str, BTreeMap<&str, &UtilityOutcome>> = BTreeMap::new();
    for outcome in outcomes {
        if !forecast_map.contains_key(outcome.candidate_id.as_str()) {
            return Err(CalibrationError::UnknownCandidate);
        }
        if outcome.candidate_id.trim().is_empty()
            || outcome.independent_unit_id.trim().is_empty()
            || !policy
                .evaluation_manifest
                .independent_unit_ids
                .contains(&outcome.independent_unit_id)
            || outcome.dataset_manifest_sha256
                != policy.evaluation_manifest.manifest_sha256
            || outcome.dataset_role != policy.evaluation_manifest.role
            || outcome.sample_unit != policy.sample_unit
            || !policy.required_strata.contains(&outcome.stratum_id)
            || !outcome.realized.is_valid()
        {
            return Err(CalibrationError::InvalidOutcome);
        }
        if !outcome_keys.insert((
            outcome.candidate_id.as_str(),
            outcome.independent_unit_id.as_str(),
            outcome.stratum_id.as_str(),
        )) {
            return Err(CalibrationError::DuplicateIndependentOutcome);
        }
        by_candidate_stratum
            .entry((outcome.candidate_id.as_str(), outcome.stratum_id.as_str()))
            .or_default()
            .push(outcome);
        if by_unit
            .entry(outcome.independent_unit_id.as_str())
            .or_default()
            .insert(outcome.candidate_id.as_str(), outcome)
            .is_some()
        {
            return Err(CalibrationError::DuplicateIndependentOutcome);
        }
    }

    if policy.sampling_design == SamplingDesign::AdaptiveWithUnknownPropensity {
        return Ok(not_calibratable(
            policy,
            vec![NotCalibratableReason::UnknownSelectionPropensity],
        ));
    }

    let mut unit_strata: BTreeMap<&str, &str> = BTreeMap::new();
    for outcome in outcomes {
        if let Some(previous) = unit_strata.insert(
            outcome.independent_unit_id.as_str(),
            outcome.stratum_id.as_str(),
        ) && previous != outcome.stratum_id
        {
            return Err(CalibrationError::InvalidOutcome);
        }
    }

    let candidate_ids: Vec<&str> = forecast_map.keys().copied().collect();
    let mut candidate_strata = Vec::new();
    let mut not_calibratable_reasons = BTreeSet::new();
    let mut failures = BTreeSet::new();

    for candidate_id in &candidate_ids {
        let forecast = forecast_map[candidate_id];
        for stratum in &policy.required_strata {
            let Some(sample) = by_candidate_stratum.get(&(*candidate_id, stratum.as_str())) else {
                not_calibratable_reasons
                    .insert(NotCalibratableReason::InsufficientIndependentUnits);
                continue;
            };
            if sample.len() < policy.minimum_units_per_stratum as usize {
                not_calibratable_reasons
                    .insert(NotCalibratableReason::InsufficientIndependentUnits);
                continue;
            }

            let metrics = [
                UtilityMetric::UncertaintyReduction,
                UtilityMetric::ModelDiscrimination,
                UtilityMetric::WorstCaseRobustReduction,
                UtilityMetric::DecisionRelevance,
            ];
            let mut effective_n = f64::INFINITY;
            let mut metric_coverage = Vec::new();
            for metric in metrics {
                let lower = forecast_lower(forecast, metric);
                let result = weighted_proportion(sample, policy, |outcome| {
                    outcome.realized.value(metric) >= lower
                });
                let (estimate, n_eff) = match result {
                    Ok(value) => value,
                    Err(reason) => {
                        not_calibratable_reasons.insert(reason);
                        continue;
                    }
                };
                effective_n = effective_n.min(n_eff);
                let interval = wilson_one_sided(estimate, n_eff, z);
                if interval.lower < policy.minimum_lower_bound_coverage {
                    failures.insert(CalibrationFailure::LowerBoundUndercoverage);
                }
                metric_coverage.push(MetricCoverage {
                    metric,
                    independent_unit_count: sample.len(),
                    coverage: interval,
                });
            }

            let uninformative_result = weighted_proportion(sample, policy, |outcome| {
                outcome.realized.uninformative
            });
            let uninformative_rate = match uninformative_result {
                Ok((estimate, n_eff)) => {
                    effective_n = effective_n.min(n_eff);
                    wilson_one_sided(estimate, n_eff, z)
                }
                Err(reason) => {
                    not_calibratable_reasons.insert(reason);
                    continue;
                }
            };
            if uninformative_rate.upper > forecast.probability_uninformative {
                failures.insert(CalibrationFailure::UninformativeUpperBoundUndercoverage);
            }
            let mean_lower_bound_sharpness = metrics
                .iter()
                .map(|metric| forecast_lower(forecast, *metric))
                .sum::<f64>()
                / metrics.len() as f64;
            if mean_lower_bound_sharpness < policy.minimum_mean_lower_bound_sharpness {
                failures.insert(CalibrationFailure::BoundsTooVacuous);
            }
            if forecast.probability_uninformative
                > policy.maximum_mean_uninformative_upper_bound
            {
                failures.insert(CalibrationFailure::UninformativeUpperBoundTooWide);
            }
            candidate_strata.push(CandidateStratumCalibration {
                candidate_id: (*candidate_id).to_owned(),
                stratum_id: stratum.clone(),
                independent_unit_count: sample.len(),
                effective_sample_size: effective_n,
                metric_coverage,
                uninformative_rate,
                mean_lower_bound_sharpness,
                submitted_uninformative_upper_bound: forecast.probability_uninformative,
            });
        }
    }

    if !not_calibratable_reasons.is_empty() {
        return Ok(not_calibratable(
            policy,
            not_calibratable_reasons.into_iter().collect(),
        ));
    }

    let expected_units: BTreeSet<&str> = policy
        .evaluation_manifest
        .independent_unit_ids
        .iter()
        .map(String::as_str)
        .collect();
    let observed_units: BTreeSet<&str> = by_unit.keys().copied().collect();
    if expected_units != observed_units {
        return Ok(not_calibratable(
            policy,
            vec![NotCalibratableReason::IncompletePairedCampaign],
        ));
    }

    for unit in &expected_units {
        let Some(candidate_results) = by_unit.get(*unit) else {
            return Ok(not_calibratable(
                policy,
                vec![NotCalibratableReason::IncompletePairedCampaign],
            ));
        };
        if candidate_results.len() != candidate_ids.len()
            || candidate_ids
                .iter()
                .any(|candidate| !candidate_results.contains_key(candidate))
        {
            return Ok(not_calibratable(
                policy,
                vec![NotCalibratableReason::IncompletePairedCampaign],
            ));
        }
    }

    let mut decision_map: BTreeMap<(&str, &str), &SelectionDecision> = BTreeMap::new();
    for decision in decisions {
        if !policy.required_selector_ids.contains(&decision.selector_id)
            || !expected_units.contains(decision.independent_unit_id.as_str())
            || !forecast_map.contains_key(decision.candidate_id.as_str())
            || decision.resource_budget_sha256 != policy.resource_budget_sha256
            || decision.selector_id.trim().is_empty()
            || decision.candidate_id.trim().is_empty()
            || decision.independent_unit_id.trim().is_empty()
            || decision_map
                .insert(
                    (decision.selector_id.as_str(), decision.independent_unit_id.as_str()),
                    decision,
                )
                .is_some()
        {
            return Err(CalibrationError::InvalidSelectionDecision);
        }
    }

    let mut selector_regrets = Vec::new();
    for selector_id in &policy.required_selector_ids {
        let mut regrets = Vec::new();
        for unit in &expected_units {
            let Some(decision) = decision_map
                .get(&(selector_id.as_str(), *unit))
                .copied()
            else {
                return Ok(not_calibratable(
                    policy,
                    vec![NotCalibratableReason::MissingSelectorDecision],
                ));
            };
            let candidates = by_unit
                .get(*unit)
                .ok_or(CalibrationError::InvalidSelectionDecision)?;
            let oracle_best = candidates
                .values()
                .map(|outcome| outcome.realized.decision_relevance)
                .fold(0.0_f64, f64::max);
            let selected = candidates
                .get(decision.candidate_id.as_str())
                .ok_or(CalibrationError::InvalidSelectionDecision)?;
            regrets.push((oracle_best - selected.realized.decision_relevance).max(0.0));
        }
        let mean_regret = regrets.iter().sum::<f64>() / regrets.len() as f64;
        let maximum_regret = regrets.iter().copied().fold(0.0_f64, f64::max);
        selector_regrets.push(SelectorRegret {
            selector_id: selector_id.clone(),
            paired_unit_count: regrets.len(),
            mean_regret,
            maximum_regret,
        });
    }
    selector_regrets.sort_by(|left, right| left.selector_id.cmp(&right.selector_id));

    let primary = selector_regrets
        .iter()
        .find(|item| item.selector_id == policy.primary_selector_id)
        .ok_or(CalibrationError::InvalidSelectionDecision)?;
    let random = selector_regrets
        .iter()
        .find(|item| item.selector_id == policy.random_baseline_selector_id)
        .ok_or(CalibrationError::InvalidSelectionDecision)?;
    let improvement = random.mean_regret - primary.mean_regret;
    if primary.mean_regret > policy.maximum_primary_mean_regret {
        failures.insert(CalibrationFailure::PrimaryDecisionRegretExceeded);
    }
    if improvement < policy.minimum_regret_improvement_vs_random {
        failures.insert(CalibrationFailure::InsufficientImprovementOverRandom);
    }

    let failures: Vec<CalibrationFailure> = failures.into_iter().collect();
    let disposition = if failures.is_empty() {
        CalibrationDisposition::CalibratedForDeclaredCampaign
    } else {
        CalibrationDisposition::AcceptanceCriteriaFailed
    };
    Ok(CalibrationResult {
        disposition,
        candidate_strata,
        selector_regrets,
        primary_mean_regret: Some(primary.mean_regret),
        random_mean_regret: Some(random.mean_regret),
        improvement_over_random: Some(improvement),
        not_calibratable_reasons: Vec::new(),
        failures,
        policy_sha256: policy.policy_sha256.clone(),
        evaluation_manifest_sha256: policy.evaluation_manifest.manifest_sha256.clone(),
    })
}
