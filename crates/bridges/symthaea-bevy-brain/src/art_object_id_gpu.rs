// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real Bevy 0.19 object-ID GPU acquisition for ARTIST-EYE-v1E.
//!
//! The committed scene is never recolored or material-swapped. Instead the
//! adapter creates a one-frame proxy scene on a dedicated render layer, using
//! the same mesh handles and world transforms but an exact ID material. A
//! dedicated camera renders those proxies into an `Rgba8Unorm` target with
//! MSAA and tonemapping disabled, then the target is detached and asynchronously
//! read back. The resulting bytes are decoded through the renderer-neutral
//! object-ID evidence layer.

#![cfg(feature = "realtime-art-object-id")]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::{
    asset::{load_internal_asset, uuid_handle},
    camera::{RenderTarget, visibility::RenderLayers},
    color::LinearRgba,
    core_pipeline::tonemapping::Tonemapping,
    pbr::{Material, MaterialPlugin, MeshMaterial3d},
    prelude::*,
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{AsBindGroup, TextureFormat, TextureUsages},
    },
    shader::{Shader, ShaderRef},
};

use crate::{
    art_capture::{ArtCaptureReceipt, ArtRenderChannel},
    art_object_id::{ObjectIdError, ObjectIdObservation, ObjectIdRegistry},
    art_object_id_codec::{ObjectIdCodecError, decode_rgba8_plane, raster_id_to_rgba8},
    art_object_render_plan::{ObjectIdRenderPlan, ObjectIdRenderPlanError},
};

pub const OBJECT_ID_RENDER_LAYER: usize = 31;
pub const OBJECT_ID_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("84ca02b7-0e12-4e90-a8e6-e50c95957d6b");

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ObjectIdMaterial {
    #[uniform(0)]
    encoded_id: LinearRgba,
}

impl ObjectIdMaterial {
    pub fn from_raster_id(raster_id: u32) -> Self {
        let [r, g, b, a] = raster_id_to_rgba8(raster_id);
        Self {
            encoded_id: LinearRgba::new(
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
                f32::from(a) / 255.0,
            ),
        }
    }
}

impl Material for ObjectIdMaterial {
    fn fragment_shader() -> ShaderRef {
        OBJECT_ID_SHADER_HANDLE.into()
    }
}

pub struct ObjectIdGpuPlugin;

impl Plugin for ObjectIdGpuPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            OBJECT_ID_SHADER_HANDLE,
            "object_id_material.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<ObjectIdMaterial>::default())
            .init_resource::<ObjectIdGpuReadbackQueue>();
    }
}

/// Snapshot of one source mesh used to build the isolated evidence proxy scene.
#[derive(Debug, Clone)]
pub struct ObjectIdGpuSource {
    pub stable_id: String,
    pub mesh: Handle<Mesh>,
    pub world_transform: Transform,
    pub visible: bool,
}

impl ObjectIdGpuSource {
    pub fn from_bevy(
        stable_id: impl Into<String>,
        mesh: &Mesh3d,
        global_transform: &GlobalTransform,
        visible: bool,
    ) -> Result<Self, ObjectIdGpuError> {
        let stable_id = stable_id.into();
        let world_transform = global_transform.compute_transform();
        if stable_id.trim().is_empty() || !world_transform.is_finite() {
            return Err(ObjectIdGpuError::InvalidSource);
        }
        Ok(Self {
            stable_id,
            mesh: mesh.0.clone(),
            world_transform,
            visible,
        })
    }
}

#[derive(Component, Debug, Clone)]
struct ObjectIdProxy {
    capture_id: String,
    stable_id: String,
}

#[derive(Component, Debug, Clone)]
struct ObjectIdEvidenceCamera {
    capture_id: String,
}

/// One armed render. The host must allow exactly one Bevy render frame before
/// calling `finish_render`.
#[derive(Debug)]
pub struct PreparedObjectIdGpuCapture {
    plan: ObjectIdRenderPlan,
    registry_digest: String,
    image: Handle<Image>,
    camera_entity: Entity,
    proxy_entities: Vec<Entity>,
    render_epoch: u64,
}

