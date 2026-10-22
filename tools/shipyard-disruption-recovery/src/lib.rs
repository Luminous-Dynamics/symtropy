//! Evaluate post-disruption feasibility, recourse, and schedule churn.
//!
//! This crate evaluates a supplied recovery plan; it does not search for one. A disturbance is
//! applied to baseline and candidate plans using the same time windows, exclusion zones, and
//! vehicle outages. Candidate feasibility is recalculated by the independent space-time verifier.
//!
//! Reservations completed at or before the disruption time are immutable. Feasibility, changes to
//! affected work, churn imposed on unaffected work, and recovery horizon are separate metrics.

use std::collections::{BTreeMap, BTreeSet};

use shipyard_space_time::{
    ExclusionZone, Footprint, Placement, Scenario, TimeRange, Transfer, Violation, ViolationCode,
    verify,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VehicleOutage {
    pub vehicle_id: String,
    pub unavailable: TimeRange,
}

/// Observed evidence that a planned placement footprint remains occupied longer than scheduled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementOccupancyExtension {
    pub reservation_id: String,
    /// Authoritative earliest tick at which the block's footprint is confirmed to be clear.
    pub occupied_until_tick: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Disruption {
    /// First tick at which the disturbance is known to the planner.
    pub observed_at_tick: u64,
    /// New zones which become authoritative at this disruption.
    pub new_exclusion_zones: Vec<ExclusionZone>,
    /// Vehicle unavailability intervals.
    pub vehicle_outages: Vec<VehicleOutage>,
    /// Observed delayed-clearance events; applied to both old and candidate plans.
    pub placement_occupancy_extensions: Vec<PlacementOccupancyExtension>,
    /// Known work that requires reconsideration; other affected work is derived automatically.
    pub explicitly_affected_reservation_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RecoveryViolationCode {
    BaselineNotFeasibleBeforeDisruption,
    CandidateNotFeasibleAfterDisruption,
    EmptyVehicleOutageId,
    InvalidVehicleOutageInterval,
    DuplicateVehicleOutage,
    DuplicateAffectedReservationId,
    UnknownAffectedReservationId,
    EmptyOccupancyExtensionId,
    DuplicateOccupancyExtension,
    UnknownOccupancyExtensionReservation,
    InvalidOccupancyExtension,
    OccupancyExtensionAfterCompletion,
    OccupancyExtensionTargetMismatch,
    HistoricalPlacementChanged,
    HistoricalTransferChanged,
    VehicleUnavailable,
}

impl RecoveryViolationCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BaselineNotFeasibleBeforeDisruption => "BaselineNotFeasibleBeforeDisruption",
            Self::CandidateNotFeasibleAfterDisruption => "CandidateNotFeasibleAfterDisruption",
            Self::EmptyVehicleOutageId => "EmptyVehicleOutageId",
            Self::InvalidVehicleOutageInterval => "InvalidVehicleOutageInterval",
            Self::DuplicateVehicleOutage => "DuplicateVehicleOutage",
            Self::DuplicateAffectedReservationId => "DuplicateAffectedReservationId",
            Self::UnknownAffectedReservationId => "UnknownAffectedReservationId",
            Self::EmptyOccupancyExtensionId => "EmptyOccupancyExtensionId",
            Self::DuplicateOccupancyExtension => "DuplicateOccupancyExtension",
            Self::UnknownOccupancyExtensionReservation => "UnknownOccupancyExtensionReservation",
            Self::InvalidOccupancyExtension => "InvalidOccupancyExtension",
            Self::OccupancyExtensionAfterCompletion => "OccupancyExtensionAfterCompletion",
            Self::OccupancyExtensionTargetMismatch => "OccupancyExtensionTargetMismatch",
            Self::HistoricalPlacementChanged => "HistoricalPlacementChanged",
            Self::HistoricalTransferChanged => "HistoricalTransferChanged",
            Self::VehicleUnavailable => "VehicleUnavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryViolation {
    pub code: RecoveryViolationCode,
    pub subject_ids: Vec<String>,
    /// Underlying independent space-time violation when applicable.
    pub spatial_code: Option<ViolationCode>,
}

fn push_violation(
    violations: &mut Vec<RecoveryViolation>,
    code: RecoveryViolationCode,
    mut subject_ids: Vec<String>,
    spatial_code: Option<ViolationCode>,
) {
    subject_ids.sort();
    violations.push(RecoveryViolation {
        code,
        subject_ids,
        spatial_code,
    });
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecoveryMetrics {
    pub baseline_feasible_before_disruption: bool,
    pub baseline_feasible_after_disruption: bool,
    pub candidate_feasible_after_disruption: bool,
    /// Placement reservation IDs added, removed, or changed.
    pub changed_placement_reservations: usize,
    /// Transfer reservation IDs added, removed, or changed.
    pub changed_transfer_reservations: usize,
    /// Changes among reservations outside the declared/derived affected set.
    pub unaffected_placement_churn: usize,
    pub unaffected_transfer_churn: usize,
    /// Sum of absolute start/end-tick changes for matching reservation IDs.
    pub time_shift_ticks: u64,
    /// Sum of absolute min/max x/y edge shifts for matching rectangles.
    pub rectangle_edge_shift_units: u64,
    pub affected_reservation_count: usize,
    pub candidate_makespan_tick: Option<u64>,
    pub recovery_completion_tick: Option<u64>,
    /// Recovery completion relative to the tick when the disruption was observed.
    pub recovery_horizon_ticks: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryReport {
    pub accepted: bool,
    pub metrics: RecoveryMetrics,
    pub violations: Vec<RecoveryViolation>,
    pub affected_reservation_ids: Vec<String>,
    pub candidate_spatial_violations: Vec<Violation>,
    pub baseline_spatial_violations: Vec<Violation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryInput {
    /// Feasible schedule immediately before the disruption.
    pub baseline: Scenario,
    /// Proposed replacement plan before this evaluator applies new disturbance evidence.
    pub candidate: Scenario,
    pub disruption: Disruption,
}

/// One plausible disturbance trajectory for a deterministic robust-envelope evaluation.
///
/// Scenarios are not assigned probabilities in V0: every supplied trajectory is a hard
/// feasibility case, not a statistical forecast.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryEnvelopeScenario {
    pub scenario_id: String,
    pub disruption: Disruption,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RecoveryEnvelopeViolationCode {
    EmptyScenarioSet,
    EmptyScenarioId,
    DuplicateScenarioId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryEnvelopeViolation {
    pub code: RecoveryEnvelopeViolationCode,
    pub scenario_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryEnvelopeScenarioReport {
    pub scenario_id: String,
    pub report: RecoveryReport,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecoveryEnvelopeMetrics {
    pub total_scenarios: usize,
    pub accepted_scenarios: usize,
    pub worst_case_recovery_horizon_ticks: Option<u64>,
    pub worst_case_makespan_tick: Option<u64>,
    pub worst_case_changed_placement_reservations: usize,
    pub worst_case_changed_transfer_reservations: usize,
    pub worst_case_unaffected_placement_churn: usize,
    pub worst_case_unaffected_transfer_churn: usize,
    pub worst_case_time_shift_ticks: u64,
    pub worst_case_rectangle_edge_shift_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryEnvelopeReport {
    /// True only when the scenario set is valid and every candidate report is accepted.
    pub robustly_accepted: bool,
    pub metrics: RecoveryEnvelopeMetrics,
    pub violations: Vec<RecoveryEnvelopeViolation>,
    pub scenarios: Vec<RecoveryEnvelopeScenarioReport>,
}

/// A candidate schedule and its finite robust-envelope assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryEnvelopeAlternative {
    pub candidate_id: String,
    pub report: RecoveryEnvelopeReport,
}

/// Maximize transparency by comparing each objective independently; no hidden weights are used.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecoveryParetoScore {
    pub worst_case_recovery_horizon_ticks: Option<u64>,
    pub worst_case_makespan_tick: Option<u64>,
    pub worst_case_changed_placement_reservations: usize,
    pub worst_case_changed_transfer_reservations: usize,
    pub worst_case_unaffected_placement_churn: usize,
    pub worst_case_unaffected_transfer_churn: usize,
    pub worst_case_time_shift_ticks: u64,
    pub worst_case_rectangle_edge_shift_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryParetoCandidate {
    pub candidate_id: String,
    pub score: RecoveryParetoScore,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RecoveryParetoViolationCode {
    EmptyCandidateId,
    DuplicateCandidateId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryParetoViolation {
    pub code: RecoveryParetoViolationCode,
    pub candidate_id: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecoveryParetoReport {
    /// Candidates that are not dominated on all published objectives.
    pub frontier: Vec<RecoveryParetoCandidate>,
    /// Robustly feasible candidates dominated by another robustly feasible candidate.
    pub dominated_candidate_ids: Vec<String>,
    /// Candidates that failed at least one scenario in their finite envelope.
    pub rejected_candidate_ids: Vec<String>,
    pub violations: Vec<RecoveryParetoViolation>,
}

fn pareto_score(report: &RecoveryEnvelopeReport) -> RecoveryParetoScore {
    RecoveryParetoScore {
        worst_case_recovery_horizon_ticks: report.metrics.worst_case_recovery_horizon_ticks,
        worst_case_makespan_tick: report.metrics.worst_case_makespan_tick,
        worst_case_changed_placement_reservations: report
            .metrics
            .worst_case_changed_placement_reservations,
        worst_case_changed_transfer_reservations: report
            .metrics
            .worst_case_changed_transfer_reservations,
        worst_case_unaffected_placement_churn: report
            .metrics
            .worst_case_unaffected_placement_churn,
        worst_case_unaffected_transfer_churn: report
            .metrics
            .worst_case_unaffected_transfer_churn,
        worst_case_time_shift_ticks: report.metrics.worst_case_time_shift_ticks,
        worst_case_rectangle_edge_shift_units: report
            .metrics
            .worst_case_rectangle_edge_shift_units,
    }
}

fn dominates(left: &RecoveryParetoScore, right: &RecoveryParetoScore) -> bool {
    let no_worse_everywhere = left.worst_case_recovery_horizon_ticks
        <= right.worst_case_recovery_horizon_ticks
        && left.worst_case_makespan_tick <= right.worst_case_makespan_tick
        && left.worst_case_changed_placement_reservations
            <= right.worst_case_changed_placement_reservations
        && left.worst_case_changed_transfer_reservations
            <= right.worst_case_changed_transfer_reservations
        && left.worst_case_unaffected_placement_churn
            <= right.worst_case_unaffected_placement_churn
        && left.worst_case_unaffected_transfer_churn
            <= right.worst_case_unaffected_transfer_churn
        && left.worst_case_time_shift_ticks <= right.worst_case_time_shift_ticks
        && left.worst_case_rectangle_edge_shift_units
            <= right.worst_case_rectangle_edge_shift_units;
    let strictly_better_somewhere = left.worst_case_recovery_horizon_ticks
        < right.worst_case_recovery_horizon_ticks
        || left.worst_case_makespan_tick < right.worst_case_makespan_tick
        || left.worst_case_changed_placement_reservations
            < right.worst_case_changed_placement_reservations
        || left.worst_case_changed_transfer_reservations
            < right.worst_case_changed_transfer_reservations
        || left.worst_case_unaffected_placement_churn
            < right.worst_case_unaffected_placement_churn
        || left.worst_case_unaffected_transfer_churn
            < right.worst_case_unaffected_transfer_churn
        || left.worst_case_time_shift_ticks < right.worst_case_time_shift_ticks
        || left.worst_case_rectangle_edge_shift_units
            < right.worst_case_rectangle_edge_shift_units;
    no_worse_everywhere && strictly_better_somewhere
}

/// Compute a Pareto frontier over robustly accepted candidates, without a weighted scalar.
///
/// A candidate failing any disturbance case is placed in rejected_candidate_ids and cannot
/// dominate a robustly feasible plan. Candidate IDs must be unique/non-empty; ambiguous identity
/// invalidates the comparison. None for recovery horizon/makespan sorts before Some, meaning
/// the metric is not applicable (for example, no changed/affected reservations).
pub fn pareto_recovery_candidates(
    alternatives: &[RecoveryEnvelopeAlternative],
) -> RecoveryParetoReport {
    let mut report = RecoveryParetoReport::default();
    let mut seen = BTreeSet::new();

    for alternative in alternatives {
        if alternative.candidate_id.trim().is_empty() {
            report.violations.push(RecoveryParetoViolation {
                code: RecoveryParetoViolationCode::EmptyCandidateId,
                candidate_id: alternative.candidate_id.clone(),
            });
        } else if !seen.insert(alternative.candidate_id.as_str()) {
            report.violations.push(RecoveryParetoViolation {
                code: RecoveryParetoViolationCode::DuplicateCandidateId,
                candidate_id: alternative.candidate_id.clone(),
            });
        }
    }

    if !report.violations.is_empty() {
        report.violations.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        report.violations.dedup();
        return report;
    }

    let mut eligible: Vec<RecoveryParetoCandidate> = Vec::new();
    for alternative in alternatives {
        if alternative.report.robustly_accepted {
            eligible.push(RecoveryParetoCandidate {
                candidate_id: alternative.candidate_id.clone(),
                score: pareto_score(&alternative.report),
            });
        } else {
            report
                .rejected_candidate_ids
                .push(alternative.candidate_id.clone());
        }
    }
    eligible.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    for candidate in &eligible {
        let is_dominated = eligible.iter().any(|other| {
            other.candidate_id != candidate.candidate_id
                && dominates(&other.score, &candidate.score)
        });
        if is_dominated {
            report
                .dominated_candidate_ids
                .push(candidate.candidate_id.clone());
        } else {
            report.frontier.push(candidate.clone());
        }
    }
    report.dominated_candidate_ids.sort();
    report.rejected_candidate_ids.sort();
    report
}

/// Evaluate one candidate against every supplied plausible disturbance trajectory.
///
/// This is a finite scenario envelope, not a probability guarantee over unlisted scenarios. Every
/// scenario is hard: the candidate is robustly accepted only when every underlying recovery report
/// is accepted. Scenario IDs are required to be non-empty and unique so reports remain auditable.
pub fn evaluate_recovery_envelope(
    baseline: &Scenario,
    candidate: &Scenario,
    scenarios: &[RecoveryEnvelopeScenario],
) -> RecoveryEnvelopeReport {
    let mut violations = Vec::new();
    if scenarios.is_empty() {
        violations.push(RecoveryEnvelopeViolation {
            code: RecoveryEnvelopeViolationCode::EmptyScenarioSet,
            scenario_id: String::new(),
        });
    }

    let mut seen = BTreeSet::new();
    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() {
            violations.push(RecoveryEnvelopeViolation {
                code: RecoveryEnvelopeViolationCode::EmptyScenarioId,
                scenario_id: scenario.scenario_id.clone(),
            });
        } else if !seen.insert(scenario.scenario_id.as_str()) {
            violations.push(RecoveryEnvelopeViolation {
                code: RecoveryEnvelopeViolationCode::DuplicateScenarioId,
                scenario_id: scenario.scenario_id.clone(),
            });
        }
    }

    let mut scenario_reports: Vec<RecoveryEnvelopeScenarioReport> = scenarios
        .iter()
        .map(|scenario| RecoveryEnvelopeScenarioReport {
            scenario_id: scenario.scenario_id.clone(),
            report: evaluate_recovery(&RecoveryInput {
                baseline: baseline.clone(),
                candidate: candidate.clone(),
                disruption: scenario.disruption.clone(),
            }),
        })
        .collect();
    scenario_reports.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let accepted_scenarios = scenario_reports
        .iter()
        .filter(|scenario| scenario.report.accepted)
        .count();
    let worst_case_recovery_horizon_ticks = scenario_reports
        .iter()
        .filter_map(|scenario| scenario.report.metrics.recovery_horizon_ticks)
        .max();
    let worst_case_makespan_tick = scenario_reports
        .iter()
        .filter_map(|scenario| scenario.report.metrics.candidate_makespan_tick)
        .max();
    let worst_case_changed_placement_reservations = scenario_reports
        .iter()
        .map(|scenario| scenario.report.metrics.changed_placement_reservations)
        .max()
        .unwrap_or_default();
    let worst_case_changed_transfer_reservations = scenario_reports
        .iter()
        .map(|scenario| scenario.report.metrics.changed_transfer_reservations)
        .max()
        .unwrap_or_default();
    let worst_case_unaffected_placement_churn = scenario_reports
        .iter()
        .map(|scenario| scenario.report.metrics.unaffected_placement_churn)
        .max()
        .unwrap_or_default();
    let worst_case_unaffected_transfer_churn = scenario_reports
        .iter()
        .map(|scenario| scenario.report.metrics.unaffected_transfer_churn)
        .max()
        .unwrap_or_default();
    let worst_case_time_shift_ticks = scenario_reports
        .iter()
        .map(|scenario| scenario.report.metrics.time_shift_ticks)
        .max()
        .unwrap_or_default();
    let worst_case_rectangle_edge_shift_units = scenario_reports
        .iter()
        .map(|scenario| scenario.report.metrics.rectangle_edge_shift_units)
        .max()
        .unwrap_or_default();

    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });
    violations.dedup();

    let robustly_accepted = scenarios.len() > 0
        && violations.is_empty()
        && accepted_scenarios == scenarios.len();

    RecoveryEnvelopeReport {
        robustly_accepted,
        metrics: RecoveryEnvelopeMetrics {
            total_scenarios: scenarios.len(),
            accepted_scenarios,
            worst_case_recovery_horizon_ticks,
            worst_case_makespan_tick,
            worst_case_changed_placement_reservations,
            worst_case_changed_transfer_reservations,
            worst_case_unaffected_placement_churn,
            worst_case_unaffected_transfer_churn,
            worst_case_time_shift_ticks,
            worst_case_rectangle_edge_shift_units,
        },
        violations,
        scenarios: scenario_reports,
    }
}

fn apply_disruption(scenario: &Scenario, disruption: &Disruption) -> Scenario {
    let mut effective = scenario.clone();
    effective
        .exclusion_zones
        .extend(disruption.new_exclusion_zones.iter().cloned());

    for extension in &disruption.placement_occupancy_extensions {
        for placement in &mut effective.placements {
            if placement.reservation_id == extension.reservation_id {
                placement.occupied.end_tick =
                    placement.occupied.end_tick.max(extension.occupied_until_tick);
            }
        }
    }

    effective
}

fn overlaps_rect(left: &Footprint, right: &Footprint) -> bool {
    match (left, right) {
        (Footprint::AxisAlignedRect(left), Footprint::AxisAlignedRect(right))
            if left.is_valid() && right.is_valid() =>
        {
            left.overlaps(*right)
        }
        _ => false,
    }
}

fn outage_violations(scenario: &Scenario, outages: &[VehicleOutage]) -> Vec<RecoveryViolation> {
    let mut result = Vec::new();
    for transfer in &scenario.transfers {
        for outage in outages {
            if transfer.vehicle_id == outage.vehicle_id
                && transfer.interval.is_valid()
                && outage.unavailable.is_valid()
                && transfer.interval.overlaps(outage.unavailable)
            {
                push_violation(
                    &mut result,
                    RecoveryViolationCode::VehicleUnavailable,
                    vec![transfer.id.clone(), outage.vehicle_id.clone()],
                    None,
                );
            }
        }
    }
    result
}

fn valid_occupancy_extensions(
    baseline: &Scenario,
    candidate: &Scenario,
    disruption: &Disruption,
    violations: &mut Vec<RecoveryViolation>,
) -> bool {
    let mut valid = true;
    let mut seen = BTreeSet::new();

    for extension in &disruption.placement_occupancy_extensions {
        if extension.reservation_id.trim().is_empty() {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::EmptyOccupancyExtensionId,
                vec![extension.reservation_id.clone()],
                None,
            );
            continue;
        }
        if !seen.insert(extension.reservation_id.as_str()) {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::DuplicateOccupancyExtension,
                vec![extension.reservation_id.clone()],
                None,
            );
            continue;
        }

        let matches: Vec<&Placement> = baseline
            .placements
            .iter()
            .filter(|placement| placement.reservation_id == extension.reservation_id)
            .collect();
        let [placement] = matches.as_slice() else {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::UnknownOccupancyExtensionReservation,
                vec![extension.reservation_id.clone()],
                None,
            );
            continue;
        };

        if placement.occupied.end_tick <= disruption.observed_at_tick {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::OccupancyExtensionAfterCompletion,
                vec![extension.reservation_id.clone()],
                None,
            );
            continue;
        }

        if extension.occupied_until_tick <= placement.occupied.end_tick
            || extension.occupied_until_tick <= disruption.observed_at_tick
        {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::InvalidOccupancyExtension,
                vec![extension.reservation_id.clone()],
                None,
            );
        }

        let candidate_matches: Vec<&Placement> = candidate
            .placements
            .iter()
            .filter(|candidate_placement| {
                candidate_placement.reservation_id == extension.reservation_id
            })
            .collect();
        let [candidate_placement] = candidate_matches.as_slice() else {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::OccupancyExtensionTargetMismatch,
                vec![extension.reservation_id.clone()],
                None,
            );
            continue;
        };
        if candidate_placement.block_id != placement.block_id
            || candidate_placement.footprint != placement.footprint
            || candidate_placement.occupied.start_tick != placement.occupied.start_tick
            || candidate_placement.occupied.end_tick < placement.occupied.end_tick
        {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::OccupancyExtensionTargetMismatch,
                vec![extension.reservation_id.clone()],
                None,
            );
        }
    }

    valid
}

