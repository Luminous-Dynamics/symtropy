use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

const PROPERTY_NAMES: &[&str] = &[
    "organization",
    "boundary",
    "energy_matter_throughput",
    "persistence",
    "homeostasis_regulation",
    "self_maintenance",
    "information_retention",
    "information_transfer",
    "reproduction",
    "heredity",
    "adaptation",
    "learning",
    "agency",
    "intelligence",
    "collective_intelligence",
    "sentience_consciousness",
];

const PROPERTY_STATES: &[&str] = &[
    "SUPPORTED_WITHIN_SCOPE",
    "CONTRADICTED_WITHIN_SCOPE",
    "INSUFFICIENT_EVIDENCE",
    "NOT_TESTED",
    "NOT_OBSERVABLE_WITH_CURRENT_INSTRUMENT",
    "MODEL_INADEQUATE",
];

const DISPOSITIONS: &[&str] = &[
    "KNOWN_BIOLOGICAL_REFERENCE",
    "ABIOTIC_CONTROL_REFERENCE",
    "PHYSICS_ONLY_REFERENCE",
    "OTHER_OR_UNRESOLVED",
    "NON_DETECTION_INCONCLUSIVE",
    "OUT_OF_DOMAIN",
];

const SCENARIOS: &[&str] = &[
    "KNOWN_BIOLOGICAL_REFERENCE",
    "ABIOTIC_SELF_ORGANIZING_CONTROL",
    "DUSTY_PLASMA_PHYSICS_REFERENCE",
    "UNKNOWN_HELD_OUT_GENERATOR",
    "TEMPORAL_UNDERSAMPLING_FALSE_NEGATIVE",
    "OOD_SENSOR_ENVIRONMENT",
];

fn get_str<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_str()
}

fn get_f64(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_f64()
}

fn get_bool(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_bool()
}

fn in_set(value: &str, allowed: &[&str]) -> bool {
    allowed.contains(&value)
}

fn validate_fraction(value: Option<f64>, name: &str, errors: &mut Vec<String>) {
    match value {
        Some(number) if (0.0..=1.0).contains(&number) => {}
        _ => errors.push(format!("{name} must be a finite fraction in [0, 1]")),
    }
}

