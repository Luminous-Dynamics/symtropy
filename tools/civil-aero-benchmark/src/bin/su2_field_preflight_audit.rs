use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use civil_aero_benchmark::su2_field_preflight::audit_su2_field_preflight;

struct Options {
    run_directory: PathBuf,
    configuration_path: String,
    mesh_path: String,
    required_history_fields: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("su2_field_preflight=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_arguments()?;
    let report = audit_su2_field_preflight(
        &options.run_directory,
        &options.configuration_path,
        &options.mesh_path,
        &options.required_history_fields,
    )?;
    eprintln!(
        "su2_field_preflight=pass run_id={} solver_family={} conv_field={} steady_fields={} time_domain={} window_cauchy={} window_fields={} available_fields={} required_fields={} preflight_sha256={} evidence_level={}",
        report.run_id,
        report.solver_family,
        report.configured_convergence_field,
        report.convergence_protocol.steady_fields.join(";"),
        if report.convergence_protocol.time_domain { "YES".to_owned() } else { "NO".to_owned() },
        match &report.convergence_protocol.window {
            civil_aero_benchmark::su2_field_preflight::Su2WindowConvergenceProtocol::Disabled => "NO".to_owned(),
            civil_aero_benchmark::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled { .. } => "YES".to_owned(),
        },
        match &report.convergence_protocol.window {
            civil_aero_benchmark::su2_field_preflight::Su2WindowConvergenceProtocol::Disabled => String::new(),
            civil_aero_benchmark::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled { fields, .. } => fields.join(";"),
        },
        report.available_history_fields.len(),
        report.required_history_fields.len(),
        report.preflight_sha256,
        report.evidence_level
    );
    print!("{}", report.to_csv());
    Ok(())
}

fn parse_arguments() -> Result<Options, String> {
    let mut run_directory = None;
    let mut configuration_path = None;
    let mut mesh_path = None;
    let mut required_history_fields = Vec::new();

    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value after {argument}"))?;
        match argument.as_str() {
            "--run-dir" => run_directory = Some(PathBuf::from(value)),
            "--config" => configuration_path = Some(value),
            "--mesh" => mesh_path = Some(value),
            "--require-field" => required_history_fields.push(value),
            _ => return Err(format!("unknown option {argument}")),
        }
    }

    Ok(Options {
        run_directory: run_directory.ok_or_else(|| "missing --run-dir".to_owned())?,
        configuration_path: configuration_path.ok_or_else(|| "missing --config".to_owned())?,
        mesh_path: mesh_path.ok_or_else(|| "missing --mesh".to_owned())?,
        required_history_fields,
    })
}
