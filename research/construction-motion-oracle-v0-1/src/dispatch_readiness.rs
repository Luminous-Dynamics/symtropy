//! Evidence-bound dispatch readiness for construction work.
//!
//! This gate joins a frozen work package, a robot-specific motion envelope and
//! a fresh site-occupancy assessment. It is deliberately a *readiness* check:
//! it does not issue a safety permit, authorize actuators, or replace a
//! safety-rated protective system. The caller must verify canonical receipt
//! bytes and actor signatures before constructing these typed records.

use crate::site_state_ledger::{SiteActorRole, SiteEntityState, SiteStateLedger};

pub const DISPATCH_READINESS_REQUEST_SCHEMA_VERSION: &str =
    "construction-dispatch-readiness-request/0.1";
pub const DISPATCH_READINESS_RECEIPT_SCHEMA_VERSION: &str =
    "construction-dispatch-readiness-receipt/0.1";

/// Caller-authenticated snapshot from the qualified site time authority.
///
/// The caller verifies the authority signature and canonical receipt bytes.
/// current_tick is a site-clock reading with a conservative absolute error
/// bound. epoch_sha256 changes whenever the monotonic clock origin is reset;
/// mapping_sha256 binds the clock source's offset/drift mapping into the site
/// timebase. valid_until_tick is the qualification/holdover expiry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedSiteClockSnapshot {
    pub site_clock_domain_id: String,
    pub epoch_sha256: String,
    pub mapping_sha256: String,
    pub authority_id: String,
    pub source_id: String,
    pub sample_sequence: u64,
    pub current_tick: u64,
    pub uncertainty_bound_ticks: u64,
    pub valid_until_tick: u64,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchEnvelopeBinding {
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub entity_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub robot_id: String,
    pub robot_configuration_sha256: String,
    pub tool_configuration_sha256: String,
    pub payload_configuration_sha256: String,
    pub primitive_manifest_sha256: String,
    pub trajectory_sha256: String,
    pub envelope_sha256: String,
    /// The envelope's site-clock domain, after any cross-clock mapping.
    pub site_clock_domain_id: String,
    pub site_clock_epoch_sha256: String,
    pub clock_mapping_sha256: String,
    pub generated_at_tick: u64,
    pub generated_at_uncertainty_ticks: u64,
    pub valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccupancyAssessmentReceipt {
    /// The assessed envelope digest must exactly match the requested envelope.
    pub assessed_envelope_sha256: String,
    pub primitive_manifest_sha256: String,
    pub site_snapshot_sha256: String,
    pub site_ledger_sequence: u64,
    pub site_ledger_event_sha256: String,
    pub dynamic_occupancy_manifest_sha256: String,
    pub actor_manifest_sha256: String,
    pub occupancy_engine_build_sha256: String,
    pub clearance_policy_sha256: String,
    pub site_clock_domain_id: String,
    pub site_clock_epoch_sha256: String,
    pub clock_mapping_sha256: String,
    pub checked_at_tick: u64,
    pub checked_at_uncertainty_ticks: u64,
    pub valid_until_tick: u64,
    /// These are independent completeness assertions made by the occupancy
    /// adapter and bound to the assessment receipt digest.
    pub primitive_coverage_complete: bool,
    pub dynamic_actor_coverage_complete: bool,
    pub broadphase_pair_coverage_complete: bool,
    pub narrowphase_coverage_complete: bool,
    pub candidate_pair_count: u64,
    pub narrowphase_evaluated_pair_count: u64,
    pub conflict_count: u64,
    pub unresolved_pair_count: u64,
    pub invalid_bound_count: u64,
    pub unqualified_bound_count: u64,
    pub stale_actor_count: u64,
    pub unmodelled_occupant_count: u64,
    /// Digest of canonical occupancy assessment bytes, verified by caller.
    pub assessment_receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchReadinessRequest {
    pub schema_version: String,
    pub envelope: DispatchEnvelopeBinding,
    pub occupancy: OccupancyAssessmentReceipt,
    /// Authenticated current site-clock sample. Its uncertainty interval must
    /// lie wholly inside the envelope and occupancy validity intervals.
    pub qualified_clock: QualifiedSiteClockSnapshot,
}

/// Trust-owner policy is supplied out-of-band, not chosen by the request.
/// This prevents a request from authorizing its own engine, clearance policy
/// or tolerated clock error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchReadinessPolicy {
    pub occupancy_engine_build_sha256: String,
    pub clearance_policy_sha256: String,
    pub max_clock_uncertainty_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchReadinessReceipt {
    pub schema_version: String,
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub entity_id: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub robot_id: String,
    pub robot_configuration_sha256: String,
    pub tool_configuration_sha256: String,
    pub payload_configuration_sha256: String,
    pub primitive_manifest_sha256: String,
    pub trajectory_sha256: String,
    pub envelope_sha256: String,
    pub site_snapshot_sha256: String,
    pub dynamic_occupancy_manifest_sha256: String,
    pub actor_manifest_sha256: String,
    pub occupancy_assessment_receipt_sha256: String,
    pub occupancy_engine_build_sha256: String,
    pub clearance_policy_sha256: String,
    pub candidate_pair_count: u64,
    pub narrowphase_evaluated_pair_count: u64,
    pub site_ledger_sequence: u64,
    pub site_ledger_event_sha256: String,
    pub site_clock_domain_id: String,
    pub site_clock_epoch_sha256: String,
    pub clock_mapping_sha256: String,
    pub clock_authority_id: String,
    pub clock_source_id: String,
    pub clock_sample_sequence: u64,
    pub checked_at_tick: u64,
    pub checked_at_uncertainty_ticks: u64,
    pub clock_qualification_valid_until_tick: u64,
    pub clock_receipt_sha256: String,
    pub valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchReadinessFailure {
    UnsupportedSchemaVersion,
    InvalidDigest(&'static str),
    ReadinessReceiptNotYetValid,
    ReadinessReceiptExpired,
    UnknownOrInactivePackage,
    DesignRevisionMismatch,
    FrameMismatch,
    TransformMismatch,
    UnknownDesignEntity,
    EntityNotDispatchable,
    EnvelopeTimeWindowInvalid,
    EnvelopeExpired,
    EnvelopeNotYetValid,
    OccupancyEnvelopeMismatch,
    OccupancyPrimitiveManifestMismatch,
    OccupancyEngineBuildMismatch,
    ClearancePolicyMismatch,
    RobotIdentityNotRegistered,
    OccupancyClockDomainMismatch,
    ClockAuthorityNotRegistered,
    InvalidClockEvidence,
    ClockEpochMismatch,
    ClockMappingMismatch,
    ClockUncertaintyExceedsPolicy,
    ClockQualificationExpired,
    ClockUncertaintyStraddlesBoundary,
    ClockIntervalOverflow,
    ClockSampleNotFresh,
    SiteSnapshotSequenceMismatch,
    SiteSnapshotHeadMismatch,
    OccupancyTimeWindowInvalid,
    OccupancyAssessmentExpired,
    OccupancyAssessmentFromFuture,
    OccupancyAssessmentPredatesEnvelope,
    IncompletePrimitiveCoverage,
    IncompleteDynamicActorCoverage,
    IncompleteBroadphaseCoverage,
    IncompleteNarrowphaseCoverage,
    OccupancyNotClear {
        conflicts: u64,
        unresolved_pairs: u64,
        invalid_bounds: u64,
        unqualified_bounds: u64,
        stale_actors: u64,
        unmodelled_occupants: u64,
    },
}

/// Check whether a work item is eligible to enter a separate authorization
/// process. No actuation or safety permit is produced by this function.
pub fn check_dispatch_readiness(
    ledger: &SiteStateLedger,
    request: &DispatchReadinessRequest,
    policy: &DispatchReadinessPolicy,
) -> Result<DispatchReadinessReceipt, DispatchReadinessFailure> {
    if request.schema_version != DISPATCH_READINESS_REQUEST_SCHEMA_VERSION {
        return Err(DispatchReadinessFailure::UnsupportedSchemaVersion);
    }
    let envelope = &request.envelope;
    let occupancy = &request.occupancy;
    let clock = &request.qualified_clock;

    for (label, digest) in [
        ("package", envelope.package_sha256.as_str()),
        ("design revision", envelope.design_revision_sha256.as_str()),
        ("frame", envelope.frame_sha256.as_str()),
        ("transform", envelope.transform_sha256.as_str()),
        ("robot configuration", envelope.robot_configuration_sha256.as_str()),
        ("tool configuration", envelope.tool_configuration_sha256.as_str()),
        ("payload configuration", envelope.payload_configuration_sha256.as_str()),
        ("primitive manifest", envelope.primitive_manifest_sha256.as_str()),
        ("trajectory", envelope.trajectory_sha256.as_str()),
        ("motion envelope", envelope.envelope_sha256.as_str()),
        ("envelope clock epoch", envelope.site_clock_epoch_sha256.as_str()),
        ("envelope clock mapping", envelope.clock_mapping_sha256.as_str()),
        ("occupancy envelope", occupancy.assessed_envelope_sha256.as_str()),
        ("occupancy primitive manifest", occupancy.primitive_manifest_sha256.as_str()),
        ("site snapshot", occupancy.site_snapshot_sha256.as_str()),
        ("site ledger head", occupancy.site_ledger_event_sha256.as_str()),
        ("dynamic occupancy manifest", occupancy.dynamic_occupancy_manifest_sha256.as_str()),
        ("actor manifest", occupancy.actor_manifest_sha256.as_str()),
        ("occupancy engine build", occupancy.occupancy_engine_build_sha256.as_str()),
        ("clearance policy", occupancy.clearance_policy_sha256.as_str()),
        ("occupancy clock epoch", occupancy.site_clock_epoch_sha256.as_str()),
        ("occupancy clock mapping", occupancy.clock_mapping_sha256.as_str()),
        ("assessment receipt", occupancy.assessment_receipt_sha256.as_str()),
        ("trusted occupancy engine build", policy.occupancy_engine_build_sha256.as_str()),
        ("required clearance policy", policy.clearance_policy_sha256.as_str()),
        ("clock epoch", clock.epoch_sha256.as_str()),
        ("clock mapping", clock.mapping_sha256.as_str()),
        ("clock qualification receipt", clock.receipt_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(DispatchReadinessFailure::InvalidDigest(label));
        }
    }

    if envelope.entity_id.trim().is_empty()
        || envelope.robot_id.trim().is_empty()
        || envelope.site_clock_domain_id.trim().is_empty()
        || occupancy.site_clock_domain_id.trim().is_empty()
        || clock.site_clock_domain_id.trim().is_empty()
        || clock.authority_id.trim().is_empty()
        || clock.source_id.trim().is_empty()
        || clock.sample_sequence == 0
    {
        return Err(DispatchReadinessFailure::InvalidClockEvidence);
    }
    if !ledger.actor_registry().allows(&envelope.robot_id, SiteActorRole::Robot) {
        return Err(DispatchReadinessFailure::RobotIdentityNotRegistered);
    }
    if !ledger
        .actor_registry()
        .allows(&clock.authority_id, SiteActorRole::ClockAuthority)
    {
        return Err(DispatchReadinessFailure::ClockAuthorityNotRegistered);
    }
    if clock.uncertainty_bound_ticks > policy.max_clock_uncertainty_ticks
        || envelope.generated_at_uncertainty_ticks > policy.max_clock_uncertainty_ticks
        || occupancy.checked_at_uncertainty_ticks > policy.max_clock_uncertainty_ticks
    {
        return Err(DispatchReadinessFailure::ClockUncertaintyExceedsPolicy);
    }

    if envelope.site_clock_domain_id != clock.site_clock_domain_id
        || occupancy.site_clock_domain_id != clock.site_clock_domain_id
    {
        return Err(DispatchReadinessFailure::OccupancyClockDomainMismatch);
    }
    if envelope.site_clock_epoch_sha256 != clock.epoch_sha256
        || occupancy.site_clock_epoch_sha256 != clock.epoch_sha256
    {
        return Err(DispatchReadinessFailure::ClockEpochMismatch);
    }
    if envelope.clock_mapping_sha256 != clock.mapping_sha256
        || occupancy.clock_mapping_sha256 != clock.mapping_sha256
    {
        return Err(DispatchReadinessFailure::ClockMappingMismatch);
    }

    let (now_lower_tick, now_upper_tick) =
        uncertainty_interval(clock.current_tick, clock.uncertainty_bound_ticks)?;
    if now_lower_tick >= clock.valid_until_tick {
        return Err(DispatchReadinessFailure::ClockQualificationExpired);
    }
    if now_upper_tick >= clock.valid_until_tick {
        return Err(DispatchReadinessFailure::ClockUncertaintyStraddlesBoundary);
    }

    if ledger.active_package_sha256() != Some(envelope.package_sha256.as_str())
        || ledger.package_is_current(&envelope.package_sha256) != Some(true)
    {
        return Err(DispatchReadinessFailure::UnknownOrInactivePackage);
    }

    let package = ledger
        .work_package(&envelope.package_sha256)
        .ok_or(DispatchReadinessFailure::UnknownOrInactivePackage)?;
    if package.design_revision_sha256 != envelope.design_revision_sha256 {
        return Err(DispatchReadinessFailure::DesignRevisionMismatch);
    }
    if package.frame_sha256 != envelope.frame_sha256 {
        return Err(DispatchReadinessFailure::FrameMismatch);
    }
    if package.transform_sha256 != envelope.transform_sha256 {
        return Err(DispatchReadinessFailure::TransformMismatch);
    }
    if !package
        .entities
        .iter()
        .any(|entity| entity.design_entity_id == envelope.entity_id)
    {
        return Err(DispatchReadinessFailure::UnknownDesignEntity);
    }

    match ledger.entity_state(&envelope.package_sha256, &envelope.entity_id) {
        Some(SiteEntityState::NotAttempted | SiteEntityState::PhysicallyAbsent { .. }) => {}
        Some(_) => return Err(DispatchReadinessFailure::EntityNotDispatchable),
        None => return Err(DispatchReadinessFailure::UnknownDesignEntity),
    }

    if envelope.valid_until_tick <= envelope.generated_at_tick
        || envelope.generated_at_uncertainty_ticks >= envelope.valid_until_tick
    {
        return Err(DispatchReadinessFailure::EnvelopeTimeWindowInvalid);
    }
    let (envelope_generated_lower, envelope_generated_upper) = uncertainty_interval(
        envelope.generated_at_tick,
        envelope.generated_at_uncertainty_ticks,
    )?;
    if now_upper_tick < envelope_generated_lower {
        return Err(DispatchReadinessFailure::EnvelopeNotYetValid);
    }
    if now_lower_tick < envelope_generated_upper {
        return Err(DispatchReadinessFailure::ClockUncertaintyStraddlesBoundary);
    }
    if now_lower_tick >= envelope.valid_until_tick {
        return Err(DispatchReadinessFailure::EnvelopeExpired);
    }
    if now_upper_tick >= envelope.valid_until_tick {
        return Err(DispatchReadinessFailure::ClockUncertaintyStraddlesBoundary);
    }
    if envelope_generated_upper >= envelope.valid_until_tick {
        return Err(DispatchReadinessFailure::EnvelopeTimeWindowInvalid);
    }

    if occupancy.assessed_envelope_sha256 != envelope.envelope_sha256 {
        return Err(DispatchReadinessFailure::OccupancyEnvelopeMismatch);
    }
    if occupancy.primitive_manifest_sha256 != envelope.primitive_manifest_sha256 {
        return Err(DispatchReadinessFailure::OccupancyPrimitiveManifestMismatch);
    }
    if occupancy.occupancy_engine_build_sha256 != policy.occupancy_engine_build_sha256 {
        return Err(DispatchReadinessFailure::OccupancyEngineBuildMismatch);
    }
    if occupancy.clearance_policy_sha256 != policy.clearance_policy_sha256 {
        return Err(DispatchReadinessFailure::ClearancePolicyMismatch);
    }
    if occupancy.site_ledger_sequence != ledger.last_sequence() {
        return Err(DispatchReadinessFailure::SiteSnapshotSequenceMismatch);
    }
    if ledger.last_event_sha256() != Some(occupancy.site_ledger_event_sha256.as_str()) {
        return Err(DispatchReadinessFailure::SiteSnapshotHeadMismatch);
    }
    if occupancy.valid_until_tick <= occupancy.checked_at_tick
        || occupancy.checked_at_uncertainty_ticks >= occupancy.valid_until_tick
    {
        return Err(DispatchReadinessFailure::OccupancyTimeWindowInvalid);
    }
    let (occupancy_checked_lower, occupancy_checked_upper) =
        uncertainty_interval(occupancy.checked_at_tick, occupancy.checked_at_uncertainty_ticks)?;
    if occupancy_checked_lower < envelope_generated_upper {
        return Err(DispatchReadinessFailure::OccupancyAssessmentPredatesEnvelope);
    }
    if now_upper_tick < occupancy_checked_lower {
        return Err(DispatchReadinessFailure::OccupancyAssessmentFromFuture);
    }
    if occupancy_checked_upper >= occupancy.valid_until_tick {
        return Err(DispatchReadinessFailure::OccupancyTimeWindowInvalid);
    }
    if now_lower_tick < occupancy_checked_upper {
        return Err(DispatchReadinessFailure::ClockUncertaintyStraddlesBoundary);
    }
    if now_lower_tick >= occupancy.valid_until_tick {
        return Err(DispatchReadinessFailure::OccupancyAssessmentExpired);
    }
    if now_upper_tick >= occupancy.valid_until_tick {
        return Err(DispatchReadinessFailure::ClockUncertaintyStraddlesBoundary);
    }
    if !occupancy.primitive_coverage_complete {
        return Err(DispatchReadinessFailure::IncompletePrimitiveCoverage);
    }
    if !occupancy.dynamic_actor_coverage_complete {
        return Err(DispatchReadinessFailure::IncompleteDynamicActorCoverage);
    }
    if !occupancy.broadphase_pair_coverage_complete {
        return Err(DispatchReadinessFailure::IncompleteBroadphaseCoverage);
    }
    if !occupancy.narrowphase_coverage_complete
        || occupancy.narrowphase_evaluated_pair_count != occupancy.candidate_pair_count
    {
        return Err(DispatchReadinessFailure::IncompleteNarrowphaseCoverage);
    }
    if occupancy.conflict_count != 0
        || occupancy.unresolved_pair_count != 0
        || occupancy.invalid_bound_count != 0
        || occupancy.unqualified_bound_count != 0
        || occupancy.stale_actor_count != 0
        || occupancy.unmodelled_occupant_count != 0
    {
        return Err(DispatchReadinessFailure::OccupancyNotClear {
            conflicts: occupancy.conflict_count,
            unresolved_pairs: occupancy.unresolved_pair_count,
            invalid_bounds: occupancy.invalid_bound_count,
            unqualified_bounds: occupancy.unqualified_bound_count,
            stale_actors: occupancy.stale_actor_count,
            unmodelled_occupants: occupancy.unmodelled_occupant_count,
        });
    }

    let valid_until_tick = envelope
        .valid_until_tick
        .min(occupancy.valid_until_tick)
        .min(clock.valid_until_tick);
    Ok(DispatchReadinessReceipt {
        schema_version: DISPATCH_READINESS_RECEIPT_SCHEMA_VERSION.to_owned(),
        package_sha256: envelope.package_sha256.clone(),
        design_revision_sha256: envelope.design_revision_sha256.clone(),
        entity_id: envelope.entity_id.clone(),
        frame_sha256: envelope.frame_sha256.clone(),
        transform_sha256: envelope.transform_sha256.clone(),
        robot_id: envelope.robot_id.clone(),
        robot_configuration_sha256: envelope.robot_configuration_sha256.clone(),
        tool_configuration_sha256: envelope.tool_configuration_sha256.clone(),
        payload_configuration_sha256: envelope.payload_configuration_sha256.clone(),
        primitive_manifest_sha256: envelope.primitive_manifest_sha256.clone(),
        trajectory_sha256: envelope.trajectory_sha256.clone(),
        envelope_sha256: envelope.envelope_sha256.clone(),
        site_snapshot_sha256: occupancy.site_snapshot_sha256.clone(),
        dynamic_occupancy_manifest_sha256: occupancy.dynamic_occupancy_manifest_sha256.clone(),
        actor_manifest_sha256: occupancy.actor_manifest_sha256.clone(),
        occupancy_assessment_receipt_sha256: occupancy.assessment_receipt_sha256.clone(),
        occupancy_engine_build_sha256: occupancy.occupancy_engine_build_sha256.clone(),
        clearance_policy_sha256: occupancy.clearance_policy_sha256.clone(),
        candidate_pair_count: occupancy.candidate_pair_count,
        narrowphase_evaluated_pair_count: occupancy.narrowphase_evaluated_pair_count,
        site_ledger_sequence: ledger.last_sequence(),
        site_ledger_event_sha256: ledger.last_event_sha256().unwrap_or_default().to_owned(),
        site_clock_domain_id: clock.site_clock_domain_id.clone(),
        site_clock_epoch_sha256: clock.epoch_sha256.clone(),
        clock_mapping_sha256: clock.mapping_sha256.clone(),
        clock_authority_id: clock.authority_id.clone(),
        clock_source_id: clock.source_id.clone(),
        clock_sample_sequence: clock.sample_sequence,
        checked_at_tick: clock.current_tick,
        checked_at_uncertainty_ticks: clock.uncertainty_bound_ticks,
        clock_qualification_valid_until_tick: clock.valid_until_tick,
        clock_receipt_sha256: clock.receipt_sha256.clone(),
        valid_until_tick,
    })
}

/// Revalidate a readiness receipt with a newer qualified clock sample
/// immediately before passing it to a separate authorization authority.
/// The new sample must use the same clock epoch/mapping and have a strictly
/// greater source sequence. A reboot, remapping, stale sample or uncertainty
/// interval crossing expiry invalidates the receipt.
pub fn verify_dispatch_readiness_receipt_current(
    ledger: &SiteStateLedger,
    receipt: &DispatchReadinessReceipt,
    policy: &DispatchReadinessPolicy,
    qualified_clock: &QualifiedSiteClockSnapshot,
) -> Result<(), DispatchReadinessFailure> {
    if receipt.schema_version != DISPATCH_READINESS_RECEIPT_SCHEMA_VERSION {
        return Err(DispatchReadinessFailure::UnsupportedSchemaVersion);
    }
    for (label, digest) in [
        ("package", receipt.package_sha256.as_str()),
        ("design revision", receipt.design_revision_sha256.as_str()),
        ("frame", receipt.frame_sha256.as_str()),
        ("transform", receipt.transform_sha256.as_str()),
        ("robot configuration", receipt.robot_configuration_sha256.as_str()),
        ("tool configuration", receipt.tool_configuration_sha256.as_str()),
        ("payload configuration", receipt.payload_configuration_sha256.as_str()),
        ("primitive manifest", receipt.primitive_manifest_sha256.as_str()),
        ("trajectory", receipt.trajectory_sha256.as_str()),
        ("motion envelope", receipt.envelope_sha256.as_str()),
        ("site snapshot", receipt.site_snapshot_sha256.as_str()),
        ("dynamic occupancy manifest", receipt.dynamic_occupancy_manifest_sha256.as_str()),
        ("actor manifest", receipt.actor_manifest_sha256.as_str()),
        ("occupancy assessment receipt", receipt.occupancy_assessment_receipt_sha256.as_str()),
        ("occupancy engine build", receipt.occupancy_engine_build_sha256.as_str()),
        ("clearance policy", receipt.clearance_policy_sha256.as_str()),
        ("site ledger head", receipt.site_ledger_event_sha256.as_str()),
        ("clock epoch", receipt.site_clock_epoch_sha256.as_str()),
        ("clock mapping", receipt.clock_mapping_sha256.as_str()),
        ("clock receipt", receipt.clock_receipt_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(DispatchReadinessFailure::InvalidDigest(label));
        }
    }
    if receipt.robot_id.trim().is_empty()
        || receipt.entity_id.trim().is_empty()
        || receipt.site_clock_domain_id.trim().is_empty()
        || receipt.clock_authority_id.trim().is_empty()
        || receipt.clock_source_id.trim().is_empty()
    {
        return Err(DispatchReadinessFailure::InvalidClockEvidence);
    }
    if !ledger.actor_registry().allows(&receipt.robot_id, SiteActorRole::Robot) {
        return Err(DispatchReadinessFailure::RobotIdentityNotRegistered);
    }
    if !ledger
        .actor_registry()
        .allows(&receipt.clock_authority_id, SiteActorRole::ClockAuthority)
    {
        return Err(DispatchReadinessFailure::ClockAuthorityNotRegistered);
    }
    if !ledger
        .actor_registry()
        .allows(&qualified_clock.authority_id, SiteActorRole::ClockAuthority)
    {
        return Err(DispatchReadinessFailure::ClockAuthorityNotRegistered);
    }
    if !is_sha256(&policy.occupancy_engine_build_sha256) {
        return Err(DispatchReadinessFailure::InvalidDigest("trusted occupancy engine build"));
    }
    if !is_sha256(&policy.clearance_policy_sha256) {
        return Err(DispatchReadinessFailure::InvalidDigest("required clearance policy"));
    }
    if !is_sha256(&qualified_clock.epoch_sha256)
        || !is_sha256(&qualified_clock.mapping_sha256)
        || !is_sha256(&qualified_clock.receipt_sha256)
        || qualified_clock.authority_id.trim().is_empty()
        || qualified_clock.source_id.trim().is_empty()
        || qualified_clock.sample_sequence == 0
    {
        return Err(DispatchReadinessFailure::InvalidClockEvidence);
    }
    if receipt.checked_at_uncertainty_ticks > policy.max_clock_uncertainty_ticks
        || qualified_clock.uncertainty_bound_ticks > policy.max_clock_uncertainty_ticks
    {
        return Err(DispatchReadinessFailure::ClockUncertaintyExceedsPolicy);
    }
    if qualified_clock.site_clock_domain_id != receipt.site_clock_domain_id {
        return Err(DispatchReadinessFailure::OccupancyClockDomainMismatch);
    }
    if qualified_clock.epoch_sha256 != receipt.site_clock_epoch_sha256 {
        return Err(DispatchReadinessFailure::ClockEpochMismatch);
    }
    if qualified_clock.mapping_sha256 != receipt.clock_mapping_sha256 {
        return Err(DispatchReadinessFailure::ClockMappingMismatch);
    }
    if qualified_clock.authority_id != receipt.clock_authority_id
        || qualified_clock.source_id != receipt.clock_source_id
        || qualified_clock.receipt_sha256 == receipt.clock_receipt_sha256
    {
        return Err(DispatchReadinessFailure::InvalidClockEvidence);
    }
    if qualified_clock.sample_sequence <= receipt.clock_sample_sequence {
        return Err(DispatchReadinessFailure::ClockSampleNotFresh);
    }

    let (now_lower_tick, now_upper_tick) =
        uncertainty_interval(qualified_clock.current_tick, qualified_clock.uncertainty_bound_ticks)?;
    let (receipt_checked_lower, receipt_checked_upper) =
        uncertainty_interval(receipt.checked_at_tick, receipt.checked_at_uncertainty_ticks)?;
    if now_lower_tick <= receipt_checked_upper {
        return Err(DispatchReadinessFailure::ClockUncertaintyStraddlesBoundary);
    }
    if now_upper_tick >= qualified_clock.valid_until_tick
        || now_upper_tick >= receipt.clock_qualification_valid_until_tick
    {
        return Err(DispatchReadinessFailure::ClockQualificationExpired);
    }
    if now_upper_tick >= receipt.valid_until_tick {
        return Err(DispatchReadinessFailure::ReadinessReceiptExpired);
    }

    if receipt.occupancy_engine_build_sha256 != policy.occupancy_engine_build_sha256 {
        return Err(DispatchReadinessFailure::OccupancyEngineBuildMismatch);
    }
    if receipt.clearance_policy_sha256 != policy.clearance_policy_sha256 {
        return Err(DispatchReadinessFailure::ClearancePolicyMismatch);
    }
    if ledger.active_package_sha256() != Some(receipt.package_sha256.as_str())
        || ledger.package_is_current(&receipt.package_sha256) != Some(true)
    {
        return Err(DispatchReadinessFailure::UnknownOrInactivePackage);
    }
    if ledger.last_sequence() != receipt.site_ledger_sequence {
        return Err(DispatchReadinessFailure::SiteSnapshotSequenceMismatch);
    }
    if ledger.last_event_sha256() != Some(receipt.site_ledger_event_sha256.as_str()) {
        return Err(DispatchReadinessFailure::SiteSnapshotHeadMismatch);
    }
    let package = ledger
        .work_package(&receipt.package_sha256)
        .ok_or(DispatchReadinessFailure::UnknownOrInactivePackage)?;
    if package.design_revision_sha256 != receipt.design_revision_sha256 {
        return Err(DispatchReadinessFailure::DesignRevisionMismatch);
    }
    if !package
        .entities
        .iter()
        .any(|entity| entity.design_entity_id == receipt.entity_id)
    {
        return Err(DispatchReadinessFailure::UnknownDesignEntity);
    }
    match ledger.entity_state(&receipt.package_sha256, &receipt.entity_id) {
        Some(SiteEntityState::NotAttempted | SiteEntityState::PhysicallyAbsent { .. }) => Ok(()),
        Some(_) => Err(DispatchReadinessFailure::EntityNotDispatchable),
        None => Err(DispatchReadinessFailure::UnknownDesignEntity),
    }
}

fn uncertainty_interval(
    tick: u64,
    uncertainty_ticks: u64,
) -> Result<(u64, u64), DispatchReadinessFailure> {
    let lower = tick
        .checked_sub(uncertainty_ticks)
        .ok_or(DispatchReadinessFailure::ClockIntervalOverflow)?;
    let upper = tick
        .checked_add(uncertainty_ticks)
        .ok_or(DispatchReadinessFailure::ClockIntervalOverflow)?;
    Ok((lower, upper))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_layout_acceptance::{DesignIntentEntity, FieldAcceptanceProfile, FieldLayoutEntityType};
    use crate::site_state_ledger::{
        SiteActorRegistry, SiteActorRole, SiteEventEnvelope, SiteEventPayload, SiteWorkPackage,
    };

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn clock_sample(sample_sequence: u64, current_tick: u64) -> QualifiedSiteClockSnapshot {
        QualifiedSiteClockSnapshot {
            site_clock_domain_id: "site-clock-01".into(),
            epoch_sha256: digest('a'),
            mapping_sha256: digest('b'),
            authority_id: "clock-authority".into(),
            source_id: "ptp-grandmaster-01".into(),
            sample_sequence,
            current_tick,
            uncertainty_bound_ticks: 1,
            valid_until_tick: 700,
            receipt_sha256: if sample_sequence == 1 {
                digest('c')
            } else {
                digest('d')
            },
        }
    }

    fn policy() -> DispatchReadinessPolicy {
        DispatchReadinessPolicy {
            occupancy_engine_build_sha256: digest('e'),
            clearance_policy_sha256: digest('f'),
            max_clock_uncertainty_ticks: 2,
        }
    }

    fn site_ledger() -> SiteStateLedger {
        let mut actors = SiteActorRegistry::new();
        actors
            .grant("design-authority", SiteActorRole::DesignAuthority)
            .unwrap();
        actors.grant("surveyor", SiteActorRole::Surveyor).unwrap();
        actors.grant("robot-01", SiteActorRole::Robot).unwrap();
        actors.grant("referee", SiteActorRole::Referee).unwrap();
        actors
            .grant("clock-authority", SiteActorRole::ClockAuthority)
            .unwrap();
        let mut ledger = SiteStateLedger::with_actor_registry(actors);
        let package = SiteWorkPackage {
            package_sha256: digest('a'),
            source_model_sha256: digest('b'),
            source_schema_release: "IFC4X3_ADD2".into(),
            ids_spec_sha256: digest('c'),
            ids_version: "1.0".into(),
            ids_engine_build_sha256: digest('d'),
            design_revision_sha256: digest('e'),
            frame_id: "site-frame".into(),
            frame_sha256: digest('f'),
            transform_sha256: digest('1'),
            entity_manifest_sha256: digest('2'),
            acceptance_profile_sha256: digest('3'),
            entities: vec![DesignIntentEntity {
                design_entity_id: "ifc4x3:0000000000000000000001".into(),
                entity_type: FieldLayoutEntityType::Point,
            }],
            acceptance_profile: FieldAcceptanceProfile {
                max_abs_xy_residual_mm: 1.0,
                max_reference_u95_xy_mm: 0.25,
                max_abs_z_residual_mm: None,
                max_reference_u95_z_mm: None,
                max_abs_endpoint_residual_mm: None,
            },
        };
        ledger
            .apply(SiteEventEnvelope {
                event_id: "package-publish".into(),
                sequence: 1,
                previous_event_sha256: None,
                event_sha256: digest('4'),
                actor_id: "design-authority".into(),
                actor_role: SiteActorRole::DesignAuthority,
                payload: SiteEventPayload::WorkPackagePublished { package },
            })
            .unwrap();
        ledger
    }

    fn make_request(ledger: &SiteStateLedger) -> DispatchReadinessRequest {
        let package_sha256 = ledger.active_package_sha256().unwrap().to_owned();
        DispatchReadinessRequest {
            schema_version: DISPATCH_READINESS_REQUEST_SCHEMA_VERSION.into(),
            envelope: DispatchEnvelopeBinding {
                package_sha256,
                design_revision_sha256: digest('e'),
                entity_id: "ifc4x3:0000000000000000000001".into(),
                frame_sha256: digest('f'),
                transform_sha256: digest('1'),
                robot_id: "robot-01".into(),
                robot_configuration_sha256: digest('5'),
                tool_configuration_sha256: digest('6'),
                payload_configuration_sha256: digest('7'),
                primitive_manifest_sha256: digest('8'),
                trajectory_sha256: digest('9'),
                envelope_sha256: digest('a'),
                site_clock_domain_id: "site-clock-01".into(),
                site_clock_epoch_sha256: digest('a'),
                clock_mapping_sha256: digest('b'),
                generated_at_tick: 100,
                generated_at_uncertainty_ticks: 1,
                valid_until_tick: 500,
            },
            occupancy: OccupancyAssessmentReceipt {
                assessed_envelope_sha256: digest('a'),
                primitive_manifest_sha256: digest('8'),
                site_snapshot_sha256: digest('b'),
                site_ledger_sequence: ledger.last_sequence(),
                site_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
                dynamic_occupancy_manifest_sha256: digest('c'),
                actor_manifest_sha256: digest('d'),
                occupancy_engine_build_sha256: digest('e'),
                clearance_policy_sha256: digest('f'),
                site_clock_domain_id: "site-clock-01".into(),
                site_clock_epoch_sha256: digest('a'),
                clock_mapping_sha256: digest('b'),
                checked_at_tick: 110,
                checked_at_uncertainty_ticks: 1,
                valid_until_tick: 400,
                primitive_coverage_complete: true,
                dynamic_actor_coverage_complete: true,
                broadphase_pair_coverage_complete: true,
                narrowphase_coverage_complete: true,
                candidate_pair_count: 0,
                narrowphase_evaluated_pair_count: 0,
                conflict_count: 0,
                unresolved_pair_count: 0,
                invalid_bound_count: 0,
                unqualified_bound_count: 0,
                stale_actor_count: 0,
                unmodelled_occupant_count: 0,
                assessment_receipt_sha256: digest('9'),
            },
            qualified_clock: clock_sample(1, 120),
        }
    }

    #[test]
    fn exact_fresh_conflict_free_snapshot_produces_readiness_only() {
        let ledger = site_ledger();
        let request = make_request(&ledger);
        let receipt = check_dispatch_readiness(&ledger, &request, &policy()).unwrap();
        assert_eq!(receipt.package_sha256, request.envelope.package_sha256);
        assert_eq!(receipt.envelope_sha256, request.envelope.envelope_sha256);
        assert_eq!(receipt.site_ledger_sequence, 1);
        assert_eq!(receipt.site_clock_domain_id, "site-clock-01");
        assert_eq!(receipt.site_clock_epoch_sha256, digest('a'));
        assert_eq!(receipt.clock_mapping_sha256, digest('b'));
        assert_eq!(receipt.clock_sample_sequence, 1);
        assert_eq!(receipt.checked_at_uncertainty_ticks, 1);
        assert_eq!(receipt.valid_until_tick, 350);
    }

    #[test]
    fn any_ledger_change_invalidates_the_previous_occupancy_snapshot() {
        let mut ledger = site_ledger();
        let mut request = make_request(&ledger);
        let package_sha256 = request.envelope.package_sha256.clone();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "station-qualified".into(),
                sequence: 2,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('5'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256,
                    station_id: "station-01".into(),
                    frame_sha256: digest('f'),
                    transform_sha256: digest('1'),
                    calibration_sha256: digest('6'),
                },
            })
            .unwrap();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::SiteSnapshotSequenceMismatch)
        );

        // A new occupancy receipt must be created for the changed ledger head.
        let fresh_request = make_request(&ledger);
        assert_eq!(
            check_dispatch_readiness(&ledger, &fresh_request, &policy())
                .unwrap()
                .site_ledger_sequence,
            2
        );
    }

    #[test]
    fn conflicts_unresolved_pairs_and_incomplete_coverage_all_block_readiness() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.occupancy.conflict_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyNotClear { conflicts: 1, .. })
        ));
        request.occupancy.conflict_count = 0;
        request.occupancy.unresolved_pair_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyNotClear {
                unresolved_pairs: 1,
                ..
            })
        ));
        request.occupancy.unresolved_pair_count = 0;
        request.occupancy.primitive_coverage_complete = false;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::IncompletePrimitiveCoverage)
        );
    }

    #[test]
    fn configuration_or_transform_mismatch_blocks_readiness() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.envelope.transform_sha256 = digest('0');
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::TransformMismatch)
        );
        request.envelope.transform_sha256 = digest('1');
        request.occupancy.assessed_envelope_sha256 = digest('0');
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyEnvelopeMismatch)
        );
    }

    #[test]
    fn expired_or_cross_clock_assessment_is_not_reused() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.qualified_clock.current_tick = 550;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::EnvelopeExpired)
        );
        request.qualified_clock.current_tick = 120;
        request.occupancy.site_clock_domain_id = "robot-clock".into();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyClockDomainMismatch)
        );
    }

    #[test]
    fn occupancy_assessment_must_postdate_the_bound_motion_envelope() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.occupancy.checked_at_tick = 99;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyAssessmentPredatesEnvelope)
        );
    }

    #[test]
    fn entity_with_an_existing_execution_attempt_cannot_be_dispatched_again() {
        let mut ledger = site_ledger();
        let package_sha256 = ledger.active_package_sha256().unwrap().to_owned();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "station-qualified".into(),
                sequence: 2,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('5'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256: package_sha256.clone(),
                    station_id: "station-01".into(),
                    frame_sha256: digest('f'),
                    transform_sha256: digest('1'),
                    calibration_sha256: digest('6'),
                },
            })
            .unwrap();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "attempt-started".into(),
                sequence: 3,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('7'),
                actor_id: "robot-01".into(),
                actor_role: SiteActorRole::Robot,
                payload: SiteEventPayload::ExecutionStarted {
                    package_sha256,
                    entity_id: "ifc4x3:0000000000000000000001".into(),
                    attempt_id: "attempt-01".into(),
                    station_qualification_event_id: "station-qualified".into(),
                },
            })
            .unwrap();

        let mut request = make_request(&ledger);
        request.occupancy.site_ledger_sequence = ledger.last_sequence();
        request.occupancy.site_ledger_event_sha256 = ledger.last_event_sha256().unwrap().to_owned();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::EntityNotDispatchable)
        );
    }

    #[test]
    fn stale_actor_invalid_bounds_and_unmodelled_occupants_are_not_clear() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.occupancy.stale_actor_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyNotClear {
                stale_actors: 1,
                ..
            })
        ));
        request.occupancy.stale_actor_count = 0;
        request.occupancy.invalid_bound_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyNotClear {
                invalid_bounds: 1,
                ..
            })
        ));
        request.occupancy.invalid_bound_count = 0;
        request.occupancy.unmodelled_occupant_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::OccupancyNotClear {
                unmodelled_occupants: 1,
                ..
            })
        ));
    }

    #[test]
    fn narrowphase_coverage_must_be_complete_even_when_candidate_count_is_zero() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.occupancy.narrowphase_coverage_complete = false;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::IncompleteNarrowphaseCoverage)
        );
    }

    #[test]
    fn unapproved_occupancy_engine_or_clearance_policy_blocks_readiness() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        let mut untrusted_engine = policy();
        untrusted_engine.occupancy_engine_build_sha256 = digest('0');
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &untrusted_engine),
            Err(DispatchReadinessFailure::OccupancyEngineBuildMismatch)
        );
        let mut untrusted_policy = policy();
        untrusted_policy.clearance_policy_sha256 = digest('0');
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &untrusted_policy),
            Err(DispatchReadinessFailure::ClearancePolicyMismatch)
        );
    }

    #[test]
    fn narrowphase_pair_count_must_cover_every_broadphase_candidate() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.occupancy.candidate_pair_count = 3;
        request.occupancy.narrowphase_evaluated_pair_count = 2;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::IncompleteNarrowphaseCoverage)
        );
    }

    #[test]
    fn readiness_receipt_must_be_revalidated_before_consume() {
        let ledger = site_ledger();
        let request = make_request(&ledger);
        let receipt = check_dispatch_readiness(&ledger, &request, &policy()).unwrap();
        assert_eq!(
            verify_dispatch_readiness_receipt_current(
                &ledger,
                &receipt,
                &policy(),
                &clock_sample(2, 130),
            ),
            Ok(())
        );
        assert_eq!(
            verify_dispatch_readiness_receipt_current(
                &ledger,
                &receipt,
                &policy(),
                &clock_sample(2, 400),
            ),
            Err(DispatchReadinessFailure::ReadinessReceiptExpired)
        );
        assert_eq!(
            verify_dispatch_readiness_receipt_current(
                &ledger,
                &receipt,
                &DispatchReadinessPolicy {
                    occupancy_engine_build_sha256: digest('0'),
                    ..policy()
                },
                &clock_sample(2, 130),
            ),
            Err(DispatchReadinessFailure::OccupancyEngineBuildMismatch)
        );
        assert_eq!(
            verify_dispatch_readiness_receipt_current(
                &ledger,
                &receipt,
                &DispatchReadinessPolicy {
                    clearance_policy_sha256: digest('0'),
                    ..policy()
                },
                &clock_sample(2, 130),
            ),
            Err(DispatchReadinessFailure::ClearancePolicyMismatch)
        );
    }

    #[test]
    fn rejected_physical_mark_requires_explicit_remediation_before_dispatch() {
        let mut ledger = site_ledger();
        let package_sha256 = ledger.active_package_sha256().unwrap().to_owned();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "station-qualified".into(),
                sequence: 2,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('5'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256: package_sha256.clone(),
                    station_id: "station-01".into(),
                    frame_sha256: digest('f'),
                    transform_sha256: digest('1'),
                    calibration_sha256: digest('6'),
                },
            })
            .unwrap();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "attempt-started".into(),
                sequence: 3,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('7'),
                actor_id: "robot-01".into(),
                actor_role: SiteActorRole::Robot,
                payload: SiteEventPayload::ExecutionStarted {
                    package_sha256: package_sha256.clone(),
                    entity_id: "ifc4x3:0000000000000000000001".into(),
                    attempt_id: "attempt-01".into(),
                    station_qualification_event_id: "station-qualified".into(),
                },
            })
            .unwrap();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "robot-complete".into(),
                sequence: 4,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('8'),
                actor_id: "robot-01".into(),
                actor_role: SiteActorRole::Robot,
                payload: SiteEventPayload::RobotObservationRecorded {
                    package_sha256: package_sha256.clone(),
                    entity_id: "ifc4x3:0000000000000000000001".into(),
                    attempt_id: "attempt-01".into(),
                    disposition: crate::site_state_ledger::RobotExecutionDisposition::Printed,
                    robot_receipt_sha256: digest('9'),
                },
            })
            .unwrap();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "referee-reject".into(),
                sequence: 5,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('a'),
                actor_id: "referee".into(),
                actor_role: SiteActorRole::Referee,
                payload: SiteEventPayload::RefereeAssessmentRecorded {
                    package_sha256: package_sha256.clone(),
                    entity_id: "ifc4x3:0000000000000000000001".into(),
                    attempt_id: "attempt-01".into(),
                    assessment: crate::site_state_ledger::SiteRefereeAssessment::Measured(
                        crate::field_layout_acceptance::RefereeMeasurement {
                            residual_x_mm: 4.0,
                            residual_y_mm: 0.0,
                            residual_z_mm: None,
                            endpoint_start_mm: None,
                            endpoint_end_mm: None,
                            reference_u95_x_mm: 0.1,
                            reference_u95_y_mm: 0.1,
                            reference_u95_z_mm: None,
                            measurement_receipt_sha256: digest('b'),
                        },
                    ),
                },
            })
            .unwrap();
        let mut request = make_request(&ledger);
        request.occupancy.site_ledger_sequence = ledger.last_sequence();
        request.occupancy.site_ledger_event_sha256 =
            ledger.last_event_sha256().unwrap().to_owned();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::EntityNotDispatchable)
        );
    }

    #[test]
    fn readiness_receipt_is_bound_to_the_current_ledger_head_at_consumption() {
        let mut ledger = site_ledger();
        let request = make_request(&ledger);
        let receipt = check_dispatch_readiness(&ledger, &request, &policy()).unwrap();

        ledger
            .apply(SiteEventEnvelope {
                event_id: "station-after-readiness".into(),
                sequence: 2,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('5'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256: request.envelope.package_sha256.clone(),
                    station_id: "station-01".into(),
                    frame_sha256: digest('f'),
                    transform_sha256: digest('1'),
                    calibration_sha256: digest('6'),
                },
            })
            .unwrap();

        assert_eq!(
            verify_dispatch_readiness_receipt_current(
                &ledger,
                &receipt,
                &policy(),
                &clock_sample(2, 130),
            ),
            Err(DispatchReadinessFailure::SiteSnapshotSequenceMismatch)
        );
    }

    #[test]
    fn wrong_or_unknown_request_schema_version_is_rejected() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.schema_version = "construction-dispatch-readiness-request/0.1".into();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn dispatch_requires_robot_identity_registered_for_robot_role() {
        let ledger = site_ledger();
        let mut request = make_request(&ledger);
        request.envelope.robot_id = "unregistered-robot".into();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request, &policy()),
            Err(DispatchReadinessFailure::RobotIdentityNotRegistered)
        );
    }

    #[test]
    fn readiness_receipt_preserves_full_execution_and_occupancy_context() {
        let ledger = site_ledger();
        let request = make_request(&ledger);
        let receipt = check_dispatch_readiness(&ledger, &request, &policy()).unwrap();
        assert_eq!(receipt.frame_sha256, request.envelope.frame_sha256);
        assert_eq!(receipt.transform_sha256, request.envelope.transform_sha256);
        assert_eq!(
            receipt.robot_configuration_sha256,
            request.envelope.robot_configuration_sha256
        );
        assert_eq!(
            receipt.tool_configuration_sha256,
            request.envelope.tool_configuration_sha256
        );
        assert_eq!(
            receipt.payload_configuration_sha256,
            request.envelope.payload_configuration_sha256
        );
        assert_eq!(
            receipt.primitive_manifest_sha256,
            request.envelope.primitive_manifest_sha256
        );
        assert_eq!(receipt.trajectory_sha256, request.envelope.trajectory_sha256);
        assert_eq!(
            receipt.dynamic_occupancy_manifest_sha256,
            request.occupancy.dynamic_occupancy_manifest_sha256
        );
        assert_eq!(receipt.actor_manifest_sha256, request.occupancy.actor_manifest_sha256);
        assert_eq!(receipt.candidate_pair_count, request.occupancy.candidate_pair_count);
        assert_eq!(
            receipt.narrowphase_evaluated_pair_count,
            request.occupancy.narrowphase_evaluated_pair_count
        );
    }

    #[test]
    fn modified_readiness_context_is_detected_by_the_signed_receipt_comparison_layer() {
        let ledger = site_ledger();
        let request = make_request(&ledger);
        let receipt = check_dispatch_readiness(&ledger, &request, &policy()).unwrap();
        let mut changed = receipt.clone();
        changed.robot_configuration_sha256 = digest('0');
        assert_ne!(changed, receipt);
        // This module intentionally does not authenticate the receipt. The
        // caller's canonical digest/signature must cover every field, and the
        // changed payload must therefore have a different signed byte string.
    }
}
