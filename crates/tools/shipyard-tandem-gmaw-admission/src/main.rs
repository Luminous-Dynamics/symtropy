use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "SHIPYARD-TANDEM-GMAW-DATA-ADMISSION-001";
const SCHEMA_VERSION: &str = "0.1.0";
const DEFAULT_MANIFEST: &str = "benchmarks/shipyard/tandem-gmaw/data-admission-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/tech/SHIPYARD_TANDEM_GMAW_DATA_ADMISSION_V0_1.schema.json";

#[derive(Debug, PartialEq, Eq)]
struct Summary {
    workpieces: usize,
    layers: usize,
    controls: usize,
    positive: usize,
    negative: usize,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn ids_equal(actual: &[String], expected: &[&str]) -> bool {
    actual
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
}

fn string_array(value: &Value, key: &str) -> Option<Vec<String>> {
    value
        .get(key)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn validate_manifest(schema_text: &str, manifest_text: &str) -> Result<Summary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("schema JSON parse failed: {error}")]),
    };
    let manifest: Value = match serde_json::from_str(manifest_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("manifest JSON parse failed: {error}")]),
    };

    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        errors.push("schema must reject undeclared top-level fields".to_owned());
    }
    if str_at(&manifest, "schema_version") != Some(SCHEMA_VERSION) {
        errors.push("manifest schema_version mismatch".to_owned());
    }
    if str_at(&manifest, "benchmark_id") != Some(BENCHMARK_ID) {
        errors.push("manifest benchmark_id mismatch".to_owned());
    }

    let Some(source) = manifest.get("source_record") else {
        return Err(vec!["source_record is required".to_owned()]);
    };
    if str_at(source, "doi") != Some("10.5281/zenodo.21824586")
        || str_at(source, "record_url") != Some("https://zenodo.org/records/21824586")
        || str_at(source, "version") != Some("2.0.0")
        || str_at(source, "published_on") != Some("2026-08-07")
        || str_at(source, "modified_on") != Some("2026-08-10")
    {
        errors.push("Zenodo source identity/version differs from the pinned record".to_owned());
    }
    if bool_at(source, "dataset_open_badge") != Some(true) {
        errors.push("source record must preserve the open badge as metadata".to_owned());
    }
    if !source.get("license_field").is_some_and(Value::is_null) {
        errors.push("license field must remain null until explicit terms are recorded".to_owned());
    }
    if bool_at(source, "raw_files_downloaded") != Some(false)
        || bool_at(source, "raw_files_vendored") != Some(false)
    {
        errors.push(
            "raw dataset files cannot be downloaded or vendored under the current rights gate"
                .to_owned(),
        );
    }
    if str_at(source, "rights_notice") != Some("Copyright (C) 2026 The Authors.") {
        errors.push("source rights notice is missing or changed".to_owned());
    }

    let Some(rights) = manifest.get("rights_gate") else {
        return Err(vec!["rights_gate is required".to_owned()]);
    };
    if str_at(rights, "status") != Some("RIGHTS_PENDING")
        || bool_at(rights, "permission_text_recorded") != Some(false)
        || bool_at(rights, "redistribution_allowed") != Some(false)
        || bool_at(rights, "derivative_publication_allowed") != Some(false)
        || bool_at(rights, "model_training_allowed") != Some(false)
        || bool_at(rights, "raw_data_scoring_allowed") != Some(false)
        || bool_at(rights, "raw_data_license_still_pending") != Some(true)
    {
        errors.push(
            "rights gate has been promoted without explicit source-owner permission".to_owned(),
        );
    }

    let Some(design) = manifest.get("experiment_design") else {
        return Err(vec!["experiment_design is required".to_owned()]);
    };
    if design.get("workpiece_count").and_then(Value::as_u64) != Some(5)
        || design.get("layers_per_workpiece").and_then(Value::as_u64) != Some(6)
        || design.get("layer_row_count").and_then(Value::as_u64) != Some(30)
        || str_at(design, "independent_unit") != Some("workpiece")
        || str_at(design, "split_group") != Some("workpiece_id")
        || bool_at(design, "random_layer_split_allowed") != Some(false)
        || bool_at(design, "mode_causal_claim_allowed") != Some(false)
        || bool_at(design, "tandem_only_analysis_excludes_root_layer") != Some(true)
    {
        errors.push(
            "five-workpiece / six-layer / WID-grouped study design boundary violated".to_owned(),
        );
    }

    let Some(repeat_comparator) = design.get("repeat_comparator") else {
        errors.push("repeat_comparator is required".to_owned());
        return Err(errors);
    };
    if str_at(repeat_comparator, "workpiece_id") != Some("WID617")
        || bool_at(repeat_comparator, "independent_replicate") != Some(false)
    {
        errors.push("WID617 repeat-comparator semantics were altered".to_owned());
    }
    let Some(matched_pair) = design.get("matched_mode_pair") else {
        errors.push("matched_mode_pair is required".to_owned());
        return Err(errors);
    };
    let matched_pair_ids = string_array(matched_pair, "workpieces").unwrap_or_default();
    if matched_pair_ids.len() != 2
        || matched_pair_ids.first().map(String::as_str) != Some("WID631")
        || matched_pair_ids.get(1).map(String::as_str) != Some("WID632")
        || str_at(matched_pair, "contrast") != Some("PMC+PMC versus PMC+CMT")
        || matched_pair
            .get("workpiece_replicates_per_mode")
            .and_then(Value::as_u64)
            != Some(1)
        || bool_at(matched_pair, "supports_causal_generalization") != Some(false)
    {
        errors.push(
            "matched WID631/WID632 contrast cannot be promoted to causal replication".to_owned(),
        );
    }
    let Some(materials) = design.get("materials_detail") else {
        errors.push("materials_detail is required".to_owned());
        return Err(errors);
    };
    if str_at(materials, "position") != Some("1G_flat")
        || str_at(materials, "shielding_gas") != Some("82% Ar / 18% CO2 (ISO 14175 M21-ArC-18)")
        || materials.get("wire_diameter_mm").and_then(Value::as_f64) != Some(1.2)
        || materials
            .get("nominal_gas_flow_l_per_min")
            .and_then(Value::as_u64)
            != Some(20)
    {
        errors.push("material/process context differs from the published experiment".to_owned());
    }

    let Some(workpieces) = manifest.get("workpieces").and_then(Value::as_array) else {
        return Err(vec!["workpieces must be an array".to_owned()]);
    };
    if workpieces.len() != 5 {
        errors.push(format!("expected five WIDs, found {}", workpieces.len()));
    }
    let expected_wid_ids = ["WID615", "WID617", "WID630", "WID631", "WID632"];
    let mut actual_wids = Vec::new();
    let mut layer_total = 0usize;

    for (w_index, workpiece) in workpieces.iter().enumerate() {
        let wid = match str_at(workpiece, "workpiece_id") {
            Some(value) => value.to_owned(),
            None => {
                errors.push(format!(
                    "workpieces[{w_index}].workpiece_id must be a string"
                ));
                continue;
            }
        };
        if actual_wids.contains(&wid) {
            errors.push(format!("duplicate workpiece ID {wid}"));
        }
        actual_wids.push(wid.clone());

        let expected_mode = match wid.as_str() {
            "WID615" | "WID632" => Some("PMC+CMT"),
            "WID617" | "WID631" => Some("PMC+PMC"),
            "WID630" => Some("CMT+CMT"),
            _ => None,
        };
        if expected_mode.is_none() || str_at(workpiece, "tandem_mode") != expected_mode {
            errors.push(format!(
                "{wid}: unexpected or missing tandem mode assignment"
            ));
        }

        let Some(layers) = workpiece.get("layers").and_then(Value::as_array) else {
            errors.push(format!("{wid}: layers must be an array"));
            continue;
        };
        if layers.len() != 6 {
            errors.push(format!(
                "{wid}: expected six layers, found {}",
                layers.len()
            ));
        }
        layer_total += layers.len();

        for (layer_index, layer) in layers.iter().enumerate() {
            let expected_order = (layer_index + 1) as u64;
            let expected_id = format!("L{expected_order}");
            let expected_class = if layer_index == 0 {
                "single_wire_root"
            } else {
                "tandem"
            };
            if str_at(layer, "layer_id") != Some(expected_id.as_str())
                || layer.get("layer_order").and_then(Value::as_u64) != Some(expected_order)
                || str_at(layer, "layer_class") != Some(expected_class)
            {
                errors.push(format!(
                    "{wid}: layer {expected_order} has invalid order or root/tandem semantics"
                ));
            }

            let expected_roles = if layer_index == 0 {
                vec![
                    "robot_motion",
                    "process_voltage_current",
                    "power_source_leading",
                    "scan_pre",
                    "video",
                ]
            } else if layer_index == 5 {
                vec![
                    "robot_motion",
                    "process_voltage_current",
                    "power_source_leading",
                    "power_source_trailing",
                    "scan_pre",
                    "video",
                    "scan_post",
                ]
            } else {
                vec![
                    "robot_motion",
                    "process_voltage_current",
                    "power_source_leading",
                    "power_source_trailing",
                    "scan_pre",
                    "video",
                ]
            };
            let actual_roles = string_array(layer, "expected_artifact_roles").unwrap_or_default();
            if !ids_equal(&actual_roles, &expected_roles) {
                errors.push(format!(
                    "{wid} L{expected_order}: artifact roles do not match source-described topology"
                ));
            }

            let Some(t85) = layer.get("t85_field") else {
                errors.push(format!("{wid} L{expected_order}: t85_field missing"));
                continue;
            };
            let unavailable_t85 = matches!(
                (wid.as_str(), expected_order),
                ("WID630", 6) | ("WID631", 5) | ("WID631", 6) | ("WID632", 2) | ("WID632", 6)
            );
            let expected_t85_availability = if unavailable_t85 {
                "NOT_AVAILABLE"
            } else {
                "NOT_LISTED_AS_MISSING"
            };
            if str_at(t85, "value_ingestion_status") != Some("NOT_INGESTED")
                || !t85.get("raw_value").is_some_and(Value::is_null)
                || str_at(t85, "source_declared_unit") != Some("s")
                || str_at(t85, "semantic_definition_status") != Some("PAPER_DEFINED_T8_5")
                || str_at(t85, "physical_definition")
                    != Some("elapsed cooling time from 800 C to 500 C")
                || str_at(t85, "source_declared_availability") != Some(expected_t85_availability)
                || str_at(t85, "semantic_source_doi") != Some("10.1016/j.dib.2026.113329")
            {
                errors.push(format!(
                    "{wid} L{expected_order}: t85 missingness/semantic boundary was altered"
                ));
            }
        }
    }
    if !ids_equal(&actual_wids, &expected_wid_ids) {
        errors.push("WID inventory differs from the five pinned source identifiers".to_owned());
    }
    if layer_total != 30 {
        errors.push(format!(
            "expected 30 metadata layer slots, found {layer_total}"
        ));
    }

    let Some(article) = manifest.get("published_article") else {
        errors.push("published_article metadata is required".to_owned());
        return Err(errors);
    };
    if str_at(article, "doi") != Some("10.1016/j.dib.2026.113329")
        || str_at(article, "license") != Some("CC-BY-4.0")
        || bool_at(article, "not_a_license_for_zenodo_raw_archive") != Some(true)
        || bool_at(rights, "redistribution_allowed") != Some(false)
    {
        errors.push("article licence must remain separate from raw Zenodo data rights".to_owned());
    }

    let Some(clock) = manifest.get("clock_contract") else {
        errors.push("clock_contract is required".to_owned());
        return Err(errors);
    };
    if str_at(clock, "timestamp_representation") != Some("UTC")
        || str_at(clock, "master_clock") != Some("TwinCAT3_runtime_EtherCAT_master")
        || bool_at(
            clock,
            "common_master_clock_declared_for_recorded_time_series_and_video_triggers",
        ) != Some(true)
        || bool_at(clock, "distributed_clocks_enabled") != Some(true)
        || clock
            .get("reported_analog_signal_drift_upper_bound_us")
            .and_then(Value::as_f64)
            != Some(10.0)
        || clock
            .get("file_minimum_sample_intervals_ms")
            .and_then(|v| v.get("robot_file"))
            .and_then(Value::as_f64)
            != Some(1.0)
        || clock
            .get("file_minimum_sample_intervals_ms")
            .and_then(|v| v.get("process_file"))
            .and_then(Value::as_f64)
            != Some(0.1)
        || clock
            .get("file_minimum_sample_intervals_ms")
            .and_then(|v| v.get("power_source_files"))
            .and_then(Value::as_f64)
            != Some(100.0)
        || clock
            .get("channel_sampling_intervals_ms")
            .and_then(|v| v.get("robot_tcp_coords"))
            .and_then(Value::as_f64)
            != Some(8.0)
        || clock.get("video_frame_trigger_hz").and_then(Value::as_u64) != Some(40)
    {
        errors.push("clock/sample interval metadata differs from the published article".to_owned());
    }
    if str_at(clock, "drift_bound_scope").is_none_or(str::is_empty) {
        errors.push("analog drift-bound scope must be explicit".to_owned());
    }

    let Some(scan_contract) = manifest.get("surface_scan_contract") else {
        errors.push("surface_scan_contract is required".to_owned());
        return Err(errors);
    };
    if scan_contract
        .get("files_per_workpiece")
        .and_then(Value::as_u64)
        != Some(7)
        || scan_contract.get("total_files").and_then(Value::as_u64) != Some(35)
        || scan_contract
            .get("layer_transitions_per_workpiece")
            .and_then(Value::as_u64)
            != Some(6)
        || scan_contract
            .get("total_layer_transitions")
            .and_then(Value::as_u64)
            != Some(30)
        || scan_contract
            .get("total_laser_line_profiles")
            .and_then(Value::as_u64)
            != Some(47_188)
        || scan_contract
            .get("points_per_profile")
            .and_then(Value::as_u64)
            != Some(2_048)
        || scan_contract
            .get("profile_trigger_hz")
            .and_then(Value::as_u64)
            != Some(50)
        || str_at(scan_contract, "coordinates_frame") != Some("welding_reference_frame")
        || bool_at(
            scan_contract,
            "profile_position_recorded_with_robot_position",
        ) != Some(true)
    {
        errors.push(
            "surface scan topology/coordinate metadata differs from the published article"
                .to_owned(),
        );
    }

    let Some(scan_transitions) = manifest
        .get("scan_state_transitions")
        .and_then(Value::as_array)
    else {
        errors.push("scan_state_transitions must be an array".to_owned());
        return Err(errors);
    };
    if scan_transitions.len() != 30 {
        errors.push(format!(
            "expected 30 scan transitions, found {}",
            scan_transitions.len()
        ));
    }
    let mut scan_artifacts = HashSet::new();
    let wid_numbers = ["615", "617", "630", "631", "632"];
    let mut transition_index = 0usize;
    for number in wid_numbers {
        for layer in 1..=6 {
            if let Some(transition) = scan_transitions.get(transition_index) {
                let expected_wid = format!("WID{number}");
                let expected_layer = format!("L{layer}");
                let expected_before = format!("WID{number}_L{layer}_T1_scan_pre.nc");
                let (expected_after, expected_storage) = if layer < 6 {
                    (
                        format!("WID{number}_L{}_T1_scan_pre.nc", layer + 1),
                        "next_layer_scan_pre",
                    )
                } else {
                    (
                        format!("WID{number}_L6_T1_scan_post.nc"),
                        "same_layer_scan_post",
                    )
                };
                if str_at(transition, "workpiece_id") != Some(expected_wid.as_str())
                    || str_at(transition, "layer_id") != Some(expected_layer.as_str())
                    || str_at(transition, "before_scan_artifact") != Some(expected_before.as_str())
                    || str_at(transition, "after_scan_artifact") != Some(expected_after.as_str())
                    || str_at(transition, "after_scan_storage") != Some(expected_storage)
                {
                    errors.push(format!(
                        "{expected_wid} L{layer}: before/after scan state mapping is invalid"
                    ));
                }
                scan_artifacts.insert(expected_before);
                scan_artifacts.insert(expected_after);
            }
            transition_index += 1;
        }
    }
    if scan_artifacts.len() != 35 {
        errors.push(format!(
            "expected 35 unique scan artifacts, found {}",
            scan_artifacts.len()
        ));
    }

    let Some(controls) = manifest.get("controls").and_then(Value::as_array) else {
        return Err(vec!["controls must be an array".to_owned()]);
    };
    if controls.len() != 40 {
        errors.push(format!(
            "expected 40 metadata controls, found {}",
            controls.len()
        ));
    }
    let allowed_dispositions = [
        "METADATA_ONLY_ADMISSION_VALID",
        "BLOCK_RIGHTS",
        "BLOCK_SOURCE_IDENTITY",
        "BLOCK_SOURCE_INTEGRITY",
        "BLOCK_LAYER_TOPOLOGY",
        "BLOCK_LAYER_SEMANTICS",
        "BLOCK_ARTIFACT_ROLES",
        "BLOCK_T85_MISSINGNESS",
        "BLOCK_T85_SEMANTICS",
        "BLOCK_TIMEBASE",
        "BLOCK_VIDEO_COMPARABILITY",
        "BLOCK_SCAN_SEMANTICS",
        "BLOCK_SCAN_GEOMETRY",
        "BLOCK_SPLIT_LEAKAGE",
        "BLOCK_SAMPLE_INDEPENDENCE",
        "BLOCK_CAUSAL_CLAIM",
        "BLOCK_CLAIM_PROMOTION",
        "BLOCK_DERIVATION_PROVENANCE",
        "BLOCK_EVIDENCE_CLASS",
        "BLOCK_STATISTICAL_SCOPE",
    ];
    let mut control_ids = HashSet::new();
    let mut positive = 0usize;
    let mut negative = 0usize;
    for (index, control) in controls.iter().enumerate() {
        let Some(control_id) = str_at(control, "control_id") else {
            errors.push(format!("controls[{index}].control_id must be a string"));
            continue;
        };
        let expected_control_id = format!("TGMAW-{:03}", index + 1);
        if control_id != expected_control_id {
            errors.push(format!(
                "controls[{index}] expected {expected_control_id}, found {control_id}"
            ));
        }
        if !control_ids.insert(control_id.to_owned()) {
            errors.push(format!("duplicate control ID {control_id}"));
        }
        let expected_class = if index == 0 {
            "positive_contract_control"
        } else {
            "negative_control"
        };
        if str_at(control, "class") != Some(expected_class) {
            errors.push(format!("{control_id}: expected class {expected_class}"));
        }
        match str_at(control, "class") {
            Some("positive_contract_control") => positive += 1,
            Some("negative_control") => negative += 1,
            _ => errors.push(format!("{control_id}: unknown control class")),
        }
        if index == 0
            && str_at(control, "expected_disposition") != Some("METADATA_ONLY_ADMISSION_VALID")
        {
            errors.push("positive control must remain metadata-only admission".to_owned());
        }
        if str_at(control, "expected_disposition")
            .is_none_or(|value| !allowed_dispositions.contains(&value))
        {
            errors.push(format!("{control_id}: unknown expected disposition"));
        }
        for field in ["mutation", "rationale"] {
            if str_at(control, field).is_none_or(str::is_empty) {
                errors.push(format!("{control_id}: {field} must be non-empty"));
            }
        }
    }
    if positive != 1 || negative != 39 {
        errors.push(format!(
            "expected 1 positive / 39 negative controls, found {positive}/{negative}"
        ));
    }
    if str_at(&manifest, "claim_ceiling").is_none_or(str::is_empty) {
        errors.push("claim ceiling must be explicit".to_owned());
    }

    if errors.is_empty() {
        Ok(Summary {
            workpieces: workpieces.len(),
            layers: layer_total,
            controls: controls.len(),
            positive,
            negative,
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let manifest_path = args.next().unwrap_or_else(|| DEFAULT_MANIFEST.to_owned());
    let schema_path = args.next().unwrap_or_else(|| DEFAULT_SCHEMA.to_owned());
    if args.next().is_some() {
        eprintln!("usage: shipyard-tandem-gmaw-admission [MANIFEST] [SCHEMA]");
        return ExitCode::from(2);
    }
    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read schema {schema_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let manifest_text = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read manifest {manifest_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match validate_manifest(&schema_text, &manifest_text) {
        Ok(summary) => {
            println!(
                concat!(
                    "PASS: {workpieces} WIDs, {layers} layer slots, {controls} controls ",
                    "({positive} positive / {negative} negative); raw rights pending"
                ),
                workpieces = summary.workpieces,
                layers = summary.layers,
                controls = summary.controls,
                positive = summary.positive,
                negative = summary.negative
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("FAIL: {} dataset-admission violation(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_TEXT: &str =
        include_str!("../../../../docs/tech/SHIPYARD_TANDEM_GMAW_DATA_ADMISSION_V0_1.schema.json");
    const MANIFEST_TEXT: &str =
        include_str!("../../../../benchmarks/shipyard/tandem-gmaw/data-admission-v0.1.json");

    fn manifest_value() -> Value {
        serde_json::from_str(MANIFEST_TEXT).expect("manifest must parse")
    }

    #[test]
    fn frozen_metadata_baseline_is_admitted_without_raw_file_rights() {
        let summary = validate_manifest(SCHEMA_TEXT, MANIFEST_TEXT).expect("baseline should pass");
        assert_eq!(
            summary,
            Summary {
                workpieces: 5,
                layers: 30,
                controls: 40,
                positive: 1,
                negative: 39,
            }
        );
    }

    #[test]
    fn open_badge_does_not_promote_blank_license_to_reuse_permission() {
        let mut manifest = manifest_value();
        manifest["rights_gate"]["redistribution_allowed"] = Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn blank_t85_field_is_never_imputed_to_zero() {
        let mut manifest = manifest_value();
        manifest["workpieces"][0]["layers"][2]["t85_field"]["value_ingestion_status"] =
            Value::String("INGESTED".to_owned());
        manifest["workpieces"][0]["layers"][2]["t85_field"]["raw_value"] = Value::from(0);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn tandem_trailing_power_source_role_is_required() {
        let mut manifest = manifest_value();
        manifest["workpieces"][0]["layers"][1]["expected_artifact_roles"] = Value::Array(vec![
            Value::String("robot_motion".to_owned()),
            Value::String("process_voltage_current".to_owned()),
            Value::String("power_source_leading".to_owned()),
            Value::String("scan_pre".to_owned()),
            Value::String("video".to_owned()),
        ]);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn final_scan_is_not_falsely_promoted_to_every_layer() {
        let mut manifest = manifest_value();
        manifest["workpieces"][0]["layers"][1]["expected_artifact_roles"] = Value::Array(vec![
            Value::String("robot_motion".to_owned()),
            Value::String("process_voltage_current".to_owned()),
            Value::String("power_source_leading".to_owned()),
            Value::String("power_source_trailing".to_owned()),
            Value::String("scan_pre".to_owned()),
            Value::String("scan_post".to_owned()),
            Value::String("video".to_owned()),
        ]);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn random_layer_split_is_not_admissible() {
        let mut manifest = manifest_value();
        manifest["experiment_design"]["random_layer_split_allowed"] = Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn next_layer_pre_scan_is_the_previous_layer_after_state() {
        let mut manifest = manifest_value();
        manifest["scan_state_transitions"][0]["after_scan_artifact"] =
            Value::String("WID615_L1_T1_scan_post.nc".to_owned());
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn source_reported_missing_t85_records_cannot_be_imputed() {
        let mut manifest = manifest_value();
        manifest["workpieces"][2]["layers"][5]["t85_field"]["raw_value"] = Value::from(0.0);
        manifest["workpieces"][2]["layers"][5]["t85_field"]["source_declared_availability"] =
            Value::String("NOT_LISTED_AS_MISSING".to_owned());
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn article_cc_by_license_does_not_authorize_raw_archive_reuse() {
        let mut manifest = manifest_value();
        manifest["rights_gate"]["redistribution_allowed"] = Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn a_common_clock_does_not_mean_a_common_sample_interval() {
        let mut manifest = manifest_value();
        manifest["clock_contract"]["file_minimum_sample_intervals_ms"]["power_source_files"] =
            Value::from(0.1);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn source_article_license_does_not_change_raw_archive_rights() {
        let mut manifest = manifest_value();
        manifest["rights_gate"]["raw_data_license_still_pending"] = Value::Bool(false);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn root_pass_cannot_be_promoted_into_tandem_only_analysis() {
        let mut manifest = manifest_value();
        manifest["experiment_design"]["tandem_only_analysis_excludes_root_layer"] =
            Value::Bool(false);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn repeat_comparator_is_not_an_independent_replicate() {
        let mut manifest = manifest_value();
        manifest["experiment_design"]["repeat_comparator"]["independent_replicate"] =
            Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn matched_mode_pair_cannot_be_promoted_to_causal_replication() {
        let mut manifest = manifest_value();
        manifest["experiment_design"]["matched_mode_pair"]["supports_causal_generalization"] =
            Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn duplicate_wid_is_rejected() {
        let mut manifest = manifest_value();
        manifest["workpieces"][1]["workpiece_id"] = Value::String("WID615".to_owned());
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn mode_group_cannot_be_silently_reassigned() {
        let mut manifest = manifest_value();
        manifest["workpieces"][2]["tandem_mode"] = Value::String("PMC+PMC".to_owned());
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn rights_pending_raw_data_scoring_cannot_be_enabled() {
        let mut manifest = manifest_value();
        manifest["rights_gate"]["raw_data_scoring_allowed"] = Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn raw_files_cannot_be_promoted_before_rights_admission() {
        let mut manifest = manifest_value();
        manifest["source_record"]["raw_files_vendored"] = Value::Bool(true);
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }
}
