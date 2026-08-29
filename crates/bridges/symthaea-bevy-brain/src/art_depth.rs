// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic depth and occlusion evidence for ARTIST-EYE-v1B.
//!
//! This module is renderer-neutral. A host may obtain depth through a Bevy
//! render-graph adapter, a custom material pass, or another trusted path, but
//! the evidence layer accepts only an explicitly typed depth plane bound to an
//! existing [`ArtCaptureReceipt`].
//!
//! Depth evidence is descriptive. It does not define artistic value, utility,
//! beauty, reward, fitness, or mutation authority.

use crate::art_capture::{ArtCaptureError, ArtCaptureReceipt, ArtRenderChannel};
use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DepthPlaneEncoding {
    LinearMeters,
    Linear01 {
        near_meters: f32,
        far_meters: f32,
        reversed: bool,
    },
}

impl DepthPlaneEncoding {
    fn validate(self) -> Result<Self, ArtistDepthError> {
        match self {
            Self::LinearMeters => Ok(self),
            Self::Linear01 {
                near_meters,
                far_meters,
                ..
            } if near_meters.is_finite()
                && far_meters.is_finite()
                && near_meters >= 0.0
                && far_meters > near_meters => Ok(self),
            _ => Err(ArtistDepthError::InvalidEncoding),
        }
    }

