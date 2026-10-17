use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const SCHEMA_VERSION: &str = "0.1.0";
const MIN_VALID_EPOCH: i64 = 100;
const MAX_VALID_EPOCH: i64 = 120;
const MIN_TEMPERATURE_MILLIC: i64 = -40_000;
const MAX_TEMPERATURE_MILLIC: i64 = 40_000;
const MIN_INPUT_MILLI: i64 = 0;
const MAX_INPUT_MILLI: i64 = 10_000;
const DRIFT_THRESHOLD_SIGMA_MILLI: i64 = 3_000;
const DRIFT_CONSECUTIVE_LIMIT: usize = 2;
const SYNTHETIC_RECALIBRATION_COST: i64 = 7;
const SYNTHETIC_LOSS_PER_SHIFTED_SAMPLE: i64 = 4;

const STANDARD_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MODEL_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Clone, Copy, Debug)]
struct CalibrationEnvelope {
    instrument_id: &'static str,
    instrument_revision: &'static str,
    standard_id: &'static str,
    standard_sha256: &'static str,
    site_id: &'static str,
    body_id: &'static str,
    patch_id: &'static str,
    valid_from_epoch: i64,
    valid_through_epoch: i64,
    min_temperature_millic: i64,
    max_temperature_millic: i64,
    power_mode: &'static str,
    min_input_milli: i64,
    max_input_milli: i64,
    sample_prep: &'static str,
    model_sha256: &'static str,
}

const ENVELOPE: CalibrationEnvelope = CalibrationEnvelope {
    instrument_id: "spectrometer-01",
    instrument_revision: "rev-c",
    standard_id: "reference-standard-42",
    standard_sha256: STANDARD_SHA256,
    site_id: "ridge-07",
    body_id: "moon",
    patch_id: "ridge-07-patch-a",
    valid_from_epoch: MIN_VALID_EPOCH,
    valid_through_epoch: MAX_VALID_EPOCH,
    min_temperature_millic: MIN_TEMPERATURE_MILLIC,
    max_temperature_millic: MAX_TEMPERATURE_MILLIC,
    power_mode: "nominal",
    min_input_milli: MIN_INPUT_MILLI,
    max_input_milli: MAX_INPUT_MILLI,
    sample_prep: "dry-sieved-v1",
    model_sha256: MODEL_SHA256,
};

