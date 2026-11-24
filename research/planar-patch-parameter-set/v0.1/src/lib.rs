// SPDX-License-Identifier: AGPL-3.0-or-later
//! Feasible parameter sets for the aggregate planar patch friction ellipsoid.
//!
//! For each boundary observation, the caller supplies the already
//! nondimensionalized wrench channels:
//!
//!   z = [f_u/(mu*N), f_v/(mu*N), m_n/(mu*N*L)]
//!   x = [z_0^2, z_1^2, z_2^2]
//!
//! The unknown beta = [1/e_u^2, 1/e_v^2, 1/(e_n/L)^2] must satisfy:
//!
//!   1 - epsilon_k <= x_k dot beta <= 1 + epsilon_k
//!   beta >= 0
//!
//! The intersection is a convex polyhedron in three parameters. This crate
//! enumerates feasible vertices and reports bounded or unbounded axis ranges.
//! It never regularizes a missing excitation into a point estimate.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "planar-patch-parameter-set-v0.1";

mod support_envelope;
pub use support_envelope::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterSetErrorKind {
    EmptyObservations,
    InvalidIdentity,
    DuplicateObservationId,
    InvalidInput,
    InvalidTolerance,
    EnumerationBudgetExceeded,
    NonFiniteEvaluation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterSetError {
    pub kind: ParameterSetErrorKind,
    pub reason: String,
}

impl ParameterSetError {
    fn new(kind: ParameterSetErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }
}

impl Display for ParameterSetError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for ParameterSetError {}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedBoundaryObservation {
    pub observation_id: String,
    pub trial_group_id: String,
    /// [f_u/(mu*N), f_v/(mu*N), m_n/(mu*N*L)].
    /// The torsional component must already be nondimensionalized by L.
    pub normalized_boundary_components: [f64; 3],
    /// Guaranteed bound epsilon_k on the normalized boundary equation error.
    /// Must lie in [0,1).
    pub normalized_residual_bound: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSetInput {
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    pub capacity_convention_digest: String,
    pub characteristic_length_m: f64,
    pub observations: Vec<BoundedBoundaryObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterSetConfig {
    pub feasibility_tolerance: f64,
    pub plane_pivot_relative_tolerance: f64,
    pub vertex_deduplication_tolerance: f64,
    pub maximum_plane_triples: u64,
}

impl Default for ParameterSetConfig {
    fn default() -> Self {
        Self {
            feasibility_tolerance: 1.0e-9,
            plane_pivot_relative_tolerance: 1.0e-12,
            vertex_deduplication_tolerance: 1.0e-9,
            maximum_plane_triples: 2_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterSetStatus {
    FeasibleWithFiniteIntervals,
    FeasibleWithUnboundedAxes,
    InconclusiveOrEmpty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeAxis {
    TangentialU,
    TangentialV,
    TorsionalNormal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisIdentifiability {
    FiniteInterval,
    ShapeUpperBoundUnbounded,
    BetaUpperBoundUnboundedDueToNoExcitation,
    NoFeasibleVertexForAxis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterInterval {
    /// None for the lower beta bound means no feasible vertex was established.
    pub beta_lower: Option<f64>,
    /// None means beta is unbounded above.
    pub beta_upper: Option<f64>,
    /// e = 1/sqrt(beta); the torsional axis is multiplied by L for metres.
    /// A value of 0 is an infimum when beta is unbounded above.
    pub shape_scale_lower: Option<f64>,
    /// None means the shape scale is unbounded above because beta can approach 0.
    pub shape_scale_upper: Option<f64>,
    pub identifiability: AxisIdentifiability,
    pub axis_was_excited: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSetWitness {
    pub observation_id: String,
    pub normalized_residual: f64,
    pub allowed_residual: f64,
    pub beta: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSetReport {
    pub schema_version: String,
    pub specimen_digest: String,
    pub patch_geometry_digest: String,
    pub coordinate_frame_digest: String,
    pub rig_calibration_digest: String,
    pub observation_profile_digest: String,
    pub capacity_convention_digest: String,
    pub characteristic_length_m: f64,
    pub observation_count: usize,
    pub unique_trial_group_count: usize,
    pub plane_count: usize,
    pub plane_triples_checked: u64,
    pub feasible_vertex_count: usize,
    pub duplicate_vertices_collapsed: usize,
    pub status: ParameterSetStatus,
    pub intervals: [ParameterInterval; 3],
    pub feasible_vertices: Vec<[f64; 3]>,
    pub worst_residual_witness: Option<ParameterSetWitness>,
    /// True only when every plane triple within the configured budget was checked.
    pub enumeration_complete: bool,
    /// A zero design row contradicts the strictly positive lower boundary bound.
    pub direct_infeasibility_observation_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Plane {
    normal: [f64; 3],
    bound: f64,
}

fn axis_from_index(index: usize) -> ShapeAxis {
    match index {
        0 => ShapeAxis::TangentialU,
        1 => ShapeAxis::TangentialV,
        _ => ShapeAxis::TorsionalNormal,
    }
}

fn finite3(values: [f64; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn norm3(values: [f64; 3]) -> f64 {
    values[0].hypot(values[1]).hypot(values[2])
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn validate(
    input: &ParameterSetInput,
    config: ParameterSetConfig,
) -> Result<(), ParameterSetError> {
    for (label, digest) in [
        ("specimen", &input.specimen_digest),
        ("patch geometry", &input.patch_geometry_digest),
        ("coordinate frame", &input.coordinate_frame_digest),
        ("rig calibration", &input.rig_calibration_digest),
        ("observation profile", &input.observation_profile_digest),
        ("capacity convention", &input.capacity_convention_digest),
    ] {
        if digest.trim().is_empty() {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::InvalidIdentity,
                format!("{label} digest must be non-empty"),
            ));
        }
    }
    if !input.characteristic_length_m.is_finite() || input.characteristic_length_m <= 0.0 {
        return Err(ParameterSetError::new(
            ParameterSetErrorKind::InvalidInput,
            "characteristic patch length must be finite and positive",
        ));
    }
    if input.observations.is_empty() {
        return Err(ParameterSetError::new(
            ParameterSetErrorKind::EmptyObservations,
            "at least one bounded boundary observation is required",
        ));
    }
    if !config.feasibility_tolerance.is_finite()
        || config.feasibility_tolerance <= 0.0
        || !config.plane_pivot_relative_tolerance.is_finite()
        || config.plane_pivot_relative_tolerance <= 0.0
        || config.plane_pivot_relative_tolerance >= 1.0
        || !config.vertex_deduplication_tolerance.is_finite()
        || config.vertex_deduplication_tolerance <= 0.0
        || config.maximum_plane_triples == 0
    {
        return Err(ParameterSetError::new(
            ParameterSetErrorKind::InvalidTolerance,
            "tolerances must be finite/positive and the plane-triple budget non-zero",
        ));
    }

    let mut ids = BTreeSet::new();
    for observation in &input.observations {
        if observation.observation_id.trim().is_empty()
            || observation.trial_group_id.trim().is_empty()
        {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::InvalidIdentity,
                "observation and trial-group IDs must be non-empty",
            ));
        }
        if !ids.insert(observation.observation_id.as_str()) {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::DuplicateObservationId,
                "observation IDs must be unique",
            ));
        }
        if !finite3(observation.normalized_boundary_components) {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::InvalidInput,
                "normalized boundary channels must be finite",
            ));
        }
        if !observation.normalized_residual_bound.is_finite()
            || !(0.0..1.0).contains(&observation.normalized_residual_bound)
        {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::InvalidInput,
                "normalized residual bound must be finite and in [0,1)",
            ));
        }
    }
    Ok(())
}

fn choose_three(value: u64) -> Option<u64> {
    if value < 3 {
        return Some(0);
    }
    let factors = [value, value - 1, value - 2];
    let mut reduced = factors;
    let mut divisor = 6_u64;
    for factor in &mut reduced {
        let common = gcd(*factor, divisor);
        *factor /= common;
        divisor /= common;
    }
    if divisor != 1 {
        return None;
    }
    reduced[0]
        .checked_mul(reduced[1])
        .and_then(|product| product.checked_mul(reduced[2]))
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn solve_three_planes(
    planes: [Plane; 3],
    relative_pivot_tolerance: f64,
) -> Result<Option<[f64; 3]>, ParameterSetError> {
    let mut augmented = [[0.0_f64; 4]; 3];
    for (row_index, plane) in planes.iter().enumerate() {
        let normal_norm = norm3(plane.normal);
        if !normal_norm.is_finite() {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "plane normal norm is non-finite",
            ));
        }
        if normal_norm == 0.0 {
            return Ok(None);
        }
        for (column, value) in plane.normal.iter().enumerate() {
            augmented[row_index][column] = *value / normal_norm;
        }
        augmented[row_index][3] = plane.bound / normal_norm;
        if !augmented[row_index][3].is_finite() {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "normalized plane bound is non-finite",
            ));
        }
    }

    for column in 0..3 {
        let mut pivot_row = column;
        let mut pivot_magnitude = augmented[column][column].abs();
        for (row_index, row) in augmented.iter().enumerate().skip(column + 1) {
            let candidate = row[column].abs();
            if candidate > pivot_magnitude {
                pivot_magnitude = candidate;
                pivot_row = row_index;
            }
        }
        if pivot_magnitude <= relative_pivot_tolerance {
            return Ok(None);
        }
        if pivot_row != column {
            augmented.swap(pivot_row, column);
        }
        let pivot = augmented[column][column];
        for row_index in (column + 1)..3 {
            let factor = augmented[row_index][column] / pivot;
            for entry in column..4 {
                augmented[row_index][entry] -= factor * augmented[column][entry];
            }
        }
    }

    let mut solution = [0.0_f64; 3];
    for row_index in (0..3).rev() {
        let mut value = augmented[row_index][3];
        for (column, solution_value) in solution.iter().enumerate().skip(row_index + 1) {
            value -= augmented[row_index][column] * solution_value;
        }
        let pivot = augmented[row_index][row_index];
        if pivot.abs() <= relative_pivot_tolerance {
            return Ok(None);
        }
        solution[row_index] = value / pivot;
        if !solution[row_index].is_finite() {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "plane intersection is non-finite",
            ));
        }
    }
    Ok(Some(solution))
}

