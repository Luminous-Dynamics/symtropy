// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fidelity-aware multi-view and counterfactual observation contracts.
//!
//! Render fidelity is evidence provenance. A low-resolution preview and a
//! portfolio render may both be useful, but they must not be compared as if
//! they were interchangeable observations.

use std::collections::BTreeSet;

use crate::art_capture::{
    ArtCaptureError, ArtCapturePurpose, ArtCaptureReceipt,
};
use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderFidelityClass {
    InteractivePreview,
    CognitiveObservation,
    Portfolio,
    Diagnostic,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFidelity {
    pub class: RenderFidelityClass,
    pub width: u32,
    pub height: u32,
    /// Optional rendering sample budget. `None` means host-defined/default.
    pub samples_per_pixel: Option<u32>,
    /// Host-defined simulation/render profile identity (for example
    /// `full-physics`, `preview-physics`, or `raster-low`).
    pub profile: String,
}

impl RenderFidelity {
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.width == 0 || self.height == 0 {
            return Err(ObservationError::InvalidFidelityResolution);
        }
        if self.profile.trim().is_empty() {
            return Err(ObservationError::EmptyFidelityProfile);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FidelityTaggedCapture {
    pub receipt: ArtCaptureReceipt,
    pub fidelity: RenderFidelity,
}

impl FidelityTaggedCapture {
    pub fn validate(&self) -> Result<(), ObservationError> {
        self.receipt
            .validate_alignment()
            .map_err(ObservationError::Capture)?;
        self.fidelity.validate()?;
        if self.receipt.request.width != self.fidelity.width
            || self.receipt.request.height != self.fidelity.height
        {
            return Err(ObservationError::FidelityResolutionMismatch);
        }
        Ok(())
    }
}

/// Baseline plus candidate renders that are safe to compare directly because
/// revision, frame, scene identity, and fidelity all match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignedCounterfactualObservationSet {
    pub baseline: FidelityTaggedCapture,
    pub candidates: Vec<FidelityTaggedCapture>,
}

impl AlignedCounterfactualObservationSet {
    pub fn validate(&self) -> Result<(), ObservationError> {
        self.baseline.validate()?;
        if self.baseline.receipt.request.purpose != ArtCapturePurpose::CommittedObservation {
            return Err(ObservationError::BaselinePurposeMismatch);
        }
        for candidate in &self.candidates {
            candidate.validate()?;
            if candidate.receipt.request.purpose != ArtCapturePurpose::CounterfactualPreview {
                return Err(ObservationError::CandidatePurposeMismatch);
            }
            require_same_observation_plane(&self.baseline, candidate)?;
        }
        Ok(())
    }
}

/// Synchronized captures from multiple cameras/views of the same artistic
/// moment. Camera IDs must be present and unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynchronizedViewSet {
    pub revision_id: String,
    pub frame: StudioFrame,
    pub captures: Vec<FidelityTaggedCapture>,
}

impl SynchronizedViewSet {
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.captures.is_empty() {
            return Err(ObservationError::EmptyViewSet);
        }
        let mut cameras = BTreeSet::new();
        let fidelity = self.captures[0].fidelity.clone();
        let scene_hash = self.captures[0].receipt.observed_scene_hash.clone();
        for capture in &self.captures {
            capture.validate()?;
            if capture.receipt.observed_revision_id != self.revision_id
                || capture.receipt.observed_frame != self.frame
                || capture.receipt.observed_scene_hash != scene_hash
                || capture.fidelity != fidelity
            {
                return Err(ObservationError::CrossPlaneViewSet);
            }
            let camera = capture
                .receipt
                .request
                .camera_stable_id
                .as_ref()
                .ok_or(ObservationError::MissingCameraIdentity)?;
            if !cameras.insert(camera.as_str()) {
                return Err(ObservationError::DuplicateCameraIdentity(camera.clone()));
            }
        }
        Ok(())
    }
}

/// Time-ordered observation history for one camera/fidelity class. This keeps
/// motion evidence explicit and rejects accidental frame duplication/reordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalCaptureWindow {
    pub camera_stable_id: String,
    pub captures: Vec<FidelityTaggedCapture>,
}

impl TemporalCaptureWindow {
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.camera_stable_id.trim().is_empty() {
            return Err(ObservationError::MissingCameraIdentity);
        }
        if self.captures.is_empty() {
            return Err(ObservationError::EmptyTemporalWindow);
        }
        let fidelity = self.captures[0].fidelity.clone();
        let mut previous = None;
        for capture in &self.captures {
            capture.validate()?;
            if capture.fidelity != fidelity {
                return Err(ObservationError::MixedFidelityTemporalWindow);
            }
            if capture.receipt.request.camera_stable_id.as_deref()
                != Some(self.camera_stable_id.as_str())
            {
                return Err(ObservationError::CrossCameraTemporalWindow);
            }
            if previous.is_some_and(|frame| capture.receipt.observed_frame <= frame) {
                return Err(ObservationError::NonMonotonicTemporalFrames);
            }
            previous = Some(capture.receipt.observed_frame);
        }
        Ok(())
    }
}

