use sim_rel_joint_stochastic_oracle::{
    benchmark::{generate_suite, BenchmarkSuiteConfig},
    particle_filter::benchmark_particle_counts,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("particle-filter benchmark failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let suite = generate_suite(BenchmarkSuiteConfig {
        seed: 0x5349_4D52_454C_3031,
        instances_per_family: 8,
    })?;
    let particle_counts = [32, 64, 128, 256, 512, 1024, 4096];

    println!(
        "family\tseed\tmanifest_fingerprint\tparticles\texact_predictive_num\texact_predictive_den\tpredictive_abs_error\tjoint_tv\tjoint_sq_error\tmechanism_sq_error\tess_before\tess_after\tsupport_loss\tresampled\telapsed_ns"
    );

    for instance in &suite {
        let nominal = instance
            .worlds
            .iter()
            .find(|world| world.id == instance.nominal_world_id)
            .ok_or_else(|| format!("instance {} has no nominal world", instance.id))?;
        let records = benchmark_particle_counts(
            &nominal.kernel,
            &nominal.kernel.initial_belief,
            0,
            Some(0),
            &particle_counts,
            0.5,
            instance.seed,
        )?;

        for record in records {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{:.12}\t{:.12}\t{:.12}\t{:.12}\t{:.3}\t{:.3}\t{:.6}\t{}\t{}",
                instance.family.as_str(),
                instance.seed,
                instance.fingerprint_hex,
                record.particle_count,
                record.predictive_probability_exact.numerator(),
                record.predictive_probability_exact.denominator(),
                record.predictive_probability_absolute_error,
                record.joint_posterior_total_variation_distance,
                record.joint_posterior_squared_error,
                record.mechanism_posterior_squared_error,
                record.effective_sample_size_before_resampling,
                record.effective_sample_size_after_resampling,
                record.support_loss_fraction,
                u8::from(record.resampled),
                record.elapsed_nanos,
            );
        }
    }
    Ok(())
}
