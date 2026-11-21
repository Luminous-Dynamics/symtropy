// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use arrayvec::ArrayVec;
use nalgebra::SVector;

use crate::body::BodyHandle;

/// Maximum contact points per manifold. D+1 covers all common cases up to 4D.
pub const MAX_CONTACTS: usize = 8;

/// A single contact point within a manifold.
#[derive(Clone, Debug)]
pub struct ContactPoint<const D: usize> {
    /// Contact position in world space.
    pub position: SVector<f64, D>,
    /// Penetration depth at this point (positive = overlapping).
    pub depth: f64,
    /// Accumulated normal impulse this frame (TGS Soft solver state).
    /// Initialised to 0.0 each frame; warm-started from the previous frame's cache.
    pub lambda: f64,
    /// Restitution target velocity (bias), computed ONCE per frame from the
    /// pre-solve approach speed (`-e * v_rel_n` when bodies are approaching
    /// faster than a small threshold, else 0.0). The solver combines this
    /// with the Baumgarte position-correction bias via `max()`, so the target
    /// departing speed reflects the true approach velocity rather than being
    /// re-derived (and diluted) on every TGS iteration. See
    /// `PhysicsWorld::compute_restitution_bias` and the `-(1+e)*v_rel_n`
    /// term in [`ContactManifold::impulse_magnitude`] below, which this
    /// mirrors.
    pub restitution_bias: f64,
}

/// Collision event emitted when two bodies collide.
#[derive(Clone, Debug)]
pub struct CollisionEvent<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    /// Impulse magnitude applied during resolution.
    pub impulse: f64,
    /// Contact normal.
    pub normal: SVector<f64, D>,
    /// Penetration depth.
    pub depth: f64,
}

/// Sensor overlap event (no collision resolution, just notification).
#[derive(Clone, Debug)]
pub struct SensorEvent {
    /// The sensor body.
    pub sensor: BodyHandle,
    /// The other body overlapping the sensor.
    pub other: BodyHandle,
}

/// Failure while evaluating the rotational contribution to contact effective mass.
#[derive(Clone, Debug, PartialEq)]
pub enum ContactResponseError {
    /// A lever arm or contact direction contains NaN or infinity.
    NonFiniteInput,
    /// The body's strict rotational-inertia admission or solve failed.
    RotationalResponse(crate::body::RotationalResponseError),
    /// A finite input produced a non-finite effective-mass contribution.
    NonFiniteEffectiveMass,
    /// A positive-definite inertia response produced a negative quadratic value.
    NegativeEffectiveMass { value: f64 },
}

/// Compute the angular contribution to normal-contact effective mass.
///
/// For a unit impulse direction n at center-of-mass lever arm r, this
/// evaluates n dot (-(I^-1(r wedge n)) r) using the body's admitted
/// rotational-inertia model. The value is the rotational term in the
/// contact denominator (units 1/kg). The caller must supply a normalized
/// contact normal.
///
/// Unlike the legacy world helper, this function never converts an
/// unsupported or stale inertia model into a physical zero. A negative
/// result is rejected because an admitted positive-definite inertia
/// operator must produce a non-negative quadratic response.
pub fn try_angular_contact_effective_mass<const D: usize>(
    body: &crate::body::RigidBody<D>,
    lever_arm: &SVector<f64, D>,
    normal: &SVector<f64, D>,
) -> Result<f64, ContactResponseError> {
    if !lever_arm.iter().all(|value| value.is_finite())
        || !normal.iter().all(|value| value.is_finite())
    {
        return Err(ContactResponseError::NonFiniteInput);
    }

    let angular_impulse = crate::integrator::angular_impulse_from_point(lever_arm, normal);
    let delta_angular_velocity = match body
        .try_apply_inverse_angular_inertia(&angular_impulse)
        .map_err(ContactResponseError::RotationalResponse)?
    {
        crate::body::RotationalResponse::StaticOrKinematic => {
            symtropy_math::Bivector::<D>::zero()
        }
        crate::body::RotationalResponse::Applied {
            delta_angular_velocity,
            ..
        } => delta_angular_velocity,
    };

    let point_velocity =
        crate::integrator::angular_point_velocity(&delta_angular_velocity, lever_arm);
    let effective_mass = normal.dot(&point_velocity);
    if !effective_mass.is_finite() {
        return Err(ContactResponseError::NonFiniteEffectiveMass);
    }
    if effective_mass < 0.0 {
        return Err(ContactResponseError::NegativeEffectiveMass { value: effective_mass });
    }
    Ok(effective_mass)
}
/// Contact information from a collision between two bodies.
///
/// Holds one or more contact points. Multi-point manifolds arise from
/// flat-on-flat contacts (e.g., box on plane = 4 points in 3D).
#[derive(Clone, Debug)]
pub struct ContactManifold<const D: usize> {
    /// Handle of body A.
    pub body_a: BodyHandle,
    /// Handle of body B.
    pub body_b: BodyHandle,
    /// Contact normal pointing from A to B.
    pub normal: SVector<f64, D>,
    /// Contact points (at least 1). Zero-heap via ArrayVec.
    pub points: ArrayVec<ContactPoint<D>, MAX_CONTACTS>,
    /// Material elasticity (Young's modulus in Pa) at the contact surface.
    pub elasticity: Option<f64>,
}

