use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, Capsule, CapsuleMotionSpan, ClockDomainCapsuleBody, QualifiedClockMapping,
    RelativeTimeBroadphaseConfig, RelativeTimeMultiBodyConfig, TimedCapsuleBody, Vec3,
    chord_collision, co_translating_capsule_capsule_collision,
    co_translating_capsule_swept_aabb_candidate,
    co_translating_capsule_time_sliced_aabb_candidates, co_translating_capsule_vs_capsule,
    endpoints_only_co_translating_capsule_collision, endpoints_only_collision,
    endpoints_only_translating_capsule_collision, evaluate_relative_time_multibody_ccd,
    evaluate_relative_time_multibody_ccd_with_broadphase, moving_sphere_capsule_collision,
    swept_sphere_capsule_aabb_candidate, time_swept_capsule_broadphase,
    time_swept_capsule_broadphase_hierarchical, timed_co_translating_capsule_vs_capsule,
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
    println!("multibody,time_swept_broadphase");
    run_multibody_broadphase_benchmarks();

    measure("absolute-time-capsule-pair-narrow-phase", || {
        black_box(
            timed_co_translating_capsule_vs_capsule(
                black_box(&timed_moving),
                black_box(&timed_obstacle),
            )
            .expect("valid absolute-time capsule query"),
        );
    });
    run_relative_time_multibody_comparison_benchmarks();
    run_hierarchical_sap_algorithm_benchmarks();
}

#[derive(Clone, Copy)]
enum BroadphaseDistribution {
    Sparse,
    Clustered,
    Dense,
    LongThin,
}

impl BroadphaseDistribution {
    fn label(self) -> &'static str {
        match self {
            Self::Sparse => "sparse",
            Self::Clustered => "clustered",
            Self::Dense => "dense",
            Self::LongThin => "long-thin",
        }
    }
}

fn build_broadphase_bodies(
    count: usize,
    distribution: BroadphaseDistribution,
) -> Vec<TimedCapsuleBody> {
    (0..count)
        .map(|index| {
            let i = index as f64;
            let (x, y, z, radius) = match distribution {
                BroadphaseDistribution::Sparse => (
                    i * 40.0,
                    (index % 31) as f64 * 30.0,
                    (index % 7) as f64 * 20.0,
                    0.5,
                ),
                BroadphaseDistribution::Clustered => (
                    (index % 32) as f64 * 3.0,
                    ((index / 32) % 16) as f64 * 3.0,
                    (index % 5) as f64 * 1.0,
                    1.0,
                ),
                BroadphaseDistribution::Dense => (0.0, 0.0, 0.0, 1.0),
                BroadphaseDistribution::LongThin => (i * 15.0, (index % 11) as f64 * 5.0, 0.0, 0.5),
            };
            let half_length = if matches!(distribution, BroadphaseDistribution::LongThin) {
                25.0
            } else {
                1.0
            };
            let translation = match distribution {
                BroadphaseDistribution::Sparse => Vec3::new(2.0, 0.0, 0.0),
                BroadphaseDistribution::Clustered => {
                    if index % 2 == 0 {
                        Vec3::new(10.0, 0.0, 0.0)
                    } else {
                        Vec3::new(-10.0, 0.0, 0.0)
                    }
                }
                BroadphaseDistribution::Dense => {
                    Vec3::new(if index % 2 == 0 { 1.0 } else { -1.0 }, 0.0, 0.0)
                }
                BroadphaseDistribution::LongThin => Vec3::new(0.0, 5.0, 0.0),
            };
            TimedCapsuleBody {
                body_id: format!("body-{index:06}"),
                motion: CapsuleMotionSpan {
                    capsule_at_start: Capsule::new(
                        Vec3::new(x, y, z - half_length),
                        Vec3::new(x, y, z + half_length),
                        radius,
                    )
                    .expect("benchmark capsule is finite"),
                    translation,
                    start_ms: (index % 8) as u64 * 100,
                    end_ms: 1_000 + (index % 8) as u64 * 100,
                    site_epoch: "benchmark-epoch".into(),
                    frame_id: "benchmark-frame".into(),
                    clock_domain_id: "benchmark-clock".into(),
                    clock_uncertainty_ms: 0,
                },
            }
        })
        .collect()
}

