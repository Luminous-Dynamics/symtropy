// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic semantic scene identity for Bevy-hosted art worlds.
//!
//! Bevy `Entity` values are runtime-local identifiers and are therefore not
//! suitable as persistent artistic identity or provenance keys. This module
//! gives explicitly tagged artistic entities stable IDs and defines a small,
//! pure scene-record representation whose digest is independent of query/order
//! iteration.

use bevy::prelude::*;
use std::collections::BTreeSet;

use crate::art_port::ArtPerceptionFrame;

#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ArtEntityId(pub String);

impl ArtEntityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ArtEntitySemantics {
    pub kind: String,
    pub material_id: Option<String>,
    pub visible: bool,
}

impl ArtEntitySemantics {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            material_id: None,
            visible: true,
        }
    }
}

/// Pure semantic record used to hash a committed art scene. It intentionally
/// avoids Bevy `Entity` and asset-handle internals.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtSceneRecord {
    pub stable_id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub material_id: Option<String>,
    pub translation: [f32; 3],
    pub rotation_xyzw: [f32; 4],
    pub scale: [f32; 3],
    pub visible: bool,
}

impl ArtSceneRecord {
    pub fn from_transform(
        id: &ArtEntityId,
        semantics: &ArtEntitySemantics,
        parent_id: Option<&ArtEntityId>,
        transform: &Transform,
    ) -> Self {
        Self {
            stable_id: id.0.clone(),
            parent_id: parent_id.map(|id| id.0.clone()),
            kind: semantics.kind.clone(),
            material_id: semantics.material_id.clone(),
            translation: transform.translation.to_array(),
            rotation_xyzw: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
            visible: semantics.visible,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtSceneError {
    DuplicateStableId(String),
    EmptyStableId,
    NonFiniteTransform(String),
}

impl std::fmt::Display for ArtSceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateStableId(id) => write!(f, "duplicate stable art entity id: {id}"),
            Self::EmptyStableId => write!(f, "stable art entity id may not be empty"),
            Self::NonFiniteTransform(id) => {
                write!(f, "art entity {id} contains a non-finite transform")
            }
        }
    }
}

impl std::error::Error for ArtSceneError {}

/// Order-independent, deterministic digest of semantic scene records.
///
/// FNV-1a is used as a small stable protocol hash rather than `DefaultHasher`,
/// whose algorithm is not a cross-version provenance contract. This is not a
/// cryptographic digest; callers needing tamper evidence should wrap the
/// canonical record bytes in the repository's cryptographic evidence layer.
pub fn stable_scene_hash(records: &[ArtSceneRecord]) -> Result<String, ArtSceneError> {
    let mut ordered: Vec<&ArtSceneRecord> = records.iter().collect();
    ordered.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));

    let mut seen = BTreeSet::new();
    let mut hash = Fnv1a64::new();
    hash.bytes(b"symthaea.bevy-art-scene.v1\0");
    hash.u64(ordered.len() as u64);

    for record in ordered {
        validate_record(record)?;
        if !seen.insert(record.stable_id.as_str()) {
            return Err(ArtSceneError::DuplicateStableId(record.stable_id.clone()));
        }

        hash.string(&record.stable_id);
        hash.optional_string(record.parent_id.as_deref());
        hash.string(&record.kind);
        hash.optional_string(record.material_id.as_deref());
        for value in record.translation {
            hash.f32(value);
        }
        for value in record.rotation_xyzw {
            hash.f32(value);
        }
        for value in record.scale {
            hash.f32(value);
        }
        hash.u8(u8::from(record.visible));
    }

    Ok(format!("{:016x}", hash.finish()))
}

/// Build the art-port perception envelope from a deterministic semantic scene.
pub fn perception_frame_from_records(
    world_id: impl Into<String>,
    revision_id: impl Into<String>,
    revision_sequence: u64,
    records: &[ArtSceneRecord],
) -> Result<ArtPerceptionFrame, ArtSceneError> {
    let digest = stable_scene_hash(records)?;
    let mut frame = ArtPerceptionFrame::new(world_id, revision_id, revision_sequence, digest);

    let mut ordered: Vec<&ArtSceneRecord> = records.iter().collect();
    ordered.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));
    frame.scene_summary = ordered
        .into_iter()
        .map(|record| {
            format!(
                "{}:{}:{}",
                record.stable_id,
                record.kind,
                if record.visible { "visible" } else { "hidden" }
            )
        })
        .collect();
    Ok(frame)
}

fn validate_record(record: &ArtSceneRecord) -> Result<(), ArtSceneError> {
    if record.stable_id.is_empty() {
        return Err(ArtSceneError::EmptyStableId);
    }
    let finite = record
        .translation
        .iter()
        .chain(record.rotation_xyzw.iter())
        .chain(record.scale.iter())
        .all(|value| value.is_finite());
    if !finite {
        return Err(ArtSceneError::NonFiniteTransform(record.stable_id.clone()));
    }
    Ok(())
}

struct Fnv1a64(u64);

impl Fnv1a64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn f32(&mut self, value: f32) {
        // Canonicalize signed zero so visually/physically equivalent zeroes do
        // not create different revisions.
        let canonical = if value == 0.0 { 0.0 } else { value };
        self.bytes(&canonical.to_bits().to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.u64(value.len() as u64);
        self.bytes(value.as_bytes());
    }

    fn optional_string(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.string(value);
            }
            None => self.u8(0),
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, x: f32) -> ArtSceneRecord {
        ArtSceneRecord {
            stable_id: id.into(),
            parent_id: None,
            kind: "form".into(),
            material_id: Some("clay".into()),
            translation: [x, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            visible: true,
        }
    }

    #[test]
    fn hash_is_independent_of_record_order() {
        let a = vec![record("b", 2.0), record("a", 1.0)];
        let b = vec![record("a", 1.0), record("b", 2.0)];
        assert_eq!(stable_scene_hash(&a).unwrap(), stable_scene_hash(&b).unwrap());
    }

    #[test]
    fn transform_change_advances_digest() {
        let a = vec![record("a", 1.0)];
        let b = vec![record("a", 1.1)];
        assert_ne!(stable_scene_hash(&a).unwrap(), stable_scene_hash(&b).unwrap());
    }

    #[test]
    fn duplicate_stable_identity_is_rejected() {
        let records = vec![record("a", 1.0), record("a", 2.0)];
        assert_eq!(
            stable_scene_hash(&records),
            Err(ArtSceneError::DuplicateStableId("a".into()))
        );
    }

    #[test]
    fn non_finite_transform_is_rejected_not_normalized() {
        let records = vec![record("a", f32::NAN)];
        assert_eq!(
            stable_scene_hash(&records),
            Err(ArtSceneError::NonFiniteTransform("a".into()))
        );
    }

    #[test]
    fn signed_zero_is_canonicalized() {
        let a = vec![record("a", 0.0)];
        let b = vec![record("a", -0.0)];
        assert_eq!(stable_scene_hash(&a).unwrap(), stable_scene_hash(&b).unwrap());
    }
}
