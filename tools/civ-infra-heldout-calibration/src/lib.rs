#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "civ-infra-heldout-calibration/v0.1";
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RECORDS: usize = 10_000;
pub const MIN_CALIBRATION_RECORDS: usize = 3;
const CLAIM_CEILING: &str =
    "synthetic_held_out_validation_only_no_physical_bound_or_statistical_coverage_guarantee";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CalibrationInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub quantity_kind: String,
    pub unit: String,
    pub decision_limit_ticks: i64,
    pub lower_is_better: bool,
    pub records: Vec<PairedRecord>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PairedRecord {
    pub case_id: String,
    pub split: DataSplit,
    pub site_id: String,
    pub instrument_id: String,
    pub model_family_id: String,
    pub applicability_domain_id: String,
    pub unit: String,
    pub model_prediction_ticks: i64,
    pub model_numerical_uncertainty_ticks: u64,
    pub reference_value_ticks: i64,
    pub reference_uncertainty_ticks: u64,
    pub synthetic_ground_truth_ticks: i64,
    pub case_input_sha256: String,
    pub model_artifact_sha256: String,
    pub reference_artifact_sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSplit {
    Calibration,
    Holdout,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    CalibrationOnly,
    RobustFeasible,
    RobustInfeasible,
    Indeterminate,
    NotQualifiedOutOfDomain,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibratedEnvelope {
    pub lower_residual_ticks: i128,
    pub upper_residual_ticks: i128,
    pub calibration_record_count: usize,
    pub width_ticks: i128,
    pub derivation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CaseAssessment {
    pub case_id: String,
    pub disposition: DecisionStatus,
    pub applicability_supported: bool,
    pub residual_ticks: Option<i128>,
    pub residual_covered_by_envelope: Option<bool>,
    pub reference_interval_covers_synthetic_truth: bool,
    pub nominal_feasible: Option<bool>,
    pub calibrated_feasible: Option<bool>,
    pub synthetic_truth_feasible: Option<bool>,
    pub decision_changed_from_nominal: Option<bool>,
    pub false_robust_feasible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub benchmark_id: String,
    pub input_sha256: String,
    pub split_manifest_sha256: String,
    pub claim_ceiling: String,
    pub evidence_semantics: String,
    pub quantity_kind: String,
    pub unit: String,
    pub decision_limit_ticks: i64,
    pub calibration_record_count: usize,
    pub holdout_record_count: usize,
    pub qualified_holdout_count: usize,
    pub out_of_domain_holdout_count: usize,
    pub envelope: CalibratedEnvelope,
    pub heldout_residual_covered_count: usize,
    pub heldout_residual_miss_count: usize,
    pub heldout_coverage_denominator: usize,
    pub worst_residual_miss_distance_ticks: i128,
    pub nominal_to_calibrated_decision_change_count: usize,
    pub false_robust_feasible_count: usize,
    pub reference_interval_truth_miss_count: usize,
    pub case_assessments: Vec<CaseAssessment>,
}

#[derive(Debug, Clone)]
struct Interval {
    lower: i128,
    upper: i128,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Hash the exact input bytes, then validate and evaluate the paired synthetic records.
/// The discrepancy envelope is derived only from the calibration split.
pub fn evaluate_bytes(bytes: &[u8]) -> Result<CalibrationReport, String> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("input byte count exceeds limit {MAX_INPUT_BYTES}"));
    }
    let input_sha256 = sha256_hex(bytes);
    let input: CalibrationInput =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid input JSON: {error}"))?;
    evaluate(&input, &input_sha256)
}

fn evaluate(input: &CalibrationInput, input_sha256: &str) -> Result<CalibrationReport, String> {
    validate_digest(input_sha256, "input_sha256")?;
    validate_input(input)?;
    let split_manifest_sha256 = split_manifest_sha256(input)?;

    let calibration: Vec<&PairedRecord> = input
        .records
        .iter()
        .filter(|record| record.split == DataSplit::Calibration)
        .collect();
    let holdout: Vec<&PairedRecord> = input
        .records
        .iter()
        .filter(|record| record.split == DataSplit::Holdout)
        .collect();

    let mut envelope_lower = i128::MAX;
    let mut envelope_upper = i128::MIN;
    for record in &calibration {
        let interval = residual_interval(record);
        envelope_lower = envelope_lower.min(interval.lower);
        envelope_upper = envelope_upper.max(interval.upper);
    }
    if envelope_lower > envelope_upper {
        return Err("derived residual envelope is inverted".to_owned());
    }
    let envelope = CalibratedEnvelope {
        lower_residual_ticks: envelope_lower,
        upper_residual_ticks: envelope_upper,
        calibration_record_count: calibration.len(),
        width_ticks: envelope_upper - envelope_lower,
        derivation: "min/max of calibration-split residual intervals; finite synthetic sample envelope only, not a deterministic physical bound or statistical prediction guarantee".to_owned(),
    };

    let supported_domains: BTreeSet<&str> = calibration
        .iter()
        .map(|record| record.applicability_domain_id.as_str())
        .collect();

    let mut assessments = Vec::with_capacity(input.records.len());
    let mut qualified_count = 0usize;
    let mut ood_count = 0usize;
    let mut covered_count = 0usize;
    let mut miss_count = 0usize;
    let mut worst_miss = 0i128;
    let mut changed_count = 0usize;
    let mut false_robust_feasible_count = 0usize;
    let mut reference_truth_miss_count = 0usize;

    for record in input
        .records
        .iter()
        .filter(|r| r.split == DataSplit::Calibration)
    {
        let reference = reference_interval(record);
        let truth = i128::from(record.synthetic_ground_truth_ticks);
        if !contains(&reference, truth) {
            reference_truth_miss_count += 1;
        }
        assessments.push(CaseAssessment {
            case_id: record.case_id.clone(),
            disposition: DecisionStatus::CalibrationOnly,
            applicability_supported: true,
            residual_ticks: Some(truth - i128::from(record.model_prediction_ticks)),
            residual_covered_by_envelope: None,
            reference_interval_covers_synthetic_truth: contains(&reference, truth),
            nominal_feasible: None,
            calibrated_feasible: None,
            synthetic_truth_feasible: None,
            decision_changed_from_nominal: None,
            false_robust_feasible: false,
        });
    }

    for record in holdout {
        let reference = reference_interval(record);
        let truth = i128::from(record.synthetic_ground_truth_ticks);
        let reference_covers_truth = contains(&reference, truth);
        if !reference_covers_truth {
            reference_truth_miss_count += 1;
        }
        let in_domain = supported_domains.contains(record.applicability_domain_id.as_str());
        if !in_domain {
            ood_count += 1;
            assessments.push(CaseAssessment {
                case_id: record.case_id.clone(),
                disposition: DecisionStatus::NotQualifiedOutOfDomain,
                applicability_supported: false,
                residual_ticks: None,
                residual_covered_by_envelope: None,
                reference_interval_covers_synthetic_truth: reference_covers_truth,
                nominal_feasible: None,
                calibrated_feasible: None,
                synthetic_truth_feasible: None,
                decision_changed_from_nominal: None,
                false_robust_feasible: false,
            });
            continue;
        }

        qualified_count += 1;
        let residual = truth - i128::from(record.model_prediction_ticks);
        let covered = (envelope_lower..=envelope_upper).contains(&residual);
        if covered {
            covered_count += 1;
        } else {
            miss_count += 1;
            let distance = if residual < envelope_lower {
                envelope_lower - residual
            } else {
                residual - envelope_upper
            };
            worst_miss = worst_miss.max(distance);
        }

        let prediction = i128::from(record.model_prediction_ticks);
        let numerical_error = i128::from(record.model_numerical_uncertainty_ticks);
        let robust_lower = prediction - numerical_error + envelope_lower;
        let robust_upper = prediction + numerical_error + envelope_upper;
        let limit = i128::from(input.decision_limit_ticks);
        let nominal_feasible = if input.lower_is_better {
            prediction <= limit
        } else {
            prediction >= limit
        };
        let truth_feasible = if input.lower_is_better {
            truth <= limit
        } else {
            truth >= limit
        };
        let disposition = classify(robust_lower, robust_upper, limit, input.lower_is_better);
        let calibrated_feasible = disposition == DecisionStatus::RobustFeasible;
        let changed = if nominal_feasible {
            disposition != DecisionStatus::RobustFeasible
        } else {
            disposition != DecisionStatus::RobustInfeasible
        };
        if changed {
            changed_count += 1;
        }
        let false_robust_feasible = calibrated_feasible && !truth_feasible;
        if false_robust_feasible {
            false_robust_feasible_count += 1;
        }

        assessments.push(CaseAssessment {
            case_id: record.case_id.clone(),
            disposition,
            applicability_supported: true,
            residual_ticks: Some(residual),
            residual_covered_by_envelope: Some(covered),
            reference_interval_covers_synthetic_truth: reference_covers_truth,
            nominal_feasible: Some(nominal_feasible),
            calibrated_feasible: Some(calibrated_feasible),
            synthetic_truth_feasible: Some(truth_feasible),
            decision_changed_from_nominal: Some(changed),
            false_robust_feasible,
        });
    }

    Ok(CalibrationReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        evaluator_version: env!("CARGO_PKG_VERSION").to_owned(),
        benchmark_id: input.benchmark_id.clone(),
        input_sha256: input_sha256.to_owned(),
        split_manifest_sha256,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        evidence_semantics: "artifact SHA-256 strings are format-checked declarations only; this evaluator does not retrieve or verify external artifact bytes. Ground truth is synthetic. Coverage and envelope width are finite-suite diagnostics, not physical guarantees or probabilistic claims.".to_owned(),
        quantity_kind: input.quantity_kind.clone(),
        unit: input.unit.clone(),
        decision_limit_ticks: input.decision_limit_ticks,
        calibration_record_count: calibration.len(),
        holdout_record_count: assessments.len() - calibration.len(),
        qualified_holdout_count: qualified_count,
        out_of_domain_holdout_count: ood_count,
        envelope,
        heldout_residual_covered_count: covered_count,
        heldout_residual_miss_count: miss_count,
        heldout_coverage_denominator: qualified_count,
        worst_residual_miss_distance_ticks: worst_miss,
        nominal_to_calibrated_decision_change_count: changed_count,
        false_robust_feasible_count,
        reference_interval_truth_miss_count: reference_truth_miss_count,
        case_assessments: assessments,
    })
}

