use aeroweift_sensor_time_evidence::{
    CaptureWindow, DataRepresentation, SensorCapture, SensorModality, TemporalAlignmentPolicy,
    TemporalDisposition, TemporalEvidenceClass, TemporalGap, TimeInterval, evaluate_cycle,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn window(start: i128, end: i128, uncertainty: i128) -> CaptureWindow {
    CaptureWindow {
        start: TimeInterval {
            lower_ns: start,
            upper_ns: start + uncertainty,
        },
        end: TimeInterval {
            lower_ns: end,
            upper_ns: end + uncertainty,
        },
    }
}

fn sensor(
    id: &str,
    modality: SensorModality,
    sequence: u64,
    capture: Option<CaptureWindow>,
) -> SensorCapture {
    SensorCapture {
        sensor_id: id.into(),
        modality,
        acquisition_cycle_id: "cycle-001".into(),
        file_sequence: sequence,
        representation: DataRepresentation::Raw,
        preprocessing_sha256: None,
        calibration_profile_sha256: Some(digest('c')),
        time_scale_id: Some("TAI".into()),
        clock_domain_id: Some("clock-001".into()),
        hardware_time_reference_sha256: Some(digest('b')),
        clock_drift_bound_ppb: Some(20),
        capture_window: capture,
    }
}

fn hardware_policy() -> TemporalAlignmentPolicy {
    TemporalAlignmentPolicy {
        evidence_class: TemporalEvidenceClass::IndependentHardwareBounded,
        policy_sha256: digest('a'),
        time_scale_id: Some("TAI".into()),
        shared_clock_domain_id: None,
        shared_clock_profile_sha256: None,
        independent_hardware_reference_sha256: Some(digest('b')),
        maximum_pairwise_skew_ns: 1_000,
        maximum_drift_ppb: 100,
    }
}

fn shared_clock_policy() -> TemporalAlignmentPolicy {
    TemporalAlignmentPolicy {
        evidence_class: TemporalEvidenceClass::SharedClockBounded,
        policy_sha256: digest('a'),
        time_scale_id: Some("TAI".into()),
        shared_clock_domain_id: Some("clock-001".into()),
        shared_clock_profile_sha256: Some(digest('d')),
        independent_hardware_reference_sha256: None,
        maximum_pairwise_skew_ns: 1_000,
        maximum_drift_ppb: 50,
    }
}

fn software_cycle_policy() -> TemporalAlignmentPolicy {
    TemporalAlignmentPolicy {
        evidence_class: TemporalEvidenceClass::SoftwareCycleAssociated,
        policy_sha256: digest('a'),
        time_scale_id: None,
        shared_clock_domain_id: None,
        shared_clock_profile_sha256: None,
        independent_hardware_reference_sha256: None,
        maximum_pairwise_skew_ns: 1_000,
        maximum_drift_ppb: 0,
    }
}

#[test]
fn positive_hardware_bounded_alignment_is_eligible() {
    let sensors = vec![
        sensor(
            "cw-1",
            SensorModality::ContinuousWaveRadar,
            20,
            Some(window(0, 100_000, 50)),
        ),
        sensor(
            "fmcw-1",
            SensorModality::FmcwRadar,
            200,
            Some(window(100, 100_100, 50)),
        ),
    ];
    let result = evaluate_cycle(&sensors, &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::BoundedFusionTimeEligible);
    assert_eq!(result.modality_count, 2);
    assert!(result.maximum_pairwise_skew_ns.unwrap() <= 1_000);
}

#[test]
fn shared_clock_within_drift_and_skew_bounds_is_eligible() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        999,
        Some(window(100, 100_100, 10)),
    );
    first.hardware_time_reference_sha256 = None;
    second.hardware_time_reference_sha256 = None;

    let result =
        evaluate_cycle(&[first, second], &shared_clock_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::BoundedFusionTimeEligible);
}

