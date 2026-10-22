// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strict analytical displacement comparison for a pinned linear axial-bar profile.
//!
//! The parser consumes exact request, input-deck, .dat, manifest, and build-manifest
//! bytes. It does not execute or authenticate a solver; the execution workflow binds
//! the produced bytes to the binary/build receipt and keeps qualification unresolved.

use crate::{Check, sha256_hex};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_DECK_BYTES: usize = 16 * 1024 * 1024;
const MAX_DAT_BYTES: usize = 32 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;

const MANIFEST_SCHEMA: &str = "calculix-displacement-qoi-manifest-v0.1";
const BUILD_SCHEMA: &str = "calculix-build-manifest-v0.1";
const DAT_HEADER_PREFIX: &str = "displacements (vx,vy,vz) for set ";

const PINNED_BUILDER_REPOSITORY: &str = "OmnibusCloud/CalculiX";
const PINNED_BUILDER_COMMIT: &str = "b7977a4e90b40fd28fa149fc69f1647661d10f56";
const PINNED_CONTAINER_IMAGE: &str =
    "ubuntu@sha256:2e863c44b718727c860746568e1d54afd13b2fa71b160f5cd9058fc436217b30";
const PINNED_CCX_SOURCE_URL: &str = "http://www.dhondt.de/ccx_2.22.src.tar.bz2";
const PINNED_CCX_SOURCE_SHA256: &str =
    "3a94dcc775a31f570229734b341d6b06301ebdc759863df901c8b9bf1854c0bc";
const PINNED_SPOOLES_SHA256: &str =
    "a84559a0e987a1e423055ef4fdf3035d55b65bbe4bf915efaa1a35bef7f8c5dd";

#[derive(Clone, Debug)]
struct DeckCard {
    name: String,
    params: BTreeMap<String, String>,
    rows: Vec<(usize, String)>,
}

#[derive(Clone, Debug)]
struct ParsedDeck {
    nodes: BTreeMap<u64, [f64; 3]>,
    elements: Vec<[u64; 2]>,
    node_sets: BTreeMap<String, BTreeSet<u64>>,
    point_loads: Vec<(u64, u8, f64)>,
    youngs_modulus_pa: f64,
    poisson_ratio: f64,
    area_m2: f64,
    printed_set: String,
}

#[derive(Clone, Debug)]
struct DisplacementRow {
    node_id: u64,
    displacement_m: [f64; 3],
    source_line: usize,
    raw_line: String,
}

