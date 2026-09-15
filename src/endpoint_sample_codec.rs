// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical persistent endpoint-membership samples (PHYS-EVID-02C).
//!
//! The public encoder captures PHYS-OBS-05 evidence through an already-live
//! persistent session, preventing retrospective attachment of a detached token
//! to a session established later. Decoding returns untrusted canonical
//! semantics and independently rechecks geometry/result consistency.

use crate::evidence_session::{PersistentEvidenceSession, QualifiedEvidenceSessionBinding};
use crate::evidence_session_binding_codec::{
    DecodedEvidenceSessionBindingV1, EvidenceSessionBindingCodecV1Error,
    decode_evidence_session_binding_v1, encode_qualified_evidence_session_binding_v1,
};
use symtropy_physics::{
    AuthorityEndpointBoxSpec, EndpointMembership, LocalQualifiedStampedEndpointBoxObservation,
    LocalQualifiedStampedEndpointObservationError, PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN,
    PhysicsBodySubject, PhysicsEvidenceFrameKindV1, PhysicsEvidenceFrameV1Error,
    decode_physics_evidence_frame_v1, encode_physics_evidence_frame_v1,
};

pub const ENDPOINT_SAMPLE_PAYLOAD_V1_VERSION: u32 = 1;
pub const ENDPOINT_SAMPLE_SESSION_BINDING_FRAME_V1_LEN: usize = 143;
pub const ENDPOINT_SAMPLE_FIXED_PREFIX_V1_LEN: usize = 176;
pub const ENDPOINT_SAMPLE_PER_AXIS_V1_LEN: usize = 48;

const VERSION_START: usize = 0;
const DIMENSION_START: usize = 4;
const SESSION_LEN_START: usize = 6;
const SESSION_FRAME_START: usize = 8;
const STEP_INDEX_START: usize = 151;
const TARGET_NET_ID_START: usize = 159;
const ANCHOR_NET_ID_START: usize = 167;
const MEMBERSHIP_START: usize = 175;
const ARRAYS_START: usize = 176;

const CENTER_OFFSET_GROUP: usize = 0;
const HALF_EXTENTS_GROUP: usize = 1;
const TARGET_TRANSLATION_GROUP: usize = 2;
const ANCHOR_TRANSLATION_GROUP: usize = 3;
const REGION_CENTER_GROUP: usize = 4;
const OFFSET_FROM_CENTER_GROUP: usize = 5;
const ARRAY_GROUPS: usize = 6;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum CanonicalEndpointMembershipV1 {
    Outside,
    Inside,
}

impl CanonicalEndpointMembershipV1 {
    const fn code(self) -> u8 {
        match self {
            Self::Outside => 0,
            Self::Inside => 1,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Outside),
            1 => Some(Self::Inside),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndpointSampleCodecV1Error {
    Frame(PhysicsEvidenceFrameV1Error),
    SessionBinding(EvidenceSessionBindingCodecV1Error),
    EndpointCapture(LocalQualifiedStampedEndpointObservationError),
    WrongFrameKind { actual: PhysicsEvidenceFrameKindV1 },
    PayloadTooShort { actual: usize },
    WrongPayloadLength { actual: usize, expected: usize },
    UnsupportedPayloadVersion { version: u32 },
    DimensionZero,
    DimensionOutOfRange { dimension: usize },
    SessionBindingFrameLength { actual: usize },
    QualifiedSessionMismatch,
    QualifiedObservationInvariant,
    StepIndexZero,
    SameTargetAnchorNetId { net_id: u64 },
    InvalidMembership { code: u8 },
    NonFiniteCenterOffset { axis: usize },
    NonCanonicalCenterOffsetZero { axis: usize },
    NonFiniteHalfExtent { axis: usize },
    NonPositiveHalfExtent { axis: usize },
    NonFiniteTargetTranslation { axis: usize },
    NonFiniteAnchorTranslation { axis: usize },
    NonFiniteRegionCenter { axis: usize },
    NonFiniteOffsetFromCenter { axis: usize },
    RegionCenterMismatch { axis: usize },
    OffsetFromCenterMismatch { axis: usize },
    MembershipMismatch {
        encoded: CanonicalEndpointMembershipV1,
        recomputed: CanonicalEndpointMembershipV1,
    },
}

/// Untrusted, canonically decoded endpoint-membership semantics.
///
/// The backing payload remains borrowed. Per-axis accessors return exact stored
/// IEEE-754 bit patterns without allocating arrays based on encoded dimension.
#[derive(Debug, PartialEq, Eq)]
pub struct DecodedEndpointSampleV1<'a> {
    session_binding: DecodedEvidenceSessionBindingV1,
    dimension: u16,
    step_index: u64,
    target_net_id: u64,
    anchor_net_id: u64,
    membership: CanonicalEndpointMembershipV1,
    payload: &'a [u8],
}

impl<'a> DecodedEndpointSampleV1<'a> {
    pub const fn session_binding(&self) -> &DecodedEvidenceSessionBindingV1 {
        &self.session_binding
    }

