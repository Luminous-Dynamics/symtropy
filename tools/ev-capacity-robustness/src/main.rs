use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const EXPECTED_SUITE: &str = "EV-CHARGE-RESILIENCE-001";
const SERVICE_TARGET: f64 = 0.95;
const TARGET_CAPACITY_COVERAGE: f64 = 0.95;
const POINT_COUNT: usize = 20;
const EPSILON: f64 = 1.0e-6;

#[derive(Debug, Clone)]
struct Options {
    runs_manifest: PathBuf,
    output_json: PathBuf,
    output_csv: PathBuf,
    expected_seeds: usize,
    minimum_fleet_sizes: usize,
    minimum_vocations: usize,
    coverage_target: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Coordinate {
    vocation: String,
    fleet_size: usize,
    seed: u64,
}

#[derive(Debug, Deserialize)]
struct SimulationReport {
    schema_version: String,
    suite_id: String,
    claim_ceiling: String,
    results: Vec<PolicyResult>,
    capacity_frontiers: Vec<Frontier>,
}

#[derive(Debug, Deserialize)]
struct PolicyResult {
    scenario_id: String,
    evidence_class: String,
    evidence_reference: String,
    policy: String,
    job_count: usize,
    completed_by_deadline_count: usize,
    missed_deadline_count: usize,
    requested_battery_energy_kwh: f64,
    delivered_battery_energy_kwh: f64,
    unserved_battery_energy_kwh: f64,
    energy_balance_residual_kwh: f64,
    energy_service_fraction: f64,
    median_queue_wait_minutes: f64,
    p95_queue_wait_minutes: f64,
    maximum_site_input_power_kw: f64,
    site_power_limit_respected: bool,
    individual_power_limits_respected: bool,
    jobs: Vec<JobResult>,
}

#[derive(Debug, Deserialize)]
struct JobResult {
    job_id: String,
    requested_battery_energy_kwh: f64,
}

#[derive(Debug, Deserialize)]
struct Frontier {
    scenario_id: String,
    evidence_class: String,
    policy: String,
    base_site_power_budget_kw: f64,
    minimum_deadline_completion_target: f64,
    minimum_energy_service_target: f64,
    sample_spacing_percent: u32,
    points: Vec<CapacityPoint>,
}

#[derive(Debug, Deserialize)]
struct CapacityPoint {
    installed_site_power_budget_kw: f64,
    fraction_of_declared_base_capacity: f64,
    completed_by_deadline_count: usize,
    missed_deadline_count: usize,
    deadline_completion_fraction: f64,
    energy_service_fraction: f64,
    delivered_battery_energy_kwh: f64,
    unserved_battery_energy_kwh: f64,
    maximum_site_input_power_kw: f64,
    site_power_limit_respected: bool,
    individual_power_limits_respected: bool,
    meets_service_target: bool,
}

#[derive(Debug, Clone)]
struct Run {
    coordinate: Coordinate,
    report_sha256: String,
    source_evidence_reference: String,
    source_report: SimulationReportOwned,
}

#[derive(Debug, Clone)]
struct SimulationReportOwned {
    results: Vec<PolicyResultOwned>,
    frontiers: Vec<FrontierOwned>,
}

#[derive(Debug, Clone)]
struct PolicyResultOwned {
    policy: String,
    job_count: usize,
    completed_by_deadline_count: usize,
    missed_deadline_count: usize,
    requested_battery_energy_kwh: f64,
    delivered_battery_energy_kwh: f64,
    unserved_battery_energy_kwh: f64,
    energy_balance_residual_kwh: f64,
    energy_service_fraction: f64,
    median_queue_wait_minutes: f64,
    p95_queue_wait_minutes: f64,
    maximum_site_input_power_kw: f64,
}

#[derive(Debug, Clone)]
struct FrontierOwned {
    policy: String,
    base_site_power_budget_kw: f64,
    points: Vec<CapacityPointOwned>,
}

#[derive(Debug, Clone)]
struct CapacityPointOwned {
    installed_site_power_budget_kw: f64,
    fraction_of_declared_base_capacity: f64,
    completed_by_deadline_count: usize,
    missed_deadline_count: usize,
    deadline_completion_fraction: f64,
    energy_service_fraction: f64,
    delivered_battery_energy_kwh: f64,
    unserved_battery_energy_kwh: f64,
    maximum_site_input_power_kw: f64,
    meets_service_target: bool,
}

#[derive(Debug, Serialize)]
struct Distribution {
    count: usize,
    min: Option<f64>,
    p05: Option<f64>,
    p50: Option<f64>,
    p95: Option<f64>,
    max: Option<f64>,
    mean: Option<f64>,
}

#[derive(Debug, Serialize)]
struct CapacityPointAggregate {
    capacity_fraction_of_port_nameplate: f64,
    installed_site_power_budget_kw: f64,
    installed_site_power_kw_per_vehicle: f64,
    replicate_count: usize,
    replications_meeting_service_target: usize,
    empirical_feasibility_fraction: f64,
    coverage_target: f64,
    robust_target_met: bool,
    wilson_95_percent_interval: [f64; 2],
    deadline_completion_fraction: Distribution,
    energy_service_fraction: Distribution,
    delivered_battery_energy_kwh: Distribution,
    unserved_battery_energy_kwh: Distribution,
    maximum_site_input_power_kw: Distribution,
    failed_seed_ids: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct PolicyAggregate {
    vocation: String,
    fleet_size: usize,
    policy: String,
    replicate_count: usize,
    jobs_per_replicate: usize,
    port_count: usize,
    port_to_vehicle_ratio: f64,
    declared_port_nameplate_site_power_kw: f64,
    coverage_target: f64,
    robust_minimum_sampled_site_power_kw: Option<f64>,
    robust_minimum_sampled_site_power_kw_per_vehicle: Option<f64>,
    sampled_capacity_spacing_percent: u32,
    deadline_completion_fraction: Distribution,
    energy_service_fraction: Distribution,
    missed_deadline_jobs_total: usize,
    unserved_battery_energy_kwh: Distribution,
    median_queue_wait_minutes: Distribution,
    p95_queue_wait_minutes: Distribution,
    maximum_site_input_power_kw: Distribution,
    capacity_points: Vec<CapacityPointAggregate>,
}

#[derive(Debug, Serialize)]
struct RunReceipt {
    vocation: String,
    fleet_size: usize,
    seed: u64,
    report_sha256: String,
    source_evidence_reference: String,
}

#[derive(Debug, Serialize)]
struct RobustReport {
    schema_version: String,
    tool: String,
    benchmark_id: String,
    expected_seeds_per_vocation_size: usize,
    fleet_sizes: Vec<usize>,
    vocations: Vec<String>,
    policies: Vec<String>,
    empirical_capacity_coverage_target: f64,
    per_run_service_targets: ServiceTargets,
    caveat: String,
    run_manifest_sha256: String,
    validated_run_count: usize,
    run_receipts: Vec<RunReceipt>,
    policy_aggregates: Vec<PolicyAggregate>,
    claim_ceiling: String,
}

#[derive(Debug, Serialize)]
struct ServiceTargets {
    minimum_deadline_completion_fraction: f64,
    minimum_energy_service_fraction: f64,
    all_modeled_power_limits_respected: bool,
}

#[derive(Debug)]
struct RunManifestEntry {
    coordinate: Coordinate,
    report_path: PathBuf,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn finite_in_range(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && value >= min - EPSILON && value <= max + EPSILON
}

fn distribution(values: &[f64]) -> Distribution {
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let count = values.len();
    let rank = |percent: usize| -> Option<f64> {
        if count == 0 {
            None
        } else {
            let index = (percent * count).div_ceil(100).saturating_sub(1);
            values.get(index).copied()
        }
    };
    let p50 = if count == 0 {
        None
    } else if count % 2 == 1 {
        Some(values[count / 2])
    } else {
        Some((values[count / 2 - 1] + values[count / 2]) / 2.0)
    };
    Distribution {
        count,
        min: values.first().copied(),
        p05: rank(5),
        p50,
        p95: rank(95),
        max: values.last().copied(),
        mean: if count == 0 {
            None
        } else {
            Some(values.iter().sum::<f64>() / count as f64)
        },
    }
}

fn wilson_interval(successes: usize, n: usize) -> [f64; 2] {
    if n == 0 {
        return [0.0, 0.0];
    }
    let z = 1.959_963_984_540_054;
    let p = successes as f64 / n as f64;
    let z2 = z * z;
    let denom = 1.0 + z2 / n as f64;
    let center = (p + z2 / (2.0 * n as f64)) / denom;
    let half = z * ((p * (1.0 - p) / n as f64 + z2 / (4.0 * (n as f64).powi(2))).sqrt()) / denom;
    [(center - half).max(0.0), (center + half).min(1.0)]
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    if args.len() < 2 {
        return Err("usage: ev-capacity-robustness --runs-manifest FILE --output-json FILE --output-csv FILE --expected-seeds N --minimum-fleet-sizes N --minimum-vocations N --coverage-target FRACTION".to_string());
    }
    let valid = [
        "--runs-manifest",
        "--output-json",
        "--output-csv",
        "--expected-seeds",
        "--minimum-fleet-sizes",
        "--minimum-vocations",
        "--coverage-target",
    ];
    let mut seen = BTreeSet::new();
    let mut values = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let key = args[i].as_str();
        if !valid.contains(&key) {
            return Err(format!("unknown_argument:{key}"));
        }
        if !seen.insert(key.to_string()) {
            return Err(format!("duplicate_argument:{key}"));
        }
        let value = args.get(i + 1).ok_or_else(|| format!("{key}_requires_a_value"))?;
        values.insert(key.to_string(), value.to_string());
        i += 2;
    }
    for key in valid {
        if !values.contains_key(key) {
            return Err(format!("{key}_required"));
        }
    }
    let parse_usize = |key: &str| -> Result<usize, String> {
        values[key]
            .parse::<usize>()
            .map_err(|_| format!("{key}_must_be_a_positive_integer"))
    };
    let expected_seeds = parse_usize("--expected-seeds")?;
    let minimum_fleet_sizes = parse_usize("--minimum-fleet-sizes")?;
    let minimum_vocations = parse_usize("--minimum-vocations")?;
    if expected_seeds < 2 || minimum_fleet_sizes < 2 || minimum_vocations < 1 {
        return Err("expected_seeds_must_be_at_least_2_fleet_sizes_at_least_2_and_vocations_at_least_1".into());
    }
    let coverage_target: f64 = values["--coverage-target"]
        .parse()
        .map_err(|_| "--coverage-target_must_be_numeric")?;
    if !coverage_target.is_finite() || !(0.5..=1.0).contains(&coverage_target) {
        return Err("--coverage-target_must_be_in_[0.5,1]".to_string());
    }
    let output_json = PathBuf::from(&values["--output-json"]);
    let output_csv = PathBuf::from(&values["--output-csv"]);
    if output_json == output_csv {
        return Err("output_json_and_output_csv_must_differ".into());
    }
    Ok(Options {
        runs_manifest: PathBuf::from(&values["--runs-manifest"]),
        output_json,
        output_csv,
        expected_seeds,
        minimum_fleet_sizes,
        minimum_vocations,
        coverage_target,
    })
}

fn parse_run_manifest(text: &str) -> Result<Vec<RunManifestEntry>, String> {
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for (line_index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if entries.is_empty() && line.starts_with("fleet_size\tseed\tvocation\treport_path") {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 4 {
            return Err(format!("run_manifest_line_{}_must_have_4_tab_fields", line_index + 1));
        }
        let fleet_size = fields[0]
            .parse::<usize>()
            .map_err(|_| format!("run_manifest_line_{}_fleet_size_invalid", line_index + 1))?;
        let seed = fields[1]
            .parse::<u64>()
            .map_err(|_| format!("run_manifest_line_{}_seed_invalid", line_index + 1))?;
        let vocation = fields[2].trim().to_string();
        let report_path = PathBuf::from(fields[3].trim());
        if fleet_size == 0 || vocation.is_empty() || report_path.as_os_str().is_empty() {
            return Err(format!("run_manifest_line_{}_has_empty_or_zero_coordinate", line_index + 1));
        }
        let coordinate = Coordinate {
            vocation,
            fleet_size,
            seed,
        };
        if !seen.insert(coordinate.clone()) {
            return Err(format!(
                "duplicate_run_coordinate:vocation={},fleet_size={},seed={}",
                coordinate.vocation, coordinate.fleet_size, coordinate.seed
            ));
        }
        entries.push(RunManifestEntry { coordinate, report_path });
    }
    if entries.is_empty() {
        return Err("run_manifest_has_no_entries".to_string());
    }
    entries.sort_by(|a, b| a.coordinate.cmp(&b.coordinate));
    Ok(entries)
}

fn validate_and_own(report: SimulationReport, coordinate: &Coordinate) -> Result<(SimulationReportOwned, String), String> {
    if report.schema_version != "0.1.0" || report.suite_id != EXPECTED_SUITE {
        return Err("unsupported_simulation_report_schema_or_suite".into());
    }
    if report.claim_ceiling.trim().is_empty() {
        return Err("simulation_report_missing_claim_ceiling".into());
    }
    if report.results.len() != 2 || report.capacity_frontiers.len() != 2 {
        return Err("each_run_must_have_two_policy_results_and_two_capacity_frontiers".into());
    }
    let expected_suffix = format!("-fleet-{}-seed-{}", coordinate.fleet_size, coordinate.seed);
    let scenario_id = report.results.first().map(|r| r.scenario_id.as_str()).unwrap_or("");
    if !scenario_id.ends_with(&expected_suffix) {
        return Err(format!("scenario_id_does_not_match_run_coordinate:expected_suffix={expected_suffix}"));
    }
    if report.results.iter().any(|result| result.scenario_id != scenario_id || result.evidence_class != "estimate") {
        return Err("policy_results_must_share_scenario_and_be_estimate_evidence".into());
    }
    if report.capacity_frontiers.iter().any(|frontier| frontier.scenario_id != scenario_id || frontier.evidence_class != "estimate") {
        return Err("frontiers_must_share_scenario_and_be_estimate_evidence".into());
    }

    let policy_names: BTreeSet<String> = report.results.iter().map(|result| result.policy.clone()).collect();
    let frontier_policy_names: BTreeSet<String> = report.capacity_frontiers.iter().map(|frontier| frontier.policy.clone()).collect();
    let expected_policies: BTreeSet<String> = ["fcfs".to_string(), "earliest_departure_first".to_string()].into_iter().collect();
    if policy_names != expected_policies || frontier_policy_names != expected_policies {
        return Err("run_must_include_exactly_fcfs_and_earliest_departure_first".into());
    }

    let first = &report.results[0];
    if first.job_count == 0 {
        return Err("policy_result_has_zero_jobs".into());
    }
    let mut reference_job_ids = BTreeMap::<String, f64>::new();
    for job in &first.jobs {
        if !job.requested_battery_energy_kwh.is_finite() || job.requested_battery_energy_kwh < 0.0 {
            return Err("job_request_energy_invalid".into());
        }
        if reference_job_ids.insert(job.job_id.clone(), job.requested_battery_energy_kwh).is_some() {
            return Err("duplicate_job_id_in_policy_result".into());
        }
    }
    if reference_job_ids.len() != first.job_count {
        return Err("job_count_does_not_match_job_records".into());
    }

    for result in &report.results {
        if result.job_count != first.job_count
            || result.completed_by_deadline_count > result.job_count
            || result.missed_deadline_count != result.job_count - result.completed_by_deadline_count
            || result.jobs.len() != result.job_count
            || !result.site_power_limit_respected
            || !result.individual_power_limits_respected
            || !result.energy_balance_residual_kwh.is_finite()
            || result.energy_balance_residual_kwh.abs() > EPSILON
            || !finite_in_range(result.energy_service_fraction, 0.0, 1.0)
            || !result.median_queue_wait_minutes.is_finite()
            || !result.p95_queue_wait_minutes.is_finite()
            || !result.maximum_site_input_power_kw.is_finite()
            || !result.requested_battery_energy_kwh.is_finite()
            || !result.delivered_battery_energy_kwh.is_finite()
            || !result.unserved_battery_energy_kwh.is_finite()
        {
            return Err(format!("policy_result_failed_accounting_or_hard_limit_checks:{}", result.policy));
        }
        let current: BTreeMap<String, f64> = result.jobs.iter().map(|job| (job.job_id.clone(), job.requested_battery_energy_kwh)).collect();
        if current.len() != result.job_count || current.len() != reference_job_ids.len() {
            return Err("policy_pair_job_denominator_mismatch".into());
        }
        for (job_id, requested) in &reference_job_ids {
            let other = current.get(job_id).ok_or("policy_pair_job_identity_mismatch")?;
            if (other - requested).abs() > EPSILON {
                return Err("policy_pair_requested_energy_mismatch".into());
            }
        }
    }

    let mut owned_frontiers = Vec::new();
    for frontier in &report.capacity_frontiers {
        if !(frontier.base_site_power_budget_kw.is_finite() && frontier.base_site_power_budget_kw > 0.0)
            || frontier.minimum_deadline_completion_target != SERVICE_TARGET
            || frontier.minimum_energy_service_target != SERVICE_TARGET
            || frontier.sample_spacing_percent != 5
            || frontier.points.len() != POINT_COUNT
        {
            return Err(format!("frontier_contract_mismatch:{}", frontier.policy));
        }
        let mut points = Vec::new();
        for (i, point) in frontier.points.iter().enumerate() {
            let expected_fraction = (i + 1) as f64 / POINT_COUNT as f64;
            if (point.fraction_of_declared_base_capacity - expected_fraction).abs() > EPSILON
                || (point.installed_site_power_budget_kw - frontier.base_site_power_budget_kw * expected_fraction).abs() > EPSILON
                || !point.site_power_limit_respected
                || !point.individual_power_limits_respected
                || !finite_in_range(point.deadline_completion_fraction, 0.0, 1.0)
                || !finite_in_range(point.energy_service_fraction, 0.0, 1.0)
                || point.completed_by_deadline_count > first.job_count
                || point.missed_deadline_count != first.job_count - point.completed_by_deadline_count
                || !point.delivered_battery_energy_kwh.is_finite()
                || !point.unserved_battery_energy_kwh.is_finite()
                || !point.maximum_site_input_power_kw.is_finite()
            {
                return Err(format!("frontier_point_invalid:{}:index={i}", frontier.policy));
            }
            let expected_meets = point.deadline_completion_fraction + EPSILON >= SERVICE_TARGET
                && point.energy_service_fraction + EPSILON >= SERVICE_TARGET
                && point.site_power_limit_respected
                && point.individual_power_limits_respected;
            if expected_meets != point.meets_service_target {
                return Err(format!("frontier_service_target_flag_mismatch:{}:index={i}", frontier.policy));
            }
            points.push(CapacityPointOwned {
                installed_site_power_budget_kw: point.installed_site_power_budget_kw,
                fraction_of_declared_base_capacity: point.fraction_of_declared_base_capacity,
                completed_by_deadline_count: point.completed_by_deadline_count,
                missed_deadline_count: point.missed_deadline_count,
                deadline_completion_fraction: point.deadline_completion_fraction,
                energy_service_fraction: point.energy_service_fraction,
                delivered_battery_energy_kwh: point.delivered_battery_energy_kwh,
                unserved_battery_energy_kwh: point.unserved_battery_energy_kwh,
                maximum_site_input_power_kw: point.maximum_site_input_power_kw,
                meets_service_target: point.meets_service_target,
            });
        }
        owned_frontiers.push(FrontierOwned {
            policy: frontier.policy.clone(),
            base_site_power_budget_kw: frontier.base_site_power_budget_kw,
            points,
        });
    }
    let evidence_reference = report.results.first().unwrap().evidence_reference.clone();
    let owned_results = report.results.into_iter().map(|result| PolicyResultOwned {
        policy: result.policy,
        job_count: result.job_count,
        completed_by_deadline_count: result.completed_by_deadline_count,
        missed_deadline_count: result.missed_deadline_count,
        requested_battery_energy_kwh: result.requested_battery_energy_kwh,
        delivered_battery_energy_kwh: result.delivered_battery_energy_kwh,
        unserved_battery_energy_kwh: result.unserved_battery_energy_kwh,
        energy_balance_residual_kwh: result.energy_balance_residual_kwh,
        energy_service_fraction: result.energy_service_fraction,
        median_queue_wait_minutes: result.median_queue_wait_minutes,
        p95_queue_wait_minutes: result.p95_queue_wait_minutes,
        maximum_site_input_power_kw: result.maximum_site_input_power_kw,
    }).collect();
    Ok((
        SimulationReportOwned {
            results: owned_results,
            frontiers: owned_frontiers,
        },
        evidence_reference,
    ))
}

fn aggregate(runs: &[Run], options: &Options, run_manifest_sha256: String) -> Result<(RobustReport, String), String> {
    let mut fleet_sizes = BTreeSet::<usize>::new();
    let mut vocations = BTreeSet::<String>::new();
    let mut grouped = BTreeMap::<(String, usize), Vec<&Run>>::new();
    for run in runs {
        fleet_sizes.insert(run.coordinate.fleet_size);
        vocations.insert(run.coordinate.vocation.clone());
        grouped.entry((run.coordinate.vocation.clone(), run.coordinate.fleet_size)).or_default().push(run);
    }
    if fleet_sizes.len() < options.minimum_fleet_sizes || vocations.len() < options.minimum_vocations {
        return Err(format!("matrix_too_small:fleet_sizes={},vocations={}", fleet_sizes.len(), vocations.len()));
    }

    let mut policy_aggregates = Vec::<PolicyAggregate>::new();
    for ((vocation, fleet_size), group_runs) in &grouped {
        if group_runs.len() != options.expected_seeds {
            return Err(format!("seed_count_mismatch:vocation={vocation},fleet_size={fleet_size},expected={},actual={}", options.expected_seeds, group_runs.len()));
        }
        let seeds: BTreeSet<u64> = group_runs.iter().map(|run| run.coordinate.seed).collect();
        let expected_seeds: BTreeSet<u64> = (0..options.expected_seeds as u64).collect();
        if seeds != expected_seeds {
            return Err(format!("seed_coordinates_incomplete:vocation={vocation},fleet_size={fleet_size}"));
        }
        let expected_base = (fleet_size.div_ceil(2) * 150) as f64;
        for run in group_runs {
            for frontier in &run.source_report.frontiers {
                if (frontier.base_site_power_budget_kw - expected_base).abs() > EPSILON {
                    return Err(format!("base_site_budget_mismatch:vocation={vocation},fleet_size={fleet_size},seed={}", run.coordinate.seed));
                }
            }
        }

        for policy in ["fcfs", "earliest_departure_first"] {
            let mut run_results = Vec::<(&Run, &PolicyResultOwned)>::new();
            let mut run_frontiers = Vec::<(&Run, &FrontierOwned)>::new();
            for run in group_runs {
                let result = run.source_report.results.iter().find(|r| r.policy == policy).ok_or("missing policy result")?;
                let frontier = run.source_report.frontiers.iter().find(|f| f.policy == policy).ok_or("missing policy frontier")?;
                run_results.push((run, result));
                run_frontiers.push((run, frontier));
            }
            let reference_points = &run_frontiers[0].1.points;
            for (_, frontier) in &run_frontiers {
                if frontier.points.len() != POINT_COUNT {
                    return Err("incomplete frontier while aggregating".into());
                }
                for (index, point) in frontier.points.iter().enumerate() {
                    if (point.fraction_of_declared_base_capacity - reference_points[index].fraction_of_declared_base_capacity).abs() > EPSILON {
                        return Err("frontier_capacity_fraction_mismatch_across_seeds".into());
                    }
                }
            }
            let base_capacity = run_frontiers[0].1.base_site_power_budget_kw;
            let port_count = fleet_size.div_ceil(2);
            let mut capacity_points = Vec::with_capacity(POINT_COUNT);
            let mut robust_minimum = None;
            for point_index in 0..POINT_COUNT {
                let observed: Vec<&CapacityPointOwned> = run_frontiers.iter().map(|(_, f)| &f.points[point_index]).collect();
                let feasible_count = observed.iter().filter(|p| p.meets_service_target).count();
                let empirical_fraction = feasible_count as f64 / observed.len() as f64;
                let fraction = observed[0].fraction_of_declared_base_capacity;
                let capacity_kw = observed[0].installed_site_power_budget_kw;
                if observed.iter().any(|p| (p.installed_site_power_budget_kw - capacity_kw).abs() > EPSILON) {
                    return Err("capacity_kw_mismatch_across_seeds".into());
                }
                let failed_seed_ids: Vec<u64> = run_frontiers.iter()
                    .filter(|(_, f)| !f.points[point_index].meets_service_target)
                    .map(|(run, _)| run.coordinate.seed)
                    .collect();
                let robust_target_met = empirical_fraction + EPSILON >= options.coverage_target;
                if robust_target_met && robust_minimum.is_none() {
                    robust_minimum = Some(capacity_kw);
                }
                capacity_points.push(CapacityPointAggregate {
                    capacity_fraction_of_port_nameplate: fraction,
                    installed_site_power_budget_kw: capacity_kw,
                    installed_site_power_kw_per_vehicle: capacity_kw / *fleet_size as f64,
                    replicate_count: observed.len(),
                    replications_meeting_service_target: feasible_count,
                    empirical_feasibility_fraction: empirical_fraction,
                    coverage_target: options.coverage_target,
                    robust_target_met,
                    wilson_95_percent_interval: wilson_interval(feasible_count, observed.len()),
                    deadline_completion_fraction: distribution(&observed.iter().map(|p| p.deadline_completion_fraction).collect::<Vec<_>>()),
                    energy_service_fraction: distribution(&observed.iter().map(|p| p.energy_service_fraction).collect::<Vec<_>>()),
                    delivered_battery_energy_kwh: distribution(&observed.iter().map(|p| p.delivered_battery_energy_kwh).collect::<Vec<_>>()),
                    unserved_battery_energy_kwh: distribution(&observed.iter().map(|p| p.unserved_battery_energy_kwh).collect::<Vec<_>>()),
                    maximum_site_input_power_kw: distribution(&observed.iter().map(|p| p.maximum_site_input_power_kw).collect::<Vec<_>>()),
                    failed_seed_ids,
                });
            }
            let total_job_misses: usize = run_results.iter().map(|(_, r)| r.missed_deadline_count).sum();
            let jobs_per_replicate = run_results[0].1.job_count;
            if run_results.iter().any(|(_, r)| r.job_count != jobs_per_replicate) {
                return Err("job_count_mismatch_across_seeds".into());
            }
            policy_aggregates.push(PolicyAggregate {
                vocation: vocation.clone(),
                fleet_size: *fleet_size,
                policy: policy.to_string(),
                replicate_count: run_results.len(),
                jobs_per_replicate,
                port_count,
                port_to_vehicle_ratio: port_count as f64 / *fleet_size as f64,
                declared_port_nameplate_site_power_kw: base_capacity,
                coverage_target: options.coverage_target,
                robust_minimum_sampled_site_power_kw: robust_minimum,
                robust_minimum_sampled_site_power_kw_per_vehicle: robust_minimum.map(|kw| kw / *fleet_size as f64),
                sampled_capacity_spacing_percent: 5,
                deadline_completion_fraction: distribution(&run_results.iter().map(|(_, r)| r.completed_by_deadline_count as f64 / r.job_count as f64).collect::<Vec<_>>()),
                energy_service_fraction: distribution(&run_results.iter().map(|(_, r)| r.energy_service_fraction).collect::<Vec<_>>()),
                missed_deadline_jobs_total: total_job_misses,
                unserved_battery_energy_kwh: distribution(&run_results.iter().map(|(_, r)| r.unserved_battery_energy_kwh).collect::<Vec<_>>()),
                median_queue_wait_minutes: distribution(&run_results.iter().map(|(_, r)| r.median_queue_wait_minutes).collect::<Vec<_>>()),
                p95_queue_wait_minutes: distribution(&run_results.iter().map(|(_, r)| r.p95_queue_wait_minutes).collect::<Vec<_>>()),
                maximum_site_input_power_kw: distribution(&run_results.iter().map(|(_, r)| r.maximum_site_input_power_kw).collect::<Vec<_>>()),
                capacity_points,
            });
        }
    }
    policy_aggregates.sort_by(|a, b| a.vocation.cmp(&b.vocation).then(a.fleet_size.cmp(&b.fleet_size)).then(a.policy.cmp(&b.policy)));

    let receipts = runs.iter().map(|run| RunReceipt {
        vocation: run.coordinate.vocation.clone(),
        fleet_size: run.coordinate.fleet_size,
        seed: run.coordinate.seed,
        report_sha256: run.report_sha256.clone(),
        source_evidence_reference: run.source_evidence_reference.clone(),
    }).collect();

    let report = RobustReport {
        schema_version: "0.1.0".into(),
        tool: "ev-capacity-robustness".into(),
        benchmark_id: "EV-ROBUST-001".into(),
        expected_seeds_per_vocation_size: options.expected_seeds,
        fleet_sizes: fleet_sizes.into_iter().collect(),
        vocations: vocations.into_iter().collect(),
        policies: vec!["fcfs".into(), "earliest_departure_first".into()],
        empirical_capacity_coverage_target: options.coverage_target,
        per_run_service_targets: ServiceTargets {
            minimum_deadline_completion_fraction: SERVICE_TARGET,
            minimum_energy_service_fraction: SERVICE_TARGET,
            all_modeled_power_limits_respected: true,
        },
        caveat: "Coverage is the observed fraction of deterministic resamples meeting the service target, not a population-level reliability guarantee. The Wilson interval describes binomial sampling uncertainty only if the sampled runs can reasonably be treated as independent; it does not validate the input duty model. This benchmark uses estimate-only workloads from published operational-summary rows plus declared vehicle/battery/grid assumptions.".into(),
        run_manifest_sha256,
        validated_run_count: runs.len(),
        run_receipts: receipts,
        policy_aggregates,
        claim_ceiling: "Empirical robustness summary of simulated, estimate-only workloads. Not metered field performance, a probability guarantee for unobserved fleets, a validated battery/charger model, AC power-flow study, causal policy comparison, or evidence of South African operational feasibility.".into(),
    };
    let csv = to_csv(&report);
    Ok((report, csv))
}

fn to_csv(report: &RobustReport) -> String {
    let mut out = String::from("vocation,fleet_size,policy,replicate_count,capacity_fraction,installed_capacity_kw,capacity_kw_per_vehicle,feasible_replicates,empirical_feasibility_fraction,coverage_target,robust_target_met,wilson95_low,wilson95_high,deadline_p05,deadline_p50,deadline_p95,energy_service_p05,energy_service_p50,energy_service_p95,failed_seed_ids\n");
    for group in &report.policy_aggregates {
        for point in &group.capacity_points {
            let fields = [
                csv_escape(&group.vocation),
                group.fleet_size.to_string(),
                csv_escape(&group.policy),
                group.replicate_count.to_string(),
                format!("{:.6}", point.capacity_fraction_of_port_nameplate),
                format!("{:.6}", point.installed_site_power_budget_kw),
                format!("{:.6}", point.installed_site_power_kw_per_vehicle),
                point.replications_meeting_service_target.to_string(),
                format!("{:.6}", point.empirical_feasibility_fraction),
                format!("{:.6}", point.coverage_target),
                point.robust_target_met.to_string(),
                format!("{:.6}", point.wilson_95_percent_interval[0]),
                format!("{:.6}", point.wilson_95_percent_interval[1]),
                display_opt(point.deadline_completion_fraction.p05),
                display_opt(point.deadline_completion_fraction.p50),
                display_opt(point.deadline_completion_fraction.p95),
                display_opt(point.energy_service_fraction.p05),
                display_opt(point.energy_service_fraction.p50),
                display_opt(point.energy_service_fraction.p95),
                csv_escape(&point.failed_seed_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(";")),
            ];
            out.push_str(&fields.join(","));
            out.push('\n');
        }
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn display_opt(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.6}")).unwrap_or_default()
}

fn run(options: Options) -> Result<(), String> {
    let manifest_bytes = fs::read(&options.runs_manifest)
        .map_err(|error| format!("cannot_read_run_manifest:{}:{error}", options.runs_manifest.display()))?;
    let manifest_text = std::str::from_utf8(&manifest_bytes).map_err(|_| "run_manifest_must_be_utf8")?;
    let entries = parse_run_manifest(manifest_text)?;
    let mut runs = Vec::<Run>::new();
    for entry in entries {
        let report_bytes = fs::read(&entry.report_path)
            .map_err(|error| format!("cannot_read_report:{}:{error}", entry.report_path.display()))?;
        let report: SimulationReport = serde_json::from_slice(&report_bytes)
            .map_err(|error| format!("invalid_simulation_report_json:{}:{error}", entry.report_path.display()))?;
        let (owned, source_evidence_reference) = validate_and_own(report, &entry.coordinate)
            .map_err(|error| format!("invalid_run:{}:{}:{}", entry.coordinate.vocation, entry.coordinate.fleet_size, error))?;
        runs.push(Run {
            coordinate: entry.coordinate,
            report_sha256: sha256(&report_bytes),
            source_evidence_reference,
            source_report: owned,
        });
    }
    let mut coordinate_groups = BTreeMap::<(String, usize), BTreeSet<u64>>::new();
    for run in &runs {
        coordinate_groups.entry((run.coordinate.vocation.clone(), run.coordinate.fleet_size)).or_default().insert(run.coordinate.seed);
    }
    let expected_coordinate_groups = coordinate_groups.len();
    let expected_rows = expected_coordinate_groups * options.expected_seeds;
    if runs.len() != expected_rows {
        return Err(format!("run_manifest_row_count_mismatch:expected={expected_rows},actual={}", runs.len()));
    }
    for ((vocation, fleet_size), seeds) in &coordinate_groups {
        let expected: BTreeSet<u64> = (0..options.expected_seeds as u64).collect();
        if seeds != &expected {
            return Err(format!("seed_set_mismatch:vocation={vocation},fleet_size={fleet_size}"));
        }
    }
    let (report, csv) = aggregate(&runs, &options, sha256(&manifest_bytes))?;
    let json = serde_json::to_string_pretty(&report).map_err(|error| format!("serialize_json:{error}"))?;
    fs::write(&options.output_json, json).map_err(|error| format!("write_json:{}:{error}", options.output_json.display()))?;
    fs::write(&options.output_csv, csv).map_err(|error| format!("write_csv:{}:{error}", options.output_csv.display()))?;
    let robust_count = report.policy_aggregates.iter().filter(|g| g.robust_minimum_sampled_site_power_kw.is_some()).count();
    println!(
        "aggregated {} estimate-only runs across {} fleet/vocation groups; {} policy groups have an observed robust sampled capacity",
        report.validated_run_count,
        report.policy_aggregates.len() / 2,
        robust_count
    );
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match parse_args(&args).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ev-capacity-robustness: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_percentiles_keep_sample_denominator() {
        let d = distribution(&(1..=20).map(|x| x as f64).collect::<Vec<_>>());
        assert_eq!(d.count, 20);
        assert_eq!(d.p05, Some(1.0));
        assert_eq!(d.p50, Some(10.5));
        assert_eq!(d.p95, Some(19.0));
    }

    #[test]
    fn wilson_interval_is_bounded_and_nonzero_width() {
        let interval = wilson_interval(20, 20);
        assert!(interval[0] > 0.80 && interval[0] < 0.90);
        assert_eq!(interval[1], 1.0);
        let zero = wilson_interval(0, 20);
        assert_eq!(zero[0], 0.0);
        assert!(zero[1] < 0.20);
    }

    #[test]
    fn parses_manifest_and_rejects_duplicate_run_coordinates() {
        let good = "fleet_size\tseed\tvocation\treport_path\n10\t0\tbeverage\t/tmp/a.json\n";
        assert_eq!(parse_run_manifest(good).unwrap().len(), 1);
        let duplicate = "10\t0\tbeverage\t/tmp/a.json\n10\t0\tbeverage\t/tmp/b.json\n";
        assert!(parse_run_manifest(duplicate).unwrap_err().contains("duplicate_run_coordinate"));
    }

    #[test]
    fn csv_escaping_preserves_vocation_and_failed_seed_lists() {
        assert_eq!(csv_escape("food, delivery"), "\"food, delivery\"");
        assert_eq!(csv_escape("seeds; 1; 2"), "seeds; 1; 2");
        assert_eq!(display_opt(None), "");
    }

    #[test]
    fn empirical_95_percent_coverage_needs_19_of_20_runs() {
        assert!((19.0_f64 / 20.0) >= TARGET_CAPACITY_COVERAGE);
        assert!((18.0_f64 / 20.0) < TARGET_CAPACITY_COVERAGE);
        assert!(wilson_interval(19, 20)[0] < TARGET_CAPACITY_COVERAGE);
    }
}
