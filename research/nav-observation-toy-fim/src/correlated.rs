//! Correlation-aware linearized range information.
//!
//! This is an isolated research benchmark companion to the independent-noise
//! FIM toy. It models one explicit scalar shared error per declared
//! common-mode group:
//!
//! R = diag(sigma_private^2) + sum_g tau_g^2 * 1_g * 1_g^T
//! J = H^T R^-1 H
//!
//! A group is an explicit modelling input. Different IDs or modalities do
//! not prove independence. The shared-error model is deliberately narrow:
//! it does not represent arbitrary clock, calibration, anchor-survey or map
//! correlations, and it is not an integrity bound.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::{DesignCriterion, Fisher2};

const MAX_CORRELATED_ACTIONS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelatedRangeAction {
    pub id: String,
    pub line_of_sight_rad: f64,
    pub independent_sigma_m: f64,
    pub cost_units: u32,
    pub common_mode_group: Option<String>,
    pub common_mode_sigma_m: Option<f64>,
}

impl CorrelatedRangeAction {
    pub fn independent(
        id: impl Into<String>,
        line_of_sight_rad: f64,
        sigma_m: f64,
        cost_units: u32,
    ) -> Self {
        Self {
            id: id.into(),
            line_of_sight_rad,
            independent_sigma_m: sigma_m,
            cost_units,
            common_mode_group: None,
            common_mode_sigma_m: None,
        }
    }

    pub fn with_common_mode(
        id: impl Into<String>,
        line_of_sight_rad: f64,
        independent_sigma_m: f64,
        common_mode_group: impl Into<String>,
        common_mode_sigma_m: f64,
        cost_units: u32,
    ) -> Self {
        Self {
            id: id.into(),
            line_of_sight_rad,
            independent_sigma_m,
            cost_units,
            common_mode_group: Some(common_mode_group.into()),
            common_mode_sigma_m: Some(common_mode_sigma_m),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CorrelatedError {
    TooManyActions(usize),
    EmptyId,
    DuplicateId(String),
    InvalidBearing(String),
    InvalidIndependentSigma(String),
    InvalidCommonMode(String),
    InconsistentCommonModeSigma(String),
    MissingCommonModeSigma(String),
    UnexpectedCommonModeSigma(String),
    ZeroCost(String),
    NonPositiveDefiniteCovariance,
    NonFiniteInformation,
    InvalidRankTolerance,
    CostOverflow,
}

impl fmt::Display for CorrelatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyActions(n) => write!(f, "{n} actions exceed the correlated exhaustive limit of {MAX_CORRELATED_ACTIONS}"),
            Self::EmptyId => write!(f, "action ID must not be empty"),
            Self::DuplicateId(id) => write!(f, "duplicate action ID: {id}"),
            Self::InvalidBearing(id) => write!(f, "invalid bearing for action: {id}"),
            Self::InvalidIndependentSigma(id) => write!(f, "invalid independent sigma for action: {id}"),
            Self::InvalidCommonMode(id) => write!(f, "invalid common-mode group/sigma for action: {id}"),
            Self::InconsistentCommonModeSigma(group) => write!(f, "common-mode sigma differs within group: {group}"),
            Self::MissingCommonModeSigma(id) => write!(f, "common-mode group lacks common-mode sigma: {id}"),
            Self::UnexpectedCommonModeSigma(id) => write!(f, "common-mode sigma supplied without a group: {id}"),
            Self::ZeroCost(id) => write!(f, "action cost must be positive: {id}"),
            Self::NonPositiveDefiniteCovariance => write!(f, "observation covariance is not positive definite"),
            Self::NonFiniteInformation => write!(f, "correlated Fisher information is non-finite"),
            Self::InvalidRankTolerance => write!(f, "rank tolerance must be finite and nonnegative"),
            Self::CostOverflow => write!(f, "total action cost overflowed u32"),
        }
    }
}

impl std::error::Error for CorrelatedError {}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelatedSelection {
    pub action_ids: Vec<String>,
    pub total_cost_units: u32,
    pub fisher: Fisher2,
    pub rank: u8,
    pub minimum_eigenvalue: f64,
    pub log_determinant: Option<f64>,
    pub a_optimal_trace_inverse: Option<f64>,
    pub condition_number: Option<f64>,
}

