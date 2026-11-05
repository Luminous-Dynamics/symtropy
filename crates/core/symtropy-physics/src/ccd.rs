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
    /// Time of impact in seconds, within the current step's [0, dt] interval.
    /// 0 = start of step; dt = end of step.
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Contact normal at time of impact.
    pub normal: SVector<f64, D>,
}

/// Speed threshold below which CCD is not applied (optimization).
/// Bodies moving slower than this per step won't tunnel through typical geometry.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

#[inline]
fn is_finite_vector<const D: usize>(value: &SVector<f64, D>) -> bool {
    value.iter().all(|component| component.is_finite())
}

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
    if !is_finite_vector(pos_a)
        || !is_finite_vector(vel_a)
        || !is_finite_vector(pos_b)
        || !is_finite_vector(vel_b)
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
    let r = radius_a + radius_b;
    if !is_finite_vector(&dp) || !is_finite_vector(&dv) || !r.is_finite() {
        return None;
    }

    let a = dv.dot(&dv);
    let b = 2.0 * dp.dot(&dv);
    let c = dp.dot(&dp) - r * r;
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return None;
    }

    // Already overlapping at t=0
    if c <= 0.0 {
        let normal = if dp.norm() > 1e-15 {
            dp.normalize()
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

    // No relative motion
    if a < 1e-20 {
        return None;
    }

    let discriminant = b.mul_add(b, -4.0 * a * c);
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None; // No intersection or numerically invalid query.
    }

    let sqrt_disc = discriminant.sqrt();
    // Use the q-form of the quadratic roots to avoid catastrophic cancellation
    // in (-b ± sqrt(discriminant)) when the roots have very different magnitudes.
    let q = -0.5 * (b + sqrt_disc.copysign(b));
    if q == 0.0 || !q.is_finite() {
        return None;
    }
    let mut roots = [q / a, c / q];
    roots.sort_by(f64::total_cmp);

    // We want the earliest root within the closed [0, dt] interval.
    for toi in roots {
        if toi < 0.0 || toi > dt {
            continue;
        }
        let pos_a_t = pos_a + vel_a * toi;
        let pos_b_t = pos_b + vel_b * toi;
        let delta = pos_b_t - pos_a_t;
        let delta_norm = delta.norm();
        let normal = if delta_norm.is_finite() && delta_norm > 1e-15 {
            delta / delta_norm
        } else {
            let mut n = SVector::zeros();
            n[0] = 1.0;
            n
        };
        return Some(CcdHit {
            toi,
            point: pos_a_t + normal * radius_a,
            normal,
        });
    }
    None // Intersection happens outside [0, dt]
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
    if !is_finite_vector(pos)
        || !is_finite_vector(vel)
        || !is_finite_vector(plane_normal)
        || !radius.is_finite()
        || radius < 0.0
        || !plane_offset.is_finite()
        || !dt.is_finite()
        || dt < 0.0
    {
        return None;
    }

    // The plane is represented by n · x = offset. Normalize both sides of
    // that equation so the geometry is invariant to positive scaling of n.
    let normal_length = plane_normal.norm();
    if !normal_length.is_finite() || normal_length <= 1e-15 {
        return None;
    }
    let unit_normal = plane_normal / normal_length;
    let normalized_offset = plane_offset / normal_length;
    let signed_dist = unit_normal.dot(pos) - normalized_offset;
    if !signed_dist.is_finite() {
        return None;
    }

    // Already penetrating
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

    // Time to reach distance = radius from plane
    let t = (signed_dist - radius) / vel_toward_plane;

    if t >= 0.0 && t <= dt {
        let pos_t = pos + vel * t;
        let contact = pos_t - unit_normal * radius;
        Some(CcdHit {
            toi: t,
            point: contact,
            normal: unit_normal,
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
    fn sphere_sphere_grazing_contact_has_analytic_toi() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 1.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0).unwrap();

        assert!((hit.toi - 0.5).abs() < 1e-12);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        assert!((hit.normal[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_contact_at_exact_step_end_is_included() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([2.0, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0).unwrap();
        assert!((hit.toi - 1.0).abs() < 1e-12);
        assert!(sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0 - 1e-9).is_none());
    }

    #[test]
    fn plane_normal_scale_does_not_change_halfspace_toi_or_contact() {
        let pos = SVector::from([0.0, 3.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let unit = SVector::from([0.0, 1.0, 0.0]);
        let scaled = SVector::from([0.0, 2.0, 0.0]);

        let a = sphere_halfspace(&pos, &velocity, 0.5, &unit, 0.0, 3.0).unwrap();
        let b = sphere_halfspace(&pos, &velocity, 0.5, &scaled, 0.0, 3.0).unwrap();

        assert!((a.toi - 2.5).abs() < 1e-12);
        assert!((a.toi - b.toi).abs() < 1e-12);
        assert!((a.point - b.point).norm() < 1e-12);
        assert!((a.point[1]).abs() < 1e-12);
        assert!((b.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_halfspace_geometry_and_sphere_parameters_fail_closed() {
        let pos = SVector::from([0.0, 3.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let zero_normal = SVector::zeros();
        let normal = SVector::from([0.0, 1.0, 0.0]);

        assert!(sphere_halfspace(&pos, &velocity, 0.5, &zero_normal, 0.0, 3.0).is_none());
        assert!(sphere_halfspace(&pos, &velocity, 0.5, &normal, f64::NAN, 3.0).is_none());
        assert!(sphere_halfspace(&pos, &velocity, -0.5, &normal, 0.0, 3.0).is_none());
        assert!(sphere_halfspace(&pos, &velocity, 0.5, &normal, 0.0, f64::INFINITY).is_none());
        assert!(sphere_sphere(&pos, &velocity, -0.5, &pos, &velocity, 0.5, 3.0).is_none());
        assert!(sphere_sphere(&pos, &velocity, 0.5, &pos, &velocity, 0.5, f64::NAN).is_none());
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
