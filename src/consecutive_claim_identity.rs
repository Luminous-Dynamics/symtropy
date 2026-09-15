// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordered semantic claim identity for persistent consecutive sampled presence.
//!
//! PHYS-EVID-01B composes exact PHYS-EVID-01A endpoint semantic transcripts
//! only after PHYS-EVID-02D admits the ordered persistent relation. Raw endpoint
//! or relation framing bytes never become part of derived semantic identity.

use crate::consecutive_presence_codec::{
    ConsecutivePresenceCodecV1Error, decode_consecutive_sampled_presence_v1,
};
use crate::endpoint_claim_identity::{
    ENDPOINT_SAMPLE_CLAIM_POSITION_V1_MAGIC, ENDPOINT_SAMPLE_CLAIM_VALUE_V1_MAGIC,
    EndpointSampleClaimIdentityV1Error, derive_endpoint_sample_claim_v1,
};

pub const CONSECUTIVE_PRESENCE_CLAIM_V1_DOMAIN: &[u8] = b"consecutive-presence-v1\0";
pub const CONSECUTIVE_PRESENCE_CLAIM_TRANSCRIPT_V1_VERSION: u32 = 1;

/// Canonical semantic position for one admitted ordered consecutive-presence claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalConsecutivePresenceClaimPositionV1 {
    bytes: Vec<u8>,
}

impl CanonicalConsecutivePresenceClaimPositionV1 {
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

/// Canonical semantic value supporting one admitted ordered consecutive-presence claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalConsecutivePresenceClaimValueV1 {
    bytes: Vec<u8>,
}

impl CanonicalConsecutivePresenceClaimValueV1 {
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

/// One canonically admitted derived claim split into semantic position/value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalConsecutivePresenceClaimV1 {
    position: CanonicalConsecutivePresenceClaimPositionV1,
    value: CanonicalConsecutivePresenceClaimValueV1,
}

impl CanonicalConsecutivePresenceClaimV1 {
    pub const fn position(&self) -> &CanonicalConsecutivePresenceClaimPositionV1 {
        &self.position
    }

