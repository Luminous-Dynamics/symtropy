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
    if !radius_a.is_finite()
        || radius_a < 0.0
        || !radius_b.is_finite()
        || radius_b < 0.0
        || !dt.is_finite()
        || dt < 0.0
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
    let b = 2.0 * dp.dot(&dv);
    let c = dp.dot(&dp) - radius * radius;
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return None;
    }

    // Already overlapping at t=0.
    if c <= 0.0 {
        let distance = dp.norm();
        if !distance.is_finite() {
            return None;
        }
        let normal = if distance > 1e-15 {
            dp / distance
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

    // No meaningful relative motion.
    if a < 1e-20 {
        return None;
    }

    let discriminant = b * b - 4.0 * a * c;
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }

    // Use the q-form of the quadratic roots to reduce cancellation when
    // |b| is large relative to the discriminant.
    let sqrt_discriminant = discriminant.sqrt();
    let q = -0.5 * (b + b.signum() * sqrt_discriminant);
    let (root_a, root_b) = if q.abs() > 1e-15 {
        (q / a, c / q)
    } else {
        let repeated_root = -b / (2.0 * a);
        (repeated_root, repeated_root)
    };

    let toi = [root_a, root_b]
        .into_iter()
        .filter(|time| time.is_finite() && *time >= 0.0 && *time <= dt)
        .min_by(f64::total_cmp)?;

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
    let point = pos_a_t + normal * radius_a;
    if !point.iter().all(|value| value.is_finite()) || !normal.iter().all(|value| value.is_finite())
    {
        return None;
    }

    Some(CcdHit { toi, point, normal })
}

/// Swept sphere vs half-space (infinite plane): find time of impact.
///
/// Solve for t where `normal · (pos + vel*t) - offset = radius`.
/// This is linear: `t = (radius + offset - normal · pos) / (normal · vel)`.
pub fn sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    if !radius.is_finite()
        || radius < 0.0
        || !plane_offset.is_finite()
        || !dt.is_finite()
        || dt < 0.0
        || !pos.iter().all(|value| value.is_finite())
        || !vel.iter().all(|value| value.is_finite())
        || !plane_normal.iter().all(|value| value.is_finite())
    {
        return None;
    }

    // A plane is represented by n · x = offset. Normalize both n and
    // offset so geometric results are invariant under positive scaling of
    // the plane equation (n, offset) -> (k*n, k*offset).
    let normal_length = plane_normal.norm();
    if !normal_length.is_finite() || normal_length <= 1e-15 {
        return None;
    }
    let unit_normal = plane_normal / normal_length;
    let signed_dist = (plane_normal.dot(pos) - plane_offset) / normal_length;
    if !signed_dist.is_finite() {
        return None;
    }

    // Already touching or penetrating.
    if signed_dist <= radius {
        let contact = pos - unit_normal * signed_dist;
        if !contact.iter().all(|value| value.is_finite()) {
            return None;
        }
        return Some(CcdHit {
            toi: 0.0,
            point: contact,
            normal: unit_normal,
        });
    }

    let vel_toward_plane = -unit_normal.dot(vel);
    if !vel_toward_plane.is_finite() || vel_toward_plane <= 0.0 {
        return None;
    }

    // Time to reach signed distance = radius from the plane.
    let toi = (signed_dist - radius) / vel_toward_plane;
    if !toi.is_finite() || toi < 0.0 || toi > dt {
        return None;
    }

    let pos_t = pos + vel * toi;
    let contact = pos_t - unit_normal * radius;
    if !contact.iter().all(|value| value.is_finite()) {
        return None;
    }

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
    fn sphere_halfspace_is_invariant_to_plane_normal_scaling() {
        let position = SVector::from([0.0, 5.0, 0.0]);
        let velocity = SVector::from([0.0, -500.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 10.0, 0.0]);

        let unit_hit =
            sphere_halfspace(&position, &velocity, 0.5, &unit_normal, 2.0, 0.01).unwrap();
        let scaled_hit =
            sphere_halfspace(&position, &velocity, 0.5, &scaled_normal, 20.0, 0.01).unwrap();

        assert!((unit_hit.toi - 0.005).abs() < 1e-12);
        assert!((scaled_hit.toi - unit_hit.toi).abs() < 1e-12);
        assert!((scaled_hit.point - unit_hit.point).norm() < 1e-12);
        assert!((scaled_hit.normal - unit_hit.normal).norm() < 1e-12);
        assert!((scaled_hit.point[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_halfspace_rejects_degenerate_or_non_finite_inputs() {
        let position = SVector::from([0.0, 5.0, 0.0]);
        let velocity = SVector::from([0.0, -5.0, 0.0]);
        let zero_normal = SVector::zeros();
        let valid_normal = SVector::from([0.0, 1.0, 0.0]);
        let invalid_velocity = SVector::from([0.0, f64::NAN, 0.0]);

        assert!(sphere_halfspace(&position, &velocity, 0.5, &zero_normal, 0.0, 1.0).is_none());
        assert!(
            sphere_halfspace(&position, &invalid_velocity, 0.5, &valid_normal, 0.0, 1.0).is_none()
        );
        assert!(sphere_halfspace(&position, &velocity, -0.5, &valid_normal, 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&position, &velocity, 0.5, &valid_normal, 0.0, -1.0).is_none());
    }

    #[test]
    fn sphere_sphere_rejects_non_finite_and_negative_inputs() {
        let position_a = SVector::from([0.0, 0.0, 0.0]);
        let velocity_a = SVector::from([10.0, 0.0, 0.0]);
        let position_b = SVector::from([5.0, 0.0, 0.0]);
        let velocity_b = SVector::from([-10.0, 0.0, 0.0]);
        let invalid_position = SVector::from([f64::INFINITY, 0.0, 0.0]);

        assert!(
            sphere_sphere(
                &invalid_position,
                &velocity_a,
                0.5,
                &position_b,
                &velocity_b,
                0.5,
                1.0
            )
            .is_none()
        );
        assert!(
            sphere_sphere(
                &position_a,
                &velocity_a,
                -0.5,
                &position_b,
                &velocity_b,
                0.5,
                1.0
            )
            .is_none()
        );
        assert!(
            sphere_sphere(
                &position_a,
                &velocity_a,
                0.5,
                &position_b,
                &velocity_b,
                0.5,
                f64::NAN
            )
            .is_none()
        );
    }

    #[test]
    fn sphere_sphere_head_on_toi_matches_analytic_solution() {
        let position_a = SVector::from([0.0, 0.0, 0.0]);
        let velocity_a = SVector::from([10.0, 0.0, 0.0]);
        let position_b = SVector::from([5.0, 0.0, 0.0]);
        let velocity_b = SVector::from([-10.0, 0.0, 0.0]);
        let radius_a = 0.5;
        let radius_b = 0.5;

        let hit = sphere_sphere(
            &position_a,
            &velocity_a,
            radius_a,
            &position_b,
            &velocity_b,
            radius_b,
            1.0,
        )
        .unwrap();

        let gap = 5.0 - radius_a - radius_b;
        let closing_speed = 20.0;
        let analytic_toi = gap / closing_speed;
        assert!((hit.toi - analytic_toi).abs() < 1e-12);
        assert!((hit.normal - SVector::from([1.0, 0.0, 0.0])).norm() < 1e-12);
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
