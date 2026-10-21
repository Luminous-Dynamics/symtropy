use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{env, fs, process};

const BENCHMARK_ID: &str = "EV-FACTORY-KPI-001";
const SCHEMA_VERSION: &str = "0.1.0";
const CLAIM_CEILING: &str = "Evidence-aware KPI envelope generated from a synthetic factory DES report. ISO 22400 is used as an informative vocabulary crosswalk only; this output does not claim standards conformance, calibrated plant performance, commercial SOTA, or production/safety/quality/legal release.";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Computability {
    Computed,
    NotEstimable,
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

#[derive(Debug, Clone, Serialize, PartialEq)]
struct KpiMetric {
    kpi_id: String,
    name: String,
    formula_id: String,
    formula_definition: String,
    unit: String,
    computability: Computability,
    value: Option<f64>,
    distribution: Option<DistributionSummary>,
    reference_window: String,
    numerator_definition: Option<String>,
    denominator_definition: Option<String>,
    source_fields: Vec<String>,
    evidence_class: String,
    unavailable_reason: Option<String>,
    interpretation: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct CrosswalkItem {
    metric_family: String,
    informative_iso_22400_concept: String,
    implementation_note: String,
    conformance_claim: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct KpiReport {
    schema_version: String,
    benchmark_id: String,
    source_catalog_sha256: String,
    source_simulation_report_sha256: String,
    source_evidence_class: String,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    dispatch_policy: String,
    metrics: Vec<KpiMetric>,
    iso_22400_informative_crosswalk: Vec<CrosswalkItem>,
    model_safety_quality_compliance_release_authorized: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: ev_factory_kpi_contract <catalog.json> <simulation-report.json>");
        process::exit(2);
    }
    let catalog_bytes = match fs::read(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read catalog {}: {error}", args[0]);
            process::exit(2);
        }
    };
    let report_bytes = match fs::read(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read simulation report {}: {error}", args[1]);
            process::exit(2);
        }
    };
    let catalog: Value = match serde_json::from_slice(&catalog_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid catalog JSON: {error}");
            process::exit(2);
        }
    };
    let simulation: Value = match serde_json::from_slice(&report_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid simulation report JSON: {error}");
            process::exit(2);
        }
    };
    let catalog_sha = sha256(&catalog_bytes);
    let report_sha = sha256(&report_bytes);
    let report = match build_report(&catalog, &simulation, &catalog_sha, &report_sha) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("KPI envelope rejected input: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize KPI report: {error}");
            process::exit(2);
        }
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
    let result = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{key}_missing_or_not_number"))?;
    if !result.is_finite() {
        return Err(format!("{key}_must_be_finite"));
    }
    Ok(result)
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{key}_missing_or_not_array"))
}

fn get_distribution(
    value: &Value,
    key: &str,
    expected_count: Option<usize>,
) -> Result<DistributionSummary, String> {
    let distribution: DistributionSummary = serde_json::from_value(
        value
            .get(key)
            .cloned()
            .ok_or_else(|| format!("{key}_missing"))?,
    )
    .map_err(|error| format!("{key}_invalid_distribution:{error}"))?;
    if distribution.sample_count == 0
        || !distribution.mean.is_finite()
        || distribution.minimum > distribution.maximum
        || expected_count.is_some_and(|count| count != distribution.sample_count)
    {
        return Err(format!("{key}_distribution_invariants_failed"));
    }
    Ok(distribution)
}

fn summarize(values: &[f64]) -> Result<DistributionSummary, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("summary_requires_nonempty_finite_values".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    let sd = if count > 1 {
        Some(
            (sorted
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (count - 1) as f64)
                .sqrt(),
        )
    } else {
        None
    };
    let ci = sd.map(|sample_sd| {
        let margin = student_t_critical_95((count - 1) as u32) * sample_sd / (count as f64).sqrt();
        (mean - margin, mean + margin)
    });
    Ok(DistributionSummary {
        sample_count: count,
        mean,
        sample_standard_deviation: sd,
        minimum: sorted[0],
        p05: quantile(&sorted, 0.05),
        median: quantile(&sorted, 0.5),
        p95: quantile(&sorted, 0.95),
        maximum: sorted[count - 1],
        mean_confidence_95_lower: ci.map(|bounds| bounds.0),
        mean_confidence_95_upper: ci.map(|bounds| bounds.1),
    })
}

