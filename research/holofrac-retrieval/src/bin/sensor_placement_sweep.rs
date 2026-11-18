use std::env;

use holofrac_spatial_retrieval::sensor_placement::{
    run_sensor_placement_benchmark, SensorPlacementPolicy,
};

const RANDOM_SEEDS: [u64; 10] = [11, 23, 37, 41, 53, 67, 79, 89, 101, 113];

fn print_report(report: &holofrac_spatial_retrieval::sensor_placement::SensorPlacementReport) {
    println!(
        "{},{},{},{},{},{},{},{},{:.8},{:.8},{:.8},{:.8}",
        report.policy.as_str(),
        report.seed,
        report.candidate_count,
        report.sensor_count,
        report.query_grid_size,
        report.time_samples,
        report.channel_count,
        report.query_count,
        report.mean_nearest_distance,
        report.max_nearest_distance,
        report.mean_absolute_error,
        report.root_mean_squared_error
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac sensor placement and virtual sensing benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("# random_seed_replicates={}", RANDOM_SEEDS.len());
    println!(
        "policy,seed,candidate_count,sensor_count,query_grid_size,time_samples,channel_count,query_count,mean_nearest_distance,max_nearest_distance,mean_absolute_error,root_mean_squared_error"
    );

    for sensor_count in [4_usize, 8, 16, 24] {
        let deterministic =
            run_sensor_placement_benchmark(20, sensor_count, 10, 24, 42)?;

        for policy in [
            SensorPlacementPolicy::RegularGrid,
            SensorPlacementPolicy::GreedyMaximin,
        ] {
            let report = deterministic
                .iter()
                .find(|report| report.policy == policy)
                .expect("deterministic placement policy is present");
            print_report(report);
        }

        for seed in RANDOM_SEEDS {
            let reports =
                run_sensor_placement_benchmark(20, sensor_count, 10, 24, seed)?;
            let report = reports
                .iter()
                .find(|report| report.policy == SensorPlacementPolicy::Random)
                .expect("random placement policy is present");
            print_report(report);
        }
    }

    Ok(())
}