#[derive(Serialize)]
struct SplitManifest<'a> {
    calibration_case_ids: Vec<&'a str>,
    holdout_case_ids: Vec<&'a str>,
}

fn split_manifest_sha256(input: &CalibrationInput) -> Result<String, String> {
    let mut calibration_case_ids = input
        .records
        .iter()
        .filter(|record| record.split == DataSplit::Calibration)
        .map(|record| record.case_id.as_str())
        .collect::<Vec<_>>();
    let mut holdout_case_ids = input
        .records
        .iter()
        .filter(|record| record.split == DataSplit::Holdout)
        .map(|record| record.case_id.as_str())
        .collect::<Vec<_>>();
    calibration_case_ids.sort_unstable();
    holdout_case_ids.sort_unstable();
    let manifest = SplitManifest {
        calibration_case_ids,
        holdout_case_ids,
    };
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| format!("cannot serialize split manifest: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn classify(lower: i128, upper: i128, limit: i128, lower_is_better: bool) -> DecisionStatus {
    if lower_is_better {
        if upper <= limit {
            DecisionStatus::RobustFeasible
        } else if lower > limit {
            DecisionStatus::RobustInfeasible
        } else {
            DecisionStatus::Indeterminate
        }
    } else if lower >= limit {
        DecisionStatus::RobustFeasible
    } else if upper < limit {
        DecisionStatus::RobustInfeasible
    } else {
        DecisionStatus::Indeterminate
    }
}

fn residual_interval(record: &PairedRecord) -> Interval {
    let center =
        i128::from(record.reference_value_ticks) - i128::from(record.model_prediction_ticks);
    // Keep reference measurement uncertainty in the residual envelope. Model numerical
    // uncertainty remains a separate term when the envelope is applied to a prediction.
    let spread = i128::from(record.reference_uncertainty_ticks);
    Interval {
        lower: center - spread,
        upper: center + spread,
    }
}

fn reference_interval(record: &PairedRecord) -> Interval {
    let center = i128::from(record.reference_value_ticks);
    let radius = i128::from(record.reference_uncertainty_ticks);
    Interval {
        lower: center - radius,
        upper: center + radius,
    }
}

fn contains(interval: &Interval, value: i128) -> bool {
    interval.lower <= value && value <= interval.upper
}

fn validate_input(input: &CalibrationInput) -> Result<(), String> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {:?}",
            input.schema_version
        ));
    }
    validate_name(&input.benchmark_id, "benchmark_id")?;
    validate_name(&input.quantity_kind, "quantity_kind")?;
    validate_name(&input.unit, "unit")?;
    if input.claim_ceiling != CLAIM_CEILING {
        return Err("claim_ceiling must match the fixed synthetic-only ceiling".to_owned());
    }
    if input.records.is_empty() || input.records.len() > MAX_RECORDS {
        return Err(format!("record count must be within 1..={MAX_RECORDS}"));
    }

    let mut ids = BTreeSet::new();
    let mut calibration_count = 0usize;
    let mut holdout_count = 0usize;
    let mut calibration_sites = BTreeSet::new();
    let mut calibration_instruments = BTreeSet::new();
    let mut calibration_families = BTreeSet::new();
    let mut holdout_sites = BTreeSet::new();
    let mut holdout_instruments = BTreeSet::new();
    let mut holdout_families = BTreeSet::new();

    for record in &input.records {
        validate_name(&record.case_id, "case_id")?;
        if !ids.insert(record.case_id.clone()) {
            return Err(format!("duplicate case_id {:?}", record.case_id));
        }
        for (label, value) in [
            ("site_id", record.site_id.as_str()),
            ("instrument_id", record.instrument_id.as_str()),
            ("model_family_id", record.model_family_id.as_str()),
            (
                "applicability_domain_id",
                record.applicability_domain_id.as_str(),
            ),
        ] {
            validate_name(value, label)?;
        }
        if record.unit != input.unit {
            return Err(format!(
                "unit mismatch for {:?}: expected {:?}, received {:?}",
                record.case_id, input.unit, record.unit
            ));
        }
        validate_digest(&record.case_input_sha256, "case_input_sha256")?;
        validate_digest(&record.model_artifact_sha256, "model_artifact_sha256")?;
        validate_digest(
            &record.reference_artifact_sha256,
            "reference_artifact_sha256",
        )?;

        match record.split {
            DataSplit::Calibration => {
                calibration_count += 1;
                calibration_sites.insert(record.site_id.as_str());
                calibration_instruments.insert(record.instrument_id.as_str());
                calibration_families.insert(record.model_family_id.as_str());
            }
            DataSplit::Holdout => {
                holdout_count += 1;
                holdout_sites.insert(record.site_id.as_str());
                holdout_instruments.insert(record.instrument_id.as_str());
                holdout_families.insert(record.model_family_id.as_str());
            }
        }
    }

    if calibration_count < MIN_CALIBRATION_RECORDS {
        return Err(format!(
            "calibration record count must be at least {MIN_CALIBRATION_RECORDS}"
        ));
    }
    if holdout_count == 0 {
        return Err("at least one held-out record is required".to_owned());
    }
    if !calibration_sites.is_disjoint(&holdout_sites) {
        return Err("calibration/holdout leakage: site_id overlaps split boundary".to_owned());
    }
    if !calibration_instruments.is_disjoint(&holdout_instruments) {
        return Err(
            "calibration/holdout leakage: instrument_id overlaps split boundary".to_owned(),
        );
    }
    if !calibration_families.is_disjoint(&holdout_families) {
        return Err(
            "calibration/holdout leakage: model_family_id overlaps split boundary".to_owned(),
        );
    }
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > 1024 {
        return Err(format!("{label} exceeds 1024 UTF-8 bytes"));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../fixtures/calibration-v0.1.json");

    fn parse_fixture() -> CalibrationInput {
        serde_json::from_slice(FIXTURE).expect("frozen fixture parses")
    }

    fn report_for(input: &CalibrationInput) -> CalibrationReport {
        let bytes = serde_json::to_vec(input).expect("test input serializes");
        evaluate_bytes(&bytes).expect("test input is valid")
    }

    fn report_for_error(input: &CalibrationInput) -> String {
        let bytes = serde_json::to_vec(input).expect("input serializes");
        evaluate_bytes(&bytes).expect_err("invalid input fails closed")
    }

    fn digest(index: usize) -> String {
        format!("{index:064x}")
    }

    fn generated_world(world: usize) -> CalibrationInput {
        let r = world as i64;
        let mode = world % 5;
        let calibration_residuals: [i64; 4] = match mode {
            0 => [0, 0, 0, 0],
            1 => [4, 5, 6, 7],
            2 => [-4, -1, 2, 5],
            3 => [-2, -1, 0, 7],
            _ => [-4, -1, 2, 5],
        };
        let holdout_residuals: [i64; 3] = match mode {
            0 => [0, 0, 0],
            1 => [5, 6, 7],
            2 => [-2, 0, 4],
            3 => [-1, 2, 6],
            _ => [-2, 0, 12],
        };
        let holdout_predictions = if mode == 4 {
            [80_i64, 100, 70]
        } else {
            [80_i64, 100, 120]
        };

        let mut records = Vec::new();
        for (index, residual) in calibration_residuals.into_iter().enumerate() {
            let prediction = 100 + (index as i64 * 20) + r % 7;
            let reference_uncertainty = if mode == 2 {
                (index + 1) as u64
            } else if mode == 3 && index == 3 {
                2
            } else {
                1
            };
            records.push(PairedRecord {
                case_id: format!("w{world:03}-cal-{index:02}"),
                split: DataSplit::Calibration,
                site_id: format!("w{world:03}-cal-site-{index:02}"),
                instrument_id: format!("w{world:03}-cal-inst-{index:02}"),
                model_family_id: format!("w{world:03}-cal-model-{index:02}"),
                applicability_domain_id: "synthetic-corridor-domain-v1".to_owned(),
                unit: "mm".to_owned(),
                model_prediction_ticks: prediction,
                model_numerical_uncertainty_ticks: 1,
                reference_value_ticks: prediction + residual,
                reference_uncertainty_ticks: reference_uncertainty,
                synthetic_ground_truth_ticks: prediction + residual,
                case_input_sha256: digest(world * 100 + index + 1),
                model_artifact_sha256: digest(world * 100 + index + 11),
                reference_artifact_sha256: digest(world * 100 + index + 21),
            });
        }
        for index in 0..holdout_residuals.len() {
            let prediction = holdout_predictions[index] + r % 5;
            let residual = holdout_residuals[index];
            records.push(PairedRecord {
                case_id: format!("w{world:03}-holdout-{index:02}"),
                split: DataSplit::Holdout,
                site_id: format!("w{world:03}-holdout-site-{index:02}"),
                instrument_id: format!("w{world:03}-holdout-inst-{index:02}"),
                model_family_id: format!("w{world:03}-holdout-model-{index:02}"),
                applicability_domain_id: "synthetic-corridor-domain-v1".to_owned(),
                unit: "mm".to_owned(),
                model_prediction_ticks: prediction,
                model_numerical_uncertainty_ticks: 1,
                reference_value_ticks: prediction + residual,
                reference_uncertainty_ticks: 1,
                synthetic_ground_truth_ticks: prediction + residual,
                case_input_sha256: digest(world * 100 + index + 41),
                model_artifact_sha256: digest(world * 100 + index + 51),
                reference_artifact_sha256: digest(world * 100 + index + 61),
            });
        }

        CalibrationInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark_id: format!("generated-world-{world:03}"),
            quantity_kind: "synthetic-structural-response".to_owned(),
            unit: "mm".to_owned(),
            decision_limit_ticks: 100,
            lower_is_better: true,
            records,
            claim_ceiling: CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn frozen_fixture_has_hand_checkable_envelope_and_decision_results() {
        let report = evaluate_bytes(FIXTURE).expect("frozen fixture passes");
        assert_eq!(report.calibration_record_count, 3);
        assert_eq!(report.holdout_record_count, 3);
        assert_eq!(report.envelope.lower_residual_ticks, -3);
        assert_eq!(report.envelope.upper_residual_ticks, 8);
        assert_eq!(report.envelope.width_ticks, 11);
        assert_eq!(report.heldout_residual_covered_count, 3);
        assert_eq!(report.heldout_residual_miss_count, 0);
        assert_eq!(report.false_robust_feasible_count, 0);
        assert_eq!(report.nominal_to_calibrated_decision_change_count, 1);
        assert_eq!(report.reference_interval_truth_miss_count, 0);
        assert!(report.evidence_semantics.contains("does not retrieve"));
    }

    #[test]
    fn heldout_mutation_never_changes_calibrated_envelope_and_flags_false_robust_feasible() {
        let mut input = parse_fixture();
        let envelope_before = report_for(&input).envelope;
        let record = input
            .records
            .iter_mut()
            .find(|record| record.case_id == "heldout-near-boundary")
            .expect("heldout boundary case");
        record.model_prediction_ticks = 90;
        record.reference_value_ticks = 105;
        record.synthetic_ground_truth_ticks = 105;
        record.reference_uncertainty_ticks = 1;

        let report = report_for(&input);
        assert_eq!(report.envelope, envelope_before);
        assert_eq!(report.false_robust_feasible_count, 1);
        assert_eq!(report.heldout_residual_miss_count, 1);
    }

    #[test]
    fn rejects_unit_mismatch_and_malformed_provenance_digest() {
        let mut input = parse_fixture();
        input.records[0].unit = "m".to_owned();
        assert!(report_for_error(&input).contains("unit mismatch"));

        let mut input = parse_fixture();
        input.records[0].reference_artifact_sha256 = "not-a-digest".to_owned();
        assert!(report_for_error(&input).contains("lowercase 64-character"));
    }

    #[test]
    fn rejects_site_instrument_and_model_family_split_leakage() {
        for field in 0..3 {
            let mut input = parse_fixture();
            let cal = input.records[0].clone();
            let holdout = input
                .records
                .iter_mut()
                .find(|record| record.split == DataSplit::Holdout)
                .expect("holdout");
            match field {
                0 => holdout.site_id = cal.site_id,
                1 => holdout.instrument_id = cal.instrument_id,
                _ => holdout.model_family_id = cal.model_family_id,
            }
            assert!(report_for_error(&input).contains("leakage"));
        }
    }

    #[test]
    fn out_of_domain_holdout_is_not_scored_as_a_qualified_decision() {
        let mut input = parse_fixture();
        input
            .records
            .iter_mut()
            .find(|record| record.case_id == "heldout-safe")
            .expect("safe holdout")
            .applicability_domain_id = "unseen-domain".to_owned();
        let report = report_for(&input);
        assert_eq!(report.out_of_domain_holdout_count, 1);
        assert_eq!(report.qualified_holdout_count, 2);
        let assessment = report
            .case_assessments
            .iter()
            .find(|item| item.case_id == "heldout-safe")
            .expect("assessment");
        assert_eq!(
            assessment.disposition,
            DecisionStatus::NotQualifiedOutOfDomain
        );
        assert_eq!(assessment.nominal_feasible, None);
    }

    #[test]
    fn same_bytes_replay_identically_and_input_hash_binds_report() {
        let first = evaluate_bytes(FIXTURE).expect("first");
        let second = evaluate_bytes(FIXTURE).expect("second");
        assert_eq!(first, second);
        assert_eq!(first.input_sha256, sha256_hex(FIXTURE));
    }

    #[test]
    fn generated_100_worlds_match_independent_envelope_and_decision_oracle() {
        for world in 0..100 {
            let input = generated_world(world);
            let report = report_for(&input);

            let calibration = input
                .records
                .iter()
                .filter(|record| record.split == DataSplit::Calibration)
                .collect::<Vec<_>>();
            let mut oracle_lower = i128::MAX;
            let mut oracle_upper = i128::MIN;
            for record in calibration {
                let residual = i128::from(record.reference_value_ticks)
                    - i128::from(record.model_prediction_ticks);
                let spread = i128::from(record.reference_uncertainty_ticks);
                oracle_lower = oracle_lower.min(residual - spread);
                oracle_upper = oracle_upper.max(residual + spread);
            }
            assert_eq!(report.envelope.lower_residual_ticks, oracle_lower);
            assert_eq!(report.envelope.upper_residual_ticks, oracle_upper);
            assert_eq!(report.qualified_holdout_count, 3);
            let expected_misses = if world % 5 == 4 { 1 } else { 0 };
            assert_eq!(report.heldout_residual_covered_count, 3 - expected_misses);
            assert_eq!(report.heldout_residual_miss_count, expected_misses);
            assert_eq!(report.false_robust_feasible_count, 0);
            assert_eq!(report.heldout_coverage_denominator, 3);
        }
    }

    #[test]
    fn reference_interval_miss_is_reported_not_hidden() {
        let mut input = parse_fixture();
        input.records[0].reference_value_ticks = 101;
        input.records[0].synthetic_ground_truth_ticks = 103;
        input.records[0].reference_uncertainty_ticks = 1;
        let report = report_for(&input);
        assert_eq!(report.reference_interval_truth_miss_count, 1);
    }

    #[test]
    fn duplicate_case_ids_and_unrecognized_probability_fields_are_rejected() {
        let mut input = parse_fixture();
        input.records[3].case_id = input.records[0].case_id.clone();
        assert!(report_for_error(&input).contains("duplicate case_id"));

        let mut value: serde_json::Value =
            serde_json::from_slice(FIXTURE).expect("fixture JSON parses");
        value
            .as_object_mut()
            .expect("top-level object")
            .insert("magic_probability".to_owned(), serde_json::json!(0.99));
        let bytes = serde_json::to_vec(&value).expect("mutated JSON serializes");
        let error =
            evaluate_bytes(&bytes).expect_err("unknown probabilistic field must fail closed");
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn split_manifest_digest_is_stable_when_record_order_changes() {
        let mut input = parse_fixture();
        let baseline = report_for(&input);
        input.records.reverse();
        let reordered = report_for(&input);
        assert_ne!(baseline.input_sha256, reordered.input_sha256);
        assert_eq!(
            baseline.split_manifest_sha256,
            reordered.split_manifest_sha256
        );
        assert_eq!(baseline.split_manifest_sha256.len(), 64);
    }

    #[test]
    fn higher_is_better_threshold_has_symmetric_boundary_semantics() {
        let mut input = parse_fixture();
        input.lower_is_better = false;
        input.decision_limit_ticks = 100;
        for record in &mut input.records {
            record.model_prediction_ticks += 200;
            record.reference_value_ticks += 200;
            record.synthetic_ground_truth_ticks += 200;
        }
        let report = report_for(&input);
        assert_eq!(report.heldout_residual_covered_count, 3);
        assert_eq!(report.false_robust_feasible_count, 0);
    }
}
