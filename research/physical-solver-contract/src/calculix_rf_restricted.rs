// SPDX-License-Identifier: AGPL-3.0-or-later
//! Restricted proof that CalculiX RF can be interpreted as support reaction.
//!
//! This path is intentionally separate from the patched-CCX RR parser. RF is normally
//! an external-force quantity (reaction + applied concentrated/distributed loads).
//! This module accepts only an allowlisted static linear truss deck with point loads
//! disjoint from the prescribed-support nodes and no distributed or transformed loads.

use crate::{Check, sha256_hex};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const MAX_DECK_BYTES: usize = 16 * 1024 * 1024;
const MAX_DAT_BYTES: usize = 32 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;
const MANIFEST_SCHEMA: &str = "calculix-rf-restricted-proof-manifest-v0.1";
const CATALOG_SCHEMA: &str = "physical-support-node-catalog-v0.1";
const RF_HEADER_PREFIX: &str = "forces (fx,fy,fz) for set ";

#[derive(Clone, Debug)]
struct Card {
    name: String,
    params: BTreeMap<String, String>,
    data: Vec<(usize, String)>,
}

#[derive(Clone, Debug)]
struct ParsedDeck {
    nodes: BTreeMap<u64, [f64; 3]>,
    elements: BTreeMap<u64, [u64; 2]>,
    node_sets: BTreeMap<String, BTreeSet<u64>>,
    boundary_dofs: BTreeSet<(u64, u8)>,
    cloads: BTreeMap<(u64, u8), f64>,
    node_print_set: String,
    elastic_modulus_pa: f64,
    poisson_ratio: f64,
    section_area_m2: f64,
}

#[derive(Clone, Debug)]
struct LoadNodeMap {
    target_id: String,
    node_id: u64,
}

#[derive(Clone, Debug)]
struct SupportNode {
    node_id: u64,
    position_m: [f64; 3],
}

#[derive(Clone, Debug)]
struct RfRow {
    node_id: u64,
    force_n: [f64; 3],
    source_line: usize,
    raw_line: String,
}

struct SupportCatalogContext<'a> {
    case_id: &'a str,
    request_id: &'a str,
    request_sha256: &'a str,
    scene_sha256: &'a str,
    scene_revision: u64,
    coordinate_frame_id: &'a str,
    deck_sha256: &'a str,
}

