use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use civil_aero_benchmark::su2_history::{Su2HistoryField, Su2HistorySchema, import_su2_history};

struct Options {
    run_directory: PathBuf,
    history_relative_path: String,
    expected_header: Vec<String>,
    iteration_column: String,
    numeric_fields: Vec<Su2HistoryField>,
    minimum_rows: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("su2_history_import=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_arguments()?;
    let schema = Su2HistorySchema {
        history_relative_path: options.history_relative_path,
        expected_header: options.expected_header,
        iteration_column: options.iteration_column,
        numeric_fields: options.numeric_fields,
        minimum_rows: options.minimum_rows,
    };
    let report = import_su2_history(&options.run_directory, &schema)?;
    eprintln!(
        "su2_history_import=pass run_id={} points={} selected_fields={} history_sha256={} runner_receipt_sha256={} import_sha256={} evidence_level={}",
        report.run_id,
        report.points.len(),
        report.numeric_fields.len(),
        report.history_sha256,
        report.runner_receipt_sha256,
        report.import_sha256,
        report.evidence_level
    );
    print!("{}", report.to_csv());
    Ok(())
}

fn parse_arguments() -> Result<Options, String> {
    let mut run_directory = None;
    let mut history_relative_path = None;
    let mut expected_header = Vec::new();
    let mut iteration_column = None;
    let mut numeric_fields = Vec::new();
    let mut minimum_rows = 2_usize;

    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value after {argument}"))?;
        match argument.as_str() {
            "--run-dir" => run_directory = Some(PathBuf::from(value)),
            "--history" => history_relative_path = Some(value),
            "--header-column" => expected_header.push(value),
            "--iteration-column" => iteration_column = Some(value),
            "--field" => {
                let (column, unit) = value
                    .split_once('=')
                    .ok_or_else(|| "--field must be COLUMN=UNIT".to_owned())?;
                if column.is_empty() || unit.is_empty() {
                    return Err("--field column and unit must be non-empty".to_owned());
                }
                numeric_fields.push(Su2HistoryField {
                    column: column.to_owned(),
                    unit: unit.to_owned(),
                });
            }
            "--minimum-rows" => {
                minimum_rows = value
                    .parse()
                    .map_err(|error| format!("invalid --minimum-rows: {error}"))?;
            }
            _ => return Err(format!("unknown option {argument}")),
        }
    }

    Ok(Options {
        run_directory: run_directory.ok_or_else(|| "missing --run-dir".to_owned())?,
        history_relative_path: history_relative_path
            .ok_or_else(|| "missing --history".to_owned())?,
        expected_header,
        iteration_column: iteration_column
            .ok_or_else(|| "missing --iteration-column".to_owned())?,
        numeric_fields,
        minimum_rows,
    })
}
