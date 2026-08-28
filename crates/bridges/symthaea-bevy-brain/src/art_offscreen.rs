// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single-frame off-screen rendering and asynchronous GPU readback for art observation.
//!
//! The central provenance rule is that a capture gets a dedicated render target.
//! The target is rendered for one host frame, detached from the camera, and only
//! then queued for GPU readback. Because the image is no longer a live camera
//! target when readback begins, later frames cannot silently overwrite the
//! evidence bytes.

use bevy::{
    asset::{Assets, Handle},
    camera::{Camera, RenderTarget},
    image::Image,
    prelude::*,
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{TextureFormat, TextureUsages},
    },
};
use std::collections::VecDeque;

use crate::art_capture::{
    ArtCaptureError, ArtCaptureReceipt, ArtCaptureRequest, ArtRenderChannel,
};
use crate::art_timeline::StudioFrame;

/// Host-side stamp captured when the dedicated render target is armed.
///
/// These fields are intentionally independent from the request so a host cannot
/// manufacture alignment merely by copying request identity at receipt time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtRenderStamp {
    pub revision_id: String,
    pub frame: StudioFrame,
    pub scene_hash: String,
    pub camera_stable_id: Option<String>,
    pub render_epoch: u64,
}

impl ArtRenderStamp {
    pub fn validate_against(&self, request: &ArtCaptureRequest) -> Result<(), ArtOffscreenError> {
        request.validate().map_err(ArtOffscreenError::Capture)?;
        if self.revision_id != request.revision_id {
            return Err(ArtOffscreenError::RevisionMismatch);
        }
        if self.frame != request.frame {
            return Err(ArtOffscreenError::FrameMismatch);
        }
        if self.scene_hash != request.scene_hash {
            return Err(ArtOffscreenError::SceneHashMismatch);
        }
        if let Some(expected) = request.camera_stable_id.as_deref() {
            if self.camera_stable_id.as_deref() != Some(expected) {
                return Err(ArtOffscreenError::CameraMismatch);
            }
        }
        Ok(())
    }
}

/// Prepared, dedicated image target for exactly one artistic capture.
///
/// The type is intentionally not `Clone`: one request should have one target
/// lineage. After one rendered frame, call [`PreparedArtCaptureTarget::finish_render`]
/// to restore the camera and obtain a readback-ready target.
#[derive(Debug)]
pub struct PreparedArtCaptureTarget {
    request: ArtCaptureRequest,
    stamp: ArtRenderStamp,
    image: Handle<Image>,
    previous_target: RenderTarget,
    format: TextureFormat,
}

