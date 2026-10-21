// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strict parser for CalculiX reaction-only RR output.
//! This parser never treats RF/external-force output as support reactions.

use crate::{Check, sha256_hex};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const MAX_DECK_BYTES: usize = 16 * 1024 * 1024;
const MAX_DAT_BYTES: usize = 32 * 1024 * 1024;
const RR_HEADER_PREFIX: &str = "reaction forces (rfx,rfy,rfz) for set ";

#[derive(Clone, Debug)]
struct ReactionRow {
    node_id: u64,
    force: [f64; 3],
    source_line: usize,
    raw_line: String,
}

/// Parse one reaction-only RR section from an exact deck and .dat byte pair.
///
/// CCX output does not encode a trustworthy unit system. Numeric components are
/// returned as written; units must be inherited from the exact request/deck contract.
pub fn parse_reaction_only_rr_bytes(
    deck_bytes: &[u8],
    dat_bytes: &[u8],
    expected_nset: &str,
    expected_node_ids: &[u64],
) -> Check<Value> {
    if deck_bytes.is_empty() || deck_bytes.len() > MAX_DECK_BYTES {
        return Err("CalculiX input deck is empty or exceeds 16 MiB".to_owned());
    }
    if dat_bytes.is_empty() || dat_bytes.len() > MAX_DAT_BYTES {
        return Err("CalculiX .dat output is empty or exceeds 32 MiB".to_owned());
    }
    validate_token(expected_nset, "expected node set")?;
    if expected_node_ids.is_empty() {
        return Err("expected support-node inventory must not be empty".to_owned());
    }
    let expected: BTreeSet<u64> = expected_node_ids.iter().copied().collect();
    if expected.len() != expected_node_ids.len() {
        return Err("expected support-node inventory contains duplicate node IDs".to_owned());
    }

    let deck = std::str::from_utf8(deck_bytes)
        .map_err(|_| "CalculiX input deck must be UTF-8/ASCII text".to_owned())?;
    if !deck_declares_rr_for_set(deck, expected_nset)? {
        return Err(format!(
            "exact input deck does not explicitly request RR for node set '{expected_nset}'"
        ));
    }
    let dat = std::str::from_utf8(dat_bytes)
        .map_err(|_| "CalculiX .dat output must be UTF-8/ASCII text".to_owned())?;
    let lines: Vec<&str> = dat.lines().collect();
    let headers: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line.trim()
                .to_ascii_lowercase()
                .starts_with(RR_HEADER_PREFIX)
                .then_some(index)
        })
        .collect();
    if headers.is_empty() {
        return Err("no reaction-only RR section found; RF/ordinary force sections are not accepted as reactions".to_owned());
    }
    if headers.len() != 1 {
        return Err("multiple RR sections are ambiguous in parser v0.1".to_owned());
    }

    let header_index = headers[0];
    let (observed_nset, time_value, raw_time) = parse_rr_header(lines[header_index])?;
    if !observed_nset.eq_ignore_ascii_case(expected_nset) {
        return Err(format!(
            "RR section node set '{observed_nset}' does not match expected '{expected_nset}'"
        ));
    }

    let mut rows = BTreeMap::<u64, ReactionRow>::new();
    for (index, raw) in lines.iter().enumerate().skip(header_index + 1) {
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
        let fields: Vec<&str> = trimmed
            .split(|character: char| character.is_whitespace() || character == ',')
            .filter(|field| !field.is_empty())
            .collect();
        if fields.is_empty() {
            continue;
        }
        let node_id = match fields[0].parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                let first = fields[0].as_bytes().first().copied().unwrap_or_default();
                if first.is_ascii_digit() || first == b'+' || first == b'-' {
                    return Err(format!(
                        "malformed RR row at .dat line {}: node ID is not an unsigned integer",
                        index + 1
                    ));
                }
                break;
            }
        };
        if fields.len() != 4 {
            return Err(format!(
                "malformed RR row at .dat line {}: expected node ID and exactly three force components",
                index + 1
            ));
        }
        if !expected.contains(&node_id) {
            return Err(format!(
                "RR output contains unexpected node ID {node_id} at .dat line {}",
                index + 1
            ));
        }
        let force = [
            parse_finite(fields[1], index + 1)?,
            parse_finite(fields[2], index + 1)?,
            parse_finite(fields[3], index + 1)?,
        ];
        if rows
            .insert(
                node_id,
                ReactionRow {
                    node_id,
                    force,
                    source_line: index + 1,
                    raw_line: trimmed.to_owned(),
                },
            )
            .is_some()
        {
            return Err(format!("duplicate RR row for node ID {node_id}"));
        }
    }

    let observed: BTreeSet<u64> = rows.keys().copied().collect();
    if observed != expected {
        let missing: Vec<u64> = expected.difference(&observed).copied().collect();
        let extra: Vec<u64> = observed.difference(&expected).copied().collect();
        return Err(format!(
            "RR node inventory mismatch; missing={missing:?}, extra={extra:?}"
        ));
    }

    let nodes: Vec<Value> = rows
        .values()
        .map(|row| {
            json!({
                "node_id": row.node_id,
                "force_components_as_written": row.force,
                "source_line": row.source_line,
                "raw_line": row.raw_line
            })
        })
        .collect();
    Ok(json!({
        "schema_version": "calculix-rr-parsed-reactions-v0.1",
        "parse_status": "PASS",
        "source_semantics": "reaction_only_RR",
        "deck_sha256": sha256_hex(deck_bytes),
        "dat_sha256": sha256_hex(dat_bytes),
        "node_set": observed_nset,
        "time_value_as_written": raw_time,
        "time_value": time_value,
        "expected_node_ids": expected.iter().copied().collect::<Vec<_>>(),
        "nodes": nodes,
        "unit_status": "NOT_ENCODED_IN_CCX_OUTPUT",
        "coordinate_frame_status": "NOT_INFERRED_BY_PARSER",
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "claim_ceiling": "one raw RR section parsed under a bound deck/node-set expectation; no execution authentication, unit inference, equilibrium pass, physical validation or solver qualification"
    }))
}

