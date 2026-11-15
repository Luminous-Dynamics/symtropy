//! Continuous collision reference for two independently translating,
//! fixed-orientation capsules over one common normalized time interval.
//!
//! The query is reduced to relative translation: delta = moving_delta -
//! obstacle_delta. It then reuses the bounded convex-quadratic active-set
//! solver in translation_ccd. Returned closest points are reconstructed in
//! world coordinates at the reported common time.

use crate::{
    Capsule, CapsuleAabb3, TranslationCcdError, Vec3,
    endpoints_only_translating_capsule_collision,
    translating_capsule_vs_capsule,
};

const CONTACT_EPSILON_MM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoTranslatingCapsuleClosestApproach {
    pub time_parameter: f64,
    pub moving_axis_parameter: f64,
    pub obstacle_axis_parameter: f64,
    /// Closest point on the moving capsule axis in world coordinates.
    pub moving_point: Vec3,
    /// Closest point on the obstacle capsule axis in world coordinates.
    pub obstacle_point: Vec3,
    pub distance_mm: f64,
}

impl CoTranslatingCapsuleClosestApproach {
    pub fn is_contact(self, moving_radius_mm: f64, obstacle_radius_mm: f64) -> bool {
        self.distance_mm <= moving_radius_mm + obstacle_radius_mm + CONTACT_EPSILON_MM
    }
}

/// Global closest approach between two fixed-orientation capsules that each
/// translate linearly over the same normalized time interval [0, 1].
///
/// If A(t) = A(0) + da*t and B(t) = B(0) + db*t, the distance depends on
/// relative translation (da - db)*t. The existing three-variable active-set
/// oracle therefore solves the exact bounded problem without sampling time.
///
/// Result points are translated back into world coordinates. A successful
/// result is valid only for the declared rigid-translation geometry family;
/// rotation and articulated motion are outside this API's scope.
pub fn co_translating_capsule_vs_capsule(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<CoTranslatingCapsuleClosestApproach, TranslationCcdError> {
    if !is_finite(moving_translation) || !is_finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let relative_translation = moving_translation.sub(obstacle_translation);
    let relative = translating_capsule_vs_capsule(moving, relative_translation, obstacle)?;
    let common_world_shift = obstacle_translation.scale(relative.time_parameter);
    let moving_point = relative.moving_point.add(common_world_shift);
    let obstacle_point = relative.obstacle_point.add(common_world_shift);
    let distance_mm = moving_point.distance(obstacle_point);

    if !is_finite(moving_point) || !is_finite(obstacle_point) || !distance_mm.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    Ok(CoTranslatingCapsuleClosestApproach {
        time_parameter: relative.time_parameter,
        moving_axis_parameter: relative.moving_axis_parameter,
        obstacle_axis_parameter: relative.obstacle_axis_parameter,
        moving_point,
        obstacle_point,
        distance_mm,
    })
}

/// Continuous contact truth for two fixed-orientation translating capsules.
pub fn co_translating_capsule_capsule_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    let closest = co_translating_capsule_vs_capsule(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
    )?;
    Ok(closest.is_contact(moving.radius_mm, obstacle.radius_mm))
}

/// Endpoint-only baseline at t=0 and t=1. It uses relative translation so
/// both bodies are evaluated at the same endpoint time.
pub fn endpoints_only_co_translating_capsule_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    if !is_finite(moving_translation) || !is_finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }
    endpoints_only_translating_capsule_collision(
        moving,
        moving_translation.sub(obstacle_translation),
        obstacle,
    )
}

/// Conservative swept-AABB candidate for two independently translating
/// capsules. Each capsule's initial and final AABBs are unioned over the full
/// interval. Their overlap is a broad-phase candidate only: the two swept
/// sets may intersect at different times without a synchronized contact.
pub fn co_translating_capsule_swept_aabb_candidate(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    let moving_swept = swept_bounds(moving, moving_translation)?;
    let obstacle_swept = swept_bounds(obstacle, obstacle_translation)?;
    Ok(moving_swept.overlaps(obstacle_swept))
}

