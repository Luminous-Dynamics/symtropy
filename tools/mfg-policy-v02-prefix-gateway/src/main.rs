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

fn validate_stream_case(
    policy_case: &Value,
    stream_case: &Value,
    decision_epoch: usize,
) -> Result<(), String> {
    let resources = policy_case
        .get("resources")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("policy_resources_missing"))?;
    let mut capability_by_resource =
        std::collections::BTreeMap::<String, std::collections::BTreeSet<String>>::new();
    for resource in resources {
        let resource_id = required_string(resource, "resource_id", "policy_resource")?;
        let capabilities = resource
            .get("capability_families")
            .and_then(Value::as_array)
            .ok_or_else(|| fail("policy_resource_capabilities_missing"))?;
        let mut capability_set = std::collections::BTreeSet::new();
        for capability in capabilities {
            let capability = capability
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| fail("policy_resource_capability_invalid"))?;
            if !capability_set.insert(capability.to_string()) {
                return Err(fail("policy_resource_capability_duplicate"));
            }
        }
        if capability_by_resource
            .insert(resource_id.to_string(), capability_set)
            .is_some()
        {
            return Err(fail("policy_resource_id_duplicate"));
        }
    }

    let station_id = required_string(
        policy_case
            .get("inspection_station")
            .ok_or_else(|| fail("policy_inspection_station_missing"))?,
        "station_id",
        "policy_inspection_station",
    )?;

    let scenarios = stream_case
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("scenario_streams_missing"))?;
    if scenarios.len() != 4 {
        return Err(fail("scenario_count_invalid"));
    }

    let mut scenario_ids = std::collections::BTreeSet::new();
    let mut probability_total = 0_u64;
    for scenario in scenarios {
        let scenario_id = required_string(scenario, "scenario_id", "scenario")?;
        if !scenario_ids.insert(scenario_id.to_string()) {
            return Err(fail("scenario_id_duplicate"));
        }
        let probability = scenario
            .get("probability_ppm")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| fail("scenario_probability_invalid"))?;
        probability_total = probability_total
            .checked_add(probability)
            .ok_or_else(|| fail("scenario_probability_overflow"))?;

        let observations = scenario
            .get("observations_by_epoch")
            .and_then(Value::as_array)
            .ok_or_else(|| fail("scenario_observations_missing"))?;
        if observations.len() != 4 {
            return Err(fail("scenario_observation_horizon_invalid"));
        }

        let mut source_event_ids = std::collections::BTreeSet::new();
        let mut qualification_state = std::collections::BTreeMap::<(String, String), String>::new();
        for resource in resources {
            let resource_id = required_string(resource, "resource_id", "policy_resource")?;
            let qualifications = resource
                .get("initial_qualifications")
                .and_then(Value::as_array)
                .ok_or_else(|| fail("policy_initial_qualifications_missing"))?;
            for qualification in qualifications {
                let family = required_string(qualification, "process_family", "qualification")?;
                let disposition = required_string(qualification, "disposition", "qualification")?;
                qualification_state.insert(
                    (resource_id.to_string(), family.to_string()),
                    disposition.to_string(),
                );
            }
        }

        for (epoch, observation) in observations.iter().enumerate() {
            let actual_epoch = observation
                .get("epoch")
                .and_then(Value::as_u64)
                .ok_or_else(|| fail("observation_epoch_missing_or_invalid"))?;
            if actual_epoch != epoch as u64 {
                return Err(fail("observation_epoch_order_invalid"));
            }

            let resource_events = observation
                .get("resource_events")
                .and_then(Value::as_array)
                .ok_or_else(|| fail("resource_events_missing"))?;
            for event in resource_events {
                let event_id = required_string(event, "source_event_id", "resource_event")?;
                let resource_id = required_string(event, "resource_id", "resource_event")?;
                let kind = required_string(event, "kind", "resource_event")?;
                let occurred = event
                    .get("occurred_at_epoch")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail("resource_event_occurrence_invalid"))?;
                let observed = event
                    .get("observed_at_epoch")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail("resource_event_observation_invalid"))?;
                if !source_event_ids.insert(event_id.to_string())
                    || !capability_by_resource.contains_key(resource_id)
                    || observed != epoch as u64
                    || occurred > observed
                {
                    return Err(fail("resource_event_identity_scope_or_chronology_invalid"));
                }
                let forecast_min = event.get("recovery_forecast_min_epochs");
                let forecast_max = event.get("recovery_forecast_max_epochs");
                match kind {
                    "FAILED" => {
                        let lower = forecast_min
                            .and_then(Value::as_u64)
                            .ok_or_else(|| fail("failure_recovery_forecast_missing"))?;
                        let upper = forecast_max
                            .and_then(Value::as_u64)
                            .ok_or_else(|| fail("failure_recovery_forecast_missing"))?;
                        if lower > upper {
                            return Err(fail("failure_recovery_forecast_invalid"));
                        }
                    }
                    "RESTORED" => {
                        if !forecast_min.is_some_and(Value::is_null)
                            || !forecast_max.is_some_and(Value::is_null)
                        {
                            return Err(fail("restoration_must_not_carry_recovery_forecast"));
                        }
                    }
                    _ => return Err(fail("resource_event_kind_invalid")),
                }
            }

            let updates = observation
                .get("qualification_updates")
                .and_then(Value::as_array)
                .ok_or_else(|| fail("qualification_updates_missing"))?;
            for update in updates {
                let receipt =
                    required_string(update, "evidence_receipt_id", "qualification_update")?;
                let resource_id = required_string(update, "resource_id", "qualification_update")?;
                let family = required_string(update, "process_family", "qualification_update")?;
                let previous =
                    required_string(update, "previous_disposition", "qualification_update")?;
                let next = required_string(update, "new_disposition", "qualification_update")?;
                let occurred = update
                    .get("occurred_at_epoch")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail("qualification_update_occurrence_invalid"))?;
                let observed = update
                    .get("observed_at_epoch")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail("qualification_update_observation_invalid"))?;
                if receipt.trim().is_empty()
                    || !source_event_ids.insert(receipt.to_string())
                    || observed != epoch as u64
                    || occurred > observed
                {
                    return Err(fail("qualification_update_identity_or_chronology_invalid"));
                }
                let capabilities = capability_by_resource
                    .get(resource_id)
                    .ok_or_else(|| fail("qualification_update_resource_unknown"))?;
                if !capabilities.contains(family) {
                    return Err(fail("qualification_update_process_not_supported"));
                }
                let key = (resource_id.to_string(), family.to_string());
                let state = qualification_state
                    .get_mut(&key)
                    .ok_or_else(|| fail("qualification_update_prior_state_missing"))?;
                if state != previous || previous == next {
                    return Err(fail("qualification_update_prior_state_mismatch"));
                }
                *state = next.to_string();
            }

            let inspection_events = observation
                .get("inspection_capacity_events")
                .and_then(Value::as_array)
                .ok_or_else(|| fail("inspection_capacity_events_missing"))?;
            for event in inspection_events {
                let event_id = required_string(event, "source_event_id", "inspection_event")?;
                let event_station = required_string(event, "station_id", "inspection_event")?;
                let occurred = event
                    .get("occurred_at_epoch")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail("inspection_event_occurrence_invalid"))?;
                let observed = event
                    .get("observed_at_epoch")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| fail("inspection_event_observation_invalid"))?;
                if !source_event_ids.insert(event_id.to_string())
                    || event_station != station_id
                    || observed != epoch as u64
                    || occurred > observed
                    || event
                        .get("available_capacity_units")
                        .and_then(Value::as_u64)
                        .is_none()
                {
                    return Err(fail(
                        "inspection_event_identity_scope_or_chronology_invalid",
                    ));
                }
            }
        }
    }

    if probability_total != 1_000_000 {
        return Err(fail("scenario_probabilities_must_sum_to_one_million"));
    }

    let paired_first = scenarios[0]
        .get("observations_by_epoch")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("paired_scenario_observations_missing"))?;
    let paired_second = scenarios[1]
        .get("observations_by_epoch")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("paired_scenario_observations_missing"))?;
    let visible_len = decision_epoch + 1;
    if paired_first.len() < visible_len
        || paired_second.len() < visible_len
        || paired_first[..visible_len] != paired_second[..visible_len]
    {
        return Err(fail(
            "paired_scenarios_must_share_identical_visible_history",
        ));
    }

    Ok(())
}

