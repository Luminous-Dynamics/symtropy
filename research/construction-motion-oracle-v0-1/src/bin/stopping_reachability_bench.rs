use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    StoppingDynamicsQualification, StoppingReachabilityRequest,
    calculate_stopping_reachability,
};

const BATCH_SIZE: usize = 20_000;
const SAMPLE_COUNT: usize = 31;

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let index = (samples.len() - 1) * percentile / 100;
    samples[index]
}

fn main() {
    let qualification = StoppingDynamicsQualification {
        profile_sha256: "1111111111111111111111111111111111111111111111111111111111111111".into(),
        qualification_receipt_sha256:
            "2222222222222222222222222222222222222222222222222222222222222222".into(),
        robot_brake_configuration_sha256:
            "3333333333333333333333333333333333333333333333333333333333333333".into(),
        operating_condition_sha256:
            "4444444444444444444444444444444444444444444444444444444444444444".into(),
        maximum_qualified_speed_mm_s: 2_500.0,
        maximum_detection_to_brake_latency_ms: 150,
        maximum_positive_acceleration_mm_s2: 750.0,
        minimum_net_deceleration_mm_s2: 850.0,
        minimum_braking_jerk_mm_s3: 1_500.0,
        additional_distance_guard_mm: 10.0,
    };

    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut checksum = 0_u128;
    for sample in 0..SAMPLE_COUNT {
        let start = Instant::now();
        for index in 0..BATCH_SIZE {
            let speed = ((index * 37 + sample * 13) % 2_500) as f64 + 0.25;
            let result = calculate_stopping_reachability(
                black_box(&qualification),
                black_box(StoppingReachabilityRequest {
                    speed_upper_mm_s: speed,
                }),
            )
            .expect("benchmark input must be inside the qualified profile");
            checksum = checksum.wrapping_add(u128::from(
                black_box(result.conservative_distance_bound_mm),
            ));
        }
        samples.push(start.elapsed());
    }
    samples.sort_unstable();

    let p50 = percentile(&samples, 50);
    let p95 = percentile(&samples, 95);
    let p99 = percentile(&samples, 99);
    let median_ns_per_call = p50.as_nanos() / BATCH_SIZE as u128;
    let throughput_per_second =
        BATCH_SIZE as f64 / p50.as_secs_f64();

    println!(
        "stopping_reachability,samples,{},calls_per_sample,{},p50_us,{:.3},p95_us,{:.3},p99_us,{:.3},median_ns_per_call,{},median_calls_per_second,{:.0},checksum,{}",
        SAMPLE_COUNT,
        BATCH_SIZE,
        p50.as_secs_f64() * 1_000_000.0,
        p95.as_secs_f64() * 1_000_000.0,
        p99.as_secs_f64() * 1_000_000.0,
        median_ns_per_call,
        throughput_per_second,
        checksum
    );
}
