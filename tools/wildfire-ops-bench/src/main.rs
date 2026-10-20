use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const CASE_IDS: [&str; 6] = [
    "HOTSPOT-NOT-PERIMETER-001",
    "EVAC-ROAD-LOSS-002",
    "COMMON-CAUSE-LIFELINE-003",
    "MISSING-CONFLICTING-OBSERVATIONS-004",
    "ACCESSIBLE-EVACUATION-005",
    "POSTFIRE-RECOVERY-006",
];
const OBSERVATION_KINDS: [&str; 6] = [
    "hotspot_detection",
    "operational_perimeter",
    "road_status",
    "lifeline_status",
    "post_fire_severity",
    "watershed_assessment",
];
const EVIDENCE_STATUSES: [&str; 5] = ["confirmed", "reported", "unknown", "stale", "conflicted"];

type Check<T> = Result<T, String>;

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected object"))
}

fn array<'a>(value: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{at}: expected array"))
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn text<'a>(value: &'a Value, at: &str) -> Check<&'a str> {
    value
        .as_str()
        .ok_or_else(|| format!("{at}: expected string"))
}

fn text_field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    text(field(value, key, at)?, &format!("{at}.{key}"))
}

fn require_keys(value: &Value, required: &[&str], allowed: &[&str], at: &str) -> Check<()> {
    let map = object(value, at)?;
    let allowed_set: BTreeSet<&str> = allowed.iter().copied().collect();
    for key in required {
        if !map.contains_key(*key) {
            return Err(format!("{at}: missing required key {key}"));
        }
    }
    for key in map.keys() {
        if !allowed_set.contains(key.as_str()) {
            return Err(format!("{at}: unknown key {key}"));
        }
    }
    Ok(())
}

fn timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
            .iter()
            .all(|i| b[*i].is_ascii_digit())
}

fn positive_u64(value: &Value, at: &str) -> Check<u64> {
    value
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(|| format!("{at}: expected positive integer"))
}

fn nonnegative_u64(value: &Value, at: &str) -> Check<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("{at}: expected non-negative integer"))
}

fn unique_ids(items: &[Value], at: &str) -> Check<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let item_at = format!("{at}[{index}]");
        let id = text_field(item, "id", &item_at)?.to_owned();
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(format!("{item_at}: empty or duplicate ID {id}"));
        }
    }
    Ok(ids)
}

fn candidate_projection(case: &Value) -> Value {
    let as_of = case.get("as_of").and_then(Value::as_str).unwrap_or("");
    let mut observations: Vec<Value> = case
        .get("hazard_observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            item.get("observed_at")
                .and_then(Value::as_str)
                .is_some_and(|t| t <= as_of)
        })
        .cloned()
        .collect();
    observations.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });

    let mut edges: Vec<Value> = case
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            item.get("observed_at")
                .and_then(Value::as_str)
                .is_some_and(|t| t <= as_of)
        })
        .cloned()
        .collect();
    edges.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });

    let mut lifelines: Vec<Value> = case
        .get("lifelines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            item.get("observed_at")
                .and_then(Value::as_str)
                .is_some_and(|t| t <= as_of)
        })
        .cloned()
        .collect();
    lifelines.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });

    let mut shelters: Vec<Value> = case
        .get("shelters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            item.get("observed_at")
                .and_then(Value::as_str)
                .is_some_and(|t| t <= as_of)
        })
        .cloned()
        .collect();
    shelters.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });

    let mut plans: Vec<Value> = case
        .get("plans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    plans.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });

    let mut common_cause_groups: Vec<Value> = case
        .get("common_cause_groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    common_cause_groups.sort_by(|a, b| {
        a.get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
    });

    json!({
        "case_id": case.get("case_id"),
        "as_of": case.get("as_of"),
        "incident_phase": case.get("incident_phase"),
        "demand": case.get("demand"),
        "hazard_observations": observations,
        "edges": edges,
        "lifelines": lifelines,
        "shelters": shelters,
        "plans": plans,
        "common_cause_groups": common_cause_groups
    })
}

fn active_hazard_state(input: &Value) -> &'static str {
    let observations = input.get("hazard_observations").and_then(Value::as_array);
    if observations.into_iter().flatten().any(|o| {
        o.get("kind").and_then(Value::as_str) == Some("operational_perimeter")
            && o.get("status").and_then(Value::as_str) == Some("confirmed")
    }) {
        return "confirmed_operational_perimeter_observation_present";
    }
    if observations
        .into_iter()
        .flatten()
        .any(|o| o.get("kind").and_then(Value::as_str) == Some("hotspot_detection"))
    {
        return "hotspot_point_observed_perimeter_unconfirmed";
    }
    "operational_perimeter_unconfirmed"
}

fn plan_time(plan: &Value) -> u64 {
    plan.get("travel_minutes")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX)
}

fn shortest_plan<'a>(plans: &'a [Value]) -> Option<&'a Value> {
    plans.iter().min_by(|a, b| {
        plan_time(a).cmp(&plan_time(b)).then_with(|| {
            a.get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
        })
    })
}

