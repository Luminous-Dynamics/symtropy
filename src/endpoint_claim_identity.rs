// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical semantic claim identity for persistent endpoint samples.
//!
//! PHYS-EVID-01A separates the unique semantic sample position from the value
//! asserted at that position. It derives both transcripts only from a
//! PHYS-EVID-02C-admitted endpoint frame. The transcripts are language-neutral
//! representation records, not cryptographic commitments or authenticated
//! authority.

use crate::endpoint_sample_codec::{
    CanonicalEndpointMembershipV1, DecodedEndpointSampleV1, EndpointSampleCodecV1Error,
    decode_endpoint_sample_v1,
};

pub const ENDPOINT_SAMPLE_CLAIM_POSITION_V1_MAGIC: &[u8] =
    b"symtropy-physics-claim-position\0";
pub const ENDPOINT_SAMPLE_CLAIM_VALUE_V1_MAGIC: &[u8] = b"symtropy-physics-claim-value\0";
pub const ENDPOINT_SAMPLE_CLAIM_V1_DOMAIN: &[u8] = b"endpoint-sample-v1\0";
pub const ENDPOINT_SAMPLE_CLAIM_TRANSCRIPT_V1_VERSION: u32 = 1;

const POSITION_FIXED_V1_LEN: usize = 157;
const POSITION_PER_AXIS_V1_LEN: usize = 16;
const VALUE_FIXED_V1_LEN: usize = 55;
const VALUE_PER_AXIS_V1_LEN: usize = 32;

/// Canonical semantic position for one admitted persistent endpoint sample.
///
/// The private bytes identify *which proposition at which qualified sample
/// coordinate* is being asserted. They deliberately exclude membership and all
/// observed/derived geometry values so incompatible results at the same
/// position can be surfaced as contradictions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEndpointSampleClaimPositionV1 {
    bytes: Vec<u8>,
}

impl CanonicalEndpointSampleClaimPositionV1 {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// Canonical semantic value asserted at one endpoint sample position.
///
/// This contains the theorem-relevant observed translations plus PHYS-OBS-03
/// derived center/offset/membership facts admitted by PHYS-EVID-02C.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEndpointSampleClaimValueV1 {
    bytes: Vec<u8>,
}

impl CanonicalEndpointSampleClaimValueV1 {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// One canonically admitted endpoint semantic claim split into position/value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEndpointSampleClaimV1 {
    position: CanonicalEndpointSampleClaimPositionV1,
    value: CanonicalEndpointSampleClaimValueV1,
}

impl CanonicalEndpointSampleClaimV1 {
    pub const fn position(&self) -> &CanonicalEndpointSampleClaimPositionV1 {
        &self.position
    }

