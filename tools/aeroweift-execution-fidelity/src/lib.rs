//! Structural verifier for AeroWeft experiment execution and evidence completeness.
//!
//! This crate compares a frozen experiment plan with a separate actual-run record.
//! It does not execute experiments, control aircraft, authenticate receipts or raw bytes,
//! calculate physical residuals, establish safety, or grant operational authority.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// Half-open interval on an explicitly declared time scale.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeWindow {
    /// Inclusive start tick.
    pub start: i64,
    /// Exclusive end tick.
    pub end: i64,
}

impl TimeWindow {
    fn is_valid(self) -> bool {
        self.start < self.end
    }

    fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    fn duration(self) -> u64 {
        self.end.saturating_sub(self.start) as u64
    }
}

/// Exact physical/computational configuration under which the experiment is planned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationIdentity {
    /// Exact aircraft configuration digest.
    pub aircraft_configuration_sha256: String,
    /// Exact model artifact digest.
    pub model_artifact_sha256: String,
    /// Exact flight-control and guidance stack digest.
    pub control_stack_sha256: String,
    /// Exact sensor/receiver/calibration/placement stack digest.
    pub sensor_stack_sha256: String,
    /// Exact solver/integration/numerical configuration digest.
    pub solver_configuration_sha256: String,
    /// Exact independent reference-instrument/calibration profile digest.
    pub reference_measurement_sha256: String,
    /// Exact operating-domain profile digest.
    pub operating_domain_sha256: String,
    /// Explicit time-scale identity.
    pub time_scale_id: String,
    /// Horizontal coordinate-frame identity.
    pub horizontal_frame_id: String,
    /// Vertical datum identity.
    pub vertical_datum_id: String,
}

impl ConfigurationIdentity {
    fn is_complete(&self) -> bool {
        is_sha256(&self.aircraft_configuration_sha256)
            && is_sha256(&self.model_artifact_sha256)
            && is_sha256(&self.control_stack_sha256)
            && is_sha256(&self.sensor_stack_sha256)
            && is_sha256(&self.solver_configuration_sha256)
            && is_sha256(&self.reference_measurement_sha256)
            && is_sha256(&self.operating_domain_sha256)
            && !self.time_scale_id.trim().is_empty()
            && !self.horizontal_frame_id.trim().is_empty()
            && !self.vertical_datum_id.trim().is_empty()
    }
}

/// One expected measurement channel and time window in the approved plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedDataWindow {
    /// Stable expected window identity.
    pub window_id: String,
    /// Expected data-source identity.
    pub source_id: String,
    /// Expected stream within that source.
    pub source_stream_id: String,
    /// Planned half-open time interval.
    pub window: TimeWindow,
    /// Minimum independent samples required for this window.
    pub minimum_samples: u64,
    /// Exact measurement/calibration profile expected for this window.
    pub measurement_profile_sha256: String,
    /// Explicit time-scale identity for this channel.
    pub time_scale_id: String,
    /// Horizontal frame for this channel.
    pub horizontal_frame_id: String,
    /// Vertical datum for this channel.
    pub vertical_datum_id: String,
}

/// Frozen contract emitted by Track V for one selected experiment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPlan {
    /// Stable experiment-candidate identity selected by Track V.
    pub candidate_id: String,
    /// Exact candidate payload digest.
    pub candidate_sha256: String,
    /// Exact Track V selection-policy digest.
    pub selection_policy_sha256: String,
    /// Exact experiment-design digest.
    pub design_sha256: String,
    /// Exact procedure revision digest.
    pub procedure_sha256: String,
    /// Exact initial-condition/scenario manifest digest.
    pub scenario_manifest_sha256: String,
    /// Exact measurement-plan digest.
    pub measurement_plan_sha256: String,
    /// Exact approved configuration identity.
    pub configuration: ConfigurationIdentity,
    /// Environment/traffic strata permitted by the plan.
    pub allowed_environment_strata: Vec<String>,
    /// Whether an external authorization is required for this run.
    pub authorization_required: bool,
    /// Exact authorization scope digest required by the plan.
    pub authorization_scope_sha256: String,
    /// Permitted actual run-time interval.
    pub permitted_run_window: TimeWindow,
    /// Every data window required to support the full planned claim.
    pub expected_data_windows: Vec<ExpectedDataWindow>,
    /// Exact execution-fidelity policy manifest digest.
    pub policy_sha256: String,
}