#[test]
fn software_cycle_is_not_promoted_to_bounded_time_fusion() {
    let mut first = sensor("cw-1", SensorModality::ContinuousWaveRadar, 11, None);
    let mut second = sensor("rf-1", SensorModality::RfReceiver, 11, None);
    first.time_scale_id = None;
    second.time_scale_id = None;
    first.clock_domain_id = None;
    second.clock_domain_id = None;
    first.hardware_time_reference_sha256 = None;
    second.hardware_time_reference_sha256 = None;
    first.clock_drift_bound_ppb = None;
    second.clock_drift_bound_ppb = None;

    let result =
        evaluate_cycle(&[first, second], &software_cycle_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::PerCycleAssociationOnly);
    assert!(result.maximum_pairwise_skew_ns.is_none());
}

#[test]
fn order_only_evidence_remains_modality_specific() {
    let mut policy = hardware_policy();
    policy.evidence_class = TemporalEvidenceClass::OrderOnlyOrUnknown;
    policy.time_scale_id = None;
    policy.independent_hardware_reference_sha256 = None;

    let mut first = sensor("cw-1", SensorModality::ContinuousWaveRadar, 10, None);
    let mut second = sensor("rf-1", SensorModality::RfReceiver, 11, None);
    for record in [&mut first, &mut second] {
        record.time_scale_id = None;
        record.clock_domain_id = None;
        record.hardware_time_reference_sha256 = None;
        record.clock_drift_bound_ppb = None;
    }

    let result = evaluate_cycle(&[first, second], &policy).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::ModalitySpecificOnly);
    assert!(result.gaps.contains(&TemporalGap::TimeRelationUnknown));
}

#[test]
fn file_sequence_changes_do_not_change_temporal_qualification() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "fmcw-1",
        SensorModality::FmcwRadar,
        2,
        Some(window(20, 100_020, 10)),
    );
    let original = evaluate_cycle(&[first.clone(), second.clone()], &hardware_policy()).unwrap();
    first.file_sequence = 99_999;
    second.file_sequence = 3;
    let changed = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(original, changed);
}

#[test]
fn mismatched_hardware_reference_is_indeterminate() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    second.hardware_time_reference_sha256 = Some(digest('e'));

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::HardwareReferenceMissingOrMismatch));
}

#[test]
fn time_scale_mismatch_is_indeterminate() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    second.time_scale_id = Some("UTC".into());

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::TimeScaleMissingOrMismatch));
}

#[test]
fn missing_capture_window_is_indeterminate() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor("rf-1", SensorModality::RfReceiver, 2, None);

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::CaptureWindowMissing));
}

#[test]
fn excessive_capture_skew_is_indeterminate() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10_000, 110_000, 10)),
    );

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::PairwiseSkewExceedsBound));
}

#[test]
fn shared_clock_domain_mismatch_is_indeterminate() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    first.hardware_time_reference_sha256 = None;
    second.hardware_time_reference_sha256 = None;
    second.clock_domain_id = Some("other-clock".into());

    let result = evaluate_cycle(&[first, second], &shared_clock_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::SharedClockMissingOrMismatch));
}

#[test]
fn shared_clock_drift_over_policy_is_indeterminate() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    first.hardware_time_reference_sha256 = None;
    second.hardware_time_reference_sha256 = None;
    second.clock_drift_bound_ppb = Some(51);

    let result = evaluate_cycle(&[first, second], &shared_clock_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::SharedClockMissingOrMismatch));
}

#[test]
fn calibration_gap_does_not_falsify_an_independent_clock_relation() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    first.calibration_profile_sha256 = None;

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::BoundedFusionTimeEligible);
    assert!(result.gaps.contains(&TemporalGap::CalibrationProfileMissing));
}

#[test]
fn processed_data_requires_exact_preprocessing_identity() {
    let mut processed = sensor(
        "fmcw-1",
        SensorModality::FmcwRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    processed.representation = DataRepresentation::Processed;
    processed.preprocessing_sha256 = None;

    assert_eq!(
        evaluate_cycle(&[processed], &hardware_policy()),
        Err(aeroweift_sensor_time_evidence::TemporalError::InvalidSensorCapture)
    );
}

#[test]
fn raw_data_cannot_claim_a_preprocessing_digest() {
    let mut raw = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    raw.preprocessing_sha256 = Some(digest('d'));

    assert_eq!(
        evaluate_cycle(&[raw], &hardware_policy()),
        Err(aeroweift_sensor_time_evidence::TemporalError::InvalidSensorCapture)
    );
}

#[test]
fn malformed_capture_window_is_indeterminate() {
    let mut invalid = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(100, 50, 0)),
    );
    invalid.capture_window = Some(CaptureWindow {
        start: TimeInterval {
            lower_ns: 10,
            upper_ns: 5,
        },
        end: TimeInterval {
            lower_ns: 50,
            upper_ns: 60,
        },
    });
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_000, 0)),
    );

    let result = evaluate_cycle(&[invalid, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::InvalidCaptureWindow));
}

