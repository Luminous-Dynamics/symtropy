use std::{env, fs, process};

use civil_aero_benchmark::integrity::verify_source_bytes;
use civil_aero_benchmark::reference_data::{
    ForceObservation, PressureObservation, parse_ladson_force_data, parse_pressure_data,
};

const SOURCE_REVISION: &str = "20a39f549dbff988a6accc421ac84dafc4487695";
const HEADER: &str = "dataset_id,source_sha256,source_revision,observation_kind,zone,source_line,alpha_deg,cl,cd,x_over_c,cp,evidence_level,raw_source_retained";

const FORCE_SHA256: &str = "78cd2f6aa4968e80f44cbf6c96f699bd9c6e45681d958ec528a10cf72ed23357";
const LADSON_PRESSURE_SHA256: &str =
    "547e022634a360cd7697813543348cd447ee0038cdbc8f00a5296c7822ec7af8";
const GREGORY_PRESSURE_SHA256: &str =
    "38d8474bc67661c3cef5e4cd9eeddb95d4a436d06abc1f58a8518066f510647c";

fn main() {
    if let Err(error) = run() {
        eprintln!("reference export rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let dataset = args.next().unwrap_or_else(|| "all".to_owned());
    if args.next().is_some()
        || !matches!(
            dataset.as_str(),
            "all" | "forces" | "ladson-pressure" | "gregory-pressure"
        )
    {
        return Err(
            "usage: reference_export [all|forces|ladson-pressure|gregory-pressure]".to_owned(),
        );
    }

    println!("{HEADER}");
    if dataset == "all" || dataset == "forces" {
        let input = read(
            "CLCD_Ladson_expdata.dat",
            1343,
            FORCE_SHA256,
        )?;
        let parsed = parse_ladson_force_data(&input)?;
        export_forces(&parsed.observations, parsed.dataset_id, FORCE_SHA256);
    }
    if dataset == "all" || dataset == "ladson-pressure" {
        let input = read(
            "CP_Ladson.dat",
            6331,
            LADSON_PRESSURE_SHA256,
        )?;
        let parsed = parse_pressure_data(&input, "NASA-TMR-NACA0012-LADSON-PRESSURE")?;
        export_pressure(
            &parsed.observations,
            parsed.dataset_id,
            LADSON_PRESSURE_SHA256,
        );
    }
    if dataset == "all" || dataset == "gregory-pressure" {
        let input = read(
            "CP_Gregory_expdata.dat",
            1608,
            GREGORY_PRESSURE_SHA256,
        )?;
        let parsed = parse_pressure_data(&input, "NASA-TMR-NACA0012-GREGORY-PRESSURE")?;
        export_pressure(
            &parsed.observations,
            parsed.dataset_id,
            GREGORY_PRESSURE_SHA256,
        );
    }
    Ok(())
}

fn read(name: &str, expected_length: usize, expected_sha256: &str) -> Result<String, String> {
    let path = format!("data/reference/nasa-tmr/{name}");
    let bytes = fs::read(&path)
        .map_err(|error| format!("cannot read pinned reference data {name}: {error}"))?;
    verify_source_bytes(&bytes, name, expected_length, expected_sha256)?;
    String::from_utf8(bytes)
        .map_err(|error| format!("pinned reference data {name} is not UTF-8: {error}"))
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn export_forces(rows: &[ForceObservation], dataset_id: &str, source_sha256: &str) {
    for row in rows {
        println!(
            "{},{},{},{},{},{},{},{},{},,,source_parser_output_unqualified,true",
            csv_escape(dataset_id),
            source_sha256,
            SOURCE_REVISION,
            "force",
            csv_escape(&row.zone),
            row.source_line,
            row.alpha_deg,
            row.cl,
            row.cd
        );
    }
}

fn export_pressure(rows: &[PressureObservation], dataset_id: &str, source_sha256: &str) {
    for row in rows {
        println!(
            "{},{},{},{},{},{},,,,{},{},source_parser_output_unqualified,true",
            csv_escape(dataset_id),
            source_sha256,
            SOURCE_REVISION,
            "pressure",
            csv_escape(&row.zone),
            row.source_line,
            row.x_over_c,
            row.cp
        );
    }
}
