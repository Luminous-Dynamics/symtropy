use aeroweift_execution_fidelity::{
    AbortReason, AbortRecord, AuthorityEvidence, AuthorityState, ConfigurationIdentity,
    DataSegment, DeviationKind, DeviationSeverity, ExecutionDeviation, ExecutionError,
    ExecutionPlan, ExecutionPolicy, ExecutionRecord, ExecutionStatus, ExpectedDataWindow,
    MismatchDimension, TimeWindow, evaluate_execution,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn interval(start: i64, end: i64) -> TimeWindow {
    TimeWindow { start, end }
}

fn configuration() -> ConfigurationIdentity {
    ConfigurationIdentity {
        aircraft_configuration_sha256: digest('a'),
        model_artifact_sha256: digest('b'),
        control_stack_sha256: digest('c'),
        sensor_stack_sha256: digest('d'),
        solver_configuration_sha256: digest('e'),
        reference_measurement_sha256: digest('f'),
        operating_domain_sha256: digest('1'),
        time_scale_id: "UTC-SYNTHETIC".into(),
        horizontal_frame_id: "LOCAL-ENU-01".into(),
        vertical_datum_id: "SYNTHETIC-VD-01".into(),
    }
}

fn plan() -> ExecutionPlan {
    ExecutionPlan {
        candidate_id: "candidate-001".into(),
        candidate_sha256: digest('3'),
        selection_policy_sha256: digest('4'),
        design_sha256: digest('5'),
        procedure_sha256: digest('6'),
        scenario_manifest_sha256: digest('7'),
        measurement_plan_sha256: digest('8'),
        configuration: configuration(),
        allowed_environment_strata: vec!["crosswind-high".into()],
        authorization_required: true,
        authorization_scope_sha256: digest('2'),
        expected_authorization_receipt_sha256: Some(digest('a')),
        permitted_run_window: interval(0, 40),
        expected_data_windows: vec![ExpectedDataWindow {
            window_id: "window-telemetry-001".into(),
            source_id: "sensor-source-01".into(),
            source_stream_id: "sensor-stream-01".into(),
            window: interval(10, 30),
            minimum_samples: 20,
            measurement_profile_sha256: digest('9'),
            calibration_receipt_sha256: digest('0'),
            time_scale_id: "UTC-SYNTHETIC".into(),
            horizontal_frame_id: "LOCAL-ENU-01".into(),
            vertical_datum_id: "SYNTHETIC-VD-01".into(),
        }],
        policy_sha256: digest('f'),
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        policy_sha256: digest('f'),
        required_capture_build_sha256: digest('e'),
    }
}

fn segment(id: &str, sequence: u64, start: i64, end: i64, samples: u64) -> DataSegment {
    DataSegment {
        segment_id: id.into(),
        source_id: "sensor-source-01".into(),
        source_stream_id: "sensor-stream-01".into(),
        source_sequence: sequence,
        window: interval(start, end),
        sample_count: samples,
        payload_sha256: digest('a'),
        raw_data_manifest_sha256: digest('b'),
        capture_receipt_sha256: digest('c'),
        calibration_receipt_sha256: digest('0'),
        measurement_profile_sha256: digest('9'),
        calibration_receipt_sha256: digest('0'),
        time_scale_id: "UTC-SYNTHETIC".into(),
        horizontal_frame_id: "LOCAL-ENU-01".into(),
        vertical_datum_id: "SYNTHETIC-VD-01".into(),
    }
}

fn record() -> ExecutionRecord {
    ExecutionRecord {
        run_id: "run-001".into(),
        candidate_id: "candidate-001".into(),
        candidate_sha256: digest('3'),
        selection_policy_sha256: digest('4'),
        design_sha256: digest('5'),
        procedure_sha256: digest('6'),
        scenario_manifest_sha256: digest('7'),
        measurement_plan_sha256: digest('8'),
        configuration: configuration(),
        actual_environment_strata: vec!["crosswind-high".into()],
        actual_start: 5,
        actual_end: 35,
        authority: AuthorityEvidence {
            state: AuthorityState::Authorized,
            receipt_sha256: Some(digest('a')),
            scope_sha256: Some(digest('2')),
            valid_window: Some(interval(0, 40)),
        },
        deviations: vec![],
        abort: None,
        segments: vec![segment("segment-001", 1, 10, 30, 20)],
        capture_build_sha256: digest('e'),
        policy_sha256: digest('f'),
    }
}

fn assess(
    plan: &ExecutionPlan,
    record: &ExecutionRecord,
) -> Result<aeroweift_execution_fidelity::ExecutionAssessment, ExecutionError> {
    evaluate_execution(plan, record, &policy())
}

#[test]
fn exact_positive_control_is_execution_conforming() {
    let result = assess(&plan(), &record()).unwrap();
    assert_eq!(result.status, ExecutionStatus::ExecutionConforming);
    assert!(result.full_plan_evidence_complete);
    assert!(result.mismatch_dimensions.is_empty());
    assert!(result.deviation_ids.is_empty());
    assert!(result.incomplete_window_ids.is_empty());
    assert_eq!(result.window_coverage[0].covered_ticks, 20);
    assert_eq!(result.window_coverage[0].sample_count, 20);
}

#[test]
fn contiguous_segments_combine_to_cover_one_expected_window() {
    let mut actual = record();
    actual.segments = vec![
        segment("segment-a", 1, 10, 20, 10),
        segment("segment-b", 2, 20, 30, 10),
    ];
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::ExecutionConforming);
    assert_eq!(result.window_coverage[0].covered_ticks, 20);
    assert_eq!(result.total_samples, 20);
}

