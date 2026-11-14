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
    /// Time of impact in seconds from the beginning of the sweep, in [0, dt].
    /// 0 = start of step; dt = end of the declared timestep.
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
    // Invalid queries are treated as no hit by this Option-based compatibility
    // API. A fallible diagnostic API can be added separately if callers need
    // to distinguish invalid input from a valid miss.
    if !dt.is_finite()
        || dt < 0.0
        || !radius_a.is_finite()
        || radius_a < 0.0
        || !radius_b.is_finite()
        || radius_b < 0.0
        || !pos_a.iter().all(|value| value.is_finite())
        || !pos_b.iter().all(|value| value.is_finite())
        || !vel_a.iter().all(|value| value.is_finite())
        || !vel_b.iter().all(|value| value.is_finite())
    {
        return None;
    }

    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    let radius = radius_a + radius_b;
    if !radius.is_finite() {
        return None;
    }

    let a = dv.dot(&dv);
    let c = dp.dot(&dp) - radius * radius;
    if !a.is_finite() || !c.is_finite() {
        return None;
    }

    // Already touching or overlapping at the beginning of the sweep.
    if c <= 0.0 {
        let normal = if dp.norm() > 1e-15 {
            dp.normalize()
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

    // No relative motion.
    if a <= f64::MIN_POSITIVE {
        return None;
    }

    // Project the relative trajectory onto its closest-approach time, then
    // compute the half-width of the intersection interval. This avoids the
    // cancellation-prone b*b - 4*a*c discriminant and the direct quadratic
    // root subtraction for distant, fast-moving spheres.
    let time_of_closest_approach = -dp.dot(&dv) / a;
    if !time_of_closest_approach.is_finite() {
        return None;
    }
    let closest_delta = dp + dv * time_of_closest_approach;
    let closest_distance_squared = closest_delta.dot(&closest_delta);
    let radius_squared = radius * radius;
    if !closest_distance_squared.is_finite() || !radius_squared.is_finite() {
        return None;
    }

    let clearance_squared = radius_squared - closest_distance_squared;
    let roundoff_scale = radius_squared
        .abs()
        .max(closest_distance_squared.abs())
        .max(f64::MIN_POSITIVE);
    let roundoff_tolerance = 32.0 * f64::EPSILON * roundoff_scale;
    if clearance_squared < -roundoff_tolerance {
        return None;
    }

    let half_interval = (clearance_squared.max(0.0) / a).sqrt();
    let entry_time = time_of_closest_approach - half_interval;
    let exit_time = time_of_closest_approach + half_interval;
    let toi = [entry_time, exit_time]
        .into_iter()
        .filter(|time| time.is_finite() && *time >= 0.0 && *time <= dt)
        .min_by(f64::total_cmp)?;

    let pos_a_t = pos_a + vel_a * toi;
    let pos_b_t = pos_b + vel_b * toi;
    let delta = pos_b_t - pos_a_t;
    let normal = if delta.norm() > 1e-15 {
        delta.normalize()
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
/// Solve for the earliest t where the sphere center reaches one radius
/// from the plane. The plane is `n · x = offset`; both are normalized by
/// `|n|`, so radius and time remain in world units even for non-unit normals.
pub fn sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    if !dt.is_finite()
        || dt < 0.0
        || !radius.is_finite()
        || radius < 0.0
        || !plane_offset.is_finite()
        || !pos.iter().all(|value| value.is_finite())
        || !vel.iter().all(|value| value.is_finite())
        || !plane_normal.iter().all(|value| value.is_finite())
    {
        return None;
    }

    // Plane equations are scale-invariant: n·x = offset. Normalize both the
    // normal and offset so the sphere radius is measured in world units even
    // when callers provide a non-unit normal.
    let normal_length = plane_normal.norm();
    if !normal_length.is_finite() || normal_length <= f64::MIN_POSITIVE {
        return None;
    }
    let normal = plane_normal / normal_length;
    let normalized_offset = plane_offset / normal_length;
    let signed_distance = normal.dot(pos) - normalized_offset;
    if !signed_distance.is_finite() {
        return None;
    }

    // Already touching or penetrating at the beginning of the sweep.
    if signed_distance <= radius {
        let contact = pos - normal * signed_distance;
        return Some(CcdHit {
            toi: 0.0,
            point: contact,
            normal,
        });
    }

    let velocity_toward_plane = -normal.dot(vel);
    if !velocity_toward_plane.is_finite() || velocity_toward_plane <= 0.0 {
        return None;
    }

    let toi = (signed_distance - radius) / velocity_toward_plane;
    if toi.is_finite() && toi >= 0.0 && toi <= dt {
        let position_at_impact = pos + vel * toi;
        let contact = position_at_impact - normal * radius;
        Some(CcdHit {
            toi,
            point: contact,
            normal,
        })
    } else {
        None
    }
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
    fn time_of_impact_is_reported_in_seconds_not_normalized_fraction() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let hit = sphere_halfspace(&pos, &velocity, 0.5, &normal, 0.0, 10.0).unwrap();
        assert!((hit.toi - 9.5).abs() < 1e-12);
        assert!(hit.toi > 1.0);
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
    fn sphere_sphere_distant_fast_pair_avoids_quadratic_cancellation() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0e12, 0.0, 0.0]);
        let pos_b = SVector::from([1.0e12, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.1).unwrap();
        let expected = 1.0 - 1.0e-12;
        assert!(
            (hit.toi - expected).abs() < 5.0e-16,
            "stable TOI {}, expected {expected}",
            hit.toi
        );
    }

    #[test]
    fn sphere_halfspace_is_invariant_to_plane_normal_scaling() {
        let pos = SVector::from([0.0, 12.0, 0.0]);
        let velocity = SVector::from([0.0, -100.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 7.0, 0.0]);

        let unit_hit = sphere_halfspace(&pos, &velocity, 0.5, &unit_normal, 2.0, 1.0).unwrap();
        let scaled_hit = sphere_halfspace(&pos, &velocity, 0.5, &scaled_normal, 14.0, 1.0).unwrap();

        assert!((unit_hit.toi - scaled_hit.toi).abs() < 1e-12);
        assert!((unit_hit.point - scaled_hit.point).norm() < 1e-12);
        assert!((scaled_hit.normal.norm() - 1.0).abs() < 1e-12);
        assert!((scaled_hit.normal - unit_normal).norm() < 1e-12);
    }

    #[test]
    fn invalid_ccd_inputs_fail_closed_without_non_finite_hits() {
        let position = SVector::from([0.0, 2.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let zero_normal = SVector::zeros();
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);

        assert!(sphere_halfspace(&position, &velocity, 0.5, &zero_normal, 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&position, &velocity, -0.5, &unit_normal, 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&position, &velocity, 0.5, &unit_normal, 0.0, f64::NAN).is_none());

        assert!(
            sphere_sphere(
                &position,
                &velocity,
                f64::INFINITY,
                &SVector::zeros(),
                &SVector::zeros(),
                0.5,
                1.0
            )
            .is_none()
        );
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