    pub const fn dimension(&self) -> usize {
        self.dimension as usize
    }

    pub const fn step_index(&self) -> u64 {
        self.step_index
    }

    pub const fn target_net_id(&self) -> u64 {
        self.target_net_id
    }

    pub const fn anchor_net_id(&self) -> u64 {
        self.anchor_net_id
    }

    pub const fn membership(&self) -> CanonicalEndpointMembershipV1 {
        self.membership
    }

    pub fn session_binding_frame(&self) -> &'a [u8] {
        &self.payload[SESSION_FRAME_START..STEP_INDEX_START]
    }

    pub fn center_offset_bits(&self, axis: usize) -> Option<u64> {
        self.array_bits(CENTER_OFFSET_GROUP, axis)
    }

    pub fn half_extent_bits(&self, axis: usize) -> Option<u64> {
        self.array_bits(HALF_EXTENTS_GROUP, axis)
    }

    pub fn target_translation_bits(&self, axis: usize) -> Option<u64> {
        self.array_bits(TARGET_TRANSLATION_GROUP, axis)
    }

    pub fn anchor_translation_bits(&self, axis: usize) -> Option<u64> {
        self.array_bits(ANCHOR_TRANSLATION_GROUP, axis)
    }

    pub fn region_center_bits(&self, axis: usize) -> Option<u64> {
        self.array_bits(REGION_CENTER_GROUP, axis)
    }

    pub fn offset_from_center_bits(&self, axis: usize) -> Option<u64> {
        self.array_bits(OFFSET_FROM_CENTER_GROUP, axis)
    }

    fn array_bits(&self, group: usize, axis: usize) -> Option<u64> {
        if group >= ARRAY_GROUPS || axis >= self.dimension() {
            return None;
        }
        let dimension = self.dimension();
        let start = ARRAYS_START + (group * dimension + axis) * 8;
        Some(read_u64(&self.payload[start..start + 8]))
    }
}

/// Capture and canonically encode the current endpoint sample from an already
/// established persistence session.
///
/// This is the only public persistent endpoint producer. It does not accept a
/// detached endpoint token or detached session binding, so an observation made
/// before session bootstrap cannot be retrospectively attributed to that
/// persistence session through this API.
pub fn capture_and_encode_current_endpoint_sample_v1<const D: usize>(
    session: &PersistentEvidenceSession<D>,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<D>,
) -> Result<Vec<u8>, EndpointSampleCodecV1Error> {
    let token = LocalQualifiedStampedEndpointBoxObservation::capture(
        session.evidence_authority(),
        subject,
        region,
    )
    .map_err(EndpointSampleCodecV1Error::EndpointCapture)?;
    encode_qualified_endpoint_token_v1(session.binding(), &token)
}

