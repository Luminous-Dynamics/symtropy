use serde::Serialize;
use serde_json::Value;
use std::process::ExitCode;

const POLICY_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_challenge_corpus.schema.json");
const STREAMS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_scenario_streams.schema.json");
const DECISION_FRAME_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_decision_frame.schema.json");

#[derive(Debug, Serialize, PartialEq, Eq)]
struct PolicyDecisionFrame {
    schema_id: &'static str,
    benchmark_id: String,
    case_id: String,
    scenario_family_id: String,
    decision_epoch: usize,
    static_case: Value,
    observable_history: Vec<Value>,
    claim_ceiling: &'static str,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate_schema(instance: &Value, schema_source: &str, label: &str) -> Result<(), String> {
    let schema: Value = serde_json::from_str(schema_source)
        .map_err(|error| fail(format!("{label}_schema_parse:{error}")))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| fail(format!("{label}_schema_compile:{error}")))?;
    validator
        .validate(instance)
        .map_err(|error| fail(format!("{label}_schema_validation:{error}")))
}

fn required_string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| fail(format!("{context}_{key}_missing_or_invalid")))
}

fn find_unique_case<'a>(
    cases: &'a [Value],
    case_id: &str,
    context: &str,
) -> Result<&'a Value, String> {
    let matches: Vec<&Value> = cases
        .iter()
        .filter(|case| case.get("case_id").and_then(Value::as_str) == Some(case_id))
        .collect();
    match matches.as_slice() {
        [case] => Ok(*case),
        [] => Err(fail(format!("{context}_case_not_found"))),
        _ => Err(fail(format!("{context}_duplicate_case_id"))),
    }
}

fn project_prefix(
    benchmark_id: &str,
    policy_case: &Value,
    stream_case: &Value,
    scenario_slot: usize,
    decision_epoch: usize,
) -> Result<PolicyDecisionFrame, String> {
    if decision_epoch >= 4 {
        return Err(fail("decision_epoch_out_of_range"));
    }

    let case_id = required_string(policy_case, "case_id", "policy_case")?;
    let scenario_family_id = required_string(policy_case, "scenario_family_id", "policy_case")?;
    if required_string(stream_case, "case_id", "stream_case")? != case_id
        || required_string(stream_case, "scenario_family_id", "stream_case")? != scenario_family_id
    {
        return Err(fail("policy_and_stream_case_identity_mismatch"));
    }

    let scenarios = stream_case
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("scenario_streams_missing"))?;
    let scenario = scenarios
        .get(scenario_slot)
        .ok_or_else(|| fail("scenario_slot_out_of_range"))?;

    // The scenario ID and probability are deliberately read only by the simulator-side
    // selector. Neither value is copied into the policy decision frame.
    let observations = scenario
        .get("observations_by_epoch")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("scenario_observations_missing"))?;
    if observations.len() != 4 {
        return Err(fail("scenario_observation_horizon_invalid"));
    }

    for (expected_epoch, observation) in observations.iter().enumerate() {
        let actual_epoch = observation
            .get("epoch")
            .and_then(Value::as_u64)
            .ok_or_else(|| fail("observation_epoch_missing_or_invalid"))?;
        if actual_epoch != expected_epoch as u64 {
            return Err(fail("observation_epoch_order_invalid"));
        }
    }

    let visible_history = observations
        .iter()
        .take(decision_epoch + 1)
        .cloned()
        .collect::<Vec<_>>();
    if visible_history.len() != decision_epoch + 1 {
        return Err(fail("observable_prefix_length_invalid"));
    }

    Ok(PolicyDecisionFrame {
        schema_id: "https://luminousdynamics.io/schemas/mfg-policy-v0.2-decision-frame",
        benchmark_id: benchmark_id.to_string(),
        case_id: case_id.to_string(),
        scenario_family_id: scenario_family_id.to_string(),
        decision_epoch,
        static_case: policy_case.clone(),
        observable_history: visible_history,
        claim_ceiling: "Synthetic policy-input projection only; no evidence authentication, schedule feasibility, execution, acceptance or production authority.",
    })
}

