// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal composition evidence for ARTIST-EYE-v1C.
//!
//! This module compares already-bound spatial/depth observations across time.
//! Camera motion, focal migration, image-structure change, and depth/occlusion
//! change remain separate evidence channels. No cinematic-quality scalar or
//! mutation authority is defined here.

use bevy::prelude::{Quat, Vec3};

use crate::{
    art_depth::{ArtistDepthConsequenceEvidence, ArtistDepthObservation},
    art_eye::{ArtistEyeConsequenceEvidence, ArtistEyeError, ArtistEyeObservation},
    art_timeline::StudioFrame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtistTemporalConfig {
    pub max_frame_gap: u64,
}

impl Default for ArtistTemporalConfig {
    fn default() -> Self {
        Self { max_frame_gap: 4 }
    }
}

impl ArtistTemporalConfig {
    pub fn validate(self) -> Result<Self, ArtistTemporalError> {
        if self.max_frame_gap == 0 {
            return Err(ArtistTemporalError::InvalidConfig);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtistCameraPoseSample {
    pub position: Vec3,
    pub rotation: Quat,
}

impl ArtistCameraPoseSample {
    pub fn validate(self) -> Result<Self, ArtistTemporalError> {
        let finite_position = self.position.to_array().into_iter().all(f32::is_finite);
        let finite_rotation = self.rotation.to_array().into_iter().all(f32::is_finite);
        if !finite_position || !finite_rotation || self.rotation.length_squared() <= f32::EPSILON {
            return Err(ArtistTemporalError::InvalidCameraPose);
        }
        Ok(Self {
            position: self.position,
            rotation: self.rotation.normalize(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistTemporalFrame {
    pub camera_stable_id: String,
    pub spatial: ArtistEyeObservation,
    pub depth: Option<ArtistDepthObservation>,
    pub camera_pose: Option<ArtistCameraPoseSample>,
}

impl ArtistTemporalFrame {
    pub fn frame(&self) -> StudioFrame {
        self.spatial.frame
    }

    pub fn validate(&self) -> Result<(), ArtistTemporalError> {
        if self.camera_stable_id.trim().is_empty()
            || self.spatial.revision_id.trim().is_empty()
            || self.spatial.rendered_scene_hash.trim().is_empty()
            || self.spatial.spatial.levels.is_empty()
        {
            return Err(ArtistTemporalError::MissingIdentity);
        }
        if let Some(depth) = &self.depth {
            if depth.revision_id != self.spatial.revision_id
                || depth.frame != self.spatial.frame
                || depth.rendered_scene_hash != self.spatial.rendered_scene_hash
            {
                return Err(ArtistTemporalError::DepthSpatialMisalignment);
            }
        }
        if let Some(pose) = self.camera_pose {
            pose.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraMotionEvidence {
    pub translation_meters: f64,
    pub rotation_radians: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FocalMigrationEvidence {
    pub level: u8,
    pub strongest_region_migration: Option<f64>,
    pub strongest_salience_delta: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibilityChangeEvidence {
    pub occupied_fraction_delta: f64,
    pub negative_space_fraction_delta: f64,
    pub depth_valid_fraction_delta: Option<f64>,
    pub depth_boundary_fraction_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistTemporalTransition {
    pub from_frame: StudioFrame,
    pub to_frame: StudioFrame,
    pub frame_gap: u64,
    pub from_revision: String,
    pub to_revision: String,
    pub from_scene_hash: String,
    pub to_scene_hash: String,
    pub spatial_delta: ArtistEyeConsequenceEvidence,
    pub focal_migration: Vec<FocalMigrationEvidence>,
    pub depth_delta: Option<ArtistDepthConsequenceEvidence>,
    pub camera_motion: Option<CameraMotionEvidence>,
    pub visibility_change: VisibilityChangeEvidence,
}

impl ArtistTemporalTransition {
    pub fn between(
        from: &ArtistTemporalFrame,
        to: &ArtistTemporalFrame,
        config: ArtistTemporalConfig,
    ) -> Result<Self, ArtistTemporalError> {
        let config = config.validate()?;
        from.validate()?;
        to.validate()?;
        if from.camera_stable_id != to.camera_stable_id {
            return Err(ArtistTemporalError::CrossCameraTransition);
        }
        if to.frame().0 <= from.frame().0 {
            return Err(ArtistTemporalError::NonMonotonicFrames);
        }
        let frame_gap = to.frame().0 - from.frame().0;
        if frame_gap > config.max_frame_gap {
            return Err(ArtistTemporalError::FrameGapExceeded {
                allowed: config.max_frame_gap,
                actual: frame_gap,
            });
        }

        let spatial_delta = ArtistEyeConsequenceEvidence::between(&from.spatial, &to.spatial)
            .map_err(ArtistTemporalError::Spatial)?;
        let focal_migration = focal_migration(&from.spatial, &to.spatial)?;
        let depth_delta = match (&from.depth, &to.depth) {
            (Some(a), Some(b)) => Some(ArtistDepthConsequenceEvidence::between(a, b)),
            (None, None) => None,
            _ => return Err(ArtistTemporalError::InconsistentDepthAvailability),
        };
        let camera_motion = match (from.camera_pose, to.camera_pose) {
            (Some(a), Some(b)) => Some(camera_motion(a, b)?),
            (None, None) => None,
            _ => return Err(ArtistTemporalError::InconsistentCameraPoseAvailability),
        };

        let a0 = from
            .spatial
            .spatial
            .levels
            .first()
            .ok_or(ArtistTemporalError::MissingSpatialLevel)?;
        let b0 = to
            .spatial
            .spatial
            .levels
            .first()
            .ok_or(ArtistTemporalError::MissingSpatialLevel)?;
        let visibility_change = VisibilityChangeEvidence {
            occupied_fraction_delta: b0.silhouette.occupied_fraction
                - a0.silhouette.occupied_fraction,
            negative_space_fraction_delta: b0.silhouette.negative_space_fraction
                - a0.silhouette.negative_space_fraction,
            depth_valid_fraction_delta: match (&from.depth, &to.depth) {
                (Some(a), Some(b)) => Some(
                    b.evidence.distribution.valid_fraction
                        - a.evidence.distribution.valid_fraction,
                ),
                _ => None,
            },
            depth_boundary_fraction_delta: match (&from.depth, &to.depth) {
                (Some(a), Some(b)) => Some(
                    b.evidence.discontinuities.boundary_fraction
                        - a.evidence.discontinuities.boundary_fraction,
                ),
                _ => None,
            },
        };

        Ok(Self {
            from_frame: from.frame(),
            to_frame: to.frame(),
            frame_gap,
            from_revision: from.spatial.revision_id.clone(),
            to_revision: to.spatial.revision_id.clone(),
            from_scene_hash: from.spatial.rendered_scene_hash.clone(),
            to_scene_hash: to.spatial.rendered_scene_hash.clone(),
            spatial_delta,
            focal_migration,
            depth_delta,
            camera_motion,
            visibility_change,
        })
    }
}

fn focal_migration(
    from: &ArtistEyeObservation,
    to: &ArtistEyeObservation,
) -> Result<Vec<FocalMigrationEvidence>, ArtistTemporalError> {
    if from.spatial.levels.len() != to.spatial.levels.len() {
        return Err(ArtistTemporalError::PyramidShapeMismatch);
    }
    let mut out = Vec::with_capacity(from.spatial.levels.len());
    for (a, b) in from.spatial.levels.iter().zip(&to.spatial.levels) {
        if a.level != b.level {
            return Err(ArtistTemporalError::PyramidShapeMismatch);
        }
        let (migration, salience_delta) = match (
            a.focal_hierarchy.regions.first(),
            b.focal_hierarchy.regions.first(),
        ) {
            (Some(ar), Some(br)) => {
                let dx = br.center_x_normalized - ar.center_x_normalized;
                let dy = br.center_y_normalized - ar.center_y_normalized;
                (
                    Some((dx * dx + dy * dy).sqrt()),
                    Some(br.salience_magnitude - ar.salience_magnitude),
                )
            }
            _ => (None, None),
        };
        out.push(FocalMigrationEvidence {
            level: a.level,
            strongest_region_migration: migration,
            strongest_salience_delta: salience_delta,
        });
    }
    Ok(out)
}

fn camera_motion(
    from: ArtistCameraPoseSample,
    to: ArtistCameraPoseSample,
) -> Result<CameraMotionEvidence, ArtistTemporalError> {
    let from = from.validate()?;
    let to = to.validate()?;
    let translation_meters = f64::from(from.position.distance(to.position));
    let dot = from.rotation.dot(to.rotation).abs().clamp(0.0, 1.0);
    let rotation_radians = f64::from(2.0 * dot.acos());
    Ok(CameraMotionEvidence {
        translation_meters,
        rotation_radians,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArtistTemporalError {
    InvalidConfig,
    InvalidCameraPose,
    MissingIdentity,
    MissingSpatialLevel,
    DepthSpatialMisalignment,
    CrossCameraTransition,
    NonMonotonicFrames,
    FrameGapExceeded { allowed: u64, actual: u64 },
    InconsistentDepthAvailability,
    InconsistentCameraPoseAvailability,
    PyramidShapeMismatch,
    Spatial(ArtistEyeError),
}

impl std::fmt::Display for ArtistTemporalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "temporal eye config is invalid"),
            Self::InvalidCameraPose => write!(f, "camera pose contains invalid values"),
            Self::MissingIdentity => write!(f, "temporal frame is missing identity/evidence"),
            Self::MissingSpatialLevel => write!(f, "temporal frame has no spatial pyramid level"),
            Self::DepthSpatialMisalignment => write!(f, "depth and spatial observations are not aligned"),
            Self::CrossCameraTransition => write!(f, "temporal transition mixes camera identities"),
            Self::NonMonotonicFrames => write!(f, "temporal frames must be strictly increasing"),
            Self::FrameGapExceeded { allowed, actual } => write!(f, "frame gap {actual} exceeds allowed {allowed}"),
            Self::InconsistentDepthAvailability => write!(f, "depth availability changed inside transition"),
            Self::InconsistentCameraPoseAvailability => write!(f, "camera-pose availability changed inside transition"),
            Self::PyramidShapeMismatch => write!(f, "spatial pyramid shapes differ across frames"),
            Self::Spatial(error) => write!(f, "spatial evidence error: {error}"),
        }
    }
}

impl std::error::Error for ArtistTemporalError {}
