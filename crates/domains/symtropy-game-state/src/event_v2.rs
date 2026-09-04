// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical serializer-independent causal event identity v2.
//!
//! V2 is additive. The historical `EventChain<T>` JSON hash contract remains unchanged.

use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    canonical::{CanonicalError, CanonicalEventPayload, CanonicalWriter, EventDigestV2, PayloadDigest},
    namespace::{NamespaceError, StableIdNamespace},
    StableId, StateError,
};

/// Canonical event schema understood by this implementation.
pub const CANONICAL_EVENT_SCHEMA_VERSION_V2: u32 = 2;
const EVENT_DIGEST_DOMAIN_V2: &[u8] = b"symtropy/game-state/event/v2";
const MAX_SEMANTIC_ID_LEN: usize = 96;

/// Validated stable event-kind identifier used by canonical v2 events.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct StableEventKind(String);

impl<'de> Deserialize<'de> for StableEventKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl StableEventKind {
    /// Parses a portable event kind such as `fold.rewind.applied`.
    pub fn parse(value: impl Into<String>) -> Result<Self, EventV2Error> {
        let kind = Self(value.into());
        kind.validate()?;
        Ok(kind)
    }

    /// Re-validates an event kind after deserialization.
    pub fn validate(&self) -> Result<(), EventV2Error> {
        if valid_semantic_id(&self.0) {
            Ok(())
        } else {
            Err(EventV2Error::InvalidEventKind(self.0.clone()))
        }
    }

    /// Returns stable event-kind text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableEventKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Canonical v2 event envelope. Payload bytes are retained for replay/application, but canonical
/// event identity binds only the payload's stable schema ID and domain-owned semantic digest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelopeV2<T> {
    /// Canonical event schema version.
    pub schema_version: u32,
    /// Deterministically derived stable occurrence identity.
    #[serde(deserialize_with = "deserialize_stable_id")]
    pub event_id: StableId,
    /// Authoritative fixed-step simulation tick.
    pub simulation_tick: u64,
    /// Stable machine-readable event kind.
    pub kind: StableEventKind,
    /// Acting entity, when any.
    #[serde(default, deserialize_with = "deserialize_optional_stable_id")]
    pub actor_id: Option<StableId>,
    /// Observer/instrument, when any.
    #[serde(default, deserialize_with = "deserialize_optional_stable_id")]
    pub observer_id: Option<StableId>,
    /// Direct causal parents. V2 interprets these as a set and hashes canonical sorted order.
    #[serde(deserialize_with = "deserialize_stable_ids")]
    pub causal_parents: Vec<StableId>,
    /// Stable domain-owned payload schema identifier.
    #[serde(deserialize_with = "deserialize_payload_schema")]
    pub payload_schema: String,
    /// Typed semantic digest supplied by the payload-owning domain.
    pub payload_digest: PayloadDigest,
    /// Typed payload retained for application/replay; its serializer bytes are not event identity.
    pub payload: T,
    /// Previous canonical event digest, absent only at genesis.
    pub previous_digest: Option<EventDigestV2>,
    /// Serializer-independent canonical identity of this event.
    pub event_digest: EventDigestV2,
}

impl<T: CanonicalEventPayload> EventEnvelopeV2<T> {
    fn calculate_digest(&self) -> Result<EventDigestV2, EventV2Error> {
        validate_stable_id(&self.event_id)?;
        self.kind.validate()?;
        validate_optional_stable_id(self.actor_id.as_ref())?;
        validate_optional_stable_id(self.observer_id.as_ref())?;
        validate_payload_schema(&self.payload_schema)?;

        let parents = canonical_parents(&self.causal_parents)?;
        for parent in &parents {
            validate_stable_id(parent)?;
        }

        let mut writer = CanonicalWriter::new(EVENT_DIGEST_DOMAIN_V2)?;
        writer.write_u32(self.schema_version);
        writer.write_str(self.event_id.as_str())?;
        writer.write_u64(self.simulation_tick);
        writer.write_str(self.kind.as_str())?;
        writer.write_option(self.actor_id.as_ref(), |writer, id| {
            writer.write_str(id.as_str())
        })?;
        writer.write_option(self.observer_id.as_ref(), |writer, id| {
            writer.write_str(id.as_str())
        })?;
        writer.write_count(parents.len())?;
        for parent in &parents {
            writer.write_str(parent.as_str())?;
        }
        writer.write_str(&self.payload_schema)?;
        writer.write_digest(self.payload_digest.canonical());
        writer.write_option(self.previous_digest.as_ref(), |writer, digest| {
            writer.write_digest(digest.canonical());
            Ok(())
        })?;
        Ok(EventDigestV2::new(writer.finish()))
    }