fn project_from_documents(
    policy_corpus: &Value,
    scenario_streams: &Value,
    case_id: &str,
    scenario_slot: usize,
    decision_epoch: usize,
) -> Result<PolicyDecisionFrame, String> {
    validate_schema(policy_corpus, POLICY_SCHEMA_JSON, "policy")?;
    validate_schema(scenario_streams, STREAMS_SCHEMA_JSON, "scenario_streams")?;

    let benchmark_id = required_string(policy_corpus, "benchmark_id", "policy_corpus")?;
    if required_string(scenario_streams, "benchmark_id", "scenario_streams")? != benchmark_id {
        return Err(fail("benchmark_id_mismatch"));
    }

    let policy_cases = policy_corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("policy_cases_missing"))?;
    let stream_cases = scenario_streams
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("scenario_stream_cases_missing"))?;

    let policy_case = find_unique_case(policy_cases, case_id, "policy")?;
    let stream_case = find_unique_case(stream_cases, case_id, "scenario_stream")?;
    project_prefix(
        benchmark_id,
        policy_case,
        stream_case,
        scenario_slot,
        decision_epoch,
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        return Err(
            "usage: mfg-policy-v02-prefix-gateway <policy-corpus.json> <scenario-streams.json> <case-id> <scenario-slot-0..3> <decision-epoch-0..3>"
                .into(),
        );
    }

    let policy_bytes =
        std::fs::read(&args[1]).map_err(|error| fail(format!("policy_corpus_read:{error}")))?;
    let streams_bytes =
        std::fs::read(&args[2]).map_err(|error| fail(format!("scenario_streams_read:{error}")))?;
    let policy_corpus: Value = serde_json::from_slice(&policy_bytes)
        .map_err(|error| fail(format!("policy_corpus_json:{error}")))?;
    let scenario_streams: Value = serde_json::from_slice(&streams_bytes)
        .map_err(|error| fail(format!("scenario_streams_json:{error}")))?;
    let scenario_slot = args[4]
        .parse::<usize>()
        .map_err(|error| fail(format!("scenario_slot_parse:{error}")))?;
    let decision_epoch = args[5]
        .parse::<usize>()
        .map_err(|error| fail(format!("decision_epoch_parse:{error}")))?;

    let frame = project_from_documents(
        &policy_corpus,
        &scenario_streams,
        &args[3],
        scenario_slot,
        decision_epoch,
    )?;
    let frame_value = serde_json::to_value(&frame)
        .map_err(|error| fail(format!("decision_frame_to_json:{error}")))?;
    validate_schema(&frame_value, DECISION_FRAME_SCHEMA_JSON, "decision_frame")?;
    let output = serde_json::to_string_pretty(&frame)
        .map_err(|error| fail(format!("decision_frame_serialize:{error}")))?;
    println!("{output}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_policy_case() -> Value {
        json!({
            "case_id": "CROSS-BOTTLENECKS-01",
            "scenario_family_id": "crossed-bottlenecks",
            "split": "development",
            "replication_seed": 1,
            "generator_seed": 42,
            "paired_seed_id": "PAIR-SEED-01",
            "horizon_epochs": 4,
            "decision_visibility_rule": "Static case only; events revealed incrementally.",
            "jobs": [],
            "resources": [],
            "setup_transitions": [],
            "inspection_station": {}
        })
    }

    fn sample_stream_case() -> Value {
        let observations = (0..4)
            .map(|epoch| {
                json!({
                    "epoch": epoch,
                    "resource_events": [{
                        "source_event_id": format!("EVENT-{epoch}"),
                        "resource_id": "RESOURCE-A",
                        "kind": "FAILED",
                        "occurred_at_epoch": epoch,
                        "observed_at_epoch": epoch,
                        "common_cause_id": null,
                        "recovery_forecast_min_epochs": 1,
                        "recovery_forecast_max_epochs": 2
                    }],
                    "qualification_updates": [],
                    "inspection_capacity_events": []
                })
            })
            .collect::<Vec<_>>();
        json!({
            "case_id": "CROSS-BOTTLENECKS-01",
            "scenario_family_id": "crossed-bottlenecks",
            "scenarios": [
                {"scenario_id": "NOMINAL-01", "probability_ppm": 300000, "observations_by_epoch": observations},
                {"scenario_id": "NOMINAL-02", "probability_ppm": 200000, "observations_by_epoch": observations},
                {"scenario_id": "PRIMARY-DISRUPTION", "probability_ppm": 300000, "observations_by_epoch": observations},
                {"scenario_id": "ALTERNATE-DISRUPTION", "probability_ppm": 200000, "observations_by_epoch": observations}
            ]
        })
    }

    #[test]
    fn frame_exposes_only_the_prefix_through_the_requested_epoch() {
        let policy_case = sample_policy_case();
        let stream_case = sample_stream_case();
        let frame = project_prefix(
            "MFG-POLICY-V02-CHALLENGE-001",
            &policy_case,
            &stream_case,
            2,
            1,
        )
        .expect("valid policy prefix");
        assert_eq!(frame.decision_epoch, 1);
        assert_eq!(frame.observable_history.len(), 2);
        assert_eq!(frame.observable_history[0]["epoch"], Value::from(0_u64));
        assert_eq!(frame.observable_history[1]["epoch"], Value::from(1_u64));
        let serialized = serde_json::to_string(&frame).unwrap();
        assert!(!serialized.contains("EVENT-2"));
        assert!(!serialized.contains("EVENT-3"));
        assert!(!serialized.contains("scenario_id"));
        assert!(!serialized.contains("probability_ppm"));
    }

    #[test]
    fn same_observable_prefix_produces_identical_policy_frames() {
        let policy_case = sample_policy_case();
        let stream_case = sample_stream_case();
        let left = project_prefix(
            "MFG-POLICY-V02-CHALLENGE-001",
            &policy_case,
            &stream_case,
            0,
            2,
        )
        .unwrap();
        let right = project_prefix(
            "MFG-POLICY-V02-CHALLENGE-001",
            &policy_case,
            &stream_case,
            1,
            2,
        )
        .unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn mutation_of_future_events_cannot_change_an_earlier_decision_frame() {
        let policy_case = sample_policy_case();
        let original_streams = sample_stream_case();
        let original = project_prefix(
            "MFG-POLICY-V02-CHALLENGE-001",
            &policy_case,
            &original_streams,
            0,
            1,
        )
        .unwrap();
        let mut mutated_streams = original_streams;
        mutated_streams["scenarios"][0]["observations_by_epoch"][3]["resource_events"][0]["source_event_id"] =
            Value::from("MUTATED-FUTURE-EVENT");
        let mutated = project_prefix(
            "MFG-POLICY-V02-CHALLENGE-001",
            &policy_case,
            &mutated_streams,
            0,
            1,
        )
        .unwrap();
        assert_eq!(original, mutated);
    }

    #[test]
    fn out_of_range_epoch_and_scenario_slot_fail_closed() {
        let policy_case = sample_policy_case();
        let stream_case = sample_stream_case();
        assert_eq!(
            project_prefix(
                "MFG-POLICY-V02-CHALLENGE-001",
                &policy_case,
                &stream_case,
                0,
                4
            )
            .unwrap_err(),
            "decision_epoch_out_of_range"
        );
        assert_eq!(
            project_prefix(
                "MFG-POLICY-V02-CHALLENGE-001",
                &policy_case,
                &stream_case,
                4,
                1
            )
            .unwrap_err(),
            "scenario_slot_out_of_range"
        );
    }

    #[test]
    fn future_history_and_scenario_identity_are_not_in_the_frame_contract() {
        let policy_case = sample_policy_case();
        let stream_case = sample_stream_case();
        let frame = project_prefix(
            "MFG-POLICY-V02-CHALLENGE-001",
            &policy_case,
            &stream_case,
            3,
            0,
        )
        .unwrap();
        let value = serde_json::to_value(frame).unwrap();
        assert!(value.get("scenario_id").is_none());
        assert!(value.get("scenario_slot").is_none());
        assert!(value.get("probability_ppm").is_none());
        assert!(
            value
                .get("observable_history")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                == 1
        );
    }
}
