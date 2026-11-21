//! Thin M2→M1 receipt adapter. The pinned source modules are supplied by CI.
#[path = "../../../m1-src/benchmarks/mission-supervisor-reference/v0.1/src/lib.rs"]
pub mod m1;
#[path = "../../../m2-src/research/mobility-action-evidence/src/lib.rs"]
pub mod m2;

/// The exact Mycelix Position time-correlation owner pinned by the adapter CI.
pub use positioning::time_correlation_v2 as mycelix_time;

#[derive(Clone, Debug, PartialEq)]
pub enum QualifiedReceiptTime {
    Qualified {
        correlation: Box<mycelix_time::ClockCorrelationV2>,
        normalized: Box<mycelix_time::NormalizedInstantV2>,
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
    TimeDestinationUnitMismatch,
    TimeValueNotExactMilliseconds,
    TimeUncertaintyOutOfRange,
    TimeCommitmentEncoding,
    MycelixTime(mycelix_time::TimeCorrelationError),
    OutcomeDispositionMismatch,
    Admission(m2::AdmissionError),
    Outcome(m2::OutcomeError),
    Supervisor(m1::SupervisorError),
}
impl From<m2::AdmissionError> for AdapterError {
    fn from(value: m2::AdmissionError) -> Self {
        Self::Admission(value)
    }
}
impl From<m2::OutcomeError> for AdapterError {
    fn from(value: m2::OutcomeError) -> Self {
        Self::Outcome(value)
    }
}
impl From<m1::SupervisorError> for AdapterError {
    fn from(value: m1::SupervisorError) -> Self {
        Self::Supervisor(value)
    }
}

fn digest_from_mycelix_commitment(
    commitment: &mycelix_time::TimeCommitmentV2,
) -> Result<m1::Digest, AdapterError> {
    if commitment.algorithm != "sha256" || commitment.digest.len() != 64 {
        return Err(AdapterError::TimeCommitmentEncoding);
    }
    let mut bytes = [0u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&commitment.digest[start..start + 2], 16)
            .map_err(|_| AdapterError::TimeCommitmentEncoding)?;
    }
    Ok(m1::Digest(bytes))
}

fn canonical_time_scale_id(scale: &mycelix_time::TimeScaleIdV2) -> String {
    match scale {
        mycelix_time::TimeScaleIdV2::Tai => "tai".into(),
        mycelix_time::TimeScaleIdV2::Tt => "tt".into(),
        mycelix_time::TimeScaleIdV2::Tdb => "tdb".into(),
        mycelix_time::TimeScaleIdV2::Gps => "gps".into(),
        mycelix_time::TimeScaleIdV2::UnixTime { profile_version } => {
            format!("unix-time@{profile_version}")
        }
        mycelix_time::TimeScaleIdV2::Named {
            authority,
            code,
            version,
        } => format!("named({authority}:{code}:{version})"),
    }
}