/// Private composition boundary used only after the endpoint token has been
/// captured through an already-established `PersistentEvidenceSession`.
fn encode_qualified_endpoint_token_v1<const D: usize>(
    binding: &QualifiedEvidenceSessionBinding,
    token: &LocalQualifiedStampedEndpointBoxObservation<D>,
) -> Result<Vec<u8>, EndpointSampleCodecV1Error> {
    let expected_len = endpoint_payload_len_v1(D)?;
    let dimension = u16::try_from(D)
        .map_err(|_| EndpointSampleCodecV1Error::DimensionOutOfRange { dimension: D })?;

    let stamp = token.stamp();
    if binding.physical_authority_id() != stamp.physical_authority_id()
        || binding.world_generation_id() != stamp.world_generation_id()
        || binding.temporal_incarnation_id() != stamp.temporal_incarnation_id()
    {
        return Err(EndpointSampleCodecV1Error::QualifiedSessionMismatch);
    }
    if stamp.step_index() == 0 {
        return Err(EndpointSampleCodecV1Error::QualifiedObservationInvariant);
    }

    let observation = token.observation();
    let target = observation.subject.subject;
    let anchor = observation.anchor.subject;
    if target.physical_authority_id() != stamp.physical_authority_id()
        || target.world_generation_id() != stamp.world_generation_id()
        || anchor.physical_authority_id() != stamp.physical_authority_id()
        || anchor.world_generation_id() != stamp.world_generation_id()
        || observation.region.anchor() != anchor
        || target == anchor
    {
        return Err(EndpointSampleCodecV1Error::QualifiedObservationInvariant);
    }

    let session_frame = encode_qualified_evidence_session_binding_v1(binding)
        .map_err(EndpointSampleCodecV1Error::Frame)?;
    if session_frame.len() != ENDPOINT_SAMPLE_SESSION_BINDING_FRAME_V1_LEN {
        return Err(EndpointSampleCodecV1Error::SessionBindingFrameLength {
            actual: session_frame.len(),
        });
    }

    let center_offset = observation.region.center_offset();
    let half_extents = observation.region.half_extents();
    let center_offset_bits: [u64; D] =
        std::array::from_fn(|axis| center_offset[axis].to_bits());
    let half_extent_bits: [u64; D] =
        std::array::from_fn(|axis| half_extents[axis].to_bits());

    let membership = match observation.membership {
        EndpointMembership::Outside => CanonicalEndpointMembershipV1::Outside,
        EndpointMembership::Inside => CanonicalEndpointMembershipV1::Inside,
    };

    let arrays: [&[u64]; ARRAY_GROUPS] = [
        &center_offset_bits,
        &half_extent_bits,
        &observation.subject.translation,
        &observation.anchor.translation,
        &observation.region_center,
        &observation.offset_from_center,
    ];

    let payload = encode_endpoint_payload_fields_v1(
        dimension,
        &session_frame,
        stamp.step_index(),
        target.net_id().0,
        anchor.net_id().0,
        membership,
        arrays,
        expected_len,
    );

    let frame = encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, &payload)
        .map_err(EndpointSampleCodecV1Error::Frame)?;

    // Use one canonical admission predicate for both locally produced and
    // externally supplied persistent bytes. Future producer drift fails closed.
    decode_endpoint_sample_v1(&frame)?;
    Ok(frame)
}

