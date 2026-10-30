// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Image-aware warm-start cache and contact-frame geometry.
//!
//! The Euclidean ContactCache is keyed only by body pair. A periodic image is
//! part of contact identity, so this cache uses a canonical body-pair/image key.
//!
//! This adapter is not yet integrated into PhysicsWorld. Its canonical point
//! convention shifts the entire contact into the coordinate frame of the lower
//! BodyHandle so reversing body order and reversing the relative image offset
//! identifies the same cached geometry.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox, PeriodicDomainError};

use crate::body::BodyHandle;
use crate::periodic_contact::{PeriodicContactKey, PeriodicContactKeyError};

/// Validated contact-frame data for one actual pair ordering.
///
/// lever_arm_a and lever_arm_b are computed against the actual image transforms:
/// B's centre is translated by image_offset_b_relative_to_a. They are therefore
/// suitable for torque/effective-mass calculations without a one-box-length
/// lever-arm error.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicContactGeometry<const D: usize> {
    pub key: PeriodicContactKey<D>,
    /// Contact point expressed in the coordinate frame of canonical body_a.
    pub canonical_point: SVector<f64, D>,
    /// Contact point minus A's unshifted centre.
    pub lever_arm_a: SVector<f64, D>,
    /// Contact point minus B's image-translated centre.
    pub lever_arm_b: SVector<f64, D>,
    /// Translation applied to B for this actual A/B ordering.
    pub image_translation: SVector<f64, D>,
}

/// Warm-start impulse stored in canonical body order. The tangent impulse is
/// a world-space vector applied to the canonical higher-ID body, not a scalar
/// tied to whichever tangent basis the narrow phase happened to choose.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicCachedImpulse<const D: usize> {
    pub normal_impulse: f64,
    pub tangent_impulse: SVector<f64, D>,
    pub point: SVector<f64, D>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicContactCacheError {
    Key(PeriodicContactKeyError),
    Domain(PeriodicDomainError),
    NonFiniteContactPoint { axis: usize },
    NonFiniteBodyCentre { body: &'static str, axis: usize },
    NonFiniteImpulse,
    InvalidNormalImpulse,
    NonFiniteDistance,
    InvalidMatchThreshold,
}

impl fmt::Display for PeriodicContactCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(error) => write!(f, "periodic contact key error: {error}"),
            Self::Domain(error) => write!(f, "periodic contact domain error: {error}"),
            Self::NonFiniteContactPoint { axis } => {
                write!(f, "non-finite contact point on axis {axis}")
            }
            Self::NonFiniteBodyCentre { body, axis } => {
                write!(f, "non-finite centre of body {body} on axis {axis}")
            }
            Self::NonFiniteImpulse => write!(f, "periodic cached impulse must be finite"),
            Self::InvalidNormalImpulse => {
                write!(f, "periodic cached normal impulse must be non-negative")
            }
            Self::NonFiniteDistance => write!(f, "periodic contact matching distance is non-finite"),
            Self::InvalidMatchThreshold => {
                write!(f, "periodic contact cache match threshold must be finite and positive")
            }
        }
    }
}

impl Error for PeriodicContactCacheError {}

impl From<PeriodicContactKeyError> for PeriodicContactCacheError {
    fn from(value: PeriodicContactKeyError) -> Self {
        Self::Key(value)
    }
}

impl From<PeriodicDomainError> for PeriodicContactCacheError {
    fn from(value: PeriodicDomainError) -> Self {
        Self::Domain(value)
    }
}

