use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use civil_aero_benchmark::su2_boundary_audit::{Su2BoundaryOptionSpec, audit_su2_boundary_markers};

struct Options {
    run_directory: PathBuf,
    configuration_path: String,
    mesh_path: String,
    boundary_options: Vec<Su2BoundaryOptionSpec>,
    non_boundary_options: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("su2_boundary_audit=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_arguments()?;
    let report = audit_su2_boundary_markers(
        &options.run_directory,
        &options.configuration_path,
        &options.mesh_path,
        &options.boundary_options,
        &options.non_boundary_options,
    )?;
    eprintln!(
        "su2_boundary_audit=pass run_id={} mesh_markers={} boundary_assignments={} boundary_options={} non_boundary_options={} audit_sha256={} evidence_level={}",
        report.run_id,
        report.mesh_markers.len(),
        report.assignments.len(),
        report.boundary_option_specs.len(),
        report.non_boundary_marker_options.len(),
        report.audit_sha256,
        report.evidence_level
    );
    print!("{}", report.to_csv());
    Ok(())
}

fn parse_arguments() -> Result<Options, String> {
    let mut run_directory = None;
    let mut configuration_path = None;
    let mut mesh_path = None;
    let mut boundary_options = Vec::new();
    let mut non_boundary_options = Vec::new();

    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value after {argument}"))?;
        match argument.as_str() {
            "--run-dir" => run_directory = Some(PathBuf::from(value)),
            "--config" => configuration_path = Some(value),
            "--mesh" => mesh_path = Some(value),
            "--boundary-option" => {
                let (name, schema) = value.split_once('=').ok_or_else(|| {
                    "--boundary-option must be OPTION=RECORD_WIDTH:MARKER_POSITIONS".to_owned()
                })?;
                let (width, positions) = schema.split_once(':').ok_or_else(|| {
                    "--boundary-option schema must be RECORD_WIDTH:MARKER_POSITIONS".to_owned()
                })?;
                let record_width = width
                    .parse::<usize>()
                    .map_err(|error| format!("invalid boundary record width: {error}"))?;
                let marker_positions = positions
                    .split(',')
                    .map(|position| {
                        position
                            .parse::<usize>()
                            .map_err(|error| format!("invalid marker field position: {error}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                boundary_options.push(Su2BoundaryOptionSpec {
                    option_name: name.to_owned(),
                    record_width,
                    marker_positions,
                });
            }
            "--non-boundary-option" => non_boundary_options.push(value),
            _ => return Err(format!("unknown option {argument}")),
        }
    }

    Ok(Options {
        run_directory: run_directory.ok_or_else(|| "missing --run-dir".to_owned())?,
        configuration_path: configuration_path.ok_or_else(|| "missing --config".to_owned())?,
        mesh_path: mesh_path.ok_or_else(|| "missing --mesh".to_owned())?,
        boundary_options,
        non_boundary_options,
    })
}
