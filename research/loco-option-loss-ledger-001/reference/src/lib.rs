//! Deterministic, evidence-preserving accounting for finite future-option changes.
//!
//! This crate consumes already-qualified option-set snapshots and transition
//! receipts. It does not calculate viability, physical branches, thermal state,
//! actuator authority, route topology, or control commands.
//!
//! All physical/resource dimensions remain separate. The report is retrospective
//! and never authorizes actuation.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Evidence state supplied by the owner of a measurement or qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    /// Evidence is qualified for the exact bound scope.
    Qualified,
    /// Required evidence is missing, stale, incomplete, or not established.
    Unknown,
    /// Evidence exists but cannot resolve the declared claim.
    Inconclusive,
    /// Evidence contradicts a declared invariant or hard constraint.
    Rejected,
}

/// Overall result of one option-set transition audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditDisposition {
    /// Every changed option has qualified, scope-matched evidence.
    Qualified,
    /// At least one required evidence item is not established.
    Unknown,
    /// At least one upstream measurement is inconclusive.
    Inconclusive,
    /// A supplied identity or structural invariant is contradicted.
    Rejected,
}

/// A stable identity for the finite comparison scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonScope {
    /// Mission-run identity.
    pub mission_id: String,
    /// Robot or embodied-agent identity.
    pub robot_id: String,
    /// Frozen scenario-manifest digest.
    pub scenario_manifest_digest: String,
    /// Complete physically possible branch-manifest digest.
    pub branch_manifest_digest: String,
    /// Exact canonical continuation-action domain digest.
    pub action_domain_digest: String,
}

/// A qualified finite option-set snapshot at one mission event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSnapshot {
    /// Frozen comparison scope.
    pub scope: ComparisonScope,
    /// Monotonically increasing mission event sequence.
    pub event_sequence: u64,
    /// Physical/reserve/thermal state before this transition.
    pub predecessor_state_digest: String,
    /// Physical/reserve/thermal state after this transition.
    pub result_state_digest: String,
    /// Digest of the delivered-observation partition.
    pub observation_partition_digest: String,
    /// Exact canonical action identifiers in the declared finite domain.
    pub action_domain_ids: Vec<String>,
    /// Canonical actions available to the controller for this observation.
    pub available_option_ids: Vec<String>,
    /// Completeness and qualification of the supplied snapshot.
    pub disposition: EvidenceDisposition,
    /// Whether all physically possible branches are represented.
    pub branch_manifest_complete: bool,
    /// Whether the finite continuation-action domain is complete.
    pub action_domain_complete: bool,
}

/// Receipt proving that a delivered observation partition changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationPartitionReceipt {
    /// Prior partition digest.
    pub predecessor_partition_digest: String,
    /// Resulting partition digest.
    pub result_partition_digest: String,
    /// Receipt digest from the observation/calibration owner.
    pub receipt_digest: String,
    /// Evidence state of the observation refinement.
    pub disposition: EvidenceDisposition,
}

/// The physical or informational cause of an option becoming unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LossCause {
    /// Per-node thermal state removed the action from the qualified domain.
    ThermalDerating,
    /// Torque or slew authority became insufficient.
    ActuatorAuthorityLoss,
    /// Locomotion or safe-stop energy reserve became insufficient.
    EnergyDepletion,
    /// A required action or recovery deadline expired.
    DeadlineExpiry,
    /// Contact or terrain uncertainty removed a robust continuation.
    ContactUncertainty,
    /// Delivered observations no longer distinguish required hidden states.
    ObservationAliasing,
    /// Sensor or clock evidence expired or became unavailable.
    ObservationEvidenceExpiry,
    /// A shared actuator, power bus, sensor, or other dependency failed.
    SharedDependencyFailure,
    /// The qualified recovery topology changed.
    RecoveryTopologyChange,
    /// A caller-defined cause with an explicit stable identifier.
    Other(String),
}

/// Whether an unavailable option can be restored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reversibility {
    /// No qualified restoration path exists in the declared scope.
    Irreversible,
    /// Restoration is possible only after named conditions are met.
    Conditional {
        /// Required conditions, each bound to an evidence owner.
        required_preconditions: Vec<PreconditionEvidence>,
    },
    /// A temporary condition may clear without physical repair.
    Temporary {
        /// Required conditions for requalification.
        required_preconditions: Vec<PreconditionEvidence>,
    },
}

