// SPDX-License-Identifier: AGPL-3.0-or-later
//! Narrow, receipt-producing CalculiX T3D2 axial-bar smoke adapter.
//!
//! This binary intentionally supports one tiny analytical code-verification case.
//! It is not a general CalculiX adapter and does not qualify CalculiX for other use.

use physical_solver_contract::{
    sha256_hex, validate_request_bytes, validate_solver_receipt_bytes,
    verify_solver_output_artifact_bytes,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

type Check<T> = Result<T, String>;
const CASE_ID: &str = "ccx-t3d2-axial-bar-v0.1";
const ADAPTER_ID: &str = "symtropy.calculix.t3d2-axial-bar";
const ADAPTER_VERSION: &str = "0.1.0";
const SCENE_SCHEMA: &str = "physical-scene-truss-v0.1";
const MAX_LOG_BYTES: u64 = 32 * 1024 * 1024;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("calculix-t3d2-smoke: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Check<()> {
    let mut request_path = None;
    let mut scene_path = None;
    let mut artifact_root = None;
    let mut ccx_path = None;
    let mut validator_revision = None;
    let mut solver_package_version = None;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{} requires a value", args[index]))?
            .clone();
        match args[index].as_str() {
            "--request" if request_path.is_none() => request_path = Some(value),
            "--scene" if scene_path.is_none() => scene_path = Some(value),
            "--artifact-root" if artifact_root.is_none() => artifact_root = Some(value),
            "--ccx" if ccx_path.is_none() => ccx_path = Some(value),
            "--validator-revision" if validator_revision.is_none() => {
                validator_revision = Some(value)
            }
            "--solver-package-version" if solver_package_version.is_none() => {
                solver_package_version = Some(value)
            }
            "--request" | "--scene" | "--artifact-root" | "--ccx"
            | "--validator-revision" | "--solver-package-version" => {
                return Err(format!("{} may be supplied only once", args[index]));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 2;
    }

    let request_path = request_path.ok_or_else(|| "--request is required".to_owned())?;
    let scene_path = scene_path.ok_or_else(|| "--scene is required".to_owned())?;
    let artifact_root =
        artifact_root.ok_or_else(|| "--artifact-root is required".to_owned())?;
    let ccx_path = ccx_path.ok_or_else(|| "--ccx is required".to_owned())?;
    let validator_revision =
        validator_revision.ok_or_else(|| "--validator-revision is required".to_owned())?;
    let solver_package_version = solver_package_version
        .ok_or_else(|| "--solver-package-version is required".to_owned())?;

    validate_token(&solver_package_version, "solver package version")?;
    validate_revision(&validator_revision)?;
    fs::create_dir_all(&artifact_root)
        .map_err(|error| format!("cannot create artifact root: {error}"))?;
    let root = fs::canonicalize(&artifact_root)
        .map_err(|error| format!("cannot canonicalize artifact root: {error}"))?;

    let request_bytes = fs::read(&request_path)
        .map_err(|error| format!("cannot read request {request_path}: {error}"))?;
    let scene_bytes = fs::read(&scene_path)
        .map_err(|error| format!("cannot read scene {scene_path}: {error}"))?;
    let request_report = validate_request_bytes(&request_bytes, &validator_revision)?;
    fs::write(root.join("request.json"), &request_bytes)
        .map_err(|error| format!("cannot preserve request bytes: {error}"))?;
    fs::write(root.join("scene.json"), &scene_bytes)
        .map_err(|error| format!("cannot preserve scene bytes: {error}"))?;

    let request: Value =
        serde_json::from_slice(&request_bytes).map_err(|error| format!("invalid request JSON: {error}"))?;
    let scene: Value =
        serde_json::from_slice(&scene_bytes).map_err(|error| format!("invalid scene JSON: {error}"))?;
    validate_case(&request, &scene, &request_bytes, &scene_bytes)?;
    let input_deck = generate_input_deck(&request, &scene)?;
    let input_deck_bytes = input_deck.as_bytes();
    fs::write(root.join("bar.inp"), input_deck_bytes)
        .map_err(|error| format!("cannot write CalculiX input deck: {error}"))?;

    let request_sha256 = sha256_hex(&request_bytes);
    let scene_sha256 = sha256_hex(&scene_bytes);
    let input_deck_sha256 = sha256_hex(input_deck_bytes);
    let ccx_canonical = fs::canonicalize(&ccx_path)
        .map_err(|error| format!("cannot resolve CalculiX executable {ccx_path}: {error}"))?;
    let ccx_bytes = fs::read(&ccx_canonical)
        .map_err(|error| format!("cannot read CalculiX executable bytes: {error}"))?;
    let ccx_sha256 = sha256_hex(&ccx_bytes);
    let adapter_path = env::current_exe()
        .map_err(|error| format!("cannot resolve adapter executable path: {error}"))?;
    let adapter_bytes = fs::read(&adapter_path)
        .map_err(|error| format!("cannot read adapter executable bytes: {error}"))?;
    let adapter_sha256 = sha256_hex(&adapter_bytes);

    let configuration = json!({
        "adapter_protocol": "calculix-t3d2-axial-bar-v0.1",
        "solver_arguments": ["-i", "bar"],
        "element_type": "T3D2",
        "analysis_step": "STATIC",
        "printed_nodal_fields": ["U", "RF"],
        "environment": {
            "OMP_NUM_THREADS": "1",
            "OPENBLAS_NUM_THREADS": "1"
        },
        "parser_protocol": "calculix-node-print-u-rf-v0.1"
    });
    let configuration_bytes = serde_json::to_vec(&configuration)
        .map_err(|error| format!("cannot serialize adapter configuration: {error}"))?;
    let configuration_sha256 = sha256_hex(&configuration_bytes);

    let solver_output = Command::new(&ccx_canonical)
        .arg("-i")
        .arg("bar")
        .current_dir(&root)
        .env("OMP_NUM_THREADS", "1")
        .env("OPENBLAS_NUM_THREADS", "1")
        .output()
        .map_err(|error| format!("could not launch CalculiX: {error}"))?;
    fs::write(root.join("ccx-stdout.log"), &solver_output.stdout)
        .map_err(|error| format!("cannot preserve CalculiX stdout: {error}"))?;
    fs::write(root.join("ccx-stderr.log"), &solver_output.stderr)
        .map_err(|error| format!("cannot preserve CalculiX stderr: {error}"))?;

    let data_path = root.join("bar.dat");
    let status_path = root.join("bar.sta");
    let data_bytes = fs::read(&data_path)
        .map_err(|error| format!("CalculiX did not produce bar.dat: {error}"))?;
    let status_bytes = fs::read(&status_path)
        .map_err(|error| format!("CalculiX did not produce bar.sta: {error}"))?;
    if data_bytes.len() as u64 > MAX_LOG_BYTES || status_bytes.len() as u64 > MAX_LOG_BYTES {
        return Err("CalculiX output exceeded the 32 MiB parser budget".to_owned());
    }

    let execution_evidence = json!({
        "schema_version": "calculix-execution-evidence-v0.1",
        "case_id": CASE_ID,
        "request_sha256": request_sha256,
        "scene_sha256": scene_sha256,
        "input_deck_sha256": input_deck_sha256,
        "solver_package_version": solver_package_version,
        "solver_executable_path": ccx_canonical.to_string_lossy(),
        "solver_executable_sha256": ccx_sha256,
        "adapter_executable_sha256": adapter_sha256,
        "configuration_sha256": configuration_sha256,
        "exit_code": solver_output.status.code(),
        "exit_success": solver_output.status.success(),
        "stdout_sha256": sha256_hex(&solver_output.stdout),
        "stderr_sha256": sha256_hex(&solver_output.stderr),
        "dat_sha256": sha256_hex(&data_bytes),
        "sta_sha256": sha256_hex(&status_bytes),
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "claim_ceiling": "one-element T3D2 axial-bar smoke case only; not general solver qualification"
    });
    write_json(&root.join("execution-evidence.json"), &execution_evidence)?;
    if !solver_output.status.success() {
        return Err(format!(
            "CalculiX exited unsuccessfully with {:?}; logs and raw outputs were retained",
            solver_output.status.code()
        ));
    }

    let dat_text = std::str::from_utf8(&data_bytes)
        .map_err(|error| format!("CalculiX dat output is not UTF-8: {error}"))?;
    let tip_displacement = parse_node_vector(dat_text, "displacements", 2)?;
    let support_reaction = parse_node_vector(dat_text, "forces", 1)?;
    if !tip_displacement.iter().all(|value| value.is_finite())
        || !support_reaction.iter().all(|value| value.is_finite())
    {
        return Err("parsed solver outputs contain non-finite values".to_owned());
    }

    let request_id = required_string(&request, "request_id")?.to_owned();
    let request_source_revision = required_string(&request, "source_revision")?.to_owned();
    let scene_revision = required_u64(&request, "scene_revision")?;
    let scene_digest = required_string(&request, "scene_sha256")?.to_owned();

    let displacement_observation = json!({
        "schema_version": "physical-normalized-observables-v0.1",
        "request_sha256": request_sha256,
        "request_id": request_id,
        "scene_sha256": scene_digest,
        "scene_revision": scene_revision,
        "case_id": CASE_ID,
        "observables": [
            { "observable_id": "tip_ux", "value": tip_displacement[0], "unit": "m" }
        ]
    });
    let reaction_observation = json!({
        "schema_version": "physical-normalized-observables-v0.1",
        "request_sha256": request_sha256,
        "request_id": request_id,
        "scene_sha256": scene_digest,
        "scene_revision": scene_revision,
        "case_id": CASE_ID,
        "observables": [
            { "observable_id": "support_rx", "value": support_reaction[0], "unit": "N" }
        ]
    });
    let displacement_bytes = serde_json::to_vec_pretty(&displacement_observation)
        .map_err(|error| format!("cannot serialize displacement observation: {error}"))?;
    let reaction_bytes = serde_json::to_vec_pretty(&reaction_observation)
        .map_err(|error| format!("cannot serialize reaction observation: {error}"))?;
    fs::write(root.join("normalized-displacement.json"), &displacement_bytes)
        .map_err(|error| format!("cannot write normalized displacement: {error}"))?;
    fs::write(root.join("normalized-reaction-force.json"), &reaction_bytes)
        .map_err(|error| format!("cannot write normalized reaction: {error}"))?;

    let outputs = json!([
        {
            "output_id": "tip-displacement",
            "kind": "displacement",
            "sha256": sha256_hex(&displacement_bytes),
            "media_type": "application/json",
            "unit": "m",
            "artifact_ref": "normalized-displacement.json"
        },
        {
            "output_id": "support-reaction",
            "kind": "reaction_force",
            "sha256": sha256_hex(&reaction_bytes),
            "media_type": "application/json",
            "unit": "N",
            "artifact_ref": "normalized-reaction-force.json"
        }
    ]);
    let input_artifacts = json!([
        {
            "input_id": "scene",
            "kind": "physical_scene",
            "sha256": scene_sha256,
            "media_type": "application/json",
            "artifact_ref": "scene.json"
        },
        {
            "input_id": "solver-input-deck",
            "kind": "finite_element_input_deck",
            "sha256": input_deck_sha256,
            "media_type": "text/plain",
            "artifact_ref": "bar.inp"
        }
    ]);
    let receipt = json!({
        "schema_version": "physical-solver-receipt-v0.1",
        "request_id": request_id,
        "request_sha256": request_sha256,
        "source_revision": request_source_revision,
        "input_artifacts": input_artifacts,
        "adapter": {
            "adapter_id": ADAPTER_ID,
            "version": ADAPTER_VERSION,
            "executable_sha256": adapter_sha256,
            "configuration_sha256": configuration_sha256,
            "solver_name": "CalculiX CrunchiX",
            "solver_version": solver_package_version,
            "solver_build_ref": format!(
                "Ubuntu package calculix-ccx={};ccx_sha256={}",
                solver_package_version, ccx_sha256
            )
        },
        "status": "PASS",
        "evidence_class": "simulated",
        "physics_evaluated": ["static_linear_elastic"],
        "physics_omitted": [],
        "convergence": {
            "state": "not_applicable",
            "iterations": null,
            "residual": null,
            "mesh_refinement_evaluated": false,
            "details": "One-element linear static truss direct solve; iterative convergence is not applicable. Reference accuracy is scored separately."
        },
        "outputs": outputs,
        "uncertainty": {
            "state": "unknown",
            "method": null,
            "summary": "No uncertainty propagation or measurement comparison is included in this one-element smoke case."
        },
        "validity_envelope": {
            "supported_geometry": "Two-node straight x-aligned T3D2 axial bar",
            "supported_material_model": "Homogeneous isotropic linear elasticity",
            "load_regime": "One static axial point force and an axial guide",
            "limitations": [
                "One element and one load case only",
                "Only the x-axis axial displacement and fixed-end x reaction are normalized",
                "No mesh-refinement, nonlinear, contact, dynamics or physical-measurement validation"
            ]
        },
        "claim_ceiling": "CalculiX one-element T3D2 axial-bar analytical smoke case only; not general solver qualification or engineering-use authorization.",
        "diagnostics": []
    });
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("cannot serialize solver receipt: {error}"))?;
    fs::write(root.join("receipt.json"), &receipt_bytes)
        .map_err(|error| format!("cannot write solver receipt: {error}"))?;

    let receipt_validation = validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)?;
    write_json(&root.join("receipt-validation.json"), &receipt_validation)?;
    let artifact_verification =
        verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)?;
    write_json(&root.join("artifact-verification.json"), &artifact_verification)?;
    if artifact_verification["artifact_verification_status"] != "PASS" {
        return Err("adapter receipt or input/output byte verification did not PASS".to_owned());
    }

    let mut final_evidence = execution_evidence;
    final_evidence["request_validation"] = request_report;
    final_evidence["receipt_sha256"] = json!(sha256_hex(&receipt_bytes));
    final_evidence["tip_displacement_m"] = json!(tip_displacement[0]);
    final_evidence["support_reaction_x_n"] = json!(support_reaction[0]);
    final_evidence["receipt_validation_status"] = json!("PASS");
    final_evidence["artifact_verification_status"] = json!("PASS");
    final_evidence["physical_result_qualified"] = json!(false);
    final_evidence["real_world_validation_status"] = json!("UNRESOLVED");
    write_json(&root.join("execution-evidence.json"), &final_evidence)?;
    Ok(())
}

