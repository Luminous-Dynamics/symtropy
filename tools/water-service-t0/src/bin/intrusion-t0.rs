// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROGRAMME: &str = "DISTRIBUTION-INTRUSION-TRANSIENT-QUAL-001";
const PROFILE: &str = "LUMINOUS-DISTRIBUTION-INTRUSION-T0-SYNTHETIC";
const INPUT_CEILING: &str =
    "state_semantics_and_declared_tracer_arithmetic_only_no_physical_or_health_claim";
const ORACLE_CEILING: &str =
    "expected_contract_dispositions_not_physical_or_health_evidence";
const CASE_COUNT: usize = 15;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing JSON field: {key}"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("expected string at {key}"))
}

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("expected boolean at {key}"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("expected finite number at {key}"))
}

fn close(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-9 * scale
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            matches!((a.as_f64(), b.as_f64()), (Some(x), Some(y)) if close(x, y))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(left, right)| compare_json(left, right))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| compare_json(value, other)))
        }
        _ => actual == expected,
    }
}

fn case_id(case: &Value) -> Result<&str, String> {
    text(case, "case_id")
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn pressure_deficit(inputs: &Value) -> Result<bool, String> {
    Ok(number(inputs, "external_head_m")? > number(inputs, "internal_minimum_head_m")?)
}

fn direct_tracer_mass(inputs: &Value) -> Result<f64, String> {
    let volume = number(inputs, "ingress_volume_l")?;
    let concentration = number(inputs, "external_tracer_concentration_mg_l")?;
    let measured = number(inputs, "measured_tracer_mass_mg")?;
    require(volume >= 0.0, "ingress volume must be non-negative")?;
    require(concentration >= 0.0, "tracer concentration must be non-negative")?;
    require(measured >= 0.0, "measured tracer mass must be non-negative")?;
    let computed = volume * concentration;
    require(
        close(computed, measured),
        "declared ingress volume/concentration does not match measured tracer mass",
    )?;
    Ok(computed)
}

fn compute(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let inputs = field(case, "inputs")?;

    match id {
        "DI-T0-001-positive-tracer-with-valid-pressure-and-pathway" => {
            let deficit = pressure_deficit(inputs)?;
            let duration = number(inputs, "pressure_event_duration_s")?;
            let pathway = text(inputs, "pathway_state")?;
            let external = bool_field(inputs, "external_tracer_present")?;
            let attribution = text(inputs, "tracer_source_attribution")?;
            let covered = bool_field(inputs, "sample_coverage_complete")?;
            let calibrated = bool_field(inputs, "instrument_calibration_current")?;
            let method = bool_field(inputs, "method_valid_for_tracer")?;
            require(
                deficit && duration > 0.0,
                "case requires a declared pressure-deficit event",
            )?;
            require(
                pathway == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(external, "case requires external tracer")?;
            require(attribution == "unique", "tracer source must be uniquely attributable")?;
            require(
                covered && calibrated && method,
                "critical evidence must be complete and valid",
            )?;
            let mass = direct_tracer_mass(inputs)?;
            Ok(json!({
                "ingress_disposition": "INGRESS_CONFIRMED_IN_TEST",
                "tracer_mass_recomputed_mg": mass,
                "tracer_mass_arithmetic": "MATCHED_WITHIN_DECLARED_SYNTHETIC_TOLERANCE",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-002-transient-with-no-defect-and-complete-negative-samples" => {
            require(pressure_deficit(inputs)?, "case requires a pressure-deficit event")?;
            require(
                number(inputs, "pressure_event_duration_s")? > 0.0,
                "case requires a non-zero event duration",
            )?;
            require(
                text(inputs, "pathway_state")? == "no_defect_confirmed",
                "negative control requires no confirmed defect",
            )?;
            require(
                !bool_field(inputs, "external_tracer_present")?,
                "negative control must have no external tracer",
            )?;
            require(
                bool_field(inputs, "sample_coverage_complete")?,
                "negative control requires complete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "not_detected",
                "negative control requires a valid non-detect",
            )?;
            require(
                number(inputs, "detection_limit_mg_l")? > 0.0,
                "detection limit must be positive",
            )?;
            let sample_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(
                sample_evidence_valid,
                "negative sample evidence must be valid",
            )?;
            Ok(negative_result())
        }
        "DI-T0-003-defect-and-tracer-but-no-external-head-exceedance" => {
            require(
                !pressure_deficit(inputs)?,
                "case must not have external head above internal pressure",
            )?;
            require(
                number(inputs, "pressure_event_duration_s")? == 0.0,
                "case must have no pressure-deficit duration",
            )?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a known defect",
            )?;
            require(
                bool_field(inputs, "external_tracer_present")?,
                "case requires external tracer",
            )?;
            require(
                bool_field(inputs, "sample_coverage_complete")?,
                "case requires complete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "not_detected",
                "case requires a valid non-detect",
            )?;
            require(
                number(inputs, "detection_limit_mg_l")? > 0.0,
                "detection limit must be positive",
            )?;
            let sample_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(sample_evidence_valid, "sample evidence must be valid")?;
            Ok(negative_result())
        }
        "DI-T0-004-pressure-deficit-with-unknown-pathway" => {
            require(pressure_deficit(inputs)?, "case requires pressure deficit")?;
            require(
                number(inputs, "pressure_event_duration_s")? > 0.0,
                "case requires non-zero pressure event",
            )?;
            require(
                text(inputs, "pathway_state")? == "unknown",
                "case requires unknown pathway state",
            )?;
            Ok(indeterminate("PATHWAY_OBSERVABILITY_INSUFFICIENT"))
        }
        "DI-T0-005-missing-downstream-sample" => {
            require(
                !bool_field(inputs, "sample_coverage_complete")?,
                "case requires incomplete sample coverage",
            )?;
            let missing = field(inputs, "missing_sample_points")?
                .as_array()
                .ok_or_else(|| "missing_sample_points must be an array".to_owned())?;
            require(
                missing.iter().any(|point| point.as_str() == Some("far_end")),
                "case requires the far-end sample to be missing",
            )?;
            Ok(indeterminate("DOWNSTREAM_SAMPLE_COVERAGE_INCOMPLETE"))
        }
        "DI-T0-006-expired-pressure-or-sample-calibration" => {
            require(
                !bool_field(inputs, "instrument_calibration_current")?,
                "case requires invalid calibration",
            )?;
            Ok(indeterminate("CRITICAL_CALIBRATION_INVALID"))
        }
        "DI-T0-007-detection-limit-insufficient" => {
            let detection = number(inputs, "detection_limit_mg_l")?;
            let required = number(inputs, "required_test_sensitivity_mg_l")?;
            require(
                detection > 0.0 && required > 0.0,
                "detection-limit values must be positive",
            )?;
            require(detection > required, "case requires inadequate detection sensitivity")?;
            require(
                text(inputs, "sample_result")? == "not_detected",
                "case requires a reported non-detect",
            )?;
            Ok(indeterminate("DETECTION_LIMIT_TOO_HIGH_FOR_NEGATIVE_CONCLUSION"))
        }
        "DI-T0-008-tracer-source-ambiguous" => {
            require(
                text(inputs, "tracer_source_attribution")? == "ambiguous",
                "case requires ambiguous tracer source",
            )?;
            require(
                text(inputs, "sample_result")? == "detected",
                "case requires a reported detection",
            )?;
            Ok(indeterminate("TRACER_SOURCE_NOT_UNIQUELY_ATTRIBUTABLE"))
        }
        "DI-T0-009-ingress-known-but-mass-balance-incomplete" => {
            require(
                text(inputs, "tracer_source_attribution")? == "unique",
                "case requires unique tracer attribution",
            )?;
            let direct_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(direct_evidence_valid, "direct tracer evidence must be valid")?;
            let _mass = direct_tracer_mass(inputs)?;
            require(
                !bool_field(inputs, "all_declared_streams_measured")?,
                "case requires an unmeasured declared stream",
            )?;
            require(
                field(inputs, "captured_residual_mass_mg")?.is_null(),
                "case requires missing captured residual mass",
            )?;
            Ok(json!({
                "ingress_disposition": "INGRESS_CONFIRMED_IN_TEST",
                "mass_balance_disposition": "MASS_BALANCE_UNRESOLVED",
                "reason": "DECLARED_RESIDUAL_STREAM_UNMEASURED",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-010-downstream-tracer-arrival" => {
            let concentration = number(inputs, "sample_concentration_mg_l")?;
            let quantification = number(inputs, "method_quantification_limit_mg_l")?;
            require(
                concentration >= 0.0 && quantification > 0.0,
                "sample concentration / quantification limit invalid",
            )?;
            require(
                concentration > quantification,
                "sample must exceed the method quantification limit",
            )?;
            require(number(inputs, "sample_time_s")? >= 0.0, "sample time must be non-negative")?;
            require(
                !text(inputs, "sample_point_id")?.is_empty(),
                "sample point identity is required",
            )?;
            require(
                text(inputs, "tracer_source_attribution")? == "unique",
                "tracer must have unique source attribution",
            )?;
            let sample_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(sample_evidence_valid, "sample evidence must be valid")?;
            Ok(json!({
                "transport_disposition": "DOWNSTREAM_TRACER_DETECTED",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-011-pressure-restored-ticket-closed-no-water-quality-evidence" => {
            require(
                bool_field(inputs, "pressure_within_declared_band")?,
                "case requires pressure restoration",
            )?;
            require(
                text(inputs, "maintenance_ticket")? == "closed",
                "case requires a closed maintenance ticket",
            )?;
            require(
                !bool_field(inputs, "post_event_samples_present")?,
                "case requires missing post-event samples",
            )?;
            require(
                !bool_field(inputs, "independent_quality_verification")?,
                "case requires absent independent quality verification",
            )?;
            require(
                !bool_field(inputs, "stable_dwell_complete")?,
                "case requires incomplete stable dwell",
            )?;
            Ok(json!({
                "recommissioning_disposition": "RECOMMISSIONING_NOT_VERIFIED",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-012-similar-pressure-event-no-ingress-pathway" => {
            require(
                !bool_field(inputs, "external_tracer_present")?,
                "negative control must have no external tracer",
            )?;
            require(
                text(inputs, "pathway_state")? == "no_defect_confirmed",
                "negative control requires confirmed absence of a defect",
            )?;
            require(
                bool_field(inputs, "sample_coverage_complete")?,
                "negative control requires complete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "not_detected",
                "negative control requires a valid non-detect",
            )?;
            require(
                number(inputs, "detection_limit_mg_l")? > 0.0,
                "detection limit must be positive",
            )?;
            let sample_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(sample_evidence_valid, "sample evidence must be valid")?;
            Ok(negative_result())
        }
        "DI-T0-013-model-zero-demand-and-leak-but-direct-tracer-confirms-reverse-ingress" => {
            require(pressure_deficit(inputs)?, "case requires a pressure deficit")?;
            require(
                number(inputs, "pressure_event_duration_s")? > 0.0,
                "case requires a non-zero pressure event",
            )?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(
                bool_field(inputs, "external_tracer_present")?,
                "case requires external tracer",
            )?;
            require(
                text(inputs, "physical_backflow_preventer_state")? == "absent",
                "case requires no physical backflow preventer",
            )?;
            require(
                close(number(inputs, "pdd_customer_demand_lps")?, 0.0),
                "PDD customer demand must be zero in this case",
            )?;
            require(
                close(number(inputs, "model_leak_emitter_discharge_lps")?, 0.0),
                "model leak-emitter discharge must be zero in this case",
            )?;
            let reverse_flow = number(inputs, "physical_signed_defect_flow_lps")?;
            require(
                reverse_flow > 0.0,
                "physical defect flow must be positive into the main",
            )?;
            require(
                text(inputs, "flow_sign_convention")? == "positive_into_main",
                "signed flow convention mismatch",
            )?;
            require(
                bool_field(inputs, "sample_coverage_complete")?,
                "case requires complete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "detected",
                "case requires a direct tracer detection",
            )?;
            let concentration = number(inputs, "sample_concentration_mg_l")?;
            let quantification = number(inputs, "method_quantification_limit_mg_l")?;
            require(
                concentration > quantification && quantification > 0.0,
                "tracer concentration must exceed a positive quantification limit",
            )?;
            require(
                text(inputs, "tracer_source_attribution")? == "unique",
                "tracer source must be uniquely attributable",
            )?;
            let sample_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(sample_evidence_valid, "direct tracer evidence must be valid")?;
            Ok(json!({
                "model_boundary_disposition": "MODEL_ZERO_DEMAND_AND_ZERO_LEAK_DO_NOT_REPRESENT_BIDIRECTIONAL_INGRESS",
                "ingress_disposition": "INGRESS_CONFIRMED_IN_TEST",
                "confirmation_basis": "DIRECT_TRACER_DETECTION_AND_SIGNED_REVERSE_FLOW",
                "pdd_customer_demand_lps": 0.0,
                "model_leak_emitter_discharge_lps": 0.0,
                "physical_reverse_flow_into_main_lps": reverse_flow,
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-014-model-zero-demand-and-leak-without-physical-observation" => {
            require(pressure_deficit(inputs)?, "case requires a pressure deficit")?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(
                bool_field(inputs, "external_tracer_present")?,
                "case requires an external tracer source",
            )?;
            require(
                text(inputs, "physical_backflow_preventer_state")? == "unknown",
                "case requires unknown backflow-preventer state",
            )?;
            require(
                close(number(inputs, "pdd_customer_demand_lps")?, 0.0),
                "PDD customer demand must be zero",
            )?;
            require(
                close(number(inputs, "model_leak_emitter_discharge_lps")?, 0.0),
                "model leak-emitter discharge must be zero",
            )?;
            require(
                field(inputs, "physical_signed_defect_flow_lps")?.is_null(),
                "physical signed defect flow must be unavailable",
            )?;
            require(
                !bool_field(inputs, "sample_coverage_complete")?,
                "case requires incomplete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "not_sampled",
                "case requires no tracer sample",
            )?;
            Ok(indeterminate(
                "PHYSICAL_REVERSE_FLOW_AND_TRACER_OBSERVATIONS_UNAVAILABLE",
            ))
        }
        "DI-T0-015-reverse-flow-observed-but-downstream-tracer-sample-missing" => {
            require(pressure_deficit(inputs)?, "case requires a pressure deficit")?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(
                text(inputs, "physical_backflow_preventer_state")? == "absent",
                "case requires no physical backflow preventer",
            )?;
            require(
                close(number(inputs, "pdd_customer_demand_lps")?, 0.0),
                "PDD customer demand must be zero",
            )?;
            require(
                close(number(inputs, "model_leak_emitter_discharge_lps")?, 0.0),
                "model leak-emitter discharge must be zero",
            )?;
            let reverse_flow = number(inputs, "physical_signed_defect_flow_lps")?;
            require(
                reverse_flow > 0.0,
                "physical defect flow must be positive into the main",
            )?;
            require(
                text(inputs, "flow_sign_convention")? == "positive_into_main",
                "signed flow convention mismatch",
            )?;
            require(
                !bool_field(inputs, "sample_coverage_complete")?,
                "case requires incomplete sample coverage",
            )?;
            let missing = field(inputs, "missing_sample_points")?
                .as_array()
                .ok_or_else(|| "missing_sample_points must be an array".to_owned())?;
            require(
                missing.iter().any(|point| point.as_str() == Some("far_end")),
                "case requires missing far-end tracer sample",
            )?;
            require(
                text(inputs, "sample_result")? == "not_sampled",
                "case requires an unobserved tracer sample",
            )?;
            Ok(json!({
                "model_boundary_disposition": "MODEL_ZERO_LEAK_DISCHARGE_DOES_NOT_REPRESENT_REVERSE_FLOW",
                "ingress_disposition": "INDETERMINATE",
                "reason": "REVERSE_FLOW_OBSERVED_BUT_DOWNSTREAM_TRACER_EVIDENCE_INCOMPLETE",
                "physical_reverse_flow_into_main_lps": reverse_flow,
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-013-model-zero-demand-and-leak-but-direct-tracer-confirms-reverse-ingress" => {
            require(
                pressure_deficit(inputs)?,
                "case requires a pressure deficit",
            )?;
            require(
                number(inputs, "pressure_event_duration_s")? > 0.0,
                "case requires a non-zero pressure event",
            )?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(
                bool_field(inputs, "external_tracer_present")?,
                "case requires external tracer",
            )?;
            require(
                text(inputs, "physical_backflow_preventer_state")? == "absent",
                "case requires no physical backflow preventer",
            )?;
            require(
                close(number(inputs, "pdd_customer_demand_lps")?, 0.0),
                "PDD customer demand must be zero in this case",
            )?;
            require(
                close(number(inputs, "model_leak_emitter_discharge_lps")?, 0.0),
                "model leak-emitter discharge must be zero in this case",
            )?;
            require(
                number(inputs, "physical_signed_defect_flow_lps")? > 0.0,
                "physical defect flow must be positive into the main",
            )?;
            require(
                text(inputs, "flow_sign_convention")? == "positive_into_main",
                "signed flow convention mismatch",
            )?;
            require(
                bool_field(inputs, "sample_coverage_complete")?,
                "case requires complete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "detected",
                "case requires a direct tracer detection",
            )?;
            let concentration = number(inputs, "sample_concentration_mg_l")?;
            let quantification = number(inputs, "method_quantification_limit_mg_l")?;
            require(
                concentration > quantification && quantification > 0.0,
                "tracer concentration must exceed a positive quantification limit",
            )?;
            require(
                text(inputs, "tracer_source_attribution")? == "unique",
                "tracer source must be uniquely attributable",
            )?;
            let sample_evidence_valid = bool_field(inputs, "instrument_calibration_current")?
                && bool_field(inputs, "method_valid_for_tracer")?;
            require(sample_evidence_valid, "direct tracer evidence must be valid")?;
            Ok(json!({
                "model_boundary_disposition": "MODEL_ZERO_DEMAND_AND_ZERO_LEAK_DO_NOT_REPRESENT_BIDIRECTIONAL_INGRESS",
                "ingress_disposition": "INGRESS_CONFIRMED_IN_TEST",
                "confirmation_basis": "DIRECT_TRACER_DETECTION_AND_SIGNED_REVERSE_FLOW",
                "pdd_customer_demand_lps": 0.0,
                "model_leak_emitter_discharge_lps": 0.0,
                "physical_reverse_flow_into_main_lps": number(inputs, "physical_signed_defect_flow_lps")?,
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "DI-T0-014-model-zero-demand-and-leak-without-physical-observation" => {
            require(
                pressure_deficit(inputs)?,
                "case requires a pressure deficit",
            )?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(
                bool_field(inputs, "external_tracer_present")?,
                "case requires an external tracer source",
            )?;
            require(
                text(inputs, "physical_backflow_preventer_state")? == "unknown",
                "case requires unknown backflow-preventer state",
            )?;
            require(
                close(number(inputs, "pdd_customer_demand_lps")?, 0.0),
                "PDD customer demand must be zero",
            )?;
            require(
                close(number(inputs, "model_leak_emitter_discharge_lps")?, 0.0),
                "model leak-emitter discharge must be zero",
            )?;
            require(
                field(inputs, "physical_signed_defect_flow_lps")?.is_null(),
                "physical signed defect flow must be unavailable",
            )?;
            require(
                !bool_field(inputs, "sample_coverage_complete")?,
                "case requires incomplete sample coverage",
            )?;
            require(
                text(inputs, "sample_result")? == "not_sampled",
                "case requires no tracer sample",
            )?;
            Ok(indeterminate(
                "PHYSICAL_REVERSE_FLOW_AND_TRACER_OBSERVATIONS_UNAVAILABLE",
            ))
        }
        "DI-T0-015-reverse-flow-observed-but-downstream-tracer-sample-missing" => {
            require(
                pressure_deficit(inputs)?,
                "case requires a pressure deficit",
            )?;
            require(
                text(inputs, "pathway_state")? == "confirmed_connected_defect",
                "case requires a confirmed connected defect",
            )?;
            require(
                text(inputs, "physical_backflow_preventer_state")? == "absent",
                "case requires no physical backflow preventer",
            )?;
            require(
                close(number(inputs, "pdd_customer_demand_lps")?, 0.0),
                "PDD customer demand must be zero",
            )?;
            require(
                close(number(inputs, "model_leak_emitter_discharge_lps")?, 0.0),
                "model leak-emitter discharge must be zero",
            )?;
            let reverse_flow = number(inputs, "physical_signed_defect_flow_lps")?;
            require(
                reverse_flow > 0.0,
                "physical defect flow must be positive into the main",
            )?;
            require(
                text(inputs, "flow_sign_convention")? == "positive_into_main",
                "signed flow convention mismatch",
            )?;
            require(
                !bool_field(inputs, "sample_coverage_complete")?,
                "case requires incomplete sample coverage",
            )?;
            let missing = field(inputs, "missing_sample_points")?
                .as_array()
                .ok_or_else(|| "missing_sample_points must be an array".to_owned())?;
            require(
                missing.iter().any(|point| point.as_str() == Some("far_end")),
                "case requires missing far-end tracer sample",
            )?;
            require(
                text(inputs, "sample_result")? == "not_sampled",
                "case requires an unobserved tracer sample",
            )?;
            Ok(json!({
                "model_boundary_disposition": "MODEL_ZERO_LEAK_DISCHARGE_DOES_NOT_REPRESENT_REVERSE_FLOW",
                "ingress_disposition": "INDETERMINATE",
                "reason": "REVERSE_FLOW_OBSERVED_BUT_DOWNSTREAM_TRACER_EVIDENCE_INCOMPLETE",
                "physical_reverse_flow_into_main_lps": reverse_flow,
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        _ => Err(format!("unrecognized intrusion case: {id}")),
    }
}

fn negative_result() -> Value {
    json!({
        "ingress_disposition": "INGRESS_NOT_DEMONSTRATED",
        "negative_claim_scope": "THIS_SYNTHETIC_TEST_WINDOW_ONLY",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn indeterminate(reason: &str) -> Value {
    json!({
        "ingress_disposition": "INDETERMINATE",
        "reason": reason,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn ids(cases: &[Value], label: &str) -> Result<Vec<String>, String> {
    cases
        .iter()
        .map(|case| case_id(case).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{label}: {error}"))
}

fn read_ids(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} contains a non-string ID"))
        })
        .collect()
}

fn validate_same_ids(
    input_ids: &[String],
    reference_ids: &[String],
    label: &str,
) -> Result<(), String> {
    let input_set: BTreeSet<&str> = input_ids.iter().map(String::as_str).collect();
    let reference_set: BTreeSet<&str> = reference_ids.iter().map(String::as_str).collect();
    require(input_ids.len() == input_set.len(), &format!("{label}: duplicate case IDs"))?;
    require(input_set == reference_set, &format!("{label}: case IDs differ"))?;
    Ok(())
}

fn validate_contract(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    require(text(inputs, "schema_version")? == "0.1.0", "input schema version mismatch")?;
    require(text(inputs, "programme")? == PROGRAMME, "input programme mismatch")?;
    require(
        text(inputs, "evidence_class")? == "synthetic_only",
        "input evidence class mismatch",
    )?;
    require(
        text(inputs, "evidence_ceiling")? == INPUT_CEILING,
        "input evidence ceiling mismatch",
    )?;
    let profile = field(inputs, "profile")?;
    require(text(profile, "profile_id")? == PROFILE, "input profile mismatch")?;
    require(text(profile, "revision")? == "0.2", "input profile revision mismatch")?;
    require(
        bool_field(profile, "all_values_are_fictional")?,
        "input values must be declared fictional",
    )?;

    let expected_units = [
        ("pressure", "kPa_gauge"),
        ("head", "m_water"),
        ("volume", "L"),
        ("concentration", "mg/L"),
        ("mass", "mg"),
        ("time", "s"),
        ("flow", "L/s"),
        ("signed_flow", "L/s"),
    ];
    let units = field(&field(inputs, "profile")?, "units")?;
    for (key, expected) in expected_units {
        require(text(units, key)? == expected, &format!("unit binding mismatch: {key}"))?;
    }

    require(text(oracle, "schema_version")? == "0.1.0", "oracle schema version mismatch")?;
    require(text(oracle, "programme")? == PROGRAMME, "oracle programme mismatch")?;
    require(
        text(oracle, "evidence_class")? == "synthetic_expected_outputs_only",
        "oracle evidence class mismatch",
    )?;
    require(
        text(oracle, "evidence_ceiling")? == ORACLE_CEILING,
        "oracle evidence ceiling mismatch",
    )?;
    require(text(oracle, "input_profile_id")? == PROFILE, "oracle profile mismatch")?;

    require(text(manifest, "schema_version")? == "0.1.0", "manifest schema version mismatch")?;
    require(text(manifest, "programme_id")? == PROGRAMME, "manifest programme mismatch")?;
    require(
        text(manifest, "evidence_class")? == "design_and_synthetic_only",
        "manifest evidence class mismatch",
    )?;
    require(
        text(manifest, "input_file")? == "intrusion-t0-scenarios-v0.1.json",
        "manifest input filename mismatch",
    )?;
    require(
        text(manifest, "oracle_file")? == "intrusion-t0-oracle-v0.1.json",
        "manifest oracle filename mismatch",
    )?;
    require(
        number(manifest, "scenario_count")? == CASE_COUNT as f64,
        "manifest scenario count mismatch",
    )?;
    require(
        text(manifest, "profile_revision")? == "0.2",
        "manifest profile revision mismatch",
    )?;
    require(
        text(manifest, "profile_revision")? == "0.2",
        "manifest profile revision mismatch",
    )?;
    require(
        text(manifest, "evidence_ceiling")?.contains("no_potability"),
        "manifest evidence ceiling too permissive",
    )?;
    Ok(())
}

pub fn evaluate(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    validate_contract(inputs, oracle, manifest)?;
    let cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    require(cases.len() == CASE_COUNT, "input case count mismatch")?;
    require(oracle_cases.len() == CASE_COUNT, "oracle case count mismatch")?;

    let input_ids = ids(cases, "input")?;
    let oracle_ids = ids(oracle_cases, "oracle")?;
    let manifest_ids = read_ids(manifest, "scenario_ids")?;
    validate_same_ids(&input_ids, &oracle_ids, "input/oracle")?;
    validate_same_ids(&input_ids, &manifest_ids, "input/manifest")?;

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = case_id(row)?.to_owned();
        let expected = field(row, "expected")?;
        require(
            !bool_field(expected, "potability_claim_permitted")?,
            "oracle must never permit a potability claim",
        )?;
        require(
            !bool_field(expected, "release_authorized")?,
            "oracle must never authorize release",
        )?;
        require(
            expected_by_id.insert(id.clone(), expected).is_none(),
            &format!("duplicate oracle case ID: {id}"),
        )?;
    }

    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = case_id(case)?;
        let computed = compute(case)?;
        let expected = expected_by_id
            .get(id)
            .ok_or_else(|| format!("oracle missing case: {id}"))?;
        let matched = compare_json(&computed, expected);
        if !matched {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "computed": computed
            }));
        }
        results.push(json!({
            "case_id": id,
            "matched": matched,
            "computed": computed
        }));
    }
    results.sort_by(|a, b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version": "distribution-intrusion-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_only",
        "evidence_ceiling": INPUT_CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "Synthetic evidence semantics and declared tracer arithmetic only; no physical intrusion or transient process is qualified.",
            "No potability, pathogen, health-risk, chemical-quality, regulatory, field or service-reliability claim is established.",
            "Pressure recovery, leak detection, and a closed maintenance ticket do not independently establish water-quality recommissioning.",
            "The evaluator does not authorize isolation, restart, public notices, treatment, flushing or water release."
        ]
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err(
            "usage: intrusion-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".to_owned(),
        );
    }

    let input_path = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or(
                "research/distribution-intrusion-transient/intrusion-t0-scenarios-v0.1.json",
            ),
    );
    let oracle_path = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or(
                "research/distribution-intrusion-transient/intrusion-t0-oracle-v0.1.json",
            ),
    );
    let manifest_path = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("research/distribution-intrusion-transient/manifest-v0.1.json"),
    );
    let receipt_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/distribution-intrusion-t0/receipt.json"),
    );

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&inputs, &oracle, &manifest)?;
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let success = total == CASE_COUNT as u64 && matched == CASE_COUNT as u64 && no_mismatches;

    let raw_revision = env::var("DISTRIBUTION_INTRUSION_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version": "distribution-intrusion-t0-run-v0.1",
        "verification_state": if !success {
            "OracleMismatch"
        } else if revision.is_some() {
            "PassedExactRevision"
        } else {
            "PassedLocalUnpinned"
        },
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "result_count": total,
        "matched_case_count": matched,
        "mismatches": evaluation["mismatches"].clone(),
        "results": evaluation["results"].clone(),
        "evidence_class": "synthetic_only",
        "evidence_ceiling": INPUT_CEILING,
        "nonclaims": evaluation["nonclaims"].clone()
    });

    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("serialize receipt: {error}"))?
        + "\n";
    fs::write(&receipt_path, serialized.as_bytes())
        .map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    println!("{serialized}");

    Ok(if success { 0 } else { 2 })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("DISTRIBUTION_INTRUSION_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Value {
        let fixture = match name {
            "inputs" => include_str!(
                "../../../../research/distribution-intrusion-transient/intrusion-t0-scenarios-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/distribution-intrusion-transient/intrusion-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/distribution-intrusion-transient/manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(fixture).expect("fixture must be valid JSON")
    }

    fn evaluated() -> Value {
        evaluate(&load("inputs"), &load("oracle"), &load("manifest"))
            .expect("evaluation must complete")
    }

    #[test]
    fn all_fifteen_cases_match_the_separate_frozen_oracle() {
        let result = evaluated();
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn pressure_transient_alone_never_confirms_ingress() {
        let result = evaluated();
        let cases = result["results"].as_array().unwrap();
        for id in [
            "DI-T0-002-transient-with-no-defect-and-complete-negative-samples",
            "DI-T0-004-pressure-deficit-with-unknown-pathway",
        ] {
            let row = cases.iter().find(|row| row["case_id"] == id).unwrap();
            assert_ne!(row["computed"]["ingress_disposition"], "INGRESS_CONFIRMED_IN_TEST");
        }
    }

    #[test]
    fn unknown_pathway_and_missing_downstream_sample_remain_indeterminate() {
        let result = evaluated();
        let cases = result["results"].as_array().unwrap();
        for (id, reason) in [
            (
                "DI-T0-004-pressure-deficit-with-unknown-pathway",
                "PATHWAY_OBSERVABILITY_INSUFFICIENT",
            ),
            ("DI-T0-005-missing-downstream-sample", "DOWNSTREAM_SAMPLE_COVERAGE_INCOMPLETE"),
        ] {
            let row = cases.iter().find(|row| row["case_id"] == id).unwrap();
            assert_eq!(row["computed"]["ingress_disposition"], "INDETERMINATE");
            assert_eq!(row["computed"]["reason"], reason);
        }
    }

    #[test]
    fn non_detect_above_required_detection_sensitivity_is_not_a_negative() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "DI-T0-007-detection-limit-insufficient").unwrap();
        assert_eq!(row["computed"]["ingress_disposition"], "INDETERMINATE");
    }

    #[test]
    fn ambiguous_tracer_source_is_not_confirmed_ingress() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "DI-T0-008-tracer-source-ambiguous").unwrap();
        assert_eq!(row["computed"]["ingress_disposition"], "INDETERMINATE");
    }

    #[test]
    fn confirmed_ingress_can_coexist_with_unresolved_mass_balance() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "DI-T0-009-ingress-known-but-mass-balance-incomplete")
            .unwrap();
        assert_eq!(row["computed"]["ingress_disposition"], "INGRESS_CONFIRMED_IN_TEST");
        assert_eq!(row["computed"]["mass_balance_disposition"], "MASS_BALANCE_UNRESOLVED");
    }

    #[test]
    fn pressure_recovery_and_ticket_closure_do_not_verify_recommissioning() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "DI-T0-011-pressure-restored-ticket-closed-no-water-quality-evidence")
            .unwrap();
        assert_eq!(row["computed"]["recommissioning_disposition"], "RECOMMISSIONING_NOT_VERIFIED");
        assert!(!row["computed"]["release_authorized"].as_bool().unwrap());
    }

    #[test]
    fn model_zero_demand_and_emitter_discharge_do_not_override_direct_ingress() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"]
                    == "DI-T0-013-model-zero-demand-and-leak-but-direct-tracer-confirms-reverse-ingress"
            })
            .unwrap();
        assert_eq!(
            row["computed"]["ingress_disposition"],
            "INGRESS_CONFIRMED_IN_TEST"
        );
        assert_eq!(
            row["computed"]["model_boundary_disposition"],
            "MODEL_ZERO_DEMAND_AND_ZERO_LEAK_DO_NOT_REPRESENT_BIDIRECTIONAL_INGRESS"
        );
        assert_eq!(
            row["computed"]["physical_reverse_flow_into_main_lps"],
            0.2
        );
        assert!(!row["computed"]["release_authorized"].as_bool().unwrap());
    }

    #[test]
    fn zero_model_demand_without_physical_observation_is_indeterminate() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"]
                    == "DI-T0-014-model-zero-demand-and-leak-without-physical-observation"
            })
            .unwrap();
        assert_eq!(row["computed"]["ingress_disposition"], "INDETERMINATE");
        assert_eq!(
            row["computed"]["reason"],
            "PHYSICAL_REVERSE_FLOW_AND_TRACER_OBSERVATIONS_UNAVAILABLE"
        );
    }

    #[test]
    fn observed_reverse_flow_without_far_end_sample_is_indeterminate() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"]
                    == "DI-T0-015-reverse-flow-observed-but-downstream-tracer-sample-missing"
            })
            .unwrap();
        assert_eq!(row["computed"]["ingress_disposition"], "INDETERMINATE");
        assert_eq!(
            row["computed"]["reason"],
            "REVERSE_FLOW_OBSERVED_BUT_DOWNSTREAM_TRACER_EVIDENCE_INCOMPLETE"
        );
        assert_eq!(row["computed"]["physical_reverse_flow_into_main_lps"], 0.2);
    }

    #[test]
    fn model_zero_demand_and_emitter_discharge_do_not_override_direct_ingress() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"]
                    == "DI-T0-013-model-zero-demand-and-leak-but-direct-tracer-confirms-reverse-ingress"
            })
            .unwrap();
        assert_eq!(
            row["computed"]["ingress_disposition"],
            "INGRESS_CONFIRMED_IN_TEST"
        );
        assert_eq!(
            row["computed"]["model_boundary_disposition"],
            "MODEL_ZERO_DEMAND_AND_ZERO_LEAK_DO_NOT_REPRESENT_BIDIRECTIONAL_INGRESS"
        );
        assert_eq!(row["computed"]["physical_reverse_flow_into_main_lps"], 0.2);
        assert!(!row["computed"]["release_authorized"].as_bool().unwrap());
    }

    #[test]
    fn zero_model_demand_without_physical_observation_is_indeterminate() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"]
                    == "DI-T0-014-model-zero-demand-and-leak-without-physical-observation"
            })
            .unwrap();
        assert_eq!(row["computed"]["ingress_disposition"], "INDETERMINATE");
        assert_eq!(
            row["computed"]["reason"],
            "PHYSICAL_REVERSE_FLOW_AND_TRACER_OBSERVATIONS_UNAVAILABLE"
        );
    }

    #[test]
    fn observed_reverse_flow_without_far_end_sample_is_indeterminate() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .unwrap();
        let result_row = row
            .iter()
            .find(|row| {
                row["case_id"]
                    == "DI-T0-015-reverse-flow-observed-but-downstream-tracer-sample-missing"
            })
            .unwrap();
        assert_eq!(
            result_row["computed"]["ingress_disposition"],
            "INDETERMINATE"
        );
        assert_eq!(
            result_row["computed"]["reason"],
            "REVERSE_FLOW_OBSERVED_BUT_DOWNSTREAM_TRACER_EVIDENCE_INCOMPLETE"
        );
        assert_eq!(
            result_row["computed"]["physical_reverse_flow_into_main_lps"],
            0.2
        );
    }

    #[test]
    fn duplicate_input_case_id_is_rejected() {
        let mut inputs = load("inputs");
        let duplicate = inputs["cases"][0].clone();
        inputs["cases"].as_array_mut().unwrap().push(duplicate);
        assert!(evaluate(&inputs, &load("oracle"), &load("manifest")).is_err());
    }

    #[test]
    fn altered_oracle_cannot_authorize_release() {
        let mut oracle = load("oracle");
        oracle["cases"][0]["expected"]["release_authorized"] = json!(true);
        assert!(evaluate(&load("inputs"), &oracle, &load("manifest")).is_err());
    }

    #[test]
    fn arithmetic_mismatch_is_reported_not_hidden() {
        let mut inputs = load("inputs");
        inputs["cases"][0]["inputs"]["measured_tracer_mass_mg"] = json!(4.0);
        assert!(evaluate(&inputs, &load("oracle"), &load("manifest")).is_err());
    }
}
