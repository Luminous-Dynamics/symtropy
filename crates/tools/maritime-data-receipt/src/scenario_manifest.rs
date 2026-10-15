// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Source-bound scenario evidence DAG validation.
//!
//! A valid manifest binds input receipts and raw artifacts to exact local bytes,
//! pins transformation/model revisions, and keeps assumptions explicit. It does
//! not admit operational use or establish source authenticity, scientific truth,
//! freshness, or transformation correctness.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "manifest_id",
    "purpose",
    "decision_time_utc",
    "analysis_window",
    "model",
    "nodes",
    "policy",
    "intended_use",
];
const ANALYSIS_KEYS: &[&str] = &["start_utc", "end_utc", "geographic_scope"];
const MODEL_KEYS: &[&str] = &["model_id", "revision", "config_sha256"];
const POLICY_KEYS: &[&str] = &[
    "missing_value_policy",
    "zero_fill_allowed",
    "forward_fill_allowed",
    "interpolation_allowed",
    "independent_review_required",
    "sensitivity_analysis_required",
    "input_receipts_must_be_verified",
];
const USE_KEYS: &[&str] = &[
    "scenario_only",
    "live_operational_feed",
    "navigation_authorized",
    "physical_truth_claim_authorized",
    "operational_decision_admitted",
];
const SOURCE_KEYS: &[&str] = &[
    "node_id",
    "node_kind",
    "parents",
    "evidence_class",
    "source_kind",
    "evidence_state",
    "receipt_relative_path",
    "receipt_sha256",
    "artifact_relative_path",
    "artifact_sha256",
    "data_time_start_utc",
    "data_time_end_utc",
];
const TRANSFORM_KEYS: &[&str] = &[
    "node_id",
    "node_kind",
    "parents",
    "algorithm_id",
    "algorithm_revision",
    "config_sha256",
];
const ASSUMPTION_KEYS: &[&str] = &[
    "node_id",
    "node_kind",
    "parents",
    "assumption_id",
    "statement",
    "sensitivity_status",
];
const CLAIM_TARGET_KEYS: &[&str] = &["node_id", "node_kind", "parents", "output_claim"];
const VALID: &str = "SCENARIO_MANIFEST_VALID_NOT_ADMITTED";

fn exact_keys(value: &Value, expected: &[&str]) -> Result<(), &'static str> {
    let object = value.as_object().ok_or("INVALID_OBJECT")?;
    if object.keys().any(|key| !expected.contains(&key.as_str())) {
        return Err("UNKNOWN_FIELD");
    }
    if expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("MISSING_REQUIRED_FIELD");
    }
    Ok(())
}

fn member<'a>(value: &'a Value, key: &str) -> Result<&'a Value, &'static str> {
    value.as_object().ok_or("INVALID_OBJECT")?.get(key).ok_or("MISSING_REQUIRED_FIELD")
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    member(value, key)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or("INVALID_FIELD_TYPE")
}

fn nullable_text<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, &'static str> {
    match member(value, key)? {
        Value::Null => Ok(None),
        Value::String(item) if !item.trim().is_empty() => Ok(Some(item)),
        _ => Err("INVALID_FIELD_TYPE"),
    }
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_timestamp(value: &str) -> bool {
    value.len() == 20 && super::valid_utc_timestamp(value)
}

fn valid_relative_path(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn parents(value: &Value) -> Result<Vec<String>, &'static str> {
    let items = member(value, "parents")?.as_array().ok_or("INVALID_FIELD_TYPE")?;
    if items.len() > 256 {
        return Err("TOO_MANY_DEPENDENCY_EDGES");
    }
    let mut unique = BTreeSet::new();
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        let id = item
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or("INVALID_FIELD_TYPE")?;
        if !unique.insert(id) {
            return Err("DUPLICATE_PARENT");
        }
        result.push(id.to_owned());
    }
    Ok(result)
}

#[derive(Debug, Clone)]
struct NodeFacts {
    id: String,
    kind: String,
    parents: Vec<String>,
    source_kind: Option<String>,
    evidence_state: Option<String>,
    receipt_relative_path: Option<String>,
    receipt_sha256: Option<String>,
    artifact_relative_path: Option<String>,
    artifact_sha256: Option<String>,
    data_time_start_utc: Option<String>,
    data_time_end_utc: Option<String>,
    output_claim: Option<String>,
}

