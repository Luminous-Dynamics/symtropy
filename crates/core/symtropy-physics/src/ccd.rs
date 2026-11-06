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
    /// Time of impact in seconds from the start of the query interval.
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Unit contact normal at time of impact, pointing from A toward B for
    /// sphere-sphere queries and from the halfspace toward the sphere otherwise.
    pub normal: SVector<f64, D>,
}

/// Invalid or numerically unresolvable CCD query input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdQueryError {
    /// A query cannot construct a normal in a zero-dimensional space.
    ZeroDimension,
    /// One or more input values are NaN or infinite.
    NonFiniteInput,
    /// A sphere radius is negative.
    NegativeRadius,
    /// The query timestep is negative.
    NegativeTimeStep,
    /// The halfspace normal has zero length.
    DegeneratePlaneNormal,
    /// A finite input produced an unrepresentable intermediate or output.
    NumericalOverflow,
    /// A nonzero physical sweep underflowed to zero or lost all representable
    /// quadratic motion coefficients.
    NumericalUnderflow,
}

/// Primitive CCD query that produced a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdQueryKind {
    /// Swept sphere against sphere.
    SphereSphere,
    /// Swept sphere against halfspace.
    SphereHalfspace,
}

/// A CCD query that could not produce a trustworthy hit/no-hit result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CcdQueryDiagnostic {
    /// Moving/query body.
    pub body_a: crate::body::BodyHandle,
    /// Target body.
    pub body_b: crate::body::BodyHandle,
    /// Primitive query kind.
    pub kind: CcdQueryKind,
    /// Invalid-input or numerical failure.
    pub error: CcdQueryError,
}

fn vector_is_finite<const D: usize>(value: &SVector<f64, D>) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn vector_max_abs<const D: usize>(value: &SVector<f64, D>) -> f64 {
    value
        .iter()
        .fold(0.0_f64, |maximum, component| maximum.max(component.abs()))
}

/// Scaled Euclidean norm that avoids squaring large finite components.
fn vector_norm<const D: usize>(value: &SVector<f64, D>) -> f64 {
    value
        .iter()
        .fold(0.0_f64, |norm, component| norm.hypot(*component))
}

fn fallback_normal<const D: usize>() -> Result<SVector<f64, D>, CcdQueryError> {
    if D == 0 {
        return Err(CcdQueryError::ZeroDimension);
    }
    let mut normal = SVector::zeros();
    normal[0] = 1.0;
    Ok(normal)
}

fn valid_query_interval(dt: f64) -> Result<(), CcdQueryError> {
    if !dt.is_finite() {
        return Err(CcdQueryError::NonFiniteInput);
    }
    if dt < 0.0 {
        return Err(CcdQueryError::NegativeTimeStep);
    }
    Ok(())
}

/// Speed threshold below which CCD is not applied (optimization).
/// Bodies moving slower than this per step won't tunnel through typical geometry.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

