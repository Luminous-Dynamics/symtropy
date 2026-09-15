// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical semantic-claim commitment preimages.
//!
//! PHYS-EVID-01D0 freezes only the bytes a future cryptographic digest will
//! commit. It deliberately does not select, invoke, or depend on a hash
//! algorithm and does not create authenticated evidence or action authority.

use crate::consecutive_claim_identity::CanonicalConsecutivePresenceClaimV1;
use crate::endpoint_claim_identity::CanonicalEndpointSampleClaimV1;

/// Exact application namespace for semantic commitment preimages v1.
pub const SEMANTIC_COMMITMENT_PREIMAGE_V1_MAGIC: &[u8] =
    b"symtropy-physics-semantic-commitment\0";
/// Domain for position-only commitments.
pub const SEMANTIC_COMMITMENT_POSITION_V1_DOMAIN: &[u8] = b"position-v1\0";
/// Domain for full position+value claim commitments.
pub const SEMANTIC_COMMITMENT_FULL_CLAIM_V1_DOMAIN: &[u8] = b"full-claim-v1\0";
/// Exact preimage grammar version.
pub const SEMANTIC_COMMITMENT_PREIMAGE_V1_VERSION: u32 = 1;

/// Closed v1 registry of semantic claim profiles admitted by this theorem.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SemanticClaimProfileV1 {
    EndpointSample,
    ConsecutivePresence,
}

impl SemanticClaimProfileV1 {
    pub const fn domain_bytes(self) -> &'static [u8] {
        match self {
            Self::EndpointSample => b"endpoint-sample-v1",
            Self::ConsecutivePresence => b"consecutive-presence-v1",
        }
    }
}

/// Canonical preimage bytes for one already-admitted semantic claim.
///
/// These bytes are representation only. They are not digests, signatures,
/// issuer evidence, replay state, or authorization tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticClaimCommitmentPreimagesV1 {
    profile: SemanticClaimProfileV1,
    position_preimage: Vec<u8>,
    full_claim_preimage: Vec<u8>,
}

impl SemanticClaimCommitmentPreimagesV1 {
    pub const fn profile(&self) -> SemanticClaimProfileV1 {
        self.profile
    }

    pub fn position_preimage(&self) -> &[u8] {
        &self.position_preimage
    }

