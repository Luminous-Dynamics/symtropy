//! Deterministic, synthetic 2D surface-profile measurement qualification.
//!
//! This oracle does not register 3D point clouds and does not determine weld
//! quality. Coordinates are integer micrometres in an already-qualified frame.

use serde::Deserialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize)]
pub struct Point {
    pub x_um: i64,
    pub z_um: i64,
    pub uncertainty_um: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Baseline {
    pub profile_id: String,
    pub scan_id: String,
    pub scan_sha256: String,
    pub registration_receipt_sha256: String,
    pub coordinate_frame: String,
    pub units: String,
    pub reference_points: Vec<Point>,
    pub observed_points: Vec<Point>,
    pub registration: RegistrationReceipt,
    pub domain: Domain,
    pub limits: QualityLimits,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegistrationReceipt {
    pub scan_sha256: String,
    pub receipt_sha256: String,
    pub target_frame: String,
    pub residual_um: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Domain {
    pub x_min_um: i64,
    pub x_max_um: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QualityLimits {
    pub maximum_point_gap_um: u64,
    pub coverage_gap_threshold_um: u64,
    pub minimum_coverage_permille: u64,
    pub maximum_point_uncertainty_um: u64,
    pub maximum_registration_residual_um: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CorpusCase {
    pub case_id: String,
    pub mutation: String,
    pub expected_disposition: String,
    pub expected_reason: Option<String>,
    pub expected_metrics: Option<Metrics>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Corpus {
    pub schema_version: String,
    pub benchmark_id: String,
    pub units: String,
    pub baseline: Baseline,
    pub cases: Vec<CorpusCase>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Metrics {
    pub coverage_permille: u64,
    pub maximum_gap_um: u64,
    pub registration_residual_um: u64,
    pub maximum_absolute_deviation_um: u64,
    pub mean_absolute_deviation_um: u64,
    pub absolute_profile_area_um2: u64,
    pub uncertainty_expanded_area_upper_um2: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    InvalidDomain,
    InvalidQualityLimits,
    EmptyIdentity,
    InvalidDigest,
    ScanDigestMismatch,
    RegistrationReceiptMismatch,
    FrameMismatch,
    UnitsMismatch,
    RegistrationResidualTooLarge,
    TooFewPoints,
    NonMonotonicStations,
    PointOutsideDomain,
    UncertaintyMissing,
    PointUncertaintyTooLarge,
    GapTooLarge,
    CoverageTooLow,
    StationGridMismatch,
    NumericOverflow,
}

impl BlockReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidDomain => "INVALID_DOMAIN",
            Self::InvalidQualityLimits => "INVALID_QUALITY_LIMITS",
            Self::EmptyIdentity => "EMPTY_IDENTITY",
            Self::InvalidDigest => "INVALID_DIGEST",
            Self::ScanDigestMismatch => "SCAN_DIGEST_MISMATCH",
            Self::RegistrationReceiptMismatch => "REGISTRATION_RECEIPT_MISMATCH",
            Self::FrameMismatch => "FRAME_MISMATCH",
            Self::UnitsMismatch => "UNITS_MISMATCH",
            Self::RegistrationResidualTooLarge => "REGISTRATION_RESIDUAL_TOO_LARGE",
            Self::TooFewPoints => "TOO_FEW_POINTS",
            Self::NonMonotonicStations => "NON_MONOTONIC_STATIONS",
            Self::PointOutsideDomain => "POINT_OUTSIDE_DOMAIN",
            Self::UncertaintyMissing => "UNCERTAINTY_MISSING",
            Self::PointUncertaintyTooLarge => "POINT_UNCERTAINTY_TOO_LARGE",
            Self::GapTooLarge => "GAP_TOO_LARGE",
            Self::CoverageTooLow => "COVERAGE_TOO_LOW",
            Self::StationGridMismatch => "STATION_GRID_MISMATCH",
            Self::NumericOverflow => "NUMERIC_OVERFLOW",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Assessment {
    Qualified(Metrics),
    Blocked(BlockReason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusSummary {
    pub cases: usize,
    pub qualified: usize,
    pub blocked: usize,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn checked_u64(value: u128) -> Result<u64, BlockReason> {
    u64::try_from(value).map_err(|_| BlockReason::NumericOverflow)
}

fn rounded_ratio(numerator: u128, denominator: u128) -> Result<u128, BlockReason> {
    if denominator == 0 {
        return Err(BlockReason::InvalidDomain);
    }
    numerator
        .checked_add(denominator / 2)
        .map(|value| value / denominator)
        .ok_or(BlockReason::NumericOverflow)
}

fn absolute_delta(left: i64, right: i64) -> u128 {
    (i128::from(left) - i128::from(right)).unsigned_abs()
}

fn checked_mul(left: u128, right: u128) -> Result<u128, BlockReason> {
    left.checked_mul(right).ok_or(BlockReason::NumericOverflow)
}

fn segment_absolute_area(
    dx_um: u128,
    left_delta: i128,
    right_delta: i128,
) -> Result<u128, BlockReason> {
    let left_abs = left_delta.unsigned_abs();
    let right_abs = right_delta.unsigned_abs();
    let crosses_zero = (left_delta < 0 && right_delta > 0) || (left_delta > 0 && right_delta < 0);

    if crosses_zero {
        let left_sq = checked_mul(left_abs, left_abs)?;
        let right_sq = checked_mul(right_abs, right_abs)?;
        let numerator = checked_mul(dx_um, left_sq.checked_add(right_sq)
            .ok_or(BlockReason::NumericOverflow)?)?;
        let denominator = checked_mul(2, left_abs.checked_add(right_abs)
            .ok_or(BlockReason::NumericOverflow)?)?;
        rounded_ratio(numerator, denominator)
    } else {
        let height_sum = left_abs
            .checked_add(right_abs)
            .ok_or(BlockReason::NumericOverflow)?;
        rounded_ratio(checked_mul(dx_um, height_sum)?, 2)
    }
}

fn segment_nonnegative_area(
    dx_um: u128,
    left_height_um: u128,
    right_height_um: u128,
) -> Result<u128, BlockReason> {
    let height_sum = left_height_um
        .checked_add(right_height_um)
        .ok_or(BlockReason::NumericOverflow)?;
    rounded_ratio(checked_mul(dx_um, height_sum)?, 2)
}

fn validate_profile_points(
    points: &[Point],
    domain: &Domain,
    limits: &QualityLimits,
) -> Result<(), BlockReason> {
    if points.len() < 2 {
        return Err(BlockReason::TooFewPoints);
    }
    let mut previous_x = None;
    for point in points {
        if let Some(previous) = previous_x {
            if point.x_um <= previous {
                return Err(BlockReason::NonMonotonicStations);
            }
        }
        previous_x = Some(point.x_um);
        if point.x_um < domain.x_min_um || point.x_um > domain.x_max_um {
            return Err(BlockReason::PointOutsideDomain);
        }
        if point.uncertainty_um == 0 {
            return Err(BlockReason::UncertaintyMissing);
        }
        if point.uncertainty_um > limits.maximum_point_uncertainty_um {
            return Err(BlockReason::PointUncertaintyTooLarge);
        }
    }
    Ok(())
}

fn coverage_metrics(
    points: &[Point],
    domain: &Domain,
    limits: &QualityLimits,
) -> Result<(u64, u64), BlockReason> {
    let domain_width = i128::from(domain.x_max_um) - i128::from(domain.x_min_um);
    if domain_width <= 0 {
        return Err(BlockReason::InvalidDomain);
    }
    let max_allowed = u128::from(limits.maximum_point_gap_um);
    let coverage_gap_threshold = u128::from(limits.coverage_gap_threshold_um);
    let mut maximum_gap = 0u128;
    let mut covered = 0u128;

    let left_gap = (i128::from(points[0].x_um) - i128::from(domain.x_min_um))
        .unsigned_abs();
    maximum_gap = maximum_gap.max(left_gap);
    if left_gap <= coverage_gap_threshold {
        covered = covered
            .checked_add(left_gap)
            .ok_or(BlockReason::NumericOverflow)?;
    }

    for pair in points.windows(2) {
        let gap = (i128::from(pair[1].x_um) - i128::from(pair[0].x_um))
            .unsigned_abs();
        maximum_gap = maximum_gap.max(gap);
        if gap <= coverage_gap_threshold {
            covered = covered.checked_add(gap).ok_or(BlockReason::NumericOverflow)?;
        }
    }

    let right_gap = (i128::from(domain.x_max_um) - i128::from(points[points.len() - 1].x_um))
        .unsigned_abs();
    maximum_gap = maximum_gap.max(right_gap);
    if right_gap <= coverage_gap_threshold {
        covered = covered
            .checked_add(right_gap)
            .ok_or(BlockReason::NumericOverflow)?;
    }

    let coverage = rounded_ratio(
        checked_mul(covered, 1_000)?,
        domain_width as u128,
    )?;
    if maximum_gap > max_allowed {
        return Err(BlockReason::GapTooLarge);
    }
    if coverage < u128::from(limits.minimum_coverage_permille) {
        return Err(BlockReason::CoverageTooLow);
    }
    Ok((checked_u64(coverage)?, checked_u64(maximum_gap)?))
}

/// Validate identity, registration, coverage and uncertainty before comparing
/// two piecewise-linear profiles on exactly the same x-station grid.
pub fn assess(baseline: &Baseline) -> Assessment {
    match assess_inner(baseline) {
        Ok(metrics) => Assessment::Qualified(metrics),
        Err(reason) => Assessment::Blocked(reason),
    }
}

fn assess_inner(baseline: &Baseline) -> Result<Metrics, BlockReason> {
    if baseline.domain.x_min_um >= baseline.domain.x_max_um {
        return Err(BlockReason::InvalidDomain);
    }
    if baseline.limits.maximum_point_gap_um == 0
        || baseline.limits.coverage_gap_threshold_um == 0
        || baseline.limits.coverage_gap_threshold_um > baseline.limits.maximum_point_gap_um
        || baseline.limits.minimum_coverage_permille == 0
        || baseline.limits.minimum_coverage_permille > 1_000
        || baseline.limits.maximum_point_uncertainty_um == 0
    {
        return Err(BlockReason::InvalidQualityLimits);
    }
    if baseline.profile_id.trim().is_empty()
        || baseline.scan_id.trim().is_empty()
        || baseline.coordinate_frame.trim().is_empty()
    {
        return Err(BlockReason::EmptyIdentity);
    }
    if !valid_digest(&baseline.scan_sha256)
        || !valid_digest(&baseline.registration_receipt_sha256)
        || !valid_digest(&baseline.registration.scan_sha256)
        || !valid_digest(&baseline.registration.receipt_sha256)
    {
        return Err(BlockReason::InvalidDigest);
    }
    if baseline.registration.scan_sha256 != baseline.scan_sha256 {
        return Err(BlockReason::ScanDigestMismatch);
    }
    if baseline.registration.receipt_sha256 != baseline.registration_receipt_sha256 {
        return Err(BlockReason::RegistrationReceiptMismatch);
    }
    if baseline.coordinate_frame != baseline.registration.target_frame {
        return Err(BlockReason::FrameMismatch);
    }
    if baseline.units != "um" {
        return Err(BlockReason::UnitsMismatch);
    }
    if baseline.registration.residual_um > baseline.limits.maximum_registration_residual_um {
        return Err(BlockReason::RegistrationResidualTooLarge);
    }

    validate_profile_points(
        &baseline.reference_points,
        &baseline.domain,
        &baseline.limits,
    )?;
    validate_profile_points(
        &baseline.observed_points,
        &baseline.domain,
        &baseline.limits,
    )?;

    let (coverage_permille, maximum_gap_um) = coverage_metrics(
        &baseline.observed_points,
        &baseline.domain,
        &baseline.limits,
    )?;
    let (reference_coverage, _) = coverage_metrics(
        &baseline.reference_points,
        &baseline.domain,
        &baseline.limits,
    )?;
    if reference_coverage < baseline.limits.minimum_coverage_permille {
        return Err(BlockReason::CoverageTooLow);
    }

    if baseline.reference_points.len() != baseline.observed_points.len()
        || baseline
            .reference_points
            .iter()
            .zip(&baseline.observed_points)
            .any(|(reference, observed)| reference.x_um != observed.x_um)
    {
        return Err(BlockReason::StationGridMismatch);
    }

    let mut maximum_absolute_deviation = 0u128;
    let mut sum_absolute_deviation = 0u128;
    let mut absolute_area = 0u128;
    let mut uncertainty_expanded_area = 0u128;

    for (reference, observed) in baseline
        .reference_points
        .iter()
        .zip(&baseline.observed_points)
    {
        let delta = i128::from(observed.z_um) - i128::from(reference.z_um);
        let absolute = delta.unsigned_abs();
        maximum_absolute_deviation = maximum_absolute_deviation.max(absolute);
        sum_absolute_deviation = sum_absolute_deviation
            .checked_add(absolute)
            .ok_or(BlockReason::NumericOverflow)?;
    }

    for index in 0..baseline.reference_points.len() - 1 {
        let reference_left = &baseline.reference_points[index];
        let reference_right = &baseline.reference_points[index + 1];
        let observed_left = &baseline.observed_points[index];
        let observed_right = &baseline.observed_points[index + 1];
        let dx = (i128::from(reference_right.x_um) - i128::from(reference_left.x_um))
            .unsigned_abs();
        let left_delta = i128::from(observed_left.z_um) - i128::from(reference_left.z_um);
        let right_delta = i128::from(observed_right.z_um) - i128::from(reference_right.z_um);

        absolute_area = absolute_area
            .checked_add(segment_absolute_area(dx, left_delta, right_delta)?)
            .ok_or(BlockReason::NumericOverflow)?;

        let left_upper = left_delta
            .unsigned_abs()
            .checked_add(u128::from(reference_left.uncertainty_um))
            .and_then(|value| value.checked_add(u128::from(observed_left.uncertainty_um)))
            .ok_or(BlockReason::NumericOverflow)?;
        let right_upper = right_delta
            .unsigned_abs()
            .checked_add(u128::from(reference_right.uncertainty_um))
            .and_then(|value| value.checked_add(u128::from(observed_right.uncertainty_um)))
            .ok_or(BlockReason::NumericOverflow)?;
        uncertainty_expanded_area = uncertainty_expanded_area
            .checked_add(segment_nonnegative_area(dx, left_upper, right_upper)?)
            .ok_or(BlockReason::NumericOverflow)?;
    }

    let mean_absolute_deviation = rounded_ratio(
        sum_absolute_deviation,
        baseline.reference_points.len() as u128,
    )?;

    Ok(Metrics {
        coverage_permille,
        maximum_gap_um,
        registration_residual_um: baseline.registration.residual_um,
        maximum_absolute_deviation_um: checked_u64(maximum_absolute_deviation)?,
        mean_absolute_deviation_um: checked_u64(mean_absolute_deviation)?,
        absolute_profile_area_um2: checked_u64(absolute_area)?,
        uncertainty_expanded_area_upper_um2: checked_u64(uncertainty_expanded_area)?,
    })
}

fn apply_mutation(
    baseline: &Baseline,
    mutation: &str,
) -> Result<Baseline, String> {
    let mut input = baseline.clone();
    match mutation {
        "none" => {}
        "uniform_offset_1000_um" => {
            for point in &mut input.observed_points {
                point.z_um = point.z_um.saturating_add(1_000);
            }
        }
        "sign_crossing_100_um" => {
            let values = [-100, 100, 0];
            if input.observed_points.len() != values.len() {
                return Err("sign-crossing fixture requires three observed points".to_owned());
            }
            for (point, value) in input.observed_points.iter_mut().zip(values) {
                point.z_um = value;
            }
        }
        "long_internal_gap" => {
            if input.observed_points.len() < 3 {
                return Err("gap fixture requires at least three observed points".to_owned());
            }
            input.observed_points.remove(1);
        }
        "wrong_coordinate_frame" => {
            input.coordinate_frame = "OTHER-COORDINATE-FRAME".to_owned();
        }
        "scan_digest_mismatch" => {
            input.registration.scan_sha256 = "c".repeat(64);
        }
        "registration_receipt_digest_mismatch" => {
            input.registration.receipt_sha256 = "d".repeat(64);
        }
        "registration_residual_over_limit" => {
            input.registration.residual_um =
                input.limits.maximum_registration_residual_um.saturating_add(1);
        }
        "point_uncertainty_over_limit" => {
            let point = input.observed_points.get_mut(1)
                .ok_or_else(|| "uncertainty fixture requires a middle point".to_owned())?;
            point.uncertainty_um = input.limits.maximum_point_uncertainty_um.saturating_add(1);
        }
        "duplicate_x_station" => {
            let x = input.observed_points.first()
                .ok_or_else(|| "duplicate fixture has no first point".to_owned())?.x_um;
            let point = input.observed_points.get_mut(1)
                .ok_or_else(|| "duplicate fixture has no second point".to_owned())?;
            point.x_um = x;
        }
        "non_monotonic_x_station" => {
            let point = input.observed_points.get_mut(1)
                .ok_or_else(|| "non-monotonic fixture has no middle point".to_owned())?;
            point.x_um = input.domain.x_max_um;
        }
        "station_grid_mismatch" => {
            let point = input.observed_points.get_mut(1)
                .ok_or_else(|| "station mismatch fixture has no middle point".to_owned())?;
            point.x_um = point.x_um.saturating_add(1);
        }
        "zero_point_uncertainty" => {
            let point = input.observed_points.get_mut(1)
                .ok_or_else(|| "zero uncertainty fixture has no middle point".to_owned())?;
            point.uncertainty_um = 0;
        }
        "invalid_domain" => {
            input.domain.x_max_um = input.domain.x_min_um;
        }
        "coverage_below_minimum" => {
            let point = input.observed_points.get_mut(1)
                .ok_or_else(|| "coverage fixture has no middle point".to_owned())?;
            point.x_um = input.domain.x_min_um.saturating_add(1_150);
        }
        "point_outside_domain" => {
            let point = input.observed_points.last_mut()
                .ok_or_else(|| "domain fixture has no final point".to_owned())?;
            point.x_um = input.domain.x_max_um.saturating_add(1);
        }
        _ => return Err(format!("unknown mutation: {mutation}")),
    }
    Ok(input)
}

pub fn verify_corpus(corpus: &Corpus) -> Result<CorpusSummary, Vec<String>> {
    let mut errors = Vec::new();
    if corpus.schema_version != "0.1.0" {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if corpus.benchmark_id != "SHIPYARD-GEOMETRY-MEASUREMENT-ORACLE-001" {
        errors.push("benchmark_id mismatch".to_owned());
    }
    if corpus.units != "um" {
        errors.push("corpus units must be um".to_owned());
    }
    if corpus.claim_ceiling != "SYNTHETIC_2D_PROFILE_COMPARISON_ONLY" {
        errors.push("claim ceiling mismatch".to_owned());
    }
    if corpus.cases.len() != 16 {
        errors.push(format!("expected 16 cases, found {}", corpus.cases.len()));
    }

    let mut case_ids = HashSet::new();
    let mut qualified = 0usize;
    let mut blocked = 0usize;

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            errors.push(format!("duplicate case_id {}", case.case_id));
            continue;
        }
        let input = match apply_mutation(&corpus.baseline, &case.mutation) {
            Ok(input) => input,
            Err(error) => {
                errors.push(format!("{}: {error}", case.case_id));
                continue;
            }
        };
        match assess(&input) {
            Assessment::Qualified(metrics) => {
                qualified += 1;
                if case.expected_disposition != "QUALIFIED" {
                    errors.push(format!(
                        "{}: expected {}, observed QUALIFIED",
                        case.case_id, case.expected_disposition
                    ));
                }
                if case.expected_metrics.as_ref() != Some(&metrics) {
                    errors.push(format!("{}: known-answer metrics mismatch", case.case_id));
                }
                if case.expected_reason.is_some() {
                    errors.push(format!("{}: positive case must not declare a block reason", case.case_id));
                }
            }
            Assessment::Blocked(reason) => {
                blocked += 1;
                let expected = format!("BLOCK_{}", reason.as_str());
                if case.expected_disposition != expected {
                    errors.push(format!(
                        "{}: expected {}, observed {}",
                        case.case_id, case.expected_disposition, expected
                    ));
                }
                if case.expected_reason.as_deref() != Some(reason.as_str()) {
                    errors.push(format!("{}: expected_reason mismatch", case.case_id));
                }
                if case.expected_metrics.is_some() {
                    errors.push(format!("{}: blocked case must not declare metrics", case.case_id));
                }
            }
        }
    }

    if qualified != 3 || blocked != 13 {
        errors.push(format!(
            "expected 3 positive / 13 negative cases, observed {qualified}/{blocked}"
        ));
    }

    if errors.is_empty() {
        Ok(CorpusSummary {
            cases: corpus.cases.len(),
            qualified,
            blocked,
        })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS_TEXT: &str = include_str!(
        "../../../../benchmarks/shipyard/geometry-measurement-oracle/corpus-v0.1.json"
    );

    fn corpus() -> Corpus {
        serde_json::from_str(CORPUS_TEXT).expect("frozen geometry corpus must parse")
    }

    #[test]
    fn all_fourteen_frozen_cases_match_expected_dispositions() {
        let summary = verify_corpus(&corpus()).expect("all known answers must reproduce");
        assert_eq!(
            summary,
            CorpusSummary {
                cases: 16,
                qualified: 3,
                blocked: 13,
            }
        );
    }

    #[test]
    fn exact_match_has_zero_observed_area_and_bounded_uncertainty_area() {
        let corpus = corpus();
        let Assessment::Qualified(metrics) = assess(
            &apply_mutation(&corpus.baseline, "none").expect("baseline mutation"),
        ) else {
            panic!("exact match should qualify");
        };
        assert_eq!(metrics.maximum_absolute_deviation_um, 0);
        assert_eq!(metrics.mean_absolute_deviation_um, 0);
        assert_eq!(metrics.absolute_profile_area_um2, 0);
        assert_eq!(metrics.uncertainty_expanded_area_upper_um2, 6_000);
    }

    #[test]
    fn uniform_offset_has_exact_area_answer() {
        let corpus = corpus();
        let Assessment::Qualified(metrics) = assess(
            &apply_mutation(&corpus.baseline, "uniform_offset_1000_um")
                .expect("offset mutation"),
        ) else {
            panic!("uniform offset should qualify");
        };
        assert_eq!(metrics.maximum_absolute_deviation_um, 1_000);
        assert_eq!(metrics.mean_absolute_deviation_um, 1_000);
        assert_eq!(metrics.absolute_profile_area_um2, 2_000_000);
        assert_eq!(metrics.uncertainty_expanded_area_upper_um2, 2_006_000);
    }

    #[test]
    fn sign_crossing_integrates_two_triangles() {
        let corpus = corpus();
        let Assessment::Qualified(metrics) = assess(
            &apply_mutation(&corpus.baseline, "sign_crossing_100_um")
                .expect("crossing mutation"),
        ) else {
            panic!("sign crossing should qualify");
        };
        assert_eq!(metrics.maximum_absolute_deviation_um, 100);
        assert_eq!(metrics.mean_absolute_deviation_um, 67);
        assert_eq!(metrics.absolute_profile_area_um2, 100_000);
        assert_eq!(metrics.uncertainty_expanded_area_upper_um2, 156_000);
    }

    #[test]
    fn long_internal_gap_is_not_hidden_by_point_count() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "long_internal_gap")
            .expect("gap mutation");
        assert_eq!(assess(&input), Assessment::Blocked(BlockReason::GapTooLarge));
    }

    #[test]
    fn scan_digest_must_match_registration_receipt() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "scan_digest_mismatch")
            .expect("digest mutation");
        assert_eq!(assess(&input), Assessment::Blocked(BlockReason::ScanDigestMismatch));
    }

    #[test]
    fn transform_receipt_must_match_profile_receipt_digest() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "registration_receipt_digest_mismatch")
            .expect("receipt mutation");
        assert_eq!(
            assess(&input),
            Assessment::Blocked(BlockReason::RegistrationReceiptMismatch)
        );
    }

    #[test]
    fn excessive_registration_residual_blocks_geometry_comparison() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "registration_residual_over_limit")
            .expect("registration residual mutation");
        assert_eq!(
            assess(&input),
            Assessment::Blocked(BlockReason::RegistrationResidualTooLarge)
        );
    }

    #[test]
    fn missing_uncertainty_is_not_zero() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "zero_point_uncertainty")
            .expect("uncertainty mutation");
        assert_eq!(
            assess(&input),
            Assessment::Blocked(BlockReason::UncertaintyMissing)
        );
    }