/// Authority state supplied by an external owner.
/// The verifier checks declared scope and validity; it does not authenticate the receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityState {
    /// Responsible authority supplied an affirmative authorization.
    Authorized,
    /// Responsible authority explicitly denied the experiment.
    NotAuthorized,
    /// Modality/run did not require separate external authorization.
    NotRequired,
    /// Authorization state is unknown or not assessable.
    Unknown,
}

/// Owner-supplied authorization evidence for the actual run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityEvidence {
    /// Declared authority state.
    pub state: AuthorityState,
    /// Exact authorization-receipt payload digest, if supplied.
    pub receipt_sha256: Option<String>,
    /// Scope digest the authority says it authorized.
    pub scope_sha256: Option<String>,
    /// Effective validity interval of the authorization.
    pub valid_window: Option<TimeWindow>,
}

/// Severity of a documented difference from the frozen plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviationSeverity {
    /// Known, documented deviation; it may support a narrower engineering interpretation.
    Documented,
    /// Deviation exists but its consequences cannot be assessed from supplied evidence.
    Unknown,
    /// Deviation violates an authority, envelope or safety constraint.
    OutsideAuthorityEnvelope,
}

/// Category of an actual-run deviation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviationKind {
    /// Actual procedure differed from the approved procedure.
    Procedure,
    /// Actual aircraft/model/control/sensor/solver configuration changed.
    Configuration,
    /// Environment/traffic stratum differed from the permitted set.
    Environment,
    /// Operator override or manual intervention occurred.
    OperatorOverride,
    /// Clock, frame or datum condition differed from the planned basis.
    TimeOrReferenceBasis,
    /// Measurement instrument, calibration or protocol differed.
    MeasurementSystem,
    /// Other explicitly recorded deviation.
    Other,
}

/// One immutable deviation record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionDeviation {
    /// Stable deviation identifier.
    pub deviation_id: String,
    /// Deviation category.
    pub kind: DeviationKind,
    /// Owner-assessed severity.
    pub severity: DeviationSeverity,
    /// Effective interval of the deviation, if bounded.
    pub effective_window: Option<TimeWindow>,
    /// Digest of the detailed deviation record; syntax checked only.
    pub record_sha256: String,
}

/// Why an execution was aborted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbortReason {
    /// Safety/procedure abort.
    Safety,
    /// Aircraft, model, sensor or control-system anomaly.
    SystemAnomaly,
    /// Environment or airspace condition changed.
    Environment,
    /// Data acquisition or measurement failure.
    EvidenceCaptureFailure,
    /// Operator or test director terminated the run.
    OperatorDecision,
    /// Other documented reason.
    Other,
}

/// Immutable abort record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbortRecord {
    /// Effective abort time.
    pub aborted_at: i64,
    /// Reason for abort.
    pub reason: AbortReason,
    /// Digest of the abort record; syntax checked only.
    pub record_sha256: String,
}

/// One time-bounded raw-data segment captured during the run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSegment {
    /// Stable segment identity.
    pub segment_id: String,
    /// Data-source identity.
    pub source_id: String,
    /// Stream within that source.
    pub source_stream_id: String,
    /// Monotonic sequence number within the stream.
    pub source_sequence: u64,
    /// Captured half-open time interval.
    pub window: TimeWindow,
    /// Number of samples in the segment.
    pub sample_count: u64,
    /// Raw payload digest string; bytes are not recomputed here.
    pub payload_sha256: String,
    /// Measurement/calibration profile actually used.
    pub measurement_profile_sha256: String,
    /// Time scale actually used.
    pub time_scale_id: String,
    /// Horizontal frame actually used.
    pub horizontal_frame_id: String,
    /// Vertical datum actually used.
    pub vertical_datum_id: String,
}

