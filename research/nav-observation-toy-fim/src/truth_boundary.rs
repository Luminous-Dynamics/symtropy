//! Executable in-process boundary between controller-visible inputs and
//! evaluator-only truth.
//!
//! The controller proposal function has no EvaluatorTruth argument or field.
//! A post-acquisition event must bind byte-for-byte to the exact proposal.
//! Evaluator truth is consumed only by a later scoring function.
//!
//! This is a type/API separation regression harness, not an OS process sandbox
//! or a proof that arbitrary controller software cannot read evaluator data.

use std::collections::BTreeSet;
use std::fmt;

/// Hard resource budget. The three dimensions are non-compensatory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub energy_units: u32,
    pub latency_ticks: u32,
    pub bandwidth_units: u32,
}

/// One action candidate visible before any acquisition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateAction {
    pub id: String,
    pub energy_units: u32,
    pub latency_ticks: u32,
    pub bandwidth_units: u32,
    /// Pre-acquisition model output in integer micro-value units.
    /// This is a controller-visible estimate, not evaluator truth.
    pub expected_decision_value_micros: i64,
}

impl CandidateAction {
    fn fits(&self, budget: ResourceBudget) -> bool {
        self.energy_units <= budget.energy_units
            && self.latency_ticks <= budget.latency_ticks
            && self.bandwidth_units <= budget.bandwidth_units
    }
}

/// Public controller state. There is deliberately no evaluator-truth field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerVisibleInput {
    pub policy_revision: String,
    pub input_commitment: [u8; 32],
    pub deterministic_seed: u64,
    pub belief_state_one_ppm: u32,
    pub false_safe_loss_micros: u64,
    pub false_degraded_loss_micros: u64,
    pub budget: ResourceBudget,
    pub candidates: Vec<CandidateAction>,
}

/// Controller disposition is explicit: select one observation action or abstain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalDisposition {
    Acquire { action_id: String },
    Abstain { reason: AbstainReason },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    NoPositiveValue,
    NoFeasibleAction,
}

/// Immutable pre-acquisition proposal.
///
/// It contains only controller-visible commitments and the controller's
/// proposed action/claim. Evaluator truth is not representable here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionProposal {
    pub policy_revision: String,
    pub input_commitment: [u8; 32],
    pub deterministic_seed: u64,
    pub disposition: ProposalDisposition,
    pub expected_decision_value_micros: i64,
    pub claim_state_one: bool,
}

impl ActionProposal {
    /// Canonical, versioned byte encoding. No serializer/map iteration or
    /// process-local address contributes to this representation.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(b"NAV-ACTION-PROPOSAL-v1\0");
        put_string(&mut bytes, &self.policy_revision);
        bytes.extend_from_slice(&self.input_commitment);
        bytes.extend_from_slice(&self.deterministic_seed.to_le_bytes());
        match &self.disposition {
            ProposalDisposition::Acquire { action_id } => {
                bytes.push(1);
                put_string(&mut bytes, action_id);
            }
            ProposalDisposition::Abstain { reason } => {
                bytes.push(0);
                bytes.push(match reason {
                    AbstainReason::NoPositiveValue => 1,
                    AbstainReason::NoFeasibleAction => 2,
                });
            }
        }
        bytes.extend_from_slice(&self.expected_decision_value_micros.to_le_bytes());
        bytes.push(u8::from(self.claim_state_one));
        bytes
    }
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    let len = u32::try_from(value.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

/// Controller errors are deterministic and fail closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryError {
    EmptyPolicyRevision,
    InvalidBelief,
    InvalidLoss,
    EmptyActionId,
    DuplicateActionId(String),
    ProposalMismatch,
    MissingEvaluatorOutcome,
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPolicyRevision => write!(f, "policy revision must not be empty"),
            Self::InvalidBelief => write!(f, "belief must be within [0, 1_000_000] ppm"),
            Self::InvalidLoss => write!(f, "both decision losses cannot be zero"),
            Self::EmptyActionId => write!(f, "candidate action ID must not be empty"),
            Self::DuplicateActionId(id) => write!(f, "duplicate candidate action ID: {id}"),
            Self::ProposalMismatch => write!(f, "acquisition event is not bound to this exact proposal"),
            Self::MissingEvaluatorOutcome => write!(f, "successful acquisition has no evaluator outcome"),
        }
    }
}
impl std::error::Error for BoundaryError {}

