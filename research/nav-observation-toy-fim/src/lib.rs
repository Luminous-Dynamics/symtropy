pub mod correlated;
pub mod decision_value;
pub mod robust_decision_value;
pub mod risk_coverage;
pub mod clustered_risk;
pub mod correlation_envelope;
pub mod correlation_audit;
pub mod correlation_planning;
pub mod observability_3d;
pub mod correlated_3d;
pub mod geometry_outcome;
pub mod robust;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

/// Numerical tolerance used only to classify the rank of the 2x2 Fisher
/// information matrix. Raw eigenvalues and conditioning remain visible.
pub const DEFAULT_RANK_TOLERANCE: f64 = 1.0e-10;
const MAX_EXHAUSTIVE_ACTIONS: usize = 20;

/// One candidate scalar range observation, linearized at a fixed 2D position.
///
/// line_of_sight_rad is the line-of-sight direction in a local 2D tangent
/// plane. Range noise is modeled as independent, zero-mean Gaussian noise with
/// standard deviation sigma_m. cost_units is an abstract integer budget.
#[derive(Clone, Debug, PartialEq)]
pub struct RangeAction {
    pub id: String,
    pub line_of_sight_rad: f64,
    pub sigma_m: f64,
    pub cost_units: u32,
}

impl RangeAction {
    pub fn new(
        id: impl Into<String>,
        line_of_sight_rad: f64,
        sigma_m: f64,
        cost_units: u32,
    ) -> Self {
        Self {
            id: id.into(),
            line_of_sight_rad,
            sigma_m,
            cost_units,
        }
    }
}

/// Symmetric 2x2 Fisher information matrix:
///
///     [ xx  xy ]
///     [ xy  yy ]
///
/// For independent scalar range observations with unit line-of-sight vector u_i
/// and variance sigma_i^2, J = sum_i (u_i u_i^T / sigma_i^2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fisher2 {
    pub xx: f64,
    pub xy: f64,
    pub yy: f64,
}

impl Fisher2 {
    pub fn add_range(&mut self, line_of_sight_rad: f64, sigma_m: f64) {
        let c = line_of_sight_rad.cos();
        let s = line_of_sight_rad.sin();
        let precision = 1.0 / (sigma_m * sigma_m);

        self.xx += precision * c * c;
        self.xy += precision * c * s;
        self.yy += precision * s * s;
    }

    pub fn trace(self) -> f64 {
        self.xx + self.yy
    }

    pub fn determinant(self) -> f64 {
        // FMA reduces cancellation for nearly rank-deficient 2x2 matrices.
        self.xx.mul_add(self.yy, -(self.xy * self.xy))
    }

    pub fn eigenvalues(self) -> (f64, f64) {
        let trace = self.trace();
        let discriminant = (self.xx - self.yy).hypot(2.0 * self.xy);
        let maximum = 0.5 * (trace + discriminant);
        // Computing (trace - discriminant) / 2 directly loses significant
        // digits when the minimum eigenvalue is much smaller than the maximum.
        // For a PSD 2x2 matrix, lambda_min = det(J) / lambda_max is better
        // conditioned. Tiny negative roundoff in det is clamped to zero.
        let minimum = if maximum > 0.0 {
            (self.determinant() / maximum).max(0.0)
        } else {
            0.0
        };
        (minimum, maximum)
    }

    pub fn rank(self, relative_tolerance: f64) -> u8 {
        let (minimum, maximum) = self.eigenvalues();
        if maximum <= 0.0 {
            0
        } else if minimum <= maximum * relative_tolerance {
            1
        } else {
            2
        }
    }

    /// D-optimal log(det(J)); undefined for rank-deficient information.
    pub fn log_determinant(self, relative_tolerance: f64) -> Option<f64> {
        if self.rank(relative_tolerance) < 2 {
            return None;
        }
        let determinant = self.determinant();
        (determinant.is_finite() && determinant > 0.0).then(|| determinant.ln())
    }