/// Actual execution record; planned values and actual values are kept separate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionRecord {
    /// Stable run identity.
    pub run_id: String,
    /// Candidate identity recorded by the execution system.
    pub candidate_id: String,
    /// Candidate payload digest recorded by the execution system.
    pub candidate_sha256: String,
    /// Track V selection-policy digest recorded by the execution system.
    pub selection_policy_sha256: String,
    /// Experiment-design digest actually executed.
    pub design_sha256: String,
    /// Procedure revision actually used.
    pub procedure_sha256: String,
    /// Scenario/initial-condition manifest actually used.
    pub scenario_manifest_sha256: String,
    /// Measurement plan actually used.
    pub measurement_plan_sha256: String,
    /// Actual aircraft/model/control/sensor/solver/reference configuration.
    pub configuration: ConfigurationIdentity,
    /// Environment/traffic strata actually realized.
    pub actual_environment_strata: Vec<String>,
    /// Actual run start time.
    pub actual_start: i64,
    /// Actual run end time.
    pub actual_end: i64,
    /// Owner-supplied authority evidence.
    pub authority: AuthorityEvidence,
    /// Documented deviations, including overrides and substitutions.
    pub deviations: Vec<ExecutionDeviation>,
    /// Abort record, when the run did not complete normally.
    pub abort: Option<AbortRecord>,
    /// Raw-data segments captured during the run.
    pub segments: Vec<DataSegment>,
    /// Exact data-capture software build.
    pub capture_build_sha256: String,
    /// Exact execution-fidelity policy used by the run recorder.
    pub policy_sha256: String,
}

/// Frozen structural evaluation policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPolicy {
    /// Exact policy manifest digest.
    pub policy_sha256: String,
    /// Exact permitted capture-system build digest.
    pub required_capture_build_sha256: String,
}

/// Primary result state for the actual run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionStatus {
    /// Exact plan executed, authority valid and every required data window complete.
    ExecutionConforming,
    /// Known noncritical deviations occurred; do not treat as an unmodified plan execution.
    ExecutionDeviated,
    /// Execution terminated before the approved run completed.
    Aborted,
    /// Authority, deviation impact or relevant evidence could not be established.
    Indeterminate,
    /// One or more expected data windows are missing, partial or below sample floor.
    EvidenceIncomplete,
    /// One or more exact plan, authority, configuration or envelope requirements were violated.
    NotAdmissibleForClaim,
}

/// Configuration or scope dimension that differs from the frozen plan.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MismatchDimension {
    /// Candidate or selection policy identity.
    SelectionIdentity,
    /// Experiment design identity.
    ExperimentDesign,
    /// Procedure revision.
    Procedure,
    /// Initial-condition/scenario manifest.
    ScenarioManifest,
    /// Measurement-plan identity.
    MeasurementPlan,
    /// Aircraft configuration.
    AircraftConfiguration,
    /// Dynamics/model artifact.
    ModelArtifact,
    /// Flight-control/guidance stack.
    ControlStack,
    /// Sensor/receiver/calibration stack.
    SensorStack,
    /// Solver/numerical configuration.
    SolverConfiguration,
    /// Independent reference measurement profile.
    ReferenceMeasurement,
    /// Operating-domain profile.
    OperatingDomain,
    /// Time scale, coordinate frame or vertical datum.
    TimeOrReferenceBasis,
    /// Actual run started or ended outside the permitted window.
    RunTimeWindow,
    /// Authorization scope or validity differs.
    Authorization,
    /// Actual environment/traffic stratum is not permitted.
    EnvironmentStratum,
    /// Unexpected data stream or measurement channel.
    DataStream,
    /// Raw segment measurement/time/frame/datum basis differs.
    SegmentMeasurementBasis,
    /// Capture software build differs from the pinned policy.
    CaptureBuild,
    /// Execution-fidelity policy differs from the frozen plan.
    Policy,
}

/// Window-level coverage result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowCoverage {
    /// Expected window identity.
    pub window_id: String,
    /// Expected window duration in ticks.
    pub expected_ticks: u64,
    /// Covered duration in ticks.
    pub covered_ticks: u64,
    /// Captured sample count.
    pub sample_count: u64,
    /// Whether the entire expected interval and sample floor were satisfied.
    pub complete: bool,
}

/// Per-run execution-fidelity assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionAssessment {
    /// Run being assessed.
    pub run_id: String,
    /// Exact candidate digest whose plan was evaluated.
    pub candidate_sha256: String,
    /// Frozen policy digest.
    pub policy_sha256: String,
    /// Primary status.
    pub status: ExecutionStatus,
    /// Exact plan/actual mismatches.
    pub mismatch_dimensions: Vec<MismatchDimension>,
    /// Deviation IDs retained in the assessment.
    pub deviation_ids: Vec<String>,
    /// Missing or partial expected data-window IDs.
    pub incomplete_window_ids: Vec<String>,
    /// Canonically ordered window coverage results.
    pub window_coverage: Vec<WindowCoverage>,
    /// Total captured sample count.
    pub total_samples: u64,
    /// Whether the entire planned experiment claim is supported by execution evidence structure.
    pub full_plan_evidence_complete: bool,
    /// Whether the run was aborted.
    pub aborted: bool,
}

