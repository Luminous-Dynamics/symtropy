// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compare receipt-bound normalized observables against a pinned independent reference.
//!
//! This is numerical verification for one named case, not physical validation.
//! The comparator never authenticates execution and never promotes a result to
//! engineering-use qualification.

use crate::{sha256_hex, verify_solver_output_artifact_bytes};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

pub type CompareResult<T> = Result<T, String>;
const REFERENCE_SCHEMA: &str = "physical-reference-observables-v0.1";
const OBSERVATION_SCHEMA: &str = "physical-normalized-observables-v0.1";
const MAX_NORMALIZED_BYTES: u64 = 16 * 1024 * 1024;

/// Verify receipt/output byte identity, then compare one normalized-observable output
/// with a request-bound reference manifest. The reference must be frozen by the caller
/// before the candidate run; the comparator binds its bytes but cannot prove chronology.
pub fn compare_reference_observables(
    request_bytes: &[u8],
    receipt_bytes: &[u8],
    artifact_root: &Path,
    observation_output_id: &str,
    reference_bytes: &[u8],
) -> CompareResult<Value> {
    let artifact_report =
        verify_solver_output_artifact_bytes(request_bytes, receipt_bytes, artifact_root)?;
    let reference_sha256 = sha256_hex(reference_bytes);
    let request_sha256 = sha256_hex(request_bytes);

    let reference: Value = serde_json::from_slice(reference_bytes)
        .map_err(|error| format!("invalid reference manifest JSON: {error}"))?;
    let reference_parsed = parse_reference(&reference, &request_sha256)?;

    let receipt: Value = serde_json::from_slice(receipt_bytes)
        .map_err(|error| format!("invalid receipt JSON: {error}"))?;
    let producer_status = get_str(&receipt, "status", "receipt")?.to_owned();

    if artifact_report["artifact_verification_status"] != "PASS" {
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            "output artifact byte verification did not PASS",
        ));
    }
    if producer_status != "PASS" {
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            "producer status is not PASS; numerical comparison is not scoreable",
        ));
    }

    let outputs = get_array(&receipt, "outputs", "receipt")?;
    let selected = outputs
        .iter()
        .find(|output| {
            output.get("output_id").and_then(Value::as_str) == Some(observation_output_id)
        })
        .ok_or_else(|| {
            format!(
                "observation output ID '{observation_output_id}' is not present in the receipt"
            )
        })?;
    let artifact_ref = get_str(selected, "artifact_ref", "receipt output")?;
    let expected_artifact_sha = get_str(selected, "sha256", "receipt output")?;
    let report_entry = artifact_report["artifact_results"]
        .as_array()
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry.get("output_id").and_then(Value::as_str) == Some(observation_output_id)
            })
        })
        .ok_or_else(|| "selected output has no byte-verification report entry".to_owned())?;
    let first_observed_sha = get_str(report_entry, "observed_sha256", "artifact report")?;

    let root = fs::canonicalize(artifact_root)
        .map_err(|error| format!("could not canonicalize artifact root: {error}"))?;
    let relative = safe_relative_path(artifact_ref)?;
    let candidate = root.join(relative);
    let canonical_file = fs::canonicalize(&candidate)
        .map_err(|error| format!("could not resolve normalized observation artifact: {error}"))?;
    if !canonical_file.starts_with(&root) {
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            "selected observation artifact resolved outside the artifact root",
        ));
    }
    let metadata = fs::metadata(&canonical_file)
        .map_err(|error| format!("could not inspect normalized observation artifact: {error}"))?;
    if !metadata.is_file() {
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            "selected observation artifact is not a regular file",
        ));
    }
    if metadata.len() > MAX_NORMALIZED_BYTES {
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            "normalized-observable JSON exceeds the 16 MiB parser budget",
        ));
    }
    let observation_bytes = fs::read(&canonical_file)
        .map_err(|error| format!("could not read normalized observation artifact: {error}"))?;
    let observed_artifact_sha256 = sha256_hex(&observation_bytes);
    if observed_artifact_sha256 != expected_artifact_sha
        || observed_artifact_sha256 != first_observed_sha
    {
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            "observation artifact changed after byte verification; comparison aborted",
        ));
    }

    let observation: Value = serde_json::from_slice(&observation_bytes)
        .map_err(|error| format!("invalid normalized-observables JSON: {error}"))?;
    let observed = parse_observation(&observation, &request_sha256, &reference_parsed.case_id)?;
    let reference_ids: BTreeSet<&str> = reference_parsed
        .observables
        .keys()
        .map(String::as_str)
        .collect();
    let observed_ids: BTreeSet<&str> = observed.keys().map(String::as_str).collect();
    if reference_ids != observed_ids {
        let missing: Vec<_> = reference_ids.difference(&observed_ids).copied().collect();
        let extra: Vec<_> = observed_ids.difference(&reference_ids).copied().collect();
        return Ok(unresolved_report(
            &artifact_report,
            &reference_parsed,
            &request_sha256,
            &reference_sha256,
            observation_output_id,
            &format!("observable inventory mismatch; missing={missing:?}, extra={extra:?}"),
        ));
    }

    let mut results = Vec::<Value>::new();
    let mut diagnostics = Vec::<String>::new();
    let mut any_outside_tolerance = false;
    for (id, expected) in &reference_parsed.observables {
        let actual = observed
            .get(id)
            .expect("equal observable inventories were checked");
        if actual.unit != expected.unit {
            return Ok(unresolved_report(
                &artifact_report,
                &reference_parsed,
                &request_sha256,
                &reference_sha256,
                observation_output_id,
                &format!(
                    "observable '{id}' unit mismatch: expected '{}', observed '{}'",
                    expected.unit, actual.unit
                ),
            ));
        }
        let absolute_error = (actual.value - expected.expected_value).abs();
        let allowed_error =
            expected.absolute_tolerance + expected.relative_tolerance * expected.expected_value.abs();
        if !absolute_error.is_finite() || !allowed_error.is_finite() {
            return Ok(unresolved_report(
                &artifact_report,
                &reference_parsed,
                &request_sha256,
                &reference_sha256,
                observation_output_id,
                &format!("observable '{id}' error calculation overflowed"),
            ));
        }
        let within_tolerance = absolute_error <= allowed_error;
        if !within_tolerance {
            any_outside_tolerance = true;
            diagnostics.push(format!(
                "observable '{id}' exceeds its preregistered tolerance"
            ));
        }
        let (relative_error, relative_error_status) = if expected.expected_value == 0.0 {
            (Value::Null, "undefined_zero_reference")
        } else {
            let ratio = absolute_error / expected.expected_value.abs();
            if ratio.is_finite() {
                (json!(ratio), "computed")
            } else {
                (Value::Null, "overflow")
            }
        };
        results.push(json!({
            "observable_id": id,
            "unit": expected.unit,
            "reference_value": expected.expected_value,
            "observed_value": actual.value,
            "absolute_error": absolute_error,
            "relative_error": relative_error,
            "relative_error_defined": expected.expected_value != 0.0,
            "relative_error_status": relative_error_status,
            "absolute_tolerance": expected.absolute_tolerance,
            "relative_tolerance": expected.relative_tolerance,
            "allowed_absolute_error": allowed_error,
            "within_tolerance": within_tolerance
        }));
    }

    let status = if any_outside_tolerance {
        "FAIL"
    } else {
        "PASS"
    };
    Ok(json!({
        "schema_version": "physical-numerical-comparison-report-v0.1",
        "comparison_status": status,
        "comparison_method": "per_observable_absolute_plus_relative_tolerance",
        "comparison_protocol_id": reference_parsed.protocol_id,
        "reference_id": reference_parsed.reference_id,
        "reference_sha256": reference_sha256,
        "oracle_method": reference_parsed.oracle_method,
        "oracle_identity": reference_parsed.oracle_identity,
        "oracle_source_ref": reference_parsed.oracle_source_ref,
        "request_sha256": request_sha256,
        "case_id": reference_parsed.case_id,
        "observation_output_id": observation_output_id,
        "observation_artifact_sha256": observed_artifact_sha256,
        "artifact_verification_status": "PASS",
        "producer_status": producer_status,
        "numerical_observable_comparison": status,
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "diagnostics": diagnostics,
        "observable_results": results,
        "claim_ceiling": "named normalized observables compared with the exact reference-manifest bytes and predeclared tolerances; no execution authentication, measured validation, model credibility or engineering-use qualification"
    }))
}

