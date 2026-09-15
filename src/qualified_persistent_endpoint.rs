// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Issuer-qualified persistent endpoint samples.
//!
//! Canonical PHYS-EVID-02C bytes are representation, not issuer authority.
//! This module retains the opaque PHYS-OBS-05 live token beside the exact
//! canonical frame and exposes no deserialization/promotion path from bytes.

use crate::endpoint_sample_codec::{
    CanonicalEndpointMembershipV1, DecodedEndpointSampleV1, EndpointSampleCodecV1Error,
    capture_and_encode_current_endpoint_sample_v1, decode_endpoint_sample_v1,
};
use crate::evidence_session::PersistentEvidenceSession;
use symtropy_physics::{
    AuthorityEndpointBoxSpec, EndpointMembership, LocalQualifiedStampedEndpointBoxObservation,
    LocalQualifiedStampedEndpointObservationError, PhysicsBodySubject,
};

/// Issuer-side proof that one canonical PHYS-EVID-02C frame was produced from
/// the same exact live endpoint sample retained here.
///
/// This type is deliberately non-Clone, non-Copy, and non-Serde. It cannot be
/// reconstructed from canonical bytes after persistence/restart.
pub struct QualifiedPersistentEndpointSampleV1<const D: usize> {
    live_token: LocalQualifiedStampedEndpointBoxObservation<D>,
    frame: Vec<u8>,
}

impl<const D: usize> QualifiedPersistentEndpointSampleV1<D> {
    /// Capture one issuer-qualified persistent endpoint sample through an
    /// already-established persistence session.
    ///
    /// V0.1 intentionally uses two immutable captures: PHYS-EVID-02C produces
    /// the canonical frame, then PHYS-OBS-05 produces the retained opaque token
    /// under the same immutable session/world state. Exact field comparison
    /// below proves both projections refer to one semantic sample.
    pub fn capture(
        session: &PersistentEvidenceSession<D>,
        subject: PhysicsBodySubject,
        region: AuthorityEndpointBoxSpec<D>,
    ) -> Result<Self, QualifiedPersistentEndpointSampleV1Error> {
        let frame = capture_and_encode_current_endpoint_sample_v1(session, subject, region)
            .map_err(QualifiedPersistentEndpointSampleV1Error::EndpointCodec)?;

        let live_token = LocalQualifiedStampedEndpointBoxObservation::capture(
            session.evidence_authority(),
            subject,
            region,
        )
        .map_err(QualifiedPersistentEndpointSampleV1Error::EndpointCapture)?;

        let decoded = decode_endpoint_sample_v1(&frame)
            .map_err(QualifiedPersistentEndpointSampleV1Error::EndpointCodec)?;
        ensure_exact_match(session, &live_token, &decoded)?;

        Ok(Self { live_token, frame })
    }

    /// Read-only downward projection to the original issuer-qualified live
    /// PHYS-OBS-05 token. Future strong derived evidence should consume this
    /// evidence class rather than raw canonical bytes.
    pub const fn live_token(&self) -> &LocalQualifiedStampedEndpointBoxObservation<D> {
        &self.live_token
    }

    /// Read-only canonical PHYS-EVID-02C frame bytes.
    pub fn frame_bytes(&self) -> &[u8] {
        &self.frame
    }