/// Structural input failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionError {
    /// Frozen plan identity, time window, configuration or data-window contract is malformed.
    InvalidPlan,
    /// Run identity, actual time interval or actual configuration is malformed.
    InvalidExecutionRecord,
    /// Policy identity or required capture build is malformed.
    InvalidPolicy,
    /// Duplicate expected window identity.
    DuplicateExpectedWindow,
    /// Duplicate deviation identity.
    DuplicateDeviationId,
    /// Deviation interval or record digest is malformed.
    InvalidDeviation,
    /// Abort time or record digest is malformed.
    InvalidAbort,
    /// Duplicate raw-data segment identity.
    DuplicateSegmentId,
    /// One source stream reuses a sequence number.
    ForkedSegmentSequence,
    /// Segment intervals overlap within one source stream.
    OverlappingSegments,
    /// Segment ordering contradicts its monotonic source sequence.
    SegmentSequenceTimeInversion,
    /// Segment identity, interval, sample count or digest is malformed.
    InvalidSegment,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value.as_str()))
}

fn validate_plan(plan: &ExecutionPlan) -> Result<(), ExecutionError> {
    if plan.candidate_id.trim().is_empty()
        || !is_sha256(&plan.candidate_sha256)
        || !is_sha256(&plan.selection_policy_sha256)
        || !is_sha256(&plan.design_sha256)
        || !is_sha256(&plan.procedure_sha256)
        || !is_sha256(&plan.scenario_manifest_sha256)
        || !is_sha256(&plan.measurement_plan_sha256)
        || !plan.configuration.is_complete()
        || !is_sha256(&plan.authorization_scope_sha256)
        || !plan.permitted_run_window.is_valid()
        || !is_sha256(&plan.policy_sha256)
        || plan.allowed_environment_strata.is_empty()
        || !unique_nonempty(&plan.allowed_environment_strata)
        || plan.expected_data_windows.is_empty()
    {
        return Err(ExecutionError::InvalidPlan);
    }

    let mut window_ids = BTreeSet::new();
    for expected in &plan.expected_data_windows {
        if expected.window_id.trim().is_empty()
            || expected.source_id.trim().is_empty()
            || expected.source_stream_id.trim().is_empty()
            || !expected.window.is_valid()
            || !plan.permitted_run_window.contains(expected.window)
            || expected.minimum_samples == 0
            || !is_sha256(&expected.measurement_profile_sha256)
            || expected.time_scale_id != plan.configuration.time_scale_id
            || expected.horizontal_frame_id != plan.configuration.horizontal_frame_id
            || expected.vertical_datum_id != plan.configuration.vertical_datum_id
            || !window_ids.insert(expected.window_id.as_str())
        {
            if window_ids.contains(expected.window_id.as_str()) {
                return Err(ExecutionError::DuplicateExpectedWindow);
            }
            return Err(ExecutionError::InvalidPlan);
        }
    }
    Ok(())
}

