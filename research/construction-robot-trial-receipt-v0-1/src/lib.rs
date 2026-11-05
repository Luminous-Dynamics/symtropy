use serde_json::Value;

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}

fn evidence_rank(value: &str) -> Option<u8> {
    match value {
        "synthetic" => Some(0),
        "simulated" => Some(1),
        "bench_measured" => Some(2),
        "hil_measured" => Some(3),
        "field_observed" => Some(4),
        "independently_adjudicated" => Some(5),
        _ => None,
    }
}

fn is_measured_status(status: &str) -> bool {
    matches!(
        status,
        "measured_within_declared_criterion" | "measured_outside_declared_criterion"
    )
}

/// Returns stable semantic reason codes. These checks do not authenticate
/// artifacts, certificates, instrument independence, or physical truth.
pub fn semantic_errors(receipt: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    let evidence_class = string_at(receipt, &["claim", "evidence_class"]);
    let claim_ceiling = string_at(receipt, &["claim", "claim_ceiling"]);
    let result_status = string_at(receipt, &["result", "status"]);
    let evidence_rank_value = evidence_class.and_then(evidence_rank);
    let ceiling_rank_value = claim_ceiling.and_then(evidence_rank);

    if let (Some(class_rank), Some(ceiling_rank)) = (evidence_rank_value, ceiling_rank_value)
        && ceiling_rank > class_rank
    {
        errors.push("claim_ceiling_exceeds_evidence_class".to_owned());
    }

    let measured = result_status.is_some_and(is_measured_status);
    if measured && evidence_rank_value.is_none_or(|rank| rank < 2) {
        errors.push("measured_result_without_physical_evidence_class".to_owned());
    }

    if measured {
        let reference = receipt.get("reference");
        let independence_status = reference
            .and_then(|value| value.get("independence"))
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str);
        let shared_domains_empty = reference
            .and_then(|value| value.get("independence"))
            .and_then(|value| value.get("shared_fault_domains"))
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);

        if independence_status != Some("independent") || !shared_domains_empty {
            errors.push("measured_result_reference_not_independent".to_owned());
        }

        let start_ns = u64_at(receipt, &["trial", "started_at_unix_ns"]);
        let end_ns = u64_at(receipt, &["trial", "ended_at_unix_ns"]);
        let instruments = reference
            .and_then(|value| value.get("instruments"))
            .and_then(Value::as_array);

        if let (Some(start_ns), Some(end_ns), Some(sensors)) = (
            start_ns,
            end_ns,
            receipt
                .get("subject")
                .and_then(|value| value.get("sensors"))
                .and_then(Value::as_array),
        ) {
            for sensor in sensors {
                let sensor_id = sensor
                    .get("sensor_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let from = sensor
                    .get("calibration_valid_from_unix_ns")
                    .and_then(Value::as_u64);
                let until = sensor
                    .get("calibration_valid_until_unix_ns")
                    .and_then(Value::as_u64);
                if !matches!((from, until), (Some(from), Some(until)) if from <= start_ns && until >= end_ns && from <= until)
                {
                    errors.push(format!(
                        "subject_sensor_calibration_does_not_cover_trial:{sensor_id}"
                    ));
                }
            }
        }

        // Check the declared reference fault domains against the evaluated
        // robot's own sensor domains. A self-declared "independent" status is
        // insufficient when the named domains visibly overlap.
        let subject_fault_domains = receipt
            .get("subject")
            .and_then(|value| value.get("sensors"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|sensor| sensor.get("fault_domain").and_then(Value::as_str))
            .collect::<std::collections::BTreeSet<_>>();
        if let Some(instruments) = instruments {
            for instrument in instruments {
                if let Some(domain) = instrument.get("fault_domain").and_then(Value::as_str)
                    && subject_fault_domains.contains(domain)
                {
                    errors.push(format!("reference_fault_domain_overlaps_subject:{domain}"));
                }
            }
        }

        // Reference time must be a distinct clock domain from the evaluated
        // robot/event clock. Synchronization evidence binds the two clocks;
        // it must not silently make them the same failure source.
        let subject_clock = string_at(receipt, &["trial", "clock_domain"]);
        let reference_clock = string_at(
            receipt,
            &[
                "reference",
                "time_synchronization",
                "reference_clock_domain",
            ],
        );
        if subject_clock.is_some() && subject_clock == reference_clock {
            errors.push("reference_clock_domain_overlaps_subject_clock_domain".to_owned());
        }

        if let (Some(start_ns), Some(end_ns), Some(instruments)) = (start_ns, end_ns, instruments) {
            for instrument in instruments {
                let instrument_id = instrument
                    .get("instrument_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let from = instrument
                    .get("calibration_valid_from_unix_ns")
                    .and_then(Value::as_u64);
                let until = instrument
                    .get("calibration_valid_until_unix_ns")
                    .and_then(Value::as_u64);
                if !matches!((from, until), (Some(from), Some(until)) if from <= start_ns && until >= end_ns && from <= until)
                {
                    errors.push(format!(
                        "reference_calibration_does_not_cover_trial:{instrument_id}"
                    ));
                }
            }
        }

        let sample = receipt.get("sample_plan");
        let total = sample
            .and_then(|v| v.get("total_encounters"))
            .and_then(Value::as_u64);
        let valid = sample
            .and_then(|v| v.get("valid_encounters"))
            .and_then(Value::as_u64);
        let excluded = sample
            .and_then(|v| v.get("excluded_encounters"))
            .and_then(Value::as_u64);
        let independent = sample
            .and_then(|v| v.get("independent_encounters"))
            .and_then(Value::as_u64);

        match (total, valid, excluded, independent) {
            (Some(total), Some(valid), Some(excluded), Some(independent)) => {
                if total == 0 || independent == 0 {
                    errors.push("measured_result_missing_independent_encounters".to_owned());
                }
                if valid.saturating_add(excluded) != total {
                    errors.push("encounter_denominators_do_not_reconcile".to_owned());
                }
                if independent > valid {
                    errors.push("independent_encounters_exceed_valid_encounters".to_owned());
                }
                if sample
                    .and_then(|value| value.get("distinct_sites"))
                    .and_then(Value::as_u64)
                    .is_none_or(|count| count == 0)
                    || sample
                        .and_then(|value| value.get("distinct_configurations"))
                        .and_then(Value::as_u64)
                        .is_none_or(|count| count == 0)
                {
                    errors.push(
                        "measured_result_missing_site_or_configuration_denominator".to_owned(),
                    );
                }
            }
            _ => errors.push("measured_result_missing_sample_denominator".to_owned()),
        }

        let instrument_ids = instruments
            .into_iter()
            .flatten()
            .filter_map(|v| v.get("instrument_id").and_then(Value::as_str))
            .collect::<std::collections::BTreeSet<_>>();
        let artifact_ids = receipt
            .get("evidence")
            .and_then(|v| v.get("raw_artifacts"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| v.get("artifact_id").and_then(Value::as_str))
            .collect::<std::collections::BTreeSet<_>>();

        if artifact_ids.is_empty() {
            errors.push("measured_result_missing_raw_artifacts".to_owned());
        }

        if let Some(odd_strata) = receipt
            .get("odd")
            .and_then(|v| v.get("strata"))
            .and_then(Value::as_array)
        {
            for stratum in odd_strata {
                if let Some(ids) = stratum
                    .get("evidence_artifact_ids")
                    .and_then(Value::as_array)
                {
                    for id in ids.iter().filter_map(Value::as_str) {
                        if !artifact_ids.contains(id) {
                            errors.push(format!("odd_stratum_artifact_not_found:{id}"));
                        }
                    }
                }
            }
        }

        if let Some(measurements) = receipt.get("measurements").and_then(Value::as_array) {
            for measurement in measurements {
                let measurement_id = measurement
                    .get("measurement_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let u95 = measurement
                    .get("expanded_uncertainty_u95")
                    .and_then(Value::as_f64);
                let margin = measurement.get("decision_margin").and_then(Value::as_f64);
                let unit = measurement.get("unit").and_then(Value::as_str);
                let margin_unit = measurement
                    .get("decision_margin_unit")
                    .and_then(Value::as_str);
                if unit.is_none() || unit != margin_unit {
                    errors.push(format!(
                        "measurement_decision_margin_unit_mismatch:{measurement_id}"
                    ));
                }
                match (u95, margin) {
                    (Some(u95), Some(margin)) if margin > 0.0 && u95 <= margin / 3.0 => {}
                    (Some(_), Some(margin)) if margin > 0.0 => {
                        errors.push(format!(
                            "measurement_uncertainty_exceeds_one_third_margin:{measurement_id}"
                        ));
                    }
                    _ => errors.push(format!(
                        "measurement_uncertainty_or_margin_missing:{measurement_id}"
                    )),
                }

                if let Some(ids) = measurement
                    .get("reference_instrument_ids")
                    .and_then(Value::as_array)
                {
                    for id in ids.iter().filter_map(Value::as_str) {
                        if !instrument_ids.contains(id) {
                            errors.push(format!("measurement_instrument_not_found:{id}"));
                        }
                    }
                }

                if let Some(ids) = measurement
                    .get("raw_artifact_ids")
                    .and_then(Value::as_array)
                {
                    for id in ids.iter().filter_map(Value::as_str) {
                        if !artifact_ids.contains(id) {
                            errors.push(format!("measurement_artifact_not_found:{id}"));
                        }
                    }
                }

                let sample_count = measurement.get("sample_count").and_then(Value::as_u64);
                let sample_unit = measurement.get("sample_unit").and_then(Value::as_str);
                if sample_unit == Some("encounter")
                    && matches!((sample_count, valid), (Some(count), Some(valid)) if count > valid)
                {
                    errors.push(format!(
                        "measurement_sample_count_exceeds_valid_encounters:{measurement_id}"
                    ));
                }
                if sample_unit == Some("site")
                    && matches!(
                        (sample_count, u64_at(receipt, &["sample_plan", "distinct_sites"])),
                        (Some(count), Some(sites)) if count > sites
                    )
                {
                    errors.push(format!(
                        "measurement_sample_count_exceeds_distinct_sites:{measurement_id}"
                    ));
                }
            }
        }
    }

    if string_at(receipt, &["claim", "evidence_class"]) == Some("independently_adjudicated") {
        let adjudication = receipt.get("adjudication");
        let review_is_independent = adjudication
            .and_then(|v| v.get("independent_review"))
            .and_then(Value::as_bool)
            == Some(true);
        if !review_is_independent {
            errors.push("independent_adjudication_not_independently_reviewed".to_owned());
        }

        let result = result_status.unwrap_or_default();
        let decision = adjudication
            .and_then(|v| v.get("decision"))
            .and_then(Value::as_str);
        let aligned = match result {
            "measured_within_declared_criterion" => decision == Some("accepted_within_scope"),
            "measured_outside_declared_criterion" => decision == Some("rejected"),
            _ => true,
        };
        if !aligned {
            errors.push("adjudication_decision_conflicts_with_measured_result".to_owned());
        }
    }

    if string_at(receipt, &["challenge", "visibility"]) == Some("blind_independent") {
        let custodian = string_at(receipt, &["challenge", "custodian_reference"]);
        let output_commitment =
            string_at(receipt, &["challenge", "producer_output_commitment_sha256"]);
        let reveal_time = receipt
            .get("challenge")
            .and_then(|v| v.get("revealed_at_unix_ns"))
            .and_then(Value::as_u64);
        let trial_end = u64_at(receipt, &["trial", "ended_at_unix_ns"]);

        if custodian.is_none_or(str::is_empty) || output_commitment.is_none() {
            errors.push("blind_challenge_custody_or_output_commitment_missing".to_owned());
        }
        if matches!((reveal_time, trial_end), (Some(reveal), Some(end)) if reveal < end) {
            errors.push("blind_challenge_revealed_before_trial_end".to_owned());
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

/// Validates the JSON Schema and then semantic cross-field constraints.
/// Returned strings are stable reason codes, not a certification decision.
pub fn validate_receipt(schema: &Value, receipt: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(_) => return vec!["receipt_schema_could_not_be_compiled".to_owned()],
    };

    for error in validator.iter_errors(receipt) {
        errors.push(format!("schema_violation:{}", error.instance_path()));
    }
    errors.extend(semantic_errors(receipt));
    errors.sort();
    errors.dedup();
    errors
}

/// True only when the schema document validates against the Draft 2020-12 meta-schema.
pub fn schema_is_valid(schema: &Value) -> bool {
    jsonschema::draft202012::meta::is_valid(schema)
}

pub fn read_json(path: &std::path::Path) -> Result<Value, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot_read_file:{}:{error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid_json:{}:{error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn package_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn schema() -> Value {
        read_json(&package_path(
            "schema/construction-robot-physical-trial-v0.1.schema.json",
        ))
        .expect("schema JSON must parse")
    }

    fn fixture(path: &str) -> Value {
        read_json(&package_path(path)).expect("fixture JSON must parse")
    }

    #[test]
    fn schema_validates_against_draft_2020_12_meta_schema() {
        assert!(schema_is_valid(&schema()));
    }

    #[test]
    fn valid_examples_pass_schema_and_semantic_checks() {
        let schema = schema();
        for name in [
            "examples/bench-measured-stopping.json",
            "examples/not-evaluated-synthetic.json",
        ] {
            let receipt = fixture(name);
            let errors = validate_receipt(&schema, &receipt);
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
    }

    #[test]
    fn adversarial_fixtures_are_rejected() {
        let schema = schema();
        let directory = package_path("fixtures/invalid");
        let mut paths = std::fs::read_dir(directory)
            .expect("invalid fixture directory must exist")
            .map(|entry| entry.expect("directory entry must be readable").path())
            .filter(|path| path.extension().and_then(|v| v.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();

        assert!(
            !paths.is_empty(),
            "adversarial fixture corpus must not be empty"
        );
        for path in paths {
            let receipt = read_json(&path).expect("invalid fixture must still be valid JSON");
            let errors = validate_receipt(&schema, &receipt);
            assert!(
                !errors.is_empty(),
                "{} unexpectedly passed: {receipt}",
                path.display()
            );
        }
    }

    #[test]
    fn uncertainty_and_margin_gate_is_not_silently_ignored() {
        let receipt = fixture("fixtures/invalid/uncertainty-exceeds-margin.json");
        let errors = semantic_errors(&receipt);
        assert!(errors.iter().any(|error| {
            error.starts_with("measurement_uncertainty_exceeds_one_third_margin:")
        }));
    }

    #[test]
    fn decision_margin_must_use_the_measurement_unit() {
        let receipt = fixture("fixtures/invalid/decision-margin-unit-mismatch.json");
        assert!(semantic_errors(&receipt).iter().any(
            |error| error == "measurement_decision_margin_unit_mismatch:stopping-distance-p95"
        ));
    }

    #[test]
    fn feature_sample_count_is_not_misclassified_as_encounter_count() {
        let mut receipt = fixture("examples/bench-measured-stopping.json");
        receipt["measurements"][0]["sample_unit"] = Value::String("feature".to_owned());
        receipt["measurements"][0]["sample_count"] = Value::from(300);
        assert!(!semantic_errors(&receipt).iter().any(|error| error
            == "measurement_sample_count_exceeds_valid_encounters:stopping-distance-p95"));
    }

    #[test]
    fn claim_ceiling_cannot_be_promoted_above_evidence_class() {
        let receipt = fixture("fixtures/invalid/claim-ceiling-exceeds-evidence.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error == "claim_ceiling_exceeds_evidence_class")
        );
    }

    #[test]
    fn reference_calibration_must_cover_entire_trial() {
        let receipt = fixture("fixtures/invalid/expired-reference-calibration.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error.starts_with("reference_calibration_does_not_cover_trial:"))
        );
    }

    #[test]
    fn subject_sensor_calibration_must_cover_entire_trial() {
        let receipt = fixture("fixtures/invalid/expired-sensor-calibration.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error.starts_with("subject_sensor_calibration_does_not_cover_trial:"))
        );
    }

    #[test]
    fn shared_fault_domain_prevents_independent_reference_claim() {
        let receipt = fixture("fixtures/invalid/shared-reference-fault-domain.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error == "measured_result_reference_not_independent")
        );
    }

    #[test]
    fn matching_instrument_and_robot_fault_domain_is_rejected_even_if_declared_independent() {
        let receipt =
            fixture("fixtures/invalid/declared_independent_but_shared_instrument_domain.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error == "reference_fault_domain_overlaps_subject:robot-range-power")
        );
    }

    #[test]
    fn shared_reference_clock_domain_is_rejected() {
        let receipt = fixture("fixtures/invalid/shared_reference_clock_domain.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error == "reference_clock_domain_overlaps_subject_clock_domain")
        );
    }

    #[test]
    fn blind_challenge_requires_custody_and_output_commitment() {
        let receipt = fixture("fixtures/invalid/blind-challenge-without-custodian.json");
        assert!(
            semantic_errors(&receipt)
                .iter()
                .any(|error| error == "blind_challenge_custody_or_output_commitment_missing")
        );
    }
}
