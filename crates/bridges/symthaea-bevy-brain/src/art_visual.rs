// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Small deterministic whole-frame visual measurements for live art observations.
//!
//! These features are perception evidence, not aesthetic judgment. They are
//! intentionally cheap enough to run on bounded cognitive-resolution readbacks
//! and preserve separate dimensions rather than producing one beauty score.

use crate::art_capture::{ArtCaptureError, ArtCaptureReceipt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelLayout {
    Rgba8,
    Bgra8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePlaneFeatures {
    pub mean_luminance: f64,
    pub rms_contrast: f64,
    pub edge_energy: f64,
    pub mean_saturation: f64,
    /// Positive means the left half is brighter than the right half.
    pub horizontal_balance: f64,
    /// Positive means the upper half is brighter than the lower half.
    pub vertical_balance: f64,
    /// Center luminance minus the full-frame mean.
    pub center_emphasis: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualObservation {
    pub capture_id: String,
    pub revision_id: String,
    pub frame: crate::art_timeline::StudioFrame,
    pub rendered_scene_hash: String,
    pub features: ImagePlaneFeatures,
}

impl VisualObservation {
    pub fn from_capture_bytes(
        receipt: &ArtCaptureReceipt,
        bytes: &[u8],
        row_stride_bytes: usize,
        layout: PixelLayout,
    ) -> Result<Self, VisualPerceptionError> {
        receipt
            .validate_alignment()
            .map_err(VisualPerceptionError::Capture)?;
        let features = analyze_pixel_plane(
            receipt.request.width,
            receipt.request.height,
            row_stride_bytes,
            bytes,
            layout,
        )?;
        Ok(Self {
            capture_id: receipt.request.capture_id.clone(),
            revision_id: receipt.observed_revision_id.clone(),
            frame: receipt.observed_frame,
            rendered_scene_hash: receipt.observed_scene_hash.clone(),
            features,
        })
    }
}

/// Candidate minus baseline visual consequences. No aggregate preference is
/// defined; positive and negative values are descriptive changes only.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualConsequenceVector {
    pub mean_luminance_delta: f64,
    pub rms_contrast_delta: f64,
    pub edge_energy_delta: f64,
    pub mean_saturation_delta: f64,
    pub horizontal_balance_delta: f64,
    pub vertical_balance_delta: f64,
    pub center_emphasis_delta: f64,
}

impl VisualConsequenceVector {
    pub fn between(baseline: &VisualObservation, candidate: &VisualObservation) -> Self {
        let b = &baseline.features;
        let c = &candidate.features;
        Self {
            mean_luminance_delta: c.mean_luminance - b.mean_luminance,
            rms_contrast_delta: c.rms_contrast - b.rms_contrast,
            edge_energy_delta: c.edge_energy - b.edge_energy,
            mean_saturation_delta: c.mean_saturation - b.mean_saturation,
            horizontal_balance_delta: c.horizontal_balance - b.horizontal_balance,
            vertical_balance_delta: c.vertical_balance - b.vertical_balance,
            center_emphasis_delta: c.center_emphasis - b.center_emphasis,
        }
    }
}

pub fn analyze_pixel_plane(
    width: u32,
    height: u32,
    row_stride_bytes: usize,
    bytes: &[u8],
    layout: PixelLayout,
) -> Result<ImagePlaneFeatures, VisualPerceptionError> {
    if width == 0 || height == 0 {
        return Err(VisualPerceptionError::InvalidDimensions);
    }
    let width = width as usize;
    let height = height as usize;
    let min_stride = width
        .checked_mul(4)
        .ok_or(VisualPerceptionError::DimensionOverflow)?;
    if row_stride_bytes < min_stride {
        return Err(VisualPerceptionError::RowStrideTooSmall);
    }
    let required = row_stride_bytes
        .checked_mul(height)
        .ok_or(VisualPerceptionError::DimensionOverflow)?;
    if bytes.len() < required {
        return Err(VisualPerceptionError::InsufficientBytes {
            required,
            actual: bytes.len(),
        });
    }

    let mut luminances = Vec::with_capacity(width * height);
    let mut saturation_sum = 0.0;
    let mut left_sum = 0.0;
    let mut left_n = 0usize;
    let mut right_sum = 0.0;
    let mut right_n = 0usize;
    let mut upper_sum = 0.0;
    let mut upper_n = 0usize;
    let mut lower_sum = 0.0;
    let mut lower_n = 0usize;
    let mut center_sum = 0.0;
    let mut center_n = 0usize;

    let center_x0 = width / 4;
    let center_x1 = width.saturating_sub(width / 4);
    let center_y0 = height / 4;
    let center_y1 = height.saturating_sub(height / 4);

    for y in 0..height {
        let row = &bytes[y * row_stride_bytes..y * row_stride_bytes + min_stride];
        for x in 0..width {
            let pixel = &row[x * 4..x * 4 + 4];
            let (r, g, b) = match layout {
                PixelLayout::Rgba8 => (pixel[0], pixel[1], pixel[2]),
                PixelLayout::Bgra8 => (pixel[2], pixel[1], pixel[0]),
            };
            let r = srgb_u8_to_linear(r);
            let g = srgb_u8_to_linear(g);
            let b = srgb_u8_to_linear(b);
            let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            luminances.push(luminance);

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            saturation_sum += if max <= f64::EPSILON {
                0.0
            } else {
                (max - min) / max
            };

            if x < width / 2 {
                left_sum += luminance;
                left_n += 1;
            } else {
                right_sum += luminance;
                right_n += 1;
            }
            if y < height / 2 {
                upper_sum += luminance;
                upper_n += 1;
            } else {
                lower_sum += luminance;
                lower_n += 1;
            }
            if x >= center_x0 && x < center_x1 && y >= center_y0 && y < center_y1 {
                center_sum += luminance;
                center_n += 1;
            }
        }
    }

    let n = luminances.len() as f64;
    let mean = luminances.iter().sum::<f64>() / n;
    let variance = luminances
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / n;

    let mut edge_sum = 0.0;
    let mut edge_n = 0usize;
    for y in 0..height {
        for x in 0..width {
            let here = luminances[y * width + x];
            if x + 1 < width {
                edge_sum += (here - luminances[y * width + x + 1]).abs();
                edge_n += 1;
            }
            if y + 1 < height {
                edge_sum += (here - luminances[(y + 1) * width + x]).abs();
                edge_n += 1;
            }
        }
    }

    let mean_or_zero = |sum: f64, count: usize| {
        if count == 0 {
            0.0
        } else {
            sum / count as f64
        }
    };

    Ok(ImagePlaneFeatures {
        mean_luminance: mean,
        rms_contrast: variance.sqrt(),
        edge_energy: mean_or_zero(edge_sum, edge_n),
        mean_saturation: saturation_sum / n,
        horizontal_balance: mean_or_zero(left_sum, left_n) - mean_or_zero(right_sum, right_n),
        vertical_balance: mean_or_zero(upper_sum, upper_n) - mean_or_zero(lower_sum, lower_n),
        center_emphasis: mean_or_zero(center_sum, center_n) - mean,
    })
}

fn srgb_u8_to_linear(value: u8) -> f64 {
    let x = f64::from(value) / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualPerceptionError {
    Capture(ArtCaptureError),
    InvalidDimensions,
    DimensionOverflow,
    RowStrideTooSmall,
    InsufficientBytes { required: usize, actual: usize },
}

impl std::fmt::Display for VisualPerceptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture error: {error}"),
            Self::InvalidDimensions => write!(f, "pixel plane dimensions must be non-zero"),
            Self::DimensionOverflow => write!(f, "pixel plane dimensions overflowed address space"),
            Self::RowStrideTooSmall => write!(f, "pixel row stride is smaller than width * 4"),
            Self::InsufficientBytes { required, actual } => write!(
                f,
                "pixel buffer too short: required {required} bytes, got {actual}"
            ),
        }
    }
}

