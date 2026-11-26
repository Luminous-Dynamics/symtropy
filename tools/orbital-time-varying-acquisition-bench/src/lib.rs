use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-time-varying-acquisition.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-time-varying-acquisition.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-TIME-VARYING-ACQUISITION-001";
pub const CLAIM_CEILING: &str = "synthetic_acceleration_jerk_bounded_projective_path_envelopes_only";

const MAX_INPUT: u64 = 1_000_000_000_000;
const MAX_TIME_NS: u64 = 1_000_000_000;
const NS_PER_SECOND: u128 = 1_000_000_000;
const NS2_PER_SECOND2: u128 = 1_000_000_000_000_000_000;
const NS3_PER_SECOND3: u128 = 1_000_000_000_000_000_000_000_000_000;

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
    pub reference_point_speed_um_s: u64,
    pub reference_point_acceleration_um_s2: u64,
    pub point_jerk_upper_um_s3: u64,
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
    pub kinematic_envelope_state: GateState,
    pub support_envelope_state: GateState,
    pub timing_state: GateState,
    pub calibration_state: GateState,
    pub motion_receipt_sha256: String,
    pub support_receipt_sha256: String,
    pub timing_receipt_sha256: String,
    pub calibration_receipt_sha256: String,
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
pub struct DisplacementTerms {
    pub speed_um: u64,
    pub acceleration_um: u64,
    pub jerk_um: u64,
    pub total_um: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectionBounds {
    pub row_time: DisplacementTerms,
    pub latest_row_exposure: DisplacementTerms,
    pub combined_frame: DisplacementTerms,
    pub constant_max_speed_frame: DisplacementTerms,
    pub constant_speed_excess_um_upper: u64,
    pub constant_speed_frame_x_micro_px_upper: u64,
    pub constant_speed_frame_y_micro_px_upper: u64,
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
    pub calibration_receipt_sha256: String,
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

fn mul3(a: u128, b: u128, c: u128) -> Result<u128, String> {
    a.checked_mul(b)
        .and_then(|value| value.checked_mul(c))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())
}

fn speed_term(speed_um_s: u64, duration_ns: u64) -> Result<u64, String> {
    to_u64(ceil_div(
        u128::from(speed_um_s)
            .checked_mul(u128::from(duration_ns))
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?,
        NS_PER_SECOND,
    )?)
}

fn acceleration_row_term(acceleration_um_s2: u64, duration_ns: u64) -> Result<u64, String> {
    let t = u128::from(duration_ns);
    let numerator = mul3(u128::from(acceleration_um_s2), t, t)?;
    to_u64(ceil_div(numerator, 2 * NS2_PER_SECOND2)?)
}

fn jerk_row_term(jerk_um_s3: u64, duration_ns: u64) -> Result<u64, String> {
    let t = u128::from(duration_ns);
    let t2 = t.checked_mul(t).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let t3 = t2.checked_mul(t).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let numerator = u128::from(jerk_um_s3)
        .checked_mul(t3)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, 6 * NS3_PER_SECOND3)?)
}

fn exposure_acceleration_term(
    acceleration_um_s2: u64,
    row_span_ns: u64,
    exposure_ns: u64,
) -> Result<u64, String> {
    let t = u128::from(row_span_ns);
    let e = u128::from(exposure_ns);
    let four_i1 = if 2 * t >= e {
        mul3(4, t, e)?
    } else {
        let four_t2 = t
            .checked_mul(t)
            .and_then(|value| value.checked_mul(4))
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
        four_t2
            .checked_add(e.checked_mul(e).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?)
            .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?
    };
    let numerator = u128::from(acceleration_um_s2)
        .checked_mul(four_i1)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, 4 * NS2_PER_SECOND2)?)
}

fn exposure_jerk_term(
    jerk_um_s3: u64,
    row_span_ns: u64,
    exposure_ns: u64,
) -> Result<u64, String> {
    let t = u128::from(row_span_ns);
    let e = u128::from(exposure_ns);
    let t2 = t.checked_mul(t).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let e2 = e.checked_mul(e).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let twelve_e_t2 = mul3(12, e, t2)?;
    let e3 = e2.checked_mul(e).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let twelve_i2 = twelve_e_t2
        .checked_add(e3)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let numerator = u128::from(jerk_um_s3)
        .checked_mul(twelve_i2)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, 24 * NS3_PER_SECOND3)?)
}