fn validate_node(value: &Value) -> Result<NodeFacts, &'static str> {
    let kind = text(value, "node_kind")?.to_owned();
    let keys = match kind.as_str() {
        "source_evidence" => SOURCE_KEYS,
        "transformation" => TRANSFORM_KEYS,
        "assumption" => ASSUMPTION_KEYS,
        "claim_target" => CLAIM_TARGET_KEYS,
        _ => return Err("INVALID_NODE_KIND"),
    };
    exact_keys(value, keys)?;
    let id = text(value, "node_id")?.to_owned();
    let parents = parents(value)?;
    let mut facts = NodeFacts {
        id,
        kind: kind.clone(),
        parents,
        source_kind: None,
        evidence_state: None,
        receipt_relative_path: None,
        receipt_sha256: None,
        artifact_relative_path: None,
        artifact_sha256: None,
        data_time_start_utc: None,
        data_time_end_utc: None,
        output_claim: None,
    };

    match kind.as_str() {
        "source_evidence" => {
            if !facts.parents.is_empty() {
                return Err("SOURCE_NODE_HAS_PARENTS");
            }
            let evidence_class = text(value, "evidence_class")?;
            if ![
                "macro_connectivity",
                "port_statistics",
                "hydrography",
                "water_level",
                "current",
                "vessel_event",
                "custody_event",
                "other",
            ]
            .contains(&evidence_class)
            {
                return Err("INVALID_EVIDENCE_CLASS");
            }
            let source_kind = text(value, "source_kind")?;
            if !["synthetic_fixture", "public_dataset"].contains(&source_kind) {
                return Err("INVALID_SOURCE_KIND");
            }
            let state = text(value, "evidence_state")?;
            if ![
                "observed",
                "estimated",
                "modeled",
                "synthetic",
                "unknown",
                "disputed",
                "stale",
                "superseded",
            ]
            .contains(&state)
            {
                return Err("INVALID_EVIDENCE_STATE");
            }
            if (source_kind == "synthetic_fixture") != (state == "synthetic") {
                return Err("SOURCE_KIND_STATE_MISMATCH");
            }
            let receipt_path = text(value, "receipt_relative_path")?;
            let artifact_path = text(value, "artifact_relative_path")?;
            if !valid_relative_path(receipt_path) || !valid_relative_path(artifact_path) {
                return Err("UNSAFE_PATH");
            }
            let receipt_digest = text(value, "receipt_sha256")?;
            let artifact_digest = text(value, "artifact_sha256")?;
            if !is_lower_hex(receipt_digest, 64) || !is_lower_hex(artifact_digest, 64) {
                return Err("INVALID_DIGEST");
            }
            let start = nullable_text(value, "data_time_start_utc")?;
            let end = nullable_text(value, "data_time_end_utc")?;
            if start.is_some() != end.is_some()
                || start.is_some_and(|item| !valid_timestamp(item))
                || end.is_some_and(|item| !valid_timestamp(item))
                || start.zip(end).is_some_and(|(start, end)| start > end)
            {
                return Err("INVALID_VALIDITY_INTERVAL");
            }
            facts.source_kind = Some(source_kind.to_owned());
            facts.evidence_state = Some(state.to_owned());
            facts.receipt_relative_path = Some(receipt_path.to_owned());
            facts.receipt_sha256 = Some(receipt_digest.to_owned());
            facts.artifact_relative_path = Some(artifact_path.to_owned());
            facts.artifact_sha256 = Some(artifact_digest.to_owned());
            facts.data_time_start_utc = start.map(str::to_owned);
            facts.data_time_end_utc = end.map(str::to_owned);
        }
        "transformation" => {
            if facts.parents.is_empty() {
                return Err("TRANSFORM_REQUIRES_INPUT");
            }
            text(value, "algorithm_id")?;
            let revision = text(value, "algorithm_revision")?;
            if !is_lower_hex(revision, 40) {
                return Err("UNPINNED_ALGORITHM_REVISION");
            }
            if !is_lower_hex(text(value, "config_sha256")?, 64) {
                return Err("INVALID_DIGEST");
            }
        }
        "assumption" => {
            text(value, "assumption_id")?;
            text(value, "statement")?;
            let status = text(value, "sensitivity_status")?;
            if !["not_run", "reviewed", "passed", "failed", "not_applicable"].contains(&status) {
                return Err("INVALID_ASSUMPTION");
            }
        }
        "claim_target" => {
            if facts.parents.is_empty() {
                return Err("CLAIM_TARGET_REQUIRES_DEPENDENCY");
            }
            let claim = text(value, "output_claim")?;
            if ![
                "comparative_network_resilience",
                "continuity_scenario",
                "operational_port_state",
                "navigation_safety",
                "custody_delivery",
            ]
            .contains(&claim)
            {
                return Err("INVALID_OUTPUT_CLAIM");
            }
            facts.output_claim = Some(claim.to_owned());
        }
        _ => return Err("INVALID_NODE_KIND"),
    }
    Ok(facts)
}

