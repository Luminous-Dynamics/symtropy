use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use civil_aero_benchmark::su2_convergence_screen::{
    ConvergenceCriteria, MonitorCriterion, ResidualCriterion, screen_su2_history, screen_to_csv,
};
use civil_aero_benchmark::su2_history::{Su2HistoryField, Su2HistorySchema, import_su2_history};

struct Options {
    run_directory: PathBuf,
    history_relative_path: String,
    expected_header: Vec<String>,
    iteration_column: String,
    numeric_fields: Vec<Su2HistoryField>,
    residuals: Vec<ResidualCriterion>,
    monitors: Vec<MonitorCriterion>,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(3),
        Err(error) => {
            eprintln!("su2_convergence_screen=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let options = parse_arguments()?;
    let schema = Su2HistorySchema {
        history_relative_path: options.history_relative_path,
        expected_header: options.expected_header,
        iteration_column: options.iteration_column,
        numeric_fields: options.numeric_fields,
        minimum_rows: 2,
    };
    let report = import_su2_history(&options.run_directory, &schema)?;
    let audit = screen_su2_history(
        &report,
        &ConvergenceCriteria {
            residuals: options.residuals,
            monitors: options.monitors,
        },
    )?;
    eprintln!(
        "su2_convergence_screen={} run_id={} passed={} failed={} insufficient={} screen_sha256={} evidence_level={}",
        if audit.status == "declared_thresholds_met" {
            "pass"
        } else {
            "not_admitted"
        },
        audit.run_id,
        audit.passed_count,
        audit.failed_count,
        audit.insufficient_count,
        audit.screen_sha256,
        audit.evidence_level
    );
    print!("{}", screen_to_csv(&audit));
    Ok(audit.status == "declared_thresholds_met")
}

fn parse_arguments() -> Result<Options, String> {
    let mut run_directory = None;
    let mut history_relative_path = None;
    let mut expected_header = Vec::new();
    let mut iteration_column = None;
    let mut numeric_fields = Vec::new();
    let mut residuals = Vec::new();
    let mut monitors = Vec::new();

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
            "--residual" => {
                let (column, limits) = value.split_once('=').ok_or_else(|| {
                    "--residual must be COLUMN=MAX_FINAL_ABS:MAX_FINAL_RATIO".to_owned()
                })?;
                let (maximum_final_absolute_value, maximum_final_to_initial_ratio) = limits
                    .split_once(':')
                    .ok_or_else(|| "--residual needs MAX_FINAL_ABS:MAX_FINAL_RATIO".to_owned())?;
                residuals.push(ResidualCriterion {
                    column: column.to_owned(),
                    maximum_final_absolute_value: maximum_final_absolute_value
                        .parse()
                        .map_err(|error| format!("invalid residual absolute limit: {error}"))?,
                    maximum_final_to_initial_ratio: maximum_final_to_initial_ratio
                        .parse()
                        .map_err(|error| format!("invalid residual reduction ratio: {error}"))?,
                });
            }
            "--monitor" => {
                let (column, limits) = value.split_once('=').ok_or_else(|| {
                    "--monitor must be COLUMN=WINDOW_ROWS:MAX_ABS_RANGE".to_owned()
                })?;
                let (trailing_window_rows, maximum_absolute_range) = limits
                    .split_once(':')
                    .ok_or_else(|| "--monitor needs WINDOW_ROWS:MAX_ABS_RANGE".to_owned())?;
                monitors.push(MonitorCriterion {
                    column: column.to_owned(),
                    trailing_window_rows: trailing_window_rows
                        .parse()
                        .map_err(|error| format!("invalid monitor window: {error}"))?,
                    maximum_absolute_range: maximum_absolute_range
                        .parse()
                        .map_err(|error| format!("invalid monitor range: {error}"))?,
                });
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
        residuals,
        monitors,
    })
}
