// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-attributed reproduction of the Ersöz–Bosché 2026 matching indices.
//!
//! This module evaluates the element-level metric layer from already sampled,
//! theoretically-visible BIM surface samples and already computed nearest-scan
//! correspondences. It deliberately does not implement IFC parsing, theoretical
//! visibility, Open3D-compatible mesh sampling, OBB cropping, voxel downsampling,
//! or a spatial index. Those inputs must be bound by the caller's replay receipt.
//!
//! Two profiles are retained because the pinned paper and released source differ:
//! - `paper_text_v1`: 5 equal polar bands × 8 azimuth bands, unsmoothed
//!   D_KL(P_scan || Q_bim), and the literal 1 - KL result (which may be negative
//!   or unbounded).
//! - `upstream_code_ff21962_v1`: exact released-code polar caps, epsilon
//!   smoothing/renormalization, and [0, 1] clamp.
//!
//! For unique scan-point normals, the released code transfers the normal from
//! the first matched BIM sample. The stable sample ID below represents that
//! original BIM sample order, making this tie rule reproducible under input
//! permutation when IDs are preserved.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use super::metrics::Point3;

pub const UPSTREAM_REPOSITORY: &str =
    "https://github.com/CyberbuildLab/pcd-bim-matching-confidence";
pub const UPSTREAM_COMMIT: &str = "ff21962c1c65ae9bfb966f21dfd66af0b8648a19";
pub const UPSTREAM_UTILS_BLOB: &str = "713122d4118d21250778e032beeba50858566f99";
pub const UPSTREAM_MAIN_BLOB: &str = "9482d2956045c39d791187755d24486c6a2f109a";
pub const PAPER_DOI: &str = "10.1016/j.autcon.2025.106636";
pub const UPSTREAM_KL_EPSILON: f64 = 1.0e-10;
const NORMAL_LENGTH_EPSILON: f64 = 1.0e-15;
const HISTOGRAM_BIN_COUNT: usize = 40;

/// A unit-direction vector is expected, but magnitude is normalized away when
/// converting to spherical angles. Zero and non-finite normals are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Normal3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Normal3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn spherical_angles(self) -> Result<(f64, f64), String> {
        if !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite() {
            return Err("normal contains a non-finite component".to_owned());
        }
        let horizontal = self.x.hypot(self.y);
        let length = horizontal.hypot(self.z);
        if !length.is_finite() || length <= NORMAL_LENGTH_EPSILON {
            return Err("normal must have a finite non-zero length".to_owned());
        }
        let theta = horizontal.atan2(self.z);
        let phi = self.y.atan2(self.x);
        if !theta.is_finite() || !phi.is_finite() {
            return Err("normal spherical conversion is non-finite".to_owned());
        }
        Ok((theta, phi))
    }
}

/// One sample from the theoretically-visible BIM surface population.
/// `sample_id` is a stable identity and, for exact upstream tie reproduction,
/// must preserve the original BIM sample order (e.g. its original ordinal).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BimSurfaceSample {
    pub sample_id: u64,
    pub normal: Normal3,
}

/// A nearest-neighbour result for one BIM sample. None means no scan
/// correspondence was found. Distances above delta are deliberately retained
/// as inputs but are not members of B' or S'.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NearestScanMatch {
    pub scan_point_id: u64,
    pub distance_m: f64,
}

/// Caller-supplied preprocessing parameters. The metric implementation records
/// these values but does not itself sample meshes, crop OBBs, or downsample scans.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MatchingConfidenceParameters {
    pub delta_m: f64,
    pub bim_surface_spacing_m: f64,
    pub scan_voxel_size_m: f64,
    /// Increase in the full OBB extent on each axis. The pinned code uses
    /// 2 * delta (= 0.10 m at its default 0.05 m threshold), i.e. 0.05 m
    /// additional margin on each side of the OBB.
    pub coarse_obb_extent_increment_m: f64,
}

