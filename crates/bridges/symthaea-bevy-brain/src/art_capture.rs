// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Revision/frame-bound render capture requests and bounded queueing.
//!
//! Actual GPU readback is intentionally delegated to a host adapter. This
//! module makes capture identity, backpressure, and evidence alignment explicit
//! so real-time video observation cannot silently drift away from the scene it
//! claims to represent.

use bevy::prelude::*;
use std::collections::VecDeque;

use crate::art_port::ArtPerceptionFrame;
use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtCapturePurpose {
    CommittedObservation,
    CounterfactualPreview,
    PortfolioFrame,
    Diagnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtRenderChannel {
    Color,
    Depth,
    Normals,
    ObjectId,
    Motion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtCaptureRequest {
    pub capture_id: String,
    pub revision_id: String,
    pub frame: StudioFrame,
    pub scene_hash: String,
    pub camera_stable_id: Option<String>,
    pub width: u32,
    pub height: u32,
    pub purpose: ArtCapturePurpose,
    pub channels: Vec<ArtRenderChannel>,
}

impl ArtCaptureRequest {
    pub fn from_perception(
        capture_id: impl Into<String>,
        perception: &ArtPerceptionFrame,
        frame: StudioFrame,
        width: u32,
        height: u32,
        purpose: ArtCapturePurpose,
    ) -> Self {
        Self {
            capture_id: capture_id.into(),
            revision_id: perception.revision_id.clone(),
            frame,
            scene_hash: perception.content_hash.clone(),
            camera_stable_id: None,
            width,
            height,
            purpose,
            channels: vec![ArtRenderChannel::Color],
        }
    }

    pub fn validate(&self) -> Result<(), ArtCaptureError> {
        if self.capture_id.trim().is_empty() {
            return Err(ArtCaptureError::EmptyCaptureId);
        }
        if self.revision_id.trim().is_empty() || self.scene_hash.trim().is_empty() {
            return Err(ArtCaptureError::MissingSceneIdentity);
        }
        if self.width == 0 || self.height == 0 {
            return Err(ArtCaptureError::InvalidResolution);
        }
        if self.channels.is_empty() {
            return Err(ArtCaptureError::NoRenderChannels);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtCaptureReceipt {
    pub request: ArtCaptureRequest,
    pub observed_revision_id: String,
    pub observed_frame: StudioFrame,
    pub observed_scene_hash: String,
    pub artifact_locator: String,
    pub artifact_digest: Option<String>,
}

impl ArtCaptureReceipt {
    pub fn validate_alignment(&self) -> Result<(), ArtCaptureError> {
        self.request.validate()?;
        if self.request.revision_id != self.observed_revision_id {
            return Err(ArtCaptureError::RevisionMismatch {
                requested: self.request.revision_id.clone(),
                observed: self.observed_revision_id.clone(),
            });
        }
        if self.request.frame != self.observed_frame {
            return Err(ArtCaptureError::FrameMismatch {
                requested: self.request.frame,
                observed: self.observed_frame,
            });
        }
        if self.request.scene_hash != self.observed_scene_hash {
            return Err(ArtCaptureError::SceneHashMismatch);
        }
        if self.artifact_locator.trim().is_empty() {
            return Err(ArtCaptureError::MissingArtifactLocator);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtCaptureOverflowPolicy {
    RejectNewest,
    EvictOldest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtCaptureEnqueueReceipt {
    pub accepted: bool,
    pub rejected_capture_id: Option<String>,
    pub evicted_capture_id: Option<String>,
    pub dropped_total: u64,
}

#[derive(Resource, Debug, Clone)]
pub struct ArtCaptureQueue {
    capacity: usize,
    policy: ArtCaptureOverflowPolicy,
    pending: VecDeque<ArtCaptureRequest>,
    dropped_total: u64,
}

impl ArtCaptureQueue {
    pub fn new(
        capacity: usize,
        policy: ArtCaptureOverflowPolicy,
    ) -> Result<Self, ArtCaptureError> {
        if capacity == 0 {
            return Err(ArtCaptureError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            policy,
            pending: VecDeque::with_capacity(capacity),
            dropped_total: 0,
        })
    }

    pub fn enqueue(
        &mut self,
        request: ArtCaptureRequest,
    ) -> Result<ArtCaptureEnqueueReceipt, ArtCaptureError> {
        request.validate()?;
        if self
            .pending
            .iter()
            .any(|pending| pending.capture_id == request.capture_id)
        {
            return Err(ArtCaptureError::DuplicateCaptureId(request.capture_id));
        }

        if self.pending.len() < self.capacity {
            self.pending.push_back(request);
            return Ok(ArtCaptureEnqueueReceipt {
                accepted: true,
                rejected_capture_id: None,
                evicted_capture_id: None,
                dropped_total: self.dropped_total,
            });
        }

        self.dropped_total = self.dropped_total.saturating_add(1);
        match self.policy {
            ArtCaptureOverflowPolicy::RejectNewest => Ok(ArtCaptureEnqueueReceipt {
                accepted: false,
                rejected_capture_id: Some(request.capture_id),
                evicted_capture_id: None,
                dropped_total: self.dropped_total,
            }),
            ArtCaptureOverflowPolicy::EvictOldest => {
                let evicted = self.pending.pop_front().map(|request| request.capture_id);
                self.pending.push_back(request);
                Ok(ArtCaptureEnqueueReceipt {
                    accepted: true,
                    rejected_capture_id: None,
                    evicted_capture_id: evicted,
                    dropped_total: self.dropped_total,
                })
            }
        }
    }

    pub fn pop_next(&mut self) -> Option<ArtCaptureRequest> {
        self.pending.pop_front()
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_total
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtCaptureError {
    EmptyCaptureId,
    MissingSceneIdentity,
    InvalidResolution,
    NoRenderChannels,
    ZeroCapacity,
    DuplicateCaptureId(String),
    RevisionMismatch { requested: String, observed: String },
    FrameMismatch { requested: StudioFrame, observed: StudioFrame },
    SceneHashMismatch,
    MissingArtifactLocator,
}

impl std::fmt::Display for ArtCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCaptureId => write!(f, "capture id may not be empty"),
            Self::MissingSceneIdentity => write!(f, "capture request lacks revision/scene identity"),
            Self::InvalidResolution => write!(f, "capture resolution must be non-zero"),
            Self::NoRenderChannels => write!(f, "capture request needs at least one render channel"),
            Self::ZeroCapacity => write!(f, "capture queue capacity must be non-zero"),
            Self::DuplicateCaptureId(id) => write!(f, "duplicate capture id: {id}"),
            Self::RevisionMismatch { requested, observed } => write!(
                f,
                "capture revision mismatch: requested {requested}, observed {observed}"
            ),
            Self::FrameMismatch { requested, observed } => write!(
                f,
                "capture frame mismatch: requested {:?}, observed {:?}",
                requested, observed
            ),
            Self::SceneHashMismatch => write!(f, "capture scene hash does not match request"),
            Self::MissingArtifactLocator => write!(f, "capture receipt has no artifact locator"),
        }
    }
}

impl std::error::Error for ArtCaptureError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, frame: u64) -> ArtCaptureRequest {
        ArtCaptureRequest {
            capture_id: id.into(),
            revision_id: "r1".into(),
            frame: StudioFrame(frame),
            scene_hash: "scene".into(),
            camera_stable_id: None,
            width: 320,
            height: 180,
            purpose: ArtCapturePurpose::CommittedObservation,
            channels: vec![ArtRenderChannel::Color],
        }
    }

    #[test]
    fn receipt_rejects_pixels_from_wrong_frame() {
        let receipt = ArtCaptureReceipt {
            request: request("c1", 4),
            observed_revision_id: "r1".into(),
            observed_frame: StudioFrame(5),
            observed_scene_hash: "scene".into(),
            artifact_locator: "memory://frame".into(),
            artifact_digest: None,
        };
        assert!(matches!(
            receipt.validate_alignment(),
            Err(ArtCaptureError::FrameMismatch { .. })
        ));
    }

    #[test]
    fn reject_newest_is_explicit_not_silent() {
        let mut queue =
            ArtCaptureQueue::new(1, ArtCaptureOverflowPolicy::RejectNewest).unwrap();
        queue.enqueue(request("c1", 1)).unwrap();
        let receipt = queue.enqueue(request("c2", 2)).unwrap();
        assert!(!receipt.accepted);
        assert_eq!(receipt.rejected_capture_id.as_deref(), Some("c2"));
        assert_eq!(receipt.dropped_total, 1);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn evict_oldest_reports_exact_capture() {
        let mut queue =
            ArtCaptureQueue::new(1, ArtCaptureOverflowPolicy::EvictOldest).unwrap();
        queue.enqueue(request("c1", 1)).unwrap();
        let receipt = queue.enqueue(request("c2", 2)).unwrap();
        assert!(receipt.accepted);
        assert_eq!(receipt.evicted_capture_id.as_deref(), Some("c1"));
        assert_eq!(queue.pop_next().unwrap().capture_id, "c2");
    }
}
