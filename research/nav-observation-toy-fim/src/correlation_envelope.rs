//! Finite-envelope evaluation for uncertain cross-correlation between sensors.
//!
//! Unlike correlated.rs, which consumes one declared common-mode model, this
//! module treats cross-correlation as an uncertain input. Each scenario must
//! provide every pairwise correlation explicitly. We reject incomplete or
//! non-positive-semidefinite covariance matrices; a missing pair is never
//! interpreted as zero correlation.
//!
//! This is a finite adversarial scenario envelope, not a continuous interval
//! certificate. The scenario set must be frozen independently of test results.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::correlated::CorrelatedRangeAction;
use crate::Fisher2;

#[derive(Clone, Debug, PartialEq)]
pub struct PairwiseCorrelation {
    pub action_a: String,
    pub action_b: String,
    /// Pearson correlation for the two scalar range errors, in [-1, 1].
    pub rho: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelationScenario {
    pub scenario_id: String,
    /// Exactly one entry for each unordered candidate-action pair.
    pub pairs: Vec<PairwiseCorrelation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelationScenarioMetric {
    pub scenario_id: String,
    pub fisher: Fisher2,
    pub rank: u8,
    pub determinant: f64,
    pub log_determinant: Option<f64>,
    pub condition_number: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelationEnvelopeEvaluation {
    pub scenario_metrics: Vec<CorrelationScenarioMetric>,
    pub worst_case_scenario_id: String,
    pub worst_case_rank: u8,
    /// Minimum eigenvalue across the finite scenarios: weakest-direction information.
    pub worst_case_minimum_eigenvalue: f64,
    /// Determinant for the weakest-direction scenario; zero if rank < 2.
    pub worst_case_determinant: f64,
    pub worst_case_log_determinant: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CorrelationEnvelopeError {
    EmptyActions,
    TooManyActions(usize),
    EmptyActionId,
    InvalidBearing(String),
    DuplicateActionId(String),
    InvalidSigma(String),
    ZeroCost(String),
    MixedCorrelationModels,
    EmptyScenarioSet,
    EmptyScenarioId,
    DuplicateScenarioId(String),
    UnknownAction(String),
    SelfPair(String),
    DuplicatePair(String, String),
    MissingPair(String, String),
    InvalidCorrelation(String, String),
    CovarianceNotPositiveDefinite(String),
    NonFiniteInformation(String),
    NoFeasibleSelection,
}

impl fmt::Display for CorrelationEnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActions => write!(f, "at least one action is required"),
            Self::TooManyActions(n) => write!(f, "{n} actions exceed the correlation-envelope limit of 16"),
            Self::EmptyActionId => write!(f, "action ID must not be empty"),
            Self::InvalidBearing(id) => write!(f, "non-finite line-of-sight bearing for action: {id}"),
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::InvalidSigma(id) => write!(f, "invalid private sigma for action: {id}"),
            Self::ZeroCost(id) => write!(f, "action cost must be positive: {id}"),
            Self::MixedCorrelationModels => write!(f, "pairwise scenarios cannot be mixed with common-mode-group inputs"),
            Self::EmptyScenarioSet => write!(f, "at least one correlation scenario is required"),
            Self::EmptyScenarioId => write!(f, "correlation scenario ID must not be empty"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate correlation scenario ID: {id}"),
            Self::UnknownAction(id) => write!(f, "correlation scenario references unknown action: {id}"),
            Self::SelfPair(id) => write!(f, "correlation pair references the same action twice: {id}"),
            Self::DuplicatePair(a, b) => write!(f, "duplicate correlation pair: {a} / {b}"),
            Self::MissingPair(a, b) => write!(f, "missing correlation pair: {a} / {b}"),
            Self::InvalidCorrelation(a, b) => write!(f, "invalid correlation coefficient for pair: {a} / {b}"),
            Self::CovarianceNotPositiveDefinite(id) => write!(f, "covariance matrix is not positive definite in scenario: {id}"),
            Self::NonFiniteInformation(id) => write!(f, "non-finite Fisher information in scenario: {id}"),
            Self::NoFeasibleSelection => write!(f, "no nonempty action subset fits the observation budget"),
        }
    }
}
impl std::error::Error for CorrelationEnvelopeError {}

fn validate_actions(
    actions: &[CorrelatedRangeAction],
) -> Result<Vec<CorrelatedRangeAction>, CorrelationEnvelopeError> {
    if actions.is_empty() {
        return Err(CorrelationEnvelopeError::EmptyActions);
    }
    if actions.len() > 16 {
        return Err(CorrelationEnvelopeError::TooManyActions(actions.len()));
    }

    let mut ids = BTreeSet::new();
    let mut ordered = actions.to_vec();
    for action in &ordered {
        if action.id.trim().is_empty() {
            return Err(CorrelationEnvelopeError::EmptyActionId);
        }
        if !ids.insert(action.id.clone()) {
            return Err(CorrelationEnvelopeError::DuplicateActionId(action.id.clone()));
        }
        if !action.line_of_sight_rad.is_finite() {
            return Err(CorrelationEnvelopeError::InvalidBearing(action.id.clone()));
        }
        let variance = action.independent_sigma_m * action.independent_sigma_m;
        if !action.independent_sigma_m.is_finite()
            || action.independent_sigma_m <= 0.0
            || !variance.is_finite()
            || variance <= 0.0
        {
            return Err(CorrelationEnvelopeError::InvalidSigma(action.id.clone()));
        }
        if action.cost_units == 0 {
            return Err(CorrelationEnvelopeError::ZeroCost(action.id.clone()));
        }
        if action.common_mode_group.is_some() || action.common_mode_sigma_m.is_some() {
            return Err(CorrelationEnvelopeError::MixedCorrelationModels);
        }
    }
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ordered)
}

fn canonical_pair(a: &str, b: &str) -> Result<(String, String), CorrelationEnvelopeError> {
    if a == b {
        return Err(CorrelationEnvelopeError::SelfPair(a.to_owned()));
    }
    if a < b {
        Ok((a.to_owned(), b.to_owned()))
    } else {
        Ok((b.to_owned(), a.to_owned()))
    }
}

fn cholesky(
    matrix: &[Vec<f64>],
    scenario_id: &str,
) -> Result<Vec<Vec<f64>>, CorrelationEnvelopeError> {
    let n = matrix.len();
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut value = matrix[i][j];
            let correction = lower[i][..j]
                .iter()
                .zip(lower[j][..j].iter())
                .map(|(left, right)| left * right)
                .sum::<f64>();
            value -= correction;
            if i == j {
                if !value.is_finite() || value <= 1.0e-12 {
                    return Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(
                        scenario_id.to_owned(),
                    ));
                }
                lower[i][j] = value.sqrt();
            } else {
                let divisor = lower[j][j];
                if !divisor.is_finite() || divisor <= 0.0 {
                    return Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(
                        scenario_id.to_owned(),
                    ));
                }
                lower[i][j] = value / divisor;
                if !lower[i][j].is_finite() {
                    return Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(
                        scenario_id.to_owned(),
                    ));
                }
            }
        }
    }
    Ok(lower)
}

