// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compare a caller-reported external solver run against a pinned benchmark.
//!
//! This module compares normalized numerical outputs and provenance fields. It
//! cannot authenticate a solver execution from JSON alone and never upgrades a
//! passing numerical comparison into physical model validation.

use super::{Check, sha256_hex};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

const MANIFEST_SCHEMA: &str = "solver-benchmark-manifest-v0.1";
const RUN_SCHEMA: &str = "solver-run-receipt-v0.1";
const COMPARISON_SCHEMA: &str = "solver-comparison-receipt-v0.1";
const REACTION_RESIDUAL: &str = "reaction_balance_residual_relative";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Pass,
    Fail,
    Unresolved,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

#[derive(Clone, Debug)]
struct ReferenceOutput {
    quantity: String,
    value: f64,
    unit: String,
    semantics: String,
    relative_tolerance: f64,
    absolute_tolerance: f64,
}

#[derive(Clone, Debug)]
struct CandidateIdentity {
    name: String,
    version_pin: Option<String>,
    binary_sha256: Option<String>,
    configuration_sha256: Option<String>,
}

#[derive(Clone, Debug)]
struct ParsedRun {
    level: u64,
    element_count: u64,
    exit_code: i64,
    convergence_status: String,
    outputs: BTreeMap<String, f64>,
    output_units: BTreeMap<String, String>,
    output_semantics: BTreeMap<String, String>,
    result_sha256: String,
}

fn object<'a>(value: &'a Value, label: &str) -> Check<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

fn array<'a>(value: &'a Value, label: &str) -> Check<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))
}

fn string_at(value: &Value, key: &str, label: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{label}.{key} must be a string"))
}

fn optional_string_at(value: &Value, key: &str, label: &str) -> Check<Option<String>> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("{label}.{key} must be a string or null")),
    }
}

fn number_at(value: &Value, key: &str, label: &str) -> Check<f64> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{label}.{key} must be a number"))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(format!("{label}.{key} must be finite"))
    }
}

fn u64_at(value: &Value, key: &str, label: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label}.{key} must be a positive integer"))
        .and_then(|number| {
            if number > 0 {
                Ok(number)
            } else {
                Err(format!("{label}.{key} must be a positive integer"))
            }
        })
}

fn i64_at(value: &Value, key: &str, label: &str) -> Check<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.{key} must be an integer"))
}

fn exact_keys(value: &Value, expected: &[&str], label: &str) -> Check<()> {
    let actual = object(value, label)?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if missing.is_empty() && unknown.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{label}: missing keys {missing:?}; unknown keys {unknown:?}"
        ))
    }
}

fn map_of_numbers(value: &Value, label: &str) -> Check<BTreeMap<String, f64>> {
    let map = object(value, label)?;
    map.iter()
        .map(|(key, value)| {
            let number = value
                .as_f64()
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("{label}.{key} must be finite numeric output"))?;
            Ok((key.clone(), number))
        })
        .collect()
}

