use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const THRESHOLDS: [u64; 4] = [300, 900, 1_800, 3_600];
const PRIMARY_FAMILY_SIZE: usize = 24;
const T_CRITICAL_CONSERVATIVE: f64 = 4.0;
const FLOAT_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Deserialize)]
struct SimulationReport {
    benchmark_id: String,
    dispatch_policy: String,
    maximum_queue_age_seconds: u64,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    scenario_count: usize,
    release_ready: bool,
    output_never_exceeds_released_input: bool,
    all_buffers_respected_capacity: bool,
    station_time_accounting_within_horizon: bool,
    replication_scenario_records: Vec<ReplicationRecord>,
}

#[derive(Debug, Deserialize)]
struct ReplicationRecord {
    seed: u64,
    scenario_id: String,
    measured_good_units_per_hour: f64,
    good_units_total: u64,
    released_jobs_total: u64,
}

#[derive(Debug, Deserialize)]
struct SensitivityReport {
    schema_version: String,
    benchmark_id: String,
    baseline_policy: String,
    candidate_policy: String,
    thresholds_seconds: Vec<u64>,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    paired_seed_scenario_keys_match: bool,
    primary_metric_family: String,
    primary_comparison_count: usize,
    primary_familywise_interval_method: String,
    primary_throughput_familywise_intervals: Vec<PrimaryInterval>,
}

#[derive(Debug, Deserialize)]
struct PrimaryInterval {
    maximum_queue_age_seconds: u64,
    scenario_id: String,
    sample_count: usize,
    mean_delta_units_per_hour: f64,
    sample_standard_deviation: f64,
    confidence_95_lower_bonferroni: f64,
    confidence_95_upper_bonferroni: f64,
    family_size: usize,
    t_critical_conservative: f64,
}

type RecordIndex<'a> = BTreeMap<(u64, String), &'a ReplicationRecord>;

