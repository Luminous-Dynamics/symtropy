use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_robot_interop_core::resources::{ReservationBook, ReservationRequest};

fn request(
    index: u64,
    resource_id: &str,
    start_ms: u64,
    end_ms: u64,
    expiry_ms: u64,
) -> ReservationRequest {
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
        expires_at_ms: expiry_ms,
        units: 1,
    }
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    assert!(!samples.is_empty(), "benchmark sample set is non-empty");
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted[rank - 1]
}

fn measure_query<F>(mut query: F) -> Vec<Duration>
where
    F: FnMut(),
{
    let mut samples = Vec::with_capacity(25);
    for _ in 0..25 {
        let start = Instant::now();
        query();
        samples.push(start.elapsed());
    }
    samples
}

fn sparse_run(size: u64) {
    let mut insert_samples = Vec::with_capacity(25);
    let mut indexed_query_samples = Vec::with_capacity(625);
    let mut full_scan_query_samples = Vec::with_capacity(625);
    let mut indexed_local_candidates = 0_usize;
    let mut reference_local_overlaps = 0_usize;
    let mut indexed_gap_candidates = 0_usize;
    let mut reference_gap_overlaps = 0_usize;
    let mut active = 0_usize;

    for _ in 0..25 {
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
                request(
                    index,
                    "shared-hoist",
                    interval_start,
                    interval_end,
                    interval_end + 10_000,
                ),
                0,
            )
            .expect("sparse non-overlapping reservation");
        }
        insert_samples.push(start.elapsed());

        let middle = size / 2;
        let local_start = middle * 120_000 + 10_000;
        let gap_start = middle * 120_000 + 70_000;
        active = book
            .active_reservation_count_for_resource("shared-hoist")
            .expect("active resource count");
        indexed_local_candidates = book
            .candidate_count_for_interval("shared-hoist", local_start, local_start + 1_000)
            .expect("indexed local candidates");
        reference_local_overlaps = book
            .reference_candidate_count_for_interval(
                "shared-hoist",
                local_start,
                local_start + 1_000,
            )
            .expect("reference local overlaps");
        indexed_gap_candidates = book
            .candidate_count_for_interval("shared-hoist", gap_start, gap_start + 1_000)
            .expect("indexed gap candidates");
        reference_gap_overlaps = book
            .reference_candidate_count_for_interval("shared-hoist", gap_start, gap_start + 1_000)
            .expect("reference gap overlaps");

        indexed_query_samples.extend(measure_query(|| {
            black_box(
                book.candidate_count_for_interval("shared-hoist", local_start, local_start + 1_000)
                    .expect("indexed local query"),
            );
        }));
        full_scan_query_samples.extend(measure_query(|| {
            black_box(
                book.reference_candidate_count_for_interval(
                    "shared-hoist",
                    local_start,
                    local_start + 1_000,
                )
                .expect("full-scan local query"),
            );
        }));
        black_box(book);
    }

    println!(
        "distribution=sparse reservations={size} repetitions=25 query_samples=625 active={active} insert_p50_ms={:.3} insert_p95_ms={:.3} insert_p99_ms={:.3} indexed_local_candidates={indexed_local_candidates} indexed_query_p50_us={:.3} indexed_query_p95_us={:.3} indexed_query_p99_us={:.3} reference_local_overlaps={reference_local_overlaps} full_scan_p50_us={:.3} full_scan_p95_us={:.3} full_scan_p99_us={:.3} indexed_gap_candidates={indexed_gap_candidates} reference_gap_overlaps={reference_gap_overlaps}",
        percentile(&insert_samples, 50).as_secs_f64() * 1_000.0,
        percentile(&insert_samples, 95).as_secs_f64() * 1_000.0,
        percentile(&insert_samples, 99).as_secs_f64() * 1_000.0,
        percentile(&indexed_query_samples, 50).as_secs_f64() * 1_000_000.0,
        percentile(&indexed_query_samples, 95).as_secs_f64() * 1_000_000.0,
        percentile(&indexed_query_samples, 99).as_secs_f64() * 1_000_000.0,
        percentile(&full_scan_query_samples, 50).as_secs_f64() * 1_000_000.0,
        percentile(&full_scan_query_samples, 95).as_secs_f64() * 1_000_000.0,
        percentile(&full_scan_query_samples, 99).as_secs_f64() * 1_000_000.0,
    );
}

