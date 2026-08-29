// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bevy 0.19 depth acquisition for ARTIST-EYE-v1B/v1C.
//!
//! This adapter copies the exact `ViewDepthTexture` belonging to a marked 3D
//! camera into a dedicated `Depth32Float` image after the camera's main pass,
//! then uses Bevy's asynchronous GPU readback path. Raw Bevy device depth is
//! linearized on the CPU with explicit projection provenance before entering
//! the renderer-neutral [`crate::art_depth`] evidence layer.
//!
//! The adapter is intentionally two-phase: arm for one host render frame, then
//! detach/queue readback. This prevents a continuously reused depth texture from
//! silently changing underneath an evidence receipt.

#![cfg(feature = "realtime-art-render")]

use std::collections::VecDeque;

use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{schedule::Core3d, Core3dSystems},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_resource::{
            Extent3d, Origin3d, TexelCopyTextureInfo, TextureAspect, TextureDimension,
            TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, ViewQuery},
        texture::GpuImage,
        view::ViewDepthTexture,
        RenderApp,
    },
};

use crate::{
    art_capture::{ArtCaptureError, ArtCaptureReceipt, ArtCaptureRequest, ArtRenderChannel},
    art_depth::{ArtistDepthConfig, ArtistDepthError, ArtistDepthObservation, DepthPlaneEncoding},
};

/// Projection information required to convert Bevy reverse-Z device depth into
/// linear forward distance in world units/meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BevyDepthProjection {
    /// Bevy's standard perspective projection is infinite reverse-Z:
    /// `device_depth = near / linear_distance`.
    PerspectiveInfiniteReverseZ {
        near_meters: f32,
        /// Retained as culling/provenance metadata. It does not enter the
        /// infinite-reverse-Z reconstruction formula.
        culling_far_meters: f32,
    },
    /// Bevy orthographic depth is reverse-Z and linear over the declared range.
    OrthographicReverseZ {
        near_meters: f32,
        far_meters: f32,
    },
}

impl BevyDepthProjection {
    pub fn from_bevy_projection(projection: &Projection) -> Result<Self, ArtDepthReadbackError> {
        let value = match projection {
            Projection::Perspective(perspective) => Self::PerspectiveInfiniteReverseZ {
                near_meters: perspective.near,
                culling_far_meters: perspective.far,
            },
            Projection::Orthographic(orthographic) => Self::OrthographicReverseZ {
                near_meters: orthographic.near,
                far_meters: orthographic.far,
            },
            Projection::Custom(_) => return Err(ArtDepthReadbackError::UnsupportedProjection),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(self) -> Result<(), ArtDepthReadbackError> {
        match self {
            Self::PerspectiveInfiniteReverseZ {
                near_meters,
                culling_far_meters,
            } if near_meters.is_finite()
                && culling_far_meters.is_finite()
                && near_meters > 0.0
                && culling_far_meters > near_meters => Ok(()),
            Self::OrthographicReverseZ {
                near_meters,
                far_meters,
            } if near_meters.is_finite()
                && far_meters.is_finite()
                && far_meters > near_meters => Ok(()),
            _ => Err(ArtDepthReadbackError::InvalidProjection),
        }
    }

    /// Convert one NDC/device depth sample into linear positive distance.
    ///
    /// For perspective, zero is the infinite far background and remains
    /// missing rather than being turned into an arbitrary finite distance.
    pub fn linearize(self, device_depth: f32) -> Option<f32> {
        if !device_depth.is_finite() || !(0.0..=1.0).contains(&device_depth) {
            return None;
        }
        match self {
            Self::PerspectiveInfiniteReverseZ { near_meters, .. } => {
                (device_depth > f32::EPSILON).then_some(near_meters / device_depth)
            }
            Self::OrthographicReverseZ {
                near_meters,
                far_meters,
            } => Some(near_meters + (1.0 - device_depth) * (far_meters - near_meters)),
        }
    }
}

/// Extracted only for the one camera whose depth buffer should be copied this
/// render frame. The destination image is dedicated to this capture.
#[derive(Component, Clone, ExtractComponent, Debug)]
pub struct ArtDepthCopyTarget {
    pub capture_id: String,
    pub destination: Handle<Image>,
    pub width: u32,
    pub height: u32,
}

impl ArtDepthCopyTarget {
    fn validate(&self) -> bool {
        !self.capture_id.trim().is_empty() && self.width > 0 && self.height > 0
    }
}

/// Plugin that installs the render-world copy pass.
pub struct ArtDepthReadbackPlugin;

impl Plugin for ArtDepthReadbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<ArtDepthCopyTarget>::default())
            .init_resource::<ArtDepthGpuReadbackQueue>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Core3d,
            copy_art_depth_texture_system.after(Core3dSystems::MainPass),
        );
    }
}

