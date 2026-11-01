use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use civil_aero_benchmark::su2_convergence_screen::{
    ConvergenceCriteria, MonitorCriterion, ResidualAcceptance, ResidualCriterion,
    screen_su2_history, screen_to_csv,
};
use civil_aero_benchmark::su2_history::{Su2HistoryField, Su2HistorySchema, import_su2_history};
use civil_aero_benchmark::su2_run_binding::{
    Su2RunInputBindings, bind_su2_run_lineage, binding_to_csv,
};

struct Options {
    run_directory: PathBuf,
    history_relative_path: String,
    expected_header: Vec<String>,
    iteration_column: String,
    numeric_fields: Vec<Su2HistoryField>,
    residuals: Vec<ResidualCriterion>,
    monitors: Vec<MonitorCriterion>,
    physics_configuration_path: Option<String>,
    geometry_path: Option<String>,
    mesh_path: Option<String>,
    additional_output_paths: Vec<String>,
    lineage_output_path: Option<PathBuf>,
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
    let run_directory = options.run_directory.clone();
    let schema = Su2HistorySchema {
        history_relative_path: options.history_relative_path,
        expected_header: options.expected_header,
        iteration_column: options.iteration_column,
        numeric_fields: options.numeric_fields,
        minimum_rows: 2,
    };
    let history = import_su2_history(&run_directory, &schema)?;
    let audit = screen_su2_history(
        &history,
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

    let lineage_requested = options.physics_configuration_path.is_some()
        || options.geometry_path.is_some()
        || options.mesh_path.is_some()
        || !options.additional_output_paths.is_empty()
        || options.lineage_output_path.is_some();
    if lineage_requested {
        let input_bindings = Su2RunInputBindings {
            physics_configuration_path: options
                .physics_configuration_path
                .ok_or_else(|| "lineage export requires --config-input".to_owned())?,
            geometry_path: options
                .geometry_path
                .ok_or_else(|| "lineage export requires --geometry-input".to_owned())?,
            mesh_path: options
                .mesh_path
                .ok_or_else(|| "lineage export requires --mesh-input".to_owned())?,
        };
        let lineage_output_path = options
            .lineage_output_path
            .ok_or_else(|| "lineage export requires --lineage-output".to_owned())?;
        let binding = bind_su2_run_lineage(
            &run_directory,
            &history,
            &audit,
            &input_bindings,
            &options.additional_output_paths,
        )?;
        fs::write(&lineage_output_path, binding_to_csv(&binding)).map_err(|error| {
            format!(
                "cannot write lineage receipt {}: {error}",
                lineage_output_path.display()
            )
        })?;
        eprintln!(
            "su2_run_lineage=pass run_id={} outputs={} binding_sha256={} evidence_level={}",
            binding.run_id,
            binding.additional_outputs.len(),
            binding.binding_sha256,
            binding.evidence_level
        );
    }
    print!("{}", screen_to_csv(&audit));
    Ok(audit.status == "declared_thresholds_met")
}

fn split_spec<'a>(
    value: &'a str,
    name: &str,
    expected_parts: usize,
) -> Result<Vec<&'a str>, String> {
    let parts = value.split('|').collect::<Vec<_>>();
    if parts.len() != expected_parts || parts.iter().any(|part| part.trim().is_empty()) {
        return Err(format!(
            "{name} requires exactly {expected_parts} non-empty pipe-separated fields"
        ));
    }
    Ok(parts)
}

fn parse_f64(value: &str, name: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {name}: {error}"))
}

fn parse_arguments() -> Result<Options, String> {
    let mut run_directory = None;
    let mut history_relative_path = None;
    let mut expected_header = Vec::new();
    let mut iteration_column = None;
    let mut numeric_fields = Vec::new();
    let mut residuals = Vec::new();
    let mut monitors = Vec::new();
    let mut physics_configuration_path = None;
    let mut geometry_path = None;
    let mut mesh_path = None;
    let mut additional_output_paths = Vec::new();
    let mut lineage_output_path = None;

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
            "--config-input" => physics_configuration_path = Some(value),
            "--geometry-input" => geometry_path = Some(value),
            "--mesh-input" => mesh_path = Some(value),
            "--additional-output" => additional_output_paths.push(value),
            "--lineage-output" => lineage_output_path = Some(PathBuf::from(value)),
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
            "--residual-log10" => {
                let parts = split_spec(&value, "--residual-log10", 6)?;
                residuals.push(ResidualCriterion {
                    column: parts[0].to_owned(),
                    unit: parts[1].to_owned(),
                    terminal_window_rows: parts[2]
                        .parse()
                        .map_err(|error| format!("invalid log10 residual window: {error}"))?,
                    acceptance: ResidualAcceptance::Log10 {
                        maximum_final_log10_residual: parse_f64(
                            parts[3],
                            "maximum final log10 residual",
                        )?,
                        minimum_log10_reduction: parse_f64(parts[4], "minimum log10 reduction")?,
                        maximum_terminal_span_log10: parse_f64(
                            parts[5],
                            "maximum terminal log10 span",
                        )?,
                    },
                });
            }
            "--residual-linear" => {
                let parts = split_spec(&value, "--residual-linear", 6)?;
                residuals.push(ResidualCriterion {
                    column: parts[0].to_owned(),
                    unit: parts[1].to_owned(),
                    terminal_window_rows: parts[2]
                        .parse()
                        .map_err(|error| format!("invalid linear residual window: {error}"))?,
                    acceptance: ResidualAcceptance::PositiveLinear {
                        maximum_final_value: parse_f64(parts[3], "maximum final residual value")?,
                        maximum_final_to_initial_ratio: parse_f64(
                            parts[4],
                            "maximum residual ratio",
                        )?,
                        maximum_terminal_range: parse_f64(
                            parts[5],
                            "maximum terminal residual range",
                        )?,
                    },
                });
            }
            "--monitor" => {
                let parts = split_spec(&value, "--monitor", 6)?;
                monitors.push(MonitorCriterion {
                    column: parts[0].to_owned(),
                    unit: parts[1].to_owned(),
                    trailing_window_rows: parts[2]
                        .parse()
                        .map_err(|error| format!("invalid monitor window: {error}"))?,
                    maximum_absolute_range: parse_f64(parts[3], "maximum monitor range")?,
                    maximum_absolute_mean_drift: parse_f64(parts[4], "maximum monitor mean drift")?,
                    maximum_absolute_slope_per_1000_iterations: parse_f64(
                        parts[5],
                        "maximum monitor slope per 1000 iterations",
                    )?,
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
        physics_configuration_path,
        geometry_path,
        mesh_path,
        additional_output_paths,
        lineage_output_path,
    })
}
