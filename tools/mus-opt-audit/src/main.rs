use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const DEFAULT_SCHEMA: &str =
    "schemas/manufacturing/instrument_architecture_matrix_v0_1.schema.json";
const DEFAULT_CORPUS: &str = "fixtures/manufacturing/instrument_architecture_matrix_v0_1.json";
const REQUIRED_CHALLENGES: &[&str] = &[
    "unknown_cost_treated_as_zero",
    "unapproved_substitute_treated_as_qualified",
    "simulation_metric_treated_as_physical",
    "failed_builds_omitted",
    "repair_success_without_post_test",
    "rank_without_required_evidence",
    "license_missing_but_release_allowed",
    "stale_design_revision_accepted",
];

#[derive(Debug, Serialize)]
struct AuditReport {
    report_version: &'static str,
    tool_version: &'static str,
    schema_draft: &'static str,
    report_status: &'static str,
    claim_ceiling: &'static str,
    schema_sha256: String,
    corpus_sha256: String,
    scenario_count: usize,
    baseline_count: usize,
    stress_test_count: usize,
    negative_control_count: usize,
    findings: Vec<String>,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn str_at<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn array_strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn expected_disposition(challenge: &str) -> Option<&'static str> {
    match challenge {
        "unknown_cost_treated_as_zero"
        | "unapproved_substitute_treated_as_qualified"
        | "simulation_metric_treated_as_physical"
        | "failed_builds_omitted"
        | "repair_success_without_post_test"
        | "stale_design_revision_accepted" => Some("REJECT_CLAIM"),
        "rank_without_required_evidence" => Some("BLOCK_RANKING"),
        "license_missing_but_release_allowed" => Some("BLOCK_RELEASE"),
        _ => None,
    }
}

