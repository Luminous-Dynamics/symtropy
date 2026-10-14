// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Canonical framing primitive for future persistent physics evidence payloads.
//!
//! This module freezes byte framing and type/domain separation only. Framing
//! arbitrary payload bytes does not qualify those bytes as physics evidence,
//! and decoding a frame never recreates a live authority-issued evidence token.

/// Exact application namespace for canonical physics evidence framing v1.
pub const PHYSICS_EVIDENCE_FRAME_V1_MAGIC: &[u8] = b"symtropy-physics-evidence\0";
/// Exact framing version committed into every v1 frame.
pub const PHYSICS_EVIDENCE_FRAME_V1_VERSION: u32 = 1;
/// Maximum admitted payload size for v1 framing.
pub const PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN: usize = 1_048_576;
/// Maximum admitted evidence-kind domain length, excluding its NUL terminator.
pub const PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN: usize = 32;

const VERSION_BYTES: usize = 4;
const LENGTH_BYTES: usize = 8;

/// Closed v1 registry of framing-level physics evidence families.
///
/// These discriminants provide byte-level type separation. They do not by
/// themselves certify the semantic strength or provenance of a payload.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum PhysicsEvidenceFrameKindV1 {
    EvidenceSessionBinding,
    EndpointSample,
    ConsecutiveSampledPresence,
    StepExecutionReceipt,
    FixedCadenceStepReceipt,
    TimedSampledPresenceTransition,
}

impl PhysicsEvidenceFrameKindV1 {
    /// Frozen ASCII domain bytes for this v1 kind.
    pub const fn domain_bytes(self) -> &'static [u8] {
        match self {
            Self::EvidenceSessionBinding => b"evidence-session-binding",
            Self::EndpointSample => b"endpoint-sample",
            Self::ConsecutiveSampledPresence => b"consecutive-presence",
            Self::StepExecutionReceipt => b"step-execution-receipt",
            Self::FixedCadenceStepReceipt => b"fixed-cadence-step-receipt",
            Self::TimedSampledPresenceTransition => b"timed-sampled-transition",
        }
    }

    fn from_domain_bytes(domain: &[u8]) -> Option<Self> {
        match domain {
            b"evidence-session-binding" => Some(Self::EvidenceSessionBinding),
            b"endpoint-sample" => Some(Self::EndpointSample),
            b"consecutive-presence" => Some(Self::ConsecutiveSampledPresence),
            b"step-execution-receipt" => Some(Self::StepExecutionReceipt),
            b"fixed-cadence-step-receipt" => Some(Self::FixedCadenceStepReceipt),
            b"timed-sampled-transition" => Some(Self::TimedSampledPresenceTransition),
            _ => None,
        }
    }
}

/// Framing-level parse/encode failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhysicsEvidenceFrameV1Error {
    PayloadTooLarge { len: usize },
    TruncatedMagic,
    WrongMagic,
    MissingKindTerminator,
    UnknownKind,
    TruncatedHeader,
    UnsupportedVersion { version: u32 },
    DeclaredPayloadTooLarge { len: u64 },
    TruncatedPayload { declared: u64, available: usize },
    TrailingBytes { trailing: usize },
}

/// Borrowed view of one canonically framed payload.
///
/// This type intentionally exposes only framing-level kind and payload bytes.
/// It is not, and cannot be promoted by this module into, a live physics
/// evidence token.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DecodedPhysicsEvidenceFrameV1<'a> {
    kind: PhysicsEvidenceFrameKindV1,
    payload: &'a [u8],
}

