// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic identifiers, simulation time, causal events, hash chains, and multiplayer commitments.

pub mod multiplayer;
pub use multiplayer::{MultiplayerSessionV1, StateCheckpointV1, UnqualifiedIdentityDigest};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

/// Schema version emitted by the first product-state implementation.
pub const GAME_STATE_SCHEMA_VERSION: u32 = 1;

/// Stable, serializable identifier derived from authored or deterministic input.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StableId(String);

impl StableId {
    /// Creates an identifier after validating its portable text representation.
    pub fn parse(value: impl Into<String>) -> Result<Self, StateError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 96
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(StateError::InvalidStableId(value))
        }
    }

    /// Derives a stable identifier from namespace, seed, and ordinal.
    pub fn derive(namespace: &str, seed: u64, ordinal: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(namespace.as_bytes());
        hasher.update([0]);
        hasher.update(seed.to_le_bytes());
        hasher.update(ordinal.to_le_bytes());
        let digest = hasher.finalize();
        Self(format!("{namespace}:{}", hex(&digest[..16])))
    }

    /// Returns the portable identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Fixed-step simulation clock. Wall-clock time is never authoritative gameplay time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationClock {
    tick: u64,
    step_nanoseconds: u64,
}

impl SimulationClock {
    /// Creates a zeroed clock at the requested fixed update frequency.
    pub fn from_hz(hz: u32) -> Result<Self, StateError> {
        if hz == 0 || 1_000_000_000 % u64::from(hz) != 0 {
            return Err(StateError::InvalidFixedFrequency(hz));
        }
        Ok(Self {
            tick: 0,
            step_nanoseconds: 1_000_000_000 / u64::from(hz),
        })
    }

    /// Returns the current authoritative tick.
    pub const fn tick(self) -> u64 {
        self.tick
    }

    /// Returns the duration of one simulation step.
    pub const fn step_nanoseconds(self) -> u64 {
        self.step_nanoseconds
    }

    /// Advances exactly one deterministic step.
    pub fn advance(&mut self) -> Result<u64, StateError> {
        self.tick = self.tick.checked_add(1).ok_or(StateError::ClockOverflow)?;
        Ok(self.tick)
    }

    /// Advances a bounded number of deterministic steps.
    pub fn advance_by(&mut self, steps: u64) -> Result<u64, StateError> {
        self.tick = self
            .tick
            .checked_add(steps)
            .ok_or(StateError::ClockOverflow)?;
        Ok(self.tick)
    }

    /// Returns elapsed simulation nanoseconds, if representable.
    pub fn elapsed_nanoseconds(self) -> Result<u128, StateError> {
        u128::from(self.tick)
            .checked_mul(u128::from(self.step_nanoseconds))
            .ok_or(StateError::ClockOverflow)
    }
}

/// An immutable gameplay event with causal and observer provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope<T> {
    /// State schema used when the event was authored.
    pub schema_version: u32,
    /// Stable event identity.
    pub event_id: StableId,
    /// Authoritative simulation tick.
    pub simulation_tick: u64,
    /// Machine-readable event kind.
    pub kind: String,
    /// Acting entity, when an actor exists.
    pub actor_id: Option<StableId>,
    /// Observer or instrument that produced the claim, when relevant.
    pub observer_id: Option<StableId>,
    /// Direct causal parents preserved for explanation and replay.
    pub causal_parents: Vec<StableId>,
    /// Typed event payload.
    pub payload: T,
    /// Hash of the previous event, or the genesis marker.
    pub previous_hash: String,
    /// Hash of every preceding field.
    pub event_hash: String,
}

#[derive(Serialize)]
struct UnsignedEvent<'a, T> {
    schema_version: u32,
    event_id: &'a StableId,
    simulation_tick: u64,
    kind: &'a str,
    actor_id: &'a Option<StableId>,
    observer_id: &'a Option<StableId>,
    causal_parents: &'a [StableId],
    payload: &'a T,
    previous_hash: &'a str,
}

impl<T: Serialize> EventEnvelope<T> {
    fn calculate_hash(&self) -> Result<String, StateError> {
        let unsigned = UnsignedEvent {
            schema_version: self.schema_version,
            event_id: &self.event_id,
            simulation_tick: self.simulation_tick,
            kind: &self.kind,
            actor_id: &self.actor_id,
            observer_id: &self.observer_id,
            causal_parents: &self.causal_parents,
            payload: &self.payload,
            previous_hash: &self.previous_hash,
        };
        let bytes = serde_json::to_vec(&unsigned).map_err(StateError::Serialization)?;
        Ok(hash_bytes(&bytes))
    }