/// Prove the restricted stock-CCX RF equivalence conditions, then normalize one exact RF section.
///
/// This function never treats RF as reactions in general. It accepts only a deliberately
/// small allowlisted linear-static T3D2 deck. All unknown keywords, load types, frame
/// transforms, multi-step semantics, local support coordinates and ambiguous output sections
/// are rejected. The independent manifest maps request target IDs to exact deck node IDs.
pub fn normalize_restricted_rf_bytes(
    request_bytes: &[u8],
    deck_bytes: &[u8],
    dat_bytes: &[u8],
    manifest_bytes: &[u8],
    node_catalog_bytes: &[u8],
) -> Check<Value> {
    for (name, bytes, limit) in [
        ("request", request_bytes, MAX_JSON_BYTES),
        ("manifest", manifest_bytes, MAX_JSON_BYTES),
        ("node catalogue", node_catalog_bytes, MAX_JSON_BYTES),
    ] {
        if bytes.is_empty() || bytes.len() > limit {
            return Err(format!(
                "{name} JSON is empty or exceeds the {limit}-byte budget"
            ));
        }
    }
    if deck_bytes.is_empty() || deck_bytes.len() > MAX_DECK_BYTES {
        return Err("CalculiX input deck is empty or exceeds 16 MiB".to_owned());
    }
    if dat_bytes.is_empty() || dat_bytes.len() > MAX_DAT_BYTES {
        return Err("CalculiX .dat output is empty or exceeds 32 MiB".to_owned());
    }

    let request_sha256 = sha256_hex(request_bytes);
    let deck_sha256 = sha256_hex(deck_bytes);
    let dat_sha256 = sha256_hex(dat_bytes);
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let node_catalog_sha256 = sha256_hex(node_catalog_bytes);
    let request = parse_json(request_bytes, "request")?;
    let manifest = parse_json(manifest_bytes, "restricted-RF manifest")?;
    let catalog = parse_json(node_catalog_bytes, "support-node catalogue")?;

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
            "node_set",
            "deck_sha256",
            "node_catalog_sha256",
            "support_node_map",
            "load_node_map",
            "units",
            "provenance",
        ],
        "manifest",
    )?;
    if field_str(&manifest, "schema_version", "manifest")? != MANIFEST_SCHEMA {
        return Err(format!(
            "manifest.schema_version must equal {MANIFEST_SCHEMA}"
        ));
    }
    let case_id = token_field(&manifest, "case_id", "manifest")?;
    let request_id = field_str(&manifest, "request_id", "manifest")?.to_owned();
    let scene_sha256 = field_sha(&manifest, "scene_sha256", "manifest")?;
    let scene_revision = field_u64(&manifest, "scene_revision", "manifest")?;
    let coordinate_frame_id = token_field(&manifest, "coordinate_frame_id", "manifest")?;
    let node_set = token_field(&manifest, "node_set", "manifest")?;
    if field_sha(&manifest, "request_sha256", "manifest")? != request_sha256
        || field_sha(&manifest, "deck_sha256", "manifest")? != deck_sha256
        || field_sha(&manifest, "node_catalog_sha256", "manifest")? != node_catalog_sha256
    {
        return Err(
            "manifest request/deck/node-catalogue SHA-256 binding does not match exact bytes"
                .to_owned(),
        );
    }
    if field_str(&request, "request_id", "request")? != request_id
        || field_sha(&request, "scene_sha256", "request")? != scene_sha256
        || field_u64(&request, "scene_revision", "request")? != scene_revision
        || field_str(&request, "coordinate_frame_id", "request")? != coordinate_frame_id
    {
        return Err("manifest request/scene/frame identity differs from exact request".to_owned());
    }
    require_linear_static_request(&request)?;
    require_si_units(&request, "request")?;
    require_si_units(&manifest, "manifest")?;
    validate_provenance(
        manifest
            .get("provenance")
            .ok_or_else(|| "manifest.provenance is required".to_owned())?,
        "manifest.provenance",
    )?;

    let support_map = parse_node_map(&manifest, "support_node_map", "manifest")?;
    let load_map = parse_node_map(&manifest, "load_node_map", "manifest")?;
    if support_map.is_empty() || load_map.is_empty() {
        return Err("support_node_map and load_node_map must both be non-empty".to_owned());
    }
    ensure_unique_map(&support_map, "support_node_map")?;
    ensure_unique_map(&load_map, "load_node_map")?;

    let deck_text = std::str::from_utf8(deck_bytes)
        .map_err(|_| "CalculiX deck must be UTF-8/ASCII text".to_owned())?;
    let cards = parse_cards(deck_text)?;
    let parsed_deck = validate_and_parse_deck(&cards, &node_set)?;
    let support_ids: BTreeSet<u64> = support_map.iter().map(|map| map.node_id).collect();
    let load_ids: BTreeSet<u64> = load_map.iter().map(|map| map.node_id).collect();
    if support_ids.len() != support_map.len() || load_ids.len() != load_map.len() {
        return Err("support/load node maps contain repeated node IDs".to_owned());
    }
    if !support_ids.is_disjoint(&load_ids) {
        return Err("load-node inventory intersects prescribed-support inventory; RF cannot be treated as pure reaction".to_owned());
    }

    if parsed_deck.node_print_set != node_set {
        return Err("RF output request is not bound to the manifest support node set".to_owned());
    }
    let deck_support_set = parsed_deck
        .node_sets
        .get(&node_set)
        .ok_or_else(|| format!("deck has no explicit node set '{node_set}'"))?;
    if deck_support_set != &support_ids {
        return Err("deck support node set differs from manifest support_node_map".to_owned());
    }
    if parsed_deck
        .boundary_dofs
        .iter()
        .map(|(node, _)| *node)
        .collect::<BTreeSet<_>>()
        != support_ids
    {
        return Err(
            "all prescribed-boundary nodes must exactly match the declared support-node inventory"
                .to_owned(),
        );
    }
    let catalog_context = SupportCatalogContext {
        case_id: &case_id,
        request_id: &request_id,
        request_sha256: &request_sha256,
        scene_sha256,
        scene_revision,
        coordinate_frame_id: &coordinate_frame_id,
        deck_sha256: &deck_sha256,
    };
    let catalog_nodes = parse_support_catalog(&catalog, &catalog_context, &support_ids)?;
    for support in &catalog_nodes {
        let deck_position = parsed_deck.nodes.get(&support.node_id).ok_or_else(|| {
            format!(
                "support node {} does not occur in deck node inventory",
                support.node_id
            )
        })?;
        if deck_position != &support.position_m {
            return Err(format!(
                "support node {} coordinates differ between deck and independent catalogue",
                support.node_id
            ));
        }
    }

    let declared_support_map: BTreeMap<&str, u64> = support_map
        .iter()
        .map(|item| (item.target_id.as_str(), item.node_id))
        .collect();
    let declared_load_map: BTreeMap<&str, u64> = load_map
        .iter()
        .map(|item| (item.target_id.as_str(), item.node_id))
        .collect();
    // Boundary conditions may target a support node or a separately declared
    // load-end guide (for example, constraining only uy/uz on a 3D axial bar).
    // The load map still remains separate from the RF support-node inventory.
    let mut declared_boundary_map = declared_support_map.clone();
    for (target_id, node_id) in &declared_load_map {
        if let Some(existing) = declared_boundary_map.insert(*target_id, *node_id)
            && existing != *node_id
        {
            return Err(format!(
                "boundary target '{target_id}' resolves to conflicting node IDs {existing} and {node_id}"
            ));
        }
    }
    let expected_boundary = request_boundary_dofs(&request, &declared_boundary_map)?;
    if expected_boundary != parsed_deck.boundary_dofs {
        return Err("deck zero-displacement DOF inventory differs from the exact request boundary conditions".to_owned());
    }
    let expected_cloads = request_point_loads(&request, &declared_load_map, &parsed_deck.nodes)?;
    if expected_cloads != parsed_deck.cloads {
        return Err(format!(
            "deck CLOAD inventory does not exactly match request loads; expected={expected_cloads:?}, observed={:?}",
            parsed_deck.cloads
        ));
    }
    if parsed_deck
        .cloads
        .keys()
        .any(|(node, _)| support_ids.contains(node))
    {
        return Err(
            "a CLOAD is applied to a support node; RF would include that concentrated load"
                .to_owned(),
        );
    }

    let dat_text = std::str::from_utf8(dat_bytes)
        .map_err(|_| "CalculiX .dat output must be UTF-8/ASCII text".to_owned())?;
    let rf_rows = parse_rf_section(dat_text, &node_set, &support_ids)?;
    let positions: BTreeMap<u64, [f64; 3]> = catalog_nodes
        .iter()
        .map(|node| (node.node_id, node.position_m))
        .collect();
    let normalized_forces: Vec<Value> = rf_rows
        .iter()
        .map(|row| {
            json!({
                "reaction_id": format!("node-{}", row.node_id),
                "force_n": row.force_n,
                "position_m": positions[&row.node_id],
            })
        })
        .collect();
    let normalized = json!({
        "schema_version": "physical-equilibrium-reaction-forces-v0.1",
        "case_id": case_id,
        "request_id": request_id,
        "request_sha256": request_sha256,
        "scene_sha256": scene_sha256,
        "scene_revision": scene_revision,
        "coordinate_frame_id": coordinate_frame_id,
        "units": {"length":"m","force":"N"},
        "reaction_forces": normalized_forces
    });
    let normalized_bytes = serde_json::to_vec(&normalized)
        .map_err(|error| format!("normalized output JSON serialization failed: {error}"))?;
    Ok(json!({
        "schema_version": "calculix-rf-restricted-normalization-report-v0.1",
        "normalization_status": "PASS",
        "source_semantics": "RF_under_restricted_deck_equivalence",
        "restricted_equivalence_proved": true,
        "case_id": case_id,
        "request_id": request_id,
        "request_sha256": request_sha256,
        "scene_sha256": scene_sha256,
        "scene_revision": scene_revision,
        "coordinate_frame_id": coordinate_frame_id,
        "deck_sha256": deck_sha256,
        "dat_sha256": dat_sha256,
        "manifest_sha256": manifest_sha256,
        "node_catalog_sha256": node_catalog_sha256,
        "support_node_set": node_set,
        "support_node_ids": support_ids,
        "load_node_ids": load_ids,
        "mesh_element_count": parsed_deck.elements.len(),
        "material_youngs_modulus_pa": parsed_deck.elastic_modulus_pa,
        "material_poisson_ratio": parsed_deck.poisson_ratio,
        "section_area_m2": parsed_deck.section_area_m2,
        "rf_source_rows": rf_rows.iter().map(|row| json!({
            "node_id":row.node_id,
            "force_n":row.force_n,
            "source_line":row.source_line,
            "raw_line":row.raw_line
        })).collect::<Vec<_>>(),
        "normalized_output_sha256": sha256_hex(&normalized_bytes),
        "normalized_output": normalized,
        "rf_equals_support_reaction_proof": {
            "request_loads_match_exact_deck_cloads": true,
            "support_and_load_node_sets_disjoint": true,
            "no_distributed_or_other_load_cards": true,
            "no_local_coordinate_transform_or_system_card": true,
            "all_deck_cards_allowlisted": true,
            "support_catalog_matches_deck_coordinates": true,
            "output_set_and_node_inventory_exact": true
        },
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "claim_ceiling": "RF normalized as support reactions only after the narrow allowlisted-deck equivalence proof; no execution authentication, equilibrium PASS, physical validation or solver qualification"
    }))
}

