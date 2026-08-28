// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical isolated preview-scene substrate for counterfactual art branches.
//!
//! This module operates only on cloned semantic scene records. It owns no main
//! Bevy `World`, no `Commands`, and no committed revision mutator. That makes it
//! useful as the deterministic intermediate form between an artistic proposal
//! and a later rendered proposal ghost.

use std::collections::BTreeMap;

use crate::art_scene::{stable_scene_hash, ArtSceneError, ArtSceneRecord};

#[derive(Debug, Clone)]
pub struct IsolatedPreviewScene {
    branch_id: String,
    base_revision: String,
    base_scene_hash: String,
    records: BTreeMap<String, ArtSceneRecord>,
    applied_proposals: Vec<String>,
}

impl IsolatedPreviewScene {
    pub fn from_committed(
        branch_id: impl Into<String>,
        base_revision: impl Into<String>,
        base_scene_hash: impl Into<String>,
        committed_records: &[ArtSceneRecord],
    ) -> Result<Self, PreviewSceneError> {
        let branch_id = branch_id.into();
        let base_revision = base_revision.into();
        let base_scene_hash = base_scene_hash.into();
        if branch_id.trim().is_empty() {
            return Err(PreviewSceneError::EmptyBranchId);
        }
        if base_revision.trim().is_empty() || base_scene_hash.trim().is_empty() {
            return Err(PreviewSceneError::MissingBaseIdentity);
        }
        let actual = stable_scene_hash(committed_records).map_err(PreviewSceneError::Scene)?;
        if actual != base_scene_hash {
            return Err(PreviewSceneError::BaseHashMismatch {
                expected: base_scene_hash,
                actual,
            });
        }

        let mut records = BTreeMap::new();
        for record in committed_records {
            if records.insert(record.stable_id.clone(), record.clone()).is_some() {
                return Err(PreviewSceneError::DuplicateStableId(record.stable_id.clone()));
            }
        }

        Ok(Self {
            branch_id,
            base_revision,
            base_scene_hash,
            records,
            applied_proposals: Vec::new(),
        })
    }

    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }

    pub fn base_revision(&self) -> &str {
        &self.base_revision
    }

    pub fn base_scene_hash(&self) -> &str {
        &self.base_scene_hash
    }

    pub fn applied_proposals(&self) -> &[String] {
        &self.applied_proposals
    }

    pub fn records(&self) -> impl Iterator<Item = &ArtSceneRecord> {
        self.records.values()
    }

    /// Apply a preview-only transform replacement to one stable entity.
    pub fn preview_transform(
        &mut self,
        proposal_id: impl Into<String>,
        stable_id: &str,
        translation: [f32; 3],
        rotation_xyzw: [f32; 4],
        scale: [f32; 3],
    ) -> Result<(), PreviewSceneError> {
        validate_finite_transform(stable_id, &translation, &rotation_xyzw, &scale)?;
        let record = self
            .records
            .get_mut(stable_id)
            .ok_or_else(|| PreviewSceneError::UnknownStableId(stable_id.to_string()))?;
        record.translation = translation;
        record.rotation_xyzw = rotation_xyzw;
        record.scale = scale;
        self.record_proposal(proposal_id)
    }

    /// Preview visibility without mutating the committed scene.
    pub fn preview_visibility(
        &mut self,
        proposal_id: impl Into<String>,
        stable_id: &str,
        visible: bool,
    ) -> Result<(), PreviewSceneError> {
        let record = self
            .records
            .get_mut(stable_id)
            .ok_or_else(|| PreviewSceneError::UnknownStableId(stable_id.to_string()))?;
        record.visible = visible;
        self.record_proposal(proposal_id)
    }

    /// Preview material identity without touching the main-world asset graph.
    pub fn preview_material(
        &mut self,
        proposal_id: impl Into<String>,
        stable_id: &str,
        material_id: Option<String>,
    ) -> Result<(), PreviewSceneError> {
        let record = self
            .records
            .get_mut(stable_id)
            .ok_or_else(|| PreviewSceneError::UnknownStableId(stable_id.to_string()))?;
        record.material_id = material_id;
        self.record_proposal(proposal_id)
    }

    pub fn preview_scene_hash(&self) -> Result<String, PreviewSceneError> {
        let records: Vec<_> = self.records.values().cloned().collect();
        stable_scene_hash(&records).map_err(PreviewSceneError::Scene)
    }

    /// Prove that a caller-supplied committed scene still has the exact base
    /// digest after arbitrary preview edits.
    pub fn verify_committed_unchanged(
        &self,
        committed_records: &[ArtSceneRecord],
    ) -> Result<(), PreviewSceneError> {
        let actual = stable_scene_hash(committed_records).map_err(PreviewSceneError::Scene)?;
        if actual != self.base_scene_hash {
            return Err(PreviewSceneError::CommittedSceneChanged {
                expected: self.base_scene_hash.clone(),
                actual,
            });
        }
        Ok(())
    }

    fn record_proposal(&mut self, proposal_id: impl Into<String>) -> Result<(), PreviewSceneError> {
        let proposal_id = proposal_id.into();
        if proposal_id.trim().is_empty() {
            return Err(PreviewSceneError::EmptyProposalId);
        }
        if self.applied_proposals.iter().any(|id| id == &proposal_id) {
            return Err(PreviewSceneError::DuplicateProposal(proposal_id));
        }
        self.applied_proposals.push(proposal_id);
        Ok(())
    }
}