    pub const fn value(&self) -> &CanonicalEndpointSampleClaimValueV1 {
        &self.value
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EndpointSampleClaimComparisonV1 {
    /// The records assert different semantic sample positions/propositions.
    DistinctPosition,
    /// The records assert the exact same position and exact same value.
    Duplicate,
    /// The records assert the same position but incompatible admitted values.
    Contradiction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndpointSampleClaimIdentityV1Error {
    EndpointSample(EndpointSampleCodecV1Error),
    TranscriptSizeOverflow,
    DecodedInvariant,
}

/// Derive canonical semantic position/value transcripts from one admitted
/// PHYS-EVID-02C endpoint frame.
///
/// This function always passes through `decode_endpoint_sample_v1` first. It
/// cannot mint a claim identity from caller-supplied detached fields.
pub fn derive_endpoint_sample_claim_v1(
    frame: &[u8],
) -> Result<CanonicalEndpointSampleClaimV1, EndpointSampleClaimIdentityV1Error> {
    let decoded = decode_endpoint_sample_v1(frame)
        .map_err(EndpointSampleClaimIdentityV1Error::EndpointSample)?;
    derive_from_decoded_endpoint_sample_v1(&decoded)
}

/// Compare two already-derived endpoint semantic claims.
pub fn compare_endpoint_sample_claims_v1(
    first: &CanonicalEndpointSampleClaimV1,
    second: &CanonicalEndpointSampleClaimV1,
) -> EndpointSampleClaimComparisonV1 {
    if first.position != second.position {
        EndpointSampleClaimComparisonV1::DistinctPosition
    } else if first.value == second.value {
        EndpointSampleClaimComparisonV1::Duplicate
    } else {
        EndpointSampleClaimComparisonV1::Contradiction
    }
}

/// Decode, derive, and compare two canonical endpoint frames.
///
/// The result is semantic classification only. Without PHYS-EVID-03
/// authentication, `Contradiction` does not yet prove an authority fork.
pub fn classify_endpoint_sample_frames_v1(
    first_frame: &[u8],
    second_frame: &[u8],
) -> Result<EndpointSampleClaimComparisonV1, EndpointSampleClaimIdentityV1Error> {
    let first = derive_endpoint_sample_claim_v1(first_frame)?;
    let second = derive_endpoint_sample_claim_v1(second_frame)?;
    Ok(compare_endpoint_sample_claims_v1(&first, &second))
}

fn derive_from_decoded_endpoint_sample_v1(
    decoded: &DecodedEndpointSampleV1<'_>,
) -> Result<CanonicalEndpointSampleClaimV1, EndpointSampleClaimIdentityV1Error> {
    let dimension = decoded.dimension();
    let dimension_u16 = u16::try_from(dimension)
        .map_err(|_| EndpointSampleClaimIdentityV1Error::DecodedInvariant)?;

    let position_len = POSITION_FIXED_V1_LEN
        .checked_add(
            POSITION_PER_AXIS_V1_LEN
                .checked_mul(dimension)
                .ok_or(EndpointSampleClaimIdentityV1Error::TranscriptSizeOverflow)?,
        )
        .ok_or(EndpointSampleClaimIdentityV1Error::TranscriptSizeOverflow)?;
    let value_len = VALUE_FIXED_V1_LEN
        .checked_add(
            VALUE_PER_AXIS_V1_LEN
                .checked_mul(dimension)
                .ok_or(EndpointSampleClaimIdentityV1Error::TranscriptSizeOverflow)?,
        )
        .ok_or(EndpointSampleClaimIdentityV1Error::TranscriptSizeOverflow)?;

    let session = decoded.session_binding();

    let mut position = Vec::with_capacity(position_len);
    position.extend_from_slice(ENDPOINT_SAMPLE_CLAIM_POSITION_V1_MAGIC);
    position.extend_from_slice(ENDPOINT_SAMPLE_CLAIM_V1_DOMAIN);
    position.extend_from_slice(&ENDPOINT_SAMPLE_CLAIM_TRANSCRIPT_V1_VERSION.to_be_bytes());
    position.extend_from_slice(&dimension_u16.to_be_bytes());
    position.extend_from_slice(session.session_id_bytes());
    position.extend_from_slice(&session.physical_authority_raw().to_be_bytes());
    position.extend_from_slice(&session.world_generation_raw().to_be_bytes());
    position.extend_from_slice(&session.temporal_incarnation_raw().to_be_bytes());
    position.extend_from_slice(&session.session_profile_code().to_be_bytes());
    position.extend_from_slice(&decoded.step_index().to_be_bytes());
    position.extend_from_slice(&decoded.target_net_id().to_be_bytes());
    position.extend_from_slice(&decoded.anchor_net_id().to_be_bytes());
    for axis in 0..dimension {
        push_axis_bits(&mut position, decoded.center_offset_bits(axis))?;
    }
    for axis in 0..dimension {
        push_axis_bits(&mut position, decoded.half_extent_bits(axis))?;
    }
    if position.len() != position_len {
        return Err(EndpointSampleClaimIdentityV1Error::DecodedInvariant);
    }

    let mut value = Vec::with_capacity(value_len);
    value.extend_from_slice(ENDPOINT_SAMPLE_CLAIM_VALUE_V1_MAGIC);
    value.extend_from_slice(ENDPOINT_SAMPLE_CLAIM_V1_DOMAIN);
    value.extend_from_slice(&ENDPOINT_SAMPLE_CLAIM_TRANSCRIPT_V1_VERSION.to_be_bytes());
    value.extend_from_slice(&dimension_u16.to_be_bytes());
    value.push(match decoded.membership() {
        CanonicalEndpointMembershipV1::Outside => 0,
        CanonicalEndpointMembershipV1::Inside => 1,
    });
    for axis in 0..dimension {
        push_axis_bits(&mut value, decoded.target_translation_bits(axis))?;
    }
    for axis in 0..dimension {
        push_axis_bits(&mut value, decoded.anchor_translation_bits(axis))?;
    }
    for axis in 0..dimension {
        push_axis_bits(&mut value, decoded.region_center_bits(axis))?;
    }
    for axis in 0..dimension {
        push_axis_bits(&mut value, decoded.offset_from_center_bits(axis))?;
    }
    if value.len() != value_len {
        return Err(EndpointSampleClaimIdentityV1Error::DecodedInvariant);
    }

    Ok(CanonicalEndpointSampleClaimV1 {
        position: CanonicalEndpointSampleClaimPositionV1 { bytes: position },
        value: CanonicalEndpointSampleClaimValueV1 { bytes: value },
    })
}

fn push_axis_bits(
    output: &mut Vec<u8>,
    bits: Option<u64>,
) -> Result<(), EndpointSampleClaimIdentityV1Error> {
    let bits = bits.ok_or(EndpointSampleClaimIdentityV1Error::DecodedInvariant)?;
    output.extend_from_slice(&bits.to_be_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::{
        PhysicsEvidenceFrameKindV1, decode_physics_evidence_frame_v1,
        encode_physics_evidence_frame_v1,
    };

    const GOLDEN_ENDPOINT_FRAME_HEX: &str = "73796d74726f70792d706879736963732d65766964656e636500656e64706f696e742d73616d706c6500000000010000000000000140000000010003008f73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728000000010000000000000001010203040506070811121314151617180100000000000000003fe0000000000000bfe000000000000040000000000000004008000000000000401000000000000040260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";
    const GOLDEN_POSITION_HEX: &str = "73796d74726f70792d706879736963732d636c61696d2d706f736974696f6e00656e64706f696e742d73616d706c652d763100000000010003000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f2021222324252627280000000100000000000000010102030405060708111213141516171800000000000000003fe0000000000000bfe0000000000000400000000000000040080000000000004010000000000000";
    const GOLDEN_VALUE_HEX: &str = "73796d74726f70792d706879736963732d636c61696d2d76616c756500656e64706f696e742d73616d706c652d7631000000000100030140260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";

    const MEMBERSHIP_OFFSET: usize = 175;
    const TARGET_TRANSLATION_X: usize = 224;
    const REGION_CENTER_X: usize = 272;
    const OFFSET_X: usize = 296;

    fn from_hex(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect()
    }

    fn to_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut out, "{byte:02x}").unwrap();
        }
        out
    }

    fn golden_frame() -> Vec<u8> {
        from_hex(GOLDEN_ENDPOINT_FRAME_HEX)
    }

    fn mutate_payload<F>(frame: &[u8], mutate: F) -> Vec<u8>
    where
        F: FnOnce(&mut Vec<u8>),
    {
        let decoded = decode_physics_evidence_frame_v1(frame).unwrap();
        assert_eq!(decoded.kind(), PhysicsEvidenceFrameKindV1::EndpointSample);
        let mut payload = decoded.payload().to_vec();
        mutate(&mut payload);
        encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, &payload)
            .unwrap()
    }

