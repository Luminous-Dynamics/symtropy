// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pixel-aligned object/depth fusion for ARTIST-EYE-v1E.
//!
//! This layer fuses an object-ID raster with an already-linearized metric depth
//! raster only after both receipts prove the same revision/frame/scene/camera
//! and resolution. It reports physical distance per visible persistent object;
//! it does not assign aesthetic value or mutation authority.

use std::collections::BTreeMap;

use crate::{
    art_capture::{ArtCaptureError, ArtCaptureReceipt, ArtRenderChannel},
    art_object_id::ObjectIdRegistry,
    art_timeline::StudioFrame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectDepthFusionConfig {
    pub far_clip_meters: f32,
}

impl Default for ObjectDepthFusionConfig {
    fn default() -> Self {
        Self {
            far_clip_meters: 1000.0,
        }
    }
}

impl ObjectDepthFusionConfig {
    pub fn validate(self) -> Result<Self, ObjectDepthFusionError> {
        if !self.far_clip_meters.is_finite() || self.far_clip_meters <= 0.0 {
            return Err(ObjectDepthFusionError::InvalidConfig);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerObjectDepthEvidence {
    pub stable_id: String,
    pub raster_id: u32,
    pub visible_pixels: u64,
    pub valid_depth_pixels: u64,
    pub valid_depth_fraction: f64,
    pub minimum_meters: Option<f64>,
    pub p10_meters: Option<f64>,
    pub median_meters: Option<f64>,
    pub p90_meters: Option<f64>,
    pub maximum_meters: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectDepthPixel {
    pub raster_id: u32,
    pub depth_meters: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectDepthFusionFrame {
    pub object_capture_id: String,
    pub depth_capture_id: String,
    pub revision_id: String,
    pub frame: StudioFrame,
    pub scene_hash: String,
    pub camera_stable_id: String,
    pub width: u32,
    pub height: u32,
    pub registry_digest: String,
    pub objects: Vec<PerObjectDepthEvidence>,
    /// Compact cognitive-resolution aligned plane retained for causal
    /// occlusion/reveal analysis. Background has raster ID zero.
    pub pixels: Vec<ObjectDepthPixel>,
}

impl ObjectDepthFusionFrame {
    pub fn object(&self, stable_id: &str) -> Option<&PerObjectDepthEvidence> {
        self.objects.iter().find(|object| object.stable_id == stable_id)
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<ObjectDepthPixel> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels
            .get(y as usize * self.width as usize + x as usize)
            .copied()
    }
}

#[derive(Default)]
struct DepthAccumulator {
    visible_pixels: u64,
    depths: Vec<f32>,
}

#[allow(clippy::too_many_arguments)]
pub fn fuse_object_id_and_linear_depth(
    object_receipt: &ArtCaptureReceipt,
    object_ids: &[u32],
    object_row_stride_values: usize,
    depth_receipt: &ArtCaptureReceipt,
    linear_depth_meters: &[f32],
    depth_row_stride_values: usize,
    registry: &ObjectIdRegistry,
    config: ObjectDepthFusionConfig,
) -> Result<ObjectDepthFusionFrame, ObjectDepthFusionError> {
    let config = config.validate()?;
    object_receipt
        .validate_alignment()
        .map_err(ObjectDepthFusionError::Capture)?;
    depth_receipt
        .validate_alignment()
        .map_err(ObjectDepthFusionError::Capture)?;
    if !object_receipt
        .request
        .channels
        .contains(&ArtRenderChannel::ObjectId)
    {
        return Err(ObjectDepthFusionError::ObjectIdChannelNotDeclared);
    }
    if !depth_receipt.request.channels.contains(&ArtRenderChannel::Depth) {
        return Err(ObjectDepthFusionError::DepthChannelNotDeclared);
    }

    let a = &object_receipt.request;
    let b = &depth_receipt.request;
    if a.revision_id != b.revision_id
        || a.frame != b.frame
        || a.scene_hash != b.scene_hash
        || a.camera_stable_id != b.camera_stable_id
        || a.width != b.width
        || a.height != b.height
    {
        return Err(ObjectDepthFusionError::PlaneMisalignment);
    }
    let camera_stable_id = a
        .camera_stable_id
        .clone()
        .ok_or(ObjectDepthFusionError::MissingCameraIdentity)?;

    let width = a.width as usize;
    let height = a.height as usize;
    if object_row_stride_values < width || depth_row_stride_values < width {
        return Err(ObjectDepthFusionError::RowStrideTooSmall);
    }
    let required_objects = object_row_stride_values
        .checked_mul(height)
        .ok_or(ObjectDepthFusionError::DimensionOverflow)?;
    let required_depth = depth_row_stride_values
        .checked_mul(height)
        .ok_or(ObjectDepthFusionError::DimensionOverflow)?;
    if object_ids.len() < required_objects || linear_depth_meters.len() < required_depth {
        return Err(ObjectDepthFusionError::InsufficientSamples);
    }

    let mut accumulators: BTreeMap<u32, DepthAccumulator> = BTreeMap::new();
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        let object_row = &object_ids[y * object_row_stride_values..y * object_row_stride_values + width];
        let depth_row = &linear_depth_meters[y * depth_row_stride_values..y * depth_row_stride_values + width];
        for (&raster_id, &depth) in object_row.iter().zip(depth_row) {
            let depth = (depth.is_finite() && depth >= 0.0 && depth <= config.far_clip_meters)
                .then_some(depth);
            pixels.push(ObjectDepthPixel {
                raster_id,
                depth_meters: depth,
            });
            if raster_id == 0 {
                continue;
            }
            if registry.stable_id(raster_id).is_none() {
                return Err(ObjectDepthFusionError::UnknownRasterId(raster_id));
            }
            let accumulator = accumulators.entry(raster_id).or_default();
            accumulator.visible_pixels = accumulator.visible_pixels.saturating_add(1);
            if let Some(depth) = depth {
                accumulator.depths.push(depth);
            }
        }
    }

    let mut objects = Vec::with_capacity(accumulators.len());
    for (raster_id, mut accumulator) in accumulators {
        accumulator.depths.sort_by(f32::total_cmp);
        let stable_id = registry
            .stable_id(raster_id)
            .ok_or(ObjectDepthFusionError::UnknownRasterId(raster_id))?
            .to_owned();
        let valid_depth_pixels = accumulator.depths.len() as u64;
        let valid_depth_fraction = if accumulator.visible_pixels == 0 {
            0.0
        } else {
            valid_depth_pixels as f64 / accumulator.visible_pixels as f64
        };
        objects.push(PerObjectDepthEvidence {
            stable_id,
            raster_id,
            visible_pixels: accumulator.visible_pixels,
            valid_depth_pixels,
            valid_depth_fraction,
            minimum_meters: quantile(&accumulator.depths, 0.0),
            p10_meters: quantile(&accumulator.depths, 0.10),
            median_meters: quantile(&accumulator.depths, 0.50),
            p90_meters: quantile(&accumulator.depths, 0.90),
            maximum_meters: quantile(&accumulator.depths, 1.0),
        });
    }
    objects.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));

    Ok(ObjectDepthFusionFrame {
        object_capture_id: object_receipt.request.capture_id.clone(),
        depth_capture_id: depth_receipt.request.capture_id.clone(),
        revision_id: a.revision_id.clone(),
        frame: a.frame,
        scene_hash: a.scene_hash.clone(),
        camera_stable_id,
        width: a.width,
        height: a.height,
        registry_digest: registry.digest().to_owned(),
        objects,
        pixels,
    })
}

fn quantile(values: &[f32], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let index = ((values.len() - 1) as f64 * q).round() as usize;
    values.get(index).copied().map(f64::from)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectDepthFusionError {
    Capture(ArtCaptureError),
    InvalidConfig,
    ObjectIdChannelNotDeclared,
    DepthChannelNotDeclared,
    MissingCameraIdentity,
    PlaneMisalignment,
    RowStrideTooSmall,
    DimensionOverflow,
    InsufficientSamples,
    UnknownRasterId(u32),
}

impl std::fmt::Display for ObjectDepthFusionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture alignment error: {error}"),
            Self::InvalidConfig => write!(f, "object/depth fusion config is invalid"),
            Self::ObjectIdChannelNotDeclared => write!(f, "object receipt does not declare ObjectId"),
            Self::DepthChannelNotDeclared => write!(f, "depth receipt does not declare Depth"),
            Self::MissingCameraIdentity => write!(f, "object/depth fusion requires a stable camera identity"),
            Self::PlaneMisalignment => write!(f, "object-ID and depth planes are not the same causal render plane"),
            Self::RowStrideTooSmall => write!(f, "object/depth row stride is smaller than image width"),
            Self::DimensionOverflow => write!(f, "object/depth dimensions overflow usize"),
            Self::InsufficientSamples => write!(f, "object/depth planes do not contain all declared rows"),
            Self::UnknownRasterId(id) => write!(f, "object/depth plane contains unknown raster ID {id}"),
        }
    }
}

impl std::error::Error for ObjectDepthFusionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_capture::{ArtCapturePurpose, ArtCaptureRequest};

    fn receipt(id: &str, channel: ArtRenderChannel) -> ArtCaptureReceipt {
        let request = ArtCaptureRequest {
            capture_id: id.into(),
            revision_id: "r1".into(),
            frame: StudioFrame(7),
            scene_hash: "scene".into(),
            camera_stable_id: Some("camera".into()),
            width: 3,
            height: 1,
            purpose: ArtCapturePurpose::CommittedObservation,
            channels: vec![channel],
        };
        ArtCaptureReceipt {
            observed_revision_id: request.revision_id.clone(),
            observed_frame: request.frame,
            observed_scene_hash: request.scene_hash.clone(),
            artifact_locator: format!("memory://{id}"),
            artifact_digest: None,
            request,
        }
    }

    #[test]
    fn per_object_metric_depth_stays_separate() {
        let registry = ObjectIdRegistry::from_stable_ids(["near", "far"]).unwrap();
        let ids = [1, 1, 2];
        let depths = [2.0, 4.0, 20.0];
        let frame = fuse_object_id_and_linear_depth(
            &receipt("objects", ArtRenderChannel::ObjectId),
            &ids,
            3,
            &receipt("depth", ArtRenderChannel::Depth),
            &depths,
            3,
            &registry,
            ObjectDepthFusionConfig::default(),
        )
        .unwrap();
        assert_eq!(frame.object("near").unwrap().median_meters, Some(4.0));
        assert_eq!(frame.object("far").unwrap().median_meters, Some(20.0));
    }

    #[test]
    fn mismatched_scene_plane_is_rejected() {
        let registry = ObjectIdRegistry::from_stable_ids(["near"]).unwrap();
        let objects = receipt("objects", ArtRenderChannel::ObjectId);
        let mut depth = receipt("depth", ArtRenderChannel::Depth);
        depth.request.scene_hash = "other".into();
        depth.observed_scene_hash = "other".into();
        assert!(matches!(
            fuse_object_id_and_linear_depth(
                &objects,
                &[1, 0, 0],
                3,
                &depth,
                &[1.0, 1.0, 1.0],
                3,
                &registry,
                ObjectDepthFusionConfig::default(),
            ),
            Err(ObjectDepthFusionError::PlaneMisalignment)
        ));
    }
}