    pub fn condition_number(self, relative_tolerance: f64) -> Option<f64> {
        if self.rank(relative_tolerance) < 2 {
            return None;
        }
        let (minimum, maximum) = self.eigenvalues();
        let value = maximum / minimum;
        value.is_finite().then_some(value)
    }

    /// A-optimal criterion: trace(J^-1), defined only for full-rank geometry.
    ///
    /// Smaller values indicate less total marginal variance in the local
    /// linearized 2D model. This is not a calibrated posterior covariance.
    pub fn a_optimal_trace_inverse(self, relative_tolerance: f64) -> Option<f64> {
        if self.rank(relative_tolerance) < 2 {
            return None;
        }
        let determinant = self.determinant();
        if !determinant.is_finite() || determinant <= 0.0 {
            return None;
        }
        let value = self.trace() / determinant;
        value.is_finite().then_some(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionError {
    TooManyActions(usize),
    EmptyActionId,
    DuplicateActionId(String),
    NonFiniteBearing(String),
    InvalidSigma(String),
    ZeroCost(String),
    InvalidRankTolerance,
    NonFiniteInformation,
    CostOverflow,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyActions(count) => write!(
                f,
                "{count} actions exceed the exhaustive-oracle limit of {MAX_EXHAUSTIVE_ACTIONS}"
            ),
            Self::EmptyActionId => write!(f, "action ID must not be empty"),
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::NonFiniteBearing(id) => write!(f, "non-finite line-of-sight bearing: {id}"),
            Self::InvalidSigma(id) => write!(f, "range noise variance/precision is invalid: {id}"),
            Self::ZeroCost(id) => write!(f, "action cost must be positive: {id}"),
            Self::InvalidRankTolerance => write!(f, "rank tolerance must be finite and nonnegative"),
            Self::NonFiniteInformation => write!(f, "Fisher information overflowed or became non-finite"),
            Self::CostOverflow => write!(f, "total action cost overflowed u32"),
        }
    }
}

impl std::error::Error for SelectionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignCriterion {
    /// Maximize the determinant of Fisher information (D-optimality).
    DOptimal,
    /// Minimize trace(J^-1), the sum of marginal variances (A-optimality).
    AOptimal,
    /// Maximize the weakest information eigenvalue (E-optimality).
    EOptimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub action_ids: Vec<String>,
    pub total_cost_units: u32,
    pub fisher: Fisher2,
    pub rank: u8,
    pub minimum_eigenvalue: f64,
    pub log_determinant: Option<f64>,
    pub a_optimal_trace_inverse: Option<f64>,
    pub condition_number: Option<f64>,
}

fn canonical_actions(actions: &[RangeAction]) -> Result<Vec<RangeAction>, SelectionError> {
    if actions.len() > MAX_EXHAUSTIVE_ACTIONS {
        return Err(SelectionError::TooManyActions(actions.len()));
    }

    let mut ids = BTreeSet::new();
    let mut ordered = actions.to_vec();

    for action in &ordered {
        if action.id.trim().is_empty() {
            return Err(SelectionError::EmptyActionId);
        }
        if !ids.insert(action.id.clone()) {
            return Err(SelectionError::DuplicateActionId(action.id.clone()));
        }
        if !action.line_of_sight_rad.is_finite() {
            return Err(SelectionError::NonFiniteBearing(action.id.clone()));
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
            return Err(SelectionError::InvalidSigma(action.id.clone()));
        }
        if action.cost_units == 0 {
            return Err(SelectionError::ZeroCost(action.id.clone()));
        }
    }

    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ordered)
}

