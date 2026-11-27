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
    let canonical = path.canonicalize().map_err(|_| SandboxError::PathResolution(name))?;
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
    let canonical = path.canonicalize().map_err(|_| SandboxError::PathResolution(name))?;
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

fn is_allowed_sandbox_absolute(path: &Path) -> bool {
    [
        "/candidate/input",
        "/candidate/output",
        "/runtime",
        "/tmp",
    ]
    .iter()
    .any(|prefix| path == Path::new(prefix) || path.starts_with(prefix))
}

fn validate_candidate_argument(argument: &OsString) -> Result<(), SandboxError> {
    let Some(value) = argument.to_str() else {
        return Err(SandboxError::InvalidCandidateArgument);
    };
    if value.contains('\0') {
        return Err(SandboxError::InvalidCandidateArgument);
    }

    for segment in value.split(['=', ':', ',']) {
        let path = Path::new(segment);
        if path.components().any(|component| {
            matches!(component, Component::ParentDir | Component::Prefix(_))
        }) {
            return Err(SandboxError::InvalidCandidateArgument);
        }
        if path.is_absolute() && !is_allowed_sandbox_absolute(path) {
            return Err(SandboxError::InvalidCandidateArgument);
        }
    }

    // Catch compact options such as -I/usr/lib while allowing subsequent
    // slashes inside already-approved sandbox absolute paths.
    for (index, byte) in value.bytes().enumerate() {
        if byte == b'/' && index > 0 {
            let prefix = &value[..index];
            if !prefix.contains('/')
                && prefix.starts_with('-')
                && !prefix.contains('=')
                && !prefix.contains(':')
                && !prefix.contains(',')
            {
                let embedded_path = Path::new(&value[index..]);
                if !is_allowed_sandbox_absolute(embedded_path) {
                    return Err(SandboxError::InvalidCandidateArgument);
                }
            }
        }
    }
    Ok(())
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
    candidate_arguments: &[OsString],
) -> Result<SandboxCommand, SandboxError> {
    limits.validate()?;

    let bubblewrap = canonical_executable(&paths.bubblewrap, "bubblewrap")?;
    let prlimit = canonical_executable(&paths.prlimit, "prlimit")?;
    let candidate_input = canonical_dir(&paths.candidate_input_root, "candidate_input_root")?;
    let runtime_root = canonical_dir(&paths.runtime_bundle_root, "runtime_bundle_root")?;
    let candidate_output = canonical_dir(&paths.candidate_output_root, "candidate_output_root")?;
    let evaluator_secret = canonical_dir(&paths.evaluator_secret_root, "evaluator_secret_root")?;
    let candidate_executable = canonical_executable(&paths.candidate_executable, "candidate_executable")?;

    let roots = [
        ("candidate input/runtime", &candidate_input, &runtime_root),
        ("candidate input/output", &candidate_input, &candidate_output),
        ("candidate input/evaluator", &candidate_input, &evaluator_secret),
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
        matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
    }) || !candidate_executable.starts_with(&runtime_root)
    {
        return Err(SandboxError::ExecutableOutsideRuntime);
    }
    for argument in candidate_arguments {
        validate_candidate_argument(argument)?;
    }

    let executable_in_sandbox = Path::new("/runtime").join(executable_relative);
    let executable_in_sandbox = executable_in_sandbox
        .to_str()
        .ok_or(SandboxError::NonUtf8Path("candidate executable in sandbox"))?;

    let mut bubblewrap_args: Vec<OsString> = Vec::new();
    for item in [
        "--die-with-parent",
        "--new-session",
        "--unshare-all",
        "--unshare-net",
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
    bubblewrap_args.extend(candidate_arguments.iter().cloned());

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
            let root = std::env::temp_dir().join(format!(
                "aerb-sandbox-test-{}-{nonce}",
                std::process::id()
            ));
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
        command.args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn mounts_only_candidate_input_runtime_and_candidate_output() {
        let (_tree, paths, limits) = fixture();
        let command = build_candidate_sandbox_command(&paths, limits, &[]).expect("command");
        let args = strings(&command);
        let input = command.candidate_input_root.to_string_lossy().into_owned();
        let runtime = command.runtime_bundle_root.to_string_lossy().into_owned();
        let output = command.candidate_output_root.to_string_lossy().into_owned();
        assert!(args.windows(3).any(|w| w[0] == "--ro-bind" && w[1] == input && w[2] == "/candidate/input"));
        assert!(args.windows(3).any(|w| w[0] == "--ro-bind" && w[1] == runtime && w[2] == "/runtime"));
        assert!(args.windows(3).any(|w| w[0] == "--bind" && w[1] == output && w[2] == "/candidate/output"));
        assert!(!args.iter().any(|arg| arg == command.evaluator_secret_root.to_string_lossy().as_ref()));
        assert!(!args.iter().any(|arg| arg == "/evaluator"));
    }

    #[test]
    fn network_and_host_capabilities_are_explicitly_removed() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        assert!(args.contains(&"--unshare-all".to_owned()));
        assert!(args.contains(&"--unshare-net".to_owned()));
        assert!(args.windows(2).any(|w| w[0] == "--cap-drop" && w[1] == "ALL"));
        assert!(args.contains(&"--clearenv".to_owned()));
    }

    #[test]
    fn candidate_output_is_the_only_writable_host_mount() {
        let (_tree, paths, limits) = fixture();
        let args = strings(&build_candidate_sandbox_command(&paths, limits, &[]).expect("command"));
        assert!(args.windows(3).any(|w| w[0] == "--bind" && w[2] == "/candidate/output"));
        assert_eq!(args.iter().filter(|arg| arg.as_str() == "--bind").count(), 1);
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
    fn absolute_host_paths_in_candidate_arguments_are_rejected() {
        let (_tree, paths, limits) = fixture();
        for argument in ["/etc/passwd", "--world-pack=/secrets/world.json", "-I/usr/lib"] {
            assert_eq!(
                build_candidate_sandbox_command(&paths, limits, &[OsString::from(argument)]),
                Err(SandboxError::InvalidCandidateArgument),
                "argument {argument}"
            );
        }
    }

    #[test]
    fn sandbox_relative_paths_and_ordinary_flags_are_allowed() {
        let (_tree, paths, limits) = fixture();
        let args = vec![
            OsString::from("--episode=/candidate/input/episode.json"),
            OsString::from("--output=/candidate/output/result.json"),
            OsString::from("--weights=weights/model.bin"),
        ];
        assert!(build_candidate_sandbox_command(&paths, limits, &args).is_ok());
    }

    #[test]
    fn parent_traversal_and_nul_arguments_are_rejected() {
        let (_tree, paths, limits) = fixture();
        for argument in ["../evaluator/world.json", "--input=../../secret", "x\0y"] {
            assert_eq!(
                build_candidate_sandbox_command(&paths, limits, &[OsString::from(argument)]),
                Err(SandboxError::InvalidCandidateArgument)
            );
        }
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
        let size = args.iter().position(|arg| arg == "--size").expect("size option");
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
