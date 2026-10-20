use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const MAX_EXACT_ROADS: usize = 16;
const EPSILON: f64 = 1e-9;
const CASE_IDS: [&str; 4] = [
    "OFF_ROUTE_UNCERTAINTY_VS_ROUTE_CRITICAL_001",
    "NO_DECISION_IMPACT_002",
    "BRIDGE_FAILURE_TAIL_COST_003",
    "OBSERVATION_AFTER_DEADLINE_004",
];

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct ProbeScore {
    id: String,
    road_id: String,
    entropy: f64,
    expected_cost_after: f64,
    gross_voi: f64,
    inspection_cost: f64,
    net_voi: f64,
    available_at_step: u64,
    eligible_by_deadline: bool,
}

fn string<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn array<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: expected array"))
}

fn integer(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}

fn finite_number(value: &Value, key: &str, at: &str) -> Check<f64> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{at}.{key}: expected finite number"))?;
    if !number.is_finite() {
        return Err(format!("{at}.{key}: expected finite number"));
    }
    Ok(number)
}

fn entropy(p_open: f64) -> f64 {
    let open = if p_open > 0.0 {
        -p_open * p_open.ln()
    } else {
        0.0
    };
    let closed_probability = 1.0 - p_open;
    let closed = if closed_probability > 0.0 {
        -closed_probability * closed_probability.ln()
    } else {
        0.0
    };
    open + closed
}

fn road_probabilities(case: &Value) -> Check<(Vec<String>, Vec<f64>, BTreeMap<String, usize>)> {
    let case_id = string(case, "case_id", "case")?;
    let roads = array(case, "roads", case_id)?;
    if roads.is_empty() || roads.len() > MAX_EXACT_ROADS {
        return Err(format!(
            "{case_id}: exact enumeration requires 1..={MAX_EXACT_ROADS} roads"
        ));
    }
    let mut ids = Vec::new();
    let mut probabilities = Vec::new();
    let mut index = BTreeMap::new();
    for (i, road) in roads.iter().enumerate() {
        let road_at = format!("{case_id}.roads[{i}]");
        let id = string(road, "id", &road_at)?.to_owned();
        let p = finite_number(road, "p_open", &road_at)?;
        if id.is_empty() || index.insert(id.clone(), i).is_some() {
            return Err(format!("{road_at}: empty or duplicate road ID"));
        }
        if !(0.0..=1.0).contains(&p) {
            return Err(format!("{road_at}.p_open must be in [0,1]"));
        }
        ids.push(id);
        probabilities.push(p);
    }
    Ok((ids, probabilities, index))
}

fn expected_best_cost(case: &Value, fixed: Option<(&str, bool)>) -> Check<f64> {
    let case_id = string(case, "case_id", "case")?;
    let roads = array(case, "roads", case_id)?;
    let routes = array(case, "routes", case_id)?;
    let (road_ids, probabilities, road_index) = road_probabilities(case)?;
    if routes.is_empty() {
        return Err(format!("{case_id}: at least one route is required"));
    }
    let fixed_index = if let Some((road_id, _)) = fixed {
        Some(*road_index.get(road_id).ok_or_else(|| {
            format!("{case_id}: fixed observation references unknown road {road_id}")
        })?)
    } else {
        None
    };

    let world_count = 1u64 << roads.len();
    let mut worlds: Vec<(Vec<bool>, f64)> = Vec::new();
    for mask in 0..world_count {
        let mut states = Vec::with_capacity(roads.len());
        let mut probability = 1.0;
        for (i, p_open) in probabilities.iter().enumerate() {
            let is_open = mask & (1u64 << i) != 0;
            states.push(is_open);
            probability *= if is_open { *p_open } else { 1.0 - *p_open };
        }
        if probability <= 0.0 {
            continue;
        }
        if let (Some(index), Some((_, required_state))) = (fixed_index, fixed) {
            if states[index] != required_state {
                continue;
            }
        }
        worlds.push((states, probability));
    }

    let total_probability: f64 = worlds.iter().map(|(_, p)| *p).sum();
    if total_probability <= 0.0 {
        return Err(format!(
            "{case_id}: conditional road state has zero probability"
        ));
    }

    let mut best = f64::INFINITY;
    for route in routes {
        let route_id = string(route, "id", case_id)?;
        let travel = integer(route, "travel_cost", route_id)? as f64;
        let blocked = integer(route, "blocked_cost", route_id)? as f64;
        let required_roads = array(route, "required_road_ids", route_id)?;
        let mut weighted_cost = 0.0;
        for (states, world_probability) in &worlds {
            let mut all_open = true;
            for required_id in required_roads {
                let required_id = required_id
                    .as_str()
                    .ok_or_else(|| format!("{route_id}: required road ID must be a string"))?;
                let index = road_index
                    .get(required_id)
                    .ok_or_else(|| format!("{route_id}: unknown required road {required_id}"))?;
                if !states[*index] {
                    all_open = false;
                    break;
                }
            }
            weighted_cost += world_probability * if all_open { travel } else { blocked };
        }
        best = best.min(weighted_cost / total_probability);
    }
    let _ = road_ids;
    Ok(best)
}

