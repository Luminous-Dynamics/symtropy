//! Linux cgroup v2 process-tree supervision for operation-scoped CPU and memory evidence.
//!
//! This module requires an operator-provided delegated cgroup subtree with cpu and memory
//! controllers already enabled. It never changes cgroup.subtree_control or writes into a
//! manager-owned ancestor. The target is held behind a pre-exec gate until its PID has been
//! placed in a fresh child cgroup; descendants forked after exec inherit that membership.
//!
//! Measurement evidence ends only when cgroup.events reports populated=0 for the whole
//! hierarchy. GPU time and energy are not inferred from CPU/memory counters.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::integrity::sha256_hex;
use crate::resource_telemetry::{
    CgroupV2ResourceReport, CgroupV2Snapshot, CgroupV2SnapshotInput,
    cgroup_v2_resource_evidence_to_csv, derive_cgroup_v2_resource_report,
};

pub const CGROUP_V2_RUN_SCHEMA: &str = "symtropy-cgroup-v2-run/0.1.0";
pub const CGROUP_V2_RUN_EVIDENCE_LEVEL: &str =
    "delegated_cgroup_v2_process_tree_cpu_wall_peak_memory_measured_gpu_energy_unknown_physics_unverified";

const BASE_ENVIRONMENT_KEYS: &[&str] = &[
    "PATH", "HOME", "LANG", "LC_ALL", "TMPDIR", "LD_LIBRARY_PATH",
    "OMP_NUM_THREADS", "MKL_NUM_THREADS", "OPENBLAS_NUM_THREADS",
    "NUMEXPR_NUM_THREADS", "PYTHONHASHSEED", "PYTHONNOUSERSITE",
    "CUDA_VISIBLE_DEVICES", "CUBLAS_WORKSPACE_CONFIG",
];
const CGROUP_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CGROUP_KILL_GRACE: Duration = Duration::from_secs(10);
const F_GETFL: i32 = 3;
const O_ACCMODE: i32 = 0b11;
const O_RDONLY: i32 = 0;

unsafe extern "C" {
    fn read(fd: i32, buffer: *mut u8, count: usize) -> isize;
    fn close(fd: i32) -> i32;
    fn fcntl(fd: i32, command: i32, ...) -> i32;
}

#[derive(Debug, Clone)]
pub struct CgroupV2RunRequest {
    pub delegated_root: PathBuf,
    /// Must not exist. Result files are retained here, outside the cgroup subtree.
    pub result_directory: PathBuf,
    pub run_id: String,
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub current_directory: PathBuf,
    /// Cleared before launch; only reviewed, explicit environment keys are passed.
    pub environment: BTreeMap<String, String>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgroupV2RunResult {
    pub run_id: String,
    pub cgroup_id: String,
    pub state: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    pub elapsed_wall_time_ns: u64,
    pub cpu_usage_usec: u64,
    pub cpu_user_usec: u64,
    pub cpu_system_usec: u64,
    pub peak_memory_bytes: u64,
    pub oom_event_delta: u64,
    pub oom_kill_event_delta: u64,
    pub cleanup_state: String,
    pub receipt_sha256: String,
    pub evidence_level: &'static str,
}

#[derive(Debug, Clone)]
struct RawCgroupSnapshot {
    cpu_stat: String,
    memory_peak: String,
    memory_events: String,
    cgroup_events: String,
}

#[derive(Debug, Clone)]
struct ArtifactRecord {
    role: String,
    path: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Debug, Clone)]
struct MountEntry {
    mount_point: PathBuf,
    raw_line: String,
}

#[derive(Debug)]
struct ChildCompletion {
    status: ExitStatus,
    timed_out: bool,
    elapsed_wall_time_ns: u64,
}

fn validate_run_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("run_id must be 1..64 ASCII letters, digits, '-' or '_'".to_owned());
    }
    Ok(())
}

fn validate_environment(environment: &BTreeMap<String, String>) -> Result<(), String> {
    for (key, value) in environment {
        if !BASE_ENVIRONMENT_KEYS.contains(&key.as_str()) {
            return Err(format!("environment key {key:?} is not in the reviewed allow-list"));
        }
        if value.contains('\0') {
            return Err(format!("environment value for {key} contains NUL"));
        }
    }
    Ok(())
}

