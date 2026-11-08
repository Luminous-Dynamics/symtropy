// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic area-uniform sampling of triangulated surfaces.
//!
//! This Luminous-defined sampler is not claimed to reproduce the historical
//! ISPRS sample generator. The tessellation and visibility annotations are
//! caller-supplied, and must be digest-bound by the replay receipt.

use crate::metrics::Point3;
use std::collections::{BTreeMap, BTreeSet};

pub const SAMPLING_ALGORITHM_ID: &str = "uniform_area_halton_v1";
const DEGENERATE_TWICE_AREA_EPSILON: f64 = 1.0e-15;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceVisibility {
    Observable,
    SelfOccluded,
    ExternallyOccluded,
    OutsideSensorCoverage,
    Unknown,
    Contradictory,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceTriangleInput {
    pub surface_id: String,
    pub triangle_id: String,
    pub a: Point3,
    pub b: Point3,
    pub c: Point3,
    pub visibility: SurfaceVisibility,
}

impl SurfaceTriangleInput {
    pub fn new(
        surface_id: impl Into<String>,
        triangle_id: impl Into<String>,
        a: Point3,
        b: Point3,
        c: Point3,
        visibility: SurfaceVisibility,
    ) -> Self {
        Self {
            surface_id: surface_id.into(),
            triangle_id: triangle_id.into(),
            a,
            b,
            c,
            visibility,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaWeightedSamplingProfile {
    /// Density-equivalent spacing in metres. Per surface, sample count is
    /// ceil(observable_area / spacing²), with at least one sample for
    /// non-zero observable area. This is not a minimum-separation guarantee.
    pub target_spacing_m: f64,
    /// Deterministic sequence scramble; this is not a cryptographic seed.
    pub seed: u64,
    /// Hard total sample budget. Exceeding it is an error, never truncation.
    pub max_samples: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceSample {
    /// Stable identity is the tuple (surface_id, sample_ordinal).
    pub surface_id: String,
    pub sample_ordinal: u64,
    pub triangle_id: String,
    /// Barycentric weights in vertex order (a, b, c).
    pub barycentric: [f64; 3],
    pub point: Point3,
    /// Equal per-surface area represented by this sample.
    pub represented_area_m2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibilityArea {
    pub visibility: SurfaceVisibility,
    pub area_m2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceSamplingSummary {
    pub surface_id: String,
    pub observable_area_m2: f64,
    pub excluded_area_m2: f64,
    pub excluded_by_visibility: Vec<VisibilityArea>,
    pub sample_count: usize,
    pub represented_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceSamplingReceipt {
    pub algorithm_id: String,
    pub target_spacing_m: f64,
    pub seed: u64,
    pub max_samples: usize,
    pub observable_area_m2: f64,
    pub excluded_area_m2: f64,
    pub sample_count: usize,
    pub surfaces: Vec<SurfaceSamplingSummary>,
    pub samples: Vec<SurfaceSample>,
}

#[derive(Debug, Clone)]
struct PreparedTriangle {
    triangle_id: String,
    a: Point3,
    b: Point3,
    c: Point3,
    cumulative_area_m2: f64,
}

fn finite_point(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn triangle_area_m2(a: Point3, b: Point3, c: Point3) -> Result<f64, String> {
    if !finite_point(a) || !finite_point(b) || !finite_point(c) {
        return Err("surface triangle contains a non-finite coordinate".to_owned());
    }
    let ab = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let cross = Point3::new(
        ab.y * ac.z - ab.z * ac.y,
        ab.z * ac.x - ab.x * ac.z,
        ab.x * ac.y - ab.y * ac.x,
    );
    let twice_area = cross.x.hypot(cross.y).hypot(cross.z);
    if !twice_area.is_finite() {
        return Err("surface triangle area overflowed or became non-finite".to_owned());
    }
    if twice_area <= DEGENERATE_TWICE_AREA_EPSILON {
        return Err("surface triangle is degenerate".to_owned());
    }
    let area = twice_area * 0.5;
    if !area.is_finite() || area <= 0.0 {
        return Err("surface triangle area is not finite and positive".to_owned());
    }
    Ok(area)
}

fn validate_profile(profile: AreaWeightedSamplingProfile) -> Result<f64, String> {
    if !profile.target_spacing_m.is_finite() || profile.target_spacing_m <= 0.0 {
        return Err("target_spacing_m must be finite and positive metres".to_owned());
    }
    if profile.max_samples == 0 {
        return Err("max_samples must be positive".to_owned());
    }
    let spacing_squared = profile.target_spacing_m * profile.target_spacing_m;
    if !spacing_squared.is_finite() || spacing_squared <= 0.0 {
        return Err("target_spacing_m squared must be finite and positive".to_owned());
    }
    Ok(spacing_squared)
}

fn requested_count(area_m2: f64, spacing_squared: f64) -> Result<usize, String> {
    if area_m2 == 0.0 {
        return Ok(0);
    }
    let count = (area_m2 / spacing_squared).ceil().max(1.0);
    if !count.is_finite() || count > usize::MAX as f64 {
        return Err("surface sample count exceeds the platform usize range".to_owned());
    }
    Ok(count as usize)
}

fn surface_scramble(seed: u64, surface_id: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS ^ seed;
    for byte in surface_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

fn radical_inverse(mut index: u64, base: u64) -> f64 {
    let inverse_base = 1.0 / base as f64;
    let mut factor = inverse_base;
    let mut result = 0.0;
    while index > 0 {
        result += (index % base) as f64 * factor;
        index /= base;
        factor *= inverse_base;
    }
    result
}

fn halton_coordinate(index: u64, scramble: u64, base: u64) -> f64 {
    let value = radical_inverse(index, base) + radical_inverse(scramble, base);
    value - value.floor()
}

fn sample_triangle(
    triangles: &[PreparedTriangle],
    observable_area_m2: f64,
    selector: f64,
) -> &PreparedTriangle {
    let mut target = selector * observable_area_m2;
    if target >= observable_area_m2 {
        target = f64::from_bits(observable_area_m2.to_bits() - 1);
    }
    let index = triangles.partition_point(|triangle| triangle.cumulative_area_m2 <= target);
    &triangles[index.min(triangles.len() - 1)]
}

fn barycentric_sample(a: Point3, b: Point3, c: Point3, u: f64, v: f64) -> ([f64; 3], Point3) {
    let root = u.sqrt();
    let weights = [1.0 - root, root * (1.0 - v), root * v];
    let point = Point3::new(
        weights[0] * a.x + weights[1] * b.x + weights[2] * c.x,
        weights[0] * a.y + weights[1] * b.y + weights[2] * c.y,
        weights[0] * a.z + weights[1] * b.z + weights[2] * c.z,
    );
    (weights, point)
}

/// Generate an ordered, deterministic area-uniform sample receipt.
///
/// The sampler sorts by surface and triangle IDs, so reordering the input
/// array does not change the result. Changing the tessellation or IDs changes
/// the input identity and must be separately digest-bound. Every triangle is
/// validated even when its visibility class excludes it from sampling.
pub fn sample_observable_surfaces(
    input: &[SurfaceTriangleInput],
    profile: AreaWeightedSamplingProfile,
) -> Result<SurfaceSamplingReceipt, String> {
    let spacing_squared = validate_profile(profile)?;
    if input.is_empty() {
        return Err("surface sampling requires at least one triangle".to_owned());
    }

    let mut grouped: BTreeMap<String, Vec<&SurfaceTriangleInput>> = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for triangle in input {
        if triangle.surface_id.trim().is_empty() || triangle.triangle_id.trim().is_empty() {
            return Err("surface_id and triangle_id must be non-empty".to_owned());
        }
        if !identities.insert((triangle.surface_id.as_str(), triangle.triangle_id.as_str())) {
            return Err(format!(
                "duplicate triangle identity ({}, {})",
                triangle.surface_id, triangle.triangle_id
            ));
        }
        let _ = triangle_area_m2(triangle.a, triangle.b, triangle.c)?;
        grouped.entry(triangle.surface_id.clone()).or_default().push(triangle);
    }

    let mut all_samples = Vec::new();
    let mut surface_summaries = Vec::with_capacity(grouped.len());
    let mut total_observable_area_m2 = 0.0;
    let mut total_excluded_area_m2 = 0.0;

    for (surface_id, mut triangles) in grouped {
        triangles.sort_by(|left, right| left.triangle_id.cmp(&right.triangle_id));
        let mut excluded_areas: BTreeMap<SurfaceVisibility, f64> = BTreeMap::new();
        let mut observable = Vec::new();
        let mut observable_area_m2 = 0.0;

        for triangle in triangles {
            let area_m2 = triangle_area_m2(triangle.a, triangle.b, triangle.c)?;
            if triangle.visibility == SurfaceVisibility::Observable {
                observable_area_m2 += area_m2;
                if !observable_area_m2.is_finite() {
                    return Err(format!("observable area overflowed for surface {surface_id}"));
                }
                observable.push((
                    triangle.triangle_id.clone(),
                    triangle.a,
                    triangle.b,
                    triangle.c,
                    area_m2,
                ));
            } else {
                let total = excluded_areas.entry(triangle.visibility).or_insert(0.0);
                *total += area_m2;
                if !total.is_finite() {
                    return Err(format!("excluded area overflowed for surface {surface_id}"));
                }
            }
        }

        let excluded_by_visibility: Vec<VisibilityArea> = excluded_areas
            .iter()
            .map(|(visibility, area_m2)| VisibilityArea {
                visibility: *visibility,
                area_m2: *area_m2,
            })
            .collect();
        let excluded_area_m2: f64 = excluded_by_visibility.iter().map(|entry| entry.area_m2).sum();
        if !excluded_area_m2.is_finite() {
            return Err(format!("excluded area overflowed for surface {surface_id}"));
        }

        total_observable_area_m2 += observable_area_m2;
        total_excluded_area_m2 += excluded_area_m2;
        if !total_observable_area_m2.is_finite() || !total_excluded_area_m2.is_finite() {
            return Err("total surface area overflowed".to_owned());
        }

        let sample_count = requested_count(observable_area_m2, spacing_squared)?;
        let remaining_budget = profile.max_samples.saturating_sub(all_samples.len());
        if sample_count > remaining_budget {
            return Err(format!(
                "sample budget exceeded: requested {sample_count}, remaining {remaining_budget}"
            ));
        }
        let represented_area_m2 = if sample_count == 0 {
            0.0
        } else {
            observable_area_m2 / sample_count as f64
        };
        if sample_count > 0 && (!represented_area_m2.is_finite() || represented_area_m2 <= 0.0) {
            return Err(format!("invalid represented area weight for surface {surface_id}"));
        }

        let mut cumulative = 0.0;
        let mut prepared = Vec::with_capacity(observable.len());
        for (triangle_id, a, b, c, area_m2) in observable {
            cumulative += area_m2;
            if !cumulative.is_finite() {
                return Err(format!("triangle area accumulation overflowed for {surface_id}"));
            }
            prepared.push(PreparedTriangle {
                triangle_id,
                a,
                b,
                c,
                cumulative_area_m2: cumulative,
            });
        }
        if sample_count > 0 && prepared.is_empty() {
            return Err(format!("positive observable area has no triangles for {surface_id}"));
        }

        let scramble = surface_scramble(profile.seed, &surface_id);
        for ordinal in 0..sample_count {
            let index = (ordinal as u64) + 1;
            let u0 = halton_coordinate(index, scramble, 2);
            let u1 = halton_coordinate(index, scramble.rotate_left(17), 3);
            let u2 = halton_coordinate(index, scramble.rotate_left(31), 5);
            let target = sample_triangle(&prepared, observable_area_m2, u0);
            let (barycentric, point) = barycentric_sample(target.a, target.b, target.c, u1, u2);
            if !finite_point(point) {
                return Err(format!("sample point became non-finite for {surface_id}"));
            }
            all_samples.push(SurfaceSample {
                surface_id: surface_id.clone(),
                sample_ordinal: ordinal as u64,
                triangle_id: target.triangle_id.clone(),
                barycentric,
                point,
                represented_area_m2,
            });
        }

        let represented_total = represented_area_m2 * sample_count as f64;
        let tolerance = observable_area_m2.abs().max(1.0) * 1.0e-12;
        if (represented_total - observable_area_m2).abs() > tolerance {
            return Err(format!("sample weights do not conserve area for {surface_id}"));
        }

        surface_summaries.push(SurfaceSamplingSummary {
            surface_id,
            observable_area_m2,
            excluded_area_m2,
            excluded_by_visibility,
            sample_count,
            represented_area_m2: represented_total,
        });
    }

    Ok(SurfaceSamplingReceipt {
        algorithm_id: SAMPLING_ALGORITHM_ID.to_owned(),
        target_spacing_m: profile.target_spacing_m,
        seed: profile.seed,
        max_samples: profile.max_samples,
        observable_area_m2: total_observable_area_m2,
        excluded_area_m2: total_excluded_area_m2,
        sample_count: all_samples.len(),
        surfaces: surface_summaries,
        samples: all_samples,
    })
}
