// SPDX-License-Identifier: AGPL-3.0-or-later
//! Static design-contract auditor for the WaterLoop source-separated architecture.
//!
//! This is not a hydraulic solver, potable-water quality assessment, plumbing
//! approval, or authorization to actuate equipment.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::PathBuf;

const ARCHITECTURE_SCHEMA: &str = "waterloop-reference-architecture-v0.1";
const ARCHITECTURE_ID: &str = "WATERLOOP-REFERENCE-ARCHITECTURE";
const CLAIM_CEILING: &str = "source_separation_interfaces_and_safe_state_contract_only";
const POTABLE_CIRCUIT: &str = "POTABLE";
const REUSE_STATE: &str = "REUSE_ADMITTED";

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing field: {key}"))
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{key}: expected non-empty string"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key}: expected array"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    array(value, key)?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key}[{index}]: expected string"))
        })
        .collect()
}

fn object_index<'a>(
    rows: &'a [Value],
    key: &str,
    label: &str,
) -> Result<BTreeMap<String, &'a Value>, String> {
    let mut indexed = BTreeMap::new();
    for row in rows {
        let id = text(row, key)?;
        if indexed.insert(id.clone(), row).is_some() {
            return Err(format!("duplicate {label} ID: {id}"));
        }
    }
    Ok(indexed)
}

fn path_exists(
    start: &str,
    target: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
    blocked: &BTreeSet<String>,
) -> bool {
    if blocked.contains(start) || blocked.contains(target) {
        return false;
    }
    let mut pending = VecDeque::from([start.to_owned()]);
    let mut visited = BTreeSet::from([start.to_owned()]);
    while let Some(node) = pending.pop_front() {
        if node == target {
            return true;
        }
        if let Some(neighbours) = adjacency.get(&node) {
            for neighbour in neighbours {
                if !blocked.contains(neighbour) && visited.insert(neighbour.clone()) {
                    pending.push_back(neighbour.clone());
                }
            }
        }
    }
    false
}

fn validate_circuits(architecture: &Value) -> Result<(usize, usize), String> {
    let circuits = array(architecture, "circuits")?;
    let nodes = array(architecture, "nodes")?;
    let edges = array(architecture, "edges")?;
    if circuits.len() < 3 {
        return Err("architecture must declare separate potable, greywater and roof-rainwater circuits".to_owned());
    }
    let circuit_index = object_index(circuits, "circuit_id", "circuit")?;
    let node_index = object_index(nodes, "node_id", "node")?;
    let edge_index = object_index(edges, "edge_id", "edge")?;
    let mut node_membership = BTreeMap::<String, String>::new();

    for (node_id, node) in &node_index {
        let circuit_id = text(node, "circuit_id")?;
        if !circuit_index.contains_key(&circuit_id) {
            return Err(format!("node {node_id} references unknown circuit {circuit_id}"));
        }
        node_membership.insert(node_id.clone(), circuit_id);
    }

    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for (edge_id, edge) in &edge_index {
        let circuit_id = text(edge, "circuit_id")?;
        if !circuit_index.contains_key(&circuit_id) {
            return Err(format!("edge {edge_id} references unknown circuit {circuit_id}"));
        }
        let from = text(edge, "from")?;
        let to = text(edge, "to")?;
        let from_circuit = node_membership
            .get(&from)
            .ok_or_else(|| format!("edge {edge_id} references unknown source node {from}"))?;
        let to_circuit = node_membership
            .get(&to)
            .ok_or_else(|| format!("edge {edge_id} references unknown destination node {to}"))?;
        if from_circuit != &circuit_id || to_circuit != &circuit_id {
            return Err(format!(
                "edge {edge_id} crosses a circuit boundary: {from_circuit} -> {to_circuit}, declared {circuit_id}"
            ));
        }
        adjacency.entry(from).or_default().push(to);
    }

    for (circuit_id, circuit) in &circuit_index {
        let sources = strings(circuit, "source_node_ids")?;
        let quality_gates = strings(circuit, "quality_gate_node_ids")?;
        let outlets = strings(circuit, "reuse_outlet_node_ids")?;
        let reject = text(circuit, "reject_node_id")?;
        if sources.is_empty() || quality_gates.is_empty() || outlets.is_empty() {
            return Err(format!("{circuit_id}: source, quality gate and outlet sets must be non-empty"));
        }
        let permitted = strings(circuit, "permitted_use_classes")?;
        let forbidden = strings(circuit, "forbidden_use_classes")?;
        if circuit_id != POTABLE_CIRCUIT {
            if permitted.iter().any(|use_class| forbidden.contains(use_class)) {
                return Err(format!("{circuit_id}: a use class is both permitted and forbidden"));
            }
            if permitted.iter().any(|use_class| {
                ["drinking", "food_preparation", "personal_hygiene", "bathing", "cooking"]
                    .contains(&use_class.as_str())
            }) {
                return Err(format!("{circuit_id}: non-potable circuit permits a potable use class"));
            }
        }

        for source in &sources {
            let node = node_index.get(source).ok_or_else(|| format!("{circuit_id}: unknown source node {source}"))?;
            if text(node, "circuit_id")? != *circuit_id || text(node, "role")? != "source" {
                return Err(format!("{circuit_id}: source node {source} has the wrong circuit or role"));
            }
            if !path_exists(source, &reject, &adjacency, &BTreeSet::new()) {
                return Err(format!("{circuit_id}: no fail-safe reject route from source {source}"));
            }
            for outlet in &outlets {
                let outlet_node = node_index.get(outlet).ok_or_else(|| format!("{circuit_id}: unknown outlet {outlet}"))?;
                if text(outlet_node, "circuit_id")? != *circuit_id || text(outlet_node, "role")? != "use_outlet" {
                    return Err(format!("{circuit_id}: outlet node {outlet} has the wrong circuit or role"));
                }
                if !path_exists(source, outlet, &adjacency, &BTreeSet::new()) {
                    return Err(format!("{circuit_id}: no source-to-outlet path from {source} to {outlet}"));
                }
                let blocked_gates = quality_gates.iter().cloned().collect::<BTreeSet<_>>();
                if path_exists(source, outlet, &adjacency, &blocked_gates) {
                    return Err(format!("{circuit_id}: a source-to-outlet path bypasses all declared quality gates"));
                }
                let outlet_uses = strings(outlet_node, "use_classes")?;
                for use_class in outlet_uses {
                    if !permitted.contains(&use_class) || forbidden.contains(&use_class) {
                        return Err(format!("{circuit_id}: outlet {outlet} declares unpermitted use {use_class}"));
                    }
                }
            }
        }

        for gate in &quality_gates {
            let node = node_index.get(gate).ok_or_else(|| format!("{circuit_id}: unknown quality gate {gate}"))?;
            if text(node, "circuit_id")? != *circuit_id || text(node, "role")? != "quality_gate" {
                return Err(format!("{circuit_id}: declared gate {gate} is not a quality_gate node in this circuit"));
            }
        }
        let reject_node = node_index.get(&reject).ok_or_else(|| format!("{circuit_id}: unknown reject node {reject}"))?;
        if text(reject_node, "circuit_id")? != *circuit_id || text(reject_node, "role")? != "reject_or_quarantine" && text(reject_node, "role")? != "reject_or_residual" {
            return Err(format!("{circuit_id}: reject node {reject} has the wrong circuit or role"));
        }
    }

    if !circuit_index.contains_key(POTABLE_CIRCUIT) {
        return Err("POTABLE circuit is missing".to_owned());
    }
    Ok((circuit_index.len(), edge_index.len()))
}