impl<const D: usize> ContactManifold<D> {
    /// Create a single-point manifold (backward compatible).
    pub fn single(
        body_a: BodyHandle,
        body_b: BodyHandle,
        normal: SVector<f64, D>,
        point: SVector<f64, D>,
        depth: f64,
    ) -> Self {
        let mut points = ArrayVec::new();
        points.push(ContactPoint {
            position: point,
            depth,
            lambda: 0.0,
            restitution_bias: 0.0,
        });
        Self {
            body_a,
            body_b,
            normal,
            points,
            elasticity: None,
        }
    }

    /// Primary contact point (deepest penetration).
    pub fn primary_point(&self) -> &ContactPoint<D> {
        self.points
            .iter()
            .max_by(|a, b| a.depth.total_cmp(&b.depth))
            .unwrap_or(&self.points[0])
    }

    /// Primary penetration depth (deepest point).
    pub fn depth(&self) -> f64 {
        self.primary_point().depth
    }

    /// Primary contact position (deepest point).
    pub fn point(&self) -> SVector<f64, D> {
        self.primary_point().position
    }

    /// Impulse magnitude for elastic collision with given restitution.
    ///
    /// j = -(1 + e) * v_rel · n / (1/m_a + 1/m_b)
    pub fn impulse_magnitude(
        &self,
        relative_velocity: &SVector<f64, D>,
        inv_mass_a: f64,
        inv_mass_b: f64,
        restitution: f64,
    ) -> f64 {
        let v_rel_n = relative_velocity.dot(&self.normal);

        // Separating — no impulse needed
        if v_rel_n > 0.0 {
            return 0.0;
        }

        let denom = inv_mass_a + inv_mass_b;
        if denom < 1e-15 {
            return 0.0;
        }

        -(1.0 + restitution) * v_rel_n / denom
    }
}

/// Cached impulse from a previous frame for warm-starting.
#[derive(Clone, Debug)]
pub struct CachedImpulse<const D: usize> {
    /// Normal impulse magnitude from previous resolution.
    pub normal_impulse: f64,
    /// Tangent impulse magnitude from previous resolution.
    pub tangent_impulse: f64,
    /// Contact point from previous frame (for proximity matching).
    pub point: SVector<f64, D>,
}

/// Cache of previous-frame impulses for warm-starting the contact solver.
///
/// Uses `BTreeMap` for deterministic iteration order (critical for replay).
/// Maps `(body_a, body_b)` → cached impulse data.
pub struct ContactCache<const D: usize> {
    entries: std::collections::BTreeMap<(BodyHandle, BodyHandle), Vec<CachedImpulse<D>>>,
    /// Proximity threshold for matching contacts across frames.
    pub match_threshold: f64,
}

impl<const D: usize> ContactCache<D> {
    pub fn new() -> Self {
        Self {
            entries: std::collections::BTreeMap::new(),
            match_threshold: 2.0,
        }
    }