fn parse_json(bytes: &[u8], label: &str) -> Check<Value> {
    serde_json::from_slice(bytes).map_err(|error| format!("invalid {label} JSON: {error}"))
}

fn require_linear_static_request(request: &Value) -> Check<()> {
    let fidelity = field_str(request, "model_fidelity", "request")?;
    if !["analytical_linear_elastic", "continuum_linear_elastic"].contains(&fidelity) {
        return Err("restricted RF path supports only linear-elastic request fidelity".to_owned());
    }
    let physics = field_string_array(request, "requested_physics", "request")?;
    if physics.as_slice() != ["static_linear_elastic"] {
        return Err(
            "restricted RF path requires static_linear_elastic as the sole physics".to_owned(),
        );
    }
    if !field_string_array(request, "required_outputs", "request")?
        .iter()
        .any(|value| value == "reaction_force")
    {
        return Err("request must explicitly require reaction_force".to_owned());
    }
    Ok(())
}

fn require_si_units(value: &Value, at: &str) -> Check<()> {
    let units = value
        .get("units")
        .ok_or_else(|| format!("{at}.units is required"))?;
    if field_str(units, "length", &format!("{at}.units"))? != "m"
        || field_str(units, "force", &format!("{at}.units"))? != "N"
    {
        return Err(format!("{at} units must be SI length=m and force=N"));
    }
    Ok(())
}

fn parse_node_map(value: &Value, key: &str, at: &str) -> Check<Vec<LoadNodeMap>> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?;
    let mut result = Vec::with_capacity(array.len());
    for (index, entry) in array.iter().enumerate() {
        let entry_at = format!("{at}.{key}[{index}]");
        exact_keys(entry, &["target_id", "node_id"], &entry_at)?;
        result.push(LoadNodeMap {
            target_id: token_field(entry, "target_id", &entry_at)?,
            node_id: field_u64(entry, "node_id", &entry_at)?,
        });
    }
    Ok(result)
}

fn ensure_unique_map(map: &[LoadNodeMap], at: &str) -> Check<()> {
    let mut targets = BTreeSet::new();
    let mut nodes = BTreeSet::new();
    for item in map {
        if !targets.insert(item.target_id.as_str()) {
            return Err(format!("{at} duplicates target ID '{}'", item.target_id));
        }
        if !nodes.insert(item.node_id) {
            return Err(format!(
                "{at} maps more than one target to node {}",
                item.node_id
            ));
        }
    }
    Ok(())
}

fn parse_support_catalog(
    catalog: &Value,
    context: &SupportCatalogContext<'_>,
    expected_support_ids: &BTreeSet<u64>,
) -> Check<Vec<SupportNode>> {
    exact_keys(
        catalog,
        &[
            "schema_version",
            "case_id",
            "request_id",
            "request_sha256",
            "scene_sha256",
            "scene_revision",
            "coordinate_frame_id",
            "deck_sha256",
            "units",
            "nodes",
            "provenance",
        ],
        "node catalogue",
    )?;
    if field_str(catalog, "schema_version", "node catalogue")? != CATALOG_SCHEMA
        || field_str(catalog, "case_id", "node catalogue")? != context.case_id
        || field_str(catalog, "request_id", "node catalogue")? != context.request_id
        || field_sha(catalog, "request_sha256", "node catalogue")? != context.request_sha256
        || field_sha(catalog, "scene_sha256", "node catalogue")? != context.scene_sha256
        || field_u64(catalog, "scene_revision", "node catalogue")? != context.scene_revision
        || field_str(catalog, "coordinate_frame_id", "node catalogue")?
            != context.coordinate_frame_id
        || field_sha(catalog, "deck_sha256", "node catalogue")? != context.deck_sha256
    {
        return Err("support-node catalogue identity/frame/deck digest mismatch".to_owned());
    }
    let catalog_units = catalog
        .get("units")
        .ok_or_else(|| "node catalogue.units is required".to_owned())?;
    exact_keys(catalog_units, &["length"], "node catalogue.units")?;
    if field_str(catalog_units, "length", "node catalogue.units")? != "m" {
        return Err("node catalogue length unit must be m".to_owned());
    }
    validate_provenance(
        catalog
            .get("provenance")
            .ok_or_else(|| "node catalogue.provenance missing".to_owned())?,
        "node catalogue.provenance",
    )?;
    let values = catalog
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "node catalogue.nodes must be an array".to_owned())?;
    let mut result = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let at = format!("node catalogue.nodes[{index}]");
        exact_keys(value, &["node_id", "position_m"], &at)?;
        let node_id = field_u64(value, "node_id", &at)?;
        let position_m = field_vec3(value, "position_m", &at)?;
        if !seen.insert(node_id) {
            return Err(format!("{at}.node_id duplicates {node_id}"));
        }
        result.push(SupportNode {
            node_id,
            position_m,
        });
    }
    if &seen != expected_support_ids {
        return Err(
            "support-node catalogue IDs differ from the declared support inventory".to_owned(),
        );
    }
    Ok(result)
}