#[test]
fn candidate_identity_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.candidate_id = "other-candidate".into();
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::SelectionIdentity)
    );
}

#[test]
fn candidate_payload_digest_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.candidate_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::SelectionIdentity)
    );
}

#[test]
fn track_v_selection_policy_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.selection_policy_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::SelectionIdentity)
    );
}

#[test]
fn experiment_design_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.design_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::ExperimentDesign)
    );
}

#[test]
fn procedure_revision_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.procedure_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::Procedure)
    );
}

#[test]
fn scenario_or_initial_condition_manifest_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.scenario_manifest_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::ScenarioManifest)
    );
}

#[test]
fn measurement_plan_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.measurement_plan_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::MeasurementPlan)
    );
}

#[test]
fn actual_model_artifact_change_cannot_inherit_plan_credit() {
    let mut actual = record();
    actual.configuration.model_artifact_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::ModelArtifact)
    );
}

#[test]
fn sensor_recalibration_during_run_invalidates_the_planned_measurement_scope() {
    let mut actual = record();
    actual.configuration.sensor_stack_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::SensorStack)
    );
}

#[test]
fn changed_control_stack_cannot_inherit_execution_credit() {
    let mut actual = record();
    actual.configuration.control_stack_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::ControlStack)
    );
}

#[test]
fn changed_reference_measurement_profile_cannot_be_silently_accepted() {
    let mut actual = record();
    actual.configuration.reference_measurement_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::ReferenceMeasurement)
    );
}

#[test]
fn unknown_authority_state_is_indeterminate_not_positive_or_denied() {
    let mut actual = record();
    actual.authority = AuthorityEvidence {
        state: AuthorityState::Unknown,
        receipt_sha256: None,
        scope_sha256: None,
        valid_window: None,
    };
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::Indeterminate);
}

#[test]
fn missing_authority_receipt_is_indeterminate() {
    let mut actual = record();
    actual.authority.receipt_sha256 = None;
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::Indeterminate);
}

#[test]
fn explicitly_denied_authority_is_not_admissible() {
    let mut actual = record();
    actual.authority.state = AuthorityState::NotAuthorized;
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::Authorization)
    );
}

#[test]
fn missing_required_authorization_cannot_be_relabelled_not_required() {
    let mut actual = record();
    actual.authority.state = AuthorityState::NotRequired;
    actual.authority.receipt_sha256 = None;
    actual.authority.scope_sha256 = None;
    actual.authority.valid_window = None;
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
}

#[test]
fn authorization_expiry_mid_run_is_not_admissible() {
    let mut actual = record();
    actual.authority.valid_window = Some(interval(0, 20));
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::Authorization)
    );
}

#[test]
fn authorization_scope_must_match_the_exact_experiment_scope() {
    let mut actual = record();
    actual.authority.scope_sha256 = Some(digest('0'));
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::Authorization)
    );
}

