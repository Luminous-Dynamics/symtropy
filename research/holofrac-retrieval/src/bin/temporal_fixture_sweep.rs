use std::time::Instant;

use holofrac_spatial_retrieval::temporal::{FixtureConfig, TemporalFixture};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha =
        std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac temporal observation fixture v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("seed,sensors,time_steps,channels,events,observed,missing,corrections,delayed,late_before_cutoff,truth_records,cutoff_visible_events,cutoff_visible_missing,cutoff_visible_corrections,generator_us,manifest_digest");

    for sensor_count in [8_usize, 16, 32] {
        for missing_probability in [0.0_f64, 0.1, 0.25] {
            let config = FixtureConfig {
                seed: 0x484f_4c4f_5445_4d50,
                sensor_count,
                time_steps: 48,
                missing_probability,
                correction_probability: 0.08,
                ..FixtureConfig::default()
            };
            let started = Instant::now();
            let fixture = TemporalFixture::generate(config.clone())?;
            let generator_micros = started.elapsed().as_secs_f64() * 1_000_000.0;
            let cutoff_seconds =
                (config.time_steps as i64 / 2) * config.nominal_step_seconds;
            let cutoff_micros = cutoff_seconds.saturating_mul(1_000_000);
            let snapshot = fixture.snapshot_as_of(cutoff_micros);
            let observed = fixture.events.len()
                - fixture.missing_count()
                - fixture.correction_count();
            let late_before_cutoff = fixture
                .events
                .iter()
                .filter(|event| {
                    event.valid_at_micros <= cutoff_micros
                        && event.recorded_at_micros > cutoff_micros
                })
                .count();

            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{:016x}",
                config.seed,
                config.sensor_count,
                config.time_steps,
                holofrac_spatial_retrieval::temporal::SpatialChannel::ALL.len(),
                fixture.events.len(),
                observed,
                fixture.missing_count(),
                fixture.correction_count(),
                fixture.delayed_count(),
                late_before_cutoff,
                fixture.evaluator_truth.len(),
                snapshot.observations.len(),
                snapshot.missing_count(),
                snapshot.correction_count(),
                generator_micros,
                fixture.manifest.digest_non_security
            );
        }
    }

    Ok(())
}
