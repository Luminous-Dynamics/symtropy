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
    actual.iter().map(String::as_str).eq(expected.iter().copied())
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
        errors.push(
            "license field must remain null until explicit terms are recorded".to_owned(),
        );
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
    {
        errors.push(
            "five-workpiece / six-layer / WID-grouped study design boundary violated"
                .to_owned(),
        );
    }

    let expected_modes = [
        ("WID615", "PMC+CMT"),
        ("WID617", "PMC+PMC"),
        ("WID630", "CMT+CMT"),
        ("WID631", "PMC+PMC"),
        ("WID632", "PMC+CMT"),
    ];
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
                errors.push(format!("workpieces[{w_index}].workpiece_id must be a string"));
                continue;
            }
        };
        if actual_wids.contains(&wid) {
            errors.push(format!("duplicate workpiece ID {wid}"));
        }
        actual_wids.push(wid.clone());

        let expected_mode = expected_modes
            .iter()
            .find(|(expected_wid, _)| *expected_wid == wid)
            .map(|(_, mode)| *mode);
        if expected_mode.is_none() || str_at(workpiece, "tandem_mode") != expected_mode {
            errors.push(format!("{wid}: unexpected or missing tandem mode assignment"));
        }

        let Some(layers) = workpiece.get("layers").and_then(Value::as_array) else {
            errors.push(format!("{wid}: layers must be an array"));
            continue;
        };
        if layers.len() != 6 {
            errors.push(format!("{wid}: expected six layers, found {}", layers.len()));
        }
        layer_total += layers.len();

        for (layer_index, layer) in layers.iter().enumerate() {
            let expected_order = (layer_index + 1) as u64;
            let expected_id = format!("L{expected_order}");
            let expected_class = if layer_index == 0 { "single_wire_root" } else { "tandem" };
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
            if str_at(t85, "value_ingestion_status") != Some("NOT_INGESTED")
                || !t85.get("raw_value").is_some_and(Value::is_null)
                || str_at(t85, "source_declared_unit") != Some("s")
                || str_at(t85, "semantic_definition_status") != Some("UNRESOLVED")
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
        errors.push(format!("expected 30 metadata layer slots, found {layer_total}"));
    }

    let Some(controls) = manifest.get("controls").and_then(Value::as_array) else {
        return Err(vec!["controls must be an array".to_owned()]);
    };
    if controls.len() != 31 {
        errors.push(format!("expected 31 metadata controls, found {}", controls.len()));
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
    if positive != 1 || negative != 30 {
        errors.push(format!(
            "expected 1 positive / 30 negative controls, found {positive}/{negative}"
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

    const SCHEMA_TEXT: &str = include_str!(
        "../../../../docs/tech/SHIPYARD_TANDEM_GMAW_DATA_ADMISSION_V0_1.schema.json"
    );
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
                controls: 31,
                positive: 1,
                negative: 30,
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
        manifest["workpieces"][0]["layers"][1]["expected_artifact_roles"] =
            Value::Array(vec![
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
    fn duplicate_wid_is_rejected() {
        let mut manifest = manifest_value();
        manifest["workpieces"][1]["workpiece_id"] =
            Value::String("WID615".to_owned());
        let serialized = serde_json::to_string(&manifest).expect("serialize mutation");
        assert!(validate_manifest(SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn mode_group_cannot_be_silently_reassigned() {
        let mut manifest = manifest_value();
        manifest["workpieces"][2]["tandem_mode"] =
            Value::String("PMC+PMC".to_owned());
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
