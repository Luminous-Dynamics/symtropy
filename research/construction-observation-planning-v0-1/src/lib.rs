//! Synthetic-first active observation planning and robust value-of-information oracle.
//!
//! This module selects declared observations against current work needs and the existing
//! conservative space-time occupancy index. It evaluates explicit scenario outcomes
//! without assigning scenario probabilities. A qualified clear observation only makes
//! downstream work ready for its separate acceptance process; it never marks work accepted.
//!
//! It is not a sensor model, survey adjustment engine, inspection authority, safety
//! controller, physical access permit or probability-calibrated risk model.

use std::collections::{BTreeMap, BTreeSet};

use construction_robot_interop_core::resources::{
    ReservationBook, ReservationError, ReservationOutcome,
};
use construction_spacetime_occupancy_core::{
    ConflictRecord, SpatialError, SpatialOccupancy, SpatialOccupancyBook,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkNeed {
    pub work_unit_id: String,
    pub region_id: String,
    /// Current conservative geometric uncertainty bound from the survey owner.
    /// This is not a probability or a sensor-quality estimate.
    pub uncertainty_bound_mm: [u64; 3],
}

impl WorkNeed {
    fn validate(&self) -> Result<(), ObservationPlanError> {
        require_non_empty("work_unit_id", &self.work_unit_id)?;
        require_non_empty("region_id", &self.region_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationAction {
    pub action_id: String,
    /// Opaque profile reference owned by the survey/observation authority.
    pub coverage_profile_ref: String,
    /// Opaque calibration record reference; validity must be established upstream.
    pub calibration_ref: String,
    /// Regions this declared action claims it can observe. This is not a coverage proof.
    pub covered_region_ids: Vec<String>,
    /// Access / instrument / work envelope bound to the current site and frame.
    pub occupancy: SpatialOccupancy,
    pub duration_minutes: u64,
    pub crew_minutes: u64,
    pub cost_minor_units: u64,
    pub energy_wh: u64,
}

impl ObservationAction {
    fn validate(&self) -> Result<(), ObservationPlanError> {
        require_non_empty("action_id", &self.action_id)?;
        require_non_empty("coverage_profile_ref", &self.coverage_profile_ref)?;
        require_non_empty("calibration_ref", &self.calibration_ref)?;
        if self.duration_minutes == 0 {
            return Err(ObservationPlanError::InvalidDuration);
        }
        if self.covered_region_ids.is_empty() {
            return Err(ObservationPlanError::NoCoveredRegions);
        }
        for region_id in &self.covered_region_ids {
            require_non_empty("covered_region_id", region_id)?;
        }
        require_non_empty("occupancy_id", &self.occupancy.occupancy_id)?;
        require_non_empty("occupancy_task_id", &self.occupancy.task_id)?;
        Ok(())
    }

    fn covered_regions(&self) -> BTreeSet<&str> {
        self.covered_region_ids.iter().map(String::as_str).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedCandidate {
    pub action_id: String,
    pub conflicts: Vec<ConflictRecord>,
    /// Exact resource-calendar rejection, if spatial admission passed but the
    /// existing reservation owner could not admit the required capacity.
    pub reservation_error: Option<ReservationError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationChoice {
    pub action_id: String,
    pub covered_work_unit_ids: Vec<String>,
    pub duration_minutes: u64,
    pub crew_minutes: u64,
    pub cost_minor_units: u64,
    pub energy_wh: u64,
    /// None for the geometry-only baseline; otherwise the read-only result from
    /// the existing resource owner. Duplicate means the identical booking is active.
    pub reservation_outcome: Option<ReservationOutcome>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetedPlan {
    pub selected: Option<ObservationChoice>,
    pub rejected_conflicts: Vec<RejectedCandidate>,
    /// Valid, accessible actions that cover no currently declared work need.
    pub irrelevant_action_ids: Vec<String>,
}

/// Fixed-checklist baseline: evaluate exactly the named action and do not substitute
/// a different action when it conflicts or covers no current work need. This baseline
/// checks spatial conflicts only; use `choose_fixed_observation_with_resources` for
/// resource-capacity-aware planning.
pub fn choose_fixed_observation(
    action_id: &str,
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
) -> Result<TargetedPlan, ObservationPlanError> {
    require_non_empty("fixed_action_id", action_id)?;
    let action = find_unique_action(action_id, actions)?;
    choose_targeted_observation(std::slice::from_ref(action), work_needs, occupancy_book)
}

pub fn choose_fixed_observation_with_resources(
    action_id: &str,
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
    reservation_book: &ReservationBook,
    now_ms: u64,
) -> Result<TargetedPlan, ObservationPlanError> {
    require_non_empty("fixed_action_id", action_id)?;
    let action = find_unique_action(action_id, actions)?;
    choose_targeted_observation_with_resources(
        std::slice::from_ref(action),
        work_needs,
        occupancy_book,
        reservation_book,
        now_ms,
    )
}

fn find_unique_action<'a>(
    action_id: &str,
    actions: &'a [ObservationAction],
) -> Result<&'a ObservationAction, ObservationPlanError> {
    let mut matches = actions.iter().filter(|action| action.action_id == action_id);
    let action = matches
        .next()
        .ok_or_else(|| ObservationPlanError::UnknownAction(action_id.to_owned()))?;
    if matches.next().is_some() {
        return Err(ObservationPlanError::DuplicateAction(action_id.to_owned()));
    }
    Ok(action)
}

/// Threshold-triggered baseline: only work needs whose declared uncertainty bound
/// exceeds the configured per-axis threshold are treated as observation targets.
/// The threshold is a benchmark policy input, not a safety or measurement tolerance.
pub fn choose_threshold_observation(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    threshold_mm: [u64; 3],
    occupancy_book: &SpatialOccupancyBook,
) -> Result<TargetedPlan, ObservationPlanError> {
    let mut seen_work_units = BTreeSet::new();
    for need in work_needs {
        need.validate()?;
        if !seen_work_units.insert(need.work_unit_id.as_str()) {
            return Err(ObservationPlanError::DuplicateWorkUnit(
                need.work_unit_id.clone(),
            ));
        }
    }

    let triggered_needs: Vec<WorkNeed> = work_needs
        .iter()
        .filter(|need| {
            need.uncertainty_bound_mm
                .iter()
                .zip(threshold_mm.iter())
                .any(|(uncertainty, threshold)| uncertainty > threshold)
        })
        .cloned()
        .collect();

    choose_targeted_observation(actions, &triggered_needs, occupancy_book)
}

/// Resource-aware threshold baseline. Every need is validated; only above-threshold
/// needs become observation targets. Resource previews are read-only and use the
/// same current clock value for every candidate.
pub fn choose_threshold_observation_with_resources(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    threshold_mm: [u64; 3],
    occupancy_book: &SpatialOccupancyBook,
    reservation_book: &ReservationBook,
    now_ms: u64,
) -> Result<TargetedPlan, ObservationPlanError> {
    let mut seen_work_units = BTreeSet::new();
    for need in work_needs {
        need.validate()?;
        if !seen_work_units.insert(need.work_unit_id.as_str()) {
            return Err(ObservationPlanError::DuplicateWorkUnit(
                need.work_unit_id.clone(),
            ));
        }
    }
    let triggered_needs: Vec<WorkNeed> = work_needs
        .iter()
        .filter(|need| {
            need.uncertainty_bound_mm
                .iter()
                .zip(threshold_mm.iter())
                .any(|(uncertainty, threshold)| uncertainty > threshold)
        })
        .cloned()
        .collect();

    choose_targeted_observation_with_resources(
        actions,
        &triggered_needs,
        occupancy_book,
        reservation_book,
        now_ms,
    )
}

/// Select one observation without access to hidden scenario truth.
///
/// Policy is deterministic and lexicographic:
/// 1. cover the greatest number of currently blocked work units;
/// 2. minimize crew minutes;
/// 3. minimize declared observation cost;
/// 4. minimize elapsed observation duration;
/// 5. break ties by action ID.
///
/// Spatial conflicts are returned, not silently ignored. Invalid context bindings
/// propagate as typed errors. No probabilities or hidden-state labels enter selection.
pub fn choose_targeted_observation(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
) -> Result<TargetedPlan, ObservationPlanError> {
    choose_targeted_observation_inner(actions, work_needs, occupancy_book, None, 0)
}

/// Select a targeted observation using both existing spatial occupancy and the
/// existing time/capacity reservation owner. Resource feasibility is previewed
/// without mutating the reservation book; only an explicit later commit may reserve.
pub fn choose_targeted_observation_with_resources(
    actions: &[ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
    reservation_book: &ReservationBook,
    now_ms: u64,
) -> Result<TargetedPlan, ObservationPlanError> {
    choose_targeted_observation_inner(
        actions,
        work_needs,
        occupancy_book,
        Some(reservation_book),
        now_ms,
    )
}

fn choose_targeted_observation_inner<'a>(
    actions: &'a [ObservationAction],
    work_needs: &[WorkNeed],
    occupancy_book: &SpatialOccupancyBook,
    reservation_book: Option<&ReservationBook>,
    now_ms: u64,
) -> Result<TargetedPlan, ObservationPlanError> {
    let mut seen_actions = BTreeSet::new();
    let mut seen_work = BTreeSet::new();

    for need in work_needs {
        need.validate()?;
        if !seen_work.insert(need.work_unit_id.as_str()) {
            return Err(ObservationPlanError::DuplicateWorkUnit(
                need.work_unit_id.clone(),
            ));
        }
    }

    let mut candidates: Vec<(&ObservationAction, Vec<String>, Option<ReservationOutcome>)> =
        Vec::new();
    let mut rejected_conflicts = Vec::new();
    let mut irrelevant_action_ids = Vec::new();

    for action in actions {
        action.validate()?;
        if !seen_actions.insert(action.action_id.as_str()) {
            return Err(ObservationPlanError::DuplicateAction(
                action.action_id.clone(),
            ));
        }

        let covered_regions = action.covered_regions();
        let mut covered_work_unit_ids: Vec<String> = work_needs
            .iter()
            .filter(|need| covered_regions.contains(need.region_id.as_str()))
            .map(|need| need.work_unit_id.clone())
            .collect();
        covered_work_unit_ids.sort();

        if covered_work_unit_ids.is_empty() {
            irrelevant_action_ids.push(action.action_id.clone());
            continue;
        }

        let conflicts = occupancy_book
            .conflicts_for(&action.occupancy)
            .map_err(ObservationPlanError::Spatial)?;
        if !conflicts.is_empty() {
            rejected_conflicts.push(RejectedCandidate {
                action_id: action.action_id.clone(),
                conflicts,
                reservation_error: None,
            });
            continue;
        }

        let reservation_outcome = if let Some(reservation_book) = reservation_book {
            match reservation_book.preview_reservation(&action.occupancy.reservation, now_ms) {
                Ok(outcome) => Some(outcome),
                Err(error) => {
                    rejected_conflicts.push(RejectedCandidate {
                        action_id: action.action_id.clone(),
                        conflicts: Vec::new(),
                        reservation_error: Some(error),
                    });
                    continue;
                }
            }
        } else {
            None
        };

        candidates.push((action, covered_work_unit_ids, reservation_outcome));
    }

    candidates.sort_by(|(left, left_coverage, _), (right, right_coverage, _)| {
        right_coverage
            .len()
            .cmp(&left_coverage.len())
            .then_with(|| left.crew_minutes.cmp(&right.crew_minutes))
            .then_with(|| left.cost_minor_units.cmp(&right.cost_minor_units))
            .then_with(|| left.duration_minutes.cmp(&right.duration_minutes))
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    let selected = candidates
        .first()
        .map(|(action, covered, reservation_outcome)| ObservationChoice {
            action_id: action.action_id.clone(),
            covered_work_unit_ids: covered.clone(),
            duration_minutes: action.duration_minutes,
            crew_minutes: action.crew_minutes,
            cost_minor_units: action.cost_minor_units,
            energy_wh: action.energy_wh,
            reservation_outcome: *reservation_outcome,
        });

    rejected_conflicts.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    irrelevant_action_ids.sort();

    Ok(TargetedPlan {
        selected,
        rejected_conflicts,
        irrelevant_action_ids,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationOutcome {
    /// A qualified measurement bounded to a declared coverage and uncertainty claim.
    QualifiedClear {
        evidence_ref: String,
        coverage_receipt_ref: String,
        uncertainty_report_ref: String,
    },
    /// A qualified observation found an issue; professional / site disposition is required.
    Finding {
        evidence_ref: String,
        finding_ref: String,
    },
    /// Coverage or quality was insufficient. This is unknown, never clear.
    Incomplete {
        evidence_ref: Option<String>,
        reason: String,
    },
    /// The evidence lost validity (for example, site epoch / frame / calibration changed).
    Invalidated {
        reason: String,
    },
    /// The action could not be performed or its result was not delivered.
    Unavailable {
        reason: String,
    },
}

impl ObservationOutcome {
    fn validate(&self) -> Result<(), ObservationPlanError> {
        match self {
            Self::QualifiedClear {
                evidence_ref,
                coverage_receipt_ref,
                uncertainty_report_ref,
            } => {
                require_non_empty("evidence_ref", evidence_ref)?;
                require_non_empty("coverage_receipt_ref", coverage_receipt_ref)?;
                require_non_empty("uncertainty_report_ref", uncertainty_report_ref)
            }
            Self::Finding {
                evidence_ref,
                finding_ref,
            } => {
                require_non_empty("evidence_ref", evidence_ref)?;
                require_non_empty("finding_ref", finding_ref)
            }
            Self::Incomplete {
                evidence_ref,
                reason,
            } => {
                if let Some(evidence_ref) = evidence_ref {
                    require_non_empty("evidence_ref", evidence_ref)?;
                }
                require_non_empty("reason", reason)
            }
            Self::Invalidated { reason } | Self::Unavailable { reason } => {
                require_non_empty("reason", reason)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkDisposition {
    /// A separate downstream acceptance process may now be considered.
    ReadyForIndependentAcceptance,
    /// A positive finding requires accountable professional / site disposition.
    RequiresProfessionalDisposition,
    /// Missing, incomplete, stale or unavailable information keeps the work held.
    HeldUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioActionResult {
    pub outcome: ObservationOutcome,
    /// Scenario-specific residual loss after the declared outcome and response.
    /// This is an explicit benchmark input, not a probability or model estimate.
    pub residual_loss_minor_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioOracle {
    pub scenario_id: String,
    /// Hidden truth identity belongs to the independent test/evaluation oracle.
    /// It is never passed to the observation-selection policy.
    pub hidden_state_ref: String,
    /// Loss of the safe no-observation policy (for example, held-work delay).
    pub no_observation_loss_minor_units: u64,
    pub results_by_action: BTreeMap<String, ScenarioActionResult>,
}

impl ScenarioOracle {
    fn validate(&self) -> Result<(), ObservationPlanError> {
        require_non_empty("scenario_id", &self.scenario_id)?;
        require_non_empty("hidden_state_ref", &self.hidden_state_ref)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationEvaluation {
    pub scenario_id: String,
    pub action_id: Option<String>,
    pub outcome: Option<ObservationOutcome>,
    pub work_disposition: WorkDisposition,
    pub no_observation_loss_minor_units: u64,
    pub observation_cost_minor_units: u64,
    pub observation_duration_minutes: u64,
    pub observation_crew_minutes: u64,
    pub observation_energy_wh: u64,
    pub residual_loss_minor_units: u64,
    /// no-observation loss - (observation cost + residual loss).
    /// Positive is beneficial in this scenario; negative is harmful.
    pub net_voi_minor_units: i128,
}

/// Evaluate the safe baseline. Without qualified evidence, dependent work remains held.
pub fn evaluate_no_observation(
    scenario: &ScenarioOracle,
) -> Result<ObservationEvaluation, ObservationPlanError> {
    scenario.validate()?;
    Ok(ObservationEvaluation {
        scenario_id: scenario.scenario_id.clone(),
        action_id: None,
        outcome: None,
        work_disposition: WorkDisposition::HeldUnknown,
        no_observation_loss_minor_units: scenario.no_observation_loss_minor_units,
        observation_cost_minor_units: 0,
        observation_duration_minutes: 0,
        observation_crew_minutes: 0,
        observation_energy_wh: 0,
        residual_loss_minor_units: scenario.no_observation_loss_minor_units,
        net_voi_minor_units: 0,
    })
}

/// Evaluate one selected observation under one explicit hidden-state scenario.
/// This is scenario-wise VOI; no probability distribution is assumed or fabricated.
pub fn evaluate_observation(
    action: &ObservationAction,
    scenario: &ScenarioOracle,
) -> Result<ObservationEvaluation, ObservationPlanError> {
    action.validate()?;
    scenario.validate()?;

    let result = scenario
        .results_by_action
        .get(&action.action_id)
        .ok_or_else(|| {
            ObservationPlanError::MissingScenarioActionResult(action.action_id.clone())
        })?;
    result.outcome.validate()?;

    let work_disposition = match &result.outcome {
        ObservationOutcome::QualifiedClear { .. } => {
            WorkDisposition::ReadyForIndependentAcceptance
        }
        ObservationOutcome::Finding { .. } => {
            WorkDisposition::RequiresProfessionalDisposition
        }
        ObservationOutcome::Incomplete { .. }
        | ObservationOutcome::Invalidated { .. }
        | ObservationOutcome::Unavailable { .. } => WorkDisposition::HeldUnknown,
    };

    let total_cost = i128::from(action.cost_minor_units)
        + i128::from(result.residual_loss_minor_units);
    let net_voi_minor_units = i128::from(scenario.no_observation_loss_minor_units) - total_cost;

    Ok(ObservationEvaluation {
        scenario_id: scenario.scenario_id.clone(),
        action_id: Some(action.action_id.clone()),
        outcome: Some(result.outcome.clone()),
        work_disposition,
        no_observation_loss_minor_units: scenario.no_observation_loss_minor_units,
        observation_cost_minor_units: action.cost_minor_units,
        observation_duration_minutes: action.duration_minutes,
        observation_crew_minutes: action.crew_minutes,
        observation_energy_wh: action.energy_wh,
        residual_loss_minor_units: result.residual_loss_minor_units,
        net_voi_minor_units,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationPlanError {
    EmptyField(&'static str),
    InvalidDuration,
    NoCoveredRegions,
    DuplicateAction(String),
    DuplicateWorkUnit(String),
    UnknownAction(String),
    MissingScenarioActionResult(String),
    Spatial(SpatialError),
}

fn require_non_empty(
    name: &'static str,
    value: &str,
) -> Result<(), ObservationPlanError> {
    if value.trim().is_empty() {
        return Err(ObservationPlanError::EmptyField(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use construction_spacetime_occupancy_core::{
        Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancyBook,
    };
    use construction_spacetime_occupancy_core::SpatialOccupancy;
    use construction_robot_interop_core::resources::{
        ReservationBook, ReservationOutcome, ReservationRequest, ReservationError,
    };

    fn binding() -> SiteBinding {
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

    fn reservation(id: &str, task: &str, start: u64, end: u64) -> ReservationRequest {
        ReservationRequest {
            reservation_id: format!("reservation-{id}"),
            resource_id: "survey-resource".to_owned(),
            task_id: task.to_owned(),
            owner_id: format!("owner-{task}"),
            site_epoch: "site-epoch-1".to_owned(),
            clock_domain_id: "site-clock-1".to_owned(),
            clock_uncertainty_ms: 0,
            interval_start_ms: start,
            interval_end_ms: end,
            expires_at_ms: end.saturating_add(100),
            units: 1,
        }
    }

    fn occupancy(
        id: &str,
        task: &str,
        bounds: Bounds3Mm,
        start: u64,
        end: u64,
    ) -> SpatialOccupancy {
        SpatialOccupancy {
            occupancy_id: id.to_owned(),
            task_id: task.to_owned(),
            kind: OccupancyKind::InspectionAccess,
            binding: binding(),
            bounds,
            uncertainty_margin_mm: [0, 0, 0],
            clearance_margin_mm: [0, 0, 0],
            reservation: reservation(id, task, start, end),
        }
    }

    fn action(
        id: &str,
        regions: &[&str],
        area: Bounds3Mm,
        start: u64,
        end: u64,
        crew_minutes: u64,
        cost_minor_units: u64,
    ) -> ObservationAction {
        ObservationAction {
            action_id: id.to_owned(),
            coverage_profile_ref: format!("coverage-profile:{id}"),
            calibration_ref: format!("calibration:{id}"),
            covered_region_ids: regions.iter().map(|region| (*region).to_owned()).collect(),
            occupancy: occupancy(id, &format!("task-{id}"), area, start, end),
            duration_minutes: 10,
            crew_minutes,
            cost_minor_units,
            energy_wh: 50,
        }
    }

    fn book() -> SpatialOccupancyBook {
        SpatialOccupancyBook::new(binding(), "site-clock-1").expect("valid occupancy book")
    }

    fn resource_book(resources: &[(&str, u32)]) -> ReservationBook {
        let mut book =
            ReservationBook::new("site-epoch-1", "site-clock-1", 0).expect("valid resource book");
        for (resource_id, capacity) in resources {
            book.register_resource(*resource_id, *capacity)
                .expect("register resource");
        }
        book
    }

    fn qualified_clear() -> ObservationOutcome {
        ObservationOutcome::QualifiedClear {
            evidence_ref: "evidence:scan-01".to_owned(),
            coverage_receipt_ref: "coverage-receipt:scan-01".to_owned(),
            uncertainty_report_ref: "uncertainty-report:scan-01".to_owned(),
        }
    }

    fn scenario(
        action_id: &str,
        outcome: ObservationOutcome,
        no_observation_loss: u64,
        residual_loss: u64,
    ) -> ScenarioOracle {
        ScenarioOracle {
            scenario_id: "hidden-embed-offset-01".to_owned(),
            hidden_state_ref: "oracle-only:hidden-embed-offset-01".to_owned(),
            no_observation_loss_minor_units: no_observation_loss,
            results_by_action: BTreeMap::from([(
                action_id.to_owned(),
                ScenarioActionResult {
                    outcome,
                    residual_loss_minor_units: residual_loss,
                },
            )]),
        }
    }

    #[test]
    fn targeted_policy_selects_most_covered_feasible_action() {
        let actions = vec![
            action(
                "survey-a",
                &["region-a", "region-b"],
                bounds([0, 0, 0], [1_000, 1_000, 1_000]),
                0,
                100,
                20,
                300,
            ),
            action(
                "survey-b",
                &["region-a", "region-b", "region-c"],
                bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
                0,
                100,
                25,
                400,
            ),
            action(
                "survey-irrelevant",
                &["region-z"],
                bounds([6_000, 0, 0], [7_000, 1_000, 1_000]),
                0,
                100,
                5,
                100,
            ),
        ];
        let needs = vec![
            WorkNeed {
                work_unit_id: "rebar-a".to_owned(),
                region_id: "region-a".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
            WorkNeed {
                work_unit_id: "embed-b".to_owned(),
                region_id: "region-b".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
            WorkNeed {
                work_unit_id: "mep-c".to_owned(),
                region_id: "region-c".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
        ];

        let plan = choose_targeted_observation(&actions, &needs, &book())
            .expect("targeted observation plan");
        let selected = plan.selected.expect("a feasible action is selected");
        assert_eq!(selected.action_id, "survey-b");
        assert_eq!(
            selected.covered_work_unit_ids,
            vec!["embed-b", "mep-c", "rebar-a"]
        );
        assert_eq!(plan.irrelevant_action_ids, vec!["survey-irrelevant"]);
        assert!(plan.rejected_conflicts.is_empty());
    }

    #[test]
    fn fixed_checklist_baseline_uses_only_the_named_action() {
        let actions = vec![
            action(
                "fixed-survey",
                &["region-a"],
                bounds([0, 0, 0], [1_000, 1_000, 1_000]),
                0,
                100,
                10,
                200,
            ),
            action(
                "better-targeted-survey",
                &["region-a", "region-b"],
                bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
                0,
                100,
                10,
                200,
            ),
        ];
        let needs = vec![
            WorkNeed {
                work_unit_id: "unit-a".to_owned(),
                region_id: "region-a".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
            WorkNeed {
                work_unit_id: "unit-b".to_owned(),
                region_id: "region-b".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
        ];

        let plan = choose_fixed_observation("fixed-survey", &actions, &needs, &book())
            .expect("fixed plan");
        assert_eq!(
            plan.selected.expect("fixed action selected").action_id,
            "fixed-survey"
        );
    }

    #[test]
    fn fixed_checklist_does_not_silently_substitute_after_conflict() {
        let mut index = book();
        index
            .insert(occupancy(
                "active-workface",
                "task-active",
                bounds([0, 0, 0], [1_000, 1_000, 1_000]),
                0,
                100,
            ))
            .expect("active workface");
        let actions = vec![
            action(
                "fixed-survey",
                &["region-a"],
                bounds([500, 500, 0], [1_500, 1_500, 1_000]),
                50,
                150,
                5,
                100,
            ),
            action(
                "alternate-survey",
                &["region-a"],
                bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
                50,
                150,
                10,
                150,
            ),
        ];
        let needs = vec![WorkNeed {
            work_unit_id: "unit-a".to_owned(),
            region_id: "region-a".to_owned(),
            uncertainty_bound_mm: [100, 100, 100],
        }];

        let plan = choose_fixed_observation("fixed-survey", &actions, &needs, &index)
            .expect("fixed baseline");
        assert!(plan.selected.is_none());
        assert_eq!(plan.rejected_conflicts.len(), 1);
        assert_eq!(plan.rejected_conflicts[0].action_id, "fixed-survey");
    }

    #[test]
    fn threshold_policy_targets_only_regions_above_declared_uncertainty_bounds() {
        let actions = vec![
            action(
                "scan-high-uncertainty",
                &["region-high"],
                bounds([0, 0, 0], [1_000, 1_000, 1_000]),
                0,
                100,
                10,
                200,
            ),
            action(
                "scan-low-uncertainty",
                &["region-low"],
                bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
                0,
                100,
                1,
                10,
            ),
        ];
        let needs = vec![
            WorkNeed {
                work_unit_id: "high-unit".to_owned(),
                region_id: "region-high".to_owned(),
                uncertainty_bound_mm: [250, 20, 10],
            },
            WorkNeed {
                work_unit_id: "low-unit".to_owned(),
                region_id: "region-low".to_owned(),
                uncertainty_bound_mm: [20, 20, 10],
            },
        ];

        let plan = choose_threshold_observation(
            &actions,
            &needs,
            [100, 100, 100],
            &book(),
        )
        .expect("threshold policy");
        let selected = plan.selected.expect("high uncertainty scan selected");
        assert_eq!(selected.action_id, "scan-high-uncertainty");
        assert_eq!(selected.covered_work_unit_ids, vec!["high-unit"]);
        assert_eq!(
            plan.irrelevant_action_ids,
            vec!["scan-low-uncertainty"]
        );
    }

    #[test]
    fn threshold_policy_with_no_triggered_need_performs_no_observation() {
        let actions = vec![action(
            "scan-low",
            &["region-low"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            100,
        )];
        let needs = vec![WorkNeed {
            work_unit_id: "low-unit".to_owned(),
            region_id: "region-low".to_owned(),
            uncertainty_bound_mm: [10, 10, 10],
        }];
        let plan =
            choose_threshold_observation(&actions, &needs, [100, 100, 100], &book())
                .expect("threshold plan");
        assert!(plan.selected.is_none());
        assert_eq!(plan.irrelevant_action_ids, vec!["scan-low"]);
    }

    #[test]
    fn spatial_conflict_excludes_observation_action_and_is_reported() {
        let mut index = book();
        index
            .insert(occupancy(
                "active-workface",
                "task-active",
                bounds([0, 0, 0], [1_000, 1_000, 1_000]),
                0,
                100,
            ))
            .expect("active workface");

        let blocked = action(
            "blocked-survey",
            &["region-a"],
            bounds([500, 500, 0], [1_500, 1_500, 1_000]),
            50,
            150,
            5,
            100,
        );
        let feasible = action(
            "remote-survey",
            &["region-a"],
            bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
            50,
            150,
            10,
            150,
        );
        let needs = vec![WorkNeed {
            work_unit_id: "unit-a".to_owned(),
            region_id: "region-a".to_owned(),
            uncertainty_bound_mm: [100, 100, 100],
        }];

        let plan = choose_targeted_observation(&[blocked, feasible], &needs, &index)
            .expect("observation plan");
        assert_eq!(
            plan.selected.expect("remote survey selected").action_id,
            "remote-survey"
        );
        assert_eq!(plan.rejected_conflicts.len(), 1);
        assert_eq!(
            plan.rejected_conflicts[0].action_id,
            "blocked-survey"
        );
        assert_eq!(plan.rejected_conflicts[0].conflicts.len(), 1);
    }

    #[test]
    fn resource_aware_policy_rejects_busy_hoist_without_mutation_and_selects_alternative() {
        let mut resources = resource_book(&[("survey-resource", 1), ("survey-resource-alt", 1)]);
        resources
            .reserve(
                reservation("existing-hoist", "existing-task", 50, 150),
                0,
            )
            .expect("book the shared survey resource");
        let before = resources.clone();

        let blocked = action(
            "survey-high-coverage",
            &["region-a", "region-b"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            50,
            150,
            1,
            10,
        );
        let mut alternative = action(
            "survey-alternative",
            &["region-a"],
            bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
            50,
            150,
            10,
            30,
        );
        alternative.occupancy.reservation.resource_id = "survey-resource-alt".to_owned();

        let needs = vec![
            WorkNeed {
                work_unit_id: "unit-a".to_owned(),
                region_id: "region-a".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
            WorkNeed {
                work_unit_id: "unit-b".to_owned(),
                region_id: "region-b".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            },
        ];

        let plan = choose_targeted_observation_with_resources(
            &[blocked, alternative],
            &needs,
            &book(),
            &resources,
            0,
        )
        .expect("resource-aware observation plan");
        let selected = plan.selected.expect("alternative remains feasible");
        assert_eq!(selected.action_id, "survey-alternative");
        assert_eq!(selected.reservation_outcome, Some(ReservationOutcome::Granted));
        assert_eq!(plan.rejected_conflicts.len(), 1);
        assert!(plan.rejected_conflicts[0].conflicts.is_empty());
        assert_eq!(
            plan.rejected_conflicts[0].reservation_error,
            Some(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2,
            })
        );
        // A plan preview does not reserve the selected action or mutate any other row.
        assert_eq!(resources, before);
    }

    #[test]
    fn resource_aware_fixed_policy_reports_capacity_rejection_without_substitution() {
        let mut resources = resource_book(&[("survey-resource", 1)]);
        resources
            .reserve(
                reservation("busy-hoist", "existing-task", 50, 150),
                0,
            )
            .expect("existing hoist booking");
        let before = resources.clone();
        let actions = vec![
            action(
                "fixed-scan",
                &["region-a"],
                bounds([0, 0, 0], [1_000, 1_000, 1_000]),
                50,
                150,
                5,
                100,
            ),
            action(
                "alternate-scan",
                &["region-a"],
                bounds([3_000, 0, 0], [4_000, 1_000, 1_000]),
                50,
                150,
                10,
                150,
            ),
        ];
        let needs = vec![WorkNeed {
            work_unit_id: "unit-a".to_owned(),
            region_id: "region-a".to_owned(),
            uncertainty_bound_mm: [100, 100, 100],
        }];

        let plan = choose_fixed_observation_with_resources(
            "fixed-scan",
            &actions,
            &needs,
            &book(),
            &resources,
            0,
        )
        .expect("fixed resource-aware baseline");
        assert!(plan.selected.is_none());
        assert_eq!(plan.rejected_conflicts.len(), 1);
        assert_eq!(plan.rejected_conflicts[0].action_id, "fixed-scan");
        assert_eq!(
            plan.rejected_conflicts[0].reservation_error,
            Some(ReservationError::CapacityExceeded {
                capacity: 1,
                requested_total: 2,
            })
        );
        assert_eq!(resources, before);
    }

    #[test]
    fn resource_aware_policy_ignores_due_bookings_without_expiring_them() {
        let mut resources = resource_book(&[("survey-resource", 1)]);
        resources
            .reserve(
                reservation("expired-in-time", "past-task", 50, 150),
                0,
            )
            .expect("past reservation initially active");
        let before = resources.clone();
        let scan = action(
            "scan-after-expiry",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            300,
            400,
            5,
            100,
        );
        let needs = vec![WorkNeed {
            work_unit_id: "unit-a".to_owned(),
            region_id: "region-a".to_owned(),
            uncertainty_bound_mm: [100, 100, 100],
        }];

        let plan = choose_targeted_observation_with_resources(
            &[scan],
            &needs,
            &book(),
            &resources,
            250,
        )
        .expect("expired reservation no longer consumes capacity");
        assert_eq!(
            plan.selected
                .expect("scan selected")
                .reservation_outcome,
            Some(ReservationOutcome::Granted)
        );
        assert_eq!(resources, before);
        assert_eq!(
            resources.state("reservation-expired-in-time"),
            Some(construction_robot_interop_core::resources::ReservationState::Active)
        );
    }

    #[test]
    fn qualified_clear_only_releases_work_to_independent_acceptance() {
        let scan = action(
            "scan-clear",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );
        let result = evaluate_observation(
            &scan,
            &scenario("scan-clear", qualified_clear(), 2_000, 100),
        )
        .expect("observation evaluation");

        assert_eq!(
            result.work_disposition,
            WorkDisposition::ReadyForIndependentAcceptance
        );
        assert_eq!(result.net_voi_minor_units, 1_650);
        assert_eq!(result.action_id.as_deref(), Some("scan-clear"));
        assert_eq!(result.observation_duration_minutes, 10);
        assert_eq!(result.observation_crew_minutes, 5);
        assert_eq!(result.observation_energy_wh, 50);
    }

    #[test]
    fn finding_requires_disposition_and_never_marks_work_accepted() {
        let scan = action(
            "scan-finding",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );
        let result = evaluate_observation(
            &scan,
            &scenario(
                "scan-finding",
                ObservationOutcome::Finding {
                    evidence_ref: "evidence:scan-finding".to_owned(),
                    finding_ref: "finding:embed-offset".to_owned(),
                },
                2_000,
                500,
            ),
        )
        .expect("finding evaluation");

        assert_eq!(
            result.work_disposition,
            WorkDisposition::RequiresProfessionalDisposition
        );
        assert_eq!(result.net_voi_minor_units, 1_250);
    }

    #[test]
    fn incomplete_invalidated_and_unavailable_results_remain_unknown() {
        let scan = action(
            "scan-unknown",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );
        let outcomes = vec![
            ObservationOutcome::Incomplete {
                evidence_ref: Some("partial:scan".to_owned()),
                reason: "occluded region".to_owned(),
            },
            ObservationOutcome::Invalidated {
                reason: "site epoch changed during observation".to_owned(),
            },
            ObservationOutcome::Unavailable {
                reason: "instrument did not return data".to_owned(),
            },
        ];

        for outcome in outcomes {
            let result = evaluate_observation(
                &scan,
                &scenario("scan-unknown", outcome, 2_000, 2_000),
            )
            .expect("unknown outcome evaluation");
            assert_eq!(result.work_disposition, WorkDisposition::HeldUnknown);
            assert_eq!(result.net_voi_minor_units, -250);
        }
    }

    #[test]
    fn no_observation_baseline_never_releases_unknown_work() {
        let base = scenario(
            "scan-unused",
            ObservationOutcome::Unavailable {
                reason: "not performed".to_owned(),
            },
            1_000,
            1_000,
        );
        let result = evaluate_no_observation(&base).expect("no-observation baseline");

        assert_eq!(result.work_disposition, WorkDisposition::HeldUnknown);
        assert_eq!(result.action_id, None);
        assert_eq!(result.net_voi_minor_units, 0);
        assert_eq!(result.residual_loss_minor_units, 1_000);
        assert_eq!(result.observation_duration_minutes, 0);
        assert_eq!(result.observation_crew_minutes, 0);
        assert_eq!(result.observation_energy_wh, 0);
    }

    #[test]
    fn scenario_wise_voi_can_be_positive_or_negative_without_probabilities() {
        let scan = action(
            "scan-voi",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );

        let positive = evaluate_observation(
            &scan,
            &scenario("scan-voi", qualified_clear(), 2_000, 100),
        )
        .expect("positive scenario");
        let negative = evaluate_observation(
            &scan,
            &scenario(
                "scan-voi",
                ObservationOutcome::Incomplete {
                    evidence_ref: None,
                    reason: "coverage failed".to_owned(),
                },
                2_000,
                2_000,
            ),
        )
        .expect("negative scenario");

        assert_eq!(positive.net_voi_minor_units, 1_650);
        assert_eq!(negative.net_voi_minor_units, -250);
    }

    #[test]
    fn missing_scenario_result_is_typed_not_interpreted_as_clear() {
        let scan = action(
            "scan-missing",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );
        let base = ScenarioOracle {
            scenario_id: "scenario-1".to_owned(),
            hidden_state_ref: "oracle-only:scenario-1".to_owned(),
            no_observation_loss_minor_units: 1_000,
            results_by_action: BTreeMap::new(),
        };
        assert_eq!(
            evaluate_observation(&scan, &base),
            Err(ObservationPlanError::MissingScenarioActionResult(
                "scan-missing".to_owned()
            ))
        );
    }

    #[test]
    fn duplicate_action_and_work_ids_are_rejected() {
        let first = action(
            "same-action",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );
        let second = first.clone();
        let need = WorkNeed {
            work_unit_id: "unit-a".to_owned(),
            region_id: "region-a".to_owned(),
            uncertainty_bound_mm: [100, 100, 100],
        };

        assert_eq!(
            choose_targeted_observation(&[first.clone(), second], &[need.clone()], &book()),
            Err(ObservationPlanError::DuplicateAction(
                "same-action".to_owned()
            ))
        );
        assert_eq!(
            choose_targeted_observation(&[first], &[need.clone(), need], &book()),
            Err(ObservationPlanError::DuplicateWorkUnit(
                "unit-a".to_owned()
            ))
        );
    }

    #[test]
    fn stale_frame_is_rejected_by_existing_occupancy_owner() {
        let mut stale = action(
            "stale-frame",
            &["region-a"],
            bounds([0, 0, 0], [1_000, 1_000, 1_000]),
            0,
            100,
            5,
            250,
        );
        stale.occupancy.binding.target_frame_id = "old-local-frame".to_owned();

        let error = choose_targeted_observation(
            &[stale],
            &[WorkNeed {
                work_unit_id: "unit-a".to_owned(),
                region_id: "region-a".to_owned(),
                uncertainty_bound_mm: [100, 100, 100],
            }],
            &book(),
        )
        .expect_err("stale frame fails closed");
        assert_eq!(
            error,
            ObservationPlanError::Spatial(SpatialError::FrameMismatch)
        );
    }
}