fn swept_bounds(
    capsule: Capsule,
    translation: Vec3,
) -> Result<CapsuleAabb3, TranslationCcdError> {
    if !is_finite(translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let final_capsule = Capsule::new(
        capsule.start.add(translation),
        capsule.end.add(translation),
        capsule.radius_mm,
    )
    .map_err(|error| match error {
        crate::CapsuleError::InvalidRadius => TranslationCcdError::InvalidRadius,
        crate::CapsuleError::NonFiniteCoordinate => TranslationCcdError::NumericalOverflow,
    })?;
    let initial = capsule.aabb();
    let final_bounds = final_capsule.aabb();
    let bounds = CapsuleAabb3 {
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

    if !is_finite(bounds.min) || !is_finite(bounds.max) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(bounds)
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
    fn detects_head_on_mid_interval_contact_when_both_endpoints_are_clear() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let moving_delta = p(20.0, 0.0, 0.0);
        let obstacle_delta = p(-20.0, 0.0, 0.0);

        assert!(!endpoints_only_co_translating_capsule_collision(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap());
        assert!(co_translating_capsule_capsule_collision(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap());

        let closest = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        assert!((closest.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(closest.distance_mm <= CONTACT_EPSILON_MM);
        assert!((closest.moving_point.x - closest.obstacle_point.x).abs() < 1.0e-9);
    }

    #[test]
    fn equal_translation_preserves_static_separation() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 5.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 5.0, 0.0), 0.5);
        let common_delta = p(100.0, -30.0, 4.0);
        let closest = co_translating_capsule_vs_capsule(
            moving,
            common_delta,
            obstacle,
            common_delta,
        )
        .unwrap();

        assert!((closest.distance_mm - 5.0).abs() < 1.0e-9);
        assert_eq!(closest.time_parameter, 0.0);
        assert!(!closest.is_contact(moving.radius_mm, obstacle.radius_mm));
    }

    #[test]
    fn different_same_direction_speeds_use_relative_motion() {
        let moving = capsule(p(0.0, -1.0, 0.0), p(0.0, 1.0, 0.0), 0.25);
        let obstacle = capsule(p(10.0, -1.0, 0.0), p(10.0, 1.0, 0.0), 0.25);
        let closest = co_translating_capsule_vs_capsule(
            moving,
            p(20.0, 0.0, 0.0),
            obstacle,
            p(5.0, 0.0, 0.0),
        )
        .unwrap();

        assert!((closest.time_parameter - (2.0 / 3.0)).abs() < 1.0e-9);
        assert!(closest.distance_mm <= CONTACT_EPSILON_MM);
    }

    #[test]
    fn exact_tangency_at_final_time_is_contact() {
        let moving = capsule(p(-10.0, 0.0, 0.0), p(-10.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, 0.0, 0.0), p(10.0, 0.0, 0.0), 0.5);
        let closest = co_translating_capsule_vs_capsule(
            moving,
            p(9.5, 0.0, 0.0),
            obstacle,
            p(-9.5, 0.0, 0.0),
        )
        .unwrap();

        assert!((closest.time_parameter - 1.0).abs() < 1.0e-9);
        assert!((closest.distance_mm - 1.0).abs() < 1.0e-9);
        assert!(closest.is_contact(0.5, 0.5));
    }

    #[test]
    fn synchronized_near_miss_can_have_overlapping_swept_aabbs() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.25);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 0.0), 0.25);
        let shared_delta = p(10.0, 0.0, 0.0);

        assert!(co_translating_capsule_swept_aabb_candidate(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
        )
        .unwrap());
        let closest = co_translating_capsule_vs_capsule(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
        )
        .unwrap();
        assert!((closest.distance_mm - 5.0).abs() < 1.0e-9);
        assert!(!closest.is_contact(0.25, 0.25));
    }

    #[test]
    fn endpoint_clamping_and_skew_three_dimensional_axes_are_supported() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(4.0, 0.0, 0.0), 0.2);
        let obstacle = capsule(p(2.0, 0.0, 3.0), p(2.0, 0.0, 7.0), 0.2);
        let closest = co_translating_capsule_vs_capsule(
            moving,
            p(0.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
        )
        .unwrap();
        assert!((closest.distance_mm - 3.0).abs() < 1.0e-9);
        assert!((0.0..=1.0).contains(&closest.moving_axis_parameter));
        assert!((0.0..=1.0).contains(&closest.obstacle_axis_parameter));
    }

    #[test]
    fn swapping_capsules_swaps_closest_points_and_axis_parameters() {
        let moving = capsule(p(-5.0, -1.0, 0.0), p(-5.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, -2.0, 1.0), p(5.0, 2.0, 1.0), 0.25);
        let moving_delta = p(12.0, 0.0, 0.0);
        let obstacle_delta = p(-2.0, 0.0, 0.0);
        let ab = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        let ba = co_translating_capsule_vs_capsule(
            obstacle,
            obstacle_delta,
            moving,
            moving_delta,
        )
        .unwrap();

        assert!((ab.distance_mm - ba.distance_mm).abs() < 1.0e-9);
        assert!((ab.time_parameter - ba.time_parameter).abs() < 1.0e-9);
        assert!((ab.moving_point.distance(ba.obstacle_point)).abs() < 1.0e-9);
        assert!((ab.obstacle_point.distance(ba.moving_point)).abs() < 1.0e-9);
        assert!((ab.moving_axis_parameter - ba.obstacle_axis_parameter).abs() < 1.0e-9);
        assert!((ab.obstacle_axis_parameter - ba.moving_axis_parameter).abs() < 1.0e-9);
    }

    #[test]
    fn rigid_world_transform_preserves_closest_distance() {
        let moving = capsule(p(-4.0, -2.0, 0.0), p(4.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, -3.0, 3.0), p(0.0, 3.0, 3.0), 0.25);
        let moving_delta = p(0.0, 0.0, 2.0);
        let obstacle_delta = p(0.0, 0.0, -1.0);
        let original = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        let transform = |v: Vec3| p(100.0 - v.y, -30.0 + v.x, 25.0 + v.z);
        let rotate_delta = |v: Vec3| p(-v.y, v.x, v.z);
        let transformed = co_translating_capsule_vs_capsule(
            capsule(transform(moving.start), transform(moving.end), moving.radius_mm),
            rotate_delta(moving_delta),
            capsule(transform(obstacle.start), transform(obstacle.end), obstacle.radius_mm),
            rotate_delta(obstacle_delta),
        )
        .unwrap();

        assert!((original.distance_mm - transformed.distance_mm).abs() < 1.0e-9);
        assert!((original.time_parameter - transformed.time_parameter).abs() < 1.0e-9);
    }

    #[test]
    fn uniform_scaling_preserves_contact_truth() {
        let moving = capsule(p(-5.0, 0.0, 0.0), p(-5.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 2.0, 0.0), 0.5);
        let moving_delta = p(10.0, 0.0, 0.0);
        let obstacle_delta = p(-10.0, 0.0, 0.0);
        let base = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        let scale = 8.0;
        let scaled = co_translating_capsule_vs_capsule(
            capsule(moving.start.scale(scale), moving.end.scale(scale), moving.radius_mm * scale),
            moving_delta.scale(scale),
            capsule(obstacle.start.scale(scale), obstacle.end.scale(scale), obstacle.radius_mm * scale),
            obstacle_delta.scale(scale),
        )
        .unwrap();

        assert!((scaled.distance_mm - base.distance_mm * scale).abs() < 1.0e-8);
        assert_eq!(
            base.is_contact(moving.radius_mm, obstacle.radius_mm),
            scaled.is_contact(moving.radius_mm * scale, obstacle.radius_mm * scale)
        );
    }

    #[test]
    fn rejects_nonfinite_translations_and_invalid_radii() {
        let valid = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        assert_eq!(
            co_translating_capsule_vs_capsule(
                valid,
                p(0.0, f64::NAN, 0.0),
                valid,
                p(0.0, 0.0, 0.0),
            ),
            Err(TranslationCcdError::NonFiniteTranslation)
        );
        let invalid = Capsule {
            radius_mm: -1.0,
            ..valid
        };
        assert_eq!(
            co_translating_capsule_vs_capsule(
                invalid,
                p(0.0, 0.0, 0.0),
                valid,
                p(0.0, 0.0, 0.0),
            ),
            Err(TranslationCcdError::InvalidRadius)
        );
    }

    #[test]
    fn generated_results_are_never_worse_than_synchronized_time_samples() {
        let mut seed = 0x71ab_902d_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 401) as f64 - 200.0
        };

        for _ in 0..20 {
            let moving = capsule(
                p(next(), next(), next()),
                p(next(), next(), next()),
                0.5,
            );
            let obstacle = capsule(
                p(next(), next(), next()),
                p(next(), next(), next()),
                0.5,
            );
            let moving_delta = p(next(), next(), next());
            let obstacle_delta = p(next(), next(), next());
            let exact = co_translating_capsule_vs_capsule(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap();

            let mut sampled_best = f64::INFINITY;
            for ti in 0..=16 {
                let t = f64::from(ti) / 16.0;
                let moving_start = moving.start.add(moving_delta.scale(t));
                let moving_end = moving.end.add(moving_delta.scale(t));
                let obstacle_start = obstacle.start.add(obstacle_delta.scale(t));
                let obstacle_end = obstacle.end.add(obstacle_delta.scale(t));
                for ui in 0..=8 {
                    let u = f64::from(ui) / 8.0;
                    let moving_point = moving_start.add(moving_end.sub(moving_start).scale(u));
                    for vi in 0..=8 {
                        let v = f64::from(vi) / 8.0;
                        let obstacle_point =
                            obstacle_start.add(obstacle_end.sub(obstacle_start).scale(v));
                        sampled_best = sampled_best.min(moving_point.distance(obstacle_point));
                    }
                }
            }
            assert!(
                exact.distance_mm <= sampled_best + 1.0e-8,
                "analytic={} sampled={sampled_best}",
                exact.distance_mm
            );
        }
    }
}