struct ReferenceObservable {
    expected_value: f64,
    unit: String,
    absolute_tolerance: f64,
    relative_tolerance: f64,
}

struct ReferenceManifest {
    reference_id: String,
    case_id: String,
    protocol_id: String,
    oracle_method: String,
    oracle_identity: String,
    oracle_source_ref: String,
    observables: BTreeMap<String, ReferenceObservable>,
}

struct ObservedObservable {
    value: f64,
    unit: String,
}

fn parse_reference(value: &Value, request_sha256: &str) -> CompareResult<ReferenceManifest> {
    exact_keys(
        value,
        &[
            "schema_version",
            "reference_id",
            "case_id",
            "request_sha256",
            "comparison_protocol_id",
            "oracle",
            "observables",
        ],
        "reference manifest",
    )?;
    expect_str(
        value,
        "schema_version",
        REFERENCE_SCHEMA,
        "reference manifest",
    )?;
    let bound_request_sha = get_str(value, "request_sha256", "reference manifest")?;
    if bound_request_sha != request_sha256 {
        return Err("reference manifest is not bound to the exact request bytes".to_owned());
    }
    let reference_id = token(
        get_str(value, "reference_id", "reference manifest")?,
        "reference ID",
    )?;
    let case_id = token(get_str(value, "case_id", "reference manifest")?, "case ID")?;
    let protocol_id = token(
        get_str(value, "comparison_protocol_id", "reference manifest")?,
        "comparison protocol ID",
    )?;
    let oracle = get_object(value, "oracle", "reference manifest")?;
    exact_keys(
        oracle,
        &["method", "identity", "source_ref"],
        "reference.oracle",
    )?;
    let oracle_method = get_str(oracle, "method", "reference.oracle")?;
    if !["analytical_exact", "independent_numerical"].contains(&oracle_method) {
        return Err(
            "reference.oracle.method must be analytical_exact or independent_numerical".to_owned(),
        );
    }
    let oracle_identity = non_empty(
        get_str(oracle, "identity", "reference.oracle")?,
        "reference.oracle.identity",
        512,
    )?;
    let oracle_source_ref = non_empty(
        get_str(oracle, "source_ref", "reference.oracle")?,
        "reference.oracle.source_ref",
        2048,
    )?;

    let array = get_array(value, "observables", "reference manifest")?;
    if array.is_empty() {
        return Err("reference manifest must declare at least one observable".to_owned());
    }
    let mut observables = BTreeMap::new();
    for (index, item) in array.iter().enumerate() {
        let at = format!("reference.observables[{index}]");
        exact_keys(
            item,
            &[
                "observable_id",
                "expected_value",
                "unit",
                "absolute_tolerance",
                "relative_tolerance",
            ],
            &at,
        )?;
        let id = token(get_str(item, "observable_id", &at)?, "observable ID")?;
        let expected_value = finite_number(item, "expected_value", &at)?;
        let unit = non_empty(get_str(item, "unit", &at)?, &format!("{at}.unit"), 64)?;
        let absolute_tolerance = non_negative(item, "absolute_tolerance", &at)?;
        let relative_tolerance = non_negative(item, "relative_tolerance", &at)?;
        let max_allowed = absolute_tolerance + relative_tolerance * expected_value.abs();
        if !max_allowed.is_finite() {
            return Err(format!("{at} combined tolerance overflows finite range"));
        }
        let observable = ReferenceObservable {
            expected_value,
            unit,
            absolute_tolerance,
            relative_tolerance,
        };
        if observables.insert(id.clone(), observable).is_some() {
            return Err(format!("{at} duplicates observable ID '{id}'"));
        }
    }

    Ok(ReferenceManifest {
        reference_id,
        case_id,
        protocol_id,
        oracle_method: oracle_method.to_owned(),
        oracle_identity,
        oracle_source_ref,
        observables,
    })
}

