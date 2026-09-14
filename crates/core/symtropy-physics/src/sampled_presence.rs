// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact consecutive sampled endpoint presence.
//!
//! This module proves a relation between two already-issued PHYS-OBS-05 endpoint
//! samples. It deliberately remains a sampled-state theorem: adjacent `Inside`
//! endpoint samples do not prove continuous occupancy between those samples.

use crate::authority_endpoint::{AuthorityEndpointBoxSpec, EndpointMembership};
use crate::evidence_authority::{
    LocalQualifiedAuthorityStepStamp, LocalTemporalIncarnationId,
};
use crate::identity_authority::{PhysicalAuthorityId, PhysicsBodySubject, WorldGenerationId};
use crate::qualified_endpoint::LocalQualifiedStampedEndpointBoxObservation;

/// Opaque proof that two issued endpoint observations are the same exact
/// proposition and are `Inside` at exactly adjacent qualified step indexes.
///
/// The input observations are borrowed rather than consumed. This permits
/// overlapping chains such as `N -> N+1` and `N+1 -> N+2` without cloning the
/// non-cloneable issued PHYS-OBS-05 tokens.
///
/// This proof is intentionally not `Clone`, `Copy`, or serializable authority.
#[derive(Debug, PartialEq, Eq)]
pub struct LocalConsecutiveSampledEndpointPresence<const D: usize> {
    previous_stamp: LocalQualifiedAuthorityStepStamp,
    current_stamp: LocalQualifiedAuthorityStepStamp,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<D>,
}

impl<const D: usize> LocalConsecutiveSampledEndpointPresence<D> {
    /// Verify exact adjacent sampled presence from two opaque PHYS-OBS-05 tokens.
    ///
    /// No raw endpoint record or detached stamp is accepted. Construction fails
    /// closed on lineage mismatch, proposition mismatch, non-`Inside`
    /// membership, duplicate sampling, reversed order, or a skipped step.
    pub fn try_from_samples(
        previous: &LocalQualifiedStampedEndpointBoxObservation<D>,
        current: &LocalQualifiedStampedEndpointBoxObservation<D>,
    ) -> Result<Self, LocalConsecutiveSampledPresenceError> {
        let previous_stamp = previous.stamp();
        let current_stamp = current.stamp();

        if previous_stamp.physical_authority_id() != current_stamp.physical_authority_id() {
            return Err(LocalConsecutiveSampledPresenceError::PhysicalAuthorityMismatch {
                previous: previous_stamp.physical_authority_id(),
                current: current_stamp.physical_authority_id(),
            });
        }
        if previous_stamp.world_generation_id() != current_stamp.world_generation_id() {
            return Err(LocalConsecutiveSampledPresenceError::WorldGenerationMismatch {
                previous: previous_stamp.world_generation_id(),
                current: current_stamp.world_generation_id(),
            });
        }
        if previous_stamp.temporal_incarnation_id() != current_stamp.temporal_incarnation_id() {
            return Err(LocalConsecutiveSampledPresenceError::TemporalIncarnationMismatch {
                previous: previous_stamp.temporal_incarnation_id(),
                current: current_stamp.temporal_incarnation_id(),
            });
        }

        let previous_observation = previous.observation();
        let current_observation = current.observation();
        let previous_subject = previous_observation.subject.subject;
        let current_subject = current_observation.subject.subject;

        if previous_subject != current_subject {
            return Err(LocalConsecutiveSampledPresenceError::TargetMismatch {
                previous: previous_subject,
                current: current_subject,
            });
        }
        if previous_observation.region != current_observation.region {
            return Err(LocalConsecutiveSampledPresenceError::RegionMismatch);
        }
        if previous_observation.membership != EndpointMembership::Inside {
            return Err(LocalConsecutiveSampledPresenceError::PreviousOutside);
        }
        if current_observation.membership != EndpointMembership::Inside {
            return Err(LocalConsecutiveSampledPresenceError::CurrentOutside);
        }

        let previous_step = previous_stamp.step_index();
        let current_step = current_stamp.step_index();
        let step_delta = current_step.checked_sub(previous_step).ok_or(
            LocalConsecutiveSampledPresenceError::ReversedStep {
                previous_step,
                current_step,
            },
        )?;
        match step_delta {
            0 => {
                return Err(LocalConsecutiveSampledPresenceError::DuplicateStep {
                    step_index: previous_step,
                });
            }
            1 => {}
            _ => {
                return Err(LocalConsecutiveSampledPresenceError::StepGap {
                    previous_step,
                    current_step,
                });
            }
        }

        Ok(Self {
            previous_stamp,
            current_stamp,
            subject: previous_subject,
            region: previous_observation.region,
        })
    }

    pub const fn previous_stamp(&self) -> LocalQualifiedAuthorityStepStamp {
        self.previous_stamp
    }

    pub const fn current_stamp(&self) -> LocalQualifiedAuthorityStepStamp {
        self.current_stamp
    }

    pub const fn subject(&self) -> PhysicsBodySubject {
        self.subject
    }

    pub const fn region(&self) -> AuthorityEndpointBoxSpec<D> {
        self.region
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocalConsecutiveSampledPresenceError {
    PhysicalAuthorityMismatch {
        previous: PhysicalAuthorityId,
        current: PhysicalAuthorityId,
    },
    WorldGenerationMismatch {
        previous: WorldGenerationId,
        current: WorldGenerationId,
    },
    TemporalIncarnationMismatch {
        previous: LocalTemporalIncarnationId,
        current: LocalTemporalIncarnationId,
    },
    TargetMismatch {
        previous: PhysicsBodySubject,
        current: PhysicsBodySubject,
    },
    RegionMismatch,
    PreviousOutside,
    CurrentOutside,
    DuplicateStep {
        step_index: u64,
    },
    ReversedStep {
        previous_step: u64,
        current_step: u64,
    },
    StepGap {
        previous_step: u64,
        current_step: u64,
    },
}