/// Copy the marked camera's final main-pass depth attachment into a dedicated
/// `Depth32Float` image. Bevy 0.19 render work is a normal Core3d system.
fn copy_art_depth_texture_system(
    view: ViewQuery<(&ExtractedCamera, &ViewDepthTexture, &ArtDepthCopyTarget)>,
    image_assets: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
) {
    let (_camera, depth_texture, target) = view.into_inner();
    if !target.validate() {
        return;
    }
    let Some(destination) = image_assets.get(target.destination.id()) else {
        return;
    };

    let encoder = ctx.command_encoder();
    encoder.push_debug_group("symthaea art depth copy");
    encoder.copy_texture_to_texture(
        TexelCopyTextureInfo {
            texture: &depth_texture.texture,
            mip_level: 0,
            origin: Origin3d::default(),
            aspect: TextureAspect::All,
        },
        TexelCopyTextureInfo {
            texture: &destination.texture,
            mip_level: 0,
            origin: Origin3d::default(),
            aspect: TextureAspect::All,
        },
        Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
    );
    encoder.pop_debug_group();
}

/// One-frame armed depth capture. Keep this value until the host has allowed
/// exactly one Bevy render frame to execute, then call [`Self::finish_render`].
#[derive(Debug)]
pub struct PreparedArtDepthCapture {
    camera_entity: Entity,
    request: ArtCaptureRequest,
    projection: BevyDepthProjection,
    render_epoch: u64,
    destination: Handle<Image>,
    previous_depth_usages: bevy::camera::Camera3dDepthTextureUsage,
}

