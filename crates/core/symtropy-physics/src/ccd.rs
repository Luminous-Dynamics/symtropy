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
    /// Time of impact in seconds within the current timestep: [0, dt].
    /// 0 = start of step, dt = end of step.
    pub toi: f64,
    /// Hit point in world space at time of impact.
    pub point: SVector<f64, D>,
    /// Contact normal at time of impact.
    pub normal: SVector<f64, D>,
}

/// Input field associated with a checked CCD query error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdInputField {
    PositionA,
    VelocityA,
    PositionB,
    VelocityB,
    Position,
    Velocity,
    PlaneNormal,
    RadiusA,
    RadiusB,
    Radius,
    PlaneOffset,
    TimeStep,
}

/// Errors that would otherwise be indistinguishable from a legitimate no-hit result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdInputError {
    ZeroDimension,
    NonFiniteVector { field: CcdInputField },
    NonFiniteScalar { field: CcdInputField },
    NegativeRadius { field: CcdInputField },
    NegativeTimeStep,
    ZeroPlaneNormal,
    NumericOverflow,
}

fn validate_vector<const D: usize>(
    value: &SVector<f64, D>,
    field: CcdInputField,
) -> Result<(), CcdInputError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(CcdInputError::NonFiniteVector { field })
    }
}

fn validate_radius(value: f64, field: CcdInputField) -> Result<(), CcdInputError> {
    if !value.is_finite() {
        return Err(CcdInputError::NonFiniteScalar { field });
    }
    if value < 0.0 {
        return Err(CcdInputError::NegativeRadius { field });
    }
    Ok(())
}

fn validate_timestep(dt: f64) -> Result<(), CcdInputError> {
    if !dt.is_finite() {
        return Err(CcdInputError::NonFiniteScalar {
            field: CcdInputField::TimeStep,
        });
    }
    if dt < 0.0 {
        return Err(CcdInputError::NegativeTimeStep);
    }
    Ok(())
}

fn validate_hit<const D: usize>(hit: &Option<CcdHit<D>>) -> Result<(), CcdInputError> {
    if let Some(hit) = hit
        && (!hit.toi.is_finite()
            || !hit.point.iter().all(|component| component.is_finite())
            || !hit.normal.iter().all(|component| component.is_finite()))
    {
        return Err(CcdInputError::NumericOverflow);
    }
    Ok(())
}

/// Speed threshold below which CCD is not applied (optimization).
/// Bodies moving slower than this per step won't tunnel through typical geometry.
pub const CCD_SPEED_THRESHOLD: f64 = 5.0;

/// Backwards-compatible CCD query. Invalid inputs return `None`.
///
/// Prefer [`sphere_sphere_checked`] when invalid input must be distinguished
/// from a valid query with no collision.
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

/// Checked swept-sphere query with explicit invalid-input and overflow states.
///
/// All positions use the same length unit, all velocities use that unit per
/// second, radii are non-negative lengths and `dt` is a non-negative interval.
pub fn sphere_sphere_checked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    if D == 0 {
        return Err(CcdInputError::ZeroDimension);
    }
    validate_vector(pos_a, CcdInputField::PositionA)?;
    validate_vector(vel_a, CcdInputField::VelocityA)?;
    validate_vector(pos_b, CcdInputField::PositionB)?;
    validate_vector(vel_b, CcdInputField::VelocityB)?;
    validate_radius(radius_a, CcdInputField::RadiusA)?;
    validate_radius(radius_b, CcdInputField::RadiusB)?;
    validate_timestep(dt)?;

    let dp = pos_b - pos_a;
    let dv = vel_b - vel_a;
    let combined_radius = radius_a + radius_b;
    let a = dv.dot(&dv);
    let b = 2.0 * dp.dot(&dv);
    let c = dp.dot(&dp) - combined_radius * combined_radius;
    let discriminant = b * b - 4.0 * a * c;
    if !combined_radius.is_finite()
        || !a.is_finite()
        || !b.is_finite()
        || !c.is_finite()
        || (!c.is_sign_negative() && !discriminant.is_finite())
    {
        return Err(CcdInputError::NumericOverflow);
    }

    let hit = sphere_sphere_impl(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt);
    validate_hit(&hit)?;
    Ok(hit)
}

