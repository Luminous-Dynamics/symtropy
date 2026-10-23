use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const DEFAULT_CORPUS: &str = "docs/medical-robotics/medical-robot-tool-corpus-v0.1.json";
const EXPECTED_CASE_COUNT: usize = 16;
const NULL_VALUE: Value = Value::Null;
const CORPUS_SOURCE: &[u8] =
    include_bytes!("../../../docs/medical-robotics/medical-robot-tool-corpus-v0.1.json");
const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");
const CLAIM_CEILING: &str = concat!(
    "Synthetic tool-manifest qualification only. No CAD verification, hardware safety, ",
    "cleaning validation, clinical performance, or regulatory claim.",
);

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
        "sealed_supply_gripper"
            if task == "sealed_supply_transfer" && contact == "object_only" => {}
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

    let mechanical = tool.get("mechanical").unwrap_or(&NULL_VALUE);
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
        || (["sealed_supply_gripper", "tray_carrier"].contains(&kind)
            && force.is_none_or(|value| value <= 0.0))
        || torque.is_none_or(|value| !(0.0..=20.0).contains(&value))
    {
        push_findings(&mut findings, "MECHANICAL_ENVELOPE_INVALID");
    }
    if !valid_box(mechanical.get("geometry_bbox_mm").unwrap_or(&NULL_VALUE)) {
        push_findings(&mut findings, "GEOMETRY_OUT_OF_ENVELOPE");
    }
    if ["sealed_supply_gripper", "tray_carrier"].contains(&kind)
        && (bool_at(mechanical, "retention_fail_closed") != Some(true)
            || force.is_none_or(|value| value <= 0.0))
    {
        push_findings(&mut findings, "FAIL_CLOSED_RETENTION_REQUIRED");
    }

    let hygiene = tool.get("hygiene").unwrap_or(&NULL_VALUE);
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

    let electrical = tool.get("electrical").unwrap_or(&NULL_VALUE);
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
        || (powered == Some(true)
            && (power.is_none_or(|value| value <= 0.0) || isolated != Some(true)))
        || (powered == Some(false) && power != Some(0.0))
    {
        push_findings(&mut findings, "POWER_ENVELOPE_INVALID");
    }

    let privacy = tool.get("privacy").unwrap_or(&NULL_VALUE);
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

    let authority = tool.get("authority").unwrap_or(&NULL_VALUE);
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
    let allowed_root_fields = [
        "suite_id",
        "suite_version",
        "evidence_class",
        "purpose",
        "claim_ceiling",
        "execution_mode",
        "expected_case_count",
        "cases",
    ];
    let Some(root_object) = root.as_object() else {
        return Err(vec!["corpus root must be a JSON object".to_owned()]);
    };
    if root_object
        .keys()
        .any(|key| !allowed_root_fields.contains(&key.as_str()))
    {
        schema_errors.push("unknown corpus root field".to_owned());
    }
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
    if cases.len() != EXPECTED_CASE_COUNT {
        schema_errors.push(format!(
            "v0 corpus requires exactly {EXPECTED_CASE_COUNT} cases, got {}",
            cases.len()
        ));
    }
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut ids = BTreeSet::new();
    let mut matched = 0;
    let mut failures = Vec::new();
    for case in cases {
        let allowed_case_fields = [
            "case_id",
            "manifest",
            "expected_disposition",
            "expected_findings",
        ];
        let Some(case_object) = case.as_object() else {
            schema_errors.push("case entry must be a JSON object".to_owned());
            continue;
        };
        if case_object
            .keys()
            .any(|key| !allowed_case_fields.contains(&key.as_str()))
        {
            schema_errors.push("unknown case field".to_owned());
            continue;
        }
        let case_id = str_at(case, "case_id").unwrap_or("<missing-case-id>");
        if !ids.insert(case_id.to_owned()) {
            schema_errors.push(format!("duplicate case_id '{case_id}'"));
            continue;
        }
        let manifest = case.get("manifest").unwrap_or(&NULL_VALUE);
        let actual_findings = validate_tool(manifest);
        let expected_findings = case
            .get("expected_findings")
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            });
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
        let expected_disposition = expected_disposition.expect("validated above");
        let expected_findings = expected_findings.expect("validated above");
        if actual_disposition == expected_disposition
            && actual_findings.as_slice() == expected_findings.as_slice()
        {
            matched += 1;
        } else {
            failures.push(format!(
                "{case_id}: actual={} findings={:?}; expected={} findings={:?}",
                actual_disposition, actual_findings, expected_disposition, expected_findings
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

fn tool_template(tool_kind: &str) -> Option<Value> {
    let (
        task,
        contact_class,
        mass_g,
        payload_g,
        geometry_bbox_mm,
        max_object_force_n,
        max_torque_nm,
        powered,
        max_power_w,
        captures_audio,
        captures_video,
    ) = match tool_kind {
        "sealed_supply_gripper" => (
            "sealed_supply_transfer",
            "object_only",
            1500,
            1500,
            [250, 120, 180],
            15.0,
            1.5,
            true,
            20,
            false,
            false,
        ),
        "tray_carrier" => (
            "tray_transfer",
            "object_only",
            700,
            2000,
            [420, 280, 80],
            10.0,
            2.0,
            false,
            0,
            false,
            false,
        ),
        "telepresence_head" => (
            "telepresence_support",
            "none",
            1000,
            0,
            [220, 150, 180],
            0.0,
            0.0,
            true,
            25,
            true,
            true,
        ),
        "trainer_probe" => (
            "training_mannequin_measurement",
            "mannequin_only",
            500,
            100,
            [160, 60, 50],
            3.0,
            0.4,
            true,
            20,
            false,
            false,
        ),
        _ => return None,
    };

    Some(json!({
        "schema_version": "medical-robot-tool-manifest-v0",
        "tool_id": format!("CARE-FRAME-{tool_kind}-CONCEPT-001"),
        "revision": 1,
        "tool_kind": tool_kind,
        "intended_task": task,
        "contact_class": contact_class,
        "design_stage": "concept",
        "evidence_class": "synthetic_design_proposal",
        "mechanical": {
            "mount_interface": "SMF-TOOL-01",
            "mass_g": mass_g,
            "payload_g": payload_g,
            "geometry_bbox_mm": geometry_bbox_mm,
            "max_object_force_n": max_object_force_n,
            "max_torque_nm": max_torque_nm,
            "retention_fail_closed": true
        },
        "hygiene": {
            "surface_materials_declared": true,
            "cleaning_claim": "not_validated",
            "single_use": false
        },
        "electrical": {
            "powered": powered,
            "max_power_w": max_power_w,
            "protective_isolation_declared": true
        },
        "privacy": {
            "captures_audio": captures_audio,
            "captures_video": captures_video,
            "physical_privacy_shutter": true,
            "default_capture_state": "disabled"
        },
        "authority": {
            "simulation_only": true,
            "physical_actuation_authorized": false,
            "patient_contact_authorized": false,
            "clinical_use_authorized": false
        }
    }))
}

fn write_or_print_template(tool_kind: &str, output_path: Option<&str>) {
    let Some(manifest) = tool_template(tool_kind) else {
        eprintln!("unsupported tool template: {tool_kind}");
        process::exit(2);
    };
    let serialized =
        serde_json::to_string_pretty(&manifest).expect("tool template serialization cannot fail");
    if let Some(path) = output_path {
        if let Err(error) = fs::write(path, format!("{serialized}\n")) {
            eprintln!("failed to write tool template '{path}': {error}");
            process::exit(2);
        }
    } else {
        println!("{serialized}");
    }
}

const MORPHOLOGY_PATH: &str =
    "docs/medical-robotics/careframe-wheeled-upper-body-morphology-v0.json";
const MORPHOLOGY_SOURCE: &[u8] =
    include_bytes!(
        "../../../docs/medical-robotics/careframe-wheeled-upper-body-morphology-v0.json"
    );

fn finite_vec3(value: &Value) -> Option<[f64; 3]> {
    let items = value.as_array()?;
    if items.len() != 3 {
        return None;
    }
    let x = items[0].as_f64().filter(|value| value.is_finite())?;
    let y = items[1].as_f64().filter(|value| value.is_finite())?;
    let z = items[2].as_f64().filter(|value| value.is_finite())?;
    Some([x, y, z])
}

fn morphology_unknown_fields(value: &Value, allowed: &[&str], findings: &mut BTreeSet<String>) {
    match value.as_object() {
        Some(object) => {
            if object.keys().any(|key| !allowed.contains(&key.as_str())) {
                push_findings(findings, "MORPHOLOGY_UNKNOWN_FIELD");
            }
        }
        None => push_findings(findings, "MORPHOLOGY_INCOMPLETE"),
    }
}

fn morphology_required_fields(value: &Value, required: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|object| required.iter().all(|key| object.contains_key(*key)))
}

fn validate_morphology(model: &Value) -> Vec<String> {
    let mut findings = BTreeSet::new();
    const ROOT_FIELDS: &[&str] = &[
        "schema_version",
        "model_id",
        "revision",
        "morphology_kind",
        "evidence_class",
        "geometry_fidelity",
        "units",
        "root_link",
        "links",
        "joints",
        "controller",
        "authority",
        "caveats",
    ];
    morphology_unknown_fields(model, ROOT_FIELDS, &mut findings);
    if !morphology_required_fields(model, ROOT_FIELDS) {
        push_findings(&mut findings, "MORPHOLOGY_INCOMPLETE");
    }
    if str_at(model, "schema_version") != Some("careframe-morphology-v0") {
        push_findings(&mut findings, "MORPHOLOGY_SCHEMA_UNSUPPORTED");
    }
    if str_at(model, "model_id").is_none_or(|id| id.trim().is_empty()) {
        push_findings(&mut findings, "MORPHOLOGY_MODEL_ID_INVALID");
    }
    if model.get("revision").and_then(Value::as_u64).is_none_or(|value| value == 0) {
        push_findings(&mut findings, "MORPHOLOGY_REVISION_INVALID");
    }
    if str_at(model, "morphology_kind") != Some("wheeled_upper_body_humanoid") {
        push_findings(&mut findings, "MORPHOLOGY_KIND_UNSUPPORTED");
    }
    if str_at(model, "evidence_class") != Some("synthetic_morphology_proposal") {
        push_findings(&mut findings, "MORPHOLOGY_EVIDENCE_CLASS_UNSUPPORTED");
    }
    if str_at(model, "geometry_fidelity")
        != Some("bounding_primitives_not_manufacturing_geometry")
    {
        push_findings(&mut findings, "MORPHOLOGY_GEOMETRY_FIDELITY_UNSUPPORTED");
    }

    let units = model.get("units").unwrap_or(&NULL_VALUE);
    const UNIT_FIELDS: &[&str] = &[
        "length",
        "mass",
        "joint_angle",
        "joint_effort",
        "joint_velocity",
    ];
    morphology_unknown_fields(units, UNIT_FIELDS, &mut findings);
    if !morphology_required_fields(units, UNIT_FIELDS)
        || str_at(units, "length") != Some("m")
        || str_at(units, "mass") != Some("kg")
        || str_at(units, "joint_angle") != Some("rad")
        || str_at(units, "joint_effort") != Some("N_m")
        || str_at(units, "joint_velocity") != Some("rad_s")
    {
        push_findings(&mut findings, "MORPHOLOGY_UNITS_INVALID");
    }

    let controller = model.get("controller").unwrap_or(&NULL_VALUE);
    const CONTROLLER_FIELDS: &[&str] = &[
        "kinematics_only",
        "commands_enabled",
        "inertia_source",
    ];
    morphology_unknown_fields(controller, CONTROLLER_FIELDS, &mut findings);
    if !morphology_required_fields(controller, CONTROLLER_FIELDS) {
        push_findings(&mut findings, "MORPHOLOGY_INCOMPLETE");
    }
    if bool_at(controller, "kinematics_only") != Some(true)
        || bool_at(controller, "commands_enabled") != Some(false)
        || str_at(controller, "inertia_source")
            != Some("derived_from_synthetic_bounding_primitives")
    {
        push_findings(&mut findings, "MORPHOLOGY_CONTROLLER_NOT_DISABLED");
    }

    let authority = model.get("authority").unwrap_or(&NULL_VALUE);
    const AUTHORITY_FIELDS: &[&str] = &[
        "simulation_only",
        "physical_actuation_authorized",
        "patient_contact_authorized",
        "clinical_use_authorized",
    ];
    morphology_unknown_fields(authority, AUTHORITY_FIELDS, &mut findings);
    if !morphology_required_fields(authority, AUTHORITY_FIELDS) {
        push_findings(&mut findings, "MORPHOLOGY_INCOMPLETE");
    }
    if bool_at(authority, "simulation_only") != Some(true) {
        push_findings(&mut findings, "MORPHOLOGY_SIMULATION_ONLY_REQUIRED");
    }
    if bool_at(authority, "physical_actuation_authorized") != Some(false)
        || bool_at(controller, "commands_enabled") != Some(false)
    {
        push_findings(&mut findings, "MORPHOLOGY_PHYSICAL_AUTHORITY_FORBIDDEN");
    }
    if bool_at(authority, "patient_contact_authorized") != Some(false) {
        push_findings(&mut findings, "MORPHOLOGY_PATIENT_CONTACT_FORBIDDEN");
    }
    if bool_at(authority, "clinical_use_authorized") != Some(false) {
        push_findings(&mut findings, "MORPHOLOGY_CLINICAL_USE_FORBIDDEN");
    }

    let caveats_valid = model
        .get("caveats")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            !items.is_empty()
                && items.iter().all(|item| {
                    item.as_str()
                        .is_some_and(|value| !value.trim().is_empty())
                })
        });
    if !caveats_valid {
        push_findings(&mut findings, "MORPHOLOGY_CAVEATS_REQUIRED");
    }

    let Some(links) = model.get("links").and_then(Value::as_array) else {
        push_findings(&mut findings, "MORPHOLOGY_LINKS_REQUIRED");
        return findings.into_iter().collect();
    };
    if links.len() < 2 || links.len() > 64 {
        push_findings(&mut findings, "MORPHOLOGY_LINK_COUNT_OUT_OF_RANGE");
    }

    let mut link_ids = BTreeSet::new();
    for link in links {
        const LINK_FIELDS: &[&str] = &["link_id", "mass_kg", "collision"];
        morphology_unknown_fields(link, LINK_FIELDS, &mut findings);
        if !morphology_required_fields(link, LINK_FIELDS) {
            push_findings(&mut findings, "MORPHOLOGY_INCOMPLETE");
        }
        let Some(link_id) = str_at(link, "link_id") else {
            push_findings(&mut findings, "MORPHOLOGY_LINK_ID_INVALID");
            continue;
        };
        if link_id.trim().is_empty() || !link_ids.insert(link_id.to_owned()) {
            push_findings(&mut findings, "MORPHOLOGY_LINK_ID_INVALID");
        }
        if num_at(link, "mass_kg").is_none_or(|mass| !(0.001..=250.0).contains(&mass)) {
            push_findings(&mut findings, "MORPHOLOGY_LINK_MASS_INVALID");
        }
        let collision = link.get("collision").unwrap_or(&NULL_VALUE);
        const COLLISION_FIELDS: &[&str] = &["shape", "dimensions_m", "source"];
        morphology_unknown_fields(collision, COLLISION_FIELDS, &mut findings);
        if !morphology_required_fields(collision, COLLISION_FIELDS) {
            push_findings(&mut findings, "MORPHOLOGY_INCOMPLETE");
        }
        if !["box", "capsule"].contains(&str_at(collision, "shape").unwrap_or("")) {
            push_findings(&mut findings, "MORPHOLOGY_COLLISION_SHAPE_UNSUPPORTED");
        }
        let valid_dimensions = collision
            .get("dimensions_m")
            .and_then(Value::as_array)
            .is_some_and(|dimensions| {
                dimensions.len() == 3
                    && dimensions.iter().all(|dimension| {
                        dimension
                            .as_f64()
                            .is_some_and(|value| value.is_finite() && value > 0.0 && value <= 2.0)
                    })
            });
        if !valid_dimensions {
            push_findings(&mut findings, "MORPHOLOGY_COLLISION_DIMENSIONS_INVALID");
        }
        if str_at(collision, "source") != Some("synthetic_placeholder") {
            push_findings(&mut findings, "MORPHOLOGY_GEOMETRY_SOURCE_UNSUPPORTED");
        }
    }

    let root_link = str_at(model, "root_link").unwrap_or("");
    if !link_ids.contains(root_link) {
        push_findings(&mut findings, "MORPHOLOGY_ROOT_LINK_UNKNOWN");
    }

    let Some(joints) = model.get("joints").and_then(Value::as_array) else {
        push_findings(&mut findings, "MORPHOLOGY_JOINTS_REQUIRED");
        return findings.into_iter().collect();
    };
    if joints.len() > 128 || joints.len().saturating_add(1) != links.len() {
        push_findings(&mut findings, "MORPHOLOGY_EDGE_COUNT_INVALID");
    }

    let mut joint_ids = BTreeSet::new();
    let mut incoming = BTreeMap::<String, usize>::new();
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for joint in joints {
        const JOINT_FIELDS: &[&str] = &[
            "joint_id",
            "parent_link",
            "child_link",
            "joint_type",
            "origin_m",
            "axis_xyz",
            "lower_limit_rad",
            "upper_limit_rad",
            "max_effort_nm",
            "max_velocity_rad_s",
            "command_enabled",
            "source",
        ];
        morphology_unknown_fields(joint, JOINT_FIELDS, &mut findings);
        if !morphology_required_fields(joint, JOINT_FIELDS) {
            push_findings(&mut findings, "MORPHOLOGY_INCOMPLETE");
        }
        let joint_id = str_at(joint, "joint_id").unwrap_or("");
        if joint_id.trim().is_empty() || !joint_ids.insert(joint_id.to_owned()) {
            push_findings(&mut findings, "MORPHOLOGY_JOINT_ID_INVALID");
        }
        let parent = str_at(joint, "parent_link").unwrap_or("");
        let child = str_at(joint, "child_link").unwrap_or("");
        if !link_ids.contains(parent) || !link_ids.contains(child) {
            push_findings(&mut findings, "MORPHOLOGY_JOINT_LINK_UNKNOWN");
        }
        if parent == child {
            push_findings(&mut findings, "MORPHOLOGY_SELF_EDGE");
        }
        if link_ids.contains(child) {
            *incoming.entry(child.to_owned()).or_default() += 1;
        }
        if link_ids.contains(parent) && link_ids.contains(child) && parent != child {
            adjacency
                .entry(parent.to_owned())
                .or_default()
                .push(child.to_owned());
        }

        let origin = finite_vec3(joint.get("origin_m").unwrap_or(&NULL_VALUE));
        if origin.is_none_or(|coords| coords.iter().any(|value| value.abs() > 10.0)) {
            push_findings(&mut findings, "MORPHOLOGY_JOINT_ORIGIN_INVALID");
        }
        let axis = finite_vec3(joint.get("axis_xyz").unwrap_or(&NULL_VALUE));
        let lower = num_at(joint, "lower_limit_rad");
        let upper = num_at(joint, "upper_limit_rad");
        let effort = num_at(joint, "max_effort_nm");
        let velocity = num_at(joint, "max_velocity_rad_s");
        if bool_at(joint, "command_enabled") != Some(false) {
            push_findings(&mut findings, "MORPHOLOGY_JOINT_COMMAND_FORBIDDEN");
        }
        if str_at(joint, "source") != Some("synthetic_placeholder") {
            push_findings(&mut findings, "MORPHOLOGY_JOINT_SOURCE_UNSUPPORTED");
        }

        match str_at(joint, "joint_type") {
            Some("fixed") => {
                let zero_axis = axis.is_some_and(|axis| {
                    axis.iter().all(|component| component.abs() <= 1.0e-8)
                });
                if !zero_axis
                    || lower != Some(0.0)
                    || upper != Some(0.0)
                    || effort != Some(0.0)
                    || velocity != Some(0.0)
                {
                    push_findings(&mut findings, "MORPHOLOGY_FIXED_JOINT_ENVELOPE_INVALID");
                }
            }
            Some("revolute") => {
                let unit_axis = axis.is_some_and(|axis| {
                    let norm = axis
                        .iter()
                        .map(|component| component * component)
                        .sum::<f64>()
                        .sqrt();
                    (norm - 1.0).abs() <= 1.0e-3
                });
                if !unit_axis {
                    push_findings(&mut findings, "MORPHOLOGY_REVOLUTE_AXIS_INVALID");
                }
                if lower.is_none_or(|value| !(-50.0..=50.0).contains(&value))
                    || upper.is_none_or(|value| !(-50.0..=50.0).contains(&value))
                    || lower.zip(upper).is_none_or(|(lo, hi)| lo >= hi)
                {
                    push_findings(&mut findings, "MORPHOLOGY_REVOLUTE_LIMITS_INVALID");
                }
                if effort.is_none_or(|value| !(0.001..=1000.0).contains(&value))
                    || velocity.is_none_or(|value| !(0.001..=20.0).contains(&value))
                {
                    push_findings(&mut findings, "MORPHOLOGY_JOINT_ENVELOPE_INVALID");
                }
            }
            _ => push_findings(&mut findings, "MORPHOLOGY_JOINT_TYPE_UNSUPPORTED"),
        }
    }

    if incoming.get(root_link).copied().unwrap_or(0) != 0 {
        push_findings(&mut findings, "MORPHOLOGY_ROOT_HAS_PARENT");
    }
    for link_id in &link_ids {
        let expected_incoming = if link_id.as_str() == root_link { 0 } else { 1 };
        if incoming.get(link_id).copied().unwrap_or(0) != expected_incoming {
            push_findings(&mut findings, "MORPHOLOGY_PARENT_COUNT_INVALID");
        }
    }

    let mut reached = BTreeSet::new();
    let mut pending = vec![root_link.to_owned()];
    while let Some(link_id) = pending.pop() {
        if !reached.insert(link_id.clone()) {
            continue;
        }
        if let Some(children) = adjacency.get(&link_id) {
            pending.extend(children.iter().cloned());
        }
    }
    if reached.len() != link_ids.len() || reached.contains("") {
        push_findings(&mut findings, "MORPHOLOGY_GRAPH_DISCONNECTED_OR_CYCLIC");
    }

    findings.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq)]
