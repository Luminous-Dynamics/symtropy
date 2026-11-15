use maritime_manoeuvring_contract::{
    ContextOfUse, EvidenceRef, EvidenceRole, EvidenceState, ManifestDisposition,
    ManoeuvringManifest, Trajectory, TrajectoryError, TrajectorySample, ZigZagCommand,
    stopping_metrics, turning_circle_metrics, zig_zag_metrics,
};

fn sample(time_s: f64, x_m: f64, y_m: f64, heading_rad: f64) -> TrajectorySample {
    TrajectorySample {
        time_s,
        x_m,
        y_m,
        heading_rad,
        surge_mps: 5.0,
        sway_mps: 0.0,
        yaw_rate_rad_s: 0.0,
    }
}

fn positive_heading_turn() -> Trajectory {
    let headings: [f64; 5] = [0.0, 45.0, 90.0, 135.0, 180.0];
    let samples = headings
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            let angle = heading.to_radians();
            sample(
                index as f64,
                10.0 * angle.sin(),
                10.0 * (1.0 - angle.cos()),
                angle,
            )
        })
        .collect();
    Trajectory {
        vessel_id: "synthetic-vessel-01".to_owned(),
        vessel_length_m: 100.0,
        samples,
    }
}

fn evidence(role: EvidenceRole, id: &str, state: EvidenceState) -> EvidenceRef {
    EvidenceRef {
        id: id.to_owned(),
        case_id: "synthetic-case-01".to_owned(),
        role,
        source_id: format!("synthetic-source-{id}"),
        source_revision: "fixture-v1".to_owned(),
        sha256: "a".repeat(64),
        published_at_s: 0.0,
        valid_from_s: 0.0,
        valid_until_s: 20.0,
        state,
    }
}

fn manifest(context: ContextOfUse) -> ManoeuvringManifest {
    let roles = context.required_roles();
    ManoeuvringManifest {
        case_id: "synthetic-case-01".to_owned(),
        context,
        decision_cutoff_s: 1.0,
        operation_start_s: 2.0,
        operation_end_s: 10.0,
        evidence: roles
            .iter()
            .enumerate()
            .map(|(index, role)| evidence(*role, &format!("e-{index}"), EvidenceState::Reviewed))
            .collect(),
    }
}

#[test]
fn positive_heading_turn_metrics_match_hand_calculated_circle() {
    let result = turning_circle_metrics(&positive_heading_turn()).expect("valid turning circle");
    assert!((result.advance_at_90_m - 10.0).abs() < 1.0e-9);
    assert!((result.transfer_at_90_m - 10.0).abs() < 1.0e-9);
    assert!((result.tactical_diameter_at_180_m - 20.0).abs() < 1.0e-9);
    assert!((result.time_to_90_s - 2.0).abs() < 1.0e-9);
    assert!((result.time_to_180_s - 4.0).abs() < 1.0e-9);
}

#[test]
fn negative_heading_turn_reports_signed_transfer_and_positive_tactical_diameter() {
    let headings: [f64; 5] = [0.0, -45.0, -90.0, -135.0, -180.0];
    let samples = headings
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            let angle = heading.to_radians();
            sample(
                index as f64,
                -10.0 * angle.sin(),
                -10.0 * (1.0 - angle.cos()),
                angle,
            )
        })
        .collect();
    let trajectory = Trajectory {
        vessel_id: "left-turn".to_owned(),
        vessel_length_m: 100.0,
        samples,
    };
    let result = turning_circle_metrics(&trajectory).expect("valid left turn");
    assert!((result.advance_at_90_m - 10.0).abs() < 1.0e-9);
    assert!((result.transfer_at_90_m + 10.0).abs() < 1.0e-9);
    assert!((result.tactical_diameter_at_180_m - 20.0).abs() < 1.0e-9);
}

#[test]
fn wrapped_heading_samples_are_unwrapped_before_crossing_detection() {
    let base = 350.0_f64.to_radians();
    let offsets: [f64; 5] = [0.0, 45.0, 90.0, 135.0, 180.0];
    let samples = offsets
        .iter()
        .enumerate()
        .map(|(index, offset)| {
            let relative = offset.to_radians();
            let absolute = (base + relative).rem_euclid(std::f64::consts::TAU);
            let local_x = 10.0 * relative.sin();
            let local_y = 10.0 * (1.0 - relative.cos());
            let x = local_x * base.cos() - local_y * base.sin();
            let y = local_x * base.sin() + local_y * base.cos();
            sample(index as f64, x, y, absolute)
        })
        .collect();
    let trajectory = Trajectory {
        vessel_id: "wrapped-heading".to_owned(),
        vessel_length_m: 100.0,
        samples,
    };
    let result = turning_circle_metrics(&trajectory).expect("wrapped heading accepted");
    assert!((result.advance_at_90_m - 10.0).abs() < 1.0e-9);
    assert!((result.transfer_at_90_m - 10.0).abs() < 1.0e-9);
}