fn canonical(actions: &[CorrelatedRangeAction]) -> Result<Vec<CorrelatedRangeAction>, CorrelatedError> {
    if actions.len() > MAX_CORRELATED_ACTIONS {
        return Err(CorrelatedError::TooManyActions(actions.len()));
    }

    let mut ids = BTreeSet::new();
    let mut group_sigmas: BTreeMap<String, f64> = BTreeMap::new();
    let mut ordered = actions.to_vec();

    for action in &ordered {
        if action.id.trim().is_empty() {
            return Err(CorrelatedError::EmptyId);
        }
        if !ids.insert(action.id.clone()) {
            return Err(CorrelatedError::DuplicateId(action.id.clone()));
        }
        if !action.line_of_sight_rad.is_finite() {
            return Err(CorrelatedError::InvalidBearing(action.id.clone()));
        }
        let private_variance = action.independent_sigma_m * action.independent_sigma_m;
        if !action.independent_sigma_m.is_finite()
            || action.independent_sigma_m <= 0.0
            || !private_variance.is_finite()
            || private_variance <= 0.0
        {
            return Err(CorrelatedError::InvalidIndependentSigma(action.id.clone()));
        }
        if action.cost_units == 0 {
            return Err(CorrelatedError::ZeroCost(action.id.clone()));
        }

        match (&action.common_mode_group, action.common_mode_sigma_m) {
            (None, None) => {}
            (None, Some(_)) => return Err(CorrelatedError::UnexpectedCommonModeSigma(action.id.clone())),
            (Some(_), None) => return Err(CorrelatedError::MissingCommonModeSigma(action.id.clone())),
            (Some(group), Some(sigma)) => {
                if group.trim().is_empty() || !sigma.is_finite() || sigma <= 0.0 {
                    return Err(CorrelatedError::InvalidCommonMode(action.id.clone()));
                }
                let variance = sigma * sigma;
                if !variance.is_finite() || variance <= 0.0 {
                    return Err(CorrelatedError::InvalidCommonMode(action.id.clone()));
                }
                if let Some(existing) = group_sigmas.get(group) {
                    if existing.to_bits() != sigma.to_bits() {
                        return Err(CorrelatedError::InconsistentCommonModeSigma(group.clone()));
                    }
                } else {
                    group_sigmas.insert(group.clone(), sigma);
                }
            }
        }
    }

    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ordered)
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, CorrelatedError> {
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
                if !value.is_finite() || value <= 0.0 {
                    return Err(CorrelatedError::NonPositiveDefiniteCovariance);
                }
                lower[i][j] = value.sqrt();
            } else {
                let divisor = lower[j][j];
                if !divisor.is_finite() || divisor <= 0.0 {
                    return Err(CorrelatedError::NonPositiveDefiniteCovariance);
                }
                lower[i][j] = value / divisor;
                if !lower[i][j].is_finite() {
                    return Err(CorrelatedError::NonPositiveDefiniteCovariance);
                }
            }
        }
    }

    Ok(lower)
}

fn solve_covariance(lower: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, CorrelatedError> {
    let n = rhs.len();
    let mut y = vec![0.0; n];

    // Forward substitution: L y = rhs.
    for i in 0..n {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * y[j];
        }
        y[i] = value / lower[i][i];
        if !y[i].is_finite() {
            return Err(CorrelatedError::NonPositiveDefiniteCovariance);
        }
    }

    // Back substitution: L^T x = y.
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = y[i];
        for j in (i + 1)..n {
            value -= lower[j][i] * x[j];
        }
        x[i] = value / lower[i][i];
        if !x[i].is_finite() {
            return Err(CorrelatedError::NonPositiveDefiniteCovariance);
        }
    }

    Ok(x)
}

