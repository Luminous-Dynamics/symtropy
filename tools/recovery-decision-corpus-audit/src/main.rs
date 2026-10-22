use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-decision-scenarios-v0.1.json");

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    corpus_sha256: String,
    source_commit: Option<String>,
    scenario_count: usize,
    unique_scenario_count: usize,
    domain_profile_count: usize,
    gate_status_counts: BTreeMap<String, usize>,
    disposition_counts: BTreeMap<String, usize>,
    all_plan_only: bool,
    claim_ceiling: String,
    errors: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join("")
}

fn nonempty_string<'a>(value: Option<&'a Value>) -> Option<&'a str> {
    value.and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn is_hex_sha(value: &str, digits: usize) -> bool {
    value.len() == digits && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn count(map: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *map.entry(key.into()).or_insert(0) += 1;
}

fn make_receipt(
    raw: &[u8],
    source_commit: Option<String>,
    parsed: Option<Value>,
    parse_error: Option<String>,
) -> AuditReceipt {
    let had_parse_error = parse_error.is_some();
    let mut errors = Vec::new();
    let mut scenario_ids = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut domain_profiles = BTreeSet::new();
    let mut gate_status_counts = BTreeMap::new();
    let mut disposition_counts = BTreeMap::new();
    let mut scenario_count = 0usize;
    let mut all_plan_only = true;
    let mut schema_version = "unavailable".to_string();

    if let Some(error) = parse_error {
        errors.push(format!("CORPUS_JSON_INVALID: {error}"));
    }

    if let Some(commit) = source_commit.as_deref() {
        if !is_hex_sha(commit, 40) {
            errors.push("SOURCE_COMMIT_INVALID: expected 40 hexadecimal characters".to_string());
        }
    }

    if let Some(root) = parsed.as_ref() {
        schema_version = nonempty_string(root.get("schema_version"))
            .unwrap_or("unavailable")
            .to_string();

        if schema_version != "recovery-decision-scenarios-v0.1" {
            errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
        }
        if nonempty_string(root.get("corpus_id")) != Some("RECOVERY-ORACLE-001") {
            errors.push("CORPUS_ID_UNEXPECTED".to_string());
        }
        if nonempty_string(root.get("status")) != Some("synthetic-logic-fixtures") {
            errors.push("CORPUS_STATUS_UNEXPECTED".to_string());
        }

        let evidence_contract = root.get("evidence_contract");
        if evidence_contract.and_then(|v| v.get("synthetic_fixture_evidence_only")).and_then(Value::as_bool)
            != Some(true)
        {
            errors.push("EVIDENCE_CONTRACT_MUST_IDENTIFY_SYNTHETIC_FIXTURES".to_string());
        }
        if evidence_contract.and_then(|v| v.get("source_reference_required")).and_then(Value::as_bool)
            != Some(true)
        {
            errors.push("EVIDENCE_SOURCE_REFERENCE_MUST_BE_REQUIRED".to_string());
        }
        if evidence_contract.and_then(|v| v.get("empirical_calibration_claimed")).and_then(Value::as_bool)
            != Some(false)
        {
            errors.push("SYNTHETIC_EVIDENCE_MUST_NOT_CLAIM_CALIBRATION".to_string());
        }
        let required_fields = [
            "evidence_id", "claim", "source_ref", "method", "evidence_class",
            "uncertainty", "coverage", "freshness_class",
        ];
        let declared_fields = evidence_contract.and_then(|v| v.get("required_fields")).and_then(Value::as_array);
        if !declared_fields.map(|fields| required_fields.iter().all(|field|
            fields.iter().any(|item| item.as_str() == Some(*field))
        )).unwrap_or(false) {
            errors.push("EVIDENCE_REQUIRED_FIELDS_INCOMPLETE".to_string());
        }

        let claim_ceiling = root.get("claim_ceiling");
        if claim_ceiling.and_then(|v| v.get("expected_actuation")).and_then(Value::as_bool)
            != Some(false)
        {
            errors.push("CLAIM_CEILING_ACTUATION_MUST_BE_FALSE".to_string());
        }
        if nonempty_string(claim_ceiling.and_then(|v| v.get("execution_authority")))
            != Some("NO_ACTUATION_AUTHORIZED")
        {
            errors.push("CLAIM_CEILING_AUTHORITY_INVALID".to_string());
        }
        if claim_ceiling.and_then(|v| v.get("real_world_measurements")).and_then(Value::as_bool)
            != Some(false)
        {
            errors.push("REAL_WORLD_MEASUREMENTS_MUST_NOT_BE_CLAIMED".to_string());
        }
        if claim_ceiling.and_then(|v| v.get("field_accuracy_claimed")).and_then(Value::as_bool)
            != Some(false)
        {
            errors.push("FIELD_ACCURACY_MUST_NOT_BE_CLAIMED".to_string());
        }

        let contract = root.get("contract");
        if contract.and_then(|v| v.get("unknown_is_pass")).and_then(Value::as_bool) != Some(false) {
            errors.push("CONTRACT_UNKNOWN_MUST_NOT_EQUAL_PASS".to_string());
        }
        if contract
            .and_then(|v| v.get("failed_or_unknown_mandatory_gate_makes_affected_plan_feasible"))
            .and_then(Value::as_bool)
            != Some(false)
        {
            errors.push("CONTRACT_FAILED_OR_UNKNOWN_GATE_MUST_BLOCK_FEASIBILITY".to_string());
        }
        if contract
            .and_then(|v| v.get("domain_thresholds_transfer_between_profiles"))
            .and_then(Value::as_bool)
            != Some(false)
        {
            errors.push("CONTRACT_DOMAIN_THRESHOLDS_MUST_NOT_TRANSFER".to_string());
        }

        let scenarios = root.get("scenarios").and_then(Value::as_array);
        if scenarios.is_none() {
            errors.push("SCENARIOS_MISSING_OR_NOT_ARRAY".to_string());
        }

        if let Some(scenarios) = scenarios {
            scenario_count = scenarios.len();
            match root.get("scenario_count").and_then(Value::as_u64) {
                Some(declared) if declared as usize == scenario_count => {}
                Some(declared) => errors.push(format!(
                    "SCENARIO_COUNT_MISMATCH: declared {declared}, actual {scenario_count}"
                )),
                None => errors.push("SCENARIO_COUNT_MISSING_OR_INVALID".to_string()),
            }

            for (index, scenario) in scenarios.iter().enumerate() {
                let label = format!("SCENARIO_{}", index + 1);
                if !scenario.is_object() {
                    errors.push(format!("{label}: NOT_AN_OBJECT"));
                    continue;
                }

                let id = nonempty_string(scenario.get("id"));
                match id {
                    Some(id) => {
                        if !scenario_ids.insert(id.to_string()) {
                            errors.push(format!("{id}: DUPLICATE_SCENARIO_ID"));
                        }
                    }
                    None => errors.push(format!("{label}: ID_MISSING")),
                }

                match nonempty_string(scenario.get("domain_profile")) {
                    Some(profile) => {
                        domain_profiles.insert(profile.to_string());
                    }
                    None => errors.push(format!("{label}: DOMAIN_PROFILE_MISSING")),
                }

                let disposition = nonempty_string(scenario.get("expected_disposition"));
                let disposition_class = match disposition {
                    Some(value) if value.starts_with("BLOCK") => "blocked",
                    Some(value) if value.starts_with("HOLD_FOR") => "held",
                    Some("ELIGIBLE_FOR_HUMAN_REVIEW") => "eligible_for_review",
                    Some(value) => {
                        errors.push(format!("{label}: DISPOSITION_UNSUPPORTED: {value}"));
                        "invalid"
                    }
                    None => {
                        errors.push(format!("{label}: DISPOSITION_MISSING"));
                        "invalid"
                    }
                };
                count(&mut disposition_counts, disposition_class);

                if scenario.get("expected_actuation").and_then(Value::as_bool) != Some(false) {
                    all_plan_only = false;
                    errors.push(format!("{label}: EXPECTED_ACTUATION_MUST_BE_FALSE"));
                }

                let evidence = scenario.get("evidence").and_then(Value::as_array);
                match evidence {
                    Some(records) if !records.is_empty() => {
                        for (evidence_index, record) in records.iter().enumerate() {
                            let evidence_label = format!("{label}: EVIDENCE_{}", evidence_index + 1);
                            if !record.is_object() {
                                errors.push(format!("{evidence_label}: NOT_AN_OBJECT"));
                                continue;
                            }
                            match nonempty_string(record.get("evidence_id")) {
                                Some(evidence_id) => {
                                    if !evidence_ids.insert(evidence_id.to_string()) {
                                        errors.push(format!("{evidence_label}: DUPLICATE_EVIDENCE_ID"));
                                    }
                                }
                                None => errors.push(format!("{evidence_label}: ID_MISSING")),
                            }
                            for field in ["claim", "source_ref", "method", "evidence_class", "coverage", "freshness_class"] {
                                if nonempty_string(record.get(field)).is_none() {
                                    errors.push(format!("{evidence_label}: {}_MISSING", field.to_ascii_uppercase()));
                                }
                            }
                            if !nonempty_string(record.get("source_ref"))
                                .map(|source| source.starts_with("synthetic://"))
                                .unwrap_or(false)
                            {
                                errors.push(format!("{evidence_label}: SYNTHETIC_SOURCE_REF_INVALID"));
                            }
                            if nonempty_string(record.get("evidence_class")) != Some("synthetic_assumption") {
                                errors.push(format!("{evidence_label}: EVIDENCE_CLASS_MUST_BE_SYNTHETIC_ASSUMPTION"));
                            }
                            if nonempty_string(record.get("freshness_class")) != Some("synthetic_static") {
                                errors.push(format!("{evidence_label}: FRESHNESS_CLASS_INVALID_FOR_SYNTHETIC_CORPUS"));
                            }
                            if nonempty_string(record.get("uncertainty").and_then(|v| v.get("kind"))).is_none() {
                                errors.push(format!("{evidence_label}: UNCERTAINTY_KIND_MISSING"));
                            }
                        }
                    }
                    _ => errors.push(format!("{label}: EVIDENCE_RECORDS_MISSING_OR_EMPTY")),
                }

                let reasons = scenario.get("expected_reason_codes").and_then(Value::as_array);
                match reasons {
                    Some(reasons) if !reasons.is_empty() => {
                        for (reason_index, reason) in reasons.iter().enumerate() {
                            if nonempty_string(Some(reason)).is_none() {
                                errors.push(format!(
                                    "{label}: REASON_CODE_{}_INVALID",
                                    reason_index + 1
                                ));
                            }
                        }
                    }
                    _ => errors.push(format!("{label}: REASON_CODES_MISSING_OR_EMPTY")),
                }

                let gates = scenario.get("mandatory_gates").and_then(Value::as_array);
                let mut has_block = false;
                let mut has_unknown = false;
                if let Some(gates) = gates {
                    for (gate_index, gate) in gates.iter().enumerate() {
                        let gate_label = format!("{label}: GATE_{}", gate_index + 1);
                        if !gate.is_object() {
                            errors.push(format!("{gate_label}: NOT_AN_OBJECT"));
                            continue;
                        }
                        if nonempty_string(gate.get("gate")).is_none() {
                            errors.push(format!("{gate_label}: NAME_MISSING"));
                        }
                        let status = nonempty_string(gate.get("status"));
                        match status {
                            Some("PASS") => count(&mut gate_status_counts, "PASS"),
                            Some("BLOCK") => {
                                has_block = true;
                                count(&mut gate_status_counts, "BLOCK");
                            }
                            Some("UNKNOWN") => {
                                has_unknown = true;
                                count(&mut gate_status_counts, "UNKNOWN");
                            }
                            Some(other) => errors.push(format!(
                                "{gate_label}: STATUS_UNSUPPORTED: {other}"
                            )),
                            None => errors.push(format!("{gate_label}: STATUS_MISSING")),
                        }
                    }
                } else {
                    errors.push(format!("{label}: MANDATORY_GATES_MISSING_OR_NOT_ARRAY"));
                }

                let disposition_starts_block = disposition
                    .map(|value| value.starts_with("BLOCK"))
                    .unwrap_or(false);
                let disposition_starts_hold = disposition
                    .map(|value| value.starts_with("HOLD_FOR"))
                    .unwrap_or(false);

                if has_block && !disposition_starts_block {
                    errors.push(format!("{label}: BLOCK_GATE_NOT_REFLECTED_IN_DISPOSITION"));
                }
                if has_unknown && !disposition_starts_block && !disposition_starts_hold {
                    errors.push(format!("{label}: UNKNOWN_GATE_IMPROPERLY_ADMITTED"));
                }

                let candidates = scenario.get("candidate_plans").and_then(Value::as_array);
                match candidates {
                    Some(plans) if !plans.is_empty() => {
                        let mut plan_names = BTreeSet::new();
                        for (plan_index, plan) in plans.iter().enumerate() {
                            match nonempty_string(Some(plan)) {
                                Some(name) => {
                                    if !plan_names.insert(name.to_string()) {
                                        errors.push(format!("{label}: DUPLICATE_CANDIDATE_PLAN: {name}"));
                                    }
                                }
                                None => errors.push(format!("{label}: CANDIDATE_PLAN_{}_INVALID", plan_index + 1)),
                            }
                        }
                    }
                    Some(_) => errors.push(format!("{label}: CANDIDATE_PLANS_EMPTY")),
                    None => errors.push(format!("{label}: CANDIDATE_PLANS_MISSING_OR_NOT_ARRAY")),
                }
                if let Some(preferred) = nonempty_string(scenario.get("preferred_plan_for_review")) {
                    let found = candidates
                        .map(|plans| plans.iter().any(|plan| plan.as_str() == Some(preferred)))
                        .unwrap_or(false);
                    if !found {
                        errors.push(format!("{label}: PREFERRED_PLAN_NOT_IN_CANDIDATES"));
                    }
                    if disposition != Some("ELIGIBLE_FOR_HUMAN_REVIEW") {
                        errors.push(format!("{label}: PREFERRED_PLAN_WITHOUT_REVIEW_ELIGIBILITY"));
                    }
                }

                if let Some(voi) = scenario.get("value_of_information") {
                    let loss_avoided = voi.get("expected_decision_loss_avoided_cents").and_then(Value::as_i64);
                    let inspection_cost = voi.get("inspection_cost_cents").and_then(Value::as_i64);
                    let delay_burden = voi.get("delay_burden_cents").and_then(Value::as_i64);
                    let inspection_risk = voi.get("inspection_risk_cents").and_then(Value::as_i64);
                    let reported_net = voi.get("expected_net_value_cents").and_then(Value::as_i64);
                    let values = [loss_avoided, inspection_cost, delay_burden, inspection_risk, reported_net];
                    if values.iter().any(|value| value.is_none()) {
                        errors.push(format!("{label}: VALUE_OF_INFORMATION_FIELDS_INVALID"));
                    } else {
                        let loss_avoided = loss_avoided.unwrap();
                        let inspection_cost = inspection_cost.unwrap();
                        let delay_burden = delay_burden.unwrap();
                        let inspection_risk = inspection_risk.unwrap();
                        let reported_net = reported_net.unwrap();
                        if loss_avoided < 0 || inspection_cost < 0 || delay_burden < 0 || inspection_risk < 0 {
                            errors.push(format!("{label}: VALUE_OF_INFORMATION_BURDENS_MUST_BE_NONNEGATIVE"));
                        }
                        let computed_net = inspection_cost
                            .checked_add(delay_burden)
                            .and_then(|burden| burden.checked_add(inspection_risk))
                            .and_then(|burden| loss_avoided.checked_sub(burden));
                        match computed_net {
                            Some(net) => {
                                if net != reported_net {
                                    errors.push(format!("{label}: VALUE_OF_INFORMATION_NET_MISMATCH"));
                                }
                                let expected_action = if net > 0 {
                                    "INSPECT"
                                } else {
                                    "DO_NOT_INSPECT_ON_VALUE_ALONE"
                                };
                                if nonempty_string(voi.get("expected_information_action")) != Some(expected_action) {
                                    errors.push(format!("{label}: VALUE_OF_INFORMATION_ACTION_MISMATCH"));
                                }
                                if expected_action == "INSPECT" {
                                    let inspection_candidate_exists = candidates
                                        .map(|plans| plans.iter().any(|plan| {
                                            plan.as_str()
                                                .map(|name| name.to_ascii_lowercase().contains("inspect"))
                                                .unwrap_or(false)
                                        }))
                                        .unwrap_or(false);
                                    if !inspection_candidate_exists {
                                        errors.push(format!("{label}: POSITIVE_VOI_WITHOUT_INSPECTION_CANDIDATE"));
                                    }
                                }
                            }
                            None => errors.push(format!("{label}: VALUE_OF_INFORMATION_ARITHMETIC_OVERFLOW")),
                        }
                    }
                    if nonempty_string(voi.get("unit")) != Some("synthetic_eur_cents") {
                        errors.push(format!("{label}: VALUE_OF_INFORMATION_UNIT_INVALID"));
                    }
                    if nonempty_string(voi.get("decision_rule")).is_none() {
                        errors.push(format!("{label}: VALUE_OF_INFORMATION_DECISION_RULE_MISSING"));
                    }
                    if voi.get("calibrated_probabilities_claimed").and_then(Value::as_bool) != Some(false) {
                        errors.push(format!("{label}: SYNTHETIC_VOI_MUST_NOT_CLAIM_CALIBRATED_PROBABILITIES"));
                    }
                }

                if let Some(estimates) = scenario.get("synthetic_estimates") {
                    if !estimates.is_object() {
                        errors.push(format!("{label}: SYNTHETIC_ESTIMATES_NOT_AN_OBJECT"));
                    } else if let Some(entries) = estimates.as_object() {
                        for (plan, estimate) in entries {
                            let candidate_exists = candidates
                                .map(|plans| plans.iter().any(|candidate| candidate.as_str() == Some(plan.as_str())))
                                .unwrap_or(false);
                            if !candidate_exists {
                                errors.push(format!("{label}: ESTIMATE_FOR_UNKNOWN_CANDIDATE_PLAN: {plan}"));
                            }
                            if !estimate.is_object() {
                                errors.push(format!("{label}: ESTIMATE_NOT_AN_OBJECT: {plan}"));
                                continue;
                            }
                            match estimate.get("cost_cents_per_qualified_unit").and_then(Value::as_i64) {
                                Some(cost) if cost >= 0 => {}
                                _ => errors.push(format!("{label}: SYNTHETIC_COST_INVALID: {plan}")),
                            }
                            if let Some(yield_value) = estimate.get("qualified_yield_fraction") {
                                match yield_value.as_f64() {
                                    Some(value) if (0.0..=1.0).contains(&value) => {}
                                    _ => errors.push(format!(
                                        "{label}: SYNTHETIC_YIELD_FRACTION_INVALID: {plan}"
                                    )),
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if !had_parse_error {
        errors.push("CORPUS_VALUE_MISSING".to_string());
    }

    if scenario_count != scenario_ids.len() {
        errors.push(format!(
            "SCENARIO_ID_COVERAGE_MISMATCH: {} IDs for {scenario_count} scenarios",
            scenario_ids.len()
        ));
    }

    AuditReceipt {
        result: if errors.is_empty() { "pass" } else { "fail" }.to_string(),
        schema_version,
        corpus_sha256: sha256_hex(raw),
        source_commit,
        scenario_count,
        unique_scenario_count: scenario_ids.len(),
        domain_profile_count: domain_profiles.len(),
        gate_status_counts,
        disposition_counts,
        all_plan_only,
        claim_ceiling: "NO_ACTUATION_AUTHORIZED".to_string(),
        errors,
    }
}

fn run() -> Result<(), String> {
    let mut path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source-commit" => {
                source_commit = Some(
                    args.next()
                        .ok_or_else(|| "--source-commit requires a value".to_string())?,
                );
            }
            "--help" | "-h" => {
                println!(
                    "Usage: recovery-decision-corpus-audit <corpus.json> [--source-commit <40-hex-sha>]"
                );
                return Ok(());
            }
            _ if arg.starts_with('-') => return Err(format!("unknown argument: {arg}")),
            _ if path.is_none() => path = Some(arg),
            _ => return Err("provide exactly one corpus path".to_string()),
        }
    }

    let path = path.ok_or_else(|| {
        "missing corpus path; use --help for usage (embedded reference corpus is used by tests)"
            .to_string()
    })?;
    let bytes = fs::read(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let parsed = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => make_receipt(&bytes, source_commit, Some(value), None),
        Err(error) => make_receipt(&bytes, source_commit, None, Some(error.to_string())),
    };

    let output = serde_json::to_string_pretty(&parsed)
        .map_err(|error| format!("cannot serialize audit receipt: {error}"))?;
    println!("{output}");

    if parsed.result != "pass" {
        return Err("corpus contract audit failed".to_string());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("recovery-decision-corpus-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedded_value() -> Value {
        serde_json::from_str(EMBEDDED_CORPUS).expect("embedded corpus JSON must parse")
    }

    fn audit_value(mut value: Value) -> AuditReceipt {
        let bytes = serde_json::to_vec(&value).expect("test corpus must serialize");
        make_receipt(&bytes, None, Some(std::mem::take(&mut value)), None)
    }

    #[test]
    fn frozen_corpus_passes_and_claims_no_actuation() {
        let receipt = make_receipt(
            EMBEDDED_CORPUS.as_bytes(),
            Some("a".repeat(40)),
            Some(embedded_value()),
            None,
        );
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.scenario_count, 17);
        assert_eq!(receipt.unique_scenario_count, 17);
        assert_eq!(receipt.domain_profile_count, 8);
        assert!(receipt.all_plan_only);
        assert_eq!(receipt.claim_ceiling, "NO_ACTUATION_AUTHORIZED");
        assert_eq!(receipt.gate_status_counts.get("UNKNOWN"), Some(&19));
        assert_eq!(receipt.gate_status_counts.get("PASS"), Some(&13));
        assert_eq!(receipt.gate_status_counts.get("BLOCK"), Some(&3));
        assert_eq!(receipt.corpus_sha256.len(), 64);
    }

    #[test]
    fn unknown_gate_cannot_be_admitted_as_eligible() {
        let mut corpus = embedded_value();
        corpus["scenarios"][2]["expected_disposition"] = json!("ELIGIBLE_FOR_HUMAN_REVIEW");
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("UNKNOWN_GATE_IMPROPERLY_ADMITTED")));
    }

    #[test]
    fn block_gate_must_be_reflected_in_block_disposition() {
        let mut corpus = embedded_value();
        corpus["scenarios"][5]["expected_disposition"] = json!("ELIGIBLE_FOR_HUMAN_REVIEW");
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("BLOCK_GATE_NOT_REFLECTED_IN_DISPOSITION")));
    }

    #[test]
    fn actuation_cannot_be_enabled_by_a_fixture() {
        let mut corpus = embedded_value();
        corpus["scenarios"][0]["expected_actuation"] = json!(true);
        let receipt = audit_value(corpus);
        assert!(!receipt.all_plan_only);
        assert!(receipt.errors.iter().any(|e| e.contains("EXPECTED_ACTUATION_MUST_BE_FALSE")));
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let mut corpus = embedded_value();
        corpus["scenarios"][1]["id"] = corpus["scenarios"][0]["id"].clone();
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("DUPLICATE_SCENARIO_ID")));
    }

    #[test]
    fn preferred_plan_must_be_a_candidate() {
        let mut corpus = embedded_value();
        corpus["scenarios"][1]["preferred_plan_for_review"] = json!("invented_plan");
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("PREFERRED_PLAN_NOT_IN_CANDIDATES")));
    }

    #[test]
    fn negative_synthetic_cost_is_rejected() {
        let mut corpus = embedded_value();
        corpus["scenarios"][1]["synthetic_estimates"]["cable_extraction"]
            ["cost_cents_per_qualified_unit"] = json!(-1);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("SYNTHETIC_COST_INVALID")));
    }

    #[test]
    fn synthetic_yield_outside_unit_interval_is_rejected() {
        let mut corpus = embedded_value();
        corpus["scenarios"][1]["synthetic_estimates"]["cable_extraction"]
            ["qualified_yield_fraction"] = json!(1.2);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("SYNTHETIC_YIELD_FRACTION_INVALID")));
    }

    #[test]
    fn a_plan_cannot_be_preferred_unless_it_is_review_eligible() {
        let mut corpus = embedded_value();
        corpus["scenarios"][1]["expected_disposition"] = json!("HOLD_FOR_EVIDENCE");
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("PREFERRED_PLAN_WITHOUT_REVIEW_ELIGIBILITY")));
    }

    #[test]
    fn wrong_gate_contract_is_rejected() {
        let mut corpus = embedded_value();
        corpus["contract"]["unknown_is_pass"] = json!(true);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("CONTRACT_UNKNOWN_MUST_NOT_EQUAL_PASS")));
    }

    #[test]
    fn incorrect_declared_count_is_rejected() {
        let mut corpus = embedded_value();
        corpus["scenario_count"] = json!(13);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("SCENARIO_COUNT_MISMATCH")));
    }

    #[test]
    fn every_fixture_requires_source_coverage_and_uncertainty_metadata() {
        let mut corpus = embedded_value();
        corpus["scenarios"][0]["evidence"][0]["source_ref"] = json!("");
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("SOURCE_REF_MISSING")));
    }

    #[test]
    fn value_of_information_arithmetic_and_action_must_agree() {
        let mut corpus = embedded_value();
        corpus["scenarios"][15]["value_of_information"]["expected_net_value_cents"] = json!(999);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("VALUE_OF_INFORMATION_NET_MISMATCH")));
    }

    #[test]
    fn value_of_information_action_cannot_ignore_negative_net_value() {
        let mut corpus = embedded_value();
        corpus["scenarios"][16]["value_of_information"]["expected_information_action"] = json!("INSPECT");
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("VALUE_OF_INFORMATION_ACTION_MISMATCH")));
    }

    #[test]
    fn duplicate_evidence_ids_are_rejected() {
        let mut corpus = embedded_value();
        let duplicate = corpus["scenarios"][0]["evidence"][0]["evidence_id"].clone();
        corpus["scenarios"][1]["evidence"][0]["evidence_id"] = duplicate;
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("DUPLICATE_EVIDENCE_ID")));
    }

    #[test]
    fn empty_candidate_plan_list_is_rejected() {
        let mut corpus = embedded_value();
        corpus["scenarios"][1]["candidate_plans"] = json!([]);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("CANDIDATE_PLANS_EMPTY")));
    }

    #[test]
    fn positive_value_of_information_requires_an_inspection_option() {
        let mut corpus = embedded_value();
        corpus["scenarios"][15]["candidate_plans"] = json!(["repair", "replace", "hold"]);
        let receipt = audit_value(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("POSITIVE_VOI_WITHOUT_INSPECTION_CANDIDATE")));
    }

    #[test]
    fn source_commit_must_be_exactly_40_hex_characters() {
        let receipt = make_receipt(
            EMBEDDED_CORPUS.as_bytes(),
            Some("not-a-commit".to_string()),
            Some(embedded_value()),
            None,
        );
        assert!(receipt.errors.iter().any(|e| e.contains("SOURCE_COMMIT_INVALID")));
    }
}
