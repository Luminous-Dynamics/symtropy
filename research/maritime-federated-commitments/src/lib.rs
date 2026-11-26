//! Synthetic event-sourced lifecycle for cross-operator capacity commitments.
//!
//! No external booking, dispatch, TOS or custody API is invoked here.
//! The module projects append-only commitment histories and models a guard
//! that blocks new mutations while a prior external outcome is unknown.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Offer {
    pub commitment_id: String,
    pub revision: u32,
    pub controller_id: String,
    pub accepting_participant_id: String,
    pub resource_id: String,
    pub slot: u32,
    pub quantity: u64,
    pub unit: String,
    pub expires_at: i64,
    pub terms_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitmentEvent {
    Offered {
        offer: Offer,
        recorded_at: i64,
    },
    Counteroffered {
        parent_revision: u32,
        offer: Offer,
        recorded_at: i64,
    },
    Accepted {
        commitment_id: String,
        revision: u32,
        actor_id: String,
        recorded_at: i64,
    },
    Rejected {
        commitment_id: String,
        revision: u32,
        actor_id: String,
        recorded_at: i64,
    },
    Revoked {
        commitment_id: String,
        revision: u32,
        actor_id: String,
        effective_at: i64,
        recorded_at: i64,
    },
}

impl CommitmentEvent {
    fn recorded_at(&self) -> i64 {
        match self {
            Self::Offered { recorded_at, .. }
            | Self::Counteroffered { recorded_at, .. }
            | Self::Accepted { recorded_at, .. }
            | Self::Rejected { recorded_at, .. }
            | Self::Revoked { recorded_at, .. } => *recorded_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitmentState {
    Offered,
    Accepted,
    Rejected,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentProjection {
    pub offer: Offer,
    pub state: CommitmentState,
    pub accepted_at: Option<i64>,
    pub revoked_effective_at: Option<i64>,
    pub knowledge_cutoff: i64,
    pub last_recorded_at: i64,
}

impl CommitmentProjection {
    pub fn is_eligible(&self) -> bool {
        self.state == CommitmentState::Accepted
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitmentError {
    EmptyIdentifier,
    EmptyUnit,
    ZeroQuantity,
    InvalidExpiry,
    EmptyHistory,
    EventTimeRegression,
    UnexpectedInitialRevision(u32),
    CommitmentIdentityMismatch,
    RevisionMismatch,
    RevisionGap,
    ControllerChanged,
    OfferNotOpen,
    WrongAcceptingParticipant,
    WrongController,
    AcceptanceAfterExpiry,
    OfferExpired,
    RevocationBeforeAcceptance,
    ConflictingTermsForRevision,
}

impl std::fmt::Display for CommitmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CommitmentError {}

fn validate_offer(offer: &Offer, recorded_at: i64) -> Result<(), CommitmentError> {
    if offer.commitment_id.is_empty()
        || offer.controller_id.is_empty()
        || offer.accepting_participant_id.is_empty()
        || offer.resource_id.is_empty()
        || offer.terms_digest.is_empty()
    {
        return Err(CommitmentError::EmptyIdentifier);
    }
    if offer.unit.is_empty() {
        return Err(CommitmentError::EmptyUnit);
    }
    if offer.quantity == 0 {
        return Err(CommitmentError::ZeroQuantity);
    }
    if offer.expires_at <= recorded_at {
        return Err(CommitmentError::InvalidExpiry);
    }
    Ok(())
}

/// Reconstructs the commitment state known by the receiving organization at
/// knowledge_cutoff. Events recorded later are excluded, even if their
/// effective time is earlier. A later projection may therefore differ.
pub fn project_commitment(
    events: &[CommitmentEvent],
    knowledge_cutoff: i64,
) -> Result<CommitmentProjection, CommitmentError> {
    let mut current_offer: Option<Offer> = None;
    let mut state: Option<CommitmentState> = None;
    let mut accepted_at = None;
    let mut revoked_effective_at = None;
    let mut last_recorded_at = None;

    for event in events {
        let recorded_at = event.recorded_at();
        if recorded_at > knowledge_cutoff {
            continue;
        }
        if last_recorded_at.is_some_and(|previous| recorded_at < previous) {
            return Err(CommitmentError::EventTimeRegression);
        }
        last_recorded_at = Some(recorded_at);

        match event {
            CommitmentEvent::Offered { offer, .. } => {
                if current_offer.is_some() {
                    return Err(CommitmentError::ConflictingTermsForRevision);
                }
                if offer.revision != 1 {
                    return Err(CommitmentError::UnexpectedInitialRevision(offer.revision));
                }
                validate_offer(offer, recorded_at)?;
                current_offer = Some(offer.clone());
                state = Some(CommitmentState::Offered);
            }
            CommitmentEvent::Counteroffered {
                parent_revision,
                offer,
                ..
            } => {
                let current = current_offer
                    .as_ref()
                    .ok_or(CommitmentError::EmptyHistory)?;
                let current_state = state.ok_or(CommitmentError::EmptyHistory)?;
                if recorded_at >= current.expires_at {
                    return Err(CommitmentError::OfferExpired);
                }
                if matches!(
                    current_state,
                    CommitmentState::Accepted | CommitmentState::Revoked
                ) {
                    return Err(CommitmentError::OfferNotOpen);
                }
                if offer.commitment_id != current.commitment_id {
                    return Err(CommitmentError::CommitmentIdentityMismatch);
                }
                if offer.controller_id != current.controller_id {
                    return Err(CommitmentError::ControllerChanged);
                }
                if *parent_revision != current.revision {
                    return Err(CommitmentError::RevisionMismatch);
                }
                if offer.revision != current.revision.saturating_add(1) {
                    return Err(CommitmentError::RevisionGap);
                }
                validate_offer(offer, recorded_at)?;
                current_offer = Some(offer.clone());
                state = Some(CommitmentState::Offered);
                accepted_at = None;
                revoked_effective_at = None;
            }
            CommitmentEvent::Accepted {
                commitment_id,
                revision,
                actor_id,
                ..
            } => {
                let current = current_offer
                    .as_ref()
                    .ok_or(CommitmentError::EmptyHistory)?;
                if commitment_id != &current.commitment_id {
                    return Err(CommitmentError::CommitmentIdentityMismatch);
                }
                if *revision != current.revision {
                    return Err(CommitmentError::RevisionMismatch);
                }
                if actor_id != &current.accepting_participant_id {
                    return Err(CommitmentError::WrongAcceptingParticipant);
                }
                if state != Some(CommitmentState::Offered) {
                    return Err(CommitmentError::OfferNotOpen);
                }
                if recorded_at >= current.expires_at {
                    return Err(CommitmentError::AcceptanceAfterExpiry);
                }
                state = Some(CommitmentState::Accepted);
                accepted_at = Some(recorded_at);
            }
            CommitmentEvent::Rejected {
                commitment_id,
                revision,
                actor_id,
                ..
            } => {
                let current = current_offer
                    .as_ref()
                    .ok_or(CommitmentError::EmptyHistory)?;
                if commitment_id != &current.commitment_id {
                    return Err(CommitmentError::CommitmentIdentityMismatch);
                }
                if *revision != current.revision {
                    return Err(CommitmentError::RevisionMismatch);
                }
                if actor_id != &current.accepting_participant_id {
                    return Err(CommitmentError::WrongAcceptingParticipant);
                }
                if state != Some(CommitmentState::Offered) {
                    return Err(CommitmentError::OfferNotOpen);
                }
                if recorded_at >= current.expires_at {
                    return Err(CommitmentError::OfferExpired);
                }
                state = Some(CommitmentState::Rejected);
            }
            CommitmentEvent::Revoked {
                commitment_id,
                revision,
                actor_id,
                effective_at,
                ..
            } => {
                let current = current_offer
                    .as_ref()
                    .ok_or(CommitmentError::EmptyHistory)?;
                if commitment_id != &current.commitment_id {
                    return Err(CommitmentError::CommitmentIdentityMismatch);
                }
                if *revision != current.revision {
                    return Err(CommitmentError::RevisionMismatch);
                }
                if actor_id != &current.controller_id {
                    return Err(CommitmentError::WrongController);
                }
                if state != Some(CommitmentState::Accepted) {
                    return Err(CommitmentError::OfferNotOpen);
                }
                if accepted_at.is_some_and(|accepted| *effective_at < accepted) {
                    return Err(CommitmentError::RevocationBeforeAcceptance);
                }
                revoked_effective_at = Some(*effective_at);
                if *effective_at <= knowledge_cutoff {
                    state = Some(CommitmentState::Revoked);
                }
            }
        }
    }

    let offer = current_offer.ok_or(CommitmentError::EmptyHistory)?;
    let mut state = state.ok_or(CommitmentError::EmptyHistory)?;
    if state == CommitmentState::Accepted || state == CommitmentState::Offered {
        if revoked_effective_at.is_some_and(|effective| effective <= knowledge_cutoff) {
            state = CommitmentState::Revoked;
        } else if knowledge_cutoff >= offer.expires_at {
            state = CommitmentState::Expired;
        }
    }

    Ok(CommitmentProjection {
        offer,
        state,
        accepted_at,
        revoked_effective_at,
        knowledge_cutoff,
        last_recorded_at: last_recorded_at.ok_or(CommitmentError::EmptyHistory)?,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationState {
    Ready,
    OutcomeUnknown {
        action_id: String,
        idempotency_key: String,
    },
    Executed {
        action_id: String,
        idempotency_key: String,
    },
    NotExecuted {
        action_id: String,
        idempotency_key: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttemptDisposition {
    DispatchOnce,
    IdempotentReplayOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationError {
    EmptyIdentifier,
    UnknownOutcomeMustBeReconciled,
    AlreadyExecuted,
    ReconciliationIdentityMismatch,
    NoUnknownOutcome,
}

impl std::fmt::Display for MutationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for MutationError {}

/// Local guard only: it never invokes the external system.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MutationGuard {
    state: Option<MutationState>,
}

impl MutationGuard {
    pub fn state(&self) -> MutationState {
        self.state.clone().unwrap_or(MutationState::Ready)
    }

    pub fn begin_attempt(
        &mut self,
        action_id: &str,
        idempotency_key: &str,
    ) -> Result<AttemptDisposition, MutationError> {
        if action_id.is_empty() || idempotency_key.is_empty() {
            return Err(MutationError::EmptyIdentifier);
        }

        match self.state() {
            MutationState::Ready | MutationState::NotExecuted { .. } => {
                self.state = Some(MutationState::OutcomeUnknown {
                    action_id: action_id.to_owned(),
                    idempotency_key: idempotency_key.to_owned(),
                });
                Ok(AttemptDisposition::DispatchOnce)
            }
            MutationState::OutcomeUnknown {
                action_id: pending_action,
                idempotency_key: pending_key,
            } if pending_action == action_id && pending_key == idempotency_key => {
                Ok(AttemptDisposition::IdempotentReplayOnly)
            }
            MutationState::OutcomeUnknown { .. } => {
                Err(MutationError::UnknownOutcomeMustBeReconciled)
            }
            MutationState::Executed { .. } => Err(MutationError::AlreadyExecuted),
        }
    }

    pub fn reconcile(
        &mut self,
        action_id: &str,
        idempotency_key: &str,
        executed: bool,
    ) -> Result<(), MutationError> {
        match self.state() {
            MutationState::OutcomeUnknown {
                action_id: pending_action,
                idempotency_key: pending_key,
            } if pending_action == action_id && pending_key == idempotency_key => {
                self.state = Some(if executed {
                    MutationState::Executed {
                        action_id: action_id.to_owned(),
                        idempotency_key: idempotency_key.to_owned(),
                    }
                } else {
                    MutationState::NotExecuted {
                        action_id: action_id.to_owned(),
                        idempotency_key: idempotency_key.to_owned(),
                    }
                });
                Ok(())
            }
            MutationState::OutcomeUnknown { .. } => {
                Err(MutationError::ReconciliationIdentityMismatch)
            }
            _ => Err(MutationError::NoUnknownOutcome),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(revision: u32, expires_at: i64, slot: u32, digest: &str) -> Offer {
        Offer {
            commitment_id: "COMMIT-1".to_owned(),
            revision,
            controller_id: "TERMINAL-A".to_owned(),
            accepting_participant_id: "CARRIER-B".to_owned(),
            resource_id: "BERTH-2".to_owned(),
            slot,
            quantity: 12,
            unit: "container".to_owned(),
            expires_at,
            terms_digest: digest.to_owned(),
        }
    }

    fn accepted_history() -> Vec<CommitmentEvent> {
        vec![
            CommitmentEvent::Offered {
                offer: offer(1, 200, 4, "terms-v1"),
                recorded_at: 80,
            },
            CommitmentEvent::Accepted {
                commitment_id: "COMMIT-1".to_owned(),
                revision: 1,
                actor_id: "CARRIER-B".to_owned(),
                recorded_at: 100,
            },
        ]
    }

    #[test]
    fn accepted_commitment_is_eligible_before_expiry_or_revocation() {
        let projection = project_commitment(&accepted_history(), 150).unwrap();
        assert_eq!(projection.state, CommitmentState::Accepted);
        assert!(projection.is_eligible());
        assert_eq!(projection.offer.slot, 4);
        assert_eq!(projection.offer.quantity, 12);
    }

    #[test]
    fn wrong_participant_cannot_accept() {
        let events = vec![
            CommitmentEvent::Offered {
                offer: offer(1, 200, 4, "terms-v1"),
                recorded_at: 80,
            },
            CommitmentEvent::Accepted {
                commitment_id: "COMMIT-1".to_owned(),
                revision: 1,
                actor_id: "OTHER-CARRIER".to_owned(),
                recorded_at: 100,
            },
        ];
        assert_eq!(
            project_commitment(&events, 150),
            Err(CommitmentError::WrongAcceptingParticipant)
        );
    }

    #[test]
    fn acceptance_after_expiry_is_rejected() {
        let events = vec![
            CommitmentEvent::Offered {
                offer: offer(1, 100, 4, "terms-v1"),
                recorded_at: 80,
            },
            CommitmentEvent::Accepted {
                commitment_id: "COMMIT-1".to_owned(),
                revision: 1,
                actor_id: "CARRIER-B".to_owned(),
                recorded_at: 100,
            },
        ];
        assert_eq!(
            project_commitment(&events, 150),
            Err(CommitmentError::AcceptanceAfterExpiry)
        );
    }

    #[test]
    fn counteroffer_supersedes_terms_and_resets_acceptance() {
        let events = vec![
            CommitmentEvent::Offered {
                offer: offer(1, 200, 4, "terms-v1"),
                recorded_at: 80,
            },
            CommitmentEvent::Accepted {
                commitment_id: "COMMIT-1".to_owned(),
                revision: 1,
                actor_id: "CARRIER-B".to_owned(),
                recorded_at: 90,
            },
            CommitmentEvent::Counteroffered {
                parent_revision: 1,
                offer: offer(2, 250, 5, "terms-v2"),
                recorded_at: 110,
            },
        ];
        let projection = project_commitment(&events, 120).unwrap();
        assert_eq!(projection.offer.revision, 2);
        assert_eq!(projection.offer.slot, 5);
        assert_eq!(projection.state, CommitmentState::Offered);
        assert!(!projection.is_eligible());
    }

    #[test]
    fn counteroffer_after_expiry_is_rejected() {
        let events = vec![
            CommitmentEvent::Offered {
                offer: offer(1, 100, 4, "terms-v1"),
                recorded_at: 80,
            },
            CommitmentEvent::Counteroffered {
                parent_revision: 1,
                offer: offer(2, 200, 5, "terms-v2"),
                recorded_at: 100,
            },
        ];
        assert_eq!(
            project_commitment(&events, 150),
            Err(CommitmentError::OfferExpired)
        );
    }

    #[test]
    fn revocation_effective_by_cutoff_removes_eligibility() {
        let mut events = accepted_history();
        events.push(CommitmentEvent::Revoked {
            commitment_id: "COMMIT-1".to_owned(),
            revision: 1,
            actor_id: "TERMINAL-A".to_owned(),
            effective_at: 140,
            recorded_at: 145,
        });
        let projection = project_commitment(&events, 150).unwrap();
        assert_eq!(projection.state, CommitmentState::Revoked);
        assert!(!projection.is_eligible());
    }

    #[test]
    fn later_recorded_revocation_does_not_rewrite_earlier_receiver_projection() {
        let mut events = accepted_history();
        events.push(CommitmentEvent::Revoked {
            commitment_id: "COMMIT-1".to_owned(),
            revision: 1,
            actor_id: "TERMINAL-A".to_owned(),
            effective_at: 140,
            recorded_at: 160,
        });
        let earlier = project_commitment(&events, 150).unwrap();
        let later = project_commitment(&events, 170).unwrap();
        assert_eq!(earlier.state, CommitmentState::Accepted);
        assert_eq!(later.state, CommitmentState::Revoked);
    }

    #[test]
    fn expired_commitment_is_not_eligible() {
        let projection = project_commitment(&accepted_history(), 200).unwrap();
        assert_eq!(projection.state, CommitmentState::Expired);
        assert!(!projection.is_eligible());
    }

    #[test]
    fn unknown_outcome_blocks_distinct_mutation() {
        let mut guard = MutationGuard::default();
        assert_eq!(
            guard.begin_attempt("ACTION-1", "IDEMPOTENCY-1"),
            Ok(AttemptDisposition::DispatchOnce)
        );
        assert_eq!(
            guard.begin_attempt("ACTION-2", "IDEMPOTENCY-2"),
            Err(MutationError::UnknownOutcomeMustBeReconciled)
        );
        assert_eq!(
            guard.begin_attempt("ACTION-1", "IDEMPOTENCY-1"),
            Ok(AttemptDisposition::IdempotentReplayOnly)
        );
    }

    #[test]
    fn reconciliation_must_match_unknown_action_and_key() {
        let mut guard = MutationGuard::default();
        guard.begin_attempt("ACTION-1", "IDEMPOTENCY-1").unwrap();
        assert_eq!(
            guard.reconcile("ACTION-OTHER", "IDEMPOTENCY-1", false),
            Err(MutationError::ReconciliationIdentityMismatch)
        );
        guard.reconcile("ACTION-1", "IDEMPOTENCY-1", false).unwrap();
        assert_eq!(
            guard.begin_attempt("ACTION-2", "IDEMPOTENCY-2"),
            Ok(AttemptDisposition::DispatchOnce)
        );
    }

    #[test]
    fn executed_action_cannot_be_started_again() {
        let mut guard = MutationGuard::default();
        guard.begin_attempt("ACTION-1", "IDEMPOTENCY-1").unwrap();
        guard.reconcile("ACTION-1", "IDEMPOTENCY-1", true).unwrap();
        assert_eq!(
            guard.begin_attempt("ACTION-2", "IDEMPOTENCY-2"),
            Err(MutationError::AlreadyExecuted)
        );
    }
}
