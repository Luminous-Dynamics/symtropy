use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const DEFAULT_CORPUS: &str = "docs/medical-robotics/medical-robot-tool-corpus-v0.1.json";
const CORPUS_SOURCE: &[u8] =
    include_bytes!("../../../docs/medical-robotics/medical-robot-tool-corpus-v0.1.json");
const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");
const CLAIM_CEILING: &str = "Synthetic tool-manifest qualification only. No CAD verification, hardware safety, cleaning validation, clinical performance, or regulatory claim.";

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn num_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite())
}

fn push_findings(findings: &mut BTreeSet<String>, code: &str) {
    findings.insert(code.to_owned());
}

fn unknown_fields(value: &Value, allowed: &[&str], findings: &mut BTreeSet<String>) {
    match value.as_object() {
        Some(object) => {
            if object.keys().any(|key| !allowed.contains(&key.as_str())) {
                push_findings(findings, "UNKNOWN_FIELD");
            }
        }
        None => push_findings(findings, "MANIFEST_INCOMPLETE"),
    }
}

fn has_required_keys(value: &Value, required: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|object| required.iter().all(|key| object.contains_key(*key)))
}

fn valid_box(value: &Value) -> bool {
    let Some(items) = value.as_array() else {
        return false;
    };
    items.len() == 3
        && items.iter().all(|item| {
            item.as_f64()
                .is_some_and(|number| number.is_finite() && number > 0.0 && number <= 2000.0)
        })
}

