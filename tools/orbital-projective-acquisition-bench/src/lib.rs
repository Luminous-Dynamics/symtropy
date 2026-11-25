use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-projective-acquisition.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-projective-acquisition.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-PROJECTIVE-ACQUISITION-001";
pub const CLAIM_CEILING: &str = "synthetic_outward_rounded_projective_motion_bounds_only";
const MAX_INPUT: u64 = 1_000_000_000_000;
const MICRORADIANS_PER_RADIAN: u128 = 1_000_000;
const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    WithinDeclaredBudget,
    BudgetExceeded,
    Blocked,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MotionModel {
    pub relative_translation_speed_um_s: u64,
    pub relative_angular_speed_urad_s: u64,
    pub max_point_radius_um: u64,
    pub min_depth_um: u64,
    pub max_abs_x_um: u64,
    pub max_abs_y_um: u64,
    pub focal_x_micro_px: u64,
    pub focal_y_micro_px: u64,
    pub row_exposure_duration_ns: u64,
    pub first_to_last_row_midpoint_span_ns: u64,
    pub exposure_budget_x_micro_px: u64,
    pub exposure_budget_y_micro_px: u64,
    pub row_time_budget_x_micro_px: u64,
    pub row_time_budget_y_micro_px: u64,
    pub combined_frame_budget_x_micro_px: u64,
    pub combined_frame_budget_y_micro_px: u64,
    pub relative_motion_state: GateState,
    pub support_envelope_state: GateState,
    pub timing_state: GateState,
    pub motion_receipt_sha256: String,
    pub support_receipt_sha256: String,
    pub timing_receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub model: MotionModel,
    pub expected_status: Status,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectionBounds {
    pub rotational_speed_um_s_upper: u64,
    pub total_relative_point_speed_um_s_upper: u64,
    pub exposure_point_displacement_um_upper: u64,
    pub row_time_point_displacement_um_upper: u64,
    pub exposure_x_micro_px_upper: u64,
    pub exposure_y_micro_px_upper: u64,
    pub row_time_x_micro_px_upper: u64,
    pub row_time_y_micro_px_upper: u64,
    pub combined_frame_x_micro_px_upper: u64,
    pub combined_frame_y_micro_px_upper: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: Status,
    pub bounds: Option<ProjectionBounds>,
    pub reason_codes: Vec<String>,
    pub motion_receipt_sha256: String,
    pub support_receipt_sha256: String,
    pub timing_receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, String> {
    if denominator == 0 {
        return Err("DIVISION_BY_ZERO".to_owned());
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())
    }
}

fn to_u64(value: u128) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "ARITHMETIC_OVERFLOW".to_owned())
}

fn validate_case(case: &Case) -> Result<(), Vec<String>> {
    let mut reasons = BTreeSet::new();
    if !valid_text(&case.case_id) {
        reasons.insert("CASE_ID_EMPTY".to_owned());
    }

    let model = &case.model;
    let numeric = [
        model.relative_translation_speed_um_s,
        model.relative_angular_speed_urad_s,
        model.max_point_radius_um,
        model.min_depth_um,
        model.max_abs_x_um,
        model.max_abs_y_um,
        model.focal_x_micro_px,
        model.focal_y_micro_px,
        model.row_exposure_duration_ns,
        model.first_to_last_row_midpoint_span_ns,
        model.exposure_budget_x_micro_px,
        model.exposure_budget_y_micro_px,
        model.row_time_budget_x_micro_px,
        model.row_time_budget_y_micro_px,
        model.combined_frame_budget_x_micro_px,
        model.combined_frame_budget_y_micro_px,
    ];
    if numeric.iter().any(|value| *value > MAX_INPUT) {
        reasons.insert("INPUT_EXCEEDS_DECLARED_LIMIT".to_owned());
    }
    if model.min_depth_um == 0 {
        reasons.insert("NON_POSITIVE_MINIMUM_DEPTH".to_owned());
    }
    if model.focal_x_micro_px == 0 || model.focal_y_micro_px == 0 {
        reasons.insert("NON_POSITIVE_FOCAL_LENGTH".to_owned());
    }
    if model.max_point_radius_um < model.max_abs_x_um
        || model.max_point_radius_um < model.max_abs_y_um
        || model.max_point_radius_um < model.min_depth_um
    {
        reasons.insert("RADIUS_ENVELOPE_INCONSISTENT".to_owned());
    }
    for (name, digest) in [
        ("MOTION_RECEIPT", &model.motion_receipt_sha256),
        ("SUPPORT_RECEIPT", &model.support_receipt_sha256),
        ("TIMING_RECEIPT", &model.timing_receipt_sha256),
    ] {
        if !valid_sha256(digest) {
            reasons.insert(format!("{name}_DIGEST_INVALID"));
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons.into_iter().collect())
    }
}

