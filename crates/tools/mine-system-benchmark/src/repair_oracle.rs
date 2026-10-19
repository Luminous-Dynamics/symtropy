// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact small-instance oracle for repair sequencing under crew, parts, access,
//! and precedence constraints. This is a synthetic scheduling contract, not a
//! mine-specific repair-time predictor, safety case, or operational scheduler.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const HARD_MAX_JOBS: usize = 12;
const HARD_MAX_HORIZON_TICKS: u64 = 64;
const HARD_MAX_SEARCH_NODES: u64 = 2_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartRequirement {
    pub part_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairJob {
    pub id: String,
    /// Service restored by this completed and accepted repair.
    pub target_service: String,
    pub duration_ticks: u64,
    pub crew_class: String,
    pub parts: Vec<PartRequirement>,
    pub prerequisite_jobs: Vec<String>,
    /// Each route must be initially open or opened by a completed prerequisite job.
    pub requires_access: Vec<String>,
    /// Access routes made available once this job completes.
    pub opens_access: Vec<String>,
    /// Weight for this job's target service completion time; zero for enabling work.
    pub critical_weight: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairProblem {
    pub case_id: String,
    pub horizon_ticks: u64,
    pub max_jobs: usize,
    pub max_search_nodes: u64,
    pub jobs: Vec<RepairJob>,
    /// Number of simultaneous crews available for each class.
    pub crew_counts: BTreeMap<String, usize>,
    /// Consumable initial parts inventory.
    pub parts_available: BTreeMap<String, u32>,
    pub initially_open_access: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledJob {
    pub job_id: String,
    pub start_tick: u64,
    pub finish_tick: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairPlan {
    pub case_id: String,
    /// Canonical job-ID order, independent of input Vec order.
    pub schedule: Vec<ScheduledJob>,
    /// Sum of critical_weight × target completion tick. Smaller is better.
    pub weighted_service_loss_ticks: u128,
    pub explored_nodes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepairError {
    InvalidInput(String),
    Infeasible,
    SearchLimitExceeded { limit: u64 },
}

impl fmt::Display for RepairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "invalid repair problem: {message}"),
            Self::Infeasible => write!(f, "no feasible repair schedule within the declared horizon"),
            Self::SearchLimitExceeded { limit } => {
                write!(f, "exact search limit exceeded: more than {limit} nodes")
            }
        }
    }
}
impl std::error::Error for RepairError {}

#[derive(Clone)]
struct ValidatedProblem {
    source: RepairProblem,
    jobs: Vec<RepairJob>,
    dependencies: Vec<Vec<usize>>,
}

struct Search<'a> {
    problem: &'a ValidatedProblem,
    assigned: Vec<Option<(u64, u64)>>,
    explored_nodes: u64,
    limit_hit: bool,
    best: Option<(u128, Vec<u64>)>,
}

