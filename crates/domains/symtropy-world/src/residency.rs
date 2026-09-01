// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic representation residency and domain-approved release.
//!
//! Adaptive fidelity may request more detail, but detail must not disappear merely
//! because a timer elapsed. This module provides the complementary release gate:
//! a representation remains resident through a deterministic simulation-time
//! threshold, after which the owning domain must issue a fresh permit bound to
//! the current state digest before any replacement representation may be attempted.
//! Actual representation transfer and conservation/equivalence proof remain the
//! owning domain's responsibility via `RepresentationTransferReceipt`.

use std::{error::Error, fmt};

use symtropy_sim_contracts::{
    AuthorityId, ContractError, RepresentationId, ScopeId, SimInstant, TypedDigest32,
};

/// The representation currently active for one authority/scope pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveRepresentation {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub representation: RepresentationId,
    pub activated_at: SimInstant,
    pub state_digest: TypedDigest32,
}

impl ActiveRepresentation {
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        representation: RepresentationId,
        activated_at: SimInstant,
        state_digest: TypedDigest32,
    ) -> Result<Self, ResidencyError> {
        state_digest.validate().map_err(ResidencyError::Contract)?;
        Ok(Self {
            authority,
            scope,
            representation,
            activated_at,
            state_digest,
        })
    }

    pub fn validate(&self) -> Result<(), ResidencyError> {
        self.state_digest
            .validate()
            .map_err(ResidencyError::Contract)
    }
}

/// Minimum deterministic residency commitment for one active representation.
///
/// The lease is intentionally *not* bound to a particular mutable state digest:
/// it says that this authority/scope/representation must remain available until
/// `minimum_residency_until`. It does not say that the state is frozen during
/// that interval. `basis` records why the residency commitment exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepresentationLease {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub representation: RepresentationId,
    pub issued_at: SimInstant,
    /// The first instant at which domain release may be reviewed.
    pub minimum_residency_until: SimInstant,
    pub basis: TypedDigest32,
}

impl RepresentationLease {
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        representation: RepresentationId,
        issued_at: SimInstant,
        minimum_residency_until: SimInstant,
        basis: TypedDigest32,
    ) -> Result<Self, ResidencyError> {
        if minimum_residency_until < issued_at {
            return Err(ResidencyError::LeaseEndsBeforeIssue {
                issued_at,
                minimum_residency_until,
            });
        }
        basis.validate().map_err(ResidencyError::Contract)?;
        Ok(Self {
            authority,
            scope,
            representation,
            issued_at,
            minimum_residency_until,
            basis,
        })
    }

    pub fn validate(&self) -> Result<(), ResidencyError> {
        if self.minimum_residency_until < self.issued_at {
            return Err(ResidencyError::LeaseEndsBeforeIssue {
                issued_at: self.issued_at,
                minimum_residency_until: self.minimum_residency_until,
            });
        }
        self.basis.validate().map_err(ResidencyError::Contract)
    }
}

/// Domain-owned authorization to attempt replacing an active representation.
///
/// Representation identifiers are opaque. This permit does not infer that
/// `to_representation` is finer or coarser than `from_representation`; the
/// owning domain decides whether the transition is semantically appropriate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepresentationReleasePermit {
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub from_representation: RepresentationId,
    pub to_representation: RepresentationId,
    pub assessed_at: SimInstant,
    /// Exact authoritative state for which release was judged safe.
    pub source_state_digest: TypedDigest32,
    /// Domain-specific evidence supporting the release decision.
    pub evidence: TypedDigest32,
}

impl RepresentationReleasePermit {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        from_representation: RepresentationId,
        to_representation: RepresentationId,
        assessed_at: SimInstant,
        source_state_digest: TypedDigest32,
        evidence: TypedDigest32,
    ) -> Result<Self, ResidencyError> {
        if from_representation == to_representation {
            return Err(ResidencyError::SameRepresentationRelease {
                representation: from_representation,
            });
        }
        source_state_digest
            .validate()
            .map_err(ResidencyError::Contract)?;
        evidence.validate().map_err(ResidencyError::Contract)?;
        Ok(Self {
            authority,
            scope,
            from_representation,
            to_representation,
            assessed_at,
            source_state_digest,
            evidence,
        })
    }

    pub fn validate(&self) -> Result<(), ResidencyError> {
        if self.from_representation == self.to_representation {
            return Err(ResidencyError::SameRepresentationRelease {
                representation: self.from_representation.clone(),
            });
        }
        self.source_state_digest
            .validate()
            .map_err(ResidencyError::Contract)?;
        self.evidence.validate().map_err(ResidencyError::Contract)
    }
}

