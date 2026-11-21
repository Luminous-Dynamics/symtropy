//! Correlation-aware 3D range-observation information.
//!
//! Each action contributes a scalar linearized range measurement. Private
//! errors are independent; actions in the same explicitly declared common-mode
//! group share one additive scalar error. R is assembled as:
//!
//! R = diag(sigma_private^2) + sum_g tau_g^2 1_g 1_g^T
//! J = H^T R^-1 H
//!
//! The model is intentionally explicit and narrow. A group declaration is
//! evidence input, not proof of a physical covariance; unknown dependencies
//! must not be represented as independent. This is a local 3D FIM model, not
//! a nonlinear estimator or a protection-level calculation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::observability_3d::Fisher3;

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelatedRangeAction3D {
    pub id: String,
    pub line_of_sight: [f64; 3],
    pub independent_sigma_m: f64,
    pub common_mode_group: Option<String>,
    pub common_mode_sigma_m: Option<f64>,
    pub cost_units: u32,
}

impl CorrelatedRangeAction3D {
    pub fn independent(
        id: impl Into<String>,
        line_of_sight: [f64; 3],
        sigma_m: f64,
        cost_units: u32,
    ) -> Self {
        Self {
            id: id.into(),
            line_of_sight,
            independent_sigma_m: sigma_m,
            common_mode_group: None,
            common_mode_sigma_m: None,
            cost_units,
        }
    }

    pub fn with_common_mode(
        id: impl Into<String>,
        line_of_sight: [f64; 3],
        independent_sigma_m: f64,
        group: impl Into<String>,
        common_mode_sigma_m: f64,
        cost_units: u32,
    ) -> Self {
        Self {
            id: id.into(),
            line_of_sight,
            independent_sigma_m,
            common_mode_group: Some(group.into()),
            common_mode_sigma_m: Some(common_mode_sigma_m),
            cost_units,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Correlated3DError {
    EmptyActions,
    TooManyActions(usize),
    EmptyId,
    DuplicateId(String),
    InvalidDirection(String),
    InvalidPrivateSigma(String),
    InvalidCommonModeSigma(String),
    MissingCommonModeSigma(String),
    UnexpectedCommonModeSigma(String),
    EmptyCommonModeGroup(String),
    InconsistentCommonModeSigma(String),
    ZeroCost(String),
    InvalidCovariance,
    NonFiniteInformation,
    NoFeasibleSelection,
}

impl fmt::Display for Correlated3DError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActions => write!(f, "at least one 3D action is required"),
            Self::TooManyActions(n) => write!(f, "{n} actions exceed the 3D correlation oracle limit of 16"),
            Self::EmptyId => write!(f, "action ID must not be empty"),
            Self::DuplicateId(id) => write!(f, "duplicate action ID: {id}"),
            Self::InvalidDirection(id) => write!(f, "invalid unit line-of-sight direction: {id}"),
            Self::InvalidPrivateSigma(id) => write!(f, "invalid private range sigma: {id}"),
            Self::InvalidCommonModeSigma(id) => write!(f, "invalid shared-error sigma: {id}"),
            Self::MissingCommonModeSigma(id) => write!(f, "common-mode group lacks a declared sigma: {id}"),
            Self::UnexpectedCommonModeSigma(id) => write!(f, "common-mode sigma supplied without a group: {id}"),
            Self::EmptyCommonModeGroup(id) => write!(f, "common-mode group ID is empty for action: {id}"),
            Self::InconsistentCommonModeSigma(group) => write!(f, "inconsistent shared-error sigma within group: {group}"),
            Self::ZeroCost(id) => write!(f, "action cost must be positive: {id}"),
            Self::InvalidCovariance => write!(f, "measurement covariance is not finite positive definite"),
            Self::NonFiniteInformation => write!(f, "3D Fisher information is non-finite"),
            Self::NoFeasibleSelection => write!(f, "no nonempty 3D action subset fits the budget"),
        }
    }
}
impl std::error::Error for Correlated3DError {}

