use std::{env, fs, process};

use civil_aero_benchmark::dpw8_submission::{audit_v5, parse_v5, V5_CSV_HEADER};

const DEFAULT_FIXTURE: &str = "data/dpw8/synthetic_two_view_fixture_v0.1.dat";

fn main() {
    if let Err(error) = run() {
        eprintln!("DPW-8 V5 audit rejected input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() > 1 {
        return Err("usage: dpw8_v5_audit [submission-v5.dat]".to_owned());
    }
    let path = arguments
        .first()
        .map(String::as_str)
        .unwrap_or(DEFAULT_FIXTURE);
    let input = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let dataset = parse_v5(&input)?;
    let report = audit_v5(&dataset)?;
    let source_evidence = if path == DEFAULT_FIXTURE {
        "synthetic_fixture_only"
    } else {
        "caller_supplied_unverified_not_officially_authenticated"
    };

    eprintln!(
        "source_evidence={source_evidence} title={} zones={} raw_records={} grid_view_records={} alpha_view_records={} canonical_cells={}",
        dataset.title,
        dataset.zones.len(),
        report.raw_record_count,
        report.grid_view_record_count,
        report.alpha_view_record_count,
        report.canonical_cells.len()
    );
    for cell in report
        .canonical_cells
        .iter()
        .filter(|cell| cell.anomaly_warning)
    {
        eprintln!(
            "warning=unresolved_result_pattern grid_level={} alpha_deg={:.8} grid_view_line={} alpha_view_line={} preserved=true",
            cell.row.grid_level,
            cell.row.alpha_deg,
            cell.grid_view_source_line,
            cell.alpha_view_source_line
        );
    }

    println!("{V5_CSV_HEADER}");
    for cell in &report.canonical_cells {
        println!(
            "{},{},{:.8},{:.8},{:.8},{:.8},{},{},{},{},{},{}",
            cell.row.grid_level,
            cell.row.grid_size,
            cell.row.alpha_deg,
            cell.row.cl_total(),
            cell.row.cd_total(),
            cell.row.cm_total(),
            cell.grid_view_source_line,
            cell.alpha_view_source_line,
            cell.row.missing_optional_count(),
            cell.duplicate_cl_cd_at_other_alpha,
            cell.adjacent_grid_discontinuity,
            cell.anomaly_warning
        );
    }
    Ok(())
}