fn validate_tool(tool: &Value) -> Vec<String> {
    let mut findings = BTreeSet::new();
    const TOP_FIELDS: &[&str] = &[
        "schema_version",
        "tool_id",
        "revision",
        "tool_kind",
        "intended_task",
        "contact_class",
        "design_stage",
        "evidence_class",
        "mechanical",
        "hygiene",
        "electrical",
        "privacy",
        "authority",
    ];
    unknown_fields(tool, TOP_FIELDS, &mut findings);

    if !tool.is_object()
        || !has_required_keys(
            tool,
            &[
                "schema_version",
                "tool_id",
                "revision",
                "tool_kind",
                "intended_task",
                "contact_class",
                "design_stage",
                "evidence_class",
                "mechanical",
                "hygiene",
                "electrical",
                "privacy",
                "authority",
            ],
        )
    {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }

    if str_at(tool, "schema_version") != Some("medical-robot-tool-manifest-v0") {
        push_findings(&mut findings, "SCHEMA_VERSION_UNSUPPORTED");
    }
    if str_at(tool, "tool_id").is_none_or(|value| value.trim().is_empty()) {
        push_findings(&mut findings, "TOOL_ID_INVALID");
    }
    if tool.get("revision").and_then(Value::as_u64).is_none_or(|value| value == 0) {
        push_findings(&mut findings, "REVISION_INVALID");
    }
    if str_at(tool, "design_stage") != Some("concept") {
        push_findings(&mut findings, "DESIGN_STAGE_UNSUPPORTED");
    }
    if str_at(tool, "evidence_class") != Some("synthetic_design_proposal") {
        push_findings(&mut findings, "EVIDENCE_CLASS_UNSUPPORTED");
    }

    let kind = str_at(tool, "tool_kind").unwrap_or_default();
    let task = str_at(tool, "intended_task").unwrap_or_default();
    let contact = str_at(tool, "contact_class").unwrap_or_default();

    if ![
        "sealed_supply_gripper",
        "tray_carrier",
        "telepresence_head",
        "trainer_probe",
        "patient_transfer",
        "medication_dispenser",
    ]
    .contains(&kind)
    {
        push_findings(&mut findings, "TOOL_KIND_UNSUPPORTED");
    }

    if ![
        "sealed_supply_transfer",
        "tray_transfer",
        "telepresence_support",
        "training_mannequin_measurement",
        "patient_transfer",
        "medication_dispensation",
    ]
    .contains(&task)
    {
        push_findings(&mut findings, "TASK_UNSUPPORTED");
    }

    if !["none", "object_only", "mannequin_only", "patient_contact", "invasive"]
        .contains(&contact)
    {
        push_findings(&mut findings, "CONTACT_CLASS_UNSUPPORTED");
    }
    if ["patient_contact", "invasive"].contains(&contact) {
        push_findings(&mut findings, "PATIENT_CONTACT_NOT_AUTHORIZED");
    }

    match kind {
        "sealed_supply_gripper" if task == "sealed_supply_transfer" && contact == "object_only" => {}
        "tray_carrier" if task == "tray_transfer" && contact == "object_only" => {}
        "telepresence_head" if task == "telepresence_support" && contact == "none" => {}
        "trainer_probe"
            if task == "training_mannequin_measurement" && contact == "mannequin_only" => {}
        "patient_transfer" | "medication_dispenser" => {
            push_findings(&mut findings, "TOOL_KIND_OUT_OF_SCOPE");
            push_findings(&mut findings, "TASK_NOT_SUPPORTED");
        }
        _ => {
            if [
                "sealed_supply_gripper",
                "tray_carrier",
                "telepresence_head",
                "trainer_probe",
            ]
            .contains(&kind)
            {
                push_findings(&mut findings, "TASK_TOOL_MISMATCH");
                if (kind == "trainer_probe") != (contact == "mannequin_only") {
                    push_findings(&mut findings, "CONTACT_SCOPE_MISMATCH");
                }
            }
        }
    }

    let mechanical = tool.get("mechanical").unwrap_or(&Value::Null);
    const MECHANICAL_FIELDS: &[&str] = &[
        "mount_interface",
        "mass_g",
        "payload_g",
        "geometry_bbox_mm",
        "max_object_force_n",
        "max_torque_nm",
        "retention_fail_closed",
    ];
    unknown_fields(mechanical, MECHANICAL_FIELDS, &mut findings);
    if !has_required_keys(mechanical, MECHANICAL_FIELDS) {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }
    if str_at(mechanical, "mount_interface") != Some("SMF-TOOL-01") {
        push_findings(&mut findings, "MOUNT_INTERFACE_UNSUPPORTED");
    }
    let mass = num_at(mechanical, "mass_g");
    let payload = num_at(mechanical, "payload_g");
    let force = num_at(mechanical, "max_object_force_n");
    let torque = num_at(mechanical, "max_torque_nm");
    if mass.is_none_or(|value| !(1.0..=5000.0).contains(&value))
        || payload.is_none_or(|value| !(0.0..=20000.0).contains(&value))
        || force.is_none_or(|value| !(0.0..=50.0).contains(&value))
        || torque.is_none_or(|value| !(0.0..=20.0).contains(&value))
    {
        push_findings(&mut findings, "MECHANICAL_ENVELOPE_INVALID");
    }
    if !valid_box(mechanical.get("geometry_bbox_mm").unwrap_or(&Value::Null)) {
        push_findings(&mut findings, "GEOMETRY_OUT_OF_ENVELOPE");
    }
    if ["sealed_supply_gripper", "tray_carrier"].contains(&kind)
        && (bool_at(mechanical, "retention_fail_closed") != Some(true)
            || force.is_none_or(|value| value <= 0.0))
    {
        push_findings(&mut findings, "FAIL_CLOSED_RETENTION_REQUIRED");
    }

    let hygiene = tool.get("hygiene").unwrap_or(&Value::Null);
    const HYGIENE_FIELDS: &[&str] = &[
        "surface_materials_declared",
        "cleaning_claim",
        "single_use",
    ];
    unknown_fields(hygiene, HYGIENE_FIELDS, &mut findings);
    if !has_required_keys(hygiene, HYGIENE_FIELDS) {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }
    if bool_at(hygiene, "surface_materials_declared") != Some(true) {
        push_findings(&mut findings, "MATERIALS_UNDECLARED");
    }
    if str_at(hygiene, "cleaning_claim") != Some("not_validated") {
        push_findings(&mut findings, "HYGIENE_VALIDATION_CLAIM_UNSUPPORTED");
    }
    if bool_at(hygiene, "single_use").is_none() {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }

    let electrical = tool.get("electrical").unwrap_or(&Value::Null);
    const ELECTRICAL_FIELDS: &[&str] = &[
        "powered",
        "max_power_w",
        "protective_isolation_declared",
    ];
    unknown_fields(electrical, ELECTRICAL_FIELDS, &mut findings);
    if !has_required_keys(electrical, ELECTRICAL_FIELDS) {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }
    let powered = bool_at(electrical, "powered");
    let power = num_at(electrical, "max_power_w");
    let isolated = bool_at(electrical, "protective_isolation_declared");
    if powered.is_none()
        || power.is_none_or(|value| !(0.0..=100.0).contains(&value))
        || isolated.is_none()
        || (powered == Some(true) && (power.is_none_or(|value| value <= 0.0) || isolated != Some(true)))
        || (powered == Some(false) && power != Some(0.0))
    {
        push_findings(&mut findings, "POWER_ENVELOPE_INVALID");
    }

    let privacy = tool.get("privacy").unwrap_or(&Value::Null);
    const PRIVACY_FIELDS: &[&str] = &[
        "captures_audio",
        "captures_video",
        "physical_privacy_shutter",
        "default_capture_state",
    ];
    unknown_fields(privacy, PRIVACY_FIELDS, &mut findings);
    if !has_required_keys(privacy, PRIVACY_FIELDS) {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }
    let captures = bool_at(privacy, "captures_audio") == Some(true)
        || bool_at(privacy, "captures_video") == Some(true);
    if bool_at(privacy, "captures_audio").is_none()
        || bool_at(privacy, "captures_video").is_none()
        || bool_at(privacy, "physical_privacy_shutter").is_none()
        || str_at(privacy, "default_capture_state").is_none()
    {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }
    if captures
        && (bool_at(privacy, "physical_privacy_shutter") != Some(true)
            || str_at(privacy, "default_capture_state") != Some("disabled"))
    {
        push_findings(&mut findings, "PRIVACY_FAIL_CLOSED_REQUIRED");
    }

    let authority = tool.get("authority").unwrap_or(&Value::Null);
    const AUTHORITY_FIELDS: &[&str] = &[
        "simulation_only",
        "physical_actuation_authorized",
        "patient_contact_authorized",
        "clinical_use_authorized",
    ];
    unknown_fields(authority, AUTHORITY_FIELDS, &mut findings);
    if !has_required_keys(authority, AUTHORITY_FIELDS) {
        push_findings(&mut findings, "MANIFEST_INCOMPLETE");
    }
    if bool_at(authority, "simulation_only") != Some(true) {
        push_findings(&mut findings, "SIMULATION_ONLY_REQUIRED");
    }
    if bool_at(authority, "physical_actuation_authorized") != Some(false) {
        push_findings(&mut findings, "PHYSICAL_ACTUATION_AUTHORITY_FORBIDDEN");
    }
    if bool_at(authority, "patient_contact_authorized") != Some(false) {
        push_findings(&mut findings, "PATIENT_CONTACT_AUTHORITY_FORBIDDEN");
    }
    if bool_at(authority, "clinical_use_authorized") != Some(false) {
        push_findings(&mut findings, "CLINICAL_USE_AUTHORITY_FORBIDDEN");
    }

    findings.into_iter().collect()
}