fn expected_cost_after_probe(case: &Value, road_id: &str, p_open: f64) -> Check<f64> {
    let mut expected = 0.0;
    if p_open > 0.0 {
        expected += p_open * expected_best_cost(case, Some((road_id, true)))?;
    }
    if p_open < 1.0 {
        expected += (1.0 - p_open) * expected_best_cost(case, Some((road_id, false)))?;
    }
    Ok(expected)
}

fn probe_scores(case: &Value) -> Check<(f64, Vec<ProbeScore>)> {
    let case_id = string(case, "case_id", "case")?;
    let deadline = integer(case, "decision_deadline_step", case_id)?;
    let observations = array(case, "observations", case_id)?;
    let (_, probabilities, road_index) = road_probabilities(case)?;
    let roads = array(case, "roads", case_id)?;
    let probability_by_id: BTreeMap<String, f64> = roads
        .iter()
        .zip(probabilities.iter())
        .map(|(road, p)| {
            (
                road.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                *p,
            )
        })
        .collect();
    let before = expected_best_cost(case, None)?;
    let mut scores = Vec::with_capacity(observations.len());

    for (i, observation) in observations.iter().enumerate() {
        let at = format!("{case_id}.observations[{i}]");
        let id = string(observation, "id", &at)?.to_owned();
        let road_id = string(observation, "road_id", &at)?.to_owned();
        let p_open = *probability_by_id
            .get(&road_id)
            .ok_or_else(|| format!("{at}: observation references unknown road {road_id}"))?;
        if !road_index.contains_key(&road_id) {
            return Err(format!(
                "{at}: observation references unknown road {road_id}"
            ));
        }
        let cost = finite_number(observation, "inspection_cost", &at)?;
        if cost < 0.0 {
            return Err(format!("{at}.inspection_cost must be non-negative"));
        }
        let available = integer(observation, "available_at_step", &at)?;
        let after = expected_cost_after_probe(case, &road_id, p_open)?;
        let mut gross = before - after;
        if gross.abs() < EPSILON {
            gross = 0.0;
        }
        scores.push(ProbeScore {
            id,
            road_id,
            entropy: entropy(p_open),
            expected_cost_after: after,
            gross_voi: gross,
            inspection_cost: cost,
            net_voi: gross - cost,
            available_at_step: available,
            eligible_by_deadline: available <= deadline,
        });
    }
    Ok((before, scores))
}

