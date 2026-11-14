use std::hint::black_box;
use std::time::Instant;

use construction_robot_interop_core::resources::ReservationRequest;
use construction_spacetime_occupancy_core::{
    Bounds3Mm, OccupancyKind, SiteBinding, SpatialOccupancy, SpatialOccupancyBook,
};

fn binding() -> SiteBinding {
    SiteBinding {
        floor_id: "benchmark-floor".to_owned(),
        site_epoch: "benchmark-epoch-1".to_owned(),
        design_revision_digest: "sha256:benchmark-design".to_owned(),
        target_frame_id: "benchmark-grid".to_owned(),
    }
}

fn occupancy(index: u64) -> SpatialOccupancy {
    let x = i64::try_from(index)
        .expect("benchmark index fits i64")
        .checked_mul(2_000)
        .expect("benchmark coordinate does not overflow");
    SpatialOccupancy {
        occupancy_id: format!("occupancy-{index:05}"),
        task_id: format!("task-{index:05}"),
        kind: OccupancyKind::Workface,
        binding: binding(),
        bounds: Bounds3Mm {
            min: [x, 0, 0],
            max: [x + 1_000, 1_000, 2_000],
        },
        uncertainty_margin_mm: [0, 0, 0],
        clearance_margin_mm: [0, 0, 0],
        reservation: ReservationRequest {
            reservation_id: format!("reservation-{index:05}"),
            resource_id: format!("resource-{index:05}"),
            task_id: format!("task-{index:05}"),
            owner_id: format!("owner-{index:05}"),
            site_epoch: "benchmark-epoch-1".to_owned(),
            clock_domain_id: "benchmark-clock".to_owned(),
            clock_uncertainty_ms: 0,
            interval_start_ms: 0,
            interval_end_ms: 100,
            expires_at_ms: 200,
            units: 1,
        },
    }
}

fn dense_occupancy(index: u64) -> SpatialOccupancy {
    // 1,000 disjoint small envelopes packed into a 10 m cube. They share
    // coarse spatial neighbourhoods but do not physically overlap.
    let x = i64::try_from(index % 10).expect("x index") * 900;
    let y = i64::try_from((index / 10) % 10).expect("y index") * 900;
    let z = i64::try_from((index / 100) % 10).expect("z index") * 900;
    let task_id = format!("dense-task-{index:04}");
    SpatialOccupancy {
        occupancy_id: format!("dense-occupancy-{index:04}"),
        task_id: task_id.clone(),
        kind: OccupancyKind::Workface,
        binding: binding(),
        bounds: Bounds3Mm {
            min: [x, y, z],
            max: [x + 400, y + 400, z + 400],
        },
        uncertainty_margin_mm: [0, 0, 0],
        clearance_margin_mm: [0, 0, 0],
        reservation: ReservationRequest {
            reservation_id: format!("dense-reservation-{index:04}"),
            resource_id: "dense-workface".to_owned(),
            task_id,
            owner_id: format!("dense-owner-{index:04}"),
            site_epoch: "benchmark-epoch-1".to_owned(),
            clock_domain_id: "benchmark-clock".to_owned(),
            clock_uncertainty_ms: 0,
            interval_start_ms: 0,
            interval_end_ms: 100,
            expires_at_ms: 200,
            units: 1,
        },
    }
}

fn main() {
    for size in [10_u64, 100, 1_000, 10_000] {
        let mut book = SpatialOccupancyBook::new(binding(), "benchmark-clock").expect("valid book");
        let start = Instant::now();
        for index in 0..size {
            book.insert(occupancy(index))
                .expect("sparse non-overlapping occupancy");
        }
        let insert_elapsed = start.elapsed();

        let mut local_query = occupancy(size + 1);
        let local_x = i64::try_from(size / 2)
            .expect("local query index fits i64")
            .checked_mul(2_000)
            .expect("local query coordinate does not overflow");
        local_query.bounds = Bounds3Mm {
            min: [local_x, 0, 0],
            max: [local_x + 1_000, 1_000, 2_000],
        };
        local_query.reservation.reservation_id = format!("local-query-{size}");
        local_query.task_id = format!("local-query-task-{size}");
        local_query.reservation.task_id = local_query.task_id.clone();

        let local_candidates = book
            .candidate_count_for(black_box(&local_query))
            .expect("valid local candidate query");
        let local_start = Instant::now();
        let local_conflicts = book
            .conflicts_for(black_box(&local_query))
            .expect("valid local conflict query");
        let local_elapsed = local_start.elapsed();

        let mut far_query = occupancy(size + 2);
        far_query.bounds = Bounds3Mm {
            min: [i64::MAX - 2_000, 0, 0],
            max: [i64::MAX - 1_000, 1_000, 2_000],
        };
        far_query.reservation.reservation_id = format!("far-query-{size}");
        far_query.task_id = format!("far-query-task-{size}");
        far_query.reservation.task_id = far_query.task_id.clone();
        let far_candidates = book
            .candidate_count_for(black_box(&far_query))
            .expect("valid far candidate query");

        println!(
            "distribution=sparse records={size} inserted={} insert_ms={:.3} local_candidates={local_candidates} local_query_ms={:.3} local_conflicts={} far_candidates={far_candidates}",
            book.occupancy_count(),
            insert_elapsed.as_secs_f64() * 1_000.0,
            local_elapsed.as_secs_f64() * 1_000.0,
            local_conflicts.len()
        );
        black_box(book);
    }

    let mut dense_book =
        SpatialOccupancyBook::new(binding(), "benchmark-clock").expect("valid dense book");
    let dense_start = Instant::now();
    for index in 0..1_000_u64 {
        dense_book
            .insert(dense_occupancy(index))
            .expect("dense but disjoint occupancy");
    }
    let dense_insert_elapsed = dense_start.elapsed();

    let mut dense_gap = dense_occupancy(1_001);
    dense_gap.occupancy_id = "dense-gap-query".to_owned();
    dense_gap.task_id = "dense-gap-query-task".to_owned();
    dense_gap.reservation.reservation_id = "dense-gap-query-reservation".to_owned();
    dense_gap.reservation.task_id = dense_gap.task_id.clone();
    dense_gap.bounds = Bounds3Mm {
        min: [450, 0, 0],
        max: [850, 400, 400],
    };
    let dense_candidates = dense_book
        .candidate_count_for(black_box(&dense_gap))
        .expect("dense gap candidate query");
    let dense_query_start = Instant::now();
    let dense_conflicts = dense_book
        .conflicts_for(black_box(&dense_gap))
        .expect("dense gap conflict query");
    let dense_query_elapsed = dense_query_start.elapsed();

    println!(
        "distribution=dense records={} insert_ms={:.3} gap_candidates={dense_candidates} gap_query_ms={:.3} gap_conflicts={}",
        dense_book.occupancy_count(),
        dense_insert_elapsed.as_secs_f64() * 1_000.0,
        dense_query_elapsed.as_secs_f64() * 1_000.0,
        dense_conflicts.len()
    );
    black_box(dense_book);
}
