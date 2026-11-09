#![forbid(unsafe_code)]

//! A deliberately narrow VDA 5050 -> construction-contract projection guard.
//!
//! This crate classifies robot/fleet observations and validates exact sidecar
//! bindings. It contains no task dispatcher, MQTT client, safety controller,
//! physical evidence evaluator, identity authenticator, or acceptance API.

/// How a construction semantic relates to a VDA 5050 v3 surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingClass {
    /// VDA has a value, but it identifies only a VDA transport concept.
    TransportMetadataOnly,
    /// A constrained projection is possible, but it retains only VDA meaning.
    BoundedProjection,
    /// The construction value must be carried and checked out of band.
    SidecarRequired,
    /// VDA 5050 does not provide an equivalent semantic.
    NotRepresentable,
}

/// Required handling when projecting a VDA value into a construction contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossRule {
    KeepTransportIdentityDistinct,
    KeepTransportRevisionDistinct,
    RequireExactSidecarAndRejectIfMissing,
    ProjectOnlyAsNavigationData,
    ProjectOnlyAsFleetTrafficData,
    TreatAsReportedCapabilityOnly,
    ProjectOnlyAsExecutorCommands,
    NeverPromoteExecutorEvidence,
    RequireExternalEvidenceReceipt,
    RequireExternalInspectionReceipt,
    NeverPromoteToIndependentAcceptance,
    NeverPromoteToSafetyOrMotionAuthority,
    RetainConstructionEventIdentityOutOfBand,
    TreatAsIdentityClaimNotAuthentication,
}

/// One row in the closed, versioned semantic crosswalk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldMapping {
    pub canonical_field: &'static str,
    pub vda_surface: Option<&'static str>,
    pub class: MappingClass,
    pub sidecar_required: bool,
    pub loss_rule: LossRule,
}

/// Canonical fields required by the construction work-contract profile.
pub const REQUIRED_CANONICAL_FIELDS: &[&str] = &[
    "task.id",
    "task.update_revision",
    "work_package.id",
    "design.revision_digest",
    "site.epoch",
    "spatial.target_frame",
    "spatial.transform_bundle_digest",
    "navigation.map_identity",
    "work_zone.id",
    "robot.capability_profile",
    "robot.configuration_digest",
    "task.family",
    "task.route",
    "task.robot_actions",
    "task.preconditions_postconditions",
    "resource.reservation",
    "execution.progress",
    "output.physical_evidence",
    "inspection.result",
    "acceptance.independent",
    "safety.authorization",
    "message.event_identity",
    "robot.identity",
];

