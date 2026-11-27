// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable canary for the AERB blind candidate sandbox profile.
//!
//! Host mode builds the sandbox argv and launches this same statically-linked
//! binary from the isolated runtime bundle. Candidate mode performs the probes
//! from inside the namespace and writes a small report only to candidate output.

use mobility_rrvr_aerb_evaluator::blind_execution_sandbox_v0_1::{
    build_candidate_sandbox_command, SandboxLimits, SandboxPaths,
};
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn create() -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "aerb-isolation-canary-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        set_mode(&root, 0o700)?;
        Ok(Self { root })
    }

    fn dir(&self, name: &str) -> Result<PathBuf, String> {
        let path = self.root.join(name);
        fs::create_dir_all(&path).map_err(|error| error.to_string())?;
        set_mode(&path, 0o700)?;
        Ok(path)
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|error| error.to_string())?
        .permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions).map_err(|error| error.to_string())
}

fn find_executable(name: &str) -> Result<PathBuf, String> {
    let path = env::var_os("PATH").ok_or_else(|| "PATH is unavailable".to_owned())?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            let metadata = fs::metadata(&candidate).map_err(|error| error.to_string())?;
            if metadata.permissions().mode() & 0o111 != 0 {
                return candidate
                    .canonicalize()
                    .map_err(|error| error.to_string());
            }
        }
    }
    Err(format!("required executable not found in PATH: {name}"))
}

fn current_pid_namespace() -> Result<String, String> {
    fs::read_link("/proc/self/ns/pid")
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}

fn limit_soft_is_bounded(limits: &str, label: &str, bound: u64) -> bool {
    limits.lines().find(|line| line.starts_with(label)).is_some_and(|line| {
        let values: Vec<&str> = line[label.len()..].split_whitespace().collect();
        values
            .first()
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|soft| soft <= bound)
            && values
                .get(1)
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|hard| hard <= bound)
    })
}

