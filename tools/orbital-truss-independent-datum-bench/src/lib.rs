use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-truss-independent-datum.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-truss-independent-datum.receipt.v0.1";
pub const CLAIM_CEILING: &str = "synthetic_2d_external_datum_registration_only";
const GEOMETRY_EPS: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    Qualified,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub expected_disposition: Disposition,
    pub datum_origin: String,
    pub article_geometry_used: bool,
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub datum_set_id: String,
    pub minimum_span_m: f64,
    pub minimum_triangle_area_m2: f64,
    pub maximum_control_residual_m: f64,
    pub maximum_check_residual_m: f64,
    pub registration_error_bound_m: f64,
    pub controls: Vec<PointPair>,
    pub check_points: Vec<PointPair>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PointPair {
    pub point_id: String,
    pub source_position_m: [f64; 2],
    pub target_position_m: [f64; 2],
    pub source_error_bound_m: f64,
    pub target_error_bound_m: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PointResidual {
    pub point_id: String,
    pub residual_m: f64,
    pub allowed_residual_m: f64,
    pub within_bound: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub reason_codes: Vec<String>,
    pub rotation_radians: Option<f64>,
    pub translation_m: Option<[f64; 2]>,
    pub control_span_m: Option<f64>,
    pub maximum_triangle_area_m2: Option<f64>,
    pub control_residuals: Vec<PointResidual>,
    pub check_residuals: Vec<PointResidual>,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub case_count: usize,
    pub declared_matches: usize,
    pub declared_mismatches: usize,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone, Copy)]
struct Pose {
    angle: f64,
    cosine: f64,
    sine: f64,
    translation: [f64; 2],
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn finite_point(point: [f64; 2]) -> bool {
    point[0].is_finite() && point[1].is_finite()
}

fn transform(pose: Pose, point: [f64; 2]) -> [f64; 2] {
    [
        pose.cosine * point[0] - pose.sine * point[1] + pose.translation[0],
        pose.sine * point[0] + pose.cosine * point[1] + pose.translation[1],
    ]
}

fn distance(left: [f64; 2], right: [f64; 2]) -> f64 {
    (left[0] - right[0]).hypot(left[1] - right[1])
}

fn sorted_points(points: &[PointPair]) -> Vec<&PointPair> {
    let mut sorted: Vec<&PointPair> = points.iter().collect();
    sorted.sort_by(|left, right| left.point_id.cmp(&right.point_id));
    sorted
}

fn control_span(points: &[&PointPair]) -> f64 {
    let mut maximum = 0.0_f64;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            maximum = maximum.max(distance(
                points[i].source_position_m,
                points[j].source_position_m,
            ));
        }
    }
    maximum
}

fn maximum_triangle_area(points: &[&PointPair]) -> f64 {
    let mut maximum = 0.0_f64;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            for k in (j + 1)..points.len() {
                let a = points[i].source_position_m;
                let b = points[j].source_position_m;
                let c = points[k].source_position_m;
                let twice = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                maximum = maximum.max(twice.abs() * 0.5);
            }
        }
    }
    maximum
}

fn fit_proper_rigid_2d(points: &[&PointPair]) -> Option<Pose> {
    if points.len() < 2 {
        return None;
    }

    let count = points.len() as f64;
    let mut source_mean = [0.0, 0.0];
    let mut target_mean = [0.0, 0.0];

    for point in points {
        source_mean[0] += point.source_position_m[0] / count;
        source_mean[1] += point.source_position_m[1] / count;
        target_mean[0] += point.target_position_m[0] / count;
        target_mean[1] += point.target_position_m[1] / count;
    }

    let (mut dot, mut cross) = (0.0, 0.0);
    for point in points {
        let sx = point.source_position_m[0] - source_mean[0];
        let sy = point.source_position_m[1] - source_mean[1];
        let tx = point.target_position_m[0] - target_mean[0];
        let ty = point.target_position_m[1] - target_mean[1];
        dot += sx * tx + sy * ty;
        cross += sx * ty - sy * tx;
    }

    if !dot.is_finite() || !cross.is_finite() || dot.hypot(cross) <= GEOMETRY_EPS {
        return None;
    }

    let angle = cross.atan2(dot);
    let (cosine, sine) = (angle.cos(), angle.sin());
    let rotated_mean = [
        cosine * source_mean[0] - sine * source_mean[1],
        sine * source_mean[0] + cosine * source_mean[1],
    ];
    let translation = [
        target_mean[0] - rotated_mean[0],
        target_mean[1] - rotated_mean[1],
    ];

    if !angle.is_finite() || !finite_point(translation) {
        return None;
    }

    Some(Pose {
        angle,
        cosine,
        sine,
        translation,
    })
}

