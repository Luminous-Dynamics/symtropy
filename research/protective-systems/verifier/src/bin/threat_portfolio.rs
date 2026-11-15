//! Structural and decision-boundary gate for the defensive threat portfolio.
//!
//! This gate validates evidence metadata and synthetic portfolio-decision
//! controls. It does not calculate event probabilities, damage, weapon effects,
//! or operational threat values. It intentionally does not replace the robust
//! decision oracle owned by PROTECT-ROBUST-001.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::process;

const REGISTRY_SCHEMA: &str = "protective-threat-portfolio/0.1";
const CASE_SCHEMA: &str = "protective-threat-decision-cases/0.1";
const REGISTRY: &str =
    include_str!("../../../fixtures/protective-threat-family-register-v0.1.json");
const CASES: &str =
    include_str!("../../../fixtures/protective-threat-decision-cases-v0.1.json");

const DISPOSITIONS: [&str; 5] = [
    "INVEST_IMPLEMENT",
    "PILOT_VALIDATE",
    "MONITOR_PREPARE",
    "DEFER_NO_DEDICATED_INVESTMENT",
    "REJECT_INFEASIBLE",
];

#[derive(Debug, Serialize)]
struct GateReport {
    registry_schema: &'static str,
    case_schema: &'static str,
    family_count: usize,
    synthetic_case_count: usize,
    structural_checks_passed: bool,
    claim_ceiling: &'static str,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

fn required_text(value: &Value, key: &str, where_: &str) -> Result<(), String> {
    match text(value, key).map(str::trim).filter(|item| !item.is_empty()) {
        Some(_) => Ok(()),
        None => Err(format!("{where_}.{key} must be non-empty text")),
    }
}

fn reject_unqualified_numeric_risk_fields(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}.{key}");
                let lower = key.to_ascii_lowercase();
                let forbidden_numeric = [
                    "probability",
                    "event_probability",
                    "annual_probability",
                    "annual_frequency",
                    "expected_annual_loss",
                    "damage_magnitude",
                ]
                .contains(&lower.as_str());

                if forbidden_numeric && child.is_number() {
                    return Err(format!(
                        "{child_path} is numeric in v0.1; numeric risk claims require a separately sourced and context-bound schema"
                    ));
                }
                reject_unqualified_numeric_risk_fields(child, &child_path)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_unqualified_numeric_risk_fields(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_json(input: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(input).map_err(|error| format!("{label} JSON invalid: {error}"))
}

fn validate_registry(input: &str) -> Result<usize, String> {
    let root = parse_json(input, "threat registry")?;
    if text(&root, "schema_version") != Some(REGISTRY_SCHEMA) {
        return Err(format!("schema_version must equal {REGISTRY_SCHEMA}"));
    }

    let families = root
        .get("families")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "families must be a non-empty array".to_owned())?;

    let mut ids = BTreeSet::new();
    for (index, family) in families.iter().enumerate() {
        let where_ = format!("families[{index}]");
        required_text(family, "id", &where_)?;
        required_text(family, "label", &where_)?;
        required_text(family, "evidence_maturity", &where_)?;
        required_text(family, "event_frequency_context", &where_)?;
        required_text(family, "initial_priority", &where_)?;
        required_text(family, "disposition", &where_)?;
        required_text(family, "confidence", &where_)?;

        let id = text(family, "id").expect("required above");
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate threat family id: {id}"));
        }

        let disposition = text(family, "disposition").expect("required above");
        if !DISPOSITIONS.contains(&disposition) {
            return Err(format!("{where_}.disposition is unsupported: {disposition}"));
        }

        if text(family, "evidence_maturity")
            == Some("SPECULATIVE_NO_CREDIBLE_FIELDABLE_PATH_ESTABLISHED")
            && disposition != "DEFER_NO_DEDICATED_INVESTMENT"
        {
            return Err(format!(
                "{where_}: speculative family without a credible fieldable path must remain deferred"
            ));
        }

        if family
            .get("evidence_refs")
            .and_then(Value::as_array)
            .is_none()
        {
            return Err(format!("{where_}.evidence_refs must be an array"));
        }
    }

    reject_unqualified_numeric_risk_fields(&root, "registry")?;
    Ok(families.len())
}