fn solve(
    lower: &[Vec<f64>],
    rhs: &[f64],
    scenario_id: &str,
) -> Result<Vec<f64>, CorrelationEnvelopeError> {
    let n = rhs.len();
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * y[j];
        }
        y[i] = value / lower[i][i];
        if !y[i].is_finite() {
            return Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(
                scenario_id.to_owned(),
            ));
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = y[i];
        for j in (i + 1)..n {
            value -= lower[j][i] * x[j];
        }
        x[i] = value / lower[i][i];
        if !x[i].is_finite() {
            return Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(
                scenario_id.to_owned(),
            ));
        }
    }
    Ok(x)
}

pub fn correlation_scenario_fisher(
    actions: &[CorrelatedRangeAction],
    scenario: &CorrelationScenario,
) -> Result<Fisher2, CorrelationEnvelopeError> {
    let ordered = validate_actions(actions)?;
    let ids: BTreeSet<String> = ordered.iter().map(|a| a.id.clone()).collect();
    let n = ordered.len();
    let mut pair_map = BTreeMap::new();

    for pair in &scenario.pairs {
        if !ids.contains(&pair.action_a) {
            return Err(CorrelationEnvelopeError::UnknownAction(pair.action_a.clone()));
        }
        if !ids.contains(&pair.action_b) {
            return Err(CorrelationEnvelopeError::UnknownAction(pair.action_b.clone()));
        }
        let key = canonical_pair(&pair.action_a, &pair.action_b)?;
        if !pair.rho.is_finite() || !(-1.0..=1.0).contains(&pair.rho) {
            return Err(CorrelationEnvelopeError::InvalidCorrelation(key.0, key.1));
        }
        if pair_map.insert(key.clone(), pair.rho).is_some() {
            return Err(CorrelationEnvelopeError::DuplicatePair(key.0, key.1));
        }
    }

    let mut covariance = vec![vec![0.0; n]; n];
    let mut h = Vec::with_capacity(n);
    for (i, action) in ordered.iter().enumerate() {
        covariance[i][i] = action.independent_sigma_m * action.independent_sigma_m;
        h.push([
            action.line_of_sight_rad.cos(),
            action.line_of_sight_rad.sin(),
        ]);
        for j in (i + 1)..n {
            let a = &ordered[i].id;
            let b = &ordered[j].id;
            let key = canonical_pair(a, b)?;
            let rho = *pair_map
                .get(&key)
                .ok_or_else(|| CorrelationEnvelopeError::MissingPair(key.0.clone(), key.1.clone()))?;
            let cross = rho * action.independent_sigma_m * ordered[j].independent_sigma_m;
            covariance[i][j] = cross;
            covariance[j][i] = cross;
        }
    }

    let lower = cholesky(&covariance, &scenario.scenario_id)?;
    let hx: Vec<f64> = h.iter().map(|v| v[0]).collect();
    let hy: Vec<f64> = h.iter().map(|v| v[1]).collect();
    let rx = solve(&lower, &hx, &scenario.scenario_id)?;
    let ry = solve(&lower, &hy, &scenario.scenario_id)?;

    let xx = hx.iter().zip(&rx).map(|(a, b)| a * b).sum::<f64>();
    let xy = hx.iter().zip(&ry).map(|(a, b)| a * b).sum::<f64>();
    let yy = hy.iter().zip(&ry).map(|(a, b)| a * b).sum::<f64>();
    let fisher = Fisher2 { xx, xy, yy };
    if !xx.is_finite() || !xy.is_finite() || !yy.is_finite() || !fisher.determinant().is_finite() {
        return Err(CorrelationEnvelopeError::NonFiniteInformation(
            scenario.scenario_id.clone(),
        ));
    }
    Ok(fisher)
}