/// Evidence that one restoration precondition has been met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreconditionEvidence {
    /// Stable precondition identifier.
    pub precondition_id: String,
    /// Digest of the evidence receipt.
    pub receipt_digest: String,
    /// Evidence state.
    pub disposition: EvidenceDisposition,
}

/// Reason why an option becomes available in the result snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdditionCause {
    /// Previously lost capability is restored by thermal cooldown.
    ThermalCooldown,
    /// Stored energy has been replenished.
    Recharge,
    /// A physical fault has been repaired and requalified.
    Repair,
    /// Sensor, actuator, or model qualification has been renewed.
    Requalification,
    /// A fresh qualified observation distinguishes previously aliased states.
    FreshObservation,
    /// A physically independent route has become available.
    RouteChange,
    /// A caller-defined, explicitly identified cause.
    Other(String),
}

/// Resource costs required to restore or newly qualify an option.
///
/// These fields intentionally use separate units and are never summed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RestorationCost {
    /// Bounded restoration time in nanoseconds.
    pub time_ns: u64,
    /// Electrical energy in microjoules.
    pub electrical_energy_microjoules: u64,
    /// Mechanical work in microjoules.
    pub mechanical_work_microjoules: u64,
    /// Compute time in microseconds.
    pub compute_microseconds: u64,
    /// Number of additional qualified observations required.
    pub observation_count: u32,
    /// Signed actuator-authority change in declared fixed-point units.
    pub actuator_authority_delta_units: i64,
}

/// Evidence explaining exactly one changed canonical action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionChangeEvidence {
    /// Evidence for an option removed by this transition.
    Removed {
        /// Canonical action identifier.
        action_id: String,
        /// Cause of removal.
        cause: LossCause,
        /// Whether and how the option may be restored.
        reversibility: Reversibility,
        /// Evidence receipt digest.
        receipt_digest: String,
        /// Evidence state.
        disposition: EvidenceDisposition,
    },
    /// Evidence for an option newly available in the result snapshot.
    Added {
        /// Canonical action identifier.
        action_id: String,
        /// Why it became available.
        cause: AdditionCause,
        /// Exact transition-bound restoration/requalification preconditions.
        preconditions: Vec<PreconditionEvidence>,
        /// Separate bounded resource costs.
        cost: RestorationCost,
        /// State digest from which restoration began.
        predecessor_state_digest: String,
        /// State digest after restoration/requalification.
        result_state_digest: String,
        /// Evidence receipt digest.
        receipt_digest: String,
        /// Evidence state.
        disposition: EvidenceDisposition,
    },
}

/// A qualified loss entry in the append-only option ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionLossEntry {
    /// Canonical action identifier.
    pub action_id: String,
    /// Cause of loss.
    pub cause: LossCause,
    /// Restoration classification.
    pub reversibility: Reversibility,
    /// Evidence receipt digest.
    pub receipt_digest: String,
}

/// A qualified restoration or requalification entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionAdditionEntry {
    /// Canonical action identifier.
    pub action_id: String,
    /// Cause of becoming available.
    pub cause: AdditionCause,
    /// Separate resource costs.
    pub cost: RestorationCost,
    /// Receipt digest.
    pub receipt_digest: String,
}

/// Result of auditing one sequential option-set transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerReport {
    /// Overall audit disposition.
    pub disposition: AuditDisposition,
    /// Prior and resulting snapshot sequence numbers.
    pub predecessor_sequence: u64,
    /// Resulting snapshot sequence number.
    pub result_sequence: u64,
    /// Canonical options removed during this transition.
    pub losses: Vec<OptionLossEntry>,
    /// Canonical options newly available during this transition.
    pub additions: Vec<OptionAdditionEntry>,
    /// Options available in the resulting qualified snapshot.
    pub result_option_ids: Vec<String>,
    /// This retrospective ledger never grants motor authority.
    pub actuation_authorized: bool,
}

/// Input evidence for one transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionEvidence {
    /// State digest immediately before this evidence transition.
    pub predecessor_state_digest: String,
    /// State digest immediately after this evidence transition.
    pub result_state_digest: String,
    /// Optional observation-partition refinement receipt.
    pub observation_partition_change: Option<ObservationPartitionReceipt>,
    /// Exactly one evidence item for each removed or added canonical action.
    pub changes: Vec<OptionChangeEvidence>,
}

