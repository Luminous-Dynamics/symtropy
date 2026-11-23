//! Constraint-first, nuisance-adjusted experiment portfolio selection.
//!
//! This is an abstract linear-Gaussian reference kernel with two physical
//! parameters, two nuisance parameters, and three competing model hypotheses.
//! It is not an EHD constitutive model or physical parameter estimator.
//!
//! The key invariants are:
//! - the complete candidate-level measurement covariance is represented;
//! - physical information is profiled against nuisance directions;
//! - model-pair separation is profiled against the same nuisance subspace;
//! - all declared parameter/noise scenarios are evaluated and reduced by
//!   worst-case (minimum) criteria;
//! - the output is a Pareto frontier, never a hidden scalarized winner;
//! - invalid/rank-deficient inputs fail closed.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const PHYSICAL_PARAMETER_COUNT: usize = 2;
pub const NUISANCE_PARAMETER_COUNT: usize = 2;
pub const MODEL_COUNT: usize = 3;
pub const MAX_CANDIDATES: usize = 16;
const COVARIANCE_SYMMETRY_REL_TOLERANCE: f64 = 1.0e-10;
const NUISANCE_RELATIVE_DETERMINANT_TOLERANCE: f64 = 1.0e-12;
const INFORMATION_RELATIVE_RANK_TOLERANCE: f64 = 1.0e-10;
const NUMERIC_NEGATIVE_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateExperiment {
    pub candidate_id: String,
    pub family_id: String,
    pub cost: u32,
    pub control_tags: BTreeSet<String>,
    /// One physical sensitivity row for every declared scenario.
    pub physical_sensitivities_by_scenario: Vec<[f64; PHYSICAL_PARAMETER_COUNT]>,
    /// Shared nuisance-mode sensitivities (e.g. force offset and drift).
    /// These are declared nuisance directions, not fitted physical effects.
    pub nuisance_sensitivity: [f64; NUISANCE_PARAMETER_COUNT],
    /// Three competing model predictions for every declared scenario.
    pub model_predictions_by_scenario: Vec<[f64; MODEL_COUNT]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioProblem {
    pub candidates: Vec<CandidateExperiment>,
    /// Full covariance across all candidate observations, scenario by scenario.
    /// Off-diagonal entries encode shared-reference, calibration, timing or
    /// other common-mode dependence between proposed experiments.
    pub observation_covariance_by_scenario: Vec<Vec<Vec<f64>>>,
    pub observation_budget: u32,
    pub minimum_distinct_families: usize,
    pub required_control_tags: BTreeSet<String>,
    /// Predeclared gate on the worst-case minimum eigenvalue of I_physical|nuisance.
    pub minimum_worst_case_information_eigenvalue: f64,
    /// Predeclared gate on the worst-case, nuisance-profiled pairwise separation.
    pub minimum_worst_case_model_separation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PortfolioError {
    EmptyCandidateSet,
    CandidateLimitExceeded { actual: usize, maximum: usize },
    EmptyScenarioSet,
    ScenarioCountMismatch { candidate_id: String },
    CovarianceScenarioCountMismatch,
    CovarianceShapeMismatch { scenario: usize, expected: usize, actual: usize },
    NonFiniteCovariance { scenario: usize, row: usize, column: usize },
    InvalidCovarianceVariance { scenario: usize, index: usize },
    AsymmetricCovariance { scenario: usize, row: usize, column: usize },
    NonPositiveDefiniteCovariance { scenario: usize },
    EmptyCandidateId { index: usize },
    DuplicateCandidateId(String),
    EmptyFamilyId { candidate_id: String },
    InvalidCandidateCost { candidate_id: String },
    EmptyControlTag { candidate_id: String },
    NonFinitePhysicalSensitivity { candidate_id: String, scenario: usize, parameter: usize },
    NonFiniteNuisanceSensitivity { candidate_id: String, parameter: usize },
    NonFiniteModelPrediction { candidate_id: String, scenario: usize, model: usize },
    InvalidObservationBudget,
    InvalidFamilyRequirement,
    InvalidInformationThreshold,
    InvalidModelSeparationThreshold,
    NonFiniteDerivedValue,
}

impl fmt::Display for PortfolioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCandidateSet => write!(f, "at least one candidate experiment is required"),
            Self::CandidateLimitExceeded { actual, maximum } => {
                write!(f, "candidate count {actual} exceeds exhaustive-search cap {maximum}")
            }
            Self::EmptyScenarioSet => write!(f, "at least one robustness scenario is required"),
            Self::ScenarioCountMismatch { candidate_id } => {
                write!(f, "scenario arrays have inconsistent lengths for {candidate_id}")
            }
            Self::CovarianceScenarioCountMismatch => {
                write!(f, "measurement covariance scenario count does not match candidate scenarios")
            }
            Self::CovarianceShapeMismatch { scenario, expected, actual } => write!(
                f,
                "scenario {scenario} covariance has {actual} rows; expected {expected}"
            ),
            Self::NonFiniteCovariance { scenario, row, column } => write!(
                f,
                "scenario {scenario} covariance entry ({row}, {column}) is non-finite"
            ),
            Self::InvalidCovarianceVariance { scenario, index } => {
                write!(f, "scenario {scenario} variance {index} must be finite and positive")
            }
            Self::AsymmetricCovariance { scenario, row, column } => write!(
                f,
                "scenario {scenario} covariance entries ({row}, {column}) are materially asymmetric"
            ),
            Self::NonPositiveDefiniteCovariance { scenario } => {
                write!(f, "scenario {scenario} covariance is not positive definite")
            }
            Self::EmptyCandidateId { index } => write!(f, "candidate {index} has an empty ID"),
            Self::DuplicateCandidateId(id) => write!(f, "duplicate candidate ID: {id}"),
            Self::EmptyFamilyId { candidate_id } => {
                write!(f, "candidate {candidate_id} has an empty family ID")
            }
            Self::InvalidCandidateCost { candidate_id } => {
                write!(f, "candidate {candidate_id} must have positive cost")
            }
            Self::EmptyControlTag { candidate_id } => {
                write!(f, "candidate {candidate_id} contains an empty control tag")
            }
            Self::NonFinitePhysicalSensitivity { candidate_id, scenario, parameter } => write!(
                f,
                "candidate {candidate_id}, scenario {scenario}, physical sensitivity {parameter} is non-finite"
            ),
            Self::NonFiniteNuisanceSensitivity { candidate_id, parameter } => write!(
                f,
                "candidate {candidate_id}, nuisance sensitivity {parameter} is non-finite"
            ),
            Self::NonFiniteModelPrediction { candidate_id, scenario, model } => write!(
                f,
                "candidate {candidate_id}, scenario {scenario}, model prediction {model} is non-finite"
            ),
            Self::InvalidObservationBudget => write!(f, "observation budget must be positive"),
            Self::InvalidFamilyRequirement => write!(f, "minimum distinct family count is invalid"),
            Self::InvalidInformationThreshold => {
                write!(f, "minimum information eigenvalue must be finite and non-negative")
            }
            Self::InvalidModelSeparationThreshold => {
                write!(f, "minimum model separation must be finite and non-negative")
            }
            Self::NonFiniteDerivedValue => write!(f, "portfolio evaluation produced a non-finite value"),
        }
    }
}