fn request_boundary_dofs(
    request: &Value,
    support_map: &BTreeMap<&str, u64>,
) -> Check<BTreeSet<(u64, u8)>> {
    let conditions = request
        .get("boundary_conditions")
        .and_then(Value::as_array)
        .ok_or_else(|| "request.boundary_conditions must be an array".to_owned())?;
    let mut result = BTreeSet::new();
    for (index, condition) in conditions.iter().enumerate() {
        let at = format!("request.boundary_conditions[{index}]");
        if field_str(condition, "kind", &at)? != "fixed" {
            return Err(format!("{at} is not a fixed displacement condition"));
        }
        let target = field_str(condition, "target_id", &at)?;
        let node_id = *support_map.get(target).ok_or_else(|| {
            format!("{at}.target_id '{target}' has no exact support-node mapping")
        })?;
        let dofs = condition
            .get("dofs")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{at}.dofs must be an array"))?;
        if dofs.is_empty() {
            return Err(format!("{at}.dofs must not be empty"));
        }
        let mut local_seen = BTreeSet::new();
        for value in dofs {
            let dof = match value.as_str() {
                Some("ux") => 1,
                Some("uy") => 2,
                Some("uz") => 3,
                Some(other) => {
                    return Err(format!(
                        "{at} DOF '{other}' outside translational T3D2 envelope"
                    ));
                }
                None => return Err(format!("{at}.dofs entries must be strings")),
            };
            if !local_seen.insert(dof) || !result.insert((node_id, dof)) {
                return Err(format!("{at} duplicates a boundary DOF"));
            }
        }
    }
    Ok(result)
}

fn request_point_loads(
    request: &Value,
    load_map: &BTreeMap<&str, u64>,
    deck_nodes: &BTreeMap<u64, [f64; 3]>,
) -> Check<BTreeMap<(u64, u8), f64>> {
    let loads = request
        .get("loads")
        .and_then(Value::as_array)
        .ok_or_else(|| "request.loads must be an array".to_owned())?;
    if loads.is_empty() {
        return Err("request.loads must not be empty".to_owned());
    }
    let mut result = BTreeMap::new();
    let mut used_targets = BTreeSet::new();
    for (index, load) in loads.iter().enumerate() {
        let at = format!("request.loads[{index}]");
        if field_str(load, "kind", &at)? != "point_force" {
            return Err(format!("{at} is outside the point-force-only RF envelope"));
        }
        let target_id = field_str(load, "target_id", &at)?;
        if !used_targets.insert(target_id) {
            return Err(format!("{at}.target_id duplicates '{target_id}'"));
        }
        let node_id = *load_map.get(target_id).ok_or_else(|| {
            format!("{at}.target_id '{target_id}' has no exact load-node mapping")
        })?;
        if !deck_nodes.contains_key(&node_id) {
            return Err(format!(
                "{at} maps to node {node_id}, which is absent from deck"
            ));
        }
        let frame = field_str(load, "coordinate_frame_id", &at)?;
        if frame != field_str(request, "coordinate_frame_id", "request")? {
            return Err(format!(
                "{at} coordinate frame differs from the global request frame"
            ));
        }
        let vector = field_vec3(load, "vector_xyz", &at)?;
        for (component, magnitude) in vector.iter().enumerate() {
            if *magnitude != 0.0 {
                result.insert((node_id, (component + 1) as u8), *magnitude);
            }
        }
    }
    if used_targets.len() != load_map.len() {
        return Err("load_node_map contains unused or missing target mappings".to_owned());
    }
    Ok(result)
}

fn parse_cards(deck: &str) -> Check<Vec<Card>> {
    let mut cards = Vec::new();
    for (index, raw) in deck.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("**") {
            continue;
        }
        if trimmed.starts_with('*') {
            let mut pieces = trimmed.split(',');
            let name = pieces
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches('*')
                .trim()
                .to_ascii_uppercase();
            if name.is_empty() {
                return Err(format!("empty keyword at deck line {line_number}"));
            }
            let mut params = BTreeMap::new();
            for parameter in pieces {
                let part = parameter.trim();
                if part.is_empty() {
                    return Err(format!("empty parameter at deck line {line_number}"));
                }
                let (key, value) = part
                    .split_once('=')
                    .map(|(key, value)| {
                        (
                            key.trim().to_ascii_uppercase(),
                            Some(value.trim().to_owned()),
                        )
                    })
                    .unwrap_or((part.to_ascii_uppercase(), None));
                let value = value.ok_or_else(|| {
                    format!("unsupported bare parameter '{key}' at deck line {line_number}")
                })?;
                if key.is_empty() || value.is_empty() || params.insert(key.clone(), value).is_some()
                {
                    return Err(format!(
                        "invalid or duplicate parameter '{key}' at deck line {line_number}"
                    ));
                }
            }
            cards.push(Card {
                name,
                params,
                data: Vec::new(),
            });
        } else {
            let card = cards.last_mut().ok_or_else(|| {
                format!("deck data occurs before any keyword at line {line_number}")
            })?;
            card.data.push((line_number, trimmed.to_owned()));
        }
    }
    if cards.is_empty() {
        return Err("deck contains no keyword cards".to_owned());
    }
    Ok(cards)
}

