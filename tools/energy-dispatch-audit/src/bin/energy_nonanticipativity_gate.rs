//! Verify non-anticipativity for the frozen energy causal-policy counterexample.
//!
//! This gate checks only information consistency. It is not the physical dispatch
//! auditor: a causal trace must still pass the independent Sol-Atlas feasibility
//! audit before it can be used as qualified energy evidence.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs,
    io,
    path::Path,
};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const FIXTURE_SCHEMA: &str = "energy-nonanticipativity-counterexample-v0.1";
const REPORT_SCHEMA: &str = "energy-nonanticipativity-gate-report-v0.1";

#[derive(Clone, Debug)]
struct PrefixMeta {
    time_index: u64,
    information_set_id: String,
    signature: String,
}

#[derive(Clone, Debug, Serialize)]
struct NonAnticipativityConflict {
    time_index: u64,
    information_set_id: String,
    observable_prefix_sha256: String,
    actions: Vec<String>,
    scenarios: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PolicyVerdict {
    policy_id: String,
    reference_only: bool,
    expected_gate: String,
    actual_gate: String,
    scenario_count: usize,
    step_count: usize,
    non_anticipativity_violation_count: usize,
    conflicts: Vec<NonAnticipativityConflict>,
}

#[derive(Clone, Debug, Serialize)]
struct GateReport {
    schema_version: &'static str,
    benchmark_id: &'static str,
    status: &'static str,
    source_fixture_sha256: String,
    scenario_count: usize,
    observable_prefix_count: usize,
    policy_count: usize,
    policy_verdicts: Vec<PolicyVerdict>,
    declared_negative_control_count: usize,
    negative_controls_executed: bool,
    physical_feasibility_audit: &'static str,
    measurement_scope: &'static str,
    claim_ceiling: &'static str,
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value.get(key).and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("missing string field {key}")))
}

fn required_u64(value: &Value, key: &str) -> Result<u64, Box<dyn Error>> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| invalid(format!("missing unsigned field {key}")))
}

fn canonical_prefix_signature(prefix: &Value) -> Result<String, Box<dyn Error>> {
    let time_index = required_u64(prefix, "time_index")?;
    let observable_state = prefix.get("observable_state")
        .ok_or_else(|| invalid("observable prefix has no observable_state"))?;
    if !observable_state.is_object() {
        return Err(invalid("observable_state must be an object"));
    }
    // Only policy-visible context participates. A prefix ID and information-set
    // label are deliberately excluded so neither can vouch for itself.
    let available_forecast = prefix.get("available_forecast").cloned().unwrap_or(Value::Null);
    let canonical = json!({
        "time_index": time_index,
        "observable_state": observable_state,
        "available_forecast": available_forecast
    });
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(sha256_hex(&bytes))
}

fn build_prefix_index(fixture: &Value) -> Result<BTreeMap<String, PrefixMeta>, Box<dyn Error>> {
    let prefixes = fixture["observable_prefixes"].as_array()
        .ok_or_else(|| invalid("observable_prefixes must be an array"))?;
    if prefixes.is_empty() {
        return Err(invalid("observable_prefixes must not be empty"));
    }

    let mut index = BTreeMap::<String, PrefixMeta>::new();
    let mut signature_to_information_set = BTreeMap::<String, String>::new();
    let mut information_set_to_signature = BTreeMap::<String, String>::new();

    for prefix in prefixes {
        let id = required_str(prefix, "prefix_id")?.to_owned();
        let information_set_id = required_str(prefix, "expected_information_set")?.to_owned();
        if id.is_empty() || information_set_id.is_empty() {
            return Err(invalid("prefix and information-set IDs must be non-empty"));
        }
        let time_index = required_u64(prefix, "time_index")?;
        let signature = canonical_prefix_signature(prefix)?;

        if let Some(previous) = signature_to_information_set.get(&signature) {
            if previous != &information_set_id {
                return Err(invalid(format!(
                    "equivalent observable prefixes map to different information sets: {previous} vs {information_set_id}"
                )));
            }
        } else {
            signature_to_information_set.insert(signature.clone(), information_set_id.clone());
        }
        if let Some(previous) = information_set_to_signature.get(&information_set_id) {
            if previous != &signature {
                return Err(invalid(format!(
                    "information set {information_set_id} combines unequal observable prefixes"
                )));
            }
        } else {
            information_set_to_signature.insert(information_set_id.clone(), signature.clone());
        }

        if index.insert(id.clone(), PrefixMeta {
            time_index,
            information_set_id,
            signature,
        }).is_some() {
            return Err(invalid(format!("duplicate observable prefix ID {id}")));
        }
    }
    Ok(index)
}