fn measure_multibody(
    bodies: &[TimedCapsuleBody],
    subdivisions: u32,
    samples: usize,
) -> (Duration, Duration, Duration, usize, u64, u64) {
    assert!(samples > 0, "benchmark sample count must be positive");
    let mut durations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        black_box(
            time_swept_capsule_broadphase(black_box(bodies), subdivisions)
                .expect("benchmark broadphase inputs are valid"),
        );
        durations.push(start.elapsed());
    }
    durations.sort_unstable();
    let report = time_swept_capsule_broadphase(bodies, subdivisions)
        .expect("benchmark broadphase inputs are valid");
    (
        percentile(&durations, 50),
        percentile(&durations, 95),
        percentile(&durations, 99),
        report.pairs.len(),
        report.active_body_slab_cells,
        report.total_coarse_spatial_pairs,
    )
}

fn whole_horizon_all_pairs_candidate_count(bodies: &[TimedCapsuleBody]) -> u64 {
    let bounds: Vec<_> = bodies
        .iter()
        .map(|body| {
            let start = body.motion.capsule_at_start;
            let delta = body.motion.translation;
            let end = Capsule::new(
                Vec3::new(
                    start.start.x + delta.x,
                    start.start.y + delta.y,
                    start.start.z + delta.z,
                ),
                Vec3::new(
                    start.end.x + delta.x,
                    start.end.y + delta.y,
                    start.end.z + delta.z,
                ),
                start.radius_mm,
            )
            .expect("benchmark final pose is valid");
            let first = start.aabb();
            let last = end.aabb();
            (
                Vec3::new(
                    first.min.x.min(last.min.x),
                    first.min.y.min(last.min.y),
                    first.min.z.min(last.min.z),
                ),
                Vec3::new(
                    first.max.x.max(last.max.x),
                    first.max.y.max(last.max.y),
                    first.max.z.max(last.max.z),
                ),
            )
        })
        .collect();

    let mut candidates = 0_u64;
    for left in 0..bodies.len() {
        for right in (left + 1)..bodies.len() {
            let temporal = bodies[left]
                .motion
                .start_ms
                .max(bodies[right].motion.start_ms)
                < bodies[left].motion.end_ms.min(bodies[right].motion.end_ms);
            if !temporal {
                continue;
            }
            let (left_min, left_max) = bounds[left];
            let (right_min, right_max) = bounds[right];
            let overlap = left_min.x <= right_max.x
                && left_max.x >= right_min.x
                && left_min.y <= right_max.y
                && left_max.y >= right_min.y
                && left_min.z <= right_max.z
                && left_max.z >= right_min.z;
            if overlap {
                candidates += 1;
            }
        }
    }
    candidates
}

fn measure_whole_horizon_all_pairs(
    bodies: &[TimedCapsuleBody],
    samples: usize,
) -> (Duration, Duration, Duration, u64) {
    assert!(samples > 0, "benchmark sample count must be positive");
    let mut durations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        black_box(whole_horizon_all_pairs_candidate_count(black_box(bodies)));
        durations.push(start.elapsed());
    }
    durations.sort_unstable();
    (
        percentile(&durations, 50),
        percentile(&durations, 95),
        percentile(&durations, 99),
        whole_horizon_all_pairs_candidate_count(bodies),
    )
}

fn run_multibody_broadphase_benchmarks() {
    println!(
        "distribution,body_count,slabs,method,p50_us,p95_us,p99_us,unique_pairs,active_cells,coarse_spatial_pairs,theoretical_all_pairs"
    );
    for count in [16_usize, 64, 256, 1_024, 4_096, 10_000] {
        let bodies = build_broadphase_bodies(count, BroadphaseDistribution::Sparse);
        let (p50, p95, p99, pair_count, cells, coarse) = measure_multibody(&bodies, 8, 21);
        let all_pairs = (count as u64) * ((count - 1) as u64) / 2;
        println!(
            "sparse,{count},8,dyadic-sap,{},{},{},{pair_count},{cells},{coarse},{all_pairs}",
            p50.as_micros(),
            p95.as_micros(),
            p99.as_micros()
        );
        if count <= 1_024 {
            let (p50, p95, p99, candidates) = measure_whole_horizon_all_pairs(&bodies, 21);
            println!(
                "sparse,{count},1,whole-horizon-all-pairs,{},{},{},{candidates},0,0,{all_pairs}",
                p50.as_micros(),
                p95.as_micros(),
                p99.as_micros()
            );
        }
    }

    for distribution in [
        BroadphaseDistribution::Clustered,
        BroadphaseDistribution::LongThin,
        BroadphaseDistribution::Dense,
    ] {
        for count in [16_usize, 64, 256, 1_024] {
            let bodies = build_broadphase_bodies(count, distribution);
            let samples = if matches!(distribution, BroadphaseDistribution::Dense) && count == 1_024
            {
                3
            } else {
                21
            };
            let (p50, p95, p99, pair_count, cells, coarse) = measure_multibody(&bodies, 8, samples);
            let all_pairs = (count as u64) * ((count - 1) as u64) / 2;
            println!(
                "{},{count},8,dyadic-sap,{},{},{},{pair_count},{cells},{coarse},{all_pairs}",
                distribution.label(),
                p50.as_micros(),
                p95.as_micros(),
                p99.as_micros()
            );
        }
    }
}

