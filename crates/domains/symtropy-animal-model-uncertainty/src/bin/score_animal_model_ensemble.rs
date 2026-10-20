// SPDX-License-Identifier: AGPL-3.0-or-later

//! Score declared action losses across a registered set of alternative models.
//!
//! Example input schema:
//! {
//!   "schema_version": "ANIMAL-MODEL-DECISION-OUTCOMES-V1",
//!   "cvar_alpha": 0.9,
//!   "models": [{
//!     "model_id": "structure_a",
//!     "weight": 0.5,
//!     "action_losses": { "wait": 3.0 },
//!     "admissible_actions": ["wait"]
//!   }]
//! }
//!
//! This evaluates the declared numbers. It does not validate ecological realism,
//! the empirical basis for model weights, or the normative choice of an action.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use symtropy_animal_model_uncertainty::decision::{ActionRisk, ModelOutcome, score_model_ensemble};

const SYNTHETIC_EXAMPLE: &str =
    include_str!("../../../../../docs/living-world/ANIMAL_MODEL_DECISION_OUTCOMES_EXAMPLE_V1.json");
const EMBEDDED_INPUT_SCHEMA: &str =
    include_str!("../../../../../docs/living-world/ANIMAL_MODEL_DECISION_OUTCOMES_V1.schema.json");
const EMBEDDED_REPORT_SCHEMA: &str =
    include_str!("../../../../../docs/living-world/ANIMAL_MODEL_DECISION_RISK_REPORT_V1.schema.json");