fn validate_and_parse_deck(cards: &[Card], expected_node_set: &str) -> Check<ParsedDeck> {
    let mut nodes = BTreeMap::new();
    let mut elements = BTreeMap::new();
    let mut node_sets = BTreeMap::<String, BTreeSet<u64>>::new();
    let mut element_set = String::new();
    let mut element_members = BTreeSet::new();
    let mut boundary_dofs = BTreeSet::new();
    let mut cloads = BTreeMap::new();
    let mut node_print_set = String::new();
    let mut elastic_modulus_pa = None;
    let mut poisson_ratio = None;
    let mut section_area_m2 = None;
    let mut section_elset = String::new();
    let mut section_material = String::new();
    let mut material_names = BTreeSet::new();
    let mut have_heading = false;
    let mut have_node = false;
    let mut have_element = false;
    let mut have_boundary = false;
    let mut have_cload = false;
    let mut have_print = false;
    let mut have_static = false;
    let mut have_step = false;
    let mut have_end_step = false;
    let mut have_material = false;
    let mut have_elastic = false;
    let mut have_section = false;
    let mut static_count = 0;
    let mut step_count = 0;

    for card in cards {
        match card.name.as_str() {
            "HEADING" => {
                if have_heading || !card.params.is_empty() {
                    return Err("v0.1 permits at most one parameter-free HEADING".to_owned());
                }
                have_heading = true;
            }
            "NODE" => {
                if have_node || !card.params.is_empty() {
                    return Err("v0.1 permits exactly one parameter-free NODE card".to_owned());
                }
                have_node = true;
                for (line, row) in &card.data {
                    let f = csv_fields(row);
                    if f.len() != 4 {
                        return Err(format!(
                            "NODE row at deck line {line} must contain node ID and x,y,z"
                        ));
                    }
                    let id = parse_id(f[0], *line, "node ID")?;
                    let xyz = [
                        parse_finite(f[1], *line)?,
                        parse_finite(f[2], *line)?,
                        parse_finite(f[3], *line)?,
                    ];
                    if nodes.insert(id, xyz).is_some() {
                        return Err(format!("duplicate node ID {id}"));
                    }
                }
            }
            "NSET" => {
                only_params(card, &["NSET"])?;
                let name = param(card, "NSET")?.to_ascii_uppercase();
                validate_token(&name, "NSET name")?;
                if node_sets.contains_key(&name) {
                    return Err(format!("duplicate NSET '{name}'"));
                }
                let mut values = BTreeSet::new();
                for (line, row) in &card.data {
                    for field in csv_fields(row) {
                        if field.eq_ignore_ascii_case("GENERATE") {
                            return Err(
                                "NSET GENERATE is not supported in RF proof v0.1".to_owned()
                            );
                        }
                        let id = parse_id(field, *line, "NSET node ID")?;
                        if !values.insert(id) {
                            return Err(format!("NSET '{name}' repeats node ID {id}"));
                        }
                    }
                }
                if values.is_empty() {
                    return Err(format!("NSET '{name}' is empty"));
                }
                node_sets.insert(name, values);
            }
            "ELEMENT" => {
                if have_element {
                    return Err("v0.1 permits exactly one ELEMENT card".to_owned());
                }
                have_element = true;
                only_params(card, &["TYPE", "ELSET"])?;
                if !param(card, "TYPE")?.eq_ignore_ascii_case("T3D2") {
                    return Err("restricted RF v0.1 permits T3D2 elements only".to_owned());
                }
                element_set = param(card, "ELSET")?.to_ascii_uppercase();
                validate_token(&element_set, "element set name")?;
                for (line, row) in &card.data {
                    let f = csv_fields(row);
                    if f.len() != 3 {
                        return Err(format!(
                            "T3D2 element row at deck line {line} requires element ID and two node IDs"
                        ));
                    }
                    let eid = parse_id(f[0], *line, "element ID")?;
                    let n1 = parse_id(f[1], *line, "element node ID")?;
                    let n2 = parse_id(f[2], *line, "element node ID")?;
                    if n1 == n2 || elements.insert(eid, [n1, n2]).is_some() {
                        return Err(format!("invalid/duplicate T3D2 element {eid}"));
                    }
                    element_members.insert(eid);
                }
            }
            "MATERIAL" => {
                if have_material {
                    return Err("v0.1 permits one MATERIAL only".to_owned());
                }
                have_material = true;
                only_params(card, &["NAME"])?;
                let name = param(card, "NAME")?.to_ascii_uppercase();
                validate_token(&name, "material name")?;
                material_names.insert(name);
                if !card.data.is_empty() {
                    return Err("MATERIAL card must not contain data rows in v0.1".to_owned());
                }
            }
            "ELASTIC" => {
                if have_elastic || !card.params.is_empty() {
                    return Err("v0.1 permits one parameter-free ELASTIC card".to_owned());
                }
                have_elastic = true;
                if card.data.len() != 1 {
                    return Err("ELASTIC must have exactly one isotropic data row".to_owned());
                }
                let f = csv_fields(&card.data[0].1);
                if f.len() != 2 {
                    return Err("ELASTIC row must contain E and Poisson ratio".to_owned());
                }
                let e = parse_finite(f[0], card.data[0].0)?;
                let nu = parse_finite(f[1], card.data[0].0)?;
                if e <= 0.0 || !(-1.0..0.5).contains(&nu) {
                    return Err(
                        "ELASTIC constants are outside the stable isotropic range".to_owned()
                    );
                }
                elastic_modulus_pa = Some(e);
                poisson_ratio = Some(nu);
            }
            "SOLID SECTION" => {
                if have_section {
                    return Err("v0.1 permits one SOLID SECTION only".to_owned());
                }
                have_section = true;
                only_params(card, &["ELSET", "MATERIAL"])?;
                section_elset = param(card, "ELSET")?.to_ascii_uppercase();
                section_material = param(card, "MATERIAL")?.to_ascii_uppercase();
                if card.data.len() != 1 {
                    return Err("T3D2 SOLID SECTION must have exactly one area row".to_owned());
                }
                let f = csv_fields(&card.data[0].1);
                if f.len() != 1 {
                    return Err("T3D2 section area row must have one numeric value".to_owned());
                }
                let area = parse_finite(f[0], card.data[0].0)?;
                if area <= 0.0 {
                    return Err("T3D2 section area must be strictly positive".to_owned());
                }
                section_area_m2 = Some(area);
            }
            "BOUNDARY" => {
                if have_boundary || !card.params.is_empty() {
                    return Err("v0.1 permits one parameter-free BOUNDARY card".to_owned());
                }
                have_boundary = true;
                for (line, row) in &card.data {
                    let f = csv_fields(row);
                    if f.len() != 4 {
                        return Err(format!(
                            "BOUNDARY row at deck line {line} must contain node, first DOF, last DOF, zero value"
                        ));
                    }
                    let node = parse_id(f[0], *line, "boundary node ID")?;
                    let first = parse_dof(f[1], *line)?;
                    let last = parse_dof(f[2], *line)?;
                    let value = parse_finite(f[3], *line)?;
                    if first > last || value != 0.0 {
                        return Err(format!(
                            "BOUNDARY at deck line {line} must be a zero-value translational constraint with ordered DOFs"
                        ));
                    }
                    for dof in first..=last {
                        if !boundary_dofs.insert((node, dof)) {
                            return Err(format!(
                                "duplicate BOUNDARY DOF for node {node}, DOF {dof}"
                            ));
                        }
                    }
                }
            }
            "CLOAD" => {
                if have_cload || !card.params.is_empty() {
                    return Err("v0.1 permits one parameter-free CLOAD card".to_owned());
                }
                have_cload = true;
                for (line, row) in &card.data {
                    let f = csv_fields(row);
                    if f.len() != 3 {
                        return Err(format!(
                            "CLOAD row at deck line {line} must contain node, DOF, magnitude"
                        ));
                    }
                    let node = parse_id(f[0], *line, "CLOAD node ID")?;
                    let dof = parse_dof(f[1], *line)?;
                    let value = parse_finite(f[2], *line)?;
                    if value == 0.0 {
                        return Err(format!(
                            "zero CLOAD row at deck line {line} is non-canonical and unsupported"
                        ));
                    }
                    if cloads.insert((node, dof), value).is_some() {
                        return Err(format!("duplicate CLOAD node/DOF ({node},{dof})"));
                    }
                }
            }
            "STEP" => {
                if have_step || !card.params.is_empty() || !card.data.is_empty() {
                    return Err("v0.1 permits one parameter-free STEP card".to_owned());
                }
                have_step = true;
                step_count += 1;
            }
            "STATIC" => {
                if have_static || !card.params.is_empty() || card.data.len() > 1 {
                    return Err(
                        "v0.1 permits one parameter-free STATIC card with at most one control row"
                            .to_owned(),
                    );
                }
                have_static = true;
                static_count += 1;
                for (line, row) in &card.data {
                    let f = csv_fields(row);
                    if f.is_empty() || f.len() > 4 {
                        return Err(format!(
                            "STATIC control row at deck line {line} has invalid field count"
                        ));
                    }
                    for value in f {
                        let n = parse_finite(value, *line)?;
                        if n <= 0.0 {
                            return Err(format!(
                                "STATIC control value at deck line {line} must be positive"
                            ));
                        }
                    }
                }
            }
            "NODE PRINT" => {
                if have_print {
                    return Err("v0.1 permits exactly one NODE PRINT card".to_owned());
                }
                have_print = true;
                only_params(card, &["NSET"])?;
                node_print_set = param(card, "NSET")?.to_ascii_uppercase();
                validate_token(&node_print_set, "NODE PRINT NSET")?;
                if card.data.len() != 1
                    || csv_fields(&card.data[0].1).len() != 1
                    || !csv_fields(&card.data[0].1)[0].eq_ignore_ascii_case("RF")
                {
                    return Err(
                        "NODE PRINT must request RF only, without other output keys".to_owned()
                    );
                }
            }
            "END STEP" => {
                if have_end_step || !card.params.is_empty() || !card.data.is_empty() {
                    return Err("v0.1 permits one parameter-free END STEP".to_owned());
                }
                have_end_step = true;
            }
            other => {
                return Err(format!(
                    "unsupported deck keyword '*{other}' in restricted RF proof v0.1"
                ));
            }
        }
    }

    if !have_node
        || nodes.is_empty()
        || !have_element
        || elements.is_empty()
        || !have_boundary
        || boundary_dofs.is_empty()
        || !have_cload
        || cloads.is_empty()
        || !have_print
        || node_print_set != expected_node_set
        || !have_step
        || !have_static
        || !have_end_step
        || step_count != 1
        || static_count != 1
        || !have_material
        || material_names.len() != 1
        || !have_elastic
        || !have_section
    {
        return Err("deck is incomplete for restricted static-linear T3D2 RF semantics".to_owned());
    }
    if section_material != *material_names.iter().next().expect("one material required") {
        return Err("SOLID SECTION material does not match the sole MATERIAL card".to_owned());
    }
    if section_elset != element_set || element_members.is_empty() {
        return Err("SOLID SECTION ELSET must exactly match the sole T3D2 element set".to_owned());
    }
    for node in boundary_dofs
        .iter()
        .map(|(node, _)| node)
        .chain(cloads.keys().map(|(node, _)| node))
    {
        if !nodes.contains_key(node) {
            return Err(format!(
                "load/boundary node {node} is absent from NODE inventory"
            ));
        }
    }
    for (eid, endpoints) in &elements {
        if !endpoints.iter().all(|node| nodes.contains_key(node)) {
            return Err(format!("element {eid} refers to a missing node"));
        }
        if !element_members.contains(eid) {
            return Err(format!("element {eid} is absent from its declared ELSET"));
        }
    }
    let connected: BTreeSet<u64> = elements
        .values()
        .flat_map(|nodes| nodes.iter().copied())
        .collect();
    if !boundary_dofs
        .iter()
        .map(|(node, _)| *node)
        .collect::<BTreeSet<_>>()
        .is_subset(&connected)
        || !cloads
            .keys()
            .map(|(node, _)| *node)
            .collect::<BTreeSet<_>>()
            .is_subset(&connected)
    {
        return Err(
            "all load and boundary nodes must belong to the T3D2 element inventory".to_owned(),
        );
    }
    node_sets.iter().try_for_each(|(name, members)| {
        if !members.iter().all(|id| nodes.contains_key(id)) {
            return Err(format!("NSET '{name}' references a missing node"));
        }
        Ok(())
    })?;
    Ok(ParsedDeck {
        nodes,
        elements,
        node_sets,
        boundary_dofs,
        cloads,
        node_print_set,
        elastic_modulus_pa: elastic_modulus_pa.expect("checked above"),
        poisson_ratio: poisson_ratio.expect("checked above"),
        section_area_m2: section_area_m2.expect("checked above"),
    })
}