fn candidate_mode(args: &[String]) -> Result<(), String> {
    let network_port = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--network-port="))
        .ok_or_else(|| "missing network port argument".to_owned())?
        .parse::<u16>()
        .map_err(|error| error.to_string())?;
    let host_pid_namespace = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--host-pid-ns="))
        .ok_or_else(|| "missing host pid namespace argument".to_owned())?;

    let visible = fs::read_to_string("/candidate/input/candidate-visible.txt")
        .map_err(|error| error.to_string())?
        == "VISIBLE_EVIDENCE";
    let input_write_blocked = File::create("/candidate/input/forbidden-write")
        .and_then(|mut file| file.write_all(b"forbidden"))
        .is_err()
        && !Path::new("/candidate/input/forbidden-write").exists();
    let runtime_write_blocked = File::create("/runtime/forbidden-write")
        .and_then(|mut file| file.write_all(b"forbidden"))
        .is_err()
        && !Path::new("/runtime/forbidden-write").exists();

    let output_write_succeeded = File::create("/candidate/output/canary-report.txt")
        .and_then(|mut file| {
            file.write_all(b"candidate-output-writable")
                .and_then(|()| file.sync_all())
        })
        .is_ok();

    let evaluator_mount_absent = !Path::new("/evaluator/evaluator-canary.txt").exists()
        && !Path::new("/evaluator").exists()
        && !Path::new("/host-secret.txt").exists();
    let evaluator_environment_absent = env::var_os("AERB_EVALUATOR_SECRET").is_none();
    let process_one_environment =
        fs::read("/proc/1/environ").map_err(|error| error.to_string())?;
    let evaluator_secret_absent_from_process_table =
        !process_one_environment
            .windows(b"AERB_EVALUATOR_SECRET".len())
            .any(|window| window == b"AERB_EVALUATOR_SECRET")
            && !process_one_environment
                .windows(b"CI_SENTINEL_SHOULD_NOT_ENTER_SANDBOX".len())
                .any(|window| window == b"CI_SENTINEL_SHOULD_NOT_ENTER_SANDBOX");
    let expected_env = [
        ("HOME", "/tmp"),
        ("PWD", "/candidate/input"),
        ("TMPDIR", "/tmp"),
        ("PATH", "/runtime/bin"),
        ("LANG", "C"),
        ("TZ", "UTC"),
    ];
    let environment_matches = expected_env
        .iter()
        .all(|(key, value)| env::var(key).is_ok_and(|actual| actual == *value));

    let own_pid_namespace = current_pid_namespace()?;
    let pid_namespace_isolated = own_pid_namespace != host_pid_namespace;

    // Bubblewrap's --disable-userns configures user.max_user_namespaces
    // in the sandbox. --assert-userns-disabled makes setup fail if this
    // protection cannot be established; the canary verifies the visible limit.
    let user_namespace_limit = fs::read_to_string("/proc/sys/user/max_user_namespaces")
        .map_err(|error| error.to_string())?
        .trim()
        .parse::<u64>()
        .map_err(|error| error.to_string())?;
    let nested_user_namespaces_disabled = user_namespace_limit == 1;

    let process_status = fs::read_to_string("/proc/self/status")
        .map_err(|error| error.to_string())?;
    let capabilities_dropped = ["CapEff:", "CapPrm:", "CapBnd:"].iter().all(|label| {
        process_status
            .lines()
            .find(|line| line.starts_with(label))
            .is_some_and(|line| {
                line.split_whitespace()
                    .nth(1)
                    .is_some_and(|value| value.bytes().all(|byte| byte == b'0'))
            })
    });

    let limits = fs::read_to_string("/proc/self/limits").map_err(|error| error.to_string())?;
    let resource_limits_active = limit_soft_is_bounded(&limits, "Max address space", 1_073_741_824)
        && limit_soft_is_bounded(&limits, "Max cpu time", 15)
        && limit_soft_is_bounded(&limits, "Max processes", 32)
        && limit_soft_is_bounded(&limits, "Max open files", 64)
        && limit_soft_is_bounded(&limits, "Max file size", 4_194_304);

    let network_blocked = TcpStream::connect_timeout(
        &format!("127.0.0.1:{network_port}")
            .parse()
            .map_err(|error: std::net::AddrParseError| error.to_string())?,
        Duration::from_millis(400),
    )
    .is_err();

    let report = format!(
        "candidate_input_read={visible}\ninput_write_blocked={input_write_blocked}\nruntime_write_blocked={runtime_write_blocked}\noutput_write_succeeded={output_write_succeeded}\nevaluator_mount_absent={evaluator_mount_absent}\nevaluator_environment_absent={evaluator_environment_absent}\nenvironment_matches={environment_matches}\npid_namespace_isolated={pid_namespace_isolated}\nnested_user_namespaces_disabled={nested_user_namespaces_disabled}\nresource_limits_active={resource_limits_active}\nnetwork_blocked={network_blocked}\n"
    );
    fs::write("/candidate/output/canary-report.txt", report.as_bytes())
        .map_err(|error| error.to_string())?;

    let checks = [
        ("candidate input read", visible),
        ("candidate input read-only", input_write_blocked),
        ("runtime read-only", runtime_write_blocked),
        ("candidate output writable", output_write_succeeded),
        ("evaluator mount absent", evaluator_mount_absent),
        ("evaluator environment absent", evaluator_environment_absent),\n        ("evaluator secret absent from process table", evaluator_secret_absent_from_process_table),
        ("environment sanitized", environment_matches),
        ("PID namespace isolated", pid_namespace_isolated),
        ("nested user namespaces disabled", nested_user_namespaces_disabled),\n        ("capabilities dropped", capabilities_dropped),
        ("resource limits active", resource_limits_active),
        ("host loopback unreachable", network_blocked),
    ];
    let failed: Vec<&str> = checks
        .iter()
        .filter_map(|(name, passed)| (!passed).then_some(*name))
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("sandbox canary failures: {}", failed.join(", ")))
    }
}