impl PreparedArtCaptureTarget {
    pub fn arm(
        images: &mut Assets<Image>,
        camera: &mut Camera,
        request: ArtCaptureRequest,
        stamp: ArtRenderStamp,
        format: TextureFormat,
    ) -> Result<Self, ArtOffscreenError> {
        stamp.validate_against(&request)?;
        if request.channels != [ArtRenderChannel::Color] {
            return Err(ArtOffscreenError::UnsupportedChannels);
        }

        let mut image = Image::new_target_texture(request.width, request.height, format, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let image = images.add(image);

        let previous_target = camera.target.clone();
        camera.target = image.clone().into();

        Ok(Self {
            request,
            stamp,
            image,
            previous_target,
            format,
        })
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn render_epoch(&self) -> u64 {
        self.stamp.render_epoch
    }

    /// Restore the camera after the host has allowed exactly one render pass to
    /// target this image. The returned target can no longer retarget a camera;
    /// it only permits readback.
    pub fn finish_render(self, camera: &mut Camera) -> RenderedArtCaptureTarget {
        camera.target = self.previous_target;
        RenderedArtCaptureTarget {
            request: self.request,
            stamp: self.stamp,
            image: self.image,
            format: self.format,
        }
    }
}

/// Dedicated render image after it has been detached from the camera.
#[derive(Debug)]
pub struct RenderedArtCaptureTarget {
    request: ArtCaptureRequest,
    stamp: ArtRenderStamp,
    image: Handle<Image>,
    format: TextureFormat,
}

impl RenderedArtCaptureTarget {
    pub fn request(&self) -> &ArtCaptureRequest {
        &self.request
    }

    pub fn queue_readback(self, commands: &mut Commands) -> Entity {
        let pending = PendingArtReadback {
            request: self.request,
            stamp: self.stamp,
            format: self.format,
        };
        commands
            .spawn((Readback::texture(self.image), pending))
            .observe(complete_art_readback)
            .id()
    }
}

#[derive(Component, Debug)]
struct PendingArtReadback {
    request: ArtCaptureRequest,
    stamp: ArtRenderStamp,
    format: TextureFormat,
}

/// Completed raw GPU bytes plus their revision/frame-bound receipt.
#[derive(Debug)]
pub struct ArtGpuReadback {
    pub receipt: ArtCaptureReceipt,
    pub format: TextureFormat,
    pub bytes: Vec<u8>,
    pub render_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtGpuReadbackEnqueueReceipt {
    pub accepted: bool,
    pub evicted_capture: bool,
    pub dropped_total: u64,
}

/// Bounded completed-readback queue. Large frame buffers must never grow without
/// bound simply because perception is slower than rendering.
#[derive(Resource, Debug)]
pub struct ArtGpuReadbackQueue {
    capacity: usize,
    completed: VecDeque<ArtGpuReadback>,
    dropped_total: u64,
}

impl ArtGpuReadbackQueue {
    pub fn new(capacity: usize) -> Result<Self, ArtOffscreenError> {
        if capacity == 0 {
            return Err(ArtOffscreenError::ZeroCompletedCapacity);
        }
        Ok(Self {
            capacity,
            completed: VecDeque::with_capacity(capacity),
            dropped_total: 0,
        })
    }

    pub fn push(&mut self, capture: ArtGpuReadback) -> ArtGpuReadbackEnqueueReceipt {
        let evicted_capture = if self.completed.len() == self.capacity {
            self.completed.pop_front();
            self.dropped_total = self.dropped_total.saturating_add(1);
            true
        } else {
            false
        };
        self.completed.push_back(capture);
        ArtGpuReadbackEnqueueReceipt {
            accepted: true,
            evicted_capture,
            dropped_total: self.dropped_total,
        }
    }

    pub fn pop_next(&mut self) -> Option<ArtGpuReadback> {
        self.completed.pop_front()
    }

    pub fn len(&self) -> usize {
        self.completed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.completed.is_empty()
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_total
    }
}

impl Default for ArtGpuReadbackQueue {
    fn default() -> Self {
        Self::new(8).expect("default completed art readback capacity is non-zero")
    }
}

fn complete_art_readback(
    event: On<ReadbackComplete>,
    pending: Query<&PendingArtReadback>,
    mut completed: ResMut<ArtGpuReadbackQueue>,
    mut commands: Commands,
) {
    let Ok(pending) = pending.get(event.entity) else {
        return;
    };

    let bytes = event.data.clone();
    let digest = raw_bytes_digest(&bytes);
    let receipt = ArtCaptureReceipt {
        request: pending.request.clone(),
        observed_revision_id: pending.stamp.revision_id.clone(),
        observed_frame: pending.stamp.frame,
        observed_scene_hash: pending.stamp.scene_hash.clone(),
        artifact_locator: format!("memory://bevy-gpu-readback/{}", pending.request.capture_id),
        artifact_digest: Some(digest),
    };

    completed.push(ArtGpuReadback {
        receipt,
        format: pending.format,
        bytes,
        render_epoch: pending.stamp.render_epoch,
    });

    // `Readback` repeats every frame while its entity exists. This is a one-shot
    // artistic observation, so the entity is removed immediately after success.
    commands.entity(event.entity).despawn();
}

fn raw_bytes_digest(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtOffscreenError {
    Capture(ArtCaptureError),
    RevisionMismatch,
    FrameMismatch,
    SceneHashMismatch,
    CameraMismatch,
    UnsupportedChannels,
    ZeroCompletedCapacity,
}

impl std::fmt::Display for ArtOffscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture request error: {error}"),
            Self::RevisionMismatch => write!(f, "render stamp revision differs from request"),
            Self::FrameMismatch => write!(f, "render stamp frame differs from request"),
            Self::SceneHashMismatch => write!(f, "render stamp scene hash differs from request"),
            Self::CameraMismatch => write!(f, "render stamp camera differs from request"),
            Self::UnsupportedChannels => write!(f, "RT2 currently supports color readback only"),
            Self::ZeroCompletedCapacity => write!(f, "completed readback capacity must be non-zero"),
        }
    }
}

impl std::error::Error for ArtOffscreenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_capture::ArtCapturePurpose;

    fn request() -> ArtCaptureRequest {
        ArtCaptureRequest {
            capture_id: "capture-1".into(),
            revision_id: "r7".into(),
            frame: StudioFrame(42),
            scene_hash: "scene-7".into(),
            camera_stable_id: Some("camera-a".into()),
            width: 320,
            height: 180,
            purpose: ArtCapturePurpose::CommittedObservation,
            channels: vec![ArtRenderChannel::Color],
        }
    }

    fn stamp() -> ArtRenderStamp {
        ArtRenderStamp {
            revision_id: "r7".into(),
            frame: StudioFrame(42),
            scene_hash: "scene-7".into(),
            camera_stable_id: Some("camera-a".into()),
            render_epoch: 9,
        }
    }

    #[test]
    fn stamp_must_independently_match_request() {
        let mut bad = stamp();
        bad.frame = StudioFrame(43);
        assert_eq!(
            bad.validate_against(&request()),
            Err(ArtOffscreenError::FrameMismatch)
        );
    }

    #[test]
    fn raw_digest_is_deterministic_and_byte_sensitive() {
        assert_eq!(raw_bytes_digest(b"abc"), raw_bytes_digest(b"abc"));
        assert_ne!(raw_bytes_digest(b"abc"), raw_bytes_digest(b"abd"));
    }

    #[test]
    fn completed_queue_is_bounded_and_reports_eviction() {
        let mut queue = ArtGpuReadbackQueue::new(1).unwrap();
        let make = |id: &str| ArtGpuReadback {
            receipt: ArtCaptureReceipt {
                request: ArtCaptureRequest {
                    capture_id: id.into(),
                    ..request()
                },
                observed_revision_id: "r7".into(),
                observed_frame: StudioFrame(42),
                observed_scene_hash: "scene-7".into(),
                artifact_locator: format!("memory://{id}"),
                artifact_digest: None,
            },
            format: TextureFormat::Rgba8UnormSrgb,
            bytes: vec![1, 2, 3, 4],
            render_epoch: 1,
        };
        queue.push(make("a"));
        let receipt = queue.push(make("b"));
        assert!(receipt.evicted_capture);
        assert_eq!(receipt.dropped_total, 1);
        assert_eq!(queue.pop_next().unwrap().receipt.request.capture_id, "b");
    }
}