#[derive(Clone, Copy, Debug)]
struct ObservationContext {
    scenario_id: &'static str,
    instrument_id: Option<&'static str>,
    instrument_revision: Option<&'static str>,
    standard_id: Option<&'static str>,
    standard_sha256: Option<&'static str>,
    standard_revoked: bool,
    site_id: Option<&'static str>,
    body_id: Option<&'static str>,
    patch_id: Option<&'static str>,
    epoch: i64,
    temperature_millic: i64,
    power_mode: Option<&'static str>,
    input_milli: i64,
    sample_prep: Option<&'static str>,
    model_sha256: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
struct Scenario {
    context: ObservationContext,
    expected_reason: Option<&'static str>,
}

fn in_domain_context(scenario_id: &'static str) -> ObservationContext {
    ObservationContext {
        scenario_id,
        instrument_id: Some(ENVELOPE.instrument_id),
        instrument_revision: Some(ENVELOPE.instrument_revision),
        standard_id: Some(ENVELOPE.standard_id),
        standard_sha256: Some(ENVELOPE.standard_sha256),
        standard_revoked: false,
        site_id: Some(ENVELOPE.site_id),
        body_id: Some(ENVELOPE.body_id),
        patch_id: Some(ENVELOPE.patch_id),
        epoch: 110,
        temperature_millic: 10_000,
        power_mode: Some(ENVELOPE.power_mode),
        input_milli: 4_000,
        sample_prep: Some(ENVELOPE.sample_prep),
        model_sha256: Some(ENVELOPE.model_sha256),
    }
}

fn scenarios() -> Vec<Scenario> {
    let mut values = Vec::new();
    values.push(Scenario { context: in_domain_context("in_domain_control"), expected_reason: None });

    let mut context = in_domain_context("temperature_shift");
    context.temperature_millic = 52_000;
    values.push(Scenario { context, expected_reason: Some("TEMPERATURE_OUT_OF_ENVELOPE") });

    let mut context = in_domain_context("expired_epoch");
    context.epoch = 121;
    values.push(Scenario { context, expected_reason: Some("EPOCH_OUT_OF_ENVELOPE") });

    let mut context = in_domain_context("revoked_reference_standard");
    context.standard_revoked = true;
    values.push(Scenario { context, expected_reason: Some("REFERENCE_STANDARD_REVOKED") });

    let mut context = in_domain_context("unseen_site");
    context.site_id = Some("crater-03");
    values.push(Scenario { context, expected_reason: Some("SITE_OUT_OF_ENVELOPE") });

    let mut context = in_domain_context("instrument_identity_change");
    context.instrument_id = Some("spectrometer-02");
    values.push(Scenario { context, expected_reason: Some("INSTRUMENT_ID_MISMATCH") });

    let mut context = in_domain_context("instrument_revision_change");
    context.instrument_revision = Some("rev-d");
    values.push(Scenario { context, expected_reason: Some("INSTRUMENT_REVISION_MISMATCH") });

    let mut context = in_domain_context("calibration_model_hash_mismatch");
    context.model_sha256 = Some("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    values.push(Scenario { context, expected_reason: Some("CALIBRATION_MODEL_HASH_MISMATCH") });

    let mut context = in_domain_context("missing_site_metadata");
    context.site_id = None;
    values.push(Scenario { context, expected_reason: Some("REQUIRED_METADATA_MISSING") });

    let mut context = in_domain_context("degraded_power_mode");
    context.power_mode = Some("battery-saver");
    values.push(Scenario { context, expected_reason: Some("POWER_MODE_MISMATCH") });

    let mut context = in_domain_context("input_out_of_range");
    context.input_milli = 12_000;
    values.push(Scenario { context, expected_reason: Some("INPUT_OUT_OF_ENVELOPE") });

    let mut context = in_domain_context("sample_preparation_changed");
    context.sample_prep = Some("wet-washed-v2");
    values.push(Scenario { context, expected_reason: Some("SAMPLE_PREPARATION_MISMATCH") });

    let mut context = in_domain_context("reference_standard_hash_mismatch");
    context.standard_sha256 = Some("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    values.push(Scenario { context, expected_reason: Some("REFERENCE_STANDARD_MISMATCH") });

    let mut context = in_domain_context("reference_standard_lineage_changed");
    context.standard_id = Some("reference-standard-99");
    context.standard_sha256 = Some("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
    values.push(Scenario { context, expected_reason: Some("REFERENCE_STANDARD_MISMATCH") });

    let mut context = in_domain_context("body_mismatch");
    context.body_id = Some("mars");
    values.push(Scenario { context, expected_reason: Some("BODY_OUT_OF_ENVELOPE") });

    let mut context = in_domain_context("patch_mismatch");
    context.patch_id = Some("ridge-07-patch-b");
    values.push(Scenario { context, expected_reason: Some("PATCH_OUT_OF_ENVELOPE") });

    values
}

fn required_metadata_present(context: &ObservationContext) -> bool {
    context.instrument_id.is_some()
        && context.instrument_revision.is_some()
        && context.standard_id.is_some()
        && context.standard_sha256.is_some()
        && context.site_id.is_some()
        && context.body_id.is_some()
        && context.patch_id.is_some()
        && context.power_mode.is_some()
        && context.sample_prep.is_some()
        && context.model_sha256.is_some()
}

fn validate_observation(context: &ObservationContext) -> Result<(), &'static str> {
    if !required_metadata_present(context) {
        return Err("REQUIRED_METADATA_MISSING");
    }
    if context.instrument_id != Some(ENVELOPE.instrument_id) {
        return Err("INSTRUMENT_ID_MISMATCH");
    }
    if context.instrument_revision != Some(ENVELOPE.instrument_revision) {
        return Err("INSTRUMENT_REVISION_MISMATCH");
    }
    if context.standard_revoked {
        return Err("REFERENCE_STANDARD_REVOKED");
    }
    if context.standard_id != Some(ENVELOPE.standard_id)
        || context.standard_sha256 != Some(ENVELOPE.standard_sha256)
    {
        return Err("REFERENCE_STANDARD_MISMATCH");
    }
    if context.model_sha256 != Some(ENVELOPE.model_sha256) {
        return Err("CALIBRATION_MODEL_HASH_MISMATCH");
    }
    if context.body_id != Some(ENVELOPE.body_id) {
        return Err("BODY_OUT_OF_ENVELOPE");
    }
    if context.site_id != Some(ENVELOPE.site_id) {
        return Err("SITE_OUT_OF_ENVELOPE");
    }
    if context.patch_id != Some(ENVELOPE.patch_id) {
        return Err("PATCH_OUT_OF_ENVELOPE");
    }
    if !(ENVELOPE.valid_from_epoch..=ENVELOPE.valid_through_epoch).contains(&context.epoch) {
        return Err("EPOCH_OUT_OF_ENVELOPE");
    }
    if !(ENVELOPE.min_temperature_millic..=ENVELOPE.max_temperature_millic)
        .contains(&context.temperature_millic)
    {
        return Err("TEMPERATURE_OUT_OF_ENVELOPE");
    }
    if context.power_mode != Some(ENVELOPE.power_mode) {
        return Err("POWER_MODE_MISMATCH");
    }
    if !(ENVELOPE.min_input_milli..=ENVELOPE.max_input_milli).contains(&context.input_milli) {
        return Err("INPUT_OUT_OF_ENVELOPE");
    }
    if context.sample_prep != Some(ENVELOPE.sample_prep) {
        return Err("SAMPLE_PREPARATION_MISMATCH");
    }
    Ok(())
}

fn validate_source_commit(source_commit: &str) -> Result<(), String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn drift_detection_index(residual_sigma_milli: &[i64]) -> Option<usize> {
    let mut consecutive = 0_usize;
    for (index, residual) in residual_sigma_milli.iter().enumerate() {
        if residual.abs() >= DRIFT_THRESHOLD_SIGMA_MILLI {
            consecutive += 1;
            if consecutive >= DRIFT_CONSECUTIVE_LIMIT {
                return Some(index);
            }
        } else {
            consecutive = 0;
        }
    }
    None
}

fn partition_integrity() -> Value {
    // Synthetic grouped and time-ordered split. Site groups are disjoint;
    // no test observations participate in fit/calibration decisions.
    let partitions = [
        ("training", "ridge-01", 80_i64, 89_i64),
        ("calibration", "ridge-07", 90_i64, 99_i64),
        ("validation", "ridge-03", 100_i64, 109_i64),
        ("test", "crater-09", 110_i64, 119_i64),
    ];
    let mut site_groups = Vec::new();
    let mut temporal_order_pass = true;
    let mut no_duplicate_site_groups = true;
    for (index, (name, site, epoch_min, epoch_max)) in partitions.iter().enumerate() {
        if epoch_min > epoch_max {
            temporal_order_pass = false;
        }
        if site_groups.contains(site) {
            no_duplicate_site_groups = false;
        }
        site_groups.push(*site);
        if index > 0 && partitions[index - 1].3 >= *epoch_min {
            temporal_order_pass = false;
        }
        let _ = name;
    }
    json!({
        "grouped_by": "site_id",
        "time_ordered": true,
        "partitions": partitions.iter().map(|(name, site, min_epoch, max_epoch)| json!({
            "name": name,
            "site_group": site,
            "epoch_min": min_epoch,
            "epoch_max": max_epoch
        })).collect::<Vec<_>>(),
        "site_groups_disjoint": no_duplicate_site_groups,
        "temporal_order_pass": temporal_order_pass,
        "test_data_used_for_fit_or_calibration": false,
        "integrity_pass": no_duplicate_site_groups && temporal_order_pass
    })
}

fn envelope_json() -> Value {
    json!({
        "instrument_id": ENVELOPE.instrument_id,
        "instrument_revision": ENVELOPE.instrument_revision,
        "reference_standard": {
            "standard_id": ENVELOPE.standard_id,
            "standard_sha256": ENVELOPE.standard_sha256,
            "revocation_checked_at_decision_time": true
        },
        "site_id": ENVELOPE.site_id,
        "body_id": ENVELOPE.body_id,
        "patch_id": ENVELOPE.patch_id,
        "valid_epoch_inclusive": [ENVELOPE.valid_from_epoch, ENVELOPE.valid_through_epoch],
        "temperature_millic_inclusive": [ENVELOPE.min_temperature_millic, ENVELOPE.max_temperature_millic],
        "power_mode": ENVELOPE.power_mode,
        "input_milli_inclusive": [ENVELOPE.min_input_milli, ENVELOPE.max_input_milli],
        "sample_prep": ENVELOPE.sample_prep,
        "calibration_model_sha256": ENVELOPE.model_sha256
    })
}

fn execute(source_commit: &str) -> Result<Value, String> {
    validate_source_commit(source_commit)?;
    let fixture_scenarios = scenarios();
    let mut scenario_reports = Vec::new();
    let mut out_of_domain_count = 0_i64;
    let mut naive_false_valid_count = 0_i64;
    let mut gate_false_valid_count = 0_i64;
    let mut gate_false_rejection_count = 0_i64;
    let mut reason_codes_match = true;
    let mut naive_accepted_ids = Vec::new();
    let mut gate_accepted_ids = Vec::new();
    let mut negative_controls = Vec::new();

    for scenario in &fixture_scenarios {
        let validation = validate_observation(&scenario.context);
        let accepted = validation.is_ok();
        let rejection_reason = validation.err();
        let expected_in_domain = scenario.expected_reason.is_none();
        if !expected_in_domain {
            out_of_domain_count += 1;
        }
        if accepted && !expected_in_domain {
            gate_false_valid_count += 1;
        }
        if !accepted && expected_in_domain {
            gate_false_rejection_count += 1;
        }
        if rejection_reason != scenario.expected_reason {
            reason_codes_match = false;
        }
        // The naïve policy ignores every envelope and revocation field.
        let naive_accepts = true;
        if naive_accepts && !expected_in_domain {
            naive_false_valid_count += 1;
        }
        if naive_accepts {
            naive_accepted_ids.push(scenario.context.scenario_id);
        }
        if accepted {
            gate_accepted_ids.push(scenario.context.scenario_id);
        }
        if !expected_in_domain {
            negative_controls.push(json!({
                "scenario_id": scenario.context.scenario_id,
                "expected_rejection_reason": scenario.expected_reason,
                "actual_rejection_reason": rejection_reason,
                "evidence_credit_under_naive_policy": 1,
                "evidence_credit_under_envelope_gate": 0,
                "negative_control_pass": !accepted && rejection_reason == scenario.expected_reason
            }));
        }
        scenario_reports.push(json!({
            "scenario_id": scenario.context.scenario_id,
            "expected_in_domain": expected_in_domain,
            "envelope_validation": {
                "accepted": accepted,
                "rejection_reason": rejection_reason
            },
            "policies": {
                "naive_always_apply": {
                    "accepts_likelihood_update": naive_accepts,
                    "credited_evidence_units": 1
                },
                "fail_closed_envelope_gate": {
                    "accepts_likelihood_update": accepted,
                    "credited_evidence_units": if accepted { 1 } else { 0 },
                    "rejection_reason": rejection_reason
                }
            }
        }));
    }

    let noise_residuals = [0_i64, 1_000, 2_000, 1_000, 0, 4_000, 1_000, 0];
    let shifted_residuals = [0_i64, 1_000, 1_000, 2_000, 4_000, 4_000, 5_000, 5_000];
    let noise_alarm = drift_detection_index(&noise_residuals);
    let drift_detection = drift_detection_index(&shifted_residuals);
    let drift_onset_index = 4_usize;
    let detection_delay = drift_detection.map(|index| index.saturating_sub(drift_onset_index));
    let shifted_samples = shifted_residuals.len().saturating_sub(drift_onset_index) as i64;
    let pre_detection_shifted_samples = drift_detection
        .map(|index| index.saturating_sub(drift_onset_index) as i64)
        .unwrap_or(shifted_samples);
    let loss_without_detection = shifted_samples * SYNTHETIC_LOSS_PER_SHIFTED_SAMPLE;
    let loss_with_detection = pre_detection_shifted_samples * SYNTHETIC_LOSS_PER_SHIFTED_SAMPLE;
    let avoided_regret = loss_without_detection - loss_with_detection;
    let net_value_of_recalibration = avoided_regret - SYNTHETIC_RECALIBRATION_COST;

    let partitions = partition_integrity();
    let expected_negative_count = fixture_scenarios.len() as i64 - 1;
    if fixture_scenarios.len() != 16
        || out_of_domain_count != expected_negative_count
        || naive_false_valid_count != expected_negative_count
        || gate_false_valid_count != 0
        || gate_false_rejection_count != 0
        || !reason_codes_match
        || negative_controls.len() != expected_negative_count as usize
        || negative_controls.iter().any(|control| control["negative_control_pass"] != true)
        || noise_alarm.is_some()
        || drift_detection != Some(5)
        || detection_delay != Some(1)
        || partitions["integrity_pass"] != true
    {
        return Err("calibration-transfer shift reference oracle mismatch".to_owned());
    }

    let envelope_value = envelope_json();
    let envelope_bytes = serde_json::to_vec(&envelope_value).map_err(|error| error.to_string())?;
    let model_spec = json!({
        "schema_version": SCHEMA_VERSION,
        "fixture_type": "synthetic_calibration_transfer_shift",
        "drift_threshold_sigma_milli": DRIFT_THRESHOLD_SIGMA_MILLI,
        "drift_consecutive_limit": DRIFT_CONSECUTIVE_LIMIT,
        "noise_residuals_sigma_milli": noise_residuals,
        "shifted_residuals_sigma_milli": shifted_residuals,
        "synthetic_recalibration_cost_units": SYNTHETIC_RECALIBRATION_COST,
        "synthetic_loss_per_shifted_sample_units": SYNTHETIC_LOSS_PER_SHIFTED_SAMPLE
    });
    let model_bytes = serde_json::to_vec(&model_spec).map_err(|error| error.to_string())?;
    let source_sha = sha256_hex(include_str!("calibration_transfer_shift_reference.rs").as_bytes());

    Ok(json!({
        "schema_version": SCHEMA_VERSION,
        "result": "pass",
        "source_commit": source_commit,
        "reference_source_sha256": source_sha,
        "calibration_envelope_sha256": sha256_hex(&envelope_bytes),
        "drift_model_spec_sha256": sha256_hex(&model_bytes),
        "fixture_type": "synthetic_calibration_transfer_shift",
        "evidence_boundary": "Synthetic contract validation only; no physical calibration stability, lunar compatibility, or operational performance is established.",
        "calibration_envelope": envelope_value,
        "summary": {
            "scenario_count": fixture_scenarios.len(),
            "out_of_domain_negative_control_count": out_of_domain_count,
            "naive_false_valid_claim_count": naive_false_valid_count,
            "envelope_gate_false_valid_claim_count": gate_false_valid_count,
            "envelope_gate_false_rejection_count": gate_false_rejection_count,
            "all_rejection_reason_codes_match": reason_codes_match,
            "zero_false_valid_on_declared_out_of_domain_controls": gate_false_valid_count == 0,
            "zero_in_domain_false_rejections": gate_false_rejection_count == 0
        },
        "policies": {
            "naive_always_apply": {
                "policy_id": "naive_always_apply",
                "accepted_observation_ids": naive_accepted_ids,
                "false_valid_claim_count": naive_false_valid_count,
                "selection_authority": "negative_control_only"
            },
            "fail_closed_envelope_gate": {
                "policy_id": "fail_closed_envelope_gate",
                "accepted_observation_ids": gate_accepted_ids,
                "false_valid_claim_count": gate_false_valid_count,
                "false_rejection_count": gate_false_rejection_count,
                "selection_authority": "eligible_only_when_all_envelope_fields_match"
            }
        },
        "scenarios": scenario_reports,
        "negative_controls": negative_controls,
        "drift_monitor": {
            "method": "two_consecutive_absolute_residuals_at_or_above_threshold",
            "threshold_sigma_milli": DRIFT_THRESHOLD_SIGMA_MILLI,
            "required_consecutive_threshold_exceedances": DRIFT_CONSECUTIVE_LIMIT,
            "noise_only_residuals_sigma_milli": noise_residuals,
            "noise_only_false_alarm": noise_alarm.is_some(),
            "shifted_residuals_sigma_milli": shifted_residuals,
            "declared_shift_onset_index_zero_based": drift_onset_index,
            "detection_index_zero_based": drift_detection,
            "detection_delay_samples": detection_delay,
            "shifted_samples_before_detection": pre_detection_shifted_samples,
            "synthetic_loss_without_detection_units": loss_without_detection,
            "synthetic_loss_with_detection_units": loss_with_detection,
            "synthetic_regret_avoided_units": avoided_regret,
            "synthetic_recalibration_cost_units": SYNTHETIC_RECALIBRATION_COST,
            "synthetic_net_value_of_recalibration_units": net_value_of_recalibration,
            "recommended_fixture_action": if net_value_of_recalibration > 0 { "pause_and_recalibrate" } else { "continue_monitoring" },
            "interpretation": "The first shifted sample precedes the sequential alarm; detection latency is reported rather than hidden. The statistical monitor complements but does not replace metadata envelope gates."
        },
        "partition_integrity": partitions,
        "acceptance_targets": {
            "false_valid_claims_on_declared_out_of_domain_controls": 0,
            "lookahead_or_partition_leakage": 0,
            "noise_only_false_alarms": 0,
            "detection_delay_reported_alongside_false_alarm_rate": true,
            "unverified_or_expired_standard_evidence_credit": 0,
            "report_recalibration_cost_and_regret_avoided_together": true,
            "require_held_out_calibration_coverage_before_real_world_thresholds": true
        },
        "model_specification": model_spec
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut source_commit = None;
    let mut index = 1;
    while index < args.len() {
        if args[index] == "--source-commit" && index + 1 < args.len() {
            source_commit = Some(args[index + 1].as_str());
            index += 2;
        } else {
            eprintln!("unknown or incomplete argument: {}", args[index]);
            return ExitCode::from(2);
        }
    }
    let Some(source_commit) = source_commit else {
        eprintln!("missing required --source-commit <40-character-lowercase-git-sha1>");
        return ExitCode::from(2);
    };
    match execute(source_commit) {
        Ok(receipt) => {
            println!("{}", serde_json::to_string_pretty(&receipt).expect("JSON serialization should succeed"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("calibration transfer shift reference failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_domain_observation_is_accepted() {
        assert_eq!(validate_observation(&in_domain_context("valid")), Ok(()));
    }

    #[test]
    fn every_declared_negative_control_fails_closed_with_expected_reason() {
        for scenario in scenarios().iter().filter(|scenario| scenario.expected_reason.is_some()) {
            assert_eq!(
                validate_observation(&scenario.context),
                Err(scenario.expected_reason.unwrap()),
                "scenario {}",
                scenario.context.scenario_id
            );
        }
    }

    #[test]
    fn expired_epoch_is_not_authorized_by_instrument_identity_alone() {
        let mut context = in_domain_context("expired");
        context.epoch = MAX_VALID_EPOCH + 1;
        assert_eq!(validate_observation(&context), Err("EPOCH_OUT_OF_ENVELOPE"));
    }

    #[test]
    fn revoked_standard_is_rejected_even_when_hash_matches() {
        let mut context = in_domain_context("revoked");
        context.standard_revoked = true;
        assert_eq!(validate_observation(&context), Err("REFERENCE_STANDARD_REVOKED"));
    }

    #[test]
    fn single_spike_does_not_trigger_sequential_drift_alarm() {
        let residuals = [0_i64, 1_000, 2_000, 1_000, 0, 4_000, 1_000, 0];
        assert_eq!(drift_detection_index(&residuals), None);
    }

    #[test]
    fn sustained_shift_triggers_and_reports_one_sample_detection_delay() {
        let residuals = [0_i64, 1_000, 1_000, 2_000, 4_000, 4_000, 5_000, 5_000];
        assert_eq!(drift_detection_index(&residuals), Some(5));
        assert_eq!(5_usize.saturating_sub(4), 1);
    }

    #[test]
    fn split_groups_are_disjoint_and_epochs_are_strictly_ordered() {
        let result = partition_integrity();
        assert_eq!(result["site_groups_disjoint"], true);
        assert_eq!(result["temporal_order_pass"], true);
        assert_eq!(result["test_data_used_for_fit_or_calibration"], false);
        assert_eq!(result["integrity_pass"], true);
    }

    #[test]
    fn source_commit_is_validated_before_receipt_generation() {
        assert!(execute("not-a-commit").is_err());
        assert!(execute(&"A".repeat(40)).is_err());
        assert!(execute(&"a".repeat(40)).is_ok());
    }

    #[test]
    fn receipt_proves_zero_false_valids_and_binds_envelope_and_model() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["summary"]["scenario_count"], 16);
        assert_eq!(receipt["summary"]["out_of_domain_negative_control_count"], 15);
        assert_eq!(receipt["summary"]["naive_false_valid_claim_count"], 15);
        assert_eq!(receipt["summary"]["envelope_gate_false_valid_claim_count"], 0);
        assert_eq!(receipt["summary"]["envelope_gate_false_rejection_count"], 0);
        assert_eq!(receipt["drift_monitor"]["detection_delay_samples"], 1);
        assert_eq!(receipt["drift_monitor"]["noise_only_false_alarm"], false);
        assert_eq!(receipt["calibration_envelope_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["drift_model_spec_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(receipt["reference_source_sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn calibration_expiry_is_not_confused_with_model_residual_drift() {
        let mut context = in_domain_context("expired-not-drift");
        context.epoch = MAX_VALID_EPOCH + 1;
        assert_eq!(validate_observation(&context), Err("EPOCH_OUT_OF_ENVELOPE"));
        let residuals = [0_i64, 1_000, 1_000, 2_000, 4_000, 4_000];
        assert_eq!(drift_detection_index(&residuals), Some(5));
    }
}