fn require_same_observation_plane(
    left: &FidelityTaggedCapture,
    right: &FidelityTaggedCapture,
) -> Result<(), ObservationError> {
    if left.receipt.observed_revision_id != right.receipt.observed_revision_id
        || left.receipt.observed_frame != right.receipt.observed_frame
        || left.receipt.observed_scene_hash != right.receipt.observed_scene_hash
    {
        return Err(ObservationError::CounterfactualAlignmentMismatch);
    }
    if left.fidelity != right.fidelity {
        return Err(ObservationError::CounterfactualFidelityMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    Capture(ArtCaptureError),
    InvalidFidelityResolution,
    EmptyFidelityProfile,
    FidelityResolutionMismatch,
    BaselinePurposeMismatch,
    CandidatePurposeMismatch,
    CounterfactualAlignmentMismatch,
    CounterfactualFidelityMismatch,
    EmptyViewSet,
    CrossPlaneViewSet,
    MissingCameraIdentity,
    DuplicateCameraIdentity(String),
    EmptyTemporalWindow,
    MixedFidelityTemporalWindow,
    CrossCameraTemporalWindow,
    NonMonotonicTemporalFrames,
}

impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture error: {error}"),
            Self::InvalidFidelityResolution => write!(f, "fidelity resolution must be non-zero"),
            Self::EmptyFidelityProfile => write!(f, "fidelity profile may not be empty"),
            Self::FidelityResolutionMismatch => {
                write!(f, "capture resolution does not match fidelity receipt")
            }
            Self::BaselinePurposeMismatch => write!(f, "counterfactual baseline must be committed observation"),
            Self::CandidatePurposeMismatch => write!(f, "counterfactual candidate must be preview observation"),
            Self::CounterfactualAlignmentMismatch => write!(f, "counterfactual captures differ in revision/frame/scene"),
            Self::CounterfactualFidelityMismatch => write!(f, "counterfactual captures differ in render fidelity"),
            Self::EmptyViewSet => write!(f, "synchronized view set may not be empty"),
            Self::CrossPlaneViewSet => write!(f, "synchronized views do not share one observation plane"),
            Self::MissingCameraIdentity => write!(f, "camera stable identity is required"),
            Self::DuplicateCameraIdentity(id) => write!(f, "duplicate camera identity: {id}"),
            Self::EmptyTemporalWindow => write!(f, "temporal capture window may not be empty"),
            Self::MixedFidelityTemporalWindow => write!(f, "temporal capture window mixes render fidelity"),
            Self::CrossCameraTemporalWindow => write!(f, "temporal capture window mixes camera identity"),
            Self::NonMonotonicTemporalFrames => write!(f, "temporal capture frames must be strictly increasing"),
        }
    }
}

impl std::error::Error for ObservationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_capture::{ArtCaptureRequest, ArtRenderChannel};

    fn tagged(
        id: &str,
        frame: u64,
        camera: &str,
        purpose: ArtCapturePurpose,
        fidelity: RenderFidelity,
    ) -> FidelityTaggedCapture {
        let request = ArtCaptureRequest {
            capture_id: id.into(),
            revision_id: "r1".into(),
            frame: StudioFrame(frame),
            scene_hash: "scene".into(),
            camera_stable_id: Some(camera.into()),
            width: fidelity.width,
            height: fidelity.height,
            purpose,
            channels: vec![ArtRenderChannel::Color],
        };
        FidelityTaggedCapture {
            receipt: ArtCaptureReceipt {
                request,
                observed_revision_id: "r1".into(),
                observed_frame: StudioFrame(frame),
                observed_scene_hash: "scene".into(),
                artifact_locator: format!("memory://{id}"),
                artifact_digest: Some(format!("digest-{id}")),
            },
            fidelity,
        }
    }

    fn fidelity() -> RenderFidelity {
        RenderFidelity {
            class: RenderFidelityClass::CognitiveObservation,
            width: 320,
            height: 180,
            samples_per_pixel: None,
            profile: "raster-cognitive-v1".into(),
        }
    }

    #[test]
    fn counterfactual_comparison_rejects_mixed_fidelity() {
        let baseline = tagged(
            "base",
            1,
            "camera",
            ArtCapturePurpose::CommittedObservation,
            fidelity(),
        );
        let mut other = fidelity();
        other.profile = "portfolio".into();
        let candidate = tagged(
            "candidate",
            1,
            "camera",
            ArtCapturePurpose::CounterfactualPreview,
            other,
        );
        let set = AlignedCounterfactualObservationSet {
            baseline,
            candidates: vec![candidate],
        };
        assert_eq!(
            set.validate(),
            Err(ObservationError::CounterfactualFidelityMismatch)
        );
    }

    #[test]
    fn synchronized_views_require_unique_camera_identity() {
        let f = fidelity();
        let set = SynchronizedViewSet {
            revision_id: "r1".into(),
            frame: StudioFrame(1),
            captures: vec![
                tagged("a", 1, "camera", ArtCapturePurpose::CommittedObservation, f.clone()),
                tagged("b", 1, "camera", ArtCapturePurpose::CommittedObservation, f),
            ],
        };
        assert!(matches!(
            set.validate(),
            Err(ObservationError::DuplicateCameraIdentity(_))
        ));
    }

    #[test]
    fn temporal_window_rejects_reordered_frames() {
        let f = fidelity();
        let window = TemporalCaptureWindow {
            camera_stable_id: "camera".into(),
            captures: vec![
                tagged("a", 2, "camera", ArtCapturePurpose::CommittedObservation, f.clone()),
                tagged("b", 1, "camera", ArtCapturePurpose::CommittedObservation, f),
            ],
        };
        assert_eq!(
            window.validate(),
            Err(ObservationError::NonMonotonicTemporalFrames)
        );
    }
}