pub fn compare_displacement_reference_bytes(
    request_bytes: &[u8],
    deck_bytes: &[u8],
    dat_bytes: &[u8],
    manifest_bytes: &[u8],
    build_manifest_bytes: &[u8],
) -> Check<Value> {
    for (name, bytes, limit) in [
        ("request", request_bytes, MAX_REQUEST_BYTES),
        ("input deck", deck_bytes, MAX_DECK_BYTES),
        (".dat output", dat_bytes, MAX_DAT_BYTES),
        ("displacement manifest", manifest_bytes, MAX_MANIFEST_BYTES),
        ("build manifest", build_manifest_bytes, MAX_MANIFEST_BYTES),
    ] {
        if bytes.is_empty() || bytes.len() > limit {
            return Err(format!("{name} is empty or exceeds its byte budget"));
        }
    }

    let request_sha256 = sha256_hex(request_bytes);
    let deck_sha256 = sha256_hex(deck_bytes);
    let dat_sha256 = sha256_hex(dat_bytes);
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let build_manifest_sha256 = sha256_hex(build_manifest_bytes);

    let request = parse_object(request_bytes, "request")?;
    let manifest = parse_object(manifest_bytes, "displacement manifest")?;
    let build = parse_object(build_manifest_bytes, "build manifest")?;
    validate_build_manifest(&build)?;

    exact_keys(
        &manifest,
        &[
            "schema_version",
            "case_id",
            "request_id",
            "request_sha256",
            "scene_sha256",
            "scene_revision",
            "coordinate_frame_id",
            "deck_sha256",
            "dat_sha256",
            "build_manifest_sha256",
            "solver_executable_sha256",
            "support_target_id",
            "load_target_id",
            "support_node_id",
            "load_node_id",
            "node_set",
            "component",
            "absolute_tolerance_m",
            "relative_tolerance",
            "target_time",
            "model_nodes",
            "provenance",
        ],
        "displacement manifest",
    )?;

    if field_string(&manifest, "schema_version", "manifest")? != MANIFEST_SCHEMA {
        return Err(format!(
            "manifest.schema_version must equal {MANIFEST_SCHEMA}"
        ));
    }
    let case_id = token(field_string(&manifest, "case_id", "manifest")?, "case_id")?;
    let request_id = field_string(&manifest, "request_id", "manifest")?.to_owned();
    let scene_sha256 = field_sha(&manifest, "scene_sha256", "manifest")?;
    let scene_revision = field_u64(&manifest, "scene_revision", "manifest")?;
    let coordinate_frame_id = token(
        field_string(&manifest, "coordinate_frame_id", "manifest")?,
        "coordinate_frame_id",
    )?;
    let support_target_id = token(
        field_string(&manifest, "support_target_id", "manifest")?,
        "support_target_id",
    )?;
    let load_target_id = token(
        field_string(&manifest, "load_target_id", "manifest")?,
        "load_target_id",
    )?;
    let support_node_id = field_u64(&manifest, "support_node_id", "manifest")?;
    let load_node_id = field_u64(&manifest, "load_node_id", "manifest")?;
    let node_set = token(field_string(&manifest, "node_set", "manifest")?, "node_set")?;
    let component = field_string(&manifest, "component", "manifest")?;
    let nodes = parse_model_nodes(&manifest)?;
    if component != "ux" {
        return Err("v0.1 only supports the global axial displacement component ux".to_owned());
    }
    let expected_time = field_f64(&manifest, "target_time", "manifest")?;
    if expected_time <= 0.0 {
        return Err("manifest.target_time must be positive".to_owned());
    }

    for (field, observed) in [
        ("request_sha256", &request_sha256),
        ("deck_sha256", &deck_sha256),
        ("dat_sha256", &dat_sha256),
        ("build_manifest_sha256", &build_manifest_sha256),
    ] {
        if field_sha(&manifest, field, "manifest")? != observed {
            return Err(format!(
                "manifest.{field} does not match the exact input bytes"
            ));
        }
    }
    let solver_executable_sha256 = field_sha(&manifest, "solver_executable_sha256", "manifest")?;
    if solver_executable_sha256 != field_sha(&build, "ccx_executable_sha256", "build manifest")? {
        return Err(
            "displacement manifest executable digest differs from build manifest".to_owned(),
        );
    }

    if field_string(&request, "schema_version", "request")? != "physical-solver-request-v0.1"
        || field_string(&request, "request_id", "request")? != request_id
        || field_sha(&request, "scene_sha256", "request")? != scene_sha256
        || field_u64(&request, "scene_revision", "request")? != scene_revision
        || field_string(&request, "coordinate_frame_id", "request")? != coordinate_frame_id
    {
        return Err("manifest identity differs from the exact physical request".to_owned());
    }
    if field_string(&request, "model_fidelity", "request")? != "analytical_linear_elastic" {
        return Err(
            "displacement comparison requires analytical_linear_elastic model fidelity".to_owned(),
        );
    }
    let units = request
        .get("units")
        .and_then(Value::as_object)
        .ok_or_else(|| "request.units must be an object".to_owned())?;
    if units.get("length").and_then(Value::as_str) != Some("m")
        || units.get("force").and_then(Value::as_str) != Some("N")
        || units.get("stress").and_then(Value::as_str) != Some("Pa")
    {
        return Err("request must declare SI length=m, force=N, and stress=Pa".to_owned());
    }

    let materials = array_field(&request, "materials", "request")?;
    if materials.len() != 1 {
        return Err("v0.1 requires exactly one declared material".to_owned());
    }
    let material = &materials[0];
    let request_modulus = field_positive_f64(material, "youngs_modulus_pa", "material")?;
    let request_poisson = field_f64(material, "poisson_ratio", "material")?;
    if !(0.0..0.5).contains(&request_poisson) {
        return Err("material Poisson ratio must be finite and in [0, 0.5)".to_owned());
    }

    let loads = array_field(&request, "loads", "request")?;
    if loads.len() != 1 {
        return Err("v0.1 requires exactly one point-force load".to_owned());
    }
    let load = &loads[0];
    if field_string(load, "kind", "load")? != "point_force"
        || field_string(load, "target_id", "load")? != load_target_id
        || field_string(load, "coordinate_frame_id", "load")? != coordinate_frame_id
    {
        return Err("manifest load target/frame differs from the exact request".to_owned());
    }
    let load_vector = array_field(load, "vector_xyz", "load")?;
    if load_vector.len() != 3 {
        return Err("load.vector_xyz must contain exactly three components".to_owned());
    }
    let request_force = [
        finite_number(&load_vector[0], "load.vector_xyz[0]")?,
        finite_number(&load_vector[1], "load.vector_xyz[1]")?,
        finite_number(&load_vector[2], "load.vector_xyz[2]")?,
    ];
    if request_force[0] <= 0.0 || request_force[1] != 0.0 || request_force[2] != 0.0 {
        return Err("v0.1 requires a positive global-x force with zero y/z components".to_owned());
    }

    let boundaries = array_field(&request, "boundary_conditions", "request")?;
    if boundaries.len() != nodes.len() {
        return Err(format!(
            "v0.1 requires one fixed support plus one zero-valued lateral guide per non-support mesh node (expected {} boundaries, observed {})",
            nodes.len(),
            boundaries.len()
        ));
    }
    let manifest_nodes = array_field(&manifest, "model_nodes", "manifest")?;
    let mut observed_boundary_targets = BTreeSet::new();
    let mut support_boundary_seen = false;
    for boundary in boundaries {
        if field_string(boundary, "kind", "boundary")? != "fixed" {
            return Err(
                "v0.1 axial-bar boundaries must be zero-valued fixed constraints".to_owned(),
            );
        }
        let target_id = token(
            field_string(boundary, "target_id", "boundary")?,
            "boundary.target_id",
        )?;
        if !observed_boundary_targets.insert(target_id) {
            return Err(format!("duplicate request boundary target '{target_id}'"));
        }
        let dofs: BTreeSet<&str> = array_field(boundary, "dofs", "boundary")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "boundary.dofs must contain strings".to_owned())
            })
            .collect::<Check<BTreeSet<&str>>>()?;
        if target_id == support_target_id {
            if dofs != ["ux", "uy", "uz"].into_iter().collect() || support_boundary_seen {
                return Err("v0.1 support boundary must fix ux, uy and uz exactly once".to_owned());
            }
            support_boundary_seen = true;
        } else {
            let target_is_mesh_node = manifest_nodes
                .iter()
                .any(|node| node.get("target_id").and_then(Value::as_str) == Some(target_id));
            if !target_is_mesh_node || dofs != ["uy", "uz"].into_iter().collect() {
                return Err(format!(
                    "v0.1 non-support mesh boundary '{target_id}' must be a declared node target fixed only in uy/uz"
                ));
            }
        }
    }
    let expected_boundary_targets: BTreeSet<&str> = manifest_nodes
        .iter()
        .filter_map(|node| node.get("target_id").and_then(Value::as_str))
        .collect();
    if !support_boundary_seen || observed_boundary_targets != expected_boundary_targets {
        return Err(
            "request boundaries must cover the support and exactly every model-node lateral guide"
                .to_owned(),
        );
    }

    let support_pos = nodes
        .get(&support_node_id)
        .ok_or_else(|| "manifest model_nodes omit support_node_id".to_owned())?;
    let load_pos = nodes
        .get(&load_node_id)
        .ok_or_else(|| "manifest model_nodes omit load_node_id".to_owned())?;
    if support_node_id == load_node_id {
        return Err("support and load node identities must differ".to_owned());
    }
    if !approx_eq(support_pos[1], load_pos[1], 1.0e-12)
        || !approx_eq(support_pos[2], load_pos[2], 1.0e-12)
        || !approx_eq(support_pos[1], 0.0, 1.0e-12)
        || !approx_eq(support_pos[2], 0.0, 1.0e-12)
    {
        return Err("v0.1 bar endpoints must lie on the global x axis".to_owned());
    }
    let length_m = load_pos[0] - support_pos[0];
    if !length_m.is_finite() || length_m <= 0.0 {
        return Err(
            "v0.1 requires the load endpoint to lie at positive x from the support".to_owned(),
        );
    }

    let deck_text = std::str::from_utf8(deck_bytes)
        .map_err(|_| "CalculiX input deck must be UTF-8/ASCII text".to_owned())?;
    let deck = parse_and_validate_deck(
        deck_text,
        support_node_id,
        load_node_id,
        &node_set,
        expected_time,
    )?;
    if deck.nodes.len() != deck.elements.len() + 1 {
        return Err(
            "axial bar mesh must contain exactly one more node than T3D2 elements".to_owned(),
        );
    }
    for (id, position) in &deck.nodes {
        let manifest_position = nodes
            .get(id)
            .ok_or_else(|| format!("manifest model_nodes omit deck node {id}"))?;
        for axis in 0..3 {
            if !approx_eq(position[axis], manifest_position[axis], 1.0e-12) {
                return Err(format!(
                    "node {id} coordinate differs between deck and model-node manifest"
                ));
            }
        }
    }
    if nodes.len() != deck.nodes.len() {
        return Err("manifest model_nodes contains IDs absent from the input deck".to_owned());
    }
    if deck.nodes.get(&support_node_id) != Some(support_pos)
        || deck.nodes.get(&load_node_id) != Some(load_pos)
    {
        return Err("support/load node coordinates disagree with the exact deck".to_owned());
    }
    if !approx_eq(
        deck.youngs_modulus_pa,
        request_modulus,
        request_modulus * 1.0e-12,
    ) || !approx_eq(deck.poisson_ratio, request_poisson, 1.0e-12)
    {
        return Err("deck elastic material constants differ from the physical request".to_owned());
    }
    if deck.printed_set != node_set {
        return Err("deck U output node set differs from manifest.node_set".to_owned());
    }
    if deck.node_sets.get(node_set) != Some(&[load_node_id].into_iter().collect()) {
        return Err("tip output set must contain exactly the declared load node".to_owned());
    }
    if deck.point_loads.len() != 1 {
        return Err("deck must contain exactly one concentrated load".to_owned());
    }
    let (observed_load_node, observed_dof, observed_force) = deck.point_loads[0];
    if observed_load_node != load_node_id
        || observed_dof != 1
        || !approx_eq(
            observed_force,
            request_force[0],
            request_force[0].abs() * 1.0e-12,
        )
    {
        return Err("deck CLOAD does not match the exact request load".to_owned());
    }

    let absolute_tolerance_m = field_positive_f64(&manifest, "absolute_tolerance_m", "manifest")?;
    let relative_tolerance = field_nonnegative_f64(&manifest, "relative_tolerance", "manifest")?;
    validate_provenance(
        manifest
            .get("provenance")
            .ok_or_else(|| "manifest.provenance is required".to_owned())?,
    )?;

    let dat_text = std::str::from_utf8(dat_bytes)
        .map_err(|_| "CalculiX .dat output must be UTF-8/ASCII text".to_owned())?;
    let (observed_set, observed_time, row) =
        parse_displacement_section(dat_text, &node_set, load_node_id)?;
    let time_tolerance = 1.0e-8 * expected_time.abs().max(1.0);
    let time_matches = (observed_time - expected_time).abs() <= time_tolerance;

    let expected_displacement_m =
        request_force[0] * length_m / (deck.youngs_modulus_pa * deck.area_m2);
    if !expected_displacement_m.is_finite() || expected_displacement_m <= 0.0 {
        return Err("computed analytical displacement is non-finite or non-positive".to_owned());
    }
    let observed_displacement_m = row.displacement_m[0];
    let absolute_error_m = (observed_displacement_m - expected_displacement_m).abs();
    let allowed_error_m = absolute_tolerance_m + relative_tolerance * expected_displacement_m.abs();
    let numeric_pass =
        time_matches && observed_displacement_m.is_finite() && absolute_error_m <= allowed_error_m;

    Ok(json!({
        "schema_version": "calculix-displacement-qoi-report-v0.1",
        "case_id": case_id,
        "request_id": request_id,
        "request_sha256": request_sha256,
        "deck_sha256": deck_sha256,
        "dat_sha256": dat_sha256,
        "manifest_sha256": manifest_sha256,
        "build_manifest_sha256": build_manifest_sha256,
        "builder_repository": field_string(&build, "builder_repository", "build manifest")?,
        "builder_repository_commit": field_string(&build, "builder_repository_commit", "build manifest")?,
        "container_image": field_string(&build, "container_image", "build manifest")?,
        "solver_executable_sha256": solver_executable_sha256,
        "source_archive_sha256": PINNED_CCX_SOURCE_SHA256,
        "measurement": "global_x_displacement",
        "unit": "m",
        "node_set": observed_set,
        "node_id": row.node_id,
        "observed_time": observed_time,
        "expected_time": expected_time,
        "time_matches": time_matches,
        "raw_displacement_m": row.displacement_m,
        "raw_source_line": row.raw_line,
        "source_line": row.source_line,
        "reference_method": "u=F*L/(E*A), straight uniform axial bar with small-strain linear elasticity",
        "reference_force_n": request_force[0],
        "reference_length_m": length_m,
        "reference_youngs_modulus_pa": deck.youngs_modulus_pa,
        "reference_area_m2": deck.area_m2,
        "reference_displacement_m": expected_displacement_m,
        "observed_displacement_m": observed_displacement_m,
        "absolute_error_m": absolute_error_m,
        "allowed_error_m": allowed_error_m,
        "absolute_tolerance_m": absolute_tolerance_m,
        "relative_tolerance": relative_tolerance,
        "numeric_reference_status": if numeric_pass { "PASS" } else { "FAIL" },
        "solver_process_exit_verified_by_caller": false,
        "solver_execution_attested": false,
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "engineering_use_authorized": false,
        "claim_ceiling": "analytical axial-bar displacement agreement for this exact request/deck/output/build-manifest byte set; the report does not authenticate process execution, qualify general CalculiX capability, or establish experimental physical validity"
    }))
}

