use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const EPSILON: f64 = 1e-9;
const FIXTURE: &str = include_str!("../../fixtures/ev_factory_structure_uncertainty_v0_1.json");
const CLAIM_CEILING: &str = "Synthetic structural-model decision-oracle harness only. It does not establish calibrated robustness, model-set completeness, field accuracy, industry SOTA, ROI, safety compliance, purchase approval or production release.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    reference_model_id: String,
    constraints: Constraints,
    actions: Vec<Action>,
    model_families: Vec<ModelFamily>,
    outcomes: Vec<Outcome>,
    uncertainty_contract: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Constraints {
    capex_budget_usd: f64,
    staffed_labor_hours_per_shift_limit: f64,
    active_power_kw_limit: f64,
    floor_area_sqm_limit: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    action_id: String,
    label: String,
    capex_usd: f64,
    staffed_labor_hours_per_shift: f64,
    active_power_kw: f64,
    floor_area_sqm: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelFamily {
    model_id: String,
    mechanism: String,
    structural_change: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    model_id: String,
    action_id: String,
    incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ModelGain {
    model_id: String,
    incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ActionScore {
    action_id: String,
    label: String,
    resource_feasible: bool,
    minimum_gain_across_model_families_units_per_hour: Option<f64>,
    maximum_regret_across_model_families_units_per_hour: Option<f64>,
    per_model_gains: Vec<ModelGain>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FamilyWinner {
    model_id: String,
    best_action_id: String,
    best_gain_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PairwiseRanking {
    action_a_id: String,
    action_b_id: String,
    models_preferring_a: Vec<String>,
    models_preferring_b: Vec<String>,
    tied_models: Vec<String>,
    ranking_reversal: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct LeaveOneFamilyOut {
    omitted_model_id: String,
    remaining_model_family_count: usize,
    maximin_action_id: String,
    minimax_regret_action_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    fixture_sha256: String,
    action_count: usize,
    model_family_count: usize,
    outcome_cell_count: usize,
    feasible_action_count: usize,
    infeasible_action_count: usize,
    family_probabilities_used: bool,
    model_families_treated_as_iid_replications: bool,
    reference_model_best_action_id: String,
    maximin_action_id: String,
    maximin_minimum_gain_units_per_hour: f64,
    minimax_regret_action_id: String,
    minimax_regret_upper_bound_units_per_hour: f64,
    ranking_reversal_pair_count: usize,
    action_scores: Vec<ActionScore>,
    family_winners: Vec<FamilyWinner>,
    pairwise_rankings: Vec<PairwiseRanking>,
    leave_one_family_out: Vec<LeaveOneFamilyOut>,
    independent_resource_check: bool,
    deployment_authorized: bool,
    uncertainty_contract: String,
    claim_ceiling: String,
}

fn canonical_id(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn check_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "0.1.0" {
        return Err("unsupported_schema_version".into());
    }
    if fixture.benchmark_id != "EV-FACTORY-STRUCTURE-UNCERTAINTY-001" {
        return Err("unexpected_benchmark_id".into());
    }
    if fixture.evidence_class != "synthetic" {
        return Err("evidence_class_must_be_synthetic".into());
    }
    if fixture.uncertainty_contract.trim().is_empty() || fixture.claim_ceiling != CLAIM_CEILING {
        return Err("missing_or_weakened_claim_boundary".into());
    }
    if fixture.actions.is_empty() || fixture.model_families.len() < 2 {
        return Err("requires_actions_and_at_least_two_model_families".into());
    }

    let constraint_values = [
        fixture.constraints.capex_budget_usd,
        fixture.constraints.staffed_labor_hours_per_shift_limit,
        fixture.constraints.active_power_kw_limit,
        fixture.constraints.floor_area_sqm_limit,
    ];
    if constraint_values.iter().any(|value| !finite_nonnegative(*value)) {
        return Err("constraints_must_be_finite_and_nonnegative".into());
    }

    let mut action_ids = BTreeSet::new();
    for action in &fixture.actions {
        if !canonical_id(&action.action_id)
            || action.label.trim().is_empty()
            || !action_ids.insert(action.action_id.clone())
        {
            return Err("action_id_invalid_duplicate_or_label_missing".into());
        }
        let values = [
            action.capex_usd,
            action.staffed_labor_hours_per_shift,
            action.active_power_kw,
            action.floor_area_sqm,
        ];
        if values.iter().any(|value| !finite_nonnegative(*value)) {
            return Err("action_resources_must_be_finite_and_nonnegative".into());
        }
    }

    let mut model_ids = BTreeSet::new();
    for family in &fixture.model_families {
        if !canonical_id(&family.model_id)
            || !model_ids.insert(family.model_id.clone())
            || family.mechanism.trim().is_empty()
            || family.structural_change.trim().is_empty()
        {
            return Err("model_family_id_duplicate_or_structural_manifest_missing".into());
        }
    }
    if !model_ids.contains(&fixture.reference_model_id) {
        return Err("reference_model_not_registered".into());
    }

    let expected_cells = action_ids.len() * model_ids.len();
    if fixture.outcomes.len() != expected_cells {
        return Err("outcome_catalog_incomplete_or_has_extra_rows".into());
    }
    let mut seen = BTreeSet::new();
    for outcome in &fixture.outcomes {
        if !model_ids.contains(&outcome.model_id) || !action_ids.contains(&outcome.action_id) {
            return Err("outcome_has_unknown_model_or_action_id".into());
        }
        if !outcome.incremental_good_units_per_hour.is_finite() {
            return Err("outcome_must_be_finite".into());
        }
        if !seen.insert((outcome.model_id.clone(), outcome.action_id.clone())) {
            return Err("duplicate_model_action_outcome".into());
        }
    }
    if seen.len() != expected_cells {
        return Err("outcome_catalog_has_missing_cells".into());
    }
    Ok(())
}

fn is_resource_feasible(action: &Action, constraints: &Constraints) -> bool {
    action.capex_usd <= constraints.capex_budget_usd + EPSILON
        && action.staffed_labor_hours_per_shift
            <= constraints.staffed_labor_hours_per_shift_limit + EPSILON
        && action.active_power_kw <= constraints.active_power_kw_limit + EPSILON
        && action.floor_area_sqm <= constraints.floor_area_sqm_limit + EPSILON
}

fn gain(
    outcomes: &BTreeMap<(String, String), f64>,
    model_id: &str,
    action_id: &str,
) -> f64 {
    outcomes[&(model_id.to_string(), action_id.to_string())]
}

fn compare_max(left: f64, right: f64) -> Ordering {
    left.total_cmp(&right)
}

fn family_best_action<'a>(
    actions: &[&'a Action],
    model_id: &str,
    outcomes: &BTreeMap<(String, String), f64>,
) -> &'a Action {
    let mut best = actions[0];
    let mut best_gain = gain(outcomes, model_id, &best.action_id);
    for action in &actions[1..] {
        let candidate_gain = gain(outcomes, model_id, &action.action_id);
        let ordering = compare_max(candidate_gain, best_gain);
        if ordering == Ordering::Greater
            || (ordering == Ordering::Equal && action.action_id < best.action_id)
        {
            best = action;
            best_gain = candidate_gain;
        }
    }
    best
}

fn maximin_action<'a>(
    actions: &[&'a Action],
    model_ids: &[String],
    outcomes: &BTreeMap<(String, String), f64>,
) -> (&'a Action, f64) {
    let mut best = actions[0];
    let mut best_minimum = model_ids
        .iter()
        .map(|model_id| gain(outcomes, model_id, &best.action_id))
        .fold(f64::INFINITY, f64::min);
    for action in &actions[1..] {
        let minimum = model_ids
            .iter()
            .map(|model_id| gain(outcomes, model_id, &action.action_id))
            .fold(f64::INFINITY, f64::min);
        let ordering = compare_max(minimum, best_minimum);
        if ordering == Ordering::Greater
            || (ordering == Ordering::Equal && action.action_id < best.action_id)
        {
            best = action;
            best_minimum = minimum;
        }
    }
    (best, best_minimum)
}

fn minimax_regret_action<'a>(
    actions: &[&'a Action],
    model_ids: &[String],
    outcomes: &BTreeMap<(String, String), f64>,
) -> (&'a Action, f64) {
    let best_gain_by_model: BTreeMap<String, f64> = model_ids
        .iter()
        .map(|model_id| {
            let winner = family_best_action(actions, model_id, outcomes);
            (
                model_id.clone(),
                gain(outcomes, model_id, &winner.action_id),
            )
        })
        .collect();

    let mut best = actions[0];
    let mut best_worst_regret = model_ids
        .iter()
        .map(|model_id| {
            best_gain_by_model[model_id] - gain(outcomes, model_id, &best.action_id)
        })
        .fold(0.0, f64::max);
    for action in &actions[1..] {
        let worst_regret = model_ids
            .iter()
            .map(|model_id| {
                best_gain_by_model[model_id] - gain(outcomes, model_id, &action.action_id)
            })
            .fold(0.0, f64::max);
        let ordering = worst_regret.total_cmp(&best_worst_regret);
        if ordering == Ordering::Less
            || (ordering == Ordering::Equal && action.action_id < best.action_id)
        {
            best = action;
            best_worst_regret = worst_regret;
        }
    }
    (best, best_worst_regret)
}

fn evaluate(fixture: &Fixture, source_bytes: &[u8]) -> Result<Report, String> {
    check_fixture(fixture)?;

    let outcomes: BTreeMap<(String, String), f64> = fixture
        .outcomes
        .iter()
        .map(|row| {
            (
                (row.model_id.clone(), row.action_id.clone()),
                row.incremental_good_units_per_hour,
            )
        })
        .collect();

    let mut actions: Vec<&Action> = fixture.actions.iter().collect();
    actions.sort_by(|a, b| a.action_id.cmp(&b.action_id));
    let feasible_actions: Vec<&Action> = actions
        .iter()
        .copied()
        .filter(|action| is_resource_feasible(action, &fixture.constraints))
        .collect();
    if feasible_actions.is_empty() {
        return Err("no_resource_feasible_action".into());
    }

    let mut model_ids: Vec<String> = fixture
        .model_families
        .iter()
        .map(|family| family.model_id.clone())
        .collect();
    model_ids.sort();

    let family_winners: Vec<FamilyWinner> = model_ids
        .iter()
        .map(|model_id| {
            let winner = family_best_action(&feasible_actions, model_id, &outcomes);
            FamilyWinner {
                model_id: model_id.clone(),
                best_action_id: winner.action_id.clone(),
                best_gain_units_per_hour: gain(&outcomes, model_id, &winner.action_id),
            }
        })
        .collect();

    let (maximin_action, maximin_minimum_gain) =
        maximin_action(&feasible_actions, &model_ids, &outcomes);
    let (minimax_regret_action, minimax_regret_upper_bound) =
        minimax_regret_action(&feasible_actions, &model_ids, &outcomes);
    let reference_action = family_best_action(
        &feasible_actions,
        &fixture.reference_model_id,
        &outcomes,
    );

    let best_gain_by_model: BTreeMap<String, f64> = family_winners
        .iter()
        .map(|row| (row.model_id.clone(), row.best_gain_units_per_hour))
        .collect();
    let action_scores: Vec<ActionScore> = actions
        .iter()
        .map(|action| {
            let is_feasible = is_resource_feasible(action, &fixture.constraints);
            let per_model_gains: Vec<ModelGain> = model_ids
                .iter()
                .map(|model_id| ModelGain {
                    model_id: model_id.clone(),
                    incremental_good_units_per_hour: gain(
                        &outcomes,
                        model_id,
                        &action.action_id,
                    ),
                })
                .collect();
            let minimum_gain = is_feasible.then(|| {
                per_model_gains
                    .iter()
                    .map(|row| row.incremental_good_units_per_hour)
                    .fold(f64::INFINITY, f64::min)
            });
            let maximum_regret = is_feasible.then(|| {
                per_model_gains
                    .iter()
                    .map(|row| {
                        best_gain_by_model[&row.model_id] - row.incremental_good_units_per_hour
                    })
                    .fold(0.0, f64::max)
            });
            ActionScore {
                action_id: action.action_id.clone(),
                label: action.label.clone(),
                resource_feasible: is_feasible,
                minimum_gain_across_model_families_units_per_hour: minimum_gain,
                maximum_regret_across_model_families_units_per_hour: maximum_regret,
                per_model_gains,
            }
        })
        .collect();

    let mut pairwise_rankings = Vec::new();
    for index_a in 0..feasible_actions.len() {
        for index_b in (index_a + 1)..feasible_actions.len() {
            let action_a = feasible_actions[index_a];
            let action_b = feasible_actions[index_b];
            let mut models_preferring_a = Vec::new();
            let mut models_preferring_b = Vec::new();
            let mut tied_models = Vec::new();
            for model_id in &model_ids {
                match compare_max(
                    gain(&outcomes, model_id, &action_a.action_id),
                    gain(&outcomes, model_id, &action_b.action_id),
                ) {
                    Ordering::Greater => models_preferring_a.push(model_id.clone()),
                    Ordering::Less => models_preferring_b.push(model_id.clone()),
                    Ordering::Equal => tied_models.push(model_id.clone()),
                }
            }
            pairwise_rankings.push(PairwiseRanking {
                action_a_id: action_a.action_id.clone(),
                action_b_id: action_b.action_id.clone(),
                ranking_reversal: !models_preferring_a.is_empty()
                    && !models_preferring_b.is_empty(),
                models_preferring_a,
                models_preferring_b,
                tied_models,
            });
        }
    }

    let mut leave_one_family_out = Vec::new();
    for omitted_model_id in &model_ids {
        let remaining_model_ids: Vec<String> = model_ids
            .iter()
            .filter(|model_id| *model_id != omitted_model_id)
            .cloned()
            .collect();
        let (maximin, _) =
            maximin_action(&feasible_actions, &remaining_model_ids, &outcomes);
        let (minimax_regret, _) =
            minimax_regret_action(&feasible_actions, &remaining_model_ids, &outcomes);
        leave_one_family_out.push(LeaveOneFamilyOut {
            omitted_model_id: omitted_model_id.clone(),
            remaining_model_family_count: remaining_model_ids.len(),
            maximin_action_id: maximin.action_id.clone(),
            minimax_regret_action_id: minimax_regret.action_id.clone(),
        });
    }

    let ranking_reversal_pair_count = pairwise_rankings
        .iter()
        .filter(|pair| pair.ranking_reversal)
        .count();
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: fixture.benchmark_id.clone(),
        evidence_class: fixture.evidence_class.clone(),
        fixture_sha256: format!("{:x}", Sha256::digest(source_bytes)),
        action_count: actions.len(),
        model_family_count: model_ids.len(),
        outcome_cell_count: fixture.outcomes.len(),
        feasible_action_count: feasible_actions.len(),
        infeasible_action_count: actions.len() - feasible_actions.len(),
        family_probabilities_used: false,
        model_families_treated_as_iid_replications: false,
        reference_model_best_action_id: reference_action.action_id.clone(),
        maximin_action_id: maximin_action.action_id.clone(),
        maximin_minimum_gain_units_per_hour: maximin_minimum_gain,
        minimax_regret_action_id: minimax_regret_action.action_id.clone(),
        minimax_regret_upper_bound_units_per_hour: minimax_regret_upper_bound,
        ranking_reversal_pair_count,
        action_scores,
        family_winners,
        pairwise_rankings,
        leave_one_family_out,
        independent_resource_check: true,
        deployment_authorized: false,
        uncertainty_contract: fixture.uncertainty_contract.clone(),
        claim_ceiling: fixture.claim_ceiling.clone(),
    })
}

fn parse_and_evaluate(source_bytes: &[u8]) -> Result<Report, String> {
    let fixture: Fixture = serde_json::from_slice(source_bytes)
        .map_err(|error| format!("invalid_fixture_json: {error}"))?;
    evaluate(&fixture, source_bytes)
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!(
            "usage: ev_factory_structure_uncertainty <fixture.json>\n\
             Use --help for claim boundaries and output fields."
        );
        process::exit(2);
    };
    if path == "--help" {
        println!(
            "EV-FACTORY-STRUCTURE-UNCERTAINTY-001\n\
             Usage: ev_factory_structure_uncertainty <fixture.json>\n\
             Evaluates a finite synthetic action-by-model outcome catalogue.\n\
             Reports reference-model best, maximin, minimax regret, ranking reversals,\n\
             leave-one-family-out stability and a digest of exact fixture bytes.\n\
             No model-family probabilities or IID replication assumptions are made."
        );
        return;
    }
    if args.next().is_some() {
        eprintln!("unexpected_extra_argument");
        process::exit(2);
    }

    let source_bytes = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("cannot_read_fixture: {error}");
        process::exit(2);
    });
    let report = parse_and_evaluate(&source_bytes).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize report")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("embedded fixture")
    }

    fn report() -> Report {
        parse_and_evaluate(FIXTURE.as_bytes()).expect("valid frozen fixture")
    }

    #[test]
    fn exact_model_family_oracle_pins_both_robust_choices() {
        let report = report();
        assert_eq!(report.action_count, 5);
        assert_eq!(report.model_family_count, 6);
        assert_eq!(report.outcome_cell_count, 30);
        assert_eq!(report.feasible_action_count, 5);
        assert_eq!(report.infeasible_action_count, 0);
        assert_eq!(report.reference_model_best_action_id, "add-eol-test");
        assert_eq!(report.maximin_action_id, "extend-buffer");
        assert!((report.maximin_minimum_gain_units_per_hour - 0.8).abs() < EPSILON);
        assert_eq!(report.minimax_regret_action_id, "improve-yield-rework");
        assert!((report.minimax_regret_upper_bound_units_per_hour - 0.8).abs() < EPSILON);
        assert!(!report.family_probabilities_used);
        assert!(!report.model_families_treated_as_iid_replications);
        assert!(!report.deployment_authorized);
        let winners: BTreeMap<&str, &str> = report
            .family_winners
            .iter()
            .map(|winner| (winner.model_id.as_str(), winner.best_action_id.as_str()))
            .collect();
        assert_eq!(winners["independent-equipment-faults"], "add-eol-test");
        assert_eq!(winners["common-cause-outages"], "extend-buffer");
        assert_eq!(winners["bursty-repair-times"], "extend-buffer");
        assert_eq!(winners["finite-buffer-blocking"], "extend-buffer");
        assert_eq!(winners["rework-feedback"], "improve-yield-rework");
        assert_eq!(winners["supplier-product-mix-coupling"], "improve-yield-rework");
    }

    #[test]
    fn structural_assumptions_create_explicit_pairwise_ranking_reversals() {
        let report = report();
        assert!(report.ranking_reversal_pair_count > 0);
        let pair = report
            .pairwise_rankings
            .iter()
            .find(|pair| {
                pair.action_a_id == "add-eol-test" && pair.action_b_id == "extend-buffer"
            })
            .expect("EOL and buffer comparison");
        assert!(pair.ranking_reversal);
        assert!(
            pair.models_preferring_a
                .contains(&"independent-equipment-faults".to_string())
        );
        assert!(
            pair.models_preferring_b
                .contains(&"common-cause-outages".to_string())
        );
    }

    #[test]
    fn row_permutations_change_source_digest_but_not_decisions() {
        let original = report();
        let mut value = fixture_value();
        value["actions"].as_array_mut().expect("actions").reverse();
        value["model_families"]
            .as_array_mut()
            .expect("families")
            .reverse();
        value["outcomes"].as_array_mut().expect("outcomes").reverse();
        let reordered_bytes = serde_json::to_vec(&value).expect("serialize reordered fixture");
        let reordered = parse_and_evaluate(&reordered_bytes).expect("reordered catalogue");
        assert_ne!(original.fixture_sha256, reordered.fixture_sha256);
        assert_eq!(
            original.reference_model_best_action_id,
            reordered.reference_model_best_action_id
        );
        assert_eq!(original.maximin_action_id, reordered.maximin_action_id);
        assert_eq!(
            original.maximin_minimum_gain_units_per_hour,
            reordered.maximin_minimum_gain_units_per_hour
        );
        assert_eq!(original.minimax_regret_action_id, reordered.minimax_regret_action_id);
        assert_eq!(
            original.minimax_regret_upper_bound_units_per_hour,
            reordered.minimax_regret_upper_bound_units_per_hour
        );
        assert_eq!(original.action_scores, reordered.action_scores);
        assert_eq!(original.family_winners, reordered.family_winners);
        assert_eq!(original.pairwise_rankings, reordered.pairwise_rankings);
        assert_eq!(original.leave_one_family_out, reordered.leave_one_family_out);
    }

    #[test]
    fn missing_and_duplicate_model_action_cells_fail_closed() {
        let mut missing = fixture_value();
        missing["outcomes"].as_array_mut().expect("outcomes").pop();
        let missing_bytes = serde_json::to_vec(&missing).expect("serialize missing cell");
        assert!(
            parse_and_evaluate(&missing_bytes)
                .expect_err("missing cell rejected")
                .contains("outcome_catalog_incomplete")
        );

        let mut duplicate = fixture_value();
        let first = duplicate["outcomes"][0].clone();
        let outcomes = duplicate["outcomes"].as_array_mut().expect("outcomes");
        outcomes.pop();
        outcomes.push(first);
        let duplicate_bytes = serde_json::to_vec(&duplicate).expect("serialize duplicate cell");
        assert!(
            parse_and_evaluate(&duplicate_bytes)
                .expect_err("duplicate cell rejected")
                .contains("duplicate_model_action_outcome")
        );
    }

    #[test]
    fn unknown_model_and_invalid_resource_values_fail_closed() {
        let mut unknown_model = fixture_value();
        unknown_model["outcomes"][0]["model_id"] = json!("unregistered-model");
        let bytes = serde_json::to_vec(&unknown_model).expect("serialize unknown model");
        assert!(
            parse_and_evaluate(&bytes)
                .expect_err("unknown model rejected")
                .contains("outcome_has_unknown_model_or_action_id")
        );

        let mut invalid_resource = fixture_value();
        invalid_resource["actions"][0]["capex_usd"] = json!(-1.0);
        let bytes = serde_json::to_vec(&invalid_resource).expect("serialize invalid resource");
        assert!(
            parse_and_evaluate(&bytes)
                .expect_err("negative capex rejected")
                .contains("action_resources_must_be_finite_and_nonnegative")
        );
    }

    #[test]
    fn infeasible_high_gain_action_cannot_win_or_change_regret_oracle() {
        let original = report();
        let mut value = fixture_value();
        value["actions"].as_array_mut().expect("actions").push(json!({
            "action_id": "over-budget-action",
            "label": "Synthetic over-budget decoy",
            "capex_usd": 51.0,
            "staffed_labor_hours_per_shift": 0.0,
            "active_power_kw": 0.0,
            "floor_area_sqm": 0.0
        }));
        let family_ids: Vec<Value> = value["model_families"]
            .as_array()
            .expect("families")
            .iter()
            .map(|family| family["model_id"].clone())
            .collect();
        for model_id in family_ids {
            value["outcomes"].as_array_mut().expect("outcomes").push(json!({
                "model_id": model_id,
                "action_id": "over-budget-action",
                "incremental_good_units_per_hour": 99.0
            }));
        }
        let bytes = serde_json::to_vec(&value).expect("serialize over-budget decoy");
        let report = parse_and_evaluate(&bytes).expect("complete decoy catalogue");
        assert_eq!(report.feasible_action_count, 5);
        assert_eq!(report.infeasible_action_count, 1);
        assert_eq!(report.maximin_action_id, original.maximin_action_id);
        assert_eq!(report.minimax_regret_action_id, original.minimax_regret_action_id);
        let decoy = report
            .action_scores
            .iter()
            .find(|score| score.action_id == "over-budget-action")
            .expect("decoy score");
        assert!(!decoy.resource_feasible);
        assert_eq!(decoy.minimum_gain_across_model_families_units_per_hour, None);
        assert_eq!(decoy.maximum_regret_across_model_families_units_per_hour, None);
    }

    #[test]
    fn leave_one_family_out_records_sensitivity_without_probabilities() {
        let report = report();
        assert_eq!(report.leave_one_family_out.len(), 6);
        assert!(
            report
                .leave_one_family_out
                .iter()
                .all(|row| row.remaining_model_family_count == 5)
        );
        assert!(
            report
                .leave_one_family_out
                .iter()
                .any(|row| row.maximin_action_id != report.maximin_action_id)
                || report
                    .leave_one_family_out
                    .iter()
                    .any(|row| row.minimax_regret_action_id != report.minimax_regret_action_id)
        );
    }
}
