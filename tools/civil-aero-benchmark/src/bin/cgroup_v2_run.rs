use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use civil_aero_benchmark::cgroup_v2_supervisor::{
    CgroupV2RunRequest, run_cgroup_v2_process_tree,
};

const ENVIRONMENT_ALLOWLIST: &[&str] = &[
    "PATH", "HOME", "LANG", "LC_ALL", "TMPDIR", "LD_LIBRARY_PATH",
    "OMP_NUM_THREADS", "MKL_NUM_THREADS", "OPENBLAS_NUM_THREADS",
    "NUMEXPR_NUM_THREADS", "PYTHONHASHSEED", "PYTHONNOUSERSITE",
    "CUDA_VISIBLE_DEVICES", "CUBLAS_WORKSPACE_CONFIG",
];

struct Options {
    delegated_root: PathBuf,
    result_directory: PathBuf,
    run_id: String,
    timeout: Duration,
    current_directory: PathBuf,
    environment: BTreeMap<String, String>,
    program: PathBuf,
    arguments: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(3),
        Err(error) => {
            eprintln!("cgroup_v2_run=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let options = parse_arguments()?;
    let request = CgroupV2RunRequest {
        delegated_root: options.delegated_root,
        result_directory: options.result_directory,
        run_id: options.run_id,
        program: options.program,
        arguments: options.arguments,
        current_directory: options.current_directory,
        environment: options.environment,
        timeout: options.timeout,
    };
    let receipt = run_cgroup_v2_process_tree(&request)?;
    eprintln!(
        "cgroup_v2_run={} run_id={} state={} exit_code={} signal={} timed_out={} wall_ns={} cpu_usage_usec={} peak_memory_bytes={} oom_delta={} oom_kill_delta={} cleanup={} receipt_sha256={} evidence_level={}",
        if receipt.state == "succeeded" && receipt.cleanup_state == "removed" {
            "pass"
        } else {
            "recorded_non_success"
        },
        receipt.run_id,
        receipt.state,
        receipt.exit_code.map(|value| value.to_string()).unwrap_or_else(|| "-".to_owned()),
        receipt.signal.map(|value| value.to_string()).unwrap_or_else(|| "-".to_owned()),
        receipt.timed_out,
        receipt.elapsed_wall_time_ns,
        receipt.cpu_usage_usec,
        receipt.peak_memory_bytes,
        receipt.oom_event_delta,
        receipt.oom_kill_event_delta,
        receipt.cleanup_state,
        receipt.receipt_sha256,
        receipt.evidence_level
    );
    Ok(receipt.state == "succeeded" && receipt.cleanup_state == "removed")
}

fn parse_arguments() -> Result<Options, String> {
    let mut delegated_root = None;
    let mut result_directory = None;
    let mut run_id = None;
    let mut timeout_seconds = 3_600_u64;
    let mut current_directory = env::current_dir()
        .map_err(|error| format!("cannot resolve current directory: {error}"))?;
    let mut environment = BTreeMap::new();
    for key in ENVIRONMENT_ALLOWLIST {
        if let Some(value) = env::var_os(key) {
            let value = value
                .into_string()
                .map_err(|_| format!("environment variable {key} is not valid UTF-8"))?;
            environment.insert((*key).to_owned(), value);
        }
    }
    let mut program = None;
    let mut child_arguments = Vec::new();

    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--" {
            program = args.next().map(PathBuf::from);
            child_arguments = args.collect();
            break;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value after {argument}"))?;
        match argument.as_str() {
            "--delegate-root" => delegated_root = Some(PathBuf::from(value)),
            "--result-dir" => result_directory = Some(PathBuf::from(value)),
            "--run-id" => run_id = Some(value),
            "--timeout-seconds" => {
                timeout_seconds = value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid --timeout-seconds: {error}"))?;
            }
            "--cwd" => current_directory = PathBuf::from(value),
            "--env" => {
                let (key, env_value) = value
                    .split_once('=')
                    .ok_or_else(|| "--env must be KEY=VALUE".to_owned())?;
                if key.is_empty() || environment.insert(key.to_owned(), env_value.to_owned()).is_some() {
                    return Err("--env key must be non-empty and must not be duplicated".to_owned());
                }
            }
            _ => return Err(format!("unknown option {argument}")),
        }
    }
    let program = program.ok_or_else(|| "target command is required after --".to_owned())?;
    if program.as_os_str().is_empty() || timeout_seconds == 0 {
        return Err("target program and positive timeout are required".to_owned());
    }
    Ok(Options {
        delegated_root: delegated_root.ok_or_else(|| "missing --delegate-root".to_owned())?,
        result_directory: result_directory.ok_or_else(|| "missing --result-dir".to_owned())?,
        run_id: run_id.ok_or_else(|| "missing --run-id".to_owned())?,
        timeout: Duration::from_secs(timeout_seconds),
        current_directory,
        environment,
        program,
        arguments: child_arguments,
    })
}
