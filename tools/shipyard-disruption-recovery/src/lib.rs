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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Disruption {
    /// First tick at which the disturbance is known to the planner.
    pub observed_at_tick: u64,
    /// New zones which become authoritative at this disruption.
    pub new_exclusion_zones: Vec<ExclusionZone>,
    /// Vehicle unavailability intervals.
    pub vehicle_outages: Vec<VehicleOutage>,
    /// Known work that requires reconsideration; geometry/outage impacts are also derived.
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

fn apply_disruption(scenario: &Scenario, disruption: &Disruption) -> Scenario {
    let mut effective = scenario.clone();
    effective
        .exclusion_zones
        .extend(disruption.new_exclusion_zones.iter().cloned());
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
    let affected = derive_affected_ids(&input.baseline, &input.disruption, &mut violations);
    let baseline_after = apply_disruption(&input.baseline, &input.disruption);
    let candidate_after = apply_disruption(&input.candidate, &input.disruption);

    let baseline_spatial_report = verify(&baseline_after);
    let baseline_outage_violations =
        outage_violations(&baseline_after, &input.disruption.vehicle_outages);
    let baseline_feasible_after_disruption =
        baseline_spatial_report.is_feasible()
            && baseline_outage_violations.is_empty()
            && outages_valid;

    let candidate_spatial_report = verify(&candidate_after);
    let candidate_outage_violations =
        outage_violations(&candidate_after, &input.disruption.vehicle_outages);
    let candidate_feasible_after_disruption = candidate_spatial_report.is_feasible()
        && candidate_outage_violations.is_empty()
        && outages_valid;

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
        candidate_makespan_tick: makespan(&input.candidate),
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

    metrics.recovery_completion_tick = recovery_completion(&input.candidate, &changed_ids);
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