fn validate_record(record: &ExecutionRecord) -> Result<(), ExecutionError> {
    if record.run_id.trim().is_empty()
        || record.candidate_id.trim().is_empty()
        || !is_sha256(&record.candidate_sha256)
        || !is_sha256(&record.selection_policy_sha256)
        || !is_sha256(&record.design_sha256)
        || !is_sha256(&record.procedure_sha256)
        || !is_sha256(&record.scenario_manifest_sha256)
        || !is_sha256(&record.measurement_plan_sha256)
        || !record.configuration.is_complete()
        || record.actual_start >= record.actual_end
        || !unique_nonempty(&record.actual_environment_strata)
        || !is_sha256(&record.capture_build_sha256)
        || !is_sha256(&record.policy_sha256)
    {
        return Err(ExecutionError::InvalidExecutionRecord);
    }

    let mut deviation_ids = BTreeSet::new();
    for deviation in &record.deviations {
        if deviation.deviation_id.trim().is_empty()
            || !is_sha256(&deviation.record_sha256)
            || deviation.effective_window.is_some_and(|window| {
                !window.is_valid()
                    || !TimeWindow {
                        start: record.actual_start,
                        end: record.actual_end,
                    }
                    .contains(window)
            })
        {
            return Err(ExecutionError::InvalidDeviation);
        }
        if !deviation_ids.insert(deviation.deviation_id.as_str()) {
            return Err(ExecutionError::DuplicateDeviationId);
        }
    }

    if let Some(abort) = &record.abort
        && (abort.aborted_at < record.actual_start
            || abort.aborted_at >= record.actual_end
            || !is_sha256(&abort.record_sha256))
    {
        return Err(ExecutionError::InvalidAbort);
    }

    let mut segment_ids = BTreeSet::new();
    let mut sequences: BTreeMap<(&str, &str, u64), (&str, TimeWindow)> = BTreeMap::new();
    let mut by_stream: BTreeMap<(&str, &str), Vec<&DataSegment>> = BTreeMap::new();
    for segment in &record.segments {
        if segment.segment_id.trim().is_empty()
            || segment.source_id.trim().is_empty()
            || segment.source_stream_id.trim().is_empty()
            || segment.source_sequence == 0
            || !segment.window.is_valid()
            || segment.window.start < record.actual_start
            || segment.window.end > record.actual_end
            || segment.sample_count == 0
            || !is_sha256(&segment.payload_sha256)
            || !is_sha256(&segment.measurement_profile_sha256)
            || segment.time_scale_id.trim().is_empty()
            || segment.horizontal_frame_id.trim().is_empty()
            || segment.vertical_datum_id.trim().is_empty()
        {
            return Err(ExecutionError::InvalidSegment);
        }
        if !segment_ids.insert(segment.segment_id.as_str()) {
            return Err(ExecutionError::DuplicateSegmentId);
        }
        let sequence_key = (
            segment.source_id.as_str(),
            segment.source_stream_id.as_str(),
            segment.source_sequence,
        );
        if sequences
            .insert(
                sequence_key,
                (segment.segment_id.as_str(), segment.window),
            )
            .is_some()
        {
            return Err(ExecutionError::ForkedSegmentSequence);
        }
        by_stream
            .entry((segment.source_id.as_str(), segment.source_stream_id.as_str()))
            .or_default()
            .push(segment);
    }

    for segments in by_stream.values_mut() {
        segments.sort_by_key(|segment| segment.source_sequence);
        for pair in segments.windows(2) {
            if pair[0].window.start > pair[1].window.start {
                return Err(ExecutionError::SegmentSequenceTimeInversion);
            }
        }
        segments.sort_by_key(|segment| segment.window.start);
        for pair in segments.windows(2) {
            if pair[0].window.end > pair[1].window.start {
                return Err(ExecutionError::OverlappingSegments);
            }
        }
    }

    Ok(())
}

fn identity_mismatches(
    expected: &ConfigurationIdentity,
    actual: &ConfigurationIdentity,
) -> BTreeSet<MismatchDimension> {
    let mut mismatches = BTreeSet::new();
    if expected.aircraft_configuration_sha256 != actual.aircraft_configuration_sha256 {
        mismatches.insert(MismatchDimension::AircraftConfiguration);
    }
    if expected.model_artifact_sha256 != actual.model_artifact_sha256 {
        mismatches.insert(MismatchDimension::ModelArtifact);
    }
    if expected.control_stack_sha256 != actual.control_stack_sha256 {
        mismatches.insert(MismatchDimension::ControlStack);
    }
    if expected.sensor_stack_sha256 != actual.sensor_stack_sha256 {
        mismatches.insert(MismatchDimension::SensorStack);
    }
    if expected.solver_configuration_sha256 != actual.solver_configuration_sha256 {
        mismatches.insert(MismatchDimension::SolverConfiguration);
    }
    if expected.reference_measurement_sha256 != actual.reference_measurement_sha256 {
        mismatches.insert(MismatchDimension::ReferenceMeasurement);
    }
    if expected.operating_domain_sha256 != actual.operating_domain_sha256 {
        mismatches.insert(MismatchDimension::OperatingDomain);
    }
    if expected.time_scale_id != actual.time_scale_id
        || expected.horizontal_frame_id != actual.horizontal_frame_id
        || expected.vertical_datum_id != actual.vertical_datum_id
    {
        mismatches.insert(MismatchDimension::TimeOrReferenceBasis);
    }
    mismatches
}