    fn verify_payload_contract(&self) -> Result<(), EventV2Error> {
        validate_payload_schema(T::PAYLOAD_SCHEMA)?;
        validate_payload_schema(&self.payload_schema)?;
        if self.payload_schema != T::PAYLOAD_SCHEMA {
            return Err(EventV2Error::PayloadSchemaMismatch {
                expected: T::PAYLOAD_SCHEMA.to_owned(),
                actual: self.payload_schema.clone(),
            });
        }
        let actual = self.payload.canonical_payload_digest();
        if actual != self.payload_digest {
            return Err(EventV2Error::PayloadDigestMismatch {
                event_id: self.event_id.clone(),
                expected: self.payload_digest,
                actual,
            });
        }
        Ok(())
    }
}

/// Append-only canonical v2 causal event chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventChainV2<T> {
    namespace: StableIdNamespace,
    seed: u64,
    events: Vec<EventEnvelopeV2<T>>,
}

impl<T> EventChainV2<T> {
    /// Creates an empty v2 chain using a validated deterministic ID namespace.
    pub fn new(namespace: StableIdNamespace, seed: u64) -> Self {
        Self {
            namespace,
            seed,
            events: Vec::new(),
        }
    }

    /// Returns committed events in chain order.
    pub fn events(&self) -> &[EventEnvelopeV2<T>] {
        &self.events
    }

    /// Returns the current canonical event head, or `None` at genesis.
    pub fn head_digest(&self) -> Option<EventDigestV2> {
        self.events.last().map(|event| event.event_digest)
    }

    /// Reconstructs a persisted chain before explicit verification.
    pub fn from_events(
        namespace: StableIdNamespace,
        seed: u64,
        events: Vec<EventEnvelopeV2<T>>,
    ) -> Self {
        Self {
            namespace,
            seed,
            events,
        }
    }
}