#[derive(Debug)]
struct ManifestFacts {
    manifest_id: String,
    purpose: String,
    decision_time_utc: String,
    model_revision: String,
    node_count: usize,
    source_count: usize,
    transformation_count: usize,
    assumption_count: usize,
    claim_target_count: usize,
    dependency_edge_count: usize,
    unbounded_temporal_scope_source_count: usize,
    synthetic_source_count: usize,
    non_observed_source_count: usize,
}

fn validate_manifest(value: &Value) -> Result<ManifestFacts, &'static str> {
    exact_keys(value, ROOT_KEYS)?;
    if text(value, "schema_id")? != "luminous.symtropy.maritime-scenario-manifest"
        || text(value, "schema_version")? != "0.1.0"
    {
        return Err("UNSUPPORTED_MANIFEST_SCHEMA");
    }
    let manifest_id = text(value, "manifest_id")?.to_owned();
    let purpose = text(value, "purpose")?.to_owned();
    if ![
        "comparative_network_resilience",
        "continuity_scenario",
        "operational_port_state",
        "navigation_safety",
        "custody_delivery",
    ]
    .contains(&purpose.as_str())
    {
        return Err("UNSUPPORTED_PURPOSE");
    }
    let decision_time = text(value, "decision_time_utc")?;
    if !valid_timestamp(decision_time) {
        return Err("INVALID_DECISION_TIME");
    }

    let window = member(value, "analysis_window")?;
    exact_keys(window, ANALYSIS_KEYS)?;
    let start = text(window, "start_utc")?;
    let end = text(window, "end_utc")?;
    text(window, "geographic_scope")?;
    if !valid_timestamp(start) || !valid_timestamp(end) || start > end {
        return Err("INVALID_ANALYSIS_WINDOW");
    }

    let model = member(value, "model")?;
    exact_keys(model, MODEL_KEYS)?;
    text(model, "model_id")?;
    let model_revision = text(model, "revision")?;
    if !is_lower_hex(model_revision, 40) {
        return Err("UNPINNED_MODEL_REVISION");
    }
    if !is_lower_hex(text(model, "config_sha256")?, 64) {
        return Err("INVALID_DIGEST");
    }

    let policy = member(value, "policy")?;
    exact_keys(policy, POLICY_KEYS)?;
    if text(policy, "missing_value_policy")? != "preserve_unknown"
        || member(policy, "zero_fill_allowed")?.as_bool() != Some(false)
        || member(policy, "forward_fill_allowed")?.as_bool() != Some(false)
        || member(policy, "interpolation_allowed")?.as_bool() != Some(false)
        || member(policy, "independent_review_required")?.as_bool() != Some(true)
        || member(policy, "sensitivity_analysis_required")?.as_bool() != Some(true)
        || member(policy, "input_receipts_must_be_verified")?.as_bool() != Some(true)
    {
        return Err("POLICY_ESCALATION");
    }

    let intended = member(value, "intended_use")?;
    exact_keys(intended, USE_KEYS)?;
    if member(intended, "scenario_only")?.as_bool() != Some(true)
        || member(intended, "live_operational_feed")?.as_bool() != Some(false)
        || member(intended, "navigation_authorized")?.as_bool() != Some(false)
        || member(intended, "physical_truth_claim_authorized")?.as_bool() != Some(false)
        || member(intended, "operational_decision_admitted")?.as_bool() != Some(false)
    {
        return Err("AUTHORITY_OR_POLICY_ESCALATION");
    }

    let nodes = member(value, "nodes")?.as_array().ok_or("INVALID_FIELD_TYPE")?;
    if nodes.is_empty() || nodes.len() > 256 {
        return Err("INVALID_NODE_COUNT");
    }
    let mut node_facts = Vec::with_capacity(nodes.len());
    let mut ids = BTreeSet::new();
    let mut source_count = 0;
    let mut transformation_count = 0;
    let mut assumption_count = 0;
    let mut claim_target_count = 0;
    let mut unbounded_temporal_scope_source_count = 0;
    let mut synthetic_source_count = 0;
    let mut non_observed_source_count = 0;

    for node in nodes {
        let facts = validate_node(node)?;
        if !ids.insert(facts.id.clone()) {
            return Err("DUPLICATE_NODE_ID");
        }
        match facts.kind.as_str() {
            "source_evidence" => {
                source_count += 1;
                if facts.data_time_start_utc.is_none() {
                    unbounded_temporal_scope_source_count += 1;
                }
                if facts.evidence_state.as_deref() == Some("synthetic") {
                    synthetic_source_count += 1;
                }
                if facts.evidence_state.as_deref() != Some("observed") {
                    non_observed_source_count += 1;
                }
            }
            "transformation" => transformation_count += 1,
            "assumption" => assumption_count += 1,
            "claim_target" => {
                claim_target_count += 1;
                if facts.output_claim.as_deref() != Some(purpose.as_str()) {
                    return Err("OUTPUT_CLAIM_MISMATCH");
                }
            }
            _ => return Err("INVALID_NODE_KIND"),
        }
        node_facts.push(facts);
    }

    if claim_target_count == 0 {
        return Err("NO_SCENARIO_OUTPUT");
    }
    if source_count == 0 {
        return Err("NO_SOURCE_EVIDENCE");
    }
    if transformation_count == 0 {
        return Err("NO_TRANSFORMATION");
    }
    if assumption_count == 0 {
        return Err("NO_ASSUMPTION");
    }

    let indices: BTreeMap<String, usize> = node_facts
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    let mut indegree: BTreeMap<String, usize> = node_facts
        .iter()
        .map(|node| (node.id.clone(), node.parents.len()))
        .collect();
    let mut children: BTreeMap<String, Vec<String>> = node_facts
        .iter()
        .map(|node| (node.id.clone(), Vec::new()))
        .collect();
    let mut dependency_edge_count = 0_usize;

    for node in &node_facts {
        for parent in &node.parents {
            if parent == &node.id {
                return Err("SELF_DEPENDENCY");
            }
            let parent_index = *indices.get(parent).ok_or("DANGLING_PARENT")?;
            if node_facts[parent_index].kind == "claim_target" {
                return Err("CLAIM_TARGET_NOT_TERMINAL");
            }
            dependency_edge_count += 1;
            if dependency_edge_count > 2048 {
                return Err("TOO_MANY_DEPENDENCY_EDGES");
            }
            children.get_mut(parent).ok_or("DANGLING_PARENT")?.push(node.id.clone());
        }
    }

    let mut ready: VecDeque<String> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited_topological = 0_usize;
    while let Some(id) = ready.pop_front() {
        visited_topological += 1;
        if let Some(dependents) = children.get(&id) {
            for dependent in dependents {
                let degree = indegree.get_mut(dependent).ok_or("DANGLING_PARENT")?;
                *degree = degree.checked_sub(1).ok_or("DEPENDENCY_CYCLE")?;
                if *degree == 0 {
                    ready.push_back(dependent.clone());
                }
            }
        }
    }
    if visited_topological != node_facts.len() {
        return Err("DEPENDENCY_CYCLE");
    }

    let by_id: BTreeMap<&str, &NodeFacts> =
        node_facts.iter().map(|node| (node.id.as_str(), node)).collect();
    let mut reachable = BTreeSet::new();
    let mut stack: Vec<String> = node_facts
        .iter()
        .filter(|node| node.kind == "claim_target")
        .map(|node| node.id.clone())
        .collect();
    while let Some(id) = stack.pop() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        let node = by_id.get(id.as_str()).ok_or("DANGLING_PARENT")?;
        stack.extend(node.parents.iter().cloned());
    }
    if reachable.len() != node_facts.len() {
        return Err("UNUSED_NODE");
    }

    Ok(ManifestFacts {
        manifest_id,
        purpose,
        decision_time_utc: decision_time.to_owned(),
        model_revision: model_revision.to_owned(),
        node_count: node_facts.len(),
        source_count,
        transformation_count,
        assumption_count,
        claim_target_count,
        dependency_edge_count,
        unbounded_temporal_scope_source_count,
        synthetic_source_count,
        non_observed_source_count,
    })
}

