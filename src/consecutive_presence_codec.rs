// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical persistent consecutive sampled endpoint presence (PHYS-EVID-02D).
//!
//! Strong production accepts only issuer-qualified persistent endpoint samples,
//! delegates live adjacency/proposition/Inside semantics to PHYS-OBS-06, then
//! preserves the two exact PHYS-EVID-02C endpoint frames in previous/current
//! order. Decoding returns only untrusted persistent semantics.

use crate::endpoint_sample_codec::{
    CanonicalEndpointMembershipV1, DecodedEndpointSampleV1, EndpointSampleCodecV1Error,
    decode_endpoint_sample_v1,
};
use crate::qualified_persistent_endpoint::QualifiedPersistentEndpointSampleV1;
use symtropy_physics::{
    LocalConsecutiveSampledEndpointPresence, LocalConsecutiveSampledPresenceError,
    PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN, PhysicsEvidenceFrameKindV1,
    PhysicsEvidenceFrameV1Error, decode_physics_evidence_frame_v1,
    encode_physics_evidence_frame_v1,
};

pub const CONSECUTIVE_PRESENCE_PAYLOAD_V1_VERSION: u32 = 1;
pub const CONSECUTIVE_PRESENCE_FIXED_V1_LEN: usize = 12;

const VERSION_START: usize = 0;
const PREVIOUS_LEN_START: usize = 4;
const PREVIOUS_FRAME_START: usize = 8;

/// Issuer-side proof that one exact ordered pair of issuer-qualified endpoint
/// samples satisfies PHYS-OBS-06 and has one canonical PHYS-EVID-02D frame.
///
/// This type is intentionally non-Clone, non-Copy, and non-Serde. Canonical
/// bytes cannot reconstruct it after downgrade or restart.
pub struct QualifiedPersistentConsecutivePresenceV1<const D: usize> {
    live_relation: LocalConsecutiveSampledEndpointPresence<D>,
    frame: Vec<u8>,
}

impl<const D: usize> QualifiedPersistentConsecutivePresenceV1<D> {
    /// Build one persistent consecutive-presence proof from two issuer-qualified
    /// persistent endpoint samples.
    ///
    /// PHYS-OBS-06 owns live lineage/proposition/membership/adjacency semantics.
    /// This module only adds ordered persistent composition and an independent
    /// persistent decoder check over the exact canonical predecessor frames.
    pub fn try_from_samples(
        previous: &QualifiedPersistentEndpointSampleV1<D>,
        current: &QualifiedPersistentEndpointSampleV1<D>,
    ) -> Result<Self, ConsecutivePresenceCodecV1Error> {
        let live_relation = LocalConsecutiveSampledEndpointPresence::try_from_samples(
            previous.live_token(),
            current.live_token(),
        )
        .map_err(ConsecutivePresenceCodecV1Error::LiveRelation)?;

        let frame = encode_ordered_endpoint_frames_v1(
            previous.frame_bytes(),
            current.frame_bytes(),
        )?;

        let decoded = decode_consecutive_sampled_presence_v1(&frame)?;
        if decoded.previous_frame() != previous.frame_bytes()
            || decoded.current_frame() != current.frame_bytes()
        {
            return Err(ConsecutivePresenceCodecV1Error::ProducerInvariant);
        }

        Ok(Self {
            live_relation,
            frame,
        })
    }

    pub const fn live_relation(&self) -> &LocalConsecutiveSampledEndpointPresence<D> {
        &self.live_relation
    }

    pub fn frame_bytes(&self) -> &[u8] {
        &self.frame
    }

    /// Consuming downgrade to representation-only bytes.
    pub fn into_frame_bytes(self) -> Vec<u8> {
        self.frame
    }
}

/// Borrowed, untrusted canonical persistent consecutive-presence semantics.
#[derive(Debug)]
pub struct DecodedConsecutiveSampledPresenceV1<'a> {
    previous_frame: &'a [u8],
    current_frame: &'a [u8],
    previous: DecodedEndpointSampleV1<'a>,
    current: DecodedEndpointSampleV1<'a>,
}

