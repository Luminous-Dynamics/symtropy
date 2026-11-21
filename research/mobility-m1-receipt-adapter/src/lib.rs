//! Thin M2→M1 receipt adapter. The pinned source modules are supplied by CI.
#[path = "../../../m1-src/benchmarks/mission-supervisor-reference/v0.1/src/lib.rs"]
pub mod m1;
#[path = "../../../m2-src/research/mobility-action-evidence/src/lib.rs"]
pub mod m2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QualifiedReceiptTime {
    Qualified {
        source_clock: m2::ClockBinding,
        comparison_clock: m2::ClockBinding,
        normalized_observed_at_ms: u64,
        normalization_commitment: Option<m2::Digest>,
    },
    Unqualified,
    Unknown,
}

#[derive(Debug, Eq, PartialEq)]
pub enum AdapterError {
    M1TicketMismatch,
    TimeUnknown,
    TimeUnqualified,
    TimeSourceClockMismatch,
    TimeComparisonClockMismatch,
    TimeNormalizationCommitmentMissing,
    OutcomeDispositionMismatch,
    Admission(m2::AdmissionError),
    Outcome(m2::OutcomeError),
    Supervisor(m1::SupervisorError),
}
impl From<m2::AdmissionError> for AdapterError {
    fn from(value: m2::AdmissionError) -> Self { Self::Admission(value) }
}
impl From<m2::OutcomeError> for AdapterError {
    fn from(value: m2::OutcomeError) -> Self { Self::Outcome(value) }
}
impl From<m1::SupervisorError> for AdapterError {
    fn from(value: m1::SupervisorError) -> Self { Self::Supervisor(value) }
}

