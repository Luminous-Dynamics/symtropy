//! Integrated, dependency-current and stop-aware construction dispatch readiness.
//!
//! This module composes three independently maintained reference owners:
//! - the site ledger and space-time occupancy readiness gate;
//! - dependency-scoped evidence currentness;
//! - independently qualified braking and hazard-detection distance budgets.
//!
//! It does not issue an actuation permit. The caller must verify canonical
//! digests and signatures for every upstream receipt and must hand the complete
//! readiness receipt to the applicable external authorization authority.

use crate::dependency_scoped_invalidation::{
    DependencyGraphError, EvidenceDependencyGraph, EvidenceNodeKind, EvidenceSnapshot,
};
use crate::dispatch_readiness::{
    DispatchReadinessFailure, DispatchReadinessReceipt, DispatchReadinessRequest,
    check_dispatch_readiness, verify_dispatch_readiness_receipt_current,
};
use crate::site_state_ledger::{
    SiteActorRole, SiteEventEnvelope, SiteEventPayload, SiteStateLedger,
};
use crate::stopping_reachability::{
    HazardDetectionQualification, HazardStoppingBudget, HazardStoppingMargins,
    StoppingDynamicsQualification, StoppingReachabilityError, StoppingReachabilityRequest,
    calculate_stopping_reachability, evaluate_hazard_stopping_budget,
};

pub const INTEGRATED_DISPATCH_REQUEST_SCHEMA_VERSION: &str =
    "construction-integrated-dispatch-readiness-request/0.1";
pub const INTEGRATED_DISPATCH_RECEIPT_SCHEMA_VERSION: &str =
    "construction-integrated-dispatch-readiness-receipt/0.1";
pub const STOP_TUBE_ENVELOPE_SCHEMA_VERSION: &str =
    "construction-stop-tube-envelope-receipt/0.1";

