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
    PhysicsAuthorityWorld, PhysicsBodySubject, PhysicsIdentityError,
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

        Ok(Self {
            subject: validated.subject(),
            body_type: body.body_type,
            translation,
            rotation,
            linear_velocity,
            angular_velocity,
            sleeping: body.sleeping,
            sleep_counter: body.sleep_counter,
        })
    }
}