fn decode_mount_field(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

fn cgroup2_mount_for(root: &Path) -> Result<MountEntry, String> {
    let mountinfo = fs::read_to_string("/proc/self/mountinfo")
        .map_err(|error| format!("cannot read /proc/self/mountinfo: {error}"))?;
    let mut best: Option<MountEntry> = None;
    for line in mountinfo.lines() {
        let Some((left, right)) = line.split_once(" - ") else {
            continue;
        };
        let left_fields = left.split_whitespace().collect::<Vec<_>>();
        let right_fields = right.split_whitespace().collect::<Vec<_>>();
        if left_fields.len() < 6 || right_fields.first().copied() != Some("cgroup2") {
            continue;
        }
        let mount_point = PathBuf::from(decode_mount_field(left_fields[4]));
        if root.starts_with(&mount_point) {
            let candidate = MountEntry {
                mount_point,
                raw_line: line.to_owned(),
            };
            if best
                .as_ref()
                .is_none_or(|current| candidate.mount_point.as_os_str().len() > current.mount_point.as_os_str().len())
            {
                best = Some(candidate);
            }
        }
    }
    best.ok_or_else(|| "delegated root is not beneath a cgroup2 mount in mountinfo".to_owned())
}

fn tokens_from_file(path: &Path, label: &str) -> Result<BTreeSet<String>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {label} at {}: {error}", path.display()))?;
    Ok(raw.split_whitespace().map(str::to_owned).collect())
}

fn validate_delegated_root(path: &Path) -> Result<(PathBuf, MountEntry), String> {
    let root = fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve delegated cgroup root {}: {error}", path.display()))?;
    if !fs::metadata(&root)
        .map_err(|error| format!("cannot inspect delegated cgroup root: {error}"))?
        .is_dir()
    {
        return Err("delegated cgroup root must be a directory".to_owned());
    }
    let mount = cgroup2_mount_for(&root)?;
    if root == mount.mount_point {
        return Err("refusing to create operation groups directly below the cgroup2 mount root; provide a delegated subtree".to_owned());
    }
    let available = tokens_from_file(&root.join("cgroup.controllers"), "cgroup.controllers")?;
    let enabled = tokens_from_file(&root.join("cgroup.subtree_control"), "cgroup.subtree_control")?;
    for controller in ["cpu", "memory"] {
        if !available.contains(controller) {
            return Err(format!(
                "delegated root does not expose the {controller} controller"
            ));
        }
        if !enabled.contains(controller) {
            return Err(format!(
                "delegated root has not enabled {controller} for children; enable it through the administrator/delegation manager, not this runner"
            ));
        }
    }
    OpenOptions::new()
        .write(true)
        .open(root.join("cgroup.procs"))
        .map_err(|error| format!("delegated root cgroup.procs is not writable: {error}"))?;
    Ok((root, mount))
}

fn cgroup_event_flag(path: &Path, key: &str) -> Result<bool, String> {
    let raw = fs::read_to_string(path.join("cgroup.events"))
        .map_err(|error| format!("cannot read cgroup.events: {error}"))?;
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some(key) {
            return match parts.next() {
                Some("0") => Ok(false),
                Some("1") => Ok(true),
                value => Err(format!("cgroup.events {key} has invalid value {value:?}")),
            };
        }
    }
    Err(format!("cgroup.events is missing {key}"))
}

fn read_memory_peak(reader: &mut File) -> Result<String, String> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot rewind retained memory.peak descriptor: {error}"))?;
    let mut raw = String::new();
    reader
        .read_to_string(&mut raw)
        .map_err(|error| format!("cannot read retained memory.peak descriptor: {error}"))?;
    Ok(raw)
}

