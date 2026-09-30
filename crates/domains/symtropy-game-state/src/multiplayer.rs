//! Transport-independent multiplayer provenance commitments.
//!
//! These value objects sit below the live network/session loop. They do not
//! assign authority, create world identities, or serialize the live ECS state.
//! Instead they commit to already-canonical identity digests supplied by the
//! owning world/authority systems.

use crate::{CommitmentDigest, ReplayProfile, StateError};
use serde::{Deserialize, Serialize};

pub const MAX_SESSION_PARTICIPANTS: usize = 256;
pub const MULTIPLAYER_PROVENANCE_SCHEMA_VERSION: u32 = 1;

/// Canonical binary encoding version. This is deliberately independent of serde/JSON.
pub const MULTIPLAYER_CANONICAL_ENCODING_VERSION: u8 = 1;

/// Encodes an optional digest without relying on an all-zero sentinel.
fn encode_optional_digest(digest: Option<CommitmentDigest>, out: &mut Vec<u8>) {
    match digest {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(value.as_bytes());
        }
        None => out.push(0),
    }
}

/// Produces the exact language-neutral bytes committed as the semantic session identity.
///
/// Replay-profile claims are intentionally excluded. A caller changing an unqualified
/// claim about replay strength must not fork the semantic identity of an otherwise
/// identical session.
pub fn canonical_session_bytes(session: &MultiplayerSessionV1) -> Vec<u8> {
    canonical_session_fields(
        session.world_instance,
        session.world_continuation,
        session.simulation_identity,
        session.ruleset_identity,
        session.initial_state_commitment,
        session.authority_config,
        &session.participants,
    )
}

fn canonical_session_fields(
    world_instance: UnqualifiedIdentityDigest,
    world_continuation: UnqualifiedIdentityDigest,
    simulation_identity: UnqualifiedIdentityDigest,
    ruleset_identity: UnqualifiedIdentityDigest,
    initial_state_commitment: CommitmentDigest,
    authority_config: UnqualifiedIdentityDigest,
    participants: &[UnqualifiedIdentityDigest],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 + participants.len() * 32);
    out.extend_from_slice(b"SYMPROV");
    out.push(MULTIPLAYER_CANONICAL_ENCODING_VERSION);
    out.extend_from_slice(&MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
    for digest in [
        world_instance.as_bytes(),
        world_continuation.as_bytes(),
        simulation_identity.as_bytes(),
        ruleset_identity.as_bytes(),
        initial_state_commitment.as_bytes(),
        authority_config.as_bytes(),
    ] {
        out.extend_from_slice(digest);
    }
    out.extend_from_slice(&(participants.len() as u16).to_le_bytes());
    for participant in participants {
        out.extend_from_slice(participant.as_bytes());
    }
    out
}

/// Produces the exact language-neutral bytes committed as a replay claim.
///
/// The claim is bound to the semantic session identity and the caller-declared
/// replay profile, but remains metadata rather than qualification evidence.
pub fn canonical_replay_claim_bytes(session: &MultiplayerSessionV1) -> Vec<u8> {
    canonical_replay_claim_fields(session.session_digest, session.replay_profile)
}

