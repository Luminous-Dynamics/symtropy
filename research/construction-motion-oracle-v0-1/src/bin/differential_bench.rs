use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, Capsule, CapsuleMotionSpan, ClockUncertaintyDisposition, Vec3, chord_collision,
    clock_uncertainty_aware_capsule_query, co_translating_capsule_capsule_collision,
    co_translating_capsule_swept_aabb_candidate,
    co_translating_capsule_time_sliced_aabb_candidates, co_translating_capsule_vs_capsule,
    endpoints_only_co_translating_capsule_collision, endpoints_only_collision,
    endpoints_only_translating_capsule_collision, moving_sphere_capsule_collision,
    swept_sphere_capsule_aabb_candidate, timed_co_translating_capsule_vs_capsule,
    translating_capsule_aabb_candidate, translating_capsule_capsule_collision,
    translating_capsule_vs_capsule,
};

const BATCH_SIZE: usize = 10_000;
const SAMPLES: usize = 101;

fn percentile(values: &[Duration], percentile: usize) -> Duration {
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values[index]
}

fn measure(label: &str, mut operation: impl FnMut()) {
    let mut durations = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..BATCH_SIZE {
            operation();
        }
        let elapsed = start.elapsed();
        durations.push(elapsed / BATCH_SIZE as u32);
    }
    durations.sort_unstable();
    println!(
        "{label},p50_ns={},p95_ns={},p99_ns={}",
        percentile(&durations, 50).as_nanos(),
        percentile(&durations, 95).as_nanos(),
        percentile(&durations, 99).as_nanos()
    );
}

