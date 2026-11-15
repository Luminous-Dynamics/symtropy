//! Bounded-clock-skew continuous collision reference for two translating
//! fixed-orientation capsules over independently observed time parameters.
//!
//! Unlike the synchronized relative-motion query, A and B are evaluated at
//! times ta and tb that may differ by at most a caller-declared epsilon.
//! The resulting minimum-distance problem is a convex quadratic over a
//! ten-inequality polytope in four variables. We enumerate every possible
//! active face with up to four independent constraints and solve its KKT
//! system. Exact singular faces are represented by lower-dimensional faces;
//! nonzero ill-conditioned systems abstain as NumericallyUnresolved.

use crate::{Capsule, CapsuleError, TranslationCcdError, Vec3};

const VARIABLE_COUNT: usize = 4;
const MAX_KKT_SIZE: usize = VARIABLE_COUNT * 2;
const CONSTRAINT_COUNT: usize = 10;
const RELATIVE_PIVOT_EPSILON: f64 = 1.0e-12;
const FEASIBILITY_EPSILON: f64 = 1.0e-10;
const CONTACT_EPSILON_MM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockSkewCcdError {
    InvalidTimeSkew,
    InvalidRadius,
    NonFiniteCoordinate,
    NonFiniteTranslation,
    NumericalOverflow,
    NumericallyUnresolved,
}

impl From<CapsuleError> for ClockSkewCcdError {
    fn from(value: CapsuleError) -> Self {
        match value {
            CapsuleError::InvalidRadius => Self::InvalidRadius,
            CapsuleError::NonFiniteCoordinate => Self::NonFiniteCoordinate,
        }
    }
}

