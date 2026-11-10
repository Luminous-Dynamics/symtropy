// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic area-uniform sampling of triangulated surfaces.
//!
//! This Luminous-defined sampler is not claimed to reproduce the historical
//! ISPRS sample generator. The tessellation and visibility annotations are
//! caller-supplied, and must be digest-bound by the replay receipt.

use crate::metrics::Point3;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SAMPLING_ALGORITHM_ID: &str = "uniform_area_halton_v1";
const DEGENERATE_TWICE_AREA_EPSILON: f64 = 1.0e-15;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceVisibility {
    Observable,
    SelfOccluded,
    ExternallyOccluded,
    Grazing,
    MixedOcclusion,
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSample {
    /// Stable identity is the tuple (surface_id, sample_ordinal).
    pub surface_id: String,
    pub sample_ordinal: u64,
    pub triangle_id: String,
    /// Barycentric weights in vertex order (a, b, c).
    pub barycentric: [f64; 3],
    pub point: Point3,
    /// Unit normal following source triangle winding. This does not assert
    /// outward orientation; incidence consumers must be sign-invariant.
    pub normal: Point3,
    /// Equal per-surface area represented by this sample.
    pub represented_area_m2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibilityArea {
    pub visibility: SurfaceVisibility,
    pub area_m2: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSamplingSummary {
    pub surface_id: String,
    pub observable_area_m2: f64,
    pub excluded_area_m2: f64,
    pub excluded_by_visibility: Vec<VisibilityArea>,
    pub sample_count: usize,
    pub represented_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSamplingReceipt {
    pub algorithm_id: String,
    pub target_spacing_m: f64,
    pub seed: u64,
    pub max_samples: usize,
    pub observable_area_m2: f64,
    pub excluded_area_m2: f64,
    pub sample_count: usize,
    pub sample_set_sha256: String,
    pub surfaces: Vec<SurfaceSamplingSummary>,
    pub samples: Vec<SurfaceSample>,
}

#[derive(Debug, Clone)]
struct PreparedTriangle {
    triangle_id: String,
    a: Point3,
    b: Point3,
    c: Point3,
    normal: Point3,
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

fn triangle_unit_normal(a: Point3, b: Point3, c: Point3) -> Result<Point3, String> {
    let ab = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let cross = Point3::new(
        ab.y * ac.z - ab.z * ac.y,
        ab.z * ac.x - ab.x * ac.z,
        ab.x * ac.y - ab.y * ac.x,
    );
    let length = cross.x.hypot(cross.y).hypot(cross.z);
    if !length.is_finite() || length <= DEGENERATE_TWICE_AREA_EPSILON {
        return Err("surface triangle normal is degenerate".to_owned());
    }
    Ok(Point3::new(
        cross.x / length,
        cross.y / length,
        cross.z / length,
    ))
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
    if !count.is_finite() || count >= usize::MAX as f64 {
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

fn update_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn update_f64(hasher: &mut Sha256, value: f64) {
    hasher.update(value.to_bits().to_le_bytes());
}

fn update_string(hasher: &mut Sha256, value: &str) {
    update_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn visibility_label(value: SurfaceVisibility) -> &'static str {
    match value {
        SurfaceVisibility::Observable => "observable",
        SurfaceVisibility::SelfOccluded => "self_occluded",
        SurfaceVisibility::ExternallyOccluded => "externally_occluded",
        SurfaceVisibility::Grazing => "grazing",
        SurfaceVisibility::MixedOcclusion => "mixed_occlusion",
        SurfaceVisibility::OutsideSensorCoverage => "outside_sensor_coverage",
        SurfaceVisibility::Unknown => "unknown",
        SurfaceVisibility::Contradictory => "contradictory",
    }
}

fn sampling_receipt_digest(
    profile: AreaWeightedSamplingProfile,
    observable_area_m2: f64,
    excluded_area_m2: f64,
    surfaces: &[SurfaceSamplingSummary],
    samples: &[SurfaceSample],
) -> String {
    let mut hasher = Sha256::new();
    update_string(&mut hasher, SAMPLING_ALGORITHM_ID);
    update_f64(&mut hasher, profile.target_spacing_m);
    update_u64(&mut hasher, profile.seed);
    update_u64(&mut hasher, profile.max_samples as u64);
    update_f64(&mut hasher, observable_area_m2);
    update_f64(&mut hasher, excluded_area_m2);
    update_u64(&mut hasher, surfaces.len() as u64);

    for surface in surfaces {
        update_string(&mut hasher, &surface.surface_id);
        update_f64(&mut hasher, surface.observable_area_m2);
        update_f64(&mut hasher, surface.excluded_area_m2);
        update_u64(&mut hasher, surface.sample_count as u64);
        update_f64(&mut hasher, surface.represented_area_m2);
        update_u64(&mut hasher, surface.excluded_by_visibility.len() as u64);
        for excluded in &surface.excluded_by_visibility {
            update_string(&mut hasher, visibility_label(excluded.visibility));
            update_f64(&mut hasher, excluded.area_m2);
        }
    }

    update_u64(&mut hasher, samples.len() as u64);
    for sample in samples {
        update_string(&mut hasher, &sample.surface_id);
        update_u64(&mut hasher, sample.sample_ordinal);
        update_string(&mut hasher, &sample.triangle_id);
        for weight in sample.barycentric {
            update_f64(&mut hasher, weight);
        }
        update_f64(&mut hasher, sample.point.x);
        update_f64(&mut hasher, sample.point.y);
        update_f64(&mut hasher, sample.point.z);
        update_f64(&mut hasher, sample.normal.x);
        update_f64(&mut hasher, sample.normal.y);
        update_f64(&mut hasher, sample.normal.z);
        update_f64(&mut hasher, sample.represented_area_m2);
    }

    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Verify a sampling receipt's internal accounting and canonical digest.
///
/// This does not verify the source mesh file itself. The caller must separately
/// bind the receipt to the exact reference geometry artifact and evaluator
/// dataset manifest.
pub fn verify_surface_sampling_receipt(
    receipt: &SurfaceSamplingReceipt,
) -> Result<(), String> {
    if receipt.algorithm_id != SAMPLING_ALGORITHM_ID {
        return Err("unsupported surface sampling algorithm_id".to_owned());
    }
    let profile = AreaWeightedSamplingProfile {
        target_spacing_m: receipt.target_spacing_m,
        seed: receipt.seed,
        max_samples: receipt.max_samples,
    };
    let spacing_squared = validate_profile(profile)?;
    let _ = spacing_squared;
    if !receipt.observable_area_m2.is_finite()
        || receipt.observable_area_m2 < 0.0
        || !receipt.excluded_area_m2.is_finite()
        || receipt.excluded_area_m2 < 0.0
    {
        return Err("sampling receipt contains invalid top-level area totals".to_owned());
    }
    if receipt.sample_count != receipt.samples.len() {
        return Err("sampling receipt sample_count does not match samples length".to_owned());
    }
    if receipt.sample_count > receipt.max_samples {
        return Err("sampling receipt exceeds its declared max_samples".to_owned());
    }
    if receipt.surfaces.is_empty() {
        return Err("sampling receipt must contain at least one surface summary".to_owned());
    }
    if receipt.sample_set_sha256.len() != 64
        || !receipt
            .sample_set_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("sample_set_sha256 must be 64 lowercase hexadecimal characters".to_owned());
    }

    let mut summaries = BTreeMap::new();
    let mut observable_area_m2 = 0.0;
    let mut excluded_area_m2 = 0.0;
    let mut previous_surface: Option<&str> = None;
    for summary in &receipt.surfaces {
        if summary.surface_id.trim().is_empty() {
            return Err("sampling receipt contains an empty surface_id".to_owned());
        }
        if previous_surface.is_some_and(|previous| previous >= summary.surface_id.as_str()) {
            return Err("sampling receipt surface summaries are not uniquely sorted".to_owned());
        }
        previous_surface = Some(&summary.surface_id);
        if [
            summary.observable_area_m2,
            summary.excluded_area_m2,
            summary.represented_area_m2,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("sampling surface summary contains invalid area".to_owned());
        }
        if summary.sample_count > receipt.max_samples {
            return Err("surface summary exceeds the declared sample budget".to_owned());
        }
        if summary.excluded_area_m2 != 0.0 || !summary.excluded_by_visibility.is_empty() {
            return Err(
                "reference sampling receipt must include all source area before visibility"
                    .to_owned(),
            );
        }
        let excluded_sum: f64 = summary
            .excluded_by_visibility
            .iter()
            .map(|entry| entry.area_m2)
            .sum();
        if !excluded_sum.is_finite()
            || (excluded_sum - summary.excluded_area_m2).abs()
                > summary.excluded_area_m2.abs().max(1.0) * 1.0e-10
        {
            return Err("excluded_by_visibility does not conserve surface area".to_owned());
        }
        for excluded in &summary.excluded_by_visibility {
            if excluded.visibility == SurfaceVisibility::Observable
                || !excluded.area_m2.is_finite()
                || excluded.area_m2 <= 0.0
            {
                return Err("excluded visibility accounting contains an invalid class or area"
                    .to_owned());
            }
        }
        if (summary.represented_area_m2 - summary.observable_area_m2).abs()
            > summary.observable_area_m2.abs().max(1.0) * 1.0e-10
        {
            return Err("represented area does not equal observable area".to_owned());
        }
        if (summary.sample_count == 0 && summary.observable_area_m2 != 0.0)
            || (summary.sample_count > 0 && summary.observable_area_m2 <= 0.0)
        {
            return Err("surface sample count is inconsistent with observable area".to_owned());
        }
        observable_area_m2 += summary.observable_area_m2;
        excluded_area_m2 += summary.excluded_area_m2;
        summaries.insert(summary.surface_id.as_str(), summary);
    }

    if !observable_area_m2.is_finite() || !excluded_area_m2.is_finite() {
        return Err("sampling receipt total area overflowed".to_owned());
    }
    if (observable_area_m2 - receipt.observable_area_m2).abs()
        > receipt.observable_area_m2.abs().max(1.0) * 1.0e-10
        || (excluded_area_m2 - receipt.excluded_area_m2).abs()
            > receipt.excluded_area_m2.abs().max(1.0) * 1.0e-10
    {
        return Err("sampling receipt top-level areas do not match surface summaries".to_owned());
    }

    let mut per_surface_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_surface_weights: BTreeMap<&str, f64> = BTreeMap::new();
    let mut per_surface_ordinals: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    let mut previous_sample: Option<(&str, u64)> = None;
    for sample in &receipt.samples {
        if sample.surface_id.trim().is_empty() || sample.triangle_id.trim().is_empty() {
            return Err("sampling receipt contains an empty sample identity".to_owned());
        }
        if let Some((surface_id, ordinal)) = previous_sample {
            if (surface_id, ordinal) >= (sample.surface_id.as_str(), sample.sample_ordinal) {
                return Err("sampling receipt samples are not uniquely sorted".to_owned());
            }
        }
        previous_sample = Some((&sample.surface_id, sample.sample_ordinal));
        let summary = summaries
            .get(sample.surface_id.as_str())
            .ok_or("sample references a surface without a summary")?;
        if !sample.point.x.is_finite()
            || !sample.point.y.is_finite()
            || !sample.point.z.is_finite()
            || !sample.normal.x.is_finite()
            || !sample.normal.y.is_finite()
            || !sample.normal.z.is_finite()
            || !sample.represented_area_m2.is_finite()
            || sample.represented_area_m2 <= 0.0
        {
            return Err("sampling receipt contains a non-finite point/normal or invalid weight"
                .to_owned());
        }
        let normal_length = sample
            .normal
            .x
            .hypot(sample.normal.y)
            .hypot(sample.normal.z);
        if (normal_length - 1.0).abs() > 1.0e-8 {
            return Err("sample normal is not unit length".to_owned());
        }
        if sample
            .barycentric
            .iter()
            .any(|weight| !weight.is_finite() || *weight < -1.0e-12 || *weight > 1.0 + 1.0e-12)
            || (sample.barycentric.iter().sum::<f64>() - 1.0).abs() > 1.0e-10
        {
            return Err("sample barycentric coordinates are invalid".to_owned());
        }
        if sample.sample_ordinal >= summary.sample_count as u64 {
            return Err("sample ordinal exceeds its surface sample count".to_owned());
        }
        let expected_weight = summary.represented_area_m2 / summary.sample_count as f64;
        if (sample.represented_area_m2 - expected_weight).abs()
            > expected_weight.abs().max(1.0) * 1.0e-10
        {
            return Err("sample weights are not equal within the surface".to_owned());
        }
        *per_surface_counts.entry(sample.surface_id.as_str()).or_insert(0) += 1;
        *per_surface_weights.entry(sample.surface_id.as_str()).or_insert(0.0) +=
            sample.represented_area_m2;
        if !per_surface_weights[sample.surface_id.as_str()].is_finite() {
            return Err("sample represented area overflowed".to_owned());
        }
        if !per_surface_ordinals
            .entry(sample.surface_id.as_str())
            .or_default()
            .insert(sample.sample_ordinal)
        {
            return Err("duplicate sample ordinal within a surface".to_owned());
        }
    }

    for summary in &receipt.surfaces {
        let count = per_surface_counts.get(summary.surface_id.as_str()).copied().unwrap_or(0);
        if count != summary.sample_count {
            return Err("surface summary sample_count does not match sample rows".to_owned());
        }
        if count > 0 {
            let weight_sum = per_surface_weights
                .get(summary.surface_id.as_str())
                .copied()
                .unwrap_or(0.0);
            if (weight_sum - summary.represented_area_m2).abs()
                > summary.represented_area_m2.abs().max(1.0) * 1.0e-10
            {
                return Err("surface sample weights do not conserve represented area".to_owned());
            }
            let ordinals = per_surface_ordinals
                .get(summary.surface_id.as_str())
                .ok_or("surface sample ordinals are missing")?;
            if ordinals.len() != count
                || (0..count as u64).any(|ordinal| !ordinals.contains(&ordinal))
            {
                return Err("surface sample ordinals are not contiguous from zero".to_owned());
            }
        }
    }

    let digest = sampling_receipt_digest(
        profile,
        receipt.observable_area_m2,
        receipt.excluded_area_m2,
        &receipt.surfaces,
        &receipt.samples,
    );
    if digest != receipt.sample_set_sha256 {
        return Err("sampling receipt canonical SHA-256 does not match".to_owned());
    }
    Ok(())
}

/// Sample the entire evaluator reference surface before visibility is known.
///
/// This function intentionally ignores any pre-existing visibility annotations
/// and treats every reference triangle as a sampling target. Its output and
/// source geometry must remain evaluator-only; it must never be run over or
/// shared with candidate inputs.
pub fn sample_reference_surfaces_for_visibility(
    input: &[SurfaceTriangleInput],
    profile: AreaWeightedSamplingProfile,
) -> Result<SurfaceSamplingReceipt, String> {
    let reference_triangles: Vec<SurfaceTriangleInput> = input
        .iter()
        .cloned()
        .map(|mut triangle| {
            triangle.visibility = SurfaceVisibility::Observable;
            triangle
        })
        .collect();
    sample_observable_surfaces(&reference_triangles, profile)
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
        grouped
            .entry(triangle.surface_id.clone())
            .or_default()
            .push(triangle);
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
                    return Err(format!(
                        "observable area overflowed for surface {surface_id}"
                    ));
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
        let excluded_area_m2: f64 = excluded_by_visibility
            .iter()
            .map(|entry| entry.area_m2)
            .sum();
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
            return Err(format!(
                "invalid represented area weight for surface {surface_id}"
            ));
        }

        let mut cumulative = 0.0;
        let mut prepared = Vec::with_capacity(observable.len());
        for (triangle_id, a, b, c, area_m2) in observable {
            cumulative += area_m2;
            if !cumulative.is_finite() {
                return Err(format!(
                    "triangle area accumulation overflowed for {surface_id}"
                ));
            }
            prepared.push(PreparedTriangle {
                triangle_id,
                a,
                b,
                c,
                normal: triangle_unit_normal(a, b, c)?,
                cumulative_area_m2: cumulative,
            });
        }
        if sample_count > 0 && prepared.is_empty() {
            return Err(format!(
                "positive observable area has no triangles for {surface_id}"
            ));
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
                normal: target.normal,
                represented_area_m2,
            });
        }

        let represented_total = represented_area_m2 * sample_count as f64;
        let tolerance = observable_area_m2.abs().max(1.0) * 1.0e-12;
        if (represented_total - observable_area_m2).abs() > tolerance {
            return Err(format!(
                "sample weights do not conserve area for {surface_id}"
            ));
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

    let sample_set_sha256 = sampling_receipt_digest(
        profile,
        total_observable_area_m2,
        total_excluded_area_m2,
        &surface_summaries,
        &all_samples,
    );

    Ok(SurfaceSamplingReceipt {
        algorithm_id: SAMPLING_ALGORITHM_ID.to_owned(),
        target_spacing_m: profile.target_spacing_m,
        seed: profile.seed,
        max_samples: profile.max_samples,
        observable_area_m2: total_observable_area_m2,
        excluded_area_m2: total_excluded_area_m2,
        sample_count: all_samples.len(),
        sample_set_sha256,
        surfaces: surface_summaries,
        samples: all_samples,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AreaWeightedSamplingProfile, SurfaceTriangleInput, SurfaceVisibility,
        sample_observable_surfaces, sample_reference_surfaces_for_visibility,
        verify_surface_sampling_receipt,
    };
    use crate::metrics::Point3;

    fn profile(spacing: f64, seed: u64) -> AreaWeightedSamplingProfile {
        AreaWeightedSamplingProfile {
            target_spacing_m: spacing,
            seed,
            max_samples: 100_000,
        }
    }

    fn triangle(
        surface: &str,
        id: &str,
        a: [f64; 3],
        b: [f64; 3],
        c: [f64; 3],
        visibility: SurfaceVisibility,
    ) -> SurfaceTriangleInput {
        SurfaceTriangleInput::new(
            surface,
            id,
            Point3::new(a[0], a[1], a[2]),
            Point3::new(b[0], b[1], b[2]),
            Point3::new(c[0], c[1], c[2]),
            visibility,
        )
    }

    fn square(
        surface: &str,
        prefix: &str,
        area: f64,
        visibility: SurfaceVisibility,
    ) -> Vec<SurfaceTriangleInput> {
        let width = area.sqrt();
        vec![
            triangle(
                surface,
                &format!("{prefix}-a"),
                [0.0, 0.0, 0.0],
                [width, 0.0, 0.0],
                [width, width, 0.0],
                visibility,
            ),
            triangle(
                surface,
                &format!("{prefix}-b"),
                [0.0, 0.0, 0.0],
                [width, width, 0.0],
                [0.0, width, 0.0],
                visibility,
            ),
        ]
    }

    #[test]
    fn sample_count_is_area_proportional_and_weights_conserve_area() {
        let mut input = square("SURFACE-A", "a", 1.0, SurfaceVisibility::Observable);
        input.extend(square("SURFACE-B", "b", 3.0, SurfaceVisibility::Observable));
        let receipt =
            sample_observable_surfaces(&input, profile(0.2, 7)).expect("analytic mesh samples");
        assert_eq!(receipt.surfaces.len(), 2);
        assert_eq!(receipt.surfaces[0].surface_id, "SURFACE-A");
        assert_eq!(receipt.surfaces[1].surface_id, "SURFACE-B");
        assert!((receipt.observable_area_m2 - 4.0).abs() < 1e-12);
        assert_eq!(receipt.surfaces[0].sample_count, 25);
        assert_eq!(receipt.surfaces[1].sample_count, 75);
        assert_eq!(receipt.sample_count, 100);
        assert!((receipt.surfaces[0].represented_area_m2 - 1.0).abs() < 1e-12);
        assert!((receipt.surfaces[1].represented_area_m2 - 3.0).abs() < 1e-12);
    }

    #[test]
    fn generated_barycentric_points_lie_inside_their_source_square() {
        let input = square("WALL", "wall", 1.0, SurfaceVisibility::Observable);
        let receipt = sample_observable_surfaces(&input, profile(0.1, 21))
            .expect("valid triangulated wall should be sampled");
        assert_eq!(receipt.sample_count, 100);
        for sample in &receipt.samples {
            assert!(sample.barycentric.iter().all(|weight| *weight >= 0.0));
            assert!((sample.barycentric.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            assert!(sample.point.x >= -1e-12 && sample.point.y >= -1e-12);
            assert!(sample.point.x <= 1.0 + 1e-12 && sample.point.y <= 1.0 + 1e-12);
            assert!(sample.represented_area_m2 > 0.0);
        }
    }

    #[test]
    fn triangle_input_permutation_does_not_change_the_ordered_receipt() {
        let mut input = square("WALL", "wall", 1.0, SurfaceVisibility::Observable);
        input.extend(square("FLOOR", "floor", 2.0, SurfaceVisibility::Observable));
        let first = sample_observable_surfaces(&input, profile(0.1, 4)).expect("first receipt");
        input.reverse();
        let second = sample_observable_surfaces(&input, profile(0.1, 4)).expect("second receipt");
        assert_eq!(first, second);
    }

    #[test]
    fn seed_changes_points_but_preserves_sample_identity_and_count() {
        let input = square("WALL", "wall", 1.0, SurfaceVisibility::Observable);
        let first = sample_observable_surfaces(&input, profile(0.1, 1)).expect("seed one");
        let second = sample_observable_surfaces(&input, profile(0.1, 2)).expect("seed two");
        assert_eq!(first.sample_count, second.sample_count);
        for (left, right) in first.samples.iter().zip(&second.samples) {
            assert_eq!(left.surface_id, right.surface_id);
            assert_eq!(left.sample_ordinal, right.sample_ordinal);
        }
        assert_ne!(first.samples[0].point, second.samples[0].point);
        assert_ne!(first.sample_set_sha256, second.sample_set_sha256);
        assert_eq!(first.sample_set_sha256.len(), 64);
    }

    #[test]
    fn unobservable_area_is_reported_and_never_sampled() {
        let mut input = square("WALL", "visible", 1.0, SurfaceVisibility::Observable);
        input.extend(square(
            "WALL",
            "occluded",
            2.0,
            SurfaceVisibility::ExternallyOccluded,
        ));
        let receipt =
            sample_observable_surfaces(&input, profile(0.1, 12)).expect("visible wall samples");
        assert!((receipt.observable_area_m2 - 1.0).abs() < 1e-12);
        assert!((receipt.excluded_area_m2 - 2.0).abs() < 1e-12);
        assert_eq!(receipt.sample_count, 100);
        assert_eq!(receipt.surfaces[0].excluded_by_visibility.len(), 1);
        assert_eq!(
            receipt.surfaces[0].excluded_by_visibility[0].visibility,
            SurfaceVisibility::ExternallyOccluded
        );
        assert!(
            receipt
                .samples
                .iter()
                .all(|sample| sample.triangle_id.starts_with("visible-"))
        );
    }

    #[test]
    fn entirely_unobservable_mesh_has_an_empty_not_zero_error_receipt() {
        let input = square("WALL", "hidden", 1.0, SurfaceVisibility::Unknown);
        let receipt =
            sample_observable_surfaces(&input, profile(0.1, 12)).expect("visibility accounting");
        assert_eq!(receipt.sample_count, 0);
        assert!(receipt.samples.is_empty());
        assert_eq!(receipt.observable_area_m2, 0.0);
        assert!((receipt.excluded_area_m2 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn verifies_reference_sampling_receipt_and_rejects_digest_tampering() {
        let input = square("REFERENCE-WALL", "wall", 1.0, SurfaceVisibility::SelfOccluded);
        let receipt = sample_reference_surfaces_for_visibility(&input, profile(0.1, 31))
            .expect("reference sampling ignores prior observation labels");
        verify_surface_sampling_receipt(&receipt).expect("valid reference receipt");

        let mut tampered = receipt.clone();
        tampered.samples[0].point.x += 0.001;
        let error = verify_surface_sampling_receipt(&tampered)
            .expect_err("modified sample points must change the canonical digest");
        assert!(error.contains("canonical SHA-256"));
    }

    #[test]
    fn rejects_sampling_receipt_area_or_weight_inconsistency() {
        let input = square("REFERENCE-WALL", "wall", 1.0, SurfaceVisibility::Observable);
        let receipt = sample_observable_surfaces(&input, profile(0.1, 37))
            .expect("valid sample receipt");
        let mut area_tampered = receipt.clone();
        area_tampered.surfaces[0].observable_area_m2 += 0.25;
        let error = verify_surface_sampling_receipt(&area_tampered)
            .expect_err("surface area totals must conserve");
        assert!(error.contains("represented area"));

        let mut weight_tampered = receipt.clone();
        weight_tampered.samples[0].represented_area_m2 *= 2.0;
        let error = verify_surface_sampling_receipt(&weight_tampered)
            .expect_err("sample weights must be equal and conserve area");
        assert!(error.contains("sample weights are not equal"));
    }

    #[test]
    fn samples_preserve_outward_normal_from_reference_triangle_winding() {
        let input = square("WALL", "wall", 1.0, SurfaceVisibility::Observable);
        let receipt = sample_observable_surfaces(&input, profile(0.1, 17))
            .expect("valid oriented reference surface");
        assert!(
            receipt
                .samples
                .iter()
                .all(|sample| sample.normal.x.abs() < 1e-12
                    && sample.normal.y.abs() < 1e-12
                    && (sample.normal.z - 1.0).abs() < 1e-12)
        );
    }

    #[test]
    fn reference_visibility_probes_cover_hidden_reference_triangles() {
        let mut input = square("WALL", "visible", 1.0, SurfaceVisibility::Observable);
        input.extend(square(
            "WALL",
            "previously-hidden",
            2.0,
            SurfaceVisibility::SelfOccluded,
        ));
        let receipt = sample_reference_surfaces_for_visibility(&input, profile(0.1, 23))
            .expect("reference probes sample the entire reference mesh");
        assert!((receipt.observable_area_m2 - 3.0).abs() < 1e-12);
        assert_eq!(receipt.excluded_area_m2, 0.0);
        assert!(receipt.samples.iter().any(|sample| {
            sample.triangle_id.starts_with("previously-hidden-")
        }));
        assert_eq!(
            receipt.sample_count,
            receipt.samples.iter().filter(|sample| sample.normal.z > 0.99).count()
        );
    }

    #[test]
    fn sample_budget_overflow_fails_instead_of_truncating() {
        let input = square("WALL", "wall", 1.0, SurfaceVisibility::Observable);
        let constrained = AreaWeightedSamplingProfile {
            target_spacing_m: 0.001,
            seed: 1,
            max_samples: 10,
        };
        let error = sample_observable_surfaces(&input, constrained)
            .expect_err("the declared sample budget is a hard limit");
        assert!(error.contains("sample budget exceeded"));
    }

    #[test]
    fn duplicate_surface_triangle_identity_fails_closed() {
        let input = vec![
            triangle(
                "WALL",
                "duplicate",
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                SurfaceVisibility::Observable,
            ),
            triangle(
                "WALL",
                "duplicate",
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                SurfaceVisibility::Observable,
            ),
        ];
        let error = sample_observable_surfaces(&input, profile(0.1, 1))
            .expect_err("duplicate triangle IDs are ambiguous");
        assert!(error.contains("duplicate triangle identity"));
    }

    #[test]
    fn degenerate_and_non_finite_triangles_fail_even_when_occluded() {
        let degenerate = vec![triangle(
            "HIDDEN",
            "bad",
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [2.0, 2.0, 2.0],
            SurfaceVisibility::SelfOccluded,
        )];
        let error = sample_observable_surfaces(&degenerate, profile(0.1, 1))
            .expect_err("hidden degenerate geometry must still be rejected");
        assert!(error.contains("degenerate"));

        let mut non_finite = square("HIDDEN", "bad", 1.0, SurfaceVisibility::Unknown);
        non_finite[0].a.x = f64::NAN;
        let error = sample_observable_surfaces(&non_finite, profile(0.1, 1))
            .expect_err("hidden non-finite geometry must still be rejected");
        assert!(error.contains("non-finite"));
    }

    #[test]
    fn invalid_spacing_and_zero_sample_budget_fail_closed() {
        let input = square("WALL", "wall", 1.0, SurfaceVisibility::Observable);
        let error = sample_observable_surfaces(&input, profile(0.0, 1))
            .expect_err("zero spacing is invalid");
        assert!(error.contains("target_spacing_m"));
        let mut zero_budget = profile(0.1, 1);
        zero_budget.max_samples = 0;
        let error =
            sample_observable_surfaces(&input, zero_budget).expect_err("zero budget is invalid");
        assert!(error.contains("max_samples"));
    }
}