fn deck_declares_rr_for_set(deck: &str, expected_nset: &str) -> Check<bool> {
    let lines: Vec<&str> = deck.lines().collect();
    let mut matching_cards = 0usize;
    let mut has_rr = false;
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("**") || !lower.starts_with("*node print") {
            continue;
        }
        let params: Vec<&str> = line.split(',').skip(1).collect();
        let card_nset = params.iter().find_map(|parameter| {
            let (key, value) = parameter.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("nset")
                .then_some(value.trim())
        });
        if !card_nset.is_some_and(|value| value.eq_ignore_ascii_case(expected_nset)) {
            continue;
        }
        matching_cards += 1;
        for variable_line in lines.iter().skip(index + 1) {
            let variable = variable_line.trim();
            if variable.starts_with('*') && !variable.starts_with("**") {
                break;
            }
            if variable.starts_with("**") || variable.is_empty() {
                continue;
            }
            for field in variable
                .split(|character: char| character.is_whitespace() || character == ',')
                .filter(|field| !field.is_empty())
            {
                if field.eq_ignore_ascii_case("RR") {
                    has_rr = true;
                }
            }
        }
    }
    if matching_cards > 1 {
        return Err(format!(
            "input deck contains {matching_cards} *NODE PRINT cards for '{expected_nset}'; expected one unambiguous RR request"
        ));
    }
    Ok(matching_cards == 1 && has_rr)
}

