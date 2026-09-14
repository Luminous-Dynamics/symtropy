// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Authority-bound instantaneous endpoint-box observations.
//!
//! Endpoint boxes in this tranche are translation-anchored and world-axis-aligned.
//! Anchor rotation is deliberately ignored until a separate proper-frame theorem
//! qualifies oriented/local-frame endpoint regions.

use symtropy_math::{HyperBox, Point};

use crate::authority_observation::AuthorityBodySnapshot;
use crate::identity_authority::{
    PhysicsAuthorityWorld, PhysicsBodySubject, PhysicsIdentityError,
};

/// Exact endpoint-box specification anchored to one live physics subject.
///
/// `center_offset` and `half_extents` are expressed in world axes. The box center
/// at capture time is:
///
/// `anchor.translation + center_offset`
///
/// The target body is represented by its transform origin only. This type does
/// not claim whole-collider containment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AuthorityEndpointBoxSpec<const D: usize> {
    anchor: PhysicsBodySubject,
    center_offset: [u64; D],
    half_extents: [u64; D],
}

impl<const D: usize> AuthorityEndpointBoxSpec<D> {
    pub fn new(
        anchor: PhysicsBodySubject,
        center_offset: [f64; D],
        half_extents: [f64; D],
    ) -> Result<Self, EndpointBoxSpecError> {
        for (axis, value) in center_offset.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(EndpointBoxSpecError::NonFiniteCenterOffset { axis });
            }
        }
        for (axis, value) in half_extents.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(EndpointBoxSpecError::NonFiniteHalfExtent { axis });
            }
            if value <= 0.0 {
                return Err(EndpointBoxSpecError::NonPositiveHalfExtent { axis });
            }
        }

        Ok(Self {
            anchor,
            center_offset: std::array::from_fn(|axis| {
                let value = center_offset[axis];
                if value == 0.0 { 0.0_f64.to_bits() } else { value.to_bits() }
            }),
            half_extents: std::array::from_fn(|axis| half_extents[axis].to_bits()),
        })
    }

    pub const fn anchor(self) -> PhysicsBodySubject {
        self.anchor
    }

    pub fn center_offset(self) -> [f64; D] {
        std::array::from_fn(|axis| f64::from_bits(self.center_offset[axis]))
    }

    pub fn half_extents(self) -> [f64; D] {
        std::array::from_fn(|axis| f64::from_bits(self.half_extents[axis]))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EndpointMembership {
    Outside,
    Inside,
}

/// Instantaneous membership observation for one target body origin relative to
/// one authority-bound endpoint box.
///
/// The observation is handle-free. Both the target and box anchor are captured
/// as authority/generation-bound body snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityEndpointBoxObservation<const D: usize> {
    pub region: AuthorityEndpointBoxSpec<D>,
    pub subject: AuthorityBodySnapshot<D>,
    pub anchor: AuthorityBodySnapshot<D>,
    pub region_center: [u64; D],
    pub offset_from_center: [u64; D],
    pub membership: EndpointMembership,
}

impl<const D: usize> AuthorityEndpointBoxObservation<D> {
    pub fn capture(
        authority: &PhysicsAuthorityWorld<D>,
        subject: PhysicsBodySubject,
        region: AuthorityEndpointBoxSpec<D>,
    ) -> Result<Self, AuthorityEndpointObservationError> {
        let validated_subject = authority
            .validate_subject(subject)
            .map_err(AuthorityEndpointObservationError::Subject)?;
        let validated_anchor = authority
            .validate_subject(region.anchor)
            .map_err(AuthorityEndpointObservationError::Anchor)?;

        if subject == region.anchor {
            return Err(AuthorityEndpointObservationError::SameSubject { subject });
        }

        let center_offset = region.center_offset();
        let half_extents = region.half_extents();
        let mut center = [0.0_f64; D];
        let mut offset = [0.0_f64; D];

        for axis in 0..D {
            let value = validated_anchor.body().transform.translation.0[axis] + center_offset[axis];
            if !value.is_finite() {
                return Err(AuthorityEndpointObservationError::NonFiniteRegionCenter { axis });
            }
            center[axis] = value;

            let relative = validated_subject.body().transform.translation.0[axis] - value;
            if !relative.is_finite() {
                return Err(AuthorityEndpointObservationError::NonFiniteOffsetFromCenter { axis });
            }
            offset[axis] = relative;
        }

        let membership = if HyperBox::<D>::new(half_extents).contains_local(&Point::new(offset)) {
            EndpointMembership::Inside
        } else {
            EndpointMembership::Outside
        };

        Ok(Self {
            region,
            subject: AuthorityBodySnapshot::from_validated(&validated_subject),
            anchor: AuthorityBodySnapshot::from_validated(&validated_anchor),
            region_center: std::array::from_fn(|axis| center[axis].to_bits()),
            offset_from_center: std::array::from_fn(|axis| offset[axis].to_bits()),
            membership,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EndpointBoxSpecError {
    NonFiniteCenterOffset { axis: usize },
    NonFiniteHalfExtent { axis: usize },
    NonPositiveHalfExtent { axis: usize },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthorityEndpointObservationError {
    Subject(PhysicsIdentityError),
    Anchor(PhysicsIdentityError),
    SameSubject { subject: PhysicsBodySubject },
    NonFiniteRegionCenter { axis: usize },
    NonFiniteOffsetFromCenter { axis: usize },
}
