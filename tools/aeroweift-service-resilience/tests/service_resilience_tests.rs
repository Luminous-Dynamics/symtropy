use aeroweift_service_resilience::{
    BitemporalCutoff, Capability, CapabilityRequirement, CapabilityStatus, ContingencyBinding,
    DeclaredServiceState, FlightEvidenceDisposition, FlightMode, FlightScope,
    NeighborNotification, NotificationState, OwnerEvidenceState, ResilienceError,
    ResiliencePolicy, ServiceObservation, TrustEvidenceState, evaluate_campaign,
    evaluate_flight,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn policy() -> ResiliencePolicy {
    ResiliencePolicy {
        policy_sha256: digest('a'),
    }
}

fn cutoff(effective_at: i64, known_at: i64) -> BitemporalCutoff {
    BitemporalCutoff {
        effective_at,
        known_at,
    }
}

fn requirement(capability: Capability) -> CapabilityRequirement {
    CapabilityRequirement {
        capability,
        minimum_qualified_providers: 1,
        minimum_independent_providers: 1,
    }
}

fn scope(id: &str, mode: FlightMode) -> FlightScope {
    FlightScope {
        flight_id: id.into(),
        flight_scope_sha256: digest('b'),
        mode,
        requirements: vec![
            requirement(Capability::Surveillance),
            requirement(Capability::DynamicRestrictions),
        ],
        contingency_bindings: Vec::new(),
        required_neighbor_flight_ids: Vec::new(),
        neighbor_notifications: Vec::new(),
    }
}

fn observation(
    snapshot_id: &str,
    service_id: &str,
    provider_id: &str,
    capability: Capability,
    state: DeclaredServiceState,
    sequence: u64,
    domain: &str,
    scope_char: char,
) -> ServiceObservation {
    ServiceObservation {
        snapshot_id: snapshot_id.into(),
        service_id: service_id.into(),
        provider_id: provider_id.into(),
        capability,
        declared_state: state,
        owner_evidence: OwnerEvidenceState::Qualified,
        trust_evidence: TrustEvidenceState::Qualified,
        observed_at: 10,
        recorded_at: 10,
        valid_from: 0,
        valid_until: 100,
        source_sequence: sequence,
        epoch: 1,
        service_scope_sha256: digest(scope_char),
        snapshot_sha256: digest('c'),
        source_coverage_complete: true,
        anti_rollback_verified: true,
        reconciliation_complete: true,
        failure_domain_ids: vec![domain.into()],
        failure_domain_receipt_sha256: Some(digest('d')),
        failure_domain_scope_sha256: Some(digest(scope_char)),
    }
}

fn healthy_services() -> Vec<ServiceObservation> {
    vec![
        observation(
            "surveillance-snapshot-1",
            "surveillance-service-1",
            "provider-surveillance-1",
            Capability::Surveillance,
            DeclaredServiceState::AvailableQualified,
            1,
            "surveillance-region-a",
            'e',
        ),
        observation(
            "restriction-snapshot-1",
            "restriction-service-1",
            "provider-restriction-1",
            Capability::DynamicRestrictions,
            DeclaredServiceState::AvailableQualified,
            1,
            "restriction-region-a",
            'f',
        ),
    ]
}

fn contingency(scope: &FlightScope, capability: Capability) -> ContingencyBinding {
    ContingencyBinding {
        capability,
        flight_scope_sha256: scope.flight_scope_sha256.clone(),
        plan_receipt_sha256: digest('8'),
        approved_for_scope: true,
    }
}

fn notification(scope: &FlightScope, neighbor: &str, state: NotificationState) -> NeighborNotification {
    NeighborNotification {
        neighbor_flight_id: neighbor.into(),
        flight_scope_sha256: scope.flight_scope_sha256.clone(),
        receipt_sha256: digest('9'),
        recorded_at: 20,
        state,
    }
}

#[test]
fn positive_control_supports_planning_evidence_only() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let result = evaluate_flight(&flight, &healthy_services(), cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::PlanningEvidenceQualifiedOnly
    );
    assert_eq!(result.capabilities.len(), 2);
    assert!(result.affected_service_ids.is_empty());
}

#[test]
fn active_flight_reports_contingency_and_neighbor_coordination_evidence() {
    let mut flight = scope("flight-a", FlightMode::Active);
    flight.required_neighbor_flight_ids = vec!["flight-b".into()];
    flight.contingency_bindings = vec![contingency(&flight, Capability::Surveillance)];
    flight.neighbor_notifications = vec![notification(
        &flight,
        "flight-b",
        NotificationState::Acknowledged,
    )];
    let mut services = healthy_services();
    services[0].declared_state = DeclaredServiceState::Unavailable;

    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::ContingencyCoordinationEvidencePresent
    );
    assert_eq!(
        result.contingency_required_capabilities,
        vec![Capability::Surveillance]
    );
    assert!(result.missing_contingency_capabilities.is_empty());
    assert!(result.missing_neighbor_flight_ids.is_empty());
}