fn map_of_strings(value: &Value, label: &str) -> Check<BTreeMap<String, String>> {
    let map = object(value, label)?;
    map.iter()
        .map(|(key, value)| {
            let text = value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| format!("{label}.{key} must be a non-empty string"))?;
            Ok((key.clone(), text.to_owned()))
        })
        .collect()
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_manifest(manifest: &Value) -> Check<(String, String, Option<String>, CandidateIdentity, Vec<ReferenceOutput>, Vec<u64>, f64)> {
    exact_keys(
        manifest,
        &[
            "schema_version",
            "benchmark_id",
            "execution_state",
            "solver_candidate",
            "verification_level",
            "case",
            "reference",
            "input_fixture",
            "mesh_plan",
            "acceptance",
            "claim_ceiling",
        ],
        "manifest",
    )?;
    if string_at(manifest, "schema_version", "manifest")? != MANIFEST_SCHEMA {
        return Err(format!("manifest.schema_version must be '{MANIFEST_SCHEMA}'"));
    }
    let benchmark_id = string_at(manifest, "benchmark_id", "manifest")?;
    let case = manifest
        .get("case")
        .ok_or_else(|| "manifest.case is missing".to_owned())?;
    let case_id = string_at(case, "case_id", "manifest.case")?;
    let candidate = manifest
        .get("solver_candidate")
        .ok_or_else(|| "manifest.solver_candidate is missing".to_owned())?;
    exact_keys(
        candidate,
        &["name", "role", "version_pin", "binary_sha256", "configuration_sha256"],
        "manifest.solver_candidate",
    )?;
    let candidate_identity = CandidateIdentity {
        name: string_at(candidate, "name", "manifest.solver_candidate")?,
        version_pin: optional_string_at(candidate, "version_pin", "manifest.solver_candidate")?,
        binary_sha256: optional_string_at(candidate, "binary_sha256", "manifest.solver_candidate")?,
        configuration_sha256: optional_string_at(
            candidate,
            "configuration_sha256",
            "manifest.solver_candidate",
        )?,
    };
    if candidate_identity
        .version_pin
        .as_deref()
        .is_some_and(|version| version.trim().is_empty())
    {
        return Err("manifest.solver_candidate.version_pin must not be empty".to_owned());
    }
    for (label, digest) in [
        ("binary_sha256", candidate_identity.binary_sha256.as_deref()),
        (
            "configuration_sha256",
            candidate_identity.configuration_sha256.as_deref(),
        ),
    ] {
        if let Some(digest) = digest
            && !valid_hex(digest, 64)
        {
            return Err(format!("manifest.solver_candidate.{label} is not a lowercase SHA-256"));
        }
    }

    let reference = manifest
        .get("reference")
        .ok_or_else(|| "manifest.reference is missing".to_owned())?;
    let reference_revision = string_at(reference, "source_revision", "manifest.reference")?;
    if !valid_hex(&reference_revision, 40) {
        return Err("manifest.reference.source_revision must be a lowercase Git SHA-1".to_owned());
    }

    let mut outputs = Vec::new();
    for (index, item) in array(
        reference
            .get("outputs")
            .ok_or_else(|| "manifest.reference.outputs is missing".to_owned())?,
        "manifest.reference.outputs",
    )?
    .iter()
    .enumerate()
    {
        let label = format!("manifest.reference.outputs[{index}]");
        let quantity = string_at(item, "quantity", &label)?;
        let unit = string_at(item, "unit", &label)?;
        let semantics = string_at(item, "semantics", &label)?;
        let value = number_at(item, "value", &label)?;
        let relative_tolerance = number_at(item, "relative_tolerance", &label)?;
        let absolute_tolerance = number_at(item, "absolute_tolerance", &label)?;
        if relative_tolerance < 0.0 || absolute_tolerance < 0.0 {
            return Err(format!("{label} tolerances must be non-negative"));
        }
        outputs.push(ReferenceOutput {
            quantity,
            value,
            unit,
            semantics,
            relative_tolerance,
            absolute_tolerance,
        });
    }
    if outputs.is_empty() {
        return Err("manifest.reference.outputs must not be empty".to_owned());
    }

    let fixture = manifest
        .get("input_fixture")
        .ok_or_else(|| "manifest.input_fixture is missing".to_owned())?;
    let fixture_sha256 = optional_string_at(fixture, "sha256", "manifest.input_fixture")?;
    if let Some(digest) = fixture_sha256.as_deref()
        && !valid_hex(digest, 64)
    {
        return Err("manifest.input_fixture.sha256 is not a lowercase SHA-256".to_owned());
    }

    let mesh_plan = manifest
        .get("mesh_plan")
        .ok_or_else(|| "manifest.mesh_plan is missing".to_owned())?;
    let mut levels = Vec::new();
    for level in array(
        mesh_plan
            .get("refinement_levels")
            .ok_or_else(|| "manifest.mesh_plan.refinement_levels is missing".to_owned())?,
        "manifest.mesh_plan.refinement_levels",
    )? {
        let value = level
            .as_u64()
            .filter(|value| *value > 0)
            .ok_or_else(|| "mesh refinement levels must be positive integers".to_owned())?;
        if levels.contains(&value) {
            return Err("manifest mesh refinement levels must be unique".to_owned());
        }
        levels.push(value);
    }
    if levels.is_empty() {
        return Err("manifest must declare at least one mesh refinement level".to_owned());
    }
    levels.sort_unstable();

    let acceptance = manifest
        .get("acceptance")
        .ok_or_else(|| "manifest.acceptance is missing".to_owned())?;
    let reaction_limit = number_at(
        acceptance,
        "maximum_reaction_balance_residual",
        "manifest.acceptance",
    )?;
    if reaction_limit < 0.0 {
        return Err("maximum reaction balance residual must be non-negative".to_owned());
    }
    Ok((
        benchmark_id,
        case_id,
        fixture_sha256,
        candidate_identity,
        outputs,
        levels,
        reaction_limit,
    ))
}