fn parse_rr_header(line: &str) -> Check<(String, f64, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let suffix = lower
        .strip_prefix(RR_HEADER_PREFIX)
        .ok_or_else(|| "RR header does not match supported CalculiX grammar".to_owned())?;
    let (nset, time_text) = suffix
        .split_once(" and time ")
        .ok_or_else(|| "RR header is missing the 'and time' field".to_owned())?;
    let nset = nset.trim();
    if nset.is_empty() {
        return Err("RR header node-set name is empty".to_owned());
    }
    let raw_time = time_text.trim();
    let time = parse_finite(raw_time, 0)?;
    Ok((nset.to_owned(), time, raw_time.to_owned()))
}

fn parse_finite(value: &str, line_number: usize) -> Check<f64> {
    let normalized = value.replace(['d', 'D'], "E");
    let parsed = normalized.parse::<f64>().map_err(|_| {
        if line_number == 0 {
            format!("invalid RR header time value '{value}'")
        } else {
            format!("invalid RR numeric value '{value}' at .dat line {line_number}")
        }
    })?;
    if !parsed.is_finite() {
        return Err(if line_number == 0 {
            "RR header time must be finite".to_owned()
        } else {
            format!("RR numeric value at .dat line {line_number} must be finite")
        });
    }
    Ok(parsed)
}

fn validate_token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(format!("{label} is not a valid stable token"));
    }
    Ok(())
}

