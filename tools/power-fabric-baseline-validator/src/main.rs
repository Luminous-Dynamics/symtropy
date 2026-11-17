// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;

const BASELINES: &str =
    include_str!("../../../docs/research/power-fabric/data/transport-modality-baselines.v0.1.json");
const BASELINES_JSONL: &str = include_str!(
    "../../../docs/research/power-fabric/data/transport-modality-baselines.v0.1.jsonl"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaselineSummary {
    pub records: usize,
    pub measurements: usize,
}

fn field<'a>(value: &'a Value, key: &str, label: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("{label}: missing field {key}"))
}

fn non_empty_string(value: &Value, key: &str, label: &str) -> Result<String, String> {
    let text = field(value, key, label)?
        .as_str()
        .ok_or_else(|| format!("{label}: {key} must be a string"))?;
    if text.trim().is_empty() {
        return Err(format!("{label}: {key} must not be empty"));
    }
    Ok(text.to_owned())
}

fn non_empty_string_array(value: &Value, key: &str, label: &str) -> Result<usize, String> {
    let values = field(value, key, label)?
        .as_array()
        .ok_or_else(|| format!("{label}: {key} must be an array"))?;
    if values.is_empty() {
        return Err(format!("{label}: {key} must not be empty"));
    }
    for (index, item) in values.iter().enumerate() {
        let text = item
            .as_str()
            .ok_or_else(|| format!("{label}: {key}[{index}] must be a string"))?;
        if text.trim().is_empty() {
            return Err(format!("{label}: {key}[{index}] must not be empty"));
        }
    }
    Ok(values.len())
}

fn validate_publication_date(value: &str, label: &str) -> Result<(), String> {
    let valid_shape = match value.len() {
        7 => value.as_bytes()[4] == b'-',
        10 => value.as_bytes()[4] == b'-' && value.as_bytes()[7] == b'-',
        _ => false,
    };
    let digits_only = value
        .bytes()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    if !valid_shape || !digits_only {
        return Err(format!(
            "{label}: publication_date must be YYYY-MM or YYYY-MM-DD"
        ));
    }
    let month = value[5..7]
        .parse::<u8>()
        .map_err(|_| format!("{label}: invalid publication month"))?;
    if !(1..=12).contains(&month) {
        return Err(format!("{label}: publication month must be in 01..=12"));
    }
    if value.len() == 10 {
        let day = value[8..10]
            .parse::<u8>()
            .map_err(|_| format!("{label}: invalid publication day"))?;
        if !(1..=31).contains(&day) {
            return Err(format!("{label}: publication day must be in 01..=31"));
        }
    }
    Ok(())
}

fn validate_source(source: &Value, label: &str) -> Result<(), String> {
    non_empty_string(source, "title", label)?;
    non_empty_string(source, "publisher", label)?;
    let url = non_empty_string(source, "url", label)?;
    if !url.starts_with("https://") {
        return Err(format!("{label}: source URL must use HTTPS"));
    }
    let date = non_empty_string(source, "publication_date", label)?;
    validate_publication_date(&date, label)?;
    if let Some(doi) = source.get("doi") {
        let doi = doi
            .as_str()
            .ok_or_else(|| format!("{label}: doi must be a string"))?;
        if !doi.starts_with("10.") || !doi.contains('/') {
            return Err(format!("{label}: DOI must retain its 10.xxxx/... form"));
        }
    }
    Ok(())
}

fn validate_service(service: &Value, label: &str) -> Result<(), String> {
    non_empty_string(service, "service_kind", label)?;
    non_empty_string(service, "application", label)?;
    non_empty_string(service, "medium", label)?;
    if let Some(distance) = service.get("distance_m") {
        let distance = distance
            .as_f64()
            .ok_or_else(|| format!("{label}: distance_m must be numeric"))?;
        if !distance.is_finite() || distance <= 0.0 {
            return Err(format!("{label}: distance_m must be finite and positive"));
        }
    }
    Ok(())
}