/// Closed reference mapping table. It never claims VDA-native construction acceptance.
pub const FIELD_MAPPINGS: &[FieldMapping] = &[
    FieldMapping {
        canonical_field: "task.id",
        vda_surface: Some("order.orderId"),
        class: MappingClass::TransportMetadataOnly,
        sidecar_required: true,
        loss_rule: LossRule::KeepTransportIdentityDistinct,
    },
    FieldMapping {
        canonical_field: "task.update_revision",
        vda_surface: Some("order.orderUpdateId"),
        class: MappingClass::TransportMetadataOnly,
        sidecar_required: true,
        loss_rule: LossRule::KeepTransportRevisionDistinct,
    },
    FieldMapping {
        canonical_field: "work_package.id",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "design.revision_digest",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "site.epoch",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "spatial.target_frame",
        vda_surface: Some("order.nodes[].nodePosition"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "spatial.transform_bundle_digest",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "navigation.map_identity",
        vda_surface: Some("state.mobileRobotPosition.mapId; state.maps[].mapVersion"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::ProjectOnlyAsNavigationData,
    },
    FieldMapping {
        canonical_field: "work_zone.id",
        vda_surface: Some("zoneSet.zones[].zoneId"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::ProjectOnlyAsFleetTrafficData,
    },
    FieldMapping {
        canonical_field: "robot.capability_profile",
        vda_surface: Some("factsheet.typeSpecifications"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::TreatAsReportedCapabilityOnly,
    },
    FieldMapping {
        canonical_field: "robot.configuration_digest",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "task.family",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "task.route",
        vda_surface: Some("order.nodes[]; order.edges[]"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::ProjectOnlyAsNavigationData,
    },
    FieldMapping {
        canonical_field: "task.robot_actions",
        vda_surface: Some("order.nodes[].actions[]; order.edges[].actions[]"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::ProjectOnlyAsExecutorCommands,
    },
    FieldMapping {
        canonical_field: "task.preconditions_postconditions",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExactSidecarAndRejectIfMissing,
    },
    FieldMapping {
        canonical_field: "resource.reservation",
        vda_surface: Some("zoneSet; state.zoneRequests; responses"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::ProjectOnlyAsFleetTrafficData,
    },
    FieldMapping {
        canonical_field: "execution.progress",
        vda_surface: Some(
            "state.nodeStates[]; state.edgeStates[]; state.actionStates[]; state.lastNodeId",
        ),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::NeverPromoteExecutorEvidence,
    },
    FieldMapping {
        canonical_field: "output.physical_evidence",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExternalEvidenceReceipt,
    },
    FieldMapping {
        canonical_field: "inspection.result",
        vda_surface: None,
        class: MappingClass::SidecarRequired,
        sidecar_required: true,
        loss_rule: LossRule::RequireExternalInspectionReceipt,
    },
    FieldMapping {
        canonical_field: "acceptance.independent",
        vda_surface: None,
        class: MappingClass::NotRepresentable,
        sidecar_required: true,
        loss_rule: LossRule::NeverPromoteToIndependentAcceptance,
    },
    FieldMapping {
        canonical_field: "safety.authorization",
        vda_surface: None,
        class: MappingClass::NotRepresentable,
        sidecar_required: true,
        loss_rule: LossRule::NeverPromoteToSafetyOrMotionAuthority,
    },
    FieldMapping {
        canonical_field: "message.event_identity",
        vda_surface: Some("headerId; orderId; orderUpdateId"),
        class: MappingClass::TransportMetadataOnly,
        sidecar_required: true,
        loss_rule: LossRule::RetainConstructionEventIdentityOutOfBand,
    },
    FieldMapping {
        canonical_field: "robot.identity",
        vda_surface: Some("manufacturer; serialNumber"),
        class: MappingClass::BoundedProjection,
        sidecar_required: true,
        loss_rule: LossRule::TreatAsIdentityClaimNotAuthentication,
    },
];

/// Errors found in the static crosswalk table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingTableError {
    DuplicateCanonicalField,
    MissingRequiredCanonicalField,
    SidecarNotRequired,
    AcceptanceIncorrectlyRepresented,
    SafetyIncorrectlyRepresented,
}

/// Validate completeness and the two most important non-equivalence rules.
pub fn validate_mapping_table() -> Result<(), MappingTableError> {
    for (index, row) in FIELD_MAPPINGS.iter().enumerate() {
        if !row.sidecar_required {
            return Err(MappingTableError::SidecarNotRequired);
        }
        if FIELD_MAPPINGS[index + 1..]
            .iter()
            .any(|other| other.canonical_field == row.canonical_field)
        {
            return Err(MappingTableError::DuplicateCanonicalField);
        }
    }
    for required in REQUIRED_CANONICAL_FIELDS {
        if !FIELD_MAPPINGS
            .iter()
            .any(|row| row.canonical_field == *required)
        {
            return Err(MappingTableError::MissingRequiredCanonicalField);
        }
    }
    let acceptance = mapping_for("acceptance.independent")
        .ok_or(MappingTableError::AcceptanceIncorrectlyRepresented)?;
    if acceptance.class != MappingClass::NotRepresentable
        || acceptance.vda_surface.is_some()
        || acceptance.loss_rule != LossRule::NeverPromoteToIndependentAcceptance
    {
        return Err(MappingTableError::AcceptanceIncorrectlyRepresented);
    }
    let safety = mapping_for("safety.authorization")
        .ok_or(MappingTableError::SafetyIncorrectlyRepresented)?;
    if safety.class != MappingClass::NotRepresentable
        || safety.vda_surface.is_some()
        || safety.loss_rule != LossRule::NeverPromoteToSafetyOrMotionAuthority
    {
        return Err(MappingTableError::SafetyIncorrectlyRepresented);
    }
    Ok(())
}

/// Look up one canonical field in the fixed reference table.
pub fn mapping_for(canonical_field: &str) -> Option<&'static FieldMapping> {
    FIELD_MAPPINGS
        .iter()
        .find(|row| row.canonical_field == canonical_field)
}

/// VDA-level observations accepted by this projection seam.
///
/// No variant means construction work is accepted or safety authority is granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdaObservation<'a> {
    OrderReceived {
        order_id: &'a str,
        order_update_id: u32,
    },
    NodeTraversed {
        order_id: &'a str,
        node_id: &'a str,
        sequence_id: u32,
    },
    ActionStateFinished {
        order_id: &'a str,
        action_id: &'a str,
    },
    ZoneAccessGranted {
        zone_set_id: &'a str,
        zone_id: &'a str,
    },
    RobotError {
        error_type: &'a str,
    },
    ConnectionLost,
    StateUnavailable,
    Unrecognized {
        signal_type: &'a str,
    },
}

/// Meaning-preserving projection of VDA observations.
///
/// This enum intentionally has no Completed, Accepted, Authorized, or Safe variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection<'a> {
    TransportOrderReceived {
        order_id: &'a str,
        order_update_id: u32,
    },
    RouteProgressReported {
        order_id: &'a str,
        node_id: &'a str,
        sequence_id: u32,
    },
    ExecutorActionFinishedReported {
        order_id: &'a str,
        action_id: &'a str,
    },
    FleetTrafficAccessReported {
        zone_set_id: &'a str,
        zone_id: &'a str,
    },
    RobotFaultReported {
        error_type: &'a str,
    },
    CommunicationUnavailable,
    RobotStateUnavailable,
    UnclassifiedSignal {
        signal_type: &'a str,
    },
}

/// Project a VDA observation without upgrading its authority or evidence class.
pub fn project_observation<'a>(observation: VdaObservation<'a>) -> Projection<'a> {
    match observation {
        VdaObservation::OrderReceived {
            order_id,
            order_update_id,
        } => Projection::TransportOrderReceived {
            order_id,
            order_update_id,
        },
        VdaObservation::NodeTraversed {
            order_id,
            node_id,
            sequence_id,
        } => Projection::RouteProgressReported {
            order_id,
            node_id,
            sequence_id,
        },
        VdaObservation::ActionStateFinished {
            order_id,
            action_id,
        } => Projection::ExecutorActionFinishedReported {
            order_id,
            action_id,
        },
        VdaObservation::ZoneAccessGranted {
            zone_set_id,
            zone_id,
        } => Projection::FleetTrafficAccessReported {
            zone_set_id,
            zone_id,
        },
        VdaObservation::RobotError { error_type } => Projection::RobotFaultReported { error_type },
        VdaObservation::ConnectionLost => Projection::CommunicationUnavailable,
        VdaObservation::StateUnavailable => Projection::RobotStateUnavailable,
        VdaObservation::Unrecognized { signal_type } => {
            Projection::UnclassifiedSignal { signal_type }
        }
    }
}

/// Exact task context that must accompany a VDA transport order out of band.
///
/// Digests are required to be 64 lowercase hexadecimal characters. This crate
/// checks their syntax and equality only; it does not calculate or authenticate them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstructionSidecar<'a> {
    pub task_id: &'a str,
    pub work_package_id: &'a str,
    pub vda_order_id: &'a str,
    pub vda_order_update_id: u32,
    pub design_revision_digest: &'a str,
    pub site_epoch: u64,
    pub target_frame_id: &'a str,
    pub transform_bundle_digest: &'a str,
    pub robot_configuration_digest: &'a str,
    pub capability_profile_digest: &'a str,
    pub work_zone_id: &'a str,
    pub resource_reservation_digest: &'a str,
    pub preconditions_digest: &'a str,
    pub postconditions_digest: &'a str,
    pub evidence_profile_ref: &'a str,
    pub acceptance_authority_ref: &'a str,
}