/// Compute J = H^T R^-1 H for the declared common-mode additive-error model.
///
/// R_ij = delta_ij * sigma_private_i^2 + tau_g^2 when i,j share group g.
/// The covariance is factored with Cholesky; no matrix inverse is formed.
pub fn correlated_fisher(actions: &[CorrelatedRangeAction]) -> Result<Fisher2, CorrelatedError> {
    let ordered = canonical(actions)?;
    let n = ordered.len();
    if n == 0 {
        return Ok(Fisher2::default());
    }

    let mut covariance = vec![vec![0.0; n]; n];
    let mut directions = Vec::with_capacity(n);
    let mut group_sigmas: BTreeMap<String, f64> = BTreeMap::new();

    for (i, action) in ordered.iter().enumerate() {
        let private_variance = action.independent_sigma_m * action.independent_sigma_m;
        covariance[i][i] = private_variance;
        let c = action.line_of_sight_rad.cos();
        let s = action.line_of_sight_rad.sin();
        directions.push([c, s]);

        if let (Some(group), Some(sigma)) = (&action.common_mode_group, action.common_mode_sigma_m) {
            group_sigmas.insert(group.clone(), sigma);
        }
    }

    for i in 0..n {
        for j in 0..n {
            if let (Some(group_i), Some(group_j)) =
                (&ordered[i].common_mode_group, &ordered[j].common_mode_group)
                && group_i == group_j
            {
                let sigma = *group_sigmas.get(group_i).ok_or_else(|| {
                    CorrelatedError::MissingCommonModeSigma(ordered[i].id.clone())
                })?;
                covariance[i][j] += sigma * sigma;
            }
        }
    }

    let lower = cholesky(&covariance)?;
    let hx: Vec<f64> = directions.iter().map(|v| v[0]).collect();
    let hy: Vec<f64> = directions.iter().map(|v| v[1]).collect();
    let rx = solve_covariance(&lower, &hx)?;
    let ry = solve_covariance(&lower, &hy)?;

    let xx = hx.iter().zip(&rx).map(|(a, b)| a * b).sum::<f64>();
    let xy = hx.iter().zip(&ry).map(|(a, b)| a * b).sum::<f64>();
    let yy = hy.iter().zip(&ry).map(|(a, b)| a * b).sum::<f64>();

    let fisher = Fisher2 { xx, xy, yy };
    let determinant = fisher.determinant();
    if !xx.is_finite() || !xy.is_finite() || !yy.is_finite() || !determinant.is_finite() {
        return Err(CorrelatedError::NonFiniteInformation);
    }
    Ok(fisher)
}

fn from_actions(
    actions: &[CorrelatedRangeAction],
    rank_tolerance: f64,
    mask: u64,
) -> Result<CorrelatedSelection, CorrelatedError> {
    let chosen: Vec<CorrelatedRangeAction> = actions
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1_u64 << i) != 0)
        .map(|(_, a)| a.clone())
        .collect();

    let total_cost_units = chosen.iter().try_fold(0_u32, |acc, a| {
        acc.checked_add(a.cost_units).ok_or(CorrelatedError::CostOverflow)
    })?;
    let fisher = correlated_fisher(&chosen)?;
    let (minimum_eigenvalue, _) = fisher.eigenvalues();
    if !minimum_eigenvalue.is_finite() {
        return Err(CorrelatedError::NonFiniteInformation);
    }

    Ok(CorrelatedSelection {
        action_ids: chosen.iter().map(|a| a.id.clone()).collect(),
        total_cost_units,
        fisher,
        rank: fisher.rank(rank_tolerance),
        minimum_eigenvalue,
        log_determinant: fisher.log_determinant(rank_tolerance),
        a_optimal_trace_inverse: fisher.a_optimal_trace_inverse(rank_tolerance),
        condition_number: fisher.condition_number(rank_tolerance),
    })
}

fn compare_information(a: &CorrelatedSelection, b: &CorrelatedSelection, criterion: DesignCriterion) -> Ordering {
    match a.rank.cmp(&b.rank) {
        Ordering::Equal => {}
        other => return other,
    }
    if a.rank < 2 {
        return a.fisher.trace().total_cmp(&b.fisher.trace());
    }
    match criterion {
        DesignCriterion::DOptimal => a.log_determinant.unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&b.log_determinant.unwrap_or(f64::NEG_INFINITY)),
        DesignCriterion::AOptimal => b.a_optimal_trace_inverse.unwrap_or(f64::INFINITY)
            .total_cmp(&a.a_optimal_trace_inverse.unwrap_or(f64::INFINITY)),
        DesignCriterion::EOptimal => a.minimum_eigenvalue.total_cmp(&b.minimum_eigenvalue),
    }
}

fn prefer(a: &CorrelatedSelection, b: &CorrelatedSelection, criterion: DesignCriterion) -> bool {
    match compare_information(a, b, criterion) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match a.total_cost_units.cmp(&b.total_cost_units) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => a.action_ids < b.action_ids,
        },
    }
}

