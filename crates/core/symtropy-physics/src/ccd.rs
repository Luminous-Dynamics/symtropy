// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Continuous Collision Detection (CCD): prevents fast-moving bodies from
//! tunneling through surfaces between frames.
//!
//! Provides validated analytical time-of-impact (TOI) queries for linear
//! sphere trajectories:
//! - **Sphere-sphere**: closest-approach geometry for a stable earliest hit
//! - **Sphere-halfspace**: linear plane-crossing solve with a normalized normal
//!
//! These primitives do not implement general convex shape casting, angular
//! sweeps, or conservative advancement for arbitrary shape pairs. They are
//! exact for the declared constant-linear-velocity sphere/halfspace model.
//! The compatibility wrappers preserve the historical Option API; checked
//! callers should use the try_* variants to distinguish invalid input from a
//! valid no-hit result.
//!
//! # World integration
//!
//! PhysicsWorld uses these primitives for its current sphere and halfspace
//! fast-body cases. General rotational and convex-shape CCD remain separate
//! research requirements; deterministic query correctness alone is not a
//! proof that an integrated simulation cannot tunnel.

use nalgebra::SVector;

/// Result of a continuous collision detection query.
#[derive(Clone, Debug)]
pub struct CcdHit<const D: usize> {
    /// Absolute time of impact in seconds from the start of the sweep.
    /// The value is in [0, dt], not a normalized fraction of the timestep.
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Unit contact normal at time of impact, pointing from sphere A to B
    /// for sphere-sphere and toward the free half-space for sphere-plane.
    pub normal: SVector<f64, D>,
}

/// Errors that distinguish invalid CCD inputs from a valid no-hit result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdInputError {
    /// A position, velocity, radius, plane offset or time interval was non-finite.
    NonFiniteInput,
    /// A sphere radius was negative.
    NegativeRadius,
    /// The sweep duration was negative.
    NegativeTimeStep,
    /// A zero-dimensional vector cannot define a contact normal.
    ZeroDimension,
    /// The plane normal had zero length.
    ZeroPlaneNormal,
    /// Finite inputs produced a non-finite intermediate or output.
    NumericalOverflow,
    /// Floating-point error prevents a reliable classification near contact.
    NumericalAmbiguity,
}

/// Speed threshold below which CCD is not applied (optimization).
/// Bodies moving slower than this per step won't tunnel through typical geometry.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

fn vector_is_finite<const D: usize>(value: &SVector<f64, D>) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn stable_norm<const D: usize>(value: &SVector<f64, D>) -> f64 {
    value
        .iter()
        .fold(0.0_f64, |norm, component| norm.hypot(*component))
}

