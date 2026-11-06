// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Continuous Collision Detection (CCD): prevents fast-moving bodies from
//! tunneling through surfaces between frames.
//!
//! The analytic query helpers cover swept sphere-sphere and sphere-halfspace
//! cases. Checked variants distinguish invalid input from a valid no-hit
//! result; compatibility wrappers retain the original Option-returning API.
//! Time of impact is expressed in seconds from the beginning of the step.
//!
//! The integrated world pass currently supports sphere colliders against
//! sphere and half-space colliders. Rotational sweep and arbitrary convex
//! shape CCD are not implemented by this module.

use nalgebra::SVector;

/// Result of a continuous collision detection query.
#[derive(Clone, Debug, PartialEq)]
pub struct CcdHit<const D: usize> {
    /// Time of impact in seconds from step start, in [0, dt].
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Contact normal at time of impact.
    pub normal: SVector<f64, D>,
}

/// Invalid input or non-finite intermediate in a checked CCD query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdInputError {
    /// A position, velocity, radius, plane offset or time step is non-finite.
    NonFiniteInput,
    /// A radius is negative.
    NegativeRadius,
    /// The time step is negative or non-finite.
    InvalidTimeStep,
    /// A plane normal is zero or numerically degenerate.
    DegeneratePlaneNormal,
    /// Valid finite inputs produced a non-finite intermediate/result.
    NonFiniteIntermediate,
}

/// Speed threshold above which sphere CCD is always attempted.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

/// Maximum fraction of a sphere radius allowed to be swept before CCD is
/// attempted, even below the absolute speed threshold.
pub const CCD_SWEPT_RADIUS_FRACTION: f64 = 0.5;

/// Decide whether a sphere's translational sweep deserves an analytic CCD
/// query.
///
/// The absolute speed threshold is retained as a fast-path guard, but the
/// dimensionless displacement/radius criterion prevents small spheres or
/// large timesteps from being skipped solely because their speed is below
/// the absolute threshold. This is an activation heuristic, not a proof of
/// collision-free motion when it returns false.
pub fn should_sweep_sphere(speed: f64, radius: f64, dt: f64) -> bool {
    if !speed.is_finite()
        || !radius.is_finite()
        || !dt.is_finite()
        || speed < 0.0
        || radius < 0.0
        || dt < 0.0
    {
        return false;
    }

    speed > CCD_SPEED_THRESHOLD || speed * dt > radius * CCD_SWEPT_RADIUS_FRACTION
}

fn vector_is_finite<const D: usize>(value: &SVector<f64, D>) -> bool {
    value.iter().all(|component| component.is_finite())
}

