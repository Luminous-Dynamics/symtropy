// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Candidate-only sandbox command construction for blind AERB runs.
//!
//! This module builds a fail-closed bubblewrap + prlimit invocation. It does
//! not launch the process, prove that user namespaces are enabled, enforce
//! cgroup-level resource limits, or establish that the host is correctly
//! configured. The caller must execute argv directly (never through a shell)
//! and treat launch failure as campaign failure. There is no unsandboxed
//! fallback.

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxPaths {
    pub bubblewrap: PathBuf,
    pub prlimit: PathBuf,
    pub candidate_input_root: PathBuf,
    pub runtime_bundle_root: PathBuf,
    pub candidate_output_root: PathBuf,
    /// Used only to reject overlapping mounts; never mounted into the candidate.
    pub evaluator_secret_root: PathBuf,
    /// Must resolve to a file strictly inside runtime_bundle_root.
    pub candidate_executable: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SandboxLimits {
    /// RLIMIT_AS, not a cgroup memory.max guarantee.
    pub address_space_bytes: u64,
    /// RLIMIT_CPU soft/hard ceiling in seconds.
    pub cpu_seconds: u64,
    /// RLIMIT_NPROC; defense-in-depth, not a cgroup pids.max guarantee.
    pub max_processes: u64,
    pub max_open_files: u64,
    /// RLIMIT_FSIZE per-file output ceiling.
    pub max_file_bytes: u64,
    /// Maximum size for the private in-sandbox /tmp tmpfs.
    pub tmpfs_bytes: u64,
}

impl SandboxLimits {
    pub fn validate(self) -> Result<(), SandboxError> {
        if self.address_space_bytes == 0
            || self.cpu_seconds == 0
            || self.max_processes == 0
            || self.max_open_files == 0
            || self.max_file_bytes == 0
            || self.tmpfs_bytes == 0
        {
            return Err(SandboxError::ZeroResourceLimit);
        }
        Ok(())
    }
}

/// The sandbox launcher accepts only a closed set of typed arguments.
/// Free-form argv is forbidden so callers cannot smuggle hidden labels or
/// evaluator paths into the candidate process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateSandboxArgument {
    CanaryMode,
    NetworkProbePort(u16),
    HostPidNamespace(String),
    CampaignId(String),
    EpisodeId(String),
    CandidateInputArtifact(String),
    CandidateOutputArtifact(String),
}