#[derive(Clone, Copy)]
enum ClockBroadphaseDistribution {
    Sparse,
    Clustered,
    Dense,
    LongThin,
    TimeStaggered,
    HighDrift,
}

impl ClockBroadphaseDistribution {
    fn label(self) -> &'static str {
        match self {
            Self::Sparse => "sparse",
            Self::Clustered => "clustered",
            Self::Dense => "dense",
            Self::LongThin => "long-thin",
            Self::TimeStaggered => "time-staggered",
            Self::HighDrift => "high-drift",
        }
    }
}

fn build_clock_mapped_bodies_with_distribution(
    count: usize,
    distribution: ClockBroadphaseDistribution,
) -> Vec<ClockDomainCapsuleBody> {
    let base_ms = 1_000_000_u64;
    (0..count)
        .map(|index| {
            let i = index as f64;
            let (
                x,
                y,
                half_length,
                radius,
                translation_x,
                translation_y,
                start_offset_ms,
                duration_ms,
                offset_bound_ms,
                drift_bound_ppm,
                horizontal,
            ) = match distribution {
                ClockBroadphaseDistribution::Sparse => (
                    i * 40.0,
                    (index % 17) as f64 * 20.0,
                    5.0,
                    0.5,
                    if index % 2 == 0 { 4.0 } else { -4.0 },
                    0.0,
                    0_u64,
                    1_000_u64,
                    2_i64,
                    20_u64,
                    false,
                ),
                ClockBroadphaseDistribution::Clustered => (
                    (index % 16) as f64 * 3.0,
                    ((index / 16) % 16) as f64 * 3.0,
                    2.0,
                    0.75,
                    if index % 2 == 0 { 8.0 } else { -8.0 },
                    0.0,
                    0,
                    1_000,
                    4,
                    100,
                    false,
                ),
                ClockBroadphaseDistribution::Dense => (
                    0.0,
                    0.0,
                    2.0,
                    1.0,
                    if index % 2 == 0 { 1.0 } else { -1.0 },
                    0.0,
                    0,
                    1_000,
                    4,
                    100,
                    false,
                ),
                ClockBroadphaseDistribution::LongThin => (
                    i * 15.0,
                    (index % 11) as f64 * 5.0,
                    25.0,
                    0.5,
                    0.0,
                    5.0,
                    0,
                    1_000,
                    4,
                    100,
                    true,
                ),
                ClockBroadphaseDistribution::TimeStaggered => (
                    0.0,
                    0.0,
                    3.0,
                    0.75,
                    if index % 2 == 0 { 2.0 } else { -2.0 },
                    0.0,
                    (index % 8) as u64 * 200,
                    250,
                    0,
                    0,
                    false,
                ),
                ClockBroadphaseDistribution::HighDrift => (
                    i * 12.0,
                    (index % 13) as f64 * 8.0,
                    4.0,
                    0.5,
                    if index % 2 == 0 { 4.0 } else { -4.0 },
                    0.0,
                    0,
                    1_000,
                    50,
                    100_000,
                    false,
                ),
            };
            let clock = format!("robot-clock-{index:05}");
            let (start, end) = if horizontal {
                (
                    Vec3::new(x - half_length, y, 0.0),
                    Vec3::new(x + half_length, y, 0.0),
                )
            } else {
                (
                    Vec3::new(x, y - half_length, 0.0),
                    Vec3::new(x, y + half_length, 0.0),
                )
            };
            let capsule = Capsule::new(start, end, radius).expect("benchmark capsule is finite");
            let start_ms = base_ms + start_offset_ms;
            let end_ms = start_ms + duration_ms;
            let motion = CapsuleMotionSpan {
                capsule_at_start: capsule,
                translation: Vec3::new(translation_x, translation_y, 0.0),
                start_ms,
                end_ms,
                site_epoch: "benchmark-site-epoch".into(),
                frame_id: "benchmark-site-world".into(),
                clock_domain_id: clock.clone(),
                clock_uncertainty_ms: 0,
            };
            let mapping = QualifiedClockMapping {
                source_clock_domain_id: clock.clone(),
                site_clock_domain_id: "benchmark-site-clock".into(),
                site_epoch: "benchmark-site-epoch".into(),
                mapping_id: format!("mapping-{index:05}"),
                evidence_ref: format!("synthetic://clock-evidence/{index:05}"),
                reference_source_time_ms: start_ms + duration_ms / 2,
                offset_min_ms: -offset_bound_ms,
                offset_max_ms: offset_bound_ms,
                drift_bound_ppm: Some(drift_bound_ppm),
                valid_from_source_ms: base_ms,
                valid_until_source_ms: base_ms + 100_000,
            };
            ClockDomainCapsuleBody {
                body_id: format!("robot-{index:05}"),
                motion,
                clock_mapping: Some(mapping),
            }
        })
        .collect()
}

