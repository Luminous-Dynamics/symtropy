// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative motion attribution for ARTIST-EYE-v1D.
//!
//! This module does not infer physical causality from optical displacement.
//! It classifies whether motion evidence is supported by the semantic transform
//! plane, the recorded camera-pose plane, both, neither, or is unavailable due
//! to a raster visibility transition.

use crate::art_object_temporal::{
    ObjectIdentityTransition, PersistentObjectTransition,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionAttributionConfig {
    pub semantic_translation_epsilon: f64,
    pub semantic_rotation_epsilon: f64,
    pub semantic_scale_epsilon: f64,
    pub camera_translation_epsilon: f64,
    pub camera_rotation_epsilon: f64,
    pub screen_motion_epsilon: f64,
}

impl Default for MotionAttributionConfig {
    fn default() -> Self {
        Self {
            semantic_translation_epsilon: 1e-5,
            semantic_rotation_epsilon: 1e-5,
            semantic_scale_epsilon: 1e-5,
            camera_translation_epsilon: 1e-5,
            camera_rotation_epsilon: 1e-5,
            screen_motion_epsilon: 1e-5,
        }
    }
}

impl MotionAttributionConfig {
    pub fn validate(self) -> Result<Self, MotionAttributionError> {
        let values = [
            self.semantic_translation_epsilon,
            self.semantic_rotation_epsilon,
            self.semantic_scale_epsilon,
            self.camera_translation_epsilon,
            self.camera_rotation_epsilon,
            self.screen_motion_epsilon,
        ];
        if values.iter().any(|value| !value.is_finite() || *value < 0.0) {
            return Err(MotionAttributionError::InvalidConfig);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMotionAttribution {
    /// Object was visible in only one endpoint, so centroid motion is not a
    /// valid persistent trajectory measurement.
    VisibilityTransition,
    /// No semantic-transform, camera-pose or screen-centroid change exceeded
    /// the frozen thresholds.
    NoTrackedMotion,
    /// The semantic transform changed while camera pose remained effectively
    /// stable. This supports object/scene motion in the semantic transform
    /// space, but does not by itself prove rigid physical motion.
    SemanticTransformMotion,
    /// Camera pose changed while the object's semantic transform remained
    /// stable. Any screen motion may therefore be camera-induced.
    CameraMotionWithSemanticTransformStable,
    /// Both camera pose and object semantic transform changed.
    MixedCameraAndSemanticTransformMotion,
    /// The screen centroid moved, but neither recorded semantic transform nor
    /// camera pose changed enough to explain it. Possible causes include
    /// deformation, parent/global transform effects, animation omitted from the
    /// semantic plane, render effects, or a measurement defect.
    UnattributedScreenMotion,
    /// Semantic/camera evidence changed without measurable centroid motion.
    /// This is valid for depth-axis motion, symmetric motion, or sub-threshold
    /// raster displacement and must not be coerced into "no motion".
    NonScreenMotionEvidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMotionAttributionEvidence {
    pub stable_id: String,
    pub attribution: ObjectMotionAttribution,
    pub semantic_transform_changed: bool,
    pub camera_changed: bool,
    pub screen_centroid_changed: bool,
}

pub fn attribute_transition_motion(
    transition: &ObjectIdentityTransition,
    config: MotionAttributionConfig,
) -> Result<Vec<ObjectMotionAttributionEvidence>, MotionAttributionError> {
    let config = config.validate()?;
    let camera_changed = transition.camera_motion.is_some_and(|camera| {
        camera.translation_meters > config.camera_translation_epsilon
            || camera.rotation_radians > config.camera_rotation_epsilon
    });

    Ok(transition
        .objects
        .iter()
        .map(|object| attribute_object(object, camera_changed, config))
        .collect())
}

fn attribute_object(
    object: &PersistentObjectTransition,
    camera_changed: bool,
    config: MotionAttributionConfig,
) -> ObjectMotionAttributionEvidence {
    let semantic_transform_changed = object.semantic_transform_delta.is_some_and(|delta| {
        delta.translation_distance > config.semantic_translation_epsilon
            || delta.rotation_radians > config.semantic_rotation_epsilon
            || delta.scale_l2_delta > config.semantic_scale_epsilon
    });
    let screen_centroid_changed = object.screen_trajectory.is_some_and(|screen| {
        screen.centroid_distance_normalized > config.screen_motion_epsilon
    });

    let attribution = if !(object.raster_visible_from && object.raster_visible_to) {
        ObjectMotionAttribution::VisibilityTransition
    } else {
        match (
            semantic_transform_changed,
            camera_changed,
            screen_centroid_changed,
        ) {
            (false, false, false) => ObjectMotionAttribution::NoTrackedMotion,
            (true, false, true) | (true, false, false) => {
                if screen_centroid_changed {
                    ObjectMotionAttribution::SemanticTransformMotion
                } else {
                    ObjectMotionAttribution::NonScreenMotionEvidence
                }
            }
            (false, true, true) => {
                ObjectMotionAttribution::CameraMotionWithSemanticTransformStable
            }
            (false, true, false) => ObjectMotionAttribution::NonScreenMotionEvidence,
            (true, true, _) => ObjectMotionAttribution::MixedCameraAndSemanticTransformMotion,
            (false, false, true) => ObjectMotionAttribution::UnattributedScreenMotion,
        }
    };

    ObjectMotionAttributionEvidence {
        stable_id: object.stable_id.clone(),
        attribution,
        semantic_transform_changed,
        camera_changed,
        screen_centroid_changed,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionAttributionError {
    InvalidConfig,
}

impl std::fmt::Display for MotionAttributionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "motion attribution thresholds must be finite and non-negative"),
        }
    }
}

impl std::error::Error for MotionAttributionError {}
