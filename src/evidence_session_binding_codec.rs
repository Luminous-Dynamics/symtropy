// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical semantic codec for PHYS-EVID-05 evidence-session bindings.
//!
//! Encoding is authority-preserving downward projection: only a qualified
//! session binding may enter the public encoder. Decoding returns an untrusted
//! plain record and never recreates a qualified binding or live session.

use crate::evidence_session::{
    EVIDENCE_SESSION_ID_LEN, EvidenceSessionProfileV1, QualifiedEvidenceSessionBinding,
};
use symtropy_physics::{
    PhysicsEvidenceFrameKindV1, PhysicsEvidenceFrameV1Error, decode_physics_evidence_frame_v1,
    encode_physics_evidence_frame_v1,
};

pub const EVIDENCE_SESSION_BINDING_PAYLOAD_V1_VERSION: u32 = 1;
pub const EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN: usize = 80;

const VERSION_START: usize = 0;
const SESSION_ID_START: usize = 4;
const AUTHORITY_START: usize = 36;
const GENERATION_START: usize = 52;
const INCARNATION_START: usize = 68;
const PROFILE_START: usize = 76;

/// Untrusted canonical-decoded session-binding semantics.
///
/// This record is deliberately distinct from `QualifiedEvidenceSessionBinding`.
/// Canonical syntax/field validity does not prove local bootstrap provenance or
/// external authentication.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct DecodedEvidenceSessionBindingV1 {
    session_id_bytes: [u8; EVIDENCE_SESSION_ID_LEN],
    physical_authority_raw: u128,
    world_generation_raw: u128,
    temporal_incarnation_raw: u64,
    session_profile_code: u32,
}

impl DecodedEvidenceSessionBindingV1 {
    pub const fn session_id_bytes(&self) -> &[u8; EVIDENCE_SESSION_ID_LEN] {
        &self.session_id_bytes
    }

    pub const fn physical_authority_raw(&self) -> u128 {
        self.physical_authority_raw
    }

    pub const fn world_generation_raw(&self) -> u128 {
        self.world_generation_raw
    }

    pub const fn temporal_incarnation_raw(&self) -> u64 {
        self.temporal_incarnation_raw
    }