impl From<TranslationCcdError> for ClockSkewCcdError {
    fn from(value: TranslationCcdError) -> Self {
        match value {
            TranslationCcdError::InvalidRadius => Self::InvalidRadius,
            TranslationCcdError::NonFiniteCoordinate => Self::NonFiniteCoordinate,
            TranslationCcdError::NonFiniteTranslation => Self::NonFiniteTranslation,
            TranslationCcdError::NumericalOverflow => Self::NumericalOverflow,
            TranslationCcdError::NumericallyUnresolved => Self::NumericallyUnresolved,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundedClockSkewClosestApproach {
    pub moving_time_parameter: f64,
    pub obstacle_time_parameter: f64,
    pub moving_axis_parameter: f64,
    pub obstacle_axis_parameter: f64,
    pub moving_point: Vec3,
    pub obstacle_point: Vec3,
    pub distance_mm: f64,
    /// Absolute normalized time difference at the closest-approach pair.
    pub time_skew_used: f64,
}

impl BoundedClockSkewClosestApproach {
    pub fn is_contact(self, moving_radius_mm: f64, obstacle_radius_mm: f64) -> bool {
        self.distance_mm <= moving_radius_mm + obstacle_radius_mm + CONTACT_EPSILON_MM
    }
}

#[derive(Clone, Copy)]
struct LinearConstraint {
    coefficients: [f64; VARIABLE_COUNT],
    upper_bound: f64,
}

/// Global minimum distance between two fixed-orientation capsules whose
/// linear translations are observed on potentially skewed normalized clocks.
///
/// The caller supplies a maximum relative time skew epsilon in [0, 1].
/// The minimization variables are [ta, tb, u, v] with:
/// - 0 <= ta,tb,u,v <= 1
/// - |ta - tb| <= epsilon
///
/// The returned closest points are expressed in world coordinates at their
/// respective time parameters. This is an analytic software reference for the
/// declared rigid-translation family, not a model of unknown clock behavior.
pub fn bounded_clock_skew_capsule_vs_capsule(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    maximum_time_skew: f64,
) -> Result<BoundedClockSkewClosestApproach, ClockSkewCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !is_finite(moving_translation) || !is_finite(obstacle_translation) {
        return Err(ClockSkewCcdError::NonFiniteTranslation);
    }
    if !maximum_time_skew.is_finite()
        || !(0.0..=1.0).contains(&maximum_time_skew)
    {
        return Err(ClockSkewCcdError::InvalidTimeSkew);
    }

    let moving_axis = moving.end.sub(moving.start);
    let obstacle_axis = obstacle.end.sub(obstacle.start);
    let origin = moving.start.sub(obstacle.start);
    let columns = [
        moving_translation,
        obstacle_translation.scale(-1.0),
        moving_axis,
        obstacle_axis.scale(-1.0),
    ];

    let parameters = minimize_clock_skew_box(origin, columns, maximum_time_skew)?;
    let moving_time_parameter = parameters[0];
    let obstacle_time_parameter = parameters[1];
    let moving_axis_parameter = parameters[2];
    let obstacle_axis_parameter = parameters[3];

    let moving_point = moving
        .start
        .add(moving_translation.scale(moving_time_parameter))
        .add(moving_axis.scale(moving_axis_parameter));
    let obstacle_point = obstacle
        .start
        .add(obstacle_translation.scale(obstacle_time_parameter))
        .add(obstacle_axis.scale(obstacle_axis_parameter));
    let distance_mm = moving_point.distance(obstacle_point);
    let time_skew_used = (moving_time_parameter - obstacle_time_parameter).abs();

    if !is_finite(moving_point)
        || !is_finite(obstacle_point)
        || !distance_mm.is_finite()
        || !time_skew_used.is_finite()
    {
        return Err(ClockSkewCcdError::NumericalOverflow);
    }

    Ok(BoundedClockSkewClosestApproach {
        moving_time_parameter,
        obstacle_time_parameter,
        moving_axis_parameter,
        obstacle_axis_parameter,
        moving_point,
        obstacle_point,
        distance_mm,
        time_skew_used,
    })
}

/// Continuous contact truth under the declared bounded clock-skew relation.
pub fn bounded_clock_skew_capsule_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    maximum_time_skew: f64,
) -> Result<bool, ClockSkewCcdError> {
    let closest = bounded_clock_skew_capsule_vs_capsule(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        maximum_time_skew,
    )?;
    Ok(closest.is_contact(moving.radius_mm, obstacle.radius_mm))
}

fn validate_capsule(capsule: Capsule) -> Result<(), ClockSkewCcdError> {
    Capsule::new(capsule.start, capsule.end, capsule.radius_mm)?;
    Ok(())
}

fn minimize_clock_skew_box(
    origin: Vec3,
    columns: [Vec3; VARIABLE_COUNT],
    maximum_time_skew: f64,
) -> Result<[f64; VARIABLE_COUNT], ClockSkewCcdError> {
    if !is_finite(origin) || columns.iter().any(|column| !is_finite(*column)) {
        return Err(ClockSkewCcdError::NumericalOverflow);
    }

    let scale = max_abs_vec(origin).max(
        columns
            .iter()
            .map(|column| max_abs_vec(*column))
            .fold(0.0_f64, f64::max),
    );
    if !scale.is_finite() {
        return Err(ClockSkewCcdError::NumericalOverflow);
    }
    if scale == 0.0 {
        return Ok([0.0; VARIABLE_COUNT]);
    }

    let normalized_origin = origin.scale(1.0 / scale);
    let normalized_columns = columns.map(|column| column.scale(1.0 / scale));
    let constraints = constraints(maximum_time_skew);
    let mut best: Option<(f64, [f64; VARIABLE_COUNT])> = None;
    let mut ill_conditioned_face = false;

    // Every face of a bounded four-dimensional polytope has an active subset
    // containing at most four linearly independent constraints. Enumerating all
    // 2^10 subsets is small and keeps the reference implementation auditable.
    for mask in 0_u16..(1_u16 << CONSTRAINT_COUNT) {
        let active_count = mask.count_ones() as usize;
        if active_count > VARIABLE_COUNT {
            continue;
        }

        let mut active = [0_usize; VARIABLE_COUNT];
        let mut active_index = 0;
        for (constraint_index, _) in constraints.iter().enumerate() {
            if (mask & (1 << constraint_index)) != 0 {
                active[active_index] = constraint_index;
                active_index += 1;
            }
        }

        match solve_face(
            normalized_origin,
            normalized_columns,
            &constraints,
            &active[..active_count],
        )? {
            FaceSolution::Singular => continue,
            FaceSolution::IllConditioned => {
                ill_conditioned_face = true;
                continue;
            }
            FaceSolution::Candidate(parameters) => {
                if !is_feasible(parameters, &constraints) {
                    continue;
                }

                let mut difference = normalized_origin;
                for variable in 0..VARIABLE_COUNT {
                    difference = difference
                        .add(normalized_columns[variable].scale(parameters[variable]));
                }
                let distance_squared = difference.norm_squared();
                if !distance_squared.is_finite() {
                    return Err(ClockSkewCcdError::NumericalOverflow);
                }

                let replace = match best {
                    None => true,
                    Some((best_squared, best_parameters)) => {
                        distance_squared < best_squared
                            || (distance_squared == best_squared && parameters < best_parameters)
                    }
                };
                if replace {
                    best = Some((distance_squared, parameters));
                }
            }
        }
    }

    // A near-singular face may contain the true optimum; a different face's
    // candidate cannot prove it is not better. Fail closed instead of turning
    // incomplete active-face coverage into a clear verdict.
    if ill_conditioned_face {
        return Err(ClockSkewCcdError::NumericallyUnresolved);
    }

    best.map(|(_, parameters)| parameters)
        .ok_or(ClockSkewCcdError::NumericallyUnresolved)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FaceSolution {
    Singular,
    IllConditioned,
    Candidate([f64; VARIABLE_COUNT]),
}

fn solve_face(
    origin: Vec3,
    columns: [Vec3; VARIABLE_COUNT],
    constraints: &[LinearConstraint; CONSTRAINT_COUNT],
    active: &[usize],
) -> Result<FaceSolution, ClockSkewCcdError> {
    let size = VARIABLE_COUNT + active.len();
    let mut matrix = [[0.0; MAX_KKT_SIZE]; MAX_KKT_SIZE];
    let mut right = [0.0; MAX_KKT_SIZE];

    for row in 0..VARIABLE_COUNT {
        right[row] = -columns[row].dot(origin);
        for col in 0..VARIABLE_COUNT {
            matrix[row][col] = columns[row].dot(columns[col]);
        }
    }

    for (active_offset, constraint_index) in active.iter().copied().enumerate() {
        let constraint = constraints[constraint_index];
        let kkt_index = VARIABLE_COUNT + active_offset;
        right[kkt_index] = constraint.upper_bound;
        for variable in 0..VARIABLE_COUNT {
            matrix[variable][kkt_index] = constraint.coefficients[variable];
            matrix[kkt_index][variable] = constraint.coefficients[variable];
        }
    }

    match solve_kkt(matrix, right, size)? {
        KktSolution::Singular => Ok(FaceSolution::Singular),
        KktSolution::IllConditioned => Ok(FaceSolution::IllConditioned),
        KktSolution::Solved(solution) => Ok(FaceSolution::Candidate([
            solution[0],
            solution[1],
            solution[2],
            solution[3],
        ])),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum KktSolution {
    Singular,
    IllConditioned,
    Solved([f64; MAX_KKT_SIZE]),
}

/// Partial-pivoted Gaussian elimination on a <=8x8 KKT system. The geometric
/// columns are normalized before forming H, keeping the constraint and Hessian
/// blocks on comparable numeric scales.
fn solve_kkt(
    mut matrix: [[f64; MAX_KKT_SIZE]; MAX_KKT_SIZE],
    mut right: [f64; MAX_KKT_SIZE],
    size: usize,
) -> Result<KktSolution, ClockSkewCcdError> {
    let scale = (0..size)
        .flat_map(|row| (0..size).map(move |col| matrix[row][col].abs()))
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() {
        return Err(ClockSkewCcdError::NumericalOverflow);
    }
    if scale == 0.0 {
        return Ok(KktSolution::Singular);
    }
    let pivot_threshold = scale * RELATIVE_PIVOT_EPSILON;

    for pivot_col in 0..size {
        let mut pivot_row = pivot_col;
        for row in (pivot_col + 1)..size {
            if matrix[row][pivot_col].abs() > matrix[pivot_row][pivot_col].abs() {
                pivot_row = row;
            }
        }

        let pivot_magnitude = matrix[pivot_row][pivot_col].abs();
        if !pivot_magnitude.is_finite() {
            return Err(ClockSkewCcdError::NumericalOverflow);
        }
        if pivot_magnitude == 0.0 {
            return Ok(KktSolution::Singular);
        }
        if pivot_magnitude <= pivot_threshold {
            return Ok(KktSolution::IllConditioned);
        }

        if pivot_row != pivot_col {
            matrix.swap(pivot_row, pivot_col);
            right.swap(pivot_row, pivot_col);
        }

        let pivot = matrix[pivot_col][pivot_col];
        for col in pivot_col..size {
            matrix[pivot_col][col] /= pivot;
        }
        right[pivot_col] /= pivot;

        for row in 0..size {
            if row == pivot_col {
                continue;
            }
            let factor = matrix[row][pivot_col];
            for col in pivot_col..size {
                matrix[row][col] -= factor * matrix[pivot_col][col];
            }
            right[row] -= factor * right[pivot_col];
        }
    }

    if (0..size).any(|index| !right[index].is_finite()) {
        return Err(ClockSkewCcdError::NumericalOverflow);
    }

    let mut solution = [0.0; MAX_KKT_SIZE];
    solution[..size].copy_from_slice(&right[..size]);
    Ok(KktSolution::Solved(solution))
}

fn constraints(maximum_time_skew: f64) -> [LinearConstraint; CONSTRAINT_COUNT] {
    [
        LinearConstraint {
            coefficients: [-1.0, 0.0, 0.0, 0.0],
            upper_bound: 0.0,
        },
        LinearConstraint {
            coefficients: [1.0, 0.0, 0.0, 0.0],
            upper_bound: 1.0,
        },
        LinearConstraint {
            coefficients: [0.0, -1.0, 0.0, 0.0],
            upper_bound: 0.0,
        },
        LinearConstraint {
            coefficients: [0.0, 1.0, 0.0, 0.0],
            upper_bound: 1.0,
        },
        LinearConstraint {
            coefficients: [0.0, 0.0, -1.0, 0.0],
            upper_bound: 0.0,
        },
        LinearConstraint {
            coefficients: [0.0, 0.0, 1.0, 0.0],
            upper_bound: 1.0,
        },
        LinearConstraint {
            coefficients: [0.0, 0.0, 0.0, -1.0],
            upper_bound: 0.0,
        },
        LinearConstraint {
            coefficients: [0.0, 0.0, 0.0, 1.0],
            upper_bound: 1.0,
        },
        LinearConstraint {
            coefficients: [1.0, -1.0, 0.0, 0.0],
            upper_bound: maximum_time_skew,
        },
        LinearConstraint {
            coefficients: [-1.0, 1.0, 0.0, 0.0],
            upper_bound: maximum_time_skew,
        },
    ]
}

fn is_feasible(
    parameters: [f64; VARIABLE_COUNT],
    constraints: &[LinearConstraint; CONSTRAINT_COUNT],
) -> bool {
    parameters.iter().all(|value| {
        value.is_finite()
            && *value >= -FEASIBILITY_EPSILON
            && *value <= 1.0 + FEASIBILITY_EPSILON
    }) && constraints.iter().all(|constraint| {
        let value = constraint
            .coefficients
            .iter()
            .zip(parameters)
            .map(|(coefficient, parameter)| *coefficient * parameter)
            .sum::<f64>();
        value.is_finite() && value <= constraint.upper_bound + FEASIBILITY_EPSILON
    })
}

fn max_abs_vec(vector: Vec3) -> f64 {
    vector.x.abs().max(vector.y.abs()).max(vector.z.abs())
}

fn is_finite(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        co_translating_capsule_vs_capsule, endpoints_only_co_translating_capsule_collision,
    };

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn capsule(start: Vec3, end: Vec3, radius_mm: f64) -> Capsule {
        Capsule::new(start, end, radius_mm).unwrap()
    }

    #[test]
    fn zero_skew_agrees_with_synchronized_relative_motion() {
        let moving = capsule(p(-4.0, -1.0, 0.0), p(-4.0, 1.0, 0.0), 0.25);
        let obstacle = capsule(p(4.0, -1.0, 0.5), p(4.0, 1.0, 0.5), 0.25);
        let moving_delta = p(10.0, 0.0, 0.0);
        let obstacle_delta = p(-10.0, 0.0, 0.0);

        let synced = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        let skewed = bounded_clock_skew_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
            0.0,
        )
        .unwrap();

        assert!((synced.distance_mm - skewed.distance_mm).abs() < 1.0e-8);
        assert!((synced.time_parameter - skewed.moving_time_parameter).abs() < 1.0e-8);
        assert!((synced.time_parameter - skewed.obstacle_time_parameter).abs() < 1.0e-8);
    }

    #[test]
    fn bounded_skew_finds_asynchronous_contact_missed_by_synchronized_query() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.1);
        let obstacle = capsule(p(0.5, 0.0, 0.0), p(0.5, 0.0, 0.0), 0.1);
        let moving_delta = p(1.0, 0.0, 0.0);
        let obstacle_delta = p(2.0, 0.0, 0.0);

        assert!(!endpoints_only_co_translating_capsule_collision(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap());
        let synchronized = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        assert!(!synchronized.is_contact(moving.radius_mm, obstacle.radius_mm));

        let closest = bounded_clock_skew_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
            0.5,
        )
        .unwrap();
        assert!(closest.is_contact(moving.radius_mm, obstacle.radius_mm));
        assert!((closest.moving_time_parameter - closest.obstacle_time_parameter).abs() <= 0.5 + 1.0e-10);
        assert!(closest.time_skew_used > 0.0);
    }

