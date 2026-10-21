use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const MAX_EXACT_OPTIONS: usize = 10;
const EPSILON: f64 = 1e-9;
const CLAIM_CEILING: &str = "Synthetic independent coefficient-interval decision benchmark only. It does not establish calibrated uncertainty bounds, field accuracy, industry SOTA, ROI, safety compliance, purchase approval or production release.";

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
    uncertainty_contract: String,
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
    uncertainty_half_width_units_per_hour: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct InteractionEffect {
    scenario_id: String,
    option_a_id: String,
    option_b_id: String,
    incremental_good_units_per_hour: f64,
    uncertainty_half_width_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioGain {
    scenario_id: String,
    nominal_incremental_good_units_per_hour: f64,
    interval_lower_bound_incremental_good_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Design {
    design_id: String,
    selected_option_ids: Vec<String>,
    scenario_gains: Vec<ScenarioGain>,
    nominal_worst_case_gain_units_per_hour: f64,
    interval_lower_bound_worst_case_gain_units_per_hour: f64,
    nominal_worst_case_regret_units_per_hour: f64,
    interval_robust_regret_upper_bound_units_per_hour: f64,
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
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_fixture_sha256: String,
    source_evidence_class: String,
    scenario_count: usize,
    option_count: usize,
    exact_design_space_size: u64,
    feasible_design_count: usize,
    infeasible_design_count: usize,
    nominal_scenario_robust_best_design_id: String,
    nominal_scenario_robust_best_gain_units_per_hour: f64,
    interval_robust_best_design_id: String,
    interval_robust_best_lower_bound_gain_units_per_hour: f64,
    minimax_regret_design_id: String,
    minimax_regret_upper_bound_units_per_hour: f64,
    nominal_choice_regret_upper_bound_units_per_hour: f64,
    nominal_choice_and_interval_robust_choice_differ: bool,
    nominal_choice_and_minimax_regret_choice_differ: bool,
    objective_definition: Vec<Objective>,
    pareto_frontier_design_ids: Vec<String>,
    designs: Vec<Design>,
    coefficient_intervals_are_independent: bool,
    scenario_probabilities_used: bool,
    expected_value_reported: bool,
    roi_computed: bool,
    all_inputs_are_synthetic: bool,
    field_sota_claim: bool,
    production_release_authorized: bool,
    uncertainty_contract: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Objective {
    field: String,
    direction: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("factory model-uncertainty benchmark rejected input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(
            "usage: ev_factory_model_uncertainty <synthetic-model-uncertainty-fixture.json>".into(),
        );
    }
    let input_bytes = fs::read(&args[1])
        .map_err(|error| format!("cannot read model-uncertainty fixture: {error}"))?;
    let fixture: Fixture = serde_json::from_slice(&input_bytes)
        .map_err(|error| format!("model-uncertainty fixture JSON/schema invalid: {error}"))?;
    validate_fixture(&fixture)?;
    let digest = format!("{:x}", Sha256::digest(&input_bytes));
    let report = evaluate(&fixture, digest)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize model-uncertainty report: {error}"))?
    );
    Ok(())
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "0.1.0"
        || fixture.benchmark_id != "EV-FACTORY-MODEL-UNCERTAINTY-001"
        || fixture.evidence_class != "synthetic"
        || fixture.currency.trim().is_empty()
        || fixture.uncertainty_contract.trim().is_empty()
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
    if limits.iter().any(|value| !value.is_finite() || *value < 0.0) {
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
            || !effect.uncertainty_half_width_units_per_hour.is_finite()
            || effect.uncertainty_half_width_units_per_hour < 0.0
            || !seen_effects.insert((effect.scenario_id.clone(), effect.option_id.clone()))
        {
            return Err("option_effect_unknown_duplicate_or_invalid_uncertainty".into());
        }
    }
    if seen_effects.len() != scenario_ids.len() * option_ids.len() {
        return Err("option_effect_catalog_must_cover_every_option_scenario_pair".into());
    }

    let mut expected_pairs = BTreeSet::new();
    let ids: Vec<String> = option_ids.iter().cloned().collect();
    for scenario_id in &scenario_ids {
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                expected_pairs.insert((scenario_id.clone(), ids[i].clone(), ids[j].clone()));
            }
        }
    }

    let mut seen_interactions = BTreeSet::new();
    for interaction in &fixture.interaction_effects {
        let key = (
            interaction.scenario_id.clone(),
            interaction.option_a_id.clone(),
            interaction.option_b_id.clone(),
        );
        if !option_ids.contains(&interaction.option_a_id)
            || !option_ids.contains(&interaction.option_b_id)
            || interaction.option_a_id >= interaction.option_b_id
            || !scenario_ids.contains(&interaction.scenario_id)
            || !interaction.incremental_good_units_per_hour.is_finite()
            || !interaction.uncertainty_half_width_units_per_hour.is_finite()
            || interaction.uncertainty_half_width_units_per_hour < 0.0
            || !seen_interactions.insert(key)
        {
            return Err("interaction_effect_unknown_noncanonical_duplicate_or_invalid_uncertainty".into());
        }
    }
    if seen_interactions != expected_pairs {
        return Err("interaction_effect_catalog_must_cover_every_scenario_pair_of_options".into());
    }
    Ok(())
}