fn validate_controller(architecture: &Value) -> Result<usize, String> {
    let controller = field(architecture, "controller")?;
    let initial = text(controller, "initial_state")?;
    let safe_states = strings(controller, "safe_states")?;
    let states = strings(controller, "states")?;
    if initial != "BYPASS_SAFE" || !safe_states.contains(&initial) {
        return Err("initial controller state must be BYPASS_SAFE and declared safe".to_owned());
    }
    if !safe_states.contains(&"POWER_LOSS_SAFE".to_owned())
        || !safe_states.contains(&"SENSOR_OR_COMMS_LOSS_SAFE".to_owned())
        || !safe_states.contains(&"MAINTENANCE_LOCKOUT".to_owned())
    {
        return Err("power, sensor/comms and maintenance safe states must be declared".to_owned());
    }
    let transitions = array(controller, "transitions")?;
    for transition in transitions {
        let from = text(transition, "from")?;
        let to = text(transition, "to")?;
        let trigger = text(transition, "trigger")?;
        if !states.contains(&from) || !states.contains(&to) {
            return Err(format!("controller transition {from} -> {to} references an undeclared state"));
        }
        if to == REUSE_STATE {
            if from != "QUALITY_HOLD"
                || !trigger.contains("quality_gate_admitted")
                || !trigger.contains("metrology_admitted")
                || !trigger.contains("storage_age_within_qualified_limit")
            {
                return Err("REUSE_ADMITTED may only follow QUALITY_HOLD with quality, metrology and storage-age admission".to_owned());
            }
        }
        let lower = trigger.to_ascii_lowercase();
        let fault_trigger = [
            "reject", "missing", "unknown", "invalid", "blocked", "failure", "power_loss",
            "expiry", "out_of_profile", "sensor_fault"
        ]
        .iter()
        .any(|keyword| lower.contains(keyword));
        if fault_trigger && !safe_states.contains(&to) {
            return Err(format!("fault-triggered transition {from} -> {to} does not enter a declared safe state"));
        }
    }
    Ok(transitions.len())
}