fn selection_for_mask(
    actions: &[RangeAction],
    mask: u64,
    rank_tolerance: f64,
) -> Result<Selection, SelectionError> {
    let mut fisher = Fisher2::default();
    let mut action_ids = Vec::new();
    let mut total_cost_units = 0_u32;

    for (index, action) in actions.iter().enumerate() {
        if mask & (1_u64 << index) != 0 {
            total_cost_units = total_cost_units
                .checked_add(action.cost_units)
                .ok_or(SelectionError::CostOverflow)?;
            fisher.add_range(action.line_of_sight_rad, action.sigma_m);
            action_ids.push(action.id.clone());
        }
    }

    let determinant = fisher.determinant();
    let trace = fisher.trace();
    if !fisher.xx.is_finite()
        || !fisher.xy.is_finite()
        || !fisher.yy.is_finite()
        || !trace.is_finite()
        || !determinant.is_finite()
    {
        return Err(SelectionError::NonFiniteInformation);
    }

    let (minimum_eigenvalue, _) = fisher.eigenvalues();
    if !minimum_eigenvalue.is_finite() {
        return Err(SelectionError::NonFiniteInformation);
    }

    Ok(Selection {
        action_ids,
        total_cost_units,
        fisher,
        rank: fisher.rank(rank_tolerance),
        minimum_eigenvalue,
        log_determinant: fisher.log_determinant(rank_tolerance),
        a_optimal_trace_inverse: fisher.a_optimal_trace_inverse(rank_tolerance),
        condition_number: fisher.condition_number(rank_tolerance),
    })
}

/// Compare information quality without folding physical/resource cost into a
/// made-up scalar. Prefer rank, then D-optimality for full-rank cases, and
/// observable-direction information (trace) for rank-deficient cases.
fn compare_information(
    a: &Selection,
    b: &Selection,
    criterion: DesignCriterion,
) -> Ordering {
    match a.rank.cmp(&b.rank) {
        Ordering::Equal => {}
        other => return other,
    }

    if a.rank < 2 {
        // All three criteria are undefined or degenerate for rank-deficient
        // 2D geometry; compare only information in the observable subspace.
        return a.fisher.trace().total_cmp(&b.fisher.trace());
    }

    match criterion {
        DesignCriterion::DOptimal => a
            .log_determinant
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&b.log_determinant.unwrap_or(f64::NEG_INFINITY)),
        DesignCriterion::AOptimal => b
            .a_optimal_trace_inverse
            .unwrap_or(f64::INFINITY)
            .total_cmp(&a.a_optimal_trace_inverse.unwrap_or(f64::INFINITY)),
        DesignCriterion::EOptimal => a
            .minimum_eigenvalue
            .total_cmp(&b.minimum_eigenvalue),
    }
}