    /// Store impulses from this frame's contact resolution.
    pub fn store(
        &mut self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        point: SVector<f64, D>,
        normal_impulse: f64,
        tangent_impulse: f64,
    ) {
        let key = if body_a < body_b {
            (body_a, body_b)
        } else {
            (body_b, body_a)
        };
        let entry = self.entries.entry(key).or_default();
        entry.push(CachedImpulse {
            normal_impulse,
            tangent_impulse,
            point,
        });
    }

    /// Look up the cached impulse for a contact point from the previous frame.
    ///
    /// Matches by body pair + proximity of contact point.
    pub fn lookup(
        &self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        point: &SVector<f64, D>,
    ) -> Option<&CachedImpulse<D>> {
        let key = if body_a < body_b {
            (body_a, body_b)
        } else {
            (body_b, body_a)
        };
        let entries = self.entries.get(&key)?;
        // Find the closest cached point within threshold
        entries
            .iter()
            .filter(|c| (c.point - point).norm() < self.match_threshold)
            .min_by(|a, b| {
                let da = (a.point - point).norm();
                let db = (b.point - point).norm();
                da.total_cmp(&db)
            })
    }

    /// Clear all cached data (call at the start of each frame before resolving).
    pub fn begin_frame(&mut self) {
        self.entries.clear();
    }

    /// Number of cached contact pairs.
    pub fn pair_count(&self) -> usize {
        self.entries.len()
    }
}