fn feasible_plan_reasons(plan: &Value, input: &Value) -> Vec<String> {
    let mut reasons = Vec::new();
    let edge_map: BTreeMap<String, &Value> = input
        .get("edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e.get("id")?.as_str()?.to_owned(), e)))
        .collect();
    let lifeline_map: BTreeMap<String, &Value> = input
        .get("lifelines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e.get("id")?.as_str()?.to_owned(), e)))
        .collect();
    let shelter_map: BTreeMap<String, &Value> = input
        .get("shelters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| Some((e.get("id")?.as_str()?.to_owned(), e)))
        .collect();

    for edge_id in plan
        .get("edge_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        match edge_map.get(edge_id) {
            None => reasons.push(format!("edge_missing:{edge_id}")),
            Some(edge) => {
                let status = edge.get("status").and_then(Value::as_str).unwrap_or("");
                let state = edge.get("state").and_then(Value::as_str).unwrap_or("");
                if status != "confirmed" || state != "open" {
                    reasons.push(format!(
                        "edge_not_confirmed_open:{edge_id}:{status}:{state}"
                    ));
                }
            }
        }
    }
    for lifeline_id in plan
        .get("required_lifeline_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        match lifeline_map.get(lifeline_id) {
            None => reasons.push(format!("lifeline_missing:{lifeline_id}")),
            Some(lifeline) => {
                let status = lifeline.get("status").and_then(Value::as_str).unwrap_or("");
                let state = lifeline.get("state").and_then(Value::as_str).unwrap_or("");
                if status != "confirmed" || state != "available" {
                    reasons.push(format!(
                        "lifeline_not_confirmed_available:{lifeline_id}:{status}:{state}"
                    ));
                }
            }
        }
    }

    let demand = input.get("demand");
    let people = demand
        .and_then(|d| d.get("people"))
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    let assisted = demand
        .and_then(|d| d.get("assisted_trips"))
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX);
    let shelter_id = plan.get("shelter_id").and_then(Value::as_str).unwrap_or("");
    match shelter_map.get(shelter_id) {
        None => reasons.push(format!("shelter_missing:{shelter_id}")),
        Some(shelter) => {
            if shelter.get("status").and_then(Value::as_str) != Some("confirmed") {
                reasons.push(format!("shelter_capacity_not_confirmed:{shelter_id}"));
            }
            let capacity = shelter.get("capacity").and_then(Value::as_u64).unwrap_or(0);
            let accessible = shelter
                .get("accessible_capacity")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if capacity < people {
                reasons.push(format!(
                    "shelter_capacity_shortfall:{shelter_id}:{capacity}<{people}"
                ));
            }
            if accessible < assisted {
                reasons.push(format!(
                    "accessible_capacity_shortfall:{shelter_id}:{accessible}<{assisted}"
                ));
            }
        }
    }
    reasons.sort();
    reasons
}

