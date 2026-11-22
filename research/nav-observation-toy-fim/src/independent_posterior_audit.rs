//! Estimator-independent geometric consistency audit for 3D range-set posteriors.
//!
//! This module checks whether a candidate 3D position and its declared
//! uncertainty radius are geometrically compatible with independently
//! qualified range intervals and anchor-position bounds. It also measures
//! whether distinct dependency groups provide geometrically diverse
//! directions.
//!
//! It does not estimate the position, trust a front-end covariance, prove the
//! candidate is true, calibrate range intervals, or establish independence
//! between failure domains. "Pass" means only "no declared geometric
//! contradiction and sufficient directional support under this input
//! contract." The caller must qualify the range sets, anchor bounds, dependency
//! labels and candidate-position error separately.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

const DIM: usize = 3;
const JACOBI_MAX_SWEEPS: usize = 32;
const JACOBI_OFF_DIAGONAL_TOLERANCE: f64 = 1.0e-14;

/// A range observation whose interval and anchor uncertainty have already
/// passed their own measurement/provenance qualification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeSetConstraint3d {
    pub anchor_id: u32,
    /// Externally assigned common-cause / dependency group.
    ///
    /// Observations sharing this ID count as one geometric evidence group.
    /// This does not prove observations in different groups are independent.
    pub dependency_group_id: u64,
    pub anchor_position_m: [f64; DIM],
    pub anchor_position_abs_error_m: f64,
    pub observed_range_lower_m: f64,
    pub observed_range_upper_m: f64,
}

/// Rules for rejecting a position that is unsupported or contradicted by the
/// qualified range sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndependentPosteriorAuditConfig {
    pub minimum_observation_count: usize,
    pub minimum_dependency_group_count: usize,
    /// Minimum eigenvalue of the dependency-group-balanced direction matrix.
    /// Its trace is one, so values lie in [0, 1/3] up to numerical tolerance.
    pub minimum_geometry_eigenvalue: f64,
    /// Maximum allowed gap between predicted and observed range sets.
    pub maximum_range_conflict_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeConstraintAudit3d {
    pub anchor_id: u32,
    pub dependency_group_id: u64,
    pub predicted_range_lower_m: f64,
    pub predicted_range_upper_m: f64,
    pub observed_range_lower_m: f64,
    pub observed_range_upper_m: f64,
    /// Zero means the predicted and observed range sets intersect.
    pub range_set_conflict_m: f64,
    pub compatible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IndependentPosteriorAudit3d {
    pub observation_count: usize,
    pub dependency_group_count: usize,
    pub compatible_observation_count: usize,
    pub conflicting_observation_count: usize,
    pub maximum_range_conflict_m: f64,
    /// Eigenvalues of the normalized 3D direction matrix, ascending.
    pub geometry_eigenvalues: [f64; DIM],
    pub minimum_geometry_eigenvalue: f64,
    pub maximum_geometry_eigenvalue: f64,
    pub range_constraints: Vec<RangeConstraintAudit3d>,
    pub all_range_sets_compatible: bool,
    pub geometry_sufficient: bool,
    /// True only when count, dependency-group, consistency and geometry
    /// conditions all satisfy the caller-provided configuration.
    pub audit_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndependentPosteriorAuditError {
    EmptyConstraints,
    InsufficientConfiguredObservations,
    InsufficientConfiguredDependencyGroups,
    InsufficientObservationCount,
    InsufficientDependencyGroupCount,
    InvalidConfiguration,
    InvalidCandidatePosition,
    InvalidCandidatePositionBound,
    InvalidAnchorIdentity,
    DuplicateAnchorIdentity(u32),
    InvalidDependencyGroupIdentity,
    InvalidAnchorPosition(u32),
    InvalidAnchorPositionBound(u32),
    InvalidObservedRangeInterval(u32),
    CoincidentCandidateAndAnchor(u32),
    ArithmeticOverflow,
    EigenvalueFailure,
}

impl fmt::Display for IndependentPosteriorAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConstraints => write!(f, "at least one range constraint is required"),
            Self::InsufficientConfiguredObservations => {
                write!(f, "minimum observation count must be at least four")
            }
            Self::InsufficientConfiguredDependencyGroups => {
                write!(f, "minimum dependency-group count must be at least four")
            }
            Self::InsufficientObservationCount => {
                write!(f, "candidate posterior has fewer observations than the configured minimum")
            }
            Self::InsufficientDependencyGroupCount => {
                write!(f, "candidate posterior has fewer dependency groups than the configured minimum")
            }
            Self::InvalidConfiguration => {
                write!(f, "posterior audit thresholds must be finite and physically meaningful")
            }
            Self::InvalidCandidatePosition => {
                write!(f, "candidate position coordinates must be finite")
            }
            Self::InvalidCandidatePositionBound => {
                write!(f, "candidate position absolute error bound must be finite and nonnegative")
            }
            Self::InvalidAnchorIdentity => write!(f, "anchor identity must be nonzero"),
            Self::DuplicateAnchorIdentity(id) => write!(f, "duplicate anchor identity: {id}"),
            Self::InvalidDependencyGroupIdentity => {
                write!(f, "dependency group identity must be nonzero")
            }
            Self::InvalidAnchorPosition(id) => {
                write!(f, "anchor position coordinates must be finite: {id}")
            }
            Self::InvalidAnchorPositionBound(id) => {
                write!(f, "anchor position error bound must be finite and nonnegative: {id}")
            }
            Self::InvalidObservedRangeInterval(id) => {
                write!(f, "observed range interval is invalid: {id}")
            }
            Self::CoincidentCandidateAndAnchor(id) => {
                write!(f, "candidate position coincides with anchor; direction is undefined: {id}")
            }
            Self::ArithmeticOverflow => write!(f, "posterior audit arithmetic overflowed"),
            Self::EigenvalueFailure => write!(f, "3D geometry eigenvalue calculation failed"),
        }
    }
}