impl<T: CanonicalEventPayload> EventChainV2<T> {
    /// Appends a canonical event after validating causal-parent existence and current ordering.
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &mut self,
        simulation_tick: u64,
        kind: StableEventKind,
        actor_id: Option<StableId>,
        observer_id: Option<StableId>,
        causal_parents: Vec<StableId>,
        payload: T,
    ) -> Result<StableId, EventV2Error> {
        self.namespace.validate()?;
        kind.validate()?;
        validate_optional_stable_id(actor_id.as_ref())?;
        validate_optional_stable_id(observer_id.as_ref())?;
        validate_payload_schema(T::PAYLOAD_SCHEMA)?;

        if let Some(previous) = self.events.last() {
            if simulation_tick < previous.simulation_tick {
                return Err(EventV2Error::NonMonotonicTick {
                    event_id: previous.event_id.clone(),
                    previous: previous.simulation_tick,
                    actual: simulation_tick,
                });
            }
        }

        let ordinal = u64::try_from(self.events.len()).map_err(|_| EventV2Error::EventOverflow)?;
        let event_id = StableId::derive_v2(&self.namespace, self.seed, ordinal)?;
        let parents = canonical_parents(&causal_parents)?;
        for parent in &parents {
            validate_stable_id(parent)?;
            if parent == &event_id {
                return Err(EventV2Error::SelfCausalParent {
                    event_id: event_id.clone(),
                });
            }
            if !self.events.iter().any(|event| &event.event_id == parent) {
                return Err(EventV2Error::UnknownCausalParent {
                    event_id: event_id.clone(),
                    parent_id: parent.clone(),
                });
            }
        }

        let payload_digest = payload.canonical_payload_digest();
        let previous_digest = self.head_digest();
        let mut envelope = EventEnvelopeV2 {
            schema_version: CANONICAL_EVENT_SCHEMA_VERSION_V2,
            event_id: event_id.clone(),
            simulation_tick,
            kind,
            actor_id,
            observer_id,
            causal_parents: parents,
            payload_schema: T::PAYLOAD_SCHEMA.to_owned(),
            payload_digest,
            payload,
            previous_digest,
            event_digest: EventDigestV2::new(crate::canonical::CanonicalDigest::from_bytes([0; 32])),
        };
        envelope.event_digest = envelope.calculate_digest()?;
        self.events.push(envelope);
        Ok(event_id)
    }

    /// Verifies schema, deterministic IDs, chain links, time, payload identity, and causal parents.
    pub fn verify(&self) -> Result<(), EventV2Error> {
        self.namespace.validate()?;

        let mut positions = BTreeMap::<StableId, usize>::new();
        for (index, event) in self.events.iter().enumerate() {
            validate_stable_id(&event.event_id)?;
            if positions.insert(event.event_id.clone(), index).is_some() {
                return Err(EventV2Error::DuplicateEventId(event.event_id.clone()));
            }
        }

        let mut expected_previous = None;
        let mut previous_tick = 0;
        for (index, event) in self.events.iter().enumerate() {
            if event.schema_version != CANONICAL_EVENT_SCHEMA_VERSION_V2 {
                return Err(EventV2Error::UnsupportedSchema {
                    expected: CANONICAL_EVENT_SCHEMA_VERSION_V2,
                    actual: event.schema_version,
                });
            }

            let ordinal = u64::try_from(index).map_err(|_| EventV2Error::EventOverflow)?;
            let expected_id = StableId::derive_v2(&self.namespace, self.seed, ordinal)?;
            if event.event_id != expected_id {
                return Err(EventV2Error::EventIdMismatch {
                    expected: expected_id,
                    actual: event.event_id.clone(),
                });
            }

            if event.previous_digest != expected_previous {
                return Err(EventV2Error::PreviousDigestMismatch {
                    event_id: event.event_id.clone(),
                    expected: expected_previous,
                    actual: event.previous_digest,
                });
            }

            if index > 0 && event.simulation_tick < previous_tick {
                return Err(EventV2Error::NonMonotonicTick {
                    event_id: event.event_id.clone(),
                    previous: previous_tick,
                    actual: event.simulation_tick,
                });
            }

            event.kind.validate()?;
            validate_optional_stable_id(event.actor_id.as_ref())?;
            validate_optional_stable_id(event.observer_id.as_ref())?;
            let parents = canonical_parents(&event.causal_parents)?;
            for parent in &parents {
                validate_stable_id(parent)?;
                match positions.get(parent).copied() {
                    None => {
                        return Err(EventV2Error::UnknownCausalParent {
                            event_id: event.event_id.clone(),
                            parent_id: parent.clone(),
                        });
                    }
                    Some(parent_index) if parent_index == index => {
                        return Err(EventV2Error::SelfCausalParent {
                            event_id: event.event_id.clone(),
                        });
                    }
                    Some(parent_index) if parent_index > index => {
                        return Err(EventV2Error::FutureCausalParent {
                            event_id: event.event_id.clone(),
                            parent_id: parent.clone(),
                        });
                    }
                    Some(_) => {}
                }
            }

            event.verify_payload_contract()?;
            let actual_digest = event.calculate_digest()?;
            if actual_digest != event.event_digest {
                return Err(EventV2Error::EventDigestMismatch {
                    event_id: event.event_id.clone(),
                    expected: event.event_digest,
                    actual: actual_digest,
                });
            }

            previous_tick = event.simulation_tick;
            expected_previous = Some(event.event_digest);
        }
        Ok(())
    }
}

fn canonical_parents(parents: &[StableId]) -> Result<Vec<StableId>, EventV2Error> {
    let mut canonical = parents.to_vec();
    canonical.sort();
    for pair in canonical.windows(2) {
        if pair[0] == pair[1] {
            return Err(EventV2Error::DuplicateCausalParent(pair[0].clone()));
        }
    }
    Ok(canonical)
}

