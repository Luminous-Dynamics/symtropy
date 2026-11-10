#![forbid(unsafe_code)]

//! Offline, deterministic evaluator for the synthetic maritime human-re-entry corpus.
//!
//! This module checks packet admissibility, actor/event sequencing, scoped authority,
//! deadline handling, idempotent duplicate events, local-contingency continuity and
//! re-entry prerequisites. It does not model human cognition, vessel physics, or
//! command/control a real ship.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

const EXPECTED_CORPUS_ID: &str = "luminous.symtropy.maritime-human-reentry";
const EXPECTED_CORPUS_VERSION: &str = "0.1.0";
const MIN_CASES: usize = 30;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub corpus_id: String,
    pub corpus_version: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub case_class: CaseClass,
    pub description: String,
    pub setup: Setup,
    pub events: Vec<Event>,
    pub expected: Expected,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseClass {
    Positive,
    Negative,
    Ambiguity,
    Recovery,
    Metamorphic,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub vessel_config: String,
    pub active_mode: ActiveMode,
    pub decision_clock: DecisionClock,
    pub deadline_ms: u64,
    pub packet_state: PacketState,
    pub operator_state: OperatorState,
    pub authority_state: AuthorityState,
    pub link_state: LinkState,
    pub local_contingency: LocalContingency,
    pub target_source_class: TargetSourceClass,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveMode {
    BoundedAutonomy,
    SupervisedAutonomy,
    RemoteControl,
    DegradedLocal,
    ManualOnboard,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionClock {
    MonotonicAndUtcBound,
    UtcOnly,
    ClockConflict,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PacketState {
    CurrentComplete,
    Stale,
    Missing,
    Conflicted,
    WrongConfiguration,
    ContainsSimulatorTruth,
    MissingLineage,
    LateDelivery,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorState {
    Ready,
    Unavailable,
    Overloaded,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityState {
    ValidScoped,
    Expired,
    WrongScope,
    Revoked,
    Ambiguous,
    Absent,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Available,
    Delayed,
    Partitioned,
    Restored,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalContingency {
    Ready,
    Active,
    Unknown,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetSourceClass {
    LocalSensor,
    FusedWithLineage,
    AisOnly,
    V2xOnly,
    SimulatorTruth,
    MixedOrUnknown,
    NotApplicable,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub after_ms: u64,
    pub kind: EventKind,
    pub actor: Actor,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    TakeoverRequested,
    PacketPublished,
    PacketDelivered,
    PacketOpened,
    ClarificationRequested,
    ClarificationDelivered,
    TransferAccepted,
    TransferDeclined,
    TransferAcknowledged,
    LeaseGranted,
    LeaseRevoked,
    LinkLost,
    LinkRestored,
    FallbackStarted,
    ReentryRequested,
    ReentryAccepted,
    ConfigChanged,
    PacketSuperseded,
    OperatorUnavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Vessel,
    LocalController,
    Autonomy,
    Operator,
    Roc,
    Authority,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub final_authority_owner: AuthorityOwner,
    pub must_never_overlap: bool,
    pub fallback_preserved: bool,
    pub reconciliation_required: bool,
    pub invariants: Vec<Invariant>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    HandoverAccepted,
    FallbackContinues,
    ClarificationRequired,
    TransferRejected,
    ReconciliationRequired,
    OperatorUnavailable,
    ReentryBlocked,
    ReentryAccepted,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityOwner {
    LocalController,
    RemoteOperator,
    OnboardMaster,
    Pending,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Invariant {
    NoUnacknowledgedTransfer,
    NoExpiredAuthority,
    NoWrongScopeAuthority,
    NoDualAuthority,
    NoControlGap,
    NoSimulatorTruthLeakage,
    NoSourceRolePromotion,
    NoStalePacketAcceptance,
    NoFutureTimeAcceptance,
    NoReentryWithoutFreshApproval,
    NoDuplicateEffect,
    PreserveLocalContingency,
    RequireReconciliation,
    PositiveControl,
    NoHiddenStateLeakage,
    NoInferredOperatorConsent,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaseOutcome {
    pub disposition: Disposition,
    pub final_authority_owner: AuthorityOwner,
    pub fallback_preserved: bool,
    pub reconciliation_required: bool,
    pub reason_code: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaseReport {
    pub case_id: String,
    pub case_class: CaseClass,
    pub expected_disposition: Disposition,
    pub expected_authority_owner: AuthorityOwner,
    pub actual: CaseOutcome,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvaluationReport {
    pub report_version: String,
    pub corpus_id: String,
    pub corpus_version: String,
    pub corpus_sha256: String,
    pub case_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub all_pass: bool,
    pub cases: Vec<CaseReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationError(pub String);

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EvaluationError {}

impl Corpus {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EvaluationError> {
        let corpus: Self = serde_json::from_slice(bytes)
            .map_err(|error| EvaluationError(format!("invalid corpus JSON: {error}")))?;
        corpus.validate()?;
        Ok(corpus)
    }

    pub fn validate(&self) -> Result<(), EvaluationError> {
        if self.corpus_id != EXPECTED_CORPUS_ID {
            return Err(EvaluationError(format!(
                "unexpected corpus_id: {}",
                self.corpus_id
            )));
        }
        if self.corpus_version != EXPECTED_CORPUS_VERSION {
            return Err(EvaluationError(format!(
                "unsupported corpus_version: {}",
                self.corpus_version
            )));
        }
        if self.claim_ceiling.trim().len() < 30 {
            return Err(EvaluationError("claim_ceiling is absent or too short".into()));
        }
        if self.cases.len() < MIN_CASES {
            return Err(EvaluationError(format!(
                "corpus has {} cases; at least {MIN_CASES} are required",
                self.cases.len()
            )));
        }

        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.as_str()) {
                return Err(EvaluationError(format!(
                    "empty or duplicate case_id: {}",
                    case.case_id
                )));
            }
            if case.description.trim().is_empty() {
                return Err(EvaluationError(format!(
                    "{}: description is empty",
                    case.case_id
                )));
            }
            if case.setup.vessel_config.trim().is_empty() || case.setup.deadline_ms == 0 {
                return Err(EvaluationError(format!(
                    "{}: vessel configuration or deadline is invalid",
                    case.case_id
                )));
            }
            if case.events.is_empty() {
                return Err(EvaluationError(format!(
                    "{}: event tape is empty",
                    case.case_id
                )));
            }
            if !case.expected.must_never_overlap {
                return Err(EvaluationError(format!(
                    "{}: corpus relaxes the non-overlapping-authority invariant",
                    case.case_id
                )));
            }
            if case.expected.invariants.is_empty() {
                return Err(EvaluationError(format!(
                    "{}: no invariants are declared",
                    case.case_id
                )));
            }

            let mut previous = 0;
            for (index, event) in case.events.iter().enumerate() {
                if index > 0 && event.after_ms < previous {
                    return Err(EvaluationError(format!(
                        "{}: event time decreases at index {index}",
                        case.case_id
                    )));
                }
                if event.actor != expected_actor(event.kind) {
                    return Err(EvaluationError(format!(
                        "{}: {:?} has actor {:?}; expected {:?}",
                        case.case_id,
                        event.kind,
                        event.actor,
                        expected_actor(event.kind)
                    )));
                }
                previous = event.after_ms;
            }
        }
        Ok(())
    }
}

fn expected_actor(kind: EventKind) -> Actor {
    match kind {
        EventKind::TakeoverRequested | EventKind::PacketPublished | EventKind::PacketSuperseded => {
            Actor::Autonomy
        }
        EventKind::PacketDelivered
        | EventKind::ClarificationDelivered
        | EventKind::OperatorUnavailable
        | EventKind::LinkLost
        | EventKind::LinkRestored => Actor::Roc,
        EventKind::PacketOpened
        | EventKind::ClarificationRequested
        | EventKind::TransferAccepted
        | EventKind::TransferDeclined
        | EventKind::ReentryRequested => Actor::Operator,
        EventKind::TransferAcknowledged => Actor::Roc,
        EventKind::LeaseGranted | EventKind::LeaseRevoked | EventKind::ReentryAccepted => {
            Actor::Authority
        }
        EventKind::FallbackStarted => Actor::LocalController,
        EventKind::ConfigChanged => Actor::Vessel,
    }
}

fn has_event(case: &Case, kind: EventKind) -> bool {
    case.events.iter().any(|event| event.kind == kind)
}

fn first_event_time(case: &Case, kind: EventKind) -> Option<u64> {
    case.events
        .iter()
        .find(|event| event.kind == kind)
        .map(|event| event.after_ms)
}

fn event_times(case: &Case, kind: EventKind) -> Vec<u64> {
    case.events
        .iter()
        .filter(|event| event.kind == kind)
        .map(|event| event.after_ms)
        .collect()
}

fn local_owner(disposition: Disposition, reason_code: &str) -> CaseOutcome {
    CaseOutcome {
        disposition,
        final_authority_owner: AuthorityOwner::LocalController,
        fallback_preserved: true,
        reconciliation_required: false,
        reason_code: reason_code.to_owned(),
    }
}

fn evaluate_reentry(case: &Case) -> CaseOutcome {
    let setup = &case.setup;
    let accepted_at = first_event_time(case, EventKind::ReentryAccepted);
    let delivered_at = first_event_time(case, EventKind::PacketDelivered);
    let requested_at = first_event_time(case, EventKind::ReentryRequested);

    let has_simulator_truth = setup.packet_state == PacketState::ContainsSimulatorTruth
        || setup.target_source_class == TargetSourceClass::SimulatorTruth;
    if has_simulator_truth || setup.local_contingency == LocalContingency::Unavailable {
        return local_owner(Disposition::Unsupported, "reentry_prerequisite_unsupported");
    }

    let prerequisites_valid = setup.packet_state == PacketState::CurrentComplete
        && setup.authority_state == AuthorityState::ValidScoped
        && matches!(
            setup.local_contingency,
            LocalContingency::Ready | LocalContingency::Active
        )
        && matches!(
            setup.decision_clock,
            DecisionClock::MonotonicAndUtcBound | DecisionClock::UtcOnly
        )
        && !has_event(case, EventKind::ConfigChanged)
        && !has_event(case, EventKind::PacketSuperseded);

    let sequence_valid = matches!((requested_at, delivered_at, accepted_at),
        (Some(requested), Some(delivered), Some(accepted))
            if requested <= delivered
                && delivered <= accepted
                && accepted <= setup.deadline_ms
    );

    if prerequisites_valid && sequence_valid {
        CaseOutcome {
            disposition: Disposition::ReentryAccepted,
            final_authority_owner: AuthorityOwner::LocalController,
            fallback_preserved: true,
            reconciliation_required: false,
            reason_code: "fresh_packet_and_scoped_reentry_approval".into(),
        }
    } else {
        local_owner(Disposition::ReentryBlocked, "reentry_prerequisite_or_sequence_failed")
    }
}

/// Evaluate one case without importing production navigation, ODD or controller logic.
///
/// Duplicate request/delivery/acknowledgement events are treated as idempotent observations.
/// More than one authority lease grant is not idempotent and is rejected.
pub fn evaluate_case(case: &Case) -> CaseOutcome {
    let setup = &case.setup;
    if has_event(case, EventKind::ReentryRequested) {
        return evaluate_reentry(case);
    }

    if !has_event(case, EventKind::TakeoverRequested) {
        return local_owner(Disposition::Unsupported, "no_takeover_or_reentry_request");
    }

    if setup.packet_state == PacketState::ContainsSimulatorTruth
        || setup.target_source_class == TargetSourceClass::SimulatorTruth
    {
        return local_owner(Disposition::Unsupported, "simulator_truth_not_deployment_evidence");
    }

    if !matches!(
        setup.local_contingency,
        LocalContingency::Ready | LocalContingency::Active
    ) {
        return local_owner(Disposition::Unsupported, "local_contingency_not_established");
    }

    if matches!(
        setup.operator_state,
        OperatorState::Unavailable | OperatorState::Overloaded
    ) || has_event(case, EventKind::OperatorUnavailable)
    {
        return local_owner(Disposition::OperatorUnavailable, "operator_not_available");
    }
    if setup.operator_state == OperatorState::Unknown {
        return local_owner(Disposition::Unsupported, "operator_readiness_unknown");
    }

    match setup.packet_state {
        PacketState::Missing | PacketState::LateDelivery => {
            return local_owner(Disposition::FallbackContinues, "no_current_packet_before_deadline");
        }
        PacketState::Stale | PacketState::WrongConfiguration => {
            return local_owner(Disposition::TransferRejected, "packet_not_current_for_configuration");
        }
        PacketState::Conflicted | PacketState::MissingLineage => {
            let clarification = first_event_time(case, EventKind::ClarificationRequested);
            let fallback = first_event_time(case, EventKind::FallbackStarted);
            if matches!((clarification, fallback), (Some(c), Some(f)) if c < f)
                || clarification.is_some() && fallback.is_none()
            {
                return local_owner(Disposition::ClarificationRequired, "material_evidence_conflict");
            }
            if fallback.is_some() {
                return local_owner(Disposition::TransferRejected, "fallback_preceded_resolution");
            }
            return local_owner(Disposition::ClarificationRequired, "material_evidence_conflict");
        }
        PacketState::CurrentComplete | PacketState::ContainsSimulatorTruth => {}
    }

    if setup.decision_clock == DecisionClock::Unknown {
        return local_owner(Disposition::TransferRejected, "decision_clock_unknown");
    }
    if setup.decision_clock == DecisionClock::ClockConflict {
        return local_owner(Disposition::ClarificationRequired, "decision_clock_conflict");
    }

    if matches!(
        setup.target_source_class,
        TargetSourceClass::AisOnly
            | TargetSourceClass::V2xOnly
            | TargetSourceClass::MixedOrUnknown
    ) {
        return local_owner(Disposition::ClarificationRequired, "target_source_not_admissible_as_local_truth");
    }

    if setup.authority_state != AuthorityState::ValidScoped {
        return local_owner(Disposition::TransferRejected, "authority_invalid_or_out_of_scope");
    }

    if has_event(case, EventKind::TransferDeclined) {
        return local_owner(Disposition::FallbackContinues, "operator_declined_transfer");
    }

    let accepted_at = first_event_time(case, EventKind::TransferAccepted);
    let delivered_at = first_event_time(case, EventKind::PacketDelivered);
    let acknowledged_at = first_event_time(case, EventKind::TransferAcknowledged);
    let lease_times = event_times(case, EventKind::LeaseGranted);
    let revoked_at = first_event_time(case, EventKind::LeaseRevoked);
    let fallback_at = first_event_time(case, EventKind::FallbackStarted);
    let link_lost_at = first_event_time(case, EventKind::LinkLost);

    if let Some(ack_at) = acknowledged_at {
        if ack_at > setup.deadline_ms || fallback_at.is_some_and(|fallback| fallback < ack_at) {
            return local_owner(Disposition::TransferRejected, "acknowledgement_late_or_after_fallback");
        }
    }

    let Some(accepted) = accepted_at else {
        if first_event_time(case, EventKind::ClarificationRequested).is_some() {
            return local_owner(Disposition::ClarificationRequired, "operator_requested_clarification");
        }
        return local_owner(Disposition::FallbackContinues, "no_operator_acceptance");
    };

    if accepted > setup.deadline_ms
        || delivered_at.is_none_or(|delivered| delivered > accepted || delivered > setup.deadline_ms)
    {
        return local_owner(Disposition::TransferRejected, "acceptance_without_timely_packet");
    }

    if revoked_at.is_some_and(|revoked| revoked <= setup.deadline_ms) {
        return local_owner(Disposition::TransferRejected, "authority_revoked_during_transfer");
    }

    if acknowledged_at.is_none() {
        if setup.link_state == LinkState::Partitioned
            && link_lost_at.is_some_and(|lost| accepted <= lost && lost <= setup.deadline_ms)
        {
            return CaseOutcome {
                disposition: Disposition::ReconciliationRequired,
                final_authority_owner: AuthorityOwner::Pending,
                fallback_preserved: true,
                reconciliation_required: true,
                reason_code: "accepted_request_response_lost_under_partition".into(),
            };
        }
        return local_owner(Disposition::FallbackContinues, "acceptance_not_acknowledged");
    }

    if lease_times.len() > 1 {
        return local_owner(Disposition::TransferRejected, "multiple_authority_lease_grants");
    }

    let Some(lease_granted_at) = lease_times.first().copied() else {
        return local_owner(Disposition::FallbackContinues, "acknowledged_transfer_without_lease");
    };

    let ack_at = acknowledged_at.unwrap_or_default();
    if ack_at < accepted
        || lease_granted_at < ack_at
        || lease_granted_at > setup.deadline_ms
        || fallback_at.is_some_and(|fallback| fallback < lease_granted_at)
        || revoked_at.is_some_and(|revoked| revoked <= lease_granted_at)
    {
        return local_owner(Disposition::TransferRejected, "invalid_lease_sequence_or_revocation");
    }

    CaseOutcome {
        disposition: Disposition::HandoverAccepted,
        final_authority_owner: AuthorityOwner::RemoteOperator,
        fallback_preserved: true,
        reconciliation_required: false,
        reason_code: "scoped_acknowledged_single_lease".into(),
    }
}

pub fn evaluate_corpus(corpus: &Corpus, corpus_bytes: &[u8]) -> EvaluationReport {
    let cases: Vec<CaseReport> = corpus
        .cases
        .iter()
        .map(|case| {
            let actual = evaluate_case(case);
            let passed = actual.disposition == case.expected.disposition
                && actual.final_authority_owner == case.expected.final_authority_owner
                && actual.fallback_preserved == case.expected.fallback_preserved
                && actual.reconciliation_required == case.expected.reconciliation_required;
            CaseReport {
                case_id: case.case_id.clone(),
                case_class: case.case_class,
                expected_disposition: case.expected.disposition,
                expected_authority_owner: case.expected.final_authority_owner,
                actual,
                passed,
            }
        })
        .collect();

    let passed_count = cases.iter().filter(|case| case.passed).count();
    let failed_count = cases.len() - passed_count;
    let digest = Sha256::digest(corpus_bytes);
    EvaluationReport {
        report_version: "0.1.0".into(),
        corpus_id: corpus.corpus_id.clone(),
        corpus_version: corpus.corpus_version.clone(),
        corpus_sha256: format!("{digest:x}"),
        case_count: cases.len(),
        passed_count,
        failed_count,
        all_pass: failed_count == 0,
        cases,
    }
}

pub fn evaluate_bytes(bytes: &[u8]) -> Result<EvaluationReport, EvaluationError> {
    let corpus = Corpus::from_bytes(bytes)?;
    Ok(evaluate_corpus(&corpus, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../maritime/human-reentry-corpus-v0.1.json");

    fn parsed_corpus() -> Corpus {
        Corpus::from_bytes(CORPUS.as_bytes()).expect("frozen corpus must parse")
    }

    fn case_named<'a>(corpus: &'a Corpus, case_id: &str) -> &'a Case {
        corpus.cases.iter().find(|case| case.case_id == case_id).expect("case exists")
    }

    #[test]
    fn all_frozen_cases_match_independent_expected_dispositions() {
        let report = evaluate_bytes(CORPUS.as_bytes()).expect("evaluate frozen corpus");
        assert_eq!(report.case_count, 33);
        assert_eq!(report.passed_count, 33, "{:#?}", report.cases.iter().filter(|c| !c.passed).collect::<Vec<_>>());
        assert_eq!(report.failed_count, 0);
        assert!(report.all_pass);
        assert_eq!(report.corpus_sha256.len(), 64);
    }

    #[test]
    fn duplicate_acknowledgement_is_idempotent_but_duplicate_lease_is_rejected() {
        let mut corpus = parsed_corpus();
        let mut case = case_named(&corpus, "positive-current-scoped-transfer").clone();
        case.events.push(Event {
            after_ms: 2_300,
            kind: EventKind::TransferAcknowledged,
            actor: Actor::Roc,
        });
        assert_eq!(evaluate_case(&case).disposition, Disposition::HandoverAccepted);

        case.events.push(Event {
            after_ms: 2_400,
            kind: EventKind::LeaseGranted,
            actor: Actor::Authority,
        });
        assert_eq!(evaluate_case(&case).disposition, Disposition::TransferRejected);
        // Keep the parsed corpus alive to make the fixture's provenance explicit.
        corpus.validate().expect("corpus remains structurally valid");
    }

    #[test]
    fn an_accepted_request_lost_in_partition_requires_reconciliation() {
        let corpus = parsed_corpus();
        let case = case_named(&corpus, "ambiguity-ack-response-lost");
        let actual = evaluate_case(case);
        assert_eq!(actual.disposition, Disposition::ReconciliationRequired);
        assert_eq!(actual.final_authority_owner, AuthorityOwner::Pending);
        assert!(actual.reconciliation_required);
        assert!(actual.fallback_preserved);
    }

    #[test]
    fn ais_only_target_is_not_promoted_to_local_sensor_truth() {
        let corpus = parsed_corpus();
        let case = case_named(&corpus, "negative-ais-only-target");
        assert_eq!(evaluate_case(case).disposition, Disposition::ClarificationRequired);
    }

    #[test]
    fn simulator_truth_is_not_admissible_for_deployment_handover() {
        let corpus = parsed_corpus();
        let case = case_named(&corpus, "negative-simulator-truth-leak");
        assert_eq!(evaluate_case(case).disposition, Disposition::Unsupported);
    }

    #[test]
    fn stale_authority_blocks_reentry() {
        let corpus = parsed_corpus();
        let case = case_named(&corpus, "negative-reentry-stale-approval");
        assert_eq!(evaluate_case(case).disposition, Disposition::ReentryBlocked);
    }

    #[test]
    fn malformed_monotonic_tape_is_rejected_before_evaluation() {
        let mut corpus = parsed_corpus();
        let case = corpus.cases.iter_mut().find(|c| c.case_id == "positive-current-scoped-transfer").unwrap();
        case.events[1].after_ms = 0;
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutated corpus");
        let error = Corpus::from_bytes(&bytes).expect_err("non-monotonic tape must fail closed");
        assert!(error.to_string().contains("event time decreases"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let invalid = CORPUS.replace(
            "\"corpus_version\": \"0.1.0\"",
            "\"corpus_version\": \"0.1.0\", \"unreviewed_override\": true",
        );
        let error = Corpus::from_bytes(invalid.as_bytes()).expect_err("unknown fields must fail closed");
        assert!(error.to_string().contains("invalid corpus JSON"));
    }
}