impl std::error::Error for IndependentPosteriorAuditError {}

/// Audit one front-end candidate against independent range-set and anchor
/// evidence. No front-end covariance or estimator health flag is accepted.
pub fn audit_independent_posterior_3d(
    candidate_position_m: [f64; DIM],
    candidate_position_abs_error_m: f64,
    constraints: &[RangeSetConstraint3d],
    config: IndependentPosteriorAuditConfig,
) -> Result<IndependentPosteriorAudit3d, IndependentPosteriorAuditError> {
    validate_config(config)?;
    if constraints.is_empty() {
        return Err(IndependentPosteriorAuditError::EmptyConstraints);
    }
    if !candidate_position_m.iter().all(|x| x.is_finite()) {
        return Err(IndependentPosteriorAuditError::InvalidCandidatePosition);
    }
    if !candidate_position_abs_error_m.is_finite() || candidate_position_abs_error_m < 0.0 {
        return Err(IndependentPosteriorAuditError::InvalidCandidatePositionBound);
    }

    let mut anchor_ids = HashSet::with_capacity(constraints.len());
    let mut group_matrices: BTreeMap<u64, ([[f64; DIM]; DIM], usize)> = BTreeMap::new();
    let mut range_audits = Vec::with_capacity(constraints.len());
    let mut maximum_conflict = 0.0_f64;
    let mut compatible_count = 0_usize;

    for constraint in constraints {
        if constraint.anchor_id == 0 {
            return Err(IndependentPosteriorAuditError::InvalidAnchorIdentity);
        }
        if !anchor_ids.insert(constraint.anchor_id) {
            return Err(IndependentPosteriorAuditError::DuplicateAnchorIdentity(
                constraint.anchor_id,
            ));
        }
        if constraint.dependency_group_id == 0 {
            return Err(IndependentPosteriorAuditError::InvalidDependencyGroupIdentity);
        }
        if !constraint.anchor_position_m.iter().all(|x| x.is_finite()) {
            return Err(IndependentPosteriorAuditError::InvalidAnchorPosition(
                constraint.anchor_id,
            ));
        }
        if !constraint.anchor_position_abs_error_m.is_finite()
            || constraint.anchor_position_abs_error_m < 0.0
        {
            return Err(IndependentPosteriorAuditError::InvalidAnchorPositionBound(
                constraint.anchor_id,
            ));
        }
        if !constraint.observed_range_lower_m.is_finite()
            || !constraint.observed_range_upper_m.is_finite()
            || constraint.observed_range_lower_m < 0.0
            || constraint.observed_range_upper_m < constraint.observed_range_lower_m
        {
            return Err(IndependentPosteriorAuditError::InvalidObservedRangeInterval(
                constraint.anchor_id,
            ));
        }

        let delta = [
            constraint.anchor_position_m[0] - candidate_position_m[0],
            constraint.anchor_position_m[1] - candidate_position_m[1],
            constraint.anchor_position_m[2] - candidate_position_m[2],
        ];
        let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if !distance.is_finite() {
            return Err(IndependentPosteriorAuditError::ArithmeticOverflow);
        }
        if distance <= f64::EPSILON {
            return Err(IndependentPosteriorAuditError::CoincidentCandidateAndAnchor(
                constraint.anchor_id,
            ));
        }

        let total_position_error =
            candidate_position_abs_error_m + constraint.anchor_position_abs_error_m;
        if !total_position_error.is_finite() {
            return Err(IndependentPosteriorAuditError::ArithmeticOverflow);
        }
        let predicted_lower = (distance - total_position_error).max(0.0);
        let predicted_upper = distance + total_position_error;
        if !predicted_lower.is_finite() || !predicted_upper.is_finite() {
            return Err(IndependentPosteriorAuditError::ArithmeticOverflow);
        }

        // Distance between two closed intervals; zero iff they intersect.
        let conflict = if predicted_upper < constraint.observed_range_lower_m {
            constraint.observed_range_lower_m - predicted_upper
        } else if constraint.observed_range_upper_m < predicted_lower {
            predicted_lower - constraint.observed_range_upper_m
        } else {
            0.0
        };
        if !conflict.is_finite() {
            return Err(IndependentPosteriorAuditError::ArithmeticOverflow);
        }
        let compatible = conflict <= config.maximum_range_conflict_m;
        if compatible {
            compatible_count += 1;
        }
        maximum_conflict = maximum_conflict.max(conflict);

        range_audits.push(RangeConstraintAudit3d {
            anchor_id: constraint.anchor_id,
            dependency_group_id: constraint.dependency_group_id,
            predicted_range_lower_m: predicted_lower,
            predicted_range_upper_m: predicted_upper,
            observed_range_lower_m: constraint.observed_range_lower_m,
            observed_range_upper_m: constraint.observed_range_upper_m,
            range_set_conflict_m: conflict,
            compatible,
        });

        let direction = [delta[0] / distance, delta[1] / distance, delta[2] / distance];
        let matrix = group_matrices
            .entry(constraint.dependency_group_id)
            .or_insert(([[0.0; DIM]; DIM], 0));
        for row in 0..DIM {
            for column in 0..DIM {
                matrix.0[row][column] += direction[row] * direction[column];
            }
        }
        matrix.1 += 1;
    }

    let dependency_group_count = group_matrices.len();
    if constraints.len() < config.minimum_observation_count {
        return Err(IndependentPosteriorAuditError::InsufficientObservationCount);
    }
    if dependency_group_count < config.minimum_dependency_group_count {
        return Err(IndependentPosteriorAuditError::InsufficientDependencyGroupCount);
    }

    // Each dependency group contributes one normalized average direction
    // matrix, regardless of how many rows it contains. This avoids inflating
    // geometric support by duplicating observations from one declared cause.
    let mut geometry_matrix = [[0.0_f64; DIM]; DIM];
    for (group_matrix, count) in group_matrices.values() {
        if *count == 0 {
            return Err(IndependentPosteriorAuditError::ArithmeticOverflow);
        }
        for row in 0..DIM {
            for column in 0..DIM {
                geometry_matrix[row][column] += group_matrix[row][column] / (*count as f64);
            }
        }
    }
    let group_scale = dependency_group_count as f64;
    for row in &mut geometry_matrix {
        for value in row {
            *value /= group_scale;
        }
    }

    let eigenvalues = symmetric_eigenvalues_3x3(geometry_matrix)?;
    let minimum_geometry_eigenvalue = eigenvalues[0].max(0.0);
    let maximum_geometry_eigenvalue = eigenvalues[2].max(0.0);
    let all_range_sets_compatible = compatible_count == constraints.len();
    let geometry_sufficient =
        dependency_group_count >= config.minimum_dependency_group_count
            && minimum_geometry_eigenvalue >= config.minimum_geometry_eigenvalue;
    let audit_passed = all_range_sets_compatible && geometry_sufficient;

    Ok(IndependentPosteriorAudit3d {
        observation_count: constraints.len(),
        dependency_group_count,
        compatible_observation_count: compatible_count,
        conflicting_observation_count: constraints.len() - compatible_count,
        maximum_range_conflict_m: maximum_conflict,
        geometry_eigenvalues: eigenvalues,
        minimum_geometry_eigenvalue,
        maximum_geometry_eigenvalue,
        range_constraints: range_audits,
        all_range_sets_compatible,
        geometry_sufficient,
        audit_passed,
    })
}

