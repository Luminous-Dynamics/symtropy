// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generate and audit the PHYS-MMS-001 plane-stress manufactured-solution fixture.
//!
//! This command does not launch a solver. The caller must bind the generated input,
//! exact solver process receipt, raw output and build provenance into the run manifest.

use std::{
    collections::BTreeMap,
    env,
    fs,
    path::Path,
};

use physical_solver_contract::{
    calculix_mms::{MmsMesh, MmsParameters, audit_nodal_displacements, parse_mms_output},
    sha256_hex,
};
use serde_json::{Value, json};

const BENCHMARK_ID: &str = "PHYS-MMS-001-PLANE-STRESS-CPS3";
const MODEL_SCHEMA: &str = "physical-mms-model-manifest-v0.1";
const RUN_SCHEMA: &str = "physical-mms-run-manifest-v0.1";
const REPORT_SCHEMA: &str = "physical-mms-audit-report-v0.1";

fn main() {
    if let Err(error) = run() {
        eprintln!("physical-mms: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    let options = parse_options(&args[1..])?;
    match command {
        "generate" => generate(&options),
        "audit" => audit(&options),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: physical-mms generate --cells N --deck PATH --model-manifest PATH --source-sha SHA40 --checkout-sha SHA40 --tool-sha SHA64\n       physical-mms audit --cells N --deck PATH --dat PATH --model-manifest PATH --build-manifest PATH --solver PATH --auditor PATH --process-log PATH --run-manifest PATH --report PATH".to_owned()
}

fn parse_options(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    if args.len() % 2 != 0 {
        return Err(usage());
    }
    let mut options = BTreeMap::new();
    let allowed = [
        "--cells",
        "--deck",
        "--model-manifest",
        "--source-sha",
        "--checkout-sha",
        "--tool-sha",
        "--dat",
        "--build-manifest",
        "--solver",
        "--auditor",
        "--process-log",
        "--run-manifest",
        "--report",
    ];
    for pair in args.chunks_exact(2) {
        let key = pair[0].as_str();
        if !allowed.contains(&key) {
            return Err(format!("unknown option '{key}'"));
        }
        if !key.starts_with("--") || pair[1].is_empty() {
            return Err(format!("invalid option pair for '{key}'"));
        }
        if options.insert(key.to_owned(), pair[1].clone()).is_some() {
            return Err(format!("duplicate option '{key}'"));
        }
    }
    Ok(options)
}

fn required(options: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    options
        .get(key)
        .cloned()
        .ok_or_else(|| format!("required option {key} is missing"))
}

fn cells(options: &BTreeMap<String, String>) -> Result<usize, String> {
    let value = required(options, "--cells")?
        .parse::<usize>()
        .map_err(|_| "--cells must be an integer".to_owned())?;
    if !(2..=128).contains(&value) {
        return Err("--cells must be in [2,128]".to_owned());
    }
    Ok(value)
}

fn validate_sha(value: &str, label: &str, length: usize) -> Result<(), String> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!("{label} must be {length} lowercase hexadecimal characters"));
    }
    Ok(())
}

fn read(path: &str, label: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("read {label} '{path}' failed: {error}"))
}

fn write(path: &str, bytes: &[u8], label: &str) -> Result<(), String> {
    let parent = Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("create {label} parent directory failed: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("write {label} '{path}' failed: {error}"))
}

fn json_object(bytes: &[u8], label: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("{label} JSON is invalid: {error}"))?;
    if !value.is_object() {
        return Err(format!("{label} must be a JSON object"));
    }
    Ok(value)
}

fn string_field(value: &Value, key: &str, label: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{label}.{key} must be a string"))
}

fn integer_field(value: &Value, key: &str, label: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label}.{key} must be an unsigned integer"))
}

fn require_equal_string(
    value: &Value,
    key: &str,
    expected: &str,
    label: &str,
) -> Result<(), String> {
    let observed = string_field(value, key, label)?;
    if observed != expected {
        return Err(format!("{label}.{key} differs from the bound run input"));
    }
    Ok(())
}