fn canonical_replay_claim_fields(
    session_digest: CommitmentDigest,
    replay_profile: ReplayProfile,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(b"SYMPROV");
    out.push(MULTIPLAYER_CANONICAL_ENCODING_VERSION);
    out.extend_from_slice(&MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
    out.extend_from_slice(session_digest.as_bytes());
    out.push(replay_profile_tag(replay_profile));
    out
}

/// Produces the exact language-neutral bytes committed as the checkpoint identity.
pub fn canonical_checkpoint_bytes(checkpoint: &StateCheckpointV1) -> Vec<u8> {
    canonical_checkpoint_fields(
        checkpoint.session_digest,
        checkpoint.world_instance,
        checkpoint.authority_epoch,
        checkpoint.simulation_tick,
        checkpoint.simulation_instant,
        checkpoint.previous_checkpoint,
        checkpoint.state_digest,
        checkpoint.continuation_digest,
        checkpoint.simulation_identity,
        checkpoint.ruleset_identity,
    )
}

// Keep the canonical field list explicit: each parameter is a distinct, ordered
// commitment field, and collapsing them into a bag/tuple would obscure the grammar.
// This is a serialization boundary, not a general-purpose application API.
#[allow(clippy::too_many_arguments)]
fn canonical_checkpoint_fields(
    session_digest: CommitmentDigest,
    world_instance: UnqualifiedIdentityDigest,
    authority_epoch: u64,
    simulation_tick: u64,
    simulation_instant: u64,
    previous_checkpoint: Option<CommitmentDigest>,
    state_digest: CommitmentDigest,
    continuation_digest: CommitmentDigest,
    simulation_identity: UnqualifiedIdentityDigest,
    ruleset_identity: UnqualifiedIdentityDigest,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(320);
    out.extend_from_slice(b"SYMPROV");
    out.push(MULTIPLAYER_CANONICAL_ENCODING_VERSION);
    out.extend_from_slice(&MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
    out.extend_from_slice(session_digest.as_bytes());
    out.extend_from_slice(world_instance.as_bytes());
    out.extend_from_slice(&authority_epoch.to_le_bytes());
    out.extend_from_slice(&simulation_tick.to_le_bytes());
    out.extend_from_slice(&simulation_instant.to_le_bytes());
    encode_optional_digest(previous_checkpoint, &mut out);
    out.extend_from_slice(state_digest.as_bytes());
    out.extend_from_slice(continuation_digest.as_bytes());
    out.extend_from_slice(simulation_identity.as_bytes());
    out.extend_from_slice(ruleset_identity.as_bytes());
    out
}

/// Temporary bridge for an identity whose semantic owner has not yet exposed
/// an owner-issued reference type.
///
/// This proves only possession of a 32-byte commitment. It does not prove that
/// the commitment is a world identity, continuation identity, simulation
/// identity, ruleset identity, authority configuration, or accepted/current
/// authority state. The name is intentionally explicit so callers cannot
/// mistake this bridge for semantic evidence.
///
/// Replace each field with an owner-issued typed reference as soon as its
/// owning subsystem exists. Do not add semantic constructors here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnqualifiedIdentityDigest(CommitmentDigest);

impl UnqualifiedIdentityDigest {
    /// Creates a temporary unqualified bridge from an already-computed digest.
    ///
    /// This deliberately does not claim semantic ownership or acceptance.
    /// Owner-specific constructors belong in the owning subsystem.
    pub const fn from_digest(digest: CommitmentDigest) -> Self {
        Self(digest)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiplayerSessionV1 {
    pub world_instance: UnqualifiedIdentityDigest,
    pub world_continuation: UnqualifiedIdentityDigest,
    pub simulation_identity: UnqualifiedIdentityDigest,
    pub ruleset_identity: UnqualifiedIdentityDigest,
    /// External pre-session admission anchor. This is not a `StateCheckpointV1` digest;
    /// keeping it independent prevents a session/checkpoint identity cycle.
    pub initial_state_commitment: CommitmentDigest,
    pub authority_config: UnqualifiedIdentityDigest,
    pub participants: Vec<UnqualifiedIdentityDigest>,
    /// Caller-declared replay claim strength. This field is metadata, not qualification
    /// evidence; the commitment layer never upgrades it into proof.
    pub replay_profile: ReplayProfile,
    /// Commitment to the caller-declared replay claim, bound to session_digest.
    /// This is metadata commitment, not qualification or acceptance evidence.
    pub replay_claim_digest: CommitmentDigest,
    /// Semantic session identity commitment. Replay claims are deliberately excluded.
    pub session_digest: CommitmentDigest,
}

impl MultiplayerSessionV1 {
    // The constructor mirrors the v1 semantic contract field-for-field. Keeping the
    // arguments explicit makes accidental omission/substitution visible at call sites.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_instance: UnqualifiedIdentityDigest,
        world_continuation: UnqualifiedIdentityDigest,
        simulation_identity: UnqualifiedIdentityDigest,
        ruleset_identity: UnqualifiedIdentityDigest,
        // External pre-session admission anchor; intentionally not a StateCheckpointV1 digest.
        initial_state_commitment: CommitmentDigest,
        authority_config: UnqualifiedIdentityDigest,
        mut participants: Vec<UnqualifiedIdentityDigest>,
        replay_profile: ReplayProfile,
    ) -> Result<Self, StateError> {
        canonicalize_participants(&mut participants)?;
        let session_digest = CommitmentDigest::derive(
            "multiplayer.session.v1",
            &[&canonical_session_fields(
                world_instance,
                world_continuation,
                simulation_identity,
                ruleset_identity,
                initial_state_commitment,
                authority_config,
                &participants,
            )],
        )?;
        let replay_claim_bytes = canonical_replay_claim_fields(session_digest, replay_profile);
        let replay_claim_digest =
            CommitmentDigest::derive("multiplayer.replay_claim.v1", &[&replay_claim_bytes])?;
        Ok(Self {
            world_instance,
            world_continuation,
            simulation_identity,
            ruleset_identity,
            initial_state_commitment,
            authority_config,
            participants,
            replay_profile,
            replay_claim_digest,
            session_digest,
        })
    }

    /// Verifies only the self-consistency of the canonical commitment.
    ///
    /// This does not establish semantic ownership, owner issuance, authority acceptance,
    /// or currentness. Those properties require evidence from the owning subsystem.
    pub fn verify_commitment(&self) -> Result<(), StateError> {
        let mut participants = self.participants.clone();
        canonicalize_participants(&mut participants)?;
        if participants != self.participants {
            return Err(StateError::NonCanonicalMultiplayerParticipants);
        }
        let expected = Self::new(
            self.world_instance,
            self.world_continuation,
            self.simulation_identity,
            self.ruleset_identity,
            self.initial_state_commitment,
            self.authority_config,
            participants,
            self.replay_profile,
        )?;
        if expected.session_digest != self.session_digest
            || expected.replay_claim_digest != self.replay_claim_digest
        {
            return Err(StateError::MultiplayerCommitmentMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateCheckpointV1 {
    pub session_digest: CommitmentDigest,
    pub world_instance: UnqualifiedIdentityDigest,
    /// Epoch number carried by the checkpoint. A number alone is not an accepted-authority
    /// receipt and cannot establish a handoff.
    pub authority_epoch: u64,
    pub simulation_tick: u64,
    pub simulation_instant: u64,
    pub previous_checkpoint: Option<CommitmentDigest>,
    pub state_digest: CommitmentDigest,
    /// Commitment identifying continuation bytes. Identity alone does not establish that
    /// the continuation is admitted or current.
    pub continuation_digest: CommitmentDigest,
    pub simulation_identity: UnqualifiedIdentityDigest,
    pub ruleset_identity: UnqualifiedIdentityDigest,
    pub checkpoint_digest: CommitmentDigest,
}

impl StateCheckpointV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_digest: CommitmentDigest,
        world_instance: UnqualifiedIdentityDigest,
        authority_epoch: u64,
        simulation_tick: u64,
        simulation_instant: u64,
        previous_checkpoint: Option<CommitmentDigest>,
        state_digest: CommitmentDigest,
        continuation_digest: CommitmentDigest,
        simulation_identity: UnqualifiedIdentityDigest,
        ruleset_identity: UnqualifiedIdentityDigest,
    ) -> Result<Self, StateError> {
        let checkpoint_digest = CommitmentDigest::derive(
            "multiplayer.checkpoint.v1",
            &[&canonical_checkpoint_fields(
                session_digest,
                world_instance,
                authority_epoch,
                simulation_tick,
                simulation_instant,
                previous_checkpoint,
                state_digest,
                continuation_digest,
                simulation_identity,
                ruleset_identity,
            )],
        )?;
        Ok(Self {
            session_digest,
            world_instance,
            authority_epoch,
            simulation_tick,
            simulation_instant,
            previous_checkpoint,
            state_digest,
            continuation_digest,
            simulation_identity,
            ruleset_identity,
            checkpoint_digest,
        })
    }

    /// Verifies only the self-consistency of the canonical checkpoint commitment.
    ///
    /// This does not establish semantic ownership, accepted authority, continuation
    /// admission, or currentness. Those properties require owner-issued evidence.
    pub fn verify_commitment(&self) -> Result<(), StateError> {
        let expected = Self::new(
            self.session_digest,
            self.world_instance,
            self.authority_epoch,
            self.simulation_tick,
            self.simulation_instant,
            self.previous_checkpoint,
            self.state_digest,
            self.continuation_digest,
            self.simulation_identity,
            self.ruleset_identity,
        )?;
        if expected.checkpoint_digest != self.checkpoint_digest {
            return Err(StateError::MultiplayerCommitmentMismatch);
        }
        Ok(())
    }

    /// Checks checkpoint-chain continuity within one already-accepted authority epoch.
    ///
    /// Authority handoff/epoch advancement is intentionally outside this predicate:
    /// a provenance record must not manufacture acceptance of a new authority epoch.
    pub fn is_successor_of(&self, previous: &Self) -> bool {
        self.session_digest == previous.session_digest
            && self.world_instance == previous.world_instance
            && self.previous_checkpoint == Some(previous.checkpoint_digest)
            && self.authority_epoch == previous.authority_epoch
            && self.simulation_tick > previous.simulation_tick
            && self.simulation_instant > previous.simulation_instant
            && self.simulation_identity == previous.simulation_identity
            && self.ruleset_identity == previous.ruleset_identity
    }
}

fn canonicalize_participants(
    participants: &mut [UnqualifiedIdentityDigest],
) -> Result<(), StateError> {
    if participants.len() > MAX_SESSION_PARTICIPANTS {
        return Err(StateError::TooManyMultiplayerParticipants);
    }
    participants.sort_unstable_by_key(|digest| *digest.as_bytes());
    if participants.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(StateError::DuplicateMultiplayerParticipant);
    }
    Ok(())
}

const fn replay_profile_tag(profile: ReplayProfile) -> u8 {
    match profile {
        ReplayProfile::BitExactReplay => 0,
        ReplayProfile::AuthoritativeStateReplay => 1,
        ReplayProfile::ObservationOnly => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> CommitmentDigest {
        CommitmentDigest::derive("fixture", &[&[tag]]).expect("fixture digest")
    }
    fn identity(tag: u8) -> UnqualifiedIdentityDigest {
        UnqualifiedIdentityDigest::from_digest(digest(tag))
    }

    fn session(participants: Vec<UnqualifiedIdentityDigest>) -> MultiplayerSessionV1 {
        MultiplayerSessionV1::new(
            identity(1),
            identity(2),
            identity(3),
            identity(4),
            digest(5),
            identity(6),
            participants,
            ReplayProfile::BitExactReplay,
        )
        .expect("session")
    }

    #[test]
    fn session_identity_is_hash_of_canonical_bytes() {
        let s = session(vec![identity(7), identity(8)]);
        let expected =
            CommitmentDigest::derive("multiplayer.session.v1", &[&canonical_session_bytes(&s)])
                .expect("session commitment");
        assert_eq!(s.session_digest, expected);
    }

    #[test]
    fn replay_claim_commitment_is_separate_from_session_identity() {
        let observation = session(vec![identity(7)]);
        let bit_exact = MultiplayerSessionV1::new(
            observation.world_instance,
            observation.world_continuation,
            observation.simulation_identity,
            observation.ruleset_identity,
            observation.initial_state_commitment,
            observation.authority_config,
            observation.participants.clone(),
            ReplayProfile::BitExactReplay,
        )
        .expect("bit-exact claim");
        assert_eq!(observation.session_digest, bit_exact.session_digest);
        assert_ne!(
            observation.replay_claim_digest,
            bit_exact.replay_claim_digest
        );
        assert_ne!(
            canonical_replay_claim_bytes(&observation),
            canonical_replay_claim_bytes(&bit_exact)
        );
    }

    #[test]
    fn commitment_verification_is_not_owner_evidence() {
        let s = session(vec![identity(7)]);
        let checkpoint = StateCheckpointV1::new(
            s.session_digest,
            identity(200),
            999,
            1,
            1,
            None,
            digest(201),
            digest(202),
            identity(203),
            identity(204),
        )
        .expect("self-consistent checkpoint");
        checkpoint
            .verify_commitment()
            .expect("commitment is internally consistent");
    }

    #[test]
    fn replay_profile_is_claim_metadata_not_qualification_evidence() {
        let observation = session(vec![identity(7)]);
        let bit_exact = MultiplayerSessionV1::new(
            observation.world_instance,
            observation.world_continuation,
            observation.simulation_identity,
            observation.ruleset_identity,
            observation.initial_state_commitment,
            observation.authority_config,
            observation.participants.clone(),
            ReplayProfile::BitExactReplay,
        )
        .expect("bit-exact claim");
        assert_eq!(observation.session_digest, bit_exact.session_digest);
        assert_ne!(
            observation.replay_claim_digest,
            bit_exact.replay_claim_digest
        );
        assert_eq!(bit_exact.replay_profile, ReplayProfile::BitExactReplay);
        bit_exact
            .verify_commitment()
            .expect("claim commitment is self-consistent");
    }

    #[test]
    fn equal_epoch_numbers_do_not_establish_cross_lineage_continuity() {
        let s = session(vec![identity(7)]);
        let a = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            7,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("first lineage");
        let b = StateCheckpointV1::new(
            s.session_digest,
            identity(9),
            7,
            200,
            200,
            Some(a.checkpoint_digest),
            digest(12),
            digest(13),
            identity(3),
            identity(4),
        )
        .expect("second lineage");
        assert_eq!(a.authority_epoch, b.authority_epoch);
        assert!(!b.is_successor_of(&a));
    }

    #[test]
    fn valid_historical_continuation_digest_is_not_currentness_evidence() {
        let s = session(vec![identity(7)]);
        let historical = digest(11);
        let checkpoint = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            200,
            200,
            None,
            digest(12),
            historical,
            identity(3),
            identity(4),
        )
        .expect("checkpoint");
        assert_eq!(checkpoint.continuation_digest, historical);
        checkpoint
            .verify_commitment()
            .expect("identity commitment verifies");
    }

    #[test]
    fn checkpoint_commitment_is_hash_of_canonical_bytes() {
        let s = session(vec![identity(7)]);
        let checkpoint = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");
        let expected = CommitmentDigest::derive(
            "multiplayer.checkpoint.v1",
            &[&canonical_checkpoint_bytes(&checkpoint)],
        )
        .expect("checkpoint commitment");
        assert_eq!(checkpoint.checkpoint_digest, expected);
    }

    #[test]
    fn session_golden_vector_v1() {
        let s = session(vec![identity(7), identity(8)]);
        assert_eq!(canonical_session_bytes(&s).len(), 270);
        assert_eq!(
            s.session_digest.to_hex(),
            "2ac0da1bb25c1b1344b8ec1402b954bc0574d735ef1c1f4d8bb85589555cf8cf"
        );
    }

    #[test]
    fn initial_state_anchor_is_not_a_checkpoint_identity_cycle() {
        let a = session(vec![identity(7)]);
        let b = MultiplayerSessionV1::new(
            identity(1),
            identity(2),
            identity(3),
            identity(4),
            digest(55),
            identity(6),
            vec![identity(7)],
            ReplayProfile::BitExactReplay,
        )
        .expect("session");
        assert_ne!(a.session_digest, b.session_digest);
        assert_ne!(a.initial_state_commitment, b.initial_state_commitment);
    }

    #[test]
    fn participant_order_does_not_change_session_identity() {
        let a = session(vec![identity(8), identity(7)]);
        let b = session(vec![identity(7), identity(8)]);
        assert_eq!(a.session_digest, b.session_digest);
        assert_eq!(a.participants, b.participants);
        assert_eq!(canonical_session_bytes(&a), canonical_session_bytes(&b));
    }

    #[test]
    fn canonical_session_encoding_has_fixed_header() {
        let s = session(vec![identity(7)]);
        let bytes = canonical_session_bytes(&s);
        assert_eq!(&bytes[..7], b"SYMPROV");
        assert_eq!(bytes[7], MULTIPLAYER_CANONICAL_ENCODING_VERSION);
        assert_eq!(
            &bytes[8..12],
            &MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes()
        );
        assert_eq!(bytes.len(), 270);
    }

    #[test]
    fn replay_claim_commitment_is_hash_of_canonical_bytes() {
        let s = session(vec![identity(7)]);
        let expected = CommitmentDigest::derive(
            "multiplayer.replay_claim.v1",
            &[&canonical_replay_claim_bytes(&s)],
        )
        .expect("replay claim commitment");
        assert_eq!(s.replay_claim_digest, expected);
    }

    #[test]
    fn replay_claim_encoding_and_digest_are_profile_sensitive() {
        let observation = session(vec![identity(7)]);
        let claim = canonical_replay_claim_bytes(&observation);
        assert_eq!(claim.len(), 45);
        assert_eq!(&claim[..7], b"SYMPROV");
        assert_eq!(claim[7], MULTIPLAYER_CANONICAL_ENCODING_VERSION);
        assert_eq!(
            &claim[8..12],
            &MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes()
        );
        assert_eq!(claim[44], 0);
        assert_eq!(
            observation.replay_claim_digest.to_hex(),
            "2d945f50ea879ea8c2a52626b124cd52b392922ce919300df9105581698c49d1"
        );
    }

    #[test]
    fn checkpoint_field_perturbations_change_identity() {
        let s = session(vec![identity(7), identity(8)]);
        let base = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");

        let variants = [
            StateCheckpointV1::new(
                s.session_digest,
                identity(9),
                0,
                100,
                100,
                None,
                digest(10),
                digest(11),
                identity(3),
                identity(4),
            )
            .expect("world variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                1,
                100,
                100,
                None,
                digest(10),
                digest(11),
                identity(3),
                identity(4),
            )
            .expect("epoch variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                101,
                100,
                None,
                digest(10),
                digest(11),
                identity(3),
                identity(4),
            )
            .expect("tick variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                100,
                101,
                None,
                digest(10),
                digest(11),
                identity(3),
                identity(4),
            )
            .expect("instant variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                100,
                100,
                Some(digest(99)),
                digest(10),
                digest(11),
                identity(3),
                identity(4),
            )
            .expect("previous variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                100,
                100,
                None,
                digest(12),
                digest(11),
                identity(3),
                identity(4),
            )
            .expect("state variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                100,
                100,
                None,
                digest(10),
                digest(12),
                identity(3),
                identity(4),
            )
            .expect("continuation variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                100,
                100,
                None,
                digest(10),
                digest(11),
                identity(5),
                identity(4),
            )
            .expect("simulation variant"),
            StateCheckpointV1::new(
                s.session_digest,
                identity(1),
                0,
                100,
                100,
                None,
                digest(10),
                digest(11),
                identity(3),
                identity(5),
            )
            .expect("ruleset variant"),
        ];

        for variant in variants {
            assert_ne!(base.checkpoint_digest, variant.checkpoint_digest);
        }
    }

    #[test]
    fn checkpoint_canonical_encoding_distinguishes_absent_predecessor() {
        let s = session(vec![identity(7)]);
        let checkpoint = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");
        let bytes = canonical_checkpoint_bytes(&checkpoint);
        assert_eq!(&bytes[..7], b"SYMPROV");
        assert_eq!(bytes[7], MULTIPLAYER_CANONICAL_ENCODING_VERSION);
        assert_eq!(bytes[12 + 32 + 32 + 8 + 8 + 8], 0);
    }

    #[test]
    fn duplicate_participants_fail_closed() {
        let result = MultiplayerSessionV1::new(
            identity(1),
            identity(2),
            identity(3),
            identity(4),
            digest(5),
            identity(6),
            vec![identity(7), identity(7)],
            ReplayProfile::ObservationOnly,
        );
        assert!(matches!(
            result,
            Err(StateError::DuplicateMultiplayerParticipant)
        ));
    }

    #[test]
    fn session_field_substitution_changes_identity() {
        let a = session(vec![identity(7)]);
        let b = MultiplayerSessionV1::new(
            identity(1),
            identity(2),
            identity(99),
            identity(4),
            digest(5),
            identity(6),
            vec![identity(7)],
            ReplayProfile::BitExactReplay,
        )
        .expect("session");
        assert_ne!(a.session_digest, b.session_digest);
    }

    #[test]
    fn checkpoint_binds_session_and_lineage() {
        let s = session(vec![identity(7)]);
        let a = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");
        let b = StateCheckpointV1::new(
            digest(200),
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");
        assert_ne!(a.checkpoint_digest, b.checkpoint_digest);
        a.verify_commitment().expect("valid checkpoint");
    }

    #[test]
    fn checkpoint_successor_requires_explicit_predecessor() {
        let s = session(vec![identity(7)]);
        let first = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("first");
        let second = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            200,
            200,
            Some(first.checkpoint_digest),
            digest(12),
            digest(13),
            identity(3),
            identity(4),
        )
        .expect("second");
        assert!(second.is_successor_of(&first));
    }

    #[test]
    fn checkpoint_chain_does_not_implicitly_accept_authority_handoff() {
        let s = session(vec![identity(7)]);
        let first = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("first");
        let next_epoch = StateCheckpointV1::new(
            s.session_digest,
            identity(1),
            1,
            200,
            200,
            Some(first.checkpoint_digest),
            digest(12),
            digest(13),
            identity(3),
            identity(4),
        )
        .expect("next epoch checkpoint");
        assert!(!next_epoch.is_successor_of(&first));
    }

    #[test]
    fn checkpoint_cannot_cross_session_boundary() {
        let s1 = session(vec![identity(7)]);
        let s2 = session(vec![identity(8)]);
        let first = StateCheckpointV1::new(
            s1.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("first");
        let second = StateCheckpointV1::new(
            s2.session_digest,
            identity(1),
            0,
            200,
            200,
            Some(first.checkpoint_digest),
            digest(12),
            digest(13),
            identity(3),
            identity(4),
        )
        .expect("second");
        assert!(!second.is_successor_of(&first));
    }

    #[test]
    fn checkpoint_serde_round_trip_preserves_commitment_contract() {
        let session = session(vec![identity(7)]);
        let original = StateCheckpointV1::new(
            session.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");

        let encoded = serde_json::to_vec(&original).expect("serialize checkpoint");
        let decoded: StateCheckpointV1 =
            serde_json::from_slice(&encoded).expect("deserialize checkpoint");

        assert_eq!(decoded, original);
        assert_eq!(
            canonical_checkpoint_bytes(&decoded),
            canonical_checkpoint_bytes(&original)
        );
        assert_eq!(decoded.checkpoint_digest, original.checkpoint_digest);
        decoded.verify_commitment().expect("round-trip commitment");

        let mut substituted = decoded;
        substituted.simulation_tick += 1;
        assert!(matches!(
            substituted.verify_commitment(),
            Err(StateError::MultiplayerCommitmentMismatch)
        ));
    }

    #[test]
    fn session_persisted_record_field_substitution_fails_closed() {
        let original = session(vec![identity(7), identity(8)]);
        let encoded = serde_json::to_vec(&original).expect("serialize session");
        let decoded: MultiplayerSessionV1 =
            serde_json::from_slice(&encoded).expect("deserialize session");

        let mut variants = Vec::new();
        let mut world_instance = decoded.clone();
        world_instance.world_instance = identity(9);
        variants.push(world_instance);
        let mut world_continuation = decoded.clone();
        world_continuation.world_continuation = identity(9);
        variants.push(world_continuation);
        let mut simulation = decoded.clone();
        simulation.simulation_identity = identity(9);
        variants.push(simulation);
        let mut ruleset = decoded.clone();
        ruleset.ruleset_identity = identity(9);
        variants.push(ruleset);
        let mut initial_state = decoded.clone();
        initial_state.initial_state_commitment = digest(9);
        variants.push(initial_state);
        let mut authority = decoded.clone();
        authority.authority_config = identity(9);
        variants.push(authority);
        let mut session_digest = decoded.clone();
        session_digest.session_digest = digest(9);
        variants.push(session_digest);
        let mut replay_claim_digest = decoded.clone();
        replay_claim_digest.replay_claim_digest = digest(9);
        variants.push(replay_claim_digest);
        let mut participants = decoded.clone();
        participants.participants.swap(0, 1);
        variants.push(participants);

        for variant in variants {
            assert!(matches!(
                variant.verify_commitment(),
                Err(
                    StateError::MultiplayerCommitmentMismatch
                        | StateError::NonCanonicalMultiplayerParticipants
                )
            ));
        }
    }

    #[test]
    fn checkpoint_persisted_record_field_substitution_fails_closed() {
        let session = session(vec![identity(7)]);
        let original = StateCheckpointV1::new(
            session.session_digest,
            identity(1),
            0,
            100,
            100,
            None,
            digest(10),
            digest(11),
            identity(3),
            identity(4),
        )
        .expect("checkpoint");
        let encoded = serde_json::to_vec(&original).expect("serialize checkpoint");
        let decoded: StateCheckpointV1 =
            serde_json::from_slice(&encoded).expect("deserialize checkpoint");

        let mut variants = Vec::new();
        let mut session_digest = decoded.clone();
        session_digest.session_digest = digest(20);
        variants.push(session_digest);
        let mut world_instance = decoded.clone();
        world_instance.world_instance = identity(20);
        variants.push(world_instance);
        let mut epoch = decoded.clone();
        epoch.authority_epoch += 1;
        variants.push(epoch);
        let mut tick = decoded.clone();
        tick.simulation_tick += 1;
        variants.push(tick);
        let mut instant = decoded.clone();
        instant.simulation_instant += 1;
        variants.push(instant);
        let mut predecessor = decoded.clone();
        predecessor.previous_checkpoint = Some(digest(20));
        variants.push(predecessor);
        let mut state = decoded.clone();
        state.state_digest = digest(20);
        variants.push(state);
        let mut continuation = decoded.clone();
        continuation.continuation_digest = digest(20);
        variants.push(continuation);
        let mut simulation = decoded.clone();
        simulation.simulation_identity = identity(20);
        variants.push(simulation);
        let mut ruleset = decoded.clone();
        ruleset.ruleset_identity = identity(20);
        variants.push(ruleset);
        let mut checkpoint_digest = decoded.clone();
        checkpoint_digest.checkpoint_digest = digest(20);
        variants.push(checkpoint_digest);

        for variant in variants {
            assert!(matches!(
                variant.verify_commitment(),
                Err(StateError::MultiplayerCommitmentMismatch)
            ));
        }
    }

    #[test]
    fn serde_round_trip_preserves_commitment_contract_but_does_not_validate_it() {
        let original = session(vec![identity(7), identity(8)]);
        let encoded = serde_json::to_vec(&original).expect("serialize session");
        let decoded: MultiplayerSessionV1 =
            serde_json::from_slice(&encoded).expect("deserialize session");

        assert_eq!(decoded, original);
        assert_eq!(
            canonical_session_bytes(&decoded),
            canonical_session_bytes(&original)
        );
        assert_eq!(
            canonical_replay_claim_bytes(&decoded),
            canonical_replay_claim_bytes(&original)
        );
        assert_eq!(decoded.session_digest, original.session_digest);
        assert_eq!(decoded.replay_claim_digest, original.replay_claim_digest);
        decoded.verify_commitment().expect("round-trip commitment");

        // Deserialization reconstructs data; it is not an identity or commitment verifier.
        let mut substituted = decoded;
        substituted.replay_profile = ReplayProfile::ObservationOnly;
        assert!(matches!(
            substituted.verify_commitment(),
            Err(StateError::MultiplayerCommitmentMismatch)
        ));
    }
}