struct LinkPose {
    rotation: [[f64; 3]; 3],
    translation: [f64; 3],
}

impl LinkPose {
    fn identity() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0, 0.0, 0.0],
        }
    }
}

fn rotation_product(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..3)
                .map(|index| a[row][index] * b[index][column])
                .sum()
        })
    })
}

fn rotate_vector(rotation: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|row| {
        (0..3)
            .map(|column| rotation[row][column] * vector[column])
            .sum()
    })
}

fn axis_angle_rotation(axis: [f64; 3], angle: f64) -> [[f64; 3]; 3] {
    let [x, y, z] = axis;
    let cosine = angle.cos();
    let sine = angle.sin();
    let one_minus_cosine = 1.0 - cosine;
    [
        [
            cosine + x * x * one_minus_cosine,
            x * y * one_minus_cosine - z * sine,
            x * z * one_minus_cosine + y * sine,
        ],
        [
            y * x * one_minus_cosine + z * sine,
            cosine + y * y * one_minus_cosine,
            y * z * one_minus_cosine - x * sine,
        ],
        [
            z * x * one_minus_cosine - y * sine,
            z * y * one_minus_cosine + x * sine,
            cosine + z * z * one_minus_cosine,
        ],
    ]
}

/// Computes geometry-only forward kinematics for a previously validated tree.
/// Input is a map from every revolute joint ID to a bounded position in radians.
/// This function has no actuator, controller, velocity or physics side effects.
fn compute_forward_kinematics(
    model: &Value,
    joint_state: &Value,
) -> Result<BTreeMap<String, LinkPose>, Vec<String>> {
    let morphology_findings = validate_morphology(model);
    if !morphology_findings.is_empty() {
        return Err(morphology_findings);
    }

    let mut findings = BTreeSet::new();
    let Some(states) = joint_state.as_object() else {
        return Err(vec!["JOINT_STATE_ROOT_MUST_BE_OBJECT".to_owned()]);
    };
    let joints = model["joints"]
        .as_array()
        .expect("morphology validator checked the joint array");
    let mut positions = BTreeMap::<String, f64>::new();

    for (joint_id, value) in states {
        let Some(joint) = joints
            .iter()
            .find(|joint| str_at(joint, "joint_id") == Some(joint_id.as_str()))
        else {
            push_findings(&mut findings, "JOINT_STATE_UNKNOWN_ID");
            continue;
        };
        if str_at(joint, "joint_type") != Some("revolute") {
            push_findings(&mut findings, "JOINT_STATE_FOR_FIXED_JOINT");
            continue;
        }
        let Some(position) = value.as_f64().filter(|position| position.is_finite()) else {
            push_findings(&mut findings, "JOINT_STATE_VALUE_INVALID");
            continue;
        };
        let lower = num_at(joint, "lower_limit_rad").unwrap_or(f64::NAN);
        let upper = num_at(joint, "upper_limit_rad").unwrap_or(f64::NAN);
        if position < lower || position > upper {
            push_findings(&mut findings, "JOINT_STATE_OUT_OF_LIMITS");
            continue;
        }
        positions.insert(joint_id.clone(), position);
    }

    for joint in joints {
        if str_at(joint, "joint_type") == Some("revolute") {
            let joint_id = str_at(joint, "joint_id").unwrap_or("");
            if !positions.contains_key(joint_id) {
                push_findings(&mut findings, "JOINT_STATE_MISSING_REVOLUTE_JOINT");
            }
        }
    }
    if !findings.is_empty() {
        return Err(findings.into_iter().collect());
    }

    let root_id = str_at(model, "root_link").expect("morphology validator checked root");
    let mut poses = BTreeMap::new();
    poses.insert(root_id.to_owned(), LinkPose::identity());
    let mut processed = BTreeSet::new();

    while processed.len() < joints.len() {
        let mut made_progress = false;
        for (index, joint) in joints.iter().enumerate() {
            if processed.contains(&index) {
                continue;
            }
            let parent_id = str_at(joint, "parent_link").expect("validated parent link");
            let child_id = str_at(joint, "child_link").expect("validated child link");
            let Some(parent_pose) = poses.get(parent_id).cloned() else {
                continue;
            };
            let origin = finite_vec3(&joint["origin_m"]).expect("validated joint origin");
            let parent_frame_offset = rotate_vector(parent_pose.rotation, origin);
            let translation = std::array::from_fn(|axis| {
                parent_pose.translation[axis] + parent_frame_offset[axis]
            });

            let joint_rotation = if str_at(joint, "joint_type") == Some("revolute") {
                let axis = finite_vec3(&joint["axis_xyz"]).expect("validated revolute axis");
                axis_angle_rotation(axis, positions[joint["joint_id"].as_str().unwrap_or("")])
            } else {
                LinkPose::identity().rotation
            };
            let rotation = rotation_product(parent_pose.rotation, joint_rotation);
            poses.insert(child_id.to_owned(), LinkPose { rotation, translation });
            processed.insert(index);
            made_progress = true;
        }
        if !made_progress {
            return Err(vec!["KINEMATICS_GRAPH_TRAVERSAL_STALLED".to_owned()]);
        }
    }

    Ok(poses)
}