fn evaluate_case(case: &Value, index: usize) -> Check<Value> {
    let at = format!("cases[{index}]");
    let case_id = text_field(case, "case_id", &at)?;
    let as_of = text_field(case, "as_of", &at)?;
    let input = candidate_projection(case);
    let demand = field(&input, "demand", &format!("{at}.candidate_projection"))?;
    let people = nonnegative_u64(
        field(demand, "people", &format!("{at}.demand"))?,
        &format!("{at}.demand.people"),
    )?;
    let assisted = nonnegative_u64(
        field(demand, "assisted_trips", &format!("{at}.demand"))?,
        &format!("{at}.demand.assisted_trips"),
    )?;
    let plans = array(
        field(&input, "plans", &format!("{at}.candidate_projection"))?,
        &format!("{at}.plans"),
    )?;

    let baseline_plan = shortest_plan(plans);
    let baseline_violations = baseline_plan
        .map(|p| feasible_plan_reasons(p, &input))
        .unwrap_or_default();
    let baseline_feasible = baseline_plan.is_some() && baseline_violations.is_empty();

    let mut plan_results = Vec::new();
    let mut feasible_plans: Vec<&Value> = Vec::new();
    for plan in plans {
        let reasons = feasible_plan_reasons(plan, &input);
        let feasible = reasons.is_empty();
        if feasible {
            feasible_plans.push(plan);
        }
        plan_results.push(json!({
            "plan_id": plan.get("id"),
            "travel_minutes": plan.get("travel_minutes"),
            "feasible": feasible,
            "rejection_reasons": reasons
        }));
    }
    let selected_id = feasible_plans
        .iter()
        .min_by(|a, b| {
            plan_time(a).cmp(&plan_time(b)).then_with(|| {
                a.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(b.get("id").and_then(Value::as_str).unwrap_or(""))
            })
        })
        .and_then(|p| p.get("id").and_then(Value::as_str));

    let truth = field(case, "oracle_truth", &at)?;
    let actual_blocked: BTreeSet<String> = truth
        .get("actual_blocked_edge_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let baseline_edge_violations = baseline_plan
        .map(|p| {
            p.get("edge_ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|id| actual_blocked.contains(*id))
                .count()
        })
        .unwrap_or(0);
    let expected_id = truth
        .get("expected_candidate_plan_id")
        .and_then(Value::as_str);
    let b3_matches_oracle = selected_id == expected_id;
    let b1_id = baseline_plan.and_then(|p| p.get("id").and_then(Value::as_str));
    let b1_matches_oracle = b1_id == expected_id;

    let hazard_state = active_hazard_state(&input);
    let active_postfire_observation_count = input
        .get("hazard_observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|o| {
            matches!(
                o.get("kind").and_then(Value::as_str),
                Some("post_fire_severity" | "watershed_assessment")
            )
        })
        .count();
    let common_cause_group_count = input
        .get("common_cause_groups")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "as_of": as_of,
        "candidate_projection_sha256": format!("{:x}", Sha256::digest(serde_json::to_vec(&input).unwrap_or_default())),
        "hazard_state": hazard_state,
        "active_postfire_observation_count": active_postfire_observation_count,
        "common_cause_group_count": common_cause_group_count,
        "demand": { "people": people, "assisted_trips": assisted },
        "B1_static_shortest_route": {
            "selected_plan_id": b1_id,
            "travel_minutes": baseline_plan.map(plan_time),
            "declared_feasible": baseline_feasible,
            "rejection_reasons": baseline_violations,
            "actual_blocked_edge_violations_against_oracle": baseline_edge_violations,
            "matches_oracle_expected_plan": b1_matches_oracle
        },
        "B3_evidence_bounded_feasible_choice": {
            "selected_plan_id": selected_id,
            "travel_minutes": selected_id.and_then(|id| plans.iter().find(|p| p.get("id").and_then(Value::as_str)==Some(id))).map(plan_time),
            "feasible_plan_count": feasible_plans.len(),
            "no_evidenced_feasible_plan": feasible_plans.is_empty(),
            "rejection_count": plan_results.iter().filter(|p| p.get("feasible").and_then(Value::as_bool)==Some(false)).count(),
            "candidate_plans": plan_results,
            "matches_oracle_expected_plan": b3_matches_oracle,
            "claim_boundary": "synthetic candidate-plan feasibility gate; not an evacuation recommendation"
        }
    }))
}

fn benchmark_summary(scenarios: &[Value]) -> Value {
    let mut b1_selected = 0usize;
    let mut b1_selected_infeasible = 0usize;
    let mut b1_oracle_matches = 0usize;
    let mut b1_actual_blocked_violations = 0usize;
    let mut b3_selected = 0usize;
    let mut b3_abstentions = 0usize;
    let mut b3_oracle_matches = 0usize;
    let mut b3_selected_infeasible = 0usize;

    for scenario in scenarios {
        let b1 = &scenario["B1_static_shortest_route"];
        if !b1["selected_plan_id"].is_null() {
            b1_selected += 1;
            if b1["declared_feasible"].as_bool() != Some(true) {
                b1_selected_infeasible += 1;
            }
        }
        if b1["matches_oracle_expected_plan"].as_bool() == Some(true) {
            b1_oracle_matches += 1;
        }
        b1_actual_blocked_violations += b1["actual_blocked_edge_violations_against_oracle"]
            .as_u64()
            .unwrap_or(0) as usize;

        let b3 = &scenario["B3_evidence_bounded_feasible_choice"];
        if b3["no_evidenced_feasible_plan"].as_bool() == Some(true) {
            b3_abstentions += 1;
        }
        if !b3["selected_plan_id"].is_null() {
            b3_selected += 1;
            let selected_id = b3["selected_plan_id"].as_str();
            let selected_plan = b3["candidate_plans"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|plan| plan["plan_id"].as_str() == selected_id);
            if selected_plan.is_none_or(|plan| plan["feasible"].as_bool() != Some(true)) {
                b3_selected_infeasible += 1;
            }
        }
        if b3["matches_oracle_expected_plan"].as_bool() == Some(true) {
            b3_oracle_matches += 1;
        }
    }

    json!({
        "scenario_count": scenarios.len(),
        "B1_static_shortest_route": {
            "selected_plan_count": b1_selected,
            "selected_infeasible_plan_count": b1_selected_infeasible,
            "actual_blocked_edge_violations_against_oracle": b1_actual_blocked_violations,
            "expected_choice_match_count": b1_oracle_matches,
            "expected_choice_mismatch_count": scenarios.len().saturating_sub(b1_oracle_matches)
        },
        "B3_evidence_bounded_feasible_choice": {
            "selected_plan_count": b3_selected,
            "abstention_count": b3_abstentions,
            "selected_infeasible_plan_count": b3_selected_infeasible,
            "expected_choice_match_count": b3_oracle_matches,
            "expected_choice_mismatch_count": scenarios.len().saturating_sub(b3_oracle_matches)
        },
        "claim_boundary": "synthetic fixture scorecard only; not a real-world safety or predictive-performance estimate"
    })
}