impl Search<'_> {
    fn visit(&mut self, assigned_count: usize) {
        if self.limit_hit {
            return;
        }
        self.explored_nodes += 1;
        if self.explored_nodes > self.problem.source.max_search_nodes {
            self.limit_hit = true;
            return;
        }

        if assigned_count == self.assigned.len() {
            self.consider_complete_schedule();
            return;
        }

        // Deterministic first-ready rule only chooses enumeration order; every
        // feasible integer start time remains a branch of the exhaustive search.
        let Some(job_index) = (0..self.assigned.len()).find(|&index| {
            self.assigned[index].is_none()
                && self.problem.dependencies[index]
                    .iter()
                    .all(|&dependency| self.assigned[dependency].is_some())
        }) else {
            return;
        };

        let job = &self.problem.jobs[job_index];
        let earliest = self.problem.dependencies[job_index]
            .iter()
            .filter_map(|&dependency| self.assigned[dependency].map(|(_, finish)| finish))
            .max()
            .unwrap_or(0);
        let latest = self.problem.source.horizon_ticks - job.duration_ticks;
        if earliest > latest {
            return;
        }

        for start in earliest..=latest {
            let finish = start + job.duration_ticks;
            if !self.crew_available(job_index, start, finish)
                || !self.parts_available_after(job_index)
            {
                continue;
            }
            self.assigned[job_index] = Some((start, finish));
            self.visit(assigned_count + 1);
            self.assigned[job_index] = None;
            if self.limit_hit {
                return;
            }
        }
    }

    fn crew_available(&self, job_index: usize, start: u64, finish: u64) -> bool {
        let job = &self.problem.jobs[job_index];
        let capacity = self.problem.source.crew_counts[&job.crew_class];
        if capacity == 0 {
            return false;
        }
        for tick in start..finish {
            let active = self
                .assigned
                .iter()
                .enumerate()
                .filter(|(other_index, interval)| {
                    if *other_index == job_index {
                        return false;
                    }
                    let other = &self.problem.jobs[*other_index];
                    other.crew_class == job.crew_class
                        && interval.is_some_and(|(other_start, other_finish)| {
                            other_start <= tick && tick < other_finish
                        })
                })
                .count();
            if active >= capacity {
                return false;
            }
        }
        true
    }

    fn parts_available_after(&self, additional_job: usize) -> bool {
        let mut used = BTreeMap::<String, u32>::new();
        for (index, job) in self.problem.jobs.iter().enumerate() {
            if self.assigned[index].is_none() && index != additional_job {
                continue;
            }
            for part in &job.parts {
                let entry = used.entry(part.part_id.clone()).or_default();
                let Some(total) = entry.checked_add(part.quantity) else {
                    return false;
                };
                *entry = total;
                if total > self.problem.source.parts_available[&part.part_id] {
                    return false;
                }
            }
        }
        true
    }

    fn consider_complete_schedule(&mut self) {
        let mut objective = 0_u128;
        let mut starts = Vec::with_capacity(self.assigned.len());
        for (index, job) in self.problem.jobs.iter().enumerate() {
            let Some((start, finish)) = self.assigned[index] else {
                return;
            };
            starts.push(start);
            objective += u128::from(job.critical_weight) * u128::from(finish);
        }
        let better = match &self.best {
            None => true,
            Some((best_objective, best_starts)) => {
                objective < *best_objective
                    || (objective == *best_objective && starts < *best_starts)
            }
        };
        if better {
            self.best = Some((objective, starts));
        }
    }
}

/// Find a globally optimal schedule for a bounded synthetic repair problem.
///
/// Every declared job must complete within the horizon. Crew use is capacity
/// constrained over half-open intervals [start, finish); parts are consumed
/// once per job; prerequisites and access-opening jobs must finish by start.
/// The search fails explicitly on its node cap even if a provisional incumbent
/// exists, because a partial search is not proof of optimality.
pub fn solve_exact(problem: &RepairProblem) -> Result<RepairPlan, RepairError> {
    let validated = validate(problem)?;
    let mut search = Search {
        problem: &validated,
        assigned: vec![None; validated.jobs.len()],
        explored_nodes: 0,
        limit_hit: false,
        best: None,
    };
    search.visit(0);

    if search.limit_hit {
        return Err(RepairError::SearchLimitExceeded {
            limit: problem.max_search_nodes,
        });
    }
    let Some((objective, starts)) = search.best else {
        return Err(RepairError::Infeasible);
    };

    let mut schedule = Vec::with_capacity(validated.jobs.len());
    for (job, start_tick) in validated.jobs.iter().zip(starts) {
        schedule.push(ScheduledJob {
            job_id: job.id.clone(),
            start_tick,
            finish_tick: start_tick + job.duration_ticks,
        });
    }
    Ok(RepairPlan {
        case_id: problem.case_id.clone(),
        schedule,
        weighted_service_loss_ticks: objective,
        explored_nodes: search.explored_nodes,
    })
}

