use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
    env, fs, process,
};

const DEFAULT_FIXTURE: &str =
    "tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json";
const CLAIM_CEILING: &str = "Deterministic synthetic discrete-event sensitivity model only. It is not a calibrated plant digital twin, audited production forecast, site design, verified safety case, vendor quote, or capital-investment recommendation.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    seed: u64,
    #[serde(default)]
    replication_seeds: Vec<u64>,
    horizon_seconds: u64,
    warmup_seconds: u64,
    productive_seconds_per_year: u64,
    variant_count: usize,
    variant_batch_size: u64,
    #[serde(default)]
    dispatch_policy: DispatchPolicy,
    #[serde(default = "default_max_variant_skip_seconds")]
    max_variant_skip_seconds: u64,
    mean_supply_interarrival_seconds: f64,
    supply_interarrival_variability_fraction: f64,
    source_buffer_capacity: usize,
    buffer_capacities: Vec<usize>,
    input_evidence_class: EvidenceClass,
    calibration_gate: GateStatus,
    local_compliance_gate: GateStatus,
    line_safety_gate: GateStatus,
    product_quality_release_gate: GateStatus,
    stations: Vec<Station>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Station {
    station_id: String,
    label: String,
    servers: usize,
    mean_cycle_seconds: f64,
    cycle_time_variability_fraction: f64,
    changeover_seconds: f64,
    mean_time_between_failures_seconds: f64,
    mean_time_to_repair_seconds: f64,
    good_yield_fraction: f64,
    max_rework_attempts: u32,
    evidence_class: EvidenceClass,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum DispatchPolicy {
    #[default]
    Fifo,
    MinimizeChangeover,
}

fn default_max_variant_skip_seconds() -> u64 {
    600
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    PublicMarketData,
    ManufacturerSpec,
    OfficialRecord,
    Measured,
    IndependentTest,
    AuditedCost,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GateStatus {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_catalog_sha256: String,
    seed: u64,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    random_stream_policy: String,
    dispatch_policy: DispatchPolicy,
    max_variant_skip_seconds: u64,
    scenario_count: usize,
    scenario_statistics: Vec<ScenarioStatistics>,
    replication_outcomes: Vec<ReplicationOutcome>,
    station_sequence_statistics: Vec<StationSequenceStatistics>,
    measured_horizon_seconds: u64,
    productive_seconds_per_year: u64,
    input_evidence_class: EvidenceClass,
    calibration_gate: GateStatus,
    local_compliance_gate: GateStatus,
    line_safety_gate: GateStatus,
    product_quality_release_gate: GateStatus,
    release_ready: bool,
    output_never_exceeds_released_input: bool,
    all_buffers_respected_capacity: bool,
    station_time_accounting_within_horizon: bool,
    scenarios: Vec<ScenarioResult>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    scenario_id: String,
    job_lifecycle_summary: JobLifecycleSummary,
    job_lifecycle_events: Vec<JobLifecycleEvent>,
    intervention_station_id: Option<String>,
    added_servers: usize,
    released_jobs_total: u64,
    good_units_total: u64,
    measured_good_units: u64,
    scrapped_jobs_total: u64,
    measured_rework_attempts: u64,
    measured_good_units_per_hour: f64,
    annualized_good_units_at_declared_operating_hours: f64,
    supplier_backlog_peak_units: usize,
    supplier_backlog_average_units: f64,
    source_buffer_peak_units: usize,
    source_buffer_average_units: f64,
    interstation_buffers: Vec<BufferResult>,
    stations: Vec<StationResult>,
    output_never_exceeds_released_input: bool,
    all_buffers_respected_capacity: bool,
    station_time_accounting_within_horizon: bool,
    release_ready: bool,
    claim_ceiling: String,
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
struct ScenarioStatistics {
    scenario_id: String,
    intervention_station_id: Option<String>,
    good_units_per_hour: DistributionSummary,
    paired_good_units_per_hour_delta_vs_baseline: Option<DistributionSummary>,
    paired_relative_output_delta_fraction_vs_baseline: Option<DistributionSummary>,
    scrapped_jobs_total: DistributionSummary,
    measured_rework_attempts: DistributionSummary,
    per_seed_median_good_job_lead_time_seconds: Option<DistributionSummary>,
    per_seed_p95_good_job_lead_time_seconds: Option<DistributionSummary>,
    right_censored_jobs_at_horizon: DistributionSummary,
    station_first_pass_yield_by_station: Vec<StationFirstPassYieldStatistics>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationFirstPassYieldStatistics {
    station_id: String,
    first_attempts_started_in_cohort: DistributionSummary,
    observed_first_attempts: DistributionSummary,
    first_attempt_passed: DistributionSummary,
    first_attempt_failed: DistributionSummary,
    unresolved_first_attempts_at_horizon: DistributionSummary,
    observed_first_pass_yield_fraction: Option<DistributionSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationFirstPassYield {
    station_id: String,
    first_attempts_started_in_cohort: u64,
    first_attempts_with_observed_outcome: u64,
    first_attempt_passed: u64,
    first_attempt_failed: u64,
    unresolved_first_attempts_at_horizon: u64,
    observed_first_pass_yield_fraction: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JobLifecycleSummary {
    cohort_definition: String,
    eligible_released_jobs_after_warmup: u64,
    accepted_good_jobs_in_cohort: u64,
    scrapped_jobs_in_cohort: u64,
    right_censored_jobs_at_horizon: u64,
    good_job_lead_time_seconds: Option<DistributionSummary>,
    median_good_job_lead_time_seconds: Option<f64>,
    p95_good_job_lead_time_seconds: Option<f64>,
    station_first_pass_yield: Vec<StationFirstPassYield>,
    event_count_in_ledger: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum JobLifecycleEventKind {
    JobRelease,
    BufferEnter,
    BufferExit,
    StationStart,
    StationEnd,
    QualityPass,
    QualityFail,
    ReworkStart,
    JobScrapped,
    FinalGoodCompletion,
    JobCensoredAtHorizon,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JobLifecycleEvent {
    schema_version: String,
    seed: u64,
    scenario_id: String,
    event_sequence: u64,
    timestamp_seconds: u64,
    kind: JobLifecycleEventKind,
    job_id: u64,
    variant_id: usize,
    station_id: Option<String>,
    resource_id: Option<String>,
    buffer_id: Option<String>,
    attempt_number: Option<u32>,
    evidence_class: String,
}

#[derive(Debug, Clone)]
struct JobRecord {
    job_id: u64,
    variant_id: usize,
    released_at_seconds: u64,
    first_attempt_started_at_by_station: Vec<Option<u64>>,
    first_attempt_outcome_by_station: Vec<Option<bool>>,
    attempts_started_by_station: Vec<u32>,
    final_good_at_seconds: Option<u64>,
    scrapped_at_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ReplicationOutcome {
    seed: u64,
    scenario_id: String,
    intervention_station_id: Option<String>,
    good_units_per_hour: f64,
    baseline_good_units_per_hour: f64,
    paired_good_units_per_hour_delta_vs_baseline: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationSequenceStatistics {
    scenario_id: String,
    station_id: String,
    changeovers_started: DistributionSummary,
    output_transfers: DistributionSummary,
    out_of_order_output_transfers: DistributionSummary,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct BufferResult {
    after_station_id: String,
    capacity_units: usize,
    peak_occupancy_units: usize,
    average_occupancy_units: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StationResult {
    station_id: String,
    label: String,
    server_count: usize,
    attempts_started_after_warmup: u64,
    successful_attempts_after_warmup: u64,
    rework_attempts_after_warmup: u64,
    scrapped_jobs_after_warmup: u64,
    changeovers_started_after_warmup: u64,
    output_transfers_after_warmup: u64,
    out_of_order_output_transfers_after_warmup: u64,
    busy_fraction: f64,
    blocked_fraction: f64,
    starved_fraction: f64,
    down_fraction: f64,
    accounted_fraction: f64,
    added_server_good_output_delta_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
struct Job {
    id: u64,
    variant: usize,
    queued_since_seconds: u64,
}

#[derive(Debug, Clone)]
enum ServerState {
    Idle,
    Busy {
        job: Job,
        station_attempts: u32,
        service_seconds_for_job: u64,
    },
    WaitingOutput {
        job: Job,
        machine_fails_after_release: bool,
    },
    Down,
}

#[derive(Debug, Clone)]
struct Server {
    state: ServerState,
    last_variant: Option<usize>,
}

#[derive(Debug, Default, Clone)]
struct StationMetrics {
    busy_seconds: f64,
    blocked_seconds: f64,
    starved_seconds: f64,
    down_seconds: f64,
    attempts_started_after_warmup: u64,
    successful_attempts_after_warmup: u64,
    rework_attempts_after_warmup: u64,
    scrapped_jobs_after_warmup: u64,
    changeovers_started_after_warmup: u64,
    output_transfers_after_warmup: u64,
    out_of_order_output_transfers_after_warmup: u64,
}

#[derive(Debug, Clone)]
struct StationRuntime {
    servers: Vec<Server>,
    metrics: StationMetrics,
    last_output_job_id: Option<u64>,
}

#[derive(Debug, Clone)]
struct SimState {
    now: u64,
    seed: u64,
    scenario_id: String,
    record_lifecycle_events: bool,
    next_lifecycle_sequence: u64,
    lifecycle_events: Vec<JobLifecycleEvent>,
    job_records: BTreeMap<u64, JobRecord>,
    next_event_sequence: u64,
    events: BinaryHeap<TimedEvent>,
    supplier_rng: DeterministicRng,
    stations: Vec<StationRuntime>,
    buffers: Vec<VecDeque<Job>>,
    buffer_peak_occupancy: Vec<usize>,
    buffer_wip_area: Vec<f64>,
    source_buffer: VecDeque<Job>,
    source_buffer_peak: usize,
    source_buffer_wip_area: f64,
    supplier_backlog: VecDeque<Job>,
    supplier_backlog_peak: usize,
    supplier_backlog_wip_area: f64,
    next_job_id: u64,
    released_jobs_total: u64,
    good_units_total: u64,
    measured_good_units: u64,
    scrapped_jobs_total: u64,
    measured_rework_attempts: u64,
    buffer_capacity_violation: bool,
}

#[derive(Debug, Clone)]
struct TimedEvent {
    at: u64,
    sequence: u64,
    kind: EventKind,
}

#[derive(Debug, Clone)]
enum EventKind {
    SupplyArrival,
    OperationComplete { station: usize, server: usize },
    RepairComplete { station: usize, server: usize },
}

impl PartialEq for TimedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at && self.sequence == other.sequence
    }
}
impl Eq for TimedEvent {}
impl PartialOrd for TimedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TimedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max heap; reverse time and sequence to create a
        // deterministic min-heap with FIFO tie-breaking.
        other
            .at
            .cmp(&self.at)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

#[derive(Debug, Clone)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9e3779b97f4a7c15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*: fixed algorithm and integer width for portable fixtures.
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545f4914f6cdd1d)
    }

    fn unit(&mut self) -> f64 {
        // Use 53 high bits to obtain a deterministic value in [0, 1).
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
    }

    fn around(&mut self, mean: f64, variability: f64) -> u64 {
        let factor = 1.0 + ((self.unit() * 2.0 - 1.0) * variability);
        (mean * factor).round().max(1.0) as u64
    }

    fn for_key(seed: u64, domain: &str, station_id: &str, job_id: u64, attempt: u32) -> Self {
        // Stable FNV-1a over explicit typed fields; don't rely on DefaultHasher
        // as a cross-version serialization contract.
        let mut hash = 0xcbf29ce484222325_u64;
        fn absorb(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash ^= u64::from(*byte);
                *hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        let seed_bytes = seed.to_le_bytes();
        let domain_bytes = domain.as_bytes();
        let station_bytes = station_id.as_bytes();
        let job_bytes = job_id.to_le_bytes();
        let attempt_bytes = attempt.to_le_bytes();
        for part in [
            seed_bytes.as_slice(),
            domain_bytes,
            station_bytes,
            job_bytes.as_slice(),
            attempt_bytes.as_slice(),
        ] {
            absorb(&mut hash, &(part.len() as u64).to_le_bytes());
            absorb(&mut hash, part);
        }
        Self::new(hash)
    }
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_FIXTURE.to_string());
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let catalog: Catalog = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid factory simulation catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid factory simulation catalog: {error}");
        process::exit(2);
    }
    let mut report = match evaluate_catalog(&catalog) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("factory simulation failed: {error}");
            process::exit(2);
        }
    };
    report.source_catalog_sha256 = format!("{:x}", Sha256::digest(input.as_bytes()));
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize factory simulation report: {error}");
            process::exit(2);
        }
    }
}

fn validate_fraction(name: &str, value: f64, allow_one: bool) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 || if allow_one { value > 1.0 } else { value >= 1.0 } {
        return Err(format!("{name}_must_be_a_valid_fraction"));
    }
    Ok(())
}

fn validate_positive_finite(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name}_must_be_finite_and_positive"));
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0"
        || catalog.benchmark_id.trim().is_empty()
        || catalog.claim_ceiling.trim().is_empty()
    {
        return Err("invalid_version_benchmark_id_or_claim_ceiling".into());
    }
    if catalog.seed == 0 {
        return Err("seed_must_be_nonzero_for_explicit_reproducibility".into());
    }
    if !catalog.replication_seeds.is_empty() {
        if catalog.replication_seeds[0] != catalog.seed {
            return Err("replication_seeds_must_start_with_primary_seed".into());
        }
        let mut seen_seeds = BTreeSet::new();
        for seed in &catalog.replication_seeds {
            if *seed == 0 || !seen_seeds.insert(*seed) {
                return Err("replication_seeds_must_be_positive_and_unique".into());
            }
        }
    }
    if catalog.horizon_seconds < 10
        || catalog.warmup_seconds >= catalog.horizon_seconds
        || catalog.productive_seconds_per_year == 0
    {
        return Err("horizon_warmup_or_annual_operating_seconds_invalid".into());
    }
    if catalog.variant_count == 0
        || catalog.variant_batch_size == 0
        || catalog.source_buffer_capacity == 0
    {
        return Err("variant_and_source_buffer_dimensions_must_be_positive".into());
    }
    if catalog.max_variant_skip_seconds == 0 {
        return Err("max_variant_skip_seconds_must_be_positive".into());
    }
    validate_positive_finite(
        "mean_supply_interarrival_seconds",
        catalog.mean_supply_interarrival_seconds,
    )?;
    validate_fraction(
        "supply_interarrival_variability_fraction",
        catalog.supply_interarrival_variability_fraction,
        true,
    )?;
    if catalog.stations.len() < 2 {
        return Err("at_least_two_stations_required".into());
    }
    if catalog.buffer_capacities.len() != catalog.stations.len() - 1
        || catalog.buffer_capacities.contains(&0)
    {
        return Err("one_positive_capacity_required_for_each_interstation_buffer".into());
    }
    let mut ids = BTreeSet::new();
    for station in &catalog.stations {
        if station.station_id.trim().is_empty()
            || station.label.trim().is_empty()
            || station.note.trim().is_empty()
            || !ids.insert(station.station_id.clone())
        {
            return Err("station_ids_must_be_unique_and_station_text_nonempty".into());
        }
        if station.servers == 0 || station.max_rework_attempts > 10 {
            return Err(format!(
                "station_{}_server_or_rework_count_invalid",
                station.station_id
            ));
        }
        validate_positive_finite("mean_cycle_seconds", station.mean_cycle_seconds)?;
        validate_fraction(
            "cycle_time_variability_fraction",
            station.cycle_time_variability_fraction,
            true,
        )?;
        if station.cycle_time_variability_fraction >= 1.0 {
            return Err("cycle_time_variability_fraction_must_be_less_than_one".into());
        }
        if !station.changeover_seconds.is_finite() || station.changeover_seconds < 0.0 {
            return Err("changeover_seconds_must_be_finite_and_nonnegative".into());
        }
        validate_positive_finite(
            "mean_time_between_failures_seconds",
            station.mean_time_between_failures_seconds,
        )?;
        validate_positive_finite(
            "mean_time_to_repair_seconds",
            station.mean_time_to_repair_seconds,
        )?;
        validate_fraction("good_yield_fraction", station.good_yield_fraction, true)?;
        if station.good_yield_fraction <= 0.0 {
            return Err("good_yield_fraction_must_be_positive".into());
        }
    }
    Ok(())
}

fn evaluate_catalog(catalog: &Catalog) -> Result<Report, String> {
    let seeds = if catalog.replication_seeds.is_empty() {
        vec![catalog.seed]
    } else {
        catalog.replication_seeds.clone()
    };
    let mut runs = Vec::with_capacity(seeds.len());
    for seed in &seeds {
        let mut run_catalog = catalog.clone();
        run_catalog.seed = *seed;
        run_catalog.replication_seeds.clear();
        runs.push(evaluate_single_seed(&run_catalog, runs.is_empty())?);
    }
    let reference = runs
        .first()
        .cloned()
        .ok_or_else(|| "replication_seed_list_must_not_be_empty".to_string())?;
    if runs
        .iter()
        .any(|run| run.scenario_count != reference.scenario_count)
    {
        return Err("replications_returned_inconsistent_scenario_counts".into());
    }

    let baseline_rates: Vec<f64> = runs
        .iter()
        .map(|run| run.scenarios[0].measured_good_units_per_hour)
        .collect();
    let mut statistics = Vec::with_capacity(reference.scenario_count);
    for scenario_index in 0..reference.scenario_count {
        let first_scenario = &reference.scenarios[scenario_index];
        let mut good_rates = Vec::with_capacity(runs.len());
        let mut delta_rates = Vec::with_capacity(runs.len());
        let mut relative_deltas = Vec::with_capacity(runs.len());
        let mut scrapped = Vec::with_capacity(runs.len());
        let mut rework = Vec::with_capacity(runs.len());
        let mut per_seed_lead_medians = Vec::new();
        let mut per_seed_lead_p95 = Vec::new();
        let mut right_censored = Vec::with_capacity(runs.len());
        for (run, baseline_rate) in runs.iter().zip(baseline_rates.iter()) {
            let scenario = &run.scenarios[scenario_index];
            if scenario.scenario_id != first_scenario.scenario_id {
                return Err("replications_returned_inconsistent_scenario_order".into());
            }
            good_rates.push(scenario.measured_good_units_per_hour);
            scrapped.push(scenario.scrapped_jobs_total as f64);
            rework.push(scenario.measured_rework_attempts as f64);
            right_censored.push(scenario.job_lifecycle_summary.right_censored_jobs_at_horizon as f64);
            if let Some(median) = scenario.job_lifecycle_summary.median_good_job_lead_time_seconds {
                per_seed_lead_medians.push(median);
            }
            if let Some(p95) = scenario.job_lifecycle_summary.p95_good_job_lead_time_seconds {
                per_seed_lead_p95.push(p95);
            }
            if scenario_index > 0 {
                delta_rates.push(scenario.measured_good_units_per_hour - baseline_rate);
                if *baseline_rate > 0.0 {
                    relative_deltas.push(
                        (scenario.measured_good_units_per_hour - baseline_rate) / baseline_rate,
                    );
                }
            }
        }
        let mut station_first_pass_yield_by_station = Vec::new();
        for station_index in 0..catalog.stations.len() {
            let mut starts = Vec::with_capacity(runs.len());
            let mut observed = Vec::with_capacity(runs.len());
            let mut passed = Vec::with_capacity(runs.len());
            let mut failed = Vec::with_capacity(runs.len());
            let mut unresolved = Vec::with_capacity(runs.len());
            let mut yields = Vec::new();
            for run in &runs {
                let rows = &run.scenarios[scenario_index].job_lifecycle_summary.station_first_pass_yield;
                let row = rows.get(station_index)
                    .ok_or_else(|| "per_seed_station_first_pass_summary_missing".to_string())?;
                if row.station_id != catalog.stations[station_index].station_id {
                    return Err("replications_returned_inconsistent_first_pass_station_order".into());
                }
                starts.push(row.first_attempts_started_in_cohort as f64);
                observed.push(row.first_attempts_with_observed_outcome as f64);
                passed.push(row.first_attempt_passed as f64);
                failed.push(row.first_attempt_failed as f64);
                unresolved.push(row.unresolved_first_attempts_at_horizon as f64);
                if let Some(value) = row.observed_first_pass_yield_fraction {
                    yields.push(value);
                }
            }
            station_first_pass_yield_by_station.push(StationFirstPassYieldStatistics {
                station_id: catalog.stations[station_index].station_id.clone(),
                first_attempts_started_in_cohort: summarize_distribution(&starts)?,
                observed_first_attempts: summarize_distribution(&observed)?,
                first_attempt_passed: summarize_distribution(&passed)?,
                first_attempt_failed: summarize_distribution(&failed)?,
                unresolved_first_attempts_at_horizon: summarize_distribution(&unresolved)?,
                observed_first_pass_yield_fraction: if yields.is_empty() {
                    None
                } else {
                    Some(summarize_distribution(&yields)?)
                },
            });
        }
        statistics.push(ScenarioStatistics {
            scenario_id: first_scenario.scenario_id.clone(),
            intervention_station_id: first_scenario.intervention_station_id.clone(),
            good_units_per_hour: summarize_distribution(&good_rates)?,
            paired_good_units_per_hour_delta_vs_baseline: if scenario_index > 0 {
                Some(summarize_distribution(&delta_rates)?)
            } else {
                None
            },
            paired_relative_output_delta_fraction_vs_baseline: if scenario_index > 0
                && !relative_deltas.is_empty()
            {
                Some(summarize_distribution(&relative_deltas)?)
            } else {
                None
            },
            scrapped_jobs_total: summarize_distribution(&scrapped)?,
            measured_rework_attempts: summarize_distribution(&rework)?,
            per_seed_median_good_job_lead_time_seconds: if per_seed_lead_medians.is_empty() {
                None
            } else {
                Some(summarize_distribution(&per_seed_lead_medians)?)
            },
            per_seed_p95_good_job_lead_time_seconds: if per_seed_lead_p95.is_empty() {
                None
            } else {
                Some(summarize_distribution(&per_seed_lead_p95)?)
            },
            right_censored_jobs_at_horizon: summarize_distribution(&right_censored)?,
            station_first_pass_yield_by_station,
        });
    }

    let mut report = reference;
    report.replication_count = runs.len();
    report.replication_seeds = seeds;
    report.scenario_statistics = statistics;
    let mut replication_outcomes = Vec::with_capacity(runs.len() * report.scenario_count);
    for run in &runs {
        let baseline = run
            .scenarios
            .first()
            .ok_or_else(|| "replication_baseline_scenario_missing".to_string())?;
        for scenario in &run.scenarios {
            let delta = if scenario.scenario_id == "baseline" {
                None
            } else {
                Some(scenario.measured_good_units_per_hour - baseline.measured_good_units_per_hour)
            };
            replication_outcomes.push(ReplicationOutcome {
                seed: run.seed,
                scenario_id: scenario.scenario_id.clone(),
                intervention_station_id: scenario.intervention_station_id.clone(),
                good_units_per_hour: scenario.measured_good_units_per_hour,
                baseline_good_units_per_hour: baseline.measured_good_units_per_hour,
                paired_good_units_per_hour_delta_vs_baseline: delta,
            });
        }
    }
    report.replication_outcomes = replication_outcomes;
    let mut station_statistics = Vec::new();
    for scenario_index in 0..report.scenario_count {
        for station_index in 0..runs[0].scenarios[scenario_index].stations.len() {
            let reference_station = &runs[0].scenarios[scenario_index].stations[station_index];
            let mut changeovers = Vec::with_capacity(runs.len());
            let mut transfers = Vec::with_capacity(runs.len());
            let mut out_of_order_transfers = Vec::with_capacity(runs.len());
            for run in &runs {
                let station = &run.scenarios[scenario_index].stations[station_index];
                if station.station_id != reference_station.station_id {
                    return Err("replications_returned_inconsistent_station_order".into());
                }
                changeovers.push(station.changeovers_started_after_warmup as f64);
                transfers.push(station.output_transfers_after_warmup as f64);
                out_of_order_transfers
                    .push(station.out_of_order_output_transfers_after_warmup as f64);
            }
            station_statistics.push(StationSequenceStatistics {
                scenario_id: runs[0].scenarios[scenario_index].scenario_id.clone(),
                station_id: reference_station.station_id.clone(),
                changeovers_started: summarize_distribution(&changeovers)?,
                output_transfers: summarize_distribution(&transfers)?,
                out_of_order_output_transfers: summarize_distribution(&out_of_order_transfers)?,
            });
        }
    }
    report.station_sequence_statistics = station_statistics;
    report.output_never_exceeds_released_input = runs
        .iter()
        .all(|run| run.output_never_exceeds_released_input);
    report.all_buffers_respected_capacity =
        runs.iter().all(|run| run.all_buffers_respected_capacity);
    report.station_time_accounting_within_horizon = runs
        .iter()
        .all(|run| run.station_time_accounting_within_horizon);
    report.release_ready = false;
    Ok(report)
}

fn summarize_distribution(values: &[f64]) -> Result<DistributionSummary, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("distribution_requires_nonempty_finite_samples".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    let sample_standard_deviation = if count > 1 {
        let sum_squares = sorted
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>();
        Some((sum_squares / (count - 1) as f64).sqrt())
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

fn evaluate_single_seed(catalog: &Catalog, include_lifecycle_ledger: bool) -> Result<Report, String> {
    let baseline = simulate(catalog, None, include_lifecycle_ledger)?;
    let mut scenarios = Vec::with_capacity(catalog.stations.len() + 1);
    scenarios.push(baseline.clone());
    for index in 0..catalog.stations.len() {
        scenarios.push(simulate(catalog, Some(index), false)?);
    }

    let baseline_rate = baseline.measured_good_units_per_hour;
    for scenario in scenarios.iter_mut().skip(1) {
        let added_station = scenario
            .intervention_station_id
            .as_ref()
            .ok_or_else(|| "intervention_scenario_missing_station_id".to_string())?;
        let stat = scenario
            .stations
            .iter_mut()
            .find(|item| &item.station_id == added_station)
            .ok_or_else(|| "intervention_station_not_found_in_scenario".to_string())?;
        stat.added_server_good_output_delta_fraction = Some(if baseline_rate > 0.0 {
            (scenario.measured_good_units_per_hour - baseline_rate) / baseline_rate
        } else {
            0.0
        });
    }

    let output_never_exceeds_released_input = scenarios
        .iter()
        .all(|scenario| scenario.good_units_total <= scenario.released_jobs_total);
    let all_buffers_respected_capacity = scenarios
        .iter()
        .all(|scenario| scenario.all_buffers_respected_capacity);
    let station_time_accounting_within_horizon = scenarios
        .iter()
        .all(|scenario| scenario.station_time_accounting_within_horizon);
    let release_ready = false; // Simulator output alone can never approve production release.

    Ok(Report {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        source_catalog_sha256: String::new(),
        seed: catalog.seed,
        replication_count: 1,
        replication_seeds: vec![catalog.seed],
        random_stream_policy:
            "entity_keyed_service_quality_failure; dedicated_supplier_arrival_stream".to_string(),
        dispatch_policy: catalog.dispatch_policy,
        max_variant_skip_seconds: catalog.max_variant_skip_seconds,
        scenario_count: scenarios.len(),
        scenario_statistics: Vec::new(),
        replication_outcomes: Vec::new(),
        station_sequence_statistics: Vec::new(),
        measured_horizon_seconds: catalog.horizon_seconds - catalog.warmup_seconds,
        productive_seconds_per_year: catalog.productive_seconds_per_year,
        input_evidence_class: catalog.input_evidence_class,
        calibration_gate: catalog.calibration_gate,
        local_compliance_gate: catalog.local_compliance_gate,
        line_safety_gate: catalog.line_safety_gate,
        product_quality_release_gate: catalog.product_quality_release_gate,
        release_ready,
        output_never_exceeds_released_input,
        all_buffers_respected_capacity,
        station_time_accounting_within_horizon,
        scenarios,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn simulate(
    catalog: &Catalog,
    extra_server_at: Option<usize>,
    include_lifecycle_ledger: bool,
) -> Result<ScenarioResult, String> {
    if let Some(index) = extra_server_at {
        if index >= catalog.stations.len() {
            return Err("intervention_station_index_out_of_bounds".into());
        }
    }
    let station_runtime = catalog
        .stations
        .iter()
        .enumerate()
        .map(|(index, station)| {
            let count = station.servers + if extra_server_at == Some(index) { 1 } else { 0 };
            StationRuntime {
                servers: (0..count)
                    .map(|_| Server {
                        state: ServerState::Idle,
                        last_variant: None,
                    })
                    .collect(),
                metrics: StationMetrics::default(),
                last_output_job_id: None,
            }
        })
        .collect();
    let scenario_id = match extra_server_at {
        None => "baseline".to_string(),
        Some(index) => format!("add-one-server-{}", catalog.stations[index].station_id),
    };
    let mut state = SimState {
        now: 0,
        seed: catalog.seed,
        scenario_id,
        record_lifecycle_events: include_lifecycle_ledger && extra_server_at.is_none(),
        next_lifecycle_sequence: 0,
        lifecycle_events: Vec::new(),
        job_records: BTreeMap::new(),
        next_event_sequence: 0,
        events: BinaryHeap::new(),
        supplier_rng: DeterministicRng::new(catalog.seed ^ 0x535550504c595f31),
        stations: station_runtime,
        buffers: catalog
            .buffer_capacities
            .iter()
            .map(|_| VecDeque::new())
            .collect(),
        buffer_peak_occupancy: vec![0; catalog.buffer_capacities.len()],
        buffer_wip_area: vec![0.0; catalog.buffer_capacities.len()],
        source_buffer: VecDeque::new(),
        source_buffer_peak: 0,
        source_buffer_wip_area: 0.0,
        supplier_backlog: VecDeque::new(),
        supplier_backlog_peak: 0,
        supplier_backlog_wip_area: 0.0,
        next_job_id: 0,
        released_jobs_total: 0,
        good_units_total: 0,
        measured_good_units: 0,
        scrapped_jobs_total: 0,
        measured_rework_attempts: 0,
        buffer_capacity_violation: false,
    };
    schedule_event(&mut state, 0, EventKind::SupplyArrival);

    while let Some(event) = state.events.pop() {
        if event.at > catalog.horizon_seconds {
            break;
        }
        advance_time(&mut state, catalog, event.at)?;
        state.now = event.at;
        match event.kind {
            EventKind::SupplyArrival => {
                let id = state.next_job_id;
                state.next_job_id += 1;
                state.released_jobs_total += 1;
                let variant = ((id / catalog.variant_batch_size)
                    % catalog.variant_count as u64) as usize;
                state.job_records.insert(
                    id,
                    JobRecord {
                        job_id: id,
                        variant_id: variant,
                        released_at_seconds: state.now,
                        first_attempt_started_at_by_station: vec![None; catalog.stations.len()],
                        first_attempt_outcome_by_station: vec![None; catalog.stations.len()],
                        attempts_started_by_station: vec![0; catalog.stations.len()],
                        final_good_at_seconds: None,
                        scrapped_at_seconds: None,
                    },
                );
                record_lifecycle_event(
                    &mut state,
                    catalog,
                    JobLifecycleEventKind::JobRelease,
                    id,
                    variant,
                    None,
                    None,
                    None,
                    None,
                );
                state.supplier_backlog.push_back(Job {
                    id,
                    variant,
                    queued_since_seconds: state.now,
                });
                state.supplier_backlog_peak = state
                    .supplier_backlog_peak
                    .max(state.supplier_backlog.len());
                refill_source_buffer(&mut state, catalog);
                let delta = state.supplier_rng.around(
                    catalog.mean_supply_interarrival_seconds,
                    catalog.supply_interarrival_variability_fraction,
                );
                let next_at = state.now.saturating_add(delta);
                if next_at <= catalog.horizon_seconds {
                    schedule_event(&mut state, next_at, EventKind::SupplyArrival);
                }
            }
            EventKind::OperationComplete { station, server } => {
                complete_operation(&mut state, catalog, station, server)?;
            }
            EventKind::RepairComplete { station, server } => {
                let target = state
                    .stations
                    .get_mut(station)
                    .and_then(|runtime| runtime.servers.get_mut(server))
                    .ok_or_else(|| "repair_event_references_missing_server".to_string())?;
                if matches!(&target.state, ServerState::Down) {
                    target.state = ServerState::Idle;
                }
            }
        }
        reconcile(&mut state, catalog)?;
    }
    advance_time(&mut state, catalog, catalog.horizon_seconds)?;
    state.now = catalog.horizon_seconds;
    let censored_jobs: Vec<(u64, usize)> = state.job_records.values()
        .filter(|record| record.final_good_at_seconds.is_none() && record.scrapped_at_seconds.is_none())
        .map(|record| (record.job_id, record.variant_id))
        .collect();
    for (job_id, variant_id) in censored_jobs {
        record_lifecycle_event(
            &mut state,
            catalog,
            JobLifecycleEventKind::JobCensoredAtHorizon,
            job_id,
            variant_id,
            None,
            None,
            None,
            None,
        );
    }

    let measured_seconds = catalog.horizon_seconds - catalog.warmup_seconds;
    let good_per_hour = state.measured_good_units as f64 / measured_seconds as f64 * 3600.0;
    let annualized = good_per_hour * (catalog.productive_seconds_per_year as f64 / 3600.0);
    let mut station_results = Vec::with_capacity(catalog.stations.len());
    let mut time_accounting_within_horizon = true;
    for (index, (station, runtime)) in catalog
        .stations
        .iter()
        .zip(state.stations.iter())
        .enumerate()
    {
        let denominator = measured_seconds as f64 * runtime.servers.len() as f64;
        let busy = safe_ratio(runtime.metrics.busy_seconds, denominator);
        let blocked = safe_ratio(runtime.metrics.blocked_seconds, denominator);
        let starved = safe_ratio(runtime.metrics.starved_seconds, denominator);
        let down = safe_ratio(runtime.metrics.down_seconds, denominator);
        let accounted = busy + blocked + starved + down;
        if accounted > 1.000_001 || denominator <= 0.0 {
            time_accounting_within_horizon = false;
        }
        station_results.push(StationResult {
            station_id: station.station_id.clone(),
            label: station.label.clone(),
            server_count: runtime.servers.len(),
            attempts_started_after_warmup: runtime.metrics.attempts_started_after_warmup,
            successful_attempts_after_warmup: runtime.metrics.successful_attempts_after_warmup,
            rework_attempts_after_warmup: runtime.metrics.rework_attempts_after_warmup,
            scrapped_jobs_after_warmup: runtime.metrics.scrapped_jobs_after_warmup,
            changeovers_started_after_warmup: runtime.metrics.changeovers_started_after_warmup,
            output_transfers_after_warmup: runtime.metrics.output_transfers_after_warmup,
            out_of_order_output_transfers_after_warmup: runtime
                .metrics
                .out_of_order_output_transfers_after_warmup,
            busy_fraction: busy,
            blocked_fraction: blocked,
            starved_fraction: starved,
            down_fraction: down,
            accounted_fraction: accounted,
            added_server_good_output_delta_fraction: extra_server_at
                .filter(|added| *added == index)
                .map(|_| 0.0),
        });
    }

    let interstation_buffers: Vec<BufferResult> = catalog
        .buffer_capacities
        .iter()
        .enumerate()
        .map(|(index, capacity)| BufferResult {
            after_station_id: catalog.stations[index].station_id.clone(),
            capacity_units: *capacity,
            peak_occupancy_units: state.buffer_peak_occupancy[index],
            average_occupancy_units: safe_ratio(
                state.buffer_wip_area[index],
                measured_seconds as f64,
            ),
        })
        .collect();
    let all_buffers_respected_capacity = !state.buffer_capacity_violation
        && interstation_buffers
            .iter()
            .all(|buffer| buffer.peak_occupancy_units <= buffer.capacity_units)
        && state.source_buffer_peak <= catalog.source_buffer_capacity;
    let output_never_exceeds_released_input = state.good_units_total <= state.released_jobs_total;
    let baseline_id = state.scenario_id.clone();
    let lifecycle_summary = build_job_lifecycle_summary(&state, catalog)?;
    Ok(ScenarioResult {
        scenario_id: baseline_id,
        job_lifecycle_summary: lifecycle_summary,
        job_lifecycle_events: std::mem::take(&mut state.lifecycle_events),
        intervention_station_id: extra_server_at
            .map(|index| catalog.stations[index].station_id.clone()),
        added_servers: if extra_server_at.is_some() { 1 } else { 0 },
        released_jobs_total: state.released_jobs_total,
        good_units_total: state.good_units_total,
        measured_good_units: state.measured_good_units,
        scrapped_jobs_total: state.scrapped_jobs_total,
        measured_rework_attempts: state.measured_rework_attempts,
        measured_good_units_per_hour: good_per_hour,
        annualized_good_units_at_declared_operating_hours: annualized,
        supplier_backlog_peak_units: state.supplier_backlog_peak,
        supplier_backlog_average_units: safe_ratio(
            state.supplier_backlog_wip_area,
            measured_seconds as f64,
        ),
        source_buffer_peak_units: state.source_buffer_peak,
        source_buffer_average_units: safe_ratio(
            state.source_buffer_wip_area,
            measured_seconds as f64,
        ),
        interstation_buffers,
        stations: station_results,
        output_never_exceeds_released_input,
        all_buffers_respected_capacity,
        station_time_accounting_within_horizon: time_accounting_within_horizon,
        release_ready: false,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn safe_ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

fn schedule_event(state: &mut SimState, at: u64, kind: EventKind) {
    let sequence = state.next_event_sequence;
    state.next_event_sequence = state.next_event_sequence.wrapping_add(1);
    state.events.push(TimedEvent { at, sequence, kind });
}

fn advance_time(state: &mut SimState, catalog: &Catalog, next_time: u64) -> Result<(), String> {
    if next_time < state.now {
        return Err("event_clock_moved_backwards".into());
    }
    let start = state.now.max(catalog.warmup_seconds);
    let end = next_time.min(catalog.horizon_seconds);
    let dt = end.saturating_sub(start) as f64;
    if dt <= 0.0 {
        return Ok(());
    }
    for (index, runtime) in state.stations.iter_mut().enumerate() {
        let input_empty = if index == 0 {
            state.source_buffer.is_empty()
        } else {
            state.buffers[index - 1].is_empty()
        };
        for server in &runtime.servers {
            match &server.state {
                ServerState::Busy { .. } => runtime.metrics.busy_seconds += dt,
                ServerState::WaitingOutput { .. } => runtime.metrics.blocked_seconds += dt,
                ServerState::Down => runtime.metrics.down_seconds += dt,
                ServerState::Idle if input_empty => runtime.metrics.starved_seconds += dt,
                ServerState::Idle => {}
            }
        }
    }
    for (area, queue) in state.buffer_wip_area.iter_mut().zip(state.buffers.iter()) {
        *area += dt * queue.len() as f64;
    }
    state.source_buffer_wip_area += dt * state.source_buffer.len() as f64;
    state.supplier_backlog_wip_area += dt * state.supplier_backlog.len() as f64;
    Ok(())
}

fn refill_source_buffer(state: &mut SimState, catalog: &Catalog) {
    while state.source_buffer.len() < catalog.source_buffer_capacity {
        match state.supplier_backlog.pop_front() {
            Some(job) => {
                let job_id = job.id;
                let variant_id = job.variant;
                state.source_buffer.push_back(job);
                record_lifecycle_event(
                    state,
                    catalog,
                    JobLifecycleEventKind::BufferEnter,
                    job_id,
                    variant_id,
                    None,
                    None,
                    Some("source-material".into()),
                    None,
                );
            }
            None => break,
        }
    }
    state.source_buffer_peak = state.source_buffer_peak.max(state.source_buffer.len());
}

fn sample_service_seconds(catalog: &Catalog, station: &Station, job_id: u64, attempt: u32) -> u64 {
    DeterministicRng::for_key(
        catalog.seed,
        "service-time",
        &station.station_id,
        job_id,
        attempt,
    )
    .around(
        station.mean_cycle_seconds,
        station.cycle_time_variability_fraction,
    )
}

fn machine_fails_after_job(
    catalog: &Catalog,
    station: &Station,
    job_id: u64,
    attempt: u32,
    service_seconds: u64,
) -> bool {
    let probability =
        1.0 - (-(service_seconds as f64) / station.mean_time_between_failures_seconds).exp();
    DeterministicRng::for_key(
        catalog.seed,
        "machine-failure",
        &station.station_id,
        job_id,
        attempt,
    )
    .unit()
        < probability
}

fn complete_operation(
    state: &mut SimState,
    catalog: &Catalog,
    station_index: usize,
    server_index: usize,
) -> Result<(), String> {
    let previous = {
        let server = state
            .stations
            .get_mut(station_index)
            .and_then(|runtime| runtime.servers.get_mut(server_index))
            .ok_or_else(|| "completion_event_references_missing_server".to_string())?;
        std::mem::replace(&mut server.state, ServerState::Idle)
    };
    let ServerState::Busy {
        job,
        station_attempts,
        service_seconds_for_job,
    } = previous
    else {
        return Err("completion_event_server_not_busy".into());
    };

    let station = &catalog.stations[station_index];
    record_lifecycle_event(
        state,
        catalog,
        JobLifecycleEventKind::StationEnd,
        job.id,
        job.variant,
        Some(station_index),
        Some(server_index),
        None,
        Some(station_attempts),
    );
    let passed = DeterministicRng::for_key(
        catalog.seed,
        "quality-outcome",
        &station.station_id,
        job.id,
        station_attempts,
    )
    .unit()
        < station.good_yield_fraction;
    {
        let record = state.job_records.get_mut(&job.id)
            .ok_or_else(|| format!("job_{}_lifecycle_record_missing_at_quality_outcome", job.id))?;
        if station_attempts == 1 {
            if record.first_attempt_outcome_by_station[station_index].is_some() {
                return Err(format!("job_{}_first_attempt_outcome_duplicated_for_station_{}", job.id, station.station_id));
            }
            record.first_attempt_outcome_by_station[station_index] = Some(passed);
        }
    }
    record_lifecycle_event(
        state,
        catalog,
        if passed { JobLifecycleEventKind::QualityPass } else { JobLifecycleEventKind::QualityFail },
        job.id,
        job.variant,
        Some(station_index),
        Some(server_index),
        None,
        Some(station_attempts),
    );
    if !passed && station_attempts <= station.max_rework_attempts {
        let rework_attempt = station_attempts + 1;
        let rework_seconds = sample_service_seconds(catalog, station, job.id, rework_attempt);
        let record = state.job_records.get_mut(&job.id)
            .ok_or_else(|| format!("job_{}_lifecycle_record_missing_at_rework", job.id))?;
        record.attempts_started_by_station[station_index] += 1;
        record_lifecycle_event(
            state,
            catalog,
            JobLifecycleEventKind::ReworkStart,
            job.id,
            job.variant,
            Some(station_index),
            Some(server_index),
            None,
            Some(rework_attempt),
        );
        let runtime = &mut state.stations[station_index];
        if state.now >= catalog.warmup_seconds {
            runtime.metrics.rework_attempts_after_warmup += 1;
            runtime.metrics.attempts_started_after_warmup += 1;
            state.measured_rework_attempts += 1;
        }
        runtime.servers[server_index].state = ServerState::Busy {
            job,
            station_attempts: rework_attempt,
            service_seconds_for_job: service_seconds_for_job.saturating_add(rework_seconds),
        };
        let completion_at = state.now.saturating_add(rework_seconds);
        schedule_event(
            state,
            completion_at,
            EventKind::OperationComplete {
                station: station_index,
                server: server_index,
            },
        );
        return Ok(());
    }

    let machine_fails = machine_fails_after_job(
        catalog,
        station,
        job.id,
        station_attempts,
        service_seconds_for_job,
    );
    if passed {
        if state.now >= catalog.warmup_seconds {
            state.stations[station_index]
                .metrics
                .successful_attempts_after_warmup += 1;
        }
        state.stations[station_index].servers[server_index].state = ServerState::WaitingOutput {
            job,
            machine_fails_after_release: machine_fails,
        };
    } else {
        let record = state.job_records.get_mut(&job.id)
            .ok_or_else(|| format!("job_{}_lifecycle_record_missing_at_scrap", job.id))?;
        if record.scrapped_at_seconds.replace(state.now).is_some() {
            return Err(format!("job_{}_scrap_terminal_record_duplicated", job.id));
        }
        record_lifecycle_event(
            state,
            catalog,
            JobLifecycleEventKind::JobScrapped,
            job.id,
            job.variant,
            Some(station_index),
            Some(server_index),
            None,
            Some(station_attempts),
        );
        state.scrapped_jobs_total += 1;
        if state.now >= catalog.warmup_seconds {
            state.stations[station_index]
                .metrics
                .scrapped_jobs_after_warmup += 1;
        }
        release_server_after_operation(state, catalog, station_index, server_index, machine_fails);
    }
    Ok(())
}

fn record_lifecycle_event(
    state: &mut SimState,
    catalog: &Catalog,
    kind: JobLifecycleEventKind,
    job_id: u64,
    variant_id: usize,
    station_index: Option<usize>,
    server_index: Option<usize>,
    buffer_id: Option<String>,
    attempt_number: Option<u32>,
) {
    if !state.record_lifecycle_events {
        return;
    }
    let station_id = station_index.map(|index| catalog.stations[index].station_id.clone());
    let resource_id = match (station_id.as_deref(), server_index) {
        (Some(station_id), Some(server_index)) => {
            Some(format!("{station_id}-server-{:02}", server_index + 1))
        }
        _ => None,
    };
    let event = JobLifecycleEvent {
        schema_version: "0.1.0".into(),
        seed: state.seed,
        scenario_id: state.scenario_id.clone(),
        event_sequence: state.next_lifecycle_sequence,
        timestamp_seconds: state.now,
        kind,
        job_id,
        variant_id,
        station_id,
        resource_id,
        buffer_id,
        attempt_number,
        evidence_class: "synthetic".into(),
    };
    state.next_lifecycle_sequence = state.next_lifecycle_sequence.saturating_add(1);
    state.lifecycle_events.push(event);
}

fn build_job_lifecycle_summary(
    state: &SimState,
    catalog: &Catalog,
) -> Result<JobLifecycleSummary, String> {
    let mut eligible_released_jobs = 0_u64;
    let mut accepted_good_jobs = 0_u64;
    let mut scrapped_jobs = 0_u64;
    let mut right_censored_jobs = 0_u64;
    let mut lead_times = Vec::<f64>::new();
    let mut first_pass: Vec<StationFirstPassYield> = catalog.stations.iter()
        .map(|station| StationFirstPassYield {
            station_id: station.station_id.clone(),
            first_attempts_started_in_cohort: 0,
            first_attempts_with_observed_outcome: 0,
            first_attempt_passed: 0,
            first_attempt_failed: 0,
            unresolved_first_attempts_at_horizon: 0,
            observed_first_pass_yield_fraction: None,
        })
        .collect();

    for record in state.job_records.values() {
        if record.released_at_seconds < catalog.warmup_seconds {
            continue;
        }
        eligible_released_jobs += 1;
        if let Some(completed_at) = record.final_good_at_seconds {
            accepted_good_jobs += 1;
            if completed_at >= record.released_at_seconds {
                lead_times.push((completed_at - record.released_at_seconds) as f64);
            } else {
                return Err(format!("job_{}_final_good_timestamp_precedes_release", record.job_id));
            }
        } else if record.scrapped_at_seconds.is_some() {
            scrapped_jobs += 1;
        } else {
            right_censored_jobs += 1;
        }

        for station_index in 0..catalog.stations.len() {
            let Some(first_start) = record.first_attempt_started_at_by_station[station_index] else {
                continue;
            };
            if first_start < catalog.warmup_seconds {
                continue;
            }
            let station = &mut first_pass[station_index];
            station.first_attempts_started_in_cohort += 1;
            match record.first_attempt_outcome_by_station[station_index] {
                Some(true) => {
                    station.first_attempts_with_observed_outcome += 1;
                    station.first_attempt_passed += 1;
                }
                Some(false) => {
                    station.first_attempts_with_observed_outcome += 1;
                    station.first_attempt_failed += 1;
                }
                None => station.unresolved_first_attempts_at_horizon += 1,
            }
        }
    }
    for station in &mut first_pass {
        if station.first_attempts_with_observed_outcome > 0 {
            station.observed_first_pass_yield_fraction = Some(
                station.first_attempt_passed as f64
                    / station.first_attempts_with_observed_outcome as f64
            );
        }
    }
    let (lead_time_summary, median, p95) = if lead_times.is_empty() {
        (None, None, None)
    } else {
        let summary = summarize_distribution(&lead_times)?;
        (Some(summary.clone()), Some(summary.median), Some(summary.p95))
    };
    Ok(JobLifecycleSummary {
        cohort_definition: "jobs released at or after warmup_seconds; good-job lead time is final-good transfer timestamp minus release timestamp; jobs still nonterminal at horizon are right-censored; first-pass yield is based on observed first-attempt outcomes whose first attempt began at or after warm-up".into(),
        eligible_released_jobs_after_warmup: eligible_released_jobs,
        accepted_good_jobs_in_cohort: accepted_good_jobs,
        scrapped_jobs_in_cohort: scrapped_jobs,
        right_censored_jobs_at_horizon: right_censored_jobs,
        good_job_lead_time_seconds: lead_time_summary,
        median_good_job_lead_time_seconds: median,
        p95_good_job_lead_time_seconds: p95,
        station_first_pass_yield: first_pass,
        event_count_in_ledger: state.lifecycle_events.len(),
    })
}

fn record_output_transfer(
    metrics: &mut StationMetrics,
    last_output_job_id: &mut Option<u64>,
    job_id: u64,
    is_measured: bool,
) {
    if !is_measured {
        return;
    }
    metrics.output_transfers_after_warmup += 1;
    if last_output_job_id.is_some_and(|last| job_id < last) {
        metrics.out_of_order_output_transfers_after_warmup += 1;
    }
    *last_output_job_id = Some(last_output_job_id.map_or(job_id, |last| last.max(job_id)));
}

fn dispatch_pop(
    queue: &mut VecDeque<Job>,
    policy: DispatchPolicy,
    last_variant: Option<usize>,
    now_seconds: u64,
    max_variant_skip_seconds: u64,
) -> Option<Job> {
    if policy == DispatchPolicy::MinimizeChangeover {
        if let Some(oldest) = queue.front() {
            let oldest_wait_seconds = now_seconds.saturating_sub(oldest.queued_since_seconds);
            if oldest_wait_seconds < max_variant_skip_seconds {
                if let Some(variant) = last_variant {
                    if let Some(index) = queue.iter().position(|job| job.variant == variant) {
                        return queue.remove(index);
                    }
                }
            }
        }
    }
    // FIFO fallback plus oldest-job aging prevents indefinite skipping.
    queue.pop_front()
}

fn reconcile(state: &mut SimState, catalog: &Catalog) -> Result<(), String> {
    loop {
        let mut changed = false;
        refill_source_buffer(state, catalog);

        // Completed stations transfer in downstream-to-upstream order. A full buffer
        // leaves the machine holding the job in WaitingOutput (blocked).
        for station_index in (0..catalog.stations.len()).rev() {
            let server_count = state.stations[station_index].servers.len();
            for server_index in 0..server_count {
                let waiting = matches!(
                    &state.stations[station_index].servers[server_index].state,
                    ServerState::WaitingOutput { .. }
                );
                if !waiting {
                    continue;
                }
                let downstream_has_room = station_index + 1 == catalog.stations.len()
                    || state.buffers[station_index].len()
                        < catalog.buffer_capacities[station_index];
                if !downstream_has_room {
                    continue;
                }
                let previous = std::mem::replace(
                    &mut state.stations[station_index].servers[server_index].state,
                    ServerState::Idle,
                );
                let ServerState::WaitingOutput {
                    job,
                    machine_fails_after_release,
                } = previous
                else {
                    return Err("blocked_transfer_state_changed_unexpectedly".into());
                };
                {
                    let runtime = &mut state.stations[station_index];
                    record_output_transfer(
                        &mut runtime.metrics,
                        &mut runtime.last_output_job_id,
                        job.id,
                        state.now >= catalog.warmup_seconds,
                    );
                }
                if station_index + 1 == catalog.stations.len() {
                    state.good_units_total += 1;
                    if state.now >= catalog.warmup_seconds {
                        state.measured_good_units += 1;
                    }
                    let record = state.job_records.get_mut(&job.id)
                        .ok_or_else(|| format!("job_{}_lifecycle_record_missing_at_final_good", job.id))?;
                    if record.final_good_at_seconds.replace(state.now).is_some() {
                        return Err(format!("job_{}_final_good_terminal_record_duplicated", job.id));
                    }
                    record_lifecycle_event(
                        state,
                        catalog,
                        JobLifecycleEventKind::FinalGoodCompletion,
                        job.id,
                        job.variant,
                        Some(station_index),
                        Some(server_index),
                        None,
                        None,
                    );
                } else {
                    record_lifecycle_event(
                        state,
                        catalog,
                        JobLifecycleEventKind::BufferEnter,
                        job.id,
                        job.variant,
                        Some(station_index),
                        Some(server_index),
                        Some(format!("buffer-after-{}", catalog.stations[station_index].station_id)),
                        None,
                    );
                    let mut queued_job = job;
                    queued_job.queued_since_seconds = state.now;
                    state.buffers[station_index].push_back(queued_job);
                    let length = state.buffers[station_index].len();
                    state.buffer_peak_occupancy[station_index] =
                        state.buffer_peak_occupancy[station_index].max(length);
                    if length > catalog.buffer_capacities[station_index] {
                        state.buffer_capacity_violation = true;
                    }
                }
                release_server_after_operation(
                    state,
                    catalog,
                    station_index,
                    server_index,
                    machine_fails_after_release,
                );
                changed = true;
            }
        }

        // Start work from downstream to upstream, immediately freeing bounded buffers.
        for station_index in (0..catalog.stations.len()).rev() {
            let server_count = state.stations[station_index].servers.len();
            for server_index in 0..server_count {
                if !matches!(
                    &state.stations[station_index].servers[server_index].state,
                    ServerState::Idle
                ) {
                    continue;
                }
                let last_variant = state.stations[station_index].servers[server_index].last_variant;
                let job = if station_index == 0 {
                    let value = dispatch_pop(
                        &mut state.source_buffer,
                        catalog.dispatch_policy,
                        last_variant,
                        state.now,
                        catalog.max_variant_skip_seconds,
                    );
                    if value.is_some() {
                        refill_source_buffer(state, catalog);
                    }
                    value
                } else {
                    dispatch_pop(
                        &mut state.buffers[station_index - 1],
                        catalog.dispatch_policy,
                        last_variant,
                        state.now,
                        catalog.max_variant_skip_seconds,
                    )
                };
                let Some(job) = job else {
                    continue;
                };
                let input_buffer_id = if station_index == 0 {
                    "source-material".to_string()
                } else {
                    format!("buffer-after-{}", catalog.stations[station_index - 1].station_id)
                };
                record_lifecycle_event(
                    state,
                    catalog,
                    JobLifecycleEventKind::BufferExit,
                    job.id,
                    job.variant,
                    Some(station_index),
                    Some(server_index),
                    Some(input_buffer_id),
                    None,
                );
                let attempt_number = {
                    let job_record = state.job_records.get_mut(&job.id)
                        .ok_or_else(|| format!("job_{}_lifecycle_record_missing_at_station_start", job.id))?;
                    if job_record.attempts_started_by_station[station_index] == 0 {
                        job_record.first_attempt_started_at_by_station[station_index] = Some(state.now);
                    }
                    job_record.attempts_started_by_station[station_index] += 1;
                    job_record.attempts_started_by_station[station_index]
                };
                record_lifecycle_event(
                    state,
                    catalog,
                    JobLifecycleEventKind::StationStart,
                    job.id,
                    job.variant,
                    Some(station_index),
                    Some(server_index),
                    None,
                    Some(attempt_number),
                );
                let station = &catalog.stations[station_index];
                let mut service_seconds = sample_service_seconds(catalog, station, job.id, 1);
                let server = &mut state.stations[station_index].servers[server_index];
                let has_changeover = server.last_variant.is_some_and(|last| last != job.variant);
                if has_changeover {
                    service_seconds =
                        service_seconds.saturating_add(station.changeover_seconds.round() as u64);
                }
                server.last_variant = Some(job.variant);
                server.state = ServerState::Busy {
                    job,
                    station_attempts: 1,
                    service_seconds_for_job: service_seconds,
                };
                if state.now >= catalog.warmup_seconds {
                    state.stations[station_index]
                        .metrics
                        .attempts_started_after_warmup += 1;
                    if has_changeover {
                        state.stations[station_index]
                            .metrics
                            .changeovers_started_after_warmup += 1;
                    }
                }
                let completion_at = state.now.saturating_add(service_seconds);
                schedule_event(
                    state,
                    completion_at,
                    EventKind::OperationComplete {
                        station: station_index,
                        server: server_index,
                    },
                );
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }
    Ok(())
}

fn release_server_after_operation(
    state: &mut SimState,
    catalog: &Catalog,
    station_index: usize,
    server_index: usize,
    machine_fails: bool,
) {
    if machine_fails {
        state.stations[station_index].servers[server_index].state = ServerState::Down;
        let repair_seconds = catalog.stations[station_index]
            .mean_time_to_repair_seconds
            .round()
            .max(1.0) as u64;
        let repair_at = state.now.saturating_add(repair_seconds);
        schedule_event(
            state,
            repair_at,
            EventKind::RepairComplete {
                station: station_index,
                server: server_index,
            },
        );
    } else {
        state.stations[station_index].servers[server_index].state = ServerState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Catalog {
        serde_json::from_str(include_str!(
            "../../fixtures/ev_factory_simulation_v0_1.json"
        ))
        .expect("fixture parses")
    }

    #[test]
    fn keyed_random_streams_are_reproducible_and_independent_of_unrelated_draws() {
        let expected_job_b =
            DeterministicRng::for_key(20261020, "service-time", "battery-module", 42, 1).next_u64();
        let mut unrelated_job =
            DeterministicRng::for_key(20261020, "service-time", "battery-module", 41, 1);
        for _ in 0..100 {
            let _ = unrelated_job.next_u64();
        }
        let actual_job_b =
            DeterministicRng::for_key(20261020, "service-time", "battery-module", 42, 1).next_u64();
        assert_eq!(expected_job_b, actual_job_b);
        let job_a =
            DeterministicRng::for_key(20261020, "service-time", "battery-module", 41, 1).next_u64();
        assert_ne!(job_a, expected_job_b);
        assert_ne!(
            DeterministicRng::for_key(20261020, "service-time", "battery-module", 42, 1).next_u64(),
            DeterministicRng::for_key(20261020, "quality-outcome", "battery-module", 42, 1)
                .next_u64()
        );
    }

    #[test]
    fn paired_replication_statistics_have_expected_shape_and_intervals() {
        let catalog = fixture();
        let report = evaluate_catalog(&catalog).expect("replicated simulation");
        assert_eq!(report.replication_count, 20);
        assert_eq!(report.replication_seeds.len(), 20);
        assert_eq!(report.replication_seeds[0], catalog.seed);
        assert_eq!(report.scenario_statistics.len(), report.scenario_count);
        let baseline = &report.scenario_statistics[0];
        assert_eq!(baseline.good_units_per_hour.sample_count, 20);
        assert!(
            baseline
                .paired_good_units_per_hour_delta_vs_baseline
                .is_none()
        );
        for statistic in report.scenario_statistics.iter().skip(1) {
            let delta = statistic
                .paired_good_units_per_hour_delta_vs_baseline
                .as_ref()
                .expect("paired intervention delta");
            assert_eq!(delta.sample_count, 20);
            assert!(delta.mean_confidence_95_lower.is_some());
            assert!(delta.mean_confidence_95_upper.is_some());
        }
        assert!(report.output_never_exceeds_released_input);
        assert!(report.all_buffers_respected_capacity);
        assert!(report.station_time_accounting_within_horizon);
        assert!(!report.release_ready);
    }

    fn job(id: u64, variant: usize, queued_since_seconds: u64) -> Job {
        Job {
            id,
            variant,
            queued_since_seconds,
        }
    }

    #[test]
    fn fifo_policy_never_reorders_input_queue() {
        let mut queue = VecDeque::from([job(1, 0, 0), job(2, 1, 1), job(3, 0, 2)]);
        assert_eq!(
            dispatch_pop(&mut queue, DispatchPolicy::Fifo, Some(1), 3, 10).map(|item| item.id),
            Some(1)
        );
        assert_eq!(
            queue.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn minimize_changeover_selects_oldest_matching_variant_with_fifo_ties() {
        let mut queue = VecDeque::from([job(1, 0, 0), job(2, 1, 2), job(3, 1, 3), job(4, 0, 4)]);
        assert_eq!(
            dispatch_pop(
                &mut queue,
                DispatchPolicy::MinimizeChangeover,
                Some(1),
                5,
                20,
            )
            .map(|item| item.id),
            Some(2)
        );
        assert_eq!(
            dispatch_pop(
                &mut queue,
                DispatchPolicy::MinimizeChangeover,
                Some(1),
                5,
                20,
            )
            .map(|item| item.id),
            Some(3)
        );
    }

    #[test]
    fn aging_threshold_forces_oldest_job_and_prevents_indefinite_skipping() {
        let mut queue = VecDeque::from([job(1, 0, 0), job(2, 1, 80)]);
        assert_eq!(
            dispatch_pop(
                &mut queue,
                DispatchPolicy::MinimizeChangeover,
                Some(1),
                101,
                100,
            )
            .map(|item| item.id),
            Some(1)
        );
    }

    #[test]
    fn minimize_changeover_policy_is_explicit_and_reproducible() {
        let mut catalog = fixture();
        catalog.dispatch_policy = DispatchPolicy::MinimizeChangeover;
        catalog.max_variant_skip_seconds = 240;
        let first = evaluate_catalog(&catalog).expect("first policy run");
        let second = evaluate_catalog(&catalog).expect("repeat policy run");
        assert_eq!(first, second);
        assert_eq!(first.dispatch_policy, DispatchPolicy::MinimizeChangeover);
        assert!(first.all_buffers_respected_capacity);
        assert!(first.output_never_exceeds_released_input);
        assert!(!first.release_ready);
    }

    #[test]
    fn homogeneous_no_setup_control_makes_policies_equivalent() {
        let mut fifo = fixture();
        fifo.replication_seeds.clear();
        fifo.variant_count = 1;
        fifo.variant_batch_size = 1;
        for station in &mut fifo.stations {
            station.changeover_seconds = 0.0;
        }
        fifo.dispatch_policy = DispatchPolicy::Fifo;
        let mut variant_aware = fifo.clone();
        variant_aware.dispatch_policy = DispatchPolicy::MinimizeChangeover;

        let fifo_report = evaluate_catalog(&fifo).expect("FIFO no-setup control");
        let aware_report =
            evaluate_catalog(&variant_aware).expect("variant-aware no-setup control");
        assert_eq!(fifo_report.scenarios, aware_report.scenarios);
        assert_eq!(
            fifo_report.scenario_statistics,
            aware_report.scenario_statistics
        );
        assert_eq!(
            fifo_report.station_sequence_statistics,
            aware_report.station_sequence_statistics
        );
    }

    #[test]
    fn zero_skip_age_limit_fails_closed() {
        let mut catalog = fixture();
        catalog.max_variant_skip_seconds = 0;
        assert!(validate_catalog(&catalog).is_err());
    }

    #[test]
    fn sequence_inversions_are_counted_at_transfer_not_at_completion() {
        let mut metrics = StationMetrics::default();
        let mut high_water = None;
        record_output_transfer(&mut metrics, &mut high_water, 99, false);
        assert_eq!(metrics.output_transfers_after_warmup, 0);
        assert_eq!(high_water, None);

        for id in [10_u64, 12, 11, 15, 14] {
            record_output_transfer(&mut metrics, &mut high_water, id, true);
        }
        assert_eq!(metrics.output_transfers_after_warmup, 5);
        assert_eq!(metrics.out_of_order_output_transfers_after_warmup, 2);
        assert_eq!(high_water, Some(15));
    }

    #[test]
    fn replication_outcomes_preserve_seed_pairing_and_scenario_identity() {
        let report = evaluate_catalog(&fixture()).expect("replicated simulation");
        assert_eq!(
            report.replication_outcomes.len(),
            20 * report.scenario_count
        );
        for seed in &report.replication_seeds {
            let outcomes: Vec<_> = report
                .replication_outcomes
                .iter()
                .filter(|outcome| outcome.seed == *seed)
                .collect();
            assert_eq!(outcomes.len(), report.scenario_count);
            let baseline = outcomes
                .iter()
                .find(|outcome| outcome.scenario_id == "baseline")
                .expect("per-seed baseline");
            assert!(
                baseline
                    .paired_good_units_per_hour_delta_vs_baseline
                    .is_none()
            );
            for outcome in outcomes
                .iter()
                .filter(|outcome| outcome.scenario_id != "baseline")
            {
                let delta = outcome
                    .paired_good_units_per_hour_delta_vs_baseline
                    .expect("intervention paired delta");
                assert!(
                    (delta - (outcome.good_units_per_hour - baseline.good_units_per_hour)).abs()
                        < 1e-12
                );
                assert_eq!(
                    outcome.baseline_good_units_per_hour,
                    baseline.good_units_per_hour
                );
            }
        }
    }

    #[test]
    fn lifecycle_ledger_replays_terminal_counts_lead_time_and_first_pass_yield() {
        let catalog = fixture();
        let report = evaluate_catalog(&catalog).expect("lifecycle fixture simulation");
        let baseline = report.scenarios.iter()
            .find(|scenario| scenario.scenario_id == "baseline")
            .expect("primary seed baseline");
        let ledger = &baseline.job_lifecycle_events;
        assert!(!ledger.is_empty());
        assert_eq!(baseline.job_lifecycle_summary.event_count_in_ledger, ledger.len());
        assert!(ledger.iter().all(|event| {
            event.schema_version == "0.1.0"
                && event.seed == report.seed
                && event.scenario_id == "baseline"
                && event.evidence_class == "synthetic"
        }));
        assert!(ledger.windows(2).all(|pair| {
            pair[0].timestamp_seconds <= pair[1].timestamp_seconds
                && pair[0].event_sequence < pair[1].event_sequence
        }));
        assert!(ledger.iter().enumerate().all(|(i, event)| event.event_sequence == i as u64));

        let mut releases = BTreeMap::<u64, u64>::new();
        let mut terminal_counts = BTreeMap::<u64, u32>::new();
        let mut lead_times = Vec::<f64>::new();
        let mut first_starts = BTreeMap::<(u64, String), u64>::new();
        let mut first_outcomes = BTreeMap::<(u64, String), bool>::new();
        for event in ledger {
            match event.kind {
                JobLifecycleEventKind::JobRelease => {
                    assert!(releases.insert(event.job_id, event.timestamp_seconds).is_none());
                }
                JobLifecycleEventKind::FinalGoodCompletion => {
                    *terminal_counts.entry(event.job_id).or_default() += 1;
                    let released_at = *releases.get(&event.job_id).expect("release precedes completion");
                    assert!(event.timestamp_seconds >= released_at);
                    if released_at >= catalog.warmup_seconds {
                        lead_times.push((event.timestamp_seconds - released_at) as f64);
                    }
                }
                JobLifecycleEventKind::JobScrapped | JobLifecycleEventKind::JobCensoredAtHorizon => {
                    *terminal_counts.entry(event.job_id).or_default() += 1;
                }
                JobLifecycleEventKind::StationStart if event.attempt_number == Some(1) => {
                    first_starts.insert(
                        (event.job_id, event.station_id.clone().expect("start station")),
                        event.timestamp_seconds,
                    );
                }
                JobLifecycleEventKind::QualityPass | JobLifecycleEventKind::QualityFail
                    if event.attempt_number == Some(1) =>
                {
                    let station_id = event.station_id.clone().expect("quality station");
                    first_outcomes.insert(
                        (event.job_id, station_id),
                        event.kind == JobLifecycleEventKind::QualityPass,
                    );
                }
                _ => {}
            }
        }
        assert_eq!(releases.len() as u64, baseline.released_jobs_total);
        assert!(releases.keys().all(|job_id| terminal_counts.get(job_id) == Some(&1)));
        assert_eq!(terminal_counts.len(), releases.len());

        let summary = &baseline.job_lifecycle_summary;
        assert_eq!(
            summary.eligible_released_jobs_after_warmup,
            summary.accepted_good_jobs_in_cohort
                + summary.scrapped_jobs_in_cohort
                + summary.right_censored_jobs_at_horizon
        );
        let lead_summary = summary.good_job_lead_time_seconds.as_ref().expect("good lead times");
        let recomputed_leads = summarize_distribution(&lead_times).expect("ledger-derived lead times");
        assert_eq!(lead_summary.sample_count, recomputed_leads.sample_count);
        assert!((lead_summary.mean - recomputed_leads.mean).abs() < 1e-12);
        assert!((lead_summary.median - recomputed_leads.median).abs() < 1e-12);
        assert!((lead_summary.p95 - recomputed_leads.p95).abs() < 1e-12);

        for station_summary in &summary.station_first_pass_yield {
            let station_id = station_summary.station_id.clone();
            let eligible_starts: Vec<_> = first_starts.iter()
                .filter(|((_, station), timestamp)| station == &station_id
                    && **timestamp >= catalog.warmup_seconds)
                .map(|(key, _)| key)
                .collect();
            let outcomes: Vec<bool> = eligible_starts.iter()
                .filter_map(|key| first_outcomes.get(*key).copied())
                .collect();
            let passed = outcomes.iter().filter(|passed| **passed).count() as u64;
            let failed = outcomes.len() as u64 - passed;
            assert_eq!(
                station_summary.first_attempts_started_in_cohort,
                eligible_starts.len() as u64
            );
            assert_eq!(station_summary.first_attempts_with_observed_outcome, outcomes.len() as u64);
            assert_eq!(station_summary.first_attempt_passed, passed);
            assert_eq!(station_summary.first_attempt_failed, failed);
            assert_eq!(
                station_summary.unresolved_first_attempts_at_horizon,
                eligible_starts.len() as u64 - outcomes.len() as u64
            );
            let expected_yield = if outcomes.is_empty() {
                None
            } else {
                Some(passed as f64 / outcomes.len() as f64)
            };
            assert_eq!(station_summary.observed_first_pass_yield_fraction, expected_yield);
        }
    }

    #[test]
    fn sequence_metrics_are_reported_at_the_buffer_transfer_boundary() {
        let report = evaluate_catalog(&fixture()).expect("replicated simulation");
        assert_eq!(
            report.station_sequence_statistics.len(),
            report.scenario_count * 5
        );
        assert!(report.station_sequence_statistics.iter().all(|stat| {
            stat.changeovers_started.sample_count == 20
                && stat.output_transfers.sample_count == 20
                && stat.out_of_order_output_transfers.sample_count == 20
                && stat.out_of_order_output_transfers.mean <= stat.output_transfers.mean
        }));
        assert!(
            report.station_sequence_statistics.iter().any(|stat| {
                stat.scenario_id == "baseline" && stat.changeovers_started.mean > 0.0
            })
        );
    }

    #[test]
    fn one_replication_does_not_claim_confidence_interval() {
        let mut catalog = fixture();
        catalog.replication_seeds.clear();
        let report = evaluate_catalog(&catalog).expect("single-seed simulation");
        assert_eq!(report.replication_count, 1);
        assert_eq!(
            report.scenario_statistics[0]
                .good_units_per_hour
                .sample_count,
            1
        );
        assert!(
            report.scenario_statistics[0]
                .good_units_per_hour
                .mean_confidence_95_lower
                .is_none()
        );
        assert!(
            report.scenario_statistics[1]
                .paired_good_units_per_hour_delta_vs_baseline
                .as_ref()
                .expect("paired delta")
                .mean_confidence_95_lower
                .is_none()
        );
    }

    #[test]
    fn distribution_summary_uses_sample_standard_deviation_and_quantiles() {
        let summary = summarize_distribution(&[1.0, 2.0, 3.0]).expect("summary");
        assert!((summary.mean - 2.0).abs() < 1e-12);
        assert!((summary.sample_standard_deviation.unwrap() - 1.0).abs() < 1e-12);
        assert!((summary.median - 2.0).abs() < 1e-12);
        assert!((summary.p05 - 1.1).abs() < 1e-12);
        assert!((summary.p95 - 2.9).abs() < 1e-12);
    }

    #[test]
    fn fixture_is_strictly_valid_and_has_one_buffer_between_stations() {
        let catalog = fixture();
        validate_catalog(&catalog).expect("fixture validates");
        assert_eq!(catalog.buffer_capacities.len(), catalog.stations.len() - 1);
        assert_eq!(catalog.stations.len() + 1, 6);
    }

    #[test]
    fn simulation_is_deterministic_for_a_fixed_seed() {
        let catalog = fixture();
        let first = evaluate_catalog(&catalog).expect("first simulation");
        let second = evaluate_catalog(&catalog).expect("second simulation");
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("serializes"),
            serde_json::to_vec(&second).expect("serializes")
        );
    }

    #[test]
    fn finite_buffers_and_input_output_conservation_hold_for_all_interventions() {
        let catalog = fixture();
        let report = evaluate_catalog(&catalog).expect("simulation");
        assert_eq!(report.scenario_count, catalog.stations.len() + 1);
        assert!(report.output_never_exceeds_released_input);
        assert!(report.all_buffers_respected_capacity);
        assert!(report.station_time_accounting_within_horizon);
        assert!(report.scenarios.iter().all(|scenario| {
            scenario.good_units_total <= scenario.released_jobs_total
                && scenario.all_buffers_respected_capacity
                && scenario.station_time_accounting_within_horizon
                && scenario.release_ready == false
        }));
        for scenario in report.scenarios {
            for station in scenario.stations {
                assert!(station.accounted_fraction <= 1.000_001);
                assert!(station.busy_fraction >= 0.0);
                assert!(station.blocked_fraction >= 0.0);
                assert!(station.starved_fraction >= 0.0);
                assert!(station.down_fraction >= 0.0);
            }
        }
    }

    #[test]
    fn invalid_yield_and_zero_capacity_fail_closed() {
        let mut catalog = fixture();
        catalog.stations[0].good_yield_fraction = 1.2;
        assert!(validate_catalog(&catalog).is_err());

        let mut catalog = fixture();
        catalog.buffer_capacities[0] = 0;
        assert!(validate_catalog(&catalog).is_err());
    }

    #[test]
    fn missing_calibration_or_safety_evidence_never_authorizes_release() {
        let mut catalog = fixture();
        catalog.input_evidence_class = EvidenceClass::Measured;
        catalog.calibration_gate = GateStatus::Passed;
        catalog.local_compliance_gate = GateStatus::Passed;
        catalog.line_safety_gate = GateStatus::Passed;
        catalog.product_quality_release_gate = GateStatus::Passed;
        let report = evaluate_catalog(&catalog).expect("simulation");
        assert!(!report.release_ready);
        assert!(
            report
                .scenarios
                .iter()
                .all(|scenario| !scenario.release_ready)
        );
    }

    #[test]
    fn pcb_cell_route_fixtures_are_deterministic_and_never_release_from_synthetic_evidence() {
        let fixtures = [
            (
                "PCB-CELL-P0-OUTSOURCED-BARE-BOARD-LOCAL-ASSEMBLY",
                include_str!("../../fixtures/pcb_cell_p0_outsourced_bare_board_v0_1.json"),
                5usize,
            ),
            (
                "PCB-CELL-P1-LOCAL-ISOLATION-MILL-AND-ASSEMBLY",
                include_str!("../../fixtures/pcb_cell_p1_local_milling_v0_1.json"),
                7usize,
            ),
        ];

        for (expected_id, fixture_json, expected_stations) in fixtures {
            let catalog: Catalog =
                serde_json::from_str(fixture_json).expect("PCB cell fixture parses");
            validate_catalog(&catalog).expect("PCB cell fixture validates");
            assert_eq!(catalog.benchmark_id, expected_id);
            assert_eq!(catalog.stations.len(), expected_stations);
            assert_eq!(catalog.buffer_capacities.len(), expected_stations - 1);
            assert_eq!(catalog.input_evidence_class, EvidenceClass::Synthetic);
            assert_eq!(catalog.calibration_gate, GateStatus::Unknown);
            assert_eq!(catalog.local_compliance_gate, GateStatus::Unknown);
            assert_eq!(catalog.line_safety_gate, GateStatus::Unknown);
            assert_eq!(catalog.product_quality_release_gate, GateStatus::Unknown);

            let first = evaluate_catalog(&catalog).expect("first PCB cell simulation");
            let second = evaluate_catalog(&catalog).expect("repeat PCB cell simulation");
            assert_eq!(first, second, "fixture {expected_id} is deterministic");
            assert_eq!(first.scenario_count, expected_stations + 1);
            assert!(!first.release_ready);
            assert!(first.output_never_exceeds_released_input);
            assert!(first.all_buffers_respected_capacity);
            assert!(first.station_time_accounting_within_horizon);
            assert!(first.scenarios.iter().all(|scenario| {
                !scenario.release_ready
                    && scenario.good_units_total <= scenario.released_jobs_total
                    && scenario.all_buffers_respected_capacity
                    && scenario.station_time_accounting_within_horizon
            }));
        }
    }
}