/// Deterministic outcome of a residency review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidencyDecision {
    /// The common layer must retain the active representation at least until
    /// the stated simulation instant.
    Retain {
        until: SimInstant,
    },
    /// Minimum residency has elapsed, but no valid domain release permit exists.
    AwaitDomainPermit,
    /// A domain permit applies to the exact current state. The caller may now
    /// ask the owning domain to perform the transition and mint its transfer
    /// receipt; this decision is not itself a transfer authorization receipt.
    TransitionPermitted(RepresentationReleasePermit),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ResidencyGate;

impl ResidencyGate {
    pub fn evaluate(
        &self,
        active: &ActiveRepresentation,
        lease: &RepresentationLease,
        permit: Option<&RepresentationReleasePermit>,
        now: SimInstant,
    ) -> Result<ResidencyDecision, ResidencyError> {
        active.validate()?;
        lease.validate()?;
        self.require_lease_matches(active, lease)?;

        if now < lease.issued_at {
            return Err(ResidencyError::EvaluationBeforeLeaseIssue {
                now,
                issued_at: lease.issued_at,
            });
        }

        if now < lease.minimum_residency_until {
            return Ok(ResidencyDecision::Retain {
                until: lease.minimum_residency_until,
            });
        }

        let Some(permit) = permit else {
            return Ok(ResidencyDecision::AwaitDomainPermit);
        };
        permit.validate()?;
        self.require_permit_matches(active, lease, permit, now)?;
        Ok(ResidencyDecision::TransitionPermitted(permit.clone()))
    }

    fn require_lease_matches(
        &self,
        active: &ActiveRepresentation,
        lease: &RepresentationLease,
    ) -> Result<(), ResidencyError> {
        if active.authority != lease.authority
            || active.scope != lease.scope
            || active.representation != lease.representation
        {
            return Err(ResidencyError::LeaseIdentityMismatch);
        }
        Ok(())
    }

    fn require_permit_matches(
        &self,
        active: &ActiveRepresentation,
        lease: &RepresentationLease,
        permit: &RepresentationReleasePermit,
        now: SimInstant,
    ) -> Result<(), ResidencyError> {
        if active.authority != permit.authority
            || active.scope != permit.scope
            || active.representation != permit.from_representation
        {
            return Err(ResidencyError::PermitIdentityMismatch);
        }
        if permit.assessed_at < lease.minimum_residency_until {
            return Err(ResidencyError::PermitPredatesReviewThreshold {
                assessed_at: permit.assessed_at,
                minimum_residency_until: lease.minimum_residency_until,
            });
        }
        if permit.assessed_at > now {
            return Err(ResidencyError::PermitFromFuture {
                assessed_at: permit.assessed_at,
                now,
            });
        }
        if !permit
            .source_state_digest
            .same_typed_value(&active.state_digest)
        {
            return Err(ResidencyError::StalePermitState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidencyError {
    Contract(ContractError),
    LeaseEndsBeforeIssue {
        issued_at: SimInstant,
        minimum_residency_until: SimInstant,
    },
    EvaluationBeforeLeaseIssue {
        now: SimInstant,
        issued_at: SimInstant,
    },
    LeaseIdentityMismatch,
    PermitIdentityMismatch,
    PermitPredatesReviewThreshold {
        assessed_at: SimInstant,
        minimum_residency_until: SimInstant,
    },
    PermitFromFuture {
        assessed_at: SimInstant,
        now: SimInstant,
    },
    StalePermitState,
    SameRepresentationRelease {
        representation: RepresentationId,
    },
}

impl From<ContractError> for ResidencyError {
    fn from(value: ContractError) -> Self {
        Self::Contract(value)
    }
}

impl fmt::Display for ResidencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "simulation contract rejected residency evidence: {error}"),
            Self::LeaseEndsBeforeIssue {
                issued_at,
                minimum_residency_until,
            } => write!(
                formatter,
                "representation lease ends at {minimum_residency_until:?} before issue at {issued_at:?}"
            ),
            Self::EvaluationBeforeLeaseIssue { now, issued_at } => write!(
                formatter,
                "residency evaluated at {now:?} before lease issue at {issued_at:?}"
            ),
            Self::LeaseIdentityMismatch => write!(
                formatter,
                "representation lease does not match the active authority/scope/representation"
            ),
            Self::PermitIdentityMismatch => write!(
                formatter,
                "release permit does not match the active authority/scope/representation"
            ),
            Self::PermitPredatesReviewThreshold {
                assessed_at,
                minimum_residency_until,
            } => write!(
                formatter,
                "release permit assessed at {assessed_at:?} before residency review threshold {minimum_residency_until:?}"
            ),
            Self::PermitFromFuture { assessed_at, now } => write!(
                formatter,
                "release permit assessed at future instant {assessed_at:?} relative to {now:?}"
            ),
            Self::StalePermitState => write!(
                formatter,
                "release permit is bound to a state digest that is no longer active"
            ),
            Self::SameRepresentationRelease { representation } => write!(
                formatter,
                "release permit cannot replace {representation} with the same representation"
            ),
        }
    }
}