pub fn decode_endpoint_sample_v1(
    frame: &[u8],
) -> Result<DecodedEndpointSampleV1<'_>, EndpointSampleCodecV1Error> {
    let decoded = decode_physics_evidence_frame_v1(frame).map_err(EndpointSampleCodecV1Error::Frame)?;
    if decoded.kind() != PhysicsEvidenceFrameKindV1::EndpointSample {
        return Err(EndpointSampleCodecV1Error::WrongFrameKind {
            actual: decoded.kind(),
        });
    }

    let payload = decoded.payload();
    if payload.len() < ENDPOINT_SAMPLE_FIXED_PREFIX_V1_LEN {
        return Err(EndpointSampleCodecV1Error::PayloadTooShort {
            actual: payload.len(),
        });
    }

    let version = read_u32(&payload[VERSION_START..DIMENSION_START]);
    if version != ENDPOINT_SAMPLE_PAYLOAD_V1_VERSION {
        return Err(EndpointSampleCodecV1Error::UnsupportedPayloadVersion { version });
    }

    let dimension = read_u16(&payload[DIMENSION_START..SESSION_LEN_START]);
    if dimension == 0 {
        return Err(EndpointSampleCodecV1Error::DimensionZero);
    }
    let expected_len = endpoint_payload_len_v1(dimension as usize)?;
    if payload.len() != expected_len {
        return Err(EndpointSampleCodecV1Error::WrongPayloadLength {
            actual: payload.len(),
            expected: expected_len,
        });
    }

    let session_len = read_u16(&payload[SESSION_LEN_START..SESSION_FRAME_START]) as usize;
    if session_len != ENDPOINT_SAMPLE_SESSION_BINDING_FRAME_V1_LEN {
        return Err(EndpointSampleCodecV1Error::SessionBindingFrameLength {
            actual: session_len,
        });
    }
    let session_frame = &payload[SESSION_FRAME_START..STEP_INDEX_START];
    let session_binding = decode_evidence_session_binding_v1(session_frame)
        .map_err(EndpointSampleCodecV1Error::SessionBinding)?;

    let step_index = read_u64(&payload[STEP_INDEX_START..TARGET_NET_ID_START]);
    if step_index == 0 {
        return Err(EndpointSampleCodecV1Error::StepIndexZero);
    }
    let target_net_id = read_u64(&payload[TARGET_NET_ID_START..ANCHOR_NET_ID_START]);
    let anchor_net_id = read_u64(&payload[ANCHOR_NET_ID_START..MEMBERSHIP_START]);
    if target_net_id == anchor_net_id {
        return Err(EndpointSampleCodecV1Error::SameTargetAnchorNetId {
            net_id: target_net_id,
        });
    }

    let membership_code = payload[MEMBERSHIP_START];
    let membership = CanonicalEndpointMembershipV1::from_code(membership_code)
        .ok_or(EndpointSampleCodecV1Error::InvalidMembership {
            code: membership_code,
        })?;

    let dimension_usize = dimension as usize;
    for axis in 0..dimension_usize {
        let center_offset_bits =
            payload_array_bits(payload, dimension_usize, CENTER_OFFSET_GROUP, axis);
        let half_extent_bits =
            payload_array_bits(payload, dimension_usize, HALF_EXTENTS_GROUP, axis);
        let target_translation_bits =
            payload_array_bits(payload, dimension_usize, TARGET_TRANSLATION_GROUP, axis);
        let anchor_translation_bits =
            payload_array_bits(payload, dimension_usize, ANCHOR_TRANSLATION_GROUP, axis);
        let region_center_bits =
            payload_array_bits(payload, dimension_usize, REGION_CENTER_GROUP, axis);
        let offset_bits =
            payload_array_bits(payload, dimension_usize, OFFSET_FROM_CENTER_GROUP, axis);

        let center_offset = f64::from_bits(center_offset_bits);
        if !center_offset.is_finite() {
            return Err(EndpointSampleCodecV1Error::NonFiniteCenterOffset { axis });
        }
        if center_offset == 0.0 && center_offset_bits != 0 {
            return Err(EndpointSampleCodecV1Error::NonCanonicalCenterOffsetZero { axis });
        }

        let half_extent = f64::from_bits(half_extent_bits);
        if !half_extent.is_finite() {
            return Err(EndpointSampleCodecV1Error::NonFiniteHalfExtent { axis });
        }
        if half_extent <= 0.0 {
            return Err(EndpointSampleCodecV1Error::NonPositiveHalfExtent { axis });
        }

        let target_translation = f64::from_bits(target_translation_bits);
        if !target_translation.is_finite() {
            return Err(EndpointSampleCodecV1Error::NonFiniteTargetTranslation { axis });
        }
        let anchor_translation = f64::from_bits(anchor_translation_bits);
        if !anchor_translation.is_finite() {
            return Err(EndpointSampleCodecV1Error::NonFiniteAnchorTranslation { axis });
        }
        let region_center = f64::from_bits(region_center_bits);
        if !region_center.is_finite() {
            return Err(EndpointSampleCodecV1Error::NonFiniteRegionCenter { axis });
        }
        let offset = f64::from_bits(offset_bits);
        if !offset.is_finite() {
            return Err(EndpointSampleCodecV1Error::NonFiniteOffsetFromCenter { axis });
        }

        let recomputed_center = anchor_translation + center_offset;
        if !recomputed_center.is_finite() || recomputed_center.to_bits() != region_center_bits {
            return Err(EndpointSampleCodecV1Error::RegionCenterMismatch { axis });
        }
        let recomputed_offset = target_translation - region_center;
        if !recomputed_offset.is_finite() || recomputed_offset.to_bits() != offset_bits {
            return Err(EndpointSampleCodecV1Error::OffsetFromCenterMismatch { axis });
        }
    }

    let recomputed_membership = if (0..dimension_usize).all(|axis| {
        let offset = f64::from_bits(payload_array_bits(
            payload,
            dimension_usize,
            OFFSET_FROM_CENTER_GROUP,
            axis,
        ));
        let half_extent = f64::from_bits(payload_array_bits(
            payload,
            dimension_usize,
            HALF_EXTENTS_GROUP,
            axis,
        ));
        offset.abs() <= half_extent
    }) {
        CanonicalEndpointMembershipV1::Inside
    } else {
        CanonicalEndpointMembershipV1::Outside
    };
    if membership != recomputed_membership {
        return Err(EndpointSampleCodecV1Error::MembershipMismatch {
            encoded: membership,
            recomputed: recomputed_membership,
        });
    }

    Ok(DecodedEndpointSampleV1 {
        session_binding,
        dimension,
        step_index,
        target_net_id,
        anchor_net_id,
        membership,
        payload,
    })
}