    fn mutate_nested_session<F>(frame: &[u8], mutate: F) -> Vec<u8>
    where
        F: FnOnce(&mut Vec<u8>),
    {
        mutate_payload(frame, |payload| {
            let nested = payload[8..151].to_vec();
            let decoded = decode_physics_evidence_frame_v1(&nested).unwrap();
            assert_eq!(
                decoded.kind(),
                PhysicsEvidenceFrameKindV1::EvidenceSessionBinding
            );
            let mut session_payload = decoded.payload().to_vec();
            mutate(&mut session_payload);
            let rebuilt = encode_physics_evidence_frame_v1(
                PhysicsEvidenceFrameKindV1::EvidenceSessionBinding,
                &session_payload,
            )
            .unwrap();
            assert_eq!(rebuilt.len(), 143);
            payload[8..151].copy_from_slice(&rebuilt);
        })
    }

    #[test]
    fn golden_endpoint_derives_exact_language_neutral_position_and_value() {
        let claim = derive_endpoint_sample_claim_v1(&golden_frame()).unwrap();
        assert_eq!(claim.position().len(), 205);
        assert_eq!(claim.value().len(), 151);
        assert_eq!(to_hex(claim.position().as_bytes()), GOLDEN_POSITION_HEX);
        assert_eq!(to_hex(claim.value().as_bytes()), GOLDEN_VALUE_HEX);

        let fixture = include_str!("../tests/fixtures/endpoint_claim_identity_v1_vector.json");
        assert!(fixture.contains(GOLDEN_POSITION_HEX));
        assert!(fixture.contains(GOLDEN_VALUE_HEX));
    }

