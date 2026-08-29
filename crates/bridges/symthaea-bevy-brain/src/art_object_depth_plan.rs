// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Prospective pairing of object-ID and metric-depth acquisition.
//!
//! The pair is frozen before either GPU pass runs. This prevents asynchronous
//! completion order or capture-ID confusion from deciding post hoc which depth
//! plane belongs to which object-ID plane.

use crate::{
    art_capture::{ArtCapturePurpose, ArtCaptureReceipt, ArtCaptureRequest, ArtRenderChannel},
    art_object_id::ObjectIdRegistry,
    art_object_render_plan::{ObjectIdRenderPlan, ObjectIdRenderPlanError},
    art_timeline::StudioFrame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDepthCapturePlan {
    pub pair_id: String,
    pub object_id_plan: ObjectIdRenderPlan,
    pub depth_capture_id: String,
}

impl ObjectDepthCapturePlan {
    pub fn build(
        pair_id: impl Into<String>,
        object_id_plan: ObjectIdRenderPlan,
        depth_capture_id: impl Into<String>,
        registry: &ObjectIdRegistry,
    ) -> Result<Self, ObjectDepthCapturePlanError> {
        let pair_id = pair_id.into();
        let depth_capture_id = depth_capture_id.into();
        if pair_id.trim().is_empty() {
            return Err(ObjectDepthCapturePlanError::EmptyPairId);
        }
        if depth_capture_id.trim().is_empty() {
            return Err(ObjectDepthCapturePlanError::EmptyDepthCaptureId);
        }
        if depth_capture_id == object_id_plan.capture_id {
            return Err(ObjectDepthCapturePlanError::DuplicateCaptureId);
        }
        object_id_plan
            .validate(registry)
            .map_err(ObjectDepthCapturePlanError::ObjectPlan)?;
        Ok(Self {
            pair_id,
            object_id_plan,
            depth_capture_id,
        })
    }

    pub fn revision_id(&self) -> &str {
        &self.object_id_plan.revision_id
    }

    pub fn frame(&self) -> StudioFrame {
        self.object_id_plan.frame
    }

    pub fn scene_hash(&self) -> &str {
        &self.object_id_plan.scene_hash
    }

    pub fn camera_stable_id(&self) -> &str {
        &self.object_id_plan.camera_stable_id
    }

    pub fn object_request(&self) -> ArtCaptureRequest {
        self.object_id_plan.capture_request()
    }

    pub fn depth_request(&self) -> ArtCaptureRequest {
        ArtCaptureRequest {
            capture_id: self.depth_capture_id.clone(),
            revision_id: self.object_id_plan.revision_id.clone(),
            frame: self.object_id_plan.frame,
            scene_hash: self.object_id_plan.scene_hash.clone(),
            camera_stable_id: Some(self.object_id_plan.camera_stable_id.clone()),
            width: self.object_id_plan.width,
            height: self.object_id_plan.height,
            purpose: ArtCapturePurpose::CommittedObservation,
            channels: vec![ArtRenderChannel::Depth],
        }
    }

    pub fn validate_receipts(
        &self,
        object_receipt: &ArtCaptureReceipt,
        depth_receipt: &ArtCaptureReceipt,
        registry: &ObjectIdRegistry,
    ) -> Result<(), ObjectDepthCapturePlanError> {
        self.object_id_plan
            .validate(registry)
            .map_err(ObjectDepthCapturePlanError::ObjectPlan)?;
        object_receipt
            .validate_alignment()
            .map_err(|error| ObjectDepthCapturePlanError::Receipt(error.to_string()))?;
        depth_receipt
            .validate_alignment()
            .map_err(|error| ObjectDepthCapturePlanError::Receipt(error.to_string()))?;
        if object_receipt.request != self.object_request() {
            return Err(ObjectDepthCapturePlanError::ObjectReceiptDoesNotMatchPlan);
        }
        if depth_receipt.request != self.depth_request() {
            return Err(ObjectDepthCapturePlanError::DepthReceiptDoesNotMatchPlan);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectDepthCapturePlanError {
    EmptyPairId,
    EmptyDepthCaptureId,
    DuplicateCaptureId,
    ObjectPlan(ObjectIdRenderPlanError),
    Receipt(String),
    ObjectReceiptDoesNotMatchPlan,
    DepthReceiptDoesNotMatchPlan,
}

impl std::fmt::Display for ObjectDepthCapturePlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPairId => write!(f, "object/depth pair ID may not be empty"),
            Self::EmptyDepthCaptureId => write!(f, "depth capture ID may not be empty"),
            Self::DuplicateCaptureId => write!(f, "object and depth captures require distinct IDs"),
            Self::ObjectPlan(error) => write!(f, "object-ID plan error: {error}"),
            Self::Receipt(error) => write!(f, "capture receipt error: {error}"),
            Self::ObjectReceiptDoesNotMatchPlan => write!(f, "object receipt differs from prospective pair plan"),
            Self::DepthReceiptDoesNotMatchPlan => write!(f, "depth receipt differs from prospective pair plan"),
        }
    }
}

impl std::error::Error for ObjectDepthCapturePlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_object_temporal::{SemanticObjectFrame, SemanticObjectState};

    fn semantic() -> SemanticObjectFrame {
        SemanticObjectFrame {
            revision_id: "r1".into(),
            frame: StudioFrame(4),
            scene_hash: "scene".into(),
            objects: vec![SemanticObjectState {
                stable_id: "form".into(),
                parent_id: None,
                kind: "form".into(),
                material_id: None,
                translation: [0.0, 0.0, 0.0],
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
                authored_visible: true,
            }],
        }
    }

    #[test]
    fn pair_generates_identical_causal_plane_requests() {
        let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
        let object_plan = ObjectIdRenderPlan::build(
            "objects",
            "camera",
            64,
            32,
            &semantic(),
            &registry,
        )
        .unwrap();
        let pair = ObjectDepthCapturePlan::build("pair", object_plan, "depth", &registry).unwrap();
        let objects = pair.object_request();
        let depth = pair.depth_request();
        assert_eq!(objects.revision_id, depth.revision_id);
        assert_eq!(objects.frame, depth.frame);
        assert_eq!(objects.scene_hash, depth.scene_hash);
        assert_eq!(objects.camera_stable_id, depth.camera_stable_id);
        assert_eq!((objects.width, objects.height), (depth.width, depth.height));
        assert_eq!(objects.channels, vec![ArtRenderChannel::ObjectId]);
        assert_eq!(depth.channels, vec![ArtRenderChannel::Depth]);
    }
}
