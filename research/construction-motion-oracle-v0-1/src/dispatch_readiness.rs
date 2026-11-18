//! Evidence-bound dispatch readiness for construction work.
//!
//! This gate joins a frozen work package, a robot-specific motion envelope and
//! a fresh site-occupancy assessment. It is deliberately a *readiness* check:
//! it does not issue a safety permit, authorize actuators, or replace a
//! safety-rated protective system. The caller must verify canonical receipt
//! bytes and actor signatures before constructing these typed records.

use crate::site_state_ledger::{SiteEntityState, SiteStateLedger};

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
    pub generated_at_tick: u64,
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
    pub checked_at_tick: u64,
    pub valid_until_tick: u64,
    /// These are independent completeness assertions made by the occupancy
    /// adapter and bound to the assessment receipt digest.
    pub primitive_coverage_complete: bool,
    pub dynamic_actor_coverage_complete: bool,
    pub broadphase_pair_coverage_complete: bool,
    pub narrowphase_coverage_complete: bool,
    pub candidate_pair_count: u64,
    pub conflict_count: u64,
    pub unresolved_pair_count: u64,
    pub invalid_bound_count: u64,
    pub stale_actor_count: u64,
    pub unmodelled_occupant_count: u64,
    /// Digest of canonical occupancy assessment bytes, verified by caller.
    pub assessment_receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchReadinessRequest {
    pub envelope: DispatchEnvelopeBinding,
    pub occupancy: OccupancyAssessmentReceipt,
    /// Current tick in the declared site clock domain, from a qualified
    /// clock source. This is not a wall-clock timestamp.
    pub now_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchReadinessReceipt {
    pub package_sha256: String,
    pub design_revision_sha256: String,
    pub entity_id: String,
    pub robot_id: String,
    pub envelope_sha256: String,
    pub site_snapshot_sha256: String,
    pub occupancy_assessment_receipt_sha256: String,
    pub site_ledger_sequence: u64,
    pub site_ledger_event_sha256: String,
    pub site_clock_domain_id: String,
    pub checked_at_tick: u64,
    pub valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchReadinessFailure {
    InvalidDigest(&'static str),
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
    OccupancyClockDomainMismatch,
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
        stale_actors: u64,
        unmodelled_occupants: u64,
    },
}

/// Check whether a work item is eligible to enter a separate authorization
/// process. No actuation or safety permit is produced by this function.
pub fn check_dispatch_readiness(
    ledger: &SiteStateLedger,
    request: &DispatchReadinessRequest,
) -> Result<DispatchReadinessReceipt, DispatchReadinessFailure> {
    let envelope = &request.envelope;
    let occupancy = &request.occupancy;

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
        ("occupancy envelope", occupancy.assessed_envelope_sha256.as_str()),
        ("occupancy primitive manifest", occupancy.primitive_manifest_sha256.as_str()),
        ("site snapshot", occupancy.site_snapshot_sha256.as_str()),
        ("site ledger head", occupancy.site_ledger_event_sha256.as_str()),
        ("dynamic occupancy manifest", occupancy.dynamic_occupancy_manifest_sha256.as_str()),
        ("actor manifest", occupancy.actor_manifest_sha256.as_str()),
        ("occupancy engine build", occupancy.occupancy_engine_build_sha256.as_str()),
        ("clearance policy", occupancy.clearance_policy_sha256.as_str()),
        ("assessment receipt", occupancy.assessment_receipt_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(DispatchReadinessFailure::InvalidDigest(label));
        }
    }
    if envelope.entity_id.trim().is_empty() || envelope.robot_id.trim().is_empty()
        || envelope.site_clock_domain_id.trim().is_empty()
        || occupancy.site_clock_domain_id.trim().is_empty()
    {
        return Err(DispatchReadinessFailure::OccupancyClockDomainMismatch);
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
        Some(
            SiteEntityState::NotAttempted
            | SiteEntityState::Rejected { .. }
            | SiteEntityState::PhysicallyAbsent { .. },
        ) => {}
        Some(_) => return Err(DispatchReadinessFailure::EntityNotDispatchable),
        None => return Err(DispatchReadinessFailure::UnknownDesignEntity),
    }

    if envelope.valid_until_tick <= envelope.generated_at_tick {
        return Err(DispatchReadinessFailure::EnvelopeTimeWindowInvalid);
    }
    if request.now_tick < envelope.generated_at_tick {
        return Err(DispatchReadinessFailure::EnvelopeNotYetValid);
    }
    if request.now_tick >= envelope.valid_until_tick {
        return Err(DispatchReadinessFailure::EnvelopeExpired);
    }

    if occupancy.assessed_envelope_sha256 != envelope.envelope_sha256 {
        return Err(DispatchReadinessFailure::OccupancyEnvelopeMismatch);
    }
    if occupancy.primitive_manifest_sha256 != envelope.primitive_manifest_sha256 {
        return Err(DispatchReadinessFailure::OccupancyPrimitiveManifestMismatch);
    }
    if occupancy.site_clock_domain_id != envelope.site_clock_domain_id {
        return Err(DispatchReadinessFailure::OccupancyClockDomainMismatch);
    }
    if occupancy.site_ledger_sequence != ledger.last_sequence() {
        return Err(DispatchReadinessFailure::SiteSnapshotSequenceMismatch);
    }
    if ledger.last_event_sha256() != Some(occupancy.site_ledger_event_sha256.as_str()) {
        return Err(DispatchReadinessFailure::SiteSnapshotHeadMismatch);
    }
    if occupancy.valid_until_tick <= occupancy.checked_at_tick {
        return Err(DispatchReadinessFailure::OccupancyTimeWindowInvalid);
    }
    if occupancy.checked_at_tick < envelope.generated_at_tick {
        return Err(DispatchReadinessFailure::OccupancyAssessmentPredatesEnvelope);
    }
    if request.now_tick < occupancy.checked_at_tick {
        return Err(DispatchReadinessFailure::OccupancyAssessmentFromFuture);
    }
    if request.now_tick >= occupancy.valid_until_tick {
        return Err(DispatchReadinessFailure::OccupancyAssessmentExpired);
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
    if !occupancy.narrowphase_coverage_complete {
        return Err(DispatchReadinessFailure::IncompleteNarrowphaseCoverage);
    }
    if occupancy.conflict_count != 0
        || occupancy.unresolved_pair_count != 0
        || occupancy.invalid_bound_count != 0
        || occupancy.stale_actor_count != 0
        || occupancy.unmodelled_occupant_count != 0
    {
        return Err(DispatchReadinessFailure::OccupancyNotClear {
            conflicts: occupancy.conflict_count,
            unresolved_pairs: occupancy.unresolved_pair_count,
            invalid_bounds: occupancy.invalid_bound_count,
            stale_actors: occupancy.stale_actor_count,
            unmodelled_occupants: occupancy.unmodelled_occupant_count,
        });
    }

    let valid_until_tick = envelope.valid_until_tick.min(occupancy.valid_until_tick);
    Ok(DispatchReadinessReceipt {
        package_sha256: envelope.package_sha256.clone(),
        design_revision_sha256: envelope.design_revision_sha256.clone(),
        entity_id: envelope.entity_id.clone(),
        robot_id: envelope.robot_id.clone(),
        envelope_sha256: envelope.envelope_sha256.clone(),
        site_snapshot_sha256: occupancy.site_snapshot_sha256.clone(),
        occupancy_assessment_receipt_sha256: occupancy.assessment_receipt_sha256.clone(),
        site_ledger_sequence: ledger.last_sequence(),
        site_ledger_event_sha256: ledger
            .last_event_sha256()
            .unwrap_or_default()
            .to_owned(),
        checked_at_tick: request.now_tick,
        valid_until_tick,
    })
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
    use crate::site_state_ledger::{SiteActorRole, SiteEventEnvelope, SiteEventPayload, SiteWorkPackage};

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn site_ledger() -> SiteStateLedger {
        let mut ledger = SiteStateLedger::new();
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

    fn request(ledger: &SiteStateLedger) -> DispatchReadinessRequest {
        let package_sha256 = ledger.active_package_sha256().unwrap().to_owned();
        DispatchReadinessRequest {
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
                generated_at_tick: 100,
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
                checked_at_tick: 110,
                valid_until_tick: 400,
                primitive_coverage_complete: true,
                dynamic_actor_coverage_complete: true,
                broadphase_pair_coverage_complete: true,
                narrowphase_coverage_complete: true,
                candidate_pair_count: 0,
                conflict_count: 0,
                unresolved_pair_count: 0,
                invalid_bound_count: 0,
                stale_actor_count: 0,
                unmodelled_occupant_count: 0,
                assessment_receipt_sha256: digest('9'),
            },
            now_tick: 120,
        }
    }

    #[test]
    fn exact_fresh_conflict_free_snapshot_produces_readiness_only() {
        let ledger = site_ledger();
        let request = request(&ledger);
        let receipt = check_dispatch_readiness(&ledger, &request).unwrap();
        assert_eq!(receipt.package_sha256, request.envelope.package_sha256);
        assert_eq!(receipt.envelope_sha256, request.envelope.envelope_sha256);
        assert_eq!(receipt.site_ledger_sequence, 1);
        assert_eq!(receipt.valid_until_tick, 400);
    }

    #[test]
    fn any_ledger_change_invalidates_the_previous_occupancy_snapshot() {
        let mut ledger = site_ledger();
        let mut request = request(&ledger);
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
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::SiteSnapshotSequenceMismatch)
        );
        request.occupancy.site_ledger_sequence = ledger.last_sequence();
        request.occupancy.site_ledger_event_sha256 =
            ledger.last_event_sha256().unwrap().to_owned();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request).unwrap().site_ledger_sequence,
            2
        );
    }

    #[test]
    fn conflicts_unresolved_pairs_and_incomplete_coverage_all_block_readiness() {
        let ledger = site_ledger();
        let mut request = request(&ledger);
        request.occupancy.conflict_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::OccupancyNotClear { conflicts: 1, .. })
        ));
        request.occupancy.conflict_count = 0;
        request.occupancy.unresolved_pair_count = 1;
        assert!(matches!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::OccupancyNotClear {
                unresolved_pairs: 1,
                ..
            })
        ));
        request.occupancy.unresolved_pair_count = 0;
        request.occupancy.primitive_coverage_complete = false;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::IncompletePrimitiveCoverage)
        );
    }

    #[test]
    fn configuration_or_transform_mismatch_blocks_readiness() {
        let ledger = site_ledger();
        let mut request = request(&ledger);
        request.envelope.transform_sha256 = digest('0');
        assert_eq!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::TransformMismatch)
        );
        request.envelope.transform_sha256 = digest('1');
        request.occupancy.assessed_envelope_sha256 = digest('0');
        assert_eq!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::OccupancyEnvelopeMismatch)
        );
    }

    #[test]
    fn expired_or_cross_clock_assessment_is_not_reused() {
        let ledger = site_ledger();
        let mut request = request(&ledger);
        request.now_tick = 450;
        assert_eq!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::EnvelopeExpired)
        );
        request.now_tick = 120;
        request.occupancy.site_clock_domain_id = "robot-clock".into();
        assert_eq!(
            check_dispatch_readiness(&ledger, &request),
            Err(DispatchReadinessFailure::OccupancyClockDomainMismatch)
        );
    }
}
