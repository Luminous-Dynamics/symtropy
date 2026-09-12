// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Content-bound immutable design revision contracts for Symtropy.
//!
//! This crate owns exact design identity and revision lineage only. It does not
//! evaluate engineering constraints, execute solvers, create physical matter,
//! plan fabrication, commission devices, or confer legal/economic authority.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

pub const DESIGN_REVISION_SCHEMA_VERSION: u32 = 1;
const DESIGN_REVISION_DIGEST_DOMAIN: &[u8] = b"symtropy.design.revision.v1\0";
const SHA256_ALGORITHM_ID: &str = "sha256";

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(StableId);

        impl $name {
            pub const fn new(id: StableId) -> Self {
                Self(id)
            }

            pub const fn stable_id(&self) -> &StableId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id_type!(DesignId);
stable_id_type!(DesignArtifactId);

/// Portable content digest used by exact design references.
///
/// The algorithm is explicit so external artifacts are not forced to use the
/// same hash algorithm as the internal design-manifest digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentDigest {
    pub algorithm: StableId,
    pub value: String,
}

impl ContentDigest {
    pub fn new(algorithm: StableId, value: impl Into<String>) -> Result<Self, DesignError> {
        let digest = Self {
            algorithm,
            value: value.into(),
        };
        digest.validate()?;
        Ok(digest)
    }

    pub fn validate(&self) -> Result<(), DesignError> {
        validate_stable_id(&self.algorithm)?;
        let valid = !self.value.is_empty()
            && self.value.len() <= 256
            && self.value.bytes().all(|byte| byte.is_ascii_graphic());
        if valid {
            Ok(())
        } else {
            Err(DesignError::InvalidDigestValue(self.value.clone()))
        }
    }

    fn sha256(bytes: &[u8]) -> Self {
        let value = hex(&Sha256::digest(bytes));
        Self {
            algorithm: StableId::parse(SHA256_ALGORITHM_ID)
                .expect("sha256 is a valid stable identifier literal"),
            value,
        }
    }
}

/// Exact immutable reference to one design revision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DesignRevisionRef {
    pub design_id: DesignId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

impl DesignRevisionRef {
    pub fn new(
        design_id: DesignId,
        revision: u64,
        content_digest: ContentDigest,
    ) -> Result<Self, DesignError> {
        let reference = Self {
            design_id,
            revision,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), DesignError> {
        validate_stable_id(self.design_id.stable_id())?;
        self.content_digest.validate()
    }
}

/// Exact reference to an artifact used by a design revision.
///
/// Storage location is deliberately absent. A Git path, object-store URL or
/// DHT locator may move without changing artifact identity; changing artifact
/// bytes must change `content_digest`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DesignArtifactRef {
    pub id: DesignArtifactId,
    pub role_id: StableId,
    pub content_digest: ContentDigest,
}

impl DesignArtifactRef {
    pub fn new(
        id: DesignArtifactId,
        role_id: StableId,
        content_digest: ContentDigest,
    ) -> Result<Self, DesignError> {
        let reference = Self {
            id,
            role_id,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), DesignError> {
        validate_stable_id(self.id.stable_id())?;
        validate_stable_id(&self.role_id)?;
        self.content_digest.validate()
    }
}

/// Generic exact reference to design semantics owned by another layer.
///
/// D1 intentionally does not define requirements, systems decomposition or
/// verification predicates. It binds their exact identities so later tranches
/// can add those schemas without changing the revision-digest theorem.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DesignSemanticRef {
    pub kind_id: StableId,
    pub subject_id: StableId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

impl DesignSemanticRef {
    pub fn new(
        kind_id: StableId,
        subject_id: StableId,
        revision: u64,
        content_digest: ContentDigest,
    ) -> Result<Self, DesignError> {
        let reference = Self {
            kind_id,
            subject_id,
            revision,
            content_digest,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), DesignError> {
        validate_stable_id(&self.kind_id)?;
        validate_stable_id(&self.subject_id)?;
        self.content_digest.validate()
    }
}

/// Canonical semantic snapshot for one immutable design revision.
///
/// The vectors are stored in canonical order. This makes manifest identity
/// independent of caller insertion order while rejecting ambiguous duplicate
/// identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignRevisionManifest {
    pub schema_version: u32,
    pub design_id: DesignId,
    pub revision: u64,
    parents: Vec<DesignRevisionRef>,
    artifacts: Vec<DesignArtifactRef>,
    semantic_refs: Vec<DesignSemanticRef>,
}

impl DesignRevisionManifest {
    pub fn new(
        design_id: DesignId,
        revision: u64,
        mut parents: Vec<DesignRevisionRef>,
        mut artifacts: Vec<DesignArtifactRef>,
        mut semantic_refs: Vec<DesignSemanticRef>,
    ) -> Result<Self, DesignError> {
        parents.sort_by(compare_parent_identity);
        artifacts.sort_by(|left, right| left.id.cmp(&right.id));
        semantic_refs.sort_by(compare_semantic_identity);

        let manifest = Self {
            schema_version: DESIGN_REVISION_SCHEMA_VERSION,
            design_id,
            revision,
            parents,
            artifacts,
            semantic_refs,
        };
        manifest.validate_canonical()?;
        Ok(manifest)
    }