fn validate_fixture(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if !value.is_object() {
        return vec!["fixture root must be a JSON object".to_owned()];
    }

    if get_str(value, &["schema_version"]) != Some("oli-m0-fixture-v0.1") {
        errors.push("schema_version must equal oli-m0-fixture-v0.1".to_owned());
    }

    let fixture_id = get_str(value, &["fixture_id"]).unwrap_or_default();
    if fixture_id.trim().is_empty() {
        errors.push("fixture_id must be a non-empty string".to_owned());
    }

    let scenario = get_str(value, &["scenario_family"]).unwrap_or_default();
    if !in_set(scenario, SCENARIOS) {
        errors.push(format!("unsupported scenario_family: {scenario}"));
    }

    let evidence_class = get_str(value, &["evidence_class"]).unwrap_or_default();
    if !in_set(
        evidence_class,
        &[
            "SYNTHETIC_KNOWN_TRUTH",
            "PUBLISHED_EXPERIMENT_REFERENCE_METADATA_ONLY",
            "SYNTHETIC_UNKNOWN_GENERATOR",
        ],
    ) {
        errors.push(format!("unsupported evidence_class: {evidence_class}"));
    }

    if get_str(value, &["physical_substrate"])
        .is_none_or(|text| text.trim().is_empty())
    {
        errors.push("physical_substrate must be a non-empty string".to_owned());
    }

    let boundary_status = get_str(value, &["system_boundary", "status"]).unwrap_or_default();
    if !in_set(
        boundary_status,
        &["DECLARED", "FRAGMENTED", "UNKNOWN", "NOT_APPLICABLE"],
    ) {
        errors.push(format!("unsupported system_boundary.status: {boundary_status}"));
    }
    if get_str(value, &["system_boundary", "description"])
        .is_none_or(|text| text.trim().is_empty())
    {
        errors.push("system_boundary.description must be a non-empty string".to_owned());
    }

    let units = value
        .get("measurement")
        .and_then(|measurement| measurement.get("units"))
        .and_then(Value::as_array);
    match units {
        Some(values) if !values.is_empty() => {
            if values.iter().any(|item| item.as_str().is_none_or(str::is_empty)) {
                errors.push("measurement.units must contain only non-empty strings".to_owned());
            }
        }
        _ => errors.push("measurement.units must be a non-empty array".to_owned()),
    }

    let unit_status = get_str(value, &["measurement", "unit_status"]).unwrap_or_default();
    if !in_set(unit_status, &["DECLARED", "UNKNOWN"]) {
        errors.push(format!("unsupported measurement.unit_status: {unit_status}"));
    }

    let calibration = get_str(value, &["measurement", "calibration_status"]).unwrap_or_default();
    if !in_set(
        calibration,
        &[
            "QUALIFIED_SYNTHETIC_REFERENCE",
            "SOURCE_REPORTED_NOT_INDEPENDENTLY_REPRODUCED",
            "NOT_ESTABLISHED",
            "UNQUALIFIED",
        ],
    ) {
        errors.push(format!("unsupported calibration_status: {calibration}"));
    }

    let modality = get_str(value, &["measurement", "modality"]).unwrap_or_default();
    if modality.trim().is_empty() {
        errors.push("measurement.modality must be a non-empty string".to_owned());
    }

    let window = get_f64(value, &["measurement", "temporal", "window_seconds"]);
    let cadence = get_f64(value, &["measurement", "temporal", "sampling_interval_seconds"]);
    let process_scale = get_f64(value, &["measurement", "temporal", "target_process_timescale_seconds"]);
    for (name, number) in [
        ("window_seconds", window),
        ("sampling_interval_seconds", cadence),
        ("target_process_timescale_seconds", process_scale),
    ] {
        if number.is_none_or(|number| !number.is_finite() || number <= 0.0) {
            errors.push(format!("measurement.temporal.{name} must be positive and finite"));
        }
    }
    validate_fraction(
        get_f64(value, &["measurement", "temporal", "coverage_fraction"]),
        "measurement.temporal.coverage_fraction",
        &mut errors,
    );
    validate_fraction(
        get_f64(value, &["measurement", "spatial", "footprint_fraction"]),
        "measurement.spatial.footprint_fraction",
        &mut errors,
    );
    if get_str(value, &["measurement", "spatial", "coordinate_frame"])
        .is_none_or(|text| text.trim().is_empty())
    {
        errors.push("measurement.spatial.coordinate_frame must be a non-empty string".to_owned());
    }

    let assessments = value.get("property_assessments");
    let Some(assessments) = assessments.and_then(Value::as_object) else {
        errors.push("property_assessments must be an object".to_owned());
        return errors;
    };

    for property in PROPERTY_NAMES {
        match assessments.get(*property).and_then(Value::as_str) {
            Some(state) if in_set(state, PROPERTY_STATES) => {}
            Some(state) => errors.push(format!(
                "property_assessments.{property} has unsupported state {state}"
            )),
            None => errors.push(format!(
                "property_assessments.{property} is missing or not a string"
            )),
        }
    }

    let disposition = get_str(value, &["expected_result", "disposition"]).unwrap_or_default();
    if !in_set(disposition, DISPOSITIONS) {
        errors.push(format!("unsupported expected_result.disposition: {disposition}"));
    }
    if get_bool(value, &["expected_result", "absence_inference_allowed"]).is_none() {
        errors.push("expected_result.absence_inference_allowed must be boolean".to_owned());
    }
    if get_str(value, &["expected_result", "reason"])
        .is_none_or(|text| text.trim().is_empty())
    {
        errors.push("expected_result.reason must be a non-empty string".to_owned());
    }

    if get_str(value, &["source", "reference"]).is_none()
        || get_str(value, &["source", "data_status"]).is_none()
    {
        errors.push("source.reference and source.data_status are required strings".to_owned());
    }

    let absence_allowed =
        get_bool(value, &["expected_result", "absence_inference_allowed"]).unwrap_or(false);

    match scenario {
        "KNOWN_BIOLOGICAL_REFERENCE" => {
            if evidence_class != "SYNTHETIC_KNOWN_TRUTH"
                || disposition != "KNOWN_BIOLOGICAL_REFERENCE"
            {
                errors.push("known biological fixture must be synthetic known-truth reference".to_owned());
            }
        }
        "ABIOTIC_SELF_ORGANIZING_CONTROL" => {
            if evidence_class != "SYNTHETIC_KNOWN_TRUTH"
                || disposition != "ABIOTIC_CONTROL_REFERENCE"
            {
                errors.push("abiotic control must have synthetic known-truth disposition".to_owned());
            }
            if assessments.get("organization").and_then(Value::as_str)
                != Some("SUPPORTED_WITHIN_SCOPE")
            {
                errors.push("abiotic hard negative must preserve supported self-organization".to_owned());
            }
        }
        "DUSTY_PLASMA_PHYSICS_REFERENCE" => {
            if evidence_class != "PUBLISHED_EXPERIMENT_REFERENCE_METADATA_ONLY"
                || disposition != "PHYSICS_ONLY_REFERENCE"
            {
                errors.push("dusty plasma fixture must remain a physics-only metadata reference".to_owned());
            }
            for property in [
                "self_maintenance",
                "information_retention",
                "reproduction",
                "heredity",
                "adaptation",
                "learning",
                "agency",
                "intelligence",
                "collective_intelligence",
                "sentience_consciousness",
            ] {
                if assessments.get(property).and_then(Value::as_str)
                    == Some("SUPPORTED_WITHIN_SCOPE")
                {
                    errors.push(format!(
                        "dusty-plasma physics result cannot support life/intelligence property {property}"
                    ));
                }
            }
            if absence_allowed {
                errors.push("physics-only dusty-plasma reference cannot authorize absence inference".to_owned());
            }
        }
        "UNKNOWN_HELD_OUT_GENERATOR" => {
            if evidence_class != "SYNTHETIC_UNKNOWN_GENERATOR"
                || disposition != "OTHER_OR_UNRESOLVED"
                || absence_allowed
            {
                errors.push("unknown generator must remain OTHER_OR_UNRESOLVED without absence inference".to_owned());
            }
            if assessments.values().any(|state| {
                state.as_str() == Some("SUPPORTED_WITHIN_SCOPE")
                    || state.as_str() == Some("CONTRADICTED_WITHIN_SCOPE")
            }) {
                errors.push("unknown generator cannot carry unsupported property conclusions".to_owned());
            }
        }
        "TEMPORAL_UNDERSAMPLING_FALSE_NEGATIVE" => {
            if disposition != "NON_DETECTION_INCONCLUSIVE" || absence_allowed {
                errors.push("temporal undersampling must remain inconclusive and block absence inference".to_owned());
            }
            if let (Some(cadence), Some(process_scale)) = (cadence, process_scale) {
                if cadence <= process_scale {
                    errors.push("temporal false-negative control must sample more slowly than its target process timescale".to_owned());
                }
            }
            if get_f64(value, &["measurement", "temporal", "coverage_fraction"])
                .is_none_or(|fraction| fraction >= 0.5)
            {
                errors.push("temporal false-negative control requires less than 0.5 temporal coverage".to_owned());
            }
        }
        "OOD_SENSOR_ENVIRONMENT" => {
            if disposition != "OUT_OF_DOMAIN"
                || unit_status != "UNKNOWN"
                || calibration != "UNQUALIFIED"
                || absence_allowed
            {
                errors.push("OOD fixture must remain out-of-domain with unknown units, unqualified calibration, and no absence inference".to_owned());
            }
            if assessments.values().any(|state| {
                state.as_str() == Some("SUPPORTED_WITHIN_SCOPE")
                    || state.as_str() == Some("CONTRADICTED_WITHIN_SCOPE")
            }) {
                errors.push("OOD fixture cannot make in-domain property conclusions".to_owned());
            }
        }
        _ => {}
    }

    if evidence_class == "PUBLISHED_EXPERIMENT_REFERENCE_METADATA_ONLY"
        && get_str(value, &["source", "data_status"])
            != Some("METADATA_ONLY_RIGHTS_ADMISSION_PENDING")
    {
        errors.push("published experimental reference must remain metadata-only until data rights are admitted".to_owned());
    }

    errors
}

