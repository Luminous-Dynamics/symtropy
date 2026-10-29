use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const SEARCH_BUDGET: usize = 38;
const REPLAY_BUDGET: usize = 2;
const TOTAL_BUDGET: usize = SEARCH_BUDGET + REPLAY_BUDGET;
const FAILURE_FAMILIES: [&str; 2] = ["critical_energy", "communications_partition"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswerFixture {
    schema_version: String,
    artifact_kind: String,
    evidence_class: String,
    status: String,
    oracle_id: String,
    cases: Vec<KnownAnswerCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswerCase {
    case_id: String,
    x: f64,
    y: f64,
    expected_failure_families: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct OracleOutcome {
    energy_margin: f64,
    communications_margin: f64,
    failure_families: Vec<&'static str>,
}

fn oracle(point: Point) -> OracleOutcome {
    let energy_margin = 1.4 - point.x - point.y;
    // The gate passes if either condition is satisfied. Its signed margin is
    // negative only in the analytically specified low-x/high-y failure region.
    let communications_margin = (point.x - 0.2).max(0.7 - point.y);
    let mut failure_families = Vec::new();
    if energy_margin < 0.0 {
        failure_families.push("critical_energy");
    }
    if communications_margin < 0.0 {
        failure_families.push("communications_partition");
    }
    OracleOutcome {
        energy_margin,
        communications_margin,
        failure_families,
    }
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && (0.0..=1.0).contains(&point.x)
        && (0.0..=1.0).contains(&point.y)
}

fn verify_known_answer_fixture() -> Result<usize, String> {
    let fixture: KnownAnswerFixture = serde_json::from_str(include_str!(
        "../../fixtures/analytic-counterexample-known-answer-2026.json"
    )).map_err(|err| format!("known-answer fixture JSON invalid: {err}"))?;
    if fixture.schema_version != "0.1.0"
        || fixture.artifact_kind != "analytic-counterexample-known-answer-fixture"
        || fixture.evidence_class != "synthetic_known_answer"
        || fixture.status != "design_only_not_physical_evidence"
        || fixture.oracle_id != "two_gate_analytic_oracle_v0"
    {
        return Err("unsupported analytic known-answer fixture header".into());
    }
    if fixture.cases.is_empty() {
        return Err("known-answer fixture must contain at least one case".into());
    }
    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.clone()) {
            return Err("known-answer fixture has an empty or duplicate case ID".into());
        }
        let point = Point { x: case.x, y: case.y };
        if !valid_point(point) {
            return Err(format!("known-answer point is outside the declared domain: {}", case.case_id));
        }
        let allowed: BTreeSet<&str> = FAILURE_FAMILIES.iter().copied().collect();
        let expected: Vec<&str> = case.expected_failure_families.iter().map(String::as_str).collect();
        if expected.iter().any(|family| !allowed.contains(family))
            || expected.iter().copied().collect::<BTreeSet<_>>().len() != expected.len()
        {
            return Err(format!("unknown or duplicate expected failure family: {}", case.case_id));
        }
        let actual = oracle(point).failure_families;
        if actual != expected {
            return Err(format!(
                "analytic oracle disagrees with independent known-answer fixture for {}: actual={actual:?}, expected={expected:?}",
                case.case_id
            ));
        }
    }
    Ok(fixture.cases.len())
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9e3779b97f4a7c15 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

fn uniform_random(seed: u64, budget: usize) -> Vec<Point> {
    let mut rng = XorShift64::new(seed);
    (0..budget)
        .map(|_| Point { x: rng.unit(), y: rng.unit() })
        .collect()
}

fn halton(mut index: u64, base: u64) -> f64 {
    let mut fraction = 1.0;
    let mut result = 0.0;
    while index > 0 {
        fraction /= base as f64;
        result += fraction * (index % base) as f64;
        index /= base;
    }
    result
}

fn space_filling(seed_offset: u64, budget: usize) -> Vec<Point> {
    (0..budget)
        .map(|i| {
            let index = seed_offset + i as u64 + 1;
            Point { x: halton(index, 2), y: halton(index, 3) }
        })
        .collect()
}

fn stratified_grid(budget: usize) -> Vec<Point> {
    let mut points = Vec::new();
    let columns = 5;
    let rows = 8;
    for row in 0..rows {
        for column in 0..columns {
            points.push(Point {
                x: (column as f64 + 0.5) / columns as f64,
                y: (row as f64 + 0.5) / rows as f64,
            });
        }
    }
    points.truncate(budget);
    points
}

fn energy_only_diagnostic(seed: u64, budget: usize) -> Vec<Point> {
    let mut rng = XorShift64::new(seed);
    (0..budget)
        .map(|_| Point {
            x: 0.76 + 0.24 * rng.unit(),
            y: 0.76 + 0.24 * rng.unit(),
        })
        .collect()
}

fn execute_arm(arm_id: &str, role: &str, proposals: Vec<Point>) -> Value {
    assert_eq!(proposals.len(), SEARCH_BUDGET, "arm must use the identical search budget");
    let mut discovered = BTreeSet::new();
    let mut first_failure_query: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut first_failure_point: BTreeMap<&'static str, Point> = BTreeMap::new();
    let mut failure_observations = 0usize;
    let mut invalid_points = 0usize;

    for (index, point) in proposals.into_iter().enumerate() {
        if !valid_point(point) {
            invalid_points += 1;
            continue;
        }
        let outcome = oracle(point);
        if !outcome.failure_families.is_empty() {
            failure_observations += 1;
        }
        for family in outcome.failure_families {
            discovered.insert(family);
            first_failure_query.entry(family).or_insert(index + 1);
            first_failure_point.entry(family).or_insert(point);
        }
    }

    // Two replay slots are reserved inside the same fixed 40-call budget.
    // If the search misses a family, that slot becomes a known-safe positive
    // control; it is not silently reassigned to additional search calls.
    let safe_control = Point { x: 0.5, y: 0.5 };
    let mut replay_confirmed = BTreeSet::new();
    let mut replay_mismatches = 0usize;
    let mut positive_control_passes = 0usize;
    for family in FAILURE_FAMILIES {
        let replay_point = first_failure_point.get(family).copied().unwrap_or(safe_control);
        let replay_outcome = oracle(replay_point);
        if first_failure_point.contains_key(family) {
            if replay_outcome.failure_families == vec![family] {
                replay_confirmed.insert(family);
            } else {
                replay_mismatches += 1;
            }
        } else if replay_outcome.failure_families.is_empty() {
            positive_control_passes += 1;
        } else {
            replay_mismatches += 1;
        }
    }

    let discovered_vec: Vec<&str> = discovered.iter().copied().collect();
    let confirmed_vec: Vec<&str> = replay_confirmed.iter().copied().collect();
    let first_indices: BTreeMap<&str, usize> = first_failure_query
        .iter()
        .map(|(family, index)| (*family, *index))
        .collect();

    json!({
        "arm_id": arm_id,
        "role": role,
        "search_query_count": SEARCH_BUDGET,
        "replay_query_count": REPLAY_BUDGET,
        "total_oracle_invocations": SEARCH_BUDGET + REPLAY_BUDGET,
        "budget_respected": SEARCH_BUDGET + REPLAY_BUDGET == TOTAL_BUDGET,
        "invalid_search_points": invalid_points,
        "failure_observations": failure_observations,
        "unique_failure_families_discovered": discovered_vec,
        "first_failure_query_index_by_family": first_indices,
        "replay_confirmed_failure_families": confirmed_vec,
        "positive_control_replays_passed": positive_control_passes,
        "replay_mismatch_count": replay_mismatches,
        "all_discovered_families_replay_confirmed": replay_mismatches == 0 && replay_confirmed == discovered,
        "performance_claim": false
    })
}

fn build_report() -> Value {
    let known_answer_case_count = verify_known_answer_fixture()
        .expect("analytic known-answer fixture must pass before emitting the smoke report");
    let arms = vec![
        execute_arm("uniform_random_seeded", "simple_baseline", uniform_random(0xA11CE, SEARCH_BUDGET)),
        execute_arm("halton_space_filling", "simple_baseline", space_filling(0, SEARCH_BUDGET)),
        execute_arm("stratified_grid", "simple_baseline", stratified_grid(SEARCH_BUDGET)),
        execute_arm("energy_only_corner", "diagnostic_single_gate_control", energy_only_diagnostic(0xC0FFEE, SEARCH_BUDGET)),
    ];
    json!({
        "schema_version": "0.1.0",
        "artifact_kind": "symtropy-analytic-known-answer-counterexample-track-report",
        "status": "executed_analytic_oracle_only",
        "evidence_class": "synthetic_known_answer",
        "oracle": {
            "domain": {"x_min": 0.0, "x_max": 1.0, "y_min": 0.0, "y_max": 1.0},
            "failure_families": [
                {"family_id": "critical_energy", "condition": "x + y > 1.4", "signed_margin": "1.4 - x - y"},
                {"family_id": "communications_partition", "condition": "x < 0.2 and y > 0.7", "signed_margin": "max(x - 0.2, 0.7 - y)"}
            ],
            "boundary_equalities_are_passes": true,
            "deterministic": true
        },
        "known_answer_fixture_validation": {
            "status": "passed",
            "fixture_path": "research/polar-infrastructure/benchmark/fixtures/analytic-counterexample-known-answer-2026.json",
            "case_count": known_answer_case_count,
            "searcher_sampling_not_used_for_expected_labels": true
        },
        "budget_contract": {
            "search_queries_per_arm": SEARCH_BUDGET,
            "reserved_replay_queries_per_arm": REPLAY_BUDGET,
            "total_oracle_invocations_per_arm": TOTAL_BUDGET,
            "replay_queries_count_against_budget": true,
            "unused_replay_slots_use_known_safe_positive_control": true
        },
        "arms": arms,
        "claim_policy": {
            "physical_claim": false,
            "controller_superiority_claim": false,
            "industry_sota_claim": false,
            "purpose": "exercise fixed-budget accounting, analytic boundary semantics, failure-family coverage and exact replay plumbing"
        },
        "result_scope": "finite deterministic toy oracle only; not a physical plant, validated controller or industry comparison"
    })
}

fn main() {
    println!("{}", build_report());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_known_answer_fixture_matches_the_analytic_oracle() {
        assert_eq!(verify_known_answer_fixture().unwrap(), 7);
    }

    #[test]
    fn analytic_boundaries_have_known_gate_semantics() {
        assert!(oracle(Point { x: 0.7, y: 0.7 }).failure_families.is_empty());
        assert_eq!(oracle(Point { x: 0.7001, y: 0.7 }).failure_families, vec!["critical_energy"]);
        assert!(oracle(Point { x: 0.2, y: 0.8 }).failure_families.is_empty());
        assert_eq!(oracle(Point { x: 0.199, y: 0.701 }).failure_families, vec!["communications_partition"]);
    }

    #[test]
    fn all_searchers_respect_domain_and_fixed_budget() {
        let candidates = [
            uniform_random(7, SEARCH_BUDGET),
            space_filling(4, SEARCH_BUDGET),
            stratified_grid(SEARCH_BUDGET),
            energy_only_diagnostic(8, SEARCH_BUDGET),
        ];
        for points in candidates {
            assert_eq!(points.len(), SEARCH_BUDGET);
            assert!(points.iter().copied().all(valid_point));
        }
        assert_eq!(TOTAL_BUDGET, 40);
    }

    #[test]
    fn grid_finds_and_replays_both_analytic_failure_families() {
        let arm = execute_arm("stratified_grid", "simple_baseline", stratified_grid(SEARCH_BUDGET));
        assert_eq!(arm["unique_failure_families_discovered"].as_array().unwrap().len(), 2);
        assert_eq!(arm["replay_confirmed_failure_families"].as_array().unwrap().len(), 2);
        assert_eq!(arm["total_oracle_invocations"], TOTAL_BUDGET);
        assert_eq!(arm["replay_mismatch_count"], 0);
    }

    #[test]
    fn single_gate_diagnostic_exposes_missed_failure_family() {
        let arm = execute_arm("energy_only_corner", "diagnostic_single_gate_control", energy_only_diagnostic(8, SEARCH_BUDGET));
        assert_eq!(arm["unique_failure_families_discovered"].as_array().unwrap().len(), 1);
        assert_eq!(arm["unique_failure_families_discovered"][0], "critical_energy");
        assert_eq!(arm["positive_control_replays_passed"], 1);
    }

    #[test]
    fn report_keeps_toy_results_out_of_physical_claims() {
        let report = build_report();
        assert_eq!(report["status"], "executed_analytic_oracle_only");
        assert_eq!(report["evidence_class"], "synthetic_known_answer");
        assert_eq!(report["claim_policy"]["physical_claim"], false);
        assert_eq!(report["claim_policy"]["industry_sota_claim"], false);
        for arm in report["arms"].as_array().unwrap() {
            assert_eq!(arm["total_oracle_invocations"], TOTAL_BUDGET);
            assert_eq!(arm["performance_claim"], false);
            assert_eq!(arm["all_discovered_families_replay_confirmed"], true);
        }
    }

    #[test]
    fn case_boundaries_do_not_create_a_failure_family_overlap() {
        assert_eq!(oracle(Point { x: 0.1, y: 0.9 }).failure_families, vec!["communications_partition"]);
        assert_eq!(oracle(Point { x: 0.9, y: 0.9 }).failure_families, vec!["critical_energy"]);
    }
}
