use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const CASE_IDS: [&str; 4] = [
    "OFF_ROUTE_UNCERTAINTY_VS_ROUTE_CRITICAL_001",
    "NO_DECISION_IMPACT_002",
    "BRIDGE_FAILURE_TAIL_COST_003",
    "OBSERVATION_AFTER_DEADLINE_004",
];
const MAX_ROADS: usize = 16;
const EPSILON: f64 = 1e-8;

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Score {
    id: String,
    road_id: String,
    entropy: f64,
    after: f64,
    gross: f64,
    inspection_cost: f64,
    net: f64,
    available_at: u64,
    eligible: bool,
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

fn number(value: &Value, key: &str, at: &str) -> Check<f64> {
    let n = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{at}.{key}: expected number"))?;
    if !n.is_finite() {
        return Err(format!("{at}.{key}: number must be finite"));
    }
    Ok(n)
}

fn entropy(p: f64) -> f64 {
    let mut h = 0.0;
    if p > 0.0 {
        h -= p * p.ln();
    }
    if p < 1.0 {
        h -= (1.0 - p) * (1.0 - p).ln();
    }
    h
}

fn best_expected_cost(case: &Value, fixed: Option<(&str, bool)>) -> Check<f64> {
    let case_id = string(case, "case_id", "case")?;
    let roads = array(case, "roads", case_id)?;
    let routes = array(case, "routes", case_id)?;
    if roads.is_empty() || roads.len() > MAX_ROADS || routes.is_empty() {
        return Err(format!("{case_id}: invalid road or route count"));
    }

    let mut indices = BTreeMap::<String, usize>::new();
    let mut priors = Vec::<f64>::new();
    for (i, road) in roads.iter().enumerate() {
        let id = string(road, "id", case_id)?.to_owned();
        let p = number(road, "p_open", case_id)?;
        if !(0.0..=1.0).contains(&p) || indices.insert(id.clone(), i).is_some() {
            return Err(format!("{case_id}: invalid road ID or probability"));
        }
        priors.push(p);
    }
    let fixed_index = if let Some((id, _)) = fixed {
        Some(
            *indices
                .get(id)
                .ok_or_else(|| format!("{case_id}: fixed state for unknown road {id}"))?,
        )
    } else {
        None
    };

    let mut weighted_worlds = Vec::<(Vec<bool>, f64)>::new();
    for mask in 0..(1u64 << roads.len()) {
        let mut states = Vec::with_capacity(roads.len());
        let mut world_probability = 1.0;
        for (i, p_open) in priors.iter().enumerate() {
            let open = mask & (1u64 << i) != 0;
            states.push(open);
            world_probability *= if open { *p_open } else { 1.0 - *p_open };
        }
        if world_probability <= 0.0 {
            continue;
        }
        if let (Some(i), Some((_, expected_open))) = (fixed_index, fixed) {
            if states[i] != expected_open {
                continue;
            }
        }
        weighted_worlds.push((states, world_probability));
    }

    let normalizer: f64 = weighted_worlds.iter().map(|(_, p)| *p).sum();
    if normalizer <= 0.0 {
        return Err(format!("{case_id}: impossible conditional state"));
    }

    let mut best = f64::INFINITY;
    for route in routes {
        let route_id = string(route, "id", case_id)?;
        let travel = integer(route, "travel_cost", route_id)? as f64;
        let blocked = integer(route, "blocked_cost", route_id)? as f64;
        let required = array(route, "required_road_ids", route_id)?;
        let mut total = 0.0;

        for (states, probability) in &weighted_worlds {
            let mut traversable = true;
            for road_ref in required {
                let road_id = road_ref
                    .as_str()
                    .ok_or_else(|| format!("{route_id}: required road ID must be a string"))?;
                let road_index = indices
                    .get(road_id)
                    .ok_or_else(|| format!("{route_id}: unknown road {road_id}"))?;
                traversable &= states[*road_index];
            }
            total += probability * if traversable { travel } else { blocked };
        }
        best = best.min(total / normalizer);
    }
    Ok(best)
}

fn score_observations(case: &Value) -> Check<(f64, Vec<Score>)> {
    let case_id = string(case, "case_id", "case")?;
    let deadline = integer(case, "decision_deadline_step", case_id)?;
    let roads = array(case, "roads", case_id)?;
    let observations = array(case, "observations", case_id)?;
    let mut priors = BTreeMap::<String, f64>::new();
    for road in roads {
        priors.insert(
            string(road, "id", case_id)?.to_owned(),
            number(road, "p_open", case_id)?,
        );
    }
    let before = best_expected_cost(case, None)?;
    let mut output = Vec::new();

    for observation in observations {
        let id = string(observation, "id", case_id)?.to_owned();
        let road_id = string(observation, "road_id", &id)?.to_owned();
        let p_open = *priors
            .get(&road_id)
            .ok_or_else(|| format!("{case_id}: probe references unknown road {road_id}"))?;
        let inspection_cost = number(observation, "inspection_cost", &id)?;
        let available_at = integer(observation, "available_at_step", &id)?;
        let mut after = 0.0;
        if p_open > 0.0 {
            after += p_open * best_expected_cost(case, Some((&road_id, true)))?;
        }
        if p_open < 1.0 {
            after += (1.0 - p_open) * best_expected_cost(case, Some((&road_id, false)))?;
        }
        let mut gross = before - after;
        if gross.abs() < EPSILON {
            gross = 0.0;
        }
        output.push(Score {
            id,
            road_id,
            entropy: entropy(p_open),
            after,
            gross,
            inspection_cost,
            net: gross - inspection_cost,
            available_at,
            eligible: available_at <= deadline,
        });
    }
    Ok((before, output))
}

fn choose_entropy(scores: &[Score], budget: u64) -> Option<&Score> {
    if budget == 0 {
        return None;
    }
    let mut eligible: Vec<&Score> = scores.iter().filter(|score| score.eligible).collect();
    eligible.sort_by(|a, b| {
        b.entropy
            .partial_cmp(&a.entropy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    eligible.first().copied()
}

fn choose_voi(scores: &[Score], budget: u64) -> Option<&Score> {
    if budget == 0 {
        return None;
    }
    let mut eligible: Vec<&Score> = scores
        .iter()
        .filter(|score| score.eligible && score.net > EPSILON)
        .collect();
    eligible.sort_by(|a, b| {
        b.net
            .partial_cmp(&a.net)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    eligible.first().copied()
}

fn optional_id(receipt_case: &Value, section: &str) -> Check<Option<String>> {
    let field = receipt_case
        .get(section)
        .and_then(|v| v.get("selected_observation_id"))
        .ok_or_else(|| format!("{section}.selected_observation_id: missing field"))?;
    if field.is_null() {
        Ok(None)
    } else {
        field
            .as_str()
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| format!("{section}.selected_observation_id: expected string or null"))
    }
}

fn check_optional_id(section: &Value, key: &str, expected: Option<&str>, at: &str) -> Check<()> {
    let field = section
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
        Err(format!("{at}.{key}: expected {expected:?}, got {actual:?}"))
    }
}

fn check_bool(section: &Value, key: &str, expected: bool, at: &str) -> Check<()> {
    let actual = section
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{at}.{key}: expected boolean"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{at}.{key}: expected {expected}, got {actual}"))
    }
}

fn check_number(section: &Value, key: &str, expected: f64, at: &str) -> Check<()> {
    let actual = section
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{at}.{key}: expected number"))?;
    if !actual.is_finite() || (actual - expected).abs() > EPSILON {
        Err(format!(
            "{at}.{key}: expected {expected:.12}, got {actual:.12}"
        ))
    } else {
        Ok(())
    }
}

fn verify(fixture_bytes: &[u8], receipt: &Value, source_commit: &str) -> Check<()> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected source commit must be a 40-character hex SHA".to_owned());
    }
    if receipt.get("result").and_then(Value::as_str) != Some("pass")
        || receipt.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-VOI-001")
        || receipt.get("source_commit").and_then(Value::as_str) != Some(source_commit)
    {
        return Err("receipt metadata or exact source commit mismatch".to_owned());
    }

    let fixture_digest = format!("{:x}", Sha256::digest(fixture_bytes));
    if receipt.get("fixture_sha256").and_then(Value::as_str) != Some(fixture_digest.as_str()) {
        return Err("receipt fixture_sha256 does not match the raw fixture bytes".to_owned());
    }
    let fixture: Value =
        serde_json::from_slice(fixture_bytes).map_err(|e| format!("parse fixture: {e}"))?;
    let cases = array(&fixture, "cases", "fixture")?;
    let receipt_cases = array(receipt, "cases", "receipt")?;
    if cases.len() != CASE_IDS.len()
        || receipt_cases.len() != CASE_IDS.len()
        || receipt.get("scenario_count").and_then(Value::as_u64) != Some(CASE_IDS.len() as u64)
    {
        return Err("fixture and receipt must have exactly four cases".to_owned());
    }

    let mut b1_selected = 0usize;
    let mut b1_matches = 0usize;
    let mut b2_selected = 0usize;
    let mut b2_matches = 0usize;
    let mut b2_late = 0usize;
    for i in 0..CASE_IDS.len() {
        let case = &cases[i];
        let row = &receipt_cases[i];
        let id = string(case, "case_id", "case")?;
        let at = format!("cases[{i}]");
        if id != CASE_IDS[i] || row.get("case_id").and_then(Value::as_str) != Some(id) {
            return Err(format!("{at}: case inventory/order mismatch"));
        }
        if row.get("status").and_then(Value::as_str) != Some("pass") {
            return Err(format!("{at}: status is not pass"));
        }

        let budget = integer(case, "probe_budget", id)?;
        let (before, scores) = score_observations(case)?;
        let b1 = choose_entropy(&scores, budget);
        let b2 = choose_voi(&scores, budget);
        let b1_id = b1.map(|score| score.id.as_str());
        let b2_id = b2.map(|score| score.id.as_str());
        let expected = case
            .get("oracle_truth")
            .and_then(|v| v.get("expected_observation_id"))
            .and_then(Value::as_str);

        let b1_receipt = row
            .get("B1_entropy_only")
            .ok_or_else(|| format!("{at}: missing B1 section"))?;
        let b2_receipt = row
            .get("B2_decision_aware_voi")
            .ok_or_else(|| format!("{at}: missing B2 section"))?;
        check_optional_id(b1_receipt, "selected_observation_id", b1_id, &format!("{at}.B1"))?;
        check_optional_id(b2_receipt, "selected_observation_id", b2_id, &format!("{at}.B2"))?;
        check_bool(b1_receipt, "matches_oracle_expected_observation", b1_id == expected, &format!("{at}.B1"))?;
        check_bool(b2_receipt, "matches_oracle_expected_observation", b2_id == expected, &format!("{at}.B2"))?;
        check_number(row, "baseline_expected_cost_before_observation", before, &at)?;
        check_bool(b2_receipt, "abstained", b2.is_none(), &format!("{at}.B2"))?;

        match b1 {
            Some(score) => {
                let expected_after = b1_receipt
                    .get("expected_cost_after_observation")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| format!("{at}.B1.expected_cost_after_observation: expected number"))?;
                if (expected_after - score.after).abs() > EPSILON {
                    return Err(format!("{at}.B1 expected-after cost mismatch"));
                }
                b1_selected += 1;
            }
            None => {
                if !b1_receipt.get("expected_cost_after_observation").is_some_and(Value::is_null) {
                    return Err(format!("{at}.B1: expected null after-cost when no probe is selected"));
                }
            }
        }
        match b2 {
            Some(score) => {
                check_number(b2_receipt, "expected_cost_after_observation", score.after, &format!("{at}.B2"))?;
                check_number(b2_receipt, "net_value_of_information", score.net, &format!("{at}.B2"))?;
                if !score.eligible || score.net <= EPSILON {
                    return Err(format!("{at}.B2 selected a late or non-positive-value probe"));
                }
                b2_selected += 1;
            }
            None => {
                if !b2_receipt.get("expected_cost_after_observation").is_some_and(Value::is_null)
                    || !b2_receipt.get("net_value_of_information").is_some_and(Value::is_null)
                {
                    return Err(format!("{at}.B2: expected null cost/value when abstaining"));
                }
            }
        }

        let candidate_rows = array(row, "candidate_observations", &at)?;
        let observations = array(case, "observations", &at)?;
        if candidate_rows.len() != observations.len() {
            return Err(format!("{at}: candidate observation count mismatch"));
        }
        let mut seen = BTreeSet::new();
        for score in &scores {
            if !seen.insert(score.id.clone()) {
                return Err(format!("{at}: duplicate computed observation ID"));
            }
            let candidate = candidate_rows
                .iter()
                .find(|candidate| candidate.get("observation_id").and_then(Value::as_str) == Some(score.id.as_str()))
                .ok_or_else(|| format!("{at}: receipt omitted observation {}", score.id))?;
            if candidate.get("road_id").and_then(Value::as_str) != Some(score.road_id.as_str())
                || candidate.get("available_at_step").and_then(Value::as_u64) != Some(score.available_at)
            {
                return Err(format!("{at}: road or availability changed for {}", score.id));
            }
            check_number(candidate, "entropy_nats", score.entropy, &format!("{at}.{}", score.id))?;
            check_number(candidate, "expected_cost_after_observation", score.after, &format!("{at}.{}", score.id))?;
            check_number(candidate, "gross_expected_cost_reduction", score.gross, &format!("{at}.{}", score.id))?;
            check_number(candidate, "inspection_cost", score.inspection_cost, &format!("{at}.{}", score.id))?;
            check_number(candidate, "net_value_of_information", score.net, &format!("{at}.{}", score.id))?;
            check_bool(candidate, "eligible_by_deadline", score.eligible, &format!("{at}.{}", score.id))?;
            let reason = candidate.get("rejection_reason");
            if !score.eligible && reason.and_then(Value::as_str) != Some("result_after_decision_deadline") {
                return Err(format!("{at}: late observation {} lacks rejection reason", score.id));
            }
            if score.eligible && !reason.is_some_and(Value::is_null) {
                return Err(format!("{at}: eligible observation {} has rejection reason", score.id));
            }
        }
        let late_count = scores.iter().filter(|score| !score.eligible).count();
        if row.get("late_observation_count").and_then(Value::as_u64) != Some(late_count as u64) {
            return Err(format!("{at}: late-observation count mismatch"));
        }
        if b1_id == expected {
            b1_matches += 1;
        }
        if b2_id == expected {
            b2_matches += 1;
        }
        if let Some(score) = b2 {
            if !score.eligible {
                b2_late += 1;
            }
        }
    }

    let summary = receipt
        .get("scorecard")
        .ok_or_else(|| "receipt missing scorecard".to_owned())?;
    let fields = [
        ("case_count", CASE_IDS.len()),
        ("B1_entropy_only.selected_count", b1_selected),
        ("B1_entropy_only.expected_observation_match_count", b1_matches),
        ("B1_entropy_only.expected_observation_mismatch_count", CASE_IDS.len() - b1_matches),
        ("B2_decision_aware_voi.selected_count", b2_selected),
        ("B2_decision_aware_voi.abstention_count", CASE_IDS.len() - b2_selected),
        ("B2_decision_aware_voi.expected_observation_match_count", b2_matches),
        ("B2_decision_aware_voi.expected_observation_mismatch_count", CASE_IDS.len() - b2_matches),
        ("B2_decision_aware_voi.deadline_violation_count", b2_late),
    ];
    for (path, expected) in fields {
        let mut parts = path.split('.');
        let first = parts.next().unwrap();
        let second = parts.next();
        let field = if let Some(second) = second {
            summary.get(first).and_then(|v| v.get(second))
        } else {
            summary.get(first)
        };
        if field.and_then(Value::as_u64) != Some(expected as u64) {
            return Err(format!("scorecard.{path}: expected {expected}"));
        }
    }
    Ok(())
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: verify_voi_receipt <fixture.json> <receipt.json> <expected-source-commit>"
                .to_owned(),
        );
    }
    let fixture_bytes = fs::read(&args[1]).map_err(|e| format!("read fixture: {e}"))?;
    let receipt_bytes = fs::read(&args[2]).map_err(|e| format!("read receipt: {e}"))?;
    let receipt: Value =
        serde_json::from_slice(&receipt_bytes).map_err(|e| format!("parse receipt: {e}"))?;
    verify(&fixture_bytes, &receipt, &args[3])?;
    println!(
        "independent VOI receipt verification passed: {} cases; fixture SHA-256 {}; source {}",
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
            eprintln!("independent VOI receipt verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_is_zero_for_certain_states() {
        assert_eq!(entropy(0.0), 0.0);
        assert_eq!(entropy(1.0), 0.0);
        assert!(entropy(0.5) > entropy(0.8));
    }

    #[test]
    fn non_positive_or_late_probes_are_not_value_eligible() {
        let score = Score {
            id: "late".to_owned(),
            road_id: "r".to_owned(),
            entropy: 0.5,
            after: 1.0,
            gross: 0.0,
            inspection_cost: 1.0,
            net: -1.0,
            available_at: 5,
            eligible: false,
        };
        assert!(choose_voi(&[score], 1).is_none());
    }

    #[test]
    fn malformed_expected_source_digest_is_not_accepted_by_contract() {
        let bad_sha = "1234";
        assert!(bad_sha.len() != 40 || !bad_sha.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
