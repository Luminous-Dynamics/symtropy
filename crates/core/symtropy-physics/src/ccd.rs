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

/// Invalid geometry or kinematics passed to a checked CCD query.
///
/// The legacy Option-returning APIs remain available for compatibility, but
/// they intentionally conflate malformed input with a valid no-hit result.
/// The checked APIs below preserve that distinction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcdInputError {
    /// The vector space has no dimensions for a geometric contact normal.
    ZeroDimension,
    /// Position input contains NaN or infinity.
    NonFinitePosition,
    /// Velocity input contains NaN or infinity.
    NonFiniteVelocity,
    /// Radius is NaN or infinity.
    NonFiniteRadius,
    /// Radius is negative.
    NegativeRadius,
    /// Timestep is NaN or infinity.
    NonFiniteTimeStep,
    /// Timestep is negative.
    NegativeTimeStep,
    /// Plane normal contains NaN or infinity.
    NonFinitePlaneNormal,
    /// Plane normal has zero length.
    DegeneratePlaneNormal,
    /// Plane offset is NaN or infinity.
    NonFinitePlaneOffset,
}

#[inline]
fn is_finite_vector<const D: usize>(value: &SVector<f64, D>) -> bool {
    value.iter().all(|component| component.is_finite())
}

/// Scale-resistant Euclidean norm for finite vectors.
///
/// Returns None when the mathematical norm cannot be represented as a finite
/// f64. Scaling before squaring avoids the usual intermediate overflow and
/// underflow for vectors whose components are still representable.
fn stable_vector_norm<const D: usize>(value: &SVector<f64, D>) -> Option<f64> {
    if !is_finite_vector(value) {
        return None;
    }

    let scale = value
        .iter()
        .map(|component| component.abs())
        .fold(0.0, f64::max);
    if scale == 0.0 {
        return Some(0.0);
    }

    let scaled_sum = value
        .iter()
        .map(|component| {
            let scaled = component / scale;
            scaled * scaled
        })
        .sum::<f64>();
    let norm = scale * scaled_sum.sqrt();
    norm.is_finite().then_some(norm)
}

/// Deterministic fallback normal for coincident points, used only in D > 0.
fn fallback_normal<const D: usize>() -> SVector<f64, D> {
    let mut normal = SVector::zeros();
    normal[0] = 1.0;
    normal
}

fn validate_sphere_sphere_input<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<(), CcdInputError> {
    if D == 0 {
        return Err(CcdInputError::ZeroDimension);
    }
    if !is_finite_vector(pos_a) || !is_finite_vector(pos_b) {
        return Err(CcdInputError::NonFinitePosition);
    }
    if !is_finite_vector(vel_a) || !is_finite_vector(vel_b) {
        return Err(CcdInputError::NonFiniteVelocity);
    }
    for radius in [radius_a, radius_b] {
        if !radius.is_finite() {
            return Err(CcdInputError::NonFiniteRadius);
        }
        if radius < 0.0 {
            return Err(CcdInputError::NegativeRadius);
        }
    }
    if !dt.is_finite() {
        return Err(CcdInputError::NonFiniteTimeStep);
    }
    if dt < 0.0 {
        return Err(CcdInputError::NegativeTimeStep);
    }
    Ok(())
}

fn validate_sphere_halfspace_input<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Result<(), CcdInputError> {
    if D == 0 {
        return Err(CcdInputError::ZeroDimension);
    }
    if !is_finite_vector(pos) {
        return Err(CcdInputError::NonFinitePosition);
    }
    if !is_finite_vector(vel) {
        return Err(CcdInputError::NonFiniteVelocity);
    }
    if !is_finite_vector(plane_normal) {
        return Err(CcdInputError::NonFinitePlaneNormal);
    }
    let normal_length =
        stable_vector_norm(plane_normal).ok_or(CcdInputError::NonFinitePlaneNormal)?;
    if normal_length == 0.0 {
        return Err(CcdInputError::DegeneratePlaneNormal);
    }
    if !radius.is_finite() {
        return Err(CcdInputError::NonFiniteRadius);
    }
    if radius < 0.0 {
        return Err(CcdInputError::NegativeRadius);
    }
    if !plane_offset.is_finite() {
        return Err(CcdInputError::NonFinitePlaneOffset);
    }
    if !dt.is_finite() {
        return Err(CcdInputError::NonFiniteTimeStep);
    }
    if dt < 0.0 {
        return Err(CcdInputError::NegativeTimeStep);
    }
    Ok(())
}