fn endpoint_payload_len_v1(dimension: usize) -> Result<usize, EndpointSampleCodecV1Error> {
    if dimension == 0 {
        return Err(EndpointSampleCodecV1Error::DimensionZero);
    }
    if dimension > u16::MAX as usize {
        return Err(EndpointSampleCodecV1Error::DimensionOutOfRange { dimension });
    }
    let variable = ENDPOINT_SAMPLE_PER_AXIS_V1_LEN
        .checked_mul(dimension)
        .ok_or(EndpointSampleCodecV1Error::DimensionOutOfRange { dimension })?;
    let total = ENDPOINT_SAMPLE_FIXED_PREFIX_V1_LEN
        .checked_add(variable)
        .ok_or(EndpointSampleCodecV1Error::DimensionOutOfRange { dimension })?;
    if total > PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN {
        return Err(EndpointSampleCodecV1Error::DimensionOutOfRange { dimension });
    }
    Ok(total)
}

fn encode_endpoint_payload_fields_v1(
    dimension: u16,
    session_frame: &[u8],
    step_index: u64,
    target_net_id: u64,
    anchor_net_id: u64,
    membership: CanonicalEndpointMembershipV1,
    arrays: [&[u64]; ARRAY_GROUPS],
    expected_len: usize,
) -> Vec<u8> {
    debug_assert_eq!(session_frame.len(), ENDPOINT_SAMPLE_SESSION_BINDING_FRAME_V1_LEN);
    debug_assert!(arrays.iter().all(|values| values.len() == dimension as usize));

    let mut payload = Vec::with_capacity(expected_len);
    payload.extend_from_slice(&ENDPOINT_SAMPLE_PAYLOAD_V1_VERSION.to_be_bytes());
    payload.extend_from_slice(&dimension.to_be_bytes());
    payload.extend_from_slice(&(ENDPOINT_SAMPLE_SESSION_BINDING_FRAME_V1_LEN as u16).to_be_bytes());
    payload.extend_from_slice(session_frame);
    payload.extend_from_slice(&step_index.to_be_bytes());
    payload.extend_from_slice(&target_net_id.to_be_bytes());
    payload.extend_from_slice(&anchor_net_id.to_be_bytes());
    payload.push(membership.code());
    for values in arrays {
        for bits in values {
            payload.extend_from_slice(&bits.to_be_bytes());
        }
    }
    debug_assert_eq!(payload.len(), expected_len);
    payload
}

