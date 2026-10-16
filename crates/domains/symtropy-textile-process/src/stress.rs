// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic stress-corpus evaluation for textile schedules.
//!
//! Stress cases are explicit perturbations of an input planning problem. They
//! are equally enumerated fixtures, not probability samples unless an external
//! study independently establishes that interpretation. The exact bounded
//! solver distinguishes a truly infeasible scenario from a dispatch heuristic
//! that fails to find a feasible plan.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::scheduler::{
    DispatchRule, ObjectiveOptima, ScheduleError, ScheduleMetrics, ScheduleProblem,
    ScheduledOperation, dispatch_schedule, solve_exhaustive,
};

pub const MAX_STRESS_SCENARIOS: usize = 32;
pub const STRESS_SCOPE_NOTE: &str = concat!(
    "Stress-corpus counts and percentiles describe the enumerated scenarios; ",
    "they do not estimate real-world probabilities or prove factory savings."
);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessingDelay {
    pub job_id: String,
    pub additional_ticks: u64,
}

/// Additional delay before the job's inputs or specialist preparation are ready.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobReleaseDelay {
    pub job_id: String,
    /// Additional delay before the job's inputs or specialist preparation are ready.
    pub additional_ticks: u64,
}

/// Machine unavailability through an absolute tick in the problem's declared
/// time unit. The current v0.1 model treats this as a pre-horizon availability
/// shift; it does not simulate failures during an in-progress operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineAvailabilityDelay {
    pub machine_id: String,
    pub unavailable_until_tick: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtilityBudgetShock {
    /// A revised hard budget; it may tighten or preserve, never loosen, the baseline cap.
    pub energy_budget_units: Option<u64>,
    pub water_budget_units: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StressScenario {
    pub id: String,
    /// Stable fixture or signed manifest reference; not authentication.
    pub fixture_ref: String,
    pub processing_delays: Vec<ProcessingDelay>,
    pub release_delays: Vec<JobReleaseDelay>,
    pub utility_budget_shock: Option<UtilityBudgetShock>,
    pub machine_availability_delays: Vec<MachineAvailabilityDelay>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StressCorpus {
    pub id: String,
    pub version: String,
    pub scenarios: Vec<StressScenario>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScenarioExactOutcome {
    Feasible {
        frontier_size: usize,
        objective_optima: ObjectiveOptima,
    },
    NoFeasibleSchedule,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectiveGap {
    pub late_jobs: u64,
    pub total_tardiness_ticks: u64,
    pub makespan_tick: u64,
    pub energy_units: u64,
    pub water_units: u64,
    pub setup_duration_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleScenarioResult {
    pub rule: DispatchRule,
    /// True iff this heuristic produced a schedule satisfying the exact
    /// instance's hard constraints.
    pub plan_found: bool,
    pub failure: Option<ScheduleError>,
    pub metrics: Option<ScheduleMetrics>,
    /// Full candidate for downstream auditing; profile and functional-unit IDs
    /// remain attached to each operation.
    pub operations: Option<Vec<ScheduledOperation>>,
    /// Differences from independently best exact values for each objective.
    /// These optima are not necessarily jointly attainable by one schedule.
    pub gap_from_individual_optima: Option<ObjectiveGap>,
    /// True when the exact reference found a schedule but this dispatch rule
    /// did not. That is a heuristic failure, not an infeasible factory case.
    pub missed_feasible_schedule: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioResult {
    /// Original shock inputs and fixture reference for audit/replay.
    pub scenario: StressScenario,
    pub scenario_id: String,
    pub fixture_ref: String,
    pub exact_outcome: ScenarioExactOutcome,
    pub dispatch_results: Vec<RuleScenarioResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricPercentiles {
    /// Nearest-rank median over dispatch plans found in exactly feasible cases.
    pub p50: Option<u64>,
    /// Nearest-rank 95th percentile over dispatch plans found in exactly
    /// feasible cases; not an estimated probability of a real-world tail event.
    pub p95: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleStressSummary {
    pub rule: DispatchRule,
    pub exact_feasible_scenarios: usize,
    pub plans_found_on_feasible_scenarios: usize,
    pub feasible_scenarios_missed: usize,
    pub total_tardiness_ticks: MetricPercentiles,
    pub late_jobs: MetricPercentiles,
    pub makespan_tick: MetricPercentiles,
    pub energy_units: MetricPercentiles,
    pub water_units: MetricPercentiles,
    pub setup_duration_ticks: MetricPercentiles,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StressReport {
    pub problem_id: String,
    /// Exact planning problem before any scenario perturbations.
    pub baseline_problem: ScheduleProblem,
    pub corpus_id: String,
    pub corpus_version: String,
    pub scenario_count: usize,
    pub exact_feasible_scenarios: usize,
    pub exact_no_schedule_scenarios: usize,
    pub scenario_results: Vec<ScenarioResult>,
    pub rule_summaries: Vec<RuleStressSummary>,
    pub interpretation_boundary: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StressHarnessError {
    EmptyProblemId,
    EmptyCorpusId,
    EmptyCorpusVersion,
    EmptyScenarioSet,
    TooManyScenarios { actual: usize, maximum: usize },
    EmptyScenarioId,
    DuplicateScenarioId { scenario_id: String },
    EmptyFixtureReference { scenario_id: String },
    EmptyJobIdInDelay { scenario_id: String },
    DuplicateJobDelay { scenario_id: String, job_id: String },
    UnknownJobInDelay { scenario_id: String, job_id: String },
    ZeroProcessingDelay { scenario_id: String, job_id: String },
    ProcessingDelayOverflow { scenario_id: String, job_id: String },
    EmptyJobIdInReleaseDelay { scenario_id: String },
    DuplicateJobReleaseDelay { scenario_id: String, job_id: String },
    UnknownJobInReleaseDelay { scenario_id: String, job_id: String },
    ZeroReleaseDelay { scenario_id: String, job_id: String },
    ReleaseDelayOverflow { scenario_id: String, job_id: String },
    EmptyUtilityBudgetShock { scenario_id: String },
    LoosenedEnergyBudget { scenario_id: String },
    LoosenedWaterBudget { scenario_id: String },
    EmptyMachineIdInDelay { scenario_id: String },
    DuplicateMachineDelay { scenario_id: String, machine_id: String },
    UnknownMachineInDelay { scenario_id: String, machine_id: String },
    ScenarioEvaluationRejected { scenario_id: String, source: ScheduleError },
    DispatchContradictsExactOracle { scenario_id: String, rule: DispatchRule },
    HeuristicBetterThanExactOptimum {
        scenario_id: String,
        rule: DispatchRule,
        objective: &'static str,
    },
}

impl fmt::Display for StressHarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "textile stress harness error: {self:?}")
    }
}

impl Error for StressHarnessError {}

const DISPATCH_RULES: [DispatchRule; 4] = [
    DispatchRule::EarliestDue,
    DispatchRule::LowestSetup,
    DispatchRule::LowestEnergy,
    DispatchRule::LeastSlack,
];

/// Evaluate each stress scenario using a bounded exact solver and three
/// deterministic dispatch rules. This is intended for small oracle cases.
pub fn evaluate_stress_corpus(
    baseline: &ScheduleProblem,
    corpus: &StressCorpus,
) -> Result<StressReport, StressHarnessError> {
    if baseline.id.trim().is_empty() {
        return Err(StressHarnessError::EmptyProblemId);
    }
    if corpus.id.trim().is_empty() {
        return Err(StressHarnessError::EmptyCorpusId);
    }
    if corpus.version.trim().is_empty() {
        return Err(StressHarnessError::EmptyCorpusVersion);
    }
    if corpus.scenarios.is_empty() {
        return Err(StressHarnessError::EmptyScenarioSet);
    }
    if corpus.scenarios.len() > MAX_STRESS_SCENARIOS {
        return Err(StressHarnessError::TooManyScenarios {
            actual: corpus.scenarios.len(),
            maximum: MAX_STRESS_SCENARIOS,
        });
    }

    let job_ids: BTreeSet<&str> = baseline.jobs.iter().map(|job| job.id.as_str()).collect();
    let machine_ids: BTreeSet<&str> = baseline
        .machines
        .iter()
        .map(|machine| machine.id.as_str())
        .collect();
    let mut scenario_ids = BTreeSet::new();
    let mut scenario_results = Vec::with_capacity(corpus.scenarios.len());

    for scenario in &corpus.scenarios {
        if scenario.id.trim().is_empty() {
            return Err(StressHarnessError::EmptyScenarioId);
        }
        if !scenario_ids.insert(scenario.id.as_str()) {
            return Err(StressHarnessError::DuplicateScenarioId {
                scenario_id: scenario.id.clone(),
            });
        }
        if scenario.fixture_ref.trim().is_empty() {
            return Err(StressHarnessError::EmptyFixtureReference {
                scenario_id: scenario.id.clone(),
            });
        }

        let mut perturbed = baseline.clone();
        let mut changed_jobs = BTreeSet::new();
        for shock in &scenario.processing_delays {
            if shock.job_id.trim().is_empty() {
                return Err(StressHarnessError::EmptyJobIdInDelay {
                    scenario_id: scenario.id.clone(),
                });
            }
            if !changed_jobs.insert(shock.job_id.as_str()) {
                return Err(StressHarnessError::DuplicateJobDelay {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                });
            }
            if !job_ids.contains(shock.job_id.as_str()) {
                return Err(StressHarnessError::UnknownJobInDelay {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                });
            }
            if shock.additional_ticks == 0 {
                return Err(StressHarnessError::ZeroProcessingDelay {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                });
            }
            let job = perturbed
                .jobs
                .iter_mut()
                .find(|job| job.id == shock.job_id)
                .expect("job ID validated against baseline");
            job.processing_ticks = job
                .processing_ticks
                .checked_add(shock.additional_ticks)
                .ok_or_else(|| StressHarnessError::ProcessingDelayOverflow {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                })?;
        }

        let mut delayed_jobs = BTreeSet::new();
        for shock in &scenario.release_delays {
            if shock.job_id.trim().is_empty() {
                return Err(StressHarnessError::EmptyJobIdInReleaseDelay {
                    scenario_id: scenario.id.clone(),
                });
            }
            if !delayed_jobs.insert(shock.job_id.as_str()) {
                return Err(StressHarnessError::DuplicateJobReleaseDelay {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                });
            }
            if !job_ids.contains(shock.job_id.as_str()) {
                return Err(StressHarnessError::UnknownJobInReleaseDelay {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                });
            }
            if shock.additional_ticks == 0 {
                return Err(StressHarnessError::ZeroReleaseDelay {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                });
            }
            let job = perturbed
                .jobs
                .iter_mut()
                .find(|job| job.id == shock.job_id)
                .expect("job ID validated against baseline");
            job.release_tick = job
                .release_tick
                .checked_add(shock.additional_ticks)
                .ok_or_else(|| StressHarnessError::ReleaseDelayOverflow {
                    scenario_id: scenario.id.clone(),
                    job_id: shock.job_id.clone(),
                })?;
        }

        if let Some(shock) = &scenario.utility_budget_shock {
            if shock.energy_budget_units.is_none() && shock.water_budget_units.is_none() {
                return Err(StressHarnessError::EmptyUtilityBudgetShock {
                    scenario_id: scenario.id.clone(),
                });
            }
            if shock
                .energy_budget_units
                .is_some_and(|budget| budget > baseline.energy_budget_units)
            {
                return Err(StressHarnessError::LoosenedEnergyBudget {
                    scenario_id: scenario.id.clone(),
                });
            }
            if shock
                .water_budget_units
                .is_some_and(|budget| budget > baseline.water_budget_units)
            {
                return Err(StressHarnessError::LoosenedWaterBudget {
                    scenario_id: scenario.id.clone(),
                });
            }
            if let Some(budget) = shock.energy_budget_units {
                perturbed.energy_budget_units = budget;
            }
            if let Some(budget) = shock.water_budget_units {
                perturbed.water_budget_units = budget;
            }
        }

        let mut changed_machines = BTreeSet::new();
        for shock in &scenario.machine_availability_delays {
            if shock.machine_id.trim().is_empty() {
                return Err(StressHarnessError::EmptyMachineIdInDelay {
                    scenario_id: scenario.id.clone(),
                });
            }
            if !changed_machines.insert(shock.machine_id.as_str()) {
                return Err(StressHarnessError::DuplicateMachineDelay {
                    scenario_id: scenario.id.clone(),
                    machine_id: shock.machine_id.clone(),
                });
            }
            if !machine_ids.contains(shock.machine_id.as_str()) {
                return Err(StressHarnessError::UnknownMachineInDelay {
                    scenario_id: scenario.id.clone(),
                    machine_id: shock.machine_id.clone(),
                });
            }
            let machine = perturbed
                .machines
                .iter_mut()
                .find(|machine| machine.id == shock.machine_id)
                .expect("machine ID validated against baseline");
            machine.available_tick = machine
                .available_tick
                .max(shock.unavailable_until_tick);
        }

        let exact = match solve_exhaustive(&perturbed) {
            Ok(report) => ScenarioExactOutcome::Feasible {
                frontier_size: report.pareto_frontier.len(),
                objective_optima: report.objective_optima,
            },
            Err(ScheduleError::NoFeasibleSchedule) => ScenarioExactOutcome::NoFeasibleSchedule,
            Err(source) => {
                return Err(StressHarnessError::ScenarioEvaluationRejected {
                    scenario_id: scenario.id.clone(),
                    source,
                });
            }
        };
        let exact_optima = match &exact {
            ScenarioExactOutcome::Feasible { objective_optima, .. } => {
                Some(objective_optima.clone())
            }
            ScenarioExactOutcome::NoFeasibleSchedule => None,
        };

        let mut dispatch_results = Vec::with_capacity(DISPATCH_RULES.len());
        for rule in DISPATCH_RULES {
            match dispatch_schedule(&perturbed, rule) {
                Ok(candidate) => {
                    if exact_optima.is_none() {
                        return Err(StressHarnessError::DispatchContradictsExactOracle {
                            scenario_id: scenario.id.clone(),
                            rule,
                        });
                    }
                    let optima = exact_optima.as_ref().expect("exact optima exists");
                    let gap = calculate_gap(
                        &scenario.id,
                        rule,
                        &candidate.metrics,
                        optima,
                    )?;
                    dispatch_results.push(RuleScenarioResult {
                        rule,
                        plan_found: true,
                        failure: None,
                        metrics: Some(candidate.metrics),
                        operations: Some(candidate.operations),
                        gap_from_individual_optima: Some(gap),
                        missed_feasible_schedule: false,
                    });
                }
                Err(source @ ScheduleError::NoFeasibleDispatch { .. }) => {
                    dispatch_results.push(RuleScenarioResult {
                        rule,
                        plan_found: false,
                        failure: Some(source),
                        metrics: None,
                        operations: None,
                        gap_from_individual_optima: None,
                        missed_feasible_schedule: exact_optima.is_some(),
                    });
                }
                Err(source) => {
                    return Err(StressHarnessError::ScenarioEvaluationRejected {
                        scenario_id: scenario.id.clone(),
                        source,
                    });
                }
            }
        }

        scenario_results.push(ScenarioResult {
            scenario: scenario.clone(),
            scenario_id: scenario.id.clone(),
            fixture_ref: scenario.fixture_ref.clone(),
            exact_outcome: exact,
            dispatch_results,
        });
    }

    let exact_feasible_scenarios = scenario_results
        .iter()
        .filter(|result| matches!(&result.exact_outcome, ScenarioExactOutcome::Feasible { .. }))
        .count();
    let exact_no_schedule_scenarios = scenario_results.len() - exact_feasible_scenarios;
    let rule_summaries = DISPATCH_RULES
        .iter()
        .copied()
        .map(|rule| summarize_rule(rule, &scenario_results, exact_feasible_scenarios))
        .collect();

    Ok(StressReport {
        problem_id: baseline.id.clone(),
        baseline_problem: baseline.clone(),
        corpus_id: corpus.id.clone(),
        corpus_version: corpus.version.clone(),
        scenario_count: scenario_results.len(),
        exact_feasible_scenarios,
        exact_no_schedule_scenarios,
        scenario_results,
        rule_summaries,
        interpretation_boundary: STRESS_SCOPE_NOTE.to_owned(),
    })
}

fn calculate_gap(
    scenario_id: &str,
    rule: DispatchRule,
    metrics: &ScheduleMetrics,
    optima: &ObjectiveOptima,
) -> Result<ObjectiveGap, StressHarnessError> {
    fn gap(
        actual: u64,
        optimum: u64,
        scenario_id: &str,
        rule: DispatchRule,
        objective: &'static str,
    ) -> Result<u64, StressHarnessError> {
        actual
            .checked_sub(optimum)
            .ok_or(StressHarnessError::HeuristicBetterThanExactOptimum {
                scenario_id: scenario_id.to_owned(),
                rule,
                objective,
            })
    }

    Ok(ObjectiveGap {
        late_jobs: gap(
            metrics.late_jobs,
            optima.minimum_late_jobs,
            scenario_id,
            rule,
            "late_jobs",
        )?,
        total_tardiness_ticks: gap(
            metrics.total_tardiness_ticks,
            optima.minimum_total_tardiness_ticks,
            scenario_id,
            rule,
            "total_tardiness_ticks",
        )?,
        makespan_tick: gap(
            metrics.makespan_tick,
            optima.minimum_makespan_tick,
            scenario_id,
            rule,
            "makespan_tick",
        )?,
        energy_units: gap(
            metrics.energy_units,
            optima.minimum_energy_units,
            scenario_id,
            rule,
            "energy_units",
        )?,
        water_units: gap(
            metrics.water_units,
            optima.minimum_water_units,
            scenario_id,
            rule,
            "water_units",
        )?,
        setup_duration_ticks: gap(
            metrics.setup_duration_ticks,
            optima.minimum_setup_duration_ticks,
            scenario_id,
            rule,
            "setup_duration_ticks",
        )?,
    })
}

fn summarize_rule(
    rule: DispatchRule,
    scenarios: &[ScenarioResult],
    exact_feasible_scenarios: usize,
) -> RuleStressSummary {
    let selected: Vec<&ScheduleMetrics> = scenarios
        .iter()
        .filter_map(|scenario| {
            if !matches!(&scenario.exact_outcome, ScenarioExactOutcome::Feasible { .. }) {
                return None;
            }
            scenario
                .dispatch_results
                .iter()
                .find(|result| result.rule == rule)
                .and_then(|result| result.metrics.as_ref())
        })
        .collect();

    let plans_found_on_feasible_scenarios = selected.len();
    debug_assert!(plans_found_on_feasible_scenarios <= exact_feasible_scenarios);
    let feasible_scenarios_missed =
        exact_feasible_scenarios - plans_found_on_feasible_scenarios;
    let metric = |select: fn(&ScheduleMetrics) -> u64| {
        let mut values: Vec<u64> = selected.iter().map(|item| select(item)).collect();
        percentiles(&mut values)
    };

    RuleStressSummary {
        rule,
        exact_feasible_scenarios,
        plans_found_on_feasible_scenarios,
        feasible_scenarios_missed,
        total_tardiness_ticks: metric(|item| item.total_tardiness_ticks),
        late_jobs: metric(|item| item.late_jobs),
        makespan_tick: metric(|item| item.makespan_tick),
        energy_units: metric(|item| item.energy_units),
        water_units: metric(|item| item.water_units),
        setup_duration_ticks: metric(|item| item.setup_duration_ticks),
    }
}

fn percentiles(values: &mut [u64]) -> MetricPercentiles {
    if values.is_empty() {
        return MetricPercentiles { p50: None, p95: None };
    }
    values.sort_unstable();
    let nearest_rank = |percent: usize| {
        let one_based_rank = (percent * values.len()).div_ceil(100).max(1);
        values[one_based_rank - 1]
    };
    MetricPercentiles {
        p50: Some(nearest_rank(50)),
        p95: Some(nearest_rank(95)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{GateEvidence, GateOutcome, Machine, TextileJob};
    use std::collections::BTreeSet;

    fn fixture_problem() -> ScheduleProblem {
        ScheduleProblem {
            id: "single-job-stress-fixture".to_owned(),
            time_unit_id: "minute-tick-v0".to_owned(),
            energy_unit_id: "energy-unit-v0".to_owned(),
            water_unit_id: "water-unit-v0".to_owned(),
            energy_budget_units: 4,
            water_budget_units: 4,
            jobs: vec![TextileJob {
                id: "ceremonial-repair".to_owned(),
                profile_id: "ceremonial-or-heritage-v0".to_owned(),
                functional_unit_id: "service-event-v0".to_owned(),
                setup_family: "specialist-repair".to_owned(),
                required_capabilities: BTreeSet::from(["heritage-safe".to_owned()]),
                release_tick: 0,
                due_tick: 5,
                hard_finish_by_tick: Some(10),
                processing_ticks: 3,
                energy_units: 1,
                water_units: 1,
                process_evidence_ref: "fixture:repair-procedure-v0".to_owned(),
                required_gates: vec![GateEvidence {
                    id: "repair-readiness".to_owned(),
                    outcome: GateOutcome::Pass {
                        evidence_ref: "fixture:qualified-materials-review".to_owned(),
                    },
                }],
            }],
            machines: vec![Machine {
                id: "heritage-repair-cell".to_owned(),
                capabilities: BTreeSet::from(["heritage-safe".to_owned()]),
                initial_setup_family: "specialist-repair".to_owned(),
                available_tick: 0,
                capability_evidence_ref: "fixture:cell-qualification-v0".to_owned(),
            }],
            setup_transitions: vec![],
        }
    }

    fn scenario(
        id: &str,
        processing_delays: Vec<ProcessingDelay>,
        machine_availability_delays: Vec<MachineAvailabilityDelay>,
    ) -> StressScenario {
        StressScenario {
            id: id.to_owned(),
            fixture_ref: format!("fixture:stress:{id}"),
            processing_delays,
            release_delays: vec![],
            utility_budget_shock: None,
            machine_availability_delays,
        }
    }

    #[test]
    fn exact_oracle_distinguishes_duration_shock_from_hard_deadline_infeasibility() {
        let corpus = StressCorpus {
            id: "repair-resilience-v0".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![
                scenario("nominal", vec![], vec![]),
                scenario(
                    "slow-repair",
                    vec![ProcessingDelay {
                        job_id: "ceremonial-repair".to_owned(),
                        additional_ticks: 3,
                    }],
                    vec![],
                ),
                StressScenario {
                    id: "specialist-material-arrives-late".to_owned(),
                    fixture_ref: "fixture:stress:specialist-material-arrives-late".to_owned(),
                    processing_delays: vec![],
                    release_delays: vec![JobReleaseDelay {
                        job_id: "ceremonial-repair".to_owned(),
                        additional_ticks: 2,
                    }],
                    utility_budget_shock: None,
                    machine_availability_delays: vec![],
                },
                StressScenario {
                    id: "energy-curtailed".to_owned(),
                    fixture_ref: "fixture:stress:energy-curtailed".to_owned(),
                    processing_delays: vec![],
                    release_delays: vec![],
                    utility_budget_shock: Some(UtilityBudgetShock {
                        energy_budget_units: Some(0),
                        water_budget_units: None,
                    }),
                    machine_availability_delays: vec![],
                },
                scenario(
                    "cell-unavailable-too-long",
                    vec![],
                    vec![MachineAvailabilityDelay {
                        machine_id: "heritage-repair-cell".to_owned(),
                        unavailable_until_tick: 8,
                    }],
                ),
            ],
        };

        let report = evaluate_stress_corpus(&fixture_problem(), &corpus)
            .expect("the tiny stress corpus is valid");

        assert_eq!(report.scenario_count, 5);
        assert_eq!(report.exact_feasible_scenarios, 3);
        assert_eq!(report.exact_no_schedule_scenarios, 2);
        assert_eq!(report.rule_summaries.len(), 4);
        assert!(report.rule_summaries.iter().all(|summary| {
            summary.plans_found_on_feasible_scenarios == 3
                && summary.feasible_scenarios_missed == 0
                && summary.total_tardiness_ticks.p95.is_some()
        }));
        assert_eq!(
            report.scenario_results[4].exact_outcome,
            ScenarioExactOutcome::NoFeasibleSchedule
        );
        assert!(report.scenario_results[4]
            .dispatch_results
            .iter()
            .all(|result| !result.plan_found));
        assert!(report.interpretation_boundary.contains("do not estimate"));
    }

    #[test]
    fn stress_scenario_order_does_not_change_summary() {
        let first = scenario(
            "slow",
            vec![ProcessingDelay {
                job_id: "ceremonial-repair".to_owned(),
                additional_ticks: 2,
            }],
            vec![],
        );
        let second = scenario("nominal", vec![], vec![]);
        let corpus_a = StressCorpus {
            id: "ordering-test".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![first.clone(), second.clone()],
        };
        let corpus_b = StressCorpus {
            id: "ordering-test".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![second, first],
        };
        let report_a = evaluate_stress_corpus(&fixture_problem(), &corpus_a).unwrap();
        let report_b = evaluate_stress_corpus(&fixture_problem(), &corpus_b).unwrap();

        assert_eq!(report_a.rule_summaries, report_b.rule_summaries);
        let normalize = |report: &StressReport| {
            let mut pairs: Vec<(String, ScenarioExactOutcome)> = report
                .scenario_results
                .iter()
                .map(|item| (item.scenario_id.clone(), item.exact_outcome.clone()))
                .collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            pairs
        };
        assert_eq!(normalize(&report_a), normalize(&report_b));
    }

    #[test]
    fn duplicate_or_unknown_shock_targets_fail_closed() {
        let duplicate = StressCorpus {
            id: "invalid-duplicate".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![scenario(
                "duplicate-job",
                vec![
                    ProcessingDelay {
                        job_id: "ceremonial-repair".to_owned(),
                        additional_ticks: 1,
                    },
                    ProcessingDelay {
                        job_id: "ceremonial-repair".to_owned(),
                        additional_ticks: 2,
                    },
                ],
                vec![],
            )],
        };
        assert!(matches!(
            evaluate_stress_corpus(&fixture_problem(), &duplicate),
            Err(StressHarnessError::DuplicateJobDelay { .. })
        ));

        let unknown = StressCorpus {
            id: "invalid-unknown".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![scenario(
                "unknown-job",
                vec![ProcessingDelay {
                    job_id: "invented-job".to_owned(),
                    additional_ticks: 1,
                }],
                vec![],
            )],
        };
        assert!(matches!(
            evaluate_stress_corpus(&fixture_problem(), &unknown),
            Err(StressHarnessError::UnknownJobInDelay { .. })
        ));
    }

    #[test]
    fn zero_and_overflowing_processing_shocks_are_rejected() {
        let zero = StressCorpus {
            id: "invalid-zero".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![scenario(
                "zero-shock",
                vec![ProcessingDelay {
                    job_id: "ceremonial-repair".to_owned(),
                    additional_ticks: 0,
                }],
                vec![],
            )],
        };
        assert!(matches!(
            evaluate_stress_corpus(&fixture_problem(), &zero),
            Err(StressHarnessError::ZeroProcessingDelay { .. })
        ));

        let mut problem = fixture_problem();
        problem.jobs[0].processing_ticks = u64::MAX;
        let overflow = StressCorpus {
            id: "invalid-overflow".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![scenario(
                "overflow-shock",
                vec![ProcessingDelay {
                    job_id: "ceremonial-repair".to_owned(),
                    additional_ticks: 1,
                }],
                vec![],
            )],
        };
        assert!(matches!(
            evaluate_stress_corpus(&problem, &overflow),
            Err(StressHarnessError::ProcessingDelayOverflow { .. })
        ));
    }

    #[test]
    fn stress_reports_do_not_label_synthetic_scenarios_as_probabilities() {
        let corpus = StressCorpus {
            id: "statement-boundary".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![scenario("nominal", vec![], vec![])],
        };
        let report = evaluate_stress_corpus(&fixture_problem(), &corpus).unwrap();
        assert_eq!(report.interpretation_boundary, STRESS_SCOPE_NOTE);
        assert!(!report.interpretation_boundary.contains("probability of failure"));
        assert_eq!(report.scenario_count, 1);
    }

    #[test]
    fn material_release_delay_overflow_is_rejected() {
        let mut problem = fixture_problem();
        problem.jobs[0].release_tick = u64::MAX;
        let corpus = StressCorpus {
            id: "release-overflow".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![StressScenario {
                id: "supplier-delay".to_owned(),
                fixture_ref: "fixture:stress:supplier-delay".to_owned(),
                processing_delays: vec![],
                release_delays: vec![JobReleaseDelay {
                    job_id: "ceremonial-repair".to_owned(),
                    additional_ticks: 1,
                }],
                utility_budget_shock: None,
                machine_availability_delays: vec![],
            }],
        };
        assert!(matches!(
            evaluate_stress_corpus(&problem, &corpus),
            Err(StressHarnessError::ReleaseDelayOverflow { .. })
        ));
    }


    #[test]
    fn exact_feasibility_is_not_confused_with_greedy_dispatch_failure() {
        let mut problem = fixture_problem();
        problem.jobs[0].id = "long-early-soft-due".to_owned();
        problem.jobs[0].processing_ticks = 5;
        problem.jobs[0].due_tick = 2;
        problem.jobs[0].hard_finish_by_tick = Some(10);
        problem.jobs[0].functional_unit_id = "ceremonial-service-event-v0".to_owned();

        let mut short_job = problem.jobs[0].clone();
        short_job.id = "short-hard-window".to_owned();
        short_job.processing_ticks = 2;
        short_job.due_tick = 5;
        short_job.hard_finish_by_tick = Some(6);
        short_job.functional_unit_id = "alteration-event-v0".to_owned();
        problem.jobs.push(short_job);

        let corpus = StressCorpus {
            id: "greedy-deadline-trap".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![scenario("nominal", vec![], vec![])],
        };
        let report = evaluate_stress_corpus(&problem, &corpus)
            .expect("valid but challenging scheduling case is evaluable");

        assert_eq!(report.exact_feasible_scenarios, 1);
        assert_eq!(report.exact_no_schedule_scenarios, 0);
        assert!(report.scenario_results[0]
            .dispatch_results
            .iter()
            .filter(|result| result.rule != DispatchRule::LeastSlack)
            .all(|result| result.missed_feasible_schedule && !result.plan_found));
        let least_slack = report.scenario_results[0]
            .dispatch_results
            .iter()
            .find(|result| result.rule == DispatchRule::LeastSlack)
            .expect("least-slack rule is evaluated");
        assert!(least_slack.plan_found);
        assert!(!least_slack.missed_feasible_schedule);
        let least_slack_summary = report
            .rule_summaries
            .iter()
            .find(|summary| summary.rule == DispatchRule::LeastSlack)
            .expect("least-slack summary is present");
        assert_eq!(least_slack_summary.plans_found_on_feasible_scenarios, 1);
        assert_eq!(least_slack_summary.feasible_scenarios_missed, 0);
        assert!(report.rule_summaries.iter().filter(|summary| {
            summary.rule != DispatchRule::LeastSlack
        }).all(|summary| {
            summary.plans_found_on_feasible_scenarios == 0
                && summary.feasible_scenarios_missed == 1
        }));
    }


    #[test]
    fn utility_budget_shock_cannot_loosen_the_declared_cap() {
        let corpus = StressCorpus {
            id: "relaxed-budget-rejected".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![StressScenario {
                id: "more-energy-than-baseline".to_owned(),
                fixture_ref: "fixture:stress:invalid-relaxed-budget".to_owned(),
                processing_delays: vec![],
                release_delays: vec![],
                utility_budget_shock: Some(UtilityBudgetShock {
                    energy_budget_units: Some(5),
                    water_budget_units: None,
                }),
                machine_availability_delays: vec![],
            }],
        };
        assert!(matches!(
            evaluate_stress_corpus(&fixture_problem(), &corpus),
            Err(StressHarnessError::LoosenedEnergyBudget { .. })
        ));
    }

    #[test]
    fn empty_utility_budget_shock_is_rejected() {
        let corpus = StressCorpus {
            id: "empty-budget-shock".to_owned(),
            version: "1".to_owned(),
            scenarios: vec![StressScenario {
                id: "empty-budget".to_owned(),
                fixture_ref: "fixture:stress:empty-budget".to_owned(),
                processing_delays: vec![],
                release_delays: vec![],
                utility_budget_shock: Some(UtilityBudgetShock {
                    energy_budget_units: None,
                    water_budget_units: None,
                }),
                machine_availability_delays: vec![],
            }],
        };
        assert!(matches!(
            evaluate_stress_corpus(&fixture_problem(), &corpus),
            Err(StressHarnessError::EmptyUtilityBudgetShock { .. })
        ));
    }

}