fn validate_build_manifest(build: &Value) -> Check<()> {
    exact_keys(
        build,
        &[
            "schema_version",
            "builder_repository",
            "builder_repository_commit",
            "container_image",
            "ccx_source_url",
            "ccx_source_sha256",
            "spooles_sha256",
            "arpack_enabled",
            "ccx_executable_sha256",
            "toolchain_manifest_sha256",
            "build_log_sha256",
            "build_config",
        ],
        "build manifest",
    )?;
    if field_string(build, "schema_version", "build manifest")? != BUILD_SCHEMA
        || field_string(build, "builder_repository", "build manifest")? != PINNED_BUILDER_REPOSITORY
        || field_string(build, "builder_repository_commit", "build manifest")?
            != PINNED_BUILDER_COMMIT
        || field_string(build, "container_image", "build manifest")? != PINNED_CONTAINER_IMAGE
        || field_string(build, "ccx_source_url", "build manifest")? != PINNED_CCX_SOURCE_URL
        || field_sha(build, "ccx_source_sha256", "build manifest")? != PINNED_CCX_SOURCE_SHA256
        || field_sha(build, "spooles_sha256", "build manifest")? != PINNED_SPOOLES_SHA256
    {
        return Err("build manifest does not match the pinned CCX source/build profile".to_owned());
    }
    if build.get("arpack_enabled").and_then(Value::as_bool) != Some(false) {
        return Err("v0.1 static-bar profile must explicitly disable ARPACK".to_owned());
    }
    field_sha(build, "ccx_executable_sha256", "build manifest")?;
    field_sha(build, "toolchain_manifest_sha256", "build manifest")?;
    field_sha(build, "build_log_sha256", "build manifest")?;
    let config = build
        .get("build_config")
        .and_then(Value::as_object)
        .ok_or_else(|| "build_manifest.build_config must be an object".to_owned())?;
    if config.get("with_pardiso").and_then(Value::as_bool) != Some(false)
        || config.get("with_pastix").and_then(Value::as_bool) != Some(false)
        || config.get("with_arpack").and_then(Value::as_bool) != Some(false)
        || config.get("with_mt").and_then(Value::as_bool) != Some(true)
    {
        return Err(
            "build config does not match the declared SPOOLES-only static-bar profile".to_owned(),
        );
    }
    Ok(())
}