/// Validate the Mycelix correlation and preserve its source, profile and
/// normalized-output commitments in M1. M1 currently accepts integer
/// milliseconds, so every other output unit or fractional value is rejected.
fn qualify_receipt_time(
    receipt_time: &QualifiedReceiptTime,
    evidence_clock: &m2::ClockBinding,
    expected_subject_ref: &str,
) -> Result<(u64, m1::ReceiptTimeQualification), AdapterError> {
    let (correlation, normalized) = match receipt_time {
        QualifiedReceiptTime::Unknown => return Err(AdapterError::TimeUnknown),
        QualifiedReceiptTime::Unqualified => return Err(AdapterError::TimeUnqualified),
        QualifiedReceiptTime::Qualified {
            correlation,
            normalized,
        } => (correlation, normalized),
    };

    normalized
        .validate_against(correlation)
        .map_err(AdapterError::MycelixTime)?;

    let (subject_ref, source_clock_id, source_boot_epoch_id) =
        match &normalized.source_reading.basis {
            mycelix_time::TimeBasisV2::Clock(mycelix_time::ClockDomainV2::Monotonic {
                subject_ref,
                clock_id,
                boot_epoch_id,
            }) => (subject_ref, clock_id, boot_epoch_id),
            _ => return Err(AdapterError::TimeSourceClockMismatch),
        };
    if subject_ref != expected_subject_ref
        || source_clock_id != &evidence_clock.clock_id
        || source_boot_epoch_id != &evidence_clock.boot_epoch_id
    {
        return Err(AdapterError::TimeSourceClockMismatch);
    }

    let (scale, epoch_id) = match &normalized.normalized_reading.basis {
        mycelix_time::TimeBasisV2::Scale { scale, epoch_id } => (scale, epoch_id),
        mycelix_time::TimeBasisV2::Clock(_) => {
            return Err(AdapterError::TimeDestinationUnitMismatch);
        }
    };
    if correlation.destination_unit != mycelix_time::TimeUnitV2::Milliseconds
        || normalized.normalized_reading.unit != mycelix_time::TimeUnitV2::Milliseconds
    {
        return Err(AdapterError::TimeDestinationUnitMismatch);
    }

    let value = normalized.normalized_reading.value;
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= u64::MAX as f64 {
        return Err(AdapterError::TimeValueNotExactMilliseconds);
    }

    let uncertainty_ns = normalized.normalized_reading.uncertainty_bound_seconds * 1_000_000_000.0;
    if !uncertainty_ns.is_finite() || uncertainty_ns < 0.0 || uncertainty_ns >= u64::MAX as f64 {
        return Err(AdapterError::TimeUncertaintyOutOfRange);
    }
    // Round upward: the mission receipt must never understate the Mycelix hard bound.
    let uncertainty_bound_nanoseconds = uncertainty_ns.ceil() as u64;

    let time_qualification = m1::ReceiptTimeQualification {
        source_clock_id: format!("{subject_ref}/{source_clock_id}"),
        source_boot_epoch_id: source_boot_epoch_id.clone(),
        comparison_basis_id: canonical_time_scale_id(scale),
        comparison_epoch_id: epoch_id.clone(),
        source_time_commitment: digest_from_mycelix_commitment(
            &normalized.source_reading.content_commitment,
        )?,
        correlation_commitment: digest_from_mycelix_commitment(
            &correlation.correlation_commitment,
        )?,
        normalized_time_commitment: digest_from_mycelix_commitment(
            &normalized.normalized_reading.content_commitment,
        )?,
        uncertainty_bound_nanoseconds,
    };
    Ok((value as u64, time_qualification))
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

    let (observed_at_ms, time_qualification) = qualify_receipt_time(
        receipt_time,
        &evidence.clock,
        &m2_ticket.platform.platform_id,
    )?;

    let verdict = match (&evidence.outcome, disposition) {
        (m2::PhysicalOutcome::Satisfied, m2::OutcomeDisposition::Accepted) => {
            m1::GoalVerdict::Satisfied
        }
        (m2::PhysicalOutcome::NoEffect, m2::OutcomeDisposition::RecoveryRequired) => {
            m1::GoalVerdict::NotSatisfied
        }
        (
            m2::PhysicalOutcome::PartialEffect | m2::PhysicalOutcome::Unknown,
            m2::OutcomeDisposition::ReconciliationRequired,
        ) => m1::GoalVerdict::Ambiguous,
        _ => return Err(AdapterError::OutcomeDispositionMismatch),
    };
    let receipt = m1::GoalEvidence {
        envelope: m1::ReceiptEnvelope {
            event_id: evidence.event_id.clone(),
            content_digest: m1::Digest(evidence.content_digest.0),
            source_id: evidence.source_id.clone(),
            source_sequence: evidence.source_sequence,
            observed_at_ms,
            time_qualification,
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

    fn digest(byte: u8) -> m2::Digest {
        m2::Digest([byte; 32])
    }

    fn time_commitment(byte: u8) -> mycelix_time::TimeCommitmentV2 {
        mycelix_time::TimeCommitmentV2 {
            algorithm: "sha256".into(),
            digest: format!("{byte:02x}").repeat(32),
        }
    }

    fn qualified_receipt_time(
        clock: &m2::ClockBinding,
        subject_ref: &str,
        scale: mycelix_time::TimeScaleIdV2,
    ) -> QualifiedReceiptTime {
        let source_clock = mycelix_time::ClockDomainV2::Monotonic {
            subject_ref: subject_ref.into(),
            clock_id: clock.clock_id.clone(),
            boot_epoch_id: clock.boot_epoch_id.clone(),
        };
        let destination_basis = mycelix_time::TimeBasisV2::Scale {
            scale,
            epoch_id: "mission-epoch-1970".into(),
        };
        let source_commitment = time_commitment(0xa1);
        let correlation_commitment = time_commitment(0xa2);
        let output_commitment = time_commitment(0xa3);
        let correlation = mycelix_time::ClockCorrelationV2 {
            correlation_id: "robot-clock-correlation-v1".into(),
            source_clock: source_clock.clone(),
            destination_basis: destination_basis.clone(),
            destination_unit: mycelix_time::TimeUnitV2::Milliseconds,
            profile_id: "robot-monotonic-to-tai".into(),
            profile_version: "1".into(),
            calibration_or_kernel_commitment: time_commitment(0xc1),
            valid_source_interval: mycelix_time::SourceClockValidityV2 {
                start_value: 1_000.0,
                end_value: 2_000.0,
                unit: mycelix_time::TimeUnitV2::DeviceTicks,
                basis: source_clock.clone(),
            },
            uncertainty: mycelix_time::ClockCorrelationUncertaintyV2 {
                maximum_error_bound_seconds: 0.005,
                standard_deviation_seconds: Some(0.002),
            },
            correlation_commitment: correlation_commitment.clone(),
        };
        let source_reading = mycelix_time::QualifiedInstantV2 {
            value: 1_070.0,
            unit: mycelix_time::TimeUnitV2::DeviceTicks,
            basis: mycelix_time::TimeBasisV2::Clock(source_clock),
            uncertainty_bound_in_reading_units: 0.25,
            uncertainty_bound_seconds: 0.001,
            content_commitment: source_commitment.clone(),
        };
        let normalized_reading = mycelix_time::QualifiedInstantV2 {
            value: 1_070_000.0,
            unit: mycelix_time::TimeUnitV2::Milliseconds,
            basis: destination_basis,
            uncertainty_bound_in_reading_units: 0.5,
            uncertainty_bound_seconds: 0.006_000_001,
            content_commitment: output_commitment.clone(),
        };
        let normalized = mycelix_time::NormalizedInstantV2 {
            source_reading,
            normalized_reading,
            correlation_id: correlation.correlation_id.clone(),
            correlation_commitment,
            input_commitment: source_commitment,
            output_commitment,
        };
        QualifiedReceiptTime::Qualified {
            correlation: Box::new(correlation),
            normalized: Box::new(normalized),
        }
    }

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
            "mission-adapter-1",
            9_000,
            vec![m1::SubgoalSpec::new(
                "pick",
                "predicate.object-in-gripper",
                vec![],
                vec![m1::Digest([90; 32])],
            )],
        )
        .expect("valid M1 plan");
        let mut supervisor = m1::MissionSupervisor::new(plan);
        let m1_ticket = supervisor
            .dispatch_action("pick", "action-pick-1")
            .expect("M1 dispatch");
        supervisor
            .report_execution(&m1_ticket, m1::ExecutionDisposition::Acknowledged)
            .expect("M1 acknowledgement");

        let clock = m2::ClockBinding {
            clock_id: "vehicle-monotonic-1".into(),
            boot_epoch_id: "vehicle-boot-8".into(),
        };
        let m2_ticket = m2::ActionTicket {
            ticket_commitment: digest(1),
            mission_id: m1_ticket.mission_id.clone(),
            mission_generation: 4,
            subgoal_id: m1_ticket.subgoal_id.clone(),
            action_id: m1_ticket.action_id.clone(),
            attempt: m1_ticket.attempt,
            action_digest: digest(2),
            expected_effect_predicate_id: "predicate.object-in-gripper".into(),
            platform: m2::PlatformBinding {
                platform_id: "robot-1".into(),
                configuration_digest: digest(3),
            },
            operation: m2::OperationBinding {
                odd_id: "odd-lab-cell".into(),
                odd_revision: digest(4),
                mode_id: "supervised-pick".into(),
                mode_revision: digest(5),
            },
            authority: m2::AuthorityBinding {
                authority_id: "operator-session-1".into(),
                epoch: 7,
                required_scope: "mission.pick".into(),
            },
            clock: clock.clone(),
            issued_at_ticks: 1_000,
            expires_at_ticks: 2_000,
            navigation: m2::NavigationRequirement {
                profile_digest: digest(6),
                required_service: m2::NavigationService::Degraded,
                spatial_frame_id: "frame.local-1".into(),
                source_id: "localizer-1".into(),
                minimum_source_sequence: 20,
                maximum_age_ticks: 100,
            },
            expected_observation_frame_id: "frame.local-1".into(),
            expected_calibration_id: "calibration-1".into(),
            expected_effect_digest: digest(7),
            expected_observer_failure_domains: vec![
                "gripper-camera".into(),
                "observer-processor".into(),
                "observer-power".into(),
            ],
            observer_independence_policy_digest: digest(40),
        };
        let live = m2::LiveContext {
            mission_id: m2_ticket.mission_id.clone(),
            mission_generation: m2_ticket.mission_generation,
            active_subgoal_id: m2_ticket.subgoal_id.clone(),
            active_action_id: m2_ticket.action_id.clone(),
            active_attempt: m2_ticket.attempt,
            active_effect_predicate_id: m2_ticket.expected_effect_predicate_id.clone(),
            platform: m2_ticket.platform.clone(),
            operation: m2_ticket.operation.clone(),
            authority_id: m2_ticket.authority.authority_id.clone(),
            authority_epoch: m2_ticket.authority.epoch,
            authority_revoked: false,
            authority_scopes: vec!["mission.pick".into()],
            clock: clock.clone(),
            now_ticks: 1_100,
            navigation_profile_digest: m2_ticket.navigation.profile_digest,
            navigation_service: m2::NavigationService::Qualified,
            navigation_frame_id: m2_ticket.navigation.spatial_frame_id.clone(),
            navigation_source_id: m2_ticket.navigation.source_id.clone(),
            navigation_source_sequence: 22,
            navigation_time: m2::NavigationTimeEvidence::Qualified {
                source_clock: clock.clone(),
                comparison_clock: clock.clone(),
                normalized_observed_at_ticks: Some(1_090),
                normalization_commitment: None,
            },
        };
        let evidence = m2::PhysicalOutcomeEvidence {
            ticket_commitment: m2_ticket.ticket_commitment,
            content_digest: digest(8),
            event_id: "physical-effect-1".into(),
            source_id: "independent-gripper-camera".into(),
            executing_controller_id: "arm-controller-1".into(),
            observer_failure_domains: vec![
                "gripper-camera".into(),
                "observer-processor".into(),
                "observer-power".into(),
            ],
            observer_role: m2::ObserverRole::IndependentSensor,
            source_sequence: 31,
            observed_at_ticks: 1_070,
            clock: clock.clone(),
            platform_configuration_digest: m2_ticket.platform.configuration_digest,
            observation_frame_id: m2_ticket.expected_observation_frame_id.clone(),
            calibration_id: m2_ticket.expected_calibration_id.clone(),
            effect_predicate_digest: m2_ticket.expected_effect_digest,
            confidence_basis_points: 9_900,
            outcome,
        };
        let policy = m2::OutcomePolicy {
            clock: clock.clone(),
            now_ticks: 1_100,
            maximum_age_ticks: 100,
            sequence_source_id: evidence.source_id.clone(),
            last_source_sequence: 30,
            minimum_confidence_basis_points: 9_000,
            execution_failure_domains: vec![
                "arm-controller".into(),
                "vehicle-bus".into(),
                "vehicle-power".into(),
            ],
            expected_observer_failure_domains: vec![
                "gripper-camera".into(),
                "observer-processor".into(),
                "observer-power".into(),
            ],
            observer_independence_policy_digest: digest(40),
        };
        let receipt_time = qualified_receipt_time(
            &evidence.clock,
            &m2_ticket.platform.platform_id,
            mycelix_time::TimeScaleIdV2::Tai,
        );
        Fixture {
            supervisor,
            m1_ticket,
            m2_ticket,
            live,
            evidence,
            policy,
            receipt_time,
        }
    }

    #[test]
    fn verified_effect_maps_own_digest_and_completes_m1_predicate() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let result = apply_outcome_to_m1(
            &mut f.supervisor,
            &f.m1_ticket,
            &f.m2_ticket,
            &f.live,
            &f.evidence,
            f.policy.clone(),
            &f.receipt_time,
        )
        .expect("qualified receipt");
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
            &mut no_effect.supervisor,
            &no_effect.m1_ticket,
            &no_effect.m2_ticket,
            &no_effect.live,
            &no_effect.evidence,
            no_effect.policy.clone(),
            &no_effect.receipt_time,
        )
        .expect("no effect");
        assert_eq!(
            no_effect.supervisor.phase("pick"),
            Ok(m1::SubgoalPhase::RecoveryRequired)
        );
        for outcome in [
            m2::PhysicalOutcome::PartialEffect,
            m2::PhysicalOutcome::Unknown,
        ] {
            let mut f = fixture(outcome);
            apply_outcome_to_m1(
                &mut f.supervisor,
                &f.m1_ticket,
                &f.m2_ticket,
                &f.live,
                &f.evidence,
                f.policy.clone(),
                &f.receipt_time,
            )
            .expect("ambiguous effect");
            assert_eq!(
                f.supervisor.phase("pick"),
                Ok(m1::SubgoalPhase::ReconciliationRequired)
            );
        }
    }

    #[test]
    fn unknown_unqualified_and_uncommitted_time_never_reach_m1() {
        let mut qualifications = vec![
            QualifiedReceiptTime::Unknown,
            QualifiedReceiptTime::Unqualified,
        ];
        let f = fixture(m2::PhysicalOutcome::Satisfied);
        let mut uncommitted = qualified_receipt_time(
            &f.evidence.clock,
            &f.m2_ticket.platform.platform_id,
            mycelix_time::TimeScaleIdV2::Tai,
        );
        if let QualifiedReceiptTime::Qualified { correlation, .. } = &mut uncommitted {
            correlation.correlation_commitment.digest.clear();
        }
        qualifications.push(uncommitted);
        for qualification in qualifications {
            let mut f = fixture(m2::PhysicalOutcome::Satisfied);
            assert!(
                apply_outcome_to_m1(
                    &mut f.supervisor,
                    &f.m1_ticket,
                    &f.m2_ticket,
                    &f.live,
                    &f.evidence,
                    f.policy.clone(),
                    &qualification,
                )
                .is_err()
            );
            assert_eq!(
                f.supervisor.phase("pick"),
                Ok(m1::SubgoalPhase::AwaitingEvidence)
            );
        }
    }

    #[test]
    fn wrong_time_clock_domain_is_rejected_without_mutating_m1() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let bad_clock = m2::ClockBinding {
            clock_id: "other-clock".into(),
            boot_epoch_id: "other-boot".into(),
        };
        let bad = qualified_receipt_time(
            &bad_clock,
            &f.m2_ticket.platform.platform_id,
            mycelix_time::TimeScaleIdV2::Tai,
        );
        assert_eq!(
            apply_outcome_to_m1(
                &mut f.supervisor,
                &f.m1_ticket,
                &f.m2_ticket,
                &f.live,
                &f.evidence,
                f.policy.clone(),
                &bad,
            ),
            Err(AdapterError::TimeSourceClockMismatch)
        );
        assert_eq!(
            f.supervisor.phase("pick"),
            Ok(m1::SubgoalPhase::AwaitingEvidence)
        );
    }

    #[test]
    fn distinct_destination_time_scale_is_preserved_without_raw_tick_cast() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let alternate_scale = qualified_receipt_time(
            &f.evidence.clock,
            &f.m2_ticket.platform.platform_id,
            mycelix_time::TimeScaleIdV2::Tt,
        );
        let result = apply_outcome_to_m1(
            &mut f.supervisor,
            &f.m1_ticket,
            &f.m2_ticket,
            &f.live,
            &f.evidence,
            f.policy.clone(),
            &alternate_scale,
        )
        .expect("qualified Tt destination scale");
        assert_eq!(result, m1::ReceiptDisposition::Applied);
        assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::Complete));
    }

    #[test]
    fn normalized_time_uncertainty_is_rounded_up_and_preserved_in_m1() {
        let f = fixture(m2::PhysicalOutcome::Satisfied);
        let (_, qualification) = qualify_receipt_time(
            &f.receipt_time,
            &f.evidence.clock,
            &f.m2_ticket.platform.platform_id,
        )
        .expect("qualified time");
        assert_eq!(qualification.uncertainty_bound_nanoseconds, 6_000_001);
    }

    #[test]
    fn non_finite_normalized_time_uncertainty_is_rejected() {
        let f = fixture(m2::PhysicalOutcome::Satisfied);
        let mut invalid = f.receipt_time.clone();
        if let QualifiedReceiptTime::Qualified { normalized, .. } = &mut invalid {
            normalized.normalized_reading.uncertainty_bound_seconds = f64::INFINITY;
        }
        assert_eq!(
            qualify_receipt_time(
                &invalid,
                &f.evidence.clock,
                &f.m2_ticket.platform.platform_id,
            ),
            Err(AdapterError::MycelixTime(
                mycelix_time::TimeCorrelationError::InvalidUncertainty
            ))
        );
    }

    #[test]
    fn source_reading_outside_myclix_correlation_validity_is_rejected() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let mut invalid = f.receipt_time.clone();
        if let QualifiedReceiptTime::Qualified { normalized, .. } = &mut invalid {
            normalized.source_reading.value = 2_100.0;
        }
        assert!(matches!(
            apply_outcome_to_m1(
                &mut f.supervisor,
                &f.m1_ticket,
                &f.m2_ticket,
                &f.live,
                &f.evidence,
                f.policy.clone(),
                &invalid,
            ),
            Err(AdapterError::MycelixTime(
                mycelix_time::TimeCorrelationError::SourceOutsideValidity
            ))
        ));
        assert_eq!(
            f.supervisor.phase("pick"),
            Ok(m1::SubgoalPhase::AwaitingEvidence)
        );
    }

    #[test]
    fn mycelix_output_not_in_integer_milliseconds_is_rejected() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let mut invalid = f.receipt_time.clone();
        if let QualifiedReceiptTime::Qualified {
            correlation,
            normalized,
        } = &mut invalid
        {
            correlation.destination_unit = mycelix_time::TimeUnitV2::Seconds;
            normalized.normalized_reading.unit = mycelix_time::TimeUnitV2::Seconds;
            normalized.normalized_reading.value = 1_070.0;
        }
        assert_eq!(
            apply_outcome_to_m1(
                &mut f.supervisor,
                &f.m1_ticket,
                &f.m2_ticket,
                &f.live,
                &f.evidence,
                f.policy.clone(),
                &invalid,
            ),
            Err(AdapterError::TimeDestinationUnitMismatch)
        );
        assert_eq!(
            f.supervisor.phase("pick"),
            Ok(m1::SubgoalPhase::AwaitingEvidence)
        );
    }

    #[test]
    fn common_cause_observer_is_rejected_before_m1_receipt_mutation() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        f.evidence
            .observer_failure_domains
            .push("vehicle-power".into());
        f.policy
            .expected_observer_failure_domains
            .push("vehicle-power".into());
        f.m2_ticket
            .expected_observer_failure_domains
            .push("vehicle-power".into());
        assert_eq!(
            apply_outcome_to_m1(
                &mut f.supervisor,
                &f.m1_ticket,
                &f.m2_ticket,
                &f.live,
                &f.evidence,
                f.policy.clone(),
                &f.receipt_time,
            ),
            Err(AdapterError::Outcome(
                m2::OutcomeError::CommonCauseFailureDomainOverlap
            ))
        );
        assert_eq!(
            f.supervisor.phase("pick"),
            Ok(m1::SubgoalPhase::AwaitingEvidence)
        );
    }

    #[test]
    fn stale_prior_m1_attempt_is_rejected_after_valid_m2_check() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        f.supervisor
            .observe_goal(m1::GoalEvidence {
                envelope: m1::ReceiptEnvelope {
                    event_id: "no-effect-1".into(),
                    content_digest: m1::Digest([11; 32]),
                    source_id: "observer-a".into(),
                    source_sequence: 1,
                    observed_at_ms: 1_050,
                    time_qualification: m1::ReceiptTimeQualification {
                        source_clock_id: "observer-clock".into(),
                        source_boot_epoch_id: "observer-boot".into(),
                        comparison_basis_id: "tai".into(),
                        comparison_epoch_id: "tai-epoch-1970".into(),
                        source_time_commitment: m1::Digest([15; 32]),
                        correlation_commitment: m1::Digest([16; 32]),
                        normalized_time_commitment: m1::Digest([17; 32]),
                        uncertainty_bound_nanoseconds: 6_000_000,
                    },
                    observer_role: m1::ObserverRole::IndependentSensor,
                    confidence_basis_points: 9_900,
                },
                subgoal_id: f.m1_ticket.subgoal_id.clone(),
                action_id: f.m1_ticket.action_id.clone(),
                attempt: f.m1_ticket.attempt,
                predicate_id: "predicate.object-in-gripper".into(),
                verdict: m1::GoalVerdict::NotSatisfied,
            })
            .expect("first attempt no effect");
        f.supervisor
            .authorize_recovery(m1::RecoveryPermit {
                envelope: m1::ReceiptEnvelope {
                    event_id: "recovery-1".into(),
                    content_digest: m1::Digest([12; 32]),
                    source_id: "safety-monitor".into(),
                    source_sequence: 1,
                    observed_at_ms: 1_060,
                    time_qualification: m1::ReceiptTimeQualification {
                        source_clock_id: "safety-clock".into(),
                        source_boot_epoch_id: "safety-boot".into(),
                        comparison_basis_id: "tai".into(),
                        comparison_epoch_id: "tai-epoch-1970".into(),
                        source_time_commitment: m1::Digest([18; 32]),
                        correlation_commitment: m1::Digest([19; 32]),
                        normalized_time_commitment: m1::Digest([20; 32]),
                        uncertainty_bound_nanoseconds: 6_000_000,
                    },
                    observer_role: m1::ObserverRole::IndependentSensor,
                    confidence_basis_points: 9_900,
                },
                subgoal_id: f.m1_ticket.subgoal_id.clone(),
                action_id: f.m1_ticket.action_id.clone(),
                attempt: f.m1_ticket.attempt,
                recovery_plan_digest: m1::Digest([90; 32]),
                safe_to_mutate: true,
            })
            .expect("recover first attempt");
        f.supervisor
            .dispatch_action("pick", "action-pick-2")
            .expect("second action");
        let result = apply_outcome_to_m1(
            &mut f.supervisor,
            &f.m1_ticket,
            &f.m2_ticket,
            &f.live,
            &f.evidence,
            f.policy.clone(),
            &f.receipt_time,
        );
        assert_eq!(
            result,
            Err(AdapterError::Supervisor(
                m1::SupervisorError::ActionIdMismatch
            ))
        );
        assert_eq!(f.supervisor.phase("pick"), Ok(m1::SubgoalPhase::Executing));
    }

    #[test]
    fn mismatched_m1_and_m2_tickets_are_rejected_before_processing() {
        let mut f = fixture(m2::PhysicalOutcome::Satisfied);
        let mut wrong = f.m1_ticket.clone();
        wrong.action_id = "different-action".into();
        assert_eq!(
            apply_outcome_to_m1(
                &mut f.supervisor,
                &wrong,
                &f.m2_ticket,
                &f.live,
                &f.evidence,
                f.policy.clone(),
                &f.receipt_time,
            ),
            Err(AdapterError::M1TicketMismatch)
        );
        assert_eq!(
            f.supervisor.phase("pick"),
            Ok(m1::SubgoalPhase::AwaitingEvidence)
        );
    }
}