#[test]
fn turning_circle_rejects_missing_180_degree_crossing() {
    let mut trajectory = positive_heading_turn();
    trajectory.samples.pop();
    assert_eq!(
        turning_circle_metrics(&trajectory),
        Err(TrajectoryError::HeadingTargetNotReached(180))
    );
}

#[test]
fn turning_circle_rejects_ambiguous_heading_jump() {
    let mut trajectory = positive_heading_turn();
    trajectory.samples[1].heading_rad = std::f64::consts::PI;
    assert_eq!(
        turning_circle_metrics(&trajectory),
        Err(TrajectoryError::AmbiguousHeadingIncrement(1))
    );
}

#[test]
fn stopping_metrics_interpolate_first_threshold_crossing() {
    let trajectory = Trajectory {
        vessel_id: "stopping-vessel".to_owned(),
        vessel_length_m: 100.0,
        samples: vec![
            TrajectorySample {
                time_s: 0.0,
                x_m: 0.0,
                y_m: 0.0,
                heading_rad: 0.0,
                surge_mps: 10.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
            TrajectorySample {
                time_s: 1.0,
                x_m: 9.0,
                y_m: 0.0,
                heading_rad: 0.0,
                surge_mps: 8.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
            TrajectorySample {
                time_s: 2.0,
                x_m: 14.0,
                y_m: 0.0,
                heading_rad: 0.0,
                surge_mps: 2.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
            TrajectorySample {
                time_s: 3.0,
                x_m: 15.0,
                y_m: 0.0,
                heading_rad: 0.0,
                surge_mps: 0.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
        ],
    };
    let result = stopping_metrics(&trajectory, 0, 1.0).expect("stop crossing");
    assert!((result.time_from_command_s - 2.5).abs() < 1.0e-9);
    assert!((result.along_track_displacement_m - 14.5).abs() < 1.0e-9);
    assert!(result.cross_track_displacement_m.abs() < 1.0e-9);
}

#[test]
fn stopping_metrics_uses_command_sample_not_trajectory_origin() {
    let trajectory = Trajectory {
        vessel_id: "stop-after-turn".to_owned(),
        vessel_length_m: 50.0,
        samples: vec![
            TrajectorySample {
                time_s: 0.0,
                x_m: 0.0,
                y_m: 0.0,
                heading_rad: 0.0,
                surge_mps: 10.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
            TrajectorySample {
                time_s: 1.0,
                x_m: 10.0,
                y_m: 0.0,
                heading_rad: 0.0,
                surge_mps: 8.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
            TrajectorySample {
                time_s: 2.0,
                x_m: 10.0,
                y_m: 5.0,
                heading_rad: std::f64::consts::FRAC_PI_2,
                surge_mps: 4.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
            TrajectorySample {
                time_s: 3.0,
                x_m: 10.0,
                y_m: 7.0,
                heading_rad: std::f64::consts::FRAC_PI_2,
                surge_mps: 0.0,
                sway_mps: 0.0,
                yaw_rate_rad_s: 0.0,
            },
        ],
    };
    let result = stopping_metrics(&trajectory, 1, 1.0).expect("stop after command");
    assert!(result.along_track_displacement_m.abs() < 1.0e-9);
    assert!((result.cross_track_displacement_m - 6.5).abs() < 1.0e-9);
}

#[test]
fn stopping_metrics_rejects_bad_threshold_and_missing_stop() {
    let trajectory = positive_heading_turn();
    assert_eq!(
        stopping_metrics(&trajectory, 0, 0.0),
        Err(TrajectoryError::InvalidSpeedThreshold)
    );
    assert_eq!(
        stopping_metrics(&trajectory, 0, 0.1),
        Err(TrajectoryError::StopThresholdNotReached)
    );
}

#[test]
fn stopping_metrics_rejects_invalid_command_index() {
    assert_eq!(
        stopping_metrics(&positive_heading_turn(), 100, 0.5),
        Err(TrajectoryError::InvalidCommandIndex)
    );
}

#[test]
fn manifest_is_ready_only_when_context_roles_are_reviewed_and_current() {
    let report = manifest(ContextOfUse::TugAssisted)
        .evaluate()
        .expect("valid manifest");
    assert_eq!(report.disposition, ManifestDisposition::ReadyForComparison);
    assert!(report.missing_roles.is_empty());
    assert!(report.unresolved_roles.is_empty());
}

#[test]
fn manifest_does_not_infer_missing_shallow_water_evidence() {
    let mut case = manifest(ContextOfUse::ShallowWater);
    case.evidence
        .retain(|item| item.role != EvidenceRole::Bathymetry);
    let report = case.evaluate().expect("structural manifest");
    assert_eq!(report.disposition, ManifestDisposition::NeedsEvidence);
    assert_eq!(report.missing_roles, vec![EvidenceRole::Bathymetry]);
}

#[test]
fn manifest_rejects_evidence_published_after_decision_cutoff() {
    let mut case = manifest(ContextOfUse::DeepWaterStandard);
    case.evidence[0].published_at_s = 1.5;
    assert_eq!(
        case.evaluate(),
        Err(maritime_manoeuvring_contract::ManifestError::FutureEvidence("e-0".to_owned()))
    );
}

#[test]
fn stale_unknown_and_unsupported_are_not_promoted_to_ready() {
    let mut stale = manifest(ContextOfUse::DeepWaterStandard);
    stale.evidence[0].state = EvidenceState::Stale;
    assert_eq!(
        stale.evaluate().expect("valid stale record").disposition,
        ManifestDisposition::NeedsEvidence
    );

    let mut unknown = manifest(ContextOfUse::DeepWaterStandard);
    unknown.evidence[0].state = EvidenceState::Unknown;
    assert_eq!(
        unknown
            .evaluate()
            .expect("valid unknown record")
            .disposition,
        ManifestDisposition::NeedsEvidence
    );

    let mut unsupported = manifest(ContextOfUse::DeepWaterStandard);
    unsupported.evidence[0].state = EvidenceState::Unsupported;
    assert_eq!(
        unsupported
            .evaluate()
            .expect("valid unsupported record")
            .disposition,
        ManifestDisposition::Unsupported
    );
}

#[test]
fn manifest_rejects_malformed_digest_and_duplicate_roles() {
    let mut malformed = manifest(ContextOfUse::DeepWaterStandard);
    malformed.evidence[0].sha256 = "not-a-digest".to_owned();
    assert!(malformed.evaluate().is_err());

    let mut duplicate = manifest(ContextOfUse::DeepWaterStandard);
    duplicate.evidence.push(evidence(
        EvidenceRole::VesselConfiguration,
        "duplicate-role",
        EvidenceState::Reviewed,
    ));
    assert!(duplicate.evaluate().is_err());
}

#[test]
fn manifest_rejects_evidence_that_does_not_cover_the_operation_window() {
    let mut case = manifest(ContextOfUse::ShallowWater);
    case.evidence
        .iter_mut()
        .find(|item| item.role == EvidenceRole::Bathymetry)
        .expect("bathymetry fixture")
        .valid_until_s = 9.0;
    let report = case.evaluate().expect("well-formed but stale evidence");
    assert_eq!(report.disposition, ManifestDisposition::NeedsEvidence);
    assert_eq!(report.unresolved_roles, vec![EvidenceRole::Bathymetry]);
}

#[test]
fn manifest_rejects_evidence_bound_to_a_different_case() {
    let mut case = manifest(ContextOfUse::DeepWaterStandard);
    case.evidence[0].case_id = "other-case".to_owned();
    assert_eq!(
        case.evaluate(),
        Err(maritime_manoeuvring_contract::ManifestError::EvidenceCaseMismatch("e-0".to_owned()))
    );
}

fn zig_zag_trajectory() -> Trajectory {
    let records: [(f64, f64, f64); 14] = [
        (0.0, 0.0, 3.0),
        (1.0, 5.0, 3.0),
        (2.0, 10.0, 2.0),
        (3.0, 14.0, 1.0),
        (4.0, 15.0, 0.0),
        (5.0, 14.0, -1.0),
        (6.0, 10.0, -2.0),
        (7.0, 0.0, -3.0),
        (8.0, -8.0, -3.0),
        (9.0, -10.0, -2.0),
        (10.0, -14.0, -1.0),
        (11.0, -16.0, 0.0),
        (12.0, -15.0, 1.0),
        (13.0, -10.0, 2.0),
    ];
    let samples = records
        .iter()
        .map(|(time_s, heading_deg, yaw_rate_deg_s)| TrajectorySample {
            time_s: *time_s,
            x_m: 0.0,
            y_m: 0.0,
            heading_rad: heading_deg.to_radians(),
            surge_mps: 5.0,
            sway_mps: 0.0,
            yaw_rate_rad_s: yaw_rate_deg_s.to_radians(),
        })
        .collect();
    Trajectory {
        vessel_id: "zig-zag-vessel".to_owned(),
        vessel_length_m: 100.0,
        samples,
    }
}

fn zig_zag_commands() -> [ZigZagCommand; 3] {
    [
        ZigZagCommand {
            time_s: 0.0,
            rudder_deg: 10.0,
        },
        ZigZagCommand {
            time_s: 2.0,
            rudder_deg: -10.0,
        },
        ZigZagCommand {
            time_s: 9.0,
            rudder_deg: 10.0,
        },
    ]
}

#[test]
fn ten_ten_zig_zag_overshoots_match_hand_checkable_fixture() {
    let metrics = zig_zag_metrics(&zig_zag_trajectory(), &zig_zag_commands(), 10.0)
        .expect("valid 10/10 zig-zag");
    assert!((metrics.first_overshoot_deg - 5.0).abs() < 1.0e-9);
    assert!((metrics.second_overshoot_deg - 6.0).abs() < 1.0e-9);
    assert!((metrics.first_peak_time_s - 4.0).abs() < 1.0e-9);
    assert!((metrics.second_peak_time_s - 11.0).abs() < 1.0e-9);
}

#[test]
fn zig_zag_rejects_non_alternating_rudder_commands() {
    let mut commands = zig_zag_commands();
    commands[2].rudder_deg = -10.0;
    assert_eq!(
        zig_zag_metrics(&zig_zag_trajectory(), &commands, 10.0),
        Err(TrajectoryError::InvalidRudderSequence)
    );
}

#[test]
fn zig_zag_rejects_reversal_before_heading_target() {
    let mut commands = zig_zag_commands();
    commands[1].time_s = 1.0;
    assert_eq!(
        zig_zag_metrics(&zig_zag_trajectory(), &commands, 10.0),
        Err(TrajectoryError::FirstReversalBeforeTarget)
    );
}

#[test]
fn zig_zag_rejects_unobserved_opposite_heading_target() {
    let mut trajectory = zig_zag_trajectory();
    trajectory.samples.truncate(7);
    let commands = [
        ZigZagCommand {
            time_s: 0.0,
            rudder_deg: 10.0,
        },
        ZigZagCommand {
            time_s: 2.0,
            rudder_deg: -10.0,
        },
        ZigZagCommand {
            time_s: 6.0,
            rudder_deg: 10.0,
        },
    ];
    assert_eq!(
        zig_zag_metrics(&trajectory, &commands, 10.0),
        Err(TrajectoryError::HeadingTargetNotReached(10))
    );
}

#[test]
fn zig_zag_rejects_missing_first_peak() {
    let mut trajectory = zig_zag_trajectory();
    for sample in &mut trajectory.samples[3..10] {
        sample.yaw_rate_rad_s = 1.0;
    }
    assert_eq!(
        zig_zag_metrics(&trajectory, &zig_zag_commands(), 10.0),
        Err(TrajectoryError::OvershootPeakNotObserved(1))
    );
}

#[test]
fn zig_zag_rejects_zero_yaw_rate_without_observed_sign_reversal() {
    let mut trajectory = zig_zag_trajectory();
    for sample in &mut trajectory.samples[5..10] {
        sample.yaw_rate_rad_s = 1.0;
    }
    assert_eq!(
        zig_zag_metrics(&trajectory, &zig_zag_commands(), 10.0),
        Err(TrajectoryError::OvershootPeakNotObserved(1))
    );
}

#[test]
fn zig_zag_requires_command_events_to_align_with_trajectory_samples() {
    let mut commands = zig_zag_commands();
    commands[1].time_s = 2.5;
    assert_eq!(
        zig_zag_metrics(&zig_zag_trajectory(), &commands, 10.0),
        Err(TrajectoryError::CommandTimeNotSampleAligned)
    );
}

#[test]
fn twenty_twenty_zig_zag_uses_the_same_metric_definition() {
    let mut trajectory = zig_zag_trajectory();
    for sample in &mut trajectory.samples {
        sample.heading_rad *= 2.0;
        sample.yaw_rate_rad_s *= 2.0;
    }
    let commands = [
        ZigZagCommand {
            time_s: 0.0,
            rudder_deg: 20.0,
        },
        ZigZagCommand {
            time_s: 2.0,
            rudder_deg: -20.0,
        },
        ZigZagCommand {
            time_s: 9.0,
            rudder_deg: 20.0,
        },
    ];
    let metrics = zig_zag_metrics(&trajectory, &commands, 20.0).expect("valid 20/20 zig-zag");
    assert!((metrics.first_overshoot_deg - 10.0).abs() < 1.0e-9);
    assert!((metrics.second_overshoot_deg - 12.0).abs() < 1.0e-9);
}

#[test]
fn zig_zag_metrics_support_negative_heading_direction() {
    let mut trajectory = zig_zag_trajectory();
    for sample in &mut trajectory.samples {
        sample.heading_rad = -sample.heading_rad;
        sample.yaw_rate_rad_s = -sample.yaw_rate_rad_s;
    }
    let metrics = zig_zag_metrics(&trajectory, &zig_zag_commands(), 10.0)
        .expect("valid negative-direction zig-zag");
    assert!((metrics.first_overshoot_deg - 5.0).abs() < 1.0e-9);
    assert!((metrics.second_overshoot_deg - 6.0).abs() < 1.0e-9);
}
