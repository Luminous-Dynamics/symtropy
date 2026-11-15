//! Evidence-bridge conformance gate for the defensive protective-systems portfolio.
//!
//! This binary validates synthetic boundary fixtures only. It consumes no
//! physical hazard model and does not calculate probabilities, damage or
//! real-world protective effectiveness. Upstream scenario and recovery owners
//! remain authoritative for their respective outputs.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::process;

const FIXTURE: &str =
    include_str!("../../../fixtures/protective-resilience-bridge-cases-v0.1.json");
const REGISTRY: &str =
    include_str!("../../../fixtures/protective-threat-family-register-v0.1.json");
const SCHEMA: &str = "protective-resilience-bridge-cases/0.1";

const DISPOSITIONS: [&str; 9] = [
    "FEASIBLE",
    "INFEASIBLE",
    "ABSTAIN_MISSING_UPSTREAM_EVIDENCE",
    "ABSTAIN_INCOMPARABLE_SCOPE",
    "ABSTAIN_UNSUPPORTED_PROBABILITY",
    "ABSTAIN_DOUBLE_COUNTED_BENEFIT",
    "MIXED_SERVICE_BUFFER_RECOVERY",
    "MONITOR_OR_PREPARE",
    "DEFER_NO_DEDICATED_INVESTMENT",
];

#[derive(Debug, Serialize)]
struct GateReport {
    schema_version: &'static str,
    case_count: usize,
    hazard_reference_count: usize,
    unique_hazard_family_count: usize,
    fixture_sha256: String,
    structural_checks_passed: bool,
    claim_ceiling: &'static str,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

fn required_text(value: &Value, key: &str, path: &str) -> Result<&str, String> {
    text(value, key)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn require_nested_text<'a>(
    value: &'a Value,
    parent: &str,
    key: &str,
    path: &str,
) -> Result<&'a str, String> {
    let nested = value
        .get(parent)
        .ok_or_else(|| format!("{path}.{parent} is required"))?;
    required_text(nested, key, &format!("{path}.{parent}"))
}