    fn to_linear_meters(self, value: f32) -> Option<f32> {
        if !value.is_finite() {
            return None;
        }
        match self {
            Self::LinearMeters => (value >= 0.0).then_some(value),
            Self::Linear01 {
                near_meters,
                far_meters,
                reversed,
            } => {
                if !(0.0..=1.0).contains(&value) {
                    return None;
                }
                let t = if reversed { 1.0 - value } else { value };
                Some(near_meters + t * (far_meters - near_meters))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtistDepthConfig {
    pub far_clip_meters: f32,
    pub discontinuity_threshold_meters: f32,
    pub near_split_meters: f32,
    pub far_split_meters: f32,
}

impl Default for ArtistDepthConfig {
    fn default() -> Self {
        Self {
            far_clip_meters: 1000.0,
            discontinuity_threshold_meters: 0.25,
            near_split_meters: 5.0,
            far_split_meters: 25.0,
        }
    }
}

impl ArtistDepthConfig {
    pub fn validate(self) -> Result<Self, ArtistDepthError> {
        if !self.far_clip_meters.is_finite()
            || !self.discontinuity_threshold_meters.is_finite()
            || !self.near_split_meters.is_finite()
            || !self.far_split_meters.is_finite()
            || self.far_clip_meters <= 0.0
            || self.discontinuity_threshold_meters <= 0.0
            || self.near_split_meters < 0.0
            || self.far_split_meters <= self.near_split_meters
            || self.far_split_meters > self.far_clip_meters
        {
            return Err(ArtistDepthError::InvalidConfig);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DepthDistributionEvidence {
    pub valid_fraction: f64,
    pub clipped_far_fraction: f64,
    pub minimum_meters: Option<f64>,
    pub p10_meters: Option<f64>,
    pub median_meters: Option<f64>,
    pub p90_meters: Option<f64>,
    pub maximum_meters: Option<f64>,
    pub depth_span_meters: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DepthLayerEvidence {
    pub near_fraction: f64,
    pub middle_fraction: f64,
    pub far_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DepthCentroidEvidence {
    pub near_x: Option<f64>,
    pub near_y: Option<f64>,
    pub far_x: Option<f64>,
    pub far_y: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DepthDiscontinuityEvidence {
    pub horizontal_fraction: f64,
    pub vertical_fraction: f64,
    pub horizontal_mean_delta_meters: f64,
    pub vertical_mean_delta_meters: f64,
    pub boundary_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistDepthEvidence {
    pub width: u32,
    pub height: u32,
    pub distribution: DepthDistributionEvidence,
    pub layers: DepthLayerEvidence,
    pub centroids: DepthCentroidEvidence,
    pub discontinuities: DepthDiscontinuityEvidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistDepthObservation {
    pub capture_id: String,
    pub revision_id: String,
    pub frame: StudioFrame,
    pub rendered_scene_hash: String,
    pub evidence: ArtistDepthEvidence,
}

impl ArtistDepthObservation {
    pub fn from_capture_f32(
        receipt: &ArtCaptureReceipt,
        samples: &[f32],
        row_stride_values: usize,
        encoding: DepthPlaneEncoding,
        config: ArtistDepthConfig,
    ) -> Result<Self, ArtistDepthError> {
        receipt
            .validate_alignment()
            .map_err(ArtistDepthError::Capture)?;
        if !receipt.request.channels.contains(&ArtRenderChannel::Depth) {
            return Err(ArtistDepthError::DepthChannelNotDeclared);
        }
        let evidence = analyze_depth_plane(
            receipt.request.width,
            receipt.request.height,
            row_stride_values,
            samples,
            encoding,
            config,
        )?;
        Ok(Self {
            capture_id: receipt.request.capture_id.clone(),
            revision_id: receipt.observed_revision_id.clone(),
            frame: receipt.observed_frame,
            rendered_scene_hash: receipt.observed_scene_hash.clone(),
            evidence,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistDepthConsequenceEvidence {
    pub valid_fraction_delta: f64,
    pub clipped_far_fraction_delta: f64,
    pub median_depth_delta_meters: Option<f64>,
    pub depth_span_delta_meters: Option<f64>,
    pub near_fraction_delta: f64,
    pub middle_fraction_delta: f64,
    pub far_fraction_delta: f64,
    pub near_centroid_x_delta: Option<f64>,
    pub near_centroid_y_delta: Option<f64>,
    pub boundary_fraction_delta: f64,
    pub horizontal_boundary_delta: f64,
    pub vertical_boundary_delta: f64,
}

impl ArtistDepthConsequenceEvidence {
    pub fn between(baseline: &ArtistDepthObservation, candidate: &ArtistDepthObservation) -> Self {
        let b = &baseline.evidence;
        let c = &candidate.evidence;
        Self {
            valid_fraction_delta: c.distribution.valid_fraction - b.distribution.valid_fraction,
            clipped_far_fraction_delta: c.distribution.clipped_far_fraction
                - b.distribution.clipped_far_fraction,
            median_depth_delta_meters: option_delta(
                b.distribution.median_meters,
                c.distribution.median_meters,
            ),
            depth_span_delta_meters: option_delta(
                b.distribution.depth_span_meters,
                c.distribution.depth_span_meters,
            ),
            near_fraction_delta: c.layers.near_fraction - b.layers.near_fraction,
            middle_fraction_delta: c.layers.middle_fraction - b.layers.middle_fraction,
            far_fraction_delta: c.layers.far_fraction - b.layers.far_fraction,
            near_centroid_x_delta: option_delta(b.centroids.near_x, c.centroids.near_x),
            near_centroid_y_delta: option_delta(b.centroids.near_y, c.centroids.near_y),
            boundary_fraction_delta: c.discontinuities.boundary_fraction
                - b.discontinuities.boundary_fraction,
            horizontal_boundary_delta: c.discontinuities.horizontal_fraction
                - b.discontinuities.horizontal_fraction,
            vertical_boundary_delta: c.discontinuities.vertical_fraction
                - b.discontinuities.vertical_fraction,
        }
    }
}

pub fn analyze_depth_plane(
    width: u32,
    height: u32,
    row_stride_values: usize,
    samples: &[f32],
    encoding: DepthPlaneEncoding,
    config: ArtistDepthConfig,
) -> Result<ArtistDepthEvidence, ArtistDepthError> {
    let encoding = encoding.validate()?;
    let config = config.validate()?;
    if width == 0 || height == 0 {
        return Err(ArtistDepthError::InvalidDimensions);
    }
    let width_usize = width as usize;
    let height_usize = height as usize;
    if row_stride_values < width_usize {
        return Err(ArtistDepthError::RowStrideTooSmall);
    }
    let required = row_stride_values
        .checked_mul(height_usize)
        .ok_or(ArtistDepthError::DimensionOverflow)?;
    if samples.len() < required {
        return Err(ArtistDepthError::InsufficientSamples {
            required,
            actual: samples.len(),
        });
    }

    let mut plane = vec![None; width_usize * height_usize];
    let mut valid = Vec::with_capacity(width_usize * height_usize);
    let mut clipped_far = 0usize;
    let mut layer_values = Vec::new();

    for y in 0..height_usize {
        let row = &samples[y * row_stride_values..y * row_stride_values + width_usize];
        for (x, sample) in row.iter().copied().enumerate() {
            let Some(depth) = encoding.to_linear_meters(sample) else {
                continue;
            };
            plane[y * width_usize + x] = Some(depth);
            valid.push((depth, x, y));
            if depth >= config.far_clip_meters {
                clipped_far += 1;
            } else {
                layer_values.push((depth, x, y));
            }
        }
    }

    let total = width_usize * height_usize;
    let valid_fraction = fraction(valid.len(), total);
    let clipped_far_fraction = fraction(clipped_far, total);

    let mut sorted_depths: Vec<f32> = valid.iter().map(|entry| entry.0).collect();
    sorted_depths.sort_by(|a, b| a.total_cmp(b));

    let minimum_meters = sorted_depths.first().map(|v| f64::from(*v));
    let maximum_meters = sorted_depths.last().map(|v| f64::from(*v));
    let p10_meters = quantile(&sorted_depths, 0.10);
    let median_meters = quantile(&sorted_depths, 0.50);
    let p90_meters = quantile(&sorted_depths, 0.90);
    let depth_span_meters = match (minimum_meters, maximum_meters) {
        (Some(min), Some(max)) => Some(max - min),
        _ => None,
    };

    let layer_denominator = layer_values.len();
    let near = layer_values
        .iter()
        .filter(|entry| entry.0 < config.near_split_meters)
        .count();
    let middle = layer_values
        .iter()
        .filter(|entry| entry.0 >= config.near_split_meters && entry.0 < config.far_split_meters)
        .count();
    let far = layer_values
        .iter()
        .filter(|entry| entry.0 >= config.far_split_meters)
        .count();

    let layers = DepthLayerEvidence {
        near_fraction: fraction(near, layer_denominator),
        middle_fraction: fraction(middle, layer_denominator),
        far_fraction: fraction(far, layer_denominator),
    };

    let centroids = depth_centroids(&layer_values, width_usize, height_usize);
    let discontinuities = depth_discontinuities(
        &plane,
        width_usize,
        height_usize,
        config.discontinuity_threshold_meters,
    );

    Ok(ArtistDepthEvidence {
        width,
        height,
        distribution: DepthDistributionEvidence {
            valid_fraction,
            clipped_far_fraction,
            minimum_meters,
            p10_meters,
            median_meters,
            p90_meters,
            maximum_meters,
            depth_span_meters,
        },
        layers,
        centroids,
        discontinuities,
    })
}

fn depth_centroids(
    values: &[(f32, usize, usize)],
    width: usize,
    height: usize,
) -> DepthCentroidEvidence {
    if values.is_empty() {
        return DepthCentroidEvidence {
            near_x: None,
            near_y: None,
            far_x: None,
            far_y: None,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let quartile_len = ((sorted.len() + 3) / 4).max(1);
    let near = &sorted[..quartile_len.min(sorted.len())];
    let far_start = sorted.len().saturating_sub(quartile_len);
    let far = &sorted[far_start..];

    let centroid = |slice: &[(f32, usize, usize)]| -> (Option<f64>, Option<f64>) {
        if slice.is_empty() {
            return (None, None);
        }
        let x = slice.iter().map(|entry| entry.1 as f64).sum::<f64>() / slice.len() as f64;
        let y = slice.iter().map(|entry| entry.2 as f64).sum::<f64>() / slice.len() as f64;
        let nx = if width <= 1 { 0.5 } else { x / (width - 1) as f64 };
        let ny = if height <= 1 { 0.5 } else { y / (height - 1) as f64 };
        (Some(nx), Some(ny))
    };

    let (near_x, near_y) = centroid(near);
    let (far_x, far_y) = centroid(far);
    DepthCentroidEvidence {
        near_x,
        near_y,
        far_x,
        far_y,
    }
}

fn depth_discontinuities(
    plane: &[Option<f32>],
    width: usize,
    height: usize,
    threshold: f32,
) -> DepthDiscontinuityEvidence {
    let mut horizontal_pairs = 0usize;
    let mut horizontal_boundaries = 0usize;
    let mut horizontal_delta = 0.0f64;
    let mut vertical_pairs = 0usize;
    let mut vertical_boundaries = 0usize;
    let mut vertical_delta = 0.0f64;

    for y in 0..height {
        for x in 0..width {
            let Some(here) = plane[y * width + x] else {
                continue;
            };
            if x + 1 < width {
                if let Some(right) = plane[y * width + x + 1] {
                    let delta = (here - right).abs();
                    horizontal_pairs += 1;
                    horizontal_delta += f64::from(delta);
                    if delta >= threshold {
                        horizontal_boundaries += 1;
                    }
                }
            }
            if y + 1 < height {
                if let Some(down) = plane[(y + 1) * width + x] {
                    let delta = (here - down).abs();
                    vertical_pairs += 1;
                    vertical_delta += f64::from(delta);
                    if delta >= threshold {
                        vertical_boundaries += 1;
                    }
                }
            }
        }
    }

    let all_pairs = horizontal_pairs + vertical_pairs;
    let all_boundaries = horizontal_boundaries + vertical_boundaries;
    DepthDiscontinuityEvidence {
        horizontal_fraction: fraction(horizontal_boundaries, horizontal_pairs),
        vertical_fraction: fraction(vertical_boundaries, vertical_pairs),
        horizontal_mean_delta_meters: mean_or_zero(horizontal_delta, horizontal_pairs),
        vertical_mean_delta_meters: mean_or_zero(vertical_delta, vertical_pairs),
        boundary_fraction: fraction(all_boundaries, all_pairs),
    }
}

fn quantile(sorted: &[f32], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let index = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted.get(index).copied().map(f64::from)
}

fn fraction(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn mean_or_zero(sum: f64, count: usize) -> f64 {
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn option_delta(baseline: Option<f64>, candidate: Option<f64>) -> Option<f64> {
    match (baseline, candidate) {
        (Some(b), Some(c)) => Some(c - b),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtistDepthError {
    Capture(ArtCaptureError),
    DepthChannelNotDeclared,
    InvalidEncoding,
    InvalidConfig,
    InvalidDimensions,
    DimensionOverflow,
    RowStrideTooSmall,
    InsufficientSamples { required: usize, actual: usize },
}

impl std::fmt::Display for ArtistDepthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture error: {error}"),
            Self::DepthChannelNotDeclared => write!(f, "capture did not declare a depth channel"),
            Self::InvalidEncoding => write!(f, "invalid depth encoding"),
            Self::InvalidConfig => write!(f, "invalid ARTIST-EYE depth configuration"),
            Self::InvalidDimensions => write!(f, "depth plane dimensions must be non-zero"),
            Self::DimensionOverflow => write!(f, "depth plane dimensions overflowed address space"),
            Self::RowStrideTooSmall => write!(f, "depth row stride is smaller than width"),
            Self::InsufficientSamples { required, actual } => write!(
                f,
                "depth plane too short: required {required} samples, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ArtistDepthError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_capture::{ArtCapturePurpose, ArtCaptureRequest};

    fn receipt(width: u32, height: u32) -> ArtCaptureReceipt {
        ArtCaptureReceipt {
            request: ArtCaptureRequest {
                capture_id: "depth-1".into(),
                revision_id: "r1".into(),
                frame: StudioFrame(7),
                scene_hash: "scene".into(),
                camera_stable_id: Some("camera-a".into()),
                width,
                height,
                purpose: ArtCapturePurpose::CommittedObservation,
                channels: vec![ArtRenderChannel::Depth],
            },
            observed_revision_id: "r1".into(),
            observed_frame: StudioFrame(7),
            observed_scene_hash: "scene".into(),
            artifact_locator: "memory://depth".into(),
            artifact_digest: None,
        }
    }

    #[test]
    fn flat_plane_has_no_occlusion_boundaries() {
        let samples = vec![10.0f32; 16];
        let observation = ArtistDepthObservation::from_capture_f32(
            &receipt(4, 4),
            &samples,
            4,
            DepthPlaneEncoding::LinearMeters,
            ArtistDepthConfig::default(),
        )
        .unwrap();
        assert_eq!(observation.evidence.discontinuities.boundary_fraction, 0.0);
        assert_eq!(observation.evidence.layers.middle_fraction, 1.0);
    }

    #[test]
    fn near_far_split_creates_vertical_depth_boundary() {
        let mut samples = Vec::new();
        for _ in 0..4 {
            samples.extend_from_slice(&[2.0, 2.0, 30.0, 30.0]);
        }
        let evidence = analyze_depth_plane(
            4,
            4,
            4,
            &samples,
            DepthPlaneEncoding::LinearMeters,
            ArtistDepthConfig::default(),
        )
        .unwrap();
        assert!(evidence.discontinuities.horizontal_fraction > 0.0);
        assert_eq!(evidence.discontinuities.vertical_fraction, 0.0);
        assert_eq!(evidence.layers.near_fraction, 0.5);
        assert_eq!(evidence.layers.far_fraction, 0.5);
    }

    #[test]
    fn reversed_linear01_is_explicitly_supported() {
        let config = ArtistDepthConfig {
            far_clip_meters: 100.0,
            discontinuity_threshold_meters: 1.0,
            near_split_meters: 10.0,
            far_split_meters: 50.0,
        };
        let evidence = analyze_depth_plane(
            2,
            1,
            2,
            &[1.0, 0.0],
            DepthPlaneEncoding::Linear01 {
                near_meters: 1.0,
                far_meters: 100.0,
                reversed: true,
            },
            config,
        )
        .unwrap();
        assert_eq!(evidence.distribution.minimum_meters, Some(1.0));
        assert_eq!(evidence.distribution.maximum_meters, Some(100.0));
    }

    #[test]
    fn undeclared_depth_channel_is_rejected() {
        let mut receipt = receipt(1, 1);
        receipt.request.channels = vec![ArtRenderChannel::Color];
        let error = ArtistDepthObservation::from_capture_f32(
            &receipt,
            &[1.0],
            1,
            DepthPlaneEncoding::LinearMeters,
            ArtistDepthConfig::default(),
        )
        .unwrap_err();
        assert_eq!(error, ArtistDepthError::DepthChannelNotDeclared);
    }
}
