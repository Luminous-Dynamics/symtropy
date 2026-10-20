// SPDX-License-Identifier: AGPL-3.0-or-later

//! Score candidate observations by their expected reduction in decision loss.
//!
//! This reports decision value conditional on the declared model set, weights,
//! likelihoods, loss function, and cost units. It does not validate ecology or
//! establish that the declared model set is complete.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use symtropy_animal_model_uncertainty::decision::ModelOutcome;
use symtropy_animal_model_uncertainty::experiment_design::{
    ObservationDesign, ObservationValue, score_observation_designs,
};

const SYNTHETIC_EXAMPLE: &str = include_str!(
    "../../../../../docs/living-world/ANIMAL_MODEL_OBSERVATION_DESIGNS_EXAMPLE_V1.json"
);
const EMBEDDED_INPUT_SCHEMA: &str = include_str!(
    "../../../../../docs/living-world/ANIMAL_MODEL_OBSERVATION_DESIGNS_V1.schema.json"
);
const EMBEDDED_REPORT_SCHEMA: &str = include_str!(
    "../../../../../docs/living-world/ANIMAL_MODEL_OBSERVATION_DESIGN_REPORT_V1.schema.json"
);

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some('a'..='z'))
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn ensure_only_fields(value: &Value, allowed: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context} must be an object"))?;
    if let Some(field) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{context} contains unknown field '{field}'"));
    }
    Ok(())
}