#[test]
fn active_flight_with_service_loss_and_no_contingency_is_flagged() {
    let flight = scope("flight-a", FlightMode::Active);
    let mut services = healthy_services();
    services[0].declared_state = DeclaredServiceState::Unavailable;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::ContingencyEvidenceMissing
    );
    assert_eq!(
        result.missing_contingency_capabilities,
        vec![Capability::Surveillance]
    );
}

#[test]
fn active_flight_requires_acknowledged_neighbor_coordination() {
    let mut flight = scope("flight-a", FlightMode::Active);
    flight.required_neighbor_flight_ids = vec!["flight-b".into()];
    flight.contingency_bindings = vec![contingency(&flight, Capability::Surveillance)];
    flight.neighbor_notifications = vec![notification(
        &flight,
        "flight-b",
        NotificationState::Delivered,
    )];
    let mut services = healthy_services();
    services[0].declared_state = DeclaredServiceState::Unavailable;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::NeighborCoordinationIncomplete
    );
    assert_eq!(result.missing_neighbor_flight_ids, vec!["flight-b".to_string()]);
}

#[test]
fn proposed_flight_is_blocked_when_mandatory_service_is_unavailable() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].declared_state = DeclaredServiceState::Unavailable;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::NewPlanningBlocked
    );
}

#[test]
fn unknown_provider_evidence_is_indeterminate_not_qualified() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].owner_evidence = OwnerEvidenceState::Unknown;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::Indeterminate
    );
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::Indeterminate
    );
}

#[test]
fn common_mode_providers_do_not_count_as_independent_redundancy() {
    let mut flight = scope("flight-a", FlightMode::Proposed);
    flight.requirements[0].minimum_qualified_providers = 2;
    flight.requirements[0].minimum_independent_providers = 2;
    let mut services = healthy_services();
    let mut backup = services[0].clone();
    backup.snapshot_id = "surveillance-snapshot-2".into();
    backup.service_id = "surveillance-service-2".into();
    backup.provider_id = "provider-surveillance-2".into();
    backup.source_sequence = 1;
    backup.failure_domain_ids = vec!["surveillance-region-a".into()];
    backup.service_scope_sha256 = digest('7');
    backup.failure_domain_scope_sha256 = Some(digest('7'));
    services.push(backup);

    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::AvailableDegraded
    );
    assert_eq!(
        result.capabilities[1].independent_provider_ids.len(),
        1
    );
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::NewPlanningBlocked
    );
}

#[test]
fn independent_qualified_failover_preserves_capability_evidence() {
    let mut flight = scope("flight-a", FlightMode::Proposed);
    flight.requirements[0].minimum_qualified_providers = 2;
    flight.requirements[0].minimum_independent_providers = 2;
    let mut services = healthy_services();
    let mut backup = services[0].clone();
    backup.snapshot_id = "surveillance-snapshot-2".into();
    backup.service_id = "surveillance-service-2".into();
    backup.provider_id = "provider-surveillance-2".into();
    backup.failure_domain_ids = vec!["surveillance-region-b".into()];
    backup.service_scope_sha256 = digest('7');
    backup.failure_domain_scope_sha256 = Some(digest('7'));
    backup.snapshot_sha256 = digest('8');
    services.push(backup);

    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::AvailableQualified
    );
    assert_eq!(
        result.capabilities[1].independent_provider_ids.len(),
        2
    );
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::PlanningEvidenceQualifiedOnly
    );
}

#[test]
fn failure_domain_receipt_bound_to_another_service_scope_is_not_independence() {
    let mut flight = scope("flight-a", FlightMode::Proposed);
    flight.requirements[0].minimum_qualified_providers = 2;
    flight.requirements[0].minimum_independent_providers = 2;
    let mut services = healthy_services();
    let mut backup = services[0].clone();
    backup.snapshot_id = "surveillance-snapshot-2".into();
    backup.service_id = "surveillance-service-2".into();
    backup.provider_id = "provider-surveillance-2".into();
    backup.failure_domain_ids = vec!["surveillance-region-b".into()];
    backup.service_scope_sha256 = digest('7');
    backup.failure_domain_scope_sha256 = Some(digest('e'));
    backup.snapshot_sha256 = digest('8');
    services.push(backup);

    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::AvailableDegraded
    );
}

#[test]
fn recovered_endpoint_without_reconciliation_is_not_qualified() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].reconciliation_complete = false;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::ReconciliationRequired
    );
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::NewPlanningBlocked
    );
}

#[test]
fn anti_rollback_failure_requires_reconciliation() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].anti_rollback_verified = false;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::ReconciliationRequired
    );
}

#[test]
fn incomplete_source_coverage_is_indeterminate() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].source_coverage_complete = false;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::Indeterminate
    );
}

#[test]
fn conflicting_provider_state_is_never_hidden_by_a_healthy_peer() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    let mut conflict = services[0].clone();
    conflict.snapshot_id = "surveillance-conflict".into();
    conflict.service_id = "surveillance-service-conflict".into();
    conflict.provider_id = "provider-surveillance-conflict".into();
    conflict.declared_state = DeclaredServiceState::Conflicting;
    conflict.failure_domain_ids = vec!["surveillance-region-b".into()];
    conflict.service_scope_sha256 = digest('7');
    conflict.failure_domain_scope_sha256 = Some(digest('7'));
    conflict.snapshot_sha256 = digest('8');
    services.push(conflict);

    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::Conflicting
    );
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::NewPlanningBlocked
    );
}

