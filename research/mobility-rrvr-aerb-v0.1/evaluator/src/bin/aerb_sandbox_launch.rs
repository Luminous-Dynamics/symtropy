// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Host-side supervisor for the isolated reference-policy conformance run.
//! The evaluator root is never passed to the candidate command line or mounts.

use std::{
    collections::BTreeSet,
    env, fs,
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use mobility_rrvr_aerb_evaluator::{
    blind_assignment_v0_1::CandidateBlindManifest,
    blind_execution_sandbox_v0_1::{
        CandidateSandboxArgument, SandboxLimits, SandboxPaths, build_candidate_sandbox_command,
    },
    sha256_hex,
};
use serde::{Deserialize, Serialize};

const RUN_RECEIPT_SCHEMA: &str = "aerb-candidate-run-receipt-v0.2";
const WALL_TIMEOUT_SECONDS: u64 = 45;
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum RunnerOutcome {
    Completed { exit_code: i32 },
    ExitedNonzero { exit_code: i32 },
    TimedOutUnknown { exit_code: i32 },
    TerminatedWithoutExitCode,
    BuildRejected { reason: String },
    LaunchFailedUnknown { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutputArtifact {
    episode_id: String,
    relative_file_name: String,
    sha256: String,
    byte_length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CandidateRunReceipt {
    schema_version: String,
    campaign_id: String,
    candidate_manifest_sha256: String,
    candidate_executable_sha256: String,
    runtime_bundle_sha256: String,
    sandbox_profile_sha256: String,
    wall_timeout_seconds: u64,
    outcome: RunnerOutcome,
    output_artifacts: Vec<OutputArtifact>,
    missing_episode_ids: Vec<String>,
    unexpected_output_entries: Vec<String>,
}

fn read_regular_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("FILE_UNAVAILABLE {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("FILE_NOT_REGULAR: {}", path.display()));
    }
    if metadata.len() > max_bytes {
        return Err(format!("FILE_EXCEEDS_LIMIT: {}", path.display()));
    }
    fs::read(path).map_err(|error| format!("FILE_READ_FAILED {}: {error}", path.display()))
}

fn digest_tree(root: &Path) -> Result<String, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("RUNTIME_ROOT_UNAVAILABLE: {error}"))?;
    let mut entries = Vec::<(String, String, u64)>::new();
    let mut stack = vec![canonical_root.clone()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("RUNTIME_DIRECTORY_READ_FAILED: {error}"))?
        {
            let entry = entry.map_err(|error| format!("RUNTIME_ENTRY_READ_FAILED: {error}"))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("RUNTIME_METADATA_FAILED: {error}"))?;
            if metadata.file_type().is_symlink() {
                return Err(format!("RUNTIME_SYMLINK_FORBIDDEN: {}", path.display()));
            }
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() || metadata.len() > MAX_ARTIFACT_BYTES {
                return Err(format!("RUNTIME_ENTRY_NOT_REGULAR_OR_TOO_LARGE: {}", path.display()));
            }
            let relative = path
                .strip_prefix(&canonical_root)
                .map_err(|_| "RUNTIME_PATH_ESCAPES_ROOT".to_owned())?
                .to_str()
                .ok_or_else(|| "RUNTIME_PATH_NOT_UTF8".to_owned())?
                .replace('\', "/");
            let bytes = fs::read(&path)
                .map_err(|error| format!("RUNTIME_FILE_READ_FAILED: {error}"))?;
            entries.push((relative, sha256_hex(&bytes), metadata.len()));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    if entries.is_empty() {
        return Err("RUNTIME_BUNDLE_EMPTY".to_owned());
    }
    let serialized = serde_json::to_vec(&entries)
        .map_err(|error| format!("RUNTIME_INVENTORY_SERIALIZATION_FAILED: {error}"))?;
    Ok(sha256_hex(&serialized))
}

fn expected_episode_ids(manifest: &CandidateBlindManifest) -> Result<BTreeSet<String>, String> {
    if manifest.schema_version != "aerb-blind-campaign-v0.1"
        || manifest.corpus_class != "blind_evaluation"
        || manifest.campaign_id.len() != 37
        || !manifest.campaign_id.starts_with("camp_")
        || manifest.episode_ids.is_empty()
    {
        return Err("CANDIDATE_MANIFEST_PROFILE_INVALID".to_owned());
    }
    let ids = manifest.episode_ids.iter().cloned().collect::<BTreeSet<_>>();
    if ids.len() != manifest.episode_ids.len()
        || ids.iter().any(|value| {
            !value.starts_with("ep_")
                || value.len() != 35
                || !value[3..]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    {
        return Err("CANDIDATE_MANIFEST_EPISODE_IDS_INVALID".to_owned());
    }
    Ok(ids)
}

fn capture_outputs(
    output_root: &Path,
    expected_ids: &BTreeSet<String>,
) -> (Vec<OutputArtifact>, Vec<String>, Vec<String>) {
    let mut artifacts = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut unexpected = Vec::new();
    let mut missing = Vec::new();

    let entries = match fs::read_dir(output_root) {
        Ok(entries) => entries,
        Err(error) => {
            return (
                artifacts,
                expected_ids.iter().cloned().collect(),
                vec![format!("OUTPUT_DIRECTORY_READ_FAILED:{error}")],
            );
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                unexpected.push(format!("UNREADABLE_DIRECTORY_ENTRY:{error}"));
                continue;
            }
        };
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => {
                unexpected.push("NON_UTF8_OUTPUT_NAME".to_owned());
                continue;
            }
        };
        let Some(episode_id) = name
            .strip_prefix("submission-")
            .and_then(|value| value.strip_suffix(".json"))
        else {
            unexpected.push(name);
            continue;
        };
        if !expected_ids.contains(episode_id) || !seen_ids.insert(episode_id.to_owned()) {
            unexpected.push(name);
            continue;
        }
        match read_regular_file(&entry.path(), MAX_ARTIFACT_BYTES) {
            Ok(bytes) => artifacts.push(OutputArtifact {
                episode_id: episode_id.to_owned(),
                relative_file_name: name,
                sha256: sha256_hex(&bytes),
                byte_length: bytes.len() as u64,
            }),
            Err(error) => unexpected.push(format!("{name}:{error}")),
        }
    }
    for episode_id in expected_ids {
        if !seen_ids.contains(episode_id) {
            missing.push(episode_id.clone());
        }
    }
    artifacts.sort_by(|left, right| left.episode_id.cmp(&right.episode_id));
    missing.sort();
    unexpected.sort();
    (artifacts, missing, unexpected)
}

fn sandbox_profile_sha256() -> String {
    sha256_hex(
        b"AERB-REFERENCE-RUNNER-V0.2\0bubblewrap+prlimit\0user,pid,ipc,net,uts-unshare\0caps-drop-all\0input-ro\0runtime-ro\0output-rw\0evaluator-unmounted\0env-clear\0rlimit-as-cpu-nproc-nofile-fsize-core\0timeout-45s",
    )
}

fn write_receipt(path: &Path, receipt: &CandidateRunReceipt) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("RUN_RECEIPT_SERIALIZATION_FAILED: {error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("RUN_RECEIPT_CREATE_FAILED: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("RUN_RECEIPT_WRITE_FAILED: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("RUN_RECEIPT_FSYNC_FAILED: {error}"))?;
    fs::File::open(path.parent().ok_or_else(|| "RUN_RECEIPT_PARENT_MISSING".to_owned())?)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("RUN_RECEIPT_DIRECTORY_FSYNC_FAILED: {error}"))?;
    Ok(())
}

fn run() -> Result<RunnerOutcome, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(
            "usage: aerb_sandbox_launch <candidate-input> <runtime-root> <candidate-output> <evaluator-root> <candidate-executable>"
                .to_owned(),
        );
    }
    let input_root = PathBuf::from(&args[0]).canonicalize()
        .map_err(|error| format!("CANDIDATE_INPUT_ROOT_UNAVAILABLE: {error}"))?;
    let runtime_root = PathBuf::from(&args[1]).canonicalize()
        .map_err(|error| format!("RUNTIME_ROOT_UNAVAILABLE: {error}"))?;
    let output_root = PathBuf::from(&args[2]).canonicalize()
        .map_err(|error| format!("CANDIDATE_OUTPUT_ROOT_UNAVAILABLE: {error}"))?;
    let evaluator_root = PathBuf::from(&args[3]).canonicalize()
        .map_err(|error| format!("EVALUATOR_ROOT_UNAVAILABLE: {error}"))?;
    let candidate_executable = PathBuf::from(&args[4]).canonicalize()
        .map_err(|error| format!("CANDIDATE_EXECUTABLE_UNAVAILABLE: {error}"))?;

    let manifest_bytes = read_regular_file(&input_root.join("candidate_manifest.json"), 64 * 1024 * 1024)?;
    let manifest: CandidateBlindManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("CANDIDATE_MANIFEST_INVALID: {error}"))?;
    let episode_ids = expected_episode_ids(&manifest)?;
    if !fs::read_dir(&output_root)
        .map_err(|error| format!("OUTPUT_ROOT_READ_FAILED: {error}"))?
        .next()
        .is_none()
    {
        return Err("CANDIDATE_OUTPUT_ROOT_MUST_BE_EMPTY_BEFORE_EXECUTION".to_owned());
    }

    let candidate_executable_bytes =
        read_regular_file(&candidate_executable, MAX_ARTIFACT_BYTES)?;
    let candidate_executable_sha256 = sha256_hex(&candidate_executable_bytes);
    let runtime_bundle_sha256 = digest_tree(&runtime_root)?;
    let sandbox_profile_sha256 = sandbox_profile_sha256();
    let paths = SandboxPaths {
        bubblewrap: PathBuf::from("/usr/bin/bwrap"),
        prlimit: PathBuf::from("/usr/bin/prlimit"),
        candidate_input_root: input_root.clone(),
        runtime_bundle_root: runtime_root.clone(),
        candidate_output_root: output_root.clone(),
        evaluator_secret_root: evaluator_root.clone(),
        candidate_executable: candidate_executable.clone(),
    };
    let limits = SandboxLimits {
        address_space_bytes: 1_073_741_824,
        cpu_seconds: 15,
        max_processes: 32,
        max_open_files: 64,
        max_file_bytes: 16 * 1024 * 1024,
        tmpfs_bytes: 64 * 1024 * 1024,
    };

    let (outcome, args_for_profile) = match build_candidate_sandbox_command(
        &paths,
        limits,
        &[CandidateSandboxArgument::CanaryMode],
    ) {
        Ok(spec) => {
            let mut command = Command::new("/usr/bin/timeout");
            command
                .args([
                    "--signal=TERM",
                    "--kill-after=2s",
                    &format!("{WALL_TIMEOUT_SECONDS}s"),
                ])
                .arg(&spec.program)
                .args(&spec.args)
                .env_clear()
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let profile = serde_json::to_vec(&spec.args)
                .map_err(|error| format!("SANDBOX_ARGV_DIGEST_FAILED: {error}"))?;
            let result = match command.status() {
                Ok(status) if status.success() => RunnerOutcome::Completed {
                    exit_code: status.code().unwrap_or(0),
                },
                Ok(status) if status.code() == Some(124) || status.code() == Some(137) => {
                    RunnerOutcome::TimedOutUnknown {
                        exit_code: status.code().unwrap_or(124),
                    }
                }
                Ok(status) => match status.code() {
                    Some(code) => RunnerOutcome::ExitedNonzero { exit_code: code },
                    None => RunnerOutcome::TerminatedWithoutExitCode,
                },
                Err(error) => RunnerOutcome::LaunchFailedUnknown {
                    reason: error.to_string(),
                },
            };
            (result, profile)
        }
        Err(error) => (
            RunnerOutcome::BuildRejected {
                reason: error.to_string(),
            },
            Vec::new(),
        ),
    };

    let (output_artifacts, missing_episode_ids, unexpected_output_entries) =
        capture_outputs(&output_root, &episode_ids);
    let profile_binding = serde_json::json!({
        "profile": sandbox_profile_sha256,
        "argv_sha256": sha256_hex(&args_for_profile),
        "limits": {
            "address_space_bytes": limits.address_space_bytes,
            "cpu_seconds": limits.cpu_seconds,
            "max_processes": limits.max_processes,
            "max_open_files": limits.max_open_files,
            "max_file_bytes": limits.max_file_bytes,
            "tmpfs_bytes": limits.tmpfs_bytes
        }
    });
    let profile_sha256 = sha256_hex(
        &serde_json::to_vec(&profile_binding)
            .map_err(|error| format!("PROFILE_BINDING_SERIALIZATION_FAILED: {error}"))?,
    );
    let receipt = CandidateRunReceipt {
        schema_version: RUN_RECEIPT_SCHEMA.to_owned(),
        campaign_id: manifest.campaign_id,
        candidate_manifest_sha256: sha256_hex(&manifest_bytes),
        candidate_executable_sha256,
        runtime_bundle_sha256,
        sandbox_profile_sha256: profile_sha256,
        wall_timeout_seconds: WALL_TIMEOUT_SECONDS,
        outcome: outcome.clone(),
        output_artifacts,
        missing_episode_ids,
        unexpected_output_entries,
    };
    write_receipt(&evaluator_root.join("candidate-run-receipt.json"), &receipt)?;
    Ok(outcome)
}

fn main() -> ExitCode {
    match run() {
        Ok(RunnerOutcome::Completed { exit_code: 0 }) => ExitCode::SUCCESS,
        Ok(outcome) => {
            eprintln!("AERB_SANDBOX_RUN_NOT_QUALIFIED: {outcome:?}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("AERB_SANDBOX_LAUNCH_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
