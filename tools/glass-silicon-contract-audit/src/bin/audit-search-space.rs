use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const CLAIM_CEILING: &str = "Structural proposal audit only; proposed families are not admitted for physical testing, no feasibility or property model is validated, and no industrial benefit is claimed.";
const BENCHMARK_ID: &str = "glass-space-expansion-synthetic-v0-1";

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn add_finding(report: &mut Value, level: &str, code: &str, detail: &str) {
    let current = report.get("status").and_then(Value::as_str).unwrap_or("PASS");
    let next = if current == "FAIL" || level == "FAIL" {
        "FAIL"
    } else if current == "UNRESOLVED" || level == "UNRESOLVED" {
        "UNRESOLVED"
    } else {
        "PASS"
    };
    report["status"] = json!(next);
    if let Some(findings) = report.get_mut("findings").and_then(Value::as_array_mut) {
        findings.push(json!({"code": code, "detail": detail}));
    }
}

fn object_has_only(value: &Value, allowed: &[&str], path: &str, report: &mut Value) {
    let Some(map) = value.as_object() else {
        add_finding(report, "FAIL", "SCHEMA_OBJECT_REQUIRED", &format!("{path} must be a JSON object"));
        return;
    };
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            add_finding(report, "FAIL", "UNKNOWN_PROPERTY", &format!("{path}.{key} is not permitted"));
        }
    }
}

fn nonempty_string(value: &Value, field: &str, report: &mut Value) -> Option<String> {
    match value.get(field).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(s.to_owned()),
        None => {
            add_finding(report, "FAIL", "REQUIRED_STRING", &format!("{field} must be a non-empty string"));
            None
        }
    }
}

fn string_set(value: &Value, field: &str, report: &mut Value, nonempty: bool) -> HashSet<String> {
    let Some(items) = value.get(field).and_then(Value::as_array) else {
        add_finding(report, "FAIL", "ARRAY_REQUIRED", &format!("{field} must be an array of strings"));
        return HashSet::new();
    };
    if nonempty && items.is_empty() {
        add_finding(report, "UNRESOLVED", "EVIDENCE_OR_SCOPE_MISSING", &format!("{field} must not be empty"));
    }
    let mut result = HashSet::new();
    for item in items {
        let Some(s) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            add_finding(report, "FAIL", "STRING_ARRAY_ITEM_REQUIRED", &format!("{field} entries must be non-empty strings"));
            continue;
        };
        if !result.insert(s.to_owned()) {
            add_finding(report, "FAIL", "DUPLICATE_ARRAY_ITEM", &format!("{field} contains duplicate value {s}"));
        }
    }
    result
}

