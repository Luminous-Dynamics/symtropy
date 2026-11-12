#![forbid(unsafe_code)]

//! Evidence contract for material-preserving textile attachment removal.
//!
//! This crate checks task/attempt semantics and evidence ceilings. It does not
//! plan robot motion, control tools, authenticate evidence sources, or qualify
//! a downstream recycling route.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinMethod {
    Sewn,
    BarTacked,
    Riveted,
    Zippered,
    Buttoned,
    SnapFastened,
    HookAndLoop,
    AdhesiveBonded,
    ThermallyFused,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestedAction {
    Remove,
    Retain,
    Replace,
    Inspect,
    ManualReview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionState {
    Planned,
    Executed,
    Aborted,
    Abstained,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemovalOutcome {
    NotAttempted,
    Removed,
    Partial,
    NotRemoved,
    Aborted,
    Abstained,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceTier {
    SyntheticFixture,
    Simulation,
    PhysicalObserved,
    IndependentlyInspected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceCeiling {
    ContractOnly,
    SimulationOnly,
    PhysicalExecutionObserved,
    PhysicalRemovalIndependentlyInspected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub sha256: String,
    pub tier: EvidenceTier,
    pub calibration_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSpec {
    pub task_id: String,
    pub specimen_id: String,
    pub garment_revision: String,
    pub attachment_id: String,
    pub join_method: JoinMethod,
    pub requested_action: RequestedAction,
    pub tool_profile_id: String,
    pub max_force_mn: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MassBalance {
    /// Mass of the specimen region entering this removal attempt.
    pub input_mass_mg: u64,
    /// Textile mass recovered in the intended textile output stream.
    pub textile_recovered_mass_mg: u64,
    /// Detached attachment mass recovered as a separate component.
    pub attachment_recovered_mass_mg: u64,
    /// Measured residual material retained with the specimen or reject.
    pub residual_mass_mg: u64,
    /// Explicitly measured/unresolved mass not assigned to another output.
    pub unexplained_mass_mg: u64,
}

impl MassBalance {
    pub fn is_closed(&self) -> bool {
        self.textile_recovered_mass_mg
            .checked_add(self.attachment_recovered_mass_mg)
            .and_then(|sum| sum.checked_add(self.residual_mass_mg))
            .and_then(|sum| sum.checked_add(self.unexplained_mass_mg))
            == Some(self.input_mass_mg)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageObservation {
    pub damage_area_mm2: u64,
    pub severed_yarns: u64,
    pub holes: u64,
    pub tears: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptRecord {
    pub attempt_id: String,
    pub task: TaskSpec,
    pub execution_state: ExecutionState,
    pub outcome: RemovalOutcome,
    pub action_evidence: Option<EvidenceRef>,
    pub post_inspection: Option<EvidenceRef>,
    pub peak_force_mn: Option<u64>,
    pub force_calibration_id: Option<String>,
    pub mass_balance: Option<MassBalance>,
    pub damage: Option<DamageObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub valid: bool,
    pub evidence_ceiling: EvidenceCeiling,
    pub findings: Vec<Finding>,
}

fn add(
    findings: &mut Vec<Finding>,
    code: &'static str,
    message: impl Into<String>,
) {
    findings.push(Finding {
        code,
        message: message.into(),
    });
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn has_positive_recovered_mass(mass: &MassBalance) -> bool {
    mass.textile_recovered_mass_mg > 0 || mass.attachment_recovered_mass_mg > 0
}

fn evidence_ceiling(record: &AttemptRecord) -> EvidenceCeiling {
    match (
        record.action_evidence.as_ref().map(|item| item.tier),
        record.post_inspection.as_ref().map(|item| item.tier),
        record.outcome,
    ) {
        (
            Some(EvidenceTier::PhysicalObserved),
            Some(EvidenceTier::IndependentlyInspected),
            RemovalOutcome::Removed,
        ) if record.mass_balance.is_some() && record.damage.is_some() => {
            EvidenceCeiling::PhysicalRemovalIndependentlyInspected
        }
        (Some(EvidenceTier::PhysicalObserved), _, _) => {
            EvidenceCeiling::PhysicalExecutionObserved
        }
        (Some(EvidenceTier::Simulation), _, _) => EvidenceCeiling::SimulationOnly,
        _ => EvidenceCeiling::ContractOnly,
    }
}

/// Validate a task attempt while preserving the difference between planned,
/// simulated, physically observed and independently inspected outcomes.
///
/// A valid report does not certify material purity, recovery eligibility,
/// production safety, worker safety, or downstream recycling acceptance.
pub fn validate(record: &AttemptRecord) -> ValidationReport {
    let mut findings = Vec::new();

    for (field, value) in [
        ("attempt_id", record.attempt_id.as_str()),
        ("task_id", record.task.task_id.as_str()),
        ("specimen_id", record.task.specimen_id.as_str()),
        ("garment_revision", record.task.garment_revision.as_str()),
        ("attachment_id", record.task.attachment_id.as_str()),
        ("tool_profile_id", record.task.tool_profile_id.as_str()),
    ] {
        if !nonempty(value) {
            add(
                &mut findings,
                "MISSING_IDENTITY",
                format!("{field} must be non-empty"),
            );
        }
    }

    for evidence in [
        record.action_evidence.as_ref(),
        record.post_inspection.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !nonempty(&evidence.evidence_id) {
            add(
                &mut findings,
                "MISSING_EVIDENCE_ID",
                "evidence identifier must be non-empty",
            );
        }
        if !valid_sha256(&evidence.sha256) {
            add(
                &mut findings,
                "INVALID_EVIDENCE_DIGEST",
                format!("evidence {} must carry a 64-character SHA-256 digest", evidence.evidence_id),
            );
        }
        if evidence.tier == EvidenceTier::PhysicalObserved
            && evidence.calibration_id.as_deref().is_none_or(|id| !nonempty(id))
            && record.action_evidence.as_ref().is_some_and(|action| {
                action.evidence_id == evidence.evidence_id
            })
        {
            add(
                &mut findings,
                "MISSING_ACTION_CALIBRATION",
                "physical action evidence must identify its calibration profile",
            );
        }
    }

    if record.task.join_method == JoinMethod::Unknown
        && record.execution_state == ExecutionState::Executed
    {
        add(
            &mut findings,
            "UNKNOWN_JOIN_METHOD_MUST_ABSTAIN",
            "unknown attachment construction cannot proceed as an executed removal task",
        );
    }

    match record.execution_state {
        ExecutionState::Planned => {
            if record.outcome != RemovalOutcome::NotAttempted {
                add(
                    &mut findings,
                    "PLANNED_OUTCOME_MISMATCH",
                    "planned work must use NotAttempted outcome",
                );
            }
            if record.action_evidence.is_some() || record.post_inspection.is_some() {
                add(
                    &mut findings,
                    "PLANNED_HAS_EXECUTION_EVIDENCE",
                    "planned work cannot claim action or post-inspection evidence",
                );
            }
        }
        ExecutionState::Abstained => {
            if record.outcome != RemovalOutcome::Abstained {
                add(
                    &mut findings,
                    "ABSTENTION_OUTCOME_MISMATCH",
                    "an abstained task must use Abstained outcome",
                );
            }
            if record.action_evidence.is_some() {
                add(
                    &mut findings,
                    "ABSTENTION_HAS_ACTION_EVIDENCE",
                    "an abstained task cannot claim an executed action",
                );
            }
        }
        ExecutionState::Aborted => {
            if record.outcome != RemovalOutcome::Aborted {
                add(
                    &mut findings,
                    "ABORT_OUTCOME_MISMATCH",
                    "an aborted task must use Aborted outcome",
                );
            }
        }
        ExecutionState::Executed => {
            if !matches!(
                record.outcome,
                RemovalOutcome::Removed
                    | RemovalOutcome::Partial
                    | RemovalOutcome::NotRemoved
            ) {
                add(
                    &mut findings,
                    "EXECUTED_OUTCOME_MISMATCH",
                    "executed work must be Removed, Partial or NotRemoved",
                );
            }
            if record.action_evidence.is_none() {
                add(
                    &mut findings,
                    "MISSING_ACTION_EVIDENCE",
                    "executed work requires an action evidence reference",
                );
            }
            if record.post_inspection.is_none() {
                add(
                    &mut findings,
                    "MISSING_POST_INSPECTION",
                    "executed work requires a post-attempt inspection, including NotRemoved outcomes",
                );
            }
            if record
                .action_evidence
                .as_ref()
                .is_some_and(|item| item.tier == EvidenceTier::SyntheticFixture)
            {
                add(
                    &mut findings,
                    "SYNTHETIC_EVIDENCE_CANNOT_PROVE_EXECUTION",
                    "a synthetic fixture cannot establish an executed action",
                );
            }
        }
    }

    if matches!(
        record.outcome,
        RemovalOutcome::Removed | RemovalOutcome::Partial
    ) {
        if record.task.requested_action != RequestedAction::Remove {
            add(
                &mut findings,
                "UNREQUESTED_REMOVAL",
                "Removed or Partial outcome requires a Remove task request",
            );
        }
        if record.mass_balance.is_none() {
            add(
                &mut findings,
                "MISSING_MASS_BALANCE",
                "Removed or Partial outcome requires a mass-balance record",
            );
        }
        if record.post_inspection.is_none() {
            add(
                &mut findings,
                "MISSING_POST_INSPECTION",
                "Removed or Partial outcome requires post-removal inspection evidence",
            );
        }
    }

    if let Some(mass) = &record.mass_balance {
        if !mass.is_closed() {
            add(
                &mut findings,
                "MASS_BALANCE_NOT_CLOSED",
                "input mass must equal textile output + attachment output + residual + unexplained mass",
            );
        }
        if matches!(
            record.outcome,
            RemovalOutcome::Removed | RemovalOutcome::Partial
        ) && mass.attachment_recovered_mass_mg == 0
        {
            add(
                &mut findings,
                "REMOVAL_OUTCOME_WITHOUT_ATTACHMENT_MASS",
                "Removed or Partial outcome requires positive recovered attachment mass",
            );
        }
        if matches!(
            record.outcome,
            RemovalOutcome::NotRemoved
                | RemovalOutcome::Aborted
                | RemovalOutcome::Abstained
                | RemovalOutcome::NotAttempted
        ) && mass.attachment_recovered_mass_mg > 0
        {
            add(
                &mut findings,
                "DETACHED_MASS_WITHOUT_REMOVAL_OUTCOME",
                "positive attachment output mass conflicts with the declared outcome",
            );
        }
        if matches!(
            record.execution_state,
            ExecutionState::Planned | ExecutionState::Abstained
        ) && has_positive_recovered_mass(mass)
        {
            add(
                &mut findings,
                "UNEXECUTED_POSITIVE_OUTPUT",
                "planned or abstained work cannot claim recovered physical output",
            );
        }
    }

    if let Some(action) = &record.action_evidence {
        match action.tier {
            EvidenceTier::PhysicalObserved => {
                if record.peak_force_mn.is_none() {
                    add(
                        &mut findings,
                        "MISSING_PEAK_FORCE",
                        "physical action requires a measured peak-force observation",
                    );
                }
                if record
                    .force_calibration_id
                    .as_deref()
                    .is_none_or(|id| !nonempty(id))
                {
                    add(
                        &mut findings,
                        "MISSING_FORCE_CALIBRATION",
                        "physical peak force must bind to a calibration profile",
                    );
                }
                if matches!(
                    record.outcome,
                    RemovalOutcome::Removed | RemovalOutcome::Partial
                ) {
                    if !record.post_inspection.as_ref().is_some_and(|inspection| {
                        inspection.tier == EvidenceTier::IndependentlyInspected
                    }) {
                        add(
                            &mut findings,
                            "PHYSICAL_REMOVAL_NEEDS_INDEPENDENT_INSPECTION",
                            "physical Removed/Partial outcome requires independent physical post-inspection",
                        );
                    }
                    if record.damage.is_none() {
                        add(
                            &mut findings,
                            "MISSING_TEXTILE_DAMAGE_OBSERVATION",
                            "physical Removed/Partial outcome requires a textile-damage observation",
                        );
                    }
                }
            }
            EvidenceTier::Simulation => {
                if record
                    .post_inspection
                    .as_ref()
                    .is_some_and(|inspection| {
                        inspection.tier == EvidenceTier::IndependentlyInspected
                    })
                {
                    add(
                        &mut findings,
                        "SIMULATION_PROMOTED_TO_PHYSICAL_INSPECTION",
                        "simulation action evidence cannot be promoted by a physical inspection label",
                    );
                }
            }
            EvidenceTier::SyntheticFixture => {}
            EvidenceTier::IndependentlyInspected => {
                add(
                    &mut findings,
                    "INSPECTION_TIER_USED_AS_ACTION",
                    "independent inspection evidence cannot substitute for action evidence",
                );
            }
        }
    }

    if let (Some(maximum), Some(peak)) = (record.task.max_force_mn, record.peak_force_mn) {
        if peak > maximum {
            add(
                &mut findings,
                "FORCE_LIMIT_EXCEEDED",
                format!(
                    "peak force {peak} mN exceeds declared maximum {maximum} mN"
                ),
            );
        }
    }

    findings.sort_by(|left, right| {
        (&left.code, &left.message).cmp(&(&right.code, &right.message))
    });

    let valid = findings.is_empty();
    let ceiling = if valid {
        evidence_ceiling(record)
    } else {
        EvidenceCeiling::ContractOnly
    };

    ValidationReport {
        valid,
        evidence_ceiling: ceiling,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, tier: EvidenceTier, calibration_id: Option<&str>) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.to_owned(),
            sha256: "a".repeat(64),
            tier,
            calibration_id: calibration_id.map(str::to_owned),
        }
    }

    fn physical_removed() -> AttemptRecord {
        AttemptRecord {
            attempt_id: "attempt-001".to_owned(),
            task: TaskSpec {
                task_id: "task-001".to_owned(),
                specimen_id: "specimen-001".to_owned(),
                garment_revision: "overshirt-r1".to_owned(),
                attachment_id: "care-label-001".to_owned(),
                join_method: JoinMethod::Sewn,
                requested_action: RequestedAction::Remove,
                tool_profile_id: "guarded-seam-ripper-v1".to_owned(),
                max_force_mn: Some(12_000),
            },
            execution_state: ExecutionState::Executed,
            outcome: RemovalOutcome::Removed,
            action_evidence: Some(evidence(
                "physical-action-001",
                EvidenceTier::PhysicalObserved,
                Some("force-cal-2026-01"),
            )),
            post_inspection: Some(evidence(
                "inspection-001",
                EvidenceTier::IndependentlyInspected,
                None,
            )),
            peak_force_mn: Some(7_500),
            force_calibration_id: Some("force-cal-2026-01".to_owned()),
            mass_balance: Some(MassBalance {
                input_mass_mg: 10_000,
                textile_recovered_mass_mg: 7_000,
                attachment_recovered_mass_mg: 1_000,
                residual_mass_mg: 1_500,
                unexplained_mass_mg: 500,
            }),
            damage: Some(DamageObservation {
                damage_area_mm2: 0,
                severed_yarns: 0,
                holes: 0,
                tears: 0,
            }),
        }
    }

    fn simulated_removed() -> AttemptRecord {
        let mut record = physical_removed();
        record.action_evidence = Some(evidence(
            "sim-action-001",
            EvidenceTier::Simulation,
            None,
        ));
        record.post_inspection = Some(evidence(
            "sim-inspection-001",
            EvidenceTier::Simulation,
            None,
        ));
        record.peak_force_mn = None;
        record.force_calibration_id = None;
        record.damage = None;
        record
    }

    #[test]
    fn independently_inspected_physical_removal_is_valid() {
        let report = validate(&physical_removed());
        assert!(report.valid, "{:#?}", report.findings);
        assert_eq!(
            report.evidence_ceiling,
            EvidenceCeiling::PhysicalRemovalIndependentlyInspected
        );
    }

    #[test]
    fn simulated_removal_remains_simulation_only() {
        let report = validate(&simulated_removed());
        assert!(report.valid, "{:#?}", report.findings);
        assert_eq!(report.evidence_ceiling, EvidenceCeiling::SimulationOnly);
    }

    #[test]
    fn unknown_join_method_must_abstain() {
        let mut record = simulated_removed();
        record.task.join_method = JoinMethod::Unknown;
        let report = validate(&record);
        assert!(!report.valid);
        assert!(report.findings.iter().any(|item| {
            item.code == "UNKNOWN_JOIN_METHOD_MUST_ABSTAIN"
        }));
    }

    #[test]
    fn unknown_join_method_abstention_is_valid_without_claiming_output() {
        let mut record = simulated_removed();
        record.task.join_method = JoinMethod::Unknown;
        record.execution_state = ExecutionState::Abstained;
        record.outcome = RemovalOutcome::Abstained;
        record.action_evidence = None;
        record.post_inspection = None;
        record.peak_force_mn = None;
        record.force_calibration_id = None;
        record.mass_balance = None;
        record.damage = None;
        let report = validate(&record);
        assert!(report.valid, "{:#?}", report.findings);
        assert_eq!(report.evidence_ceiling, EvidenceCeiling::ContractOnly);
    }

    #[test]
    fn non_closed_mass_balance_is_rejected() {
        let mut record = physical_removed();
        record.mass_balance.as_mut().unwrap().unexplained_mass_mg = 0;
        let report = validate(&record);
        assert!(report.findings.iter().any(|item| {
            item.code == "MASS_BALANCE_NOT_CLOSED"
        }));
    }

    #[test]
    fn physical_removal_requires_independent_inspection_and_damage_measurement() {
        let mut record = physical_removed();
        record.post_inspection = Some(evidence(
            "not-independent",
            EvidenceTier::PhysicalObserved,
            Some("inspection-cal-01"),
        ));
        record.damage = None;
        let report = validate(&record);
        assert!(report.findings.iter().any(|item| {
            item.code == "PHYSICAL_REMOVAL_NEEDS_INDEPENDENT_INSPECTION"
        }));
        assert!(report.findings.iter().any(|item| {
            item.code == "MISSING_TEXTILE_DAMAGE_OBSERVATION"
        }));
    }

    #[test]
    fn synthetic_fixture_cannot_claim_executed_removal() {
        let mut record = simulated_removed();
        record.action_evidence = Some(evidence(
            "synthetic-action-001",
            EvidenceTier::SyntheticFixture,
            None,
        ));
        let report = validate(&record);
        assert!(report.findings.iter().any(|item| {
            item.code == "SYNTHETIC_EVIDENCE_CANNOT_PROVE_EXECUTION"
        }));
    }

    #[test]
    fn force_limit_violation_is_retained_as_a_hard_finding() {
        let mut record = physical_removed();
        record.peak_force_mn = Some(12_001);
        let report = validate(&record);
        assert!(report.findings.iter().any(|item| {
            item.code == "FORCE_LIMIT_EXCEEDED"
        }));
    }

    #[test]
    fn planned_work_cannot_claim_recovered_output() {
        let mut record = simulated_removed();
        record.execution_state = ExecutionState::Planned;
        record.outcome = RemovalOutcome::NotAttempted;
        record.action_evidence = None;
        record.post_inspection = None;
        let report = validate(&record);
        assert!(!report.findings.iter().any(|item| {
            item.code == "PLANNED_OUTCOME_MISMATCH"
        }));
        assert!(report.findings.iter().any(|item| {
            item.code == "UNEXECUTED_POSITIVE_OUTPUT"
        }));
    }

    #[test]
    fn simulated_result_cannot_be_promoted_by_inspection_label() {
        let mut record = simulated_removed();
        record.post_inspection = Some(evidence(
            "inspection-claim-001",
            EvidenceTier::IndependentlyInspected,
            None,
        ));
        let report = validate(&record);
        assert!(report.findings.iter().any(|item| {
            item.code == "SIMULATION_PROMOTED_TO_PHYSICAL_INSPECTION"
        }));
    }
}