fn validate_parameters_manifest(
    model_manifest: &Value,
    parameters: MmsParameters,
) -> Result<(), String> {
    let object = model_manifest
        .get("parameters")
        .and_then(Value::as_object)
        .ok_or_else(|| "model manifest.parameters must be an object".to_owned())?;
    if object.len() != 5 {
        return Err("model manifest.parameters must contain exactly five fields".to_owned());
    }
    for (key, expected) in [
        ("youngs_modulus_pa", parameters.youngs_modulus_pa),
        ("poisson_ratio", parameters.poisson_ratio),
        ("thickness_m", parameters.thickness_m),
        ("amplitude_u_m", parameters.amplitude_u_m),
        ("amplitude_v_m", parameters.amplitude_v_m),
    ] {
        let observed = object
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("model manifest.parameters.{key} must be numeric"))?;
        if !observed.is_finite() || observed != expected {
            return Err(format!("model manifest.parameters.{key} differs from the pinned fixture"));
        }
    }
    Ok(())
}

fn generate(options: &BTreeMap<String, String>) -> Result<(), String> {
    let n = cells(options)?;
    let deck_path = required(options, "--deck")?;
    let model_manifest_path = required(options, "--model-manifest")?;
    let source_sha = required(options, "--source-sha")?;
    let checkout_sha = required(options, "--checkout-sha")?;
    let tool_sha = required(options, "--tool-sha")?;
    validate_sha(&source_sha, "source SHA", 40)?;
    validate_sha(&checkout_sha, "checkout SHA", 40)?;
    validate_sha(&tool_sha, "generator executable SHA", 64)?;

    let parameters = MmsParameters::default();
    let mesh = MmsMesh::generate(n, parameters)?;
    let deck = mesh.generate_input_deck(parameters)?;
    let deck_sha = sha256_hex(&deck);

    let model_manifest = json!({
        "schema_version": MODEL_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_sha": source_sha,
        "checkout_sha": checkout_sha,
        "generator_executable_sha256": tool_sha,
        "cells_per_axis": n,
        "node_count": mesh.nodes.len(),
        "element_count": mesh.elements.len(),
        "parameters": {
            "youngs_modulus_pa": parameters.youngs_modulus_pa,
            "poisson_ratio": parameters.poisson_ratio,
            "thickness_m": parameters.thickness_m,
            "amplitude_u_m": parameters.amplitude_u_m,
            "amplitude_v_m": parameters.amplitude_v_m
        },
        "field": {
            "u": "A*sin(pi*x)*sin(pi*y)",
            "v": "B*sin(2*pi*x)*sin(pi*y)",
            "governing_equation": "div(sigma)+body_force=0",
            "constitutive_law": "isotropic small-strain plane stress",
            "body_force": "negative divergence of the exact analytical plane-stress tensor",
            "load_discretization": "consistent nodal body-force integrals using 12x12 tensor Gauss-Legendre quadrature after a Duffy triangle map"
        },
        "element": "CPS3",
        "domain_m": [0.0, 1.0, 0.0, 1.0],
        "terminal_time": 1.0,
        "deck_sha256": deck_sha,
        "quadrature_order_per_axis": 12,
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "engineering_use_authorized": false
    });

    let manifest_bytes = serde_json::to_vec_pretty(&model_manifest)
        .map_err(|error| format!("serialize model manifest failed: {error}"))?;
    write(&deck_path, &deck, "input deck")?;
    write(&model_manifest_path, &manifest_bytes, "model manifest")?;
    println!(
        "{{\"benchmark_id\":\"{BENCHMARK_ID}\",\"cells_per_axis\":{n},\"node_count\":{},\"element_count\":{},\"deck_sha256\":\"{deck_sha}\"}}",
        mesh.nodes.len(),
        mesh.elements.len()
    );
    Ok(())
}