#[test]
fn mismatched_acquisition_cycle_is_indeterminate() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    second.acquisition_cycle_id = "cycle-002".into();

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::AcquisitionCycleMismatch));
}

#[test]
fn duplicate_sensor_id_is_a_structural_error() {
    let first = sensor(
        "same-id",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "same-id",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );

    assert_eq!(
        evaluate_cycle(&[first, second], &hardware_policy()),
        Err(aeroweift_sensor_time_evidence::TemporalError::DuplicateSensorId)
    );
}

#[test]
fn empty_or_invalid_policy_fails_closed() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let mut invalid = hardware_policy();
    invalid.policy_sha256 = "not-a-digest".into();

    assert_eq!(
        evaluate_cycle(&[first], &invalid),
        Err(aeroweift_sensor_time_evidence::TemporalError::InvalidPolicy)
    );
}

#[test]
fn sensor_input_permutation_preserves_assessment() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    let expected =
        evaluate_cycle(&[first.clone(), second.clone()], &hardware_policy()).unwrap();
    let reversed = evaluate_cycle(&[second, first], &hardware_policy()).unwrap();
    assert_eq!(expected, reversed);
}


#[test]
fn one_sensor_remains_modality_specific_not_fusion_qualified() {
    let only = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let result = evaluate_cycle(&[only], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::ModalitySpecificOnly);
    assert!(result.gaps.contains(&TemporalGap::InsufficientSensors));
}

#[test]
fn missing_calibration_does_not_create_a_fusion_time_claim_gap() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    first.calibration_profile_sha256 = None;

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::BoundedFusionTimeEligible);
    assert!(result.gaps.contains(&TemporalGap::CalibrationProfileMissing));
}


#[test]
fn one_sensor_cannot_claim_cross_modal_time_qualification() {
    let only = sensor(
        "cw-only",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let result = evaluate_cycle(&[only], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::ModalitySpecificOnly);
    assert!(result.gaps.contains(&TemporalGap::InsufficientSensors));
}

#[test]
fn overlapping_start_and_end_uncertainty_is_not_a_valid_capture_window() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    first.capture_window = Some(CaptureWindow {
        start: TimeInterval {
            lower_ns: 0,
            upper_ns: 100,
        },
        end: TimeInterval {
            lower_ns: 50,
            upper_ns: 150,
        },
    });
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(200, 100_200, 10)),
    );

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::Indeterminate);
    assert!(result.gaps.contains(&TemporalGap::InvalidCaptureWindow));
}

#[test]
fn missing_calibration_is_separate_from_valid_clock_alignment() {
    let mut first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );
    first.calibration_profile_sha256 = None;

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert_eq!(result.disposition, TemporalDisposition::BoundedFusionTimeEligible);
    assert!(!result.measurement_metadata_complete);
    assert!(result.gaps.contains(&TemporalGap::CalibrationProfileMissing));
}

#[test]
fn complete_calibration_metadata_is_reported_separately() {
    let first = sensor(
        "cw-1",
        SensorModality::ContinuousWaveRadar,
        1,
        Some(window(0, 100_000, 10)),
    );
    let second = sensor(
        "rf-1",
        SensorModality::RfReceiver,
        2,
        Some(window(10, 100_010, 10)),
    );

    let result = evaluate_cycle(&[first, second], &hardware_policy()).unwrap();
    assert!(result.measurement_metadata_complete);
    assert_eq!(result.disposition, TemporalDisposition::BoundedFusionTimeEligible);
}
