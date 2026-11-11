// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const MAX_CORPUS_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CASES: usize = 10_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    corpus_id: String,
    claim_ceiling: String,
    cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    building_group_id: String,
    capture_group_id: String,
    visibility: VisibilityStatus,
    observation: ObservationStatus,
    sensor: SensorEvidence,
    validation: ValidationEvidence,
    measurement: Option<Measurement>,
    task: TaskRequirement,
    expected: Expected,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum VisibilityStatus {
    Observable,
    SelfOccluded,
    ExternallyOccluded,
    OutsideSensorCoverage,
    Unknown,
    Contradictory,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ObservationStatus {
    Captured,
    NotCaptured,
    InvalidReturn,
    MixedReturn,
    Contradictory,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CalibrationStatus {
    Qualified,
    Missing,
    Stale,
    Invalid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct QualifiedSensorDomain {
    min_range_m: f64,
    max_range_m: f64,
    max_incidence_angle_rad: f64,
    surface_response_classes: Vec<String>,
    environment_classes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservedSensorConditions {
    range_m: f64,
    incidence_angle_rad: f64,
    surface_response_class: String,
    environment_class: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ValidationStatus {
    Qualified,
    Missing,
    Stale,
    Contradictory,
    Invalid,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum IndependenceStatus {
    Independent,
    Correlated,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MeasurementUnit {
    Metre,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SensorEvidence {
    sensor_id: String,
    profile_id: String,
    profile_sha256: String,
    calibration_status: CalibrationStatus,
    calibration_evidence_sha256: Option<String>,
    qualified_domain: Option<QualifiedSensorDomain>,
    observed_conditions: Option<ObservedSensorConditions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ValidationEvidence {
    status: ValidationStatus,
    evidence_sha256: Option<String>,
    independence: IndependenceStatus,
    calibration_reference_group_ids: Vec<String>,
    holdout_reference_group_ids: Vec<String>,
    independent_reference_lineage_ids: Vec<String>,
    minimum_independent_lineages: usize,
    parameters_frozen_before_holdout: bool,
    calibration_fit_receipt_sha256: Option<String>,
    frozen_parameter_digest_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Measurement {
    value_m: f64,
    standard_uncertainty_m: f64,
    coverage_factor: f64,
    coverage_probability: f64,
    uncertainty_method_id: String,
    uncertainty_evidence_sha256: String,
    systematic_error_bound_m: Option<f64>,
    systematic_bound_method_id: Option<String>,
    systematic_bound_evidence_sha256: Option<String>,
    unit: MeasurementUnit,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TaskRequirement {
    task_id: String,
    maximum_total_error_bound_m: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    overall_disposition: OverallDisposition,
    reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OverallDisposition {
    Pass,
    Fail,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GateStatus {
    Pass,
    Fail,
    Unresolved,
    NotRun,
}

#[derive(Debug, Clone, Serialize)]
struct GateResult {
    status: GateStatus,
    reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CaseResult {
    case_id: String,
    building_group_id: String,
    capture_group_id: String,
    visibility_gate: GateResult,
    observation_gate: GateResult,
    metrology_gate: GateResult,
    task_sufficiency_gate: GateResult,
    conservative_total_error_bound_m: Option<f64>,
    overall_disposition: OverallDisposition,
    reason_codes: Vec<String>,
    expected_disposition: OverallDisposition,
    expected_reason_codes: Vec<String>,
    matches_expected: bool,
}

#[derive(Debug, Serialize)]
struct ReceiptCore {
    schema_version: String,
    corpus_id: String,
    claim_ceiling: String,
    case_count: usize,
    matched_expected_cases: usize,
    mismatch_count: usize,
    passed: bool,
    cases: Vec<CaseResult>,
}

#[derive(Debug, Serialize)]
struct Receipt {
    input_sha256: String,
    receipt_sha256: String,
    core: ReceiptCore,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn gate(status: GateStatus, reason_codes: &[&str]) -> GateResult {
    let mut reasons = reason_codes
        .iter()
        .map(|reason| (*reason).to_owned())
        .collect::<Vec<_>>();
    reasons.sort();
    reasons.dedup();
    GateResult {
        status,
        reason_codes: reasons,
    }
}

fn not_run() -> GateResult {
    gate(GateStatus::NotRun, &[])
}

fn visibility_gate(status: VisibilityStatus) -> GateResult {
    match status {
        VisibilityStatus::Observable => gate(GateStatus::Pass, &[]),
        VisibilityStatus::SelfOccluded => gate(GateStatus::Fail, &["GEOMETRICALLY_SELF_OCCLUDED"]),
        VisibilityStatus::ExternallyOccluded => {
            gate(GateStatus::Fail, &["GEOMETRICALLY_EXTERNALLY_OCCLUDED"])
        }
        VisibilityStatus::OutsideSensorCoverage => {
            gate(GateStatus::Fail, &["OUTSIDE_SENSOR_COVERAGE"])
        }
        VisibilityStatus::Unknown => gate(GateStatus::Unresolved, &["VISIBILITY_UNKNOWN"]),
        VisibilityStatus::Contradictory => {
            gate(GateStatus::Unresolved, &["VISIBILITY_CONTRADICTORY"])
        }
    }
}

fn observation_gate(case: &Case) -> GateResult {
    match case.observation {
        ObservationStatus::Captured => {
            if case.visibility != VisibilityStatus::Observable {
                return gate(
                    GateStatus::Unresolved,
                    &["CAPTURE_CONFLICTS_WITH_VISIBILITY"],
                );
            }
            if case.measurement.is_none() {
                return gate(GateStatus::Fail, &["CAPTURED_MEASUREMENT_MISSING"]);
            }
            gate(GateStatus::Pass, &[])
        }
        ObservationStatus::NotCaptured => {
            if case.measurement.is_some() {
                gate(
                    GateStatus::Fail,
                    &["MEASUREMENT_PRESENT_WITHOUT_CAPTURE_STATUS"],
                )
            } else {
                gate(GateStatus::Fail, &["OBSERVATION_NOT_CAPTURED"])
            }
        }
        ObservationStatus::InvalidReturn => gate(GateStatus::Fail, &["OBSERVATION_INVALID_RETURN"]),
        ObservationStatus::MixedReturn => {
            gate(GateStatus::Unresolved, &["OBSERVATION_MIXED_RETURN"])
        }
        ObservationStatus::Contradictory => {
            gate(GateStatus::Unresolved, &["OBSERVATION_CONTRADICTORY"])
        }
        ObservationStatus::Unknown => gate(GateStatus::Unresolved, &["OBSERVATION_UNKNOWN"]),
    }
}

fn check_sensor_domain(sensor: &SensorEvidence, reasons: &mut BTreeSet<&'static str>) {
    let Some(domain) = &sensor.qualified_domain else {
        reasons.insert("SENSOR_QUALIFIED_DOMAIN_UNKNOWN");
        return;
    };
    let Some(conditions) = &sensor.observed_conditions else {
        reasons.insert("SENSOR_OBSERVED_CONDITIONS_UNKNOWN");
        return;
    };

    let surface_classes: BTreeSet<&str> = domain
        .surface_response_classes
        .iter()
        .map(String::as_str)
        .collect();
    let environment_classes: BTreeSet<&str> = domain
        .environment_classes
        .iter()
        .map(String::as_str)
        .collect();

    let invalid_domain = !domain.min_range_m.is_finite()
        || !domain.max_range_m.is_finite()
        || domain.min_range_m < 0.0
        || domain.max_range_m <= domain.min_range_m
        || !domain.max_incidence_angle_rad.is_finite()
        || domain.max_incidence_angle_rad <= 0.0
        || domain.max_incidence_angle_rad > std::f64::consts::FRAC_PI_2
        || domain.surface_response_classes.is_empty()
        || domain.environment_classes.is_empty()
        || domain
            .surface_response_classes
            .iter()
            .any(|class| class.trim().is_empty())
        || domain
            .environment_classes
            .iter()
            .any(|class| class.trim().is_empty())
        || surface_classes.len() != domain.surface_response_classes.len()
        || environment_classes.len() != domain.environment_classes.len();

    if invalid_domain {
        reasons.insert("SENSOR_PROFILE_DOMAIN_INVALID");
        return;
    }

    let invalid_conditions = !conditions.range_m.is_finite()
        || conditions.range_m < 0.0
        || !conditions.incidence_angle_rad.is_finite()
        || conditions.incidence_angle_rad < 0.0
        || conditions.incidence_angle_rad > std::f64::consts::FRAC_PI_2
        || conditions.surface_response_class.trim().is_empty()
        || conditions.environment_class.trim().is_empty();

    if invalid_conditions {
        reasons.insert("OBSERVED_SENSOR_CONDITIONS_INVALID");
        return;
    }

    if conditions.range_m < domain.min_range_m || conditions.range_m > domain.max_range_m {
        reasons.insert("SENSOR_OPERATING_DOMAIN_OUT_OF_DOMAIN");
    }
    if conditions.incidence_angle_rad > domain.max_incidence_angle_rad {
        reasons.insert("SENSOR_INCIDENCE_OUT_OF_DOMAIN");
    }
    if !surface_classes.contains(conditions.surface_response_class.as_str()) {
        reasons.insert("SENSOR_SURFACE_RESPONSE_OUT_OF_DOMAIN");
    }
    if !environment_classes.contains(conditions.environment_class.as_str()) {
        reasons.insert("SENSOR_ENVIRONMENT_OUT_OF_DOMAIN");
    }
}

fn metrology_gate(case: &Case, observation: &GateResult) -> GateResult {
    if observation.status != GateStatus::Pass {
        return not_run();
    }

    let mut reasons = BTreeSet::new();
    let sensor = &case.sensor;
    let validation = &case.validation;
    let measurement = match &case.measurement {
        Some(measurement) => measurement,
        None => return gate(GateStatus::Fail, &["CAPTURED_MEASUREMENT_MISSING"]),
    };

    if sensor.sensor_id.trim().is_empty()
        || sensor.profile_id.trim().is_empty()
        || !is_sha256(&sensor.profile_sha256)
    {
        reasons.insert("SENSOR_PROFILE_BINDING_INVALID");
    }

    match sensor.calibration_status {
        CalibrationStatus::Qualified => {
            if !sensor
                .calibration_evidence_sha256
                .as_deref()
                .is_some_and(is_sha256)
            {
                reasons.insert("CALIBRATION_EVIDENCE_MISSING_OR_INVALID");
            }
        }
        CalibrationStatus::Missing => {
            reasons.insert("CALIBRATION_MISSING");
        }
        CalibrationStatus::Stale => {
            reasons.insert("CALIBRATION_STALE");
        }
        CalibrationStatus::Invalid => {
            reasons.insert("CALIBRATION_INVALID");
        }
    }

    check_sensor_domain(sensor, &mut reasons);

    match validation.status {
        ValidationStatus::Qualified => {
            if !validation.evidence_sha256.as_deref().is_some_and(is_sha256) {
                reasons.insert("INDEPENDENT_VALIDATION_EVIDENCE_MISSING_OR_INVALID");
            }
        }
        ValidationStatus::Missing => {
            reasons.insert("INDEPENDENT_VALIDATION_MISSING");
        }
        ValidationStatus::Stale => {
            reasons.insert("INDEPENDENT_VALIDATION_STALE");
        }
        ValidationStatus::Contradictory => {
            reasons.insert("INDEPENDENT_VALIDATION_CONTRADICTORY");
        }
        ValidationStatus::Invalid => {
            reasons.insert("INDEPENDENT_VALIDATION_INVALID");
        }
    }

    match validation.independence {
        IndependenceStatus::Independent => {}
        IndependenceStatus::Correlated => {
            reasons.insert("REFERENCE_LINEAGES_CORRELATED");
        }
        IndependenceStatus::Unknown => {
            reasons.insert("REFERENCE_LINEAGE_INDEPENDENCE_UNKNOWN");
        }
    }

    let lineage_ids: BTreeSet<&str> = validation
        .independent_reference_lineage_ids
        .iter()
        .map(String::as_str)
        .collect();

    if validation.minimum_independent_lineages == 0
        || validation
            .independent_reference_lineage_ids
            .iter()
            .any(|lineage| lineage.trim().is_empty())
    {
        reasons.insert("REFERENCE_LINEAGE_REQUIREMENT_INVALID");
    } else if lineage_ids.len() != validation.independent_reference_lineage_ids.len() {
        reasons.insert("DUPLICATE_REFERENCE_LINEAGE_NOT_INDEPENDENT");
    } else if lineage_ids.len() < validation.minimum_independent_lineages {
        reasons.insert("INSUFFICIENT_INDEPENDENT_REFERENCE_LINEAGES");
    }

    // P1 interval validation uses distinct calibration and holdout
    // groups. Freeze parameters fitted on the calibration groups before
    // evaluating independent holdout observations; never refit on them.
    if validation.status == ValidationStatus::Qualified {
        let calibration_group_ids: BTreeSet<&str> = validation
            .calibration_reference_group_ids
            .iter()
            .map(String::as_str)
            .collect();
        let holdout_group_ids: BTreeSet<&str> = validation
            .holdout_reference_group_ids
            .iter()
            .map(String::as_str)
            .collect();

        if validation.calibration_reference_group_ids.is_empty() {
            reasons.insert("CALIBRATION_REFERENCE_GROUPS_MISSING");
        }
        if validation.holdout_reference_group_ids.is_empty() {
            reasons.insert("HOLDOUT_REFERENCE_GROUPS_MISSING");
        }
        if validation
            .calibration_reference_group_ids
            .iter()
            .any(|group| group.trim().is_empty())
        {
            reasons.insert("CALIBRATION_REFERENCE_GROUP_ID_INVALID");
        }
        if validation
            .holdout_reference_group_ids
            .iter()
            .any(|group| group.trim().is_empty())
        {
            reasons.insert("HOLDOUT_REFERENCE_GROUP_ID_INVALID");
        }
        if calibration_group_ids.len() != validation.calibration_reference_group_ids.len() {
            reasons.insert("CALIBRATION_REFERENCE_GROUP_ID_DUPLICATE");
        }
        if holdout_group_ids.len() != validation.holdout_reference_group_ids.len() {
            reasons.insert("HOLDOUT_REFERENCE_GROUP_ID_DUPLICATE");
        }
        if !calibration_group_ids.is_disjoint(&holdout_group_ids) {
            reasons.insert("CALIBRATION_HOLDOUT_GROUP_OVERLAP");
        }
        if !validation.parameters_frozen_before_holdout {
            reasons.insert("HOLDOUT_PARAMETERS_NOT_FROZEN");
        }
        if !validation
            .calibration_fit_receipt_sha256
            .as_deref()
            .is_some_and(is_sha256)
        {
            reasons.insert("CALIBRATION_FIT_RECEIPT_MISSING_OR_INVALID");
        }
        if !validation
            .frozen_parameter_digest_sha256
            .as_deref()
            .is_some_and(is_sha256)
        {
            reasons.insert("FROZEN_PARAMETER_DIGEST_MISSING_OR_INVALID");
        }
    }

    if !measurement.value_m.is_finite()
        || !measurement.standard_uncertainty_m.is_finite()
        || measurement.standard_uncertainty_m < 0.0
        || !measurement.coverage_factor.is_finite()
        || measurement.coverage_factor <= 0.0
        || !measurement.coverage_probability.is_finite()
        || !(0.0..1.0).contains(&measurement.coverage_probability)
        || measurement.uncertainty_method_id.trim().is_empty()
        || !is_sha256(&measurement.uncertainty_evidence_sha256)
        || measurement.unit != MeasurementUnit::Metre
    {
        reasons.insert("INVALID_MEASUREMENT_UNCERTAINTY");
    }

    match measurement.systematic_error_bound_m {
        None => {
            reasons.insert("SYSTEMATIC_ERROR_BOUND_UNKNOWN");
        }
        Some(bound) if !bound.is_finite() || bound < 0.0 => {
            reasons.insert("INVALID_SYSTEMATIC_ERROR_BOUND");
        }
        Some(_) => {
            let method_id = measurement.systematic_bound_method_id.as_deref();
            let evidence_digest = measurement.systematic_bound_evidence_sha256.as_deref();
            if !method_id.is_some_and(|value| !value.trim().is_empty())
                || !evidence_digest.is_some_and(is_sha256)
            {
                reasons.insert("SYSTEMATIC_ERROR_BOUND_EVIDENCE_MISSING_OR_INVALID");
            }
        }
    }

    if !reasons.is_empty() {
        let fail = reasons.contains("CALIBRATION_INVALID")
            || reasons.contains("INDEPENDENT_VALIDATION_INVALID")
            || reasons.contains("INVALID_MEASUREMENT_UNCERTAINTY")
            || reasons.contains("SENSOR_PROFILE_BINDING_INVALID")
            || reasons.contains("SENSOR_PROFILE_DOMAIN_INVALID")
            || reasons.contains("OBSERVED_SENSOR_CONDITIONS_INVALID")
            || reasons.contains("INVALID_SYSTEMATIC_ERROR_BOUND")
            || reasons.contains("CALIBRATION_REFERENCE_GROUP_ID_INVALID")
            || reasons.contains("CALIBRATION_REFERENCE_GROUP_ID_DUPLICATE")
            || reasons.contains("HOLDOUT_REFERENCE_GROUP_ID_INVALID")
            || reasons.contains("HOLDOUT_REFERENCE_GROUP_ID_DUPLICATE")
            || reasons.contains("CALIBRATION_HOLDOUT_GROUP_OVERLAP")
            || reasons.contains("HOLDOUT_PARAMETERS_NOT_FROZEN");
        let status = if fail {
            GateStatus::Fail
        } else {
            GateStatus::Unresolved
        };
        return GateResult {
            status,
            reason_codes: reasons.into_iter().map(str::to_owned).collect(),
        };
    }

    gate(GateStatus::Pass, &[])
}

fn task_gate(case: &Case, metrology: &GateResult) -> (GateResult, Option<f64>) {
    if metrology.status != GateStatus::Pass {
        return (not_run(), None);
    }

    let measurement = match &case.measurement {
        Some(measurement) => measurement,
        None => return (gate(GateStatus::Fail, &["TASK_MEASUREMENT_MISSING"]), None),
    };

    if case.task.task_id.trim().is_empty()
        || !case.task.maximum_total_error_bound_m.is_finite()
        || case.task.maximum_total_error_bound_m <= 0.0
    {
        return (gate(GateStatus::Fail, &["TASK_REQUIREMENT_INVALID"]), None);
    }

    let Some(systematic_bound) = measurement.systematic_error_bound_m else {
        return (
            gate(GateStatus::Unresolved, &["SYSTEMATIC_ERROR_BOUND_UNKNOWN"]),
            None,
        );
    };

    // Keep stochastic dispersion and persistent systematic bounds separate.
    // The task-facing conservative bound is k*u + b, never RSS-combined.
    let conservative_total_error_bound_m =
        measurement.standard_uncertainty_m * measurement.coverage_factor + systematic_bound;
    if !conservative_total_error_bound_m.is_finite() {
        return (
            gate(GateStatus::Fail, &["TOTAL_ERROR_BOUND_NON_FINITE"]),
            None,
        );
    }

    if conservative_total_error_bound_m > case.task.maximum_total_error_bound_m {
        (
            gate(GateStatus::Fail, &["TASK_TOTAL_ERROR_BOUND_EXCEEDED"]),
            Some(conservative_total_error_bound_m),
        )
    } else {
        (
            gate(GateStatus::Pass, &[]),
            Some(conservative_total_error_bound_m),
        )
    }
}

fn overall_disposition(gates: [&GateResult; 4]) -> OverallDisposition {
    if gates.iter().any(|result| result.status == GateStatus::Fail) {
        OverallDisposition::Fail
    } else if gates.iter().all(|result| result.status == GateStatus::Pass) {
        OverallDisposition::Pass
    } else {
        OverallDisposition::Unresolved
    }
}

fn evaluate_case(case: &Case) -> CaseResult {
    let visibility = visibility_gate(case.visibility);
    let observation = observation_gate(case);
    let metrology = metrology_gate(case, &observation);
    let (task, conservative_total_error_bound_m) = task_gate(case, &metrology);
    let disposition = overall_disposition([&visibility, &observation, &metrology, &task]);

    let mut reasons = BTreeSet::new();
    for result in [&visibility, &observation, &metrology, &task] {
        reasons.extend(result.reason_codes.iter().cloned());
    }

    let mut expected_reasons = case.expected.reason_codes.clone();
    expected_reasons.sort();
    expected_reasons.dedup();
    let reason_codes = reasons.into_iter().collect::<Vec<_>>();
    let matches_expected =
        disposition == case.expected.overall_disposition && reason_codes == expected_reasons;

    CaseResult {
        case_id: case.case_id.clone(),
        building_group_id: case.building_group_id.clone(),
        capture_group_id: case.capture_group_id.clone(),
        visibility_gate: visibility,
        observation_gate: observation,
        metrology_gate: metrology,
        task_sufficiency_gate: task,
        conservative_total_error_bound_m,
        overall_disposition: disposition,
        reason_codes,
        expected_disposition: case.expected.overall_disposition,
        expected_reason_codes: expected_reasons,
        matches_expected,
    }
}

fn evaluate_corpus(corpus: &Corpus, input_bytes: &[u8]) -> Result<Receipt, String> {
    if corpus.schema_version != "0.1.0" {
        return Err(format!(
            "unsupported schema_version {}; expected 0.1.0",
            corpus.schema_version
        ));
    }
    if corpus.corpus_id.trim().is_empty() || corpus.claim_ceiling.trim().is_empty() {
        return Err("corpus_id and claim_ceiling must be non-empty".to_owned());
    }
    if corpus.cases.is_empty() || corpus.cases.len() > MAX_CASES {
        return Err(format!(
            "case count must be in 1..={MAX_CASES}; received {}",
            corpus.cases.len()
        ));
    }

    let mut seen_case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if case.case_id.trim().is_empty()
            || case.building_group_id.trim().is_empty()
            || case.capture_group_id.trim().is_empty()
        {
            return Err("case and lineage identifiers must be non-empty".to_owned());
        }
        if !seen_case_ids.insert(case.case_id.as_str()) {
            return Err(format!("duplicate case_id {}", case.case_id));
        }
    }

    let mut cases = corpus.cases.iter().map(evaluate_case).collect::<Vec<_>>();
    cases.sort_by_key(|case| case.case_id.clone());

    let matched_expected_cases = cases.iter().filter(|case| case.matches_expected).count();
    let mismatch_count = cases.len() - matched_expected_cases;
    let core = ReceiptCore {
        schema_version: corpus.schema_version.clone(),
        corpus_id: corpus.corpus_id.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        case_count: cases.len(),
        matched_expected_cases,
        mismatch_count,
        passed: mismatch_count == 0,
        cases,
    };
    let core_bytes = serde_json::to_vec(&core).map_err(|error| error.to_string())?;
    Ok(Receipt {
        input_sha256: sha256_hex(input_bytes),
        receipt_sha256: sha256_hex(&core_bytes),
        core,
    })
}

fn run() -> Result<i32, Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let mut corpus_path: Option<PathBuf> = None;

    while let Some(argument) = args.next() {
        if argument == "--corpus" {
            if corpus_path.is_some() {
                return Err("--corpus may be supplied only once".into());
            }
            corpus_path = Some(PathBuf::from(
                args.next().ok_or("missing value after --corpus")?,
            ));
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()).into());
        }
    }

    let corpus_path = corpus_path.ok_or("expected --corpus <json-file>")?;
    let metadata = fs::metadata(&corpus_path)?;
    if metadata.len() > MAX_CORPUS_BYTES {
        return Err(format!("corpus exceeds maximum size of {MAX_CORPUS_BYTES} bytes").into());
    }

    let input_bytes = fs::read(&corpus_path)?;
    let corpus: Corpus = serde_json::from_slice(&input_bytes)?;
    let receipt = evaluate_corpus(&corpus, &input_bytes).map_err(std::io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);

    Ok(if receipt.core.passed { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("scan-bim-measurement-observability: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/measurement-observability.json");

    fn fixture() -> Corpus {
        serde_json::from_str(FIXTURE).expect("fixture corpus must parse")
    }

    #[test]
    fn frozen_synthetic_corpus_matches_every_expected_result() {
        let corpus = fixture();
        let receipt = evaluate_corpus(&corpus, FIXTURE.as_bytes()).expect("evaluate corpus");
        assert!(receipt.core.passed, "{:#?}", receipt.core.cases);
        assert_eq!(receipt.core.case_count, 34);
        assert_eq!(receipt.core.mismatch_count, 0);
    }

    #[test]
    fn visibility_does_not_imply_observation() {
        let mut corpus = fixture();
        let case = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "visible_but_not_captured")
            .expect("known fixture case");
        assert_eq!(visibility_gate(case.visibility).status, GateStatus::Pass);
        case.observation = ObservationStatus::NotCaptured;
        case.measurement = None;
        let result = evaluate_case(case);
        assert_eq!(result.observation_gate.status, GateStatus::Fail);
        assert_eq!(result.overall_disposition, OverallDisposition::Fail);
    }

    #[test]
    fn duplicate_reference_lineage_never_meets_independence_requirement() {
        let mut corpus = fixture();
        let case = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "duplicate_reference_lineage")
            .expect("known fixture case");
        case.validation.independent_reference_lineage_ids =
            vec!["reference-a".to_owned(), "reference-a".to_owned()];
        let result = evaluate_case(case);
        assert_eq!(result.metrology_gate.status, GateStatus::Unresolved);
        assert!(
            result
                .reason_codes
                .contains(&"DUPLICATE_REFERENCE_LINEAGE_NOT_INDEPENDENT".to_owned())
        );
    }

    #[test]
    fn same_measurement_can_pass_one_task_and_fail_another() {
        let corpus = fixture();
        let coarse = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "coarse_layout_pass")
            .expect("coarse task fixture");
        let tight = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "tight_installation_fail")
            .expect("tight task fixture");
        assert_eq!(
            evaluate_case(coarse).overall_disposition,
            OverallDisposition::Pass
        );
        assert_eq!(
            evaluate_case(tight).overall_disposition,
            OverallDisposition::Fail
        );
    }

    #[test]
    fn case_order_does_not_change_qualification_receipt_digest() {
        let mut corpus = fixture();
        let bytes = serde_json::to_vec(&corpus).expect("serialize original");
        let original = evaluate_corpus(&corpus, &bytes).expect("original receipt");
        corpus.cases.reverse();
        let reversed_bytes = serde_json::to_vec(&corpus).expect("serialize reversed");
        let reversed = evaluate_corpus(&corpus, &reversed_bytes).expect("reversed receipt");
        assert_eq!(original.receipt_sha256, reversed.receipt_sha256);
        assert_ne!(original.input_sha256, reversed.input_sha256);
    }

    #[test]
    fn closed_schema_rejects_unknown_fields() {
        let mut value: serde_json::Value =
            serde_json::from_str(FIXTURE).expect("fixture JSON parses");
        value
            .as_object_mut()
            .expect("corpus root is an object")
            .insert("unexpected_field".to_owned(), serde_json::Value::Bool(true));
        let encoded = serde_json::to_vec(&value).expect("mutated JSON serializes");
        assert!(serde_json::from_slice::<Corpus>(&encoded).is_err());
    }

    #[test]
    fn calibration_and_holdout_group_ids_must_be_disjoint() {
        let mut corpus = fixture();
        let case = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "calibration_holdout_lineage_overlap_fail")
            .expect("lineage overlap fixture");
        let result = evaluate_case(case);
        assert_eq!(result.metrology_gate.status, GateStatus::Fail);
        assert!(
            result
                .reason_codes
                .contains(&"CALIBRATION_HOLDOUT_GROUP_OVERLAP".to_owned())
        );
    }

    #[test]
    fn duplicate_calibration_or_holdout_group_ids_are_rejected() {
        let mut corpus = fixture();
        let duplicate_calibration = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "duplicate_calibration_reference_group_fail")
            .expect("duplicate calibration group fixture");
        let result = evaluate_case(duplicate_calibration);
        assert_eq!(result.metrology_gate.status, GateStatus::Fail);
        assert!(
            result
                .reason_codes
                .contains(&"CALIBRATION_REFERENCE_GROUP_ID_DUPLICATE".to_owned())
        );

        let duplicate_holdout = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "duplicate_holdout_reference_group_fail")
            .expect("duplicate holdout group fixture");
        let result = evaluate_case(duplicate_holdout);
        assert_eq!(result.metrology_gate.status, GateStatus::Fail);
        assert!(
            result
                .reason_codes
                .contains(&"HOLDOUT_REFERENCE_GROUP_ID_DUPLICATE".to_owned())
        );
    }

    #[test]
    fn holdout_reference_group_is_required_for_qualified_validation() {
        let corpus = fixture();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "holdout_reference_groups_missing_unresolved")
            .expect("missing holdout groups fixture");
        let result = evaluate_case(case);
        assert_eq!(result.metrology_gate.status, GateStatus::Unresolved);
        assert!(
            result
                .reason_codes
                .contains(&"HOLDOUT_REFERENCE_GROUPS_MISSING".to_owned())
        );
    }

    #[test]
    fn holdout_parameters_must_be_frozen_before_validation() {
        let corpus = fixture();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "parameters_refit_on_holdout_fail")
            .expect("refit fixture");
        let result = evaluate_case(case);
        assert_eq!(result.metrology_gate.status, GateStatus::Fail);
        assert!(
            result
                .reason_codes
                .contains(&"HOLDOUT_PARAMETERS_NOT_FROZEN".to_owned())
        );
    }

    #[test]
    fn systematic_error_bound_is_added_conservatively_to_stochastic_coverage() {
        let corpus = fixture();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "systematic_error_bound_changes_task_acceptance")
            .expect("systematic bound fixture");
        let result = evaluate_case(case);
        assert_eq!(result.overall_disposition, OverallDisposition::Fail);
        assert_eq!(result.conservative_total_error_bound_m, Some(0.016));
        assert!(
            result
                .reason_codes
                .contains(&"TASK_TOTAL_ERROR_BOUND_EXCEEDED".to_owned())
        );
    }

    #[test]
    fn unknown_systematic_bound_never_defaults_to_zero() {
        let corpus = fixture();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "systematic_error_bound_unknown_unresolved")
            .expect("unknown systematic fixture");
        let result = evaluate_case(case);
        assert_eq!(result.metrology_gate.status, GateStatus::Unresolved);
        assert_eq!(result.overall_disposition, OverallDisposition::Unresolved);
        assert!(
            result
                .reason_codes
                .contains(&"SYSTEMATIC_ERROR_BOUND_UNKNOWN".to_owned())
        );
    }

    #[test]
    fn grazing_incidence_and_unsupported_surface_are_out_of_domain() {
        let mut corpus = fixture();
        let grazing = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "grazing_incidence_out_of_domain")
            .expect("grazing fixture");
        let result = evaluate_case(grazing);
        assert_eq!(result.metrology_gate.status, GateStatus::Unresolved);
        assert!(
            result
                .reason_codes
                .contains(&"SENSOR_INCIDENCE_OUT_OF_DOMAIN".to_owned())
        );

        let unsupported = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "unsupported_surface_response_out_of_domain")
            .expect("surface fixture");
        let result = evaluate_case(unsupported);
        assert_eq!(result.metrology_gate.status, GateStatus::Unresolved);
        assert!(
            result
                .reason_codes
                .contains(&"SENSOR_SURFACE_RESPONSE_OUT_OF_DOMAIN".to_owned())
        );
    }

    #[test]
    fn sensor_applicability_boolean_is_rejected_as_an_unknown_field() {
        let mut value: serde_json::Value =
            serde_json::from_str(FIXTURE).expect("fixture JSON parses");
        value["cases"][0]["sensor"]["applicability"] =
            serde_json::Value::String("in_domain".to_owned());
        let encoded = serde_json::to_vec(&value).expect("mutated JSON serializes");
        assert!(serde_json::from_slice::<Corpus>(&encoded).is_err());
    }

    #[test]
    fn unsupported_sensor_domain_cannot_pass() {
        let mut corpus = fixture();
        let case = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "out_of_domain_unresolved")
            .expect("known fixture case");
        case.sensor
            .observed_conditions
            .as_mut()
            .expect("sensor conditions")
            .range_m = 25.0;
        let result = evaluate_case(case);
        assert_eq!(result.metrology_gate.status, GateStatus::Unresolved);
        assert!(!result.reason_codes.is_empty());
    }
}