impl<'a> DecodedConsecutiveSampledPresenceV1<'a> {
    pub const fn previous_frame(&self) -> &'a [u8] {
        self.previous_frame
    }

    pub const fn current_frame(&self) -> &'a [u8] {
        self.current_frame
    }

    pub const fn previous(&self) -> &DecodedEndpointSampleV1<'a> {
        &self.previous
    }

    pub const fn current(&self) -> &DecodedEndpointSampleV1<'a> {
        &self.current
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsecutivePresenceCodecV1Error {
    Frame(PhysicsEvidenceFrameV1Error),
    Endpoint(EndpointSampleCodecV1Error),
    LiveRelation(LocalConsecutiveSampledPresenceError),
    WrongFrameKind { actual: PhysicsEvidenceFrameKindV1 },
    PayloadTooShort { actual: usize },
    UnsupportedPayloadVersion { version: u32 },
    EmbeddedLengthOverflow,
    TruncatedPreviousFrame { declared: usize, available: usize },
    MissingCurrentLength,
    TruncatedCurrentFrame { declared: usize, available: usize },
    TrailingPayloadBytes { trailing: usize },
    PayloadTooLarge { len: usize },
    PersistentSessionMismatch,
    DimensionMismatch { previous: usize, current: usize },
    TargetMismatch { previous: u64, current: u64 },
    AnchorMismatch { previous: u64, current: u64 },
    CenterOffsetMismatch { axis: usize },
    HalfExtentMismatch { axis: usize },
    PreviousOutside,
    CurrentOutside,
    DuplicateStep { step_index: u64 },
    ReversedStep { previous_step: u64, current_step: u64 },
    StepGap { previous_step: u64, current_step: u64 },
    DecodedInvariant,
    ProducerInvariant,
}

/// Decode and semantically admit one canonical PHYS-EVID-02D relation.
///
/// This function allocates nothing from attacker-declared embedded lengths. It
/// slices the already-bounded outer payload, delegates each predecessor to the
/// PHYS-EVID-02C decoder, then independently checks persistent relation
/// semantics. It never reconstructs issuer-side authority types.
pub fn decode_consecutive_sampled_presence_v1(
    frame: &[u8],
) -> Result<DecodedConsecutiveSampledPresenceV1<'_>, ConsecutivePresenceCodecV1Error> {
    let outer = decode_physics_evidence_frame_v1(frame)
        .map_err(ConsecutivePresenceCodecV1Error::Frame)?;
    if outer.kind() != PhysicsEvidenceFrameKindV1::ConsecutiveSampledPresence {
        return Err(ConsecutivePresenceCodecV1Error::WrongFrameKind {
            actual: outer.kind(),
        });
    }

    let payload = outer.payload();
    if payload.len() < PREVIOUS_FRAME_START {
        return Err(ConsecutivePresenceCodecV1Error::PayloadTooShort {
            actual: payload.len(),
        });
    }

    let version = read_u32(&payload[VERSION_START..PREVIOUS_LEN_START]);
    if version != CONSECUTIVE_PRESENCE_PAYLOAD_V1_VERSION {
        return Err(ConsecutivePresenceCodecV1Error::UnsupportedPayloadVersion { version });
    }

    let previous_len = read_u32(&payload[PREVIOUS_LEN_START..PREVIOUS_FRAME_START]) as usize;
    let previous_end = PREVIOUS_FRAME_START
        .checked_add(previous_len)
        .ok_or(ConsecutivePresenceCodecV1Error::EmbeddedLengthOverflow)?;
    if previous_end > payload.len() {
        return Err(ConsecutivePresenceCodecV1Error::TruncatedPreviousFrame {
            declared: previous_len,
            available: payload.len().saturating_sub(PREVIOUS_FRAME_START),
        });
    }

    let current_len_end = previous_end
        .checked_add(4)
        .ok_or(ConsecutivePresenceCodecV1Error::EmbeddedLengthOverflow)?;
    if current_len_end > payload.len() {
        return Err(ConsecutivePresenceCodecV1Error::MissingCurrentLength);
    }
    let current_len = read_u32(&payload[previous_end..current_len_end]) as usize;
    let current_end = current_len_end
        .checked_add(current_len)
        .ok_or(ConsecutivePresenceCodecV1Error::EmbeddedLengthOverflow)?;
    if current_end > payload.len() {
        return Err(ConsecutivePresenceCodecV1Error::TruncatedCurrentFrame {
            declared: current_len,
            available: payload.len().saturating_sub(current_len_end),
        });
    }
    if current_end != payload.len() {
        return Err(ConsecutivePresenceCodecV1Error::TrailingPayloadBytes {
            trailing: payload.len() - current_end,
        });
    }

    let previous_frame = &payload[PREVIOUS_FRAME_START..previous_end];
    let current_frame = &payload[current_len_end..current_end];
    let previous = decode_endpoint_sample_v1(previous_frame)
        .map_err(ConsecutivePresenceCodecV1Error::Endpoint)?;
    let current = decode_endpoint_sample_v1(current_frame)
        .map_err(ConsecutivePresenceCodecV1Error::Endpoint)?;

    ensure_persistent_relation(&previous, &current)?;

    Ok(DecodedConsecutiveSampledPresenceV1 {
        previous_frame,
        current_frame,
        previous,
        current,
    })
}

