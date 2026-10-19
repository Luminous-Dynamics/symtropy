use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const EPSILON: f64 = 1.0e-9;
const MAX_EVENTS: usize = 100_000;
const CLAIM_CEILING: &str = "Deterministic fleet charging-contingency screen using declared SOC-banded acceptance and piecewise-constant supply/port availability. Inputs are synthetic unless explicitly sourced; this is not an electrochemical, stochastic-reliability, AC load-flow, control-safety, or field-validation model.";

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    claim_ceiling: String,
    vehicle_templates: Vec<VehicleTemplate>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
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
struct CurveBand {
    soc_lower_fraction: f64,
    soc_upper_fraction: f64,
    max_battery_acceptance_kw: f64,
}

#[derive(Debug, Clone, Deserialize)]
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
}

#[derive(Debug, Clone, Deserialize)]
struct Job {
    job_id: String,
    template_id: String,
    arrival_minute: f64,
    departure_minute: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
}

#[derive(Debug, Clone, Deserialize)]
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
    policy: String,
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
struct Report {
    schema_version: String,
    suite_id: String,
    method: &'static str,
    results: Vec<ScenarioResult>,
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
                    eprintln!("simulation failed for {} / {}: {error}", scenario.scenario_id, policy.as_str());
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
    let report = Report {
        schema_version: corpus.schema_version,
        suite_id: corpus.suite_id,
        method: "event-driven, non-voluntary-preemptive priority scheduling with SOC-band events; max-min site-power sharing and declared capacity contingencies",
        results,
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
        validate_template(template).map_err(|error| format!("{}: {error}", template.template_id))?;
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
    if !["synthetic", "measured", "manufacturer", "literature", "estimate", "unknown"]
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
        ("nominal_pack_capacity_kwh", template.nominal_pack_capacity_kwh),
        ("maximum_port_input_power_kw", template.maximum_port_input_power_kw),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name}_must_be_finite_and_positive"));
        }
    }
    for (name, value) in [
        ("state_of_health_fraction", template.state_of_health_fraction),
        ("temperature_capacity_fraction", template.temperature_capacity_fraction),
        ("temperature_acceptance_fraction", template.temperature_acceptance_fraction),
        ("charging_efficiency_fraction", template.charging_efficiency_fraction),
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

fn validate_scenario(
    scenario: &Scenario,
    template_ids: &BTreeSet<String>,
) -> Result<(), String> {
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
            return Err(format!("invalid_arrival_or_departure_for_job_{}", job.job_id));
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
            .then_with(|| a.end_minute.partial_cmp(&b.end_minute).unwrap_or(Ordering::Equal))
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
            return Err(format!("capacity_event_{index}_has_invalid_bounds_or_budget"));
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

        if states.iter().all(|state| state.completion_minute.is_some() || state.expired) {
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
            let template = templates
                .get(&state.job.template_id)
                .ok_or_else(|| format!("unknown_template_id: {}", state.job.template_id))?;
            let battery_acceptance_kw = acceptance_power_at(template, state.current_soc_fraction)
                * template.temperature_acceptance_fraction;
            let acceptance_input_cap = battery_acceptance_kw / template.charging_efficiency_fraction;
            caps.push((
                index,
                template.maximum_port_input_power_kw.min(acceptance_input_cap),
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
                if let Some(boundary) =
                    next_curve_boundary(template, state.current_soc_fraction, state.job.target_soc_fraction)
                {
                    let energy_to_boundary =
                        (boundary - state.current_soc_fraction).max(0.0) * state.effective_capacity_kwh;
                    let minutes_to_boundary = energy_to_boundary / battery_kw * 60.0;
                    if minutes_to_boundary > EPSILON {
                        let event_time = now + minutes_to_boundary;
                        next_time = Some(next_time.map_or(event_time, |value| value.min(event_time)));
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
                state.delivered_battery_energy_kwh =
                    state.delivered_battery_energy_kwh.min(state.requested_battery_energy_kwh);
                state.current_soc_fraction = state.job.target_soc_fraction;
                state.completion_minute = Some(now);
            }
        }
        active.retain(|index| states[*index].completion_minute.is_none() && !states[*index].expired);
    }

    if states.iter().any(|state| state.completion_minute.is_none() && !state.expired) {
        return Err(format!("event_limit_exceeded_{MAX_EVENTS}"));
    }

    let mut jobs = Vec::with_capacity(states.len());
    for state in &states {
        let template = templates
            .get(&state.job.template_id)
            .ok_or_else(|| format!("unknown_template_id: {}", state.job.template_id))?;
        let unserved = (state.requested_battery_energy_kwh - state.delivered_battery_energy_kwh)
            .max(0.0);
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
    let requested_battery_energy_kwh: f64 =
        jobs.iter().map(|job| job.requested_battery_energy_kwh).sum();
    let delivered_battery_energy_kwh: f64 =
        jobs.iter().map(|job| job.delivered_battery_energy_kwh).sum();
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
        policy: policy.as_str().to_string(),
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
        .find(|event| {
            event.start_minute <= minute + EPSILON
                && minute < event.end_minute - EPSILON
        })
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
            soc >= band.soc_lower_fraction - EPSILON
                && soc < band.soc_upper_fraction - EPSILON
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

    fn template(capacity_kwh: f64, efficiency: f64, port_kw: f64, curve: Vec<CurveBand>) -> VehicleTemplate {
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

    fn scenario(jobs: Vec<Job>, port_count: usize, site_kw: f64, events: Vec<CapacityEvent>) -> Scenario {
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
        }
    }

    fn templates(template: VehicleTemplate) -> BTreeMap<String, VehicleTemplate> {
        [(template.template_id.clone(), template)].into_iter().collect()
    }

    #[test]
    fn constant_power_oracle_has_expected_energy_time_and_loss() {
        let template = template(10.0, 0.9, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 0.0, 90.0, 0.0, 0.9)], 1, 10.0, vec![]);
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
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
                CurveBand { soc_lower_fraction: 0.0, soc_upper_fraction: 0.5, max_battery_acceptance_kw: 10.0 },
                CurveBand { soc_lower_fraction: 0.5, soc_upper_fraction: 1.0, max_battery_acceptance_kw: 5.0 },
            ],
        );
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 0.0, 120.0, 0.4, 0.8)], 1, 100.0, vec![]);
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert!((result.jobs[0].completion_minute.unwrap() - 84.0).abs() < 1.0e-7);
        assert!((result.jobs[0].site_input_energy_kwh - 8.0).abs() < 1.0e-8);
        assert!((result.maximum_site_input_power_kw - 10.0).abs() < 1.0e-8);
    }

    #[test]
    fn max_min_sharing_never_exceeds_site_budget() {
        let template = template(100.0, 1.0, 10.0, constant_curve(10.0));
        let templates = templates(template);
        let scenario = scenario(
            vec![job("a", 0.0, 130.0, 0.0, 0.1), job("b", 0.0, 130.0, 0.0, 0.1)],
            2,
            10.0,
            vec![],
        );
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.completed_by_deadline_count, 2);
        assert!(result.site_power_limit_respected);
        assert!(result.maximum_site_input_power_kw <= 10.0 + 1.0e-8);
        assert!(result.jobs.iter().all(|job| job.maximum_observed_input_power_kw <= 10.0 + 1.0e-8));
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
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
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
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.completed_by_deadline_count, 1);
        assert_eq!(result.missed_deadline_count, 0);
        assert!((result.jobs[0].completion_minute.unwrap() - 60.0).abs() < 1.0e-7);
    }

    #[test]
    fn urgent_job_deadline_oracle_distinguishes_policies() {
        let template = template(100.0, 1.0, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(
            vec![job("a-long", 0.0, 120.0, 0.0, 0.1), job("b-short", 0.0, 20.0, 0.0, 0.03)],
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
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
        assert_eq!(result.job_count, 1);
        assert_eq!(result.missed_deadline_count, 1);
        assert!((result.unserved_battery_energy_kwh - 8.25).abs() < 1.0e-7);
    }

    #[test]
    fn zero_energy_job_is_completed_without_consuming_a_port() {
        let template = template(10.0, 0.9, 10.0, constant_curve(100.0));
        let templates = templates(template);
        let scenario = scenario(vec![job("a", 10.0, 20.0, 0.5, 0.5)], 0, 0.0, vec![]);
        let result = simulate_scenario(&templates, &scenario, Policy::FirstComeFirstServed).unwrap();
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
                CurveBand { soc_lower_fraction: 0.0, soc_upper_fraction: 0.4, max_battery_acceptance_kw: 10.0 },
                CurveBand { soc_lower_fraction: 0.5, soc_upper_fraction: 1.0, max_battery_acceptance_kw: 5.0 },
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
            CapacityEvent { start_minute: 10.0, end_minute: 30.0, site_power_budget_kw: 0.0, available_port_count: 0 },
            CapacityEvent { start_minute: 20.0, end_minute: 40.0, site_power_budget_kw: 5.0, available_port_count: 1 },
        ];
        assert!(validate_scenario(&scenario, &templates.keys().cloned().collect()).is_err());
    }

    #[test]
    fn canonical_job_order_does_not_depend_on_input_order() {
        let template = template(100.0, 1.0, 10.0, constant_curve(10.0));
        let templates = templates(template);
        let a = job("z-last", 0.0, 120.0, 0.0, 0.1);
        let b = job("a-first", 0.0, 120.0, 0.0, 0.03);
        let first = scenario(vec![a.clone(), b.clone()], 1, 10.0, vec![]);
        let second = scenario(vec![b, a], 1, 10.0, vec![]);
        assert_eq!(
            simulate_scenario(&templates, &first, Policy::FirstComeFirstServed).unwrap(),
            simulate_scenario(&templates, &second, Policy::FirstComeFirstServed).unwrap()
        );
        assert_eq!(
            simulate_scenario(&templates, &first, Policy::EarliestDepartureFirst).unwrap(),
            simulate_scenario(&templates, &second, Policy::EarliestDepartureFirst).unwrap()
        );
    }
}
