use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-acquisition-integrity.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-acquisition-integrity.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-ACQUISITION-INTEGRITY-001";
pub const CLAIM_CEILING: &str =
    "synthetic_constant_relative_image_plane_motion_exposure_and_readout_bounds_only";
const MICRO_PIXELS_PER_PIXEL: u128 = 1_000_000;
const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CalibrationState {
    Valid,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClockMapState {
    Valid,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MotionModelState {
    Admitted,
    OutOfDomain,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShutterMode {
    GlobalShutter,
    RollingShutter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadoutDirection {
    TopToBottom,
    BottomToTop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimestampReference {
    CenterRowExposureMidpoint,
    FirstRowExposureStart,
    HostArrival,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AcquisitionStatus {
    AcquisitionValidWithinBudget,
    CalibrationNotValid,
    ProjectorPatternMismatch,
    MotionModelOutOfDomain,
    TimingUncertaintyExceeded,
    MotionBlurBudgetExceeded,
    RollingShutterBudgetExceeded,
    CombinedAcquisitionBudgetExceeded,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectorWindow {
    pub pattern_id: String,
    pub stable_start_ns: i64,
    pub stable_end_ns: i64,
    pub pattern_receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub camera_id: String,
    pub calibration_state: CalibrationState,
    pub calibration_receipt_sha256: String,
    pub clock_map_state: ClockMapState,
    pub clock_map_receipt_sha256: String,
    pub timestamp_reference: TimestampReference,
    pub frame_timestamp_ns: i64,
    pub timestamp_uncertainty_ns: u64,
    pub trigger_jitter_ns: u64,
    pub motion_model_id: String,
    pub motion_model_state: MotionModelState,
    pub relative_velocity_x_um_per_s: i64,
    pub relative_velocity_y_um_per_s: i64,
    pub gsd_um_per_pixel: u64,
    pub exposure_duration_ns: u64,
    pub shutter_mode: ShutterMode,
    pub image_height_px: u32,
    pub readout_duration_ns: u64,
    pub readout_uncertainty_ns: u64,
    pub readout_direction: ReadoutDirection,
    pub motion_budget_micro_px: u64,
    pub requires_projector: bool,
    pub projector_window: Option<ProjectorWindow>,
    pub expected_status: AcquisitionStatus,
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
pub struct CaseResult {
    pub case_id: String,
    pub camera_id: String,
    pub calibration_state: CalibrationState,
    pub calibration_receipt_sha256: String,
    pub clock_map_state: ClockMapState,
    pub clock_map_receipt_sha256: String,
    pub timestamp_reference: TimestampReference,
    pub shutter_mode: ShutterMode,
    pub readout_direction: ReadoutDirection,
    pub exposure_smear_bound_micro_px: u64,
    pub rolling_shutter_bound_micro_px: u64,
    pub timing_association_bound_micro_px: u64,
    pub total_motion_bound_micro_px: u64,
    pub motion_budget_micro_px: u64,
    pub maximum_row_offset_ns: u64,
    pub projector_frame_interval_ns: Option<[i128; 2]>,
    pub status: AcquisitionStatus,
    pub reason_codes: Vec<String>,
    pub may_enter_measurement_pipeline: bool,
    pub visibility_assessed: bool,
    pub photometric_usability_assessed: bool,
    pub feature_measurability_assessed: bool,
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

fn precedence(status: AcquisitionStatus) -> u8 {
    match status {
        AcquisitionStatus::CalibrationNotValid => 0,
        AcquisitionStatus::ProjectorPatternMismatch => 1,
        AcquisitionStatus::MotionModelOutOfDomain => 2,
        AcquisitionStatus::TimingUncertaintyExceeded => 3,
        AcquisitionStatus::MotionBlurBudgetExceeded => 4,
        AcquisitionStatus::RollingShutterBudgetExceeded => 5,
        AcquisitionStatus::CombinedAcquisitionBudgetExceeded => 6,
        AcquisitionStatus::Indeterminate => 7,
        AcquisitionStatus::AcquisitionValidWithinBudget => 8,
    }
}

fn choose_status(statuses: &[AcquisitionStatus]) -> AcquisitionStatus {
    statuses
        .iter()
        .copied()
        .min_by_key(|status| precedence(*status))
        .unwrap_or(AcquisitionStatus::AcquisitionValidWithinBudget)
}

fn add_reason(
    status: AcquisitionStatus,
    code: &str,
    statuses: &mut Vec<AcquisitionStatus>,
    reasons: &mut BTreeSet<String>,
) {
    statuses.push(status);
    reasons.insert(code.to_owned());
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, String> {
    if denominator == 0 {
        return Err("ZERO_DENOMINATOR".to_owned());
    }
    let quotient = numerator / denominator;
    if numerator % denominator == 0 {
        Ok(quotient)
    } else {
        quotient.checked_add(1).ok_or_else(|| "ARITHMETIC_OVERFLOW".to_owned())
    }
}

fn pixel_bound_micro(
    l1_velocity_um_per_s: u128,
    duration_ns: u64,
    gsd_um_per_pixel: u64,
) -> Result<u64, String> {
    if gsd_um_per_pixel == 0 {
        return Err("GSD_MUST_BE_POSITIVE".to_owned());
    }
    let numerator = l1_velocity_um_per_s
        .checked_mul(u128::from(duration_ns))
        .ok_or_else(|| "MOTION_NUMERATOR_OVERFLOW".to_owned())?;
    let denominator = u128::from(gsd_um_per_pixel)
        .checked_mul(1_000)
        .ok_or_else(|| "MOTION_DENOMINATOR_OVERFLOW".to_owned())?;
    let result = ceil_div(numerator, denominator)?;
    u64::try_from(result).map_err(|_| "MOTION_BOUND_EXCEEDS_U64".to_owned())
}

fn checked_add(left: u64, right: u64, code: &str) -> Result<u64, String> {
    left.checked_add(right).ok_or_else(|| code.to_owned())
}

fn checked_projector_interval(
    case: &Case,
    half_exposure_ns: u64,
    maximum_row_offset_ns: u64,
) -> Result<[i128; 2], String> {
    let mut span = u128::from(half_exposure_ns);
    span = span
        .checked_add(u128::from(maximum_row_offset_ns))
        .ok_or_else(|| "PROJECTOR_SPAN_OVERFLOW".to_owned())?;
    span = span
        .checked_add(u128::from(case.timestamp_uncertainty_ns))
        .ok_or_else(|| "PROJECTOR_SPAN_OVERFLOW".to_owned())?;
    span = span
        .checked_add(u128::from(case.trigger_jitter_ns))
        .ok_or_else(|| "PROJECTOR_SPAN_OVERFLOW".to_owned())?;
    let span = i128::try_from(span).map_err(|_| "PROJECTOR_SPAN_OVERFLOW".to_owned())?;
    let center = i128::from(case.frame_timestamp_ns);
    let lower = center.checked_sub(span).ok_or_else(|| "PROJECTOR_INTERVAL_OVERFLOW".to_owned())?;
    let upper = center.checked_add(span).ok_or_else(|| "PROJECTOR_INTERVAL_OVERFLOW".to_owned())?;
    Ok([lower, upper])
}

fn validate_case(case: &Case) -> Result<(), String> {
    if !valid_text(&case.case_id)
        || !valid_text(&case.camera_id)
        || !valid_text(&case.motion_model_id)
        || !valid_sha256(&case.calibration_receipt_sha256)
        || !valid_sha256(&case.clock_map_receipt_sha256)
        || case.gsd_um_per_pixel == 0
        || case.exposure_duration_ns == 0
        || case.image_height_px == 0
        || case.motion_budget_micro_px == 0
    {
        return Err("ACQUISITION_INPUT_IDENTITY_OR_LIMIT_INVALID".to_owned());
    }

    match case.shutter_mode {
        ShutterMode::GlobalShutter => {
            if case.readout_duration_ns != 0 || case.readout_uncertainty_ns != 0 {
                return Err("GLOBAL_SHUTTER_HAS_ROLLING_READOUT".to_owned());
            }
        }
        ShutterMode::RollingShutter => {
            if case.image_height_px < 2 || case.readout_duration_ns == 0 {
                return Err("ROLLING_SHUTTER_READOUT_INVALID".to_owned());
            }
        }
    }

    if !case.requires_projector && case.projector_window.is_some() {
        return Err("UNEXPECTED_PROJECTOR_WINDOW".to_owned());
    }
    if let Some(window) = &case.projector_window {
        if !valid_text(&window.pattern_id)
            || !valid_sha256(&window.pattern_receipt_sha256)
            || window.stable_end_ns <= window.stable_start_ns
        {
            return Err("PROJECTOR_WINDOW_INVALID".to_owned());
        }
    }
    Ok(())
}

fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    validate_case(case)?;

    let mut statuses = Vec::new();
    let mut reasons = BTreeSet::new();

    match case.calibration_state {
        CalibrationState::Valid => {}
        CalibrationState::Invalid => add_reason(
            AcquisitionStatus::CalibrationNotValid,
            "CALIBRATION_CHAIN_NOT_VALID",
            &mut statuses,
            &mut reasons,
        ),
        CalibrationState::Unknown => add_reason(
            AcquisitionStatus::Indeterminate,
            "CALIBRATION_CHAIN_STATE_UNKNOWN",
            &mut statuses,
            &mut reasons,
        ),
    }

    match case.clock_map_state {
        ClockMapState::Valid => {}
        ClockMapState::Invalid => add_reason(
            AcquisitionStatus::TimingUncertaintyExceeded,
            "CLOCK_MAP_NOT_VALID",
            &mut statuses,
            &mut reasons,
        ),
        ClockMapState::Unknown => add_reason(
            AcquisitionStatus::Indeterminate,
            "CLOCK_MAP_STATE_UNKNOWN",
            &mut statuses,
            &mut reasons,
        ),
    }

    match case.motion_model_state {
        MotionModelState::Admitted => {}
        MotionModelState::OutOfDomain => add_reason(
            AcquisitionStatus::MotionModelOutOfDomain,
            "MOTION_MODEL_OUT_OF_DOMAIN",
            &mut statuses,
            &mut reasons,
        ),
        MotionModelState::Unknown => add_reason(
            AcquisitionStatus::Indeterminate,
            "MOTION_MODEL_STATE_UNKNOWN",
            &mut statuses,
            &mut reasons,
        ),
    }

    if case.timestamp_reference != TimestampReference::CenterRowExposureMidpoint {
        add_reason(
            AcquisitionStatus::Indeterminate,
            "TIMESTAMP_REFERENCE_UNSUPPORTED",
            &mut statuses,
            &mut reasons,
        );
    }

    let vx = u128::from(case.relative_velocity_x_um_per_s.unsigned_abs());
    let vy = u128::from(case.relative_velocity_y_um_per_s.unsigned_abs());
    let l1_velocity = vx
        .checked_add(vy)
        .ok_or_else(|| "RELATIVE_VELOCITY_OVERFLOW".to_owned())?;

    let exposure_smear = pixel_bound_micro(
        l1_velocity,
        case.exposure_duration_ns,
        case.gsd_um_per_pixel,
    )?;

    let maximum_row_offset_ns = match case.shutter_mode {
        ShutterMode::GlobalShutter => 0,
        ShutterMode::RollingShutter => {
            let full_readout = checked_add(
                case.readout_duration_ns,
                case.readout_uncertainty_ns,
                "READOUT_DURATION_OVERFLOW",
            )?;
            full_readout
                .checked_add(1)
                .ok_or_else(|| "READOUT_DURATION_OVERFLOW".to_owned())?
                / 2
        }
    };

    let rolling_shutter_bound = pixel_bound_micro(
        l1_velocity,
        maximum_row_offset_ns,
        case.gsd_um_per_pixel,
    )?;

    let association_time_ns = checked_add(
        case.timestamp_uncertainty_ns,
        case.trigger_jitter_ns,
        "TIMING_ASSOCIATION_OVERFLOW",
    )?;
    let timing_association_bound = pixel_bound_micro(
        l1_velocity,
        association_time_ns,
        case.gsd_um_per_pixel,
    )?;

    let total_motion_bound = exposure_smear
        .checked_add(rolling_shutter_bound)
        .and_then(|value| value.checked_add(timing_association_bound))
        .ok_or_else(|| "TOTAL_MOTION_BOUND_OVERFLOW".to_owned())?;

    if exposure_smear > case.motion_budget_micro_px {
        add_reason(
            AcquisitionStatus::MotionBlurBudgetExceeded,
            "EXPOSURE_SMEAR_BUDGET_EXCEEDED",
            &mut statuses,
            &mut reasons,
        );
    }
    if rolling_shutter_bound > case.motion_budget_micro_px {
        add_reason(
            AcquisitionStatus::RollingShutterBudgetExceeded,
            "ROLLING_SHUTTER_BUDGET_EXCEEDED",
            &mut statuses,
            &mut reasons,
        );
    }
    if timing_association_bound > case.motion_budget_micro_px {
        add_reason(
            AcquisitionStatus::TimingUncertaintyExceeded,
            "TIMING_ASSOCIATION_BUDGET_EXCEEDED",
            &mut statuses,
            &mut reasons,
        );
    }
    if total_motion_bound > case.motion_budget_micro_px {
        add_reason(
            AcquisitionStatus::CombinedAcquisitionBudgetExceeded,
            "COMBINED_ACQUISITION_BUDGET_EXCEEDED",
            &mut statuses,
            &mut reasons,
        );
    }

    let half_exposure_ns = case
        .exposure_duration_ns
        .checked_add(1)
        .ok_or_else(|| "EXPOSURE_DURATION_OVERFLOW".to_owned())?
        / 2;

    let projector_frame_interval_ns = if case.requires_projector {
        let interval = checked_projector_interval(
            case,
            half_exposure_ns,
            maximum_row_offset_ns,
        )?;
        if case.timestamp_reference != TimestampReference::CenterRowExposureMidpoint {
            add_reason(
                AcquisitionStatus::Indeterminate,
                "PROJECTOR_TIME_REFERENCE_UNRESOLVED",
                &mut statuses,
                &mut reasons,
            );
        } else if let Some(window) = &case.projector_window {
            let stable_start = i128::from(window.stable_start_ns);
            let stable_end = i128::from(window.stable_end_ns);
            if interval[1] < stable_start || interval[0] >= stable_end {
                add_reason(
                    AcquisitionStatus::ProjectorPatternMismatch,
                    "PROJECTOR_PATTERN_WINDOW_DISJOINT",
                    &mut statuses,
                    &mut reasons,
                );
            } else if interval[0] < stable_start || interval[1] >= stable_end {
                add_reason(
                    AcquisitionStatus::Indeterminate,
                    "PROJECTOR_PATTERN_WINDOW_BOUNDARY_OVERLAP",
                    &mut statuses,
                    &mut reasons,
                );
            }
        } else {
            add_reason(
                AcquisitionStatus::Indeterminate,
                "PROJECTOR_PATTERN_EVIDENCE_MISSING",
                &mut statuses,
                &mut reasons,
            );
        }
        Some(interval)
    } else {
        None
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        camera_id: case.camera_id.clone(),
        calibration_state: case.calibration_state,
        calibration_receipt_sha256: case.calibration_receipt_sha256.clone(),
        clock_map_state: case.clock_map_state,
        clock_map_receipt_sha256: case.clock_map_receipt_sha256.clone(),
        timestamp_reference: case.timestamp_reference,
        shutter_mode: case.shutter_mode,
        readout_direction: case.readout_direction,
        exposure_smear_bound_micro_px: exposure_smear,
        rolling_shutter_bound_micro_px: rolling_shutter_bound,
        timing_association_bound_micro_px: timing_association_bound,
        total_motion_bound_micro_px: total_motion_bound,
        motion_budget_micro_px: case.motion_budget_micro_px,
        maximum_row_offset_ns,
        projector_frame_interval_ns,
        status: choose_status(&statuses),
        reason_codes: reasons.into_iter().collect(),
        may_enter_measurement_pipeline: choose_status(&statuses)
            == AcquisitionStatus::AcquisitionValidWithinBudget,
        visibility_assessed: false,
        photometric_usability_assessed: false,
        feature_measurability_assessed: false,
    })
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
        || corpus.cases.len() > 256
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.clone()) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }

    let expected_outcome_matches = cases
        .iter()
        .zip(corpus.cases.iter())
        .filter(|(result, source)| result.status == source.expected_status)
        .count();

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

    fn base_case() -> Case {
        Case {
            case_id: "base".to_owned(),
            camera_id: "CAM-01".to_owned(),
            calibration_state: CalibrationState::Valid,
            calibration_receipt_sha256: "a".repeat(64),
            clock_map_state: ClockMapState::Valid,
            clock_map_receipt_sha256: "b".repeat(64),
            timestamp_reference: TimestampReference::CenterRowExposureMidpoint,
            frame_timestamp_ns: 5_000,
            timestamp_uncertainty_ns: 0,
            trigger_jitter_ns: 0,
            motion_model_id: "REL-VELOCITY-BOUND-01".to_owned(),
            motion_model_state: MotionModelState::Admitted,
            relative_velocity_x_um_per_s: 0,
            relative_velocity_y_um_per_s: 0,
            gsd_um_per_pixel: 10,
            exposure_duration_ns: 1_000_000,
            shutter_mode: ShutterMode::GlobalShutter,
            image_height_px: 100,
            readout_duration_ns: 0,
            readout_uncertainty_ns: 0,
            readout_direction: ReadoutDirection::TopToBottom,
            motion_budget_micro_px: 100_000,
            requires_projector: false,
            projector_window: None,
            expected_status: AcquisitionStatus::AcquisitionValidWithinBudget,
        }
    }

    #[test]
    fn static_global_shutter_case_is_valid() {
        let result = evaluate_case(&base_case()).expect("valid case");
        assert_eq!(result.status, AcquisitionStatus::AcquisitionValidWithinBudget);
        assert_eq!(result.total_motion_bound_micro_px, 0);
        assert!(!result.visibility_assessed);
        assert!(!result.feature_measurability_assessed);
    }

    #[test]
    fn exact_exposure_budget_boundary_is_admitted() {
        let mut value = base_case();
        value.relative_velocity_x_um_per_s = 1_000;
        let result = evaluate_case(&value).expect("valid case");
        assert_eq!(result.exposure_smear_bound_micro_px, 100_000);
        assert_eq!(result.status, AcquisitionStatus::AcquisitionValidWithinBudget);
    }

    #[test]
    fn exposure_motion_exceedance_is_rejected() {
        let mut value = base_case();
        value.relative_velocity_x_um_per_s = 2_000;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::MotionBlurBudgetExceeded);
    }

    #[test]
    fn rolling_shutter_row_time_exceedance_is_separate() {
        let mut value = base_case();
        value.relative_velocity_x_um_per_s = 1_000;
        value.exposure_duration_ns = 100_000;
        value.shutter_mode = ShutterMode::RollingShutter;
        value.readout_duration_ns = 3_000_000;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.rolling_shutter_bound_micro_px, 150_000);
        assert_eq!(result.status, AcquisitionStatus::RollingShutterBudgetExceeded);
    }

    #[test]
    fn timing_association_error_is_separate() {
        let mut value = base_case();
        value.relative_velocity_x_um_per_s = 1_000;
        value.exposure_duration_ns = 100_000;
        value.timestamp_uncertainty_ns = 2_000_000;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::TimingUncertaintyExceeded);
    }

    #[test]
    fn individual_bounds_can_pass_while_the_composite_budget_fails() {
        let mut value = base_case();
        value.relative_velocity_x_um_per_s = 1_000;
        value.exposure_duration_ns = 700_000;
        value.shutter_mode = ShutterMode::RollingShutter;
        value.readout_duration_ns = 700_000;
        value.timestamp_uncertainty_ns = 100_000;
        let result = evaluate_case(&value).expect("valid input");
        assert!(result.exposure_smear_bound_micro_px < value.motion_budget_micro_px);
        assert!(result.rolling_shutter_bound_micro_px < value.motion_budget_micro_px);
        assert!(result.timing_association_bound_micro_px < value.motion_budget_micro_px);
        assert_eq!(result.status, AcquisitionStatus::CombinedAcquisitionBudgetExceeded);
    }

    #[test]
    fn calibration_invalidity_blocks_acquisition() {
        let mut value = base_case();
        value.calibration_state = CalibrationState::Invalid;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::CalibrationNotValid);
    }

    #[test]
    fn unknown_clock_map_is_indeterminate() {
        let mut value = base_case();
        value.clock_map_state = ClockMapState::Unknown;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::Indeterminate);
    }

    #[test]
    fn out_of_domain_motion_is_not_admitted() {
        let mut value = base_case();
        value.motion_model_state = MotionModelState::OutOfDomain;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::MotionModelOutOfDomain);
    }

    #[test]
    fn unsupported_timestamp_reference_is_indeterminate() {
        let mut value = base_case();
        value.timestamp_reference = TimestampReference::HostArrival;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::Indeterminate);
    }

    #[test]
    fn missing_projector_pattern_is_indeterminate() {
        let mut value = base_case();
        value.requires_projector = true;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::Indeterminate);
    }

    #[test]
    fn disjoint_projector_pattern_window_is_rejected() {
        let mut value = base_case();
        value.requires_projector = true;
        value.projector_window = Some(ProjectorWindow {
            pattern_id: "PATTERN-1".to_owned(),
            stable_start_ns: 0,
            stable_end_ns: 1_000,
            pattern_receipt_sha256: "c".repeat(64),
        });
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::ProjectorPatternMismatch);
    }

    #[test]
    fn projector_window_overlap_is_indeterminate() {
        let mut value = base_case();
        value.requires_projector = true;
        value.projector_window = Some(ProjectorWindow {
            pattern_id: "PATTERN-1".to_owned(),
            stable_start_ns: 4_700,
            stable_end_ns: 6_000,
            pattern_receipt_sha256: "c".repeat(64),
        });
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::Indeterminate);
    }

    #[test]
    fn complete_projector_pattern_window_is_admitted() {
        let mut value = base_case();
        value.requires_projector = true;
        value.projector_window = Some(ProjectorWindow {
            pattern_id: "PATTERN-1".to_owned(),
            stable_start_ns: 4_000,
            stable_end_ns: 6_000,
            pattern_receipt_sha256: "c".repeat(64),
        });
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.status, AcquisitionStatus::AcquisitionValidWithinBudget);
    }

    #[test]
    fn reversed_rolling_readout_has_same_symmetric_bound() {
        let mut value = base_case();
        value.relative_velocity_x_um_per_s = 1_000;
        value.exposure_duration_ns = 100_000;
        value.shutter_mode = ShutterMode::RollingShutter;
        value.readout_duration_ns = 1_000_000;
        value.readout_direction = ReadoutDirection::BottomToTop;
        let result = evaluate_case(&value).expect("valid input");
        assert_eq!(result.maximum_row_offset_ns, 500_000);
        assert_eq!(result.rolling_shutter_bound_micro_px, 50_000);
        assert_eq!(result.status, AcquisitionStatus::AcquisitionValidWithinBudget);
    }

    #[test]
    fn global_shutter_cannot_claim_nonzero_readout() {
        let mut value = base_case();
        value.readout_duration_ns = 10;
        assert_eq!(evaluate_case(&value).unwrap_err(), "GLOBAL_SHUTTER_HAS_ROLLING_READOUT");
    }

    #[test]
    fn rolling_shutter_requires_readout_model() {
        let mut value = base_case();
        value.shutter_mode = ShutterMode::RollingShutter;
        assert_eq!(evaluate_case(&value).unwrap_err(), "ROLLING_SHUTTER_READOUT_INVALID");
    }

    #[test]
    fn zero_gsd_is_rejected() {
        let mut value = base_case();
        value.gsd_um_per_pixel = 0;
        assert_eq!(evaluate_case(&value).unwrap_err(), "ACQUISITION_INPUT_IDENTITY_OR_LIMIT_INVALID");
    }
}
