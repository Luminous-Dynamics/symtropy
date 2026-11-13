use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_robot_interop_core::resources::{
    ReservationBook, ReservationRequest,
};

fn request(index: u64, resource_id: &str, start_ms: u64, end_ms: u64) -> ReservationRequest {
    ReservationRequest {
        reservation_id: format!("reservation-{index:06}"),
        resource_id: resource_id.to_owned(),
        task_id: format!("task-{index:06}"),
        owner_id: format!("owner-{index:06}"),
        site_epoch: "site-epoch-1".to_owned(),
        clock_domain_id: "site-monotonic-ms".to_owned(),
        clock_uncertainty_ms: 0,
        interval_start_ms: start_ms,
        interval_end_ms: end_ms,
        expires_at_ms: end_ms + 10_000,
        units: 1,
    }
}

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn sparse_run(size: u64) {
    let mut insert_samples = Vec::new();
    let mut local_candidates = 0_usize;
    let mut gap_candidates = 0_usize;

    for _ in 0..5 {
        let mut book = ReservationBook::new("site-epoch-1", "site-monotonic-ms", 0)
            .expect("valid reservation book");
        book.register_resource("shared-hoist", u32::MAX)
            .expect("shared capacity");

        let start = Instant::now();
        for index in 0..size {
            let interval_start = index
                .checked_mul(120_000)
                .and_then(|value| value.checked_add(10_000))
                .expect("sparse interval start");
            let interval_end = interval_start + 1_000;
            book.reserve(
                request(index, "shared-hoist", interval_start, interval_end),
                0,
            )
            .expect("sparse non-overlapping reservation");
        }
        insert_samples.push(start.elapsed());

        let middle = size / 2;
        let local_start = middle * 120_000 + 10_000;
        local_candidates = book
            .candidate_count_for_interval("shared-hoist", local_start, local_start + 1_000)
            .expect("local interval candidate count");

        let gap_start = middle * 120_000 + 70_000;
        gap_candidates = book
            .candidate_count_for_interval("shared-hoist", gap_start, gap_start + 1_000)
            .expect("gap interval candidate count");
        black_box(book);
    }

    println!(
        "distribution=sparse reservations={size} repetitions=5 insert_median_ms={:.3} local_candidates={local_candidates} gap_candidates={gap_candidates}",
        median(insert_samples).as_secs_f64() * 1_000.0
    );
}

fn dense_run(size: u64) {
    let mut book = ReservationBook::new("site-epoch-1", "site-monotonic-ms", 0)
        .expect("valid reservation book");
    book.register_resource("shared-workface", u32::MAX)
        .expect("shared capacity");

    let start = Instant::now();
    for index in 0..size {
        let interval_start = 1_000 + index * 40;
        let interval_end = interval_start + 20;
        book.reserve(
            request(index, "shared-workface", interval_start, interval_end),
            0,
        )
        .expect("dense but non-overlapping reservation");
    }
    let elapsed = start.elapsed();

    let dense_candidates = book
        .candidate_count_for_interval("shared-workface", 20_000, 20_100)
        .expect("dense bucket candidate count");
    println!(
        "distribution=dense_single_bucket reservations={size} insert_ms={:.3} query_candidates={dense_candidates}",
        elapsed.as_secs_f64() * 1_000.0
    );
    black_box(book);
}

fn main() {
    for size in [10_u64, 100, 1_000, 10_000, 50_000] {
        sparse_run(size);
    }
    dense_run(1_000);
}