    pub fn parents(&self) -> &[DesignRevisionRef] {
        &self.parents
    }

    pub fn artifacts(&self) -> &[DesignArtifactRef] {
        &self.artifacts
    }

    pub fn semantic_refs(&self) -> &[DesignSemanticRef] {
        &self.semantic_refs
    }

    fn validate_canonical(&self) -> Result<(), DesignError> {
        if self.schema_version != DESIGN_REVISION_SCHEMA_VERSION {
            return Err(DesignError::UnsupportedSchemaVersion(self.schema_version));
        }
        validate_stable_id(self.design_id.stable_id())?;
        if self.artifacts.is_empty() && self.semantic_refs.is_empty() {
            return Err(DesignError::ContentRequired);
        }

        for parent in &self.parents {
            parent.validate()?;
            if parent.design_id == self.design_id && parent.revision >= self.revision {
                return Err(DesignError::NonPriorParent {
                    design_id: parent.design_id.clone(),
                    parent_revision: parent.revision,
                    child_revision: self.revision,
                });
            }
        }
        for pair in self.parents.windows(2) {
            match compare_parent_identity(&pair[0], &pair[1]) {
                std::cmp::Ordering::Greater => {
                    return Err(DesignError::NonCanonicalOrder("parents"));
                }
                std::cmp::Ordering::Equal => {
                    return Err(DesignError::DuplicateParentIdentity {
                        design_id: pair[0].design_id.clone(),
                        revision: pair[0].revision,
                    });
                }
                std::cmp::Ordering::Less => {}
            }
        }

        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        for pair in self.artifacts.windows(2) {
            if pair[0].id > pair[1].id {
                return Err(DesignError::NonCanonicalOrder("artifacts"));
            }
            if pair[0].id == pair[1].id {
                return Err(DesignError::DuplicateArtifact(pair[0].id.clone()));
            }
        }

        for semantic in &self.semantic_refs {
            semantic.validate()?;
        }
        for pair in self.semantic_refs.windows(2) {
            match compare_semantic_identity(&pair[0], &pair[1]) {
                std::cmp::Ordering::Greater => {
                    return Err(DesignError::NonCanonicalOrder("semantic_refs"));
                }
                std::cmp::Ordering::Equal => {
                    return Err(DesignError::DuplicateSemanticIdentity {
                        kind_id: pair[0].kind_id.clone(),
                        subject_id: pair[0].subject_id.clone(),
                        revision: pair[0].revision,
                    });
                }
                std::cmp::Ordering::Less => {}
            }
        }

        Ok(())
    }

    fn content_digest(&self) -> Result<ContentDigest, DesignError> {
        Ok(ContentDigest::sha256(&self.canonical_preimage()?))
    }