    pub const fn session_profile_code(&self) -> u32 {
        self.session_profile_code
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceSessionBindingCodecV1Error {
    Frame(PhysicsEvidenceFrameV1Error),
    WrongFrameKind { actual: PhysicsEvidenceFrameKindV1 },
    WrongPayloadLength { actual: usize },
    UnsupportedPayloadVersion { version: u32 },
    ZeroSessionId,
    ZeroPhysicalAuthorityId,
    ZeroWorldGenerationId,
    ZeroTemporalIncarnationId,
    UnknownSessionProfile { code: u32 },
}

/// Canonically encode one already-qualified PHYS-EVID-05 session binding.
///
/// This is the only public semantic encoding path. Detached caller-supplied raw
/// fields cannot enter the authority-bearing encoder.
pub fn encode_qualified_evidence_session_binding_v1(
    binding: &QualifiedEvidenceSessionBinding,
) -> Result<Vec<u8>, PhysicsEvidenceFrameV1Error> {
    let payload = encode_binding_payload_fields_v1(
        binding.session_id().to_bytes(),
        binding.physical_authority_id().get(),
        binding.world_generation_id().get(),
        binding.temporal_incarnation_id().get(),
        binding.profile().code(),
    );
    encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EvidenceSessionBinding, &payload)
}

/// Decode canonical session-binding semantics into an explicitly untrusted
/// plain record. This operation does not reconstruct PHYS-EVID-05 authority.
pub fn decode_evidence_session_binding_v1(
    frame: &[u8],
) -> Result<DecodedEvidenceSessionBindingV1, EvidenceSessionBindingCodecV1Error> {
    let decoded = decode_physics_evidence_frame_v1(frame)
        .map_err(EvidenceSessionBindingCodecV1Error::Frame)?;
    if decoded.kind() != PhysicsEvidenceFrameKindV1::EvidenceSessionBinding {
        return Err(EvidenceSessionBindingCodecV1Error::WrongFrameKind {
            actual: decoded.kind(),
        });
    }

    decode_binding_payload_v1(decoded.payload())
}

fn encode_binding_payload_fields_v1(
    session_id_bytes: [u8; EVIDENCE_SESSION_ID_LEN],
    physical_authority_raw: u128,
    world_generation_raw: u128,
    temporal_incarnation_raw: u64,
    session_profile_code: u32,
) -> [u8; EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN] {
    let mut payload = [0_u8; EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN];
    payload[VERSION_START..SESSION_ID_START]
        .copy_from_slice(&EVIDENCE_SESSION_BINDING_PAYLOAD_V1_VERSION.to_be_bytes());
    payload[SESSION_ID_START..AUTHORITY_START].copy_from_slice(&session_id_bytes);
    payload[AUTHORITY_START..GENERATION_START]
        .copy_from_slice(&physical_authority_raw.to_be_bytes());
    payload[GENERATION_START..INCARNATION_START]
        .copy_from_slice(&world_generation_raw.to_be_bytes());
    payload[INCARNATION_START..PROFILE_START]
        .copy_from_slice(&temporal_incarnation_raw.to_be_bytes());
    payload[PROFILE_START..EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN]
        .copy_from_slice(&session_profile_code.to_be_bytes());
    payload
}

fn decode_binding_payload_v1(
    payload: &[u8],
) -> Result<DecodedEvidenceSessionBindingV1, EvidenceSessionBindingCodecV1Error> {
    if payload.len() != EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN {
        return Err(EvidenceSessionBindingCodecV1Error::WrongPayloadLength {
            actual: payload.len(),
        });
    }

    let mut version_bytes = [0_u8; 4];
    version_bytes.copy_from_slice(&payload[VERSION_START..SESSION_ID_START]);
    let version = u32::from_be_bytes(version_bytes);
    if version != EVIDENCE_SESSION_BINDING_PAYLOAD_V1_VERSION {
        return Err(EvidenceSessionBindingCodecV1Error::UnsupportedPayloadVersion {
            version,
        });
    }

    let mut session_id_bytes = [0_u8; EVIDENCE_SESSION_ID_LEN];
    session_id_bytes.copy_from_slice(&payload[SESSION_ID_START..AUTHORITY_START]);
    if session_id_bytes.iter().all(|byte| *byte == 0) {
        return Err(EvidenceSessionBindingCodecV1Error::ZeroSessionId);
    }

    let mut authority_bytes = [0_u8; 16];
    authority_bytes.copy_from_slice(&payload[AUTHORITY_START..GENERATION_START]);
    let physical_authority_raw = u128::from_be_bytes(authority_bytes);
    if physical_authority_raw == 0 {
        return Err(EvidenceSessionBindingCodecV1Error::ZeroPhysicalAuthorityId);
    }

    let mut generation_bytes = [0_u8; 16];
    generation_bytes.copy_from_slice(&payload[GENERATION_START..INCARNATION_START]);
    let world_generation_raw = u128::from_be_bytes(generation_bytes);
    if world_generation_raw == 0 {
        return Err(EvidenceSessionBindingCodecV1Error::ZeroWorldGenerationId);
    }

    let mut incarnation_bytes = [0_u8; 8];
    incarnation_bytes.copy_from_slice(&payload[INCARNATION_START..PROFILE_START]);
    let temporal_incarnation_raw = u64::from_be_bytes(incarnation_bytes);
    if temporal_incarnation_raw == 0 {
        return Err(EvidenceSessionBindingCodecV1Error::ZeroTemporalIncarnationId);
    }

    let mut profile_bytes = [0_u8; 4];
    profile_bytes.copy_from_slice(&payload[PROFILE_START..EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN]);
    let session_profile_code = u32::from_be_bytes(profile_bytes);
    if session_profile_code != EvidenceSessionProfileV1::OsCsprngBoundLiveIncarnation.code() {
        return Err(EvidenceSessionBindingCodecV1Error::UnknownSessionProfile {
            code: session_profile_code,
        });
    }

    Ok(DecodedEvidenceSessionBindingV1 {
        session_id_bytes,
        physical_authority_raw,
        world_generation_raw,
        temporal_incarnation_raw,
        session_profile_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_session::PersistentEvidenceSession;
    use symtropy_physics::{
        LocalEvidencePhysicsAuthorityWorld, LocalNamespacePhysicsAuthorityWorld,
        LocalQualifiedPhysicalAuthority, PhysicsWorld,
    };

    const GOLDEN_PAYLOAD_HEX: &str = "00000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272800000001";
    const GOLDEN_FRAME_HEX: &str = "73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272800000001";

    fn to_hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut out, "{byte:02x}").unwrap();
        }
        out
    }

    fn golden_payload() -> [u8; EVIDENCE_SESSION_BINDING_PAYLOAD_V1_LEN] {
        let mut sid = [0_u8; EVIDENCE_SESSION_ID_LEN];
        for (index, byte) in sid.iter_mut().enumerate() {
            *byte = index as u8;
        }
        encode_binding_payload_fields_v1(
            sid,
            0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10,
            0x1112_1314_1516_1718_191a_1b1c_1d1e_1f20,
            0x2122_2324_2526_2728,
            1,
        )
    }