#[test]
fn actual_run_outside_the_permitted_time_window_is_not_admissible() {
    let mut actual = record();
    actual.actual_end = 45;
    actual.segments[0].window = interval(10, 30);
    actual.authority.valid_window = Some(interval(0, 50));
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::RunTimeWindow)
    );
}

#[test]
fn unexpected_environment_stratum_is_not_admissible() {
    let mut actual = record();
    actual.actual_environment_strata = vec!["unplanned-icing".into()];
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::EnvironmentStratum)
    );
}

#[test]
fn documented_noncritical_deviation_is_retained_and_changes_status() {
    let mut actual = record();
    actual.deviations.push(ExecutionDeviation {
        deviation_id: "operator-override-1".into(),
        kind: DeviationKind::OperatorOverride,
        severity: DeviationSeverity::Documented,
        effective_window: Some(interval(15, 16)),
        record_sha256: digest('b'),
    });
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::ExecutionDeviated);
    assert_eq!(
        result.deviation_ids,
        vec!["operator-override-1".to_string()]
    );
    assert!(!result.full_plan_evidence_complete);
}

#[test]
fn unknown_deviation_impact_is_indeterminate() {
    let mut actual = record();
    actual.deviations.push(ExecutionDeviation {
        deviation_id: "unknown-effect-1".into(),
        kind: DeviationKind::Other,
        severity: DeviationSeverity::Unknown,
        effective_window: Some(interval(15, 16)),
        record_sha256: digest('b'),
    });
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::Indeterminate);
}

#[test]
fn deviation_outside_authority_envelope_is_not_admissible() {
    let mut actual = record();
    actual.deviations.push(ExecutionDeviation {
        deviation_id: "out-of-envelope-1".into(),
        kind: DeviationKind::Environment,
        severity: DeviationSeverity::OutsideAuthorityEnvelope,
        effective_window: Some(interval(15, 16)),
        record_sha256: digest('b'),
    });
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
}

#[test]
fn aborted_run_is_never_marked_as_a_completed_full_plan_execution() {
    let mut actual = record();
    actual.abort = Some(AbortRecord {
        aborted_at: 25,
        reason: AbortReason::Safety,
        record_sha256: digest('c'),
    });
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::Aborted);
    assert!(result.aborted);
    assert!(!result.full_plan_evidence_complete);
}

#[test]
fn missing_data_segment_makes_full_plan_evidence_incomplete() {
    let mut actual = record();
    actual.segments.clear();
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::EvidenceIncomplete);
    assert_eq!(
        result.incomplete_window_ids,
        vec!["window-telemetry-001".to_string()]
    );
    assert_eq!(result.total_samples, 0);
}

#[test]
fn sample_count_below_the_frozen_floor_is_incomplete() {
    let mut actual = record();
    actual.segments[0].sample_count = 19;
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::EvidenceIncomplete);
    assert_eq!(result.window_coverage[0].sample_count, 19);
}

#[test]
fn segment_measurement_profile_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.segments[0].measurement_profile_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::SegmentMeasurementBasis)
    );
}

#[test]
fn segment_time_frame_or_datum_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.segments[0].horizontal_frame_id = "OTHER-FRAME".into();
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::SegmentMeasurementBasis)
    );
}

#[test]
fn duplicate_segment_identity_is_rejected() {
    let mut actual = record();
    actual.segments.push(actual.segments[0].clone());
    assert_eq!(
        assess(&plan(), &actual),
        Err(ExecutionError::DuplicateSegmentId)
    );
}

#[test]
fn duplicate_source_sequence_is_rejected_as_a_fork() {
    let mut actual = record();
    actual.segments = vec![
        segment("segment-a", 1, 10, 20, 10),
        segment("segment-b", 1, 20, 30, 10),
    ];
    assert_eq!(
        assess(&plan(), &actual),
        Err(ExecutionError::ForkedSegmentSequence)
    );
}

#[test]
fn overlapping_segments_cannot_double_count_time_or_samples() {
    let mut actual = record();
    actual.segments = vec![
        segment("segment-a", 1, 10, 21, 12),
        segment("segment-b", 2, 20, 30, 12),
    ];
    assert_eq!(
        assess(&plan(), &actual),
        Err(ExecutionError::OverlappingSegments)
    );
}