fn validate_case(
    request: &Value,
    scene: &Value,
    request_bytes: &[u8],
    scene_bytes: &[u8],
) -> Check<()> {
    if required_string(request, "model_fidelity")? != "continuum_linear_elastic" {
        return Err("smoke adapter requires continuum_linear_elastic fidelity".to_owned());
    }
    if string_array(request, "requested_physics")? != ["static_linear_elastic"] {
        return Err("smoke adapter supports only static_linear_elastic physics".to_owned());
    }
    let requested_outputs: BTreeSet<String> = string_array(request, "required_outputs")?.into_iter().collect();
    if requested_outputs != ["displacement".to_owned(), "reaction_force".to_owned()].into_iter().collect() {
        return Err("smoke adapter requires exactly displacement and reaction_force outputs".to_owned());
    }
    let request_scene_sha = required_string(request, "scene_sha256")?;
    if request_scene_sha != sha256_hex(scene_bytes) {
        return Err("request.scene_sha256 does not bind the exact scene bytes".to_owned());
    }
    if required_string(scene, "schema_version")? != SCENE_SCHEMA {
        return Err(format!("scene.schema_version must be '{SCENE_SCHEMA}'"));
    }
    if required_string(scene, "coordinate_frame_id")? != required_string(request, "coordinate_frame_id")? {
        return Err("scene and request coordinate frames differ".to_owned());
    }
    let nodes = required_array(scene, "nodes")?;
    if nodes.len() != 2 {
        return Err("smoke adapter supports exactly two nodes".to_owned());
    }
    let node_a_id = required_string(&nodes[0], "node_id")?;
    let node_b_id = required_string(&nodes[1], "node_id")?;
    let coordinates_a = vec3(&nodes[0], "coordinates_m")?;
    let coordinates_b = vec3(&nodes[1], "coordinates_m")?;
    let length = coordinates_b[0] - coordinates_a[0];
    if !length.is_finite()
        || length <= 0.0
        || coordinates_a[1] != coordinates_b[1]
        || coordinates_a[2] != coordinates_b[2]
    {
        return Err("T3D2 smoke scene must be a positive-length x-aligned bar".to_owned());
    }
    let elements = required_array(scene, "elements")?;
    if elements.len() != 1 || required_string(&elements[0], "element_type")? != "T3D2" {
        return Err("smoke adapter supports exactly one T3D2 element".to_owned());
    }
    let element = &elements[0];
    let element_id = required_string(element, "element_id")?;
    let element_nodes = string_array(element, "node_ids")?;
    if element_nodes != [node_a_id.to_owned(), node_b_id.to_owned()] {
        return Err("T3D2 element connectivity must match the ordered scene node inventory".to_owned());
    }
    let area = required_number(element, "cross_section_area_m2")?;
    if area <= 0.0 {
        return Err("element cross-section area must be positive".to_owned());
    }

    let materials = required_array(request, "materials")?;
    if materials.len() != 1 {
        return Err("smoke adapter supports exactly one material".to_owned());
    }
    let material = &materials[0];
    let material_ids = string_array(material, "component_ids")?;
    if material_ids != [element_id.to_owned()] {
        return Err("material component_ids must select the single T3D2 element".to_owned());
    }
    let youngs_modulus = required_number(material, "youngs_modulus_pa")?;
    let poisson_ratio = required_number(material, "poisson_ratio")?;
    if youngs_modulus <= 0.0 || poisson_ratio <= -1.0 || poisson_ratio >= 0.5 {
        return Err("material requires E > 0 and -1 < poisson_ratio < 0.5".to_owned());
    }

    let loads = required_array(request, "loads")?;
    if loads.len() != 1 {
        return Err("smoke adapter supports exactly one point force".to_owned());
    }
    let load = &loads[0];
    if required_string(load, "kind")? != "point_force"
        || required_string(load, "target_id")? != node_b_id
        || required_string(load, "coordinate_frame_id")? != required_string(request, "coordinate_frame_id")?
    {
        return Err("load must be a point_force on the bar tip in the declared frame".to_owned());
    }
    let force = vec3(load, "vector_xyz")?;
    if force[0] <= 0.0 || force[1] != 0.0 || force[2] != 0.0 {
        return Err("smoke adapter supports a positive x-axis tensile point force only".to_owned());
    }

    let boundary_conditions = required_array(request, "boundary_conditions")?;
    if boundary_conditions.len() != 2 {
        return Err("smoke adapter requires a fixed support and an axial guide".to_owned());
    }
    let mut has_fixed = false;
    let mut has_guide = false;
    for condition in boundary_conditions {
        let target = required_string(condition, "target_id")?;
        let kind = required_string(condition, "kind")?;
        let dofs: BTreeSet<String> = string_array(condition, "dofs")?.into_iter().collect();
        if target == node_a_id
            && kind == "fixed"
            && dofs == ["ux".to_owned(), "uy".to_owned(), "uz".to_owned()].into_iter().collect()
        {
            has_fixed = true;
        } else if target == node_b_id
            && kind == "support_link"
            && dofs == ["uy".to_owned(), "uz".to_owned()].into_iter().collect()
        {
            has_guide = true;
        } else {
            return Err("boundary conditions are outside the fixed-end/axial-guide smoke envelope".to_owned());
        }
    }
    if !has_fixed || !has_guide {
        return Err("request must declare the fixed-end support and lateral axial-guide support".to_owned());
    }

    let length_unit = required_string(required_object(request, "units")?, "length")?;
    let force_unit = required_string(required_object(request, "units")?, "force")?;
    if length_unit != "m" || force_unit != "N" {
        return Err("smoke adapter requires SI request units (m, N)".to_owned());
    }
    let _ = request_bytes;
    Ok(())
}