fn validate_measurement(measurement: &Value, label: &str) -> Result<(), String> {
    let name = non_empty_string(measurement, "name", label)?;
    let value = field(measurement, "value", label)?
        .as_f64()
        .ok_or_else(|| format!("{label}: value must be numeric"))?;
    if !value.is_finite() {
        return Err(format!("{label}: value must be finite"));
    }
    non_empty_string(measurement, "unit", label)?;
    let relation = non_empty_string(measurement, "relation", label)?;
    let upper = measurement.get("value_upper").and_then(Value::as_f64);
    match relation.as_str() {
        "exact" | "approximate" | "greater_than" | "less_than" => {
            if upper.is_some() {
                return Err(format!(
                    "{label}: {relation} measurements must not have value_upper"
                ));
            }
        }
        "range" => {
            let upper = upper.ok_or_else(|| format!("{label}: range requires value_upper"))?;
            if !upper.is_finite() || upper <= value {
                return Err(format!(
                    "{label}: range upper bound must be finite and greater than lower"
                ));
            }
        }
        _ => return Err(format!("{label}: unsupported relation {relation}")),
    }
    let evidence_kind = non_empty_string(measurement, "evidence_kind", label)?;
    let allowed_evidence_kinds = [
        "source_reported_measurement",
        "source_reported_derived",
        "source_declared_parameter",
        "source_reported_model_fit",
        "source_reported_announcement_claim",
        "source_reported_simulation",
    ];
    if !allowed_evidence_kinds.contains(&evidence_kind.as_str()) {
        return Err(format!(
            "{label}: unsupported evidence_kind {evidence_kind}"
        ));
    }
    non_empty_string(measurement, "measurement_plane", label)?;
    let boundary = non_empty_string(measurement, "boundary", label)?;
    if !measurement
        .get("source_reported")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(format!("{label}: source_reported must be true in v0.1"));
    }
    non_empty_string_array(measurement, "conditions", label)?;
    if name.contains("efficiency") && boundary.trim().is_empty() {
        return Err(format!(
            "{label}: efficiency must have an explicit boundary"
        ));
    }
    Ok(())
}

fn parse_jsonl_records(raw: &str) -> Result<Vec<Value>, String> {
    let mut records = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            return Err(format!("JSONL line {} must not be blank", index + 1));
        }
        let record: Value = serde_json::from_str(line)
            .map_err(|error| format!("JSONL line {} is invalid: {error}", index + 1))?;
        if !record.is_object() {
            return Err(format!(
                "JSONL line {} must contain one record object",
                index + 1
            ));
        }
        records.push(record);
    }
    if records.is_empty() {
        return Err("JSONL register must contain at least one record".to_owned());
    }
    Ok(records)
}

pub fn validate_frozen_artifacts(document: &Value, jsonl: &str) -> Result<BaselineSummary, String> {
    let manifest_records = field(document, "records", "root")?;
    let jsonl_records = parse_jsonl_records(jsonl)?;
    if manifest_records.as_array() != Some(&jsonl_records) {
        return Err("JSONL records differ from the frozen JSON manifest".to_owned());
    }
    validate_document(document)
}