fn parse_model_nodes(manifest: &Value) -> Check<BTreeMap<u64, [f64; 3]>> {
    let values = array_field(manifest, "model_nodes", "manifest")?;
    if values.len() < 2 || values.len() > 100_000 {
        return Err("manifest.model_nodes must contain between 2 and 100000 nodes".to_owned());
    }
    let mut nodes = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for value in values {
        exact_keys(value, &["node_id", "target_id", "position_m"], "model node")?;
        let id = field_u64(value, "node_id", "model node")?;
        let target = token(
            field_string(value, "target_id", "model node")?,
            "model node target_id",
        )?;
        if !targets.insert(target.to_owned()) {
            return Err("model-node target IDs must be unique".to_owned());
        }
        let position = array_field(value, "position_m", "model node")?;
        if position.len() != 3 {
            return Err("model-node position_m must contain three coordinates".to_owned());
        }
        let coordinate = [
            finite_number(&position[0], "position_m[0]")?,
            finite_number(&position[1], "position_m[1]")?,
            finite_number(&position[2], "position_m[2]")?,
        ];
        if nodes.insert(id, coordinate).is_some() {
            return Err(format!("duplicate model node ID {id}"));
        }
    }
    let support_id = field_u64(manifest, "support_node_id", "manifest")?;
    let load_id = field_u64(manifest, "load_node_id", "manifest")?;
    if !nodes.contains_key(&support_id) || !nodes.contains_key(&load_id) {
        return Err("model_nodes must include both declared endpoint node IDs".to_owned());
    }
    let support_target = field_string(manifest, "support_target_id", "manifest")?;
    let load_target = field_string(manifest, "load_target_id", "manifest")?;
    let entries: Vec<&Value> = values.iter().collect();
    if entries
        .iter()
        .filter(|node| {
            node.get("node_id").and_then(Value::as_u64) == Some(support_id)
                && node.get("target_id").and_then(Value::as_str) == Some(support_target)
        })
        .count()
        != 1
        || entries
            .iter()
            .filter(|node| {
                node.get("node_id").and_then(Value::as_u64) == Some(load_id)
                    && node.get("target_id").and_then(Value::as_str) == Some(load_target)
            })
            .count()
            != 1
    {
        return Err(
            "support/load endpoint target identities differ from model-node catalogue".to_owned(),
        );
    }
    Ok(nodes)
}

