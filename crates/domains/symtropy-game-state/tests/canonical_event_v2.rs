// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use symtropy_game_state::{
    CanonicalEventPayload, CanonicalWriter, EventChainV2, PayloadDigest, StableEventKind, StableId,
    StableIdNamespace,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenPayload {
    value: u32,
}

impl CanonicalEventPayload for GoldenPayload {
    const PAYLOAD_SCHEMA: &'static str = "test.payload.v1";

    fn canonical_payload_digest(&self) -> PayloadDigest {
        let mut writer =
            CanonicalWriter::new(b"symtropy/test-payload/v1").expect("frozen valid domain");
        writer.write_u32(self.value);
        PayloadDigest::new(writer.finish())
    }
}

#[test]
fn canonical_event_v2_golden_vector_001() {
    let namespace = StableIdNamespace::parse("fold.event").expect("frozen valid namespace");
    let mut chain = EventChainV2::new(namespace, 91);
    let event_id = chain
        .append(
            7,
            StableEventKind::parse("fold.observed").expect("frozen valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 5 },
        )
        .expect("golden event appends");

    chain.verify().expect("golden chain verifies");
    let event = &chain.events()[0];

    // Independently derived from the frozen binary grammar in CANONICAL_EVENT_V2.md.
    assert_eq!(
        event_id.as_str(),
        "fold.event:51dcf21565f1ac6e2f0d3c63c36b5f87"
    );
    assert_eq!(
        event.payload_digest.canonical().to_hex(),
        "fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a83"
    );
    assert_eq!(
        event.event_digest.canonical().to_hex(),
        "7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2"
    );
}

#[test]
fn canonical_event_v2_golden_vector_002_non_genesis() {
    let namespace = StableIdNamespace::parse("fold.event").expect("frozen valid namespace");
    let mut chain = EventChainV2::new(namespace, 91);
    let parent_id = chain
        .append(
            7,
            StableEventKind::parse("fold.observed").expect("frozen valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 5 },
        )
        .expect("golden parent appends");
    let event_id = chain
        .append(
            8,
            StableEventKind::parse("fold.rewind.applied").expect("frozen valid event kind"),
            Some(StableId::parse("actor:player").expect("frozen valid actor id")),
            Some(StableId::parse("observer:alice").expect("frozen valid observer id")),
            vec![parent_id],
            GoldenPayload { value: 9 },
        )
        .expect("golden child appends");

    chain.verify().expect("golden chain verifies");
    let event = &chain.events()[1];

    // Independently derived from the frozen binary grammar in CANONICAL_EVENT_V2.md. This vector
    // exercises Some(actor), Some(observer), causal parents, and Some(previous_digest).
    assert_eq!(
        event_id.as_str(),
        "fold.event:00fb94311f6bf3be801601321504d560"
    );
    assert_eq!(
        event.payload_digest.canonical().to_hex(),
        "1d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63"
    );
    assert_eq!(
        event
            .previous_digest
            .expect("non-genesis event has previous digest")
            .canonical()
            .to_hex(),
        "7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2"
    );
    assert_eq!(
        event.event_digest.canonical().to_hex(),
        "bdb881578c4db99b954d4bbb1907adeaede631f9b8b49db3396c81c08dcc74a7"
    );
}
