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
#[derive(Clone, Debug, PartialEq)]
pub struct CcdHit<const D: usize> {
    /// Elapsed physical time from the beginning of the step to impact, in seconds.
    ///
    /// This is in [0, dt], not a normalized [0, 1] fraction. Use
    /// `normalized_toi(dt)` when a dimensionless step fraction is required.
    /// The world integrator uses this value directly in position + velocity * toi.
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Contact normal at time of impact.
    pub normal: SVector<f64, D>,
}

impl<const D: usize> CcdHit<D> {
    /// Return the elapsed time of impact in seconds.
    pub fn time_of_impact_seconds(&self) -> f64 {
        self.toi
    }

    /// Convert elapsed time of impact into a dimensionless fraction of the step.
    ///
    /// Returns None for non-finite/non-positive timesteps or a TOI outside
    /// [0, dt]. A zero-duration step has no meaningful normalized fraction.
    pub fn normalized_toi(&self, dt_seconds: f64) -> Option<f64> {
        if !dt_seconds.is_finite()
            || dt_seconds <= 0.0
            || !self.toi.is_finite()
            || self.toi < 0.0
            || self.toi > dt_seconds
        {
            return None;
        }
        Some(self.toi / dt_seconds)
    }
}

/// Invalid or numerically unusable continuous-collision query input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdQueryError {
    /// One or more position, velocity, plane, or scalar inputs is not finite.
    NonFiniteInput,
    /// A timestep is negative or not finite.
    InvalidTimeStep,
    /// A sphere radius is non-positive or not finite.
    InvalidRadius,
    /// A plane normal is not a finite unit vector.
    NonUnitPlaneNormal,
    /// The analytical calculation produced a non-finite or out-of-step result.
    InvalidResult,
}

fn validate_ccd_timestep(dt_seconds: f64) -> Result<(), CcdQueryError> {
    if !dt_seconds.is_finite() || dt_seconds < 0.0 {
        return Err(CcdQueryError::InvalidTimeStep);
    }
    Ok(())
}

fn validate_ccd_radius(radius: f64) -> Result<(), CcdQueryError> {
    if !radius.is_finite() || radius <= 0.0 {
        return Err(CcdQueryError::InvalidRadius);
    }
    Ok(())
}

fn validate_ccd_vector<const D: usize>(vector: &SVector<f64, D>) -> Result<(), CcdQueryError> {
    if vector.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(CcdQueryError::NonFiniteInput)
    }
}

fn validate_ccd_hit<const D: usize>(hit: &CcdHit<D>, dt_seconds: f64) -> Result<(), CcdQueryError> {
    if !hit.toi.is_finite()
        || hit.toi < 0.0
        || hit.toi > dt_seconds
        || hit.point.iter().any(|value| !value.is_finite())
        || hit.normal.iter().any(|value| !value.is_finite())
    {
        return Err(CcdQueryError::InvalidResult);
    }
    Ok(())
}

/// Speed threshold below which CCD is not applied (optimization).
/// Bodies moving slower than this per step won't tunnel through typical geometry.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

/// Checked swept sphere-vs-sphere query.
///
/// Given two spheres with positions and velocities, solve the quadratic
/// `|Δp + Δv*t|² = r²` for earliest impact during `[0, dt_seconds]`.
/// `dt_seconds` and the returned TOI are elapsed physical seconds. Radii must
/// be finite and strictly positive; position and velocity vectors must be finite.
pub fn sphere_sphere_checked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt_seconds: f64,
) -> Result<Option<CcdHit<D>>, CcdQueryError> {
    validate_ccd_timestep(dt_seconds)?;
    validate_ccd_radius(radius_a)?;
    validate_ccd_radius(radius_b)?;
    validate_ccd_vector(pos_a)?;
    validate_ccd_vector(vel_a)?;
    validate_ccd_vector(pos_b)?;
    validate_ccd_vector(vel_b)?;

    let relative_position = pos_b - pos_a;
    let relative_velocity = vel_b - vel_a;
    let combined_radius = radius_a + radius_b;
    if !relative_position.norm_squared().is_finite()
        || !relative_velocity.norm_squared().is_finite()
        || !combined_radius.is_finite()
        || !(combined_radius * combined_radius).is_finite()
    {
        return Err(CcdQueryError::NonFiniteInput);
    }

    let hit = sphere_sphere_unchecked(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt_seconds);
    if let Some(hit) = &hit {
        validate_ccd_hit(hit, dt_seconds)?;
    }
    Ok(hit)
}

/// Compatibility wrapper for the checked sphere-vs-sphere query.
///
/// Invalid inputs fail closed as `None`. New callers that need to distinguish
/// invalid inputs from a geometrically valid miss should use
/// [`sphere_sphere_checked`].
pub fn sphere_sphere<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt_seconds: f64,
) -> Option<CcdHit<D>> {
    sphere_sphere_checked(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt_seconds)
        .ok()
        .flatten()
}

