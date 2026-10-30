use std::{collections::BTreeMap, fs, process};

use civil_aero_benchmark::integrity::verify_source_bytes;
use civil_aero_benchmark::reference_data::{parse_ladson_force_data, parse_pressure_data};

const ROOT: &str = "data/reference/nasa-tmr/";

fn main() {
    if let Err(error) = run() {
        eprintln!("reference-data audit rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let force = read(
        "CLCD_Ladson_expdata.dat",
        1343,
        "78cd2f6aa4968e80f44cbf6c96f699bd9c6e45681d958ec528a10cf72ed23357",
    )?;
    let ladson_pressure = read(
        "CP_Ladson.dat",
        6331,
        "547e022634a360cd7697813543348cd447ee0038cdbc8f00a5296c7822ec7af8",
    )?;
    let gregory_pressure = read(
        "CP_Gregory_expdata.dat",
        1608,
        "38d8474bc67661c3cef5e4cd9eeddb95d4a436d06abc1f58a8518066f510647c",
    )?;

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

fn read(name: &str, expected_length: usize, expected_sha256: &str) -> Result<String, String> {
    let path = format!("{ROOT}{name}");
    let bytes = fs::read(&path)
        .map_err(|error| format!("cannot read pinned reference data {name}: {error}"))?;
    verify_source_bytes(&bytes, name, expected_length, expected_sha256)?;
    String::from_utf8(bytes)
        .map_err(|error| format!("pinned reference data {name} is not UTF-8: {error}"))
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