/// Checked swept-sphere versus sphere query.
///
/// Returns `Ok(None)` for valid geometry with no impact in [0, dt], and
/// `Err(CcdInputError)` for invalid geometry or kinematic inputs. TOI is in
/// seconds, not a normalized fraction.
pub fn sphere_sphere_checked<const D: usize>(
    pos_a: &SVector<f64, D>,
    vel_a: &SVector<f64, D>,
    radius_a: f64,
    pos_b: &SVector<f64, D>,
    vel_b: &SVector<f64, D>,
    radius_b: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    validate_sphere_sphere_input(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt)?;
    Ok(sphere_sphere(
        pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt,
    ))
}

/// Checked swept-sphere versus half-space query.
///
/// Returns `Ok(None)` for valid geometry with no impact in [0, dt], and
/// `Err(CcdInputError)` for invalid geometry or kinematic inputs. The plane
/// equation is n·x = offset; positive rescaling of both terms is invariant.
pub fn sphere_halfspace_checked<const D: usize>(
    pos: &SVector<f64, D>,
    vel: &SVector<f64, D>,
    radius: f64,
    plane_normal: &SVector<f64, D>,
    plane_offset: f64,
    dt: f64,
) -> Result<Option<CcdHit<D>>, CcdInputError> {
    validate_sphere_halfspace_input(pos, vel, radius, plane_normal, plane_offset, dt)?;
    Ok(sphere_halfspace(
        pos,
        vel,
        radius,
        plane_normal,
        plane_offset,
        dt,
    ))
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
    validate_sphere_sphere_input(pos_a, vel_a, radius_a, pos_b, vel_b, radius_b, dt).ok()?;

    let dp = pos_b - pos_a;
    let relative_velocity = vel_b - vel_a;
    let radius_sum = radius_a + radius_b;
    if !is_finite_vector(&dp) || !is_finite_vector(&relative_velocity) || !radius_sum.is_finite() {
        return None;
    }

    let initial_distance = stable_vector_norm(&dp)?;
    if initial_distance <= radius_sum {
        let normal = if initial_distance > 0.0 {
            dp / initial_distance
        } else {
            fallback_normal::<D>()
        };
        let point = pos_a + normal * radius_a;
        return is_finite_vector(&point).then_some(CcdHit {
            toi: 0.0,
            point,
            normal,
        });
    }

    // Parameterize time as u = t / dt. The relative displacement over the
    // whole interval avoids a dimensionful speed-squared cutoff, so very slow
    // motion over a correspondingly long step is still treated correctly.
    let relative_displacement = relative_velocity * dt;
    if !is_finite_vector(&relative_displacement) {
        return None;
    }
    let a = relative_displacement.dot(&relative_displacement);
    if !a.is_finite() || a == 0.0 {
        return None;
    }

    let b = 2.0 * dp.dot(&relative_displacement);
    let c = (initial_distance - radius_sum) * (initial_distance + radius_sum);
    if !b.is_finite() || !c.is_finite() {
        return None;
    }

    // Scaling all coefficients by the same positive factor leaves the
    // roots unchanged and keeps the discriminant calculation bounded.
    let coefficient_scale = a.abs().max(b.abs()).max(c.abs());
    if !coefficient_scale.is_finite() || coefficient_scale == 0.0 {
        return None;
    }
    let a_scaled = a / coefficient_scale;
    let b_scaled = b / coefficient_scale;
    let c_scaled = c / coefficient_scale;

    let four_ac = 4.0 * a_scaled * c_scaled;
    let mut discriminant = b_scaled.mul_add(b_scaled, -four_ac);
    if !discriminant.is_finite() {
        return None;
    }
    if discriminant < 0.0 {
        // A mathematically tangent contact can produce a tiny negative
        // discriminant after rounding. Clamp only within a relative floating-
        // point error bound; a resolvable geometric near-miss remains a miss.
        let discriminant_scale = b_scaled * b_scaled + four_ac.abs();
        let roundoff_bound = 8.0 * f64::EPSILON * discriminant_scale;
        if discriminant >= -roundoff_bound {
            discriminant = 0.0;
        } else {
            return None;
        }
    }

    let sqrt_discriminant = discriminant.sqrt();
    // Cancellation-resistant q-form of the quadratic roots in normalized
    // interval coordinates u ∈ [0, 1].
    let q = -0.5 * (b_scaled + sqrt_discriminant.copysign(b_scaled));
    if q == 0.0 || !q.is_finite() {
        return None;
    }
    let mut roots = [q / a_scaled, c_scaled / q];
    roots.sort_by(f64::total_cmp);

    for normalized_toi in roots {
        if !normalized_toi.is_finite() || !(0.0..=1.0).contains(&normalized_toi) {
            continue;
        }
        let toi = normalized_toi * dt;
        if !toi.is_finite() {
            continue;
        }

        let center_a = pos_a + vel_a * toi;
        let center_b = pos_b + vel_b * toi;
        let delta = center_b - center_a;
        if !is_finite_vector(&center_a) || !is_finite_vector(&center_b) || !is_finite_vector(&delta)
        {
            return None;
        }

        let distance_at_hit = stable_vector_norm(&delta)?;
        let normal = if distance_at_hit > 0.0 {
            delta / distance_at_hit
        } else {
            fallback_normal::<D>()
        };
        let point = center_a + normal * radius_a;
        if !is_finite_vector(&point) || !is_finite_vector(&normal) {
            return None;
        }
        return Some(CcdHit { toi, point, normal });
    }

    None
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
    validate_sphere_halfspace_input(pos, vel, radius, plane_normal, plane_offset, dt).ok()?;

    // The plane is represented by n · x = offset. Normalize both sides of
    // that equation so any positive finite rescaling leaves its geometry
    // unchanged. No arbitrary normal-length threshold is used.
    let normal_length = stable_vector_norm(plane_normal)?;
    if normal_length == 0.0 {
        return None;
    }
    let unit_normal = plane_normal / normal_length;
    let normalized_offset = plane_offset / normal_length;
    if !normalized_offset.is_finite() {
        return None;
    }

    let signed_distance = unit_normal.dot(pos) - normalized_offset;
    if !signed_distance.is_finite() {
        return None;
    }

    // Already touching/penetrating at the start of the interval.
    if signed_distance <= radius {
        let point = pos - unit_normal * signed_distance;
        return is_finite_vector(&point).then_some(CcdHit {
            toi: 0.0,
            point,
            normal: unit_normal,
        });
    }

    let velocity_toward_plane = -unit_normal.dot(vel);
    if !velocity_toward_plane.is_finite() || velocity_toward_plane <= 0.0 {
        return None;
    }

    let toi = (signed_distance - radius) / velocity_toward_plane;
    if !toi.is_finite() || toi < 0.0 || toi > dt {
        return None;
    }

    let center_at_hit = pos + vel * toi;
    let point = center_at_hit - unit_normal * radius;
    if !is_finite_vector(&center_at_hit) || !is_finite_vector(&point) {
        return None;
    }

    Some(CcdHit {
        toi,
        point,
        normal: unit_normal,
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
    fn checked_queries_distinguish_invalid_inputs_from_valid_no_hit() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([1.0, 0.0, 0.0]);
        let pos_b = SVector::from([0.0, 10.0, 0.0]);
        let vel_b = SVector::zeros();

        assert!(matches!(
            sphere_sphere_checked(&pos_a, &vel_a, -0.5, &pos_b, &vel_b, 0.5, 1.0),
            Err(CcdInputError::NegativeRadius)
        ));
        assert!(matches!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, f64::INFINITY),
            Err(CcdInputError::NonFiniteTimeStep)
        ));
        assert!(matches!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0),
            Ok(None)
        ));

        let zero_normal = SVector::zeros();
        assert!(matches!(
            sphere_halfspace_checked(&pos_a, &vel_a, 0.5, &zero_normal, 0.0, 1.0),
            Err(CcdInputError::DegeneratePlaneNormal)
        ));
    }

    #[test]
    fn sphere_sphere_contact_residual_and_common_velocity_are_invariant() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let vel_b = SVector::from([-10.0, 0.0, 0.0]);
        let boost = SVector::from([123.0, -37.0, 4.5]);

        let baseline = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();
        let boosted = sphere_sphere_checked(
            &pos_a,
            &(vel_a + boost),
            0.5,
            &pos_b,
            &(vel_b + boost),
            0.5,
            1.0,
        )
        .unwrap()
        .unwrap();

        assert!((baseline.toi - 0.2).abs() < 1e-12);
        assert!((baseline.toi - boosted.toi).abs() < 1e-12);
        assert!((baseline.normal - boosted.normal).norm() < 1e-12);

        let center_a_at_hit = pos_a + vel_a * baseline.toi;
        let center_b_at_hit = pos_b + vel_b * baseline.toi;
        assert!(((center_b_at_hit - center_a_at_hit).norm() - 1.0).abs() < 1e-11);
        assert!(((baseline.point - center_a_at_hit).norm() - 0.5).abs() < 1e-11);
        assert!((baseline.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_toi_scales_with_time_reparameterization() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let vel_b = SVector::from([-10.0, 0.0, 0.0]);

        let original = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
            .unwrap()
            .unwrap();
        let time_scale = 4.0;
        let rescaled = sphere_sphere_checked(
            &pos_a,
            &(vel_a * time_scale),
            0.5,
            &pos_b,
            &(vel_b * time_scale),
            0.5,
            1.0 / time_scale,
        )
        .unwrap()
        .unwrap();

        assert!((original.toi - time_scale * rescaled.toi).abs() < 1e-12);
        assert!((original.point - rescaled.point).norm() < 1e-12);
    }

    #[test]
    fn halfspace_contact_residual_is_zero_for_non_unit_plane_normal() {
        let pos = SVector::from([0.0, 3.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let normal = SVector::from([0.0, 7.0, 0.0]);
        let offset = 0.0;

        let hit = sphere_halfspace_checked(&pos, &velocity, 0.5, &normal, offset, 3.0)
            .unwrap()
            .unwrap();
        let signed_plane_residual = hit.normal.dot(&hit.point) - offset / normal.norm();

        assert!((hit.toi - 2.5).abs() < 1e-12);
        assert!(signed_plane_residual.abs() < 1e-12);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn sphere_sphere_resolvable_grazing_near_miss_is_rejected() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let vel_a = SVector::from([10.0, 0.0, 0.0]);
        let pos_b = SVector::from([5.0, 1.000_001, 0.0]);
        let vel_b = SVector::zeros();

        assert!(
            sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1.0)
                .unwrap()
                .is_none()
        );
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
    fn plane_equation_scale_is_invariant_for_nonzero_offsets() {
        let pos = SVector::from([0.0, 3.0, 0.0]);
        let velocity = SVector::from([0.0, -1.0, 0.0]);
        let expected = sphere_halfspace_checked(
            &pos,
            &velocity,
            0.5,
            &SVector::from([0.0, 1.0, 0.0]),
            1.0,
            3.0,
        )
        .unwrap()
        .unwrap();

        for scale in [1e-20, 1e-6, 1.0, 1e6, 1e20] {
            let normal = SVector::from([0.0, scale, 0.0]);
            let offset = scale;
            let hit = sphere_halfspace_checked(&pos, &velocity, 0.5, &normal, offset, 3.0)
                .unwrap()
                .unwrap();

            assert!((hit.toi - expected.toi).abs() < 1e-12);
            assert!((hit.point - expected.point).norm() < 1e-12);
            assert!((hit.normal - expected.normal).norm() < 1e-12);
            assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        }
        assert!((expected.toi - 1.5).abs() < 1e-12);
        assert!((expected.point[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tiny_relative_speed_over_long_timestep_is_not_discarded() {
        let pos_a = SVector::from([0.0, 0.0, 0.0]);
        let pos_b = SVector::from([2.0, 0.0, 0.0]);
        let vel_a = SVector::from([1e-10, 0.0, 0.0]);
        let vel_b = SVector::zeros();

        let hit = sphere_sphere_checked(&pos_a, &vel_a, 0.5, &pos_b, &vel_b, 0.5, 1e10)
            .unwrap()
            .unwrap();

        assert!((hit.toi - 1e10).abs() < 1e-3);
        assert!((hit.normal.norm() - 1.0).abs() < 1e-12);
        let center_a = pos_a + vel_a * hit.toi;
        let center_b = pos_b + vel_b * hit.toi;
        assert!(((center_b - center_a).norm() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn zero_dimensional_queries_fail_without_indexing_a_normal() {
        let pos = SVector::<f64, 0>::zeros();
        let velocity = SVector::<f64, 0>::zeros();

        assert!(matches!(
            sphere_sphere_checked(&pos, &velocity, 0.5, &pos, &velocity, 0.5, 1.0),
            Err(CcdInputError::ZeroDimension)
        ));
        assert!(matches!(
            sphere_halfspace_checked(&pos, &velocity, 0.5, &pos, 0.0, 1.0),
            Err(CcdInputError::ZeroDimension)
        ));
        assert!(sphere_sphere(&pos, &velocity, 0.5, &pos, &velocity, 0.5, 1.0).is_none());
        assert!(sphere_halfspace(&pos, &velocity, 0.5, &pos, 0.0, 1.0).is_none());
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
