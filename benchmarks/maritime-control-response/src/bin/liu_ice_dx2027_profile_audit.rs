#![forbid(unsafe_code)]

//! Audits the public metadata contract for LiU-ICE DX/Safeprocess 2027.
//! This binary intentionally does not inspect or invent archive contents.

use serde_json::Value;
use std::error::Error;
use std::fs;

const PROFILE: &str = include_str!("../../liu-ice-dx2027-profile-v0.1.json");
const SCHEMA_VERSION: &str = "liu-ice-dx2027-profile-v0.1";

fn require_str<'a>(value: &'a Value, key: &str, expected: &str) -> Result<(), String> {
    if value.get(key).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{key} must equal {expected:?}"))
    }
}

fn require_bool(value: &Value, key: &str, expected: bool) -> Result<(), String> {
    if value.get(key).and_then(Value::as_bool) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{key} must equal {expected}"))
    }
}

fn require_u64(value: &Value, key: &str, expected: u64) -> Result<(), String> {
    if value.get(key).and_then(Value::as_u64) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{key} must equal {expected}"))
    }
}

fn string_array(value: &Value, key: &str) -> Result<Vec<String>, String> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} must contain only strings"))
        })
        .collect()
}

pub fn validate_profile(profile: &Value) -> Result<(), String> {
    require_str(profile, "schema_version", SCHEMA_VERSION)?;
    require_str(profile, "benchmark_id", "LIU-ICE-DX-SAFEPROCESS-2027")?;
    require_u64(profile, "edition", 2027)?;
    require_str(
        profile,
        "status",
        "official_source_metadata_only_archive_not_acquired",
    )?;

    let source = profile.get("source").ok_or("source is required")?;
    require_str(
        source,
        "url",
        "https://vehsys.gitlab-pages.liu.se/dx_safeprocess_benchmarks/liuice/",
    )?;
    require_str(
        source,
        "comparison_edition_url",
        "https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index",
    )?;

    let acquisition = profile.get("acquisition").ok_or("acquisition is required")?;
    require_bool(acquisition, "download_link_published", true)?;
    require_str(acquisition, "archive_status", "linked_not_downloaded")?;
    require_bool(acquisition, "archive_bytes_acquired", false)?;
    require_str(
        acquisition,
        "license_status",
        "not_verified_from_archive_or_download_terms",
    )?;
    require_bool(acquisition, "redistribution_permission_verified", false)?;
    if !acquisition.get("archive_sha256").is_some_and(Value::is_null)
        || !acquisition
            .get("recording_manifest_sha256")
            .is_some_and(Value::is_null)
        || acquisition
            .get("file_inventory")
            .and_then(Value::as_array)
            .is_none_or(|items| !items.is_empty())
    {
        return Err("unacquired archive must not claim hashes or an inventory".into());
    }

    let dataset = profile.get("dataset").ok_or("dataset is required")?;
    require_u64(dataset, "training_dataset_count_reported", 41)?;
    require_u64(dataset, "sample_rate_hz", 20)?;
    require_u64(dataset, "fault_onset_seconds_approx", 120)?;
    require_str(dataset, "training_cycle", "WLTP")?;
    require_bool(dataset, "test_may_include_other_cycles", true)?;
    require_bool(dataset, "model_parameters_known", false)?;
    require_bool(
        dataset,
        "available_signals_sufficient_to_uniquely_determine_model_parameters",
        false,
    )?;

    let sensors = dataset
        .get("sensors")
        .and_then(Value::as_array)
        .ok_or("sensors must be an array")?;
    let sensor_ids: Vec<&str> = sensors
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .collect();
    let expected_sensors = ["ypic", "yTic", "ypim", "ywaf", "yxpos", "yomega", "ypamb", "yTamb"];
    if sensors.len() != expected_sensors.len()
        || expected_sensors.iter().any(|id| !sensor_ids.contains(id))
    {
        return Err("2027 sensor inventory does not match the published eight signals".into());
    }
    let x_position = sensors
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some("yxpos"))
        .ok_or("yxpos is missing")?;
    require_str(
        x_position,
        "role",
        "candidate_actuator_feedback",
    )?;
    require_str(
        x_position,
        "semantics_review",
        "instrument_and_io_dictionary_review_required",
    )?;

    let actuators = dataset
        .get("actuator_signals")
        .and_then(Value::as_array)
        .ok_or("actuator_signals must be an array")?;
    let actuator_ids: Vec<&str> = actuators
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .collect();
    if actuators.len() != 2 || !actuator_ids.contains(&"umf") || !actuator_ids.contains(&"uwg") {
        return Err("2027 actuator inventory must contain umf and uwg".into());
    }
    for actuator in actuators {
        require_str(actuator, "role", "control_intent")?;
    }

    let labels = string_array(dataset, "fault_labels_as_published")?;
    if labels != ["f_pic", "y_pim", "y_waf", "f_iml"] {
        return Err("published fault-label spellings changed; refresh the crosswalk".into());
    }
    let crosswalk = dataset
        .get("fault_label_crosswalk")
        .and_then(Value::as_array)
        .ok_or("fault_label_crosswalk must be an array")?;
    if crosswalk.len() != 4
        || !crosswalk.iter().any(|entry| {
            entry.get("source_fault_label").and_then(Value::as_str) == Some("y_pim")
                && entry.get("isolation_slot").and_then(Value::as_str) == Some("f_pim")
                && entry.get("mapping_status").and_then(Value::as_str)
                    == Some("needs_official_evaluator_crosscheck")
        })
        || !crosswalk.iter().any(|entry| {
            entry.get("source_fault_label").and_then(Value::as_str) == Some("y_waf")
                && entry.get("isolation_slot").and_then(Value::as_str) == Some("f_waf")
                && entry.get("mapping_status").and_then(Value::as_str)
                    == Some("needs_official_evaluator_crosscheck")
        })
    {
        return Err("fault-label aliases must remain visible and unverified".into());
    }

    let interface = profile.get("interface").ok_or("interface is required")?;
    require_str(interface, "input_method", "Input(sample)")?;
    require_bool(interface, "one_output_per_sample", true)?;
    require_bool(interface, "historical_outputs_may_be_revised", false)?;
    require_bool(interface, "detection_type", true).or_else(|_| {
        require_str(interface, "detection_type", "boolean")
    })?;
    require_u64(interface, "isolation_vector_length", 5)?;
    if string_array(interface, "isolation_class_order")?
        != ["f_pic", "f_pim", "f_waf", "f_iml", "f_x"]
    {
        return Err("five-way isolation class order changed".into());
    }
    require_bool(
        interface,
        "one_output_per_sample",
        true,
    )?;
    require_str(
        interface,
        "language_policy_status",
        "source_text_requires_organizer_confirmation",
    )?;

    let scoring = profile.get("scoring").ok_or("scoring is required")?;
    require_str(
        scoring,
        "total_score_formula",
        "((1 - FAR) + TDR + FIA) / 3",
    )?;
    require_bool(scoring, "evaluation_uses_secret_test_data", true)?;

    let comparison = profile
        .get("edition_comparison")
        .ok_or("edition_comparison is required")?;
    require_u64(comparison, "dx2026_training_dataset_count_reported", 26)?;
    require_u64(comparison, "dx2027_training_dataset_count_reported", 41)?;
    require_bool(
        comparison,
        "dx2026_score_may_be_reused_as_dx2027_result",
        false,
    )?;

    let ambiguities = profile
        .get("source_ambiguities")
        .and_then(Value::as_array)
        .ok_or("source_ambiguities must be an array")?;
    for required in ["training-count-narrative", "fault-label-aliases", "submission-language"] {
        if !ambiguities.iter().any(|item| {
            item.get("id").and_then(Value::as_str) == Some(required)
        }) {
            return Err(format!("source ambiguity {required} must remain explicit"));
        }
    }

    let claims = profile.get("claim_ceiling").ok_or("claim_ceiling is required")?;
    require_bool(claims, "source_metadata_audited", true)?;
    for key in [
        "archive_bytes_audited",
        "file_schema_audited",
        "model_reproduced",
        "benchmark_score_claim_allowed",
        "physical_validation_claim_allowed",
        "cross_engine_transfer_claim_allowed",
    ] {
        require_bool(claims, key, false)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let profile: Value = serde_json::from_str(PROFILE)?;
    validate_profile(&profile).map_err(|message| format!("profile rejected: {message}"))?;
    let bytes = serde_json::to_vec(&profile)?;
    let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
    println!(
        "status=OFFICIAL_METADATA_CONTRACT_VALID schema={} edition=2027 training_datasets=41 archive=NOT_ACQUIRED profile_sha256={digest}",
        SCHEMA_VERSION
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(PROFILE).expect("profile JSON must parse")
    }

    #[test]
    fn official_2027_metadata_profile_is_valid_and_keeps_archive_unacquired() {
        let profile = fixture();
        assert!(validate_profile(&profile).is_ok());
        assert_eq!(profile["dataset"]["training_dataset_count_reported"], 41);
        assert_eq!(profile["acquisition"]["archive_sha256"], Value::Null);
    }

    #[test]
    fn 2026_and_2027_recording_counts_are_not_interchangeable() {
        let mut profile = fixture();
        profile["dataset"]["training_dataset_count_reported"] = 26.into();
        assert!(validate_profile(&profile).is_err());
    }

    #[test]
    fn no_archive_hash_or_file_inventory_can_be_fabricated() {
        let mut profile = fixture();
        profile["acquisition"]["archive_bytes_acquired"] = true.into();
        assert!(validate_profile(&profile).is_err());

        let mut profile = fixture();
        profile["acquisition"]["archive_sha256"] =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
        assert!(validate_profile(&profile).is_err());
    }

    #[test]
    fn fault_label_aliases_cannot_be_silently_normalized() {
        let mut profile = fixture();
        profile["dataset"]["fault_labels_as_published"][1] = "f_pim".into();
        assert!(validate_profile(&profile).is_err());
    }

    #[test]
    fn an_incorrect_isolation_order_is_rejected() {
        let mut profile = fixture();
        profile["interface"]["isolation_class_order"][0] = "f_pim".into();
        assert!(validate_profile(&profile).is_err());
    }

    #[test]
    fn source_policy_ambiguities_must_be_preserved() {
        let mut profile = fixture();
        profile["source_ambiguities"] = serde_json::json!([]);
        assert!(validate_profile(&profile).is_err());
    }

    #[test]
    fn the_2026_score_cannot_be_promoted_to_a_2027_result() {
        let mut profile = fixture();
        profile["edition_comparison"]["dx2026_score_may_be_reused_as_dx2027_result"] =
            true.into();
        assert!(validate_profile(&profile).is_err());
    }
}