fn capture_snapshot(
    cgroup_path: &Path,
    cgroup_id: &str,
    memory_peak_reader: &mut File,
    memory_peak_reader_id: &str,
    memory_peak_reader_flags: u64,
    mount_options: &str,
    monotonic_origin: Instant,
) -> Result<(CgroupV2Snapshot, RawCgroupSnapshot), String> {
    let cpu_stat = fs::read_to_string(cgroup_path.join("cpu.stat"))
        .map_err(|error| format!("cannot read cpu.stat: {error}"))?;
    let memory_peak = read_memory_peak(memory_peak_reader)?;
    let memory_events = fs::read_to_string(cgroup_path.join("memory.events"))
        .map_err(|error| format!("cannot read memory.events: {error}"))?;
    let cgroup_events = fs::read_to_string(cgroup_path.join("cgroup.events"))
        .map_err(|error| format!("cannot read cgroup.events: {error}"))?;
    let raw = RawCgroupSnapshot {
        cpu_stat,
        memory_peak,
        memory_events,
        cgroup_events,
    };
    let monotonic_time_ns = u64::try_from(monotonic_origin.elapsed().as_nanos())
        .map_err(|_| "monotonic timestamp exceeds u64 nanoseconds".to_owned())?;
    let snapshot = crate::resource_telemetry::parse_cgroup_v2_snapshot(CgroupV2SnapshotInput {
        cgroup_id,
        memory_peak_reader_id,
        memory_peak_reader_flags,
        cgroup2_mount_options: mount_options,
        monotonic_time_ns,
        cpu_stat: &raw.cpu_stat,
        memory_peak: &raw.memory_peak,
        memory_events: &raw.memory_events,
        cgroup_events: &raw.cgroup_events,
    })?;
    Ok((snapshot, raw))
}

fn write_cgroup_procs(cgroup_path: &Path, pid: u32) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .open(cgroup_path.join("cgroup.procs"))
        .map_err(|error| format!("cannot open child cgroup.procs: {error}"))?;
    file.write_all(format!("{pid}\n").as_bytes())
        .map_err(|error| format!("cannot place PID {pid} into the child cgroup: {error}"))
}

fn spawn_gated(
    mut command: Command,
    cgroup_path: &Path,
) -> Result<Child, String> {
    let (child_gate, mut parent_gate) =
        UnixStream::pair().map_err(|error| format!("cannot create pre-exec gate: {error}"))?;
    let child_fd = child_gate.as_raw_fd();
    let parent_fd = parent_gate.as_raw_fd();

    // SAFETY: the child-side pre_exec closure calls only close/read/close on inherited
    // descriptors. It does not allocate, take locks, or run target code before placement.
    unsafe {
        command.pre_exec(move || {
            // SAFETY: parent_fd is an inherited descriptor from UnixStream::pair().
            if close(parent_fd) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut token = 0u8;
            // SAFETY: child_fd remains open in the forked child and token is a valid 1-byte buffer.
            let count = read(child_fd, &mut token as *mut u8, 1);
            // SAFETY: child_fd is closed exactly once in this process.
            let close_result = close(child_fd);
            if close_result != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if count != 1 || token != 1 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "cgroup placement gate closed before release",
                ));
            }
            Ok(())
        });
    }

    let child = command
        .spawn()
        .map_err(|error| format!("cannot spawn gated child process: {error}"))?;
    drop(child_gate);
    let pid = child.id();
    if let Err(error) = write_cgroup_procs(cgroup_path, pid) {
        drop(parent_gate);
        let mut child = child;
        let _ = child.wait();
        return Err(error);
    }
    if let Err(error) = parent_gate.write_all(&[1]) {
        drop(parent_gate);
        let mut child = child;
        let _ = child.wait();
        return Err(format!("cannot release child after cgroup placement: {error}"));
    }
    drop(parent_gate);
    Ok(child)
}