    #[test]
    fn repeated_transport_of_same_canonical_claim_is_duplicate() {
        let frame = golden_frame();
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &frame).unwrap(),
            EndpointSampleClaimComparisonV1::Duplicate
        );

        let first = derive_endpoint_sample_claim_v1(&frame).unwrap();
        let second = derive_endpoint_sample_claim_v1(&from_hex(GOLDEN_ENDPOINT_FRAME_HEX)).unwrap();
        assert_eq!(first.position(), second.position());
        assert_eq!(first.value(), second.value());
    }

    #[test]
    fn same_position_with_different_admitted_value_is_contradiction() {
        let frame = golden_frame();
        let outside = mutate_payload(&frame, |payload| {
            payload[MEMBERSHIP_OFFSET] = 0;
            payload[TARGET_TRANSLATION_X..TARGET_TRANSLATION_X + 8]
                .copy_from_slice(&13.0_f64.to_bits().to_be_bytes());
            payload[OFFSET_X..OFFSET_X + 8]
                .copy_from_slice(&3.0_f64.to_bits().to_be_bytes());
        });

        let first = derive_endpoint_sample_claim_v1(&frame).unwrap();
        let second = derive_endpoint_sample_claim_v1(&outside).unwrap();
        assert_eq!(first.position(), second.position());
        assert_ne!(first.value(), second.value());
        assert_eq!(
            compare_endpoint_sample_claims_v1(&first, &second),
            EndpointSampleClaimComparisonV1::Contradiction
        );
    }

    #[test]
    fn adjacent_step_is_a_distinct_position() {
        let frame = golden_frame();
        let next = mutate_payload(&frame, |payload| {
            payload[151..159].copy_from_slice(&2_u64.to_be_bytes());
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &next).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );
    }

    #[test]
    fn different_persistent_session_is_a_distinct_position() {
        let frame = golden_frame();
        let other = mutate_nested_session(&frame, |payload| {
            payload[4] = 0x80;
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &other).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );
    }

    #[test]
    fn different_temporal_incarnation_is_a_distinct_position() {
        let frame = golden_frame();
        let other = mutate_nested_session(&frame, |payload| {
            payload[68..76].copy_from_slice(&0x3132_3334_3536_3738_u64.to_be_bytes());
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &other).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );
    }

    #[test]
    fn target_anchor_and_region_changes_are_distinct_positions() {
        let frame = golden_frame();

        let target = mutate_payload(&frame, |payload| {
            payload[159..167].copy_from_slice(&0x0203_0405_0607_0809_u64.to_be_bytes());
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &target).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );

        let anchor = mutate_payload(&frame, |payload| {
            payload[167..175].copy_from_slice(&0x2122_2324_2526_2728_u64.to_be_bytes());
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &anchor).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );

        let center_offset = mutate_payload(&frame, |payload| {
            payload[176..184].copy_from_slice(&1.0_f64.to_bits().to_be_bytes());
            payload[REGION_CENTER_X..REGION_CENTER_X + 8]
                .copy_from_slice(&11.0_f64.to_bits().to_be_bytes());
            payload[OFFSET_X..OFFSET_X + 8]
                .copy_from_slice(&0.0_f64.to_bits().to_be_bytes());
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &center_offset).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );

        let half_extent = mutate_payload(&frame, |payload| {
            payload[200..208].copy_from_slice(&3.0_f64.to_bits().to_be_bytes());
        });
        assert_eq!(
            classify_endpoint_sample_frames_v1(&frame, &half_extent).unwrap(),
            EndpointSampleClaimComparisonV1::DistinctPosition
        );
    }

    #[test]
    fn malformed_endpoint_bytes_reject_before_claim_derivation() {
        let bad = mutate_payload(&golden_frame(), |payload| {
            payload[MEMBERSHIP_OFFSET] = 9;
        });
        assert!(matches!(
            derive_endpoint_sample_claim_v1(&bad),
            Err(EndpointSampleClaimIdentityV1Error::EndpointSample(
                EndpointSampleCodecV1Error::InvalidMembership { code: 9 }
            ))
        ));
    }
}
