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
    /// Time of impact in the same time units as the supplied timestep.
    /// The query returns a value in the closed interval [0, dt].
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Contact normal at time of impact.
    pub normal: SVector<f64, D>,
}

/// True when every component is finite.
#[inline]
fn vector_is_finite<const D: usize>(value: &SVector<f64, D>) -> bool {
    value.iter().all(|component| component.is_finite())
}

/// Compute a finite vector norm without squaring unscaled large components.
fn stable_vector_norm<const D: usize>(value: &SVector<f64, D>) -> Option<f64> {
    if !vector_is_finite(value) {
        return None;
    }
    let scale = value.amax();
    if scale == 0.0 {
        return Some(0.0);
    }
    if !scale.is_finite() {
        return None;
    }
    let scaled_norm = (value / scale).norm();
    let norm = scale * scaled_norm;
    norm.is_finite().then_some(norm)
}

/// Normalize a finite vector without overflowing on large magnitudes.
fn stable_unit_vector<const D: usize>(value: &SVector<f64, D>) -> Option<SVector<f64, D>> {
    if !vector_is_finite(value) {
        return None;
    }
    let scale = value.amax();
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let scaled = value / scale;
    let norm = scaled.norm();
    if norm == 0.0 || !norm.is_finite() {
        return None;
    }
    Some(scaled / norm)
}

