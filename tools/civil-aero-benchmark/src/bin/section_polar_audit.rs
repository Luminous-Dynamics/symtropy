use std::{env, fs, process};

use civil_aero_benchmark::reference_data::parse_ladson_force_data;
use civil_aero_benchmark::section_polar::{audit_section_polar, report_to_csv};

fn main() {
    if let Err(error) = run() {
        eprintln!("section-polar audit rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "section_polar_audit".to_owned());
    let input_path = args.next().unwrap_or_else(|| {
        "tools/civil-aero-benchmark/data/reference/nasa-tmr/CLCD_Ladson_expdata.dat".to_owned()
    });
    if args.next().is_some() {
        return Err(format!("usage: {program} [pinned_ladson_force_data.dat]"));
    }

    let input = fs::read_to_string(&input_path)
        .map_err(|error| format!("cannot read {input_path}: {error}"))?;
    let parsed = parse_ladson_force_data(&input)?;
    let report = audit_section_polar(&parsed.observations)?;
    eprintln!(
        "2D section-polar diagnostic only; fitted Cd^2 coefficient is empirical and is not wing induced drag. High-angle bins are not physical operating limits. Verify raw bytes with the repository SHA256SUMS before treating this as the pinned source."
    );
    for zone in &report.zones {
        eprintln!(
            "zone={} n={} calibration_rmse={:.6} intermediate_rmse={:.6} high_angle_rmse={:.6}",
            zone.zone,
            zone.observation_count,
            zone.calibration.root_mean_square_error,
            zone.intermediate_holdout.root_mean_square_error,
            zone.high_angle_extrapolation.root_mean_square_error
        );
    }
    print!("{}", report_to_csv(&report));
    Ok(())
}