fn semantic_findings(corpus: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    let cases = match corpus.get("scenarios").and_then(Value::as_array) {
        Some(value) => value,
        None => return vec!["$.scenarios: missing or not an array".to_owned()],
    };

    let architectures = array_strings(corpus, "architectures");
    let routes = array_strings(corpus, "manufacturing_routes");
    let definitions = match corpus.get("objective_definitions") {
        Some(value) => value,
        None => {
            findings.push("$.objective_definitions: missing".to_owned());
            return findings;
        }
    };

    let mut scenario_ids = BTreeSet::new();
    let mut baseline_pairs = BTreeSet::new();
    let mut baseline_count = 0usize;
    let mut stress_count = 0usize;
    let mut negative_count = 0usize;
    let mut challenges = BTreeSet::new();

    for (index, case) in cases.iter().enumerate() {
        let path = format!("$.scenarios[{index}]");
        let id = str_at(case, "scenario_id");
        let kind = str_at(case, "scenario_type");
        let architecture = str_at(case, "architecture");
        let route = str_at(case, "manufacturing_route");
        let condition = str_at(case, "condition");
        let challenge = str_at(case, "challenge");
        let disposition = str_at(case, "expected_disposition");

        if !scenario_ids.insert(id.to_owned()) {
            findings.push(format!("{path}.scenario_id: duplicate ID '{id}'"));
        }

        match kind {
            "baseline" => {
                baseline_count += 1;
                let pair = format!("{architecture}|{route}");
                if !baseline_pairs.insert(pair.clone()) {
                    findings.push(format!("{path}: duplicate baseline pair '{pair}'"));
                }
                if condition != "reference"
                    || challenge != "none"
                    || disposition != "UNRANKED_MISSING_EVIDENCE"
                {
                    findings.push(format!(
                        "{path}: baseline must remain reference/none/UNRANKED_MISSING_EVIDENCE"
                    ));
                }
            }
            "stress_test" => {
                stress_count += 1;
                if disposition != "BLOCKED_UNVERIFIED" {
                    findings.push(format!("{path}: stress case must remain BLOCKED_UNVERIFIED until actual inputs exist"));
                }
                if challenge != "none" {
                    findings.push(format!(
                        "{path}: stress test must not masquerade as a negative control"
                    ));
                }
            }
            "negative_control" => {
                negative_count += 1;
                if challenge == "none" {
                    findings.push(format!("{path}: negative control has no challenge"));
                }
                if !challenges.insert(challenge.to_owned()) {
                    findings.push(format!(
                        "{path}: duplicate negative-control challenge '{challenge}'"
                    ));
                }
                match expected_disposition(challenge) {
                    Some(expected) if expected != disposition => findings.push(format!(
                        "{path}: challenge '{challenge}' requires {expected}, got {disposition}"
                    )),
                    None => findings.push(format!(
                        "{path}: unknown negative-control challenge '{challenge}'"
                    )),
                    _ => {}
                }
            }
            _ => {}
        }

        let observations = match case.get("objective_evidence").and_then(Value::as_object) {
            Some(value) => value,
            None => {
                findings.push(format!(
                    "{path}.objective_evidence: missing or not an object"
                ));
                continue;
            }
        };

        for (metric_name, definition) in definitions.as_object().into_iter().flatten() {
            let expected_unit = str_at(definition, "unit");
            let expected_direction = str_at(definition, "direction");
            let metric_path = format!("{path}.objective_evidence.{metric_name}");
            let Some(observation) = observations.get(metric_name) else {
                findings.push(format!("{metric_path}: required metric missing"));
                continue;
            };
            if str_at(observation, "unit") != expected_unit {
                findings.push(format!(
                    "{metric_path}.unit: expected '{expected_unit}', got '{}'",
                    str_at(observation, "unit")
                ));
            }
            if str_at(observation, "direction") != expected_direction {
                findings.push(format!(
                    "{metric_path}.direction: expected '{expected_direction}', got '{}'",
                    str_at(observation, "direction")
                ));
            }

            let evidence_status = str_at(observation, "evidence_status");
            let value = observation.get("value");
            let source_ref = observation.get("source_ref");
            let as_of_date = observation.get("as_of_date");
            let uncertainty = observation.get("uncertainty_absolute");

            if kind == "baseline" && evidence_status != "unresolved" {
                findings.push(format!("{metric_path}: baseline objective must remain unresolved until actual source/observation evidence is attached"));
            }

            if evidence_status == "unresolved" {
                if !value.is_some_and(Value::is_null) {
                    findings.push(format!("{metric_path}.value: unresolved objective must be null, never a zero-value pass"));
                }
                if !source_ref.is_some_and(Value::is_null)
                    || !as_of_date.is_some_and(Value::is_null)
                    || !uncertainty.is_some_and(Value::is_null)
                {
                    findings.push(format!(
                        "{metric_path}: unresolved objective must have null source/date/uncertainty"
                    ));
                }
                if metric_name == "bom_cost"
                    && !observation.get("currency_code").is_some_and(Value::is_null)
                {
                    findings.push(format!(
                        "{metric_path}.currency_code: unresolved BOM cost must have null currency"
                    ));
                }
            } else if ["quoted", "observed", "derived"].contains(&evidence_status) {
                if !value.is_some_and(Value::is_number) {
                    findings.push(format!(
                        "{metric_path}.value: resolved evidence requires a numeric value"
                    ));
                }
                if !source_ref
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    findings.push(format!("{metric_path}.source_ref: resolved evidence requires a non-empty source reference"));
                }
                if !as_of_date
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    findings.push(format!(
                        "{metric_path}.as_of_date: resolved evidence requires an as-of date"
                    ));
                }
                if metric_name == "bom_cost"
                    && !observation
                        .get("currency_code")
                        .and_then(Value::as_str)
                        .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase()))
                {
                    findings.push(format!("{metric_path}.currency_code: resolved BOM cost requires an uppercase three-letter currency code"));
                }
            }
        }
    }

    if baseline_count
        != corpus
            .get("baseline_coverage_required")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX) as usize
    {
        findings.push(format!(
            "$.scenarios: baseline count {baseline_count} differs from declared required coverage"
        ));
    }
    for architecture in &architectures {
        for route in &routes {
            let pair = format!("{architecture}|{route}");
            if !baseline_pairs.contains(&pair) {
                findings.push(format!("$.scenarios: baseline pair missing '{pair}'"));
            }
        }
    }
    let expected_pair_count = architectures.len().saturating_mul(routes.len());
    if baseline_count != expected_pair_count {
        findings.push(format!("$.scenarios: expected {expected_pair_count} baseline combinations, got {baseline_count}"));
    }

    let expected_challenges: BTreeSet<String> = REQUIRED_CHALLENGES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    for missing in expected_challenges.difference(&challenges) {
        findings.push(format!(
            "$.scenarios: negative-control challenge missing '{missing}'"
        ));
    }
    for extra in challenges.difference(&expected_challenges) {
        findings.push(format!(
            "$.scenarios: unexpected negative-control challenge '{extra}'"
        ));
    }

    findings
}