fn is_feasible(
    beta: [f64; 3],
    planes: &[Plane],
    tolerance: f64,
) -> bool {
    if !finite3(beta) || beta.iter().any(|value| *value < -tolerance) {
        return false;
    }
    let beta_norm = norm3(beta);
    if !beta_norm.is_finite() {
        return false;
    }
    planes.iter().all(|plane| {
        let left = dot3(plane.normal, beta);
        let normal_norm = norm3(plane.normal);
        let scale = 1.0 + plane.bound.abs() + normal_norm * beta_norm;
        left.is_finite()
            && normal_norm.is_finite()
            && scale.is_finite()
            && left <= plane.bound + tolerance * scale
    })
}

fn same_vertex(
    left: [f64; 3],
    right: [f64; 3],
    tolerance: f64,
) -> bool {
    left.iter().zip(right.iter()).all(|(a, b)| {
        (a - b).abs() <= tolerance * (1.0 + a.abs().max(b.abs()))
    })
}

fn empty_intervals() -> [ParameterInterval; 3] {
    std::array::from_fn(|_| ParameterInterval {
        beta_lower: None,
        beta_upper: None,
        shape_scale_lower: None,
        shape_scale_upper: None,
        identifiability: AxisIdentifiability::NoFeasibleVertexForAxis,
        axis_was_excited: false,
    })
}