fn policy_static_projection(policy_case: &Value) -> Result<Value, String> {
    let required_fields = [
        "case_id",
        "horizon_epochs",
        "jobs",
        "resources",
        "setup_transitions",
        "inspection_station",
    ];
    let mut projection = serde_json::Map::new();
    for key in required_fields {
        let value = policy_case
            .get(key)
            .ok_or_else(|| fail(format!("policy_static_field_missing:{key}")))?;
        projection.insert(key.to_string(), value.clone());
    }
    Ok(Value::Object(projection))
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
    validate_stream_case(policy_case, stream_case, decision_epoch)?;
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
        decision_epoch,
        static_case: policy_static_projection(policy_case)?,
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
            "resources": [{
                "resource_id": "RESOURCE-A",
                "capability_families": ["CUTTING"],
                "initial_qualifications": [{
                    "process_family": "CUTTING",
                    "disposition": "ADMISSIBLE"
                }]
            }],
            "setup_transitions": [],
            "inspection_station": {"station_id": "INSPECTION-01"}
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
        assert!(!serialized.contains("generator_seed"));
        assert!(!serialized.contains("replication_seed"));
        assert!(!serialized.contains("paired_seed_id"));
        assert!(!serialized.contains("scenario_family_id"));
        assert!(!serialized.contains("decision_visibility_rule"));
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
    fn invalid_resource_reference_in_simulator_stream_fails_closed() {
        let policy_case = sample_policy_case();
        let mut stream_case = sample_stream_case();
        stream_case["scenarios"][0]["observations_by_epoch"][0]["resource_events"][0]["resource_id"] =
            Value::from("UNKNOWN-RESOURCE");
        assert_eq!(
            validate_stream_case(&policy_case, &stream_case, 0),
            Err("resource_event_identity_scope_or_chronology_invalid".into())
        );
    }

    #[test]
    fn event_arrival_before_occurrence_fails_closed() {
        let policy_case = sample_policy_case();
        let mut stream_case = sample_stream_case();
        stream_case["scenarios"][0]["observations_by_epoch"][1]["resource_events"][0]["occurred_at_epoch"] =
            Value::from(2_u64);
        assert_eq!(
            validate_stream_case(&policy_case, &stream_case, 1),
            Err("resource_event_identity_scope_or_chronology_invalid".into())
        );
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
    fn paired_scenario_divergence_is_checked_only_after_it_becomes_visible() {
        let policy_case = sample_policy_case();
        let mut stream_case = sample_stream_case();
        stream_case["scenarios"][1]["observations_by_epoch"][3]["resource_events"][0]["source_event_id"] =
            Value::from("LATE-DIVERGENCE");

        assert!(
            project_prefix(
                "MFG-POLICY-V02-CHALLENGE-001",
                &policy_case,
                &stream_case,
                0,
                1,
            )
            .is_ok()
        );
        assert_eq!(
            project_prefix(
                "MFG-POLICY-V02-CHALLENGE-001",
                &policy_case,
                &stream_case,
                0,
                3,
            )
            .unwrap_err(),
            "paired_scenarios_must_share_identical_visible_history"
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
        assert!(value.get("scenario_family_id").is_none());
        assert!(
            value
                .get("static_case")
                .unwrap()
                .get("generator_seed")
                .is_none()
        );
        assert!(
            value
                .get("static_case")
                .unwrap()
                .get("replication_seed")
                .is_none()
        );
        assert!(
            value
                .get("static_case")
                .unwrap()
                .get("paired_seed_id")
                .is_none()
        );
        assert!(
            value
                .get("static_case")
                .unwrap()
                .get("decision_visibility_rule")
                .is_none()
        );
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