fn selected_entropy(scores: &[ProbeScore], budget: u64) -> Option<&ProbeScore> {
    if budget == 0 {
        return None;
    }
    let mut candidates: Vec<&ProbeScore> = scores
        .iter()
        .filter(|score| score.eligible_by_deadline)
        .collect();
    candidates.sort_by(|a, b| {
        b.entropy
            .partial_cmp(&a.entropy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    candidates.first().copied()
}

fn selected_by_value(scores: &[ProbeScore], budget: u64) -> Option<&ProbeScore> {
    if budget == 0 {
        return None;
    }
    let mut candidates: Vec<&ProbeScore> = scores
        .iter()
        .filter(|score| score.eligible_by_deadline && score.net_voi > EPSILON)
        .collect();
    candidates.sort_by(|a, b| {
        b.net_voi
            .partial_cmp(&a.net_voi)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    candidates.first().copied()
}

fn evaluate_case(case: &Value, index: usize) -> Check<Value> {
    let case_id = string(case, "case_id", "case")?;
    let budget = integer(case, "probe_budget", case_id)?;
    let deadline = integer(case, "decision_deadline_step", case_id)?;
    let (baseline_expected_cost, scores) = probe_scores(case)?;
    let entropy_pick = selected_entropy(&scores, budget);
    let voi_pick = selected_by_value(&scores, budget);
    let expected = case
        .get("oracle_truth")
        .and_then(|truth| truth.get("expected_observation_id"))
        .and_then(Value::as_str);
    let baseline_id = entropy_pick.map(|score| score.id.as_str());
    let voi_id = voi_pick.map(|score| score.id.as_str());
    let baseline_after = entropy_pick.map(|score| score.expected_cost_after);
    let voi_after = voi_pick.map(|score| score.expected_cost_after);
    let late_count = scores
        .iter()
        .filter(|score| !score.eligible_by_deadline)
        .count();

    let candidates: Vec<Value> = scores
        .iter()
        .map(|score| {
            json!({
                "observation_id": score.id,
                "road_id": score.road_id,
                "entropy_nats": score.entropy,
                "expected_cost_after_observation": score.expected_cost_after,
                "gross_expected_cost_reduction": score.gross_voi,
                "inspection_cost": score.inspection_cost,
                "net_value_of_information": score.net_voi,
                "available_at_step": score.available_at_step,
                "eligible_by_deadline": score.eligible_by_deadline,
                "rejection_reason": if score.eligible_by_deadline { Value::Null } else { json!("result_after_decision_deadline") }
            })
        })
        .collect();

    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "decision_deadline_step": deadline,
        "probe_budget": budget,
        "baseline_expected_cost_before_observation": baseline_expected_cost,
        "B1_entropy_only": {
            "selected_observation_id": baseline_id,
            "expected_cost_after_observation": baseline_after,
            "matches_oracle_expected_observation": baseline_id == expected
        },
        "B2_decision_aware_voi": {
            "selected_observation_id": voi_id,
            "expected_cost_after_observation": voi_after,
            "net_value_of_information": voi_pick.map(|score| score.net_voi),
            "abstained": voi_pick.is_none(),
            "abstention_reason": if voi_pick.is_some() { Value::Null } else if budget == 0 { json!("no_probe_budget") } else if scores.iter().all(|score| !score.eligible_by_deadline) { json!("no_observation_before_deadline") } else { json!("no_positive_net_decision_value") },
            "matches_oracle_expected_observation": voi_id == expected
        },
        "late_observation_count": late_count,
        "candidate_observations": candidates
    }))
}

fn validate_case(case: &Value, index: usize) -> Check<Value> {
    let at = format!("cases[{index}]");
    let case_id = string(case, "case_id", &at)?;
    let deadline = integer(case, "decision_deadline_step", &at)?;
    let budget = integer(case, "probe_budget", &at)?;
    if deadline == 0 {
        return Err(format!("{at}: deadline must be a positive step"));
    }
    if budget > 1 {
        return Err(format!(
            "{at}: v0.1 emits at most one probe; budget must be 0 or 1"
        ));
    }
    let (road_ids, _, road_index) = road_probabilities(case)?;
    let road_set: BTreeSet<String> = road_ids.into_iter().collect();
    let routes = array(case, "routes", &at)?;
    if routes.is_empty() {
        return Err(format!("{at}: at least one route is required"));
    }
    let mut route_ids = BTreeSet::new();
    for (i, route) in routes.iter().enumerate() {
        let route_at = format!("{at}.routes[{i}]");
        let id = string(route, "id", &route_at)?;
        if id.is_empty() || !route_ids.insert(id.to_owned()) {
            return Err(format!("{route_at}: empty or duplicate route ID"));
        }
        integer(route, "travel_cost", &route_at)?;
        integer(route, "blocked_cost", &route_at)?;
        let required = array(route, "required_road_ids", &route_at)?;
        for road in required {
            let road_id = road
                .as_str()
                .ok_or_else(|| format!("{route_at}: required road ID must be a string"))?;
            if !road_set.contains(road_id) {
                return Err(format!("{route_at}: unknown required road {road_id}"));
            }
        }
    }

    let observations = array(case, "observations", &at)?;
    let mut observation_ids = BTreeSet::new();
    for (i, observation) in observations.iter().enumerate() {
        let observation_at = format!("{at}.observations[{i}]");
        let id = string(observation, "id", &observation_at)?;
        if id.is_empty() || !observation_ids.insert(id.to_owned()) {
            return Err(format!(
                "{observation_at}: empty or duplicate observation ID"
            ));
        }
        let road_id = string(observation, "road_id", &observation_at)?;
        if !road_index.contains_key(road_id) {
            return Err(format!("{observation_at}: unknown road {road_id}"));
        }
        let cost = finite_number(observation, "inspection_cost", &observation_at)?;
        if cost < 0.0 {
            return Err(format!(
                "{observation_at}: inspection cost must be non-negative"
            ));
        }
        integer(observation, "available_at_step", &observation_at)?;
    }
    let expected = case
        .get("oracle_truth")
        .and_then(|truth| truth.get("expected_observation_id"))
        .ok_or_else(|| format!("{at}: missing oracle expected_observation_id field"))?;
    if let Some(expected_id) = expected.as_str() {
        if !observation_ids.contains(expected_id) {
            return Err(format!(
                "{at}: oracle labels unknown observation {expected_id}"
            ));
        }
    } else if !expected.is_null() {
        return Err(format!(
            "{at}: oracle expected_observation_id must be string or null"
        ));
    }
    Ok(case.clone())
}

fn validate_fixture(fixture: &Value) -> Check<Vec<Value>> {
    if fixture.get("schema_version").and_then(Value::as_str) != Some(VERSION)
        || fixture.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-VOI-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
    {
        return Err("fixture metadata mismatch".to_owned());
    }
    let cases = array(fixture, "cases", "fixture")?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!(
            "fixture must contain exactly {} cases",
            CASE_IDS.len()
        ));
    }
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let id = string(case, "case_id", "case")?;
        if id != CASE_IDS[i] {
            return Err(format!("case inventory/order mismatch at index {i}: {id}"));
        }
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate case ID {id}"));
        }
        let _ = validate_case(case, i)?;
        results.push(evaluate_case(case, i)?);
    }
    Ok(results)
}