    pub fn full_claim_preimage(&self) -> &[u8] {
        &self.full_claim_preimage
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SemanticCommitmentPreimageV1Error {
    TranscriptLengthOverflow,
    PreimageLengthOverflow,
}

/// Derive commitment preimages from one typed PHYS-EVID-01A endpoint claim.
pub fn endpoint_sample_commitment_preimages_v1(
    claim: &CanonicalEndpointSampleClaimV1,
) -> Result<SemanticClaimCommitmentPreimagesV1, SemanticCommitmentPreimageV1Error> {
    build_preimages_v1(
        SemanticClaimProfileV1::EndpointSample,
        claim.position().as_bytes(),
        claim.value().as_bytes(),
    )
}

/// Derive commitment preimages from one typed PHYS-EVID-01B consecutive claim.
pub fn consecutive_presence_commitment_preimages_v1(
    claim: &CanonicalConsecutivePresenceClaimV1,
) -> Result<SemanticClaimCommitmentPreimagesV1, SemanticCommitmentPreimageV1Error> {
    build_preimages_v1(
        SemanticClaimProfileV1::ConsecutivePresence,
        claim.position().as_bytes(),
        claim.value().as_bytes(),
    )
}

fn build_preimages_v1(
    profile: SemanticClaimProfileV1,
    position: &[u8],
    value: &[u8],
) -> Result<SemanticClaimCommitmentPreimagesV1, SemanticCommitmentPreimageV1Error> {
    let position_len = u64::try_from(position.len())
        .map_err(|_| SemanticCommitmentPreimageV1Error::TranscriptLengthOverflow)?;
    let value_len = u64::try_from(value.len())
        .map_err(|_| SemanticCommitmentPreimageV1Error::TranscriptLengthOverflow)?;
    let profile_domain = profile.domain_bytes();

    let common_prefix_len = SEMANTIC_COMMITMENT_PREIMAGE_V1_MAGIC
        .len()
        .checked_add(profile_domain.len())
        .and_then(|len| len.checked_add(1 + 4 + 8))
        .ok_or(SemanticCommitmentPreimageV1Error::PreimageLengthOverflow)?;

    let position_capacity = common_prefix_len
        .checked_add(SEMANTIC_COMMITMENT_POSITION_V1_DOMAIN.len())
        .and_then(|len| len.checked_add(position.len()))
        .ok_or(SemanticCommitmentPreimageV1Error::PreimageLengthOverflow)?;
    let full_capacity = common_prefix_len
        .checked_add(SEMANTIC_COMMITMENT_FULL_CLAIM_V1_DOMAIN.len())
        .and_then(|len| len.checked_add(position.len()))
        .and_then(|len| len.checked_add(8))
        .and_then(|len| len.checked_add(value.len()))
        .ok_or(SemanticCommitmentPreimageV1Error::PreimageLengthOverflow)?;

    let mut position_preimage = Vec::with_capacity(position_capacity);
    position_preimage.extend_from_slice(SEMANTIC_COMMITMENT_PREIMAGE_V1_MAGIC);
    position_preimage.extend_from_slice(SEMANTIC_COMMITMENT_POSITION_V1_DOMAIN);
    position_preimage.extend_from_slice(profile_domain);
    position_preimage.push(0);
    position_preimage.extend_from_slice(&SEMANTIC_COMMITMENT_PREIMAGE_V1_VERSION.to_be_bytes());
    position_preimage.extend_from_slice(&position_len.to_be_bytes());
    position_preimage.extend_from_slice(position);
    debug_assert_eq!(position_preimage.len(), position_capacity);

    let mut full_claim_preimage = Vec::with_capacity(full_capacity);
    full_claim_preimage.extend_from_slice(SEMANTIC_COMMITMENT_PREIMAGE_V1_MAGIC);
    full_claim_preimage.extend_from_slice(SEMANTIC_COMMITMENT_FULL_CLAIM_V1_DOMAIN);
    full_claim_preimage.extend_from_slice(profile_domain);
    full_claim_preimage.push(0);
    full_claim_preimage.extend_from_slice(&SEMANTIC_COMMITMENT_PREIMAGE_V1_VERSION.to_be_bytes());
    full_claim_preimage.extend_from_slice(&position_len.to_be_bytes());
    full_claim_preimage.extend_from_slice(position);
    full_claim_preimage.extend_from_slice(&value_len.to_be_bytes());
    full_claim_preimage.extend_from_slice(value);
    debug_assert_eq!(full_claim_preimage.len(), full_capacity);

    Ok(SemanticClaimCommitmentPreimagesV1 {
        profile,
        position_preimage,
        full_claim_preimage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consecutive_claim_identity::derive_consecutive_presence_claim_v1;
    use crate::endpoint_claim_identity::derive_endpoint_sample_claim_v1;
    use symtropy_physics::{
        PhysicsEvidenceFrameKindV1, decode_physics_evidence_frame_v1,
        encode_physics_evidence_frame_v1,
    };

    const GOLDEN_ENDPOINT_FRAME_HEX: &str = "73796d74726f70792d706879736963732d65766964656e636500656e64706f696e742d73616d706c6500000000010000000000000140000000010003008f73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728000000010000000000000001010203040506070811121314151617180100000000000000003fe0000000000000bfe000000000000040000000000000004008000000000000401000000000000040260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";

    fn from_hex(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect()
    }

    fn to_hex(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").unwrap();
        }
        output
    }

    fn endpoint_with_step(frame: &[u8], step: u64) -> Vec<u8> {
        let decoded = decode_physics_evidence_frame_v1(frame).unwrap();
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

    #[test]
    fn golden_typed_claims_match_exact_language_neutral_preimages() {
        let endpoint_frame = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let endpoint_claim = derive_endpoint_sample_claim_v1(&endpoint_frame).unwrap();
        let endpoint = endpoint_sample_commitment_preimages_v1(&endpoint_claim).unwrap();
        assert_eq!(endpoint.profile(), SemanticClaimProfileV1::EndpointSample);
        assert_eq!(endpoint.position_preimage().len(), 285);
        assert_eq!(endpoint.full_claim_preimage().len(), 446);

        let current = endpoint_with_step(&endpoint_frame, 2);
        let consecutive_claim =
            derive_consecutive_presence_claim_v1(&relation(&endpoint_frame, &current)).unwrap();
        let consecutive =
            consecutive_presence_commitment_preimages_v1(&consecutive_claim).unwrap();
        assert_eq!(
            consecutive.profile(),
            SemanticClaimProfileV1::ConsecutivePresence
        );
        assert_eq!(consecutive.position_preimage().len(), 563);
        assert_eq!(consecutive.full_claim_preimage().len(), 940);

        let fixture = include_str!("../tests/fixtures/semantic_commitment_preimage_v1_vector.json");
        for (key, bytes) in [
            ("endpoint_position_preimage_hex", endpoint.position_preimage()),
            ("endpoint_full_claim_preimage_hex", endpoint.full_claim_preimage()),
            (
                "consecutive_position_preimage_hex",
                consecutive.position_preimage(),
            ),
            (
                "consecutive_full_claim_preimage_hex",
                consecutive.full_claim_preimage(),
            ),
        ] {
            let exact_entry = format!("\"{key}\": \"{}\"", to_hex(bytes));
            assert!(fixture.contains(&exact_entry), "missing exact fixture entry for {key}");
        }
    }

    #[test]
    fn same_position_different_value_changes_only_full_claim_preimage() {
        let endpoint_frame = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let first = derive_endpoint_sample_claim_v1(&endpoint_frame).unwrap();

        let decoded = decode_physics_evidence_frame_v1(&endpoint_frame).unwrap();
        let mut payload = decoded.payload().to_vec();
        payload[224..232].copy_from_slice(&10.5_f64.to_bits().to_be_bytes());
        payload[296..304].copy_from_slice(&0.5_f64.to_bits().to_be_bytes());
        let altered_frame = encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::EndpointSample,
            &payload,
        )
        .unwrap();
        let second = derive_endpoint_sample_claim_v1(&altered_frame).unwrap();
        assert_eq!(first.position(), second.position());
        assert_ne!(first.value(), second.value());

        let first_preimages = endpoint_sample_commitment_preimages_v1(&first).unwrap();
        let second_preimages = endpoint_sample_commitment_preimages_v1(&second).unwrap();
        assert_eq!(
            first_preimages.position_preimage(),
            second_preimages.position_preimage()
        );
        assert_ne!(
            first_preimages.full_claim_preimage(),
            second_preimages.full_claim_preimage()
        );
    }

    #[test]
    fn profile_domain_separates_identical_private_transcripts() {
        let position = b"same-position";
        let value = b"same-value";
        let endpoint = build_preimages_v1(SemanticClaimProfileV1::EndpointSample, position, value)
            .unwrap();
        let consecutive = build_preimages_v1(
            SemanticClaimProfileV1::ConsecutivePresence,
            position,
            value,
        )
        .unwrap();
        assert_ne!(endpoint.position_preimage(), consecutive.position_preimage());
        assert_ne!(endpoint.full_claim_preimage(), consecutive.full_claim_preimage());
    }

    #[test]
    fn position_and_full_claim_domains_are_distinct() {
        let endpoint_frame = from_hex(GOLDEN_ENDPOINT_FRAME_HEX);
        let claim = derive_endpoint_sample_claim_v1(&endpoint_frame).unwrap();
        let preimages = endpoint_sample_commitment_preimages_v1(&claim).unwrap();
        assert_ne!(preimages.position_preimage(), preimages.full_claim_preimage());
        assert!(
            preimages
                .position_preimage()
                .starts_with(SEMANTIC_COMMITMENT_PREIMAGE_V1_MAGIC)
        );
        assert!(
            preimages
                .full_claim_preimage()
                .starts_with(SEMANTIC_COMMITMENT_PREIMAGE_V1_MAGIC)
        );
    }
}