fn evaluate(fixture: &Fixture, source_fixture_sha256: String) -> Result<Report, String> {
    validate_fixture(fixture)?;
    let mut options = fixture.options.clone();
    options.sort_by(|a, b| a.option_id.cmp(&b.option_id));
    let mut scenarios = fixture.scenarios.clone();
    scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    let mut option_effects = BTreeMap::<(String, String), (f64, f64)>::new();
    for effect in &fixture.option_effects {
        option_effects.insert(
            (effect.scenario_id.clone(), effect.option_id.clone()),
            (
                effect.incremental_good_units_per_hour,
                effect.uncertainty_half_width_units_per_hour,
            ),
        );
    }
    let mut interactions = BTreeMap::<(String, String, String), (f64, f64)>::new();
    for effect in &fixture.interaction_effects {
        interactions.insert(
            (
                effect.scenario_id.clone(),
                effect.option_a_id.clone(),
                effect.option_b_id.clone(),
            ),
            (
                effect.incremental_good_units_per_hour,
                effect.uncertainty_half_width_units_per_hour,
            ),
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

    assign_regret_bounds(&mut designs, &scenarios, &options, &option_effects, &interactions);
    mark_pareto_frontier(&mut designs);

    let mut feasible: Vec<&Design> = designs.iter().filter(|design| design.feasible).collect();
    let nominal_best = (**feasible
        .iter()
        .min_by(|a, b| nominal_preference_cmp(a, b))
        .ok_or_else(|| "no_nominal_feasible_design".to_string())?)
        .clone();
    let interval_best = (**feasible
        .iter()
        .min_by(|a, b| interval_preference_cmp(a, b))
        .ok_or_else(|| "no_interval_robust_feasible_design".to_string())?)
        .clone();
    let minimax_regret = (**feasible
        .iter()
        .min_by(|a, b| regret_preference_cmp(a, b))
        .ok_or_else(|| "no_minimax_regret_feasible_design".to_string())?)
        .clone();

    let mut objectives = vec![
        Objective { field: "interval_lower_bound_worst_case_gain_units_per_hour".into(), direction: "maximize".into() },
        Objective { field: "capex".into(), direction: "minimize".into() },
        Objective { field: "staffed_labor_hours_per_shift".into(), direction: "minimize".into() },
        Objective { field: "energy_kwh_per_shift".into(), direction: "minimize".into() },
        Objective { field: "emissions_kgco2e_per_shift".into(), direction: "minimize".into() },
        Objective { field: "floor_area_sqm".into(), direction: "minimize".into() },
    ];
    objectives.sort_by(|a, b| a.field.cmp(&b.field));

    let mut frontier_ids: Vec<String> = designs
        .iter()
        .filter(|design| design.feasible && design.pareto_frontier_member)
        .map(|design| design.design_id.clone())
        .collect();
    frontier_ids.sort();

    let nominal_regret = nominal_best.interval_robust_regret_upper_bound_units_per_hour;
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: fixture.benchmark_id.clone(),
        source_fixture_sha256,
        source_evidence_class: fixture.evidence_class.clone(),
        scenario_count: scenarios.len(),
        option_count: options.len(),
        exact_design_space_size: design_space_size,
        feasible_design_count: feasible_count,
        infeasible_design_count: designs.len() - feasible_count,
        nominal_scenario_robust_best_design_id: nominal_best.design_id.clone(),
        nominal_scenario_robust_best_gain_units_per_hour: nominal_best.nominal_worst_case_gain_units_per_hour,
        interval_robust_best_design_id: interval_best.design_id.clone(),
        interval_robust_best_lower_bound_gain_units_per_hour: interval_best.interval_lower_bound_worst_case_gain_units_per_hour,
        minimax_regret_design_id: minimax_regret.design_id.clone(),
        minimax_regret_upper_bound_units_per_hour: minimax_regret.interval_robust_regret_upper_bound_units_per_hour,
        nominal_choice_regret_upper_bound_units_per_hour: nominal_regret,
        nominal_choice_and_interval_robust_choice_differ: nominal_best.design_id != interval_best.design_id,
        nominal_choice_and_minimax_regret_choice_differ: nominal_best.design_id != minimax_regret.design_id,
        objective_definition: objectives,
        pareto_frontier_design_ids: frontier_ids,
        designs,
        coefficient_intervals_are_independent: true,
        scenario_probabilities_used: false,
        expected_value_reported: false,
        roi_computed: false,
        all_inputs_are_synthetic: true,
        field_sota_claim: false,
        production_release_authorized: false,
        uncertainty_contract: fixture.uncertainty_contract.clone(),
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn evaluate_design(
    fixture: &Fixture,
    options: &[DesignOption],
    scenarios: &[Scenario],
    option_effects: &BTreeMap<(String, String), (f64, f64)>,
    interactions: &BTreeMap<(String, String, String), (f64, f64)>,
    selected: &BTreeSet<String>,
) -> Result<Design, String> {
    let chosen: Vec<&DesignOption> = options
        .iter()
        .filter(|option| selected.contains(&option.option_id))
        .collect();
    let capex: f64 = chosen.iter().map(|option| option.capex).sum();
    let labor: f64 = chosen
        .iter()
        .map(|option| option.staffed_labor_hours_per_shift)
        .sum();
    let power: f64 = chosen.iter().map(|option| option.active_power_kw).sum();
    let energy: f64 = chosen.iter().map(|option| option.energy_kwh_per_shift).sum();
    let area: f64 = chosen.iter().map(|option| option.floor_area_sqm).sum();

    let mut scenario_gains = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let mut nominal = 0.0;
        let mut lower = 0.0;
        for option in &chosen {
            let (mean, half_width) = option_effects
                .get(&(scenario.scenario_id.clone(), option.option_id.clone()))
                .copied()
                .ok_or_else(|| {
                    format!(
                        "option_effect_missing_for_{}_{}",
                        scenario.scenario_id, option.option_id
                    )
                })?;
            nominal += mean;
            lower += mean - half_width;
        }
        for i in 0..chosen.len() {
            for j in (i + 1)..chosen.len() {
                let (a, b) = if chosen[i].option_id < chosen[j].option_id {
                    (&chosen[i].option_id, &chosen[j].option_id)
                } else {
                    (&chosen[j].option_id, &chosen[i].option_id)
                };
                let (mean, half_width) = interactions
                    .get(&(scenario.scenario_id.clone(), a.clone(), b.clone()))
                    .copied()
                    .ok_or_else(|| format!("interaction_effect_missing_for_{}_{}_{}", scenario.scenario_id, a, b))?;
                nominal += mean;
                lower += mean - half_width;
            }
        }
        if !nominal.is_finite() || !lower.is_finite() {
            return Err("computed_design_effect_nonfinite".into());
        }
        scenario_gains.push(ScenarioGain {
            scenario_id: scenario.scenario_id.clone(),
            nominal_incremental_good_units_per_hour: nominal,
            interval_lower_bound_incremental_good_units_per_hour: lower,
        });
    }
    let nominal_worst = scenario_gains
        .iter()
        .map(|row| row.nominal_incremental_good_units_per_hour)
        .fold(f64::INFINITY, f64::min);
    let interval_lower_worst = scenario_gains
        .iter()
        .map(|row| row.interval_lower_bound_incremental_good_units_per_hour)
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
        scenario_gains,
        nominal_worst_case_gain_units_per_hour: nominal_worst,
        interval_lower_bound_worst_case_gain_units_per_hour: interval_lower_worst,
        nominal_worst_case_regret_units_per_hour: 0.0,
        interval_robust_regret_upper_bound_units_per_hour: 0.0,
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

fn assign_regret_bounds(
    designs: &mut [Design],
    scenarios: &[Scenario],
    options: &[DesignOption],
    option_effects: &BTreeMap<(String, String), (f64, f64)>,
    interactions: &BTreeMap<(String, String, String), (f64, f64)>,
) {
    let feasible_indices: Vec<usize> = designs
        .iter()
        .enumerate()
        .filter_map(|(index, design)| design.feasible.then_some(index))
        .collect();
    let mut bounds = vec![(0.0, 0.0); designs.len()];

    for &candidate_index in &feasible_indices {
        let candidate: BTreeSet<String> = designs[candidate_index]
            .selected_option_ids
            .iter()
            .cloned()
            .collect();
        let mut nominal_regret: f64 = 0.0;
        let mut interval_regret: f64 = 0.0;
        for &challenger_index in &feasible_indices {
            let challenger: BTreeSet<String> = designs[challenger_index]
                .selected_option_ids
                .iter()
                .cloned()
                .collect();
            for scenario in scenarios {
                let mut nominal_gap = 0.0;
                let mut interval_gap = 0.0;
                for option in options {
                    let delta = (if challenger.contains(&option.option_id) { 1_i32 } else { 0_i32 })
                        - (if candidate.contains(&option.option_id) { 1_i32 } else { 0_i32 });
                    if delta == 0 {
                        continue;
                    }
                    let (mean, half_width) = option_effects
                        .get(&(scenario.scenario_id.clone(), option.option_id.clone()))
                        .copied()
                        .expect("validated complete option effects");
                    nominal_gap += f64::from(delta) * mean;
                    interval_gap += f64::from(delta) * mean + f64::from(delta.abs()) * half_width;
                }
                for i in 0..options.len() {
                    for j in (i + 1)..options.len() {
                        let option_a = &options[i].option_id;
                        let option_b = &options[j].option_id;
                        let challenger_has_pair = challenger.contains(option_a) && challenger.contains(option_b);
                        let candidate_has_pair = candidate.contains(option_a) && candidate.contains(option_b);
                        let delta = (if challenger_has_pair { 1_i32 } else { 0_i32 })
                            - (if candidate_has_pair { 1_i32 } else { 0_i32 });
                        if delta == 0 {
                            continue;
                        }
                        let (mean, half_width) = interactions
                            .get(&(scenario.scenario_id.clone(), option_a.clone(), option_b.clone()))
                            .copied()
                            .expect("validated complete interaction effects");
                        nominal_gap += f64::from(delta) * mean;
                        interval_gap += f64::from(delta) * mean + f64::from(delta.abs()) * half_width;
                    }
                }
                nominal_regret = nominal_regret.max(nominal_gap);
                interval_regret = interval_regret.max(interval_gap);
            }
        }
        bounds[candidate_index] = (nominal_regret.max(0.0), interval_regret.max(0.0));
    }

    for (design, (nominal, interval)) in designs.iter_mut().zip(bounds) {
        if design.feasible {
            design.nominal_worst_case_regret_units_per_hour = nominal;
            design.interval_robust_regret_upper_bound_units_per_hour = interval;
        }
    }
}

fn nominal_preference_cmp(a: &Design, b: &Design) -> Ordering {
    b.nominal_worst_case_gain_units_per_hour
        .total_cmp(&a.nominal_worst_case_gain_units_per_hour)
        .then_with(|| a.capex.total_cmp(&b.capex))
        .then_with(|| a.energy_kwh_per_shift.total_cmp(&b.energy_kwh_per_shift))
        .then_with(|| a.design_id.cmp(&b.design_id))
}

fn interval_preference_cmp(a: &Design, b: &Design) -> Ordering {
    b.interval_lower_bound_worst_case_gain_units_per_hour
        .total_cmp(&a.interval_lower_bound_worst_case_gain_units_per_hour)
        .then_with(|| a.capex.total_cmp(&b.capex))
        .then_with(|| a.energy_kwh_per_shift.total_cmp(&b.energy_kwh_per_shift))
        .then_with(|| a.design_id.cmp(&b.design_id))
}

fn regret_preference_cmp(a: &Design, b: &Design) -> Ordering {
    a.interval_robust_regret_upper_bound_units_per_hour
        .total_cmp(&b.interval_robust_regret_upper_bound_units_per_hour)
        .then_with(|| {
            b.interval_lower_bound_worst_case_gain_units_per_hour
                .total_cmp(&a.interval_lower_bound_worst_case_gain_units_per_hour)
        })
        .then_with(|| a.capex.total_cmp(&b.capex))
        .then_with(|| a.design_id.cmp(&b.design_id))
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
    let no_worse = a.interval_lower_bound_worst_case_gain_units_per_hour + EPSILON
        >= b.interval_lower_bound_worst_case_gain_units_per_hour
        && a.capex <= b.capex + EPSILON
        && a.staffed_labor_hours_per_shift <= b.staffed_labor_hours_per_shift + EPSILON
        && a.energy_kwh_per_shift <= b.energy_kwh_per_shift + EPSILON
        && a.emissions_kgco2e_per_shift <= b.emissions_kgco2e_per_shift + EPSILON
        && a.floor_area_sqm <= b.floor_area_sqm + EPSILON;
    let strictly_better = a.interval_lower_bound_worst_case_gain_units_per_hour
        > b.interval_lower_bound_worst_case_gain_units_per_hour + EPSILON
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
            "../../fixtures/ev_factory_model_uncertainty_v0_1.json"
        ))
        .expect("model-uncertainty fixture parses")
    }

    #[test]
    fn exact_oracle_enumerates_entire_design_space_deterministically() {
        let fixture = fixture();
        let first = evaluate(&fixture, "fixture-digest".into()).expect("first report");
        let second = evaluate(&fixture, "fixture-digest".into()).expect("second report");
        assert_eq!(first, second);
        assert_eq!(first.exact_design_space_size, 32);
        assert_eq!(first.designs.len(), 32);
        assert_eq!(first.feasible_design_count + first.infeasible_design_count, 32);
        assert!(!first.scenario_probabilities_used);
        assert!(!first.expected_value_reported);
        assert!(!first.roi_computed);
        assert!(!first.field_sota_claim);
        assert!(!first.production_release_authorized);
    }

    #[test]
    fn feasible_designs_obey_all_hard_constraints_and_frontier_excludes_infeasible() {
        let fixture = fixture();
        let report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        for design in report.designs.iter().filter(|design| design.feasible) {
            assert!(design.capex <= fixture.constraints.capex_budget + EPSILON);
            assert!(design.staffed_labor_hours_per_shift <= fixture.constraints.staffed_labor_hours_per_shift_limit + EPSILON);
            assert!(design.active_power_kw <= fixture.constraints.active_power_kw_limit + EPSILON);
            assert!(design.floor_area_sqm <= fixture.constraints.floor_area_sqm_limit + EPSILON);
            assert!(design.infeasibility_reasons.is_empty());
            assert!(design.interval_robust_regret_upper_bound_units_per_hour + EPSILON
                >= design.nominal_worst_case_regret_units_per_hour);
        }
        for design in report.designs.iter().filter(|design| !design.feasible) {
            assert!(!design.infeasibility_reasons.is_empty());
            assert!(!design.pareto_frontier_member);
        }
    }

    #[test]
    fn interval_robustness_and_minimax_regret_are_explicitly_compared() {
        let report = evaluate(&fixture(), "fixture-digest".into()).expect("report");
        let nominal = report
            .designs
            .iter()
            .find(|design| design.design_id == report.nominal_scenario_robust_best_design_id)
            .expect("nominal choice");
        let interval = report
            .designs
            .iter()
            .find(|design| design.design_id == report.interval_robust_best_design_id)
            .expect("interval choice");
        let minimax = report
            .designs
            .iter()
            .find(|design| design.design_id == report.minimax_regret_design_id)
            .expect("minimax regret choice");
        assert!(nominal.feasible && interval.feasible && minimax.feasible);
        assert_eq!(
            report.nominal_scenario_robust_best_design_id,
            "add-body-prep+add-eol-test+improve-yield"
        );
        assert_eq!(report.interval_robust_best_design_id, "extend-buffer");
        assert_eq!(
            report.minimax_regret_design_id,
            "add-body-prep+add-eol-test+improve-yield"
        );
        assert!((report.nominal_scenario_robust_best_gain_units_per_hour - 2.5).abs() < EPSILON);
        assert!(
            (interval.interval_lower_bound_worst_case_gain_units_per_hour - 0.18).abs() < EPSILON
        );
        assert!(
            (nominal.interval_lower_bound_worst_case_gain_units_per_hour + 0.11).abs() < EPSILON
        );
        assert!(
            (minimax.interval_robust_regret_upper_bound_units_per_hour - 6.41).abs() < EPSILON
        );
        assert!(report.nominal_choice_and_interval_robust_choice_differ);
        assert!(!report.nominal_choice_and_minimax_regret_choice_differ);
        assert!(interval.interval_lower_bound_worst_case_gain_units_per_hour
            >= nominal.interval_lower_bound_worst_case_gain_units_per_hour - EPSILON);
        assert!(minimax.interval_robust_regret_upper_bound_units_per_hour
            <= nominal.interval_robust_regret_upper_bound_units_per_hour + EPSILON);
    }

    #[test]
    fn pairwise_independent_interval_regret_matches_hand_calculation() {
        let fixture = fixture();
        let report = evaluate(&fixture, "fixture-digest".into()).expect("report");
        let candidate = report
            .designs
            .iter()
            .find(|design| design.design_id == "hold-capital")
            .expect("hold capital");
        let expected = report
            .designs
            .iter()
            .filter(|design| design.feasible)
            .flat_map(|design| design.scenario_gains.iter())
            .map(|gain| gain.interval_lower_bound_incremental_good_units_per_hour)
            .fold(0.0_f64, f64::max);
        assert!(candidate.interval_robust_regret_upper_bound_units_per_hour >= expected - EPSILON);
        assert!(
            (candidate.interval_robust_regret_upper_bound_units_per_hour - 8.88).abs() < EPSILON
        );
    }

    #[test]
    fn duplicate_main_effect_reverse_pair_and_invalid_uncertainty_fail_closed() {
        let mut duplicate = fixture();
        duplicate.option_effects.push(duplicate.option_effects[0].clone());
        assert!(validate_fixture(&duplicate).expect_err("duplicate main effect").contains("option_effect_unknown"));

        let mut reverse_pair = fixture();
        reverse_pair.interaction_effects.push(InteractionEffect {
            scenario_id: "body-prep-constrained".into(),
            option_a_id: "add-eol-test".into(),
            option_b_id: "add-body-prep".into(),
            incremental_good_units_per_hour: 0.8,
            uncertainty_half_width_units_per_hour: 0.1,
        });
        assert!(validate_fixture(&reverse_pair).expect_err("reverse pair").contains("interaction_effect_unknown"));

        let mut invalid_width = fixture();
        invalid_width.option_effects[0].uncertainty_half_width_units_per_hour = -0.1;
        assert!(validate_fixture(&invalid_width).expect_err("negative width").contains("invalid_uncertainty"));
    }

    #[test]
    fn incomplete_interaction_catalog_and_exact_search_cap_fail_closed() {
        let mut incomplete = fixture();
        incomplete.interaction_effects.pop();
        assert!(validate_fixture(&incomplete).expect_err("incomplete interactions").contains("must_cover_every_scenario_pair"));

        let mut oversized = fixture();
        oversized.options = (0..11).map(|index| DesignOption {
            option_id: format!("option-{index:02}"),
            label: "synthetic".into(),
            capex: 0.0,
            staffed_labor_hours_per_shift: 0.0,
            active_power_kw: 0.0,
            energy_kwh_per_shift: 0.0,
            floor_area_sqm: 0.0,
        }).collect();
        assert!(validate_fixture(&oversized).expect_err("size limit").contains("exact-enumeration"));
    }
}