fn build_report(
    schema_bytes: &[u8],
    corpus_bytes: &[u8],
    schema: &Value,
    corpus: &Value,
) -> Result<AuditReport, String> {
    jsonschema::meta::validate(schema)
        .map_err(|e| format!("schema meta-validation failed: {e}"))?;
    let validator =
        jsonschema::validator_for(schema).map_err(|e| format!("schema compilation failed: {e}"))?;
    let mut findings = validator
        .iter_errors(corpus)
        .map(|e| format!("schema validation: {e}"))
        .collect::<Vec<_>>();
    findings.extend(semantic_findings(corpus));

    let scenarios = corpus
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(AuditReport {
        report_version: "mus-opt-audit-report/0.1",
        tool_version: env!("CARGO_PKG_VERSION"),
        schema_draft: "https://json-schema.org/draft/2020-12/schema",
        report_status: if findings.is_empty() {
            "CONSISTENT_UNQUALIFIED"
        } else {
            "REJECTED"
        },
        claim_ceiling: "SYNTHETIC_SCENARIO_MATRIX_CONSISTENCY_ONLY",
        schema_sha256: sha256(schema_bytes),
        corpus_sha256: sha256(corpus_bytes),
        scenario_count: scenarios.len(),
        baseline_count: scenarios
            .iter()
            .filter(|s| str_at(s, "scenario_type") == "baseline")
            .count(),
        stress_test_count: scenarios
            .iter()
            .filter(|s| str_at(s, "scenario_type") == "stress_test")
            .count(),
        negative_control_count: scenarios
            .iter()
            .filter(|s| str_at(s, "scenario_type") == "negative_control")
            .count(),
        findings,
    })
}