fn validate_collection(fixtures: &[(String, Value)]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();

    for (name, fixture) in fixtures {
        for error in validate_fixture(fixture) {
            errors.push(format!("{name}: {error}"));
        }
        let id = get_str(fixture, &["fixture_id"]).unwrap_or_default();
        if !id.is_empty() && !seen.insert(id.to_owned()) {
            errors.push(format!("{name}: duplicate fixture_id {id}"));
        }
    }
    errors
}

fn load_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let directory = env::args().nth(1).unwrap_or_else(|| "fixtures".to_owned());
    let root = Path::new(&directory);
    let entries = fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();

    if paths.is_empty() {
        return Err(format!("no .json fixtures found in {}", root.display()));
    }

    let mut fixtures = Vec::new();
    for path in &paths {
        let value = load_json(path)?;
        fixtures.push((
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<non-utf8-name>")
                .to_owned(),
            value,
        ));
    }

    let errors = validate_collection(&fixtures);
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("FAIL: {error}");
        }
        return Err(format!("{} validation error(s)", errors.len()));
    }

    println!("OLI M0 semantic validation: PASS");
    println!("fixtures_checked={}", fixtures.len());
    println!("schema_version=oli-m0-fixture-v0.1");
    println!("claim_ceiling=synthetic_contract_semantics_only");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("OLI M0 semantic validation: FAIL: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> Vec<(String, Value)> {
        vec![
            ("bio".into(), serde_json::from_str(include_str!("../fixtures/oli-m0-bio-001.json")).unwrap()),
            ("abiotic".into(), serde_json::from_str(include_str!("../fixtures/oli-m0-abiotic-001.json")).unwrap()),
            ("dusty_plasma".into(), serde_json::from_str(include_str!("../fixtures/oli-m0-dusty-plasma-001.json")).unwrap()),
            ("unknown".into(), serde_json::from_str(include_str!("../fixtures/oli-m0-unknown-001.json")).unwrap()),
            ("temporal_false_negative".into(), serde_json::from_str(include_str!("../fixtures/oli-m0-temporal-fn-001.json")).unwrap()),
            ("ood_sensor".into(), serde_json::from_str(include_str!("../fixtures/oli-m0-ood-sensor-001.json")).unwrap()),
        ]
    }

    #[test]
    fn all_six_frozen_fixtures_pass_semantic_validation() {
        let errors = validate_collection(&fixtures());
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(fixtures().len(), 6);
    }

    #[test]
    fn dusty_plasma_physics_fit_cannot_be_promoted_to_intelligence() {
        let mut value: Value =
            serde_json::from_str(include_str!("../fixtures/oli-m0-dusty-plasma-001.json")).unwrap();
        value["property_assessments"]["intelligence"] =
            Value::String("SUPPORTED_WITHIN_SCOPE".to_owned());
        assert!(!validate_fixture(&value).is_empty());
    }

    #[test]
    fn temporal_non_detection_cannot_become_absence() {
        let mut value: Value =
            serde_json::from_str(include_str!("../fixtures/oli-m0-temporal-fn-001.json")).unwrap();
        value["expected_result"]["absence_inference_allowed"] = Value::Bool(true);
        assert!(!validate_fixture(&value).is_empty());
    }

    #[test]
    fn out_of_domain_sensor_cannot_make_in_domain_claims() {
        let mut value: Value =
            serde_json::from_str(include_str!("../fixtures/oli-m0-ood-sensor-001.json")).unwrap();
        value["property_assessments"]["organization"] =
            Value::String("SUPPORTED_WITHIN_SCOPE".to_owned());
        assert!(!validate_fixture(&value).is_empty());
    }

    #[test]
    fn duplicate_fixture_ids_are_rejected() {
        let mut fixtures = fixtures();
        fixtures[1].1["fixture_id"] = fixtures[0].1["fixture_id"].clone();
        assert!(validate_collection(&fixtures).iter().any(|error| error.contains("duplicate fixture_id")));
    }

    #[test]
    fn missing_measurement_units_are_rejected() {
        let mut value: Value =
            serde_json::from_str(include_str!("../fixtures/oli-m0-bio-001.json")).unwrap();
        value["measurement"]["units"] = Value::Array(Vec::new());
        assert!(!validate_fixture(&value).is_empty());
    }
}