    /// Stable digest protocol for design revision identity.
    ///
    /// This preimage is deliberately independent of Serde/JSON representation:
    /// domain tag + schema version + fixed-width integers + length-prefixed
    /// strings/collections in constructor-canonical order.
    fn canonical_preimage(&self) -> Result<Vec<u8>, DesignError> {
        self.validate_canonical()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(DESIGN_REVISION_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.design_id.stable_id())?;
        bytes.extend_from_slice(&self.revision.to_le_bytes());

        encode_len(&mut bytes, self.parents.len())?;
        for parent in &self.parents {
            encode_stable_id(&mut bytes, parent.design_id.stable_id())?;
            bytes.extend_from_slice(&parent.revision.to_le_bytes());
            encode_digest(&mut bytes, &parent.content_digest)?;
        }

        encode_len(&mut bytes, self.artifacts.len())?;
        for artifact in &self.artifacts {
            encode_stable_id(&mut bytes, artifact.id.stable_id())?;
            encode_stable_id(&mut bytes, &artifact.role_id)?;
            encode_digest(&mut bytes, &artifact.content_digest)?;
        }

        encode_len(&mut bytes, self.semantic_refs.len())?;
        for semantic in &self.semantic_refs {
            encode_stable_id(&mut bytes, &semantic.kind_id)?;
            encode_stable_id(&mut bytes, &semantic.subject_id)?;
            bytes.extend_from_slice(&semantic.revision.to_le_bytes());
            encode_digest(&mut bytes, &semantic.content_digest)?;
        }

        Ok(bytes)
    }
}

/// Sealed immutable design revision.
///
/// Deserialization revalidates canonical ordering and recomputes the digest, so
/// changing semantic content without changing the exact reference fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignRevision {
    manifest: DesignRevisionManifest,
    content_digest: ContentDigest,
}

impl DesignRevision {
    pub fn seal(manifest: DesignRevisionManifest) -> Result<Self, DesignError> {
        let content_digest = manifest.content_digest()?;
        Ok(Self {
            manifest,
            content_digest,
        })
    }

    pub fn manifest(&self) -> &DesignRevisionManifest {
        &self.manifest
    }

    pub fn content_digest(&self) -> &ContentDigest {
        &self.content_digest
    }

    pub fn exact_ref(&self) -> DesignRevisionRef {
        DesignRevisionRef {
            design_id: self.manifest.design_id.clone(),
            revision: self.manifest.revision,
            content_digest: self.content_digest.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), DesignError> {
        let actual = self.manifest.content_digest()?;
        if actual == self.content_digest {
            Ok(())
        } else {
            Err(DesignError::DigestMismatch {
                expected: self.content_digest.clone(),
                actual,
            })
        }
    }
}

#[derive(Deserialize)]
struct DesignRevisionWire {
    manifest: DesignRevisionManifest,
    content_digest: ContentDigest,
}

impl<'de> Deserialize<'de> for DesignRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DesignRevisionWire::deserialize(deserializer)?;
        let revision = Self {
            manifest: wire.manifest,
            content_digest: wire.content_digest,
        };
        revision.validate().map_err(serde::de::Error::custom)?;
        Ok(revision)
    }
}

fn compare_parent_identity(
    left: &DesignRevisionRef,
    right: &DesignRevisionRef,
) -> std::cmp::Ordering {
    (&left.design_id, left.revision).cmp(&(&right.design_id, right.revision))
}

fn compare_semantic_identity(
    left: &DesignSemanticRef,
    right: &DesignSemanticRef,
) -> std::cmp::Ordering {
    (&left.kind_id, &left.subject_id, left.revision).cmp(&(
        &right.kind_id,
        &right.subject_id,
        right.revision,
    ))
}

fn encode_digest(bytes: &mut Vec<u8>, digest: &ContentDigest) -> Result<(), DesignError> {
    digest.validate()?;
    encode_stable_id(bytes, &digest.algorithm)?;
    encode_string(bytes, &digest.value)
}