impl Error for PortfolioError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioScore {
    pub scenario_index: usize,
    pub minimum_information_eigenvalue: f64,
    pub log_determinant_information: f64,
    pub minimum_model_separation: f64,
    pub least_separable_model_pair: (usize, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioScore {
    /// Sorted IDs make output invariant to candidate input order.
    pub candidate_ids: Vec<String>,
    pub family_ids: Vec<String>,
    pub control_tags: BTreeSet<String>,
    pub total_cost: u32,
    pub worst_case_minimum_information_eigenvalue: f64,
    pub worst_case_log_determinant_information: f64,
    pub worst_case_minimum_model_separation: f64,
    pub least_separable_model_pair: (usize, usize),
    pub least_separable_scenario: usize,
    pub scenario_scores: Vec<ScenarioScore>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RejectionCounts {
    pub over_budget: usize,
    pub missing_control_coverage: usize,
    pub insufficient_families: usize,
    pub singular_nuisance_information: usize,
    pub non_identifiable_physical_information: usize,
    pub insufficient_model_separation: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortfolioReport {
    pub evaluated_subsets: usize,
    pub qualified_subsets: usize,
    pub rejection_counts: RejectionCounts,
    /// All non-dominated designs under E-like information, D-like information,
    /// model-pair discrimination and cost. This is not a single winner.
    pub pareto_frontier: Vec<PortfolioScore>,
    pub abstention_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScenarioRejection {
    SingularNuisanceInformation,
    NonIdentifiablePhysicalInformation,
    InsufficientModelSeparation,
}

#[derive(Clone, Debug)]
struct NormalizedCovariance {
    correlation: Vec<Vec<f64>>,
    standard_deviations: Vec<f64>,
}

#[derive(Clone, Debug)]
struct ScenarioEvaluation {
    score: ScenarioScore,
}

pub fn evaluate_portfolio_problem(
    problem: &PortfolioProblem,
) -> Result<PortfolioReport, PortfolioError> {
    validate_problem(problem)?;
    let candidate_count = problem.candidates.len();
    let subset_limit = 1_u32 << candidate_count;
    let normalized_covariances = problem
        .observation_covariance_by_scenario
        .iter()
        .enumerate()
        .map(|(scenario, covariance)| normalize_and_validate_covariance(covariance, scenario))
        .collect::<Result<Vec<_>, _>>()?;

    let mut report = PortfolioReport {
        evaluated_subsets: 0,
        qualified_subsets: 0,
        rejection_counts: RejectionCounts::default(),
        pareto_frontier: Vec::new(),
        abstention_reason: None,
    };

    let mut qualified = Vec::new();
    for mask in 1_u32..subset_limit {
        report.evaluated_subsets += 1;
        let selected_indices: Vec<usize> = (0..candidate_count)
            .filter(|index| (mask & (1_u32 << index)) != 0)
            .collect();

        let cost = selected_indices.iter().try_fold(0_u32, |sum, index| {
            sum.checked_add(problem.candidates[*index].cost)
        });
        let Some(cost) = cost else {
            return Err(PortfolioError::NonFiniteDerivedValue);
        };
        if cost > problem.observation_budget {
            report.rejection_counts.over_budget += 1;
            continue;
        }

        let mut tags = BTreeSet::new();
        let mut families = BTreeSet::new();
        for index in &selected_indices {
            let candidate = &problem.candidates[*index];
            tags.extend(candidate.control_tags.iter().cloned());
            families.insert(candidate.family_id.clone());
        }
        if !problem.required_control_tags.is_subset(&tags) {
            report.rejection_counts.missing_control_coverage += 1;
            continue;
        }
        if families.len() < problem.minimum_distinct_families {
            report.rejection_counts.insufficient_families += 1;
            continue;
        }

        match evaluate_selected_portfolio(
            problem,
            &normalized_covariances,
            &selected_indices,
            cost,
            families,
            tags,
        )? {
            Ok(score) => {
                report.qualified_subsets += 1;
                qualified.push(score);
            }
            Err(ScenarioRejection::SingularNuisanceInformation) => {
                report.rejection_counts.singular_nuisance_information += 1;
            }
            Err(ScenarioRejection::NonIdentifiablePhysicalInformation) => {
                report.rejection_counts.non_identifiable_physical_information += 1;
            }
            Err(ScenarioRejection::InsufficientModelSeparation) => {
                report.rejection_counts.insufficient_model_separation += 1;
            }
        }
    }

    for index in 0..qualified.len() {
        let dominated = qualified
            .iter()
            .enumerate()
            .any(|(other_index, other)| {
                other_index != index && dominates(other, &qualified[index])
            });
        if !dominated {
            report.pareto_frontier.push(qualified[index].clone());
        }
    }

    report.pareto_frontier.sort_by(score_order);
    if report.pareto_frontier.is_empty() {
        report.abstention_reason = Some(
            "no portfolio satisfies the frozen budget, control, family, nuisance-rank, physical-information and model-separation gates"
                .to_owned(),
        );
    }
    Ok(report)
}

fn validate_problem(problem: &PortfolioProblem) -> Result<(), PortfolioError> {
    let n = problem.candidates.len();
    if n == 0 {
        return Err(PortfolioError::EmptyCandidateSet);
    }
    if n > MAX_CANDIDATES {
        return Err(PortfolioError::CandidateLimitExceeded {
            actual: n,
            maximum: MAX_CANDIDATES,
        });
    }
    if problem.observation_budget == 0 {
        return Err(PortfolioError::InvalidObservationBudget);
    }
    if problem.minimum_distinct_families == 0 || problem.minimum_distinct_families > n {
        return Err(PortfolioError::InvalidFamilyRequirement);
    }
    if !problem.minimum_worst_case_information_eigenvalue.is_finite()
        || problem.minimum_worst_case_information_eigenvalue < 0.0
    {
        return Err(PortfolioError::InvalidInformationThreshold);
    }
    if !problem.minimum_worst_case_model_separation.is_finite()
        || problem.minimum_worst_case_model_separation < 0.0
    {
        return Err(PortfolioError::InvalidModelSeparationThreshold);
    }
    for tag in &problem.required_control_tags {
        if tag.trim().is_empty() || tag.chars().any(char::is_control) {
            return Err(PortfolioError::EmptyControlTag {
                candidate_id: "<problem>".to_owned(),
            });
        }
    }

    let scenario_count = problem
        .candidates
        .first()
        .map(|candidate| candidate.physical_sensitivities_by_scenario.len())
        .unwrap_or(0);
    if scenario_count == 0 {
        return Err(PortfolioError::EmptyScenarioSet);
    }
    if problem.observation_covariance_by_scenario.len() != scenario_count {
        return Err(PortfolioError::CovarianceScenarioCountMismatch);
    }

    let mut candidate_ids = BTreeSet::new();
    for (index, candidate) in problem.candidates.iter().enumerate() {
        if candidate.candidate_id.trim().is_empty()
            || candidate.candidate_id.chars().any(char::is_control)
        {
            return Err(PortfolioError::EmptyCandidateId { index });
        }
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(PortfolioError::DuplicateCandidateId(candidate.candidate_id.clone()));
        }
        if candidate.family_id.trim().is_empty()
            || candidate.family_id.chars().any(char::is_control)
        {
            return Err(PortfolioError::EmptyFamilyId {
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        if candidate.cost == 0 {
            return Err(PortfolioError::InvalidCandidateCost {
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        for tag in &candidate.control_tags {
            if tag.trim().is_empty() || tag.chars().any(char::is_control) {
                return Err(PortfolioError::EmptyControlTag {
                    candidate_id: candidate.candidate_id.clone(),
                });
            }
        }
        if !candidate
            .nuisance_sensitivity
            .iter()
            .all(|value| value.is_finite())
        {
            let parameter = candidate
                .nuisance_sensitivity
                .iter()
                .position(|value| !value.is_finite())
                .unwrap_or(0);
            return Err(PortfolioError::NonFiniteNuisanceSensitivity {
                candidate_id: candidate.candidate_id.clone(),
                parameter,
            });
        }
        if candidate.physical_sensitivities_by_scenario.len() != scenario_count
            || candidate.model_predictions_by_scenario.len() != scenario_count
        {
            return Err(PortfolioError::ScenarioCountMismatch {
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        for scenario in 0..scenario_count {
            for parameter in 0..PHYSICAL_PARAMETER_COUNT {
                if !candidate.physical_sensitivities_by_scenario[scenario][parameter].is_finite() {
                    return Err(PortfolioError::NonFinitePhysicalSensitivity {
                        candidate_id: candidate.candidate_id.clone(),
                        scenario,
                        parameter,
                    });
                }
            }
            for model in 0..MODEL_COUNT {
                if !candidate.model_predictions_by_scenario[scenario][model].is_finite() {
                    return Err(PortfolioError::NonFiniteModelPrediction {
                        candidate_id: candidate.candidate_id.clone(),
                        scenario,
                        model,
                    });
                }
            }
        }
    }
    Ok(())
}

fn normalize_and_validate_covariance(
    covariance: &[Vec<f64>],
    scenario: usize,
) -> Result<NormalizedCovariance, PortfolioError> {
    let n = covariance.len();
    if n == 0 {
        return Err(PortfolioError::CovarianceShapeMismatch {
            scenario,
            expected: 1,
            actual: 0,
        });
    }
    for (row, values) in covariance.iter().enumerate() {
        if values.len() != n {
            return Err(PortfolioError::CovarianceShapeMismatch {
                scenario,
                expected: n,
                actual: values.len(),
            });
        }
        for (column, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(PortfolioError::NonFiniteCovariance {
                    scenario,
                    row,
                    column,
                });
            }
        }
    }

    let mut standard_deviations = Vec::with_capacity(n);
    for index in 0..n {
        let variance = covariance[index][index];
        if !variance.is_finite() || variance <= 0.0 {
            return Err(PortfolioError::InvalidCovarianceVariance { scenario, index });
        }
        standard_deviations.push(variance.sqrt());
    }

    let mut correlation = vec![vec![0.0_f64; n]; n];
    for row in 0..n {
        correlation[row][row] = 1.0;
        for column in (row + 1)..n {
            let upper = covariance[row][column];
            let lower = covariance[column][row];
            let scale = (standard_deviations[row] * standard_deviations[column])
                .max(upper.abs())
                .max(lower.abs())
                .max(f64::MIN_POSITIVE);
            if (upper - lower).abs() > COVARIANCE_SYMMETRY_REL_TOLERANCE * scale {
                return Err(PortfolioError::AsymmetricCovariance {
                    scenario,
                    row,
                    column,
                });
            }
            let normalized = (upper * 0.5 + lower * 0.5)
                / (standard_deviations[row] * standard_deviations[column]);
            if !normalized.is_finite() {
                return Err(PortfolioError::NonFiniteDerivedValue);
            }
            correlation[row][column] = normalized;
            correlation[column][row] = normalized;
        }
    }

    if cholesky_decompose(&correlation).is_none() {
        return Err(PortfolioError::NonPositiveDefiniteCovariance { scenario });
    }
    Ok(NormalizedCovariance {
        correlation,
        standard_deviations,
    })
}

fn evaluate_selected_portfolio(
    problem: &PortfolioProblem,
    covariances: &[NormalizedCovariance],
    selected_indices: &[usize],
    total_cost: u32,
    families: BTreeSet<String>,
    tags: BTreeSet<String>,
) -> Result<Result<PortfolioScore, ScenarioRejection>, PortfolioError> {
    let mut scenario_scores = Vec::with_capacity(covariances.len());
    for scenario in 0..covariances.len() {
        let evaluation = match evaluate_scenario(
            problem,
            &covariances[scenario],
            selected_indices,
            scenario,
        )? {
            Ok(value) => value,
            Err(rejection) => return Ok(Err(rejection)),
        };
        scenario_scores.push(evaluation.score);
    }

    let mut worst_information = f64::INFINITY;
    let mut worst_logdet = f64::INFINITY;
    let mut worst_model_separation = f64::INFINITY;
    let mut least_separable_model_pair = (0, 1);
    let mut least_separable_scenario = 0;

    for score in &scenario_scores {
        worst_information = worst_information.min(score.minimum_information_eigenvalue);
        worst_logdet = worst_logdet.min(score.log_determinant_information);
        if score.minimum_model_separation < worst_model_separation {
            worst_model_separation = score.minimum_model_separation;
            least_separable_model_pair = score.least_separable_model_pair;
            least_separable_scenario = score.scenario_index;
        }
    }

    if ![
        worst_information,
        worst_logdet,
        worst_model_separation,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(PortfolioError::NonFiniteDerivedValue);
    }
    if worst_information < problem.minimum_worst_case_information_eigenvalue {
        return Ok(Err(ScenarioRejection::NonIdentifiablePhysicalInformation));
    }
    if worst_model_separation < problem.minimum_worst_case_model_separation {
        return Ok(Err(ScenarioRejection::InsufficientModelSeparation));
    }

    let mut candidate_ids: Vec<String> = selected_indices
        .iter()
        .map(|index| problem.candidates[*index].candidate_id.clone())
        .collect();
    candidate_ids.sort();

    Ok(Ok(PortfolioScore {
        candidate_ids,
        family_ids: families.into_iter().collect(),
        control_tags: tags,
        total_cost,
        worst_case_minimum_information_eigenvalue: worst_information,
        worst_case_log_determinant_information: worst_logdet,
        worst_case_minimum_model_separation: worst_model_separation,
        least_separable_model_pair,
        least_separable_scenario,
        scenario_scores,
    }))
}

fn evaluate_scenario(
    problem: &PortfolioProblem,
    covariance: &NormalizedCovariance,
    selected_indices: &[usize],
    scenario: usize,
) -> Result<Result<ScenarioEvaluation, ScenarioRejection>, PortfolioError> {
    let n = selected_indices.len();
    let mut correlation = vec![vec![0.0_f64; n]; n];
    let mut standard_deviations = Vec::with_capacity(n);
    for (row, source_row) in selected_indices.iter().enumerate() {
        standard_deviations.push(covariance.standard_deviations[*source_row]);
        for (column, source_column) in selected_indices.iter().enumerate() {
            correlation[row][column] =
                covariance.correlation[*source_row][*source_column];
        }
    }
    let Some(cholesky) = cholesky_decompose(&correlation) else {
        return Err(PortfolioError::NonFiniteDerivedValue);
    };

    let mut p0 = Vec::with_capacity(n);
    let mut p1 = Vec::with_capacity(n);
    let mut n0 = Vec::with_capacity(n);
    let mut n1 = Vec::with_capacity(n);
    for index in selected_indices {
        let candidate = &problem.candidates[*index];
        let scale = standard_deviations[p0.len()];
        let physical = candidate.physical_sensitivities_by_scenario[scenario];
        p0.push(physical[0] / scale);
        p1.push(physical[1] / scale);
        n0.push(candidate.nuisance_sensitivity[0] / scale);
        n1.push(candidate.nuisance_sensitivity[1] / scale);
    }

    let ipp = [
        [weighted_dot(&p0, &p0, &cholesky)?, weighted_dot(&p0, &p1, &cholesky)?],
        [weighted_dot(&p1, &p0, &cholesky)?, weighted_dot(&p1, &p1, &cholesky)?],
    ];
    let ipn = [
        [weighted_dot(&p0, &n0, &cholesky)?, weighted_dot(&p0, &n1, &cholesky)?],
        [weighted_dot(&p1, &n0, &cholesky)?, weighted_dot(&p1, &n1, &cholesky)?],
    ];
    let inn = [
        [weighted_dot(&n0, &n0, &cholesky)?, weighted_dot(&n0, &n1, &cholesky)?],
        [weighted_dot(&n1, &n0, &cholesky)?, weighted_dot(&n1, &n1, &cholesky)?],
    ];

    let Some(inverse_inn) = inverse_positive_definite_2x2(inn) else {
        return Ok(Err(ScenarioRejection::SingularNuisanceInformation));
    };
    let correction = multiply_2x2(multiply_2x2(ipn, inverse_inn), transpose_2x2(ipn));
    let effective = [
        [
            ipp[0][0] - correction[0][0],
            0.5 * (ipp[0][1] + ipp[1][0] - correction[0][1] - correction[1][0]),
        ],
        [
            0.5 * (ipp[1][0] + ipp[0][1] - correction[1][0] - correction[0][1]),
            ipp[1][1] - correction[1][1],
        ],
    ];
    if !effective.iter().flatten().all(|value| value.is_finite()) {
        return Err(PortfolioError::NonFiniteDerivedValue);
    }

    let determinant = effective[0][0] * effective[1][1]
        - effective[0][1] * effective[1][0];
    let trace_half = 0.5 * (effective[0][0] + effective[1][1]);
    let radius = (0.5 * (effective[0][0] - effective[1][1])).hypot(effective[0][1]);
    let maximum_eigenvalue = trace_half + radius;
    let minimum_eigenvalue = if maximum_eigenvalue > 0.0 {
        determinant / maximum_eigenvalue
    } else {
        trace_half - radius
    };

    if !determinant.is_finite()
        || !maximum_eigenvalue.is_finite()
        || !minimum_eigenvalue.is_finite()
    {
        return Err(PortfolioError::NonFiniteDerivedValue);
    }
    if maximum_eigenvalue <= 0.0
        || determinant <= 0.0
        || minimum_eigenvalue
            <= INFORMATION_RELATIVE_RANK_TOLERANCE * maximum_eigenvalue
    {
        return Ok(Err(ScenarioRejection::NonIdentifiablePhysicalInformation));
    }
    let log_determinant = determinant.ln();

    let model_pairs = [(0, 1), (0, 2), (1, 2)];
    let mut minimum_model_separation = f64::INFINITY;
    let mut least_separable_model_pair = (0, 1);
    for pair in model_pairs {
        let mut delta = Vec::with_capacity(n);
        for (position, index) in selected_indices.iter().enumerate() {
            let candidate = &problem.candidates[*index];
            let predictions = candidate.model_predictions_by_scenario[scenario];
            delta.push((predictions[pair.0] - predictions[pair.1]) / standard_deviations[position]);
        }
        let total_distance = weighted_dot(&delta, &delta, &cholesky)?;
        let nuisance_projection = [
            weighted_dot(&n0, &delta, &cholesky)?,
            weighted_dot(&n1, &delta, &cholesky)?,
        ];
        let explained = quadratic_form_2x2(inverse_inn, nuisance_projection);
        let raw_separation = total_distance - explained;
        let tolerance = NUMERIC_NEGATIVE_TOLERANCE * total_distance.abs().max(1.0);
        if raw_separation < -tolerance {
            return Err(PortfolioError::NonFiniteDerivedValue);
        }
        let separation = raw_separation.max(0.0);
        if separation < minimum_model_separation {
            minimum_model_separation = separation;
            least_separable_model_pair = pair;
        }
    }

    if !minimum_model_separation.is_finite() {
        return Err(PortfolioError::NonFiniteDerivedValue);
    }

    Ok(Ok(ScenarioEvaluation {
        score: ScenarioScore {
            scenario_index: scenario,
            minimum_information_eigenvalue: minimum_eigenvalue,
            log_determinant_information: log_determinant,
            minimum_model_separation,
            least_separable_model_pair,
        },
    }))
}

fn cholesky_decompose(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    let mut lower = vec![vec![0.0_f64; n]; n];
    for row in 0..n {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for index in 0..column {
                value -= lower[row][index] * lower[column][index];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return None;
                }
                lower[row][column] = value.sqrt();
            } else {
                let divisor = lower[column][column];
                if !divisor.is_finite() || divisor <= 0.0 {
                    return None;
                }
                lower[row][column] = value / divisor;
            }
        }
    }
    Some(lower)
}

fn solve_cholesky(lower: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, PortfolioError> {
    let n = lower.len();
    if rhs.len() != n {
        return Err(PortfolioError::NonFiniteDerivedValue);
    }
    let mut intermediate = vec![0.0_f64; n];
    for row in 0..n {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * intermediate[column];
        }
        intermediate[row] = value / lower[row][row];
    }

    let mut solution = vec![0.0_f64; n];
    for row in (0..n).rev() {
        let mut value = intermediate[row];
        for column in (row + 1)..n {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
    }
    if !solution.iter().all(|value| value.is_finite()) {
        return Err(PortfolioError::NonFiniteDerivedValue);
    }
    Ok(solution)
}

fn weighted_dot(
    left: &[f64],
    right: &[f64],
    covariance_cholesky: &[Vec<f64>],
) -> Result<f64, PortfolioError> {
    let solved = solve_cholesky(covariance_cholesky, right)?;
    let value = left.iter().zip(solved.iter()).map(|(a, b)| a * b).sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(PortfolioError::NonFiniteDerivedValue)
    }
}

fn inverse_positive_definite_2x2(matrix: [[f64; 2]; 2]) -> Option<[[f64; 2]; 2]> {
    let a = matrix[0][0];
    let b = 0.5 * (matrix[0][1] + matrix[1][0]);
    let c = matrix[1][1];
    let determinant = a * c - b * b;
    let scale = a.abs().max(b.abs()).max(c.abs());
    if ![a, b, c, determinant, scale]
        .iter()
        .all(|value| value.is_finite())
        || a <= 0.0
        || c <= 0.0
        || determinant <= NUISANCE_RELATIVE_DETERMINANT_TOLERANCE * scale * scale
    {
        return None;
    }
    Some([
        [c / determinant, -b / determinant],
        [-b / determinant, a / determinant],
    ])
}

fn multiply_2x2(left: [[f64; 2]; 2], right: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [
        [
            left[0][0] * right[0][0] + left[0][1] * right[1][0],
            left[0][0] * right[0][1] + left[0][1] * right[1][1],
        ],
        [
            left[1][0] * right[0][0] + left[1][1] * right[1][0],
            left[1][0] * right[0][1] + left[1][1] * right[1][1],
        ],
    ]
}

fn transpose_2x2(matrix: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
}

fn quadratic_form_2x2(matrix: [[f64; 2]; 2], vector: [f64; 2]) -> f64 {
    vector[0] * (matrix[0][0] * vector[0] + matrix[0][1] * vector[1])
        + vector[1] * (matrix[1][0] * vector[0] + matrix[1][1] * vector[1])
}

fn dominates(left: &PortfolioScore, right: &PortfolioScore) -> bool {
    let tolerance = |a: f64, b: f64| 1.0e-12 * a.abs().max(b.abs()).max(1.0);
    let e_better_or_equal = left.worst_case_minimum_information_eigenvalue
        + tolerance(
            left.worst_case_minimum_information_eigenvalue,
            right.worst_case_minimum_information_eigenvalue,
        )
        >= right.worst_case_minimum_information_eigenvalue;
    let d_better_or_equal = left.worst_case_log_determinant_information
        + tolerance(
            left.worst_case_log_determinant_information,
            right.worst_case_log_determinant_information,
        )
        >= right.worst_case_log_determinant_information;
    let model_better_or_equal = left.worst_case_minimum_model_separation
        + tolerance(
            left.worst_case_minimum_model_separation,
            right.worst_case_minimum_model_separation,
        )
        >= right.worst_case_minimum_model_separation;
    let cost_better_or_equal = left.total_cost <= right.total_cost;

    let strictly_better = left.worst_case_minimum_information_eigenvalue
        > right.worst_case_minimum_information_eigenvalue
            + tolerance(
                left.worst_case_minimum_information_eigenvalue,
                right.worst_case_minimum_information_eigenvalue,
            )
        || left.worst_case_log_determinant_information
            > right.worst_case_log_determinant_information
                + tolerance(
                    left.worst_case_log_determinant_information,
                    right.worst_case_log_determinant_information,
                )
        || left.worst_case_minimum_model_separation
            > right.worst_case_minimum_model_separation
                + tolerance(
                    left.worst_case_minimum_model_separation,
                    right.worst_case_minimum_model_separation,
                )
        || left.total_cost < right.total_cost;

    e_better_or_equal && d_better_or_equal && model_better_or_equal && cost_better_or_equal
        && strictly_better
}

fn score_order(left: &PortfolioScore, right: &PortfolioScore) -> std::cmp::Ordering {
    left.total_cost
        .cmp(&right.total_cost)
        .then_with(|| {
            right
                .worst_case_minimum_information_eigenvalue
                .total_cmp(&left.worst_case_minimum_information_eigenvalue)
        })
        .then_with(|| {
            right
                .worst_case_minimum_model_separation
                .total_cmp(&left.worst_case_minimum_model_separation)
        })
        .then_with(|| left.candidate_ids.cmp(&right.candidate_ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn candidate(
        id: &str,
        family: &str,
        cost: u32,
        control_tags: &[&str],
        physical: [[f64; 2]; 2],
        nuisance: [f64; 2],
        predictions: [[f64; 3]; 2],
    ) -> CandidateExperiment {
        CandidateExperiment {
            candidate_id: id.to_owned(),
            family_id: family.to_owned(),
            cost,
            control_tags: tags(control_tags),
            physical_sensitivities_by_scenario: physical.to_vec(),
            nuisance_sensitivity: nuisance,
            model_predictions_by_scenario: predictions.to_vec(),
        }
    }

    fn fixture_problem() -> PortfolioProblem {
        let candidates = vec![
            candidate(
                "baseline",
                "waveform-a",
                1,
                &["baseline"],
                [[1.0, 0.0], [0.8, 0.0]],
                [1.0, 0.0],
                [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            ),
            candidate(
                "sham",
                "waveform-b",
                1,
                &["sham"],
                [[0.0, 1.0], [0.0, 0.8]],
                [0.0, 1.0],
                [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
            ),
            candidate(
                "reversal",
                "orientation-a",
                1,
                &["reversal"],
                [[1.0, 0.0], [1.2, 0.0]],
                [0.0, 0.0],
                [[0.0, 1.0, 0.0], [0.0, 1.1, 0.0]],
            ),
            candidate(
                "independent-diagnostic",
                "diagnostic-a",
                1,
                &["independent_diagnostic"],
                [[0.0, 1.0], [0.0, 1.2]],
                [0.0, 0.0],
                [[0.0, 0.0, 1.0], [0.0, 0.0, 1.1]],
            ),
            candidate(
                "geometry-extension",
                "geometry-b",
                2,
                &["geometry_diversity"],
                [[0.5, 0.5], [0.6, 0.4]],
                [1.0, 1.0],
                [[0.0, 0.5, 1.0], [0.0, 0.6, 1.1]],
            ),
        ];

        let covariance = vec![
            vec![1.0, 0.20, 0.10, 0.15, 0.10],
            vec![0.20, 1.0, 0.12, 0.10, 0.08],
            vec![0.10, 0.12, 1.0, 0.18, 0.16],
            vec![0.15, 0.10, 0.18, 1.0, 0.14],
            vec![0.10, 0.08, 0.16, 0.14, 1.0],
        ];
        let covariance_scenario_2 = vec![
            vec![1.2, 0.28, 0.18, 0.20, 0.15],
            vec![0.28, 1.1, 0.20, 0.16, 0.12],
            vec![0.18, 0.20, 1.3, 0.24, 0.22],
            vec![0.20, 0.16, 0.24, 1.2, 0.18],
            vec![0.15, 0.12, 0.22, 0.18, 1.1],
        ];
        PortfolioProblem {
            candidates,
            observation_covariance_by_scenario: vec![covariance, covariance_scenario_2],
            observation_budget: 6,
            minimum_distinct_families: 4,
            required_control_tags: tags(&[
                "baseline",
                "sham",
                "reversal",
                "independent_diagnostic",
            ]),
            minimum_worst_case_information_eigenvalue: 0.01,
            minimum_worst_case_model_separation: 0.05,
        }
    }

    #[test]
    fn control_rich_portfolio_is_accepted_and_frontier_is_not_scalarized() {
        let report = evaluate_portfolio_problem(&fixture_problem()).unwrap();
        assert!(!report.pareto_frontier.is_empty());
        assert!(report.qualified_subsets > 0);
        assert!(report
            .pareto_frontier
            .iter()
            .any(|score| score.candidate_ids.len() == 4));
        assert!(report
            .pareto_frontier
            .iter()
            .any(|score| score.candidate_ids.len() == 5));
    }

    #[test]
    fn physical_only_identifiability_is_removed_by_nuisance_profiling() {
        let mut problem = fixture_problem();
        for candidate in &mut problem.candidates {
            candidate.physical_sensitivities_by_scenario = vec![
                candidate.nuisance_sensitivity,
                candidate.nuisance_sensitivity,
            ];
        }
        let report = evaluate_portfolio_problem(&problem).unwrap();
        assert!(report.pareto_frontier.is_empty());
        assert!(report.rejection_counts.non_identifiable_physical_information > 0);
        assert!(report.abstention_reason.is_some());
    }

    #[test]
    fn model_differences_inside_nuisance_subspace_are_not_discriminated() {
        let mut problem = fixture_problem();
        let nuisance_rows = problem
            .candidates
            .iter()
            .map(|candidate| candidate.nuisance_sensitivity)
            .collect::<Vec<_>>();
        for candidate in &mut problem.candidates {
            let index = nuisance_rows
                .iter()
                .position(|row| *row == candidate.nuisance_sensitivity)
                .unwrap();
            let row = nuisance_rows[index];
            candidate.model_predictions_by_scenario = vec![
                [0.0, row[0], row[1]],
                [0.0, row[0], row[1]],
            ];
        }
        let report = evaluate_portfolio_problem(&problem).unwrap();
        assert!(report.pareto_frontier.is_empty());
        assert!(report.rejection_counts.insufficient_model_separation > 0);
    }

    #[test]
    fn shared_measurement_covariance_changes_profiled_model_separation() {
        let mut independent = fixture_problem();
        for covariance in &mut independent.observation_covariance_by_scenario {
            for row in 0..covariance.len() {
                for column in 0..covariance.len() {
                    covariance[row][column] = if row == column { covariance[row][column] } else { 0.0 };
                }
            }
        }
        let correlated_report = evaluate_portfolio_problem(&fixture_problem()).unwrap();
        let independent_report = evaluate_portfolio_problem(&independent).unwrap();
        let correlated = correlated_report
            .pareto_frontier
            .iter()
            .find(|score| score.candidate_ids.len() == 5)
            .unwrap();
        let diagonal = independent_report
            .pareto_frontier
            .iter()
            .find(|score| score.candidate_ids.len() == 5)
            .unwrap();
        assert!(
            (correlated.worst_case_minimum_model_separation
                - diagonal.worst_case_minimum_model_separation)
                .abs()
                > 1.0e-6
        );
    }

    #[test]
    fn singular_nuisance_block_fails_closed() {
        let mut problem = fixture_problem();
        for candidate in &mut problem.candidates {
            candidate.nuisance_sensitivity = [1.0, 0.0];
        }
        let report = evaluate_portfolio_problem(&problem).unwrap();
        assert!(report.pareto_frontier.is_empty());
        assert!(report.rejection_counts.singular_nuisance_information > 0);
    }

    #[test]
    fn missing_required_control_is_an_explicit_abstention() {
        let mut problem = fixture_problem();
        problem.required_control_tags.insert("thermal_sham".to_owned());
        let report = evaluate_portfolio_problem(&problem).unwrap();
        assert!(report.pareto_frontier.is_empty());
        assert!(report.rejection_counts.missing_control_coverage > 0);
    }

    #[test]
    fn scenario_worst_case_is_not_replaced_by_nominal_average() {
        let report = evaluate_portfolio_problem(&fixture_problem()).unwrap();
        let score = report
            .pareto_frontier
            .iter()
            .find(|score| score.candidate_ids.len() == 5)
            .unwrap();
        let expected = score
            .scenario_scores
            .iter()
            .map(|scenario| scenario.minimum_information_eigenvalue)
            .fold(f64::INFINITY, f64::min);
        assert_eq!(score.worst_case_minimum_information_eigenvalue, expected);
        assert_eq!(score.scenario_scores.len(), 2);
    }

    #[test]
    fn candidate_permutation_preserves_sorted_frontier_and_scores() {
        let original = fixture_problem();
        let original_report = evaluate_portfolio_problem(&original).unwrap();
        let mut permuted = original.clone();
        let count = permuted.candidates.len();
        permuted.candidates.reverse();
        for covariance in &mut permuted.observation_covariance_by_scenario {
            covariance.reverse();
            for row in covariance {
                row.reverse();
            }
        }
        let permuted_report = evaluate_portfolio_problem(&permuted).unwrap();

        assert_eq!(
            original_report.pareto_frontier.len(),
            permuted_report.pareto_frontier.len()
        );
        for (left, right) in original_report
            .pareto_frontier
            .iter()
            .zip(permuted_report.pareto_frontier.iter())
        {
            assert_eq!(left.candidate_ids, right.candidate_ids);
            assert_eq!(left.total_cost, right.total_cost);
            assert!((left.worst_case_minimum_information_eigenvalue
                - right.worst_case_minimum_information_eigenvalue)
                .abs()
                < 1.0e-10);
            assert!((left.worst_case_minimum_model_separation
                - right.worst_case_minimum_model_separation)
                .abs()
                < 1.0e-10);
        }
        assert_eq!(count, permuted.candidates.len());
    }

    #[test]
    fn unit_rescaling_does_not_change_covariance_qualification() {
        let original = fixture_problem();
        let original_report = evaluate_portfolio_problem(&original).unwrap();

        let scales = [1.0e6, 1.0e-4, 1.0e3, 1.0e-2, 1.0e5];
        let mut rescaled = original.clone();
        for scenario in 0..rescaled.observation_covariance_by_scenario.len() {
            let covariance = &mut rescaled.observation_covariance_by_scenario[scenario];
            for row in 0..covariance.len() {
                for column in 0..covariance.len() {
                    covariance[row][column] *= scales[row] * scales[column];
                }
            }
        }
        for (index, candidate) in rescaled.candidates.iter_mut().enumerate() {
            for sensitivity in &mut candidate.physical_sensitivities_by_scenario {
                sensitivity[0] *= scales[index];
                sensitivity[1] *= scales[index];
            }
            candidate.nuisance_sensitivity[0] *= scales[index];
            candidate.nuisance_sensitivity[1] *= scales[index];
            for prediction in &mut candidate.model_predictions_by_scenario {
                for value in prediction {
                    *value *= scales[index];
                }
            }
        }
        let rescaled_report = evaluate_portfolio_problem(&rescaled).unwrap();
        assert_eq!(
            original_report.pareto_frontier.len(),
            rescaled_report.pareto_frontier.len()
        );
        for (left, right) in original_report
            .pareto_frontier
            .iter()
            .zip(rescaled_report.pareto_frontier.iter())
        {
            assert_eq!(left.candidate_ids, right.candidate_ids);
            assert!((left.worst_case_minimum_information_eigenvalue
                - right.worst_case_minimum_information_eigenvalue)
                .abs()
                < 1.0e-8);
            assert!((left.worst_case_minimum_model_separation
                - right.worst_case_minimum_model_separation)
                .abs()
                < 1.0e-8);
        }
    }

    #[test]
    fn invalid_singular_covariance_is_rejected() {
        let mut problem = fixture_problem();
        for covariance in &mut problem.observation_covariance_by_scenario {
            for row in 0..covariance.len() {
                for column in 0..covariance.len() {
                    covariance[row][column] = 1.0;
                }
            }
        }
        assert!(matches!(
            evaluate_portfolio_problem(&problem),
            Err(PortfolioError::NonPositiveDefiniteCovariance { .. })
        ));
    }

    #[test]
    fn covariance_asymmetry_is_rejected() {
        let mut problem = fixture_problem();
        problem.observation_covariance_by_scenario[0][0][1] += 0.1;
        assert!(matches!(
            evaluate_portfolio_problem(&problem),
            Err(PortfolioError::AsymmetricCovariance { .. })
        ));
    }

    #[test]
    fn no_pareto_pair_is_dominated() {
        let report = evaluate_portfolio_problem(&fixture_problem()).unwrap();
        for left in 0..report.pareto_frontier.len() {
            for right in 0..report.pareto_frontier.len() {
                if left != right {
                    assert!(!dominates(
                        &report.pareto_frontier[left],
                        &report.pareto_frontier[right]
                    ));
                }
            }
        }
    }

    #[test]
    fn too_many_candidates_are_rejected_before_subset_enumeration() {
        let mut problem = fixture_problem();
        let template = problem.candidates[0].clone();
        while problem.candidates.len() <= MAX_CANDIDATES {
            let mut candidate = template.clone();
            candidate.candidate_id = format!("extra-{}", problem.candidates.len());
            problem.candidates.push(candidate);
        }
        assert!(matches!(
            evaluate_portfolio_problem(&problem),
            Err(PortfolioError::CandidateLimitExceeded { .. })
        ));
    }
}