fn evaluate_authority(
    plan: &ExecutionPlan,
    record: &ExecutionRecord,
    mismatches: &mut BTreeSet<MismatchDimension>,
    unknown: &mut bool,
) -> bool {
    if !plan.authorization_required {
        return true;
    }
    match record.authority.state {
        AuthorityState::Authorized => {
            let Some(receipt) = record.authority.receipt_sha256.as_deref() else {
                *unknown = true;
                return true;
            };
            if !is_sha256(receipt) {
                *unknown = true;
                return true;
            }
            if record.authority.scope_sha256.as_deref()
                != Some(plan.authorization_scope_sha256.as_str())
            {
                mismatches.insert(MismatchDimension::Authorization);
                return false;
            }
            let Some(validity) = record.authority.valid_window else {
                *unknown = true;
                return true;
            };
            if !validity.is_valid()
                || !validity.contains(TimeWindow {
                    start: record.actual_start,
                    end: record.actual_end,
                })
            {
                mismatches.insert(MismatchDimension::Authorization);
                return false;
            }
            true
        }
        AuthorityState::Unknown => {
            *unknown = true;
            true
        }
        AuthorityState::NotAuthorized | AuthorityState::NotRequired => {
            mismatches.insert(MismatchDimension::Authorization);
            false
        }
    }
}

fn assess_window(
    expected: &ExpectedDataWindow,
    segments: &[DataSegment],
    record: &ExecutionRecord,
    mismatches: &mut BTreeSet<MismatchDimension>,
) -> WindowCoverage {
    let matching_stream: Vec<&DataSegment> = segments
        .iter()
        .filter(|segment| {
            segment.source_id == expected.source_id
                && segment.source_stream_id == expected.source_stream_id
        })
        .collect();

    let mut accepted: Vec<&DataSegment> = Vec::new();
    for segment in matching_stream {
        if segment.window.start < expected.window.start
            || segment.window.end > expected.window.end
        {
            // A segment may not straddle a planned window boundary: its sample
            // count cannot safely be apportioned to the in-scope portion.
            if segment.window.start < expected.window.end
                && expected.window.start < segment.window.end
            {
                mismatches.insert(MismatchDimension::DataStream);
            }
            continue;
        }
        if segment.measurement_profile_sha256 != expected.measurement_profile_sha256
            || segment.time_scale_id != expected.time_scale_id
            || segment.horizontal_frame_id != expected.horizontal_frame_id
            || segment.vertical_datum_id != expected.vertical_datum_id
        {
            mismatches.insert(MismatchDimension::SegmentMeasurementBasis);
            continue;
        }
        accepted.push(segment);
    }

    accepted.sort_by_key(|segment| segment.window.start);
    let mut cursor = expected.window.start;
    let mut samples = 0_u64;
    let mut covered_ticks = 0_u64;
    for segment in accepted {
        if segment.window.start != cursor {
            if segment.window.start > cursor {
                break;
            }
            mismatches.insert(MismatchDimension::DataStream);
            break;
        }
        cursor = segment.window.end;
        covered_ticks = covered_ticks.saturating_add(segment.window.duration());
        samples = samples.saturating_add(segment.sample_count);
    }
    let complete = cursor == expected.window.end && samples >= expected.minimum_samples;
    if record.actual_end < expected.window.end {
        // A shortened run cannot satisfy a window that extends beyond the actual run.
    }
    WindowCoverage {
        window_id: expected.window_id.clone(),
        expected_ticks: expected.window.duration(),
        covered_ticks,
        sample_count: samples,
        complete,
    }
}