/// Structural contradictions found before option changes can be qualified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// Mission, robot, scenario, branch, or action-domain identity changed.
    ScopeMismatch(&'static str),
    /// Mission event order did not advance.
    EventSequenceNotIncreasing,
    /// Physical/reserve/thermal predecessor does not match.
    StatePredecessorMismatch,
    /// Action domain changed between compared snapshots.
    ActionDomainMismatch,
    /// An action identifier is empty or duplicated.
    DuplicateOrEmptyActionId,
    /// An available action is outside the declared finite action domain.
    OptionOutsideActionDomain(String),
    /// Duplicate or unexpected change evidence was supplied.
    DuplicateOrUnexpectedChangeEvidence(String),
    /// The declared change direction contradicts the snapshot set difference.
    ChangeDirectionMismatch(String),
    /// A supplied evidence receipt does not bind to this transition.
    TransitionReceiptBindingMismatch(String),
    /// An irreversible option has restoration preconditions, or vice versa.
    InvalidReversibilityDeclaration(String),
    /// A required receipt digest is absent.
    MissingReceiptDigest(String),
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn unique_ids(ids: &[String]) -> bool {
    ids.iter().all(|id| nonempty(id)) && ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
}

fn set(ids: &[String]) -> BTreeSet<String> {
    ids.iter().cloned().collect()
}

fn disposition_rank(value: EvidenceDisposition) -> u8 {
    match value {
        EvidenceDisposition::Qualified => 0,
        EvidenceDisposition::Unknown => 1,
        EvidenceDisposition::Inconclusive => 2,
        EvidenceDisposition::Rejected => 3,
    }
}

fn aggregate_disposition(values: impl IntoIterator<Item = EvidenceDisposition>) -> AuditDisposition {
    let rank = values.into_iter().map(disposition_rank).max().unwrap_or(0);
    match rank {
        0 => AuditDisposition::Qualified,
        1 => AuditDisposition::Unknown,
        2 => AuditDisposition::Inconclusive,
        _ => AuditDisposition::Rejected,
    }
}

fn validate_snapshot(snapshot: &OptionSnapshot) -> Result<(), LedgerError> {
    if !unique_ids(&snapshot.action_domain_ids) || !unique_ids(&snapshot.available_option_ids) {
        return Err(LedgerError::DuplicateOrEmptyActionId);
    }
    let domain = set(&snapshot.action_domain_ids);
    for action in &snapshot.available_option_ids {
        if !domain.contains(action) {
            return Err(LedgerError::OptionOutsideActionDomain(action.clone()));
        }
    }
    for (name, value) in [
        ("mission_id", snapshot.scope.mission_id.as_str()),
        ("robot_id", snapshot.scope.robot_id.as_str()),
        ("scenario_manifest_digest", snapshot.scope.scenario_manifest_digest.as_str()),
        ("branch_manifest_digest", snapshot.scope.branch_manifest_digest.as_str()),
        ("action_domain_digest", snapshot.scope.action_domain_digest.as_str()),
        ("predecessor_state_digest", snapshot.predecessor_state_digest.as_str()),
        ("result_state_digest", snapshot.result_state_digest.as_str()),
        ("observation_partition_digest", snapshot.observation_partition_digest.as_str()),
    ] {
        if !nonempty(value) {
            return Err(LedgerError::MissingReceiptDigest(name.to_owned()));
        }
    }
    Ok(())
}

fn validate_scope(previous: &OptionSnapshot, next: &OptionSnapshot) -> Result<(), LedgerError> {
    if previous.scope.mission_id != next.scope.mission_id {
        return Err(LedgerError::ScopeMismatch("mission_id"));
    }
    if previous.scope.robot_id != next.scope.robot_id {
        return Err(LedgerError::ScopeMismatch("robot_id"));
    }
    if previous.scope.scenario_manifest_digest != next.scope.scenario_manifest_digest {
        return Err(LedgerError::ScopeMismatch("scenario_manifest_digest"));
    }
    if previous.scope.branch_manifest_digest != next.scope.branch_manifest_digest {
        return Err(LedgerError::ScopeMismatch("branch_manifest_digest"));
    }
    if previous.scope.action_domain_digest != next.scope.action_domain_digest {
        return Err(LedgerError::ScopeMismatch("action_domain_digest"));
    }
    if set(&previous.action_domain_ids) != set(&next.action_domain_ids) {
        return Err(LedgerError::ActionDomainMismatch);
    }
    Ok(())
}

fn validate_reversibility(action_id: &str, reversibility: &Reversibility) -> Result<(), LedgerError> {
    match reversibility {
        Reversibility::Irreversible => Ok(()),
        Reversibility::Conditional { required_preconditions }
        | Reversibility::Temporary { required_preconditions } => {
            if required_preconditions.is_empty() {
                return Err(LedgerError::InvalidReversibilityDeclaration(action_id.to_owned()));
            }
            for precondition in required_preconditions {
                if !nonempty(&precondition.precondition_id) || !nonempty(&precondition.receipt_digest) {
                    return Err(LedgerError::InvalidReversibilityDeclaration(action_id.to_owned()));
                }
            }
            Ok(())
        }
    }
}

/// Audit one transition between two evidence-bearing finite option snapshots.
///
/// The function never infers physical truth. It only checks set differences,
/// identity continuity and the supplied reason-coded transition receipts.
pub fn audit_transition(
    previous: &OptionSnapshot,
    next: &OptionSnapshot,
    evidence: &TransitionEvidence,
) -> Result<LedgerReport, LedgerError> {
    validate_snapshot(previous)?;
    validate_snapshot(next)?;
    validate_scope(previous, next)?;

    if next.event_sequence <= previous.event_sequence {
        return Err(LedgerError::EventSequenceNotIncreasing);
    }
    if next.predecessor_state_digest != previous.result_state_digest {
        return Err(LedgerError::StatePredecessorMismatch);
    }
    if evidence.predecessor_state_digest != previous.result_state_digest
        || evidence.result_state_digest != next.result_state_digest
    {
        return Err(LedgerError::TransitionReceiptBindingMismatch(
            "transition_state".to_owned(),
        ));
    }

    let partition_changed =
        previous.observation_partition_digest != next.observation_partition_digest;
    match (&evidence.observation_partition_change, partition_changed) {
        (Some(receipt), true) => {
            if receipt.predecessor_partition_digest != previous.observation_partition_digest
                || receipt.result_partition_digest != next.observation_partition_digest
                || !nonempty(&receipt.receipt_digest)
            {
                return Err(LedgerError::TransitionReceiptBindingMismatch(
                    "observation_partition".to_owned(),
                ));
            }
        }
        (None, true) => {
            // The state transition may be valid, but its changed observation
            // partition cannot be credited as a qualified capability change.
        }
        (Some(_), false) => {
            return Err(LedgerError::TransitionReceiptBindingMismatch(
                "spurious_observation_partition_receipt".to_owned(),
            ));
        }
        (None, false) => {}
    }

    let previous_set = set(&previous.available_option_ids);
    let next_set = set(&next.available_option_ids);
    let removed: BTreeSet<_> = previous_set.difference(&next_set).cloned().collect();
    let added: BTreeSet<_> = next_set.difference(&previous_set).cloned().collect();

    let mut change_map = BTreeMap::new();
    for change in &evidence.changes {
        let action_id = match change {
            OptionChangeEvidence::Removed { action_id, .. }
            | OptionChangeEvidence::Added { action_id, .. } => action_id,
        };
        if !nonempty(action_id) || change_map.insert(action_id.clone(), change).is_some() {
            return Err(LedgerError::DuplicateOrUnexpectedChangeEvidence(action_id.clone()));
        }
    }

    let expected: BTreeSet<_> = removed.union(&added).cloned().collect();
    for action_id in change_map.keys() {
        if !expected.contains(action_id) {
            return Err(LedgerError::DuplicateOrUnexpectedChangeEvidence(action_id.clone()));
        }
    }

    let mut statuses = vec![previous.disposition, next.disposition];
    if !previous.branch_manifest_complete || !next.branch_manifest_complete
        || !previous.action_domain_complete || !next.action_domain_complete
    {
        statuses.push(EvidenceDisposition::Unknown);
    }

    let mut losses = Vec::new();
    let mut additions = Vec::new();

    for action_id in &removed {
        let Some(change) = change_map.get(action_id) else {
            statuses.push(EvidenceDisposition::Unknown);
            continue;
        };
        match change {
            OptionChangeEvidence::Removed {
                action_id: _,
                cause,
                reversibility,
                receipt_digest,
                disposition,
            } => {
                validate_reversibility(action_id, reversibility)?;
                if !nonempty(receipt_digest) {
                    return Err(LedgerError::MissingReceiptDigest(action_id.clone()));
                }
                statuses.push(*disposition);
                if let Reversibility::Conditional { required_preconditions }
                    | Reversibility::Temporary { required_preconditions } = reversibility
                {
                    statuses.extend(required_preconditions.iter().map(|p| p.disposition));
                }
                losses.push(OptionLossEntry {
                    action_id: action_id.clone(),
                    cause: cause.clone(),
                    reversibility: reversibility.clone(),
                    receipt_digest: receipt_digest.clone(),
                });
            }
            OptionChangeEvidence::Added { .. } => {
                return Err(LedgerError::ChangeDirectionMismatch(action_id.clone()));
            }
        }
    }

    for action_id in &added {
        let Some(change) = change_map.get(action_id) else {
            statuses.push(EvidenceDisposition::Unknown);
            continue;
        };
        match change {
            OptionChangeEvidence::Added {
                action_id: _,
                cause,
                preconditions,
                cost,
                predecessor_state_digest,
                result_state_digest,
                receipt_digest,
                disposition,
            } => {
                if predecessor_state_digest != &previous.result_state_digest
                    || result_state_digest != &next.result_state_digest
                {
                    return Err(LedgerError::TransitionReceiptBindingMismatch(
                        action_id.clone(),
                    ));
                }
                if !nonempty(receipt_digest) {
                    return Err(LedgerError::MissingReceiptDigest(action_id.clone()));
                }
                if preconditions.iter().any(|p| {
                    !nonempty(&p.precondition_id) || !nonempty(&p.receipt_digest)
                }) {
                    return Err(LedgerError::InvalidReversibilityDeclaration(action_id.clone()));
                }
                statuses.push(*disposition);
                statuses.extend(preconditions.iter().map(|p| p.disposition));
                additions.push(OptionAdditionEntry {
                    action_id: action_id.clone(),
                    cause: cause.clone(),
                    cost: *cost,
                    receipt_digest: receipt_digest.clone(),
                });
            }
            OptionChangeEvidence::Removed { .. } => {
                return Err(LedgerError::ChangeDirectionMismatch(action_id.clone()));
            }
        }
    }

    if partition_changed {
        let receipt = evidence.observation_partition_change.as_ref();
        statuses.push(receipt.map_or(EvidenceDisposition::Unknown, |value| value.disposition));
    }

    let disposition = aggregate_disposition(statuses);
    Ok(LedgerReport {
        disposition,
        predecessor_sequence: previous.event_sequence,
        result_sequence: next.event_sequence,
        losses,
        additions,
        result_option_ids: if next.disposition == EvidenceDisposition::Qualified
            && next.branch_manifest_complete
            && next.action_domain_complete
            && disposition == AuditDisposition::Qualified
        {
            let mut ids = next.available_option_ids.clone();
            ids.sort();
            ids
        } else {
            Vec::new()
        },
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ComparisonScope {
        ComparisonScope {
            mission_id: "mission-1".into(),
            robot_id: "robot-1".into(),
            scenario_manifest_digest: "scenario-a".into(),
            branch_manifest_digest: "branches-a".into(),
            action_domain_digest: "actions-a".into(),
        }
    }

    fn snapshot(
        sequence: u64,
        predecessor: &str,
        result: &str,
        options: &[&str],
    ) -> OptionSnapshot {
        OptionSnapshot {
            scope: scope(),
            event_sequence: sequence,
            predecessor_state_digest: predecessor.into(),
            result_state_digest: result.into(),
            observation_partition_digest: "observation-a".into(),
            action_domain_ids: vec!["retreat".into(), "step-left".into(), "step-right".into()],
            available_option_ids: options.iter().map(|value| (*value).to_owned()).collect(),
            disposition: EvidenceDisposition::Qualified,
            branch_manifest_complete: true,
            action_domain_complete: true,
        }
    }

    fn remove(action_id: &str, cause: LossCause, reversibility: Reversibility) -> OptionChangeEvidence {
        OptionChangeEvidence::Removed {
            action_id: action_id.into(),
            cause,
            reversibility,
            receipt_digest: "loss-receipt".into(),
            disposition: EvidenceDisposition::Qualified,
        }
    }

    fn add(action_id: &str, previous: &OptionSnapshot, next: &OptionSnapshot) -> OptionChangeEvidence {
        OptionChangeEvidence::Added {
            action_id: action_id.into(),
            cause: AdditionCause::ThermalCooldown,
            preconditions: vec![PreconditionEvidence {
                precondition_id: "cooldown-verified".into(),
                receipt_digest: "cooldown-receipt".into(),
                disposition: EvidenceDisposition::Qualified,
            }],
            cost: RestorationCost {
                time_ns: 1_000_000,
                electrical_energy_microjoules: 500,
                mechanical_work_microjoules: 200,
                compute_microseconds: 100,
                observation_count: 2,
                actuator_authority_delta_units: 15,
            },
            predecessor_state_digest: previous.result_state_digest.clone(),
            result_state_digest: next.result_state_digest.clone(),
            receipt_digest: "restoration-receipt".into(),
            disposition: EvidenceDisposition::Qualified,
        }
    }

    fn transition(
        previous: &OptionSnapshot,
        next: &OptionSnapshot,
        changes: Vec<OptionChangeEvidence>,
    ) -> TransitionEvidence {
        TransitionEvidence {
            predecessor_state_digest: previous.result_state_digest.clone(),
            result_state_digest: next.result_state_digest.clone(),
            observation_partition_change: None,
            changes,
        }
    }

    #[test]
    fn qualified_loss_is_recorded_with_cause_and_reversibility() {
        let previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        let report = audit_transition(
            &previous,
            &next,
            &transition(&previous, &next, vec![remove(
                "step-left",
                LossCause::ThermalDerating,
                Reversibility::Conditional {
                    required_preconditions: vec![PreconditionEvidence {
                        precondition_id: "motor-below-requal-temp".into(),
                        receipt_digest: "temperature-receipt".into(),
                        disposition: EvidenceDisposition::Qualified,
                    }],
                },
            )]),
        )
        .expect("valid transition");
        assert_eq!(report.disposition, AuditDisposition::Qualified);
        assert_eq!(report.losses.len(), 1);
        assert_eq!(report.losses[0].action_id, "step-left");
        assert!(!report.actuation_authorized);
    }

    #[test]
    fn missing_loss_cause_remains_unknown() {
        let previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![])).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Unknown);
        assert!(report.result_option_ids.is_empty());
    }

    #[test]
    fn unknown_loss_evidence_never_qualifies_the_ledger() {
        let previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        let mut evidence = remove(
            "step-left",
            LossCause::ObservationEvidenceExpiry,
            Reversibility::Temporary {
                required_preconditions: vec![PreconditionEvidence {
                    precondition_id: "fresh-calibrated-observation".into(),
                    receipt_digest: "pending".into(),
                    disposition: EvidenceDisposition::Unknown,
                }],
            },
        );
        if let OptionChangeEvidence::Removed { disposition, .. } = &mut evidence {
            *disposition = EvidenceDisposition::Unknown;
        }
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![evidence])).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Unknown);
        assert!(report.result_option_ids.is_empty());
    }

    #[test]
    fn duplicate_change_evidence_is_rejected() {
        let previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        let evidence = transition(vec![
            remove("step-left", LossCause::EnergyDepletion, Reversibility::Irreversible),
            remove("step-left", LossCause::EnergyDepletion, Reversibility::Irreversible),
        ]);
        assert!(matches!(
            audit_transition(&previous, &next, &evidence),
            Err(LedgerError::DuplicateOrUnexpectedChangeEvidence(_))
        ));
    }

    #[test]
    fn predecessor_state_reset_is_rejected() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "unrelated", "s2", &["retreat"]);
        assert_eq!(
            audit_transition(&previous, &next, &transition(&previous, &next, vec![])),
            Err(LedgerError::StatePredecessorMismatch)
        );
    }

    #[test]
    fn changed_branch_manifest_is_rejected() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let mut next = snapshot(2, "s1", "s2", &["retreat"]);
        next.scope.branch_manifest_digest = "branches-b".into();
        assert_eq!(
            audit_transition(&previous, &next, &transition(&previous, &next, vec![])),
            Err(LedgerError::ScopeMismatch("branch_manifest_digest"))
        );
    }

    #[test]
    fn changed_action_domain_is_rejected() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let mut next = snapshot(2, "s1", "s2", &["retreat"]);
        next.action_domain_ids.push("crawl".into());
        assert_eq!(
            audit_transition(&previous, &next, &transition(&previous, &next, vec![])),
            Err(LedgerError::ActionDomainMismatch)
        );
    }

    #[test]
    fn option_outside_domain_is_rejected() {
        let mut previous = snapshot(1, "s0", "s1", &["retreat"]);
        previous.available_option_ids.push("teleport".into());
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        assert_eq!(
            audit_transition(&previous, &next, &transition(&previous, &next, vec![])),
            Err(LedgerError::OptionOutsideActionDomain("teleport".into()))
        );
    }

    #[test]
    fn incomplete_branch_or_domain_manifest_is_unknown() {
        let mut previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        previous.branch_manifest_complete = false;
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        let report = audit_transition(
            &previous,
            &next,
            &transition(&previous, &next, vec![remove("step-left", LossCause::ContactUncertainty, Reversibility::Irreversible)]),
        )
        .unwrap();
        assert_eq!(report.disposition, AuditDisposition::Unknown);
        assert!(report.result_option_ids.is_empty());
    }

    #[test]
    fn event_sequence_must_increase() {
        let previous = snapshot(4, "s0", "s1", &["retreat"]);
        let next = snapshot(4, "s1", "s2", &["retreat"]);
        assert_eq!(
            audit_transition(&previous, &next, &transition(&previous, &next, vec![])),
            Err(LedgerError::EventSequenceNotIncreasing)
        );
    }

    #[test]
    fn qualified_restoration_binds_exact_state_transition() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "s1", "s2", &["retreat", "step-left"]);
        let report = audit_transition(
            &previous,
            &next,
            &transition(&previous, &next, vec![add("step-left", &previous, &next)]),
        )
        .unwrap();
        assert_eq!(report.disposition, AuditDisposition::Qualified);
        assert_eq!(report.additions.len(), 1);
        assert_eq!(report.additions[0].cost.electrical_energy_microjoules, 500);
        assert_eq!(report.additions[0].cost.mechanical_work_microjoules, 200);
    }

    #[test]
    fn restoration_receipt_with_wrong_predecessor_is_rejected() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "s1", "s2", &["retreat", "step-left"]);
        let mut evidence = add("step-left", &previous, &next);
        if let OptionChangeEvidence::Added { predecessor_state_digest, .. } = &mut evidence {
            *predecessor_state_digest = "stale-state".into();
        }
        assert!(matches!(
            audit_transition(&previous, &next, &transition(&previous, &next, vec![evidence])),
            Err(LedgerError::TransitionReceiptBindingMismatch(_))
        ));
    }

    #[test]
    fn missing_restoration_receipt_is_unknown() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "s1", "s2", &["retreat", "step-left"]);
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![])).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Unknown);
        assert!(report.result_option_ids.is_empty());
    }

    #[test]
    fn unknown_restoration_precondition_does_not_restore_option() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "s1", "s2", &["retreat", "step-left"]);
        let mut evidence = add("step-left", &previous, &next);
        if let OptionChangeEvidence::Added { preconditions, .. } = &mut evidence {
            preconditions[0].disposition = EvidenceDisposition::Unknown;
        }
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![evidence])).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Unknown);
        assert!(report.result_option_ids.is_empty());
    }

    #[test]
    fn changed_observation_partition_requires_a_qualified_receipt() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let mut next = snapshot(2, "s1", "s2", &["retreat"]);
        next.observation_partition_digest = "observation-b".into();
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![])).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Unknown);
    }

    #[test]
    fn observation_refinement_is_bound_to_both_partition_digests() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let mut next = snapshot(2, "s1", "s2", &["retreat"]);
        next.observation_partition_digest = "observation-b".into();
        let evidence = TransitionEvidence {
            predecessor_state_digest: previous.result_state_digest.clone(),
            result_state_digest: next.result_state_digest.clone(),
            observation_partition_change: Some(ObservationPartitionReceipt {
                predecessor_partition_digest: "observation-a".into(),
                result_partition_digest: "observation-b".into(),
                receipt_digest: "partition-receipt".into(),
                disposition: EvidenceDisposition::Qualified,
            }),
            changes: vec![],
        };
        let report = audit_transition(&previous, &next, &evidence).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Qualified);
    }

    #[test]
    fn input_order_does_not_change_canonical_report() {
        let previous_a = snapshot(1, "s0", "s1", &["retreat", "step-left", "step-right"]);
        let next_a = snapshot(2, "s1", "s2", &["retreat"]);
        let mut previous_b = snapshot(1, "s0", "s1", &["step-right", "retreat", "step-left"]);
        let mut next_b = snapshot(2, "s1", "s2", &["retreat"]);
        previous_b.action_domain_ids.reverse();
        next_b.action_domain_ids.reverse();
        let changes_a = vec![
            remove("step-left", LossCause::ThermalDerating, Reversibility::Irreversible),
            remove("step-right", LossCause::EnergyDepletion, Reversibility::Irreversible),
        ];
        let changes_b = vec![
            remove("step-right", LossCause::EnergyDepletion, Reversibility::Irreversible),
            remove("step-left", LossCause::ThermalDerating, Reversibility::Irreversible),
        ];
        let a = audit_transition(&previous_a, &next_a, &transition(&previous_a, &next_a, changes_a)).unwrap();
        let b = audit_transition(&previous_b, &next_b, &transition(&previous_b, &next_b, changes_b)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn event_order_changes_predecessor_binding() {
        let first = snapshot(10, "s9", "s10", &["retreat"]);
        let second = snapshot(9, "s8", "s9", &["retreat"]);
        assert!(matches!(
            audit_transition(&first, &second, &transition(&first, &second, vec![])),
            Err(LedgerError::EventSequenceNotIncreasing)
        ));
    }

    #[test]
    fn irreversible_loss_cannot_claim_restoration_conditions() {
        let previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        let mut change = remove(
            "step-left",
            LossCause::ActuatorAuthorityLoss,
            Reversibility::Irreversible,
        );
        if let OptionChangeEvidence::Removed { reversibility, .. } = &mut change {
            *reversibility = Reversibility::Irreversible;
        }
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![change])).unwrap();
        assert_eq!(report.losses[0].reversibility, Reversibility::Irreversible);
    }

    #[test]
    fn conditional_loss_requires_preconditions() {
        let previous = snapshot(1, "s0", "s1", &["retreat", "step-left"]);
        let next = snapshot(2, "s1", "s2", &["retreat"]);
        assert!(matches!(
            audit_transition(
                &previous,
                &next,
                &transition(&previous, &next, vec![remove(
                    "step-left",
                    LossCause::ThermalDerating,
                    Reversibility::Conditional { required_preconditions: vec![] },
                )]),
            ),
            Err(LedgerError::InvalidReversibilityDeclaration(_))
        ));
    }

    #[test]
    fn rejected_precondition_rejects_overall_transition() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "s1", "s2", &["retreat", "step-left"]);
        let mut evidence = add("step-left", &previous, &next);
        if let OptionChangeEvidence::Added { preconditions, .. } = &mut evidence {
            preconditions[0].disposition = EvidenceDisposition::Rejected;
        }
        let report = audit_transition(&previous, &next, &transition(&previous, &next, vec![evidence])).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Rejected);
        assert!(report.result_option_ids.is_empty());
    }

    #[test]
    fn resource_cost_dimensions_are_not_collapsed() {
        let previous = snapshot(1, "s0", "s1", &["retreat"]);
        let next = snapshot(2, "s1", "s2", &["retreat", "step-left"]);
        let report = audit_transition(
            &previous,
            &next,
            &transition(&previous, &next, vec![add("step-left", &previous, &next)]),
        )
        .unwrap();
        let cost = report.additions[0].cost;
        assert_eq!(cost.time_ns, 1_000_000);
        assert_eq!(cost.electrical_energy_microjoules, 500);
        assert_eq!(cost.mechanical_work_microjoules, 200);
        assert_eq!(cost.compute_microseconds, 100);
        assert_eq!(cost.observation_count, 2);
        assert_eq!(cost.actuator_authority_delta_units, 15);
    }
}