fn canonical_actions(
    actions: &[CorrelatedRangeAction3D],
) -> Result<Vec<CorrelatedRangeAction3D>, Correlated3DError> {
    if actions.is_empty() {
        return Err(Correlated3DError::EmptyActions);
    }
    if actions.len() > 16 {
        return Err(Correlated3DError::TooManyActions(actions.len()));
    }

    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::<String, f64>::new();
    let mut ordered = actions.to_vec();

    for action in &ordered {
        if action.id.trim().is_empty() {
            return Err(Correlated3DError::EmptyId);
        }
        if !ids.insert(action.id.clone()) {
            return Err(Correlated3DError::DuplicateId(action.id.clone()));
        }
        let norm = action.line_of_sight.iter().map(|value| value * value).sum::<f64>().sqrt();
        if !action.line_of_sight.iter().all(|value| value.is_finite())
            || !norm.is_finite()
            || (norm - 1.0).abs() > 1.0e-6
        {
            return Err(Correlated3DError::InvalidDirection(action.id.clone()));
        }
        let variance = action.independent_sigma_m * action.independent_sigma_m;
        let precision = 1.0 / variance;
        if !action.independent_sigma_m.is_finite()
            || action.independent_sigma_m <= 0.0
            || !variance.is_finite()
            || variance <= 0.0
            || !precision.is_finite()
            || precision <= 0.0
        {
            return Err(Correlated3DError::InvalidPrivateSigma(action.id.clone()));
        }
        if action.cost_units == 0 {
            return Err(Correlated3DError::ZeroCost(action.id.clone()));
        }

        match (&action.common_mode_group, action.common_mode_sigma_m) {
            (None, None) => {}
            (None, Some(_)) => {
                return Err(Correlated3DError::UnexpectedCommonModeSigma(action.id.clone()));
            }
            (Some(group), None) => {
                if group.trim().is_empty() {
                    return Err(Correlated3DError::EmptyCommonModeGroup(action.id.clone()));
                }
                return Err(Correlated3DError::MissingCommonModeSigma(action.id.clone()));
            }
            (Some(group), Some(sigma)) => {
                if group.trim().is_empty() {
                    return Err(Correlated3DError::EmptyCommonModeGroup(action.id.clone()));
                }
                if !sigma.is_finite() || sigma < 0.0 {
                    return Err(Correlated3DError::InvalidCommonModeSigma(action.id.clone()));
                }
                if let Some(existing) = groups.get(group) {
                    if (existing - sigma).abs() > 1.0e-12 {
                        return Err(Correlated3DError::InconsistentCommonModeSigma(group.clone()));
                    }
                } else {
                    groups.insert(group.clone(), sigma);
                }
            }
        }
    }

    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ordered)
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, Correlated3DError> {
    let n = matrix.len();
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let correction = lower[i][..j]
                .iter()
                .zip(lower[j][..j].iter())
                .map(|(a, b)| a * b)
                .sum::<f64>();
            let value = matrix[i][j] - correction;
            if i == j {
                if !value.is_finite() || value <= 1.0e-12 {
                    return Err(Correlated3DError::InvalidCovariance);
                }
                lower[i][j] = value.sqrt();
            } else {
                let divisor = lower[j][j];
                if !divisor.is_finite() || divisor <= 0.0 {
                    return Err(Correlated3DError::InvalidCovariance);
                }
                lower[i][j] = value / divisor;
                if !lower[i][j].is_finite() {
                    return Err(Correlated3DError::InvalidCovariance);
                }
            }
        }
    }
    Ok(lower)
}