fn parse_run(run_value: &Value) -> Check<(String, String, String, String, CandidateIdentity, Vec<ParsedRun>)> {
    exact_keys(
        run_value,
        &[
            "schema_version",
            "benchmark_id",
            "manifest_sha256",
            "reference_source_revision",
            "input_fixture_sha256",
            "solver",
            "runner",
            "refinement_runs",
        ],
        "run",
    )?;
    if string_at(run_value, "schema_version", "run")? != RUN_SCHEMA {
        return Err(format!("run.schema_version must be '{RUN_SCHEMA}'"));
    }
    let benchmark_id = string_at(run_value, "benchmark_id", "run")?;
    let manifest_sha256 = string_at(run_value, "manifest_sha256", "run")?;
    let reference_revision = string_at(run_value, "reference_source_revision", "run")?;
    let input_fixture_sha256 = string_at(run_value, "input_fixture_sha256", "run")?;
    if !valid_hex(&manifest_sha256, 64)
        || !valid_hex(&input_fixture_sha256, 64)
        || !valid_hex(&reference_revision, 40)
    {
        return Err("run manifest, fixture, or reference digest has invalid syntax".to_owned());
    }

    let solver = run_value
        .get("solver")
        .ok_or_else(|| "run.solver is missing".to_owned())?;
    exact_keys(
        solver,
        &["name", "version_pin", "binary_sha256", "configuration_sha256"],
        "run.solver",
    )?;
    let identity = CandidateIdentity {
        name: string_at(solver, "name", "run.solver")?,
        version_pin: Some(string_at(solver, "version_pin", "run.solver")?),
        binary_sha256: Some(string_at(solver, "binary_sha256", "run.solver")?),
        configuration_sha256: Some(string_at(solver, "configuration_sha256", "run.solver")?),
    };
    if !valid_hex(identity.binary_sha256.as_deref().unwrap_or(""), 64)
        || !valid_hex(identity.configuration_sha256.as_deref().unwrap_or(""), 64)
    {
        return Err("run solver binary/configuration digest is invalid".to_owned());
    }

    let runner = run_value
        .get("runner")
        .ok_or_else(|| "run.runner is missing".to_owned())?;
    exact_keys(
        runner,
        &["workflow_head_sha", "workflow_run_id", "runner_image", "execution_log_sha256"],
        "run.runner",
    )?;
    let workflow_head = string_at(runner, "workflow_head_sha", "run.runner")?;
    let run_id = string_at(runner, "workflow_run_id", "run.runner")?;
    let runner_image = string_at(runner, "runner_image", "run.runner")?;
    let log_digest = string_at(runner, "execution_log_sha256", "run.runner")?;
    if !valid_hex(&workflow_head, 40) || !valid_hex(&log_digest, 64) || run_id.is_empty() || runner_image.is_empty() {
        return Err("run runner identity is incomplete or malformed".to_owned());
    }

    let mut runs = Vec::new();
    for (index, value) in array(
        run_value
            .get("refinement_runs")
            .ok_or_else(|| "run.refinement_runs is missing".to_owned())?,
        "run.refinement_runs",
    )?
    .iter()
    .enumerate()
    {
        let label = format!("run.refinement_runs[{index}]");
        exact_keys(
            value,
            &[
                "refinement_level",
                "element_count",
                "exit_code",
                "convergence_status",
                "outputs",
                "output_units",
                "output_semantics",
                "result_file_sha256",
            ],
            &label,
        )?;
        let level = u64_at(value, "refinement_level", &label)?;
        let element_count = u64_at(value, "element_count", &label)?;
        let exit_code = i64_at(value, "exit_code", &label)?;
        let convergence_status = string_at(value, "convergence_status", &label)?;
        if !["converged", "not_converged", "unknown"].contains(&convergence_status.as_str()) {
            return Err(format!("{label}.convergence_status is unsupported"));
        }
        let outputs = map_of_numbers(
            value
                .get("outputs")
                .ok_or_else(|| format!("{label}.outputs is missing"))?,
            &format!("{label}.outputs"),
        )?;
        let output_units = map_of_strings(
            value
                .get("output_units")
                .ok_or_else(|| format!("{label}.output_units is missing"))?,
            &format!("{label}.output_units"),
        )?;
        let output_semantics = map_of_strings(
            value
                .get("output_semantics")
                .ok_or_else(|| format!("{label}.output_semantics is missing"))?,
            &format!("{label}.output_semantics"),
        )?;
        let result_sha256 = string_at(value, "result_file_sha256", &label)?;
        if !valid_hex(&result_sha256, 64) {
            return Err(format!("{label}.result_file_sha256 is invalid"));
        }
        runs.push(ParsedRun {
            level,
            element_count,
            exit_code,
            convergence_status,
            outputs,
            output_units,
            output_semantics,
            result_sha256,
        });
    }

    Ok((
        benchmark_id,
        manifest_sha256,
        reference_revision,
        input_fixture_sha256,
        identity,
        runs,
    ))
}

