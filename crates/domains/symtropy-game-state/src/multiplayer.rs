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

/// Produces the language-neutral canonical bytes for a session.
///
/// Grammar (little-endian integers, fixed-width digests):
/// magic "SYMPROV", encoding version, schema version, then fields in declaration order.
/// Participants are already sorted by raw digest bytes and are length-prefixed.
pub fn canonical_session_bytes(session: &MultiplayerSessionV1) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 + session.participants.len() * 32);
    out.extend_from_slice(b"SYMPROV");
    out.push(MULTIPLAYER_CANONICAL_ENCODING_VERSION);
    out.extend_from_slice(&MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
    for digest in [
        session.world_instance.as_bytes(),
        session.world_continuation.as_bytes(),
        session.simulation_identity.as_bytes(),
        session.ruleset_identity.as_bytes(),
        session.initial_checkpoint.as_bytes(),
        session.authority_config.as_bytes(),
    ] {
        out.extend_from_slice(digest);
    }
    out.extend_from_slice(&(session.participants.len() as u16).to_le_bytes());
    for participant in &session.participants {
        out.extend_from_slice(participant.as_bytes());
    }
    out.push(replay_profile_tag(session.replay_profile));
    out
}

/// Produces the language-neutral canonical bytes for a checkpoint.
pub fn canonical_checkpoint_bytes(checkpoint: &StateCheckpointV1) -> Vec<u8> {
    let mut out = Vec::with_capacity(320);
    out.extend_from_slice(b"SYMPROV");
    out.push(MULTIPLAYER_CANONICAL_ENCODING_VERSION);
    out.extend_from_slice(&MULTIPLAYER_PROVENANCE_SCHEMA_VERSION.to_le_bytes());
    out.extend_from_slice(checkpoint.session_digest.as_bytes());
    out.extend_from_slice(checkpoint.world_instance.as_bytes());
    out.extend_from_slice(&checkpoint.authority_epoch.to_le_bytes());
    out.extend_from_slice(&checkpoint.simulation_tick.to_le_bytes());
    out.extend_from_slice(&checkpoint.simulation_instant.to_le_bytes());
    encode_optional_digest(checkpoint.previous_checkpoint, &mut out);
    out.extend_from_slice(checkpoint.state_digest.as_bytes());
    out.extend_from_slice(checkpoint.continuation_digest.as_bytes());
    out.extend_from_slice(checkpoint.simulation_identity.as_bytes());
    out.extend_from_slice(checkpoint.ruleset_identity.as_bytes());
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdentityDigest(pub CommitmentDigest);

impl IdentityDigest {
    pub const fn new(digest: CommitmentDigest) -> Self { Self(digest) }
    pub const fn as_bytes(&self) -> &[u8; 32] { self.0.as_bytes() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiplayerSessionV1 {
    pub world_instance: IdentityDigest,
    pub world_continuation: IdentityDigest,
    pub simulation_identity: IdentityDigest,
    pub ruleset_identity: IdentityDigest,
    pub initial_checkpoint: CommitmentDigest,
    pub authority_config: IdentityDigest,
    pub participants: Vec<IdentityDigest>,
    pub replay_profile: ReplayProfile,
    pub session_digest: CommitmentDigest,
}

impl MultiplayerSessionV1 {
    pub fn new(
        world_instance: IdentityDigest,
        world_continuation: IdentityDigest,
        simulation_identity: IdentityDigest,
        ruleset_identity: IdentityDigest,
        initial_checkpoint: CommitmentDigest,
        authority_config: IdentityDigest,
        mut participants: Vec<IdentityDigest>,
        replay_profile: ReplayProfile,
    ) -> Result<Self, StateError> {
        canonicalize_participants(&mut participants)?;
        let participant_bytes = canonical_participant_bytes(&participants);
        let session_digest = CommitmentDigest::derive(
            "multiplayer.session.v1",
            &[
                world_instance.as_bytes(),
                world_continuation.as_bytes(),
                simulation_identity.as_bytes(),
                ruleset_identity.as_bytes(),
                initial_checkpoint.as_bytes(),
                authority_config.as_bytes(),
                &participant_bytes,
                &[replay_profile_tag(replay_profile)],
            ],
        )?;
        Ok(Self {
            world_instance,
            world_continuation,
            simulation_identity,
            ruleset_identity,
            initial_checkpoint,
            authority_config,
            participants,
            replay_profile,
            session_digest,
        })
    }

    pub fn verify(&self) -> Result<(), StateError> {
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
            self.initial_checkpoint,
            self.authority_config,
            participants,
            self.replay_profile,
        )?;
        if expected.session_digest != self.session_digest {
            return Err(StateError::MultiplayerCommitmentMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateCheckpointV1 {
    pub session_digest: CommitmentDigest,
    pub world_instance: IdentityDigest,
    pub authority_epoch: u64,
    pub simulation_tick: u64,
    pub simulation_instant: u64,
    pub previous_checkpoint: Option<CommitmentDigest>,
    pub state_digest: CommitmentDigest,
    pub continuation_digest: CommitmentDigest,
    pub simulation_identity: IdentityDigest,
    pub ruleset_identity: IdentityDigest,
    pub checkpoint_digest: CommitmentDigest,
}

impl StateCheckpointV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_digest: CommitmentDigest,
        world_instance: IdentityDigest,
        authority_epoch: u64,
        simulation_tick: u64,
        simulation_instant: u64,
        previous_checkpoint: Option<CommitmentDigest>,
        state_digest: CommitmentDigest,
        continuation_digest: CommitmentDigest,
        simulation_identity: IdentityDigest,
        ruleset_identity: IdentityDigest,
    ) -> Result<Self, StateError> {
        let checkpoint_digest = checkpoint_commitment(
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

    pub fn verify(&self) -> Result<(), StateError> {
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

    pub fn is_successor_of(&self, previous: &Self) -> bool {
        self.session_digest == previous.session_digest
            && self.world_instance == previous.world_instance
            && self.previous_checkpoint == Some(previous.checkpoint_digest)
            && self.authority_epoch >= previous.authority_epoch
            && self.simulation_tick > previous.simulation_tick
            && self.simulation_instant > previous.simulation_instant
            && self.simulation_identity == previous.simulation_identity
            && self.ruleset_identity == previous.ruleset_identity
    }
}

fn checkpoint_commitment(
    session_digest: CommitmentDigest,
    world_instance: IdentityDigest,
    authority_epoch: u64,
    simulation_tick: u64,
    simulation_instant: u64,
    previous_checkpoint: Option<CommitmentDigest>,
    state_digest: CommitmentDigest,
    continuation_digest: CommitmentDigest,
    simulation_identity: IdentityDigest,
    ruleset_identity: IdentityDigest,
) -> Result<CommitmentDigest, StateError> {
    let previous = previous_checkpoint
        .map(|digest| *digest.as_bytes())
        .unwrap_or([0u8; 32]);
    CommitmentDigest::derive(
        "multiplayer.checkpoint.v1",
        &[
            session_digest.as_bytes(),
            world_instance.as_bytes(),
            &authority_epoch.to_le_bytes(),
            &simulation_tick.to_le_bytes(),
            &simulation_instant.to_le_bytes(),
            &previous,
            state_digest.as_bytes(),
            continuation_digest.as_bytes(),
            simulation_identity.as_bytes(),
            ruleset_identity.as_bytes(),
        ],
    )
}

fn canonicalize_participants(participants: &mut Vec<IdentityDigest>) -> Result<(), StateError> {
    if participants.len() > MAX_SESSION_PARTICIPANTS {
        return Err(StateError::TooManyMultiplayerParticipants);
    }
    participants.sort_unstable_by_key(|digest| *digest.as_bytes());
    if participants.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(StateError::DuplicateMultiplayerParticipant);
    }
    Ok(())
}

fn canonical_participant_bytes(participants: &[IdentityDigest]) -> Vec<u8> {
    participants
        .iter()
        .flat_map(|digest| digest.as_bytes().iter().copied())
        .collect()
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
    fn identity(tag: u8) -> IdentityDigest { IdentityDigest::new(digest(tag)) }

    fn session(participants: Vec<IdentityDigest>) -> MultiplayerSessionV1 {
        MultiplayerSessionV1::new(
            identity(1), identity(2), identity(3), identity(4), digest(5),
            identity(6), participants, ReplayProfile::BitExactReplay,
        ).expect("session")
    }

    #[test]
    fn participant_order_does_not_change_session_identity() {
        let a = session(vec![identity(8), identity(7)]);
        let b = session(vec![identity(7), identity(8)]);
        assert_eq!(a.session_digest, b.session_digest);
        assert_eq!(a.participants, b.participants);
    }

    #[test]
    fn duplicate_participants_fail_closed() {
        let result = MultiplayerSessionV1::new(
            identity(1), identity(2), identity(3), identity(4), digest(5),
            identity(6), vec![identity(7), identity(7)], ReplayProfile::ObservationOnly,
        );
        assert!(matches!(result, Err(StateError::DuplicateMultiplayerParticipant)));
    }

    #[test]
    fn session_field_substitution_changes_identity() {
        let a = session(vec![identity(7)]);
        let b = MultiplayerSessionV1::new(
            identity(1), identity(2), identity(99), identity(4), digest(5),
            identity(6), vec![identity(7)], ReplayProfile::BitExactReplay,
        ).expect("session");
        assert_ne!(a.session_digest, b.session_digest);
    }

    #[test]
    fn checkpoint_binds_session_and_lineage() {
        let s = session(vec![identity(7)]);
        let a = StateCheckpointV1::new(
            s.session_digest, identity(1), 0, 100, 100, None,
            digest(10), digest(11), identity(3), identity(4),
        ).expect("checkpoint");
        let b = StateCheckpointV1::new(
            digest(200), identity(1), 0, 100, 100, None,
            digest(10), digest(11), identity(3), identity(4),
        ).expect("checkpoint");
        assert_ne!(a.checkpoint_digest, b.checkpoint_digest);
        a.verify().expect("valid checkpoint");
    }

    #[test]
    fn checkpoint_successor_requires_explicit_predecessor() {
        let s = session(vec![identity(7)]);
        let first = StateCheckpointV1::new(
            s.session_digest, identity(1), 0, 100, 100, None,
            digest(10), digest(11), identity(3), identity(4),
        ).expect("first");
        let second = StateCheckpointV1::new(
            s.session_digest, identity(1), 0, 200, 200, Some(first.checkpoint_digest),
            digest(12), digest(13), identity(3), identity(4),
        ).expect("second");
        assert!(second.is_successor_of(&first));
    }

    #[test]
    fn checkpoint_cannot_cross_session_boundary() {
        let s1 = session(vec![identity(7)]);
        let s2 = session(vec![identity(8)]);
        let first = StateCheckpointV1::new(
            s1.session_digest, identity(1), 0, 100, 100, None,
            digest(10), digest(11), identity(3), identity(4),
        ).expect("first");
        let second = StateCheckpointV1::new(
            s2.session_digest, identity(1), 0, 200, 200, Some(first.checkpoint_digest),
            digest(12), digest(13), identity(3), identity(4),
        ).expect("second");
        assert!(!second.is_successor_of(&first));
    }
}