fn projected_axis_upper(
    focal_micro_px: u64,
    displacement_um: u64,
    coordinate_extent_um: u64,
    min_depth_um: u64,
) -> Result<u64, String> {
    let z = u128::from(min_depth_um);
    let extent_plus_depth = z
        .checked_add(u128::from(coordinate_extent_um))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let numerator = u128::from(focal_micro_px)
        .checked_mul(u128::from(displacement_um))
        .and_then(|value| value.checked_mul(extent_plus_depth))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let denominator = z
        .checked_mul(z)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, denominator)?)
}

fn calculate_bounds(model: &MotionModel) -> Result<ProjectionBounds, String> {
    let rotational_speed = ceil_div(
        u128::from(model.relative_angular_speed_urad_s)
            .checked_mul(u128::from(model.max_point_radius_um))
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?,
        MICRORADIANS_PER_RADIAN,
    )?;
    let total_speed = u128::from(model.relative_translation_speed_um_s)
        .checked_add(rotational_speed)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;

    let exposure_delta = ceil_div(
        total_speed
            .checked_mul(u128::from(model.row_exposure_duration_ns))
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?,
        NANOSECONDS_PER_SECOND,
    )?;
    let row_delta = ceil_div(
        total_speed
            .checked_mul(u128::from(model.first_to_last_row_midpoint_span_ns))
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?,
        NANOSECONDS_PER_SECOND,
    )?;

    let exposure_delta = to_u64(exposure_delta)?;
    let row_delta = to_u64(row_delta)?;
    let exposure_x = projected_axis_upper(
        model.focal_x_micro_px,
        exposure_delta,
        model.max_abs_x_um,
        model.min_depth_um,
    )?;
    let exposure_y = projected_axis_upper(
        model.focal_y_micro_px,
        exposure_delta,
        model.max_abs_y_um,
        model.min_depth_um,
    )?;
    let row_x = projected_axis_upper(
        model.focal_x_micro_px,
        row_delta,
        model.max_abs_x_um,
        model.min_depth_um,
    )?;
    let row_y = projected_axis_upper(
        model.focal_y_micro_px,
        row_delta,
        model.max_abs_y_um,
        model.min_depth_um,
    )?;

    Ok(ProjectionBounds {
        rotational_speed_um_s_upper: to_u64(rotational_speed)?,
        total_relative_point_speed_um_s_upper: to_u64(total_speed)?,
        exposure_point_displacement_um_upper: exposure_delta,
        row_time_point_displacement_um_upper: row_delta,
        exposure_x_micro_px_upper: exposure_x,
        exposure_y_micro_px_upper: exposure_y,
        row_time_x_micro_px_upper: row_x,
        row_time_y_micro_px_upper: row_y,
        combined_frame_x_micro_px_upper: exposure_x
            .checked_add(row_x)
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?,
        combined_frame_y_micro_px_upper: exposure_y
            .checked_add(row_y)
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?,
    })
}

