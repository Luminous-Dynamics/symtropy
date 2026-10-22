use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    env, fs, process,
};

const BENCHMARK_ID: &str = "EV-FACTORY-ROBUST-001";
const MIN_FAMILIES: usize = 5;
const PRACTICAL_MARGIN: f64 = 0.05;
const CLAIM_CEILING: &str = "Scenario-family robustness analysis over synthetic factory DES reports only. It does not quantify real-world model uncertainty, validate site performance, establish SOTA, or authorize capital spending, safety or production release.";

#[derive(Debug, Clone)]
struct FamilyInput {
    family_id: String,
    sha256: String,
    replication_seeds: Vec<u64>,
    baseline_mean_good_units_per_hour: f64,
    gains_by_station: BTreeMap<String, BTreeMap<u64, f64>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct DistributionSummary {
    sample_count: usize,
    mean: f64,
    sample_standard_deviation: Option<f64>,
    minimum: f64,
    p05: f64,
    median: f64,
    p95: f64,
    maximum: f64,
    mean_confidence_95_lower: Option<f64>,
    mean_confidence_95_upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FamilyBaselineResult {
    family_id: String,
    source_report_sha256: String,
    baseline_mean_good_units_per_hour: f64,
    replication_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FamilyStationResult {
    family_id: String,
    source_report_sha256: String,
    intervention_station_id: String,
    mean_paired_gain_units_per_hour: f64,
    paired_gain: DistributionSummary,
    positive_gain_clear_of_margin: bool,
    family_oracle_best_station_id: String,
    regret_vs_family_oracle_units_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FamilyPairComparison {
    family_id: String,
    station_a_id: String,
    station_b_id: String,
    mean_paired_effect_a_minus_b_units_per_hour: f64,
    paired_effect_a_minus_b: DistributionSummary,
    empirical_fraction_a_beats_b_beyond_margin: f64,
    empirical_fraction_b_beats_a_beyond_margin: f64,
    practical_equivalence_margin_units_per_hour: f64,
    decision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct RobustStationScore {
    station_id: String,
    mean_gain_across_families_units_per_hour: f64,
    worst_family_mean_gain_units_per_hour: f64,
    worst_family_gain_ci_lower_units_per_hour: Option<f64>,
    families_with_gain_ci_lower_above_margin: usize,
    family_count: usize,
    mean_regret_vs_family_oracle_units_per_hour: f64,
    maximum_regret_vs_family_oracle_units_per_hour: f64,
    pareto_nondominated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    family_count: usize,
    family_ids: Vec<String>,
    shared_replication_count: usize,
    shared_replication_seeds: Vec<u64>,
    practical_equivalence_margin_units_per_hour: f64,
    input_family_digests: BTreeMap<String, String>,
    family_baseline_results: Vec<FamilyBaselineResult>,
    family_station_results: Vec<FamilyStationResult>,
    family_pair_comparisons: Vec<FamilyPairComparison>,
    station_scores: Vec<RobustStationScore>,
    pareto_frontier_station_ids: Vec<String>,
    robust_best_station_id: Option<String>,
    recommendation_status: String,
    family_aggregation_note: String,
    results_are_synthetic_model_conditional: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < MIN_FAMILIES {
        eprintln!(
            "usage: ev_factory_robustness_frontier family-id=report.json [family-id=report.json ...] (at least {MIN_FAMILIES} families)"
        );
        process::exit(2);
    }
    let mut inputs = Vec::with_capacity(args.len());
    for argument in &args {
        let Some((family_id, path)) = argument.split_once('=') else {
            eprintln!("invalid family argument {argument:?}; expected family-id=report.json");
            process::exit(2);
        };
        if family_id.trim().is_empty() || path.trim().is_empty() {
            eprintln!("empty family ID or report path in {argument:?}");
            process::exit(2);
        }
        match read_family(family_id, path) {
            Ok(family) => inputs.push(family),
            Err(error) => {
                eprintln!("family {family_id} rejected: {error}");
                process::exit(2);
            }
        }
    }
    let report = match evaluate(inputs) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("robustness frontier rejected input: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize robustness report: {error}");
            process::exit(2);
        }
    }
}

fn read_family(family_id: &str, path: &str) -> Result<FamilyInput, String> {
    let raw = fs::read_to_string(path).map_err(|error| format!("cannot_read_report: {error}"))?;
    let sha256 = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let report: Value =
        serde_json::from_str(&raw).map_err(|error| format!("invalid_json: {error}"))?;
    if field_str(&report, "benchmark_id")? != "EV-FACTORY-SIM-001"
        || field_str(&report, "schema_version")? != "0.1.0"
        || field_str(&report, "input_evidence_class")? != "synthetic"
    {
        return Err("unsupported_benchmark_schema_or_non_synthetic_evidence".into());
    }
    if field_bool(&report, "release_ready")?
        || !field_bool(&report, "output_never_exceeds_released_input")?
        || !field_bool(&report, "all_buffers_respected_capacity")?
        || !field_bool(&report, "station_time_accounting_within_horizon")?
    {
        return Err("simulation_invariants_failed_or_release_ready_claimed".into());
    }
    let replication_count = field_usize(&report, "replication_count")?;
    let seeds = report
        .get("replication_seeds")
        .and_then(Value::as_array)
        .ok_or_else(|| "replication_seeds_missing_or_not_array".to_string())?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| "replication_seed_not_unsigned_integer".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if replication_count < 2
        || seeds.len() != replication_count
        || seeds.iter().any(|seed| *seed == 0)
    {
        return Err("replication_count_and_positive_seed_list_must_match".into());
    }
    let unique_seeds: BTreeSet<u64> = seeds.iter().copied().collect();
    if unique_seeds.len() != seeds.len() {
        return Err("replication_seeds_must_be_unique".into());
    }

    let scenarios = report
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| "scenarios_missing_or_not_array".to_string())?;
    let mut scenario_to_station = BTreeMap::<String, Option<String>>::new();
    let mut station_ids = BTreeSet::new();
    let mut baseline_row_count = 0_usize;
    for scenario in scenarios {
        let scenario_id = field_str(scenario, "scenario_id")?.to_string();
        let station = scenario
            .get("intervention_station_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        if scenario_to_station
            .insert(scenario_id.clone(), station.clone())
            .is_some()
        {
            return Err("scenario_ids_must_be_unique".into());
        }
        if scenario_id == "baseline" {
            baseline_row_count += 1;
            if station.is_some() {
                return Err("baseline_scenario_must_not_declare_intervention_station".into());
            }
            let station_results = scenario
                .get("stations")
                .and_then(Value::as_array)
                .ok_or_else(|| "baseline_stations_missing_or_not_array".to_string())?;
            for station_result in station_results {
                let id = field_str(station_result, "station_id")?.to_string();
                if !station_ids.insert(id) {
                    return Err("baseline_station_ids_must_be_unique".into());
                }
            }
        } else {
            let station_id =
                station.ok_or_else(|| "intervention_scenario_missing_station_id".to_string())?;
            if !station_ids.contains(&station_id) && !station_ids.is_empty() {
                // Scenario traversal may list the baseline first; if not, it is rejected below.
                return Err("intervention_references_unknown_station".into());
            }
        }
    }
    if baseline_row_count != 1
        || station_ids.len() < 2
        || scenario_to_station.len() != station_ids.len() + 1
    {
        return Err(
            "scenario_catalog_must_contain_one_baseline_and_one_intervention_per_station".into(),
        );
    }
    let mut intervention_ids = BTreeSet::new();
    for (scenario_id, station) in &scenario_to_station {
        if scenario_id == "baseline" {
            continue;
        }
        let station_id = station
            .as_ref()
            .ok_or_else(|| "intervention_station_missing".to_string())?;
        if !station_ids.contains(station_id) || !intervention_ids.insert(station_id.clone()) {
            return Err("intervention_station_ids_must_uniquely_match_baseline".into());
        }
        if scenario_id != &format!("add-one-server-{station_id}") {
            return Err("unexpected_intervention_scenario_id".into());
        }
    }
    if intervention_ids != station_ids {
        return Err("every_baseline_station_requires_one_add-server_counterfactual".into());
    }

    let outcomes = report
        .get("replication_outcomes")
        .and_then(Value::as_array)
        .ok_or_else(|| "replication_outcomes_missing_or_not_array".to_string())?;
    if outcomes.len() != replication_count * scenario_to_station.len() {
        return Err("per_seed_outcome_count_must_equal_seed_count_times_scenarios".into());
    }
    let mut seen = HashSet::<(u64, String)>::new();
    let mut gains_by_station: BTreeMap<String, BTreeMap<u64, f64>> = station_ids
        .iter()
        .map(|station| (station.clone(), BTreeMap::new()))
        .collect();
    let mut baseline_rates = BTreeMap::<u64, f64>::new();
    for outcome in outcomes {
        let seed = field_u64(outcome, "seed")?;
        let scenario_id = field_str(outcome, "scenario_id")?.to_string();
        if !unique_seeds.contains(&seed)
            || !scenario_to_station.contains_key(&scenario_id)
            || !seen.insert((seed, scenario_id.clone()))
        {
            return Err("outcome_has_unknown_seed_scenario_or_duplicate_seed_scenario_pair".into());
        }
        let output = field_f64(outcome, "good_units_per_hour")?;
        let stored_baseline = field_f64(outcome, "baseline_good_units_per_hour")?;
        if output < 0.0 || stored_baseline < 0.0 {
            return Err("throughput_rate_must_be_nonnegative".into());
        }
        if scenario_id == "baseline" {
            if (output - stored_baseline).abs() > 1e-9
                || !outcome
                    .get("intervention_station_id")
                    .is_none_or(Value::is_null)
                || !outcome
                    .get("paired_good_units_per_hour_delta_vs_baseline")
                    .is_none_or(Value::is_null)
            {
                return Err("baseline_outcome_shape_or_value_invalid".into());
            }
            baseline_rates.insert(seed, output);
        } else {
            let station_id = scenario_to_station[&scenario_id]
                .as_ref()
                .expect("validated station");
            let delta = field_f64(outcome, "paired_good_units_per_hour_delta_vs_baseline")?;
            let expected_baseline = baseline_rates.get(&seed).copied().ok_or_else(|| {
                "baseline_outcome_must_be_emitted_before_interventions_for_each_seed".to_string()
            })?;
            if (stored_baseline - expected_baseline).abs() > 1e-9 {
                return Err("seed_outcome_baseline_rate_mismatch".into());
            }
            if (delta - (output - stored_baseline)).abs() > 1e-9 {
                return Err("stored_paired_delta_does_not_match_output_minus_baseline".into());
            }
            if field_str(outcome, "intervention_station_id")? != station_id {
                return Err("outcome_intervention_station_mismatch".into());
            }
            gains_by_station
                .get_mut(station_id)
                .expect("station map has candidate")
                .insert(seed, delta);
        }
    }
    if seen.len() != replication_count * scenario_to_station.len()
        || baseline_rates.len() != replication_count
        || gains_by_station
            .values()
            .any(|rows| rows.len() != replication_count)
    {
        return Err("missing_per_seed_baseline_or_intervention_outcome".into());
    }

    // Cross-check aggregate statistics against source per-seed outcomes instead of trusting both independently.
    let statistics = report
        .get("scenario_statistics")
        .and_then(Value::as_array)
        .ok_or_else(|| "scenario_statistics_missing_or_not_array".to_string())?;
    if statistics.len() != scenario_to_station.len() {
        return Err("scenario_statistics_count_mismatch".into());
    }
    let mut summary_stations = BTreeSet::new();
    for statistic in statistics {
        let scenario_id = field_str(statistic, "scenario_id")?;
        if scenario_id == "baseline" {
            let summary = statistic
                .get("good_units_per_hour")
                .ok_or_else(|| "baseline_rate_summary_missing".to_string())?;
            if field_usize(summary, "sample_count")? != replication_count {
                return Err("baseline_summary_seed_count_mismatch".into());
            }
            continue;
        }
        let station_id = field_str(statistic, "intervention_station_id")?;
        if scenario_id != format!("add-one-server-{station_id}") {
            return Err("scenario_summary_id_station_mismatch".into());
        }
        if !summary_stations.insert(station_id.to_string()) {
            return Err("duplicate_intervention_summary".into());
        }
        let stored = statistic
            .get("paired_good_units_per_hour_delta_vs_baseline")
            .ok_or_else(|| "summary_paired_gain_missing".to_string())?;
        if field_usize(stored, "sample_count")? != replication_count {
            return Err("paired_summary_seed_count_mismatch".into());
        }
        let values: Vec<f64> = seeds
            .iter()
            .map(|seed| gains_by_station[station_id][seed])
            .collect();
        let computed = summarize(&values)?;
        if (field_f64(stored, "mean")? - computed.mean).abs() > 1e-6 {
            return Err("summary_mean_does_not_match_seed_level_outcomes".into());
        }
    }
    if summary_stations != station_ids {
        return Err("scenario_summary_intervention_set_mismatch".into());
    }
    Ok(FamilyInput {
        family_id: family_id.to_string(),
        sha256,
        replication_seeds: seeds,
        baseline_mean_good_units_per_hour: baseline_rates.values().sum::<f64>()
            / replication_count as f64,
        gains_by_station,
    })
}

fn evaluate(mut families: Vec<FamilyInput>) -> Result<Report, String> {
    if families.len() < MIN_FAMILIES {
        return Err(format!(
            "at_least_{MIN_FAMILIES}_scenario_families_required"
        ));
    }
    families.sort_by(|a, b| a.family_id.cmp(&b.family_id));
    let mut family_ids = BTreeSet::new();
    for family in &families {
        if family.family_id.trim().is_empty() || !family_ids.insert(family.family_id.clone()) {
            return Err("family_ids_must_be_nonempty_and_unique".into());
        }
    }
    let seed_reference = families[0].replication_seeds.clone();
    for family in &families[1..] {
        if family.replication_seeds != seed_reference {
            return Err("all_families_must_use_identical_ordered_seed_lists".into());
        }
    }
    let station_ids: Vec<String> = families[0].gains_by_station.keys().cloned().collect();
    for family in &families[1..] {
        if family.gains_by_station.keys().cloned().collect::<Vec<_>>() != station_ids {
            return Err("station_sets_must_match_across_families".into());
        }
    }

    let mut family_baseline_results = Vec::new();
    let mut family_station_results = Vec::new();
    let mut family_pair_comparisons = Vec::new();
    let mut per_family_station_mean = BTreeMap::<String, BTreeMap<String, f64>>::new();
    let mut per_family_pair_decision = BTreeMap::<(String, String, String), String>::new();
    let mut input_family_digests = BTreeMap::new();
    for family in &families {
        input_family_digests.insert(family.family_id.clone(), family.sha256.clone());
        family_baseline_results.push(FamilyBaselineResult {
            family_id: family.family_id.clone(),
            source_report_sha256: family.sha256.clone(),
            baseline_mean_good_units_per_hour: family.baseline_mean_good_units_per_hour,
            replication_count: family.replication_seeds.len(),
        });
        let family_means: BTreeMap<String, f64> = family
            .gains_by_station
            .iter()
            .map(|(station, rows)| {
                (
                    station.clone(),
                    rows.values().sum::<f64>() / rows.len() as f64,
                )
            })
            .collect();
        let family_best = family_means
            .iter()
            .max_by(|(station_a, gain_a), (station_b, gain_b)| {
                gain_a
                    .total_cmp(gain_b)
                    .then_with(|| station_b.cmp(station_a))
            })
            .ok_or_else(|| "no_station_gain_candidates".to_string())?
            .0
            .clone();
        per_family_station_mean.insert(family.family_id.clone(), family_means.clone());
        for station in &station_ids {
            let values: Vec<f64> = family
                .replication_seeds
                .iter()
                .map(|seed| family.gains_by_station[station][seed])
                .collect();
            let summary = summarize(&values)?;
            family_station_results.push(FamilyStationResult {
                family_id: family.family_id.clone(),
                source_report_sha256: family.sha256.clone(),
                intervention_station_id: station.clone(),
                mean_paired_gain_units_per_hour: summary.mean,
                positive_gain_clear_of_margin: summary
                    .mean_confidence_95_lower
                    .is_some_and(|lower| lower > PRACTICAL_MARGIN),
                paired_gain: summary,
                family_oracle_best_station_id: family_best.clone(),
                regret_vs_family_oracle_units_per_hour: (family_means[&family_best]
                    - family_means[station])
                    .max(0.0),
            });
        }
        for first in 0..station_ids.len() {
            for second in (first + 1)..station_ids.len() {
                let a = &station_ids[first];
                let b = &station_ids[second];
                let values: Vec<f64> = family
                    .replication_seeds
                    .iter()
                    .map(|seed| family.gains_by_station[a][seed] - family.gains_by_station[b][seed])
                    .collect();
                let summary = summarize(&values)?;
                let margin = PRACTICAL_MARGIN;
                let a_wins = values.iter().filter(|value| **value > margin).count();
                let b_wins = values.iter().filter(|value| **value < -margin).count();
                let decision = paired_decision(&summary, margin);
                per_family_pair_decision.insert(
                    (family.family_id.clone(), a.clone(), b.clone()),
                    decision.to_string(),
                );
                family_pair_comparisons.push(FamilyPairComparison {
                    family_id: family.family_id.clone(),
                    station_a_id: a.clone(),
                    station_b_id: b.clone(),
                    mean_paired_effect_a_minus_b_units_per_hour: summary.mean,
                    paired_effect_a_minus_b: summary,
                    empirical_fraction_a_beats_b_beyond_margin: a_wins as f64 / values.len() as f64,
                    empirical_fraction_b_beats_a_beyond_margin: b_wins as f64 / values.len() as f64,
                    practical_equivalence_margin_units_per_hour: margin,
                    decision: decision.to_string(),
                });
            }
        }
    }

    let mut scores = Vec::new();
    for station in &station_ids {
        let family_means: Vec<f64> = families
            .iter()
            .map(|family| per_family_station_mean[&family.family_id][station])
            .collect();
        let worst_family_idx = family_means
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map(|(index, _)| index)
            .expect("nonempty families");
        let worst_family_result = family_station_results
            .iter()
            .filter(|row| row.intervention_station_id == *station)
            .min_by(|a, b| {
                a.paired_gain
                    .mean_confidence_95_lower
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(
                        &b.paired_gain
                            .mean_confidence_95_lower
                            .unwrap_or(f64::NEG_INFINITY),
                    )
            })
            .expect("family row exists");
        let positive_count = family_station_results
            .iter()
            .filter(|row| {
                row.intervention_station_id == *station
                    && row
                        .paired_gain
                        .mean_confidence_95_lower
                        .is_some_and(|lower| lower > PRACTICAL_MARGIN)
            })
            .count();
        let regrets: Vec<f64> = family_station_results
            .iter()
            .filter(|row| row.intervention_station_id == *station)
            .map(|row| row.regret_vs_family_oracle_units_per_hour)
            .collect();
        scores.push(RobustStationScore {
            station_id: station.clone(),
            mean_gain_across_families_units_per_hour: family_means.iter().sum::<f64>()
                / family_means.len() as f64,
            worst_family_mean_gain_units_per_hour: family_means
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
            worst_family_gain_ci_lower_units_per_hour: worst_family_result
                .paired_gain
                .mean_confidence_95_lower,
            families_with_gain_ci_lower_above_margin: positive_count,
            family_count: families.len(),
            mean_regret_vs_family_oracle_units_per_hour: regrets.iter().sum::<f64>()
                / regrets.len() as f64,
            maximum_regret_vs_family_oracle_units_per_hour: regrets
                .iter()
                .copied()
                .fold(0.0, f64::max),
            pareto_nondominated: false,
        });
    }
    let mut frontier_ids = Vec::new();
    for i in 0..scores.len() {
        let dominated = (0..scores.len()).any(|j| {
            if i == j {
                return false;
            }
            let other = &scores[j];
            let this = &scores[i];
            let no_worse = other.mean_gain_across_families_units_per_hour
                >= this.mean_gain_across_families_units_per_hour
                && other.worst_family_mean_gain_units_per_hour
                    >= this.worst_family_mean_gain_units_per_hour
                && other.maximum_regret_vs_family_oracle_units_per_hour
                    <= this.maximum_regret_vs_family_oracle_units_per_hour;
            let strictly_better = other.mean_gain_across_families_units_per_hour
                > this.mean_gain_across_families_units_per_hour
                || other.worst_family_mean_gain_units_per_hour
                    > this.worst_family_mean_gain_units_per_hour
                || other.maximum_regret_vs_family_oracle_units_per_hour
                    < this.maximum_regret_vs_family_oracle_units_per_hour;
            no_worse && strictly_better
        });
        scores[i].pareto_nondominated = !dominated;
        if !dominated {
            frontier_ids.push(scores[i].station_id.clone());
        }
    }

    let mut robust_best = Vec::new();
    for station in &station_ids {
        let wins_all = families.iter().all(|family| {
            station_ids
                .iter()
                .filter(|other| *other != station)
                .all(|other| {
                    let (a, b) = if station < other {
                        (station, other)
                    } else {
                        (other, station)
                    };
                    let Some(decision) = per_family_pair_decision.get(&(
                        family.family_id.clone(),
                        a.clone(),
                        b.clone(),
                    )) else {
                        return false;
                    };
                    if station == a {
                        decision == "station_a_meaningfully_better_under_paired_model"
                    } else {
                        decision == "station_b_meaningfully_better_under_paired_model"
                    }
                })
        });
        if wins_all {
            robust_best.push(station.clone());
        }
    }
    let robust_best_station_id = (robust_best.len() == 1).then(|| robust_best[0].clone());
    let recommendation_status = if robust_best_station_id.is_some() {
        "one_intervention_clears_paired_margin_in_every_family"
    } else {
        "no_unique_cross_family_winner_abstain"
    }
    .to_string();
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: BENCHMARK_ID.into(),
        family_count: families.len(),
        family_ids: family_ids.into_iter().collect(),
        shared_replication_count: seed_reference.len(),
        shared_replication_seeds: seed_reference,
        practical_equivalence_margin_units_per_hour: PRACTICAL_MARGIN,
        input_family_digests,
        family_baseline_results,
        family_station_results,
        family_pair_comparisons,
        station_scores: scores,
        pareto_frontier_station_ids: frontier_ids,
        robust_best_station_id,
        recommendation_status,
        family_aggregation_note: "Family regimes are separate model specifications, not IID replications. Family summaries and worst-case metrics are reported individually; they are not pooled into one fictitious larger sample. Robust-best requires the paired effect interval to clear the practical margin against every alternative in every family.".into(),
        results_are_synthetic_model_conditional: true,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn paired_decision(summary: &DistributionSummary, margin: f64) -> &'static str {
    match (
        summary.mean_confidence_95_lower,
        summary.mean_confidence_95_upper,
    ) {
        (Some(lower), Some(_upper)) if lower > margin => {
            "station_a_meaningfully_better_under_paired_model"
        }
        (Some(_lower), Some(upper)) if upper < -margin => {
            "station_b_meaningfully_better_under_paired_model"
        }
        (Some(lower), Some(upper)) if lower >= -margin && upper <= margin => {
            "practically_equivalent_within_declared_margin"
        }
        _ => "inconclusive_under_paired_interval",
    }
}

fn summarize(values: &[f64]) -> Result<DistributionSummary, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("distribution_requires_nonempty_finite_samples".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let mean = sorted.iter().sum::<f64>() / n as f64;
    let sample_standard_deviation = if n > 1 {
        Some(
            (sorted
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (n - 1) as f64)
                .sqrt(),
        )
    } else {
        None
    };
    let confidence = sample_standard_deviation.map(|sd| {
        let margin = student_t_critical_95((n - 1) as u32) * sd / (n as f64).sqrt();
        (mean - margin, mean + margin)
    });
    Ok(DistributionSummary {
        sample_count: n,
        mean,
        sample_standard_deviation,
        minimum: sorted[0],
        p05: quantile(&sorted, 0.05),
        median: quantile(&sorted, 0.5),
        p95: quantile(&sorted, 0.95),
        maximum: sorted[n - 1],
        mean_confidence_95_lower: confidence.map(|v| v.0),
        mean_confidence_95_upper: confidence.map(|v| v.1),
    })
}

fn quantile(sorted: &[f64], p: f64) -> f64 {
    let pos = p.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    sorted[lo] * (1.0 - (pos - lo as f64)) + sorted[hi] * (pos - lo as f64)
}
fn student_t_critical_95(df: u32) -> f64 {
    match df {
        0 => f64::NAN,
        1 => 12.706,
        2 => 4.303,
        3 => 3.182,
        4 => 2.776,
        5 => 2.571,
        6 => 2.447,
        7 => 2.365,
        8 => 2.306,
        9 => 2.262,
        10 => 2.228,
        11 => 2.201,
        12 => 2.179,
        13 => 2.160,
        14 => 2.145,
        15 => 2.131,
        16 => 2.120,
        17 => 2.110,
        18 => 2.101,
        19 => 2.093,
        20 => 2.086,
        21 => 2.080,
        22 => 2.074,
        23 => 2.069,
        24 => 2.064,
        25 => 2.060,
        26 => 2.056,
        27 => 2.052,
        28 => 2.048,
        29 => 2.045,
        _ => 1.960,
    }
}
fn field_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key}_missing_or_not_string"))
}
fn field_bool(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{key}_missing_or_not_bool"))
}
fn field_u64(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key}_missing_or_not_u64"))
}
fn field_usize(value: &Value, key: &str) -> Result<usize, String> {
    usize::try_from(field_u64(value, key)?).map_err(|_| format!("{key}_out_of_range"))
}
fn field_f64(value: &Value, key: &str) -> Result<f64, String> {
    let out = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{key}_missing_or_not_number"))?;
    if !out.is_finite() {
        return Err(format!("{key}_must_be_finite"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_distinguish_advantage_equivalence_and_uncertainty() {
        let mut clear = summarize(&[0.8, 0.8, 0.8, 0.8, 0.8]).unwrap();
        assert_eq!(
            paired_decision(&clear, 0.05),
            "station_a_meaningfully_better_under_paired_model"
        );
        let equal = summarize(&[0.01, 0.02, 0.00, 0.01, 0.02]).unwrap();
        assert_eq!(
            paired_decision(&equal, 0.05),
            "practically_equivalent_within_declared_margin"
        );
        clear.mean_confidence_95_lower = Some(-0.2);
        clear.mean_confidence_95_upper = Some(0.3);
        assert_eq!(
            paired_decision(&clear, 0.05),
            "inconclusive_under_paired_interval"
        );
    }

    fn family(id: &str, a: &[f64], b: &[f64]) -> FamilyInput {
        let seeds: Vec<u64> = (1..=a.len() as u64).collect();
        FamilyInput {
            family_id: id.to_string(),
            sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            replication_seeds: seeds.clone(),
            baseline_mean_good_units_per_hour: 10.0,
            gains_by_station: BTreeMap::from([
                (
                    "a".to_string(),
                    seeds.iter().copied().zip(a.iter().copied()).collect(),
                ),
                (
                    "b".to_string(),
                    seeds.iter().copied().zip(b.iter().copied()).collect(),
                ),
            ]),
        }
    }

    #[test]
    fn robust_winner_requires_paired_margin_to_hold_in_every_family() {
        let inputs = vec![
            family("nominal", &[0.8; 5], &[0.0; 5]),
            family("supply", &[0.8; 5], &[0.0; 5]),
            family("setup", &[0.8; 5], &[0.0; 5]),
            family("repair", &[0.8; 5], &[0.0; 5]),
            family("buffers", &[0.8; 5], &[0.0; 5]),
        ];
        let report = evaluate(inputs).expect("robust synthetic inputs");
        assert_eq!(report.robust_best_station_id.as_deref(), Some("a"));
        assert_eq!(report.family_pair_comparisons.len(), 5);
        assert_eq!(report.family_count, 5);
        assert!(report.production_release_authorized == false);
        assert!(report.results_are_synthetic_model_conditional);
    }

    #[test]
    fn regime_switch_causes_cross_family_abstention() {
        let inputs = vec![
            family("nominal", &[0.8; 5], &[0.0; 5]),
            family("supply", &[0.8; 5], &[0.0; 5]),
            family("setup", &[0.0; 5], &[0.8; 5]),
            family("repair", &[0.0; 5], &[0.8; 5]),
            family("buffers", &[0.0; 5], &[0.8; 5]),
        ];
        let report = evaluate(inputs).expect("regime-switch inputs");
        assert_eq!(report.robust_best_station_id, None);
        assert_eq!(
            report.recommendation_status,
            "no_unique_cross_family_winner_abstain"
        );
    }

    #[test]
    fn practical_equivalence_does_not_force_a_robust_winner() {
        let inputs = vec![
            family("nominal", &[0.02; 5], &[0.0; 5]),
            family("supply", &[0.02; 5], &[0.0; 5]),
            family("setup", &[0.02; 5], &[0.0; 5]),
            family("repair", &[0.02; 5], &[0.0; 5]),
            family("buffers", &[0.02; 5], &[0.0; 5]),
        ];
        let report = evaluate(inputs).expect("equivalence inputs");
        assert_eq!(report.robust_best_station_id, None);
        assert!(report.family_pair_comparisons.iter().all(|comparison| {
            comparison.decision == "practically_equivalent_within_declared_margin"
        }));
    }

    #[test]
    fn empirical_quantiles_and_student_t_interval_are_deterministic() {
        let summary = summarize(&[1.0, 2.0, 3.0]).unwrap();
        assert!((summary.mean - 2.0).abs() < 1e-12);
        assert!((summary.p05 - 1.1).abs() < 1e-12);
        assert_eq!(
            summary.mean_confidence_95_lower,
            Some(2.0 - 4.303 / 3.0_f64.sqrt())
        );
    }
}
