// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Longitudinal ARTIST-EYE-v1C evidence windows and descriptive rhythm.
//!
//! Rhythm here means measured rates/changes in perceptual evidence. It is not
//! an aesthetic rhythm score and cannot authorize an artistic mutation.

use crate::art_temporal::{
    ArtistTemporalConfig, ArtistTemporalError, ArtistTemporalFrame, ArtistTemporalTransition,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistTemporalRhythmEvidence {
    pub transition_count: usize,
    pub total_frame_span: u64,
    pub mean_abs_luminance_change: f64,
    pub mean_abs_occupancy_change: f64,
    pub mean_focal_migration: Option<f64>,
    pub mean_camera_translation_meters: Option<f64>,
    pub mean_camera_rotation_radians: Option<f64>,
    pub mean_abs_depth_boundary_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistTemporalWindow {
    pub camera_stable_id: String,
    pub frames: Vec<ArtistTemporalFrame>,
    pub transitions: Vec<ArtistTemporalTransition>,
    pub rhythm: ArtistTemporalRhythmEvidence,
}

impl ArtistTemporalWindow {
    pub fn build(
        frames: Vec<ArtistTemporalFrame>,
        config: ArtistTemporalConfig,
    ) -> Result<Self, ArtistTemporalWindowError> {
        config
            .validate()
            .map_err(ArtistTemporalWindowError::Temporal)?;
        if frames.len() < 2 {
            return Err(ArtistTemporalWindowError::RequiresAtLeastTwoFrames);
        }
        for frame in &frames {
            frame
                .validate()
                .map_err(ArtistTemporalWindowError::Temporal)?;
        }
        let camera_stable_id = frames[0].camera_stable_id.clone();
        if frames
            .iter()
            .any(|frame| frame.camera_stable_id != camera_stable_id)
        {
            return Err(ArtistTemporalWindowError::CrossCameraWindow);
        }

        let mut transitions = Vec::with_capacity(frames.len() - 1);
        for pair in frames.windows(2) {
            transitions.push(
                ArtistTemporalTransition::between(&pair[0], &pair[1], config)
                    .map_err(ArtistTemporalWindowError::Temporal)?,
            );
        }
        let rhythm = summarize_rhythm(&transitions)?;
        Ok(Self {
            camera_stable_id,
            frames,
            transitions,
            rhythm,
        })
    }
}

fn summarize_rhythm(
    transitions: &[ArtistTemporalTransition],
) -> Result<ArtistTemporalRhythmEvidence, ArtistTemporalWindowError> {
    if transitions.is_empty() {
        return Err(ArtistTemporalWindowError::RequiresAtLeastTwoFrames);
    }

    let first = transitions.first().expect("checked non-empty");
    let last = transitions.last().expect("checked non-empty");
    let total_frame_span = last.to_frame.0.saturating_sub(first.from_frame.0);

    let mut luminance = 0.0;
    let mut occupancy = 0.0;
    let mut focal = Vec::new();
    let mut camera_translation = Vec::new();
    let mut camera_rotation = Vec::new();
    let mut depth_boundary = Vec::new();

    for transition in transitions {
        let level = transition
            .spatial_delta
            .levels
            .first()
            .ok_or(ArtistTemporalWindowError::MissingSpatialDelta)?;
        luminance += level.mean_luminance_delta.abs();
        occupancy += level.occupied_fraction_delta.abs();
        if let Some(value) = transition
            .focal_migration
            .first()
            .and_then(|evidence| evidence.strongest_region_migration)
        {
            focal.push(value);
        }
        if let Some(camera) = transition.camera_motion {
            camera_translation.push(camera.translation_meters);
            camera_rotation.push(camera.rotation_radians);
        }
        if let Some(value) = transition.visibility_change.depth_boundary_fraction_delta {
            depth_boundary.push(value.abs());
        }
    }

    let n = transitions.len() as f64;
    Ok(ArtistTemporalRhythmEvidence {
        transition_count: transitions.len(),
        total_frame_span,
        mean_abs_luminance_change: luminance / n,
        mean_abs_occupancy_change: occupancy / n,
        mean_focal_migration: mean_optional(&focal),
        mean_camera_translation_meters: mean_optional(&camera_translation),
        mean_camera_rotation_radians: mean_optional(&camera_rotation),
        mean_abs_depth_boundary_change: mean_optional(&depth_boundary),
    })
}

fn mean_optional(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArtistTemporalWindowError {
    Temporal(ArtistTemporalError),
    RequiresAtLeastTwoFrames,
    CrossCameraWindow,
    MissingSpatialDelta,
}

impl std::fmt::Display for ArtistTemporalWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Temporal(error) => write!(f, "temporal evidence error: {error}"),
            Self::RequiresAtLeastTwoFrames => {
                write!(f, "temporal window requires at least two frames")
            }
            Self::CrossCameraWindow => write!(f, "temporal window mixes camera identities"),
            Self::MissingSpatialDelta => write!(f, "temporal transition has no spatial delta"),
        }
    }
}

impl std::error::Error for ArtistTemporalWindowError {}