/// Swept sphere vs sphere: find the earliest time of impact.
///
/// Given two spheres with positions and velocities, solve for the time `t ∈ [0, dt]`
/// when `|pos_a(t) - pos_b(t)| = radius_a + radius_b`.
///
/// This is a quadratic in t:
/// `|Δp + Δv*t|² = r²` where Δp = pos_b - pos_a, Δv = vel_b - vel_a, r = ra + rb.
fn sphere_sphere_impl<const D: usize>(
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

/// Backwards-compatible swept-sphere/half-space query. Invalid inputs return `None`.
///
/// Prefer [`sphere_halfspace_checked`] when invalid input must be distinguished
/// from a valid query with no collision.
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

/// Checked swept-sphere/half-space query.
///
/// The plane normal is normalized internally, and the plane offset is scaled
/// by the same norm. The returned contact normal is always unit length.
pub fn sphere_halfspace_checked<const D: usize>(
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
    validate_vector(pos, CcdInputField::Position)?;
    validate_vector(vel, CcdInputField::Velocity)?;
    validate_vector(plane_normal, CcdInputField::PlaneNormal)?;
    validate_radius(radius, CcdInputField::Radius)?;
    validate_timestep(dt)?;
    if !plane_offset.is_finite() {
        return Err(CcdInputError::NonFiniteScalar {
            field: CcdInputField::PlaneOffset,
        });
    }

    let normal_length = plane_normal.norm();
    if !normal_length.is_finite() {
        return Err(CcdInputError::NumericOverflow);
    }
    if normal_length <= 1e-15 {
        return Err(CcdInputError::ZeroPlaneNormal);
    }

    let normalized_normal = plane_normal / normal_length;
    let normalized_offset = plane_offset / normal_length;
    let signed_distance = normalized_normal.dot(pos) - normalized_offset;
    let velocity_toward_plane = -normalized_normal.dot(vel);
    if !normalized_offset.is_finite()
        || !signed_distance.is_finite()
        || !velocity_toward_plane.is_finite()
    {
        return Err(CcdInputError::NumericOverflow);
    }

    let hit = sphere_halfspace_impl(pos, vel, radius, &normalized_normal, normalized_offset, dt);
    validate_hit(&hit)?;
    Ok(hit)
}

/// Swept sphere vs half-space (infinite plane): find time of impact.
///
/// Solve for t where `normal · (pos + vel*t) - offset = radius`.
/// This is linear: `t = (radius + offset - normal · pos) / (normal · vel)`.
fn sphere_halfspace_impl<const D: usize>(
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
    fn checked_sphere_sphere_toi_satisfies_analytic_contact_residual() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_b = SVector::from([-10.0, 0.0, 0.0]);

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();
        assert!((hit.toi - 0.2).abs() < 1e-12);

        let center_a_at_toi = pos_a + vel_a * hit.toi;
        let center_b_at_toi = pos_b + vel_b * hit.toi;
        let distance_residual = (center_b_at_toi - center_a_at_toi).norm() - 1.0;
        assert!(distance_residual.abs() < 1e-12);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        assert!((hit.point - (center_a_at_toi + hit.normal * 0.5)).norm() < 1e-12);
    }

    #[test]
    fn checked_sphere_sphere_detects_grazing_tangent_contact() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 1.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();
        assert!((hit.toi - 0.5).abs() < 1e-12);

        let center_a_at_toi = pos_a + vel_a * hit.toi;
        let center_b_at_toi = pos_b + vel_b * hit.toi;
        assert!(((center_b_at_toi - center_a_at_toi).norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn checked_queries_distinguish_invalid_inputs_from_no_hit() {
        let pos = SVector::zeros();
        let velocity = SVector::from([1.0, 0.0, 0.0]);

        assert_eq!(
            sphere_sphere_checked(&pos, &velocity, -0.5, &pos, &velocity, 0.5, 1.0),
            Err(CcdInputError::NegativeRadius {
                field: CcdInputField::RadiusA
            })
        );
        assert_eq!(
            sphere_sphere_checked(&pos, &velocity, 0.5, &pos, &velocity, 0.5, -0.1),
            Err(CcdInputError::NegativeTimeStep)
        );

        let nan_position = SVector::from([f64::NAN, 0.0, 0.0]);
        assert_eq!(
            sphere_sphere_checked(&nan_position, &velocity, 0.5, &pos, &velocity, 0.5, 1.0),
            Err(CcdInputError::NonFiniteVector {
                field: CcdInputField::PositionA
            })
        );

        // A valid, non-intersecting path is Ok(None), not an input error.
        let far = SVector::from([0.0, 10.0, 0.0]);
        assert_eq!(
            sphere_sphere_checked(&pos, &velocity, 0.5, &far, &SVector::zeros(), 0.5, 1.0),
            Ok(None)
        );
    }

    #[test]
    fn checked_halfspace_normalizes_non_unit_plane_normal_and_offset() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        // 2*y - 4 = 0 describes y=2; normal normalization must divide the
        // offset by the same magnitude, while radius remains in metres.
        let normal = SVector::from([0.0, 2.0, 0.0]);
        let hit = sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 4.0, 1.0)
            .unwrap()
            .unwrap();

        assert!((hit.toi - 0.075).abs() < 1e-12);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        assert!((hit.normal[1] - 1.0).abs() < 1e-12);
        assert!((hit.point[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn checked_halfspace_rejects_zero_normal_and_non_finite_offset() {
        let pos = SVector::from([0.0, 2.0, 0.0]);
        let vel = SVector::from([0.0, -1.0, 0.0]);
        let zero = SVector::zeros();

        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &zero, 0.0, 1.0),
            Err(CcdInputError::ZeroPlaneNormal)
        );

        let normal = SVector::from([0.0, 1.0, 0.0]);
        assert_eq!(
            sphere_halfspace_checked(&pos, &vel, 0.5, &normal, f64::INFINITY, 1.0),
            Err(CcdInputError::NonFiniteScalar {
                field: CcdInputField::PlaneOffset
            })
        );
    }

    #[test]
    fn checked_halfspace_toi_satisfies_signed_distance_residual() {
        let pos = SVector::from([0.0, 10.0, 0.0]);
        let vel = SVector::from([0.0, -100.0, 0.0]);
        let normal = SVector::from([0.0, 3.0, 0.0]);
        let hit = sphere_halfspace_checked(&pos, &vel, 0.5, &normal, 0.0, 1.0)
            .unwrap()
            .unwrap();

        let center_at_toi = pos + vel * hit.toi;
        let signed_distance_at_toi = hit.normal.dot(&center_at_toi);
        assert!((signed_distance_at_toi - 0.5).abs() < 1e-12);
        assert!(hit.point.norm() < 1e-12);
    }

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