    /// Consuming downgrade to representation-only bytes.
    ///
    /// The stronger issuer-qualified wrapper is lost and cannot be recreated
    /// from the returned bytes through this module.
    pub fn into_frame_bytes(self) -> Vec<u8> {
        self.frame
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QualifiedPersistentEndpointSampleV1Error {
    EndpointCodec(EndpointSampleCodecV1Error),
    EndpointCapture(LocalQualifiedStampedEndpointObservationError),
    DimensionMismatch { expected: usize, actual: usize },
    SessionMismatch,
    StepMismatch { live: u64, canonical: u64 },
    TargetMismatch { live: u64, canonical: u64 },
    AnchorMismatch { live: u64, canonical: u64 },
    MembershipMismatch,
    CenterOffsetMismatch { axis: usize },
    HalfExtentMismatch { axis: usize },
    TargetTranslationMismatch { axis: usize },
    AnchorTranslationMismatch { axis: usize },
    RegionCenterMismatch { axis: usize },
    OffsetFromCenterMismatch { axis: usize },
    DecodedInvariant,
}

fn ensure_exact_match<const D: usize>(
    session: &PersistentEvidenceSession<D>,
    token: &LocalQualifiedStampedEndpointBoxObservation<D>,
    decoded: &DecodedEndpointSampleV1<'_>,
) -> Result<(), QualifiedPersistentEndpointSampleV1Error> {
    if decoded.dimension() != D {
        return Err(QualifiedPersistentEndpointSampleV1Error::DimensionMismatch {
            expected: D,
            actual: decoded.dimension(),
        });
    }

    let decoded_session = decoded.session_binding();
    let binding = session.binding();
    if decoded_session.session_id_bytes() != binding.session_id().as_bytes()
        || decoded_session.physical_authority_raw() != binding.physical_authority_id().get()
        || decoded_session.world_generation_raw() != binding.world_generation_id().get()
        || decoded_session.temporal_incarnation_raw() != binding.temporal_incarnation_id().get()
        || decoded_session.session_profile_code() != binding.profile().code()
    {
        return Err(QualifiedPersistentEndpointSampleV1Error::SessionMismatch);
    }

    let stamp = token.stamp();
    if stamp.physical_authority_id() != binding.physical_authority_id()
        || stamp.world_generation_id() != binding.world_generation_id()
        || stamp.temporal_incarnation_id() != binding.temporal_incarnation_id()
    {
        return Err(QualifiedPersistentEndpointSampleV1Error::SessionMismatch);
    }
    if decoded.step_index() != stamp.step_index() {
        return Err(QualifiedPersistentEndpointSampleV1Error::StepMismatch {
            live: stamp.step_index(),
            canonical: decoded.step_index(),
        });
    }

    let observation = token.observation();
    let target_net_id = observation.subject.subject.net_id().0;
    let anchor_net_id = observation.anchor.subject.net_id().0;
    if decoded.target_net_id() != target_net_id {
        return Err(QualifiedPersistentEndpointSampleV1Error::TargetMismatch {
            live: target_net_id,
            canonical: decoded.target_net_id(),
        });
    }
    if decoded.anchor_net_id() != anchor_net_id {
        return Err(QualifiedPersistentEndpointSampleV1Error::AnchorMismatch {
            live: anchor_net_id,
            canonical: decoded.anchor_net_id(),
        });
    }

    let live_membership = match observation.membership {
        EndpointMembership::Outside => CanonicalEndpointMembershipV1::Outside,
        EndpointMembership::Inside => CanonicalEndpointMembershipV1::Inside,
    };
    if decoded.membership() != live_membership {
        return Err(QualifiedPersistentEndpointSampleV1Error::MembershipMismatch);
    }

    let center_offset = observation.region.center_offset();
    let half_extents = observation.region.half_extents();
    for axis in 0..D {
        if decoded.center_offset_bits(axis)
            != Some(center_offset[axis].to_bits())
        {
            return Err(QualifiedPersistentEndpointSampleV1Error::CenterOffsetMismatch { axis });
        }
        if decoded.half_extent_bits(axis)
            != Some(half_extents[axis].to_bits())
        {
            return Err(QualifiedPersistentEndpointSampleV1Error::HalfExtentMismatch { axis });
        }
        if decoded.target_translation_bits(axis) != Some(observation.subject.translation[axis]) {
            return Err(
                QualifiedPersistentEndpointSampleV1Error::TargetTranslationMismatch { axis },
            );
        }
        if decoded.anchor_translation_bits(axis) != Some(observation.anchor.translation[axis]) {
            return Err(
                QualifiedPersistentEndpointSampleV1Error::AnchorTranslationMismatch { axis },
            );
        }
        if decoded.region_center_bits(axis) != Some(observation.region_center[axis]) {
            return Err(QualifiedPersistentEndpointSampleV1Error::RegionCenterMismatch { axis });
        }
        if decoded.offset_from_center_bits(axis) != Some(observation.offset_from_center[axis]) {
            return Err(QualifiedPersistentEndpointSampleV1Error::OffsetFromCenterMismatch { axis });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_session::PersistentEvidenceSession;
    use symtropy_math::Point;
    use symtropy_physics::{
        LocalEvidencePhysicsAuthorityWorld, LocalNamespacePhysicsAuthorityWorld,
        LocalQualifiedPhysicalAuthority, NetId, PhysicsEvidenceFrameKindV1, PhysicsWorld,
        decode_physics_evidence_frame_v1, encode_physics_evidence_frame_v1,
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
            .bind_net_id(anchor_handle, NetId(9301))
            .unwrap();
        let target = namespace
            .authority_world_mut()
            .bind_net_id(target_handle, NetId(9302))
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
    fn issuer_wrapper_requires_a_current_qualified_step() {
        let (session, target, anchor) = prepared_session();
        let error = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .err()
        .expect("capture before step must fail");
        assert_eq!(
            error,
            QualifiedPersistentEndpointSampleV1Error::EndpointCodec(
                EndpointSampleCodecV1Error::EndpointCapture(
                    LocalQualifiedStampedEndpointObservationError::NoQualifiedStep,
                ),
            )
        );
    }

    #[test]
    fn successful_wrapper_binds_exact_live_token_and_canonical_frame() {
        let (mut session, target, anchor) = prepared_session();
        let issued = session.step_authorized(1.0 / 64.0).unwrap();
        let qualified = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();

        assert_eq!(qualified.live_token().stamp(), issued);
        let decoded = decode_endpoint_sample_v1(qualified.frame_bytes()).unwrap();
        ensure_exact_match(&session, qualified.live_token(), &decoded).unwrap();

        // A fresh representation capture under the same immutable current
        // session state is byte-identical; repeated capture is not progression.
        let direct = capture_and_encode_current_endpoint_sample_v1(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        assert_eq!(qualified.frame_bytes(), direct.as_slice());
    }

    #[test]
    fn consuming_downgrade_preserves_exact_frame_and_loses_stronger_wrapper() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let qualified = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();
        let before = qualified.frame_bytes().to_vec();
        let downgraded = qualified.into_frame_bytes();
        assert_eq!(downgraded, before);
        decode_endpoint_sample_v1(&downgraded).unwrap();
    }

    #[test]
    fn exact_match_rejects_a_different_but_canonical_target_identity() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let qualified = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();

        let outer = decode_physics_evidence_frame_v1(qualified.frame_bytes()).unwrap();
        assert_eq!(outer.kind(), PhysicsEvidenceFrameKindV1::EndpointSample);
        let mut payload = outer.payload().to_vec();
        payload[159..167].copy_from_slice(&9999_u64.to_be_bytes());
        let altered = encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::EndpointSample,
            &payload,
        )
        .unwrap();
        let decoded = decode_endpoint_sample_v1(&altered).unwrap();

        assert_eq!(
            ensure_exact_match(&session, qualified.live_token(), &decoded),
            Err(QualifiedPersistentEndpointSampleV1Error::TargetMismatch {
                live: target.net_id().0,
                canonical: 9999,
            })
        );
    }

    #[test]
    fn exact_match_rejects_a_different_but_canonical_session_id() {
        let (mut session, target, anchor) = prepared_session();
        session.step_authorized(1.0 / 64.0).unwrap();
        let qualified = QualifiedPersistentEndpointSampleV1::capture(
            &session,
            target,
            region(anchor),
        )
        .unwrap();

        let outer = decode_physics_evidence_frame_v1(qualified.frame_bytes()).unwrap();
        let mut payload = outer.payload().to_vec();
        let nested = payload[8..151].to_vec();
        let decoded_nested = decode_physics_evidence_frame_v1(&nested).unwrap();
        let mut nested_payload = decoded_nested.payload().to_vec();
        nested_payload[4] ^= 0x80;
        let rebuilt_nested = encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::EvidenceSessionBinding,
            &nested_payload,
        )
        .unwrap();
        payload[8..151].copy_from_slice(&rebuilt_nested);
        let altered = encode_physics_evidence_frame_v1(
            PhysicsEvidenceFrameKindV1::EndpointSample,
            &payload,
        )
        .unwrap();
        let decoded = decode_endpoint_sample_v1(&altered).unwrap();

        assert_eq!(
            ensure_exact_match(&session, qualified.live_token(), &decoded),
            Err(QualifiedPersistentEndpointSampleV1Error::SessionMismatch)
        );
    }
}
