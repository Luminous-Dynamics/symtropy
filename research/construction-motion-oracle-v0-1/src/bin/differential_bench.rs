use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, Capsule, CapsuleMotionSpan, ClockDomainCapsuleBody, QualifiedClockMapping,
    RelativeTimeMultiBodyConfig, TimedCapsuleBody, Vec3, chord_collision,
    co_translating_capsule_capsule_collision, co_translating_capsule_swept_aabb_candidate,
    co_translating_capsule_time_sliced_aabb_candidates, co_translating_capsule_vs_capsule,
    endpoints_only_co_translating_capsule_collision, endpoints_only_collision,
    endpoints_only_translating_capsule_collision, evaluate_relative_time_multibody_ccd,
    moving_sphere_capsule_collision, swept_sphere_capsule_aabb_candidate,
    time_swept_capsule_broadphase, timed_co_translating_capsule_vs_capsule,
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
    run_relative_time_multibody_benchmarks();
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
                BroadphaseDistribution::Sparse => {
                    (i * 40.0, (index % 31) as f64 * 30.0, (index % 7) as f64 * 20.0, 0.5)
                }
                BroadphaseDistribution::Clustered => {
                    (
                        (index % 32) as f64 * 3.0,
                        ((index / 32) % 16) as f64 * 3.0,
                        (index % 5) as f64 * 1.0,
                        1.0,
                    )
                }
                BroadphaseDistribution::Dense => (0.0, 0.0, 0.0, 1.0),
                BroadphaseDistribution::LongThin => {
                    (i * 15.0, (index % 11) as f64 * 5.0, 0.0, 0.5)
                }
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
            let samples = if matches!(distribution, BroadphaseDistribution::Dense) && count == 1_024 {
                3
            } else {
                21
            };
            let (p50, p95, p99, pair_count, cells, coarse) =
                measure_multibody(&bodies, 8, samples);
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


fn build_clock_mapped_bodies(count: usize) -> Vec<ClockDomainCapsuleBody> {
    let base_ms = 1_000_000_u64;
    (0..count)
        .map(|index| {
            let i = index as f64;
            let x = i * 40.0;
            let y = (index % 17) as f64 * 20.0;
            let clock = format!("robot-clock-{index:05}");
            let capsule = Capsule::new(
                Vec3::new(x, y - 5.0, 0.0),
                Vec3::new(x, y + 5.0, 0.0),
                0.5,
            )
            .expect("benchmark capsule is finite");
            let motion = CapsuleMotionSpan {
                capsule_at_start: capsule,
                translation: Vec3::new(if index % 2 == 0 { 4.0 } else { -4.0 }, 0.0, 0.0),
                start_ms: base_ms,
                end_ms: base_ms + 1_000,
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
                reference_source_time_ms: base_ms + 500,
                offset_min_ms: -2,
                offset_max_ms: 2,
                drift_bound_ppm: Some(20),
                valid_from_source_ms: base_ms,
                valid_until_source_ms: base_ms + 1_000,
            };
            ClockDomainCapsuleBody {
                body_id: format!("robot-{index:05}"),
                motion,
                clock_mapping: Some(mapping),
            }
        })
        .collect()
}

fn run_relative_time_multibody_benchmarks() {
    println!(
        "relative_time_multibody,body_count,pair_checks,unresolved_pairs,potential_contact_pairs,proven_clear_pairs,p50_us,p95_us,p99_us"
    );
    for count in [8_usize, 16, 32, 64] {
        let bodies = build_clock_mapped_bodies(count);
        let config = RelativeTimeMultiBodyConfig {
            site_epoch: "benchmark-site-epoch".into(),
            frame_id: "benchmark-site-world".into(),
            site_clock_domain_id: "benchmark-site-clock".into(),
            max_pair_checks: 524_288,
        };
        let mut durations = Vec::with_capacity(21);
        for _ in 0..21 {
            let start = Instant::now();
            black_box(
                evaluate_relative_time_multibody_ccd(black_box(&bodies), black_box(&config))
                    .expect("bounded multi-clock reference inputs are valid"),
            );
            durations.push(start.elapsed());
        }
        durations.sort_unstable();
        let report = evaluate_relative_time_multibody_ccd(&bodies, &config)
            .expect("bounded multi-clock reference inputs are valid");
        println!(
            "reference,{count},{},{},{},{},{},{},{}",
            report.pair_checks,
            report.unresolved_pairs,
            report.potential_contact_unresolved_pairs,
            report.no_possible_contact_pairs + report.no_possible_temporal_overlap_pairs,
            percentile(&durations, 50).as_micros(),
            percentile(&durations, 95).as_micros(),
            percentile(&durations, 99).as_micros(),
        );
    }
}