/// Fields used in precise sidecar validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextField {
    TaskId,
    WorkPackageId,
    VdaOrderId,
    VdaOrderUpdateId,
    DesignRevisionDigest,
    SiteEpoch,
    TargetFrameId,
    TransformBundleDigest,
    RobotConfigurationDigest,
    CapabilityProfileDigest,
    WorkZoneId,
    ResourceReservationDigest,
    PreconditionsDigest,
    PostconditionsDigest,
    EvidenceProfileRef,
    AcceptanceAuthorityRef,
}

/// Structural binding failures. These are not authentication or authorization results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextError {
    MissingSidecar,
    EmptyField(ContextField),
    InvalidDigest(ContextField),
    StaleSiteEpoch { expected: u64, current: u64 },
    Mismatch(ContextField),
}

/// Validate completeness, digest syntax, task/order binding, and current site epoch.
///
/// The caller must independently authenticate the sidecar, the actor, and any
/// referenced authority/evidence. Success does not grant dispatch or motion authority.
pub fn validate_sidecar_binding(
    expected: &ConstructionSidecar<'_>,
    presented: Option<&ConstructionSidecar<'_>>,
    current_site_epoch: u64,
) -> Result<(), ContextError> {
    let presented = presented.ok_or(ContextError::MissingSidecar)?;
    validate_context_shape(expected)?;
    validate_context_shape(presented)?;

    if current_site_epoch != expected.site_epoch {
        return Err(ContextError::StaleSiteEpoch {
            expected: expected.site_epoch,
            current: current_site_epoch,
        });
    }

    let comparisons = [
        (expected.task_id == presented.task_id, ContextField::TaskId),
        (
            expected.work_package_id == presented.work_package_id,
            ContextField::WorkPackageId,
        ),
        (
            expected.vda_order_id == presented.vda_order_id,
            ContextField::VdaOrderId,
        ),
        (
            expected.vda_order_update_id == presented.vda_order_update_id,
            ContextField::VdaOrderUpdateId,
        ),
        (
            expected.design_revision_digest == presented.design_revision_digest,
            ContextField::DesignRevisionDigest,
        ),
        (
            expected.site_epoch == presented.site_epoch,
            ContextField::SiteEpoch,
        ),
        (
            expected.target_frame_id == presented.target_frame_id,
            ContextField::TargetFrameId,
        ),
        (
            expected.transform_bundle_digest == presented.transform_bundle_digest,
            ContextField::TransformBundleDigest,
        ),
        (
            expected.robot_configuration_digest == presented.robot_configuration_digest,
            ContextField::RobotConfigurationDigest,
        ),
        (
            expected.capability_profile_digest == presented.capability_profile_digest,
            ContextField::CapabilityProfileDigest,
        ),
        (
            expected.work_zone_id == presented.work_zone_id,
            ContextField::WorkZoneId,
        ),
        (
            expected.resource_reservation_digest == presented.resource_reservation_digest,
            ContextField::ResourceReservationDigest,
        ),
        (
            expected.preconditions_digest == presented.preconditions_digest,
            ContextField::PreconditionsDigest,
        ),
        (
            expected.postconditions_digest == presented.postconditions_digest,
            ContextField::PostconditionsDigest,
        ),
        (
            expected.evidence_profile_ref == presented.evidence_profile_ref,
            ContextField::EvidenceProfileRef,
        ),
        (
            expected.acceptance_authority_ref == presented.acceptance_authority_ref,
            ContextField::AcceptanceAuthorityRef,
        ),
    ];
    for (matches, field) in comparisons {
        if !matches {
            return Err(ContextError::Mismatch(field));
        }
    }

    Ok(())
}

