use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use civil_aero_benchmark::external_solver::{
    ExternalSolverRequest, StagedInputSpec, execute_external_solver, read_external_solver_receipt,
};
use civil_aero_benchmark::integrity::sha256_file;

struct Options {
    run_id: String,
    solver_id: String,
    solver_version_claim: String,
    executable_path: PathBuf,
    expected_executable_sha256: String,
    run_directory: PathBuf,
    inputs: Vec<(PathBuf, String)>,
    outputs: Vec<String>,
    environment: BTreeMap<String, String>,
    timeout_seconds: u64,
    max_log_bytes_per_stream: u64,
    solver_arguments: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(3),
        Err(error) => {
            eprintln!("external_solver_run=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let options = parse_arguments()?;
    if options.inputs.is_empty() {
        return Err("at least one --input SOURCE=RUN_RELATIVE_PATH is required".to_owned());
    }
    if options.solver_id == "SU2_CFD_DRY_RUN" {
        if !options.outputs.is_empty() {
            return Err(
                "SU2_CFD_DRY_RUN must use a process-only receipt with no expected outputs"
                    .to_owned(),
            );
        }
        if options.solver_arguments.len() != 2 || options.solver_arguments[0] != "-d" {
            return Err("SU2_CFD_DRY_RUN requires exactly: -- -d CONFIG_PATH".to_owned());
        }
    } else if options.outputs.is_empty() {
        return Err("only SU2_CFD_DRY_RUN may use a process-only receipt".to_owned());
    }
    if options.solver_arguments.is_empty() {
        return Err(
            "solver arguments are required after -- (for SU2_CFD, use -f config.cfg)".to_owned(),
        );
    }

    let expected_executable_sha256 = options.expected_executable_sha256.to_ascii_lowercase();
    if expected_executable_sha256.len() != 64
        || !expected_executable_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("--expected-executable-sha256 must be 64 hexadecimal characters".to_owned());
    }
    let actual_before = sha256_file(&options.executable_path)?;
    if actual_before != expected_executable_sha256 {
        return Err(format!(
            "executable SHA-256 pin mismatch before launch: expected {}, got {}",
            expected_executable_sha256, actual_before
        ));
    }

    let mut staged_inputs = Vec::with_capacity(options.inputs.len());
    for (source_path, relative_path) in &options.inputs {
        let canonical_source = fs::canonicalize(source_path)
            .map_err(|error| format!("cannot resolve input {}: {error}", source_path.display()))?;
        let metadata = fs::metadata(&canonical_source).map_err(|error| {
            format!(
                "cannot inspect input {}: {error}",
                canonical_source.display()
            )
        })?;
        if !metadata.is_file() {
            return Err(format!(
                "input {} is not a regular file",
                canonical_source.display()
            ));
        }
        staged_inputs.push(StagedInputSpec {
            source_path: canonical_source.clone(),
            relative_path: relative_path.clone(),
            expected_byte_length: metadata.len(),
            expected_sha256: sha256_file(&canonical_source)?,
            source_reference: format!(
                "operator_supplied_local_path:{}",
                canonical_source.display()
            ),
        });
    }

    let request = ExternalSolverRequest {
        run_id: options.run_id.clone(),
        solver_id: options.solver_id.clone(),
        solver_version_claim: options.solver_version_claim.clone(),
        executable_path: options.executable_path,
        run_directory: options.run_directory.clone(),
        arguments: options.solver_arguments,
        environment: options.environment,
        staged_inputs,
        expected_outputs: options.outputs,
        timeout: Duration::from_secs(options.timeout_seconds),
        max_log_bytes_per_stream: options.max_log_bytes_per_stream,
    };

    let receipt = execute_external_solver(&request)?;
    let replayed = read_external_solver_receipt(&request.run_directory)?;
    if replayed != receipt {
        return Err("persisted receipt differs from the just-created receipt".to_owned());
    }

    let executable_pin_matches = receipt.executable_sha256 == expected_executable_sha256;
    eprintln!(
        "external_solver_run={} run_id={} solver_id={} status={} executable_pin_matches={} outputs_present={} outputs_missing={} receipt_sha256={} evidence_level={}",
        if receipt.is_process_success() && executable_pin_matches {
            "pass"
        } else {
            "recorded_failure"
        },
        receipt.run_id,
        receipt.solver_id,
        receipt.status,
        executable_pin_matches,
        receipt
            .outputs
            .iter()
            .filter(|output| output.present)
            .count(),
        receipt.missing_output_count(),
        receipt.receipt_sha256,
        receipt.evidence_level
    );
    if !executable_pin_matches {
        eprintln!(
            "external_solver_run=pin_mismatch expected={} recorded={}",
            expected_executable_sha256, receipt.executable_sha256
        );
    }
    Ok(receipt.is_process_success() && executable_pin_matches)
}

fn parse_arguments() -> Result<Options, String> {
    let mut run_id = None;
    let mut solver_id = None;
    let mut solver_version_claim = None;
    let mut executable_path = None;
    let mut expected_executable_sha256 = None;
    let mut run_directory = None;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut environment = BTreeMap::new();
    let mut timeout_seconds = 7_200_u64;
    let mut max_log_bytes_per_stream = 64 * 1024 * 1024_u64;
    let mut solver_arguments = Vec::new();

    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            solver_arguments = arguments.collect();
            break;
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value after {argument}"))?;
        match argument.as_str() {
            "--run-id" => run_id = Some(value),
            "--solver-id" => solver_id = Some(value),
            "--version-claim" => solver_version_claim = Some(value),
            "--executable" => executable_path = Some(PathBuf::from(value)),
            "--expected-executable-sha256" => expected_executable_sha256 = Some(value),
            "--run-dir" => run_directory = Some(PathBuf::from(value)),
            "--input" => {
                let (source, relative) = value
                    .split_once('=')
                    .ok_or_else(|| "--input must be SOURCE_PATH=RUN_RELATIVE_PATH".to_owned())?;
                if source.is_empty() || relative.is_empty() {
                    return Err("--input source and relative path must be non-empty".to_owned());
                }
                inputs.push((PathBuf::from(source), relative.to_owned()));
            }
            "--output" => outputs.push(value),
            "--env" => {
                let (key, entry_value) = value
                    .split_once('=')
                    .ok_or_else(|| "--env must be KEY=VALUE".to_owned())?;
                if key.is_empty()
                    || environment
                        .insert(key.to_owned(), entry_value.to_owned())
                        .is_some()
                {
                    return Err("--env keys must be non-empty and unique".to_owned());
                }
            }
            "--timeout-seconds" => {
                timeout_seconds = value
                    .parse()
                    .map_err(|error| format!("invalid --timeout-seconds: {error}"))?;
            }
            "--max-log-bytes" => {
                max_log_bytes_per_stream = value
                    .parse()
                    .map_err(|error| format!("invalid --max-log-bytes: {error}"))?;
            }
            _ => return Err(format!("unknown option {argument}")),
        }
    }

    let required = |value: Option<String>, name: &str| {
        value.ok_or_else(|| format!("missing required option {name}"))
    };
    Ok(Options {
        run_id: required(run_id, "--run-id")?,
        solver_id: required(solver_id, "--solver-id")?,
        solver_version_claim: required(solver_version_claim, "--version-claim")?,
        executable_path: executable_path
            .ok_or_else(|| "missing required option --executable".to_owned())?,
        expected_executable_sha256: required(
            expected_executable_sha256,
            "--expected-executable-sha256",
        )?,
        run_directory: run_directory
            .ok_or_else(|| "missing required option --run-dir".to_owned())?,
        inputs,
        outputs,
        environment,
        timeout_seconds,
        max_log_bytes_per_stream,
        solver_arguments,
    })
}
