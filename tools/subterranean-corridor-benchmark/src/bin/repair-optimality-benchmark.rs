//! Bounded exact reference oracle for the deterministic repair scheduler.
//!
//! This binary enumerates every precedence-feasible task order up to
//! MAX_EXACT_TASKS, applies the serial schedule-generation scheme to each order,
//! and reports the minimum makespan. It is a small-instance oracle, not a
//! production-scale optimizer or a physical recovery model.

use std::collections::{BTreeMap, BTreeSet};

use subterranean_corridor_benchmark::{
    EvidenceClass, GateState,
    repair::{
        CrewPool, CrewRequirement, RepairScheduleScenario, RepairTask, RepairTaskKind,
        schedule_repairs,
    },
};

const EPS: f64 = 1.0e-9;
const MAX_EXACT_TASKS: usize = 9;
const GENERATED_FIXTURES: u64 = 50;

#[derive(Debug, Clone)]
struct PlacedTask {
    task_id: String,
    start_hour: f64,
    finish_hour: f64,
}

#[derive(Debug, Clone)]
struct ExactResult {
    makespan_hours: f64,
    best_order: Vec<String>,
    orders_evaluated: usize,
}

#[derive(Debug)]
struct SearchState {
    makespan_hours: f64,
    best_order: Vec<String>,
    orders_evaluated: usize,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            makespan_hours: f64::INFINITY,
            best_order: Vec::new(),
            orders_evaluated: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Comparison {
    greedy_makespan_hours: f64,
    greedy_lower_bound_hours: f64,
    exact: ExactResult,
}

fn exact_minimum_makespan(scenario: &RepairScheduleScenario) -> Result<ExactResult, String> {
    if scenario.tasks.is_empty() {
        return Err("EXACT_ORACLE_REQUIRES_TASKS".to_owned());
    }
    if scenario.tasks.len() > MAX_EXACT_TASKS {
        return Err(format!(
            "EXACT_ORACLE_TASK_LIMIT_EXCEEDED:{}>{MAX_EXACT_TASKS}",
            scenario.tasks.len()
        ));
    }

    let by_id: BTreeMap<String, &RepairTask> = scenario
        .tasks
        .iter()
        .map(|task| (task.task_id.clone(), task))
        .collect();
    if by_id.len() != scenario.tasks.len() || by_id.keys().any(|task_id| task_id.trim().is_empty()) {
        return Err("EXACT_ORACLE_EMPTY_OR_DUPLICATE_TASK_ID".to_owned());
    }

    let capacities: BTreeMap<String, u32> = scenario
        .crews
        .iter()
        .map(|crew| (crew.skill_id.clone(), crew.available_crews))
        .collect();
    if capacities.len() != scenario.crews.len()
        || capacities
            .iter()
            .any(|(skill_id, count)| skill_id.trim().is_empty() || *count == 0)
    {
        return Err("EXACT_ORACLE_INVALID_CREW_POOLS".to_owned());
    }

    for task in &scenario.tasks {
        if !task.duration_hours.is_finite() || task.duration_hours <= 0.0 {
            return Err(format!("EXACT_ORACLE_INVALID_DURATION:{}", task.task_id));
        }
        if task.crew_requirements.is_empty() {
            return Err(format!(
                "EXACT_ORACLE_MISSING_CREW_REQUIREMENTS:{}",
                task.task_id
            ));
        }
        for predecessor in &task.predecessors {
            if !by_id.contains_key(predecessor) {
                return Err(format!(
                    "EXACT_ORACLE_UNKNOWN_PREDECESSOR:{}:{predecessor}",
                    task.task_id
                ));
            }
        }
        for requirement in &task.crew_requirements {
            let Some(available) = capacities.get(&requirement.skill_id) else {
                return Err(format!(
                    "EXACT_ORACLE_UNKNOWN_SKILL:{}:{}",
                    task.task_id, requirement.skill_id
                ));
            };
            if requirement.crews_required == 0 || requirement.crews_required > *available {
                return Err(format!(
                    "EXACT_ORACLE_INVALID_CREW_DEMAND:{}:{}",
                    task.task_id, requirement.skill_id
                ));
            }
        }
    }

    let mut remaining: BTreeSet<String> = by_id.keys().cloned().collect();
    let mut order = Vec::with_capacity(scenario.tasks.len());
    let mut state = SearchState::default();
    enumerate_orders(&by_id, &capacities, &mut order, &mut remaining, &mut state)?;

    if state.orders_evaluated == 0 || !state.makespan_hours.is_finite() {
        return Err("EXACT_ORACLE_NO_PRECEDENCE_FEASIBLE_SCHEDULE".to_owned());
    }

    Ok(ExactResult {
        makespan_hours: state.makespan_hours,
        best_order: state.best_order,
        orders_evaluated: state.orders_evaluated,
    })
}

fn enumerate_orders(
    by_id: &BTreeMap<String, &RepairTask>,
    capacities: &BTreeMap<String, u32>,
    order: &mut Vec<String>,
    remaining: &mut BTreeSet<String>,
    state: &mut SearchState,
) -> Result<(), String> {
    if remaining.is_empty() {
        let schedule = serial_schedule_for_order(order, by_id, capacities)?;
        let makespan = schedule
            .iter()
            .map(|task| task.finish_hour)
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        if !makespan.is_finite() {
            return Err("EXACT_ORACLE_NONFINITE_MAKESPAN".to_owned());
        }
        state.orders_evaluated = state.orders_evaluated.saturating_add(1);
        if makespan < state.makespan_hours - EPS {
            state.makespan_hours = makespan;
            state.best_order.clone_from(order);
        }
        return Ok(());
    }

    let already_placed: BTreeSet<&str> = order.iter().map(String::as_str).collect();
    let ready: Vec<String> = remaining
        .iter()
        .filter(|task_id| {
            by_id.get(*task_id).is_some_and(|task| {
                task.predecessors
                    .iter()
                    .all(|predecessor| already_placed.contains(predecessor.as_str()))
            })
        })
        .cloned()
        .collect();

    // An empty ready set before completion means this branch is cyclic.
    for task_id in ready {
        remaining.remove(&task_id);
        order.push(task_id.clone());
        enumerate_orders(by_id, capacities, order, remaining, state)?;
        order.pop();
        remaining.insert(task_id);
    }
    Ok(())
}

/// Generate the earliest-start serial schedule for one precedence-feasible order.
/// Enumerating every such order spans all active schedules for this bounded classic
/// renewable-resource model; the minimum makespan is therefore exact for the
/// declared fixed-duration model, not merely a lower-bound comparison.
fn serial_schedule_for_order(
    order: &[String],
    by_id: &BTreeMap<String, &RepairTask>,
    capacities: &BTreeMap<String, u32>,
) -> Result<Vec<PlacedTask>, String> {
    let mut placed: Vec<PlacedTask> = Vec::with_capacity(order.len());
    let mut finish_by_id: BTreeMap<String, f64> = BTreeMap::new();

    for task_id in order {
        let task = by_id
            .get(task_id)
            .ok_or_else(|| format!("EXACT_ORACLE_UNKNOWN_TASK:{task_id}"))?;
        let release = task
            .predecessors
            .iter()
            .map(|predecessor| {
                finish_by_id
                    .get(predecessor)
                    .copied()
                    .ok_or_else(|| format!("EXACT_ORACLE_NON_TOPOLOGICAL_ORDER:{task_id}"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);

        let mut candidates = vec![release];
        candidates.extend(
            placed
                .iter()
                .map(|previous| previous.finish_hour)
                .filter(|finish| *finish >= release),
        );
        candidates.sort_by(f64::total_cmp);
        candidates.dedup_by(|left, right| (*left - *right).abs() <= EPS);

        let mut found = None;
        for start in candidates {
            let finish = start + task.duration_hours;
            if !finish.is_finite() {
                return Err(format!("EXACT_ORACLE_TIME_OVERFLOW:{task_id}"));
            }
            if resources_fit(start, finish, task, &placed, by_id, capacities) {
                found = Some((start, finish));
                break;
            }
        }

        let (start_hour, finish_hour) =
            found.ok_or_else(|| format!("EXACT_ORACLE_NO_FEASIBLE_INSERTION:{task_id}"))?;
        placed.push(PlacedTask {
            task_id: task_id.clone(),
            start_hour,
            finish_hour,
        });
        finish_by_id.insert(task_id.clone(), finish_hour);
    }
    Ok(placed)
}

fn resources_fit(
    start: f64,
    finish: f64,
    task: &RepairTask,
    placed: &[PlacedTask],
    by_id: &BTreeMap<String, &RepairTask>,
    capacities: &BTreeMap<String, u32>,
) -> bool {
    let mut boundaries = vec![start, finish];
    for previous in placed {
        if previous.start_hour > start && previous.start_hour < finish {
            boundaries.push(previous.start_hour);
        }
        if previous.finish_hour > start && previous.finish_hour < finish {
            boundaries.push(previous.finish_hour);
        }
    }
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|left, right| (*left - *right).abs() <= EPS);

    for segment in boundaries.windows(2) {
        if segment[1] <= segment[0] {
            continue;
        }
        let midpoint = segment[0] + (segment[1] - segment[0]) * 0.5;
        for requested in &task.crew_requirements {
            let used: u64 = placed
                .iter()
                .filter(|previous| {
                    previous.start_hour < midpoint && midpoint < previous.finish_hour
                })
                .filter_map(|previous| by_id.get(&previous.task_id))
                .flat_map(|previous_task| previous_task.crew_requirements.iter())
                .filter(|requirement| requirement.skill_id == requested.skill_id)
                .map(|requirement| u64::from(requirement.crews_required))
                .sum();
            let Some(capacity) = capacities.get(&requested.skill_id) else {
                return false;
            };
            if used + u64::from(requested.crews_required) > u64::from(*capacity) {
                return false;
            }
        }
    }
    true
}

fn compare(scenario: &RepairScheduleScenario) -> Result<Comparison, String> {
    let greedy = schedule_repairs(scenario);
    let report = greedy.report.ok_or_else(|| {
        format!(
            "GREEDY_SCHEDULER_NO_REPORT:{}:{}",
            scenario.scenario_id,
            greedy
                .reasons
                .first()
                .map(String::as_str)
                .unwrap_or("UNKNOWN")
        )
    })?;
    let exact = exact_minimum_makespan(scenario)?;
    if exact.makespan_hours > report.total_work_makespan_hours + EPS {
        return Err(format!(
            "ORACLE_EXCEEDS_FEASIBLE_GREEDY:{}",
            scenario.scenario_id
        ));
    }
    if exact.makespan_hours + EPS < report.makespan_lower_bound_hours {
        return Err(format!(
            "EXACT_RESULT_BELOW_VALID_LOWER_BOUND:{}",
            scenario.scenario_id
        ));
    }

    Ok(Comparison {
        greedy_makespan_hours: report.total_work_makespan_hours,
        greedy_lower_bound_hours: report.makespan_lower_bound_hours,
        exact,
    })
}

fn repair_task(
    task_id: &str,
    kind: RepairTaskKind,
    duration_hours: f64,
    predecessors: Vec<String>,
    skills: &[&str],
) -> RepairTask {
    RepairTask {
        task_id: task_id.to_owned(),
        kind,
        duration_hours,
        predecessors,
        crew_requirements: skills
            .iter()
            .map(|skill_id| CrewRequirement {
                skill_id: (*skill_id).to_owned(),
                crews_required: 1,
            })
            .collect(),
    }
}

/// Deliberately exposes an ID-order heuristic failure with a hand-checkable one-hour
/// gap: the exact schedule fills both B crews early, then places the joint task in
/// the resource window that minimizes the overall makespan.
fn hand_checked_fixture() -> RepairScheduleScenario {
    let repair_ids = vec![
        "a-assessment",
        "b-assessment",
        "c-clearance",
        "d-repair",
        "e-joint-repair",
    ];
    let mut tasks = vec![
        repair_task(
            "a-assessment",
            RepairTaskKind::Assessment,
            8.0,
            vec![],
            &["B"],
        ),
        repair_task(
            "b-assessment",
            RepairTaskKind::Assessment,
            10.0,
            vec![],
            &["B"],
        ),
        repair_task(
            "c-clearance",
            RepairTaskKind::Clearance,
            1.0,
            vec![],
            &["C"],
        ),
        repair_task("d-repair", RepairTaskKind::Repair, 12.0, vec![], &["B"]),
        repair_task(
            "e-joint-repair",
            RepairTaskKind::Repair,
            7.0,
            vec!["a-assessment".to_owned()],
            &["A", "B"],
        ),
    ];
    tasks.push(repair_task(
        "post-event-inspection",
        RepairTaskKind::PostEventInspection,
        1.0,
        repair_ids.iter().map(|id| (*id).to_owned()).collect(),
        &["inspector"],
    ));

    RepairScheduleScenario {
        scenario_id: "HAND-CHECKED-GREEDY-GAP-V1".to_owned(),
        evidence_class: EvidenceClass::Synthetic,
        source_refs: Vec::new(),
        crews: vec![
            CrewPool {
                skill_id: "A".to_owned(),
                available_crews: 1,
            },
            CrewPool {
                skill_id: "B".to_owned(),
                available_crews: 2,
            },
            CrewPool {
                skill_id: "C".to_owned(),
                available_crews: 1,
            },
            CrewPool {
                skill_id: "inspector".to_owned(),
                available_crews: 1,
            },
        ],
        tasks,
        service_enabling_task_ids: vec!["post-event-inspection".to_owned()],
        post_event_inspection_task_id: "post-event-inspection".to_owned(),
        reentry_authorization: GateState::Satisfied,
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn generated_fixture(seed: u64) -> RepairScheduleScenario {
    let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let repair_count = 3 + (seed as usize % 4);
    let skill_ids = ["civil", "power", "control"];
    let mut crews: Vec<CrewPool> = skill_ids
        .iter()
        .map(|skill_id| CrewPool {
            skill_id: (*skill_id).to_owned(),
            available_crews: 1 + (next_random(&mut state) % 2) as u32,
        })
        .collect();
    crews.push(CrewPool {
        skill_id: "inspector".to_owned(),
        available_crews: 1,
    });

    let mut tasks = Vec::with_capacity(repair_count + 1);
    let mut repair_ids = Vec::with_capacity(repair_count);
    for index in 0..repair_count {
        let task_id = format!("repair-{index:02}");
        let duration_hours = (1 + next_random(&mut state) % 12) as f64;
        let mask = 1 + (next_random(&mut state) % 7);
        let crew_requirements = skill_ids
            .iter()
            .enumerate()
            .filter_map(|(bit, skill_id)| {
                (mask & (1_u64 << bit) != 0).then(|| CrewRequirement {
                    skill_id: (*skill_id).to_owned(),
                    crews_required: 1,
                })
            })
            .collect();

        let predecessors = (0..index)
            .filter(|_| next_random(&mut state) % 5 == 0)
            .map(|predecessor| format!("repair-{predecessor:02}"))
            .collect();
        let kind = match index % 3 {
            0 => RepairTaskKind::Assessment,
            1 => RepairTaskKind::Repair,
            _ => RepairTaskKind::UtilityRestoration,
        };
        repair_ids.push(task_id.clone());
        tasks.push(RepairTask {
            task_id,
            kind,
            duration_hours,
            predecessors,
            crew_requirements,
        });
    }
    tasks.push(RepairTask {
        task_id: "post-event-inspection".to_owned(),
        kind: RepairTaskKind::PostEventInspection,
        duration_hours: (1 + next_random(&mut state) % 3) as f64,
        predecessors: repair_ids,
        crew_requirements: vec![CrewRequirement {
            skill_id: "inspector".to_owned(),
            crews_required: 1,
        }],
    });

    RepairScheduleScenario {
        scenario_id: format!("SYNTHETIC-SEED-{seed:03}"),
        evidence_class: EvidenceClass::Synthetic,
        source_refs: Vec::new(),
        crews,
        tasks,
        service_enabling_task_ids: vec!["post-event-inspection".to_owned()],
        post_event_inspection_task_id: "post-event-inspection".to_owned(),
        reentry_authorization: GateState::Satisfied,
    }
}

fn run_benchmark() -> Result<(), String> {
    let hand = compare(&hand_checked_fixture())?;
    let hand_gap = hand.greedy_makespan_hours - hand.exact.makespan_hours;
    println!("benchmark=bounded-repair-scheduler-exact-oracle");
    println!("scope=synthetic_fixed_durations_renewable_crews_no_physical_hazard_model");
    println!("exact_task_limit={MAX_EXACT_TASKS}");
    println!(
        "hand_checked_greedy_h={:.2} exact_h={:.2} gap_h={:.2} relative_gap={:.4} orders={}",
        hand.greedy_makespan_hours,
        hand.exact.makespan_hours,
        hand_gap,
        hand_gap / hand.exact.makespan_hours,
        hand.exact.orders_evaluated
    );
    println!(
        "hand_checked_exact_order={}",
        hand.exact.best_order.join(",")
    );

    let mut gap_sum = 0.0;
    let mut max_gap = 0.0_f64;
    let mut strict_losses = 0_u64;
    let mut total_orders = 0_usize;
    for seed in 0..GENERATED_FIXTURES {
        let result = compare(&generated_fixture(seed))?;
        let gap = (result.greedy_makespan_hours - result.exact.makespan_hours).max(0.0);
        let relative_gap = gap / result.exact.makespan_hours;
        gap_sum += relative_gap;
        max_gap = max_gap.max(relative_gap);
        if gap > EPS {
            strict_losses += 1;
        }
        total_orders = total_orders.saturating_add(result.exact.orders_evaluated);
    }
    println!(
        "generated_fixtures={GENERATED_FIXTURES} strict_greedy_losses={strict_losses} mean_relative_gap={:.4} max_relative_gap={:.4} total_topological_orders={total_orders}",
        gap_sum / GENERATED_FIXTURES as f64,
        max_gap
    );
    println!(
        "interpretation=exact_only_for_each_enumerated_small_fixture_no_SOTA_or_field_accuracy_claim"
    );
    Ok(())
}

fn main() {
    if let Err(error) = run_benchmark() {
        eprintln!("repair-optimality-benchmark failed closed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_checked_fixture_proves_greedy_can_lose_to_exact_optimum() {
        let result = compare(&hand_checked_fixture()).expect("hand fixture must be schedulable");
        assert!((result.greedy_makespan_hours - 21.0).abs() <= EPS);
        assert!((result.exact.makespan_hours - 20.0).abs() <= EPS);
        assert!(result.greedy_lower_bound_hours <= result.exact.makespan_hours + EPS);
        assert!(result.exact.orders_evaluated > 0);
        assert!((result.greedy_makespan_hours - result.exact.makespan_hours - 1.0).abs() <= EPS);
    }

    #[test]
    fn fifty_deterministic_fixtures_match_the_exhaustive_oracle_and_lower_bounds() {
        for seed in 0..GENERATED_FIXTURES {
            let scenario = generated_fixture(seed);
            let greedy = schedule_repairs(&scenario);
            let report = greedy
                .report
                .expect("generated scenario must produce a greedy schedule");
            let exact = exact_minimum_makespan(&scenario)
                .expect("bounded generated scenario must have an exact optimum");
            assert!(exact.orders_evaluated > 0, "seed={seed}");
            assert!(
                exact.makespan_hours + EPS >= report.makespan_lower_bound_hours,
                "exact optimum violated lower bound for seed={seed}"
            );
            assert!(
                exact.makespan_hours <= report.total_work_makespan_hours + EPS,
                "greedy schedule beat purported exact optimum for seed={seed}"
            );
        }
    }

    #[test]
    fn exact_oracle_fails_closed_above_declared_size_limit() {
        let mut scenario = generated_fixture(1);
        let template = scenario.tasks[0].clone();
        scenario.tasks.clear();
        for index in 0..=MAX_EXACT_TASKS {
            let mut task = template.clone();
            task.task_id = format!("oversize-{index:02}");
            task.predecessors.clear();
            scenario.tasks.push(task);
        }
        scenario.service_enabling_task_ids = vec!["oversize-00".to_owned()];
        scenario.post_event_inspection_task_id = "oversize-00".to_owned();
        scenario.tasks[0].kind = RepairTaskKind::PostEventInspection;
        let error =
            exact_minimum_makespan(&scenario).expect_err("oracle must refuse oversized case");
        assert!(error.starts_with("EXACT_ORACLE_TASK_LIMIT_EXCEEDED"));
    }
}