fn parse_and_validate_deck(
    deck_text: &str,
    support_node_id: u64,
    load_node_id: u64,
    expected_node_set: &str,
    expected_time: f64,
) -> Check<ParsedDeck> {
    let cards = parse_cards(deck_text)?;
    let allowed: BTreeSet<&str> = [
        "HEADING",
        "NODE",
        "ELEMENT",
        "NSET",
        "MATERIAL",
        "ELASTIC",
        "SOLID SECTION",
        "BOUNDARY",
        "STEP",
        "STATIC",
        "CLOAD",
        "NODE PRINT",
        "END STEP",
    ]
    .into_iter()
    .collect();
    if cards
        .iter()
        .any(|card| !allowed.contains(card.name.as_str()))
    {
        let unsupported: Vec<&str> = cards
            .iter()
            .filter(|card| !allowed.contains(card.name.as_str()))
            .map(|card| card.name.as_str())
            .collect();
        return Err(format!(
            "unsupported CalculiX card(s) in v0.1 profile: {unsupported:?}"
        ));
    }
    for (name, expected) in [
        ("HEADING", 1),
        ("NODE", 1),
        ("ELEMENT", 1),
        ("NSET", 2),
        ("MATERIAL", 1),
        ("ELASTIC", 1),
        ("SOLID SECTION", 1),
        ("BOUNDARY", 1),
        ("STEP", 1),
        ("STATIC", 1),
        ("CLOAD", 1),
        ("NODE PRINT", 1),
        ("END STEP", 1),
    ] {
        if cards.iter().filter(|card| card.name == name).count() != expected {
            return Err(format!(
                "v0.1 deck requires exactly {expected} {name} card(s)"
            ));
        }
    }

    // A strict order keeps this small qualified profile from accidentally
    // interpreting a different CalculiX step/card scope as the same problem.
    let expected_order = [
        "HEADING",
        "NODE",
        "ELEMENT",
        "NSET",
        "NSET",
        "MATERIAL",
        "ELASTIC",
        "SOLID SECTION",
        "BOUNDARY",
        "STEP",
        "STATIC",
        "CLOAD",
        "NODE PRINT",
        "END STEP",
    ];
    let observed_order: Vec<&str> = cards.iter().map(|card| card.name.as_str()).collect();
    if observed_order != expected_order {
        return Err("deck cards are not in the exact v0.1 linear-static order".to_owned());
    }

    for card in &cards {
        let expected_params: BTreeSet<&str> = match card.name.as_str() {
            "ELEMENT" => ["TYPE", "ELSET"].into_iter().collect(),
            "NSET" => ["NSET"].into_iter().collect(),
            "MATERIAL" => ["NAME"].into_iter().collect(),
            "SOLID SECTION" => ["ELSET", "MATERIAL"].into_iter().collect(),
            "NODE PRINT" => ["NSET"].into_iter().collect(),
            _ => BTreeSet::new(),
        };
        let observed_params: BTreeSet<&str> = card.params.keys().map(String::as_str).collect();
        if observed_params != expected_params {
            return Err(format!(
                "card {} has unsupported or missing parameters: expected {expected_params:?}, observed {observed_params:?}",
                card.name
            ));
        }
    }

    let mut nodes = BTreeMap::new();
    for (line_no, raw) in &cards.iter().find(|card| card.name == "NODE").unwrap().rows {
        let fields = csv_fields(raw);
        if fields.len() != 4 {
            return Err(format!("NODE row at line {line_no} requires ID,x,y,z"));
        }
        let id = parse_u64(fields[0], *line_no, "NODE ID")?;
        let pos = [
            parse_finite(fields[1], *line_no)?,
            parse_finite(fields[2], *line_no)?,
            parse_finite(fields[3], *line_no)?,
        ];
        if nodes.insert(id, pos).is_some() {
            return Err(format!("duplicate NODE ID {id}"));
        }
    }
    if nodes.len() < 2 || nodes.len() > 100_000 {
        return Err("NODE inventory outside the v0.1 size budget".to_owned());
    }

    let element_card = cards.iter().find(|card| card.name == "ELEMENT").unwrap();
    if element_card.params.get("TYPE").map(String::as_str) != Some("T3D2")
        || element_card.params.get("ELSET").map(String::as_str) != Some("BAR")
    {
        return Err("v0.1 requires T3D2 elements in ELSET=BAR".to_owned());
    }
    let mut elements = Vec::new();
    for (line_no, raw) in &element_card.rows {
        let fields = csv_fields(raw);
        if fields.len() != 3 {
            return Err(format!(
                "ELEMENT row at line {line_no} requires ID,node1,node2"
            ));
        }
        let _element_id = parse_u64(fields[0], *line_no, "ELEMENT ID")?;
        let a = parse_u64(fields[1], *line_no, "ELEMENT node ID")?;
        let b = parse_u64(fields[2], *line_no, "ELEMENT node ID")?;
        if a == b || !nodes.contains_key(&a) || !nodes.contains_key(&b) {
            return Err(format!(
                "ELEMENT row at line {line_no} has invalid node references"
            ));
        }
        elements.push([a, b]);
    }
    if elements.is_empty() {
        return Err("ELEMENT inventory must not be empty".to_owned());
    }

    let mut node_sets = BTreeMap::<String, BTreeSet<u64>>::new();
    for card in cards.iter().filter(|card| card.name == "NSET") {
        let name = card
            .params
            .get("NSET")
            .ok_or_else(|| "NSET card is missing NSET= name".to_owned())?
            .to_ascii_uppercase();
        if node_sets.contains_key(&name) {
            return Err(format!("duplicate NSET definition '{name}'"));
        }
        let mut set = BTreeSet::new();
        for (line_no, raw) in &card.rows {
            for field in csv_fields(raw) {
                let id = parse_u64(field, *line_no, "NSET node ID")?;
                if !nodes.contains_key(&id) || !set.insert(id) {
                    return Err(format!(
                        "NSET {name} contains an absent or duplicate node ID {id}"
                    ));
                }
            }
        }
        node_sets.insert(name, set);
    }
    if node_sets.get("SUPPORT") != Some(&[support_node_id].into_iter().collect())
        || node_sets.get(expected_node_set) != Some(&[load_node_id].into_iter().collect())
    {
        return Err(
            "SUPPORT and tip NSET inventories must match the exact declared endpoint nodes"
                .to_owned(),
        );
    }

    let material_card = cards.iter().find(|card| card.name == "MATERIAL").unwrap();
    if material_card.params.get("NAME").map(String::as_str) != Some("STEEL")
        || !material_card.rows.is_empty()
    {
        return Err("v0.1 requires *MATERIAL,NAME=STEEL with no data rows".to_owned());
    }
    let elastic_card = cards.iter().find(|card| card.name == "ELASTIC").unwrap();
    if elastic_card.rows.len() != 1 {
        return Err("ELASTIC must contain exactly one data row".to_owned());
    }
    let elastic = csv_fields(&elastic_card.rows[0].1);
    if elastic.len() != 2 {
        return Err("ELASTIC row requires Young's modulus and Poisson ratio".to_owned());
    }
    let youngs_modulus_pa = parse_finite(elastic[0], elastic_card.rows[0].0)?;
    let poisson_ratio = parse_finite(elastic[1], elastic_card.rows[0].0)?;
    if youngs_modulus_pa <= 0.0 || !(0.0..0.5).contains(&poisson_ratio) {
        return Err("ELASTIC constants are outside the v0.1 envelope".to_owned());
    }

    let section_card = cards
        .iter()
        .find(|card| card.name == "SOLID SECTION")
        .unwrap();
    if section_card.params.get("ELSET").map(String::as_str) != Some("BAR")
        || section_card.params.get("MATERIAL").map(String::as_str) != Some("STEEL")
        || section_card.rows.len() != 1
    {
        return Err("v0.1 requires one BAR SOLID SECTION using STEEL".to_owned());
    }
    let section_values = csv_fields(&section_card.rows[0].1);
    if section_values.len() != 1 {
        return Err("SOLID SECTION data must contain only the axial area".to_owned());
    }
    let area_m2 = parse_finite(section_values[0], section_card.rows[0].0)?;
    if area_m2 <= 0.0 {
        return Err("SOLID SECTION area must be positive".to_owned());
    }

    let mut boundary_dofs = BTreeSet::new();
    let boundary_card = cards.iter().find(|card| card.name == "BOUNDARY").unwrap();
    for (line_no, raw) in &boundary_card.rows {
        let fields = csv_fields(raw);
        if fields.len() != 4 {
            return Err(format!(
                "BOUNDARY row at line {line_no} requires node,first_dof,last_dof,value"
            ));
        }
        let node = parse_u64(fields[0], *line_no, "BOUNDARY node ID")?;
        let first = parse_u64(fields[1], *line_no, "BOUNDARY first DOF")?;
        let last = parse_u64(fields[2], *line_no, "BOUNDARY last DOF")?;
        let value = parse_finite(fields[3], *line_no)?;
        if !nodes.contains_key(&node) || first < 1 || last > 3 || first > last || value != 0.0 {
            return Err(format!(
                "BOUNDARY row at line {line_no} exceeds the v0.1 zero-displacement profile"
            ));
        }
        for dof in first..=last {
            if !boundary_dofs.insert((node, dof as u8)) {
                return Err(format!(
                    "duplicate BOUNDARY condition for node {node}, DOF {dof}"
                ));
            }
        }
    }
    let expected_boundary: BTreeSet<(u64, u8)> = nodes
        .keys()
        .flat_map(|node| {
            let dofs: Vec<u8> = if *node == support_node_id {
                vec![1, 2, 3]
            } else {
                vec![2, 3]
            };
            dofs.into_iter().map(|dof| (*node, dof)).collect::<Vec<_>>()
        })
        .collect();
    if boundary_dofs != expected_boundary {
        return Err(
            "boundary inventory must pin all support DOFs and y/z only at every other node"
                .to_owned(),
        );
    }

    let cload_card = cards.iter().find(|card| card.name == "CLOAD").unwrap();
    let mut point_loads = Vec::new();
    for (line_no, raw) in &cload_card.rows {
        let fields = csv_fields(raw);
        if fields.len() != 3 {
            return Err(format!(
                "CLOAD row at line {line_no} requires node,DOF,value"
            ));
        }
        point_loads.push((
            parse_u64(fields[0], *line_no, "CLOAD node ID")?,
            parse_u64(fields[1], *line_no, "CLOAD DOF")? as u8,
            parse_finite(fields[2], *line_no)?,
        ));
    }
    if point_loads.len() != 1 || point_loads[0].0 != load_node_id || point_loads[0].1 != 1 {
        return Err(
            "v0.1 requires a single global-x point force at the declared tip node".to_owned(),
        );
    }

    let static_card = cards.iter().find(|card| card.name == "STATIC").unwrap();
    if static_card.rows.len() != 1 {
        return Err("STATIC card must have exactly one load increment row".to_owned());
    }
    let increments = csv_fields(&static_card.rows[0].1);
    if increments.len() != 4 {
        return Err(
            "STATIC increment row must contain initial, period, minimum, and maximum time values"
                .to_owned(),
        );
    }
    let initial_increment = parse_finite(increments[0], static_card.rows[0].0)?;
    let time_period = parse_finite(increments[1], static_card.rows[0].0)?;
    let minimum_increment = parse_finite(increments[2], static_card.rows[0].0)?;
    let maximum_increment = parse_finite(increments[3], static_card.rows[0].0)?;
    if initial_increment <= 0.0
        || time_period <= 0.0
        || minimum_increment <= 0.0
        || maximum_increment <= 0.0
        || minimum_increment > initial_increment
        || initial_increment > maximum_increment
        || time_period < initial_increment
        || !approx_eq(
            time_period,
            expected_time,
            1.0e-12 * expected_time.abs().max(1.0),
        )
    {
        return Err("STATIC time controls must be finite, positive, ordered, and match manifest.target_time".to_owned());
    }

    let node_print = cards.iter().find(|card| card.name == "NODE PRINT").unwrap();
    let printed_set = node_print
        .params
        .get("NSET")
        .ok_or_else(|| "NODE PRINT is missing NSET=".to_owned())?
        .to_ascii_uppercase();
    if node_print.rows.len() != 1 || node_print.rows[0].1.trim().to_ascii_uppercase() != "U" {
        return Err(
            "displacement deck must request exactly U from a single NODE PRINT set".to_owned(),
        );
    }

    // Verify the T3D2 connectivity is a chain in x order, with no duplicate edges.
    let mut sorted_nodes: Vec<(f64, u64)> = nodes.iter().map(|(id, p)| (p[0], *id)).collect();
    sorted_nodes.sort_by(|a, b| a.0.total_cmp(&b.0));
    for pair in sorted_nodes.windows(2) {
        if pair[1].0 <= pair[0].0 {
            return Err("bar nodes must have strictly increasing x coordinates".to_owned());
        }
    }
    for (_, position) in nodes.iter() {
        if !approx_eq(position[1], 0.0, 1.0e-12) || !approx_eq(position[2], 0.0, 1.0e-12) {
            return Err("v0.1 only supports nodes on the global x axis".to_owned());
        }
    }
    let expected_edges: BTreeSet<(u64, u64)> = sorted_nodes
        .windows(2)
        .map(|pair| {
            let (a, b) = (pair[0].1, pair[1].1);
            if a < b { (a, b) } else { (b, a) }
        })
        .collect();
    let actual_edges: BTreeSet<(u64, u64)> = elements
        .iter()
        .map(|edge| {
            if edge[0] < edge[1] {
                (edge[0], edge[1])
            } else {
                (edge[1], edge[0])
            }
        })
        .collect();
    if actual_edges != expected_edges || elements.len() != expected_edges.len() {
        return Err("T3D2 element connectivity must form one non-branching axial chain".to_owned());
    }
    if sorted_nodes.first().map(|n| n.1) != Some(support_node_id)
        || sorted_nodes.last().map(|n| n.1) != Some(load_node_id)
    {
        return Err("support/tip IDs must be the first/last nodes in the axial chain".to_owned());
    }

    Ok(ParsedDeck {
        nodes,
        elements,
        node_sets,
        point_loads,
        youngs_modulus_pa,
        poisson_ratio,
        area_m2,
        printed_set,
    })
}