fn validate_context_shape(context: &ConstructionSidecar<'_>) -> Result<(), ContextError> {
    let required_strings = [
        (context.task_id, ContextField::TaskId),
        (context.work_package_id, ContextField::WorkPackageId),
        (context.vda_order_id, ContextField::VdaOrderId),
        (context.target_frame_id, ContextField::TargetFrameId),
        (context.work_zone_id, ContextField::WorkZoneId),
        (
            context.evidence_profile_ref,
            ContextField::EvidenceProfileRef,
        ),
        (
            context.acceptance_authority_ref,
            ContextField::AcceptanceAuthorityRef,
        ),
    ];
    for (value, field) in required_strings {
        if value.trim().is_empty() {
            return Err(ContextError::EmptyField(field));
        }
    }
    let digests = [
        (
            context.design_revision_digest,
            ContextField::DesignRevisionDigest,
        ),
        (
            context.transform_bundle_digest,
            ContextField::TransformBundleDigest,
        ),
        (
            context.robot_configuration_digest,
            ContextField::RobotConfigurationDigest,
        ),
        (
            context.capability_profile_digest,
            ContextField::CapabilityProfileDigest,
        ),
        (
            context.resource_reservation_digest,
            ContextField::ResourceReservationDigest,
        ),
        (
            context.preconditions_digest,
            ContextField::PreconditionsDigest,
        ),
        (
            context.postconditions_digest,
            ContextField::PostconditionsDigest,
        ),
    ];
    for (value, field) in digests {
        if !is_lower_hex_digest(value) {
            return Err(ContextError::InvalidDigest(field));
        }
    }
    Ok(())
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn context() -> ConstructionSidecar<'static> {
        ConstructionSidecar {
            task_id: "task-01",
            work_package_id: "wp-01",
            vda_order_id: "transport-91",
            vda_order_update_id: 2,
            design_revision_digest: DIGEST_A,
            site_epoch: 12,
            target_frame_id: "site-grid-01",
            transform_bundle_digest: DIGEST_B,
            robot_configuration_digest: DIGEST_A,
            capability_profile_digest: DIGEST_B,
            work_zone_id: "zone-08",
            resource_reservation_digest: DIGEST_A,
            preconditions_digest: DIGEST_B,
            postconditions_digest: DIGEST_A,
            evidence_profile_ref: "acceptance-profile-04",
            acceptance_authority_ref: "authority-ref-02",
        }
    }

    #[test]
    fn mapping_table_is_complete_unique_and_closed() {
        assert_eq!(validate_mapping_table(), Ok(()));
        assert_eq!(FIELD_MAPPINGS.len(), REQUIRED_CANONICAL_FIELDS.len());
        for field in REQUIRED_CANONICAL_FIELDS {
            assert!(mapping_for(field).is_some(), "missing mapping for {field}");
        }
    }

    #[test]
    fn task_and_order_ids_remain_distinct() {
        let mapping = mapping_for("task.id").expect("task id mapping");
        assert_eq!(mapping.vda_surface, Some("order.orderId"));
        assert_eq!(mapping.class, MappingClass::TransportMetadataOnly);
        assert_eq!(mapping.loss_rule, LossRule::KeepTransportIdentityDistinct);
    }

    #[test]
    fn order_update_is_not_a_design_revision() {
        let mapping = mapping_for("task.update_revision").expect("update mapping");
        assert_eq!(mapping.vda_surface, Some("order.orderUpdateId"));
        assert_eq!(mapping.class, MappingClass::TransportMetadataOnly);
        assert_eq!(mapping.loss_rule, LossRule::KeepTransportRevisionDistinct);
    }

    #[test]
    fn map_identity_is_navigation_only() {
        let mapping = mapping_for("navigation.map_identity").expect("map mapping");
        assert_eq!(mapping.class, MappingClass::BoundedProjection);
        assert_eq!(mapping.loss_rule, LossRule::ProjectOnlyAsNavigationData);
    }

    #[test]
    fn zone_identity_is_traffic_only() {
        let mapping = mapping_for("work_zone.id").expect("zone mapping");
        assert_eq!(mapping.class, MappingClass::BoundedProjection);
        assert_eq!(mapping.loss_rule, LossRule::ProjectOnlyAsFleetTrafficData);
    }

    #[test]
    fn independent_acceptance_is_not_representable() {
        let mapping = mapping_for("acceptance.independent").expect("acceptance mapping");
        assert_eq!(mapping.class, MappingClass::NotRepresentable);
        assert_eq!(mapping.vda_surface, None);
        assert_eq!(
            mapping.loss_rule,
            LossRule::NeverPromoteToIndependentAcceptance
        );
    }

    #[test]
    fn safety_authority_is_not_representable() {
        let mapping = mapping_for("safety.authorization").expect("safety mapping");
        assert_eq!(mapping.class, MappingClass::NotRepresentable);
        assert_eq!(mapping.vda_surface, None);
        assert_eq!(
            mapping.loss_rule,
            LossRule::NeverPromoteToSafetyOrMotionAuthority
        );
    }

    #[test]
    fn action_finished_is_executor_report_only() {
        assert_eq!(
            project_observation(VdaObservation::ActionStateFinished {
                order_id: "transport-91",
                action_id: "drill-1",
            }),
            Projection::ExecutorActionFinishedReported {
                order_id: "transport-91",
                action_id: "drill-1",
            }
        );
    }

    #[test]
    fn node_traversal_is_route_progress_only() {
        assert_eq!(
            project_observation(VdaObservation::NodeTraversed {
                order_id: "transport-91",
                node_id: "n-9",
                sequence_id: 6,
            }),
            Projection::RouteProgressReported {
                order_id: "transport-91",
                node_id: "n-9",
                sequence_id: 6,
            }
        );
    }

    #[test]
    fn zone_grant_is_fleet_traffic_only() {
        assert_eq!(
            project_observation(VdaObservation::ZoneAccessGranted {
                zone_set_id: "zs-4",
                zone_id: "zone-8",
            }),
            Projection::FleetTrafficAccessReported {
                zone_set_id: "zs-4",
                zone_id: "zone-8",
            }
        );
    }

    #[test]
    fn disconnect_never_implies_completion() {
        assert_eq!(
            project_observation(VdaObservation::ConnectionLost),
            Projection::CommunicationUnavailable
        );
    }

    #[test]
    fn unknown_vda_extension_is_not_success() {
        assert_eq!(
            project_observation(VdaObservation::Unrecognized {
                signal_type: "vendor_extension_unknown",
            }),
            Projection::UnclassifiedSignal {
                signal_type: "vendor_extension_unknown",
            }
        );
    }

    #[test]
    fn missing_sidecar_is_rejected() {
        assert_eq!(
            validate_sidecar_binding(&context(), None, 12).err(),
            Some(ContextError::MissingSidecar)
        );
    }

    #[test]
    fn exact_sidecar_is_structurally_bound_not_authorized() {
        let expected = context();
        let presented = context();
        assert_eq!(
            validate_sidecar_binding(&expected, Some(&presented), 12),
            Ok(())
        );
    }

    #[test]
    fn stale_site_epoch_is_rejected() {
        assert_eq!(
            validate_sidecar_binding(&context(), Some(&context()), 13).err(),
            Some(ContextError::StaleSiteEpoch {
                expected: 12,
                current: 13,
            })
        );
    }

    #[test]
    fn changed_design_revision_is_rejected() {
        let expected = context();
        let mut presented = context();
        presented.design_revision_digest = DIGEST_B;
        assert_eq!(
            validate_sidecar_binding(&expected, Some(&presented), 12).err(),
            Some(ContextError::Mismatch(ContextField::DesignRevisionDigest))
        );
    }

    #[test]
    fn changed_transport_order_update_is_rejected() {
        let expected = context();
        let mut presented = context();
        presented.vda_order_update_id = 3;
        assert_eq!(
            validate_sidecar_binding(&expected, Some(&presented), 12).err(),
            Some(ContextError::Mismatch(ContextField::VdaOrderUpdateId))
        );
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut presented = context();
        presented.robot_configuration_digest = "ABC123";
        assert_eq!(
            validate_sidecar_binding(&context(), Some(&presented), 12).err(),
            Some(ContextError::InvalidDigest(
                ContextField::RobotConfigurationDigest
            ))
        );
    }

    #[test]
    fn blank_required_sidecar_field_is_rejected() {
        let mut presented = context();
        presented.work_zone_id = "  ";
        assert_eq!(
            validate_sidecar_binding(&context(), Some(&presented), 12).err(),
            Some(ContextError::EmptyField(ContextField::WorkZoneId))
        );
    }
}