fn sphere_sphere_unchecked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    let r = radius_a + radius_b;

    let a = dv.dot(&dv);
    let b = 2.0 * dp.dot(&dv);
    let c = dp.dot(&dp) - r * r;

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

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None; // No intersection
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);

    // We want the earliest positive t within [0, dt]
    if t1 >= 0.0 && t1 <= dt {
        let pos_a_t = pos_a + vel_a * t1;
        let pos_b_t = pos_b + vel_b * t1;
        let delta = pos_b_t - pos_a_t;
        let normal = if delta.norm() > 1e-15 {
            delta.normalize()
        } else {
            let mut n = SVector::zeros();
            n[0] = 1.0;
            n
        };
        Some(CcdHit {
            toi: t1,
            point: pos_a_t + normal * radius_a,
            normal,
        })
    } else {
        let t2 = (-b + sqrt_disc) / (2.0 * a);
        if t2 >= 0.0 && t2 <= dt {
            let pos_a_t = pos_a + vel_a * t2;
            let pos_b_t = pos_b + vel_b * t2;
            let delta = pos_b_t - pos_a_t;
            let normal = if delta.norm() > 1e-15 {
                delta.normalize()
            } else {
                let mut n = SVector::zeros();
                n[0] = 1.0;
                n
            };
            Some(CcdHit {
                toi: t2,
                point: pos_a_t + normal * radius_a,
                normal,
            })
        } else {
            None // Intersection happens outside [0, dt]
        }
    }
}

/// Checked swept sphere-vs-half-space query.
///
/// The plane equation is `unit_normal · x = offset`. The normal must be a
/// finite unit vector. The linear solve returns elapsed physical seconds for
/// `unit_normal · (pos + vel*t) - offset = radius`.
pub fn sphere_halfspace_checked<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt_seconds: f64,
) -> Result<Option<CcdHit<D>>, CcdQueryError> {
    validate_ccd_timestep(dt_seconds)?;
    validate_ccd_radius(radius)?;
    validate_ccd_vector(pos)?;
    validate_ccd_vector(vel)?;
    validate_ccd_vector(plane_normal)?;
    if !plane_offset.is_finite() {
        return Err(CcdQueryError::NonFiniteInput);
    }

    let normal_norm = plane_normal.norm();
    if !normal_norm.is_finite() || (normal_norm - 1.0).abs() > 1e-9 {
        return Err(CcdQueryError::NonUnitPlaneNormal);
    }
    if !plane_normal.dot(pos).is_finite() || !plane_normal.dot(vel).is_finite() {
        return Err(CcdQueryError::NonFiniteInput);
    }

    let hit = sphere_halfspace_unchecked(pos, vel, radius, plane_normal, plane_offset, dt_seconds);
    if let Some(hit) = &hit {
        validate_ccd_hit(hit, dt_seconds)?;
    }
    Ok(hit)
}

/// Compatibility wrapper for the checked sphere-vs-half-space query.
///
/// Invalid inputs fail closed as `None`. New callers that need to distinguish
/// invalid inputs from a geometrically valid miss should use
/// [`sphere_halfspace_checked`].
pub fn sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt_seconds: f64,
) -> Option<CcdHit<D>> {
    sphere_halfspace_checked(pos, vel, radius, plane_normal, plane_offset, dt_seconds)
        .ok()
        .flatten()
}