fn parse_observation(
    value: &Value,
    request_sha256: &str,
    expected_case_id: &str,
) -> CompareResult<BTreeMap<String, ObservedObservable>> {
    exact_keys(
        value,
        &["schema_version", "request_sha256", "case_id", "observables"],
        "normalized observation",
    )?;
    expect_str(
        value,
        "schema_version",
        OBSERVATION_SCHEMA,
        "normalized observation",
    )?;
    if get_str(value, "request_sha256", "normalized observation")? != request_sha256 {
        return Err("normalized observation is not bound to the exact request bytes".to_owned());
    }
    if get_str(value, "case_id", "normalized observation")? != expected_case_id {
        return Err("normalized observation case ID differs from reference case ID".to_owned());
    }
    let array = get_array(value, "observables", "normalized observation")?;
    let mut observables = BTreeMap::new();
    for (index, item) in array.iter().enumerate() {
        let at = format!("observation.observables[{index}]");
        exact_keys(item, &["observable_id", "value", "unit"], &at)?;
        let id = token(
            get_str(item, "observable_id", &at)?,
            "observed observable ID",
        )?;
        let value = finite_number(item, "value", &at)?;
        let unit = non_empty(get_str(item, "unit", &at)?, &format!("{at}.unit"), 64)?;
        if observables
            .insert(id.clone(), ObservedObservable { value, unit })
            .is_some()
        {
            return Err(format!("{at} duplicates observable ID '{id}'"));
        }
    }
    Ok(observables)
}