/// Validate M2 admission and physical outcome, require a Mycelix-qualified
/// conversion into the M1 millisecond field, then let M1 apply the receipt.
/// This never casts a raw monotonic tick count into milliseconds.
pub fn apply_outcome_to_m1(
    supervisor: &mut m1::MissionSupervisor,
    m1_ticket: &m1::ActionTicket,
    m2_ticket: &m2::ActionTicket,
    live: &m2::LiveContext,
    evidence: &m2::PhysicalOutcomeEvidence,
    policy: m2::OutcomePolicy,
    receipt_time: &QualifiedReceiptTime,
) -> Result<m1::ReceiptDisposition, AdapterError> {
    if m1_ticket.mission_id != m2_ticket.mission_id
        || m1_ticket.subgoal_id != m2_ticket.subgoal_id
        || m1_ticket.action_id != m2_ticket.action_id
        || m1_ticket.attempt != m2_ticket.attempt
    {
        return Err(AdapterError::M1TicketMismatch);
    }
    m2::validate_ticket_applicability(m2_ticket, live)?;
    let disposition = m2::validate_physical_outcome(m2_ticket, evidence, policy)?;

    let observed_at_ms = match receipt_time {
        QualifiedReceiptTime::Unknown => return Err(AdapterError::TimeUnknown),
        QualifiedReceiptTime::Unqualified => return Err(AdapterError::TimeUnqualified),
        QualifiedReceiptTime::Qualified {
            source_clock, comparison_clock, normalized_observed_at_ms,
            normalization_commitment,
        } => {
            if source_clock != &evidence.clock {
                return Err(AdapterError::TimeSourceClockMismatch);
            }
            if comparison_clock != &m2_ticket.clock {
                return Err(AdapterError::TimeComparisonClockMismatch);
            }
            if normalization_commitment.is_none() {
                return Err(AdapterError::TimeNormalizationCommitmentMissing);
            }
            *normalized_observed_at_ms
        }
    };

    let verdict = match (&evidence.outcome, disposition) {
        (m2::PhysicalOutcome::Satisfied, m2::OutcomeDisposition::Accepted) => m1::GoalVerdict::Satisfied,
        (m2::PhysicalOutcome::NoEffect, m2::OutcomeDisposition::RecoveryRequired) => m1::GoalVerdict::NotSatisfied,
        (m2::PhysicalOutcome::PartialEffect | m2::PhysicalOutcome::Unknown,
            m2::OutcomeDisposition::ReconciliationRequired) => m1::GoalVerdict::Ambiguous,
        _ => return Err(AdapterError::OutcomeDispositionMismatch),
    };
    let receipt = m1::GoalEvidence {
        envelope: m1::ReceiptEnvelope {
            event_id: evidence.event_id.clone(),
            content_digest: m1::Digest(evidence.content_digest.0),
            source_id: evidence.source_id.clone(),
            source_sequence: evidence.source_sequence,
            observed_at_ms,
            observer_role: match evidence.observer_role {
                m2::ObserverRole::AgentSelfReport => m1::ObserverRole::AgentSelfReport,
                m2::ObserverRole::IndependentSensor => m1::ObserverRole::IndependentSensor,
                m2::ObserverRole::IndependentEvaluator => m1::ObserverRole::IndependentEvaluator,
            },
            confidence_basis_points: evidence.confidence_basis_points,
        },
        subgoal_id: m2_ticket.subgoal_id.clone(),
        action_id: m2_ticket.action_id.clone(),
        attempt: m2_ticket.attempt,
        predicate_id: m2_ticket.expected_effect_predicate_id.clone(),
        verdict,
    };
    Ok(supervisor.observe_goal(receipt)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> m2::Digest { m2::Digest([byte; 32]) }

    struct Fixture {
        supervisor: m1::MissionSupervisor,
        m1_ticket: m1::ActionTicket,
        m2_ticket: m2::ActionTicket,
        live: m2::LiveContext,
        evidence: m2::PhysicalOutcomeEvidence,
        policy: m2::OutcomePolicy,
        receipt_time: QualifiedReceiptTime,
    }

    fn fixture(outcome: m2::PhysicalOutcome) -> Fixture {
        let plan = m1::MissionPlan::new(
            "mission-adapter-1", 9_000,
            vec![m1::SubgoalSpec::new(
                "pick", "predicate.object-in-gripper", vec![],
                vec![m1::Digest([90; 32])],
            )],
        ).expect("valid M1 plan");
        let mut supervisor = m1::MissionSupervisor::new(plan);
        let m1_ticket = supervisor.dispatch_action("pick", "action-pick-1").expect("M1 dispatch");
        supervisor.report_execution(&m1_ticket, m1::ExecutionDisposition::Acknowledged)
            .expect("M1 acknowledgement");

        let clock = m2::ClockBinding {
            clock_id: "vehicle-monotonic-1".into(), boot_epoch_id: "vehicle-boot-8".into(),
        };
        let m2_ticket = m2::ActionTicket {
            ticket_commitment: digest(1), mission_id: m1_ticket.mission_id.clone(),
            mission_generation: 4, subgoal_id: m1_ticket.subgoal_id.clone(),
            action_id: m1_ticket.action_id.clone(), attempt: m1_ticket.attempt,
            action_digest: digest(2),
            expected_effect_predicate_id: "predicate.object-in-gripper".into(),
            platform: m2::PlatformBinding {
                platform_id: "robot-1".into(), configuration_digest: digest(3),
            },
            operation: m2::OperationBinding {
                odd_id: "odd-lab-cell".into(), odd_revision: digest(4),
                mode_id: "supervised-pick".into(), mode_revision: digest(5),
            },
            authority: m2::AuthorityBinding {
                authority_id: "operator-session-1".into(), epoch: 7,
                required_scope: "mission.pick".into(),
            },
            clock: clock.clone(), issued_at_ticks: 1_000, expires_at_ticks: 2_000,
            navigation: m2::NavigationRequirement {
                profile_digest: digest(6), required_service: m2::NavigationService::Degraded,
                spatial_frame_id: "frame.local-1".into(), source_id: "localizer-1".into(),
                minimum_source_sequence: 20, maximum_age_ticks: 100,
            },
            expected_observation_frame_id: "frame.local-1".into(),
            expected_calibration_id: "calibration-1".into(),
            expected_effect_digest: digest(7),
        };
        let live = m2::LiveContext {
            mission_id: m2_ticket.mission_id.clone(), mission_generation: m2_ticket.mission_generation,
            active_subgoal_id: m2_ticket.subgoal_id.clone(), active_action_id: m2_ticket.action_id.clone(),
            active_attempt: m2_ticket.attempt,
            active_effect_predicate_id: m2_ticket.expected_effect_predicate_id.clone(),
            platform: m2_ticket.platform.clone(), operation: m2_ticket.operation.clone(),
            authority_id: m2_ticket.authority.authority_id.clone(), authority_epoch: m2_ticket.authority.epoch,
            authority_revoked: false, authority_scopes: vec!["mission.pick".into()],
            clock: clock.clone(), now_ticks: 1_100,
            navigation_profile_digest: m2_ticket.navigation.profile_digest,
            navigation_service: m2::NavigationService::Qualified,
            navigation_frame_id: m2_ticket.navigation.spatial_frame_id.clone(),
            navigation_source_id: m2_ticket.navigation.source_id.clone(),
            navigation_source_sequence: 22,
            navigation_time: m2::NavigationTimeEvidence::Qualified {
                source_clock: clock.clone(), comparison_clock: clock.clone(),
                normalized_observed_at_ticks: Some(1_090), normalization_commitment: None,
            },
        };
        let evidence = m2::PhysicalOutcomeEvidence {
            ticket_commitment: m2_ticket.ticket_commitment, content_digest: digest(8),
            event_id: "physical-effect-1".into(), source_id: "independent-gripper-camera".into(),
            executing_controller_id: "arm-controller-1".into(),
            observer_role: m2::ObserverRole::IndependentSensor, source_sequence: 31,
            observed_at_ticks: 1_070, clock: clock.clone(),
            platform_configuration_digest: m2_ticket.platform.configuration_digest,
            observation_frame_id: m2_ticket.expected_observation_frame_id.clone(),
            calibration_id: m2_ticket.expected_calibration_id.clone(),
            effect_predicate_digest: m2_ticket.expected_effect_digest,
            confidence_basis_points: 9_900, outcome,
        };
        let policy = m2::OutcomePolicy {
            clock: clock.clone(), now_ticks: 1_100, maximum_age_ticks: 100,
            sequence_source_id: evidence.source_id.clone(), last_source_sequence: 30,
            minimum_confidence_basis_points: 9_000,
        };
        let receipt_time = QualifiedReceiptTime::Qualified {
            source_clock: evidence.clock.clone(), comparison_clock: m2_ticket.clock.clone(),
            normalized_observed_at_ms: 1_070, normalization_commitment: Some(digest(9)),
        };
        Fixture { supervisor, m1_ticket, m2_ticket, live, evidence, policy, receipt_time }
    }

    #[test]
    fn verified_effect_maps_own_digest_and_completes_m1_predicate() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let result = apply_outcome_to_m1(
            &mut f.supervisor, &f.m1_ticket, &f.m2_ticket, &f.live, &f.evidence,
            f.policy.clone(), &f.receipt_time,
        ).expect("qualified receipt");
        assert_eq!(result, m1::ReceiptDisposition::Applied);
        assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::Complete));
        assert!(f.supervisor.decisions().iter().any(|decision| matches!(
            &decision.kind,
            m1::DecisionKind::GoalCompleted { event_id, evidence_digest }
                if event_id == "physical-effect-1" && evidence_digest == &m1::Digest([8; 32])
        )));
    }

    #[test]
    fn no_effect_partial_and_unknown_map_to_non_success_states() {
        let mut no_effect = fixture(m2::PhysicalOutcome::NoEffect);
        apply_outcome_to_m1(
            &mut no_effect.supervisor, &no_effect.m1_ticket, &no_effect.m2_ticket,
            &no_effect.live, &no_effect.evidence, no_effect.policy.clone(), &no_effect.receipt_time,
        ).expect("no effect");
        assert_eq!(no_effect.supervisor.phase("pick"), Ok(m1::SubgoalPhase::RecoveryRequired));
        for outcome in [m2::PhysicalOutcome::PartialEffect, m2::PhysicalOutcome::Unknown] {
            let mut f = fixture(outcome);
            apply_outcome_to_m1(
                &mut f.supervisor, &f.m1_ticket, &f.m2_ticket, &f.live, &f.evidence,
                f.policy.clone(), &f.receipt_time,
            ).expect("ambiguous effect");
            assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::ReconciliationRequired));
        }
    }

    #[test]
    fn unknown_unqualified_and_uncommitted_time_never_reach_m1() {
        let mut qualifications = vec![QualifiedReceiptTime::Unknown, QualifiedReceiptTime::Unqualified];
        qualifications.push(QualifiedReceiptTime::Qualified {
            source_clock: m2::ClockBinding {
                clock_id: "vehicle-monotonic-1".into(), boot_epoch_id: "vehicle-boot-8".into(),
            },
            comparison_clock: m2::ClockBinding {
                clock_id: "vehicle-monotonic-1".into(), boot_epoch_id: "vehicle-boot-8".into(),
            },
            normalized_observed_at_ms: 1_070, normalization_commitment: None,
        });
        for qualification in qualifications {
            let mut f = fixture(m2::PhysicalOutcome::Satisfied);
            assert!(apply_outcome_to_m1(
                &mut f.supervisor, &f.m1_ticket, &f.m2_ticket, &f.live, &f.evidence,
                f.policy.clone(), &qualification,
            ).is_err());
            assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::AwaitingEvidence));
        }
    }

    #[test]
    fn wrong_time_clock_domain_is_rejected_without_mutating_m1() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let bad = QualifiedReceiptTime::Qualified {
            source_clock: m2::ClockBinding { clock_id: "other-clock".into(), boot_epoch_id: "other-boot".into() },
            comparison_clock: f.m2_ticket.clock.clone(), normalized_observed_at_ms: 1_070,
            normalization_commitment: Some(digest(9)),
        };
        assert_eq!(apply_outcome_to_m1(
            &mut f.supervisor, &f.m1_ticket, &f.m2_ticket, &f.live, &f.evidence,
            f.policy.clone(), &bad,
        ), Err(AdapterError::TimeSourceClockMismatch));
        assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::AwaitingEvidence));
    }

    #[test]
    fn stale_prior_m1_attempt_is_rejected_after_valid_m2_check() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        f.supervisor.observe_goal(m1::GoalEvidence {
            envelope: m1::ReceiptEnvelope {
                event_id: "no-effect-1".into(), content_digest: m1::Digest([11; 32]),
                source_id: "observer-a".into(), source_sequence: 1, observed_at_ms: 1_050,
                observer_role: m1::ObserverRole::IndependentSensor, confidence_basis_points: 9_900,
            },
            subgoal_id: f.m1_ticket.subgoal_id.clone(), action_id: f.m1_ticket.action_id.clone(),
            attempt: f.m1_ticket.attempt, predicate_id: "predicate.object-in-gripper".into(),
            verdict: m1::GoalVerdict::NotSatisfied,
        }).expect("first attempt no effect");
        f.supervisor.authorize_recovery(m1::RecoveryPermit {
            envelope: m1::ReceiptEnvelope {
                event_id: "recovery-1".into(), content_digest: m1::Digest([12; 32]),
                source_id: "safety-monitor".into(), source_sequence: 1, observed_at_ms: 1_060,
                observer_role: m1::ObserverRole::IndependentSensor, confidence_basis_points: 9_900,
            },
            subgoal_id: f.m1_ticket.subgoal_id.clone(), action_id: f.m1_ticket.action_id.clone(),
            attempt: f.m1_ticket.attempt, recovery_plan_digest: m1::Digest([90; 32]),
            safe_to_mutate: true,
        }).expect("recover first attempt");
        f.supervisor.dispatch_action("pick", "action-pick-2").expect("second action");
        let result = apply_outcome_to_m1(
            &mut f.supervisor, &f.m1_ticket, &f.m2_ticket, &f.live, &f.evidence,
            f.policy.clone(), &f.receipt_time,
        );
        assert_eq!(result, Err(AdapterError::Supervisor(m1::SupervisorError::ActionIdMismatch)));
        assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::Executing));
    }

    #[test]
    fn mismatched_m1_and_m2_tickets_are_rejected_before_processing() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let mut wrong = f.m1_ticket.clone();
        wrong.action_id = "different-action".into();
        assert_eq!(apply_outcome_to_m1(
            &mut f.supervisor, &wrong, &f.m2_ticket, &f.live, &f.evidence,
            f.policy.clone(), &f.receipt_time,
        ), Err(AdapterError::M1TicketMismatch));
        assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::AwaitingEvidence));
    }
}
