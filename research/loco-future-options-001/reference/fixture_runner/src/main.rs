//! Run the committed LOCO-FUTURE-OPTIONS-001 JSON known-answer corpus through
//! the Rust reference evaluator. This is a data-to-engine conformance runner,
//! not a JSON Schema validator or a physical successor generator.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;

use loco_future_options_reference::{
    ActionAlias, ContinuationDomain, EvidenceManifest, FutureOptionScenario,
    HardConstraintEvidence, ObservationEvidence, RecoveryEvidence, ScenarioDisposition,
    SuccessorBranch, ThermalEvidence, ThermalNode, ThermalSourceClass, UpstreamDisposition,
    evaluate_future_options,
};
use serde_json::Value;

fn object<'a>(value: &'a Value, field: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{field}: expected object"))
}

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{field}: expected array"))
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{field}: expected string"))
}

fn bool_value(value: &Value, field: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("{field}: expected boolean"))
}

fn number(value: &Value, field: &str) -> Result<f64, String> {
    let number = value
        .as_f64()
        .ok_or_else(|| format!("{field}: expected number"))?;
    if !number.is_finite() {
        return Err(format!("{field}: non-finite number"));
    }
    Ok(number)
}

fn required<'a>(value: &'a Value, field: &str) -> Result<&'a Value, String> {
    object(value, "object")?
        .get(field)
        .ok_or_else(|| format!("missing field {field}"))
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    Ok(string(required(value, field)?, field)?.to_owned())
}

fn required_bool(value: &Value, field: &str) -> Result<bool, String> {
    bool_value(required(value, field)?, field)
}

fn to_milli_c(value: f64, field: &str) -> Result<i64, String> {
    let scaled = value * 1000.0;
    if !scaled.is_finite() || scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(format!("{field}: temperature outside i64 milli-degree range"));
    }
    Ok(scaled.round() as i64)
}

fn disposition_name(value: ScenarioDisposition) -> &'static str {
    match value {
        ScenarioDisposition::Qualified => "Qualified",
        ScenarioDisposition::Unknown => "Unknown",
        ScenarioDisposition::Invalid => "Invalid",
    }
}

fn parse_thermal(inputs: &Value) -> Result<ThermalEvidence, String> {
    let chain = required(inputs, "thermal_chain")?;
    let status = required_string(chain, "status")?;
    match status.as_str() {
        "not_applicable" => Ok(ThermalEvidence::NotApplicable),
        "valid" => {
            let model_digest = required_string(chain, "model_digest")?;
            let node_manifest_digest = required_string(chain, "node_manifest_digest")?;
            let predecessor = required_string(chain, "predecessor_state_digest")?;
            let result_state_digest = required_string(chain, "result_state_digest")?;
            let raw_nodes = array(required(inputs, "thermal_nodes")?, "thermal_nodes")?;
            let mut required_node_ids = Vec::new();
            let mut observed_nodes = Vec::new();
            for raw in raw_nodes {
                let node_id = required_string(raw, "node_id")?;
                let temperature = number(required(raw, "temperature_c")?, "temperature_c")?;
                let uncertainty = number(required(raw, "uncertainty_c")?, "uncertainty_c")?;
                let hard_limit = number(required(raw, "hard_limit_c")?, "hard_limit_c")?;
                if uncertainty < 0.0 {
                    return Err(format!("{node_id}: negative temperature uncertainty"));
                }
                let source = required_string(raw, "source_class")?;
                let source_class = match source.as_str() {
                    "measured" => ThermalSourceClass::Measured,
                    "estimated" => ThermalSourceClass::Estimated,
                    "environmental_boundary" => ThermalSourceClass::EnvironmentalBoundary,
                    "synthetic_fixture" => ThermalSourceClass::SyntheticFixture,
                    _ => return Err(format!("{node_id}: unknown thermal source class {source}")),
                };
                let evidence_digest = required_string(raw, "evidence_digest")?;
                required_node_ids.push(node_id.clone());
                observed_nodes.push(ThermalNode {
                    node_id,
                    upper_temperature_milli_c: to_milli_c(
                        temperature + uncertainty,
                        "temperature_upper",
                    )?,
                    hard_limit_milli_c: to_milli_c(hard_limit, "hard_limit_c")?,
                    source_class,
                    evidence_digest,
                });
            }
            Ok(ThermalEvidence::Valid {
                model_digest,
                node_manifest_digest,
                expected_predecessor_digest: predecessor.clone(),
                observed_predecessor_digest: predecessor,
                result_state_digest,
                required_node_ids,
                observed_nodes,
            })
        }
        "mismatched_predecessor" => Ok(ThermalEvidence::Valid {
            model_digest: required_string(chain, "model_digest")?,
            node_manifest_digest: required_string(chain, "node_manifest_digest")?,
            expected_predecessor_digest: required_string(
                chain,
                "expected_predecessor_state_digest",
            )?,
            observed_predecessor_digest: required_string(
                chain,
                "observed_predecessor_state_digest",
            )?,
            result_state_digest: required_string(chain, "result_state_digest")?,
            required_node_ids: Vec::new(),
            observed_nodes: Vec::new(),
        }),
        "unknown" => Ok(ThermalEvidence::Unknown {
            reason: "fixture marks thermal evidence unknown".to_owned(),
        }),
        "invalid" => Ok(ThermalEvidence::Invalid {
            reason: "fixture marks thermal evidence invalid".to_owned(),
        }),
        other => Err(format!("unknown thermal_chain.status: {other}")),
    }
}