    #[test]
    fn duplicate_and_non_monotonic_stations_are_rejected() {
        let corpus = corpus();
        for mutation in ["duplicate_x_station", "non_monotonic_x_station"] {
            let input = apply_mutation(&corpus.baseline, mutation).expect("station mutation");
            assert_eq!(
                assess(&input),
                Assessment::Blocked(BlockReason::NonMonotonicStations)
            );
        }
    }

    #[test]
    fn reference_and_observed_station_grids_must_match() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "station_grid_mismatch")
            .expect("station grid mutation");
        assert_eq!(
            assess(&input),
            Assessment::Blocked(BlockReason::StationGridMismatch)
        );
    }

    #[test]
    fn wrong_frame_is_rejected_before_geometry_scoring() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "wrong_coordinate_frame")
            .expect("frame mutation");
        assert_eq!(assess(&input), Assessment::Blocked(BlockReason::FrameMismatch));
    }

    #[test]
    fn nonpositive_domain_is_rejected() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "invalid_domain")
            .expect("domain mutation");
        assert_eq!(assess(&input), Assessment::Blocked(BlockReason::InvalidDomain));
    }

    #[test]
    fn aggregate_coverage_has_a_threshold_distinct_from_maximum_gap() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "coverage_below_minimum")
            .expect("coverage mutation");
        assert_eq!(assess(&input), Assessment::Blocked(BlockReason::CoverageTooLow));
    }

    #[test]
    fn point_outside_declared_profile_domain_is_rejected() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "point_outside_domain")
            .expect("domain point mutation");
        assert_eq!(
            assess(&input),
            Assessment::Blocked(BlockReason::PointOutsideDomain)
        );
    }

    #[test]
    fn point_uncertainty_above_limit_is_rejected() {
        let corpus = corpus();
        let input = apply_mutation(&corpus.baseline, "point_uncertainty_over_limit")
            .expect("uncertainty limit mutation");
        assert_eq!(
            assess(&input),
            Assessment::Blocked(BlockReason::PointUncertaintyTooLarge)
        );
    }
}