fn unresolved_report(
    artifact_report: &Value,
    reference: &ReferenceManifest,
    request_sha256: &str,
    reference_sha256: &str,
    observation_output_id: &str,
    diagnostic: &str,
) -> Value {
    json!({
        "schema_version": "physical-numerical-comparison-report-v0.1",
        "comparison_status": "UNRESOLVED",
        "comparison_method": "per_observable_absolute_plus_relative_tolerance",
        "comparison_protocol_id": reference.protocol_id,
        "reference_id": reference.reference_id,
        "reference_sha256": reference_sha256,
        "oracle_method": reference.oracle_method,
        "oracle_identity": reference.oracle_identity,
        "oracle_source_ref": reference.oracle_source_ref,
        "request_sha256": request_sha256,
        "case_id": reference.case_id,
        "observation_output_id": observation_output_id,
        "artifact_verification_status": artifact_report["artifact_verification_status"],
        "producer_status": artifact_report["producer_status"],
        "numerical_observable_comparison": "UNRESOLVED",
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "diagnostics": [diagnostic],
        "observable_results": [],
        "claim_ceiling": "numerical comparison not scoreable; missing, inconsistent or unqualified prerequisite evidence remains unresolved"
    })
}

fn safe_relative_path(reference: &str) -> CompareResult<PathBuf> {
    if reference.is_empty()
        || reference.trim() != reference
        || reference
            .chars()
            .any(|character| matches!(character, '\\' | ':' | '\0'))
        || reference
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err("artifact_ref must be a normalized relative POSIX path".to_owned());
    }
    let path = Path::new(reference);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("artifact_ref must stay below the artifact root".to_owned());
    }
    Ok(path.to_path_buf())
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> CompareResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn get_object<'a>(value: &'a Value, key: &str, at: &str) -> CompareResult<&'a Map<String, Value>> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{at}.{key} must be an object"))
}

fn get_array<'a>(value: &'a Value, key: &str, at: &str) -> CompareResult<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))
}

fn get_str<'a>(value: &'a Value, key: &str, at: &str) -> CompareResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn expect_str(value: &Value, key: &str, expected: &str, at: &str) -> CompareResult<()> {
    let actual = get_str(value, key, at)?;
    if actual != expected {
        return Err(format!("{at}.{key} must equal '{expected}'"));
    }
    Ok(())
}

fn finite_number(value: &Value, key: &str, at: &str) -> CompareResult<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{at}.{key} must be a finite number"))
}

fn non_negative(value: &Value, key: &str, at: &str) -> CompareResult<f64> {
    let number = finite_number(value, key, at)?;
    if number < 0.0 {
        return Err(format!("{at}.{key} must be non-negative"));
    }
    Ok(number)
}

fn non_empty(value: &str, at: &str, max_bytes: usize) -> CompareResult<String> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(format!(
            "{at} must be non-empty and no longer than {max_bytes} bytes"
        ));
    }
    Ok(value.to_owned())
}

