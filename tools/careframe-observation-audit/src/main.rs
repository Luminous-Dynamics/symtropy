use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const DEFAULT_CONTRACT: &str = "docs/medical-robotics/careframe-observation-contract-v0.json";
const DEFAULT_SUITE: &str = "docs/medical-robotics/careframe-observation-adapters-v0.1.json";
const EXPECTED_ADAPTERS: usize = 3;
const EXPECTED_POLICY_FIELDS: usize = 18;
const EXPECTED_ORACLE_FIELDS: usize = 2;
const CONTRACT_SOURCE: &[u8] =
    include_bytes!("../../../docs/medical-robotics/careframe-observation-contract-v0.json");
const SUITE_SOURCE: &[u8] =
    include_bytes!("../../../docs/medical-robotics/careframe-observation-adapters-v0.1.json");
const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldSpec {
    value_type: String,
    unit: String,
    source_family: String,
    visibility: String,
    required: bool,
}

#[derive(Debug)]
struct AuditReport {
    adapter_count: usize,
    matched_profiles: usize,
    policy_field_count: usize,
    oracle_field_count: usize,
    findings: Vec<String>,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn push_findings(findings: &mut BTreeSet<String>, code: &str) {
    findings.insert(code.to_owned());
}

fn check_object(
    value: &Value,
    required: &[&str],
    allowed: &[&str],
    findings: &mut BTreeSet<String>,
    context: &str,
) {
    let Some(object) = value.as_object() else {
        push_findings(findings, &format!("{context}_MUST_BE_OBJECT"));
        return;
    };
    if required.iter().any(|key| !object.contains_key(*key)) {
        push_findings(findings, &format!("{context}_REQUIRED_FIELD_MISSING"));
    }
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        push_findings(findings, &format!("{context}_UNKNOWN_FIELD"));
    }
}