fn parse_cards(text: &str) -> Check<Vec<DeckCard>> {
    let mut cards = Vec::<DeckCard>::new();
    let mut current: Option<DeckCard> = None;
    for (line_index, raw) in text.lines().enumerate() {
        let line_no = line_index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with("**") {
            continue;
        }
        if line.starts_with('*') {
            if let Some(card) = current.take() {
                cards.push(card);
            }
            let parts: Vec<&str> = line[1..].split(',').collect();
            let name = parts
                .first()
                .copied()
                .unwrap_or_default()
                .trim()
                .to_ascii_uppercase();
            if name.is_empty() {
                return Err(format!("empty keyword card at line {line_no}"));
            }
            let mut params = BTreeMap::new();
            for part in parts.iter().skip(1) {
                let Some((key, value)) = part.split_once('=') else {
                    return Err(format!("malformed card parameter at line {line_no}"));
                };
                let key = key.trim().to_ascii_uppercase();
                let value = value.trim().to_ascii_uppercase();
                if key.is_empty() || value.is_empty() || params.insert(key.clone(), value).is_some()
                {
                    return Err(format!(
                        "empty or duplicate card parameter '{key}' at line {line_no}"
                    ));
                }
            }
            current = Some(DeckCard {
                name,
                params,
                rows: Vec::new(),
            });
        } else {
            let card = current
                .as_mut()
                .ok_or_else(|| format!("data row outside a keyword card at line {line_no}"))?;
            card.rows.push((line_no, line.to_owned()));
        }
    }
    if let Some(card) = current {
        cards.push(card);
    }
    Ok(cards)
}

fn parse_displacement_section(
    text: &str,
    expected_set: &str,
    expected_node_id: u64,
) -> Check<(String, f64, DisplacementRow)> {
    let lines: Vec<&str> = text.lines().collect();
    let headers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, raw)| {
            raw.trim()
                .to_ascii_lowercase()
                .starts_with(DAT_HEADER_PREFIX)
                .then_some(index)
        })
        .collect();
    if headers.len() != 1 {
        return Err(format!(
            "expected exactly one displacement U section, found {}",
            headers.len()
        ));
    }
    let header_index = headers[0];
    let header = lines[header_index].trim();
    let rest = &header[DAT_HEADER_PREFIX.len()..];
    let marker = " and time ";
    let marker_index = rest
        .to_ascii_lowercase()
        .find(marker)
        .ok_or_else(|| "displacement header is missing the time field".to_owned())?;
    let observed_set = rest[..marker_index].trim().to_owned();
    validate_displacement_set_token(&observed_set, "observed node set")?;
    if !observed_set.eq_ignore_ascii_case(expected_set) {
        return Err(format!(
            "observed displacement set '{observed_set}' does not match '{expected_set}'"
        ));
    }
    let time_text = rest[marker_index + marker.len()..].trim();
    let observed_time = parse_finite(time_text, header_index + 1)?;

    let mut rows = Vec::new();
    let mut saw_data = false;
    for (index, raw) in lines.iter().enumerate().skip(header_index + 1) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            if saw_data {
                break;
            }
            continue;
        }
        let fields = whitespace_fields(trimmed);
        if fields.is_empty() {
            continue;
        }
        let node_id = match fields[0].parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                if looks_numeric(fields[0]) {
                    return Err(format!(
                        "malformed displacement node ID at .dat line {}",
                        index + 1
                    ));
                }
                if saw_data {
                    break;
                }
                return Err(format!(
                    "unexpected non-data line after displacement header at .dat line {}",
                    index + 1
                ));
            }
        };
        if fields.len() != 4 {
            return Err(format!(
                "displacement row at .dat line {} requires node ID and three components",
                index + 1
            ));
        }
        let displacement_m = [
            parse_finite(fields[1], index + 1)?,
            parse_finite(fields[2], index + 1)?,
            parse_finite(fields[3], index + 1)?,
        ];
        rows.push(DisplacementRow {
            node_id,
            displacement_m,
            source_line: index + 1,
            raw_line: trimmed.to_owned(),
        });
        saw_data = true;
    }

    if rows.len() != 1 || rows[0].node_id != expected_node_id {
        return Err(format!(
            "displacement section must contain exactly the expected node {expected_node_id}; observed node IDs {:?}",
            rows.iter().map(|row| row.node_id).collect::<Vec<_>>()
        ));
    }
    Ok((observed_set, observed_time, rows.remove(0)))
}

fn validate_provenance(value: &Value) -> Check<()> {
    exact_keys(
        value,
        &["evidence_class", "method", "source_ref"],
        "provenance",
    )?;
    if field_string(value, "evidence_class", "provenance")? != "derived_analytical"
        || field_string(value, "method", "provenance")? != "linear_axial_bar_u_FL_EA"
    {
        return Err("provenance must identify the derived analytical u=FL/(EA) oracle".to_owned());
    }
    token(
        field_string(value, "source_ref", "provenance")?,
        "provenance.source_ref",
    )?;
    Ok(())
}

fn parse_object(bytes: &[u8], label: &str) -> Check<Value> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("{label} JSON is invalid: {error}"))?;
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be a JSON object"))?;
    Ok(value)
}

fn exact_keys(value: &Value, expected: &[&str], label: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    let observed: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if observed != expected {
        return Err(format!(
            "{label} keys differ: expected {expected:?}, observed {observed:?}"
        ));
    }
    Ok(())
}

fn field_string<'a>(value: &'a Value, key: &str, label: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.{key} must be a string"))
}