fn checked_workspace_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !valid_relative_path(relative) {
        return Err(format!("unsafe workspace-relative path: {relative}"));
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve workspace root {}: {error}", root.display()))?;
    let resolved = fs::canonicalize(canonical_root.join(relative))
        .map_err(|error| format!("cannot resolve referenced file {relative}: {error}"))?;
    if !resolved.starts_with(&canonical_root) || !resolved.is_file() {
        return Err(format!("referenced file escapes workspace or is not a file: {relative}"));
    }
    Ok(resolved)
}

fn verify_source_receipts(manifest: &Value, workspace_root: &Path) -> Result<usize, String> {
    let nodes = manifest["nodes"]
        .as_array()
        .ok_or_else(|| "manifest nodes must be an array".to_owned())?;
    let mut checked = 0_usize;

    for node in nodes.iter().filter(|node| node["node_kind"] == "source_evidence") {
        let receipt_relative = node["receipt_relative_path"]
            .as_str()
            .ok_or_else(|| "source node has no receipt path".to_owned())?;
        let artifact_relative = node["artifact_relative_path"]
            .as_str()
            .ok_or_else(|| "source node has no artifact path".to_owned())?;
        let receipt_path = checked_workspace_path(workspace_root, receipt_relative)?;
        let artifact_path = checked_workspace_path(workspace_root, artifact_relative)?;
        let metadata = fs::metadata(&receipt_path)
            .map_err(|error| format!("cannot stat source receipt {receipt_relative}: {error}"))?;
        if metadata.len() > super::MAX_RECEIPT_BYTES as u64 {
            return Err(format!("source receipt exceeds bounded size: {receipt_relative}"));
        }
        let receipt_bytes = fs::read(&receipt_path)
            .map_err(|error| format!("cannot read source receipt {receipt_relative}: {error}"))?;
        super::parse_strict(&receipt_bytes)
            .map_err(|error| format!("invalid source receipt {receipt_relative}: {error}"))?;
        let receipt: Value = serde_json::from_slice(&receipt_bytes)
            .map_err(|error| format!("invalid source receipt JSON {receipt_relative}: {error}"))?;

        let expected_receipt_hash = node["receipt_sha256"]
            .as_str()
            .ok_or_else(|| format!("source node has no receipt digest: {receipt_relative}"))?;
        let actual_receipt_hash = super::sha256_bytes(&receipt_bytes);
        if actual_receipt_hash != expected_receipt_hash {
            return Err(format!("scenario manifest receipt SHA-256 mismatch: {receipt_relative}"));
        }
        if receipt["artifact"]["relative_path"].as_str() != Some(artifact_relative) {
            return Err(format!("scenario/receipt artifact path mismatch: {receipt_relative}"));
        }
        if receipt["source"]["kind"].as_str() != node["source_kind"].as_str() {
            return Err(format!("scenario/receipt source kind mismatch: {receipt_relative}"));
        }
        if receipt["data_context"]["time_coverage_start_utc"] != node["data_time_start_utc"]
            || receipt["data_context"]["time_coverage_end_utc"] != node["data_time_end_utc"]
        {
            return Err(format!("scenario/receipt data-time scope mismatch: {receipt_relative}"));
        }

        let receipt_path_display = receipt_path.to_string_lossy().into_owned();
        let report = super::verify(
            &receipt_bytes,
            &artifact_path,
            &receipt_path_display,
            "SCENARIO_MANIFEST_VALIDATOR",
        )?;
        if report.source_kind != node["source_kind"].as_str().unwrap_or_default() {
            return Err(format!("verified receipt source kind mismatch: {receipt_relative}"));
        }
        if report.receipt_sha256 != expected_receipt_hash {
            return Err(format!("verified receipt SHA-256 mismatch: {receipt_relative}"));
        }
        if report.artifact_sha256 != node["artifact_sha256"].as_str().unwrap_or_default() {
            return Err(format!("scenario manifest artifact SHA-256 mismatch: {artifact_relative}"));
        }
        checked += 1;
    }
    Ok(checked)
}