fn wait_for_process_tree(
    child: &mut Child,
    cgroup_path: &Path,
    timeout: Duration,
) -> Result<ChildCompletion, String> {
    let started = Instant::now();
    let mut status = None;
    let mut timed_out = false;
    let mut kill_started: Option<Instant> = None;
    loop {
        if status.is_none() {
            status = child
                .try_wait()
                .map_err(|error| format!("cannot poll direct child: {error}"))?;
        }
        let populated = cgroup_event_flag(cgroup_path, "populated")?;
        if status.is_some() && !populated {
            let elapsed_wall_time_ns = u64::try_from(started.elapsed().as_nanos())
                .map_err(|_| "elapsed wall time exceeds u64 nanoseconds".to_owned())?;
            return Ok(ChildCompletion {
                status: status.expect("checked above"),
                timed_out,
                elapsed_wall_time_ns,
            });
        }
        if !timed_out && started.elapsed() >= timeout {
            timed_out = true;
            fs::write(cgroup_path.join("cgroup.kill"), b"1\n")
                .map_err(|error| format!("cannot recursively kill timed-out cgroup: {error}"))?;
            kill_started = Some(Instant::now());
        }
        if kill_started.is_some_and(|value| value.elapsed() >= CGROUP_KILL_GRACE) {
            return Err(
                "cgroup remained populated after recursive kill grace; preserving cgroup for inspection"
                    .to_owned(),
            );
        }
        thread::sleep(CGROUP_POLL_INTERVAL);
    }
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn csv_row(values: &[String]) -> String {
    values
        .iter()
        .map(|value| csv_escape(value))
        .collect::<Vec<_>>()
        .join(",")
}

fn canonical_named_fields(names: &[&str], values: &[String]) -> Result<Vec<u8>, String> {
    if names.len() != values.len() {
        return Err("receipt field names/values length mismatch".to_owned());
    }
    let mut bytes = Vec::new();
    for (name, value) in names.iter().zip(values) {
        append_string(&mut bytes, name);
        append_string(&mut bytes, value);
    }
    Ok(bytes)
}

fn hex_encode(value: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(value.len() * 2);
    for byte in value {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

fn command_manifest(program: &Path, arguments: &[String], cwd: &Path) -> Result<(String, Vec<u8>), String> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, &program.to_string_lossy());
    append_string(&mut bytes, &cwd.to_string_lossy());
    append_u64(&mut bytes, arguments.len() as u64);
    let mut output = String::from("schema=symtropy-command-identity/0.1.0\n");
    output.push_str(&format!("program_sha256={}\n", sha256_hex(program.to_string_lossy().as_bytes())?));
    output.push_str(&format!("cwd_sha256={}\n", sha256_hex(cwd.to_string_lossy().as_bytes())?));
    for (index, argument) in arguments.iter().enumerate() {
        append_string(&mut bytes, argument);
        output.push_str(&format!(
            "arg_{index}_bytes={} arg_{index}_sha256={}\n",
            argument.len(),
            sha256_hex(argument.as_bytes())?
        ));
    }
    Ok((sha256_hex(&bytes)?, output.into_bytes()))
}

fn environment_manifest(environment: &BTreeMap<String, String>) -> Result<(String, Vec<u8>), String> {
    let mut canonical = Vec::new();
    let mut output = String::from("schema=symtropy-environment-identity/0.1.0\n");
    append_u64(&mut canonical, environment.len() as u64);
    for (key, value) in environment {
        append_string(&mut canonical, key);
        append_string(&mut canonical, value);
        output.push_str(&format!(
            "key_utf8_hex={} value_bytes={} value_sha256={}\n",
            hex_encode(key.as_bytes()),
            value.len(),
            sha256_hex(value.as_bytes())?
        ));
    }
    Ok((sha256_hex(&canonical)?, output.into_bytes()))
}

fn write_artifact(
    root: &Path,
    role: &str,
    relative_path: &str,
    bytes: &[u8],
) -> Result<ArtifactRecord, String> {
    let path = root.join(relative_path);
    fs::write(&path, bytes)
        .map_err(|error| format!("cannot write artifact {}: {error}", path.display()))?;
    Ok(ArtifactRecord {
        role: role.to_owned(),
        path: relative_path.to_owned(),
        byte_length: bytes.len() as u64,
        sha256: sha256_hex(bytes)?,
    })
}

fn record_existing_artifact(
    root: &Path,
    role: &str,
    relative_path: &str,
) -> Result<ArtifactRecord, String> {
    let bytes = fs::read(root.join(relative_path))
        .map_err(|error| format!("cannot read artifact {relative_path}: {error}"))?;
    Ok(ArtifactRecord {
        role: role.to_owned(),
        path: relative_path.to_owned(),
        byte_length: bytes.len() as u64,
        sha256: sha256_hex(&bytes)?,
    })
}

fn write_artifact_manifest(root: &Path, artifacts: &[ArtifactRecord]) -> Result<(String, ArtifactRecord), String> {
    let header = "role,path,byte_length,sha256\n";
    let mut output = String::from(header);
    for artifact in artifacts {
        output.push_str(&csv_row(&[
            artifact.role.clone(),
            artifact.path.clone(),
            artifact.byte_length.to_string(),
            artifact.sha256.clone(),
        ]));
        output.push('\n');
    }
    let digest = sha256_hex(output.as_bytes())?;
    let record = write_artifact(root, "artifact_manifest", "artifacts.csv", output.as_bytes())?;
    Ok((digest, record))
}

fn validate_result_directory(
    path: &Path,
    delegated_root: &Path,
    cgroup2_mount_point: &Path,
) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "result directory must have a parent".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create result parent {}: {error}", parent.display()))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve result parent: {error}"))?;
    let candidate = canonical_parent.join(
        path.file_name()
            .ok_or_else(|| "result directory name is empty".to_owned())?,
    );
    if candidate.starts_with(cgroup2_mount_point) {
        return Err("result directory must be outside the cgroup2 filesystem".to_owned());
    }
    if candidate.starts_with(delegated_root) || delegated_root.starts_with(&candidate) {
        return Err("result directory must be outside the delegated cgroup subtree and its ancestors".to_owned());
    }
    if fs::symlink_metadata(&candidate).is_ok() {
        return Err("result directory already exists; run bundles are immutable and unique".to_owned());
    }
    fs::create_dir(&candidate)
        .map_err(|error| format!("cannot create result directory: {error}"))?;
    Ok(candidate)
}

