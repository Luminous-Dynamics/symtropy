//! Synthetic-first service-degradation and cross-flight contingency evidence oracle.
//!
//! This crate evaluates declared service-capability evidence at separate effective and
//! knowledge cutoffs, conservatively estimates provider/failure-domain diversity, and
//! reports per-flight capability impact plus missing operator/neighbor coordination
//! evidence. It never chooses a flight maneuver, authorizes an operation, or declares
//! that continuing a flight is safe.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// Safety-relevant service capability consumed by a flight assessment.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    /// Position, navigation and clock service.
    PntAndTime,
    /// Cooperative/non-cooperative traffic surveillance and tracking.
    Surveillance,
    /// Dynamic airspace restrictions and reconfiguration feed.
    DynamicRestrictions,
    /// Operational authorization issuer/status service.
    Authorization,
    /// Provider-to-provider and operator coordination communications.
    CoordinationCommunications,
    /// Weather and environmental information.
    Weather,
    /// Identity and trust-policy service.
    IdentityAndTrust,
}

/// Declared provider/service state. Owner qualification is assessed separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclaredServiceState {
    /// Provider claims the exact capability is available and qualified.
    AvailableQualified,
    /// Capability is available but degraded or below expected redundancy.
    AvailableDegraded,
    /// Evidence is outside its freshness/validity interval.
    Stale,
    /// Provider/service is partitioned or unreachable from some consumers.
    Partitioned,
    /// Providers or sources report incompatible state/epoch/content.
    Conflicting,
    /// Evidence or provider is not qualified for reliance.
    Unqualified,
    /// Provider explicitly reports loss of the capability.
    Unavailable,
    /// Provider is in a recovery process.
    Recovering,
    /// Provider has returned but its evidence/snapshot is not reconciled.
    ReconciliationRequired,
    /// This service does not claim this capability for the declared scope.
    NotApplicable,
}

/// Owner qualification of a service observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerEvidenceState {
    /// Evidence is qualified under the provider/owner's declared source contract.
    Qualified,
    /// Evidence is explicitly not qualified.
    Unqualified,
    /// Evidence qualification is unknown or incomplete.
    Unknown,
}

/// Trust/revocation state supplied by the trust-custody owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustEvidenceState {
    /// Identity, policy and key status are owner-qualified for this cutoff.
    Qualified,
    /// Identity/authority/revocation state is not acceptable.
    Unqualified,
    /// Trust state is unknown or not yet reconciled.
    Unknown,
}

/// One versioned service observation at a bitemporal cutoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceObservation {
    /// Unique observation/snapshot identity.
    pub snapshot_id: String,
    /// Stable service identity across snapshots.
    pub service_id: String,
    /// Provider identity.
    pub provider_id: String,
    /// Capability this service instance claims to supply.
    pub capability: Capability,
    /// Provider-declared capability state.
    pub declared_state: DeclaredServiceState,
    /// Owner qualification of this observation.
    pub owner_evidence: OwnerEvidenceState,
    /// Trust-custody qualification.
    pub trust_evidence: TrustEvidenceState,
    /// Effective/observed time, on the declared time scale.
    pub observed_at: i64,
    /// Time this observation became recorded/known to the assessor.
    pub recorded_at: i64,
    /// Start of the validity interval.
    pub valid_from: i64,
    /// End of the validity interval.
    pub valid_until: i64,
    /// Monotonic source sequence for this service stream.
    pub source_sequence: u64,
    /// Provider/service epoch.
    pub epoch: u64,
    /// Exact service/configuration identity to which this observation applies.
    pub service_scope_sha256: String,
    /// Exact recovery/configuration snapshot digest.
    pub snapshot_sha256: String,
    /// Whether source-feed coverage is complete for this observation.
    pub source_coverage_complete: bool,
    /// Whether anti-rollback/freshness checks were completed.
    pub anti_rollback_verified: bool,
    /// Whether recovery reconciliation completed.
    pub reconciliation_complete: bool,
    /// Declared common-cause/failure-domain identities.
    pub failure_domain_ids: Vec<String>,
    /// Owner receipt qualifying the failure-domain declaration.
    pub failure_domain_receipt_sha256: Option<String>,
    /// Service-scope digest bound by the failure-domain receipt.
    pub failure_domain_scope_sha256: Option<String>,
}