fn sum_displacement_terms(speed: u64, acceleration: u64, jerk: u64) -> Result<DisplacementTerms, String> {
    let total = speed
        .checked_add(acceleration)
        .and_then(|value| value.checked_add(jerk))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    Ok(DisplacementTerms {
        speed_um: speed,
        acceleration_um: acceleration,
        jerk_um: jerk,
        total_um: total,
    })
}

fn acceleration_frame_term(
    acceleration_um_s2: u64,
    row_span_ns: u64,
    exposure_ns: u64,
) -> Result<u64, String> {
    // Integral of |t| over [-E/2, T+E/2] is
    // (E^2 + (2T+E)^2) / 8.
    let t = u128::from(row_span_ns);
    let e = u128::from(exposure_ns);
    let twice_t_plus_e = t
        .checked_mul(2)
        .and_then(|value| value.checked_add(e))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let e2 = e.checked_mul(e).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let q2 = twice_t_plus_e
        .checked_mul(twice_t_plus_e)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let eight_i1 = e2
        .checked_add(q2)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let numerator = u128::from(acceleration_um_s2)
        .checked_mul(eight_i1)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, 8 * NS2_PER_SECOND2)?)
}

fn jerk_frame_term(
    jerk_um_s3: u64,
    row_span_ns: u64,
    exposure_ns: u64,
) -> Result<u64, String> {
    // Integral of t^2 over [-E/2, T+E/2] is
    // ((2T+E)^3 + E^3) / 24. The speed envelope contains J*t^2/2.
    let t = u128::from(row_span_ns);
    let e = u128::from(exposure_ns);
    let twice_t_plus_e = t
        .checked_mul(2)
        .and_then(|value| value.checked_add(e))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let q2 = twice_t_plus_e
        .checked_mul(twice_t_plus_e)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let q3 = q2
        .checked_mul(twice_t_plus_e)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let e2 = e.checked_mul(e).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let e3 = e2.checked_mul(e).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let twenty_four_i2 = q3
        .checked_add(e3)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let numerator = u128::from(jerk_um_s3)
        .checked_mul(twenty_four_i2)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, 48 * NS3_PER_SECOND3)?)
}

fn projected_axis_upper(
    focal_micro_px: u64,
    displacement_um: u64,
    coordinate_extent_um: u64,
    min_depth_um: u64,
) -> Result<u64, String> {
    let z = u128::from(min_depth_um);
    let z_plus_extent = z
        .checked_add(u128::from(coordinate_extent_um))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let numerator = mul3(
        u128::from(focal_micro_px),
        u128::from(displacement_um),
        z_plus_extent,
    )?;
    let denominator = z
        .checked_mul(z)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    to_u64(ceil_div(numerator, denominator)?)
}