/// Join a parsed RR block with a support-node catalogue bound to the exact deck.
///
/// This emits the existing normalized reaction-force output contract. It accepts only
/// SI request units and one explicit coordinate frame; it applies no implicit unit or
/// frame transform. The caller must record the raw deck and .dat plus node-catalogue
/// digests as receipt input artifacts.
pub fn normalize_reaction_only_rr_bytes(
    request_bytes: &[u8],
    deck_bytes: &[u8],
    dat_bytes: &[u8],
    manifest_bytes: &[u8],
    node_catalog_bytes: &[u8],
) -> Check<Value> {
    const MANIFEST_SCHEMA: &str = "calculix-rr-normalization-manifest-v0.1";
    const CATALOG_SCHEMA: &str = "physical-support-node-catalog-v0.1";
    let request_sha256 = sha256_hex(request_bytes);
    let deck_sha256 = sha256_hex(deck_bytes);
    let dat_sha256 = sha256_hex(dat_bytes);
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let node_catalog_sha256 = sha256_hex(node_catalog_bytes);

    let request: Value = serde_json::from_slice(request_bytes)
        .map_err(|error| format!("invalid request JSON: {error}"))?;
    let manifest: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("invalid RR normalization manifest JSON: {error}"))?;
    exact_object_keys(
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
            "expected_node_ids",
            "units",
            "provenance",
        ],
        "RR normalization manifest",
    )?;
    if string_field(&manifest, "schema_version", "manifest")? != MANIFEST_SCHEMA {
        return Err(format!("manifest.schema_version must equal '{MANIFEST_SCHEMA}'"));
    }
    let case_id = token_field(&manifest, "case_id", "manifest")?;
    let request_id = string_field(&manifest, "request_id", "manifest")?;
    let scene_sha256 = string_field(&manifest, "scene_sha256", "manifest")?;
    let coordinate_frame_id = token_field(&manifest, "coordinate_frame_id", "manifest")?;
    let scene_revision = integer_field(&manifest, "scene_revision", "manifest")?;
    let node_set = token_field(&manifest, "node_set", "manifest")?;
    let manifest_request_sha = string_field(&manifest, "request_sha256", "manifest")?;
    let manifest_deck_sha = string_field(&manifest, "deck_sha256", "manifest")?;
    let manifest_catalog_sha = string_field(&manifest, "node_catalog_sha256", "manifest")?;
    if manifest_request_sha != request_sha256
        || manifest_deck_sha != deck_sha256
        || manifest_catalog_sha != node_catalog_sha256
    {
        return Err("manifest request/deck/node-catalogue digest binding does not match exact input bytes".to_owned());
    }
    if string_field(&request, "request_id", "request")? != request_id
        || string_field(&request, "scene_sha256", "request")? != scene_sha256
        || integer_field(&request, "scene_revision", "request")? != scene_revision
        || string_field(&request, "coordinate_frame_id", "request")? != coordinate_frame_id
    {
        return Err("manifest request/scene/frame identity differs from the exact request".to_owned());
    }
    let fidelity = string_field(&request, "model_fidelity", "request")?;
    if !["analytical_linear_elastic", "continuum_linear_elastic"].contains(&fidelity) {
        return Err("RR normalization v0.1 only supports linear-elastic fidelity".to_owned());
    }
    let physics = string_array_field(&request, "requested_physics", "request")?;
    if physics.as_slice() != ["static_linear_elastic"] {
        return Err("RR normalization v0.1 only supports static_linear_elastic".to_owned());
    }
    if !string_array_field(&request, "required_outputs", "request")?
        .iter().any(|kind| kind == "reaction_force")
    {
        return Err("request must explicitly require reaction_force output".to_owned());
    }
    let request_units = request.get("units").ok_or_else(|| "request.units is required".to_owned())?;
    if string_field(request_units, "length", "request.units")? != "m"
        || string_field(request_units, "force", "request.units")? != "N"
    {
        return Err("RR normalization v0.1 requires request units m and N".to_owned());
    }

    let units = manifest.get("units").ok_or_else(|| "manifest.units is required".to_owned())?;
    exact_object_keys(units, &["length", "force"], "manifest.units")?;
    if string_field(units, "length", "manifest.units")? != "m"
        || string_field(units, "force", "manifest.units")? != "N"
    {
        return Err("manifest units must be exactly m and N".to_owned());
    }
    validate_provenance(manifest.get("provenance").ok_or_else(|| "manifest.provenance is required".to_owned())?, "manifest.provenance")?;

    let expected_node_ids = id_array_u64(&manifest, "expected_node_ids", "manifest")?;
    if expected_node_ids.is_empty() {
        return Err("manifest.expected_node_ids must not be empty".to_owned());
    }
    let expected_ids: BTreeSet<u64> = expected_node_ids.iter().copied().collect();
    if expected_ids.len() != expected_node_ids.len() {
        return Err("manifest.expected_node_ids contains duplicate IDs".to_owned());
    }

    let catalog: Value = serde_json::from_slice(node_catalog_bytes)
        .map_err(|error| format!("invalid support-node catalogue JSON: {error}"))?;
    exact_object_keys(
        &catalog,
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
        "support-node catalogue",
    )?;
    if string_field(&catalog, "schema_version", "node catalogue")? != CATALOG_SCHEMA
        || string_field(&catalog, "case_id", "node catalogue")? != case_id
        || string_field(&catalog, "request_id", "node catalogue")? != request_id
        || string_field(&catalog, "request_sha256", "node catalogue")? != request_sha256
        || string_field(&catalog, "scene_sha256", "node catalogue")? != scene_sha256
        || integer_field(&catalog, "scene_revision", "node catalogue")? != scene_revision
        || string_field(&catalog, "coordinate_frame_id", "node catalogue")? != coordinate_frame_id
        || string_field(&catalog, "deck_sha256", "node catalogue")? != deck_sha256
    {
        return Err("support-node catalogue identity/frame/deck does not match exact inputs".to_owned());
    }
    let catalog_units = catalog.get("units").ok_or_else(|| "node catalogue.units is required".to_owned())?;
    exact_object_keys(catalog_units, &["length"], "node catalogue.units")?;
    if string_field(catalog_units, "length", "node catalogue.units")? != "m" {
        return Err("support-node catalogue length unit must be m".to_owned());
    }
    validate_provenance(catalog.get("provenance").ok_or_else(|| "node catalogue.provenance is required".to_owned())?, "node catalogue.provenance")?;

    let catalog_nodes = catalog.get("nodes").and_then(Value::as_array)
        .ok_or_else(|| "node catalogue.nodes must be an array".to_owned())?;
    let mut positions = BTreeMap::<u64, [f64; 3]>::new();
    for (index, node) in catalog_nodes.iter().enumerate() {
        let at = format!("node catalogue.nodes[{index}]");
        exact_object_keys(node, &["node_id", "position_m"], &at)?;
        let node_id = integer_field(node, "node_id", &at)?;
        let position = vec3_field(node, "position_m", &at)?;
        if positions.insert(node_id, position).is_some() {
            return Err(format!("{at}.node_id duplicates node {node_id}"));
        }
    }
    let catalog_ids: BTreeSet<u64> = positions.keys().copied().collect();
    if catalog_ids != expected_ids {
        let missing: Vec<u64> = expected_ids.difference(&catalog_ids).copied().collect();
        let extra: Vec<u64> = catalog_ids.difference(&expected_ids).copied().collect();
        return Err(format!("support-node catalogue inventory mismatch; missing={missing:?}, extra={extra:?}"));
    }

    let parsed =
        parse_reaction_only_rr_bytes(deck_bytes, dat_bytes, &node_set, &expected_node_ids)?;
    let parsed_nodes = parsed["nodes"]
        .as_array()
        .ok_or_else(|| "RR parser did not return a nodes array".to_owned())?;
    let mut reaction_forces = Vec::with_capacity(parsed_nodes.len());
    for node in parsed_nodes {
        let node_id = integer_field(node, "node_id", "parsed RR node")?;
        let force_n = vec3_field(node, "force_components_as_written", "parsed RR node")?;
        let position_m = positions
            .get(&node_id)
            .ok_or_else(|| format!("RR node {node_id} has no catalogue position"))?;
        reaction_forces.push(json!({
            "reaction_id": format!("node-{node_id}"),
            "force_n": force_n,
            "position_m": position_m
        }));
    }
    let normalized_output = json!({
        "schema_version": "physical-equilibrium-reaction-forces-v0.1",
        "case_id": case_id,
        "request_id": request_id,
        "request_sha256": request_sha256,
        "scene_sha256": scene_sha256,
        "scene_revision": scene_revision,
        "coordinate_frame_id": coordinate_frame_id,
        "units": {"length":"m","force":"N"},
        "reaction_forces": reaction_forces
    });
    let normalized_bytes = serde_json::to_vec(&normalized_output)
        .map_err(|error| format!("normalized reaction JSON serialization failed: {error}"))?;
    let normalized_sha256 = sha256_hex(&normalized_bytes);
    Ok(json!({
        "schema_version": "calculix-rr-normalization-report-v0.1",
        "normalization_status": "PASS",
        "case_id": case_id,
        "request_id": request_id,
        "request_sha256": request_sha256,
        "scene_sha256": scene_sha256,
        "scene_revision": scene_revision,
        "coordinate_frame_id": coordinate_frame_id,
        "node_set": node_set,
        "deck_sha256": deck_sha256,
        "dat_sha256": dat_sha256,
        "manifest_sha256": manifest_sha256,
        "node_catalog_sha256": node_catalog_sha256,
        "normalized_output_sha256": normalized_sha256,
        "normalized_output": normalized_output,
        "source_semantics": "reaction_only_RR",
        "source_output_sha256": parsed["dat_sha256"],
        "unit_provenance": "exact request plus manifest, SI m and N",
        "coordinate_frame_provenance": "request manifest and support-node catalogue agree; no transform applied",
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "claim_ceiling": "receipt-ready normalized reaction-force payload from a strict RR parser and exact deck-bound node catalogue; no execution authentication, balance pass, physical validation or solver qualification"
    }))
}

