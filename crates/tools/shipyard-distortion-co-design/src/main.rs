use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "SHIPYARD-DISTORTION-CO-DESIGN-001";
const SCHEMA_VERSION: &str = "0.1.0";
const DEFAULT_CORPUS: &str = "benchmarks/shipyard/distortion-co-design/corpus-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/tech/SHIPYARD_DISTORTION_CO_DESIGN_V0_1.schema.json";

#[derive(Debug, PartialEq, Eq)]
struct Summary {
    cases: usize,
    positive: usize,
    negative: usize,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn number_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn string_list(value: &Value, key: &str) -> Option<Vec<&str>> {
    value
        .get(key)?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect()
}

fn evaluate_case(case: &Value) -> &'static str {
    for field in ["panel_id", "panel_revision"] {
        if str_at(case, field).is_none_or(str::is_empty) {
            return "BLOCK_CONFIGURATION";
        }
    }
    for field in ["fixture_id", "fixture_revision"] {
        if str_at(case, field).is_none_or(str::is_empty) {
            return "BLOCK_FIXTURE_LIMIT";
        }
    }

    let Some(tasks) = string_list(case, "weld_tasks") else {
        return "BLOCK_SEQUENCE";
    };
    let Some(sequence) = string_list(case, "sequence") else {
        return "BLOCK_SEQUENCE";
    };

    let task_set: HashSet<&str> = tasks.iter().copied().collect();
    let sequence_set: HashSet<&str> = sequence.iter().copied().collect();
    if tasks.len() < 2
        || task_set.len() != tasks.len()
        || sequence.len() != tasks.len()
        || sequence_set.len() != sequence.len()
        || sequence_set != task_set
    {
        return "BLOCK_SEQUENCE";
    }

    let positions: HashMap<&str, usize> = sequence
        .iter()
        .enumerate()
        .map(|(index, task)| (*task, index))
        .collect();

    let Some(precedence) = case.get("precedence").and_then(Value::as_array) else {
        return "BLOCK_SEQUENCE";
    };
    for edge in precedence {
        let Some(pair) = edge.as_array() else {
            return "BLOCK_SEQUENCE";
        };
        if pair.len() != 2 {
            return "BLOCK_SEQUENCE";
        }
        let (Some(before), Some(after)) = (pair[0].as_str(), pair[1].as_str()) else {
            return "BLOCK_SEQUENCE";
        };
        let (Some(before_position), Some(after_position)) =
            (positions.get(before), positions.get(after))
        else {
            return "BLOCK_SEQUENCE";
        };
        if before_position >= after_position {
            return "BLOCK_SEQUENCE";
        }
    }

    let (Some(limit), Some(predicted)) = (
        number_at(case, "fixture_reaction_limit_units"),
        number_at(case, "predicted_fixture_reaction_units"),
    ) else {
        return "BLOCK_FIXTURE_LIMIT";
    };
    if limit < 0 || predicted < 0 || predicted > limit {
        return "BLOCK_FIXTURE_LIMIT";
    }

    let Some(release_events) = case.get("release_events").and_then(Value::as_array) else {
        return "BLOCK_RELEASE_STATE";
    };
    let Some(last_release) = release_events.last() else {
        return "BLOCK_RELEASE_STATE";
    };
    let mut previous_order = None;
    for event in release_events {
        let Some(order) = number_at(event, "event_order") else {
            return "BLOCK_RELEASE_STATE";
        };
        if previous_order.is_some_and(|previous| order <= previous) {
            return "BLOCK_RELEASE_STATE";
        }
        previous_order = Some(order);
    }
    if bool_at(last_release, "cooling_complete") != Some(true)
        || bool_at(last_release, "all_restraints_removed") != Some(true)
        || str_at(last_release, "state_after") != Some("fully_released")
    {
        return "BLOCK_RELEASE_STATE";
    }

    let Some(measurement) = case.get("final_measurement").filter(|v| !v.is_null()) else {
        return "BLOCK_FINAL_MEASUREMENT";
    };
    if str_at(measurement, "panel_id") != str_at(case, "panel_id")
        || str_at(measurement, "panel_revision") != str_at(case, "panel_revision")
        || str_at(measurement, "fixture_id") != str_at(case, "fixture_id")
        || str_at(measurement, "phase") != Some("final_free_state")
        || bool_at(measurement, "cooling_complete") != Some(true)
        || bool_at(measurement, "restraints_removed") != Some(true)
        || str_at(measurement, "datum_id").is_none_or(str::is_empty)
        || str_at(measurement, "coordinate_frame").is_none_or(str::is_empty)
        || bool_at(measurement, "uncertainty_present") != Some(true)
        || str_at(measurement, "evidence_class") != Some("synthetic_contract")
    {
        return "BLOCK_FINAL_MEASUREMENT";
    }

    let Some(model) = case.get("model") else {
        return "BLOCK_MODEL_SCOPE";
    };
    if str_at(model, "model_id").is_none_or(str::is_empty)
        || str_at(model, "model_revision").is_none_or(str::is_empty)
        || str_at(model, "scope") != Some("synthetic_toy")
        || str_at(model, "validation_state") != Some("synthetic_contract_only")
        || bool_at(model, "out_of_domain") != Some(false)
        || bool_at(model, "physical_claim") != Some(false)
    {
        return "BLOCK_MODEL_SCOPE";
    }

    if bool_at(case, "future_information_used") != Some(false)
        || bool_at(case, "resource_conflict") != Some(false)
    {
        return "BLOCK_SCHEDULE";
    }