fn main() {
    if let Err(error) = run() {
        eprintln!("ev_factory_dispatch_threshold_verify: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 6 {
        return Err(
            "usage: ev_factory_dispatch_threshold_verify BASELINE_JSON SENSITIVITY_JSON 300=PATH 900=PATH 1800=PATH 3600=PATH"
                .into(),
        );
    }

    let baseline: SimulationReport = read_json(&args[0])?;
    validate_report(&baseline, "fifo", None)?;
    let sensitivity: SensitivityReport = read_json(&args[1])?;
    validate_sensitivity(&sensitivity, &baseline)?;

    let baseline_index = index_records(&baseline)?;
    let mut candidates = Vec::new();
    for arg in &args[2..] {
        let (raw_age, path) = arg
            .split_once('=')
            .ok_or_else(|| format!("expected_THRESHOLD=PATH: {arg}"))?;
        let age = raw_age
            .parse::<u64>()
            .map_err(|_| format!("invalid_threshold: {raw_age}"))?;
        let report: SimulationReport = read_json(path)?;
        validate_report(&report, "minimize_changeover", Some(age))?;
        validate_pairing(&baseline, &report)?;
        candidates.push((age, report));
    }

    let actual_thresholds: Vec<u64> = candidates.iter().map(|(age, _)| *age).collect();
    if actual_thresholds != THRESHOLDS || sensitivity.thresholds_seconds != THRESHOLDS {
        return Err(format!(
            "threshold_grid_mismatch_expected_{THRESHOLDS:?}_got_{actual_thresholds:?}"
        ));
    }

    let mut expected_intervals = BTreeMap::new();
    for (age, report) in &candidates {
        let index = index_records(report)?;
        let scenarios: BTreeSet<String> = baseline_index
            .keys()
            .map(|(_, scenario_id)| scenario_id.clone())
            .collect();
        for scenario_id in scenarios {
            let mut deltas = Vec::with_capacity(baseline.replication_count);
            for seed in &baseline.replication_seeds {
                let baseline_record = baseline_index
                    .get(&(*seed, scenario_id.clone()))
                    .ok_or_else(|| format!("baseline_record_missing:{seed}:{scenario_id}"))?;
                let candidate_record = index
                    .get(&(*seed, scenario_id.clone()))
                    .ok_or_else(|| format!("candidate_record_missing:{age}:{seed}:{scenario_id}"))?;
                deltas.push(
                    candidate_record.measured_good_units_per_hour
                        - baseline_record.measured_good_units_per_hour,
                );
            }
            let (mean, sample_sd) = moments(&deltas)?;
            let half_width =
                T_CRITICAL_CONSERVATIVE * sample_sd / (deltas.len() as f64).sqrt();
            let key = (*age, scenario_id.clone());
            if expected_intervals
                .insert(
                    key,
                    (
                        deltas.len(),
                        mean,
                        sample_sd,
                        mean - half_width,
                        mean + half_width,
                    ),
                )
                .is_some()
            {
                return Err("duplicate_recomputed_primary_interval_key".into());
            }
        }
    }

    if expected_intervals.len() != PRIMARY_FAMILY_SIZE {
        return Err(format!(
            "recomputed_family_size_mismatch_expected_{PRIMARY_FAMILY_SIZE}_got_{}",
            expected_intervals.len()
        ));
    }

    let mut reported_intervals = BTreeMap::new();
    for row in &sensitivity.primary_throughput_familywise_intervals {
        let key = (row.maximum_queue_age_seconds, row.scenario_id.clone());
        if reported_intervals.insert(key, row).is_some() {
            return Err("duplicate_reported_primary_interval_key".into());
        }
    }
    if reported_intervals.len() != PRIMARY_FAMILY_SIZE {
        return Err(format!(
            "reported_family_size_mismatch_expected_{PRIMARY_FAMILY_SIZE}_got_{}",
            reported_intervals.len()
        ));
    }

    for (key, (sample_count, mean, sample_sd, lower, upper)) in &expected_intervals {
        let row = reported_intervals
            .get(key)
            .ok_or_else(|| format!("reported_interval_missing:{}:{}", key.0, key.1))?;
        if row.sample_count != *sample_count
            || row.family_size != PRIMARY_FAMILY_SIZE
            || !approximately_equal(row.t_critical_conservative, T_CRITICAL_CONSERVATIVE)
            || !approximately_equal(row.mean_delta_units_per_hour, *mean)
            || !approximately_equal(row.sample_standard_deviation, *sample_sd)
            || !approximately_equal(row.confidence_95_lower_bonferroni, *lower)
            || !approximately_equal(row.confidence_95_upper_bonferroni, *upper)
            || row.confidence_95_lower_bonferroni > row.mean_delta_units_per_hour
            || row.confidence_95_upper_bonferroni < row.mean_delta_units_per_hour
        {
            return Err(format!(
                "primary_interval_recompute_mismatch:{}:{}",
                key.0, key.1
            ));
        }
    }

    println!(
        "PASS: independently recomputed {} Bonferroni-conservative primary throughput intervals from raw paired seed/scenario records; thresholds={:?}; fixed family size={}; conservative t critical={}",
        expected_intervals.len(),
        THRESHOLDS,
        PRIMARY_FAMILY_SIZE,
        T_CRITICAL_CONSERVATIVE
    );
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("read_{path}:{error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("parse_{path}:{error}"))
}

fn validate_report(
    report: &SimulationReport,
    expected_policy: &str,
    expected_age: Option<u64>,
) -> Result<(), String> {
    if report.benchmark_id != "EV-FACTORY-SIM-001" || report.dispatch_policy != expected_policy {
        return Err(format!(
            "unexpected_benchmark_or_policy:{}:{}",
            report.benchmark_id, report.dispatch_policy
        ));
    }
    if expected_age.is_some_and(|age| report.maximum_queue_age_seconds != age) {
        return Err(format!(
            "threshold_mismatch_expected_{expected_age:?}_got_{}",
            report.maximum_queue_age_seconds
        ));
    }
    let seeds: BTreeSet<u64> = report.replication_seeds.iter().copied().collect();
    if report.replication_count != 20
        || report.replication_seeds.len() != 20
        || seeds.len() != 20
        || seeds.contains(&0)
    {
        return Err("expected_twenty_unique_positive_seed_replications".into());
    }
    if report.scenario_count != 6 || report.replication_scenario_records.len() != 120 {
        return Err("expected_six_scenarios_and_120_records".into());
    }
    if report.release_ready
        || !report.output_never_exceeds_released_input
        || !report.all_buffers_respected_capacity
        || !report.station_time_accounting_within_horizon
    {
        return Err("simulation_guardrail_failed_or_release_ready_true".into());
    }
    let mut keys = BTreeSet::new();
    for record in &report.replication_scenario_records {
        if !seeds.contains(&record.seed)
            || record.good_units_total > record.released_jobs_total
            || !record.measured_good_units_per_hour.is_finite()
        {
            return Err("invalid_raw_replication_record".into());
        }
        if !keys.insert((record.seed, record.scenario_id.clone())) {
            return Err("duplicate_seed_scenario_record".into());
        }
    }
    let scenarios: BTreeSet<String> = keys.iter().map(|(_, scenario)| scenario.clone()).collect();
    if scenarios.len() != 6 {
        return Err("expected_six_unique_scenario_ids".into());
    }
    Ok(())
}

fn validate_sensitivity(
    sensitivity: &SensitivityReport,
    baseline: &SimulationReport,
) -> Result<(), String> {
    if sensitivity.schema_version != "ev-factory-age-threshold-sensitivity/0.2"
        || sensitivity.benchmark_id != baseline.benchmark_id
        || sensitivity.baseline_policy != "fifo"
        || sensitivity.candidate_policy != "minimize_changeover"
        || sensitivity.replication_count != baseline.replication_count
        || sensitivity.replication_seeds != baseline.replication_seeds
        || !sensitivity.paired_seed_scenario_keys_match
        || sensitivity.primary_comparison_count != PRIMARY_FAMILY_SIZE
        || sensitivity.primary_throughput_familywise_intervals.len() != PRIMARY_FAMILY_SIZE
        || sensitivity.thresholds_seconds != THRESHOLDS
        || !sensitivity.primary_metric_family.contains("good units/hour delta vs FIFO")
        || !sensitivity
            .primary_familywise_interval_method
            .contains("Bonferroni")
    {
        return Err("sensitivity_metadata_or_family_size_invalid".into());
    }
    Ok(())
}

fn index_records(report: &SimulationReport) -> Result<RecordIndex<'_>, String> {
    let mut index = BTreeMap::new();
    for record in &report.replication_scenario_records {
        let key = (record.seed, record.scenario_id.clone());
        if index.insert(key, record).is_some() {
            return Err("duplicate_seed_scenario_record".into());
        }
    }
    Ok(index)
}

fn validate_pairing(
    baseline: &SimulationReport,
    candidate: &SimulationReport,
) -> Result<(), String> {
    if baseline.replication_seeds != candidate.replication_seeds
        || baseline.scenario_count != candidate.scenario_count
    {
        return Err("paired_replication_metadata_mismatch".into());
    }
    let baseline_keys: BTreeSet<(u64, String)> = index_records(baseline)?.keys().cloned().collect();
    let candidate_keys: BTreeSet<(u64, String)> =
        index_records(candidate)?.keys().cloned().collect();
    if baseline_keys != candidate_keys {
        return Err("paired_seed_scenario_key_mismatch".into());
    }
    Ok(())
}

fn moments(deltas: &[f64]) -> Result<(f64, f64), String> {
    if deltas.len() != 20 || deltas.iter().any(|value| !value.is_finite()) {
        return Err("twenty_finite_paired_deltas_required".into());
    }
    let n = deltas.len() as f64;
    let mean = deltas.iter().sum::<f64>() / n;
    let variance = deltas
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let sample_sd = variance.sqrt();
    if !mean.is_finite() || !sample_sd.is_finite() {
        return Err("non_finite_paired_moments".into());
    }
    Ok((mean, sample_sd))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= FLOAT_TOLERANCE * left.abs().max(right.abs()).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_moments_use_twenty_paired_deltas() {
        let values: Vec<f64> = (1..=20).map(|value| value as f64).collect();
        let (mean, sd) = moments(&values).expect("moments");
        assert!(approximately_equal(mean, 10.5));
        assert!(approximately_equal(sd, 35.0_f64.sqrt()));
        assert!(moments(&values[..19]).is_err());
    }

    #[test]
    fn tolerance_is_relative_and_stable_near_zero() {
        assert!(approximately_equal(1.0, 1.0 + 1e-10));
        assert!(approximately_equal(0.0, 1e-10));
        assert!(!approximately_equal(1.0, 1.01));
    }
}