fn scorecard(cases: &[Value]) -> Value {
    let mut entropy_selected = 0usize;
    let mut entropy_matches = 0usize;
    let mut voi_selected = 0usize;
    let mut voi_abstentions = 0usize;
    let mut voi_matches = 0usize;
    let mut voi_late_selections = 0usize;
    for case in cases {
        let entropy = &case["B1_entropy_only"];
        if !entropy["selected_observation_id"].is_null() {
            entropy_selected += 1;
        }
        if entropy["matches_oracle_expected_observation"].as_bool() == Some(true) {
            entropy_matches += 1;
        }
        let voi = &case["B2_decision_aware_voi"];
        if voi["selected_observation_id"].is_null() {
            voi_abstentions += 1;
        } else {
            voi_selected += 1;
            let selected_id = voi["selected_observation_id"].as_str();
            let selected = case["candidate_observations"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|candidate| candidate["observation_id"].as_str() == selected_id);
            if selected
                .is_some_and(|candidate| candidate["eligible_by_deadline"].as_bool() != Some(true))
            {
                voi_late_selections += 1;
            }
        }
        if voi["matches_oracle_expected_observation"].as_bool() == Some(true) {
            voi_matches += 1;
        }
    }
    json!({
        "case_count": cases.len(),
        "B1_entropy_only": {
            "selected_count": entropy_selected,
            "expected_observation_match_count": entropy_matches,
            "expected_observation_mismatch_count": cases.len() - entropy_matches
        },
        "B2_decision_aware_voi": {
            "selected_count": voi_selected,
            "abstention_count": voi_abstentions,
            "expected_observation_match_count": voi_matches,
            "expected_observation_mismatch_count": cases.len() - voi_matches,
            "deadline_violation_count": voi_late_selections
        },
        "claim_boundary": "exact enumeration over a tiny synthetic independent-road world only; not operational calibration"
    })
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: wildfire-voi-bench <fixture.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = &args[3];
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character hex SHA".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|error| format!("read fixture: {error}"))?;
    let fixture: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("parse fixture JSON: {error}"))?;
    let cases = validate_fixture(&fixture)?;
    let summary = scorecard(&cases);
    let receipt = json!({
        "result": "pass",
        "validator_version": VERSION,
        "benchmark_id": "WILDFIRE-VOI-001",
        "scenario_count": cases.len(),
        "fixture_sha256": format!("{:x}", Sha256::digest(&bytes)),
        "source_commit": source_commit,
        "scorecard": summary,
        "cases": cases
    });
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|error| format!("serialize receipt: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wildfire VOI benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../benchmarks/fixtures/WILDFIRE_VOI_V0_1.json"
        ))
        .expect("synthetic fixture parses")
    }

    #[test]
    fn decision_aware_probe_beats_entropy_on_route_critical_case() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(
            results[0]["B1_entropy_only"]["selected_observation_id"],
            "OBS-OFFROUTE"
        );
        assert_eq!(
            results[0]["B2_decision_aware_voi"]["selected_observation_id"],
            "OBS-MAIN"
        );
        let candidate = results[0]["candidate_observations"]
            .as_array()
            .expect("candidates")
            .iter()
            .find(|candidate| candidate["observation_id"] == "OBS-MAIN")
            .expect("critical observation");
        assert!(
            (candidate["gross_expected_cost_reduction"]
                .as_f64()
                .expect("gross value")
                - 14.0)
                .abs()
                < 1e-8
        );
        assert!(
            (candidate["net_value_of_information"]
                .as_f64()
                .expect("net value")
                - 13.0)
                .abs()
                < 1e-8
        );
    }

    #[test]
    fn abstains_when_a_probe_cannot_change_the_decision() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(
            results[1]["B1_entropy_only"]["selected_observation_id"],
            "OBS-IRRELEVANT"
        );
        assert!(results[1]["B2_decision_aware_voi"]["selected_observation_id"].is_null());
        assert_eq!(
            results[1]["B2_decision_aware_voi"]["abstention_reason"],
            "no_positive_net_decision_value"
        );
    }

    #[test]
    fn tail_cost_bottleneck_beats_off_route_entropy() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(
            results[2]["B1_entropy_only"]["selected_observation_id"],
            "OBS-OFFROUTE"
        );
        assert_eq!(
            results[2]["B2_decision_aware_voi"]["selected_observation_id"],
            "OBS-BRIDGE"
        );
        let candidate = results[2]["candidate_observations"]
            .as_array()
            .expect("candidates")
            .iter()
            .find(|candidate| candidate["observation_id"] == "OBS-BRIDGE")
            .expect("bridge observation");
        assert!(
            (candidate["gross_expected_cost_reduction"]
                .as_f64()
                .expect("gross value")
                - 45.0)
                .abs()
                < 1e-8
        );
        assert!(
            (candidate["net_value_of_information"]
                .as_f64()
                .expect("net value")
                - 42.0)
                .abs()
                < 1e-8
        );
    }

    #[test]
    fn observations_after_deadline_are_never_selected() {
        let results = validate_fixture(&fixture()).expect("fixture validates");
        assert_eq!(
            results[3]["B2_decision_aware_voi"]["selected_observation_id"],
            Value::Null
        );
        assert_eq!(
            results[3]["B2_decision_aware_voi"]["abstention_reason"],
            "no_positive_net_decision_value"
        );
        assert_eq!(results[3]["late_observation_count"], 1);
        assert_eq!(
            results[3]["candidate_observations"][1]["eligible_by_deadline"],
            false
        );
    }

    #[test]
    fn policy_output_does_not_depend_on_oracle_labels() {
        let mut case = fixture()["cases"][0].clone();
        let expected = evaluate_case(&case, 0).expect("evaluates without editing truth");
        case.as_object_mut()
            .expect("case object")
            .remove("oracle_truth");
        let without_truth = evaluate_case(&case, 0).expect("policy still evaluates");
        assert_eq!(
            expected["B2_decision_aware_voi"]["selected_observation_id"],
            without_truth["B2_decision_aware_voi"]["selected_observation_id"]
        );
    }

    #[test]
    fn input_order_does_not_change_policy_choices() {
        let mut fixture = fixture();
        let before = validate_fixture(&fixture).expect("original");
        for case in fixture["cases"].as_array_mut().expect("cases") {
            case["roads"].as_array_mut().expect("roads").reverse();
            case["routes"].as_array_mut().expect("routes").reverse();
            case["observations"]
                .as_array_mut()
                .expect("observations")
                .reverse();
        }
        let after = validate_fixture(&fixture).expect("reordered");
        for (a, b) in before.iter().zip(after.iter()) {
            assert_eq!(
                a["B1_entropy_only"]["selected_observation_id"],
                b["B1_entropy_only"]["selected_observation_id"]
            );
            assert_eq!(
                a["B2_decision_aware_voi"]["selected_observation_id"],
                b["B2_decision_aware_voi"]["selected_observation_id"]
            );
        }
    }

    #[test]
    fn zero_probe_budget_forces_both_policies_to_abstain() {
        let mut fixture = fixture();
        fixture["cases"][0]["probe_budget"] = json!(0);
        let results = validate_fixture(&fixture).expect("zero-budget case validates");
        assert!(results[0]["B1_entropy_only"]["selected_observation_id"].is_null());
        assert!(results[0]["B2_decision_aware_voi"]["selected_observation_id"].is_null());
    }

    #[test]
    fn rejects_worlds_beyond_exact_enumeration_limit() {
        let mut fixture = fixture();
        let mut roads = fixture["cases"][0]["roads"]
            .as_array()
            .expect("roads")
            .clone();
        while roads.len() <= MAX_EXACT_ROADS {
            roads.push(json!({"id":format!("EXTRA-{}", roads.len()),"p_open":0.5}));
        }
        fixture["cases"][0]["roads"] = json!(roads);
        assert!(validate_fixture(&fixture).is_err());
    }
}