fn parse_mountpoint(raw: &str) -> Result<PathBuf, String> {
    let fields = raw.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 6 {
        return Err("mountinfo line is malformed".to_owned());
    }
    Ok(PathBuf::from(
        fields[4]
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\"),
    ))
}

fn cgroup_identity(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot stat operation cgroup: {error}"))?;
    Ok(format!(
        "{}:dev={}:ino={}",
        path.display(),
        metadata.dev(),
        metadata.ino()
    ))
}

fn open_memory_peak(path: &Path) -> Result<(File, String, u64), String> {
    let file = OpenOptions::new()
        .read(true)
        .open(path.join("memory.peak"))
        .map_err(|error| format!("cannot open read-only memory.peak FD: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot stat memory.peak FD: {error}"))?;
    // SAFETY: F_GETFL reads flags for the live descriptor returned by OpenOptions.
    let flags = unsafe { fcntl(file.as_raw_fd(), F_GETFL) };
    if flags < 0 {
        return Err(format!(
            "fcntl(F_GETFL) failed for memory.peak: {}",
            std::io::Error::last_os_error()
        ));
    }
    if flags & O_ACCMODE != O_RDONLY {
        return Err("memory.peak FD is not O_RDONLY".to_owned());
    }
    Ok((
        file,
        format!("dev={}:ino={}", metadata.dev(), metadata.ino()),
        flags as u64,
    ))
}

struct EmptyCgroupGuard {
    path: PathBuf,
}

impl Drop for EmptyCgroupGuard {
    fn drop(&mut self) {
        // Never remove a cgroup that may still contain a process or descendant.
        if read_populated(&self.path).is_ok_and(|populated| !populated) {
            let _ = fs::remove_dir(&self.path);
        }
    }
}