fn audit(options: &BTreeMap<String, String>) -> Result<(), String> {
    let n = cells(options)?;
    let deck_path = required(options, "--deck")?;
    let dat_path = required(options, "--dat")?;
    let model_manifest_path = required(options, "--model-manifest")?;
    let build_manifest_path = required(options, "--build-manifest")?;
    let solver_path = required(options, "--solver")?;
    let auditor_path = required(options, "--auditor")?;
    let process_log_path = required(options, "--process-log")?;
    let run_manifest_path = required(options, "--run-manifest")?;
    let report_path = required(options, "--report")?;

    let deck_bytes = read(&deck_path, "input deck")?;
    let dat_bytes = read(&dat_path, "solver .dat output")?;
    let model_manifest_bytes = read(&model_manifest_path, "model manifest")?;
    let build_manifest_bytes = read(&build_manifest_path, "build manifest")?;
    let solver_bytes = read(&solver_path, "solver executable")?;
    let auditor_bytes = read(&auditor_path, "auditor executable")?;
    let process_log_bytes = read(&process_log_path, "process log")?;
    let run_manifest_bytes = read(&run_manifest_path, "run manifest")?;

    let model_manifest = json_object(&model_manifest_bytes, "model manifest")?;
    let build_manifest = json_object(&build_manifest_bytes, "build manifest")?;
    let run_manifest = json_object(&run_manifest_bytes, "run manifest")?;
    if string_field(&model_manifest, "schema_version", "model manifest")? != MODEL_SCHEMA
        || string_field(&model_manifest, "benchmark_id", "model manifest")? != BENCHMARK_ID
    {
        return Err("model manifest has an unsupported schema or benchmark ID".to_owned());
    }
    if string_field(&run_manifest, "schema_version", "run manifest")? != RUN_SCHEMA
        || string_field(&run_manifest, "benchmark_id", "run manifest")? != BENCHMARK_ID
    {
        return Err("run manifest has an unsupported schema or benchmark ID".to_owned());
    }
    if integer_field(&model_manifest, "cells_per_axis", "model manifest")? != n as u64
        || integer_field(&run_manifest, "cells_per_axis", "run manifest")? != n as u64
    {
        return Err(
            "mesh cell count differs between command, model manifest and run manifest".to_owned(),
        );
    }

    let source_sha = string_field(&run_manifest, "source_sha", "run manifest")?;
    let checkout_sha = string_field(&run_manifest, "checkout_sha", "run manifest")?;
    validate_sha(&source_sha, "source SHA", 40)?;
    validate_sha(&checkout_sha, "checkout SHA", 40)?;
    require_equal_string(&model_manifest, "source_sha", &source_sha, "model manifest")?;
    require_equal_string(&model_manifest, "checkout_sha", &checkout_sha, "model manifest")?;

    let deck_sha = sha256_hex(&deck_bytes);
    let dat_sha = sha256_hex(&dat_bytes);
    let model_manifest_sha = sha256_hex(&model_manifest_bytes);
    let build_manifest_sha = sha256_hex(&build_manifest_bytes);
    let solver_sha = sha256_hex(&solver_bytes);
    let auditor_sha = sha256_hex(&auditor_bytes);
    let process_log_sha = sha256_hex(&process_log_bytes);
    let run_manifest_sha = sha256_hex(&run_manifest_bytes);

    require_equal_string(&model_manifest, "deck_sha256", &deck_sha, "model manifest")?;
    require_equal_string(
        &model_manifest,
        "generator_executable_sha256",
        &auditor_sha,
        "model manifest",
    )?;
    for (key, digest, label) in [
        ("model_manifest_sha256", model_manifest_sha.as_str(), "run manifest"),
        ("deck_sha256", deck_sha.as_str(), "run manifest"),
        ("auditor_executable_sha256", auditor_sha.as_str(), "run manifest"),
        ("dat_sha256", dat_sha.as_str(), "run manifest"),
        ("build_manifest_sha256", build_manifest_sha.as_str(), "run manifest"),
        ("solver_executable_sha256", solver_sha.as_str(), "run manifest"),
        ("process_log_sha256", process_log_sha.as_str(), "run manifest"),
    ] {
        require_equal_string(&run_manifest, key, digest, label)?;
    }
    if integer_field(&run_manifest, "solver_exit_code", "run manifest")? != 0
        || run_manifest.get("solver_completed").and_then(Value::as_bool) != Some(true)
    {
        return Err(
            "run manifest does not attest a completed process with zero exit code".to_owned(),
        );
    }
    for (key, expected) in [
        ("schema_version", "calculix-build-manifest-v0.1"),
        ("builder_repository", "OmnibusCloud/CalculiX"),
        ("builder_repository_commit", "b7977a4e90b40fd28fa149fc69f1647661d10f56"),
        ("ccx_source_sha256", "3a94dcc775a31f570229734b341d6b06301ebdc759863df901c8b9bf1854c0bc"),
        ("spooles_sha256", "a84559a0e987a1e423055ef4fdf3035d55b65bbe4bf915efaa1a35bef7f8c5dd"),
        ("container_image", "ubuntu@sha256:2e863c44b718727c860746568e1d54afd13b2fa71b160f5cd9058fc436217b30"),
    ] {
        require_equal_string(&build_manifest, key, expected, "build manifest")?;
    }
    require_equal_string(
        &build_manifest,
        "ccx_source_url",
        "http://www.dhondt.de/ccx_2.22.src.tar.bz2",
        "build manifest",
    )?;
    if build_manifest.get("arpack_enabled").and_then(Value::as_bool) != Some(false) {
        return Err("pinned PHYS-MMS-001 build must explicitly disable ARPACK".to_owned());
    }
    let build_config = build_manifest
        .get("build_config")
        .and_then(Value::as_object)
        .ok_or_else(|| "build manifest.build_config must be an object".to_owned())?;
    if build_config.len() != 4
        || build_config.get("with_pardiso").and_then(Value::as_bool) != Some(false)
        || build_config.get("with_pastix").and_then(Value::as_bool) != Some(false)
        || build_config.get("with_arpack").and_then(Value::as_bool) != Some(false)
        || build_config.get("with_mt").and_then(Value::as_bool) != Some(true)
    {
        return Err("build manifest flags differ from the pinned SPOOLES-only profile".to_owned());
    }
    if string_field(&build_manifest, "ccx_executable_sha256", "build manifest")? != solver_sha {
        return Err(
            "build manifest executable digest differs from the audited solver bytes".to_owned(),
        );
    }

    let parameters = MmsParameters::default();
    validate_parameters_manifest(&model_manifest, parameters)?;
    let mesh = MmsMesh::generate(n, parameters)?;
    let expected_deck = mesh.generate_input_deck(parameters)?;
    if expected_deck != deck_bytes {
        return Err(
            "input deck bytes differ from the deterministic manufactured mesh generator".to_owned(),
        );
    }
    if integer_field(&model_manifest, "node_count", "model manifest")? != mesh.nodes.len() as u64
        || integer_field(&model_manifest, "element_count", "model manifest")?
            != mesh.elements.len() as u64
    {
        return Err(
            "model manifest node/element counts differ from regenerated topology".to_owned(),
        );
    }

    let parsed_output = parse_mms_output(
        std::str::from_utf8(&dat_bytes)
            .map_err(|_| "solver .dat output is not UTF-8 text".to_owned())?,
        mesh.nodes.len(),
    )?;
    let metrics = audit_nodal_displacements(&mesh, parameters, &parsed_output.displacements_m)?;

    let report = json!({
        "schema_version": REPORT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_sha": source_sha,
        "checkout_sha": checkout_sha,
        "cells_per_axis": n,
        "node_count": mesh.nodes.len(),
        "element_count": mesh.elements.len(),
        "model_manifest_sha256": model_manifest_sha,
        "deck_sha256": deck_sha,
        "dat_sha256": dat_sha,
        "build_manifest_sha256": build_manifest_sha,
        "solver_executable_sha256": solver_sha,
        "auditor_executable_sha256": auditor_sha,
        "process_log_sha256": process_log_sha,
        "run_manifest_sha256": run_manifest_sha,
        "solver_exit_code": 0,
        "observed_terminal_time": parsed_output.terminal_time_s,
        "displacement_l1_integral_m3": metrics.displacement_l1_integral_m3,
        "displacement_l2_norm_m2": metrics.displacement_l2_norm_m2,
        "displacement_quadrature_linf_error_m": metrics.displacement_quadrature_linf_error_m,
        "displacement_max_nodal_error_m": metrics.displacement_max_nodal_error_m,
        "displacement_h1_seminorm_error_m": metrics.displacement_h1_seminorm_error_m,
        "stress_l2_error_pa_m": metrics.stress_l2_error_pa_m,
        "numeric_reference_status": "MEASURED_UNQUALIFIED",
        "mesh_convergence_status": "NOT_EVALUATED_SINGLE_MESH",
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "engineering_use_authorized": false,
        "claim_ceiling": "field-error measurement against an exact manufactured plane-stress solution for this exact input/output/build/provenance byte set; does not establish physical validation or engineering safety"
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize audit report failed: {error}"))?;
    write(&report_path, &report_bytes, "audit report")?;
    println!(
        "{{\"benchmark_id\":\"{BENCHMARK_ID}\",\"cells_per_axis\":{n},\"numeric_reference_status\":\"MEASURED_UNQUALIFIED\",\"mesh_convergence_status\":\"NOT_EVALUATED_SINGLE_MESH\",\"physical_result_qualified\":false,\"report_sha256\":\"{}\"}}",
        sha256_hex(&report_bytes)
    );
    Ok(())
}
