// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistence-session bootstrap around Symtropy's sealed live evidence facade.
//!
//! The physics crate remains responsible for live authority semantics. This
//! integration layer owns the concrete v1 OS-CSPRNG session namespace used to
//! distinguish durable export sessions across process lifetimes. It does not
//! authenticate the binding; Xenia/PHYS-EVID-03 remains responsible for that.

use rand::{RngCore, rngs::OsRng};
use symtropy_physics::{
    LocalEvidenceAuthorityError, LocalEvidencePhysicsAuthorityWorld,
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedAuthorityStepStamp,
    LocalTaintedEvidenceAuthority, LocalTemporalIncarnationId, PhysicalAuthorityId,
    PhysicsCallback, WorldGenerationId,
};

pub const EVIDENCE_SESSION_ID_LEN: usize = 32;
const ZERO_ID_RETRY_LIMIT: usize = 8;

/// Persistence-safe namespace for one qualified evidence-export session.
///
/// Bytes are private and there is no caller-selected constructor or Serde
/// authority. The production minting path is OS CSPRNG bootstrap below.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct EvidenceSessionId([u8; EVIDENCE_SESSION_ID_LEN]);

impl EvidenceSessionId {
    pub const fn as_bytes(&self) -> &[u8; EVIDENCE_SESSION_ID_LEN] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; EVIDENCE_SESSION_ID_LEN] {
        self.0
    }
}

/// Exact bootstrap profile bound into the qualified session record.
#[repr(u32)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum EvidenceSessionProfileV1 {
    OsCsprngBoundLiveIncarnation = 1,
}

impl EvidenceSessionProfileV1 {
    pub const fn code(self) -> u32 {
        match self {
            Self::OsCsprngBoundLiveIncarnation => 1,
        }
    }
}

/// Qualified binding between one persistence session ID and one exact live
/// sealed authority incarnation.
///
/// The binding is intentionally non-Clone and non-Serde. It is created only as
/// part of [`PersistentEvidenceSession::bootstrap`].
#[derive(Debug, PartialEq, Eq)]
pub struct QualifiedEvidenceSessionBinding {
    session_id: EvidenceSessionId,
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    temporal_incarnation_id: LocalTemporalIncarnationId,
    profile: EvidenceSessionProfileV1,
}

