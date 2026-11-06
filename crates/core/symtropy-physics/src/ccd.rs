// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Continuous Collision Detection (CCD): prevents fast-moving bodies from
//! tunneling through surfaces between frames.
//!
//! Uses conservative advancement with analytical solutions for common cases:
//! - **Sphere-sphere**: Quadratic solve for earliest contact time
//! - **Sphere-plane**: Linear solve for earliest contact time
//!
//! # How It Works
//!
//! After integration and before collision resolution, the CCD pass checks
//! fast-moving bodies (speed > threshold) against static/slow geometry.
//! If a tunneling event is detected, the body is moved back to the
//! time of impact (TOI) and given a corrected velocity.

use nalgebra::SVector;

/// Result of a continuous collision detection query.
#[derive(Clone, Debug)]
pub struct CcdHit<const D: usize> {
    /// Time of impact [0, 1] within the current timestep.
    /// 0 = start of step, 1 = end of step.
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Contact normal at time of impact.
    pub normal: SVector<f64, D>,
}

/// Speed threshold below which CCD is not applied (optimization).
/// Bodies moving slower than this per step won't tunnel through typical geometry.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

/// Swept sphere vs sphere: find the earliest time of impact.
///
/// Given two spheres with positions and velocities, solve for the time `t ∈ [0, dt]`
/// when `|pos_a(t) - pos_b(t)| = radius_a + radius_b`.
///
/// This is a quadratic in t:
/// `|Δp + Δv*t|² = r²` where Δp = pos_b - pos_a, Δv = vel_b - vel_a, r = ra + rb.
pub fn sphere_sphere<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    if !pos_a.iter().all(|value| value.is_finite())
        || !pos_b.iter().all(|value| value.is_finite())
        || !vel_a.iter().all(|value| value.is_finite())
        || !vel_b.iter().all(|value| value.is_finite())
        || !radius_a.is_finite()
        || !radius_b.is_finite()
        || radius_a < 0.0
        || radius_b < 0.0
        || !dt.is_finite()
        || dt < 0.0
    {
        return None;
    }

    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    let radius_sum = radius_a + radius_b;
    if !radius_sum.is_finite() {
        return None;
    }

    let relative_speed_sq = dv.dot(&dv);
    let separation_sq = dp.dot(&dp);
    let radius_sum_sq = radius_sum * radius_sum;
    if !relative_speed_sq.is_finite() || !separation_sq.is_finite() || !radius_sum_sq.is_finite() {
        return None;
    }

    let separation = separation_sq.sqrt();

    // Already overlapping or touching at t=0.
    if separation <= radius_sum {
        let normal = if separation > 1e-15 {
            dp / separation
        } else {
            let mut normal = SVector::zeros();
            normal[0] = 1.0;
            normal
        };
        return Some(CcdHit {
            toi: 0.0,
            point: pos_a + normal * radius_a,
            normal,
        });
    }

    // No relative motion, or a relative speed too small to resolve safely.
    if relative_speed_sq < 1e-20 {
        return None;
    }

    // Work in relative coordinates and find the closest point on the infinite
    // relative trajectory. This avoids subtracting two large, nearly equal
    // terms in the quadratic discriminant for long, shallow/grazing sweeps.
    let projected_time = -dp.dot(&dv) / relative_speed_sq;
    if !projected_time.is_finite() || projected_time <= 0.0 {
        return None;
    }
    let closest_separation = dp + dv * projected_time;
    let closest_distance_sq = closest_separation.dot(&closest_separation);
    if !closest_distance_sq.is_finite() {
        return None;
    }

    let radial_room_sq = radius_sum_sq - closest_distance_sq;
    let tolerance = 64.0
        * f64::EPSILON
        * radius_sum_sq
            .max(closest_distance_sq)
            .max(f64::MIN_POSITIVE);
    if radial_room_sq < -tolerance {
        return None;
    }

    let half_chord_time = (radial_room_sq.max(0.0) / relative_speed_sq).sqrt();
    let toi = projected_time - half_chord_time;
    if !toi.is_finite() || toi < 0.0 || toi > dt {
        return None;
    }

    let pos_a_t = pos_a + vel_a * toi;
    let pos_b_t = pos_b + vel_b * toi;
    let delta = pos_b_t - pos_a_t;
    let distance = delta.norm();
    if !distance.is_finite() {
        return None;
    }
    let normal = if distance > 1e-15 {
        delta / distance
    } else {
        let mut normal = SVector::zeros();
        normal[0] = 1.0;
        normal
    };

    Some(CcdHit {
        toi,
        point: pos_a_t + normal * radius_a,
        normal,
    })
}

