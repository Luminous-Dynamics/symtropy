use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_envelope_occupancy_adapter::{ProjectedOccupancy, project_slice};
use construction_motion_envelope::{Context, EvidenceRefs, MarginsMm, Point3Mm, Segment, build};
use construction_robot_interop_core::resources::ReservationRequest;
use construction_spacetime_occupancy_core::{SiteBinding, SpatialOccupancyBook};

const QUERY_SAMPLES: usize = 101;
const INSERT_SAMPLES: usize = 25;

fn percentile(values: &[Duration], percentile: usize) -> Duration {
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values[index]
}

fn report_for(task_index: usize, x_offset: i64) -> construction_motion_envelope::Report {
    let context = Context {
        project_id: "adapter-benchmark-project".into(),
        work_package_id: "adapter-benchmark-package".into(),
        task_id: format!("task-{task_index}"),
        trajectory_id: format!("trajectory-{task_index}"),
        robot_id: "robot-bench".into(),
        platform_config_sha256: "a".repeat(64),
        tool_config_sha256: "b".repeat(64),
        payload_config_sha256: "c".repeat(64),
        collision_model_sha256: "d".repeat(64),
        primitive_set_ref: "bench://primitive-manifest".into(),
        expected_primitive_ids: vec!["tool".into()],
        calibration_ref: "bench://calibration".into(),
        controller_profile_ref: "bench://controller".into(),
        frame_id: "bench-frame".into(),
        transform_sha256: "e".repeat(64),
        design_revision_sha256: "f".repeat(64),
        site_epoch: "bench-epoch".into(),
        clock_domain_id: "bench-clock".into(),
        trajectory_profile_sha256: "1".repeat(64),
    };
    let segment = Segment {
        slice_index: 0,
        primitive_id: "tool".into(),
        start_ms: 10,
        end_ms: 40,
        from_mm: Point3Mm {
            x: x_offset,
            y: 0,
            z: 0,
        },
        to_mm: Point3Mm {
            x: x_offset + 50,
            y: 0,
            z: 0,
        },
        radius_mm: 10,
        margins: MarginsMm::default(),
        evidence: EvidenceRefs {
            path_deviation: "bench://path".into(),
            localization: "bench://localization".into(),
            tracking: "bench://tracking".into(),
            model_uncertainty: "bench://model".into(),
            numerical_enclosure: "bench://numerical".into(),
            stopping_reachability: "bench://stopping".into(),
            policy_clearance: "bench://policy".into(),
        },
    };
    build(&context, &[segment]).expect("fixed benchmark report is valid")
}

fn project_for(task_index: usize, x_offset: i64) -> ProjectedOccupancy {
    let report = report_for(task_index, x_offset);
    let slice = &report.slices[0];
    let binding = SiteBinding {
        floor_id: "bench-floor".into(),
        site_epoch: report.context.site_epoch.clone(),
        design_revision_digest: report.context.design_revision_sha256.clone(),
        target_frame_id: report.context.frame_id.clone(),
    };
    let reservation = ReservationRequest {
        reservation_id: format!("reservation-{task_index}"),
        resource_id: "robot-resource".into(),
        task_id: report.context.task_id.clone(),
        owner_id: "bench-owner".into(),
        site_epoch: report.context.site_epoch.clone(),
        clock_domain_id: report.context.clock_domain_id.clone(),
        clock_uncertainty_ms: 0,
        interval_start_ms: slice.start_ms,
        interval_end_ms: slice.end_ms,
        expires_at_ms: 1_000,
        units: 1,
    };
    project_slice(
        &report,
        slice,
        binding,
        reservation,
        format!("occupancy-{task_index}"),
        construction_spacetime_occupancy_core::OccupancyKind::RobotOperatingEnvelope,
    )
    .expect("fixed benchmark projection is valid")
}

