// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic identifiers, simulation time, causal events, and hash chains.

pub mod canonical;
pub mod event_v2;
pub mod namespace;

pub use canonical::{
    CanonicalDigest, CanonicalError, CanonicalEventPayload, CanonicalWriter, EventDigestV2,
    PayloadDigest,
};
pub use event_v2::{
    CANONICAL_EVENT_SCHEMA_VERSION_V2, EventChainV2, EventEnvelopeV2, EventV2Error,
    StableEventKind,
};
pub use namespace::{NamespaceError, StableIdNamespace};

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

    #[test]
    fn v1_identity_fixture_remains_byte_for_byte_stable() {
        let mut chain = EventChain::new("firstlight", 99);
        let first = chain
            .append(
                1,
                "observation",
                None,
                None,
                Vec::new(),
                TestPayload { value: 5 },
            )
            .expect("append first event");
        let second = chain
            .append(
                2,
                "repair",
                None,
                None,
                Vec::new(),
                TestPayload { value: 8 },
            )
            .expect("append second event");

        assert_eq!(
            first.as_str(),
            "firstlight:cff9ae668c60133e813c3cac57fe427c"
        );
        assert_eq!(
            second.as_str(),
            "firstlight:6b17f7e3683a6fe1a510f7a7cbdc8600"
        );
        assert_eq!(
            chain.events()[0].event_hash,
            "f56c500e2c4884e4a7dc43cebdf037c836c37cdcc6c52c719b185d07bd20a03f"
        );
        assert_eq!(
            chain.events()[1].event_hash,
            "a87d9e06d4c2455ea485010ff63403143fcf6799ac358c1115c3c1a1c4e6a4c7"
        );
    }
}
