// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Element-level Scan-to-BIM matching indices.
//!
//! Independently implements the 2026 Ersöz–Bosché coverage, distance and
//! normal-distribution definitions. The article-text and pinned released-code
//! histogram profiles are kept separate because they disagree.
//!
//! Comparator source: CyberbuildLab/pcd-bim-matching-confidence
//! commit ff21962c1c65ae9bfb966f21dfd66af0b8648a19 (Apache-2.0).

use crate::metrics::Point3;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

const UPSTREAM_KL_EPSILON: f64 = 1.0e-10;
const NORMAL_EPSILON: f64 = 1.0e-12;
const SAFE_CELL_LIMIT: f64 = 9.0e18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchingConfidenceProfile {
    /// Article text: five equal polar bands (pi/5), eight azimuth bands,
    /// and score 1 - D_KL(scan || BIM), without clamping.
    PaperTextV1,
    /// Pinned code: polar boundaries pi/8, 3pi/8, 5pi/8, 7pi/8;
    /// azimuth-independent caps; epsilon-smoothed KL and clamp to [0,1].
    UpstreamCodeFf21962V1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BimSurfaceSample {
    pub sample_id: String,
    pub point: Point3,
    /// Normal from BIM element geometry, not a noisy scan-derived estimate.
    pub normal: Point3,
    pub represented_area_m2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanSample {
    pub sample_id: String,
    pub point: Point3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchedPair {
    pub bim_sample_id: String,
    pub scan_sample_id: String,
    pub distance_m: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchingConfidenceReport {
    pub profile: MatchingConfidenceProfile,
    pub delta_m: f64,
    pub bim_sample_count: usize,
    pub matched_bim_sample_count: usize,
    pub unique_matched_scan_sample_count: usize,
    /// Published equation: |B'| / |B|.
    pub coverage_index_sample_count: f64,
    /// Luminous extension; area-weighted and deliberately separate.
    pub coverage_index_area_weighted: f64,
    /// Published equation: mean(1 - d_i / delta) over matched B'.
    pub distance_index_sample_count: Option<f64>,
    /// Luminous extension; area-weighted and deliberately separate.
    pub distance_index_area_weighted: Option<f64>,
    pub distribution_index_paper_text: Option<f64>,
    pub distribution_index_upstream_code: Option<f64>,
    pub distribution_index_selected: Option<f64>,
    /// A product of comparator indices, never a calibrated probability.
    pub confidence_product_selected: Option<f64>,
    pub bim_normal_histogram: [usize; 40],
    pub scan_normal_histogram: [usize; 40],
    pub matched_pairs: Vec<MatchedPair>,
}

#[derive(Debug, Clone, Copy)]
struct Cell(i64, i64, i64);

fn finite_point(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn normalise(vector: Point3) -> Result<Point3, String> {
    if !finite_point(vector) {
        return Err("BIM normal contains a non-finite component".to_owned());
    }
    let length = vector.x.hypot(vector.y).hypot(vector.z);
    if !length.is_finite() || length <= NORMAL_EPSILON {
        return Err("BIM normal must have finite positive length".to_owned());
    }
    Ok(Point3::new(
        vector.x / length,
        vector.y / length,
        vector.z / length,
    ))
}

fn cell_for(point: Point3, cell_size_m: f64) -> Result<Cell, String> {
    if !finite_point(point) {
        return Err("sample point contains a non-finite coordinate".to_owned());
    }
    let coordinates = [
        (point.x / cell_size_m).floor(),
        (point.y / cell_size_m).floor(),
        (point.z / cell_size_m).floor(),
    ];
    if coordinates
        .iter()
        .any(|value| !value.is_finite() || value.abs() >= SAFE_CELL_LIMIT)
    {
        return Err("spatial-hash cell coordinate exceeds supported integer range".to_owned());
    }
    Ok(Cell(
        coordinates[0] as i64,
        coordinates[1] as i64,
        coordinates[2] as i64,
    ))
}

fn squared_distance(left: Point3, right: Point3) -> Result<f64, String> {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    let dz = left.z - right.z;
    let distance_squared = dx.mul_add(dx, dy.mul_add(dy, dz * dz));
    if !distance_squared.is_finite() || distance_squared < 0.0 {
        return Err("matching distance overflowed or became non-finite".to_owned());
    }
    Ok(distance_squared)
}

fn histogram_bin(normal: Point3, profile: MatchingConfidenceProfile) -> Result<usize, String> {
    let n = normalise(normal)?;
    let theta = n.z.clamp(-1.0, 1.0).acos();
    let phi = n.y.atan2(n.x);

    let (polar_bin, azimuth_bin) = match profile {
        MatchingConfidenceProfile::PaperTextV1 => {
            let polar = ((theta / (PI / 5.0)).floor() as usize).min(4);
            let azimuth = (((phi + PI) / (PI / 4.0)).floor() as usize).min(7);
            (polar, azimuth)
        }
        MatchingConfidenceProfile::UpstreamCodeFf21962V1 => {
            let polar = if theta < PI / 8.0 {
                0
            } else if theta < 3.0 * PI / 8.0 {
                1
            } else if theta < 5.0 * PI / 8.0 {
                2
            } else if theta < 7.0 * PI / 8.0 {
                3
            } else {
                4
            };
            let azimuth =
                if polar == 0 || polar == 4 || !(-3.0 * PI / 4.0..3.0 * PI / 4.0).contains(&phi) {
                    0
                } else if phi < -PI / 2.0 {
                    1
                } else if phi < -PI / 4.0 {
                    2
                } else if phi < 0.0 {
                    3
                } else if phi < PI / 4.0 {
                    4
                } else if phi < PI / 2.0 {
                    5
                } else if phi < 3.0 * PI / 4.0 {
                    6
                } else {
                    7
                };
            (polar, azimuth)
        }
    };
    Ok(polar_bin * 8 + azimuth_bin)
}

fn normal_histogram(
    normals: &[Point3],
    profile: MatchingConfidenceProfile,
) -> Result<[usize; 40], String> {
    let mut histogram = [0usize; 40];
    for normal in normals {
        let bin = histogram_bin(*normal, profile)?;
        histogram[bin] = histogram[bin]
            .checked_add(1)
            .ok_or("normal histogram count overflow")?;
    }
    Ok(histogram)
}

fn kl_divergence(
    scan_histogram: &[usize; 40],
    bim_histogram: &[usize; 40],
    epsilon_smoothing: bool,
) -> Result<f64, String> {
    let scan_total: usize = scan_histogram.iter().sum();
    let bim_total: usize = bim_histogram.iter().sum();
    if scan_total == 0 || bim_total == 0 {
        return Err("normal histograms must both contain at least one sample".to_owned());
    }

    let mut p = [0.0f64; 40];
    let mut q = [0.0f64; 40];
    for index in 0..40 {
        p[index] = scan_histogram[index] as f64 / scan_total as f64;
        q[index] = bim_histogram[index] as f64 / bim_total as f64;
    }

    if epsilon_smoothing {
        for value in &mut p {
            *value = value.max(UPSTREAM_KL_EPSILON);
        }
        for value in &mut q {
            *value = value.max(UPSTREAM_KL_EPSILON);
        }
        let p_sum: f64 = p.iter().sum();
        let q_sum: f64 = q.iter().sum();
        for value in &mut p {
            *value /= p_sum;
        }
        for value in &mut q {
            *value /= q_sum;
        }
    }

    let mut divergence = 0.0;
    for index in 0..40 {
        if p[index] == 0.0 {
            continue;
        }
        if q[index] == 0.0 {
            return Err("scan normal histogram has mass in a zero-probability BIM bin".to_owned());
        }
        divergence += p[index] * (p[index] / q[index]).ln();
    }
    if !divergence.is_finite() || divergence < -1.0e-12 {
        return Err("KL divergence is non-finite or materially negative".to_owned());
    }
    Ok(divergence.max(0.0))
}

fn distribution_index(
    bim_normals: &[Point3],
    scan_normals: &[Point3],
    profile: MatchingConfidenceProfile,
) -> Result<(f64, [usize; 40], [usize; 40]), String> {
    let bim_histogram = normal_histogram(bim_normals, profile)?;
    let scan_histogram = normal_histogram(scan_normals, profile)?;
    let smooth = profile == MatchingConfidenceProfile::UpstreamCodeFf21962V1;
    let divergence = kl_divergence(&scan_histogram, &bim_histogram, smooth)?;
    let raw_index = 1.0 - divergence;
    let selected = match profile {
        MatchingConfidenceProfile::PaperTextV1 => raw_index,
        MatchingConfidenceProfile::UpstreamCodeFf21962V1 => raw_index.clamp(0.0, 1.0),
    };
    Ok((selected, bim_histogram, scan_histogram))
}

fn neighbour_cells(cell: Cell) -> impl Iterator<Item = Cell> {
    (-1i64..=1).flat_map(move |dx| {
        (-1i64..=1).flat_map(move |dy| {
            (-1i64..=1).map(move |dz| Cell(cell.0 + dx, cell.1 + dy, cell.2 + dz))
        })
    })
}

/// Evaluate the 2026 element-level matching indices.
///
/// Each BIM point is paired with its nearest scan point within delta (inclusive).
/// A scan point used by multiple BIM samples occurs once in S'; the first BIM
/// sample in stable ID order supplies its normal, as in the pinned implementation.
/// The report emits count-based published indices and separate area-weighted
/// extensions. Empty scan input is an explicit error, not a zero-confidence pass.
pub fn evaluate_matching_confidence(
    bim_samples: &[BimSurfaceSample],
    scan_samples: &[ScanSample],
    delta_m: f64,
    profile: MatchingConfidenceProfile,
) -> Result<MatchingConfidenceReport, String> {
    if !delta_m.is_finite() || delta_m <= 0.0 {
        return Err("delta_m must be finite positive metres".to_owned());
    }
    if bim_samples.is_empty() || scan_samples.is_empty() {
        return Err("BIM and scan sample populations must both be non-empty".to_owned());
    }
    let delta_squared = delta_m * delta_m;
    if !delta_squared.is_finite() || delta_squared <= 0.0 {
        return Err("delta_m squared must be finite positive".to_owned());
    }

    let mut bim_sorted: Vec<&BimSurfaceSample> = bim_samples.iter().collect();
    bim_sorted.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));
    let mut bim_ids = BTreeSet::new();
    let mut total_bim_area_m2 = 0.0;
    for sample in &bim_sorted {
        if sample.sample_id.trim().is_empty() || !bim_ids.insert(sample.sample_id.as_str()) {
            return Err("BIM sample IDs must be non-empty and unique".to_owned());
        }
        if !finite_point(sample.point) {
            return Err("BIM sample coordinate must be finite".to_owned());
        }
        let _ = normalise(sample.normal)?;
        if !sample.represented_area_m2.is_finite() || sample.represented_area_m2 <= 0.0 {
            return Err("BIM sample represented area must be finite and positive".to_owned());
        }
        total_bim_area_m2 += sample.represented_area_m2;
        if !total_bim_area_m2.is_finite() {
            return Err("BIM represented area accumulation overflowed".to_owned());
        }
    }

    let mut scan_sorted: Vec<&ScanSample> = scan_samples.iter().collect();
    scan_sorted.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));
    let mut scan_ids = BTreeSet::new();
    let mut spatial_index: BTreeMap<(i64, i64, i64), Vec<&ScanSample>> = BTreeMap::new();
    for sample in &scan_sorted {
        if sample.sample_id.trim().is_empty() || !scan_ids.insert(sample.sample_id.as_str()) {
            return Err("scan sample IDs must be non-empty and unique".to_owned());
        }
        let cell = cell_for(sample.point, delta_m)?;
        spatial_index
            .entry((cell.0, cell.1, cell.2))
            .or_default()
            .push(sample);
    }

    let mut matched_pairs = Vec::new();
    let mut matched_scan_normals: BTreeMap<String, Point3> = BTreeMap::new();
    let mut matched_area_m2 = 0.0;
    let mut distance_weighted_area = 0.0;

    for bim in &bim_sorted {
        let cell = cell_for(bim.point, delta_m)?;
        let mut best: Option<(&ScanSample, f64)> = None;
        for neighbour in neighbour_cells(cell) {
            if let Some(candidates) = spatial_index.get(&(neighbour.0, neighbour.1, neighbour.2)) {
                for scan in candidates {
                    let distance_squared = squared_distance(bim.point, scan.point)?;
                    let within_delta = distance_squared <= delta_squared;
                    let is_better = match best {
                        None => within_delta,
                        Some((best_scan, best_squared)) => {
                            within_delta
                                && (distance_squared < best_squared
                                    || (distance_squared == best_squared
                                        && scan.sample_id < best_scan.sample_id))
                        }
                    };
                    if is_better {
                        best = Some((scan, distance_squared));
                    }
                }
            }
        }

        if let Some((scan, distance_squared)) = best {
            let distance_m = distance_squared.sqrt();
            if !distance_m.is_finite() {
                return Err("matching distance became non-finite".to_owned());
            }
            matched_pairs.push(MatchedPair {
                bim_sample_id: bim.sample_id.clone(),
                scan_sample_id: scan.sample_id.clone(),
                distance_m,
            });
            matched_scan_normals
                .entry(scan.sample_id.clone())
                .or_insert(bim.normal);
            matched_area_m2 += bim.represented_area_m2;
            distance_weighted_area += bim.represented_area_m2 * (1.0 - distance_m / delta_m);
        }
    }

    if !matched_area_m2.is_finite() || !distance_weighted_area.is_finite() {
        return Err("matched area or distance accumulation overflowed".to_owned());
    }

    let matched_count = matched_pairs.len();
    let coverage_index_sample_count = matched_count as f64 / bim_sorted.len() as f64;
    let coverage_index_area_weighted = matched_area_m2 / total_bim_area_m2;
    let distance_index_sample_count = if matched_count == 0 {
        None
    } else {
        Some(
            matched_pairs
                .iter()
                .map(|pair| 1.0 - pair.distance_m / delta_m)
                .sum::<f64>()
                / matched_count as f64,
        )
    };
    let distance_index_area_weighted = if matched_area_m2 == 0.0 {
        None
    } else {
        Some(distance_weighted_area / matched_area_m2)
    };

    let mut paper_distribution = None;
    let mut upstream_distribution = None;
    let mut selected_distribution = None;
    let mut bim_histogram = [0usize; 40];
    let mut scan_histogram = [0usize; 40];
    if !matched_scan_normals.is_empty() {
        let bim_normals: Vec<Point3> = bim_sorted.iter().map(|sample| sample.normal).collect();
        let scan_normals: Vec<Point3> = matched_scan_normals.values().copied().collect();
        paper_distribution = Some(
            distribution_index(
                &bim_normals,
                &scan_normals,
                MatchingConfidenceProfile::PaperTextV1,
            )?
            .0,
        );
        upstream_distribution = Some(
            distribution_index(
                &bim_normals,
                &scan_normals,
                MatchingConfidenceProfile::UpstreamCodeFf21962V1,
            )?
            .0,
        );
        let (selected, bim_bins, scan_bins) =
            distribution_index(&bim_normals, &scan_normals, profile)?;
        selected_distribution = Some(selected);
        bim_histogram = bim_bins;
        scan_histogram = scan_bins;
    }

    let confidence_product_selected = match (distance_index_sample_count, selected_distribution) {
        (Some(distance), Some(distribution)) => {
            Some(coverage_index_sample_count * distance * distribution)
        }
        _ => None,
    };

    Ok(MatchingConfidenceReport {
        profile,
        delta_m,
        bim_sample_count: bim_sorted.len(),
        matched_bim_sample_count: matched_count,
        unique_matched_scan_sample_count: matched_scan_normals.len(),
        coverage_index_sample_count,
        coverage_index_area_weighted,
        distance_index_sample_count,
        distance_index_area_weighted,
        distribution_index_paper_text: paper_distribution,
        distribution_index_upstream_code: upstream_distribution,
        distribution_index_selected: selected_distribution,
        confidence_product_selected,
        bim_normal_histogram: bim_histogram,
        scan_normal_histogram: scan_histogram,
        matched_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        BimSurfaceSample, MatchingConfidenceProfile, ScanSample, evaluate_matching_confidence,
    };
    use crate::metrics::Point3;
    use std::f64::consts::PI;

    fn bim(id: &str, point: [f64; 3], normal: [f64; 3], area: f64) -> BimSurfaceSample {
        BimSurfaceSample {
            sample_id: id.to_owned(),
            point: Point3::new(point[0], point[1], point[2]),
            normal: Point3::new(normal[0], normal[1], normal[2]),
            represented_area_m2: area,
        }
    }

    fn scan(id: &str, point: [f64; 3]) -> ScanSample {
        ScanSample {
            sample_id: id.to_owned(),
            point: Point3::new(point[0], point[1], point[2]),
        }
    }

    #[test]
    fn exact_matches_produce_unit_coverage_distance_and_distribution() {
        let bim_samples = vec![
            bim("b0", [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0),
            bim("b1", [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0),
            bim("b2", [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], 1.0),
        ];
        let scan_samples = vec![
            scan("s0", [0.0, 0.0, 0.0]),
            scan("s1", [1.0, 0.0, 0.0]),
            scan("s2", [0.0, 1.0, 0.0]),
        ];
        let report = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("exact matches");
        assert_eq!(report.coverage_index_sample_count, 1.0);
        assert_eq!(report.coverage_index_area_weighted, 1.0);
        assert_eq!(report.distance_index_sample_count, Some(1.0));
        assert_eq!(report.distribution_index_paper_text, Some(1.0));
        assert_eq!(report.distribution_index_upstream_code, Some(1.0));
        assert_eq!(report.confidence_product_selected, Some(1.0));
    }

    #[test]
    fn coverage_and_distance_separate_unmatched_and_offset_samples() {
        let bim_samples = vec![
            bim("b0", [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0),
            bim("b1", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0),
        ];
        let scan_samples = vec![scan("s0", [0.0, 0.0, 0.025])];
        let report = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("one sample matched");
        assert_eq!(report.coverage_index_sample_count, 0.5);
        assert_eq!(report.distance_index_sample_count, Some(0.5));
        assert_eq!(report.matched_bim_sample_count, 1);
        assert_eq!(report.unique_matched_scan_sample_count, 1);
    }

    #[test]
    fn matching_threshold_is_inclusive() {
        let bim_samples = vec![
            bim("at-delta", [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0),
            bim("beyond", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0),
        ];
        let scan_samples = vec![
            scan("s0", [0.05, 0.0, 0.0]),
            scan("s1", [1.050_000_1, 0.0, 0.0]),
        ];
        let report = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("threshold matching");
        assert_eq!(report.matched_bim_sample_count, 1);
        assert_eq!(report.coverage_index_sample_count, 0.5);
        assert_eq!(report.distance_index_sample_count, Some(0.0));
    }

    #[test]
    fn scan_sample_reused_by_many_bim_samples_is_unique_in_s_prime() {
        let bim_samples = vec![
            bim("b0", [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0),
            bim("b1", [0.001, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0),
        ];
        let scan_samples = vec![scan("s0", [0.0005, 0.0, 0.0])];
        let report = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("one S sample may match multiple B samples");
        assert_eq!(report.matched_bim_sample_count, 2);
        assert_eq!(report.unique_matched_scan_sample_count, 1);
        assert_eq!(report.coverage_index_sample_count, 1.0);
    }

    #[test]
    fn area_weighted_coverage_is_separate_from_published_count_coverage() {
        let bim_samples = vec![
            bim("large", [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 9.0),
            bim("small", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0),
        ];
        let scan_samples = vec![scan("s0", [0.0, 0.0, 0.0])];
        let report = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("one of two samples matched");
        assert_eq!(report.coverage_index_sample_count, 0.5);
        assert!((report.coverage_index_area_weighted - 0.9).abs() < 1e-12);
    }

    #[test]
    fn input_permutations_do_not_change_the_matching_receipt() {
        let bim_samples = vec![
            bim("b1", [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0),
            bim("b0", [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0),
        ];
        let scan_samples = vec![scan("s1", [1.01, 0.0, 0.0]), scan("s0", [0.01, 0.0, 0.0])];
        let first = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("first result");
        let mut reversed_bim = bim_samples.clone();
        reversed_bim.reverse();
        let mut reversed_scan = scan_samples.clone();
        reversed_scan.reverse();
        let second = evaluate_matching_confidence(
            &reversed_bim,
            &reversed_scan,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("second result");
        assert_eq!(first, second);
    }

    #[test]
    fn paper_and_upstream_code_polar_caps_are_not_interchangeable() {
        // Two normals in a north-polar cap have different azimuths. The paper
        // text splits their azimuth bins; the upstream code collapses both into
        // its first polar bucket.
        let normals = [Point3::new(0.1, 0.0, 1.0), Point3::new(-0.1, 0.0, 1.0)];
        let bim_samples = vec![
            bim(
                "b0",
                [0.0, 0.0, 0.0],
                [normals[0].x, normals[0].y, normals[0].z],
                1.0,
            ),
            bim(
                "b1",
                [0.001, 0.0, 0.0],
                [normals[1].x, normals[1].y, normals[1].z],
                1.0,
            ),
        ];
        let scan_samples = vec![scan("s0", [0.0005, 0.0, 0.0])];
        let report = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("paper profile");
        assert_ne!(
            report.distribution_index_paper_text,
            report.distribution_index_upstream_code
        );
        assert_eq!(report.bim_normal_histogram.iter().sum::<usize>(), 2);
        assert_eq!(report.scan_normal_histogram.iter().sum::<usize>(), 1);
    }

    #[test]
    fn upstream_code_profile_clamps_negative_kl_similarity_to_zero() {
        let mut bim_samples = Vec::new();
        for index in 0..40 {
            let angle = 2.0 * PI * index as f64 / 40.0;
            bim_samples.push(bim(
                &format!("b{index:02}"),
                [index as f64 * 0.0001, 0.0, 0.0],
                [angle.cos(), angle.sin(), 0.0],
                1.0,
            ));
        }
        let scan_samples = vec![scan("s0", [0.0, 0.0, 0.0])];
        let paper = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect("paper profile preserves raw 1-KL");
        let upstream = evaluate_matching_confidence(
            &bim_samples,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::UpstreamCodeFf21962V1,
        )
        .expect("upstream code profile clamps 1-KL");
        assert!(paper.distribution_index_selected.expect("paper index") < 0.0);
        assert_eq!(upstream.distribution_index_selected, Some(0.0));
    }

    #[test]
    fn rejects_non_finite_points_invalid_normals_and_bad_thresholds() {
        let scan_samples = vec![scan("s0", [0.0, 0.0, 0.0])];
        let invalid_point = vec![bim("bad-point", [f64::NAN, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0)];
        let error = evaluate_matching_confidence(
            &invalid_point,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect_err("non-finite coordinates must fail closed");
        assert!(error.contains("finite"));

        let invalid_normal = vec![bim("bad-normal", [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0)];
        let error = evaluate_matching_confidence(
            &invalid_normal,
            &scan_samples,
            0.05,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect_err("zero normal must fail closed");
        assert!(error.contains("normal"));

        let valid = vec![bim("b0", [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0)];
        let error = evaluate_matching_confidence(
            &valid,
            &scan_samples,
            0.0,
            MatchingConfidenceProfile::PaperTextV1,
        )
        .expect_err("zero matching radius must fail closed");
        assert!(error.contains("delta_m"));
    }
}