fn validate_sweep<const D: usize>(
    position_a: &SVector<f64, D>,
    velocity_a: &SVector<f64, D>,
    radius_a: f64,
    position_b: &SVector<f64, D>,
    velocity_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<(), CcdInputError> {
    if D == 0 {
        return Err(CcdInputError::ZeroDimension);
    }
    if !vector_is_finite(position_a)
        || !vector_is_finite(velocity_a)
        || !vector_is_finite(position_b)
        || !vector_is_finite(velocity_b)
        || !radius_a.is_finite()
        || !radius_b.is_finite()
        || !dt.is_finite()
    {
        return Err(CcdInputError::NonFiniteInput);
    }
    if radius_a < 0.0 || radius_b < 0.0 {
        return Err(CcdInputError::NegativeRadius);
    }
    if dt < 0.0 {
        return Err(CcdInputError::NegativeTimeStep);
    }
    Ok(())
}

/// Validated swept sphere vs sphere query.
///
/// Uses a closest-approach formulation rather than subtracting two large
/// quadratic-discriminant terms. Let t_c be the time of closest approach and
/// d_c the separation there. The contact half-chord is
/// sqrt(((r - d_c) * (r + d_c)) / |delta-v|^2). This avoids the catastrophic
/// cancellation in the textbook b^2 - 4ac discriminant for large travel
/// distances and small sphere radii.
///
/// All positions are metres, velocities metres/second, radii metres and dt
/// seconds. The result toi is absolute seconds in [0, dt].
pub fn try_sphere_sphere<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    validate_sweep(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt)?;

    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    if !vector_is_finite(&dp) || !vector_is_finite(&dv) {
        return Err(CcdInputError::NumericalOverflow);
    }

    let radius = radius_a + radius_b;
    let initial_distance = stable_norm(&dp);
    if !radius.is_finite() || !initial_distance.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }

    // Already overlapping at t=0.
    if initial_distance <= radius {
        let normal = if initial_distance > 1e-15 {
            dp / initial_distance
        } else {
            let mut axis = SVector::zeros();
            axis[0] = 1.0;
            axis
        };
        let point = pos_a + normal * radius_a;
        if !vector_is_finite(&point) {
            return Err(CcdInputError::NumericalOverflow);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal,
        }));
    }

    let relative_speed_squared = dv.dot(&dv);
    let projected_motion = dp.dot(&dv);
    if !relative_speed_squared.is_finite() || !projected_motion.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }
    if relative_speed_squared == 0.0 {
        return Ok(None);
    }

    // Time of closest approach for the infinite relative-motion line.
    let closest_time = -projected_motion / relative_speed_squared;
    if !closest_time.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }
    if closest_time < 0.0 {
        return Ok(None);
    }

    let closest_separation = dp + dv * closest_time;
    if !vector_is_finite(&closest_separation) {
        return Err(CcdInputError::NumericalOverflow);
    }
    let closest_distance = stable_norm(&closest_separation);
    if !closest_distance.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }
    if closest_distance > radius {
        return Ok(None);
    }

    // This product is better conditioned than a quadratic discriminant
    // when the spheres are far apart but their radii are small.
    let chord_squared = (radius - closest_distance) * (radius + closest_distance);
    let half_chord_time = (chord_squared / relative_speed_squared).sqrt();
    if !chord_squared.is_finite() || !half_chord_time.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }

    let entry_time = closest_time - half_chord_time;
    let exit_time = closest_time + half_chord_time;
    if entry_time < 0.0 && exit_time >= 0.0 {
        return Err(CcdInputError::NumericalAmbiguity);
    }
    if entry_time < 0.0 || entry_time > dt {
        return Ok(None);
    }
    let toi = entry_time;

    let pos_a_at_hit = pos_a + vel_a * toi;
    let relative_at_hit = dp + dv * toi;
    if !vector_is_finite(&pos_a_at_hit) || !vector_is_finite(&relative_at_hit) {
        return Err(CcdInputError::NumericalOverflow);
    }
    let separation_at_hit = stable_norm(&relative_at_hit);
    let normal = if separation_at_hit > 1e-15 {
        relative_at_hit / separation_at_hit
    } else {
        let mut axis = SVector::zeros();
        axis[0] = 1.0;
        axis
    };
    let point = pos_a_at_hit + normal * radius_a;
    if !vector_is_finite(&point) || !vector_is_finite(&normal) {
        return Err(CcdInputError::NumericalOverflow);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility wrapper for try_sphere_sphere.
///
/// Invalid input is represented as None; new callers that need to
/// distinguish invalid input from a valid no-hit result should use the
/// checked function.
pub fn sphere_sphere<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    try_sphere_sphere(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt)
        .ok()
        .flatten()
}