/// Minimum qualified provider and failure-domain diversity for a capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityRequirement {
    /// Required capability.
    pub capability: Capability,
    /// Minimum number of distinct qualified provider identities.
    pub minimum_qualified_providers: usize,
    /// Minimum pairwise failure-domain-independent providers.
    pub minimum_independent_providers: usize,
}

/// Whether a flight is already active or merely proposed for planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlightMode {
    /// New planning/dispatch request; no execution permission is implied.
    Proposed,
    /// Existing active flight; this evaluator only reports evidence/coordination state.
    Active,
}

/// Operator-supplied approved contingency reference for one capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContingencyBinding {
    /// Capability for which this contingency applies.
    pub capability: Capability,
    /// Exact flight scope to which the plan applies.
    pub flight_scope_sha256: String,
    /// Digest of the operator/authority-supplied contingency plan/volume reference.
    pub plan_receipt_sha256: String,
    /// Whether the responsible owner says this plan is approved for this exact scope.
    pub approved_for_scope: bool,
}

/// State of an owner-supplied neighbor notification/coordination receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationState {
    /// Recipient has acknowledged the exact coordination message.
    Acknowledged,
    /// Message was delivered but not acknowledged.
    Delivered,
    /// Notification failed.
    Failed,
    /// Receipt state is unknown.
    Unknown,
}

/// Evidence that an affected neighboring flight was notified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeighborNotification {
    /// Recipient/affected neighboring flight ID.
    pub neighbor_flight_id: String,
    /// Exact flight assessment scope.
    pub flight_scope_sha256: String,
    /// Digest of the exact notification/coordination receipt.
    pub receipt_sha256: String,
    /// Time the notification/acknowledgement became recorded.
    pub recorded_at: i64,
    /// Owner-supplied notification state.
    pub state: NotificationState,
}

/// Exact flight scope and its required service/contingency dependencies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlightScope {
    /// Stable pseudonymous flight identifier.
    pub flight_id: String,
    /// Exact operation/flight assessment scope digest.
    pub flight_scope_sha256: String,
    /// Proposed or active status.
    pub mode: FlightMode,
    /// Required service capabilities and redundancy floors.
    pub requirements: Vec<CapabilityRequirement>,
    /// Operator-supplied contingency plans; no plan is synthesized here.
    pub contingency_bindings: Vec<ContingencyBinding>,
    /// Neighbor flights that the responsible owner says must be coordinated.
    pub required_neighbor_flight_ids: Vec<String>,
    /// Owner-supplied neighbor notification receipts.
    pub neighbor_notifications: Vec<NeighborNotification>,
}

/// Effective and knowledge cutoffs for reproducible service-state assessment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitemporalCutoff {
    /// Effective time being assessed.
    pub effective_at: i64,
    /// Knowledge cutoff; later-recorded observations are excluded.
    pub known_at: i64,
}

/// Derived state of an individual provider observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceEvidenceState {
    /// The capability is currently qualified under the supplied scope and cutoffs.
    AvailableQualified,
    /// The provider claims availability but its qualification/recovery evidence is incomplete.
    AvailableDegraded,
    /// Validity interval does not cover the requested effective cutoff.
    Stale,
    /// Service/provider is partitioned.
    Partitioned,
    /// Provider/source evidence conflicts.
    Conflicting,
    /// Owner/trust/source evidence is unqualified.
    Unqualified,
    /// Provider reports the capability unavailable.
    Unavailable,
    /// Provider is still recovering.
    Recovering,
    /// Recovery requires reconciliation before reliance.
    ReconciliationRequired,
    /// Observation does not apply to this capability/scope.
    NotApplicable,
    /// Evidence is unknown or unavailable at the requested cutoff.
    Indeterminate,
}

/// Aggregate capability status for one flight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityStatus {
    /// Provider count and independent failure-domain floors are met.
    AvailableQualified,
    /// Some qualified service remains, but the declared redundancy floor is not met.
    AvailableDegraded,
    /// All observed evidence is stale.
    Stale,
    /// Service path is partitioned.
    Partitioned,
    /// Provider/source states conflict.
    Conflicting,
    /// Service/trust evidence is unqualified.
    Unqualified,
    /// No provider reports the capability available.
    Unavailable,
    /// At least one provider is recovering.
    Recovering,
    /// Recovery/anti-rollback/source reconciliation is incomplete.
    ReconciliationRequired,
    /// Required evidence is absent or not assessable.
    Indeterminate,
}