    let Some(objectives) = string_list(case, "objective_vector") else {
        return "BLOCK_OBJECTIVES";
    };
    let objective_set: HashSet<&str> = objectives.iter().copied().collect();
    let required = [
        "final_free_state_geometry",
        "production_time",
        "fixture_cost",
        "worst_case_robustness",
    ];
    if objective_set.len() != objectives.len()
        || !required.iter().all(|item| objective_set.contains(item))
    {
        return "BLOCK_OBJECTIVES";
    }

    if bool_at(case, "as_built_writeback") != Some(false) {
        return "BLOCK_AUTHORITY";
    }

    "SYNTHETIC_PLAN_CONTRACT_VALID"
}

fn validate_manifest(schema_text: &str, corpus_text: &str) -> Result<Summary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("schema JSON parse failed: {error}")]),
    };
    let corpus: Value = match serde_json::from_str(corpus_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("corpus JSON parse failed: {error}")]),
    };

    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        errors.push("schema must reject undeclared top-level fields".to_owned());
    }
    if str_at(&corpus, "schema_version") != Some(SCHEMA_VERSION) {
        errors.push("corpus schema_version must be 0.1.0".to_owned());
    }
    if str_at(&corpus, "benchmark_id") != Some(BENCHMARK_ID) {
        errors.push("corpus benchmark_id does not match this evaluator".to_owned());
    }
    if str_at(&corpus, "claim_ceiling").is_none_or(str::is_empty) {
        errors.push("claim_ceiling must be explicit".to_owned());
    }

    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["cases must be an array".to_owned()]);
    };
    if cases.len() != 31 {
        errors.push(format!("v0.1 requires 31 cases, found {}", cases.len()));
    }

    let mut ids = HashSet::new();
    let mut positive = 0usize;
    let mut negative = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let context = format!("cases[{index}]");
        let Some(case_id) = str_at(case, "case_id") else {
            errors.push(format!("{context}.case_id must be a string"));
            continue;
        };
        if !case_id.starts_with("SDC-") || case_id.len() != 7 {
            errors.push(format!("{context}.case_id must match SDC-NNN"));
        }
        if !ids.insert(case_id.to_owned()) {
            errors.push(format!("duplicate case_id {case_id}"));
        }

        match str_at(case, "case_class") {
            Some("positive_contract_control") => positive += 1,
            Some("negative_control") => negative += 1,
            _ => errors.push(format!("{context}.case_class is unknown")),
        }

        let actual = evaluate_case(case);
        let expected = str_at(case, "expected_disposition");
        if expected != Some(actual) {
            errors.push(format!(
                "{context} {case_id}: expected {}, evaluator returned {actual}",
                expected.unwrap_or("<missing>")
            ));
        }

        if str_at(case, "case_class") == Some("positive_contract_control")
            && actual != "SYNTHETIC_PLAN_CONTRACT_VALID"
        {
            errors.push(format!("{context} positive control did not pass"));
        }
        if str_at(case, "case_class") == Some("negative_control")
            && actual == "SYNTHETIC_PLAN_CONTRACT_VALID"
        {
            errors.push(format!("{context} negative control was falsely accepted"));
        }
    }

    if positive != 2 || negative != 29 {
        errors.push(format!(
            "v0.1 requires 2 positive and 29 negative controls; got {positive}/{negative}"
        ));
    }

    if errors.is_empty() {
        Ok(Summary {
            cases: cases.len(),
            positive,
            negative,
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    let schema_path = args.next().unwrap_or_else(|| DEFAULT_SCHEMA.to_owned());
    if args.next().is_some() {
        eprintln!("usage: shipyard-distortion-co-design [CORPUS_PATH] [SCHEMA_PATH]");
        return ExitCode::from(2);
    }

    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read schema {schema_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus_text = match fs::read_to_string(&corpus_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read corpus {corpus_path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    match validate_manifest(&schema_text, &corpus_text) {
        Ok(summary) => {
            println!(
                "PASS: {cases} cases ({positive} positive / {negative} negative); fixture/panel identity bound; no physical prediction claimed",
                cases = summary.cases,
                positive = summary.positive,
                negative = summary.negative
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("FAIL: {} contract violation(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_TEXT: &str =
        include_str!("../../../../docs/tech/SHIPYARD_DISTORTION_CO_DESIGN_V0_1.schema.json");
    const CORPUS_TEXT: &str =
        include_str!("../../../../benchmarks/shipyard/distortion-co-design/corpus-v0.1.json");

    fn corpus_value() -> Value {
        serde_json::from_str(CORPUS_TEXT).expect("corpus must parse")
    }

    #[test]
    fn all_frozen_controls_match_the_independent_disposition_rules() {
        let summary = validate_manifest(SCHEMA_TEXT, CORPUS_TEXT).expect("corpus should validate");
        assert_eq!(
            summary,
            Summary {
                cases: 31,
                positive: 2,
                negative: 29,
            }
        );
    }

    #[test]
    fn in_fixture_measurement_cannot_replace_final_free_state() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["final_measurement"]["phase"] = Value::String("in_fixture".to_owned());
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_FINAL_MEASUREMENT");
    }

    #[test]
    fn missing_release_cannot_be_treated_as_complete() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["release_events"] = Value::Array(Vec::new());
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_RELEASE_STATE");
    }

    #[test]
    fn future_information_is_not_admissible() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["future_information_used"] = Value::Bool(true);
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_SCHEDULE");
    }

    #[test]
    fn synthetic_state_cannot_be_written_to_as_built_record() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["as_built_writeback"] = Value::Bool(true);
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_AUTHORITY");
    }
}
