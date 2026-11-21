use mobility_action_evidence::{
    ActionTicket, AuthorityBinding, ClockBinding, Digest, LiveContext, NavigationRequirement,
    NavigationService, NavigationTimeEvidence, ObserverRole, OperationBinding, OutcomeDisposition,
    OutcomePolicy, PhysicalOutcome, PhysicalOutcomeEvidence, PlatformBinding,
    validate_physical_outcome, validate_ticket_applicability,
};

fn digest(byte: u8) -> Digest {
    Digest([byte; 32])
}

fn ticket() -> ActionTicket {
    ActionTicket {
        ticket_commitment: digest(1),
        mission_id: "mission-1".into(),
        mission_generation: 7,
        subgoal_id: "subgoal-1".into(),
        action_id: "action-1".into(),
        attempt: 1,
        action_digest: digest(2),
        expected_effect_predicate_id: "predicate.reached-waypoint".into(),
        platform: PlatformBinding {
            platform_id: "vehicle-1".into(),
            configuration_digest: digest(3),
        },
        operation: OperationBinding {
            odd_id: "odd-coastal-day".into(),
            odd_revision: digest(4),
            mode_id: "autonomous-supervised".into(),
            mode_revision: digest(5),
        },
        authority: AuthorityBinding {
            authority_id: "operator-delegation-1".into(),
            epoch: 9,
            required_scope: "mission.navigate".into(),
        },
        clock: ClockBinding {
            clock_id: "monotonic-clock-1".into(),
            boot_epoch_id: "boot-epoch-4".into(),
        },
        issued_at_ticks: 1_000,
        expires_at_ticks: 2_000,
        navigation: NavigationRequirement {
            profile_digest: digest(6),
            required_service: NavigationService::Degraded,
            spatial_frame_id: "frame.local-enu-1".into(),
            source_id: "navigation-sensor-1".into(),
            minimum_source_sequence: 20,
            maximum_age_ticks: 100,
        },
        expected_observation_frame_id: "frame.local-enu-1".into(),
        expected_calibration_id: "calibration-camera-1".into(),
        expected_effect_digest: digest(7),
        observer_independence_policy_digest: digest(40),
    }
}

fn live() -> LiveContext {
    let t = ticket();
    LiveContext {
        mission_id: t.mission_id.clone(),
        mission_generation: t.mission_generation,
        active_subgoal_id: t.subgoal_id.clone(),
        active_action_id: t.action_id.clone(),
        active_attempt: t.attempt,
        active_effect_predicate_id: t.expected_effect_predicate_id.clone(),
        platform: t.platform.clone(),
        operation: t.operation.clone(),
        authority_id: t.authority.authority_id.clone(),
        authority_epoch: t.authority.epoch,
        authority_revoked: false,
        authority_scopes: vec!["mission.navigate".into()],
        clock: t.clock.clone(),
        now_ticks: 1_100,
        navigation_profile_digest: t.navigation.profile_digest,
        navigation_service: NavigationService::Qualified,
        navigation_frame_id: t.navigation.spatial_frame_id.clone(),
        navigation_source_id: t.navigation.source_id.clone(),
        navigation_source_sequence: 22,
        navigation_time: NavigationTimeEvidence::Qualified {
            source_clock: t.clock.clone(),
            comparison_clock: t.clock.clone(),
            normalized_observed_at_ticks: Some(1_090),
            normalization_commitment: None,
        },
    }
}