fn parse_input(
    root: &Value,
) -> Result<(String, Vec<ModelOutcome>, Vec<ObservationDesign>), String> {
    ensure_only_fields(
        root,
        &[
            "schema_version",
            "status",
            "scenario",
            "note",
            "models",
            "designs",
        ],
        "input",
    )?;
    if root.get("schema_version").and_then(Value::as_str)
        != Some("ANIMAL-MODEL-OBSERVATION-DESIGNS-V1")
    {
        return Err("schema_version must be ANIMAL-MODEL-OBSERVATION-DESIGNS-V1".to_owned());
    }
    if root.get("status").and_then(Value::as_str)
        != Some("synthetic_model_discriminating_observation_designs_not_empirical")
    {
        return Err("status must state these are non-empirical observation designs".to_owned());
    }
    let scenario = root
        .get("scenario")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "scenario must be a non-empty string".to_owned())?
        .to_owned();
    let note = root
        .get("note")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "note must be a non-empty string".to_owned())?;
    if !note.to_ascii_lowercase().contains("synthetic") {
        return Err("note must explicitly scope the fixture as synthetic".to_owned());
    }

    let model_values = root
        .get("models")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| "models must be a non-empty array".to_owned())?;
    let mut models = Vec::with_capacity(model_values.len());
    for (index, value) in model_values.iter().enumerate() {
        let prefix = format!("models[{index}]");
        ensure_only_fields(
            value,
            &["model_id", "weight", "action_losses", "admissible_actions"],
            &prefix,
        )?;
        let model_id = value
            .get("model_id")
            .and_then(Value::as_str)
            .filter(|id| valid_identifier(id))
            .ok_or_else(|| {
                format!("{prefix}.model_id must match lowercase snake_case identifier syntax")
            })?
            .to_owned();
        let weight = value
            .get("weight")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("{prefix}.weight must be a number"))?;
        let loss_values = value
            .get("action_losses")
            .and_then(Value::as_object)
            .filter(|values| !values.is_empty())
            .ok_or_else(|| format!("{prefix}.action_losses must be a non-empty object"))?;
        let mut action_losses = BTreeMap::new();
        for (action, loss_value) in loss_values {
            if !valid_identifier(action) {
                return Err(format!(
                    "{prefix}.action_losses contains invalid action identifier '{action}'"
                ));
            }
            let loss = loss_value
                .as_f64()
                .ok_or_else(|| format!("{prefix}.action_losses.{action} must be a number"))?;
            action_losses.insert(action.clone(), loss);
        }
        let action_values = value
            .get("admissible_actions")
            .and_then(Value::as_array)
            .filter(|values| !values.is_empty())
            .ok_or_else(|| format!("{prefix}.admissible_actions must be a non-empty array"))?;
        let mut admissible_actions = BTreeSet::new();
        for (action_index, action_value) in action_values.iter().enumerate() {
            let action = action_value
                .as_str()
                .filter(|value| valid_identifier(value))
                .ok_or_else(|| {
                    format!("{prefix}.admissible_actions[{action_index}] must be a lowercase snake_case identifier")
                })?;
            admissible_actions.insert(action.to_owned());
        }
        if admissible_actions.len() != action_values.len() {
            return Err(format!(
                "{prefix}.admissible_actions contains duplicate values"
            ));
        }
        models.push(ModelOutcome {
            model_id,
            weight,
            action_losses,
            admissible_actions,
        });
    }

    let design_values = root
        .get("designs")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| "designs must be a non-empty array".to_owned())?;
    let mut designs = Vec::with_capacity(design_values.len());
    for (index, value) in design_values.iter().enumerate() {
        let prefix = format!("designs[{index}]");
        ensure_only_fields(
            value,
            &[
                "observation_id",
                "loss_equivalent_cost",
                "outcome_likelihoods",
            ],
            &prefix,
        )?;
        let observation_id = value
            .get("observation_id")
            .and_then(Value::as_str)
            .filter(|id| valid_identifier(id))
            .ok_or_else(|| {
                format!("{prefix}.observation_id must match lowercase snake_case identifier syntax")
            })?
            .to_owned();
        let loss_equivalent_cost = value
            .get("loss_equivalent_cost")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("{prefix}.loss_equivalent_cost must be a number"))?;
        let outcome_values = value
            .get("outcome_likelihoods")
            .and_then(Value::as_object)
            .filter(|values| !values.is_empty())
            .ok_or_else(|| format!("{prefix}.outcome_likelihoods must be a non-empty object"))?;
        let mut outcome_likelihoods = BTreeMap::new();
        for (outcome, likelihoods_value) in outcome_values {
            if !valid_identifier(outcome) {
                return Err(format!(
                    "{prefix}.outcome_likelihoods contains invalid outcome identifier '{outcome}'"
                ));
            }
            let outcome_prefix = format!("{prefix}.outcome_likelihoods.{outcome}");
            let likelihoods = likelihoods_value
                .as_object()
                .filter(|values| !values.is_empty())
                .ok_or_else(|| format!("{outcome_prefix} must be a non-empty object"))?;
            let mut probabilities = BTreeMap::new();
            for (model_id, probability_value) in likelihoods {
                if !valid_identifier(model_id) {
                    return Err(format!(
                        "{outcome_prefix} contains invalid model identifier '{model_id}'"
                    ));
                }
                let probability = probability_value
                    .as_f64()
                    .ok_or_else(|| format!("{outcome_prefix}.{model_id} must be a number"))?;
                probabilities.insert(model_id.clone(), probability);
            }
            outcome_likelihoods.insert(outcome.clone(), probabilities);
        }
        designs.push(ObservationDesign {
            observation_id,
            loss_equivalent_cost,
            outcome_likelihoods,
        });
    }

    Ok((scenario, models, designs))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn build_report(
    raw_input: &str,
    scenario: &str,
    models: &[ModelOutcome],
    values: &[ObservationValue],
) -> Value {
    json!({
        "schema_version": "ANIMAL-MODEL-OBSERVATION-DESIGN-REPORT-V1",
        "status": "scored_observation_designs_conditional_on_declared_model_set",
        "qualification": {
            "model_set_adequacy": "not_assessed",
            "empirical_validation": "not_assessed",
            "use_scope": "conditional_model_set_analysis_only"
        },
        "scenario": scenario,
        "input_sha256": sha256_hex(raw_input.as_bytes()),
        "model_ids": models.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>(),
        "model_weights": models.iter().map(|model| json!({
            "model_id": model.model_id,
            "weight": model.weight
        })).collect::<Vec<Value>>(),
        "observation_count": values.len(),
        "ranked_observations": values.iter().map(|value| json!({
            "observation_id": value.observation_id,
            "baseline_action": value.baseline_action,
            "baseline_expected_loss": value.baseline_expected_loss,
            "expected_post_observation_loss": value.expected_post_observation_loss,
            "gross_value_of_information": value.gross_value_of_information,
            "loss_equivalent_cost": value.loss_equivalent_cost,
            "net_decision_value": value.net_decision_value,
            "outcomes": value.outcomes.iter().map(|outcome| json!({
                "outcome": outcome.outcome,
                "probability": outcome.probability,
                "selected_action": outcome.selected_action,
                "expected_loss_after_observation": outcome.expected_loss_after_observation,
                "posterior_model_weights": &outcome.posterior_model_weights
            })).collect::<Vec<Value>>()
        })).collect::<Vec<Value>>()
    })
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!(
            "usage: score_animal_observation_designs <designs.json> (options: --help, --example, --input-schema, --report-schema)"
        );
        process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!("usage: score_animal_observation_designs <designs.json> (options: --help, --example, --input-schema, --report-schema)");
        process::exit(2);
    }

    match path.to_str() {
        Some("--help" | "-h") => {
            println!(
                r#"Score candidate observations by expected decision-loss reduction minus cost.
Usage: score_animal_observation_designs <designs.json>
--example       print a synthetic input example
--input-schema  print the expected input schema
--report-schema print the observation report schema

Results are conditional on the declared model set, weights, likelihoods and loss units. This is not biological validation."#
            );
            return;
        }
        Some("--example") => {
            print!("{SYNTHETIC_EXAMPLE}");
            return;
        }
        Some("--input-schema") => {
            print!("{EMBEDDED_INPUT_SCHEMA}");
            return;
        }
        Some("--report-schema") => {
            print!("{EMBEDDED_REPORT_SCHEMA}");
            return;
        }
        _ => {}
    }

    let raw = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!(
                "could not read observation design input '{}': {error}",
                path.to_string_lossy()
            );
            process::exit(2);
        }
    };
    let root: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid JSON: {error}");
            process::exit(2);
        }
    };
    let (scenario, models, designs) = match parse_input(&root) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("invalid observation design input: {error}");
            process::exit(2);
        }
    };
    let values = match score_observation_designs(&models, &designs) {
        Ok(values) => values,
        Err(errors) => {
            for error in errors {
                eprintln!("observation design scoring rejected input: {error}");
            }
            process::exit(1);
        }
    };
    let report = build_report(&raw, &scenario, &models, &values);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("observation report should serialize")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_example_scores_expected_decision_value() {
        let root: Value = serde_json::from_str(SYNTHETIC_EXAMPLE)
            .expect("example JSON must parse");
        let (scenario, models, designs) = parse_input(&root).expect("valid example");
        let values = score_observation_designs(&models, &designs).expect("valid likelihoods");
        assert_eq!(
            scenario,
            "two_ecological_structures_require_different_actions"
        );
        assert_eq!(values.len(), 1);
        let value = &values[0];
        assert!((value.baseline_expected_loss - 5.0).abs() < 1e-9);
        assert!((value.expected_post_observation_loss - 1.0).abs() < 1e-9);
        assert!((value.gross_value_of_information - 4.0).abs() < 1e-9);
        assert!((value.loss_equivalent_cost - 1.0).abs() < 1e-9);
        assert!((value.net_decision_value - 3.0).abs() < 1e-9);
    }

    #[test]
    fn report_binds_input_and_matches_schema_contract() {
        let root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON must parse");
        let (scenario, models, designs) = parse_input(&root).expect("valid example");
        let values = score_observation_designs(&models, &designs).expect("valid likelihoods");
        let report = build_report(SYNTHETIC_EXAMPLE, &scenario, &models, &values);
        let schema: Value =
            serde_json::from_str(EMBEDDED_REPORT_SCHEMA).expect("report schema must parse");
        assert_eq!(
            report["schema_version"].as_str(),
            schema["properties"]["schema_version"]["const"].as_str()
        );
        assert!(
            schema["required"]
                .as_array()
                .is_some_and(|required| required.contains(&json!("qualification")))
        );
        assert!(
            schema["properties"]["ranked_observations"]["items"]["properties"]["outcomes"]["items"]["required"]
                .as_array()
                .is_some_and(|required| required.contains(&json!("posterior_model_weights")))
        );
        assert_eq!(
            report["status"].as_str(),
            Some("scored_observation_designs_conditional_on_declared_model_set")
        );
        assert_eq!(
            report["input_sha256"].as_str(),
            Some(sha256_hex(SYNTHETIC_EXAMPLE.as_bytes()).as_str())
        );
        assert_eq!(report["observation_count"].as_u64(), Some(1));
        assert_eq!(
            report["qualification"]["model_set_adequacy"].as_str(),
            Some("not_assessed")
        );
        assert_eq!(
            report["qualification"]["empirical_validation"].as_str(),
            Some("not_assessed")
        );
        assert_eq!(
            report["ranked_observations"][0]["outcomes"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            report["ranked_observations"][0]["outcomes"][0]["posterior_model_weights"]["structure_a"]
                .as_f64(),
            Some(0.9)
        );
    }

    #[test]
    fn parser_rejects_unknown_root_fields() {
        let mut root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON must parse");
        root["hidden_truth"] = json!("privileged");
        assert!(parse_input(&root).is_err());
    }

    #[test]
    fn parser_rejects_noncanonical_model_identifiers() {
        let mut root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON must parse");
        root["models"][0]["model_id"] = json!("Structure A");
        assert!(parse_input(&root).is_err());
    }

    #[test]
    fn parser_requires_synthetic_scope_declaration() {
        let mut root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON must parse");
        root["status"] = json!("validated_ecology");
        assert!(parse_input(&root).is_err());
    }

    #[test]
    fn scorer_rejects_missing_model_likelihood() {
        let mut root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON must parse");
        root["designs"][0]["outcome_likelihoods"]["cue_a"]
            .as_object_mut()
            .expect("likelihoods")
            .remove("structure_b");
        let (_, models, designs) = parse_input(&root).expect("shape may parse");
        assert!(score_observation_designs(&models, &designs).is_err());
    }

    #[test]
    fn scorer_rejects_likelihoods_that_do_not_sum_to_one() {
        let mut root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON must parse");
        root["designs"][0]["outcome_likelihoods"]["cue_b"]["structure_a"] = json!(0.2);
        let (_, models, designs) = parse_input(&root).expect("shape may parse");
        assert!(score_observation_designs(&models, &designs).is_err());
    }
}
