//! Exact small-instance 3D range-observation Fisher-information oracle.
//!
//! This module generalizes the 2D geometry oracle to three spatial axes. It
//! exposes the full 3D spectrum so a horizontal ENU projection cannot conceal
//! an unobservable vertical direction. It is a local linearized information
//! model, not a nonlinear estimator or a protection-level calculation.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

const MAX_ACTIONS_3D: usize = 16;
pub const DEFAULT_RANK_TOLERANCE_3D: f64 = 1.0e-10;

#[derive(Clone, Debug, PartialEq)]
pub struct RangeAction3D {
    pub id: String,
    /// Unit line-of-sight vector in a declared local 3D frame.
    pub line_of_sight: [f64; 3],
    pub sigma_m: f64,
    pub cost_units: u32,
}

impl RangeAction3D {
    pub fn new(
        id: impl Into<String>,
        line_of_sight: [f64; 3],
        sigma_m: f64,
        cost_units: u32,
    ) -> Self {
        Self { id: id.into(), line_of_sight, sigma_m, cost_units }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fisher3 {
    pub matrix: [[f64; 3]; 3],
}

impl Fisher3 {
    pub fn add_range(&mut self, direction: [f64; 3], sigma_m: f64) {
        let precision = 1.0 / (sigma_m * sigma_m);
        for i in 0..3 {
            for j in 0..3 {
                self.matrix[i][j] += precision * direction[i] * direction[j];
            }
        }
    }

    pub fn trace(self) -> f64 {
        self.matrix[0][0] + self.matrix[1][1] + self.matrix[2][2]
    }

    /// Sorted ascending eigenvalues from a symmetric Jacobi diagonalization.
    pub fn eigenvalues(self) -> [f64; 3] {
        let mut a = self.matrix;
        for _ in 0..48 {
            let mut p = 0;
            let mut q = 1;
            let mut largest = a[0][1].abs();
            for (i, j) in [(0, 2), (1, 2)] {
                if a[i][j].abs() > largest {
                    largest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
            if largest <= 1.0e-14 {
                break;
            }

            let apq = a[p][q];
            let tau = (a[q][q] - a[p][p]) / (2.0 * apq);
            let t = if tau >= 0.0 {
                1.0 / (tau + (1.0 + tau * tau).sqrt())
            } else {
                -1.0 / (-tau + (1.0 + tau * tau).sqrt())
            };
            let c = 1.0 / (1.0 + t * t).sqrt();
            let s = t * c;

            let app = a[p][p];
            let aqq = a[q][q];
            a[p][p] = app - t * apq;
            a[q][q] = aqq + t * apq;
            a[p][q] = 0.0;
            a[q][p] = 0.0;

            for k in 0..3 {
                if k != p && k != q {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    let next_kp = c * akp - s * akq;
                    let next_kq = s * akp + c * akq;
                    a[k][p] = next_kp;
                    a[p][k] = next_kp;
                    a[k][q] = next_kq;
                    a[q][k] = next_kq;
                }
            }
        }

        let mut values = [
            a[0][0].max(0.0),
            a[1][1].max(0.0),
            a[2][2].max(0.0),
        ];
        values.sort_by(f64::total_cmp);
        values
    }

    pub fn rank(self, relative_tolerance: f64) -> u8 {
        let eigenvalues = self.eigenvalues();
        let maximum = eigenvalues[2];
        if maximum <= 0.0 {
            0
        } else {
            eigenvalues.iter()
                .filter(|value| **value > maximum * relative_tolerance)
                .count() as u8
        }
    }

    pub fn determinant(self) -> f64 {
        let e = self.eigenvalues();
        e[0] * e[1] * e[2]
    }

    pub fn minimum_eigenvalue(self) -> f64 {
        self.eigenvalues()[0]
    }

    pub fn log_determinant(self, relative_tolerance: f64) -> Option<f64> {
        if self.rank(relative_tolerance) < 3 {
            return None;
        }
        let determinant = self.determinant();
        (determinant.is_finite() && determinant > 0.0).then(|| determinant.ln())
    }

    pub fn condition_number(self, relative_tolerance: f64) -> Option<f64> {
        if self.rank(relative_tolerance) < 3 {
            return None;
        }
        let e = self.eigenvalues();
        let value = e[2] / e[0];
        value.is_finite().then_some(value)
    }

    pub fn a_optimal_trace_inverse(self, relative_tolerance: f64) -> Option<f64> {
        if self.rank(relative_tolerance) < 3 {
            return None;
        }
        let e = self.eigenvalues();
        let value = 1.0 / e[0] + 1.0 / e[1] + 1.0 / e[2];
        value.is_finite().then_some(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionError3D {
    TooManyActions(usize),
    EmptyId,
    DuplicateId(String),
    InvalidDirection(String),
    InvalidSigma(String),
    ZeroCost(String),
    InvalidTolerance,
    NonFiniteInformation,
    CostOverflow,
}

impl fmt::Display for SelectionError3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyActions(n) => write!(f, "{n} actions exceed the 3D exhaustive limit of {MAX_ACTIONS_3D}"),
            Self::EmptyId => write!(f, "action ID must not be empty"),
            Self::DuplicateId(id) => write!(f, "duplicate action ID: {id}"),
            Self::InvalidDirection(id) => write!(f, "line-of-sight direction must be finite and unit length: {id}"),
            Self::InvalidSigma(id) => write!(f, "invalid range-noise sigma: {id}"),
            Self::ZeroCost(id) => write!(f, "action cost must be positive: {id}"),
            Self::InvalidTolerance => write!(f, "rank tolerance must be finite and nonnegative"),
            Self::NonFiniteInformation => write!(f, "3D Fisher information became non-finite"),
            Self::CostOverflow => write!(f, "total action cost overflowed u32"),
        }
    }
}
impl std::error::Error for SelectionError3D {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignCriterion3D {
    DOptimal,
    AOptimal,
    EOptimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selection3D {
    pub action_ids: Vec<String>,
    pub total_cost_units: u32,
    pub fisher: Fisher3,
    pub rank: u8,
    pub eigenvalues: [f64; 3],
    pub log_determinant: Option<f64>,
    pub a_optimal_trace_inverse: Option<f64>,
    pub condition_number: Option<f64>,
}

fn canonical_actions(actions: &[RangeAction3D]) -> Result<Vec<RangeAction3D>, SelectionError3D> {
    if actions.len() > MAX_ACTIONS_3D {
        return Err(SelectionError3D::TooManyActions(actions.len()));
    }
    let mut ids = BTreeSet::new();
    let mut ordered = actions.to_vec();
    for action in &ordered {
        if action.id.trim().is_empty() {
            return Err(SelectionError3D::EmptyId);
        }
        if !ids.insert(action.id.clone()) {
            return Err(SelectionError3D::DuplicateId(action.id.clone()));
        }
        let norm = action.line_of_sight.iter().map(|x| x * x).sum::<f64>().sqrt();
        if !action.line_of_sight.iter().all(|x| x.is_finite())
            || !norm.is_finite()
            || (norm - 1.0).abs() > 1.0e-6
        {
            return Err(SelectionError3D::InvalidDirection(action.id.clone()));
        }
        let variance = action.sigma_m * action.sigma_m;
        let precision = 1.0 / variance;
        if !action.sigma_m.is_finite()
            || action.sigma_m <= 0.0
            || !variance.is_finite()
            || variance <= 0.0
            || !precision.is_finite()
            || precision <= 0.0
        {
            return Err(SelectionError3D::InvalidSigma(action.id.clone()));
        }
        if action.cost_units == 0 {
            return Err(SelectionError3D::ZeroCost(action.id.clone()));
        }
    }
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ordered)
}

fn selection_for_mask(
    actions: &[RangeAction3D],
    mask: u64,
    tolerance: f64,
) -> Result<Selection3D, SelectionError3D> {
    let mut fisher = Fisher3::default();
    let mut action_ids = Vec::new();
    let mut total_cost_units = 0_u32;
    for (index, action) in actions.iter().enumerate() {
        if mask & (1_u64 << index) != 0 {
            total_cost_units = total_cost_units
                .checked_add(action.cost_units)
                .ok_or(SelectionError3D::CostOverflow)?;
            fisher.add_range(action.line_of_sight, action.sigma_m);
            action_ids.push(action.id.clone());
        }
    }
    let eigenvalues = fisher.eigenvalues();
    let determinant = fisher.determinant();
    if !fisher.matrix.iter().flatten().all(|x| x.is_finite())
        || !eigenvalues.iter().all(|x| x.is_finite())
        || !determinant.is_finite()
    {
        return Err(SelectionError3D::NonFiniteInformation);
    }
    Ok(Selection3D {
        action_ids,
        total_cost_units,
        fisher,
        rank: fisher.rank(tolerance),
        eigenvalues,
        log_determinant: fisher.log_determinant(tolerance),
        a_optimal_trace_inverse: fisher.a_optimal_trace_inverse(tolerance),
        condition_number: fisher.condition_number(tolerance),
    })
}

fn compare(candidate: &Selection3D, current: &Selection3D, criterion: DesignCriterion3D) -> Ordering {
    match candidate.rank.cmp(&current.rank) {
        Ordering::Equal => {}
        other => return other,
    }
    if candidate.rank < 3 {
        return candidate.fisher.trace().total_cmp(&current.fisher.trace());
    }
    match criterion {
        DesignCriterion3D::DOptimal => candidate.log_determinant.unwrap().total_cmp(&current.log_determinant.unwrap()),
        DesignCriterion3D::AOptimal => current.a_optimal_trace_inverse.unwrap().total_cmp(&candidate.a_optimal_trace_inverse.unwrap()),
        DesignCriterion3D::EOptimal => candidate.eigenvalues[0].total_cmp(&current.eigenvalues[0]),
    }
}

fn prefer(candidate: &Selection3D, current: &Selection3D, criterion: DesignCriterion3D) -> bool {
    match compare(candidate, current, criterion) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match candidate.total_cost_units.cmp(&current.total_cost_units) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => candidate.action_ids < current.action_ids,
        },
    }
}

/// Exact exhaustive 3D subset oracle for at most 16 candidate observations.
pub fn exhaustive_select_3d(
    actions: &[RangeAction3D],
    budget_units: u32,
    criterion: DesignCriterion3D,
) -> Result<Selection3D, SelectionError3D> {
    exhaustive_select_3d_with_tolerance(
        actions,
        budget_units,
        criterion,
        DEFAULT_RANK_TOLERANCE_3D,
    )
}

pub fn exhaustive_select_3d_with_tolerance(
    actions: &[RangeAction3D],
    budget_units: u32,
    criterion: DesignCriterion3D,
    tolerance: f64,
) -> Result<Selection3D, SelectionError3D> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(SelectionError3D::InvalidTolerance);
    }
    let ordered = canonical_actions(actions)?;
    let subset_count = 1_u64 << ordered.len();
    let mut best: Option<Selection3D> = None;
    for mask in 0..subset_count {
        let candidate = selection_for_mask(&ordered, mask, tolerance)?;
        if candidate.total_cost_units > budget_units {
            continue;
        }
        if best.as_ref().is_none_or(|current| prefer(&candidate, current, criterion)) {
            best = Some(candidate);
        }
    }
    Ok(best.expect("empty subset is always feasible"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: &str, direction: [f64; 3]) -> RangeAction3D {
        RangeAction3D::new(id, direction, 1.0, 1)
    }

    fn orthogonal() -> Vec<RangeAction3D> {
        vec![
            action("east", [1.0, 0.0, 0.0]),
            action("north", [0.0, 1.0, 0.0]),
            action("up", [0.0, 0.0, 1.0]),
        ]
    }

    #[test]
    fn one_range_has_rank_one_in_3d() {
        let selected = exhaustive_select_3d(&[action("east", [1.0, 0.0, 0.0])], 1, DesignCriterion3D::DOptimal).unwrap();
        assert_eq!(selected.rank, 1);
        assert_eq!(selected.log_determinant, None);
    }

    #[test]
    fn two_orthogonal_ranges_leave_one_unobservable_axis() {
        let selected = exhaustive_select_3d(
            &[action("east", [1.0, 0.0, 0.0]), action("north", [0.0, 1.0, 0.0])],
            2,
            DesignCriterion3D::EOptimal,
        ).unwrap();
        assert_eq!(selected.rank, 2);
        assert!(selected.eigenvalues[0] <= 1.0e-12);
        assert!(selected.eigenvalues[1] > 0.9);
    }

    #[test]
    fn three_orthogonal_ranges_produce_full_rank_isotropic_information() {
        let selected = exhaustive_select_3d(&orthogonal(), 3, DesignCriterion3D::DOptimal).unwrap();
        assert_eq!(selected.rank, 3);
        assert!((selected.fisher.determinant() - 1.0).abs() < 1.0e-12);
        assert!(selected.eigenvalues.iter().all(|value| (*value - 1.0).abs() < 1.0e-12));
        assert!((selected.condition_number.unwrap() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn planar_directions_have_no_vertical_observability() {
        let planar = vec![
            action("east", [1.0, 0.0, 0.0]),
            action("north", [0.0, 1.0, 0.0]),
            action("diagonal", [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0]),
        ];
        let selected = exhaustive_select_3d(&planar, 3, DesignCriterion3D::DOptimal).unwrap();
        assert_eq!(selected.rank, 2);
        assert!(selected.eigenvalues[0] <= 1.0e-12);
    }

    #[test]
    fn nearly_planar_geometry_is_not_misreported_as_3d_full_rank() {
        let epsilon = 1.0e-8;
        let tilt = (1.0 - epsilon * epsilon).sqrt();
        let near_planar = vec![
            action("east", [1.0, 0.0, 0.0]),
            action("north", [0.0, 1.0, 0.0]),
            action("tilted", [tilt, 0.0, epsilon]),
        ];
        let selected = exhaustive_select_3d(&near_planar, 3, DesignCriterion3D::DOptimal).unwrap();
        assert_eq!(selected.rank, 2);
        assert!(selected.condition_number.is_none());
    }

    #[test]
    fn eigenvalues_and_determinant_are_rotation_invariant() {
        let original = orthogonal();
        let angle = 0.731_f64;
        let (s, c) = angle.sin_cos();
        let rotated = original.iter().map(|item| {
            let [x, y, z] = item.line_of_sight;
            // Orthogonal rotation around the normalized (1,1,1) axis.
            let axis = [1.0 / 3.0_f64.sqrt(); 3];
            let dot = axis[0] * x + axis[1] * y + axis[2] * z;
            let cross = [
                axis[1] * z - axis[2] * y,
                axis[2] * x - axis[0] * z,
                axis[0] * y - axis[1] * x,
            ];
            let direction = [
                x * c + cross[0] * s + axis[0] * dot * (1.0 - c),
                y * c + cross[1] * s + axis[1] * dot * (1.0 - c),
                z * c + cross[2] * s + axis[2] * dot * (1.0 - c),
            ];
            action(&item.id, direction)
        }).collect::<Vec<_>>();
        let a = exhaustive_select_3d(&original, 3, DesignCriterion3D::DOptimal).unwrap();
        let b = exhaustive_select_3d(&rotated, 3, DesignCriterion3D::DOptimal).unwrap();
        for i in 0..3 {
            assert!((a.eigenvalues[i] - b.eigenvalues[i]).abs() < 1.0e-10);
        }
        assert!((a.fisher.determinant() - b.fisher.determinant()).abs() < 1.0e-10);
    }

    #[test]
    fn 3d_subset_oracle_selects_the_vertical_constraint_under_budget() {
        let candidates = vec![
            action("east", [1.0, 0.0, 0.0]),
            action("north", [0.0, 1.0, 0.0]),
            action("north-alt", [0.0, 0.99, 0.1410673598]),
            action("up", [0.0, 0.0, 1.0]),
        ];
        let selected = exhaustive_select_3d(&candidates, 3, DesignCriterion3D::EOptimal).unwrap();
        assert_eq!(selected.rank, 3);
        assert!(selected.action_ids.contains(&"up".to_owned()));
    }

    #[test]
    fn invalid_direction_and_duplicate_ids_fail_closed() {
        assert!(matches!(
            exhaustive_select_3d(&[action("bad", [2.0, 0.0, 0.0])], 1, DesignCriterion3D::DOptimal),
            Err(SelectionError3D::InvalidDirection(_))
        ));
        let duplicate = vec![action("same", [1.0, 0.0, 0.0]), action("same", [0.0, 1.0, 0.0])];
        assert!(matches!(
            exhaustive_select_3d(&duplicate, 2, DesignCriterion3D::DOptimal),
            Err(SelectionError3D::DuplicateId(_))
        ));
    }
}