fn generate_input_deck(request: &Value, scene: &Value) -> Check<String> {
    let nodes = required_array(scene, "nodes")?;
    let coordinates_a = vec3(&nodes[0], "coordinates_m")?;
    let coordinates_b = vec3(&nodes[1], "coordinates_m")?;
    let area = required_number(&required_array(scene, "elements")?[0], "cross_section_area_m2")?;
    let youngs_modulus = required_number(&required_array(request, "materials")?[0], "youngs_modulus_pa")?;
    let poisson_ratio = required_number(&required_array(request, "materials")?[0], "poisson_ratio")?;
    let force = vec3(&required_array(request, "loads")?[0], "vector_xyz")?;
    Ok(format!(
        concat!(
            "*HEADING\n",
            "Symtropy CalculiX T3D2 axial-bar smoke v0.1\n",
            "*NODE, NSET=NALL\n",
            "1, {:.16e}, {:.16e}, {:.16e}\n",
            "2, {:.16e}, {:.16e}, {:.16e}\n",
            "*ELEMENT, TYPE=T3D2, ELSET=BAR\n",
            "1, 1, 2\n",
            "*NSET, NSET=FIXED\n",
            "1\n",
            "*NSET, NSET=TIP\n",
            "2\n",
            "*MATERIAL, NAME=STEEL\n",
            "*ELASTIC\n",
            "{:.16e}, {:.16e}\n",
            "*SOLID SECTION, ELSET=BAR, MATERIAL=STEEL\n",
            "{:.16e}\n",
            "*BOUNDARY\n",
            "FIXED, 1, 3\n",
            "TIP, 2, 3\n",
            "*STEP\n",
            "*STATIC\n",
            "*CLOAD\n",
            "TIP, 1, {:.16e}\n",
            "*NODE PRINT, NSET=NALL\n",
            "U\n",
            "RF\n",
            "*END STEP\n"
        ),
        coordinates_a[0], coordinates_a[1], coordinates_a[2],
        coordinates_b[0], coordinates_b[1], coordinates_b[2],
        youngs_modulus, poisson_ratio, area, force[0]
    ))
}