    /// Verifies the event's content hash.
    pub fn verify_hash(&self) -> Result<(), StateError> {
        let actual = self.calculate_hash()?;
        if actual == self.event_hash {
            Ok(())
        } else {
            Err(StateError::EventHashMismatch {
                event_id: self.event_id.clone(),
                expected: self.event_hash.clone(),
                actual,
            })
        }
    }
}

/// Append-only in-memory event chain used by save journals and deterministic replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventChain<T> {
    namespace: String,
    seed: u64,
    events: Vec<EventEnvelope<T>>,
}

impl<T> EventChain<T> {
    /// Creates an empty deterministic chain.
    pub fn new(namespace: impl Into<String>, seed: u64) -> Self {
        Self {
            namespace: namespace.into(),
            seed,
            events: Vec::new(),
        }
    }

    /// Returns all committed events in chain order.
    pub fn events(&self) -> &[EventEnvelope<T>] {
        &self.events
    }

    /// Returns the current chain head or the genesis marker.
    pub fn head_hash(&self) -> &str {
        self.events
            .last()
            .map_or("GENESIS", |event| event.event_hash.as_str())
    }

    /// Reconstructs a chain from persisted events before running verification.
    pub fn from_events(
        namespace: impl Into<String>,
        seed: u64,
        events: Vec<EventEnvelope<T>>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            seed,
            events,
        }
    }
}

impl<T: Serialize> EventChain<T> {
    /// Appends a typed event and returns its stable identity.
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &mut self,
        simulation_tick: u64,
        kind: impl Into<String>,
        actor_id: Option<StableId>,
        observer_id: Option<StableId>,
        causal_parents: Vec<StableId>,
        payload: T,
    ) -> Result<StableId, StateError> {
        let ordinal = u64::try_from(self.events.len()).map_err(|_| StateError::EventOverflow)?;
        let event_id = StableId::derive(&self.namespace, self.seed, ordinal);
        let previous_hash = self.head_hash().to_owned();
        let mut envelope = EventEnvelope {
            schema_version: GAME_STATE_SCHEMA_VERSION,
            event_id: event_id.clone(),
            simulation_tick,
            kind: kind.into(),
            actor_id,
            observer_id,
            causal_parents,
            payload,
            previous_hash,
            event_hash: String::new(),
        };
        envelope.event_hash = envelope.calculate_hash()?;
        self.events.push(envelope);
        Ok(event_id)
    }

    /// Verifies hashes, causal order, identifiers, and monotonic simulation time.
    pub fn verify(&self) -> Result<(), StateError> {
        let mut previous_hash = "GENESIS";
        let mut previous_tick = 0;
        for (index, event) in self.events.iter().enumerate() {
            let ordinal = u64::try_from(index).map_err(|_| StateError::EventOverflow)?;
            let expected_id = StableId::derive(&self.namespace, self.seed, ordinal);
            if event.event_id != expected_id {
                return Err(StateError::EventIdMismatch {
                    expected: expected_id,
                    actual: event.event_id.clone(),
                });
            }
            if event.previous_hash != previous_hash {
                return Err(StateError::PreviousHashMismatch {
                    event_id: event.event_id.clone(),
                    expected: previous_hash.to_owned(),
                    actual: event.previous_hash.clone(),
                });
            }
            if index > 0 && event.simulation_tick < previous_tick {
                return Err(StateError::NonMonotonicTick {
                    event_id: event.event_id.clone(),
                    previous: previous_tick,
                    actual: event.simulation_tick,
                });
            }
            event.verify_hash()?;
            previous_tick = event.simulation_tick;
            previous_hash = &event.event_hash;
        }
        Ok(())
    }
}