/// Evaluate one candidate action set against a frozen finite set of admissible
/// pairwise-correlation matrices. Worst-case rank loss maps the determinant to 0.
pub fn evaluate_correlation_envelope(
    actions: &[CorrelatedRangeAction],
    scenarios: &[CorrelationScenario],
) -> Result<CorrelationEnvelopeEvaluation, CorrelationEnvelopeError> {
    if scenarios.is_empty() {
        return Err(CorrelationEnvelopeError::EmptyScenarioSet);
    }
    let mut scenario_ids = BTreeSet::new();
    let mut metrics = Vec::with_capacity(scenarios.len());

    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() {
            return Err(CorrelationEnvelopeError::EmptyScenarioId);
        }
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(CorrelationEnvelopeError::DuplicateScenarioId(
                scenario.scenario_id.clone(),
            ));
        }
        let fisher = correlation_scenario_fisher(actions, scenario)?;
        let rank = fisher.rank(1.0e-10);
        let determinant = if rank == 2 {
            fisher.determinant().max(0.0)
        } else {
            0.0
        };
        let log_determinant = (rank == 2).then(|| determinant.ln());
        let condition_number = fisher.condition_number(1.0e-10);
        metrics.push(CorrelationScenarioMetric {
            scenario_id: scenario.scenario_id.clone(),
            fisher,
            rank,
            determinant,
            log_determinant,
            condition_number,
        });
    }

    let worst = metrics
        .iter()
        .min_by(|a, b| {
            let (a_min, _) = a.fisher.eigenvalues();
            let (b_min, _) = b.fisher.eigenvalues();
            a_min.total_cmp(&b_min)
        })
        .expect("nonempty scenario set validated");
    let (worst_case_minimum_eigenvalue, _) = worst.fisher.eigenvalues();
    let worst_case_scenario_id = worst.scenario_id.clone();
    let worst_case_rank = worst.rank;
    let worst_case_determinant = worst.determinant;
    let worst_case_log_determinant = worst.log_determinant;

    Ok(CorrelationEnvelopeEvaluation {
        scenario_metrics: metrics,
        worst_case_scenario_id,
        worst_case_rank,
        worst_case_minimum_eigenvalue,
        worst_case_determinant,
        worst_case_log_determinant,
    })
}


