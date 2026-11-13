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

fn main() {
    for size in [10_u64, 100, 1_000, 10_000] {
        let mut book = SpatialOccupancyBook::new(binding(), "benchmark-clock").expect("valid book");
        let start = Instant::now();
        for index in 0..size {
            book.insert(occupancy(index))
                .expect("sparse non-overlapping occupancy");
        }
        let insert_elapsed = start.elapsed();

        let mut query = occupancy(size + 1);
        query.bounds = Bounds3Mm {
            min: [i64::MAX - 2_000, 0, 0],
            max: [i64::MAX - 1_000, 1_000, 2_000],
        };
        query.reservation.reservation_id = format!("query-{size}");
        query.task_id = format!("query-task-{size}");
        query.reservation.task_id = query.task_id.clone();

        let query_start = Instant::now();
        let conflicts = book
            .conflicts_for(black_box(&query))
            .expect("valid conflict query");
        let query_elapsed = query_start.elapsed();

        println!(
            "records={size} inserted={} insert_ms={:.3} query_ms={:.3} query_conflicts={}",
            book.occupancy_count(),
            insert_elapsed.as_secs_f64() * 1_000.0,
            query_elapsed.as_secs_f64() * 1_000.0,
            conflicts.len()
        );
        black_box(book);
    }
}