fn validate(architecture: &Value) -> Result<Value, String> {
    if text(architecture, "schema_version")? != ARCHITECTURE_SCHEMA
        || text(architecture, "architecture_id")? != ARCHITECTURE_ID
        || text(architecture, "status")? != "design_candidate_not_site_approved"
        || text(architecture, "evidence_class")? != "design_contract_only"
        || text(architecture, "claim_ceiling")? != CLAIM_CEILING
    {
        return Err("architecture identity, status, evidence class or claim ceiling mismatch".to_owned());
    }
    let (circuit_count, edge_count) = validate_circuits(architecture)?;
    let transition_count = validate_controller(architecture)?;
    let nonclaims = strings(architecture, "nonclaims")?;
    if nonclaims.len() < 5 {
        return Err("architecture nonclaims are incomplete".to_owned());
    }
    Ok(json!({
        "validation_state": "ArchitectureContractPassed",
        "architecture_id": ARCHITECTURE_ID,
        "revision": text(architecture, "revision")?,
        "circuit_count": circuit_count,
        "node_count": array(architecture, "nodes")?.len(),
        "edge_count": edge_count,
        "controller_transition_count": transition_count,
        "all_edges_respect_circuit_identity": true,
        "all_reuse_paths_pass_quality_gate": true,
        "all_circuits_have_reject_paths": true,
        "fault_transitions_fail_safe": true,
        "evidence_class": "design_contract_only",
        "qualification_status": "NotRun",
        "nonclaims": nonclaims
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn run() -> Result<i32, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 2 {
        return Err("usage: waterloop-architecture-t0 [ARCHITECTURE_JSON] [RECEIPT_JSON]".to_owned());
    }
    let architecture_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-service-engineering/waterloop-reference-architecture-v0.1.json",
    ));
    let receipt_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "artifacts/waterloop-architecture-t0/receipt.json",
    ));
    let bytes = fs::read(&architecture_path)
        .map_err(|error| format!("read {}: {error}", architecture_path.display()))?;
    let architecture: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", architecture_path.display()))?;
    let validation = validate(&architecture)?;
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let revision = env::var("WATERLOOP_ARCH_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let receipt = json!({
        "schema_version": "waterloop-architecture-t0-run-v0.1",
        "verification_state": if revision.is_some() { "PassedExactRevision" } else { "PassedLocalUnpinned" },
        "implementation_revision": revision,
        "architecture_sha256": sha256(&bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "validation": validation,
        "evidence_class": "design_contract_only",
        "qualification_status": "NotRun"
    });
    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("serialize receipt: {error}"))?
        + "\n";
    fs::write(&receipt_path, serialized.as_bytes())
        .map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    println!("{serialized}");
    Ok(0)
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATERLOOP_ARCHITECTURE_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../research/water-service-engineering/waterloop-reference-architecture-v0.1.json"
        ))
        .expect("architecture fixture parses")
    }

    #[test]
    fn reference_architecture_passes_separation_and_safe_state_contract() {
        let report = validate(&fixture()).expect("reference architecture validates");
        assert_eq!(report["validation_state"], "ArchitectureContractPassed");
        assert_eq!(report["circuit_count"], 3);
        assert_eq!(report["all_reuse_paths_pass_quality_gate"], true);
        assert_eq!(report["all_circuits_have_reject_paths"], true);
    }

    #[test]
    fn cross_circuit_edge_is_rejected() {
        let mut architecture = fixture();
        architecture["edges"][5]["to"] = json!("potable-outlets");
        assert!(validate(&architecture).is_err());
    }

    #[test]
    fn nonpotable_quality_gate_bypass_is_rejected() {
        let mut architecture = fixture();
        architecture["edges"].as_array_mut().expect("edge array").push(json!({
            "edge_id": "g-negative-control",
            "circuit_id": "GREYWATER",
            "from": "greywater-treatment-core",
            "to": "greywater-nonpotable-outlets"
        }));
        assert!(validate(&architecture).is_err());
    }

    #[test]
    fn fault_transition_cannot_release_reuse_water() {
        let mut architecture = fixture();
        let transitions = architecture["controller"]["transitions"]
            .as_array_mut()
            .expect("transition array");
        let transition = transitions
            .iter_mut()
            .find(|item| item["from"] == "TREATMENT_RUN")
            .expect("treatment transition");
        transition["to"] = json!("REUSE_ADMITTED");
        transition["trigger"] = json!("process_failure_or_power_loss");
        assert!(validate(&architecture).is_err());
    }

    #[test]
    fn missing_reject_path_is_rejected() {
        let mut architecture = fixture();
        let edges = architecture["edges"].as_array_mut().expect("edge array");
        edges.retain(|edge| edge["circuit_id"] != "GREYWATER" || !edge["edge_id"].as_str().unwrap_or("").starts_with("g0"));
        assert!(validate(&architecture).is_err());
    }
}