fn create_operation_cgroup(root: &Path, run_id: &str) -> Result<PathBuf, String> {
    let name = format!("symtropy-{run_id}");
    let path = root.join(name);
    if fs::symlink_metadata(&path).is_ok() {
        return Err("operation cgroup already exists".to_owned());
    }
    fs::create_dir(&path)
        .map_err(|error| format!("cannot create operation cgroup in delegated subtree: {error}"))?;
    for required in ["cgroup.procs", "cgroup.events", "cpu.stat", "memory.peak", "memory.events", "cgroup.kill"] {
        if !path.join(required).exists() {
            return Err(format!("new cgroup is missing required interface {required}"));
        }
    }
    if fs::read_to_string(path.join("cgroup.type"))
        .map_err(|error| format!("cannot read operation cgroup type: {error}"))?
        .trim() != "domain"
    {
        return Err("operation cgroup must be a domain cgroup".to_owned());
    }
    let available = tokens_from_file(&path.join("cgroup.controllers"), "operation cgroup.controllers")?;
    for controller in ["cpu", "memory"] {
        if !available.contains(controller) {
            return Err(format!("operation cgroup did not inherit {controller} controller"));
        }
    }
    Ok(path)
}

fn read_populated(path: &Path) -> Result<bool, String> {
    let raw = fs::read_to_string(path.join("cgroup.events"))
        .map_err(|error| format!("cannot read cgroup.events: {error}"))?;
    for line in raw.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() == Some("populated") {
            return match fields.next() {
                Some("0") => Ok(false),
                Some("1") => Ok(true),
                other => Err(format!("invalid cgroup.events populated value: {other:?}")),
            };
        }
    }
    Err("cgroup.events is missing populated".to_owned())
}

fn kill_operation_cgroup(path: &Path) -> Result<(), String> {
    fs::write(path.join("cgroup.kill"), b"1\n")
        .map_err(|error| format!("cannot write cgroup.kill: {error}"))
}