fn field_sha<'a>(value: &'a Value, key: &str, label: &str) -> Check<&'a str> {
    let text = field_string(value, key, label)?;
    validate_hex(text, 64, &format!("{label}.{key}"))?;
    Ok(text)
}

fn field_u64(value: &Value, key: &str, label: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label}.{key} must be an unsigned integer"))
}

fn field_f64(value: &Value, key: &str, label: &str) -> Check<f64> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{label}.{key} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{label}.{key} must be finite"));
    }
    Ok(number)
}

fn field_positive_f64(value: &Value, key: &str, label: &str) -> Check<f64> {
    let result = field_f64(value, key, label)?;
    if result <= 0.0 {
        return Err(format!("{label}.{key} must be positive"));
    }
    Ok(result)
}

fn field_nonnegative_f64(value: &Value, key: &str, label: &str) -> Check<f64> {
    let result = field_f64(value, key, label)?;
    if result < 0.0 {
        return Err(format!("{label}.{key} must be non-negative"));
    }
    Ok(result)
}

fn array_field<'a>(value: &'a Value, key: &str, label: &str) -> Check<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.{key} must be an array"))
}

fn finite_number(value: &Value, label: &str) -> Check<f64> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{label} must be a numeric JSON value"))?;
    if !number.is_finite() {
        return Err(format!("{label} must be finite"));
    }
    Ok(number)
}

fn validate_hex(value: &str, length: usize, label: &str) -> Check<()> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!(
            "{label} must be {length} lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn token<'a>(value: &'a str, label: &str) -> Check<&'a str> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
    {
        return Err(format!("{label} is not a valid token"));
    }
    Ok(value)
}

fn validate_displacement_set_token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(format!("{label} is not a valid stable token"));
    }
    Ok(())
}

fn csv_fields(text: &str) -> Vec<&str> {
    text.split(',').map(str::trim).collect()
}

fn whitespace_fields(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

fn looks_numeric(token: &str) -> bool {
    token
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'+' || *byte == b'-')
}

fn parse_u64(text: &str, line: usize, label: &str) -> Check<u64> {
    text.trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid {label} at line {line}"))
}

fn parse_finite(text: &str, line: usize) -> Check<f64> {
    let normalized = text.trim().replace('D', "E").replace('d', "E");
    let value = normalized
        .parse::<f64>()
        .map_err(|_| format!("invalid numeric value at line {line}"))?;
    if !value.is_finite() {
        return Err(format!("non-finite numeric value at line {line}"));
    }
    Ok(value)
}