/// Exact subset oracle for at most 16 candidate observations.
pub fn exhaustive_select_correlated(
    actions: &[CorrelatedRangeAction],
    budget_units: u32,
    criterion: DesignCriterion,
) -> Result<CorrelatedSelection, CorrelatedError> {
    exhaustive_select_correlated_with_tolerance(
        actions,
        budget_units,
        criterion,
        1.0e-10,
    )
}

pub fn exhaustive_select_correlated_with_tolerance(
    actions: &[CorrelatedRangeAction],
    budget_units: u32,
    criterion: DesignCriterion,
    rank_tolerance: f64,
) -> Result<CorrelatedSelection, CorrelatedError> {
    if !rank_tolerance.is_finite() || rank_tolerance < 0.0 {
        return Err(CorrelatedError::InvalidRankTolerance);
    }
    let ordered = canonical(actions)?;
    let subset_count = 1_u64 << ordered.len();
    let mut best: Option<CorrelatedSelection> = None;

    for mask in 0..subset_count {
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

        let selection = from_actions(&ordered, rank_tolerance, mask)?;
        if best.as_ref().is_none_or(|current| prefer(&selection, current, criterion)) {
            best = Some(selection);
        }
    }

    best.ok_or(CorrelatedError::NonFiniteInformation)
}