fn host_mode() -> Result<(), String> {
    let tree = TempTree::create()?;
    let candidate_input = tree.dir("candidate-input")?;
    let runtime = tree.dir("runtime")?;
    let runtime_bin = runtime.join("bin");
    fs::create_dir_all(&runtime_bin).map_err(|error| error.to_string())?;
    set_mode(&runtime_bin, 0o700)?;
    let output = tree.dir("candidate-output")?;
    let evaluator_secret = tree.dir("evaluator-secret")?;

    fs::write(
        candidate_input.join("candidate-visible.txt"),
        b"VISIBLE_EVIDENCE",
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        evaluator_secret.join("evaluator-canary.txt"),
        b"NEVER_VISIBLE_TO_CANDIDATE",
    )
    .map_err(|error| error.to_string())?;
    let host_secret = tree.root.join("host-secret.txt");
    fs::write(&host_secret, b"NEVER_VISIBLE_TO_CANDIDATE")
        .map_err(|error| error.to_string())?;

    let current_executable = env::current_exe().map_err(|error| error.to_string())?;
    let candidate_executable = runtime_bin.join("aerb_sandbox_canary");
    fs::copy(&current_executable, &candidate_executable)
        .map_err(|error| error.to_string())?;
    set_mode(&candidate_executable, 0o755)?;

    let bubblewrap = find_executable("bwrap")?;
    let prlimit = find_executable("prlimit")?;
    let host_pid_namespace = current_pid_namespace()?;

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|error| error.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let network_port = listener.local_addr().map_err(|error| error.to_string())?.port();
    let network_accepted = Arc::new(AtomicBool::new(false));
    let network_accepted_thread = Arc::clone(&network_accepted);
    let server = thread::spawn(move || {
        for _ in 0..150 {
            match listener.accept() {
                Ok((_stream, _address)) => {
                    network_accepted_thread.store(true, Ordering::SeqCst);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });

    let paths = SandboxPaths {
        bubblewrap,
        prlimit,
        candidate_input_root: candidate_input,
        runtime_bundle_root: runtime,
        candidate_output_root: output.clone(),
        evaluator_secret_root: evaluator_secret,
        candidate_executable,
    };
    let limits = SandboxLimits {
        address_space_bytes: 1_073_741_824,
        cpu_seconds: 15,
        max_processes: 32,
        max_open_files: 64,
        max_file_bytes: 4_194_304,
        tmpfs_bytes: 16_777_216,
    };
    let candidate_arguments = vec![
        "--candidate".to_owned(),
        format!("--network-port={network_port}"),
        format!("--host-pid-ns={host_pid_namespace}"),
    ];
    let spec = build_candidate_sandbox_command(
        &paths,
        limits,
        &candidate_arguments.iter().map(Into::into).collect::<Vec<_>>(),
    )
    .map_err(|error| error.to_string())?;

    let status = spec
        .process_command()
        .status()
        .map_err(|error| format!("sandbox launch failed: {error}"))?;

    server
        .join()
        .map_err(|_| "network canary server thread panicked".to_owned())?;

    if !status.success() {
        return Err(format!("candidate sandbox exited with {status}"));
    }
    if network_accepted.load(Ordering::SeqCst) {
        return Err("candidate reached host loopback listener".to_owned());
    }

    let report = fs::read_to_string(output.join("canary-report.txt"))
        .map_err(|error| format!("candidate canary report missing: {error}"))?;
    for key in [
        "candidate_input_read=true",
        "input_write_blocked=true",
        "runtime_write_blocked=true",
        "output_write_succeeded=true",
        "evaluator_mount_absent=true",
        "evaluator_environment_absent=true",\n        "evaluator_secret_absent_from_process_table=true",
        "environment_matches=true",
        "pid_namespace_isolated=true",
        "nested_user_namespaces_disabled=true",\n        "capabilities_dropped=true",
        "resource_limits_active=true",
        "network_blocked=true",
    ] {
        if !report.lines().any(|line| line == key) {
            return Err(format!("canary report missing positive result {key}: {report}"));
        }
    }
    println!("AERB_BLIND_SANDBOX_CANARY=PASS");
    println!("{report}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = if args.first().is_some_and(|arg| arg == "--candidate") {
        candidate_mode(&args[1..])
    } else {
        host_mode()
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_BLIND_SANDBOX_CANARY=FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}