#[test]
fn stale_service_snapshot_is_not_available_qualified() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].valid_until = 49;
    let result = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.capabilities[1].status,
        CapabilityStatus::Stale
    );
}

#[test]
fn future_recorded_duplicate_snapshot_does_not_poison_earlier_knowledge_view() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    let mut late = services[0].clone();
    late.recorded_at = 70;
    services.push(late.clone());

    let earlier = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        earlier.disposition,
        FlightEvidenceDisposition::PlanningEvidenceQualifiedOnly
    );
    let later = evaluate_flight(&flight, &services, cutoff(50, 80), &policy());
    assert_eq!(later, Err(ResilienceError::InvalidServiceObservation));
}

#[test]
fn notification_receipt_recorded_after_cutoff_does_not_poison_historical_view() {
    let mut flight = scope("flight-a", FlightMode::Active);
    flight.required_neighbor_flight_ids = vec!["flight-b".into()];
    flight.contingency_bindings = vec![contingency(&flight, Capability::Surveillance)];
    let mut late = notification(&flight, "flight-b", NotificationState::Acknowledged);
    late.recorded_at = 70;
    flight.neighbor_notifications = vec![late];

    let mut services = healthy_services();
    services[0].declared_state = DeclaredServiceState::Unavailable;
    let earlier = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        earlier.disposition,
        FlightEvidenceDisposition::NeighborCoordinationIncomplete
    );
    assert_eq!(
        earlier.missing_neighbor_flight_ids,
        vec!["flight-b".to_string()]
    );
    let later = evaluate_flight(&flight, &services, cutoff(50, 80), &policy()).unwrap();
    assert_eq!(
        later.disposition,
        FlightEvidenceDisposition::ContingencyCoordinationEvidencePresent
    );
}

#[test]
fn healthy_active_flight_does_not_require_contingency_notifications() {
    let mut flight = scope("flight-a", FlightMode::Active);
    flight.required_neighbor_flight_ids = vec!["flight-b".into()];
    let result = evaluate_flight(&flight, &healthy_services(), cutoff(50, 50), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        FlightEvidenceDisposition::ActiveFlightEvidenceAvailableOnly
    );
    assert!(result.missing_neighbor_flight_ids.is_empty());
}

#[test]
fn campaign_identifies_shared_service_impact_across_flights() {
    let flight_a = scope("flight-a", FlightMode::Proposed);
    let flight_b = scope("flight-b", FlightMode::Proposed);
    let mut services = healthy_services();
    services[0].declared_state = DeclaredServiceState::Unavailable;

    let result = evaluate_campaign(
        &[flight_a, flight_b],
        &services,
        cutoff(50, 50),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.flights.len(), 2);
    assert_eq!(result.shared_service_impacts.len(), 1);
    assert_eq!(
        result.shared_service_impacts[0].service_id,
        "surveillance-service-1"
    );
    assert_eq!(
        result.shared_service_impacts[0].affected_flight_ids,
        vec!["flight-a".to_string(), "flight-b".to_string()]
    );
}

#[test]
fn campaign_rejects_duplicate_flight_ids() {
    let flight_a = scope("flight-a", FlightMode::Proposed);
    let duplicate = scope("flight-a", FlightMode::Active);
    assert_eq!(
        evaluate_campaign(
            &[flight_a, duplicate],
            &healthy_services(),
            cutoff(50, 50),
            &policy()
        ),
        Err(ResilienceError::DuplicateFlightId)
    );
}

#[test]
fn same_service_identity_cannot_change_provider_or_capability_across_snapshots() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let mut services = healthy_services();
    let mut fork = services[0].clone();
    fork.snapshot_id = "service-identity-fork".into();
    fork.source_sequence = 2;
    fork.provider_id = "different-provider".into();
    fork.recorded_at = 20;
    services.push(fork);
    assert_eq!(
        evaluate_flight(&flight, &services, cutoff(50, 50), &policy()),
        Err(ResilienceError::InvalidServiceObservation)
    );
}

#[test]
fn event_array_permutation_preserves_all_flight_assessments() {
    let flight = scope("flight-a", FlightMode::Proposed);
    let services = healthy_services();
    let expected = evaluate_flight(&flight, &services, cutoff(50, 50), &policy()).unwrap();
    let mut permuted = services;
    permuted.reverse();
    assert_eq!(
        evaluate_flight(&flight, &permuted, cutoff(50, 50), &policy()).unwrap(),
        expected
    );
}

#[test]
fn invalid_effective_or_knowledge_cutoff_is_rejected() {
    let flight = scope("flight-a", FlightMode::Proposed);
    assert_eq!(
        evaluate_flight(&flight, &healthy_services(), cutoff(-1, 50), &policy()),
        Err(ResilienceError::InvalidCutoff)
    );
}
