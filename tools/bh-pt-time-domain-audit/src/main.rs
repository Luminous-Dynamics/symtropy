use std::{env, fs, path::Path, process};

use bh_pt_time_domain_audit::{CompactPulse, SimulationConfig, evolve};
use serde_json::json;
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}

fn write_error(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(1);
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--output" {
        eprintln!("Usage: bh-pt-time-domain-audit --output <waveform.csv>");
        process::exit(2);
    }

    let output_path = &args[1];
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
        write_error(&format!("time-domain evolution failed: {error}"));
    });

    let mut csv = String::from("time,field\n");
    for sample in &waveform.samples {
        csv.push_str(&format!("{:.12e},{:.16e}\n", sample.time, sample.value));
    }
    fs::write(output_path, csv.as_bytes()).unwrap_or_else(|error| {
        write_error(&format!("cannot write {output_path}: {error}"));
    });

    let config_bytes = serde_json::to_vec(&config).unwrap_or_else(|error| {
        write_error(&format!("cannot serialize simulation configuration: {error}"));
    });
    let config_sha256 = sha256_hex(&config_bytes);
    let csv_sha256 = sha256_hex(csv.as_bytes());
    let first_physical_arrival = config.first_physical_arrival();
    let pre_front_peak = waveform.max_abs_before(first_physical_arrival);
    let waveform_peak = waveform.max_abs();
    let relative_pre_front_leakage = if waveform_peak > 0.0 {
        pre_front_peak / waveform_peak
    } else {
        0.0
    };
    let output_name = Path::new(output_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(output_path);
    let manifest_path = format!("{output_path}.manifest.json");

    let manifest = json!({
        "manifest_schema_version": 1,
        "artifact_type": "symtropy.poeschl_teller.direct_time_domain_run",
        "claim_status": {
            "code_validation": "separate_ci_evidence_required",
            "physical_validation": "direct_pde_reference_only",
            "cross_representation_agreement": "not_claimed",
            "causality": "finite_grid_pre_front_leakage_measured",
            "boundary_condition": "fixed_dirichlet_no_absorbing_boundary_claim"
        },
        "software": {
            "package": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "source_revision": env!("BUILD_SOURCE_REVISION"),
            "target_os": env::consts::OS,
            "target_arch": env::consts::ARCH,
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" }
        },
        "physical_problem": {
            "equation": "(d_t^2 - d_x^2 + V(x)) u(t,x) = S(t,x)",
            "potential": "V(x) = lambda / width^2 * sech^2(x / width)",
            "field_sector": "one_dimensional_scalar_master_equation",
            "field_units": "dimensionless_normalized_pilot",
            "coordinate_units": "dimensionless_x_and_t",
            "fourier_convention": "time_domain_evolution; frequency-domain adapter convention is exp(-i*omega*t)",
            "potential_parameters": {
                "lambda": config.lambda,
                "width": config.width
            },
            "source": {
                "profile": "compact_bump(z) = exp(1 - 1/(1-z^2)) for |z|<1, zero otherwise",
                "spatial_center_x": config.source.center_x,
                "spatial_half_width": config.source.spatial_half_width,
                "temporal_center_t": config.source.center_time,
                "temporal_half_width": config.source.temporal_half_width,
                "amplitude": config.source.amplitude,
                "spatial_support": [
                    config.source.center_x - config.source.spatial_half_width,
                    config.source.center_x + config.source.spatial_half_width
                ],
                "temporal_support": [
                    config.source.start_time(),
                    config.source.end_time()
                ]
            },
            "initial_data": {
                "field": "u(0,x)=0",
                "time_derivative": "du/dt(0,x)=0"
            },
            "boundaries": {
                "left": "Dirichlet u(t,x_min)=0",
                "right": "Dirichlet u(t,x_max)=0",
                "absorbing_boundary": false,
                "earliest_boundary_return_time": config.boundary_reflection_free_until()
            }
        },
        "numerical_method": {
            "method": "method_of_lines",
            "spatial_operator": "second_order_centered_finite_difference",
            "time_integrator": "classical_runge_kutta_order_4",
            "spatial_domain": [config.x_min, config.x_max],
            "dx": config.dx,
            "dt": config.dt,
            "final_time": config.final_time,
            "output_interval": config.output_interval,
            "observer_x": config.observer_x,
            "observer_interpolation": "linear",
            "solver_configuration_sha256": config_sha256
        },
        "waveform": {
            "file": output_name,
            "sha256": csv_sha256,
            "sample_count": waveform.samples.len(),
            "peak_absolute_value": waveform_peak,
            "first_physical_arrival_time": first_physical_arrival,
            "boundary_reflection_free_until": waveform.diagnostics.boundary_reflection_free_until
        },
        "diagnostics": {
            "discrete_final_energy": waveform.diagnostics.final_energy,
            "integrated_source_work": waveform.diagnostics.source_work,
            "relative_energy_work_balance_error": waveform.diagnostics.relative_balance_error,
            "maximum_pre_front_absolute_value": pre_front_peak,
            "relative_pre_front_leakage": relative_pre_front_leakage,
            "pre_front_leakage_note": "This finite-grid RK4 method-of-lines scheme is not exactly finite-propagation-speed. Leakage is measured; the validation suite requires a bounded value and decrease under spatial refinement."
        },
        "reproduction_command": "cargo run --release --manifest-path tools/bh-pt-time-domain-audit/Cargo.toml -- --output <waveform.csv>",
        "source_input_sha256": config_sha256
    });

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_else(|error| {
        write_error(&format!("cannot serialize run manifest: {error}"));
    });
    fs::write(&manifest_path, &manifest_bytes).unwrap_or_else(|error| {
        write_error(&format!("cannot write run manifest {manifest_path}: {error}"));
    });

    println!(
        "samples={} dx={:.6e} dt={:.6e} max_abs={:.6e} energy={:.6e} source_work={:.6e} energy_balance={:.6e} pre_front_leakage={:.6e} reflection_free_until={:.6e}",
        waveform.samples.len(),
        waveform.dx,
        waveform.dt,
        waveform.max_abs(),
        waveform.diagnostics.final_energy,
        waveform.diagnostics.source_work,
        waveform.diagnostics.relative_balance_error,
        relative_pre_front_leakage,
        waveform.diagnostics.boundary_reflection_free_until
    );
    println!("waveform_sha256={csv_sha256}");
    println!("config_sha256={config_sha256}");
    println!("manifest={manifest_path}");
}