fn parse_input(root: &Value) -> Result<(Vec<ModelOutcome>, f64), String> {
    if root.get("schema_version").and_then(Value::as_str)
        != Some("ANIMAL-MODEL-DECISION-OUTCOMES-V1")
    {
        return Err("schema_version must be ANIMAL-MODEL-DECISION-OUTCOMES-V1".to_owned());
    }
    let cvar_alpha = root
        .get("cvar_alpha")
        .and_then(Value::as_f64)
        .ok_or_else(|| "cvar_alpha must be a number".to_owned())?;
    let model_values = root
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| "models must be an array".to_owned())?;

    let mut models = Vec::with_capacity(model_values.len());
    for (index, value) in model_values.iter().enumerate() {
        let prefix = format!("models[{index}]");
        let model_id = value
            .get("model_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{prefix}.model_id must be a string"))?
            .to_owned();
        let weight = value
            .get("weight")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("{prefix}.weight must be a number"))?;

        let loss_values = value
            .get("action_losses")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{prefix}.action_losses must be an object"))?;
        let mut action_losses = BTreeMap::new();
        for (action, loss_value) in loss_values {
            let loss = loss_value
                .as_f64()
                .ok_or_else(|| format!("{prefix}.action_losses.{action} must be a number"))?;
            action_losses.insert(action.clone(), loss);
        }

        let action_values = value
            .get("admissible_actions")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{prefix}.admissible_actions must be an array"))?;
        let mut admissible_actions = BTreeSet::new();
        for (action_index, action_value) in action_values.iter().enumerate() {
            let action = action_value.as_str().ok_or_else(|| {
                format!("{prefix}.admissible_actions[{action_index}] must be a string")
            })?;
            admissible_actions.insert(action.to_owned());
        }

        models.push(ModelOutcome {
            model_id,
            weight,
            action_losses,
            admissible_actions,
        });
    }
    Ok((models, cvar_alpha))
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
    root: &Value,
    models: &[ModelOutcome],
    cvar_alpha: f64,
    scores: &[ActionRisk],
) -> Value {
    json!({
        "schema_version": "ANIMAL-MODEL-DECISION-RISK-REPORT-V1",
        "status": "computed_risk_summary_not_model_validation",
        "cvar_alpha": cvar_alpha,
        "input_sha256": sha256_hex(raw_input.as_bytes()),
        "scenario": root.get("scenario").and_then(Value::as_str).unwrap_or("unspecified"),
        "model_ids": models.iter().map(|model| model.model_id.as_str()).collect::<Vec<_>>(),
        "model_weights": models.iter().map(|model| json!({ "model_id": model.model_id, "weight": model.weight })).collect::<Vec<Value>>(),
        "model_count": models.len(),
        "scores": scores.iter().map(|score| json!({
            "action": score.action,
            "expected_loss": score.expected_loss,
            "worst_case_loss": score.worst_case_loss,
            "cvar_loss": score.cvar_loss,
            "per_model_regret": score.per_model_regret,
            "max_regret": score.max_regret,
            "admissible_across_all_models": score.admissible_across_all_models
        })).collect::<Vec<Value>>()
    })
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: score_animal_model_ensemble <outcomes.json>");
        process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!("usage: score_animal_model_ensemble <outcomes.json>");
        process::exit(2);
    }

    let raw = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!(
                "could not read model outcomes '{}': {error}",
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
    let (models, cvar_alpha) = match parse_input(&root) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("invalid model outcome input: {error}");
            process::exit(2);
        }
    };
    let scores = match score_model_ensemble(&models, cvar_alpha) {
        Ok(scores) => scores,
        Err(errors) => {
            for error in errors {
                eprintln!("risk scoring rejected input: {error}");
            }
            process::exit(1);
        }
    };

    let output = build_report(&raw, &root, &models, cvar_alpha, &scores);
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("JSON risk report should serialize")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_committed_two_model_synthetic_example() {
        let input: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("committed example should parse");
        let (models, alpha) = parse_input(&input).expect("valid synthetic model set");
        let scores = score_model_ensemble(&models, alpha).expect("valid synthetic losses");
        let graze = scores
            .iter()
            .find(|score| score.action == "graze")
            .expect("graze");
        let wait = scores
            .iter()
            .find(|score| score.action == "wait")
            .expect("wait");
        assert!((graze.expected_loss - 4.8).abs() < 1e-9);
        assert!(!graze.admissible_across_all_models);
        assert_eq!(wait.worst_case_loss, 3.0);
    }

    #[test]
    fn decision_input_schema_matches_example_contract() {
        let example: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        let schema: Value =
            serde_json::from_str(EMBEDDED_INPUT_SCHEMA).expect("input schema should parse");
        assert_eq!(
            example["schema_version"].as_str(),
            schema["properties"]["schema_version"]["const"].as_str()
        );
        assert_eq!(schema["properties"]["models"]["minItems"].as_u64(), Some(1));
        assert!(
            example["models"]
                .as_array()
                .is_some_and(|models| !models.is_empty())
        );
    }

    #[test]
    fn risk_report_is_bound_to_input_and_matches_report_contract() {
        let root: Value =
            serde_json::from_str(SYNTHETIC_EXAMPLE).expect("example JSON should parse");
        let (models, alpha) = parse_input(&root).expect("input should parse");
        let scores = score_model_ensemble(&models, alpha).expect("scores should be valid");
        let report = build_report(SYNTHETIC_EXAMPLE, &root, &models, alpha, &scores);
        let schema: Value =
            serde_json::from_str(EMBEDDED_REPORT_SCHEMA).expect("report schema should parse");
        assert_eq!(
            report["schema_version"].as_str(),
            schema["properties"]["schema_version"]["const"].as_str()
        );
        assert_eq!(
            report["input_sha256"].as_str(),
            Some(sha256_hex(SYNTHETIC_EXAMPLE.as_bytes()).as_str())
        );
        assert_eq!(report["model_count"].as_u64(), Some(2));
        assert_eq!(report["model_weights"][0]["weight"].as_f64(), Some(0.6));
        assert_eq!(report["model_ids"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            report["scores"].as_array().map(Vec::len),
            root["models"][0]["action_losses"].as_object().map(|actions| actions.len())
        );
    }

    #[test]
    fn sha256_digest_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let input = json!({
            "schema_version": "wrong",
            "cvar_alpha": 0.9,
            "models": []
        });
        assert!(
            parse_input(&input)
                .expect_err("wrong schema must fail")
                .contains("schema_version")
        );
    }
}