fn validate_case(case: &Case) -> Result<(), Vec<String>> {
    let mut reasons = BTreeSet::new();
    let model = &case.model;
    if !valid_text(&case.case_id) {
        reasons.insert("CASE_ID_EMPTY".to_owned());
    }

    let numeric_inputs = [
        model.reference_point_speed_um_s,
        model.reference_point_acceleration_um_s2,
        model.point_jerk_upper_um_s3,
        model.max_point_radius_um,
        model.min_depth_um,
        model.max_abs_x_um,
        model.max_abs_y_um,
        model.focal_x_micro_px,
        model.focal_y_micro_px,
        model.exposure_budget_x_micro_px,
        model.exposure_budget_y_micro_px,
        model.row_time_budget_x_micro_px,
        model.row_time_budget_y_micro_px,
        model.combined_frame_budget_x_micro_px,
        model.combined_frame_budget_y_micro_px,
    ];
    if numeric_inputs.iter().any(|value| *value > MAX_INPUT)
        || model.row_exposure_duration_ns > MAX_TIME_NS
        || model.first_to_last_row_midpoint_span_ns > MAX_TIME_NS
        || model.point_jerk_upper_um_s3 > 1_000_000_000
    {
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
        ("CALIBRATION_RECEIPT", &model.calibration_receipt_sha256),
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

fn constant_max_speed_frame_terms(
    model: &MotionModel,
    full_frame_duration_ns: u64,
) -> Result<DisplacementTerms, String> {
    // The maximum absolute time in [-E/2, T+E/2] is T+E/2.
    // Apply that single maximum speed to the entire frame as a deliberately
    // conservative comparator, not as the decision-bound calculation.
    let twice_max_time_ns = u128::from(model.first_to_last_row_midpoint_span_ns)
        .checked_mul(2)
        .and_then(|value| value.checked_add(u128::from(model.row_exposure_duration_ns)))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let duration_ns = u128::from(full_frame_duration_ns);

    let speed = speed_term(model.reference_point_speed_um_s, full_frame_duration_ns)?;
    let acceleration_numerator = mul3(
        u128::from(model.reference_point_acceleration_um_s2),
        twice_max_time_ns,
        duration_ns,
    )?;
    let acceleration = to_u64(ceil_div(
        acceleration_numerator,
        2 * NS2_PER_SECOND2,
    )?)?;

    let twice_max_time_squared = twice_max_time_ns
        .checked_mul(twice_max_time_ns)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let jerk_numerator = u128::from(model.point_jerk_upper_um_s3)
        .checked_mul(twice_max_time_squared)
        .and_then(|value| value.checked_mul(duration_ns))
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let jerk = to_u64(ceil_div(jerk_numerator, 8 * NS3_PER_SECOND3)?)?;

    sum_displacement_terms(speed, acceleration, jerk)
}

fn calculate_bounds(model: &MotionModel) -> Result<ProjectionBounds, String> {
    let exposure_ns = model.row_exposure_duration_ns;
    let row_span_ns = model.first_to_last_row_midpoint_span_ns;

    // Row-time motion is integrated from the first-row midpoint (t = 0) to the last.
    let row = sum_displacement_terms(
        speed_term(model.reference_point_speed_um_s, row_span_ns)?,
        acceleration_row_term(model.reference_point_acceleration_um_s2, row_span_ns)?,
        jerk_row_term(model.point_jerk_upper_um_s3, row_span_ns)?,
    )?;

    // The latest row is the worst exposure window for the non-decreasing speed envelope.
    let exposure = sum_displacement_terms(
        speed_term(model.reference_point_speed_um_s, exposure_ns)?,
        exposure_acceleration_term(
            model.reference_point_acceleration_um_s2,
            row_span_ns,
            exposure_ns,
        )?,
        exposure_jerk_term(model.point_jerk_upper_um_s3, row_span_ns, exposure_ns)?,
    )?;

    // The combined frame is integrated directly over the complete acquisition
    // interval [-E/2, T+E/2]. Adding separately rounded row/exposure maxima can
    // under-bound this interval when T < E/2.
    let full_frame_duration_ns = row_span_ns
        .checked_add(exposure_ns)
        .ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())?;
    let combined_frame = sum_displacement_terms(
        speed_term(model.reference_point_speed_um_s, full_frame_duration_ns)?,
        acceleration_frame_term(
            model.reference_point_acceleration_um_s2,
            row_span_ns,
            exposure_ns,
        )?,
        jerk_frame_term(model.point_jerk_upper_um_s3, row_span_ns, exposure_ns)?,
    )?;
    let constant_max_speed_frame = constant_max_speed_frame_terms(model, full_frame_duration_ns)?;
    let constant_speed_excess_um_upper = constant_max_speed_frame
        .total_um
        .checked_sub(combined_frame.total_um)
        .ok_or_else(|| "CONSTANT_SPEED_COMPARATOR_UNDER_BOUND".to_owned())?;

    let constant_speed_frame_x = projected_axis_upper(
        model.focal_x_micro_px,
        constant_max_speed_frame.total_um,
        model.max_abs_x_um,
        model.min_depth_um,
    )?;
    let constant_speed_frame_y = projected_axis_upper(
        model.focal_y_micro_px,
        constant_max_speed_frame.total_um,
        model.max_abs_y_um,
        model.min_depth_um,
    )?;
    let exposure_x = projected_axis_upper(
        model.focal_x_micro_px,
        exposure.total_um,
        model.max_abs_x_um,
        model.min_depth_um,
    )?;
    let exposure_y = projected_axis_upper(
        model.focal_y_micro_px,
        exposure.total_um,
        model.max_abs_y_um,
        model.min_depth_um,
    )?;
    let row_x = projected_axis_upper(
        model.focal_x_micro_px,
        row.total_um,
        model.max_abs_x_um,
        model.min_depth_um,
    )?;
    let row_y = projected_axis_upper(
        model.focal_y_micro_px,
        row.total_um,
        model.max_abs_y_um,
        model.min_depth_um,
    )?;
    let combined_x = projected_axis_upper(
        model.focal_x_micro_px,
        combined_frame.total_um,
        model.max_abs_x_um,
        model.min_depth_um,
    )?;
    let combined_y = projected_axis_upper(
        model.focal_y_micro_px,
        combined_frame.total_um,
        model.max_abs_y_um,
        model.min_depth_um,
    )?;

    Ok(ProjectionBounds {
        row_time: row,
        latest_row_exposure: exposure,
        combined_frame,
        constant_max_speed_frame,
        constant_speed_excess_um_upper,
        constant_speed_frame_x_micro_px_upper: constant_speed_frame_x,
        constant_speed_frame_y_micro_px_upper: constant_speed_frame_y,
        exposure_x_micro_px_upper: exposure_x,
        exposure_y_micro_px_upper: exposure_y,
        row_time_x_micro_px_upper: row_x,
        row_time_y_micro_px_upper: row_y,
        combined_frame_x_micro_px_upper: combined_x,
        combined_frame_y_micro_px_upper: combined_y,
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
        calibration_receipt_sha256: model.calibration_receipt_sha256.clone(),
    };

    if let Err(reasons) = validate_case(case) {
        return result(Status::InvalidInput, None, reasons);
    }

    let gates = [
        ("KINEMATIC_ENVELOPE", model.kinematic_envelope_state),
        ("SUPPORT_ENVELOPE", model.support_envelope_state),
        ("TIMING", model.timing_state),
        ("CALIBRATION", model.calibration_state),
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
            "../../../docs/research/orbital-truss/fixtures/time-varying-acquisition-v0_1.json"
        )
    }

    fn corpus() -> Corpus {
        serde_json::from_slice(corpus_bytes()).expect("fixture parses")
    }

    #[test]
    fn frozen_corpus_matches_all_expected_statuses() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus audits");
        assert_eq!(receipt.case_count, 17);
        assert_eq!(receipt.expected_outcome_matches, 17);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
    }

    #[test]
    fn zero_acceleration_and_jerk_reduce_to_constant_speed() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-001-CONSTANT-SPEED")
            .unwrap();
        let result = evaluate_case(case);
        let bounds = result.bounds.unwrap();
        assert_eq!(bounds.row_time.speed_um, 1);
        assert_eq!(bounds.row_time.acceleration_um, 0);
        assert_eq!(bounds.row_time.jerk_um, 0);
        assert_eq!(bounds.latest_row_exposure.speed_um, 1);
        assert_eq!(bounds.latest_row_exposure.acceleration_um, 0);
        assert_eq!(bounds.latest_row_exposure.jerk_um, 0);
    }

    #[test]
    fn acceleration_and_jerk_growth_are_included() {
        let input = corpus();
        let accelerated = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-002-ACCELERATION-GROWTH")
            .unwrap();
        let jerked = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-003-JERK-ONLY")
            .unwrap();
        let accelerated_result = evaluate_case(accelerated).bounds.unwrap();
        let jerked_result = evaluate_case(jerked).bounds.unwrap();
        assert!(accelerated_result.row_time.acceleration_um > 0);
        assert!(accelerated_result.latest_row_exposure.acceleration_um > 0);
        assert!(jerked_result.row_time.jerk_um > 0);
        assert!(jerked_result.latest_row_exposure.jerk_um > 0);
    }

    #[test]
    fn displacement_bounds_are_monotone_in_speed_acceleration_jerk_and_time() {
        let input = corpus();
        let source = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-002-ACCELERATION-GROWTH")
            .unwrap();
        let base = calculate_bounds(&source.model).unwrap();

        let mut faster = source.model.clone();
        faster.reference_point_speed_um_s += 10_000;
        let faster_bounds = calculate_bounds(&faster).unwrap();
        assert!(faster_bounds.combined_frame.total_um >= base.combined_frame.total_um);

        let mut more_acceleration = source.model.clone();
        more_acceleration.reference_point_acceleration_um_s2 += 10_000;
        let acceleration_bounds = calculate_bounds(&more_acceleration).unwrap();
        assert!(acceleration_bounds.combined_frame.total_um >= base.combined_frame.total_um);

        let mut more_jerk = source.model.clone();
        more_jerk.point_jerk_upper_um_s3 += 10_000;
        let jerk_bounds = calculate_bounds(&more_jerk).unwrap();
        assert!(jerk_bounds.combined_frame.total_um >= base.combined_frame.total_um);

        let mut longer = source.model.clone();
        longer.row_exposure_duration_ns += 100_000;
        longer.first_to_last_row_midpoint_span_ns += 100_000;
        let longer_bounds = calculate_bounds(&longer).unwrap();
        assert!(longer_bounds.combined_frame.total_um >= base.combined_frame.total_um);
    }

    #[test]
    fn common_spatial_unit_scaling_preserves_projected_pixel_bounds() {
        let input = corpus();
        let source = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-002-ACCELERATION-GROWTH")
            .unwrap();
        let base = calculate_bounds(&source.model).unwrap();

        let mut scaled = source.model.clone();
        scaled.reference_point_speed_um_s *= 1_000;
        scaled.reference_point_acceleration_um_s2 *= 1_000;
        scaled.point_jerk_upper_um_s3 *= 1_000;
        scaled.max_point_radius_um *= 1_000;
        scaled.min_depth_um *= 1_000;
        scaled.max_abs_x_um *= 1_000;
        scaled.max_abs_y_um *= 1_000;

        let scaled_bounds = calculate_bounds(&scaled).unwrap();
        assert_eq!(
            base.exposure_x_micro_px_upper,
            scaled_bounds.exposure_x_micro_px_upper
        );
        assert_eq!(
            base.exposure_y_micro_px_upper,
            scaled_bounds.exposure_y_micro_px_upper
        );
        assert_eq!(
            base.row_time_x_micro_px_upper,
            scaled_bounds.row_time_x_micro_px_upper
        );
        assert_eq!(
            base.combined_frame_x_micro_px_upper,
            scaled_bounds.combined_frame_x_micro_px_upper
        );
    }

    #[test]
    fn latest_row_exposure_is_worst_under_the_monotone_envelope() {
        let input = corpus();
        let late = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-008-LATE-ROW")
            .unwrap();
        let model = &late.model;
        let earliest = sum_displacement_terms(
            speed_term(
                model.reference_point_speed_um_s,
                model.row_exposure_duration_ns,
            )
            .unwrap(),
            exposure_acceleration_term(
                model.reference_point_acceleration_um_s2,
                0,
                model.row_exposure_duration_ns,
            )
            .unwrap(),
            exposure_jerk_term(
                model.point_jerk_upper_um_s3,
                0,
                model.row_exposure_duration_ns,
            )
            .unwrap(),
        )
        .unwrap();
        let latest = calculate_bounds(model).unwrap().latest_row_exposure;
        assert!(latest.total_um > earliest.total_um);
    }

    #[test]
    fn combined_frame_can_fail_when_individual_channels_pass() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-004-COMBINED-ONLY")
            .unwrap();
        let result = evaluate_case(case);
        let bounds = result.bounds.unwrap();
        assert_eq!(bounds.exposure_x_micro_px_upper, 12_000);
        assert_eq!(bounds.row_time_x_micro_px_upper, 12_000);
        assert_eq!(bounds.combined_frame_x_micro_px_upper, 24_000);
        assert_eq!(result.status, Status::BudgetExceeded);
    }

    #[test]
    fn piecewise_absolute_time_integral_handles_both_branches() {
        let input = corpus();
        let below = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-006-T_LT_E_OVER_2")
            .unwrap();
        let boundary = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-007-T_EQ_E_OVER_2")
            .unwrap();
        let below_bounds = calculate_bounds(&below.model).unwrap();
        let boundary_bounds = calculate_bounds(&boundary.model).unwrap();
        assert!(below_bounds.latest_row_exposure.acceleration_um > 0);
        assert!(boundary_bounds.latest_row_exposure.acceleration_um > 0);
    }

    #[test]
    fn complete_frame_is_integrated_directly_not_summed_from_rounded_channels() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-016-FRAME-WINDOW-TRAP")
            .unwrap();
        let result = evaluate_case(case);
        let bounds = result.bounds.unwrap();

        assert_eq!(bounds.row_time.total_um, 1);
        assert_eq!(bounds.latest_row_exposure.total_um, 10);
        assert_eq!(bounds.combined_frame.total_um, 13);
        assert_eq!(bounds.exposure_x_micro_px_upper, 120_000);
        assert_eq!(bounds.row_time_x_micro_px_upper, 12_000);
        assert_eq!(bounds.combined_frame_x_micro_px_upper, 156_000);
        assert_eq!(result.status, Status::BudgetExceeded);
    }

    #[test]
    fn symmetric_first_row_exposure_includes_negative_time_half() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-015-NEGATIVE-TIME-HALF-EXPOSURE")
            .unwrap();
        let bounds = calculate_bounds(&case.model).unwrap();

        assert_eq!(bounds.row_time.total_um, 0);
        assert_eq!(bounds.latest_row_exposure.acceleration_um, 1);
        assert_eq!(bounds.combined_frame.acceleration_um, 1);
    }

    #[test]
    fn piecewise_exposure_integral_is_exact_on_both_sides_of_half_exposure() {
        let below = exposure_acceleration_term(1_000_000_000, 250_000, 1_000_000).unwrap();
        let at_boundary = exposure_acceleration_term(1_000_000_000, 500_000, 1_000_000).unwrap();
        let above = exposure_acceleration_term(1_000_000_000, 750_000, 1_000_000).unwrap();

        assert_eq!(below, 313);
        assert_eq!(at_boundary, 500);
        assert_eq!(above, 750);
    }

    #[test]
    fn time_varying_integral_is_never_larger_than_constant_max_speed_comparator() {
        let input = corpus();
        for case in &input.cases {
            if let Ok(bounds) = calculate_bounds(&case.model) {
                assert!(bounds.constant_max_speed_frame.total_um >= bounds.combined_frame.total_um);
                assert!(
                    bounds.constant_speed_frame_x_micro_px_upper
                        >= bounds.combined_frame_x_micro_px_upper
                );
                assert!(
                    bounds.constant_speed_frame_y_micro_px_upper
                        >= bounds.combined_frame_y_micro_px_upper
                );
            }
        }
    }

    #[test]
    fn acceleration_and_jerk_show_strict_constant_speed_excess() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-016-FRAME-WINDOW-TRAP")
            .unwrap();
        let bounds = calculate_bounds(&case.model).unwrap();
        assert!(bounds.constant_speed_excess_um_upper > 0);
        assert!(bounds.constant_max_speed_frame.total_um > bounds.combined_frame.total_um);
    }

    #[test]
    fn combined_frame_matches_constant_speed_over_full_interval() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-001-CONSTANT-SPEED")
            .unwrap();
        let bounds = calculate_bounds(&case.model).unwrap();
        let expected = speed_term(
            case.model.reference_point_speed_um_s,
            case.model
                .row_exposure_duration_ns
                .checked_add(case.model.first_to_last_row_midpoint_span_ns)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(bounds.combined_frame.speed_um, expected);
        assert_eq!(bounds.combined_frame.acceleration_um, 0);
        assert_eq!(bounds.combined_frame.jerk_um, 0);
        assert_eq!(
            bounds.constant_max_speed_frame.total_um,
            bounds.combined_frame.total_um
        );
        assert_eq!(bounds.constant_speed_excess_um_upper, 0);
    }

    #[test]
    fn global_shutter_has_zero_row_time_channel() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-005-GLOBAL-SHUTTER")
            .unwrap();
        let bounds = calculate_bounds(&case.model).unwrap();
        assert_eq!(bounds.row_time.total_um, 0);
        assert_eq!(bounds.row_time_x_micro_px_upper, 0);
        assert_eq!(bounds.row_time_y_micro_px_upper, 0);
    }

    #[test]
    fn gates_fail_closed_for_unknown_and_failed_upstream_evidence() {
        let input = corpus();
        let unknown = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-009-UNKNOWN-KINEMATICS")
            .unwrap();
        let failed = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-010-FAILED-SUPPORT")
            .unwrap();
        assert_eq!(evaluate_case(unknown).status, Status::Indeterminate);
        assert_eq!(evaluate_case(failed).status, Status::Blocked);
        let revoked = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-017-FAILED-CALIBRATION")
            .unwrap();
        assert_eq!(evaluate_case(revoked).status, Status::Blocked);
    }

    #[test]
    fn exact_inclusive_boundary_passes_and_one_unit_over_fails() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TV-014-EXACT-BOUNDARY")
            .unwrap();
        assert_eq!(evaluate_case(case).status, Status::WithinDeclaredBudget);
        let mut over = case.clone();
        over.model.exposure_budget_x_micro_px -= 1;
        assert_eq!(evaluate_case(&over).status, Status::BudgetExceeded);
    }

    #[test]
    fn invalid_and_overflow_inputs_never_return_bounds() {
        let input = corpus();
        for id in [
            "TV-011-ZERO-DEPTH",
            "TV-012-PROJECTION-OVERFLOW",
            "TV-013-BAD-MOTION-DIGEST",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            let result = evaluate_case(case);
            assert_eq!(result.status, Status::InvalidInput);
            assert!(result.bounds.is_none());
        }
    }
}