fn sha256_string(value: Option<&str>) -> bool {
    let Some(s) = value else { return false };
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn audit(base_raw: &[u8], proposal_raw: &[u8]) -> Value {
    let base_sha256 = sha256(base_raw);
    let proposal_sha256 = sha256(proposal_raw);
    let mut report = json!({
        "status": "PASS",
        "audit_kind": "adaptive_search_space_proposal",
        "benchmark_id": BENCHMARK_ID,
        "proposal_id": null,
        "base_space_sha256": base_sha256,
        "proposal_sha256": proposal_sha256,
        "resulting_space_sha256": null,
        "disposition": "PROPOSED_NOT_ADMITTED",
        "added_candidate_family_ids": [],
        "removed_candidate_family_ids": [],
        "candidate_family_count_before": null,
        "candidate_family_count_after": null,
        "hard_constraints_unchanged": null,
        "generation_cost_units": null,
        "generation_duration_seconds": null,
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    });

    let base: Value = match serde_json::from_slice(base_raw) {
        Ok(v) => v,
        Err(e) => {
            add_finding(&mut report, "FAIL", "BASE_JSON_INVALID", &e.to_string());
            return report;
        }
    };
    let proposal: Value = match serde_json::from_slice(proposal_raw) {
        Ok(v) => v,
        Err(e) => {
            add_finding(&mut report, "FAIL", "PROPOSAL_JSON_INVALID", &e.to_string());
            return report;
        }
    };

    object_has_only(&base, &["schema_version", "space_id", "evidence_class", "claim_ceiling", "allowed_domains", "hard_constraints", "candidate_families"], "base", &mut report);
    object_has_only(&proposal, &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "proposal_id", "parent_space_sha256", "proposer", "rationale", "applicability_domains", "add_candidate_families", "remove_candidate_family_ids", "hard_constraint_changes", "expected_decision_consequence", "generation_cost_units", "generation_duration_seconds", "evidence_refs"], "proposal", &mut report);

    if base.get("schema_version").and_then(Value::as_u64) != Some(1) {
        add_finding(&mut report, "FAIL", "BASE_SCHEMA_VERSION", "base schema_version must equal 1");
    }
    if proposal.get("schema_version").and_then(Value::as_u64) != Some(1) {
        add_finding(&mut report, "FAIL", "PROPOSAL_SCHEMA_VERSION", "proposal schema_version must equal 1");
    }
    if proposal.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID) {
        add_finding(&mut report, "FAIL", "BENCHMARK_ID_MISMATCH", "proposal benchmark_id is not the registered synthetic contract");
    }
    if base.get("evidence_class").and_then(Value::as_str) != Some("synthetic") || proposal.get("evidence_class").and_then(Value::as_str) != Some("synthetic") {
        add_finding(&mut report, "FAIL", "EVIDENCE_CLASS_MISMATCH", "this contract accepts synthetic fixtures only");
    }

    if let Some(id) = nonempty_string(&proposal, "proposal_id", &mut report) {
        if !valid_identifier(&id) {
            add_finding(&mut report, "FAIL", "INVALID_IDENTIFIER", "proposal_id must be a lowercase slug");
        }
        report["proposal_id"] = json!(id);
    }
    let _space_id = nonempty_string(&base, "space_id", &mut report);
    let _base_claim = nonempty_string(&base, "claim_ceiling", &mut report);
    let _proposal_claim = nonempty_string(&proposal, "claim_ceiling", &mut report);

    let parent = proposal.get("parent_space_sha256").and_then(Value::as_str);
    if !sha256_string(parent) {
        add_finding(&mut report, "FAIL", "PARENT_DIGEST_INVALID", "parent_space_sha256 must be a 64-character hexadecimal SHA-256 digest");
    } else if parent.unwrap().to_ascii_lowercase() != base_sha256 {
        add_finding(&mut report, "UNRESOLVED", "PARENT_SPACE_MISMATCH", "proposal parent digest does not match the exact base-space bytes supplied to this audit");
    }

    let allowed_domains = string_set(&base, "allowed_domains", &mut report, true);
    let domains = string_set(&proposal, "applicability_domains", &mut report, true);
    for domain in &domains {
        if !allowed_domains.contains(domain) {
            add_finding(&mut report, "UNRESOLVED", "DOMAIN_NOT_ADMITTED", &format!("applicability domain {domain} is not in the base space's admitted domain set"));
        }
    }
    let _rationale = nonempty_string(&proposal, "rationale", &mut report);
    let _consequence = nonempty_string(&proposal, "expected_decision_consequence", &mut report);

    // Generation must be charged even if every proposed candidate is rejected.
    for (field, code) in [("generation_cost_units", "GENERATION_COST_INVALID"), ("generation_duration_seconds", "GENERATION_DURATION_INVALID")] {
        match proposal.get(field).and_then(Value::as_f64) {
            Some(n) if n.is_finite() && n > 0.0 => report[field] = json!(n),
            _ => add_finding(&mut report, "FAIL", code, &format!("{field} must be a finite number greater than zero, including when the proposal is rejected")),
        }
    }

    let Some(proposer) = proposal.get("proposer") else {
        add_finding(&mut report, "FAIL", "PROPOSER_REQUIRED", "proposer object is required");
        return report;
    };
    object_has_only(proposer, &["kind", "id", "model_id", "config_sha256", "prompt_sha256"], "proposal.proposer", &mut report);
    let proposer_kind = proposer.get("kind").and_then(Value::as_str);
    if !matches!(proposer_kind, Some("fixed_space") | Some("random") | Some("rule_based") | Some("llm_outer_loop")) {
        add_finding(&mut report, "FAIL", "PROPOSER_KIND_INVALID", "proposer.kind must be fixed_space, random, rule_based, or llm_outer_loop");
    }
    if proposer.get("id").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).is_none() {
        add_finding(&mut report, "FAIL", "PROPOSER_ID_REQUIRED", "proposer.id must be a non-empty versioned identifier");
    }
    if proposer_kind == Some("llm_outer_loop") {
        for field in ["model_id", "config_sha256", "prompt_sha256"] {
            if proposer.get(field).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).is_none() {
                add_finding(&mut report, "UNRESOLVED", "LLM_PROVENANCE_INCOMPLETE", &format!("LLM proposer requires {field}"));
            }
        }
        for field in ["config_sha256", "prompt_sha256"] {
            if let Some(value) = proposer.get(field).and_then(Value::as_str) {
                if !sha256_string(Some(value)) {
                    add_finding(&mut report, "FAIL", "LLM_DIGEST_INVALID", &format!("proposer.{field} must be a SHA-256 digest"));
                }
            }
        }
    }

    let Some(constraints) = base.get("hard_constraints").and_then(Value::as_array) else {
        add_finding(&mut report, "FAIL", "HARD_CONSTRAINTS_REQUIRED", "base hard_constraints must be an array");
        return report;
    };
    if constraints.is_empty() {
        add_finding(&mut report, "FAIL", "HARD_CONSTRAINTS_EMPTY", "base must register at least one immutable hard constraint");
    }
    let mut base_constraints = HashSet::new();
    for (index, constraint) in constraints.iter().enumerate() {
        object_has_only(constraint, &["id", "definition"], &format!("base.hard_constraints[{index}]"), &mut report);
        let id = constraint.get("id").and_then(Value::as_str).unwrap_or("");
        let definition = constraint.get("definition").and_then(Value::as_str).unwrap_or("");
        if !valid_identifier(id) || definition.trim().is_empty() {
            add_finding(&mut report, "FAIL", "BASE_CONSTRAINT_INVALID", &format!("base hard constraint at index {index} requires a slug ID and non-empty definition"));
        }
        if !base_constraints.insert(id.to_owned()) {
            add_finding(&mut report, "FAIL", "BASE_CONSTRAINT_DUPLICATE", &format!("base hard constraint ID {id} is duplicated"));
        }
    }

    let mut existing: Map<String, Value> = Map::new();
    let Some(base_families) = base.get("candidate_families").and_then(Value::as_array) else {
        add_finding(&mut report, "FAIL", "BASE_CANDIDATE_FAMILIES_REQUIRED", "base candidate_families must be an array");
        return report;
    };
    for (index, family) in base_families.iter().enumerate() {
        object_has_only(family, &["id", "domain", "description"], &format!("base.candidate_families[{index}]"), &mut report);
        let id = family.get("id").and_then(Value::as_str).unwrap_or("");
        let domain = family.get("domain").and_then(Value::as_str).unwrap_or("");
        let description = family.get("description").and_then(Value::as_str).unwrap_or("");
        if !valid_identifier(id) || description.trim().is_empty() || !allowed_domains.contains(domain) {
            add_finding(&mut report, "FAIL", "BASE_CANDIDATE_INVALID", &format!("base candidate family at index {index} has an invalid ID, domain, or description"));
        }
        if existing.insert(id.to_owned(), family.clone()).is_some() {
            add_finding(&mut report, "FAIL", "BASE_CANDIDATE_DUPLICATE", &format!("base candidate family ID {id} is duplicated"));
        }
    }
    report["candidate_family_count_before"] = json!(existing.len());

    let constraint_changes = proposal.get("hard_constraint_changes").and_then(Value::as_array);
    match constraint_changes {
        Some(items) if !items.is_empty() => add_finding(&mut report, "FAIL", "HARD_CONSTRAINT_MUTATION_ATTEMPTED", "hard constraints are immutable within a search-space proposal; a change requires a separately reviewed contract revision"),
        None => add_finding(&mut report, "FAIL", "HARD_CONSTRAINT_CHANGES_REQUIRED", "hard_constraint_changes must be an explicit empty array"),
        _ => {}
    }
    report["hard_constraints_unchanged"] = json!(constraint_changes.is_some_and(|v| v.is_empty()));

    let mut additions = Vec::<Value>::new();
    let mut added_ids = HashSet::new();
    match proposal.get("add_candidate_families").and_then(Value::as_array) {
        Some(items) => {
            for (index, family) in items.iter().enumerate() {
                object_has_only(family, &["id", "domain", "description", "evidence_refs"], &format!("proposal.add_candidate_families[{index}]"), &mut report);
                let id = family.get("id").and_then(Value::as_str).unwrap_or("");
                let domain = family.get("domain").and_then(Value::as_str).unwrap_or("");
                let description = family.get("description").and_then(Value::as_str).unwrap_or("");
                if !valid_identifier(id) || description.trim().is_empty() {
                    add_finding(&mut report, "FAIL", "PROPOSED_CANDIDATE_INVALID", &format!("proposed candidate at index {index} requires a slug ID and non-empty description"));
                }
                if existing.contains_key(id) || !added_ids.insert(id.to_owned()) {
                    add_finding(&mut report, "FAIL", "CANDIDATE_ID_COLLISION", &format!("proposed candidate ID {id} is already present or duplicated"));
                }
                if !allowed_domains.contains(domain) {
                    add_finding(&mut report, "UNRESOLVED", "DOMAIN_NOT_ADMITTED", &format!("candidate {id} belongs to domain {domain}, which is not admitted by the base space"));
                }
                if !domains.contains(domain) {
                    add_finding(&mut report, "FAIL", "CANDIDATE_OUTSIDE_DECLARED_SCOPE", &format!("candidate {id} domain {domain} is not listed in applicability_domains"));
                }
                let evidence = family.get("evidence_refs").and_then(Value::as_array);
                if evidence.is_none_or(|v| v.is_empty() || v.iter().any(|r| r.as_str().map(str::trim).filter(|s| !s.is_empty()).is_none())) {
                    add_finding(&mut report, "UNRESOLVED", "CANDIDATE_EVIDENCE_MISSING", &format!("candidate {id} lacks usable evidence_refs"));
                }
                additions.push(json!({"id": id, "domain": domain, "description": description}));
            }
        }
        None => add_finding(&mut report, "FAIL", "CANDIDATE_ADDITIONS_REQUIRED", "add_candidate_families must be an array, empty when no additions are proposed"),
    }
    let proposal_evidence = string_set(&proposal, "evidence_refs", &mut report, true);
    if proposal_evidence.is_empty() {
        add_finding(&mut report, "UNRESOLVED", "PROPOSAL_EVIDENCE_MISSING", "proposal-level evidence provenance is missing");
    }

    let removed_set = string_set(&proposal, "remove_candidate_family_ids", &mut report, false);
    for id in &removed_set {
        if !existing.contains_key(id) {
            add_finding(&mut report, "FAIL", "REMOVE_UNKNOWN_CANDIDATE", &format!("cannot remove unknown base candidate family {id}"));
        }
        if added_ids.contains(id) {
            add_finding(&mut report, "FAIL", "CANDIDATE_ADD_REMOVE_CONFLICT", &format!("candidate family {id} cannot be added and removed in the same proposal"));
        }
        existing.remove(id);
    }
    for family in additions {
        let id = family.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
        existing.insert(id, family);
    }
    if existing.is_empty() {
        add_finding(&mut report, "FAIL", "EMPTY_SEARCH_SPACE", "proposal cannot leave zero candidate families; an empty-space result cannot count as success");
    }

    if let Some(items) = proposal.get("add_candidate_families").and_then(Value::as_array) {
        report["added_candidate_family_ids"] = json!(items.iter().filter_map(|v| v.get("id").and_then(Value::as_str)).collect::<Vec<_>>());
    }
    let mut removed: Vec<_> = removed_set.iter().cloned().collect();
    removed.sort();
    report["removed_candidate_family_ids"] = json!(removed);
    report["candidate_family_count_after"] = json!(existing.len());

    if report["status"] == "PASS" {
        let mut resulting = base.clone();
        if let Some(object) = resulting.as_object_mut() {
            let mut families: Vec<Value> = existing.into_iter().map(|(_, value)| value).collect();
            families.sort_by(|a, b| a.get("id").and_then(Value::as_str).cmp(&b.get("id").and_then(Value::as_str)));
            object.insert("candidate_families".to_owned(), Value::Array(families));
        }
        let normalized = serde_json::to_vec(&resulting).unwrap_or_default();
        report["resulting_space_sha256"] = json!(sha256(&normalized));
        report["disposition"] = json!("PROPOSED_NOT_ADMITTED");
    } else if report["status"] == "UNRESOLVED" {
        report["disposition"] = json!("PENDING_EVIDENCE_OR_DOMAIN_REVIEW");
    } else {
        report["disposition"] = json!("REJECTED_CONTRACT_VIOLATION");
    }
    report
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn usage() {
    eprintln!("Usage: audit-search-space <base-space.json> <proposal.json>");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        usage();
        return ExitCode::from(64);
    }
    let base = match read_bytes(&args[1]) {
        Ok(v) => v,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(2); }
    };
    let proposal = match read_bytes(&args[2]) {
        Ok(v) => v,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(2); }
    };
    let report = audit(&base, &proposal);
    let exit_code = match report.get("status").and_then(Value::as_str) {
        Some("PASS") => 0,
        Some("UNRESOLVED") => 3,
        _ => 2,
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("JSON report serializes"));
    ExitCode::from(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &[u8] = include_bytes!("../../fixtures/base-space.synthetic.json");
    const VALID: &[u8] = include_bytes!("../../fixtures/space-proposal-helpful.synthetic.json");

    fn mutated<F>(mutation: F) -> Vec<u8>
    where
        F: FnOnce(&mut Value),
    {
        let mut proposal: Value = serde_json::from_slice(VALID).unwrap();
        proposal["parent_space_sha256"] = json!(sha256(BASE));
        mutation(&mut proposal);
        serde_json::to_vec(&proposal).unwrap()
    }

    #[test]
    fn synthetic_known_answer_is_proposed_not_admitted() {
        let report = audit(BASE, VALID);
        assert_eq!(report["status"], "PASS");
        assert_eq!(report["disposition"], "PROPOSED_NOT_ADMITTED");
        assert_eq!(report["candidate_family_count_before"], 2);
        assert_eq!(report["candidate_family_count_after"], 3);
        assert_eq!(report["hard_constraints_unchanged"], true);
        assert_eq!(report["base_space_sha256"], sha256(BASE));
        assert_eq!(report["proposal_sha256"], sha256(VALID));
        assert!(report["resulting_space_sha256"].as_str().is_some_and(|s| s.len() == 64));
    }

    #[test]
    fn hard_constraint_weakening_is_a_hard_failure() {
        let bytes = mutated(|p| p["hard_constraint_changes"] = json!([{"constraint_id":"accepted-grade-output","operation":"weaken","replacement_definition":"Any output counts"}]));
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "HARD_CONSTRAINT_MUTATION_ATTEMPTED"));
        assert_eq!(report["disposition"], "REJECTED_CONTRACT_VIOLATION");
    }

    #[test]
    fn stale_parent_is_unresolved_not_silently_rebased() {
        let bytes = mutated(|p| p["parent_space_sha256"] = json!("0".repeat(64)));
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "UNRESOLVED");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "PARENT_SPACE_MISMATCH"));
    }

    #[test]
    fn missing_evidence_is_unresolved() {
        let bytes = mutated(|p| {
            p["add_candidate_families"][0]["evidence_refs"] = json!([]);
            p["evidence_refs"] = json!([]);
        });
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "UNRESOLVED");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "CANDIDATE_EVIDENCE_MISSING"));
    }

    #[test]
    fn unknown_domain_stays_unresolved() {
        let bytes = mutated(|p| {
            p["applicability_domains"] = json!(["glass", "unknown-refractory"]);
            p["add_candidate_families"][0]["domain"] = json!("unknown-refractory");
        });
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "UNRESOLVED");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "DOMAIN_NOT_ADMITTED"));
    }

    #[test]
    fn sealed_outcome_in_proposal_is_rejected() {
        let bytes = mutated(|p| p["sealed_outcomes"] = json!({"candidate-glass-family-a": 1.0}));
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "UNKNOWN_PROPERTY"));
    }

    #[test]
    fn reject_all_shortcut_is_rejected() {
        let bytes = mutated(|p| {
            p["add_candidate_families"] = json!([]);
            p["remove_candidate_family_ids"] = json!(["baseline-glass-family", "baseline-silicon-family"]);
        });
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "EMPTY_SEARCH_SPACE"));
    }

    #[test]
    fn receipt_is_deterministic_on_exact_inputs() {
        assert_eq!(audit(BASE, VALID), audit(BASE, VALID));
    }

    #[test]
    fn invalid_budget_cost_is_rejected() {
        let bytes = mutated(|p| p["generation_cost_units"] = json!(0));
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "GENERATION_COST_INVALID"));
    }

    #[test]
    fn duplicate_proposed_candidate_is_rejected() {
        let bytes = mutated(|p| {
            let first = p["add_candidate_families"][0].clone();
            p["add_candidate_families"].as_array_mut().unwrap().push(first);
        });
        let report = audit(BASE, &bytes);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|x| x["code"] == "CANDIDATE_ID_COLLISION"));
    }
}