/// Checked swept sphere vs sphere query using a scaled, cancellation-resistant
/// quadratic solve.
///
/// The normalized solve uses tau = t / dt in [0, 1] and scales both relative
/// position and swept displacement before evaluating the discriminant. The
/// smaller quadratic root uses the stable q formulation instead of the
/// cancellation-prone textbook expression.
pub fn sphere_sphere_checked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdQueryError> {
    if D == 0 {
        return Err(CcdQueryError::ZeroDimension);
    }
    valid_query_interval(dt)?;
    if !vector_is_finite(pos_a)
        || !vector_is_finite(vel_a)
        || !vector_is_finite(pos_b)
        || !vector_is_finite(vel_b)
        || !radius_a.is_finite()
        || !radius_b.is_finite()
    {
        return Err(CcdQueryError::NonFiniteInput);
    }
    if radius_a < 0.0 || radius_b < 0.0 {
        return Err(CcdQueryError::NegativeRadius);
    }

    let combined_radius = radius_a + radius_b;
    if !combined_radius.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }

    let relative_position = pos_b - pos_a;
    let relative_velocity = vel_b - vel_a;
    if !vector_is_finite(&relative_position) || !vector_is_finite(&relative_velocity) {
        return Err(CcdQueryError::NumericalOverflow);
    }

    let initial_distance = vector_norm(&relative_position);
    if !initial_distance.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }

    if initial_distance <= combined_radius {
        let normal = if initial_distance > 0.0 {
            relative_position / initial_distance
        } else {
            fallback_normal::<D>()?
        };
        let point = pos_a + normal * radius_a;
        if !vector_is_finite(&point) {
            return Err(CcdQueryError::NumericalOverflow);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal,
        }));
    }

    if dt == 0.0 {
        return Ok(None);
    }

    let velocity_scale = vector_max_abs(&relative_velocity);
    if velocity_scale == 0.0 {
        return Ok(None);
    }

    let travel_scale = velocity_scale * dt;
    if !travel_scale.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }
    if travel_scale == 0.0 {
        return Err(CcdQueryError::NumericalUnderflow);
    }

    // The maximum Euclidean displacement is bounded by sqrt(D) times the
    // largest component displacement. This early-out also avoids underflow
    // when the entire sweep is negligible relative to the initial gap.
    let separation = initial_distance - combined_radius;
    let max_displacement = travel_scale * (D as f64).sqrt();
    if !max_displacement.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }
    if separation > max_displacement {
        return Ok(None);
    }

    let length_scale = vector_max_abs(&relative_position)
        .max(combined_radius)
        .max(travel_scale);
    if !length_scale.is_finite() || length_scale == 0.0 {
        return Err(CcdQueryError::NumericalOverflow);
    }

    let scaled_position = relative_position / length_scale;
    let scaled_velocity = (relative_velocity / velocity_scale) * (travel_scale / length_scale);
    let scaled_radius = combined_radius / length_scale;

    let a = scaled_velocity.dot(&scaled_velocity);
    let b = 2.0 * scaled_position.dot(&scaled_velocity);
    let c = scaled_position.dot(&scaled_position) - scaled_radius * scaled_radius;
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }
    if a == 0.0 {
        return Err(CcdQueryError::NumericalUnderflow);
    }

    let four_ac = 4.0 * a * c;
    let b_squared = b * b;
    let discriminant = b_squared - four_ac;
    if !four_ac.is_finite() || !b_squared.is_finite() || !discriminant.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }

    // A small negative discriminant within the rounding bound is a tangent
    // contact. Larger negative values are a genuine miss.
    let discriminant_tolerance =
        32.0 * f64::EPSILON * (b_squared.abs() + four_ac.abs()).max(f64::MIN_POSITIVE);
    if discriminant < -discriminant_tolerance {
        return Ok(None);
    }
    let sqrt_discriminant = discriminant.max(0.0).sqrt();

    let q = -0.5
        * (b + if b >= 0.0 {
            sqrt_discriminant
        } else {
            -sqrt_discriminant
        });
    let (root_a, root_b) = if q == 0.0 {
        let root = -b / (2.0 * a);
        (root, root)
    } else {
        (q / a, c / q)
    };
    if !root_a.is_finite() || !root_b.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }

    let earliest = root_a.min(root_b);
    let latest = root_a.max(root_b);
    let time_tolerance = 32.0 * f64::EPSILON;
    let tau = if (-time_tolerance..=1.0 + time_tolerance).contains(&earliest) {
        earliest.clamp(0.0, 1.0)
    } else if (-time_tolerance..=1.0 + time_tolerance).contains(&latest) {
        latest.clamp(0.0, 1.0)
    } else {
        return Ok(None);
    };

    let toi = tau * dt;
    let pos_a_t = pos_a + vel_a * toi;
    let pos_b_t = pos_b + vel_b * toi;
    let delta_at_impact = pos_b_t - pos_a_t;
    let impact_distance = vector_norm(&delta_at_impact);
    if !toi.is_finite()
        || !vector_is_finite(&pos_a_t)
        || !vector_is_finite(&pos_b_t)
        || !impact_distance.is_finite()
    {
        return Err(CcdQueryError::NumericalOverflow);
    }

    let normal = if impact_distance > 0.0 {
        delta_at_impact / impact_distance
    } else {
        fallback_normal::<D>()?
    };
    let point = pos_a_t + normal * radius_a;
    if !vector_is_finite(&point) || !vector_is_finite(&normal) {
        return Err(CcdQueryError::NumericalOverflow);
    }

    Ok(Some(CcdHit { toi, point, normal }))
}

/// Compatibility wrapper. Invalid inputs map to None; new callers should use
/// sphere_sphere_checked to distinguish invalid input from a valid no-hit.
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

/// Checked swept sphere vs halfspace query.
///
/// The plane equation is n dot x = offset. The normal and offset are
/// normalized together, so non-unit normals describe the same plane without
/// changing its geometric location.
pub fn sphere_halfspace_checked<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdQueryError> {
    if D == 0 {
        return Err(CcdQueryError::ZeroDimension);
    }
    valid_query_interval(dt)?;
    if !vector_is_finite(pos)
        || !vector_is_finite(vel)
        || !vector_is_finite(plane_normal)
        || !radius.is_finite()
        || !plane_offset.is_finite()
    {
        return Err(CcdQueryError::NonFiniteInput);
    }
    if radius < 0.0 {
        return Err(CcdQueryError::NegativeRadius);
    }

    let normal_length = vector_norm(plane_normal);
    if !normal_length.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }
    if normal_length == 0.0 {
        return Err(CcdQueryError::DegeneratePlaneNormal);
    }

    let unit_normal = plane_normal / normal_length;
    let normalized_offset = plane_offset / normal_length;
    if !normalized_offset.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }

    let signed_distance = unit_normal.dot(pos) - normalized_offset;
    if !signed_distance.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }

    if signed_distance <= radius {
        let point = pos - unit_normal * signed_distance;
        if !vector_is_finite(&point) {
            return Err(CcdQueryError::NumericalOverflow);
        }
        return Ok(Some(CcdHit {
            toi: 0.0,
            point,
            normal: unit_normal,
        }));
    }

    if dt == 0.0 {
        return Ok(None);
    }

    let velocity_toward_plane = -unit_normal.dot(vel);
    if !velocity_toward_plane.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }
    if velocity_toward_plane <= 0.0 {
        return Ok(None);
    }

    let toi = (signed_distance - radius) / velocity_toward_plane;
    if !toi.is_finite() {
        return Err(CcdQueryError::NumericalOverflow);
    }
    if toi < 0.0 || toi > dt {
        return Ok(None);
    }

    let center_at_impact = pos + vel * toi;
    let point = center_at_impact - unit_normal * radius;
    if !vector_is_finite(&center_at_impact) || !vector_is_finite(&point) {
        return Err(CcdQueryError::NumericalOverflow);
    }

    Ok(Some(CcdHit {
        toi,
        point,
        normal: unit_normal,
    }))
}

