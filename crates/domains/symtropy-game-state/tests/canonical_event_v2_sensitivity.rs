// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::{
    CanonicalEventPayload, CanonicalWriter, EventChainV2, EventV2Error, PayloadDigest,
    StableEventKind, StableId, StableIdNamespace,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PayloadV1 {
    value: u32,
}

impl CanonicalEventPayload for PayloadV1 {
    const PAYLOAD_SCHEMA: &'static str = "test.payload.v1";

    fn canonical_payload_digest(&self) -> PayloadDigest {
        payload_digest(self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PayloadV2 {
    value: u32,
}

impl CanonicalEventPayload for PayloadV2 {
    const PAYLOAD_SCHEMA: &'static str = "test.payload.v2";

    fn canonical_payload_digest(&self) -> PayloadDigest {
        payload_digest(self.value)
    }
}

fn payload_digest(value: u32) -> PayloadDigest {
    let mut writer =
        CanonicalWriter::new(b"symtropy/test-payload/v1").expect("frozen valid domain");
    writer.write_u32(value);
    PayloadDigest::new(writer.finish())
}

fn namespace() -> StableIdNamespace {
    StableIdNamespace::parse("fold.event").expect("frozen valid namespace")
}

fn kind(value: &str) -> StableEventKind {
    StableEventKind::parse(value).expect("frozen valid event kind")
}

fn id(value: &str) -> StableId {
    StableId::parse(value).expect("frozen valid stable id")
}

fn single_event_head(
    tick: u64,
    event_kind: &str,
    actor: Option<StableId>,
    observer: Option<StableId>,
) -> symtropy_game_state::EventDigestV2 {
    let mut chain = EventChainV2::new(namespace(), 44);
    chain
        .append(
            tick,
            kind(event_kind),
            actor,
            observer,
            Vec::new(),
            PayloadV1 { value: 7 },
        )
        .expect("event appends");
    chain.head_digest().expect("one-event chain has a head")
}

#[test]
fn canonical_event_identity_is_sensitive_to_each_core_envelope_field() {
    let baseline = single_event_head(10, "fold.observed", None, None);

    assert_ne!(
        baseline,
        single_event_head(11, "fold.observed", None, None),
        "simulation tick must affect canonical identity"
    );
    assert_ne!(
        baseline,
        single_event_head(10, "fold.changed", None, None),
        "event kind must affect canonical identity"
    );
    assert_ne!(
        baseline,
        single_event_head(10, "fold.observed", Some(id("actor:gerald")), None),
        "actor must affect canonical identity"
    );
    assert_ne!(
        baseline,
        single_event_head(10, "fold.observed", None, Some(id("observer:alice"))),
        "observer must affect canonical identity"
    );
}

#[test]
fn payload_schema_is_bound_even_when_semantic_payload_digest_is_equal() {
    let mut v1 = EventChainV2::new(namespace(), 51);
    v1.append(
        3,
        kind("fold.observed"),
        None,
        None,
        Vec::new(),
        PayloadV1 { value: 9 },
    )
    .expect("v1 payload event appends");

    let mut v2 = EventChainV2::new(namespace(), 51);
    v2.append(
        3,
        kind("fold.observed"),
        None,
        None,
        Vec::new(),
        PayloadV2 { value: 9 },
    )
    .expect("v2 payload event appends");

    assert_eq!(
        v1.events()[0].payload_digest,
        v2.events()[0].payload_digest,
        "fixture requires equal domain-owned payload digests"
    );
    assert_ne!(
        v1.head_digest(),
        v2.head_digest(),
        "payload schema must remain part of event identity"
    );
}

#[test]
fn causal_parent_membership_is_bound_but_parent_vector_order_is_not() {
    let mut without_parent = EventChainV2::new(namespace(), 60);
    without_parent
        .append(
            1,
            kind("test.parent"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 1 },
        )
        .expect("parent appends");
    without_parent
        .append(
            2,
            kind("test.child"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 2 },
        )
        .expect("child appends");

    let mut with_parent = EventChainV2::new(namespace(), 60);
    let parent = with_parent
        .append(
            1,
            kind("test.parent"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 1 },
        )
        .expect("parent appends");
    with_parent
        .append(
            2,
            kind("test.child"),
            None,
            None,
            vec![parent],
            PayloadV1 { value: 2 },
        )
        .expect("child appends");

    assert_ne!(without_parent.head_digest(), with_parent.head_digest());
}

#[test]
fn previous_digest_is_bound_into_each_non_genesis_event() {
    let mut first_history = EventChainV2::new(namespace(), 70);
    first_history
        .append(
            1,
            kind("test.parent"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 1 },
        )
        .expect("first parent appends");
    first_history
        .append(
            2,
            kind("test.child"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 2 },
        )
        .expect("first child appends");

    let mut second_history = EventChainV2::new(namespace(), 70);
    second_history
        .append(
            1,
            kind("test.parent"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 99 },
        )
        .expect("second parent appends");
    second_history
        .append(
            2,
            kind("test.child"),
            None,
            None,
            Vec::new(),
            PayloadV1 { value: 2 },
        )
        .expect("second child appends");

    assert_ne!(first_history.head_digest(), second_history.head_digest());
}

#[test]
fn self_parent_is_rejected_before_unknown_parent_classification() {
    let namespace = namespace();
    let self_id = StableId::derive_v2(&namespace, 81, 0).expect("validated v2 id derivation");
    let mut chain = EventChainV2::new(namespace, 81);

    assert!(matches!(
        chain.append(
            1,
            kind("test.self-parent"),
            None,
            None,
            vec![self_id],
            PayloadV1 { value: 1 },
        ),
        Err(EventV2Error::SelfCausalParent { .. })
    ));
}