/// Deterministic greedy comparator using the same correlation model and
/// design objective. It is a baseline, not a global optimality guarantee.
pub fn greedy_select_correlated(
    actions: &[CorrelatedRangeAction],
    budget_units: u32,
    criterion: DesignCriterion,
) -> Result<CorrelatedSelection, CorrelatedError> {
    let ordered = canonical(actions)?;
    let mut current = from_actions(&ordered, 1.0e-10, 0)?;

    loop {
        let mut best_next: Option<CorrelatedSelection> = None;
        let current_ids: BTreeSet<&str> = current.action_ids.iter().map(String::as_str).collect();

        for (index, action) in ordered.iter().enumerate() {
            if current_ids.contains(action.id.as_str()) {
                continue;
            }
            let Some(next_cost) = current.total_cost_units.checked_add(action.cost_units) else {
                continue;
            };
            if next_cost > budget_units {
                continue;
            }
            let next_mask = ordered
                .iter()
                .enumerate()
                .filter(|(_, item)| current_ids.contains(item.id.as_str()))
                .fold(1_u64 << index, |mask, (i, _)| mask | (1_u64 << i));
            let candidate = from_actions(&ordered, 1.0e-10, next_mask)?;
            if candidate.total_cost_units > budget_units
                || compare_information(&candidate, &current, criterion) != Ordering::Greater
            {
                continue;
            }
            if best_next.as_ref().is_none_or(|best| prefer(&candidate, best, criterion)) {
                best_next = Some(candidate);
            }
        }

        match best_next {
            Some(next) => current = next,
            None => return Ok(current),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicated_same_bearing_observations_do_not_gain_independent_precision() {
        let independent = vec![
            CorrelatedRangeAction::independent("a", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b", 0.0, 1.0, 1),
        ];
        let shared = vec![
            CorrelatedRangeAction::with_common_mode("a", 0.0, 1.0, "clock-antenna-A", 1.0, 1),
            CorrelatedRangeAction::with_common_mode("b", 0.0, 1.0, "clock-antenna-A", 1.0, 1),
        ];
        let independent_info = correlated_fisher(&independent).unwrap();
        let shared_info = correlated_fisher(&shared).unwrap();

        assert!((independent_info.xx - 2.0).abs() < 1.0e-12);
        assert!((shared_info.xx - (2.0 / 3.0)).abs() < 1.0e-12);
    }

    #[test]
    fn shared_clock_bias_is_visible_in_cross_axis_information() {
        let actions = vec![
            CorrelatedRangeAction::with_common_mode("east", 0.0, 1.0, "shared-clock", 1.0, 1),
            CorrelatedRangeAction::with_common_mode("north", std::f64::consts::FRAC_PI_2, 1.0, "shared-clock", 1.0, 1),
        ];
        let information = correlated_fisher(&actions).unwrap();

        assert!((information.xx - (2.0 / 3.0)).abs() < 1.0e-12);
        assert!((information.xy + (1.0 / 3.0)).abs() < 1.0e-12);
        assert!((information.yy - (2.0 / 3.0)).abs() < 1.0e-12);
    }

    #[test]
    fn same_domain_repeated_measurements_saturate_to_shared_error_floor() {
        let mut previous = 0.0;
        for n in [2_usize, 4, 8, 16] {
            let actions: Vec<_> = (0..n)
                .map(|i| CorrelatedRangeAction::with_common_mode(
                    format!("rx-{i}"),
                    0.0,
                    0.1,
                    "shared-correction",
                    1.0,
                    1,
                ))
                .collect();
            let information = correlated_fisher(&actions).unwrap().xx;
            assert!(information > previous);
            assert!(information < 1.0);
            previous = information;
        }
        assert!(previous > 0.99);
    }

    #[test]
    fn distinct_declared_common_mode_groups_do_not_share_the_same_latent_error() {
        let actions = vec![
            CorrelatedRangeAction::with_common_mode("a", 0.0, 1.0, "antenna-A", 1.0, 1),
            CorrelatedRangeAction::with_common_mode("b", 0.0, 1.0, "antenna-B", 1.0, 1),
        ];
        let information = correlated_fisher(&actions).unwrap();
        assert!((information.xx - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn permutation_of_inputs_preserves_correlated_information() {
        let actions = vec![
            CorrelatedRangeAction::with_common_mode("a", 0.0, 0.5, "shared", 0.8, 1),
            CorrelatedRangeAction::with_common_mode("b", 0.7, 0.6, "shared", 0.8, 1),
            CorrelatedRangeAction::independent("c", 1.4, 0.9, 1),
        ];
        let forward = correlated_fisher(&actions).unwrap();
        let mut reverse = actions;
        reverse.reverse();
        let backward = correlated_fisher(&reverse).unwrap();
        assert!((forward.xx - backward.xx).abs() < 1.0e-12);
        assert!((forward.xy - backward.xy).abs() < 1.0e-12);
        assert!((forward.yy - backward.yy).abs() < 1.0e-12);
    }

    #[test]
    fn different_common_mode_sigmas_in_one_group_are_rejected() {
        let actions = vec![
            CorrelatedRangeAction::with_common_mode("a", 0.0, 1.0, "same", 0.5, 1),
            CorrelatedRangeAction::with_common_mode("b", 1.0, 1.0, "same", 0.7, 1),
        ];
        assert_eq!(
            correlated_fisher(&actions),
            Err(CorrelatedError::InconsistentCommonModeSigma("same".to_owned()))
        );
    }

    #[test]
    fn criterion_oracle_and_greedy_use_the_correlated_model() {
        let actions = vec![
            CorrelatedRangeAction::with_common_mode("a-colocated-1", 0.0, 1.0, "shared-front-end", 1.0, 1),
            CorrelatedRangeAction::with_common_mode("b-colocated-2", 0.0, 1.0, "shared-front-end", 1.0, 1),
            CorrelatedRangeAction::independent("c-north", std::f64::consts::FRAC_PI_2, 1.0, 1),
            CorrelatedRangeAction::independent("d-diagonal", std::f64::consts::FRAC_PI_4, 1.0, 1),
        ];
        let exact = exhaustive_select_correlated(&actions, 2, DesignCriterion::DOptimal).unwrap();
        let greedy = greedy_select_correlated(&actions, 2, DesignCriterion::DOptimal).unwrap();

        assert!(exact.total_cost_units <= 2);
        assert!(greedy.total_cost_units <= 2);
        assert_eq!(exact.rank, 2);
        assert_eq!(greedy.rank, 2);
        assert!(!exact.action_ids.contains(&"b-colocated-2".to_owned())
            || !exact.action_ids.contains(&"a-colocated-1".to_owned()));
    }

    #[test]
    fn missing_or_unexpected_common_mode_parameters_fail_closed() {
        let missing = CorrelatedRangeAction {
            id: "a".to_owned(),
            line_of_sight_rad: 0.0,
            independent_sigma_m: 1.0,
            cost_units: 1,
            common_mode_group: Some("g".to_owned()),
            common_mode_sigma_m: None,
        };
        assert_eq!(correlated_fisher(&[missing]), Err(CorrelatedError::MissingCommonModeSigma("a".to_owned())));

        let unexpected = CorrelatedRangeAction {
            id: "b".to_owned(),
            line_of_sight_rad: 0.0,
            independent_sigma_m: 1.0,
            cost_units: 1,
            common_mode_group: None,
            common_mode_sigma_m: Some(1.0),
        };
        assert_eq!(correlated_fisher(&[unexpected]), Err(CorrelatedError::UnexpectedCommonModeSigma("b".to_owned())));
    }
}
