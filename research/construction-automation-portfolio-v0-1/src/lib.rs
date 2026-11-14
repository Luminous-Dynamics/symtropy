//! Synthetic execution-mode and selective-robotization portfolio benchmark.
//!
//! This crate evaluates declared, pre-scheduled human / assisted / robot / offsite
//! alternatives for identical functional work units. It reuses the existing resource
//! reservation and conservative space-time occupancy owners. It is not a general
//! scheduler, capability authority, safety controller or acceptance ledger.
//!
//! A feasible portfolio is a planning proposal only. Independent physical acceptance,
//! defects, rework and final whole-floor actual cost remain with the accepted-floor ledger.

use std::collections::{BTreeMap, BTreeSet};

use construction_robot_interop_core::resources::{
    ReservationBook, ReservationError, ReservationRequest,
};
use construction_spacetime_occupancy_core::{SpatialError, SpatialOccupancy, SpatialOccupancyBook};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionMode {
    Human,
    Assisted,
    Robot,
    Offsite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EligibilityStatus {
    Eligible,
    Ineligible,
    Unknown,
    Stale,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModeEligibility {
    pub status: EligibilityStatus,
    /// Opaque capability evidence reference; authenticated by its external owner.
    pub capability_profile_ref: Option<String>,
    /// Opaque machine/process qualification reference; not authenticated here.
    pub qualification_ref: Option<String>,
    /// Opaque accountable authority-scope reference; not authenticated here.
    pub authority_scope_ref: Option<String>,
    pub reason: Option<String>,
}

impl ModeEligibility {
    pub fn eligible(
        capability_profile_ref: impl Into<String>,
        qualification_ref: impl Into<String>,
        authority_scope_ref: impl Into<String>,
    ) -> Self {
        Self {
            status: EligibilityStatus::Eligible,
            capability_profile_ref: Some(capability_profile_ref.into()),
            qualification_ref: Some(qualification_ref.into()),
            authority_scope_ref: Some(authority_scope_ref.into()),
            reason: None,
        }
    }

    pub fn excluded(status: EligibilityStatus, reason: impl Into<String>) -> Self {
        Self {
            status,
            capability_profile_ref: None,
            qualification_ref: None,
            authority_scope_ref: None,
            reason: Some(reason.into()),
        }
    }

    fn validate_eligible(&self) -> Result<(), PortfolioError> {
        if self.status != EligibilityStatus::Eligible {
            return Ok(());
        }
        require_reference(
            "capability_profile_ref",
            self.capability_profile_ref.as_deref(),
        )?;
        require_reference("qualification_ref", self.qualification_ref.as_deref())?;
        require_reference("authority_scope_ref", self.authority_scope_ref.as_deref())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostBreakdown {
    pub labour_minor_units: u64,
    pub equipment_minor_units: u64,
    pub energy_minor_units: u64,
    pub materials_minor_units: u64,
    pub logistics_minor_units: u64,
    pub inspection_minor_units: u64,
    pub setup_minor_units: u64,
    pub maintenance_minor_units: u64,
    pub rework_allowance_minor_units: u64,
    pub allocated_shared_minor_units: u64,
}

impl CostBreakdown {
    pub fn total_minor_units(&self) -> Result<u64, PortfolioError> {
        [
            self.labour_minor_units,
            self.equipment_minor_units,
            self.energy_minor_units,
            self.materials_minor_units,
            self.logistics_minor_units,
            self.inspection_minor_units,
            self.setup_minor_units,
            self.maintenance_minor_units,
            self.rework_allowance_minor_units,
            self.allocated_shared_minor_units,
        ]
        .into_iter()
        .try_fold(0_u64, |total, item| {
            total.checked_add(item).ok_or(PortfolioError::CostOverflow)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredWorkUnit {
    pub work_unit_id: String,
    pub functional_unit_class: String,
    pub acceptance_contract_ref: String,
    /// Work units must be listed in topological order; every predecessor must appear earlier.
    pub predecessor_work_unit_ids: Vec<String>,
    pub mode_options: Vec<ModeOption>,
}

impl RequiredWorkUnit {
    fn validate(&self, earlier_ids: &BTreeSet<&str>) -> Result<(), PortfolioError> {
        require_non_empty("work_unit_id", &self.work_unit_id)?;
        require_non_empty("functional_unit_class", &self.functional_unit_class)?;
        require_non_empty("acceptance_contract_ref", &self.acceptance_contract_ref)?;
        for predecessor in &self.predecessor_work_unit_ids {
            require_non_empty("predecessor_work_unit_id", predecessor)?;
            if !earlier_ids.contains(predecessor.as_str()) {
                return Err(PortfolioError::NonTopologicalDependency {
                    work_unit_id: self.work_unit_id.clone(),
                    predecessor_work_unit_id: predecessor.clone(),
                });
            }
        }
        let mut seen_options = BTreeSet::new();
        for option in &self.mode_options {
            option.validate_for(self)?;
            if !seen_options.insert(option.option_id.as_str()) {
                return Err(PortfolioError::DuplicateOption(option.option_id.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModeOption {
    pub option_id: String,
    pub work_unit_id: String,
    pub mode: ExecutionMode,
    pub eligibility: ModeEligibility,
    /// Proposed schedule window in the same site clock domain as the resource book.
    /// These are schedule inputs, not measured construction times.
    pub scheduled_start_ms: u64,
    pub scheduled_end_ms: u64,
    pub acceptance_contract_ref: String,
    pub crew_minutes_by_role: BTreeMap<String, u64>,
    pub skilled_crew_minutes: u64,
    pub energy_wh: u64,
    pub cost: CostBreakdown,
    /// Existing resource-calendar requests (robot, hoist, crew skill, inspection, etc.).
    pub reservation_requests: Vec<ReservationRequest>,
    /// Optional declared workface/access envelope. If present, its embedded reservation
    /// must be one of reservation_requests and is admitted through reserve_and_insert.
    pub spatial_occupancy: Option<SpatialOccupancy>,
}

impl ModeOption {
    fn validate_for(&self, unit: &RequiredWorkUnit) -> Result<(), PortfolioError> {
        require_non_empty("option_id", &self.option_id)?;
        require_non_empty("option_work_unit_id", &self.work_unit_id)?;
        require_non_empty(
            "option_acceptance_contract_ref",
            &self.acceptance_contract_ref,
        )?;
        if self.work_unit_id != unit.work_unit_id {
            return Err(PortfolioError::OptionWorkUnitMismatch {
                option_id: self.option_id.clone(),
            });
        }
        if self.acceptance_contract_ref != unit.acceptance_contract_ref {
            return Err(PortfolioError::AcceptanceContractMismatch {
                option_id: self.option_id.clone(),
            });
        }
        if self.scheduled_start_ms >= self.scheduled_end_ms {
            return Err(PortfolioError::InvalidScheduleWindow(
                self.option_id.clone(),
            ));
        }
        self.eligibility.validate_eligible()?;

        for (role, _) in &self.crew_minutes_by_role {
            require_non_empty("crew_role", role)?;
        }

        let mut reservation_ids = BTreeSet::new();
        for request in &self.reservation_requests {
            require_non_empty("reservation_id", &request.reservation_id)?;
            if !reservation_ids.insert(request.reservation_id.as_str()) {
                return Err(PortfolioError::DuplicateReservation(
                    request.reservation_id.clone(),
                ));
            }
        }

        if let Some(occupancy) = &self.spatial_occupancy {
            let matching_request = self
                .reservation_requests
                .iter()
                .any(|request| request == &occupancy.reservation);
            if !matching_request {
                return Err(PortfolioError::OccupancyReservationMismatch(
                    self.option_id.clone(),
                ));
            }
            if occupancy.reservation.interval_start_ms < self.scheduled_start_ms
                || occupancy.reservation.interval_end_ms > self.scheduled_end_ms
            {
                return Err(PortfolioError::OccupancyOutsideSchedule(
                    self.option_id.clone(),
                ));
            }
        }
        self.cost.total_minor_units()?;
        Ok(())
    }

    pub fn duration_ms(&self) -> u64 {
        self.scheduled_end_ms - self.scheduled_start_ms
    }

    pub fn total_crew_minutes(&self) -> Result<u64, PortfolioError> {
        self.crew_minutes_by_role
            .values()
            .try_fold(0_u64, |total, value| {
                total
                    .checked_add(*value)
                    .ok_or(PortfolioError::CrewOverflow)
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioProblem {
    pub currency_code: String,
    pub project_start_ms: u64,
    /// Freeze the search budget in the scenario profile; an exhausted search is incomplete.
    pub max_search_states: usize,
    pub work_units: Vec<RequiredWorkUnit>,
    pub initial_reservations: ReservationBook,
    pub initial_occupancies: SpatialOccupancyBook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortfolioPolicy {
    /// Enumerate all eligible mode / schedule options and return the non-dominated frontier.
    ExactPareto,
    /// Human option only. No silent fallback to robotic work.
    HumanOnly,
    /// Prefer a robot option for every work unit where one is eligible; otherwise human.
    RobotizeAllEligible,
    /// Choose the locally shortest eligible option before portfolio-level feasibility checks.
    FastestLocal,
    /// Choose the locally cheapest eligible option before portfolio-level feasibility checks.
    LowestCostLocal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchCompleteness {
    Exhaustive,
    BudgetExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedOption {
    pub option_id: String,
    pub work_unit_id: String,
    pub status: EligibilityStatus,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioChoice {
    pub work_unit_id: String,
    pub option_id: String,
    pub mode: ExecutionMode,
    pub scheduled_start_ms: u64,
    pub scheduled_end_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioMetrics {
    pub total_cost_minor_units: u64,
    pub currency_code: String,
    pub makespan_ms: u64,
    pub total_workflow_crew_minutes: u64,
    pub skilled_crew_minutes: u64,
    pub energy_wh: u64,
    pub human_work_units: usize,
    pub assisted_work_units: usize,
    pub robot_work_units: usize,
    pub offsite_work_units: usize,
    /// Number of required units in the proposal. None of these are claimed accepted.
    pub planned_work_units: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioPlan {
    pub choices: Vec<PortfolioChoice>,
    pub metrics: PortfolioMetrics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortfolioSearchResult {
    pub policy: PortfolioPolicy,
    pub completeness: SearchCompleteness,
    pub evaluated_states: usize,
    /// Feasible leaves encountered. On BudgetExhausted this is only a partial count.
    pub feasible_portfolio_count: usize,
    /// Non-dominated among feasible leaves found. It is a global frontier only when Exhaustive.
    pub pareto_frontier: Vec<PortfolioPlan>,
    pub excluded_options: Vec<ExcludedOption>,
    pub infeasible_declared_portfolio: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortfolioError {
    EmptyField(&'static str),
    DuplicateWorkUnit(String),
    DuplicateOption(String),
    DuplicateReservation(String),
    NonTopologicalDependency {
        work_unit_id: String,
        predecessor_work_unit_id: String,
    },
    OptionWorkUnitMismatch {
        option_id: String,
    },
    AcceptanceContractMismatch {
        option_id: String,
    },
    InvalidScheduleWindow(String),
    OccupancyReservationMismatch(String),
    OccupancyOutsideSchedule(String),
    InvalidCurrency,
    InvalidPortfolioShape,
    CostOverflow,
    CrewOverflow,
    EnergyOverflow,
    Resource(ReservationError),
    Spatial(SpatialError),
}

pub fn solve_portfolio(
    problem: &PortfolioProblem,
    policy: PortfolioPolicy,
) -> Result<PortfolioSearchResult, PortfolioError> {
    validate_problem(problem)?;
    let mut excluded_options = Vec::new();
    let candidate_options = candidate_options_for_policy(problem, policy, &mut excluded_options)?;

    let mut state = SearchState {
        max_states: problem.max_search_states,
        evaluated_states: 0,
        feasible_portfolio_count: 0,
        feasible_plans: Vec::new(),
        excluded_options,
        budget_exhausted: false,
    };

    search_portfolios(
        problem,
        &candidate_options,
        0,
        &[],
        &problem.initial_reservations,
        &problem.initial_occupancies,
        &[],
        Accumulator::default(),
        &mut state,
    )?;

    let pareto_frontier = pareto_frontier(state.feasible_plans);
    let completeness = if state.budget_exhausted {
        SearchCompleteness::BudgetExhausted
    } else {
        SearchCompleteness::Exhaustive
    };
    let infeasible_declared_portfolio =
        completeness == SearchCompleteness::Exhaustive && pareto_frontier.is_empty();

    Ok(PortfolioSearchResult {
        policy,
        completeness,
        evaluated_states: state.evaluated_states,
        feasible_portfolio_count: state.feasible_portfolio_count,
        pareto_frontier,
        excluded_options: state.excluded_options,
        infeasible_declared_portfolio,
    })
}

fn validate_problem(problem: &PortfolioProblem) -> Result<(), PortfolioError> {
    if problem.currency_code.len() != 3
        || !problem
            .currency_code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase())
    {
        return Err(PortfolioError::InvalidCurrency);
    }
    if problem.work_units.is_empty() {
        return Err(PortfolioError::InvalidPortfolioShape);
    }

    let mut seen_units = BTreeSet::new();
    for unit in &problem.work_units {
        unit.validate(&seen_units)?;
        if !seen_units.insert(unit.work_unit_id.as_str()) {
            return Err(PortfolioError::DuplicateWorkUnit(unit.work_unit_id.clone()));
        }
    }
    Ok(())
}

fn candidate_options_for_policy(
    problem: &PortfolioProblem,
    policy: PortfolioPolicy,
    excluded: &mut Vec<ExcludedOption>,
) -> Result<Vec<Vec<ModeOption>>, PortfolioError> {
    let mut result = Vec::with_capacity(problem.work_units.len());

    for unit in &problem.work_units {
        let mut eligible = Vec::new();
        for option in &unit.mode_options {
            if option.eligibility.status == EligibilityStatus::Eligible {
                eligible.push(option.clone());
            } else {
                excluded.push(ExcludedOption {
                    option_id: option.option_id.clone(),
                    work_unit_id: option.work_unit_id.clone(),
                    status: option.eligibility.status,
                    reason: option.eligibility.reason.clone(),
                });
            }
        }
        eligible.sort_by(|left, right| left.option_id.cmp(&right.option_id));

        let selected = match policy {
            PortfolioPolicy::ExactPareto => eligible,
            PortfolioPolicy::HumanOnly => eligible
                .into_iter()
                .filter(|option| option.mode == ExecutionMode::Human)
                .collect(),
            PortfolioPolicy::RobotizeAllEligible => {
                let robots: Vec<_> = eligible
                    .iter()
                    .filter(|option| option.mode == ExecutionMode::Robot)
                    .cloned()
                    .collect();
                if robots.is_empty() {
                    eligible
                        .into_iter()
                        .filter(|option| option.mode == ExecutionMode::Human)
                        .collect()
                } else {
                    robots
                }
            }
            PortfolioPolicy::FastestLocal => eligible
                .into_iter()
                .min_by(|left, right| {
                    left.duration_ms()
                        .cmp(&right.duration_ms())
                        .then_with(|| {
                            left.cost
                                .total_minor_units()
                                .unwrap_or(u64::MAX)
                                .cmp(&right.cost.total_minor_units().unwrap_or(u64::MAX))
                        })
                        .then_with(|| left.option_id.cmp(&right.option_id))
                })
                .into_iter()
                .collect(),
            PortfolioPolicy::LowestCostLocal => eligible
                .into_iter()
                .min_by(|left, right| {
                    left.cost
                        .total_minor_units()
                        .unwrap_or(u64::MAX)
                        .cmp(&right.cost.total_minor_units().unwrap_or(u64::MAX))
                        .then_with(|| left.duration_ms().cmp(&right.duration_ms()))
                        .then_with(|| left.option_id.cmp(&right.option_id))
                })
                .into_iter()
                .collect(),
        };
        result.push(selected);
    }

    excluded.sort_by(|left, right| left.option_id.cmp(&right.option_id));
    Ok(result)
}

#[derive(Clone, Debug, Default)]
struct Accumulator {
    total_cost_minor_units: u64,
    total_workflow_crew_minutes: u64,
    skilled_crew_minutes: u64,
    energy_wh: u64,
    makespan_ms: u64,
    human_work_units: usize,
    assisted_work_units: usize,
    robot_work_units: usize,
    offsite_work_units: usize,
}

impl Accumulator {
    fn with_option(&self, option: &ModeOption) -> Result<Self, PortfolioError> {
        let mut next = self.clone();
        next.total_cost_minor_units = next
            .total_cost_minor_units
            .checked_add(option.cost.total_minor_units()?)
            .ok_or(PortfolioError::CostOverflow)?;
        next.total_workflow_crew_minutes = next
            .total_workflow_crew_minutes
            .checked_add(option.total_crew_minutes()?)
            .ok_or(PortfolioError::CrewOverflow)?;
        next.skilled_crew_minutes = next
            .skilled_crew_minutes
            .checked_add(option.skilled_crew_minutes)
            .ok_or(PortfolioError::CrewOverflow)?;
        next.energy_wh = next
            .energy_wh
            .checked_add(option.energy_wh)
            .ok_or(PortfolioError::EnergyOverflow)?;
        next.makespan_ms = next.makespan_ms.max(option.scheduled_end_ms);

        match option.mode {
            ExecutionMode::Human => next.human_work_units += 1,
            ExecutionMode::Assisted => next.assisted_work_units += 1,
            ExecutionMode::Robot => next.robot_work_units += 1,
            ExecutionMode::Offsite => next.offsite_work_units += 1,
        }
        Ok(next)
    }

    fn metrics(&self, problem: &PortfolioProblem) -> PortfolioMetrics {
        PortfolioMetrics {
            total_cost_minor_units: self.total_cost_minor_units,
            currency_code: problem.currency_code.clone(),
            makespan_ms: self.makespan_ms.saturating_sub(problem.project_start_ms),
            total_workflow_crew_minutes: self.total_workflow_crew_minutes,
            skilled_crew_minutes: self.skilled_crew_minutes,
            energy_wh: self.energy_wh,
            human_work_units: self.human_work_units,
            assisted_work_units: self.assisted_work_units,
            robot_work_units: self.robot_work_units,
            offsite_work_units: self.offsite_work_units,
            planned_work_units: problem.work_units.len(),
        }
    }
}

#[derive(Clone, Debug)]
struct SearchState {
    max_states: usize,
    evaluated_states: usize,
    feasible_portfolio_count: usize,
    feasible_plans: Vec<PortfolioPlan>,
    excluded_options: Vec<ExcludedOption>,
    budget_exhausted: bool,
}

#[allow(clippy::too_many_arguments)]
fn search_portfolios(
    problem: &PortfolioProblem,
    candidate_options: &[Vec<ModeOption>],
    index: usize,
    selected_options: &[ModeOption],
    resources: &ReservationBook,
    occupancies: &SpatialOccupancyBook,
    choices: &[PortfolioChoice],
    accumulator: Accumulator,
    state: &mut SearchState,
) -> Result<(), PortfolioError> {
    if state.evaluated_states >= state.max_states {
        state.budget_exhausted = true;
        return Ok(());
    }
    state.evaluated_states += 1;

    if index == problem.work_units.len() {
        state.feasible_portfolio_count += 1;
        state.feasible_plans.push(PortfolioPlan {
            choices: choices.to_vec(),
            metrics: accumulator.metrics(problem),
        });
        return Ok(());
    }

    let unit = &problem.work_units[index];
    let options = &candidate_options[index];

    for option in options {
        if state.evaluated_states >= state.max_states {
            state.budget_exhausted = true;
            break;
        }
        if !predecessors_satisfied(unit, option, selected_options) {
            continue;
        }

        let mut next_resources = resources.clone();
        let mut next_occupancies = occupancies.clone();
        if admit_option(
            option,
            &mut next_resources,
            &mut next_occupancies,
            problem.project_start_ms,
        )
        .is_err()
        {
            continue;
        }

        let next_accumulator = accumulator.with_option(option)?;
        let mut next_selected = selected_options.to_vec();
        next_selected.push(option.clone());
        let mut next_choices = choices.to_vec();
        next_choices.push(PortfolioChoice {
            work_unit_id: unit.work_unit_id.clone(),
            option_id: option.option_id.clone(),
            mode: option.mode,
            scheduled_start_ms: option.scheduled_start_ms,
            scheduled_end_ms: option.scheduled_end_ms,
        });

        search_portfolios(
            problem,
            candidate_options,
            index + 1,
            &next_selected,
            &next_resources,
            &next_occupancies,
            &next_choices,
            next_accumulator,
            state,
        )?;
        if state.budget_exhausted {
            break;
        }
    }
    Ok(())
}

fn predecessors_satisfied(
    unit: &RequiredWorkUnit,
    option: &ModeOption,
    selected_options: &[ModeOption],
) -> bool {
    unit.predecessor_work_unit_ids.iter().all(|predecessor| {
        selected_options
            .iter()
            .find(|selected| selected.work_unit_id == *predecessor)
            .is_some_and(|selected| option.scheduled_start_ms >= selected.scheduled_end_ms)
    })
}

fn admit_option(
    option: &ModeOption,
    resources: &mut ReservationBook,
    occupancies: &mut SpatialOccupancyBook,
    now_ms: u64,
) -> Result<(), PortfolioError> {
    let mut remaining_requests = option.reservation_requests.clone();
    if let Some(occupancy) = &option.spatial_occupancy {
        let Some(index) = remaining_requests
            .iter()
            .position(|request| request == &occupancy.reservation)
        else {
            return Err(PortfolioError::OccupancyReservationMismatch(
                option.option_id.clone(),
            ));
        };
        remaining_requests.remove(index);
        occupancies
            .reserve_and_insert(resources, occupancy.clone(), now_ms)
            .map_err(PortfolioError::Spatial)?;
    }

    for request in remaining_requests {
        resources
            .reserve(request, now_ms)
            .map_err(PortfolioError::Resource)?;
    }
    Ok(())
}

fn pareto_frontier(mut plans: Vec<PortfolioPlan>) -> Vec<PortfolioPlan> {
    plans.sort_by(|left, right| {
        (
            left.metrics.total_cost_minor_units,
            left.metrics.makespan_ms,
            left.metrics.skilled_crew_minutes,
            &left.choices,
        )
            .cmp(&(
                right.metrics.total_cost_minor_units,
                right.metrics.makespan_ms,
                right.metrics.skilled_crew_minutes,
                &right.choices,
            ))
    });
    let mut frontier: Vec<PortfolioPlan> = Vec::new();
    for candidate in plans {
        let dominated = frontier
            .iter()
            .any(|existing| dominates(&existing.metrics, &candidate.metrics));
        if dominated {
            continue;
        }
        frontier.retain(|existing| !dominates(&candidate.metrics, &existing.metrics));
        if !frontier
            .iter()
            .any(|existing| existing.choices == candidate.choices)
        {
            frontier.push(candidate);
        }
    }
    frontier.sort_by(|left, right| {
        (
            left.metrics.total_cost_minor_units,
            left.metrics.makespan_ms,
            left.metrics.skilled_crew_minutes,
            &left.choices,
        )
            .cmp(&(
                right.metrics.total_cost_minor_units,
                right.metrics.makespan_ms,
                right.metrics.skilled_crew_minutes,
                &right.choices,
            ))
    });
    frontier
}

fn dominates(left: &PortfolioMetrics, right: &PortfolioMetrics) -> bool {
    let no_worse = left.total_cost_minor_units <= right.total_cost_minor_units
        && left.makespan_ms <= right.makespan_ms
        && left.skilled_crew_minutes <= right.skilled_crew_minutes;
    let strictly_better = left.total_cost_minor_units < right.total_cost_minor_units
        || left.makespan_ms < right.makespan_ms
        || left.skilled_crew_minutes < right.skilled_crew_minutes;
    no_worse && strictly_better
}

fn require_reference(name: &'static str, value: Option<&str>) -> Result<(), PortfolioError> {
    match value {
        Some(value) => require_non_empty(name, value),
        None => Err(PortfolioError::EmptyField(name)),
    }
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), PortfolioError> {
    if value.trim().is_empty() {
        return Err(PortfolioError::EmptyField(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use construction_robot_interop_core::resources::ReservationState;
    use construction_spacetime_occupancy_core::{Bounds3Mm, OccupancyKind, SiteBinding};

    fn site_binding() -> SiteBinding {
        SiteBinding {
            floor_id: "reference-floor-01".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-digest-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
        }
    }

    fn bounds(min: [i64; 3], max: [i64; 3]) -> Bounds3Mm {
        Bounds3Mm { min, max }
    }

    fn resource_book(resources: &[(&str, u32)]) -> ReservationBook {
        let mut book =
            ReservationBook::new("site-epoch-1", "site-clock-1", 0).expect("resource book");
        for (resource_id, capacity) in resources {
            book.register_resource(*resource_id, *capacity)
                .expect("resource capacity");
        }
        book
    }

    fn occupancy(
        id: &str,
        task_id: &str,
        area: Bounds3Mm,
        start_ms: u64,
        end_ms: u64,
        resource_id: &str,
    ) -> SpatialOccupancy {
        SpatialOccupancy {
            occupancy_id: format!("occupancy-{id}"),
            task_id: task_id.to_owned(),
            kind: OccupancyKind::Workface,
            binding: site_binding(),
            bounds: area,
            uncertainty_margin_mm: [0, 0, 0],
            clearance_margin_mm: [0, 0, 0],
            reservation: reservation(id, task_id, start_ms, end_ms, resource_id),
        }
    }

    fn reservation(
        id: &str,
        task_id: &str,
        start_ms: u64,
        end_ms: u64,
        resource_id: &str,
    ) -> ReservationRequest {
        ReservationRequest {
            reservation_id: format!("reservation-{id}"),
            resource_id: resource_id.to_owned(),
            task_id: task_id.to_owned(),
            owner_id: format!("owner-{task_id}"),
            site_epoch: "site-epoch-1".to_owned(),
            clock_domain_id: "site-clock-1".to_owned(),
            clock_uncertainty_ms: 0,
            interval_start_ms: start_ms,
            interval_end_ms: end_ms,
            expires_at_ms: end_ms + 10_000,
            units: 1,
        }
    }

    fn eligibility(id: &str) -> ModeEligibility {
        ModeEligibility::eligible(
            format!("capability:{id}"),
            format!("qualification:{id}"),
            format!("authority-scope:{id}"),
        )
    }

    fn excluded(status: EligibilityStatus, reason: &str) -> ModeEligibility {
        ModeEligibility::excluded(status, reason.to_owned())
    }

    fn cost(labour: u64, equipment: u64, energy: u64, setup: u64) -> CostBreakdown {
        CostBreakdown {
            labour_minor_units: labour,
            equipment_minor_units: equipment,
            energy_minor_units: energy,
            setup_minor_units: setup,
            ..CostBreakdown::default()
        }
    }

    fn option(
        option_id: &str,
        work_unit_id: &str,
        mode: ExecutionMode,
        start_ms: u64,
        end_ms: u64,
        resource_id: &str,
        area: Option<Bounds3Mm>,
        costs: CostBreakdown,
        crew_minutes: u64,
        skilled_crew_minutes: u64,
    ) -> ModeOption {
        let occupancy = area
            .map(|area| occupancy(option_id, work_unit_id, area, start_ms, end_ms, resource_id));
        let requests = if let Some(occupancy) = &occupancy {
            vec![occupancy.reservation.clone()]
        } else {
            vec![reservation(
                option_id,
                work_unit_id,
                start_ms,
                end_ms,
                resource_id,
            )]
        };
        ModeOption {
            option_id: option_id.to_owned(),
            work_unit_id: work_unit_id.to_owned(),
            mode,
            eligibility: eligibility(option_id),
            scheduled_start_ms: start_ms,
            scheduled_end_ms: end_ms,
            acceptance_contract_ref: format!("acceptance:{work_unit_id}"),
            crew_minutes_by_role: BTreeMap::from([("crew".to_owned(), crew_minutes)]),
            skilled_crew_minutes,
            energy_wh: 100,
            cost: costs,
            reservation_requests: requests,
            spatial_occupancy: occupancy,
        }
    }

    fn unit(id: &str, options: Vec<ModeOption>, predecessors: &[&str]) -> RequiredWorkUnit {
        RequiredWorkUnit {
            work_unit_id: id.to_owned(),
            functional_unit_class: "accepted-panel-module".to_owned(),
            acceptance_contract_ref: format!("acceptance:{id}"),
            predecessor_work_unit_ids: predecessors.iter().map(|item| (*item).to_owned()).collect(),
            mode_options: options,
        }
    }

    fn problem(
        work_units: Vec<RequiredWorkUnit>,
        resources: &[(&str, u32)],
        _spatial: bool,
    ) -> PortfolioProblem {
        PortfolioProblem {
            currency_code: "USD".to_owned(),
            project_start_ms: 0,
            max_search_states: 100_000,
            work_units,
            initial_reservations: resource_book(resources),
            initial_occupancies: SpatialOccupancyBook::new(site_binding(), "site-clock-1")
                .expect("spatial book"),
        }
    }

    #[test]
    fn exact_portfolio_selects_a_hybrid_plan_on_the_frozen_case() {
        let units = vec![
            unit(
                "panel-a",
                vec![
                    option(
                        "human-a",
                        "panel-a",
                        ExecutionMode::Human,
                        0,
                        100,
                        "crew",
                        None,
                        cost(1_000, 0, 0, 0),
                        60,
                        60,
                    ),
                    option(
                        "robot-a",
                        "panel-a",
                        ExecutionMode::Robot,
                        0,
                        60,
                        "robot",
                        Some(bounds([0, 0, 0], [1_000, 1_000, 1_000])),
                        cost(200, 500, 50, 100),
                        15,
                        5,
                    ),
                ],
                &[],
            ),
            unit(
                "panel-b",
                vec![
                    option(
                        "human-b",
                        "panel-b",
                        ExecutionMode::Human,
                        100,
                        200,
                        "crew",
                        None,
                        cost(1_000, 0, 0, 0),
                        60,
                        60,
                    ),
                    option(
                        "robot-b",
                        "panel-b",
                        ExecutionMode::Robot,
                        60,
                        120,
                        "robot",
                        Some(bounds([2_000, 0, 0], [3_000, 1_000, 1_000])),
                        cost(200, 500, 50, 100),
                        15,
                        5,
                    ),
                ],
                &["panel-a"],
            ),
        ];
        let p = problem(units, &[("crew", 1), ("robot", 1)], true);
        let result = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("portfolio search");

        assert_eq!(result.completeness, SearchCompleteness::Exhaustive);
        assert!(!result.infeasible_declared_portfolio);
        assert!(result.feasible_portfolio_count >= 1);
        assert!(result.pareto_frontier.iter().any(|plan| {
            plan.metrics.robot_work_units == 1 && plan.metrics.human_work_units == 1
        }));
        assert!(
            result
                .pareto_frontier
                .iter()
                .all(|plan| plan.metrics.planned_work_units == 2)
        );
    }

    #[test]
    fn robotize_all_eligible_is_not_assumed_to_be_cheapest() {
        let units = vec![unit(
            "wall-layout",
            vec![
                option(
                    "human",
                    "wall-layout",
                    ExecutionMode::Human,
                    0,
                    100,
                    "crew",
                    None,
                    cost(100, 0, 0, 0),
                    80,
                    80,
                ),
                option(
                    "robot",
                    "wall-layout",
                    ExecutionMode::Robot,
                    0,
                    60,
                    "robot",
                    None,
                    cost(100, 1_000, 100, 100),
                    10,
                    5,
                ),
            ],
            &[],
        )];
        let p = problem(units, &[("crew", 1), ("robot", 1)], false);
        let human = solve_portfolio(&p, PortfolioPolicy::HumanOnly).expect("human baseline");
        let robot =
            solve_portfolio(&p, PortfolioPolicy::RobotizeAllEligible).expect("robot baseline");
        let optimal = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("exact portfolio");

        assert_eq!(human.pareto_frontier[0].metrics.total_cost_minor_units, 100);
        assert_eq!(
            robot.pareto_frontier[0].metrics.total_cost_minor_units,
            1_300
        );
        assert_eq!(
            optimal.pareto_frontier[0].metrics.total_cost_minor_units,
            100
        );
    }

    #[test]
    fn robot_policy_falls_back_to_human_only_when_robot_is_not_eligible() {
        let mut robot = option(
            "robot-unknown",
            "repair",
            ExecutionMode::Robot,
            0,
            50,
            "robot",
            None,
            cost(100, 100, 0, 0),
            10,
            5,
        );
        robot.eligibility = excluded(EligibilityStatus::Unknown, "calibration is not current");
        let human = option(
            "human",
            "repair",
            ExecutionMode::Human,
            0,
            100,
            "crew",
            None,
            cost(500, 0, 0, 0),
            60,
            60,
        );
        let p = problem(
            vec![unit("repair", vec![robot, human], &[])],
            &[("crew", 1), ("robot", 1)],
            false,
        );

        let result = solve_portfolio(&p, PortfolioPolicy::RobotizeAllEligible)
            .expect("robot policy falls back only when robot is ineligible");
        assert_eq!(result.pareto_frontier.len(), 1);
        assert_eq!(
            result.pareto_frontier[0].choices[0].mode,
            ExecutionMode::Human
        );
        assert_eq!(result.excluded_options.len(), 1);
        assert_eq!(
            result.excluded_options[0].status,
            EligibilityStatus::Unknown
        );
    }

    #[test]
    fn resource_capacity_rejects_overlapping_robot_portfolios() {
        let units = vec![
            unit(
                "task-a",
                vec![option(
                    "robot-a",
                    "task-a",
                    ExecutionMode::Robot,
                    0,
                    100,
                    "robot-fleet",
                    None,
                    cost(100, 100, 0, 0),
                    10,
                    5,
                )],
                &[],
            ),
            unit(
                "task-b",
                vec![option(
                    "robot-b",
                    "task-b",
                    ExecutionMode::Robot,
                    50,
                    150,
                    "robot-fleet",
                    None,
                    cost(100, 100, 0, 0),
                    10,
                    5,
                )],
                &[],
            ),
        ];
        let p = problem(units, &[("robot-fleet", 1)], false);
        let result = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("portfolio search");
        assert_eq!(result.completeness, SearchCompleteness::Exhaustive);
        assert!(result.infeasible_declared_portfolio);
        assert!(result.pareto_frontier.is_empty());
    }

    #[test]
    fn space_time_conflict_rejects_overlapping_workfaces() {
        let units = vec![
            unit(
                "task-a",
                vec![option(
                    "robot-a",
                    "task-a",
                    ExecutionMode::Robot,
                    0,
                    100,
                    "robot-a",
                    Some(bounds([0, 0, 0], [1_000, 1_000, 1_000])),
                    cost(100, 100, 0, 0),
                    10,
                    5,
                )],
                &[],
            ),
            unit(
                "task-b",
                vec![option(
                    "robot-b",
                    "task-b",
                    ExecutionMode::Robot,
                    50,
                    150,
                    "robot-b",
                    Some(bounds([500, 500, 0], [1_500, 1_500, 1_000])),
                    cost(100, 100, 0, 0),
                    10,
                    5,
                )],
                &[],
            ),
        ];
        let p = problem(units, &[("robot-a", 1), ("robot-b", 1)], true);
        let result = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("portfolio search");
        assert!(result.infeasible_declared_portfolio);
        assert!(result.pareto_frontier.is_empty());
    }

    #[test]
    fn ineligible_unknown_and_stale_options_never_enter_the_frontier() {
        let mut unknown = option(
            "unknown-robot",
            "panel",
            ExecutionMode::Robot,
            0,
            50,
            "robot",
            None,
            cost(10, 10, 0, 0),
            10,
            5,
        );
        unknown.eligibility = excluded(EligibilityStatus::Unknown, "no current evidence");
        let mut stale = option(
            "stale-assisted",
            "panel",
            ExecutionMode::Assisted,
            0,
            75,
            "assisted",
            None,
            cost(20, 20, 0, 0),
            20,
            15,
        );
        stale.eligibility = excluded(EligibilityStatus::Stale, "design revision changed");
        let human = option(
            "human",
            "panel",
            ExecutionMode::Human,
            0,
            100,
            "crew",
            None,
            cost(100, 0, 0, 0),
            80,
            80,
        );
        let p = problem(
            vec![unit("panel", vec![unknown, stale, human], &[])],
            &[("crew", 1), ("robot", 1), ("assisted", 1)],
            false,
        );
        let result = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("exact portfolio");
        assert_eq!(result.pareto_frontier.len(), 1);
        assert_eq!(
            result.pareto_frontier[0].choices[0].mode,
            ExecutionMode::Human
        );
        assert_eq!(result.excluded_options.len(), 2);
    }

    #[test]
    fn non_topological_dependencies_are_rejected_before_search() {
        let units = vec![
            unit(
                "second",
                vec![option(
                    "second-human",
                    "second",
                    ExecutionMode::Human,
                    100,
                    200,
                    "crew",
                    None,
                    cost(100, 0, 0, 0),
                    60,
                    60,
                )],
                &["first"],
            ),
            unit(
                "first",
                vec![option(
                    "first-human",
                    "first",
                    ExecutionMode::Human,
                    0,
                    100,
                    "crew",
                    None,
                    cost(100, 0, 0, 0),
                    60,
                    60,
                )],
                &[],
            ),
        ];
        let p = problem(units, &[("crew", 1)], false);
        assert!(matches!(
            solve_portfolio(&p, PortfolioPolicy::ExactPareto),
            Err(PortfolioError::NonTopologicalDependency { .. })
        ));
    }

    #[test]
    fn search_budget_exhaustion_never_claims_infeasible_or_optimal() {
        let units = vec![
            unit(
                "one",
                vec![
                    option(
                        "human-one",
                        "one",
                        ExecutionMode::Human,
                        0,
                        100,
                        "crew",
                        None,
                        cost(100, 0, 0, 0),
                        60,
                        60,
                    ),
                    option(
                        "robot-one",
                        "one",
                        ExecutionMode::Robot,
                        0,
                        50,
                        "robot",
                        None,
                        cost(200, 100, 0, 0),
                        10,
                        5,
                    ),
                ],
                &[],
            ),
            unit(
                "two",
                vec![
                    option(
                        "human-two",
                        "two",
                        ExecutionMode::Human,
                        100,
                        200,
                        "crew",
                        None,
                        cost(100, 0, 0, 0),
                        60,
                        60,
                    ),
                    option(
                        "robot-two",
                        "two",
                        ExecutionMode::Robot,
                        50,
                        100,
                        "robot",
                        None,
                        cost(200, 100, 0, 0),
                        10,
                        5,
                    ),
                ],
                &["one"],
            ),
        ];
        let mut p = problem(units, &[("crew", 2), ("robot", 2)], false);
        p.max_search_states = 1;
        let result = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("bounded result");
        assert_eq!(result.completeness, SearchCompleteness::BudgetExhausted);
        assert!(!result.infeasible_declared_portfolio);
        assert_eq!(result.evaluated_states, 1);
    }

    #[test]
    fn pareto_frontier_preserves_cost_time_tradeoff() {
        let units = vec![unit(
            "module",
            vec![
                option(
                    "human-slow-cheap",
                    "module",
                    ExecutionMode::Human,
                    0,
                    200,
                    "crew",
                    None,
                    cost(100, 0, 0, 0),
                    120,
                    120,
                ),
                option(
                    "robot-fast-expensive",
                    "module",
                    ExecutionMode::Robot,
                    0,
                    100,
                    "robot",
                    None,
                    cost(500, 100, 50, 50),
                    20,
                    10,
                ),
            ],
            &[],
        )];
        let p = problem(units, &[("crew", 1), ("robot", 1)], false);
        let result = solve_portfolio(&p, PortfolioPolicy::ExactPareto).expect("frontier");
        assert_eq!(result.pareto_frontier.len(), 2);
        assert!(
            result
                .pareto_frontier
                .iter()
                .any(|plan| plan.metrics.makespan_ms == 200)
        );
        assert!(
            result
                .pareto_frontier
                .iter()
                .any(|plan| plan.metrics.makespan_ms == 100)
        );
    }
}