fn solve_cholesky(lower: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, Correlated3DError> {
    let n = rhs.len();
    let mut y = vec![0.0; n];
    for i in 0..n {
        let correction = lower[i][..i]
            .iter()
            .zip(y[..i].iter())
            .map(|(a, b)| a * b)
            .sum::<f64>();
        y[i] = (rhs[i] - correction) / lower[i][i];
        if !y[i].is_finite() {
            return Err(Correlated3DError::InvalidCovariance);
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let correction = (i + 1..n)
            .map(|j| lower[j][i] * x[j])
            .sum::<f64>();
        x[i] = (y[i] - correction) / lower[i][i];
        if !x[i].is_finite() {
            return Err(Correlated3DError::InvalidCovariance);
        }
    }
    Ok(x)
}

pub fn correlated_fisher_3d(
    actions: &[CorrelatedRangeAction3D],
) -> Result<Fisher3, Correlated3DError> {
    let ordered = canonical_actions(actions)?;
    let n = ordered.len();
    let mut covariance = vec![vec![0.0; n]; n];
    let mut h = Vec::with_capacity(n);

    for (i, action) in ordered.iter().enumerate() {
        covariance[i][i] = action.independent_sigma_m * action.independent_sigma_m;
        h.push(action.line_of_sight);
        for j in i..n {
            if let (Some(group_i), Some(group_j)) =
                (&action.common_mode_group, &ordered[j].common_mode_group)
                && group_i == group_j
            {
                let sigma_i = action.common_mode_sigma_m.ok_or_else(|| {
                    Correlated3DError::MissingCommonModeSigma(action.id.clone())
                })?;
                let sigma_j = ordered[j].common_mode_sigma_m.ok_or_else(|| {
                    Correlated3DError::MissingCommonModeSigma(ordered[j].id.clone())
                })?;
                if (sigma_i - sigma_j).abs() > 1.0e-12 {
                    return Err(Correlated3DError::InconsistentCommonModeSigma(group_i.clone()));
                }
                covariance[i][j] += sigma_i * sigma_i;
                if i != j {
                    covariance[j][i] += sigma_i * sigma_i;
                }
            }
        }
    }

    let lower = cholesky(&covariance)?;
    let hx = h.iter().map(|v| v[0]).collect::<Vec<_>>();
    let hy = h.iter().map(|v| v[1]).collect::<Vec<_>>();
    let hz = h.iter().map(|v| v[2]).collect::<Vec<_>>();
    let rx = solve_cholesky(&lower, &hx)?;
    let ry = solve_cholesky(&lower, &hy)?;
    let rz = solve_cholesky(&lower, &hz)?;

    let mut fisher = Fisher3::default();
    let columns = [&rx, &ry, &rz];
    for (row_index, row_values) in fisher.matrix.iter_mut().enumerate() {
        for (column_index, solved_column) in columns.iter().enumerate() {
            row_values[column_index] = h
                .iter()
                .zip(solved_column.iter())
                .map(|(direction, solved)| direction[row_index] * solved)
                .sum();
        }
    }
    if !fisher.matrix.iter().flatten().all(|value| value.is_finite())
        || !fisher.eigenvalues().iter().all(|value| value.is_finite())
    {
        return Err(Correlated3DError::NonFiniteInformation);
    }
    Ok(fisher)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelatedSelection3D {
    pub action_ids: Vec<String>,
    pub total_cost_units: u32,
    pub fisher: Fisher3,
    pub rank: u8,
    pub eigenvalues: [f64; 3],
}

fn prefer_selection(
    candidate: &CorrelatedSelection3D,
    current: &CorrelatedSelection3D,
) -> bool {
    match candidate.rank.cmp(&current.rank) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            match candidate.eigenvalues[0].total_cmp(&current.eigenvalues[0]) {
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

/// Exhaustive 3D E-optimal subset selector using the same shared-error model.
pub fn exhaustive_select_correlated_3d(
    actions: &[CorrelatedRangeAction3D],
    budget_units: u32,
) -> Result<CorrelatedSelection3D, Correlated3DError> {
    let ordered = canonical_actions(actions)?;
    let mut best: Option<CorrelatedSelection3D> = None;
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

        let subset = ordered.iter().enumerate()
            .filter(|(index, _)| mask & (1_u64 << index) != 0)
            .map(|(_, action)| action.clone())
            .collect::<Vec<_>>();
        let fisher = correlated_fisher_3d(&subset)?;
        let rank = fisher.rank(1.0e-10);
        let candidate = CorrelatedSelection3D {
            action_ids: subset.iter().map(|action| action.id.clone()).collect(),
            total_cost_units: cost,
            fisher,
            rank,
            eigenvalues: fisher.eigenvalues(),
        };
        if best.as_ref().is_none_or(|current| prefer_selection(&candidate, current)) {
            best = Some(candidate);
        }
    }
    best.ok_or(Correlated3DError::NoFeasibleSelection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orthogonal() -> Vec<CorrelatedRangeAction3D> {
        vec![
            CorrelatedRangeAction3D::independent("east", [1.0, 0.0, 0.0], 1.0, 1),
            CorrelatedRangeAction3D::independent("north", [0.0, 1.0, 0.0], 1.0, 1),
            CorrelatedRangeAction3D::independent("up", [0.0, 0.0, 1.0], 1.0, 1),
        ]
    }

    #[test]
    fn common_mode_duplicates_do_not_create_independent_precision_in_3d() {
        let independent = vec![
            CorrelatedRangeAction3D::independent("a", [1.0, 0.0, 0.0], 1.0, 1),
            CorrelatedRangeAction3D::independent("b", [1.0, 0.0, 0.0], 1.0, 1),
        ];
        let shared = vec![
            CorrelatedRangeAction3D::with_common_mode("a", [1.0, 0.0, 0.0], 1.0, "clock", 1.0, 1),
            CorrelatedRangeAction3D::with_common_mode("b", [1.0, 0.0, 0.0], 1.0, "clock", 1.0, 1),
        ];
        let independent_fisher = correlated_fisher_3d(&independent).unwrap();
        let shared_fisher = correlated_fisher_3d(&shared).unwrap();
        assert!((independent_fisher.matrix[0][0] - 2.0).abs() < 1.0e-12);
        assert!((shared_fisher.matrix[0][0] - (2.0 / 3.0)).abs() < 1.0e-12);
        assert_eq!(shared_fisher.rank(1.0e-10), 1);
    }

    #[test]
    fn common_mode_3d_fim_preserves_cross_axis_information() {
        let actions = vec![
            CorrelatedRangeAction3D::with_common_mode("a", [1.0, 0.0, 0.0], 0.5, "front-end", 0.8, 1),
            CorrelatedRangeAction3D::with_common_mode("b", [0.0, 1.0, 0.0], 0.5, "front-end", 0.8, 1),
            CorrelatedRangeAction3D::independent("up", [0.0, 0.0, 1.0], 0.5, 1),
        ];
        let fisher = correlated_fisher_3d(&actions).unwrap();
        assert!(fisher.matrix[0][1].abs() > 1.0e-3);
        assert!(fisher.matrix[1][0].abs() > 1.0e-3);
        assert_eq!(fisher.rank(1.0e-10), 3);
    }

    #[test]
    fn planar_correlated_geometry_still_has_no_vertical_information() {
        let actions = vec![
            CorrelatedRangeAction3D::with_common_mode("east", [1.0, 0.0, 0.0], 1.0, "clock", 0.5, 1),
            CorrelatedRangeAction3D::with_common_mode("north", [0.0, 1.0, 0.0], 1.0, "clock", 0.5, 1),
            CorrelatedRangeAction3D::independent("diag", [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0], 1.0, 1),
        ];
        let fisher = correlated_fisher_3d(&actions).unwrap();
        assert_eq!(fisher.rank(1.0e-10), 2);
        assert!(fisher.minimum_eigenvalue() <= 1.0e-12);
    }

    #[test]
    fn input_permutation_does_not_change_correlated_3d_information() {
        let actions = orthogonal().into_iter().map(|mut action| {
            action.common_mode_group = Some("clock".into());
            action.common_mode_sigma_m = Some(0.25);
            action
        }).collect::<Vec<_>>();
        let first = correlated_fisher_3d(&actions).unwrap();
        let mut reversed = actions;
        reversed.reverse();
        let second = correlated_fisher_3d(&reversed).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                assert!((first.matrix[i][j] - second.matrix[i][j]).abs() < 1.0e-12);
            }
        }
    }

    #[test]
    fn inconsistent_group_sigma_and_missing_group_sigma_are_rejected() {
        let inconsistent = vec![
            CorrelatedRangeAction3D::with_common_mode("a", [1.0, 0.0, 0.0], 1.0, "g", 0.2, 1),
            CorrelatedRangeAction3D::with_common_mode("b", [0.0, 1.0, 0.0], 1.0, "g", 0.3, 1),
        ];
        assert!(matches!(
            correlated_fisher_3d(&inconsistent),
            Err(Correlated3DError::InconsistentCommonModeSigma(_))
        ));
        let mut missing = CorrelatedRangeAction3D::independent("a", [1.0, 0.0, 0.0], 1.0, 1);
        missing.common_mode_group = Some("g".into());
        assert!(matches!(
            correlated_fisher_3d(&[missing]),
            Err(Correlated3DError::MissingCommonModeSigma(_))
        ));
    }

    #[test]
    fn correlated_3d_selector_prefers_full_rank_and_vertical_coverage() {
        let actions = vec![
            CorrelatedRangeAction3D::with_common_mode("east-a", [1.0, 0.0, 0.0], 1.0, "shared", 0.8, 1),
            CorrelatedRangeAction3D::with_common_mode("east-b", [1.0, 0.0, 0.0], 1.0, "shared", 0.8, 1),
            CorrelatedRangeAction3D::independent("north", [0.0, 1.0, 0.0], 1.0, 1),
            CorrelatedRangeAction3D::independent("up", [0.0, 0.0, 1.0], 1.0, 1),
        ];
        let selected = exhaustive_select_correlated_3d(&actions, 3).unwrap();
        assert_eq!(selected.rank, 3);
        assert!(selected.action_ids.contains(&"up".to_owned()));
        assert!(!(selected.action_ids.contains(&"east-a".to_owned())
            && selected.action_ids.contains(&"east-b".to_owned())));
    }
}
