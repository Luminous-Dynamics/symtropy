use std::{env, fs, process};

use civil_aero_benchmark::reference_data::{
    parse_ladson_force_data, parse_pressure_data, ForceObservation, ParsedReference,
    PressureObservation,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("reference export rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let dataset = args.next().unwrap_or_else(|| "all".to_owned());
    if args.next().is_some() || !matches!(dataset.as_str(), "all" | "forces" | "ladson-pressure" | "gregory-pressure") {
        return Err(
            "usage: reference_export [all|forces|ladson-pressure|gregory-pressure]".to_owned(),
        );
    }

    if dataset == "all" || dataset == "forces" {
        let input = read("CLCD_Ladson_expdata.dat")?;
        let parsed = parse_ladson_force_data(&input)?;
        export_forces(&parsed);
    }
    if dataset == "all" || dataset == "ladson-pressure" {
        let input = read("CP_Ladson.dat")?;
        let parsed = parse_pressure_data(&input, "NASA-TMR-NACA0012-LADSON-PRESSURE")?;
        export_pressure(&parsed);
    }
    if dataset == "all" || dataset == "gregory-pressure" {
        let input = read("CP_Gregory_expdata.dat")?;
        let parsed = parse_pressure_data(&input, "NASA-TMR-NACA0012-GREGORY-PRESSURE")?;
        export_pressure(&parsed);
    }
    Ok(())
}

fn read(name: &str) -> Result<String, String> {
    fs::read_to_string(format!("data/reference/nasa-tmr/{name}"))
        .map_err(|error| format!("cannot read pinned reference data {name}: {error}"))
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn export_forces(dataset: &ParsedReference<ForceObservation>) {
    println!(
        "dataset_id,zone,source_line,alpha_deg,cl,cd,evidence_level,raw_source_retained"
    );
    for row in &dataset.observations {
        println!(
            "{},{},{},{},{},{},source_parser_output_unqualified,true",
            csv_escape(dataset.dataset_id),
            csv_escape(&row.zone),
            row.source_line,
            row.alpha_deg,
            row.cl,
            row.cd
        );
    }
}

fn export_pressure(dataset: &ParsedReference<PressureObservation>) {
    println!(
        "dataset_id,zone,source_line,x_over_c,cp,evidence_level,raw_source_retained"
    );
    for row in &dataset.observations {
        println!(
            "{},{},{},{},{},source_parser_output_unqualified,true",
            csv_escape(dataset.dataset_id),
            csv_escape(&row.zone),
            row.source_line,
            row.x_over_c,
            row.cp
        );
    }
}
