// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact execution-duration binding for one already-qualified sampled-presence transition.
//!
//! This module is intentionally a semantic join only. PHYS-OBS-06 owns endpoint
//! proposition equality, Inside membership, lineage equality, and exact step
//! adjacency. PHYS-OBS-07 owns successful-step execution provenance and exact
//! binary64 `dt` bits. This layer only proves that the receipt belongs to the
//! current endpoint of the already-qualified relation.

use crate::authority_endpoint::AuthorityEndpointBoxSpec;
use crate::evidence_authority::LocalQualifiedAuthorityStepStamp;
use crate::identity_authority::PhysicsBodySubject;
use crate::sampled_presence::LocalConsecutiveSampledEndpointPresence;
use crate::timed_evidence::LocalQualifiedStepExecutionReceipt;

/// Opaque proof that one exact consecutive sampled-presence transition is bound
/// to the exact executed `dt` receipt for the transition's current step.
///
/// This token is intentionally non-`Clone`, non-`Copy`, and non-Serde. It does
/// not own or recreate either predecessor proof object; it stores only the
/// semantic facts already validated by those opaque predecessors.
#[derive(Debug, PartialEq, Eq)]
pub struct LocalExactTimedSampledPresenceTransition<const D: usize> {
    previous_stamp: LocalQualifiedAuthorityStepStamp,
    current_stamp: LocalQualifiedAuthorityStepStamp,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<D>,
    dt_bits: u64,
}

impl<const D: usize> LocalExactTimedSampledPresenceTransition<D> {
    /// Bind an already-qualified `N -> N+1` sampled-presence relation to the
    /// exact execution receipt whose full strong stamp is exactly `N+1`.
    ///
    /// Receipt `N` is deliberately rejected: it produced the first post-step
    /// sample and cannot be retroactively credited as time already known Inside.
    pub fn try_from_relation_and_receipt(
        relation: &LocalConsecutiveSampledEndpointPresence<D>,
        receipt: &LocalQualifiedStepExecutionReceipt,
    ) -> Result<Self, LocalExactTimedSampledPresenceTransitionError> {
        let expected_current = relation.current_stamp();
        let actual_receipt = receipt.stamp();
        if actual_receipt != expected_current {
            return Err(
                LocalExactTimedSampledPresenceTransitionError::ReceiptStampMismatch {
                    expected_current,
                    actual_receipt,
                },
            );
        }

        Ok(Self {
            previous_stamp: relation.previous_stamp(),
            current_stamp: expected_current,
            subject: relation.subject(),
            region: relation.region(),
            dt_bits: receipt.dt_bits(),
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

    /// Exact IEEE-754 binary64 bits of the successful physics step that ended at
    /// `current_stamp`.
    pub const fn dt_bits(&self) -> u64 {
        self.dt_bits
    }

    /// Convenience reconstruction of the exact binary64 value. This is not a
    /// cumulative clock and performs no arithmetic.
    pub fn executed_dt(&self) -> f64 {
        f64::from_bits(self.dt_bits)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocalExactTimedSampledPresenceTransitionError {
    /// The execution receipt is not the exact full strong step that terminates
    /// the supplied sampled-presence relation.
    ReceiptStampMismatch {
        expected_current: LocalQualifiedAuthorityStepStamp,
        actual_receipt: LocalQualifiedAuthorityStepStamp,
    },
}
