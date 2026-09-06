// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symtropy_game_state::{
    CanonicalEventPayload, CanonicalWriter, EventChainV2, EventV2Error, PayloadDigest,
    StableEventKind, StableId, StableIdNamespace,
};

const V1_STABLE_ID_PREIMAGE: &str = "73796d74726f70792f737461626c652d69642f763200000000000000000a666f6c642e6576656e74000000000000005b0000000000000000";
const V1_PAYLOAD_PREIMAGE: &str = "73796d74726f70792f746573742d7061796c6f61642f76310000000005";
const V1_EVENT_PREIMAGE: &str = "73796d74726f70792f67616d652d73746174652f6576656e742f76320000000002000000000000002b666f6c642e6576656e743a35316463663231353635663161633665326630643363363363333662356638370000000000000007000000000000000d666f6c642e6f6273657276656400000000000000000000000000000000000f746573742e7061796c6f61642e7631fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a8300";
const V2_STABLE_ID_PREIMAGE: &str = "73796d74726f70792f737461626c652d69642f763200000000000000000a666f6c642e6576656e74000000000000005b0000000000000001";
const V2_PAYLOAD_PREIMAGE: &str = "73796d74726f70792f746573742d7061796c6f61642f76310000000009";
const V2_EVENT_PREIMAGE: &str = "73796d74726f70792f67616d652d73746174652f6576656e742f76320000000002000000000000002b666f6c642e6576656e743a303066623934333131663662663362653830313630313332313530346435363000000000000000080000000000000013666f6c642e726577696e642e6170706c69656401000000000000000c6163746f723a706c6179657201000000000000000e6f627365727665723a616c6963650000000000000001000000000000002b666f6c642e6576656e743a3531646366323135363566316163366532663064336336336333366235663837000000000000000f746573742e7061796c6f61642e76311d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63017c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2";

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

fn decode_hex(input: &str) -> Vec<u8> {
    let chunks = input.as_bytes().chunks_exact(2);
    assert!(
        chunks.remainder().is_empty(),
        "hex fixture must have even length"
    );
    chunks
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect()
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("frozen fixture contains non-lowercase-hex byte"),
    }
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

fn digest_fixture(input: &str) -> [u8; 32] {
    decode_hex(input)
        .try_into()
        .expect("digest fixture must contain exactly 32 bytes")
}

fn prefix_fixture<const N: usize>(input: &str) -> [u8; N] {
    decode_hex(input)
        .try_into()
        .expect("prefix fixture must have the expected length")
}

#[test]
fn frozen_preimages_hash_independently_of_canonical_writer() {
    let vector_1_id = sha256(&decode_hex(V1_STABLE_ID_PREIMAGE));
    assert_eq!(
        &vector_1_id[..16],
        &prefix_fixture::<16>("51dcf21565f1ac6e2f0d3c63c36b5f87")
    );
    assert_eq!(
        sha256(&decode_hex(V1_PAYLOAD_PREIMAGE)),
        digest_fixture("fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a83")
    );
    assert_eq!(
        sha256(&decode_hex(V1_EVENT_PREIMAGE)),
        digest_fixture("7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2")
    );

    let vector_2_id = sha256(&decode_hex(V2_STABLE_ID_PREIMAGE));
    assert_eq!(
        &vector_2_id[..16],
        &prefix_fixture::<16>("00fb94311f6bf3be801601321504d560")
    );
    assert_eq!(
        sha256(&decode_hex(V2_PAYLOAD_PREIMAGE)),
        digest_fixture("1d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63")
    );
    assert_eq!(
        sha256(&decode_hex(V2_EVENT_PREIMAGE)),
        digest_fixture("bdb881578c4db99b954d4bbb1907adeaede631f9b8b49db3396c81c08dcc74a7")
    );
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

#[test]
fn append_rejects_non_monotonic_tick_without_mutating_chain() {
    let namespace = StableIdNamespace::parse("fold.event").expect("valid namespace");
    let mut chain = EventChainV2::new(namespace, 91);
    chain
        .append(
            10,
            StableEventKind::parse("fold.observed").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 5 },
        )
        .expect("first event appends");

    let head_before = chain.head_digest();
    let len_before = chain.events().len();
    let error = chain
        .append(
            9,
            StableEventKind::parse("fold.rewind.applied").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 9 },
        )
        .expect_err("backward simulation tick must fail");

    match error {
        EventV2Error::NonMonotonicTick {
            previous, actual, ..
        } => {
            assert_eq!(previous, 10);
            assert_eq!(actual, 9);
        }
        other => panic!("expected NonMonotonicTick, got {other:?}"),
    }
    assert_eq!(chain.events().len(), len_before);
    assert_eq!(chain.head_digest(), head_before);
}