fn valid_outages(outages: &[VehicleOutage], violations: &mut Vec<RecoveryViolation>) -> bool {
    let mut valid = true;
    let mut seen = BTreeSet::new();
    for outage in outages {
        if outage.vehicle_id.trim().is_empty() {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::EmptyVehicleOutageId,
                vec![outage.vehicle_id.clone()],
                None,
            );
        }
        if !outage.unavailable.is_valid() {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::InvalidVehicleOutageInterval,
                vec![outage.vehicle_id.clone()],
                None,
            );
        }
        if !seen.insert((outage.vehicle_id.as_str(), outage.unavailable)) {
            valid = false;
            push_violation(
                violations,
                RecoveryViolationCode::DuplicateVehicleOutage,
                vec![outage.vehicle_id.clone()],
                None,
            );
        }
    }
    valid
}

fn placement_map(scenario: &Scenario) -> BTreeMap<&str, &Placement> {
    scenario
        .placements
        .iter()
        .map(|placement| (placement.reservation_id.as_str(), placement))
        .collect()
}

fn transfer_map(scenario: &Scenario) -> BTreeMap<&str, &Transfer> {
    scenario
        .transfers
        .iter()
        .map(|transfer| (transfer.id.as_str(), transfer))
        .collect()
}

fn all_reservation_ids(scenario: &Scenario) -> BTreeSet<String> {
    scenario
        .placements
        .iter()
        .map(|placement| placement.reservation_id.clone())
        .chain(scenario.transfers.iter().map(|transfer| transfer.id.clone()))
        .collect()
}