fn run_relative_time_multibody_comparison_benchmarks() {
    println!(
        "distribution,body_count,subdivisions,reference_pairs,outer_bodies,coarse_candidates,refined_candidates,refinement_pair_slabs,pairwise_checks,outer_pruned,reference_unresolved,broadphase_unresolved,noncontact_candidates,ref_p50_us,ref_p95_us,ref_p99_us,bp_p50_us,bp_p95_us,bp_p99_us,p50_speedup"
    );

    let distributions = [
        ClockBroadphaseDistribution::Sparse,
        ClockBroadphaseDistribution::Clustered,
        ClockBroadphaseDistribution::Dense,
        ClockBroadphaseDistribution::LongThin,
        ClockBroadphaseDistribution::TimeStaggered,
        ClockBroadphaseDistribution::HighDrift,
    ];
    for distribution in distributions {
        let counts: &[usize] = if matches!(distribution, ClockBroadphaseDistribution::Sparse) {
            &[8, 16, 32, 64, 128, 256, 512]
        } else {
            &[32, 64, 128, 256]
        };
        for count in counts {
            let bodies = build_clock_mapped_bodies_with_distribution(*count, distribution);
            let clock_config = RelativeTimeMultiBodyConfig {
                site_epoch: "benchmark-site-epoch".into(),
                frame_id: "benchmark-site-world".into(),
                site_clock_domain_id: "benchmark-site-clock".into(),
                max_pair_checks: 524_288,
            };
            let reference_report = evaluate_relative_time_multibody_ccd(&bodies, &clock_config)
                .expect("bounded exhaustive multi-clock inputs are valid");
            let samples =
                if matches!(distribution, ClockBroadphaseDistribution::Dense) && *count >= 128 {
                    7
                } else {
                    11
                };

            for subdivision_count in [1_u32, 2, 4, 8, 16] {
                let broadphase_config = RelativeTimeBroadphaseConfig {
                    clock_mapping: clock_config.clone(),
                    subdivision_count,
                };
                let broadphase_report = evaluate_relative_time_multibody_ccd_with_broadphase(
                    &bodies,
                    &broadphase_config,
                )
                .expect("bounded clock-aware broadphase inputs are valid");

                // Warm both paths once before timing to reduce cold-start bias.
                black_box(
                    evaluate_relative_time_multibody_ccd(&bodies, &clock_config)
                        .expect("bounded exhaustive multi-clock inputs are valid"),
                );
                black_box(
                    evaluate_relative_time_multibody_ccd_with_broadphase(
                        &bodies,
                        &broadphase_config,
                    )
                    .expect("bounded clock-aware broadphase inputs are valid"),
                );

                let mut reference_durations = Vec::with_capacity(samples);
                let mut broadphase_durations = Vec::with_capacity(samples);
                for sample in 0..samples {
                    if sample % 2 == 0 {
                        let start = Instant::now();
                        black_box(
                            evaluate_relative_time_multibody_ccd(
                                black_box(&bodies),
                                black_box(&clock_config),
                            )
                            .expect("bounded exhaustive multi-clock inputs are valid"),
                        );
                        reference_durations.push(start.elapsed());

                        let start = Instant::now();
                        black_box(
                            evaluate_relative_time_multibody_ccd_with_broadphase(
                                black_box(&bodies),
                                black_box(&broadphase_config),
                            )
                            .expect("bounded clock-aware broadphase inputs are valid"),
                        );
                        broadphase_durations.push(start.elapsed());
                    } else {
                        let start = Instant::now();
                        black_box(
                            evaluate_relative_time_multibody_ccd_with_broadphase(
                                black_box(&bodies),
                                black_box(&broadphase_config),
                            )
                            .expect("bounded clock-aware broadphase inputs are valid"),
                        );
                        broadphase_durations.push(start.elapsed());

                        let start = Instant::now();
                        black_box(
                            evaluate_relative_time_multibody_ccd(
                                black_box(&bodies),
                                black_box(&clock_config),
                            )
                            .expect("bounded exhaustive multi-clock inputs are valid"),
                        );
                        reference_durations.push(start.elapsed());
                    }
                }
                reference_durations.sort_unstable();
                broadphase_durations.sort_unstable();

                if *count <= 64 {
                    for pair in reference_report.pairs.iter().filter(|pair| {
                        matches!(
                            pair.disposition,
                            construction_motion_oracle::RelativeTimePairDisposition::ExactContact
                                | construction_motion_oracle::RelativeTimePairDisposition::BoundaryCandidateUnresolved
                                | construction_motion_oracle::RelativeTimePairDisposition::PotentialContactUnresolved
                        )
                    }) {
                        assert!(
                            broadphase_report.pairs.iter().any(|candidate| {
                                candidate.first_body_id == pair.first_body_id
                                    && candidate.second_body_id == pair.second_body_id
                            }),
                            "clock-aware broad phase pruned a reference potential pair in {} at {} slabs",
                            distribution.label(),
                            subdivision_count
                        );
                    }
                }

                assert_eq!(
                    broadphase_report.reference_pair_count,
                    broadphase_report.broadphase_candidate_pairs
                        + broadphase_report.outer_space_time_pruned_pairs
                        + broadphase_report.unresolved_pairs,
                    "pair accounting does not reconcile in {} at {} slabs",
                    distribution.label(),
                    subdivision_count
                );
                assert_eq!(
                    broadphase_report.pairwise_checked_pairs,
                    broadphase_report.broadphase_candidate_pairs
                );

                let reference_p50 = percentile(&reference_durations, 50);
                let reference_p95 = percentile(&reference_durations, 95);
                let reference_p99 = percentile(&reference_durations, 99);
                let broadphase_p50 = percentile(&broadphase_durations, 50);
                let broadphase_p95 = percentile(&broadphase_durations, 95);
                let broadphase_p99 = percentile(&broadphase_durations, 99);
                let speedup =
                    reference_p50.as_nanos() as f64 / broadphase_p50.as_nanos().max(1) as f64;

                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    distribution.label(),
                    count,
                    subdivision_count,
                    reference_report.pair_checks,
                    broadphase_report.valid_outer_envelope_bodies,
                    broadphase_report.coarse_broadphase_candidate_pairs,
                    broadphase_report.broadphase_candidate_pairs,
                    broadphase_report.refinement_tested_pair_slabs,
                    broadphase_report.pairwise_checked_pairs,
                    broadphase_report.outer_space_time_pruned_pairs,
                    reference_report.unresolved_pairs,
                    broadphase_report.unresolved_pairs,
                    broadphase_report.pairwise_noncontact_candidates,
                    reference_p50.as_micros(),
                    reference_p95.as_micros(),
                    reference_p99.as_micros(),
                    broadphase_p50.as_micros(),
                    broadphase_p95.as_micros(),
                    broadphase_p99.as_micros(),
                    speedup,
                );
            }
        }
    }
}