fn validate_finite_transform(
    stable_id: &str,
    translation: &[f32; 3],
    rotation_xyzw: &[f32; 4],
    scale: &[f32; 3],
) -> Result<(), PreviewSceneError> {
    if translation
        .iter()
        .chain(rotation_xyzw.iter())
        .chain(scale.iter())
        .any(|value| !value.is_finite())
    {
        return Err(PreviewSceneError::NonFiniteTransform(stable_id.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewSceneError {
    Scene(ArtSceneError),
    EmptyBranchId,
    MissingBaseIdentity,
    BaseHashMismatch { expected: String, actual: String },
    DuplicateStableId(String),
    UnknownStableId(String),
    EmptyProposalId,
    DuplicateProposal(String),
    NonFiniteTransform(String),
    CommittedSceneChanged { expected: String, actual: String },
}

impl std::fmt::Display for PreviewSceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scene(error) => write!(f, "scene error: {error}"),
            Self::EmptyBranchId => write!(f, "preview branch id may not be empty"),
            Self::MissingBaseIdentity => write!(f, "preview base revision/hash is missing"),
            Self::BaseHashMismatch { expected, actual } => {
                write!(f, "base scene hash mismatch: expected {expected}, got {actual}")
            }
            Self::DuplicateStableId(id) => write!(f, "duplicate stable id: {id}"),
            Self::UnknownStableId(id) => write!(f, "unknown stable id: {id}"),
            Self::EmptyProposalId => write!(f, "proposal id may not be empty"),
            Self::DuplicateProposal(id) => write!(f, "proposal already applied in preview: {id}"),
            Self::NonFiniteTransform(id) => write!(f, "preview transform for {id} is non-finite"),
            Self::CommittedSceneChanged { expected, actual } => write!(
                f,
                "committed scene changed during preview: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for PreviewSceneError {}

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
    fn preview_changes_its_hash_without_changing_committed_records() {
        let committed = vec![record("a", 0.0)];
        let base = stable_scene_hash(&committed).unwrap();
        let mut preview =
            IsolatedPreviewScene::from_committed("b1", "r1", base.clone(), &committed).unwrap();
        preview
            .preview_transform(
                "p1",
                "a",
                [2.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
            )
            .unwrap();
        assert_ne!(preview.preview_scene_hash().unwrap(), base);
        preview.verify_committed_unchanged(&committed).unwrap();
        assert_eq!(stable_scene_hash(&committed).unwrap(), base);
    }

    #[test]
    fn construction_fails_if_claimed_base_hash_is_wrong() {
        let committed = vec![record("a", 0.0)];
        assert!(matches!(
            IsolatedPreviewScene::from_committed("b1", "r1", "wrong", &committed),
            Err(PreviewSceneError::BaseHashMismatch { .. })
        ));
    }

    #[test]
    fn preview_rejects_duplicate_proposal_application() {
        let committed = vec![record("a", 0.0)];
        let base = stable_scene_hash(&committed).unwrap();
        let mut preview =
            IsolatedPreviewScene::from_committed("b1", "r1", base, &committed).unwrap();
        preview.preview_visibility("p1", "a", false).unwrap();
        assert_eq!(
            preview.preview_visibility("p1", "a", true),
            Err(PreviewSceneError::DuplicateProposal("p1".into()))
        );
    }
}
