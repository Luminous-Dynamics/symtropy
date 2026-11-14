use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const EPSILON: f64 = 1.0e-9;
const CLAIM_CEILING: &str = "Deterministic discrete-event queue screen with constant charging efficiency and power limits on synthetic inputs; not a battery charge-curve model, grid study, charger availability model, or measured fleet result.";

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    scenario_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    charger_port_count: usize,
    port_power_kw: f64,
    site_power_budget_kw: f64,
    jobs: Vec<Job>,
}

#[derive(Debug, Clone, Deserialize)]
struct Job {
    job_id: String,
    arrival_minute: f64,
    departure_deadline_minute: f64,
    battery_energy_required_kwh: f64,
    charging_efficiency_fraction: f64,
    vehicle_max_acceptance_kw: f64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Policy {
    FirstComeFirstServed,
    EarliestDepartureDeadlineFirst,
}

impl Policy {
    fn label(self) -> &'static str {
        match self {
            Self::FirstComeFirstServed => "fcfs",
            Self::EarliestDepartureDeadlineFirst => "earliest_deadline_first",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct JobResult {
    job_id: String,
    arrival_minute: f64,
    departure_deadline_minute: f64,
    requested_battery_energy_kwh: f64,
    delivered_battery_energy_kwh: f64,
    site_input_energy_kwh: f64,
    conversion_loss_kwh: f64,
    charging_efficiency_fraction: f64,
    vehicle_max_acceptance_kw: f64,
    port_power_kw: f64,
    maximum_allowed_site_input_power_kw: f64,
    maximum_site_input_power_kw: f64,
    queue_wait_minutes: f64,
    started: bool,
    start_minute: Option<f64>,
    completion_minute: Option<f64>,
    deadline_slack_minutes: Option<f64>,
    completed_by_deadline: bool,
    missed_deadline: bool,
    unserved_battery_energy_kwh_at_deadline: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    scenario_id: String,
    label: String,
    policy: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    charger_port_count: usize,
    port_power_kw: f64,
    site_power_budget_kw: f64,
    total_jobs: usize,
    completed_by_deadline: usize,
    missed_deadlines: usize,
    total_requested_battery_energy_kwh: f64,
    total_delivered_battery_energy_kwh: f64,
    total_site_input_energy_kwh: f64,
    total_conversion_loss_kwh: f64,
    total_unserved_battery_energy_kwh: f64,
    energy_balance_residual_kwh: f64,
    mean_queue_wait_minutes: f64,
    median_queue_wait_minutes: f64,
    p95_queue_wait_minutes: f64,
    maximum_queue_wait_minutes: f64,
    maximum_site_input_power_kw: f64,
    site_power_limit_respected: bool,
    total_elapsed_minutes: f64,
    active_port_minutes: f64,
    port_utilization_fraction: f64,
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

#[derive(Debug, Clone)]
struct JobState {
    job: Job,
    remaining_site_energy_kwh: f64,
    site_energy_delivered_kwh: f64,
    maximum_site_input_power_kw: f64,
    start_minute: Option<f64>,
    completion_minute: Option<f64>,
    deadline_recorded: bool,
    missed_deadline: bool,
    expired: bool,
    unserved_battery_energy_kwh_at_deadline: f64,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/charging_queue_v0_1.json"
            .to_string()
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
            eprintln!("invalid charging-queue JSON: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_corpus(&corpus) {
        eprintln!("invalid charging-queue corpus: {error}");
        process::exit(2);
    }

    let mut results = Vec::new();
    for scenario in &corpus.scenarios {
        results.push(simulate(scenario, Policy::FirstComeFirstServed));
        results.push(simulate(scenario, Policy::EarliestDepartureDeadlineFirst));
    }
    results.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then_with(|| a.policy.cmp(&b.policy))
    });

    let report = Report {
        schema_version: corpus.schema_version,
        suite_id: corpus.suite_id,
        method: "deterministic non-preemptive discrete-event queue; max-min site-input power sharing; constant conversion efficiency; jobs depart at declared deadlines",
        results,
        claim_ceiling: CLAIM_CEILING,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize charging queue report: {error}");
            process::exit(2);
        }
    }
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_version != "0.1.0" || corpus.suite_id != "EV-CHARGE-QUEUE-001" {
        return Err("unsupported_schema_or_suite_id".to_string());
    }
    if corpus.claim_ceiling.trim().is_empty() || corpus.scenarios.is_empty() {
        return Err("claim_ceiling_and_nonempty_scenarios_are_required".to_string());
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &corpus.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("duplicate_scenario_id: {}", scenario.scenario_id));
        }
        validate_scenario(scenario)
            .map_err(|error| format!("{}: {error}", scenario.scenario_id))?;
    }
    Ok(())
}