impl QualifiedEvidenceSessionBinding {
    pub const fn session_id(&self) -> EvidenceSessionId {
        self.session_id
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub const fn temporal_incarnation_id(&self) -> LocalTemporalIncarnationId {
        self.temporal_incarnation_id
    }

    pub const fn profile(&self) -> EvidenceSessionProfileV1 {
        self.profile
    }

    /// Verify that this binding still refers to the exact supplied live sealed
    /// facade. This is an identity check only, not cryptographic authentication.
    pub fn matches_live<const D: usize>(
        &self,
        evidence: &LocalEvidencePhysicsAuthorityWorld<D>,
    ) -> bool {
        self.physical_authority_id == evidence.physical_authority_id()
            && self.world_generation_id == evidence.world_generation_id()
            && self.temporal_incarnation_id == evidence.temporal_incarnation_id()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EvidenceSessionBootstrapError {
    EntropyUnavailable,
    ZeroSessionIdRetryLimit,
}

/// Failed bootstrap retains the exact sealed live facade because no persistent
/// session was established.
pub struct EvidenceSessionBootstrapFailure<const D: usize> {
    evidence: LocalEvidencePhysicsAuthorityWorld<D>,
    error: EvidenceSessionBootstrapError,
}

impl<const D: usize> EvidenceSessionBootstrapFailure<D> {
    pub const fn error(&self) -> EvidenceSessionBootstrapError {
        self.error
    }

    pub fn into_evidence_authority(self) -> LocalEvidencePhysicsAuthorityWorld<D> {
        self.evidence
    }
}

/// One persistence-qualified export session owning one exact sealed live
/// evidence incarnation.
///
/// There is deliberately no public conversion back to a live
/// `LocalEvidencePhysicsAuthorityWorld` while preserving the same incarnation.
/// Clean exit consumes all the way to the namespace boundary; any later live
/// evidence session must therefore reseal and receive a fresh temporal
/// incarnation before it can bootstrap a new persistent session.
pub struct PersistentEvidenceSession<const D: usize> {
    binding: QualifiedEvidenceSessionBinding,
    evidence: LocalEvidencePhysicsAuthorityWorld<D>,
}

impl<const D: usize> PersistentEvidenceSession<D> {
    /// Consume one exact sealed live facade and bind it to a fresh 256-bit
    /// OS-CSPRNG evidence-session namespace.
    pub fn bootstrap(
        evidence: LocalEvidencePhysicsAuthorityWorld<D>,
    ) -> Result<Self, EvidenceSessionBootstrapFailure<D>> {
        Self::bootstrap_with_fill(evidence, |bytes| {
            let mut rng = OsRng;
            rng.try_fill_bytes(bytes).map_err(|_| ())
        })
    }

    pub const fn binding(&self) -> &QualifiedEvidenceSessionBinding {
        &self.binding
    }

    pub const fn session_id(&self) -> EvidenceSessionId {
        self.binding.session_id
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.binding.physical_authority_id
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.binding.world_generation_id
    }

    pub const fn temporal_incarnation_id(&self) -> LocalTemporalIncarnationId {
        self.binding.temporal_incarnation_id
    }

    /// Immutable projection for already-qualified observation capture.
    pub const fn evidence_authority(&self) -> &LocalEvidencePhysicsAuthorityWorld<D> {
        &self.evidence
    }

    pub fn last_qualified_step_stamp(&self) -> Option<LocalQualifiedAuthorityStepStamp> {
        self.evidence.last_qualified_step_stamp()
    }

    pub fn step_authorized(
        &mut self,
        dt: f64,
    ) -> Result<LocalQualifiedAuthorityStepStamp, LocalEvidenceAuthorityError> {
        self.evidence.step_authorized(dt)
    }

    pub fn step_authorized_with_callback(
        &mut self,
        dt: f64,
        callback: &mut dyn PhysicsCallback<D>,
    ) -> Result<LocalQualifiedAuthorityStepStamp, LocalEvidenceAuthorityError> {
        self.evidence.step_authorized_with_callback(dt, callback)
    }

    /// End this persistence session and leave the current live temporal
    /// incarnation entirely. Clean state returns the namespace boundary; tainted
    /// state preserves the sealed facade's quarantine theorem.
    pub fn into_namespace(
        self,
    ) -> Result<LocalNamespacePhysicsAuthorityWorld<D>, LocalTaintedEvidenceAuthority<D>> {
        self.evidence.into_namespace()
    }

    fn bootstrap_with_fill<F>(
        evidence: LocalEvidencePhysicsAuthorityWorld<D>,
        mut fill: F,
    ) -> Result<Self, EvidenceSessionBootstrapFailure<D>>
    where
        F: FnMut(&mut [u8; EVIDENCE_SESSION_ID_LEN]) -> Result<(), ()>,
    {
        let session_id = match mint_session_id_with(&mut fill) {
            Ok(session_id) => session_id,
            Err(error) => {
                return Err(EvidenceSessionBootstrapFailure { evidence, error });
            }
        };

        let binding = QualifiedEvidenceSessionBinding {
            session_id,
            physical_authority_id: evidence.physical_authority_id(),
            world_generation_id: evidence.world_generation_id(),
            temporal_incarnation_id: evidence.temporal_incarnation_id(),
            profile: EvidenceSessionProfileV1::OsCsprngBoundLiveIncarnation,
        };
        debug_assert!(binding.matches_live(&evidence));

        Ok(Self { binding, evidence })
    }
}

fn mint_session_id_with<F>(
    fill: &mut F,
) -> Result<EvidenceSessionId, EvidenceSessionBootstrapError>
where
    F: FnMut(&mut [u8; EVIDENCE_SESSION_ID_LEN]) -> Result<(), ()>,
{
    for _ in 0..ZERO_ID_RETRY_LIMIT {
        let mut bytes = [0_u8; EVIDENCE_SESSION_ID_LEN];
        fill(&mut bytes).map_err(|_| EvidenceSessionBootstrapError::EntropyUnavailable)?;
        if bytes.iter().any(|byte| *byte != 0) {
            return Ok(EvidenceSessionId(bytes));
        }
    }
    Err(EvidenceSessionBootstrapError::ZeroSessionIdRetryLimit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::{
        LocalEvidencePhysicsAuthorityWorld, LocalNamespacePhysicsAuthorityWorld,
        LocalQualifiedPhysicalAuthority, PhysicsWorld,
    };

    fn seal() -> LocalEvidencePhysicsAuthorityWorld<3> {
        let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
        let generation = root.mint_generation().unwrap();
        let namespace =
            LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<3>::default());
        match LocalEvidencePhysicsAuthorityWorld::seal(namespace) {
            Ok(evidence) => evidence,
            Err(failure) => panic!("seal failed: {:?}", failure.error()),
        }
    }

    fn bootstrap(evidence: LocalEvidencePhysicsAuthorityWorld<3>) -> PersistentEvidenceSession<3> {
        match PersistentEvidenceSession::bootstrap(evidence) {
            Ok(session) => session,
            Err(failure) => panic!("bootstrap failed: {:?}", failure.error()),
        }
    }

    #[test]
    fn bootstrap_binds_nonzero_session_id_to_exact_live_incarnation() {
        let evidence = seal();
        let authority = evidence.physical_authority_id();
        let generation = evidence.world_generation_id();
        let incarnation = evidence.temporal_incarnation_id();
        let session = bootstrap(evidence);

        assert!(session.session_id().as_bytes().iter().any(|byte| *byte != 0));
        assert_eq!(session.physical_authority_id(), authority);
        assert_eq!(session.world_generation_id(), generation);
        assert_eq!(session.temporal_incarnation_id(), incarnation);
        assert_eq!(
            session.binding().profile(),
            EvidenceSessionProfileV1::OsCsprngBoundLiveIncarnation
        );
        assert!(session.binding().matches_live(session.evidence_authority()));
    }

    #[test]
    fn stepping_preserves_session_binding_and_live_incarnation() {
        let mut session = bootstrap(seal());
        let session_id = session.session_id();
        let incarnation = session.temporal_incarnation_id();

        let first = session.step_authorized(1.0 / 64.0).unwrap();
        let second = session.step_authorized(1.0 / 64.0).unwrap();

        assert_eq!(first.step_index(), 1);
        assert_eq!(second.step_index(), 2);
        assert_eq!(session.session_id(), session_id);
        assert_eq!(session.temporal_incarnation_id(), incarnation);
        assert_eq!(second.temporal_incarnation_id(), incarnation);
    }

    #[test]
    fn clean_exit_and_reseal_requires_a_new_live_and_persistent_session() {
        let first = bootstrap(seal());
        let authority = first.physical_authority_id();
        let generation = first.world_generation_id();
        let first_session_id = first.session_id();
        let first_incarnation = first.temporal_incarnation_id();
        let namespace = match first.into_namespace() {
            Ok(namespace) => namespace,
            Err(_) => panic!("clean session unexpectedly quarantined"),
        };

        let evidence = match LocalEvidencePhysicsAuthorityWorld::seal(namespace) {
            Ok(evidence) => evidence,
            Err(failure) => panic!("reseal failed: {:?}", failure.error()),
        };
        let second = bootstrap(evidence);

        assert_eq!(second.physical_authority_id(), authority);
        assert_eq!(second.world_generation_id(), generation);
        assert_ne!(second.temporal_incarnation_id(), first_incarnation);
        assert_ne!(second.session_id(), first_session_id);
    }

    #[test]
    fn entropy_failure_returns_the_original_live_facade_without_session() {
        let evidence = seal();
        let authority = evidence.physical_authority_id();
        let generation = evidence.world_generation_id();
        let incarnation = evidence.temporal_incarnation_id();

        let failure = match PersistentEvidenceSession::bootstrap_with_fill(evidence, |_| Err(())) {
            Ok(_) => panic!("entropy failure must not establish a session"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error(), EvidenceSessionBootstrapError::EntropyUnavailable);
        let recovered = failure.into_evidence_authority();
        assert_eq!(recovered.physical_authority_id(), authority);
        assert_eq!(recovered.world_generation_id(), generation);
        assert_eq!(recovered.temporal_incarnation_id(), incarnation);
    }

    #[test]
    fn all_zero_entropy_is_retried_and_never_becomes_a_session_id() {
        let evidence = seal();
        let mut calls = 0_usize;
        let session = match PersistentEvidenceSession::bootstrap_with_fill(evidence, |bytes| {
            calls += 1;
            if calls == 1 {
                bytes.fill(0);
            } else {
                bytes.fill(0x5a);
            }
            Ok(())
        }) {
            Ok(session) => session,
            Err(failure) => panic!("retry bootstrap failed: {:?}", failure.error()),
        };

        assert_eq!(calls, 2);
        assert_eq!(session.session_id().to_bytes(), [0x5a; EVIDENCE_SESSION_ID_LEN]);
    }

    #[test]
    fn repeated_zero_entropy_fails_closed_without_binding() {
        let evidence = seal();
        let failure = match PersistentEvidenceSession::bootstrap_with_fill(evidence, |bytes| {
            bytes.fill(0);
            Ok(())
        }) {
            Ok(_) => panic!("all-zero entropy must never establish a session"),
            Err(failure) => failure,
        };
        assert_eq!(
            failure.error(),
            EvidenceSessionBootstrapError::ZeroSessionIdRetryLimit
        );
    }
}