impl<const D: usize> Default for ContactCache<D> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anisotropic_body_3d() -> crate::body::RigidBody<3> {
        let mut body = crate::body::RigidBody::<3>::dynamic_sphere(
            BodyHandle(70), symtropy_math::Point::origin(), 0.1, 1.0,
        );
        body.inertia = SVector::from([2.0, 3.0, 4.0]);
        body.inv_inertia = SVector::from([0.5, 1.0 / 3.0, 0.25]);
        body
    }

    #[test]
    fn strict_contact_effective_mass_matches_analytic_anisotropic_3d_case() {
        // r = +X and n = +Y yield a +Z impulse; Izz = 4, so response = 1/4.
        let body = anisotropic_body_3d();
        let lever = SVector::from([1.0, 0.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);
        let result = try_angular_contact_effective_mass(&body, &lever, &normal).unwrap();
        assert!((result - 0.25).abs() < 1.0e-12, "effective mass={result}");
    }

    #[test]
    fn strict_contact_effective_mass_supports_full_bivector_operator_4d() {
        let operator = crate::mass_properties::RotationalInertiaOperator::<4>::isotropic(2.0)
            .expect("isotropic 4D operator fixture must be SPD");
        let body = crate::body::RigidBody::<4>::dynamic_sphere(
            BodyHandle(71), symtropy_math::Point::origin(), 0.1, 1.0,
        )
        .with_rotational_inertia_operator(operator);
        let lever = SVector::from([1.0, 0.0, 0.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0, 0.0]);
        let result = try_angular_contact_effective_mass(&body, &lever, &normal).unwrap();
        assert!((result - 0.5).abs() < 1.0e-12, "effective mass={result}");
    }

    #[test]
    fn strict_contact_effective_mass_rejects_unsupported_model() {
        let body = crate::body::RigidBody::<4>::dynamic_sphere(
            BodyHandle(72), symtropy_math::Point::origin(), 0.1, 1.0,
        );
        let result = try_angular_contact_effective_mass(
            &body,
            &SVector::from([1.0, 0.0, 0.0, 0.0]),
            &SVector::from([0.0, 1.0, 0.0, 0.0]),
        );
        assert!(matches!(
            result,
            Err(ContactResponseError::RotationalResponse(
                crate::body::RotationalResponseError::UnsupportedModel { .. }
            ))
        ));
    }

    #[test]
    fn strict_contact_effective_mass_rejects_non_finite_geometry() {
        let body = anisotropic_body_3d();
        let mut normal = SVector::from([0.0, 1.0, 0.0]);
        normal[1] = f64::NAN;
        assert_eq!(
            try_angular_contact_effective_mass(
                &body, &SVector::from([1.0, 0.0, 0.0]), &normal,
            ),
            Err(ContactResponseError::NonFiniteInput)
        );
    }

    #[test]
    fn strict_contact_effective_mass_is_rotation_covariant() {
        let body = anisotropic_body_3d();
        let mut rotated_body = anisotropic_body_3d();
        let frame = symtropy_math::Rotor::<3>::from_plane_angle(
            &symtropy_math::Bivector::<3>::unit_plane(0, 1), 0.47,
        );
        rotated_body.transform.rotation = frame;
        let lever = SVector::from([0.7, -0.2, 0.4]);
        let normal = SVector::from([0.0, 1.0, 0.0]);
        let base = try_angular_contact_effective_mass(&body, &lever, &normal).unwrap();
        let rotated = try_angular_contact_effective_mass(
            &rotated_body, &frame.rotate_vector(&lever), &frame.rotate_vector(&normal),
        )
        .unwrap();
        assert!((base - rotated).abs() < 1.0e-11, "base={base}, rotated={rotated}");
    }
    #[test]
    fn single_point_manifold() {
        let m = ContactManifold::<3>::single(
            BodyHandle(0),
            BodyHandle(1),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([1.0, 0.0, 0.0]),
            0.5,
        );
        assert_eq!(m.points.len(), 1);
        assert!((m.depth() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn multi_point_primary_is_deepest() {
        let mut m = ContactManifold::<3>::single(
            BodyHandle(0),
            BodyHandle(1),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([1.0, 0.0, 0.0]),
            0.3,
        );
        m.points.push(ContactPoint {
            position: SVector::from([2.0, 0.0, 0.0]),
            depth: 0.7,
            lambda: 0.0,
            restitution_bias: 0.0,
        });
        assert_eq!(m.points.len(), 2);
        assert!((m.depth() - 0.7).abs() < 1e-12, "primary should be deepest");
    }

    #[test]
    fn contact_cache_store_and_lookup() {
        let mut cache = ContactCache::<3>::new();
        let pt = SVector::from([1.0, 0.0, 0.0]);
        cache.store(BodyHandle(0), BodyHandle(1), pt, 5.0, 1.0);

        let hit = cache.lookup(BodyHandle(0), BodyHandle(1), &pt);
        assert!(hit.is_some());
        assert!((hit.unwrap().normal_impulse - 5.0).abs() < 1e-12);
    }

    #[test]
    fn contact_cache_proximity_match() {
        let mut cache = ContactCache::<3>::new();
        cache.store(
            BodyHandle(0),
            BodyHandle(1),
            SVector::from([1.0, 0.0, 0.0]),
            5.0,
            1.0,
        );

        // Slightly displaced point should still match
        let nearby = SVector::from([1.5, 0.0, 0.0]);
        let hit = cache.lookup(BodyHandle(0), BodyHandle(1), &nearby);
        assert!(hit.is_some(), "nearby point should match cached contact");

        // Far point should not match
        let far = SVector::from([100.0, 0.0, 0.0]);
        let miss = cache.lookup(BodyHandle(0), BodyHandle(1), &far);
        assert!(miss.is_none(), "far point should not match");
    }

    #[test]
    fn contact_cache_symmetric_keys() {
        let mut cache = ContactCache::<3>::new();
        let pt = SVector::from([0.0, 0.0, 0.0]);
        cache.store(BodyHandle(1), BodyHandle(0), pt, 3.0, 0.5);

        // Lookup with reversed body order should still find it
        let hit = cache.lookup(BodyHandle(0), BodyHandle(1), &pt);
        assert!(hit.is_some(), "cache should be symmetric on body order");
    }

    #[test]
    fn contact_cache_begin_frame_clears() {
        let mut cache = ContactCache::<3>::new();
        cache.store(BodyHandle(0), BodyHandle(1), SVector::zeros(), 1.0, 0.0);
        assert_eq!(cache.pair_count(), 1);

        cache.begin_frame();
        assert_eq!(cache.pair_count(), 0);
    }
}
