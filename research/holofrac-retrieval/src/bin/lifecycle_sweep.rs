use std::env;

use holofrac_spatial_retrieval::lifecycle::{LifecycleFixture, LifecycleFixtureConfig};

const YEAR_MICROS: f64 = 31_557_600_000_000.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac component lifecycle ledger benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!(
        "cutoff_years,revision,state,condition,use_class,component_mass_kg,recycled_mass_kg,disposed_mass_kg,loss_mass_kg,mass_residual_kg,visible_events,manifest_digest"
    );

    let fixture = LifecycleFixture::generate(LifecycleFixtureConfig::default())?;
    fixture.validate()?;

    for cutoff_years in [0.0_f64, 6.0, 10.5, 11.5, 16.0, 20.5, 30.5, 31.5] {
        let cutoff_micros = (cutoff_years * YEAR_MICROS) as i64;
        let snapshot = fixture.ledger.snapshot_as_of(cutoff_micros)?;
        let accounting = fixture.ledger.accounting_as_of(cutoff_micros)?;

        println!(
            "{:.1},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.12},{},{:016x}",
            cutoff_years,
            snapshot.revision,
            snapshot.state.as_str(),
            snapshot.condition.as_str(),
            snapshot.use_class,
            snapshot.mass_kg,
            accounting.recycled_mass_kg,
            accounting.disposed_mass_kg,
            accounting.loss_mass_kg,
            accounting.residual_kg,
            snapshot.visible_event_count,
            fixture.manifest.digest_non_security
        );
    }

    Ok(())
}
