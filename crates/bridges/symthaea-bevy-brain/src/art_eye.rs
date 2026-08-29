// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic multi-scale spatial evidence for artistic perception.
//!
//! ARTIST-EYE-v1A deliberately remains a measurement layer. It describes
//! value structure, silhouette/occupancy, negative space, edge orientation,
//! symmetry and focal-region evidence without defining beauty, utility, taste
//! or a preferred composition.

use std::collections::VecDeque;

use crate::art_capture::{ArtCaptureError, ArtCaptureReceipt};
use crate::art_timeline::StudioFrame;
use crate::art_visual::PixelLayout;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtistEyeConfig {
    /// Maximum number of 2x pyramid reductions, including the source level.
    pub pyramid_levels: u8,
    /// Absolute linear-luminance distance from estimated border background
    /// required for a pixel to count as occupied/silhouette evidence.
    pub silhouette_threshold: f64,
    /// Grid width used for coarse focal-region evidence.
    pub focal_grid_cols: u8,
    /// Grid height used for coarse focal-region evidence.
    pub focal_grid_rows: u8,
    /// Maximum focal regions retained per pyramid level.
    pub focal_regions_per_level: u8,
}

impl Default for ArtistEyeConfig {
    fn default() -> Self {
        Self {
            pyramid_levels: 4,
            silhouette_threshold: 0.08,
            focal_grid_cols: 4,
            focal_grid_rows: 4,
            focal_regions_per_level: 4,
        }
    }
}