fn main() {
    let linear = AnalyticPath::Linear {
        start: Vec3::new(-1_000.0, 0.0, 0.0),
        end: Vec3::new(1_000.0, 0.0, 0.0),
    };
    let arc = AnalyticPath::UpperSemicircle {
        radius_mm: 1_000.0,
        z_mm: 0.0,
    };
    let linear_obstacle = Vec3::new(0.0, 0.0, 0.0);
    let arc_obstacle = Vec3::new(0.0, 1_000.0, 0.0);

    println!("case,metric,value");
    println!(
        "line-thin-barrier,analytic_collision,{}",
        black_box(linear).collides_with_stationary_sphere(
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        )
    );
    println!(
        "line-thin-barrier,endpoint_collision,{}",
        endpoints_only_collision(
            black_box(linear),
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        )
    );
    println!(
        "arc-chord-miss,analytic_collision,{}",
        black_box(arc).collides_with_stationary_sphere(
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        )
    );
    println!(
        "arc-chord-miss,chord_collision,{}",
        chord_collision(
            black_box(arc),
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        )
    );

    measure("linear-analytic-ccd", || {
        black_box(linear).collides_with_stationary_sphere(
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        );
    });
    measure("linear-endpoint-only", || {
        endpoints_only_collision(
            black_box(linear),
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        );
    });
    measure("semicircle-analytic-ccd", || {
        black_box(arc).collides_with_stationary_sphere(
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        );
    });
    measure("semicircle-endpoint-only", || {
        endpoints_only_collision(
            black_box(arc),
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        );
    });

    let capsule = Capsule::new(Vec3::new(0.0, -100.0, 0.0), Vec3::new(0.0, 100.0, 0.0), 5.0)
        .expect("valid capsule fixture");
    let capsule_start = Vec3::new(-1_000.0, 0.0, 0.0);
    let capsule_end = Vec3::new(1_000.0, 0.0, 0.0);
    println!(
        "capsule-transverse-hit,analytic_collision,{}",
        moving_sphere_capsule_collision(
            black_box(capsule_start),
            black_box(capsule_end),
            black_box(10.0),
            black_box(capsule),
        )
        .expect("valid capsule query")
    );
    measure("linear-sphere-capsule-narrow-phase", || {
        moving_sphere_capsule_collision(
            black_box(capsule_start),
            black_box(capsule_end),
            black_box(10.0),
            black_box(capsule),
        )
        .expect("valid capsule narrow-phase query");
    });
    measure("linear-sphere-capsule-aabb-broad-phase", || {
        swept_sphere_capsule_aabb_candidate(
            black_box(capsule_start),
            black_box(capsule_end),
            black_box(10.0),
            black_box(capsule),
        )
        .expect("valid capsule broad-phase query");
    });
    measure("semicircle-chord-ccd", || {
        chord_collision(
            black_box(arc),
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        );
    });
    let translating = Capsule::new(Vec3::new(-10.0, -2.0, 0.0), Vec3::new(-10.0, 2.0, 0.0), 0.5)
        .expect("valid translating capsule fixture");
    let stationary = Capsule::new(Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, 2.0, 0.0), 0.5)
        .expect("valid stationary capsule fixture");
    let translation = Vec3::new(20.0, 0.0, 0.0);
    let closest = translating_capsule_vs_capsule(
        black_box(translating),
        black_box(translation),
        black_box(stationary),
    )
    .expect("valid translating capsule narrow-phase query");
    println!(
        "translating-capsule,analytic_collision,{}",
        translating_capsule_capsule_collision(
            black_box(translating),
            black_box(translation),
            black_box(stationary),
        )
        .expect("valid translating capsule query")
    );
    println!(
        "translating-capsule,endpoint_collision,{}",
        endpoints_only_translating_capsule_collision(
            black_box(translating),
            black_box(translation),
            black_box(stationary),
        )
        .expect("valid translating capsule endpoint baseline")
    );
    println!(
        "translating-capsule,closest_time,{}",
        closest.time_parameter
    );
    println!(
        "translating-capsule,minimum_distance_mm,{}",
        closest.distance_mm
    );
    measure("translating-capsule-capsule-narrow-phase", || {
        translating_capsule_vs_capsule(
            black_box(translating),
            black_box(translation),
            black_box(stationary),
        )
        .expect("valid translating capsule narrow-phase query");
    });
    measure("translating-capsule-capsule-endpoint-only", || {
        endpoints_only_translating_capsule_collision(
            black_box(translating),
            black_box(translation),
            black_box(stationary),
        )
        .expect("valid translating capsule endpoint baseline");
    });
    measure("translating-capsule-capsule-aabb-broad-phase", || {
        translating_capsule_aabb_candidate(
            black_box(translating),
            black_box(translation),
            black_box(stationary),
        )
        .expect("valid translating capsule broad-phase query");
    });
    let moving_pair_a = Capsule::new(Vec3::new(-10.0, -2.0, 0.0), Vec3::new(-10.0, 2.0, 0.0), 0.5)
        .expect("valid co-translating capsule A");
    let moving_pair_b = Capsule::new(Vec3::new(10.0, -2.0, 0.0), Vec3::new(10.0, 2.0, 0.0), 0.5)
        .expect("valid co-translating capsule B");
    let moving_pair_a_delta = Vec3::new(20.0, 0.0, 0.0);
    let moving_pair_b_delta = Vec3::new(-20.0, 0.0, 0.0);
    let pair_closest = co_translating_capsule_vs_capsule(
        black_box(moving_pair_a),
        black_box(moving_pair_a_delta),
        black_box(moving_pair_b),
        black_box(moving_pair_b_delta),
    )
    .expect("valid co-translating pair query");
    println!(
        "co-translating-capsules,analytic_collision,{}",
        co_translating_capsule_capsule_collision(
            black_box(moving_pair_a),
            black_box(moving_pair_a_delta),
            black_box(moving_pair_b),
            black_box(moving_pair_b_delta),
        )
        .expect("valid co-translating pair collision")
    );
    println!(
        "co-translating-capsules,endpoint_collision,{}",
        endpoints_only_co_translating_capsule_collision(
            black_box(moving_pair_a),
            black_box(moving_pair_a_delta),
            black_box(moving_pair_b),
            black_box(moving_pair_b_delta),
        )
        .expect("valid co-translating pair endpoint baseline")
    );
    println!(
        "co-translating-capsules,closest_time,{}",
        pair_closest.time_parameter
    );
    println!(
        "co-translating-capsules,minimum_distance_mm,{}",
        pair_closest.distance_mm
    );
    measure("co-translating-capsule-pair-narrow-phase", || {
        co_translating_capsule_vs_capsule(
            black_box(moving_pair_a),
            black_box(moving_pair_a_delta),
            black_box(moving_pair_b),
            black_box(moving_pair_b_delta),
        )
        .expect("valid co-translating pair narrow phase");
    });
    measure("co-translating-capsule-pair-endpoint-only", || {
        endpoints_only_co_translating_capsule_collision(
            black_box(moving_pair_a),
            black_box(moving_pair_a_delta),
            black_box(moving_pair_b),
            black_box(moving_pair_b_delta),
        )
        .expect("valid co-translating pair endpoint baseline");
    });
    measure("co-translating-capsule-pair-swept-aabb", || {
        co_translating_capsule_swept_aabb_candidate(
            black_box(moving_pair_a),
            black_box(moving_pair_a_delta),
            black_box(moving_pair_b),
            black_box(moving_pair_b_delta),
        )
        .expect("valid co-translating pair AABB query");
    });

    for subdivisions in [1_u32, 2, 4, 8, 16, 32] {
        let outcome = co_translating_capsule_time_sliced_aabb_candidates(
            black_box(moving_pair_a),
            black_box(moving_pair_a_delta),
            black_box(moving_pair_b),
            black_box(moving_pair_b_delta),
            subdivisions,
        )
        .expect("valid time-sliced AABB query");
        println!(
            "co-translating-capsules,time-sliced-candidate-slabs-{subdivisions},{}",
            outcome.candidate_slab_count
        );
        println!(
            "co-translating-capsules,time-sliced-candidate-fraction-{subdivisions},{}",
            outcome.candidate_time_fraction()
        );
        measure(
            &format!("co-translating-capsule-time-sliced-aabb-{subdivisions}"),
            || {
                co_translating_capsule_time_sliced_aabb_candidates(
                    black_box(moving_pair_a),
                    black_box(moving_pair_a_delta),
                    black_box(moving_pair_b),
                    black_box(moving_pair_b_delta),
                    subdivisions,
                )
                .expect("valid time-sliced AABB query");
            },
        );
    }
    let timed_moving = CapsuleMotionSpan {
        capsule_at_start: moving_pair_a,
        translation: moving_pair_a_delta,
        start_ms: 0,
        end_ms: 1_000,
        site_epoch: "bench-epoch".into(),
        frame_id: "bench-frame".into(),
        clock_domain_id: "bench-clock".into(),
        clock_uncertainty_ms: 0,
    };
    let timed_obstacle = CapsuleMotionSpan {
        capsule_at_start: moving_pair_b,
        translation: moving_pair_b_delta,
        start_ms: 250,
        end_ms: 750,
        site_epoch: "bench-epoch".into(),
        frame_id: "bench-frame".into(),
        clock_domain_id: "bench-clock".into(),
        clock_uncertainty_ms: 0,
    };
    let timed_closest = timed_co_translating_capsule_vs_capsule(
        black_box(&timed_moving),
        black_box(&timed_obstacle),
    )
    .expect("valid absolute-time capsule query");
    println!(
        "absolute-time-capsules,overlap_start_ms,{}",
        timed_closest.overlap_start_ms
    );
    println!(
        "absolute-time-capsules,overlap_end_ms,{}",
        timed_closest.overlap_end_ms
    );
    println!(
        "absolute-time-capsules,closest_offset_ms,{}",
        timed_closest.closest_time_offset_ms
    );
    measure("absolute-time-capsule-pair-narrow-phase", || {
        black_box(
            timed_co_translating_capsule_vs_capsule(
                black_box(&timed_moving),
                black_box(&timed_obstacle),
            )
            .expect("valid absolute-time capsule query"),
        );
    });
    let mut uncertain_moving = timed_moving.clone();
    let mut uncertain_obstacle = timed_obstacle.clone();
    uncertain_moving.clock_uncertainty_ms = 25;
    uncertain_obstacle.clock_uncertainty_ms = 10;
    let uncertainty_result = clock_uncertainty_aware_capsule_query(
        black_box(&uncertain_moving),
        black_box(&uncertain_obstacle),
    )
    .expect("valid bounded clock-uncertainty query");
    println!(
        "clock-uncertainty-capsules,disposition,{:?}",
        uncertainty_result.disposition
    );
    println!(
        "clock-uncertainty-capsules,moving_margin_mm,{}",
        uncertainty_result.moving_clock_margin_mm
    );
    println!(
        "clock-uncertainty-capsules,obstacle_margin_mm,{}",
        uncertainty_result.obstacle_clock_margin_mm
    );
    measure("clock-uncertainty-capsule-pair-outer-query", || {
        black_box(
            clock_uncertainty_aware_capsule_query(
                black_box(&uncertain_moving),
                black_box(&uncertain_obstacle),
            )
            .expect("valid bounded clock-uncertainty query"),
        );
    });
}