impl<'a> DecodedPhysicsEvidenceFrameV1<'a> {
    pub const fn kind(&self) -> PhysicsEvidenceFrameKindV1 {
        self.kind
    }

    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

/// Frame arbitrary already-produced payload bytes under the frozen v1 physics
/// application namespace and kind domain.
///
/// This is a representation primitive only. The caller is responsible for
/// obtaining payload bytes from a separately qualified semantic codec.
pub fn encode_physics_evidence_frame_v1(
    kind: PhysicsEvidenceFrameKindV1,
    payload: &[u8],
) -> Result<Vec<u8>, PhysicsEvidenceFrameV1Error> {
    if payload.len() > PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN {
        return Err(PhysicsEvidenceFrameV1Error::PayloadTooLarge { len: payload.len() });
    }

    let domain = kind.domain_bytes();
    debug_assert!(domain.len() <= PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN);

    let capacity = PHYSICS_EVIDENCE_FRAME_V1_MAGIC.len()
        + domain.len()
        + 1
        + VERSION_BYTES
        + LENGTH_BYTES
        + payload.len();
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(PHYSICS_EVIDENCE_FRAME_V1_MAGIC);
    frame.extend_from_slice(domain);
    frame.push(0);
    frame.extend_from_slice(&PHYSICS_EVIDENCE_FRAME_V1_VERSION.to_be_bytes());
    frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Parse one exact v1 frame without allocating from attacker-declared lengths.
///
/// The returned payload borrows the input frame. Unknown kinds, unsupported
/// versions, oversized lengths, truncation, and trailing aliases fail closed.
pub fn decode_physics_evidence_frame_v1(
    frame: &[u8],
) -> Result<DecodedPhysicsEvidenceFrameV1<'_>, PhysicsEvidenceFrameV1Error> {
    if frame.len() < PHYSICS_EVIDENCE_FRAME_V1_MAGIC.len() {
        return Err(PhysicsEvidenceFrameV1Error::TruncatedMagic);
    }
    if !frame.starts_with(PHYSICS_EVIDENCE_FRAME_V1_MAGIC) {
        return Err(PhysicsEvidenceFrameV1Error::WrongMagic);
    }

    let domain_start = PHYSICS_EVIDENCE_FRAME_V1_MAGIC.len();
    let domain_search_end = frame.len().min(
        domain_start + PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN + 1,
    );
    let domain_tail = &frame[domain_start..domain_search_end];
    let domain_len = domain_tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(PhysicsEvidenceFrameV1Error::MissingKindTerminator)?;
    let domain_end = domain_start + domain_len;
    let kind = PhysicsEvidenceFrameKindV1::from_domain_bytes(&frame[domain_start..domain_end])
        .ok_or(PhysicsEvidenceFrameV1Error::UnknownKind)?;

    let mut cursor = domain_end + 1;
    let fixed_end = cursor + VERSION_BYTES + LENGTH_BYTES;
    if frame.len() < fixed_end {
        return Err(PhysicsEvidenceFrameV1Error::TruncatedHeader);
    }

    let version = u32::from_be_bytes(
        frame[cursor..cursor + VERSION_BYTES]
            .try_into()
            .map_err(|_| PhysicsEvidenceFrameV1Error::TruncatedHeader)?,
    );
    cursor += VERSION_BYTES;
    if version != PHYSICS_EVIDENCE_FRAME_V1_VERSION {
        return Err(PhysicsEvidenceFrameV1Error::UnsupportedVersion { version });
    }

    let declared_len = u64::from_be_bytes(
        frame[cursor..cursor + LENGTH_BYTES]
            .try_into()
            .map_err(|_| PhysicsEvidenceFrameV1Error::TruncatedHeader)?,
    );
    cursor += LENGTH_BYTES;

    if declared_len > PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN as u64 {
        return Err(PhysicsEvidenceFrameV1Error::DeclaredPayloadTooLarge {
            len: declared_len,
        });
    }
    let payload_len = declared_len as usize;
    let payload_end = cursor + payload_len;
    if frame.len() < payload_end {
        return Err(PhysicsEvidenceFrameV1Error::TruncatedPayload {
            declared: declared_len,
            available: frame.len().saturating_sub(cursor),
        });
    }
    if frame.len() > payload_end {
        return Err(PhysicsEvidenceFrameV1Error::TrailingBytes {
            trailing: frame.len() - payload_end,
        });
    }

    Ok(DecodedPhysicsEvidenceFrameV1 {
        kind,
        payload: &frame[cursor..payload_end],
    })
}
