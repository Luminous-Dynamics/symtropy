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

/// Failure reason for a checked continuous-collision query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdError {
    /// A radius was negative or non-finite.
    InvalidRadius,
    /// The time interval was negative or non-finite.
    InvalidTimeStep,
    /// An input vector or scalar contained NaN or infinity.
    NonFiniteInput,
    /// The calculation overflowed or produced a non-finite intermediate.
    ArithmeticOverflow,
    /// A plane normal had zero length.
    DegeneratePlaneNormal,
    /// CCD is undefined for a zero-dimensional vector space.
    ZeroDimension,
}

fn vector_is_finite<const D: usize>(vector: &SVector<f64, D>) -> bool {
    vector.iter().all(|component| component.is_finite())
}

fn normalize_or_axis<const D: usize>(
    vector: &SVector<f64, D>,
) -> Result<SVector<f64, D>, CcdError> {
    if D == 0 {
        return Err(CcdError::ZeroDimension);
    }

    let scale = vector
        .iter()
        .fold(0.0_f64, |largest, value| largest.max(value.abs()));
    if scale == 0.0 {
        let mut axis = SVector::zeros();
        axis[0] = 1.0;
        return Ok(axis);
    }

    let scaled = vector / scale;
    let scaled_norm = scaled.norm();
    if !scaled_norm.is_finite() || scaled_norm == 0.0 {
        return Err(CcdError::ArithmeticOverflow);
    }

    Ok(scaled / scaled_norm)
}

fn validate_radius_dt(radius: f64, dt: f64) -> Result<(), CcdError> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(CcdError::InvalidRadius);
    }
    if !dt.is_finite() || dt < 0.0 {
        return Err(CcdError::InvalidTimeStep);
    }
    Ok(())
}

