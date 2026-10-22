use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

const CASE_IDS: [&str; 6] = [
    "HOTSPOT-NOT-PERIMETER-001",
    "EVAC-ROAD-LOSS-002",
    "COMMON-CAUSE-LIFELINE-003",
    "MISSING-CONFLICTING-OBSERVATIONS-004",
    "ACCESSIBLE-EVACUATION-005",
    "POSTFIRE-RECOVERY-006",
];

type Check<T> = Result<T, String>;

fn text<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: missing string"))
}

fn array<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: missing array"))
}

fn integer(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}

fn lookup<'a>(items: &'a [Value], id: &str) -> Option<&'a Value> {
    items
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(id))
}

fn current_confirmed(item: &Value, as_of: &str) -> bool {
    item.get("status").and_then(Value::as_str) == Some("confirmed")
        && item
            .get("observed_at")
            .and_then(Value::as_str)
            .is_some_and(|observed_at| observed_at <= as_of)
}

fn feasible(case: &Value, plan: &Value) -> bool {
    let Some(as_of) = case.get("as_of").and_then(Value::as_str) else {
        return false;
    };
    let Some(demand) = case.get("demand") else {
        return false;
    };
    let Some(people) = demand.get("people").and_then(Value::as_u64) else {
        return false;
    };
    let Some(assisted) = demand.get("assisted_trips").and_then(Value::as_u64) else {
        return false;
    };

    let Ok(edges) = array(case, "edges", "case") else {
        return false;
    };
    let Ok(lifelines) = array(case, "lifelines", "case") else {
        return false;
    };
    let Ok(shelters) = array(case, "shelters", "case") else {
        return false;
    };

    let Some(edge_ids) = plan.get("edge_ids").and_then(Value::as_array) else {
        return false;
    };
    for edge_id in edge_ids.iter().filter_map(Value::as_str) {
        let Some(edge) = lookup(edges, edge_id) else {
            return false;
        };
        if !current_confirmed(edge, as_of)
            || edge.get("state").and_then(Value::as_str) != Some("open")
        {
            return false;
        }
    }

    let Some(lifeline_ids) = plan.get("required_lifeline_ids").and_then(Value::as_array) else {
        return false;
    };
    for lifeline_id in lifeline_ids.iter().filter_map(Value::as_str) {
        let Some(lifeline) = lookup(lifelines, lifeline_id) else {
            return false;
        };
        if !current_confirmed(lifeline, as_of)
            || lifeline.get("state").and_then(Value::as_str) != Some("available")
        {
            return false;
        }
    }

    let Some(shelter_id) = plan.get("shelter_id").and_then(Value::as_str) else {
        return false;
    };
    let Some(shelter) = lookup(shelters, shelter_id) else {
        return false;
    };
    current_confirmed(shelter, as_of)
        && shelter
            .get("capacity")
            .and_then(Value::as_u64)
            .is_some_and(|n| n >= people)
        && shelter
            .get("accessible_capacity")
            .and_then(Value::as_u64)
            .is_some_and(|n| n >= assisted)
}

fn shortest<'a, F>(plans: &'a [Value], mut accepted: F) -> Option<&'a Value>
where
    F: FnMut(&Value) -> bool,
{
    plans
        .iter()
        .filter(|plan| accepted(plan))
        .min_by(|left, right| {
            let left_time = left
                .get("travel_minutes")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX);
            let right_time = right
                .get("travel_minutes")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX);
            left_time.cmp(&right_time).then_with(|| {
                left.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(right.get("id").and_then(Value::as_str).unwrap_or(""))
            })
        })
}

fn plan_id(plan: Option<&Value>) -> Option<&str> {
    plan.and_then(|p| p.get("id").and_then(Value::as_str))
}

fn matches_id(value: &Value, key: &str, expected: Option<&str>, at: &str) -> Check<()> {
    let field = value
        .get(key)
        .ok_or_else(|| format!("{at}.{key}: missing field"))?;
    let actual = if field.is_null() {
        None
    } else {
        Some(
            field
                .as_str()
                .ok_or_else(|| format!("{at}.{key}: expected string or null"))?,
        )
    };
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{at}.{key}: expected {expected:?}, found {actual:?}"
        ))
    }
}

fn check_count(summary: &Value, section: &str, key: &str, expected: usize) -> Check<()> {
    let actual = summary
        .get(section)
        .and_then(|v| v.get(key))
        .and_then(Value::as_u64);
    if actual == Some(expected as u64) {
        Ok(())
    } else {
        Err(format!(
            "benchmark_summary.{section}.{key}: expected {expected}, found {actual:?}"
        ))
    }
}