/// Errors produced by deterministic game-state primitives.
#[derive(Debug)]
pub enum StateError {
    /// Commitment domain is empty, oversized, or non-ASCII.
    InvalidCommitmentDomain,
    /// A commitment field exceeded the portable input bound.
    CommitmentInputTooLarge,
    /// Multiplayer participant list was not in canonical order.
    NonCanonicalMultiplayerParticipants,
    /// A multiplayer participant appeared more than once.
    DuplicateMultiplayerParticipant,
    /// Too many participant references were supplied.
    TooManyMultiplayerParticipants,
    /// A durable multiplayer commitment did not match its canonical fields.
    MultiplayerCommitmentMismatch,
    /// Stable identifier text was empty, too long, or non-portable.
    InvalidStableId(String),
    /// The requested fixed frequency cannot divide one second exactly.
    InvalidFixedFrequency(u32),
    /// Simulation time exceeded its representation.
    ClockOverflow,
    /// The event collection exceeded a portable ordinal.
    EventOverflow,
    /// JSON serialization failed.
    Serialization(serde_json::Error),
    /// An event did not have its deterministic identifier.
    EventIdMismatch {
        expected: StableId,
        actual: StableId,
    },
    /// A chain link did not point to its predecessor.
    PreviousHashMismatch {
        event_id: StableId,
        expected: String,
        actual: String,
    },
    /// Event ticks moved backward.
    NonMonotonicTick {
        event_id: StableId,
        previous: u64,
        actual: u64,
    },
    /// Event bytes no longer match the stored hash.
    EventHashMismatch {
        event_id: StableId,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommitmentDomain => {
                formatter.write_str("invalid commitment domain")
            }
            Self::CommitmentInputTooLarge => {
                formatter.write_str("commitment input is too large")
            }
            Self::NonCanonicalMultiplayerParticipants => {
                formatter.write_str("multiplayer participants are not canonical")
            }
            Self::DuplicateMultiplayerParticipant => {
                formatter.write_str("duplicate multiplayer participant")
            }
            Self::TooManyMultiplayerParticipants => {
                formatter.write_str("too many multiplayer participants")
            }
            Self::MultiplayerCommitmentMismatch => {
                formatter.write_str("multiplayer commitment mismatch")
            }
            Self::InvalidStableId(value) => {
                write!(formatter, "invalid stable identifier: {value:?}")
            }
            Self::InvalidFixedFrequency(hz) => write!(
                formatter,
                "fixed frequency must divide 1 GHz exactly: {hz} Hz"
            ),
            Self::ClockOverflow => formatter.write_str("simulation clock overflow"),
            Self::EventOverflow => formatter.write_str("event ordinal overflow"),
            Self::Serialization(error) => write!(formatter, "event serialization failed: {error}"),
            Self::EventIdMismatch { expected, actual } => write!(
                formatter,
                "event id mismatch: expected {expected}, got {actual}"
            ),
            Self::PreviousHashMismatch {
                event_id,
                expected,
                actual,
            } => write!(
                formatter,
                "event {event_id} previous hash mismatch: expected {expected}, got {actual}"
            ),
            Self::NonMonotonicTick {
                event_id,
                previous,
                actual,
            } => write!(
                formatter,
                "event {event_id} moved backward from tick {previous} to {actual}"
            ),
            Self::EventHashMismatch {
                event_id,
                expected,
                actual,
            } => write!(
                formatter,
                "event {event_id} hash mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for StateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialization(error) => Some(error),
            _ => None,
        }
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}


/// Schema version for transport-independent multiplayer commitments.
pub const MULTIPLAYER_COMMITMENT_SCHEMA_VERSION: u32 = 1;

/// Maximum size of one commitment field accepted by the canonical primitive.
pub const MAX_COMMITMENT_FIELD_BYTES: usize = 16 * 1024 * 1024;

/// Cryptographic digest used for multiplayer provenance identities.
///
/// This is deliberately distinct from the lockstep module's FNV state hash:
/// the latter is a fast divergence detector, while this type is intended for
/// durable identity/provenance and therefore uses SHA-256.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommitmentDigest([u8; 32]);

impl CommitmentDigest {
    /// Computes a domain-separated digest over an ordered sequence of fields.
    ///
    /// Each field is length-prefixed, so concatenation boundaries cannot be
    /// ambiguous. Callers must provide already-canonical bytes.
    pub fn derive(domain: &str, fields: &[&[u8]]) -> Result<Self, StateError> {
        if domain.is_empty() || domain.len() > 128 || !domain.is_ascii() {
            return Err(StateError::InvalidCommitmentDomain);
        }

        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.commitment.v1");
        hasher.update((domain.len() as u64).to_le_bytes());
        hasher.update(domain.as_bytes());

        for field in fields {
            if field.len() > MAX_COMMITMENT_FIELD_BYTES {
                return Err(StateError::CommitmentInputTooLarge);
            }
            let len = u64::try_from(field.len())
                .map_err(|_| StateError::CommitmentInputTooLarge)?;
            hasher.update(len.to_le_bytes());
            hasher.update(field);
        }

        Ok(Self(hasher.finalize().into()))
    }