/// Normalize a vector without overflowing while computing its norm. A
/// deterministic first-axis normal is used only for a coincident sphere pair.
fn normalize_or_axis<const D: usize>(
    value: &SVector<f64, D>,
) -> Result<SVector<f64, D>, CcdInputError> {
    let scale = value
        .iter()
        .fold(0.0_f64, |largest, component| largest.max(component.abs()));

    if !scale.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    if scale <= 1e-15 {
        let mut axis = SVector::zeros();
        if D == 0 {
            return Err(CcdInputError::DegeneratePlaneNormal);
        }
        axis[0] = 1.0;
        return Ok(axis);
    }

    let scaled = value / scale;
    let norm = scaled.norm();
    if !norm.is_finite() || norm <= 0.0 {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    Ok(scaled / norm)
}

/// Swept sphere vs sphere: checked earliest time of impact.
///
/// Given sphere positions and constant velocities, solve the relative-motion
/// problem in [0, dt]. The closest-approach formulation avoids subtracting
/// nearly equal large terms in the direct quadratic discriminant. All
/// distances and radii must use the same length unit; velocities and dt must
/// use compatible length/second and second units.
pub fn sphere_sphere_checked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    if !vector_is_finite(pos_a)
        || !vector_is_finite(vel_a)
        || !vector_is_finite(pos_b)
        || !vector_is_finite(vel_b)
        || !radius_a.is_finite()
        || !radius_b.is_finite()
    {
        return Err(CcdInputError::NonFiniteInput);
    }
    if radius_a < 0.0 || radius_b < 0.0 {
        return Err(CcdInputError::NegativeRadius);
    }
    if !dt.is_finite() || dt < 0.0 {
        return Err(CcdInputError::InvalidTimeStep);
    }

    let relative_position = pos_b - pos_a;
    let relative_velocity = vel_b - vel_a;
    let combined_radius = radius_a + radius_b;
    let radius_squared = combined_radius * combined_radius;
    let separation_squared = relative_position.dot(&relative_position);
    if !vector_is_finite(&relative_position)
        || !vector_is_finite(&relative_velocity)
        || !combined_radius.is_finite()
        || !radius_squared.is_finite()
        || !separation_squared.is_finite()
    {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    // Already overlapping or touching at the beginning of the interval.
    if separation_squared <= radius_squared {
        let normal = normalize_or_axis(&relative_position)?;
        return Ok(Some(CcdHit {
            toi: 0.0,
            point: pos_a + normal * radius_a,
            normal,
        }));
    }

    let relative_speed_squared = relative_velocity.dot(&relative_velocity);
    if !relative_speed_squared.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    if relative_speed_squared <= 0.0 {
        return Ok(None);
    }

    let projection = relative_position.dot(&relative_velocity);
    if !projection.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    // A positive projection means the current separation is increasing.
    if projection >= 0.0 {
        return Ok(None);
    }

    let closest_time = -projection / relative_speed_squared;
    if !closest_time.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    let closest_separation = relative_position + relative_velocity * closest_time;
    let closest_distance_squared = closest_separation.dot(&closest_separation);
    if !vector_is_finite(&closest_separation) || !closest_distance_squared.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    let radial_squared = radius_squared - closest_distance_squared;
    let grazing_tolerance = 64.0
        * f64::EPSILON
        * radius_squared
            .max(closest_distance_squared)
            .max(f64::MIN_POSITIVE);
    if radial_squared < -grazing_tolerance {
        return Ok(None);
    }

    let half_chord_time = (radial_squared.max(0.0) / relative_speed_squared).sqrt();
    let toi = closest_time - half_chord_time;
    if !half_chord_time.is_finite() || !toi.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    if toi < 0.0 || toi > dt {
        return Ok(None);
    }

    let pos_a_at_hit = pos_a + vel_a * toi;
    let pos_b_at_hit = pos_b + vel_b * toi;
    let normal = normalize_or_axis(&(pos_b_at_hit - pos_a_at_hit))?;
    let point = pos_a_at_hit + normal * radius_a;
    if !vector_is_finite(&pos_a_at_hit) || !vector_is_finite(&point) {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility wrapper for [`sphere_sphere_checked`].
///
/// Invalid input maps to `None`; new callers that need to distinguish an
/// invalid query from a valid no-hit result should use the checked variant.
pub fn sphere_sphere<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    sphere_sphere_checked(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt)
        .ok()
        .flatten()
}

/// Swept sphere vs half-space (infinite plane): checked time of impact.
///
/// The plane is represented by normal dot x = offset. The normal and offset
/// are normalized together, so scaling both by any positive finite factor
/// preserves the same geometric plane. The sphere radius and signed distance
/// use the same length unit.
pub fn sphere_halfspace_checked<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    if !vector_is_finite(pos)
        || !vector_is_finite(vel)
        || !vector_is_finite(plane_normal)
        || !radius.is_finite()
        || !plane_offset.is_finite()
    {
        return Err(CcdInputError::NonFiniteInput);
    }
    if radius < 0.0 {
        return Err(CcdInputError::NegativeRadius);
    }
    if !dt.is_finite() || dt < 0.0 {
        return Err(CcdInputError::InvalidTimeStep);
    }

    let normal_scale = plane_normal
        .iter()
        .fold(0.0_f64, |largest, component| largest.max(component.abs()));
    if !normal_scale.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    if normal_scale == 0.0 {
        return Err(CcdInputError::DegeneratePlaneNormal);
    }

    let scaled_normal = plane_normal / normal_scale;
    let scaled_norm = scaled_normal.norm();
    if !scaled_norm.is_finite() || scaled_norm <= 0.0 {
        return Err(CcdInputError::DegeneratePlaneNormal);
    }
    let normal = scaled_normal / scaled_norm;
    let offset = (plane_offset / normal_scale) / scaled_norm;
    if !offset.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    let signed_distance = normal.dot(pos) - offset;
    if !signed_distance.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    // Already intersecting the plane.
    if signed_distance <= radius {
        let point = pos - normal * signed_distance;
        if !vector_is_finite(&point) {
            return Err(CcdInputError::NonFiniteIntermediate);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal,
        }));
    }

    let velocity_toward_plane = -normal.dot(vel);
    if !velocity_toward_plane.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    if velocity_toward_plane <= 0.0 {
        return Ok(None);
    }

    let toi = (signed_distance - radius) / velocity_toward_plane;
    if !toi.is_finite() {
        return Err(CcdInputError::NonFiniteIntermediate);
    }
    if toi < 0.0 || toi > dt {
        return Ok(None);
    }

    let pos_at_hit = pos + vel * toi;
    let point = pos_at_hit - normal * radius;
    if !vector_is_finite(&pos_at_hit) || !vector_is_finite(&point) {
        return Err(CcdInputError::NonFiniteIntermediate);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility wrapper for [`sphere_halfspace_checked`].
///
/// Invalid input maps to `None`; new callers that need to distinguish an
/// invalid query from a valid no-hit result should use the checked variant.
pub fn sphere_halfspace<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Option<CcdHit<D>> {
    sphere_halfspace_checked(pos, vel, radius, plane_normal, plane_offset, dt)
        .ok()
        .flatten()
}

#[cfg(test)]
mod ccd_qualification_tests {
    use super::*;

    #[test]
    fn toi_is_reported_in_seconds_and_clamped_to_dt() {
        let pos = SVector::from([0.0, 5.0, 0.0]);
        let vel = SVector::from([0.0, -500.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);
        let hit = sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, 0.016)
            .unwrap()
            .unwrap();

        assert!((hit.toi - 0.009).abs() < 1e-12);
        assert!((0.0..=0.016).contains(&hit.toi));
    }

    #[test]
    fn halfspace_query_is_invariant_to_normal_scaling() {
        let pos = SVector::from([0.0, 3.0, 0.0]);
        let vel = SVector::from([0.0, -4.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 17.0, 0.0]);

        let a = sphere_halfspace_checked(&pos, &vel, 0.5, &unit_normal, 0.0, 1.0)
            .unwrap()
            .unwrap();
        let b = sphere_halfspace_checked(&pos, &vel, 0.5, &scaled_normal, 0.0, 1.0)
            .unwrap()
            .unwrap();

        assert!((a.toi - b.toi).abs() < 1e-14);
        assert!((a.point - b.point).norm() < 1e-14);
        assert!((a.normal - b.normal).norm() < 1e-14);
    }

    #[test]
    fn checked_queries_distinguish_invalid_inputs_from_no_hit() {
        let pos = SVector::from([0.0, 0.0, 0.0]);
        let vel = SVector::from([1.0, 0.0, 0.0]);
        let other = SVector::from([0.0, 10.0, 0.0]);
        let stationary = SVector::zeros();

        assert_eq!(
            sphere_sphere_checked(&pos, &vel, -1.0, &other, &stationary, 0.5, 1.0),
            Err(CcdInputError::NegativeRadius)
        );
        assert_eq!(
            sphere_sphere_checked(&pos, &vel, 0.5, &other, &stationary, 0.5, f64::NAN),
            Err(CcdInputError::InvalidTimeStep)
        );
        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &stationary, 0.0, 1.0),
            Err(CcdInputError::DegeneratePlaneNormal)
        );
        assert!(
            sphere_sphere_checked(&pos, &vel, 0.5, &other, &stationary, 0.5, 1.0)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn closest_approach_formulation_handles_large_separation() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::from([1_000_000.0, 0.0]);
        let pos_b = SVector::from([1_000_000.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();

        assert!((hit.toi - 0.999_999).abs() < 1e-10);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn grazing_hit_and_nearby_miss_are_distinguished() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::from([5.0, 0.0]);
        let stationary = SVector::zeros();
        let tangent = SVector::from([5.0, 1.0]);
        let outside = SVector::from([5.0, 1.000_01]);

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &tangent, &stationary, 0.5, 1.0)
            .unwrap()
            .unwrap();
        assert!((hit.toi - 1.0).abs() < 1e-10);

        assert!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &outside, &stationary, 0.5, 1.0)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn swapping_sphere_order_reverses_normal_but_preserves_toi() {
        let a_pos = SVector::from([0.0, 0.0, 0.0]);
        let a_vel = SVector::from([2.0, 0.0, 0.0]);
        let b_pos = SVector::from([3.0, 0.0, 0.0]);
        let b_vel = SVector::zeros();

        let ab = sphere_sphere_checked(&a_pos, &a_vel, 1.0, &b_pos, &b_vel, 1.0, 1.0)
            .unwrap()
            .unwrap();
        let ba = sphere_sphere_checked(&b_pos, &b_vel, 1.0, &a_pos, &a_vel, 1.0, 1.0)
            .unwrap()
            .unwrap();

        assert!((ab.toi - ba.toi).abs() < 1e-14);
        assert!((ab.normal + ba.normal).norm() < 1e-14);
    }

    #[test]
    fn displacement_to_radius_activation_catches_small_spheres() {
        assert!(!should_sweep_sphere(1.0, 0.5, 0.016));
        assert!(should_sweep_sphere(1.0, 0.001, 0.016));
        assert!(should_sweep_sphere(5.1, 0.5, 0.016));
        assert!(!should_sweep_sphere(f64::NAN, 0.5, 0.016));
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
