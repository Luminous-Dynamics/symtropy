use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AdaptiveCollisionStatus, AdaptiveSubdivisionConfig, AnalyticPath, Capsule,
    IntervalVerificationStatus, IntervalVerifierConfig, NormalizedTimeWindow, Vec3,
    co_translating_capsule_adaptive_query_with_certificate, verify_adaptive_collision_certificate,
    replay_interval_verification_receipt, verify_adaptive_no_contact_independently,
    verify_capsule_interval_no_contact,
    chord_collision, co_translating_capsule_adaptive_query,
    co_translating_capsule_capsule_collision, co_translating_capsule_swept_aabb_candidate,
    co_translating_capsule_time_sliced_aabb_candidates, co_translating_capsule_vs_capsule,
    endpoints_only_co_translating_capsule_collision, endpoints_only_collision,
    endpoints_only_translating_capsule_collision, moving_sphere_capsule_collision,
    swept_sphere_capsule_aabb_candidate, translating_capsule_aabb_candidate,
    translating_capsule_capsule_collision, translating_capsule_vs_capsule,
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
    let asynchronous_a = Capsule::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), 0.25)
        .expect("valid asynchronous pair A");
    let asynchronous_b = Capsule::new(Vec3::new(6.0, 0.0, 0.0), Vec3::new(6.0, 0.0, 0.0), 0.25)
        .expect("valid asynchronous pair B");
    let common_translation = Vec3::new(10.0, 0.0, 0.0);
    let adaptive_config = AdaptiveSubdivisionConfig {
        maximum_depth: 8,
        minimum_window_width: 1.0 / 256.0,
    };
    let adaptive_report = co_translating_capsule_adaptive_query(
        black_box(asynchronous_a),
        black_box(common_translation),
        black_box(asynchronous_b),
        black_box(common_translation),
        adaptive_config,
    )
    .expect("valid adaptive asynchronous query");
    assert_eq!(adaptive_report.status, AdaptiveCollisionStatus::NoContact);
    println!(
        "adaptive-asynchronous-pair,status,{:?}",
        adaptive_report.status
    );
    println!(
        "adaptive-asynchronous-pair,node-visits,{}",
        adaptive_report.stats.node_visits
    );
    println!(
        "adaptive-asynchronous-pair,pruned-intervals,{}",
        adaptive_report.stats.pruned_intervals
    );
    println!(
        "adaptive-asynchronous-pair,candidate-windows,{}",
        adaptive_report.stats.candidate_window_count
    );
    println!(
        "adaptive-asynchronous-pair,narrow-phase-calls,{}",
        adaptive_report.stats.exact_narrow_phase_calls
    );
    println!(
        "adaptive-asynchronous-pair,candidate-time-fraction,{}",
        adaptive_report.stats.candidate_time_fraction
    );
    measure("adaptive-space-time-asynchronous-pair", || {
        co_translating_capsule_adaptive_query(
            black_box(asynchronous_a),
            black_box(common_translation),
            black_box(asynchronous_b),
            black_box(common_translation),
            adaptive_config,
        )
        .expect("valid adaptive asynchronous query");
    });
    measure("full-interval-analytic-asynchronous-pair", || {
        co_translating_capsule_vs_capsule(
            black_box(asynchronous_a),
            black_box(common_translation),
            black_box(asynchronous_b),
            black_box(common_translation),
        )
        .expect("valid full interval asynchronous pair");
    });
    measure("capsule-static-aabb-outward", || {
        black_box(asynchronous_a.aabb());
    });
    measure("capsule-whole-sweep-aabb-outward", || {
        co_translating_capsule_swept_aabb_candidate(
            black_box(asynchronous_a),
            black_box(common_translation),
            black_box(asynchronous_b),
            black_box(common_translation),
        )
        .expect("valid outward whole-sweep AABB");
    });
    measure("capsule-one-window-aabb-outward", || {
        co_translating_capsule_time_sliced_aabb_candidates(
            black_box(asynchronous_a),
            black_box(common_translation),
            black_box(asynchronous_b),
            black_box(common_translation),
            1,
        )
        .expect("valid outward one-window AABB");
    });
    let certificate = co_translating_capsule_adaptive_query_with_certificate(
        black_box(moving_pair_a),
        black_box(moving_pair_a_delta),
        black_box(moving_pair_b),
        black_box(moving_pair_b_delta),
        AdaptiveSubdivisionConfig {
            maximum_depth: 8,
            minimum_window_width: 1.0 / 256.0,
        },
    )
    .expect("valid certificate-producing adaptive query");
    verify_adaptive_collision_certificate(black_box(&certificate))
        .expect("generated adaptive certificate verifies");
    println!(
        "adaptive-certificate,leaf_count,{}",
        certificate.leaves.len()
    );
    println!(
        "adaptive-certificate,status,{:?}",
        certificate.report.status
    );
    measure("adaptive-certificate-generation", || {
        black_box(
            co_translating_capsule_adaptive_query_with_certificate(
                black_box(moving_pair_a),
                black_box(moving_pair_a_delta),
                black_box(moving_pair_b),
                black_box(moving_pair_b_delta),
                AdaptiveSubdivisionConfig {
                    maximum_depth: 8,
                    minimum_window_width: 1.0 / 256.0,
                },
            )
            .expect("valid certificate-producing adaptive query"),
        );
    });
    measure("adaptive-certificate-verification", || {
        verify_adaptive_collision_certificate(black_box(&certificate))
            .expect("generated adaptive certificate verifies");
    });


    let interval_moving = Capsule::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        0.6,
    )
    .expect("valid interval verifier moving capsule");
    let interval_obstacle = Capsule::new(
        Vec3::new(0.0, 0.9, 0.9),
        Vec3::new(0.0, 0.9, 0.9),
        0.6,
    )
    .expect("valid interval verifier obstacle capsule");
    let interval_window = NormalizedTimeWindow {
        start_parameter: 0.0,
        end_parameter: 1.0,
    };
    let interval_config = IntervalVerifierConfig::default();
    let interval_receipt = verify_capsule_interval_no_contact(
        black_box(interval_moving),
        black_box(Vec3::new(0.0, 0.0, 0.0)),
        black_box(interval_obstacle),
        black_box(Vec3::new(0.0, 0.0, 0.0)),
        interval_window,
        interval_config,
    )
    .expect("valid independent interval verification");
    assert_eq!(
        interval_receipt.status,
        IntervalVerificationStatus::Clear
    );
    replay_interval_verification_receipt(
        interval_moving,
        Vec3::new(0.0, 0.0, 0.0),
        interval_obstacle,
        Vec3::new(0.0, 0.0, 0.0),
        interval_receipt,
    )
    .expect("independent interval receipt replays");
    println!(
        "independent-interval-verifier,status,{:?}",
        interval_receipt.status
    );
    println!(
        "independent-interval-verifier,visited_boxes,{}",
        interval_receipt.parameter_boxes_visited
    );
    println!(
        "independent-interval-verifier,pruned_boxes,{}",
        interval_receipt.parameter_boxes_pruned
    );
    measure("independent-interval-verifier-generation", || {
        black_box(
            verify_capsule_interval_no_contact(
                black_box(interval_moving),
                black_box(Vec3::new(0.0, 0.0, 0.0)),
                black_box(interval_obstacle),
                black_box(Vec3::new(0.0, 0.0, 0.0)),
                interval_window,
                interval_config,
            )
            .expect("valid independent interval verification"),
        );
    });
    measure("independent-interval-verifier-replay", || {
        replay_interval_verification_receipt(
            interval_moving,
            Vec3::new(0.0, 0.0, 0.0),
            interval_obstacle,
            Vec3::new(0.0, 0.0, 0.0),
            black_box(interval_receipt),
        )
        .expect("independent interval receipt replays");
    });


    let asynchronous_certificate = co_translating_capsule_adaptive_query_with_certificate(
        asynchronous_a,
        common_translation,
        asynchronous_b,
        common_translation,
        adaptive_config,
    )
    .expect("valid asynchronous no-contact certificate");
    assert_eq!(
        asynchronous_certificate.report.status,
        AdaptiveCollisionStatus::NoContact
    );
    let independent_certificate_receipt = verify_adaptive_no_contact_independently(
        black_box(&asynchronous_certificate),
        IntervalVerifierConfig::default(),
    )
    .expect("independent verifier confirms every adaptive no-contact leaf");
    println!(
        "adaptive-independent-verifier,leaf_count,{}",
        independent_certificate_receipt.leaf_receipts.len()
    );
    println!(
        "adaptive-independent-verifier,parameter_boxes_visited,{}",
        independent_certificate_receipt.parameter_boxes_visited
    );
    measure("adaptive-independent-verifier-generation-and-replay", || {
        verify_adaptive_no_contact_independently(
            black_box(&asynchronous_certificate),
            IntervalVerifierConfig::default(),
        )
        .expect("independent verifier confirms every adaptive no-contact leaf");
    });

}
