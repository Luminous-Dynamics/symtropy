use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const EPSILON: f64 = 1.0e-9;
const MAX_EVENTS: usize = 100_000;
const CAPACITY_LEVELS_PERCENT: [u32; 20] = [
    5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95, 100,
];
const MIN_DEADLINE_COMPLETION_FRACTION: f64 = 0.95;
const MIN_ENERGY_SERVICE_FRACTION: f64 = 0.95;
const CLAIM_CEILING: &str = "Deterministic fleet charging-contingency screen using declared SOC-banded acceptance and piecewise-constant supply/port availability. Inputs are synthetic unless explicitly sourced; this is not an electrochemical, stochastic-reliability, AC load-flow, control-safety, or field-validation model.";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    claim_ceiling: String,
    vehicle_templates: Vec<VehicleTemplate>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VehicleTemplate {
    template_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    nominal_pack_capacity_kwh: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    temperature_acceptance_fraction: f64,
    charging_efficiency_fraction: f64,
    maximum_port_input_power_kw: f64,
    acceptance_curve: Vec<CurveBand>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurveBand {
    soc_lower_fraction: f64,
    soc_upper_fraction: f64,
    max_battery_acceptance_kw: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    charger_port_count: usize,
    site_power_budget_kw: f64,
    jobs: Vec<Job>,
    capacity_events: Vec<CapacityEvent>,
    #[serde(default)]
    port_release_policy: PortReleasePolicy,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PortReleasePolicy {
    /// Optimistic comparison: connector becomes available as soon as requested energy is delivered.
    #[default]
    ReleaseOnChargeCompletion,
    /// Physically conservative baseline: the vehicle continues occupying its connector until departure.
    HoldUntilDeparture,
}

impl PortReleasePolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReleaseOnChargeCompletion => "release_on_charge_completion",
            Self::HoldUntilDeparture => "hold_until_departure",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    job_id: String,
    template_id: String,
    arrival_minute: f64,
    departure_minute: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapacityEvent {
    start_minute: f64,
    end_minute: f64,
    site_power_budget_kw: f64,
    available_port_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    FirstComeFirstServed,
    EarliestDepartureFirst,
}

impl Policy {
    const ALL: [Self; 2] = [Self::FirstComeFirstServed, Self::EarliestDepartureFirst];

    fn as_str(self) -> &'static str {
        match self {
            Self::FirstComeFirstServed => "fcfs",
            Self::EarliestDepartureFirst => "earliest_departure_first",
        }
    }
}

#[derive(Debug, Clone)]
struct JobState {
    job: Job,
    effective_capacity_kwh: f64,
    requested_battery_energy_kwh: f64,
    current_soc_fraction: f64,
    delivered_battery_energy_kwh: f64,
    site_input_energy_kwh: f64,
    first_start_minute: Option<f64>,
    completion_minute: Option<f64>,
    expired: bool,
    missed_deadline: bool,
    maximum_observed_input_power_kw: f64,
    power_limit_respected: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JobResult {
    job_id: String,
    template_id: String,
    template_evidence_class: String,
    template_evidence_reference: String,
    template_evidence_note: String,
    arrival_minute: f64,
    departure_minute: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
    effective_capacity_kwh: f64,
    requested_battery_energy_kwh: f64,
    delivered_battery_energy_kwh: f64,
    unserved_battery_energy_kwh: f64,
    site_input_energy_kwh: f64,
    conversion_loss_kwh: f64,
    first_start_minute: Option<f64>,
    completion_minute: Option<f64>,
    queue_wait_minutes: f64,
    completed_by_deadline: bool,
    missed_deadline: bool,
    maximum_observed_input_power_kw: f64,
    maximum_allowed_port_input_power_kw: f64,
    power_limit_respected: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    scenario_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    policy: String,
    port_release_policy: String,
    job_count: usize,
    completed_by_deadline_count: usize,
    missed_deadline_count: usize,
    requested_battery_energy_kwh: f64,
    delivered_battery_energy_kwh: f64,
    unserved_battery_energy_kwh: f64,
    site_input_energy_kwh: f64,
    conversion_loss_kwh: f64,
    energy_balance_residual_kwh: f64,
    energy_service_fraction: f64,
    median_queue_wait_minutes: f64,
    p95_queue_wait_minutes: f64,
    maximum_queue_wait_minutes: f64,
    maximum_site_input_power_kw: f64,
    maximum_declared_site_power_budget_kw: f64,
    site_power_limit_respected: bool,
    individual_power_limits_respected: bool,
    jobs: Vec<JobResult>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
struct CapacityFrontier {
    scenario_id: String,
    label: String,
    evidence_class: String,
    policy: String,
    port_release_policy: String,
    base_site_power_budget_kw: f64,
    minimum_deadline_completion_target: f64,
    minimum_energy_service_target: f64,
    sample_spacing_percent: u32,
    minimum_sampled_feasible_site_power_kw: Option<f64>,
    feasible_sample_count: usize,
    service_monotonicity_violations: usize,
    feasibility_monotonicity_violations: usize,
    points: Vec<CapacityPoint>,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    suite_id: String,
    method: &'static str,
    results: Vec<ScenarioResult>,
    capacity_frontiers: Vec<CapacityFrontier>,
    claim_ceiling: &'static str,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/ev-fleet-resilience/fixtures/ev_fleet_resilience_v0_1.json".to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let corpus: Corpus = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid fleet-resilience JSON: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_corpus(&corpus) {
        eprintln!("invalid fleet-resilience corpus: {error}");
        process::exit(2);
    }

    let templates: BTreeMap<String, VehicleTemplate> = corpus
        .vehicle_templates
        .iter()
        .cloned()
        .map(|template| (template.template_id.clone(), template))
        .collect();
    let mut results = Vec::new();
    for scenario in &corpus.scenarios {
        for policy in Policy::ALL {
            match simulate_scenario(&templates, scenario, policy) {
                Ok(result) => results.push(result),
                Err(error) => {
                    eprintln!(
                        "simulation failed for {} / {}: {error}",
                        scenario.scenario_id,
                        policy.as_str()
                    );
                    process::exit(2);
                }
            }
        }
    }
    results.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then_with(|| a.policy.cmp(&b.policy))
    });
    let mut capacity_frontiers = Vec::new();
    for scenario in &corpus.scenarios {
        for policy in Policy::ALL {
            match build_capacity_frontier(&templates, scenario, policy) {
                Ok(frontier) => capacity_frontiers.push(frontier),
                Err(error) => {
                    eprintln!(
                        "capacity frontier failed for {} / {}: {error}",
                        scenario.scenario_id,
                        policy.as_str()
                    );
                    process::exit(2);
                }
            }
        }
    }
    capacity_frontiers.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then_with(|| a.policy.cmp(&b.policy))
    });
    let report = Report {
        schema_version: corpus.schema_version,
        suite_id: corpus.suite_id,
        method: "event-driven priority scheduling with SOC-band events, max-min site-power sharing, declared capacity contingencies, and an explicit post-charge connector-occupancy policy",
        results,
        capacity_frontiers,
        claim_ceiling: CLAIM_CEILING,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize fleet-resilience report: {error}");
            process::exit(2);
        }
    }
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_version != "0.1.0" || corpus.suite_id != "EV-CHARGE-RESILIENCE-001" {
        return Err("unsupported_schema_or_suite_id".to_string());
    }
    if corpus.claim_ceiling.trim().is_empty()
        || corpus.vehicle_templates.is_empty()
        || corpus.scenarios.is_empty()
    {
        return Err("claim_ceiling_templates_and_scenarios_are_required".to_string());
    }
    let mut template_ids = BTreeSet::new();
    for template in &corpus.vehicle_templates {
        if !template_ids.insert(template.template_id.clone()) {
            return Err(format!("duplicate_template_id: {}", template.template_id));
        }
        validate_template(template)
            .map_err(|error| format!("{}: {error}", template.template_id))?;
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &corpus.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("duplicate_scenario_id: {}", scenario.scenario_id));
        }
        validate_scenario(scenario, &template_ids)
            .map_err(|error| format!("{}: {error}", scenario.scenario_id))?;
    }
    Ok(())
}