fn token(value: &str, label: &str) -> CompareResult<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':'))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "physical-reference-compare-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("temporary root");
        root
    }

    fn request_bytes() -> Vec<u8> {
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json",
        ))
        .expect("existing solver request fixture is valid")
    }
    fn build_receipt(request: &[u8], output_sha: &str, status: &str) -> Value {
        let value: Value = serde_json::from_slice(request).unwrap();
        json!({
            "schema_version":"physical-solver-receipt-v0.1",
            "request_id":value["request_id"],
            "request_sha256":sha256_hex(request),
            "source_revision":value["source_revision"],
            "adapter":{"adapter_id":"test.fem","version":"0.1","executable_sha256":"c".repeat(64),"configuration_sha256":"d".repeat(64),"solver_name":"test-fem","solver_version":"0.1","solver_build_ref":"fixture-only"},
            "status":status,
            "evidence_class":"synthetic",
            "physics_evaluated":["static_linear_elastic"],
            "physics_omitted":[],
            "convergence":{"state":"converged","iterations":1,"residual":0.0,"mesh_refinement_evaluated":true,"details":"synthetic"},
            "outputs":[
                {"output_id":"normalized","kind":"displacement","sha256":output_sha,"media_type":"application/json","unit":"m","artifact_ref":"observed.json"},
                {"output_id":"aux-stress","kind":"stress","sha256":sha256_hex(br#"{"synthetic_auxiliary":true}"#),"media_type":"application/json","unit":"Pa","artifact_ref":"stress.json"}
            ],
            "uncertainty":{"state":"unknown","method":null,"summary":"test only"},
            "validity_envelope":{"supported_geometry":"test","supported_material_model":"test","load_regime":"test","limitations":["synthetic"]},
            "claim_ceiling":"synthetic test only; not qualified",
            "diagnostics": if status == "PASS" { json!([]) } else { json!(["producer did not pass"]) }
        })
    }

    fn reference(request: &[u8], expected: f64, abs_tol: f64, rel_tol: f64) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema_version":REFERENCE_SCHEMA,
            "reference_id":"fixture-known-answer-v1",
            "case_id":"fixture-bar-reference",
            "request_sha256":sha256_hex(request),
            "comparison_protocol_id":"fixture-tolerance-v1",
            "oracle":{"method":"analytical_exact","identity":"frozen synthetic known-answer oracle","source_ref":"test-oracle://normalized-observable-v1"},
            "observables":[{"observable_id":"fixture_displacement_m","expected_value":expected,"unit":"m","absolute_tolerance":abs_tol,"relative_tolerance":rel_tol}]
        })).unwrap()
    }

    fn observation(request: &[u8], value: f64) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema_version":OBSERVATION_SCHEMA,
            "request_sha256":sha256_hex(request),
            "case_id":"fixture-bar-reference",
            "observables":[{"observable_id":"fixture_displacement_m","value":value,"unit":"m"}]
        })).unwrap()
    }

    fn setup(request: &[u8], obs: &[u8]) -> (PathBuf, Vec<u8>, Vec<u8>) {
        let root = temporary_root();
        fs::write(root.join("observed.json"), obs).unwrap();
        fs::write(root.join("stress.json"), br#"{"synthetic_auxiliary":true}"#).unwrap();
        let output_sha = sha256_hex(obs);
        let receipt = serde_json::to_vec(&build_receipt(request, &output_sha, "PASS")).unwrap();
        (root, receipt, output_sha.into_bytes())
    }

    #[test]
    fn matching_reference_passes_without_promoting_physical_qualification() {
        let request = request_bytes();
        let observed = observation(&request, 0.125);
        let (root, receipt, _) = setup(&request, &observed);
        let reference = reference(&request, 0.125, 1e-12, 1e-10);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "PASS");
        assert_eq!(report["numerical_observable_comparison"], "PASS");
        assert_eq!(report["artifact_verification_status"], "PASS");
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn error_outside_combined_tolerance_is_a_numerical_fail() {
        let request = request_bytes();
        let observed = observation(&request, 0.13);
        let (root, receipt, _) = setup(&request, &observed);
        let reference = reference(&request, 0.125, 1e-6, 1e-6);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "FAIL");
        assert_eq!(report["observable_results"][0]["within_tolerance"], false);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn zero_reference_uses_absolute_tolerance_and_has_no_relative_error() {
        let request = request_bytes();
        let observed = observation(&request, 4e-10);
        let (root, receipt, _) = setup(&request, &observed);
        let reference = reference(&request, 0.0, 5e-10, 0.5);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "PASS");
        assert_eq!(report["observable_results"][0]["relative_error_defined"], false);
        assert_eq!(report["observable_results"][0]["relative_error"], Value::Null);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unrepresentable_near_zero_relative_error_is_explicit_not_infinite() {
        let request = request_bytes();
        let observed = observation(&request, 1.0);
        let (root, receipt, _) = setup(&request, &observed);
        let reference = reference(&request, 1e-320, 2.0, 0.0);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "PASS");
        assert_eq!(report["observable_results"][0]["relative_error_defined"], true);
        assert_eq!(report["observable_results"][0]["relative_error_status"], "overflow");
        assert_eq!(report["observable_results"][0]["relative_error"], Value::Null);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn zero_reference_outside_absolute_tolerance_fails_even_if_relative_tolerance_is_large() {
        let request = request_bytes();
        let observed = observation(&request, 4e-4);
        let (root, receipt, _) = setup(&request, &observed);
        let reference = reference(&request, 0.0, 5e-10, 100.0);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "FAIL");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unit_mismatch_is_unresolved_not_silently_converted() {
        let request = request_bytes();
        let mut observed: Value = serde_json::from_slice(&observation(&request, 0.125)).unwrap();
        observed["observables"][0]["unit"] = json!("mm");
        let observed = serde_json::to_vec(&observed).unwrap();
        let (root, receipt, _) = setup(&request, &observed);
        // Bind receipt to the exact modified artifact bytes.
        let mut receipt_value: Value = serde_json::from_slice(&receipt).unwrap();
        receipt_value["outputs"][0]["sha256"] = json!(sha256_hex(&observed));
        let receipt = serde_json::to_vec(&receipt_value).unwrap();
        let reference = reference(&request, 0.125, 1e-12, 1e-10);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "UNRESOLVED");
        assert!(report["diagnostics"][0].as_str().unwrap().contains("unit mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reference_and_observation_must_bind_exact_request_bytes() {
        let request = request_bytes();
        let observed = observation(&request, 0.125);
        let (root, receipt, _) = setup(&request, &observed);
        let mut reference_value: Value = serde_json::from_slice(&reference(&request, 0.125, 1e-12, 1e-10)).unwrap();
        reference_value["request_sha256"] = json!("a".repeat(64));
        let reference_bytes = serde_json::to_vec(&reference_value).unwrap();
        assert!(compare_reference_observables(&request, &receipt, &root, "normalized", &reference_bytes).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn non_pass_producer_status_never_gets_a_numeric_pass() {
        let request = request_bytes();
        let observed = observation(&request, 0.125);
        let (root, _, _) = setup(&request, &observed);
        let output_sha = sha256_hex(&observed);
        let receipt = serde_json::to_vec(&build_receipt(&request, &output_sha, "UNRESOLVED")).unwrap();
        let reference = reference(&request, 0.125, 1e-12, 1e-10);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "UNRESOLVED");
        assert_eq!(report["producer_status"], "UNRESOLVED");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_or_extra_observable_does_not_produce_a_comparison_pass() {
        let request = request_bytes();
        let mut observed: Value = serde_json::from_slice(&observation(&request, 0.125)).unwrap();
        observed["observables"][0]["observable_id"] = json!("different-metric");
        let observed = serde_json::to_vec(&observed).unwrap();
        let (root, receipt, _) = setup(&request, &observed);
        let mut receipt_value: Value = serde_json::from_slice(&receipt).unwrap();
        receipt_value["outputs"][0]["sha256"] = json!(sha256_hex(&observed));
        let receipt = serde_json::to_vec(&receipt_value).unwrap();
        let reference = reference(&request, 0.125, 1e-12, 1e-10);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "UNRESOLVED");
        assert!(report["diagnostics"][0].as_str().unwrap().contains("inventory mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_artifact_digest_blocks_numeric_comparison() {
        let request = request_bytes();
        let observed = observation(&request, 0.125);
        let root = temporary_root();
        fs::write(root.join("observed.json"), &observed).unwrap();
        fs::write(root.join("stress.json"), br#"{"synthetic_auxiliary":true}"#).unwrap();
        let receipt = serde_json::to_vec(&build_receipt(&request, &"f".repeat(64), "PASS")).unwrap();
        let reference = reference(&request, 0.125, 1e-12, 1e-10);
        let report = compare_reference_observables(&request, &receipt, &root, "normalized", &reference).unwrap();
        assert_eq!(report["comparison_status"], "UNRESOLVED");
        assert_eq!(report["artifact_verification_status"], "FAIL");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_tolerance_and_duplicate_metric_ids_fail_closed() {
        let request = request_bytes();
        let mut reference_value: Value = serde_json::from_slice(&reference(&request, 0.125, 1e-12, 1e-10)).unwrap();
        reference_value["observables"][0]["relative_tolerance"] = json!(-1.0);
        assert!(parse_reference(&reference_value, &sha256_hex(&request)).is_err());

        let mut reference_value: Value = serde_json::from_slice(&reference(&request, 0.125, 1e-12, 1e-10)).unwrap();
        let second = reference_value["observables"][0].clone();
        reference_value["observables"].as_array_mut().unwrap().push(second);
        assert!(parse_reference(&reference_value, &sha256_hex(&request)).is_err());
    }
}
