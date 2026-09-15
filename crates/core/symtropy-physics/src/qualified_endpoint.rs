// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Opaque, process-local, temporally qualified endpoint evidence.
//!
//! This layer composes the sealed evidence-authority theorem with the existing
//! PHYS-OBS-03 endpoint classifier. It adds provenance and temporal attribution;
//! it does not reimplement endpoint geometry or promote sampled state into
//! continuous occupancy, dwell, arrival, custody, delivery, or settlement.

use crate::authority_endpoint::{
    AuthorityEndpointBoxObservation, AuthorityEndpointBoxSpec, AuthorityEndpointObservationError,
};
use crate::evidence_authority::{
    LocalEvidencePhysicsAuthorityWorld, LocalQualifiedAuthorityStepStamp,
};
use crate::identity_authority::PhysicsBodySubject;

/// Opaque authority-issued endpoint token for one current qualified step.
///
/// The inner PHYS-OBS-03 record remains a plain record type with public fields.
/// This wrapper is stronger: its fields are private, it has no public detached
/// constructor, and capture obtains the qualified step stamp internally from the
/// same sealed authority facade used for qualified subject validation and the
/// endpoint observation.
///
/// This token is intentionally not `Clone`, `Copy`, or Serde-authoritative.
/// Repeated legitimate captures at one step may yield equal tokens, but they are
/// duplicate same-step evidence rather than additional temporal samples.
#[derive(Debug, PartialEq, Eq)]
pub struct LocalQualifiedStampedEndpointBoxObservation<const D: usize> {
    stamp: LocalQualifiedAuthorityStepStamp,
    observation: AuthorityEndpointBoxObservation<D>,
}

impl<const D: usize> LocalQualifiedStampedEndpointBoxObservation<D> {
    /// Capture a temporally qualified endpoint observation from one sealed
    /// evidence-authority facade.
    ///
    /// No caller-supplied stamp is accepted. Both the target and anchor first
    /// pass the qualified live-subject path; the existing PHYS-OBS-03 capture
    /// then owns all endpoint geometry and snapshot semantics.
    pub fn capture(
        authority: &LocalEvidencePhysicsAuthorityWorld<D>,
        subject: PhysicsBodySubject,
        region: AuthorityEndpointBoxSpec<D>,
    ) -> Result<Self, LocalQualifiedStampedEndpointObservationError> {
        let stamp = authority
            .last_qualified_step_stamp()
            .ok_or(LocalQualifiedStampedEndpointObservationError::NoQualifiedStep)?;

        authority.validate_subject(subject).map_err(|error| {
            LocalQualifiedStampedEndpointObservationError::Endpoint(
                AuthorityEndpointObservationError::Subject(error),
            )
        })?;
        authority.validate_subject(region.anchor()).map_err(|error| {
            LocalQualifiedStampedEndpointObservationError::Endpoint(
                AuthorityEndpointObservationError::Anchor(error),
            )
        })?;

        let observation = AuthorityEndpointBoxObservation::capture(
            authority.authority_world(),
            subject,
            region,
        )
        .map_err(LocalQualifiedStampedEndpointObservationError::Endpoint)?;

        Ok(Self { stamp, observation })
    }

    pub const fn stamp(&self) -> LocalQualifiedAuthorityStepStamp {
        self.stamp
    }

    /// Read-only downward projection to the weaker PHYS-OBS-03 record.
    pub fn observation(&self) -> &AuthorityEndpointBoxObservation<D> {
        &self.observation
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocalQualifiedStampedEndpointObservationError {
    /// No normally completed qualified step is current in this sealed temporal
    /// incarnation.
    NoQualifiedStep,
    /// Existing PHYS-OBS-03 identity or geometry rejection.
    Endpoint(AuthorityEndpointObservationError),
}