fn encode_stable_id(bytes: &mut Vec<u8>, id: &StableId) -> Result<(), DesignError> {
    validate_stable_id(id)?;
    encode_string(bytes, id.as_str())
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), DesignError> {
    encode_len(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), DesignError> {
    let len = u64::try_from(len).map_err(|_| DesignError::LengthOverflow)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn validate_stable_id(id: &StableId) -> Result<(), DesignError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| DesignError::InvalidStableId(id.as_str().to_string()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug)]
pub enum DesignError {
    InvalidStableId(String),
    InvalidDigestValue(String),
    UnsupportedSchemaVersion(u32),
    LengthOverflow,
    ContentRequired,
    DuplicateParentIdentity {
        design_id: DesignId,
        revision: u64,
    },
    NonPriorParent {
        design_id: DesignId,
        parent_revision: u64,
        child_revision: u64,
    },
    DuplicateArtifact(DesignArtifactId),
    DuplicateSemanticIdentity {
        kind_id: StableId,
        subject_id: StableId,
        revision: u64,
    },
    NonCanonicalOrder(&'static str),
    DigestMismatch {
        expected: ContentDigest,
        actual: ContentDigest,
    },
}

impl fmt::Display for DesignError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId(id) => write!(formatter, "invalid stable identifier: {id}"),
            Self::InvalidDigestValue(value) => write!(
                formatter,
                "content digest value must contain 1..=256 printable non-whitespace ASCII bytes, got {} bytes",
                value.len()
            ),
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "unsupported design revision schema version {version}"
                )
            }
            Self::LengthOverflow => write!(formatter, "canonical design length exceeds u64"),
            Self::ContentRequired => write!(
                formatter,
                "design revision must contain at least one artifact or semantic reference"
            ),
            Self::DuplicateParentIdentity {
                design_id,
                revision,
            } => write!(
                formatter,
                "duplicate parent identity {design_id} revision {revision}"
            ),
            Self::NonPriorParent {
                design_id,
                parent_revision,
                child_revision,
            } => write!(
                formatter,
                "same-design parent {design_id} revision {parent_revision} is not prior to child revision {child_revision}"
            ),
            Self::DuplicateArtifact(id) => {
                write!(formatter, "duplicate design artifact identity {id}")
            }
            Self::DuplicateSemanticIdentity {
                kind_id,
                subject_id,
                revision,
            } => write!(
                formatter,
                "duplicate semantic identity {kind_id}/{subject_id} revision {revision}"
            ),
            Self::NonCanonicalOrder(field) => {
                write!(
                    formatter,
                    "design manifest field {field} is not canonically ordered"
                )
            }
            Self::DigestMismatch { expected, actual } => write!(
                formatter,
                "design digest mismatch: expected {}:{}, actual {}:{}",
                expected.algorithm, expected.value, actual.algorithm, actual.value
            ),
        }
    }
}