#[derive(Serialize)]
struct ManifestReport {
    status: &'static str,
    manifest_id: String,
    purpose: String,
    decision_time_utc: String,
    model_revision: String,
    node_count: usize,
    source_count: usize,
    transformation_count: usize,
    assumption_count: usize,
    claim_target_count: usize,
    dependency_edge_count: usize,
    unbounded_temporal_scope_source_count: usize,
    synthetic_source_count: usize,
    non_observed_source_count: usize,
    source_receipts_and_artifact_bytes_verified: usize,
    source_authenticity_established: bool,
    data_rows_semantically_validated: bool,
    freshness_decision_performed: bool,
    transformations_executed: bool,
    assumptions_independently_tested: bool,
    scenario_results_computed: bool,
    operational_decision_admitted: bool,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    claim_ceiling: Vec<&'static str>,
}

pub(crate) fn run_cli(manifest_path: &str) -> Result<(), String> {
    let path = PathBuf::from(manifest_path);
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("cannot stat scenario manifest {}: {error}", path.display()))?;
    if metadata.len() > super::MAX_RECEIPT_BYTES as u64 {
        return Err("scenario manifest exceeds the bounded file-size limit".to_owned());
    }
    let bytes = fs::read(&path)
        .map_err(|error| format!("cannot read scenario manifest {}: {error}", path.display()))?;
    super::parse_strict(&bytes)
        .map_err(|error| format!("invalid scenario manifest {}: {error}", path.display()))?;
    let manifest: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid scenario manifest JSON: {error}"))?;
    let facts = validate_manifest(&manifest)
        .map_err(|code| format!("scenario manifest rejected: {code}"))?;
    let workspace_root = std::env::current_dir()
        .map_err(|error| format!("cannot determine workspace root: {error}"))?;
    let checked = verify_source_receipts(&manifest, &workspace_root)?;

    if checked != facts.source_count {
        return Err("not every source receipt/artifact pair was checked".to_owned());
    }
    let report = ManifestReport {
        status: "SCENARIO_MANIFEST_VALID_AND_BYTES_BOUND_NOT_ADMITTED",
        manifest_id: facts.manifest_id,
        purpose: facts.purpose,
        decision_time_utc: facts.decision_time_utc,
        model_revision: facts.model_revision,
        node_count: facts.node_count,
        source_count: facts.source_count,
        transformation_count: facts.transformation_count,
        assumption_count: facts.assumption_count,
        claim_target_count: facts.claim_target_count,
        dependency_edge_count: facts.dependency_edge_count,
        unbounded_temporal_scope_source_count: facts.unbounded_temporal_scope_source_count,
        synthetic_source_count: facts.synthetic_source_count,
        non_observed_source_count: facts.non_observed_source_count,
        source_receipts_and_artifact_bytes_verified: checked,
        source_authenticity_established: false,
        data_rows_semantically_validated: false,
        freshness_decision_performed: false,
        transformations_executed: false,
        assumptions_independently_tested: false,
        scenario_results_computed: false,
        operational_decision_admitted: false,
        navigation_authorized: false,
        physical_truth_claim_authorized: false,
        claim_ceiling: vec![
            "Manifest graph and declared metadata contract only; transformations are pinned declarations, not executed code.",
            "Receipt and artifact checks establish exact local byte agreement with their receipts, not publisher authenticity or semantic correctness.",
            "Evidence state, time scope, and sensitivity status are declared metadata; this command does not independently attest them.",
            "Unknown or absent source time coverage remains explicitly unbounded; no freshness determination is performed.",
            "No scenario result, operational decision, navigation, physical-truth, engineering-safety, custody or delivery authority is granted.",
        ],
    };
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize scenario manifest report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
fn descend_mut<'a>(value: &'a mut Value, part: &str) -> Result<&'a mut Value, &'static str> {
    match value {
        Value::Object(object) => object.get_mut(part).ok_or("INVALID_CORPUS_MUTATION"),
        Value::Array(items) => {
            let index = part.parse::<usize>().map_err(|_| "INVALID_CORPUS_MUTATION")?;
            items.get_mut(index).ok_or("INVALID_CORPUS_MUTATION")
        }
        _ => Err("INVALID_CORPUS_MUTATION"),
    }
}