/// Pure controller function. Its signature accepts only controller-visible
/// inputs; there is no route for evaluator-only truth to influence selection.
pub fn controller_propose(
    input: &ControllerVisibleInput,
) -> Result<ActionProposal, BoundaryError> {
    if input.policy_revision.trim().is_empty() {
        return Err(BoundaryError::EmptyPolicyRevision);
    }
    if input.belief_state_one_ppm > 1_000_000 {
        return Err(BoundaryError::InvalidBelief);
    }
    if input.false_safe_loss_micros == 0 && input.false_degraded_loss_micros == 0 {
        return Err(BoundaryError::InvalidLoss);
    }

    let mut ids = BTreeSet::new();
    for candidate in &input.candidates {
        if candidate.id.trim().is_empty() {
            return Err(BoundaryError::EmptyActionId);
        }
        if !ids.insert(candidate.id.clone()) {
            return Err(BoundaryError::DuplicateActionId(candidate.id.clone()));
        }
    }

    // The threshold is derived solely from prior belief and declared decision
    // losses. False-safe is the cost of claiming StateOne when StateZero is
    // true; false-degraded is the reverse.
    let p_one = input.belief_state_one_ppm as u128;
    let p_zero = 1_000_000_u128 - p_one;
    let claim_state_one =
        p_zero * input.false_safe_loss_micros as u128
            <= p_one * input.false_degraded_loss_micros as u128;

    let mut feasible: Vec<&CandidateAction> = input
        .candidates
        .iter()
        .filter(|candidate| candidate.fits(input.budget))
        .collect();
    feasible.sort_by(|a, b| {
        b.expected_decision_value_micros
            .cmp(&a.expected_decision_value_micros)
            .then_with(|| a.energy_units.cmp(&b.energy_units))
            .then_with(|| a.latency_ticks.cmp(&b.latency_ticks))
            .then_with(|| a.bandwidth_units.cmp(&b.bandwidth_units))
            .then_with(|| a.id.cmp(&b.id))
    });

    let selected = feasible
        .iter()
        .copied()
        .find(|candidate| candidate.expected_decision_value_micros > 0);

    let (disposition, expected_decision_value_micros) = match selected {
        Some(candidate) => (
            ProposalDisposition::Acquire {
                action_id: candidate.id.clone(),
            },
            candidate.expected_decision_value_micros,
        ),
        None => {
            let reason = if feasible.is_empty() {
                AbstainReason::NoFeasibleAction
            } else {
                AbstainReason::NoPositiveValue
            };
            (ProposalDisposition::Abstain { reason }, 0)
        }
    };

    Ok(ActionProposal {
        policy_revision: input.policy_revision.clone(),
        input_commitment: input.input_commitment,
        deterministic_seed: input.deterministic_seed,
        disposition,
        expected_decision_value_micros,
        claim_state_one,
    })
}

/// Acquisition result is a post-proposal event, never an input to proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionStatus {
    Succeeded { outcome_state_one_ppm: u32 },
    Failed,
    Unavailable,
}

/// The event carries the full proposal bytes for transparent exact binding.
/// A production receipt can additionally hash these bytes with a qualified
/// cryptographic implementation and bind the digest into its run manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquisitionEvent {
    pub action_id: String,
    pub proposal_bytes: Vec<u8>,
    pub status: AcquisitionStatus,
}

/// Evaluator-only truth. This type is accepted only by score_evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatorTruth {
    pub service_state_one: bool,
    pub truth_manifest_commitment: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationScore {
    pub truth_manifest_commitment: [u8; 32],
    pub correct: bool,
    pub loss_micros: u64,
}

fn validate_event_binding(
    proposal: &ActionProposal,
    event: &AcquisitionEvent,
) -> Result<(), BoundaryError> {
    if proposal.canonical_bytes() != event.proposal_bytes {
        return Err(BoundaryError::ProposalMismatch);
    }
    match &proposal.disposition {
        ProposalDisposition::Acquire { action_id } if action_id == &event.action_id => Ok(()),
        ProposalDisposition::Abstain { .. } => Err(BoundaryError::ProposalMismatch),
        ProposalDisposition::Acquire { .. } => Err(BoundaryError::ProposalMismatch),
    }
}