fn audit(schema_path: &PathBuf, corpus_path: &PathBuf) -> Result<AuditReport, String> {
    let schema_bytes = fs::read(schema_path)
        .map_err(|e| format!("cannot read schema {}: {e}", schema_path.display()))?;
    let corpus_bytes = fs::read(corpus_path)
        .map_err(|e| format!("cannot read corpus {}: {e}", corpus_path.display()))?;
    let schema: Value = serde_json::from_slice(&schema_bytes)
        .map_err(|e| format!("schema JSON parse failed: {e}"))?;
    let corpus: Value = serde_json::from_slice(&corpus_bytes)
        .map_err(|e| format!("corpus JSON parse failed: {e}"))?;
    build_report(&schema_bytes, &corpus_bytes, &schema, &corpus)
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut schema = PathBuf::from(DEFAULT_SCHEMA);
    let mut corpus = PathBuf::from(DEFAULT_CORPUS);
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--schema" => schema = args.next().ok_or("--schema requires a path")?.into(),
            "--corpus" => corpus = args.next().ok_or("--corpus requires a path")?.into(),
            "--help" | "-h" => {
                println!(
                    "mus-opt-audit [--schema PATH] [--corpus PATH]\nAudits the MUS-OPT synthetic architecture matrix."
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((schema, corpus))
}

fn main() -> ExitCode {
    let (schema_path, corpus_path) = match parse_args() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("mus-opt-audit: {error}");
            return ExitCode::from(2);
        }
    };
    let (schema_bytes, corpus_bytes) = match (fs::read(&schema_path), fs::read(&corpus_path)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) => {
            eprintln!(
                "mus-opt-audit: cannot read schema {}: {e}",
                schema_path.display()
            );
            return ExitCode::from(2);
        }
        (_, Err(e)) => {
            eprintln!(
                "mus-opt-audit: cannot read corpus {}: {e}",
                corpus_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let schema: Value = match serde_json::from_slice(&schema_bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mus-opt-audit: schema JSON parse failed: {e}");
            return ExitCode::from(2);
        }
    };
    let corpus: Value = match serde_json::from_slice(&corpus_bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mus-opt-audit: corpus JSON parse failed: {e}");
            return ExitCode::from(2);
        }
    };
    let report = match build_report(&schema_bytes, &corpus_bytes, &schema, &corpus) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mus-opt-audit: {e}");
            return ExitCode::from(2);
        }
    };
    let rejected = !report.findings.is_empty();
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| {
            json!({"report_status":"REJECTED","findings":["report serialization failed"]})
                .to_string()
        })
    );
    if rejected {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        serde_json::from_str(include_str!(
            "../../../schemas/manufacturing/instrument_architecture_matrix_v0_1.schema.json"
        ))
        .expect("schema parses")
    }
    fn corpus() -> Value {
        serde_json::from_str(include_str!(
            "../../../fixtures/manufacturing/instrument_architecture_matrix_v0_1.json"
        ))
        .expect("corpus parses")
    }
    fn findings(data: &Value) -> Vec<String> {
        semantic_findings(data)
    }

    #[test]
    fn checked_in_schema_and_corpus_validate() {
        let s = schema();
        let c = corpus();
        jsonschema::meta::validate(&s).expect("valid JSON Schema meta-schema");
        let validator = jsonschema::validator_for(&s).expect("schema compiles");
        assert!(validator.is_valid(&c));
        assert!(findings(&c).is_empty(), "{}", findings(&c).join("\n"));
    }

    #[test]
    fn duplicate_scenario_id_is_rejected() {
        let mut data = corpus();
        let first = data["scenarios"][0].clone();
        data["scenarios"].as_array_mut().unwrap().push(first);
        assert!(findings(&data).iter().any(|f| f.contains("duplicate ID")));
    }

    #[test]
    fn missing_baseline_pair_is_rejected() {
        let mut data = corpus();
        data["scenarios"]
            .as_array_mut()
            .unwrap()
            .retain(|s| s["scenario_id"] != "BASE-001");
        assert!(
            findings(&data)
                .iter()
                .any(|f| f.contains("baseline pair missing"))
        );
    }

    #[test]
    fn baseline_objective_cannot_be_zero_if_unresolved() {
        let mut data = corpus();
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["value"] = json!(0);
        assert!(
            findings(&data)
                .iter()
                .any(|f| f.contains("unresolved objective must be null"))
        );
    }

    #[test]
    fn baseline_cannot_promote_a_metric_to_quoted_evidence() {
        let mut data = corpus();
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["value"] = json!(120);
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["evidence_status"] = json!("quoted");
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["currency_code"] = json!("ZAR");
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["source_ref"] =
            json!("quote-example");
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["as_of_date"] = json!("2026-10-18");
        assert!(
            findings(&data)
                .iter()
                .any(|f| f.contains("baseline objective must remain unresolved"))
        );
    }

    #[test]
    fn metric_unit_drift_is_rejected() {
        let mut data = corpus();
        data["scenarios"][0]["objective_evidence"]["assembly_time"]["unit"] =
            json!("currency_per_unit");
        assert!(findings(&data).iter().any(|f| f.contains("unit: expected")));
    }

    #[test]
    fn negative_control_cannot_be_marked_as_baseline_pass() {
        let mut data = corpus();
        data["scenarios"][13]["expected_disposition"] = json!("UNRANKED_MISSING_EVIDENCE");
        assert!(
            findings(&data)
                .iter()
                .any(|f| f.contains("requires REJECT_CLAIM"))
        );
    }

    #[test]
    fn resolved_bom_cost_requires_currency_and_source() {
        let mut data = corpus();
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["value"] = json!(120);
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["evidence_status"] = json!("quoted");
        data["scenarios"][0]["objective_evidence"]["bom_cost"]["currency_code"] = json!(null);
        assert!(
            findings(&data)
                .iter()
                .any(|f| f.contains("currency_code: resolved BOM cost"))
        );
    }

    #[test]
    fn negative_control_challenges_are_unique_and_complete() {
        let mut data = corpus();
        data["scenarios"][20]["challenge"] = json!("unknown_cost_treated_as_zero");
        assert!(
            findings(&data)
                .iter()
                .any(|f| f.contains("duplicate negative-control challenge"))
        );
    }
}