fn validate_case(case: &Value, index: usize) -> Check<Value> {
    let at = format!("cases[{index}]");
    let case_keys = [
        "case_id",
        "as_of",
        "incident_phase",
        "demand",
        "hazard_observations",
        "edges",
        "lifelines",
        "shelters",
        "plans",
        "common_cause_groups",
        "oracle_truth",
        "expected_invariants",
    ];
    require_keys(case, &case_keys, &case_keys, &at)?;
    let case_id = text_field(case, "case_id", &at)?;
    let as_of = text_field(case, "as_of", &at)?;
    if !timestamp(as_of) {
        return Err(format!("{at}.as_of: invalid UTC timestamp"));
    }
    if text_field(case, "incident_phase", &at)? != "active_response" {
        return Err(format!(
            "{at}.incident_phase: v0.1 accepts active_response only; recovery is a separate future profile"
        ));
    }

    let demand = field(case, "demand", &at)?;
    require_keys(
        demand,
        &["people", "assisted_trips"],
        &["people", "assisted_trips"],
        &format!("{at}.demand"),
    )?;
    nonnegative_u64(
        field(demand, "people", &at)?,
        &format!("{at}.demand.people"),
    )?;
    nonnegative_u64(
        field(demand, "assisted_trips", &at)?,
        &format!("{at}.demand.assisted_trips"),
    )?;

    let observations = array(
        field(case, "hazard_observations", &at)?,
        &format!("{at}.hazard_observations"),
    )?;
    let _observation_ids = unique_ids(observations, &format!("{at}.hazard_observations"))?;
    for (i, observation) in observations.iter().enumerate() {
        let p = format!("{at}.hazard_observations[{i}]");
        require_keys(
            observation,
            &["id", "kind", "status", "observed_at", "source", "claim"],
            &["id", "kind", "status", "observed_at", "source", "claim"],
            &p,
        )?;
        let kind = text_field(observation, "kind", &p)?;
        if !OBSERVATION_KINDS.contains(&kind) {
            return Err(format!("{p}: invalid observation kind {kind}"));
        }
        let status = text_field(observation, "status", &p)?;
        if !EVIDENCE_STATUSES.contains(&status) {
            return Err(format!("{p}: invalid observation status {status}"));
        }
        let observed = text_field(observation, "observed_at", &p)?;
        if !timestamp(observed) {
            return Err(format!("{p}.observed_at: invalid UTC timestamp"));
        }
        if kind == "hotspot_detection"
            && text_field(observation, "claim", &p)? != "thermal_anomaly_point"
        {
            return Err(format!("{p}: hotspot must remain a point-detection claim"));
        }
        if matches!(kind, "post_fire_severity" | "watershed_assessment") && observed <= as_of {
            return Err(format!(
                "{p}: retrospective recovery product must be after active-response as_of"
            ));
        }
    }
    let candidate = candidate_projection(case);
    let candidate_observations = candidate
        .get("hazard_observations")
        .and_then(Value::as_array)
        .expect("projection observations");
    for o in candidate_observations {
        let kind = o.get("kind").and_then(Value::as_str).unwrap_or("");
        if matches!(kind, "post_fire_severity" | "watershed_assessment") {
            return Err(format!(
                "{at}: retrospective recovery observation leaked into active response"
            ));
        }
    }
    if case_id == "HOTSPOT-NOT-PERIMETER-001"
        && active_hazard_state(&candidate) != "hotspot_point_observed_perimeter_unconfirmed"
    {
        return Err(format!(
            "{at}: hotspot-to-perimeter nonpromotion invariant failed"
        ));
    }

    let edges = array(field(case, "edges", &at)?, &format!("{at}.edges"))?;
    let edge_ids = unique_ids(edges, &format!("{at}.edges"))?;
    for (i, edge) in edges.iter().enumerate() {
        let p = format!("{at}.edges[{i}]");
        require_keys(
            edge,
            &["id", "status", "state", "observed_at"],
            &["id", "status", "state", "observed_at"],
            &p,
        )?;
        let status = text_field(edge, "status", &p)?;
        let state = text_field(edge, "state", &p)?;
        if !EVIDENCE_STATUSES.contains(&status) {
            return Err(format!("{p}: invalid edge status {status}"));
        }
        if status == "confirmed" && !matches!(state, "open" | "blocked") {
            return Err(format!(
                "{p}: confirmed edge requires open or blocked state"
            ));
        }
        if status != "confirmed" && state != "unknown" {
            return Err(format!("{p}: nonconfirmed edge must remain unknown"));
        }
        let observed = text_field(edge, "observed_at", &p)?;
        if !timestamp(observed) || observed > as_of {
            return Err(format!(
                "{p}: edge timestamp must be valid and not after as_of"
            ));
        }
    }

    let lifelines = array(field(case, "lifelines", &at)?, &format!("{at}.lifelines"))?;
    let lifeline_ids = unique_ids(lifelines, &format!("{at}.lifelines"))?;
    for (i, line) in lifelines.iter().enumerate() {
        let p = format!("{at}.lifelines[{i}]");
        require_keys(
            line,
            &["id", "status", "state", "observed_at"],
            &["id", "status", "state", "observed_at"],
            &p,
        )?;
        let status = text_field(line, "status", &p)?;
        let state = text_field(line, "state", &p)?;
        if !EVIDENCE_STATUSES.contains(&status) {
            return Err(format!("{p}: invalid lifeline status {status}"));
        }
        if status == "confirmed" && !matches!(state, "available" | "unavailable") {
            return Err(format!(
                "{p}: confirmed lifeline requires available or unavailable state"
            ));
        }
        if status != "confirmed" && state != "unknown" {
            return Err(format!("{p}: nonconfirmed lifeline must remain unknown"));
        }
        let observed = text_field(line, "observed_at", &p)?;
        if !timestamp(observed) || observed > as_of {
            return Err(format!(
                "{p}: lifeline timestamp must be valid and not after as_of"
            ));
        }
    }

    let shelters = array(field(case, "shelters", &at)?, &format!("{at}.shelters"))?;
    let shelter_ids = unique_ids(shelters, &format!("{at}.shelters"))?;
    for (i, shelter) in shelters.iter().enumerate() {
        let p = format!("{at}.shelters[{i}]");
        require_keys(
            shelter,
            &[
                "id",
                "capacity",
                "accessible_capacity",
                "status",
                "observed_at",
            ],
            &[
                "id",
                "capacity",
                "accessible_capacity",
                "status",
                "observed_at",
            ],
            &p,
        )?;
        nonnegative_u64(field(shelter, "capacity", &p)?, &format!("{p}.capacity"))?;
        nonnegative_u64(
            field(shelter, "accessible_capacity", &p)?,
            &format!("{p}.accessible_capacity"),
        )?;
        let status = text_field(shelter, "status", &p)?;
        if !EVIDENCE_STATUSES.contains(&status) {
            return Err(format!("{p}: invalid shelter status {status}"));
        }
        let observed_at = text_field(shelter, "observed_at", &p)?;
        if !timestamp(observed_at) || observed_at > as_of {
            return Err(format!(
                "{p}: shelter-capacity timestamp must be valid and not after as_of"
            ));
        }
    }

    let plans = array(field(case, "plans", &at)?, &format!("{at}.plans"))?;
    let plan_ids = unique_ids(plans, &format!("{at}.plans"))?;
    for (i, plan) in plans.iter().enumerate() {
        let p = format!("{at}.plans[{i}]");
        require_keys(
            plan,
            &[
                "id",
                "shelter_id",
                "edge_ids",
                "travel_minutes",
                "required_lifeline_ids",
            ],
            &[
                "id",
                "shelter_id",
                "edge_ids",
                "travel_minutes",
                "required_lifeline_ids",
            ],
            &p,
        )?;
        if !shelter_ids.contains(text_field(plan, "shelter_id", &p)?) {
            return Err(format!("{p}: dangling shelter reference"));
        }
        positive_u64(
            field(plan, "travel_minutes", &p)?,
            &format!("{p}.travel_minutes"),
        )?;
        for id in array(field(plan, "edge_ids", &p)?, &format!("{p}.edge_ids"))?
            .iter()
            .map(|v| text(v, &p))
            .collect::<Check<Vec<_>>>()?
        {
            if !edge_ids.contains(id) {
                return Err(format!("{p}: dangling edge reference {id}"));
            }
        }
        for id in array(
            field(plan, "required_lifeline_ids", &p)?,
            &format!("{p}.required_lifeline_ids"),
        )?
        .iter()
        .map(|v| text(v, &p))
        .collect::<Check<Vec<_>>>()?
        {
            if !lifeline_ids.contains(id) {
                return Err(format!("{p}: dangling lifeline reference {id}"));
            }
        }
    }

    let groups = array(
        field(case, "common_cause_groups", &at)?,
        &format!("{at}.common_cause_groups"),
    )?;
    let _group_ids = unique_ids(groups, &format!("{at}.common_cause_groups"))?;
    for (i, group) in groups.iter().enumerate() {
        let p = format!("{at}.common_cause_groups[{i}]");
        require_keys(
            group,
            &[
                "id",
                "trigger",
                "affected_lifeline_ids",
                "affected_edge_ids",
                "status",
            ],
            &[
                "id",
                "trigger",
                "affected_lifeline_ids",
                "affected_edge_ids",
                "status",
            ],
            &p,
        )?;
        if text_field(group, "status", &p)? != "declared_synthetic_dependency" {
            return Err(format!("{p}: unsupported common-cause status"));
        }
        for id in array(
            field(group, "affected_lifeline_ids", &p)?,
            &format!("{p}.affected_lifeline_ids"),
        )?
        .iter()
        .map(|v| text(v, &p))
        .collect::<Check<Vec<_>>>()?
        {
            if !lifeline_ids.contains(id) {
                return Err(format!("{p}: dangling common-cause lifeline {id}"));
            }
        }
        for id in array(
            field(group, "affected_edge_ids", &p)?,
            &format!("{p}.affected_edge_ids"),
        )?
        .iter()
        .map(|v| text(v, &p))
        .collect::<Check<Vec<_>>>()?
        {
            if !edge_ids.contains(id) {
                return Err(format!("{p}: dangling common-cause edge {id}"));
            }
        }
    }

    let truth = field(case, "oracle_truth", &at)?;
    require_keys(
        truth,
        &[
            "actual_blocked_edge_ids",
            "affected_zone_ids",
            "confirmed_perimeter_exists",
            "expected_candidate_plan_id",
        ],
        &[
            "actual_blocked_edge_ids",
            "affected_zone_ids",
            "confirmed_perimeter_exists",
            "expected_candidate_plan_id",
        ],
        &format!("{at}.oracle_truth"),
    )?;
    for id in array(
        field(truth, "actual_blocked_edge_ids", &at)?,
        &format!("{at}.oracle_truth.actual_blocked_edge_ids"),
    )?
    .iter()
    .map(|v| text(v, &at))
    .collect::<Check<Vec<_>>>()?
    {
        if !edge_ids.contains(id) {
            return Err(format!("{at}.oracle_truth: dangling blocked edge {id}"));
        }
    }
    let expected = field(truth, "expected_candidate_plan_id", &at)?;
    if !expected.is_null() {
        let id = text(
            expected,
            &format!("{at}.oracle_truth.expected_candidate_plan_id"),
        )?;
        if !plan_ids.contains(id) {
            return Err(format!("{at}.oracle_truth: dangling expected plan {id}"));
        }
    }
    if !field(truth, "confirmed_perimeter_exists", &at)?.is_boolean() {
        return Err(format!(
            "{at}.oracle_truth.confirmed_perimeter_exists: expected boolean"
        ));
    }
    let invariants = array(
        field(case, "expected_invariants", &at)?,
        &format!("{at}.expected_invariants"),
    )?;
    if invariants.is_empty() {
        return Err(format!("{at}.expected_invariants: must not be empty"));
    }
    let mut seen = BTreeSet::new();
    for invariant in invariants {
        let value = text(invariant, &format!("{at}.expected_invariants"))?;
        if !seen.insert(value.to_owned()) {
            return Err(format!("{at}.expected_invariants: duplicate invariant"));
        }
    }

    let result = evaluate_case(case, index)?;
    Ok(result)
}