fn deserialize_stable_id<'de, D>(deserializer: D) -> Result<StableId, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    StableId::parse(value).map_err(serde::de::Error::custom)
}

fn deserialize_optional_stable_id<'de, D>(deserializer: D) -> Result<Option<StableId>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(StableId::parse)
        .transpose()
        .map_err(serde::de::Error::custom)
}

fn deserialize_stable_ids<'de, D>(deserializer: D) -> Result<Vec<StableId>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer)?
        .into_iter()
        .map(|value| StableId::parse(value).map_err(serde::de::Error::custom))
        .collect()
}

fn deserialize_payload_schema<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validate_payload_schema(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn validate_stable_id(id: &StableId) -> Result<(), EventV2Error> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(EventV2Error::InvalidStableId)
}

fn validate_optional_stable_id(id: Option<&StableId>) -> Result<(), EventV2Error> {
    if let Some(id) = id {
        validate_stable_id(id)?;
    }
    Ok(())
}

fn validate_payload_schema(value: &str) -> Result<(), EventV2Error> {
    if valid_semantic_id(value) {
        Ok(())
    } else {
        Err(EventV2Error::InvalidPayloadSchema(value.to_owned()))
    }
}

fn valid_semantic_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SEMANTIC_ID_LEN
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
        })
}

/// Canonical v2 event-chain validation failures.
#[derive(Debug)]
pub enum EventV2Error {
    Canonical(CanonicalError),
    Namespace(NamespaceError),
    InvalidStableId(StateError),
    InvalidEventKind(String),
    InvalidPayloadSchema(String),
    UnsupportedSchema { expected: u32, actual: u32 },
    EventOverflow,
    EventIdMismatch { expected: StableId, actual: StableId },
    DuplicateEventId(StableId),
    PreviousDigestMismatch {
        event_id: StableId,
        expected: Option<EventDigestV2>,
        actual: Option<EventDigestV2>,
    },
    NonMonotonicTick {
        event_id: StableId,
        previous: u64,
        actual: u64,
    },
    DuplicateCausalParent(StableId),
    UnknownCausalParent { event_id: StableId, parent_id: StableId },
    FutureCausalParent { event_id: StableId, parent_id: StableId },
    SelfCausalParent { event_id: StableId },
    PayloadSchemaMismatch { expected: String, actual: String },
    PayloadDigestMismatch {
        event_id: StableId,
        expected: PayloadDigest,
        actual: PayloadDigest,
    },
    EventDigestMismatch {
        event_id: StableId,
        expected: EventDigestV2,
        actual: EventDigestV2,
    },
}

impl From<CanonicalError> for EventV2Error {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}

impl From<NamespaceError> for EventV2Error {
    fn from(error: NamespaceError) -> Self {
        Self::Namespace(error)
    }
}