fn validate_evidence(class: &str, reference: &str, note: &str) -> Result<(), String> {
    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
        "unknown",
    ]
    .contains(&class)
    {
        return Err("unsupported_evidence_class".to_string());
    }
    if note.trim().is_empty() {
        return Err("evidence_note_must_be_nonempty".to_string());
    }
    if class != "synthetic" && class != "unknown" && reference.trim().is_empty() {
        return Err("non_synthetic_evidence_requires_reference".to_string());
    }
    Ok(())
}

fn validate_template(template: &VehicleTemplate) -> Result<(), String> {
    if template.template_id.trim().is_empty() || template.label.trim().is_empty() {
        return Err("template_id_and_label_must_be_nonempty".to_string());
    }
    validate_evidence(
        &template.evidence_class,
        &template.evidence_reference,
        &template.evidence_note,
    )?;
    for (name, value) in [
        (
            "nominal_pack_capacity_kwh",
            template.nominal_pack_capacity_kwh,
        ),
        (
            "maximum_port_input_power_kw",
            template.maximum_port_input_power_kw,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name}_must_be_finite_and_positive"));
        }
    }
    for (name, value) in [
        (
            "state_of_health_fraction",
            template.state_of_health_fraction,
        ),
        (
            "temperature_capacity_fraction",
            template.temperature_capacity_fraction,
        ),
        (
            "temperature_acceptance_fraction",
            template.temperature_acceptance_fraction,
        ),
        (
            "charging_efficiency_fraction",
            template.charging_efficiency_fraction,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 || value > 1.0 {
            return Err(format!("{name}_must_be_finite_and_in_(0,1]"));
        }
    }
    validate_curve(&template.acceptance_curve)
}

fn validate_curve(curve: &[CurveBand]) -> Result<(), String> {
    if curve.is_empty() {
        return Err("acceptance_curve_must_cover_full_soc_range".to_string());
    }
    let mut expected_lower = 0.0;
    for (index, band) in curve.iter().enumerate() {
        for (name, value) in [
            ("soc_lower_fraction", band.soc_lower_fraction),
            ("soc_upper_fraction", band.soc_upper_fraction),
            ("max_battery_acceptance_kw", band.max_battery_acceptance_kw),
        ] {
            if !value.is_finite() {
                return Err(format!("curve_band_{index}_{name}_must_be_finite"));
            }
        }
        if band.soc_lower_fraction < 0.0
            || band.soc_upper_fraction > 1.0
            || band.soc_upper_fraction <= band.soc_lower_fraction
            || band.max_battery_acceptance_kw <= 0.0
        {
            return Err(format!("curve_band_{index}_has_invalid_bounds_or_power"));
        }
        if (band.soc_lower_fraction - expected_lower).abs() > 1.0e-8 {
            return Err(format!("curve_band_{index}_gap_overlap_or_unsorted_curve"));
        }
        expected_lower = band.soc_upper_fraction;
    }
    if (expected_lower - 1.0).abs() > 1.0e-8 {
        return Err("acceptance_curve_must_end_at_soc_1".to_string());
    }
    Ok(())
}

fn validate_scenario(scenario: &Scenario, template_ids: &BTreeSet<String>) -> Result<(), String> {
    if scenario.scenario_id.trim().is_empty() || scenario.label.trim().is_empty() {
        return Err("scenario_id_and_label_must_be_nonempty".to_string());
    }
    validate_evidence(
        &scenario.evidence_class,
        &scenario.evidence_reference,
        &scenario.evidence_note,
    )?;
    if !scenario.site_power_budget_kw.is_finite() || scenario.site_power_budget_kw <= 0.0 {
        return Err("site_power_budget_kw_must_be_finite_and_positive".to_string());
    }
    if scenario.jobs.is_empty() {
        return Err("scenario_must_contain_at_least_one_job".to_string());
    }
    let mut job_ids = BTreeSet::new();
    for job in &scenario.jobs {
        if job.job_id.trim().is_empty() || !job_ids.insert(job.job_id.clone()) {
            return Err(format!("empty_or_duplicate_job_id: {}", job.job_id));
        }
        if !template_ids.contains(&job.template_id) {
            return Err(format!("unknown_template_id_for_job_{}", job.job_id));
        }
        if !job.arrival_minute.is_finite()
            || !job.departure_minute.is_finite()
            || job.arrival_minute < 0.0
            || job.departure_minute <= job.arrival_minute
        {
            return Err(format!(
                "invalid_arrival_or_departure_for_job_{}",
                job.job_id
            ));
        }
        if !job.initial_soc_fraction.is_finite()
            || !job.target_soc_fraction.is_finite()
            || job.initial_soc_fraction < 0.0
            || job.initial_soc_fraction > 1.0
            || job.target_soc_fraction < job.initial_soc_fraction
            || job.target_soc_fraction > 1.0
        {
            return Err(format!("invalid_soc_bounds_for_job_{}", job.job_id));
        }
    }
    let mut events: Vec<&CapacityEvent> = scenario.capacity_events.iter().collect();
    events.sort_by(|a, b| {
        a.start_minute
            .partial_cmp(&b.start_minute)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                a.end_minute
                    .partial_cmp(&b.end_minute)
                    .unwrap_or(Ordering::Equal)
            })
    });
    let mut previous_end = 0.0;
    for (index, event) in events.iter().enumerate() {
        if !event.start_minute.is_finite()
            || !event.end_minute.is_finite()
            || !event.site_power_budget_kw.is_finite()
            || event.start_minute < 0.0
            || event.end_minute <= event.start_minute
            || event.site_power_budget_kw < 0.0
        {
            return Err(format!(
                "capacity_event_{index}_has_invalid_bounds_or_budget"
            ));
        }
        if event.site_power_budget_kw > scenario.site_power_budget_kw + EPSILON
            || event.available_port_count > scenario.charger_port_count
        {
            return Err(format!(
                "capacity_event_{index}_exceeds_declared_base_capacity"
            ));
        }
        if index > 0 && event.start_minute < previous_end - EPSILON {
            return Err(format!("capacity_event_{index}_overlaps_previous_event"));
        }
        previous_end = event.end_minute;
    }
    Ok(())
}