fn build_site_time_sap_bodies(bodies: &[ClockDomainCapsuleBody]) -> Vec<TimedCapsuleBody> {
    bodies
        .iter()
        .map(|body| {
            let mut motion = body.motion.clone();
            if let Some(mapping) = body.clock_mapping.as_ref() {
                let midpoint =
                    (i128::from(mapping.offset_min_ms) + i128::from(mapping.offset_max_ms)) / 2;
                motion.start_ms = u64::try_from(i128::from(motion.start_ms) + midpoint)
                    .expect("synthetic benchmark site start is representable");
                motion.end_ms = u64::try_from(i128::from(motion.end_ms) + midpoint)
                    .expect("synthetic benchmark site end is representable");
            }
            motion.clock_domain_id = "benchmark-site-clock".into();
            // This lane isolates the broadphase indexing/refinement algorithms.
            // Clock-map uncertainty expansion is benchmarked by the integrated
            // relative-time lane above, not hidden in this zero-uncertainty SAP-only comparison.
            motion.clock_uncertainty_ms = 0;
            TimedCapsuleBody {
                body_id: body.body_id.clone(),
                motion,
            }
        })
        .collect()
}

fn run_hierarchical_sap_algorithm_benchmarks() {
    println!(
        "distribution,body_count,subdivisions,possible_pairs,direct_candidates,hierarchical_coarse_candidates,hierarchical_refined_candidates,refinement_pair_slabs,direct_p50_us,direct_p95_us,direct_p99_us,hierarchical_p50_us,hierarchical_p95_us,hierarchical_p99_us,hierarchical_speedup"
    );
    let distributions = [
        ClockBroadphaseDistribution::Sparse,
        ClockBroadphaseDistribution::Clustered,
        ClockBroadphaseDistribution::Dense,
        ClockBroadphaseDistribution::LongThin,
        ClockBroadphaseDistribution::TimeStaggered,
        ClockBroadphaseDistribution::HighDrift,
    ];

    for distribution in distributions {
        for count in [32_usize, 64, 128, 256] {
            let source = build_clock_mapped_bodies_with_distribution(count, distribution);
            let bodies = build_site_time_sap_bodies(&source);
            let possible_pairs = (count as u64) * (count.saturating_sub(1) as u64) / 2;

            for subdivisions in [2_u32, 4, 8, 16] {
                let direct = time_swept_capsule_broadphase(&bodies, subdivisions)
                    .expect("matched SAP benchmark input must be valid");
                let hierarchical =
                    time_swept_capsule_broadphase_hierarchical(&bodies, subdivisions)
                        .expect("hierarchical SAP benchmark input must be valid");
                assert_eq!(
                    hierarchical.refinement.pairs,
                    direct.pairs,
                    "hierarchical SAP candidates diverged for {} / {} bodies / {} slabs",
                    distribution.label(),
                    count,
                    subdivisions
                );

                // One warm-up call per path; then alternate execution order.
                black_box(
                    time_swept_capsule_broadphase(&bodies, subdivisions)
                        .expect("matched SAP warm-up must be valid"),
                );
                black_box(
                    time_swept_capsule_broadphase_hierarchical(&bodies, subdivisions)
                        .expect("hierarchical SAP warm-up must be valid"),
                );
                let mut direct_durations = Vec::with_capacity(9);
                let mut hierarchical_durations = Vec::with_capacity(9);
                for sample in 0..9 {
                    if sample % 2 == 0 {
                        let start = Instant::now();
                        black_box(
                            time_swept_capsule_broadphase(&bodies, subdivisions)
                                .expect("matched SAP timing input must be valid"),
                        );
                        direct_durations.push(start.elapsed());

                        let start = Instant::now();
                        black_box(
                            time_swept_capsule_broadphase_hierarchical(&bodies, subdivisions)
                                .expect("hierarchical SAP timing input must be valid"),
                        );
                        hierarchical_durations.push(start.elapsed());
                    } else {
                        let start = Instant::now();
                        black_box(
                            time_swept_capsule_broadphase_hierarchical(&bodies, subdivisions)
                                .expect("hierarchical SAP timing input must be valid"),
                        );
                        hierarchical_durations.push(start.elapsed());

                        let start = Instant::now();
                        black_box(
                            time_swept_capsule_broadphase(&bodies, subdivisions)
                                .expect("matched SAP timing input must be valid"),
                        );
                        direct_durations.push(start.elapsed());
                    }
                }
                direct_durations.sort_unstable();
                hierarchical_durations.sort_unstable();
                let direct_p50 = percentile(&direct_durations, 50);
                let direct_p95 = percentile(&direct_durations, 95);
                let direct_p99 = percentile(&direct_durations, 99);
                let hierarchical_p50 = percentile(&hierarchical_durations, 50);
                let hierarchical_p95 = percentile(&hierarchical_durations, 95);
                let hierarchical_p99 = percentile(&hierarchical_durations, 99);
                let speedup =
                    direct_p50.as_nanos() as f64 / hierarchical_p50.as_nanos().max(1) as f64;

                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    distribution.label(),
                    count,
                    subdivisions,
                    possible_pairs,
                    direct.pairs.len(),
                    hierarchical.coarse.pairs.len(),
                    hierarchical.refinement.pairs.len(),
                    hierarchical.refinement.tested_pair_slabs,
                    direct_p50.as_micros(),
                    direct_p95.as_micros(),
                    direct_p99.as_micros(),
                    hierarchical_p50.as_micros(),
                    hierarchical_p95.as_micros(),
                    hierarchical_p99.as_micros(),
                    speedup,
                );
            }
        }
    }
}
