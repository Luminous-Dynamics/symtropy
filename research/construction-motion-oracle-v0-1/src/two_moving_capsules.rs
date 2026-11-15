//! Exact relative-motion oracle for two fixed-orientation capsules that each
//! undergo constant linear translation over the same normalized time interval.
//!
//! The relative displacement is d_A - d_B. We solve A relative to the initial
//! pose of B using the existing bounded active-set CCD reference, then apply
//! d_B * t to both closest points to return world-frame geometry at common time.

use crate::{
    Capsule, CapsuleAabb3, TranslationCcdError, TranslatingCapsuleClosestApproach, Vec3,
    closest_segment_approach, translating_capsule_vs_capsule,
};

const CONTACT_EPSILON_MM: f64 = 1.0e-9;

fn finite(point: Vec3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn validate_capsule(capsule: Capsule) -> Result<(), TranslationCcdError> {
    if !capsule.radius_mm.is_finite() || capsule.radius_mm < 0.0 {
        return Err(TranslationCcdError::InvalidRadius);
    }
    if !finite(capsule.start) || !finite(capsule.end) {
        return Err(TranslationCcdError::NonFiniteCoordinate);
    }
    Ok(())
}

fn swept_capsule_aabb(capsule: Capsule, translation: Vec3) -> Result<CapsuleAabb3, TranslationCcdError> {
    let initial = capsule.aabb();
    let moved = Capsule {
        start: capsule.start.add(translation),
        end: capsule.end.add(translation),
        radius_mm: capsule.radius_mm,
    };
    let final_bounds = moved.aabb();
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
    if !finite(swept.min) || !finite(swept.max) {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(swept)
}

/// Minimum separation between two capsules translating independently without
/// rotation. Both velocities are constant and both capsules share t ∈ [0, 1].
///
/// The returned closest points are transformed back into the common world frame.
pub fn two_moving_capsules_vs_capsules(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<TranslatingCapsuleClosestApproach, TranslationCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !finite(moving_translation) || !finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let relative_translation = moving_translation.sub(obstacle_translation);
    if !finite(relative_translation) {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    let relative = translating_capsule_vs_capsule(moving, relative_translation, obstacle)?;
    let common_offset = obstacle_translation.scale(relative.time_parameter);
    let moving_point = relative.moving_point.add(common_offset);
    let obstacle_point = relative.obstacle_point.add(common_offset);
    let distance_mm = moving_point.distance(obstacle_point);

    if !finite(moving_point) || !finite(obstacle_point) || !distance_mm.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    Ok(TranslatingCapsuleClosestApproach {
        time_parameter: relative.time_parameter,
        moving_axis_parameter: relative.moving_axis_parameter,
        obstacle_axis_parameter: relative.obstacle_axis_parameter,
        moving_point,
        obstacle_point,
        distance_mm,
    })
}

pub fn two_moving_capsules_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    let closest = two_moving_capsules_vs_capsules(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
    )?;
    Ok(closest.distance_mm
        <= moving.radius_mm + obstacle.radius_mm + CONTACT_EPSILON_MM)
}

/// Endpoint-only baseline. Checks both capsule pairs at t=0 and t=1 but ignores
/// every intermediate configuration.
pub fn endpoints_only_two_moving_capsules_collision(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !finite(moving_translation) || !finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }

    let limit = moving.radius_mm + obstacle.radius_mm + CONTACT_EPSILON_MM;
    let start = closest_segment_approach(
        moving.start,
        moving.end,
        obstacle.start,
        obstacle.end,
    );
    let moving_end = Capsule {
        start: moving.start.add(moving_translation),
        end: moving.end.add(moving_translation),
        radius_mm: moving.radius_mm,
    };
    let obstacle_end = Capsule {
        start: obstacle.start.add(obstacle_translation),
        end: obstacle.end.add(obstacle_translation),
        radius_mm: obstacle.radius_mm,
    };
    let end = closest_segment_approach(
        moving_end.start,
        moving_end.end,
        obstacle_end.start,
        obstacle_end.end,
    );
    if !start.distance_mm.is_finite() || !end.distance_mm.is_finite() {
        return Err(TranslationCcdError::NumericalOverflow);
    }
    Ok(start.distance_mm <= limit || end.distance_mm <= limit)
}

/// Conservative swept-AABB broad-phase candidate for two independently
/// translating fixed-orientation capsules. Overlap is only a candidate.
pub fn two_moving_capsules_aabb_candidate(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
) -> Result<bool, TranslationCcdError> {
    validate_capsule(moving)?;
    validate_capsule(obstacle)?;
    if !finite(moving_translation) || !finite(obstacle_translation) {
        return Err(TranslationCcdError::NonFiniteTranslation);
    }
    let moving_swept = swept_capsule_aabb(moving, moving_translation)?;
    let obstacle_swept = swept_capsule_aabb(obstacle, obstacle_translation)?;
    Ok(moving_swept.overlaps(obstacle_swept))
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
    fn head_on_collision_is_found_between_clear_endpoints() {
        let moving = capsule(p(-10.0, -2.0, 0.0), p(-10.0, 2.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -2.0, 0.0), p(10.0, 2.0, 0.0), 0.5);
        let moving_delta = p(20.0, 0.0, 0.0);
        let obstacle_delta = p(-20.0, 0.0, 0.0);

        assert!(
            !endpoints_only_two_moving_capsules_collision(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap()
        );
        assert!(
            two_moving_capsules_collision(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
            )
            .unwrap()
        );

        let closest = two_moving_capsules_vs_capsules(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        assert!((closest.time_parameter - 0.5).abs() < 1.0e-9);
        assert!(closest.distance_mm <= 1.0e-9);
        assert!(closest.moving_point.distance(closest.obstacle_point) <= 1.0e-9);
    }

    #[test]
    fn equal_velocity_preserves_initial_separation() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 2.0), 0.25);
        let obstacle = capsule(p(4.0, 0.0, 0.0), p(4.0, 0.0, 2.0), 0.25);
        let shared_delta = p(100.0, -50.0, 25.0);
        let closest = two_moving_capsules_vs_capsules(
            moving,
            shared_delta,
            obstacle,
            shared_delta,
        )
        .unwrap();
        assert!((closest.distance_mm - 4.0).abs() < 1.0e-9);
        assert!(!closest.distance_mm.le(&(moving.radius_mm + obstacle.radius_mm)));
    }

    #[test]
    fn receding_motion_stays_clear_and_closing_motion_collides() {
        let moving = capsule(p(-5.0, 0.0, 0.0), p(-5.0, 0.0, 1.0), 0.25);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 1.0), 0.25);
        assert!(
            !two_moving_capsules_collision(moving, p(-10.0, 0.0, 0.0), obstacle, p(10.0, 0.0, 0.0))
                .unwrap()
        );
        assert!(
            two_moving_capsules_collision(moving, p(10.0, 0.0, 0.0), obstacle, p(-10.0, 0.0, 0.0))
                .unwrap()
        );
    }

    #[test]
    fn exact_tangency_is_contact_for_two_moving_capsules() {
        let moving = capsule(p(-10.0, 0.0, 0.0), p(-10.0, 0.0, 1.0), 0.5);
        let obstacle = capsule(p(0.0, 1.0, 0.0), p(0.0, 1.0, 1.0), 0.5);
        assert!(
            two_moving_capsules_collision(moving, p(10.0, 0.0, 0.0), obstacle, p(0.0, 0.0, 0.0))
                .unwrap()
        );
    }

    #[test]
    fn time_reversal_maps_closest_time_to_one_minus_t() {
        let moving = capsule(p(-6.0, -1.0, 0.0), p(-6.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(6.0, -1.0, 0.0), p(6.0, 1.0, 0.0), 0.5);
        let da = p(14.0, 0.0, 0.0);
        let db = p(-6.0, 0.0, 0.0);
        let forward = two_moving_capsules_vs_capsules(moving, da, obstacle, db).unwrap();

        let moved_a = capsule(moving.start.add(da), moving.end.add(da), moving.radius_mm);
        let moved_b = capsule(obstacle.start.add(db), obstacle.end.add(db), obstacle.radius_mm);
        let reverse = two_moving_capsules_vs_capsules(
            moved_a,
            da.scale(-1.0),
            moved_b,
            db.scale(-1.0),
        )
        .unwrap();
        assert!((forward.time_parameter + reverse.time_parameter - 1.0).abs() < 1.0e-9);
        assert!((forward.distance_mm - reverse.distance_mm).abs() < 1.0e-9);
    }

    #[test]
    fn rigid_translation_and_rotation_preserve_relative_result() {
        let moving = capsule(p(-5.0, -1.0, 0.0), p(-5.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, -1.0, 0.0), p(5.0, 1.0, 0.0), 0.5);
        let da = p(12.0, 1.0, 0.0);
        let db = p(-8.0, -1.0, 0.0);
        let original = two_moving_capsules_vs_capsules(moving, da, obstacle, db).unwrap();

        let transform = |value: Vec3| p(100.0 - value.y, -20.0 + value.x, 30.0 + value.z);
        let rotated = two_moving_capsules_vs_capsules(
            capsule(transform(moving.start), transform(moving.end), moving.radius_mm),
            p(-da.y, da.x, da.z),
            capsule(transform(obstacle.start), transform(obstacle.end), obstacle.radius_mm),
            p(-db.y, db.x, db.z),
        )
        .unwrap();
        assert!((original.distance_mm - rotated.distance_mm).abs() < 1.0e-9);
        assert!((original.time_parameter - rotated.time_parameter).abs() < 1.0e-9);
    }

    #[test]
    fn uniform_scale_preserves_collision_truth_and_scales_distance() {
        let moving = capsule(p(-5.0, 0.0, 0.0), p(-5.0, 0.0, 1.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 1.0), 0.5);
        let da = p(10.0, 0.0, 0.0);
        let db = p(-10.0, 0.0, 0.0);
        let original = two_moving_capsules_vs_capsules(moving, da, obstacle, db).unwrap();
        let scale = 7.0;
        let scaled = two_moving_capsules_vs_capsules(
            capsule(moving.start.scale(scale), moving.end.scale(scale), moving.radius_mm * scale),
            da.scale(scale),
            capsule(obstacle.start.scale(scale), obstacle.end.scale(scale), obstacle.radius_mm * scale),
            db.scale(scale),
        )
        .unwrap();
        assert!((scaled.distance_mm - original.distance_mm * scale).abs() < 1.0e-8);
        assert_eq!(
            two_moving_capsules_collision(moving, da, obstacle, db).unwrap(),
            two_moving_capsules_collision(
                capsule(moving.start.scale(scale), moving.end.scale(scale), moving.radius_mm * scale),
                da.scale(scale),
                capsule(obstacle.start.scale(scale), obstacle.end.scale(scale), obstacle.radius_mm * scale),
                db.scale(scale),
            )
            .unwrap()
        );
    }

    #[test]
    fn relative_near_parallel_case_abstains_instead_of_reporting_clear() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(1_000.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(0.0, 10.0, 0.0), p(1_000.0, 10.000_001, 0.0), 0.5);
        assert_eq!(
            two_moving_capsules_vs_capsules(moving, p(4.0, 0.0, 0.0), obstacle, p(4.0, 0.0, 0.0)),
            Err(TranslationCcdError::NumericallyUnresolved)
        );
    }

    #[test]
    fn swept_aabb_candidate_can_overreport_but_is_conservative_for_a_crossing() {
        let moving = capsule(p(-10.0, -1.0, 0.0), p(-10.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(10.0, -1.0, 0.0), p(10.0, 1.0, 0.0), 0.5);
        let da = p(20.0, 0.0, 0.0);
        let db = p(-20.0, 0.0, 0.0);
        assert!(two_moving_capsules_aabb_candidate(moving, da, obstacle, db).unwrap());
        assert!(two_moving_capsules_collision(moving, da, obstacle, db).unwrap());
    }

    #[test]
    fn rejects_nonfinite_and_overflowing_relative_motion() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 1.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 1.0), 0.5);
        assert_eq!(
            two_moving_capsules_vs_capsules(moving, p(f64::NAN, 0.0, 0.0), obstacle, p(0.0, 0.0, 0.0)),
            Err(TranslationCcdError::NonFiniteTranslation)
        );
        assert_eq!(
            two_moving_capsules_vs_capsules(moving, p(f64::MAX, 0.0, 0.0), obstacle, p(-f64::MAX, 0.0, 0.0)),
            Err(TranslationCcdError::NumericalOverflow)
        );
    }
}