fn evaluate_case(case: &Case) -> CaseResult {
    let model = &case.model;
    let result = |status, bounds, reason_codes| CaseResult {
        case_id: case.case_id.clone(),
        status,
        bounds,
        reason_codes,
        motion_receipt_sha256: model.motion_receipt_sha256.clone(),
        support_receipt_sha256: model.support_receipt_sha256.clone(),
        timing_receipt_sha256: model.timing_receipt_sha256.clone(),
    };

    if let Err(reasons) = validate_case(case) {
        return result(Status::InvalidInput, None, reasons);
    }

    let gates = [
        ("RELATIVE_MOTION", model.relative_motion_state),
        ("SUPPORT_ENVELOPE", model.support_envelope_state),
        ("TIMING", model.timing_state),
    ];
    let mut failed = BTreeSet::new();
    let mut unknown = BTreeSet::new();
    for (name, state) in gates {
        match state {
            GateState::Pass => {}
            GateState::Fail => {
                failed.insert(format!("GATE_FAIL:{name}"));
            }
            GateState::Unknown => {
                unknown.insert(format!("GATE_UNKNOWN:{name}"));
            }
        }
    }
    if !failed.is_empty() {
        return result(Status::Blocked, None, failed.into_iter().collect());
    }
    if !unknown.is_empty() {
        return result(Status::Indeterminate, None, unknown.into_iter().collect());
    }

    let bounds = match calculate_bounds(model) {
        Ok(bounds) => bounds,
        Err(reason) => return result(Status::InvalidInput, None, vec![reason]),
    };

    let mut reasons = BTreeSet::new();
    if bounds.exposure_x_micro_px_upper > model.exposure_budget_x_micro_px {
        reasons.insert("EXPOSURE_X_BUDGET_EXCEEDED".to_owned());
    }
    if bounds.exposure_y_micro_px_upper > model.exposure_budget_y_micro_px {
        reasons.insert("EXPOSURE_Y_BUDGET_EXCEEDED".to_owned());
    }
    if bounds.row_time_x_micro_px_upper > model.row_time_budget_x_micro_px {
        reasons.insert("ROW_TIME_X_BUDGET_EXCEEDED".to_owned());
    }
    if bounds.row_time_y_micro_px_upper > model.row_time_budget_y_micro_px {
        reasons.insert("ROW_TIME_Y_BUDGET_EXCEEDED".to_owned());
    }
    if bounds.combined_frame_x_micro_px_upper > model.combined_frame_budget_x_micro_px {
        reasons.insert("COMBINED_FRAME_X_BUDGET_EXCEEDED".to_owned());
    }
    if bounds.combined_frame_y_micro_px_upper > model.combined_frame_budget_y_micro_px {
        reasons.insert("COMBINED_FRAME_Y_BUDGET_EXCEEDED".to_owned());
    }

    let status = if reasons.is_empty() {
        Status::WithinDeclaredBudget
    } else {
        Status::BudgetExceeded
    };
    result(status, Some(bounds), reasons.into_iter().collect())
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_JSON_INVALID: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 128
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    let mut expected_outcome_matches = 0_usize;
    for case in &corpus.cases {
        if !case_ids.insert(&case.case_id) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        let result = evaluate_case(case);
        if result.status == case.expected_status {
            expected_outcome_matches += 1;
        }
        cases.push(result);
    }

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_outcome_matches,
        expected_outcome_mismatches: cases.len() - expected_outcome_matches,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_bytes() -> &'static [u8] {
        include_bytes!(
            "../../../docs/research/orbital-truss/fixtures/projective-acquisition-v0_1.json"
        )
    }

    fn corpus() -> Corpus {
        serde_json::from_slice(corpus_bytes()).expect("fixture parses")
    }

    #[test]
    fn frozen_corpus_matches_all_expected_statuses() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus audits");
        assert_eq!(receipt.case_count, 10);
        assert_eq!(receipt.expected_outcome_matches, 10);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
    }

    #[test]
    fn pure_rotation_produces_a_nonzero_bound() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "PM-004-PURE-ROTATION")
            .unwrap();
        let result = evaluate_case(case);
        let bounds = result.bounds.unwrap();
        assert!(bounds.rotational_speed_um_s_upper > 0);
        assert!(bounds.exposure_x_micro_px_upper > 0);
    }

    #[test]
    fn nearer_depth_is_never_less_conservative() {
        let mut input = corpus();
        let case = input
            .cases
            .iter_mut()
            .find(|case| case.case_id == "PM-005-NEAR-DEPTH")
            .unwrap();
        let near = evaluate_case(case).bounds.unwrap();
        case.model.min_depth_um *= 2;
        let far = evaluate_case(case).bounds.unwrap();
        assert!(near.exposure_x_micro_px_upper >= far.exposure_x_micro_px_upper);
        assert!(near.exposure_y_micro_px_upper >= far.exposure_y_micro_px_upper);
    }

    #[test]
    fn rotational_bound_increases_with_support_radius() {
        let mut input = corpus();
        let case = input
            .cases
            .iter_mut()
            .find(|case| case.case_id == "PM-004-PURE-ROTATION")
            .unwrap();
        let original = evaluate_case(case).bounds.unwrap();
        case.model.max_point_radius_um *= 2;
        let expanded = evaluate_case(case).bounds.unwrap();
        assert!(expanded.rotational_speed_um_s_upper >= original.rotational_speed_um_s_upper);
        assert!(expanded.exposure_x_micro_px_upper >= original.exposure_x_micro_px_upper);
    }

    #[test]
    fn row_time_and_exposure_are_separately_bounded_and_combined() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "PM-007-COMBINED-EXCEEDS")
            .unwrap();
        let result = evaluate_case(case);
        let bounds = result.bounds.unwrap();
        assert!(bounds.exposure_x_micro_px_upper <= case.model.exposure_budget_x_micro_px);
        assert!(bounds.row_time_x_micro_px_upper <= case.model.row_time_budget_x_micro_px);
        assert!(
            bounds.combined_frame_x_micro_px_upper > case.model.combined_frame_budget_x_micro_px
        );
        assert_eq!(result.status, Status::BudgetExceeded);
    }

    #[test]
    fn unknown_gate_is_indeterminate_and_failed_gate_is_blocked() {
        let input = corpus();
        let unknown = input
            .cases
            .iter()
            .find(|case| case.case_id == "PM-008-UNKNOWN-MOTION")
            .unwrap();
        assert_eq!(evaluate_case(unknown).status, Status::Indeterminate);
        let failed = input
            .cases
            .iter()
            .find(|case| case.case_id == "PM-009-FAILED-SUPPORT")
            .unwrap();
        assert_eq!(evaluate_case(failed).status, Status::Blocked);
    }

    #[test]
    fn invalid_depth_is_rejected_before_projection() {
        let input = corpus();
        let invalid = input
            .cases
            .iter()
            .find(|case| case.case_id == "PM-010-ZERO-DEPTH")
            .unwrap();
        let result = evaluate_case(invalid);
        assert_eq!(result.status, Status::InvalidInput);
        assert!(
            result
                .reason_codes
                .contains(&"NON_POSITIVE_MINIMUM_DEPTH".to_owned())
        );
        assert!(result.bounds.is_none());
    }
}
