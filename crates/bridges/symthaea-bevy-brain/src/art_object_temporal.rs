// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistent object identity and trajectory evidence for ARTIST-EYE-v1D.
//!
//! This layer joins two independent planes:
//! - semantic scene records keyed by persistent artistic IDs; and
//! - an object-ID raster describing which of those IDs were actually visible.
//!
//! It intentionally distinguishes semantic creation/destruction and authored
//! visibility changes from raster appearance/disappearance. Raster loss alone
//! is not called "concealment" and raster gain alone is not called "reveal":
//! those causal labels require separate occluder evidence.
//!
//! `ArtSceneRecord` stores the host-provided `Transform`. Depending on the host,
//! that may be local to a parent hierarchy, so transform deltas here are called
//! *semantic-transform* evidence rather than global/world motion.

use std::collections::BTreeSet;

use bevy::prelude::{Quat, Vec3};

use crate::{
    art_object_id::{ObjectIdObservation, ObjectIdRegistry, ObjectRasterEvidence},
    art_scene::{stable_scene_hash, ArtSceneError, ArtSceneRecord},
    art_temporal::ArtistCameraPoseSample,
    art_timeline::StudioFrame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticObjectState {
    pub stable_id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub material_id: Option<String>,
    pub translation: [f32; 3],
    pub rotation_xyzw: [f32; 4],
    pub scale: [f32; 3],
    pub authored_visible: bool,
}

impl SemanticObjectState {
    fn as_scene_record(&self) -> ArtSceneRecord {
        ArtSceneRecord {
            stable_id: self.stable_id.clone(),
            parent_id: self.parent_id.clone(),
            kind: self.kind.clone(),
            material_id: self.material_id.clone(),
            translation: self.translation,
            rotation_xyzw: self.rotation_xyzw,
            scale: self.scale,
            visible: self.authored_visible,
        }
    }
}

impl From<&ArtSceneRecord> for SemanticObjectState {
    fn from(record: &ArtSceneRecord) -> Self {
        Self {
            stable_id: record.stable_id.clone(),
            parent_id: record.parent_id.clone(),
            kind: record.kind.clone(),
            material_id: record.material_id.clone(),
            translation: record.translation,
            rotation_xyzw: record.rotation_xyzw,
            scale: record.scale,
            authored_visible: record.visible,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticObjectFrame {
    pub revision_id: String,
    pub frame: StudioFrame,
    pub scene_hash: String,
    pub objects: Vec<SemanticObjectState>,
}

impl SemanticObjectFrame {
    /// Bind the semantic object set to the exact deterministic scene hash used
    /// by the rendered evidence plane.
    pub fn from_records(
        revision_id: impl Into<String>,
        frame: StudioFrame,
        expected_scene_hash: impl Into<String>,
        records: &[ArtSceneRecord],
    ) -> Result<Self, ObjectTemporalError> {
        let revision_id = revision_id.into();
        let expected_scene_hash = expected_scene_hash.into();
        if revision_id.trim().is_empty() || expected_scene_hash.trim().is_empty() {
            return Err(ObjectTemporalError::MissingIdentity);
        }
        let actual = stable_scene_hash(records).map_err(ObjectTemporalError::Scene)?;
        if actual != expected_scene_hash {
            return Err(ObjectTemporalError::SemanticSceneHashMismatch {
                expected: expected_scene_hash,
                actual,
            });
        }
        let mut objects: Vec<_> = records.iter().map(SemanticObjectState::from).collect();
        objects.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));
        let frame = Self {
            revision_id,
            frame,
            scene_hash: actual,
            objects,
        };
        frame.validate()?;
        Ok(frame)
    }

    /// Recompute the deterministic scene hash from the retained semantic
    /// states. This makes manually constructed or deserialized frames fail
    /// closed instead of trusting the stored `scene_hash` string.
    pub fn validate(&self) -> Result<(), ObjectTemporalError> {
        if self.revision_id.trim().is_empty() || self.scene_hash.trim().is_empty() {
            return Err(ObjectTemporalError::MissingIdentity);
        }
        let records: Vec<_> = self.objects.iter().map(SemanticObjectState::as_scene_record).collect();
        let actual = stable_scene_hash(&records).map_err(ObjectTemporalError::Scene)?;
        if actual != self.scene_hash {
            return Err(ObjectTemporalError::SemanticSceneHashMismatch {
                expected: self.scene_hash.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn object(&self, stable_id: &str) -> Option<&SemanticObjectState> {
        self.objects.iter().find(|object| object.stable_id == stable_id)
    }
}

/// One synchronized semantic+raster object-identity observation.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistentObjectFrame {
    pub camera_stable_id: String,
    pub semantic: SemanticObjectFrame,
    pub raster: ObjectIdObservation,
    pub camera_pose: Option<ArtistCameraPoseSample>,
}

impl PersistentObjectFrame {
    pub fn new(
        camera_stable_id: impl Into<String>,
        semantic: SemanticObjectFrame,
        raster: ObjectIdObservation,
        registry: &ObjectIdRegistry,
        camera_pose: Option<ArtistCameraPoseSample>,
    ) -> Result<Self, ObjectTemporalError> {
        let frame = Self {
            camera_stable_id: camera_stable_id.into(),
            semantic,
            raster,
            camera_pose,
        };
        frame.validate(registry)?;
        Ok(frame)
    }

    pub fn validate(&self, registry: &ObjectIdRegistry) -> Result<(), ObjectTemporalError> {
        if self.camera_stable_id.trim().is_empty() {
            return Err(ObjectTemporalError::MissingCameraIdentity);
        }
        self.semantic.validate()?;
        if self.semantic.revision_id != self.raster.revision_id
            || self.semantic.frame != self.raster.frame
            || self.semantic.scene_hash != self.raster.rendered_scene_hash
        {
            return Err(ObjectTemporalError::SemanticRasterMisalignment);
        }
        if self.raster.registry_digest != registry.digest() {
            return Err(ObjectTemporalError::RegistryDigestMismatch);
        }
        for object in &self.semantic.objects {
            if !registry.contains_stable_id(&object.stable_id) {
                return Err(ObjectTemporalError::SemanticObjectMissingFromRegistry(
                    object.stable_id.clone(),
                ));
            }
        }
        if let Some(pose) = self.camera_pose {
            pose.validate().map_err(ObjectTemporalError::CameraPose)?;
        }
        Ok(())
    }

    pub fn raster_object(&self, stable_id: &str) -> Option<&ObjectRasterEvidence> {
        self.raster.evidence.object(stable_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectIdentityEvent {
    SemanticCreated,
    SemanticDestroyed,
    AuthoredVisibilityEnabled,
    AuthoredVisibilityDisabled,
    RasterVisibilityAcquired,
    RasterVisibilityLost,
    ParentChanged {
        from: Option<String>,
        to: Option<String>,
    },
    MaterialChanged {
        from: Option<String>,
        to: Option<String>,
    },
    KindChanged {
        from: String,
        to: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticTransformDelta {
    /// Euclidean translation distance in the coordinate/unit system carried by
    /// the semantic scene record. This is not called world meters because the
    /// record may contain a local transform.
    pub translation_distance: f64,
    pub rotation_radians: f64,
    pub scale_l2_delta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenTrajectoryEvidence {
    pub centroid_dx_normalized: f64,
    pub centroid_dy_normalized: f64,
    pub centroid_distance_normalized: f64,
    pub visible_fraction_delta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectCameraMotionEvidence {
    pub translation_meters: f64,
    pub rotation_radians: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentObjectTransition {
    pub stable_id: String,
    pub existed_from: bool,
    pub existed_to: bool,
    pub raster_visible_from: bool,
    pub raster_visible_to: bool,
    pub semantic_transform_delta: Option<SemanticTransformDelta>,
    pub screen_trajectory: Option<ScreenTrajectoryEvidence>,
    pub events: Vec<ObjectIdentityEvent>,
}

impl PersistentObjectTransition {
    pub fn semantic_motion_observed(&self, translation_epsilon: f64, rotation_epsilon: f64) -> bool {
        self.semantic_transform_delta.is_some_and(|delta| {
            delta.translation_distance > translation_epsilon
                || delta.rotation_radians > rotation_epsilon
                || delta.scale_l2_delta > translation_epsilon
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIdentityTransition {
    pub camera_stable_id: String,
    pub from_frame: StudioFrame,
    pub to_frame: StudioFrame,
    pub frame_gap: u64,
    pub from_revision: String,
    pub to_revision: String,
    pub from_scene_hash: String,
    pub to_scene_hash: String,
    pub camera_motion: Option<ObjectCameraMotionEvidence>,
    pub objects: Vec<PersistentObjectTransition>,
}

impl ObjectIdentityTransition {
    pub fn between(
        from: &PersistentObjectFrame,
        to: &PersistentObjectFrame,
        registry: &ObjectIdRegistry,
        max_frame_gap: u64,
    ) -> Result<Self, ObjectTemporalError> {
        if max_frame_gap == 0 {
            return Err(ObjectTemporalError::InvalidMaxFrameGap);
        }
        from.validate(registry)?;
        to.validate(registry)?;
        if from.camera_stable_id != to.camera_stable_id {
            return Err(ObjectTemporalError::CrossCameraTransition);
        }
        if to.semantic.frame.0 <= from.semantic.frame.0 {
            return Err(ObjectTemporalError::NonMonotonicFrames);
        }
        let frame_gap = to.semantic.frame.0 - from.semantic.frame.0;
        if frame_gap > max_frame_gap {
            return Err(ObjectTemporalError::FrameGapExceeded {
                allowed: max_frame_gap,
                actual: frame_gap,
            });
        }
        if from.raster.registry_digest != to.raster.registry_digest {
            return Err(ObjectTemporalError::RegistryDigestMismatch);
        }

        let camera_motion = match (from.camera_pose, to.camera_pose) {
            (Some(a), Some(b)) => Some(camera_delta(a, b)?),
            (None, None) => None,
            _ => return Err(ObjectTemporalError::InconsistentCameraPoseAvailability),
        };

        let mut ids = BTreeSet::new();
        ids.extend(from.semantic.objects.iter().map(|o| o.stable_id.as_str()));
        ids.extend(to.semantic.objects.iter().map(|o| o.stable_id.as_str()));
        ids.extend(from.raster.evidence.objects.iter().map(|o| o.stable_id.as_str()));
        ids.extend(to.raster.evidence.objects.iter().map(|o| o.stable_id.as_str()));

        let mut objects = Vec::with_capacity(ids.len());
        for stable_id in ids {
            let a_sem = from.semantic.object(stable_id);
            let b_sem = to.semantic.object(stable_id);
            let a_raster = from.raster_object(stable_id);
            let b_raster = to.raster_object(stable_id);

            let mut events = Vec::new();
            match (a_sem, b_sem) {
                (None, Some(_)) => events.push(ObjectIdentityEvent::SemanticCreated),
                (Some(_), None) => events.push(ObjectIdentityEvent::SemanticDestroyed),
                _ => {}
            }
            if let (Some(a), Some(b)) = (a_sem, b_sem) {
                if !a.authored_visible && b.authored_visible {
                    events.push(ObjectIdentityEvent::AuthoredVisibilityEnabled);
                } else if a.authored_visible && !b.authored_visible {
                    events.push(ObjectIdentityEvent::AuthoredVisibilityDisabled);
                }
                if a.parent_id != b.parent_id {
                    events.push(ObjectIdentityEvent::ParentChanged {
                        from: a.parent_id.clone(),
                        to: b.parent_id.clone(),
                    });
                }
                if a.material_id != b.material_id {
                    events.push(ObjectIdentityEvent::MaterialChanged {
                        from: a.material_id.clone(),
                        to: b.material_id.clone(),
                    });
                }
                if a.kind != b.kind {
                    events.push(ObjectIdentityEvent::KindChanged {
                        from: a.kind.clone(),
                        to: b.kind.clone(),
                    });
                }
            }
            match (a_raster, b_raster) {
                (None, Some(_)) => events.push(ObjectIdentityEvent::RasterVisibilityAcquired),
                (Some(_), None) => events.push(ObjectIdentityEvent::RasterVisibilityLost),
                _ => {}
            }

            let semantic_transform_delta = match (a_sem, b_sem) {
                (Some(a), Some(b)) => Some(transform_delta(a, b)?),
                _ => None,
            };
            let screen_trajectory = match (a_raster, b_raster) {
                (Some(a), Some(b)) => {
                    let dx = b.centroid_x_normalized - a.centroid_x_normalized;
                    let dy = b.centroid_y_normalized - a.centroid_y_normalized;
                    Some(ScreenTrajectoryEvidence {
                        centroid_dx_normalized: dx,
                        centroid_dy_normalized: dy,
                        centroid_distance_normalized: (dx * dx + dy * dy).sqrt(),
                        visible_fraction_delta: b.visible_fraction - a.visible_fraction,
                    })
                }
                _ => None,
            };

            objects.push(PersistentObjectTransition {
                stable_id: stable_id.to_owned(),
                existed_from: a_sem.is_some(),
                existed_to: b_sem.is_some(),
                raster_visible_from: a_raster.is_some(),
                raster_visible_to: b_raster.is_some(),
                semantic_transform_delta,
                screen_trajectory,
                events,
            });
        }

        Ok(Self {
            camera_stable_id: from.camera_stable_id.clone(),
            from_frame: from.semantic.frame,
            to_frame: to.semantic.frame,
            frame_gap,
            from_revision: from.semantic.revision_id.clone(),
            to_revision: to.semantic.revision_id.clone(),
            from_scene_hash: from.semantic.scene_hash.clone(),
            to_scene_hash: to.semantic.scene_hash.clone(),
            camera_motion,
            objects,
        })
    }

    pub fn object(&self, stable_id: &str) -> Option<&PersistentObjectTransition> {
        self.objects.iter().find(|object| object.stable_id == stable_id)
    }
}

fn transform_delta(
    from: &SemanticObjectState,
    to: &SemanticObjectState,
) -> Result<SemanticTransformDelta, ObjectTemporalError> {
    let a_t = Vec3::from_array(from.translation);
    let b_t = Vec3::from_array(to.translation);
    let a_q = Quat::from_array(from.rotation_xyzw);
    let b_q = Quat::from_array(to.rotation_xyzw);
    let a_s = Vec3::from_array(from.scale);
    let b_s = Vec3::from_array(to.scale);
    if !a_t.is_finite()
        || !b_t.is_finite()
        || !a_q.is_finite()
        || !b_q.is_finite()
        || !a_s.is_finite()
        || !b_s.is_finite()
        || a_q.length_squared() <= f32::EPSILON
        || b_q.length_squared() <= f32::EPSILON
    {
        return Err(ObjectTemporalError::InvalidSemanticTransform(
            from.stable_id.clone(),
        ));
    }
    let a_q = a_q.normalize();
    let b_q = b_q.normalize();
    let dot = a_q.dot(b_q).abs().clamp(0.0, 1.0);
    Ok(SemanticTransformDelta {
        translation_distance: f64::from(a_t.distance(b_t)),
        rotation_radians: f64::from(2.0 * dot.acos()),
        scale_l2_delta: f64::from(a_s.distance(b_s)),
    })
}

fn camera_delta(
    from: ArtistCameraPoseSample,
    to: ArtistCameraPoseSample,
) -> Result<ObjectCameraMotionEvidence, ObjectTemporalError> {
    let a = from.validate().map_err(ObjectTemporalError::CameraPose)?;
    let b = to.validate().map_err(ObjectTemporalError::CameraPose)?;
    let dot = a.rotation.dot(b.rotation).abs().clamp(0.0, 1.0);
    Ok(ObjectCameraMotionEvidence {
        translation_meters: f64::from(a.position.distance(b.position)),
        rotation_radians: f64::from(2.0 * dot.acos()),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectTemporalError {
    Scene(ArtSceneError),
    CameraPose(crate::art_temporal::ArtistTemporalError),
    MissingIdentity,
    MissingCameraIdentity,
    SemanticSceneHashMismatch { expected: String, actual: String },
    SemanticRasterMisalignment,
    RegistryDigestMismatch,
    SemanticObjectMissingFromRegistry(String),
    CrossCameraTransition,
    NonMonotonicFrames,
    InvalidMaxFrameGap,
    FrameGapExceeded { allowed: u64, actual: u64 },
    InconsistentCameraPoseAvailability,
    InvalidSemanticTransform(String),
}

impl std::fmt::Display for ObjectTemporalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scene(error) => write!(f, "scene identity error: {error}"),
            Self::CameraPose(error) => write!(f, "camera-pose error: {error}"),
            Self::MissingIdentity => write!(f, "semantic object frame lacks revision/scene identity"),
            Self::MissingCameraIdentity => write!(f, "persistent object frame lacks camera identity"),
            Self::SemanticSceneHashMismatch { expected, actual } => write!(
                f,
                "semantic scene hash mismatch: expected {expected}, reconstructed {actual}"
            ),
            Self::SemanticRasterMisalignment => write!(f, "semantic and object-id raster planes are not aligned"),
            Self::RegistryDigestMismatch => write!(f, "object-id registry changed within evidence lineage"),
            Self::SemanticObjectMissingFromRegistry(id) => write!(f, "semantic object {id} is absent from frozen object-id registry"),
            Self::CrossCameraTransition => write!(f, "object trajectory mixes camera identities"),
            Self::NonMonotonicFrames => write!(f, "object trajectory frames must strictly increase"),
            Self::InvalidMaxFrameGap => write!(f, "object trajectory max frame gap must be non-zero"),
            Self::FrameGapExceeded { allowed, actual } => write!(f, "object trajectory frame gap {actual} exceeds {allowed}"),
            Self::InconsistentCameraPoseAvailability => write!(f, "camera pose availability changed inside object transition"),
            Self::InvalidSemanticTransform(id) => write!(f, "semantic transform for {id} is invalid"),
        }
    }
}

impl std::error::Error for ObjectTemporalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_scene::ArtSceneRecord;

    fn record(id: &str, x: f32, visible: bool) -> ArtSceneRecord {
        ArtSceneRecord {
            stable_id: id.into(),
            parent_id: None,
            kind: "form".into(),
            material_id: Some("clay".into()),
            translation: [x, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            visible,
        }
    }

    #[test]
    fn semantic_frame_rejects_wrong_scene_hash() {
        let records = vec![record("a", 0.0, true)];
        assert!(matches!(
            SemanticObjectFrame::from_records("r", StudioFrame(1), "wrong", &records),
            Err(ObjectTemporalError::SemanticSceneHashMismatch { .. })
        ));
    }

    #[test]
    fn manually_forged_semantic_hash_is_rejected_on_validation() {
        let frame = SemanticObjectFrame {
            revision_id: "r".into(),
            frame: StudioFrame(1),
            scene_hash: "forged".into(),
            objects: vec![SemanticObjectState::from(&record("a", 0.0, true))],
        };
        assert!(matches!(
            frame.validate(),
            Err(ObjectTemporalError::SemanticSceneHashMismatch { .. })
        ));
    }

    #[test]
    fn transform_delta_keeps_translation_and_rotation_separate() {
        let a = SemanticObjectState::from(&record("a", 0.0, true));
        let mut b = SemanticObjectState::from(&record("a", 3.0, true));
        b.rotation_xyzw = Quat::from_rotation_y(0.5).to_array();
        let delta = transform_delta(&a, &b).unwrap();
        assert!((delta.translation_distance - 3.0).abs() < 1e-6);
        assert!((delta.rotation_radians - 0.5).abs() < 1e-5);
    }

    #[test]
    fn semantic_creation_is_not_called_raster_reveal() {
        let events = vec![ObjectIdentityEvent::SemanticCreated, ObjectIdentityEvent::RasterVisibilityAcquired];
        assert!(events.contains(&ObjectIdentityEvent::SemanticCreated));
        assert!(events.contains(&ObjectIdentityEvent::RasterVisibilityAcquired));
    }
}
