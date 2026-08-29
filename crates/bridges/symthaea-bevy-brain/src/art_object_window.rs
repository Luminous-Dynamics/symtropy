// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded persistent-object windows for ARTIST-EYE-v1D.
//!
//! A window aggregates already-qualified object-identity transitions without
//! collapsing them into an aesthetic or motion-quality score. Track summaries
//! retain semantic existence, raster visibility, world motion and screen motion
//! as separate evidence channels.

use std::collections::BTreeMap;

use crate::{
    art_object_id::ObjectIdRegistry,
    art_object_temporal::{
        ObjectIdentityEvent, ObjectIdentityTransition, ObjectTemporalError, PersistentObjectFrame,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTrackSummary {
    pub stable_id: String,
    pub semantic_present_frames: u32,
    pub raster_visible_frames: u32,
    pub raster_visibility_acquisitions: u32,
    pub raster_visibility_losses: u32,
    pub semantic_creations: u32,
    pub semantic_destructions: u32,
    pub authored_visibility_enables: u32,
    pub authored_visibility_disables: u32,
    pub cumulative_world_translation_meters: f64,
    pub cumulative_world_rotation_radians: f64,
    pub cumulative_screen_path_normalized: f64,
    pub maximum_visible_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentObjectWindowEvidence {
    pub frame_count: u32,
    pub transition_count: u32,
    pub distinct_semantic_objects: u32,
    pub distinct_raster_observed_objects: u32,
    pub camera_translation_meters: Option<f64>,
    pub camera_rotation_radians: Option<f64>,
    pub tracks: Vec<ObjectTrackSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentObjectWindow {
    pub frames: Vec<PersistentObjectFrame>,
    pub transitions: Vec<ObjectIdentityTransition>,
    pub evidence: PersistentObjectWindowEvidence,
}

impl PersistentObjectWindow {
    pub fn build(
        frames: Vec<PersistentObjectFrame>,
        registry: &ObjectIdRegistry,
        max_frame_gap: u64,
    ) -> Result<Self, ObjectWindowError> {
        if frames.len() < 2 {
            return Err(ObjectWindowError::RequiresAtLeastTwoFrames);
        }
        if max_frame_gap == 0 {
            return Err(ObjectWindowError::InvalidMaxFrameGap);
        }
        for frame in &frames {
            frame.validate(registry).map_err(ObjectWindowError::Temporal)?;
        }
        for pair in frames.windows(2) {
            if pair[1].semantic.frame.0 <= pair[0].semantic.frame.0 {
                return Err(ObjectWindowError::NonMonotonicFrames);
            }
            if pair[1].semantic.frame.0 - pair[0].semantic.frame.0 > max_frame_gap {
                return Err(ObjectWindowError::FrameGapExceeded);
            }
            if pair[0].camera_stable_id != pair[1].camera_stable_id {
                return Err(ObjectWindowError::CrossCameraWindow);
            }
        }

        let mut transitions = Vec::with_capacity(frames.len() - 1);
        for pair in frames.windows(2) {
            transitions.push(
                ObjectIdentityTransition::between(&pair[0], &pair[1], registry, max_frame_gap)
                    .map_err(ObjectWindowError::Temporal)?,
            );
        }

        let evidence = summarize(&frames, &transitions);
        Ok(Self {
            frames,
            transitions,
            evidence,
        })
    }
}

#[derive(Default)]
struct TrackAccumulator {
    semantic_present_frames: u32,
    raster_visible_frames: u32,
    raster_visibility_acquisitions: u32,
    raster_visibility_losses: u32,
    semantic_creations: u32,
    semantic_destructions: u32,
    authored_visibility_enables: u32,
    authored_visibility_disables: u32,
    cumulative_world_translation_meters: f64,
    cumulative_world_rotation_radians: f64,
    cumulative_screen_path_normalized: f64,
    maximum_visible_fraction: f64,
}

fn summarize(
    frames: &[PersistentObjectFrame],
    transitions: &[ObjectIdentityTransition],
) -> PersistentObjectWindowEvidence {
    let mut tracks: BTreeMap<String, TrackAccumulator> = BTreeMap::new();
    let mut semantic_ids = std::collections::BTreeSet::new();
    let mut raster_ids = std::collections::BTreeSet::new();

    for frame in frames {
        for object in &frame.semantic.objects {
            semantic_ids.insert(object.stable_id.clone());
            tracks
                .entry(object.stable_id.clone())
                .or_default()
                .semantic_present_frames += 1;
        }
        for object in &frame.raster.evidence.objects {
            raster_ids.insert(object.stable_id.clone());
            let track = tracks.entry(object.stable_id.clone()).or_default();
            track.raster_visible_frames += 1;
            track.maximum_visible_fraction = track.maximum_visible_fraction.max(object.visible_fraction);
        }
    }

    let mut camera_translation_meters = Some(0.0);
    let mut camera_rotation_radians = Some(0.0);
    for transition in transitions {
        match transition.camera_motion {
            Some(camera) => {
                if let Some(total) = &mut camera_translation_meters {
                    *total += camera.translation_meters;
                }
                if let Some(total) = &mut camera_rotation_radians {
                    *total += camera.rotation_radians;
                }
            }
            None => {
                camera_translation_meters = None;
                camera_rotation_radians = None;
            }
        }

        for object in &transition.objects {
            let track = tracks.entry(object.stable_id.clone()).or_default();
            if let Some(delta) = object.semantic_transform_delta {
                track.cumulative_world_translation_meters += delta.translation_meters;
                track.cumulative_world_rotation_radians += delta.rotation_radians;
            }
            if let Some(screen) = object.screen_trajectory {
                track.cumulative_screen_path_normalized += screen.centroid_distance_normalized;
            }
            for event in &object.events {
                match event {
                    ObjectIdentityEvent::SemanticCreated => track.semantic_creations += 1,
                    ObjectIdentityEvent::SemanticDestroyed => track.semantic_destructions += 1,
                    ObjectIdentityEvent::AuthoredVisibilityEnabled => {
                        track.authored_visibility_enables += 1
                    }
                    ObjectIdentityEvent::AuthoredVisibilityDisabled => {
                        track.authored_visibility_disables += 1
                    }
                    ObjectIdentityEvent::RasterVisibilityAcquired => {
                        track.raster_visibility_acquisitions += 1
                    }
                    ObjectIdentityEvent::RasterVisibilityLost => {
                        track.raster_visibility_losses += 1
                    }
                    ObjectIdentityEvent::ParentChanged { .. }
                    | ObjectIdentityEvent::MaterialChanged { .. }
                    | ObjectIdentityEvent::KindChanged { .. } => {}
                }
            }
        }
    }

    let tracks = tracks
        .into_iter()
        .map(|(stable_id, value)| ObjectTrackSummary {
            stable_id,
            semantic_present_frames: value.semantic_present_frames,
            raster_visible_frames: value.raster_visible_frames,
            raster_visibility_acquisitions: value.raster_visibility_acquisitions,
            raster_visibility_losses: value.raster_visibility_losses,
            semantic_creations: value.semantic_creations,
            semantic_destructions: value.semantic_destructions,
            authored_visibility_enables: value.authored_visibility_enables,
            authored_visibility_disables: value.authored_visibility_disables,
            cumulative_world_translation_meters: value.cumulative_world_translation_meters,
            cumulative_world_rotation_radians: value.cumulative_world_rotation_radians,
            cumulative_screen_path_normalized: value.cumulative_screen_path_normalized,
            maximum_visible_fraction: value.maximum_visible_fraction,
        })
        .collect();

    PersistentObjectWindowEvidence {
        frame_count: frames.len() as u32,
        transition_count: transitions.len() as u32,
        distinct_semantic_objects: semantic_ids.len() as u32,
        distinct_raster_observed_objects: raster_ids.len() as u32,
        camera_translation_meters,
        camera_rotation_radians,
        tracks,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectWindowError {
    RequiresAtLeastTwoFrames,
    InvalidMaxFrameGap,
    NonMonotonicFrames,
    FrameGapExceeded,
    CrossCameraWindow,
    Temporal(ObjectTemporalError),
}

impl std::fmt::Display for ObjectWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequiresAtLeastTwoFrames => write!(f, "object window requires at least two frames"),
            Self::InvalidMaxFrameGap => write!(f, "object window max frame gap must be non-zero"),
            Self::NonMonotonicFrames => write!(f, "object window frames must strictly increase"),
            Self::FrameGapExceeded => write!(f, "object window contains a frame gap beyond the frozen limit"),
            Self::CrossCameraWindow => write!(f, "object window mixes stable camera identities"),
            Self::Temporal(error) => write!(f, "object temporal evidence error: {error}"),
        }
    }
}

impl std::error::Error for ObjectWindowError {}