fn reject_unqualified_numeric_claims(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                let child_path = format!("{path}.{key}");
                let name = key.to_ascii_lowercase();
                let numeric_claim = [
                    "probability",
                    "numeric_probability",
                    "event_probability",
                    "annual_probability",
                    "annual_frequency",
                    "expected_annual_loss",
                    "annualized_loss",
                    "cost_benefit_ratio",
                    "damage_magnitude",
                ]
                .contains(&name.as_str());

                if numeric_claim && child.is_number() {
                    return Err(format!(
                        "{child_path} is numeric without a separately admitted evidence contract"
                    ));
                }
                reject_unqualified_numeric_claims(child, &child_path)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_unqualified_numeric_claims(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_case_semantics(case: &Value, path: &str, disposition: &str) -> Result<(), String> {
    let id = required_text(case, "case_id", path)?;
    match id {
        "WATER_UTILITY_SERVICE_CONTINUITY_001" => {
            if disposition != "ABSTAIN_MISSING_UPSTREAM_EVIDENCE" {
                return Err(format!("{path}: missing service/provenance receipts must abstain"));
            }
        }
        "SERVICE_RECOVERED_BUFFER_DEPLETED_002" => {
            let state = case
                .get("synthetic_state")
                .ok_or_else(|| format!("{path}.synthetic_state is required"))?;
            if text(state, "service_floor_status") != Some("PASS")
                || text(state, "sustained_service_recovery_status") != Some("PASS")
                || text(state, "reserve_condition") != Some("BELOW_DECLARED_RECOVERY_BAND")
                || disposition != "MIXED_SERVICE_BUFFER_RECOVERY"
            {
                return Err(format!(
                    "{path}: service recovery with depleted reserves must remain a mixed result"
                ));
            }
        }
        "SHARED_MULTIHAZARD_BENEFIT_SINGLE_CREDIT_003" => {
            let claims = case
                .get("benefit_claims")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("{path}.benefit_claims must be an array"))?;
            let mut seen = BTreeSet::new();
            let has_duplicate = claims.iter().any(|claim| {
                text(claim, "claim_id")
                    .map(str::to_owned)
                    .is_some_and(|id| !seen.insert(id))
            });
            if !has_duplicate || disposition != "ABSTAIN_DOUBLE_COUNTED_BENEFIT" {
                return Err(format!(
                    "{path}: duplicate shared benefit claims must abstain until reconciled"
                ));
            }
        }
        "OBSERVATION_NO_DECISION_CHANGE_004" => {
            let state = case
                .get("synthetic_state")
                .ok_or_else(|| format!("{path}.synthetic_state is required"))?;
            let safety_pass = text(state, "mandatory_human_safety_floor") == Some("PASS");
            let service_pass = text(state, "service_floor_status") == Some("PASS");
            let feasible_unchanged = state.get("feasible_set_changed").and_then(Value::as_bool)
                == Some(false);
            let action_unchanged = state.get("selected_portfolio_changed").and_then(Value::as_bool)
                == Some(false);
            let service_unchanged = state.get("service_trajectory_changed").and_then(Value::as_bool)
                == Some(false);
            if !(safety_pass
                && service_pass
                && feasible_unchanged
                && action_unchanged
                && service_unchanged
                && disposition == "FEASIBLE")
            {
                return Err(format!(
                    "{path}: a non-decision-changing observation cannot claim service benefit"
                ));
            }
        }
        "MANDATORY_HUMAN_SAFETY_FLOOR_VIOLATION_005" => {
            let state = case
                .get("synthetic_state")
                .ok_or_else(|| format!("{path}.synthetic_state is required"))?;
            if text(state, "mandatory_human_safety_floor") != Some("VIOLATED")
                || disposition != "INFEASIBLE"
            {
                return Err(format!(
                    "{path}: a violated mandatory human-safety floor must be infeasible"
                ));
            }
        }
        "UNKNOWN_FREQUENCY_NO_EXPECTED_LOSS_006" => {
            let semantics = case
                .get("scenario_semantics")
                .ok_or_else(|| format!("{path}.scenario_semantics is required"))?;
            if text(semantics, "event_frequency") != Some("UNKNOWN")
                || disposition != "ABSTAIN_UNSUPPORTED_PROBABILITY"
            {
                return Err(format!(
                    "{path}: unknown event frequency cannot support expected-loss output"
                ));
            }
        }
        _ => return Err(format!("{path}.case_id is not in the v0.1 frozen fixture set")),
    }
    Ok(())
}

fn validate(fixture: &str, registry: &str) -> Result<GateReport, String> {
    let root: Value =
        serde_json::from_str(fixture).map_err(|error| format!("fixture JSON invalid: {error}"))?;
    let registry_root: Value = serde_json::from_str(registry)
        .map_err(|error| format!("registry JSON invalid: {error}"))?;

    if text(&root, "schema_version") != Some(SCHEMA) {
        return Err(format!("schema_version must equal {SCHEMA}"));
    }
    if text(&root, "evidence_class") != Some("synthetic") {
        return Err("evidence_class must equal synthetic".to_owned());
    }

    let families = registry_root
        .get("families")
        .and_then(Value::as_array)
        .ok_or_else(|| "threat registry families must be an array".to_owned())?;
    let family_ids: BTreeSet<String> = families
        .iter()
        .filter_map(|family| text(family, "id").map(str::to_owned))
        .collect();
    if family_ids.len() != families.len() || family_ids.is_empty() {
        return Err("threat registry family IDs must be non-empty and unique".to_owned());
    }

    let owners = root
        .get("owner_contracts")
        .ok_or_else(|| "owner_contracts is required".to_owned())?;
    for key in [
        "threat_portfolio",
        "service_cascade",
        "cross_scale_resilience",
        "recovery_quality",
        "robust_decision",
        "value_of_information",
        "measurement_as_distribution",
        "power_resilience",
        "electromagnetic_pnt_resilience",
    ] {
        required_text(owners, key, "owner_contracts")?;
    }

    let declared = root
        .get("decision_dispositions")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "decision_dispositions must be a non-empty array".to_owned())?;
    let mut allowed = BTreeSet::new();
    for item in declared {
        let disposition = item
            .as_str()
            .ok_or_else(|| "each decision disposition must be text".to_owned())?;
        if !DISPOSITIONS.contains(&disposition) || !allowed.insert(disposition.to_owned()) {
            return Err(format!("unsupported or duplicate disposition: {disposition}"));
        }
    }

    let cases = root
        .get("cases")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "cases must be a non-empty array".to_owned())?;
    let mut case_ids = BTreeSet::new();
    let mut referenced_families = BTreeSet::new();
    let mut hazard_reference_count = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let path = format!("cases[{index}]");
        let case_id = required_text(case, "case_id", &path)?;
        if !case_ids.insert(case_id.to_owned()) {
            return Err(format!("{path}: duplicate case_id {case_id}"));
        }
        let disposition = required_text(case, "expected_disposition", &path)?;
        if !allowed.contains(disposition) {
            return Err(format!("{path}: undeclared disposition {disposition}"));
        }
        required_text(case, "context_id", &path)?;
        required_text(case, "description", &path)?;

        let semantics = case
            .get("scenario_semantics")
            .ok_or_else(|| format!("{path}.scenario_semantics is required"))?;
        required_text(semantics, "type", &format!("{path}.scenario_semantics"))?;
        required_text(
            semantics,
            "event_frequency",
            &format!("{path}.scenario_semantics"),
        )?;
        if semantics
            .get("not_a_probability_claim")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(format!(
                "{path}.scenario_semantics.not_a_probability_claim must be true"
            ));
        }

        let hazard_ids = case
            .get("hazard_family_ids")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .ok_or_else(|| format!("{path}.hazard_family_ids must be non-empty"))?;
        for hazard in hazard_ids {
            let id = hazard
                .as_str()
                .ok_or_else(|| format!("{path}.hazard_family_ids entries must be text"))?;
            if !family_ids.contains(id) {
                return Err(format!("{path}: unknown threat family reference {id}"));
            }
            referenced_families.insert(id.to_owned());
            hazard_reference_count += 1;
        }

        let invariants = case
            .get("expected_invariants")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .ok_or_else(|| format!("{path}.expected_invariants must be non-empty"))?;
        if invariants.iter().any(|item| item.as_str().is_none()) {
            return Err(format!("{path}.expected_invariants entries must be text"));
        }

        validate_case_semantics(case, &path, disposition)?;
    }

    if cases.len() != 6 {
        return Err(format!(
            "v0.1 requires exactly six frozen conformance cases, found {}",
            cases.len()
        ));
    }

    reject_unqualified_numeric_claims(&root, "fixture")?;

    let canonical =
        serde_json::to_vec(&root).map_err(|error| format!("fixture serialization failed: {error}"))?;
    let digest = format!("{:x}", Sha256::digest(canonical));

    Ok(GateReport {
        schema_version: SCHEMA,
        case_count: cases.len(),
        hazard_reference_count,
        unique_hazard_family_count: referenced_families.len(),
        fixture_sha256: digest,
        structural_checks_passed: true,
        claim_ceiling: "synthetic bridge-contract conformance only; no physical protection or real-world resilience claim",
    })
}