/// Compare a run receipt to the exact manifest bytes. The result can establish
/// numerical comparison and field consistency, but execution evidence is still
/// caller-reported and physical validation always remains UNRESOLVED.
pub fn compare_manifest_run_bytes(
    manifest_bytes: &[u8],
    run_bytes: &[u8],
    verifier_revision: &str,
) -> Check<Value> {
    if !valid_hex(verifier_revision, 40) {
        return Err("verifier revision must be a lowercase 40-character Git SHA-1".to_owned());
    }
    let manifest: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("manifest JSON is invalid: {error}"))?;
    let run_value: Value = serde_json::from_slice(run_bytes)
        .map_err(|error| format!("run receipt JSON is invalid: {error}"))?;
    let (benchmark_id, case_id, expected_fixture_sha, expected_solver, reference_outputs, expected_levels, reaction_limit) =
        parse_manifest(&manifest)?;
    let (run_benchmark_id, run_manifest_sha, run_reference_revision, run_fixture_sha, run_solver, mut runs) =
        parse_run(&run_value)?;

    let manifest_sha256 = sha256_hex(manifest_bytes);
    let run_receipt_sha256 = sha256_hex(run_bytes);
    let mut diagnostics = Vec::<String>::new();
    let mut hard_fail = false;

    if run_benchmark_id != benchmark_id {
        diagnostics.push("run benchmark_id differs from manifest".to_owned());
        hard_fail = true;
    }
    if run_manifest_sha != manifest_sha256 {
        diagnostics.push("run manifest_sha256 does not bind to these exact manifest bytes".to_owned());
        hard_fail = true;
    }
    if run_reference_revision
        != string_at(
            manifest
                .get("reference")
                .ok_or_else(|| "manifest.reference is missing".to_owned())?,
            "source_revision",
            "manifest.reference",
        )?
    {
        diagnostics.push("run reference_source_revision differs from the manifest".to_owned());
        hard_fail = true;
    }

    let solver_identity_status = if expected_solver.version_pin.is_none()
        || expected_solver.binary_sha256.is_none()
        || expected_solver.configuration_sha256.is_none()
    {
        diagnostics.push("manifest solver version/binary/configuration are not all pinned".to_owned());
        "UNRESOLVED"
    } else if expected_solver.name != run_solver.name
        || expected_solver.version_pin != run_solver.version_pin
        || expected_solver.binary_sha256 != run_solver.binary_sha256
        || expected_solver.configuration_sha256 != run_solver.configuration_sha256
    {
        diagnostics.push("run solver identity differs from the pinned manifest identity".to_owned());
        hard_fail = true;
        "MISMATCH"
    } else {
        "PINNED_MATCH"
    };

    let input_provenance_status = match expected_fixture_sha.as_deref() {
        None => {
            diagnostics.push("manifest input fixture SHA-256 is not pinned".to_owned());
            "UNRESOLVED"
        }
        Some(expected) if expected != run_fixture_sha => {
            diagnostics.push("run input fixture SHA-256 differs from the pinned manifest".to_owned());
            hard_fail = true;
            "MISMATCH"
        }
        Some(_) if run_manifest_sha != manifest_sha256 || run_reference_revision
            != string_at(manifest.get("reference").ok_or_else(|| "manifest.reference is missing".to_owned())?, "source_revision", "manifest.reference")? =>
        {
            "MISMATCH"
        }
        Some(_) => "PINNED_MATCH",
    };

    runs.sort_by_key(|run| run.level);
    let actual_levels: Vec<u64> = runs.iter().map(|run| run.level).collect();
    if actual_levels != expected_levels {
        diagnostics.push(format!(
            "refinement levels do not match manifest: expected {expected_levels:?}, got {actual_levels:?}"
        ));
        hard_fail = true;
    }
    for pair in runs.windows(2) {
        if pair[1].element_count <= pair[0].element_count {
            diagnostics.push("element counts must increase strictly with refinement level".to_owned());
            hard_fail = true;
            break;
        }
    }

    let mut refinement_results = Vec::new();
    let mut numerical_fail = hard_fail;
    for run in &runs {
        let mut level_fail = false;
        let mut output_comparisons = Vec::new();

        if run.exit_code != 0 {
            diagnostics.push(format!(
                "refinement level {} exited with nonzero status {}",
                run.level, run.exit_code
            ));
            level_fail = true;
        }
        if run.convergence_status != "converged" {
            diagnostics.push(format!(
                "refinement level {} is not confirmed converged",
                run.level
            ));
            level_fail = true;
        }

        let expected_output_keys: BTreeSet<&str> = reference_outputs
            .iter()
            .map(|output| output.quantity.as_str())
            .chain(std::iter::once(REACTION_RESIDUAL))
            .collect();
        let actual_output_keys: BTreeSet<&str> = run.outputs.keys().map(String::as_str).collect();
        let actual_unit_keys: BTreeSet<&str> = run.output_units.keys().map(String::as_str).collect();
        let actual_semantics_keys: BTreeSet<&str> = run.output_semantics.keys().map(String::as_str).collect();
        if expected_output_keys != actual_output_keys
            || expected_output_keys != actual_unit_keys
            || expected_output_keys != actual_semantics_keys
        {
            diagnostics.push(format!(
                "refinement level {} output value/unit/semantics keys do not match the manifest",
                run.level
            ));
            level_fail = true;
        }

        for reference in &reference_outputs {
            let actual = run.outputs.get(&reference.quantity).copied();
            let actual_unit = run.output_units.get(&reference.quantity);
            let actual_semantics = run.output_semantics.get(&reference.quantity);
            let allowed_error =
                reference.absolute_tolerance + reference.relative_tolerance * reference.value.abs();
            let error = actual.map(|value| (value - reference.value).abs());
            let unit_ok = actual_unit.is_some_and(|unit| unit == &reference.unit);
            let semantics_ok = actual_semantics.is_some_and(|semantics| semantics == &reference.semantics);
            let passed = actual.is_some_and(|value| {
                value.is_finite()
                    && error.is_some_and(|error| error.is_finite() && error <= allowed_error)
            }) && unit_ok && semantics_ok;
            if !passed {
                diagnostics.push(format!(
                    "refinement level {} output '{}' failed reference tolerance, unit or semantics check",
                    run.level, reference.quantity
                ));
                level_fail = true;
            }
            output_comparisons.push(json!({
                "quantity": reference.quantity,
                "unit": actual_unit.cloned().unwrap_or_default(),
                "semantics": actual_semantics.cloned().unwrap_or_default(),
                "reference_value": reference.value,
                "actual_value": actual,
                "absolute_error": error,
                "allowed_error": allowed_error,
                "status": if passed { "PASS" } else { "FAIL" }
            }));
        }

        let reaction_residual = run.outputs.get(REACTION_RESIDUAL).copied();
        let residual_pass = reaction_residual
            .is_some_and(|residual| residual.is_finite() && residual >= 0.0 && residual <= reaction_limit);
        if !residual_pass {
            diagnostics.push(format!(
                "refinement level {} reaction-balance residual is missing or exceeds the manifest limit",
                run.level
            ));
            level_fail = true;
        }

        let result_status = if level_fail { "FAIL" } else { "PASS" };
        numerical_fail |= level_fail;
        refinement_results.push(json!({
            "refinement_level": run.level,
            "element_count": run.element_count,
            "result_file_sha256": run.result_sha256,
            "exit_code": run.exit_code,
            "convergence_status": run.convergence_status,
            "status": result_status,
            "output_comparisons": output_comparisons,
            "reaction_balance_residual_relative": reaction_residual
        }));
    }

    let numerical_comparison_status = if runs.is_empty() {
        "UNRESOLVED"
    } else if numerical_fail {
        "FAIL"
    } else {
        "PASS"
    };
    let overall_comparison_status = if numerical_comparison_status == "FAIL" || hard_fail {
        Status::Fail.as_str()
    } else if numerical_comparison_status == "UNRESOLVED"
        || solver_identity_status == "UNRESOLVED"
        || input_provenance_status == "UNRESOLVED"
    {
        Status::Unresolved.as_str()
    } else {
        Status::Pass.as_str()
    };

    if diagnostics.is_empty() {
        diagnostics.push(
            "reported run matches the pinned analytical reference within the manifest tolerances; physical validation is not established"
                .to_owned(),
        );
    }

    Ok(json!({
        "schema_version": COMPARISON_SCHEMA,
        "benchmark_id": benchmark_id,
        "case_id": case_id,
        "manifest_sha256": manifest_sha256,
        "run_receipt_sha256": run_receipt_sha256,
        "verifier_revision": verifier_revision,
        "overall_comparison_status": overall_comparison_status,
        "numerical_comparison_status": numerical_comparison_status,
        "solver_identity_status": solver_identity_status,
        "input_provenance_status": input_provenance_status,
        "execution_evidence_status": "SELF_REPORTED",
        "physical_validation_status": "UNRESOLVED",
        "refinement_results": refinement_results,
        "diagnostics": diagnostics,
        "claim_ceiling": "caller-reported solver outputs compared against a named reference; run authenticity and physical model validation are unresolved"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    const VERIFIER_REVISION: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const SOLVER_BIN_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SOLVER_CONFIG_SHA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn manifest() -> Value {
        let bytes = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/physical-transformation/solver-benchmarks/code-aster-cantilever-planned-v0.1.json"),
        ).expect("planned benchmark manifest exists");
        let mut manifest: Value = serde_json::from_slice(&bytes).expect("manifest parses");
        manifest["solver_candidate"]["version_pin"] = json!("test-only-pinned-version");
        manifest["solver_candidate"]["binary_sha256"] = json!(SOLVER_BIN_SHA);
        manifest["solver_candidate"]["configuration_sha256"] = json!(SOLVER_CONFIG_SHA);
        manifest
    }

    fn manifest_bytes(manifest: &Value) -> Vec<u8> {
        serde_json::to_vec(manifest).expect("manifest serializes")
    }

    fn run_receipt(manifest: &Value, manifest_bytes: &[u8]) -> Value {
        let candidate = &manifest["solver_candidate"];
        let reference = &manifest["reference"];
        let mut refinement_runs = Vec::new();
        for (level, element_count) in [(1, 10), (2, 20), (4, 40)] {
            let mut outputs = Map::new();
            let mut units = Map::new();
            let mut semantics = Map::new();
            for output in reference["outputs"].as_array().unwrap() {
                let key = output["quantity"].as_str().unwrap().to_owned();
                outputs.insert(key.clone(), output["value"].clone());
                units.insert(key.clone(), output["unit"].clone());
                semantics.insert(key, output["semantics"].clone());
            }
            outputs.insert(REACTION_RESIDUAL.to_owned(), json!(0.0001));
            units.insert(REACTION_RESIDUAL.to_owned(), json!("dimensionless"));
            semantics.insert(REACTION_RESIDUAL.to_owned(), json!("relative_reaction_balance_residual"));
            refinement_runs.push(json!({
                "refinement_level": level,
                "element_count": element_count,
                "exit_code": 0,
                "convergence_status": "converged",
                "outputs": outputs,
                "output_units": units,
                "output_semantics": semantics,
                "result_file_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
            }));
        }
        json!({
            "schema_version": RUN_SCHEMA,
            "benchmark_id": manifest["benchmark_id"],
            "manifest_sha256": sha256_hex(manifest_bytes),
            "reference_source_revision": reference["source_revision"],
            "input_fixture_sha256": manifest["input_fixture"]["sha256"],
            "solver": {
                "name": candidate["name"],
                "version_pin": candidate["version_pin"],
                "binary_sha256": candidate["binary_sha256"],
                "configuration_sha256": candidate["configuration_sha256"]
            },
            "runner": {
                "workflow_head_sha": "dddddddddddddddddddddddddddddddddddddddd",
                "workflow_run_id": "unit-test-run-not-an-external-solver",
                "runner_image": "synthetic-unit-test",
                "execution_log_sha256": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            },
            "refinement_runs": refinement_runs
        })
    }

    fn compare_fixture(manifest: &Value, run: &Value) -> Value {
        let manifest_bytes = manifest_bytes(manifest);
        let run_bytes = serde_json::to_vec(run).expect("run receipt serializes");
        compare_manifest_run_bytes(&manifest_bytes, &run_bytes, VERIFIER_REVISION)
            .expect("synthetic comparator fixture is structurally valid")
    }

    #[test]
    fn pinned_synthetic_run_passes_numerical_comparison_but_not_physical_validation() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let run = run_receipt(&manifest, &bytes);
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["overall_comparison_status"], "PASS");
        assert_eq!(receipt["numerical_comparison_status"], "PASS");
        assert_eq!(receipt["solver_identity_status"], "PINNED_MATCH");
        assert_eq!(receipt["input_provenance_status"], "PINNED_MATCH");
        assert_eq!(receipt["execution_evidence_status"], "SELF_REPORTED");
        assert_eq!(receipt["physical_validation_status"], "UNRESOLVED");
        assert_eq!(receipt["refinement_results"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn unpinned_solver_identity_keeps_overall_status_unresolved() {
        let mut manifest = manifest();
        manifest["solver_candidate"]["version_pin"] = Value::Null;
        manifest["solver_candidate"]["binary_sha256"] = Value::Null;
        manifest["solver_candidate"]["configuration_sha256"] = Value::Null;
        let bytes = manifest_bytes(&manifest);
        let run = run_receipt(&manifest(), &bytes);
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["numerical_comparison_status"], "PASS");
        assert_eq!(receipt["solver_identity_status"], "UNRESOLVED");
        assert_eq!(receipt["overall_comparison_status"], "UNRESOLVED");
    }

    #[test]
    fn reference_tolerance_breach_fails_the_comparison() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let mut run = run_receipt(&manifest, &bytes);
        run["refinement_runs"][0]["outputs"]["maximum_bending_stress_pa"] = json!(3_000_000.0);
        run["manifest_sha256"] = json!(sha256_hex(&bytes));
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["numerical_comparison_status"], "FAIL");
        assert_eq!(receipt["overall_comparison_status"], "FAIL");
    }

    #[test]
    fn nonconverged_run_cannot_pass_even_with_reference_values() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let mut run = run_receipt(&manifest, &bytes);
        run["refinement_runs"][1]["convergence_status"] = json!("not_converged");
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["numerical_comparison_status"], "FAIL");
        assert_eq!(receipt["overall_comparison_status"], "FAIL");
    }

    #[test]
    fn mismatched_binary_digest_fails_identity_check() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let mut run = run_receipt(&manifest, &bytes);
        run["solver"]["binary_sha256"] = json!("9999999999999999999999999999999999999999999999999999999999999999");
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["solver_identity_status"], "MISMATCH");
        assert_eq!(receipt["overall_comparison_status"], "FAIL");
    }

    #[test]
    fn stale_manifest_digest_and_fixture_digest_fail_provenance_check() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let mut run = run_receipt(&manifest, &bytes);
        run["manifest_sha256"] = json!("9999999999999999999999999999999999999999999999999999999999999999");
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["input_provenance_status"], "MISMATCH");
        assert_eq!(receipt["overall_comparison_status"], "FAIL");

        let mut run = run_receipt(&manifest, &bytes);
        run["input_fixture_sha256"] = json!("8888888888888888888888888888888888888888888888888888888888888888");
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["input_provenance_status"], "MISMATCH");
        assert_eq!(receipt["overall_comparison_status"], "FAIL");
    }

    #[test]
    fn incomplete_mesh_levels_and_nonincreasing_elements_fail() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let mut run = run_receipt(&manifest, &bytes);
        run["refinement_runs"].as_array_mut().unwrap().pop();
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["numerical_comparison_status"], "FAIL");

        let mut run = run_receipt(&manifest, &bytes);
        run["refinement_runs"][2]["element_count"] = json!(15);
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["numerical_comparison_status"], "FAIL");
    }

    #[test]
    fn output_semantics_mismatch_is_not_accepted() {
        let manifest = manifest();
        let bytes = manifest_bytes(&manifest);
        let mut run = run_receipt(&manifest, &bytes);
        run["refinement_runs"][0]["output_semantics"]["tip_deflection_m"] = json!("some_other_metric");
        let receipt = compare_fixture(&manifest, &run);
        assert_eq!(receipt["numerical_comparison_status"], "FAIL");
        assert_eq!(receipt["physical_validation_status"], "UNRESOLVED");
    }
}