/// Score only after the controller proposal and acquisition receipt exist.
/// The truth manifest commitment is introduced here and nowhere in the
/// controller proposal path.
pub fn score_evaluation(
    proposal: &ActionProposal,
    event: &AcquisitionEvent,
    truth: &EvaluatorTruth,
    false_safe_loss_micros: u64,
    false_degraded_loss_micros: u64,
) -> Result<EvaluationScore, BoundaryError> {
    validate_event_binding(proposal, event)?;
    if false_safe_loss_micros == 0 && false_degraded_loss_micros == 0 {
        return Err(BoundaryError::InvalidLoss);
    }

    let evaluated_claim = match event.status {
        AcquisitionStatus::Succeeded { outcome_state_one_ppm } => {
            if outcome_state_one_ppm > 1_000_000 {
                return Err(BoundaryError::InvalidBelief);
            }
            outcome_state_one_ppm >= 500_000
        }
        AcquisitionStatus::Failed | AcquisitionStatus::Unavailable => {
            proposal.claim_state_one
        }
    };

    let correct = evaluated_claim == truth.service_state_one;
    let loss_micros = if correct {
        0
    } else if evaluated_claim && !truth.service_state_one {
        false_safe_loss_micros
    } else {
        false_degraded_loss_micros
    };

    Ok(EvaluationScore {
        truth_manifest_commitment: truth.truth_manifest_commitment,
        correct,
        loss_micros,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ControllerVisibleInput {
        ControllerVisibleInput {
            policy_revision: "nav-policy-v1".to_owned(),
            input_commitment: [0x11; 32],
            deterministic_seed: 7,
            belief_state_one_ppm: 800_000,
            false_safe_loss_micros: 10,
            false_degraded_loss_micros: 1,
            budget: ResourceBudget {
                energy_units: 4,
                latency_ticks: 10,
                bandwidth_units: 2,
            },
            candidates: vec![
                CandidateAction {
                    id: "uwb-reobserve".to_owned(),
                    energy_units: 2,
                    latency_ticks: 4,
                    bandwidth_units: 1,
                    expected_decision_value_micros: 120,
                },
                CandidateAction {
                    id: "camera-reobserve".to_owned(),
                    energy_units: 3,
                    latency_ticks: 5,
                    bandwidth_units: 1,
                    expected_decision_value_micros: 120,
                },
                CandidateAction {
                    id: "infeasible-high-score".to_owned(),
                    energy_units: 9,
                    latency_ticks: 1,
                    bandwidth_units: 1,
                    expected_decision_value_micros: 999,
                },
            ],
        }
    }

    fn event_for(proposal: &ActionProposal, status: AcquisitionStatus) -> AcquisitionEvent {
        let action_id = match &proposal.disposition {
            ProposalDisposition::Acquire { action_id } => action_id.clone(),
            ProposalDisposition::Abstain { .. } => String::new(),
        };
        AcquisitionEvent {
            action_id,
            proposal_bytes: proposal.canonical_bytes(),
            status,
        }
    }

    #[test]
    fn truth_swap_changes_only_evaluator_score_not_pre_acquisition_trace() {
        let public = input();
        let proposal_a = controller_propose(&public).unwrap();
        let proposal_b = controller_propose(&public).unwrap();
        let bytes_a = proposal_a.canonical_bytes();
        let bytes_b = proposal_b.canonical_bytes();
        assert_eq!(bytes_a, bytes_b);

        let event_a = event_for(
            &proposal_a,
            AcquisitionStatus::Succeeded {
                outcome_state_one_ppm: 700_000,
            },
        );
        let event_b = event_for(
            &proposal_b,
            AcquisitionStatus::Succeeded {
                outcome_state_one_ppm: 700_000,
            },
        );
        let truth_a = EvaluatorTruth {
            service_state_one: true,
            truth_manifest_commitment: [0xaa; 32],
        };
        let truth_b = EvaluatorTruth {
            service_state_one: false,
            truth_manifest_commitment: [0xbb; 32],
        };
        let score_a = score_evaluation(&proposal_a, &event_a, &truth_a, 10, 1).unwrap();
        let score_b = score_evaluation(&proposal_b, &event_b, &truth_b, 10, 1).unwrap();

        assert_eq!(bytes_a, bytes_b);
        assert_eq!(proposal_a, proposal_b);
        assert!(score_a.correct);
        assert!(!score_b.correct);
        assert_ne!(score_a.loss_micros, score_b.loss_micros);
        assert_eq!(score_a.truth_manifest_commitment, [0xaa; 32]);
        assert_eq!(score_b.truth_manifest_commitment, [0xbb; 32]);
    }

    #[test]
    fn controller_selects_feasible_candidate_with_deterministic_tie_break() {
        let proposal = controller_propose(&input()).unwrap();
        assert_eq!(
            proposal.disposition,
            ProposalDisposition::Acquire {
                action_id: "uwb-reobserve".to_owned()
            }
        );
        assert_eq!(proposal.expected_decision_value_micros, 120);
    }

    #[test]
    fn candidate_permutation_does_not_change_proposal_bytes() {
        let mut forward = input();
        let mut reverse = input();
        reverse.candidates.reverse();
        let a = controller_propose(&forward).unwrap();
        let b = controller_propose(&reverse).unwrap();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        forward.deterministic_seed += 1;
        let c = controller_propose(&forward).unwrap();
        assert_ne!(a.canonical_bytes(), c.canonical_bytes());
    }

    #[test]
    fn no_positive_value_abstains_explicitly() {
        let mut public = input();
        public.candidates.iter_mut().for_each(|item| {
            item.expected_decision_value_micros = 0;
        });
        let proposal = controller_propose(&public).unwrap();
        assert_eq!(
            proposal.disposition,
            ProposalDisposition::Abstain {
                reason: AbstainReason::NoPositiveValue
            }
        );
    }

    #[test]
    fn all_infeasible_candidates_abstain_explicitly() {
        let mut public = input();
        public.budget.energy_units = 0;
        let proposal = controller_propose(&public).unwrap();
        assert_eq!(
            proposal.disposition,
            ProposalDisposition::Abstain {
                reason: AbstainReason::NoFeasibleAction
            }
        );
    }

    #[test]
    fn acquisition_event_for_different_proposal_is_rejected() {
        let first = controller_propose(&input()).unwrap();
        let mut different = input();
        different.input_commitment = [0x22; 32];
        let second = controller_propose(&different).unwrap();
        let event = event_for(
            &second,
            AcquisitionStatus::Succeeded {
                outcome_state_one_ppm: 700_000,
            },
        );
        let truth = EvaluatorTruth {
            service_state_one: true,
            truth_manifest_commitment: [0x33; 32],
        };
        assert_eq!(
            score_evaluation(&first, &event, &truth, 10, 1),
            Err(BoundaryError::ProposalMismatch)
        );
    }

    #[test]
    fn abstention_cannot_be_followed_by_an_acquisition_event() {
        let mut public = input();
        public.candidates.clear();
        let proposal = controller_propose(&public).unwrap();
        let event = event_for(
            &proposal,
            AcquisitionStatus::Failed,
        );
        let truth = EvaluatorTruth {
            service_state_one: true,
            truth_manifest_commitment: [0x44; 32],
        };
        assert_eq!(
            score_evaluation(&proposal, &event, &truth, 10, 1),
            Err(BoundaryError::ProposalMismatch)
        );
    }

    #[test]
    fn duplicate_candidates_and_invalid_losses_fail_closed() {
        let mut public = input();
        public.candidates.push(public.candidates[0].clone());
        assert_eq!(
            controller_propose(&public),
            Err(BoundaryError::DuplicateActionId("uwb-reobserve".to_owned()))
        );

        let mut public = input();
        public.false_safe_loss_micros = 0;
        public.false_degraded_loss_micros = 0;
        assert_eq!(
            controller_propose(&public),
            Err(BoundaryError::InvalidLoss)
        );
    }

    #[test]
    fn truth_manifest_is_not_present_in_proposal_encoding() {
        let proposal = controller_propose(&input()).unwrap();
        let bytes = proposal.canonical_bytes();
        assert!(!bytes.windows(4).any(|window| window == [0xaa, 0xaa, 0xaa, 0xaa]));
        assert!(!bytes.windows(4).any(|window| window == [0xbb, 0xbb, 0xbb, 0xbb]));
    }
}