fn parse_rf_section(dat: &str, node_set: &str, expected: &BTreeSet<u64>) -> Check<Vec<RfRow>> {
    let lines: Vec<&str> = dat.lines().collect();
    let lower_set = node_set.to_ascii_lowercase();
    let mut headers = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let lower = line.trim().to_ascii_lowercase();
        if lower.starts_with(RF_HEADER_PREFIX) {
            let (observed, _) = parse_rf_header(line)?;
            if observed.eq_ignore_ascii_case(&lower_set) {
                headers.push(index);
            }
        } else if lower.starts_with("reaction forces (") {
            let (observed, _) = parse_rf_header(line)?;
            if observed.eq_ignore_ascii_case(&lower_set) {
                return Err("reaction-only output section is not admitted on restricted RF path; select RR parser instead".to_owned());
            }
        }
    }
    if headers.len() != 1 {
        return Err(format!(
            "expected exactly one RF section for set '{node_set}', observed {}",
            headers.len()
        ));
    }
    let start = headers[0];
    let mut rows = BTreeMap::<u64, RfRow>::new();
    for (index, raw) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            if !rows.is_empty() {
                break;
            }
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("forces (")
            || lower.starts_with("reaction forces (")
            || lower.starts_with("internal forces ")
            || lower.starts_with("total forces ")
        {
            break;
        }
        let fields = csv_fields(trimmed);
        if fields.is_empty() {
            continue;
        }
        let node_id = match fields[0].parse::<u64>() {
            Ok(id) => id,
            Err(_) => {
                let first = fields[0].as_bytes().first().copied().unwrap_or_default();
                if first.is_ascii_digit() || first == b'+' || first == b'-' {
                    return Err(format!("malformed RF row at .dat line {}", index + 1));
                }
                break;
            }
        };
        if fields.len() != 4 {
            return Err(format!(
                "RF row at .dat line {} must have node ID and three force components",
                index + 1
            ));
        }
        if !expected.contains(&node_id) {
            return Err(format!("RF section contains unexpected node {node_id}"));
        }
        let force = [
            parse_finite(fields[1], index + 1)?,
            parse_finite(fields[2], index + 1)?,
            parse_finite(fields[3], index + 1)?,
        ];
        if rows
            .insert(
                node_id,
                RfRow {
                    node_id,
                    force_n: force,
                    source_line: index + 1,
                    raw_line: trimmed.to_owned(),
                },
            )
            .is_some()
        {
            return Err(format!("RF section duplicates node {node_id}"));
        }
    }
    let observed: BTreeSet<u64> = rows.keys().copied().collect();
    if &observed != expected {
        return Err(format!(
            "RF support-node inventory mismatch; expected={expected:?}, observed={observed:?}"
        ));
    }
    Ok(rows.into_values().collect())
}

