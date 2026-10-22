//! Convert explicitly bound execution completion evidence into recovery constraints.
//!
//! This bridge does not infer that arbitrary operation completion clears a block. The caller must
//! explicitly map an operation to a placement whose physical clearance that completion confirms.
//! Source references are recorded for audit, but this module does not cryptographically attest them.

use std::collections::{BTreeMap, BTreeSet};

use shipyard_space_time::{Placement, Scenario};

use crate::{
    Disruption, PlacementOccupancyExtension,
    execution::{
        ExecutionEvent, ExecutionEventKind, ExecutionReplayCutoff, ExecutionReplayReport,
        PlannedOperation, replay_execution,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementClearanceBinding {
    /// Operation whose authoritative completion means the block is physically clear.
    pub operation_id: String,
    /// Existing placement reservation whose footprint may be released after that completion.
    pub placement_reservation_id: String,
}

/// Positive observation that a known placement still occupies its footprint at a decision tick.
///
/// This is distinct from missing completion evidence: absence of an event does not prove occupancy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementOccupancyWitness {
    pub witness_id: String,
    pub placement_reservation_id: String,
    pub source_id: String,
    pub evidence_ref: String,
    pub occupied_at_tick: u64,
    pub recorded_tick: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExecutionRecoveryBridgeViolationCode {
    InvalidExecutionEvidence,
    DecisionAfterExecutionCutoff,
    PriorDisruptionAfterDecision,
    EmptyBindingOperationId,
    EmptyBindingReservationId,
    DuplicateBindingOperation,
    DuplicateBindingReservation,
    UnknownBoundOperation,
    UnknownPlacementReservation,
    ClearanceConfirmationMissing,
    ClearanceConfirmationAfterDecision,
    DuplicatePriorOccupancyExtension,
    EmptyWitnessId,
    DuplicateWitnessId,
    EmptyWitnessReservationId,
    UnknownWitnessReservation,
    EmptyWitnessSourceId,
    EmptyWitnessEvidenceReference,
    WitnessRecordedBeforeObservation,
    DuplicateWitnessForReservation,
    DecisionTickOverflow,
}

impl ExecutionRecoveryBridgeViolationCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidExecutionEvidence => "InvalidExecutionEvidence",
            Self::DecisionAfterExecutionCutoff => "DecisionAfterExecutionCutoff",
            Self::PriorDisruptionAfterDecision => "PriorDisruptionAfterDecision",
            Self::EmptyBindingOperationId => "EmptyBindingOperationId",
            Self::EmptyBindingReservationId => "EmptyBindingReservationId",
            Self::DuplicateBindingOperation => "DuplicateBindingOperation",
            Self::DuplicateBindingReservation => "DuplicateBindingReservation",
            Self::UnknownBoundOperation => "UnknownBoundOperation",
            Self::UnknownPlacementReservation => "UnknownPlacementReservation",
            Self::ClearanceConfirmationMissing => "ClearanceConfirmationMissing",
            Self::ClearanceConfirmationAfterDecision => "ClearanceConfirmationAfterDecision",
            Self::DuplicatePriorOccupancyExtension => "DuplicatePriorOccupancyExtension",
            Self::EmptyWitnessId => "EmptyWitnessId",
            Self::DuplicateWitnessId => "DuplicateWitnessId",
            Self::EmptyWitnessReservationId => "EmptyWitnessReservationId",
            Self::UnknownWitnessReservation => "UnknownWitnessReservation",
            Self::EmptyWitnessSourceId => "EmptyWitnessSourceId",
            Self::EmptyWitnessEvidenceReference => "EmptyWitnessEvidenceReference",
            Self::WitnessRecordedBeforeObservation => "WitnessRecordedBeforeObservation",
            Self::DuplicateWitnessForReservation => "DuplicateWitnessForReservation",
            Self::DecisionTickOverflow => "DecisionTickOverflow",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionRecoveryBridgeViolation {
    pub code: ExecutionRecoveryBridgeViolationCode,
    pub subject_ids: Vec<String>,
    pub detail: String,
}

fn push(
    violations: &mut Vec<ExecutionRecoveryBridgeViolation>,
    code: ExecutionRecoveryBridgeViolationCode,
    mut subject_ids: Vec<String>,
    detail: impl Into<String>,
) {
    subject_ids.sort();
    violations.push(ExecutionRecoveryBridgeViolation {
        code,
        subject_ids,
        detail: detail.into(),
    });
}

fn sort_violations(violations: &mut Vec<ExecutionRecoveryBridgeViolation>) {
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject_ids.cmp(&right.subject_ids))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    violations.dedup();
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ClearanceExtensionEvidenceKind {
    OccupancyStillPresentWitness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClearanceExtensionReceipt {
    pub operation_id: String,
    pub placement_reservation_id: String,
    pub evidence_kind: ClearanceExtensionEvidenceKind,
    pub source_evidence_id: String,
    pub source_id: String,
    pub evidence_ref: String,
    pub occurred_tick: u64,
    pub recorded_tick: u64,
    pub previous_occupied_until_tick: u64,
    pub new_occupied_until_tick: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionRecoveryBridgeReport {
    pub accepted: bool,
    pub decision_tick: u64,
    pub execution_replay: ExecutionReplayReport,
    /// Present only when every binding and clearance requirement is consistent.
    pub disruption: Option<Disruption>,
    pub receipts: Vec<ClearanceExtensionReceipt>,
    pub violations: Vec<ExecutionRecoveryBridgeViolation>,
}

/// Use execution evidence to augment the existing disruption before evaluating a recovery plan.
///
/// The physical cutoff must equal the planning decision tick. Events recorded after the evidence
/// cutoff are unavailable to this decision even if they assert an earlier occurrence time. If a
/// bound placement should already have cleared by the decision tick but its completion is not
/// visible, the bridge rejects the update rather than treating missing evidence as clearance.
pub fn augment_disruption_from_execution(
    baseline: &Scenario,
    planned_operations: &[PlannedOperation],
    events: &[ExecutionEvent],
    occupancy_witnesses: &[PlacementOccupancyWitness],
    cutoff: ExecutionReplayCutoff,
    decision_tick: u64,
    prior_disruption: &Disruption,
    bindings: &[PlacementClearanceBinding],
) -> ExecutionRecoveryBridgeReport {
    let replay = replay_execution(planned_operations, events, cutoff);
    let mut violations = Vec::new();

    if !replay.evidence_valid {
        push(
            &mut violations,
            ExecutionRecoveryBridgeViolationCode::InvalidExecutionEvidence,
            Vec::new(),
            "execution replay reported malformed or ambiguous source evidence",
        );
    }
    if cutoff.as_of_occurred_tick != decision_tick {
        push(
            &mut violations,
            ExecutionRecoveryBridgeViolationCode::DecisionAfterExecutionCutoff,
            Vec::new(),
            "physical execution cutoff must equal the recovery decision tick",
        );
    }
    if prior_disruption.observed_at_tick > decision_tick {
        push(
            &mut violations,
            ExecutionRecoveryBridgeViolationCode::PriorDisruptionAfterDecision,
            Vec::new(),
            "prior disruption cannot be observed after the current decision tick",
        );
    }

    let plan_by_id: BTreeMap<&str, &PlannedOperation> = planned_operations
        .iter()
        .map(|operation| (operation.operation_id.as_str(), operation))
        .collect();
    let placement_by_id: BTreeMap<&str, &Placement> = baseline
        .placements
        .iter()
        .map(|placement| (placement.reservation_id.as_str(), placement))
        .collect();
    let assessments: BTreeMap<&str, _> = replay
        .operations
        .iter()
        .map(|assessment| (assessment.operation_id.as_str(), assessment))
        .collect();

    let mut seen_operations = BTreeSet::new();
    let mut seen_reservations = BTreeSet::new();
    let mut seen_prior_extensions = BTreeSet::new();
    for extension in &prior_disruption.placement_occupancy_extensions {
        if !seen_prior_extensions.insert(extension.reservation_id.as_str()) {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::DuplicatePriorOccupancyExtension,
                vec![extension.reservation_id.clone()],
                "prior disruption contains multiple occupancy extensions for one reservation",
            );
        }
    }

    let mut visible_witnesses_by_reservation: BTreeMap<&str, &PlacementOccupancyWitness> =
        BTreeMap::new();
    let mut visible_witness_ids = BTreeSet::new();
    for witness in occupancy_witnesses.iter().filter(|witness| {
        witness.occupied_at_tick == decision_tick
            && witness.occupied_at_tick <= cutoff.as_of_occurred_tick
            && witness.recorded_tick <= cutoff.as_of_recorded_tick
    }) {
        if witness.witness_id.trim().is_empty() {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::EmptyWitnessId,
                vec![witness.placement_reservation_id.clone()],
                "occupancy witness identity is required",
            );
        } else if !visible_witness_ids.insert(witness.witness_id.as_str()) {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::DuplicateWitnessId,
                vec![witness.witness_id.clone()],
                "visible occupancy witness IDs must be unique",
            );
        }
        if witness.placement_reservation_id.trim().is_empty() {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::EmptyWitnessReservationId,
                vec![witness.witness_id.clone()],
                "occupancy witness must identify a placement reservation",
            );
        } else if !placement_by_id.contains_key(witness.placement_reservation_id.as_str()) {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::UnknownWitnessReservation,
                vec![witness.witness_id.clone(), witness.placement_reservation_id.clone()],
                "occupancy witness references an unknown placement reservation",
            );
        }
        if witness.source_id.trim().is_empty() {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::EmptyWitnessSourceId,
                vec![witness.witness_id.clone()],
                "occupancy witness source identity is required",
            );
        }
        if witness.evidence_ref.trim().is_empty() {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::EmptyWitnessEvidenceReference,
                vec![witness.witness_id.clone()],
                "occupancy witness evidence reference is required",
            );
        }
        if witness.recorded_tick < witness.occupied_at_tick {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::WitnessRecordedBeforeObservation,
                vec![witness.witness_id.clone()],
                "recorded tick cannot precede the occupancy observation tick",
            );
        }
        if visible_witnesses_by_reservation
            .insert(witness.placement_reservation_id.as_str(), witness)
            .is_some()
        {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::DuplicateWitnessForReservation,
                vec![witness.placement_reservation_id.clone()],
                "multiple current occupancy witnesses target the same placement in V0",
            );
        }
    }

    let mut generated_extensions = Vec::new();
    let mut receipts = Vec::new();
    for binding in bindings {
        if binding.operation_id.trim().is_empty() {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::EmptyBindingOperationId,
                vec![binding.placement_reservation_id.clone()],
                "clearance binding operation ID is required",
            );
            continue;
        }
        if binding.placement_reservation_id.trim().is_empty() {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::EmptyBindingReservationId,
                vec![binding.operation_id.clone()],
                "clearance binding placement reservation ID is required",
            );
            continue;
        }
        if !seen_operations.insert(binding.operation_id.as_str()) {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::DuplicateBindingOperation,
                vec![binding.operation_id.clone()],
                "one completion event cannot clear multiple reservations through duplicated bindings",
            );
            continue;
        }
        if !seen_reservations.insert(binding.placement_reservation_id.as_str()) {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::DuplicateBindingReservation,
                vec![binding.placement_reservation_id.clone()],
                "one placement reservation can have only one clearance binding in V0",
            );
            continue;
        }

        let Some(operation) = plan_by_id.get(binding.operation_id.as_str()) else {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::UnknownBoundOperation,
                vec![binding.operation_id.clone()],
                "clearance binding references an operation outside the frozen plan",
            );
            continue;
        };
        let Some(placement) = placement_by_id.get(binding.placement_reservation_id.as_str()) else {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::UnknownPlacementReservation,
                vec![
                    binding.operation_id.clone(),
                    binding.placement_reservation_id.clone(),
                ],
                "clearance binding references no baseline placement reservation",
            );
            continue;
        };

        let assessment = assessments.get(binding.operation_id.as_str()).copied();
        let completion = assessment.and_then(|item| item.observed_completion_tick);
        if let Some(completion_tick) = completion {
            if completion_tick > decision_tick {
                push(
                    &mut violations,
                    ExecutionRecoveryBridgeViolationCode::ClearanceConfirmationAfterDecision,
                    vec![
                        binding.operation_id.clone(),
                        binding.placement_reservation_id.clone(),
                    ],
                    "completion event cannot clear a placement before the event occurred",
                );
            }
            // A completion already observed at or before this decision means the block is not
            // asserted to remain present now. Late completion is measured in execution replay;
            // only a current positive occupancy witness creates a forward constraint.
            continue;
        }

        if decision_tick < placement.occupied.end_tick {
            continue;
        }
        let Some(witness) =
            visible_witnesses_by_reservation.get(binding.placement_reservation_id.as_str()).copied()
        else {
            if decision_tick >= operation.planned_end_tick {
                push(
                    &mut violations,
                    ExecutionRecoveryBridgeViolationCode::ClearanceConfirmationMissing,
                    vec![
                        binding.operation_id.clone(),
                        binding.placement_reservation_id.clone(),
                    ],
                    "placement interval has elapsed without a visible completion event or a current positive occupancy witness",
                );
            }
            continue;
        };

        let Some(new_occupied_until_tick) = decision_tick.checked_add(1) else {
            push(
                &mut violations,
                ExecutionRecoveryBridgeViolationCode::DecisionTickOverflow,
                vec![
                    binding.operation_id.clone(),
                    binding.placement_reservation_id.clone(),
                ],
                "cannot construct a next-tick conservative occupancy interval",
            );
            continue;
        };
        let new_occupied_until_tick = new_occupied_until_tick.max(placement.occupied.end_tick);
        if new_occupied_until_tick > placement.occupied.end_tick {
            generated_extensions.push(PlacementOccupancyExtension {
                reservation_id: binding.placement_reservation_id.clone(),
                occupied_until_tick: new_occupied_until_tick,
                evidence: Some(crate::OccupancyExtensionEvidence {
                    evidence_id: witness.witness_id.clone(),
                    source_id: witness.source_id.clone(),
                    evidence_ref: witness.evidence_ref.clone(),
                    observed_present_at_tick: witness.occupied_at_tick,
                    recorded_tick: witness.recorded_tick,
                }),
            });
            receipts.push(ClearanceExtensionReceipt {
                operation_id: binding.operation_id.clone(),
                placement_reservation_id: binding.placement_reservation_id.clone(),
                evidence_kind: ClearanceExtensionEvidenceKind::OccupancyStillPresentWitness,
                source_evidence_id: witness.witness_id.clone(),
                source_id: witness.source_id.clone(),
                evidence_ref: witness.evidence_ref.clone(),
                occurred_tick: witness.occupied_at_tick,
                recorded_tick: witness.recorded_tick,
                previous_occupied_until_tick: placement.occupied.end_tick,
                new_occupied_until_tick,
            });
        }
    }

    if !violations.is_empty() {
        sort_violations(&mut violations);
        return ExecutionRecoveryBridgeReport {
            accepted: false,
            decision_tick,
            execution_replay: replay,
            disruption: None,
            receipts: Vec::new(),
            violations,
        };
    }

    let mut disruption = prior_disruption.clone();
    disruption.observed_at_tick = decision_tick;
    for generated in generated_extensions {
        if let Some(prior) = disruption
            .placement_occupancy_extensions
            .iter_mut()
            .find(|extension| extension.reservation_id == generated.reservation_id)
        {
            prior.occupied_until_tick = prior.occupied_until_tick.max(generated.occupied_until_tick);
            if generated.evidence.is_some() {
                prior.evidence = generated.evidence;
            }
        } else {
            disruption.placement_occupancy_extensions.push(generated);
        }
    }
    disruption
        .placement_occupancy_extensions
        .sort_by(|left, right| left.reservation_id.cmp(&right.reservation_id));
    disruption
        .explicitly_affected_reservation_ids
        .sort();
    disruption.explicitly_affected_reservation_ids.dedup();
    receipts.sort_by(|left, right| {
        left.placement_reservation_id
            .cmp(&right.placement_reservation_id)
            .then_with(|| left.operation_id.cmp(&right.operation_id))
    });

    ExecutionRecoveryBridgeReport {
        accepted: true,
        decision_tick,
        execution_replay: replay,
        disruption: Some(disruption),
        receipts,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{ExecutionReplayCutoff, PlannedOperation};
    use shipyard_space_time::{Rect, TimeRange};

    fn baseline(end_tick: u64) -> Scenario {
        Scenario {
            yard_bounds: Some(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 10,
                max_y: 10,
            }),
            placements: vec![Placement {
                reservation_id: "block-a-placement".to_owned(),
                block_id: "block-a".to_owned(),
                footprint: shipyard_space_time::Footprint::AxisAlignedRect(Rect {
                    min_x: 0,
                    min_y: 0,
                    max_x: 2,
                    max_y: 2,
                }),
                occupied: TimeRange {
                    start_tick: 0,
                    end_tick: end_tick,
                },
            }],
            ..Scenario::default()
        }
    }

    fn plan(end_tick: u64) -> [PlannedOperation; 1] {
        [PlannedOperation {
            operation_id: "lift-block-a-clear".to_owned(),
            planned_start_tick: 2,
            planned_end_tick: end_tick,
        }]
    }

    fn event(
        id: &str,
        kind: ExecutionEventKind,
        occurred: u64,
        recorded: u64,
    ) -> ExecutionEvent {
        ExecutionEvent {
            event_id: id.to_owned(),
            operation_id: "lift-block-a-clear".to_owned(),
            source_id: "mes-bridge-test".to_owned(),
            evidence_ref: format!("fixture://{id}"),
            kind,
            occurred_tick: occurred,
            recorded_tick: recorded,
        }
    }

    fn prior(tick: u64) -> Disruption {
        Disruption {
            observed_at_tick: tick,
            new_exclusion_zones: Vec::new(),
            vehicle_outages: Vec::new(),
            placement_occupancy_extensions: Vec::new(),
            explicitly_affected_reservation_ids: Vec::new(),
        }
    }

    fn binding() -> [PlacementClearanceBinding; 1] {
        [PlacementClearanceBinding {
            operation_id: "lift-block-a-clear".to_owned(),
            placement_reservation_id: "block-a-placement".to_owned(),
        }]
    }

    #[test]
    fn late_clearance_event_generates_receipted_occupancy_extension() {
        let report = augment_disruption_from_execution(
            &baseline(5),
            &plan(5),
            &[event("start-clear", ExecutionEventKind::Started, 2, 2)],
            &[PlacementOccupancyWitness {
                witness_id: "scan-block-a-at-8".to_owned(),
                placement_reservation_id: "block-a-placement".to_owned(),
                source_id: "yard-scanner-1".to_owned(),
                evidence_ref: "scan://block-a/8".to_owned(),
                occupied_at_tick: 8,
                recorded_tick: 8,
            }],
            ExecutionReplayCutoff {
                as_of_occurred_tick: 8,
                as_of_recorded_tick: 8,
            },
            8,
            &prior(4),
            &binding(),
        );

        assert!(report.accepted, "{:?}", report.violations);
        let disruption = report.disruption.as_ref().unwrap();
        assert_eq!(disruption.observed_at_tick, 8);
        assert_eq!(disruption.placement_occupancy_extensions.len(), 1);
        assert_eq!(
            disruption.placement_occupancy_extensions[0].occupied_until_tick,
            9
        );
        assert_eq!(report.receipts.len(), 1);
        assert_eq!(report.receipts[0].source_evidence_id, "scan-block-a-at-8");
        assert_eq!(report.receipts[0].previous_occupied_until_tick, 5);
    }

    #[test]
    fn late_recorded_completion_is_not_usable_for_an_earlier_recovery_decision() {
        let report = augment_disruption_from_execution(
            &baseline(5),
            &plan(5),
            &[],
            &[PlacementOccupancyWitness {
                witness_id: "late-scan".to_owned(),
                placement_reservation_id: "block-a-placement".to_owned(),
                source_id: "yard-scanner-1".to_owned(),
                evidence_ref: "scan://block-a/8".to_owned(),
                occupied_at_tick: 8,
                recorded_tick: 12,
            }],
            ExecutionReplayCutoff {
                as_of_occurred_tick: 8,
                as_of_recorded_tick: 10,
            },
            8,
            &prior(4),
            &binding(),
        );

        assert!(!report.accepted);
        assert!(report.disruption.is_none());
        assert!(report.violations.iter().any(|violation| {
            violation.code == ExecutionRecoveryBridgeViolationCode::ClearanceConfirmationMissing
        }));
    }

    #[test]
    fn absent_clearance_confirmation_after_planned_end_fails_closed() {
        let report = augment_disruption_from_execution(
            &baseline(5),
            &plan(5),
            &[],
            &[],
            ExecutionReplayCutoff {
                as_of_occurred_tick: 8,
                as_of_recorded_tick: 8,
            },
            8,
            &prior(4),
            &binding(),
        );
        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ExecutionRecoveryBridgeViolationCode::ClearanceConfirmationMissing
        }));
    }

    #[test]
    fn completion_before_known_placement_end_does_not_create_an_extension() {
        let report = augment_disruption_from_execution(
            &baseline(10),
            &plan(5),
            &[event("finish-clear", ExecutionEventKind::Completed, 8, 9)],
            &[],
            ExecutionReplayCutoff {
                as_of_occurred_tick: 8,
                as_of_recorded_tick: 9,
            },
            8,
            &prior(4),
            &binding(),
        );
        assert!(report.accepted);
        assert!(report.receipts.is_empty());
        assert!(report.disruption.unwrap().placement_occupancy_extensions.is_empty());
    }

    #[test]
    fn unknown_binding_and_duplicate_bindings_fail_closed() {
        let unknown = [PlacementClearanceBinding {
            operation_id: "missing-operation".to_owned(),
            placement_reservation_id: "block-a-placement".to_owned(),
        }];
        let result = augment_disruption_from_execution(
            &baseline(5),
            &plan(5),
            &[event("finish-clear", ExecutionEventKind::Completed, 5, 5)],
            &[],
            ExecutionReplayCutoff {
                as_of_occurred_tick: 5,
                as_of_recorded_tick: 5,
            },
            5,
            &prior(4),
            &unknown,
        );
        assert!(!result.accepted);
        assert!(result.violations.iter().any(|violation| {
            violation.code == ExecutionRecoveryBridgeViolationCode::UnknownBoundOperation
        }));

        let binding = binding()[0].clone();
        let duplicate = [binding.clone(), binding];
        let result = augment_disruption_from_execution(
            &baseline(5),
            &plan(5),
            &[event("finish-clear", ExecutionEventKind::Completed, 5, 5)],
            &[],
            ExecutionReplayCutoff {
                as_of_occurred_tick: 5,
                as_of_recorded_tick: 5,
            },
            5,
            &prior(4),
            &duplicate,
        );
        assert!(!result.accepted);
        assert!(result.violations.iter().any(|violation| {
            violation.code == ExecutionRecoveryBridgeViolationCode::DuplicateBindingOperation
        }));
    }
}