fn is_hex_32(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_safe_artifact_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

impl CandidateSandboxArgument {
    fn to_argv(&self) -> Result<Vec<OsString>, SandboxError> {
        let argument = match self {
            Self::CanaryMode => vec![OsString::from("--candidate")],
            Self::NetworkProbePort(port) if *port != 0 => {
                vec![OsString::from(format!("--network-port={port}"))]
            }
            Self::NetworkProbePort(_) => return Err(SandboxError::InvalidCandidateArgument),
            Self::HostPidNamespace(namespace)
                if namespace
                    .strip_prefix("pid:[")
                    .and_then(|value| value.strip_suffix(']'))
                    .is_some_and(|digits| {
                        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
                    }) =>
            {
                vec![OsString::from(format!("--host-pid-ns={namespace}"))]
            }
            Self::HostPidNamespace(_) => return Err(SandboxError::InvalidCandidateArgument),
            Self::CampaignId(value) if is_hex_32(value.strip_prefix("camp_").unwrap_or("")) => {
                vec![OsString::from(format!("--campaign-id={value}"))]
            }
            Self::CampaignId(_) => return Err(SandboxError::InvalidCandidateArgument),
            Self::EpisodeId(value) if is_hex_32(value.strip_prefix("ep_").unwrap_or("")) => {
                vec![OsString::from(format!("--episode-id={value}"))]
            }
            Self::EpisodeId(_) => return Err(SandboxError::InvalidCandidateArgument),
            Self::CandidateInputArtifact(name) if is_safe_artifact_name(name) => {
                vec![OsString::from(format!("--input=/candidate/input/{name}"))]
            }
            Self::CandidateInputArtifact(_) => return Err(SandboxError::InvalidCandidateArgument),
            Self::CandidateOutputArtifact(name) if is_safe_artifact_name(name) => {
                vec![OsString::from(format!("--output=/candidate/output/{name}"))]
            }
            Self::CandidateOutputArtifact(_) => return Err(SandboxError::InvalidCandidateArgument),
        };
        Ok(argument)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxCommand {
    /// Absolute prlimit executable.
    pub program: PathBuf,
    /// Arguments only; argv[0] is supplied by the process launcher.
    pub args: Vec<OsString>,
    pub candidate_input_root: PathBuf,
    pub runtime_bundle_root: PathBuf,
    pub candidate_output_root: PathBuf,
    pub evaluator_secret_root: PathBuf,
    pub candidate_executable_in_sandbox: PathBuf,
}

impl SandboxCommand {
    /// Create the host launcher with a cleared environment. Callers should use
    /// this method rather than reconstructing Command manually: host environment
    /// secrets must not reach prlimit or the Bubblewrap setup process.
    pub fn process_command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.program);
        command.args(&self.args);
        command.env_clear();
        command.stdin(std::process::Stdio::null());
        // Candidate-controlled stdout/stderr are not allowed to flow into the
        // evaluator log channel. Campaign output must use the designated,
        // quota-controlled candidate output artifact path.
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
        command
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SandboxError {
    MissingPath(&'static str),
    NotDirectory(&'static str),
    NotFile(&'static str),
    NotExecutable(&'static str),
    NonUtf8Path(&'static str),
    PathOverlap(&'static str),
    ExecutableOutsideRuntime,
    RuntimeBinMissing,
    InvalidCandidateArgument,
    ZeroResourceLimit,
    PathResolution(&'static str),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPath(name) => write!(formatter, "missing sandbox path: {name}"),
            Self::NotDirectory(name) => write!(formatter, "sandbox path is not a directory: {name}"),
            Self::NotFile(name) => write!(formatter, "sandbox path is not a file: {name}"),
            Self::NotExecutable(name) => write!(formatter, "sandbox tool is not executable: {name}"),
            Self::NonUtf8Path(name) => write!(formatter, "sandbox path is not valid UTF-8: {name}"),
            Self::PathOverlap(name) => write!(formatter, "sandbox mount roots overlap: {name}"),
            Self::ExecutableOutsideRuntime => formatter.write_str("candidate executable is outside the sealed runtime bundle"),
            Self::RuntimeBinMissing => formatter.write_str("runtime bundle must contain a bin directory"),
            Self::InvalidCandidateArgument => formatter.write_str("candidate argument contains a host path, traversal, NUL, or unsupported absolute path"),
            Self::ZeroResourceLimit => formatter.write_str("all sandbox resource limits must be positive"),
            Self::PathResolution(name) => write!(formatter, "unable to canonicalize sandbox path: {name}"),
        }
    }
}

impl std::error::Error for SandboxError {}

fn canonical_dir(path: &Path, name: &'static str) -> Result<PathBuf, SandboxError> {
    if !path.exists() {
        return Err(SandboxError::MissingPath(name));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| SandboxError::PathResolution(name))?;
    if !canonical.is_dir() {
        return Err(SandboxError::NotDirectory(name));
    }
    if canonical.to_str().is_none() {
        return Err(SandboxError::NonUtf8Path(name));
    }
    Ok(canonical)
}

fn canonical_executable(path: &Path, name: &'static str) -> Result<PathBuf, SandboxError> {
    if !path.exists() {
        return Err(SandboxError::MissingPath(name));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| SandboxError::PathResolution(name))?;
    if !canonical.is_file() {
        return Err(SandboxError::NotFile(name));
    }
    if canonical.to_str().is_none() {
        return Err(SandboxError::NonUtf8Path(name));
    }
    let mode = canonical
        .metadata()
        .map_err(|_| SandboxError::PathResolution(name))?
        .permissions()
        .mode();
    if mode & 0o111 == 0 {
        return Err(SandboxError::NotExecutable(name));
    }
    Ok(canonical)
}

fn paths_are_disjoint(left: &Path, right: &Path) -> bool {
    !left.starts_with(right) && !right.starts_with(left)
}

/// Construct a candidate process command with a deliberately small filesystem.
///
/// Only these host roots are mounted:
/// - candidate input at /candidate/input (read-only),
/// - the sealed executable/runtime bundle at /runtime (read-only),
/// - candidate output at /candidate/output (writable).
///
/// Evaluator secrets are never mounted. There is no host root, home, /etc, /sys
/// or host network binding. Network and other namespaces are unshared. A caller
/// must fail the campaign if this invocation cannot start; never fall back to
/// an unsandboxed process. prlimit sets rlimits, not cgroup enforcement.
pub fn build_candidate_sandbox_command(
    paths: &SandboxPaths,
    limits: SandboxLimits,
    candidate_arguments: &[CandidateSandboxArgument],
) -> Result<SandboxCommand, SandboxError> {
    limits.validate()?;

    let bubblewrap = canonical_executable(&paths.bubblewrap, "bubblewrap")?;
    let prlimit = canonical_executable(&paths.prlimit, "prlimit")?;
    let candidate_input = canonical_dir(&paths.candidate_input_root, "candidate_input_root")?;
    let runtime_root = canonical_dir(&paths.runtime_bundle_root, "runtime_bundle_root")?;
    let candidate_output = canonical_dir(&paths.candidate_output_root, "candidate_output_root")?;
    let evaluator_secret = canonical_dir(&paths.evaluator_secret_root, "evaluator_secret_root")?;
    let candidate_executable =
        canonical_executable(&paths.candidate_executable, "candidate_executable")?;

    let roots = [
        ("candidate input/runtime", &candidate_input, &runtime_root),
        (
            "candidate input/output",
            &candidate_input,
            &candidate_output,
        ),
        (
            "candidate input/evaluator",
            &candidate_input,
            &evaluator_secret,
        ),
        ("runtime/output", &runtime_root, &candidate_output),
        ("runtime/evaluator", &runtime_root, &evaluator_secret),
        ("output/evaluator", &candidate_output, &evaluator_secret),
    ];
    for (name, left, right) in roots {
        if !paths_are_disjoint(left, right) {
            return Err(SandboxError::PathOverlap(name));
        }
    }

    if !runtime_root.join("bin").is_dir() {
        return Err(SandboxError::RuntimeBinMissing);
    }
    let executable_relative = candidate_executable
        .strip_prefix(&runtime_root)
        .map_err(|_| SandboxError::ExecutableOutsideRuntime)?;
    if executable_relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) || !candidate_executable.starts_with(&runtime_root)
    {
        return Err(SandboxError::ExecutableOutsideRuntime);
    }
    let encoded_arguments = candidate_arguments
        .iter()
        .map(CandidateSandboxArgument::to_argv)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let executable_in_sandbox = Path::new("/runtime").join(executable_relative);
    let executable_in_sandbox = executable_in_sandbox
        .to_str()
        .ok_or(SandboxError::NonUtf8Path("candidate executable in sandbox"))?;

    let mut bubblewrap_args: Vec<OsString> = Vec::new();
    for item in [
        "--die-with-parent",
        "--new-session",
        "--unshare-user",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-net",
        "--unshare-uts",
        "--disable-userns",
        "--assert-userns-disabled",
        "--cap-drop",
        "ALL",
        "--dir",
        "/candidate",
        "--dir",
        "/candidate/input",
        "--ro-bind",
    ] {
        bubblewrap_args.push(OsString::from(item));
    }
    bubblewrap_args.push(candidate_input.as_os_str().to_owned());
    bubblewrap_args.push(OsString::from("/candidate/input"));
    for item in ["--dir", "/candidate/output", "--bind"] {
        bubblewrap_args.push(OsString::from(item));
    }
    bubblewrap_args.push(candidate_output.as_os_str().to_owned());
    bubblewrap_args.push(OsString::from("/candidate/output"));
    for item in ["--dir", "/runtime", "--ro-bind"] {
        bubblewrap_args.push(OsString::from(item));
    }
    bubblewrap_args.push(runtime_root.as_os_str().to_owned());
    let tmpfs_size = limits.tmpfs_bytes.to_string();
    for item in [
        "/runtime",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--size",
        tmpfs_size.as_str(),
        "--tmpfs",
        "/tmp",
        "--chdir",
        "/candidate/input",
        "--clearenv",
        "--setenv",
        "HOME",
        "/tmp",
        "--setenv",
        "PWD",
        "/candidate/input",
        "--setenv",
        "TZ",
        "UTC",
        "--setenv",
        "TMPDIR",
        "/tmp",
        "--setenv",
        "PATH",
        "/runtime/bin",
        "--setenv",
        "LANG",
        "C",
        "--",
        executable_in_sandbox,
    ] {
        bubblewrap_args.push(OsString::from(item));
    }
    bubblewrap_args.extend(encoded_arguments);

    let mut prlimit_args = vec![
        OsString::from(format!("--as={}", limits.address_space_bytes)),
        OsString::from(format!("--cpu={}", limits.cpu_seconds)),
        OsString::from(format!("--nproc={}", limits.max_processes)),
        OsString::from(format!("--nofile={}", limits.max_open_files)),
        OsString::from(format!("--fsize={}", limits.max_file_bytes)),
        OsString::from("--core=0"),
        OsString::from("--"),
        bubblewrap.as_os_str().to_owned(),
    ];
    prlimit_args.extend(bubblewrap_args);

    Ok(SandboxCommand {
        program: prlimit,
        args: prlimit_args,
        candidate_input_root: candidate_input,
        runtime_bundle_root: runtime_root,
        candidate_output_root: candidate_output,
        evaluator_secret_root: evaluator_secret,
        candidate_executable_in_sandbox: PathBuf::from(executable_in_sandbox),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempTree {
        root: PathBuf,
    }

    impl TempTree {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("aerb-sandbox-test-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&root).expect("temp root");
            Self { root }
        }

        fn dir(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::create_dir_all(&path).expect("directory");
            path
        }

        fn executable(&self, path: &Path) {
            fs::write(path, b"test executable").expect("executable file");
            let mut permissions = fs::metadata(path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("executable permissions");
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn fixture() -> (TempTree, SandboxPaths, SandboxLimits) {
        let tree = TempTree::new();
        let host = tree.dir("host-tools");
        let bubblewrap = host.join("bwrap");
        let prlimit = host.join("prlimit");
        tree.executable(&bubblewrap);
        tree.executable(&prlimit);

        let input = tree.dir("candidate-input");
        fs::write(input.join("episode.json"), b"{}").expect("episode");
        let runtime = tree.dir("runtime");
        tree.dir("runtime/bin");
        let executable = runtime.join("bin/candidate");
        tree.executable(&executable);
        let output = tree.dir("candidate-output");
        let secret = tree.dir("evaluator-secret");

        (
            tree,
            SandboxPaths {
                bubblewrap,
                prlimit,
                candidate_input_root: input,
                runtime_bundle_root: runtime,
                candidate_output_root: output,
                evaluator_secret_root: secret,
                candidate_executable: executable,
            },
            SandboxLimits {
                address_space_bytes: 1_073_741_824,
                cpu_seconds: 60,
                max_processes: 32,
                max_open_files: 128,
                max_file_bytes: 16_777_216,
                tmpfs_bytes: 67_108_864,
            },
        )
    }

    fn strings(command: &SandboxCommand) -> Vec<String> {
        command
            .args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn mounts_only_candidate_input_runtime_and_candidate_output() {
        let (_tree, paths, limits) = fixture();
        let command = build_candidate_sandbox_command(&paths, limits, &[]).expect("command");
        let args = strings(&command);
        let input = command.candidate_input_root.to_string_lossy().into_owned();
        let runtime = command.runtime_bundle_root.to_string_lossy().into_owned();
        let output = command.candidate_output_root.to_string_lossy().into_owned();
        assert!(
            args.windows(3)
                .any(|w| w[0] == "--ro-bind" && w[1] == input && w[2] == "/candidate/input")
        );
        assert!(
            args.windows(3)
                .any(|w| w[0] == "--ro-bind" && w[1] == runtime && w[2] == "/runtime")
        );
        assert!(
            args.windows(3)
                .any(|w| w[0] == "--bind" && w[1] == output && w[2] == "/candidate/output")
        );
        assert!(
            !args
                .iter()
                .any(|arg| arg == command.evaluator_secret_root.to_string_lossy().as_ref())
        );
        assert!(!args.iter().any(|arg| arg == "/evaluator"));
    }

    #[test]
    fn network_and_host_capabilities_are_explicitly_removed() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        for namespace in [
            "--unshare-user",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-net",
            "--unshare-uts",
        ] {
            assert!(args.contains(&namespace.to_owned()), "missing {namespace}");
        }
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--cap-drop" && w[1] == "ALL")
        );
        assert!(args.contains(&"--clearenv".to_owned()));
    }

    #[test]
    fn candidate_output_is_the_only_writable_host_mount() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        assert!(
            args.windows(3)
                .any(|w| w[0] == "--bind" && w[2] == "/candidate/output")
        );
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "--bind").count(),
            1
        );
    }

    #[test]
    fn resource_limits_are_forwarded_to_prlimit() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        for expected in [
            "--as=1073741824",
            "--cpu=60",
            "--nproc=32",
            "--nofile=128",
            "--fsize=16777216",
            "--core=0",
        ] {
            assert!(args.contains(&expected.to_owned()), "missing {expected}");
        }
    }

    #[test]
    fn candidate_executable_must_be_inside_runtime_bundle() {
        let (tree, mut paths, limits) = fixture();
        let external = tree.root.join("outside-candidate");
        tree.executable(&external);
        paths.candidate_executable = external;
        assert_eq!(
            build_candidate_sandbox_command(&paths, limits, &[]),
            Err(SandboxError::ExecutableOutsideRuntime)
        );
    }

    #[test]
    fn evaluator_root_overlap_is_rejected() {
        let (_tree, mut paths, limits) = fixture();
        paths.evaluator_secret_root = paths.candidate_input_root.join("evaluator");
        fs::create_dir_all(&paths.evaluator_secret_root).expect("overlapping secret");
        assert!(matches!(
            build_candidate_sandbox_command(&paths, limits, &[]),
            Err(SandboxError::PathOverlap(_))
        ));
    }

    #[test]
    fn candidate_roots_must_be_pairwise_disjoint() {
        let (_tree, mut paths, limits) = fixture();
        paths.candidate_output_root = paths.candidate_input_root.join("output");
        fs::create_dir_all(&paths.candidate_output_root).expect("nested output");
        assert!(matches!(
            build_candidate_sandbox_command(&paths, limits, &[]),
            Err(SandboxError::PathOverlap(_))
        ));
    }

    #[test]
    fn typed_launch_arguments_accept_only_blind_ids_and_safe_artifact_leaves() {
        let (_tree, paths, limits) = fixture();
        let args = vec![
            CandidateSandboxArgument::CampaignId(
                "camp_0123456789abcdef0123456789abcdef".to_owned(),
            ),
            CandidateSandboxArgument::EpisodeId(
                "ep_0123456789abcdef0123456789abcdef".to_owned(),
            ),
            CandidateSandboxArgument::CandidateInputArtifact("episode.json".to_owned()),
            CandidateSandboxArgument::CandidateOutputArtifact("submission.json".to_owned()),
        ];
        let command = build_candidate_sandbox_command(&paths, limits, &args).expect("typed argv");
        let encoded = strings(&command);
        assert!(encoded.iter().any(|arg| arg == "--campaign-id=camp_0123456789abcdef0123456789abcdef"));
        assert!(encoded.iter().any(|arg| arg == "--episode-id=ep_0123456789abcdef0123456789abcdef"));
        assert!(encoded.iter().any(|arg| arg == "--input=/candidate/input/episode.json"));
        assert!(encoded.iter().any(|arg| arg == "--output=/candidate/output/submission.json"));
    }

    #[test]
    fn typed_launch_arguments_reject_free_form_paths_and_unrecognized_labels() {
        let invalid = [
            CandidateSandboxArgument::EpisodeId("applied_once_ack_lost".to_owned()),
            CandidateSandboxArgument::CampaignId("secret-world-7".to_owned()),
            CandidateSandboxArgument::CandidateInputArtifact("/etc/passwd".to_owned()),
            CandidateSandboxArgument::CandidateInputArtifact("../world-pack.json".to_owned()),
            CandidateSandboxArgument::CandidateOutputArtifact("nested/result.json".to_owned()),
            CandidateSandboxArgument::CandidateOutputArtifact("hidden..json".to_owned()),
            CandidateSandboxArgument::HostPidNamespace("host-pid-namespace-secret".to_owned()),
            CandidateSandboxArgument::NetworkProbePort(0),
        ];
        for argument in invalid {
            assert_eq!(
                argument.to_argv(),
                Err(SandboxError::InvalidCandidateArgument),
                "argument {argument:?}"
            );
        }
    }

    #[test]
    fn canary_only_typed_arguments_emit_expected_probe_flags() {
        let (_tree, paths, limits) = fixture();
        let args = [
            CandidateSandboxArgument::CanaryMode,
            CandidateSandboxArgument::NetworkProbePort(43123),
            CandidateSandboxArgument::HostPidNamespace("pid:[4026531836]".to_owned()),
        ];
        let command = build_candidate_sandbox_command(&paths, limits, &args).expect("canary argv");
        let encoded = strings(&command);
        assert!(encoded.contains(&"--candidate".to_owned()));
        assert!(encoded.contains(&"--network-port=43123".to_owned()));
        assert!(encoded.contains(&"--host-pid-ns=pid:[4026531836]".to_owned()));
    }

    #[test]
    fn zero_resource_limit_is_rejected() {
        let (_tree, paths, mut limits) = fixture();
        limits.cpu_seconds = 0;
        assert_eq!(
            build_candidate_sandbox_command(&paths, limits, &[]),
            Err(SandboxError::ZeroResourceLimit)
        );
    }

    #[test]
    fn candidate_executable_is_relocated_into_runtime_mount() {
        let (_tree, paths, limits) = fixture();
        let command = build_candidate_sandbox_command(&paths, limits, &[]).expect("command");
        assert_eq!(
            command.candidate_executable_in_sandbox,
            PathBuf::from("/runtime/bin/candidate")
        );
        assert!(strings(&command).contains(&"/runtime/bin/candidate".to_owned()));
    }

    #[test]
    fn private_tmpfs_is_explicitly_size_bounded() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        let size = args
            .iter()
            .position(|arg| arg == "--size")
            .expect("size option");
        assert_eq!(args[size + 1], "67108864");
        assert_eq!(args[size + 2], "--tmpfs");
        assert_eq!(args[size + 3], "/tmp");
    }

    #[test]
    fn nested_user_namespaces_must_be_disabled_not_best_effort() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        assert!(args.contains(&"--disable-userns".to_owned()));
        assert!(args.contains(&"--assert-userns-disabled".to_owned()));
    }

    #[test]
    fn zero_tmpfs_limit_is_rejected() {
        let (_tree, paths, mut limits) = fixture();
        limits.tmpfs_bytes = 0;
        assert_eq!(
            build_candidate_sandbox_command(&paths, limits, &[]),
            Err(SandboxError::ZeroResourceLimit)
        );
    }
}