fn parse_recovery(inputs: &Value) -> Result<RecoveryEvidence, String> {
    let raw = required(inputs, "recovery_evidence")?;
    let status = required_string(raw, "status")?;
    let disposition = match status.as_str() {
        "qualified" => UpstreamDisposition::Qualified,
        "unknown" => UpstreamDisposition::Unknown,
        "invalid" => UpstreamDisposition::Invalid,
        other => return Err(format!("unknown recovery status: {other}")),
    };
    Ok(RecoveryEvidence {
        disposition,
        reserve_state_digest: required_string(raw, "reserve_state_digest")?,
        topology_digest: required_string(raw, "topology_digest")?,
        terminal_safe_stop_reachable: required_bool(raw, "terminal_safe_stop_reachable")?,
    })
}

fn parse_case(case: &Value) -> Result<(String, FutureOptionScenario, Value), String> {
    let case_id = required_string(case, "case_id")?;
    let inputs = required(case, "inputs")?;

    let branch_manifest_raw = required(inputs, "branch_manifest")?;
    let branch_manifest = EvidenceManifest {
        complete: required_bool(branch_manifest_raw, "complete")?,
        digest: required_string(branch_manifest_raw, "digest")?,
    };

    let continuation_raw = required(inputs, "continuation_domain")?;
    let continuation_domain = ContinuationDomain {
        manifest: EvidenceManifest {
            complete: required_bool(continuation_raw, "complete")?,
            digest: required_string(continuation_raw, "digest")?,
        },
        aliases: array(required(continuation_raw, "actions")?, "continuation_domain.actions")?
            .iter()
            .map(|action| {
                Ok(ActionAlias {
                    label: required_string(action, "label")?,
                    physical_action_id: required_string(action, "physical_action_id")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    };

    let aliases: BTreeMap<String, String> = continuation_domain
        .aliases
        .iter()
        .map(|alias| (alias.label.clone(), alias.physical_action_id.clone()))
        .collect();

    let mut branches = Vec::new();
    for raw_branch in array(required(inputs, "branches")?, "branches")? {
        let branch_id = required_string(raw_branch, "branch_id")?;
        let mut admitted_action_labels = Vec::new();
        for action in array(
            required(raw_branch, "continuation_actions")?,
            "continuation_actions",
        )? {
            let label = required_string(action, "label")?;
            let physical_action_id = required_string(action, "physical_action_id")?;
            if aliases.get(&label) != Some(&physical_action_id) {
                return Err(format!(
                    "{case_id}/{branch_id}: action label {label} does not bind to canonical ID {physical_action_id}"
                ));
            }
            admitted_action_labels.push(label);
        }
        let probability = number(
            required(raw_branch, "probability_mass")?,
            "probability_mass",
        )?;
        if !(0.0..=1.0).contains(&probability) {
            return Err(format!("{case_id}/{branch_id}: probability mass outside [0,1]"));
        }
        let probability_mass_ppm = (probability * 1_000_000.0).round() as u32;

        let observation_valid = required_bool(raw_branch, "observation_valid")?;
        let observation = if observation_valid {
            ObservationEvidence::FreshQualified {
                equivalence_class: required_string(raw_branch, "observation_class")?,
                receipt_digest: required_string(raw_branch, "observation_receipt")?,
            }
        } else {
            ObservationEvidence::Unknown {
                reason: "fixture observation receipt is not valid/fresh".to_owned(),
            }
        };

        let viability_complete = required_bool(raw_branch, "viability_complete")?;
        let viability_receipt_digest =
            required_string(raw_branch, "viability_receipt_digest")?;

        let hard_constraint = required_string(raw_branch, "hard_constraint_status")?;
        let hard_constraints = if hard_constraint == "within_limits" {
            HardConstraintEvidence::WithinLimits
        } else if hard_constraint.starts_with("violated:") {
            HardConstraintEvidence::Violated {
                constraint_id: hard_constraint.trim_start_matches("violated:").trim().to_owned(),
            }
        } else {
            HardConstraintEvidence::Unknown {
                reason: hard_constraint,
            }
        };

        branches.push(SuccessorBranch {
            branch_id,
            physically_possible: required_bool(raw_branch, "physically_possible")?,
            probability_mass_ppm,
            successor_state_digest: required_string(raw_branch, "successor_state_digest")?,
            observation,
            viability_complete,
            viability_receipt_digest,
            admitted_action_labels,
            hard_constraints,
        });
    }

    let scenario = FutureOptionScenario {
        candidate_action_id: required_string(inputs, "candidate_action_id")?,
        initial_state_digest: required_string(inputs, "initial_state_digest")?,
        transition_model_digest: required_string(inputs, "transition_model_digest")?,
        action_domain_digest: required_string(inputs, "action_domain_digest")?,
        branch_manifest,
        continuation_domain,
        branches,
        thermal: parse_thermal(inputs)?,
        recovery: parse_recovery(inputs)?,
    };
    Ok((case_id, scenario, required(case, "expected")?.clone()))
}

fn string_array(value: &Value, field: &str) -> Result<Vec<String>, String> {
    let mut items = array(value, field)?
        .iter()
        .map(|item| string(item, field).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    items.sort();
    Ok(items)
}

fn expected_string_map(
    value: &Value,
    field: &str,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut out = BTreeMap::new();
    for (key, item) in object(value, field)? {
        out.insert(key.clone(), string_array(item, field)?);
    }
    Ok(out)
}

fn actual_common_map(
    scenario: &FutureOptionScenario,
    report: &loco_future_options_reference::FutureOptionReport,
) -> BTreeMap<String, Vec<String>> {
    report
        .observation_sets
        .iter()
        .map(|(key, set)| (key.clone(), set.common_physical_action_ids.clone()))
        .collect()
}

fn expected_count_map(value: &Value, field: &str) -> Result<BTreeMap<String, usize>, String> {
    let mut out = BTreeMap::new();
    for (key, item) in object(value, field)? {
        let count = item
            .as_u64()
            .ok_or_else(|| format!("{field}.{key}: expected non-negative integer"))?;
        out.insert(key.clone(), count as usize);
    }
    Ok(out)
}

fn actual_count_map(
    report: &loco_future_options_reference::FutureOptionReport,
) -> BTreeMap<String, usize> {
    report
        .observation_sets
        .iter()
        .map(|(key, set)| (key.clone(), set.common_physical_action_ids.len()))
        .collect()
}

fn compare_case(
    case_id: &str,
    scenario: &FutureOptionScenario,
    expected: &Value,
) -> Result<(), String> {
    let report = evaluate_future_options(scenario)
        .map_err(|error| format!("{case_id}: evaluator error: {error}"))?;
    let expected_disposition = required_string(expected, "disposition")?;
    let actual_disposition = disposition_name(report.disposition);
    if actual_disposition != expected_disposition {
        return Err(format!(
            "{case_id}: disposition {actual_disposition} != {expected_disposition}"
        ));
    }
    if report.actuation_authorized {
        return Err(format!("{case_id}: retrospective evaluator authorized actuation"));
    }

    if let Some(raw) = object(expected, "expected")?.get("common_continuations_by_observation") {
        let expected_map = expected_string_map(raw, "common_continuations_by_observation")?;
        let actual_map = actual_common_map(scenario, &report);
        if actual_map != expected_map {
            return Err(format!(
                "{case_id}: common continuation mismatch: actual {actual_map:?}, expected {expected_map:?}"
            ));
        }
    }

    if let Some(raw) = object(expected, "expected")?.get("clairvoyant_branch_continuations") {
        let expected_map = expected_string_map(raw, "clairvoyant_branch_continuations")?;
        let actual_map: BTreeMap<String, Vec<String>> = report
            .clairvoyant_branch_actions
            .iter()
            .map(|(key, values)| (key.clone(), values.clone()))
            .collect();
        if actual_map != expected_map {
            return Err(format!(
                "{case_id}: clairvoyant branch set mismatch: actual {actual_map:?}, expected {expected_map:?}"
            ));
        }
    }

    if let Some(raw) = object(expected, "expected")?.get("clairvoyant_only_action_count") {
        let expected_count = raw
            .as_u64()
            .ok_or_else(|| format!("{case_id}: clairvoyant count must be a non-negative integer"))?;
        if report.clairvoyant_branch_option_count != expected_count as usize {
            return Err(format!(
                "{case_id}: clairvoyant option count {} != {}",
                report.clairvoyant_branch_option_count, expected_count
            ));
        }
    }

    if let Some(raw) = object(expected, "expected")?
        .get("deployable_common_action_count_by_observation")
    {
        let expected_counts =
            expected_count_map(raw, "deployable_common_action_count_by_observation")?;
        let actual_counts = actual_count_map(&report);
        if actual_counts != expected_counts {
            return Err(format!(
                "{case_id}: deployable counts {actual_counts:?} != {expected_counts:?}"
            ));
        }
    }

    Ok(())
}

fn run_corpus(path: &PathBuf) -> Result<usize, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let root: Value = serde_json::from_str(&source)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))?;
    if string(required(&root, "protocol_id")?, "protocol_id")?
        != loco_future_options_reference::PROTOCOL_ID
    {
        return Err("protocol_id does not match the Rust evaluator".to_owned());
    }
    let mut seen = BTreeSet::new();
    let cases = array(required(&root, "cases")?, "cases")?;
    for raw_case in cases {
        let (case_id, scenario, expected) = parse_case(raw_case)?;
        if !seen.insert(case_id.clone()) {
            return Err(format!("duplicate case_id {case_id}"));
        }
        compare_case(&case_id, &scenario, &expected)?;
        println!("PASS {case_id}");
    }
    Ok(cases.len())
}

fn main() {
    let path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "../../fixtures/known_answers_v0_1.json",
            )
        });
    match run_corpus(&path) {
        Ok(count) => println!("LOCO-FUTURE-OPTIONS-001: {count} JSON cases matched the Rust evaluator."),
        Err(error) => {
            eprintln!("LOCO-FUTURE-OPTIONS-001 fixture failure: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_json_corpus_matches_reference_evaluator() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixture = root.join("../../fixtures/known_answers_v0_1.json");
        assert_eq!(run_corpus(&fixture).unwrap(), 7);
    }
}
