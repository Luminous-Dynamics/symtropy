use std::{collections::BTreeMap, fs, process};

use civil_aero_benchmark::reference_data::{parse_ladson_force_data, parse_pressure_data};

const ROOT: &str = "data/reference/nasa-tmr/";

fn main() {
    if let Err(error) = run() {
        eprintln!("reference-data audit rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let force = read("CLCD_Ladson_expdata.dat")?;
    let ladson_pressure = read("CP_Ladson.dat")?;
    let gregory_pressure = read("CP_Gregory_expdata.dat")?;

    let force = parse_ladson_force_data(&force)?;
    let ladson_pressure =
        parse_pressure_data(&ladson_pressure, "NASA-TMR-NACA0012-LADSON-PRESSURE")?;
    let gregory_pressure =
        parse_pressure_data(&gregory_pressure, "NASA-TMR-NACA0012-GREGORY-PRESSURE")?;

    print_summary(
        force.dataset_id,
        force.observations.len(),
        &force.zone_counts,
        "source_bytes_pinned_parser_output_unqualified",
    );
    print_summary(
        ladson_pressure.dataset_id,
        ladson_pressure.observations.len(),
        &ladson_pressure.zone_counts,
        "source_bytes_pinned_parser_output_unqualified",
    );
    print_summary(
        gregory_pressure.dataset_id,
        gregory_pressure.observations.len(),
        &gregory_pressure.zone_counts,
        "source_bytes_pinned_parser_output_unqualified",
    );
    Ok(())
}

fn read(name: &str) -> Result<String, String> {
    fs::read_to_string(format!("{ROOT}{name}"))
        .map_err(|error| format!("cannot read pinned reference data {name}: {error}"))
}

fn print_summary(
    dataset_id: &str,
    observations: usize,
    zones: &BTreeMap<String, usize>,
    status: &str,
) {
    let zone_details = zones
        .iter()
        .map(|(name, count)| format!("{name}:{count}"))
        .collect::<Vec<_>>()
        .join("|");
    println!(
        "dataset={dataset_id} observations={observations} zones={} zone_counts={zone_details} evidence={status}",
        zones.len()
    );
}
