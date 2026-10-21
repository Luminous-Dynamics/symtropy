use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const CLAIM_CEILING: &str = "Synthetic intervention-ranking evaluator only. Its oracle is the paired counterfactual result of the declared simulator, not a measured factory truth, industry benchmark, causal production estimate, or capital-investment recommendation.";

#[derive(Debug, Clone, Deserialize)]
struct SimulationReport {
    schema_version: String,
    benchmark_id: String,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    dispatch_policy: String,
    scenarios: Vec<SimulationScenario>,
    scenario_statistics: Vec<SimulationScenarioStatistic>,
    replication_outcomes: Vec<ReplicationOutcome>,
    station_sequence_statistics: Vec<StationSequenceStatistic>,
    release_ready: bool,
    output_never_exceeds_released_input: bool,
    all_buffers_respected_capacity: bool,
    station_time_accounting_within_horizon: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SimulationScenario {
    scenario_id: String,
    intervention_station_id: Option<String>,
    stations: Vec<StationSignal>,
}

#[derive(Debug, Clone, Deserialize)]
struct StationSignal {
    station_id: String,
    busy_fraction: f64,
    blocked_fraction: f64,
    starved_fraction: f64,
    down_fraction: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct SimulationScenarioStatistic {
    scenario_id: String,
    intervention_station_id: Option<String>,
    paired_good_units_per_hour_delta_vs_baseline: Option<DistributionSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReplicationOutcome {
    seed: u64,
    scenario_id: String,
    intervention_station_id: Option<String>,
    good_units_per_hour: f64,
    baseline_good_units_per_hour: f64,
    paired_good_units_per_hour_delta_vs_baseline: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct StationSequenceStatistic {
    scenario_id: String,
    station_id: String,
    changeovers_started: DistributionSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum DiagnosticMethod {
    UtilizationMaximum,
    BusyPlusDownProxy,
    FlowPressureHeuristic,
    ChangeoverBurden,
}

impl DiagnosticMethod {
    const ALL: [Self; 4] = [
        Self::UtilizationMaximum,
        Self::BusyPlusDownProxy,
        Self::FlowPressureHeuristic,
        Self::ChangeoverBurden,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::UtilizationMaximum => "utilization_maximum",
            Self::BusyPlusDownProxy => "busy_plus_down_proxy",
            Self::FlowPressureHeuristic => "flow_pressure_heuristic",
            Self::ChangeoverBurden => "changeover_burden",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationCandidate {
    station_id: String,
    utilization_fraction: f64,
    busy_plus_down_proxy_fraction: f64,
    flow_pressure_heuristic_score: f64,
    mean_changeovers_started: f64,
    oracle_mean_paired_gain_units_per_hour: f64,
    oracle_paired_gain_ci_95_lower: Option<f64>,
    oracle_paired_gain_ci_95_upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct MethodScore {
    method: String,
    ranked_station_ids: Vec<String>,
    predicted_best_station_id: Option<String>,
    prediction_abstained_on_tie: bool,
    spearman_rank_correlation_vs_oracle: Option<f64>,
    kendall_tau_b_vs_oracle: Option<f64>,
    exact_top1_agreement_with_point_estimate_oracle: bool,
    absolute_regret_units_per_hour_vs_point_estimate_oracle: Option<f64>,
    relative_regret_fraction_vs_point_estimate_oracle: Option<f64>,
    policy_disposition: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PairedInterventionComparison {
    station_a_id: String,
    station_b_id: String,
    difference_definition: String,
    mean_paired_effect_a_minus_b_units_per_hour: f64,
    paired_effect_a_minus_b: DistributionSummary,
    probability_a_beats_b_beyond_margin: f64,
    probability_b_beats_a_beyond_margin: f64,
    practical_equivalence_margin_units_per_hour: f64,
    decision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_simulation_report_sha256: String,
    source_simulation_replication_count: usize,
    source_simulation_seed_count: usize,
    source_dispatch_policy: String,
    candidate_count: usize,
    candidates: Vec<StationCandidate>,
    oracle_best_station_by_mean_paired_gain: String,
    oracle_best_station_ci_lower: Option<f64>,
    runner_up_station_by_mean_paired_gain: String,
    runner_up_station_ci_upper: Option<f64>,
    nonoverlapping_interval_screen_separates_top_two: bool,
    interval_screen_note: String,
    practical_equivalence_margin_units_per_hour: f64,
    paired_intervention_comparisons: Vec<PairedInterventionComparison>,
    robust_best_station_id: Option<String>,
    paired_effect_recommendation_status: String,
    method_scores: Vec<MethodScore>,
    results_are_synthetic_model_conditional: bool,
    result_scores_are_field_sota_claims: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if !(2..=3).contains(&args.len()) {
        eprintln!(
            "usage: ev_factory_intervention_benchmark <simulation-report.json> [practical-margin-units-per-hour]"
        );
        process::exit(2);
    }
    let practical_margin = if args.len() == 3 {
        match args[2].parse::<f64>() {
            Ok(value) if value.is_finite() && value > 0.0 => value,
            _ => {
                eprintln!("practical margin must be a finite positive number");
                process::exit(2);
            }
        }
    } else {
        0.05
    };
    let report_text = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read simulation report {}: {error}", args[1]);
            process::exit(2);
        }
    };
    let simulation: SimulationReport = match serde_json::from_str(&report_text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid simulation report: {error}");
            process::exit(2);
        }
    };
    let source_digest = format!("{:x}", Sha256::digest(report_text.as_bytes()));
    let report = match evaluate(&simulation, &source_digest, practical_margin) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("intervention benchmark rejected input: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize intervention benchmark report: {error}");
            process::exit(2);
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_fraction(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name}_must_be_finite_and_in_range_0_to_1"));
    }
    Ok(())
}

fn validate_simulation_report(input: &SimulationReport) -> Result<(), String> {
    if input.schema_version != "0.1.0" || input.benchmark_id != "EV-FACTORY-SIM-001" {
        return Err("unsupported_simulation_schema_or_benchmark".into());
    }
    if input.replication_count == 0
        || input.replication_seeds.len() != input.replication_count
        || input.replication_seeds.iter().any(|seed| *seed == 0)
    {
        return Err("replication_count_and_seed_list_must_match_and_be_positive".into());
    }
    let seeds: BTreeSet<u64> = input.replication_seeds.iter().copied().collect();
    if seeds.len() != input.replication_seeds.len() {
        return Err("replication_seeds_must_be_unique".into());
    }
    if input.dispatch_policy.trim().is_empty() {
        return Err("dispatch_policy_must_be_declared".into());
    }
    if input.release_ready
        || !input.output_never_exceeds_released_input
        || !input.all_buffers_respected_capacity
        || !input.station_time_accounting_within_horizon
    {
        return Err("source_simulation_report_failed_invariants_or_claims_release_ready".into());
    }
    let baseline = input
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "baseline")
        .ok_or_else(|| "baseline_scenario_missing".to_string())?;
    if baseline.intervention_station_id.is_some() || baseline.stations.len() < 2 {
        return Err("baseline_intervention_shape_invalid".into());
    }
    let mut stations = BTreeSet::new();
    for station in &baseline.stations {
        if station.station_id.trim().is_empty() || !stations.insert(station.station_id.clone()) {
            return Err("baseline_station_ids_must_be_nonempty_and_unique".into());
        }
        valid_fraction("busy_fraction", station.busy_fraction)?;
        valid_fraction("blocked_fraction", station.blocked_fraction)?;
        valid_fraction("starved_fraction", station.starved_fraction)?;
        valid_fraction("down_fraction", station.down_fraction)?;
    }
    let mut intervention_stations = BTreeSet::new();
    for scenario in &input.scenarios {
        if scenario.scenario_id != "baseline" {
            let station_id = scenario
                .intervention_station_id
                .as_ref()
                .ok_or_else(|| "intervention_scenario_missing_station_id".to_string())?;
            if !stations.contains(station_id) || !intervention_stations.insert(station_id.clone()) {
                return Err("intervention_station_ids_must_be_unique_and_match_baseline".into());
            }
        }
    }
    if intervention_stations != stations {
        return Err("one_counterfactual_intervention_per_baseline_station_required".into());
    }
    let mut stats_interventions = BTreeSet::new();
    for statistic in &input.scenario_statistics {
        if statistic.scenario_id == "baseline" {
            if statistic.intervention_station_id.is_some()
                || statistic
                    .paired_good_units_per_hour_delta_vs_baseline
                    .is_some()
            {
                return Err("baseline_statistics_must_not_claim_intervention_delta".into());
            }
            continue;
        }
        let station_id = statistic
            .intervention_station_id
            .as_ref()
            .ok_or_else(|| "intervention_statistics_missing_station_id".to_string())?;
        let delta = statistic
            .paired_good_units_per_hour_delta_vs_baseline
            .as_ref()
            .ok_or_else(|| "intervention_paired_delta_missing".to_string())?;
        if !intervention_stations.contains(station_id)
            || !stats_interventions.insert(station_id.clone())
            || delta.sample_count != input.replication_count
            || !delta.mean.is_finite()
            || delta
                .mean_confidence_95_lower
                .is_some_and(|value| !value.is_finite())
            || delta
                .mean_confidence_95_upper
                .is_some_and(|value| !value.is_finite())
        {
            return Err("intervention_statistics_invalid_or_seed_count_mismatch".into());
        }
    }
    if stats_interventions != stations {
        return Err("one_paired_delta_distribution_per_station_required".into());
    }
    if input.station_sequence_statistics.iter().any(|item| {
        item.scenario_id == "baseline"
            && (!stations.contains(&item.station_id)
                || item.changeovers_started.sample_count != input.replication_count
                || !item.changeovers_started.mean.is_finite()
                || item.changeovers_started.mean < 0.0)
    }) {
        return Err("baseline_sequence_metrics_invalid".into());
    }

    let expected_outcome_count = input.replication_count * (stations.len() + 1);
    if input.replication_outcomes.len() != expected_outcome_count {
        return Err("per_seed_outcome_count_must_match_seeds_times_scenarios".into());
    }
    let known_scenarios: BTreeSet<String> = input
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect();
    let mut seen_outcomes = BTreeSet::new();
    let mut outcomes_by_seed: BTreeMap<u64, Vec<&ReplicationOutcome>> = BTreeMap::new();
    for outcome in &input.replication_outcomes {
        if !seeds.contains(&outcome.seed)
            || !known_scenarios.contains(&outcome.scenario_id)
            || !outcome.good_units_per_hour.is_finite()
            || outcome.good_units_per_hour < 0.0
            || !outcome.baseline_good_units_per_hour.is_finite()
            || outcome.baseline_good_units_per_hour < 0.0
            || !seen_outcomes.insert((outcome.seed, outcome.scenario_id.clone()))
        {
            return Err("per_seed_outcome_has_invalid_seed_scenario_value_or_duplicate".into());
        }
        if outcome.scenario_id == "baseline" {
            if outcome.intervention_station_id.is_some()
                || outcome
                    .paired_good_units_per_hour_delta_vs_baseline
                    .is_some()
                || (outcome.good_units_per_hour - outcome.baseline_good_units_per_hour).abs() > 1e-9
            {
                return Err("per_seed_baseline_outcome_shape_or_value_invalid".into());
            }
        } else {
            let station_id = outcome
                .intervention_station_id
                .as_ref()
                .ok_or_else(|| "per_seed_intervention_outcome_missing_station_id".to_string())?;
            let scenario = input
                .scenarios
                .iter()
                .find(|scenario| scenario.scenario_id == outcome.scenario_id)
                .ok_or_else(|| "per_seed_outcome_scenario_missing".to_string())?;
            if scenario.intervention_station_id.as_ref() != Some(station_id) {
                return Err("per_seed_outcome_intervention_station_mismatch".into());
            }
            let delta = outcome
                .paired_good_units_per_hour_delta_vs_baseline
                .ok_or_else(|| "per_seed_intervention_delta_missing".to_string())?;
            if !delta.is_finite()
                || (delta - (outcome.good_units_per_hour - outcome.baseline_good_units_per_hour))
                    .abs()
                    > 1e-9
            {
                return Err("per_seed_intervention_delta_does_not_match_paired_outputs".into());
            }
        }
        outcomes_by_seed
            .entry(outcome.seed)
            .or_default()
            .push(outcome);
    }
    for seed in &input.replication_seeds {
        let rows = outcomes_by_seed
            .get(seed)
            .ok_or_else(|| "seed_has_no_replication_outcomes".to_string())?;
        if rows.len() != stations.len() + 1 {
            return Err("each_seed_must_have_baseline_and_every_intervention".into());
        }
        let baseline_row = rows
            .iter()
            .find(|outcome| outcome.scenario_id == "baseline")
            .ok_or_else(|| "seed_missing_paired_baseline".to_string())?;
        for outcome in rows
            .iter()
            .filter(|outcome| outcome.scenario_id != "baseline")
        {
            if (outcome.baseline_good_units_per_hour - baseline_row.good_units_per_hour).abs()
                > 1e-9
            {
                return Err("seed_outcomes_do_not_share_same_baseline_rate".into());
            }
        }
    }
    Ok(())
}

fn evaluate(
    input: &SimulationReport,
    report_digest: &str,
    practical_equivalence_margin: f64,
) -> Result<Report, String> {
    if !valid_sha256(report_digest) {
        return Err("source_simulation_report_sha256_must_be_64_hex_characters".into());
    }
    if !practical_equivalence_margin.is_finite() || practical_equivalence_margin <= 0.0 {
        return Err("practical_equivalence_margin_must_be_finite_and_positive".into());
    }
    validate_simulation_report(input)?;
    let baseline = input
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "baseline")
        .expect("validated baseline exists");

    // Diagnostic scores are computed only from baseline station features.
    // Paired intervention gains enter only through the separately constructed oracle labels.
    let mut candidates = Vec::with_capacity(baseline.stations.len());
    for station in &baseline.stations {
        let sequence = input
            .station_sequence_statistics
            .iter()
            .find(|item| item.scenario_id == "baseline" && item.station_id == station.station_id)
            .ok_or_else(|| {
                format!(
                    "baseline_changeover_summary_missing_for_{}",
                    station.station_id
                )
            })?;
        let effect = input
            .scenario_statistics
            .iter()
            .find(|item| {
                item.intervention_station_id.as_deref() == Some(station.station_id.as_str())
            })
            .and_then(|item| item.paired_good_units_per_hour_delta_vs_baseline.as_ref())
            .ok_or_else(|| {
                format!(
                    "paired_intervention_effect_missing_for_{}",
                    station.station_id
                )
            })?;
        candidates.push(StationCandidate {
            station_id: station.station_id.clone(),
            utilization_fraction: station.busy_fraction,
            busy_plus_down_proxy_fraction: (station.busy_fraction + station.down_fraction).min(1.0),
            flow_pressure_heuristic_score: station.busy_fraction + 0.5 * station.blocked_fraction
                - station.starved_fraction
                - station.down_fraction,
            mean_changeovers_started: sequence.changeovers_started.mean,
            oracle_mean_paired_gain_units_per_hour: effect.mean,
            oracle_paired_gain_ci_95_lower: effect.mean_confidence_95_lower,
            oracle_paired_gain_ci_95_upper: effect.mean_confidence_95_upper,
        });
    }
    candidates.sort_by(|a, b| a.station_id.cmp(&b.station_id));
    let mut oracle_ranked = candidates.clone();
    oracle_ranked.sort_by(|a, b| {
        b.oracle_mean_paired_gain_units_per_hour
            .total_cmp(&a.oracle_mean_paired_gain_units_per_hour)
            .then_with(|| a.station_id.cmp(&b.station_id))
    });
    let best = oracle_ranked
        .first()
        .ok_or_else(|| "no_intervention_candidates".to_string())?;
    let runner_up = oracle_ranked
        .get(1)
        .ok_or_else(|| "at_least_two_intervention_candidates_required".to_string())?;
    let intervals_separate = match (
        best.oracle_paired_gain_ci_95_lower,
        runner_up.oracle_paired_gain_ci_95_upper,
    ) {
        (Some(lower), Some(upper)) => lower > upper,
        _ => false,
    };

    let replication_outcomes: BTreeMap<(u64, String), &ReplicationOutcome> = input
        .replication_outcomes
        .iter()
        .map(|outcome| ((outcome.seed, outcome.scenario_id.clone()), outcome))
        .collect();
    let mut paired_comparisons = Vec::new();
    for first in 0..candidates.len() {
        for second in (first + 1)..candidates.len() {
            let station_a = &candidates[first].station_id;
            let station_b = &candidates[second].station_id;
            let scenario_a = format!("add-one-server-{station_a}");
            let scenario_b = format!("add-one-server-{station_b}");
            let mut difference_samples = Vec::with_capacity(input.replication_count);
            let mut a_win_count = 0usize;
            let mut b_win_count = 0usize;
            for seed in &input.replication_seeds {
                let outcome_a = replication_outcomes
                    .get(&(*seed, scenario_a.clone()))
                    .ok_or_else(|| format!("missing_seed_scenario_outcome_{seed}_{scenario_a}"))?;
                let outcome_b = replication_outcomes
                    .get(&(*seed, scenario_b.clone()))
                    .ok_or_else(|| format!("missing_seed_scenario_outcome_{seed}_{scenario_b}"))?;
                let gain_a = outcome_a
                    .paired_good_units_per_hour_delta_vs_baseline
                    .ok_or_else(|| "paired_gain_missing_for_station_a".to_string())?;
                let gain_b = outcome_b
                    .paired_good_units_per_hour_delta_vs_baseline
                    .ok_or_else(|| "paired_gain_missing_for_station_b".to_string())?;
                let difference = gain_a - gain_b;
                if difference > practical_equivalence_margin {
                    a_win_count += 1;
                } else if difference < -practical_equivalence_margin {
                    b_win_count += 1;
                }
                difference_samples.push(difference);
            }
            let paired_effect = summarize_distribution(&difference_samples)?;
            let lower = paired_effect.mean_confidence_95_lower;
            let upper = paired_effect.mean_confidence_95_upper;
            let decision = paired_interval_decision(lower, upper, practical_equivalence_margin);
            paired_comparisons.push(PairedInterventionComparison {
                station_a_id: station_a.clone(),
                station_b_id: station_b.clone(),
                difference_definition: "gain_a(seed)-gain_b(seed); each gain is same-seed intervention output minus that same seed's baseline output".into(),
                mean_paired_effect_a_minus_b_units_per_hour: paired_effect.mean,
                paired_effect_a_minus_b: paired_effect,
                probability_a_beats_b_beyond_margin: a_win_count as f64 / input.replication_count as f64,
                probability_b_beats_a_beyond_margin: b_win_count as f64 / input.replication_count as f64,
                practical_equivalence_margin_units_per_hour: practical_equivalence_margin,
                decision: decision.into(),
            });
        }
    }
    let robust_best_station_id = oracle_ranked.first().and_then(|top| {
        let candidate_id = top.station_id.as_str();
        let beats_all = candidates.iter().all(|other| {
            if other.station_id == top.station_id {
                return true;
            }
            paired_comparisons
                .iter()
                .find(|comparison| {
                    (comparison.station_a_id == top.station_id
                        && comparison.station_b_id == other.station_id)
                        || (comparison.station_a_id == other.station_id
                            && comparison.station_b_id == top.station_id)
                })
                .is_some_and(|comparison| {
                    if comparison.station_a_id == top.station_id {
                        comparison.decision == "station_a_meaningfully_better_under_paired_model"
                    } else {
                        comparison.decision == "station_b_meaningfully_better_under_paired_model"
                    }
                })
        });
        if beats_all {
            Some(candidate_id.to_string())
        } else {
            None
        }
    });
    let paired_effect_recommendation_status = if robust_best_station_id.is_some() {
        "unique_best_intervention_separated_from_all_candidates_by_paired_intervals"
    } else {
        "no_robust_best_intervention_detected_abstain"
    }
    .to_string();

    let methods: Vec<(DiagnosticMethod, fn(&StationCandidate) -> f64)> = vec![
        (DiagnosticMethod::UtilizationMaximum, |candidate| {
            candidate.utilization_fraction
        }),
        (DiagnosticMethod::BusyPlusDownProxy, |candidate| {
            candidate.busy_plus_down_proxy_fraction
        }),
        (DiagnosticMethod::FlowPressureHeuristic, |candidate| {
            candidate.flow_pressure_heuristic_score
        }),
        (DiagnosticMethod::ChangeoverBurden, |candidate| {
            candidate.mean_changeovers_started
        }),
    ];
    let method_scores = methods
        .into_iter()
        .map(|(method, score)| {
            score_method(
                method,
                score,
                &candidates,
                &oracle_ranked,
                robust_best_station_id.as_deref(),
            )
        })
        .collect();
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: "EV-FACTORY-INTERVENTION-BENCH-001".into(),
        source_simulation_report_sha256: report_digest.to_ascii_lowercase(),
        source_simulation_replication_count: input.replication_count,
        source_simulation_seed_count: input.replication_seeds.len(),
        source_dispatch_policy: input.dispatch_policy.clone(),
        candidate_count: candidates.len(),
        candidates,
        oracle_best_station_by_mean_paired_gain: best.station_id.clone(),
        oracle_best_station_ci_lower: best.oracle_paired_gain_ci_95_lower,
        runner_up_station_by_mean_paired_gain: runner_up.station_id.clone(),
        runner_up_station_ci_upper: runner_up.oracle_paired_gain_ci_95_upper,
        nonoverlapping_interval_screen_separates_top_two: intervals_separate,
        interval_screen_note: "Legacy descriptive screen only; paired_intervention_comparisons is the primary comparison of intervention-vs-intervention effects.".into(),
        practical_equivalence_margin_units_per_hour: practical_equivalence_margin,
        paired_intervention_comparisons: paired_comparisons,
        robust_best_station_id,
        paired_effect_recommendation_status,
        method_scores,
        results_are_synthetic_model_conditional: true,
        result_scores_are_field_sota_claims: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn paired_interval_decision(lower: Option<f64>, upper: Option<f64>, margin: f64) -> &'static str {
    match (lower, upper) {
        (Some(lo), Some(_hi)) if lo > margin => "station_a_meaningfully_better_under_paired_model",
        (Some(_lo), Some(hi)) if hi < -margin => "station_b_meaningfully_better_under_paired_model",
        (Some(lo), Some(hi)) if lo >= -margin && hi <= margin => {
            "practically_equivalent_within_declared_margin"
        }
        _ => "inconclusive_under_paired_interval",
    }
}

fn summarize_distribution(values: &[f64]) -> Result<DistributionSummary, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("paired_distribution_requires_nonempty_finite_samples".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    let sample_standard_deviation = if count > 1 {
        let squared_error = sorted
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>();
        Some((squared_error / (count - 1) as f64).sqrt())
    } else {
        None
    };
    let confidence = sample_standard_deviation.map(|sd| {
        let margin = student_t_critical_95((count - 1) as u32) * sd / (count as f64).sqrt();
        (mean - margin, mean + margin)
    });
    Ok(DistributionSummary {
        sample_count: count,
        mean,
        sample_standard_deviation,
        minimum: sorted[0],
        p05: quantile_sorted(&sorted, 0.05),
        median: quantile_sorted(&sorted, 0.50),
        p95: quantile_sorted(&sorted, 0.95),
        maximum: sorted[count - 1],
        mean_confidence_95_lower: confidence.map(|bounds| bounds.0),
        mean_confidence_95_upper: confidence.map(|bounds| bounds.1),
    })
}

fn quantile_sorted(sorted: &[f64], probability: f64) -> f64 {
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let fraction = position - lower_index as f64;
    sorted[lower_index] * (1.0 - fraction) + sorted[upper_index] * fraction
}

fn student_t_critical_95(degrees_of_freedom: u32) -> f64 {
    match degrees_of_freedom {
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

fn score_method(
    method: DiagnosticMethod,
    score: fn(&StationCandidate) -> f64,
    candidates: &[StationCandidate],
    oracle_ranked: &[StationCandidate],
    robust_best_station_id: Option<&str>,
) -> MethodScore {
    let mut ranked = candidates.to_vec();
    ranked.sort_by(|a, b| {
        score(b)
            .total_cmp(&score(a))
            .then_with(|| a.station_id.cmp(&b.station_id))
    });
    let top_score = score(ranked.first().expect("at least two candidates"));
    let second_score = score(&ranked[1]);
    let abstained = (top_score - second_score).abs() <= 1e-9;
    let predicted = if abstained {
        None
    } else {
        Some(ranked[0].station_id.clone())
    };
    let best_gain = oracle_ranked[0].oracle_mean_paired_gain_units_per_hour;
    let selected = predicted.as_ref().and_then(|station_id| {
        candidates
            .iter()
            .find(|candidate| &candidate.station_id == station_id)
    });
    let regret = selected
        .map(|candidate| (best_gain - candidate.oracle_mean_paired_gain_units_per_hour).max(0.0));
    let relative_regret = regret.map(|value| {
        if best_gain.abs() > 1e-9 {
            value / best_gain.abs()
        } else {
            0.0
        }
    });
    let score_map: BTreeMap<&str, f64> = candidates
        .iter()
        .map(|candidate| (candidate.station_id.as_str(), score(candidate)))
        .collect();
    let oracle_map: BTreeMap<&str, f64> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.station_id.as_str(),
                candidate.oracle_mean_paired_gain_units_per_hour,
            )
        })
        .collect();
    let method_values: Vec<f64> = ranked
        .iter()
        .map(|candidate| score_map[candidate.station_id.as_str()])
        .collect();
    let oracle_values: Vec<f64> = ranked
        .iter()
        .map(|candidate| oracle_map[candidate.station_id.as_str()])
        .collect();
    let spearman = spearman_rank_correlation(&method_values, &oracle_values);
    let kendall = kendall_tau_b(&method_values, &oracle_values);
    let exact_agreement = predicted
        .as_ref()
        .is_some_and(|station_id| station_id == &oracle_ranked[0].station_id);
    let disposition = if abstained {
        "abstain_signal_tie".to_string()
    } else if robust_best_station_id.is_none() {
        "paired_intervention_effects_not_separated_no_recommendation".to_string()
    } else if predicted.as_deref() == robust_best_station_id {
        "matches_robust_best_synthetic_counterfactual_only_not_field_recommendation".to_string()
    } else {
        "diagnostic_signal_not_sufficient_for_robust_intervention_recommendation".to_string()
    };
    MethodScore {
        method: method.name().to_string(),
        ranked_station_ids: ranked
            .iter()
            .map(|candidate| candidate.station_id.clone())
            .collect(),
        predicted_best_station_id: predicted,
        prediction_abstained_on_tie: abstained,
        spearman_rank_correlation_vs_oracle: spearman,
        kendall_tau_b_vs_oracle: kendall,
        exact_top1_agreement_with_point_estimate_oracle: exact_agreement,
        absolute_regret_units_per_hour_vs_point_estimate_oracle: regret,
        relative_regret_fraction_vs_point_estimate_oracle: relative_regret,
        policy_disposition: disposition,
    }
}

fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let mut ranks = vec![0.0; values.len()];
    let mut begin = 0;
    while begin < order.len() {
        let mut end = begin + 1;
        while end < order.len() && (values[order[end]] - values[order[begin]]).abs() <= 1e-9 {
            end += 1;
        }
        let average_rank = ((begin + 1 + end) as f64) / 2.0;
        for position in begin..end {
            ranks[order[position]] = average_rank;
        }
        begin = end;
    }
    ranks
}

fn spearman_rank_correlation(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let x = average_ranks(left);
    let y = average_ranks(right);
    let mean_x = x.iter().sum::<f64>() / x.len() as f64;
    let mean_y = y.iter().sum::<f64>() / y.len() as f64;
    let covariance = x
        .iter()
        .zip(&y)
        .map(|(a, b)| (a - mean_x) * (b - mean_y))
        .sum::<f64>();
    let variance_x = x.iter().map(|a| (a - mean_x).powi(2)).sum::<f64>();
    let variance_y = y.iter().map(|a| (a - mean_y).powi(2)).sum::<f64>();
    if variance_x <= 0.0 || variance_y <= 0.0 {
        None
    } else {
        Some(covariance / (variance_x * variance_y).sqrt())
    }
}

fn kendall_tau_b(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let mut concordant = 0_i64;
    let mut discordant = 0_i64;
    let mut ties_left = 0_i64;
    let mut ties_right = 0_i64;
    for i in 0..left.len() {
        for j in (i + 1)..left.len() {
            let dx = (left[i] - left[j]).total_cmp(&0.0);
            let dy = (right[i] - right[j]).total_cmp(&0.0);
            let left_tie = (left[i] - left[j]).abs() <= 1e-9;
            let right_tie = (right[i] - right[j]).abs() <= 1e-9;
            if left_tie && right_tie {
                continue;
            }
            if left_tie {
                ties_left += 1;
                continue;
            }
            if right_tie {
                ties_right += 1;
                continue;
            }
            if matches!(
                (dx, dy),
                (std::cmp::Ordering::Less, std::cmp::Ordering::Less)
                    | (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater)
            ) {
                concordant += 1;
            } else {
                discordant += 1;
            }
        }
    }
    let denominator = (((concordant + discordant + ties_left) as f64)
        * ((concordant + discordant + ties_right) as f64))
        .sqrt();
    if denominator <= 0.0 {
        None
    } else {
        Some((concordant - discordant) as f64 / denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(mean: f64, lower: f64, upper: f64) -> DistributionSummary {
        DistributionSummary {
            sample_count: 20,
            mean,
            sample_standard_deviation: Some(0.1),
            minimum: mean - 0.2,
            p05: mean - 0.1,
            median: mean,
            p95: mean + 0.1,
            maximum: mean + 0.2,
            mean_confidence_95_lower: Some(lower),
            mean_confidence_95_upper: Some(upper),
        }
    }

    fn candidate(
        id: &str,
        busy: f64,
        blocked: f64,
        starved: f64,
        down: f64,
        changes: f64,
        gain: f64,
        lower: f64,
        upper: f64,
    ) -> StationCandidate {
        StationCandidate {
            station_id: id.into(),
            utilization_fraction: busy,
            busy_plus_down_proxy_fraction: (busy + down).min(1.0),
            flow_pressure_heuristic_score: busy + 0.5 * blocked - starved - down,
            mean_changeovers_started: changes,
            oracle_mean_paired_gain_units_per_hour: gain,
            oracle_paired_gain_ci_95_lower: Some(lower),
            oracle_paired_gain_ci_95_upper: Some(upper),
        }
    }

    #[test]
    fn rank_correlations_and_ties_are_hand_computable() {
        assert!(
            (spearman_rank_correlation(&[3.0, 2.0, 1.0], &[3.0, 2.0, 1.0]).unwrap() - 1.0).abs()
                < 1e-12
        );
        assert!(
            (spearman_rank_correlation(&[3.0, 2.0, 1.0], &[1.0, 2.0, 3.0]).unwrap() + 1.0).abs()
                < 1e-12
        );
        assert!((kendall_tau_b(&[3.0, 2.0, 1.0], &[3.0, 2.0, 1.0]).unwrap() - 1.0).abs() < 1e-12);
        assert!(spearman_rank_correlation(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]).is_none());
    }

    #[test]
    fn diagnostics_and_oracle_gain_are_separate_inputs() {
        let original = vec![
            candidate("a", 0.99, 0.01, 0.01, 0.0, 4.0, 0.2, 0.1, 0.3),
            candidate("b", 0.70, 0.40, 0.01, 0.0, 2.0, 3.0, 2.8, 3.2),
            candidate("c", 0.60, 0.01, 0.30, 0.0, 1.0, -0.1, -0.3, 0.1),
        ];
        assert_eq!(
            original
                .iter()
                .max_by(|a, b| a.utilization_fraction.total_cmp(&b.utilization_fraction))
                .unwrap()
                .station_id,
            "a"
        );
        assert_eq!(
            original
                .iter()
                .max_by(|a, b| a
                    .oracle_mean_paired_gain_units_per_hour
                    .total_cmp(&b.oracle_mean_paired_gain_units_per_hour))
                .unwrap()
                .station_id,
            "b"
        );
        let mut relabeled_oracle = original.clone();
        for item in &mut relabeled_oracle {
            item.oracle_mean_paired_gain_units_per_hour *= -1.0;
        }
        // Diagnostic features remain unchanged when the counterfactual outcome changes.
        assert_eq!(
            original
                .iter()
                .max_by(|a, b| a.utilization_fraction.total_cmp(&b.utilization_fraction))
                .unwrap()
                .station_id,
            relabeled_oracle
                .iter()
                .max_by(|a, b| a.utilization_fraction.total_cmp(&b.utilization_fraction))
                .unwrap()
                .station_id
        );
    }

    #[test]
    fn paired_effect_of_effects_detects_clear_advantage_and_equivalence() {
        let a_gains = [1.0, 1.0, 1.0, 1.0, 1.0];
        let b_gains = [0.2, 0.2, 0.2, 0.2, 0.2];
        let differences: Vec<f64> = a_gains
            .iter()
            .zip(b_gains.iter())
            .map(|(a, b)| a - b)
            .collect();
        let summary = summarize_distribution(&differences).expect("paired summary");
        assert!((summary.mean - 0.8).abs() < 1e-12);
        assert_eq!(summary.mean_confidence_95_lower, Some(0.8));
        assert_eq!(summary.mean_confidence_95_upper, Some(0.8));
        let margin = 0.05;
        assert!(summary.mean_confidence_95_lower.unwrap() > margin);

        let equal_differences = [0.01, 0.02, 0.00, 0.01, 0.02];
        let equal = summarize_distribution(&equal_differences).expect("equivalence summary");
        assert!(equal.mean_confidence_95_lower.unwrap() >= -margin);
        assert!(equal.mean_confidence_95_upper.unwrap() <= margin);
    }

    #[test]
    fn paired_intervention_difference_is_seed_aligned_and_not_marginal_ci_overlap() {
        let baseline = [10.0, 11.0, 9.0, 12.0, 10.5];
        let a_output = [12.0, 13.0, 11.0, 14.0, 12.5];
        let b_output = [11.0, 13.0, 10.0, 13.0, 11.5];
        let paired_effects: Vec<f64> = baseline
            .iter()
            .enumerate()
            .map(|(i, base)| (a_output[i] - base) - (b_output[i] - base))
            .collect();
        assert_eq!(paired_effects, vec![1.0, 0.0, 1.0, 1.0, 1.0]);
        let summary = summarize_distribution(&paired_effects).expect("paired effect of effects");
        assert_eq!(summary.sample_count, 5);
        assert!((summary.mean - 0.8).abs() < 1e-12);
    }

    #[test]
    fn margin_is_explicit_and_can_change_equivalence_decision() {
        let lower = Some(0.08);
        let upper = Some(0.08);
        assert_eq!(
            paired_interval_decision(lower, upper, 0.05),
            "station_a_meaningfully_better_under_paired_model"
        );
        assert_eq!(
            paired_interval_decision(lower, upper, 0.10),
            "practically_equivalent_within_declared_margin"
        );
        assert_eq!(
            paired_interval_decision(Some(0.02), Some(0.12), 0.05),
            "inconclusive_under_paired_interval"
        );
        assert_eq!(
            paired_interval_decision(None, None, 0.05),
            "inconclusive_under_paired_interval"
        );
    }

    #[test]
    fn input_digest_is_fail_closed() {
        assert!(!valid_sha256("x"));
        assert!(valid_sha256(&"a".repeat(64)));
    }
}
