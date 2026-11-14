use std::{env, fs, path::Path, process};

use civil_aero_benchmark::integrity::verify_source_bytes;

use civil_aero_benchmark::reference_data::parse_ladson_force_data;
use civil_aero_benchmark::section_polar::{
    SectionPolarProvenance, audit_section_polar, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("section-polar audit rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args();
    let program = args
        .next()
        .unwrap_or_else(|| "section_polar_audit".to_owned());
    let input_path = args.next().unwrap_or_else(|| {
        let crate_relative = "data/reference/nasa-tmr/CLCD_Ladson_expdata.dat";
        let repository_relative =
            "tools/civil-aero-benchmark/data/reference/nasa-tmr/CLCD_Ladson_expdata.dat";
        if Path::new(crate_relative).is_file() {
            crate_relative.to_owned()
        } else {
            repository_relative.to_owned()
        }
    });
    if args.next().is_some() {
        return Err(format!("usage: {program} [pinned_ladson_force_data.dat]"));
    }

    let bytes =
        fs::read(&input_path).map_err(|error| format!("cannot read {input_path}: {error}"))?;
    verify_source_bytes(
        &bytes,
        "NASA TMR Ladson force reference",
        1343,
        "78cd2f6aa4968e80f44cbf6c96f699bd9c6e45681d958ec528a10cf72ed23357",
    )?;
    let input = String::from_utf8(bytes)
        .map_err(|error| format!("reference data is not valid UTF-8: {error}"))?;
    let parsed = parse_ladson_force_data(&input)?;
    let report = audit_section_polar(
        &parsed.observations,
        SectionPolarProvenance::nasa_tmr_ladson(),
    )?;
    eprintln!(
        "2D section-polar diagnostic only; fitted section-polar coefficient b is empirical and is not wing induced drag. High-angle bins are not physical operating limits. Exact source byte length and SHA-256 were verified before parsing."
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
