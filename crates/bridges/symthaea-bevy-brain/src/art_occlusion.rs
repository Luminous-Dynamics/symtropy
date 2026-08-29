// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative object/depth occlusion evidence for ARTIST-EYE-v1E.
//!
//! A raster visibility transition is upgraded to depth-takeover support only
//! when the tracked target remains semantically present/authored-visible, its
//! semantic transform is stable within prospectively supplied bounds, the
//! camera is stable within prospectively supplied bounds, and another object
//! occupies enough of the target's corresponding pixel support at a
//! consistently closer metric depth.
//!
//! The thresholds are intentionally caller-supplied with no confirmatory
//! defaults. This module never turns visibility loss/gain into causal language
//! merely because an object disappeared/reappeared in the raster.

use std::collections::BTreeMap;

use crate::{
    art_object_depth::ObjectDepthFusionFrame,
    art_object_id::ObjectIdRegistry,
    art_object_temporal::{ObjectIdentityEvent, ObjectIdentityTransition, PersistentObjectTransition},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OcclusionSupportThresholds {
    pub minimum_takeover_fraction: f64,
    pub minimum_closer_fraction: f64,
    pub minimum_depth_margin_meters: f32,
    pub maximum_target_translation: f64,
    pub maximum_target_rotation: f64,
    pub maximum_target_scale_delta: f64,
    pub maximum_camera_translation: f64,
    pub maximum_camera_rotation: f64,
}

impl OcclusionSupportThresholds {
    pub fn validate(self) -> Result<Self, OcclusionEvidenceError> {
        let unit = [self.minimum_takeover_fraction, self.minimum_closer_fraction];
        let nonnegative = [
            self.maximum_target_translation,
            self.maximum_target_rotation,
            self.maximum_target_scale_delta,
            self.maximum_camera_translation,
            self.maximum_camera_rotation,
        ];
        if unit.iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || !self.minimum_depth_margin_meters.is_finite()
            || self.minimum_depth_margin_meters < 0.0
            || nonnegative
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(OcclusionEvidenceError::InvalidThresholds);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcclusionTransitionKind {
    VisibilityLoss,
    VisibilityGain,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OccluderCandidateEvidence {
    pub stable_id: String,
    pub raster_id: u32,
    pub takeover_pixels: u64,
    pub depth_comparable_pixels: u64,
    pub closer_pixels: u64,
    pub takeover_fraction: f64,
    pub closer_fraction: f64,
    pub median_positive_depth_margin_meters: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OcclusionAssessment {
    NotAQualifyingVisibilityTransition,
    StabilityPreconditionsFailed,
    NoCompetingObjectTakeover,
    BelowProspectiveSupportThresholds {
        best_candidate: OccluderCandidateEvidence,
    },
    DepthTakeoverSupported {
        kind: OcclusionTransitionKind,
        target_stable_id: String,
        occluder: OccluderCandidateEvidence,
    },
}

pub fn assess_depth_takeover(
    transition: &ObjectIdentityTransition,
    from: &ObjectDepthFusionFrame,
    to: &ObjectDepthFusionFrame,
    registry: &ObjectIdRegistry,
    target_stable_id: &str,
    thresholds: OcclusionSupportThresholds,
) -> Result<OcclusionAssessment, OcclusionEvidenceError> {
    let thresholds = thresholds.validate()?;
    validate_alignment(transition, from, to, registry)?;
    let target = transition
        .object(target_stable_id)
        .ok_or_else(|| OcclusionEvidenceError::TargetMissing(target_stable_id.to_owned()))?;

    let Some(kind) = qualifying_transition(target) else {
        return Ok(OcclusionAssessment::NotAQualifyingVisibilityTransition);
    };
    if !stability_preconditions(transition, target, thresholds) {
        return Ok(OcclusionAssessment::StabilityPreconditionsFailed);
    }

    let target_raster = registry
        .raster_id(target_stable_id)
        .ok_or_else(|| OcclusionEvidenceError::TargetMissingFromRegistry(target_stable_id.to_owned()))?;

    // Both directions use the frame in which the target is visible as the
    // support mask and the opposite frame as the competing/occluder plane. A
    // positive target_depth - candidate_depth therefore has the same meaning
    // for loss and gain: the competing object was physically closer.
    let evidence = match kind {
        OcclusionTransitionKind::VisibilityLoss => takeover_candidates(
            from,
            to,
            target_raster,
            registry,
            thresholds.minimum_depth_margin_meters,
        )?,
        OcclusionTransitionKind::VisibilityGain => takeover_candidates(
            to,
            from,
            target_raster,
            registry,
            thresholds.minimum_depth_margin_meters,
        )?,
    };

    let Some(best) = evidence.into_iter().max_by(|a, b| {
        a.takeover_pixels
            .cmp(&b.takeover_pixels)
            .then_with(|| b.raster_id.cmp(&a.raster_id))
    }) else {
        return Ok(OcclusionAssessment::NoCompetingObjectTakeover);
    };

    if best.takeover_fraction >= thresholds.minimum_takeover_fraction
        && best.closer_fraction >= thresholds.minimum_closer_fraction
    {
        Ok(OcclusionAssessment::DepthTakeoverSupported {
            kind,
            target_stable_id: target_stable_id.to_owned(),
            occluder: best,
        })
    } else {
        Ok(OcclusionAssessment::BelowProspectiveSupportThresholds {
            best_candidate: best,
        })
    }
}

fn qualifying_transition(target: &PersistentObjectTransition) -> Option<OcclusionTransitionKind> {
    if !(target.existed_from && target.existed_to) {
        return None;
    }
    if target.events.iter().any(|event| {
        matches!(
            event,
            ObjectIdentityEvent::SemanticCreated
                | ObjectIdentityEvent::SemanticDestroyed
                | ObjectIdentityEvent::AuthoredVisibilityEnabled
                | ObjectIdentityEvent::AuthoredVisibilityDisabled
        )
    }) {
        return None;
    }
    match (target.raster_visible_from, target.raster_visible_to) {
        (true, false) => Some(OcclusionTransitionKind::VisibilityLoss),
        (false, true) => Some(OcclusionTransitionKind::VisibilityGain),
        _ => None,
    }
}

fn stability_preconditions(
    transition: &ObjectIdentityTransition,
    target: &PersistentObjectTransition,
    thresholds: OcclusionSupportThresholds,
) -> bool {
    let Some(transform) = target.semantic_transform_delta else {
        return false;
    };
    let Some(camera) = transition.camera_motion else {
        return false;
    };
    transform.translation_distance <= thresholds.maximum_target_translation
        && transform.rotation_radians <= thresholds.maximum_target_rotation
        && transform.scale_l2_delta <= thresholds.maximum_target_scale_delta
        && camera.translation_meters <= thresholds.maximum_camera_translation
        && camera.rotation_radians <= thresholds.maximum_camera_rotation
}

fn validate_alignment(
    transition: &ObjectIdentityTransition,
    from: &ObjectDepthFusionFrame,
    to: &ObjectDepthFusionFrame,
    registry: &ObjectIdRegistry,
) -> Result<(), OcclusionEvidenceError> {
    if from.width != to.width || from.height != to.height {
        return Err(OcclusionEvidenceError::ResolutionMismatch);
    }
    if from.registry_digest != registry.digest() || to.registry_digest != registry.digest() {
        return Err(OcclusionEvidenceError::RegistryDigestMismatch);
    }
    if transition.camera_stable_id != from.camera_stable_id
        || transition.camera_stable_id != to.camera_stable_id
        || transition.from_frame != from.frame
        || transition.to_frame != to.frame
        || transition.from_scene_hash != from.scene_hash
        || transition.to_scene_hash != to.scene_hash
        || transition.from_revision != from.revision_id
        || transition.to_revision != to.revision_id
    {
        return Err(OcclusionEvidenceError::TransitionFusionMisalignment);
    }
    Ok(())
}

#[derive(Default)]
struct CandidateAccumulator {
    takeover_pixels: u64,
    depth_comparable_pixels: u64,
    closer_pixels: u64,
    positive_margins: Vec<f32>,
}

fn takeover_candidates(
    target_visible_frame: &ObjectDepthFusionFrame,
    competing_frame: &ObjectDepthFusionFrame,
    target_raster: u32,
    registry: &ObjectIdRegistry,
    minimum_depth_margin_meters: f32,
) -> Result<Vec<OccluderCandidateEvidence>, OcclusionEvidenceError> {
    let mut target_pixels = 0u64;
    let mut by_candidate: BTreeMap<u32, CandidateAccumulator> = BTreeMap::new();

    for (target_pixel, competing_pixel) in target_visible_frame
        .pixels
        .iter()
        .zip(&competing_frame.pixels)
    {
        if target_pixel.raster_id != target_raster {
            continue;
        }
        target_pixels = target_pixels.saturating_add(1);
        let candidate_id = competing_pixel.raster_id;
        if candidate_id == 0 || candidate_id == target_raster {
            continue;
        }
        if registry.stable_id(candidate_id).is_none() {
            return Err(OcclusionEvidenceError::UnknownRasterId(candidate_id));
        }
        let accumulator = by_candidate.entry(candidate_id).or_default();
        accumulator.takeover_pixels = accumulator.takeover_pixels.saturating_add(1);
        if let (Some(target_depth), Some(candidate_depth)) =
            (target_pixel.depth_meters, competing_pixel.depth_meters)
        {
            accumulator.depth_comparable_pixels = accumulator.depth_comparable_pixels.saturating_add(1);
            let margin = target_depth - candidate_depth;
            if margin >= minimum_depth_margin_meters {
                accumulator.closer_pixels = accumulator.closer_pixels.saturating_add(1);
                accumulator.positive_margins.push(margin);
            }
        }
    }

    if target_pixels == 0 {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(by_candidate.len());
    for (raster_id, mut accumulator) in by_candidate {
        accumulator.positive_margins.sort_by(f32::total_cmp);
        let closer_fraction = if accumulator.depth_comparable_pixels == 0 {
            0.0
        } else {
            accumulator.closer_pixels as f64 / accumulator.depth_comparable_pixels as f64
        };
        let median = if accumulator.positive_margins.is_empty() {
            None
        } else {
            Some(f64::from(
                accumulator.positive_margins[accumulator.positive_margins.len() / 2],
            ))
        };
        out.push(OccluderCandidateEvidence {
            stable_id: registry
                .stable_id(raster_id)
                .ok_or(OcclusionEvidenceError::UnknownRasterId(raster_id))?
                .to_owned(),
            raster_id,
            takeover_pixels: accumulator.takeover_pixels,
            depth_comparable_pixels: accumulator.depth_comparable_pixels,
            closer_pixels: accumulator.closer_pixels,
            takeover_fraction: accumulator.takeover_pixels as f64 / target_pixels as f64,
            closer_fraction,
            median_positive_depth_margin_meters: median,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcclusionEvidenceError {
    InvalidThresholds,
    TargetMissing(String),
    TargetMissingFromRegistry(String),
    RegistryDigestMismatch,
    ResolutionMismatch,
    TransitionFusionMisalignment,
    UnknownRasterId(u32),
}

impl std::fmt::Display for OcclusionEvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidThresholds => write!(f, "occlusion support thresholds are invalid"),
            Self::TargetMissing(id) => write!(f, "target {id} is absent from object transition"),
            Self::TargetMissingFromRegistry(id) => write!(f, "target {id} is absent from frozen object registry"),
            Self::RegistryDigestMismatch => write!(f, "object/depth fusion registry differs from object transition lineage"),
            Self::ResolutionMismatch => write!(f, "object/depth fusion frames have different resolutions"),
            Self::TransitionFusionMisalignment => write!(f, "object transition and fused planes do not describe the same frames"),
            Self::UnknownRasterId(id) => write!(f, "occlusion evidence contains unknown raster ID {id}"),
        }
    }
}

impl std::error::Error for OcclusionEvidenceError {}