fn validate(problem: &RepairProblem) -> Result<ValidatedProblem, RepairError> {
    if problem.case_id.trim().is_empty() {
        return invalid("case ID is empty");
    }
    if problem.jobs.is_empty() {
        return invalid("repair problem has no jobs");
    }
    if problem.horizon_ticks == 0 || problem.horizon_ticks > HARD_MAX_HORIZON_TICKS {
        return invalid("horizon must be within the hard limit of 1..=64 ticks");
    }
    if problem.max_jobs == 0 || problem.max_jobs > HARD_MAX_JOBS {
        return invalid("configured job cap must be within the hard limit of 1..=12");
    }
    if problem.max_search_nodes == 0 || problem.max_search_nodes > HARD_MAX_SEARCH_NODES {
        return invalid("configured search-node cap must be within the hard limit");
    }
    if problem.jobs.len() > problem.max_jobs {
        return invalid("job count exceeds configured job cap");
    }

    for id in problem.crew_counts.keys() {
        if id.trim().is_empty() {
            return invalid("crew-class ID is empty");
        }
    }
    for id in problem.parts_available.keys() {
        if id.trim().is_empty() {
            return invalid("part ID is empty");
        }
    }
    for id in &problem.initially_open_access {
        if id.trim().is_empty() {
            return invalid("initial access ID is empty");
        }
    }

    let mut jobs = problem.jobs.clone();
    for job in &jobs {
        if job.id.trim().is_empty() || job.target_service.trim().is_empty() {
            return invalid("job ID and target service must be non-empty");
        }
        if job.crew_class.trim().is_empty()
            || !problem.crew_counts.contains_key(&job.crew_class)
        {
            return invalid(format!("job {} references undeclared crew class", job.id));
        }
        if job.duration_ticks == 0 || job.duration_ticks > problem.horizon_ticks {
            return invalid(format!("job {} has invalid duration", job.id));
        }

        let mut part_ids = BTreeSet::new();
        for part in &job.parts {
            if part.part_id.trim().is_empty() || part.quantity == 0 {
                return invalid(format!("job {} has invalid part requirement", job.id));
            }
            if !problem.parts_available.contains_key(&part.part_id) {
                return invalid(format!("job {} references undeclared part", job.id));
            }
            if !part_ids.insert(part.part_id.as_str()) {
                return invalid(format!("job {} repeats part {}", job.id, part.part_id));
            }
        }

        let mut prerequisite_ids = BTreeSet::new();
        for prerequisite in &job.prerequisite_jobs {
            if prerequisite == &job.id || !prerequisite_ids.insert(prerequisite) {
                return invalid(format!("job {} has self or duplicate prerequisite", job.id));
            }
        }
        let mut access_ids = BTreeSet::new();
        for access in &job.requires_access {
            if access.trim().is_empty() || !access_ids.insert(access) {
                return invalid(format!("job {} has empty or duplicate access requirement", job.id));
            }
        }
        let mut opened_ids = BTreeSet::new();
        for access in &job.opens_access {
            if access.trim().is_empty() || !opened_ids.insert(access) {
                return invalid(format!("job {} has empty or duplicate access opening", job.id));
            }
        }
    }
    jobs.sort_by(|a, b| a.id.cmp(&b.id));

    let mut id_to_index = BTreeMap::new();
    for (index, job) in jobs.iter().enumerate() {
        if id_to_index.insert(job.id.clone(), index).is_some() {
            return invalid(format!("duplicate job ID {}", job.id));
        }
    }

    let mut opener_by_access = BTreeMap::<String, String>::new();
    for job in &jobs {
        for access in &job.opens_access {
            if problem.initially_open_access.contains(access) {
                return invalid(format!("job {} opens access that is already initially open", job.id));
            }
            if opener_by_access.insert(access.clone(), job.id.clone()).is_some() {
                return invalid(format!("multiple jobs open access route {access}"));
            }
        }
    }

    let mut dependency_ids = BTreeMap::<String, BTreeSet<String>>::new();
    for job in &jobs {
        let mut deps = BTreeSet::new();
        for prerequisite in &job.prerequisite_jobs {
            if !id_to_index.contains_key(prerequisite) {
                return invalid(format!("job {} references missing prerequisite {prerequisite}", job.id));
            }
            deps.insert(prerequisite.clone());
        }
        for access in &job.requires_access {
            if problem.initially_open_access.contains(access) {
                continue;
            }
            let Some(opener) = opener_by_access.get(access) else {
                return invalid(format!("job {} requires access {access} with no opener", job.id));
            };
            if opener == &job.id {
                return invalid(format!("job {} requires access that it itself opens", job.id));
            }
            deps.insert(opener.clone());
        }
        dependency_ids.insert(job.id.clone(), deps);
    }

    fn visit(
        id: &str,
        dependencies: &BTreeMap<String, BTreeSet<String>>,
        marks: &mut BTreeMap<String, u8>,
    ) -> Result<(), RepairError> {
        match marks.get(id).copied() {
            Some(1) => return invalid(format!("repair dependency cycle reaches {id}")),
            Some(2) => return Ok(()),
            _ => {}
        }
        marks.insert(id.to_owned(), 1);
        for dependency in &dependencies[id] {
            visit(dependency, dependencies, marks)?;
        }
        marks.insert(id.to_owned(), 2);
        Ok(())
    }

    let mut marks = BTreeMap::new();
    for id in dependency_ids.keys() {
        visit(id, &dependency_ids, &mut marks)?;
    }

    let dependencies = jobs
        .iter()
        .map(|job| {
            dependency_ids[&job.id]
                .iter()
                .map(|id| id_to_index[id])
                .collect::<Vec<_>>()
        })
        .collect();

    Ok(ValidatedProblem {
        source: problem.clone(),
        jobs,
        dependencies,
    })
}