/// Cross-flight assessment result. None of these statuses is operational permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlightEvidenceDisposition {
    /// All required capabilities meet the frozen evidence profile; planning only.
    PlanningEvidenceQualifiedOnly,
    /// A known mandatory service capability fails the evidence profile.
    NewPlanningBlocked,
    /// At least one required capability is not assessable.
    Indeterminate,
    /// Existing-flight contingency references and required neighbor receipts are present.
    ContingencyCoordinationEvidencePresent,
    /// Existing-flight contingency reference is absent or inapplicable.
    ContingencyEvidenceMissing,
    /// Existing-flight neighbor notification/coordination receipts are incomplete.
    NeighborCoordinationIncomplete,
    /// Existing-flight service evidence is incomplete; coordination status cannot be concluded.
    ActiveFlightEvidenceIndeterminate,
}

/// Per-provider state in one capability assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAssessment {
    /// Service identity.
    pub service_id: String,
    /// Provider identity.
    pub provider_id: String,
    /// Derived evidence state.
    pub state: ServiceEvidenceState,
    /// Declared failure-domain identities.
    pub failure_domain_ids: Vec<String>,
    /// Whether failure-domain identities are receipt-bound to this exact service scope.
    pub failure_domain_identity_qualified: bool,
}

/// One capability's status for a flight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityAssessment {
    /// Capability under review.
    pub capability: Capability,
    /// Aggregate capability status.
    pub status: CapabilityStatus,
    /// Qualified provider IDs.
    pub qualified_provider_ids: Vec<String>,
    /// Conservative independent-provider IDs, selected only from disjoint qualified failure-domain sets.
    pub independent_provider_ids: Vec<String>,
    /// All relevant service observations.
    pub providers: Vec<ProviderAssessment>,
}

/// Shared service failure affecting more than one assessed flight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedServiceImpact {
    /// Service identity.
    pub service_id: String,
    /// Flights whose required capability assessments depend on this service.
    pub affected_flight_ids: Vec<String>,
}

/// Campaign result across multiple proposed/active flights.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResilienceCampaignAssessment {
    /// Per-flight assessments, sorted by flight ID.
    pub flights: Vec<FlightAssessment>,
    /// Shared service impacts, sorted by service ID.
    pub shared_service_impacts: Vec<SharedServiceImpact>,
}

/// Full flight assessment, including required operator/neighbor evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlightAssessment {
    /// Flight identity.
    pub flight_id: String,
    /// Overall evidence disposition.
    pub disposition: FlightEvidenceDisposition,
    /// Per-capability outcomes.
    pub capabilities: Vec<CapabilityAssessment>,
    /// Capabilities requiring a contingency reference.
    pub contingency_required_capabilities: Vec<Capability>,
    /// Capabilities for which contingency evidence is missing/invalid.
    pub missing_contingency_capabilities: Vec<Capability>,
    /// Neighbor flights awaiting acknowledged coordination evidence.
    pub missing_neighbor_flight_ids: Vec<String>,
    /// Affected service IDs visible at the requested cutoff.
    pub affected_service_ids: Vec<String>,
}

/// Policy commitment for the resilience assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResiliencePolicy {
    /// Exact policy manifest digest.
    pub policy_sha256: String,
}

/// Typed input/structural failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResilienceError {
    /// Policy identity is malformed.
    InvalidPolicy,
    /// Effective or knowledge cutoff is invalid.
    InvalidCutoff,
    /// Flight scope, digest, requirements or IDs are malformed.
    InvalidFlightScope,
    /// A requirement is duplicated or has impossible redundancy floors.
    InvalidCapabilityRequirement,
    /// An eligible service snapshot is malformed or duplicated.
    InvalidServiceObservation,
    /// Eligible service snapshots fork or reuse a source sequence.
    ServiceSequenceFork,
    /// Duplicate flight IDs were supplied to a campaign.
    DuplicateFlightId,
    /// Contingency binding is duplicated or malformed.
    InvalidContingencyBinding,
    /// Neighbor notification is duplicated or malformed.
    InvalidNeighborNotification,
}

