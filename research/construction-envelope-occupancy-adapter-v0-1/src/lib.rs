#![forbid(unsafe_code)]

//! Pure, typed projection from a validated motion-envelope slice to the
//! existing construction space-time occupancy owner.
//!
//! The motion producer owns the already-expanded physical, stopping and
//! coordination bounds. The occupancy owner owns spatial indexing, conflict
//! evaluation and resource admission. This adapter only validates the binding
//! and projects the final coordination bound exactly once.

use std::fmt;

use construction_motion_envelope::{Context, EnvelopeSlice, Report};
use construction_robot_interop_core::resources::ReservationRequest;
use construction_spacetime_occupancy_core::{
    Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancy,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    EmptyField(&'static str),
    InvalidDigest(&'static str),
    DuplicateExpectedPrimitive(String),
    UnexpectedPrimitive(String),
    SliceNotInReport,
    InvalidSliceInterval,
    InvalidBounds {
        role: &'static str,
        axis: usize,
    },
    OuterBoundDoesNotContain {
        outer: &'static str,
        inner: &'static str,
    },
    TaskMismatch,
    SiteEpochMismatch,
    DesignRevisionMismatch,
    FrameMismatch,
    ClockDomainMismatch,
    ReservationIntervalMismatch,
    ZeroReservationUnits,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProjectionError {}

/// Returned occupancy plus the complete producer-side lineage needed to
/// explain where the projected bounds came from.
///
/// Canonical bytes are retained verbatim. This type does not compute, claim or
/// authenticate a cryptographic digest; consumers may hash the bytes using
/// their established evidence/digest owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedOccupancy {
    pub occupancy: SpatialOccupancy,
    pub lineage: EnvelopeLineage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvelopeLineage {
    /// Deterministic source report bytes from Report::canonical_bytes().
    pub source_report_canonical_bytes: Vec<u8>,
    pub source_context: Context,
    pub source_slice: EnvelopeSlice,
    pub occupancy_id: String,
    pub occupancy_kind: OccupancyKind,
    pub site_binding: SiteBinding,
    pub reservation: ReservationRequest,
}

/// Project one member slice into the existing occupancy representation.
///
/// The caller supplies floor identity, occupancy identity/class and the exact
/// ReservationRequest from the existing resource owner. This function does
/// not create or grant a reservation and does not mutate any occupancy index.
///
/// Crucially, coordination_outer already includes physical uncertainty,
/// stopping/recovery reachability and policy clearance. Therefore the
/// occupancy-level expansion margins are always zero; applying them again
/// would double-expand the declared space.
pub fn project_slice(
    report: &Report,
    slice: &EnvelopeSlice,
    site_binding: SiteBinding,
    reservation: ReservationRequest,
    occupancy_id: impl Into<String>,
    occupancy_kind: OccupancyKind,
) -> Result<ProjectedOccupancy, ProjectionError> {
    let occupancy_id = occupancy_id.into();
    require_non_empty("occupancy_id", &occupancy_id)?;
    validate_context(&report.context)?;
    validate_site_binding(&site_binding)?;
    validate_reservation(&reservation)?;

    if !report.slices.iter().any(|candidate| candidate == slice) {
        return Err(ProjectionError::SliceNotInReport);
    }
    if slice.start_ms >= slice.end_ms {
        return Err(ProjectionError::InvalidSliceInterval);
    }
    if !report
        .context
        .expected_primitive_ids
        .iter()
        .any(|expected| expected == &slice.primitive_id)
    {
        return Err(ProjectionError::UnexpectedPrimitive(
            slice.primitive_id.clone(),
        ));
    }
    validate_evidence(&slice.evidence)?;
    validate_slice_bounds(slice)?;

    if reservation.task_id != report.context.task_id {
        return Err(ProjectionError::TaskMismatch);
    }
    if site_binding.site_epoch != report.context.site_epoch
        || reservation.site_epoch != report.context.site_epoch
    {
        return Err(ProjectionError::SiteEpochMismatch);
    }
    if site_binding.design_revision_digest != report.context.design_revision_sha256 {
        return Err(ProjectionError::DesignRevisionMismatch);
    }
    if site_binding.target_frame_id != report.context.frame_id {
        return Err(ProjectionError::FrameMismatch);
    }
    if reservation.clock_domain_id != report.context.clock_domain_id {
        return Err(ProjectionError::ClockDomainMismatch);
    }
    if reservation.interval_start_ms != slice.start_ms
        || reservation.interval_end_ms != slice.end_ms
    {
        return Err(ProjectionError::ReservationIntervalMismatch);
    }

    let bounds = Bounds3Mm {
        min: [
            slice.coordination_outer.min.x,
            slice.coordination_outer.min.y,
            slice.coordination_outer.min.z,
        ],
        max: [
            slice.coordination_outer.max.x,
            slice.coordination_outer.max.y,
            slice.coordination_outer.max.z,
        ],
    };

    let occupancy = SpatialOccupancy {
        occupancy_id: occupancy_id.clone(),
        task_id: report.context.task_id.clone(),
        kind: occupancy_kind,
        binding: site_binding.clone(),
        bounds,
        // These bounds have already been expanded by the producer.
        uncertainty_margin_mm: [0, 0, 0],
        clearance_margin_mm: [0, 0, 0],
        reservation: reservation.clone(),
    };

    let lineage = EnvelopeLineage {
        source_report_canonical_bytes: report.canonical_bytes(),
        source_context: report.context.clone(),
        source_slice: slice.clone(),
        occupancy_id,
        occupancy_kind,
        site_binding,
        reservation,
    };

    Ok(ProjectedOccupancy { occupancy, lineage })
}

fn validate_context(context: &Context) -> Result<(), ProjectionError> {
    for (field, value) in [
        ("task_id", context.task_id.as_str()),
        ("site_epoch", context.site_epoch.as_str()),
        ("frame_id", context.frame_id.as_str()),
        ("clock_domain_id", context.clock_domain_id.as_str()),
        (
            "design_revision_sha256",
            context.design_revision_sha256.as_str(),
        ),
        ("primitive_set_ref", context.primitive_set_ref.as_str()),
        ("calibration_ref", context.calibration_ref.as_str()),
        (
            "controller_profile_ref",
            context.controller_profile_ref.as_str(),
        ),
    ] {
        require_non_empty(field, value)?;
    }

    for (field, digest) in [
        (
            "platform_config_sha256",
            context.platform_config_sha256.as_str(),
        ),
        ("tool_config_sha256", context.tool_config_sha256.as_str()),
        (
            "payload_config_sha256",
            context.payload_config_sha256.as_str(),
        ),
        (
            "collision_model_sha256",
            context.collision_model_sha256.as_str(),
        ),
        ("transform_sha256", context.transform_sha256.as_str()),
        (
            "design_revision_sha256",
            context.design_revision_sha256.as_str(),
        ),
        (
            "trajectory_profile_sha256",
            context.trajectory_profile_sha256.as_str(),
        ),
    ] {
        if !is_sha256(digest) {
            return Err(ProjectionError::InvalidDigest(field));
        }
    }

    if context.expected_primitive_ids.is_empty() {
        return Err(ProjectionError::EmptyField("expected_primitive_ids"));
    }

    let mut seen = std::collections::BTreeSet::new();
    for primitive_id in &context.expected_primitive_ids {
        require_non_empty("expected_primitive_id", primitive_id)?;
        if !seen.insert(primitive_id) {
            return Err(ProjectionError::DuplicateExpectedPrimitive(
                primitive_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_site_binding(binding: &SiteBinding) -> Result<(), ProjectionError> {
    require_non_empty("floor_id", &binding.floor_id)?;
    require_non_empty("site_binding.site_epoch", &binding.site_epoch)?;
    require_non_empty(
        "site_binding.design_revision_digest",
        &binding.design_revision_digest,
    )?;
    require_non_empty("site_binding.target_frame_id", &binding.target_frame_id)
}

fn validate_reservation(request: &ReservationRequest) -> Result<(), ProjectionError> {
    require_non_empty("reservation_id", &request.reservation_id)?;
    require_non_empty("resource_id", &request.resource_id)?;
    require_non_empty("reservation.task_id", &request.task_id)?;
    require_non_empty("reservation.owner_id", &request.owner_id)?;
    require_non_empty("reservation.site_epoch", &request.site_epoch)?;
    require_non_empty("reservation.clock_domain_id", &request.clock_domain_id)?;
    if request.interval_start_ms >= request.interval_end_ms {
        return Err(ProjectionError::InvalidSliceInterval);
    }
    if request.units == 0 {
        return Err(ProjectionError::ZeroReservationUnits);
    }
    Ok(())
}

fn validate_evidence(
    evidence: &construction_motion_envelope::EvidenceRefs,
) -> Result<(), ProjectionError> {
    for (field, value) in [
        ("path_deviation_evidence", evidence.path_deviation.as_str()),
        ("localization_evidence", evidence.localization.as_str()),
        ("tracking_evidence", evidence.tracking.as_str()),
        (
            "model_uncertainty_evidence",
            evidence.model_uncertainty.as_str(),
        ),
        (
            "numerical_enclosure_evidence",
            evidence.numerical_enclosure.as_str(),
        ),
        (
            "stopping_reachability_evidence",
            evidence.stopping_reachability.as_str(),
        ),
        (
            "policy_clearance_evidence",
            evidence.policy_clearance.as_str(),
        ),
    ] {
        require_non_empty(field, value)?;
    }
    Ok(())
}

fn validate_slice_bounds(slice: &EnvelopeSlice) -> Result<(), ProjectionError> {
    validate_bounds("physical_swept_outer", slice.physical_swept_outer)?;
    validate_bounds("stopping_outer", slice.stopping_outer)?;
    validate_bounds("coordination_outer", slice.coordination_outer)?;

    if !contains(slice.stopping_outer, slice.physical_swept_outer) {
        return Err(ProjectionError::OuterBoundDoesNotContain {
            outer: "stopping_outer",
            inner: "physical_swept_outer",
        });
    }
    if !contains(slice.coordination_outer, slice.stopping_outer) {
        return Err(ProjectionError::OuterBoundDoesNotContain {
            outer: "coordination_outer",
            inner: "stopping_outer",
        });
    }
    if slice.radius_mm == 0 {
        return Err(ProjectionError::EmptyField("positive primitive radius"));
    }
    Ok(())
}

fn validate_bounds(
    role: &'static str,
    bounds: construction_motion_envelope::AabbMm,
) -> Result<(), ProjectionError> {
    let minimum = [bounds.min.x, bounds.min.y, bounds.min.z];
    let maximum = [bounds.max.x, bounds.max.y, bounds.max.z];
    for (axis, (minimum, maximum)) in minimum.iter().zip(maximum.iter()).enumerate() {
        if minimum >= maximum {
            return Err(ProjectionError::InvalidBounds { role, axis });
        }
    }
    Ok(())
}

fn contains(
    outer: construction_motion_envelope::AabbMm,
    inner: construction_motion_envelope::AabbMm,
) -> bool {
    outer.min.x <= inner.min.x
        && outer.min.y <= inner.min.y
        && outer.min.z <= inner.min.z
        && outer.max.x >= inner.max.x
        && outer.max.y >= inner.max.y
        && outer.max.z >= inner.max.z
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ProjectionError> {
    if value.trim().is_empty() {
        Err(ProjectionError::EmptyField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use construction_motion_envelope::{
        AxisMm, Context, EvidenceRefs, MarginsMm, Point3Mm, Segment, build,
    };

    fn context() -> Context {
        Context {
            project_id: "project".into(),
            work_package_id: "package".into(),
            task_id: "task-001".into(),
            trajectory_id: "trajectory-001".into(),
            robot_id: "robot-001".into(),
            platform_config_sha256: "a".repeat(64),
            tool_config_sha256: "b".repeat(64),
            payload_config_sha256: "c".repeat(64),
            collision_model_sha256: "d".repeat(64),
            primitive_set_ref: "manifest://robot-001".into(),
            expected_primitive_ids: vec!["tool".into()],
            calibration_ref: "calibration://robot-001".into(),
            controller_profile_ref: "controller://robot-001".into(),
            frame_id: "site-frame".into(),
            transform_sha256: "e".repeat(64),
            design_revision_sha256: "f".repeat(64),
            site_epoch: "site-epoch-7".into(),
            clock_domain_id: "site-monotonic-clock".into(),
            trajectory_profile_sha256: "1".repeat(64),
        }
    }

    fn report() -> Report {
        let segment = Segment {
            slice_index: 0,
            primitive_id: "tool".into(),
            start_ms: 10,
            end_ms: 40,
            from_mm: Point3Mm {
                x: 100,
                y: 200,
                z: 300,
            },
            to_mm: Point3Mm {
                x: 200,
                y: 200,
                z: 300,
            },
            radius_mm: 10,
            margins: MarginsMm {
                path_deviation: AxisMm { x: 1, y: 2, z: 3 },
                localization: AxisMm { x: 4, y: 5, z: 6 },
                tracking: AxisMm { x: 7, y: 8, z: 9 },
                model_uncertainty: AxisMm {
                    x: 10,
                    y: 11,
                    z: 12,
                },
                numerical_enclosure: AxisMm {
                    x: 13,
                    y: 14,
                    z: 15,
                },
                stopping_reachability: AxisMm {
                    x: 16,
                    y: 17,
                    z: 18,
                },
                policy_clearance: AxisMm {
                    x: 19,
                    y: 20,
                    z: 21,
                },
            },
            evidence: EvidenceRefs {
                path_deviation: "evidence://path-deviation".into(),
                localization: "evidence://localization".into(),
                tracking: "evidence://tracking".into(),
                model_uncertainty: "evidence://model".into(),
                numerical_enclosure: "evidence://numerical".into(),
                stopping_reachability: "evidence://stopping".into(),
                policy_clearance: "evidence://policy".into(),
            },
        };
        build(&context(), &[segment]).unwrap()
    }

    fn binding(context: &Context) -> SiteBinding {
        SiteBinding {
            floor_id: "floor-02".into(),
            site_epoch: context.site_epoch.clone(),
            design_revision_digest: context.design_revision_sha256.clone(),
            target_frame_id: context.frame_id.clone(),
        }
    }

    fn reservation(context: &Context, slice: &EnvelopeSlice) -> ReservationRequest {
        ReservationRequest {
            reservation_id: "reservation-001".into(),
            resource_id: "robot-resource-001".into(),
            task_id: context.task_id.clone(),
            owner_id: "owner-001".into(),
            site_epoch: context.site_epoch.clone(),
            clock_domain_id: context.clock_domain_id.clone(),
            clock_uncertainty_ms: 2,
            interval_start_ms: slice.start_ms,
            interval_end_ms: slice.end_ms,
            expires_at_ms: 1_000,
            units: 1,
        }
    }

    fn project(report: &Report) -> Result<ProjectedOccupancy, ProjectionError> {
        let slice = &report.slices[0];
        project_slice(
            report,
            slice,
            binding(&report.context),
            reservation(&report.context, slice),
            "occupancy-001",
            OccupancyKind::RobotOperatingEnvelope,
        )
    }

    #[test]
    fn maps_final_coordination_bounds_once_and_keeps_full_lineage() {
        let source = report();
        let result = project(&source).unwrap();
        let slice = &source.slices[0];

        assert_eq!(
            result.occupancy.bounds,
            Bounds3Mm {
                min: [
                    slice.coordination_outer.min.x,
                    slice.coordination_outer.min.y,
                    slice.coordination_outer.min.z,
                ],
                max: [
                    slice.coordination_outer.max.x,
                    slice.coordination_outer.max.y,
                    slice.coordination_outer.max.z,
                ],
            }
        );
        assert_eq!(result.occupancy.uncertainty_margin_mm, [0, 0, 0]);
        assert_eq!(result.occupancy.clearance_margin_mm, [0, 0, 0]);
        assert_eq!(result.occupancy.reservation, result.lineage.reservation);
        assert_eq!(
            result.lineage.source_report_canonical_bytes,
            source.canonical_bytes()
        );
        assert_eq!(result.lineage.source_context, source.context);
        assert_eq!(result.lineage.source_slice, source.slices[0]);
        assert_eq!(result.lineage.site_binding, result.occupancy.binding);
    }

    #[test]
    fn preserves_every_margin_channel_and_evidence_reference() {
        let source = report();
        let result = project(&source).unwrap();
        assert_eq!(
            result.lineage.source_slice.margins,
            source.slices[0].margins
        );
        assert_eq!(
            result.lineage.source_slice.evidence,
            source.slices[0].evidence
        );
    }

    #[test]
    fn repeated_projection_is_byte_and_value_deterministic() {
        let source = report();
        let first = project(&source).unwrap();
        let second = project(&source).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_slice_not_owned_by_report() {
        let source = report();
        let mut foreign = source.slices[0].clone();
        foreign.primitive_id = "other-tool".into();
        let context = &source.context;
        assert_eq!(
            project_slice(
                &source,
                &foreign,
                binding(context),
                reservation(context, &source.slices[0]),
                "occupancy-foreign",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::SliceNotInReport
        );
    }

    #[test]
    fn rejects_task_mismatch() {
        let source = report();
        let slice = &source.slices[0];
        let mut request = reservation(&source.context, slice);
        request.task_id = "different-task".into();
        assert_eq!(
            project_slice(
                &source,
                slice,
                binding(&source.context),
                request,
                "occupancy-task",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::TaskMismatch
        );
    }

    #[test]
    fn rejects_site_epoch_mismatch() {
        let source = report();
        let slice = &source.slices[0];
        let mut site = binding(&source.context);
        site.site_epoch = "stale-epoch".into();
        assert_eq!(
            project_slice(
                &source,
                slice,
                site,
                reservation(&source.context, slice),
                "occupancy-epoch",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::SiteEpochMismatch
        );
    }

    #[test]
    fn rejects_design_revision_mismatch() {
        let source = report();
        let slice = &source.slices[0];
        let mut site = binding(&source.context);
        site.design_revision_digest = "0".repeat(64);
        assert_eq!(
            project_slice(
                &source,
                slice,
                site,
                reservation(&source.context, slice),
                "occupancy-design",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::DesignRevisionMismatch
        );
    }

    #[test]
    fn rejects_frame_mismatch() {
        let source = report();
        let slice = &source.slices[0];
        let mut site = binding(&source.context);
        site.target_frame_id = "wrong-frame".into();
        assert_eq!(
            project_slice(
                &source,
                slice,
                site,
                reservation(&source.context, slice),
                "occupancy-frame",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::FrameMismatch
        );
    }

    #[test]
    fn rejects_clock_domain_mismatch() {
        let source = report();
        let slice = &source.slices[0];
        let mut request = reservation(&source.context, slice);
        request.clock_domain_id = "wrong-clock".into();
        assert_eq!(
            project_slice(
                &source,
                slice,
                binding(&source.context),
                request,
                "occupancy-clock",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::ClockDomainMismatch
        );
    }

    #[test]
    fn rejects_reservation_interval_mismatch() {
        let source = report();
        let slice = &source.slices[0];
        let mut request = reservation(&source.context, slice);
        request.interval_end_ms += 1;
        assert_eq!(
            project_slice(
                &source,
                slice,
                binding(&source.context),
                request,
                "occupancy-time",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::ReservationIntervalMismatch
        );
    }

    #[test]
    fn rejects_empty_occupancy_and_floor_ids() {
        let source = report();
        let slice = &source.slices[0];
        assert_eq!(
            project_slice(
                &source,
                slice,
                binding(&source.context),
                reservation(&source.context, slice),
                "  ",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::EmptyField("occupancy_id")
        );

        let mut site = binding(&source.context);
        site.floor_id.clear();
        assert_eq!(
            project_slice(
                &source,
                slice,
                site,
                reservation(&source.context, slice),
                "occupancy-floor",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::EmptyField("floor_id")
        );
    }

    #[test]
    fn rejects_non_nested_or_invalid_bounds_even_if_report_was_forged() {
        let mut source = report();
        source.slices[0].coordination_outer = source.slices[0].physical_swept_outer;
        let slice = source.slices[0].clone();
        assert_eq!(
            project_slice(
                &source,
                &slice,
                binding(&source.context),
                reservation(&source.context, &slice),
                "occupancy-bounds",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::OuterBoundDoesNotContain {
                outer: "coordination_outer",
                inner: "stopping_outer",
            }
        );
    }

    #[test]
    fn rejects_zero_units_and_malformed_context_digest() {
        let source = report();
        let slice = &source.slices[0];
        let mut request = reservation(&source.context, slice);
        request.units = 0;
        assert_eq!(
            project_slice(
                &source,
                slice,
                binding(&source.context),
                request,
                "occupancy-units",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::ZeroReservationUnits
        );

        let mut forged = source.clone();
        forged.context.tool_config_sha256 = "not-a-sha".into();
        assert_eq!(
            project_slice(
                &forged,
                &forged.slices[0],
                binding(&source.context),
                reservation(&source.context, &source.slices[0]),
                "occupancy-digest",
                OccupancyKind::RobotOperatingEnvelope,
            )
            .unwrap_err(),
            ProjectionError::InvalidDigest("tool_config_sha256")
        );
    }
}