fn multi_resource_run(per_resource: u64) {
    let resources = 16_u64;
    let mut book = ReservationBook::new("site-epoch-1", "site-monotonic-ms", 0)
        .expect("valid reservation book");
    for resource in 0..resources {
        book.register_resource(format!("resource-{resource:02}"), u32::MAX)
            .expect("resource capacity");
    }

    let start = Instant::now();
    for index in 0..(per_resource * resources) {
        let resource = index % resources;
        let local = index / resources;
        let interval_start = local * 120_000 + 10_000;
        let interval_end = interval_start + 1_000;
        book.reserve(
            request(
                index,
                &format!("resource-{resource:02}"),
                interval_start,
                interval_end,
                interval_end + 10_000,
            ),
            0,
        )
        .expect("resource-partitioned reservation");
    }
    let insert = start.elapsed();
    let local_start = (per_resource / 2) * 120_000 + 10_000;
    let indexed = measure_query(|| {
        black_box(
            book.candidate_count_for_interval("resource-07", local_start, local_start + 1_000)
                .expect("resource-local indexed query"),
        );
    });
    let full_scan = measure_query(|| {
        black_box(
            book.reference_candidate_count_for_interval(
                "resource-07",
                local_start,
                local_start + 1_000,
            )
            .expect("resource-local full scan"),
        );
    });
    println!(
        "distribution=multi_resource resources={resources} per_resource={per_resource} total={} insert_ms={:.3} indexed_candidates={} indexed_query_p50_us={:.3} full_scan_overlaps={} full_scan_query_p50_us={:.3}",
        per_resource * resources,
        insert.as_secs_f64() * 1_000.0,
        book.candidate_count_for_interval("resource-07", local_start, local_start + 1_000)
            .unwrap(),
        percentile(&indexed, 50).as_secs_f64() * 1_000_000.0,
        book.reference_candidate_count_for_interval(
            "resource-07",
            local_start,
            local_start + 1_000
        )
        .unwrap(),
        percentile(&full_scan, 50).as_secs_f64() * 1_000_000.0
    );
    black_box(book);
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
            request(
                index,
                "shared-workface",
                interval_start,
                interval_end,
                interval_end + 10_000,
            ),
            0,
        )
        .expect("dense but non-overlapping reservation");
    }
    let elapsed = start.elapsed();

    let query_start = 20_000;
    let indexed = measure_query(|| {
        black_box(
            book.candidate_count_for_interval("shared-workface", query_start, query_start + 100)
                .expect("dense indexed query"),
        );
    });
    let full_scan = measure_query(|| {
        black_box(
            book.reference_candidate_count_for_interval(
                "shared-workface",
                query_start,
                query_start + 100,
            )
            .expect("dense reference query"),
        );
    });
    println!(
        "distribution=dense_single_bucket reservations={size} insert_ms={:.3} indexed_candidates={} indexed_query_p50_us={:.3} full_scan_overlaps={} full_scan_query_p50_us={:.3}",
        elapsed.as_secs_f64() * 1_000.0,
        book.candidate_count_for_interval("shared-workface", query_start, query_start + 100)
            .unwrap(),
        percentile(&indexed, 50).as_secs_f64() * 1_000_000.0,
        book.reference_candidate_count_for_interval(
            "shared-workface",
            query_start,
            query_start + 100
        )
        .unwrap(),
        percentile(&full_scan, 50).as_secs_f64() * 1_000_000.0
    );
    black_box(book);
}

fn expiry_run(size: u64) {
    let mut book = ReservationBook::new("site-epoch-1", "site-monotonic-ms", 0)
        .expect("valid reservation book");
    book.register_resource("expiry-load", u32::MAX)
        .expect("expiry resource");
    for index in 0..size {
        let (start, end, expiry) = if index < 100 {
            let start = 10_000 + index * 100;
            (start, start + 50, start + 150)
        } else {
            let start = 1_000_000 + (index - 100) * 2_000;
            (start, start + 100, 10_000_000_000)
        };
        book.reserve(request(index, "expiry-load", start, end, expiry), 0)
            .expect("expiry benchmark reservation");
    }

    let full_scan = measure_query(|| {
        black_box(book.reference_due_count_at(20_100));
    });
    let expected_due = book.reference_due_count_at(20_100);
    let start = Instant::now();
    let expired = book.expire(20_100);
    let elapsed = start.elapsed();
    let remaining = book
        .active_reservation_count_for_resource("expiry-load")
        .expect("remaining active count");
    println!(
        "distribution=expiry_index reservations={size} expected_due_by_full_scan={expected_due} expired={expired} remaining={remaining} full_scan_due_p50_us={:.3} full_scan_due_p95_us={:.3} indexed_expire_ms={:.3}",
        percentile(&full_scan, 50).as_secs_f64() * 1_000_000.0,
        percentile(&full_scan, 95).as_secs_f64() * 1_000_000.0,
        elapsed.as_secs_f64() * 1_000.0
    );
    black_box(book);
}

fn main() {
    for size in [10_u64, 100, 1_000, 10_000, 50_000] {
        sparse_run(size);
    }
    multi_resource_run(1_000);
    dense_run(1_000);
    expiry_run(50_000);
}