fn quantile(sorted: &[f64], p: f64) -> f64 {
    let pos = p.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let fraction = pos - lo as f64;
    sorted[lo] * (1.0 - fraction) + sorted[hi] * fraction
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

fn computed_distribution_metric(
    id: &str,
    name: &str,
    formula_id: &str,
    formula: &str,
    unit: &str,
    summary: DistributionSummary,
    reference_window: &str,
    numerator: &str,
    denominator: &str,
    source_fields: Vec<String>,
    interpretation: &str,
) -> KpiMetric {
    KpiMetric {
        kpi_id: id.into(),
        name: name.into(),
        formula_id: formula_id.into(),
        formula_definition: formula.into(),
        unit: unit.into(),
        computability: Computability::Computed,
        value: Some(summary.mean),
        distribution: Some(summary),
        reference_window: reference_window.into(),
        numerator_definition: Some(numerator.into()),
        denominator_definition: Some(denominator.into()),
        source_fields,
        evidence_class: "synthetic".into(),
        unavailable_reason: None,
        interpretation: interpretation.into(),
    }
}

fn computed_scalar_metric(
    id: &str,
    name: &str,
    formula_id: &str,
    formula: &str,
    unit: &str,
    value: f64,
    reference_window: &str,
    source_fields: Vec<String>,
    interpretation: &str,
) -> KpiMetric {
    let summary = DistributionSummary {
        sample_count: 1,
        mean: value,
        sample_standard_deviation: None,
        minimum: value,
        p05: value,
        median: value,
        p95: value,
        maximum: value,
        mean_confidence_95_lower: None,
        mean_confidence_95_upper: None,
    };
    computed_distribution_metric(
        id,
        name,
        formula_id,
        formula,
        unit,
        summary,
        reference_window,
        "Single value from the primary-seed baseline scenario",
        "One primary-seed simulation setting; no replication interval is implied",
        source_fields,
        interpretation,
    )
}

fn not_estimable_metric(
    id: &str,
    name: &str,
    formula_id: &str,
    formula: &str,
    unit: &str,
    reference_window: &str,
    required_fields: Vec<String>,
    reason: &str,
) -> KpiMetric {
    KpiMetric {
        kpi_id: id.into(),
        name: name.into(),
        formula_id: formula_id.into(),
        formula_definition: formula.into(),
        unit: unit.into(),
        computability: Computability::NotEstimable,
        value: None,
        distribution: None,
        reference_window: reference_window.into(),
        numerator_definition: None,
        denominator_definition: None,
        source_fields: required_fields,
        evidence_class: "not_available".into(),
        unavailable_reason: Some(reason.into()),
        interpretation:
            "A null/not-estimable result is intentional; it is not zero and must not be imputed."
                .into(),
    }
}

fn validate_job_lifecycle_evidence(
    baseline: &Value,
    primary_seed: u64,
    expected_released_jobs: u64,
) -> Result<bool, String> {
    let Some(events) = baseline
        .get("job_lifecycle_events")
        .and_then(Value::as_array)
    else {
        return Ok(false);
    };
    let Some(summary) = baseline.get("job_lifecycle_summary") else {
        return Ok(false);
    };
    if events.is_empty() {
        return Ok(false);
    }
    if field_usize(summary, "event_count_in_ledger")? != events.len() {
        return Err("lifecycle_event_count_does_not_match_ledger".into());
    }
    let mut previous_timestamp = 0_u64;
    let mut release_ids = std::collections::BTreeSet::new();
    let mut terminal_counts = std::collections::BTreeMap::<u64, usize>::new();
    let mut previous_sequence: Option<u64> = None;
    for (index, event) in events.iter().enumerate() {
        if field_str(event, "schema_version")? != "0.1.0"
            || field_u64(event, "seed")? != primary_seed
            || field_str(event, "scenario_id")? != "baseline"
            || field_str(event, "evidence_class")? != "synthetic"
        {
            return Err("lifecycle_event_schema_seed_scenario_or_evidence_mismatch".into());
        }
        let sequence = field_u64(event, "event_sequence")?;
        if sequence != index as u64 || previous_sequence.is_some_and(|prior| sequence <= prior) {
            return Err("lifecycle_event_sequence_not_contiguous_and_monotonic".into());
        }
        previous_sequence = Some(sequence);
        let timestamp = field_u64(event, "timestamp_seconds")?;
        if index > 0 && timestamp < previous_timestamp {
            return Err("lifecycle_event_timestamp_regression".into());
        }
        previous_timestamp = timestamp;
        let job_id = field_u64(event, "job_id")?;
        let kind = field_str(event, "kind")?;
        match kind {
            "job_release" => {
                if !release_ids.insert(job_id) {
                    return Err("duplicate_job_release_in_lifecycle_ledger".into());
                }
            }
            "final_good_completion" | "job_scrapped" | "job_censored_at_horizon" => {
                *terminal_counts.entry(job_id).or_default() += 1;
            }
            _ => {}
        }
    }
    if release_ids.len() as u64 != expected_released_jobs
        || terminal_counts.len() != release_ids.len()
        || release_ids
            .iter()
            .any(|job_id| terminal_counts.get(job_id) != Some(&1))
        || terminal_counts
            .keys()
            .any(|job_id| !release_ids.contains(job_id))
    {
        return Err("lifecycle_ledger_release_terminal_coverage_mismatch".into());
    }
    let eligible = field_u64(summary, "eligible_released_jobs_after_warmup")?;
    let good = field_u64(summary, "accepted_good_jobs_in_cohort")?;
    let scrapped = field_u64(summary, "scrapped_jobs_in_cohort")?;
    let censored = field_u64(summary, "right_censored_jobs_at_horizon")?;
    if eligible != good + scrapped + censored {
        return Err("lifecycle_cohort_terminal_counts_do_not_reconcile".into());
    }
    let lead_time = summary.get("good_job_lead_time_seconds");
    match (good, lead_time) {
        (0, Some(value)) if value.is_null() => {}
        (0, None) => {}
        (expected, Some(value)) if expected > 0 => {
            let distribution: DistributionSummary = serde_json::from_value(value.clone())
                .map_err(|error| format!("lifecycle_lead_time_distribution_invalid:{error}"))?;
            if distribution.sample_count as u64 != expected {
                return Err("lifecycle_lead_time_samples_do_not_match_good_jobs".into());
            }
        }
        _ => return Err("lifecycle_lead_time_distribution_presence_mismatch".into()),
    }
    for station in array(summary, "station_first_pass_yield")? {
        let started = field_u64(station, "first_attempts_started_in_cohort")?;
        let observed = field_u64(station, "first_attempts_with_observed_outcome")?;
        let passed = field_u64(station, "first_attempt_passed")?;
        let failed = field_u64(station, "first_attempt_failed")?;
        let unresolved = field_u64(station, "unresolved_first_attempts_at_horizon")?;
        if passed + failed != observed || observed + unresolved != started {
            return Err("first_pass_outcome_counts_do_not_reconcile".into());
        }
        let yield_value = station.get("observed_first_pass_yield_fraction");
        if observed == 0 {
            if yield_value.is_some_and(|value| !value.is_null()) {
                return Err("first_pass_yield_present_without_observed_outcomes".into());
            }
        } else {
            let ratio = yield_value
                .and_then(Value::as_f64)
                .ok_or_else(|| "first_pass_yield_missing_with_observed_outcomes".to_string())?;
            if !ratio.is_finite() || (ratio - passed as f64 / observed as f64).abs() > 1e-9 {
                return Err("first_pass_yield_arithmetic_mismatch".into());
            }
        }
    }
    Ok(true)
}

fn optional_distribution(
    value: &Value,
    key: &str,
    expected_count: Option<usize>,
) -> Result<Option<DistributionSummary>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => get_distribution(value, key, expected_count).map(Some),
    }
}