/// Validated swept sphere vs half-space (infinite plane) query.
///
/// The plane normal is normalized internally and plane_offset is scaled by
/// the same magnitude. This makes the geometric result independent of whether
/// the caller supplied a unit normal. All lengths are metres and dt is
/// seconds. The returned toi is absolute seconds in [0, dt].
pub fn try_sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    if D == 0 {
        return Err(CcdInputError::ZeroDimension);
    }
    if !vector_is_finite(pos)
        || !vector_is_finite(vel)
        || !vector_is_finite(plane_normal)
        || !radius.is_finite()
        || !plane_offset.is_finite()
        || !dt.is_finite()
    {
        return Err(CcdInputError::NonFiniteInput);
    }
    if radius < 0.0 {
        return Err(CcdInputError::NegativeRadius);
    }
    if dt < 0.0 {
        return Err(CcdInputError::NegativeTimeStep);
    }

    let normal_length = stable_norm(plane_normal);
    if !normal_length.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }
    if normal_length <= 1e-15 {
        return Err(CcdInputError::ZeroPlaneNormal);
    }
    let normal = plane_normal / normal_length;
    let normalized_offset = plane_offset / normal_length;
    if !normalized_offset.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }

    let signed_distance = normal.dot(pos) - normalized_offset;
    if !signed_distance.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }

    // Already penetrating or touching.
    if signed_distance <= radius {
        let point = pos - normal * signed_distance;
        if !vector_is_finite(&point) {
            return Err(CcdInputError::NumericalOverflow);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal,
        }));
    }

    let velocity_toward_plane = -normal.dot(vel);
    if !velocity_toward_plane.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }
    if velocity_toward_plane <= 0.0 {
        return Ok(None);
    }

    let toi = (signed_distance - radius) / velocity_toward_plane;
    if !toi.is_finite() {
        return Err(CcdInputError::NumericalOverflow);
    }
    if toi < 0.0 || toi > dt {
        return Ok(None);
    }

    let center_at_hit = pos + vel * toi;
    let point = center_at_hit - normal * radius;
    if !vector_is_finite(&center_at_hit) || !vector_is_finite(&point) {
        return Err(CcdInputError::NumericalOverflow);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility wrapper for try_sphere_halfspace.
///
/// Invalid input is represented as None; new callers that need to
/// distinguish invalid input from a valid no-hit result should use the
/// checked function.
pub fn sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    try_sphere_halfspace(pos, vel, radius, plane_normal, plane_offset, dt)
        .ok()
        .flatten()
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
            (hit.toi - 0.2).abs() < 1e-12,
            "TOI = {}, expected ~0.2",
            hit.toi
        );
    }

    #[test]
    fn sphere_sphere_large_travel_small_radius_is_numerically_stable() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([1.0e12, 0.0, 0.0]);
        let vel_b = SVector::from([-1.0, 0.0, 0.0]);

        let hit = try_sphere_sphere(&pos_a, &vel_a, 8.0, &pos_b, &vel_b, 0.0, 1.0e12)
            .unwrap()
            .unwrap();

        let expected = 1.0e12 - 8.0;
        assert!(
            (hit.toi - expected).abs() <= 2.0e-4,
            "stable TOI {}, expected {expected}",
            hit.toi
        );
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_is_symmetric_under_body_swap() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([2.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::from([-2.0, 0.0, 0.0]);

        let ab = try_sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 2.0)
            .unwrap()
            .unwrap();
        let ba = try_sphere_sphere(&pos_b, &vel_b, 0.5, &pos_a, &vel_a, 0.5, 2.0)
            .unwrap()
            .unwrap();

        assert!((ab.toi - ba.toi).abs() < 1e-12);
        assert!((ab.normal + ba.normal).norm() < 1e-12);
    }

    #[test]
    fn sphere_sphere_toi_is_galilean_invariant() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([2.0, 0.5, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::from([-2.0, -0.5, 0.0]);
        let boost = SVector::from([123.0, -44.0, 9.0]);

        let original = try_sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 2.0)
            .unwrap()
            .unwrap();
        let boosted = try_sphere_sphere(
            &pos_a,
            &(vel_a + boost),
            0.5,
            &pos_b,
            &(vel_b + boost),
            0.5,
            2.0,
        )
        .unwrap()
        .unwrap();

        assert!((original.toi - boosted.toi).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_initial_state_is_not_reclassified_as_exit_contact() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([0.5, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = try_sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();
        assert_eq!(hit.toi, 0.0);
    }

    #[test]
    fn sphere_sphere_grazing_contact_has_finite_toi() {
        let pos_a = SVector::zeros();
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 1.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = try_sphere_sphere(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();

        assert!((hit.toi - 0.5).abs() < 1e-12);
        assert!(hit.point.iter().all(|component| component.is_finite()));
        assert!(hit.normal.iter().all(|component| component.is_finite()));
    }

    #[test]
    fn sphere_sphere_rejects_invalid_dimensions_and_parameters() {
        let pos: SVector<f64, 3> = SVector::zeros();
        let velocity: SVector<f64, 3> = SVector::zeros();

        assert!(matches!(
            try_sphere_sphere(&pos, &velocity, -0.5, &pos, &velocity, 0.5, 1.0),
            Err(CcdInputError::NegativeRadius)
        ));
        assert!(matches!(
            try_sphere_sphere(&pos, &velocity, 0.5, &pos, &velocity, 0.5, f64::NAN),
            Err(CcdInputError::NonFiniteInput)
        ));
        assert!(matches!(
            try_sphere_sphere(&pos, &velocity, 0.5, &pos, &velocity, 0.5, -1.0),
            Err(CcdInputError::NegativeTimeStep)
        ));
        assert!(sphere_sphere(&pos, &velocity, -0.5, &pos, &velocity, 0.5, 1.0).is_none());
    }

    #[test]
    fn sphere_halfspace_normalizes_non_unit_plane_normal() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 3.0, 0.0]);

        let unit = try_sphere_halfspace(&pos, &vel, 0.5, &unit_normal, 2.0, 1.0)
            .unwrap()
            .unwrap();
        let scaled = try_sphere_halfspace(&pos, &vel, 0.5, &scaled_normal, 6.0, 1.0)
            .unwrap()
            .unwrap();

        assert!((unit.toi - 0.075).abs() < 1e-12);
        assert!((scaled.toi - unit.toi).abs() < 1e-12);
        assert!((scaled.point - unit.point).norm() < 1e-12);
        assert!((scaled.normal.norm() - 1.0).abs() < 1e-12);
        assert!((scaled.normal[1] - 1.0).abs() < 1e-12);
        assert!(scaled.point.iter().all(|component| component.is_finite()));
    }
    #[test]
    fn sphere_halfspace_rejects_degenerate_plane_and_non_finite_inputs() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -10.0, 0.0]);

        assert!(matches!(
            try_sphere_halfspace(&pos, &vel, 0.5, &SVector::zeros(), 0.0, 1.0),
            Err(CcdInputError::ZeroPlaneNormal)
        ));
        assert!(matches!(
            try_sphere_halfspace(
                &pos,
                &vel,
                0.5,
                &SVector::from([0.0, 1.0, 0.0]),
                f64::INFINITY,
                1.0
            ),
            Err(CcdInputError::NonFiniteInput)
        ));
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