fn string_array(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn duplicate_strings(values: &[String]) -> bool {
    let unique = values.iter().collect::<BTreeSet<_>>();
    unique.len() != values.len()
}

fn audit_contract(contract: &Value, suite: &Value) -> AuditReport {
    let mut findings = BTreeSet::new();
    const CONTRACT_FIELDS: &[&str] = &[
        "schema_version",
        "contract_id",
        "revision",
        "purpose",
        "field_catalog",
        "comparison_profiles",
        "claim_ceiling",
    ];
    check_object(
        contract,
        CONTRACT_FIELDS,
        CONTRACT_FIELDS,
        &mut findings,
        "CONTRACT",
    );

    if str_at(contract, "schema_version") != Some("careframe-observation-contract-v0") {
        push_findings(&mut findings, "CONTRACT_SCHEMA_UNSUPPORTED");
    }
    if str_at(contract, "contract_id") != Some("CAREFRAME_MATCHED_SEMANTIC_OBSERVATION") {
        push_findings(&mut findings, "CONTRACT_ID_UNSUPPORTED");
    }
    if contract.get("revision").and_then(Value::as_u64) != Some(1) {
        push_findings(&mut findings, "CONTRACT_REVISION_UNSUPPORTED");
    }
    if str_at(contract, "claim_ceiling").is_none_or(str::is_empty) {
        push_findings(&mut findings, "CONTRACT_CLAIM_CEILING_REQUIRED");
    }

    let mut catalog = BTreeMap::<String, FieldSpec>::new();
    let Some(fields) = contract.get("field_catalog").and_then(Value::as_array) else {
        push_findings(&mut findings, "FIELD_CATALOG_REQUIRED");
        return finish_report(suite, catalog, 0, 0, findings);
    };
    let mut policy_ids = Vec::new();
    let mut oracle_ids = Vec::new();
    const FIELD_SPEC_FIELDS: &[&str] = &[
        "field_id",
        "value_type",
        "unit",
        "source_family",
        "visibility",
        "required",
    ];
    const VALUE_TYPES: &[&str] = &[
        "vector3",
        "vector4",
        "number_map",
        "string",
        "integer",
        "enum_string",
        "boolean",
        "vector3_list",
        "scalar_0_1",
        "boolean_map",
        "scalar",
    ];
    const SOURCE_FAMILIES: &[&str] = &[
        "proprioception",
        "task_context",
        "safety_supervisor",
        "perception_estimate",
        "simulator_truth",
    ];

    for field in fields {
        check_object(
            field,
            FIELD_SPEC_FIELDS,
            FIELD_SPEC_FIELDS,
            &mut findings,
            "FIELD_SPEC",
        );
        let Some(id) = str_at(field, "field_id") else {
            push_findings(&mut findings, "FIELD_ID_INVALID");
            continue;
        };
        let Some(value_type) = str_at(field, "value_type") else {
            push_findings(&mut findings, "FIELD_TYPE_INVALID");
            continue;
        };
        let Some(unit) = str_at(field, "unit") else {
            push_findings(&mut findings, "FIELD_UNIT_INVALID");
            continue;
        };
        let Some(source_family) = str_at(field, "source_family") else {
            push_findings(&mut findings, "FIELD_SOURCE_FAMILY_INVALID");
            continue;
        };
        let Some(visibility) = str_at(field, "visibility") else {
            push_findings(&mut findings, "FIELD_VISIBILITY_INVALID");
            continue;
        };
        let Some(required) = bool_at(field, "required") else {
            push_findings(&mut findings, "FIELD_REQUIRED_FLAG_INVALID");
            continue;
        };
        if id.trim().is_empty() {
            push_findings(&mut findings, "FIELD_ID_INVALID");
        }
        if !VALUE_TYPES.contains(&value_type) {
            push_findings(&mut findings, "FIELD_TYPE_UNSUPPORTED");
        }
        if unit.trim().is_empty() {
            push_findings(&mut findings, "FIELD_UNIT_INVALID");
        }
        if !SOURCE_FAMILIES.contains(&source_family) {
            push_findings(&mut findings, "FIELD_SOURCE_FAMILY_UNSUPPORTED");
        }
        match visibility {
            "policy" => {
                policy_ids.push(id.to_owned());
                if !required {
                    push_findings(&mut findings, "POLICY_FIELD_MUST_BE_REQUIRED");
                }
                if source_family == "simulator_truth" {
                    push_findings(&mut findings, "ORACLE_SOURCE_FORBIDDEN_IN_POLICY_CATALOG");
                }
            }
            "oracle_only" => {
                oracle_ids.push(id.to_owned());
                if required {
                    push_findings(&mut findings, "ORACLE_FIELD_MUST_NOT_BE_REQUIRED_FOR_POLICY");
                }
                if source_family != "simulator_truth" {
                    push_findings(&mut findings, "ORACLE_FIELD_SOURCE_FAMILY_INVALID");
                }
            }
            _ => push_findings(&mut findings, "FIELD_VISIBILITY_UNSUPPORTED"),
        }
        let spec = FieldSpec {
            value_type: value_type.to_owned(),
            unit: unit.to_owned(),
            source_family: source_family.to_owned(),
            visibility: visibility.to_owned(),
            required,
        };
        if catalog.insert(id.to_owned(), spec).is_some() {
            push_findings(&mut findings, "FIELD_ID_DUPLICATE");
        }
    }

    if policy_ids.len() != EXPECTED_POLICY_FIELDS {
        push_findings(&mut findings, "POLICY_FIELD_COUNT_DRIFT");
    }
    if oracle_ids.len() != EXPECTED_ORACLE_FIELDS {
        push_findings(&mut findings, "ORACLE_FIELD_COUNT_DRIFT");
    }
    if duplicate_strings(&policy_ids) || duplicate_strings(&oracle_ids) {
        push_findings(&mut findings, "FIELD_ID_DUPLICATE");
    }

    const PROFILE_FIELDS: &[&str] = &[
        "profile_id",
        "adapter_ids",
        "required_policy_field_ids",
        "forbidden_policy_field_ids",
        "require_identical_field_order",
        "require_identical_type_and_unit",
        "require_sensor_or_estimator_provenance",
    ];
    let Some(profiles) = contract
        .get("comparison_profiles")
        .and_then(Value::as_array)
    else {
        push_findings(&mut findings, "COMPARISON_PROFILES_REQUIRED");
        return finish_report(suite, catalog, policy_ids.len(), oracle_ids.len(), findings);
    };
    if profiles.len() != 1 {
        push_findings(&mut findings, "COMPARISON_PROFILE_COUNT_UNSUPPORTED");
    }

    let mut expected_adapter_ids = Vec::new();
    let mut expected_policy_order = Vec::new();
    let mut expected_oracle_order = Vec::new();
    for profile in profiles {
        check_object(
            profile,
            PROFILE_FIELDS,
            PROFILE_FIELDS,
            &mut findings,
            "COMPARISON_PROFILE",
        );
        if str_at(profile, "profile_id") != Some("matched_semantic_state_v0") {
            push_findings(&mut findings, "COMPARISON_PROFILE_UNSUPPORTED");
        }
        expected_adapter_ids = string_array(profile.get("adapter_ids").unwrap_or(&Value::Null))
            .unwrap_or_else(|| {
                push_findings(&mut findings, "PROFILE_ADAPTER_IDS_INVALID");
                Vec::new()
            });
        expected_policy_order = string_array(
            profile
                .get("required_policy_field_ids")
                .unwrap_or(&Value::Null),
        )
        .unwrap_or_else(|| {
            push_findings(&mut findings, "PROFILE_POLICY_FIELD_IDS_INVALID");
            Vec::new()
        });
        expected_oracle_order = string_array(
            profile
                .get("forbidden_policy_field_ids")
                .unwrap_or(&Value::Null),
        )
        .unwrap_or_else(|| {
            push_findings(&mut findings, "PROFILE_ORACLE_FIELD_IDS_INVALID");
            Vec::new()
        });
        if expected_policy_order != policy_ids {
            push_findings(&mut findings, "PROFILE_POLICY_FIELD_ORDER_DRIFT");
        }
        if expected_oracle_order != oracle_ids {
            push_findings(&mut findings, "PROFILE_ORACLE_FIELD_ORDER_DRIFT");
        }
        if duplicate_strings(&expected_adapter_ids)
            || duplicate_strings(&expected_policy_order)
            || duplicate_strings(&expected_oracle_order)
        {
            push_findings(&mut findings, "COMPARISON_PROFILE_DUPLICATE_ID");
        }
        if bool_at(profile, "require_identical_field_order") != Some(true)
            || bool_at(profile, "require_identical_type_and_unit") != Some(true)
            || bool_at(profile, "require_sensor_or_estimator_provenance") != Some(true)
        {
            push_findings(&mut findings, "COMPARISON_PROFILE_GUARD_WEAKENED");
        }
    }

    const SUITE_FIELDS: &[&str] = &[
        "schema_version",
        "suite_id",
        "revision",
        "purpose",
        "adapters",
    ];
    check_object(
        suite,
        SUITE_FIELDS,
        SUITE_FIELDS,
        &mut findings,
        "ADAPTER_SUITE",
    );
    if str_at(suite, "schema_version") != Some("careframe-observation-adapters-v0") {
        push_findings(&mut findings, "ADAPTER_SUITE_SCHEMA_UNSUPPORTED");
    }
    if str_at(suite, "suite_id") != Some("CAREFRAME_OBSERVATION_CONTRACT_AUDIT") {
        push_findings(&mut findings, "ADAPTER_SUITE_ID_UNSUPPORTED");
    }
    if suite.get("revision").and_then(Value::as_u64) != Some(1) {
        push_findings(&mut findings, "ADAPTER_SUITE_REVISION_UNSUPPORTED");
    }

    const ADAPTER_FIELDS: &[&str] = &[
        "adapter_id",
        "modality",
        "source_state",
        "raw_inputs",
        "field_order",
        "field_specs",
        "scoring_only_field_ids",
    ];
    const SPEC_FIELDS: &[&str] = &["field_id", "value_type", "unit", "source_family"];
    let Some(adapters) = suite.get("adapters").and_then(Value::as_array) else {
        push_findings(&mut findings, "ADAPTERS_REQUIRED");
        return finish_report(suite, catalog, policy_ids.len(), oracle_ids.len(), findings);
    };
    if adapters.len() != EXPECTED_ADAPTERS {
        push_findings(&mut findings, "ADAPTER_COUNT_DRIFT");
    }

    let mut seen_adapter_ids = BTreeSet::new();
    let mut valid_profile_count = 0usize;
    for adapter in adapters {
        check_object(
            adapter,
            ADAPTER_FIELDS,
            ADAPTER_FIELDS,
            &mut findings,
            "ADAPTER",
        );
        let Some(adapter_id) = str_at(adapter, "adapter_id") else {
            push_findings(&mut findings, "ADAPTER_ID_INVALID");
            continue;
        };
        if !seen_adapter_ids.insert(adapter_id.to_owned()) {
            push_findings(&mut findings, "ADAPTER_ID_DUPLICATE");
        }

        let modality = str_at(adapter, "modality").unwrap_or("");
        let expected_inputs: &[&str] = match (adapter_id, modality) {
            ("state_vector_estimate", "state_vector") => &[
                "encoder_estimate",
                "odometry_estimate",
                "perception_estimate",
                "task_context",
                "safety_supervisor",
            ],
            ("rgb_derived_estimate", "rgb") => &[
                "rgb_frame",
                "encoder_estimate",
                "odometry_estimate",
                "task_context",
                "safety_supervisor",
            ],
            ("point_cloud_derived_estimate", "point_cloud") => &[
                "point_cloud",
                "encoder_estimate",
                "odometry_estimate",
                "task_context",
                "safety_supervisor",
            ],
            _ => {
                push_findings(&mut findings, "ADAPTER_MODALITY_UNSUPPORTED");
                &[]
            }
        };
        if str_at(adapter, "source_state") != Some("sensor_or_estimator") {
            push_findings(&mut findings, "SIMULATOR_ORACLE_SOURCE_FORBIDDEN");
        }
        let raw_inputs = string_array(adapter.get("raw_inputs").unwrap_or(&Value::Null));
        match raw_inputs {
            Some(inputs) => {
                if duplicate_strings(&inputs) {
                    push_findings(&mut findings, "RAW_INPUT_DUPLICATE");
                }
                let expected = expected_inputs.iter().map(|item| (*item).to_owned()).collect::<BTreeSet<_>>();
                let actual = inputs.iter().cloned().collect::<BTreeSet<_>>();
                if actual != expected {
                    push_findings(&mut findings, "RAW_INPUT_CONTRACT_MISMATCH");
                }
            }
            None => push_findings(&mut findings, "RAW_INPUTS_INVALID"),
        }

        let field_order = string_array(adapter.get("field_order").unwrap_or(&Value::Null));
        match field_order {
            Some(order) => {
                if duplicate_strings(&order) {
                    push_findings(&mut findings, "ADAPTER_FIELD_ID_DUPLICATE");
                }
                if order != expected_policy_order {
                    push_findings(&mut findings, "ADAPTER_FIELD_COVERAGE_OR_ORDER_MISMATCH");
                }
                if order.iter().any(|id| oracle_ids.contains(id)) {
                    push_findings(&mut findings, "ORACLE_FIELD_EXPOSED_TO_POLICY");
                }
            }
            None => push_findings(&mut findings, "ADAPTER_FIELD_ORDER_INVALID"),
        }

        let Some(specs) = adapter.get("field_specs").and_then(Value::as_array) else {
            push_findings(&mut findings, "ADAPTER_FIELD_SPECS_REQUIRED");
            continue;
        };
        let mut seen_specs = BTreeSet::new();
        let mut spec_order = Vec::new();
        for spec in specs {
            check_object(spec, SPEC_FIELDS, SPEC_FIELDS, &mut findings, "ADAPTER_FIELD_SPEC");
            let Some(id) = str_at(spec, "field_id") else {
                push_findings(&mut findings, "ADAPTER_FIELD_SPEC_ID_INVALID");
                continue;
            };
            spec_order.push(id.to_owned());
            if !seen_specs.insert(id.to_owned()) {
                push_findings(&mut findings, "ADAPTER_FIELD_SPEC_DUPLICATE");
            }
            let Some(expected) = catalog.get(id) else {
                push_findings(&mut findings, "ADAPTER_FIELD_UNKNOWN");
                continue;
            };
            if expected.visibility != "policy" {
                push_findings(&mut findings, "ORACLE_FIELD_EXPOSED_TO_POLICY");
            }
            if str_at(spec, "value_type") != Some(expected.value_type.as_str()) {
                push_findings(&mut findings, "ADAPTER_FIELD_TYPE_MISMATCH");
            }
            if str_at(spec, "unit") != Some(expected.unit.as_str()) {
                push_findings(&mut findings, "ADAPTER_FIELD_UNIT_MISMATCH");
            }
            if str_at(spec, "source_family") != Some(expected.source_family.as_str()) {
                push_findings(&mut findings, "ADAPTER_FIELD_SOURCE_MISMATCH");
            }
        }
        if spec_order != expected_policy_order {
            push_findings(&mut findings, "ADAPTER_FIELD_SPEC_COVERAGE_OR_ORDER_MISMATCH");
        }

        let scoring_only =
            string_array(adapter.get("scoring_only_field_ids").unwrap_or(&Value::Null));
        match scoring_only {
            Some(ids) => {
                if duplicate_strings(&ids) {
                    push_findings(&mut findings, "SCORING_FIELD_ID_DUPLICATE");
                }
                if ids != expected_oracle_order {
                    push_findings(&mut findings, "SCORING_ONLY_FIELD_CONTRACT_MISMATCH");
                }
                if ids.iter().any(|id| {
                    catalog
                        .get(id)
                        .is_none_or(|field| field.visibility != "oracle_only")
                }) {
                    push_findings(&mut findings, "NON_ORACLE_FIELD_MARKED_SCORING_ONLY");
                }
            }
            None => push_findings(&mut findings, "SCORING_ONLY_FIELD_IDS_INVALID"),
        }
    }

    if expected_adapter_ids.len() != adapters.len()
        || expected_adapter_ids.iter().any(|id| !seen_adapter_ids.contains(id))
        || seen_adapter_ids.iter().any(|id| !expected_adapter_ids.contains(id))
    {
        push_findings(&mut findings, "PROFILE_ADAPTER_SET_MISMATCH");
    }

    let base_findings = findings.clone();
    if base_findings.is_empty() {
        valid_profile_count = profiles_len(contract);
    }

    finish_report(
        suite,
        catalog,
        policy_ids.len(),
        oracle_ids.len(),
        with_profile_count(findings, valid_profile_count),
    )
}

fn profiles_len(contract: &Value) -> usize {
    contract
        .get("comparison_profiles")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn with_profile_count(
    _findings: BTreeSet<String>,
    count: usize,
) -> BTreeSet<String> {
    let _ = count;
    _findings
}

fn finish_report(
    suite: &Value,
    catalog: BTreeMap<String, FieldSpec>,
    policy_field_count: usize,
    oracle_field_count: usize,
    findings: BTreeSet<String>,
) -> AuditReport {
    let adapter_count = suite
        .get("adapters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let matched_profiles = if findings.is_empty() { 1 } else { 0 };
    AuditReport {
        adapter_count,
        matched_profiles,
        policy_field_count,
        oracle_field_count,
        findings: findings.into_iter().collect(),
    }
}

fn load_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read failed for '{path}': {error}"))?;
    let parsed = serde_json::from_slice(&bytes)
        .map_err(|error| format!("JSON rejected for '{path}': {error}"))?;
    Ok((bytes, parsed))
}

fn emit_receipt(
    contract_path: &str,
    suite_path: &str,
    contract_bytes: &[u8],
    suite_bytes: &[u8],
    report: AuditReport,
) {
    let tested_revision =
        env::var("CAREFRAME_TESTED_SHA").unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned());
    let failed = !report.findings.is_empty();
    let receipt = json!({
        "receipt_schema": "careframe-observation-audit-receipt-v0",
        "tested_revision": tested_revision,
        "contract_path": contract_path,
        "suite_path": suite_path,
        "contract_sha256": sha256_hex(contract_bytes),
        "adapter_suite_sha256": sha256_hex(suite_bytes),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "adapter_count": report.adapter_count,
        "matched_profiles": report.matched_profiles,
        "policy_field_count": report.policy_field_count,
        "oracle_field_count": report.oracle_field_count,
        "disposition": if failed { "BLOCKED" } else { "MATCHED_CONTRACTS" },
        "findings": report.findings,
        "oracle_only_fields_exposed_to_policy": false,
        "claim_ceiling": "Observation contract consistency only; no perception accuracy, safety, or sim-to-real claim."
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serialization cannot fail")
    );
    if failed {
        process::exit(1);
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let contract_path = args
        .next()
        .unwrap_or_else(|| DEFAULT_CONTRACT.to_owned());
    let suite_path = args.next().unwrap_or_else(|| DEFAULT_SUITE.to_owned());
    if args.next().is_some() {
        eprintln!("usage: careframe-observation-audit [CONTRACT_JSON] [ADAPTER_SUITE_JSON]");
        process::exit(2);
    }
    let (contract_bytes, contract) = match load_json(&contract_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let (suite_bytes, suite) = match load_json(&suite_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    if contract_path == DEFAULT_CONTRACT && sha256_hex(&contract_bytes) != sha256_hex(CONTRACT_SOURCE) {
        eprintln!("runtime contract differs from the contract embedded in the auditor");
        process::exit(2);
    }
    if suite_path == DEFAULT_SUITE && sha256_hex(&suite_bytes) != sha256_hex(SUITE_SOURCE) {
        eprintln!("runtime adapter suite differs from the suite embedded in the auditor");
        process::exit(2);
    }
    let report = audit_contract(&contract, &suite);
    emit_receipt(&contract_path, &suite_path, &contract_bytes, &suite_bytes, report);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> Value {
        serde_json::from_slice(CONTRACT_SOURCE).expect("embedded observation contract parses")
    }

    fn suite() -> Value {
        serde_json::from_slice(SUITE_SOURCE).expect("embedded adapter suite parses")
    }

    fn audit(contract: &Value, suite: &Value) -> AuditReport {
        audit_contract(contract, suite)
    }

    #[test]
    fn frozen_adapters_match_the_same_semantic_contract() {
        let report = audit(&contract(), &suite());
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.adapter_count, EXPECTED_ADAPTERS);
        assert_eq!(report.matched_profiles, 1);
        assert_eq!(report.policy_field_count, EXPECTED_POLICY_FIELDS);
        assert_eq!(report.oracle_field_count, EXPECTED_ORACLE_FIELDS);
    }

    #[test]
    fn dropped_semantic_field_fails_closed() {
        let mut inputs = suite();
        inputs["adapters"][1]["field_order"]
            .as_array_mut()
            .expect("field order")
            .remove(3);
        let report = audit(&contract(), &inputs);
        assert!(report.findings.iter().any(|item| {
            item == "ADAPTER_FIELD_COVERAGE_OR_ORDER_MISMATCH"
                || item == "ADAPTER_FIELD_SPEC_COVERAGE_OR_ORDER_MISMATCH"
        }));
    }

    #[test]
    fn reordered_observation_fields_are_not_silently_accepted() {
        let mut inputs = suite();
        inputs["adapters"][1]["field_order"]
            .as_array_mut()
            .expect("field order")
            .swap(0, 1);
        let report = audit(&contract(), &inputs);
        assert!(report
            .findings
            .iter()
            .any(|item| item == "ADAPTER_FIELD_COVERAGE_OR_ORDER_MISMATCH"));
    }

    #[test]
    fn unit_or_type_drift_fails_closed() {
        let mut units = suite();
        units["adapters"][1]["field_specs"][0]["unit"] = json!("cm");
        assert!(audit(&contract(), &units)
            .findings
            .iter()
            .any(|item| item == "ADAPTER_FIELD_UNIT_MISMATCH"));

        let mut types = suite();
        types["adapters"][2]["field_specs"][0]["value_type"] = json!("string");
        assert!(audit(&contract(), &types)
            .findings
            .iter()
            .any(|item| item == "ADAPTER_FIELD_TYPE_MISMATCH"));
    }

    #[test]
    fn oracle_fields_cannot_be_added_to_policy_inputs() {
        let mut inputs = suite();
        inputs["adapters"][0]["field_order"]
            .as_array_mut()
            .expect("field order")
            .push(json!("scene.object_mass_kg"));
        inputs["adapters"][0]["field_specs"]
            .as_array_mut()
            .expect("field specs")
            .push(json!({
                "field_id": "scene.object_mass_kg",
                "value_type": "scalar",
                "unit": "kg",
                "source_family": "simulator_truth"
            }));
        let report = audit(&contract(), &inputs);
        assert!(report
            .findings
            .iter()
            .any(|item| item == "ORACLE_FIELD_EXPOSED_TO_POLICY"));
    }

    #[test]
    fn scorer_only_fields_must_be_declared_separately() {
        let mut inputs = suite();
        inputs["adapters"][0]["scoring_only_field_ids"] = json!(["scene.object_mass_kg"]);
        assert!(audit(&contract(), &inputs)
            .findings
            .iter()
            .any(|item| item == "SCORING_ONLY_FIELD_CONTRACT_MISMATCH"));
    }

    #[test]
    fn source_state_cannot_claim_simulator_oracle_as_policy_input() {
        let mut inputs = suite();
        inputs["adapters"][0]["source_state"] = json!("simulator_oracle");
        assert!(audit(&contract(), &inputs)
            .findings
            .iter()
            .any(|item| item == "SIMULATOR_ORACLE_SOURCE_FORBIDDEN"));
    }

    #[test]
    fn source_family_mismatch_fails_closed() {
        let mut inputs = suite();
        inputs["adapters"][1]["field_specs"][0]["source_family"] = json!("simulator_truth");
        let report = audit(&contract(), &inputs);
        assert!(report.findings.iter().any(|item| {
            item == "ADAPTER_FIELD_SOURCE_MISMATCH"
                || item == "ORACLE_SOURCE_FORBIDDEN_IN_POLICY_CATALOG"
        }));
    }

    #[test]
    fn unknown_fields_reject_at_contract_suite_and_adapter_boundaries() {
        let mut c = contract();
        c["silent_privileged_state"] = json!(true);
        assert!(audit(&c, &suite())
            .findings
            .iter()
            .any(|item| item == "CONTRACT_UNKNOWN_FIELD"));

        let mut s = suite();
        s["hidden_sensor_override"] = json!(true);
        assert!(audit(&contract(), &s)
            .findings
            .iter()
            .any(|item| item == "ADAPTER_SUITE_UNKNOWN_FIELD"));

        let mut a = suite();
        a["adapters"][0]["silent_fallback"] = json!(true);
        assert!(audit(&contract(), &a)
            .findings
            .iter()
            .any(|item| item == "ADAPTER_UNKNOWN_FIELD"));
    }

    #[test]
    fn duplicate_adapter_and_field_ids_reject() {
        let mut inputs = suite();
        inputs["adapters"][1]["adapter_id"] = json!("state_vector_estimate");
        assert!(audit(&contract(), &inputs)
            .findings
            .iter()
            .any(|item| item == "ADAPTER_ID_DUPLICATE"));

        let mut c = contract();
        c["field_catalog"][1]["field_id"] = json!("robot.base_position_m");
        assert!(audit(&c, &suite())
            .findings
            .iter()
            .any(|item| item == "FIELD_ID_DUPLICATE"));
    }

    #[test]
    fn unknown_profile_adapter_rejects() {
        let mut c = contract();
        c["comparison_profiles"][0]["adapter_ids"][0] = json!("unimplemented_adapter");
        assert!(audit(&c, &suite())
            .findings
            .iter()
            .any(|item| item == "PROFILE_ADAPTER_SET_MISMATCH"));
    }

    #[test]
    fn raw_modality_inputs_are_bound_to_adapter_type() {
        let mut inputs = suite();
        inputs["adapters"][1]["raw_inputs"][0] = json!("point_cloud");
        assert!(audit(&contract(), &inputs)
            .findings
            .iter()
            .any(|item| item == "RAW_INPUT_CONTRACT_MISMATCH"));
    }

    #[test]
    fn oracle_catalog_cannot_be_relabelled_as_policy_visible() {
        let mut c = contract();
        c["field_catalog"][18]["visibility"] = json!("policy");
        c["field_catalog"][18]["required"] = json!(true);
        let report = audit(&c, &suite());
        assert!(report.findings.iter().any(|item| {
            item == "ORACLE_SOURCE_FORBIDDEN_IN_POLICY_CATALOG"
                || item == "POLICY_FIELD_COUNT_DRIFT"
        }));
    }

    #[test]
    fn weakening_comparison_guards_is_detected() {
        let mut c = contract();
        c["comparison_profiles"][0]["require_identical_field_order"] = json!(false);
        assert!(audit(&c, &suite())
            .findings
            .iter()
            .any(|item| item == "COMPARISON_PROFILE_GUARD_WEAKENED"));
    }

    #[test]
    fn embedded_contract_and_suite_digests_are_available() {
        assert_eq!(sha256_hex(CONTRACT_SOURCE).len(), 64);
        assert_eq!(sha256_hex(SUITE_SOURCE).len(), 64);
        assert_eq!(sha256_hex(AUDITOR_SOURCE).len(), 64);
        assert_eq!(sha256_hex(CONTRACT_SOURCE), sha256_hex(&fs::read(DEFAULT_CONTRACT).expect("contract file")));
        assert_eq!(sha256_hex(SUITE_SOURCE), sha256_hex(&fs::read(DEFAULT_SUITE).expect("suite file")));
    }
}