/// Check digest syntax only. Cryptographic verification belongs to trust custody.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

fn validate_flight_scope(scope: &FlightScope) -> Result<(), ResilienceError> {
    if scope.flight_id.trim().is_empty()
        || !is_sha256(&scope.flight_scope_sha256)
        || scope.requirements.is_empty()
        || !unique_nonempty(&scope.required_neighbor_flight_ids)
    {
        return Err(ResilienceError::InvalidFlightScope);
    }
    let mut capabilities = BTreeSet::new();
    for requirement in &scope.requirements {
        if requirement.minimum_qualified_providers == 0
            || requirement.minimum_independent_providers == 0
            || requirement.minimum_independent_providers
                > requirement.minimum_qualified_providers
            || !capabilities.insert(requirement.capability)
        {
            return Err(ResilienceError::InvalidCapabilityRequirement);
        }
    }
    let mut contingency_capabilities = BTreeSet::new();
    for binding in &scope.contingency_bindings {
        if !capabilities.contains(&binding.capability)
            || binding.flight_scope_sha256 != scope.flight_scope_sha256
            || !is_sha256(&binding.plan_receipt_sha256)
            || !contingency_capabilities.insert(binding.capability)
        {
            return Err(ResilienceError::InvalidContingencyBinding);
        }
    }
    Ok(())
}

fn validate_neighbor_notifications(
    scope: &FlightScope,
    cutoff: BitemporalCutoff,
) -> Result<(), ResilienceError> {
    let mut notified = BTreeSet::new();
    for notification in scope
        .neighbor_notifications
        .iter()
        .filter(|item| item.recorded_at <= cutoff.known_at)
    {
        if notification.neighbor_flight_id.trim().is_empty()
            || !scope
                .required_neighbor_flight_ids
                .contains(&notification.neighbor_flight_id)
            || notification.flight_scope_sha256 != scope.flight_scope_sha256
            || notification.recorded_at < 0
            || !is_sha256(&notification.receipt_sha256)
            || !notified.insert(notification.neighbor_flight_id.as_str())
        {
            return Err(ResilienceError::InvalidNeighborNotification);
        }
    }
    Ok(())
}

fn validate_observation(observation: &ServiceObservation) -> Result<(), ResilienceError> {
    if observation.snapshot_id.trim().is_empty()
        || observation.service_id.trim().is_empty()
        || observation.provider_id.trim().is_empty()
        || observation.observed_at < 0
        || observation.recorded_at < 0
        || observation.valid_from > observation.valid_until
        || observation.source_sequence == 0
        || observation.epoch == 0
        || !is_sha256(&observation.service_scope_sha256)
        || !is_sha256(&observation.snapshot_sha256)
        || !unique_nonempty(&observation.failure_domain_ids)
        || observation.failure_domain_receipt_sha256
            .as_ref()
            .is_some_and(|digest| !is_sha256(digest))
        || observation.failure_domain_scope_sha256
            .as_ref()
            .is_some_and(|digest| !is_sha256(digest))
    {
        return Err(ResilienceError::InvalidServiceObservation);
    }
    Ok(())
}

fn derive_service_state(
    observation: &ServiceObservation,
    cutoff: BitemporalCutoff,
) -> ServiceEvidenceState {
    if observation.observed_at > cutoff.effective_at
        || observation.recorded_at > cutoff.known_at
    {
        return ServiceEvidenceState::Indeterminate;
    }
    if observation.valid_from > cutoff.effective_at
        || observation.valid_until < cutoff.effective_at
    {
        return ServiceEvidenceState::Stale;
    }
    if observation.owner_evidence == OwnerEvidenceState::Unknown
        || observation.trust_evidence == TrustEvidenceState::Unknown
        || !observation.source_coverage_complete
    {
        return ServiceEvidenceState::Indeterminate;
    }
    if observation.owner_evidence == OwnerEvidenceState::Unqualified
        || observation.trust_evidence == TrustEvidenceState::Unqualified
    {
        return ServiceEvidenceState::Unqualified;
    }
    if !observation.anti_rollback_verified || !observation.reconciliation_complete {
        return ServiceEvidenceState::ReconciliationRequired;
    }
    match observation.declared_state {
        DeclaredServiceState::AvailableQualified => ServiceEvidenceState::AvailableQualified,
        DeclaredServiceState::AvailableDegraded => ServiceEvidenceState::AvailableDegraded,
        DeclaredServiceState::Stale => ServiceEvidenceState::Stale,
        DeclaredServiceState::Partitioned => ServiceEvidenceState::Partitioned,
        DeclaredServiceState::Conflicting => ServiceEvidenceState::Conflicting,
        DeclaredServiceState::Unqualified => ServiceEvidenceState::Unqualified,
        DeclaredServiceState::Unavailable => ServiceEvidenceState::Unavailable,
        DeclaredServiceState::Recovering => ServiceEvidenceState::Recovering,
        DeclaredServiceState::ReconciliationRequired => {
            ServiceEvidenceState::ReconciliationRequired
        }
        DeclaredServiceState::NotApplicable => ServiceEvidenceState::NotApplicable,
    }
}