fn parse_node_vector(dat: &str, heading: &str, node_id: u32) -> Check<[f64; 3]> {
    let mut section_active = false;
    for line in dat.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("displacements") && lower.contains("for set") {
            section_active = heading == "displacements";
            continue;
        }
        if lower.contains("forces") && lower.contains("for set") {
            section_active = heading == "forces";
            continue;
        }
        if !section_active {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 || parts[0].parse::<u32>().ok() != Some(node_id) {
            continue;
        }
        let mut vector = [0.0; 3];
        for (index, value) in parts[1..4].iter().enumerate() {
            vector[index] = parse_solver_number(value).ok_or_else(|| {
                format!("cannot parse CalculiX {heading} component '{}' for node {node_id}", value)
            })?;
        }
        if vector.iter().all(|value| value.is_finite()) {
            return Ok(vector);
        }
        return Err(format!("CalculiX {heading} for node {node_id} is non-finite"));
    }
    Err(format!("CalculiX {heading} table did not contain node {node_id}"))
}

fn parse_solver_number(value: &str) -> Option<f64> {
    value.replace(['D', 'd'], "E").parse::<f64>().ok().filter(|n| n.is_finite())
}

fn required_string<'a>(value: &'a Value, key: &str) -> Check<&'a str> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be a string"))
}
fn required_number(value: &Value, key: &str) -> Check<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite()).ok_or_else(|| format!("{key} must be finite numeric"))
}
fn required_u64(value: &Value, key: &str) -> Check<u64> {
    value.get(key).and_then(Value::as_u64).ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn required_array<'a>(value: &'a Value, key: &str) -> Check<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array).ok_or_else(|| format!("{key} must be an array"))
}
fn required_object<'a>(value: &'a Value, key: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value.get(key).and_then(Value::as_object).ok_or_else(|| format!("{key} must be an object"))
}
fn string_array(value: &Value, key: &str) -> Check<Vec<String>> {
    required_array(value, key)?
        .iter()
        .map(|item| item.as_str().map(str::to_owned).ok_or_else(|| format!("{key} entries must be strings")))
        .collect()
}
fn vec3(value: &Value, key: &str) -> Check<[f64; 3]> {
    let values = required_array(value, key)?;
    if values.len() != 3 {
        return Err(format!("{key} must contain exactly three components"));
    }
    let mut output = [0.0; 3];
    for (index, item) in values.iter().enumerate() {
        output[index] = item.as_f64().filter(|n| n.is_finite()).ok_or_else(|| format!("{key}[{index}] must be finite numeric"))?;
    }
    Ok(output)
}
fn write_json(path: &Path, value: &Value) -> Check<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| format!("cannot serialize JSON: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}
fn validate_revision(revision: &str) -> Check<()> {
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err("validator revision must be a 40-character lowercase SHA-1".to_owned());
    }
    Ok(())
}
fn validate_token(value: &str, label: &str) -> Check<()> {
    if value.is_empty() || value.len() > 128
        || !value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(format!("{label} is not a stable token"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_calculix_node_print_displacement_and_force_tables() {
        let dat = "displacements (vx,vy,vz) for set NALL and time 1.000000E+00
1 0.000000E+00 0.000000E+00 0.000000E+00
2 5.000000E-05 0.000000E+00 0.000000E+00

forces (fx,fy,fz) for set NALL and time 1.000000E+00
1 -1.000000E+03 0.000000E+00 0.000000E+00
2 0.000000E+00 0.000000E+00 0.000000E+00
";
        assert_eq!(parse_node_vector(dat, "displacements", 2).unwrap(), [5e-5, 0.0, 0.0]);
        assert_eq!(parse_node_vector(dat, "forces", 1).unwrap(), [-1000.0, 0.0, 0.0]);
    }

    #[test]
    fn missing_output_table_cannot_become_a_zero_result() {
        assert!(parse_node_vector("job finished", "displacements", 2).is_err());
    }
}
