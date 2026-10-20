use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process};

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
    method_scores: Vec<MethodScore>,
    results_are_synthetic_model_conditional: bool,
    result_scores_are_field_sota_claims: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: ev_factory_intervention_benchmark <simulation-report.json> <simulation-report-sha256>");
        process::exit(2);
    }
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
    let report = match evaluate(&simulation, &args[2]) {
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
                || statistic.paired_good_units_per_hour_delta_vs_baseline.is_some()
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
    Ok(())
}

fn evaluate(input: &SimulationReport, report_digest: &str) -> Result<Report, String> {
    if !valid_sha256(report_digest) {
        return Err("source_simulation_report_sha256_must_be_64_hex_characters".into());
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
            .ok_or_else(|| format!("baseline_changeover_summary_missing_for_{}", station.station_id))?;
        let effect = input
            .scenario_statistics
            .iter()
            .find(|item| item.intervention_station_id.as_deref() == Some(station.station_id.as_str()))
            .and_then(|item| item.paired_good_units_per_hour_delta_vs_baseline.as_ref())
            .ok_or_else(|| format!("paired_intervention_effect_missing_for_{}", station.station_id))?;
        candidates.push(StationCandidate {
            station_id: station.station_id.clone(),
            utilization_fraction: station.busy_fraction,
            busy_plus_down_proxy_fraction: (station.busy_fraction + station.down_fraction).min(1.0),
            flow_pressure_heuristic_score: station.busy_fraction
                + 0.5 * station.blocked_fraction
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

    let methods: Vec<(DiagnosticMethod, fn(&StationCandidate) -> f64)> = vec![
        (DiagnosticMethod::UtilizationMaximum, |candidate| candidate.utilization_fraction),
        (DiagnosticMethod::BusyPlusDownProxy, |candidate| candidate.busy_plus_down_proxy_fraction),
        (DiagnosticMethod::FlowPressureHeuristic, |candidate| candidate.flow_pressure_heuristic_score),
        (DiagnosticMethod::ChangeoverBurden, |candidate| candidate.mean_changeovers_started),
    ];
    let method_scores = methods
        .into_iter()
        .map(|(method, score)| score_method(method, score, &candidates, &oracle_ranked, intervals_separate))
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
        interval_screen_note: "Non-overlap of the two individual mean confidence intervals is a conservative descriptive screen, not a paired hypothesis test. The report lacks a paired interval for the difference between intervention effects; without that, abstain from a firm intervention recommendation.".into(),
        method_scores,
        results_are_synthetic_model_conditional: true,
        result_scores_are_field_sota_claims: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn score_method(
    method: DiagnosticMethod,
    score: fn(&StationCandidate) -> f64,
    candidates: &[StationCandidate],
    oracle_ranked: &[StationCandidate],
    oracle_interval_separates_top_two: bool,
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
        candidates.iter().find(|candidate| &candidate.station_id == station_id)
    });
    let regret = selected.map(|candidate| {
        (best_gain - candidate.oracle_mean_paired_gain_units_per_hour).max(0.0)
    });
    let relative_regret = regret.map(|value| {
        if best_gain.abs() > 1e-9 { value / best_gain.abs() } else { 0.0 }
    });
    let score_map: BTreeMap<&str, f64> = candidates
        .iter()
        .map(|candidate| (candidate.station_id.as_str(), score(candidate)))
        .collect();
    let oracle_map: BTreeMap<&str, f64> = candidates
        .iter()
        .map(|candidate| (candidate.station_id.as_str(), candidate.oracle_mean_paired_gain_units_per_hour))
        .collect();
    let method_values: Vec<f64> = ranked.iter().map(|candidate| score_map[candidate.station_id.as_str()]).collect();
    let oracle_values: Vec<f64> = ranked.iter().map(|candidate| oracle_map[candidate.station_id.as_str()]).collect();
    let spearman = spearman_rank_correlation(&method_values, &oracle_values);
    let kendall = kendall_tau_b(&method_values, &oracle_values);
    let exact_agreement = predicted
        .as_ref()
        .is_some_and(|station_id| station_id == &oracle_ranked[0].station_id);
    let disposition = if abstained {
        "abstain_signal_tie".to_string()
    } else if !oracle_interval_separates_top_two {
        "ranking_ambiguous_under_individual_interval_screen_no_recommendation".to_string()
    } else if selected.is_some_and(|candidate| candidate.oracle_paired_gain_ci_95_lower.is_some_and(|lower| lower > 0.0))
        && exact_agreement
    {
        "matches_top_synthetic_counterfactual_only_not_field_recommendation".to_string()
    } else {
        "diagnostic_signal_not_sufficient_for_intervention_recommendation".to_string()
    };
    MethodScore {
        method: method.name().to_string(),
        ranked_station_ids: ranked.iter().map(|candidate| candidate.station_id.clone()).collect(),
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
    let covariance = x.iter().zip(&y).map(|(a,b)| (a-mean_x)*(b-mean_y)).sum::<f64>();
    let variance_x = x.iter().map(|a| (a-mean_x).powi(2)).sum::<f64>();
    let variance_y = y.iter().map(|a| (a-mean_y).powi(2)).sum::<f64>();
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
        for j in (i+1)..left.len() {
            let dx = (left[i] - left[j]).total_cmp(&0.0);
            let dy = (right[i] - right[j]).total_cmp(&0.0);
            let left_tie = (left[i] - left[j]).abs() <= 1e-9;
            let right_tie = (right[i] - right[j]).abs() <= 1e-9;
            if left_tie && right_tie { continue; }
            if left_tie { ties_left += 1; continue; }
            if right_tie { ties_right += 1; continue; }
            if matches!((dx,dy),(std::cmp::Ordering::Less,std::cmp::Ordering::Less)|(std::cmp::Ordering::Greater,std::cmp::Ordering::Greater)) {
                concordant += 1;
            } else {
                discordant += 1;
            }
        }
    }
    let denominator = (((concordant + discordant + ties_left) as f64)
        * ((concordant + discordant + ties_right) as f64)).sqrt();
    if denominator <= 0.0 { None } else { Some((concordant - discordant) as f64 / denominator) }
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

    fn candidate(id: &str, busy: f64, blocked: f64, starved: f64, down: f64, changes: f64, gain: f64, lower: f64, upper: f64) -> StationCandidate {
        StationCandidate {
            station_id:id.into(),
            utilization_fraction:busy,
            busy_plus_down_proxy_fraction:(busy+down).min(1.0),
            flow_pressure_heuristic_score:busy +0.5*blocked-starved-down,
            mean_changeovers_started:changes,
            oracle_mean_paired_gain_units_per_hour:gain,
            oracle_paired_gain_ci_95_lower:Some(lower),
            oracle_paired_gain_ci_95_upper:Some(upper),
        }
    }

    #[test]
    fn rank_correlations_and_ties_are_hand_computable() {
        assert!((spearman_rank_correlation(&[3.0,2.0,1.0], &[3.0,2.0,1.0]).unwrap()-1.0).abs()<1e-12);
        assert!((spearman_rank_correlation(&[3.0,2.0,1.0], &[1.0,2.0,3.0]).unwrap()+1.0).abs()<1e-12);
        assert!((kendall_tau_b(&[3.0,2.0,1.0], &[3.0,2.0,1.0]).unwrap()-1.0).abs()<1e-12);
        assert!(spearman_rank_correlation(&[1.0,1.0,1.0], &[1.0,2.0,3.0]).is_none());
    }

    #[test]
    fn diagnostics_and_oracle_gain_are_separate_inputs() {
        let original = vec![
            candidate("a", 0.99, 0.01, 0.01, 0.0, 4.0, 0.2, 0.1, 0.3),
            candidate("b", 0.70, 0.40, 0.01, 0.0, 2.0, 3.0, 2.8, 3.2),
            candidate("c", 0.60, 0.01, 0.30, 0.0, 1.0, -0.1, -0.3, 0.1),
        ];
        assert_eq!(
            original.iter().max_by(|a,b| a.utilization_fraction.total_cmp(&b.utilization_fraction)).unwrap().station_id,
            "a"
        );
        assert_eq!(
            original.iter().max_by(|a,b| a.oracle_mean_paired_gain_units_per_hour.total_cmp(&b.oracle_mean_paired_gain_units_per_hour)).unwrap().station_id,
            "b"
        );
        let mut relabeled_oracle = original.clone();
        for item in &mut relabeled_oracle {
            item.oracle_mean_paired_gain_units_per_hour *= -1.0;
        }
        // Diagnostic features remain unchanged when the counterfactual outcome changes.
        assert_eq!(
            original.iter().max_by(|a,b| a.utilization_fraction.total_cmp(&b.utilization_fraction)).unwrap().station_id,
            relabeled_oracle.iter().max_by(|a,b| a.utilization_fraction.total_cmp(&b.utilization_fraction)).unwrap().station_id
        );
    }

    #[test]
    fn input_digest_is_fail_closed() {
        assert!(!valid_sha256("x"));
        assert!(valid_sha256(&"a".repeat(64)));
    }
}