fn derive_affected_ids(
    baseline: &Scenario,
    disruption: &Disruption,
    violations: &mut Vec<RecoveryViolation>,
) -> BTreeSet<String> {
    let baseline_ids = all_reservation_ids(baseline);
    let mut affected = BTreeSet::new();
    let mut explicit_seen = BTreeSet::new();

    for id in &disruption.explicitly_affected_reservation_ids {
        if !explicit_seen.insert(id.as_str()) {
            push_violation(
                violations,
                RecoveryViolationCode::DuplicateAffectedReservationId,
                vec![id.clone()],
                None,
            );
        }
        if !baseline_ids.contains(id) {
            push_violation(
                violations,
                RecoveryViolationCode::UnknownAffectedReservationId,
                vec![id.clone()],
                None,
            );
        }
        affected.insert(id.clone());
    }

    for placement in &baseline.placements {
        if disruption.new_exclusion_zones.iter().any(|zone| {
            placement.occupied.overlaps(zone.active)
                && overlaps_rect(&placement.footprint, &zone.footprint)
        }) {
            affected.insert(placement.reservation_id.clone());
        }
    }

    for extension in &disruption.placement_occupancy_extensions {
        let Some(outgoing) = baseline
            .placements
            .iter()
            .find(|placement| placement.reservation_id == extension.reservation_id)
        else {
            continue;
        };
        if extension.occupied_until_tick <= outgoing.occupied.end_tick {
            continue;
        }

        affected.insert(outgoing.reservation_id.clone());
        let extended_window = TimeRange {
            start_tick: outgoing.occupied.end_tick,
            end_tick: extension.occupied_until_tick,
        };

        for other in &baseline.placements {
            if other.reservation_id != outgoing.reservation_id
                && other.occupied.overlaps(extended_window)
                && overlaps_rect(&other.footprint, &outgoing.footprint)
            {
                affected.insert(other.reservation_id.clone());
            }
        }

        for transfer in &baseline.transfers {
            if transfer.interval.overlaps(extended_window)
                && overlaps_rect(&transfer.corridor, &outgoing.footprint)
            {
                affected.insert(transfer.id.clone());
            }
        }
    }

    for transfer in &baseline.transfers {
        let corridor_blocked = disruption.new_exclusion_zones.iter().any(|zone| {
            transfer.interval.overlaps(zone.active)
                && overlaps_rect(&transfer.corridor, &zone.footprint)
        });
        let vehicle_unavailable = disruption.vehicle_outages.iter().any(|outage| {
            transfer.vehicle_id == outage.vehicle_id
                && transfer.interval.overlaps(outage.unavailable)
        });
        if corridor_blocked || vehicle_unavailable {
            affected.insert(transfer.id.clone());
        }
    }

    affected
}