impl Error for ResidencyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(label: &[u8]) -> TypedDigest32 {
        TypedDigest32::sha256("symtropy.test.state.v1", 1, label).unwrap()
    }

    fn active(state: &[u8]) -> ActiveRepresentation {
        ActiveRepresentation::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight/cell-7").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            SimInstant::new(10, 0).unwrap(),
            d(state),
        )
        .unwrap()
    }

    fn lease() -> RepresentationLease {
        RepresentationLease::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight/cell-7").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            SimInstant::new(10, 0).unwrap(),
            SimInstant::new(20, 0).unwrap(),
            TypedDigest32::sha256(
                "symtropy.representation-residency.basis.v1",
                1,
                b"causal refinement",
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn permit(state: &[u8], assessed_at: SimInstant) -> RepresentationReleasePermit {
        RepresentationReleasePermit::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight/cell-7").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            RepresentationId::parse("terrain.aggregate.v1").unwrap(),
            assessed_at,
            d(state),
            TypedDigest32::sha256(
                "symtropy.terrain.release-evidence.v1",
                1,
                b"domain equivalence preconditions satisfied",
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn minimum_residency_retains_detail_before_threshold() {
        let decision = ResidencyGate
            .evaluate(
                &active(b"state-a"),
                &lease(),
                None,
                SimInstant::new(19, 999).unwrap(),
            )
            .unwrap();
        assert_eq!(
            decision,
            ResidencyDecision::Retain {
                until: SimInstant::new(20, 0).unwrap()
            }
        );
    }

    #[test]
    fn lease_expiry_never_auto_releases_representation() {
        let decision = ResidencyGate
            .evaluate(
                &active(b"state-a"),
                &lease(),
                None,
                SimInstant::new(20, 0).unwrap(),
            )
            .unwrap();
        assert_eq!(decision, ResidencyDecision::AwaitDomainPermit);
    }

    #[test]
    fn fresh_domain_permit_allows_transition_attempt() {
        let permit = permit(b"state-a", SimInstant::new(20, 0).unwrap());
        let decision = ResidencyGate
            .evaluate(
                &active(b"state-a"),
                &lease(),
                Some(&permit),
                SimInstant::new(21, 0).unwrap(),
            )
            .unwrap();
        assert_eq!(decision, ResidencyDecision::TransitionPermitted(permit));
    }

    #[test]
    fn stale_state_digest_blocks_old_release_permit() {
        let old_permit = permit(b"state-a", SimInstant::new(20, 0).unwrap());
        let result = ResidencyGate.evaluate(
            &active(b"state-b"),
            &lease(),
            Some(&old_permit),
            SimInstant::new(21, 0).unwrap(),
        );
        assert_eq!(result, Err(ResidencyError::StalePermitState));
    }

    #[test]
    fn permit_cannot_be_preissued_before_residency_review() {
        let early = permit(b"state-a", SimInstant::new(19, 0).unwrap());
        let result = ResidencyGate.evaluate(
            &active(b"state-a"),
            &lease(),
            Some(&early),
            SimInstant::new(21, 0).unwrap(),
        );
        assert!(matches!(
            result,
            Err(ResidencyError::PermitPredatesReviewThreshold { .. })
        ));
    }

    #[test]
    fn future_permit_is_rejected() {
        let future = permit(b"state-a", SimInstant::new(25, 0).unwrap());
        let result = ResidencyGate.evaluate(
            &active(b"state-a"),
            &lease(),
            Some(&future),
            SimInstant::new(21, 0).unwrap(),
        );
        assert!(matches!(result, Err(ResidencyError::PermitFromFuture { .. })));
    }

    #[test]
    fn mismatched_scope_is_rejected() {
        let mut wrong = lease();
        wrong.scope = ScopeId::parse("sol:earth:firstlight/cell-8").unwrap();
        assert_eq!(
            ResidencyGate.evaluate(
                &active(b"state-a"),
                &wrong,
                None,
                SimInstant::new(15, 0).unwrap(),
            ),
            Err(ResidencyError::LeaseIdentityMismatch)
        );
    }

    #[test]
    fn same_representation_is_not_a_release() {
        let representation = RepresentationId::parse("terrain.voxel.v2").unwrap();
        let result = RepresentationReleasePermit::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight/cell-7").unwrap(),
            representation.clone(),
            representation.clone(),
            SimInstant::new(20, 0).unwrap(),
            d(b"state-a"),
            d(b"release"),
        );
        assert_eq!(
            result,
            Err(ResidencyError::SameRepresentationRelease { representation })
        );
    }
}