/// Compatibility wrapper. Invalid inputs map to None; new callers should use
/// sphere_halfspace_checked to distinguish invalid input from a valid no-hit.
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
    fn zero_dimension_queries_return_typed_error_without_panicking() {
        let point = SVector::<f64, 0>::zeros();
        assert_eq!(
            sphere_sphere_checked(&point, &point, 0.0, &point, &point, 0.0, 1.0),
            Err(CcdQueryError::ZeroDimension)
        );
        assert_eq!(
            sphere_halfspace_checked(&point, &point, 0.0, &point, 0.0, 1.0),
            Err(CcdQueryError::ZeroDimension)
        );
    }

    #[test]
    fn nonzero_sweep_that_underflows_is_not_reported_as_no_hit() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::from([1e-300, 0.0]);
        let pos_b = SVector::from([1e-300, 0.0]);
        let vel_b = SVector::zeros();
        assert_eq!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.0, &pos_b, &vel_b, 0.0, 1e-300),
            Err(CcdQueryError::NumericalUnderflow)
        );
    }

    #[test]
    fn checked_query_rejects_invalid_and_non_finite_inputs() {
        let zero = SVector::zeros();
        let x = SVector::from([1.0, 0.0, 0.0]);
        assert_eq!(
            sphere_sphere_checked(&zero, &x, -0.5, &x, &zero, 0.5, 1.0),
            Err(CcdQueryError::NegativeRadius)
        );
        assert_eq!(
            sphere_sphere_checked(&zero, &x, 0.5, &x, &zero, 0.5, -1.0),
            Err(CcdQueryError::NegativeTimeStep)
        );
        let non_finite = SVector::from([f64::NAN, 0.0, 0.0]);
        assert_eq!(
            sphere_sphere_checked(&zero, &x, 0.5, &non_finite, &zero, 0.5, 1.0),
            Err(CcdQueryError::NonFiniteInput)
        );
        assert_eq!(
            sphere_halfspace_checked(&zero, &x, 0.5, &zero, 0.0, 1.0),
            Err(CcdQueryError::DegeneratePlaneNormal)
        );
    }

    #[test]
    fn non_unit_plane_normal_preserves_the_geometric_plane() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let normal = SVector::from([0.0, 10.0, 0.0]);
        let hit = sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 20.0, 1.0)
            .unwrap()
            .unwrap();
        assert!((hit.toi - 0.075).abs() < 1e-12);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        assert!((hit.point[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_handles_tangent_contact_and_closed_interval_endpoint() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0]);
        let pos_b = SVector::from([5.0, 1.0]);
        let vel_b = SVector::zeros();
        let tangent = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 5.0)
            .unwrap()
            .unwrap();
        assert!((tangent.toi - 5.0).abs() < 1e-10);

        let endpoint = sphere_sphere_checked(
            &SVector::from([0.0, 0.0]),
            &SVector::from([1.0, 0.0]),
            0.5,
            &SVector::from([2.0, 0.0]),
            &SVector::zeros(),
            0.5,
            1.0,
        )
        .unwrap()
        .unwrap();
        assert!((endpoint.toi - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_is_scale_invariant_for_micro_and_mega_geometry() {
        for scale in [1e-6, 1e6] {
            let pos_a = SVector::from([0.0, 0.0, 0.0]);
            let vel_a = SVector::from([10.0 * scale, 0.0, 0.0]);
            let pos_b = SVector::from([5.0 * scale, 0.0, 0.0]);
            let vel_b = SVector::from([-10.0 * scale, 0.0, 0.0]);
            let hit = sphere_sphere_checked(
                &pos_a,
                &vel_a,
                0.5 * scale,
                &pos_b,
                &vel_b,
                0.5 * scale,
                1.0,
            )
            .unwrap()
            .unwrap();
            assert!((hit.toi - 0.2).abs() < 1e-12);
            assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn near_cancellation_root_remains_accurate() {
        let pos_a = SVector::from([0.0, 0.0]);
        let vel_a = SVector::from([1.0e7, 0.0]);
        let pos_b = SVector::from([1.0e7, 0.0]);
        let vel_b = SVector::zeros();
        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();
        assert!((hit.toi - 0.999_999_9).abs() < 2e-9);
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