fn parse_rf_header(line: &str) -> Check<(String, f64)> {
    let lower = line.trim().to_ascii_lowercase();
    let (prefix, tail) = if let Some(tail) = lower.strip_prefix(RF_HEADER_PREFIX) {
        (RF_HEADER_PREFIX, tail)
    } else if let Some(tail) = lower.strip_prefix("reaction forces (rfx,rfy,rfz) for set ") {
        ("reaction forces (rfx,rfy,rfz) for set ", tail)
    } else {
        return Err("unsupported RF output header grammar".to_owned());
    };
    let _ = prefix;
    let (name, time) = tail
        .split_once(" and time ")
        .ok_or_else(|| "RF output header is missing 'and time'".to_owned())?;
    Ok((name.trim().to_owned(), parse_finite(time.trim(), 0)?))
}

fn csv_fields(row: &str) -> Vec<&str> {
    row.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_finite(value: &str, line: usize) -> Check<f64> {
    let normalized = value.replace(['d', 'D'], "E");
    let result = normalized
        .parse::<f64>()
        .map_err(|_| format!("invalid finite numeric token '{value}' at line {line}"))?;
    if !result.is_finite() {
        return Err(format!("non-finite numeric token at line {line}"));
    }
    Ok(result)
}

fn parse_id(value: &str, line: usize, label: &str) -> Check<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| format!("invalid {label} '{value}' at line {line}"))
}

fn parse_dof(value: &str, line: usize) -> Check<u8> {
    value
        .parse::<u8>()
        .ok()
        .filter(|dof| (1..=3).contains(dof))
        .ok_or_else(|| format!("DOF '{value}' at line {line} is outside translational DOFs 1..3"))
}

fn only_params(card: &Card, allowed: &[&str]) -> Check<()> {
    if card
        .params
        .keys()
        .any(|key| !allowed.contains(&key.as_str()))
    {
        return Err(format!("unsupported parameter on *{}", card.name));
    }
    for required in allowed {
        if !card.params.contains_key(*required) {
            return Err(format!("*{} requires parameter {}", card.name, required));
        }
    }
    Ok(())
}

fn param<'a>(card: &'a Card, key: &str) -> Check<&'a str> {
    card.params
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("*{} requires {key}", card.name))
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    for key in expected {
        if !object.contains_key(*key) {
            return Err(format!("{at}.{key} is required"));
        }
    }
    if let Some(key) = object.keys().find(|key| !expected.contains(&key.as_str())) {
        return Err(format!("{at} contains unsupported field '{key}'"));
    }
    Ok(())
}

fn field_str<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn token_field(value: &Value, key: &str, at: &str) -> Check<String> {
    let text = field_str(value, key, at)?;
    validate_token(text, &format!("{at}.{key}"))?;
    Ok(text.to_owned())
}

fn field_sha<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key} must be a string"))?;
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!("{at}.{key} must be lowercase SHA-256 hex"));
    }
    Ok(text)
}

fn field_string_array(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn field_u64(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key} must be a non-negative integer"))
}

fn field_vec3(value: &Value, key: &str, at: &str) -> Check<[f64; 3]> {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be a 3-component vector"))?;
    if values.len() != 3 {
        return Err(format!("{at}.{key} must contain exactly three components"));
    }
    let mut result = [0.0; 3];
    for (index, component) in values.iter().enumerate() {
        result[index] = component
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| format!("{at}.{key}[{index}] must be finite numeric"))?;
    }
    Ok(result)
}