impl Error for DesignError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn digest(value: &str) -> ContentDigest {
        ContentDigest::new(id("blake3"), value).unwrap()
    }

    fn artifact(name: &str, role: &str, content: &str) -> DesignArtifactRef {
        DesignArtifactRef::new(DesignArtifactId::new(id(name)), id(role), digest(content)).unwrap()
    }

    fn semantic(kind: &str, subject: &str, revision: u64, content: &str) -> DesignSemanticRef {
        DesignSemanticRef::new(id(kind), id(subject), revision, digest(content)).unwrap()
    }

    fn revision_with_order(artifacts: Vec<DesignArtifactRef>) -> DesignRevision {
        let manifest = DesignRevisionManifest::new(
            DesignId::new(id("design:bracket")),
            1,
            Vec::new(),
            artifacts,
            vec![semantic(
                "design-semantic:requirement-set",
                "requirements:bracket",
                1,
                "bbbb",
            )],
        )
        .unwrap();
        DesignRevision::seal(manifest).unwrap()
    }

    #[test]
    fn caller_insertion_order_does_not_change_revision_digest() {
        let a = artifact("artifact:a", "design-artifact:geometry", "aaaa");
        let b = artifact("artifact:b", "design-artifact:controller", "cccc");
        let left = revision_with_order(vec![a.clone(), b.clone()]);
        let right = revision_with_order(vec![b, a]);
        assert_eq!(left, right);
        assert_eq!(left.content_digest(), right.content_digest());
    }

    #[test]
    fn changed_artifact_content_changes_exact_revision_identity() {
        let left = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);
        let right = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "zzzz",
        )]);
        assert_eq!(left.manifest().design_id, right.manifest().design_id);
        assert_eq!(left.manifest().revision, right.manifest().revision);
        assert_ne!(left.content_digest(), right.content_digest());
        assert_ne!(left.exact_ref(), right.exact_ref());
    }

    #[test]
    fn duplicate_artifact_identity_is_rejected_even_when_digests_differ() {
        let result = DesignRevisionManifest::new(
            DesignId::new(id("design:bracket")),
            1,
            Vec::new(),
            vec![
                artifact("artifact:a", "design-artifact:geometry", "aaaa"),
                artifact("artifact:a", "design-artifact:geometry", "bbbb"),
            ],
            Vec::new(),
        );
        assert!(matches!(result, Err(DesignError::DuplicateArtifact(_))));
    }

    #[test]
    fn duplicate_semantic_identity_is_rejected_even_when_digests_differ() {
        let result = DesignRevisionManifest::new(
            DesignId::new(id("design:bracket")),
            1,
            Vec::new(),
            vec![artifact("artifact:a", "design-artifact:geometry", "aaaa")],
            vec![
                semantic("design-semantic:requirements", "requirements:a", 1, "aaaa"),
                semantic("design-semantic:requirements", "requirements:a", 1, "bbbb"),
            ],
        );
        assert!(matches!(
            result,
            Err(DesignError::DuplicateSemanticIdentity { .. })
        ));
    }

    #[test]
    fn same_design_parent_must_be_strictly_prior_revision() {
        let parent =
            DesignRevisionRef::new(DesignId::new(id("design:bracket")), 2, digest("aaaa")).unwrap();
        let result = DesignRevisionManifest::new(
            DesignId::new(id("design:bracket")),
            2,
            vec![parent],
            vec![artifact("artifact:a", "design-artifact:geometry", "aaaa")],
            Vec::new(),
        );
        assert!(matches!(result, Err(DesignError::NonPriorParent { .. })));
    }

    #[test]
    fn empty_revision_content_is_rejected() {
        let result = DesignRevisionManifest::new(
            DesignId::new(id("design:empty")),
            0,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert!(matches!(result, Err(DesignError::ContentRequired)));
    }

    #[test]
    fn serialized_content_tampering_fails_closed() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);
        let mut value = serde_json::to_value(&revision).unwrap();
        value["manifest"]["artifacts"][0]["content_digest"]["value"] =
            serde_json::Value::String("tampered".into());
        let decoded = serde_json::from_value::<DesignRevision>(value);
        assert!(decoded.is_err());
    }

    #[test]
    fn serialized_digest_tampering_fails_closed() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);
        let mut value = serde_json::to_value(&revision).unwrap();
        value["content_digest"]["value"] = serde_json::Value::String("0".repeat(64));
        let decoded = serde_json::from_value::<DesignRevision>(value);
        assert!(decoded.is_err());
    }

    #[test]
    fn sealed_revision_round_trips_with_exact_identity() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);
        let encoded = serde_json::to_vec(&revision).unwrap();
        let decoded: DesignRevision = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, revision);
        assert_eq!(decoded.exact_ref(), revision.exact_ref());
    }

    #[test]
    fn canonical_digest_protocol_has_frozen_golden_vector() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);

        assert_eq!(
            revision.manifest().schema_version,
            DESIGN_REVISION_SCHEMA_VERSION
        );
        assert_eq!(revision.content_digest().algorithm, id("sha256"));
        assert_eq!(
            revision.content_digest().value,
            "3754b633817fd02c8d731793a8f5b76debd8e3c0791f01f77782c1f216ff4045"
        );
    }

    #[test]
    fn schema_version_is_digest_semantics_and_wire_tampering_fails_closed() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);
        let mut value = serde_json::to_value(&revision).unwrap();
        value["manifest"]["schema_version"] = 2.into();
        let decoded = serde_json::from_value::<DesignRevision>(value);
        assert!(matches!(
            decoded.unwrap_err().to_string().as_str(),
            message if message.contains("unsupported design revision schema version 2")
        ));
    }

    #[test]
    fn invalid_external_digest_is_rejected() {
        assert!(matches!(
            ContentDigest::new(id("blake3"), "contains whitespace"),
            Err(DesignError::InvalidDigestValue(_))
        ));
    }
}
