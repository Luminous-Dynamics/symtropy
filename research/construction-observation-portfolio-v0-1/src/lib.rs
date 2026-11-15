
//! Bounded exact multi-observation campaign planning.
//!
//! A campaign is a set of declared observation actions whose claimed region coverage
//! spans the requested work needs and whose resource reservations and spatial envelopes
//! can be admitted together by the existing owners.
//!
//! The planner uses a bounded exhaustive search for small action catalogues. It never
//! mutates the caller's production books: each include branch works on scratch clones.
//! This is a planning result, not a permit, an observation receipt, a coverage proof or
//! independent acceptance of construction output.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use construction_observation_planning_core::{ObservationAction, WorkNeed};
use construction_robot_interop_core::resources::ReservationBook;
use construction_spacetime_occupancy_core::{SpatialError, SpatialOccupancyBook};

/// Exact enumeration remains intentionally limited to small catalogues.
pub const MAX_EXACT_ACTIONS: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedObservationAction {
    pub action_id: String,
    pub covered_work_unit_ids: Vec<String>,
    pub duration_minutes: u64,
    pub crew_minutes: u64,
    pub cost_minor_units: u64,
    pub energy_wh: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCampaign {
    pub selected_actions: Vec<PlannedObservationAction>,
    pub covered_work_unit_ids: Vec<String>,
    pub uncovered_work_unit_ids: Vec<String>,
    pub complete: bool,
    pub total_observation_duration_minutes: u64,
    pub total_crew_minutes: u64,
    pub total_cost_minor_units: u64,
    pub cost_currency_code: Option<String>,
    pub total_energy_wh: u64,
    /// Span of uncertainty-expanded half-open reservation intervals.
    pub scheduled_span_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPortfolioAction {
    pub action_id: String,
    /// The previously selected prefix under which this action was rejected.
    pub selected_prefix_action_ids: Vec<String>,
    /// Existing resource/occupancy owner error. No local reimplementation of either
    /// capacity or spatial conflict semantics is used.
    pub blocker: SpatialError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignSearchResult {
    /// Best complete campaign found, or best partial campaign if no complete campaign
    /// was found within the search budget. A partial campaign never releases work.
    pub best_campaign: ObservationCampaign,
    pub nodes_visited: usize,
    pub search_complete: bool,
    /// True only when exhaustive search completed and the best campaign covers every
    /// declared work need.
    pub optimality_proven: bool,
    /// True only when exhaustive search completed and no complete campaign exists.
    pub infeasibility_proven: bool,
    pub rejected_actions: Vec<RejectedPortfolioAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortfolioError {
    EmptyField(&'static str),
    InvalidCurrencyCode(String),
    InvalidDuration { action_id: String },
    NoCoveredRegions { action_id: String },
    DuplicateActionId(String),
    DuplicateOccupancyId(String),
    DuplicateReservationId(String),
    DuplicateWorkUnitId(String),
    MixedCostCurrency { expected: String, received: String },
    ZeroNodeBudget,
    TooManyActions { maximum: usize, received: usize },
    ArithmeticOverflow,
    TimeExpansionOverflow,
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), PortfolioError> {
    if value.trim().is_empty() {
        return Err(PortfolioError::EmptyField(name));
    }
    Ok(())
}

fn validate_currency_code(value: &str) -> Result<(), PortfolioError> {
    if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(PortfolioError::InvalidCurrencyCode(value.to_owned()));
    }
    Ok(())
}

fn validate_inputs(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    node_budget: usize,
) -> Result<Option<String>, PortfolioError> {
    if node_budget == 0 {
        return Err(PortfolioError::ZeroNodeBudget);
    }
    if actions.len() > MAX_EXACT_ACTIONS {
        return Err(PortfolioError::TooManyActions {
            maximum: MAX_EXACT_ACTIONS,
            received: actions.len(),
        });
    }

    let mut work_ids = BTreeSet::new();
    for need in work_needs {
        require_non_empty("work_unit_id", &need.work_unit_id)?;
        require_non_empty("region_id", &need.region_id)?;
        if !work_ids.insert(need.work_unit_id.as_str()) {
            return Err(PortfolioError::DuplicateWorkUnitId(
                need.work_unit_id.clone(),
            ));
        }
    }

    let mut action_ids = BTreeSet::new();
    let mut occupancy_ids = BTreeSet::new();
    let mut reservation_ids = BTreeSet::new();
    let mut currency: Option<String> = None;

    for action in actions {
        require_non_empty("action_id", &action.action_id)?;
        require_non_empty("coverage_profile_ref", &action.coverage_profile_ref)?;
        require_non_empty("calibration_ref", &action.calibration_ref)?;
        require_non_empty("occupancy_id", &action.occupancy.occupancy_id)?;
        require_non_empty("occupancy_task_id", &action.occupancy.task_id)?;
        require_non_empty(
            "reservation_id",
            &action.occupancy.reservation.reservation_id,
        )?;
        if action.duration_minutes == 0 {
            return Err(PortfolioError::InvalidDuration {
                action_id: action.action_id.clone(),
            });
        }
        if action.covered_region_ids.is_empty() {
            return Err(PortfolioError::NoCoveredRegions {
                action_id: action.action_id.clone(),
            });
        }
        for region in &action.covered_region_ids {
            require_non_empty("covered_region_id", region)?;
        }
        validate_currency_code(&action.cost_currency_code)?;

        if !action_ids.insert(action.action_id.as_str()) {
            return Err(PortfolioError::DuplicateActionId(action.action_id.clone()));
        }
        if !occupancy_ids.insert(action.occupancy.occupancy_id.as_str()) {
            return Err(PortfolioError::DuplicateOccupancyId(
                action.occupancy.occupancy_id.clone(),
            ));
        }
        if !reservation_ids.insert(action.occupancy.reservation.reservation_id.as_str()) {
            return Err(PortfolioError::DuplicateReservationId(
                action.occupancy.reservation.reservation_id.clone(),
            ));
        }

        match currency.as_deref() {
            Some(expected) if expected != action.cost_currency_code => {
                return Err(PortfolioError::MixedCostCurrency {
                    expected: expected.to_owned(),
                    received: action.cost_currency_code.clone(),
                });
            }
            None => currency = Some(action.cost_currency_code.clone()),
            _ => {}
        }
    }

    Ok(currency)
}

fn expanded_interval(action: &ObservationAction) -> Result<(u64, u64), PortfolioError> {
    let reservation = &action.occupancy.reservation;
    let start = reservation
        .interval_start_ms
        .checked_sub(reservation.clock_uncertainty_ms)
        .ok_or(PortfolioError::TimeExpansionOverflow)?;
    let end = reservation
        .interval_end_ms
        .checked_add(reservation.clock_uncertainty_ms)
        .ok_or(PortfolioError::TimeExpansionOverflow)?;
    if start >= end {
        return Err(PortfolioError::TimeExpansionOverflow);
    }
    Ok((start, end))
}

fn build_campaign(
    actions: &[&ObservationAction],
    selected_indices: &[usize],
    covered: &[bool],
    work_needs: &[WorkNeed],
    currency: Option<&str>,
) -> Result<ObservationCampaign, PortfolioError> {
    let mut selected_actions = Vec::with_capacity(selected_indices.len());
    let mut covered_work_unit_ids = Vec::new();
    let mut uncovered_work_unit_ids = Vec::new();
    let mut total_observation_duration_minutes = 0_u64;
    let mut total_crew_minutes = 0_u64;
    let mut total_cost_minor_units = 0_u64;
    let mut total_energy_wh = 0_u64;
    let mut earliest_start: Option<u64> = None;
    let mut latest_end: Option<u64> = None;

    for (need_index, need) in work_needs.iter().enumerate() {
        if covered[need_index] {
            covered_work_unit_ids.push(need.work_unit_id.clone());
        } else {
            uncovered_work_unit_ids.push(need.work_unit_id.clone());
        }
    }
    covered_work_unit_ids.sort();
    uncovered_work_unit_ids.sort();

    let mut attributed_work_units: BTreeSet<String> = BTreeSet::new();
    for action_index in selected_indices {
        let action = actions[*action_index];
        let covered_ids: Vec<String> = work_needs
            .iter()
            .enumerate()
            .filter(|(index, need)| {
                covered[*index]
                    && !attributed_work_units.contains(need.work_unit_id.as_str())
                    && action
                        .covered_region_ids
                        .iter()
                        .any(|region| region == &need.region_id)
            })
            .map(|(_, need)| need.work_unit_id.clone())
            .collect();
        attributed_work_units.extend(covered_ids.iter().cloned());

        total_observation_duration_minutes = total_observation_duration_minutes
            .checked_add(action.duration_minutes)
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        total_crew_minutes = total_crew_minutes
            .checked_add(action.crew_minutes)
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        total_cost_minor_units = total_cost_minor_units
            .checked_add(action.cost_minor_units)
            .ok_or(PortfolioError::ArithmeticOverflow)?;
        total_energy_wh = total_energy_wh
            .checked_add(action.energy_wh)
            .ok_or(PortfolioError::ArithmeticOverflow)?;

        let (start, end) = expanded_interval(action)?;
        earliest_start = Some(earliest_start.map_or(start, |current| current.min(start)));
        latest_end = Some(latest_end.map_or(end, |current| current.max(end)));

        selected_actions.push(PlannedObservationAction {
            action_id: action.action_id.clone(),
            covered_work_unit_ids: covered_ids,
            duration_minutes: action.duration_minutes,
            crew_minutes: action.crew_minutes,
            cost_minor_units: action.cost_minor_units,
            energy_wh: action.energy_wh,
        });
    }

    let scheduled_span_ms = match (earliest_start, latest_end) {
        (Some(start), Some(end)) => end
            .checked_sub(start)
            .ok_or(PortfolioError::TimeExpansionOverflow)?,
        _ => 0,
    };

    let complete = uncovered_work_unit_ids.is_empty();
    Ok(ObservationCampaign {
        selected_actions,
        covered_work_unit_ids,
        uncovered_work_unit_ids,
        complete,
        total_observation_duration_minutes,
        total_crew_minutes,
        total_cost_minor_units,
        cost_currency_code: currency.map(str::to_owned),
        total_energy_wh,
        scheduled_span_ms,
    })
}

fn campaign_is_better(candidate: &ObservationCampaign, current: &ObservationCampaign) -> bool {
    if candidate.covered_work_unit_ids.len() != current.covered_work_unit_ids.len() {
        return candidate.covered_work_unit_ids.len() > current.covered_work_unit_ids.len();
    }
    candidate
        .total_crew_minutes
        .cmp(&current.total_crew_minutes)
        .then_with(|| {
            candidate
                .total_cost_minor_units
                .cmp(&current.total_cost_minor_units)
        })
        .then_with(|| candidate.total_energy_wh.cmp(&current.total_energy_wh))
        .then_with(|| candidate.scheduled_span_ms.cmp(&current.scheduled_span_ms))
        .then_with(|| {
            candidate
                .selected_actions
                .len()
                .cmp(&current.selected_actions.len())
        })
        .then_with(|| {
            candidate
                .selected_actions
                .iter()
                .map(|action| action.action_id.as_str())
                .cmp(
                    current
                        .selected_actions
                        .iter()
                        .map(|action| action.action_id.as_str()),
                )
        })
        .is_lt()
}

pub struct GreedyCampaignResult {
    pub campaign: ObservationCampaign,
    /// Greedy is a heuristic. This is always false; use the exact bounded solver
    /// when an optimality or infeasibility proof is required.
    pub optimality_proven: bool,
    pub rejected_actions: Vec<RejectedPortfolioAction>,
}

struct GreedyCandidate {
    action_index: usize,
    newly_covered: Vec<usize>,
    occupancy_book: SpatialOccupancyBook,
    reservation_book: ReservationBook,
}

struct Search<'a> {
    actions: Vec<&'a ObservationAction>,
    coverage: Vec<Vec<usize>>,
    work_needs: &'a [WorkNeed],
    currency: Option<String>,
    now_ms: u64,
    node_budget: usize,
    nodes_visited: usize,
    exhausted: bool,
    best: Option<ObservationCampaign>,
    rejected_actions: Vec<RejectedPortfolioAction>,
}

impl Search<'_> {
    fn visit(
        &mut self,
        index: usize,
        covered: &mut [bool],
        selected: &mut Vec<usize>,
        occupancy_book: &SpatialOccupancyBook,
        reservation_book: &ReservationBook,
    ) -> Result<(), PortfolioError> {
        if self.exhausted {
            return Ok(());
        }
        if self.nodes_visited >= self.node_budget {
            self.exhausted = true;
            return Ok(());
        }
        self.nodes_visited += 1;

        let candidate = build_campaign(
            &self.actions,
            selected,
            covered,
            self.work_needs,
            self.currency.as_deref(),
        )?;
        if self
            .best
            .as_ref()
            .is_none_or(|current| campaign_is_better(&candidate, current))
        {
            self.best = Some(candidate);
        }

        if covered.iter().all(|value| *value) || index >= self.actions.len() {
            return Ok(());
        }

        // Safe optimistic bound: if the remaining action catalogue cannot cover every
        // outstanding need, this branch cannot produce a complete campaign.
        let mut possible = covered.to_vec();
        for remaining in index..self.actions.len() {
            for need_index in &self.coverage[remaining] {
                possible[*need_index] = true;
            }
        }
        if possible.iter().any(|value| !*value) {
            return Ok(());
        }

        let action = self.actions[index];
        let contributes = self.coverage[index]
            .iter()
            .any(|need_index| !covered[*need_index]);

        if contributes {
            let mut next_occupancy = occupancy_book.clone();
            let mut next_reservations = reservation_book.clone();

            match next_occupancy.reserve_and_insert(
                &mut next_reservations,
                action.occupancy.clone(),
                self.now_ms,
            ) {
                Ok(_) => {
                    let newly_covered: Vec<usize> = self.coverage[index]
                        .iter()
                        .copied()
                        .filter(|need_index| !covered[*need_index])
                        .collect();
                    for need_index in &newly_covered {
                        covered[*need_index] = true;
                    }
                    selected.push(index);
                    self.visit(
                        index + 1,
                        covered,
                        selected,
                        &next_occupancy,
                        &next_reservations,
                    )?;
                    selected.pop();
                    for need_index in newly_covered {
                        covered[need_index] = false;
                    }
                }
                Err(blocker) => {
                    self.rejected_actions.push(RejectedPortfolioAction {
                        action_id: action.action_id.clone(),
                        selected_prefix_action_ids: selected
                            .iter()
                            .map(|selected_index| self.actions[*selected_index].action_id.clone())
                            .collect(),
                        blocker,
                    });
                }
            }
        }

        self.visit(
            index + 1,
            covered,
            selected,
            occupancy_book,
            reservation_book,
        )
    }
}

/// Find the lexicographically best feasible observation campaign by bounded exhaustive
/// enumeration. The search returns its best complete campaign if it proves one; if the
/// node budget expires, optimality_proven is false. If exhaustive search completes
/// without covering all needs, infeasibility_proven is true.
///
/// The caller's live reservation and occupancy books are never mutated.
pub fn plan_observation_campaign(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
    reservation_book: &ReservationBook,
    now_ms: u64,
    node_budget: usize,
) -> Result<CampaignSearchResult, PortfolioError> {
    let currency = validate_inputs(actions, work_needs, node_budget)?;

    let mut ordered_actions: Vec<&ObservationAction> = actions.iter().collect();
    ordered_actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let coverage: Vec<Vec<usize>> = ordered_actions
        .iter()
        .map(|action| {
            work_needs
                .iter()
                .enumerate()
                .filter(|(_, need)| {
                    action
                        .covered_region_ids
                        .iter()
                        .any(|region| region == &need.region_id)
                })
                .map(|(index, _)| index)
                .collect()
        })
        .collect();

    let mut search = Search {
        actions: ordered_actions,
        coverage,
        work_needs,
        currency,
        now_ms,
        node_budget,
        nodes_visited: 0,
        exhausted: false,
        best: None,
        rejected_actions: Vec::new(),
    };
    let mut covered = vec![false; work_needs.len()];
    let mut selected = Vec::new();

    search.visit(
        0,
        &mut covered,
        &mut selected,
        occupancy_book,
        reservation_book,
    )?;

    let best_campaign = search.best.ok_or(PortfolioError::ArithmeticOverflow)?;
    let search_complete = !search.exhausted;
    let optimality_proven = search_complete && best_campaign.complete;
    let infeasibility_proven = search_complete && !best_campaign.complete;

    search.rejected_actions.sort_by(|left, right| {
        left.action_id.cmp(&right.action_id).then_with(|| {
            left.selected_prefix_action_ids
                .cmp(&right.selected_prefix_action_ids)
        })
    });

    Ok(CampaignSearchResult {
        best_campaign,
        nodes_visited: search.nodes_visited,
        search_complete,
        optimality_proven,
        infeasibility_proven,
        rejected_actions: search.rejected_actions,
    })
}

/// Deterministic multi-action greedy baseline for matched evaluation against the
/// bounded exact solver. At each step it chooses the feasible action covering the most
/// currently uncovered work units, then minimizes crew, cost, energy, duration and ID.
/// It never claims optimality and never mutates the caller's live planning books.
pub fn plan_greedy_observation_campaign(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
    reservation_book: &ReservationBook,
    now_ms: u64,
) -> Result<GreedyCampaignResult, PortfolioError> {
    let currency = validate_inputs(actions, work_needs, 1)?;
    let mut ordered_actions: Vec<&ObservationAction> = actions.iter().collect();
    ordered_actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let mut covered = vec![false; work_needs.len()];
    let mut selected = Vec::new();
    let mut scratch_occupancy = occupancy_book.clone();
    let mut scratch_reservations = reservation_book.clone();
    let mut rejected_actions = Vec::new();

    loop {
        if covered.iter().all(|value| *value) {
            break;
        }

        let mut best: Option<GreedyCandidate> = None;
        for (action_index, action) in ordered_actions.iter().enumerate() {
            let newly_covered: Vec<usize> = work_needs
                .iter()
                .enumerate()
                .filter(|(need_index, need)| {
                    !covered[*need_index]
                        && action
                            .covered_region_ids
                            .iter()
                            .any(|region| region == &need.region_id)
                })
                .map(|(need_index, _)| need_index)
                .collect();
            if newly_covered.is_empty() {
                continue;
            }

            let mut candidate_occupancy = scratch_occupancy.clone();
            let mut candidate_reservations = scratch_reservations.clone();
            match candidate_occupancy.reserve_and_insert(
                &mut candidate_reservations,
                action.occupancy.clone(),
                now_ms,
            ) {
                Ok(_) => {
                    let replace = best.as_ref().is_none_or(|current| {
                        let current_action = ordered_actions[current.action_index];
                        (
                            Reverse(newly_covered.len()),
                            action.crew_minutes,
                            action.cost_minor_units,
                            action.energy_wh,
                            action.duration_minutes,
                            action.action_id.as_str(),
                        ) < (
                            Reverse(current.newly_covered.len()),
                            current_action.crew_minutes,
                            current_action.cost_minor_units,
                            current_action.energy_wh,
                            current_action.duration_minutes,
                            current_action.action_id.as_str(),
                        )
                    });

                    if replace {
                        best = Some(GreedyCandidate {
                            action_index,
                            newly_covered,
                            occupancy_book: candidate_occupancy,
                            reservation_book: candidate_reservations,
                        });
                    }
                }
                Err(blocker) => rejected_actions.push(RejectedPortfolioAction {
                    action_id: action.action_id.clone(),
                    selected_prefix_action_ids: selected
                        .iter()
                        .map(|selected_index| ordered_actions[*selected_index].action_id.clone())
                        .collect(),
                    blocker,
                }),
            }
        }

        let Some(candidate) = best else {
            break;
        };
        for need_index in candidate.newly_covered {
            covered[need_index] = true;
        }
        selected.push(candidate.action_index);
        scratch_occupancy = candidate.occupancy_book;
        scratch_reservations = candidate.reservation_book;
    }

    let campaign = build_campaign(
        &ordered_actions,
        &selected,
        &covered,
        work_needs,
        currency.as_deref(),
    )?;
    rejected_actions.sort_by(|left, right| {
        left.action_id.cmp(&right.action_id).then_with(|| {
            left.selected_prefix_action_ids
                .cmp(&right.selected_prefix_action_ids)
        })
    });

    Ok(GreedyCampaignResult {
        campaign,
        optimality_proven: false,
        rejected_actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use construction_observation_planning_core::{ObservationAction, WorkNeed};
    use construction_robot_interop_core::resources::{ReservationBook, ReservationRequest};
    use construction_spacetime_occupancy_core::{
        Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancy,
    };

    fn binding() -> SiteBinding {
        SiteBinding {
            floor_id: "portfolio-floor-01".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-digest-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
        }
    }

    fn bounds(min: [i64; 3], max: [i64; 3]) -> Bounds3Mm {
        Bounds3Mm { min, max }
    }

    fn action(
        id: &str,
        regions: &[&str],
        x: i64,
        resource: &str,
        interval: (u64, u64),
        crew: u64,
        cost: u64,
    ) -> ObservationAction {
        let (start, end) = interval;
        let task_id = format!("task-{id}");
        ObservationAction {
            action_id: id.to_owned(),
            coverage_profile_ref: format!("coverage:{id}"),
            calibration_ref: format!("calibration:{id}"),
            covered_region_ids: regions.iter().map(|region| (*region).to_owned()).collect(),
            occupancy: SpatialOccupancy {
                occupancy_id: format!("occupancy-{id}"),
                task_id: task_id.clone(),
                kind: OccupancyKind::InspectionAccess,
                binding: binding(),
                bounds: bounds([x, 0, 0], [x + 500, 500, 1_000]),
                uncertainty_margin_mm: [0, 0, 0],
                clearance_margin_mm: [0, 0, 0],
                reservation: ReservationRequest {
                    reservation_id: format!("reservation-{id}"),
                    resource_id: resource.to_owned(),
                    task_id,
                    owner_id: format!("owner-{id}"),
                    site_epoch: "site-epoch-1".to_owned(),
                    clock_domain_id: "site-clock-1".to_owned(),
                    clock_uncertainty_ms: 0,
                    interval_start_ms: start,
                    interval_end_ms: end,
                    expires_at_ms: end + 1_000,
                    units: 1,
                },
            },
            duration_minutes: (end - start) / 60_000,
            crew_minutes: crew,
            cost_minor_units: cost,
            cost_currency_code: "USD".to_owned(),
            energy_wh: 50,
        }
    }

    fn need(id: &str, region: &str) -> WorkNeed {
        WorkNeed {
            work_unit_id: id.to_owned(),
            region_id: region.to_owned(),
            uncertainty_bound_mm: [100, 100, 100],
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct OracleScore {
        crew_minutes: u64,
        cost_minor_units: u64,
        energy_wh: u64,
        span_ms: u64,
        action_count: usize,
        action_ids: Vec<String>,
    }

    /// Independent tiny-fixture oracle. It enumerates subsets and uses fixture-authored
    /// forbidden action pairs as the ground truth for compatibility; it does not call
    /// the production reservation or geometry implementation.
    fn independent_subset_oracle(
        actions: &[ObservationAction],
        needs: &[WorkNeed],
        forbidden_pairs: &[(&str, &str)],
    ) -> Option<Vec<String>> {
        assert!(actions.len() <= MAX_EXACT_ACTIONS);
        let subset_count = 1_usize << actions.len();
        let mut best: Option<(OracleScore, Vec<String>)> = None;

        for mask in 0..subset_count {
            let selected: Vec<&ObservationAction> = actions
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1_usize << index) != 0)
                .map(|(_, action)| action)
                .collect();

            let incompatible = selected.iter().enumerate().any(|(left_index, left)| {
                selected.iter().skip(left_index + 1).any(|right| {
                    forbidden_pairs.iter().any(|(first, second)| {
                        (left.action_id == *first && right.action_id == *second)
                            || (left.action_id == *second && right.action_id == *first)
                    })
                })
            });
            if incompatible {
                continue;
            }

            let covered_regions: BTreeSet<&str> = selected
                .iter()
                .flat_map(|action| action.covered_region_ids.iter().map(String::as_str))
                .collect();
            if needs
                .iter()
                .any(|need| !covered_regions.contains(need.region_id.as_str()))
            {
                continue;
            }

            let crew_minutes = selected.iter().map(|action| action.crew_minutes).sum();
            let cost_minor_units = selected.iter().map(|action| action.cost_minor_units).sum();
            let energy_wh = selected.iter().map(|action| action.energy_wh).sum();
            let earliest_start = selected
                .iter()
                .map(|action| {
                    action.occupancy.reservation.interval_start_ms
                        - action.occupancy.reservation.clock_uncertainty_ms
                })
                .min()
                .unwrap_or(0);
            let latest_end = selected
                .iter()
                .map(|action| {
                    action.occupancy.reservation.interval_end_ms
                        + action.occupancy.reservation.clock_uncertainty_ms
                })
                .max()
                .unwrap_or(0);
            let mut action_ids: Vec<String> = selected
                .iter()
                .map(|action| action.action_id.clone())
                .collect();
            action_ids.sort();

            let score = OracleScore {
                crew_minutes,
                cost_minor_units,
                energy_wh,
                span_ms: latest_end - earliest_start,
                action_count: selected.len(),
                action_ids: action_ids.clone(),
            };
            if best.as_ref().is_none_or(|(current, _)| score < *current) {
                best = Some((score, action_ids));
            }
        }

        best.map(|(_, action_ids)| action_ids)
    }

    fn books(
        actions: &[ObservationAction],
        capacity: u32,
    ) -> (SpatialOccupancyBook, ReservationBook) {
        let occupancy =
            SpatialOccupancyBook::new(binding(), "site-clock-1").expect("occupancy book");
        let mut reservations =
            ReservationBook::new("site-epoch-1", "site-clock-1", 0).expect("reservation book");
        let resources: BTreeSet<&str> = actions
            .iter()
            .map(|action| action.occupancy.reservation.resource_id.as_str())
            .collect();
        for resource in resources {
            reservations
                .register_resource(resource, capacity)
                .expect("resource registration");
        }
        (occupancy, reservations)
    }

    #[test]
    fn exact_campaign_selects_two_complementary_actions() {
        let actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 2_000, "r-b", (0, 60_000), 6, 120),
        ];
        let needs = vec![need("unit-a", "a"), need("unit-b", "b")];
        let (occupancy, reservations) = books(&actions, 1);

        let result = plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 100)
            .expect("campaign search");
        assert!(result.search_complete);
        assert!(result.optimality_proven);
        assert!(!result.infeasibility_proven);
        assert!(result.best_campaign.complete);
        assert_eq!(
            result
                .best_campaign
                .selected_actions
                .iter()
                .map(|action| action.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["scan-a", "scan-b"]
        );
        assert_eq!(result.best_campaign.total_crew_minutes, 11);
        assert_eq!(result.best_campaign.total_cost_minor_units, 220);
        assert_eq!(
            result
                .best_campaign
                .selected_actions
                .iter()
                .map(|action| action.covered_work_unit_ids.len())
                .sum::<usize>(),
            2
        );
        assert_eq!(
            result.best_campaign.cost_currency_code.as_deref(),
            Some("USD")
        );
    }

    #[test]
    fn exact_search_beats_largest_coverage_greedy_counterexample() {
        let actions = vec![
            action(
                "large-expensive",
                &["a", "b", "c"],
                0,
                "r-large",
                (0, 60_000),
                10,
                900,
            ),
            action("cheap-ab", &["a", "b"], 2_000, "r-ab", (0, 60_000), 4, 100),
            action("cheap-cd", &["c", "d"], 4_000, "r-cd", (0, 60_000), 4, 100),
        ];
        let needs = vec![
            need("unit-a", "a"),
            need("unit-b", "b"),
            need("unit-c", "c"),
            need("unit-d", "d"),
        ];
        let (occupancy, reservations) = books(&actions, 1);
        let result =
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 1_000)
                .expect("campaign search");

        assert!(result.optimality_proven);
        assert_eq!(
            result
                .best_campaign
                .selected_actions
                .iter()
                .map(|action| action.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["cheap-a", "cheap-bc"]
        );
        assert_eq!(result.best_campaign.total_crew_minutes, 6);
        assert_eq!(result.best_campaign.total_cost_minor_units, 200);

        let greedy =
            plan_greedy_observation_campaign(&actions, &needs, &occupancy, &reservations, 0)
                .expect("greedy campaign");
        assert!(!greedy.optimality_proven);
        assert!(greedy.campaign.complete);
        assert_eq!(
            greedy
                .campaign
                .selected_actions
                .iter()
                .map(|action| action.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["large-expensive", "cheap-cd"]
        );
        assert_eq!(greedy.campaign.total_crew_minutes, 14);
        assert_eq!(greedy.campaign.total_cost_minor_units, 1_000);
    }

    #[test]
    fn exact_search_matches_independent_subset_oracle_for_frozen_campaign_fixture() {
        let actions = vec![
            action(
                "large-expensive",
                &["a", "b", "c"],
                0,
                "r-large",
                (0, 60_000),
                10,
                900,
            ),
            action("cheap-ab", &["a", "b"], 2_000, "r-ab", (0, 60_000), 4, 100),
            action("cheap-cd", &["c", "d"], 4_000, "r-cd", (0, 60_000), 4, 100),
        ];
        let needs = vec![
            need("unit-a", "a"),
            need("unit-b", "b"),
            need("unit-c", "c"),
            need("unit-d", "d"),
        ];
        let expected = independent_subset_oracle(&actions, &needs, &[])
            .expect("independent oracle finds complete subset");
        let (occupancy, reservations) = books(&actions, 1);
        let actual =
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 1_000)
                .expect("production exact search");

        assert!(actual.optimality_proven);
        assert_eq!(
            actual
                .best_campaign
                .selected_actions
                .iter()
                .map(|action| action.action_id.clone())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn independent_oracle_and_production_owner_agree_on_incompatible_pair() {
        let actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 250, "r-b", (0, 60_000), 5, 100),
        ];
        let needs = vec![need("unit-a", "a"), need("unit-b", "b")];
        let expected = independent_subset_oracle(&actions, &needs, &[("scan-a", "scan-b")]);
        let (occupancy, reservations) = books(&actions, 1);
        let actual =
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 1_000)
                .expect("production exact search");

        assert_eq!(expected, None);
        assert!(actual.infeasibility_proven);
    }

    #[test]
    fn pairwise_spatial_conflict_prevents_complete_campaign() {
        let actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 250, "r-b", (0, 60_000), 5, 100),
        ];
        let needs = vec![need("unit-a", "a"), need("unit-b", "b")];
        let (occupancy, reservations) = books(&actions, 1);
        let result =
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 1_000)
                .expect("campaign search");

        assert!(result.search_complete);
        assert!(result.infeasibility_proven);
        assert!(!result.best_campaign.complete);
        assert_eq!(result.best_campaign.covered_work_unit_ids.len(), 1);
        assert!(
            result
                .rejected_actions
                .iter()
                .any(|rejected| matches!(&rejected.blocker, SpatialError::PotentialConflict(_)))
        );
    }

    #[test]
    fn shared_resource_capacity_prevents_pairwise_campaign_admission() {
        let actions = vec![
            action("scan-a", &["a"], 0, "shared-hoist", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 2_000, "shared-hoist", (0, 60_000), 5, 100),
        ];
        let needs = vec![need("unit-a", "a"), need("unit-b", "b")];
        let (occupancy, reservations) = books(&actions, 1);
        let result =
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 1_000)
                .expect("campaign search");

        assert!(result.infeasibility_proven);
        assert!(!result.best_campaign.complete);
        assert!(
            result
                .rejected_actions
                .iter()
                .any(|rejected| matches!(&rejected.blocker, SpatialError::Reservation(_)))
        );
    }

    #[test]
    fn search_budget_exhaustion_is_not_reported_as_optimal_or_infeasible() {
        let actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 2_000, "r-b", 0, 60_000, 5, 100),
            action("scan-c", &["c"], 4_000, "r-c", (0, 60_000), 5, 100),
        ];
        let needs = vec![
            need("unit-a", "a"),
            need("unit-b", "b"),
            need("unit-c", "c"),
        ];
        let (occupancy, reservations) = books(&actions, 1);

        let result = plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 1)
            .expect("bounded search result");
        assert!(!result.search_complete);
        assert!(!result.optimality_proven);
        assert!(!result.infeasibility_proven);
    }

    #[test]
    fn planning_does_not_mutate_live_resource_or_occupancy_books() {
        let actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 2_000, "r-b", 0, 60_000, 5, 100),
        ];
        let needs = vec![need("unit-a", "a"), need("unit-b", "b")];
        let (occupancy, reservations) = books(&actions, 1);

        let result = plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 100)
            .expect("campaign search");
        assert!(result.best_campaign.complete);
        assert_eq!(occupancy.occupancy_count(), 0);
        assert_eq!(reservations.state("reservation-scan-a"), None);
        assert_eq!(reservations.state("reservation-scan-b"), None);
    }

    #[test]
    fn mixed_currencies_are_rejected_before_search() {
        let mut actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 2_000, "r-b", 0, 60_000, 5, 100),
        ];
        actions[1].cost_currency_code = "EUR".to_owned();
        let (occupancy, reservations) = books(&actions, 1);

        assert_eq!(
            plan_observation_campaign(
                &actions,
                &[need("unit-a", "a")],
                &occupancy,
                &reservations,
                0,
                100
            ),
            Err(PortfolioError::MixedCostCurrency {
                expected: "USD".to_owned(),
                received: "EUR".to_owned(),
            })
        );
    }

    #[test]
    fn duplicate_work_unit_identity_is_rejected() {
        let actions = vec![action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100)];
        let needs = vec![need("unit-a", "a"), need("unit-a", "a")];
        let (occupancy, reservations) = books(&actions, 1);

        assert_eq!(
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 100),
            Err(PortfolioError::DuplicateWorkUnitId("unit-a".to_owned()))
        );
    }

    #[test]
    fn non_iso_currency_shape_is_rejected() {
        let mut actions = vec![action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100)];
        actions[0].cost_currency_code = "US".to_owned();
        let (occupancy, reservations) = books(&actions, 1);

        assert_eq!(
            plan_observation_campaign(
                &actions,
                &[need("unit-a", "a")],
                &occupancy,
                &reservations,
                0,
                100
            ),
            Err(PortfolioError::InvalidCurrencyCode("US".to_owned()))
        );
    }

    #[test]
    fn aggregate_cost_overflow_is_a_typed_error() {
        let actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, u64::MAX),
            action("scan-b", &["b"], 2_000, "r-b", (0, 60_000), 5, 1),
        ];
        let needs = vec![need("unit-a", "a"), need("unit-b", "b")];
        let (occupancy, reservations) = books(&actions, 1);

        assert_eq!(
            plan_observation_campaign(&actions, &needs, &occupancy, &reservations, 0, 100),
            Err(PortfolioError::ArithmeticOverflow)
        );
    }

    #[test]
    fn empty_action_catalogue_proves_infeasibility_for_nonempty_needs() {
        let (occupancy, reservations) = books(&[], 1);
        let result = plan_observation_campaign(
            &[],
            &[need("unit-a", "a")],
            &occupancy,
            &reservations,
            0,
            10,
        )
        .expect("empty catalogue search");
        assert!(result.search_complete);
        assert!(result.infeasibility_proven);
        assert_eq!(result.best_campaign.uncovered_work_unit_ids, vec!["unit-a"]);
    }

    #[test]
    fn twenty_one_actions_are_rejected_by_the_exact_solver_bound() {
        let actions: Vec<ObservationAction> = (0..=MAX_EXACT_ACTIONS)
            .map(|index| {
                action(
                    &format!("scan-{index:02}"),
                    &["a"],
                    i64::try_from(index).expect("small index") * 2_000,
                    &format!("resource-{index:02}"),
                    (0, 60_000),
                    1,
                    1,
                )
            })
            .collect();
        let (occupancy, reservations) = books(&actions, 1);

        assert_eq!(
            plan_observation_campaign(
                &actions,
                &[need("unit-a", "a")],
                &occupancy,
                &reservations,
                0,
                100,
            ),
            Err(PortfolioError::TooManyActions {
                maximum: MAX_EXACT_ACTIONS,
                received: MAX_EXACT_ACTIONS + 1,
            })
        );
    }

    #[test]
    fn duplicate_occupancy_and_reservation_ids_are_rejected() {
        let mut actions = vec![
            action("scan-a", &["a"], 0, "r-a", (0, 60_000), 5, 100),
            action("scan-b", &["b"], 2_000, "r-b", 0, 60_000, 5, 100),
        ];
        actions[1].occupancy.occupancy_id = actions[0].occupancy.occupancy_id.clone();
        let (occupancy, reservations) = books(&actions, 1);
        assert_eq!(
            plan_observation_campaign(
                &actions,
                &[need("unit-a", "a")],
                &occupancy,
                &reservations,
                0,
                100
            ),
            Err(PortfolioError::DuplicateOccupancyId(
                "occupancy-scan-a".to_owned()
            ))
        );

        actions[1].occupancy.occupancy_id = "occupancy-scan-b".to_owned();
        actions[1].occupancy.reservation.reservation_id =
            actions[0].occupancy.reservation.reservation_id.clone();
        let (occupancy, reservations) = books(&actions, 1);
        assert_eq!(
            plan_observation_campaign(
                &actions,
                &[need("unit-a", "a")],
                &occupancy,
                &reservations,
                0,
                100
            ),
            Err(PortfolioError::DuplicateReservationId(
                "reservation-scan-a".to_owned()
            ))
        );
    }
}