/// Checked swept sphere vs sphere query.
///
/// The implementation uses closest-approach geometry rather than the
/// cancellation-prone discriminant b² - 4ac. A valid miss returns Ok(None);
/// malformed inputs and non-finite arithmetic return a typed Err. The
/// compatibility wrapper sphere_sphere maps errors to None.
pub fn sphere_sphere_checked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdError> {
    validate_radius_dt(radius_a, dt)?;
    validate_radius_dt(radius_b, dt)?;
    if !vector_is_finite(pos_a)
        || !vector_is_finite(vel_a)
        || !vector_is_finite(pos_b)
        || !vector_is_finite(vel_b)
    {
        return Err(CcdError::NonFiniteInput);
    }
    if D == 0 {
        return Err(CcdError::ZeroDimension);
    }

    let combined_radius = radius_a + radius_b;
    if !combined_radius.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    if !vector_is_finite(&dp) || !vector_is_finite(&dv) {
        return Err(CcdError::ArithmeticOverflow);
    }

    let radius_squared = combined_radius * combined_radius;
    let initial_distance_squared = dp.dot(&dp);
    if !radius_squared.is_finite() || !initial_distance_squared.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    // Already overlapping or exactly touching at the beginning of the step.
    if initial_distance_squared <= radius_squared {
        let normal = normalize_or_axis(&dp)?;
        let point = pos_a + normal * radius_a;
        if !vector_is_finite(&point) {
            return Err(CcdError::ArithmeticOverflow);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal,
        }));
    }

    let relative_speed_squared = dv.dot(&dv);
    if !relative_speed_squared.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }
    if relative_speed_squared == 0.0 {
        return Ok(None);
    }

    // Time of closest approach and perpendicular separation. This avoids
    // subtracting nearly equal large values in the quadratic discriminant.
    let projection = dp.dot(&dv);
    if !projection.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }
    let time_of_closest_approach = -projection / relative_speed_squared;
    if !time_of_closest_approach.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    let closest_separation = dp + dv * time_of_closest_approach;
    if !vector_is_finite(&closest_separation) {
        return Err(CcdError::ArithmeticOverflow);
    }
    let closest_distance_squared = closest_separation.dot(&closest_separation);
    if !closest_distance_squared.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    let chord_squared = radius_squared - closest_distance_squared;
    let geometric_scale = radius_squared.max(closest_distance_squared);
    let roundoff_tolerance = 32.0 * f64::EPSILON * geometric_scale;
    if chord_squared < -roundoff_tolerance {
        return Ok(None);
    }

    // Clamp only roundoff-sized negative values at grazing/tangent contact.
    let half_chord_time = (chord_squared.max(0.0) / relative_speed_squared).sqrt();
    if !half_chord_time.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    let near_time = time_of_closest_approach - half_chord_time;
    let far_time = time_of_closest_approach + half_chord_time;
    let toi = if near_time >= 0.0 && near_time <= dt {
        near_time
    } else if far_time >= 0.0 && far_time <= dt {
        far_time
    } else {
        return Ok(None);
    };

    let pos_a_at_hit = pos_a + vel_a * toi;
    let pos_b_at_hit = pos_b + vel_b * toi;
    if !vector_is_finite(&pos_a_at_hit) || !vector_is_finite(&pos_b_at_hit) {
        return Err(CcdError::ArithmeticOverflow);
    }

    let separation_at_hit = pos_b_at_hit - pos_a_at_hit;
    if !vector_is_finite(&separation_at_hit) {
        return Err(CcdError::ArithmeticOverflow);
    }
    let normal = normalize_or_axis(&separation_at_hit)?;
    let point = pos_a_at_hit + normal * radius_a;
    if !vector_is_finite(&point) {
        return Err(CcdError::ArithmeticOverflow);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility swept-sphere query.
///
/// Invalid inputs and arithmetic failures map to None. New code should prefer
/// sphere_sphere_checked to distinguish a valid miss from an invalid query.
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

/// Checked swept sphere vs half-space query.
///
/// The plane normal is normalized internally, and the offset is scaled by the
/// same norm. Results are invariant to non-zero scaling of the supplied plane
/// equation. A valid miss returns Ok(None); malformed inputs and non-finite
/// arithmetic return a typed Err.
pub fn sphere_halfspace_checked<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdError> {
    validate_radius_dt(radius, dt)?;
    if !vector_is_finite(pos) || !vector_is_finite(vel) || !vector_is_finite(plane_normal) {
        return Err(CcdError::NonFiniteInput);
    }
    if !plane_offset.is_finite() {
        return Err(CcdError::NonFiniteInput);
    }
    if D == 0 {
        return Err(CcdError::ZeroDimension);
    }

    let normal_scale = plane_normal
        .iter()
        .fold(0.0_f64, |largest, value| largest.max(value.abs()));
    if normal_scale == 0.0 {
        return Err(CcdError::DegeneratePlaneNormal);
    }
    let scaled_normal = plane_normal / normal_scale;
    let scaled_normal_length = scaled_normal.norm();
    if !scaled_normal_length.is_finite() || scaled_normal_length == 0.0 {
        return Err(CcdError::ArithmeticOverflow);
    }
    let normal = scaled_normal / scaled_normal_length;
    let normalized_offset = (plane_offset / normal_scale) / scaled_normal_length;
    if !normalized_offset.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    let signed_distance = normal.dot(pos) - normalized_offset;
    if !signed_distance.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }

    // Already touching or penetrating.
    if signed_distance <= radius {
        let point = pos - normal * signed_distance;
        if !vector_is_finite(&point) {
            return Err(CcdError::ArithmeticOverflow);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal,
        }));
    }

    let velocity_toward_plane = -normal.dot(vel);
    if !velocity_toward_plane.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }
    if velocity_toward_plane <= 0.0 {
        return Ok(None);
    }

    let toi = (signed_distance - radius) / velocity_toward_plane;
    if !toi.is_finite() {
        return Err(CcdError::ArithmeticOverflow);
    }
    if toi < 0.0 || toi > dt {
        return Ok(None);
    }

    let center_at_hit = pos + vel * toi;
    if !vector_is_finite(&center_at_hit) {
        return Err(CcdError::ArithmeticOverflow);
    }
    let point = center_at_hit - normal * radius;
    if !vector_is_finite(&point) {
        return Err(CcdError::ArithmeticOverflow);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility swept-sphere vs half-space query.
///
/// Invalid inputs and arithmetic failures map to None. New code should prefer
/// sphere_halfspace_checked to distinguish a valid miss from an invalid query.
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

#[cfg(test)]
mod checked_tests {
    use super::*;

    #[test]
    fn checked_sphere_sphere_distinguishes_invalid_input_from_a_miss() {
        let zero = SVector::zeros();
        let valid_position = SVector::from([0.0, 0.0, 0.0]);
        let other_position = SVector::from([10.0, 0.0, 0.0]);

        assert!(matches!(
            sphere_sphere_checked(
                &valid_position,
                &zero,
                -1.0,
                &other_position,
                &zero,
                1.0,
                1.0
            ),
            Err(CcdError::InvalidRadius)
        ));
        assert!(matches!(
            sphere_sphere_checked(
                &valid_position,
                &zero,
                1.0,
                &other_position,
                &zero,
                1.0,
                f64::NAN
            ),
            Err(CcdError::InvalidTimeStep)
        ));

        let miss = sphere_sphere_checked(
            &valid_position,
            &zero,
            0.5,
            &SVector::from([0.0, 10.0, 0.0]),
            &zero,
            0.5,
            1.0,
        )
        .unwrap();
        assert!(miss.is_none());
    }

    #[test]
    fn closest_approach_solver_preserves_small_contact_chord_at_large_scale() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([0.0, 0.0, 0.0]);
        let pos_b = SVector::from([1.0e12, 0.0, 0.0]);
        let vel_b = SVector::from([-1.0e12, 0.0, 0.0]);

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();

        let expected = 1.0 - 1.0e-12;
        assert!(
            (hit.toi - expected).abs() < 2.0e-16,
            "TOI {:.17e} should preserve small contact chord; expected {:.17e}",
            hit.toi,
            expected
        );
        assert!(hit.normal.iter().all(|component| component.is_finite()));
        assert!((hit.normal.norm() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn halfspace_query_is_invariant_to_plane_equation_scaling() {
        let position = SVector::from([0.0, 10.0, 0.0]);
        let velocity = SVector::from([0.0, -100.0, 0.0]);
        let unit_normal = SVector::from([0.0, 1.0, 0.0]);
        let scaled_normal = SVector::from([0.0, 7.0, 0.0]);

        let unit = sphere_halfspace_checked(&position, &velocity, 0.5, &unit_normal, 1.0, 0.2)
            .unwrap()
            .unwrap();
        let scaled = sphere_halfspace_checked(&position, &velocity, 0.5, &scaled_normal, 7.0, 0.2)
            .unwrap()
            .unwrap();

        assert!((unit.toi - scaled.toi).abs() < 1e-14);
        assert!((unit.point - scaled.point).norm() < 1e-12);
        assert!((scaled.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn halfspace_checked_query_rejects_degenerate_and_non_finite_inputs() {
        let position = SVector::from([0.0, 1.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let zero_normal = SVector::zeros();
        let normal = SVector::from([0.0, 1.0, 0.0]);

        assert!(matches!(
            sphere_halfspace_checked(&position, &velocity, 0.5, &zero_normal, 0.0, 1.0),
            Err(CcdError::DegeneratePlaneNormal)
        ));
        assert!(matches!(
            sphere_halfspace_checked(&position, &velocity, 0.5, &normal, f64::INFINITY, 1.0),
            Err(CcdError::NonFiniteInput)
        ));
        assert!(matches!(
            sphere_halfspace_checked(&position, &velocity, 0.5, &normal, 0.0, -0.1),
            Err(CcdError::InvalidTimeStep)
        ));
    }

    #[test]
    fn tangent_contact_is_reported_at_the_closest_approach_time() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::zeros();
        let pos_b = SVector::from([5.0, 1.0, 0.0]);
        let vel_b = SVector::from([-10.0, 0.0, 0.0]);

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();

        assert!((hit.toi - 0.5).abs() < 1e-14);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn non_finite_vectors_are_typed_errors() {
        let pos_a = SVector::from([f64::NAN, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let velocity = SVector::zeros();

        assert!(matches!(
            sphere_sphere_checked(&pos_a, &velocity, 0.5, &pos_b, &velocity, 0.5, 1.0),
            Err(CcdError::NonFiniteInput)
        ));
    }

    #[test]
    fn huge_intermediate_arithmetic_fails_closed() {
        let position_a = SVector::from([f64::MAX, 0.0, 0.0]);
        let position_b = SVector::from([-f64::MAX, 0.0, 0.0]);
        let velocity = SVector::zeros();

        assert!(matches!(
            sphere_sphere_checked(
                &position_a,
                &velocity,
                0.5,
                &position_b,
                &velocity,
                0.5,
                1.0
            ),
            Err(CcdError::ArithmeticOverflow)
        ));
    }
}