fn measure_queries(label: &str, mut operation: impl FnMut()) {
    let mut durations = Vec::with_capacity(QUERY_SAMPLES);
    for _ in 0..QUERY_SAMPLES {
        let start = Instant::now();
        for _ in 0..1_000 {
            operation();
        }
        durations.push(start.elapsed() / 1_000);
    }
    durations.sort_unstable();
    println!(
        "{label},p50_ns={},p95_ns={},p99_ns={}",
        percentile(&durations, 50).as_nanos(),
        percentile(&durations, 95).as_nanos(),
        percentile(&durations, 99).as_nanos()
    );
}

fn measure_batch_insert(label: &str, records: &[ProjectedOccupancy], binding: SiteBinding) {
    let mut durations = Vec::with_capacity(INSERT_SAMPLES);
    for _ in 0..INSERT_SAMPLES {
        let mut book = SpatialOccupancyBook::new(binding.clone(), "bench-clock")
            .expect("valid occupancy book");
        let start = Instant::now();
        for record in records {
            book.insert(black_box(record.occupancy.clone()))
                .expect("spatially disjoint benchmark records insert");
        }
        durations.push(start.elapsed() / records.len() as u32);
    }
    durations.sort_unstable();
    println!(
        "{label},per_insert_p50_ns={},per_insert_p95_ns={},per_insert_p99_ns={}",
        percentile(&durations, 50).as_nanos(),
        percentile(&durations, 95).as_nanos(),
        percentile(&durations, 99).as_nanos()
    );
}

fn main() {
    println!("workload,metric,value");
    let projection_report = report_for(0, 0);
    let projection_slice = &projection_report.slices[0];
    let projection_binding = SiteBinding {
        floor_id: "bench-floor".into(),
        site_epoch: projection_report.context.site_epoch.clone(),
        design_revision_digest: projection_report.context.design_revision_sha256.clone(),
        target_frame_id: projection_report.context.frame_id.clone(),
    };
    let projection_reservation = ReservationRequest {
        reservation_id: "reservation-projection".into(),
        resource_id: "robot-resource".into(),
        task_id: projection_report.context.task_id.clone(),
        owner_id: "bench-owner".into(),
        site_epoch: projection_report.context.site_epoch.clone(),
        clock_domain_id: projection_report.context.clock_domain_id.clone(),
        clock_uncertainty_ms: 0,
        interval_start_ms: projection_slice.start_ms,
        interval_end_ms: projection_slice.end_ms,
        expires_at_ms: 1_000,
        units: 1,
    };
    measure_queries("pure-projection", || {
        let projected = project_slice(
            black_box(&projection_report),
            black_box(projection_slice),
            black_box(projection_binding.clone()),
            black_box(projection_reservation.clone()),
            "projection-benchmark",
            construction_spacetime_occupancy_core::OccupancyKind::RobotOperatingEnvelope,
        )
        .expect("valid projection query");
        black_box(projected);
    });

    for count in [10_usize, 100, 1_000, 10_000] {
        let records: Vec<ProjectedOccupancy> = (0..count)
            .map(|index| project_for(index, (index as i64) * 10_000))
            .collect();
        let binding = records[0].occupancy.binding.clone();
        let mut book = SpatialOccupancyBook::new(binding.clone(), "bench-clock")
            .expect("valid occupancy book");
        for record in &records {
            book.insert(record.occupancy.clone())
                .expect("spatially disjoint benchmark record");
        }

        let candidate = project_for(count + 1, (count as i64) * 10_000 + 5_000);
        let suffix = format!("records-{count}");
        measure_queries(&format!("candidate-count-{suffix}"), || {
            black_box(
                book.candidate_count_for(black_box(&candidate.occupancy))
                    .expect("candidate query is valid"),
            );
        });
        measure_queries(&format!("exact-conflict-query-{suffix}"), || {
            black_box(
                book.conflicts_for(black_box(&candidate.occupancy))
                    .expect("conflict query is valid"),
            );
        });
        measure_batch_insert(&format!("batch-insert-{suffix}"), &records, binding);
        println!(
            "candidate-count-{suffix},value,{}",
            book.candidate_count_for(&candidate.occupancy)
                .expect("candidate query is valid")
        );
        println!(
            "exact-conflict-query-{suffix},value,{}",
            book.conflicts_for(&candidate.occupancy)
                .expect("conflict query is valid")
                .len()
        );
    }
}
