//! Continuous collision reference for a rigid capsule under linear translation.
//!
//! The moving capsule keeps its orientation. Its centerline is translated by
//! `translation * t`, with `t ∈ [0, 1]`. The stationary capsule is unchanged.
//! The exact bounded-domain query minimizes a convex quadratic over
//! (time, moving-axis parameter, obstacle-axis parameter) ∈ [0, 1]^3.
//!
//! All 27 active/free combinations are enumerated. Each nonsingular free face
//! contributes its normal-equation stationary point when feasible. A singular
//! face has a minimizing point on its boundary, which is represented by one of
//! the lower-dimensional active sets. This gives the global box-constrained
//! minimum for the stated linear-translation geometry family, subject to the
//! documented floating-point solver tolerances.

use crate::{Capsule, CapsuleAabb3, Vec3};

const PARAMETER_EPSILON: f64 = 1.0e-10;
const PIVOT_RELATIVE_EPSILON: f64 = 1.0e-12;
const CONTACT_EPSILON_MM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranslationCcdError {
    InvalidRadius,
    NonFiniteCoordinate,
    NonFiniteTranslation,
    NumericalOverflow,
    /// A nonzero but ill-conditioned free face cannot support a reliable clear verdict.
    /// Callers must preserve this as UNRESOLVED rather than treating it as no collision.
    NumericallyUnresolved,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TranslatingCapsuleClosestApproach {
    pub time_parameter: f64,
    pub moving_axis_parameter: f64,
    pub obstacle_axis_parameter: f64,
    pub moving_point: Vec3,
    pub obstacle_point: Vec3,
    pub distance_mm: f64,
}

impl TranslatingCapsuleClosestApproach {
    pub fn is_contact(self, moving_radius_mm: f64, obstacle_radius_mm: f64) -> bool {
        self.distance_mm <= moving_radius_mm + obstacle_radius_mm + CONTACT_EPSILON_MM
    }
}

/// Global minimum distance over a capsule translated linearly without rotation
/// against a stationary capsule.
///
/// The variables are:
/// - t: translation time parameter in [0, 1];
/// - u: parameter along the moving capsule's centerline;
/// - v: parameter along the stationary capsule's centerline.
///
/// The objective is |c + d*t + e*u - f*v|². It is convex and quadratic over
/// the unit cube. We enumerate all 3³ active/free states; for each free face,
/// solve its normal equations and retain feasible stationary points.
pub fn translating_capsule_vs_capsule(
    moving: Capsule,
    translation: Vec3,
    obstacle: Capsule,
) -> Result<TranslatingCapsuleClosestApproach, TranslationCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !is_finite(translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let moving_axis = moving.end.sub(moving.start);
    let obstacle_axis = obstacle.end.sub(obstacle.start);
    let origin = moving.start.sub(obstacle.start);
    let columns = [translation, moving_axis, obstacle_axis.scale(-1.0)];

    let parameters = minimize_box_quadratic(origin, columns)?;
    let time_parameter = parameters[0];
    let moving_axis_parameter = parameters[1];
    let obstacle_axis_parameter = parameters[2];

    let moving_point = moving
        .start
        .add(translation.scale(time_parameter))
        .add(moving_axis.scale(moving_axis_parameter));
    let obstacle_point = obstacle
        .start
        .add(obstacle_axis.scale(obstacle_axis_parameter));
    let distance_mm = moving_point.distance(obstacle_point);

    if !is_finite(moving_point) || !is_finite(obstacle_point) || !distance_mm.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    Ok(TranslatingCapsuleClosestApproach {
        time_parameter,
        moving_axis_parameter,
        obstacle_axis_parameter,
        moving_point,
        obstacle_point,
        distance_mm,
    })
}

/// Contact truth for the supported translating-capsule geometry family.
pub fn translating_capsule_capsule_collision(
    moving: Capsule,
    translation: Vec3,
    obstacle: Capsule,
) -> Result<bool, TranslationCcdError> {
    let closest = translating_capsule_vs_capsule(moving, translation, obstacle)?;
    Ok(closest.is_contact(moving.radius_mm, obstacle.radius_mm))
}

/// Endpoint-only baseline. It checks the two static capsule configurations at
/// t=0 and t=1 and intentionally does not cover intermediate motion.
pub fn endpoints_only_translating_capsule_collision(
    moving: Capsule,
    translation: Vec3,
    obstacle: Capsule,
) -> Result<bool, TranslationCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !is_finite(translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let limit = moving.radius_mm + obstacle.radius_mm + CONTACT_EPSILON_MM;
    let start_distance =
        crate::closest_segment_approach(moving.start, moving.end, obstacle.start, obstacle.end)
            .distance_mm;
    let translated_start = moving.start.add(translation);
    let translated_end = moving.end.add(translation);
    let end_distance = crate::closest_segment_approach(
        translated_start,
        translated_end,
        obstacle.start,
        obstacle.end,
    )
    .distance_mm;

    if !start_distance.is_finite() || !end_distance.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(start_distance <= limit || end_distance <= limit)
}

/// Conservative swept AABB candidate for the translating capsule. The union
/// of the initial and final capsule AABBs is exact for linear translation of a
/// fixed-orientation convex capsule. Overlap is a candidate only, not contact.
pub fn translating_capsule_aabb_candidate(
    moving: Capsule,
    translation: Vec3,
    obstacle: Capsule,
) -> Result<bool, TranslationCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !is_finite(translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let initial = moving.aabb();
    let final_capsule = Capsule {
        start: moving.start.add(translation),
        end: moving.end.add(translation),
        radius_mm: moving.radius_mm,
    };
    let final_bounds = final_capsule.aabb();
    let swept = CapsuleAabb3 {
        min: Vec3::new(
            initial.min.x.min(final_bounds.min.x),
            initial.min.y.min(final_bounds.min.y),
            initial.min.z.min(final_bounds.min.z),
        ),
        max: Vec3::new(
            initial.max.x.max(final_bounds.max.x),
            initial.max.y.max(final_bounds.max.y),
            initial.max.z.max(final_bounds.max.z),
        ),
    };
    let obstacle_bounds = obstacle.aabb();

    if !is_finite(swept.min)
        || !is_finite(swept.max)
        || !is_finite(obstacle_bounds.min)
        || !is_finite(obstacle_bounds.max)
    {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(swept.overlaps(obstacle_bounds))
}

fn minimize_box_quadratic(
    origin: Vec3,
    columns: [Vec3; 3],
) -> Result<[f64; 3], TranslationCcdError> {
    let mut best: Option<(f64, [f64; 3])> = None;

    // Base-3 encoding: 0 fixes a parameter at 0, 1 fixes it at 1,
    // and 2 leaves it free on the current face.
    for mut state_code in 0_u8..27 {
        let mut parameters = [0.0; 3];
        let mut free = [0_usize; 3];
        let mut free_count = 0_usize;
        let mut residual = origin;

        for variable in 0..3 {
            match state_code % 3 {
                0 => {}
                1 => {
                    parameters[variable] = 1.0;
                    residual = residual.add(columns[variable]);
                }
                2 => {
                    free[free_count] = variable;
                    free_count += 1;
                }
                _ => unreachable!(),
            }
            state_code /= 3;
        }

        if free_count > 0 {
            let mut normal = [[0.0; 3]; 3];
            let mut right = [0.0; 3];

            for row in 0..free_count {
                let column = columns[free[row]];
                right[row] = -column.dot(residual);
                for col in 0..free_count {
                    normal[row][col] = column.dot(columns[free[col]]);
                }
            }

            let Some(solution) = solve_small_system(normal, right, free_count)? else {
                // A singular face has a minimizing point on a lower-dimensional
                // boundary face, which is enumerated separately.
                continue;
            };

            for index in 0..free_count {
                let parameter = solution[index];
                if !parameter.is_finite()
                    || parameter < -PARAMETER_EPSILON
                    || parameter > 1.0 + PARAMETER_EPSILON
                {
                    continue;
                }
                parameters[free[index]] = parameter.clamp(0.0, 1.0);
            }

            // Reject the entire face if any free parameter was infeasible.
            let feasible = (0..free_count).all(|index| {
                let value = solution[index];
                value.is_finite() && value >= -PARAMETER_EPSILON && value <= 1.0 + PARAMETER_EPSILON
            });
            if !feasible {
                continue;
            }
        }

        let mut difference = origin;
        for variable in 0..3 {
            difference = difference.add(columns[variable].scale(parameters[variable]));
        }
        let distance_squared = difference.norm_squared();
        if !distance_squared.is_finite() {
            return Err(TranslationCcdError::NumericalOverflow);
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

    best.map(|(_, parameters)| parameters)
        .ok_or(TranslationCcdError::NumericallyUnresolved)
}

/// Gaussian elimination with partial pivoting for at most a 3x3 system.
/// The singularity threshold is relative to the largest matrix coefficient so
/// uniform geometric scaling does not change the rank decision.
fn solve_small_system(
    mut matrix: [[f64; 3]; 3],
    mut right: [f64; 3],
    size: usize,
) -> Result<Option<[f64; 3]>, TranslationCcdError> {
    let scale = (0..size)
        .flat_map(|row| (0..size).map(move |col| matrix[row][col].abs()))
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    if scale == 0.0 {
        return Ok(None);
    }
    let pivot_threshold = scale * PIVOT_RELATIVE_EPSILON;

    for pivot_col in 0..size {
        let mut pivot_row = pivot_col;
        for row in (pivot_col + 1)..size {
            if matrix[row][pivot_col].abs() > matrix[pivot_row][pivot_col].abs() {
                pivot_row = row;
            }
        }

        let pivot_magnitude = matrix[pivot_row][pivot_col].abs();
        if !pivot_magnitude.is_finite() {
            return Err(TranslationCcdError::NumericalOverflow);
        }
        if pivot_magnitude == 0.0 {
            // Exactly singular free faces have a minimizer on a lower-dimensional
            // boundary face, which is enumerated separately.
            return Ok(None);
        }
        if pivot_magnitude <= pivot_threshold {
            return Err(TranslationCcdError::NumericallyUnresolved);
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
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(Some(right))
}

fn validate_capsule(capsule: Capsule) -> Result<(), TranslationCcdError> {
    if !capsule.radius_mm.is_finite() || capsule.radius_mm < 0.0 {
        return Err(TranslationCcdError::InvalidRadius);
    }
    if !is_finite(capsule.start) || !is_finite(capsule.end) {
        return Err(TranslationCcdError::NonFiniteCoordinate);
    }
    Ok(())
}

fn is_finite(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn capsule(start: Vec3, end: Vec3, radius_mm: f64) -> Capsule {
        Capsule::new(start, end, radius_mm).unwrap()
    }

    #[test]
    fn detects_transverse_collision_between_clear_endpoints() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, -2.0, 0.0), p(0.0, 2.0, 0.0), 0.5);
        let translation = p(20.0, 0.0, 0.0);

        assert!(
            !endpoints_only_translating_capsule_collision(moving, translation, obstacle).unwrap()
        );
        assert!(translating_capsule_capsule_collision(moving, translation, obstacle).unwrap());
        let closest = translating_capsule_vs_capsule(moving, translation, obstacle).unwrap();
        assert!((closest.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(closest.distance_mm <= 1.0e-9);
    }

    #[test]
    fn parallel_capsules_outside_radius_sum_are_a_clear_near_miss() {
        let moving = capsule(p(-10.0, 0.0, 0.0), p(10.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(-10.0, 2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let closest = translating_capsule_vs_capsule(moving, p(0.0, 0.0, 0.0), obstacle).unwrap();
        assert!((closest.distance_mm - 2.0).abs() < 1.0e-9);
        assert!(!closest.is_contact(moving.radius_mm, obstacle.radius_mm));
    }

    #[test]
    fn exact_tangency_is_contact() {
        let moving = capsule(p(-10.0, 0.0, 0.0), p(10.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(-10.0, 1.0, 0.0), p(10.0, 1.0, 0.0), 0.5);
        assert!(translating_capsule_capsule_collision(moving, p(0.0, 0.0, 0.0), obstacle).unwrap());
    }

    #[test]
    fn skew_three_dimensional_axes_preserve_nonzero_separation() {
        let moving = capsule(p(-5.0, 0.0, 0.0), p(5.0, 0.0, 0.0), 0.25);
        let obstacle = capsule(p(0.0, -5.0, 2.0), p(0.0, 5.0, 2.0), 0.25);
        let closest = translating_capsule_vs_capsule(moving, p(0.0, 0.0, 0.0), obstacle).unwrap();
        assert!((closest.distance_mm - 2.0).abs() < 1.0e-9);
    }

    #[test]
    fn zero_translation_matches_static_segment_distance() {
        let moving = capsule(p(-3.0, 0.0, 0.0), p(4.0, 1.0, 0.0), 0.4);
        let obstacle = capsule(p(0.0, 4.0, 1.0), p(2.0, -1.0, 1.0), 0.3);
        let expected =
            crate::closest_segment_approach(moving.start, moving.end, obstacle.start, obstacle.end)
                .distance_mm;
        let actual = translating_capsule_vs_capsule(moving, p(0.0, 0.0, 0.0), obstacle).unwrap();
        assert!((actual.distance_mm - expected).abs() < 1.0e-9);
    }

    #[test]
    fn degenerate_capsule_axes_reduce_to_translating_point_vs_point() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 0.0), 0.5);
        let closest = translating_capsule_vs_capsule(moving, p(10.0, 0.0, 0.0), obstacle).unwrap();
        assert!((closest.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(closest.distance_mm <= 1.0e-9);
    }

    #[test]
    fn endpoint_clamping_finds_optimum_at_end_of_translation() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.1);
        let obstacle = capsule(p(20.0, 3.0, 0.0), p(20.0, 3.0, 0.0), 0.1);
        let closest = translating_capsule_vs_capsule(moving, p(10.0, 0.0, 0.0), obstacle).unwrap();
        assert!((closest.time_parameter - 1.0).abs() < 1.0e-9);
        assert!((closest.distance_mm - (10.0_f64.hypot(3.0))).abs() < 1.0e-9);
    }

    #[test]
    fn swept_aabb_is_allowed_to_be_a_false_positive() {
        let moving = capsule(p(-10.0, -10.0, 0.0), p(10.0, 10.0, 0.0), 0.1);
        let obstacle = capsule(p(-10.0, -5.0, 0.0), p(10.0, 15.0, 0.0), 0.1);
        assert!(translating_capsule_aabb_candidate(moving, p(0.0, 0.0, 0.0), obstacle).unwrap());
        let closest = translating_capsule_vs_capsule(moving, p(0.0, 0.0, 0.0), obstacle).unwrap();
        assert!(closest.distance_mm > 0.2);
    }

    #[test]
    fn rigid_translation_and_quarter_turn_preserve_distance() {
        let moving = capsule(p(-4.0, -2.0, 0.0), p(4.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, -3.0, 3.0), p(0.0, 3.0, 3.0), 0.25);
        let translation = p(0.0, 0.0, 1.0);
        let original = translating_capsule_vs_capsule(moving, translation, obstacle).unwrap();

        let transform = |v: Vec3| p(100.0 - v.y, -30.0 + v.x, 25.0 + v.z);
        let moved = translating_capsule_vs_capsule(
            capsule(
                transform(moving.start),
                transform(moving.end),
                moving.radius_mm,
            ),
            p(-translation.y, translation.x, translation.z),
            capsule(
                transform(obstacle.start),
                transform(obstacle.end),
                obstacle.radius_mm,
            ),
        )
        .unwrap();
        assert!((original.distance_mm - moved.distance_mm).abs() < 1.0e-9);
    }

    #[test]
    fn uniform_scaling_preserves_normalized_collision_truth() {
        let moving = capsule(p(-4.0, 0.0, 0.0), p(4.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, 2.0, 0.0), p(0.0, 4.0, 0.0), 0.5);
        let translation = p(0.0, 0.0, 1.0);
        let base = translating_capsule_vs_capsule(moving, translation, obstacle).unwrap();

        let scale = 7.0;
        let scaled = translating_capsule_vs_capsule(
            capsule(
                moving.start.scale(scale),
                moving.end.scale(scale),
                moving.radius_mm * scale,
            ),
            translation.scale(scale),
            capsule(
                obstacle.start.scale(scale),
                obstacle.end.scale(scale),
                obstacle.radius_mm * scale,
            ),
        )
        .unwrap();
        assert!((scaled.distance_mm - base.distance_mm * scale).abs() < 1.0e-8);
        assert_eq!(
            base.is_contact(moving.radius_mm, obstacle.radius_mm),
            scaled.is_contact(moving.radius_mm * scale, obstacle.radius_mm * scale)
        );
    }

    #[test]
    fn rejects_invalid_radii_and_nonfinite_inputs() {
        let valid = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        let invalid_radius = Capsule {
            radius_mm: -1.0,
            ..valid
        };
        assert_eq!(
            translating_capsule_vs_capsule(invalid_radius, p(0.0, 0.0, 0.0), valid),
            Err(TranslationCcdError::InvalidRadius)
        );
        assert_eq!(
            translating_capsule_vs_capsule(valid, p(f64::NAN, 0.0, 0.0), valid),
            Err(TranslationCcdError::NonFiniteTranslation)
        );
        let invalid_point = Capsule {
            start: p(f64::INFINITY, 0.0, 0.0),
            ..valid
        };
        assert_eq!(
            translating_capsule_vs_capsule(invalid_point, p(0.0, 0.0, 0.0), valid),
            Err(TranslationCcdError::NonFiniteCoordinate)
        );
    }

    #[test]
    fn near_parallel_free_face_is_explicitly_unresolved() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(1_000.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, 10.0, 0.0), p(1_000.0, 10.000_001, 0.0), 0.5);
        assert_eq!(
            translating_capsule_vs_capsule(moving, p(0.0, 0.0, 0.0), obstacle),
            Err(TranslationCcdError::NumericallyUnresolved)
        );
    }

    #[test]
    fn generated_analytic_minima_are_never_worse_than_grid_samples() {
        // Deterministic LCG fixtures. Sampling is a one-sided sanity control,
        // not the source of analytic truth and not a proof of global optimality.
        let mut seed = 0x9e37_79b9_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 401) as f64 - 200.0
        };

        for _ in 0..16 {
            let moving = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.25);
            let obstacle = capsule(p(next(), next(), next()), p(next(), next(), next()), 0.25);
            let translation = p(next(), next(), next());
            let exact = translating_capsule_vs_capsule(moving, translation, obstacle).unwrap();

            let mut sampled_best = f64::INFINITY;
            for ti in 0..=12 {
                for ui in 0..=8 {
                    for vi in 0..=8 {
                        let t = f64::from(ti) / 12.0;
                        let u = f64::from(ui) / 8.0;
                        let v = f64::from(vi) / 8.0;
                        let moving_point = moving
                            .start
                            .add(translation.scale(t))
                            .add(moving.end.sub(moving.start).scale(u));
                        let obstacle_point = obstacle
                            .start
                            .add(obstacle.end.sub(obstacle.start).scale(v));
                        sampled_best = sampled_best.min(moving_point.distance(obstacle_point));
                    }
                }
            }
            assert!(
                exact.distance_mm <= sampled_best + 1.0e-8,
                "analytic={} sampled={sampled_best}",
                exact.distance_mm
            );
            assert!((0.0..=1.0).contains(&exact.time_parameter));
            assert!((0.0..=1.0).contains(&exact.moving_axis_parameter));
            assert!((0.0..=1.0).contains(&exact.obstacle_axis_parameter));
        }
    }
}