fn validate_scenario(scenario: &Scenario) -> Result<(), String> {
    for (name, value) in [
        ("scenario_id", scenario.scenario_id.as_str()),
        ("label", scenario.label.as_str()),
        ("evidence_note", scenario.evidence_note.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name}_must_be_nonempty"));
        }
    }
    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
        "unknown",
    ]
    .contains(&scenario.evidence_class.as_str())
    {
        return Err("unsupported_evidence_class".to_string());
    }
    if scenario.evidence_class != "synthetic"
        && scenario.evidence_class != "unknown"
        && scenario.evidence_reference.trim().is_empty()
    {
        return Err("non_synthetic_evidence_requires_reference".to_string());
    }
    if scenario.charger_port_count == 0 {
        return Err("charger_port_count_must_be_positive".to_string());
    }
    for (name, value) in [
        ("port_power_kw", scenario.port_power_kw),
        ("site_power_budget_kw", scenario.site_power_budget_kw),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name}_must_be_finite_and_positive"));
        }
    }
    if scenario.jobs.is_empty() {
        return Err("scenario_must_have_jobs".to_string());
    }

    let mut job_ids = BTreeSet::new();
    for job in &scenario.jobs {
        if job.job_id.trim().is_empty() || !job_ids.insert(job.job_id.clone()) {
            return Err("job_ids_must_be_nonempty_and_unique".to_string());
        }
        for (name, value) in [
            ("arrival_minute", job.arrival_minute),
            ("departure_deadline_minute", job.departure_deadline_minute),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "job_{}_{}_must_be_finite_and_nonnegative",
                    job.job_id, name
                ));
            }
        }
        if job.departure_deadline_minute <= job.arrival_minute {
            return Err(format!("job_{}_deadline_must_follow_arrival", job.job_id));
        }
        for (name, value) in [
            (
                "battery_energy_required_kwh",
                job.battery_energy_required_kwh,
            ),
            ("vehicle_max_acceptance_kw", job.vehicle_max_acceptance_kw),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!(
                    "job_{}_{}_must_be_finite_and_positive",
                    job.job_id, name
                ));
            }
        }
        if !job.charging_efficiency_fraction.is_finite()
            || job.charging_efficiency_fraction <= 0.0
            || job.charging_efficiency_fraction > 1.0
        {
            return Err(format!(
                "job_{}_charging_efficiency_must_be_in_(0,1]",
                job.job_id
            ));
        }
    }
    Ok(())
}