fn run() -> Result<GateReport, String> {
    validate(FIXTURE, REGISTRY)
}

fn main() {
    let report = match run() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("gate report serialization")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("embedded fixture JSON")
    }

    #[test]
    fn embedded_bridge_fixture_is_valid_and_all_hazard_refs_resolve() {
        let report = run().expect("valid embedded bridge fixture");
        assert_eq!(report.case_count, 6);
        assert_eq!(report.unique_hazard_family_count, 5);
        assert!(report.structural_checks_passed);
    }

    #[test]
    fn service_recovery_with_depleted_reserves_is_not_whole_system_recovery() {
        let mut root = fixture_value();
        root["cases"][1]["expected_disposition"] = Value::String("FEASIBLE".into());
        assert!(validate(&root.to_string(), REGISTRY).is_err());
    }

    #[test]
    fn shared_benefit_cannot_be_credited_more_than_once() {
        let mut root = fixture_value();
        root["cases"][2]["expected_disposition"] = Value::String("FEASIBLE".into());
        assert!(validate(&root.to_string(), REGISTRY).is_err());
    }

    #[test]
    fn observation_without_action_or_service_change_has_no_service_benefit() {
        let mut root = fixture_value();
        root["cases"][3]["synthetic_state"]["service_trajectory_changed"] =
            Value::Bool(true);
        assert!(validate(&root.to_string(), REGISTRY).is_err());
    }

    #[test]
    fn mandatory_human_safety_floor_cannot_be_traded_for_cost() {
        let mut root = fixture_value();
        root["cases"][4]["expected_disposition"] = Value::String("FEASIBLE".into());
        assert!(validate(&root.to_string(), REGISTRY).is_err());
    }

    #[test]
    fn unknown_event_frequency_cannot_gain_numeric_probability() {
        let mut root = fixture_value();
        root["cases"][5]["scenario_semantics"]["numeric_probability"] =
            serde_json::json!(0.01);
        assert!(validate(&root.to_string(), REGISTRY).is_err());
    }

    #[test]
    fn unknown_hazard_family_reference_fails_closed() {
        let mut root = fixture_value();
        root["cases"][0]["hazard_family_ids"][0] =
            Value::String("UNREGISTERED-HAZARD".into());
        assert!(validate(&root.to_string(), REGISTRY).is_err());
    }
}