#[derive(Clone, Debug, PartialEq)]
pub struct RobustCorrelationSelection {
    pub action_ids: Vec<String>,
    pub total_cost_units: u32,
    pub envelope: CorrelationEnvelopeEvaluation,
}

fn prefer_correlation_plan(
    candidate: &RobustCorrelationSelection,
    current: &RobustCorrelationSelection,
) -> bool {
    match candidate.envelope.worst_case_rank.cmp(&current.envelope.worst_case_rank) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            match candidate
                .envelope
                .worst_case_minimum_eigenvalue
                .total_cmp(&current.envelope.worst_case_minimum_eigenvalue)
            {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => {
                    match candidate.total_cost_units.cmp(&current.total_cost_units) {
                        std::cmp::Ordering::Less => true,
                        std::cmp::Ordering::Greater => false,
                        std::cmp::Ordering::Equal => candidate.action_ids < current.action_ids,
                    }
                }
            }
        }
    }
}

/// Exhaustive small-instance action selection under a frozen family of
/// pairwise-correlation matrices.
///
/// The objective is lexicographic: maximize the worst-case observable rank,
/// then maximize the minimum FIM eigenvalue across correlation scenarios,
/// then minimize cost and use stable action IDs as the final tie-break. This
/// is exact over the finite action and scenario sets, not a continuous
/// interval-correlation certificate.
pub fn robust_select_correlation_envelope(
    actions: &[CorrelatedRangeAction],
    scenarios: &[CorrelationScenario],
    budget_units: u32,
) -> Result<RobustCorrelationSelection, CorrelationEnvelopeError> {
    let ordered = validate_actions(actions)?;
    // Validate the full correlation hypotheses before projecting them onto
    // subsets. Missing pairs must never become implicit zero correlations.
    evaluate_correlation_envelope(&ordered, scenarios)?;

    let mut best: Option<RobustCorrelationSelection> = None;
    for mask in 1_u64..(1_u64 << ordered.len()) {
        let mut cost = 0_u32;
        let mut feasible = true;
        for (index, action) in ordered.iter().enumerate() {
            if mask & (1_u64 << index) != 0 {
                match cost.checked_add(action.cost_units) {
                    Some(next) if next <= budget_units => cost = next,
                    _ => {
                        feasible = false;
                        break;
                    }
                }
            }
        }
        if !feasible {
            continue;
        }

        let subset: Vec<_> = ordered
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1_u64 << index) != 0)
            .map(|(_, action)| action.clone())
            .collect();
        let subset_ids: BTreeSet<String> =
            subset.iter().map(|action| action.id.clone()).collect();
        let projected_scenarios: Vec<_> = scenarios
            .iter()
            .map(|scenario| CorrelationScenario {
                scenario_id: scenario.scenario_id.clone(),
                pairs: scenario
                    .pairs
                    .iter()
                    .filter(|pair| {
                        subset_ids.contains(&pair.action_a)
                            && subset_ids.contains(&pair.action_b)
                    })
                    .cloned()
                    .collect(),
            })
            .collect();
        let envelope = evaluate_correlation_envelope(&subset, &projected_scenarios)?;
        let candidate = RobustCorrelationSelection {
            action_ids: subset.iter().map(|action| action.id.clone()).collect(),
            total_cost_units: cost,
            envelope,
        };

        if best
            .as_ref()
            .is_none_or(|current| prefer_correlation_plan(&candidate, current))
        {
            best = Some(candidate);
        }
    }

    best.ok_or(CorrelationEnvelopeError::NoFeasibleSelection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(rho: f64) -> Vec<PairwiseCorrelation> {
        vec![PairwiseCorrelation {
            action_a: "a".to_owned(),
            action_b: "b".to_owned(),
            rho,
        }]
    }

    fn actions() -> Vec<CorrelatedRangeAction> {
        vec![
            CorrelatedRangeAction::independent("a", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b", 0.0, 1.0, 1),
        ]
    }

    fn orthogonal_actions() -> Vec<CorrelatedRangeAction> {
        vec![
            CorrelatedRangeAction::independent("a", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b", std::f64::consts::FRAC_PI_2, 1.0, 1),
        ]
    }

    fn scenario(id: &str, rho: f64) -> CorrelationScenario {
        CorrelationScenario {
            scenario_id: id.to_owned(),
            pairs: pair(rho),
        }
    }


    fn complete_pair_scenario(
        id: &str,
        ids: &[&str],
        correlated_pair: Option<(&str, &str, f64)>,
    ) -> CorrelationScenario {
        let mut pairs = Vec::new();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let rho = correlated_pair
                    .filter(|(a, b, _)| {
                        (*a == ids[i] && *b == ids[j]) || (*a == ids[j] && *b == ids[i])
                    })
                    .map(|(_, _, value)| value)
                    .unwrap_or(0.0);
                pairs.push(PairwiseCorrelation {
                    action_a: ids[i].to_owned(),
                    action_b: ids[j].to_owned(),
                    rho,
                });
            }
        }
        CorrelationScenario {
            scenario_id: id.to_owned(),
            pairs,
        }
    }

    #[test]
    fn robust_selector_chooses_full_rank_set_under_correlation_uncertainty() {
        let actions = vec![
            CorrelatedRangeAction::independent("a-east", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b-east-copy", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("c-north", std::f64::consts::FRAC_PI_2, 1.0, 1),
            CorrelatedRangeAction::independent("d-diagonal", std::f64::consts::FRAC_PI_4, 1.0, 1),
        ];
        let ids = ["a-east", "b-east-copy", "c-north", "d-diagonal"];
        let scenarios = vec![
            complete_pair_scenario("independent", &ids, None),
            complete_pair_scenario("shared-clock", &ids, Some(("a-east", "b-east-copy", 0.9))),
        ];

        let selected = robust_select_correlation_envelope(&actions, &scenarios, 2).unwrap();
        assert_eq!(selected.envelope.worst_case_rank, 2);
        assert!(selected.envelope.worst_case_minimum_eigenvalue > 0.99);
        assert!(!selected.action_ids.contains(&"a-east".to_owned())
            || !selected.action_ids.contains(&"b-east-copy".to_owned()));
        assert_eq!(selected.total_cost_units, 2);
    }

    #[test]
    fn robust_selector_is_deterministic_under_candidate_and_pair_permutation() {
        let actions = vec![
            CorrelatedRangeAction::independent("a-east", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b-east-copy", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("c-north", std::f64::consts::FRAC_PI_2, 1.0, 1),
            CorrelatedRangeAction::independent("d-diagonal", std::f64::consts::FRAC_PI_4, 1.0, 1),
        ];
        let ids = ["a-east", "b-east-copy", "c-north", "d-diagonal"];
        let scenarios = vec![
            complete_pair_scenario("independent", &ids, None),
            complete_pair_scenario("shared-clock", &ids, Some(("a-east", "b-east-copy", 0.9))),
        ];
        let first = robust_select_correlation_envelope(&actions, &scenarios, 2).unwrap();
        let mut reverse_actions = actions;
        reverse_actions.reverse();
        let mut reverse_scenarios = scenarios;
        for scenario in &mut reverse_scenarios {
            scenario.pairs.reverse();
        }
        let second =
            robust_select_correlation_envelope(&reverse_actions, &reverse_scenarios, 2).unwrap();

        assert_eq!(first.action_ids, second.action_ids);
        assert_eq!(
            first.envelope.worst_case_minimum_eigenvalue,
            second.envelope.worst_case_minimum_eigenvalue
        );
    }

    #[test]
    fn robust_selector_rejects_zero_budget_or_no_feasible_plan() {
        let actions = actions();
        let scenario = scenario("independent", 0.0);
        assert_eq!(
            robust_select_correlation_envelope(&actions, &[scenario], 0),
            Err(CorrelationEnvelopeError::NoFeasibleSelection)
        );
    }

    #[test]
    fn positive_cross_correlation_reduces_information_without_changing_source_count() {
        let actions = actions();
        let low = correlation_scenario_fisher(&actions, &scenario("rho-0", 0.0)).unwrap();
        let medium = correlation_scenario_fisher(&actions, &scenario("rho-05", 0.5)).unwrap();
        let high = correlation_scenario_fisher(&actions, &scenario("rho-09", 0.9)).unwrap();

        assert!((low.xx - 2.0).abs() < 1.0e-12);
        assert!((medium.xx - (4.0 / 3.0)).abs() < 1.0e-12);
        assert!((high.xx - (2.0 / 1.9)).abs() < 1.0e-12);
    }

    #[test]
    fn finite_envelope_reports_adversarial_worst_case() {
        let evaluation = evaluate_correlation_envelope(
            &orthogonal_actions(),
            &[scenario("independent", 0.0), scenario("moderate", 0.5), scenario("high", 0.9)],
        ).unwrap();

        assert_eq!(evaluation.worst_case_scenario_id, "high");
        assert_eq!(evaluation.worst_case_rank, 2);
        assert!((evaluation.worst_case_minimum_eigenvalue - (1.0 / 1.9)).abs() < 1.0e-12);
        assert!(evaluation.worst_case_determinant > 5.0);
    }

    #[test]
    fn incomplete_duplicate_and_unknown_pairs_are_rejected() {
        let actions = actions();
        assert!(matches!(
            correlation_scenario_fisher(&actions, &CorrelationScenario {
                scenario_id: "missing".to_owned(),
                pairs: vec![],
            }),
            Err(CorrelationEnvelopeError::MissingPair(_, _))
        ));
        let duplicate = CorrelationScenario {
            scenario_id: "duplicate".to_owned(),
            pairs: vec![
                PairwiseCorrelation { action_a: "a".into(), action_b: "b".into(), rho: 0.1 },
                PairwiseCorrelation { action_a: "b".into(), action_b: "a".into(), rho: 0.2 },
            ],
        };
        assert!(matches!(
            correlation_scenario_fisher(&actions, &duplicate),
            Err(CorrelationEnvelopeError::DuplicatePair(_, _))
        ));
        let unknown = CorrelationScenario {
            scenario_id: "unknown".to_owned(),
            pairs: vec![PairwiseCorrelation { action_a: "a".into(), action_b: "ghost".into(), rho: 0.0 }],
        };
        assert!(matches!(
            correlation_scenario_fisher(&actions, &unknown),
            Err(CorrelationEnvelopeError::UnknownAction(_))
        ));
    }

    #[test]
    fn non_positive_definite_correlation_matrices_are_rejected() {
        let actions = vec![
            CorrelatedRangeAction::independent("a", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b", 1.0, 1.0, 1),
            CorrelatedRangeAction::independent("c", 2.0, 1.0, 1),
        ];
        let scenario = CorrelationScenario {
            scenario_id: "not-psd".to_owned(),
            pairs: vec![
                PairwiseCorrelation { action_a: "a".into(), action_b: "b".into(), rho: 0.9 },
                PairwiseCorrelation { action_a: "a".into(), action_b: "c".into(), rho: 0.9 },
                PairwiseCorrelation { action_a: "b".into(), action_b: "c".into(), rho: -0.9 },
            ],
        };
        assert!(matches!(
            correlation_scenario_fisher(&actions, &scenario),
            Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(_))
        ));
    }


    #[test]
    fn singular_correlation_matrix_is_rejected_as_not_positive_definite() {
        let actions = actions();
        assert!(matches!(
            correlation_scenario_fisher(&actions, &scenario("singular", 1.0)),
            Err(CorrelationEnvelopeError::CovarianceNotPositiveDefinite(id)) if id == "singular"
        ));
    }

    #[test]
    fn non_finite_bearing_is_rejected_before_matrix_construction() {
        let mut actions = actions();
        actions[0].line_of_sight_rad = f64::NAN;
        assert!(matches!(
            correlation_scenario_fisher(&actions, &scenario("bad-bearing", 0.0)),
            Err(CorrelationEnvelopeError::InvalidBearing(id)) if id == "a"
        ));
    }

    #[test]
    fn input_and_pair_order_are_permutation_invariant() {
        let a = actions();
        let b = vec![PairwiseCorrelation { action_a: "b".into(), action_b: "a".into(), rho: 0.5 }];
        let first = correlation_scenario_fisher(&a, &CorrelationScenario {
            scenario_id: "order".into(), pairs: pair(0.5),
        }).unwrap();
        let mut reversed_actions = a;
        reversed_actions.reverse();
        let second = correlation_scenario_fisher(&reversed_actions, &CorrelationScenario {
            scenario_id: "order".into(), pairs: b,
        }).unwrap();
        assert!((first.xx-second.xx).abs()<1.0e-12);
        assert!((first.xy-second.xy).abs()<1.0e-12);
        assert!((first.yy-second.yy).abs()<1.0e-12);
    }
}