/// Build image-aware lever arms and the canonical contact-frame point.
///
/// Convention: image_offset_b_relative_to_a translates body B relative to body
/// A before the narrow phase. If the caller reverses body order, it supplies
/// the negated offset and the corresponding contact point in that image frame.
/// The resulting key and canonical point are equal.
pub fn periodic_contact_geometry<const D: usize>(
    domain: &PeriodicBox<D>,
    body_a: BodyHandle,
    centre_a: &SVector<f64, D>,
    body_b: BodyHandle,
    centre_b: &SVector<f64, D>,
    image_offset_b_relative_to_a: LatticeOffset<D>,
    contact_point: &SVector<f64, D>,
) -> Result<PeriodicContactGeometry<D>, PeriodicContactCacheError> {
    validate_geometry_inputs(centre_a, centre_b, contact_point)?;

    let key = PeriodicContactKey::canonical(
        body_a,
        body_b,
        image_offset_b_relative_to_a,
    )?;
    let image_translation = domain.offset_vector(image_offset_b_relative_to_a)?;
    let image_centre_b = centre_b + image_translation;
    let lever_arm_a = contact_point - centre_a;
    let lever_arm_b = contact_point - image_centre_b;

    if !image_centre_b.iter().all(|value| value.is_finite())
        || !lever_arm_a.iter().all(|value| value.is_finite())
        || !lever_arm_b.iter().all(|value| value.is_finite())
    {
        let axis = (0..D)
            .find(|axis| {
                !image_centre_b[*axis].is_finite()
                    || !lever_arm_a[*axis].is_finite()
                    || !lever_arm_b[*axis].is_finite()
            })
            .unwrap_or(0);
        return Err(PeriodicContactCacheError::Domain(
            PeriodicDomainError::InvalidImageOffset { axis },
        ));
    }

    let canonical_point = canonical_contact_point(
        domain,
        body_a,
        body_b,
        image_offset_b_relative_to_a,
        contact_point,
    )?;

    Ok(PeriodicContactGeometry {
        key,
        canonical_point,
        lever_arm_a,
        lever_arm_b,
        image_translation,
    })
}

fn validate_geometry_inputs<const D: usize>(
    centre_a: &SVector<f64, D>,
    centre_b: &SVector<f64, D>,
    contact_point: &SVector<f64, D>,
) -> Result<(), PeriodicContactCacheError> {
    for axis in 0..D {
        if !centre_a[axis].is_finite() {
            return Err(PeriodicContactCacheError::NonFiniteBodyCentre {
                body: "A",
                axis,
            });
        }
        if !centre_b[axis].is_finite() {
            return Err(PeriodicContactCacheError::NonFiniteBodyCentre {
                body: "B",
                axis,
            });
        }
        if !contact_point[axis].is_finite() {
            return Err(PeriodicContactCacheError::NonFiniteContactPoint { axis });
        }
    }
    Ok(())
}

fn canonical_contact_point<const D: usize>(
    domain: &PeriodicBox<D>,
    body_a: BodyHandle,
    body_b: BodyHandle,
    image_offset_b_relative_to_a: LatticeOffset<D>,
    contact_point: &SVector<f64, D>,
) -> Result<SVector<f64, D>, PeriodicContactCacheError> {
    // Validate the full lattice offset even when the point is already in the
    // canonical body ordering. Non-periodic axes must never carry image shifts.
    let shift = domain.offset_vector(image_offset_b_relative_to_a)?;
    if body_a < body_b {
        Ok(*contact_point)
    } else if body_a > body_b {
        // Swapping A/B and reversing n translates both members of the physical
        // contact by -nL. Shift the contact witness by the same amount so
        // matching remains in one deterministic coordinate frame.
        let canonical = contact_point - shift;
        if canonical.iter().all(|value| value.is_finite()) {
            Ok(canonical)
        } else {
            let axis = canonical.iter().position(|value| !value.is_finite()).unwrap_or(0);
            Err(PeriodicContactCacheError::NonFiniteContactPoint { axis })
        }
    } else {
        Err(PeriodicContactKeyError::SameBody.into())
    }
}

/// Deterministic warm-start cache keyed by canonical body pair and periodic
/// image. It deliberately remains separate from the Euclidean ContactCache.
pub struct PeriodicContactCache<const D: usize> {
    entries: BTreeMap<(BodyHandle, BodyHandle, [i64; D]), Vec<PeriodicCachedImpulse<D>>>,
    /// Proximity threshold for matching canonical contact points.
    pub match_threshold: f64,
}