fn independent_provider_subset(providers: &[ProviderAssessment]) -> Vec<String> {
    let mut candidates: Vec<&ProviderAssessment> = providers
        .iter()
        .filter(|provider| {
            provider.state == ServiceEvidenceState::AvailableQualified
                && provider.failure_domain_identity_qualified
            && !provider.failure_domain_ids.is_empty()
        })
        .collect();
    candidates.sort_by(|left, right| left.provider_id.cmp(&right.provider_id));

    // Conservative deterministic greedy packing: only providers whose complete
    // declared failure-domain sets are disjoint are counted as independent.
    // This may under-count a maximum set, but it never manufactures independence.
    let mut used_domains = BTreeSet::new();
    let mut used_provider_ids = BTreeSet::new();
    let mut selected = Vec::new();
    for provider in candidates {
        if provider.failure_domain_ids.iter().all(|domain| !used_domains.contains(domain))
            && used_provider_ids.insert(provider.provider_id.clone())
        {
            for domain in &provider.failure_domain_ids {
                used_domains.insert(domain.clone());
            }
            selected.push(provider.provider_id.clone());
        }
    }
    selected
}

fn aggregate_capability(
    requirement: &CapabilityRequirement,
    providers: Vec<ProviderAssessment>,
) -> CapabilityAssessment {
    let mut qualified_provider_ids: Vec<String> = providers
        .iter()
        .filter(|provider| provider.state == ServiceEvidenceState::AvailableQualified)
        .map(|provider| provider.provider_id.clone())
        .collect();
    qualified_provider_ids.sort();
    qualified_provider_ids.dedup();

    let independent_provider_ids = independent_provider_subset(&providers);
    let enough_qualified =
        qualified_provider_ids.len() >= requirement.minimum_qualified_providers;
    let enough_independent =
        independent_provider_ids.len() >= requirement.minimum_independent_providers;

    let status = if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Conflicting)
    {
        CapabilityStatus::Conflicting
    } else if enough_qualified && enough_independent {
        CapabilityStatus::AvailableQualified
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Indeterminate)
    {
        CapabilityStatus::Indeterminate
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::ReconciliationRequired)
    {
        CapabilityStatus::ReconciliationRequired
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Recovering)
    {
        CapabilityStatus::Recovering
    } else if !qualified_provider_ids.is_empty() {
        CapabilityStatus::AvailableDegraded
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Partitioned)
    {
        CapabilityStatus::Partitioned
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Stale)
    {
        CapabilityStatus::Stale
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Unqualified)
    {
        CapabilityStatus::Unqualified
    } else if providers
        .iter()
        .any(|provider| provider.state == ServiceEvidenceState::Unavailable)
    {
        CapabilityStatus::Unavailable
    } else {
        CapabilityStatus::Indeterminate
    };

    CapabilityAssessment {
        capability: requirement.capability,
        status,
        qualified_provider_ids,
        independent_provider_ids,
        providers,
    }
}

fn contingency_is_valid(scope: &FlightScope, capability: Capability) -> bool {
    scope.contingency_bindings.iter().any(|binding| {
        binding.capability == capability
            && binding.approved_for_scope
            && binding.flight_scope_sha256 == scope.flight_scope_sha256
            && is_sha256(&binding.plan_receipt_sha256)
    })
}