fn prefer_plan(a: &Selection, b: &Selection, criterion: DesignCriterion) -> bool {
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

/// Return the globally best subset for this small finite candidate set.
///
/// Lexicographic objective: maximize observable rank; then maximize log(det(J))
/// for full-rank geometry (or trace(J) if rank-deficient); then minimize cost;
/// finally choose the lexicographically smallest action IDs.
pub fn exhaustive_select(
    actions: &[RangeAction],
    budget_units: u32,
) -> Result<Selection, SelectionError> {
    exhaustive_select_with_tolerance(actions, budget_units, DEFAULT_RANK_TOLERANCE)
}

pub fn exhaustive_select_with_tolerance(
    actions: &[RangeAction],
    budget_units: u32,
    rank_tolerance: f64,
) -> Result<Selection, SelectionError> {
    exhaustive_select_with_tolerance_and_criterion(
        actions,
        budget_units,
        rank_tolerance,
        DesignCriterion::DOptimal,
    )
}

/// Exhaustively optimize one explicit design criterion under a hard budget.
pub fn exhaustive_select_with_criterion(
    actions: &[RangeAction],
    budget_units: u32,
    criterion: DesignCriterion,
) -> Result<Selection, SelectionError> {
    exhaustive_select_with_tolerance_and_criterion(
        actions,
        budget_units,
        DEFAULT_RANK_TOLERANCE,
        criterion,
    )
}

pub fn exhaustive_select_with_tolerance_and_criterion(
    actions: &[RangeAction],
    budget_units: u32,
    rank_tolerance: f64,
    criterion: DesignCriterion,
) -> Result<Selection, SelectionError> {
    if !rank_tolerance.is_finite() || rank_tolerance < 0.0 {
        return Err(SelectionError::InvalidRankTolerance);
    }

    let ordered = canonical_actions(actions)?;
    let subset_count = 1_u64 << ordered.len();
    let mut best: Option<Selection> = None;

    for mask in 0..subset_count {
        let candidate = selection_for_mask(&ordered, mask, rank_tolerance)?;
        if candidate.total_cost_units > budget_units {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|current| prefer_plan(&candidate, current, criterion))
        {
            best = Some(candidate);
        }
    }

    // The empty set is always feasible, including for an empty action list.
    Ok(best.expect("empty subset is always feasible"))
}

/// Deterministic myopic baseline: add the feasible action that most improves
/// the same information objective at each step. This is not an exact oracle.
pub fn greedy_select(
    actions: &[RangeAction],
    budget_units: u32,
) -> Result<Selection, SelectionError> {
    greedy_select_with_criterion(actions, budget_units, DesignCriterion::DOptimal)
}

/// Deterministic greedy baseline for an explicitly selected design criterion.
pub fn greedy_select_with_criterion(
    actions: &[RangeAction],
    budget_units: u32,
    criterion: DesignCriterion,
) -> Result<Selection, SelectionError> {
    let ordered = canonical_actions(actions)?;
    let mut current = selection_for_mask(&ordered, 0, DEFAULT_RANK_TOLERANCE)?;

    loop {
        let mut best_next: Option<Selection> = None;
        let current_mask = ordered
            .iter()
            .enumerate()
            .filter(|(_, action)| current.action_ids.binary_search(&action.id).is_ok())
            .fold(0_u64, |mask, (i, _)| mask | (1_u64 << i));

        for index in 0..ordered.len() {
            if current_mask & (1_u64 << index) != 0 {
                continue;
            }

            let next_cost = current
                .total_cost_units
                .checked_add(ordered[index].cost_units)
                .ok_or(SelectionError::CostOverflow)?;
            if next_cost > budget_units {
                continue;
            }

            let candidate = selection_for_mask(
                &ordered,
                current_mask | (1_u64 << index),
                DEFAULT_RANK_TOLERANCE,
            )?;

            if compare_information(&candidate, &current, criterion) != Ordering::Greater {
                continue;
            }

            if best_next
                .as_ref()
                .is_none_or(|best| prefer_plan(&candidate, best, criterion))
            {
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

    fn action(id: &str, degrees: f64, cost: u32) -> RangeAction {
        RangeAction::new(id, degrees.to_radians(), 1.0, cost)
    }

    fn candidate_set() -> Vec<RangeAction> {
        vec![
            action("a-east", 0.0, 1),
            action("b-east-near", 10.0, 1),
            action("c-north-costly", 90.0, 2),
            action("d-diagonal-45", 45.0, 1),
            action("e-antidiagonal-135", 135.0, 1),
            action("f-west", 180.0, 1),
        ]
    }

    #[test]
    fn one_range_observation_has_rank_one() {
        let mut matrix = Fisher2::default();
        matrix.add_range(0.0, 1.0);
        assert_eq!(matrix.rank(DEFAULT_RANK_TOLERANCE), 1);
        assert_eq!(matrix.log_determinant(DEFAULT_RANK_TOLERANCE), None);
        assert_eq!(matrix.condition_number(DEFAULT_RANK_TOLERANCE), None);
    }

    #[test]
    fn orthogonal_ranges_produce_isotropic_full_rank_information() {
        let mut matrix = Fisher2::default();
        matrix.add_range(0.0, 1.0);
        matrix.add_range(std::f64::consts::FRAC_PI_2, 1.0);

        assert_eq!(matrix.rank(DEFAULT_RANK_TOLERANCE), 2);
        assert!((matrix.determinant() - 1.0).abs() < 1.0e-12);
        assert!((matrix.condition_number(DEFAULT_RANK_TOLERANCE).unwrap() - 1.0).abs() < 1.0e-12);
        assert!((matrix.a_optimal_trace_inverse(DEFAULT_RANK_TOLERANCE).unwrap() - 2.0).abs() < 1.0e-12);
        assert!((matrix.eigenvalues().0 - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn collinear_ranges_do_not_invent_cross_track_observability() {
        let mut matrix = Fisher2::default();
        matrix.add_range(0.0, 1.0);
        matrix.add_range(std::f64::consts::PI, 1.0);

        assert_eq!(matrix.rank(DEFAULT_RANK_TOLERANCE), 1);
        assert_eq!(matrix.log_determinant(DEFAULT_RANK_TOLERANCE), None);
    }

    #[test]
    fn exhaustive_oracle_finds_budget_feasible_orthogonal_pair() {
        let selected = exhaustive_select(&candidate_set(), 2).unwrap();

        assert_eq!(
            selected.action_ids,
            vec![
                "d-diagonal-45".to_owned(),
                "e-antidiagonal-135".to_owned()
            ]
        );
        assert_eq!(selected.total_cost_units, 2);
        assert_eq!(selected.rank, 2);
        assert!((selected.fisher.determinant() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn greedy_is_a_baseline_not_the_exhaustive_oracle() {
        let actions = candidate_set();
        let exact = exhaustive_select(&actions, 2).unwrap();
        let greedy = greedy_select(&actions, 2).unwrap();

        assert!(greedy.total_cost_units <= 2);
        assert!((exact.fisher.determinant() - 1.0).abs() < 1.0e-12);
        assert!((greedy.fisher.determinant() - 0.5).abs() < 1.0e-12);
        assert!(greedy.fisher.determinant() < exact.fisher.determinant());
    }

    #[test]
    fn result_is_stable_under_candidate_input_permutation() {
        let actions = candidate_set();
        let forward = exhaustive_select(&actions, 2).unwrap();
        let mut reversed = actions;
        reversed.reverse();
        let backward = exhaustive_select(&reversed, 2).unwrap();

        assert_eq!(forward.action_ids, backward.action_ids);
        assert_eq!(forward.fisher, backward.fisher);
    }


    #[test]
    fn common_rotation_preserves_rank_and_determinant() {
        let original = candidate_set();
        let rotated = original
            .iter()
            .map(|item| {
                RangeAction::new(
                    item.id.clone(),
                    item.line_of_sight_rad + 0.731,
                    item.sigma_m,
                    item.cost_units,
                )
            })
            .collect::<Vec<_>>();

        let before = exhaustive_select(&original, 2).unwrap();
        let after = exhaustive_select(&rotated, 2).unwrap();

        assert_eq!(before.rank, after.rank);
        assert!((before.fisher.determinant() - after.fisher.determinant()).abs() < 1.0e-12);
        assert!((before.fisher.trace() - after.fisher.trace()).abs() < 1.0e-12);
    }

    #[test]
    fn uniform_noise_scale_changes_information_by_inverse_square() {
        let original = vec![
            RangeAction::new("east", 0.0, 1.0, 1),
            RangeAction::new("north", std::f64::consts::FRAC_PI_2, 1.0, 1),
        ];
        let scaled = original
            .iter()
            .map(|item| {
                RangeAction::new(
                    item.id.clone(),
                    item.line_of_sight_rad,
                    item.sigma_m * 2.0,
                    item.cost_units,
                )
            })
            .collect::<Vec<_>>();

        let before = exhaustive_select(&original, 2).unwrap();
        let after = exhaustive_select(&scaled, 2).unwrap();

        assert!((after.fisher.trace() - before.fisher.trace() / 4.0).abs() < 1.0e-12);
        assert!((after.fisher.determinant() - before.fisher.determinant() / 16.0).abs() < 1.0e-12);
        assert_eq!(after.rank, 2);
    }

    #[test]
    fn near_collinear_geometry_is_not_misreported_as_well_conditioned() {
        let mut matrix = Fisher2::default();
        matrix.add_range(0.0, 1.0);
        matrix.add_range(1.0e-8, 1.0);

        assert_eq!(matrix.rank(DEFAULT_RANK_TOLERANCE), 1);
        assert_eq!(matrix.condition_number(DEFAULT_RANK_TOLERANCE), None);
        assert_eq!(matrix.a_optimal_trace_inverse(DEFAULT_RANK_TOLERANCE), None);
        assert!(matrix.eigenvalues().0 < 1.0e-14);
    }

    #[test]
    fn precision_overflow_is_rejected_before_scoring() {
        let extreme = vec![
            RangeAction::new("east", 0.0, 1.0e-160, 1),
            RangeAction::new("north", std::f64::consts::FRAC_PI_2, 1.0e-160, 1),
        ];
        assert_eq!(
            exhaustive_select(&extreme, 2),
            Err(SelectionError::InvalidSigma("east".to_owned()))
        );
    }


    #[test]
    fn design_criteria_can_select_different_geometries() {
        let actions = vec![
            RangeAction::new("a-east-precise", 0.0, 0.1, 1),
            RangeAction::new("b-north-weak", std::f64::consts::FRAC_PI_2, 1.0, 1),
            RangeAction::new("c-diagonal", std::f64::consts::FRAC_PI_4, 1.0 / 10.0_f64.sqrt(), 1),
            RangeAction::new("d-antidiagonal", 3.0 * std::f64::consts::FRAC_PI_4, 1.0 / 9.0_f64.sqrt(), 1),
        ];

        let d_opt = exhaustive_select_with_criterion(&actions, 2, DesignCriterion::DOptimal).unwrap();
        let a_opt = exhaustive_select_with_criterion(&actions, 2, DesignCriterion::AOptimal).unwrap();
        let e_opt = exhaustive_select_with_criterion(&actions, 2, DesignCriterion::EOptimal).unwrap();

        assert_eq!(d_opt.rank, 2);
        assert_eq!(a_opt.rank, 2);
        assert_eq!(e_opt.rank, 2);
        assert!(d_opt.fisher.determinant() > a_opt.fisher.determinant());
        assert!(a_opt.a_optimal_trace_inverse.unwrap() < d_opt.a_optimal_trace_inverse.unwrap());
        assert!(e_opt.minimum_eigenvalue > d_opt.minimum_eigenvalue);
        assert_eq!(
            a_opt.action_ids,
            vec!["c-diagonal".to_owned(), "d-antidiagonal".to_owned()]
        );
        assert_eq!(a_opt.action_ids, e_opt.action_ids);
    }

    #[test]
    fn criterion_specific_greedy_respects_the_same_budget() {
        for criterion in [
            DesignCriterion::DOptimal,
            DesignCriterion::AOptimal,
            DesignCriterion::EOptimal,
        ] {
            let selected = greedy_select_with_criterion(&candidate_set(), 2, criterion).unwrap();
            assert!(selected.total_cost_units <= 2);
        }
    }

    #[test]
    fn duplicate_ids_and_invalid_noise_are_rejected() {
        let duplicate = vec![action("same", 0.0, 1), action("same", 90.0, 1)];
        assert_eq!(
            exhaustive_select(&duplicate, 2),
            Err(SelectionError::DuplicateActionId("same".to_owned()))
        );

        let invalid_sigma = vec![RangeAction::new("bad", 0.0, 0.0, 1)];
        assert_eq!(
            exhaustive_select(&invalid_sigma, 1),
            Err(SelectionError::InvalidSigma("bad".to_owned()))
        );
    }
}