impl<const D: usize> PeriodicContactCache<D> {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            match_threshold: 2.0,
        }
    }

    /// Store a contact impulse. The point is canonicalized into the lower
    /// BodyHandle's frame before proximity matching on a later frame.
    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &mut self,
        domain: &PeriodicBox<D>,
        body_a: BodyHandle,
        body_b: BodyHandle,
        image_offset_b_relative_to_a: LatticeOffset<D>,
        point: &SVector<f64, D>,
        normal_impulse: f64,
        tangent_impulse: &SVector<f64, D>,
    ) -> Result<(), PeriodicContactCacheError> {
        self.validate_threshold()?;
        if !normal_impulse.is_finite() || !tangent_impulse.iter().all(|value| value.is_finite()) {
            return Err(PeriodicContactCacheError::NonFiniteImpulse);
        }
        if normal_impulse < 0.0 {
            return Err(PeriodicContactCacheError::InvalidNormalImpulse);
        }

        let key = PeriodicContactKey::canonical(
            body_a,
            body_b,
            image_offset_b_relative_to_a,
        )?;
        let canonical_point = canonical_contact_point(
            domain,
            body_a,
            body_b,
            image_offset_b_relative_to_a,
            point,
        )?;
        let canonical_tangent_impulse = if body_a < body_b {
            *tangent_impulse
        } else {
            -*tangent_impulse
        };

        self.entries
            .entry((key.body_a, key.body_b, key.image_offset.periods))
            .or_default()
            .push(PeriodicCachedImpulse {
                normal_impulse,
                tangent_impulse: canonical_tangent_impulse,
                point: canonical_point,
            });
        Ok(())
    }

    /// Lookup a previous-frame impulse using body pair, image identity, and
    /// proximity in the canonical coordinate frame.
    pub fn lookup(
        &self,
        domain: &PeriodicBox<D>,
        body_a: BodyHandle,
        body_b: BodyHandle,
        image_offset_b_relative_to_a: LatticeOffset<D>,
        point: &SVector<f64, D>,
    ) -> Result<Option<&PeriodicCachedImpulse<D>>, PeriodicContactCacheError> {
        self.validate_threshold()?;
        let axis = point.iter().position(|value| !value.is_finite());
        if let Some(axis) = axis {
            return Err(PeriodicContactCacheError::NonFiniteContactPoint { axis });
        }

        let key = PeriodicContactKey::canonical(
            body_a,
            body_b,
            image_offset_b_relative_to_a,
        )?;
        let canonical_point = canonical_contact_point(
            domain,
            body_a,
            body_b,
            image_offset_b_relative_to_a,
            point,
        )?;
        let Some(entries) = self
            .entries
            .get(&(key.body_a, key.body_b, key.image_offset.periods))
        else {
            return Ok(None);
        };

        let mut nearest: Option<(&PeriodicCachedImpulse<D>, f64)> = None;
        for cached in entries {
            let distance = (cached.point - canonical_point).norm();
            if !distance.is_finite() {
                return Err(PeriodicContactCacheError::NonFiniteDistance);
            }
            if distance >= self.match_threshold {
                continue;
            }
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((cached, distance));
            }
        }
        Ok(nearest.map(|(cached, _)| cached))
    }

    pub fn begin_frame(&mut self) {
        self.entries.clear();
    }

    /// Number of distinct body-pair/image identities cached.
    pub fn image_pair_count(&self) -> usize {
        self.entries.len()
    }

    /// Number of individual cached contact witnesses.
    pub fn contact_count(&self) -> usize {
        self.entries.values().map(Vec::len).sum()
    }

    fn validate_threshold(&self) -> Result<(), PeriodicContactCacheError> {
        if !self.match_threshold.is_finite() || self.match_threshold <= 0.0 {
            return Err(PeriodicContactCacheError::InvalidMatchThreshold);
        }
        Ok(())
    }
}

