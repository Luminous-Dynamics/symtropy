//! Dependency-free decision-time replay and evaluator boundary.
//!
//! Policy code receives only PolicyInput. EvaluatorTruth is passed to the
//! evaluator after the policy has produced an action; it is never embedded
//! in PolicyInput.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
}

impl GateState {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "PASS" => Ok(Self::Pass),
            "FAIL" => Ok(Self::Fail),
            "UNKNOWN" => Ok(Self::Unknown),
            other => Err(format!("invalid gate state: {other}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEvent {
    pub source_id: String,
    pub event_id: String,
    pub event_subject_id: String,
    pub event_time: i64,
    pub published_at: i64,
    pub retrieved_at: i64,
    pub decision_use_from: i64,
    pub decision_use_until: i64,
    pub rights_clear: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoLedger {
    pub expected_units: u64,
    pub receiver_accepted_units: u64,
    pub in_transit_units: u64,
    pub outstanding_units: u64,
    pub unresolved_units: u64,
    pub duplicated_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluatorTruth {
    pub physical: GateState,
    pub authority: GateState,
    pub participant_floor: GateState,
    pub cargo: CargoLedger,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayCase {
    pub case_id: String,
    pub decision_cutoff: i64,
    pub subject_id: String,
    pub source_event: SourceEvent,
    pub evaluator_truth: EvaluatorTruth,
    pub expected_disposition: String,
    pub expected_reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisibleEvent {
    pub source_id: String,
    pub event_id: String,
    pub event_time: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyInput {
    pub case_id: String,
    pub decision_cutoff: i64,
    pub subject_id: String,
    pub visible_events: Vec<VisibleEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyAction {
    ProposeReview { evidence_event_ids: Vec<String> },
    Abstain,
}

impl PolicyAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ProposeReview { .. } => "PROPOSE_REVIEW",
            Self::Abstain => "ABSTAIN",
        }
    }

    pub fn evidence_ids(&self) -> String {
        match self {
            Self::ProposeReview { evidence_event_ids } if !evidence_event_ids.is_empty() => {
                evidence_event_ids.join(",")
            }
            _ => "-".to_owned(),
        }
    }
}

/// The only input accepted by the example policy. In particular, no evaluator
/// outcome, future truth, physical oracle result, authority result, or cargo
/// realization is present here.
pub trait Policy {
    fn decide(&self, input: &PolicyInput) -> PolicyAction;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FirstVisibleEvidencePolicy;

impl Policy for FirstVisibleEvidencePolicy {
    fn decide(&self, input: &PolicyInput) -> PolicyAction {
        match input.visible_events.first() {
            Some(event) => PolicyAction::ProposeReview {
                evidence_event_ids: vec![event.event_id.clone()],
            },
            None => PolicyAction::Abstain,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    EligibleForDecisionReview,
    Abstain,
    Reject,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EligibleForDecisionReview => "eligible_for_decision_review",
            Self::Abstain => "abstain",
            Self::Reject => "reject",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    pub disposition: Disposition,
    pub reasons: Vec<&'static str>,
}

impl Evaluation {
    pub fn reason_string(&self) -> String {
        if self.reasons.is_empty() {
            "ALL_GATES_PASS".to_owned()
        } else {
            self.reasons.join("|")
        }
    }
}

/// Reconstruct the observation set that was legitimately available to the
/// policy at the decision cutoff. Event time is deliberately not compared to
/// the cutoff: a future scheduled event can be known now. Publication time,
/// retrieval time, rights, subject binding, and decision-use validity govern
/// whether the record is visible.
pub fn build_policy_input(case: &ReplayCase) -> PolicyInput {
    let event = &case.source_event;
    let visible = event.rights_clear
        && event.event_subject_id == case.subject_id
        && event.published_at <= case.decision_cutoff
        && event.retrieved_at <= case.decision_cutoff
        && event.decision_use_from <= case.decision_cutoff
        && case.decision_cutoff <= event.decision_use_until;

    let visible_events = if visible {
        vec![VisibleEvent {
            source_id: event.source_id.clone(),
            event_id: event.event_id.clone(),
            event_time: event.event_time,
        }]
    } else {
        Vec::new()
    };

    PolicyInput {
        case_id: case.case_id.clone(),
        decision_cutoff: case.decision_cutoff,
        subject_id: case.subject_id.clone(),
        visible_events,
    }
}

/// Independent post-decision evaluator. The policy cannot call this with its
/// PolicyInput alone: evaluator truth and source-admission facts are separate
/// arguments, used only after a PolicyAction already exists.
pub fn adjudicate(
    source_event: &SourceEvent,
    expected_subject_id: &str,
    policy_input: &PolicyInput,
    action: &PolicyAction,
    truth: &EvaluatorTruth,
) -> Evaluation {
    if !source_event.rights_clear {
        return Evaluation {
            disposition: Disposition::Reject,
            reasons: vec!["SOURCE_RIGHTS_DENIED"],
        };
    }
    if source_event.event_subject_id != expected_subject_id {
        return Evaluation {
            disposition: Disposition::Reject,
            reasons: vec!["SUBJECT_BINDING_MISMATCH"],
        };
    }

    let evidence_event_ids = match action {
        PolicyAction::Abstain => {
            let reason = if policy_input.visible_events.is_empty() {
                "NO_VISIBLE_EVIDENCE"
            } else {
                "POLICY_ABSTAINED"
            };
            return Evaluation {
                disposition: Disposition::Abstain,
                reasons: vec![reason],
            };
        }
        PolicyAction::ProposeReview { evidence_event_ids } => evidence_event_ids,
    };

    if evidence_event_ids.is_empty() {
        return Evaluation {
            disposition: Disposition::Reject,
            reasons: vec!["ACTION_WITHOUT_EVIDENCE"],
        };
    }

    let all_evidence_visible = evidence_event_ids.iter().all(|event_id| {
        policy_input
            .visible_events
            .iter()
            .any(|visible| visible.event_id == *event_id)
    });
    if !all_evidence_visible {
        return Evaluation {
            disposition: Disposition::Reject,
            reasons: vec!["POLICY_USED_INADMISSIBLE_EVIDENCE"],
        };
    }

    let mut rejections = Vec::new();
    let mut abstentions = Vec::new();

    record_gate(truth.physical, "PHYSICAL_INFEASIBLE", "PHYSICAL_UNKNOWN", &mut rejections, &mut abstentions);
    record_gate(truth.authority, "AUTHORITY_DENIED", "AUTHORITY_UNKNOWN", &mut rejections, &mut abstentions);
    record_gate(truth.participant_floor, "PARTICIPANT_FLOOR_VIOLATION", "PARTICIPANT_FLOOR_UNKNOWN", &mut rejections, &mut abstentions);
    record_cargo(&truth.cargo, &mut rejections, &mut abstentions);

    if !rejections.is_empty() {
        Evaluation {
            disposition: Disposition::Reject,
            reasons: rejections.into_iter().chain(abstentions).collect(),
        }
    } else if !abstentions.is_empty() {
        Evaluation {
            disposition: Disposition::Abstain,
            reasons: abstentions,
        }
    } else {
        Evaluation {
            disposition: Disposition::EligibleForDecisionReview,
            reasons: Vec::new(),
        }
    }
}

fn record_gate(
    state: GateState,
    rejection_reason: &'static str,
    abstention_reason: &'static str,
    rejections: &mut Vec<&'static str>,
    abstentions: &mut Vec<&'static str>,
) {
    match state {
        GateState::Pass => {}
        GateState::Fail => rejections.push(rejection_reason),
        GateState::Unknown => abstentions.push(abstention_reason),
    }
}

fn record_cargo(
    cargo: &CargoLedger,
    rejections: &mut Vec<&'static str>,
    abstentions: &mut Vec<&'static str>,
) {
    if cargo.duplicated_units > 0 {
        rejections.push("CARGO_DUPLICATION");
    }

    let total = cargo
        .receiver_accepted_units
        .checked_add(cargo.in_transit_units)
        .and_then(|value| value.checked_add(cargo.outstanding_units))
        .and_then(|value| value.checked_add(cargo.unresolved_units));

    match total {
        None => rejections.push("CARGO_ARITHMETIC_OVERFLOW"),
        Some(value) if value > cargo.expected_units => rejections.push("CARGO_OVERCOUNT"),
        Some(value) if value < cargo.expected_units => abstentions.push("CARGO_UNRECONCILED"),
        Some(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_case() -> ReplayCase {
        ReplayCase {
            case_id: "C00_VALID".to_owned(),
            decision_cutoff: 100,
            subject_id: "BOX-1".to_owned(),
            source_event: SourceEvent {
                source_id: "source-a".to_owned(),
                event_id: "event-1".to_owned(),
                event_subject_id: "BOX-1".to_owned(),
                event_time: 90,
                published_at: 80,
                retrieved_at: 95,
                decision_use_from: 0,
                decision_use_until: 150,
                rights_clear: true,
            },
            evaluator_truth: EvaluatorTruth {
                physical: GateState::Pass,
                authority: GateState::Pass,
                participant_floor: GateState::Pass,
                cargo: CargoLedger {
                    expected_units: 10,
                    receiver_accepted_units: 2,
                    in_transit_units: 3,
                    outstanding_units: 5,
                    unresolved_units: 0,
                    duplicated_units: 0,
                },
            },
            expected_disposition: "eligible_for_decision_review".to_owned(),
            expected_reason: "ALL_GATES_PASS".to_owned(),
        }
    }

    fn evaluate(case: &ReplayCase) -> (PolicyInput, PolicyAction, Evaluation) {
        let input = build_policy_input(case);
        let action = FirstVisibleEvidencePolicy.decide(&input);
        let result = adjudicate(
            &case.source_event,
            &case.subject_id,
            &input,
            &action,
            &case.evaluator_truth,
        );
        (input, action, result)
    }

    #[test]
    fn valid_chain_is_eligible_only_for_review() {
        let case = base_case();
        let (input, action, result) = evaluate(&case);
        assert_eq!(input.visible_events.len(), 1);
        assert_eq!(action.label(), "PROPOSE_REVIEW");
        assert_eq!(result.disposition, Disposition::EligibleForDecisionReview);
    }

    #[test]
    fn publication_after_cutoff_is_not_visible() {
        let mut case = base_case();
        case.source_event.published_at = 101;
        let (input, action, result) = evaluate(&case);
        assert!(input.visible_events.is_empty());
        assert_eq!(action, PolicyAction::Abstain);
        assert_eq!(result.reason_string(), "NO_VISIBLE_EVIDENCE");
    }

    #[test]
    fn retrieval_after_cutoff_is_not_visible() {
        let mut case = base_case();
        case.source_event.retrieved_at = 101;
        let (input, _, result) = evaluate(&case);
        assert!(input.visible_events.is_empty());
        assert_eq!(result.disposition, Disposition::Abstain);
    }

    #[test]
    fn future_event_time_can_be_known_before_cutoff() {
        let mut case = base_case();
        case.source_event.event_time = 120;
        let (input, _, result) = evaluate(&case);
        assert_eq!(input.visible_events.len(), 1);
        assert_eq!(result.disposition, Disposition::EligibleForDecisionReview);
    }

    #[test]
    fn rights_violation_rejects_even_when_no_policy_observation_is_visible() {
        let mut case = base_case();
        case.source_event.rights_clear = false;
        let (_, action, result) = evaluate(&case);
        assert_eq!(action, PolicyAction::Abstain);
        assert_eq!(result.disposition, Disposition::Reject);
        assert_eq!(result.reason_string(), "SOURCE_RIGHTS_DENIED");
    }

    #[test]
    fn subject_mismatch_is_rejected() {
        let mut case = base_case();
        case.source_event.event_subject_id = "BOX-OTHER".to_owned();
        let (_, _, result) = evaluate(&case);
        assert_eq!(result.reason_string(), "SUBJECT_BINDING_MISMATCH");
    }

    #[test]
    fn known_violation_dominates_unknown_and_preserves_both() {
        let mut case = base_case();
        case.evaluator_truth.physical = GateState::Fail;
        case.evaluator_truth.authority = GateState::Unknown;
        let (_, _, result) = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Reject);
        assert_eq!(
            result.reason_string(),
            "PHYSICAL_INFEASIBLE|AUTHORITY_UNKNOWN"
        );
    }

    #[test]
    fn unknown_physical_state_abstains() {
        let mut case = base_case();
        case.evaluator_truth.physical = GateState::Unknown;
        let (_, _, result) = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Abstain);
        assert_eq!(result.reason_string(), "PHYSICAL_UNKNOWN");
    }

    #[test]
    fn cargo_duplication_rejects() {
        let mut case = base_case();
        case.evaluator_truth.cargo.duplicated_units = 1;
        let (_, _, result) = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Reject);
        assert_eq!(result.reason_string(), "CARGO_DUPLICATION");
    }

    #[test]
    fn incomplete_cargo_ledger_abstains() {
        let mut case = base_case();
        case.evaluator_truth.cargo.outstanding_units = 4;
        let (_, _, result) = evaluate(&case);
        assert_eq!(result.disposition, Disposition::Abstain);
        assert_eq!(result.reason_string(), "CARGO_UNRECONCILED");
    }

    #[test]
    fn policy_cannot_reference_an_event_outside_its_input() {
        let case = base_case();
        let input = build_policy_input(&case);
        let action = PolicyAction::ProposeReview {
            evidence_event_ids: vec!["post-cutoff-correction".to_owned()],
        };
        let result = adjudicate(
            &case.source_event,
            &case.subject_id,
            &input,
            &action,
            &case.evaluator_truth,
        );
        assert_eq!(result.disposition, Disposition::Reject);
        assert_eq!(
            result.reason_string(),
            "POLICY_USED_INADMISSIBLE_EVIDENCE"
        );
    }

    #[test]
    fn policy_output_is_independent_of_evaluator_truth() {
        let mut first = base_case();
        let mut second = base_case();
        second.evaluator_truth.physical = GateState::Fail;
        second.evaluator_truth.cargo.duplicated_units = 9;

        let first_input = build_policy_input(&first);
        let second_input = build_policy_input(&second);
        let first_action = FirstVisibleEvidencePolicy.decide(&first_input);
        let second_action = FirstVisibleEvidencePolicy.decide(&second_input);

        assert_eq!(first_input, second_input);
        assert_eq!(first_action, second_action);

        // Keep both fixtures mutable only to make the hidden-truth distinction
        // explicit to readers and to ensure no accidental truth sharing occurs.
        first.evaluator_truth.authority = GateState::Unknown;
        assert_ne!(first.evaluator_truth, second.evaluator_truth);
    }
}
