use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_envelope::{
    AxisMm, Context, EvidenceRefs, MarginsMm, Point3Mm, Segment, build,
};
use construction_motion_oracle::{
    StopTubeBuildRequest, StopTubeKinematics, StoppingDynamicsQualification,
    StoppingReachabilityRequest, build_stop_tube, calculate_stopping_reachability,
};

fn digest(value: char) -> String {
    std::iter::repeat_n(value, 64).collect()
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let index = ((samples.len() - 1) * percentile).div_ceil(100);
    samples[index.min(samples.len() - 1)]
}

fn report_with_segments(segment_count: usize) -> construction_motion_envelope::Report {
    let primitive_ids = (0..4)
        .map(|index| format!("primitive-{index:02}"))
        .collect::<Vec<_>>();
    let context = Context {
        project_id: "stop-tube-benchmark".into(),
        work_package_id: "benchmark-work-package".into(),
        task_id: "benchmark-task".into(),
        trajectory_id: "straight-rigid-translation".into(),
        robot_id: "robot-bench-01".into(),
        platform_config_sha256: digest('1'),
        tool_config_sha256: digest('2'),
        payload_config_sha256: digest('3'),
        collision_model_sha256: digest('4'),
        primitive_set_ref: "benchmark://primitive-manifest".into(),
        expected_primitive_ids: primitive_ids.clone(),
        calibration_ref: "benchmark-calibration".into(),
        controller_profile_ref: "fixed-tangent-controller-profile".into(),
        frame_id: "benchmark-site-frame".into(),
        transform_sha256: digest('5'),
        design_revision_sha256: digest('6'),
        site_epoch: "benchmark-site-epoch".into(),
        clock_domain_id: "benchmark-site-clock".into(),
        trajectory_profile_sha256: digest('7'),
    };
    let margins = MarginsMm {
        path_deviation: AxisMm { x: 2, y: 2, z: 2 },
        localization: AxisMm { x: 5, y: 5, z: 5 },
        tracking: AxisMm { x: 1, y: 1, z: 1 },
        model_uncertainty: AxisMm { x: 2, y: 2, z: 2 },
        numerical_enclosure: AxisMm { x: 1, y: 1, z: 1 },
        stopping_reachability: AxisMm { x: 10, y: 10, z: 10 },
        policy_clearance: AxisMm { x: 50, y: 50, z: 50 },
    };
    let evidence = EvidenceRefs {
        path_deviation: "benchmark-path".into(),
        localization: "benchmark-localization".into(),
        tracking: "benchmark-tracking".into(),
        model_uncertainty: "benchmark-model".into(),
        numerical_enclosure: "benchmark-numerical".into(),
        stopping_reachability: "benchmark-stopping".into(),
        policy_clearance: "benchmark-policy".into(),
    };
    let mut segments = Vec::with_capacity(segment_count * primitive_ids.len());
    for index in 0..segment_count {
        let base_x = (index as i64) * 1_000;
        let start_ms = (index as u64) * 100;
        let end_ms = start_ms + 100;
        for (primitive_index, primitive_id) in primitive_ids.iter().enumerate() {
            let offset_y = (primitive_index as i64) * 250;
            segments.push(Segment {
                slice_index: index as u32,
                primitive_id: primitive_id.clone(),
                start_ms,
                end_ms,
                from_mm: Point3Mm {
                    x: base_x,
                    y: offset_y,
                    z: 500,
                },
                to_mm: Point3Mm {
                    x: base_x + 1_000,
                    y: offset_y,
                    z: 500,
                },
                radius_mm: if primitive_index == 0 { 300 } else { 100 },
                margins,
                evidence: evidence.clone(),
            });
        }
    }
    build(&context, &segments).expect("synthetic benchmark source report must be valid")
}

fn qualification() -> StoppingDynamicsQualification {
    StoppingDynamicsQualification {
        profile_sha256: digest('8'),
        qualification_receipt_sha256: digest('9'),
        robot_brake_configuration_sha256: digest('a'),
        operating_condition_sha256: digest('b'),
        scenario_context_sha256: digest('c'),
        site_clock_domain_id: "benchmark-site-clock".into(),
        valid_until_tick: 10_000_000,
        maximum_qualified_speed_mm_s: 2_500.0,
        maximum_detection_to_brake_latency_ms: 120,
        maximum_positive_acceleration_mm_s2: 100.0,
        minimum_net_deceleration_mm_s2: 2_500.0,
        minimum_braking_jerk_mm_s3: 8_000.0,
        additional_distance_guard_mm: 50.0,
    }
}

fn run_case(segment_count: usize) {
    let report = report_with_segments(segment_count);
    let qualification = qualification();
    let stopping_result = calculate_stopping_reachability(
        &qualification,
        StoppingReachabilityRequest {
            speed_upper_mm_s: 2_000.0,
        },
    )
    .expect("synthetic stopping qualification must be valid");
    let request = || StopTubeBuildRequest {
        nominal_report: &report,
        nominal_report_sha256: digest('d'),
        trajectory_sha256: digest('e'),
        primitive_manifest_sha256: digest('f'),
        frame_sha256: digest('1'),
        trigger_domain_sha256: digest('2'),
        builder_build_sha256: digest('3'),
        stopping_qualification: &qualification,
        stopping_result: &stopping_result,
        maximum_trigger_window_ms: 10,
        now_tick: 100,
        requested_valid_until_tick: 9_000_000,
        kinematics: StopTubeKinematics::FixedOrientationFixedTangentNoSteering,
    };

    let warm = build_stop_tube(request()).expect("stop-tube benchmark must build");
    let expected_slices = segment_count * 4 * 10;
    assert_eq!(warm.slices.len(), expected_slices);
    black_box(warm.canonical_bytes());

    let mut samples = Vec::with_capacity(9);
    let mut canonical_bytes_len = 0_usize;
    for _ in 0..9 {
        let start = Instant::now();
        let output = build_stop_tube(request()).expect("stop-tube benchmark must build");
        let bytes = output.canonical_bytes();
        samples.push(start.elapsed());
        canonical_bytes_len = bytes.len();
        black_box(bytes);
    }
    samples.sort_unstable();
    let p50 = percentile(&samples, 50);
    let p95 = percentile(&samples, 95);
    let p99 = percentile(&samples, 99);
    let throughput = expected_slices as f64 / p50.as_secs_f64();

    println!(
        "{},{},{},{},{},{},{},{:.2}",
        segment_count,
        4,
        segment_count * 10,
        expected_slices,
        p50.as_micros(),
        p95.as_micros(),
        p99.as_micros(),
        throughput,
    );
    println!(
        "  canonical_bytes={}, stop_distance_mm={}, stop_time_ms={}",
        canonical_bytes_len,
        stopping_result.conservative_distance_bound_mm,
        stopping_result.conservative_stop_time_bound_ms
    );
}

fn main() {
    println!(
        "source_segments,primitives,trigger_windows,output_slices,p50_us,p95_us,p99_us,slices_per_second"
    );
    for segment_count in [10_usize, 100, 2_500] {
        run_case(segment_count);
    }
}