fn encode_ordered_endpoint_frames_v1(
    previous: &[u8],
    current: &[u8],
) -> Result<Vec<u8>, ConsecutivePresenceCodecV1Error> {
    let previous_len = u32::try_from(previous.len())
        .map_err(|_| ConsecutivePresenceCodecV1Error::EmbeddedLengthOverflow)?;
    let current_len = u32::try_from(current.len())
        .map_err(|_| ConsecutivePresenceCodecV1Error::EmbeddedLengthOverflow)?;
    let payload_len = CONSECUTIVE_PRESENCE_FIXED_V1_LEN
        .checked_add(previous.len())
        .and_then(|value| value.checked_add(current.len()))
        .ok_or(ConsecutivePresenceCodecV1Error::EmbeddedLengthOverflow)?;
    if payload_len > PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN {
        return Err(ConsecutivePresenceCodecV1Error::PayloadTooLarge { len: payload_len });
    }

    let mut payload = Vec::with_capacity(payload_len);
    payload.extend_from_slice(&CONSECUTIVE_PRESENCE_PAYLOAD_V1_VERSION.to_be_bytes());
    payload.extend_from_slice(&previous_len.to_be_bytes());
    payload.extend_from_slice(previous);
    payload.extend_from_slice(&current_len.to_be_bytes());
    payload.extend_from_slice(current);

    encode_physics_evidence_frame_v1(
        PhysicsEvidenceFrameKindV1::ConsecutiveSampledPresence,
        &payload,
    )
    .map_err(ConsecutivePresenceCodecV1Error::Frame)
}

fn ensure_persistent_relation(
    previous: &DecodedEndpointSampleV1<'_>,
    current: &DecodedEndpointSampleV1<'_>,
) -> Result<(), ConsecutivePresenceCodecV1Error> {
    let previous_session = previous.session_binding();
    let current_session = current.session_binding();
    if previous_session.session_id_bytes() != current_session.session_id_bytes()
        || previous_session.physical_authority_raw() != current_session.physical_authority_raw()
        || previous_session.world_generation_raw() != current_session.world_generation_raw()
        || previous_session.temporal_incarnation_raw() != current_session.temporal_incarnation_raw()
        || previous_session.session_profile_code() != current_session.session_profile_code()
    {
        return Err(ConsecutivePresenceCodecV1Error::PersistentSessionMismatch);
    }

    if previous.dimension() != current.dimension() {
        return Err(ConsecutivePresenceCodecV1Error::DimensionMismatch {
            previous: previous.dimension(),
            current: current.dimension(),
        });
    }
    if previous.target_net_id() != current.target_net_id() {
        return Err(ConsecutivePresenceCodecV1Error::TargetMismatch {
            previous: previous.target_net_id(),
            current: current.target_net_id(),
        });
    }
    if previous.anchor_net_id() != current.anchor_net_id() {
        return Err(ConsecutivePresenceCodecV1Error::AnchorMismatch {
            previous: previous.anchor_net_id(),
            current: current.anchor_net_id(),
        });
    }

    for axis in 0..previous.dimension() {
        if previous.center_offset_bits(axis) != current.center_offset_bits(axis) {
            return Err(ConsecutivePresenceCodecV1Error::CenterOffsetMismatch { axis });
        }
        if previous.half_extent_bits(axis) != current.half_extent_bits(axis) {
            return Err(ConsecutivePresenceCodecV1Error::HalfExtentMismatch { axis });
        }
    }

    if previous.membership() != CanonicalEndpointMembershipV1::Inside {
        return Err(ConsecutivePresenceCodecV1Error::PreviousOutside);
    }
    if current.membership() != CanonicalEndpointMembershipV1::Inside {
        return Err(ConsecutivePresenceCodecV1Error::CurrentOutside);
    }

    let previous_step = previous.step_index();
    let current_step = current.step_index();
    if current_step == previous_step {
        return Err(ConsecutivePresenceCodecV1Error::DuplicateStep {
            step_index: previous_step,
        });
    }
    if current_step < previous_step {
        return Err(ConsecutivePresenceCodecV1Error::ReversedStep {
            previous_step,
            current_step,
        });
    }
    match previous_step.checked_add(1) {
        Some(expected) if current_step == expected => {}
        Some(_) => {
            return Err(ConsecutivePresenceCodecV1Error::StepGap {
                previous_step,
                current_step,
            });
        }
        None => {
            return Err(ConsecutivePresenceCodecV1Error::StepGap {
                previous_step,
                current_step,
            });
        }
    }

    Ok(())
}

