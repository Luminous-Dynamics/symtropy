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

/// Dimensionless travel-to-feature-size ratio above which CCD is considered.
/// A body is swept when relative travel over the step exceeds half the pair's
/// characteristic contact thickness. This is invariant under coherent unit changes.
pub const CCD_TRAVEL_FRACTION_THRESHOLD: f64 = 0.5;

/// Legacy absolute speed threshold retained for source compatibility only.
#[deprecated(note = "use CCD_TRAVEL_FRACTION_THRESHOLD; absolute speed is unit-dependent")]
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
        || dt <= 0.0
        || radius_a < 0.0
        || radius_b < 0.0
        || !radius_a.is_finite()
        || !radius_b.is_finite()
        || !pos_a.iter().all(|value| value.is_finite())
        || !pos_b.iter().all(|value| value.is_finite())
        || !vel_a.iter().all(|value| value.is_finite())
        || !vel_b.iter().all(|value| value.is_finite())
    {
        return None;
    }

    let dp = pos_b - pos_a;
    let displacement = (vel_b - vel_a) * dt;
    let distance = stable_norm(&dp);
    let travel = stable_norm(&displacement);
    let radius = radius_a + radius_b;
    let scale = distance
        .max(travel)
        .max(radius)
        .max(f64::MIN_POSITIVE);

    let normalized_position = dp / scale;
    let normalized_radius = radius / scale;

    // Already overlapping or exactly touching at t=0.
    let c = normalized_position.norm_squared() - normalized_radius * normalized_radius;
    if c <= 0.0 {
        let normal = if distance > 0.0 {
            dp / distance
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

    // Parameterize the relative segment by travelled distance rather than
    // physical time. The quadratic's leading coefficient is exactly one and
    // every coefficient is dimensionless:
    //   |dp/scale + direction * s|^2 = (r/scale)^2
    if travel <= 0.0 || !travel.is_finite() {
        return None;
    }
    let travel_direction = displacement / travel;
    let normalized_travel = travel / scale;
    if normalized_travel <= 0.0 || !normalized_travel.is_finite() {
        return None;
    }

    let linear = 2.0 * normalized_position.dot(&travel_direction);
    let discriminant = linear * linear - 4.0 * c;
    if discriminant < 0.0 || !discriminant.is_finite() {
        return None;
    }

    let sqrt_discriminant = discriminant.sqrt();
    // Stable quadratic roots for s^2 + linear*s + c = 0.
    let q = -0.5
        * (linear
            + if linear >= 0.0 {
                sqrt_discriminant
            } else {
                -sqrt_discriminant
            });
    let (root_a, root_b) = if q == 0.0 {
        (-0.5 * linear, -0.5 * linear)
    } else {
        (q, c / q)
    };
    let normalized_distance = [root_a, root_b]
        .into_iter()
        .filter(|root| *root >= 0.0 && *root <= normalized_travel)
        .min_by(f64::total_cmp)?;

    let toi_fraction = normalized_distance / normalized_travel;
    let toi = toi_fraction * dt;
    if !toi.is_finite() || !(0.0..=dt).contains(&toi) {
        return None;
    }

    let pos_a_t = pos_a + vel_a * toi;
    let pos_b_t = pos_b + vel_b * toi;
    let delta = pos_b_t - pos_a_t;
    let delta_length = stable_norm(&delta);
    let normal = if delta_length > 0.0 {
        delta / delta_length
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

/// Numerically stable Euclidean norm for vectors whose components may be
/// extremely small or large relative to ordinary world units.
fn stable_norm<const D: usize>(vector: &SVector<f64, D>) -> f64 {
    let maximum = vector
        .iter()
        .fold(0.0_f64, |current, value| current.max(value.abs()));
    if maximum == 0.0 {
        0.0
    } else {
        maximum * (vector / maximum).norm()
    }
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
    if !dt.is_finite()
        || dt <= 0.0
        || radius < 0.0
        || !radius.is_finite()
        || !plane_offset.is_finite()
        || !pos.iter().all(|value| value.is_finite())
        || !vel.iter().all(|value| value.is_finite())
        || !plane_normal.iter().all(|value| value.is_finite())
    {
        return None;
    }

    let signed_dist = plane_normal.dot(pos) - plane_offset;

    // Already penetrating.
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
        return None;
    }

    // Compute a dimensionless fraction of the step. This avoids treating
    // physically equivalent metre/millisecond and millimetre/microsecond
    // trajectories differently through an absolute time or speed epsilon.
    let travel_toward_plane = vel_toward_plane * dt;
    if !travel_toward_plane.is_finite() || travel_toward_plane <= 0.0 {
        return None;
    }
    let scale = signed_dist
        .abs()
        .max(radius.abs())
        .max(travel_toward_plane.abs())
        .max(f64::MIN_POSITIVE);
    let normalized_gap = signed_dist / scale - radius / scale;
    let normalized_travel = travel_toward_plane / scale;
    if normalized_travel <= 0.0 {
        return None;
    }
    let toi_fraction = normalized_gap / normalized_travel;
    if !(0.0..=1.0).contains(&toi_fraction) {
        return None;
    }

    let toi = toi_fraction * dt;
    let pos_t = pos + vel * toi;
    let contact = pos_t - plane_normal * radius;
    Some(CcdHit {
        toi,
        point: contact,
        normal: *plane_normal,
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
}
