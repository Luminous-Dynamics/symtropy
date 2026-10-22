//! Operational acceptance policy over an independently computed recovery report.
//!
//! Physical feasibility and operational acceptability are separate: the underlying recovery
//! evaluator reports whether the candidate satisfies spatial/history/disruption invariants; this
//! module applies site-defined dispatch stability and churn budgets without rewriting that report.

use std::collections::{BTreeMap, BTreeSet};

use shipyard_space_time::{Placement, Scenario, Transfer};

use super::{RecoveryInput, RecoveryReport, evaluate_recovery};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchStabilityPolicy {
    /// Existing reservations whose planned start is strictly before this tick are dispatch-frozen.
    pub freeze_before_tick: u64,
    /// Explicitly permits an affected frozen reservation to change when disruption requires it.
    /// This is a policy waiver, not a physical-feasibility override.
    pub allow_affected_frozen_changes: bool,
    pub max_unaffected_placement_churn: usize,
    pub max_unaffected_transfer_churn: usize,
    pub max_time_shift_ticks: u64,
    pub max_rectangle_edge_shift_units: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DispatchPolicyViolationCode {
    FrozenPlacementChanged,
    FrozenTransferChanged,
    UnaffectedPlacementChurnExceeded,
    UnaffectedTransferChurnExceeded,
    TimeShiftBudgetExceeded,
    RectangleShiftBudgetExceeded,
}

impl DispatchPolicyViolationCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FrozenPlacementChanged => "FrozenPlacementChanged",
            Self::FrozenTransferChanged => "FrozenTransferChanged",
            Self::UnaffectedPlacementChurnExceeded => "UnaffectedPlacementChurnExceeded",
            Self::UnaffectedTransferChurnExceeded => "UnaffectedTransferChurnExceeded",
            Self::TimeShiftBudgetExceeded => "TimeShiftBudgetExceeded",
            Self::RectangleShiftBudgetExceeded => "RectangleShiftBudgetExceeded",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchPolicyViolation {
    pub code: DispatchPolicyViolationCode,
    pub reservation_ids: Vec<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationalRecoveryReport {
    /// Raw physical feasibility report. Never rewritten by a dispatch-policy failure.
    pub recovery: RecoveryReport,
    /// False if the policy is violated or the underlying physical recovery is rejected.
    pub operationally_acceptable: bool,
    pub policy_violations: Vec<DispatchPolicyViolation>,
}

fn push(
    violations: &mut Vec<DispatchPolicyViolation>,
    code: DispatchPolicyViolationCode,
    mut reservation_ids: Vec<String>,
    detail: impl Into<String>,
) {
    reservation_ids.sort();
    reservation_ids.dedup();
    violations.push(DispatchPolicyViolation {
        code,
        reservation_ids,
        detail: detail.into(),
    });
}

fn sort_dedup(violations: &mut Vec<DispatchPolicyViolation>) {
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.reservation_ids.cmp(&right.reservation_ids))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    violations.dedup();
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

fn check_frozen_reservations(
    baseline: &Scenario,
    candidate: &Scenario,
    freeze_before_tick: u64,
    affected_ids: &BTreeSet<&str>,
    allow_affected_frozen_changes: bool,
    violations: &mut Vec<DispatchPolicyViolation>,
) {
    let baseline_placements = placement_map(baseline);
    let candidate_placements = placement_map(candidate);
    for (id, placement) in baseline_placements {
        if placement.occupied.start_tick >= freeze_before_tick {
            continue;
        }
        if candidate_placements.get(id) != Some(&placement)
            && !(allow_affected_frozen_changes && affected_ids.contains(id))
        {
            push(
                violations,
                DispatchPolicyViolationCode::FrozenPlacementChanged,
                vec![id.to_owned()],
                "placement begins inside the frozen dispatch horizon",
            );
        }
    }

    let baseline_transfers = transfer_map(baseline);
    let candidate_transfers = transfer_map(candidate);
    for (id, transfer) in baseline_transfers {
        if transfer.interval.start_tick >= freeze_before_tick {
            continue;
        }
        if candidate_transfers.get(id) != Some(&transfer)
            && !(allow_affected_frozen_changes && affected_ids.contains(id))
        {
            push(
                violations,
                DispatchPolicyViolationCode::FrozenTransferChanged,
                vec![id.to_owned()],
                "transfer begins inside the frozen dispatch horizon",
            );
        }
    }
}

/// Apply a dispatch-stability policy after physical recovery has been evaluated.
///
/// This wrapper intentionally distinguishes a geometrically valid but excessively disruptive
/// schedule from an invalid physical schedule. A policy pass never turns a rejected physical plan
/// into an accepted one; an explicit affected-reservation waiver only waives the freeze-window
/// policy for those IDs.
pub fn evaluate_operational_recovery(
    input: &RecoveryInput,
    policy: &DispatchStabilityPolicy,
) -> OperationalRecoveryReport {
    let recovery = evaluate_recovery(input);
    let mut violations = Vec::new();

    let affected_ids: BTreeSet<&str> = recovery
        .affected_reservation_ids
        .iter()
        .map(String::as_str)
        .collect();
    check_frozen_reservations(
        &input.baseline,
        &input.candidate,
        policy.freeze_before_tick,
        &affected_ids,
        policy.allow_affected_frozen_changes,
        &mut violations,
    );

    if recovery.metrics.unaffected_placement_churn > policy.max_unaffected_placement_churn {
        push(
            &mut violations,
            DispatchPolicyViolationCode::UnaffectedPlacementChurnExceeded,
            Vec::new(),
            format!(
                "unaffected placement churn {} exceeds policy budget {}",
                recovery.metrics.unaffected_placement_churn,
                policy.max_unaffected_placement_churn
            ),
        );
    }
    if recovery.metrics.unaffected_transfer_churn > policy.max_unaffected_transfer_churn {
        push(
            &mut violations,
            DispatchPolicyViolationCode::UnaffectedTransferChurnExceeded,
            Vec::new(),
            format!(
                "unaffected transfer churn {} exceeds policy budget {}",
                recovery.metrics.unaffected_transfer_churn,
                policy.max_unaffected_transfer_churn
            ),
        );
    }
    if recovery.metrics.time_shift_ticks > policy.max_time_shift_ticks {
        push(
            &mut violations,
            DispatchPolicyViolationCode::TimeShiftBudgetExceeded,
            Vec::new(),
            format!(
                "time shift {} exceeds policy budget {}",
                recovery.metrics.time_shift_ticks, policy.max_time_shift_ticks
            ),
        );
    }
    if recovery.metrics.rectangle_edge_shift_units > policy.max_rectangle_edge_shift_units {
        push(
            &mut violations,
            DispatchPolicyViolationCode::RectangleShiftBudgetExceeded,
            Vec::new(),
            format!(
                "rectangle edge shift {} exceeds policy budget {}",
                recovery.metrics.rectangle_edge_shift_units,
                policy.max_rectangle_edge_shift_units
            ),
        );
    }

    sort_dedup(&mut violations);
    let operationally_acceptable = recovery.accepted && violations.is_empty();
    OperationalRecoveryReport {
        recovery,
        operationally_acceptable,
        policy_violations: violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_space_time::{Footprint, Rect, TimeRange};
    use crate::Disruption;

    fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Footprint {
        Footprint::AxisAlignedRect(Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    fn placement(id: &str, x: i64, y: i64, start: u64, end: u64) -> Placement {
        Placement {
            reservation_id: id.to_owned(),
            block_id: format!("block-{id}"),
            footprint: rect(x, y, x + 2, y + 2),
            occupied: TimeRange {
                start_tick: start,
                end_tick: end,
            },
        }
    }

    fn transfer(id: &str, start: u64, end: u64, vehicle: &str) -> Transfer {
        Transfer {
            id: id.to_owned(),
            block_id: format!("block-{id}"),
            corridor: rect(0, 8, 2, 10),
            interval: TimeRange {
                start_tick: start,
                end_tick: end,
            },
            vehicle_id: vehicle.to_owned(),
            predecessor_move_ids: Vec::new(),
        }
    }

    fn input(baseline: Scenario, candidate: Scenario) -> RecoveryInput {
        RecoveryInput {
            baseline,
            candidate,
            disruption: Disruption {
                observed_at_tick: 0,
                new_exclusion_zones: Vec::new(),
                vehicle_outages: Vec::new(),
                placement_occupancy_extensions: Vec::new(),
                explicitly_affected_reservation_ids: Vec::new(),
            },
        }
    }

    fn policy(freeze_before_tick: u64) -> DispatchStabilityPolicy {
        DispatchStabilityPolicy {
            freeze_before_tick,
            allow_affected_frozen_changes: false,
            max_unaffected_placement_churn: 100,
            max_unaffected_transfer_churn: 100,
            max_time_shift_ticks: u64::MAX,
            max_rectangle_edge_shift_units: u64::MAX,
        }
    }

    fn scenario(placements: Vec<Placement>, transfers: Vec<Transfer>) -> Scenario {
        Scenario {
            yard_bounds: Some(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 20,
                max_y: 20,
            }),
            placements,
            transfers,
            ..Scenario::default()
        }
    }

    #[test]
    fn physically_valid_schedule_can_fail_frozen_dispatch_policy() {
        let baseline = scenario(vec![placement("p-a", 0, 0, 5, 10)], Vec::new());
        let candidate = scenario(vec![placement("p-a", 0, 2, 5, 10)], Vec::new());
        let report = evaluate_operational_recovery(&input(baseline, candidate), &policy(8));

        assert!(report.recovery.accepted, "{:?}", report.recovery.violations);
        assert!(!report.operationally_acceptable);
        assert!(report.policy_violations.iter().any(|violation| {
            violation.code == DispatchPolicyViolationCode::FrozenPlacementChanged
        }));
    }

    #[test]
    fn dispatch_freeze_boundary_is_half_open_and_exact_start_is_not_frozen() {
        let baseline = scenario(vec![placement("p-a", 0, 0, 8, 10)], Vec::new());
        let candidate = scenario(vec![placement("p-a", 0, 2, 8, 10)], Vec::new());
        let report = evaluate_operational_recovery(&input(baseline, candidate), &policy(8));

        assert!(report.recovery.accepted);
        assert!(report.operationally_acceptable, "{:?}", report.policy_violations);
    }

    #[test]
    fn affected_frozen_reservation_requires_explicit_policy_waiver() {
        let baseline = scenario(
            vec![placement("p-a", 0, 0, 5, 10)],
            Vec::new(),
        );
        let candidate = scenario(
            vec![placement("p-a", 0, 2, 5, 10)],
            Vec::new(),
        );
        let mut request = input(baseline, candidate);
        request.disruption.explicitly_affected_reservation_ids = vec!["p-a".to_owned()];
        let mut locked = policy(8);
        let rejected = evaluate_operational_recovery(&request, &locked);
        assert!(!rejected.operationally_acceptable);

        locked.allow_affected_frozen_changes = true;
        let allowed = evaluate_operational_recovery(&request, &locked);
        assert!(allowed.recovery.accepted);
        assert!(allowed.operationally_acceptable, "{:?}", allowed.policy_violations);
    }

    #[test]
    fn policy_reports_churn_and_geometry_budgets_without_rewriting_physical_report() {
        let baseline = scenario(
            vec![
                placement("p-a", 0, 0, 10, 20),
                placement("p-b", 4, 0, 6, 20),
            ],
            vec![transfer("t-a", 10, 12, "tug-a")],
        );
        let candidate = scenario(
            vec![
                placement("p-a", 0, 2, 10, 20),
                placement("p-b", 4, 0, 6, 20),
            ],
            vec![transfer("t-a", 10, 12, "tug-a")],
        );
        let mut limits = policy(0);
        limits.max_unaffected_placement_churn = 0;
        limits.max_rectangle_edge_shift_units = 0;
        let report = evaluate_operational_recovery(&input(baseline, candidate), &limits);

        assert!(report.recovery.accepted, "{:?}", report.recovery.violations);
        assert!(!report.operationally_acceptable);
        assert!(report.policy_violations.iter().any(|violation| {
            violation.code == DispatchPolicyViolationCode::UnaffectedPlacementChurnExceeded
        }));
        assert!(report.policy_violations.iter().any(|violation| {
            violation.code == DispatchPolicyViolationCode::RectangleShiftBudgetExceeded
        }));
    }

    #[test]
    fn frozen_transfer_reassignment_is_reported_separately() {
        let baseline = scenario(
            vec![placement("p-a", 0, 0, 0, 10)],
            vec![transfer("t-a", 5, 7, "tug-a")],
        );
        let candidate = scenario(
            vec![placement("p-a", 0, 0, 0, 10)],
            vec![transfer("t-a", 5, 7, "tug-b")],
        );
        let report = evaluate_operational_recovery(&input(baseline, candidate), &policy(8));

        assert!(!report.operationally_acceptable);
        assert!(report.policy_violations.iter().any(|violation| {
            violation.code == DispatchPolicyViolationCode::FrozenTransferChanged
        }));
    }
}