fn simulate(scenario: &Scenario, policy: Policy) -> ScenarioResult {
    let mut jobs = scenario.jobs.clone();
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    let mut states: Vec<JobState> = jobs
        .into_iter()
        .map(|job| JobState {
            remaining_site_energy_kwh: job.battery_energy_required_kwh
                / job.charging_efficiency_fraction,
            job,
            site_energy_delivered_kwh: 0.0,
            maximum_site_input_power_kw: 0.0,
            start_minute: None,
            completion_minute: None,
            deadline_recorded: false,
            missed_deadline: false,
            expired: false,
            unserved_battery_energy_kwh_at_deadline: 0.0,
        })
        .collect();

    let first_arrival = states
        .iter()
        .map(|state| state.job.arrival_minute)
        .fold(f64::INFINITY, f64::min);
    let mut now = first_arrival;
    let mut active: Vec<usize> = Vec::new();
    let mut maximum_site_input_power_kw: f64 = 0.0;
    let mut active_port_minutes = 0.0;
    let max_events = states.len().saturating_mul(4).saturating_add(16);

    for _ in 0..max_events {
        // A completion at exactly the departure deadline is a success.
        for state in &mut states {
            if state.completion_minute.is_none()
                && !state.expired
                && state.remaining_site_energy_kwh <= EPSILON
            {
                state.remaining_site_energy_kwh = 0.0;
                state.completion_minute = Some(now);
            }
        }
        active
            .retain(|index| states[*index].completion_minute.is_none() && !states[*index].expired);

        for state in &mut states {
            if !state.deadline_recorded
                && state.completion_minute.is_none()
                && now + EPSILON >= state.job.departure_deadline_minute
            {
                state.deadline_recorded = true;
                state.missed_deadline = state.remaining_site_energy_kwh > EPSILON;
                state.unserved_battery_energy_kwh_at_deadline =
                    state.remaining_site_energy_kwh * state.job.charging_efficiency_fraction;
                state.expired = true;
            } else if !state.deadline_recorded
                && state.completion_minute.is_some()
                && now + EPSILON >= state.job.departure_deadline_minute
            {
                state.deadline_recorded = true;
            }
        }
        active
            .retain(|index| !states[*index].expired && states[*index].completion_minute.is_none());

        if states
            .iter()
            .all(|state| state.completion_minute.is_some() || state.expired)
        {
            break;
        }

        while active.len() < scenario.charger_port_count {
            let mut waiting: Vec<usize> = states
                .iter()
                .enumerate()
                .filter(|(_, state)| {
                    state.start_minute.is_none()
                        && state.completion_minute.is_none()
                        && !state.expired
                        && state.job.arrival_minute <= now + EPSILON
                })
                .map(|(index, _)| index)
                .collect();
            if waiting.is_empty() {
                break;
            }
            waiting.sort_by(|left, right| {
                let a = &states[*left].job;
                let b = &states[*right].job;
                let primary = match policy {
                    Policy::FirstComeFirstServed => a
                        .arrival_minute
                        .total_cmp(&b.arrival_minute)
                        .then_with(|| a.job_id.cmp(&b.job_id)),
                    Policy::EarliestDepartureDeadlineFirst => a
                        .departure_deadline_minute
                        .total_cmp(&b.departure_deadline_minute)
                        .then_with(|| a.arrival_minute.total_cmp(&b.arrival_minute))
                        .then_with(|| a.job_id.cmp(&b.job_id)),
                };
                primary
            });
            let selected = waiting[0];
            states[selected].start_minute = Some(now);
            active.push(selected);
        }

        let limits: Vec<f64> = active
            .iter()
            .map(|index| {
                let state = &states[*index];
                scenario.port_power_kw.min(
                    state.job.vehicle_max_acceptance_kw / state.job.charging_efficiency_fraction,
                )
            })
            .collect();
        let allocations = max_min_water_fill(&limits, scenario.site_power_budget_kw);
        let site_power: f64 = allocations.iter().sum();
        maximum_site_input_power_kw = maximum_site_input_power_kw.max(site_power);
        if site_power > scenario.site_power_budget_kw + 1e-8 {
            break;
        }

        let mut next_event = f64::INFINITY;
        for state in &states {
            if state.start_minute.is_none()
                && state.completion_minute.is_none()
                && !state.expired
                && state.job.arrival_minute > now + EPSILON
            {
                next_event = next_event.min(state.job.arrival_minute);
            }
            if state.completion_minute.is_none()
                && !state.expired
                && state.job.departure_deadline_minute > now + EPSILON
            {
                next_event = next_event.min(state.job.departure_deadline_minute);
            }
        }
        for (slot, index) in active.iter().enumerate() {
            let power = allocations[slot];
            if power > EPSILON {
                next_event =
                    next_event.min(now + states[*index].remaining_site_energy_kwh / power * 60.0);
            }
        }
        if !next_event.is_finite() || next_event <= now + EPSILON {
            break;
        }

        let elapsed_minutes = next_event - now;
        let elapsed_hours = elapsed_minutes / 60.0;
        active_port_minutes += active.len() as f64 * elapsed_minutes;
        for (slot, index) in active.iter().enumerate() {
            states[*index].maximum_site_input_power_kw = states[*index]
                .maximum_site_input_power_kw
                .max(allocations[slot]);
            let delivered_site_kwh = allocations[slot] * elapsed_hours;
            states[*index].site_energy_delivered_kwh += delivered_site_kwh;
            states[*index].remaining_site_energy_kwh =
                (states[*index].remaining_site_energy_kwh - delivered_site_kwh).max(0.0);
        }
        now = next_event;
    }

    let mut job_results: Vec<JobResult> = states
        .iter()
        .map(|state| {
            let delivered_battery =
                state.site_energy_delivered_kwh * state.job.charging_efficiency_fraction;
            let completed_by_deadline = state.completion_minute.is_some_and(|completion| {
                completion <= state.job.departure_deadline_minute + EPSILON
            });
            let missed_deadline = state.missed_deadline
                || (!completed_by_deadline && state.completion_minute.is_none());
            let deadline_slack = state
                .completion_minute
                .map(|completion| state.job.departure_deadline_minute - completion);
            let queue_wait = match state.start_minute {
                Some(start) => start - state.job.arrival_minute,
                None => (state.job.departure_deadline_minute - state.job.arrival_minute).max(0.0),
            };
            let unserved = if missed_deadline {
                (state.job.battery_energy_required_kwh - delivered_battery).max(0.0)
            } else {
                0.0
            };
            JobResult {
                job_id: state.job.job_id.clone(),
                arrival_minute: state.job.arrival_minute,
                departure_deadline_minute: state.job.departure_deadline_minute,
                requested_battery_energy_kwh: state.job.battery_energy_required_kwh,
                delivered_battery_energy_kwh: delivered_battery,
                site_input_energy_kwh: state.site_energy_delivered_kwh,
                conversion_loss_kwh: state.site_energy_delivered_kwh - delivered_battery,
                charging_efficiency_fraction: state.job.charging_efficiency_fraction,
                vehicle_max_acceptance_kw: state.job.vehicle_max_acceptance_kw,
                port_power_kw: scenario.port_power_kw,
                maximum_allowed_site_input_power_kw: scenario.port_power_kw.min(
                    state.job.vehicle_max_acceptance_kw / state.job.charging_efficiency_fraction,
                ),
                maximum_site_input_power_kw: state.maximum_site_input_power_kw,
                queue_wait_minutes: queue_wait,
                started: state.start_minute.is_some(),
                start_minute: state.start_minute,
                completion_minute: state.completion_minute,
                deadline_slack_minutes: deadline_slack,
                completed_by_deadline,
                missed_deadline,
                unserved_battery_energy_kwh_at_deadline: unserved
                    .max(state.unserved_battery_energy_kwh_at_deadline),
            }
        })
        .collect();
    job_results.sort_by(|a, b| a.job_id.cmp(&b.job_id));

    let total_requested: f64 = job_results
        .iter()
        .map(|job| job.requested_battery_energy_kwh)
        .sum();
    let total_battery_delivered: f64 = job_results
        .iter()
        .map(|job| job.delivered_battery_energy_kwh)
        .sum();
    let total_site_input: f64 = job_results
        .iter()
        .map(|job| job.site_input_energy_kwh)
        .sum();
    let total_loss: f64 = job_results.iter().map(|job| job.conversion_loss_kwh).sum();
    let total_unserved: f64 = job_results
        .iter()
        .map(|job| job.unserved_battery_energy_kwh_at_deadline)
        .sum();
    let completed_by_deadline = job_results
        .iter()
        .filter(|job| job.completed_by_deadline)
        .count();
    let missed_deadlines = job_results.iter().filter(|job| job.missed_deadline).count();
    let mut waits: Vec<f64> = job_results
        .iter()
        .map(|job| job.queue_wait_minutes)
        .collect();
    waits.sort_by(f64::total_cmp);
    let mean_wait = if waits.is_empty() {
        0.0
    } else {
        waits.iter().sum::<f64>() / waits.len() as f64
    };
    let median_wait = quantile(&waits, 0.5);
    let p95_wait = quantile(&waits, 0.95);
    let maximum_wait = waits.last().copied().unwrap_or(0.0);
    let elapsed = (now - first_arrival).max(0.0);
    let available_port_minutes = scenario.charger_port_count as f64 * elapsed;
    let utilization = if available_port_minutes > 0.0 {
        (active_port_minutes / available_port_minutes).clamp(0.0, 1.0)
    } else {
        0.0
    };

    ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        policy: policy.label().to_string(),
        evidence_class: scenario.evidence_class.clone(),
        evidence_reference: scenario.evidence_reference.clone(),
        evidence_note: scenario.evidence_note.clone(),
        charger_port_count: scenario.charger_port_count,
        port_power_kw: scenario.port_power_kw,
        site_power_budget_kw: scenario.site_power_budget_kw,
        total_jobs: job_results.len(),
        completed_by_deadline,
        missed_deadlines,
        total_requested_battery_energy_kwh: total_requested,
        total_delivered_battery_energy_kwh: total_battery_delivered,
        total_site_input_energy_kwh: total_site_input,
        total_conversion_loss_kwh: total_loss,
        total_unserved_battery_energy_kwh: total_unserved,
        energy_balance_residual_kwh: total_site_input - total_battery_delivered - total_loss,
        mean_queue_wait_minutes: mean_wait,
        median_queue_wait_minutes: median_wait,
        p95_queue_wait_minutes: p95_wait,
        maximum_queue_wait_minutes: maximum_wait,
        maximum_site_input_power_kw,
        site_power_limit_respected: maximum_site_input_power_kw
            <= scenario.site_power_budget_kw + 1e-8,
        total_elapsed_minutes: elapsed,
        active_port_minutes,
        port_utilization_fraction: utilization,
        jobs: job_results,
        claim_ceiling: CLAIM_CEILING,
    }
}