struct CorpusReport {
    cases: usize,
    matched: usize,
    failures: Vec<String>,
}

fn run_corpus(root: &Value) -> Result<CorpusReport, Vec<String>> {
    let mut schema_errors = Vec::new();
    if str_at(root, "suite_id") != Some("MEDICAL_ROBOT_TOOL_GATE") {
        schema_errors.push("suite_id missing or unsupported".to_owned());
    }
    if str_at(root, "suite_version") != Some("0.1.0") {
        schema_errors.push("suite_version missing or unsupported".to_owned());
    }
    if str_at(root, "evidence_class") != Some("synthetic") {
        schema_errors.push("evidence_class must remain synthetic".to_owned());
    }
    if str_at(root, "execution_mode") != Some("manifest_qualification_only_no_hardware_authority") {
        schema_errors.push("execution_mode must prohibit hardware authority".to_owned());
    }
    if str_at(root, "claim_ceiling").is_none() {
        schema_errors.push("claim_ceiling is required".to_owned());
    }
    let cases = match root.get("cases").and_then(Value::as_array) {
        Some(cases) => cases,
        None => return Err(vec!["cases must be a JSON array".to_owned()]),
    };
    if root.get("expected_case_count").and_then(Value::as_u64) != Some(cases.len() as u64) {
        schema_errors.push("expected_case_count does not match corpus length".to_owned());
    }
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut ids = BTreeSet::new();
    let mut matched = 0;
    let mut failures = Vec::new();
    for case in cases {
        let case_id = str_at(case, "case_id").unwrap_or("<missing-case-id>");
        if !ids.insert(case_id.to_owned()) {
            schema_errors.push(format!("duplicate case_id '{case_id}'"));
            continue;
        }
        let manifest = case.get("manifest").unwrap_or(&Value::Null);
        let actual_findings = validate_tool(manifest);
        let expected_findings = case
            .get("expected_findings")
            .and_then(Value::as_array)
            .and_then(|items| items.iter().map(|item| item.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>());
        let expected_disposition = str_at(case, "expected_disposition");
        let actual_disposition = if actual_findings.is_empty() {
            "SIMULATION_ONLY"
        } else {
            "BLOCKED"
        };

        if expected_findings.is_none() || expected_disposition.is_none() {
            failures.push(format!("{case_id}: expected disposition/findings are malformed"));
            continue;
        }
        if actual_disposition == expected_disposition.unwrap_or_default()
            && actual_findings == expected_findings.unwrap_or_default()
        {
            matched += 1;
        } else {
            failures.push(format!(
                "{case_id}: actual={actual_disposition} findings={actual_findings:?}; expected={} findings={:?}",
                expected_disposition.unwrap_or("<missing>"),
                expected_findings.unwrap_or_default()
            ));
        }
    }
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }
    Ok(CorpusReport {
        cases: cases.len(),
        matched,
        failures,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn load_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read failed for '{path}': {error}"))?;
    let parsed = serde_json::from_slice(&bytes)
        .map_err(|error| format!("JSON rejected for '{path}': {error}"))?;
    Ok((bytes, parsed))
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    let (bytes, root) = match load_json(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    if path == DEFAULT_CORPUS && sha256_hex(&bytes) != sha256_hex(CORPUS_SOURCE) {
        eprintln!("runtime tool corpus differs from the corpus embedded in the compiled gate");
        process::exit(2);
    }
    let report = match run_corpus(&root) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("REJECT: {error}");
            }
            process::exit(2);
        }
    };
    let failed = !report.failures.is_empty();
    let receipt = json!({
        "receipt_schema": "medical-robot-tool-gate-receipt-v0",
        "tested_revision": env::var("CARE_FRAME_TESTED_SHA")
            .unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned()),
        "suite_id": str_at(&root, "suite_id").unwrap_or("unknown"),
        "suite_version": str_at(&root, "suite_version").unwrap_or("unknown"),
        "corpus_sha256": sha256_hex(&bytes),
        "embedded_corpus_sha256": sha256_hex(CORPUS_SOURCE),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "case_count": report.cases,
        "matched_expected_cases": report.matched,
        "failed_cases": report.failures.len(),
        "failures": report.failures,
        "clinical_use_authorized": false,
        "patient_contact_authorized": false,
        "physical_actuation_authorized": false,
        "claim_ceiling": CLAIM_CEILING
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serialization cannot fail")
    );
    if failed {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_slice(include_bytes!(
            "../../../docs/medical-robotics/medical-robot-tool-corpus-v0.1.json"
        ))
        .expect("tool-design corpus parses")
    }

    #[test]
    fn frozen_tool_design_corpus_matches_expected_dispositions() {
        let report = run_corpus(&corpus()).expect("corpus shape is valid");
        assert_eq!(report.cases, 16);
        assert_eq!(report.matched, 16);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    }

    #[test]
    fn each_authority_escalation_is_blocked() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();
        for (field, expected) in [
            ("physical_actuation_authorized", "PHYSICAL_ACTUATION_AUTHORITY_FORBIDDEN"),
            ("patient_contact_authorized", "PATIENT_CONTACT_AUTHORITY_FORBIDDEN"),
            ("clinical_use_authorized", "CLINICAL_USE_AUTHORITY_FORBIDDEN"),
        ] {
            let mut manifest = base.clone();
            manifest["authority"][field] = Value::Bool(true);
            let findings = validate_tool(&manifest);
            assert!(findings.iter().any(|finding| finding == expected), "{field}: {findings:?}");
        }
        let mut manifest = base;
        manifest["authority"]["simulation_only"] = Value::Bool(false);
        assert!(validate_tool(&manifest).iter().any(|finding| finding == "SIMULATION_ONLY_REQUIRED"));
    }

    #[test]
    fn unknown_fields_at_every_level_are_rejected() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();
        let mut top = base.clone();
        top["silent_autonomy"] = json!(true);
        assert!(validate_tool(&top).iter().any(|finding| finding == "UNKNOWN_FIELD"));

        let mut mechanical = base.clone();
        mechanical["mechanical"]["unbounded_force"] = json!(9999);
        assert!(validate_tool(&mechanical).iter().any(|finding| finding == "UNKNOWN_FIELD"));

        let mut privacy = base;
        privacy["privacy"]["capture_without_consent"] = json!(true);
        assert!(validate_tool(&privacy).iter().any(|finding| finding == "UNKNOWN_FIELD"));
    }

    #[test]
    fn no_contact_class_or_manifest_can_grant_patient_use() {
        let root = corpus();
        for case in root["cases"].as_array().expect("cases") {
            let findings = validate_tool(&case["manifest"]);
            assert!(!findings.is_empty() || bool_at(&case["manifest"]["authority"], "patient_contact_authorized") == Some(false));
            assert!(!findings.is_empty() || bool_at(&case["manifest"]["authority"], "clinical_use_authorized") == Some(false));
            assert!(!findings.is_empty() || bool_at(&case["manifest"]["authority"], "physical_actuation_authorized") == Some(false));
        }
        let mut manifest = root["cases"][0]["manifest"].clone();
        manifest["contact_class"] = json!("patient_contact");
        assert!(validate_tool(&manifest).iter().any(|finding| finding == "PATIENT_CONTACT_NOT_AUTHORIZED"));
    }

    #[test]
    fn privacy_capture_requires_shutter_and_capture_disabled_by_default() {
        let root = corpus();
        let mut no_shutter = root["cases"][2]["manifest"].clone();
        no_shutter["privacy"]["physical_privacy_shutter"] = Value::Bool(false);
        assert!(validate_tool(&no_shutter).iter().any(|finding| finding == "PRIVACY_FAIL_CLOSED_REQUIRED"));

        let mut live_default = root["cases"][2]["manifest"].clone();
        live_default["privacy"]["default_capture_state"] = json!("enabled");
        assert!(validate_tool(&live_default).iter().any(|finding| finding == "PRIVACY_FAIL_CLOSED_REQUIRED"));
    }

    #[test]
    fn hygiene_validation_cannot_be_claimed_by_a_concept_manifest() {
        let root = corpus();
        let mut manifest = root["cases"][0]["manifest"].clone();
        manifest["hygiene"]["cleaning_claim"] = json!("validated");
        assert!(validate_tool(&manifest).iter().any(|finding| finding == "HYGIENE_VALIDATION_CLAIM_UNSUPPORTED"));
    }

    #[test]
    fn geometry_mount_and_retention_mutations_fail_closed() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();
        let mut geometry = base.clone();
        geometry["mechanical"]["geometry_bbox_mm"] = json!([200, 0, 100]);
        assert!(validate_tool(&geometry).iter().any(|finding| finding == "GEOMETRY_OUT_OF_ENVELOPE"));

        let mut mount = base.clone();
        mount["mechanical"]["mount_interface"] = json!("UNKEYED_GENERIC_MOUNT");
        assert!(validate_tool(&mount).iter().any(|finding| finding == "MOUNT_INTERFACE_UNSUPPORTED"));

        let mut retention = base;
        retention["mechanical"]["retention_fail_closed"] = Value::Bool(false);
        assert!(validate_tool(&retention).iter().any(|finding| finding == "FAIL_CLOSED_RETENTION_REQUIRED"));
    }

    #[test]
    fn powered_tools_require_declared_protective_isolation() {
        let root = corpus();
        let mut manifest = root["cases"][0]["manifest"].clone();
        manifest["electrical"]["protective_isolation_declared"] = Value::Bool(false);
        assert!(validate_tool(&manifest).iter().any(|finding| finding == "POWER_ENVELOPE_INVALID"));
    }

    #[test]
    fn corpus_and_source_digests_are_bound_to_the_build() {
        assert_eq!(sha256_hex(AUDITOR_SOURCE).len(), 64);
        assert_eq!(sha256_hex(CORPUS_SOURCE).len(), 64);
        let parsed: Value = serde_json::from_slice(CORPUS_SOURCE).expect("embedded corpus");
        assert_eq!(
            sha256_hex(&serde_json::to_vec(&parsed).expect("canonical test serialization")),
            sha256_hex(&serde_json::to_vec(&corpus()).expect("canonical corpus serialization"))
        );
    }
}
