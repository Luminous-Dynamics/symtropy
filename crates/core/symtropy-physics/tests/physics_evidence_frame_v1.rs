// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_physics::{
    PHYSICS_EVIDENCE_FRAME_V1_MAGIC, PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN,
    PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN, PHYSICS_EVIDENCE_FRAME_V1_VERSION,
    PhysicsEvidenceFrameKindV1, PhysicsEvidenceFrameV1Error,
    decode_physics_evidence_frame_v1, encode_physics_evidence_frame_v1,
};

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").unwrap();
    }
    out
}

fn raw_frame(domain: &[u8], version: u32, declared_len: u64, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(PHYSICS_EVIDENCE_FRAME_V1_MAGIC);
    frame.extend_from_slice(domain);
    frame.push(0);
    frame.extend_from_slice(&version.to_be_bytes());
    frame.extend_from_slice(&declared_len.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

#[test]
fn production_encoder_matches_checked_in_golden_vectors_exactly() {
    let corpus = include_str!("fixtures/physics_evidence_frame_v1_vectors.json");
    let cases: &[(&str, PhysicsEvidenceFrameKindV1, &[u8], &str)] = &[
        (
            "session-binding",
            PhysicsEvidenceFrameKindV1::EvidenceSessionBinding,
            b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09",
            "73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000000a00010203040506070809",
        ),
        (
            "endpoint-empty",
            PhysicsEvidenceFrameKindV1::EndpointSample,
            b"",
            "73796d74726f70792d706879736963732d65766964656e636500656e64706f696e742d73616d706c6500000000010000000000000000",
        ),
        (
            "consecutive-binary",
            PhysicsEvidenceFrameKindV1::ConsecutiveSampledPresence,
            b"\x00\x01\x02\xff",
            "73796d74726f70792d706879736963732d65766964656e636500636f6e73656375746976652d70726573656e636500000000010000000000000004000102ff",
        ),
        (
            "step-receipt",
            PhysicsEvidenceFrameKindV1::StepExecutionReceipt,
            b"\xde\xad\xbe\xef",
            "73796d74726f70792d706879736963732d65766964656e636500737465702d657865637574696f6e2d7265636569707400000000010000000000000004deadbeef",
        ),
        (
            "fixed-cadence",
            PhysicsEvidenceFrameKindV1::FixedCadenceStepReceipt,
            b"\x3f\x90\x00\x00\x00\x00\x00\x00",
            "73796d74726f70792d706879736963732d65766964656e63650066697865642d636164656e63652d737465702d72656365697074000000000100000000000000083f90000000000000",
        ),
        (
            "timed-nonutf8",
            PhysicsEvidenceFrameKindV1::TimedSampledPresenceTransition,
            b"\x00\xff\x80\x7f\x10",
            "73796d74726f70792d706879736963732d65766964656e63650074696d65642d73616d706c65642d7472616e736974696f6e0000000001000000000000000500ff807f10",
        ),
    ];

    for (name, kind, payload, expected_hex) in cases {
        let encoded = encode_physics_evidence_frame_v1(*kind, payload).unwrap();
        assert_eq!(to_hex(&encoded), *expected_hex, "{name}");
        assert!(corpus.contains(name));
        assert!(corpus.contains(expected_hex));

        let decoded = decode_physics_evidence_frame_v1(&encoded).unwrap();
        assert_eq!(decoded.kind(), *kind);
        assert_eq!(decoded.payload(), *payload);
        assert_eq!(
            encode_physics_evidence_frame_v1(decoded.kind(), decoded.payload()).unwrap(),
            encoded
        );
    }
}

#[test]
fn frame_constants_and_kind_domains_are_frozen_and_distinct() {
    assert_eq!(PHYSICS_EVIDENCE_FRAME_V1_MAGIC, b"symtropy-physics-evidence\0");
    assert_eq!(PHYSICS_EVIDENCE_FRAME_V1_VERSION, 1);
    assert_eq!(PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN, 1_048_576);
    assert_eq!(PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN, 32);

    let kinds = [
        PhysicsEvidenceFrameKindV1::EvidenceSessionBinding,
        PhysicsEvidenceFrameKindV1::EndpointSample,
        PhysicsEvidenceFrameKindV1::ConsecutiveSampledPresence,
        PhysicsEvidenceFrameKindV1::StepExecutionReceipt,
        PhysicsEvidenceFrameKindV1::FixedCadenceStepReceipt,
        PhysicsEvidenceFrameKindV1::TimedSampledPresenceTransition,
    ];
    for (i, left) in kinds.iter().enumerate() {
        assert!(left.domain_bytes().len() <= PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN);
        for right in &kinds[i + 1..] {
            assert_ne!(left.domain_bytes(), right.domain_bytes());
        }
    }

    let payload = b"same semantic-agnostic payload";
    let session = encode_physics_evidence_frame_v1(kinds[0], payload).unwrap();
    let endpoint = encode_physics_evidence_frame_v1(kinds[1], payload).unwrap();
    let timing = encode_physics_evidence_frame_v1(kinds[3], payload).unwrap();
    assert_ne!(session, endpoint);
    assert_ne!(endpoint, timing);
    assert_ne!(session, timing);
}

#[test]
fn non_utf8_payload_round_trips_without_text_normalization() {
    let payload = b"\x00\xff\x80\xfe\x7f\x00";
    let encoded = encode_physics_evidence_frame_v1(
        PhysicsEvidenceFrameKindV1::TimedSampledPresenceTransition,
        payload,
    )
    .unwrap();
    let decoded = decode_physics_evidence_frame_v1(&encoded).unwrap();
    assert_eq!(decoded.payload(), payload);
}

#[test]
fn decoder_rejects_wrong_or_truncated_magic() {
    assert_eq!(
        decode_physics_evidence_frame_v1(b"symtropy"),
        Err(PhysicsEvidenceFrameV1Error::TruncatedMagic)
    );

    let mut wrong = encode_physics_evidence_frame_v1(
        PhysicsEvidenceFrameKindV1::EndpointSample,
        b"",
    )
    .unwrap();
    wrong[0] ^= 0x01;
    assert_eq!(
        decode_physics_evidence_frame_v1(&wrong),
        Err(PhysicsEvidenceFrameV1Error::WrongMagic)
    );
}

#[test]
fn decoder_rejects_missing_or_unknown_kind_domain() {
    let mut missing = PHYSICS_EVIDENCE_FRAME_V1_MAGIC.to_vec();
    missing.extend(std::iter::repeat_n(
        b'x',
        PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN + 1,
    ));
    assert_eq!(
        decode_physics_evidence_frame_v1(&missing),
        Err(PhysicsEvidenceFrameV1Error::MissingKindTerminator)
    );

    let unknown = raw_frame(b"unknown-kind", 1, 0, b"");
    assert_eq!(
        decode_physics_evidence_frame_v1(&unknown),
        Err(PhysicsEvidenceFrameV1Error::UnknownKind)
    );
}

#[test]
fn decoder_rejects_version_and_fixed_header_aliases() {
    let unsupported = raw_frame(b"endpoint-sample", 2, 0, b"");
    assert_eq!(
        decode_physics_evidence_frame_v1(&unsupported),
        Err(PhysicsEvidenceFrameV1Error::UnsupportedVersion { version: 2 })
    );

    let mut truncated = PHYSICS_EVIDENCE_FRAME_V1_MAGIC.to_vec();
    truncated.extend_from_slice(b"endpoint-sample\0\x00\x00");
    assert_eq!(
        decode_physics_evidence_frame_v1(&truncated),
        Err(PhysicsEvidenceFrameV1Error::TruncatedHeader)
    );
}

#[test]
fn decoder_rejects_oversized_declared_length_before_payload_allocation() {
    let oversized = raw_frame(
        b"endpoint-sample",
        1,
        PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN as u64 + 1,
        b"",
    );
    assert_eq!(
        decode_physics_evidence_frame_v1(&oversized),
        Err(PhysicsEvidenceFrameV1Error::DeclaredPayloadTooLarge {
            len: PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN as u64 + 1,
        })
    );
}

#[test]
fn decoder_rejects_truncated_payload_and_trailing_bytes() {
    let truncated = raw_frame(b"endpoint-sample", 1, 4, b"abc");
    assert_eq!(
        decode_physics_evidence_frame_v1(&truncated),
        Err(PhysicsEvidenceFrameV1Error::TruncatedPayload {
            declared: 4,
            available: 3,
        })
    );

    let mut trailing = encode_physics_evidence_frame_v1(
        PhysicsEvidenceFrameKindV1::EndpointSample,
        b"abc",
    )
    .unwrap();
    trailing.push(0);
    assert_eq!(
        decode_physics_evidence_frame_v1(&trailing),
        Err(PhysicsEvidenceFrameV1Error::TrailingBytes { trailing: 1 })
    );
}

#[test]
fn payload_bound_is_exact_for_encoder_and_decoder() {
    let max_payload = vec![0xa5; PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN];
    let frame = encode_physics_evidence_frame_v1(
        PhysicsEvidenceFrameKindV1::StepExecutionReceipt,
        &max_payload,
    )
    .unwrap();
    let decoded = decode_physics_evidence_frame_v1(&frame).unwrap();
    assert_eq!(decoded.payload().len(), PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN);
    assert_eq!(decoded.payload()[0], 0xa5);
    assert_eq!(decoded.payload()[PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN - 1], 0xa5);

    let oversized_payload = vec![0; PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN + 1];
    assert_eq!(
        encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::StepExecutionReceipt,
            &oversized_payload,
        ),
        Err(PhysicsEvidenceFrameV1Error::PayloadTooLarge {
            len: PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN + 1,
        })
    );
}