fn payload_array_bits(payload: &[u8], dimension: usize, group: usize, axis: usize) -> u64 {
    let start = ARRAYS_START + (group * dimension + axis) * 8;
    read_u64(&payload[start..start + 8])
}

fn read_u16(bytes: &[u8]) -> u16 {
    let mut raw = [0_u8; 2];
    raw.copy_from_slice(bytes);
    u16::from_be_bytes(raw)
}

fn read_u32(bytes: &[u8]) -> u32 {
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(bytes);
    u32::from_be_bytes(raw)
}

fn read_u64(bytes: &[u8]) -> u64 {
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(bytes);
    u64::from_be_bytes(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;
    use symtropy_physics::{
        LocalEvidencePhysicsAuthorityWorld, LocalNamespacePhysicsAuthorityWorld,
        LocalQualifiedPhysicalAuthority, NetId, PhysicsWorld,
    };

    const GOLDEN_SESSION_FRAME_HEX: &str = "73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272800000001";
    const GOLDEN_PAYLOAD_HEX: &str = "000000010003008f73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728000000010000000000000001010203040506070811121314151617180100000000000000003fe0000000000000bfe000000000000040000000000000004008000000000000401000000000000040260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";
    const GOLDEN_FRAME_HEX: &str = "73796d74726f70792d706879736963732d65766964656e636500656e64706f696e742d73616d706c6500000000010000000000000140000000010003008f73796d74726f70792d706879736963732d65766964656e63650065766964656e63652d73657373696f6e2d62696e64696e670000000001000000000000005000000001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728000000010000000000000001010203040506070811121314151617180100000000000000003fe0000000000000bfe000000000000040000000000000004008000000000000401000000000000040260000000000004036000000000000403b00000000000040240000000000004034000000000000403e00000000000040240000000000004034800000000000403d8000000000003ff00000000000003ff8000000000000c004000000000000";

    fn decode_hex(text: &str) -> Vec<u8> {
        assert_eq!(text.len() % 2, 0);
        (0..text.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
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

    fn golden_payload() -> Vec<u8> {
        let session = decode_hex(GOLDEN_SESSION_FRAME_HEX);
        let center_offset = [0.0_f64.to_bits(), 0.5_f64.to_bits(), (-0.5_f64).to_bits()];
        let half_extents = [2.0_f64.to_bits(), 3.0_f64.to_bits(), 4.0_f64.to_bits()];
        let target_translation = [11.0_f64.to_bits(), 22.0_f64.to_bits(), 27.0_f64.to_bits()];
        let anchor_translation = [10.0_f64.to_bits(), 20.0_f64.to_bits(), 30.0_f64.to_bits()];
        let region_center = [10.0_f64.to_bits(), 20.5_f64.to_bits(), 29.5_f64.to_bits()];
        let offset = [1.0_f64.to_bits(), 1.5_f64.to_bits(), (-2.5_f64).to_bits()];
        let arrays: [&[u64]; ARRAY_GROUPS] = [
            &center_offset,
            &half_extents,
            &target_translation,
            &anchor_translation,
            &region_center,
            &offset,
        ];
        encode_endpoint_payload_fields_v1(
            3,
            &session,
            1,
            0x0102_0304_0506_0708,
            0x1112_1314_1516_1718,
            CanonicalEndpointMembershipV1::Inside,
            arrays,
            endpoint_payload_len_v1(3).unwrap(),
        )
    }

    fn prepared_session() -> (
        PersistentEvidenceSession<3>,
        PhysicsBodySubject,
        PhysicsBodySubject,
    ) {
        let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
        let generation = root.mint_generation().unwrap();
        let mut namespace =
            LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<3>::default());
        let anchor_handle = namespace
            .authority_world_mut()
            .world_mut()
            .add_sphere(Point::origin(), 0.5, 1.0);
        let target_handle = namespace
            .authority_world_mut()
            .world_mut()
            .add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
        let anchor = namespace
            .authority_world_mut()
            .bind_net_id(anchor_handle, NetId(9201))
            .unwrap();
        let target = namespace
            .authority_world_mut()
            .bind_net_id(target_handle, NetId(9202))
            .unwrap();
        let evidence = LocalEvidencePhysicsAuthorityWorld::seal(namespace)
            .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error()));
        let session = PersistentEvidenceSession::bootstrap(evidence)
            .unwrap_or_else(|failure| panic!("bootstrap failed: {:?}", failure.error()));
        (session, target, anchor)
    }

    #[test]
    fn private_field_codec_matches_language_neutral_golden_vector() {
        let payload = golden_payload();
        assert_eq!(payload.len(), 320);
        assert_eq!(to_hex(&payload), GOLDEN_PAYLOAD_HEX);
        let frame = encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, &payload)
            .unwrap();
        assert_eq!(to_hex(&frame), GOLDEN_FRAME_HEX);
        let decoded = decode_endpoint_sample_v1(&frame).unwrap();
        assert_eq!(decoded.dimension(), 3);
        assert_eq!(decoded.step_index(), 1);
        assert_eq!(decoded.target_net_id(), 0x0102_0304_0506_0708);
        assert_eq!(decoded.anchor_net_id(), 0x1112_1314_1516_1718);
        assert_eq!(decoded.membership(), CanonicalEndpointMembershipV1::Inside);
        let golden_session = decode_hex(GOLDEN_SESSION_FRAME_HEX);
        assert_eq!(decoded.session_binding_frame(), golden_session.as_slice());
    }

    #[test]
    fn fixture_contains_exact_rust_golden_bytes() {
        let fixture = include_str!("../tests/fixtures/endpoint_sample_v1_vector.json");
        assert!(fixture.contains(GOLDEN_SESSION_FRAME_HEX));
        assert!(fixture.contains(GOLDEN_PAYLOAD_HEX));
        assert!(fixture.contains(GOLDEN_FRAME_HEX));
    }

    #[test]
    fn persistent_capture_requires_a_current_qualified_step() {
        let (session, target, anchor) = prepared_session();
        let region = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [2.0; 3]).unwrap();
        assert_eq!(
            capture_and_encode_current_endpoint_sample_v1(&session, target, region),
            Err(EndpointSampleCodecV1Error::EndpointCapture(
                LocalQualifiedStampedEndpointObservationError::NoQualifiedStep
            ))
        );
    }

    #[test]
    fn production_encoder_captures_inside_established_session() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let region = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [2.0; 3]).unwrap();
        let frame = capture_and_encode_current_endpoint_sample_v1(&session, target, region).unwrap();
        let decoded = decode_endpoint_sample_v1(&frame).unwrap();

        assert_eq!(decoded.step_index(), 1);
        assert_eq!(decoded.target_net_id(), target.net_id().0);
        assert_eq!(decoded.anchor_net_id(), anchor.net_id().0);
        assert_eq!(
            decoded.session_binding().session_id_bytes(),
            session.binding().session_id().as_bytes()
        );
        assert_eq!(
            decoded.session_binding().temporal_incarnation_raw(),
            session.temporal_incarnation_id().get()
        );
    }

    #[test]
    fn private_detached_composition_rejects_mismatched_session() {
        let (session_a, _, _) = prepared_session();
        let (mut session_b, target_b, anchor_b) = prepared_session();
        session_b.step_authorized(1.0 / 64.0).unwrap();
        let token_b = LocalQualifiedStampedEndpointBoxObservation::capture(
            session_b.evidence_authority(),
            target_b,
            AuthorityEndpointBoxSpec::new(anchor_b, [0.0; 3], [2.0; 3]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            encode_qualified_endpoint_token_v1(session_a.binding(), &token_b),
            Err(EndpointSampleCodecV1Error::QualifiedSessionMismatch)
        );
    }

    fn frame_payload(payload: &[u8]) -> Vec<u8> {
        encode_physics_evidence_frame_v1(PhysicsEvidenceFrameKindV1::EndpointSample, payload).unwrap()
    }

    #[test]
    fn malformed_and_internally_inconsistent_payloads_fail_closed() {
        let payload = golden_payload();

        let mut zero_step = payload.clone();
        zero_step[STEP_INDEX_START..TARGET_NET_ID_START].fill(0);
        assert_eq!(
            decode_endpoint_sample_v1(&frame_payload(&zero_step)),
            Err(EndpointSampleCodecV1Error::StepIndexZero)
        );

        let mut same_subject = payload.clone();
        let target = same_subject[TARGET_NET_ID_START..ANCHOR_NET_ID_START].to_vec();
        same_subject[ANCHOR_NET_ID_START..MEMBERSHIP_START].copy_from_slice(&target);
        assert!(matches!(
            decode_endpoint_sample_v1(&frame_payload(&same_subject)),
            Err(EndpointSampleCodecV1Error::SameTargetAnchorNetId { .. })
        ));

        let mut bad_center = payload.clone();
        let region_center_x = ARRAYS_START + REGION_CENTER_GROUP * 3 * 8;
        bad_center[region_center_x..region_center_x + 8]
            .copy_from_slice(&11.0_f64.to_bits().to_be_bytes());
        assert_eq!(
            decode_endpoint_sample_v1(&frame_payload(&bad_center)),
            Err(EndpointSampleCodecV1Error::RegionCenterMismatch { axis: 0 })
        );

        let mut bad_offset = payload.clone();
        let offset_x = ARRAYS_START + OFFSET_FROM_CENTER_GROUP * 3 * 8;
        bad_offset[offset_x..offset_x + 8]
            .copy_from_slice(&0.5_f64.to_bits().to_be_bytes());
        assert_eq!(
            decode_endpoint_sample_v1(&frame_payload(&bad_offset)),
            Err(EndpointSampleCodecV1Error::OffsetFromCenterMismatch { axis: 0 })
        );

        let mut bad_membership = payload.clone();
        bad_membership[MEMBERSHIP_START] = 0;
        assert!(matches!(
            decode_endpoint_sample_v1(&frame_payload(&bad_membership)),
            Err(EndpointSampleCodecV1Error::MembershipMismatch { .. })
        ));

        let mut negative_zero = payload.clone();
        negative_zero[ARRAYS_START..ARRAYS_START + 8]
            .copy_from_slice(&(-0.0_f64).to_bits().to_be_bytes());
        assert_eq!(
            decode_endpoint_sample_v1(&frame_payload(&negative_zero)),
            Err(EndpointSampleCodecV1Error::NonCanonicalCenterOffsetZero { axis: 0 })
        );
    }

    #[test]
    fn closed_box_boundary_is_inside() {
        let mut payload = golden_payload();
        let offset_x = ARRAYS_START + OFFSET_FROM_CENTER_GROUP * 3 * 8;
        payload[offset_x..offset_x + 8]
            .copy_from_slice(&2.0_f64.to_bits().to_be_bytes());
        let target_x = ARRAYS_START + TARGET_TRANSLATION_GROUP * 3 * 8;
        payload[target_x..target_x + 8]
            .copy_from_slice(&12.0_f64.to_bits().to_be_bytes());
        assert_eq!(
            decode_endpoint_sample_v1(&frame_payload(&payload))
                .unwrap()
                .membership(),
            CanonicalEndpointMembershipV1::Inside
        );
    }
}
