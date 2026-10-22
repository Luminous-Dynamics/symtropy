use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, process};

const DEFAULT_CORPUS: &str = "docs/tech/hmr0-tool-candidate-corpus-v0.1.json";
const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");
const EMBEDDED_CORPUS: &[u8] =
    include_bytes!("../../../docs/tech/hmr0-tool-candidate-corpus-v0.1.json");
const ELIGIBLE: &str = "eligible_for_simulation_review";
const BLOCKED: &str = "blocked";
const REJECTED: &str = "rejected";
const CLAIM_CEILING: &str = "Synthetic tool-manifest screening only. No CAD correctness, physical safety, medical-device conformity, fabrication or deployment claim.";

const ALLOWED_FIELDS: &[&str] = &[
    "schema_version",
    "tool_id",
    "revision",
    "display_name",
    "intended_use",
    "evidence_class",
    "simulation_only",
    "interaction_domain",
    "patient_contact",
    "clinical_function",
    "medication_delivery",
    "diagnostic_output",
    "invasive_or_surgical",
    "sterile_field_access",
    "physical_actuation_enabled",
    "mount_interface_id",
    "geometry_sha256",
    "simulation_model_sha256",
    "declared_max_force_n",
    "fixture_limit_force_n",
    "calibration_status",
    "cleaning_status",
    "tool_attachment_identity_verified",
    "tool_detach_detection",
    "required_safety_interlock",
    "independent_stop_required",
    "failure_release_mode",
    "sharp_or_piercing_feature",
    "energy_emission",
];

const REQUIRED_STRINGS: &[&str] = &[
    "schema_version",
    "tool_id",
    "revision",
    "display_name",
    "intended_use",
    "evidence_class",
    "interaction_domain",
    "clinical_function",
    "mount_interface_id",
    "geometry_sha256",
    "simulation_model_sha256",
    "calibration_status",
    "cleaning_status",
    "failure_release_mode",
];

const REQUIRED_BOOLS: &[&str] = &[
    "simulation_only",
    "patient_contact",
    "medication_delivery",
    "diagnostic_output",
    "invasive_or_surgical",
    "sterile_field_access",
    "physical_actuation_enabled",
    "tool_attachment_identity_verified",
    "tool_detach_detection",
    "required_safety_interlock",
    "independent_stop_required",
    "sharp_or_piercing_feature",
    "energy_emission",
];

const NUMERIC_FIELDS: &[&str] = &["declared_max_force_n", "fixture_limit_force_n"];

#[derive(Debug, PartialEq, Eq)]
struct CandidateEvaluation {
    disposition: &'static str,
    reason_codes: Vec<String>,
}