pub fn validate_document(document: &Value) -> Result<BaselineSummary, String> {
    if field(document, "schema_version", "root")?.as_str() != Some("0.1.0") {
        return Err("root: schema_version must be 0.1.0".to_owned());
    }
    if field(document, "record_status", "root")?.as_str()
        != Some("source_claims_only_not_independently_reproduced")
    {
        return Err("root: record_status must preserve source-claim-only status".to_owned());
    }

    let policy = field(document, "comparison_policy", "root")?;
    if field(policy, "global_efficiency_ranking", "comparison_policy")?.as_bool() != Some(false) {
        return Err("comparison_policy: global efficiency ranking must be disabled".to_owned());
    }
    let required_dimensions = field(policy, "required_match_dimensions", "comparison_policy")?
        .as_array()
        .ok_or_else(|| {
            "comparison_policy: required_match_dimensions must be an array".to_owned()
        })?;
    if required_dimensions.is_empty() {
        return Err("comparison_policy: required match dimensions must not be empty".to_owned());
    }
    non_empty_string(policy, "rule", "comparison_policy")?;

    let records = field(document, "records", "root")?
        .as_array()
        .ok_or_else(|| "root: records must be an array".to_owned())?;
    if records.is_empty() {
        return Err("root: records must not be empty".to_owned());
    }

    let allowed_modalities = [
        "inductive_wpt",
        "optical_laser_wpt",
        "acoustic_ultrasonic_wpt",
        "dynamic_roadway_wpt",
    ];
    let allowed_study_types = [
        "paper_reported_prototype",
        "paper_reported_model",
        "government_demonstration",
        "industry_demonstration",
        "industry_pilot",
    ];
    let allowed_relations = ["exact", "approximate", "greater_than", "range"];
    let mut record_ids = BTreeSet::new();
    let mut measurement_count = 0;

    for record in records {
        let id = non_empty_string(record, "id", "record")?;
        if !record_ids.insert(id.clone()) {
            return Err(format!("record {id}: duplicate record id"));
        }

        let modality = non_empty_string(record, "modality", &id)?;
        if !allowed_modalities.contains(&modality.as_str()) {
            return Err(format!("record {id}: unsupported modality {modality}"));
        }
        let study_type = non_empty_string(record, "study_type", &id)?;
        if !allowed_study_types.contains(&study_type.as_str()) {
            return Err(format!("record {id}: unsupported study_type {study_type}"));
        }
        non_empty_string(record, "comparison_group", &id)?;

        validate_source(field(record, "source", &id)?, &format!("{id}.source"))?;
        validate_service(field(record, "service", &id)?, &format!("{id}.service"))?;

        let metrics = field(record, "measurements", &id)?
            .as_array()
            .ok_or_else(|| format!("record {id}: measurements must be an array"))?;
        if metrics.is_empty() {
            return Err(format!("record {id}: measurements must not be empty"));
        }
        for measurement in metrics {
            let label = format!("{id}.measurement");
            let relation = non_empty_string(measurement, "relation", &label)?;
            if !allowed_relations.contains(&relation.as_str()) {
                return Err(format!("{label}: invalid relation {relation}"));
            }
            validate_measurement(measurement, &label)?;
        }
        measurement_count += metrics.len();

        non_empty_string_array(record, "known_unknowns", &id)?;
        if field(record, "reproduction_status", &id)?.as_str()
            != Some("not_independently_reproduced_by_luminous")
        {
            return Err(format!("{id}: reproduction status overstates evidence"));
        }
    }

    Ok(BaselineSummary {
        records: records.len(),
        measurements: measurement_count,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let document: Value = serde_json::from_str(BASELINES)?;
    let summary =
        validate_frozen_artifacts(&document, BASELINES_JSONL).map_err(std::io::Error::other)?;
    println!(
        "VALID: {} source-claim records, {} measurement observations; no cross-modal ranking permitted",
        summary.records, summary.measurements
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn corpus() -> Value {
        serde_json::from_str(BASELINES).unwrap()
    }

    #[test]
    fn frozen_jsonl_records_match_the_json_manifest_exactly() {
        let document = corpus();
        let summary = validate_frozen_artifacts(&document, BASELINES_JSONL).unwrap();
        assert_eq!(summary.records, 10);
        assert_eq!(summary.measurements, 46);
    }

    #[test]
    fn rejects_a_jsonl_file_with_malformed_record_data() {
        let document = corpus();
        let malformed = "{ not valid json }\\n";
        assert!(
            validate_frozen_artifacts(&document, malformed)
                .unwrap_err()
                .contains("invalid")
        );
    }

    #[test]
    fn rejects_jsonl_manifest_drift() {
        let document = corpus();
        let drifted = BASELINES_JSONL.replace("riekerk-2026-50kw-ipt", "changed-id");
        assert!(
            validate_frozen_artifacts(&document, &drifted)
                .unwrap_err()
                .contains("differ")
        );
    }

    #[test]
    fn frozen_v01_corpus_validates_with_six_source_records() {
        let summary = validate_document(&corpus()).unwrap();
        assert_eq!(summary.records, 10);
        assert_eq!(summary.measurements, 46);
    }

    #[test]
    fn distinct_lpt_boundaries_have_distinct_comparison_groups() {
        let document = corpus();
        let records = document["records"].as_array().unwrap();
        let ntt = records
            .iter()
            .find(|r| r["id"] == "ntt-mhi-2025-1km-laser")
            .unwrap();
        let yigit = records
            .iter()
            .find(|r| r["id"] == "yigit-2026-20m-laser")
            .unwrap();
        assert_ne!(ntt["comparison_group"], yigit["comparison_group"]);
    }

    #[test]
    fn multiple_records_may_share_a_group_for_a_future_matched_reproduction() {
        let mut document = corpus();
        let mut replicate = document["records"][0].clone();
        replicate["id"] = json!("riekerk-reproduction-01");
        document["records"].as_array_mut().unwrap().push(replicate);
        assert!(validate_document(&document).is_ok());
    }

    #[test]
    fn ntu_and_yigit_efficiencies_are_not_rankable_against_each_other() {
        let document = corpus();
        let records = document["records"].as_array().unwrap();
        let ntt = records.iter().find(|r| r["id"] == "ntt-mhi-2025-1km-laser").unwrap();
        let yigit = records.iter().find(|r| r["id"] == "yigit-2026-20m-laser").unwrap();
        assert_ne!(ntt["comparison_group"], yigit["comparison_group"]);
        assert_eq!(
            document["comparison_policy"]["global_efficiency_ranking"],
            false
        );
    }

    #[test]
    fn accepts_source_declared_efficiency_parameters_without_promoting_them_to_results() {
        let document = corpus();
        let record = document["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["id"] == "sfasciamuro-2026-uav-laser-pointing")
            .unwrap();
        let laser = record["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|measurement| {
                measurement["name"] == "laser_electrical_to_optical_efficiency_parameter"
            })
            .unwrap();
        assert_eq!(laser["evidence_kind"], "source_declared_parameter");
        assert!(
            laser["boundary"]
                .as_str()
                .unwrap()
                .contains("not presented as a complete")
        );
        assert!(
            record["known_unknowns"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| {
                    x.as_str()
                        .unwrap()
                        .contains("No independently metered useful battery-terminal energy")
                })
        );
    }

    #[test]
    fn accepts_less_than_relation_for_calibration_error() {
        let document = corpus();
        let record = document["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["id"] == "sfasciamuro-2026-uav-laser-pointing")
            .unwrap();
        let calibration = record["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|measurement| measurement["name"] == "spot_center_and_diameter_calibration_error")
            .unwrap();
        assert_eq!(calibration["relation"], "less_than");
        assert!(validate_measurement(calibration, "calibration").is_ok());
    }

    #[test]
    fn keeps_fault_tolerant_dynamic_wpt_simulation_separate_from_experimental_recovery() {
        let document = corpus();
        let record = document["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["id"] == "santhakumar-2026-fault-tolerant-dwpt")
            .unwrap();
        let metrics = record["measurements"].as_array().unwrap();
        let simulated = metrics
            .iter()
            .find(|metric| metric["name"] == "simulated_fault_detection_latency")
            .unwrap();
        let measured = metrics
            .iter()
            .find(|metric| metric["name"] == "experimental_fault_recovery_time")
            .unwrap();
        assert_eq!(simulated["evidence_kind"], "source_reported_simulation");
        assert_eq!(measured["evidence_kind"], "source_reported_measurement");
        assert!(
            record["known_unknowns"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x.as_str().unwrap().contains("excludes misalignment"))
        );
    }

    #[test]
    fn keeps_acoustic_cavitation_limit_as_model_not_received_power() {
        let document = corpus();
        let record = document["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["id"] == "li-2026-underwater-acoustic-cavitation-limit")
            .unwrap();
        let metric = record["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|metric| {
                metric["name"] == "cavitation_constrained_transmit_acoustic_power_ceiling"
            })
            .unwrap();
        assert_eq!(metric["evidence_kind"], "source_reported_simulation");
        assert!(
            metric["boundary"]
                .as_str()
                .unwrap()
                .contains("not transmitter electrical input, receiver power")
        );
    }

    #[test]
    fn rejects_duplicate_record_ids() {
        let mut document = corpus();
        let duplicate = document["records"][0].clone();
        document["records"].as_array_mut().unwrap().push(duplicate);
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("duplicate record id")
        );
    }

    #[test]
    fn rejects_missing_measurement_plane() {
        let mut document = corpus();
        document["records"][0]["measurements"][0]["measurement_plane"] = json!("");
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("measurement_plane")
        );
    }

    #[test]
    fn rejects_range_without_upper_bound() {
        let mut document = corpus();
        document["records"][0]["measurements"][0]
            .as_object_mut()
            .unwrap()
            .remove("value_upper");
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("range requires value_upper")
        );
    }

    #[test]
    fn rejects_greater_than_with_an_upper_bound() {
        let mut document = corpus();
        document["records"][2]["measurements"][0]["value_upper"] = json!(900.0);
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("greater_than")
        );
    }

    #[test]
    fn rejects_efficiency_without_a_declared_boundary() {
        let mut document = corpus();
        document["records"][1]["measurements"][2]["boundary"] = json!(" ");
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("boundary")
        );
    }

    #[test]
    fn rejects_unknowns_that_were_silently_omitted() {
        let mut document = corpus();
        document["records"][3]["known_unknowns"] = json!([]);
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("known_unknowns")
        );
    }

    #[test]
    fn rejects_non_https_source() {
        let mut document = corpus();
        document["records"][0]["source"]["url"] = json!("http://example.invalid/paper");
        assert!(validate_document(&document).unwrap_err().contains("HTTPS"));
    }

    #[test]
    fn rejects_global_efficiency_ranking() {
        let mut document = corpus();
        document["comparison_policy"]["global_efficiency_ranking"] = json!(true);
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("global efficiency ranking")
        );
    }

    #[test]
    fn rejects_non_source_claim_status_promotion() {
        let mut document = corpus();
        document["records"][0]["reproduction_status"] = json!("independently_verified");
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("overstates evidence")
        );
    }

    #[test]
    fn rejects_unknown_relation_labels() {
        let mut document = corpus();
        document["records"][0]["measurements"][0]["relation"] = json!("roughly");
        assert!(
            validate_document(&document)
                .unwrap_err()
                .contains("invalid relation")
        );
    }

    #[test]
    fn rejects_invalid_publication_month() {
        let mut document = corpus();
        document["records"][1]["source"]["publication_date"] = json!("2025-13-17");
        assert!(validate_document(&document).unwrap_err().contains("month"));
    }
}