    fn bootstrap_session() -> PersistentEvidenceSession<3> {
        let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
        let generation = root.mint_generation().unwrap();
        let namespace =
            LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<3>::default());
        let evidence = match LocalEvidencePhysicsAuthorityWorld::seal(namespace) {
            Ok(evidence) => evidence,
            Err(failure) => panic!("seal failed: {:?}", failure.error()),
        };
        match PersistentEvidenceSession::bootstrap(evidence) {
            Ok(session) => session,
            Err(failure) => panic!("bootstrap failed: {:?}", failure.error()),
        }
    }

    #[test]
    fn private_field_codec_matches_language_neutral_golden_vector() {
        let payload = golden_payload();
        assert_eq!(payload.len(), 80);
        assert_eq!(to_hex(&payload), GOLDEN_PAYLOAD_HEX);
        let frame = encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::EvidenceSessionBinding,
            &payload,
        )
        .unwrap();
        assert_eq!(to_hex(&frame), GOLDEN_FRAME_HEX);

        let decoded = decode_evidence_session_binding_v1(&frame).unwrap();
        assert_eq!(decoded.session_id_bytes(), &payload[4..36]);
        let rebuilt = encode_binding_payload_fields_v1(
            *decoded.session_id_bytes(),
            decoded.physical_authority_raw(),
            decoded.world_generation_raw(),
            decoded.temporal_incarnation_raw(),
            decoded.session_profile_code(),
        );
        assert_eq!(rebuilt, payload);
    }

    #[test]
    fn checked_in_vector_corpus_contains_exact_rust_golden_bytes() {
        let fixture = include_str!("../tests/fixtures/evidence_session_binding_v1_vector.json");
        assert!(fixture.contains(GOLDEN_PAYLOAD_HEX));
        assert!(fixture.contains(GOLDEN_FRAME_HEX));
    }

    #[test]
    fn production_qualified_binding_encoder_matches_live_binding_exactly() {
        let session = bootstrap_session();
        let binding = session.binding();
        let frame = encode_qualified_evidence_session_binding_v1(binding).unwrap();
        let decoded = decode_evidence_session_binding_v1(&frame).unwrap();

        assert_eq!(decoded.session_id_bytes(), binding.session_id().as_bytes());
        assert_eq!(decoded.physical_authority_raw(), binding.physical_authority_id().get());
        assert_eq!(decoded.world_generation_raw(), binding.world_generation_id().get());
        assert_eq!(decoded.temporal_incarnation_raw(), binding.temporal_incarnation_id().get());
        assert_eq!(decoded.session_profile_code(), binding.profile().code());
    }

    #[test]
    fn distinct_session_ids_change_canonical_bytes_under_equal_other_fields() {
        let first = encode_binding_payload_fields_v1([0x11; EVIDENCE_SESSION_ID_LEN], 1, 1, 1, 1);
        let second = encode_binding_payload_fields_v1([0x22; EVIDENCE_SESSION_ID_LEN], 1, 1, 1, 1);
        assert_ne!(first, second);
    }

    #[test]
    fn wrong_frame_kind_rejects_before_binding_semantics() {
        let payload = golden_payload();
        let frame = encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, &payload)
            .unwrap();
        assert_eq!(
            decode_evidence_session_binding_v1(&frame),
            Err(EvidenceSessionBindingCodecV1Error::WrongFrameKind {
                actual: PhysicsEvidenceFrameKindV1::EndpointSample,
            })
        );
    }

    fn frame_payload(payload: &[u8]) -> Vec<u8> {
        encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EvidenceSessionBinding, payload)
            .unwrap()
    }

    #[test]
    fn malformed_binding_payloads_fail_closed() {
        let short = frame_payload(&[0_u8; 79]);
        assert_eq!(
            decode_evidence_session_binding_v1(&short),
            Err(EvidenceSessionBindingCodecV1Error::WrongPayloadLength { actual: 79 })
        );
        let long = frame_payload(&[0_u8; 81]);
        assert_eq!(
            decode_evidence_session_binding_v1(&long),
            Err(EvidenceSessionBindingCodecV1Error::WrongPayloadLength { actual: 81 })
        );

        let mut bad_version = golden_payload();
        bad_version[0..4].copy_from_slice(&2_u32.to_be_bytes());
        assert_eq!(
            decode_evidence_session_binding_v1(&frame_payload(&bad_version)),
            Err(EvidenceSessionBindingCodecV1Error::UnsupportedPayloadVersion { version: 2 })
        );

        let mut zero_session = golden_payload();
        zero_session[4..36].fill(0);
        assert_eq!(
            decode_evidence_session_binding_v1(&frame_payload(&zero_session)),
            Err(EvidenceSessionBindingCodecV1Error::ZeroSessionId)
        );

        let mut zero_authority = golden_payload();
        zero_authority[36..52].fill(0);
        assert_eq!(
            decode_evidence_session_binding_v1(&frame_payload(&zero_authority)),
            Err(EvidenceSessionBindingCodecV1Error::ZeroPhysicalAuthorityId)
        );

        let mut zero_generation = golden_payload();
        zero_generation[52..68].fill(0);
        assert_eq!(
            decode_evidence_session_binding_v1(&frame_payload(&zero_generation)),
            Err(EvidenceSessionBindingCodecV1Error::ZeroWorldGenerationId)
        );

        let mut zero_incarnation = golden_payload();
        zero_incarnation[68..76].fill(0);
        assert_eq!(
            decode_evidence_session_binding_v1(&frame_payload(&zero_incarnation)),
            Err(EvidenceSessionBindingCodecV1Error::ZeroTemporalIncarnationId)
        );

        let mut unknown_profile = golden_payload();
        unknown_profile[76..80].copy_from_slice(&2_u32.to_be_bytes());
        assert_eq!(
            decode_evidence_session_binding_v1(&frame_payload(&unknown_profile)),
            Err(EvidenceSessionBindingCodecV1Error::UnknownSessionProfile { code: 2 })
        );
    }
}