    /// Returns the raw digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the lowercase hexadecimal representation.
    pub fn to_hex(self) -> String {
        hex(&self.0)
    }
}

impl fmt::Display for CommitmentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
    }
}

/// Explicit claim strength for replay evidence.
///
/// A replay implementation must select the strongest profile it can actually
/// establish; the commitment layer never upgrades a weaker profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayProfile {
    /// All claim-relevant state is covered by an exact deterministic model.
    BitExactReplay,
    /// The authoritative state/checkpoints are reproduced, but execution need
    /// not be bit-for-bit identical internally.
    AuthoritativeStateReplay,
    /// Evidence is observational and cannot establish deterministic replay.
    ObservationOnly,
}

impl ReplayProfile {
    /// Whether this profile is permitted to claim bit-for-bit replay.
    pub const fn permits_bit_exact_claim(self) -> bool {
        matches!(self, Self::BitExactReplay)
    }
}

#[cfg(test)]
mod multiplayer_commitment_tests {
    use super::*;

    #[test]
    fn commitment_field_size_limit_is_enforced() {
        let oversized = vec![0u8; MAX_COMMITMENT_FIELD_BYTES + 1];
        assert!(matches!(
            CommitmentDigest::derive("test", &[oversized.as_slice()]),
            Err(StateError::CommitmentInputTooLarge)
        ));
    }

    #[test]
    fn commitment_is_domain_separated() {
        let fields = [b"same".as_slice()];
        let a = CommitmentDigest::derive("session", &fields).expect("valid domain");
        let b = CommitmentDigest::derive("checkpoint", &fields).expect("valid domain");
        assert_ne!(a, b);
    }

    #[test]
    fn commitment_preserves_field_boundaries() {
        let concatenated = [b"ab".as_slice()];
        let split = [b"a".as_slice(), b"b".as_slice()];
        let a = CommitmentDigest::derive("test", &concatenated).expect("valid");
        let b = CommitmentDigest::derive("test", &split).expect("valid");
        assert_ne!(a, b);
    }

    #[test]
    fn commitment_is_stable_for_identical_canonical_bytes() {
        let fields = [b"world".as_slice(), b"ruleset-v1".as_slice(), &[7u8][..]];
        let first = CommitmentDigest::derive("session", &fields).expect("valid");
        let second = CommitmentDigest::derive("session", &fields).expect("valid");
        assert_eq!(first, second);
        assert_eq!(first.to_hex().len(), 64);
    }

    #[test]
    fn replay_profile_never_upgrades_itself() {
        assert!(ReplayProfile::BitExactReplay.permits_bit_exact_claim());
        assert!(!ReplayProfile::AuthoritativeStateReplay.permits_bit_exact_claim());
        assert!(!ReplayProfile::ObservationOnly.permits_bit_exact_claim());
    }

    #[test]
    fn invalid_commitment_domain_is_rejected() {
        assert!(matches!(
            CommitmentDigest::derive("", &[]),
            Err(StateError::InvalidCommitmentDomain)
        ));
        assert!(matches!(
            CommitmentDigest::derive("non-ascii-☃", &[]),
            Err(StateError::InvalidCommitmentDomain)
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        value: u32,
    }

    #[test]
    fn stable_ids_are_reproducible() {
        let first = StableId::derive("resident", 41, 7);
        let second = StableId::derive("resident", 41, 7);
        assert_eq!(first, second);
        assert!(first.as_str().starts_with("resident:"));
    }

    #[test]
    fn simulation_clock_advances_in_fixed_steps() {
        let mut clock = SimulationClock::from_hz(20).expect("20 Hz divides one second");
        clock.advance_by(40).expect("clock remains in range");
        assert_eq!(clock.tick(), 40);
        assert_eq!(
            clock.elapsed_nanoseconds().expect("elapsed time"),
            2_000_000_000
        );
    }

    #[test]
    fn chain_detects_payload_tampering() {
        let mut chain = EventChain::new("firstlight", 99);
        chain
            .append(
                1,
                "observation",
                None,
                None,
                Vec::new(),
                TestPayload { value: 5 },
            )
            .expect("append first event");
        chain
            .append(
                2,
                "repair",
                None,
                None,
                Vec::new(),
                TestPayload { value: 8 },
            )
            .expect("append second event");
        chain.verify().expect("untampered chain verifies");
        chain.events[0].payload.value = 6;
        assert!(matches!(
            chain.verify(),
            Err(StateError::EventHashMismatch { .. })
        ));
    }
}