/// Compare one actual execution record with the immutable plan and its expected data windows.
///
/// A positive result establishes structural execution conformance only. It does not
/// prove the experiment was physically performed, authenticate the raw bytes, or
/// validate any physical model or engineering conclusion.
pub fn evaluate_execution(
    plan: &ExecutionPlan,
    record: &ExecutionRecord,
    policy: &ExecutionPolicy,
) -> Result<ExecutionAssessment, ExecutionError> {
    validate_plan(plan)?;
    validate_record(record)?;
    if !is_sha256(&policy.policy_sha256)
        || !is_sha256(&policy.required_capture_build_sha256)
    {
        return Err(ExecutionError::InvalidPolicy);
    }

    let mut mismatches = identity_mismatches(&plan.configuration, &record.configuration);
    let mut unknown = false;
    let mut documented_deviation = false;
    let mut outside_authority_envelope = false;

    if record.policy_sha256 != plan.policy_sha256 || plan.policy_sha256 != policy.policy_sha256 {
        mismatches.insert(MismatchDimension::Policy);
    }
    if record.candidate_id != plan.candidate_id
        || record.candidate_sha256 != plan.candidate_sha256
        || record.selection_policy_sha256 != plan.selection_policy_sha256
    {
        mismatches.insert(MismatchDimension::SelectionIdentity);
    }
    if record.design_sha256 != plan.design_sha256 {
        mismatches.insert(MismatchDimension::ExperimentDesign);
    }
    if record.procedure_sha256 != plan.procedure_sha256 {
        mismatches.insert(MismatchDimension::Procedure);
    }
    if record.scenario_manifest_sha256 != plan.scenario_manifest_sha256 {
        mismatches.insert(MismatchDimension::ScenarioManifest);
    }
    if record.measurement_plan_sha256 != plan.measurement_plan_sha256 {
        mismatches.insert(MismatchDimension::MeasurementPlan);
    }
    if !plan.permitted_run_window.contains(TimeWindow {
        start: record.actual_start,
        end: record.actual_end,
    }) {
        mismatches.insert(MismatchDimension::RunTimeWindow);
    }
    if record.capture_build_sha256 != policy.required_capture_build_sha256 {
        mismatches.insert(MismatchDimension::CaptureBuild);
    }
    if record.actual_environment_strata.iter().any(|stratum| {
        !plan
            .allowed_environment_strata
            .iter()
            .any(|allowed| allowed == stratum)
    }) {
        mismatches.insert(MismatchDimension::EnvironmentStratum);
    }

    let authority_ok = evaluate_authority(plan, record, &mut mismatches, &mut unknown);

    let mut deviation_ids: Vec<String> = record
        .deviations
        .iter()
        .map(|deviation| deviation.deviation_id.clone())
        .collect();
    deviation_ids.sort_unstable();
    for deviation in &record.deviations {
        match deviation.severity {
            DeviationSeverity::Documented => documented_deviation = true,
            DeviationSeverity::Unknown => unknown = true,
            DeviationSeverity::OutsideAuthorityEnvelope => {
                outside_authority_envelope = true
            }
        }
    }

    let mut all_segments_valid = BTreeSet::new();
    for expected in &plan.expected_data_windows {
        all_segments_valid.insert((expected.source_id.as_str(), expected.source_stream_id.as_str()));
    }
    for segment in &record.segments {
        if !all_segments_valid.contains(&(
            segment.source_id.as_str(),
            segment.source_stream_id.as_str(),
        )) {
            mismatches.insert(MismatchDimension::DataStream);
        }
    }

    let mut window_coverage: Vec<WindowCoverage> = plan
        .expected_data_windows
        .iter()
        .map(|expected| {
            assess_window(
                expected,
                &record.segments,
                record,
                &mut mismatches,
            )
        })
        .collect();
    window_coverage.sort_by(|left, right| left.window_id.cmp(&right.window_id));

    let mut incomplete_window_ids: Vec<String> = window_coverage
        .iter()
        .filter(|window| !window.complete)
        .map(|window| window.window_id.clone())
        .collect();
    incomplete_window_ids.sort_unstable();

    let total_samples = record
        .segments
        .iter()
        .fold(0_u64, |total, segment| total.saturating_add(segment.sample_count));

    let not_admissible = !mismatches.is_empty() || !authority_ok && plan.authorization_required
        || outside_authority_envelope;
    let aborted = record.abort.is_some();
    let status = if not_admissible {
        ExecutionStatus::NotAdmissibleForClaim
    } else if unknown {
        ExecutionStatus::Indeterminate
    } else if aborted {
        ExecutionStatus::Aborted
    } else if !incomplete_window_ids.is_empty() {
        ExecutionStatus::EvidenceIncomplete
    } else if documented_deviation {
        ExecutionStatus::ExecutionDeviated
    } else {
        ExecutionStatus::ExecutionConforming
    };

    Ok(ExecutionAssessment {
        run_id: record.run_id.clone(),
        candidate_sha256: plan.candidate_sha256.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        status,
        mismatch_dimensions: mismatches.into_iter().collect(),
        deviation_ids,
        incomplete_window_ids,
        full_plan_evidence_complete: status == ExecutionStatus::ExecutionConforming,
        total_samples,
        window_coverage,
        aborted,
    })
}