fn validate_config(
    config: IndependentPosteriorAuditConfig,
) -> Result<(), IndependentPosteriorAuditError> {
    if config.minimum_observation_count < 4 {
        return Err(IndependentPosteriorAuditError::InsufficientConfiguredObservations);
    }
    if config.minimum_dependency_group_count < 4 {
        return Err(IndependentPosteriorAuditError::InsufficientConfiguredDependencyGroups);
    }
    if !config.minimum_geometry_eigenvalue.is_finite()
        || config.minimum_geometry_eigenvalue <= 0.0
        || config.minimum_geometry_eigenvalue > 1.0 / 3.0
        || !config.maximum_range_conflict_m.is_finite()
        || config.maximum_range_conflict_m < 0.0

    {
        return Err(IndependentPosteriorAuditError::InvalidConfiguration);
    }
    Ok(())
}

/// Eigenvalues of a real symmetric 3x3 matrix, returned in ascending order.
///
/// Uses bounded Jacobi rotations. It is used as a geometric observability
/// diagnostic only; no statistical Fisher-information interpretation is
/// claimed because dependency groups and input uncertainty are externally
/// declared, not learned from an independent stochastic model.
fn symmetric_eigenvalues_3x3(
    mut matrix: [[f64; DIM]; DIM],
) -> Result<[f64; DIM], IndependentPosteriorAuditError> {
    if !matrix.iter().flatten().all(|x| x.is_finite()) {
        return Err(IndependentPosteriorAuditError::EigenvalueFailure);
    }
    for _ in 0..JACOBI_MAX_SWEEPS {
        let mut p = 0;
        let mut q = 1;
        let mut maximum = matrix[0][1].abs();
        for (row, column) in [(0, 2), (1, 2)] {
            let candidate = matrix[row][column].abs();
            if candidate > maximum {
                maximum = candidate;
                p = row;
                q = column;
            }
        }
        if maximum <= JACOBI_OFF_DIAGONAL_TOLERANCE {
            break;
        }

        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
        let tangent = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;

        for index in [0, 1, 2] {
            if index != p && index != q {
                let aip = matrix[index][p];
                let aiq = matrix[index][q];
                let new_ip = cosine * aip - sine * aiq;
                let new_iq = sine * aip + cosine * aiq;
                matrix[index][p] = new_ip;
                matrix[p][index] = new_ip;
                matrix[index][q] = new_iq;
                matrix[q][index] = new_iq;
            }
        }
        matrix[p][p] = app - tangent * apq;
        matrix[q][q] = aqq + tangent * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
    }

    let mut eigenvalues = [matrix[0][0], matrix[1][1], matrix[2][2]];
    if !eigenvalues.iter().all(|x| x.is_finite()) {
        return Err(IndependentPosteriorAuditError::EigenvalueFailure);
    }
    eigenvalues.sort_by(f64::total_cmp);
    Ok(eigenvalues)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> IndependentPosteriorAuditConfig {
        IndependentPosteriorAuditConfig {
            minimum_observation_count: 4,
            minimum_dependency_group_count: 4,
            minimum_geometry_eigenvalue: 0.05,
            maximum_range_conflict_m: 0.01,
        }
    }

    fn constraints_at_origin() -> Vec<RangeSetConstraint3d> {
        let anchors: [(u32, u64, [f64; 3]); 4] = [
            (1, 1, [10.0, 0.0, 0.0]),
            (2, 2, [0.0, 10.0, 0.0]),
            (3, 3, [0.0, 0.0, 10.0]),
            (4, 4, [-10.0, -10.0, -10.0]),
        ];
        anchors
            .into_iter()
            .map(|(anchor_id, dependency_group_id, anchor_position_m)| {
                let range = (anchor_position_m[0].powi(2)
                    + anchor_position_m[1].powi(2)
                    + anchor_position_m[2].powi(2))
                .sqrt();
                RangeSetConstraint3d {
                    anchor_id,
                    dependency_group_id,
                    anchor_position_m,
                    anchor_position_abs_error_m: 0.0,
                    observed_range_lower_m: range,
                    observed_range_upper_m: range,
                }
            })
            .collect()
    }

    #[test]
    fn accepts_consistent_range_sets_with_three_dimensional_directional_support() {
        let result =
            audit_independent_posterior_3d([0.0; 3], 0.0, &constraints_at_origin(), config())
                .unwrap();
        assert!(result.all_range_sets_compatible);
        assert!(result.geometry_sufficient);
        assert!(result.audit_passed);
        assert_eq!(result.compatible_observation_count, 4);
        assert_eq!(result.dependency_group_count, 4);
        assert!(result.minimum_geometry_eigenvalue >= 0.05);
    }

    #[test]
    fn detects_a_range_set_that_contradicts_the_candidate_posterior() {
        let mut constraints = constraints_at_origin();
        constraints[0].observed_range_lower_m += 0.5;
        constraints[0].observed_range_upper_m += 0.7;
        let result =
            audit_independent_posterior_3d([0.0; 3], 0.0, &constraints, config()).unwrap();
        assert!(!result.all_range_sets_compatible);
        assert!(!result.audit_passed);
        assert!(result.range_constraints[0].range_set_conflict_m >= 0.5 - 1.0e-12);
    }

    #[test]
    fn anchor_and_candidate_position_bounds_expand_predicted_range_set() {
        let mut constraints = constraints_at_origin();
        constraints[0].anchor_position_abs_error_m = 0.2;
        constraints[0].observed_range_lower_m -= 0.15;
        constraints[0].observed_range_upper_m += 0.15;
        let result =
            audit_independent_posterior_3d([0.0; 3], 0.1, &constraints, config()).unwrap();
        assert!(result.range_constraints[0].predicted_range_lower_m <= 9.7);
        assert!(result.range_constraints[0].predicted_range_upper_m >= 10.3);
        assert_eq!(result.range_constraints[0].range_set_conflict_m, 0.0);
    }

    #[test]
    fn collinear_anchor_directions_fail_geometric_observability() {
        let mut constraints = constraints_at_origin();
        for (index, constraint) in constraints.iter_mut().enumerate() {
            constraint.anchor_position_m = [10.0 + index as f64, 0.0, 0.0];
            let range = constraint.anchor_position_m[0];
            constraint.observed_range_lower_m = range;
            constraint.observed_range_upper_m = range;
        }
        let result =
            audit_independent_posterior_3d([0.0; 3], 0.0, &constraints, config()).unwrap();
        assert!(result.all_range_sets_compatible);
        assert!(!result.geometry_sufficient);
        assert!(!result.audit_passed);
        assert!(result.minimum_geometry_eigenvalue < 1.0e-12);
    }

    #[test]
    fn repeated_observations_in_one_dependency_group_do_not_create_four_groups() {
        let mut constraints = constraints_at_origin();
        for constraint in &mut constraints {
            constraint.dependency_group_id = 77;
        }
        let result =
            audit_independent_posterior_3d([0.0; 3], 0.0, &constraints, config());
        assert_eq!(
            result,
            Err(IndependentPosteriorAuditError::InsufficientDependencyGroupCount)
        );
    }

    #[test]
    fn geometry_averaging_is_not_inflated_by_duplicate_rows_in_one_group() {
        let baseline = constraints_at_origin();
        let mut duplicated = baseline.clone();
        let mut repeated = baseline[0];
        repeated.anchor_id = 5;
        duplicated.push(repeated);

        let first =
            audit_independent_posterior_3d([0.0; 3], 0.0, &baseline, config()).unwrap();
        let second =
            audit_independent_posterior_3d([0.0; 3], 0.0, &duplicated, config()).unwrap();
        assert!((first.minimum_geometry_eigenvalue - second.minimum_geometry_eigenvalue).abs()
            < 1.0e-12);
    }

    #[test]
    fn rejects_duplicate_anchor_ids_and_coincident_candidate_anchor() {
        let mut duplicate = constraints_at_origin();
        duplicate[1].anchor_id = duplicate[0].anchor_id;
        assert_eq!(
            audit_independent_posterior_3d([0.0; 3], 0.0, &duplicate, config()),
            Err(IndependentPosteriorAuditError::DuplicateAnchorIdentity(1))
        );

        let mut coincident = constraints_at_origin();
        coincident[0].anchor_position_m = [0.0; 3];
        assert_eq!(
            audit_independent_posterior_3d([0.0; 3], 0.0, &coincident, config()),
            Err(IndependentPosteriorAuditError::CoincidentCandidateAndAnchor(1))
        );
    }

    #[test]
    fn invalid_range_bounds_candidate_bounds_and_config_fail_closed() {
        let mut invalid = constraints_at_origin();
        invalid[0].observed_range_upper_m = f64::NAN;
        assert_eq!(
            audit_independent_posterior_3d([0.0; 3], 0.0, &invalid, config()),
            Err(IndependentPosteriorAuditError::InvalidObservedRangeInterval(1))
        );
        assert_eq!(
            audit_independent_posterior_3d([0.0; 3], -0.1, &constraints_at_origin(), config()),
            Err(IndependentPosteriorAuditError::InvalidCandidatePositionBound)
        );

        let mut bad_config = config();
        bad_config.minimum_geometry_eigenvalue = 0.5;
        assert_eq!(
            audit_independent_posterior_3d([0.0; 3], 0.0, &constraints_at_origin(), bad_config),
            Err(IndependentPosteriorAuditError::InvalidConfiguration)
        );
    }
}