fn invalid<T>(message: impl Into<String>) -> Result<T, RepairError> {
    Err(RepairError::InvalidInput(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(
        id: &str,
        duration_ticks: u64,
        crew_class: &str,
        critical_weight: u64,
    ) -> RepairJob {
        RepairJob {
            id: id.into(),
            target_service: format!("service-{id}"),
            duration_ticks,
            crew_class: crew_class.into(),
            parts: Vec::new(),
            prerequisite_jobs: Vec::new(),
            requires_access: Vec::new(),
            opens_access: Vec::new(),
            critical_weight,
        }
    }

    fn problem(jobs: Vec<RepairJob>, horizon_ticks: u64) -> RepairProblem {
        RepairProblem {
            case_id: "SYNTHETIC-REPAIR-ORACLE".into(),
            horizon_ticks,
            max_jobs: 8,
            max_search_nodes: 100_000,
            jobs,
            crew_counts: BTreeMap::from([("mechanic".into(), 1), ("access".into(), 1)]),
            parts_available: BTreeMap::new(),
            initially_open_access: BTreeSet::new(),
        }
    }

    fn entry<'a>(plan: &'a RepairPlan, job_id: &str) -> &'a ScheduledJob {
        plan.schedule.iter().find(|job| job.job_id == job_id).unwrap()
    }

    #[test]
    fn one_crew_prioritizes_the_job_that_restores_more_critical_service() {
        let input = problem(
            vec![job("pump", 2, "mechanic", 10), job("aux", 3, "mechanic", 1)],
            6,
        );
        let plan = solve_exact(&input).unwrap();
        assert_eq!(plan.weighted_service_loss_ticks, 25);
        assert_eq!((entry(&plan, "pump").start_tick, entry(&plan, "pump").finish_tick), (0, 2));
        assert_eq!((entry(&plan, "aux").start_tick, entry(&plan, "aux").finish_tick), (2, 5));
    }

    #[test]
    fn independent_crews_allow_parallel_recovery() {
        let input = problem(
            vec![job("vent", 3, "mechanic", 10), job("pump", 3, "access", 10)],
            4,
        );
        let plan = solve_exact(&input).unwrap();
        assert_eq!(plan.weighted_service_loss_ticks, 60);
        assert_eq!(entry(&plan, "vent").start_tick, 0);
        assert_eq!(entry(&plan, "pump").start_tick, 0);
    }

    #[test]
    fn repair_waits_for_access_opening_job() {
        let mut clear_route = job("clear-route", 2, "access", 0);
        clear_route.opens_access = vec!["pump-gallery".into()];
        let mut repair_pump = job("repair-pump", 3, "mechanic", 10);
        repair_pump.requires_access = vec!["pump-gallery".into()];
        let plan = solve_exact(&problem(vec![repair_pump, clear_route], 6)).unwrap();
        assert_eq!(entry(&plan, "clear-route").finish_tick, 2);
        assert_eq!(entry(&plan, "repair-pump").start_tick, 2);
        assert_eq!(entry(&plan, "repair-pump").finish_tick, 5);
        assert_eq!(plan.weighted_service_loss_ticks, 50);
    }

    #[test]
    fn consumable_parts_cannot_be_double_allocated() {
        let mut a = job("pump-a", 1, "mechanic", 5);
        a.parts = vec![PartRequirement { part_id: "seal".into(), quantity: 1 }];
        let mut b = job("pump-b", 1, "access", 1);
        b.parts = vec![PartRequirement { part_id: "seal".into(), quantity: 1 }];
        let mut input = problem(vec![a, b], 3);
        input.parts_available.insert("seal".into(), 1);
        assert_eq!(solve_exact(&input), Err(RepairError::Infeasible));
    }

    #[test]
    fn input_order_does_not_change_the_canonical_optimum() {
        let jobs = vec![job("b", 2, "mechanic", 2), job("a", 1, "mechanic", 5)];
        let first = solve_exact(&problem(jobs.clone(), 5)).unwrap();
        let second = solve_exact(&problem(jobs.into_iter().rev().collect(), 5)).unwrap();
        assert_eq!(first.schedule, second.schedule);
        assert_eq!(first.weighted_service_loss_ticks, second.weighted_service_loss_ticks);
    }

    #[test]
    fn exactness_claim_is_rejected_after_search_cap() {
        let mut input = problem(vec![job("a", 1, "mechanic", 1), job("b", 1, "mechanic", 1)], 5);
        input.max_search_nodes = 1;
        assert!(matches!(solve_exact(&input), Err(RepairError::SearchLimitExceeded { limit: 1 })));
    }

    #[test]
    fn cyclic_and_undeclared_access_are_rejected() {
        let mut a = job("a", 1, "mechanic", 1);
        let mut b = job("b", 1, "mechanic", 1);
        a.prerequisite_jobs = vec!["b".into()];
        b.prerequisite_jobs = vec!["a".into()];
        assert!(matches!(solve_exact(&problem(vec![a, b], 5)), Err(RepairError::InvalidInput(_))));

        let mut no_opener = job("pump", 1, "mechanic", 1);
        no_opener.requires_access = vec!["closed-gallery".into()];
        assert!(matches!(solve_exact(&problem(vec![no_opener], 5)), Err(RepairError::InvalidInput(_))));
    }

    #[test]
    fn exact_solver_matches_independent_two_job_start_grid_oracle() {
        let weights = [0_u64, 1, 3];
        let mut checked = 0_u32;

        for duration_a in 1_u64..=3 {
            for duration_b in 1_u64..=3 {
                for weight_a in weights {
                    for weight_b in weights {
                        for horizon in duration_a.max(duration_b)..=5 {
                            for crew_count in [1_usize, 2] {
                                if crew_count == 1 && horizon < duration_a + duration_b {
                                    continue;
                                }
                                let input = RepairProblem {
                                    horizon_ticks: horizon,
                                    crew_counts: BTreeMap::from([
                                        ("mechanic".into(), crew_count),
                                        ("access".into(), 1),
                                    ]),
                                    jobs: vec![
                                        job("a", duration_a, "mechanic", weight_a),
                                        job("b", duration_b, "mechanic", weight_b),
                                    ],
                                    ..problem(Vec::new(), horizon)
                                };
                                // The independent oracle enumerates the full
                                // Cartesian product of start times. It does not
                                // use the production solver's dependency-first
                                // branch order or resource-feasibility routines.
                                let mut expected: Option<u128> = None;
                                for start_a in 0..=horizon - duration_a {
                                    for start_b in 0..=horizon - duration_b {
                                        let finish_a = start_a + duration_a;
                                        let finish_b = start_b + duration_b;
                                        let overlaps = start_a < finish_b && start_b < finish_a;
                                        if crew_count == 1 && overlaps {
                                            continue;
                                        }
                                        let objective = u128::from(weight_a)
                                            * u128::from(finish_a)
                                            + u128::from(weight_b) * u128::from(finish_b);
                                        expected = Some(expected.map_or(objective, |best| best.min(objective)));
                                    }
                                }
                                let expected = expected.expect("at least one feasible two-job schedule");
                                let actual = solve_exact(&input).unwrap();
                                assert_eq!(
                                    actual.weighted_service_loss_ticks, expected,
                                    "durations=({duration_a},{duration_b}), weights=({weight_a},{weight_b}), horizon={horizon}, crews={crew_count}"
                                );
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 450);
    }

}