fn link_poses_json(poses: &BTreeMap<String, LinkPose>) -> Value {
    let links = poses
        .iter()
        .map(|(link_id, pose)| {
            (
                link_id.clone(),
                json!({
                    "translation_m": pose.translation,
                    "rotation_matrix_row_major": pose.rotation
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    Value::Object(links)
}

fn evaluate_kinematics_file(model_path: &str, state_path: &str) {
    let (model_bytes, model) = match load_json(model_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    if model_path == MORPHOLOGY_PATH && sha256_hex(&model_bytes) != sha256_hex(MORPHOLOGY_SOURCE) {
        eprintln!("runtime morphology differs from the model embedded in the compiled validator");
        process::exit(2);
    }
    let (state_bytes, state) = match load_json(state_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let tested_revision =
        env::var("CARE_FRAME_TESTED_SHA").unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned());
    let result = compute_forward_kinematics(&model, &state);
    let (disposition, poses, findings) = match result {
        Ok(poses) => ("KINEMATICS_ONLY", Some(link_poses_json(&poses)), Vec::new()),
        Err(findings) => ("BLOCKED", None, findings),
    };
    let receipt = json!({
        "receipt_schema": "careframe-forward-kinematics-receipt-v0",
        "tested_revision": tested_revision,
        "model_id": str_at(&model, "model_id").unwrap_or("<missing>"),
        "model_revision": model.get("revision").and_then(Value::as_u64),
        "disposition": disposition,
        "morphology_sha256": sha256_hex(&model_bytes),
        "joint_state_sha256": sha256_hex(&state_bytes),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "link_poses": poses,
        "findings": findings,
        "physical_actuation_authorized": false,
        "patient_contact_authorized": false,
        "clinical_use_authorized": false,
        "claim_ceiling": concat!(
            "Geometry-only forward kinematics on a synthetic tree; not engine ",
            "or physical validation."
        )
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .expect("kinematics receipt serialization cannot fail")
    );
    if disposition == "BLOCKED" {
        process::exit(1);
    }
}

fn validate_morphology_file(path: &str) {
    let (bytes, model) = match load_json(path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    if path == MORPHOLOGY_PATH && sha256_hex(&bytes) != sha256_hex(MORPHOLOGY_SOURCE) {
        eprintln!("runtime morphology differs from the model embedded in the compiled validator");
        process::exit(2);
    }
    let findings = validate_morphology(&model);
    let blocked = !findings.is_empty();
    let tested_revision =
        env::var("CARE_FRAME_TESTED_SHA").unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned());
    let receipt = json!({
        "receipt_schema": "careframe-morphology-receipt-v0",
        "tested_revision": tested_revision,
        "model_id": str_at(&model, "model_id").unwrap_or("<missing>"),
        "model_revision": model.get("revision").and_then(Value::as_u64),
        "disposition": if blocked { "BLOCKED" } else { "SIMULATION_ONLY" },
        "findings": findings,
        "morphology_sha256": sha256_hex(&bytes),
        "embedded_morphology_sha256": sha256_hex(MORPHOLOGY_SOURCE),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "physical_actuation_authorized": false,
        "patient_contact_authorized": false,
        "clinical_use_authorized": false,
        "claim_ceiling": concat!(
            "Synthetic morphology graph/envelope only; no physical or clinical ",
            "validity claim."
        )
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serialization cannot fail")
    );
    if blocked {
        process::exit(1);
    }
}

fn validate_manifest_file(path: &str) {
    let (bytes, manifest) = match load_json(path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let findings = validate_tool(&manifest);
    let blocked = !findings.is_empty();
    let tested_revision =
        env::var("CARE_FRAME_TESTED_SHA").unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned());
    let receipt = json!({
        "receipt_schema": "medical-robot-tool-candidate-receipt-v0",
        "tested_revision": tested_revision,
        "candidate_tool_id": str_at(&manifest, "tool_id").unwrap_or("<missing>"),
        "disposition": if blocked { "BLOCKED" } else { "SIMULATION_ONLY" },
        "findings": findings,
        "tool_manifest_sha256": sha256_hex(&bytes),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "clinical_use_authorized": false,
        "patient_contact_authorized": false,
        "physical_actuation_authorized": false,
        "claim_ceiling": CLAIM_CEILING
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serialization cannot fail")
    );
    if blocked {
        process::exit(1);
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some("--template") {
        let Some(tool_kind) = args.next() else {
            eprintln!("usage: medical-robot-tool-gate --template TOOL_KIND [OUTPUT_PATH]");
            process::exit(2);
        };
        let output_path = args.next();
        if args.next().is_some() {
            eprintln!("usage: medical-robot-tool-gate --template TOOL_KIND [OUTPUT_PATH]");
            process::exit(2);
        }
        write_or_print_template(&tool_kind, output_path.as_deref());
        return;
    }
    if first.as_deref() == Some("--evaluate-kinematics") {
        let Some(model_path) = args.next() else {
            eprintln!(
                "usage: medical-robot-tool-gate --evaluate-kinematics MODEL_JSON JOINT_STATE_JSON"
            );
            process::exit(2);
        };
        let Some(state_path) = args.next() else {
            eprintln!(
                "usage: medical-robot-tool-gate --evaluate-kinematics MODEL_JSON JOINT_STATE_JSON"
            );
            process::exit(2);
        };
        if args.next().is_some() {
            eprintln!("unexpected extra argument");
            process::exit(2);
        }
        evaluate_kinematics_file(&model_path, &state_path);
        return;
    }
    if first.as_deref() == Some("--validate-morphology") {
        let Some(path) = args.next() else {
            eprintln!("usage: medical-robot-tool-gate --validate-morphology PATH");
            process::exit(2);
        };
        if args.next().is_some() {
            eprintln!("unexpected extra argument");
            process::exit(2);
        }
        validate_morphology_file(&path);
        return;
    }
    if first.as_deref() == Some("--validate-manifest") {
        let Some(path) = args.next() else {
            eprintln!("usage: medical-robot-tool-gate --validate-manifest PATH");
            process::exit(2);
        };
        if args.next().is_some() {
            eprintln!("unexpected extra argument");
            process::exit(2);
        }
        validate_manifest_file(&path);
        return;
    }
    let path = first.unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    if args.next().is_some() {
        eprintln!("usage: medical-robot-tool-gate [CORPUS_PATH]");
        eprintln!("     | --template TOOL_KIND [OUTPUT_PATH] | --validate-manifest PATH");
        eprintln!("     | --validate-morphology PATH");
        eprintln!("     | --evaluate-kinematics MODEL_JSON JOINT_STATE_JSON");
        process::exit(2);
    }
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

    fn morphology() -> Value {
        serde_json::from_slice(include_bytes!(
            "../../../docs/medical-robotics/careframe-wheeled-upper-body-morphology-v0.json"
        ))
        .expect("Careframe morphology fixture parses")
    }

    #[test]
    fn frozen_careframe_morphology_is_a_connected_kinematic_tree() {
        let model = morphology();
        assert_eq!(model["links"].as_array().expect("links").len(), 16);
        assert_eq!(model["joints"].as_array().expect("joints").len(), 15);
        let findings = validate_morphology(&model);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn morphology_unknown_fields_are_rejected_at_each_object_boundary() {
        let mut root = morphology();
        root["silent_autonomy"] = json!(true);
        assert!(validate_morphology(&root)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_UNKNOWN_FIELD"));

        let mut link = morphology();
        link["links"][0]["extra_mass_mode"] = json!("hidden");
        assert!(validate_morphology(&link)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_UNKNOWN_FIELD"));

        let mut collision = morphology();
        collision["links"][0]["collision"]["sterile"] = json!(true);
        assert!(validate_morphology(&collision)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_UNKNOWN_FIELD"));

        let mut joint = morphology();
        joint["joints"][0]["direct_motor_command"] = json!(true);
        assert!(validate_morphology(&joint)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_UNKNOWN_FIELD"));

        let mut controller = morphology();
        controller["controller"]["remote_override"] = json!(true);
        assert!(validate_morphology(&controller)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_UNKNOWN_FIELD"));

        let mut authority = morphology();
        authority["authority"]["clinical_mode"] = json!(true);
        assert!(validate_morphology(&authority)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_UNKNOWN_FIELD"));
    }

    #[test]
    fn morphology_graph_rejects_duplicate_ids_unknown_links_and_multiple_parents() {
        let mut duplicate_link = morphology();
        duplicate_link["links"][1]["link_id"] = json!("base_chassis");
        assert!(validate_morphology(&duplicate_link)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_LINK_ID_INVALID"));

        let mut unknown_parent = morphology();
        unknown_parent["joints"][0]["parent_link"] = json!("missing_link");
        assert!(validate_morphology(&unknown_parent)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_JOINT_LINK_UNKNOWN"));

        let mut multiple_parent = morphology();
        multiple_parent["joints"][1]["child_link"] = json!("left_drive_wheel");
        assert!(validate_morphology(&multiple_parent)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_PARENT_COUNT_INVALID"));

        let mut cycle = morphology();
        cycle["joints"][0]["parent_link"] = json!("right_drive_wheel");
        cycle["joints"][1]["parent_link"] = json!("left_drive_wheel");
        assert!(validate_morphology(&cycle)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_GRAPH_DISCONNECTED_OR_CYCLIC"));
    }

    #[test]
    fn morphology_rejects_invalid_axes_limits_geometry_and_link_masses() {
        let mut axis = morphology();
        axis["joints"][0]["axis_xyz"] = json!([0.0, 0.0, 0.0]);
        assert!(validate_morphology(&axis)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_REVOLUTE_AXIS_INVALID"));

        let mut limits = morphology();
        limits["joints"][0]["lower_limit_rad"] = json!(1.0);
        limits["joints"][0]["upper_limit_rad"] = json!(1.0);
        assert!(validate_morphology(&limits)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_REVOLUTE_LIMITS_INVALID"));

        let mut dimensions = morphology();
        dimensions["links"][0]["collision"]["dimensions_m"] = json!([0.0, 0.5, 0.5]);
        assert!(validate_morphology(&dimensions)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_COLLISION_DIMENSIONS_INVALID"));

        let mut mass = morphology();
        mass["links"][0]["mass_kg"] = json!(-1.0);
        assert!(validate_morphology(&mass)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_LINK_MASS_INVALID"));
    }

    #[test]
    fn morphology_fixed_joints_require_zero_motion_and_effort_envelopes() {
        let mut model = morphology();
        let fixed_index = model["joints"]
            .as_array()
            .expect("joints")
            .iter()
            .position(|joint| str_at(joint, "joint_type") == Some("fixed"))
            .expect("fixture includes fixed mount joints");
        model["joints"][fixed_index]["axis_xyz"] = json!([0.0, 1.0, 0.0]);
        assert!(validate_morphology(&model)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_FIXED_JOINT_ENVELOPE_INVALID"));

        let mut model = morphology();
        let fixed_index = model["joints"]
            .as_array()
            .expect("joints")
            .iter()
            .position(|joint| str_at(joint, "joint_type") == Some("fixed"))
            .expect("fixture includes fixed mount joints");
        model["joints"][fixed_index]["max_effort_nm"] = json!(0.1);
        assert!(validate_morphology(&model)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_FIXED_JOINT_ENVELOPE_INVALID"));
    }

    fn zero_joint_state() -> Value {
        json!({
            "left_drive": 0.0,
            "right_drive": 0.0,
            "torso_yaw": 0.0,
            "head_yaw": 0.0,
            "head_pitch": 0.0,
            "left_shoulder_pitch": 0.0,
            "left_elbow": 0.0,
            "right_shoulder_pitch": 0.0,
            "right_elbow": 0.0
        })
    }

    #[test]
    fn nominal_forward_kinematics_matches_independent_reference_points() {
        let model = morphology();
        let poses = compute_forward_kinematics(&model, &zero_joint_state())
            .expect("nominal bounded joint state must compute");
        assert_eq!(poses.len(), 16);

        let chassis = &poses["base_chassis"];
        assert_eq!(chassis.translation, [0.0, 0.0, 0.0]);
        let torso = &poses["torso_shell"];
        assert!((torso.translation[0]).abs() < 1.0e-12);
        assert!((torso.translation[1]).abs() < 1.0e-12);
        assert!((torso.translation[2] - 0.32).abs() < 1.0e-12);

        let left_mount = &poses["left_tool_mount"];
        assert!((left_mount.translation[0]).abs() < 1.0e-12);
        assert!((left_mount.translation[1] - 0.19).abs() < 1.0e-12);
        assert!((left_mount.translation[2] - 0.28).abs() < 1.0e-12);
        let right_mount = &poses["right_tool_mount"];
        assert!((right_mount.translation[0]).abs() < 1.0e-12);
        assert!((right_mount.translation[1] + 0.19).abs() < 1.0e-12);
        assert!((right_mount.translation[2] - 0.28).abs() < 1.0e-12);
    }

    #[test]
    fn forward_kinematics_propagates_rotations_down_the_tree() {
        let model = morphology();
        let mut state = zero_joint_state();
        let yaw = 1.0_f64;
        state["torso_yaw"] = json!(yaw);
        let poses = compute_forward_kinematics(&model, &state)
            .expect("bounded torso rotation must compute");
        let left_arm = &poses["left_upper_arm"];
        assert!((left_arm.translation[0] - (-yaw.sin() * 0.19)).abs() < 1.0e-10);
        assert!((left_arm.translation[1] - (yaw.cos() * 0.19)).abs() < 1.0e-10);
        assert!((left_arm.translation[2] - 0.56).abs() < 1.0e-12);

        let right_arm = &poses["right_upper_arm"];
        assert!((right_arm.translation[0] - (yaw.sin() * 0.19)).abs() < 1.0e-10);
        assert!((right_arm.translation[1] - (-yaw.cos() * 0.19)).abs() < 1.0e-10);

        let rotation = left_arm.rotation;
        let unit_column = (0..3)
            .map(|row| rotation[row][0] * rotation[row][0])
            .sum::<f64>();
        assert!((unit_column - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn forward_kinematics_rejects_missing_unknown_fixed_and_out_of_range_states() {
        let model = morphology();

        let mut missing = zero_joint_state();
        missing.as_object_mut().expect("state object").remove("left_elbow");
        assert!(compute_forward_kinematics(&model, &missing)
            .expect_err("missing state rejects")
            .iter()
            .any(|finding| finding == "JOINT_STATE_MISSING_REVOLUTE_JOINT"));

        let mut unknown = zero_joint_state();
        unknown["silent_joint"] = json!(0.0);
        assert!(compute_forward_kinematics(&model, &unknown)
            .expect_err("unknown state rejects")
            .iter()
            .any(|finding| finding == "JOINT_STATE_UNKNOWN_ID"));

        let mut fixed = zero_joint_state();
        fixed["torso_mount"] = json!(0.0);
        assert!(compute_forward_kinematics(&model, &fixed)
            .expect_err("fixed-joint state rejects")
            .iter()
            .any(|finding| finding == "JOINT_STATE_FOR_FIXED_JOINT"));

        let mut outside = zero_joint_state();
        outside["torso_yaw"] = json!(1.21);
        assert!(compute_forward_kinematics(&model, &outside)
            .expect_err("out-of-range state rejects")
            .iter()
            .any(|finding| finding == "JOINT_STATE_OUT_OF_LIMITS"));
    }

    #[test]
    fn morphology_cannot_enable_joint_commands_or_raise_authority() {
        let mut commands = morphology();
        commands["joints"][0]["command_enabled"] = json!(true);
        assert!(validate_morphology(&commands)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_JOINT_COMMAND_FORBIDDEN"));

        let mut controller = morphology();
        controller["controller"]["commands_enabled"] = json!(true);
        assert!(validate_morphology(&controller)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_CONTROLLER_NOT_DISABLED"));

        let mut authority = morphology();
        authority["authority"]["physical_actuation_authorized"] = json!(true);
        assert!(validate_morphology(&authority)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_PHYSICAL_AUTHORITY_FORBIDDEN"));

        let mut patient = morphology();
        patient["authority"]["patient_contact_authorized"] = json!(true);
        assert!(validate_morphology(&patient)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_PATIENT_CONTACT_FORBIDDEN"));

        let mut clinical = morphology();
        clinical["authority"]["clinical_use_authorized"] = json!(true);
        assert!(validate_morphology(&clinical)
            .iter()
            .any(|finding| finding == "MORPHOLOGY_CLINICAL_USE_FORBIDDEN"));
    }

    #[test]
    fn built_in_templates_are_eligible_for_simulation_only() {
        for kind in [
            "sealed_supply_gripper",
            "tray_carrier",
            "telepresence_head",
            "trainer_probe",
        ] {
            let manifest = tool_template(kind).expect("supported concept template");
            assert!(
                validate_tool(&manifest).is_empty(),
                "{kind}: {:?}",
                validate_tool(&manifest)
            );
            assert_eq!(
                bool_at(&manifest["authority"], "physical_actuation_authorized"),
                Some(false)
            );
            assert_eq!(
                bool_at(&manifest["authority"], "patient_contact_authorized"),
                Some(false)
            );
            assert_eq!(
                bool_at(&manifest["authority"], "clinical_use_authorized"),
                Some(false)
            );
        }

        assert!(tool_template("patient_transfer").is_none());
        assert!(tool_template("medication_dispenser").is_none());
        assert!(tool_template("invasive_tool").is_none());
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
            assert!(
                findings.iter().any(|finding| finding == expected),
                "{field}: {findings:?}"
            );
        }
        let mut manifest = base;
        manifest["authority"]["simulation_only"] = Value::Bool(false);
        assert!(validate_tool(&manifest)
            .iter()
            .any(|finding| finding == "SIMULATION_ONLY_REQUIRED"));
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
        assert!(validate_tool(&mechanical)
            .iter()
            .any(|finding| finding == "UNKNOWN_FIELD"));

        let mut privacy = base.clone();
        privacy["privacy"]["capture_without_consent"] = json!(true);
        assert!(validate_tool(&privacy).iter().any(|finding| finding == "UNKNOWN_FIELD"));

        let mut hygiene = base.clone();
        hygiene["hygiene"]["sterility_claim"] = json!("validated");
        assert!(validate_tool(&hygiene).iter().any(|finding| finding == "UNKNOWN_FIELD"));

        let mut electrical = base.clone();
        electrical["electrical"]["remote_override"] = json!(true);
        assert!(validate_tool(&electrical)
            .iter()
            .any(|finding| finding == "UNKNOWN_FIELD"));

        let mut authority = base;
        authority["authority"]["unreviewed_clinical_mode"] = json!(true);
        assert!(validate_tool(&authority).iter().any(|finding| finding == "UNKNOWN_FIELD"));
    }

    #[test]
    fn corpus_schema_rejects_unknown_fields_and_count_drift() {
        let mut root = corpus();
        root["unreviewed_autonomy"] = json!(true);
        assert!(run_corpus(&root).is_err());

        let mut root = corpus();
        root["cases"][0]["new_capability"] = json!("silent");
        assert!(run_corpus(&root).is_err());

        let mut root = corpus();
        root["expected_case_count"] = json!(15);
        assert!(run_corpus(&root).is_err());
    }

    #[test]
    fn no_contact_class_or_manifest_can_grant_patient_use() {
        let root = corpus();
        for case in root["cases"].as_array().expect("cases") {
            let findings = validate_tool(&case["manifest"]);
            assert!(
                !findings.is_empty()
                    || bool_at(&case["manifest"]["authority"], "patient_contact_authorized")
                        == Some(false)
            );
            assert!(
                !findings.is_empty()
                    || bool_at(&case["manifest"]["authority"], "clinical_use_authorized")
                        == Some(false)
            );
            assert!(
                !findings.is_empty()
                    || bool_at(&case["manifest"]["authority"], "physical_actuation_authorized")
                        == Some(false)
            );
        }
        let mut manifest = root["cases"][0]["manifest"].clone();
        manifest["contact_class"] = json!("patient_contact");
        assert!(validate_tool(&manifest)
            .iter()
            .any(|finding| finding == "PATIENT_CONTACT_NOT_AUTHORIZED"));
    }

    #[test]
    fn privacy_capture_requires_shutter_and_capture_disabled_by_default() {
        let root = corpus();
        let mut no_shutter = root["cases"][2]["manifest"].clone();
        no_shutter["privacy"]["physical_privacy_shutter"] = Value::Bool(false);
        assert!(validate_tool(&no_shutter)
            .iter()
            .any(|finding| finding == "PRIVACY_FAIL_CLOSED_REQUIRED"));

        let mut live_default = root["cases"][2]["manifest"].clone();
        live_default["privacy"]["default_capture_state"] = json!("enabled");
        assert!(validate_tool(&live_default)
            .iter()
            .any(|finding| finding == "PRIVACY_FAIL_CLOSED_REQUIRED"));
    }

    #[test]
    fn hygiene_validation_cannot_be_claimed_by_a_concept_manifest() {
        let root = corpus();
        let mut manifest = root["cases"][0]["manifest"].clone();
        manifest["hygiene"]["cleaning_claim"] = json!("validated");
        assert!(validate_tool(&manifest)
            .iter()
            .any(|finding| finding == "HYGIENE_VALIDATION_CLAIM_UNSUPPORTED"));
    }

    #[test]
    fn geometry_mount_and_retention_mutations_fail_closed() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();
        let mut geometry = base.clone();
        geometry["mechanical"]["geometry_bbox_mm"] = json!([200, 0, 100]);
        assert!(validate_tool(&geometry)
            .iter()
            .any(|finding| finding == "GEOMETRY_OUT_OF_ENVELOPE"));

        let mut mount = base.clone();
        mount["mechanical"]["mount_interface"] = json!("UNKEYED_GENERIC_MOUNT");
        assert!(validate_tool(&mount)
            .iter()
            .any(|finding| finding == "MOUNT_INTERFACE_UNSUPPORTED"));

        let mut retention = base;
        retention["mechanical"]["retention_fail_closed"] = Value::Bool(false);
        assert!(validate_tool(&retention)
            .iter()
            .any(|finding| finding == "FAIL_CLOSED_RETENTION_REQUIRED"));
    }

    #[test]
    fn powered_tools_require_declared_protective_isolation() {
        let root = corpus();
        let mut manifest = root["cases"][0]["manifest"].clone();
        manifest["electrical"]["protective_isolation_declared"] = Value::Bool(false);
        assert!(validate_tool(&manifest)
            .iter()
            .any(|finding| finding == "POWER_ENVELOPE_INVALID"));
    }

    #[test]
    fn only_whitelisted_tool_task_contact_triples_are_eligible() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();
        let tool_kinds = [
            "sealed_supply_gripper",
            "tray_carrier",
            "telepresence_head",
            "trainer_probe",
            "patient_transfer",
            "medication_dispenser",
        ];
        let tasks = [
            "sealed_supply_transfer",
            "tray_transfer",
            "telepresence_support",
            "training_mannequin_measurement",
            "patient_transfer",
            "medication_dispensation",
        ];
        let contacts = [
            "none",
            "object_only",
            "mannequin_only",
            "patient_contact",
            "invasive",
        ];
        let mut combinations = 0usize;
        let mut eligible = 0usize;

        for kind in tool_kinds {
            for task in tasks {
                for contact in contacts {
                    let mut manifest = base.clone();
                    manifest["tool_kind"] = json!(kind);
                    manifest["intended_task"] = json!(task);
                    manifest["contact_class"] = json!(contact);
                    let allowed = matches!(
                        (kind, task, contact),
                        ("sealed_supply_gripper", "sealed_supply_transfer", "object_only")
                            | ("tray_carrier", "tray_transfer", "object_only")
                            | ("telepresence_head", "telepresence_support", "none")
                            | (
                                "trainer_probe",
                                "training_mannequin_measurement",
                                "mannequin_only"
                            )
                    );
                    let findings = validate_tool(&manifest);
                    assert_eq!(
                        findings.is_empty(),
                        allowed,
                        "{kind}/{task}/{contact} unexpectedly returned {findings:?}"
                    );
                    combinations += 1;
                    if findings.is_empty() {
                        eligible += 1;
                    }
                }
            }
        }

        assert_eq!(combinations, 180);
        assert_eq!(eligible, 4);
    }

    #[test]
    fn numeric_limits_reject_both_sides_of_every_registered_boundary() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();

        for (field, values, valid) in [
            (
                "mass_g",
                vec![
                    json!(-1),
                    json!(0),
                    json!(1),
                    json!(5000),
                    json!(5001),
                    json!("1"),
                    Value::Null,
                ],
                vec![false, false, true, true, false, false, false],
            ),
            (
                "payload_g",
                vec![
                    json!(-1),
                    json!(0),
                    json!(20000),
                    json!(20001),
                    json!("1"),
                    Value::Null,
                ],
                vec![false, true, true, false, false, false],
            ),
            (
                "max_object_force_n",
                vec![
                    json!(-1),
                    json!(0),
                    json!(1),
                    json!(50),
                    json!(51),
                    json!("1"),
                    Value::Null,
                ],
                vec![false, false, true, true, false, false, false],
            ),
            (
                "max_torque_nm",
                vec![json!(-1), json!(0), json!(20), json!(21), json!("1"), Value::Null],
                vec![false, true, true, false, false, false],
            ),
        ] {
            for (value, should_be_valid) in values.into_iter().zip(valid) {
                let mut manifest = base.clone();
                manifest["mechanical"][field] = value.clone();
                let findings = validate_tool(&manifest);
                assert_eq!(
                    findings.is_empty(),
                    should_be_valid,
                    "mechanical.{field}={value} returned {findings:?}"
                );
            }
        }

        for (dimension, value, should_be_valid) in [
            (0usize, json!(0), false),
            (0, json!(1), true),
            (0, json!(2000), true),
            (0, json!(2001), false),
            (1, json!(0), false),
            (1, json!(2000), true),
            (2, json!(0), false),
            (2, json!(2000), true),
        ] {
            let mut manifest = base.clone();
            manifest["mechanical"]["geometry_bbox_mm"][dimension] = value.clone();
            let findings = validate_tool(&manifest);
            assert_eq!(
                findings.is_empty(),
                should_be_valid,
                "geometry[{dimension}]={value} returned {findings:?}"
            );
        }
    }

    #[test]
    fn capture_policy_truth_table_fails_closed() {
        let root = corpus();
        let base = root["cases"][2]["manifest"].clone();
        let mut combinations = 0usize;
        for captures_audio in [false, true] {
            for captures_video in [false, true] {
                for shutter in [false, true] {
                    for default_enabled in [false, true] {
                        let mut manifest = base.clone();
                        manifest["privacy"]["captures_audio"] = json!(captures_audio);
                        manifest["privacy"]["captures_video"] = json!(captures_video);
                        manifest["privacy"]["physical_privacy_shutter"] = json!(shutter);
                        manifest["privacy"]["default_capture_state"] =
                            json!(if default_enabled { "enabled" } else { "disabled" });
                        let captures = captures_audio || captures_video;
                        let allowed = !captures || (shutter && !default_enabled);
                        let findings = validate_tool(&manifest);
                        assert_eq!(
                            findings.is_empty(),
                            allowed,
                            "audio={captures_audio}, video={captures_video}, \
                             shutter={shutter}, enabled={default_enabled}: {findings:?}"
                        );
                        combinations += 1;
                    }
                }
            }
        }
        assert_eq!(combinations, 16);
    }

    #[test]
    fn powered_and_unpowered_tool_electrical_envelopes_are_exhaustive() {
        let root = corpus();
        let base = root["cases"][0]["manifest"].clone();
        let mut combinations = 0usize;
        for powered in [false, true] {
            for power in [0.0_f64, 0.1, 20.0, 100.0, 100.1] {
                for isolated in [false, true] {
                    let mut manifest = base.clone();
                    manifest["electrical"]["powered"] = json!(powered);
                    manifest["electrical"]["max_power_w"] = json!(power);
                    manifest["electrical"]["protective_isolation_declared"] =
                        json!(isolated);
                    let allowed = if powered {
                        power > 0.0 && power <= 100.0 && isolated
                    } else {
                        power == 0.0
                    };
                    let findings = validate_tool(&manifest);
                    assert_eq!(
                        findings.is_empty(),
                        allowed,
                        "powered={powered}, power={power}, \
                         isolation={isolated}: {findings:?}"
                    );
                    combinations += 1;
                }
            }
        }
        assert_eq!(combinations, 20);
    }

    #[test]
    fn corpus_and_source_digests_are_bound_to_the_build() {
        assert_eq!(sha256_hex(AUDITOR_SOURCE).len(), 64);
        assert_eq!(sha256_hex(CORPUS_SOURCE).len(), 64);
        let parsed: Value = serde_json::from_slice(CORPUS_SOURCE).expect("embedded corpus");
        assert_eq!(sha256_hex(MORPHOLOGY_SOURCE).len(), 64);
        let model: Value =
            serde_json::from_slice(MORPHOLOGY_SOURCE).expect("embedded morphology");
        assert_eq!(
            validate_morphology(&model),
            Vec::<String>::new(),
            "embedded morphology should satisfy its declared graph contract"
        );
        assert_eq!(
            sha256_hex(&serde_json::to_vec(&parsed).expect("canonical test serialization")),
            sha256_hex(
                &serde_json::to_vec(&corpus()).expect("canonical corpus serialization")
            )
        );
    }
}