impl PreparedArtDepthCapture {
    pub fn arm(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        camera_entity: Entity,
        camera3d: &mut Camera3d,
        projection: &Projection,
        request: ArtCaptureRequest,
        render_epoch: u64,
    ) -> Result<Self, ArtDepthReadbackError> {
        request.validate().map_err(ArtDepthReadbackError::Capture)?;
        if !request.channels.contains(&ArtRenderChannel::Depth) {
            return Err(ArtDepthReadbackError::DepthChannelNotDeclared);
        }
        let projection = BevyDepthProjection::from_bevy_projection(projection)?;

        let previous_depth_usages = camera3d.depth_texture_usages;
        let mut usages: TextureUsages = previous_depth_usages.into();
        usages |= TextureUsages::COPY_SRC;
        camera3d.depth_texture_usages = usages.into();

        let mut image = Image::new_uninit(
            Extent3d {
                width: request.width,
                height: request.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Depth32Float,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage |= TextureUsages::COPY_DST | TextureUsages::COPY_SRC;
        let destination = images.add(image);

        commands.entity(camera_entity).insert(ArtDepthCopyTarget {
            capture_id: request.capture_id.clone(),
            destination: destination.clone(),
            width: request.width,
            height: request.height,
        });

        Ok(Self {
            camera_entity,
            request,
            projection,
            render_epoch,
            destination,
            previous_depth_usages,
        })
    }

    /// Detach after one host render frame. This prevents a later frame from
    /// overwriting the evidence texture before asynchronous readback.
    pub fn finish_render(
        self,
        commands: &mut Commands,
        camera3d: &mut Camera3d,
    ) -> RenderedArtDepthCapture {
        commands
            .entity(self.camera_entity)
            .remove::<ArtDepthCopyTarget>();
        camera3d.depth_texture_usages = self.previous_depth_usages;
        RenderedArtDepthCapture {
            request: self.request,
            projection: self.projection,
            render_epoch: self.render_epoch,
            destination: self.destination,
        }
    }
}

#[derive(Debug)]
pub struct RenderedArtDepthCapture {
    request: ArtCaptureRequest,
    projection: BevyDepthProjection,
    render_epoch: u64,
    destination: Handle<Image>,
}

impl RenderedArtDepthCapture {
    pub fn queue_readback(self, commands: &mut Commands) -> Entity {
        let pending = PendingArtDepthReadback {
            request: self.request,
            projection: self.projection,
            render_epoch: self.render_epoch,
        };
        commands
            .spawn((Readback::texture(self.destination), pending))
            .observe(complete_art_depth_readback)
            .id()
    }
}

#[derive(Component, Debug)]
struct PendingArtDepthReadback {
    request: ArtCaptureRequest,
    projection: BevyDepthProjection,
    render_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct ArtDepthGpuReadback {
    pub receipt: ArtCaptureReceipt,
    pub projection: BevyDepthProjection,
    /// Linear positive distance, with `NaN` used only to preserve missing raw
    /// samples across the padded row plane.
    pub linear_depth_meters: Vec<f32>,
    pub row_stride_values: usize,
    pub render_epoch: u64,
}

impl ArtDepthGpuReadback {
    pub fn into_artist_depth_observation(
        self,
        config: ArtistDepthConfig,
    ) -> Result<ArtistDepthObservation, ArtistDepthError> {
        ArtistDepthObservation::from_capture_f32(
            &self.receipt,
            &self.linear_depth_meters,
            self.row_stride_values,
            DepthPlaneEncoding::LinearMeters,
            config,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtDepthReadbackFailure {
    pub capture_id: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum ArtDepthReadbackOutcome {
    Completed(ArtDepthGpuReadback),
    Failed(ArtDepthReadbackFailure),
}

/// Bounded fail-closed completion queue. New evidence is rejected rather than
/// silently evicting older evidence. Any nonzero dropped count invalidates a
/// confirmatory episode.
#[derive(Resource, Debug)]
pub struct ArtDepthGpuReadbackQueue {
    capacity: usize,
    completed: VecDeque<ArtDepthReadbackOutcome>,
    dropped_total: u64,
}

impl ArtDepthGpuReadbackQueue {
    pub fn new(capacity: usize) -> Result<Self, ArtDepthReadbackError> {
        if capacity == 0 {
            return Err(ArtDepthReadbackError::ZeroCompletedCapacity);
        }
        Ok(Self {
            capacity,
            completed: VecDeque::with_capacity(capacity),
            dropped_total: 0,
        })
    }

    pub fn push(&mut self, outcome: ArtDepthReadbackOutcome) -> bool {
        if self.completed.len() == self.capacity {
            self.dropped_total = self.dropped_total.saturating_add(1);
            return false;
        }
        self.completed.push_back(outcome);
        true
    }

    pub fn pop_next(&mut self) -> Option<ArtDepthReadbackOutcome> {
        self.completed.pop_front()
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_total
    }

    pub fn len(&self) -> usize {
        self.completed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.completed.is_empty()
    }
}

impl Default for ArtDepthGpuReadbackQueue {
    fn default() -> Self {
        Self::new(8).expect("default depth readback queue capacity is non-zero")
    }
}

fn complete_art_depth_readback(
    event: On<ReadbackComplete>,
    pending: Query<&PendingArtDepthReadback>,
    mut completed: ResMut<ArtDepthGpuReadbackQueue>,
    mut commands: Commands,
) {
    let readback = event.event();
    let entity = readback.entity;
    let Ok(pending) = pending.get(entity) else {
        return;
    };

    let outcome = decode_readback(pending, &readback.data);
    completed.push(outcome);
    commands.entity(entity).despawn();
}

fn decode_readback(pending: &PendingArtDepthReadback, bytes: &[u8]) -> ArtDepthReadbackOutcome {
    let fail = |reason: &str| {
        ArtDepthReadbackOutcome::Failed(ArtDepthReadbackFailure {
            capture_id: pending.request.capture_id.clone(),
            reason: reason.to_owned(),
        })
    };

    if bytes.len() % 4 != 0 || pending.request.height == 0 {
        return fail("Depth32Float readback byte length is not f32-aligned");
    }
    let total_values = bytes.len() / 4;
    let height = pending.request.height as usize;
    if total_values % height != 0 {
        return fail("depth readback cannot derive a stable row stride");
    }
    let row_stride_values = total_values / height;
    if row_stride_values < pending.request.width as usize {
        return fail("depth readback row stride is smaller than the requested width");
    }

    let mut linear_depth_meters = Vec::with_capacity(total_values);
    for chunk in bytes.chunks_exact(4) {
        let raw = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        linear_depth_meters.push(pending.projection.linearize(raw).unwrap_or(f32::NAN));
    }

    let receipt = ArtCaptureReceipt {
        request: pending.request.clone(),
        observed_revision_id: pending.request.revision_id.clone(),
        observed_frame: pending.request.frame,
        observed_scene_hash: pending.request.scene_hash.clone(),
        artifact_locator: format!(
            "memory://bevy-depth32float-readback/{}",
            pending.request.capture_id
        ),
        artifact_digest: Some(fnv1a64(bytes)),
    };

    ArtDepthReadbackOutcome::Completed(ArtDepthGpuReadback {
        receipt,
        projection: pending.projection,
        linear_depth_meters,
        row_stride_values,
        render_epoch: pending.render_epoch,
    })
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtDepthReadbackError {
    Capture(ArtCaptureError),
    DepthChannelNotDeclared,
    UnsupportedProjection,
    InvalidProjection,
    ZeroCompletedCapacity,
}

impl std::fmt::Display for ArtDepthReadbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture error: {error}"),
            Self::DepthChannelNotDeclared => write!(f, "depth capture request must declare Depth"),
            Self::UnsupportedProjection => write!(f, "custom Bevy projection requires an explicit depth decoder"),
            Self::InvalidProjection => write!(f, "depth projection parameters are invalid"),
            Self::ZeroCompletedCapacity => write!(f, "depth completion queue capacity must be non-zero"),
        }
    }
}

impl std::error::Error for ArtDepthReadbackError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinite_reverse_z_matches_bevy_formula() {
        let projection = BevyDepthProjection::PerspectiveInfiniteReverseZ {
            near_meters: 0.1,
            culling_far_meters: 1000.0,
        };
        assert!((projection.linearize(1.0).unwrap() - 0.1).abs() < 1e-6);
        assert!((projection.linearize(0.1).unwrap() - 1.0).abs() < 1e-6);
        assert!((projection.linearize(0.01).unwrap() - 10.0).abs() < 1e-5);
        assert_eq!(projection.linearize(0.0), None);
    }

    #[test]
    fn orthographic_reverse_z_is_linear() {
        let projection = BevyDepthProjection::OrthographicReverseZ {
            near_meters: 2.0,
            far_meters: 12.0,
        };
        assert_eq!(projection.linearize(1.0), Some(2.0));
        assert_eq!(projection.linearize(0.5), Some(7.0));
        assert_eq!(projection.linearize(0.0), Some(12.0));
    }

    #[test]
    fn bounded_queue_rejects_instead_of_evicting() {
        let mut queue = ArtDepthGpuReadbackQueue::new(1).unwrap();
        let failure = || {
            ArtDepthReadbackOutcome::Failed(ArtDepthReadbackFailure {
                capture_id: "x".into(),
                reason: "test".into(),
            })
        };
        assert!(queue.push(failure()));
        assert!(!queue.push(failure()));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.dropped_total(), 1);
    }
}