pub fn compute_parameter_set(
    input: &ParameterSetInput,
    config: ParameterSetConfig,
) -> Result<ParameterSetReport, ParameterSetError> {
    validate(input, config)?;
    let observation_count = input.observations.len();
    let mut trial_groups = BTreeSet::new();
    for observation in &input.observations {
        trial_groups.insert(observation.trial_group_id.as_str());
    }

    let mut direct_infeasibility = None;
    for observation in &input.observations {
        let z = observation.normalized_boundary_components;
        let x = [z[0] * z[0], z[1] * z[1], z[2] * z[2]];
        if !finite3(x) {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "squared normalized boundary channels overflowed",
            ));
        }
        if z.iter().zip(x.iter()).any(|(component, square)| {
            *component != 0.0 && *square == 0.0
        }) {
            return Err(ParameterSetError::new(
                ParameterSetErrorKind::NonFiniteEvaluation,
                "squared normalized boundary channel underflowed to zero",
            ));
        }
        if norm3(x) == 0.0 {
            direct_infeasibility = Some(observation.observation_id.clone());
            break;
        }
    }

    let direct_infeasibility_observation_id = direct_infeasibility.clone();
    let plane_capacity = observation_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(3))
        .ok_or_else(|| {
            ParameterSetError::new(
                ParameterSetErrorKind::EnumerationBudgetExceeded,
                "plane count overflowed",
            )
        })?;
    let mut planes = Vec::with_capacity(plane_capacity);
    for (index, observation) in input.observations.iter().enumerate() {
        let z = observation.normalized_boundary_components;
        let x = [z[0] * z[0], z[1] * z[1], z[2] * z[2]];
        let epsilon = observation.normalized_residual_bound;
        planes.push(Plane {
            normal: x,
            bound: 1.0 + epsilon,
        });
        planes.push(Plane {
            normal: [-x[0], -x[1], -x[2]],
            bound: -(1.0 - epsilon),
            observation_index: Some(index),
        });
    }
    planes.push(Plane {
        normal: [-1.0, 0.0, 0.0],
        bound: 0.0,
        observation_index: None,
    });
    planes.push(Plane {
        normal: [0.0, -1.0, 0.0],
        bound: 0.0,
        observation_index: None,
    });
    planes.push(Plane {
        normal: [0.0, 0.0, -1.0],
        bound: 0.0,
        observation_index: None,
    });

    let plane_count = planes.len();
    let triple_count = choose_three(plane_count as u64).ok_or_else(|| {
        ParameterSetError::new(
            ParameterSetErrorKind::EnumerationBudgetExceeded,
            "plane triple count overflowed",
        )
    })?;
    if triple_count > config.maximum_plane_triples {
        return Err(ParameterSetError::new(
            ParameterSetErrorKind::EnumerationBudgetExceeded,
            "plane triple count exceeds configured enumeration budget",
        ));
    }

    let mut vertices = Vec::<[f64; 3]>::new();
    let mut triples_checked = 0_u64;
    let mut duplicates = 0_usize;
    for first in 0..(plane_count - 2) {
        for second in (first + 1)..(plane_count - 1) {
            for third in (second + 1)..plane_count {
                triples_checked = triples_checked.checked_add(1).ok_or_else(|| {
                    ParameterSetError::new(
                        ParameterSetErrorKind::EnumerationBudgetExceeded,
                        "plane triple counter overflowed",
                    )
                })?;
                let Some(mut candidate) = solve_three_planes(
                    [planes[first], planes[second], planes[third]],
                    config.plane_pivot_relative_tolerance,
                )? else {
                    continue;
                };
                for value in &mut candidate {
                    if *value < 0.0 && *value >= -config.feasibility_tolerance {
                        *value = 0.0;
                    }
                }
                if !is_feasible(candidate, &planes, config.feasibility_tolerance) {
                    continue;
                }
                if vertices
                    .iter()
                    .any(|existing| same_vertex(*existing, candidate, config.vertex_deduplication_tolerance))
                {
                    duplicates += 1;
                } else {
                    vertices.push(candidate);
                }
            }
        }
    }

    if vertices.is_empty() || direct_infeasibility.is_some() {
        return Ok(ParameterSetReport {
            schema_version: SCHEMA_VERSION.to_owned(),
            specimen_digest: input.specimen_digest.clone(),
            patch_geometry_digest: input.patch_geometry_digest.clone(),
            coordinate_frame_digest: input.coordinate_frame_digest.clone(),
            rig_calibration_digest: input.rig_calibration_digest.clone(),
            observation_profile_digest: input.observation_profile_digest.clone(),
            capacity_convention_digest: input.capacity_convention_digest.clone(),
            characteristic_length_m: input.characteristic_length_m,
            observation_count,
            unique_trial_group_count: trial_groups.len(),
            plane_count,
            plane_triples_checked: triples_checked,
            feasible_vertex_count: 0,
            duplicate_vertices_collapsed: duplicates,
            status: ParameterSetStatus::InconclusiveOrEmpty,
            intervals: empty_intervals(),
            feasible_vertices: Vec::new(),
            worst_residual_witness: None,
            enumeration_complete: true,
            direct_infeasibility_observation_id,
        });
    }

    let mut intervals: [ParameterInterval; 3] = std::array::from_fn(|axis| {
        let minimum = vertices
            .iter()
            .map(|vertex| vertex[axis])
            .fold(f64::INFINITY, f64::min);
        let maximum = vertices
            .iter()
            .map(|vertex| vertex[axis])
            .fold(0.0_f64, f64::max);
        let excited = input.observations.iter().any(|observation| {
            observation.normalized_boundary_components[axis] != 0.0
        });
        let beta_upper = if excited { Some(maximum) } else { None };
        let characteristic_scale = if axis == 2 {
            input.characteristic_length_m
        } else {
            1.0
        };
        let shape_lower = beta_upper.map(|upper| {
            if upper > 0.0 {
                characteristic_scale / upper.sqrt()
            } else {
                f64::INFINITY
            }
        }).map(|value| if value.is_finite() { value } else { 0.0 });
        let shape_upper = if minimum > config.feasibility_tolerance {
            Some(characteristic_scale / minimum.sqrt())
        } else {
            None
        };
        let identifiability = if !excited {
            AxisIdentifiability::BetaUpperBoundUnboundedDueToNoExcitation
        } else if shape_upper.is_none() {
            AxisIdentifiability::ShapeUpperBoundUnbounded
        } else {
            AxisIdentifiability::FiniteInterval
        };
        ParameterInterval {
            beta_lower: Some(minimum.max(0.0)),
            beta_upper,
            shape_scale_lower: shape_lower,
            shape_scale_upper: shape_upper,
            identifiability,
            axis_was_excited: excited,
        }
    });

    // If a beta upper interval is unbounded, the corresponding shape-scale
    // lower bound is zero (an infimum); otherwise it is 1/sqrt(beta_max).
    for interval in &mut intervals {
        if interval.beta_upper.is_none() {
            interval.shape_scale_lower = Some(0.0);
        }
    }

    let has_unbounded_axis = intervals.iter().any(|interval| {
        interval.beta_upper.is_none() || interval.shape_scale_upper.is_none()
    });
    let status = if has_unbounded_axis {
        ParameterSetStatus::FeasibleWithUnboundedAxes
    } else {
        ParameterSetStatus::FeasibleWithFiniteIntervals
    };

    let mut worst_residual_witness: Option<ParameterSetWitness> = None;
    for vertex in &vertices {
        for observation in &input.observations {
            let z = observation.normalized_boundary_components;
            let x = [z[0] * z[0], z[1] * z[1], z[2] * z[2]];
            let residual = (dot3(x, *vertex) - 1.0).abs();
            if !residual.is_finite() {
                return Err(ParameterSetError::new(
                    ParameterSetErrorKind::NonFiniteEvaluation,
                    "boundary residual witness is non-finite",
                ));
            }
            if worst_residual_witness
                .as_ref()
                .is_none_or(|current| residual > current.normalized_residual)
            {
                worst_residual_witness = Some(ParameterSetWitness {
                    observation_id: observation.observation_id.clone(),
                    normalized_residual: residual,
                    allowed_residual: observation.normalized_residual_bound,
                    beta: *vertex,
                });
            }
        }
    }

    Ok(ParameterSetReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        specimen_digest: input.specimen_digest.clone(),
        patch_geometry_digest: input.patch_geometry_digest.clone(),
        coordinate_frame_digest: input.coordinate_frame_digest.clone(),
        rig_calibration_digest: input.rig_calibration_digest.clone(),
        observation_profile_digest: input.observation_profile_digest.clone(),
        capacity_convention_digest: input.capacity_convention_digest.clone(),
        characteristic_length_m: input.characteristic_length_m,
        observation_count,
        unique_trial_group_count: trial_groups.len(),
        plane_count,
        plane_triples_checked: triples_checked,
        feasible_vertex_count: vertices.len(),
        duplicate_vertices_collapsed: duplicates,
        status,
        intervals,
        feasible_vertices: vertices,
        worst_residual_witness,
        enumeration_complete: true,
        direct_infeasibility_observation_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        id: &str,
        group: &str,
        components: [f64; 3],
        epsilon: f64,
    ) -> BoundedBoundaryObservation {
        BoundedBoundaryObservation {
            observation_id: id.to_owned(),
            trial_group_id: group.to_owned(),
            normalized_boundary_components: components,
            normalized_residual_bound: epsilon,
        }
    }

    fn input(observations: Vec<BoundedBoundaryObservation>) -> ParameterSetInput {
        ParameterSetInput {
            specimen_digest: "specimen-parameter-set-001".to_owned(),
            patch_geometry_digest: "patch-geometry-parameter-set-001".to_owned(),
            coordinate_frame_digest: "patch-frame-parameter-set-001".to_owned(),
            rig_calibration_digest: "rig-parameter-set-001".to_owned(),
            observation_profile_digest: "bounded-boundary-observations-001".to_owned(),
            capacity_convention_digest: "mu-normal-load-characteristic-length-001".to_owned(),
            characteristic_length_m: 0.5,
            observations,
        }
    }

    fn config() -> ParameterSetConfig {
        ParameterSetConfig::default()
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!((left - right).abs() <= tolerance, "{left} != {right}");
    }

    #[test]
    fn exact_three_axis_data_recovers_singleton_parameter_set() {
        let problem = input(vec![
            observation("u", "u-trial", [2.0, 0.0, 0.0], 0.0),
            observation("v", "v-trial", [0.0, 0.5, 0.0], 0.0),
            observation("spin", "spin-trial", [0.0, 0.0, 0.25], 0.0),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("exact parameter set");
        assert_eq!(report.status, ParameterSetStatus::FeasibleWithFiniteIntervals);
        assert!(report.feasible_vertex_count >= 1);
        let expected_beta = [0.25, 4.0, 16.0];
        for (interval, expected) in report.intervals.iter().zip(expected_beta) {
            close(interval.beta_lower.unwrap(), expected, 1.0e-8);
            close(interval.beta_upper.unwrap(), expected, 1.0e-8);
            close(interval.shape_scale_lower.unwrap(), 1.0 / expected.sqrt(), 1.0e-8);
            close(interval.shape_scale_upper.unwrap(), 1.0 / expected.sqrt(), 1.0e-8);
        }
    }

    #[test]
    fn signed_force_and_moment_channels_are_squared_not_rejected() {
        let problem = input(vec![
            observation("u", "u-trial", [-2.0, 0.0, 0.0], 0.0),
            observation("v", "v-trial", [0.0, -0.5, 0.0], 0.0),
            observation("spin", "spin-trial", [0.0, 0.0, -0.25], 0.0),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("signed channels");
        assert_eq!(report.status, ParameterSetStatus::FeasibleWithFiniteIntervals);
        assert!(report.intervals.iter().all(|interval| interval.axis_was_excited));
        close(report.intervals[0].beta_lower.unwrap(), 0.25, 1.0e-8);
        close(report.intervals[1].beta_lower.unwrap(), 4.0, 1.0e-8);
        close(report.intervals[2].beta_lower.unwrap(), 16.0, 1.0e-8);
    }

    #[test]
    fn torsional_shape_interval_is_reported_in_metres() {
        let problem = input(vec![
            observation("u", "u-trial", [2.0, 0.0, 0.0], 0.0),
            observation("v", "v-trial", [0.0, 0.5, 0.0], 0.0),
            observation("spin", "spin-trial", [0.0, 0.0, 0.25], 0.0),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("torsional units");
        close(report.intervals[2].shape_scale_lower.unwrap(), 0.125, 1.0e-8);
        close(report.intervals[2].shape_scale_upper.unwrap(), 0.125, 1.0e-8);
    }

    #[test]
    fn bounded_noise_creates_finite_beta_and_shape_intervals() {
        let problem = input(vec![
            observation("u", "u-trial", [2.0, 0.0, 0.0], 0.05),
            observation("v", "v-trial", [0.0, 0.5, 0.0], 0.05),
            observation("spin", "spin-trial", [0.0, 0.0, 0.25], 0.05),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("bounded intervals");
        assert_eq!(report.status, ParameterSetStatus::FeasibleWithFiniteIntervals);
        for interval in report.intervals {
            assert!(interval.beta_lower.unwrap() > 0.0);
            assert!(interval.beta_upper.unwrap() > interval.beta_lower.unwrap());
            assert!(interval.shape_scale_upper.is_some());
        }
    }

    #[test]
    fn missing_torsional_excitation_returns_unbounded_torsional_axis_only() {
        let problem = input(vec![
            observation("u", "u-trial", [2.0, 0.0, 0.0], 0.05),
            observation("v", "v-trial", [0.0, 0.5, 0.0], 0.05),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("unbounded torsion");
        assert_eq!(report.status, ParameterSetStatus::FeasibleWithUnboundedAxes);
        assert_eq!(
            report.intervals[2].identifiability,
            AxisIdentifiability::BetaUpperBoundUnboundedDueToNoExcitation
        );
        assert!(report.intervals[2].beta_upper.is_none());
        assert_eq!(report.intervals[2].shape_scale_lower, Some(0.0));
        assert!(report.intervals[2].shape_scale_upper.is_none());
        assert!(report.intervals[0].shape_scale_upper.is_some());
        assert!(report.intervals[1].shape_scale_upper.is_some());
    }

    #[test]
    fn normalized_channel_square_underflow_is_rejected() {
        let problem = input(vec![
            observation("tiny", "tiny-trial", [1.0e-300, 0.0, 0.0], 0.05),
        ]);
        assert_eq!(
            compute_parameter_set(&problem, config())
                .expect_err("underflowed squared design")
                .kind,
            ParameterSetErrorKind::NonFiniteEvaluation
        );
    }

    #[test]
    fn shape_upper_bounds_can_be_unbounded_even_for_excited_axes() {
        let problem = input(vec![observation(
            "mixed-u-v",
            "mixed-trial",
            [1.0, 1.0, 0.0],
            0.05,
        )]);
        let report = compute_parameter_set(&problem, config()).expect("shape interval");
        assert_eq!(report.status, ParameterSetStatus::FeasibleWithUnboundedAxes);
        for axis in 0..2 {
            assert!(report.intervals[axis].axis_was_excited);
            assert!(report.intervals[axis].beta_upper.is_some());
            assert!(report.intervals[axis].shape_scale_upper.is_none());
            assert_eq!(
                report.intervals[axis].identifiability,
                AxisIdentifiability::ShapeUpperBoundUnbounded
            );
        }
        assert!(!report.intervals[2].axis_was_excited);
        assert!(report.intervals[2].beta_upper.is_none());
    }

    #[test]
    fn zero_design_row_is_a_direct_infeasibility_witness() {
        let problem = input(vec![
            observation("zero", "zero-trial", [0.0; 3], 0.05),
            observation("u", "u-trial", [2.0, 0.0, 0.0], 0.05),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("infeasibility witness");
        assert_eq!(report.status, ParameterSetStatus::InconclusiveOrEmpty);
        assert_eq!(
            report.direct_infeasibility_observation_id.as_deref(),
            Some("zero")
        );
        assert!(report.feasible_vertices.is_empty());
    }

    #[test]
    fn contradictory_exact_boundary_observations_return_inconclusive_or_empty() {
        let problem = input(vec![
            observation("u-a", "trial-a", [2.0, 0.0, 0.0], 0.0),
            observation("u-b", "trial-b", [3.0, 0.0, 0.0], 0.0),
        ]);
        let report = compute_parameter_set(&problem, config()).expect("empty feasible set");
        assert_eq!(report.status, ParameterSetStatus::InconclusiveOrEmpty);
        assert!(report.feasible_vertices.is_empty());
        assert!(report.direct_infeasibility_observation_id.is_none());
    }

    #[test]
    fn observation_order_does_not_change_parameter_intervals() {
        let observations = vec![
            observation("u", "u", [2.0, 0.0, 0.0], 0.05),
            observation("v", "v", [0.0, 0.5, 0.0], 0.05),
            observation("spin", "spin", [0.0, 0.0, 0.25], 0.05),
            observation("mixed", "mixed", [1.154700538, 0.2886751345, 0.14433756725], 0.05),
        ];
        let a = compute_parameter_set(&input(observations.clone()), config()).expect("forward");
        let b = compute_parameter_set(
            &input(observations.into_iter().rev().collect()),
            config(),
        )
        .expect("reverse");
        for (left, right) in a.intervals.iter().zip(&b.intervals) {
            close(left.beta_lower.unwrap(), right.beta_lower.unwrap(), 1.0e-8);
            close(left.beta_upper.unwrap(), right.beta_upper.unwrap(), 1.0e-8);
        }
    }

    #[test]
    fn invalid_residual_bounds_and_duplicate_ids_are_rejected() {
        let bad_epsilon = input(vec![observation("bad", "trial", [1.0, 0.0, 0.0], 1.0)]);
        assert_eq!(
            compute_parameter_set(&bad_epsilon, config())
                .expect_err("epsilon one")
                .kind,
            ParameterSetErrorKind::InvalidInput
        );

        let duplicate = input(vec![
            observation("same", "trial-a", [1.0, 0.0, 0.0], 0.05),
            observation("same", "trial-b", [0.0, 1.0, 0.0], 0.05),
        ]);
        assert_eq!(
            compute_parameter_set(&duplicate, config())
                .expect_err("duplicate ID")
                .kind,
            ParameterSetErrorKind::DuplicateObservationId
        );
    }

    #[test]
    fn plane_triple_budget_fails_closed() {
        let problem = input(vec![
            observation("u", "u", [2.0, 0.0, 0.0], 0.05),
            observation("v", "v", [0.0, 0.5, 0.0], 0.05),
            observation("spin", "spin", [0.0, 0.0, 0.25], 0.05),
        ]);
        let result = compute_parameter_set(
            &problem,
            ParameterSetConfig {
                maximum_plane_triples: 1,
                ..config()
            },
        );
        assert_eq!(
            result.expect_err("plane budget").kind,
            ParameterSetErrorKind::EnumerationBudgetExceeded
        );
    }
}