fn accumulate_change_metrics(
    baseline: &Scenario,
    candidate: &Scenario,
    metrics: &mut RecoveryMetrics,
) {
    let old_placements = placement_map(baseline);
    let new_placements = placement_map(candidate);
    let placement_ids: BTreeSet<&str> = old_placements
        .keys()
        .copied()
        .chain(new_placements.keys().copied())
        .collect();

    for id in placement_ids {
        match (old_placements.get(id), new_placements.get(id)) {
            (Some(old), Some(new)) if old != new => {
                metrics.changed_placement_reservations += 1;
                metrics.time_shift_ticks = metrics
                    .time_shift_ticks
                    .saturating_add(old.occupied.start_tick.abs_diff(new.occupied.start_tick))
                    .saturating_add(old.occupied.end_tick.abs_diff(new.occupied.end_tick));
                if let (
                    Footprint::AxisAlignedRect(old_rect),
                    Footprint::AxisAlignedRect(new_rect),
                ) = (&old.footprint, &new.footprint)
                {
                    metrics.rectangle_edge_shift_units = metrics
                        .rectangle_edge_shift_units
                        .saturating_add(old_rect.min_x.abs_diff(new_rect.min_x))
                        .saturating_add(old_rect.min_y.abs_diff(new_rect.min_y))
                        .saturating_add(old_rect.max_x.abs_diff(new_rect.max_x))
                        .saturating_add(old_rect.max_y.abs_diff(new_rect.max_y));
                }
            }
            (None, Some(_)) | (Some(_), None) => {
                metrics.changed_placement_reservations += 1;
            }
            _ => {}
        }
    }

    let old_transfers = transfer_map(baseline);
    let new_transfers = transfer_map(candidate);
    let transfer_ids: BTreeSet<&str> = old_transfers
        .keys()
        .copied()
        .chain(new_transfers.keys().copied())
        .collect();

    for id in transfer_ids {
        match (old_transfers.get(id), new_transfers.get(id)) {
            (Some(old), Some(new)) if old != new => {
                metrics.changed_transfer_reservations += 1;
                metrics.time_shift_ticks = metrics
                    .time_shift_ticks
                    .saturating_add(old.interval.start_tick.abs_diff(new.interval.start_tick))
                    .saturating_add(old.interval.end_tick.abs_diff(new.interval.end_tick));
            }
            (None, Some(_)) | (Some(_), None) => {
                metrics.changed_transfer_reservations += 1;
            }
            _ => {}
        }
    }
}