impl<const D: usize> Default for PeriodicContactCache<D> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PeriodicContactCache, PeriodicContactCacheError, periodic_contact_geometry,
    };
    use crate::body::BodyHandle;
    use nalgebra::SVector;
    use symtropy_math::{LatticeOffset, PeriodicBox, PeriodicDomainError};

    fn domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap()
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.15e} ~= {b:.15e}"
        );
    }

    #[test]
    fn distinct_periodic_images_never_share_warm_start_entries() {
        let domain = domain();
        let mut cache = PeriodicContactCache::<2>::new();
        let body_a = BodyHandle(2);
        let body_b = BodyHandle(7);
        let point = SVector::from([0.0, 0.0]);

        cache
            .store(
                &domain,
                body_a,
                body_b,
                LatticeOffset::new([-1, 0]),
                &point,
                3.0,
                &SVector::from([0.0, 0.25]),
            )
            .unwrap();
        cache
            .store(
                &domain,
                body_a,
                body_b,
                LatticeOffset::zero(),
                &point,
                7.0,
                &SVector::from([0.0, 0.75]),
            )
            .unwrap();

        assert_eq!(cache.image_pair_count(), 2);
        assert_eq!(cache.contact_count(), 2);
        assert_eq!(
            cache
                .lookup(
                    &domain,
                    body_a,
                    body_b,
                    LatticeOffset::new([-1, 0]),
                    &point,
                )
                .unwrap()
                .unwrap()
                .normal_impulse,
            3.0
        );
        assert_eq!(
            cache
                .lookup(
                    &domain,
                    body_a,
                    body_b,
                    LatticeOffset::zero(),
                    &point,
                )
                .unwrap()
                .unwrap()
                .normal_impulse,
            7.0
        );
    }

    #[test]
    fn reversed_body_order_and_offset_map_to_same_cached_contact() {
        let domain = domain();
        let mut cache = PeriodicContactCache::<2>::new();
        let lower = BodyHandle(2);
        let upper = BodyHandle(7);

        let point_ab = SVector::from([0.0, 0.0]);
        cache
            .store(
                &domain,
                lower,
                upper,
                LatticeOffset::new([-1, 0]),
                &point_ab,
                4.0,
                &SVector::from([0.0, -0.5]),
            )
            .unwrap();

        let point_ba = SVector::from([10.0, 0.0]);
        let cached = cache
            .lookup(
                &domain,
                upper,
                lower,
                LatticeOffset::new([1, 0]),
                &point_ba,
            )
            .unwrap()
            .unwrap();

        assert_eq!(cached.normal_impulse, 4.0);
        assert_eq!(cached.tangent_impulse, SVector::from([0.0, -0.5]));
        assert_eq!(cached.point, point_ab);
    }

    #[test]
    fn lever_arms_use_the_translated_image_centre() {
        let domain = domain();
        let centre_a = SVector::from([0.5, 0.0]);
        let centre_b = SVector::from([9.0, 0.0]);
        let point = SVector::from([0.0, 0.0]);

        let geometry = periodic_contact_geometry(
            &domain,
            BodyHandle(2),
            &centre_a,
            BodyHandle(7),
            &centre_b,
            LatticeOffset::new([-1, 0]),
            &point,
        )
        .unwrap();

        close(geometry.image_translation[0], -10.0, 1e-12);
        close(geometry.lever_arm_a[0], -0.5, 1e-12);
        close(geometry.lever_arm_b[0], 1.0, 1e-12);
        assert_eq!(geometry.canonical_point, point);

        let reverse_point = SVector::from([10.0, 0.0]);
        let reversed = periodic_contact_geometry(
            &domain,
            BodyHandle(7),
            &centre_b,
            BodyHandle(2),
            &centre_a,
            LatticeOffset::new([1, 0]),
            &reverse_point,
        )
        .unwrap();
        assert_eq!(reversed.key, geometry.key);
        assert_eq!(reversed.canonical_point, geometry.canonical_point);
        close(reversed.lever_arm_a[0], geometry.lever_arm_b[0], 1e-12);
        close(reversed.lever_arm_b[0], geometry.lever_arm_a[0], 1e-12);
    }

    #[test]
    fn nonperiodic_image_axis_is_rejected() {
        let domain = domain();
        let result = periodic_contact_geometry(
            &domain,
            BodyHandle(2),
            &SVector::zeros(),
            BodyHandle(7),
            &SVector::zeros(),
            LatticeOffset::new([0, 1]),
            &SVector::zeros(),
        );
        assert_eq!(
            result,
            Err(PeriodicContactCacheError::Domain(
                PeriodicDomainError::OffsetOnNonPeriodicAxis { axis: 1 }
            ))
        );
    }

    #[test]
    fn invalid_cache_threshold_and_nonfinite_impulse_fail_closed() {
        let domain = domain();
        let mut cache = PeriodicContactCache::<2>::new();
        cache.match_threshold = f64::NAN;
        assert!(matches!(
            cache.lookup(
                &domain,
                BodyHandle(2),
                BodyHandle(7),
                LatticeOffset::zero(),
                &SVector::zeros(),
            ),
            Err(PeriodicContactCacheError::InvalidMatchThreshold)
        ));

        cache.match_threshold = 2.0;
        assert!(matches!(
            cache.store(
                &domain,
                BodyHandle(2),
                BodyHandle(7),
                LatticeOffset::zero(),
                &SVector::zeros(),
                f64::INFINITY,
                &SVector::zeros(),
            ),
            Err(PeriodicContactCacheError::NonFiniteImpulse)
        ));
    }

    #[test]
    fn frame_reset_clears_all_image_keys() {
        let domain = domain();
        let mut cache = PeriodicContactCache::<2>::new();
        cache
            .store(
                &domain,
                BodyHandle(2),
                BodyHandle(7),
                LatticeOffset::new([-1, 0]),
                &SVector::zeros(),
                1.0,
                &SVector::zeros(),
            )
            .unwrap();
        cache.begin_frame();
        assert_eq!(cache.image_pair_count(), 0);
        assert_eq!(cache.contact_count(), 0);
    }
}
