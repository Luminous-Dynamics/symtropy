use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    process,
};

use bh_pt_time_domain_audit::{CompactPulse, SimulationConfig, evolve};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--output" {
        eprintln!("Usage: bh-pt-time-domain-audit --output <waveform.csv>");
        process::exit(2);
    }

    let config = SimulationConfig {
        width: 1.0,
        lambda: 0.75,
        x_min: -16.0,
        x_max: 16.0,
        dx: 0.05,
        dt: 0.0025,
        final_time: 8.0,
        output_interval: 0.01,
        observer_x: 2.0,
        source: CompactPulse {
            center_x: -1.0,
            spatial_half_width: 0.25,
            center_time: 0.6,
            temporal_half_width: 0.3,
            amplitude: 1.0,
        },
    };

    let waveform = evolve(config).unwrap_or_else(|error| {
        eprintln!("time-domain evolution failed: {error}");
        process::exit(1);
    });
    let file = File::create(&args[1]).unwrap_or_else(|error| {
        eprintln!("cannot create {}: {error}", args[1]);
        process::exit(1);
    });
    let mut writer = BufWriter::new(file);
    writeln!(writer, "time,field").unwrap();
    for sample in &waveform.samples {
        writeln!(writer, "{:.12e},{:.16e}", sample.time, sample.value).unwrap();
    }
    writer.flush().unwrap();

    println!(
        "samples={} dx={:.6e} dt={:.6e} max_abs={:.6e} energy={:.6e} source_work={:.6e} energy_balance={:.6e} reflection_free_until={:.6e}",
        waveform.samples.len(),
        waveform.dx,
        waveform.dt,
        waveform.max_abs(),
        waveform.diagnostics.final_energy,
        waveform.diagnostics.source_work,
        waveform.diagnostics.relative_balance_error,
        waveform.diagnostics.boundary_reflection_free_until
    );
}