fn add_churn_metrics(
    baseline: &Scenario,
    candidate: &Scenario,
    affected: &BTreeSet<String>,
    metrics: &mut RecoveryMetrics,
) {
    let old_placements = placement_map(baseline);
    let new_placements = placement_map(candidate);
    let placement_ids: BTreeSet<&str> = old_placements
        .keys()
        .copied()
        .chain(new_placements.keys().copied())
        .collect();

    for id in placement_ids {
        if old_placements.get(id) != new_placements.get(id) && !affected.contains(id) {
            metrics.unaffected_placement_churn += 1;
        }
    }

    let old_transfers = transfer_map(baseline);
    let new_transfers = transfer_map(candidate);
    let transfer_ids: BTreeSet<&str> = old_transfers
        .keys()
        .copied()
        .chain(new_transfers.keys().copied())
        .collect();

    for id in transfer_ids {
        if old_transfers.get(id) != new_transfers.get(id) && !affected.contains(id) {
            metrics.unaffected_transfer_churn += 1;
        }
    }
}

fn historical_mutations(
    baseline: &Scenario,
    candidate: &Scenario,
    disruption_tick: u64,
    violations: &mut Vec<RecoveryViolation>,
) {
    let old_placements = placement_map(baseline);
    let new_placements = placement_map(candidate);
    for (id, old) in old_placements {
        if old.occupied.end_tick <= disruption_tick && new_placements.get(id) != Some(&old) {
            push_violation(
                violations,
                RecoveryViolationCode::HistoricalPlacementChanged,
                vec![id.to_owned()],
                None,
            );
        }
    }

    let old_transfers = transfer_map(baseline);
    let new_transfers = transfer_map(candidate);
    for (id, old) in old_transfers {
        if old.interval.end_tick <= disruption_tick && new_transfers.get(id) != Some(&old) {
            push_violation(
                violations,
                RecoveryViolationCode::HistoricalTransferChanged,
                vec![id.to_owned()],
                None,
            );
        }
    }
}

fn makespan(scenario: &Scenario) -> Option<u64> {
    scenario
        .placements
        .iter()
        .map(|placement| placement.occupied.end_tick)
        .chain(
            scenario
                .transfers
                .iter()
                .map(|transfer| transfer.interval.end_tick),
        )
        .max()
}

fn recovery_completion(candidate: &Scenario, changed_ids: &BTreeSet<String>) -> Option<u64> {
    candidate
        .placements
        .iter()
        .filter(|placement| changed_ids.contains(&placement.reservation_id))
        .map(|placement| placement.occupied.end_tick)
        .chain(
            candidate
                .transfers
                .iter()
                .filter(|transfer| changed_ids.contains(&transfer.id))
                .map(|transfer| transfer.interval.end_tick),
        )
        .max()
}

