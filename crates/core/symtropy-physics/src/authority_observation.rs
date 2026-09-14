// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Authority-bound selected-subject physics observations.
//!
//! This module deliberately captures only explicitly named live subjects that
//! first pass [`PhysicsAuthorityWorld::validate_subject`]. It does not claim a
//! complete-world snapshot, historical continuity, trajectory, route, arrival,
//! or delivery evidence.

use crate::body::BodyType;
use crate::identity_authority::{
    PhysicsAuthorityWorld, PhysicsBodySubject, PhysicsIdentityError, ValidatedNetBody,
};

/// Bitwise snapshot of one exact authority/generation-bound live body.
///
/// Unlike the runtime replay [`crate::replay::BodySnapshot`], this type carries
/// no `BodyHandle`. The subject is the physics-owned
/// `(PhysicalAuthorityId, WorldGenerationId, NetId)` identity tuple; the
/// runtime handle is resolved only transiently while validating and capturing
/// the body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityBodySnapshot<const D: usize> {
    pub subject: PhysicsBodySubject,
    pub body_type: BodyType,
    pub translation: [u64; D],
    pub rotation: [[u64; D]; D],
    pub linear_velocity: [u64; D],
    pub angular_velocity: [[u64; D]; D],
    pub sleeping: bool,
    pub sleep_counter: u32,
}

impl<const D: usize> AuthorityBodySnapshot<D> {
    /// Capture one explicitly named subject after exact authority/generation
    /// and live identity validation.
    ///
    /// Failure occurs before a snapshot is returned. The resolver never falls
    /// back to a coincidentally equal runtime handle.
    pub fn capture(
        authority: &PhysicsAuthorityWorld<D>,
        subject: PhysicsBodySubject,
    ) -> Result<Self, PhysicsIdentityError> {
        let validated = authority.validate_subject(subject)?;
        Ok(Self::from_validated(&validated))
    }

    fn from_validated(validated: &ValidatedNetBody<'_, D>) -> Self {
        let body = validated.body();
        let translation = std::array::from_fn(|i| body.transform.translation.0[i].to_bits());
        let rotation_matrix = body.transform.rotation.to_matrix();
        let rotation = std::array::from_fn(|row| {
            std::array::from_fn(|column| rotation_matrix[(row, column)].to_bits())
        });
        let linear_velocity = std::array::from_fn(|i| body.linear_velocity[i].to_bits());
        let angular_matrix = body.angular_velocity.to_matrix();
        let angular_velocity = std::array::from_fn(|row| {
            std::array::from_fn(|column| angular_matrix[(row, column)].to_bits())
        });

        Self {
            subject: validated.subject(),
            body_type: body.body_type,
            translation,
            rotation,
            linear_velocity,
            angular_velocity,
            sleeping: body.sleeping,
            sleep_counter: body.sleep_counter,
        }
    }
}

/// Current-state observation of two distinct authority-bound subjects under one
/// immutable authority-world borrow.
///
/// This captures both handle-free body snapshots plus the exact current
/// displacement from A to B and squared separation. It is a simultaneous
/// current-state relation in the ordinary safe-Rust borrowing sense: no mutable
/// access to the authority world can coexist with the borrow used for capture.
/// It is not a trajectory, route, arrival, or historical observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityPairSnapshot<const D: usize> {
    pub body_a: AuthorityBodySnapshot<D>,
    pub body_b: AuthorityBodySnapshot<D>,
    pub relative_translation: [u64; D],
    pub separation_squared: u64,
}

impl<const D: usize> AuthorityPairSnapshot<D> {
    pub fn capture(
        authority: &PhysicsAuthorityWorld<D>,
        subject_a: PhysicsBodySubject,
        subject_b: PhysicsBodySubject,
    ) -> Result<Self, AuthorityPairObservationError> {
        if subject_a == subject_b {
            return Err(AuthorityPairObservationError::SameSubject { subject: subject_a });
        }

        let validated_a = authority
            .validate_subject(subject_a)
            .map_err(AuthorityPairObservationError::SubjectA)?;
        let validated_b = authority
            .validate_subject(subject_b)
            .map_err(AuthorityPairObservationError::SubjectB)?;

        let delta =
            validated_b.body().transform.translation.0 - validated_a.body().transform.translation.0;
        for (axis, value) in delta.iter().enumerate() {
            if !value.is_finite() {
                return Err(AuthorityPairObservationError::NonFiniteRelativeTranslation { axis });
            }
        }

        let separation_squared_value = delta.norm_squared();
        if !separation_squared_value.is_finite() {
            return Err(AuthorityPairObservationError::NonFiniteSeparationSquared);
        }

        let relative_translation = std::array::from_fn(|i| delta[i].to_bits());
        let separation_squared = separation_squared_value.to_bits();

        Ok(Self {
            body_a: AuthorityBodySnapshot::from_validated(&validated_a),
            body_b: AuthorityBodySnapshot::from_validated(&validated_b),
            relative_translation,
            separation_squared,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthorityPairObservationError {
    SameSubject { subject: PhysicsBodySubject },
    SubjectA(PhysicsIdentityError),
    SubjectB(PhysicsIdentityError),
    NonFiniteRelativeTranslation { axis: usize },
    NonFiniteSeparationSquared,
}