fn validate_corpus(corpus: &Value) -> Check<Vec<Value>> {
    let keys = [
        "schema_version",
        "benchmark_id",
        "classification",
        "created_at",
        "units",
        "source_boundaries",
        "allowed_observation_kinds",
        "allowed_statuses",
        "cases",
    ];
    require_keys(corpus, &keys, &keys, "corpus")?;
    if text_field(corpus, "schema_version", "corpus")? != "0.1.0"
        || text_field(corpus, "benchmark_id", "corpus")? != "WILDFIRE-OPS-001"
        || text_field(corpus, "classification", "corpus")? != "synthetic_only"
    {
        return Err("corpus: identity/version/classification mismatch".to_owned());
    }
    let created = text_field(corpus, "created_at", "corpus")?;
    if !timestamp(created) {
        return Err("corpus.created_at: invalid UTC timestamp".to_owned());
    }
    let allowed_kinds: BTreeSet<String> = array(
        field(corpus, "allowed_observation_kinds", "corpus")?,
        "corpus.allowed_observation_kinds",
    )?
    .iter()
    .map(|v| text(v, "corpus.allowed_observation_kinds").map(str::to_owned))
    .collect::<Check<Vec<String>>>()?
    .into_iter()
    .collect();
    let expected_kinds: BTreeSet<String> =
        OBSERVATION_KINDS.iter().map(|s| (*s).to_owned()).collect();
    if allowed_kinds != expected_kinds {
        return Err("corpus: observation kind vocabulary mismatch".to_owned());
    }
    let allowed_statuses: BTreeSet<String> = array(
        field(corpus, "allowed_statuses", "corpus")?,
        "corpus.allowed_statuses",
    )?
    .iter()
    .map(|v| text(v, "corpus.allowed_statuses").map(str::to_owned))
    .collect::<Check<Vec<String>>>()?
    .into_iter()
    .collect();
    let expected_statuses: BTreeSet<String> =
        EVIDENCE_STATUSES.iter().map(|s| (*s).to_owned()).collect();
    if allowed_statuses != expected_statuses {
        return Err("corpus: evidence status vocabulary mismatch".to_owned());
    }
    let boundaries = field(corpus, "source_boundaries", "corpus")?;
    require_keys(
        boundaries,
        &["active_input_rule", "oracle_truth_is_hidden", "rationale"],
        &["active_input_rule", "oracle_truth_is_hidden", "rationale"],
        "corpus.source_boundaries",
    )?;
    if boundaries
        .get("oracle_truth_is_hidden")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("corpus.source_boundaries: oracle truth must be hidden".to_owned());
    }
    let cases = array(field(corpus, "cases", "corpus")?, "corpus.cases")?;
    let ids: Vec<String> = cases
        .iter()
        .enumerate()
        .map(|(i, c)| text_field(c, "case_id", &format!("cases[{i}]")).map(str::to_owned))
        .collect::<Check<Vec<String>>>()?;
    if ids.len() != CASE_IDS.len()
        || ids
            .iter()
            .zip(CASE_IDS.iter())
            .any(|(a, b)| a.as_str() != *b)
    {
        return Err(format!(
            "corpus: case inventory/order must equal {CASE_IDS:?}"
        ));
    }
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("corpus: duplicate case ID".to_owned());
    }
    cases
        .iter()
        .enumerate()
        .map(|(i, c)| validate_case(c, i))
        .collect()
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: wildfire-ops-bench <fixture.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = &args[3];
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character hex SHA".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|e| format!("read {}: {e}", args[1]))?;
    let fixture: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse fixture JSON: {e}"))?;
    let scenarios = validate_corpus(&fixture)?;
    let summary = benchmark_summary(&scenarios);
    let receipt = json!({
        "result":"pass",
        "validator_version":VERSION,
        "benchmark_id":"WILDFIRE-OPS-001",
        "scenario_count":scenarios.len(),
        "corpus_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "source_commit":source_commit,
        "benchmark_summary":summary,
        "scenarios":scenarios
    });
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|e| format!("serialize receipt: {e}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wildfire-ops-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../benchmarks/fixtures/WILDFIRE_OPS_V0_1.json");
    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("synthetic fixture parses")
    }

    #[test]
    fn corpus_scorecard_quantifies_baseline_failures_and_b3_abstention() {
        let scenarios = validate_corpus(&fixture()).expect("fixture must validate");
        let summary = benchmark_summary(&scenarios);
        assert_eq!(summary["scenario_count"].as_u64(), Some(6));
        assert_eq!(
            summary["B1_static_shortest_route"]["selected_plan_count"].as_u64(),
            Some(5)
        );
        assert_eq!(
            summary["B1_static_shortest_route"]["selected_infeasible_plan_count"].as_u64(),
            Some(5)
        );
        assert_eq!(
            summary["B1_static_shortest_route"]["actual_blocked_edge_violations_against_oracle"]
                .as_u64(),
            Some(2)
        );
        assert_eq!(
            summary["B1_static_shortest_route"]["expected_choice_match_count"].as_u64(),
            Some(1)
        );
        assert_eq!(
            summary["B3_evidence_bounded_feasible_choice"]["selected_plan_count"].as_u64(),
            Some(3)
        );
        assert_eq!(
            summary["B3_evidence_bounded_feasible_choice"]["abstention_count"].as_u64(),
            Some(3)
        );
        assert_eq!(
            summary["B3_evidence_bounded_feasible_choice"]["selected_infeasible_plan_count"]
                .as_u64(),
            Some(0)
        );
        assert_eq!(
            summary["B3_evidence_bounded_feasible_choice"]["expected_choice_match_count"].as_u64(),
            Some(6)
        );
        assert_eq!(
            summary["B3_evidence_bounded_feasible_choice"]["expected_choice_mismatch_count"]
                .as_u64(),
            Some(0)
        );
    }

    #[test]
    fn frozen_six_case_corpus_passes() {
        let scenarios = validate_corpus(&fixture()).expect("fixture must validate");
        assert_eq!(scenarios.len(), 6);
        assert!(scenarios.iter().all(|s| s["status"] == "pass"));
    }

    #[test]
    fn hotspot_is_not_promoted_to_perimeter() {
        let scenarios = validate_corpus(&fixture()).expect("fixture validates");
        assert_eq!(
            scenarios[0]["hazard_state"].as_str(),
            Some("hotspot_point_observed_perimeter_unconfirmed")
        );
    }

    #[test]
    fn static_shortest_route_fails_road_loss_case_but_b3_chooses_alternate() {
        let scenarios = validate_corpus(&fixture()).expect("fixture validates");
        let road = &scenarios[1];
        assert_eq!(
            road["B1_static_shortest_route"]["selected_plan_id"].as_str(),
            Some("PLAN-FAST")
        );
        assert_eq!(
            road["B1_static_shortest_route"]["declared_feasible"].as_bool(),
            Some(false)
        );
        assert_eq!(
            road["B1_static_shortest_route"]["actual_blocked_edge_violations_against_oracle"]
                .as_u64(),
            Some(1)
        );
        assert_eq!(
            road["B3_evidence_bounded_feasible_choice"]["selected_plan_id"].as_str(),
            Some("PLAN-ALT")
        );
    }

    #[test]
    fn common_cause_lifeline_dependencies_are_enforced() {
        let scenarios = validate_corpus(&fixture()).expect("fixture validates");
        let case = &scenarios[2];
        assert_eq!(case["common_cause_group_count"].as_u64(), Some(1));
        assert_eq!(
            case["B1_static_shortest_route"]["selected_plan_id"].as_str(),
            Some("PLAN-DEPENDENT")
        );
        assert_eq!(
            case["B3_evidence_bounded_feasible_choice"]["selected_plan_id"].as_str(),
            Some("PLAN-RESILIENT")
        );
    }

    #[test]
    fn missing_and_conflicted_road_evidence_causes_abstention() {
        let scenarios = validate_corpus(&fixture()).expect("fixture validates");
        let case = &scenarios[3]["B3_evidence_bounded_feasible_choice"];
        assert_eq!(case["no_evidenced_feasible_plan"].as_bool(), Some(true));
        assert_eq!(case["selected_plan_id"], Value::Null);
    }

    #[test]
    fn accessible_capacity_is_a_hard_constraint() {
        let scenarios = validate_corpus(&fixture()).expect("fixture validates");
        let case = &scenarios[4];
        assert_eq!(
            case["B1_static_shortest_route"]["selected_plan_id"].as_str(),
            Some("PLAN-NEAR")
        );
        assert_eq!(
            case["B1_static_shortest_route"]["declared_feasible"].as_bool(),
            Some(false)
        );
        assert_eq!(
            case["B3_evidence_bounded_feasible_choice"]["selected_plan_id"].as_str(),
            Some("PLAN-ACCESSIBLE")
        );
    }

    #[test]
    fn stale_shelter_capacity_rejects_plan_even_when_route_is_open() {
        let mut corpus = fixture();
        corpus["cases"][4]["shelters"][1]["status"] = json!("stale");
        let scenarios =
            validate_corpus(&corpus).expect("stale shelter status is an admissible evidence state");
        let result = &scenarios[4]["B3_evidence_bounded_feasible_choice"];
        assert_eq!(result["selected_plan_id"], Value::Null);
        assert_eq!(result["no_evidenced_feasible_plan"].as_bool(), Some(true));
    }

    #[test]
    fn rejects_future_shelter_capacity_observation() {
        let mut corpus = fixture();
        corpus["cases"][4]["shelters"][1]["observed_at"] = json!("2026-07-05T12:05:00Z");
        let error =
            validate_corpus(&corpus).expect_err("future shelter capacity is not admissible");
        assert!(error.contains("shelter-capacity timestamp must be valid and not after as_of"));
    }

    #[test]
    fn future_postfire_evidence_is_absent_from_active_projection() {
        let scenarios = validate_corpus(&fixture()).expect("fixture validates");
        assert_eq!(
            scenarios[5]["active_postfire_observation_count"].as_u64(),
            Some(0)
        );
        assert_eq!(
            scenarios[5]["B3_evidence_bounded_feasible_choice"]["no_evidenced_feasible_plan"]
                .as_bool(),
            Some(true)
        );
    }

    #[test]
    fn oracle_truth_and_expected_invariants_never_enter_projection() {
        let corpus = fixture();
        let projection = candidate_projection(&corpus["cases"][1]);
        assert!(projection.get("oracle_truth").is_none());
        assert!(projection.get("expected_invariants").is_none());
    }

    #[test]
    fn candidate_projection_is_order_invariant() {
        let mut corpus = fixture();
        let original = validate_corpus(&corpus).expect("original fixture validates");
        for case in corpus["cases"].as_array_mut().expect("cases") {
            case["hazard_observations"]
                .as_array_mut()
                .expect("obs")
                .reverse();
            case["edges"].as_array_mut().expect("edges").reverse();
            case["lifelines"]
                .as_array_mut()
                .expect("lifelines")
                .reverse();
            case["shelters"].as_array_mut().expect("shelters").reverse();
            case["plans"].as_array_mut().expect("plans").reverse();
            case["common_cause_groups"]
                .as_array_mut()
                .expect("groups")
                .reverse();
        }
        let reordered = validate_corpus(&corpus).expect("reordered fixture validates");
        assert_eq!(original, reordered);
    }

    #[test]
    fn rejects_unknown_road_state_disguised_as_open() {
        let mut corpus = fixture();
        corpus["cases"][1]["edges"][0]["status"] = json!("unknown");
        let error = validate_corpus(&corpus).expect_err("unknown road cannot stay open");
        assert!(error.contains("nonconfirmed edge must remain unknown"));
    }

    #[test]
    fn rejects_recovery_observation_that_would_be_current() {
        let mut corpus = fixture();
        corpus["cases"][5]["hazard_observations"][1]["observed_at"] = json!("2026-07-06T11:00:00Z");
        let error =
            validate_corpus(&corpus).expect_err("post-fire products need a later evidence time");
        assert!(
            error.contains("retrospective recovery product must be after active-response as_of")
        );
    }

    #[test]
    fn rejects_dangling_plan_edge_reference() {
        let mut corpus = fixture();
        corpus["cases"][1]["plans"][0]["edge_ids"][0] = json!("NO-SUCH-ROAD");
        let error = validate_corpus(&corpus).expect_err("dangling road ref must fail");
        assert!(error.contains("dangling edge reference"));
    }
}
