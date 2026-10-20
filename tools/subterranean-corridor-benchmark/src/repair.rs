//! Deterministic constrained-crew repair scheduling for research scenarios.
//!
//! This is a bounded greedy scheduling heuristic, not an optimal project scheduler,
//! damage model, repair-duration predictor, or permission to reopen infrastructure.
//! All task durations, precedences, crews and re-entry authorization are supplied
//! inputs. A service-ready time is emitted only when required work is scheduled and
//! the explicit external re-entry authorization is Satisfied.

use std::collections::{BTreeMap, BTreeSet};

use super::GateState;

const TIME_EPS: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairTaskKind {
    Assessment,
    Clearance,
    Repair,
    UtilityRestoration,
    PostEventInspection,
    Commissioning,
}

impl RepairTaskKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Assessment => "assessment",
            Self::Clearance => "clearance",
            Self::Repair => "repair",
            Self::UtilityRestoration => "utility_restoration",
            Self::PostEventInspection => "post_event_inspection",
            Self::Commissioning => "commissioning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewPool {
    pub skill_id: String,
    pub available_crews: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewRequirement {
    pub skill_id: String,
    pub crews_required: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairTask {
    pub task_id: String,
    pub kind: RepairTaskKind,
    /// Fixed synthetic/declared task duration, in hours. No uncertainty distribution
    /// is implied; probabilistic repair durations require a separately declared model.
    pub duration_hours: f64,
    /// Every predecessor must finish before this task can start.
    pub predecessors: Vec<String>,
    /// Crew resources must be available for the full non-preemptive task duration.
    pub crew_requirements: Vec<CrewRequirement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairScheduleScenario {
    pub scenario_id: String,
    pub evidence_class: super::EvidenceClass,
    /// Provenance references supplied by the caller, not authenticated here.
    pub source_refs: Vec<String>,
    pub crews: Vec<CrewPool>,
    pub tasks: Vec<RepairTask>,
    /// Tasks which must finish before the declared service level can be available.
    /// This may be a subset of all repair work; post-recovery maintenance can continue.
    pub service_enabling_task_ids: Vec<String>,
    pub post_event_inspection_task_id: String,
    /// External authority/owner authorization. Finishing the inspection task alone
    /// never substitutes for this explicit state.
    pub reentry_authorization: GateState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairScheduleDisposition {
    InvalidInput,
    Ineligible,
    NeedsEvidence,
    ScheduledSynthetic,
    ScheduledDeclaredEvidence,
}

impl RepairScheduleDisposition {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::Ineligible => "INELIGIBLE",
            Self::NeedsEvidence => "NEEDS_EVIDENCE",
            Self::ScheduledSynthetic => "SCHEDULED_SYNTHETIC_ONLY",
            Self::ScheduledDeclaredEvidence => "SCHEDULED_DECLARED_EVIDENCE",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledTask {
    pub task_id: String,
    pub kind: RepairTaskKind,
    pub start_hour: f64,
    pub finish_hour: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairScheduleReport {
    pub scenario_id: String,
    pub evidence_class: super::EvidenceClass,
    pub task_schedule: Vec<ScheduledTask>,
    pub total_work_makespan_hours: f64,
    /// None until a valid inspection task is complete and external re-entry
    /// authorization is explicitly Satisfied.
    pub authorized_service_ready_hours: Option<f64>,
    /// Sum of duration × assigned crews by skill, in crew-hours.
    pub crew_hours_by_skill: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepairScheduleEvaluation {
    pub scenario_id: String,
    pub disposition: RepairScheduleDisposition,
    pub reasons: Vec<String>,
    /// A work plan may be useful for planning even while authorization is unresolved;
    /// in that case this report necessarily has no authorized service-ready time.
    pub report: Option<RepairScheduleReport>,
}

fn invalid(scenario_id: &str, reason: impl Into<String>) -> RepairScheduleEvaluation {
    RepairScheduleEvaluation {
        scenario_id: scenario_id.to_owned(),
        disposition: RepairScheduleDisposition::InvalidInput,
        reasons: vec![reason.into()],
        report: None,
    }
}

fn validate(scenario: &RepairScheduleScenario) -> Result<(), String> {
    if scenario.scenario_id.trim().is_empty() {
        return Err("EMPTY_SCENARIO_ID".to_owned());
    }
    if scenario.tasks.is_empty() {
        return Err("AT_LEAST_ONE_REPAIR_TASK_REQUIRED".to_owned());
    }
    if scenario.crews.is_empty() {
        return Err("AT_LEAST_ONE_CREW_POOL_REQUIRED".to_owned());
    }
    if scenario.evidence_class != super::EvidenceClass::Synthetic && scenario.source_refs.is_empty()
    {
        return Err("DECLARED_EVIDENCE_REQUIRES_SOURCE_REFERENCES".to_owned());
    }

    let mut refs = BTreeSet::new();
    for source_ref in &scenario.source_refs {
        if source_ref.trim().is_empty() || !refs.insert(source_ref) {
            return Err("EMPTY_OR_DUPLICATE_SOURCE_REFERENCE".to_owned());
        }
    }

    let mut crews_by_skill = BTreeMap::new();
    for crew in &scenario.crews {
        if crew.skill_id.trim().is_empty() || crew.available_crews == 0 {
            return Err("CREW_SKILL_MUST_BE_NONEMPTY_AND_CREW_COUNT_POSITIVE".to_owned());
        }
        if crews_by_skill
            .insert(crew.skill_id.as_str(), crew.available_crews)
            .is_some()
        {
            return Err("DUPLICATE_CREW_SKILL_POOL".to_owned());
        }
    }

    let task_ids: BTreeSet<&str> = scenario
        .tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect();
    if task_ids.len() != scenario.tasks.len() || task_ids.iter().any(|id| id.trim().is_empty()) {
        return Err("EMPTY_OR_DUPLICATE_TASK_ID".to_owned());
    }

    for task in &scenario.tasks {
        if !task.duration_hours.is_finite() || task.duration_hours <= 0.0 {
            return Err("TASK_DURATION_MUST_BE_FINITE_AND_POSITIVE".to_owned());
        }
        if task.crew_requirements.is_empty() {
            return Err("TASK_MUST_DECLARE_CREW_REQUIREMENTS".to_owned());
        }
        let mut requirement_skills = BTreeSet::new();
        for req in &task.crew_requirements {
            if req.skill_id.trim().is_empty()
                || req.crews_required == 0
                || !requirement_skills.insert(req.skill_id.as_str())
            {
                return Err("INVALID_OR_DUPLICATE_TASK_CREW_REQUIREMENT".to_owned());
            }
            match crews_by_skill.get(req.skill_id.as_str()) {
                None => return Err(format!("UNKNOWN_CREW_SKILL:{}", req.skill_id)),
                Some(available) if req.crews_required > *available => {
                    return Err(format!("TASK_EXCEEDS_CREW_POOL:{}", req.skill_id));
                }
                Some(_) => {}
            }
        }
        let mut predecessors = BTreeSet::new();
        for predecessor in &task.predecessors {
            if predecessor == &task.task_id {
                return Err(format!("TASK_SELF_DEPENDENCY:{}", task.task_id));
            }
            if !task_ids.contains(predecessor.as_str()) {
                return Err(format!("UNKNOWN_PREDECESSOR:{predecessor}"));
            }
            if !predecessors.insert(predecessor) {
                return Err(format!("DUPLICATE_PREDECESSOR:{}", task.task_id));
            }
        }
    }

    if scenario.service_enabling_task_ids.is_empty() {
        return Err("SERVICE_ENABLING_TASKS_REQUIRED".to_owned());
    }
    let mut service_ids = BTreeSet::new();
    for id in &scenario.service_enabling_task_ids {
        if !task_ids.contains(id.as_str()) {
            return Err(format!("UNKNOWN_SERVICE_ENABLING_TASK:{id}"));
        }
        if !service_ids.insert(id) {
            return Err("DUPLICATE_SERVICE_ENABLING_TASK".to_owned());
        }
    }
    if !task_ids.contains(scenario.post_event_inspection_task_id.as_str()) {
        return Err("POST_EVENT_INSPECTION_TASK_MISSING".to_owned());
    }
    let inspection = scenario
        .tasks
        .iter()
        .find(|task| task.task_id == scenario.post_event_inspection_task_id)
        .expect("validated inspection task exists");
    if inspection.kind != RepairTaskKind::PostEventInspection {
        return Err("REENTRY_GATE_MUST_REFERENCE_POST_EVENT_INSPECTION_TASK".to_owned());
    }
    if !service_ids.contains(&scenario.post_event_inspection_task_id) {
        return Err("INSPECTION_MUST_BE_SERVICE_ENABLING_TASK".to_owned());
    }
    Ok(())
}

fn requirements_fit(requirements: &[CrewRequirement], available: &BTreeMap<String, u32>) -> bool {
    requirements.iter().all(|req| {
        available
            .get(&req.skill_id)
            .is_some_and(|count| *count >= req.crews_required)
    })
}

fn reserve(
    requirements: &[CrewRequirement],
    available: &mut BTreeMap<String, u32>,
    subtract: bool,
) {
    for req in requirements {
        let count = available
            .get_mut(&req.skill_id)
            .expect("task skill validated");
        if subtract {
            *count -= req.crews_required;
        } else {
            *count += req.crews_required;
        }
    }
}

#[derive(Debug)]
struct RunningTask {
    task_id: String,
    finish_hour: f64,
    requirements: Vec<CrewRequirement>,
}

/// Schedule all declared tasks with a deterministic, non-preemptive greedy list
/// scheduler. At each event time, ready tasks are considered by task ID and started
/// when all required skill pools can be reserved. This is reproducible but not an
/// optimal scheduling solver; alternative priority policies may yield shorter plans.
pub fn schedule_repairs(scenario: &RepairScheduleScenario) -> RepairScheduleEvaluation {
    if let Err(reason) = validate(scenario) {
        return invalid(&scenario.scenario_id, reason);
    }

    let task_by_id: BTreeMap<&str, &RepairTask> = scenario
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect();
    let mut available: BTreeMap<String, u32> = scenario
        .crews
        .iter()
        .map(|crew| (crew.skill_id.clone(), crew.available_crews))
        .collect();
    let mut pending: BTreeSet<String> = scenario
        .tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect();
    let mut completed = BTreeSet::new();
    let mut running: Vec<RunningTask> = Vec::new();
    let mut schedule: BTreeMap<String, ScheduledTask> = BTreeMap::new();
    let mut now = 0.0;

    while !pending.is_empty() || !running.is_empty() {
        let mut next_running = Vec::new();
        for task in running.drain(..) {
            if task.finish_hour <= now + TIME_EPS {
                reserve(&task.requirements, &mut available, false);
                completed.insert(task.task_id);
            } else {
                next_running.push(task);
            }
        }
        running = next_running;

        let candidate_ids: Vec<String> = pending.iter().cloned().collect();
        let mut started_any = false;
        for task_id in candidate_ids {
            let task = task_by_id[task_id.as_str()];
            if !task
                .predecessors
                .iter()
                .all(|predecessor| completed.contains(predecessor))
            {
                continue;
            }
            if !requirements_fit(&task.crew_requirements, &available) {
                continue;
            }

            reserve(&task.crew_requirements, &mut available, true);
            let finish = now + task.duration_hours;
            if !finish.is_finite() {
                return invalid(&scenario.scenario_id, "ARITHMETIC_OVERFLOW:TASK_FINISH");
            }
            schedule.insert(
                task_id.clone(),
                ScheduledTask {
                    task_id: task_id.clone(),
                    kind: task.kind,
                    start_hour: now,
                    finish_hour: finish,
                },
            );
            running.push(RunningTask {
                task_id: task_id.clone(),
                finish_hour: finish,
                requirements: task.crew_requirements.clone(),
            });
            pending.remove(&task_id);
            started_any = true;
        }

        if pending.is_empty() && running.is_empty() {
            break;
        }
        if started_any {
            continue;
        }

        let Some(next_time) = running
            .iter()
            .map(|task| task.finish_hour)
            .min_by(f64::total_cmp)
        else {
            return invalid(
                &scenario.scenario_id,
                "PRECEDENCE_CYCLE_OR_UNSCHEDULABLE_TASKS",
            );
        };
        if !next_time.is_finite() || next_time <= now {
            return invalid(&scenario.scenario_id, "INVALID_SCHEDULER_EVENT_TIME");
        }
        now = next_time;
    }

    let mut task_schedule: Vec<ScheduledTask> = schedule.values().cloned().collect();
    task_schedule.sort_by(|a, b| {
        a.start_hour
            .total_cmp(&b.start_hour)
            .then_with(|| a.task_id.cmp(&b.task_id))
    });
    let total_work_makespan = task_schedule
        .iter()
        .map(|task| task.finish_hour)
        .max_by(f64::total_cmp)
        .unwrap_or(0.0);

    let mut crew_hours_by_skill: BTreeMap<String, f64> = BTreeMap::new();
    for task in &scenario.tasks {
        let scheduled = &schedule[&task.task_id];
        for req in &task.crew_requirements {
            let crew_hours =
                (scheduled.finish_hour - scheduled.start_hour) * req.crews_required as f64;
            let total = crew_hours_by_skill
                .entry(req.skill_id.clone())
                .or_insert(0.0);
            *total += crew_hours;
            if !total.is_finite() {
                return invalid(&scenario.scenario_id, "ARITHMETIC_OVERFLOW:CREW_HOURS");
            }
        }
    }

    let mut reasons = Vec::new();
    let authorized_service_ready_hours = match scenario.reentry_authorization {
        GateState::Satisfied => Some(
            scenario
                .service_enabling_task_ids
                .iter()
                .map(|task_id| schedule[task_id].finish_hour)
                .max_by(f64::total_cmp)
                .unwrap_or(0.0),
        ),
        GateState::Failed => {
            reasons.push("GATE_FAILED:reentry_authorization".to_owned());
            None
        }
        GateState::Unknown | GateState::NotApplicable => {
            reasons.push("NEEDS_EVIDENCE:reentry_authorization".to_owned());
            None
        }
    };

    let disposition = match scenario.reentry_authorization {
        GateState::Failed => RepairScheduleDisposition::Ineligible,
        GateState::Unknown | GateState::NotApplicable => RepairScheduleDisposition::NeedsEvidence,
        GateState::Satisfied => match scenario.evidence_class {
            super::EvidenceClass::Synthetic => RepairScheduleDisposition::ScheduledSynthetic,
            super::EvidenceClass::Sourced
            | super::EvidenceClass::Measured
            | super::EvidenceClass::IndependentlyReviewed => {
                RepairScheduleDisposition::ScheduledDeclaredEvidence
            }
        },
    };

    RepairScheduleEvaluation {
        scenario_id: scenario.scenario_id.clone(),
        disposition,
        reasons,
        report: Some(RepairScheduleReport {
            scenario_id: scenario.scenario_id.clone(),
            evidence_class: scenario.evidence_class,
            task_schedule,
            total_work_makespan_hours: total_work_makespan,
            authorized_service_ready_hours,
            crew_hours_by_skill,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EvidenceClass;

    fn base_scenario() -> RepairScheduleScenario {
        RepairScheduleScenario {
            scenario_id: "repair-plan-fixture".to_owned(),
            evidence_class: EvidenceClass::Synthetic,
            source_refs: Vec::new(),
            crews: vec![
                CrewPool {
                    skill_id: "engineer".to_owned(),
                    available_crews: 1,
                },
                CrewPool {
                    skill_id: "civil".to_owned(),
                    available_crews: 1,
                },
                CrewPool {
                    skill_id: "electrical".to_owned(),
                    available_crews: 1,
                },
            ],
            tasks: vec![
                RepairTask {
                    task_id: "assessment".to_owned(),
                    kind: RepairTaskKind::Assessment,
                    duration_hours: 2.0,
                    predecessors: Vec::new(),
                    crew_requirements: vec![CrewRequirement {
                        skill_id: "engineer".to_owned(),
                        crews_required: 1,
                    }],
                },
                RepairTask {
                    task_id: "repair-civil".to_owned(),
                    kind: RepairTaskKind::Repair,
                    duration_hours: 4.0,
                    predecessors: vec!["assessment".to_owned()],
                    crew_requirements: vec![CrewRequirement {
                        skill_id: "civil".to_owned(),
                        crews_required: 1,
                    }],
                },
                RepairTask {
                    task_id: "restore-power".to_owned(),
                    kind: RepairTaskKind::UtilityRestoration,
                    duration_hours: 6.0,
                    predecessors: vec!["assessment".to_owned()],
                    crew_requirements: vec![CrewRequirement {
                        skill_id: "electrical".to_owned(),
                        crews_required: 1,
                    }],
                },
                RepairTask {
                    task_id: "post-event-inspection".to_owned(),
                    kind: RepairTaskKind::PostEventInspection,
                    duration_hours: 1.0,
                    predecessors: vec!["repair-civil".to_owned(), "restore-power".to_owned()],
                    crew_requirements: vec![CrewRequirement {
                        skill_id: "engineer".to_owned(),
                        crews_required: 1,
                    }],
                },
            ],
            service_enabling_task_ids: vec![
                "assessment".to_owned(),
                "repair-civil".to_owned(),
                "restore-power".to_owned(),
                "post-event-inspection".to_owned(),
            ],
            post_event_inspection_task_id: "post-event-inspection".to_owned(),
            reentry_authorization: GateState::Satisfied,
        }
    }

    #[test]
    fn independent_repairs_overlap_but_inspection_waits_for_both() {
        let evaluation = schedule_repairs(&base_scenario());
        assert_eq!(
            evaluation.disposition,
            RepairScheduleDisposition::ScheduledSynthetic
        );
        let report = evaluation.report.unwrap();
        let lookup: BTreeMap<_, _> = report
            .task_schedule
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();
        assert_eq!(lookup["assessment"].start_hour, 0.0);
        assert_eq!(lookup["assessment"].finish_hour, 2.0);
        assert_eq!(lookup["repair-civil"].start_hour, 2.0);
        assert_eq!(lookup["restore-power"].start_hour, 2.0);
        assert_eq!(lookup["repair-civil"].finish_hour, 6.0);
        assert_eq!(lookup["restore-power"].finish_hour, 8.0);
        assert_eq!(lookup["post-event-inspection"].start_hour, 8.0);
        assert_eq!(lookup["post-event-inspection"].finish_hour, 9.0);
        assert_eq!(report.authorized_service_ready_hours, Some(9.0));
        assert_eq!(report.total_work_makespan_hours, 9.0);
    }

    #[test]
    fn shared_crew_bottleneck_serializes_otherwise_independent_tasks() {
        let mut scenario = base_scenario();
        scenario.crews = vec![
            CrewPool {
                skill_id: "engineer".to_owned(),
                available_crews: 1,
            },
            CrewPool {
                skill_id: "maintenance".to_owned(),
                available_crews: 1,
            },
        ];
        scenario.tasks = vec![
            RepairTask {
                task_id: "repair-a".to_owned(),
                kind: RepairTaskKind::Repair,
                duration_hours: 3.0,
                predecessors: Vec::new(),
                crew_requirements: vec![CrewRequirement {
                    skill_id: "maintenance".to_owned(),
                    crews_required: 1,
                }],
            },
            RepairTask {
                task_id: "repair-b".to_owned(),
                kind: RepairTaskKind::Repair,
                duration_hours: 4.0,
                predecessors: Vec::new(),
                crew_requirements: vec![CrewRequirement {
                    skill_id: "maintenance".to_owned(),
                    crews_required: 1,
                }],
            },
            RepairTask {
                task_id: "post-event-inspection".to_owned(),
                kind: RepairTaskKind::PostEventInspection,
                duration_hours: 1.0,
                predecessors: vec!["repair-a".to_owned(), "repair-b".to_owned()],
                crew_requirements: vec![CrewRequirement {
                    skill_id: "engineer".to_owned(),
                    crews_required: 1,
                }],
            },
        ];
        scenario.service_enabling_task_ids = vec![
            "repair-a".to_owned(),
            "repair-b".to_owned(),
            "post-event-inspection".to_owned(),
        ];
        let evaluation = schedule_repairs(&scenario);
        let report = evaluation.report.unwrap();
        let lookup: BTreeMap<_, _> = report
            .task_schedule
            .iter()
            .map(|task| (task.task_id.as_str(), task))
            .collect();
        assert_eq!(lookup["repair-a"].start_hour, 0.0);
        assert_eq!(lookup["repair-b"].start_hour, 3.0);
        assert_eq!(lookup["post-event-inspection"].start_hour, 7.0);
        assert_eq!(report.authorized_service_ready_hours, Some(8.0));
        assert_eq!(report.crew_hours_by_skill["maintenance"], 7.0);
    }

    #[test]
    fn unknown_reentry_authorization_never_reports_service_ready_time() {
        let mut scenario = base_scenario();
        scenario.reentry_authorization = GateState::Unknown;
        let evaluation = schedule_repairs(&scenario);
        assert_eq!(
            evaluation.disposition,
            RepairScheduleDisposition::NeedsEvidence
        );
        assert!(
            evaluation
                .reasons
                .contains(&"NEEDS_EVIDENCE:reentry_authorization".to_owned())
        );
        assert_eq!(
            evaluation.report.unwrap().authorized_service_ready_hours,
            None
        );
    }

    #[test]
    fn failed_reentry_authorization_never_reports_service_ready_time() {
        let mut scenario = base_scenario();
        scenario.reentry_authorization = GateState::Failed;
        let evaluation = schedule_repairs(&scenario);
        assert_eq!(
            evaluation.disposition,
            RepairScheduleDisposition::Ineligible
        );
        assert_eq!(
            evaluation.report.unwrap().authorized_service_ready_hours,
            None
        );
    }

    #[test]
    fn cyclic_precedence_graph_is_rejected() {
        let mut scenario = base_scenario();
        scenario.tasks[0].predecessors = vec!["post-event-inspection".to_owned()];
        let evaluation = schedule_repairs(&scenario);
        assert_eq!(
            evaluation.disposition,
            RepairScheduleDisposition::InvalidInput
        );
        assert!(evaluation.report.is_none());
    }

    #[test]
    fn task_that_exceeds_a_skill_pool_is_rejected() {
        let mut scenario = base_scenario();
        scenario.tasks[0].crew_requirements[0].crews_required = 2;
        assert_eq!(
            schedule_repairs(&scenario).disposition,
            RepairScheduleDisposition::InvalidInput
        );
    }

    #[test]
    fn missing_predecessor_and_missing_inspection_are_rejected() {
        let mut scenario = base_scenario();
        scenario.tasks[1].predecessors = vec!["unknown-task".to_owned()];
        assert_eq!(
            schedule_repairs(&scenario).disposition,
            RepairScheduleDisposition::InvalidInput
        );

        let mut missing_inspection = base_scenario();
        missing_inspection.post_event_inspection_task_id = "no-inspection".to_owned();
        assert_eq!(
            schedule_repairs(&missing_inspection).disposition,
            RepairScheduleDisposition::InvalidInput
        );
    }

    #[test]
    fn schedule_is_deterministic_under_input_order_permutation() {
        let original = schedule_repairs(&base_scenario()).report.unwrap();
        let mut permuted = base_scenario();
        permuted.tasks.reverse();
        permuted.crews.reverse();
        let changed = schedule_repairs(&permuted).report.unwrap();
        assert_eq!(original, changed);
    }

    #[test]
    fn declared_evidence_requires_source_reference() {
        let mut scenario = base_scenario();
        scenario.evidence_class = EvidenceClass::Measured;
        assert_eq!(
            schedule_repairs(&scenario).disposition,
            RepairScheduleDisposition::InvalidInput
        );
        scenario.source_refs.push("repair-log-2026-001".to_owned());
        assert_eq!(
            schedule_repairs(&scenario).disposition,
            RepairScheduleDisposition::ScheduledDeclaredEvidence
        );
    }
}