impl fmt::Display for EventV2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => error.fmt(formatter),
            Self::Namespace(error) => error.fmt(formatter),
            Self::InvalidStableId(error) => error.fmt(formatter),
            Self::InvalidEventKind(value) => write!(formatter, "invalid canonical event kind: {value:?}"),
            Self::InvalidPayloadSchema(value) => {
                write!(formatter, "invalid canonical payload schema: {value:?}")
            }
            Self::UnsupportedSchema { expected, actual } => write!(
                formatter,
                "unsupported canonical event schema: expected {expected}, got {actual}"
            ),
            Self::EventOverflow => formatter.write_str("canonical event ordinal overflow"),
            Self::EventIdMismatch { expected, actual } => write!(
                formatter,
                "canonical event id mismatch: expected {expected}, got {actual}"
            ),
            Self::DuplicateEventId(event_id) => {
                write!(formatter, "duplicate canonical event id: {event_id}")
            }
            Self::PreviousDigestMismatch { event_id, .. } => {
                write!(formatter, "canonical event {event_id} previous digest mismatch")
            }
            Self::NonMonotonicTick {
                event_id,
                previous,
                actual,
            } => write!(
                formatter,
                "canonical event {event_id} moved backward from tick {previous} to {actual}"
            ),
            Self::DuplicateCausalParent(parent) => {
                write!(formatter, "duplicate canonical causal parent: {parent}")
            }
            Self::UnknownCausalParent { event_id, parent_id } => write!(
                formatter,
                "canonical event {event_id} cites unknown causal parent {parent_id}"
            ),
            Self::FutureCausalParent { event_id, parent_id } => write!(
                formatter,
                "canonical event {event_id} cites future causal parent {parent_id}"
            ),
            Self::SelfCausalParent { event_id } => {
                write!(formatter, "canonical event {event_id} cites itself as a causal parent")
            }
            Self::PayloadSchemaMismatch { expected, actual } => write!(
                formatter,
                "canonical payload schema mismatch: expected {expected}, got {actual}"
            ),
            Self::PayloadDigestMismatch { event_id, .. } => {
                write!(formatter, "canonical payload digest mismatch for event {event_id}")
            }
            Self::EventDigestMismatch { event_id, .. } => {
                write!(formatter, "canonical event digest mismatch for event {event_id}")
            }
        }
    }
}