impl Default for MatchingConfidenceParameters {
    fn default() -> Self {
        Self {
            delta_m: 0.05,
            bim_surface_spacing_m: 0.02,
            scan_voxel_size_m: 0.01,
            coarse_obb_extent_increment_m: 0.10,
        }
    }
}

impl MatchingConfidenceParameters {
    fn validate(self) -> Result<(), String> {
        for (name, value) in [
            ("delta_m", self.delta_m),
            ("bim_surface_spacing_m", self.bim_surface_spacing_m),
            ("scan_voxel_size_m", self.scan_voxel_size_m),
            (
                "coarse_obb_extent_increment_m",
                self.coarse_obb_extent_increment_m,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{name} must be finite and positive"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchingConfidenceProfile {
    PaperTextV1,
    UpstreamCodeFf21962V1,
}

impl MatchingConfidenceProfile {
    pub const fn profile_id(self) -> &'static str {
        match self {
            Self::PaperTextV1 => "paper_text_v1",
            Self::UpstreamCodeFf21962V1 => "upstream_code_ff21962_v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionDisposition {
    Qualified,
    EmptyHistogram,
    UnboundedKlDivergence,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatchingConfidenceResult {
    pub profile: MatchingConfidenceProfile,
    pub paper_doi: &'static str,
    pub upstream_repository: &'static str,
    pub upstream_commit: Option<&'static str>,
    pub upstream_utils_blob: Option<&'static str>,
    pub parameters: MatchingConfidenceParameters,
    pub bim_sample_count: usize,
    pub matched_bim_sample_count: usize,
    pub scan_candidate_point_count: usize,
    pub unique_matched_scan_point_count: usize,
    pub coverage_index: f64,
    pub distance_index: f64,
    pub mean_matched_distance_m: Option<f64>,
    pub kl_divergence_nats: Option<f64>,
    pub distribution_index: Option<f64>,
    pub distribution_disposition: DistributionDisposition,
    pub confidence_index: Option<f64>,
    /// Row-major 5 × 8 counts. These histograms are primary reproduction
    /// artifacts: a single scalar index is insufficient to audit the bins.
    pub bim_normal_histogram: Vec<u64>,
    pub scan_normal_histogram: Vec<u64>,
    pub normal_assignment: &'static str,
    pub duplicate_scan_match_tie_break: &'static str,
    pub negative_distribution_index_preserved: bool,
}

#[derive(Debug, Clone, Copy)]
struct DistributionValue {
    kl: Option<f64>,
    index: Option<f64>,
    disposition: DistributionDisposition,
}

fn spherical_bin(
    profile: MatchingConfidenceProfile,
    theta: f64,
    phi: f64,
) -> usize {
    let (theta_bin, phi_bin) = match profile {
        MatchingConfidenceProfile::PaperTextV1 => {
            let theta_index = ((theta / std::f64::consts::PI) * 5.0)
                .floor()
                .clamp(0.0, 4.0) as usize;
            let phi_index = (((phi + std::f64::consts::PI)
                / (std::f64::consts::PI / 4.0))
                .floor())
                .clamp(0.0, 7.0) as usize;
            (theta_index, phi_index)
        }
        MatchingConfidenceProfile::UpstreamCodeFf21962V1 => {
            let pi = std::f64::consts::PI;
            let theta_index = if theta < pi / 8.0 {
                0
            } else if theta < 3.0 * pi / 8.0 {
                1
            } else if theta < 5.0 * pi / 8.0 {
                2
            } else if theta < 7.0 * pi / 8.0 {
                3
            } else {
                4
            };
            // Exact branch behaviour from utils.py: the negative/positive
            // azimuth seam is folded into bin zero; phi == +pi also maps to 0.
            let phi_index = if theta_index == 0 || theta_index == 4 {
                0
            } else if phi < -3.0 * pi / 4.0 || phi >= pi {
                0
            } else if phi < -pi / 2.0 {
                1
            } else if phi < -pi / 4.0 {
                2
            } else if phi < 0.0 {
                3
            } else if phi < pi / 4.0 {
                4
            } else if phi < pi / 2.0 {
                5
            } else if phi < 3.0 * pi / 4.0 {
                6
            } else {
                7
            };
            (theta_index, phi_index)
        }
    };
    theta_bin * 8 + phi_bin
}

fn normal_histogram(
    profile: MatchingConfidenceProfile,
    normals: &[Normal3],
) -> Result<Vec<u64>, String> {
    let mut counts = vec![0_u64; HISTOGRAM_BIN_COUNT];
    for (index, normal) in normals.iter().copied().enumerate() {
        let (theta, phi) = normal
            .spherical_angles()
            .map_err(|error| format!("normal {index}: {error}"))?;
        let bin = spherical_bin(profile, theta, phi);
        counts[bin] = counts[bin]
            .checked_add(1)
            .ok_or_else(|| "normal histogram count overflow".to_owned())?;
    }
    Ok(counts)
}

fn sum_counts(counts: &[u64]) -> u64 {
    counts.iter().copied().sum()
}

fn paper_text_distribution(
    scan_counts: &[u64],
    bim_counts: &[u64],
) -> DistributionValue {
    let scan_total = sum_counts(scan_counts);
    let bim_total = sum_counts(bim_counts);
    if scan_total == 0 || bim_total == 0 {
        return DistributionValue {
            kl: None,
            index: None,
            disposition: DistributionDisposition::EmptyHistogram,
        };
    }

    let mut kl = 0.0;
    for (scan_count, bim_count) in scan_counts.iter().zip(bim_counts) {
        if *scan_count == 0 {
            continue;
        }
        if *bim_count == 0 {
            return DistributionValue {
                kl: None,
                index: None,
                disposition: DistributionDisposition::UnboundedKlDivergence,
            };
        }
        let p = *scan_count as f64 / scan_total as f64;
        let q = *bim_count as f64 / bim_total as f64;
        kl += p * (p / q).ln();
    }

    let index = 1.0 - kl;
    DistributionValue {
        kl: Some(kl),
        index: Some(index),
        disposition: DistributionDisposition::Qualified,
    }
}

fn upstream_code_distribution(
    scan_counts: &[u64],
    bim_counts: &[u64],
) -> DistributionValue {
    fn smoothed_probabilities(counts: &[u64]) -> Vec<f64> {
        let total = sum_counts(counts);
        counts
            .iter()
            .map(|count| {
                if total == 0 {
                    0.0
                } else {
                    *count as f64 / total as f64
                }
            })
            .map(|probability| probability.clamp(UPSTREAM_KL_EPSILON, 1.0))
            .collect()
    }

    let mut p = smoothed_probabilities(scan_counts);
    let mut q = smoothed_probabilities(bim_counts);
    let p_total: f64 = p.iter().sum();
    let q_total: f64 = q.iter().sum();
    for value in &mut p {
        *value /= p_total;
    }
    for value in &mut q {
        *value /= q_total;
    }

    let kl: f64 = p
        .iter()
        .zip(&q)
        .map(|(p_value, q_value)| p_value * (p_value / q_value).ln())
        .sum();
    let index = (1.0 - kl).clamp(0.0, 1.0);
    DistributionValue {
        kl: Some(kl),
        index: Some(index),
        disposition: DistributionDisposition::Qualified,
    }
}

fn distribution_value(
    profile: MatchingConfidenceProfile,
    scan_counts: &[u64],
    bim_counts: &[u64],
) -> DistributionValue {
    match profile {
        MatchingConfidenceProfile::PaperTextV1 => {
            paper_text_distribution(scan_counts, bim_counts)
        }
        MatchingConfidenceProfile::UpstreamCodeFf21962V1 => {
            upstream_code_distribution(scan_counts, bim_counts)
        }
    }
}

/// Evaluate the published matching indices from deterministic upstream
/// correspondences.
///
/// Inputs:
/// - `bim_samples`: samples of the element's theoretically-visible BIM surface
///   (normally emitted by #4225 at 0.02 m spacing);
/// - `scan_point_ids`: identities in the caller's already coarse-cropped and
///   downsampled scan subset (normally 0.01 m voxel size);
/// - `nearest_matches`: one nearest-scan result per BIM sample, in the same
///   order as `bim_samples`. Matches at exactly delta are included.
///
/// This function intentionally consumes nearest correspondences instead of
/// inventing a second spatial index. Every ID/distance is checked, and the
/// returned denominators and histograms make the metric independently auditable.
pub fn evaluate_matching_confidence(
    profile: MatchingConfidenceProfile,
    parameters: MatchingConfidenceParameters,
    bim_samples: &[BimSurfaceSample],
    scan_point_ids: &[u64],
    nearest_matches: &[Option<NearestScanMatch>],
) -> Result<MatchingConfidenceResult, String> {
    parameters.validate()?;
    if nearest_matches.len() != bim_samples.len() {
        return Err("nearest_matches must contain exactly one entry per BIM sample".to_owned());
    }

    let mut bim_ids = BTreeSet::new();
    for sample in bim_samples {
        if !bim_ids.insert(sample.sample_id) {
            return Err(format!("duplicate BIM sample ID: {}", sample.sample_id));
        }
        sample.normal.spherical_angles()?;
    }

    let mut scan_ids = BTreeSet::new();
    for scan_id in scan_point_ids {
        if !scan_ids.insert(*scan_id) {
            return Err(format!("duplicate scan point ID: {scan_id}"));
        }
    }

    let mut matched_bim_sample_count = 0_usize;
    let mut matched_distances = Vec::new();
    let mut matched_scan_normals: BTreeMap<u64, (u64, Normal3)> = BTreeMap::new();

    for (sample, nearest) in bim_samples.iter().zip(nearest_matches) {
        let Some(nearest) = nearest else {
            continue;
        };
        if !scan_ids.contains(&nearest.scan_point_id) {
            return Err(format!(
                "nearest match references unknown scan point ID: {}",
                nearest.scan_point_id
            ));
        }
        if !nearest.distance_m.is_finite() || nearest.distance_m < 0.0 {
            return Err("nearest-match distance must be finite and non-negative".to_owned());
        }
        if nearest.distance_m > parameters.delta_m {
            continue;
        }

        matched_bim_sample_count += 1;
        matched_distances.push(nearest.distance_m);
        matched_scan_normals
            .entry(nearest.scan_point_id)
            .and_modify(|(current_sample_id, current_normal)| {
                if sample.sample_id < *current_sample_id {
                    *current_sample_id = sample.sample_id;
                    *current_normal = sample.normal;
                }
            })
            .or_insert((sample.sample_id, sample.normal));
    }

    let bim_normals: Vec<Normal3> = bim_samples.iter().map(|sample| sample.normal).collect();
    let scan_normals: Vec<Normal3> = matched_scan_normals
        .values()
        .map(|(_, normal)| *normal)
        .collect();

    let bim_histogram = normal_histogram(profile, &bim_normals)?;
    let scan_histogram = normal_histogram(profile, &scan_normals)?;

    let coverage_index = if bim_samples.is_empty() || matched_bim_sample_count == 0 {
        0.0
    } else {
        matched_bim_sample_count as f64 / bim_samples.len() as f64
    };

    let mean_matched_distance_m = if matched_distances.is_empty() {
        None
    } else {
        Some(matched_distances.iter().sum::<f64>() / matched_distances.len() as f64)
    };
    let distance_index = if matched_distances.is_empty() {
        0.0
    } else {
        matched_distances
            .iter()
            .map(|distance| 1.0 - distance / parameters.delta_m)
            .sum::<f64>()
            / matched_distances.len() as f64
    };

    let distribution = distribution_value(profile, &scan_histogram, &bim_histogram);
    let confidence_index = distribution
        .index
        .map(|distribution_index| coverage_index * distance_index * distribution_index);

    let is_upstream = profile == MatchingConfidenceProfile::UpstreamCodeFf21962V1;
    Ok(MatchingConfidenceResult {
        profile,
        paper_doi: PAPER_DOI,
        upstream_repository: UPSTREAM_REPOSITORY,
        upstream_commit: is_upstream.then_some(UPSTREAM_COMMIT),
        upstream_utils_blob: is_upstream.then_some(UPSTREAM_UTILS_BLOB),
        parameters,
        bim_sample_count: bim_samples.len(),
        matched_bim_sample_count,
        scan_candidate_point_count: scan_point_ids.len(),
        unique_matched_scan_point_count: matched_scan_normals.len(),
        coverage_index,
        distance_index,
        mean_matched_distance_m,
        kl_divergence_nats: distribution.kl,
        distribution_index: distribution.index,
        distribution_disposition: distribution.disposition,
        confidence_index,
        bim_normal_histogram: bim_histogram,
        scan_normal_histogram: scan_histogram,
        normal_assignment: "mapped_from_matched_bim_sample",
        duplicate_scan_match_tie_break: "lowest_stable_bim_sample_id",
        negative_distribution_index_preserved:
            profile == MatchingConfidenceProfile::PaperTextV1,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DistributionDisposition, MatchingConfidenceParameters, MatchingConfidenceProfile,
        Normal3, BimSurfaceSample, NearestScanMatch, evaluate_matching_confidence,
        normal_histogram, paper_text_distribution, spherical_bin, upstream_code_distribution,
    };

    fn direction(theta: f64, phi: f64) -> Normal3 {
        Normal3::new(
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        )
    }

    fn bim(id: u64, normal: Normal3) -> BimSurfaceSample {
        BimSurfaceSample {
            sample_id: id,
            normal,
        }
    }

    fn default_params() -> MatchingConfidenceParameters {
        MatchingConfidenceParameters::default()
    }

    #[test]
    fn paper_text_and_upstream_code_polar_bins_are_intentionally_different() {
        let normal = direction(0.2, std::f64::consts::FRAC_PI_2);
        let (theta, phi) = normal.spherical_angles().expect("finite normal");
        assert_eq!(
            spherical_bin(MatchingConfidenceProfile::PaperTextV1, theta, phi),
            6
        );
        assert_eq!(
            spherical_bin(MatchingConfidenceProfile::UpstreamCodeFf21962V1, theta, phi),
            0
        );
    }

    #[test]
    fn paper_text_uses_five_equal_polar_bands_and_eight_azimuth_bands() {
        let pi = std::f64::consts::PI;
        for row in 0..5 {
            let theta = ((row as f64) + 0.5) * pi / 5.0;
            let bins: BTreeSet<usize> = (0..8)
                .map(|col| {
                    spherical_bin(
                        MatchingConfidenceProfile::PaperTextV1,
                        theta,
                        -pi + (col as f64 + 0.5) * pi / 4.0,
                    )
                })
                .collect();
            assert_eq!(bins.len(), 8);
            assert!(bins.iter().all(|bin| bin / 8 == row));
        }
    }

    #[test]
    fn upstream_code_uses_single_azimuth_independent_polar_cap_bins() {
        let pi = std::f64::consts::PI;
        for theta in [0.0, pi / 16.0, 7.0 * pi / 8.0, pi] {
            let bins: BTreeSet<usize> = (0..8)
                .map(|col| {
                    spherical_bin(
                        MatchingConfidenceProfile::UpstreamCodeFf21962V1,
                        theta,
                        -pi + (col as f64 + 0.5) * pi / 4.0,
                    )
                })
                .collect();
            assert_eq!(bins.len(), 1);
        }
    }

    #[test]
    fn paper_text_uses_literal_unsmoothed_directional_kl_and_preserves_negative_index() {
        let mut p = vec![0_u64; 40];
        let mut q = vec![0_u64; 40];
        p[0] = 90;
        p[1] = 10;
        q[0] = 10;
        q[1] = 90;
        let result = paper_text_distribution(&p, &q);
        assert_eq!(result.disposition, DistributionDisposition::Qualified);
        let expected = 1.0 - (9.0_f64 * 0.9_f64).ln() * 0.9 - (0.1_f64 / 0.9).ln() * 0.1;
        assert!((result.index.expect("finite index") - expected).abs() < 1e-12);
        assert!(result.index.expect("finite index") < 0.0);
    }

    #[test]
    fn paper_text_reports_unbounded_kl_when_scan_mass_has_no_bim_support() {
        let mut p = vec![0_u64; 40];
        let mut q = vec![0_u64; 40];
        p[0] = 1;
        q[1] = 1;
        let result = paper_text_distribution(&p, &q);
        assert_eq!(
            result.disposition,
            DistributionDisposition::UnboundedKlDivergence
        );
        assert_eq!(result.kl, None);
        assert_eq!(result.index, None);
    }

    #[test]
    fn upstream_code_smooths_zero_bins_and_clamps_distribution_to_unit_interval() {
        let mut p = vec![0_u64; 40];
        let mut q = vec![0_u64; 40];
        p[0] = 1;
        q[1] = 1;
        let result = upstream_code_distribution(&p, &q);
        assert_eq!(result.disposition, DistributionDisposition::Qualified);
        assert!(result.kl.expect("epsilon-smoothed KL").is_finite());
        assert_eq!(result.index, Some(0.0));
    }

    #[test]
    fn paper_text_empty_histogram_is_unavailable_but_upstream_code_smooths_to_uniform() {
        let empty = vec![0_u64; 40];
        let nonempty = {
            let mut counts = vec![0_u64; 40];
            counts[4] = 5;
            counts
        };
        assert_eq!(
            paper_text_distribution(&empty, &nonempty).disposition,
            DistributionDisposition::EmptyHistogram
        );
        let upstream = upstream_code_distribution(&empty, &empty);
        assert_eq!(upstream.index, Some(1.0));
        assert_eq!(upstream.kl, Some(0.0));
    }

    #[test]
    fn coverage_and_distance_match_inclusive_delta_rule() {
        let normals = direction(std::f64::consts::FRAC_PI_2, 0.0);
        let bim_samples = vec![bim(0, normals), bim(1, normals), bim(2, normals), bim(3, normals)];
        let scan_ids = vec![10, 11, 12, 13];
        let matches = vec![
            Some(NearestScanMatch { scan_point_id: 10, distance_m: 0.0 }),
            Some(NearestScanMatch { scan_point_id: 11, distance_m: 0.025 }),
            Some(NearestScanMatch { scan_point_id: 12, distance_m: 0.05 }),
            Some(NearestScanMatch { scan_point_id: 13, distance_m: 0.050_000_1 }),
        ];
        let result = evaluate_matching_confidence(
            MatchingConfidenceProfile::UpstreamCodeFf21962V1,
            default_params(),
            &bim_samples,
            &scan_ids,
            &matches,
        )
        .expect("inclusive threshold");
        assert_eq!(result.bim_sample_count, 4);
        assert_eq!(result.matched_bim_sample_count, 3);
        assert_eq!(result.unique_matched_scan_point_count, 3);
        assert!((result.coverage_index - 0.75).abs() < 1e-12);
        assert!((result.distance_index - 0.5).abs() < 1e-12);
    }

    #[test]
    fn unique_scan_subset_uses_first_stable_bim_sample_id_not_duplicate_matches() {
        let bim_samples = vec![
            bim(20, direction(std::f64::consts::FRAC_PI_2, std::f64::consts::PI)),
            bim(10, direction(std::f64::consts::FRAC_PI_2, 0.0)),
        ];
        let scan_ids = vec![100];
        let matches = vec![
            Some(NearestScanMatch { scan_point_id: 100, distance_m: 0.01 }),
            Some(NearestScanMatch { scan_point_id: 100, distance_m: 0.02 }),
        ];
        let result = evaluate_matching_confidence(
            MatchingConfidenceProfile::UpstreamCodeFf21962V1,
            default_params(),
            &bim_samples,
            &scan_ids,
            &matches,
        )
        .expect("deduplicated scan subset");
        assert_eq!(result.matched_bim_sample_count, 2);
        assert_eq!(result.unique_matched_scan_point_count, 1);
        let expected_scan_hist = normal_histogram(
            MatchingConfidenceProfile::UpstreamCodeFf21962V1,
            &[bim_samples[1].normal],
        )
        .expect("single mapped normal");
        assert_eq!(result.scan_normal_histogram, expected_scan_hist);
    }

    #[test]
    fn preserving_sample_ids_makes_duplicate_match_tie_mapping_permutation_invariant() {
        let n1 = direction(std::f64::consts::FRAC_PI_2, 0.0);
        let n2 = direction(std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        let bim_a = vec![bim(1, n1), bim(2, n2)];
        let matches_a = vec![
            Some(NearestScanMatch { scan_point_id: 7, distance_m: 0.01 }),
            Some(NearestScanMatch { scan_point_id: 7, distance_m: 0.02 }),
        ];
        let bim_b = vec![bim(2, n2), bim(1, n1)];
        let matches_b = vec![matches_a[1], matches_a[0]];
        let scan_ids = vec![7];
        let a = evaluate_matching_confidence(
            MatchingConfidenceProfile::UpstreamCodeFf21962V1,
            default_params(),
            &bim_a,
            &scan_ids,
            &matches_a,
        )
        .expect("original order");
        let b = evaluate_matching_confidence(
            MatchingConfidenceProfile::UpstreamCodeFf21962V1,
            default_params(),
            &bim_b,
            &scan_ids,
            &matches_b,
        )
        .expect("permuted arrays with preserved stable IDs");
        assert_eq!(a.scan_normal_histogram, b.scan_normal_histogram);
        assert_eq!(a.coverage_index, b.coverage_index);
        assert_eq!(a.distance_index, b.distance_index);
        assert_eq!(a.confidence_index, b.confidence_index);
    }

    #[test]
    fn rejects_duplicate_ids_invalid_distances_and_unknown_scan_references() {
        let normal = direction(std::f64::consts::FRAC_PI_2, 0.0);
        let duplicate_bim = vec![bim(1, normal), bim(1, normal)];
        let no_matches = vec![None, None];
        assert!(
            evaluate_matching_confidence(
                MatchingConfidenceProfile::PaperTextV1,
                default_params(),
                &duplicate_bim,
                &[9],
                &no_matches,
            )
            .is_err()
        );

        let valid_bim = vec![bim(1, normal)];
        let bad_distance = vec![Some(NearestScanMatch {
            scan_point_id: 9,
            distance_m: f64::NAN,
        })];
        assert!(
            evaluate_matching_confidence(
                MatchingConfidenceProfile::PaperTextV1,
                default_params(),
                &valid_bim,
                &[9],
                &bad_distance,
            )
            .is_err()
        );
        let unknown_scan = vec![Some(NearestScanMatch {
            scan_point_id: 99,
            distance_m: 0.01,
        })];
        assert!(
            evaluate_matching_confidence(
                MatchingConfidenceProfile::PaperTextV1,
                default_params(),
                &valid_bim,
                &[9],
                &unknown_scan,
            )
            .is_err()
        );
    }
}