impl std::error::Error for VisualPerceptionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: usize, height: usize, rgba: [u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(width * height * 4);
        for _ in 0..width * height {
            bytes.extend_from_slice(&rgba);
        }
        bytes
    }

    #[test]
    fn solid_field_has_zero_contrast_and_edges() {
        let bytes = solid(4, 4, [128, 128, 128, 255]);
        let features = analyze_pixel_plane(4, 4, 16, &bytes, PixelLayout::Rgba8).unwrap();
        assert!(features.rms_contrast.abs() < 1e-12);
        assert!(features.edge_energy.abs() < 1e-12);
        assert!(features.horizontal_balance.abs() < 1e-12);
    }

    #[test]
    fn left_right_split_is_detected_without_scalar_judgment() {
        let mut bytes = Vec::new();
        for _y in 0..2 {
            bytes.extend_from_slice(&[255, 255, 255, 255]);
            bytes.extend_from_slice(&[0, 0, 0, 255]);
        }
        let features = analyze_pixel_plane(2, 2, 8, &bytes, PixelLayout::Rgba8).unwrap();
        assert!(features.horizontal_balance > 0.9);
        assert!(features.edge_energy > 0.0);
    }

    #[test]
    fn row_padding_is_ignored_explicitly() {
        let row = [255, 0, 0, 255, 0, 0, 0, 0];
        let bytes = [row, row].concat();
        let features = analyze_pixel_plane(1, 2, 8, &bytes, PixelLayout::Rgba8).unwrap();
        assert!(features.mean_luminance > 0.2);
    }
}