    pub const fn value(&self) -> &CanonicalConsecutivePresenceClaimValueV1 {
        &self.value
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConsecutivePresenceClaimComparisonV1 {
    DistinctPosition,
    Duplicate,
    Contradiction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsecutivePresenceClaimIdentityV1Error {
    Relation(ConsecutivePresenceCodecV1Error),
    EndpointClaim(EndpointSampleClaimIdentityV1Error),
    TranscriptSizeOverflow,
}

/// Derive ordered semantic position/value transcripts from one admitted
/// PHYS-EVID-02D relation frame.
///
/// The function first delegates relation admission/order to PHYS-EVID-02D,
/// then delegates each predecessor's semantic identity to PHYS-EVID-01A. It
/// never serializes endpoint fields itself and never copies raw relation or
/// endpoint frame bytes into the derived transcripts.
pub fn derive_consecutive_presence_claim_v1(
    frame: &[u8],
) -> Result<CanonicalConsecutivePresenceClaimV1, ConsecutivePresenceClaimIdentityV1Error> {
    let decoded = decode_consecutive_sampled_presence_v1(frame)
        .map_err(ConsecutivePresenceClaimIdentityV1Error::Relation)?;
    let previous = derive_endpoint_sample_claim_v1(decoded.previous_frame())
        .map_err(ConsecutivePresenceClaimIdentityV1Error::EndpointClaim)?;
    let current = derive_endpoint_sample_claim_v1(decoded.current_frame())
        .map_err(ConsecutivePresenceClaimIdentityV1Error::EndpointClaim)?;

    let position = compose_ordered_transcript(
        ENDPOINT_SAMPLE_CLAIM_POSITION_V1_MAGIC,
        previous.position().as_bytes(),
        current.position().as_bytes(),
    )?;
    let value = compose_ordered_transcript(
        ENDPOINT_SAMPLE_CLAIM_VALUE_V1_MAGIC,
        previous.value().as_bytes(),
        current.value().as_bytes(),
    )?;

    Ok(CanonicalConsecutivePresenceClaimV1 {
        position: CanonicalConsecutivePresenceClaimPositionV1 { bytes: position },
        value: CanonicalConsecutivePresenceClaimValueV1 { bytes: value },
    })
}

pub fn compare_consecutive_presence_claims_v1(
    first: &CanonicalConsecutivePresenceClaimV1,
    second: &CanonicalConsecutivePresenceClaimV1,
) -> ConsecutivePresenceClaimComparisonV1 {
    if first.position != second.position {
        ConsecutivePresenceClaimComparisonV1::DistinctPosition
    } else if first.value == second.value {
        ConsecutivePresenceClaimComparisonV1::Duplicate
    } else {
        ConsecutivePresenceClaimComparisonV1::Contradiction
    }
}

pub fn classify_consecutive_presence_frames_v1(
    first_frame: &[u8],
    second_frame: &[u8],
) -> Result<ConsecutivePresenceClaimComparisonV1, ConsecutivePresenceClaimIdentityV1Error> {
    let first = derive_consecutive_presence_claim_v1(first_frame)?;
    let second = derive_consecutive_presence_claim_v1(second_frame)?;
    Ok(compare_consecutive_presence_claims_v1(&first, &second))
}

fn compose_ordered_transcript(
    magic: &[u8],
    previous: &[u8],
    current: &[u8],
) -> Result<Vec<u8>, ConsecutivePresenceClaimIdentityV1Error> {
    let previous_len = u32::try_from(previous.len())
        .map_err(|_| ConsecutivePresenceClaimIdentityV1Error::TranscriptSizeOverflow)?;
    let current_len = u32::try_from(current.len())
        .map_err(|_| ConsecutivePresenceClaimIdentityV1Error::TranscriptSizeOverflow)?;
    let total = magic
        .len()
        .checked_add(CONSECUTIVE_PRESENCE_CLAIM_V1_DOMAIN.len())
        .and_then(|value| value.checked_add(4 + 4))
        .and_then(|value| value.checked_add(previous.len()))
        .and_then(|value| value.checked_add(4))
        .and_then(|value| value.checked_add(current.len()))
        .ok_or(ConsecutivePresenceClaimIdentityV1Error::TranscriptSizeOverflow)?;

    let mut transcript = Vec::with_capacity(total);
    transcript.extend_from_slice(magic);
    transcript.extend_from_slice(CONSECUTIVE_PRESENCE_CLAIM_V1_DOMAIN);
    transcript.extend_from_slice(&CONSECUTIVE_PRESENCE_CLAIM_TRANSCRIPT_V1_VERSION.to_be_bytes());
    transcript.extend_from_slice(&previous_len.to_be_bytes());
    transcript.extend_from_slice(previous);
    transcript.extend_from_slice(&current_len.to_be_bytes());
    transcript.extend_from_slice(current);
    debug_assert_eq!(transcript.len(), total);
    Ok(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::{
        PhysicsEvidenceFrameKindV1, decode_physics_evidence_frame_v1,
        encode_physics_evidence_frame_v1,
    };

    const GOLDEN_ENDPOINT_FRAME_HEX: &str = "73796d74726f70792d706879736963732d65766964656e636500656e64706f696e742d73616d706c6500000000010000000000000140000000010003008f73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728000000010000000000000001010203040506070811121314151617180100000000000000003fe0000000000000bfe000000000000040000000000000004008000000000000401000000000000040260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";
    const GOLDEN_DERIVED_POSITION_HEX: &str = "73796d74726f70792d706879736963732d636c61696d2d706f736974696f6e00636f6e73656375746976652d70726573656e63652d76310000000001000000cd73796d74726f70792d706879736963732d636c61696d2d706f736974696f6e00656e64706f696e742d73616d706c652d763100000000010003000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f2021222324252627280000000100000000000000010102030405060708111213141516171800000000000000003fe0000000000000bfe0000000000000400000000000000040080000000000004010000000000000000000cd73796d74726f70792d706879736963732d636c61696d2d706f736974696f6e00656e64706f696e742d73616d706c652d763100000000010003000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f2021222324252627280000000100000000000000020102030405060708111213141516171800000000000000003fe0000000000000bfe0000000000000400000000000000040080000000000004010000000000000";
    const GOLDEN_DERIVED_VALUE_HEX: &str = "73796d74726f70792d706879736963732d636c61696d2d76616c756500636f6e73656375746976652d70726573656e63652d763100000000010000009773796d74726f70792d706879736963732d636c61696d2d76616c756500656e64706f696e742d73616d706c652d7631000000000100030140260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c0040000000000000000009773796d74726f70792d706879736963732d636c61696d2d76616c756500656e64706f696e742d73616d706c652d7631000000000100030140260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";

    fn from_hex(hex: &str) -> Vec<u8> {
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

    fn endpoint_with_step(frame: &[u8], step: u64) -> Vec<u8> {
        let decoded = decode_physics_evidence_frame_v1(frame).unwrap();
        assert_eq!(decoded.kind(), PhysicsEvidenceFrameKindV1::EndpointSample);
        let mut payload = decoded.payload().to_vec();
        payload[151..159].copy_from_slice(&step.to_be_bytes());
        encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, &payload)
            .unwrap()
    }

    fn relation(previous: &[u8], current: &[u8]) -> Vec<u8> {
        let mut payload = Vec::with_capacity(12 + previous.len() + current.len());
        payload.extend_from_slice(&1_u32.to_be_bytes());
        payload.extend_from_slice(&(previous.len() as u32).to_be_bytes());
        payload.extend_from_slice(previous);
        payload.extend_from_slice(&(current.len() as u32).to_be_bytes());
        payload.extend_from_slice(current);
        encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::ConsecutiveSampledPresence,
            &payload,
        )
        .unwrap()
    }

    fn mutate_current_value_same_position(current: &[u8]) -> Vec<u8> {
        let decoded = decode_physics_evidence_frame_v1(current).unwrap();
        let mut payload = decoded.payload().to_vec();
        // target x: 11.0 -> 10.5; region center x remains 10.0;
        // offset x: 1.0 -> 0.5. Membership remains Inside and all proposition
        // fields, session, identities, and step remain unchanged.
        payload[224..232].copy_from_slice(&10.5_f64.to_bits().to_be_bytes());
        payload[296..304].copy_from_slice(&0.5_f64.to_bits().to_be_bytes());
        encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, &payload)
            .unwrap()
    }

    #[test]
    fn golden_relation_derives_exact_ordered_semantic_transcripts() {
        let previous = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let current = endpoint_with_step(&previous, 2);
        let claim = derive_consecutive_presence_claim_v1(&relation(&previous, &current)).unwrap();
        assert_eq!(claim.position().len(), 478);
        assert_eq!(claim.value().len(), 367);
        assert_eq!(to_hex(claim.position().as_bytes()), GOLDEN_DERIVED_POSITION_HEX);
        assert_eq!(to_hex(claim.value().as_bytes()), GOLDEN_DERIVED_VALUE_HEX);

        let fixture = include_str!("../tests/fixtures/consecutive_claim_identity_v1_vector.json");
        assert!(fixture.contains(GOLDEN_DERIVED_POSITION_HEX));
        assert!(fixture.contains(GOLDEN_DERIVED_VALUE_HEX));
    }

    #[test]
    fn repeated_derivation_is_duplicate() {
        let previous = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let current = endpoint_with_step(&previous, 2);
        let frame = relation(&previous, &current);
        assert_eq!(
            classify_consecutive_presence_frames_v1(&frame, &frame).unwrap(),
            ConsecutivePresenceClaimComparisonV1::Duplicate
        );
    }

    #[test]
    fn same_ordered_positions_with_different_admitted_values_are_contradiction() {
        let previous = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let current = endpoint_with_step(&previous, 2);
        let altered_current = mutate_current_value_same_position(&current);
        let first = relation(&previous, &current);
        let second = relation(&previous, &altered_current);

        let first_claim = derive_consecutive_presence_claim_v1(&first).unwrap();
        let second_claim = derive_consecutive_presence_claim_v1(&second).unwrap();
        assert_eq!(first_claim.position(), second_claim.position());
        assert_ne!(first_claim.value(), second_claim.value());
        assert_eq!(
            compare_consecutive_presence_claims_v1(&first_claim, &second_claim),
            ConsecutivePresenceClaimComparisonV1::Contradiction
        );
    }

    #[test]
    fn different_adjacent_relation_position_is_distinct() {
        let previous = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let step2 = endpoint_with_step(&previous, 2);
        let step3 = endpoint_with_step(&previous, 3);
        let first = relation(&previous, &step2);
        let second = relation(&step2, &step3);
        assert_eq!(
            classify_consecutive_presence_frames_v1(&first, &second).unwrap(),
            ConsecutivePresenceClaimComparisonV1::DistinctPosition
        );
    }
}
