// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-gated dispatch receipt around the qualified analytical adapter.
//!
//! This module selects only the backend allowlisted by capabilities.rs. It does
//! not launch external processes; future backend kinds must receive a separately
//! audited execution adapter before they become dispatch-eligible.

use crate::capabilities::{SelectionStatus, resolve_request};
use crate::evaluate_request_bytes;
use serde_json::{Value, json};

pub type Check<T> = Result<T, String>;

const RECEIPT_SCHEMA: &str = "physical-solver-dispatch-receipt-v0.1";

pub fn dispatch_request_bytes(
    bytes: &[u8],
    dispatcher_revision: &str,
    adapter_revision: &str,
) -> Check<Value> {
    validate_revision(dispatcher_revision, "dispatcher revision")?;
    let request: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("dispatch request JSON is invalid: {error}"))?;
    let adapter_receipt = evaluate_request_bytes(bytes, adapter_revision)?;

    let request_id = required_string(&adapter_receipt, "request_id", "adapter receipt")?;
    let case_id = required_string(&adapter_receipt, "case_id", "adapter receipt")?;
    let assumption_profile =
        required_string(&adapter_receipt, "assumption_profile", "adapter receipt")?;
    let requested_physics = required_string_array(
        &adapter_receipt,
        "requested_physics",
        "adapter receipt",
    )?;
    let units = request
        .get("units")
        .and_then(Value::as_object)
        .ok_or_else(|| "request.units must be an object".to_owned())?;

    let resolution = resolve_request(
        &case_id,
        &assumption_profile,
        &requested_physics,
        units,
    )?;
    let adapter_status =
        required_string(&adapter_receipt, "result_status", "adapter receipt")?;
    let selected = resolution.status == SelectionStatus::Selected;
    let evaluation_status = if !selected {
        "NOT_EVALUATED"
    } else if adapter_status == "PASS" {
        "PASS"
    } else {
        "UNRESOLVED"
    };
    let outputs = if evaluation_status == "PASS" {
        adapter_receipt.get("outputs").cloned().unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let selected_backend = resolution
        .selected_backend
        .as_ref()
        .map(backend_summary)
        .unwrap_or(Value::Null);

    let mut diagnostics = resolution.diagnostics.clone();
    if adapter_status != "PASS" {
        diagnostics.extend(
            adapter_receipt
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }

    Ok(json!({
        "schema_version": RECEIPT_SCHEMA,
        "request_id": request_id,
        "request_sha256": adapter_receipt["request_sha256"],
        "source_revision": adapter_receipt["source_revision"],
        "dispatcher_revision": dispatcher_revision,
        "adapter_revision": adapter_receipt["adapter_revision"],
        "manifest_sha256": resolution.manifest_sha256,
        "case_id": case_id,
        "requested_physics": requested_physics,
        "physics_omitted": resolution.physics_omitted,
        "selection_status": if selected { "SELECTED" } else { "UNRESOLVED" },
        "evaluation_status": evaluation_status,
        "selected_backend": selected_backend,
        "model_evaluation_attempted": selected,
        "external_solver_invoked": false,
        "outputs": outputs,
        "adapter_receipt": adapter_receipt,
        "diagnostics": diagnostics,
        "claim_ceiling": "capability dispatch plus the nested named closed-form analytical result only; no external solver execution, measured physical validation, or structural qualification"
    }))
}

fn backend_summary(backend: &Value) -> Value {
    json!({
        "backend_id": backend["backend_id"],
        "backend_version": backend["backend_version"],
        "implementation_kind": backend["implementation_kind"],
        "qualification_tier": backend["qualification_tier"],
        "real_world_validation_status": backend["real_world_validation_status"],
        "external_solver_invoked": backend["external_solver_invoked"],
        "claim_ceiling": backend["claim_ceiling"]
    })
}

fn required_string(value: &Value, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn required_string_array(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, element)| {
            element
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn validate_revision(revision: &str, label: &str) -> Check<()> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!("{label} must be a 40-character lowercase hexadecimal Git SHA-1"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{fs, path::PathBuf};

    const REV: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/physical-transformation/analytical-fixtures")
                .join(name),
        )
        .expect("analytical fixture exists")
    }

    #[test]
    fn all_supported_case_families_resolve_and_preserve_formula_outputs() {
        for (name, output_key) in [
            ("analytical-bar-v0.1.json", "axial_stress_pa"),
            ("cantilever-tip-load-v0.1.json", "tip_deflection_m"),
            (
                "simply-supported-uniform-load-v0.1.json",
                "midspan_deflection_m",
            ),
        ] {
            let receipt =
                dispatch_request_bytes(&fixture(name), REV, REV).expect("supported case dispatch");
            assert_eq!(receipt["selection_status"], "SELECTED");
            assert_eq!(receipt["evaluation_status"], "PASS");
            assert_eq!(
                receipt["selected_backend"]["backend_id"],
                "symtropy-analytical-reference"
            );
            assert!(receipt["outputs"][output_key].is_number());
            assert_eq!(receipt["external_solver_invoked"], false);
            assert_eq!(receipt["model_evaluation_attempted"], true);
            assert_eq!(receipt["manifest_sha256"].as_str().unwrap().len(), 64);
        }
    }

    #[test]
    fn unsupported_fracture_yields_no_backend_and_no_partial_output() {
        let receipt = dispatch_request_bytes(&fixture("unsupported-fracture-v0.1.json"), REV, REV)
            .expect("unsupported physics is a dispatch abstention");
        assert_eq!(receipt["selection_status"], "UNRESOLVED");
        assert_eq!(receipt["evaluation_status"], "NOT_EVALUATED");
        assert_eq!(receipt["selected_backend"], Value::Null);
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["model_evaluation_attempted"], false);
        assert_eq!(receipt["external_solver_invoked"], false);
        assert!(
            receipt["physics_omitted"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "fracture")
        );
    }

    #[test]
    fn selected_backend_with_domain_failure_does_not_become_pass() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["inputs"]["force_n"] = json!(1.0e308);
        request["inputs"]["length_m"] = json!(1.0e308);
        let bytes = serde_json::to_vec(&request).expect("serialize finite request");
        let receipt = dispatch_request_bytes(&bytes, REV, REV)
            .expect("domain failure is represented in the nested receipt");
        assert_eq!(receipt["selection_status"], "SELECTED");
        assert_eq!(receipt["evaluation_status"], "UNRESOLVED");
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["adapter_receipt"]["result_status"], "UNRESOLVED");
        assert!(receipt["diagnostics"].as_array().unwrap().iter().any(|v| {
            v.as_str().unwrap().contains("could not be completed")
        }));
    }

    #[test]
    fn dispatch_receipt_is_deterministic_for_fixed_inputs_and_revisions() {
        let bytes = fixture("analytical-bar-v0.1.json");
        let first = dispatch_request_bytes(&bytes, REV, REV).expect("first dispatch");
        let second = dispatch_request_bytes(&bytes, REV, REV).expect("repeat dispatch");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
        assert_eq!(first["request_sha256"], second["request_sha256"]);
        assert_eq!(first["manifest_sha256"], second["manifest_sha256"]);
    }

    #[test]
    fn invalid_dispatcher_revision_fails_closed() {
        assert!(
            dispatch_request_bytes(&fixture("analytical-bar-v0.1.json"), "not-a-sha", REV)
                .unwrap_err()
                .contains("dispatcher revision")
        );
    }
}