fn verify(fixture_bytes: &[u8], receipt: &Value, expected_source_commit: &str) -> Check<()> {
    if expected_source_commit.len() != 40
        || !expected_source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("expected source commit must be a 40-character hex SHA".to_owned());
    }
    if receipt.get("result").and_then(Value::as_str) != Some("pass") {
        return Err("receipt result is not pass".to_owned());
    }
    if receipt.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-OPS-001") {
        return Err("receipt benchmark_id mismatch".to_owned());
    }
    if receipt.get("source_commit").and_then(Value::as_str) != Some(expected_source_commit) {
        return Err("receipt source_commit does not match checked-out HEAD".to_owned());
    }

    let fixture: Value =
        serde_json::from_slice(fixture_bytes).map_err(|e| format!("parse fixture: {e}"))?;
    let fixture_digest = format!("{:x}", Sha256::digest(fixture_bytes));
    if receipt.get("corpus_sha256").and_then(Value::as_str) != Some(fixture_digest.as_str()) {
        return Err("receipt corpus_sha256 does not match fixture bytes".to_owned());
    }

    let cases = array(&fixture, "cases", "fixture")?;
    let scenarios = array(receipt, "scenarios", "receipt")?;
    if cases.len() != CASE_IDS.len() || scenarios.len() != CASE_IDS.len() {
        return Err("fixture and receipt must each contain exactly six cases".to_owned());
    }
    if receipt.get("scenario_count").and_then(Value::as_u64) != Some(CASE_IDS.len() as u64) {
        return Err("receipt scenario_count mismatch".to_owned());
    }

    let mut counts = [0usize; 9];
    for (index, expected_case_id) in CASE_IDS.iter().enumerate() {
        let case = &cases[index];
        let scenario = &scenarios[index];
        let case_id = text(case, "case_id", "case")?;
        let at = format!("scenarios[{index}]");
        if case_id != *expected_case_id
            || scenario.get("case_id").and_then(Value::as_str) != Some(*expected_case_id)
        {
            return Err(format!("{at}: case inventory/order mismatch"));
        }
        if scenario.get("status").and_then(Value::as_str) != Some("pass") {
            return Err(format!("{at}: scenario status is not pass"));
        }

        let plans = array(case, "plans", "case")?;
        let baseline = shortest(plans, |_| true);
        let policy = shortest(plans, |plan| feasible(case, plan));
        let baseline_id = plan_id(baseline);
        let policy_id = plan_id(policy);
        let expected_id = case
            .get("oracle_truth")
            .and_then(|v| v.get("expected_candidate_plan_id"))
            .and_then(Value::as_str);

        let b1 = scenario
            .get("B1_static_shortest_route")
            .ok_or_else(|| format!("{at}: missing B1 output"))?;
        let b3 = scenario
            .get("B3_evidence_bounded_feasible_choice")
            .ok_or_else(|| format!("{at}: missing B3 output"))?;

        matches_id(b1, "selected_plan_id", baseline_id, &format!("{at}.B1"))?;
        matches_id(b3, "selected_plan_id", policy_id, &format!("{at}.B3"))?;

        let baseline_feasible = baseline.is_some_and(|plan| feasible(case, plan));
        if b1.get("declared_feasible").and_then(Value::as_bool) != Some(baseline_feasible) {
            return Err(format!(
                "{at}: B1 declared_feasible does not match independent feasibility"
            ));
        }

        let feasible_count = plans.iter().filter(|plan| feasible(case, plan)).count();
        if b3.get("feasible_plan_count").and_then(Value::as_u64) != Some(feasible_count as u64)
            || b3
                .get("no_evidenced_feasible_plan")
                .and_then(Value::as_bool)
                != Some(feasible_count == 0)
        {
            return Err(format!("{at}: B3 feasible count/abstention mismatch"));
        }

        let reported_candidates = array(b3, "candidate_plans", &format!("{at}.B3"))?;
        if reported_candidates.len() != plans.len() {
            return Err(format!("{at}: B3 candidate inventory mismatch"));
        }
        for plan in plans {
            let id = text(plan, "id", "plan")?;
            let reported = reported_candidates
                .iter()
                .find(|candidate| candidate.get("plan_id").and_then(Value::as_str) == Some(id))
                .ok_or_else(|| format!("{at}: B3 omitted candidate {id}"))?;
            if reported.get("feasible").and_then(Value::as_bool) != Some(feasible(case, plan)) {
                return Err(format!("{at}: B3 feasibility mismatch for {id}"));
            }
        }

        let blocked_ids: Vec<&str> = case
            .get("oracle_truth")
            .and_then(|v| v.get("actual_blocked_edge_ids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let blocked_violations = baseline
            .and_then(|plan| plan.get("edge_ids").and_then(Value::as_array))
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|id| blocked_ids.contains(id))
            .count();
        if b1
            .get("actual_blocked_edge_violations_against_oracle")
            .and_then(Value::as_u64)
            != Some(blocked_violations as u64)
        {
            return Err(format!("{at}: B1 blocked-edge count mismatch"));
        }

        let b1_matches = baseline_id == expected_id;
        let b3_matches = policy_id == expected_id;
        if b1
            .get("matches_oracle_expected_plan")
            .and_then(Value::as_bool)
            != Some(b1_matches)
            || b3
                .get("matches_oracle_expected_plan")
                .and_then(Value::as_bool)
                != Some(b3_matches)
        {
            return Err(format!("{at}: oracle comparison mismatch"));
        }

        if baseline.is_some() {
            counts[0] += 1;
            if !baseline_feasible {
                counts[1] += 1;
            }
        }
        counts[2] += blocked_violations;
        if b1_matches {
            counts[3] += 1;
        }
        if policy.is_some() {
            counts[4] += 1;
            if !policy.is_some_and(|plan| feasible(case, plan)) {
                counts[6] += 1;
            }
        } else {
            counts[5] += 1;
        }
        if b3_matches {
            counts[7] += 1;
        }
    }

    let summary = receipt
        .get("benchmark_summary")
        .ok_or_else(|| "receipt missing benchmark_summary".to_owned())?;
    if summary.get("scenario_count").and_then(Value::as_u64) != Some(CASE_IDS.len() as u64) {
        return Err("benchmark_summary scenario_count mismatch".to_owned());
    }
    check_count(
        summary,
        "B1_static_shortest_route",
        "selected_plan_count",
        counts[0],
    )?;
    check_count(
        summary,
        "B1_static_shortest_route",
        "selected_infeasible_plan_count",
        counts[1],
    )?;
    check_count(
        summary,
        "B1_static_shortest_route",
        "actual_blocked_edge_violations_against_oracle",
        counts[2],
    )?;
    check_count(
        summary,
        "B1_static_shortest_route",
        "expected_choice_match_count",
        counts[3],
    )?;
    check_count(
        summary,
        "B1_static_shortest_route",
        "expected_choice_mismatch_count",
        CASE_IDS.len() - counts[3],
    )?;
    check_count(
        summary,
        "B3_evidence_bounded_feasible_choice",
        "selected_plan_count",
        counts[4],
    )?;
    check_count(
        summary,
        "B3_evidence_bounded_feasible_choice",
        "abstention_count",
        counts[5],
    )?;
    check_count(
        summary,
        "B3_evidence_bounded_feasible_choice",
        "selected_infeasible_plan_count",
        counts[6],
    )?;
    check_count(
        summary,
        "B3_evidence_bounded_feasible_choice",
        "expected_choice_match_count",
        counts[7],
    )?;
    check_count(
        summary,
        "B3_evidence_bounded_feasible_choice",
        "expected_choice_mismatch_count",
        CASE_IDS.len() - counts[7],
    )?;

    Ok(())
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: verify_receipt <fixture.json> <receipt.json> <expected-source-commit>"
                .to_owned(),
        );
    }
    let fixture_bytes = fs::read(&args[1]).map_err(|e| format!("read fixture: {e}"))?;
    let receipt_bytes = fs::read(&args[2]).map_err(|e| format!("read receipt: {e}"))?;
    let receipt: Value =
        serde_json::from_slice(&receipt_bytes).map_err(|e| format!("parse receipt: {e}"))?;
    verify(&fixture_bytes, &receipt, &args[3])?;
    println!(
        "independent receipt verification passed: {} cases; fixture SHA-256 {}; source {}",
        CASE_IDS.len(),
        format!("{:x}", Sha256::digest(&fixture_bytes)),
        args[3]
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("independent receipt verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_future_shelter_capacity_is_not_current() {
        let stale = serde_json::json!({
            "status":"stale",
            "observed_at":"2026-07-05T11:55:00Z"
        });
        let future = serde_json::json!({
            "status":"confirmed",
            "observed_at":"2026-07-05T12:05:00Z"
        });
        assert!(!current_confirmed(&stale, "2026-07-05T12:00:00Z"));
        assert!(!current_confirmed(&future, "2026-07-05T12:00:00Z"));
    }

    #[test]
    fn shortest_route_ties_are_broken_by_plan_id() {
        let plans = vec![
            serde_json::json!({"id":"PLAN-Z","travel_minutes":5}),
            serde_json::json!({"id":"PLAN-A","travel_minutes":5}),
        ];
        assert_eq!(plan_id(shortest(&plans, |_| true)), Some("PLAN-A"));
    }

    #[test]
    fn missing_or_unknown_edges_fail_closed() {
        let case = serde_json::json!({
            "as_of":"2026-07-02T12:00:00Z",
            "demand":{"people":10,"assisted_trips":2},
            "edges":[{"id":"R-1","status":"unknown","state":"unknown","observed_at":"2026-07-02T11:45:00Z"}],
            "lifelines":[],
            "shelters":[{"id":"S-1","capacity":10,"accessible_capacity":2,"status":"confirmed","observed_at":"2026-07-02T11:55:00Z"}]
        });
        let plan = serde_json::json!({
            "id":"P-1","shelter_id":"S-1","edge_ids":["R-1"],"required_lifeline_ids":[],"travel_minutes":10
        });
        assert!(!feasible(&case, &plan));
    }
}
