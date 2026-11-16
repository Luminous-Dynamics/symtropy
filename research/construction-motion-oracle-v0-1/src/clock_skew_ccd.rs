//! Conservative relative-motion classification under bounded per-controller
//! trajectory clock phase error.
//!
//! The nominal oracle assumes a shared normalized time axis. A local trajectory
//! phase error moves a capsule's centerline by at most
//! `||translation|| * phase_error` over the supported affine horizon.
//! The sum of both body bounds widens the nominal closest-distance result into
//! a robust interval. This module deliberately abstains in the uncertain band;
//! it is not an exact solver for arbitrary offset-clock trajectories.

use crate::{
    Capsule, CoTranslatingCapsuleClosestApproach, TranslationCcdError, Vec3,
    co_translating_capsule_vs_capsule,
};

const CONTACT_EPSILON_MM: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockPhaseErrorBounds {
    /// Maximum absolute interpolation-phase error for capsule A, normalized
    /// to its declared linear-translation horizon.
    pub moving_phase_error_bound: f64,
    /// Maximum absolute interpolation-phase error for capsule B.
    pub obstacle_phase_error_bound: f64,
}

impl ClockPhaseErrorBounds {
    fn validate(self) -> Result<(), TranslationCcdError> {
        for value in [
            self.moving_phase_error_bound,
            self.obstacle_phase_error_bound,
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(TranslationCcdError::InvalidPhaseErrorBound);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockRobustCollisionStatus {
    /// Every admissible phase perturbation remains within the declared contact
    /// predicate at the nominal closest-approach time.
    DefinitelyContact,
    /// No admissible phase perturbation can bring the capsules into the
    /// declared contact predicate anywhere in the shared nominal horizon.
    DefinitelyClear,
    /// The conservative distance interval intersects the contact threshold.
    /// This status must not be converted to NO_CONTACT.
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockRobustCollisionReport {
    pub status: ClockRobustCollisionStatus,
    pub nominal_closest_approach: CoTranslatingCapsuleClosestApproach,
    pub phase_error_bounds: ClockPhaseErrorBounds,
    pub maximum_relative_displacement_mm: f64,
    pub nominal_minimum_distance_mm: f64,
    pub lower_distance_bound_mm: f64,
    pub upper_distance_bound_mm: f64,
    /// The threshold follows the existing capsule contact predicate:
    /// radiusA + radiusB + 1e-9 mm.
    pub contact_threshold_mm: f64,
}

/// Classify co-translating capsule contact conservatively when each local
/// trajectory interpolation phase has a bounded absolute error.
///
/// If capsule A has total translation dA and phase error epsilonA, every point
/// on its axis can move by at most ||dA||*epsilonA relative to the nominal
/// shared-time position. The same holds for B. By the triangle inequality,
/// the actual relative distance differs from its nominal value by no more than
/// E = ||dA||*epsilonA + ||dB||*epsilonB.
///
/// Let d be the exact nominal minimum distance and R the existing contact
/// threshold. Then every admissible trajectory has minimum distance in the
/// conservative interval [max(0,d-E), d+E]. Clear/contact are only returned
/// when the entire interval lies on one side of R. Otherwise the result is
/// UNRESOLVED. This is a robust bound, not a new exact CCD solution.
pub fn co_translating_capsule_clock_robust_query(
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    phase_error_bounds: ClockPhaseErrorBounds,
) -> Result<ClockRobustCollisionReport, TranslationCcdError> {
    phase_error_bounds.validate()?;

    let nominal = co_translating_capsule_vs_capsule(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
    )?;

    let moving_translation_norm = stable_norm(moving_translation);
    let obstacle_translation_norm = stable_norm(obstacle_translation);
    let moving_error_mm =
        moving_translation_norm * phase_error_bounds.moving_phase_error_bound;
    let obstacle_error_mm =
        obstacle_translation_norm * phase_error_bounds.obstacle_phase_error_bound;
    let maximum_relative_displacement_mm = moving_error_mm + obstacle_error_mm;
    let lower_distance_bound_mm =
        (nominal.distance_mm - maximum_relative_displacement_mm).max(0.0);
    let upper_distance_bound_mm =
        nominal.distance_mm + maximum_relative_displacement_mm;
    let contact_threshold_mm =
        moving.radius_mm + obstacle.radius_mm + CONTACT_EPSILON_MM;

    if !moving_translation_norm.is_finite()
        || !obstacle_translation_norm.is_finite()
        || !maximum_relative_displacement_mm.is_finite()
        || !lower_distance_bound_mm.is_finite()
        || !upper_distance_bound_mm.is_finite()
        || !contact_threshold_mm.is_finite()
    {
        return Err(TranslationCcdError::NumericalOverflow);
    }

    let status = if lower_distance_bound_mm > contact_threshold_mm {
        ClockRobustCollisionStatus::DefinitelyClear
    } else if upper_distance_bound_mm <= contact_threshold_mm {
        ClockRobustCollisionStatus::DefinitelyContact
    } else {
        ClockRobustCollisionStatus::Unresolved
    };

    Ok(ClockRobustCollisionReport {
        status,
        nominal_closest_approach: nominal,
        phase_error_bounds,
        maximum_relative_displacement_mm,
        nominal_minimum_distance_mm: nominal.distance_mm,
        lower_distance_bound_mm,
        upper_distance_bound_mm,
        contact_threshold_mm,
    })
}

fn stable_norm(vector: Vec3) -> f64 {
    vector.x.hypot(vector.y).hypot(vector.z)
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

    fn errors(moving: f64, obstacle: f64) -> ClockPhaseErrorBounds {
        ClockPhaseErrorBounds {
            moving_phase_error_bound: moving,
            obstacle_phase_error_bound: obstacle,
        }
    }

    #[test]
    fn zero_phase_error_reproduces_nominal_contact_predicate() {
        let moving = capsule(p(-5.0, 0.0, 0.0), p(-5.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 1.0, 0.0), 0.5);
        let moving_delta = p(10.0, 0.0, 0.0);
        let obstacle_delta = p(-10.0, 0.0, 0.0);
        let nominal = co_translating_capsule_vs_capsule(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
        )
        .unwrap();
        let robust = co_translating_capsule_clock_robust_query(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
            errors(0.0, 0.0),
        )
        .unwrap();

        assert_eq!(
            robust.status,
            if nominal.is_contact(moving.radius_mm, obstacle.radius_mm) {
                ClockRobustCollisionStatus::DefinitelyContact
            } else {
                ClockRobustCollisionStatus::DefinitelyClear
            }
        );
        assert_eq!(robust.maximum_relative_displacement_mm, 0.0);
        assert_eq!(robust.lower_distance_bound_mm, nominal.distance_mm);
        assert_eq!(robust.upper_distance_bound_mm, nominal.distance_mm);
    }

    #[test]
    fn distant_pair_remains_definitely_clear_under_asymmetric_phase_error() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 2.0), 0.5);
        let obstacle = capsule(p(5.0, 0.0, 0.0), p(5.0, 0.0, 2.0), 0.5);
        let report = co_translating_capsule_clock_robust_query(
            moving,
            p(1.0, 0.0, 0.0),
            obstacle,
            p(0.0, 2.0, 0.0),
            errors(0.1, 0.05),
        )
        .unwrap();

        assert_eq!(report.status, ClockRobustCollisionStatus::DefinitelyClear);
        assert!(report.lower_distance_bound_mm > report.contact_threshold_mm);
        assert!((report.maximum_relative_displacement_mm - 0.2).abs() < 1.0e-12);
    }

    #[test]
    fn overlap_with_margin_remains_definitely_contact_for_all_phase_errors() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 2.0), 1.0);
        let obstacle = capsule(p(0.1, 0.0, 0.0), p(0.1, 0.0, 2.0), 1.0);
        let report = co_translating_capsule_clock_robust_query(
            moving,
            p(1.0, 0.0, 0.0),
            obstacle,
            p(-1.0, 0.0, 0.0),
            errors(0.1, 0.1),
        )
        .unwrap();

        assert_eq!(
            report.status,
            ClockRobustCollisionStatus::DefinitelyContact
        );
        assert!(report.upper_distance_bound_mm <= report.contact_threshold_mm);
    }

    #[test]
    fn near_boundary_becomes_unresolved_not_clear() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.0), 0.5);
        let obstacle = capsule(p(1.05, 0.0, 0.0), p(1.05, 0.0, 0.0), 0.5);
        let report = co_translating_capsule_clock_robust_query(
            moving,
            p(1.0, 0.0, 0.0),
            obstacle,
            p(0.0, 0.0, 0.0),
            errors(0.1, 0.0),
        )
        .unwrap();

        assert_eq!(report.status, ClockRobustCollisionStatus::Unresolved);
        assert!(
            report.lower_distance_bound_mm <= report.contact_threshold_mm
                && report.upper_distance_bound_mm > report.contact_threshold_mm
        );
    }

    #[test]
    fn swapping_capsules_swaps_nominal_witness_and_preserves_uncertainty_interval() {
        let moving = capsule(p(-4.0, -1.0, 0.0), p(-4.0, 1.0, 0.0), 0.5);
        let obstacle = capsule(p(4.0, -2.0, 1.0), p(4.0, 2.0, 1.0), 0.25);
        let moving_delta = p(8.0, 0.0, 0.0);
        let obstacle_delta = p(-1.0, 0.0, 0.0);
        let ab = co_translating_capsule_clock_robust_query(
            moving,
            moving_delta,
            obstacle,
            obstacle_delta,
            errors(0.02, 0.1),
        )
        .unwrap();
        let ba = co_translating_capsule_clock_robust_query(
            obstacle,
            obstacle_delta,
            moving,
            moving_delta,
            errors(0.1, 0.02),
        )
        .unwrap();

        assert_eq!(ab.status, ba.status);
        assert!((ab.maximum_relative_displacement_mm - ba.maximum_relative_displacement_mm).abs() < 1.0e-12);
        assert!((ab.lower_distance_bound_mm - ba.lower_distance_bound_mm).abs() < 1.0e-12);
        assert!((ab.upper_distance_bound_mm - ba.upper_distance_bound_mm).abs() < 1.0e-12);
        assert!((ab.nominal_closest_approach.moving_point.distance(ba.nominal_closest_approach.obstacle_point)) < 1.0e-9);
        assert!((ab.nominal_closest_approach.obstacle_point.distance(ba.nominal_closest_approach.moving_point)) < 1.0e-9);
    }

    #[test]
    fn rejects_invalid_phase_error_bounds_and_overflow() {
        let moving = capsule(p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), 0.5);
        assert_eq!(
            co_translating_capsule_clock_robust_query(
                moving,
                p(0.0, 0.0, 0.0),
                moving,
                p(0.0, 0.0, 0.0),
                errors(f64::NAN, 0.0),
            ),
            Err(TranslationCcdError::InvalidPhaseErrorBound)
        );
        assert_eq!(
            co_translating_capsule_clock_robust_query(
                moving,
                p(0.0, 0.0, 0.0),
                moving,
                p(0.0, 0.0, 0.0),
                errors(0.0, -0.1),
            ),
            Err(TranslationCcdError::InvalidPhaseErrorBound)
        );
        assert_eq!(
            co_translating_capsule_clock_robust_query(
                moving,
                p(0.0, 0.0, 0.0),
                moving,
                p(0.0, 0.0, 0.0),
                errors(1.01, 0.0),
            ),
            Err(TranslationCcdError::InvalidPhaseErrorBound)
        );
        assert_eq!(
            co_translating_capsule_clock_robust_query(
                moving,
                p(f64::MAX, f64::MAX, f64::MAX),
                moving,
                p(0.0, 0.0, 0.0),
                errors(1.0, 0.0),
            ),
            Err(TranslationCcdError::NumericalOverflow)
        );
    }

    #[test]
    fn sampled_phase_perturbations_agree_with_definite_classifications() {
        let mut seed = 0x71c10c_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 32) % 2_001) as f64 / 100.0 - 10.0
        };

        for _ in 0..128 {
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
            let phase_bounds = errors(0.08, 0.12);
            let robust = co_translating_capsule_clock_robust_query(
                moving,
                moving_delta,
                obstacle,
                obstacle_delta,
                phase_bounds,
            )
            .unwrap();

            for _ in 0..8 {
                let moving_phase = (next() / 10.0).clamp(-phase_bounds.moving_phase_error_bound, phase_bounds.moving_phase_error_bound);
                let obstacle_phase = (next() / 10.0).clamp(-phase_bounds.obstacle_phase_error_bound, phase_bounds.obstacle_phase_error_bound);
                let shifted_moving = Capsule {
                    start: moving.start.add(moving_delta.scale(moving_phase)),
                    end: moving.end.add(moving_delta.scale(moving_phase)),
                    ..moving
                };
                let shifted_obstacle = Capsule {
                    start: obstacle.start.add(obstacle_delta.scale(obstacle_phase)),
                    end: obstacle.end.add(obstacle_delta.scale(obstacle_phase)),
                    ..obstacle
                };
                let actual = co_translating_capsule_vs_capsule(
                    shifted_moving,
                    moving_delta,
                    shifted_obstacle,
                    obstacle_delta,
                )
                .unwrap();
                let actual_contact =
                    actual.is_contact(moving.radius_mm, obstacle.radius_mm);

                match robust.status {
                    ClockRobustCollisionStatus::DefinitelyContact => {
                        assert!(actual_contact);
                    }
                    ClockRobustCollisionStatus::DefinitelyClear => {
                        assert!(!actual_contact);
                    }
                    ClockRobustCollisionStatus::Unresolved => {}
                }
            }
        }
    }
}