impl Error for EventV2Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            Self::Namespace(error) => Some(error),
            Self::InvalidStableId(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{CanonicalDigest, CanonicalWriter};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestPayload {
        value: u32,
        /// Deliberately serialized but excluded from semantic payload identity.
        display_note: String,
    }

    impl CanonicalEventPayload for TestPayload {
        const PAYLOAD_SCHEMA: &'static str = "test.payload.v1";

        fn canonical_payload_digest(&self) -> PayloadDigest {
            let mut writer = CanonicalWriter::new(b"symtropy/test-payload/v1").expect("valid domain");
            writer.write_u32(self.value);
            PayloadDigest::new(writer.finish())
        }
    }

    fn namespace() -> StableIdNamespace {
        StableIdNamespace::parse("fold.event").expect("valid namespace")
    }

    fn kind(value: &str) -> StableEventKind {
        StableEventKind::parse(value).expect("valid kind")
    }

    fn payload(value: u32, display_note: &str) -> TestPayload {
        TestPayload {
            value,
            display_note: display_note.to_owned(),
        }
    }

    fn serialized_event_value() -> serde_json::Value {
        let mut chain = EventChainV2::new(namespace(), 19);
        chain
            .append(
                1,
                kind("test.event"),
                None,
                None,
                Vec::new(),
                payload(1, "event"),
            )
            .expect("event appends");
        serde_json::to_value(&chain.events()[0]).expect("event serializes")
    }

    #[test]
    fn deserialization_preserves_event_kind_validation() {
        let invalid = serde_json::from_str::<StableEventKind>("\"contains space\"");
        assert!(invalid.is_err());
    }

    #[test]
    fn v2_envelope_deserialization_rejects_invalid_semantic_identifiers() {
        let cases = [
            ("event_id", serde_json::json!("contains space")),
            ("actor_id", serde_json::json!("contains space")),
            ("observer_id", serde_json::json!("contains space")),
            ("causal_parents", serde_json::json!(["contains space"])),
            ("payload_schema", serde_json::json!("contains space")),
        ];

        for (field, invalid_value) in cases {
            let mut value = serialized_event_value();
            value[field] = invalid_value;
            let decoded = serde_json::from_value::<EventEnvelopeV2<TestPayload>>(value);
            assert!(decoded.is_err(), "invalid {field} must fail during deserialization");
        }
    }

    #[test]
    fn serializer_only_payload_changes_do_not_change_event_identity() {
        let mut first = EventChainV2::new(namespace(), 91);
        first
            .append(7, kind("fold.observed"), None, None, Vec::new(), payload(5, "a"))
            .expect("append first");

        let mut second = EventChainV2::new(namespace(), 91);
        second
            .append(7, kind("fold.observed"), None, None, Vec::new(), payload(5, "different serde bytes"))
            .expect("append second");

        assert_ne!(
            serde_json::to_vec(&first.events()[0].payload).expect("serialize"),
            serde_json::to_vec(&second.events()[0].payload).expect("serialize")
        );
        assert_eq!(first.head_digest(), second.head_digest());
    }

    #[test]
    fn causal_parent_order_is_set_semantics() {
        let mut first = EventChainV2::new(namespace(), 7);
        let a = first
            .append(1, kind("test.a"), None, None, Vec::new(), payload(1, "a"))
            .expect("a");
        let b = first
            .append(1, kind("test.b"), None, None, Vec::new(), payload(2, "b"))
            .expect("b");
        first
            .append(
                2,
                kind("test.child"),
                None,
                None,
                vec![a.clone(), b.clone()],
                payload(3, "child"),
            )
            .expect("child");

        let mut second = EventChainV2::new(namespace(), 7);
        let a2 = second
            .append(1, kind("test.a"), None, None, Vec::new(), payload(1, "a"))
            .expect("a");
        let b2 = second
            .append(1, kind("test.b"), None, None, Vec::new(), payload(2, "b"))
            .expect("b");
        second
            .append(
                2,
                kind("test.child"),
                None,
                None,
                vec![b2, a2],
                payload(3, "child"),
            )
            .expect("child");

        assert_eq!(first.head_digest(), second.head_digest());
    }

    #[test]
    fn duplicate_and_unknown_parents_fail_closed() {
        let mut chain = EventChainV2::new(namespace(), 2);
        let parent = chain
            .append(1, kind("test.parent"), None, None, Vec::new(), payload(1, "p"))
            .expect("parent");
        assert!(matches!(
            chain.append(
                2,
                kind("test.duplicate"),
                None,
                None,
                vec![parent.clone(), parent],
                payload(2, "d"),
            ),
            Err(EventV2Error::DuplicateCausalParent(_))
        ));

        let unknown = StableId::parse("event:unknown").expect("valid id");
        assert!(matches!(
            chain.append(
                2,
                kind("test.unknown"),
                None,
                None,
                vec![unknown],
                payload(2, "u"),
            ),
            Err(EventV2Error::UnknownCausalParent { .. })
        ));
    }

    #[test]
    fn verifier_rejects_self_consistent_future_parent() {
        let mut chain = EventChainV2::new(namespace(), 12);
        chain
            .append(1, kind("test.first"), None, None, Vec::new(), payload(1, "a"))
            .expect("first");
        chain
            .append(2, kind("test.future"), None, None, Vec::new(), payload(2, "b"))
            .expect("future");

        let future_id = chain.events[1].event_id.clone();
        chain.events[0].causal_parents = vec![future_id];
        chain.events[0].event_digest = chain.events[0].calculate_digest().expect("rehash test event");

        assert!(matches!(
            chain.verify(),
            Err(EventV2Error::FutureCausalParent { .. })
        ));
    }

    #[test]
    fn unknown_schema_is_not_interpreted_as_v2() {
        let mut chain = EventChainV2::new(namespace(), 3);
        chain
            .append(1, kind("test.event"), None, None, Vec::new(), payload(1, "a"))
            .expect("event");
        chain.events[0].schema_version = 99;
        assert!(matches!(
            chain.verify(),
            Err(EventV2Error::UnsupportedSchema { actual: 99, .. })
        ));
    }

    #[test]
    fn payload_digest_change_changes_event_digest() {
        let mut a = EventChainV2::new(namespace(), 5);
        a.append(1, kind("test.event"), None, None, Vec::new(), payload(1, "x"))
            .expect("a");
        let mut b = EventChainV2::new(namespace(), 5);
        b.append(1, kind("test.event"), None, None, Vec::new(), payload(2, "x"))
            .expect("b");
        assert_ne!(a.head_digest(), b.head_digest());
    }

    #[test]
    fn zero_digest_is_not_a_genesis_marker() {
        let zero = EventDigestV2::new(CanonicalDigest::from_bytes([0; 32]));
        let chain = EventChainV2::<TestPayload>::new(namespace(), 1);
        assert_eq!(chain.head_digest(), None);
        assert_ne!(Some(zero), chain.head_digest());
    }
}