/// Swept sphere vs half-space (infinite plane): find time of impact.
///
/// Solve for t where the signed distance to the plane equals `radius`.
/// The plane is represented by `normal · x = offset`; the offset must be
/// scaled together with the normal when representing the same plane.
pub fn sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    if !pos.iter().all(|value| value.is_finite())
        || !vel.iter().all(|value| value.is_finite())
        || !plane_normal.iter().all(|value| value.is_finite())
        || !radius.is_finite()
        || radius < 0.0
        || !plane_offset.is_finite()
        || !dt.is_finite()
        || dt < 0.0
    {
        return None;
    }

    // Plane equations are scale-invariant: n·x = offset. Normalize both the
    // normal and signed distance so callers do not need to supply a unit
    // normal to obtain a physically meaningful distance/time.
    let normal_length = plane_normal.norm();
    if !normal_length.is_finite() || normal_length <= 1e-15 {
        return None;
    }
    let unit_normal = plane_normal / normal_length;
    let signed_dist = (plane_normal.dot(pos) - plane_offset) / normal_length;
    if !signed_dist.is_finite() {
        return None;
    }

    // Already penetrating or touching.
    if signed_dist <= radius {
        let contact = pos - unit_normal * signed_dist;
        return Some(CcdHit {
            toi: 0.0,
            point: contact,
            normal: unit_normal,
        });
    }

    let vel_toward_plane = -unit_normal.dot(vel);
    if vel_toward_plane <= 0.0 {
        return None; // Moving away from plane
    }

    // Time to reach distance = radius from plane.
    let toi = (signed_dist - radius) / vel_toward_plane;
    if !toi.is_finite() || toi < 0.0 || toi > dt {
        return None;
    }

    let pos_t = pos + vel * toi;
    let contact = pos_t - unit_normal * radius;
    Some(CcdHit {
        toi,
        point: contact,
        normal: unit_normal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_sphere_head_on() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::from([-10.0, 0.0, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0).unwrap();

        // Spheres start 5 apart (center-center), need to close 4.0 (5-2*0.5),
        // closing at 20 units/s → t = 4.0/20 = 0.2
        assert!(
            (hit.toi - 0.2).abs() < 0.01,
            "TOI = {}, expected ~0.2",
            hit.toi
        );
    }

    #[test]
    fn sphere_sphere_no_collision() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([0.0, 10.0, 0.0]); // perpendicular
        let vel_b = SVector::from([0.0, 0.0, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0);
        assert!(hit.is_none(), "spheres moving apart should not collide");
    }

    #[test]
    fn sphere_sphere_already_overlapping() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([0.5, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere(&pos_a, &vel_a, 1.0, &pos_b, &vel_b, 1.0, 1.0).unwrap();
        assert!(
            (hit.toi - 0.0).abs() < 1e-12,
            "already overlapping: TOI should be 0"
        );
    }

    #[test]
    fn sphere_halfspace_falling_onto_ground() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]); // fast falling
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let hit = sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, 1.0).unwrap();

        // Sphere at y=10, radius 0.5, needs to fall to y=0.5 → distance 9.5
        // Speed 100 → t = 9.5/100 = 0.095
        assert!(
            (hit.toi - 0.095).abs() < 0.01,
            "TOI = {}, expected ~0.095",
            hit.toi
        );
        assert!(hit.normal[1] > 0.9, "normal should point up");
    }

    #[test]
    fn sphere_halfspace_moving_away() {
        let pos = SVector::from([0.0, 5.0, 0.0]);
        let vel = SVector::from([0.0, 10.0, 0.0]); // moving up
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let hit = sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, 1.0);
        assert!(hit.is_none(), "moving away from plane should not hit");
    }

    #[test]
    fn sphere_halfspace_already_penetrating() {
        let pos = SVector::from([0.0, 0.3, 0.0]);
        let vel = SVector::from([0.0, -10.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let hit = sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, 1.0).unwrap();
        assert!(
            (hit.toi - 0.0).abs() < 1e-12,
            "already penetrating: TOI = 0"
        );
    }

    #[test]
    fn sphere_sphere_grazing_at_large_coordinate_scale_has_accurate_toi() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([100_000_000.0, 0.999, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 20_000_000.0)
            .expect("near-grazing trajectory intersects");
        let expected = 10_000_000.0 - ((1.0_f64 - 0.999_f64.powi(2)) / 100.0).sqrt();

        assert!(
            (hit.toi - expected).abs() < 1e-7,
            "TOI {} differs from closest-approach reference {}",
            hit.toi,
            expected
        );
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_near_miss_at_large_coordinate_scale_is_rejected() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([100_000_000.0, 1.001, 0.0]);
        let vel_b = SVector::zeros();

        assert!(
            sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 20_000_000.0).is_none(),
            "a trajectory that misses by 1 mm must not become a hit"
        );
    }

    #[test]
    fn sphere_sphere_rejects_non_finite_and_negative_inputs() {
        let pos = SVector::zeros();
        let vel = SVector::from([1.0, 0.0, 0.0]);

        assert!(sphere_sphere(&pos, &vel, -0.5, &pos, &vel, 0.5, 1.0).is_none());
        assert!(sphere_sphere(&pos, &vel, 0.5, &pos, &vel, 0.5, -1.0).is_none());
        assert!(sphere_sphere(&pos, &vel, 0.5, &pos, &vel, 0.5, f64::NAN).is_none());

        let invalid_velocity = SVector::from([f64::INFINITY, 0.0, 0.0]);
        assert!(sphere_sphere(&pos, &invalid_velocity, 0.5, &pos, &vel, 0.5, 1.0).is_none());
    }

    #[test]
    fn sphere_halfspace_is_invariant_to_plane_normal_scaling() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 7.0, 0.0]);

        let unit_hit = sphere_halfspace(&pos, &vel, 0.5, &unit_normal, 0.0, 1.0).unwrap();
        let scaled_hit = sphere_halfspace(&pos, &vel, 0.5, &scaled_normal, 0.0, 1.0).unwrap();

        assert!((unit_hit.toi - scaled_hit.toi).abs() < 1e-12);
        assert!((unit_hit.point - scaled_hit.point).norm() < 1e-12);
        assert!((scaled_hit.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_halfspace_is_invariant_to_scaled_nonzero_plane_offset() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 7.0, 0.0]);

        let unit_hit = sphere_halfspace(&pos, &vel, 0.5, &unit_normal, 2.0, 1.0).unwrap();
        let scaled_hit = sphere_halfspace(&pos, &vel, 0.5, &scaled_normal, 14.0, 1.0).unwrap();

        assert!((unit_hit.toi - 0.075).abs() < 1e-12);
        assert!((unit_hit.toi - scaled_hit.toi).abs() < 1e-12);
        assert!((unit_hit.point - scaled_hit.point).norm() < 1e-12);
    }

    #[test]
    fn sphere_sphere_scale_relative_tolerance_does_not_admit_micro_scale_near_miss() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0e-3, 0.0, 0.0]);
        let pos_b = SVector::from([1.0e3, 1.000_001e-6, 0.0]);
        let vel_b = SVector::zeros();

        assert!(
            sphere_sphere(&pos_a, &vel_a, 0.5e-6, &pos_b, &vel_b, 0.5e-6, 2.0e6).is_none(),
            "a micro-scale trajectory missing by 1 pm must remain a miss"
        );
    }

    #[test]
    fn sphere_halfspace_rejects_invalid_plane_and_inputs() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let zero_normal = SVector::zeros();
        let normal = SVector::from([0.0, 1.0, 0.0]);

        assert!(sphere_halfspace(&pos, &vel, -0.5, &normal, 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&pos, &vel, 0.5, &zero_normal, 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, f64::INFINITY).is_none());
    }

    #[test]
    fn sphere_sphere_4d() {
        let pos_a = SVector::from([0.0, 0.0, 0.0, 0.0]);
        let vel_a = SVector::from([0.0, 0.0, 0.0, 20.0]); // moving along W
        let pos_b = SVector::from([0.0, 0.0, 0.0, 10.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere(&pos_a, &vel_a, 1.0, &pos_b, &vel_b, 1.0, 1.0).unwrap();
        // Need to close 8.0 (10 - 2*1.0) at speed 20 → t = 0.4
        assert!(
            (hit.toi - 0.4).abs() < 0.01,
            "4D TOI = {}, expected ~0.4",
            hit.toi
        );
    }

    #[test]
    fn sphere_halfspace_outside_dt() {
        let pos = SVector::from([0.0, 100.0, 0.0]);
        let vel = SVector::from([0.0, -1.0, 0.0]); // slow fall
        let normal = SVector::from([0.0, 1.0, 0.0]);

        // t = 99.5 / 1.0 = 99.5, way beyond dt=1.0
        let hit = sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, 1.0);
        assert!(hit.is_none(), "impact outside dt should not be reported");
    }

    #[test]
    fn ccd_prevents_tunneling_scenario() {
        // Scenario: sphere at y=5, moving at -500 units/s toward y=0 plane
        // In a single 16ms step: displacement = 8 units → would skip right through
        let pos = SVector::from([0.0, 5.0, 0.0]);
        let vel = SVector::from([0.0, -500.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let hit = sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, 0.016).unwrap();
        assert!(hit.toi < 0.016, "CCD should detect hit within dt");
        assert!(hit.toi > 0.0, "CCD should detect hit after start");

        // The corrected position: pos + vel * toi
        let corrected_y = 5.0 + (-500.0) * hit.toi;
        assert!(
            corrected_y >= 0.4,
            "corrected position should be above plane, y = {corrected_y}"
        );
    }
}
