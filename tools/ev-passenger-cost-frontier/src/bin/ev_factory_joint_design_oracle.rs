use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const MAX_EXACT_OPTIONS: usize = 20;
const EPSILON: f64 = 1e-9;
const CLAIM_CEILING: &str = "Synthetic exact-enumeration oracle over a declared main-effect plus pairwise-interaction response model. No plant data, simulator calibration, field SOTA claim, ROI, purchase decision or production authorization.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    currency: String,
    grid_kgco2e_per_kwh: f64,
    constraints: DesignConstraints,
    options: Vec<DesignOption>,
    scenarios: Vec<Scenario>,
    option_effects: Vec<OptionEffect>,
    interaction_effects: Vec<InteractionEffect>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesignConstraints {
    capex_budget: f64,
    staffed_labor_hours_per_shift_limit: f64,
    active_power_kw_limit: f64,
    floor_area_sqm_limit: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesignOption {
    option_id: String,
    label: String,
    capex: f64,
    staffed_labor_hours_per_shift: f64,
    active_power_kw: f64,
    energy_kwh_per_shift: f64,
    floor_area_sqm: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionEffect {
    scenario_id: String,
    option_id: String,
    incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct InteractionEffect {
    scenario_id: String,
    option_a_id: String,
    option_b_id: String,
    incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Design {
    design_id: String,
    selected_option_ids: Vec<String>,
    incremental_good_units_per_hour_by_scenario: Vec<ScenarioGain>,
    worst_case_incremental_good_units_per_hour: f64,
    capex: f64,
    staffed_labor_hours_per_shift: f64,
    active_power_kw: f64,
    energy_kwh_per_shift: f64,
    emissions_kgco2e_per_shift: f64,
    floor_area_sqm: f64,
    feasible: bool,
    infeasibility_reasons: Vec<String>,
    pareto_frontier_member: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioGain {
    scenario_id: String,
    incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SearchComparison {
    method: String,
    selected_design_id: String,
    worst_case_incremental_good_units_per_hour: f64,
    regret_vs_exact_oracle_good_units_per_hour: f64,
    feasible: bool,
    candidate_extensions_evaluated: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_fixture_sha256: String,
    scenario_count: usize,
    option_count: usize,
    exact_design_space_size: u64,
    feasible_design_count: usize,
    infeasible_design_count: usize,
    exact_robust_best_design_id: String,
    exact_robust_best_worst_case_incremental_good_units_per_hour: f64,
    objective_definition: Vec<Objective>,
    search_comparisons: Vec<SearchComparison>,
    pareto_frontier_design_ids: Vec<String>,
    designs: Vec<Design>,
    all_inputs_are_synthetic: bool,
    scenario_probabilities_used: bool,
    expected_value_reported: bool,
    roi_computed: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Objective {
    field: String,
    direction: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("factory joint-design oracle rejected input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err("usage: ev_factory_joint_design_oracle <synthetic-design-fixture.json>".into());
    }
    let input_bytes =
        fs::read(&args[1]).map_err(|error| format!("cannot read joint-design fixture: {error}"))?;
    let fixture: Fixture = serde_json::from_slice(&input_bytes)
        .map_err(|error| format!("joint-design fixture JSON/schema invalid: {error}"))?;
    validate_fixture(&fixture)?;
    let digest = format!("{:x}", Sha256::digest(&input_bytes));
    let report = evaluate(&fixture, digest)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize joint-design report: {error}"))?
    );
    Ok(())
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "0.1.0"
        || fixture.benchmark_id != "EV-FACTORY-DESIGN-001"
        || fixture.evidence_class != "synthetic"
        || fixture.currency.trim().is_empty()
        || fixture.claim_ceiling.trim().is_empty()
        || !fixture.grid_kgco2e_per_kwh.is_finite()
        || fixture.grid_kgco2e_per_kwh < 0.0
        || fixture.scenarios.len() < 2
        || fixture.options.is_empty()
        || fixture.options.len() > MAX_EXACT_OPTIONS
    {
        return Err("fixture_failed_synthetic_schema_or_exact-enumeration_contract".into());
    }
    let limits = [
        fixture.constraints.capex_budget,
        fixture.constraints.staffed_labor_hours_per_shift_limit,
        fixture.constraints.active_power_kw_limit,
        fixture.constraints.floor_area_sqm_limit,
    ];
    if limits
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("design_constraints_must_be_finite_and_nonnegative".into());
    }

    let mut option_ids = BTreeSet::new();
    for option in &fixture.options {
        if option.option_id.trim().is_empty()
            || option.label.trim().is_empty()
            || !option_ids.insert(option.option_id.clone())
            || [
                option.capex,
                option.staffed_labor_hours_per_shift,
                option.active_power_kw,
                option.energy_kwh_per_shift,
                option.floor_area_sqm,
            ]
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(format!(
                "option_{}_identity_or_resource_inputs_invalid",
                option.option_id
            ));
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &fixture.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.clone())
        {
            return Err("scenario_ids_must_be_nonempty_and_unique".into());
        }
    }

    let mut seen_effects = BTreeSet::new();
    for effect in &fixture.option_effects {
        if !option_ids.contains(&effect.option_id)
            || !scenario_ids.contains(&effect.scenario_id)
            || !effect.incremental_good_units_per_hour.is_finite()
            || !seen_effects.insert((effect.scenario_id.clone(), effect.option_id.clone()))
        {
            return Err("option_effect_unknown_duplicate_or_nonfinite".into());
        }
    }
    if seen_effects.len() != scenario_ids.len() * option_ids.len() {
        return Err("option_effect_catalog_must_cover_every_option_scenario_pair".into());
    }

    let mut seen_interactions = BTreeSet::new();
    for interaction in &fixture.interaction_effects {
        if !option_ids.contains(&interaction.option_a_id)
            || !option_ids.contains(&interaction.option_b_id)
            || interaction.option_a_id >= interaction.option_b_id
            || !scenario_ids.contains(&interaction.scenario_id)
            || !interaction.incremental_good_units_per_hour.is_finite()
            || !seen_interactions.insert((
                interaction.scenario_id.clone(),
                interaction.option_a_id.clone(),
                interaction.option_b_id.clone(),
            ))
        {
            return Err("interaction_effect_unknown_noncanonical_duplicate_or_nonfinite".into());
        }
    }
    Ok(())
}

fn evaluate(fixture: &Fixture, source_fixture_sha256: String) -> Result<Report, String> {
    validate_fixture(fixture)?;
    let mut options = fixture.options.clone();
    options.sort_by(|a, b| a.option_id.cmp(&b.option_id));
    let mut scenarios = fixture.scenarios.clone();
    scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    let mut option_effects = BTreeMap::<(String, String), f64>::new();
    for effect in &fixture.option_effects {
        option_effects.insert(
            (effect.scenario_id.clone(), effect.option_id.clone()),
            effect.incremental_good_units_per_hour,
        );
    }
    let mut interactions = BTreeMap::<(String, String, String), f64>::new();
    for interaction in &fixture.interaction_effects {
        interactions.insert(
            (
                interaction.scenario_id.clone(),
                interaction.option_a_id.clone(),
                interaction.option_b_id.clone(),
            ),
            interaction.incremental_good_units_per_hour,
        );
    }

    let design_space_size = 1u64
        .checked_shl(options.len() as u32)
        .ok_or_else(|| "exact_design_space_size_overflow".to_string())?;
    let mut designs = Vec::with_capacity(design_space_size as usize);
    for mask in 0..design_space_size {
        let selected: BTreeSet<String> = options
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1u64 << index) != 0)
            .map(|(_, option)| option.option_id.clone())
            .collect();
        designs.push(evaluate_design(
            fixture,
            &options,
            &scenarios,
            &option_effects,
            &interactions,
            &selected,
        )?);
    }
    designs.sort_by(|a, b| a.design_id.cmp(&b.design_id));

    let feasible_count = designs.iter().filter(|design| design.feasible).count();
    if feasible_count == 0 {
        return Err("no_feasible_design_including_hold_capital".into());
    }
    mark_pareto_frontier(&mut designs);
    let mut feasible: Vec<&Design> = designs.iter().filter(|design| design.feasible).collect();
    feasible.sort_by(|a, b| preference_cmp(a, b));
    let exact_best = *feasible
        .first()
        .ok_or_else(|| "exact_oracle_has_no_feasible_design".to_string())?;

    let no_change = designs
        .iter()
        .find(|design| design.selected_option_ids.is_empty())
        .ok_or_else(|| "hold_capital_design_missing".to_string())?;
    let best_single = designs
        .iter()
        .filter(|design| design.feasible && design.selected_option_ids.len() == 1)
        .min_by(|a, b| preference_cmp(a, b))
        .unwrap_or(no_change);
    let (greedy_design, greedy_extensions) = greedy_search(
        fixture,
        &options,
        &scenarios,
        &option_effects,
        &interactions,
    )?;

    let search_comparisons = vec![
        comparison("no-change", no_change, exact_best, 0),
        comparison("best-single-intervention", best_single, exact_best, 0),
        comparison(
            "greedy-marginal-worst-case-gain",
            &greedy_design,
            exact_best,
            greedy_extensions,
        ),
    ];

    let mut objectives = vec![
        Objective {
            field: "worst_case_incremental_good_units_per_hour".into(),
            direction: "maximize".into(),
        },
        Objective {
            field: "capex".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "staffed_labor_hours_per_shift".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "energy_kwh_per_shift".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "emissions_kgco2e_per_shift".into(),
            direction: "minimize".into(),
        },
        Objective {
            field: "floor_area_sqm".into(),
            direction: "minimize".into(),
        },
    ];
    objectives.sort_by(|a, b| a.field.cmp(&b.field));

    let mut frontier_ids: Vec<String> = designs
        .iter()
        .filter(|design| design.feasible && design.pareto_frontier_member)
        .map(|design| design.design_id.clone())
        .collect();
    frontier_ids.sort();

    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: fixture.benchmark_id.clone(),
        source_fixture_sha256,
        scenario_count: scenarios.len(),
        option_count: options.len(),
        exact_design_space_size: design_space_size,
        feasible_design_count: feasible_count,
        infeasible_design_count: designs.len() - feasible_count,
        exact_robust_best_design_id: exact_best.design_id.clone(),
        exact_robust_best_worst_case_incremental_good_units_per_hour: exact_best
            .worst_case_incremental_good_units_per_hour,
        objective_definition: objectives,
        search_comparisons,
        pareto_frontier_design_ids: frontier_ids,
        designs,
        all_inputs_are_synthetic: true,
        scenario_probabilities_used: false,
        expected_value_reported: false,
        roi_computed: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn evaluate_design(
    fixture: &Fixture,
    options: &[DesignOption],
    scenarios: &[Scenario],
    option_effects: &BTreeMap<(String, String), f64>,
    interactions: &BTreeMap<(String, String, String), f64>,
    selected: &BTreeSet<String>,
) -> Result<Design, String> {
    let chosen: Vec<&DesignOption> = options
        .iter()
        .filter(|option| selected.contains(&option.option_id))
        .collect();
    let mut capex = 0.0;
    let mut labor = 0.0;
    let mut power = 0.0;
    let mut energy = 0.0;
    let mut area = 0.0;
    for option in &chosen {
        capex += option.capex;
        labor += option.staffed_labor_hours_per_shift;
        power += option.active_power_kw;
        energy += option.energy_kwh_per_shift;
        area += option.floor_area_sqm;
    }

    let mut scenario_gains = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let mut gain = 0.0;
        for option in &chosen {
            gain += option_effects
                .get(&(scenario.scenario_id.clone(), option.option_id.clone()))
                .copied()
                .ok_or_else(|| {
                    format!(
                        "option_effect_missing_for_{}_{}",
                        scenario.scenario_id, option.option_id
                    )
                })?;
        }
        for i in 0..chosen.len() {
            for j in (i + 1)..chosen.len() {
                let (a, b) = if chosen[i].option_id < chosen[j].option_id {
                    (&chosen[i].option_id, &chosen[j].option_id)
                } else {
                    (&chosen[j].option_id, &chosen[i].option_id)
                };
                gain += interactions
                    .get(&(scenario.scenario_id.clone(), a.clone(), b.clone()))
                    .copied()
                    .unwrap_or(0.0);
            }
        }
        if !gain.is_finite() {
            return Err("computed_design_effect_nonfinite".into());
        }
        scenario_gains.push(ScenarioGain {
            scenario_id: scenario.scenario_id.clone(),
            incremental_good_units_per_hour: gain,
        });
    }
    let worst_case_gain = scenario_gains
        .iter()
        .map(|row| row.incremental_good_units_per_hour)
        .fold(f64::INFINITY, f64::min);
    let mut reasons = Vec::new();
    if capex > fixture.constraints.capex_budget + EPSILON {
        reasons.push("capex_budget_exceeded".into());
    }
    if labor > fixture.constraints.staffed_labor_hours_per_shift_limit + EPSILON {
        reasons.push("staffing_limit_exceeded".into());
    }
    if power > fixture.constraints.active_power_kw_limit + EPSILON {
        reasons.push("active_power_limit_exceeded".into());
    }
    if area > fixture.constraints.floor_area_sqm_limit + EPSILON {
        reasons.push("floor_area_limit_exceeded".into());
    }

    let design_id = if selected.is_empty() {
        "hold-capital".into()
    } else {
        selected.iter().cloned().collect::<Vec<_>>().join("+")
    };
    Ok(Design {
        design_id,
        selected_option_ids: selected.iter().cloned().collect(),
        incremental_good_units_per_hour_by_scenario: scenario_gains,
        worst_case_incremental_good_units_per_hour: worst_case_gain,
        capex,
        staffed_labor_hours_per_shift: labor,
        active_power_kw: power,
        energy_kwh_per_shift: energy,
        emissions_kgco2e_per_shift: energy * fixture.grid_kgco2e_per_kwh,
        floor_area_sqm: area,
        feasible: reasons.is_empty(),
        infeasibility_reasons: reasons,
        pareto_frontier_member: false,
    })
}

fn preference_cmp(a: &Design, b: &Design) -> Ordering {
    b.worst_case_incremental_good_units_per_hour
        .total_cmp(&a.worst_case_incremental_good_units_per_hour)
        .then_with(|| a.capex.total_cmp(&b.capex))
        .then_with(|| a.energy_kwh_per_shift.total_cmp(&b.energy_kwh_per_shift))
        .then_with(|| {
            a.staffed_labor_hours_per_shift
                .total_cmp(&b.staffed_labor_hours_per_shift)
        })
        .then_with(|| a.design_id.cmp(&b.design_id))
}

fn comparison(
    method: &str,
    selected: &Design,
    exact_best: &Design,
    evaluated: usize,
) -> SearchComparison {
    SearchComparison {
        method: method.into(),
        selected_design_id: selected.design_id.clone(),
        worst_case_incremental_good_units_per_hour: selected
            .worst_case_incremental_good_units_per_hour,
        regret_vs_exact_oracle_good_units_per_hour: (exact_best
            .worst_case_incremental_good_units_per_hour
            - selected.worst_case_incremental_good_units_per_hour)
            .max(0.0),
        feasible: selected.feasible,
        candidate_extensions_evaluated: evaluated,
    }
}

fn greedy_search(
    fixture: &Fixture,
    options: &[DesignOption],
    scenarios: &[Scenario],
    option_effects: &BTreeMap<(String, String), f64>,
    interactions: &BTreeMap<(String, String, String), f64>,
) -> Result<(Design, usize), String> {
    let mut selected = BTreeSet::<String>::new();
    let mut current = evaluate_design(
        fixture,
        options,
        scenarios,
        option_effects,
        interactions,
        &selected,
    )?;
    let mut extensions_evaluated = 0usize;
    loop {
        let mut best_extension: Option<Design> = None;
        for option in options
            .iter()
            .filter(|option| !selected.contains(&option.option_id))
        {
            let mut candidate_set = selected.clone();
            candidate_set.insert(option.option_id.clone());
            let candidate = evaluate_design(
                fixture,
                options,
                scenarios,
                option_effects,
                interactions,
                &candidate_set,
            )?;
            extensions_evaluated += 1;
            if !candidate.feasible
                || candidate.worst_case_incremental_good_units_per_hour
                    <= current.worst_case_incremental_good_units_per_hour + EPSILON
            {
                continue;
            }
            if best_extension
                .as_ref()
                .is_none_or(|best| preference_cmp(&candidate, best) == Ordering::Less)
            {
                best_extension = Some(candidate);
            }
        }
        let Some(next) = best_extension else {
            break;
        };
        selected = next.selected_option_ids.iter().cloned().collect();
        current = next;
        if selected.len() == options.len() {
            break;
        }
    }
    Ok((current, extensions_evaluated))
}

fn mark_pareto_frontier(designs: &mut [Design]) {
    let feasible_indices: Vec<usize> = designs
        .iter()
        .enumerate()
        .filter_map(|(index, design)| design.feasible.then_some(index))
        .collect();
    for &index in &feasible_indices {
        let dominated = feasible_indices
            .iter()
            .copied()
            .any(|other| other != index && dominates(&designs[other], &designs[index]));
        designs[index].pareto_frontier_member = !dominated;
    }
}

fn dominates(a: &Design, b: &Design) -> bool {
    let no_worse = a.worst_case_incremental_good_units_per_hour + EPSILON
        >= b.worst_case_incremental_good_units_per_hour
        && a.capex <= b.capex + EPSILON
        && a.staffed_labor_hours_per_shift <= b.staffed_labor_hours_per_shift + EPSILON
        && a.energy_kwh_per_shift <= b.energy_kwh_per_shift + EPSILON
        && a.emissions_kgco2e_per_shift <= b.emissions_kgco2e_per_shift + EPSILON
        && a.floor_area_sqm <= b.floor_area_sqm + EPSILON;
    let strictly_better = a.worst_case_incremental_good_units_per_hour
        > b.worst_case_incremental_good_units_per_hour + EPSILON
        || a.capex + EPSILON < b.capex
        || a.staffed_labor_hours_per_shift + EPSILON < b.staffed_labor_hours_per_shift
        || a.energy_kwh_per_shift + EPSILON < b.energy_kwh_per_shift
        || a.emissions_kgco2e_per_shift + EPSILON < b.emissions_kgco2e_per_shift
        || a.floor_area_sqm + EPSILON < b.floor_area_sqm;
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../fixtures/ev_factory_joint_design_v0_1.json"
        ))
        .expect("joint-design fixture parses")
    }

    #[test]
    fn exact_oracle_enumerates_every_binary_design_and_is_deterministic() {
        let fixture = fixture();
        let digest = "fixture-digest".to_string();
        let first = evaluate(&fixture, digest.clone()).expect("first report");
        let second = evaluate(&fixture, digest).expect("second report");
        assert_eq!(first, second);
        assert_eq!(first.exact_design_space_size, 32);
        assert_eq!(first.designs.len(), 32);
        assert_eq!(
            first.feasible_design_count + first.infeasible_design_count,
            32
        );
        let ids: BTreeSet<String> = first
            .designs
            .iter()
            .map(|design| design.design_id.clone())
            .collect();
        assert_eq!(ids.len(), 32);
        assert!(!first.scenario_probabilities_used);
        assert!(!first.expected_value_reported);
        assert!(!first.roi_computed);
        assert!(!first.production_release_authorized);
    }

    #[test]
    fn every_design_reported_feasible_obeys_all_hard_constraints() {
        let fixture = fixture();
        let report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        for design in report.designs.iter().filter(|design| design.feasible) {
            assert!(design.capex <= fixture.constraints.capex_budget + EPSILON);
            assert!(
                design.staffed_labor_hours_per_shift
                    <= fixture.constraints.staffed_labor_hours_per_shift_limit + EPSILON
            );
            assert!(design.active_power_kw <= fixture.constraints.active_power_kw_limit + EPSILON);
            assert!(design.floor_area_sqm <= fixture.constraints.floor_area_sqm_limit + EPSILON);
            assert!(design.infeasibility_reasons.is_empty());
        }
        for design in report.designs.iter().filter(|design| !design.feasible) {
            assert!(!design.infeasibility_reasons.is_empty());
            assert!(!design.pareto_frontier_member);
        }
    }

    #[test]
    fn exact_oracle_beats_greedy_and_best_single_on_synthetic_interaction_fixture() {
        let report = evaluate(&fixture(), "fixture-digest".into()).expect("report");
        let exact = report.exact_robust_best_worst_case_incremental_good_units_per_hour;
        assert!(exact > 0.0);
        assert_eq!(
            report.exact_robust_best_design_id,
            "add-body-prep+add-eol-test+improve-yield"
        );
        let best_single = report
            .search_comparisons
            .iter()
            .find(|row| row.method == "best-single-intervention")
            .expect("single baseline");
        let greedy = report
            .search_comparisons
            .iter()
            .find(|row| row.method == "greedy-marginal-worst-case-gain")
            .expect("greedy baseline");
        assert!(best_single.regret_vs_exact_oracle_good_units_per_hour > 0.0);
        assert!(greedy.regret_vs_exact_oracle_good_units_per_hour > 0.0);
        assert!(
            greedy.worst_case_incremental_good_units_per_hour
                >= best_single.worst_case_incremental_good_units_per_hour
        );
    }

    #[test]
    fn pairwise_interaction_changes_combined_effect_without_claiming_independence() {
        let fixture = fixture();
        let options = fixture.options.clone();
        let mut lookup = BTreeMap::new();
        for effect in &fixture.option_effects {
            lookup.insert(
                (effect.scenario_id.clone(), effect.option_id.clone()),
                effect.incremental_good_units_per_hour,
            );
        }
        let mut interactions = BTreeMap::new();
        for row in &fixture.interaction_effects {
            interactions.insert(
                (
                    row.scenario_id.clone(),
                    row.option_a_id.clone(),
                    row.option_b_id.clone(),
                ),
                row.incremental_good_units_per_hour,
            );
        }
        let selected = BTreeSet::from(["add-body-prep".to_string(), "add-eol-test".to_string()]);
        let design = evaluate_design(
            &fixture,
            &options,
            &fixture.scenarios,
            &lookup,
            &interactions,
            &selected,
        )
        .expect("combined design");
        let body = lookup[&("body-prep-constrained".into(), "add-body-prep".into())];
        let eol = lookup[&("body-prep-constrained".into(), "add-eol-test".into())];
        assert!(
            (design
                .incremental_good_units_per_hour_by_scenario
                .iter()
                .find(|row| row.scenario_id == "body-prep-constrained")
                .unwrap()
                .incremental_good_units_per_hour
                - (body + eol + 0.8))
                .abs()
                < EPSILON
        );
    }

    #[test]
    fn duplicate_main_effect_and_noncanonical_interaction_fail_closed() {
        let mut duplicate_effect_fixture = fixture();
        duplicate_effect_fixture
            .option_effects
            .push(duplicate_effect_fixture.option_effects[0].clone());
        assert!(
            validate_fixture(&duplicate_effect_fixture)
                .expect_err("duplicate main effect rejected")
                .contains("option_effect_unknown")
        );
        let mut noncanonical_pair_fixture = fixture();
        noncanonical_pair_fixture.interaction_effects.push(InteractionEffect {
            scenario_id: "body-prep-constrained".into(),
            option_a_id: "add-eol-test".into(),
            option_b_id: "add-body-prep".into(),
            incremental_good_units_per_hour: 0.5,
        });
        assert!(
            validate_fixture(&noncanonical_pair_fixture)
                .expect_err("noncanonical pair rejected")
                .contains("interaction_effect_unknown")
        );
    }

    #[test]
    fn exact_search_size_limit_and_invalid_resources_fail_closed() {
        let mut oversized_fixture = fixture();
        oversized_fixture.options = (0..21)
            .map(|index| DesignOption {
                option_id: format!("option-{index:02}"),
                label: "synthetic".into(),
                capex: 1.0,
                staffed_labor_hours_per_shift: 0.0,
                active_power_kw: 0.0,
                energy_kwh_per_shift: 0.0,
                floor_area_sqm: 0.0,
            })
            .collect();
        assert!(
            validate_fixture(&oversized_fixture)
                .expect_err("exact-search cap enforced")
                .contains("exact-enumeration")
        );
        let mut invalid_resource_fixture = fixture();
        invalid_resource_fixture.options[0].capex = -1.0;
        assert!(
            validate_fixture(&invalid_resource_fixture)
                .expect_err("negative resource rejected")
                .contains("resource_inputs_invalid")
        );
    }
}
