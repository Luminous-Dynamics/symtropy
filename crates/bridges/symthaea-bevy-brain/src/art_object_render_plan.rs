// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Prospective object-ID render planning for ARTIST-EYE-v1D.
//!
//! The host must know the exact stable-ID -> raster-ID assignment before it
//! renders an object-ID attachment. This plan binds that assignment to one
//! revision/frame/scene/camera/resolution and one frozen registry digest.

use crate::{
    art_capture::{ArtCapturePurpose, ArtCaptureRequest, ArtRenderChannel},
    art_object_id::ObjectIdRegistry,
    art_object_temporal::SemanticObjectFrame,
    art_timeline::StudioFrame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdRenderAssignment {
    pub stable_id: String,
    pub raster_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdRenderPlan {
    pub capture_id: String,
    pub revision_id: String,
    pub frame: StudioFrame,
    pub scene_hash: String,
    pub camera_stable_id: String,
    pub width: u32,
    pub height: u32,
    pub registry_digest: String,
    pub assignments: Vec<ObjectIdRenderAssignment>,
}

impl ObjectIdRenderPlan {
    pub fn build(
        capture_id: impl Into<String>,
        camera_stable_id: impl Into<String>,
        width: u32,
        height: u32,
        semantic: &SemanticObjectFrame,
        registry: &ObjectIdRegistry,
    ) -> Result<Self, ObjectIdRenderPlanError> {
        semantic
            .validate()
            .map_err(|error| ObjectIdRenderPlanError::Semantic(error.to_string()))?;
        let capture_id = capture_id.into();
        let camera_stable_id = camera_stable_id.into();
        if capture_id.trim().is_empty() {
            return Err(ObjectIdRenderPlanError::EmptyCaptureId);
        }
        if camera_stable_id.trim().is_empty() {
            return Err(ObjectIdRenderPlanError::EmptyCameraId);
        }
        if width == 0 || height == 0 {
            return Err(ObjectIdRenderPlanError::InvalidResolution);
        }

        let mut assignments = Vec::with_capacity(semantic.objects.len());
        for object in &semantic.objects {
            let raster_id = registry
                .raster_id(&object.stable_id)
                .ok_or_else(|| {
                    ObjectIdRenderPlanError::ObjectMissingFromRegistry(object.stable_id.clone())
                })?;
            assignments.push(ObjectIdRenderAssignment {
                stable_id: object.stable_id.clone(),
                raster_id,
            });
        }
        assignments.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));

        let plan = Self {
            capture_id,
            revision_id: semantic.revision_id.clone(),
            frame: semantic.frame,
            scene_hash: semantic.scene_hash.clone(),
            camera_stable_id,
            width,
            height,
            registry_digest: registry.digest().to_owned(),
            assignments,
        };
        plan.validate(registry)?;
        Ok(plan)
    }

    pub fn validate(&self, registry: &ObjectIdRegistry) -> Result<(), ObjectIdRenderPlanError> {
        if self.capture_id.trim().is_empty() {
            return Err(ObjectIdRenderPlanError::EmptyCaptureId);
        }
        if self.camera_stable_id.trim().is_empty() {
            return Err(ObjectIdRenderPlanError::EmptyCameraId);
        }
        if self.revision_id.trim().is_empty() || self.scene_hash.trim().is_empty() {
            return Err(ObjectIdRenderPlanError::MissingSceneIdentity);
        }
        if self.width == 0 || self.height == 0 {
            return Err(ObjectIdRenderPlanError::InvalidResolution);
        }
        if self.registry_digest != registry.digest() {
            return Err(ObjectIdRenderPlanError::RegistryDigestMismatch);
        }
        let mut last: Option<&str> = None;
        for assignment in &self.assignments {
            if assignment.stable_id.trim().is_empty() || assignment.raster_id == 0 {
                return Err(ObjectIdRenderPlanError::InvalidAssignment);
            }
            if last.is_some_and(|previous| previous >= assignment.stable_id.as_str()) {
                return Err(ObjectIdRenderPlanError::AssignmentsNotCanonical);
            }
            if registry.raster_id(&assignment.stable_id) != Some(assignment.raster_id) {
                return Err(ObjectIdRenderPlanError::RegistryAssignmentMismatch(
                    assignment.stable_id.clone(),
                ));
            }
            last = Some(&assignment.stable_id);
        }
        Ok(())
    }

    pub fn capture_request(&self) -> ArtCaptureRequest {
        ArtCaptureRequest {
            capture_id: self.capture_id.clone(),
            revision_id: self.revision_id.clone(),
            frame: self.frame,
            scene_hash: self.scene_hash.clone(),
            camera_stable_id: Some(self.camera_stable_id.clone()),
            width: self.width,
            height: self.height,
            purpose: ArtCapturePurpose::CommittedObservation,
            channels: vec![ArtRenderChannel::ObjectId],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectIdRenderPlanError {
    EmptyCaptureId,
    EmptyCameraId,
    MissingSceneIdentity,
    InvalidResolution,
    Semantic(String),
    ObjectMissingFromRegistry(String),
    RegistryDigestMismatch,
    InvalidAssignment,
    AssignmentsNotCanonical,
    RegistryAssignmentMismatch(String),
}

impl std::fmt::Display for ObjectIdRenderPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCaptureId => write!(f, "object-id render plan capture id may not be empty"),
            Self::EmptyCameraId => write!(f, "object-id render plan camera id may not be empty"),
            Self::MissingSceneIdentity => write!(f, "object-id render plan lacks revision/scene identity"),
            Self::InvalidResolution => write!(f, "object-id render plan resolution must be non-zero"),
            Self::Semantic(error) => write!(f, "semantic frame is invalid: {error}"),
            Self::ObjectMissingFromRegistry(id) => write!(f, "semantic object {id} is missing from object-id registry"),
            Self::RegistryDigestMismatch => write!(f, "object-id render plan registry digest changed"),
            Self::InvalidAssignment => write!(f, "object-id render assignment contains an empty/zero identity"),
            Self::AssignmentsNotCanonical => write!(f, "object-id render assignments are not strictly sorted and unique"),
            Self::RegistryAssignmentMismatch(id) => write!(f, "object-id render assignment for {id} differs from frozen registry"),
        }
    }
}

impl std::error::Error for ObjectIdRenderPlanError {}