fn max_min_water_fill(limits_kw: &[f64], site_budget_kw: f64) -> Vec<f64> {
    let mut allocation = vec![0.0; limits_kw.len()];
    if limits_kw.is_empty() || !site_budget_kw.is_finite() || site_budget_kw <= 0.0 {
        return allocation;
    }
    let mut order: Vec<usize> = (0..limits_kw.len()).collect();
    order.sort_by(|a, b| {
        limits_kw[*a]
            .total_cmp(&limits_kw[*b])
            .then_with(|| a.cmp(b))
    });
    let mut remaining_budget = site_budget_kw;
    let mut cursor = 0usize;
    while cursor < order.len() {
        let active_count = order.len() - cursor;
        let fair_share = remaining_budget / active_count as f64;
        let idx = order[cursor];
        let cap = limits_kw[idx];
        if cap <= fair_share + EPSILON {
            allocation[idx] = cap;
            remaining_budget = (remaining_budget - cap).max(0.0);
            cursor += 1;
        } else {
            for index in &order[cursor..] {
                allocation[*index] = fair_share;
            }
            remaining_budget = 0.0;
            break;
        }
    }
    let _unused_budget = remaining_budget;
    allocation
}

fn quantile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    if sorted_values.len() == 1 {
        return sorted_values[0];
    }
    let position = p.clamp(0.0, 1.0) * (sorted_values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted_values[lower] * (1.0 - fraction) + sorted_values[upper] * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(
        id: &str,
        arrival: f64,
        deadline: f64,
        energy: f64,
        acceptance: f64,
        efficiency: f64,
    ) -> Job {
        Job {
            job_id: id.to_string(),
            arrival_minute: arrival,
            departure_deadline_minute: deadline,
            battery_energy_required_kwh: energy,
            charging_efficiency_fraction: efficiency,
            vehicle_max_acceptance_kw: acceptance,
        }
    }

    fn scenario(jobs: Vec<Job>) -> Scenario {
        Scenario {
            scenario_id: "test-queue".to_string(),
            label: "synthetic queue test".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "test only".to_string(),
            charger_port_count: 1,
            port_power_kw: 10.0,
            site_power_budget_kw: 10.0,
            jobs,
        }
    }

    #[test]
    fn max_min_sharing_respects_small_vehicle_cap() {
        let allocation = max_min_water_fill(&[20.0, 5.0, 100.0], 40.0);
        assert!((allocation[0] - 17.5).abs() < 1e-9);
        assert!((allocation[1] - 5.0).abs() < 1e-9);
        assert!((allocation[2] - 17.5).abs() < 1e-9);
        assert!(allocation.iter().sum::<f64>() <= 40.0 + 1e-9);
    }

    #[test]
    fn site_power_never_exceeds_shared_limit() {
        let mut s = scenario(vec![
            job("a", 0.0, 120.0, 10.0, 20.0, 1.0),
            job("b", 0.0, 120.0, 10.0, 20.0, 1.0),
        ]);
        s.charger_port_count = 2;
        s.port_power_kw = 20.0;
        s.site_power_budget_kw = 12.0;
        let result = simulate(&s, Policy::FirstComeFirstServed);
        assert!(result.site_power_limit_respected);
        assert!(result.maximum_site_input_power_kw <= 12.0 + 1e-9);
        assert!(
            (result.total_site_input_energy_kwh
                - result.total_delivered_battery_energy_kwh
                - result.total_conversion_loss_kwh)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn edf_can_avoid_a_deadline_miss_when_fcfs_cannot() {
        let s = scenario(vec![
            job("a-long", 0.0, 120.0, 10.0, 10.0, 1.0),
            job("b-short", 0.0, 20.0, 3.0, 10.0, 1.0),
        ]);
        let fcfs = simulate(&s, Policy::FirstComeFirstServed);
        let edf = simulate(&s, Policy::EarliestDepartureDeadlineFirst);
        assert_eq!(fcfs.missed_deadlines, 1);
        assert_eq!(edf.missed_deadlines, 0);
        assert_eq!(edf.completed_by_deadline, 2);
        assert_eq!(
            edf.jobs
                .iter()
                .find(|x| x.job_id == "b-short")
                .unwrap()
                .completion_minute,
            Some(18.0)
        );
    }

    #[test]
    fn queued_job_is_removed_when_deadline_expires() {
        let s = scenario(vec![
            job("a-long", 0.0, 120.0, 10.0, 10.0, 1.0),
            job("b-short", 0.0, 20.0, 3.0, 10.0, 1.0),
        ]);
        let result = simulate(&s, Policy::FirstComeFirstServed);
        let short = result
            .jobs
            .iter()
            .find(|job| job.job_id == "b-short")
            .unwrap();
        assert!(!short.completed_by_deadline);
        assert!(short.missed_deadline);
        assert!(!short.started);
        assert!((short.unserved_battery_energy_kwh_at_deadline - 3.0).abs() < 1e-9);
    }

    #[test]
    fn input_order_does_not_change_fcfs_or_edf_reports() {
        let a = job("z-last", 0.0, 120.0, 10.0, 10.0, 1.0);
        let b = job("a-first", 0.0, 120.0, 3.0, 10.0, 1.0);
        let first = scenario(vec![a.clone(), b.clone()]);
        let second = scenario(vec![b, a]);
        assert_eq!(
            simulate(&first, Policy::FirstComeFirstServed),
            simulate(&second, Policy::FirstComeFirstServed)
        );
        assert_eq!(
            simulate(&first, Policy::EarliestDepartureDeadlineFirst),
            simulate(&second, Policy::EarliestDepartureDeadlineFirst)
        );
    }

    #[test]
    fn invalid_efficiency_is_rejected() {
        let mut s = scenario(vec![job("a", 0.0, 120.0, 10.0, 10.0, 0.0)]);
        assert!(validate_scenario(&s).is_err());
        s.jobs[0].charging_efficiency_fraction = 1.2;
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn a_completion_at_the_deadline_is_not_a_miss() {
        let s = scenario(vec![job("a", 0.0, 60.0, 10.0, 10.0, 1.0)]);
        let result = simulate(&s, Policy::FirstComeFirstServed);
        assert_eq!(result.completed_by_deadline, 1);
        assert_eq!(result.missed_deadlines, 0);
        assert_eq!(result.jobs[0].completion_minute, Some(60.0));
    }

    #[test]
    fn zero_port_or_site_capacity_is_rejected() {
        let mut s = scenario(vec![job("a", 0.0, 60.0, 2.0, 10.0, 1.0)]);
        s.charger_port_count = 0;
        assert_eq!(
            validate_scenario(&s).unwrap_err(),
            "charger_port_count_must_be_positive"
        );
        s.charger_port_count = 1;
        s.site_power_budget_kw = 0.0;
        assert_eq!(
            validate_scenario(&s).unwrap_err(),
            "site_power_budget_kw_must_be_finite_and_positive"
        );
    }
}