fn exact_object_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
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

fn string_field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn token_field(value: &Value, key: &str, at: &str) -> Check<String> {
    let text = string_field(value, key, at)?;
    validate_token(text, &format!("{at}.{key}"))?;
    Ok(text.to_owned())
}

fn integer_field(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key} must be a non-negative integer"))
}

fn string_array_field(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
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

fn id_array_u64(value: &Value, key: &str, at: &str) -> Check<Vec<u64>> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_u64()
                .ok_or_else(|| format!("{at}.{key}[{index}] must be a non-negative integer"))
        })
        .collect()
}

fn vec3_field(value: &Value, key: &str, at: &str) -> Check<[f64; 3]> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be a three-component vector"))?;
    if array.len() != 3 {
        return Err(format!("{at}.{key} must contain exactly three components"));
    }
    let mut vector = [0.0; 3];
    for (index, component) in array.iter().enumerate() {
        let number = component
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("{at}.{key}[{index}] must be finite numeric"))?;
        vector[index] = number;
    }
    Ok(vector)
}

fn validate_provenance(value: &Value, at: &str) -> Check<()> {
    exact_object_keys(value, &["method", "identity", "source_ref"], at)?;
    for key in ["method", "identity", "source_ref"] {
        let text = string_field(value, key, at)?;
        if text.trim().is_empty() || text.len() > 2048 {
            return Err(format!(
                "{at}.{key} must be non-empty and at most 2048 bytes"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECK_WITH_RR: &str = "*STEP\n*NODE PRINT,NSET=FIXED\nRF, RR\n*END STEP\n";
    const RR_ONLY_OUTPUT: &str = " reaction forces (rfx,rfy,rfz) for set FIXED and time  0.1000000E+01\n  1 -1.000000E+03 0.000000E+00 0.000000E+00\n  2 1.000000E+03 0.000000E+00 0.000000E+00\n";
    const RF_AND_RR_OUTPUT: &str = " forces (fx,fy,fz) for set FIXED and time 0.1000000E+01\n  1 0.000000E+00 0.000000E+00 0.000000E+00\n  2 5.000000E+02 0.000000E+00 0.000000E+00\n\n reaction forces (rfx,rfy,rfz) for set FIXED and time 0.1000000E+01\n  1 -1.000000E+03 0.000000E+00 0.000000E+00\n  2 1.000000E+03 0.000000E+00 0.000000E+00\n";

    #[test]
    fn selects_reaction_only_block_not_ordinary_force_block() {
        let report = parse_reaction_only_rr_bytes(
            DECK_WITH_RR.as_bytes(),
            RF_AND_RR_OUTPUT.as_bytes(),
            "FIXED",
            &[1, 2],
        )
        .unwrap();
        assert_eq!(report["parse_status"], "PASS");
        assert_eq!(report["source_semantics"], "reaction_only_RR");
        assert_eq!(
            report["nodes"][0]["force_components_as_written"][0],
            -1000.0
        );
        assert_eq!(report["nodes"][1]["force_components_as_written"][0], 1000.0);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
    }

    #[test]
    fn rf_only_output_is_not_accepted_as_reaction_data() {
        let rf_only =
            " forces (fx,fy,fz) for set FIXED and time 1.0\n 1 0.0 0.0 0.0\n 2 0.0 0.0 0.0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                rf_only.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn deck_must_explicitly_request_rr() {
        let deck = "*STEP\n*NODE PRINT,NSET=FIXED\nRF\n*END STEP\n";
        assert!(
            parse_reaction_only_rr_bytes(
                deck.as_bytes(),
                RR_ONLY_OUTPUT.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_set_is_rejected() {
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                RR_ONLY_OUTPUT.as_bytes(),
                "SUPPORTS",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn missing_unexpected_and_duplicate_nodes_are_rejected() {
        let missing = "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 0 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                missing.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        let unexpected =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 0 0 0\n2 0 0 0\n3 0 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                unexpected.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        let duplicate =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 0 0 0\n1 1 0 0\n2 0 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                duplicate.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_nonfinite_and_fortran_exponents_are_handled_explicitly() {
        let fortran = "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0D+00\n1 -1.0D+03 0.0D+00 0.0D+00\n2 1.0D+03 0.0D+00 0.0D+00\n";
        let parsed = parse_reaction_only_rr_bytes(
            DECK_WITH_RR.as_bytes(),
            fortran.as_bytes(),
            "FIXED",
            &[1, 2],
        )
        .unwrap();
        assert_eq!(parsed["time_value"], 1.0);
        let nonfinite =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 NaN 0 0\n2 1 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                nonfinite.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        let malformed =
            "reaction forces (rfx,rfy,rfz) for set FIXED and time 1.0\n1 1.0E+ 0 0\n2 1 0 0\n";
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                malformed.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
    }

    #[test]
    fn duplicate_sections_invalid_set_and_invalid_text_are_rejected() {
        let repeated = format!("{RR_ONLY_OUTPUT}\n{RR_ONLY_OUTPUT}");
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                repeated.as_bytes(),
                "FIXED",
                &[1, 2],
            )
            .is_err()
        );
        assert!(
            parse_reaction_only_rr_bytes(
                DECK_WITH_RR.as_bytes(),
                RR_ONLY_OUTPUT.as_bytes(),
                "bad set!",
                &[1, 2],
            )
            .is_err()
        );
        assert!(
            parse_reaction_only_rr_bytes(DECK_WITH_RR.as_bytes(), &[0xff, 0xfe], "FIXED", &[1, 2],)
                .is_err()
        );
    }

    fn make_normalization_bundle() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut request: Value = serde_json::from_str(include_str!(
            "../../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json"
        ))
        .unwrap();
        request["required_outputs"] = json!(["reaction_force"]);
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let request_sha = sha256_hex(&request_bytes);
        let scene_sha = request["scene_sha256"].as_str().unwrap();
        let scene_revision = request["scene_revision"].as_u64().unwrap();
        let request_id = request["request_id"].as_str().unwrap();
        let frame = request["coordinate_frame_id"].as_str().unwrap();
        let deck_bytes = b"*NODE,NSET=FIXED\n1,0,0,0\n2,2,0,0\n*STEP\n*NODE PRINT,NSET=FIXED\nRF, RR\n*END STEP\n".to_vec();
        let catalog = json!({
            "schema_version":"physical-support-node-catalog-v0.1",
            "case_id":"calculix-rr-normalization-synthetic",
            "request_id":request_id,
            "request_sha256":request_sha,
            "scene_sha256":scene_sha,
            "scene_revision":scene_revision,
            "coordinate_frame_id":frame,
            "deck_sha256":sha256_hex(&deck_bytes),
            "units":{"length":"m"},
            "nodes":[
                {"node_id":1,"position_m":[0.0,0.0,0.0]},
                {"node_id":2,"position_m":[2.0,0.0,0.0]}
            ],
            "provenance":{
                "method":"synthetic_known_node_positions",
                "identity":"two-node support fixture",
                "source_ref":"fixture-only:rr-node-catalog"
            }
        });
        let catalog_bytes = serde_json::to_vec(&catalog).unwrap();
        let manifest = json!({
            "schema_version":"calculix-rr-normalization-manifest-v0.1",
            "case_id":"calculix-rr-normalization-synthetic",
            "request_id":request_id,
            "request_sha256":request_sha,
            "scene_sha256":scene_sha,
            "scene_revision":scene_revision,
            "coordinate_frame_id":frame,
            "node_set":"FIXED",
            "deck_sha256":sha256_hex(&deck_bytes),
            "node_catalog_sha256":sha256_hex(&catalog_bytes),
            "expected_node_ids":[1,2],
            "units":{"length":"m","force":"N"},
            "provenance":{
                "method":"synthetic_hand_checked_manifest",
                "identity":"two-node reaction output normalization",
                "source_ref":"fixture-only:rr-normalization-manifest"
            }
        });
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        (
            request_bytes,
            deck_bytes,
            RR_ONLY_OUTPUT.as_bytes().to_vec(),
            manifest_bytes,
            catalog_bytes,
        )
    }

    #[test]
    fn normalizer_joins_rr_reactions_to_exact_deck_bound_node_positions() {
        let (request, deck, dat, manifest, catalog) = make_normalization_bundle();
        let report =
            normalize_reaction_only_rr_bytes(&request, &deck, &dat, &manifest, &catalog).unwrap();
        assert_eq!(report["normalization_status"], "PASS");
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
        assert_eq!(
            report["normalized_output"]["schema_version"],
            "physical-equilibrium-reaction-forces-v0.1"
        );
        assert_eq!(report["normalized_output"]["units"]["length"], "m");
        assert_eq!(report["normalized_output"]["units"]["force"], "N");
        assert_eq!(
            report["normalized_output"]["reaction_forces"][0]["reaction_id"],
            "node-1"
        );
        assert_eq!(
            report["normalized_output"]["reaction_forces"][0]["position_m"][0],
            0.0
        );
        assert_eq!(
            report["normalized_output"]["reaction_forces"][1]["position_m"][0],
            2.0
        );
        let normalized_bytes = serde_json::to_vec(&report["normalized_output"]).unwrap();
        assert_eq!(
            report["normalized_output_sha256"],
            sha256_hex(&normalized_bytes)
        );
        assert_eq!(report["deck_sha256"], sha256_hex(&deck));
        assert_eq!(report["dat_sha256"], sha256_hex(&dat));
    }

    #[test]
    fn normalizer_rejects_stale_node_catalogue_digest_and_wrong_frame() {
        let (request, deck, dat, manifest, catalog) = make_normalization_bundle();
        let mut manifest_value: Value = serde_json::from_slice(&manifest).unwrap();
        manifest_value["node_catalog_sha256"] = json!("d".repeat(64));
        let stale_manifest = serde_json::to_vec(&manifest_value).unwrap();
        assert!(
            normalize_reaction_only_rr_bytes(&request, &deck, &dat, &stale_manifest, &catalog)
                .is_err()
        );

        let mut catalog_value: Value = serde_json::from_slice(&catalog).unwrap();
        catalog_value["coordinate_frame_id"] = json!("wrong-frame");
        let wrong_catalog = serde_json::to_vec(&catalog_value).unwrap();
        manifest_value["node_catalog_sha256"] = json!(sha256_hex(&wrong_catalog));
        let manifest_for_wrong_catalog = serde_json::to_vec(&manifest_value).unwrap();
        assert!(
            normalize_reaction_only_rr_bytes(
                &request,
                &deck,
                &dat,
                &manifest_for_wrong_catalog,
                &wrong_catalog
            )
            .is_err()
        );
    }

    #[test]
    fn normalizer_rejects_non_si_request_units() {
        let (request, deck, dat, manifest, catalog) = make_normalization_bundle();
        let mut request_value: Value = serde_json::from_slice(&request).unwrap();
        request_value["units"]["force"] = json!("kN");
        let request_knew = serde_json::to_vec(&request_value).unwrap();
        let mut manifest_value: Value = serde_json::from_slice(&manifest).unwrap();
        manifest_value["request_sha256"] = json!(sha256_hex(&request_knew));
        let mut catalog_value: Value = serde_json::from_slice(&catalog).unwrap();
        catalog_value["request_sha256"] = json!(sha256_hex(&request_knew));
        let catalog_knew = serde_json::to_vec(&catalog_value).unwrap();
        manifest_value["node_catalog_sha256"] = json!(sha256_hex(&catalog_knew));
        let manifest_knew = serde_json::to_vec(&manifest_value).unwrap();
        assert!(
            normalize_reaction_only_rr_bytes(
                &request_knew,
                &deck,
                &dat,
                &manifest_knew,
                &catalog_knew
            )
            .is_err()
        );
    }

}