fn independent_admission_oracle(ticket: &ActionTicket, context: &LiveContext) -> bool {
    let nonempty = [
        ticket.mission_id.as_str(),
        ticket.subgoal_id.as_str(),
        ticket.action_id.as_str(),
        ticket.expected_effect_predicate_id.as_str(),
        ticket.platform.platform_id.as_str(),
        ticket.operation.odd_id.as_str(),
        ticket.operation.mode_id.as_str(),
        ticket.authority.authority_id.as_str(),
        ticket.authority.required_scope.as_str(),
        ticket.clock.clock_id.as_str(),
        ticket.clock.boot_epoch_id.as_str(),
        ticket.navigation.spatial_frame_id.as_str(),
        ticket.navigation.source_id.as_str(),
        ticket.expected_observation_frame_id.as_str(),
        ticket.expected_calibration_id.as_str(),
        context.mission_id.as_str(),
        context.active_subgoal_id.as_str(),
        context.active_action_id.as_str(),
        context.active_effect_predicate_id.as_str(),
        context.platform.platform_id.as_str(),
        context.operation.odd_id.as_str(),
        context.operation.mode_id.as_str(),
        context.authority_id.as_str(),
        context.clock.clock_id.as_str(),
        context.clock.boot_epoch_id.as_str(),
        context.navigation_frame_id.as_str(),
        context.navigation_source_id.as_str(),
    ];
    if nonempty.iter().any(|value| value.trim().is_empty()) {
        return false;
    }
    if ticket.issued_at_ticks >= ticket.expires_at_ticks || ticket.navigation.maximum_age_ticks == 0
    {
        return false;
    }
    let required_rank = match ticket.navigation.required_service {
        NavigationService::Qualified => 2,
        NavigationService::Degraded => 1,
        NavigationService::Unqualified | NavigationService::Unknown => return false,
    };
    if ticket.clock != context.clock
        || context.now_ticks < ticket.issued_at_ticks
        || context.now_ticks >= ticket.expires_at_ticks
        || ticket.mission_id != context.mission_id
        || ticket.mission_generation != context.mission_generation
        || ticket.subgoal_id != context.active_subgoal_id
        || ticket.action_id != context.active_action_id
        || ticket.attempt != context.active_attempt
        || ticket.expected_effect_predicate_id != context.active_effect_predicate_id
        || ticket.platform != context.platform
        || ticket.operation != context.operation
        || ticket.authority.authority_id != context.authority_id
        || ticket.authority.epoch != context.authority_epoch
        || context.authority_revoked
        || !context
            .authority_scopes
            .iter()
            .any(|scope| scope == &ticket.authority.required_scope)
        || ticket.navigation.profile_digest != context.navigation_profile_digest
        || ticket.navigation.spatial_frame_id != context.navigation_frame_id
        || ticket.navigation.source_id != context.navigation_source_id
        || context.navigation_source_sequence < ticket.navigation.minimum_source_sequence
    {
        return false;
    }
    let actual_rank = match context.navigation_service {
        NavigationService::Qualified => 2,
        NavigationService::Degraded => 1,
        NavigationService::Unqualified | NavigationService::Unknown => return false,
    };
    if actual_rank < required_rank {
        return false;
    }
    match &context.navigation_time {
        NavigationTimeEvidence::Unknown | NavigationTimeEvidence::Unqualified { .. } => false,
        NavigationTimeEvidence::Qualified {
            source_clock,
            comparison_clock,
            normalized_observed_at_ticks,
            normalization_commitment,
        } => {
            if source_clock.clock_id.trim().is_empty()
                || source_clock.boot_epoch_id.trim().is_empty()
                || comparison_clock.clock_id.trim().is_empty()
                || comparison_clock.boot_epoch_id.trim().is_empty()
                || comparison_clock != &context.clock
                || (source_clock != comparison_clock && normalization_commitment.is_none())
            {
                return false;
            }
            let Some(observed) = normalized_observed_at_ticks else {
                return false;
            };
            *observed <= context.now_ticks
                && context.now_ticks - *observed <= ticket.navigation.maximum_age_ticks
        }
    }
}

fn evidence() -> PhysicalOutcomeEvidence {
    let t = ticket();
    PhysicalOutcomeEvidence {
        ticket_commitment: t.ticket_commitment,
        content_digest: digest(8),
        event_id: "event-1".into(),
        source_id: "independent-camera-1".into(),
        executing_controller_id: "vehicle-controller-1".into(),
        observer_failure_domains: vec![
            "camera-sensor".into(),
            "camera-processor".into(),
            "observer-power".into(),
        ],
        observer_role: ObserverRole::IndependentSensor,
        source_sequence: 31,
        observed_at_ticks: 1_150,
        clock: t.clock.clone(),
        platform_configuration_digest: t.platform.configuration_digest,
        observation_frame_id: t.expected_observation_frame_id.clone(),
        calibration_id: t.expected_calibration_id.clone(),
        effect_predicate_digest: t.expected_effect_digest,
        confidence_basis_points: 9_900,
        outcome: PhysicalOutcome::Satisfied,
    }
}

