//! Dependency-free composition gate for maritime evidence-to-decision bundles.
//!
//! This crate owns only cross-layer composition. It does not implement source
//! capture, source authentication, physical feasibility, authority, custody,
//! statistical calibration, or operational execution. Upstream dispositions
//! are preserved and cannot be promoted by a downstream pass.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateState {
    Pass,
    Fail,
    Denied,
    Infeasible,
    NotAuthorized,
    Unknown,
    Incomplete,
    NotIdentifiable,
    NotEvaluable,
}

impl GateState {
    fn rejects(self) -> bool {
        matches!(
            self,
            Self::Fail | Self::Denied | Self::Infeasible | Self::NotAuthorized
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GateCheck {
    pub state: GateState,
    pub reason: &'static str,
}

impl GateCheck {
    pub const fn pass() -> Self {
        Self {
            state: GateState::Pass,
            reason: "ALL_REQUIRED_BINDINGS_MATCH",
        }
    }

    pub const fn blocked(state: GateState, reason: &'static str) -> Self {
        Self { state, reason }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionDisposition {
    EligibleForDecisionReview,
    Abstain,
    Reject,
}

impl DecisionDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EligibleForDecisionReview => "eligible_for_decision_review",
            Self::Abstain => "abstain",
            Self::Reject => "reject",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Blocker {
    pub field: &'static str,
    pub state: GateState,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    pub disposition: DecisionDisposition,
    pub blockers: Vec<Blocker>,
}

impl Evaluation {
    pub fn primary_reason(&self) -> &'static str {
        self.blockers
            .first()
            .map_or("ALL_REQUIRED_BINDINGS_MATCH", |blocker| blocker.reason)
    }
}

/// The cross-layer fields required for a narrow decision-review claim.
///
/// Every field represents a disposition already produced by its owning
/// subsystem. The reducer checks composition; it does not reinterpret the
/// upstream evidence or infer that an upstream owner is truthful.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceBundle {
    pub source_rights: GateCheck,
    pub as_of_visibility: GateCheck,
    pub capture_completeness: GateCheck,
    pub source_claim_admission: GateCheck,
    pub subject_binding: GateCheck,
    pub time_binding: GateCheck,
    pub source_integrity: GateCheck,
    pub oracle_independence: GateCheck,
    pub oracle_implementation: GateCheck,
    pub episode_binding: GateCheck,
    pub physical_feasibility: GateCheck,
    pub authority: GateCheck,
    pub participant_service_floor: GateCheck,
    pub custody_closure: GateCheck,
    pub receiver_acceptance: GateCheck,
    pub dependence_unit: GateCheck,
    pub calibration_evaluation_split: GateCheck,
    pub frozen_candidate_grid: GateCheck,
    pub outcome_completeness: GateCheck,
    pub cargo_conservation: GateCheck,
    pub statistical_guarantee: GateCheck,
    pub observation_freshness: GateCheck,
    pub observation_accounting: GateCheck,
    pub historical_as_of_integrity: GateCheck,
    pub feasible_authorized_action: GateCheck,
}

impl EvidenceBundle {
    /// Constructs a synthetic all-gates-pass fixture. This is not a real-world
    /// default and must never be used as a source of operational evidence.
    pub fn synthetic_valid_fixture() -> Self {
        let pass = GateCheck::pass();
        Self {
            source_rights: pass,
            as_of_visibility: pass,
            capture_completeness: pass,
            source_claim_admission: pass,
            subject_binding: pass,
            time_binding: pass,
            source_integrity: pass,
            oracle_independence: pass,
            oracle_implementation: pass,
            episode_binding: pass,
            physical_feasibility: pass,
            authority: pass,
            participant_service_floor: pass,
            custody_closure: pass,
            receiver_acceptance: pass,
            dependence_unit: pass,
            calibration_evaluation_split: pass,
            frozen_candidate_grid: pass,
            outcome_completeness: pass,
            cargo_conservation: pass,
            statistical_guarantee: pass,
            observation_freshness: pass,
            observation_accounting: pass,
            historical_as_of_integrity: pass,
            feasible_authorized_action: pass,
        }
    }

    fn checks(&self) -> [(&'static str, GateCheck); 25] {
        [
            ("source_rights", self.source_rights),
            ("as_of_visibility", self.as_of_visibility),
            ("capture_completeness", self.capture_completeness),
            ("source_claim_admission", self.source_claim_admission),
            ("subject_binding", self.subject_binding),
            ("time_binding", self.time_binding),
            ("source_integrity", self.source_integrity),
            ("oracle_independence", self.oracle_independence),
            ("oracle_implementation", self.oracle_implementation),
            ("episode_binding", self.episode_binding),
            ("physical_feasibility", self.physical_feasibility),
            ("authority", self.authority),
            ("participant_service_floor", self.participant_service_floor),
            ("custody_closure", self.custody_closure),
            ("receiver_acceptance", self.receiver_acceptance),
            ("dependence_unit", self.dependence_unit),
            ("calibration_evaluation_split", self.calibration_evaluation_split),
            ("frozen_candidate_grid", self.frozen_candidate_grid),
            ("outcome_completeness", self.outcome_completeness),
            ("cargo_conservation", self.cargo_conservation),
            ("statistical_guarantee", self.statistical_guarantee),
            ("observation_freshness", self.observation_freshness),
            ("observation_accounting", self.observation_accounting),
            ("historical_as_of_integrity", self.historical_as_of_integrity),
            ("feasible_authorized_action", self.feasible_authorized_action),
        ]
    }

    /// Evaluates all composition gates. Any contradiction/rejection dominates
    /// abstention, so an unknown field cannot hide a separately known violation.
    pub fn evaluate(&self) -> Evaluation {
        let blockers: Vec<Blocker> = self
            .checks()
            .into_iter()
            .filter(|(_, check)| check.state != GateState::Pass)
            .map(|(field, check)| Blocker {
                field,
                state: check.state,
                reason: check.reason,
            })
            .collect();

        let disposition = if blockers.iter().any(|b| b.state.rejects()) {
            DecisionDisposition::Reject
        } else if !blockers.is_empty() {
            DecisionDisposition::Abstain
        } else {
            DecisionDisposition::EligibleForDecisionReview
        };

        Evaluation {
            disposition,
            blockers,
        }
    }
}

/// Applies one named synthetic mutation from the frozen corpus.
///
/// This is a fixture injector, not a source adapter. It exists so the exact
/// negative controls exercise the same public reducer used by downstream
/// consumers.
pub fn apply_named_fault(bundle: &mut EvidenceBundle, fault: &str) -> Result<(), String> {
    let (field, state, reason) = match fault {
        "NONE" => return Ok(()),
        "CAPTURE_NOT_DURABLE" => (
            "capture",
            GateState::Incomplete,
            "CAPTURE_NOT_DURABLE",
        ),
        "EVENT_ID_PAYLOAD_CONFLICT" => (
            "integrity",
            GateState::Fail,
            "EVENT_ID_PAYLOAD_CONFLICT",
        ),
        "SUBJECT_BINDING_MISMATCH" => (
            "subject",
            GateState::Fail,
            "SUBJECT_BINDING_MISMATCH",
        ),
        "ARTIFACT_NOT_VISIBLE_AS_OF" => (
            "as_of",
            GateState::Incomplete,
            "ARTIFACT_NOT_VISIBLE_AS_OF",
        ),
        "EVENT_NOT_KNOWN_AT_DECISION_TIME" => (
            "as_of",
            GateState::Incomplete,
            "EVENT_NOT_KNOWN_AT_DECISION_TIME",
        ),
        "TIME_BINDING_NOT_EVALUABLE" => (
            "time",
            GateState::NotEvaluable,
            "TIME_BINDING_NOT_EVALUABLE",
        ),
        "UNSUPPORTED_CLAIM_PROMOTION" => (
            "claim",
            GateState::Fail,
            "UNSUPPORTED_CLAIM_PROMOTION",
        ),
        "UNSUPPORTED_OUTCOME_PROMOTION" => (
            "claim",
            GateState::Fail,
            "UNSUPPORTED_OUTCOME_PROMOTION",
        ),
        "RECEIVER_PROOF_REQUIRED" => (
            "receiver",
            GateState::Fail,
            "RECEIVER_PROOF_REQUIRED",
        ),
        "CUSTODY_CLOSURE_INCOMPLETE" => (
            "custody",
            GateState::Incomplete,
            "CUSTODY_CLOSURE_INCOMPLETE",
        ),
        "ORACLE_INDEPENDENCE_NOT_ESTABLISHED" => (
            "independence",
            GateState::NotIdentifiable,
            "ORACLE_INDEPENDENCE_NOT_ESTABLISHED",
        ),
        "ORACLE_IMPLEMENTATION_DRIFT" => (
            "implementation",
            GateState::Fail,
            "ORACLE_IMPLEMENTATION_DRIFT",
        ),
        "RECEIPT_EPISODE_BINDING_MISMATCH" => (
            "episode",
            GateState::Fail,
            "RECEIPT_EPISODE_BINDING_MISMATCH",
        ),
        "AUTHORIZATION_REVOKED" => (
            "authority",
            GateState::NotAuthorized,
            "AUTHORIZATION_REVOKED",
        ),
        "AUTHORIZATION_SCOPE_INVALID" => (
            "authority",
            GateState::Fail,
            "AUTHORIZATION_SCOPE_INVALID",
        ),
        "UNKNOWN_PHYSICAL_STATE_BLOCKS" => (
            "physical",
            GateState::Unknown,
            "UNKNOWN_PHYSICAL_STATE_BLOCKS",
        ),
        "NON_COMPENSABLE_PARTICIPANT_FLOOR" => (
            "floor",
            GateState::Fail,
            "NON_COMPENSABLE_PARTICIPANT_FLOOR",
        ),
        "DEPENDENCE_UNIT_MISDECLARED" => (
            "dependence",
            GateState::Fail,
            "DEPENDENCE_UNIT_MISDECLARED",
        ),
        "CALIBRATION_EVALUATION_LEAKAGE" => (
            "split",
            GateState::Fail,
            "CALIBRATION_EVALUATION_LEAKAGE",
        ),
        "FROZEN_POLICY_GRID_MISMATCH" => (
            "grid",
            GateState::Fail,
            "FROZEN_POLICY_GRID_MISMATCH",
        ),
        "MISSING_OUTCOME_PROMOTED" => (
            "outcome",
            GateState::Fail,
            "MISSING_OUTCOME_PROMOTED",
        ),
        "CARGO_CONSERVATION_VIOLATION" => (
            "cargo",
            GateState::Fail,
            "CARGO_CONSERVATION_VIOLATION",
        ),
        "RECEIVER_HANDOFF_BINDING_MISMATCH" => (
            "receiver",
            GateState::Fail,
            "RECEIVER_HANDOFF_BINDING_MISMATCH",
        ),
        "STATISTICAL_GUARANTEE_TYPE_MISMATCH" => (
            "guarantee",
            GateState::Fail,
            "STATISTICAL_GUARANTEE_TYPE_MISMATCH",
        ),
        "OBSERVATION_STALE_FOR_DECISION" => (
            "freshness",
            GateState::Incomplete,
            "OBSERVATION_STALE_FOR_DECISION",
        ),
        "OBSERVATION_VALUE_NOT_EVALUABLE" => (
            "observation",
            GateState::NotEvaluable,
            "OBSERVATION_VALUE_NOT_EVALUABLE",
        ),
        "AS_OF_HISTORY_MUTATED" => (
            "history",
            GateState::Fail,
            "AS_OF_HISTORY_MUTATED",
        ),
        "SOURCE_RIGHTS_VIOLATION" => (
            "rights",
            GateState::Fail,
            "SOURCE_RIGHTS_VIOLATION",
        ),
        "NO_FEASIBLE_AUTHORIZED_ACTION" => (
            "action",
            GateState::NotEvaluable,
            "NO_FEASIBLE_AUTHORIZED_ACTION",
        ),
        other => return Err(format!("unknown fault code: {other}")),
    };

    let check = GateCheck::blocked(state, reason);
    match field {
        "capture" => bundle.capture_completeness = check,
        "integrity" => bundle.source_integrity = check,
        "subject" => bundle.subject_binding = check,
        "as_of" => bundle.as_of_visibility = check,
        "time" => bundle.time_binding = check,
        "claim" => bundle.source_claim_admission = check,
        "receiver" => bundle.receiver_acceptance = check,
        "custody" => bundle.custody_closure = check,
        "independence" => bundle.oracle_independence = check,
        "implementation" => bundle.oracle_implementation = check,
        "episode" => bundle.episode_binding = check,
        "authority" => bundle.authority = check,
        "physical" => bundle.physical_feasibility = check,
        "floor" => bundle.participant_service_floor = check,
        "dependence" => bundle.dependence_unit = check,
        "split" => bundle.calibration_evaluation_split = check,
        "grid" => bundle.frozen_candidate_grid = check,
        "outcome" => bundle.outcome_completeness = check,
        "cargo" => bundle.cargo_conservation = check,
        "guarantee" => bundle.statistical_guarantee = check,
        "freshness" => bundle.observation_freshness = check,
        "observation" => bundle.observation_accounting = check,
        "history" => bundle.historical_as_of_integrity = check,
        "rights" => bundle.source_rights = check,
        "action" => bundle.feasible_authorized_action = check,
        _ => return Err(format!("internal fault mapping error: {field}")),
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorpusSummary {
    pub total: usize,
    pub passed: usize,
    pub failures: Vec<String>,
}

/// Audits the frozen pipe-delimited manifest without external dependencies.
pub fn audit_manifest(manifest: &str) -> Result<CorpusSummary, String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0;
    let mut passed = 0;
    let mut failures = Vec::new();

    for (line_number, line) in manifest.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("case_id|") {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 4 {
            return Err(format!("line {}: expected 4 fields", line_number + 1));
        }
        let [case_id, fault, expected_disposition, expected_reason] =
            [fields[0], fields[1], fields[2], fields[3]];
        if case_id.is_empty() || !seen.insert(case_id.to_owned()) {
            return Err(format!("line {}: empty or duplicate case id", line_number + 1));
        }

        let mut bundle = EvidenceBundle::synthetic_valid_fixture();
        apply_named_fault(&mut bundle, fault)?;
        let actual = bundle.evaluate();
        total += 1;

        let matches = actual.disposition.as_str() == expected_disposition
            && actual.primary_reason() == expected_reason;
        if matches {
            passed += 1;
        } else {
            failures.push(format!(
                "{case_id}: expected {expected_disposition}/{expected_reason}, got {}/{}",
                actual.disposition.as_str(),
                actual.primary_reason()
            ));
        }
    }

    if total == 0 {
        return Err("manifest contains no cases".to_owned());
    }

    Ok(CorpusSummary {
        total,
        passed,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_pass_fixture_is_review_eligible_not_operationally_authorized() {
        let result = EvidenceBundle::synthetic_valid_fixture().evaluate();
        assert_eq!(
            result.disposition,
            DecisionDisposition::EligibleForDecisionReview
        );
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn unknown_physical_state_forces_abstention() {
        let mut bundle = EvidenceBundle::synthetic_valid_fixture();
        apply_named_fault(&mut bundle, "UNKNOWN_PHYSICAL_STATE_BLOCKS").unwrap();
        let result = bundle.evaluate();
        assert_eq!(result.disposition, DecisionDisposition::Abstain);
        assert_eq!(result.primary_reason(), "UNKNOWN_PHYSICAL_STATE_BLOCKS");
    }

    #[test]
    fn known_authority_violation_dominates_unknown_physics() {
        let mut bundle = EvidenceBundle::synthetic_valid_fixture();
        apply_named_fault(&mut bundle, "UNKNOWN_PHYSICAL_STATE_BLOCKS").unwrap();
        apply_named_fault(&mut bundle, "AUTHORIZATION_REVOKED").unwrap();
        let result = bundle.evaluate();
        assert_eq!(result.disposition, DecisionDisposition::Reject);
        assert_eq!(result.primary_reason(), "UNKNOWN_PHYSICAL_STATE_BLOCKS");
        assert_eq!(result.blockers.len(), 2);
    }

    #[test]
    fn unknown_fault_is_rejected() {
        let mut bundle = EvidenceBundle::synthetic_valid_fixture();
        assert!(apply_named_fault(&mut bundle, "NOT_A_REAL_FAULT").is_err());
    }

    #[test]
    fn frozen_tsv_manifest_has_thirty_matching_cases() {
        let manifest = include_str!("../fixtures/composition-cases.tsv");
        let summary = audit_manifest(manifest).unwrap();
        assert_eq!(summary.total, 30);
        assert_eq!(summary.passed, 30);
        assert!(summary.failures.is_empty());
    }
}