fn expected_scenarios(fixture: &Value) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let scenarios = fixture["scenarios"].as_array()
        .ok_or_else(|| invalid("scenarios must be an array"))?;
    if scenarios.is_empty() {
        return Err(invalid("scenarios must not be empty"));
    }
    let mut ids = BTreeSet::new();
    for scenario in scenarios {
        let id = required_str(scenario, "scenario_id")?.to_owned();
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(invalid(format!("empty or duplicate scenario ID {id}")));
        }
    }
    Ok(ids)
}

fn evaluate_policy(
    policy: &Value,
    prefixes: &BTreeMap<String, PrefixMeta>,
    expected_scenario_ids: &BTreeSet<String>,
    known_actions: &BTreeSet<String>,
    expected_time_indices: &BTreeSet<u64>,
) -> Result<PolicyVerdict, Box<dyn Error>> {
    let policy_id = required_str(policy, "policy_id")?.to_owned();
    let reference_only = policy["reference_only"].as_bool()
        .ok_or_else(|| invalid(format!("policy {policy_id} lacks reference_only boolean")))?;
    let expected_gate = required_str(policy, "expected_nonanticipativity_gate")?.to_owned();
    if !["pass", "reject_as_deployable_policy"].contains(&expected_gate.as_str()) {
        return Err(invalid(format!("policy {policy_id} declares unsupported expected gate {expected_gate}")));
    }
    if (expected_gate == "reject_as_deployable_policy") != reference_only {
        return Err(invalid(format!("policy {policy_id} reference_only and expected gate disagree")));
    }

    let traces = policy["traces"].as_array()
        .ok_or_else(|| invalid(format!("policy {policy_id} traces must be an array")))?;
    if traces.len() != expected_scenario_ids.len() {
        return Err(invalid(format!("policy {policy_id} does not have one trace per registered scenario")));
    }

    // signature -> action -> scenarios where that action was selected
    let mut actions_by_prefix = BTreeMap::<String, BTreeMap<String, BTreeSet<String>>>::new();
    let mut info_by_prefix = BTreeMap::<String, (u64, String)>::new();
    let mut seen_scenarios = BTreeSet::<String>::new();
    let mut total_steps = 0_usize;

    for trace in traces {
        let scenario_id = required_str(trace, "scenario_id")?.to_owned();
        if !expected_scenario_ids.contains(&scenario_id) || !seen_scenarios.insert(scenario_id.clone()) {
            return Err(invalid(format!("policy {policy_id} has unknown or duplicate scenario trace {scenario_id}")));
        }
        let steps = trace["steps"].as_array()
            .ok_or_else(|| invalid(format!("policy {policy_id}/{scenario_id} steps must be an array")))?;
        if steps.len() != expected_time_indices.len() {
            return Err(invalid(format!("policy {policy_id}/{scenario_id} has incomplete time coverage")));
        }
        let mut seen_times = BTreeSet::new();

        for step in steps {
            let time_index = required_u64(step, "time_index")?;
            if !expected_time_indices.contains(&time_index) || !seen_times.insert(time_index) {
                return Err(invalid(format!("policy {policy_id}/{scenario_id} has an unknown or duplicate time index {time_index}")));
            }
            let prefix_id = required_str(step, "observable_prefix_id")?;
            let information_set_id = required_str(step, "information_set_id")?.to_owned();
            let action = required_str(step, "action")?.to_owned();
            if !known_actions.contains(&action) {
                return Err(invalid(format!("policy {policy_id}/{scenario_id} uses unknown action {action}")));
            }
            let prefix = prefixes.get(prefix_id)
                .ok_or_else(|| invalid(format!("policy {policy_id}/{scenario_id} references unknown prefix {prefix_id}")))?;
            if prefix.time_index != time_index {
                return Err(invalid(format!("prefix {prefix_id} time index disagrees with trace step")));
            }
            if prefix.information_set_id != information_set_id {
                return Err(invalid(format!("forged information-set ID {information_set_id} for observable prefix {prefix_id}")));
            }

            let entry = info_by_prefix.entry(prefix.signature.clone())
                .or_insert((time_index, information_set_id.clone()));
            if entry.0 != time_index || entry.1 != information_set_id {
                return Err(invalid("equivalent observable prefixes disagree on time or information set"));
            }
            actions_by_prefix.entry(prefix.signature.clone())
                .or_default()
                .entry(action)
                .or_default()
                .insert(scenario_id.clone());
            total_steps += 1;
        }
        if seen_times != *expected_time_indices {
            return Err(invalid(format!("policy {policy_id}/{scenario_id} does not cover each registered time exactly once")));
        }
    }

    if seen_scenarios != *expected_scenario_ids {
        return Err(invalid(format!("policy {policy_id} is missing registered scenarios")));
    }

    let mut conflicts = Vec::new();
    for (signature, by_action) in actions_by_prefix {
        if by_action.len() > 1 {
            let (time_index, information_set_id) = info_by_prefix.get(&signature)
                .ok_or_else(|| invalid("internal error: missing prefix information"))?;
            let mut actions = by_action.keys().cloned().collect::<Vec<_>>();
            actions.sort();
            let mut scenarios = by_action.values()
                .flat_map(|ids| ids.iter().cloned()).collect::<BTreeSet<_>>()
                .into_iter().collect::<Vec<_>>();
            scenarios.sort();
            conflicts.push(NonAnticipativityConflict {
                time_index: *time_index,
                information_set_id: information_set_id.clone(),
                observable_prefix_sha256: signature,
                actions,
                scenarios,
            });
        }
    }

    let actual_gate = if conflicts.is_empty() {
        "pass"
    } else if reference_only {
        "reject_as_deployable_policy"
    } else {
        "fail"
    }.to_owned();

    if actual_gate != expected_gate {
        return Err(invalid(format!(
            "policy {policy_id} expected gate {expected_gate} but observed {actual_gate}"
        )));
    }

    Ok(PolicyVerdict {
        policy_id,
        reference_only,
        expected_gate,
        actual_gate,
        scenario_count: seen_scenarios.len(),
        step_count: total_steps,
        non_anticipativity_violation_count: conflicts.len(),
        conflicts,
    })
}

