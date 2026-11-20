//! Conservative stop-tube geometry builder for a restricted, explicit kinematic domain.
//!
//! Supported model:
//! - every body/tool/payload primitive is rigidly translated by the same vector
//!   during each nominal time segment;
//! - the relative primitive offsets remain fixed across the complete trajectory;
//! - orientation is fixed and the braking direction is locked to the instantaneous
//!   nominal segment tangent after a trigger;
//! - the qualified stopping result supplies an upper bound on total path distance
//!   and elapsed stop time, including detection-to-brake latency.
//!
//! The builder refuses articulated relative motion and does not model steering,
//! rotation, trailer articulation, flexible loads, or a changing support polygon.
//! Its output is a conservative 4D outer-box cover: for every trigger instant in a
//! trigger window, the spatial box contains the full nominal-to-stop path and the
//! time interval contains the full trigger-to-stop interval.
//!
//! This is a research geometry producer, not a safety-rated stopping function.
//! The caller must verify source digests/signatures, the braking qualification,
//! clock mapping and the occupancy assessment before using this output.

use std::collections::{BTreeMap, BTreeSet};

use construction_motion_envelope::{
    AabbMm, AxisMm, Context, EvidenceRefs, MarginsMm, Point3Mm, Report, Segment, build,
};

use crate::stopping_reachability::{
    StoppingDynamicsQualification, StoppingReachabilityError, StoppingReachabilityRequest,
    StoppingReachabilityResult, calculate_stopping_reachability,
};