fn approx_eq(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance.max(f64::EPSILON * a.abs().max(b.abs()).max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: f64 = 210_000_000_000.0;
    const A: f64 = 0.01;
    const F: f64 = 1000.0;
    const U: f64 = F / (E * A);

    fn request() -> Vec<u8> {
        format!(
            r#"{{
              "schema_version":"physical-solver-request-v0.1",
              "request_id":"ccx-exec-001:bar",
              "source_revision":"556e78c6e93a63b1b734c9d1b9d3f91cdd7223ac",
              "scene_revision":1,
              "scene_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
              "model_fidelity":"analytical_linear_elastic",
              "requested_physics":["static_linear_elastic"],
              "coordinate_frame_id":"fixture:global-xyz",
              "units":{{"length":"m","mass":"kg","time":"s","force":"N","stress":"Pa"}},
              "materials":[{{"material_id":"steel","component_ids":["bar"],"youngs_modulus_pa":{E},"poisson_ratio":0.3,"density_kg_m3":7850,
                "provenance":{{"evidence_class":"synthetic","source_ref":"fixture-only","uncertainty_relative":null}}}}],
              "loads":[{{"load_id":"axial-force","kind":"point_force","target_id":"tip","vector_xyz":[{F},0,0],"coordinate_frame_id":"fixture:global-xyz",
                "provenance":{{"evidence_class":"synthetic","source_ref":"fixture-only","uncertainty_relative":null}}}}],
              "boundary_conditions":[
                {{"condition_id":"fixed-support","target_id":"support","kind":"fixed","dofs":["ux","uy","uz"],
                  "provenance":{{"evidence_class":"synthetic","source_ref":"fixture-only","uncertainty_relative":null}}}},
                {{"condition_id":"tip-lateral-guide","target_id":"tip","kind":"fixed","dofs":["uy","uz"],
                  "provenance":{{"evidence_class":"synthetic","source_ref":"fixture-only","uncertainty_relative":null}}}}
              ],
              "required_outputs":["displacement","reaction_force"],
              "acceptance":{{"relative_tolerance":1e-6,"absolute_tolerance":5e-13,"require_convergence":true,"max_iterations":1}}
            }}"#
        )
        .into_bytes()
    }

    fn deck() -> Vec<u8> {
        format!(
            "*HEADING
pinned one-element axial bar
*NODE
1,0,0,0
2,1,0,0
*ELEMENT,TYPE=T3D2,ELSET=BAR
1,1,2
*NSET,NSET=SUPPORT
1
*NSET,NSET=TIP
2
*MATERIAL,NAME=STEEL
*ELASTIC
{E},0.3
*SOLID SECTION,ELSET=BAR,MATERIAL=STEEL
{A}
*BOUNDARY
1,1,3,0
2,2,3,0
*STEP
*STATIC
1.,1.,1.E-05,1.
*CLOAD
2,1,{F}
*NODE PRINT,NSET=TIP
U
*END STEP
"
        )
        .into_bytes()
    }

    fn dat_output(u: f64) -> Vec<u8> {
        format!(
            "displacements (vx,vy,vz) for set TIP and time 0.1000000E+01
2 {u:.6E} 0.000000E+00 0.000000E+00

"
        )
        .into_bytes()
    }

    fn build_manifest() -> Vec<u8> {
        br#"{
          "schema_version":"calculix-build-manifest-v0.1",
          "builder_repository":"OmnibusCloud/CalculiX",
          "builder_repository_commit":"b7977a4e90b40fd28fa149fc69f1647661d10f56",
          "container_image":"ubuntu@sha256:2e863c44b718727c860746568e1d54afd13b2fa71b160f5cd9058fc436217b30",
          "ccx_source_url":"http://www.dhondt.de/ccx_2.22.src.tar.bz2",
          "ccx_source_sha256":"3a94dcc775a31f570229734b341d6b06301ebdc759863df901c8b9bf1854c0bc",
          "spooles_sha256":"a84559a0e987a1e423055ef4fdf3035d55b65bbe4bf915efaa1a35bef7f8c5dd",
          "arpack_enabled":false,
          "ccx_executable_sha256":"1111111111111111111111111111111111111111111111111111111111111111",
          "toolchain_manifest_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
          "build_log_sha256":"3333333333333333333333333333333333333333333333333333333333333333",
          "build_config":{"with_pardiso":false,"with_pastix":false,"with_arpack":false,"with_mt":true}
        }"#
        .to_vec()
    }

    fn manifest_bytes(request: &[u8], deck: &[u8], dat: &[u8], build: &[u8]) -> Vec<u8> {
        let req: Value = serde_json::from_slice(request).unwrap();
        let source_ref = "research/physical-solver-contract/PHYS-CCX-EXEC-001";
        let value = json!({
          "schema_version": MANIFEST_SCHEMA,
          "case_id":"CCX-EXEC-001-AXIAL-BAR",
          "request_id": req["request_id"],
          "request_sha256":sha256_hex(request),
          "scene_sha256":req["scene_sha256"],
          "scene_revision":req["scene_revision"],
          "coordinate_frame_id":req["coordinate_frame_id"],
          "deck_sha256":sha256_hex(deck),
          "dat_sha256":sha256_hex(dat),
          "build_manifest_sha256":sha256_hex(build),
          "solver_executable_sha256":"1111111111111111111111111111111111111111111111111111111111111111",
          "support_target_id":"support",
          "load_target_id":"tip",
          "support_node_id":1,
          "load_node_id":2,
          "node_set":"TIP",
          "component":"ux",
          "absolute_tolerance_m":5e-13,
          "relative_tolerance":1e-6,
          "target_time":1.0,
          "model_nodes":[
            {"node_id":1,"target_id":"support","position_m":[0.0,0.0,0.0]},
            {"node_id":2,"target_id":"tip","position_m":[1.0,0.0,0.0]}
          ],
          "provenance":{
            "evidence_class":"derived_analytical",
            "method":"linear_axial_bar_u_FL_EA",
            "source_ref":source_ref
          }
        });
        serde_json::to_vec(&value).unwrap()
    }

    #[test]
    fn exact_axial_bar_displacement_matches_independent_closed_form() {
        let request = request();
        let deck = deck();
        let dat = dat_output(U);
        let build = build_manifest();
        let manifest = manifest_bytes(&request, &deck, &dat, &build);
        let report =
            compare_displacement_reference_bytes(&request, &deck, &dat, &manifest, &build).unwrap();
        assert_eq!(report["numeric_reference_status"], "PASS");
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
        assert!((report["reference_displacement_m"].as_f64().unwrap() - U).abs() < 1e-20);
    }

    #[test]
    fn changed_deck_or_output_bytes_reject_stale_manifest() {
        let request = request();
        let deck = deck();
        let dat = dat_output(U);
        let build = build_manifest();
        let manifest = manifest_bytes(&request, &deck, &dat, &build);
        let mut changed_deck = deck.clone();
        changed_deck.extend_from_slice(b"** changed\n");
        assert!(
            compare_displacement_reference_bytes(&request, &changed_deck, &dat, &manifest, &build)
                .is_err()
        );
        let changed_dat = dat_output(U * 2.0);
        assert!(
            compare_displacement_reference_bytes(&request, &deck, &changed_dat, &manifest, &build)
                .is_err()
        );
    }

    #[test]
    fn missing_wrong_node_nonfinite_and_duplicate_output_fail_closed() {
        let request = request();
        let deck = deck();
        let build = build_manifest();
        for dat in [
            b"ordinary forces (fx,fy,fz) for set TIP and time 1.0\n2 1 0 0\n".to_vec(),
            b"displacements (vx,vy,vz) for set TIP and time 1.0\n1 1 0 0\n\n".to_vec(),
            b"displacements (vx,vy,vz) for set TIP and time 1.0\n2 NaN 0 0\n\n".to_vec(),
            b"displacements (vx,vy,vz) for set TIP and time 1.0\n2 1 0 0\n2 1 0 0\n\n".to_vec(),
            b"displacements (vx,vy,vz) for set TIP and time 1.0\n2 1 0 0\n\n displacements (vx,vy,vz) for set TIP and time 1.0\n2 1 0 0\n".to_vec(),
        ] {
            let manifest = manifest_bytes(&request, &deck, &dat, &build);
            assert!(compare_displacement_reference_bytes(&request, &deck, &dat, &manifest, &build).is_err());
        }
    }

    #[test]
    fn a_numeric_mismatch_is_reported_as_fail_not_pass() {
        let request = request();
        let deck = deck();
        let dat = dat_output(U * 2.0);
        let build = build_manifest();
        let manifest = manifest_bytes(&request, &deck, &dat, &build);
        let report =
            compare_displacement_reference_bytes(&request, &deck, &dat, &manifest, &build).unwrap();
        assert_eq!(report["numeric_reference_status"], "FAIL");
        assert_eq!(report["physical_result_qualified"], false);
    }

    #[test]
    fn unknown_deck_keywords_and_wrong_build_identity_are_rejected() {
        let request = request();
        let deck = deck();
        let dat = dat_output(U);
        let build = build_manifest();
        let manifest = manifest_bytes(&request, &deck, &dat, &build);
        let mut unsupported_deck = deck.clone();
        unsupported_deck.extend_from_slice(b"*DLOAD\nBAR,P,1.0\n");
        let unsupported_manifest = manifest_bytes(&request, &unsupported_deck, &dat, &build);
        assert!(
            compare_displacement_reference_bytes(
                &request,
                &unsupported_deck,
                &dat,
                &unsupported_manifest,
                &build
            )
            .is_err()
        );

        // Empty fields are malformed input, not separators to silently discard.
        let malformed_deck_text = String::from_utf8(deck.clone())
            .unwrap()
            .replace("1,1,2\n", "1,,2\n");
        let malformed_deck = malformed_deck_text.into_bytes();
        let malformed_manifest = manifest_bytes(&request, &malformed_deck, &dat, &build);
        let error = compare_displacement_reference_bytes(
            &request,
            &malformed_deck,
            &dat,
            &malformed_manifest,
            &build,
        )
        .unwrap_err();
        assert!(error.contains("invalid ELEMENT node ID"), "{error}");

        let changed_step_text = String::from_utf8(deck.clone())
            .unwrap()
            .replace("*STEP\n", "*STEP,NLGEOM=YES\n");
        let changed_step = changed_step_text.into_bytes();
        let changed_step_manifest = manifest_bytes(&request, &changed_step, &dat, &build);
        let step_error = compare_displacement_reference_bytes(
            &request,
            &changed_step,
            &dat,
            &changed_step_manifest,
            &build,
        )
        .unwrap_err();
        assert!(
            step_error.contains("unsupported or missing parameters"),
            "{step_error}"
        );

        let changed_time_text = String::from_utf8(deck.clone())
            .unwrap()
            .replace("1.,1.,1.E-05,1.\n", "1.,2.,1.E-05,1.\n");
        let changed_time = changed_time_text.into_bytes();
        let changed_time_manifest = manifest_bytes(&request, &changed_time, &dat, &build);
        let time_error = compare_displacement_reference_bytes(
            &request,
            &changed_time,
            &dat,
            &changed_time_manifest,
            &build,
        )
        .unwrap_err();
        assert!(
            time_error.contains("match manifest.target_time"),
            "{time_error}"
        );

        let wrong_build = build_manifest().into_iter().collect::<Vec<_>>();
        let mut changed_build: Value = serde_json::from_slice(&wrong_build).unwrap();
        changed_build["builder_repository_commit"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let changed_build = serde_json::to_vec(&changed_build).unwrap();
        let manifest = manifest_bytes(&request, &deck, &dat, &changed_build);
        assert!(
            compare_displacement_reference_bytes(&request, &deck, &dat, &manifest, &changed_build)
                .is_err()
        );
    }
}
