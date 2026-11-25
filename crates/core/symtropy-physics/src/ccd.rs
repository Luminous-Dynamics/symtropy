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
    /// Elapsed simulation time in seconds from the start of the timestep.
    /// The value is in [0, dt], not normalized to [0, 1].
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
    if !dt.is_finite()
        || dt < 0.0
        || !radius_a.is_finite()
        || !radius_b.is_finite()
        || radius_a < 0.0
        || radius_b < 0.0
        || pos_a.iter().chain(pos_b.iter()).any(|value| !value.is_finite())
        || vel_a.iter().chain(vel_b.iter()).any(|value| !value.is_finite())
    {
        return None;
    }

    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    let relative_displacement = dv * dt;
    let combined_radius = radius_a + radius_b;
    if !combined_radius.is_finite()
        || relative_displacement.iter().any(|value| !value.is_finite())
    {
        return None;
    }

    // Already overlapping at t=0.
    // Normalize before the initial-overlap test as well; the raw squared
    // distances can overflow even when every input component is finite.
    let mut initial_scale = combined_radius;
    for axis in 0..D {
        initial_scale = initial_scale.max(dp[axis].abs());
    }
    if !initial_scale.is_finite() {
        return None;
    }
    if initial_scale == 0.0 {
        // Coincident zero-radius spheres are already in contact at t=0.
        let mut normal = SVector::zeros();
        if D == 0 {
            return None;
        }
        normal[0] = 1.0;
        return Some(CcdHit {
            toi: 0.0,
            point: *pos_a,
            normal,
        });
    }
    let dp_initial_scaled = dp / initial_scale;
    let radius_initial_scaled = combined_radius / initial_scale;
    let c_initial = dp_initial_scaled.dot(&dp_initial_scaled)
        - radius_initial_scaled * radius_initial_scaled;
    if c_initial <= 0.0 {
        let normal = if dp_initial_scaled.norm() > 1e-15 {
            dp_initial_scaled.normalize()
        } else {
            let mut n = SVector::zeros();
            n[0] = 1.0;
            n
        };
        return Some(CcdHit {
            toi: 0.0,
            point: pos_a + normal * radius_a,
            normal,
        });
    }

    // Solve in normalized step time u ∈ [0, 1]:
    // |dp + (dv * dt) * u|² = (ra + rb)².
    //
    // This avoids an absolute velocity-squared cutoff, which incorrectly
    // rejects small relative velocities over long timesteps.
    // Normalize all length-valued terms before forming quadratic
    // coefficients. This keeps a, b, c and the discriminant bounded even when
    // finite world coordinates or velocities are very large.
    let mut length_scale = combined_radius;
    for axis in 0..D {
        length_scale = length_scale
            .max(dp[axis].abs())
            .max(relative_displacement[axis].abs());
    }
    if length_scale == 0.0 || !length_scale.is_finite() {
        return None;
    }
    let dp_scaled = dp / length_scale;
    let displacement_scaled = relative_displacement / length_scale;
    let radius_scaled = combined_radius / length_scale;
    let a = displacement_scaled.dot(&displacement_scaled);
    let b = 2.0 * dp_scaled.dot(&displacement_scaled);
    let c = dp_scaled.dot(&dp_scaled) - radius_scaled * radius_scaled;
    if a == 0.0 || !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return None;
    }

    // One fused multiply-add reduces one rounding in the discriminant.
    let discriminant = b.mul_add(b, -4.0 * a * c);
    if !discriminant.is_finite() {
        return None;
    }
    // Tangency is the numerically delicate boundary. If a tiny negative
    // discriminant is within a conservative floating-point error envelope,
    // retain the tangent as a candidate instead of turning rounding noise
    // into a false negative. This is a numerical robustness filter, not a
    // formal exact-arithmetic certificate.
    let discriminant_scale = b.abs() * b.abs() + (4.0 * a * c).abs();
    let discriminant_error = 8.0 * f64::EPSILON * discriminant_scale;
    let discriminant = if discriminant < 0.0 && -discriminant <= discriminant_error {
        0.0
    } else if discriminant < 0.0 {
        return None;
    } else {
        discriminant
    };

    let sqrt_discriminant = discriminant.sqrt();
    // Cancellation-resistant quadratic formula. q/a and c/q are the two
    // roots; unlike (-b ± sqrt(D))/(2a), the smaller-magnitude root is not
    // obtained by subtracting nearly equal values.
    let q = -0.5 * (b + sqrt_discriminant.copysign(b));
    let (root_a, root_b) = if q == 0.0 {
        let repeated = -b / (2.0 * a);
        (repeated, repeated)
    } else {
        (q / a, c / q)
    };
    if !root_a.is_finite() || !root_b.is_finite() {
        return None;
    }
    let (first, second) = if root_a <= root_b {
        (root_a, root_b)
    } else {
        (root_b, root_a)
    };

    let toi_fraction = if (0.0..=1.0).contains(&first) {
        first
    } else if (0.0..=1.0).contains(&second) {
        second
    } else {
        return None;
    };
    let toi = toi_fraction * dt;
    if !toi.is_finite() {
        return None;
    }

    let pos_a_t = pos_a + vel_a * toi;
    let pos_b_t = pos_b + vel_b * toi;
    let delta = pos_b_t - pos_a_t;
    let normal = if delta.norm() > 1e-15 {
        delta.normalize()
    } else {
        let mut n = SVector::zeros();
        n[0] = 1.0;
        n
    };
    Some(CcdHit {
        toi,
        point: pos_a_t + normal * radius_a,
        normal,
    })
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
    let signed_dist = plane_normal.dot(pos) - plane_offset;

    // Already penetrating
    if signed_dist <= radius {
        let contact = pos - plane_normal * signed_dist;
        return Some(CcdHit {
            toi: 0.0,
            point: contact,
            normal: *plane_normal,
        });
    }

    let vel_toward_plane = -plane_normal.dot(vel);
    if vel_toward_plane <= 0.0 {
        return None; // Moving away from plane
    }

    // Time to reach distance = radius from plane
    let t = (signed_dist - radius) / vel_toward_plane;

    if t >= 0.0 && t <= dt {
        let pos_t = pos + vel * t;
        let contact = pos_t - plane_normal * radius;
        Some(CcdHit {
            toi: t,
            point: contact,
            normal: *plane_normal,
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
    fn coincident_zero_radius_spheres_are_initial_contact() {
        let position = SVector::from([2.0, -1.0, 0.5]);
        let velocity = SVector::zeros();
        let hit = sphere_sphere(
            &position,
            &velocity,
            0.0,
            &position,
            &velocity,
            0.0,
            1.0,
        )
        .unwrap();
        assert_eq!(hit.toi, 0.0);
        assert_eq!(hit.point, position);
        assert!(hit.normal[0] > 0.99);
    }

    #[test]
    fn sphere_sphere_small_velocity_large_dt_is_not_pruned_by_absolute_threshold() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([2.0, 0.0, 0.0]);
        let vel_b = SVector::from([-1.0e-11, 0.0, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 2.0e11).unwrap();
        assert!((hit.toi - 1.0e11).abs() <= 1.0e-3);
    }

    #[test]
    fn sphere_sphere_exact_tangent_is_retained_as_contact() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([-1.0, 2.0, 0.0]);
        let vel_b = SVector::from([2.0, 0.0, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 1.0, &pos_b, &vel_b, 1.0, 1.0).unwrap();
        assert!((hit.toi - 0.5).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_near_start_contact_uses_stable_small_root() {
        let gap = 1.0e-12;
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([1.0 + gap, 0.0, 0.0]);
        let vel_b = SVector::from([-1.0, 0.0, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0).unwrap();
        assert!((hit.toi - gap).abs() <= 2.0e-16);
    }

    #[test]
    fn sphere_sphere_large_finite_scale_does_not_overflow_discriminant() {
        let scale = 1.0e150;
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0 * scale, 0.0, 0.0]);
        let pos_b = SVector::from([5.0 * scale, 0.0, 0.0]);
        let vel_b = SVector::from([-10.0 * scale, 0.0, 0.0]);

        let hit = sphere_sphere(
            &pos_a,
            &vel_a,
            0.5 * scale,
            &pos_b,
            &vel_b,
            0.5 * scale,
            1.0,
        )
        .unwrap();
        assert!((hit.toi - 0.2).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_returns_elapsed_seconds_for_normalized_internal_root() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::from([-1.0, 0.0, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 3.0).unwrap();
        assert!((hit.toi - 2.0).abs() < 1e-12);
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