fn validate_cases(input: &str) -> Result<usize, String> {
    let root = parse_json(input, "decision corpus")?;
    if text(&root, "schema_version") != Some(CASE_SCHEMA) {
        return Err(format!("schema_version must equal {CASE_SCHEMA}"));
    }
    if text(&root, "evidence_class") != Some("synthetic") {
        return Err("decision corpus evidence_class must equal synthetic".to_owned());
    }
    required_text(&root, "intended_use", "decision corpus")?;

    let declared = root
        .get("dispositions")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "dispositions must be a non-empty array".to_owned())?;
    let mut declared_set = BTreeSet::new();
    for item in declared {
        let name = item
            .as_str()
            .ok_or_else(|| "each disposition must be text".to_owned())?;
        if !DISPOSITIONS.contains(&name) {
            return Err(format!("unsupported declared disposition: {name}"));
        }
        if !declared_set.insert(name.to_owned()) {
            return Err(format!("duplicate declared disposition: {name}"));
        }
    }

    let cases = root
        .get("cases")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "cases must be a non-empty array".to_owned())?;

    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let where_ = format!("cases[{index}]");
        for key in [
            "case_id",
            "evidence_maturity",
            "event_frequency",
            "exposure",
            "consequence",
            "mitigation_readiness",
            "lifecycle_cost",
            "multi_hazard_cobenefit",
            "expected_disposition",
            "rationale",
        ] {
            required_text(case, key, &where_)?;
        }

        let id = text(case, "case_id").expect("required above");
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate decision case id: {id}"));
        }

        let probability_may_be_numeric = case
            .get("probability_may_be_numeric")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("{where_}.probability_may_be_numeric must be boolean"))?;
        let event_frequency = text(case, "event_frequency").expect("required above");
        if ["UNKNOWN", "NO_DEFENSIBLE_MODEL", "ORDINAL_SCORE_ONLY"].contains(&event_frequency)
            && probability_may_be_numeric
        {
            return Err(format!(
                "{where_}: numeric probability is not permitted for event_frequency={event_frequency}"
            ));
        }

        let disposition = text(case, "expected_disposition").expect("required above");
        if !declared_set.contains(disposition) {
            return Err(format!(
                "{where_}.expected_disposition is undeclared: {disposition}"
            ));
        }

        if text(case, "safety_floor_status") == Some("VIOLATED")
            && disposition != "REJECT_INFEASIBLE"
        {
            return Err(format!(
                "{where_}: a violated hard safety floor must return REJECT_INFEASIBLE"
            ));
        }

        if text(case, "multi_hazard_cobenefit") == Some("CLAIMED_BUT_OVERLAPPING")
            && disposition != "ABSTAIN_INSUFFICIENT_CONTEXT"
        {
            return Err(format!(
                "{where_}: overlapping benefits must abstain until double-counting is resolved"
            ));
        }

        if event_frequency == "ORDINAL_SCORE_ONLY"
            && disposition != "ABSTAIN_INSUFFICIENT_CONTEXT"
        {
            return Err(format!(
                "{where_}: ordinal-only scores cannot produce an investment disposition"
            ));
        }

        if text(case, "evidence_maturity")
            == Some("SPECULATIVE_NO_CREDIBLE_FIELDABLE_PATH_ESTABLISHED")
            && disposition != "DEFER_NO_DEDICATED_INVESTMENT"
        {
            return Err(format!(
                "{where_}: unsupported speculative claims must remain deferred"
            ));
        }
    }

    reject_unqualified_numeric_risk_fields(&root, "decision corpus")?;
    Ok(cases.len())
}

fn run() -> Result<GateReport, String> {
    let family_count = validate_registry(REGISTRY)?;
    let synthetic_case_count = validate_cases(CASES)?;
    Ok(GateReport {
        registry_schema: REGISTRY_SCHEMA,
        case_schema: CASE_SCHEMA,
        family_count,
        synthetic_case_count,
        structural_checks_passed: true,
        claim_ceiling: "defensive portfolio metadata and synthetic decision-boundary conformance only",
    })
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

    #[test]
    fn embedded_registry_has_ten_unique_evidence_bounded_families() {
        assert_eq!(validate_registry(REGISTRY).expect("valid registry"), 10);
    }

    #[test]
    fn embedded_synthetic_corpus_has_ten_cases() {
        assert_eq!(validate_cases(CASES).expect("valid cases"), 10);
    }

    #[test]
    fn unknown_frequency_cannot_enable_numeric_probability() {
        let mut root: Value = parse_json(CASES, "decision corpus").expect("valid JSON");
        root["cases"][5]["probability_may_be_numeric"] = Value::Bool(true);
        assert!(validate_cases(&root.to_string()).is_err());
    }

    #[test]
    fn safety_floor_violation_must_be_rejected_as_infeasible() {
        let mut root: Value = parse_json(CASES, "decision corpus").expect("valid JSON");
        root["cases"][9]["expected_disposition"] = Value::String("INVEST_IMPLEMENT".into());
        assert!(validate_cases(&root.to_string()).is_err());
    }

    #[test]
    fn overlapping_multi_hazard_benefit_must_abstain() {
        let mut root: Value = parse_json(CASES, "decision corpus").expect("valid JSON");
        root["cases"][7]["expected_disposition"] = Value::String("INVEST_IMPLEMENT".into());
        assert!(validate_cases(&root.to_string()).is_err());
    }

    #[test]
    fn ordinal_risk_scores_cannot_be_treated_as_expected_loss() {
        let mut root: Value = parse_json(CASES, "decision corpus").expect("valid JSON");
        root["cases"][8]["expected_disposition"] = Value::String("PILOT_VALIDATE".into());
        assert!(validate_cases(&root.to_string()).is_err());
    }

    #[test]
    fn exotic_claim_without_fieldable_path_must_remain_deferred() {
        let mut root: Value = parse_json(REGISTRY, "threat registry").expect("valid JSON");
        root["families"][9]["disposition"] = Value::String("INVEST_IMPLEMENT".into());
        assert!(validate_registry(&root.to_string()).is_err());
    }
}