fn neighbor_acknowledged(
    scope: &FlightScope,
    neighbor_id: &str,
    cutoff: BitemporalCutoff,
) -> bool {
    scope.neighbor_notifications.iter().any(|notification| {
        notification.neighbor_flight_id == neighbor_id
            && notification.flight_scope_sha256 == scope.flight_scope_sha256
            && notification.state == NotificationState::Acknowledged
            && notification.recorded_at <= cutoff.known_at
            && is_sha256(&notification.receipt_sha256)
    })
}

/// Evaluate all required service capabilities and contingency evidence for one flight.
///
/// The function reports an evidence/coordination disposition only. It does not decide
/// whether an active flight should continue, return, land, divert or change trajectory.
pub fn evaluate_flight(
    scope: &FlightScope,
    observations: &[ServiceObservation],
    cutoff: BitemporalCutoff,
    policy: &ResiliencePolicy,
) -> Result<FlightAssessment, ResilienceError> {
    validate_flight_scope(scope)?;
    if !is_sha256(&policy.policy_sha256) {
        return Err(ResilienceError::InvalidPolicy);
    }
    if cutoff.effective_at < 0 || cutoff.known_at < 0 {
        return Err(ResilienceError::InvalidCutoff);
    }
    validate_neighbor_notifications(scope, cutoff)?;

    let relevant_capabilities: BTreeSet<Capability> =
        scope.requirements.iter().map(|item| item.capability).collect();

    let eligible: Vec<&ServiceObservation> = observations
        .iter()
        .filter(|item| {
            item.observed_at <= cutoff.effective_at
                && item.recorded_at <= cutoff.known_at
                && relevant_capabilities.contains(&item.capability)
        })
        .collect();

    let mut seen_snapshots = BTreeSet::new();
    let mut seen_sequences = BTreeSet::new();
    let mut service_identity: BTreeMap<&str, (&str, Capability)> = BTreeMap::new();
    for observation in &eligible {
        validate_observation(observation)?;
        if !seen_snapshots.insert(observation.snapshot_id.as_str()) {
            return Err(ResilienceError::InvalidServiceObservation);
        }
        if !seen_sequences.insert((observation.service_id.as_str(), observation.source_sequence)) {
            return Err(ResilienceError::ServiceSequenceFork);
        }
        if let Some((provider_id, capability)) = service_identity.get(observation.service_id.as_str())
            && (*provider_id != observation.provider_id || *capability != observation.capability)
        {
            return Err(ResilienceError::InvalidServiceObservation);
        }
        service_identity.insert(
            observation.service_id.as_str(),
            (observation.provider_id.as_str(), observation.capability),
        );
    }

    let mut capability_assessments = Vec::new();
    for requirement in &scope.requirements {
        let mut latest_by_service: BTreeMap<&str, &ServiceObservation> = BTreeMap::new();
        for observation in eligible
            .iter()
            .copied()
            .filter(|item| item.capability == requirement.capability)
        {
            let replace = latest_by_service
                .get(observation.service_id.as_str())
                .is_none_or(|previous| {
                    (observation.epoch, observation.source_sequence, observation.recorded_at)
                        > (previous.epoch, previous.source_sequence, previous.recorded_at)
                });
            if replace {
                latest_by_service.insert(observation.service_id.as_str(), observation);
            }
        }

        let providers = latest_by_service
            .into_values()
            .map(|observation| ProviderAssessment {
                service_id: observation.service_id.clone(),
                provider_id: observation.provider_id.clone(),
                state: derive_service_state(observation, cutoff),
                failure_domain_ids: observation.failure_domain_ids.clone(),
                failure_domain_identity_qualified: observation
                    .failure_domain_receipt_sha256
                    .as_ref()
                    .is_some_and(|digest| is_sha256(digest))
                    && observation.failure_domain_scope_sha256.as_deref()
                        == Some(observation.service_scope_sha256.as_str()),
            })
            .collect();

        capability_assessments.push(aggregate_capability(requirement, providers));
    }

    capability_assessments.sort_by_key(|item| item.capability);

    let affected_service_ids: BTreeSet<String> = capability_assessments
        .iter()
        .filter(|capability| capability.status != CapabilityStatus::AvailableQualified)
        .flat_map(|capability| capability.providers.iter())
        .map(|provider| provider.service_id.clone())
        .collect();

    let degraded_capabilities: Vec<Capability> = capability_assessments
        .iter()
        .filter(|item| item.status != CapabilityStatus::AvailableQualified)
        .map(|item| item.capability)
        .collect();

    let mut missing_contingency_capabilities = Vec::new();
    let mut missing_neighbor_flight_ids = Vec::new();

    let has_indeterminate = capability_assessments
        .iter()
        .any(|item| item.status == CapabilityStatus::Indeterminate);
    let has_degraded = !degraded_capabilities.is_empty();

    if scope.mode == FlightMode::Active && has_degraded {
        for capability in &degraded_capabilities {
            if !contingency_is_valid(scope, *capability) {
                missing_contingency_capabilities.push(*capability);
            }
        }
        for neighbor in &scope.required_neighbor_flight_ids {
            if !neighbor_acknowledged(scope, neighbor, cutoff) {
                missing_neighbor_flight_ids.push(neighbor.clone());
            }
        }
    }

    let disposition = match scope.mode {
        FlightMode::Proposed => {
            if has_indeterminate {
                FlightEvidenceDisposition::Indeterminate
            } else if has_degraded {
                FlightEvidenceDisposition::NewPlanningBlocked
            } else {
                FlightEvidenceDisposition::PlanningEvidenceQualifiedOnly
            }
        }
        FlightMode::Active => {
            if has_indeterminate {
                FlightEvidenceDisposition::ActiveFlightEvidenceIndeterminate
            } else if !has_degraded {
                FlightEvidenceDisposition::ActiveFlightEvidenceAvailableOnly
            } else if !missing_contingency_capabilities.is_empty() {
                FlightEvidenceDisposition::ContingencyEvidenceMissing
            } else if !missing_neighbor_flight_ids.is_empty() {
                FlightEvidenceDisposition::NeighborCoordinationIncomplete
            } else {
                FlightEvidenceDisposition::ContingencyCoordinationEvidencePresent
            }
        }
    };

    Ok(FlightAssessment {
        flight_id: scope.flight_id.clone(),
        disposition,
        capabilities: capability_assessments,
        contingency_required_capabilities: degraded_capabilities,
        missing_contingency_capabilities,
        missing_neighbor_flight_ids,
        affected_service_ids: affected_service_ids.into_iter().collect(),
    })
}