impl PreparedObjectIdGpuCapture {
    #[allow(clippy::too_many_arguments)]
    pub fn arm(
        commands: &mut Commands,
        images: &mut Assets<Image>,
        materials: &mut Assets<ObjectIdMaterial>,
        plan: ObjectIdRenderPlan,
        registry: &ObjectIdRegistry,
        source_camera_transform: &GlobalTransform,
        source_projection: &Projection,
        sources: &[ObjectIdGpuSource],
        render_epoch: u64,
    ) -> Result<Self, ObjectIdGpuError> {
        plan.validate(registry).map_err(ObjectIdGpuError::Plan)?;
        if plan.capture_request().channels != vec![ArtRenderChannel::ObjectId] {
            return Err(ObjectIdGpuError::InvalidPlanChannel);
        }

        let assignments: BTreeMap<&str, u32> = plan
            .assignments
            .iter()
            .map(|assignment| (assignment.stable_id.as_str(), assignment.raster_id))
            .collect();
        let mut seen = BTreeSet::new();
        for source in sources {
            if source.stable_id.trim().is_empty() || !source.world_transform.is_finite() {
                return Err(ObjectIdGpuError::InvalidSource);
            }
            if !seen.insert(source.stable_id.as_str()) {
                return Err(ObjectIdGpuError::DuplicateSource(source.stable_id.clone()));
            }
            if !assignments.contains_key(source.stable_id.as_str()) {
                return Err(ObjectIdGpuError::UnplannedSource(source.stable_id.clone()));
            }
        }
        for assignment in &plan.assignments {
            if !seen.contains(assignment.stable_id.as_str()) {
                return Err(ObjectIdGpuError::MissingPlannedSource(
                    assignment.stable_id.clone(),
                ));
            }
        }

        let mut image = Image::new_target_texture(
            plan.width,
            plan.height,
            TextureFormat::Rgba8Unorm,
            None,
        );
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let image = images.add(image);
        let render_layer = RenderLayers::layer(OBJECT_ID_RENDER_LAYER);

        let mut proxy_entities = Vec::new();
        for source in sources.iter().filter(|source| source.visible) {
            let raster_id = assignments
                .get(source.stable_id.as_str())
                .copied()
                .ok_or_else(|| ObjectIdGpuError::UnplannedSource(source.stable_id.clone()))?;
            let material = materials.add(ObjectIdMaterial::from_raster_id(raster_id));
            let entity = commands
                .spawn((
                    Mesh3d(source.mesh.clone()),
                    MeshMaterial3d(material),
                    source.world_transform,
                    render_layer.clone(),
                    ObjectIdProxy {
                        capture_id: plan.capture_id.clone(),
                        stable_id: source.stable_id.clone(),
                    },
                ))
                .id();
            proxy_entities.push(entity);
        }

        let camera_entity = commands
            .spawn((
                Camera3d::default(),
                Camera {
                    order: -100,
                    clear_color: Color::NONE.into(),
                    ..default()
                },
                RenderTarget::Image(image.clone().into()),
                source_camera_transform.compute_transform(),
                source_projection.clone(),
                Msaa::Off,
                Tonemapping::None,
                render_layer,
                ObjectIdEvidenceCamera {
                    capture_id: plan.capture_id.clone(),
                },
            ))
            .id();

        Ok(Self {
            registry_digest: registry.digest().to_owned(),
            plan,
            image,
            camera_entity,
            proxy_entities,
            render_epoch,
        })
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn finish_render(self, commands: &mut Commands) -> RenderedObjectIdGpuCapture {
        commands.entity(self.camera_entity).despawn();
        for entity in &self.proxy_entities {
            commands.entity(*entity).despawn();
        }
        RenderedObjectIdGpuCapture {
            plan: self.plan,
            registry_digest: self.registry_digest,
            image: self.image,
            render_epoch: self.render_epoch,
        }
    }
}

#[derive(Debug)]
pub struct RenderedObjectIdGpuCapture {
    plan: ObjectIdRenderPlan,
    registry_digest: String,
    image: Handle<Image>,
    render_epoch: u64,
}

impl RenderedObjectIdGpuCapture {
    pub fn queue_readback(self, commands: &mut Commands) -> Entity {
        let pending = PendingObjectIdReadback {
            plan: self.plan,
            registry_digest: self.registry_digest,
            render_epoch: self.render_epoch,
        };
        commands
            .spawn((Readback::texture(self.image), pending))
            .observe(complete_object_id_readback)
            .id()
    }
}

#[derive(Component, Debug)]
struct PendingObjectIdReadback {
    plan: ObjectIdRenderPlan,
    registry_digest: String,
    render_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct ObjectIdGpuReadback {
    pub receipt: ArtCaptureReceipt,
    pub registry_digest: String,
    pub raster_ids: Vec<u32>,
    pub row_stride_values: usize,
    pub render_epoch: u64,
}

impl ObjectIdGpuReadback {
    pub fn into_observation(
        self,
        registry: &ObjectIdRegistry,
    ) -> Result<ObjectIdObservation, ObjectIdGpuError> {
        if self.registry_digest != registry.digest() {
            return Err(ObjectIdGpuError::RegistryDigestMismatch);
        }
        ObjectIdObservation::from_capture_u32(
            &self.receipt,
            &self.raster_ids,
            self.row_stride_values,
            registry,
        )
        .map_err(ObjectIdGpuError::ObjectId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdGpuReadbackFailure {
    pub capture_id: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum ObjectIdGpuReadbackOutcome {
    Completed(ObjectIdGpuReadback),
    Failed(ObjectIdGpuReadbackFailure),
}

#[derive(Resource, Debug)]
pub struct ObjectIdGpuReadbackQueue {
    capacity: usize,
    completed: VecDeque<ObjectIdGpuReadbackOutcome>,
    dropped_total: u64,
}

impl ObjectIdGpuReadbackQueue {
    pub fn new(capacity: usize) -> Result<Self, ObjectIdGpuError> {
        if capacity == 0 {
            return Err(ObjectIdGpuError::ZeroCompletedCapacity);
        }
        Ok(Self {
            capacity,
            completed: VecDeque::with_capacity(capacity),
            dropped_total: 0,
        })
    }

    pub fn push(&mut self, outcome: ObjectIdGpuReadbackOutcome) -> bool {
        if self.completed.len() == self.capacity {
            self.dropped_total = self.dropped_total.saturating_add(1);
            return false;
        }
        self.completed.push_back(outcome);
        true
    }

    pub fn pop_next(&mut self) -> Option<ObjectIdGpuReadbackOutcome> {
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

impl Default for ObjectIdGpuReadbackQueue {
    fn default() -> Self {
        Self::new(8).expect("default object-id completion capacity is non-zero")
    }
}

fn complete_object_id_readback(
    event: On<ReadbackComplete>,
    pending: Query<&PendingObjectIdReadback>,
    mut completed: ResMut<ObjectIdGpuReadbackQueue>,
    mut commands: Commands,
) {
    let readback = event.event();
    let entity = readback.entity;
    let Ok(pending) = pending.get(entity) else {
        return;
    };

    let outcome = decode_readback(pending, &readback.data);
    let _ = completed.push(outcome);
    commands.entity(entity).despawn();
}

fn decode_readback(pending: &PendingObjectIdReadback, bytes: &[u8]) -> ObjectIdGpuReadbackOutcome {
    let fail = |reason: String| {
        ObjectIdGpuReadbackOutcome::Failed(ObjectIdGpuReadbackFailure {
            capture_id: pending.plan.capture_id.clone(),
            reason,
        })
    };
    let height = pending.plan.height as usize;
    if height == 0 || bytes.len() % height != 0 {
        return fail("object-id readback cannot derive a stable byte row stride".into());
    }
    let row_stride_bytes = bytes.len() / height;
    let raster_ids = match decode_rgba8_plane(
        pending.plan.width,
        pending.plan.height,
        row_stride_bytes,
        bytes,
    ) {
        Ok(values) => values,
        Err(error) => return fail(error.to_string()),
    };

    let request = pending.plan.capture_request();
    let receipt = ArtCaptureReceipt {
        observed_revision_id: request.revision_id.clone(),
        observed_frame: request.frame,
        observed_scene_hash: request.scene_hash.clone(),
        artifact_locator: format!(
            "memory://bevy-object-id-rgba8-readback/{}",
            request.capture_id
        ),
        artifact_digest: Some(raw_bytes_digest(bytes)),
        request,
    };

    ObjectIdGpuReadbackOutcome::Completed(ObjectIdGpuReadback {
        receipt,
        registry_digest: pending.registry_digest.clone(),
        raster_ids,
        row_stride_values: pending.plan.width as usize,
        render_epoch: pending.render_epoch,
    })
}

fn raw_bytes_digest(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[derive(Debug)]
pub enum ObjectIdGpuError {
    Plan(ObjectIdRenderPlanError),
    ObjectId(ObjectIdError),
    Codec(ObjectIdCodecError),
    InvalidPlanChannel,
    InvalidSource,
    DuplicateSource(String),
    UnplannedSource(String),
    MissingPlannedSource(String),
    RegistryDigestMismatch,
    ZeroCompletedCapacity,
}

impl std::fmt::Display for ObjectIdGpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan(error) => write!(f, "object-id render-plan error: {error}"),
            Self::ObjectId(error) => write!(f, "object-id evidence error: {error}"),
            Self::Codec(error) => write!(f, "object-id byte-codec error: {error}"),
            Self::InvalidPlanChannel => write!(f, "object-id GPU plan must request only ObjectId"),
            Self::InvalidSource => write!(f, "object-id proxy source is invalid"),
            Self::DuplicateSource(id) => write!(f, "duplicate object-id proxy source: {id}"),
            Self::UnplannedSource(id) => write!(f, "object-id source {id} is absent from prospective render plan"),
            Self::MissingPlannedSource(id) => write!(f, "prospectively planned object {id} has no proxy source"),
            Self::RegistryDigestMismatch => write!(f, "object-id GPU readback registry differs from frozen registry"),
            Self::ZeroCompletedCapacity => write!(f, "object-id GPU completion capacity must be non-zero"),
        }
    }
}

impl std::error::Error for ObjectIdGpuError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_encoding_preserves_all_four_id_bytes() {
        let id = 0xDEADBEEF;
        let material = ObjectIdMaterial::from_raster_id(id);
        let expected = raster_id_to_rgba8(id);
        let actual = [
            (material.encoded_id.red * 255.0).round() as u8,
            (material.encoded_id.green * 255.0).round() as u8,
            (material.encoded_id.blue * 255.0).round() as u8,
            (material.encoded_id.alpha * 255.0).round() as u8,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn completed_queue_rejects_without_eviction() {
        let mut queue = ObjectIdGpuReadbackQueue::new(1).unwrap();
        let failure = || {
            ObjectIdGpuReadbackOutcome::Failed(ObjectIdGpuReadbackFailure {
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