fn policy() -> OutcomePolicy {
    OutcomePolicy {
        clock: ticket().clock,
        now_ticks: 1_160,
        maximum_age_ticks: 100,
        sequence_source_id: "independent-camera-1".into(),
        last_source_sequence: 30,
        minimum_confidence_basis_points: 9_000,
        execution_failure_domains: vec![
            "vehicle-controller".into(),
            "vehicle-bus".into(),
            "vehicle-power".into(),
        ],
        approved_observer_failure_domains: vec![
            "camera-sensor".into(),
            "camera-processor".into(),
            "observer-power".into(),
        ],
        observer_independence_policy_digest: digest(40),
    }
}

fn independent_outcome_oracle(
    ticket: &ActionTicket,
    observed: &PhysicalOutcomeEvidence,
    policy: &OutcomePolicy,
) -> Option<OutcomeDisposition> {
    let nonempty = [
        observed.event_id.as_str(),
        observed.source_id.as_str(),
        observed.executing_controller_id.as_str(),
        observed.clock.clock_id.as_str(),
        observed.clock.boot_epoch_id.as_str(),
        observed.observation_frame_id.as_str(),
        observed.calibration_id.as_str(),
    ];
    if nonempty.iter().any(|value| value.trim().is_empty())
        || observed.ticket_commitment != ticket.ticket_commitment
        || policy.observer_independence_policy_digest
            != ticket.observer_independence_policy_digest
        || policy.execution_failure_domains.is_empty()
        || policy.approved_observer_failure_domains.is_empty()
        || observed.observer_failure_domains.is_empty()
        || policy.execution_failure_domains.iter().any(|domain| domain.trim().is_empty())
        || policy.approved_observer_failure_domains.iter().any(|domain| domain.trim().is_empty())
        || observed.observer_failure_domains.iter().any(|domain| domain.trim().is_empty())
        || observed.observer_failure_domains.iter().any(|domain| {
            !policy.approved_observer_failure_domains.contains(domain)
        })
        || observed.observer_failure_domains.iter().any(|domain| {
            policy.execution_failure_domains.contains(domain)
        })
        || !matches!(
            observed.observer_role,
            ObserverRole::IndependentSensor | ObserverRole::IndependentEvaluator
        )
        || observed.source_id == observed.executing_controller_id
        || observed.source_id != policy.sequence_source_id
        || observed.source_sequence <= policy.last_source_sequence
        || policy.clock != ticket.clock
        || observed.clock != ticket.clock
        || observed.observed_at_ticks < ticket.issued_at_ticks
        || observed.observed_at_ticks > policy.now_ticks
        || policy.maximum_age_ticks == 0
        || policy.now_ticks - observed.observed_at_ticks > policy.maximum_age_ticks
        || observed.platform_configuration_digest != ticket.platform.configuration_digest
        || observed.observation_frame_id != ticket.expected_observation_frame_id
        || observed.calibration_id != ticket.expected_calibration_id
        || observed.effect_predicate_digest != ticket.expected_effect_digest
        || observed.confidence_basis_points > 10_000
        || policy.minimum_confidence_basis_points > 10_000
        || observed.confidence_basis_points < policy.minimum_confidence_basis_points
    {
        return None;
    }
    match observed.outcome {
        PhysicalOutcome::Satisfied => Some(OutcomeDisposition::Accepted),
        PhysicalOutcome::NoEffect => Some(OutcomeDisposition::RecoveryRequired),
        PhysicalOutcome::PartialEffect | PhysicalOutcome::Unknown => {
            Some(OutcomeDisposition::ReconciliationRequired)
        }
    }
}