    #[test]
    fn equal_translation_with_skew_bound_zero_remains_separated() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 5.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 5.0, 0.0), 0.5);
        let common = p(50.0, -25.0, 3.0);
        let closest = bounded_clock_skew_capsule_vs_capsule(
            moving,
            common,
            obstacle,
            common,
            0.0,
        )
        .unwrap();
        assert!((closest.distance_mm - 5.0).abs() < 1.0e-8);
    }

    #[test]
    fn time_skew_one_allows_independent_times_across_full_intervals() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.1);
        let obstacle = capsule(p(1.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.1);
        let closest = bounded_clock_skew_capsule_vs_capsule(
            moving,
            p(1.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
            1.0,
        )
        .unwrap();
        assert!((closest.distance_mm - 0.0).abs() < 1.0e-8);
        assert!((closest.moving_time_parameter - 1.0).abs() < 1.0e-8);
        assert!(closest.is_contact(0.1, 0.1));
    }

    #[test]
    fn exact_tangency_and_clear_near_miss_are_distinguished() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.5);
        let tangent = capsule(p(0.0, 1.0, 0.0), p(0.0, 1.0, 0.0), 0.5);
        let near = capsule(p(0.0, 1.001, 0.0), p(0.0, 1.001, 0.0), 0.5);
        let zero = p(0.0, 0.0, 0.0);

        let tangent_result =
            bounded_clock_skew_capsule_vs_capsule(moving, zero, tangent, zero, 0.25).unwrap();
        let near_result =
            bounded_clock_skew_capsule_vs_capsule(moving, zero, near, zero, 0.25).unwrap();
        assert!(tangent_result.is_contact(0.5, 0.5));
        assert!(!near_result.is_contact(0.5, 0.5));
    }

    #[test]
    fn body_swap_symmetry_and_world_points_are_preserved() {
        let moving = capsule(p(-2.0, -1.0, 0.0), p(-2.0, 1.0, 0.0), 0.4);
        let obstacle = capsule(p(2.0, -1.0, 0.5), p(2.0, 1.0, 0.5), 0.3);
        let moving_delta = p(6.0, 0.0, 0.0);
        let obstacle_delta = p(-3.0, 0.0, 0.0);
        let ab = bounded_clock_skew_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
            0.4,
        )
        .unwrap();
        let ba = bounded_clock_skew_capsule_vs_capsule(
            obstacle,
            obstacle_delta,
            moving,
            moving_delta,
            0.4,
        )
        .unwrap();

        assert!((ab.distance_mm - ba.distance_mm).abs() < 1.0e-8);
        assert!((ab.moving_time_parameter - ba.obstacle_time_parameter).abs() < 1.0e-8);
        assert!((ab.obstacle_time_parameter - ba.moving_time_parameter).abs() < 1.0e-8);
        assert!((ab.moving_point.distance(ba.obstacle_point)).abs() < 1.0e-8);
        assert!((ab.obstacle_point.distance(ba.moving_point)).abs() < 1.0e-8);
    }

    #[test]
    fn rigid_transform_and_uniform_scale_preserve_distance() {
        let moving = capsule(p(-3.0, -2.0, 0.0), p(3.0, 2.0, 0.0), 0.4);
        let obstacle = capsule(p(0.0, -2.0, 2.0), p(0.0, 2.0, 2.0), 0.3);
        let moving_delta = p(0.0, 0.0, 2.0);
        let obstacle_delta = p(0.0, 0.0, -1.0);
        let base = bounded_clock_skew_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
            0.3,
        )
        .unwrap();
        let transform = |v: Vec3| p(50.0 - v.y, -20.0 + v.x, 10.0 + v.z);
        let rotate = |v: Vec3| p(-v.y, v.x, v.z);
        let rotated = bounded_clock_skew_capsule_vs_capsule(
            capsule(transform(moving.start), transform(moving.end), moving.radius_mm),
            rotate(moving_delta),
            capsule(transform(obstacle.start), transform(obstacle.end), obstacle.radius_mm),
            rotate(obstacle_delta),
            0.3,
        )
        .unwrap();
        assert!((base.distance_mm - rotated.distance_mm).abs() < 1.0e-8);
        assert!((base.moving_time_parameter - rotated.moving_time_parameter).abs() < 1.0e-8);
        assert!((base.obstacle_time_parameter - rotated.obstacle_time_parameter).abs() < 1.0e-8);

        let scale = 5.0;
        let scaled = bounded_clock_skew_capsule_vs_capsule(
            capsule(moving.start.scale(scale), moving.end.scale(scale), moving.radius_mm * scale),
            moving_delta.scale(scale),
            capsule(obstacle.start.scale(scale), obstacle.end.scale(scale), obstacle.radius_mm * scale),
            obstacle_delta.scale(scale),
            0.3,
        )
        .unwrap();
        assert!((scaled.distance_mm - base.distance_mm * scale).abs() < 1.0e-7);
    }

    #[test]
    fn rejects_invalid_skew_and_nonfinite_inputs() {
        let valid = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        for skew in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert_eq!(
                bounded_clock_skew_capsule_vs_capsule(
                    valid,
                    p(0.0, 0.0, 0.0),
                    valid,
                    p(0.0, 0.0, 0.0),
                    skew,
                ),
                Err(ClockSkewCcdError::InvalidTimeSkew)
            );
        }
        assert_eq!(
            bounded_clock_skew_capsule_vs_capsule(
                valid,
                p(0.0, f64::NAN, 0.0),
                valid,
                p(0.0, 0.0, 0.0),
                0.1,
            ),
            Err(ClockSkewCcdError::NonFiniteTranslation)
        );
    }

    #[test]
    fn generated_minima_are_no_worse_than_feasible_grid_samples() {
        let mut seed = 0x2d51_7a93_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 201) as f64 - 100.0
        };

        for _ in 0..12 {
            let moving = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.25);
            let obstacle = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.25);
            let moving_delta = p(next(), next(), next());
            let obstacle_delta = p(next(), next(), next());
            let epsilon = f64::from((next() as i64 + 100).clamp(0, 200)) / 200.0;
            let exact = bounded_clock_skew_capsule_vs_capsule(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
                epsilon,
            )
            .unwrap();

            let mut sampled_best = f64::INFINITY;
            for ta_index in 0..=8 {
                let ta = f64::from(ta_index) / 8.0;
                for tb_index in 0..=8 {
                    let tb = f64::from(tb_index) / 8.0;
                    if (ta - tb).abs() > epsilon + 1.0e-12 {
                        continue;
                    }
                    let moving_start = moving.start.add(moving_delta.scale(ta));
                    let moving_end = moving.end.add(moving_delta.scale(ta));
                    let obstacle_start = obstacle.start.add(obstacle_delta.scale(tb));
                    let obstacle_end = obstacle.end.add(obstacle_delta.scale(tb));
                    for ui in 0..=6 {
                        let u = f64::from(ui) / 6.0;
                        let moving_point = moving_start.add(moving_end.sub(moving_start).scale(u));
                        for vi in 0..=6 {
                            let v = f64::from(vi) / 6.0;
                            let obstacle_point = obstacle_start
                                .add(obstacle_end.sub(obstacle_start).scale(v));
                            sampled_best = sampled_best.min(moving_point.distance(obstacle_point));
                        }
                    }
                }
            }
            assert!(
                exact.distance_mm <= sampled_best + 1.0e-7,
                "analytic={} sampled={sampled_best}",
                exact.distance_mm
            );
        }
    }
}