fn check_fixture(fixture: &Value, source_digest: String) -> Result<GateReport, Box<dyn Error>> {
    if required_str(fixture, "schema_version")? != FIXTURE_SCHEMA {
        return Err(invalid("unsupported non-anticipativity fixture schema"));
    }
    let claim_ceiling = required_str(fixture, "claim_ceiling")?;
    if !claim_ceiling.contains("not site data") {
        return Err(invalid("fixture is missing the synthetic-only claim ceiling"));
    }

    let scenario_ids = expected_scenarios(fixture)?;
    let prefixes = build_prefix_index(fixture)?;
    let expected_time_indices = prefixes.values().map(|prefix| prefix.time_index).collect::<BTreeSet<_>>();
    if expected_time_indices.is_empty() {
        return Err(invalid("fixture has no time indices"));
    }
    let known_actions = fixture["action_catalog"].as_object()
        .ok_or_else(|| invalid("action_catalog must be an object"))?
        .keys().cloned().collect::<BTreeSet<_>>();
    if known_actions.is_empty() {
        return Err(invalid("action catalog is empty"));
    }

    let policies = fixture["known_answer_traces"].as_array()
        .ok_or_else(|| invalid("known_answer_traces must be an array"))?;
    if policies.is_empty() {
        return Err(invalid("fixture has no policy traces"));
    }
    let mut policy_verdicts = Vec::new();
    let mut seen_policy_ids = BTreeSet::new();
    for policy in policies {
        let id = required_str(policy, "policy_id")?.to_owned();
        if !seen_policy_ids.insert(id.clone()) {
            return Err(invalid(format!("duplicate policy ID {id}")));
        }
        policy_verdicts.push(evaluate_policy(
            policy, &prefixes, &scenario_ids, &known_actions, &expected_time_indices,
        )?);
    }

    let hold_policy = policy_verdicts.iter()
        .find(|policy| policy.policy_id == "causal_policy_hold_for_critical")
        .ok_or_else(|| invalid("fixture lacks causal hold policy"))?;
    if hold_policy.actual_gate != "pass" {
        return Err(invalid("causal hold policy did not pass"));
    }

    let declared_controls = fixture["required_negative_controls"].as_array()
        .ok_or_else(|| invalid("required_negative_controls must be an array"))?;
    let declared_negative_control_count = declared_controls.len();
    if declared_negative_control_count < 8 {
        return Err(invalid("fixture must declare at least eight negative controls"));
    }

    Ok(GateReport {
        schema_version: REPORT_SCHEMA,
        benchmark_id: "energy-causal-policy-vs-wait-and-see-v0.1",
        status: "pass",
        source_fixture_sha256: source_digest,
        scenario_count: scenario_ids.len(),
        observable_prefix_count: prefixes.len(),
        policy_count: policy_verdicts.len(),
        policy_verdicts,
        declared_negative_control_count,
        negative_controls_executed: false,
        physical_feasibility_audit: "NOT_RUN_BY_THIS_BINARY; accepted causal traces require the independent dispatch audit",
        measurement_scope: "Frozen synthetic information-consistency fixture. The binary verifies that its known-answer causal traces pass and its explicitly non-causal wait-and-see reference is detected; it does not evaluate energy dispatch or physical feasibility.",
        claim_ceiling: "Synthetic finite counterexample only. No site resilience, calibrated forecast skill, safety certification, or industry outperformance claim.",
    })
}