/// Evaluate a candidate recovery against the old schedule and explicit disturbance set.
///
/// Both plans receive the same new exclusion zones and vehicle outages. Acceptance requires the
/// baseline to be feasible before the disturbance, the candidate to be feasible afterwards, no
/// unavailable vehicle use, and no mutation to reservations already completed at disruption time.
pub fn evaluate_recovery(input: &RecoveryInput) -> RecoveryReport {
    let mut violations = Vec::new();
    let baseline_before_report = verify(&input.baseline);
    let baseline_feasible_before_disruption = baseline_before_report.is_feasible();
    if !baseline_feasible_before_disruption {
        push_violation(
            &mut violations,
            RecoveryViolationCode::BaselineNotFeasibleBeforeDisruption,
            Vec::new(),
            None,
        );
    }

    let outages_valid = valid_outages(&input.disruption.vehicle_outages, &mut violations);
    let occupancy_extensions_valid = valid_occupancy_extensions(
        &input.baseline,
        &input.candidate,
        &input.disruption,
        &mut violations,
    );
    let affected = derive_affected_ids(&input.baseline, &input.disruption, &mut violations);
    let baseline_after = apply_disruption(&input.baseline, &input.disruption);
    let candidate_after = apply_disruption(&input.candidate, &input.disruption);

    let baseline_spatial_report = verify(&baseline_after);
    let baseline_outage_violations =
        outage_violations(&baseline_after, &input.disruption.vehicle_outages);
    let baseline_feasible_after_disruption = baseline_spatial_report.is_feasible()
        && baseline_outage_violations.is_empty()
        && outages_valid
        && occupancy_extensions_valid;

    let candidate_spatial_report = verify(&candidate_after);
    let candidate_outage_violations =
        outage_violations(&candidate_after, &input.disruption.vehicle_outages);
    let candidate_feasible_after_disruption = candidate_spatial_report.is_feasible()
        && candidate_outage_violations.is_empty()
        && outages_valid
        && occupancy_extensions_valid;

    if !candidate_feasible_after_disruption {
        for spatial in &candidate_spatial_report.violations {
            push_violation(
                &mut violations,
                RecoveryViolationCode::CandidateNotFeasibleAfterDisruption,
                spatial.subject_ids.clone(),
                Some(spatial.code),
            );
        }
        violations.extend(candidate_outage_violations.iter().cloned());
    }

    historical_mutations(
        &input.baseline,
        &input.candidate,
        input.disruption.observed_at_tick,
        &mut violations,
    );

    let baseline_ids = all_reservation_ids(&input.baseline);
    let candidate_ids = all_reservation_ids(&input.candidate);
    for id in &affected {
        if !baseline_ids.contains(id) && !candidate_ids.contains(id) {
            push_violation(
                &mut violations,
                RecoveryViolationCode::UnknownAffectedReservationId,
                vec![id.clone()],
                None,
            );
        }
    }

    let mut metrics = RecoveryMetrics {
        baseline_feasible_before_disruption,
        baseline_feasible_after_disruption,
        candidate_feasible_after_disruption,
        affected_reservation_count: affected.len(),
        candidate_makespan_tick: makespan(&candidate_after),
        ..RecoveryMetrics::default()
    };

    accumulate_change_metrics(&input.baseline, &input.candidate, &mut metrics);
    add_churn_metrics(&input.baseline, &input.candidate, &affected, &mut metrics);

    let mut changed_ids = affected.clone();
    let old_placements = placement_map(&input.baseline);
    let new_placements = placement_map(&input.candidate);
    for id in old_placements
        .keys()
        .chain(new_placements.keys())
        .copied()
    {
        if old_placements.get(id) != new_placements.get(id) {
            changed_ids.insert(id.to_owned());
        }
    }
    let old_transfers = transfer_map(&input.baseline);
    let new_transfers = transfer_map(&input.candidate);
    for id in old_transfers.keys().chain(new_transfers.keys()).copied() {
        if old_transfers.get(id) != new_transfers.get(id) {
            changed_ids.insert(id.to_owned());
        }
    }

    metrics.recovery_completion_tick = recovery_completion(&candidate_after, &changed_ids);
    metrics.recovery_horizon_ticks = metrics
        .recovery_completion_tick
        .map(|tick| tick.saturating_sub(input.disruption.observed_at_tick));

    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject_ids.cmp(&right.subject_ids))
            .then_with(|| left.spatial_code.cmp(&right.spatial_code))
    });
    violations.dedup();

    let accepted = baseline_feasible_before_disruption
        && candidate_feasible_after_disruption
        && violations.is_empty();

    RecoveryReport {
        accepted,
        metrics,
        violations,
        affected_reservation_ids: affected.into_iter().collect(),
        candidate_spatial_violations: candidate_spatial_report.violations,
        baseline_spatial_violations: baseline_spatial_report.violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_space_time::Rect;

    fn yard() -> Rect {
        Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        }
    }

    fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Footprint {
        Footprint::AxisAlignedRect(Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    fn placement(id: &str, block: &str, footprint: Footprint, start: u64, end: u64) -> Placement {
        Placement {
            reservation_id: id.to_owned(),
            block_id: block.to_owned(),
            footprint,
            occupied: TimeRange {
                start_tick: start,
                end_tick: end,
            },
        }
    }

    fn transfer(id: &str, block: &str, start: u64, end: u64, vehicle: &str) -> Transfer {
        Transfer {
            id: id.to_owned(),
            block_id: block.to_owned(),
            corridor: rect(8, 8, 9, 9),
            interval: TimeRange {
                start_tick: start,
                end_tick: end,
            },
            vehicle_id: vehicle.to_owned(),
            predecessor_move_ids: Vec::new(),
        }
    }

    fn empty_disruption(tick: u64) -> Disruption {
        Disruption {
            observed_at_tick: tick,
            new_exclusion_zones: Vec::new(),
            vehicle_outages: Vec::new(),
            placement_occupancy_extensions: Vec::new(),
            explicitly_affected_reservation_ids: Vec::new(),
        }
    }

    fn zone(id: &str, footprint: Footprint, start: u64, end: u64) -> ExclusionZone {
        ExclusionZone {
            id: id.to_owned(),
            footprint,
            active: TimeRange {
                start_tick: start,
                end_tick: end,
            },
        }
    }

    #[test]
    fn exclusion_disruption_recovers_only_affected_future_reservation() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("block-a-past", "block-a", rect(0, 0, 4, 4), 0, 5),
                placement("block-a-future", "block-a", rect(0, 0, 4, 4), 5, 10),
                placement("block-b", "block-b", rect(4, 0, 8, 4), 0, 10),
            ],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("block-a-past", "block-a", rect(0, 0, 4, 4), 0, 5),
                placement("block-a-future", "block-a", rect(0, 4, 4, 8), 5, 10),
                placement("block-b", "block-b", rect(4, 0, 8, 4), 0, 10),
            ],
            ..Scenario::default()
        };
        let disruption = Disruption {
            new_exclusion_zones: vec![zone("zone-a", rect(0, 0, 4, 4), 5, 10)],
            ..empty_disruption(5)
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption,
        });

        assert!(report.accepted, "{:?}", report.violations);
        assert!(!report.metrics.baseline_feasible_after_disruption);
        assert!(report.metrics.candidate_feasible_after_disruption);
        assert_eq!(report.affected_reservation_ids, vec!["block-a-future"]);
        assert_eq!(report.metrics.changed_placement_reservations, 1);
        assert_eq!(report.metrics.unaffected_placement_churn, 0);
        assert_eq!(report.metrics.recovery_horizon_ticks, Some(5));
    }

    #[test]
    fn vehicle_outage_requires_reassignment_for_overlapping_transfer() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("p-a", "block-a", rect(0, 0, 2, 2), 0, 8)],
            transfers: vec![
                transfer("move-past", "block-a", 0, 5, "tug-1"),
                transfer("move-future", "block-a", 5, 8, "tug-1"),
            ],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: baseline.placements.clone(),
            transfers: vec![
                transfer("move-past", "block-a", 0, 5, "tug-1"),
                transfer("move-future", "block-a", 5, 8, "tug-2"),
            ],
            ..Scenario::default()
        };
        let disruption = Disruption {
            vehicle_outages: vec![VehicleOutage {
                vehicle_id: "tug-1".to_owned(),
                unavailable: TimeRange {
                    start_tick: 5,
                    end_tick: 8,
                },
            }],
            ..empty_disruption(5)
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption,
        });

        assert!(report.accepted, "{:?}", report.violations);
        assert!(!report.metrics.baseline_feasible_after_disruption);
        assert!(report.metrics.candidate_feasible_after_disruption);
        assert_eq!(report.metrics.changed_transfer_reservations, 1);
        assert_eq!(report.metrics.unaffected_transfer_churn, 0);
        assert_eq!(report.metrics.recovery_horizon_ticks, Some(3));
    }

    #[test]
    fn unrecovered_vehicle_outage_is_not_reported_as_accepted() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("p-a", "block-a", rect(0, 0, 2, 2), 0, 8)],
            transfers: vec![transfer("move-a", "block-a", 5, 8, "tug-1")],
            ..Scenario::default()
        };
        let disruption = Disruption {
            vehicle_outages: vec![VehicleOutage {
                vehicle_id: "tug-1".to_owned(),
                unavailable: TimeRange {
                    start_tick: 5,
                    end_tick: 8,
                },
            }],
            ..empty_disruption(5)
        };
        let report = evaluate_recovery(&RecoveryInput {
            candidate: baseline.clone(),
            baseline,
            disruption,
        });

        assert!(!report.accepted);
        assert!(!report.metrics.candidate_feasible_after_disruption);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecoveryViolationCode::VehicleUnavailable
        }));
    }

    #[test]
    fn completed_reservations_cannot_be_rewritten_after_disruption() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("completed", "block-a", rect(0, 0, 2, 2), 0, 5)],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("completed", "block-a", rect(2, 0, 4, 2), 0, 5)],
            ..Scenario::default()
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption: empty_disruption(5),
        });

        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecoveryViolationCode::HistoricalPlacementChanged
        }));
    }

    #[test]
    fn new_exclusion_zone_marks_affected_transfer_and_measures_recovery() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("p-a", "block-a", rect(0, 0, 1, 1), 0, 8)],
            transfers: vec![transfer("move-a", "block-a", 5, 8, "tug-1")],
            ..Scenario::default()
        };
        let mut candidate_transfer = transfer("move-a", "block-a", 5, 8, "tug-1");
        candidate_transfer.corridor = rect(7, 8, 8, 9);
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: baseline.placements.clone(),
            transfers: vec![candidate_transfer],
            ..Scenario::default()
        };
        let disruption = Disruption {
            new_exclusion_zones: vec![zone("zone-a", rect(8, 8, 9, 9), 5, 8)],
            ..empty_disruption(5)
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption,
        });

        assert!(report.accepted, "{:?}", report.violations);
        assert!(!report.metrics.baseline_feasible_after_disruption);
        assert_eq!(report.affected_reservation_ids, vec!["move-a"]);
        assert_eq!(report.metrics.changed_transfer_reservations, 1);
        assert_eq!(report.metrics.unaffected_transfer_churn, 0);
        assert_eq!(report.metrics.recovery_horizon_ticks, Some(3));
    }

    #[test]
    fn delayed_handoff_keeps_departing_block_occupied_until_clearance_is_observed() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5),
                placement("incoming", "block-in", rect(0, 0, 2, 2), 5, 10),
                placement("unaffected", "block-c", rect(4, 0, 6, 2), 0, 10),
            ],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5),
                placement("incoming", "block-in", rect(0, 2, 2, 4), 5, 10),
                placement("unaffected", "block-c", rect(4, 0, 6, 2), 0, 10),
            ],
            ..Scenario::default()
        };
        let disruption = Disruption {
            placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                reservation_id: "outgoing".to_owned(),
                occupied_until_tick: 7,
            }],
            ..empty_disruption(4)
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption,
        });

        assert!(report.accepted, "{:?}", report.violations);
        assert!(!report.metrics.baseline_feasible_after_disruption);
        assert!(report.metrics.candidate_feasible_after_disruption);
        assert_eq!(
            report.affected_reservation_ids,
            vec!["incoming".to_owned(), "outgoing".to_owned()]
        );
        assert_eq!(report.metrics.changed_placement_reservations, 1);
        assert_eq!(report.metrics.unaffected_placement_churn, 0);
        assert_eq!(report.metrics.recovery_horizon_ticks, Some(6));
        assert!(report
            .baseline_spatial_violations
            .iter()
            .any(|violation| violation.code == ViolationCode::BlockFootprintOverlap));
    }

    #[test]
    fn clearance_extension_updates_candidate_makespan_and_recovery_horizon() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5)],
            ..Scenario::default()
        };
        let report = evaluate_recovery(&RecoveryInput {
            candidate: baseline.clone(),
            baseline,
            disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "outgoing".to_owned(),
                    occupied_until_tick: 7,
                }],
                ..empty_disruption(4)
            },
        });

        assert!(report.accepted, "{:?}", report.violations);
        assert_eq!(report.metrics.candidate_makespan_tick, Some(7));
        assert_eq!(report.metrics.recovery_completion_tick, Some(7));
        assert_eq!(report.metrics.recovery_horizon_ticks, Some(3));
    }

    #[test]
    fn late_retroactive_occupancy_extension_is_rejected_as_completed_history() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5)],
            ..Scenario::default()
        };
        let report = evaluate_recovery(&RecoveryInput {
            candidate: baseline.clone(),
            baseline,
            disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "outgoing".to_owned(),
                    occupied_until_tick: 7,
                }],
                ..empty_disruption(5)
            },
        });

        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecoveryViolationCode::OccupancyExtensionAfterCompletion
        }));
    }

    #[test]
    fn candidate_cannot_remove_or_move_block_whose_old_footprint_is_not_clear() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5)],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("outgoing", "block-out", rect(2, 0, 4, 2), 0, 5)],
            ..Scenario::default()
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "outgoing".to_owned(),
                    occupied_until_tick: 7,
                }],
                ..empty_disruption(4)
            },
        });

        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecoveryViolationCode::OccupancyExtensionTargetMismatch
        }));
    }

    #[test]
    fn unknown_occupancy_extension_target_fails_closed() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("p-a", "block-a", rect(0, 0, 2, 2), 0, 8)],
            ..Scenario::default()
        };
        let report = evaluate_recovery(&RecoveryInput {
            candidate: baseline.clone(),
            baseline,
            disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "missing-block".to_owned(),
                    occupied_until_tick: 9,
                }],
                ..empty_disruption(4)
            },
        });

        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecoveryViolationCode::UnknownOccupancyExtensionReservation
        }));
    }

    #[test]
    fn robust_envelope_accepts_only_when_candidate_survives_all_clearance_scenarios() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5),
                placement("incoming", "block-in", rect(0, 0, 2, 2), 5, 10),
                placement("unaffected", "block-c", rect(4, 0, 6, 2), 0, 10),
            ],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5),
                placement("incoming", "block-in", rect(0, 2, 2, 4), 5, 10),
                placement("unaffected", "block-c", rect(4, 0, 6, 2), 0, 10),
            ],
            ..Scenario::default()
        };
        let scenarios = [6, 7, 8].map(|clear_tick| RecoveryEnvelopeScenario {
            scenario_id: format!("clear-at-{clear_tick}"),
            disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "outgoing".to_owned(),
                    occupied_until_tick: clear_tick,
                }],
                ..empty_disruption(4)
            },
        });

        let report = evaluate_recovery_envelope(&baseline, &candidate, &scenarios);

        assert!(report.robustly_accepted);
        assert_eq!(report.metrics.total_scenarios, 3);
        assert_eq!(report.metrics.accepted_scenarios, 3);
        assert_eq!(report.metrics.worst_case_recovery_horizon_ticks, Some(6));
        assert_eq!(report.metrics.worst_case_unaffected_placement_churn, 0);
        assert_eq!(
            report.scenarios.iter().map(|s| s.scenario_id.as_str()).collect::<Vec<_>>(),
            vec!["clear-at-6", "clear-at-7", "clear-at-8"]
        );
    }

    #[test]
    fn robust_envelope_rejects_candidate_that_only_survives_optimistic_clearance() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5),
                placement("incoming", "block-in", rect(0, 0, 2, 2), 5, 10),
            ],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("outgoing", "block-out", rect(0, 0, 2, 2), 0, 5),
                placement("incoming", "block-in", rect(0, 0, 2, 2), 6, 10),
            ],
            ..Scenario::default()
        };
        let scenarios = [6, 7].map(|clear_tick| RecoveryEnvelopeScenario {
            scenario_id: format!("clear-at-{clear_tick}"),
            disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "outgoing".to_owned(),
                    occupied_until_tick: clear_tick,
                }],
                ..empty_disruption(4)
            },
        });

        let report = evaluate_recovery_envelope(&baseline, &candidate, &scenarios);

        assert!(!report.robustly_accepted);
        assert_eq!(report.metrics.total_scenarios, 2);
        assert_eq!(report.metrics.accepted_scenarios, 1);
        assert!(report.scenarios.iter().any(|scenario| {
            scenario.scenario_id == "clear-at-7"
                && !scenario.report.metrics.candidate_feasible_after_disruption
        }));
    }

    #[test]
    fn robust_envelope_rejects_empty_or_ambiguous_scenario_sets() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![placement("p-a", "block-a", rect(0, 0, 2, 2), 0, 8)],
            ..Scenario::default()
        };

        let empty = evaluate_recovery_envelope(&baseline, &baseline, &[]);
        assert!(!empty.robustly_accepted);
        assert!(empty.violations.iter().any(|violation| {
            violation.code == RecoveryEnvelopeViolationCode::EmptyScenarioSet
        }));

        let duplicate = RecoveryEnvelopeScenario {
            scenario_id: "same-id".to_owned(),
            disruption: empty_disruption(4),
        };
        let duplicate_report = evaluate_recovery_envelope(
            &baseline,
            &baseline,
            &[duplicate.clone(), duplicate],
        );
        assert!(!duplicate_report.robustly_accepted);
        assert!(duplicate_report.violations.iter().any(|violation| {
            violation.code == RecoveryEnvelopeViolationCode::DuplicateScenarioId
        }));
    }

    #[test]
    fn candidate_churn_on_unaffected_reservation_is_measured_separately() {
        let baseline = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("a-future", "block-a", rect(0, 0, 2, 2), 5, 10),
                placement("b-future", "block-b", rect(2, 0, 4, 2), 5, 10),
            ],
            ..Scenario::default()
        };
        let candidate = Scenario {
            yard_bounds: Some(yard()),
            placements: vec![
                placement("a-future", "block-a", rect(0, 2, 2, 4), 5, 10),
                placement("b-future", "block-b", rect(2, 2, 4, 4), 5, 10),
            ],
            ..Scenario::default()
        };
        let disruption = Disruption {
            new_exclusion_zones: vec![zone("zone-a", rect(0, 0, 2, 2), 5, 10)],
            ..empty_disruption(5)
        };
        let report = evaluate_recovery(&RecoveryInput {
            baseline,
            candidate,
            disruption,
        });

        assert!(report.accepted, "{:?}", report.violations);
        assert_eq!(report.metrics.changed_placement_reservations, 2);
        assert_eq!(report.metrics.unaffected_placement_churn, 1);
    }
}