fn validate_token(value: &str, label: &str) -> Check<()> {
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

fn validate_provenance(value: &Value, at: &str) -> Check<()> {
    exact_keys(value, &["method", "identity", "source_ref"], at)?;
    for key in ["method", "identity", "source_ref"] {
        let text = field_str(value, key, at)?;
        if text.trim().is_empty() || text.len() > 2048 {
            return Err(format!("{at}.{key} must be non-empty and <=2048 bytes"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Bundle {
        request: Vec<u8>,
        deck: Vec<u8>,
        dat: Vec<u8>,
        manifest: Vec<u8>,
        catalog: Vec<u8>,
    }

    const DECK: &str = "*HEADING
synthetic axial bar
*NODE
1,0,0,0
2,1,0,0
*ELEMENT,TYPE=T3D2,ELSET=BAR
1,1,2
*NSET,NSET=SUPPORT
1
*MATERIAL,NAME=STEEL
*ELASTIC
210000000000,0.3
*SOLID SECTION,ELSET=BAR,MATERIAL=STEEL
0.01
*BOUNDARY
1,1,3,0
*STEP
*STATIC
1.,1.,1.E-05,1.
*CLOAD
2,1,1000.
*NODE PRINT,NSET=SUPPORT
RF
*END STEP
";
    const DAT: &str = " forces (fx,fy,fz) for set SUPPORT and time 0.1000000E+01
  1 -1.000000E+03 0.000000E+00 0.000000E+00

";

    fn bundle_with(deck_text: &str, dat_text: &str) -> Bundle {
        let request_value = json!({
            "schema_version":"physical-solver-request-v0.1",
            "request_id":"fixture:rf-restricted",
            "source_revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "scene_revision":1,
            "scene_sha256":"a".repeat(64),
            "model_fidelity":"analytical_linear_elastic",
            "requested_physics":["static_linear_elastic"],
            "coordinate_frame_id":"fixture:global-xyz",
            "units":{"length":"m","mass":"kg","time":"s","force":"N","stress":"Pa"},
            "loads":[{
                "load_id":"load-tip",
                "kind":"point_force",
                "target_id":"bar-end-b",
                "vector_xyz":[1000.0,0.0,0.0],
                "coordinate_frame_id":"fixture:global-xyz"
            }],
            "boundary_conditions":[{
                "condition_id":"fixed-end-a",
                "target_id":"bar-end-a",
                "kind":"fixed",
                "dofs":["ux","uy","uz"]
            }],
            "required_outputs":["reaction_force"]
        });
        let request = serde_json::to_vec(&request_value).unwrap();
        let deck = deck_text.as_bytes().to_vec();
        let dat = dat_text.as_bytes().to_vec();
        let request_sha = sha256_hex(&request);
        let deck_sha = sha256_hex(&deck);
        let scene_sha = "a".repeat(64);
        let catalog_value = json!({
            "schema_version":CATALOG_SCHEMA,
            "case_id":"rf-restricted-case",
            "request_id":"fixture:rf-restricted",
            "request_sha256":request_sha,
            "scene_sha256":scene_sha,
            "scene_revision":1,
            "coordinate_frame_id":"fixture:global-xyz",
            "deck_sha256":deck_sha,
            "units":{"length":"m"},
            "nodes":[{"node_id":1,"position_m":[0.0,0.0,0.0]}],
            "provenance":{
                "method":"independently_checked_node_catalogue",
                "identity":"one-node-support-catalogue",
                "source_ref":"fixture-only:rf-restricted"
            }
        });
        let catalog = serde_json::to_vec(&catalog_value).unwrap();
        let manifest_value = json!({
            "schema_version":MANIFEST_SCHEMA,
            "case_id":"rf-restricted-case",
            "request_id":"fixture:rf-restricted",
            "request_sha256":request_sha,
            "scene_sha256":scene_sha,
            "scene_revision":1,
            "coordinate_frame_id":"fixture:global-xyz",
            "node_set":"SUPPORT",
            "deck_sha256":deck_sha,
            "node_catalog_sha256":sha256_hex(&catalog),
            "support_node_map":[{"target_id":"bar-end-a","node_id":1}],
            "load_node_map":[{"target_id":"bar-end-b","node_id":2}],
            "units":{"length":"m","force":"N"},
            "provenance":{
                "method":"independent_request_to_deck_mapping",
                "identity":"simple axial bar load/support map",
                "source_ref":"fixture-only:rf-restricted-v1"
            }
        });
        Bundle {
            request,
            deck,
            dat,
            manifest: serde_json::to_vec(&manifest_value).unwrap(),
            catalog,
        }
    }

    fn bundle() -> Bundle {
        bundle_with(DECK, DAT)
    }

    fn assess(bundle: &Bundle) -> Check<Value> {
        normalize_restricted_rf_bytes(
            &bundle.request,
            &bundle.deck,
            &bundle.dat,
            &bundle.manifest,
            &bundle.catalog,
        )
    }

    #[test]
    fn clean_allowlisted_rf_fixture_proves_restricted_equivalence_without_physical_qualification() {
        let report = assess(&bundle()).unwrap();
        assert_eq!(report["normalization_status"], "PASS");
        assert_eq!(
            report["source_semantics"],
            "RF_under_restricted_deck_equivalence"
        );
        assert_eq!(report["restricted_equivalence_proved"], true);
        assert_eq!(
            report["normalized_output"]["reaction_forces"][0]["force_n"][0],
            -1000.0
        );
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
    }

    #[test]
    fn any_distributed_load_keyword_is_rejected_by_allowlist() {
        let deck = DECK.replace("*CLOAD", "*DLOAD\nBAR,GRAV,-9.81,0,0,-1\n*CLOAD");
        let report = bundle_with(&deck, DAT);
        assert!(
            assess(&report)
                .unwrap_err()
                .contains("unsupported deck keyword '*DLOAD'")
        );
    }

    #[test]
    fn frame_transform_keyword_is_rejected() {
        let deck = DECK.replace(
            "*BOUNDARY",
            "*TRANSFORM,NSET=SUPPORT\n1,0,0,0,0,1,0\n*BOUNDARY",
        );
        let report = bundle_with(&deck, DAT);
        assert!(
            assess(&report)
                .unwrap_err()
                .contains("unsupported deck keyword '*TRANSFORM'")
        );
    }

    #[test]
    fn unknown_cards_are_not_ignored() {
        let deck = DECK.replace("*STEP", "*EQUATION\n2\n1,1,1,2,1,-1\n*STEP");
        assert!(assess(&bundle_with(&deck, DAT)).is_err());
    }

    #[test]
    fn rf_section_for_wrong_support_set_is_rejected() {
        let dat = DAT.replace("for set SUPPORT", "for set OTHER");
        assert!(assess(&bundle_with(DECK, &dat)).is_err());
    }

    #[test]
    fn reaction_only_section_is_not_substituted_for_stock_rf_section() {
        let dat = DAT.replace("forces (fx,fy,fz)", "reaction forces (rfx,rfy,rfz)");
        assert!(
            assess(&bundle_with(DECK, &dat))
                .unwrap_err()
                .contains("reaction-only output section is not admitted")
        );
    }

    #[test]
    fn stale_manifest_deck_digest_is_rejected() {
        let mut b = bundle();
        let mut manifest: Value = serde_json::from_slice(&b.manifest).unwrap();
        manifest["deck_sha256"] = json!("d".repeat(64));
        b.manifest = serde_json::to_vec(&manifest).unwrap();
        assert!(
            assess(&b)
                .unwrap_err()
                .contains("manifest request/deck/node-catalogue SHA-256")
        );
    }

    #[test]
    fn cload_at_support_is_rejected_by_support_load_disjointness() {
        let deck = DECK.replace("2,1,1000.", "1,1,1000.");
        let mut b = bundle_with(&deck, DAT);
        let mut manifest: Value = serde_json::from_slice(&b.manifest).unwrap();
        manifest["load_node_map"][0]["node_id"] = json!(1);
        b.manifest = serde_json::to_vec(&manifest).unwrap();
        assert!(
            assess(&b)
                .unwrap_err()
                .contains("load-node inventory intersects prescribed-support inventory")
        );
    }

    #[test]
    fn output_node_inventory_must_match_support_catalogue() {
        let dat = DAT.replace("  1 -1.000000E+03", "  9 -1.000000E+03");
        assert!(assess(&bundle_with(DECK, &dat)).is_err());
    }
}