/// Names the exact dependency nodes whose versions must appear in the
/// dispatch-readiness snapshot. The graph may contain additional nodes, but it
/// may not omit any binding below. The required version for each node is
/// derived from the work package, envelope, station receipt and physical
/// qualification records rather than supplied as a second unchecked value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegratedEvidenceNodeIds {
    pub dispatch_readiness: String,
    pub work_package: String,
    pub source_model: String,
    pub ids_specification: String,
    pub ids_engine_build: String,
    pub ids_evaluation_manifest: String,
    pub coordinate_context: String,
    pub coordinate_frame: String,
    pub survey_transform: String,
    pub station_qualification: String,
    pub task_plan: String,
    pub robot_configuration: String,
    pub tool_configuration: String,
    pub payload_configuration: String,
    pub motion_envelope: String,
    pub stopping_tube_envelope: String,
    pub stopping_tube_builder_build: String,
    pub sensor_configuration: String,
    pub stopping_dynamics: String,
    pub hazard_detection: String,
    pub operating_condition: String,
    pub occupancy_engine_build: String,
    pub clearance_policy: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopTubeEnvelopeReceipt {
    pub schema_version: String,
    /// Digest of the complete swept union of all stop trajectories reachable
    /// from every trigger point in the frozen nominal trajectory domain.
    pub stop_tube_sha256: String,
    /// Canonical, independently verifiable receipt digest from the envelope builder.
    pub stop_tube_receipt_sha256: String,
    pub stop_tube_builder_build_sha256: String,
    /// Composite envelope assessed by the occupancy engine: nominal motion
    /// envelope plus the complete stopping tube.
    pub composite_envelope_sha256: String,
    pub nominal_trajectory_sha256: String,
    pub trigger_domain_sha256: String,
    pub primitive_manifest_sha256: String,
    pub robot_configuration_sha256: String,
    pub tool_configuration_sha256: String,
    pub payload_configuration_sha256: String,
    pub frame_sha256: String,
    pub transform_sha256: String,
    pub stopping_profile_sha256: String,
    pub stopping_qualification_receipt_sha256: String,
    pub operating_condition_sha256: String,
    pub scenario_context_sha256: String,
    pub site_clock_domain_id: String,
    /// Trigger domain is the complete time interval in which the planned
    /// trajectory could detect a hazard and initiate a stop.
    pub trigger_domain_start_tick: u64,
    pub trigger_domain_end_tick: u64,
    /// Builder-declared time coverage in milliseconds for the union of all
    /// stop trajectories after each possible trigger point.
    pub stop_coverage_duration_ms: u64,
    pub conservative_stop_distance_bound_mm: u64,
    pub primitive_count: u64,
    pub trigger_domain_coverage_complete: bool,
    pub primitive_coverage_complete: bool,
    pub continuous_time_coverage_complete: bool,
    pub conservative_outer_bound: bool,
    pub valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedDispatchReadinessRequest {
    pub schema_version: String,
    pub dispatch: DispatchReadinessRequest,
    pub evidence_snapshot: EvidenceSnapshot,
    /// SHA-256 of the canonical serialized EvidenceSnapshot. The caller must
    /// recompute it and verify the source signature before using this request.
    pub evidence_snapshot_sha256: String,
    /// SHA-256 of the complete canonical execution context: package, task plan,
    /// frame, station qualification, robot/tool/payload, motion envelope,
    /// sensor, brake, detection, operating condition, occupancy and policy.
    pub dispatch_context_sha256: String,
    pub evidence_nodes: IntegratedEvidenceNodeIds,
    pub task_plan_sha256: String,
    pub station_id: String,
    pub station_qualification_event_id: String,
    pub speed_upper_mm_s: f64,
    pub stopping_dynamics: StoppingDynamicsQualification,
    pub stopping_envelope: StopTubeEnvelopeReceipt,
    pub hazard_detection: HazardDetectionQualification,
    pub stopping_margins: HazardStoppingMargins,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedDispatchReadinessReceipt {
    pub schema_version: String,
    pub dispatch_readiness: DispatchReadinessReceipt,
    pub evidence_snapshot_sha256: String,
    pub dispatch_context_sha256: String,
    pub work_package_sha256: String,
    pub station_id: String,
    pub station_qualification_event_id: String,
    pub station_qualification_event_sha256: String,
    pub task_plan_sha256: String,
    pub stopping_profile_sha256: String,
    pub stopping_qualification_receipt_sha256: String,
    pub robot_brake_configuration_sha256: String,
    pub operating_condition_sha256: String,
    pub sensor_profile_sha256: String,
    pub detection_qualification_receipt_sha256: String,
    pub target_condition_sha256: String,
    pub scenario_context_sha256: String,
    pub speed_upper_mm_s: f64,
    pub conservative_stop_distance_mm: u64,
    pub conservative_stop_time_ms: u64,
    pub stopping_tube_sha256: String,
    pub stopping_tube_receipt_sha256: String,
    pub stopping_tube_builder_build_sha256: String,
    pub stopping_tube_coverage_duration_ms: u64,
    pub minimum_detection_distance_mm: f64,
    pub required_stopping_distance_mm: u64,
    pub remaining_stopping_distance_mm: f64,
    pub stopping_distance_budget_satisfied: bool,
    pub checked_at_tick: u64,
    pub valid_until_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegratedDispatchReadinessFailure {
    UnsupportedSchemaVersion,
    InvalidDigest(&'static str),
    UnknownWorkPackage,
    WorkPackageContextMismatch,
    EvidenceSnapshotSubjectMismatch,
    EvidenceSnapshotNotCurrent,
    EvidenceGraph(DependencyGraphError),
    MissingEvidenceNode(String),
    EvidenceNodeKindMismatch {
        node_id: String,
        expected: EvidenceNodeKind,
        actual: EvidenceNodeKind,
    },
    EvidenceNodeVersionMismatch(String),
    StationIdentityMissing,
    StationQualificationMissing,
    StationQualificationNotLatest,
    StationQualificationContextMismatch,
    StationQualificationClockDomainMismatch,
    StationQualificationExpired,
    StoppingEvidenceClockDomainMismatch,
    DetectionEvidenceClockDomainMismatch,
    TaskPlanBindingMismatch,
    RobotBrakeConfigurationMismatch,
    StoppingEvidenceExpired,
    DetectionEvidenceExpired,
    InvalidStoppingEnvelopeDigest(&'static str),
    StoppingEnvelopeContextMismatch(&'static str),
    StoppingEnvelopeExpired,
    StoppingEnvelopeBoundTooSmall,
    StoppingEnvelopeCoverageIncomplete,
    OccupancyDidNotAssessStoppingEnvelope,
    StoppingBudgetExceeded {
        required_distance_mm: u64,
        available_distance_mm: f64,
    },
    StoppingQualification(StoppingReachabilityError),
    MotionReadiness(DispatchReadinessFailure),
    IntegratedReceiptExpired,
    IntegratedReceiptContextMismatch,
}

/// Require a full dependency snapshot, a current survey qualification and a
/// paired, qualified stopping/detection budget before the underlying occupancy
/// readiness result can be promoted to integrated readiness.
pub fn check_integrated_dispatch_readiness(
    ledger: &SiteStateLedger,
    graph: &EvidenceDependencyGraph,
    request: &IntegratedDispatchReadinessRequest,
) -> Result<IntegratedDispatchReadinessReceipt, IntegratedDispatchReadinessFailure> {
    if request.schema_version != INTEGRATED_DISPATCH_REQUEST_SCHEMA_VERSION {
        return Err(IntegratedDispatchReadinessFailure::UnsupportedSchemaVersion);
    }
    for (label, digest) in [
        ("evidence snapshot", request.evidence_snapshot_sha256.as_str()),
        ("dispatch context", request.dispatch_context_sha256.as_str()),
        ("task plan", request.task_plan_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(IntegratedDispatchReadinessFailure::InvalidDigest(label));
        }
    }
    if request.station_id.trim().is_empty()
        || request.station_qualification_event_id.trim().is_empty()
    {
        return Err(IntegratedDispatchReadinessFailure::StationIdentityMissing);
    }

    let package_sha256 = request.dispatch.envelope.package_sha256.as_str();
    if ledger.active_package_sha256() != Some(package_sha256)
        || ledger.package_is_current(package_sha256) != Some(true)
    {
        return Err(IntegratedDispatchReadinessFailure::UnknownWorkPackage);
    }
    let package = ledger
        .work_package(package_sha256)
        .ok_or(IntegratedDispatchReadinessFailure::UnknownWorkPackage)?;

    let station_event = latest_station_qualification(
        ledger,
        package_sha256,
        &request.station_id,
    )?;
    if station_event.event_id != request.station_qualification_event_id {
        return Err(IntegratedDispatchReadinessFailure::StationQualificationNotLatest);
    }
    let station_event_sha256 = station_event.event_sha256.clone();

    if request.dispatch.envelope.frame_sha256 != package.frame_sha256
        || request.dispatch.envelope.transform_sha256 != package.transform_sha256
    {
        return Err(IntegratedDispatchReadinessFailure::StationQualificationContextMismatch);
    }
    let now_tick = request.dispatch.now_tick;
    let station_valid_until_tick = if let SiteEventPayload::StationQualified {
        frame_sha256,
        transform_sha256,
        site_clock_domain_id,
        valid_until_tick,
        ..
    } = &station_event.payload
    {
        if frame_sha256 != &package.frame_sha256
            || transform_sha256 != &package.transform_sha256
        {
            return Err(IntegratedDispatchReadinessFailure::StationQualificationContextMismatch);
        }
        if site_clock_domain_id != &request.dispatch.envelope.site_clock_domain_id {
            return Err(IntegratedDispatchReadinessFailure::StationQualificationClockDomainMismatch);
        }
        *valid_until_tick
    } else {
        return Err(IntegratedDispatchReadinessFailure::StationQualificationMissing);
    };

    if station_valid_until_tick <= now_tick {
        return Err(IntegratedDispatchReadinessFailure::StationQualificationExpired);
    }
    if request.stopping_dynamics.site_clock_domain_id
        != request.dispatch.envelope.site_clock_domain_id
    {
        return Err(IntegratedDispatchReadinessFailure::StoppingEvidenceClockDomainMismatch);
    }
    if request.hazard_detection.site_clock_domain_id
        != request.dispatch.envelope.site_clock_domain_id
    {
        return Err(IntegratedDispatchReadinessFailure::DetectionEvidenceClockDomainMismatch);
    }
    if request.stopping_dynamics.valid_until_tick <= now_tick {
        return Err(IntegratedDispatchReadinessFailure::StoppingEvidenceExpired);
    }
    if request.hazard_detection.valid_until_tick <= now_tick {
        return Err(IntegratedDispatchReadinessFailure::DetectionEvidenceExpired);
    }

    if request.stopping_dynamics.robot_brake_configuration_sha256
        != request.dispatch.envelope.robot_configuration_sha256
    {
        return Err(IntegratedDispatchReadinessFailure::RobotBrakeConfigurationMismatch);
    }

    validate_dependency_snapshot(
        graph,
        &request.evidence_snapshot,
        request,
        package,
        &station_event_sha256,
    )?;

    let stopping = calculate_stopping_reachability(
        &request.stopping_dynamics,
        StoppingReachabilityRequest {
            speed_upper_mm_s: request.speed_upper_mm_s,
        },
    )
    .map_err(IntegratedDispatchReadinessFailure::StoppingQualification)?;
    let stopping_budget = evaluate_hazard_stopping_budget(
        stopping,
        &request.hazard_detection,
        request.stopping_margins,
    )
    .map_err(IntegratedDispatchReadinessFailure::StoppingQualification)?;
    validate_stopping_envelope_binding(request, stopping)?;

    if !stopping_budget.distance_budget_satisfied {
        return Err(IntegratedDispatchReadinessFailure::StoppingBudgetExceeded {
            required_distance_mm: stopping_budget.required_distance_bound_mm,
            available_distance_mm: stopping_budget.minimum_qualified_detection_distance_mm,
        });
    }

    let dispatch_readiness = check_dispatch_readiness(ledger, &request.dispatch)
        .map_err(IntegratedDispatchReadinessFailure::MotionReadiness)?;

    let checked_at_tick = now_tick;
    let valid_until_tick = dispatch_readiness
        .valid_until_tick
        .min(station_valid_until_tick)
        .min(request.stopping_dynamics.valid_until_tick)
        .min(request.hazard_detection.valid_until_tick)
        .min(request.stopping_envelope.valid_until_tick);

    Ok(IntegratedDispatchReadinessReceipt {
        schema_version: INTEGRATED_DISPATCH_RECEIPT_SCHEMA_VERSION.to_owned(),
        dispatch_readiness,
        evidence_snapshot_sha256: request.evidence_snapshot_sha256.clone(),
        dispatch_context_sha256: request.dispatch_context_sha256.clone(),
        work_package_sha256: package.package_sha256.clone(),
        station_id: request.station_id.clone(),
        station_qualification_event_id: station_event.event_id.clone(),
        station_qualification_event_sha256: station_event_sha256,
        task_plan_sha256: request.task_plan_sha256.clone(),
        stopping_profile_sha256: request.stopping_dynamics.profile_sha256.clone(),
        stopping_qualification_receipt_sha256: request
            .stopping_dynamics
            .qualification_receipt_sha256
            .clone(),
        robot_brake_configuration_sha256: request
            .stopping_dynamics
            .robot_brake_configuration_sha256
            .clone(),
        operating_condition_sha256: request
            .stopping_dynamics
            .operating_condition_sha256
            .clone(),
        sensor_profile_sha256: request.hazard_detection.sensor_profile_sha256.clone(),
        detection_qualification_receipt_sha256: request
            .hazard_detection
            .detection_qualification_receipt_sha256
            .clone(),
        target_condition_sha256: request.hazard_detection.target_condition_sha256.clone(),
        scenario_context_sha256: request.hazard_detection.scenario_context_sha256.clone(),
        speed_upper_mm_s: request.speed_upper_mm_s,
        conservative_stop_distance_mm: stopping.conservative_distance_bound_mm,
        conservative_stop_time_ms: stopping.conservative_stop_time_bound_ms,
        stopping_tube_sha256: request.stopping_envelope.stop_tube_sha256.clone(),
        stopping_tube_receipt_sha256: request.stopping_envelope.stop_tube_receipt_sha256.clone(),
        stopping_tube_builder_build_sha256: request.stopping_envelope.stop_tube_builder_build_sha256.clone(),
        stopping_tube_coverage_duration_ms: request.stopping_envelope.stop_coverage_duration_ms,
        minimum_detection_distance_mm: stopping_budget.minimum_qualified_detection_distance_mm,
        required_stopping_distance_mm: stopping_budget.required_distance_bound_mm,
        remaining_stopping_distance_mm: stopping_budget.remaining_distance_mm,
        stopping_distance_budget_satisfied: stopping_budget.distance_budget_satisfied,
        checked_at_tick,
        valid_until_tick,
    })
}

/// Revalidate immediately before a separate authorization authority consumes
/// the receipt. This checks site-ledger currentness, evidence-graph currentness,
/// station qualification recency, stopping/detection expiry and exact context
/// equality. The consuming application must still authenticate the receipt.
pub fn verify_integrated_dispatch_readiness_receipt_current(
    ledger: &SiteStateLedger,
    graph: &EvidenceDependencyGraph,
    request: &IntegratedDispatchReadinessRequest,
    receipt: &IntegratedDispatchReadinessReceipt,
    now_tick: u64,
) -> Result<(), IntegratedDispatchReadinessFailure> {
    if receipt.schema_version != INTEGRATED_DISPATCH_RECEIPT_SCHEMA_VERSION {
        return Err(IntegratedDispatchReadinessFailure::UnsupportedSchemaVersion);
    }
    if now_tick < receipt.checked_at_tick || now_tick >= receipt.valid_until_tick {
        return Err(IntegratedDispatchReadinessFailure::IntegratedReceiptExpired);
    }
    verify_dispatch_readiness_receipt_current(
        ledger,
        &receipt.dispatch_readiness,
        &request.dispatch.trusted_occupancy_engine_build_sha256,
        &request.dispatch.required_clearance_policy_sha256,
        now_tick,
    )
    .map_err(IntegratedDispatchReadinessFailure::MotionReadiness)?;

    let mut current_request = request.clone();
    current_request.dispatch.now_tick = now_tick;
    let mut current = check_integrated_dispatch_readiness(ledger, graph, &current_request)?;
    current.dispatch_readiness.checked_at_tick = receipt.dispatch_readiness.checked_at_tick;
    current.checked_at_tick = receipt.checked_at_tick;

    if &current != receipt {
        return Err(IntegratedDispatchReadinessFailure::IntegratedReceiptContextMismatch);
    }
    Ok(())
}

fn validate_stopping_envelope_binding(
    request: &IntegratedDispatchReadinessRequest,
    stopping: crate::stopping_reachability::StoppingReachabilityResult,
) -> Result<(), IntegratedDispatchReadinessFailure> {
    let tube = &request.stopping_envelope;
    for (label, digest) in [
        ("stopping tube", tube.stop_tube_sha256.as_str()),
        ("stopping tube receipt", tube.stop_tube_receipt_sha256.as_str()),
        ("stopping tube builder", tube.stop_tube_builder_build_sha256.as_str()),
        ("composite envelope", tube.composite_envelope_sha256.as_str()),
        ("nominal trajectory", tube.nominal_trajectory_sha256.as_str()),
        ("trigger domain", tube.trigger_domain_sha256.as_str()),
        ("primitive manifest", tube.primitive_manifest_sha256.as_str()),
        ("robot configuration", tube.robot_configuration_sha256.as_str()),
        ("tool configuration", tube.tool_configuration_sha256.as_str()),
        ("payload configuration", tube.payload_configuration_sha256.as_str()),
        ("frame", tube.frame_sha256.as_str()),
        ("transform", tube.transform_sha256.as_str()),
        ("stopping profile", tube.stopping_profile_sha256.as_str()),
        ("stopping qualification", tube.stopping_qualification_receipt_sha256.as_str()),
        ("operating condition", tube.operating_condition_sha256.as_str()),
        ("scenario context", tube.scenario_context_sha256.as_str()),
    ] {
        if !is_sha256(digest) {
            return Err(IntegratedDispatchReadinessFailure::InvalidStoppingEnvelopeDigest(label));
        }
    }
    if tube.schema_version != STOP_TUBE_ENVELOPE_SCHEMA_VERSION {
        return Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeContextMismatch(
            "schema version",
        ));
    }
    let envelope = &request.dispatch.envelope;
    let expected = [
        (tube.composite_envelope_sha256.as_str(), envelope.envelope_sha256.as_str(), "composite envelope"),
        (tube.nominal_trajectory_sha256.as_str(), envelope.trajectory_sha256.as_str(), "nominal trajectory"),
        (tube.trigger_domain_sha256.as_str(), envelope.trajectory_sha256.as_str(), "trigger domain"),
        (tube.primitive_manifest_sha256.as_str(), envelope.primitive_manifest_sha256.as_str(), "primitive manifest"),
        (tube.robot_configuration_sha256.as_str(), envelope.robot_configuration_sha256.as_str(), "robot configuration"),
        (tube.tool_configuration_sha256.as_str(), envelope.tool_configuration_sha256.as_str(), "tool configuration"),
        (tube.payload_configuration_sha256.as_str(), envelope.payload_configuration_sha256.as_str(), "payload configuration"),
        (tube.frame_sha256.as_str(), envelope.frame_sha256.as_str(), "frame"),
        (tube.transform_sha256.as_str(), envelope.transform_sha256.as_str(), "transform"),
        (tube.stopping_profile_sha256.as_str(), request.stopping_dynamics.profile_sha256.as_str(), "stopping profile"),
        (tube.stopping_qualification_receipt_sha256.as_str(), request.stopping_dynamics.qualification_receipt_sha256.as_str(), "stopping qualification"),
        (tube.operating_condition_sha256.as_str(), request.stopping_dynamics.operating_condition_sha256.as_str(), "operating condition"),
        (tube.scenario_context_sha256.as_str(), request.stopping_dynamics.scenario_context_sha256.as_str(), "scenario context"),
        (tube.site_clock_domain_id.as_str(), envelope.site_clock_domain_id.as_str(), "site clock domain"),
    ];
    for (actual, expected, label) in expected {
        if actual != expected {
            return Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeContextMismatch(label));
        }
    }
    if tube.trigger_domain_start_tick < envelope.generated_at_tick
        || tube.trigger_domain_end_tick <= tube.trigger_domain_start_tick
        || tube.trigger_domain_end_tick > envelope.valid_until_tick
        || tube.valid_until_tick <= request.dispatch.now_tick
    {
        return Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeExpired);
    }
    if tube.primitive_count == 0
        || !tube.trigger_domain_coverage_complete
        || !tube.primitive_coverage_complete
        || !tube.continuous_time_coverage_complete
        || !tube.conservative_outer_bound
    {
        return Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeCoverageIncomplete);
    }
    if tube.conservative_stop_distance_bound_mm == 0
        || tube.stop_coverage_duration_ms == 0
        || tube.conservative_stop_distance_bound_mm < stopping.conservative_distance_bound_mm
        || tube.stop_coverage_duration_ms < stopping.conservative_stop_time_bound_ms
    {
        return Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeBoundTooSmall);
    }
    if request.dispatch.occupancy.assessed_stop_tube_sha256 != tube.stop_tube_sha256
        || !request.dispatch.occupancy.stop_tube_coverage_complete
        || request.dispatch.occupancy.assessed_envelope_sha256
            != tube.composite_envelope_sha256
    {
        return Err(IntegratedDispatchReadinessFailure::OccupancyDidNotAssessStoppingEnvelope);
    }
    Ok(())
}

fn latest_station_qualification<'a>(
    ledger: &'a SiteStateLedger,
    package_sha256: &str,
    station_id: &str,
) -> Result<&'a SiteEventEnvelope, IntegratedDispatchReadinessFailure> {
    let mut latest = None;
    for event in ledger.event_history() {
        if event.actor_role != SiteActorRole::Surveyor
            || !ledger
                .actor_registry()
                .allows(&event.actor_id, SiteActorRole::Surveyor)
        {
            continue;
        }
        if let SiteEventPayload::StationQualified {
            package_sha256: event_package,
            station_id: event_station,
            ..
        } = &event.payload
        {
            if event_package == package_sha256 && event_station == station_id {
                latest = Some(event);
            }
        }
    }
    latest.ok_or(IntegratedDispatchReadinessFailure::StationQualificationMissing)
}