#[test]
fn admission_validator_matches_independent_oracle_under_fault_mutations() {
    let base_ticket = ticket();
    let base_live = live();
    let mut cases: Vec<(&str, ActionTicket, LiveContext)> =
        vec![("positive-control", base_ticket.clone(), base_live.clone())];

    let mut c = base_live.clone();
    c.now_ticks = 2_000;
    cases.push(("expiry-boundary", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.active_subgoal_id = "different-subgoal".into();
    cases.push(("active-subgoal-mismatch", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.active_action_id = "different-action".into();
    cases.push(("active-action-mismatch", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.active_attempt += 1;
    cases.push(("active-attempt-mismatch", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.active_effect_predicate_id = "different-predicate".into();
    cases.push(("active-predicate-mismatch", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.platform.configuration_digest = digest(90);
    cases.push(("configuration-swap", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.mission_generation += 1;
    cases.push(("mission-generation", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.operation.odd_revision = digest(91);
    cases.push(("odd-revision", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.operation.mode_revision = digest(92);
    cases.push(("mode-revision", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.authority_epoch += 1;
    cases.push(("authority-epoch", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.authority_revoked = true;
    cases.push(("authority-revocation", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.authority_scopes.clear();
    cases.push(("authority-scope", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.clock.boot_epoch_id = "new-boot".into();
    cases.push(("clock-restart", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_profile_digest = digest(93);
    cases.push(("navigation-profile", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_service = NavigationService::Unqualified;
    cases.push(("navigation-integrity-downgrade", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_frame_id = "wrong-frame".into();
    cases.push(("navigation-frame", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_source_id = "other-source".into();
    cases.push(("navigation-source", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_source_sequence = 19;
    cases.push(("navigation-replay-watermark", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_time = NavigationTimeEvidence::Unknown;
    cases.push(("navigation-time-unknown", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_time = NavigationTimeEvidence::Unqualified { source_clock: None };
    cases.push(("navigation-time-unqualified", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_time = NavigationTimeEvidence::Qualified {
        source_clock: base_ticket.clock.clone(),
        comparison_clock: base_ticket.clock.clone(),
        normalized_observed_at_ticks: None,
        normalization_commitment: None,
    };
    cases.push(("navigation-time-missing", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_time = NavigationTimeEvidence::Qualified {
        source_clock: base_ticket.clock.clone(),
        comparison_clock: ClockBinding {
            clock_id: "different-clock".into(),
            boot_epoch_id: "different-boot".into(),
        },
        normalized_observed_at_ticks: Some(1_090),
        normalization_commitment: Some(digest(94)),
    };
    cases.push(("navigation-comparison-clock", base_ticket.clone(), c));

    let mut c = base_live.clone();
    let source_clock = ClockBinding {
        clock_id: "sensor-clock".into(),
        boot_epoch_id: "sensor-boot".into(),
    };
    c.navigation_time = NavigationTimeEvidence::Qualified {
        source_clock,
        comparison_clock: base_ticket.clock.clone(),
        normalized_observed_at_ticks: Some(1_090),
        normalization_commitment: None,
    };
    cases.push(("navigation-unmapped-clock", base_ticket.clone(), c));

    let mut c = base_live.clone();
    c.navigation_time = NavigationTimeEvidence::Qualified {
        source_clock: base_ticket.clock.clone(),
        comparison_clock: base_ticket.clock.clone(),
        normalized_observed_at_ticks: Some(900),
        normalization_commitment: None,
    };
    cases.push(("navigation-stale", base_ticket.clone(), c));

    let mut t = base_ticket.clone();
    t.expires_at_ticks = t.issued_at_ticks;
    cases.push(("invalid-ticket-window", t, base_live.clone()));

    let mut t = base_ticket.clone();
    t.navigation.maximum_age_ticks = 0;
    cases.push(("zero-navigation-age", t, base_live.clone()));

    let mut t = base_ticket.clone();
    t.navigation.required_service = NavigationService::Unknown;
    cases.push(("unknown-required-service", t, base_live.clone()));

    for (name, t, c) in cases {
        let expected = independent_admission_oracle(&t, &c);
        let actual = validate_ticket_applicability(&t, &c).is_ok();
        assert_eq!(actual, expected, "oracle disagreement in {name}");
    }
}

#[test]
fn physical_outcome_validator_matches_independent_oracle_under_fault_mutations() {
    let t = ticket();
    let e = evidence();
    let p = policy();
    let mut cases: Vec<(&str, PhysicalOutcomeEvidence, OutcomePolicy)> =
        vec![("positive-control", e.clone(), p.clone())];

    let mut x = e.clone();
    x.observer_failure_domains.push("vehicle-power".into());
    cases.push(("common-cause-domain-overlap", x, p.clone()));

    let mut x = e.clone();
    x.observer_failure_domains.push("unknown-domain".into());
    cases.push(("unapproved-observer-domain", x, p.clone()));

    let mut x = e.clone();
    x.observer_failure_domains.clear();
    cases.push(("missing-observer-domain-path", x, p.clone()));

    let mut wrong_policy = p.clone();
    wrong_policy.observer_independence_policy_digest = digest(41);
    cases.push(("independence-policy-version-mismatch", e.clone(), wrong_policy));

    let mut incomplete_policy = p.clone();
    incomplete_policy.execution_failure_domains.clear();
    cases.push(("missing-execution-domain-path", e.clone(), incomplete_policy));

    let mut x = e.clone();
    x.ticket_commitment = digest(99);
    cases.push(("wrong-ticket", x, p.clone()));

    let mut x = e.clone();
    x.observer_role = ObserverRole::AgentSelfReport;
    cases.push(("self-report", x, p.clone()));

    let mut x = e.clone();
    x.source_id = x.executing_controller_id.clone();
    cases.push(("executor-as-observer", x, p.clone()));

    let mut x = e.clone();
    x.source_sequence = 30;
    cases.push(("duplicate-sequence", x, p.clone()));

    let mut x = e.clone();
    x.clock.boot_epoch_id = "new-boot".into();
    cases.push(("observer-boot-mismatch", x, p.clone()));

    let mut x = e.clone();
    x.observed_at_ticks = 999;
    cases.push(("predates-action", x, p.clone()));

    let mut x = e.clone();
    x.observed_at_ticks = 1_170;
    cases.push(("future-observation", x, p.clone()));

    let mut stale_policy = p.clone();
    stale_policy.now_ticks = 1_300;
    cases.push(("stale-observation", e.clone(), stale_policy));

    let mut x = e.clone();
    x.platform_configuration_digest = digest(98);
    cases.push(("configuration-mismatch", x, p.clone()));

    let mut x = e.clone();
    x.observation_frame_id = "wrong-frame".into();
    cases.push(("frame-mismatch", x, p.clone()));

    let mut x = e.clone();
    x.calibration_id = "wrong-calibration".into();
    cases.push(("calibration-mismatch", x, p.clone()));

    let mut x = e.clone();
    x.effect_predicate_digest = digest(97);
    cases.push(("predicate-mismatch", x, p.clone()));

    let mut x = e.clone();
    x.confidence_basis_points = 8_999;
    cases.push(("low-confidence", x, p.clone()));

    let mut wrong_clock_policy = p.clone();
    wrong_clock_policy.clock.boot_epoch_id = "policy-reboot".into();
    cases.push(("policy-clock-mismatch", e.clone(), wrong_clock_policy));

    let mut no_effect = e.clone();
    no_effect.outcome = PhysicalOutcome::NoEffect;
    cases.push(("no-effect", no_effect, p.clone()));

    let mut partial = e.clone();
    partial.outcome = PhysicalOutcome::PartialEffect;
    cases.push(("partial-effect", partial, p.clone()));

    let mut unknown = e.clone();
    unknown.outcome = PhysicalOutcome::Unknown;
    cases.push(("unknown-effect", unknown, p.clone()));

    for (name, observed, policy) in cases {
        let expected = independent_outcome_oracle(&t, &observed, &policy);
        let actual = validate_physical_outcome(&t, &observed, policy).ok();
        assert_eq!(actual, expected, "oracle disagreement in {name}");
    }
}