/// Executes the target only after its PID is assigned to the operation cgroup.
/// Result files are immutable within a fresh result directory.
pub fn run_cgroup_v2_process_tree(request: &CgroupV2RunRequest) -> Result<CgroupV2RunResult, String> {
    validate_run_id(&request.run_id)?;
    validate_environment(&request.environment)?;
    if request.program.as_os_str().is_empty() || request.arguments.iter().any(|arg| arg.contains('\0')) {
        return Err("program and argv must be non-empty and NUL-free".to_owned());
    }
    if request.timeout.is_zero() {
        return Err("timeout must be positive".to_owned());
    }
    let (delegated_root, mount) = validate_delegated_root(&request.delegated_root)?;
    let result_root = validate_result_directory(
        &request.result_directory,
        &delegated_root,
        &mount.mount_point,
    )?;
    let cgroup_path = create_operation_cgroup(&delegated_root, &request.run_id)?;
    let _empty_cgroup_guard = EmptyCgroupGuard {
        path: cgroup_path.clone(),
    };
    let cgroup_id = cgroup_identity(&cgroup_path)?;
    let (mut memory_peak_reader, reader_id, reader_flags) =
        open_memory_peak(&cgroup_path)?;
    let monotonic_origin = Instant::now();
    let (start, start_raw) = capture_snapshot(
        &cgroup_path,
        &cgroup_id,
        &mut memory_peak_reader,
        &reader_id,
        reader_flags,
        &mount.raw_line,
        monotonic_origin,
    )?;
    if start.populated || start.frozen || start.memory_peak_bytes != 0 {
        return Err("fresh operation cgroup is not empty/unfrozen or memory.peak baseline is nonzero".to_owned());
    }

    let cwd = fs::canonicalize(&request.current_directory)
        .map_err(|error| format!("cannot resolve child current directory: {error}"))?;
    if !cwd.is_dir() {
        return Err("child current directory must be a directory".to_owned());
    }
    let (command_sha256, command_bytes) =
        command_manifest(&request.program, &request.arguments, &cwd)?;
    let (environment_sha256, environment_bytes) =
        environment_manifest(&request.environment)?;
    let command_artifact = write_artifact(
        &result_root,
        "command_manifest",
        "command-manifest.txt",
        &command_bytes,
    )?;
    let environment_artifact = write_artifact(
        &result_root,
        "environment_manifest",
        "environment-manifest.txt",
        &environment_bytes,
    )?;
    let stdout_path = result_root.join("stdout.bin");
    let stderr_path = result_root.join("stderr.bin");
    let stdout_file = File::create(&stdout_path)
        .map_err(|error| format!("cannot create stdout capture: {error}"))?;
    let stderr_file = File::create(&stderr_path)
        .map_err(|error| format!("cannot create stderr capture: {error}"))?;

    let mut command = Command::new(&request.program);
    command
        .args(&request.arguments)
        .current_dir(&cwd)
        .env_clear()
        .envs(&request.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = spawn_gated(command, &cgroup_path)?;
    let completion = wait_for_process_tree(&mut child, &cgroup_path, request.timeout)?;
    let (end, end_raw) = capture_snapshot(
        &cgroup_path,
        &cgroup_id,
        &mut memory_peak_reader,
        &reader_id,
        reader_flags,
        &mount.raw_line,
        monotonic_origin,
    )?;
    let resource_report: CgroupV2ResourceReport =
        derive_cgroup_v2_resource_report(&start, &end)?;
    let resource_csv = cgroup_v2_resource_evidence_to_csv(&start, &end, &resource_report);
    let resource_artifact = write_artifact(
        &result_root,
        "resource_evidence",
        "resource-evidence.csv",
        resource_csv.as_bytes(),
    )?;

    let resource_sha256 = resource_artifact.sha256.clone();
    let mut artifacts = vec![
        command_artifact,
        environment_artifact,
        resource_artifact,
    ];
    for (prefix, raw) in [("start", &start_raw), ("end", &end_raw)] {
        for (name, value) in [
            ("cpu_stat", &raw.cpu_stat),
            ("memory_peak", &raw.memory_peak),
            ("memory_events", &raw.memory_events),
            ("cgroup_events", &raw.cgroup_events),
        ] {
            let relative = format!("{prefix}-{name}.txt");
            artifacts.push(write_artifact(
                &result_root,
                &format!("{prefix}_{name}"),
                &relative,
                value.as_bytes(),
            )?);
        }
    }
    artifacts.push(write_artifact(
        &result_root,
        "cgroup2_mountinfo",
        "cgroup2-mountinfo.txt",
        mount.raw_line.as_bytes(),
    )?);
    artifacts.push(record_existing_artifact(&result_root, "stdout", "stdout.bin")?);
    artifacts.push(record_existing_artifact(&result_root, "stderr", "stderr.bin")?);
    let (artifact_manifest_sha256, artifact_manifest) =
        write_artifact_manifest(&result_root, &artifacts)?;

    let status = completion.status;
    let exit_code = status.code();
    let signal = status.signal();
    let timed_out = completion.timed_out;
    let mut state = if timed_out {
        "timed_out"
    } else if exit_code == Some(0) {
        "succeeded"
    } else {
        "failed"
    };
    drop(memory_peak_reader);
    let cleanup_state = match fs::remove_dir(&cgroup_path) {
        Ok(()) => "removed",
        Err(_) => "retained",
    };
    if cleanup_state == "retained" && state == "succeeded" {
        state = "cleanup_failed";
    }

    let names = [
        "schema", "run_id", "cgroup_id", "state", "exit_code", "signal",
        "timed_out", "elapsed_wall_time_ns", "cpu_usage_usec", "cpu_user_usec",
        "cpu_system_usec", "cpu_core_hours_numerator_usec",
        "cpu_core_hours_denominator_usec", "peak_memory_bytes", "oom_event_delta",
        "oom_kill_event_delta", "oom_group_kill_event_delta", "command_sha256",
        "environment_sha256", "resource_evidence_sha256", "artifact_manifest_sha256",
        "stdout_sha256", "stderr_sha256", "cleanup_state", "evidence_level",
    ];
    let stdout = fs::read(&stdout_path).map_err(|error| format!("cannot read captured stdout: {error}"))?;
    let stderr = fs::read(&stderr_path).map_err(|error| format!("cannot read captured stderr: {error}"))?;
    let stdout_sha256 = sha256_hex(&stdout)?;
    let stderr_sha256 = sha256_hex(&stderr)?;
    let _manifest = artifact_manifest;
    let values = vec![
        CGROUP_V2_RUN_SCHEMA.to_owned(),
        request.run_id.clone(),
        cgroup_id.clone(),
        state.to_owned(),
        exit_code.map(|value| value.to_string()).unwrap_or_default(),
        signal.map(|value| value.to_string()).unwrap_or_default(),
        timed_out.to_string(),
        completion.elapsed_wall_time_ns.to_string(),
        resource_report.cpu_usage_usec.to_string(),
        resource_report.cpu_user_usec.to_string(),
        resource_report.cpu_system_usec.to_string(),
        resource_report.cpu_core_hours_numerator_usec.to_string(),
        resource_report.cpu_core_hours_denominator_usec.to_string(),
        resource_report.peak_memory_bytes.to_string(),
        resource_report.oom_event_delta.to_string(),
        resource_report.oom_kill_event_delta.to_string(),
        resource_report.oom_group_kill_event_delta.map(|value| value.to_string()).unwrap_or_default(),
        command_sha256,
        environment_sha256,
        resource_sha256,
        artifact_manifest_sha256,
        stdout_sha256,
        stderr_sha256,
        cleanup_state.to_owned(),
        CGROUP_V2_RUN_EVIDENCE_LEVEL.to_owned(),
    ];
    let receipt_sha256 = sha256_hex(&canonical_named_fields(&names, &values)?)?;
    let mut row = values.clone();
    row.push(receipt_sha256.clone());
    let mut header = names.join(",");
    header.push_str(",receipt_sha256\n");
    let mut csv = header;
    csv.push_str(&csv_row(&row));
    csv.push('\n');
    fs::write(result_root.join("run-receipt.csv"), csv)
        .map_err(|error| format!("cannot write cgroup run receipt: {error}"))?;

    Ok(CgroupV2RunResult {
        run_id: request.run_id.clone(),
        cgroup_id,
        state: state.to_owned(),
        exit_code,
        signal,
        timed_out,
        elapsed_wall_time_ns: completion.elapsed_wall_time_ns,
        cpu_usage_usec: resource_report.cpu_usage_usec,
        cpu_user_usec: resource_report.cpu_user_usec,
        cpu_system_usec: resource_report.cpu_system_usec,
        peak_memory_bytes: resource_report.peak_memory_bytes,
        oom_event_delta: resource_report.oom_event_delta,
        oom_kill_event_delta: resource_report.oom_kill_event_delta,
        cleanup_state: cleanup_state.to_owned(),
        receipt_sha256,
        evidence_level: CGROUP_V2_RUN_EVIDENCE_LEVEL,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_run_ids_and_environment_allowlist() {
        assert!(validate_run_id("solver-7").is_ok());
        assert!(validate_run_id("../solver").is_err());
        assert!(validate_run_id("has space").is_err());
        let env = BTreeMap::from([("OMP_NUM_THREADS".to_owned(), "4".to_owned())]);
        assert!(validate_environment(&env).is_ok());
        let unsafe_env = BTreeMap::from([("LD_PRELOAD".to_owned(), "/tmp/evil.so".to_owned())]);
        assert!(validate_environment(&unsafe_env).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn child_cannot_exec_until_pid_placement_write_completes() {
        let root = std::env::temp_dir().join(format!(
            "cgroup-v2-gate-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let cgroup_procs = root.join("cgroup.procs");
        fs::write(&cgroup_procs, b"").unwrap();
        let marker = root.join("target-executed");
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "test -s \"$1\" && printf executed > \"$2\"",
            "gate-test",
            cgroup_procs.to_str().unwrap(),
            marker.to_str().unwrap(),
        ]);
        let child = spawn_gated(command, &root).unwrap();
        let child_pid = child.id();
        assert!(child.wait_with_output().unwrap().status.success());
        let pid = fs::read_to_string(cgroup_procs).unwrap();
        assert_eq!(pid.trim(), child_pid.to_string());
        assert_eq!(fs::read_to_string(marker).unwrap(), "executed");
        fs::remove_dir_all(root).unwrap();
    }
}