impl ArtistEyeConfig {
    pub fn validate(&self) -> Result<(), ArtistEyeError> {
        if self.pyramid_levels == 0 || self.pyramid_levels > 8 {
            return Err(ArtistEyeError::InvalidPyramidLevels);
        }
        if !self.silhouette_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.silhouette_threshold)
        {
            return Err(ArtistEyeError::InvalidSilhouetteThreshold);
        }
        if self.focal_grid_cols == 0
            || self.focal_grid_rows == 0
            || self.focal_grid_cols > 32
            || self.focal_grid_rows > 32
        {
            return Err(ArtistEyeError::InvalidFocalGrid);
        }
        if self.focal_regions_per_level == 0 || self.focal_regions_per_level > 32 {
            return Err(ArtistEyeError::InvalidFocalRegionLimit);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueMassEvidence {
    pub dark_fraction: f64,
    pub mid_fraction: f64,
    pub light_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SilhouetteEvidence {
    /// Median linear luminance of the image border, used only as a local
    /// background estimate for this deterministic measurement.
    pub estimated_background_luminance: f64,
    pub occupied_fraction: f64,
    pub negative_space_fraction: f64,
    pub occupied_components: u32,
    pub negative_space_components: u32,
    pub largest_occupied_component_fraction: f64,
    pub largest_negative_component_fraction: f64,
    pub occupied_border_contact_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeOrientationEvidence {
    /// Gradient energy whose edge tangent is approximately horizontal.
    pub horizontal: f64,
    /// Gradient energy whose edge tangent is approximately vertical.
    pub vertical: f64,
    /// Gradient energy along one diagonal family.
    pub diagonal_down: f64,
    /// Gradient energy along the opposite diagonal family.
    pub diagonal_up: f64,
    /// Mean gradient magnitude before orientation normalization.
    pub mean_gradient_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymmetryEvidence {
    /// Mean absolute luminance mismatch under left-right reflection.
    pub left_right_mismatch: f64,
    /// Mean absolute luminance mismatch under top-bottom reflection.
    pub top_bottom_mismatch: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FocalRegionEvidence {
    pub grid_x: u8,
    pub grid_y: u8,
    pub center_x_normalized: f64,
    pub center_y_normalized: f64,
    /// Absolute tile/global luminance separation.
    pub value_separation: f64,
    /// Within-tile luminance standard deviation.
    pub local_contrast: f64,
    /// Mean local gradient magnitude.
    pub local_edge_energy: f64,
    /// Descriptive salience magnitude used only to rank regions for reporting.
    /// It is not an aesthetic score or policy signal.
    pub salience_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FocalHierarchyEvidence {
    pub regions: Vec<FocalRegionEvidence>,
    pub strongest_minus_second: f64,
    pub strongest_fraction_of_reported_salience: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistEyePyramidLevel {
    pub level: u8,
    pub width: u32,
    pub height: u32,
    pub mean_luminance: f64,
    pub rms_contrast: f64,
    pub value_mass: ValueMassEvidence,
    pub silhouette: SilhouetteEvidence,
    pub edges: EdgeOrientationEvidence,
    pub symmetry: SymmetryEvidence,
    pub focal_hierarchy: FocalHierarchyEvidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistEyeSpatialEvidence {
    pub levels: Vec<ArtistEyePyramidLevel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistEyeObservation {
    pub capture_id: String,
    pub revision_id: String,
    pub frame: StudioFrame,
    pub rendered_scene_hash: String,
    pub spatial: ArtistEyeSpatialEvidence,
}

impl ArtistEyeObservation {
    pub fn from_capture_bytes(
        receipt: &ArtCaptureReceipt,
        bytes: &[u8],
        row_stride_bytes: usize,
        layout: PixelLayout,
        config: ArtistEyeConfig,
    ) -> Result<Self, ArtistEyeError> {
        receipt
            .validate_alignment()
            .map_err(ArtistEyeError::Capture)?;
        let spatial = analyze_artist_eye_pixel_plane(
            receipt.request.width,
            receipt.request.height,
            row_stride_bytes,
            bytes,
            layout,
            config,
        )?;
        Ok(Self {
            capture_id: receipt.request.capture_id.clone(),
            revision_id: receipt.observed_revision_id.clone(),
            frame: receipt.observed_frame,
            rendered_scene_hash: receipt.observed_scene_hash.clone(),
            spatial,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistEyeLevelDelta {
    pub level: u8,
    pub mean_luminance_delta: f64,
    pub rms_contrast_delta: f64,
    pub dark_fraction_delta: f64,
    pub mid_fraction_delta: f64,
    pub light_fraction_delta: f64,
    pub occupied_fraction_delta: f64,
    pub negative_space_fraction_delta: f64,
    pub occupied_components_delta: i64,
    pub negative_space_components_delta: i64,
    pub horizontal_edge_delta: f64,
    pub vertical_edge_delta: f64,
    pub diagonal_down_edge_delta: f64,
    pub diagonal_up_edge_delta: f64,
    pub left_right_symmetry_mismatch_delta: f64,
    pub top_bottom_symmetry_mismatch_delta: f64,
    pub focal_separation_delta: f64,
    pub focal_concentration_delta: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtistEyeConsequenceEvidence {
    pub levels: Vec<ArtistEyeLevelDelta>,
}

impl ArtistEyeConsequenceEvidence {
    pub fn between(
        baseline: &ArtistEyeObservation,
        candidate: &ArtistEyeObservation,
    ) -> Result<Self, ArtistEyeError> {
        if baseline.spatial.levels.len() != candidate.spatial.levels.len() {
            return Err(ArtistEyeError::PyramidShapeMismatch);
        }
        let mut levels = Vec::with_capacity(baseline.spatial.levels.len());
        for (b, c) in baseline
            .spatial
            .levels
            .iter()
            .zip(candidate.spatial.levels.iter())
        {
            if b.level != c.level || b.width != c.width || b.height != c.height {
                return Err(ArtistEyeError::PyramidShapeMismatch);
            }
            levels.push(ArtistEyeLevelDelta {
                level: b.level,
                mean_luminance_delta: c.mean_luminance - b.mean_luminance,
                rms_contrast_delta: c.rms_contrast - b.rms_contrast,
                dark_fraction_delta: c.value_mass.dark_fraction - b.value_mass.dark_fraction,
                mid_fraction_delta: c.value_mass.mid_fraction - b.value_mass.mid_fraction,
                light_fraction_delta: c.value_mass.light_fraction - b.value_mass.light_fraction,
                occupied_fraction_delta: c.silhouette.occupied_fraction
                    - b.silhouette.occupied_fraction,
                negative_space_fraction_delta: c.silhouette.negative_space_fraction
                    - b.silhouette.negative_space_fraction,
                occupied_components_delta: i64::from(c.silhouette.occupied_components)
                    - i64::from(b.silhouette.occupied_components),
                negative_space_components_delta: i64::from(c.silhouette.negative_space_components)
                    - i64::from(b.silhouette.negative_space_components),
                horizontal_edge_delta: c.edges.horizontal - b.edges.horizontal,
                vertical_edge_delta: c.edges.vertical - b.edges.vertical,
                diagonal_down_edge_delta: c.edges.diagonal_down - b.edges.diagonal_down,
                diagonal_up_edge_delta: c.edges.diagonal_up - b.edges.diagonal_up,
                left_right_symmetry_mismatch_delta: c.symmetry.left_right_mismatch
                    - b.symmetry.left_right_mismatch,
                top_bottom_symmetry_mismatch_delta: c.symmetry.top_bottom_mismatch
                    - b.symmetry.top_bottom_mismatch,
                focal_separation_delta: c.focal_hierarchy.strongest_minus_second
                    - b.focal_hierarchy.strongest_minus_second,
                focal_concentration_delta: c
                    .focal_hierarchy
                    .strongest_fraction_of_reported_salience
                    - b.focal_hierarchy.strongest_fraction_of_reported_salience,
            });
        }
        Ok(Self { levels })
    }
}

pub fn analyze_artist_eye_pixel_plane(
    width: u32,
    height: u32,
    row_stride_bytes: usize,
    bytes: &[u8],
    layout: PixelLayout,
    config: ArtistEyeConfig,
) -> Result<ArtistEyeSpatialEvidence, ArtistEyeError> {
    config.validate()?;
    let mut plane = LuminancePlane::from_rgba_bytes(
        width,
        height,
        row_stride_bytes,
        bytes,
        layout,
    )?;
    let mut levels = Vec::new();

    for level in 0..config.pyramid_levels {
        levels.push(analyze_level(level, &plane, config)?);
        if plane.width <= 1 && plane.height <= 1 {
            break;
        }
        plane = plane.downsample_2x()?;
    }

    Ok(ArtistEyeSpatialEvidence { levels })
}

#[derive(Debug, Clone)]
struct LuminancePlane {
    width: usize,
    height: usize,
    values: Vec<f64>,
}

impl LuminancePlane {
    fn from_rgba_bytes(
        width: u32,
        height: u32,
        row_stride_bytes: usize,
        bytes: &[u8],
        layout: PixelLayout,
    ) -> Result<Self, ArtistEyeError> {
        if width == 0 || height == 0 {
            return Err(ArtistEyeError::InvalidDimensions);
        }
        let width = width as usize;
        let height = height as usize;
        let min_stride = width
            .checked_mul(4)
            .ok_or(ArtistEyeError::DimensionOverflow)?;
        if row_stride_bytes < min_stride {
            return Err(ArtistEyeError::RowStrideTooSmall);
        }
        let required = row_stride_bytes
            .checked_mul(height)
            .ok_or(ArtistEyeError::DimensionOverflow)?;
        if bytes.len() < required {
            return Err(ArtistEyeError::InsufficientBytes {
                required,
                actual: bytes.len(),
            });
        }

        let capacity = width
            .checked_mul(height)
            .ok_or(ArtistEyeError::DimensionOverflow)?;
        let mut values = Vec::with_capacity(capacity);
        for y in 0..height {
            let row_start = y
                .checked_mul(row_stride_bytes)
                .ok_or(ArtistEyeError::DimensionOverflow)?;
            let row = &bytes[row_start..row_start + min_stride];
            for x in 0..width {
                let p = &row[x * 4..x * 4 + 4];
                let (r, g, b) = match layout {
                    PixelLayout::Rgba8 => (p[0], p[1], p[2]),
                    PixelLayout::Bgra8 => (p[2], p[1], p[0]),
                };
                let r = srgb_u8_to_linear(r);
                let g = srgb_u8_to_linear(g);
                let b = srgb_u8_to_linear(b);
                values.push(0.2126 * r + 0.7152 * g + 0.0722 * b);
            }
        }
        Ok(Self {
            width,
            height,
            values,
        })
    }

    fn at(&self, x: usize, y: usize) -> f64 {
        self.values[y * self.width + x]
    }

    fn downsample_2x(&self) -> Result<Self, ArtistEyeError> {
        let new_width = self.width.div_ceil(2).max(1);
        let new_height = self.height.div_ceil(2).max(1);
        let mut values = Vec::with_capacity(
            new_width
                .checked_mul(new_height)
                .ok_or(ArtistEyeError::DimensionOverflow)?,
        );
        for y in 0..new_height {
            for x in 0..new_width {
                let mut sum = 0.0;
                let mut count = 0usize;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let sx = x * 2 + dx;
                        let sy = y * 2 + dy;
                        if sx < self.width && sy < self.height {
                            sum += self.at(sx, sy);
                            count += 1;
                        }
                    }
                }
                values.push(sum / count as f64);
            }
        }
        Ok(Self {
            width: new_width,
            height: new_height,
            values,
        })
    }
}

fn analyze_level(
    level: u8,
    plane: &LuminancePlane,
    config: ArtistEyeConfig,
) -> Result<ArtistEyePyramidLevel, ArtistEyeError> {
    let n = plane.values.len() as f64;
    let mean = plane.values.iter().sum::<f64>() / n;
    let variance = plane
        .values
        .iter()
        .map(|value| {
            let d = *value - mean;
            d * d
        })
        .sum::<f64>()
        / n;

    let value_mass = value_mass(&plane.values);
    let background = border_median(plane);
    let mask = occupancy_mask(plane, background, config.silhouette_threshold);
    let occupied_count = mask.iter().filter(|occupied| **occupied).count();
    let negative_count = mask.len() - occupied_count;
    let occupied_components = component_stats(&mask, plane.width, plane.height, true);
    let negative_components = component_stats(&mask, plane.width, plane.height, false);
    let border_contact = occupied_border_contact_fraction(&mask, plane.width, plane.height);

    let silhouette = SilhouetteEvidence {
        estimated_background_luminance: background,
        occupied_fraction: occupied_count as f64 / n,
        negative_space_fraction: negative_count as f64 / n,
        occupied_components: occupied_components.count,
        negative_space_components: negative_components.count,
        largest_occupied_component_fraction: occupied_components.largest as f64 / n,
        largest_negative_component_fraction: negative_components.largest as f64 / n,
        occupied_border_contact_fraction: border_contact,
    };

    let gradient = gradient_field(plane);
    let edges = orientation_evidence(&gradient);
    let symmetry = symmetry_evidence(plane);
    let focal_hierarchy = focal_hierarchy(plane, &gradient, config);

    Ok(ArtistEyePyramidLevel {
        level,
        width: plane.width as u32,
        height: plane.height as u32,
        mean_luminance: mean,
        rms_contrast: variance.sqrt(),
        value_mass,
        silhouette,
        edges,
        symmetry,
        focal_hierarchy,
    })
}

fn value_mass(values: &[f64]) -> ValueMassEvidence {
    let n = values.len() as f64;
    let mut dark = 0usize;
    let mut mid = 0usize;
    let mut light = 0usize;
    for value in values {
        if *value < 1.0 / 3.0 {
            dark += 1;
        } else if *value < 2.0 / 3.0 {
            mid += 1;
        } else {
            light += 1;
        }
    }
    ValueMassEvidence {
        dark_fraction: dark as f64 / n,
        mid_fraction: mid as f64 / n,
        light_fraction: light as f64 / n,
    }
}

fn border_median(plane: &LuminancePlane) -> f64 {
    let mut border = Vec::with_capacity((plane.width + plane.height) * 2);
    for x in 0..plane.width {
        border.push(plane.at(x, 0));
        if plane.height > 1 {
            border.push(plane.at(x, plane.height - 1));
        }
    }
    if plane.height > 2 {
        for y in 1..plane.height - 1 {
            border.push(plane.at(0, y));
            if plane.width > 1 {
                border.push(plane.at(plane.width - 1, y));
            }
        }
    }
    border.sort_by(f64::total_cmp);
    let middle = border.len() / 2;
    if border.len() % 2 == 0 && middle > 0 {
        (border[middle - 1] + border[middle]) * 0.5
    } else {
        border[middle]
    }
}

fn occupancy_mask(plane: &LuminancePlane, background: f64, threshold: f64) -> Vec<bool> {
    plane
        .values
        .iter()
        .map(|value| (*value - background).abs() >= threshold)
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct ComponentStats {
    count: u32,
    largest: usize,
}

fn component_stats(mask: &[bool], width: usize, height: usize, target: bool) -> ComponentStats {
    let mut visited = vec![false; mask.len()];
    let mut count = 0u32;
    let mut largest = 0usize;

    for start in 0..mask.len() {
        if visited[start] || mask[start] != target {
            continue;
        }
        count += 1;
        let mut size = 0usize;
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;

        while let Some(index) = queue.pop_front() {
            size += 1;
            let x = index % width;
            let y = index / width;
            let mut push = |nx: usize, ny: usize| {
                let next = ny * width + nx;
                if !visited[next] && mask[next] == target {
                    visited[next] = true;
                    queue.push_back(next);
                }
            };
            if x > 0 {
                push(x - 1, y);
            }
            if x + 1 < width {
                push(x + 1, y);
            }
            if y > 0 {
                push(x, y - 1);
            }
            if y + 1 < height {
                push(x, y + 1);
            }
        }
        largest = largest.max(size);
    }

    ComponentStats { count, largest }
}

fn occupied_border_contact_fraction(mask: &[bool], width: usize, height: usize) -> f64 {
    let mut border_total = 0usize;
    let mut occupied = 0usize;
    for y in 0..height {
        for x in 0..width {
            if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                border_total += 1;
                if mask[y * width + x] {
                    occupied += 1;
                }
            }
        }
    }
    if border_total == 0 {
        0.0
    } else {
        occupied as f64 / border_total as f64
    }
}

#[derive(Debug, Clone, Copy)]
struct GradientSample {
    gx: f64,
    gy: f64,
    magnitude: f64,
}

fn gradient_field(plane: &LuminancePlane) -> Vec<GradientSample> {
    let mut out = Vec::with_capacity(plane.values.len());
    for y in 0..plane.height {
        for x in 0..plane.width {
            let left = plane.at(x.saturating_sub(1), y);
            let right = plane.at((x + 1).min(plane.width - 1), y);
            let up = plane.at(x, y.saturating_sub(1));
            let down = plane.at(x, (y + 1).min(plane.height - 1));
            let gx = (right - left) * 0.5;
            let gy = (down - up) * 0.5;
            out.push(GradientSample {
                gx,
                gy,
                magnitude: gx.hypot(gy),
            });
        }
    }
    out
}

fn orientation_evidence(gradient: &[GradientSample]) -> EdgeOrientationEvidence {
    let mut horizontal = 0.0;
    let mut vertical = 0.0;
    let mut diagonal_down = 0.0;
    let mut diagonal_up = 0.0;
    let mut total = 0.0;

    for sample in gradient {
        let magnitude = sample.magnitude;
        if magnitude <= f64::EPSILON {
            continue;
        }
        // Gradient direction is perpendicular to the edge tangent. Bin the
        // tangent into four unsigned orientation families.
        let tangent = sample.gy.atan2(sample.gx) + std::f64::consts::FRAC_PI_2;
        let mut angle = tangent.rem_euclid(std::f64::consts::PI);
        if angle >= std::f64::consts::PI {
            angle -= std::f64::consts::PI;
        }
        let eighth = std::f64::consts::PI / 8.0;
        if angle < eighth || angle >= 7.0 * eighth {
            horizontal += magnitude;
        } else if angle < 3.0 * eighth {
            diagonal_down += magnitude;
        } else if angle < 5.0 * eighth {
            vertical += magnitude;
        } else {
            diagonal_up += magnitude;
        }
        total += magnitude;
    }

    let normalized = |value: f64| if total <= f64::EPSILON { 0.0 } else { value / total };
    EdgeOrientationEvidence {
        horizontal: normalized(horizontal),
        vertical: normalized(vertical),
        diagonal_down: normalized(diagonal_down),
        diagonal_up: normalized(diagonal_up),
        mean_gradient_magnitude: if gradient.is_empty() {
            0.0
        } else {
            total / gradient.len() as f64
        },
    }
}

fn symmetry_evidence(plane: &LuminancePlane) -> SymmetryEvidence {
    let mut lr_sum = 0.0;
    let mut lr_n = 0usize;
    for y in 0..plane.height {
        for x in 0..plane.width / 2 {
            lr_sum += (plane.at(x, y) - plane.at(plane.width - 1 - x, y)).abs();
            lr_n += 1;
        }
    }

    let mut tb_sum = 0.0;
    let mut tb_n = 0usize;
    for y in 0..plane.height / 2 {
        for x in 0..plane.width {
            tb_sum += (plane.at(x, y) - plane.at(x, plane.height - 1 - y)).abs();
            tb_n += 1;
        }
    }

    SymmetryEvidence {
        left_right_mismatch: mean_or_zero(lr_sum, lr_n),
        top_bottom_mismatch: mean_or_zero(tb_sum, tb_n),
    }
}

fn focal_hierarchy(
    plane: &LuminancePlane,
    gradient: &[GradientSample],
    config: ArtistEyeConfig,
) -> FocalHierarchyEvidence {
    let global_mean = plane.values.iter().sum::<f64>() / plane.values.len() as f64;
    let cols = usize::from(config.focal_grid_cols).min(plane.width).max(1);
    let rows = usize::from(config.focal_grid_rows).min(plane.height).max(1);
    let mut regions = Vec::with_capacity(cols * rows);

    for gy in 0..rows {
        let y0 = gy * plane.height / rows;
        let y1 = ((gy + 1) * plane.height / rows).max(y0 + 1);
        for gx in 0..cols {
            let x0 = gx * plane.width / cols;
            let x1 = ((gx + 1) * plane.width / cols).max(x0 + 1);
            let mut sum = 0.0;
            let mut sq_sum = 0.0;
            let mut edge_sum = 0.0;
            let mut n = 0usize;
            for y in y0..y1.min(plane.height) {
                for x in x0..x1.min(plane.width) {
                    let value = plane.at(x, y);
                    sum += value;
                    sq_sum += value * value;
                    edge_sum += gradient[y * plane.width + x].magnitude;
                    n += 1;
                }
            }
            let mean = mean_or_zero(sum, n);
            let variance = if n == 0 {
                0.0
            } else {
                (sq_sum / n as f64 - mean * mean).max(0.0)
            };
            let value_separation = (mean - global_mean).abs();
            let local_contrast = variance.sqrt();
            let local_edge_energy = mean_or_zero(edge_sum, n);
            // Ranking magnitude only; downstream artistic choice must retain the
            // three contributing dimensions independently.
            let salience_magnitude =
                value_separation + local_contrast + local_edge_energy;
            regions.push(FocalRegionEvidence {
                grid_x: gx as u8,
                grid_y: gy as u8,
                center_x_normalized: (x0 as f64 + x1 as f64) * 0.5 / plane.width as f64,
                center_y_normalized: (y0 as f64 + y1 as f64) * 0.5 / plane.height as f64,
                value_separation,
                local_contrast,
                local_edge_energy,
                salience_magnitude,
            });
        }
    }

    regions.sort_by(|a, b| {
        b.salience_magnitude
            .total_cmp(&a.salience_magnitude)
            .then_with(|| a.grid_y.cmp(&b.grid_y))
            .then_with(|| a.grid_x.cmp(&b.grid_x))
    });
    regions.truncate(usize::from(config.focal_regions_per_level).min(regions.len()));

    let first = regions.first().map_or(0.0, |r| r.salience_magnitude);
    let second = regions.get(1).map_or(0.0, |r| r.salience_magnitude);
    let total = regions.iter().map(|r| r.salience_magnitude).sum::<f64>();
    FocalHierarchyEvidence {
        regions,
        strongest_minus_second: first - second,
        strongest_fraction_of_reported_salience: if total <= f64::EPSILON {
            0.0
        } else {
            first / total
        },
    }
}

fn mean_or_zero(sum: f64, n: usize) -> f64 {
    if n == 0 { 0.0 } else { sum / n as f64 }
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
pub enum ArtistEyeError {
    Capture(ArtCaptureError),
    InvalidDimensions,
    DimensionOverflow,
    RowStrideTooSmall,
    InsufficientBytes { required: usize, actual: usize },
    InvalidPyramidLevels,
    InvalidSilhouetteThreshold,
    InvalidFocalGrid,
    InvalidFocalRegionLimit,
    PyramidShapeMismatch,
}

impl std::fmt::Display for ArtistEyeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "capture error: {error}"),
            Self::InvalidDimensions => write!(f, "artist-eye dimensions must be non-zero"),
            Self::DimensionOverflow => write!(f, "artist-eye dimensions overflowed address space"),
            Self::RowStrideTooSmall => write!(f, "artist-eye row stride is smaller than width * 4"),
            Self::InsufficientBytes { required, actual } => write!(
                f,
                "artist-eye buffer too short: required {required} bytes, got {actual}"
            ),
            Self::InvalidPyramidLevels => write!(f, "artist-eye pyramid levels must be in 1..=8"),
            Self::InvalidSilhouetteThreshold => write!(
                f,
                "artist-eye silhouette threshold must be finite and in [0,1]"
            ),
            Self::InvalidFocalGrid => write!(f, "artist-eye focal grid dimensions are invalid"),
            Self::InvalidFocalRegionLimit => {
                write!(f, "artist-eye focal region limit must be in 1..=32")
            }
            Self::PyramidShapeMismatch => write!(f, "artist-eye pyramid shapes do not match"),
        }
    }
}

impl std::error::Error for ArtistEyeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba_plane(width: usize, height: usize, pixel: [u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(width * height * 4);
        for _ in 0..width * height {
            bytes.extend_from_slice(&pixel);
        }
        bytes
    }

    fn centered_square(size: usize) -> Vec<u8> {
        let mut bytes = rgba_plane(size, size, [0, 0, 0, 255]);
        let q0 = size / 4;
        let q1 = size - q0;
        for y in q0..q1 {
            for x in q0..q1 {
                let i = (y * size + x) * 4;
                bytes[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        bytes
    }

    #[test]
    fn uniform_field_has_no_silhouette_or_focal_preference() {
        let bytes = rgba_plane(8, 8, [64, 64, 64, 255]);
        let eye = analyze_artist_eye_pixel_plane(
            8,
            8,
            32,
            &bytes,
            PixelLayout::Rgba8,
            ArtistEyeConfig::default(),
        )
        .unwrap();
        let l0 = &eye.levels[0];
        assert_eq!(l0.silhouette.occupied_components, 0);
        assert!(l0.silhouette.occupied_fraction.abs() < 1e-12);
        assert!(l0.edges.mean_gradient_magnitude.abs() < 1e-12);
        assert!(l0.focal_hierarchy.strongest_minus_second.abs() < 1e-12);
    }

    #[test]
    fn centered_form_is_separated_from_border_background() {
        let bytes = centered_square(16);
        let eye = analyze_artist_eye_pixel_plane(
            16,
            16,
            64,
            &bytes,
            PixelLayout::Rgba8,
            ArtistEyeConfig::default(),
        )
        .unwrap();
        let l0 = &eye.levels[0];
        assert_eq!(l0.silhouette.occupied_components, 1);
        assert!(l0.silhouette.occupied_fraction > 0.20);
        assert!(l0.silhouette.occupied_fraction < 0.30);
        assert!(l0.silhouette.negative_space_fraction > 0.70);
        assert!(l0.silhouette.occupied_border_contact_fraction.abs() < 1e-12);
    }

    #[test]
    fn vertical_boundary_produces_vertical_edge_evidence() {
        let mut bytes = Vec::new();
        for _y in 0..8 {
            for x in 0..8 {
                let v = if x < 4 { 0 } else { 255 };
                bytes.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let eye = analyze_artist_eye_pixel_plane(
            8,
            8,
            32,
            &bytes,
            PixelLayout::Rgba8,
            ArtistEyeConfig::default(),
        )
        .unwrap();
        let l0 = &eye.levels[0];
        assert!(l0.edges.vertical > 0.99);
        assert!(l0.edges.horizontal < 0.01);
    }

    #[test]
    fn asymmetric_field_is_detected_by_reflection_mismatch() {
        let mut bytes = Vec::new();
        for _y in 0..4 {
            for x in 0..4 {
                let v = if x == 0 { 255 } else { 0 };
                bytes.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let eye = analyze_artist_eye_pixel_plane(
            4,
            4,
            16,
            &bytes,
            PixelLayout::Rgba8,
            ArtistEyeConfig::default(),
        )
        .unwrap();
        assert!(eye.levels[0].symmetry.left_right_mismatch > 0.4);
    }

    #[test]
    fn analysis_is_exactly_deterministic_for_same_bytes() {
        let bytes = centered_square(9);
        let config = ArtistEyeConfig::default();
        let a = analyze_artist_eye_pixel_plane(
            9,
            9,
            36,
            &bytes,
            PixelLayout::Rgba8,
            config,
        )
        .unwrap();
        let b = analyze_artist_eye_pixel_plane(
            9,
            9,
            36,
            &bytes,
            PixelLayout::Rgba8,
            config,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn padded_rows_are_handled_without_reading_padding_as_pixels() {
        let row = [255, 255, 255, 255, 7, 8, 9, 10];
        let bytes = [row, row].concat();
        let eye = analyze_artist_eye_pixel_plane(
            1,
            2,
            8,
            &bytes,
            PixelLayout::Rgba8,
            ArtistEyeConfig::default(),
        )
        .unwrap();
        assert!(eye.levels[0].mean_luminance > 0.99);
    }
}