#[derive(Debug)]
struct CorpusReport {
    case_count: usize,
    matched_case_count: usize,
    mismatches: Vec<String>,
    disposition_counts: std::collections::BTreeMap<String, usize>,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn push_once(reasons: &mut Vec<String>, reason: &str) {
    if !reasons.iter().any(|existing| existing == reason) {
        reasons.push(reason.to_owned());
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn reject_unknown_fields(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    reasons: &mut Vec<String>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            push_once(reasons, "UNKNOWN_FIELD");
        }
    }
}

fn evaluate_candidate(candidate: &Value, allowed_mount: &str) -> CandidateEvaluation {
    let Some(object) = candidate.as_object() else {
        return CandidateEvaluation {
            disposition: REJECTED,
            reason_codes: vec!["MANIFEST_NOT_OBJECT".to_owned()],
        };
    };

    let mut schema_errors = Vec::new();
    reject_unknown_fields(object, ALLOWED_FIELDS, &mut schema_errors);

    if ALLOWED_FIELDS
        .iter()
        .any(|field| !object.contains_key(*field))
    {
        push_once(&mut schema_errors, "REQUIRED_FIELD_MISSING");
    }
    if !schema_errors.is_empty() {
        return CandidateEvaluation {
            disposition: REJECTED,
            reason_codes: schema_errors,
        };
    }

    for field in REQUIRED_STRINGS {
        if candidate
            .get(*field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            push_once(&mut schema_errors, "FIELD_TYPE_OR_VALUE_INVALID");
        }
    }
    for field in REQUIRED_BOOLS {
        if candidate.get(*field).and_then(Value::as_bool).is_none() {
            push_once(&mut schema_errors, "FIELD_TYPE_OR_VALUE_INVALID");
        }
    }
    for field in NUMERIC_FIELDS {
        if candidate
            .get(*field)
            .and_then(Value::as_f64)
            .is_none_or(|value| !value.is_finite() || value < 0.0)
        {
            push_once(&mut schema_errors, "FIELD_TYPE_OR_VALUE_INVALID");
        }
    }
    if !schema_errors.is_empty() {
        return CandidateEvaluation {
            disposition: REJECTED,
            reason_codes: schema_errors,
        };
    }

    let mut reasons = Vec::new();
    if str_at(candidate, "schema_version") != Some("hmr0-tool-manifest-v0.1") {
        push_once(&mut reasons, "SCHEMA_VERSION_UNSUPPORTED");
    }
    if str_at(candidate, "evidence_class") != Some("synthetic_tool_fixture") {
        push_once(&mut reasons, "EVIDENCE_CLASS_INVALID");
    }
    if candidate.get("simulation_only").and_then(Value::as_bool) != Some(true) {
        push_once(&mut reasons, "SIMULATION_ONLY_REQUIRED");
    }
    if candidate
        .get("patient_contact")
        .and_then(Value::as_bool)
        == Some(true)
        || str_at(candidate, "interaction_domain") == Some("patient_contact")
    {
        push_once(&mut reasons, "PATIENT_CONTACT_OUT_OF_SCOPE");
    }
    if candidate
        .get("sterile_field_access")
        .and_then(Value::as_bool)
        == Some(true)
    {
        push_once(&mut reasons, "STERILE_FIELD_ACCESS_OUT_OF_SCOPE");
    } else if str_at(candidate, "interaction_domain") != Some("non_patient_object")
        && str_at(candidate, "interaction_domain") != Some("patient_contact")
    {
        push_once(&mut reasons, "INTERACTION_DOMAIN_OUT_OF_SCOPE");
    }
    if str_at(candidate, "clinical_function") != Some("none")
        || candidate
            .get("medication_delivery")
            .and_then(Value::as_bool)
            == Some(true)
        || candidate
            .get("diagnostic_output")
            .and_then(Value::as_bool)
            == Some(true)
        || candidate
            .get("invasive_or_surgical")
            .and_then(Value::as_bool)
            == Some(true)
    {
        push_once(&mut reasons, "CLINICAL_FUNCTION_OUT_OF_SCOPE");
    }
    if candidate
        .get("physical_actuation_enabled")
        .and_then(Value::as_bool)
        == Some(true)
    {
        push_once(&mut reasons, "PHYSICAL_ACTUATION_FORBIDDEN");
    }
    if str_at(candidate, "mount_interface_id") != Some(allowed_mount) {
        push_once(&mut reasons, "TOOL_MOUNT_UNVERIFIED");
    }
    if str_at(candidate, "geometry_sha256").is_none_or(|value| !valid_sha256(value)) {
        push_once(&mut reasons, "GEOMETRY_DIGEST_INVALID");
    }
    if str_at(candidate, "simulation_model_sha256")
        .is_none_or(|value| !valid_sha256(value))
    {
        push_once(&mut reasons, "SIMULATION_MODEL_DIGEST_INVALID");
    }
    let declared_force = candidate
        .get("declared_max_force_n")
        .and_then(Value::as_f64)
        .unwrap_or(f64::INFINITY);
    let fixture_limit = candidate
        .get("fixture_limit_force_n")
        .and_then(Value::as_f64)
        .unwrap_or(-1.0);
    if declared_force > fixture_limit {
        push_once(&mut reasons, "FIXTURE_FORCE_LIMIT_EXCEEDED");
    }
    if str_at(candidate, "calibration_status") != Some("current_for_simulation") {
        push_once(&mut reasons, "TOOL_CALIBRATION_NOT_CURRENT");
    }
    if str_at(candidate, "cleaning_status") != Some("not_assessed") {
        push_once(&mut reasons, "CLEANING_EVIDENCE_UNVERIFIED");
    }
    if candidate
        .get("tool_attachment_identity_verified")
        .and_then(Value::as_bool)
        != Some(true)
    {
        push_once(&mut reasons, "TOOL_ATTACHMENT_IDENTITY_MISSING");
    }
    if candidate.get("tool_detach_detection").and_then(Value::as_bool) != Some(true) {
        push_once(&mut reasons, "TOOL_DETACH_DETECTION_MISSING");
    }
    if candidate
        .get("required_safety_interlock")
        .and_then(Value::as_bool)
        != Some(true)
        || candidate
            .get("independent_stop_required")
            .and_then(Value::as_bool)
            != Some(true)
    {
        push_once(&mut reasons, "INDEPENDENT_SAFETY_BOUNDARY_MISSING");
    }
    if str_at(candidate, "failure_release_mode") != Some("passive_safe_release") {
        push_once(&mut reasons, "FAILURE_RELEASE_UNDEFINED");
    }
    if candidate
        .get("sharp_or_piercing_feature")
        .and_then(Value::as_bool)
        == Some(true)
    {
        push_once(&mut reasons, "SHARP_OR_PIERCING_FEATURE_OUT_OF_SCOPE");
    }
    if candidate.get("energy_emission").and_then(Value::as_bool) == Some(true) {
        push_once(&mut reasons, "ENERGY_EMISSION_OUT_OF_SCOPE");
    }

    CandidateEvaluation {
        disposition: if reasons.is_empty() {
            ELIGIBLE
        } else {
            BLOCKED
        },
        reason_codes: reasons,
    }
}

fn evaluate_corpus(root: &Value) -> Result<CorpusReport, Vec<String>> {
    let mut errors = Vec::new();
    let root_object = root
        .as_object()
        .ok_or_else(|| vec!["corpus root must be an object".to_owned()])?;
    let allowed_root = [
        "suite_id",
        "suite_version",
        "evidence_class",
        "purpose",
        "physical_actuation_enabled",
        "patient_contact_enabled",
        "clinical_function_enabled",
        "claim_ceiling",
        "fixture_digest_note",
        "expected_case_count",
        "allowed_mount_interface_id",
        "cases",
    ];
    if root_object
        .keys()
        .any(|key| !allowed_root.contains(&key.as_str()))
    {
        errors.push("corpus contains unknown root fields".to_owned());
    }
    if str_at(root, "suite_id") != Some("HMR0_TOOL_QUALIFICATION") {
        errors.push("unsupported suite_id".to_owned());
    }
    if str_at(root, "suite_version") != Some("0.1.0") {
        errors.push("unsupported suite_version".to_owned());
    }
    if str_at(root, "evidence_class") != Some("synthetic") {
        errors.push("evidence_class must remain synthetic".to_owned());
    }
    for field in [
        "physical_actuation_enabled",
        "patient_contact_enabled",
        "clinical_function_enabled",
    ] {
        if root.get(field).and_then(Value::as_bool) != Some(false) {
            errors.push(format!("{field} must remain false"));
        }
    }
    if str_at(root, "claim_ceiling").is_none() || str_at(root, "fixture_digest_note").is_none() {
        errors.push("claim ceiling and fixture digest note are required".to_owned());
    }
    let allowed_mount = str_at(root, "allowed_mount_interface_id")
        .ok_or_else(|| vec!["allowed_mount_interface_id missing".to_owned()])?;
    let cases = root
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["cases must be an array".to_owned()])?;
    if root.get("expected_case_count").and_then(Value::as_u64) != Some(cases.len() as u64) {
        errors.push("expected_case_count differs from case count".to_owned());
    }