#[cfg(test)]
fn mutate_path(value: &mut Value, path: &str, mutation: &Value) -> Result<(), &'static str> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return Err("INVALID_CORPUS_MUTATION");
    }
    let mut parent = value;
    for part in &parts[..parts.len() - 1] {
        parent = descend_mut(parent, part)?;
    }
    let leaf = parts[parts.len() - 1];
    match mutation["op"].as_str().ok_or("INVALID_CORPUS_MUTATION")? {
        "set" => {
            match parent {
                Value::Object(object) => {
                    if !object.contains_key(leaf) {
                        return Err("INVALID_CORPUS_MUTATION");
                    }
                    object.insert(
                        leaf.to_owned(),
                        mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?,
                    );
                }
                Value::Array(items) => {
                    let index = leaf.parse::<usize>().map_err(|_| "INVALID_CORPUS_MUTATION")?;
                    let target = items.get_mut(index).ok_or("INVALID_CORPUS_MUTATION")?;
                    *target = mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?;
                }
                _ => return Err("INVALID_CORPUS_MUTATION"),
            }
        }
        "insert" => {
            let object = parent.as_object_mut().ok_or("INVALID_CORPUS_MUTATION")?;
            if object.contains_key(leaf) {
                return Err("INVALID_CORPUS_MUTATION");
            }
            object.insert(
                leaf.to_owned(),
                mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?,
            );
        }
        "remove" => {
            match parent {
                Value::Object(object) => {
                    if object.remove(leaf).is_none() {
                        return Err("INVALID_CORPUS_MUTATION");
                    }
                }
                Value::Array(items) => {
                    let index = leaf.parse::<usize>().map_err(|_| "INVALID_CORPUS_MUTATION")?;
                    if index >= items.len() {
                        return Err("INVALID_CORPUS_MUTATION");
                    }
                    items.remove(index);
                }
                _ => return Err("INVALID_CORPUS_MUTATION"),
            }
        }
        _ => return Err("INVALID_CORPUS_MUTATION"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn frozen_scenario_manifest_corpus_matches_expected_dispositions() {
        let corpus_text = include_str!("../tests/fixtures/scenario-manifest-corpus-v0.1.json");
        let corpus: Value =
            serde_json::from_str(corpus_text).expect("scenario manifest corpus must be valid JSON");
        assert_eq!(corpus["corpus_id"], "luminous.symtropy.maritime-scenario-manifest-corpus");
        assert_eq!(corpus["corpus_version"], "0.1.0");
        let bases = corpus["base_manifests"].as_object().expect("base manifest map");
        let cases = corpus["cases"].as_array().expect("corpus cases");
        assert_eq!(cases.len(), 24, "scenario-manifest corpus count is a contract");
        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let base = case["base"].as_str().expect("base manifest id");
            let mut candidate = bases.get(base).expect("known base manifest").clone();
            for mutation in case["mutations"].as_array().expect("mutations") {
                mutate_path(
                    &mut candidate,
                    mutation["path"].as_str().expect("mutation path"),
                    mutation,
                )
                .unwrap_or_else(|error| panic!("{id}: invalid corpus mutation: {error}"));
            }
            let actual = match validate_manifest(&candidate) {
                Ok(_) => VALID,
                Err(code) => code,
            };
            assert_eq!(case["expected_code"].as_str(), Some(actual), "{id}");
        }
    }

    #[test]
    fn schema_object_keys_match_rust_exact_key_contracts() {
        let schema_text =
            include_str!("../../../../docs/tech/MARITIME_SCENARIO_MANIFEST_V0_1.schema.json");
        let schema: Value =
            serde_json::from_str(schema_text).expect("scenario manifest schema must be valid JSON");
        let contracts = [
            (&schema["properties"], &schema["required"], ROOT_KEYS),
            (
                &schema["properties"]["analysis_window"]["properties"],
                &schema["properties"]["analysis_window"]["required"],
                ANALYSIS_KEYS,
            ),
            (
                &schema["properties"]["model"]["properties"],
                &schema["properties"]["model"]["required"],
                MODEL_KEYS,
            ),
            (
                &schema["properties"]["policy"]["properties"],
                &schema["properties"]["policy"]["required"],
                POLICY_KEYS,
            ),
            (
                &schema["properties"]["intended_use"]["properties"],
                &schema["properties"]["intended_use"]["required"],
                USE_KEYS,
            ),
            (
                &schema["$defs"]["sourceEvidenceNode"]["properties"],
                &schema["$defs"]["sourceEvidenceNode"]["required"],
                SOURCE_KEYS,
            ),
            (
                &schema["$defs"]["transformationNode"]["properties"],
                &schema["$defs"]["transformationNode"]["required"],
                TRANSFORM_KEYS,
            ),
            (
                &schema["$defs"]["assumptionNode"]["properties"],
                &schema["$defs"]["assumptionNode"]["required"],
                ASSUMPTION_KEYS,
            ),
            (
                &schema["$defs"]["claimTargetNode"]["properties"],
                &schema["$defs"]["claimTargetNode"]["required"],
                CLAIM_TARGET_KEYS,
            ),
        ];
        for (properties, required, expected_keys) in contracts {
            let mut property_names = properties
                .as_object()
                .expect("schema properties")
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            let mut required_names = required
                .as_array()
                .expect("schema required keys")
                .iter()
                .map(|item| item.as_str().expect("required key string").to_owned())
                .collect::<Vec<_>>();
            let mut expected = expected_keys
                .iter()
                .map(|key| (*key).to_owned())
                .collect::<Vec<_>>();
            property_names.sort();
            required_names.sort();
            expected.sort();
            assert_eq!(property_names, expected, "schema/Rust property drift");
            assert_eq!(required_names, expected, "schema/Rust required-key drift");
        }
        assert_eq!(
            schema["properties"]["model"]["properties"]["revision"]["pattern"],
            "^[a-f0-9]{40}$"
        );
        assert_eq!(
            schema["$defs"]["transformationNode"]["properties"]["algorithm_revision"]["pattern"],
            "^[a-f0-9]{40}$"
        );
    }

    #[test]
    fn synthetic_manifest_binds_its_receipt_and_payload_exact_bytes() {
        let fixture = include_bytes!("../tests/fixtures/synthetic-scenario-manifest.json");
        let _: super::super::JsonValue =
            super::super::parse_strict(fixture).expect("strict manifest JSON");
        let manifest: Value = serde_json::from_slice(fixture).expect("manifest fixture JSON");
        let facts = validate_manifest(&manifest).expect("valid synthetic manifest graph");
        assert_eq!(facts.node_count, 4);
        assert_eq!(facts.source_count, 1);
        assert_eq!(facts.transformation_count, 1);
        assert_eq!(facts.assumption_count, 1);
        assert_eq!(facts.claim_target_count, 1);
        assert_eq!(facts.dependency_edge_count, 3);
        assert_eq!(facts.synthetic_source_count, 1);
        assert_eq!(facts.unbounded_temporal_scope_source_count, 1);

        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        assert_eq!(
            verify_source_receipts(&manifest, &workspace).expect("exact source byte checks"),
            1
        );
    }

    #[test]
    fn scenario_manifest_rejects_receipt_and_artifact_binding_drift() {
        let fixture = include_bytes!("../tests/fixtures/synthetic-scenario-manifest.json");
        let mut manifest: Value = serde_json::from_slice(fixture).expect("manifest fixture JSON");
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");

        manifest["nodes"][0]["receipt_sha256"] =
            Value::String("0".repeat(64));
        let error = verify_source_receipts(&manifest, &workspace)
            .expect_err("a changed receipt digest must fail");
        assert!(error.contains("receipt SHA-256 mismatch"));

        let fixture = include_bytes!("../tests/fixtures/synthetic-scenario-manifest.json");
        let mut manifest: Value = serde_json::from_slice(fixture).expect("manifest fixture JSON");
        manifest["nodes"][0]["artifact_sha256"] =
            Value::String("0".repeat(64));
        let error = verify_source_receipts(&manifest, &workspace)
            .expect_err("a changed manifest artifact digest must fail");
        assert!(error.contains("artifact SHA-256 mismatch"));

        let fixture = include_bytes!("../tests/fixtures/synthetic-scenario-manifest.json");
        let mut manifest: Value = serde_json::from_slice(fixture).expect("manifest fixture JSON");
        manifest["nodes"][0]["data_time_start_utc"] =
            Value::String("2026-01-01T00:00:00Z".to_owned());
        let error = verify_source_receipts(&manifest, &workspace)
            .expect_err("invented manifest coverage must not be detached from the receipt");
        assert!(error.contains("data-time scope mismatch"));
    }
}