fn sphere_halfspace_unchecked<const D: usize>(
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
    fn toi_is_elapsed_seconds_and_normalizes_only_on_request() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -1_000.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let hit = sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, 0.02)
            .unwrap()
            .unwrap();

        assert!((hit.time_of_impact_seconds() - 0.0095).abs() < 1e-12);
        assert!((hit.normalized_toi(0.02).unwrap() - 0.475).abs() < 1e-12);
        assert!(hit.normalized_toi(0.0).is_none());
        assert!(hit.normalized_toi(f64::NAN).is_none());
    }

    #[test]
    fn sphere_sphere_toi_uses_physical_time_for_non_unit_step() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::from([-10.0, 0.0, 0.0]);

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 0.25)
            .unwrap()
            .unwrap();
        assert!((hit.toi - 0.2).abs() < 1e-12);
        assert!((hit.normalized_toi(0.25).unwrap() - 0.8).abs() < 1e-12);
    }

    #[test]
    fn checked_queries_reject_invalid_time_radii_and_non_finite_vectors() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        assert_eq!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, -0.1),
            Err(CcdQueryError::InvalidTimeStep)
        );
        assert_eq!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.0, &pos_b, &vel_b, 0.5, 1.0),
            Err(CcdQueryError::InvalidRadius)
        );
        let non_finite = SVector::from([f64::NAN, 0.0, 0.0]);
        assert_eq!(
            sphere_sphere_checked(&non_finite, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0),
            Err(CcdQueryError::NonFiniteInput)
        );
        assert_eq!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, f64::INFINITY),
            Err(CcdQueryError::InvalidTimeStep)
        );
    }

    #[test]
    fn checked_queries_reject_overflowing_combined_radii() {
        let pos_a = SVector::zeros();
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([10.0, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        assert_eq!(
            sphere_sphere_checked(&pos_a, &vel_a, f64::MAX, &pos_b, &vel_b, f64::MAX, 1.0),
            Err(CcdQueryError::NonFiniteInput)
        );
    }

    #[test]
    fn checked_halfspace_requires_a_finite_unit_normal() {
        let pos = SVector::from([0.0, 2.0, 0.0]);
        let vel = SVector::from([0.0, -10.0, 0.0]);
        let zero_normal = SVector::zeros();
        let scaled_normal = SVector::from([0.0, 2.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);

        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &zero_normal, 0.0, 1.0),
            Err(CcdQueryError::NonUnitPlaneNormal)
        );
        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &scaled_normal, 0.0, 1.0),
            Err(CcdQueryError::NonUnitPlaneNormal)
        );
        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &unit_normal, f64::NAN, 1.0),
            Err(CcdQueryError::NonFiniteInput)
        );
    }

    #[test]
    fn valid_checked_miss_is_distinct_from_invalid_query() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, 1.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);

        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, 1.0),
            Ok(None)
        );
        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, -1.0),
            Err(CcdQueryError::InvalidTimeStep)
        );
    }

    #[test]
    fn sphere_sphere_grazing_hit_and_near_miss_are_distinguished() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0]);
        let pos_b = SVector::from([5.0, 1.0]);
        let vel_b = SVector::zeros();

        let tangent = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 5.0)
            .unwrap()
            .unwrap();
        assert!((tangent.toi - 5.0).abs() < 1e-12);

        let near_miss = SVector::from([5.0, 1.0 + 1e-8]);
        assert_eq!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &near_miss, &vel_b, 0.5, 5.0),
            Ok(None)
        );
    }

    #[test]
    fn sphere_halfspace_accepts_exact_step_endpoint_and_rejects_just_outside() {
        let pos = SVector::from([0.0, 1.5, 0.0]);
        let vel = SVector::from([0.0, -1.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);

        let endpoint = sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, 1.0)
            .unwrap()
            .unwrap();
        assert!((endpoint.toi - 1.0).abs() < 1e-12);

        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, 1.0 - 1e-9),
            Ok(None)
        );
    }

    #[test]
    fn sphere_sphere_toi_is_invariant_to_common_translation_and_velocity() {
        let pos_a = SVector::from([1.0, -2.0]);
        let vel_a = SVector::from([4.0, 0.0]);
        let pos_b = SVector::from([7.0, 0.0]);
        let vel_b = SVector::zeros();

        let baseline = sphere_sphere_checked(&pos_a, &vel_a, 1.0, &pos_b, &vel_b, 1.0, 2.0)
            .unwrap()
            .unwrap();

        let translation = SVector::from([100.0, -50.0]);
        let drift = SVector::from([7.0, -3.0]);
        let translated_and_drifted = sphere_sphere_checked(
            &(pos_a + translation),
            &(vel_a + drift),
            1.0,
            &(pos_b + translation),
            &(vel_b + drift),
            1.0,
            2.0,
        )
        .unwrap()
        .unwrap();

        assert!((baseline.toi - 1.5).abs() < 1e-12);
        assert!((translated_and_drifted.toi - baseline.toi).abs() < 1e-12);
    }

    #[test]
    fn sphere_sweep_matches_independent_head_on_reference_over_fixed_grid() {
        // Frozen deterministic parameter sweep; the reference solution is
        // distance-to-contact divided by relative closing speed.
        for sample in 0..1_000 {
            let speed = 1.0 + sample as f64 * 0.01;
            let separation = 2.0 + sample as f64 * 0.03;
            let radius_a = 0.2;
            let radius_b = 0.3;
            let expected_toi = (separation - radius_a - radius_b) / speed;
            let dt = expected_toi * 1.01;

            let pos_a = SVector::zeros();
            let vel_a = SVector::from([speed, 0.0]);
            let pos_b = SVector::from([separation, 0.0]);
            let vel_b = SVector::zeros();
            let hit = sphere_sphere_checked(&pos_a, &vel_a, radius_a, &pos_b, &vel_b, radius_b, dt)
                .unwrap()
                .unwrap();

            assert!(
                (hit.toi - expected_toi).abs() <= 1e-11 * expected_toi.max(1.0),
                "sample={sample}: got {}, expected {expected_toi}",
                hit.toi
            );
        }
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