    let mut seen_ids = BTreeSet::new();
    let mut matched = 0usize;
    let mut mismatches = Vec::new();
    let mut disposition_counts = std::collections::BTreeMap::new();

    for case in cases {
        let Some(case_object) = case.as_object() else {
            errors.push("case is not an object".to_owned());
            continue;
        };
        let allowed_case = [
            "case_id",
            "label",
            "candidate",
            "expected_disposition",
            "expected_reason_codes",
        ];
        if case_object
            .keys()
            .any(|key| !allowed_case.contains(&key.as_str()))
        {
            errors.push("case contains unknown fields".to_owned());
            continue;
        }
        let Some(case_id) = str_at(case, "case_id") else {
            errors.push("case_id missing".to_owned());
            continue;
        };
        if !seen_ids.insert(case_id.to_owned()) {
            errors.push(format!("duplicate case_id {case_id}"));
            continue;
        }
        if str_at(case, "label").is_none() {
            errors.push(format!("{case_id}: label missing"));
            continue;
        }
        let expected_disposition = str_at(case, "expected_disposition");
        let expected_reasons: Option<Vec<String>> = case
            .get("expected_reason_codes")
            .and_then(Value::as_array)
            .and_then(|values| {
                values
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect()
            });
        if expected_disposition.is_none() || expected_reasons.is_none() {
            errors.push(format!("{case_id}: malformed expected outcome"));
            continue;
        }

        let actual = evaluate_candidate(&case["candidate"], allowed_mount);
        *disposition_counts
            .entry(actual.disposition.to_owned())
            .or_insert(0) += 1;
        if Some(actual.disposition) == expected_disposition
            && expected_reasons.as_ref() == Some(&actual.reason_codes)
        {
            matched += 1;
        } else {
            mismatches.push(format!(
                "{case_id}: actual {} {:?}; expected {} {:?}",
                actual.disposition,
                actual.reason_codes,
                expected_disposition.unwrap_or("<missing>"),
                expected_reasons.as_deref().unwrap_or(&[])
            ));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(CorpusReport {
        case_count: cases.len(),
        matched_case_count: matched,
        mismatches,
        disposition_counts,
    })
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("corpus read failed for '{path}': {error}");
            process::exit(2);
        }
    };
    if path == DEFAULT_CORPUS && sha256_hex(&bytes) != sha256_hex(EMBEDDED_CORPUS) {
        eprintln!("runtime corpus differs from the corpus embedded in the compiled evaluator");
        process::exit(2);
    }
    let root: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("corpus JSON rejected: {error}");
            process::exit(2);
        }
    };
    let report = match evaluate_corpus(&root) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("REJECT: {error}");
            }
            process::exit(2);
        }
    };
    let failed = !report.mismatches.is_empty();
    let receipt = json!({
        "receipt_schema": "hmr0-tool-qualification-receipt-v0",
        "tested_revision": env::var("MED_TOOL_TESTED_SHA")
            .unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned()),
        "suite_id": str_at(&root, "suite_id").unwrap_or("unknown"),
        "suite_version": str_at(&root, "suite_version").unwrap_or("unknown"),
        "corpus_path": path,
        "corpus_sha256": sha256_hex(&bytes),
        "embedded_corpus_sha256": sha256_hex(EMBEDDED_CORPUS),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "case_count": report.case_count,
        "matched_case_count": report.matched_case_count,
        "mismatch_count": report.mismatches.len(),
        "mismatches": report.mismatches,
        "disposition_counts": report.disposition_counts,
        "physical_actuation_authorized": false,
        "patient_contact_authorized": false,
        "clinical_function_authorized": false,
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
        serde_json::from_slice(EMBEDDED_CORPUS).expect("embedded corpus parses")
    }

    fn baseline_candidate() -> Value {
        corpus()["cases"][0]["candidate"].clone()
    }

    fn check_blocked(candidate: Value, reason: &str) {
        let result = evaluate_candidate(&candidate, "HMR0-TOOL-MOUNT-V0");
        assert_eq!(result.disposition, BLOCKED);
        assert!(
            result.reason_codes.iter().any(|code| code == reason),
            "expected {reason}; actual {:?}",
            result.reason_codes
        );
    }

    #[test]
    fn frozen_tool_manifest_corpus_matches_all_expected_outcomes() {
        let report = evaluate_corpus(&corpus()).expect("corpus structure valid");
        assert_eq!(report.case_count, 15);
        assert_eq!(report.matched_case_count, 15);
        assert!(report.mismatches.is_empty(), "{:?}", report.mismatches);
        assert_eq!(report.disposition_counts.get(ELIGIBLE), Some(&3));
        assert_eq!(report.disposition_counts.get(BLOCKED), Some(&10));
        assert_eq!(report.disposition_counts.get(REJECTED), Some(&2));
    }

    #[test]
    fn patient_contact_diagnostic_and_medication_tools_are_never_eligible() {
        let mut candidate = baseline_candidate();
        candidate["patient_contact"] = Value::Bool(true);
        check_blocked(candidate, "PATIENT_CONTACT_OUT_OF_SCOPE");

        let mut candidate = baseline_candidate();
        candidate["diagnostic_output"] = Value::Bool(true);
        check_blocked(candidate, "CLINICAL_FUNCTION_OUT_OF_SCOPE");

        let mut candidate = baseline_candidate();
        candidate["medication_delivery"] = Value::Bool(true);
        check_blocked(candidate, "CLINICAL_FUNCTION_OUT_OF_SCOPE");
    }

    #[test]
    fn all_independent_safety_requirements_are_non_optional() {
        let mut candidate = baseline_candidate();
        candidate["required_safety_interlock"] = Value::Bool(false);
        check_blocked(candidate, "INDEPENDENT_SAFETY_BOUNDARY_MISSING");

        let mut candidate = baseline_candidate();
        candidate["independent_stop_required"] = Value::Bool(false);
        check_blocked(candidate, "INDEPENDENT_SAFETY_BOUNDARY_MISSING");

        let mut candidate = baseline_candidate();
        candidate["tool_attachment_identity_verified"] = Value::Bool(false);
        check_blocked(candidate, "TOOL_ATTACHMENT_IDENTITY_MISSING");

        let mut candidate = baseline_candidate();
        candidate["tool_detach_detection"] = Value::Bool(false);
        check_blocked(candidate, "TOOL_DETACH_DETECTION_MISSING");
    }

    #[test]
    fn force_limit_is_fixture_specific_and_cannot_be_exceeded() {
        let mut candidate = baseline_candidate();
        candidate["declared_max_force_n"] = json!(26.0);
        candidate["fixture_limit_force_n"] = json!(25.0);
        check_blocked(candidate, "FIXTURE_FORCE_LIMIT_EXCEEDED");
    }

    #[test]
    fn unknown_fields_and_missing_fields_are_rejected_before_scoring() {
        let mut candidate = baseline_candidate();
        candidate["override_safety_controller"] = Value::Bool(true);
        let result = evaluate_candidate(&candidate, "HMR0-TOOL-MOUNT-V0");
        assert_eq!(result.disposition, REJECTED);
        assert_eq!(result.reason_codes, vec!["UNKNOWN_FIELD"]);

        let mut candidate = baseline_candidate();
        candidate
            .as_object_mut()
            .expect("candidate object")
            .remove("independent_stop_required");
        let result = evaluate_candidate(&candidate, "HMR0-TOOL-MOUNT-V0");
        assert_eq!(result.disposition, REJECTED);
        assert_eq!(result.reason_codes, vec!["REQUIRED_FIELD_MISSING"]);
    }

    #[test]
    fn invalid_geometry_or_model_digests_block_simulation_review() {
        let mut candidate = baseline_candidate();
        candidate["geometry_sha256"] = json!("bad");
        check_blocked(candidate, "GEOMETRY_DIGEST_INVALID");

        let mut candidate = baseline_candidate();
        candidate["simulation_model_sha256"] = json!("not-a-digest");
        check_blocked(candidate, "SIMULATION_MODEL_DIGEST_INVALID");
    }

    #[test]
    fn expired_calibration_and_claimed_cleanability_are_blocked() {
        let mut candidate = baseline_candidate();
        candidate["calibration_status"] = json!("expired");
        check_blocked(candidate, "TOOL_CALIBRATION_NOT_CURRENT");

        let mut candidate = baseline_candidate();
        candidate["cleaning_status"] = json!("validated");
        check_blocked(candidate, "CLEANING_EVIDENCE_UNVERIFIED");
    }

    #[test]
    fn no_tool_manifest_can_enable_physical_actuation() {
        let mut candidate = baseline_candidate();
        candidate["physical_actuation_enabled"] = Value::Bool(true);
        check_blocked(candidate, "PHYSICAL_ACTUATION_FORBIDDEN");
    }

    #[test]
    fn non_patient_fixture_tool_must_match_mount_and_release_contract() {
        let mut candidate = baseline_candidate();
        candidate["mount_interface_id"] = json!("UNREVIEWED-MOUNT");
        check_blocked(candidate, "TOOL_MOUNT_UNVERIFIED");

        let mut candidate = baseline_candidate();
        candidate["failure_release_mode"] = json!("unknown");
        check_blocked(candidate, "FAILURE_RELEASE_UNDEFINED");
    }

    #[test]
    fn sterile_field_and_sharp_energy_tools_are_out_of_scope() {
        let mut candidate = baseline_candidate();
        candidate["sterile_field_access"] = Value::Bool(true);
        check_blocked(candidate, "STERILE_FIELD_ACCESS_OUT_OF_SCOPE");

        let mut candidate = baseline_candidate();
        candidate["sharp_or_piercing_feature"] = Value::Bool(true);
        check_blocked(candidate, "SHARP_OR_PIERCING_FEATURE_OUT_OF_SCOPE");

        let mut candidate = baseline_candidate();
        candidate["energy_emission"] = Value::Bool(true);
        check_blocked(candidate, "ENERGY_EMISSION_OUT_OF_SCOPE");
    }

    #[test]
    fn embedded_corpus_has_expected_identity_and_digest_shape() {
        assert_eq!(sha256_hex(EMBEDDED_CORPUS).len(), 64);
        let parsed: Value = serde_json::from_slice(EMBEDDED_CORPUS).expect("corpus parses");
        assert_eq!(str_at(&parsed, "suite_id"), Some("HMR0_TOOL_QUALIFICATION"));
        assert_eq!(
            parsed.get("cases").and_then(Value::as_array).map(Vec::len),
            Some(15)
        );
    }
}