fn residuals(
    points: &[&PointPair],
    pose: Pose,
    base: f64,
    registration: f64,
) -> Vec<PointResidual> {
    points
        .iter()
        .map(|point| {
            let residual = distance(
                transform(pose, point.source_position_m),
                point.target_position_m,
            );
            let allowed =
                base + point.source_error_bound_m + point.target_error_bound_m + registration;
            PointResidual {
                point_id: point.point_id.clone(),
                residual_m: residual,
                allowed_residual_m: allowed,
                within_bound: residual <= allowed,
            }
        })
        .collect()
}

fn indeterminate(case: &Case, reasons: Vec<String>) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: Disposition::Indeterminate,
        reason_codes: reasons,
        rotation_radians: None,
        translation_m: None,
        control_span_m: None,
        maximum_triangle_area_m2: None,
        control_residuals: Vec::new(),
        check_residuals: Vec::new(),
        authority_granted: false,
    }
}

pub fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    if case.case_id.trim().is_empty()
        || !case.minimum_span_m.is_finite()
        || case.minimum_span_m <= 0.0
        || !case.minimum_triangle_area_m2.is_finite()
        || case.minimum_triangle_area_m2 <= 0.0
        || !case.maximum_control_residual_m.is_finite()
        || case.maximum_control_residual_m < 0.0
        || !case.maximum_check_residual_m.is_finite()
        || case.maximum_check_residual_m < 0.0
        || !case.registration_error_bound_m.is_finite()
        || case.registration_error_bound_m < 0.0
    {
        return Err(format!("CASE_CONTRACT_INVALID:{}", case.case_id));
    }

    let mut reasons = Vec::new();
    if case.datum_origin != "independent_external_reference_network" {
        reasons.push("DATUM_ORIGIN_NOT_INDEPENDENT_EXTERNAL_NETWORK".to_owned());
    }
    if case.article_geometry_used {
        reasons.push("ARTICLE_GEOMETRY_USED_FOR_REGISTRATION".to_owned());
    }
    if case.source_frame_id.trim().is_empty()
        || case.target_frame_id.trim().is_empty()
        || case.datum_set_id.trim().is_empty()
        || case.source_frame_id == case.target_frame_id
    {
        reasons.push("FRAME_OR_DATUM_IDENTITY_INVALID".to_owned());
    }
    if case.controls.len() < 3 {
        reasons.push("INSUFFICIENT_CONTROL_POINTS".to_owned());
    }
    if case.check_points.is_empty() {
        reasons.push("NO_INDEPENDENT_CHECK_POINTS".to_owned());
    }
    if case.controls.len() > 100 || case.check_points.len() > 100 {
        return Err(format!("POINT_COUNT_LIMIT_EXCEEDED:{}", case.case_id));
    }

    let mut point_ids = BTreeSet::new();
    for point in case.controls.iter().chain(case.check_points.iter()) {
        if point.point_id.trim().is_empty()
            || !point_ids.insert(point.point_id.clone())
            || !finite_point(point.source_position_m)
            || !finite_point(point.target_position_m)
            || !point.source_error_bound_m.is_finite()
            || point.source_error_bound_m < 0.0
            || !point.target_error_bound_m.is_finite()
            || point.target_error_bound_m < 0.0
        {
            reasons.push("POINT_IDENTITY_OR_COORDINATE_EVIDENCE_INVALID".to_owned());
            break;
        }
    }

    let controls = sorted_points(&case.controls);
    let checks = sorted_points(&case.check_points);
    if controls.len() < 3 || !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return Ok(indeterminate(case, reasons));
    }

    let span = control_span(&controls);
    let triangle_area = maximum_triangle_area(&controls);
    if span < case.minimum_span_m {
        reasons.push("CONTROL_NETWORK_SPAN_BELOW_MINIMUM".to_owned());
    }
    if triangle_area < case.minimum_triangle_area_m2 {
        reasons.push("CONTROL_NETWORK_SPATIAL_FOOTPRINT_BELOW_MINIMUM".to_owned());
    }

    let Some(pose) = fit_proper_rigid_2d(&controls) else {
        reasons.push("RIGID_POSE_FIT_UNOBSERVABLE".to_owned());
        reasons.sort();
        reasons.dedup();
        return Ok(indeterminate(case, reasons));
    };

    let control_residuals = residuals(
        &controls,
        pose,
        case.maximum_control_residual_m,
        case.registration_error_bound_m,
    );
    let check_residuals = residuals(
        &checks,
        pose,
        case.maximum_check_residual_m,
        case.registration_error_bound_m,
    );
    let control_mismatch = control_residuals.iter().any(|item| !item.within_bound);
    let check_mismatch = check_residuals.iter().any(|item| !item.within_bound);

    if control_mismatch {
        reasons.push("CONTROL_RESIDUAL_EXCEEDS_DECLARED_HARD_BOUND".to_owned());
    }
    if check_mismatch {
        reasons.push("WITHHELD_CHECK_RESIDUAL_EXCEEDS_DECLARED_HARD_BOUND".to_owned());
    }
    reasons.sort();
    reasons.dedup();

    let disposition = if control_mismatch || check_mismatch {
        Disposition::Rejected
    } else if !reasons.is_empty() {
        Disposition::Indeterminate
    } else {
        Disposition::Qualified
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        disposition,
        reason_codes: reasons,
        rotation_radians: Some(pose.angle),
        translation_m: Some(pose.translation),
        control_span_m: Some(span),
        maximum_triangle_area_m2: Some(triangle_area),
        control_residuals,
        check_residuals,
        authority_granted: false,
    })
}