#[test]
fn source_sequence_time_inversion_is_rejected() {
    let mut actual = record();
    actual.segments = vec![
        segment("segment-a", 1, 20, 30, 10),
        segment("segment-b", 2, 10, 20, 10),
    ];
    assert_eq!(
        assess(&plan(), &actual),
        Err(ExecutionError::SegmentSequenceTimeInversion)
    );
}

#[test]
fn unexpected_raw_data_stream_is_not_admissible_for_the_plan() {
    let mut actual = record();
    actual.segments[0].source_stream_id = "unplanned-stream".into();
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::DataStream)
    );
}

#[test]
fn raw_segment_outside_actual_run_interval_is_rejected() {
    let mut actual = record();
    actual.segments[0].window = interval(0, 30);
    assert_eq!(
        assess(&plan(), &actual),
        Err(ExecutionError::InvalidSegment)
    );
}

#[test]
fn execution_policy_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.policy_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::Policy)
    );
}

#[test]
fn capture_build_mismatch_is_not_admissible() {
    let mut actual = record();
    actual.capture_build_sha256 = digest('0');
    let result = assess(&plan(), &actual).unwrap();
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::CaptureBuild)
    );
}

#[test]
fn removing_observation_segments_cannot_increase_coverage() {
    let baseline = assess(&plan(), &record()).unwrap();
    let mut reduced_record = record();
    reduced_record.segments[0].window = interval(10, 20);
    reduced_record.segments[0].sample_count = 10;
    let reduced = assess(&plan(), &reduced_record).unwrap();
    assert_eq!(baseline.window_coverage[0].covered_ticks, 20);
    assert_eq!(reduced.window_coverage[0].covered_ticks, 10);
    assert_eq!(baseline.status, ExecutionStatus::ExecutionConforming);
    assert_eq!(reduced.status, ExecutionStatus::EvidenceIncomplete);
}

#[test]
fn record_and_segment_permutation_preserves_canonical_assessment() {
    let mut expected_plan = plan();
    expected_plan.expected_data_windows.push(ExpectedDataWindow {
        window_id: "window-second-channel".into(),
        source_id: "sensor-source-02".into(),
        source_stream_id: "sensor-stream-02".into(),
        window: interval(10, 30),
        minimum_samples: 20,
        measurement_profile_sha256: digest('9'),
        time_scale_id: "UTC-SYNTHETIC".into(),
        horizontal_frame_id: "LOCAL-ENU-01".into(),
        vertical_datum_id: "SYNTHETIC-VD-01".into(),
    });
    let mut actual = record();
    let mut second = segment("segment-002", 1, 10, 30, 20);
    second.source_id = "sensor-source-02".into();
    second.source_stream_id = "sensor-stream-02".into();
    actual.segments.push(second);
    let expected = assess(&expected_plan, &actual).unwrap();

    actual.segments.reverse();
    actual.deviations.reverse();
    let actual_result = assess(&expected_plan, &actual).unwrap();
    assert_eq!(actual_result, expected);
}

#[test]
fn capture_segment_must_bind_the_planned_calibration_receipt() {
    let mut actual = record();
    actual.segments[0].calibration_receipt_sha256 = digest('1');
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::CalibrationEvidence)
    );
}

#[test]
fn segments_for_one_window_must_share_the_exact_raw_data_manifest() {
    let mut actual = record();
    let mut second = segment("segment-002", 2, 20, 30, 10);
    second.raw_data_manifest_sha256 = digest('1');
    actual.segments = vec![segment("segment-001", 1, 10, 20, 10), second];
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::RawDataManifest)
    );
}

#[test]
fn capture_receipt_digest_is_required_for_every_raw_segment() {
    let mut actual = record();
    actual.segments[0].capture_receipt_sha256 = "not-a-digest".into();
    assert_eq!(
        assess(&plan(), &actual),
        Err(ExecutionError::InvalidSegment)
    );
}

#[test]
fn authorization_receipt_substitution_is_not_admissible_even_when_scope_matches() {
    let mut actual = record();
    actual.authority.receipt_sha256 = Some(digest('0'));
    let result = assess(&plan(), &actual).unwrap();
    assert_eq!(result.status, ExecutionStatus::NotAdmissibleForClaim);
    assert!(
        result
            .mismatch_dimensions
            .contains(&MismatchDimension::Authorization)
    );
}