fn validate_dependency_snapshot(
    graph: &EvidenceDependencyGraph,
    snapshot: &EvidenceSnapshot,
    request: &IntegratedDispatchReadinessRequest,
    package: &crate::site_state_ledger::SiteWorkPackage,
    station_event_sha256: &str,
) -> Result<(), IntegratedDispatchReadinessFailure> {
    let bindings = &request.evidence_nodes;
    let subject = graph
        .node(&bindings.dispatch_readiness)
        .ok_or_else(|| {
            IntegratedDispatchReadinessFailure::MissingEvidenceNode(
                bindings.dispatch_readiness.clone(),
            )
        })?;
    if snapshot.subject_node_id != bindings.dispatch_readiness
        || snapshot.subject_version_sha256 != request.dispatch_context_sha256
        || subject.kind != EvidenceNodeKind::DispatchReadiness
        || subject.version_sha256 != request.dispatch_context_sha256
    {
        return Err(IntegratedDispatchReadinessFailure::EvidenceSnapshotSubjectMismatch);
    }
    if !graph
        .snapshot_is_current(snapshot)
        .map_err(IntegratedDispatchReadinessFailure::EvidenceGraph)?
    {
        return Err(IntegratedDispatchReadinessFailure::EvidenceSnapshotNotCurrent);
    }

    let expected: [(&str, EvidenceNodeKind, &str); 22] = [
        (
            &bindings.work_package,
            EvidenceNodeKind::WorkPackage,
            package.package_sha256.as_str(),
        ),
        (
            &bindings.source_model,
            EvidenceNodeKind::SourceModel,
            package.source_model_sha256.as_str(),
        ),
        (
            &bindings.ids_specification,
            EvidenceNodeKind::IdsSpecification,
            package.ids_spec_sha256.as_str(),
        ),
        (
            &bindings.ids_engine_build,
            EvidenceNodeKind::IdsEngineBuild,
            package.ids_engine_build_sha256.as_str(),
        ),
        (
            &bindings.ids_evaluation_manifest,
            EvidenceNodeKind::IdsEvaluation,
            package.entity_manifest_sha256.as_str(),
        ),
        (
            &bindings.coordinate_context,
            EvidenceNodeKind::CoordinateContext,
            package.coordinate_context_sha256.as_str(),
        ),
        (
            &bindings.coordinate_frame,
            EvidenceNodeKind::CoordinateFrame,
            package.frame_sha256.as_str(),
        ),
        (
            &bindings.survey_transform,
            EvidenceNodeKind::SurveyTransform,
            package.transform_sha256.as_str(),
        ),
        (
            &bindings.station_qualification,
            EvidenceNodeKind::StationQualification,
            station_event_sha256,
        ),
        (
            &bindings.task_plan,
            EvidenceNodeKind::TaskPlan,
            request.task_plan_sha256.as_str(),
        ),
        (
            &bindings.robot_configuration,
            EvidenceNodeKind::RobotConfiguration,
            request.dispatch.envelope.robot_configuration_sha256.as_str(),
        ),
        (
            &bindings.tool_configuration,
            EvidenceNodeKind::ToolConfiguration,
            request.dispatch.envelope.tool_configuration_sha256.as_str(),
        ),
        (
            &bindings.payload_configuration,
            EvidenceNodeKind::PayloadConfiguration,
            request.dispatch.envelope.payload_configuration_sha256.as_str(),
        ),
        (
            &bindings.motion_envelope,
            EvidenceNodeKind::MotionEnvelope,
            request.dispatch.envelope.envelope_sha256.as_str(),
        ),
        (
            &bindings.stopping_tube_envelope,
            EvidenceNodeKind::StoppingTubeEnvelope,
            request.stopping_envelope.stop_tube_sha256.as_str(),
        ),
        (
            &bindings.stopping_tube_builder_build,
            EvidenceNodeKind::StoppingTubeBuilderBuild,
            request.stopping_envelope.stop_tube_builder_build_sha256.as_str(),
        ),
        (
            &bindings.sensor_configuration,
            EvidenceNodeKind::SensorConfiguration,
            request.hazard_detection.sensor_profile_sha256.as_str(),
        ),
        (
            &bindings.stopping_dynamics,
            EvidenceNodeKind::StoppingDynamicsQualification,
            request.stopping_dynamics.profile_sha256.as_str(),
        ),
        (
            &bindings.hazard_detection,
            EvidenceNodeKind::HazardDetectionQualification,
            request
                .hazard_detection
                .detection_qualification_receipt_sha256
                .as_str(),
        ),
        (
            &bindings.operating_condition,
            EvidenceNodeKind::OperatingCondition,
            request.stopping_dynamics.operating_condition_sha256.as_str(),
        ),
        (
            &bindings.occupancy_engine_build,
            EvidenceNodeKind::OccupancyEngineBuild,
            request
                .dispatch
                .occupancy
                .occupancy_engine_build_sha256
                .as_str(),
        ),
        (
            &bindings.clearance_policy,
            EvidenceNodeKind::ClearancePolicy,
            request.dispatch.occupancy.clearance_policy_sha256.as_str(),
        ),
    ];

    let mut seen = std::collections::BTreeSet::new();
    for (node_id, expected_kind, expected_version) in expected {
        if node_id.trim().is_empty() || !seen.insert(node_id) {
            return Err(IntegratedDispatchReadinessFailure::MissingEvidenceNode(
                node_id.to_owned(),
            ));
        }
        let node = graph.node(node_id).ok_or_else(|| {
            IntegratedDispatchReadinessFailure::MissingEvidenceNode(node_id.to_owned())
        })?;
        if node.kind != expected_kind {
            return Err(IntegratedDispatchReadinessFailure::EvidenceNodeKindMismatch {
                node_id: node_id.to_owned(),
                expected: expected_kind,
                actual: node.kind,
            });
        }
        if node.version_sha256 != expected_version {
            return Err(IntegratedDispatchReadinessFailure::EvidenceNodeVersionMismatch(
                node_id.to_owned(),
            ));
        }
        let dependency_version = snapshot
            .transitive_dependency_versions
            .iter()
            .find(|item| item.node_id == node_id)
            .ok_or_else(|| {
                IntegratedDispatchReadinessFailure::MissingEvidenceNode(node_id.to_owned())
            })?;
        if dependency_version.kind != expected_kind
            || dependency_version.version_sha256 != expected_version
        {
            return Err(IntegratedDispatchReadinessFailure::EvidenceNodeVersionMismatch(
                node_id.to_owned(),
            ));
        }
    }

    Ok(())
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
    use crate::dependency_scoped_invalidation::{
        EvidenceDependency, EvidenceNode, EvidenceRevisionChange,
    };
    use crate::dispatch_readiness::{
        DISPATCH_READINESS_REQUEST_SCHEMA_VERSION, DispatchEnvelopeBinding,
        OccupancyAssessmentReceipt,
    };
    use crate::field_layout_acceptance::{FieldAcceptanceProfile, FieldLayoutEntityType};
    use crate::site_state_ledger::{
        DesignIntentEntity, SiteActorRegistry, SiteEventPayload, SiteWorkPackage,
    };
    use crate::stopping_reachability::{
        HazardDetectionQualification, StoppingDynamicsQualification,
    };

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn setup_ledger() -> (SiteStateLedger, String, String) {
        let mut actors = SiteActorRegistry::new();
        actors
            .grant("design-authority", SiteActorRole::DesignAuthority)
            .unwrap();
        actors.grant("surveyor", SiteActorRole::Surveyor).unwrap();
        actors.grant("robot-01", SiteActorRole::Robot).unwrap();
        actors.grant("referee", SiteActorRole::Referee).unwrap();
        let mut ledger = SiteStateLedger::with_actor_registry(actors);
        let package = SiteWorkPackage {
            package_sha256: digest('a'),
            source_model_sha256: digest('b'),
            source_schema_release: "IFC4X3_ADD2".into(),
            ids_spec_sha256: digest('c'),
            ids_version: "1.0".into(),
            ids_engine_build_sha256: digest('d'),
            coordinate_context_sha256: digest('e'),
            design_revision_sha256: digest('f'),
            frame_id: "site-frame".into(),
            frame_sha256: digest('1'),
            transform_sha256: digest('2'),
            entity_manifest_sha256: digest('3'),
            acceptance_profile_sha256: digest('4'),
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
                event_sha256: digest('5'),
                actor_id: "design-authority".into(),
                actor_role: SiteActorRole::DesignAuthority,
                payload: SiteEventPayload::WorkPackagePublished { package },
            })
            .unwrap();
        let station_event_id = "station-qualification-01".to_owned();
        ledger
            .apply(SiteEventEnvelope {
                event_id: station_event_id.clone(),
                sequence: 2,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('6'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256: digest('a'),
                    station_id: "station-01".into(),
                    frame_sha256: digest('1'),
                    transform_sha256: digest('2'),
                    calibration_sha256: digest('7'),
                    site_clock_domain_id: "site-clock-01".into(),
                    valid_until_tick: 8_000,
                },
            })
            .unwrap();
        (ledger, digest('a'), station_event_id)
    }

    fn make_request(
        ledger: &SiteStateLedger,
        station_event_id: String,
    ) -> IntegratedDispatchReadinessRequest {
        let package = ledger.work_package(ledger.active_package_sha256().unwrap()).unwrap();
        let dispatch = DispatchReadinessRequest {
            schema_version: DISPATCH_READINESS_REQUEST_SCHEMA_VERSION.into(),
            envelope: DispatchEnvelopeBinding {
                package_sha256: package.package_sha256.clone(),
                design_revision_sha256: package.design_revision_sha256.clone(),
                entity_id: "ifc4x3:0000000000000000000001".into(),
                frame_sha256: package.frame_sha256.clone(),
                transform_sha256: package.transform_sha256.clone(),
                robot_id: "robot-01".into(),
                robot_configuration_sha256: digest('8'),
                tool_configuration_sha256: digest('9'),
                payload_configuration_sha256: digest('a'),
                primitive_manifest_sha256: digest('b'),
                trajectory_sha256: digest('c'),
                envelope_sha256: digest('d'),
                site_clock_domain_id: "site-clock-01".into(),
                generated_at_tick: 100,
                valid_until_tick: 10_000,
            },
            trusted_occupancy_engine_build_sha256: digest('e'),
            required_clearance_policy_sha256: digest('f'),
            occupancy: OccupancyAssessmentReceipt {
                assessed_envelope_sha256: digest('d'),
                assessed_stop_tube_sha256: digest('8'),
                stop_tube_coverage_complete: true,
                primitive_manifest_sha256: digest('b'),
                site_snapshot_sha256: digest('1'),
                site_ledger_sequence: ledger.last_sequence(),
                site_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
                dynamic_occupancy_manifest_sha256: digest('2'),
                actor_manifest_sha256: digest('3'),
                occupancy_engine_build_sha256: digest('e'),
                clearance_policy_sha256: digest('f'),
                site_clock_domain_id: "site-clock-01".into(),
                checked_at_tick: 110,
                valid_until_tick: 9_000,
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
                assessment_receipt_sha256: digest('4'),
            },
            now_tick: 120,
        };

        let stopping_dynamics = StoppingDynamicsQualification {
            profile_sha256: digest('1'),
            qualification_receipt_sha256: digest('2'),
            robot_brake_configuration_sha256: digest('8'),
            operating_condition_sha256: digest('3'),
            scenario_context_sha256: digest('4'),
            site_clock_domain_id: "site-clock-01".into(),
            valid_until_tick: 7_000,
            maximum_qualified_speed_mm_s: 1_000.0,
            maximum_detection_to_brake_latency_ms: 100,
            maximum_positive_acceleration_mm_s2: 1_000.0,
            minimum_net_deceleration_mm_s2: 1_500.0,
            minimum_braking_jerk_mm_s3: 3_000.0,
            additional_distance_guard_mm: 10.0,
        };
        let stopping_envelope = StopTubeEnvelopeReceipt {
            schema_version: STOP_TUBE_ENVELOPE_SCHEMA_VERSION.into(),
            stop_tube_sha256: digest('8'),
            stop_tube_receipt_sha256: digest('9'),
            stop_tube_builder_build_sha256: digest('a'),
            composite_envelope_sha256: digest('d'),
            nominal_trajectory_sha256: digest('c'),
            trigger_domain_sha256: digest('c'),
            primitive_manifest_sha256: digest('b'),
            robot_configuration_sha256: digest('8'),
            tool_configuration_sha256: digest('9'),
            payload_configuration_sha256: digest('a'),
            frame_sha256: digest('1'),
            transform_sha256: digest('2'),
            stopping_profile_sha256: digest('5'),
            stopping_qualification_receipt_sha256: digest('6'),
            operating_condition_sha256: digest('3'),
            scenario_context_sha256: digest('4'),
            site_clock_domain_id: "site-clock-01".into(),
            trigger_domain_start_tick: 100,
            trigger_domain_end_tick: 1_000,
            stop_coverage_duration_ms: 6_000,
            conservative_stop_distance_bound_mm: 10_000,
            primitive_count: 4,
            trigger_domain_coverage_complete: true,
            primitive_coverage_complete: true,
            continuous_time_coverage_complete: true,
            conservative_outer_bound: true,
            valid_until_tick: 8_000,
        };
        let hazard_detection = HazardDetectionQualification {
            sensor_profile_sha256: digest('5'),
            detection_qualification_receipt_sha256: digest('6'),
            target_condition_sha256: digest('7'),
            scenario_context_sha256: digest('4'),
            site_clock_domain_id: "site-clock-01".into(),
            valid_until_tick: 6_000,
            maximum_qualified_approach_speed_mm_s: 1_000.0,
            minimum_qualified_detection_distance_mm: 2_000.0,
        };
        let task_plan_sha256 = digest('9');
        let dispatch_context_sha256 = digest('a');
        let node_ids = IntegratedEvidenceNodeIds {
            dispatch_readiness: "dispatch-readiness".into(),
            work_package: "work-package".into(),
            source_model: "source-model".into(),
            ids_specification: "ids-spec".into(),
            ids_engine_build: "ids-engine".into(),
            ids_evaluation_manifest: "ids-evaluation-manifest".into(),
            coordinate_context: "coordinate-context".into(),
            coordinate_frame: "coordinate-frame".into(),
            survey_transform: "survey-transform".into(),
            station_qualification: "station-qualification".into(),
            task_plan: "task-plan".into(),
            robot_configuration: "robot-config".into(),
            tool_configuration: "tool-config".into(),
            payload_configuration: "payload-config".into(),
            motion_envelope: "motion-envelope".into(),
            stopping_tube_envelope: "stopping-tube-envelope".into(),
            stopping_tube_builder_build: "stopping-tube-builder-build".into(),
            sensor_configuration: "sensor-config".into(),
            stopping_dynamics: "stopping-dynamics".into(),
            hazard_detection: "hazard-detection".into(),
            operating_condition: "operating-condition".into(),
            occupancy_engine_build: "occupancy-engine".into(),
            clearance_policy: "clearance-policy".into(),
        };
        let nodes = vec![
            evidence_node("dispatch-readiness", EvidenceNodeKind::DispatchReadiness, &dispatch_context_sha256),
            evidence_node("work-package", EvidenceNodeKind::WorkPackage, &package.package_sha256),
            evidence_node("source-model", EvidenceNodeKind::SourceModel, &package.source_model_sha256),
            evidence_node("ids-spec", EvidenceNodeKind::IdsSpecification, &package.ids_spec_sha256),
            evidence_node("ids-engine", EvidenceNodeKind::IdsEngineBuild, &package.ids_engine_build_sha256),
            evidence_node("ids-evaluation-manifest", EvidenceNodeKind::IdsEvaluation, &package.entity_manifest_sha256),
            evidence_node("coordinate-context", EvidenceNodeKind::CoordinateContext, &package.coordinate_context_sha256),
            evidence_node("coordinate-frame", EvidenceNodeKind::CoordinateFrame, &package.frame_sha256),
            evidence_node("survey-transform", EvidenceNodeKind::SurveyTransform, &package.transform_sha256),
            evidence_node("station-qualification", EvidenceNodeKind::StationQualification, &digest('6')),
            evidence_node("task-plan", EvidenceNodeKind::TaskPlan, &task_plan_sha256),
            evidence_node("robot-config", EvidenceNodeKind::RobotConfiguration, &dispatch.envelope.robot_configuration_sha256),
            evidence_node("tool-config", EvidenceNodeKind::ToolConfiguration, &dispatch.envelope.tool_configuration_sha256),
            evidence_node("payload-config", EvidenceNodeKind::PayloadConfiguration, &dispatch.envelope.payload_configuration_sha256),
            evidence_node("motion-envelope", EvidenceNodeKind::MotionEnvelope, &dispatch.envelope.envelope_sha256),
            evidence_node("stopping-tube-envelope", EvidenceNodeKind::StoppingTubeEnvelope, &stopping_envelope.stop_tube_sha256),
            evidence_node("stopping-tube-builder-build", EvidenceNodeKind::StoppingTubeBuilderBuild, &stopping_envelope.stop_tube_builder_build_sha256),
            evidence_node("sensor-config", EvidenceNodeKind::SensorConfiguration, &hazard_detection.sensor_profile_sha256),
            evidence_node("stopping-dynamics", EvidenceNodeKind::StoppingDynamicsQualification, &stopping_dynamics.profile_sha256),
            evidence_node("hazard-detection", EvidenceNodeKind::HazardDetectionQualification, &hazard_detection.detection_qualification_receipt_sha256),
            evidence_node("operating-condition", EvidenceNodeKind::OperatingCondition, &stopping_dynamics.operating_condition_sha256),
            evidence_node("occupancy-engine", EvidenceNodeKind::OccupancyEngineBuild, &dispatch.occupancy.occupancy_engine_build_sha256),
            evidence_node("clearance-policy", EvidenceNodeKind::ClearancePolicy, &dispatch.occupancy.clearance_policy_sha256),
        ];
        let mut edges = vec![
            dependency("dispatch-readiness", "work-package"),
            dependency("dispatch-readiness", "task-plan"),
            dependency("dispatch-readiness", "robot-config"),
            dependency("dispatch-readiness", "tool-config"),
            dependency("dispatch-readiness", "payload-config"),
            dependency("dispatch-readiness", "motion-envelope"),
            dependency("dispatch-readiness", "stopping-tube-envelope"),
            dependency("dispatch-readiness", "stopping-tube-builder-build"),
            dependency("dispatch-readiness", "station-qualification"),
            dependency("dispatch-readiness", "sensor-config"),
            dependency("dispatch-readiness", "stopping-dynamics"),
            dependency("dispatch-readiness", "hazard-detection"),
            dependency("dispatch-readiness", "operating-condition"),
            dependency("dispatch-readiness", "occupancy-engine"),
            dependency("dispatch-readiness", "clearance-policy"),
            dependency("work-package", "source-model"),
            dependency("work-package", "ids-spec"),
            dependency("work-package", "ids-engine"),
            dependency("work-package", "ids-evaluation-manifest"),
            dependency("work-package", "coordinate-context"),
            dependency("work-package", "coordinate-frame"),
            dependency("work-package", "survey-transform"),
        ];
        edges.sort();
        let graph = EvidenceDependencyGraph::new(nodes, edges).unwrap();
        let snapshot = graph.capture_snapshot("dispatch-readiness").unwrap();

        IntegratedDispatchReadinessRequest {
            schema_version: INTEGRATED_DISPATCH_REQUEST_SCHEMA_VERSION.into(),
            dispatch,
            evidence_snapshot: snapshot,
            evidence_snapshot_sha256: digest('b'),
            dispatch_context_sha256,
            evidence_nodes: node_ids,
            task_plan_sha256,
            station_id: "station-01".into(),
            station_qualification_event_id: station_event_id,
            speed_upper_mm_s: 500.0,
            stopping_dynamics,
            stopping_envelope,
            hazard_detection,
            stopping_margins: HazardStoppingMargins {
                localization_uncertainty_mm: 10.0,
                frame_registration_uncertainty_mm: 10.0,
                body_geometry_uncertainty_mm: 5.0,
                site_policy_clearance_mm: 30.0,
            },
        }
    }

    fn evidence_node(id: &str, kind: EvidenceNodeKind, version: &str) -> EvidenceNode {
        EvidenceNode {
            node_id: id.into(),
            kind,
            version_sha256: version.into(),
        }
    }

    fn dependency(dependent: &str, prerequisite: &str) -> EvidenceDependency {
        EvidenceDependency {
            dependent_node_id: dependent.into(),
            prerequisite_node_id: prerequisite.into(),
        }
    }

    fn make_graph(request: &IntegratedDispatchReadinessRequest, ledger: &SiteStateLedger) -> EvidenceDependencyGraph {
        let package = ledger.work_package(ledger.active_package_sha256().unwrap()).unwrap();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let ids = &request.evidence_nodes;
        let entries = [
            (&ids.dispatch_readiness, EvidenceNodeKind::DispatchReadiness, &request.dispatch_context_sha256),
            (&ids.work_package, EvidenceNodeKind::WorkPackage, &package.package_sha256),
            (&ids.source_model, EvidenceNodeKind::SourceModel, &package.source_model_sha256),
            (&ids.ids_specification, EvidenceNodeKind::IdsSpecification, &package.ids_spec_sha256),
            (&ids.ids_engine_build, EvidenceNodeKind::IdsEngineBuild, &package.ids_engine_build_sha256),
            (&ids.ids_evaluation_manifest, EvidenceNodeKind::IdsEvaluation, &package.entity_manifest_sha256),
            (&ids.coordinate_context, EvidenceNodeKind::CoordinateContext, &package.coordinate_context_sha256),
            (&ids.coordinate_frame, EvidenceNodeKind::CoordinateFrame, &package.frame_sha256),
            (&ids.survey_transform, EvidenceNodeKind::SurveyTransform, &package.transform_sha256),
            (&ids.station_qualification, EvidenceNodeKind::StationQualification, &digest('6')),
            (&ids.task_plan, EvidenceNodeKind::TaskPlan, &request.task_plan_sha256),
            (&ids.robot_configuration, EvidenceNodeKind::RobotConfiguration, &request.dispatch.envelope.robot_configuration_sha256),
            (&ids.tool_configuration, EvidenceNodeKind::ToolConfiguration, &request.dispatch.envelope.tool_configuration_sha256),
            (&ids.payload_configuration, EvidenceNodeKind::PayloadConfiguration, &request.dispatch.envelope.payload_configuration_sha256),
            (&ids.motion_envelope, EvidenceNodeKind::MotionEnvelope, &request.dispatch.envelope.envelope_sha256),
            (&ids.stopping_tube_envelope, EvidenceNodeKind::StoppingTubeEnvelope, &request.stopping_envelope.stop_tube_sha256),
            (&ids.stopping_tube_builder_build, EvidenceNodeKind::StoppingTubeBuilderBuild, &request.stopping_envelope.stop_tube_builder_build_sha256),
            (&ids.sensor_configuration, EvidenceNodeKind::SensorConfiguration, &request.hazard_detection.sensor_profile_sha256),
            (&ids.stopping_dynamics, EvidenceNodeKind::StoppingDynamicsQualification, &request.stopping_dynamics.profile_sha256),
            (&ids.hazard_detection, EvidenceNodeKind::HazardDetectionQualification, &request.hazard_detection.detection_qualification_receipt_sha256),
            (&ids.operating_condition, EvidenceNodeKind::OperatingCondition, &request.stopping_dynamics.operating_condition_sha256),
            (&ids.occupancy_engine_build, EvidenceNodeKind::OccupancyEngineBuild, &request.dispatch.occupancy.occupancy_engine_build_sha256),
            (&ids.clearance_policy, EvidenceNodeKind::ClearancePolicy, &request.dispatch.occupancy.clearance_policy_sha256),
        ];
        for (id, kind, version) in entries {
            nodes.push(evidence_node(id, kind, version));
        }
        for id in [
            &ids.work_package,
            &ids.task_plan,
            &ids.robot_configuration,
            &ids.tool_configuration,
            &ids.payload_configuration,
            &ids.motion_envelope,
            &ids.stopping_tube_envelope,
            &ids.stopping_tube_builder_build,
            &ids.station_qualification,
            &ids.sensor_configuration,
            &ids.stopping_dynamics,
            &ids.hazard_detection,
            &ids.operating_condition,
            &ids.occupancy_engine_build,
            &ids.clearance_policy,
        ] {
            edges.push(dependency(&ids.dispatch_readiness, id));
        }
        for id in [
            &ids.source_model,
            &ids.ids_specification,
            &ids.ids_engine_build,
            &ids.ids_evaluation_manifest,
            &ids.coordinate_context,
            &ids.coordinate_frame,
            &ids.survey_transform,
        ] {
            edges.push(dependency(&ids.work_package, id));
        }
        EvidenceDependencyGraph::new(nodes, edges).unwrap()
    }

    #[test]
    fn integrated_gate_requires_current_dependencies_fresh_station_and_closed_stop_budget() {
        let (ledger, _, station_event_id) = setup_ledger();
        let request = make_request(&ledger, station_event_id);
        let graph = make_graph(&request, &ledger);
        let receipt = check_integrated_dispatch_readiness(&ledger, &graph, &request).unwrap();
        assert!(receipt.stopping_distance_budget_satisfied);
        assert_eq!(receipt.valid_until_tick, 6_000);
        assert_eq!(receipt.station_qualification_event_sha256, digest('6'));
        assert!(receipt.remaining_stopping_distance_mm > 0.0);

        verify_integrated_dispatch_readiness_receipt_current(
            &ledger,
            &graph,
            &request,
            &receipt,
            130,
        )
        .unwrap();
    }

    #[test]
    fn changed_sensor_or_brake_dependency_invalidates_dispatch_snapshot() {
        let (ledger, _, station_event_id) = setup_ledger();
        let request = make_request(&ledger, station_event_id);
        let mut graph = make_graph(&request, &ledger);
        graph
            .apply_revision(&[EvidenceRevisionChange {
                node_id: request.evidence_nodes.sensor_configuration.clone(),
                new_version_sha256: digest('0'),
                reason: "sensor firmware changed".into(),
            }])
            .unwrap();
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::EvidenceSnapshotNotCurrent)
        );
    }

    #[test]
    fn a_too_short_detection_range_fails_even_when_occupancy_is_clear() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.hazard_detection.minimum_qualified_detection_distance_mm = 200.0;
        let graph = make_graph(&request, &ledger);
        assert!(matches!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingBudgetExceeded { .. })
        ));
    }

    #[test]
    fn changed_station_qualification_invalidates_old_station_receipt() {
        let (mut ledger, package_sha256, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id.clone());
        ledger
            .apply(SiteEventEnvelope {
                event_id: "station-qualification-02".into(),
                sequence: 3,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('7'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256,
                    station_id: "station-01".into(),
                    frame_sha256: digest('1'),
                    transform_sha256: digest('2'),
                    calibration_sha256: digest('8'),
                    site_clock_domain_id: "site-clock-01".into(),
                    valid_until_tick: 8_000,
                },
            })
            .unwrap();
        request.dispatch.occupancy.site_ledger_sequence = ledger.last_sequence();
        request.dispatch.occupancy.site_ledger_event_sha256 =
            ledger.last_event_sha256().unwrap().to_owned();
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StationQualificationNotLatest)
        );
    }

    #[test]
    fn wrong_brake_robot_configuration_is_rejected() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_dynamics.robot_brake_configuration_sha256 = digest('0');
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::RobotBrakeConfigurationMismatch)
        );
    }

    #[test]
    fn receipt_is_invalidated_by_a_new_ledger_event_before_consumption() {
        let (mut ledger, package_sha256, station_event_id) = setup_ledger();
        let request = make_request(&ledger, station_event_id);
        let graph = make_graph(&request, &ledger);
        let receipt = check_integrated_dispatch_readiness(&ledger, &graph, &request).unwrap();
        ledger
            .apply(SiteEventEnvelope {
                event_id: "station-qualification-later".into(),
                sequence: 3,
                previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                event_sha256: digest('7'),
                actor_id: "surveyor".into(),
                actor_role: SiteActorRole::Surveyor,
                payload: SiteEventPayload::StationQualified {
                    package_sha256,
                    station_id: "station-02".into(),
                    frame_sha256: digest('1'),
                    transform_sha256: digest('2'),
                    calibration_sha256: digest('8'),
                    site_clock_domain_id: "site-clock-01".into(),
                    valid_until_tick: 8_000,
                },
            })
            .unwrap();
        assert!(matches!(
            verify_integrated_dispatch_readiness_receipt_current(
                &ledger,
                &graph,
                &request,
                &receipt,
                130,
            ),
            Err(IntegratedDispatchReadinessFailure::MotionReadiness(
                DispatchReadinessFailure::SiteSnapshotSequenceMismatch
            ))
        ));
    }

    #[test]
    fn required_dependency_binding_cannot_be_omitted_from_snapshot() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        let graph = make_graph(&request, &ledger);
        request.evidence_nodes.sensor_configuration = "not-in-graph".into();
        assert!(matches!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::MissingEvidenceNode(_))
        ));
    }

    #[test]
    fn current_receipt_expires_at_the_earliest_qualification_boundary() {
        let (ledger, _, station_event_id) = setup_ledger();
        let request = make_request(&ledger, station_event_id);
        let graph = make_graph(&request, &ledger);
        let receipt = check_integrated_dispatch_readiness(&ledger, &graph, &request).unwrap();
        assert_eq!(receipt.valid_until_tick, 6_000);
        assert_eq!(
            verify_integrated_dispatch_readiness_receipt_current(
                &ledger,
                &graph,
                &request,
                &receipt,
                6_000,
            ),
            Err(IntegratedDispatchReadinessFailure::IntegratedReceiptExpired)
        );
    }

    #[test]
    fn a_snapshot_with_the_wrong_subject_context_cannot_authorize_dispatch() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        let graph = make_graph(&request, &ledger);
        request.evidence_snapshot.subject_version_sha256 = digest('0');
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::EvidenceSnapshotSubjectMismatch)
        );
    }

    #[test]
    fn missing_stopping_profile_receipt_cannot_be_replaced_by_clear_occupancy() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_dynamics.qualification_receipt_sha256 = "bad".into();
        let graph = make_graph(&request, &ledger);
        assert!(matches!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingQualification(
                StoppingReachabilityError::InvalidQualificationReceiptDigest
            ))
        ));
    }


    #[test]
    fn stop_tube_is_mandatory_for_integrated_occupancy_readiness() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.dispatch.occupancy.assessed_stop_tube_sha256 = digest('0');
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::OccupancyDidNotAssessStoppingEnvelope)
        );
    }

    #[test]
    fn incomplete_stop_tube_primitive_or_continuous_time_coverage_fails_closed() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.continuous_time_coverage_complete = false;
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeCoverageIncomplete)
        );

        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.primitive_coverage_complete = false;
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeCoverageIncomplete)
        );
    }

    #[test]
    fn stopping_envelope_distance_and_time_must_dominate_qualified_dynamics() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.conservative_stop_distance_bound_mm = 1;
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeBoundTooSmall)
        );

        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.stop_coverage_duration_ms = 1;
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeBoundTooSmall)
        );
    }

    #[test]
    fn stop_tube_must_match_tool_payload_frame_and_composite_envelope() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.payload_configuration_sha256 = digest('0');
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeContextMismatch(
                "payload configuration"
            ))
        );
    }

    #[test]
    fn stopping_envelope_is_a_required_dependency_in_the_dispatch_snapshot() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        let graph = make_graph(&request, &ledger);
        request.evidence_nodes.stopping_tube_envelope = "missing-stop-tube-node".into();
        assert!(matches!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::MissingEvidenceNode(_))
        ));
    }


    #[test]
    fn empty_trigger_domain_and_zero_stop_coverage_are_rejected() {
        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.trigger_domain_end_tick =
            request.stopping_envelope.trigger_domain_start_tick;
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeExpired)
        );

        let (ledger, _, station_event_id) = setup_ledger();
        let mut request = make_request(&ledger, station_event_id);
        request.stopping_envelope.stop_coverage_duration_ms = 0;
        let graph = make_graph(&request, &ledger);
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::StoppingEnvelopeBoundTooSmall)
        );
    }


    #[test]
    fn changing_the_stop_tube_builder_build_invalidates_the_integrated_readiness_snapshot() {
        let (ledger, _, station_event_id) = setup_ledger();
        let request = make_request(&ledger, station_event_id);
        let mut graph = make_graph(&request, &ledger);
        graph
            .apply_revision(&[EvidenceRevisionChange {
                node_id: request.evidence_nodes.stopping_tube_builder_build.clone(),
                new_version_sha256: digest('0'),
                reason: "stop-tube builder implementation changed".into(),
            }])
            .unwrap();
        assert_eq!(
            check_integrated_dispatch_readiness(&ledger, &graph, &request),
            Err(IntegratedDispatchReadinessFailure::EvidenceSnapshotNotCurrent)
        );
    }

}
