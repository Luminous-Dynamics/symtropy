// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Conservative reference policy for end-to-end blind-campaign conformance.
//! It reads only the mounted candidate manifest/episodes and writes outputs
//! only to /candidate/output. It is a protocol test baseline, not an SOTA agent.

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    process::ExitCode,
};

use mobility_rrvr_aerb_evaluator::{
    BENCHMARK, CandidateEpisode, CandidateSubmission, SCHEMA_VERSION,
    canonical_sorted_json_sha256, sha256_hex,
    blind_assignment_v0_1::CandidateBlindManifest,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

const INPUT_ROOT: &str = "/candidate/input";
const OUTPUT_ROOT: &str = "/candidate/output";
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const WORLD_IDS: [&str; 8] = [
    "not_dispatched",
    "dispatched_no_effect",
    "applied_once_ack_lost",
    "partial_effect",
    "duplicate_effect",
    "stale_authority_effect",
    "unresolved_command_holder",
    "open_world_residual",
];

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<(T, String), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("INPUT_UNAVAILABLE {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("INPUT_NOT_REGULAR_FILE: {}", path.display()));
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(format!("INPUT_EXCEEDS_LIMIT: {}", path.display()));
    }
    let bytes = fs::read(path).map_err(|error| format!("INPUT_READ_FAILED: {error}"))?;
    let raw = String::from_utf8(bytes).map_err(|error| format!("INPUT_NOT_UTF8: {error}"))?;
    let parsed = serde_json::from_str(&raw).map_err(|error| format!("INPUT_JSON_INVALID: {error}"))?;
    Ok((parsed, raw))
}

fn write_output<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("OUTPUT_SERIALIZATION_FAILED: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("OUTPUT_EXCEEDS_LIMIT".to_owned());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("OUTPUT_CREATE_FAILED: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("OUTPUT_WRITE_FAILED: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("OUTPUT_FSYNC_FAILED: {error}"))?;
    Ok(())
}

fn valid_opaque_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn reference_submission(episode: &CandidateEpisode, episode_json: &str) -> Result<CandidateSubmission, String> {
    if episode.schema_version != SCHEMA_VERSION || episode.benchmark != BENCHMARK {
        return Err("CANDIDATE_EPISODE_PROFILE_MISMATCH".to_owned());
    }
    let snapshot = canonical_sorted_json_sha256(episode_json)
        .map_err(|error| format!("SNAPSHOT_DIGEST_FAILED: {error}"))?;
    Ok(CandidateSubmission {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK.to_owned(),
        episode_id: episode.episode_id.clone(),
        candidate_snapshot_sha256: snapshot,
        model_sha256: sha256_hex(b"aerb-conservative-reference-policy-v0.2"),
        observer_profile_sha256: sha256_hex(b"aerb-reference-observer-profile-v0.2"),
        possible_world_support_ids: WORLD_IDS.iter().map(|world| (*world).to_owned()).collect(),
        disposition: if episode.observations.iter().any(|observation| {
            observation.quality != mobility_rrvr_aerb_evaluator::ObservationQuality::Admitted
        }) {
            mobility_rrvr_aerb_evaluator::Disposition::Abstain
        } else {
            mobility_rrvr_aerb_evaluator::Disposition::HoldReconcile
        },
        requested_probe_id: None,
        reason_codes: vec![
            "REFERENCE_POLICY_NOT_A_LEARNED_AGENT".to_owned(),
            "PRESERVE_ALL_POSSIBLE_HISTORIES".to_owned(),
            "NO_NEW_PHYSICAL_EFFECT_AUTHORIZED".to_owned(),
        ],
        explanation_observation_ids: episode
            .observations
            .iter()
            .map(|observation| observation.observation_id.clone())
            .collect(),
    })
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 || args[0] != "--candidate" {
        return Err("REFERENCE_CANDIDATE_REQUIRES_THE_TYPED_SANDBOX_ARGUMENT".to_owned());
    }

    let input_root = Path::new(INPUT_ROOT);
    let output_root = Path::new(OUTPUT_ROOT);
    if !input_root.is_dir() || !output_root.is_dir() {
        return Err("SANDBOX_INPUT_OR_OUTPUT_MOUNT_MISSING".to_owned());
    }
    if fs::read_dir(output_root)
        .map_err(|error| format!("OUTPUT_DIRECTORY_READ_FAILED: {error}"))?
        .next()
        .is_some()
    {
        return Err("CANDIDATE_OUTPUT_ROOT_MUST_START_EMPTY".to_owned());
    }

    let (manifest, _) =
        read_json::<CandidateBlindManifest>(&input_root.join("candidate_manifest.json"))?;
    if manifest.schema_version != "aerb-blind-campaign-v0.1"
        || manifest.corpus_class != "blind_evaluation"
        || !valid_opaque_id(&manifest.campaign_id, "camp_")
        || manifest.episode_ids.is_empty()
    {
        return Err("CANDIDATE_MANIFEST_PROFILE_INVALID".to_owned());
    }
    let unique_ids = manifest.episode_ids.iter().collect::<BTreeSet<_>>();
    if unique_ids.len() != manifest.episode_ids.len()
        || manifest.episode_ids.iter().any(|id| !valid_opaque_id(id, "ep_"))
    {
        return Err("CANDIDATE_MANIFEST_EPISODE_IDS_INVALID".to_owned());
    }

    for episode_id in &manifest.episode_ids {
        let episode_path = input_root.join(format!("episode-{episode_id}.json"));
        let (episode, raw) = read_json::<CandidateEpisode>(&episode_path)?;
        if episode.episode_id != *episode_id {
            return Err("CANDIDATE_EPISODE_ID_MISMATCH".to_owned());
        }
        let submission = reference_submission(&episode, &raw)?;
        let output_path = output_root.join(format!("submission-{episode_id}.json"));
        write_output(&output_path, &submission)?;
    }

    fs::File::open(output_root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("OUTPUT_DIRECTORY_FSYNC_FAILED: {error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_REFERENCE_CANDIDATE_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