pub fn audit_corpus(raw: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("SOURCE_COMMIT_MUST_BE_40_LOWERCASE_HEX".to_owned());
    }

    let corpus: Corpus =
        serde_json::from_slice(raw).map_err(|error| format!("CORPUS_JSON_INVALID:{error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != "ORBITAL-TRUSS-INDEPENDENT-DATUM-001"
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
    {
        return Err("CORPUS_CONTRACT_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.clone()) {
            return Err(format!("DUPLICATE_CASE_ID:{}", case.case_id));
        }
        results.push(evaluate_case(case)?);
    }
    results.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let expected: BTreeMap<&str, Disposition> = corpus
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case.expected_disposition))
        .collect();
    let declared_matches = results
        .iter()
        .filter(|result| {
            expected
                .get(result.case_id.as_str())
                .is_some_and(|value| result.disposition == *value)
        })
        .count();

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: corpus.benchmark_id,
        source_commit: source_commit.to_owned(),
        input_sha256: sha256_hex(raw),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        case_count: results.len(),
        declared_matches,
        declared_mismatches: results.len() - declared_matches,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/research/orbital-truss/fixtures/independent-datum-v0_1.json");
    const SOURCE: &str = "0123456789abcdef0123456789abcdef01234567";

    fn receipt() -> Receipt {
        audit_corpus(FIXTURE, SOURCE).expect("frozen datum corpus must audit")
    }

    #[test]
    fn frozen_adversarial_corpus_matches_all_declared_dispositions() {
        let report = receipt();
        assert_eq!(report.case_count, 14);
        assert_eq!(report.declared_matches, 14);
        assert_eq!(report.declared_mismatches, 0);
        assert!(!report.authority_granted);
        assert_eq!(report.physical_validation, "NOT_ASSESSED");
    }

    #[test]
    fn known_rotation_and_translation_are_recovered() {
        let report = receipt();
        let case = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-001-KNOWN-POSE")
            .unwrap();
        assert_eq!(case.disposition, Disposition::Qualified);
        assert!((case.rotation_radians.unwrap() - std::f64::consts::FRAC_PI_6).abs() < 1.0e-12);
        let translation = case.translation_m.unwrap();
        assert!((translation[0] - 2.0).abs() < 1.0e-12);
        assert!((translation[1] + 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn withheld_check_points_do_not_participate_in_fit() {
        let report = receipt();
        let baseline = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-001-KNOWN-POSE")
            .unwrap();
        let changed = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-008-HOLDOUT-MISMATCH")
            .unwrap();
        assert!(
            (baseline.rotation_radians.unwrap() - changed.rotation_radians.unwrap()).abs()
                < 1.0e-12
        );
        assert_eq!(changed.disposition, Disposition::Rejected);
    }

    #[test]
    fn input_permutation_preserves_the_fitted_pose() {
        let report = receipt();
        let baseline = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-001-KNOWN-POSE")
            .unwrap();
        let permuted = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-002-PERMUTED-INPUT")
            .unwrap();
        assert_eq!(baseline.disposition, Disposition::Qualified);
        assert_eq!(permuted.disposition, Disposition::Qualified);
        assert!(
            (baseline.rotation_radians.unwrap() - permuted.rotation_radians.unwrap()).abs()
                < 1.0e-12
        );
        let baseline_translation = baseline.translation_m.unwrap();
        let permuted_translation = permuted.translation_m.unwrap();
        assert!((baseline_translation[0] - permuted_translation[0]).abs() < 1.0e-12);
        assert!((baseline_translation[1] - permuted_translation[1]).abs() < 1.0e-12);
    }

    #[test]
    fn collinear_controls_fail_the_declared_footprint_policy() {
        let report = receipt();
        let case = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-004-COLLINEAR-NETWORK")
            .unwrap();
        assert_eq!(case.disposition, Disposition::Indeterminate);
        assert!(
            case.reason_codes
                .iter()
                .any(|reason| reason == "CONTROL_NETWORK_SPATIAL_FOOTPRINT_BELOW_MINIMUM")
        );
    }

    #[test]
    fn article_derived_datum_network_never_qualifies() {
        let report = receipt();
        let case = report
            .cases
            .iter()
            .find(|item| item.case_id == "IDN-007-ARTICLE-SURFACE")
            .unwrap();
        assert_eq!(case.disposition, Disposition::Indeterminate);
        assert!(
            case.reason_codes
                .iter()
                .any(|reason| reason == "DATUM_ORIGIN_NOT_INDEPENDENT_EXTERNAL_NETWORK")
        );
    }

    #[test]
    fn independent_coordinate_frame_reparameterization_preserves_the_pose() {
        fn next_unit(state: &mut u64) -> f64 {
            *state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            ((*state >> 11) as f64) / ((1_u64 << 53) as f64)
        }

        fn rotate(point: [f64; 2], angle: f64) -> [f64; 2] {
            let (cosine, sine) = (angle.cos(), angle.sin());
            [
                cosine * point[0] - sine * point[1],
                sine * point[0] + cosine * point[1],
            ]
        }

        fn transform(point: [f64; 2], angle: f64, offset: [f64; 2]) -> [f64; 2] {
            let rotated = rotate(point, angle);
            [rotated[0] + offset[0], rotated[1] + offset[1]]
        }

        let source_points = [[-2.0, -1.0], [3.0, -1.0], [-2.0, 4.0]];
        let check_points = [[1.0, 2.0], [2.5, 3.5]];
        let mut state = 0x4f52_4249_5441_4c31_u64;

        for sample in 0..128 {
            let physical_angle =
                next_unit(&mut state) * std::f64::consts::TAU - std::f64::consts::PI;
            let physical_translation = [
                next_unit(&mut state) * 200.0 - 100.0,
                next_unit(&mut state) * 200.0 - 100.0,
            ];
            let source_frame_angle =
                next_unit(&mut state) * std::f64::consts::TAU - std::f64::consts::PI;
            let source_frame_offset = [
                next_unit(&mut state) * 200.0 - 100.0,
                next_unit(&mut state) * 200.0 - 100.0,
            ];
            let target_frame_angle =
                next_unit(&mut state) * std::f64::consts::TAU - std::f64::consts::PI;
            let target_frame_offset = [
                next_unit(&mut state) * 200.0 - 100.0,
                next_unit(&mut state) * 200.0 - 100.0,
            ];

            let make_pair = |point_id: String, source: [f64; 2]| {
                let physical_target = transform(source, physical_angle, physical_translation);
                PointPair {
                    point_id,
                    source_position_m: transform(source, source_frame_angle, source_frame_offset),
                    target_position_m: transform(
                        physical_target,
                        target_frame_angle,
                        target_frame_offset,
                    ),
                    source_error_bound_m: 1.0e-7,
                    target_error_bound_m: 1.0e-7,
                }
            };

            let controls = source_points
                .iter()
                .enumerate()
                .map(|(index, point)| make_pair(format!("CTRL-{index}"), *point))
                .collect();
            let checks = check_points
                .iter()
                .enumerate()
                .map(|(index, point)| make_pair(format!("CHECK-{index}"), *point))
                .collect();

            let case = Case {
                case_id: format!("FRAME-REPARAM-{sample:03}"),
                expected_disposition: Disposition::Qualified,
                datum_origin: "independent_external_reference_network".to_owned(),
                article_geometry_used: false,
                source_frame_id: format!("SOURCE-FRAME-{sample:03}"),
                target_frame_id: format!("TARGET-FRAME-{sample:03}"),
                datum_set_id: format!("DATUM-SET-{sample:03}"),
                minimum_span_m: 1.0,
                minimum_triangle_area_m2: 1.0,
                maximum_control_residual_m: 1.0e-6,
                maximum_check_residual_m: 1.0e-6,
                registration_error_bound_m: 1.0e-7,
                controls,
                check_points: checks,
            };

            let result = evaluate_case(&case).expect("generated frame case must evaluate");
            assert_eq!(
                result.disposition,
                Disposition::Qualified,
                "sample {sample}"
            );
            assert!(
                result
                    .control_residuals
                    .iter()
                    .all(|item| item.within_bound),
                "control residual failed for sample {sample}"
            );
            assert!(
                result.check_residuals.iter().all(|item| item.within_bound),
                "withheld check failed for sample {sample}"
            );

            let expected_angle = target_frame_angle + physical_angle - source_frame_angle;
            let angle_error = (result.rotation_radians.unwrap() - expected_angle
                + std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            assert!(
                angle_error.abs() < 1.0e-10,
                "pose angle failed for sample {sample}"
            );

            let expected_rotated_translation = rotate(physical_translation, target_frame_angle);
            let expected_rotated_source_offset = rotate(source_frame_offset, expected_angle);
            let expected_translation = [
                expected_rotated_translation[0] + target_frame_offset[0]
                    - expected_rotated_source_offset[0],
                expected_rotated_translation[1] + target_frame_offset[1]
                    - expected_rotated_source_offset[1],
            ];
            let actual_translation = result.translation_m.unwrap();
            assert!(
                (actual_translation[0] - expected_translation[0]).abs() < 1.0e-8,
                "pose x translation failed for sample {sample}"
            );
            assert!(
                (actual_translation[1] - expected_translation[1]).abs() < 1.0e-8,
                "pose y translation failed for sample {sample}"
            );
        }
    }

    #[test]
    fn outliers_and_nonrigid_correspondence_reject_registration() {
        let report = receipt();
        for case_id in [
            "IDN-008-HOLDOUT-MISMATCH",
            "IDN-009-CONTROL-OUTLIER",
            "IDN-010-SCALE-MISMATCH",
            "IDN-011-REFLECTION",
        ] {
            let case = report
                .cases
                .iter()
                .find(|item| item.case_id == case_id)
                .unwrap();
            assert_eq!(case.disposition, Disposition::Rejected, "{case_id}");
        }
    }
}