/// Evaluate a multi-flight campaign and identify services shared by affected flights.
///
/// Each flight is evaluated under the same bitemporal cutoff and frozen policy.
/// Shared impact means more than one flight has a non-qualified assessment for a
/// capability relying on the same service identity. It is not a hazard probability
/// or an operational instruction.
pub fn evaluate_campaign(
    scopes: &[FlightScope],
    observations: &[ServiceObservation],
    cutoff: BitemporalCutoff,
    policy: &ResiliencePolicy,
) -> Result<ResilienceCampaignAssessment, ResilienceError> {
    let mut flight_ids = BTreeSet::new();
    let mut flights = Vec::new();
    let mut service_to_flights: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for scope in scopes {
        if !flight_ids.insert(scope.flight_id.as_str()) {
            return Err(ResilienceError::DuplicateFlightId);
        }
        let assessment = evaluate_flight(scope, observations, cutoff, policy)?;
        for service_id in &assessment.affected_service_ids {
            service_to_flights
                .entry(service_id.clone())
                .or_default()
                .insert(scope.flight_id.clone());
        }
        flights.push(assessment);
    }

    if scopes.is_empty() {
        return Err(ResilienceError::InvalidFlightScope);
    }

    flights.sort_by(|left, right| left.flight_id.cmp(&right.flight_id));
    let mut shared_service_impacts: Vec<SharedServiceImpact> = service_to_flights
        .into_iter()
        .filter_map(|(service_id, impacted)| {
            (impacted.len() > 1).then(|| SharedServiceImpact {
                service_id,
                affected_flight_ids: impacted.into_iter().collect(),
            })
        })
        .collect();
    shared_service_impacts.sort_by(|left, right| left.service_id.cmp(&right.service_id));

    Ok(ResilienceCampaignAssessment {
        flights,
        shared_service_impacts,
    })
}