fn simulate_scenario(
    templates: &BTreeMap<String, VehicleTemplate>,
    scenario: &Scenario,
    policy: Policy,
) -> Result<ScenarioResult, String> {
    let mut states: Vec<JobState> = scenario
        .jobs
        .iter()
        .map(|job| {
            let template = templates
                .get(&job.template_id)
                .ok_or_else(|| format!("unknown_template_id: {}", job.template_id))?;
            let capacity = template.nominal_pack_capacity_kwh
                * template.state_of_health_fraction
                * template.temperature_capacity_fraction;
            let requested = capacity * (job.target_soc_fraction - job.initial_soc_fraction);
            Ok(JobState {
                job: job.clone(),
                effective_capacity_kwh: capacity,
                requested_battery_energy_kwh: requested,
                current_soc_fraction: job.initial_soc_fraction,
                delivered_battery_energy_kwh: 0.0,
                site_input_energy_kwh: 0.0,
                first_start_minute: None,
                completion_minute: None,
                expired: false,
                missed_deadline: false,
                maximum_observed_input_power_kw: 0.0,
                power_limit_respected: true,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    states.sort_by(|a, b| a.job.job_id.cmp(&b.job.job_id));

    let mut active = BTreeSet::<usize>::new();
    let mut now = 0.0_f64;
    let mut maximum_site_input_power_kw = 0.0_f64;
    let mut site_power_limit_respected = true;

    for _ in 0..MAX_EVENTS {
        for index in 0..states.len() {
            if states[index].completion_minute.is_none()
                && !states[index].expired
                && states[index].job.arrival_minute <= now + EPSILON
                && states[index].requested_battery_energy_kwh <= EPSILON
            {
                states[index].completion_minute = Some(states[index].job.arrival_minute);
            }
        }

        let mut expired_indices = Vec::new();
        for (index, state) in states.iter_mut().enumerate() {
            if state.completion_minute.is_none()
                && !state.expired
                && state.job.departure_minute <= now + EPSILON
            {
                state.expired = true;
                state.missed_deadline = true;
                expired_indices.push(index);
            }
        }
        for index in expired_indices {
            active.remove(&index);
        }
        active.retain(|index| {
            let state = &states[*index];
            !state.expired
                && (state.completion_minute.is_none()
                    || (scenario.port_release_policy == PortReleasePolicy::HoldUntilDeparture
                        && state.job.departure_minute > now + EPSILON))
        });

        if states
            .iter()
            .all(|state| state.completion_minute.is_some() || state.expired)
        {
            break;
        }

        let (available_ports, site_budget_kw) = capacity_at(scenario, now);
        if active.len() > available_ports {
            let mut held: Vec<usize> = active.iter().copied().collect();
            held.sort_by(|a, b| compare_priority(&states[*a], &states[*b], policy));
            let keep: BTreeSet<usize> = held.into_iter().take(available_ports).collect();
            active = keep;
        }

        let mut waiting: Vec<usize> = states
            .iter()
            .enumerate()
            .filter(|(index, state)| {
                !active.contains(index)
                    && state.completion_minute.is_none()
                    && !state.expired
                    && state.job.arrival_minute <= now + EPSILON
                    && state.job.departure_minute > now + EPSILON
            })
            .map(|(index, _)| index)
            .collect();
        waiting.sort_by(|a, b| compare_priority(&states[*a], &states[*b], policy));
        for index in waiting {
            if active.len() >= available_ports {
                break;
            }
            active.insert(index);
        }

        let mut caps = Vec::<(usize, f64)>::new();
        for index in active.iter().copied() {
            let state = &states[index];
            // Completed-but-held vehicles occupy connectors under HoldUntilDeparture,
            // but they receive no allocation and must contribute no electrical load.
            if state.completion_minute.is_some() || state.expired {
                continue;
            }
            let template = templates
                .get(&state.job.template_id)
                .ok_or_else(|| format!("unknown_template_id: {}", state.job.template_id))?;
            let battery_acceptance_kw = acceptance_power_at(template, state.current_soc_fraction)
                * template.temperature_acceptance_fraction;
            let acceptance_input_cap =
                battery_acceptance_kw / template.charging_efficiency_fraction;
            caps.push((
                index,
                template
                    .maximum_port_input_power_kw
                    .min(acceptance_input_cap),
            ));
        }
        let allocations = water_fill(&caps, site_budget_kw);
        let actual_site_input_kw: f64 = allocations.values().sum();
        maximum_site_input_power_kw = maximum_site_input_power_kw.max(actual_site_input_kw);
        if actual_site_input_kw > site_budget_kw + 1.0e-7 {
            site_power_limit_respected = false;
        }

        let mut next_time = next_external_event(scenario, &states, now);
        for (index, input_kw) in &allocations {
            if *input_kw <= EPSILON {
                continue;
            }
            let state = &states[*index];
            let template = templates
                .get(&state.job.template_id)
                .ok_or_else(|| format!("unknown_template_id: {}", state.job.template_id))?;
            let battery_kw = input_kw * template.charging_efficiency_fraction;
            let remaining_kwh =
                (state.requested_battery_energy_kwh - state.delivered_battery_energy_kwh).max(0.0);
            if battery_kw > EPSILON && remaining_kwh > EPSILON {
                let minutes_to_target = remaining_kwh / battery_kw * 60.0;
                let event_time = now + minutes_to_target;
                next_time = Some(next_time.map_or(event_time, |value| value.min(event_time)));
            }
            if battery_kw > EPSILON {
                if let Some(boundary) = next_curve_boundary(
                    template,
                    state.current_soc_fraction,
                    state.job.target_soc_fraction,
                ) {
                    let energy_to_boundary = (boundary - state.current_soc_fraction).max(0.0)
                        * state.effective_capacity_kwh;
                    let minutes_to_boundary = energy_to_boundary / battery_kw * 60.0;
                    if minutes_to_boundary > EPSILON {
                        let event_time = now + minutes_to_boundary;
                        next_time =
                            Some(next_time.map_or(event_time, |value| value.min(event_time)));
                    }
                }
            }
        }

        let Some(next) = next_time else {
            return Err("no_future_event_while_jobs_remain".to_string());
        };
        let delta_minutes = next - now;
        if !delta_minutes.is_finite() || delta_minutes <= EPSILON {
            return Err(format!("non_advancing_event_clock_at_minute_{now}"));
        }

        for (index, input_kw) in allocations {
            if input_kw <= EPSILON {
                continue;
            }
            let template = templates
                .get(&states[index].job.template_id)
                .ok_or_else(|| format!("unknown_template_id: {}", states[index].job.template_id))?;
            let state = &mut states[index];
            if state.first_start_minute.is_none() {
                state.first_start_minute = Some(now);
            }
            state.maximum_observed_input_power_kw =
                state.maximum_observed_input_power_kw.max(input_kw);
            let effective_input_cap = template.maximum_port_input_power_kw.min(
                acceptance_power_at(template, state.current_soc_fraction)
                    * template.temperature_acceptance_fraction
                    / template.charging_efficiency_fraction,
            );
            if input_kw > effective_input_cap + 1.0e-7 {
                state.power_limit_respected = false;
            }
            let requested_remaining =
                (state.requested_battery_energy_kwh - state.delivered_battery_energy_kwh).max(0.0);
            let nominal_site_increment = input_kw * delta_minutes / 60.0;
            let nominal_battery_increment =
                nominal_site_increment * template.charging_efficiency_fraction;
            let battery_increment = nominal_battery_increment.min(requested_remaining);
            let site_increment = if battery_increment + EPSILON < nominal_battery_increment {
                battery_increment / template.charging_efficiency_fraction
            } else {
                nominal_site_increment
            };
            state.site_input_energy_kwh += site_increment;
            state.delivered_battery_energy_kwh += battery_increment;
            state.current_soc_fraction = (state.job.initial_soc_fraction
                + state.delivered_battery_energy_kwh / state.effective_capacity_kwh)
                .min(state.job.target_soc_fraction);
        }

        now = next;
        for state in &mut states {
            if state.completion_minute.is_none()
                && !state.expired
                && state.delivered_battery_energy_kwh + EPSILON
                    >= state.requested_battery_energy_kwh
                && state.job.arrival_minute <= now + EPSILON
            {
                state.delivered_battery_energy_kwh = state
                    .delivered_battery_energy_kwh
                    .min(state.requested_battery_energy_kwh);
                state.current_soc_fraction = state.job.target_soc_fraction;
                state.completion_minute = Some(now);
            }
        }
        active.retain(|index| {
            let state = &states[*index];
            !state.expired
                && (state.completion_minute.is_none()
                    || (scenario.port_release_policy == PortReleasePolicy::HoldUntilDeparture
                        && state.job.departure_minute > now + EPSILON))
        });
    }

    if states
        .iter()
        .any(|state| state.completion_minute.is_none() && !state.expired)
    {
        return Err(format!("event_limit_exceeded_{MAX_EVENTS}"));
    }

    let mut jobs = Vec::with_capacity(states.len());
    for state in &states {
        let template = templates
            .get(&state.job.template_id)
            .ok_or_else(|| format!("unknown_template_id: {}", state.job.template_id))?;
        let unserved =
            (state.requested_battery_energy_kwh - state.delivered_battery_energy_kwh).max(0.0);
        let loss = (state.site_input_energy_kwh - state.delivered_battery_energy_kwh).max(0.0);
        let queue_wait = state
            .first_start_minute
            .map(|start| (start - state.job.arrival_minute).max(0.0))
            .unwrap_or_else(|| {
                if state.requested_battery_energy_kwh <= EPSILON {
                    0.0
                } else {
                    state.job.departure_minute - state.job.arrival_minute
                }
            });
        let completed_by_deadline = state
            .completion_minute
            .is_some_and(|completion| completion <= state.job.departure_minute + EPSILON);
        jobs.push(JobResult {
            job_id: state.job.job_id.clone(),
            template_id: state.job.template_id.clone(),
            template_evidence_class: template.evidence_class.clone(),
            template_evidence_reference: template.evidence_reference.clone(),
            template_evidence_note: template.evidence_note.clone(),
            arrival_minute: state.job.arrival_minute,
            departure_minute: state.job.departure_minute,
            initial_soc_fraction: state.job.initial_soc_fraction,
            target_soc_fraction: state.job.target_soc_fraction,
            effective_capacity_kwh: state.effective_capacity_kwh,
            requested_battery_energy_kwh: state.requested_battery_energy_kwh,
            delivered_battery_energy_kwh: state.delivered_battery_energy_kwh,
            unserved_battery_energy_kwh: unserved,
            site_input_energy_kwh: state.site_input_energy_kwh,
            conversion_loss_kwh: loss,
            first_start_minute: state.first_start_minute,
            completion_minute: state.completion_minute,
            queue_wait_minutes: queue_wait,
            completed_by_deadline,
            missed_deadline: state.missed_deadline,
            maximum_observed_input_power_kw: state.maximum_observed_input_power_kw,
            maximum_allowed_port_input_power_kw: template.maximum_port_input_power_kw,
            power_limit_respected: state.power_limit_respected,
        });
    }
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

    let job_count = jobs.len();
    let completed_by_deadline_count = jobs.iter().filter(|job| job.completed_by_deadline).count();
    let missed_deadline_count = jobs.iter().filter(|job| job.missed_deadline).count();
    let requested_battery_energy_kwh: f64 = jobs
        .iter()
        .map(|job| job.requested_battery_energy_kwh)
        .sum();
    let delivered_battery_energy_kwh: f64 = jobs
        .iter()
        .map(|job| job.delivered_battery_energy_kwh)
        .sum();
    let unserved_battery_energy_kwh: f64 =
        jobs.iter().map(|job| job.unserved_battery_energy_kwh).sum();
    let site_input_energy_kwh: f64 = jobs.iter().map(|job| job.site_input_energy_kwh).sum();
    let conversion_loss_kwh: f64 = jobs.iter().map(|job| job.conversion_loss_kwh).sum();
    let energy_balance_residual_kwh =
        site_input_energy_kwh - delivered_battery_energy_kwh - conversion_loss_kwh;
    let waits: Vec<f64> = jobs.iter().map(|job| job.queue_wait_minutes).collect();
    let maximum_declared_site_power_budget_kw = scenario
        .capacity_events
        .iter()
        .map(|event| event.site_power_budget_kw)
        .fold(scenario.site_power_budget_kw, f64::max);
    let individual_power_limits_respected = jobs.iter().all(|job| {
        job.power_limit_respected
            && job.maximum_observed_input_power_kw
                <= job.maximum_allowed_port_input_power_kw + 1.0e-7
    });

    Ok(ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        evidence_class: scenario.evidence_class.clone(),
        evidence_reference: scenario.evidence_reference.clone(),
        evidence_note: scenario.evidence_note.clone(),
        policy: policy.as_str().to_string(),
        port_release_policy: scenario.port_release_policy.as_str().to_string(),
        job_count,
        completed_by_deadline_count,
        missed_deadline_count,
        requested_battery_energy_kwh,
        delivered_battery_energy_kwh,
        unserved_battery_energy_kwh,
        site_input_energy_kwh,
        conversion_loss_kwh,
        energy_balance_residual_kwh,
        energy_service_fraction: if requested_battery_energy_kwh <= EPSILON {
            1.0
        } else {
            (delivered_battery_energy_kwh / requested_battery_energy_kwh).clamp(0.0, 1.0)
        },
        median_queue_wait_minutes: percentile_nearest_rank(&waits, 0.5),
        p95_queue_wait_minutes: percentile_nearest_rank(&waits, 0.95),
        maximum_queue_wait_minutes: waits.into_iter().fold(0.0, f64::max),
        maximum_site_input_power_kw,
        maximum_declared_site_power_budget_kw,
        site_power_limit_respected,
        individual_power_limits_respected,
        jobs,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn build_capacity_frontier(
    templates: &BTreeMap<String, VehicleTemplate>,
    scenario: &Scenario,
    policy: Policy,
) -> Result<CapacityFrontier, String> {
    let mut points = Vec::with_capacity(CAPACITY_LEVELS_PERCENT.len());
    let mut minimum_feasible = None;
    let mut feasible_sample_count = 0_usize;
    let mut service_monotonicity_violations = 0_usize;
    let mut feasibility_monotonicity_violations = 0_usize;
    let mut previous_completed = None;
    let mut previous_delivered = None;
    let mut previously_feasible = false;

    for percent in CAPACITY_LEVELS_PERCENT {
        let candidate_budget = scenario.site_power_budget_kw * f64::from(percent) / 100.0;
        let mut candidate = scenario.clone();
        candidate.site_power_budget_kw = candidate_budget;
        // Capacity events are treated as fixed physical caps; the candidate interconnection
        // cannot exceed them, and smaller candidate connections are clipped by the cap.
        for event in &mut candidate.capacity_events {
            event.site_power_budget_kw = event.site_power_budget_kw.min(candidate_budget);
        }
        let result = simulate_scenario(templates, &candidate, policy)?;
        let deadline_completion_fraction =
            result.completed_by_deadline_count as f64 / result.job_count as f64;
        let meets_service_target = deadline_completion_fraction + EPSILON
            >= MIN_DEADLINE_COMPLETION_FRACTION
            && result.energy_service_fraction + EPSILON >= MIN_ENERGY_SERVICE_FRACTION
            && result.site_power_limit_respected
            && result.individual_power_limits_respected;
        if meets_service_target {
            feasible_sample_count += 1;
            minimum_feasible.get_or_insert(candidate_budget);
        }
        if previously_feasible && !meets_service_target {
            feasibility_monotonicity_violations += 1;
        }
        if let (Some(prior_completed), Some(prior_delivered)) =
            (previous_completed, previous_delivered)
        {
            if result.completed_by_deadline_count < prior_completed
                || result.delivered_battery_energy_kwh + EPSILON < prior_delivered
            {
                service_monotonicity_violations += 1;
            }
        }
        previously_feasible = meets_service_target;
        previous_completed = Some(result.completed_by_deadline_count);
        previous_delivered = Some(result.delivered_battery_energy_kwh);
        points.push(CapacityPoint {
            installed_site_power_budget_kw: candidate_budget,
            fraction_of_declared_base_capacity: f64::from(percent) / 100.0,
            completed_by_deadline_count: result.completed_by_deadline_count,
            missed_deadline_count: result.missed_deadline_count,
            deadline_completion_fraction,
            energy_service_fraction: result.energy_service_fraction,
            delivered_battery_energy_kwh: result.delivered_battery_energy_kwh,
            unserved_battery_energy_kwh: result.unserved_battery_energy_kwh,
            maximum_site_input_power_kw: result.maximum_site_input_power_kw,
            site_power_limit_respected: result.site_power_limit_respected,
            individual_power_limits_respected: result.individual_power_limits_respected,
            meets_service_target,
        });
    }

    Ok(CapacityFrontier {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        evidence_class: scenario.evidence_class.clone(),
        policy: policy.as_str().to_string(),
        port_release_policy: scenario.port_release_policy.as_str().to_string(),
        base_site_power_budget_kw: scenario.site_power_budget_kw,
        minimum_deadline_completion_target: MIN_DEADLINE_COMPLETION_FRACTION,
        minimum_energy_service_target: MIN_ENERGY_SERVICE_FRACTION,
        sample_spacing_percent: 5,
        minimum_sampled_feasible_site_power_kw: minimum_feasible,
        feasible_sample_count,
        service_monotonicity_violations,
        feasibility_monotonicity_violations,
        points,
    })
}

fn compare_priority(a: &JobState, b: &JobState, policy: Policy) -> Ordering {
    match policy {
        Policy::FirstComeFirstServed => a
            .job
            .arrival_minute
            .partial_cmp(&b.job.arrival_minute)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.job.job_id.cmp(&b.job.job_id)),
        Policy::EarliestDepartureFirst => a
            .job
            .departure_minute
            .partial_cmp(&b.job.departure_minute)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                a.job
                    .arrival_minute
                    .partial_cmp(&b.job.arrival_minute)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| a.job.job_id.cmp(&b.job.job_id)),
    }
}

fn capacity_at(scenario: &Scenario, minute: f64) -> (usize, f64) {
    scenario
        .capacity_events
        .iter()
        .find(|event| event.start_minute <= minute + EPSILON && minute < event.end_minute - EPSILON)
        .map(|event| (event.available_port_count, event.site_power_budget_kw))
        .unwrap_or((scenario.charger_port_count, scenario.site_power_budget_kw))
}

fn next_external_event(scenario: &Scenario, states: &[JobState], now: f64) -> Option<f64> {
    let mut next: Option<f64> = None;
    let mut consider = |candidate: f64| {
        if candidate > now + EPSILON && candidate.is_finite() {
            next = Some(next.map_or(candidate, |current| current.min(candidate)));
        }
    };
    for state in states {
        if state.completion_minute.is_none() && !state.expired {
            consider(state.job.arrival_minute);
            consider(state.job.departure_minute);
        } else if state.completion_minute.is_some()
            && scenario.port_release_policy == PortReleasePolicy::HoldUntilDeparture
        {
            // A completed vehicle's departure is also an external connector-release event.
            consider(state.job.departure_minute);
        }
    }
    for event in &scenario.capacity_events {
        consider(event.start_minute);
        consider(event.end_minute);
    }
    next
}

fn acceptance_power_at(template: &VehicleTemplate, soc: f64) -> f64 {
    template
        .acceptance_curve
        .iter()
        .find(|band| {
            soc >= band.soc_lower_fraction - EPSILON && soc < band.soc_upper_fraction - EPSILON
        })
        .or_else(|| template.acceptance_curve.last())
        .map_or(0.0, |band| band.max_battery_acceptance_kw)
}

fn next_curve_boundary(template: &VehicleTemplate, soc: f64, target: f64) -> Option<f64> {
    template
        .acceptance_curve
        .iter()
        .map(|band| band.soc_upper_fraction)
        .find(|upper| *upper > soc + EPSILON && *upper < target - EPSILON)
}

fn water_fill(caps: &[(usize, f64)], budget_kw: f64) -> BTreeMap<usize, f64> {
    let mut sorted: Vec<(usize, f64)> = caps
        .iter()
        .map(|(index, cap)| (*index, cap.max(0.0)))
        .collect();
    sorted.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut allocations = BTreeMap::new();
    let mut remaining_budget = budget_kw.max(0.0);
    while !sorted.is_empty() {
        let level = remaining_budget / sorted.len() as f64;
        if sorted[0].1 <= level + EPSILON {
            let (index, cap) = sorted.remove(0);
            allocations.insert(index, cap);
            remaining_budget = (remaining_budget - cap).max(0.0);
        } else {
            for (index, cap) in sorted.drain(..) {
                allocations.insert(index, level.min(cap));
            }
        }
    }
    allocations
}

fn percentile_nearest_rank(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let rank = ((percentile.clamp(0.0, 1.0) * sorted.len() as f64).ceil() as usize)
        .max(1)
        .min(sorted.len());
    sorted[rank - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(
        capacity_kwh: f64,
        efficiency: f64,
        port_kw: f64,
        curve: Vec<CurveBand>,
    ) -> VehicleTemplate {
        VehicleTemplate {
            template_id: "test-template".to_string(),
            label: "test".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "hand-calculable fixture".to_string(),
            nominal_pack_capacity_kwh: capacity_kwh,
            state_of_health_fraction: 1.0,
            temperature_capacity_fraction: 1.0,
            temperature_acceptance_fraction: 1.0,
            charging_efficiency_fraction: efficiency,
            maximum_port_input_power_kw: port_kw,
            acceptance_curve: curve,
        }
    }

    fn constant_curve(power_kw: f64) -> Vec<CurveBand> {
        vec![CurveBand {
            soc_lower_fraction: 0.0,
            soc_upper_fraction: 1.0,
            max_battery_acceptance_kw: power_kw,
        }]
    }

    fn job(id: &str, arrival: f64, departure: f64, initial: f64, target: f64) -> Job {
        Job {
            job_id: id.to_string(),
            template_id: "test-template".to_string(),
            arrival_minute: arrival,
            departure_minute: departure,
            initial_soc_fraction: initial,
            target_soc_fraction: target,
        }
    }

    fn scenario(
        jobs: Vec<Job>,
        port_count: usize,
        site_kw: f64,
        events: Vec<CapacityEvent>,
    ) -> Scenario {
        Scenario {
            scenario_id: "test-scenario".to_string(),
            label: "test".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "hand-calculable fixture".to_string(),
            charger_port_count: port_count,
            site_power_budget_kw: site_kw,
            jobs,
            capacity_events: events,
            port_release_policy: PortReleasePolicy::ReleaseOnChargeCompletion,
        }
    }

    fn templates(template: VehicleTemplate) -> BTreeMap<String, VehicleTemplate> {
        [(template.template_id.clone(), template)]
            .into_iter()
            .collect()
    }

    #[test]
    fn constant_power_oracle_has_expected_energy_time_and_loss() {
        let template = template(10.0, 0.9, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 0.0, 90.0, 0.0, 0.9)], 1, 10.0, vec![]);
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        let job = &result.jobs[0];
        assert_eq!(result.completed_by_deadline_count, 1);
        assert!((job.requested_battery_energy_kwh - 9.0).abs() < 1.0e-8);
        assert!((job.delivered_battery_energy_kwh - 9.0).abs() < 1.0e-8);
        assert!((job.site_input_energy_kwh - 10.0).abs() < 1.0e-8);
        assert!((job.conversion_loss_kwh - 1.0).abs() < 1.0e-8);
        assert!((job.completion_minute.unwrap() - 60.0).abs() < 1.0e-7);
        assert!(result.energy_balance_residual_kwh.abs() < 1.0e-8);
    }

    #[test]
    fn soc_taper_boundary_is_an_exact_event() {
        let template = template(
            20.0,
            1.0,
            100.0,
            vec![
                CurveBand {
                    soc_lower_fraction: 0.0,
                    soc_upper_fraction: 0.5,
                    max_battery_acceptance_kw: 10.0,
                },
                CurveBand {
                    soc_lower_fraction: 0.5,
                    soc_upper_fraction: 1.0,
                    max_battery_acceptance_kw: 5.0,
                },
            ],
        );
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 0.0, 120.0, 0.4, 0.8)], 1, 100.0, vec![]);
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert!((result.jobs[0].completion_minute.unwrap() - 84.0).abs() < 1.0e-7);
        assert!((result.jobs[0].site_input_energy_kwh - 8.0).abs() < 1.0e-8);
        assert!((result.maximum_site_input_power_kw - 10.0).abs() < 1.0e-8);
    }

    #[test]
    fn max_min_sharing_never_exceeds_site_budget() {
        let template = template(100.0, 1.0, 10.0, constant_curve(10.0));
        let templates = templates(template);
        let scenario = scenario(
            vec![
                job("a", 0.0, 130.0, 0.0, 0.1),
                job("b", 0.0, 130.0, 0.0, 0.1),
            ],
            2,
            10.0,
            vec![],
        );
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.completed_by_deadline_count, 2);
        assert!(result.site_power_limit_respected);
        assert!(result.maximum_site_input_power_kw <= 10.0 + 1.0e-8);
        assert!(
            result
                .jobs
                .iter()
                .all(|job| job.maximum_observed_input_power_kw <= 10.0 + 1.0e-8)
        );
    }

    #[test]
    fn feeder_outage_pauses_energy_and_resumes_after_boundary() {
        let template = template(20.0, 1.0, 10.0, constant_curve(10.0));
        let templates = templates(template);
        let event = CapacityEvent {
            start_minute: 30.0,
            end_minute: 90.0,
            site_power_budget_kw: 0.0,
            available_port_count: 0,
        };
        let scenario = scenario(vec![job("a", 0.0, 150.0, 0.0, 0.5)], 1, 10.0, vec![event]);
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.completed_by_deadline_count, 1);
        assert!((result.jobs[0].completion_minute.unwrap() - 120.0).abs() < 1.0e-7);
        assert!((result.jobs[0].delivered_battery_energy_kwh - 10.0).abs() < 1.0e-8);
        assert!(result.site_power_limit_respected);
    }

    #[test]
    fn completion_exactly_at_departure_is_not_a_miss() {
        let template = template(10.0, 0.9, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 0.0, 60.0, 0.0, 0.9)], 1, 10.0, vec![]);
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.completed_by_deadline_count, 1);
        assert_eq!(result.missed_deadline_count, 0);
        assert!((result.jobs[0].completion_minute.unwrap() - 60.0).abs() < 1.0e-7);
    }

    #[test]
    fn urgent_job_deadline_oracle_distinguishes_policies() {
        let template = template(100.0, 1.0, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(
            vec![
                job("a-long", 0.0, 120.0, 0.0, 0.1),
                job("b-short", 0.0, 20.0, 0.0, 0.03),
            ],
            1,
            10.0,
            vec![],
        );
        let fcfs = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        let edf = simulate_scenario(&templates, &scenario, Policy::EarliestDepartureFirst).unwrap();
        assert_eq!(fcfs.missed_deadline_count, 1);
        assert_eq!(edf.missed_deadline_count, 0);
        assert_eq!(edf.completed_by_deadline_count, 2);
    }

    #[test]
    fn incomplete_jobs_remain_in_denominator_and_unserved_energy() {
        let template = template(10.0, 0.9, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 0.0, 5.0, 0.0, 0.9)], 1, 10.0, vec![]);
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.job_count, 1);
        assert_eq!(result.missed_deadline_count, 1);
        assert!((result.unserved_battery_energy_kwh - 8.25).abs() < 1.0e-7);
    }

    #[test]
    fn zero_energy_job_is_completed_without_consuming_a_port() {
        let template = template(10.0, 0.9, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 10.0, 20.0, 0.5, 0.5)], 0, 0.0, vec![]);
        let result =
            simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.job_count, 1);
        assert_eq!(result.completed_by_deadline_count, 1);
        assert_eq!(result.jobs[0].delivered_battery_energy_kwh, 0.0);
    }

    #[test]
    fn gapped_soc_curve_is_rejected() {
        let mut template = template(
            20.0,
            1.0,
            10.0,
            vec![
                CurveBand {
                    soc_lower_fraction: 0.0,
                    soc_upper_fraction: 0.4,
                    max_battery_acceptance_kw: 10.0,
                },
                CurveBand {
                    soc_lower_fraction: 0.5,
                    soc_upper_fraction: 1.0,
                    max_battery_acceptance_kw: 5.0,
                },
            ],
        );
        assert_eq!(
            validate_template(&template).unwrap_err(),
            "curve_band_1_gap_overlap_or_unsorted_curve"
        );
        template.acceptance_curve[1].soc_lower_fraction = 0.4;
        assert!(validate_template(&template).is_ok());
    }

    #[test]
    fn overlapping_capacity_events_are_rejected() {
        let templates = templates(template(10.0, 1.0, 10.0, constant_curve(100.0)));
        let mut scenario = scenario(vec![job("a", 0.0, 20.0, 0.0, 0.1)], 1, 10.0, vec![]);
        scenario.capacity_events = vec![
            CapacityEvent {
                start_minute: 10.0,
                end_minute: 30.0,
                site_power_budget_kw: 0.0,
                available_port_count: 0,
            },
            CapacityEvent {
                start_minute: 20.0,
                end_minute: 40.0,
                site_power_budget_kw: 5.0,
                available_port_count: 1,
            },
        ];
        assert!(validate_scenario(&scenario, &templates.keys().cloned().collect()).is_err());
    }

    #[test]
    fn contingency_events_cannot_exceed_installed_base_capacity() {
        let templates = templates(template(10.0, 1.0, 10.0, constant_curve(100.0)));
        let mut case = scenario(vec![job("a", 0.0, 20.0, 0.0, 0.1)], 1, 10.0, vec![]);
        case.capacity_events = vec![CapacityEvent {
            start_minute: 5.0,
            end_minute: 10.0,
            site_power_budget_kw: 11.0,
            available_port_count: 1,
        }];
        let template_ids: BTreeSet<String> = templates.keys().cloned().collect();
        assert_eq!(
            validate_scenario(&case, &template_ids).unwrap_err(),
            "capacity_event_0_exceeds_declared_base_capacity"
        );
        case.capacity_events[0].site_power_budget_kw = 10.0;
        case.capacity_events[0].available_port_count = 2;
        assert_eq!(
            validate_scenario(&case, &template_ids).unwrap_err(),
            "capacity_event_0_exceeds_declared_base_capacity"
        );
    }

    #[test]
    fn one_hundred_job_order_permutations_produce_identical_reports() {
        let template = template(100.0, 0.95, 12.0, constant_curve(80.0));
        let templates = templates(template);
        let base_jobs: Vec<Job> = (0..8)
            .map(|index| {
                job(
                    &format!("job-{index:02}"),
                    ((index % 3) * 5) as f64,
                    100.0 + ((index % 4) * 10) as f64,
                    0.1,
                    0.25 + ((index % 5) as f64 * 0.1),
                )
            })
            .collect();
        let event = CapacityEvent {
            start_minute: 20.0,
            end_minute: 30.0,
            site_power_budget_kw: 7.0,
            available_port_count: 2,
        };
        let baseline = scenario(base_jobs.clone(), 3, 20.0, vec![event.clone()]);

        for policy in Policy::ALL {
            let expected = simulate_scenario(&templates, &baseline, policy).unwrap();
            for seed in 0_u64..100 {
                let mut shuffled = base_jobs.clone();
                let mut state = seed + 1;
                for index in (1..shuffled.len()).rev() {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1);
                    let swap_index = (state as usize) % (index + 1);
                    shuffled.swap(index, swap_index);
                }
                let permutation = scenario(shuffled, 3, 20.0, vec![event.clone()]);
                assert_eq!(
                    simulate_scenario(&templates, &permutation, policy).unwrap(),
                    expected,
                    "policy {} differed for permutation seed {}",
                    policy.as_str(),
                    seed
                );
            }
        }
    }
    #[test]
    fn delivered_service_is_monotone_with_site_capacity_for_fixed_inputs() {
        let template = template(100.0, 0.92, 20.0, constant_curve(80.0));
        let templates = templates(template);
        let jobs = vec![
            job("a-urgent", 0.0, 60.0, 0.10, 0.25),
            job("b-middle", 0.0, 90.0, 0.10, 0.32),
            job("c-late", 15.0, 120.0, 0.15, 0.40),
            job("d-late2", 20.0, 125.0, 0.20, 0.45),
        ];
        for policy in Policy::ALL {
            let mut previous_delivered = 0.0;
            let mut previous_completed = 0;
            for budget_kw in [2.0, 4.0, 8.0, 12.0, 20.0, 30.0] {
                let case = scenario(jobs.clone(), 3, budget_kw, vec![]);
                let result = simulate_scenario(&templates, &case, policy).unwrap();
                assert!(
                    result.delivered_battery_energy_kwh + 1.0e-7 >= previous_delivered,
                    "delivered energy decreased at {budget_kw} kW under {}",
                    policy.as_str()
                );
                assert!(
                    result.completed_by_deadline_count >= previous_completed,
                    "deadline completions decreased at {budget_kw} kW under {}",
                    policy.as_str()
                );
                previous_delivered = result.delivered_battery_energy_kwh;
                previous_completed = result.completed_by_deadline_count;
            }
        }
    }

    #[test]
    fn policy_comparison_preserves_declared_jobs_and_energy_demand() {
        let template = template(100.0, 0.9, 12.0, constant_curve(60.0));
        let templates = templates(template);
        let case = scenario(
            vec![
                job("a", 0.0, 90.0, 0.2, 0.7),
                job("b", 5.0, 45.0, 0.3, 0.6),
                job("c", 20.0, 100.0, 0.1, 0.5),
            ],
            2,
            16.0,
            vec![],
        );
        let fcfs = simulate_scenario(&templates, &case, Policy::FirstComeFirstServed).unwrap();
        let edf = simulate_scenario(&templates, &case, Policy::EarliestDepartureFirst).unwrap();
        assert_eq!(fcfs.job_count, case.jobs.len());
        assert_eq!(edf.job_count, case.jobs.len());
        assert!(
            (fcfs.requested_battery_energy_kwh - edf.requested_battery_energy_kwh).abs() < EPSILON
        );
        assert_eq!(
            fcfs.jobs
                .iter()
                .map(|j| j.job_id.as_str())
                .collect::<Vec<_>>(),
            edf.jobs
                .iter()
                .map(|j| j.job_id.as_str())
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn capacity_frontier_finds_minimum_sampled_feasible_grid_connection() {
        let templates = templates(template(100.0, 1.0, 100.0, constant_curve(100.0)));
        let case = scenario(vec![job("one", 0.0, 45.0, 0.0, 0.1)], 1, 40.0, vec![]);
        let frontier =
            build_capacity_frontier(&templates, &case, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(frontier.points.len(), 20);
        assert_eq!(frontier.sample_spacing_percent, 5);
        assert_eq!(frontier.minimum_sampled_feasible_site_power_kw, Some(14.0));
        assert!(frontier.points.windows(2).all(|pair| {
            pair[0].installed_site_power_budget_kw < pair[1].installed_site_power_budget_kw
        }));
        assert!(!frontier.points[5].meets_service_target);
        assert!(frontier.points[6].meets_service_target);
        assert_eq!(frontier.service_monotonicity_violations, 0);
        assert_eq!(frontier.feasibility_monotonicity_violations, 0);
    }

    #[test]
    fn capacity_frontier_base_point_matches_reference_run() {
        let templates = templates(template(100.0, 0.9, 25.0, constant_curve(60.0)));
        let case = scenario(vec![job("one", 0.0, 60.0, 0.0, 0.2)], 1, 20.0, vec![]);
        let frontier =
            build_capacity_frontier(&templates, &case, Policy::EarliestDepartureFirst).unwrap();
        let reference =
            simulate_scenario(&templates, &case, Policy::EarliestDepartureFirst).unwrap();
        let base = frontier.points.last().unwrap();
        assert!((base.installed_site_power_budget_kw - case.site_power_budget_kw).abs() < EPSILON);
        assert_eq!(
            base.completed_by_deadline_count,
            reference.completed_by_deadline_count
        );
        assert!(
            (base.delivered_battery_energy_kwh - reference.delivered_battery_energy_kwh).abs()
                < EPSILON
        );
        let reference_meets = reference.completed_by_deadline_count as f64
            / reference.job_count as f64
            >= MIN_DEADLINE_COMPLETION_FRACTION
            && reference.energy_service_fraction >= MIN_ENERGY_SERVICE_FRACTION;
        assert_eq!(base.meets_service_target, reference_meets);
    }
    #[test]
    fn post_charge_port_hold_blocks_queued_job_without_draw_or_extra_energy() {
        let templates = templates(template(100.0, 1.0, 10.0, constant_curve(10.0)));
        let jobs = vec![
            job("a-long-dwell", 0.0, 120.0, 0.0, 0.10),
            job("b-queued", 10.0, 100.0, 0.0, 0.05),
        ];
        let mut release = scenario(jobs.clone(), 1, 10.0, vec![]);
        release.port_release_policy = PortReleasePolicy::ReleaseOnChargeCompletion;
        let release_result =
            simulate_scenario(&templates, &release, Policy::FirstComeFirstServed).unwrap();

        let mut hold = scenario(jobs, 1, 10.0, vec![]);
        hold.port_release_policy = PortReleasePolicy::HoldUntilDeparture;
        let hold_result =
            simulate_scenario(&templates, &hold, Policy::FirstComeFirstServed).unwrap();

        assert_eq!(release_result.completed_by_deadline_count, 2);
        assert!((release_result.site_input_energy_kwh - 15.0).abs() < EPSILON);
        assert_eq!(
            release_result.port_release_policy,
            "release_on_charge_completion"
        );

        assert_eq!(hold_result.completed_by_deadline_count, 1);
        assert_eq!(hold_result.missed_deadline_count, 1);
        assert!((hold_result.site_input_energy_kwh - 10.0).abs() < EPSILON);
        assert!((hold_result.delivered_battery_energy_kwh - 10.0).abs() < EPSILON);
        assert!((hold_result.unserved_battery_energy_kwh - 5.0).abs() < EPSILON);
        assert!(hold_result.energy_balance_residual_kwh.abs() < EPSILON);
        assert!(hold_result.site_power_limit_respected);
        assert!(hold_result.individual_power_limits_respected);
        assert_eq!(hold_result.port_release_policy, "hold_until_departure");
        let queued = hold_result
            .jobs
            .iter()
            .find(|job| job.job_id == "b-queued")
            .unwrap();
        assert_eq!(queued.site_input_energy_kwh, 0.0);
        assert_eq!(queued.delivered_battery_energy_kwh, 0.0);
        assert_eq!(queued.unserved_battery_energy_kwh, 5.0);
    }

    #[test]
    fn held_connector_releases_exactly_at_departure_boundary() {
        let templates = templates(template(100.0, 1.0, 10.0, constant_curve(10.0)));
        let jobs = vec![
            job("a-boundary", 0.0, 60.0, 0.0, 0.10),
            job("b-after-release", 30.0, 90.0, 0.0, 0.05),
        ];
        let mut hold = scenario(jobs, 1, 10.0, vec![]);
        hold.port_release_policy = PortReleasePolicy::HoldUntilDeparture;
        let result = simulate_scenario(&templates, &hold, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.completed_by_deadline_count, 2);
        assert_eq!(result.missed_deadline_count, 0);
        assert_eq!(
            result
                .jobs
                .iter()
                .find(|job| job.job_id == "a-boundary")
                .unwrap()
                .completion_minute,
            Some(60.0)
        );
        assert_eq!(
            result
                .jobs
                .iter()
                .find(|job| job.job_id == "b-after-release")
                .unwrap()
                .first_start_minute,
            Some(60.0)
        );
        assert_eq!(
            result
                .jobs
                .iter()
                .find(|job| job.job_id == "b-after-release")
                .unwrap()
                .completion_minute,
            Some(90.0)
        );
    }

}