pub const STOP_TUBE_BUILD_SCHEMA_VERSION: &str = "construction-stop-tube-build/0.1";
pub const STOP_TUBE_BUILDER_MODEL: &str = "fixed-orientation-fixed-tangent-translation/0.1";
const NUMERIC_AXIS_GUARD_MM: u64 = 1;
const MAX_TRIGGER_WINDOWS_PER_SOURCE_SEGMENT: u64 = 65_536;
/// Keeps f64 direction normalization error well below the explicit 1 mm
/// outward projection guard. This still covers continental-scale site frames.
const MAX_NUMERIC_DIRECTION_COMPONENT_MM: i128 = 1_000_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopTubeKinematics {
    FixedOrientationFixedTangentNoSteering,
    ArticulatedOrRelativeMotion,
    SteeringDuringBraking,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StopTubeBuildRequest<'a> {
    pub nominal_report: &'a Report,
    /// SHA-256 of Report::canonical_bytes(), verified by the caller.
    pub nominal_report_sha256: String,
    /// Digest of the trajectory source, including its time parameterization.
    pub trajectory_sha256: String,
    /// Digest of the complete body/link/tool/payload primitive manifest.
    pub primitive_manifest_sha256: String,
    /// Digest of the site frame definition; the report carries the frame ID and transform.
    pub frame_sha256: String,
    /// Digest of the complete set of possible trigger times in the report clock domain.
    pub trigger_domain_sha256: String,
    /// Exact executable/build identity of this stop-tube builder.
    pub builder_build_sha256: String,
    pub stopping_qualification: &'a StoppingDynamicsQualification,
    pub stopping_result: &'a StoppingReachabilityResult,
    /// Maximum width of a trigger-time partition. Smaller windows reduce 4D
    /// bounding-box inflation but increase output size.
    pub maximum_trigger_window_ms: u64,
    pub now_tick: u64,
    pub requested_valid_until_tick: u64,
    pub kinematics: StopTubeKinematics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopTubeSlice {
    pub primitive_id: String,
    pub source_slice_index: u32,
    pub trigger_window_index: u32,
    /// Half-open interval of possible trigger instants, in the source site clock.
    pub trigger_start_ms: u64,
    pub trigger_end_ms: u64,
    /// Conservative union of every complete stop interval for triggers in the
    /// trigger window. This starts at trigger_start and ends after the last
    /// possible trigger plus the qualified maximum stop duration.
    pub occupancy_start_ms: u64,
    pub occupancy_end_ms: u64,
    /// Axis-aligned outer bound of the complete nominal-to-stop path for every
    /// trigger instant in the trigger window.
    pub bounds: AabbMm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopTubeBuildReport {
    pub schema_version: String,
    pub builder_model: String,
    pub builder_build_sha256: String,
    pub nominal_report_sha256: String,
    pub trajectory_sha256: String,
    pub primitive_manifest_sha256: String,
    pub frame_sha256: String,
    pub frame_id: String,
    pub transform_sha256: String,
    pub design_revision_sha256: String,
    pub site_epoch: String,
    pub robot_id: String,
    pub robot_configuration_sha256: String,
    pub tool_configuration_sha256: String,
    pub payload_configuration_sha256: String,
    pub collision_model_sha256: String,
    pub stopping_profile_sha256: String,
    pub stopping_qualification_receipt_sha256: String,
    pub robot_brake_configuration_sha256: String,
    pub operating_condition_sha256: String,
    pub scenario_context_sha256: String,
    pub site_clock_domain_id: String,
    pub trigger_domain_sha256: String,
    pub trigger_domain_start_ms: u64,
    pub trigger_domain_end_ms: u64,
    pub trigger_domain_duration_ms: u64,
    pub maximum_trigger_window_ms: u64,
    pub conservative_stop_distance_bound_mm: u64,
    pub conservative_stop_time_bound_ms: u64,
    pub stop_coverage_duration_ms: u64,
    pub primitive_count: u64,
    pub trigger_window_count: u64,
    pub slice_count: u64,
    pub trigger_domain_coverage_complete: bool,
    pub primitive_coverage_complete: bool,
    pub continuous_time_coverage_complete: bool,
    pub conservative_outer_bound: bool,
    pub valid_until_tick: u64,
    pub slices: Vec<StopTubeSlice>,
}

impl StopTubeBuildReport {
    /// Deterministic, versioned bytes. The caller computes SHA-256 and verifies
    /// the digest/signature against the exact bytes before publishing a receipt.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"CTSTB001".to_vec();
        for value in [
            self.schema_version.as_str(),
            self.builder_model.as_str(),
            self.builder_build_sha256.as_str(),
            self.nominal_report_sha256.as_str(),
            self.trajectory_sha256.as_str(),
            self.primitive_manifest_sha256.as_str(),
            self.frame_sha256.as_str(),
            self.frame_id.as_str(),
            self.transform_sha256.as_str(),
            self.design_revision_sha256.as_str(),
            self.site_epoch.as_str(),
            self.robot_id.as_str(),
            self.robot_configuration_sha256.as_str(),
            self.tool_configuration_sha256.as_str(),
            self.payload_configuration_sha256.as_str(),
            self.collision_model_sha256.as_str(),
            self.stopping_profile_sha256.as_str(),
            self.stopping_qualification_receipt_sha256.as_str(),
            self.robot_brake_configuration_sha256.as_str(),
            self.operating_condition_sha256.as_str(),
            self.scenario_context_sha256.as_str(),
            self.site_clock_domain_id.as_str(),
            self.trigger_domain_sha256.as_str(),
        ] {
            put_str(&mut out, value);
        }
        for value in [
            self.trigger_domain_start_ms,
            self.trigger_domain_end_ms,
            self.trigger_domain_duration_ms,
            self.maximum_trigger_window_ms,
            self.conservative_stop_distance_bound_mm,
            self.conservative_stop_time_bound_ms,
            self.stop_coverage_duration_ms,
            self.primitive_count,
            self.trigger_window_count,
            self.slice_count,
            self.valid_until_tick,
        ] {
            put_u64(&mut out, value);
        }
        for flag in [
            self.trigger_domain_coverage_complete,
            self.primitive_coverage_complete,
            self.continuous_time_coverage_complete,
            self.conservative_outer_bound,
        ] {
            out.push(u8::from(flag));
        }
        put_u64(&mut out, self.slices.len() as u64);
        for slice in &self.slices {
            put_str(&mut out, &slice.primitive_id);
            put_u32(&mut out, slice.source_slice_index);
            put_u32(&mut out, slice.trigger_window_index);
            put_u64(&mut out, slice.trigger_start_ms);
            put_u64(&mut out, slice.trigger_end_ms);
            put_u64(&mut out, slice.occupancy_start_ms);
            put_u64(&mut out, slice.occupancy_end_ms);
            for value in [
                slice.bounds.min.x,
                slice.bounds.min.y,
                slice.bounds.min.z,
                slice.bounds.max.x,
                slice.bounds.max.y,
                slice.bounds.max.z,
            ] {
                put_i64(&mut out, value);
            }
        }
        out
    }

    /// Canonical bytes binding the nominal envelope digest to this stop-tube
    /// report. The composite digest is computed by the caller.
    pub fn composite_envelope_bytes(&self, nominal_envelope_sha256: &str) -> Vec<u8> {
        let mut out = b"CTCOMP01".to_vec();
        put_str(&mut out, nominal_envelope_sha256);
        let stop_tube_bytes = self.canonical_bytes();
        put_u64(&mut out, stop_tube_bytes.len() as u64);
        out.extend_from_slice(&stop_tube_bytes);
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopTubeBuildError {
    InvalidDigest(&'static str),
    InvalidValidityInterval,
    ExpiredStoppingQualification,
    StoppingQualificationClockMismatch,
    StoppingResultDoesNotMatchQualification,
    StoppingReachability(StoppingReachabilityError),
    UnsupportedKinematics(StopTubeKinematics),
    InvalidTriggerWindow,
    TooManyTriggerWindows,
    EmptyNominalReport,
    InvalidNominalReport(String),
    NonCanonicalNominalReport,
    PrimitiveSetMismatch,
    RelativePrimitiveMotionUnsupported(String),
    BoundsOverflow,
    TimeArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TriggerWindow {
    index: u32,
    start_ms: u64,
    end_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Translation {
    x: i128,
    y: i128,
    z: i128,
}

/// Build a conservative stop tube for rigidly translating, fixed-orientation
/// primitive assemblies.
///
/// For every nominal source segment and trigger window [a,b), the generated
/// spatial box encloses:
///
///   nominal_position(t0) + lambda * tangent,  t0 in [a,b), lambda in [0,Dstop]
///
/// for every primitive, plus its radius and the source path/localization/tracking/
— model/numerical/stopping-reachability margins. The occupancy time interval is
/// [a, b + Tstop). The source policy-clearance margin is intentionally excluded:
/// the occupancy policy owner must apply it exactly once after the physical tube
/// is built. This output is conservative but can be inflated because it combines
/// all positions and all stop-path phases in one box per primitive/window.
pub fn build_stop_tube(
    request: StopTubeBuildRequest<'_>,
) -> Result<StopTubeBuildReport, StopTubeBuildError> {
    if request.kinematics != StopTubeKinematics::FixedOrientationFixedTangentNoSteering {
        return Err(StopTubeBuildError::UnsupportedKinematics(
            request.kinematics,
        ));
    }
    for (name, value) in [
        ("nominal_report_sha256", request.nominal_report_sha256.as_str()),
        ("trajectory_sha256", request.trajectory_sha256.as_str()),
        ("primitive_manifest_sha256", request.primitive_manifest_sha256.as_str()),
        ("frame_sha256", request.frame_sha256.as_str()),
        ("trigger_domain_sha256", request.trigger_domain_sha256.as_str()),
        ("builder_build_sha256", request.builder_build_sha256.as_str()),
    ] {
        if !is_sha256(value) {
            return Err(StopTubeBuildError::InvalidDigest(name));
        }
    }
    if request.maximum_trigger_window_ms == 0 {
        return Err(StopTubeBuildError::InvalidTriggerWindow);
    }
    if request.requested_valid_until_tick <= request.now_tick
        || request.requested_valid_until_tick > request.stopping_qualification.valid_until_tick
    {
        return Err(StopTubeBuildError::InvalidValidityInterval);
    }
    if request.stopping_qualification.valid_until_tick <= request.now_tick {
        return Err(StopTubeBuildError::ExpiredStoppingQualification);
    }

    let report = request.nominal_report;
    if report.slices.is_empty() || report.context.expected_primitive_ids.is_empty() {
        return Err(StopTubeBuildError::EmptyNominalReport);
    }
    if report.context.clock_domain_id != request.stopping_qualification.site_clock_domain_id {
        return Err(StopTubeBuildError::StoppingQualificationClockMismatch);
    }
    if report.context.frame_id.trim().is_empty()
        || report.context.robot_id.trim().is_empty()
        || report.context.transform_sha256.trim().is_empty()
    {
        return Err(StopTubeBuildError::InvalidNominalReport(
            "missing frame, robot or transform identity".into(),
        ));
    }

    let recomputed = calculate_stopping_reachability(
        request.stopping_qualification,
        StoppingReachabilityRequest {
            speed_upper_mm_s: request.stopping_result.speed_at_trigger_upper_mm_s,
        },
    )
    .map_err(StopTubeBuildError::StoppingReachability)?;
    if recomputed != *request.stopping_result {
        return Err(StopTubeBuildError::StoppingResultDoesNotMatchQualification);
    }

    validate_nominal_report(report)?;
    let groups = group_slices(report)?;
    validate_rigid_translation(&groups, &report.context.expected_primitive_ids)?;

    let trigger_domain_start_ms = report
        .slices
        .iter()
        .map(|slice| slice.start_ms)
        .min()
        .ok_or(StopTubeBuildError::EmptyNominalReport)?;
    let trigger_domain_end_ms = report
        .slices
        .iter()
        .map(|slice| slice.end_ms)
        .max()
        .ok_or(StopTubeBuildError::EmptyNominalReport)?;
    let trigger_domain_duration_ms = trigger_domain_end_ms
        .checked_sub(trigger_domain_start_ms)
        .filter(|duration| *duration > 0)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;

    let stop_distance = request.stopping_result.conservative_distance_bound_mm;
    let stop_time = request.stopping_result.conservative_stop_time_bound_ms;
    if stop_distance == 0 || stop_time == 0 {
        return Err(StopTubeBuildError::StoppingResultDoesNotMatchQualification);
    }
    let stop_coverage_duration_ms = trigger_domain_duration_ms
        .checked_add(stop_time)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
    let valid_until_tick = request
        .requested_valid_until_tick
        .min(request.stopping_qualification.valid_until_tick);

    let mut slices = Vec::new();
    let mut trigger_window_count = 0_u64;
    for (source_slice_index, group) in &groups {
        let first = group
            .first()
            .ok_or(StopTubeBuildError::EmptyNominalReport)?;
        let source_duration = first
            .end_ms
            .checked_sub(first.start_ms)
            .filter(|duration| *duration > 0)
            .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
        let window_count = div_ceil_u64(
            source_duration,
            request.maximum_trigger_window_ms,
        )?;
        if window_count > MAX_TRIGGER_WINDOWS_PER_SOURCE_SEGMENT {
            return Err(StopTubeBuildError::TooManyTriggerWindows);
        }
        trigger_window_count = trigger_window_count
            .checked_add(window_count)
            .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
        let windows = partition_trigger_interval(
            first.start_ms,
            first.end_ms,
            window_count,
        )?;
        let displacement = stop_displacement(first.from_mm, first.to_mm, stop_distance)?;

        for window in windows {
            let mut ordered_group: Vec<&construction_motion_envelope::EnvelopeSlice> =
                group.iter().copied().collect();
            ordered_group.sort_by(|left, right| left.primitive_id.cmp(&right.primitive_id));
            for primitive in ordered_group {
                let bounds = build_window_bounds(primitive, window, displacement)?;
                let occupancy_end_ms = window
                    .end_ms
                    .checked_add(stop_time)
                    .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
                slices.push(StopTubeSlice {
                    primitive_id: primitive.primitive_id.clone(),
                    source_slice_index: *source_slice_index,
                    trigger_window_index: window.index,
                    trigger_start_ms: window.start_ms,
                    trigger_end_ms: window.end_ms,
                    occupancy_start_ms: window.start_ms,
                    occupancy_end_ms,
                    bounds,
                });
            }
        }
    }

    let primitive_count = report.context.expected_primitive_ids.len() as u64;
    let expected_slice_count = primitive_count
        .checked_mul(trigger_window_count)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
    if slices.len() as u64 != expected_slice_count {
        return Err(StopTubeBuildError::PrimitiveSetMismatch);
    }
    slices.sort_by(|left, right| {
        (
            left.source_slice_index,
            left.trigger_window_index,
            left.primitive_id.as_str(),
        )
            .cmp(&(
                right.source_slice_index,
                right.trigger_window_index,
                right.primitive_id.as_str(),
            ))
    });

    Ok(StopTubeBuildReport {
        schema_version: STOP_TUBE_BUILD_SCHEMA_VERSION.into(),
        builder_model: STOP_TUBE_BUILDER_MODEL.into(),
        builder_build_sha256: request.builder_build_sha256,
        nominal_report_sha256: request.nominal_report_sha256,
        trajectory_sha256: request.trajectory_sha256,
        primitive_manifest_sha256: request.primitive_manifest_sha256,
        frame_sha256: request.frame_sha256,
        frame_id: report.context.frame_id.clone(),
        transform_sha256: report.context.transform_sha256.clone(),
        design_revision_sha256: report.context.design_revision_sha256.clone(),
        site_epoch: report.context.site_epoch.clone(),
        robot_id: report.context.robot_id.clone(),
        robot_configuration_sha256: report.context.platform_config_sha256.clone(),
        tool_configuration_sha256: report.context.tool_config_sha256.clone(),
        payload_configuration_sha256: report.context.payload_config_sha256.clone(),
        collision_model_sha256: report.context.collision_model_sha256.clone(),
        stopping_profile_sha256: request.stopping_qualification.profile_sha256.clone(),
        stopping_qualification_receipt_sha256: request
            .stopping_qualification
            .qualification_receipt_sha256
            .clone(),
        robot_brake_configuration_sha256: request
            .stopping_qualification
            .robot_brake_configuration_sha256
            .clone(),
        operating_condition_sha256: request
            .stopping_qualification
            .operating_condition_sha256
            .clone(),
        scenario_context_sha256: request
            .stopping_qualification
            .scenario_context_sha256
            .clone(),
        site_clock_domain_id: report.context.clock_domain_id.clone(),
        trigger_domain_sha256: request.trigger_domain_sha256,
        trigger_domain_start_ms,
        trigger_domain_end_ms,
        trigger_domain_duration_ms,
        maximum_trigger_window_ms: request.maximum_trigger_window_ms,
        conservative_stop_distance_bound_mm: stop_distance,
        conservative_stop_time_bound_ms: stop_time,
        stop_coverage_duration_ms,
        primitive_count,
        trigger_window_count,
        slice_count: slices.len() as u64,
        trigger_domain_coverage_complete: true,
        primitive_coverage_complete: true,
        continuous_time_coverage_complete: true,
        conservative_outer_bound: true,
        valid_until_tick,
        slices,
    })
}

fn validate_nominal_report(report: &Report) -> Result<(), StopTubeBuildError> {
    let input: Vec<Segment> = report
        .slices
        .iter()
        .map(|slice| Segment {
            slice_index: slice.slice_index,
            primitive_id: slice.primitive_id.clone(),
            start_ms: slice.start_ms,
            end_ms: slice.end_ms,
            from_mm: slice.from_mm,
            to_mm: slice.to_mm,
            radius_mm: slice.radius_mm,
            margins: slice.margins,
            evidence: slice.evidence.clone(),
        })
        .collect();
    let rebuilt = build(&report.context, &input)
        .map_err(|error| StopTubeBuildError::InvalidNominalReport(format!("{error:?}")))?;
    if rebuilt != *report {
        return Err(StopTubeBuildError::NonCanonicalNominalReport);
    }
    Ok(())
}

fn group_slices<'a>(
    report: &'a Report,
) -> Result<BTreeMap<u32, Vec<&'a construction_motion_envelope::EnvelopeSlice>>, StopTubeBuildError>
{
    let mut groups = BTreeMap::<u32, Vec<&construction_motion_envelope::EnvelopeSlice>>::new();
    for slice in &report.slices {
        groups.entry(slice.slice_index).or_default().push(slice);
    }
    let expected: BTreeSet<&str> = report
        .context
        .expected_primitive_ids
        .iter()
        .map(String::as_str)
        .collect();
    for group in groups.values_mut() {
        group.sort_by(|left, right| left.primitive_id.cmp(&right.primitive_id));
        let actual: BTreeSet<&str> = group
            .iter()
            .map(|slice| slice.primitive_id.as_str())
            .collect();
        if actual != expected {
            return Err(StopTubeBuildError::PrimitiveSetMismatch);
        }
        let first = group
            .first()
            .ok_or(StopTubeBuildError::EmptyNominalReport)?;
        if group.iter().any(|slice| {
            slice.start_ms != first.start_ms || slice.end_ms != first.end_ms
        }) {
            return Err(StopTubeBuildError::PrimitiveSetMismatch);
        }
    }
    Ok(groups)
}

fn validate_rigid_translation(
    groups: &BTreeMap<
        u32,
        Vec<&construction_motion_envelope::EnvelopeSlice>,
    >,
    expected_primitive_ids: &[String],
) -> Result<(), StopTubeBuildError> {
    let mut reference_offsets = BTreeMap::<&str, Translation>::new();
    let mut previous_end = None;
    for group in groups.values() {
        let first = group
            .first()
            .ok_or(StopTubeBuildError::EmptyNominalReport)?;
        if previous_end.is_some_and(|end| end != first.start_ms) {
            return Err(StopTubeBuildError::PrimitiveSetMismatch);
        }
        previous_end = Some(first.end_ms);
        for primitive in group {
            let delta = translation(primitive.from_mm, primitive.to_mm)?;
            let first_delta = translation(first.from_mm, first.to_mm)?;
            if delta != first_delta {
                return Err(StopTubeBuildError::RelativePrimitiveMotionUnsupported(
                    primitive.primitive_id.clone(),
                ));
            }
            let offset = translation(first.from_mm, primitive.from_mm)?;
            let end_offset = translation(first.to_mm, primitive.to_mm)?;
            if offset != end_offset {
                return Err(StopTubeBuildError::RelativePrimitiveMotionUnsupported(
                    primitive.primitive_id.clone(),
                ));
            }
            if let Some(previous) = reference_offsets.get(primitive.primitive_id.as_str()) {
                if *previous != offset {
                    return Err(StopTubeBuildError::RelativePrimitiveMotionUnsupported(
                        primitive.primitive_id.clone(),
                    ));
                }
            } else {
                reference_offsets.insert(primitive.primitive_id.as_str(), offset);
            }
        }
    }
    if reference_offsets.len() != expected_primitive_ids.len() {
        return Err(StopTubeBuildError::PrimitiveSetMismatch);
    }
    Ok(())
}

fn build_window_bounds(
    primitive: &construction_motion_envelope::EnvelopeSlice,
    window: TriggerWindow,
    displacement: [i64; 3],
) -> Result<AabbMm, StopTubeBuildError> {
    let duration = primitive
        .end_ms
        .checked_sub(primitive.start_ms)
        .filter(|value| *value > 0)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
    let start_offset = window
        .start_ms
        .checked_sub(primitive.start_ms)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
    let end_offset = window
        .end_ms
        .checked_sub(primitive.start_ms)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
    let (start_floor, start_ceil) =
        interpolate_bounds(primitive.from_mm, primitive.to_mm, start_offset, duration)?;
    let (end_floor, end_ceil) =
        interpolate_bounds(primitive.from_mm, primitive.to_mm, end_offset, duration)?;

    let margin = physical_margin_with_stop_margin(primitive.radius_mm, primitive.margins)?;
    let min_center = [
        start_floor.x.min(end_floor.x),
        start_floor.y.min(end_floor.y),
        start_floor.z.min(end_floor.z),
    ];
    let max_center = [
        start_ceil.x.max(end_ceil.x),
        start_ceil.y.max(end_ceil.y),
        start_ceil.z.max(end_ceil.z),
    ];
    let min = std::array::from_fn(|axis| {
        min_center[axis]
            .checked_add(displacement[axis].min(0))
            .and_then(|value| value.checked_sub(margin[axis]))
            .ok_or(StopTubeBuildError::BoundsOverflow)
    });
    let max = std::array::from_fn(|axis| {
        max_center[axis]
            .checked_add(displacement[axis].max(0))
            .and_then(|value| value.checked_add(margin[axis]))
            .ok_or(StopTubeBuildError::BoundsOverflow)
    });
    Ok(AabbMm {
        min: Point3Mm {
            x: min[0]?,
            y: min[1]?,
            z: min[2]?,
        },
        max: Point3Mm {
            x: max[0]?,
            y: max[1]?,
            z: max[2]?,
        },
    })
}

fn physical_margin_with_stop_margin(
    radius_mm: u64,
    margins: MarginsMm,
) -> Result<[i64; 3], StopTubeBuildError> {
    let values = [
        [
            radius_mm,
            margins.path_deviation.x,
            margins.localization.x,
            margins.tracking.x,
            margins.model_uncertainty.x,
            margins.numerical_enclosure.x,
            margins.stopping_reachability.x,
        ],
        [
            radius_mm,
            margins.path_deviation.y,
            margins.localization.y,
            margins.tracking.y,
            margins.model_uncertainty.y,
            margins.numerical_enclosure.y,
            margins.stopping_reachability.y,
        ],
        [
            radius_mm,
            margins.path_deviation.z,
            margins.localization.z,
            margins.tracking.z,
            margins.model_uncertainty.z,
            margins.numerical_enclosure.z,
            margins.stopping_reachability.z,
        ],
    ];
    let mut output = [0_i64; 3];
    for axis in 0..3 {
        let total = values[axis]
            .into_iter()
            .try_fold(0_u64, |sum, value| sum.checked_add(value))
            .ok_or(StopTubeBuildError::BoundsOverflow)?;
        output[axis] =
            i64::try_from(total).map_err(|_| StopTubeBuildError::BoundsOverflow)?;
    }
    Ok(output)
}

fn interpolate_bounds(
    from: Point3Mm,
    to: Point3Mm,
    offset: u64,
    duration: u64,
) -> Result<(Point3Mm, Point3Mm), StopTubeBuildError> {
    let from_values = [from.x, from.y, from.z];
    let to_values = [to.x, to.y, to.z];
    let mut floor = [0_i64; 3];
    let mut ceil = [0_i64; 3];
    for axis in 0..3 {
        let delta = i128::from(to_values[axis]) - i128::from(from_values[axis]);
        let numerator = delta
            .checked_mul(i128::from(offset))
            .ok_or(StopTubeBuildError::BoundsOverflow)?;
        let denominator = i128::from(duration);
        let quotient = numerator.div_euclid(denominator);
        let remainder = numerator.rem_euclid(denominator);
        let upper = if remainder == 0 {
            quotient
        } else {
            quotient
                .checked_add(1)
                .ok_or(StopTubeBuildError::BoundsOverflow)?
        };
        floor[axis] = i64::try_from(
            i128::from(from_values[axis])
                .checked_add(quotient)
                .ok_or(StopTubeBuildError::BoundsOverflow)?,
        )
        .map_err(|_| StopTubeBuildError::BoundsOverflow)?;
        ceil[axis] = i64::try_from(
            i128::from(from_values[axis])
                .checked_add(upper)
                .ok_or(StopTubeBuildError::BoundsOverflow)?,
        )
        .map_err(|_| StopTubeBuildError::BoundsOverflow)?;
    }
    Ok((
        Point3Mm {
            x: floor[0],
            y: floor[1],
            z: floor[2],
        },
        Point3Mm {
            x: ceil[0],
            y: ceil[1],
            z: ceil[2],
        },
    ))
}

fn stop_displacement(
    from: Point3Mm,
    to: Point3Mm,
    distance_mm: u64,
) -> Result<[i64; 3], StopTubeBuildError> {
    let delta = [
        i128::from(to.x) - i128::from(from.x),
        i128::from(to.y) - i128::from(from.y),
        i128::from(to.z) - i128::from(from.z),
    ];
    if delta.iter().any(|value| value.abs() > MAX_NUMERIC_DIRECTION_COMPONENT_MM)
        || i128::from(distance_mm) > MAX_NUMERIC_DIRECTION_COMPONENT_MM
    {
        return Err(StopTubeBuildError::BoundsOverflow);
    }
    let length_squared = delta[0]
        .checked_mul(delta[0])
        .and_then(|value| value.checked_add(delta[1].checked_mul(delta[1])?))
        .and_then(|value| value.checked_add(delta[2].checked_mul(delta[2])?))
        .ok_or(StopTubeBuildError::BoundsOverflow)?;
    if length_squared == 0 {
        return Ok([0, 0, 0]);
    }
    let length = (length_squared as f64).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return Err(StopTubeBuildError::BoundsOverflow);
    }
    let mut output = [0_i64; 3];
    for axis in 0..3 {
        if delta[axis] == 0 {
            output[axis] = 0;
            continue;
        }
        let projected =
            (distance_mm as f64) * (delta[axis] as f64).abs() / length;
        if !projected.is_finite() || projected < 0.0 {
            return Err(StopTubeBuildError::BoundsOverflow);
        }
        // Outward integer projection plus an explicit per-axis numerical guard.
        let rounded = projected.ceil() + NUMERIC_AXIS_GUARD_MM as f64;
        if rounded > i64::MAX as f64 {
            return Err(StopTubeBuildError::BoundsOverflow);
        }
        let magnitude = rounded as i64;
        output[axis] = if delta[axis] < 0 {
            magnitude
                .checked_neg()
                .ok_or(StopTubeBuildError::BoundsOverflow)?
        } else {
            magnitude
        };
    }
    Ok(output)
}

fn translation(from: Point3Mm, to: Point3Mm) -> Result<Translation, StopTubeBuildError> {
    Ok(Translation {
        x: i128::from(to.x) - i128::from(from.x),
        y: i128::from(to.y) - i128::from(from.y),
        z: i128::from(to.z) - i128::from(from.z),
    })
}

fn partition_trigger_interval(
    start_ms: u64,
    end_ms: u64,
    count: u64,
) -> Result<Vec<TriggerWindow>, StopTubeBuildError> {
    if count == 0 {
        return Err(StopTubeBuildError::InvalidTriggerWindow);
    }
    let duration = end_ms
        .checked_sub(start_ms)
        .filter(|value| *value > 0)
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
    let mut output = Vec::with_capacity(
        usize::try_from(count).map_err(|_| StopTubeBuildError::TooManyTriggerWindows)?,
    );
    for index in 0..count {
        let start_offset = (u128::from(duration) * u128::from(index)) / u128::from(count);
        let end_offset =
            (u128::from(duration) * u128::from(index + 1)) / u128::from(count);
        let window_start = start_ms
            .checked_add(
                u64::try_from(start_offset).map_err(|_| StopTubeBuildError::TimeArithmeticOverflow)?,
            )
            .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
        let window_end = start_ms
            .checked_add(
                u64::try_from(end_offset).map_err(|_| StopTubeBuildError::TimeArithmeticOverflow)?,
            )
            .ok_or(StopTubeBuildError::TimeArithmeticOverflow)?;
        if window_start >= window_end {
            return Err(StopTubeBuildError::InvalidTriggerWindow);
        }
        output.push(TriggerWindow {
            index: u32::try_from(index).map_err(|_| StopTubeBuildError::TooManyTriggerWindows)?,
            start_ms: window_start,
            end_ms: window_end,
        });
    }
    Ok(output)
}

fn div_ceil_u64(value: u64, divisor: u64) -> Result<u64, StopTubeBuildError> {
    if divisor == 0 {
        return Err(StopTubeBuildError::InvalidTriggerWindow);
    }
    let quotient = value / divisor;
    let remainder = value % divisor;
    quotient
        .checked_add(u64::from(remainder != 0))
        .ok_or(StopTubeBuildError::TimeArithmeticOverflow)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn put_str(output: &mut Vec<u8>, value: &str) {
    put_u64(output, value.len() as u64);
    output.extend_from_slice(value.as_bytes());
}

fn put_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn put_i64(output: &mut Vec<u8>, value: i64) {
    output.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stopping_reachability::StoppingReachabilityResult;

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn margins() -> MarginsMm {
        MarginsMm {
            path_deviation: AxisMm { x: 2, y: 3, z: 4 },
            localization: AxisMm { x: 5, y: 6, z: 7 },
            tracking: AxisMm { x: 1, y: 1, z: 1 },
            model_uncertainty: AxisMm { x: 1, y: 2, z: 3 },
            numerical_enclosure: AxisMm { x: 1, y: 1, z: 1 },
            stopping_reachability: AxisMm { x: 10, y: 11, z: 12 },
            policy_clearance: AxisMm { x: 100, y: 100, z: 100 },
        }
    }

    fn evidence() -> EvidenceRefs {
        EvidenceRefs {
            path_deviation: "path".into(),
            localization: "localization".into(),
            tracking: "tracking".into(),
            model_uncertainty: "model".into(),
            numerical_enclosure: "numeric".into(),
            stopping_reachability: "stopping".into(),
            policy_clearance: "policy".into(),
        }
    }

    fn context() -> Context {
        Context {
            project_id: "project-1".into(),
            work_package_id: "wp-1".into(),
            task_id: "task-1".into(),
            trajectory_id: "trajectory-1".into(),
            robot_id: "robot-1".into(),
            platform_config_sha256: digest('1'),
            tool_config_sha256: digest('2'),
            payload_config_sha256: digest('3'),
            collision_model_sha256: digest('4'),
            primitive_set_ref: "manifest://robot-1".into(),
            expected_primitive_ids: vec!["base".into(), "tool".into()],
            calibration_ref: "calibration".into(),
            controller_profile_ref: "controller".into(),
            frame_id: "site-frame".into(),
            transform_sha256: digest('5'),
            design_revision_sha256: digest('6'),
            site_epoch: "site-epoch-1".into(),
            clock_domain_id: "site-clock-1".into(),
            trajectory_profile_sha256: digest('7'),
        }
    }

    fn source_report(
        start: [i64; 3],
        end: [i64; 3],
        start_ms: u64,
        end_ms: u64,
    ) -> Report {
        let base_from = Point3Mm {
            x: start[0],
            y: start[1],
            z: start[2],
        };
        let base_to = Point3Mm {
            x: end[0],
            y: end[1],
            z: end[2],
        };
        let tool_from = Point3Mm {
            x: start[0] + 200,
            y: start[1] + 50,
            z: start[2] + 25,
        };
        let tool_to = Point3Mm {
            x: end[0] + 200,
            y: end[1] + 50,
            z: end[2] + 25,
        };
        let segments = vec![
            Segment {
                slice_index: 0,
                primitive_id: "base".into(),
                start_ms,
                end_ms,
                from_mm: base_from,
                to_mm: base_to,
                radius_mm: 100,
                margins: margins(),
                evidence: evidence(),
            },
            Segment {
                slice_index: 0,
                primitive_id: "tool".into(),
                start_ms,
                end_ms,
                from_mm: tool_from,
                to_mm: tool_to,
                radius_mm: 50,
                margins: margins(),
                evidence: evidence(),
            },
        ];
        build(&context(), &segments).unwrap()
    }

    fn qualification() -> StoppingDynamicsQualification {
        StoppingDynamicsQualification {
            profile_sha256: digest('8'),
            qualification_receipt_sha256: digest('9'),
            robot_brake_configuration_sha256: digest('a'),
            operating_condition_sha256: digest('b'),
            scenario_context_sha256: digest('c'),
            site_clock_domain_id: "site-clock-1".into(),
            valid_until_tick: 20_000,
            maximum_qualified_speed_mm_s: 2_000.0,
            maximum_detection_to_brake_latency_ms: 100,
            maximum_positive_acceleration_mm_s2: 0.0,
            minimum_net_deceleration_mm_s2: 3_000.0,
            minimum_braking_jerk_mm_s3: 10_000.0,
            additional_distance_guard_mm: 20.0,
        }
    }

    fn request<'a>(report: &'a Report) -> (StoppingDynamicsQualification, StoppingReachabilityResult, StopTubeBuildRequest<'a>) {
        let qualification = qualification();
        let stopping = calculate_stopping_reachability(
            &qualification,
            StoppingReachabilityRequest {
                speed_upper_mm_s: 1_000.0,
            },
        )
        .unwrap();
        let built = StopTubeBuildRequest {
            nominal_report: report,
            nominal_report_sha256: digest('d'),
            trajectory_sha256: digest('e'),
            primitive_manifest_sha256: digest('f'),
            frame_sha256: digest('1'),
            trigger_domain_sha256: digest('2'),
            builder_build_sha256: digest('3'),
            stopping_qualification: &qualification,
            stopping_result: &stopping,
            maximum_trigger_window_ms: 250,
            now_tick: 100,
            requested_valid_until_tick: 10_000,
            kinematics: StopTubeKinematics::FixedOrientationFixedTangentNoSteering,
        };
        (qualification, stopping, built)
    }

    #[test]
    fn straight_translation_covers_every_primitive_and_the_complete_stop_distance() {
        let report = source_report([0, 0, 0], [1_000, 0, 0], 1_000, 2_000);
        let (qualification, stopping, request) = request(&report);
        let output = build_stop_tube(request).unwrap();
        assert_eq!(output.primitive_count, 2);
        assert_eq!(output.trigger_window_count, 4);
        assert_eq!(output.slice_count, 8);
        assert!(output.trigger_domain_coverage_complete);
        assert!(output.primitive_coverage_complete);
        assert!(output.continuous_time_coverage_complete);
        assert!(output.conservative_outer_bound);
        assert_eq!(output.trigger_domain_start_ms, 1_000);
        assert_eq!(output.trigger_domain_end_ms, 2_000);
        assert_eq!(
            output.stop_coverage_duration_ms,
            1_000 + stopping.conservative_stop_time_bound_ms
        );
        assert_eq!(output.valid_until_tick, 10_000.min(qualification.valid_until_tick));
        let last_base = output
            .slices
            .iter()
            .find(|slice| slice.primitive_id == "base" && slice.trigger_window_index == 3)
            .unwrap();
        assert_eq!(last_base.bounds.max.x, 1_000 + stopping.conservative_distance_bound_mm as i64 + 1 + 100 + 2 + 5 + 1 + 1 + 10);
        assert_eq!(last_base.occupancy_end_ms, 2_000 + stopping.conservative_stop_time_bound_ms);
    }

    #[test]
    fn negative_diagonal_translation_expands_the_stop_tube_in_both_axes() {
        let report = source_report([1_000, 1_000, 0], [0, 0, 0], 0, 1_000);
        let (_, stopping, request) = request(&report);
        let output = build_stop_tube(request).unwrap();
        let base = output
            .slices
            .iter()
            .filter(|slice| slice.primitive_id == "base")
            .last()
            .unwrap();
        assert!(base.bounds.min.x < 0);
        assert!(base.bounds.min.y < 0);
        assert!(base.bounds.max.x >= 1_000);
        assert!(base.bounds.max.y >= 1_000);
        assert!(base.bounds.min.x <= -(stopping.conservative_distance_bound_mm as i64 / 2));
    }

    #[test]
    fn articulated_or_steering_during_braking_models_are_explicitly_rejected() {
        let report = source_report([0, 0, 0], [1_000, 0, 0], 0, 1_000);
        let (_, stopping, mut request) = request(&report);
        request.stopping_result = &stopping;
        request.kinematics = StopTubeKinematics::ArticulatedOrRelativeMotion;
        assert_eq!(
            build_stop_tube(request.clone()),
            Err(StopTubeBuildError::UnsupportedKinematics(
                StopTubeKinematics::ArticulatedOrRelativeMotion
            ))
        );
        request.kinematics = StopTubeKinematics::SteeringDuringBraking;
        assert_eq!(
            build_stop_tube(request),
            Err(StopTubeBuildError::UnsupportedKinematics(
                StopTubeKinematics::SteeringDuringBraking
            ))
        );
    }

    #[test]
    fn mismatched_primitive_translation_is_rejected() {
        let mut report = source_report([0, 0, 0], [1_000, 0, 0], 0, 1_000);
        let tool = report
            .slices
            .iter_mut()
            .find(|slice| slice.primitive_id == "tool")
            .unwrap();
        tool.to_mm.y += 10;
        let (_, _, request) = request(&report);
        assert!(matches!(
            build_stop_tube(request),
            Err(StopTubeBuildError::NonCanonicalNominalReport)
                | Err(StopTubeBuildError::RelativePrimitiveMotionUnsupported(_))
        ));
    }

    #[test]
    fn stale_or_cross_clock_stopping_qualification_is_rejected() {
        let report = source_report([0, 0, 0], [1_000, 0, 0], 0, 1_000);
        let (mut qualification, stopping, mut request) = request(&report);
        qualification.site_clock_domain_id = "other-clock".into();
        request.stopping_qualification = &qualification;
        assert_eq!(
            build_stop_tube(request.clone()),
            Err(StopTubeBuildError::StoppingQualificationClockMismatch)
        );
        qualification.site_clock_domain_id = "site-clock-1".into();
        qualification.valid_until_tick = 50;
        request.stopping_qualification = &qualification;
        request.stopping_result = &stopping;
        assert_eq!(
            build_stop_tube(request),
            Err(StopTubeBuildError::ExpiredStoppingQualification)
        );
    }

    #[test]
    fn a_tampered_stopping_result_cannot_be_used_to_shrink_the_tube() {
        let report = source_report([0, 0, 0], [1_000, 0, 0], 0, 1_000);
        let (qualification, stopping, mut request) = request(&report);
        let mut tampered = stopping;
        tampered.conservative_distance_bound_mm /= 2;
        request.stopping_qualification = &qualification;
        request.stopping_result = &tampered;
        assert_eq!(
            build_stop_tube(request),
            Err(StopTubeBuildError::StoppingResultDoesNotMatchQualification)
        );
    }

    #[test]
    fn invalid_digest_or_trigger_window_is_rejected() {
        let report = source_report([0, 0, 0], [1_000, 0, 0], 0, 1_000);
        let (_, _, mut request) = request(&report);
        request.trigger_domain_sha256 = "not-a-digest".into();
        assert_eq!(
            build_stop_tube(request.clone()),
            Err(StopTubeBuildError::InvalidDigest("trigger_domain_sha256"))
        );
        request.trigger_domain_sha256 = digest('2');
        request.maximum_trigger_window_ms = 0;
        assert_eq!(
            build_stop_tube(request),
            Err(StopTubeBuildError::InvalidTriggerWindow)
        );
    }

    #[test]
    fn overflow_in_stop_time_or_bounds_fails_closed() {
        let report = source_report([i64::MAX - 1_000, 0, 0], [i64::MAX - 1, 0, 0], u64::MAX - 1_000, u64::MAX);
        let (_, _, request) = request(&report);
        assert!(matches!(
            build_stop_tube(request),
            Err(StopTubeBuildError::TimeArithmeticOverflow)
                | Err(StopTubeBuildError::BoundsOverflow)
        ));
    }

    #[test]
    fn canonical_output_is_deterministic_and_binds_all_source_identities() {
        let report = source_report([0, 0, 0], [1_000, 250, 0], 100, 900);
        let (_, _, request) = request(&report);
        let first = build_stop_tube(request.clone()).unwrap();
        let second = build_stop_tube(request).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert!(first.canonical_bytes().starts_with(b"CTSTB001"));
        assert_eq!(
            first.composite_envelope_bytes(&digest('4')),
            second.composite_envelope_bytes(&digest('4'))
        );
        assert_eq!(first.builder_build_sha256, digest('3'));
        assert_eq!(first.robot_id, "robot-1");
        assert_eq!(first.frame_id, "site-frame");
        assert_eq!(first.site_clock_domain_id, "site-clock-1");
    }

    #[test]
    fn independent_vertex_oracle_contains_rigid_translation_stop_parallelogram() {
        for (start, end) in [
            ([0, 0, 0], [1_000, 0, 0]),
            ([1_000, 0, 0], [0, 0, 0]),
            ([0, 0, 0], [0, 1_000, 0]),
            ([0, 0, 0], [0, 0, 1_000]),
            ([500, -200, 100], [-500, 800, -300]),
            ([-400, 300, 50], [600, -700, 450]),
        ] {
            let report = source_report(start, end, 0, 1_000);
            let (_, stopping, request) = request(&report);
            let built = build_stop_tube(request).unwrap();
            let slice = built
                .slices
                .iter()
                .find(|slice| slice.primitive_id == "base" && slice.trigger_window_index == 3)
                .unwrap();
            let delta = [
                (end[0] - start[0]) as f64,
                (end[1] - start[1]) as f64,
                (end[2] - start[2]) as f64,
            ];
            let length = (delta[0] * delta[0]
                + delta[1] * delta[1]
                + delta[2] * delta[2])
                .sqrt();
            let stop = stopping.conservative_distance_bound_mm as f64;
            let margin = [120.0, 124.0, 128.0];
            let min = [slice.bounds.min.x, slice.bounds.min.y, slice.bounds.min.z];
            let max = [slice.bounds.max.x, slice.bounds.max.y, slice.bounds.max.z];
            for axis in 0..3 {
                let projected = if length == 0.0 {
                    0.0
                } else {
                    stop * delta[axis] / length
                };
                // Independent vertex oracle for the Minkowski sum of the
                // trigger-position segment and the fixed-tangent stop segment.
                let vertices = [
                    start[axis] as f64,
                    end[axis] as f64,
                    start[axis] as f64 + projected,
                    end[axis] as f64 + projected,
                ];
                let oracle_min = vertices.iter().copied().fold(f64::INFINITY, f64::min)
                    - margin[axis];
                let oracle_max = vertices
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
                    + margin[axis];
                assert!((min[axis] as f64) <= oracle_min);
                assert!((max[axis] as f64) >= oracle_max);
            }
        }
    }
}