fn baseline_scenario<'a>(report: &'a Value) -> Result<&'a Value, String> {
    let scenarios = array(report, "scenarios")?;
    scenarios
        .iter()
        .find(|scenario| scenario.get("scenario_id").and_then(Value::as_str) == Some("baseline"))
        .ok_or_else(|| "primary_seed_baseline_scenario_missing".to_string())
}

fn build_report(
    catalog: &Value,
    simulation: &Value,
    catalog_sha: &str,
    report_sha: &str,
) -> Result<KpiReport, String> {
    if field_str(catalog, "schema_version")? != SCHEMA_VERSION
        || field_str(catalog, "benchmark_id")? != "EV-FACTORY-SIM-001"
        || field_str(simulation, "schema_version")? != SCHEMA_VERSION
        || field_str(simulation, "benchmark_id")? != "EV-FACTORY-SIM-001"
    {
        return Err("unsupported_catalog_or_simulation_schema".into());
    }
    if field_str(catalog, "input_evidence_class")? != "synthetic"
        || field_str(simulation, "input_evidence_class")? != "synthetic"
    {
        return Err("KPI_envelope_requires_explicit_synthetic_input".into());
    }
    if field_str(simulation, "source_catalog_sha256")? != catalog_sha {
        return Err("simulation_report_is_not_bound_to_exact_catalog_bytes".into());
    }
    if field_bool(simulation, "release_ready")?
        || !field_bool(simulation, "output_never_exceeds_released_input")?
        || !field_bool(simulation, "all_buffers_respected_capacity")?
        || !field_bool(simulation, "station_time_accounting_within_horizon")?
    {
        return Err("simulation_report_failed_invariants_or_claims_release_ready".into());
    }
    if field_str(simulation, "dispatch_policy")? != field_str(catalog, "dispatch_policy")? {
        return Err("simulation_catalog_dispatch_policy_mismatch".into());
    }
    let seeds: Vec<u64> = array(simulation, "replication_seeds")?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| "replication_seed_not_u64".to_string())
        })
        .collect::<Result<_, _>>()?;
    let catalog_seeds: Vec<u64> = array(catalog, "replication_seeds")?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| "catalog_seed_not_u64".to_string())
        })
        .collect::<Result<_, _>>()?;
    if seeds != catalog_seeds
        || seeds.is_empty()
        || field_usize(simulation, "replication_count")? != seeds.len()
    {
        return Err("catalog_simulation_seed_order_or_count_mismatch".into());
    }
    let replication_count = seeds.len();
    let baseline = baseline_scenario(simulation)?;
    let scenarios = array(simulation, "scenarios")?;
    if scenarios.len() != field_usize(simulation, "scenario_count")? {
        return Err("scenario_count_does_not_match_report_array".into());
    }
    let baseline_outcomes: Vec<f64> = array(simulation, "replication_outcomes")?
        .iter()
        .filter(|row| row.get("scenario_id").and_then(Value::as_str) == Some("baseline"))
        .map(|row| field_f64(row, "good_units_per_hour"))
        .collect::<Result<_, _>>()?;
    if baseline_outcomes.len() != replication_count {
        return Err("baseline_output_samples_do_not_match_replications".into());
    }
    let throughput = summarize(&baseline_outcomes)?;
    let productive_seconds_per_year = field_u64(simulation, "productive_seconds_per_year")?;
    let annualized: Vec<f64> = baseline_outcomes
        .iter()
        .map(|rate| rate * productive_seconds_per_year as f64 / 3600.0)
        .collect();
    let annualized_summary = summarize(&annualized)?;

    let scenario_statistics = array(simulation, "scenario_statistics")?;
    let baseline_stats = scenario_statistics
        .iter()
        .find(|item| item.get("scenario_id").and_then(Value::as_str) == Some("baseline"))
        .ok_or_else(|| "baseline_scenario_statistics_missing".to_string())?;
    let scrap_distribution = get_distribution(
        baseline_stats,
        "scrapped_jobs_total",
        Some(replication_count),
    )?;
    let rework_distribution = get_distribution(
        baseline_stats,
        "measured_rework_attempts",
        Some(replication_count),
    )?;

    let mut metrics = Vec::new();
    metrics.push(computed_distribution_metric(
        "good_units_per_hour",
        "Good units per hour",
        "throughput.good-units-per-hour.v1",
        "For each replication, measured good final units divided by the post-warm-up measurement seconds, multiplied by 3600; the reported distribution summarizes the 20 per-seed values.",
        "good_units/hour",
        throughput,
        "Measured horizon after warm-up; 20 synthetic replications",
        "Good final units accepted after warm-up",
        "Post-warm-up measured seconds / 3600 seconds per hour",
        vec![
            "replication_outcomes[scenario_id=baseline].good_units_per_hour".into(),
            "measured_horizon_seconds".into(),
            "replication_seeds".into(),
        ],
        "Conditional simulation output rate under the exact synthetic catalog and seed list; not a plant forecast.",
    ));
    metrics.push(computed_distribution_metric(
        "annualized_good_units_at_declared_hours",
        "Linearly annualized good units at declared operating hours",
        "throughput.linearized-annualization.v1",
        "Good units/hour for each seed multiplied by productive_seconds_per_year / 3600; assumes the measured rate persists for the declared annual productive seconds.",
        "good_units/year",
        annualized_summary,
        "Linearized projection from the post-warm-up measured rate",
        "Good units/hour × declared productive seconds/year",
        "3600 seconds/hour",
        vec![
            "replication_outcomes[scenario_id=baseline].good_units_per_hour".into(),
            "productive_seconds_per_year".into(),
        ],
        "A transparent sensitivity projection only; not capacity assurance, a sales plan, or a demand forecast.",
    ));
    metrics.push(computed_distribution_metric(
        "scrapped_jobs_total_full_horizon",
        "Scrapped jobs over full simulation horizon",
        "quality.scrap-count.v1",
        "Per-replication total scrapped jobs over the full simulated horizon, including warm-up.",
        "jobs",
        scrap_distribution,
        "Full simulation horizon including warm-up; 20 synthetic replications",
        "Scrapped jobs counted by the simulator",
        "One full simulated horizon per replication",
        vec!["scenario_statistics[scenario_id=baseline].scrapped_jobs_total".into()],
        "Count only; not a standard quality yield ratio, and counts include warm-up events.",
    ));
    metrics.push(computed_distribution_metric(
        "rework_attempts_measured_horizon",
        "Rework attempts after warm-up",
        "quality.rework-count.v1",
        "Per-replication rework attempts that start at or after the warm-up boundary.",
        "attempts",
        rework_distribution,
        "Measured horizon after warm-up; 20 synthetic replications",
        "Rework attempts recorded by the simulator after warm-up",
        "One measured horizon per replication",
        vec!["scenario_statistics[scenario_id=baseline].measured_rework_attempts".into()],
        "Count only; it is not a first-pass-yield measure and does not define an industry quality KPI by itself.",
    ));

    let station_rows = array(baseline, "stations")?;
    let baseline_station_count = array(catalog, "stations")?.len();
    if station_rows.len() != baseline_station_count {
        return Err("primary_seed_station_metrics_do_not_match_catalog_station_count".into());
    }
    for station in station_rows {
        let station_id = field_str(station, "station_id")?;
        for (suffix, name, field, definition) in [
            (
                "busy_fraction",
                "Busy fraction of server-seconds",
                "busy_fraction",
                "Busy server-seconds / total measured server-seconds",
            ),
            (
                "blocked_fraction",
                "Blocked fraction of server-seconds",
                "blocked_fraction",
                "Server-seconds waiting to transfer completed output / total measured server-seconds",
            ),
            (
                "starved_fraction",
                "Starved fraction of server-seconds",
                "starved_fraction",
                "Idle server-seconds while the immediately upstream input queue is empty / total measured server-seconds",
            ),
            (
                "down_fraction",
                "Down fraction of server-seconds",
                "down_fraction",
                "Server-seconds in the simulator's down state / total measured server-seconds",
            ),
        ] {
            let val = field_f64(station, field)?;
            if !(0.0..=1.0).contains(&val) {
                return Err(format!(
                    "station_{station_id}_{field}_outside_fraction_range"
                ));
            }
            metrics.push(computed_scalar_metric(
                &format!("station.{station_id}.{suffix}"),
                &format!("{name}: {station_id}"),
                &format!("equipment-state.{suffix}.v1"),
                definition,
                "fraction of server-seconds",
                val,
                "Primary-seed baseline scenario; measured horizon after warm-up",
                vec![
                    format!("scenarios[scenario_id=baseline].stations[station_id={station_id}].{field}"),
                    "measured_horizon_seconds".into(),
                    format!("scenarios[scenario_id=baseline].stations[station_id={station_id}].server_count"),
                ],
                "Simulator state ratio for one primary seed. This is not ISO 22400 equipment availability or OEE.",
            ));
        }
    }

    for key in [
        "supplier_backlog_average_units",
        "source_buffer_average_units",
    ] {
        let value = field_f64(baseline, key)?;
        if value < 0.0 {
            return Err(format!("{key}_must_be_nonnegative"));
        }
        metrics.push(computed_scalar_metric(
            key,
            if key == "supplier_backlog_average_units" {
                "Average supplier backlog"
            } else {
                "Average source-buffer occupancy"
            },
            &format!("logistics.{key}.v1"),
            "WIP-area integral over the post-warm-up measurement window divided by measured seconds.",
            "units",
            value,
            "Primary-seed baseline scenario; measured horizon after warm-up",
            vec![format!("scenarios[scenario_id=baseline].{key}"), "measured_horizon_seconds".into()],
            "Time-average inventory/queue level from one primary-seed run; the report currently does not aggregate this metric over all seeds.",
        ));
    }

    for buffer in array(baseline, "interstation_buffers")? {
        let after = field_str(buffer, "after_station_id")?;
        let occupancy = field_f64(buffer, "average_occupancy_units")?;
        let capacity = field_usize(buffer, "capacity_units")?;
        if occupancy < 0.0 || occupancy > capacity as f64 + 1e-9 {
            return Err(format!("buffer_after_{after}_average_occupancy_invalid"));
        }
        metrics.push(computed_scalar_metric(
            &format!("buffer.after_{after}.average_occupancy"),
            &format!("Average occupancy after {after}"),
            "logistics.average-buffer-occupancy.v1",
            "Buffer WIP-area integral over the measured window divided by measured seconds.",
            "units",
            occupancy,
            "Primary-seed baseline scenario; measured horizon after warm-up",
            vec![
                format!("scenarios[scenario_id=baseline].interstation_buffers[after_station_id={after}].average_occupancy_units"),
                format!("scenarios[scenario_id=baseline].interstation_buffers[after_station_id={after}].capacity_units"),
            ],
            "Time-average WIP in a finite interstation buffer, primary seed only.",
        ));
    }

    for station in array(simulation, "station_sequence_statistics")?
        .iter()
        .filter(|item| item.get("scenario_id").and_then(Value::as_str) == Some("baseline"))
    {
        let station_id = field_str(station, "station_id")?;
        for (suffix, name, field, unit, formula) in [
            (
                "changeovers_started",
                "Changeovers started",
                "changeovers_started",
                "changeovers",
                "Count of starts that incur a variant changeover per replication.",
            ),
            (
                "output_transfers",
                "Output transfers",
                "output_transfers",
                "transfers",
                "Count of accepted output transfers during the measured window per replication.",
            ),
            (
                "out_of_order_output_transfers",
                "Out-of-order output transfers (job-ID proxy)",
                "out_of_order_output_transfers",
                "transfers",
                "Count of transfers where job ID is below the station's prior transfer high-water ID.",
            ),
        ] {
            let summary = get_distribution(station, field, Some(replication_count))?;
            metrics.push(computed_distribution_metric(
                &format!("station.{station_id}.{suffix}"),
                &format!("{name}: {station_id}"),
                &format!("flow.{suffix}.v1"),
                formula,
                unit,
                summary,
                "Measured horizon after warm-up; 20 synthetic replications",
                formula,
                "One post-warm-up measured horizon per replication",
                vec![
                    format!("station_sequence_statistics[scenario_id=baseline,station_id={station_id}].{field}"),
                    "replication_seeds".into(),
                ],
                "Synthetic diagnostic counter; the job-ID inversion metric is a coarse sequence-order proxy, not a delivery-order/tardiness KPI.",
            ));
        }
    }

    let lifecycle_available = validate_job_lifecycle_evidence(
        baseline,
        field_u64(simulation, "seed")?,
        field_u64(baseline, "released_jobs_total")?,
    )?;
    append_lifecycle_kpis(
        &mut metrics,
        catalog,
        baseline,
        baseline_stats,
        replication_count,
        lifecycle_available,
    )?;

    metrics.extend(not_estimable_metrics());
    let crosswalk = vec![
        CrosswalkItem {
            metric_family: "Throughput and output quantity".into(),
            informative_iso_22400_concept: "Manufacturing production quantity / rate indicators".into(),
            implementation_note: "Our good-units/hour metric uses simulation post-warm-up time and a final-good-unit count; validate production-order scope and reference-time semantics before mapping to a plant KPI.".into(),
            conformance_claim: false,
        },
        CrosswalkItem {
            metric_family: "Equipment state fractions".into(),
            informative_iso_22400_concept: "Equipment and production-time indicators".into(),
            implementation_note: "Busy/blocked/starved/down ratios divide by all available simulated server-seconds and do not substitute for planned-time availability or OEE.".into(),
            conformance_claim: false,
        },
        CrosswalkItem {
            metric_family: "Buffers and backlog".into(),
            informative_iso_22400_concept: "Logistical / inventory indicators".into(),
            implementation_note: "Queue WIP is available for selected simulated buffers, but the present report only exposes primary-seed buffer time averages; it is not a complete inventory valuation.".into(),
            conformance_claim: false,
        },
        CrosswalkItem {
            metric_family: "Quality and maintenance".into(),
            informative_iso_22400_concept: "Quality, scrap/rework and maintenance indicators".into(),
            implementation_note: "Per-station observed first-pass yield now uses unique job first attempts with observed outcomes, and unresolved first attempts are counted separately. MTBF/MTTR observed distributions and schedule-adjusted availability remain not-estimable without further event/calendar semantics.".into(),
            conformance_claim: false,
        },
        CrosswalkItem {
            metric_family: "Energy, labor and economics".into(),
            informative_iso_22400_concept: "Energy and organizational/economic indicators".into(),
            implementation_note: "The DES does not currently ingest meter readings, paid labor hours, audited costs or price/FX evidence; missing values must not become zeros.".into(),
            conformance_claim: false,
        },
    ];
    Ok(KpiReport {
        schema_version: SCHEMA_VERSION.into(),
        benchmark_id: BENCHMARK_ID.into(),
        source_catalog_sha256: catalog_sha.into(),
        source_simulation_report_sha256: report_sha.into(),
        source_evidence_class: "synthetic".into(),
        replication_count,
        replication_seeds: seeds,
        dispatch_policy: field_str(simulation, "dispatch_policy")?.into(),
        metrics,
        iso_22400_informative_crosswalk: crosswalk,
        model_safety_quality_compliance_release_authorized: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn append_lifecycle_kpis(
    metrics: &mut Vec<KpiMetric>,
    catalog: &Value,
    baseline: &Value,
    baseline_stats: &Value,
    replication_count: usize,
    lifecycle_available: bool,
) -> Result<(), String> {
    let cohort = "Jobs released at or after warm-up; lead time ends at accepted final-good transfer; nonterminal jobs are right-censored at the simulation horizon.";
    let lifecycle_fields = vec![
        "scenarios[scenario_id=baseline].job_lifecycle_summary".into(),
        "scenario_statistics[scenario_id=baseline]".into(),
        "replication_seeds".into(),
    ];
    if !lifecycle_available {
        metrics.push(not_estimable_metric(
            "p95_end_to_end_lead_time",
            "P95 end-to-end good-job lead time",
            "flow.lead-time.p95-by-seed.v1",
            "For each replication, calculate P95(final-good acceptance timestamp minus release timestamp) over good jobs in the eligible cohort; summarize per-seed P95 values.",
            "seconds/job",
            cohort,
            lifecycle_fields.clone(),
            "Lifecycle event ledger or per-seed lifecycle summaries are missing or failed validation; no percentile is inferred from aggregate throughput.",
        ));
        for station in array(catalog, "stations")? {
            let station_id = field_str(station, "station_id")?;
            metrics.push(not_estimable_metric(
                &format!("station.{station_id}.first_pass_yield"),
                &format!("Observed first-pass yield: {station_id}"),
                "quality.first-pass-yield.observed.v1",
                "Unique jobs passing the first attempt / unique jobs with an observed first-attempt quality outcome, where the first attempt began at or after warm-up.",
                "fraction",
                cohort,
                lifecycle_fields.clone(),
                "The traceable first-attempt outcome cohort is unavailable; missing data is not treated as a failure, pass or zero.",
            ));
        }
        return Ok(());
    }

    let p50 = optional_distribution(
        baseline_stats,
        "per_seed_median_good_job_lead_time_seconds",
        None,
    )?;
    let p95 = optional_distribution(
        baseline_stats,
        "per_seed_p95_good_job_lead_time_seconds",
        None,
    )?;
    for (metric_id, name, formula_id, formula, distribution) in [
        (
            "median_good_job_lead_time_seconds",
            "Mean across replications of the per-seed median good-job lead time",
            "flow.lead-time.median-by-seed.v1",
            "For each seed, take the median accepted-good-job lead time in the eligible release cohort; summarize the per-seed medians.",
            p50,
        ),
        (
            "p95_end_to_end_lead_time",
            "Mean across replications of per-seed P95 good-job lead time",
            "flow.lead-time.p95-by-seed.v1",
            "For each seed, take P95 of final-good transfer timestamp minus release timestamp for eligible good jobs; summarize the per-seed P95 values. Right-censored jobs are reported separately and are not assigned invented completion times.",
            p95,
        ),
    ] {
        match distribution {
            Some(summary) => {
                if summary.sample_count > replication_count {
                    return Err(format!("{metric_id}_sample_count_exceeds_replication_count"));
                }
                metrics.push(computed_distribution_metric(
                    metric_id,
                    name,
                    formula_id,
                    formula,
                    "seconds/job",
                    summary,
                    "Eligible release cohort, per-seed lead-time summary after warm-up",
                    "Final-good acceptance timestamp minus job release timestamp",
                    "Seconds between timestamps for an accepted good job; right-censored jobs excluded and separately counted",
                    vec![
                        "scenario_statistics[scenario_id=baseline].per_seed_median_good_job_lead_time_seconds".into(),
                        "scenario_statistics[scenario_id=baseline].per_seed_p95_good_job_lead_time_seconds".into(),
                        "scenarios[scenario_id=baseline].job_lifecycle_events".into(),
                    ],
                    "Model-conditional summary; per-seed quantile summaries are not a pooled-job quantile and do not validate real plant performance.",
                ));
            }
            None => metrics.push(not_estimable_metric(
                metric_id,
                name,
                formula_id,
                formula,
                "seconds/job",
                cohort,
                lifecycle_fields.clone(),
                "No replication had an observed accepted-good-job lead-time sample for this statistic.",
            )),
        }
    }

    let right_censored = get_distribution(
        baseline_stats,
        "right_censored_jobs_at_horizon",
        Some(replication_count),
    )?;
    metrics.push(computed_distribution_metric(
        "right_censored_jobs_at_horizon",
        "Right-censored jobs at simulation horizon",
        "flow.right-censored-count.v1",
        "Count of eligible released jobs not accepted as final good or scrapped by the simulation horizon, per seed.",
        "jobs",
        right_censored,
        "Eligible job-release cohort at or after warm-up; horizon-end state",
        "Eligible released jobs that remain nonterminal at the horizon",
        "One simulation horizon per seed",
        vec![
            "scenario_statistics[scenario_id=baseline].right_censored_jobs_at_horizon".into(),
            "replication_seeds".into(),
        ],
        "Censoring is disclosed rather than silently dropping open jobs or assigning fake completion times.",
    ));

    let station_rows = array(baseline_stats, "station_first_pass_yield_by_station")?;
    let mut seen_stations = std::collections::BTreeSet::new();
    for station_summary in station_rows {
        let station_id = field_str(station_summary, "station_id")?;
        if !seen_stations.insert(station_id.to_string()) {
            return Err("duplicate_station_first_pass_summary".into());
        }
        for (suffix, name, field) in [
            (
                "first_attempts_started_in_cohort",
                "First attempts started in eligible cohort",
                "first_attempts_started_in_cohort",
            ),
            (
                "first_attempts_with_observed_outcome",
                "Observed first-attempt outcomes",
                "observed_first_attempts",
            ),
            (
                "first_attempt_passed",
                "First attempts passed",
                "first_attempt_passed",
            ),
            (
                "first_attempt_failed",
                "First attempts failed",
                "first_attempt_failed",
            ),
            (
                "unresolved_first_attempts_at_horizon",
                "Unresolved first attempts at horizon",
                "unresolved_first_attempts_at_horizon",
            ),
        ] {
            let distribution = get_distribution(station_summary, field, Some(replication_count))?;
            metrics.push(computed_distribution_metric(
                &format!("station.{station_id}.{suffix}"),
                &format!("{name}: {station_id}"),
                &format!("quality.first-attempt-count.{suffix}.v1"),
                "Per seed, count unique job first attempts whose first operation start at this station occurred at or after warm-up; report observed outcomes and unresolved attempts separately.",
                "jobs",
                distribution,
                cohort,
                "Unique job/station first-attempt record",
                "One seed simulation",
                vec![
                    format!("scenario_statistics[scenario_id=baseline].station_first_pass_yield_by_station[station_id={station_id}].{field}"),
                    "replication_seeds".into(),
                ],
                "Synthetic cohort count; not a plant quality result.",
            ));
        }
        if let Some(yield_summary) =
            optional_distribution(station_summary, "observed_first_pass_yield_fraction", None)?
        {
            if yield_summary.sample_count > replication_count {
                return Err(format!(
                    "station_{station_id}_first_pass_yield_sample_count_exceeds_replications"
                ));
            }
            metrics.push(computed_distribution_metric(
                &format!("station.{station_id}.first_pass_yield"),
                &format!("Observed first-pass yield: {station_id}"),
                "quality.first-pass-yield.observed.v1",
                "For each seed, unique jobs that pass their first attempt at this station divided by unique jobs whose first-attempt outcome was observed, where the attempt began at or after warm-up. Unresolved first attempts are counted separately.",
                "fraction",
                yield_summary,
                cohort,
                "Observed first-attempt passes",
                "Observed first-attempt pass plus fail outcomes for unique jobs",
                vec![
                    format!("scenario_statistics[scenario_id=baseline].station_first_pass_yield_by_station[station_id={station_id}].observed_first_pass_yield_fraction"),
                    format!("scenario_statistics[scenario_id=baseline].station_first_pass_yield_by_station[station_id={station_id}].unresolved_first_attempts_at_horizon"),
                    "replication_seeds".into(),
                ],
                "This is a seed-level distribution of observed first-pass fractions. It is not final yield; jobs with unresolved first-attempt outcomes are excluded from the fraction and disclosed separately.",
            ));
        } else {
            metrics.push(not_estimable_metric(
                &format!("station.{station_id}.first_pass_yield"),
                &format!("Observed first-pass yield: {station_id}"),
                "quality.first-pass-yield.observed.v1",
                "Unique jobs passing the first attempt divided by unique jobs with an observed first-attempt quality outcome.",
                "fraction",
                cohort,
                vec![
                    format!("scenario_statistics[scenario_id=baseline].station_first_pass_yield_by_station[station_id={station_id}]"),
                    "replication_seeds".into(),
                ],
                "No first-attempt outcome was observed in any replication for this station.",
            ));
        }
    }
    if seen_stations.len() != array(catalog, "stations")?.len() {
        return Err("first_pass_summaries_do_not_cover_all_catalog_stations".into());
    }
    Ok(())
}

fn not_estimable_metrics() -> Vec<KpiMetric> {
    vec![
        not_estimable_metric(
            "overall_equipment_effectiveness",
            "Overall equipment effectiveness (OEE)",
            "oee.availability-performance-quality.v1",
            "Availability × performance efficiency × quality ratio using a common defined reference-time model.",
            "fraction",
            "Plant-shift or production-order reference time",
            vec![
                "planned_production_time".into(),
                "ideal_cycle_time_by_product_and_station".into(),
                "total_units_processed_in_common_window".into(),
                "good_units_processed_in_common_window".into(),
            ],
            "Planned production-time denominator, approved ideal cycle-time reference and common-window total/good counts are not jointly supplied by this simulator report.",
        ),
        not_estimable_metric(
            "scheduled_time_equipment_availability",
            "Scheduled-time equipment availability",
            "equipment.availability.planned-time.v1",
            "Operating time / planned production time under a declared calendar and maintenance inclusion rule.",
            "fraction",
            "Shift or production-order planned time",
            vec![
                "planned_production_time".into(),
                "equipment_operating_time".into(),
                "planned_maintenance_calendar".into(),
            ],
            "The simulator provides state fractions over a synthetic measurement window, but no planned-time calendar or declared treatment of planned maintenance.",
        ),
        not_estimable_metric(
            "performance_efficiency_against_ideal_cycle",
            "Performance efficiency against ideal cycle time",
            "equipment.performance.ideal-cycle.v1",
            "Ideal cycle time × total units / operating time, with product mix and unit scope declared.",
            "fraction",
            "Station and product-family reference window",
            vec![
                "approved_ideal_cycle_seconds_by_station_and_variant".into(),
                "total_units_completed_by_station_and_variant".into(),
                "operating_time".into(),
            ],
            "Synthetic mean cycle inputs are model parameters, not independently approved ideal-cycle standards, and complete product-family counts are not reported.",
        ),
        not_estimable_metric(
            "energy_per_good_unit",
            "Energy intensity per good unit",
            "energy.kwh-per-good-unit.v1",
            "Metered electricity and declared thermal/fuel energy consumed in the production boundary / accepted good units in the same window.",
            "kWh-equivalent/good unit",
            "Metering interval aligned to measured production window",
            vec![
                "metered_electricity_kwh".into(),
                "metered_thermal_energy_kwh_equivalent".into(),
                "good_units_in_metering_window".into(),
            ],
            "No measured energy meters or operation-level energy model is included in this DES report.",
        ),
        not_estimable_metric(
            "labor_productivity",
            "Good units per paid labor hour",
            "labor.good-units-per-paid-hour.v1",
            "Accepted good units / paid direct and indirect labor hours, with scope declared.",
            "good units/paid labor hour",
            "Shift, work order or production-day window",
            vec![
                "accepted_good_units".into(),
                "paid_labor_hours_by_role".into(),
                "labor_scope_definition".into(),
            ],
            "Staffing, shift attendance, indirect labor and paid hours are absent from the simulator.",
        ),
        not_estimable_metric(
            "audited_cost_per_good_unit",
            "Audited cost per good unit",
            "economics.audited-cost-per-good-unit.v1",
            "Audited labor, energy, consumable, scrap, maintenance and overhead cost allocated to the declared production boundary / good units.",
            "currency/good unit",
            "Declared production-cost accounting period",
            vec![
                "audited_labor_cost".into(),
                "audited_energy_cost".into(),
                "audited_material_and_scrap_cost".into(),
                "audited_overhead_and_maintenance_cost".into(),
                "good_units_in_same_cost_period".into(),
            ],
            "No audited factory cost data, tariff/FX evidence, or accounting allocation policy is supplied.",
        ),
        not_estimable_metric(
            "on_time_delivery_rate",
            "On-time delivery rate",
            "delivery.on-time-rate.v1",
            "Completed customer orders on or before promised due time / all orders due within the same cohort.",
            "fraction",
            "Customer-order due-date cohort",
            vec![
                "promised_due_timestamp".into(),
                "customer_order_id".into(),
                "order_completion_timestamp".into(),
                "due_order_cohort".into(),
            ],
            "Customer due dates, order identifiers and delivery promises are not modeled.",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_has_units_of_observation_and_no_single_sample_ci() {
        let multi = summarize(&[1.0, 2.0, 3.0]).unwrap();
        assert!((multi.mean - 2.0).abs() < 1e-12);
        assert_eq!(multi.sample_standard_deviation, Some(1.0));
        assert!(multi.mean_confidence_95_lower.is_some());

        let single = summarize(&[4.0]).unwrap();
        assert_eq!(single.sample_count, 1);
        assert!(single.mean_confidence_95_lower.is_none());
        assert!(single.mean_confidence_95_upper.is_none());
    }

    #[test]
    fn unavailable_oee_and_energy_are_null_not_zero() {
        let metrics = not_estimable_metrics();
        let oee = metrics
            .iter()
            .find(|metric| metric.kpi_id == "overall_equipment_effectiveness")
            .unwrap();
        let energy = metrics
            .iter()
            .find(|metric| metric.kpi_id == "energy_per_good_unit")
            .unwrap();
        assert_eq!(oee.computability, Computability::NotEstimable);
        assert_eq!(oee.value, None);
        assert_eq!(oee.distribution, None);
        assert_eq!(energy.computability, Computability::NotEstimable);
        assert_eq!(energy.value, None);
    }

    #[test]
    fn report_digests_are_sha256_shape() {
        assert_eq!(sha256(b"abc").len(), 64);
    }
}