fn run_report(input: &Path, output: &Path) -> Result<GateReport, Box<dyn Error>> {
    let input_bytes = fs::read(input)?;
    let source_digest = sha256_hex(&input_bytes);
    let fixture: Value = serde_json::from_slice(&input_bytes)?;
    let report = check_fixture(&fixture, source_digest)?;
    let output_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(output, &output_bytes)?;
    println!("PASS: qualified {} policy traces across {} scenarios and {} observable prefixes",
        report.policy_count, report.scenario_count, report.observable_prefix_count);
    println!("source_fixture_sha256={}", report.source_fixture_sha256);
    println!("declared_negative_control_count={}", report.declared_negative_control_count);
    println!("negative_controls_executed=false");
    println!("physical_feasibility_audit=NOT_RUN_BY_THIS_BINARY");
    println!("report_sha256={}", sha256_hex(&output_bytes));
    Ok(report)
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: energy_nonanticipativity_gate <counterexample-fixture.json> <output-report.json>");
        std::process::exit(2);
    }
    if let Err(error) = run_report(Path::new(&args[1]), Path::new(&args[2])) {
        eprintln!("energy non-anticipativity gate failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_BYTES: &[u8] = include_bytes!("../../fixtures/energy-nonanticipativity-counterexample-v0.1.json");

    fn fixture() -> Value {
        serde_json::from_slice(FIXTURE_BYTES).expect("frozen fixture must be valid JSON")
    }

    fn report_for(fixture: &Value) -> Result<GateReport, Box<dyn Error>> {
        check_fixture(fixture, sha256_hex(&serde_json::to_vec(fixture)?))
    }

    fn policy_mut<'a>(fixture: &'a mut Value, id: &str) -> &'a mut Value {
        fixture["known_answer_traces"].as_array_mut().unwrap()
            .iter_mut()
            .find(|policy| policy["policy_id"].as_str() == Some(id))
            .unwrap()
    }

    #[test]
    fn golden_fixture_passes_and_detects_wait_and_see_reference() {
        let report = report_for(&fixture()).unwrap();
        assert_eq!(report.status, "pass");
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.policy_count, 3);
        assert_eq!(report.declared_negative_control_count, 8);
        assert!(!report.negative_controls_executed);
        let oracle = report.policy_verdicts.iter()
            .find(|policy| policy.policy_id == "wait_and_see_oracle_noncausal_reference").unwrap();
        assert_eq!(oracle.actual_gate, "reject_as_deployable_policy");
        assert_eq!(oracle.non_anticipativity_violation_count, 1);
        assert_eq!(oracle.conflicts[0].time_index, 0);
    }

    #[test]
    fn causal_policies_pass_and_can_react_after_observation_diverges() {
        let report = report_for(&fixture()).unwrap();
        for id in ["causal_policy_hold_for_critical", "causal_policy_discharge_to_flexible"] {
            let policy = report.policy_verdicts.iter().find(|p| p.policy_id == id).unwrap();
            assert_eq!(policy.actual_gate, "pass");
            assert_eq!(policy.non_anticipativity_violation_count, 0);
        }
        let fixture = fixture();
        let hold = fixture["known_answer_traces"].as_array().unwrap()
            .iter().find(|p| p["policy_id"].as_str() == Some("causal_policy_hold_for_critical")).unwrap();
        let early = hold["traces"].as_array().unwrap().iter()
            .find(|t| t["scenario_id"].as_str() == Some("early_outage")).unwrap();
        let normal = hold["traces"].as_array().unwrap().iter()
            .find(|t| t["scenario_id"].as_str() == Some("no_outage")).unwrap();
        assert_ne!(early["steps"][1]["action"], normal["steps"][1]["action"]);
    }

    #[test]
    fn conflicting_t0_action_in_a_declared_causal_policy_fails() {
        let mut f = fixture();
        let policy = policy_mut(&mut f, "causal_policy_hold_for_critical");
        let normal = policy["traces"].as_array_mut().unwrap().iter_mut()
            .find(|t| t["scenario_id"].as_str() == Some("no_outage")).unwrap();
        normal["steps"][0]["action"] = Value::String("discharge_to_flexible".to_owned());
        // Re-point the later observable state to the state implied by discharge.
        normal["steps"][1]["observable_prefix_id"] = Value::String("no-outage-revealed-soc0".to_owned());
        normal["steps"][1]["information_set_id"] = Value::String("no-outage-revealed-soc0-t1".to_owned());
        assert!(report_for(&f).is_err());
    }

    #[test]
    fn forged_information_set_label_fails_closed() {
        let mut f = fixture();
        let policy = policy_mut(&mut f, "causal_policy_hold_for_critical");
        let normal = policy["traces"].as_array_mut().unwrap().iter_mut()
            .find(|t| t["scenario_id"].as_str() == Some("no_outage")).unwrap();
        normal["steps"][0]["information_set_id"] = Value::String("forged-future-aware-set".to_owned());
        assert!(report_for(&f).is_err());
    }

    #[test]
    fn same_observable_prefix_cannot_be_split_into_two_information_sets() {
        let mut f = fixture();
        let mut alias = f["observable_prefixes"].as_array().unwrap()[0].clone();
        alias["prefix_id"] = Value::String("forged-alias-of-common-prefix".to_owned());
        alias["expected_information_set"] = Value::String("different-information-set".to_owned());
        f["observable_prefixes"].as_array_mut().unwrap().push(alias);
        assert!(report_for(&f).is_err());
    }

    #[test]
    fn missing_or_duplicate_time_steps_fail_closed() {
        let mut missing = fixture();
        let policy = policy_mut(&mut missing, "causal_policy_hold_for_critical");
        let normal = policy["traces"].as_array_mut().unwrap().iter_mut()
            .find(|t| t["scenario_id"].as_str() == Some("no_outage")).unwrap();
        normal["steps"].as_array_mut().unwrap().pop();
        assert!(report_for(&missing).is_err());

        let mut duplicate = fixture();
        let policy = policy_mut(&mut duplicate, "causal_policy_hold_for_critical");
        let normal = policy["traces"].as_array_mut().unwrap().iter_mut()
            .find(|t| t["scenario_id"].as_str() == Some("no_outage")).unwrap();
        let first = normal["steps"][0].clone();
        normal["steps"].as_array_mut().unwrap().push(first);
        assert!(report_for(&duplicate).is_err());
    }

    #[test]
    fn missing_or_duplicate_scenario_trace_fails_closed() {
        let mut missing = fixture();
        let policy = policy_mut(&mut missing, "causal_policy_hold_for_critical");
        policy["traces"].as_array_mut().unwrap().pop();
        assert!(report_for(&missing).is_err());

        let mut duplicate = fixture();
        let policy = policy_mut(&mut duplicate, "causal_policy_hold_for_critical");
        let first = policy["traces"][0].clone();
        policy["traces"].as_array_mut().unwrap().push(first);
        assert!(report_for(&duplicate).is_err());
    }

    #[test]
    fn a_noncausal_oracle_cannot_be_relabelled_as_a_passing_policy() {
        let mut f = fixture();
        let oracle = policy_mut(&mut f, "wait_and_see_oracle_noncausal_reference");
        oracle["reference_only"] = Value::Bool(false);
        oracle["expected_nonanticipativity_gate"] = Value::String("pass".to_owned());
        assert!(report_for(&f).is_err());
    }

    #[test]
    fn modified_fixture_is_bound_to_a_different_source_digest() {
        let original = fixture();
        let original_digest = sha256_hex(&serde_json::to_vec(&original).unwrap());
        let mut changed = original;
        changed["claim_ceiling"] = Value::String("changed".to_owned());
        let changed_digest = sha256_hex(&serde_json::to_vec(&changed).unwrap());
        assert_ne!(original_digest, changed_digest);
        assert!(report_for(&changed).is_err());
    }
}