/// Deterministic fallback normal for coincident centers.
fn axis_fallback<const D: usize>() -> SVector<f64, D> {
    let mut normal = SVector::zeros();
    normal[0] = 1.0;
    normal
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
    if !vector_is_finite(pos_a)
        || !vector_is_finite(vel_a)
        || !vector_is_finite(pos_b)
        || !vector_is_finite(vel_b)
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
    if !vector_is_finite(&dp) || !vector_is_finite(&dv) || !r.is_finite() {
        return None;
    }

    let distance = stable_vector_norm(&dp)?;
    let a = dv.dot(&dv);
    let b = 2.0 * dp.dot(&dv);
    // Factoring the difference of squares avoids losing the small gap when
    // center distance and combined radius are nearly equal.
    let c = (distance - r) * (distance + r);
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return None;
    }

    // Already overlapping or touching at t=0.
    if distance <= r {
        let normal = stable_unit_vector(&dp).unwrap_or_else(axis_fallback);
        return Some(CcdHit {
            toi: 0.0,
            point: pos_a + normal * radius_a,
            normal,
        });
    }

    // Only exactly zero computed relative speed is stationary. Avoid an
    // absolute cutoff that would discard slow but valid long-horizon hits.
    if a == 0.0 {
        return None;
    }

    let discriminant = b * b - 4.0 * a * c;
    if !discriminant.is_finite() || discriminant < 0.0 {
        return None;
    }

    let sqrt_discriminant = discriminant.sqrt();

    // Stable quadratic q-form. The conventional (-b ± sqrt(D)) / (2a)
    // loses the small root when b and sqrt(D) are nearly equal.
    let q = -0.5 * (b + sqrt_discriminant.copysign(b));
    let (root_a, root_b) = if q == 0.0 {
        let repeated_root = -b / (2.0 * a);
        (repeated_root, repeated_root)
    } else {
        (q / a, c / q)
    };

    let toi = [root_a, root_b]
        .into_iter()
        .filter(|root| root.is_finite() && *root >= 0.0 && *root <= dt)
        .min_by(f64::total_cmp)?;

    let pos_a_t = pos_a + vel_a * toi;
    let pos_b_t = pos_b + vel_b * toi;
    let delta = pos_b_t - pos_a_t;
    if !vector_is_finite(&pos_a_t) || !vector_is_finite(&pos_b_t) || !vector_is_finite(&delta) {
        return None;
    }
    let normal = stable_unit_vector(&delta).unwrap_or_else(axis_fallback);

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
    if !vector_is_finite(pos)
        || !vector_is_finite(vel)
        || !vector_is_finite(plane_normal)
        || !plane_offset.is_finite()
        || !radius.is_finite()
        || radius < 0.0
        || !dt.is_finite()
        || dt < 0.0
    {
        return None;
    }

    // Normalize the normal and offset together so equivalent representations
    // k*n · x = k*d produce identical TOI and a unit output normal.
    let normal_scale = plane_normal.amax();
    if !normal_scale.is_finite() || normal_scale == 0.0 {
        return None;
    }
    let scaled_normal = plane_normal / normal_scale;
    let scaled_norm = scaled_normal.norm();
    if !scaled_norm.is_finite() || scaled_norm == 0.0 {
        return None;
    }
    let normal = scaled_normal / scaled_norm;
    let offset = (plane_offset / normal_scale) / scaled_norm;
    if !offset.is_finite() {
        return None;
    }

    let signed_dist = normal.dot(pos) - offset;
    if !signed_dist.is_finite() {
        return None;
    }

    // Already penetrating or touching at t=0.
    if signed_dist <= radius {
        let contact = pos - normal * signed_dist;
        return Some(CcdHit {
            toi: 0.0,
            point: contact,
            normal,
        });
    }

    let vel_toward_plane = -normal.dot(vel);
    if !vel_toward_plane.is_finite() || vel_toward_plane <= 0.0 {
        return None;
    }

    let t = (signed_dist - radius) / vel_toward_plane;
    if !t.is_finite() || t < 0.0 || t > dt {
        return None;
    }

    let pos_t = pos + vel * t;
    if !vector_is_finite(&pos_t) {
        return None;
    }
    let contact = pos_t - normal * radius;
    Some(CcdHit {
        toi: t,
        point: contact,
        normal,
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
    #[test]
    fn sphere_sphere_stable_q_form_preserves_small_root() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([5.0, 0.0]);
        let vel_b = SVector::from([-1.0, 0.0]);
        let radius_a = 5.0 - 1e-14;

        let expected = pos_b[0] - radius_a;
        let hit = sphere_sphere(
            &pos_a, &vel_a, radius_a, &pos_b, &vel_b, 0.0, 1e-12,
        )
        .unwrap();

        assert!(hit.toi > 0.0);
        assert!((hit.toi - expected).abs() <= expected * 1e-6);
    }

    #[test]
    fn sphere_sphere_does_not_discard_low_speed_long_horizon_hits() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([1.5, 0.0]);
        let vel_b = SVector::from([-1e-11, 0.0]);

        let hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1e11)
            .expect("low-speed collision is within the long horizon");
        assert!((hit.toi - 5e10).abs() <= 1e-4);
    }

    #[test]
    fn sphere_sphere_accepts_exact_end_of_interval_and_tangent_hits() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([2.0, 0.0]);
        let vel_b = SVector::from([-1.0, 0.0]);
        let end_hit = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .expect("contact at exactly dt is included");
        assert!((end_hit.toi - 1.0).abs() < 1e-12);

        let tangent_pos = SVector::from([2.0, 1.0]);
        let tangent = sphere_sphere(
            &pos_a,
            &vel_a,
            0.5,
            &tangent_pos,
            &vel_b,
            0.5,
            2.0,
        )
        .expect("grazing contact is a hit");
        assert!((tangent.toi - 2.0).abs() < 1e-10);

        let near_miss_pos = SVector::from([2.0, 1.0 + 1e-8]);
        assert!(
            sphere_sphere(
                &pos_a,
                &vel_a,
                0.5,
                &near_miss_pos,
                &vel_b,
                0.5,
                2.0
            )
            .is_none(),
            "a separated near-tangent trajectory must remain a miss"
        );
    }

    #[test]
    fn sphere_sphere_reversing_body_order_preserves_toi_and_reverses_normal() {
        let pos_a = SVector::from([-2.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([2.0, 0.5, 0.0]);
        let vel_b = SVector::from([-1.0, 0.0, 0.0]);

        let forward = sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 2.0).unwrap();
        let reverse = sphere_sphere(&pos_b, &vel_b, 0.5, &pos_a, &vel_a, 0.5, 2.0).unwrap();

        assert!((forward.toi - reverse.toi).abs() < 1e-12);
        assert!((forward.normal + reverse.normal).norm() < 1e-12);
    }

    #[test]
    fn sphere_sphere_rejects_invalid_query_inputs() {
        let pos = SVector::from([0.0, 0.0, 0.0]);
        let velocity = SVector::zeros();

        assert!(sphere_sphere(&pos, &velocity, -1.0, &pos, &velocity, 1.0, 1.0).is_none());
        assert!(sphere_sphere(&pos, &velocity, f64::NAN, &pos, &velocity, 1.0, 1.0).is_none());
        assert!(sphere_sphere(&pos, &velocity, 1.0, &pos, &velocity, 1.0, f64::INFINITY).is_none());
        assert!(sphere_sphere(&pos, &velocity, 1.0, &pos, &velocity, 1.0, -1.0).is_none());

        let invalid_pos = SVector::from([f64::NAN, 0.0, 0.0]);
        assert!(sphere_sphere(&invalid_pos, &velocity, 1.0, &pos, &velocity, 1.0, 1.0).is_none());

        let invalid_velocity = SVector::from([0.0, f64::INFINITY, 0.0]);
        assert!(sphere_sphere(&pos, &velocity, 1.0, &pos, &invalid_velocity, 1.0, 1.0).is_none());
    }

    #[test]
    fn sphere_halfspace_is_invariant_to_normal_and_offset_scaling() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let reference_normal = SVector::from([0.0, 1.0, 0.0]);

        let reference = sphere_halfspace(&pos, &vel, 0.5, &reference_normal, 0.0, 1.0).unwrap();
        for scale in [1e-6, 1.0, 1e6] {
            let scaled_normal = reference_normal * scale;
            let hit = sphere_halfspace(&pos, &vel, 0.5, &scaled_normal, 0.0, 1.0).unwrap();
            assert!((hit.toi - reference.toi).abs() < 1e-12);
            assert!((hit.normal - reference.normal).norm() < 1e-12);
            assert!((hit.point - reference.point).norm() < 1e-12);
            assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        }

        // Non-zero plane offset must scale together with the normal too.
        let shifted_pos = SVector::from([0.0, 12.0, 0.0]);
        let offset_reference = sphere_halfspace(
            &shifted_pos,
            &vel,
            0.5,
            &reference_normal,
            2.0,
            1.0,
        )
        .unwrap();
        let scaled = sphere_halfspace(
            &shifted_pos,
            &vel,
            0.5,
            &(reference_normal * 1e6),
            2e6,
            1.0,
        )
        .unwrap();
        assert!((scaled.toi - offset_reference.toi).abs() < 1e-12);
    }

    #[test]
    fn sphere_halfspace_rejects_invalid_inputs_and_returns_unit_normal() {
        let pos = SVector::from([0.0, 2.0, 0.0]);
        let vel = SVector::from([0.0, -2.0, 0.0]);
        let normal = SVector::from([0.0, 5.0, 0.0]);

        let hit = sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, 1.0).unwrap();
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);

        assert!(sphere_halfspace(&pos, &vel, -1.0, &normal, 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&pos, &vel, 0.5, &SVector::zeros(), 0.0, 1.0).is_none());
        assert!(sphere_halfspace(&pos, &vel, 0.5, &normal, f64::NAN, 1.0).is_none());
        assert!(sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, f64::NAN).is_none());
        assert!(sphere_halfspace(&pos, &vel, 0.5, &normal, 0.0, f64::INFINITY).is_none());

        let invalid_pos = SVector::from([0.0, f64::INFINITY, 0.0]);
        assert!(sphere_halfspace(&invalid_pos, &vel, 0.5, &normal, 0.0, 1.0).is_none());

        let invalid_normal = SVector::from([0.0, f64::NAN, 0.0]);
        assert!(sphere_halfspace(&pos, &vel, 0.5, &invalid_normal, 0.0, 1.0).is_none());
    }

}
