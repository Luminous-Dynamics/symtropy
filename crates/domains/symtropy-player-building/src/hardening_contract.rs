// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! PB-01 hardening contracts discovered in #439.
//!
//! This module is intentionally authority-free. It closes proposal semantics
//! needed before PB-02 may cross into Construction/Fabrication/matter authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symtropy_game_state::StableId;

/// Exact subject named by authored intent before a planner may modify/remove it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntentTargetRef {
    pub authority_id: StableId,
    pub subject_id: StableId,
    pub revision: u64,
    pub digest_algorithm: StableId,
    pub digest_value: String,
}

impl IntentTargetRef {
    /// Content-addressed key. Digest is part of equality so changed physical
    /// state is a different exact target rather than an implicit update.
    pub fn canonical_key(&self) -> (&StableId, &StableId, u64, &StableId, &str) {
        (
            &self.authority_id,
            &self.subject_id,
            self.revision,
            &self.digest_algorithm,
            self.digest_value.as_str(),
        )
    }
}

/// Exact content-addressed construction-intent parent.
///
/// Unlike `(intent_id, revision)` identity, the digest is deliberately part of
/// the ordering/equality key. This permits two concurrent revision-2 forks to
/// be explicit parents of revision 3 while exact duplicate parents still fail.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentAddressedIntentParent {
    pub intent_id: StableId,
    pub revision: u64,
    pub digest_algorithm: StableId,
    pub digest_value: String,
}

impl ContentAddressedIntentParent {
    pub fn validate_for_child(
        &self,
        child_intent_id: &StableId,
        child_revision: u64,
    ) -> Result<(), HardeningError> {
        if &self.intent_id != child_intent_id {
            return Err(HardeningError::ForeignParent);
        }
        if self.revision >= child_revision {
            return Err(HardeningError::NonPriorParent);
        }
        if self.digest_value.is_empty() || self.digest_value.len() > 256 {
            return Err(HardeningError::InvalidDigest);
        }
        Ok(())
    }
}

/// PB-01 rotation identity is *authored parameter identity*.
///
/// Distinct intrinsic XYZ turn32 tuples remain distinct proposal identities
/// even when a downstream geometry/physics layer can show that the resulting
/// rigid transforms are physically equivalent near a singularity. CAD/snap
/// adapters must choose and record the tuple they author; PB-01 does not hide a
/// second transform canonicalizer inside proposal identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationIdentityPolicy {
    AuthoredIntrinsicXyzTurn32,
}

pub const ROTATION_IDENTITY_POLICY: RotationIdentityPolicy =
    RotationIdentityPolicy::AuthoredIntrinsicXyzTurn32;

/// Versioned hostile-ingress profile. This caps encoded allocation exposure
/// before Serde or another decoder sees the frame.
pub const PB01_JSON_INGRESS_PROFILE_ID: &str = "proposal-ingress:pb01-json-v1";
pub const PB01_JSON_MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

pub fn check_untrusted_frame(frame: &[u8]) -> Result<(), HardeningError> {
    if frame.len() > PB01_JSON_MAX_FRAME_BYTES {
        Err(HardeningError::IngressFrameTooLarge {
            actual: frame.len(),
            max: PB01_JSON_MAX_FRAME_BYTES,
        })
    } else {
        Ok(())
    }
}

/// Canonical identity writer used by the hardened PB-01 identity theorem.
///
/// Every primitive is fixed-width little-endian or length-prefixed bytes.
/// Wire JSON is not part of this identity.
#[derive(Debug, Default)]
pub struct CanonicalIdentityWriter {
    bytes: Vec<u8>,
}

impl CanonicalIdentityWriter {
    pub fn domain(domain: &[u8]) -> Self {
        Self { bytes: domain.to_vec() }
    }

    pub fn put_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn put_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn put_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn put_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn put_len(&mut self, value: usize) -> Result<(), HardeningError> {
        let value = u32::try_from(value).map_err(|_| HardeningError::LengthOverflow)?;
        self.put_u32(value);
        Ok(())
    }

    pub fn put_bytes(&mut self, value: &[u8]) -> Result<(), HardeningError> {
        self.put_len(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    pub fn put_text(&mut self, value: &str) -> Result<(), HardeningError> {
        self.put_bytes(value.as_bytes())
    }

    pub fn put_stable_id(&mut self, value: &StableId) -> Result<(), HardeningError> {
        self.put_text(value.as_str())
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }

    pub fn sha256(self) -> String {
        let digest = Sha256::digest(self.bytes);
        let mut out = String::with_capacity(64);
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in digest {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardeningError {
    ForeignParent,
    NonPriorParent,
    InvalidDigest,
    IngressFrameTooLarge { actual: usize, max: usize },
    LengthOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn parent(digest: &str) -> ContentAddressedIntentParent {
        ContentAddressedIntentParent {
            intent_id: id("build-intent:shelter"),
            revision: 2,
            digest_algorithm: id("sha256"),
            digest_value: digest.to_owned(),
        }
    }

    #[test]
    fn concurrent_same_revision_parents_remain_distinct() {
        let a = parent("fork-a");
        let b = parent("fork-b");
        assert_ne!(a, b);
        assert!(a.validate_for_child(&id("build-intent:shelter"), 3).is_ok());
        assert!(b.validate_for_child(&id("build-intent:shelter"), 3).is_ok());

        let mut left = vec![a.clone(), b.clone()];
        let mut right = vec![b, a];
        left.sort();
        right.sort();
        assert_eq!(left, right);
    }

    #[test]
    fn exact_target_digest_is_part_of_target_identity() {
        let a = IntentTargetRef {
            authority_id: id("authority:construction"),
            subject_id: id("structure:shelter"),
            revision: 7,
            digest_algorithm: id("sha256"),
            digest_value: "state-a".into(),
        };
        let mut b = a.clone();
        b.digest_value = "state-b".into();
        assert_ne!(a, b);
        assert_ne!(a.canonical_key(), b.canonical_key());
    }

    #[test]
    fn hostile_frame_limit_is_checked_before_decode() {
        assert!(check_untrusted_frame(&vec![0; PB01_JSON_MAX_FRAME_BYTES]).is_ok());
        assert!(matches!(
            check_untrusted_frame(&vec![0; PB01_JSON_MAX_FRAME_BYTES + 1]),
            Err(HardeningError::IngressFrameTooLarge { .. })
        ));
    }

    #[test]
    fn rotation_policy_is_explicitly_authored_parameter_identity() {
        assert_eq!(
            ROTATION_IDENTITY_POLICY,
            RotationIdentityPolicy::AuthoredIntrinsicXyzTurn32
        );
    }

    #[test]
    fn canonical_writer_has_stable_external_golden_vector() {
        let mut writer = CanonicalIdentityWriter::domain(b"pb01.golden.v1\0");
        writer.put_u32(1);
        writer.put_stable_id(&id("element:a")).unwrap();
        writer.put_i64(-7);
        writer.put_u64(42);
        writer.put_u8(3);
        assert_eq!(
            writer.sha256(),
            "9ce9d4ab3e79b59004fb42de207a2a792db89031106501ce76a6208622974fd1"
        );
    }
}
