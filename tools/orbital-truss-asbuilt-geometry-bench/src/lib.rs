use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-truss-asbuilt-geometry.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-truss-asbuilt-geometry.receipt.v0.1";
pub const CLAIM_CEILING: &str = "synthetic_2d_asbuilt_geometry_conformance_only";
const EPS: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    Accept,
    Reject,
    IndeterminateHumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureDisposition {
    WithinTolerance,
    OutsideTolerance,
    Unresolved,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub case_class: String,
    pub expected_disposition: Disposition,
    pub expected_subject_id: String,
    pub evidence_subject_id: String,
    pub design_revision: String,
    pub evidence_design_revision: String,
    pub source_time_unix_ms: u64,
    pub registration: Registration,
    pub instrument: Instrument,
    pub required_features: Vec<FeatureRequirement>,
    pub measurements: Vec<FeatureMeasurement>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub basis: String,
    pub independent_datum_set_id: String,
    pub control_point_count: u32,
    pub part_geometry_used_for_fit: bool,
    pub rotation: [[f64; 2]; 2],
    pub translation_m: [f64; 2],
    pub registration_uncertainty_m: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub instrument_id: String,
    pub calibration_id: String,
    pub calibration_evidence_sha256: String,
    pub valid_from_unix_ms: u64,
    pub valid_until_unix_ms: u64,
    pub used_at_unix_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureRequirement {
    pub feature_id: String,
    pub nominal_position_m: [f64; 2],
    pub tolerance_m: f64,
    pub critical: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureMeasurement {
    pub feature_id: String,
    pub observed_position_source_m: [f64; 2],
    pub measurement_uncertainty_m: f64,
    pub observed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeatureResult {
    pub feature_id: String,
    pub critical: bool,
    pub disposition: FeatureDisposition,
    pub residual_m: Option<f64>,
    pub uncertainty_radius_m: Option<f64>,
    pub lower_error_bound_m: Option<f64>,
    pub upper_error_bound_m: Option<f64>,
    pub tolerance_m: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub reason_codes: Vec<String>,
    pub features: Vec<FeatureResult>,
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
    pub structural_acceptance_authorized: bool,
    pub flight_authorized: bool,
    pub physical_validation: String,
    pub case_count: usize,
    pub oracle_matches: usize,
    pub oracle_mismatches: usize,
    pub cases: Vec<CaseResult>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn finite_pair(value: [f64; 2]) -> bool {
    value[0].is_finite() && value[1].is_finite()
}

fn unresolved_feature(requirement: &FeatureRequirement) -> FeatureResult {
    FeatureResult {
        feature_id: requirement.feature_id.clone(),
        critical: requirement.critical,
        disposition: FeatureDisposition::Unresolved,
        residual_m: None,
        uncertainty_radius_m: None,
        lower_error_bound_m: None,
        upper_error_bound_m: None,
        tolerance_m: requirement.tolerance_m,
    }
}

fn validate_registration(registration: &Registration) -> Result<(), String> {
    if registration.source_frame_id.trim().is_empty()
        || registration.target_frame_id.trim().is_empty()
        || registration.independent_datum_set_id.trim().is_empty()
        || registration.basis != "independent_external_datums"
        || registration.part_geometry_used_for_fit
        || registration.control_point_count < 2
        || !registration.registration_uncertainty_m.is_finite()
        || registration.registration_uncertainty_m < 0.0
        || !finite_pair(registration.translation_m)
        || registration.rotation.iter().flatten().any(|value| !value.is_finite())
    {
        return Err("REGISTRATION_EVIDENCE_INVALID".to_owned());
    }

    let [[a, b], [c, d]] = registration.rotation;
    let row0_norm = a * a + b * b;
    let row1_norm = c * c + d * d;
    let row_dot = a * c + b * d;
    let determinant = a * d - b * c;
    if (row0_norm - 1.0).abs() > EPS
        || (row1_norm - 1.0).abs() > EPS
        || row_dot.abs() > EPS
        || (determinant - 1.0).abs() > EPS
    {
        return Err("REGISTRATION_TRANSFORM_NOT_PROPER_RIGID_2D".to_owned());
    }
    Ok(())
}

fn validate_instrument(instrument: &Instrument, source_time_unix_ms: u64) -> Result<(), String> {
    let digest_valid = instrument.calibration_evidence_sha256.len() == 64
        && instrument
            .calibration_evidence_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if instrument.instrument_id.trim().is_empty()
        || instrument.calibration_id.trim().is_empty()
        || !digest_valid
        || instrument.valid_from_unix_ms > instrument.valid_until_unix_ms
        || instrument.used_at_unix_ms < instrument.valid_from_unix_ms
        || instrument.used_at_unix_ms > instrument.valid_until_unix_ms
        || instrument.used_at_unix_ms != source_time_unix_ms
    {
        return Err("CALIBRATION_EVIDENCE_INVALID_OR_OUT_OF_WINDOW".to_owned());
    }
    Ok(())
}

fn apply_registration(registration: &Registration, source: [f64; 2]) -> [f64; 2] {
    let [[a, b], [c, d]] = registration.rotation;
    [
        a * source[0] + b * source[1] + registration.translation_m[0],
        c * source[0] + d * source[1] + registration.translation_m[1],
    ]
}

fn distance(left: [f64; 2], right: [f64; 2]) -> f64 {
    (left[0] - right[0]).hypot(left[1] - right[1])
}

pub fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    if case.case_id.trim().is_empty()
        || case.expected_subject_id.trim().is_empty()
        || case.evidence_subject_id.trim().is_empty()
        || case.design_revision.trim().is_empty()
        || case.evidence_design_revision.trim().is_empty()
        || case.required_features.is_empty()
    {
        return Err("CASE_IDENTITY_OR_REQUIREMENTS_INVALID".to_owned());
    }

    let mut feature_ids = BTreeSet::new();
    for requirement in &case.required_features {
        if requirement.feature_id.trim().is_empty()
            || !feature_ids.insert(requirement.feature_id.clone())
            || !finite_pair(requirement.nominal_position_m)
            || !requirement.tolerance_m.is_finite()
            || requirement.tolerance_m <= 0.0
        {
            return Err("REQUIRED_FEATURE_CONTRACT_INVALID".to_owned());
        }
    }

    let mut measurement_map = BTreeMap::new();
    let mut duplicate_measurements = false;
    for measurement in &case.measurements {
        if !feature_ids.contains(&measurement.feature_id) {
            return Err(format!("UNDECLARED_MEASUREMENT_FEATURE:{}", measurement.feature_id));
        }
        if measurement_map.insert(measurement.feature_id.clone(), measurement).is_some() {
            duplicate_measurements = true;
        }
        if !finite_pair(measurement.observed_position_source_m)
            || !measurement.measurement_uncertainty_m.is_finite()
            || measurement.measurement_uncertainty_m < 0.0
        {
            return Err(format!("MEASUREMENT_CONTRACT_INVALID:{}", measurement.feature_id));
        }
    }

    let mut global_reasons = Vec::new();
    let mut global_disposition = None;
    if case.expected_subject_id != case.evidence_subject_id {
        global_reasons.push("PHYSICAL_UNIT_IDENTITY_MISMATCH".to_owned());
        global_disposition = Some(Disposition::Reject);
    }
    if case.design_revision != case.evidence_design_revision {
        global_reasons.push("DESIGN_REVISION_MISMATCH".to_owned());
        if global_disposition != Some(Disposition::Reject) {
            global_disposition = Some(Disposition::IndeterminateHumanReview);
        }
    }
    if let Err(reason) = validate_instrument(&case.instrument, case.source_time_unix_ms) {
        global_reasons.push(reason);
        if global_disposition != Some(Disposition::Reject) {
            global_disposition = Some(Disposition::IndeterminateHumanReview);
        }
    }
    if let Err(reason) = validate_registration(&case.registration) {
        global_reasons.push(reason);
        if global_disposition != Some(Disposition::Reject) {
            global_disposition = Some(Disposition::IndeterminateHumanReview);
        }
    }
    if duplicate_measurements {
        global_reasons.push("DUPLICATE_MEASUREMENT_IDENTITY".to_owned());
        if global_disposition != Some(Disposition::Reject) {
            global_disposition = Some(Disposition::IndeterminateHumanReview);
        }
    }

    if let Some(disposition) = global_disposition {
        return Ok(CaseResult {
            case_id: case.case_id.clone(),
            disposition,
            reason_codes: global_reasons,
            features: case.required_features.iter().map(unresolved_feature).collect(),
            authority_granted: false,
        });
    }

    let mut results = Vec::with_capacity(case.required_features.len());
    let mut has_reject = false;
    let mut has_unresolved = false;
    let mut reason_codes = Vec::new();

    for requirement in &case.required_features {
        let Some(measurement) = measurement_map.get(&requirement.feature_id) else {
            has_unresolved = true;
            reason_codes.push(format!("REQUIRED_FEATURE_NOT_MEASURED:{}", requirement.feature_id));
            results.push(unresolved_feature(requirement));
            continue;
        };
        if !measurement.observed {
            has_unresolved = true;
            reason_codes.push(format!("REQUIRED_FEATURE_UNOBSERVED:{}", requirement.feature_id));
            results.push(unresolved_feature(requirement));
            continue;
        }

        // Uncertainties are added as worst-case bounds. No statistical
        // independence or cancellation is assumed in this v0.1 contract.
        let uncertainty =
            measurement.measurement_uncertainty_m + case.registration.registration_uncertainty_m;
        let design_position = apply_registration(
            &case.registration,
            measurement.observed_position_source_m,
        );
        let residual = distance(design_position, requirement.nominal_position_m);
        let lower = (residual - uncertainty).max(0.0);
        let upper = residual + uncertainty;

        let disposition = if upper <= requirement.tolerance_m {
            FeatureDisposition::WithinTolerance
        } else if lower > requirement.tolerance_m {
            has_reject = true;
            reason_codes.push(format!("FEATURE_OUTSIDE_TOLERANCE:{}", requirement.feature_id));
            FeatureDisposition::OutsideTolerance
        } else {
            has_unresolved = true;
            reason_codes.push(format!("FEATURE_GUARD_BAND_OVERLAP:{}", requirement.feature_id));
            FeatureDisposition::Unresolved
        };

        results.push(FeatureResult {
            feature_id: requirement.feature_id.clone(),
            critical: requirement.critical,
            disposition,
            residual_m: Some(residual),
            uncertainty_radius_m: Some(uncertainty),
            lower_error_bound_m: Some(lower),
            upper_error_bound_m: Some(upper),
            tolerance_m: requirement.tolerance_m,
        });
    }

    let disposition = if has_reject {
        Disposition::Reject
    } else if has_unresolved {
        Disposition::IndeterminateHumanReview
    } else {
        Disposition::Accept
    };
    reason_codes.sort();
    Ok(CaseResult {
        case_id: case.case_id.clone(),
        disposition,
        reason_codes,
        features: results,
        authority_granted: false,
    })
}

pub fn audit_corpus(raw: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("SOURCE_COMMIT_MUST_BE_40_LOWERCASE_HEX".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(raw).map_err(|error| format!("CORPUS_JSON_INVALID:{error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != "ORBITAL-TRUSS-ASBUILT-GEOMETRY-001"
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
    let oracle_matches = results
        .iter()
        .zip(
            corpus.cases.iter().map(|case| (&case.case_id, case.expected_disposition)),
        )
        .filter(|(result, (case_id, expected))| {
            result.case_id == **case_id && result.disposition == *expected
        })
        .count();
    let oracle_mismatches = results.len() - oracle_matches;

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: corpus.benchmark_id,
        source_commit: source_commit.to_owned(),
        input_sha256: sha256_hex(raw),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        structural_acceptance_authorized: false,
        flight_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        case_count: results.len(),
        oracle_matches,
        oracle_mismatches,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!(
        "../../../docs/research/orbital-truss/fixtures/asbuilt-geometry-v0_1.json"
    );
    const SOURCE: &str = "0123456789abcdef0123456789abcdef01234567";

    fn receipt() -> Receipt {
        audit_corpus(FIXTURE, SOURCE).expect("frozen as-built geometry corpus must audit")
    }

    #[test]
    fn frozen_corpus_matches_every_declared_disposition() {
        let report = receipt();
        assert_eq!(report.case_count, 13);
        assert_eq!(report.oracle_matches, 13);
        assert_eq!(report.oracle_mismatches, 0);
        assert!(!report.authority_granted);
        assert!(!report.structural_acceptance_authorized);
        assert!(!report.flight_authorized);
    }

    #[test]
    fn common_mode_translation_is_not_removed_by_part_best_fit() {
        let result = receipt();
        let case = result.cases.iter().find(|item| item.case_id == "ATG-002-COMMON-MODE-OFFSET").unwrap();
        assert_eq!(case.disposition, Disposition::Reject);
        assert!(case.reason_codes.iter().any(|code| code.starts_with("FEATURE_OUTSIDE_TOLERANCE")));
    }

    #[test]
    fn guard_band_overlap_is_indeterminate_not_pass() {
        let result = receipt();
        let case = result.cases.iter().find(|item| item.case_id == "ATG-010-GUARD-BAND").unwrap();
        assert_eq!(case.disposition, Disposition::IndeterminateHumanReview);
    }

    #[test]
    fn invalid_registration_and_expired_calibration_cannot_pass() {
        let result = receipt();
        for id in ["ATG-004-REFLECTION", "ATG-005-SCALE", "ATG-009-EXPIRED-CALIBRATION", "ATG-011-ARTICLE-FIT"] {
            let case = result.cases.iter().find(|item| item.case_id == id).unwrap();
            assert_eq!(case.disposition, Disposition::IndeterminateHumanReview, "{id}");
        }
    }

    #[test]
    fn wrong_physical_serial_is_rejected_and_stale_revision_is_indeterminate() {
        let result = receipt();
        let wrong_serial = result.cases.iter().find(|item| item.case_id == "ATG-007-WRONG-SERIAL").unwrap();
        let stale_revision = result.cases.iter().find(|item| item.case_id == "ATG-008-STALE-REVISION").unwrap();
        assert_eq!(wrong_serial.disposition, Disposition::Reject);
        assert_eq!(stale_revision.disposition, Disposition::IndeterminateHumanReview);
    }

    #[test]
    fn equivalent_rigid_frame_change_preserves_acceptance() {
        let result = receipt();
        let nominal = result.cases.iter().find(|item| item.case_id == "ATG-001-NOMINAL").unwrap();
        let transformed = result.cases.iter().find(|item| item.case_id == "ATG-003-RIGID-FRAME-CHANGE").unwrap();
        assert_eq!(nominal.disposition, Disposition::Accept);
        assert_eq!(transformed.disposition, Disposition::Accept);
        for (left, right) in nominal.features.iter().zip(&transformed.features) {
            assert!((left.residual_m.unwrap() - right.residual_m.unwrap()).abs() <= EPS);
        }
    }

    #[test]
    fn duplicate_and_missing_feature_measurements_never_pass() {
        let result = receipt();
        for id in ["ATG-006-MISSING-CRITICAL", "ATG-012-DUPLICATE-MEASUREMENT"] {
            let case = result.cases.iter().find(|item| item.case_id == id).unwrap();
            assert_eq!(case.disposition, Disposition::IndeterminateHumanReview, "{id}");
        }
    }

    #[test]
    fn a_certain_tolerance_violation_rejects() {
        let result = receipt();
        let case = result.cases.iter().find(|item| item.case_id == "ATG-013-CLEAR-OVERSIZE").unwrap();
        assert_eq!(case.disposition, Disposition::Reject);
    }

    #[test]
    fn uncertainty_is_composed_conservatively_without_rss() {
        let mut corpus: Corpus = serde_json::from_slice(FIXTURE).unwrap();
        let case = corpus.cases.iter_mut().find(|item| item.case_id == "ATG-010-GUARD-BAND").unwrap();
        let result = evaluate_case(case).unwrap();
        let feature = result.features.first().unwrap();
        let measurement = case.measurements.first().unwrap();
        let expected = measurement.measurement_uncertainty_m + case.registration.registration_uncertainty_m;
        assert!((feature.uncertainty_radius_m.unwrap() - expected).abs() <= EPS);
    }

    #[test]
    fn malformed_transform_does_not_silently_become_a_valid_rotation() {
        let mut corpus: Corpus = serde_json::from_slice(FIXTURE).unwrap();
        let case = corpus.cases.iter_mut().find(|item| item.case_id == "ATG-004-REFLECTION").unwrap();
        assert!(validate_registration(&case.registration).is_err());
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut corpus: Corpus = serde_json::from_slice(FIXTURE).unwrap();
        corpus.cases[1].case_id = corpus.cases[0].case_id.clone();
        let bytes = serde_json::to_vec(&corpus).unwrap();
        assert!(audit_corpus(&bytes, SOURCE).unwrap_err().contains("DUPLICATE_CASE_ID"));
    }

    #[test]
    fn source_commit_identity_is_strict() {
        assert!(audit_corpus(FIXTURE, "not-a-commit").is_err());
    }
}