fn read_u32(bytes: &[u8]) -> u32 {
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(bytes);
    u32::from_be_bytes(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_session::PersistentEvidenceSession;
    use crate::qualified_persistent_endpoint::QualifiedPersistentEndpointSampleV1;
    use symtropy_math::Point;
    use symtropy_physics::{
        AuthorityEndpointBoxSpec, LocalEvidencePhysicsAuthorityWorld,
        LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority, NetId,
        PhysicsBodySubject, PhysicsWorld,
    };

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
            .bind_net_id(anchor_handle, NetId(9401))
            .unwrap();
        let target = namespace
            .authority_world_mut()
            .bind_net_id(target_handle, NetId(9402))
            .unwrap();
        let evidence = LocalEvidencePhysicsAuthorityWorld::seal(namespace)
            .unwrap_or_else(|failure| panic!("seal failed: {:?}", failure.error()));
        let session = PersistentEvidenceSession::bootstrap(evidence)
            .unwrap_or_else(|failure| panic!("bootstrap failed: {:?}", failure.error()));
        (session, target, anchor)
    }

    fn region(anchor: PhysicsBodySubject) -> AuthorityEndpointBoxSpec<3> {
        AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [2.0; 3]).unwrap()
    }

    #[test]
    fn adjacent_issuer_qualified_samples_produce_ordered_persistent_relation() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let previous = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        session.step_authorized(1.0 / 64.0).unwrap();
        let current = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();

        let relation = QualifiedPersistentConsecutivePresenceV1::try_from_samples(
            &previous,
            &current,
        )
        .unwrap();
        assert_eq!(relation.live_relation().previous_stamp().step_index(), 1);
        assert_eq!(relation.live_relation().current_stamp().step_index(), 2);

        let decoded = decode_consecutive_sampled_presence_v1(relation.frame_bytes()).unwrap();
        assert_eq!(decoded.previous().step_index(), 1);
        assert_eq!(decoded.current().step_index(), 2);
        assert_eq!(decoded.previous_frame(), previous.frame_bytes());
        assert_eq!(decoded.current_frame(), current.frame_bytes());
        assert_eq!(
            relation.frame_bytes().len(),
            819,
            "D=3 relation over two 374-byte endpoint frames has frozen length",
        );
    }

    #[test]
    fn strong_producer_reuses_live_duplicate_reversed_and_gap_rejection() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let first = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        let duplicate = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        assert!(matches!(
            QualifiedPersistentConsecutivePresenceV1::try_from_samples(&first, &duplicate),
            Err(ConsecutivePresenceCodecV1Error::LiveRelation(
                LocalConsecutiveSampledPresenceError::DuplicateStep { .. }
            ))
        ));

        session.step_authorized(1.0 / 64.0).unwrap();
        let second = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        assert!(matches!(
            QualifiedPersistentConsecutivePresenceV1::try_from_samples(&second, &first),
            Err(ConsecutivePresenceCodecV1Error::LiveRelation(
                LocalConsecutiveSampledPresenceError::ReversedStep { .. }
            ))
        ));

        session.step_authorized(1.0 / 64.0).unwrap();
        session.step_authorized(1.0 / 64.0).unwrap();
        let fourth = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        assert!(matches!(
            QualifiedPersistentConsecutivePresenceV1::try_from_samples(&second, &fourth),
            Err(ConsecutivePresenceCodecV1Error::LiveRelation(
                LocalConsecutiveSampledPresenceError::StepGap { .. }
            ))
        ));
    }

    #[test]
    fn decoder_preserves_order_and_rejects_reversal() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let previous = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        session.step_authorized(1.0 / 64.0).unwrap();
        let current = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();

        let reversed = encode_ordered_endpoint_frames_v1(
            current.frame_bytes(),
            previous.frame_bytes(),
        )
        .unwrap();
        assert!(matches!(
            decode_consecutive_sampled_presence_v1(&reversed),
            Err(ConsecutivePresenceCodecV1Error::ReversedStep { .. })
        ));
    }

    #[test]
    fn decoder_rejects_different_persistent_session_even_with_canonical_endpoints() {
        let (mut session_a, target_a, anchor_a) = prepared_session();
        let (mut session_b, target_b, anchor_b) = prepared_session();
        session_a.step_authorized(1.0 / 64.0).unwrap();
        let previous = QualifiedPersistentEndpointSampleV1::capture(
            &session_a,
            target_a,
            region(anchor_a),
        )
        .unwrap();
        session_b.step_authorized(1.0 / 64.0).unwrap();
        session_b.step_authorized(1.0 / 64.0).unwrap();
        let current = QualifiedPersistentEndpointSampleV1::capture(
            &session_b,
            target_b,
            region(anchor_b),
        )
        .unwrap();

        let frame = encode_ordered_endpoint_frames_v1(
            previous.frame_bytes(),
            current.frame_bytes(),
        )
        .unwrap();
        assert_eq!(
            decode_consecutive_sampled_presence_v1(&frame).err(),
            Some(ConsecutivePresenceCodecV1Error::PersistentSessionMismatch)
        );
    }
}
