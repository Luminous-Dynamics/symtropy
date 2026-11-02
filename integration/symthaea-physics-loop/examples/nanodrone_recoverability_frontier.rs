//! Intervention-conditioned recoverability frontier for counterfactual flight branches.
//!
//! Research-only evaluator: it consumes precomputed simulator branches. It does not
//! snapshot or restore a simulator, train a model, or certify physical flight safety.

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WILSON_Z_ONE_SIDED_95: f64 = 1.644_853_626_951_472_2;
const EPS: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CheckpointRow {
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    checkpoint_time_s: f64,
    remaining_budget_s: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BranchRow {
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    checkpoint_time_s: f64,
    remaining_budget_s: f64,
    branch_elapsed_s: f64,
    training_seed: u64,
    intervention_id: String,
    branch_seed: u64,
    state_snapshot_sha256: String,
    exogenous_tape_sha256: String,
    restore_verified: bool,
    status: String,
    mission_completed: bool,
    boundary_violation_count: u32,
    tracking_rmse_m: Option<f64>,
    energy_wh: Option<f64>,
    deadline_misses: u32,
}

#[derive(Debug)]
struct Args {
    input: PathBuf,
    checkpoints: PathBuf,
    train_families: PathBuf,
    artifact_root: PathBuf,
    dataset_receipt_sha256: String,
    model_manifest_sha256: String,
    source_commit: String,
    training_seeds: Vec<u64>,
    branch_seeds: Vec<u64>,
    interventions: Vec<String>,
    max_tracking_rmse_m: f64,
    recoverability_threshold: f64,
    bootstrap_replicates: usize,
    bootstrap_seed: u64,
    evidence_class: String,
    output: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
struct Interval {
    lower_95: Option<f64>,
    upper_95: Option<f64>,
    valid_replicates: usize,
}

#[derive(Clone, Debug, Serialize)]
struct CheckpointEstimate {
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    training_seed: u64,
    checkpoint_time_s: f64,
    remaining_budget_s: f64,
    intervention_id: String,
    branch_replicates: usize,
    successes: usize,
    success_rate: f64,
    wilson_95_one_sided_lower: f64,
    qualifies_recoverable: bool,
    boundary_violation_rate: f64,
    deadline_miss_rate: f64,
    mean_tracking_rmse_m_when_finite: Option<f64>,
    mean_energy_wh_when_finite: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct UnitHorizon {
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    training_seed: u64,
    intervention_id: String,
    latest_recoverable_checkpoint_s: f64,
}

#[derive(Clone, Debug, Serialize)]
struct InterventionSummary {
    intervention_id: String,
    reference_intervention_id: String,
    mean_recoverability_horizon_s: f64,
    recoverable_unit_rate: f64,
    mean_paired_horizon_delta_vs_continue_s: f64,
    paired_crossed_bootstrap_delta_ci95_s: Interval,
    note: String,
}

#[derive(Clone, Debug, Serialize)]
struct Report {
    schema_version: String,
    comparison_kind: String,
    evidence_class: String,
    claim_ceiling: String,
    source_commit: String,
    dataset_receipt_sha256: String,
    model_manifest_sha256: String,
    input_csv_sha256: String,
    branch_artifact_verification: BranchArtifactVerification,
    checkpoints_manifest_sha256: String,
    train_families_manifest_sha256: String,
    training_seeds: Vec<u64>,
    branch_seeds: Vec<u64>,
    intervention_ids: Vec<String>,
    recoverability_threshold: f64,
    recoverability_interval: String,
    budget_contract: String,
    horizon_bootstrap_scope: String,
    max_tracking_rmse_m: f64,
    bootstrap_replicates_requested: usize,
    bootstrap_seed: u64,
    registered_family_count: usize,
    registered_flight_count: usize,
    registered_checkpoint_count: usize,
    expected_branch_count: usize,
    checkpoint_estimates: Vec<CheckpointEstimate>,
    unit_horizons: Vec<UnitHorizon>,
    intervention_summaries: Vec<InterventionSummary>,
}


#[derive(Clone, Debug, Serialize)]
struct BranchArtifactVerification {
    status: String,
    evidence_class: String,
    snapshot_artifacts_verified: usize,
    disturbance_tape_artifacts_verified: usize,
    branch_rows_bound: usize,
    snapshot_manifest_sha256: String,
    disturbance_tape_manifest_sha256: String,
    combined_artifact_manifest_sha256: String,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StateSnapshotArtifact {
    schema_version: String,
    evidence_class: String,
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    checkpoint_time_s: f64,
    training_seed: u64,
    synthetic_state_payload: String,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DisturbanceTapeArtifact {
    schema_version: String,
    evidence_class: String,
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    checkpoint_time_s: f64,
    training_seed: u64,
    branch_seed: u64,
    synthetic_disturbance_payload: String,
    claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SnapshotArtifactIdentity {
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    checkpoint_time_bits: u64,
    training_seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TapeArtifactIdentity {
    family_id: String,
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    checkpoint_time_bits: u64,
    training_seed: u64,
    branch_seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CheckpointKey(String, String, String, u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UnitKey(String, String, String, u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SnapshotKey(String, String, String, u64, u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TapeKey(String, String, String, u64, u64, u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BranchKey(String, String, String, u64, u64, String, u64);

fn usage() -> &'static str {
    "Usage: nanodrone_recoverability_frontier --input branches.csv --checkpoints checkpoints.csv --train-families train.txt --artifact-root branch-artifacts-v0.1 --dataset-receipt-sha256 HEX64 --model-manifest-sha256 HEX64 --source-commit HEX40 --training-seeds 101,102 --branch-seeds 1,2,3,4 --interventions continue,robust_fallback --max-tracking-rmse-m 0.5 --recoverability-threshold 0.5 [--bootstrap 2000] [--seed 17] [--evidence-class synthetic|simulation|real-flight] [--output report.json]"
}

fn parse_csv_list<T: std::str::FromStr>(raw: &str, name: &str) -> Result<Vec<T>, String> {
    let values = raw.split(',')
        .map(|v| v.trim().parse::<T>().map_err(|_| format!("{name} contains invalid value {v:?}")))
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(values)
}

fn parse_args() -> Result<Args, String> {
    let mut values = HashMap::<String, String>::new();
    let mut iter = env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("{}", usage());
            std::process::exit(0);
        }
        if !key.starts_with("--") {
            return Err(format!("unexpected positional argument {key:?}\n{}", usage()));
        }
        let value = iter.next().ok_or_else(|| format!("missing value for {key}"))?;
        if values.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate argument {key}"));
        }
    }
    let mut take = |key: &str| values.remove(key).ok_or_else(|| format!("missing {key}\n{}", usage()));
    let input = PathBuf::from(take("--input")?);
    let checkpoints = PathBuf::from(take("--checkpoints")?);
    let train_families = PathBuf::from(take("--train-families")?);
    let artifact_root = PathBuf::from(take("--artifact-root")?);
    let dataset_receipt_sha256 = take("--dataset-receipt-sha256")?;
    let model_manifest_sha256 = take("--model-manifest-sha256")?;
    let source_commit = take("--source-commit")?;
    let training_seeds = parse_csv_list::<u64>(&take("--training-seeds")?, "--training-seeds")?;
    let branch_seeds = parse_csv_list::<u64>(&take("--branch-seeds")?, "--branch-seeds")?;
    let interventions = parse_csv_list::<String>(&take("--interventions")?, "--interventions")?;
    let max_tracking_rmse_m = take("--max-tracking-rmse-m")?
        .parse::<f64>().map_err(|_| "--max-tracking-rmse-m must be numeric".to_string())?;
    let recoverability_threshold = take("--recoverability-threshold")?
        .parse::<f64>().map_err(|_| "--recoverability-threshold must be numeric".to_string())?;
    let bootstrap_replicates = values.remove("--bootstrap")
        .map(|v| v.parse::<usize>().map_err(|_| "--bootstrap must be an integer".to_string()))
        .transpose()?.unwrap_or(2000);
    let bootstrap_seed = values.remove("--seed")
        .map(|v| v.parse::<u64>().map_err(|_| "--seed must be an integer".to_string()))
        .transpose()?.unwrap_or(17);
    let evidence_class = values.remove("--evidence-class").unwrap_or_else(|| "synthetic".to_string());
    let output = values.remove("--output").map(PathBuf::from);
    if !values.is_empty() {
        return Err(format!("unknown arguments: {:?}", values.keys().collect::<Vec<_>>()));
    }
    let args = Args {
        input, checkpoints, train_families, artifact_root, dataset_receipt_sha256, model_manifest_sha256,
        source_commit, training_seeds, branch_seeds, interventions, max_tracking_rmse_m,
        recoverability_threshold, bootstrap_replicates, bootstrap_seed, evidence_class, output,
    };
    validate_args(&args)?;
    Ok(args)
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_args(args: &Args) -> Result<(), String> {
    for (name, value, len) in [
        ("dataset receipt", &args.dataset_receipt_sha256, 64usize),
        ("model manifest", &args.model_manifest_sha256, 64usize),
        ("source commit", &args.source_commit, 40usize),
    ] {
        if !valid_hex(value, len) {
            return Err(format!("{name} must be a {len}-character hexadecimal digest"));
        }
    }
    if !args.max_tracking_rmse_m.is_finite() || args.max_tracking_rmse_m <= 0.0 {
        return Err("maximum tracking RMSE must be positive and finite".into());
    }
    if !args.recoverability_threshold.is_finite() || !(0.0..1.0).contains(&args.recoverability_threshold) {
        return Err("recoverability threshold must be finite and in (0,1)".into());
    }
    if args.bootstrap_replicates < 1 {
        return Err("bootstrap replicate count must be positive".into());
    }
    if args.training_seeds.len() < 2 || args.branch_seeds.len() < 2 {
        return Err("at least two distinct training seeds and two branch seeds are required".into());
    }
    let unique_training = args.training_seeds.iter().collect::<BTreeSet<_>>();
    let unique_branches = args.branch_seeds.iter().collect::<BTreeSet<_>>();
    let unique_interventions = args.interventions.iter().collect::<BTreeSet<_>>();
    if unique_training.len() != args.training_seeds.len() {
        return Err("training seed list contains duplicates".into());
    }
    if unique_branches.len() != args.branch_seeds.len() {
        return Err("branch seed list contains duplicates".into());
    }
    if unique_interventions.len() != args.interventions.len() || args.interventions.iter().any(|i| i.trim().is_empty()) {
        return Err("intervention list contains duplicates or empty names".into());
    }
    if !args.interventions.iter().any(|i| i == "continue") || args.interventions.len() < 2 {
        return Err("interventions must include 'continue' plus at least one response".into());
    }
    if !matches!(args.evidence_class.as_str(), "synthetic" | "simulation" | "real-flight") {
        return Err("evidence class must be synthetic, simulation, or real-flight".into());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}


fn require_real_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!("{label} must be a real non-symlink directory: {}", path.display()));
    }
    Ok(())
}

fn expected_artifact_names(digests: &BTreeSet<String>) -> BTreeSet<String> {
    digests.iter().map(|digest| format!("{digest}.json")).collect()
}

fn verify_exact_artifact_inventory(
    directory: &Path,
    label: &str,
    expected_names: &BTreeSet<String>,
) -> Result<(), String> {
    require_real_directory(directory, label)?;
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot read {label} {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read {label} directory entry: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            return Err(format!("{label} may contain only regular non-symlink files: {}", path.display()));
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !actual.insert(name.clone()) {
            return Err(format!("duplicate {label} filename {name:?}"));
        }
    }
    let missing = expected_names.difference(&actual).cloned().collect::<Vec<_>>();
    let extra = actual.difference(expected_names).cloned().collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(format!(
            "{label} inventory mismatch: missing={missing:?}, unreferenced={extra:?}"
        ));
    }
    Ok(())
}

fn snapshot_identity(row: &BranchRow) -> SnapshotArtifactIdentity {
    SnapshotArtifactIdentity {
        family_id: row.family_id.clone(),
        flight_id: row.flight_id.clone(),
        rollout_start_id: row.rollout_start_id.clone(),
        regime: row.regime.clone(),
        checkpoint_time_bits: row.checkpoint_time_s.to_bits(),
        training_seed: row.training_seed,
    }
}

fn tape_identity(row: &BranchRow) -> TapeArtifactIdentity {
    TapeArtifactIdentity {
        family_id: row.family_id.clone(),
        flight_id: row.flight_id.clone(),
        rollout_start_id: row.rollout_start_id.clone(),
        regime: row.regime.clone(),
        checkpoint_time_bits: row.checkpoint_time_s.to_bits(),
        training_seed: row.training_seed,
        branch_seed: row.branch_seed,
    }
}

fn verify_branch_artifacts(
    root: &Path,
    branches: &[BranchRow],
    requested_evidence_class: &str,
) -> Result<BranchArtifactVerification, String> {
    if branches.is_empty() {
        return Err("cannot verify branch artifacts for an empty branch set".into());
    }
    let artifact_evidence_class = match requested_evidence_class {
        "synthetic" => "synthetic_fixture",
        "simulation" | "real-flight" => {
            return Err(
                "branch artifact schema v0.1 is synthetic-fixture-only; simulation and real-flight require producer-attested artifact schemas".into()
            );
        }
        other => return Err(format!("unsupported artifact evidence class {other:?}")),
    };

    require_real_directory(root, "branch artifact root")?;
    let snapshot_dir = root.join("snapshots");
    let tape_dir = root.join("disturbance-tapes");
    require_real_directory(&snapshot_dir, "snapshot artifact directory")?;
    require_real_directory(&tape_dir, "disturbance tape artifact directory")?;

    // The root is an explicitly versioned bundle: exactly two artifact
    // directories and one human-readable README. No symlinks or extra roots.
    let root_entries = fs::read_dir(root)
        .map_err(|error| format!("cannot read branch artifact root: {error}"))?
        .map(|entry| entry.map(|value| value.file_name().to_string_lossy().into_owned()))
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|error| format!("cannot read branch artifact root entry: {error}"))?;
    let expected_root_entries = BTreeSet::from([
        "README.md".to_string(),
        "snapshots".to_string(),
        "disturbance-tapes".to_string(),
    ]);
    if root_entries != expected_root_entries {
        return Err(format!(
            "branch artifact root inventory mismatch: found={root_entries:?}, expected={expected_root_entries:?}"
        ));
    }
    let readme_path = root.join("README.md");
    let readme_metadata = fs::symlink_metadata(&readme_path)
        .map_err(|error| format!("cannot inspect branch artifact README: {error}"))?;
    if readme_metadata.file_type().is_symlink() || !readme_metadata.file_type().is_file() {
        return Err("branch artifact README must be a regular non-symlink file".into());
    }

    let mut expected_snapshots = BTreeMap::<String, SnapshotArtifactIdentity>::new();
    let mut expected_tapes = BTreeMap::<String, TapeArtifactIdentity>::new();
    for row in branches {
        if !valid_hex(&row.state_snapshot_sha256, 64) || !valid_hex(&row.exogenous_tape_sha256, 64) {
            return Err("branch rows must reference lowercase or uppercase 64-character SHA-256 digests".into());
        }
        let snapshot_digest = row.state_snapshot_sha256.to_ascii_lowercase();
        let snapshot_key = snapshot_identity(row);
        if let Some(previous) = expected_snapshots.insert(snapshot_digest.clone(), snapshot_key.clone()) {
            if previous != snapshot_key {
                return Err(format!("one snapshot digest is claimed by multiple snapshot identities: {snapshot_digest}"));
            }
        }
        let tape_digest = row.exogenous_tape_sha256.to_ascii_lowercase();
        let tape_key = tape_identity(row);
        if let Some(previous) = expected_tapes.insert(tape_digest.clone(), tape_key.clone()) {
            if previous != tape_key {
                return Err(format!("one disturbance-tape digest is claimed by multiple tape identities: {tape_digest}"));
            }
        }
    }

    let snapshot_digests = expected_snapshots.keys().cloned().collect::<BTreeSet<_>>();
    let tape_digests = expected_tapes.keys().cloned().collect::<BTreeSet<_>>();
    verify_exact_artifact_inventory(
        &snapshot_dir,
        "snapshot artifact directory",
        &expected_artifact_names(&snapshot_digests),
    )?;
    verify_exact_artifact_inventory(
        &tape_dir,
        "disturbance tape artifact directory",
        &expected_artifact_names(&tape_digests),
    )?;

    for (digest, expected_identity) in &expected_snapshots {
        let path = snapshot_dir.join(format!("{digest}.json"));
        let bytes = fs::read(&path).map_err(|error| format!("cannot read snapshot artifact {}: {error}", path.display()))?;
        if sha256(&bytes).as_str() != digest {
            return Err(format!("snapshot artifact SHA-256 does not match its filename: {}", path.display()));
        }
        let artifact: StateSnapshotArtifact = serde_json::from_slice(&bytes)
            .map_err(|error| format!("snapshot artifact {} is invalid JSON/schema: {error}", path.display()))?;
        let observed_identity = SnapshotArtifactIdentity {
            family_id: artifact.family_id,
            flight_id: artifact.flight_id,
            rollout_start_id: artifact.rollout_start_id,
            regime: artifact.regime,
            checkpoint_time_bits: artifact.checkpoint_time_s.to_bits(),
            training_seed: artifact.training_seed,
        };
        if artifact.schema_version != "symtropy-nanodrone-state-snapshot-v0.1"
            || artifact.evidence_class != artifact_evidence_class
            || !artifact.synthetic_state_payload.starts_with("SYNTHETIC_STATE_ONLY|")
            || artifact.claim_ceiling != "fixture_byte_integrity_only_no_simulator_origin_or_restore_attestation"
            || &observed_identity != expected_identity
        {
            return Err(format!("snapshot artifact metadata/identity mismatch: {}", path.display()));
        }
    }

    for (digest, expected_identity) in &expected_tapes {
        let path = tape_dir.join(format!("{digest}.json"));
        let bytes = fs::read(&path).map_err(|error| format!("cannot read disturbance tape artifact {}: {error}", path.display()))?;
        if sha256(&bytes).as_str() != digest {
            return Err(format!("disturbance tape artifact SHA-256 does not match its filename: {}", path.display()));
        }
        let artifact: DisturbanceTapeArtifact = serde_json::from_slice(&bytes)
            .map_err(|error| format!("disturbance tape artifact {} is invalid JSON/schema: {error}", path.display()))?;
        let observed_identity = TapeArtifactIdentity {
            family_id: artifact.family_id,
            flight_id: artifact.flight_id,
            rollout_start_id: artifact.rollout_start_id,
            regime: artifact.regime,
            checkpoint_time_bits: artifact.checkpoint_time_s.to_bits(),
            training_seed: artifact.training_seed,
            branch_seed: artifact.branch_seed,
        };
        if artifact.schema_version != "symtropy-nanodrone-exogenous-tape-v0.1"
            || artifact.evidence_class != artifact_evidence_class
            || !artifact.synthetic_disturbance_payload.starts_with("SYNTHETIC_TAPE_ONLY|")
            || artifact.claim_ceiling != "fixture_byte_integrity_only_no_simulator_origin_or_rng_state_attestation"
            || &observed_identity != expected_identity
        {
            return Err(format!("disturbance tape artifact metadata/identity mismatch: {}", path.display()));
        }
    }

    let snapshot_manifest = snapshot_digests.iter().cloned().collect::<Vec<_>>().join("\n");
    let tape_manifest = tape_digests.iter().cloned().collect::<Vec<_>>().join("\n");
    let combined_manifest = format!("snapshots\n{snapshot_manifest}\ntapes\n{tape_manifest}\n");
    Ok(BranchArtifactVerification {
        status: if artifact_evidence_class == "synthetic_fixture" {
            "synthetic_fixture_content_verified_origin_unattested".into()
        } else {
            "content_verified_origin_unattested".into()
        },
        evidence_class: artifact_evidence_class.into(),
        snapshot_artifacts_verified: expected_snapshots.len(),
        disturbance_tape_artifacts_verified: expected_tapes.len(),
        branch_rows_bound: branches.len(),
        snapshot_manifest_sha256: sha256(snapshot_manifest.as_bytes()),
        disturbance_tape_manifest_sha256: sha256(tape_manifest.as_bytes()),
        combined_artifact_manifest_sha256: sha256(combined_manifest.as_bytes()),
        claim_ceiling: "sha256_content_integrity_and_identity_only; no independent simulator state capture, restore fidelity, RNG source, or hardware origin attestation".into(),
    })
}

fn read_csv<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<(Vec<T>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(bytes.as_slice());
    let mut rows = Vec::new();
    for (index, row) in reader.deserialize::<T>().enumerate() {
        rows.push(row.map_err(|e| format!("{} row {}: {e}", path.display(), index + 2))?);
    }
    if rows.is_empty() {
        return Err(format!("{} has a header but no rows", path.display()));
    }
    Ok((rows, bytes))
}

fn read_train_families(path: &Path) -> Result<(BTreeSet<String>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read training-family manifest {}: {e}", path.display()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "training-family manifest is not UTF-8".to_string())?;
    let mut ids = BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        let id = line.trim();
        if id.is_empty() || id.starts_with('#') { continue; }
        if !ids.insert(id.to_string()) {
            return Err(format!("duplicate training family {id:?} at line {}", i + 1));
        }
    }
    if ids.is_empty() {
        return Err("training-family manifest has no family IDs".into());
    }
    Ok((ids, bytes))
}

fn checkpoint_key(c: &CheckpointRow) -> CheckpointKey {
    CheckpointKey(c.family_id.clone(), c.flight_id.clone(), c.rollout_start_id.clone(), c.checkpoint_time_s.to_bits())
}
fn branch_key(b: &BranchRow) -> BranchKey {
    BranchKey(b.family_id.clone(), b.flight_id.clone(), b.rollout_start_id.clone(),
        b.checkpoint_time_s.to_bits(), b.training_seed, b.intervention_id.clone(), b.branch_seed)
}
fn unit_key(b: &BranchRow) -> UnitKey {
    UnitKey(b.family_id.clone(), b.flight_id.clone(), b.rollout_start_id.clone(), b.training_seed)
}
fn snapshot_key(b: &BranchRow) -> SnapshotKey {
    SnapshotKey(b.family_id.clone(), b.flight_id.clone(), b.rollout_start_id.clone(),
        b.checkpoint_time_s.to_bits(), b.training_seed)
}
fn tape_key(b: &BranchRow) -> TapeKey {
    TapeKey(b.family_id.clone(), b.flight_id.clone(), b.rollout_start_id.clone(),
        b.checkpoint_time_s.to_bits(), b.training_seed, b.branch_seed)
}

fn is_success(branch: &BranchRow, max_tracking_rmse_m: f64) -> bool {
    branch.status == "ok"
        && branch.mission_completed
        && branch.boundary_violation_count == 0
        && branch.deadline_misses == 0
        && branch.branch_elapsed_s <= branch.remaining_budget_s + EPS
        && branch.tracking_rmse_m.is_some_and(|x| x.is_finite() && x >= 0.0 && x <= max_tracking_rmse_m)
}

fn wilson_lower_one_sided(successes: usize, trials: usize) -> f64 {
    if trials == 0 { return 0.0; }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z = WILSON_Z_ONE_SIDED_95;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let radius = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((center - radius) / denominator).clamp(0.0, 1.0)
}

#[derive(Clone, Debug)]
struct Validated {
    checkpoints: Vec<CheckpointRow>,
    branches: Vec<BranchRow>,
    families: Vec<String>,
    flights: Vec<String>,
    training_seeds: Vec<u64>,
    branch_seeds: Vec<u64>,
    interventions: Vec<String>,
    checkpoint_manifest_sha256: String,
    train_manifest_sha256: String,
    input_sha256: String,
}

fn validate_inputs(
    args: &Args,
    checkpoints: Vec<CheckpointRow>,
    branches: Vec<BranchRow>,
    train_families: BTreeSet<String>,
    checkpoint_manifest_sha256: String,
    train_manifest_sha256: String,
    input_sha256: String,
) -> Result<Validated, String> {
    if checkpoints.is_empty() || branches.is_empty() { return Err("checkpoint manifest and branch CSV must be non-empty".into()); }
    let mut checkpoint_map = BTreeMap::<CheckpointKey, CheckpointRow>::new();
    let mut flight_family = BTreeMap::<String, String>::new();
    let mut times_by_start = BTreeMap::<(String, String, String), BTreeSet<u64>>::new();
    let mut terminal_deadlines = BTreeMap::<(String, String, String), f64>::new();
    for c in &checkpoints {
        if c.family_id.trim().is_empty() || c.flight_id.trim().is_empty() || c.rollout_start_id.trim().is_empty() || c.regime.trim().is_empty() {
            return Err("checkpoint manifest contains empty identity/regime".into());
        }
        if !c.checkpoint_time_s.is_finite() || c.checkpoint_time_s <= 0.0 {
            return Err("checkpoint time must be positive and finite".into());
        }
        if !c.remaining_budget_s.is_finite() || c.remaining_budget_s <= 0.0 {
            return Err("remaining task budget must be positive and finite at every checkpoint".into());
        }
        let start_key = (c.family_id.clone(), c.flight_id.clone(), c.rollout_start_id.clone());
        let terminal_deadline = c.checkpoint_time_s + c.remaining_budget_s;
        if let Some(old) = terminal_deadlines.insert(start_key, terminal_deadline) {
            if (old - terminal_deadline).abs() > EPS {
                return Err(format!("checkpoint times and remaining budgets do not preserve a fixed terminal deadline for flight {} start {}", c.flight_id, c.rollout_start_id));
            }
        }
        if train_families.contains(&c.family_id) {
            return Err(format!("evaluation family {:?} also appears in training-family manifest", c.family_id));
        }
        if let Some(old) = flight_family.insert(c.flight_id.clone(), c.family_id.clone()) {
            if old != c.family_id { return Err(format!("flight {:?} is assigned to multiple families", c.flight_id)); }
        }
        let key = checkpoint_key(c);
        if checkpoint_map.insert(key, c.clone()).is_some() {
            return Err(format!("duplicate checkpoint key family={} flight={} start={} time={}", c.family_id, c.flight_id, c.rollout_start_id, c.checkpoint_time_s));
        }
        times_by_start.entry((c.family_id.clone(), c.flight_id.clone(), c.rollout_start_id.clone()))
            .or_default().insert(c.checkpoint_time_s.to_bits());
    }
    if checkpoint_map.len() != checkpoints.len() { return Err("duplicate checkpoint manifest rows".into()); }
    let families = checkpoints.iter().map(|c| c.family_id.clone()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    let flights = checkpoints.iter().map(|c| c.flight_id.clone()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    if families.len() < 2 { return Err("at least two independent evaluation families are required".into()); }
    if flights.len() < 2 { return Err("at least two evaluation flights are required".into()); }

    for ((family, flight, start), times) in &times_by_start {
        if times.len() < 2 {
            return Err(format!("family {family} flight {flight} start {start} needs at least two checkpoints"));
        }
    }

    let mut seen = BTreeSet::<BranchKey>::new();
    let mut snapshots = BTreeMap::<SnapshotKey, String>::new();
    let mut tapes = BTreeMap::<TapeKey, String>::new();
    let mut branch_seed_for_tape = BTreeMap::<(String, String, String, u64, u64, String), u64>::new();
    let training_set = args.training_seeds.iter().copied().collect::<BTreeSet<_>>();
    let branch_set = args.branch_seeds.iter().copied().collect::<BTreeSet<_>>();
    let intervention_set = args.interventions.iter().cloned().collect::<BTreeSet<_>>();
    for b in &branches {
        let checkpoint_identity = CheckpointRow {
            family_id: b.family_id.clone(), flight_id: b.flight_id.clone(),
            rollout_start_id: b.rollout_start_id.clone(), regime: b.regime.clone(),
            checkpoint_time_s: b.checkpoint_time_s, remaining_budget_s: b.remaining_budget_s,
        };
        let registered = checkpoint_map.get(&checkpoint_key(&checkpoint_identity))
            .ok_or_else(|| format!("branch references unregistered checkpoint: family={} flight={} start={} time={}", b.family_id, b.flight_id, b.rollout_start_id, b.checkpoint_time_s))?;
        if registered.regime != b.regime {
            return Err("branch regime does not match its registered checkpoint".into());
        }
        if (registered.remaining_budget_s - b.remaining_budget_s).abs() > EPS {
            return Err("branch remaining budget does not match its registered checkpoint".into());
        }
        if !training_set.contains(&b.training_seed) || !branch_set.contains(&b.branch_seed) || !intervention_set.contains(&b.intervention_id) {
            return Err("branch contains an unregistered training seed, branch seed, or intervention".into());
        }
        if !b.remaining_budget_s.is_finite() || b.remaining_budget_s <= 0.0 {
            return Err("branch remaining budget must be positive and finite".into());
        }
        if !b.branch_elapsed_s.is_finite() || b.branch_elapsed_s < 0.0 || b.branch_elapsed_s > b.remaining_budget_s + EPS {
            return Err("branch elapsed time is invalid or exceeds the registered remaining task budget".into());
        }
        if !valid_hex(&b.state_snapshot_sha256, 64) || !valid_hex(&b.exogenous_tape_sha256, 64) {
            return Err("snapshot and exogenous-tape identifiers must be 64-character hexadecimal digests".into());
        }
        if !b.restore_verified {
            return Err(format!("restore verification failed at family={} flight={} start={} time={} intervention={}", b.family_id, b.flight_id, b.rollout_start_id, b.checkpoint_time_s, b.intervention_id));
        }
        if !matches!(b.status.as_str(), "ok" | "timeout" | "diverged" | "non_finite" | "constraint_violation" | "missing_prediction") {
            return Err(format!("unsupported branch status {:?}", b.status));
        }
        for (name, value) in [("tracking RMSE", b.tracking_rmse_m), ("energy", b.energy_wh)] {
            if value.is_some_and(|x| !x.is_finite() || x < 0.0) {
                return Err(format!("{name} must be finite and non-negative when supplied"));
            }
        }
        if b.status == "ok" && (b.tracking_rmse_m.is_none() || b.energy_wh.is_none()) {
            return Err("status=ok requires finite tracking_rmse_m and energy_wh".into());
        }
        if b.status != "ok" && b.mission_completed {
            return Err("failed branch status cannot claim mission_completed=true".into());
        }
        let key = branch_key(b);
        if !seen.insert(key) {
            return Err("duplicate family/flight/start/checkpoint/training-seed/intervention/branch-seed row".into());
        }
        let snapshot_key = snapshot_key(b);
        if let Some(old) = snapshots.insert(snapshot_key, b.state_snapshot_sha256.clone()) {
            if old != b.state_snapshot_sha256 {
                return Err("counterfactual interventions/branches do not share the same pre-action state snapshot".into());
            }
        }
        let tape_key = tape_key(b);
        if let Some(old) = tapes.insert(tape_key, b.exogenous_tape_sha256.clone()) {
            if old != b.exogenous_tape_sha256 {
                return Err("paired intervention arms do not share the registered exogenous disturbance tape".into());
            }
        }
        let digest_key = (b.family_id.clone(), b.flight_id.clone(), b.rollout_start_id.clone(),
            b.checkpoint_time_s.to_bits(), b.training_seed, b.exogenous_tape_sha256.clone());
        if let Some(previous_seed) = branch_seed_for_tape.insert(digest_key, b.branch_seed) {
            if previous_seed != b.branch_seed {
                return Err("distinct branch seeds reuse the same exogenous disturbance-tape digest at a fixed state".into());
            }
        }
    }

    let expected = checkpoints.len()
        .checked_mul(args.training_seeds.len())
        .and_then(|n| n.checked_mul(args.interventions.len()))
        .and_then(|n| n.checked_mul(args.branch_seeds.len()))
        .ok_or_else(|| "expected branch-grid size overflow".to_string())?;
    if seen.len() != expected {
        return Err(format!("incomplete branch grid: found {} rows, expected {expected}", seen.len()));
    }
    for c in &checkpoints {
        for training_seed in &args.training_seeds {
            for intervention in &args.interventions {
                for branch_seed in &args.branch_seeds {
                    let key = BranchKey(c.family_id.clone(), c.flight_id.clone(), c.rollout_start_id.clone(),
                        c.checkpoint_time_s.to_bits(), *training_seed, intervention.clone(), *branch_seed);
                    if !seen.contains(&key) {
                        return Err(format!("missing branch for family={} flight={} start={} time={} train_seed={} intervention={} branch_seed={}",
                            c.family_id, c.flight_id, c.rollout_start_id, c.checkpoint_time_s, training_seed, intervention, branch_seed));
                    }
                }
            }
        }
    }

    Ok(Validated {
        checkpoints, branches, families, flights, training_seeds: args.training_seeds.clone(),
        branch_seeds: args.branch_seeds.clone(), interventions: args.interventions.clone(),
        checkpoint_manifest_sha256, train_manifest_sha256, input_sha256,
    })
}

fn quantile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() { return None; }
    let pos = p.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi { Some(sorted[lo]) }
    else { Some(sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)) }
}

#[derive(Clone)]
struct SplitMix64 { state: u64 }
impl SplitMix64 {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn index(&mut self, len: usize) -> usize { (self.next() % len as u64) as usize }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
}

fn crossed_bootstrap_delta(
    horizons: &BTreeMap<UnitKey, BTreeMap<String, f64>>,
    family_starts: &BTreeMap<String, Vec<(String, String)>>,
    families: &[String],
    training_seeds: &[u64],
    intervention: &str,
    replicates: usize,
    seed: u64,
) -> Interval {
    let mut rng = SplitMix64::new(seed);
    let mut samples = Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let sampled_families = (0..families.len()).map(|_| families[rng.index(families.len())].clone()).collect::<Vec<_>>();
        let sampled_training_seeds = (0..training_seeds.len()).map(|_| training_seeds[rng.index(training_seeds.len())]).collect::<Vec<_>>();
        let mut family_deltas = Vec::new();
        for family in &sampled_families {
            let mut deltas_within_family = Vec::new();
            for (flight, start) in family_starts.get(family).into_iter().flatten() {
                for train_seed in &sampled_training_seeds {
                    let key = UnitKey(family.clone(), flight.clone(), start.clone(), *train_seed);
                    if let Some(unit) = horizons.get(&key) {
                        if let (Some(candidate), Some(reference)) = (unit.get(intervention), unit.get("continue")) {
                            deltas_within_family.push(candidate - reference);
                        }
                    }
                }
            }
            if !deltas_within_family.is_empty() {
                family_deltas.push(mean(&deltas_within_family));
            }
        }
        if !family_deltas.is_empty() { samples.push(mean(&family_deltas)); }
    }
    samples.sort_by(f64::total_cmp);
    Interval {
        lower_95: quantile(&samples, 0.025),
        upper_95: quantile(&samples, 0.975),
        valid_replicates: samples.len(),
    }
}

fn evaluate(
    args: &Args,
    validated: Validated,
    branch_artifact_verification: BranchArtifactVerification,
) -> Result<Report, String> {
    let mut grouped = BTreeMap::<(CheckpointKey, u64, String), Vec<&BranchRow>>::new();
    for branch in &validated.branches {
        grouped.entry((checkpoint_key(&CheckpointRow {
            family_id: branch.family_id.clone(), flight_id: branch.flight_id.clone(),
            rollout_start_id: branch.rollout_start_id.clone(), regime: branch.regime.clone(),
            checkpoint_time_s: branch.checkpoint_time_s, remaining_budget_s: branch.remaining_budget_s,
        }), branch.training_seed, branch.intervention_id.clone())).or_default().push(branch);
    }

    let mut estimates = Vec::new();
    let mut horizon_by_unit = BTreeMap::<UnitKey, BTreeMap<String, f64>>::new();
    for ((cp_key, training_seed, intervention), branches) in grouped {
        let checkpoint_time_s = f64::from_bits(cp_key.3);
        let successes = branches.iter().filter(|b| is_success(b, args.max_tracking_rmse_m)).count();
        let n = branches.len();
        let success_rate = successes as f64 / n as f64;
        let lower = wilson_lower_one_sided(successes, n);
        let first = branches[0];
        let unit = horizon_by_unit.entry(unit_key(first)).or_default();
        let horizon = unit.entry(intervention.clone()).or_insert(0.0);
        if lower + EPS >= args.recoverability_threshold && checkpoint_time_s > *horizon {
            *horizon = checkpoint_time_s;
        }
        let finite_rmse = branches.iter().filter_map(|b| b.tracking_rmse_m.filter(|x| x.is_finite())).collect::<Vec<_>>();
        let finite_energy = branches.iter().filter_map(|b| b.energy_wh.filter(|x| x.is_finite())).collect::<Vec<_>>();
        let violation_rate = branches.iter().filter(|b| b.boundary_violation_count > 0).count() as f64 / n as f64;
        let deadline_rate = branches.iter().filter(|b| b.deadline_misses > 0).count() as f64 / n as f64;
        estimates.push(CheckpointEstimate {
            family_id: first.family_id.clone(), flight_id: first.flight_id.clone(),
            rollout_start_id: first.rollout_start_id.clone(), regime: first.regime.clone(),
            training_seed, checkpoint_time_s, remaining_budget_s: first.remaining_budget_s, intervention_id: intervention,
            branch_replicates: n, successes, success_rate, wilson_95_one_sided_lower: lower,
            qualifies_recoverable: lower + EPS >= args.recoverability_threshold,
            boundary_violation_rate: violation_rate, deadline_miss_rate: deadline_rate,
            mean_tracking_rmse_m_when_finite: if finite_rmse.is_empty() { None } else { Some(mean(&finite_rmse)) },
            mean_energy_wh_when_finite: if finite_energy.is_empty() { None } else { Some(mean(&finite_energy)) },
        });
    }

    let mut family_starts_sets = BTreeMap::<String, BTreeSet<(String, String)>>::new();
    for c in &validated.checkpoints {
        family_starts_sets.entry(c.family_id.clone()).or_default()
            .insert((c.flight_id.clone(), c.rollout_start_id.clone()));
    }
    let family_starts = family_starts_sets.into_iter()
        .map(|(f, pairs)| (f, pairs.into_iter().collect::<Vec<_>>()))
        .collect::<BTreeMap<_, _>>();

    let mut intervention_summaries = Vec::new();
    for intervention in &validated.interventions {
        let mut family_means = Vec::new();
        let mut family_deltas = Vec::new();
        let mut family_recoverable_rates = Vec::new();
        for family in &validated.families {
            let mut values = Vec::new();
            let mut paired = Vec::new();
            let mut recoverable = 0usize;
            let mut count = 0usize;
            for (key, arms) in &horizon_by_unit {
                if &key.0 != family { continue; }
                let value = *arms.get(intervention).ok_or_else(|| format!("missing horizon for intervention {intervention}"))?;
                let reference = *arms.get("continue").ok_or_else(|| "missing continue reference horizon".to_string())?;
                values.push(value);
                paired.push(value - reference);
                if value > 0.0 { recoverable += 1; }
                count += 1;
            }
            if count > 0 {
                family_means.push(mean(&values));
                family_deltas.push(mean(&paired));
                family_recoverable_rates.push(recoverable as f64 / count as f64);
            }
        }
        let bootstrap = crossed_bootstrap_delta(&horizon_by_unit, &family_starts, &validated.families,
            &validated.training_seeds, intervention, args.bootstrap_replicates,
            args.bootstrap_seed.wrapping_add(intervention_summaries.len() as u64));
        intervention_summaries.push(InterventionSummary {
            intervention_id: intervention.clone(),
            reference_intervention_id: "continue".into(),
            mean_recoverability_horizon_s: mean(&family_means),
            recoverable_unit_rate: mean(&family_recoverable_rates),
            mean_paired_horizon_delta_vs_continue_s: mean(&family_deltas),
            paired_crossed_bootstrap_delta_ci95_s: bootstrap,
            note: if intervention == "continue" {
                "Reference arm; a zero paired delta is expected by construction.".into()
            } else {
                "Descriptive intervention-conditioned recoverability. Positive horizon delta is not, by itself, full-policy benefit or flight-safety evidence.".into()
            },
        });
    }

    estimates.sort_by(|a, b| a.family_id.cmp(&b.family_id)
        .then(a.flight_id.cmp(&b.flight_id))
        .then(a.rollout_start_id.cmp(&b.rollout_start_id))
        .then(a.training_seed.cmp(&b.training_seed))
        .then(a.checkpoint_time_s.total_cmp(&b.checkpoint_time_s))
        .then(a.intervention_id.cmp(&b.intervention_id)));

    let mut unit_horizons = Vec::new();
    for (key, arms) in &horizon_by_unit {
        for intervention in &validated.interventions {
            let horizon = *arms.get(intervention).ok_or_else(|| "missing unit horizon".to_string())?;
            unit_horizons.push(UnitHorizon {
                family_id: key.0.clone(), flight_id: key.1.clone(), rollout_start_id: key.2.clone(),
                training_seed: key.3, intervention_id: intervention.clone(),
                latest_recoverable_checkpoint_s: horizon,
            });
        }
    }

    Ok(Report {
        schema_version: "symtropy-nanodrone-recoverability-frontier-v0.2".into(),
        comparison_kind: "counterfactual-checkpoint-intervention-recoverability".into(),
        evidence_class: args.evidence_class.clone(),
        claim_ceiling: "Recoverability under the declared precomputed branches and Wilson threshold only. Snapshot/tape content bytes and declared identities are SHA-256 verified; the restore_verified flag is still producer-declared and artifact bytes do not independently attest simulator capture, restore fidelity, RNG origin, full-policy benefit, real-flight generalization, or flight safety.".into(),
        source_commit: args.source_commit.clone(),
        dataset_receipt_sha256: args.dataset_receipt_sha256.clone(),
        model_manifest_sha256: args.model_manifest_sha256.clone(),
        input_csv_sha256: validated.input_sha256,
        branch_artifact_verification,
        checkpoints_manifest_sha256: validated.checkpoint_manifest_sha256,
        train_families_manifest_sha256: validated.train_manifest_sha256,
        training_seeds: validated.training_seeds.clone(),
        branch_seeds: validated.branch_seeds.clone(),
        intervention_ids: validated.interventions.clone(),
        recoverability_threshold: args.recoverability_threshold,
        recoverability_interval: "one-sided 95% Wilson lower bound across branch seeds at each fixed state/training seed; horizon 0 means no registered checkpoint passed".into(),
        budget_contract: "Every checkpoint binds a positive remaining task budget; checkpoint time plus remaining budget must equal the same registered terminal deadline for that flight/start, and successful branches must finish within the budget.".into(),
        horizon_bootstrap_scope: "crossed resampling of independent family clusters and training seeds, conditional on the registered branch-seed panel; per-checkpoint Wilson bounds gate recoverability, but branch-seed identities are not re-resampled into the horizon-delta interval".into(),
        max_tracking_rmse_m: args.max_tracking_rmse_m,
        bootstrap_replicates_requested: args.bootstrap_replicates,
        bootstrap_seed: args.bootstrap_seed,
        registered_family_count: validated.families.len(),
        registered_flight_count: validated.flights.len(),
        registered_checkpoint_count: validated.checkpoints.len(),
        expected_branch_count: validated.branches.len(),
        checkpoint_estimates: estimates,
        unit_horizons,
        intervention_summaries,
    })
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (checkpoints, checkpoint_bytes) = read_csv::<CheckpointRow>(&args.checkpoints)?;
    let (branches, input_bytes) = read_csv::<BranchRow>(&args.input)?;
    let (train_families, train_bytes) = read_train_families(&args.train_families)?;
    let branch_artifact_verification =
        verify_branch_artifacts(&args.artifact_root, &branches, &args.evidence_class)?;
    let validated = validate_inputs(&args, checkpoints, branches, train_families,
        sha256(&checkpoint_bytes), sha256(&train_bytes), sha256(&input_bytes))?;
    let report = evaluate(&args, validated, branch_artifact_verification)?;
    let output = serde_json::to_string_pretty(&report).map_err(|e| format!("cannot serialize report: {e}"))?;
    if let Some(path) = args.output {
        fs::write(&path, format!("{output}\n")).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    } else {
        println!("{output}");
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("nanodrone_recoverability_frontier: {err}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ARTIFACT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    fn write_artifact_fixture(mut branches: Vec<BranchRow>) -> (PathBuf, Vec<BranchRow>) {
        let root = std::env::temp_dir().join(format!(
            "nanodrone-branch-artifacts-{}-{}",
            std::process::id(),
            NEXT_ARTIFACT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let snapshots = root.join("snapshots");
        let tapes = root.join("disturbance-tapes");
        fs::create_dir_all(&snapshots).unwrap();
        fs::create_dir_all(&tapes).unwrap();
        fs::write(root.join("README.md"), b"synthetic test bundle\n").unwrap();

        let mut snapshot_files = BTreeMap::<String, Vec<u8>>::new();
        for row in &mut branches {
            let artifact = StateSnapshotArtifact {
                schema_version: "symtropy-nanodrone-state-snapshot-v0.1".into(),
                evidence_class: "synthetic_fixture".into(),
                family_id: row.family_id.clone(),
                flight_id: row.flight_id.clone(),
                rollout_start_id: row.rollout_start_id.clone(),
                regime: row.regime.clone(),
                checkpoint_time_s: row.checkpoint_time_s,
                training_seed: row.training_seed,
                synthetic_state_payload: format!(
                    "SYNTHETIC_STATE_ONLY|{}|{}|{}|{}|{}|{}",
                    row.family_id, row.flight_id, row.rollout_start_id, row.regime,
                    row.checkpoint_time_s, row.training_seed
                ),
                claim_ceiling: "fixture_byte_integrity_only_no_simulator_origin_or_restore_attestation".into(),
            };
            let mut bytes = serde_json::to_vec_pretty(&artifact).unwrap();
            bytes.push(b'\n');
            let digest = sha256(&bytes);
            row.state_snapshot_sha256 = digest.clone();
            snapshot_files.insert(digest, bytes);
        }

        let mut tape_files = BTreeMap::<String, Vec<u8>>::new();
        for row in &mut branches {
            let artifact = DisturbanceTapeArtifact {
                schema_version: "symtropy-nanodrone-exogenous-tape-v0.1".into(),
                evidence_class: "synthetic_fixture".into(),
                family_id: row.family_id.clone(),
                flight_id: row.flight_id.clone(),
                rollout_start_id: row.rollout_start_id.clone(),
                regime: row.regime.clone(),
                checkpoint_time_s: row.checkpoint_time_s,
                training_seed: row.training_seed,
                branch_seed: row.branch_seed,
                synthetic_disturbance_payload: format!(
                    "SYNTHETIC_TAPE_ONLY|{}|{}|{}|{}|{}|{}|{}",
                    row.family_id, row.flight_id, row.rollout_start_id, row.regime,
                    row.checkpoint_time_s, row.training_seed, row.branch_seed
                ),
                claim_ceiling: "fixture_byte_integrity_only_no_simulator_origin_or_rng_state_attestation".into(),
            };
            let mut bytes = serde_json::to_vec_pretty(&artifact).unwrap();
            bytes.push(b'\n');
            let digest = sha256(&bytes);
            row.exogenous_tape_sha256 = digest.clone();
            tape_files.insert(digest, bytes);
        }

        for (digest, bytes) in snapshot_files {
            fs::write(snapshots.join(format!("{digest}.json")), bytes).unwrap();
        }
        for (digest, bytes) in tape_files {
            fs::write(tapes.join(format!("{digest}.json")), bytes).unwrap();
        }
        (root, branches)
    }

    fn test_args() -> Args {
        Args {
            input: PathBuf::new(), checkpoints: PathBuf::new(), train_families: PathBuf::new(),
            artifact_root: PathBuf::new(),
            dataset_receipt_sha256: "a".repeat(64), model_manifest_sha256: "b".repeat(64),
            source_commit: "c".repeat(40), training_seeds: vec![101, 102], branch_seeds: vec![11, 12],
            interventions: vec!["continue".into(), "robust_fallback".into()],
            max_tracking_rmse_m: 0.5, recoverability_threshold: 0.5,
            bootstrap_replicates: 50, bootstrap_seed: 17, evidence_class: "synthetic".into(),
            output: None,
        }
    }

    fn fixture_inputs() -> (Vec<CheckpointRow>, Vec<BranchRow>, BTreeSet<String>) {
        let mut checkpoints = Vec::new();
        let mut branches = Vec::new();
        for (idx, (family, flight, regime)) in [
            ("family-a", "flight-a", "nominal"),
            ("family-b", "flight-b", "shifted"),
        ].into_iter().enumerate() {
            for checkpoint_time_s in [1.0, 2.0] {
                checkpoints.push(CheckpointRow {
                    family_id: family.into(), flight_id: flight.into(), rollout_start_id: "start-1".into(),
                    regime: regime.into(), checkpoint_time_s, remaining_budget_s: 4.0 - checkpoint_time_s,
                });
                for training_seed in [101, 102] {
                    let snapshot = format!("{:064x}", 1000 + idx as u64 * 100 + checkpoint_time_s as u64 * 10 + training_seed);
                    for intervention_id in ["continue", "robust_fallback"] {
                        for branch_seed in [11, 12] {
                            let tape = format!("{:064x}", 100_000 + idx as u64 * 1000 + checkpoint_time_s as u64 * 100 + training_seed + branch_seed);
                            branches.push(BranchRow {
                                family_id: family.into(), flight_id: flight.into(), rollout_start_id: "start-1".into(),
                                regime: regime.into(), checkpoint_time_s, remaining_budget_s: 4.0 - checkpoint_time_s,
                                branch_elapsed_s: 0.5, training_seed,
                                intervention_id: intervention_id.into(), branch_seed,
                                state_snapshot_sha256: snapshot.clone(), exogenous_tape_sha256: tape,
                                restore_verified: true, status: "ok".into(), mission_completed: true,
                                boundary_violation_count: 0, tracking_rmse_m: Some(0.2),
                                energy_wh: Some(1.0), deadline_misses: 0,
                            });
                        }
                    }
                }
            }
        }
        (checkpoints, branches, BTreeSet::from(["train-only-family".to_string()]))
    }

    fn validate_fixture(
        args: &Args, checkpoints: Vec<CheckpointRow>, branches: Vec<BranchRow>, train: BTreeSet<String>,
    ) -> Result<Validated, String> {
        validate_inputs(args, checkpoints, branches, train, "d".repeat(64), "e".repeat(64), "f".repeat(64))
    }

    #[test]
    fn synthetic_artifact_schemas_reject_unknown_fields() {
        let mut snapshot = serde_json::to_value(StateSnapshotArtifact {
            schema_version: "symtropy-nanodrone-state-snapshot-v0.1".into(),
            evidence_class: "synthetic_fixture".into(),
            family_id: "family-a".into(),
            flight_id: "flight-a".into(),
            rollout_start_id: "start-1".into(),
            regime: "nominal".into(),
            checkpoint_time_s: 1.0,
            training_seed: 101,
            synthetic_state_payload: "SYNTHETIC_STATE_ONLY|test".into(),
            claim_ceiling: "fixture_byte_integrity_only_no_simulator_origin_or_restore_attestation".into(),
        }).unwrap();
        snapshot.as_object_mut().unwrap().insert("restore_attested".into(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<StateSnapshotArtifact>(snapshot).is_err());

        let mut tape = serde_json::to_value(DisturbanceTapeArtifact {
            schema_version: "symtropy-nanodrone-exogenous-tape-v0.1".into(),
            evidence_class: "synthetic_fixture".into(),
            family_id: "family-a".into(),
            flight_id: "flight-a".into(),
            rollout_start_id: "start-1".into(),
            regime: "nominal".into(),
            checkpoint_time_s: 1.0,
            training_seed: 101,
            branch_seed: 11,
            synthetic_disturbance_payload: "SYNTHETIC_TAPE_ONLY|test".into(),
            claim_ceiling: "fixture_byte_integrity_only_no_simulator_origin_or_rng_state_attestation".into(),
        }).unwrap();
        tape.as_object_mut().unwrap().insert("rng_seed_attested".into(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<DisturbanceTapeArtifact>(tape).is_err());
    }

    #[test]
    fn synthetic_v0_1_artifacts_cannot_be_relabelled_as_simulation_or_real_flight() {
        let (_, branches, _) = fixture_inputs();
        let (root, branches) = write_artifact_fixture(branches);
        assert!(verify_branch_artifacts(&root, &branches, "simulation")
            .unwrap_err().contains("synthetic-fixture-only"));
        assert!(verify_branch_artifacts(&root, &branches, "real-flight")
            .unwrap_err().contains("synthetic-fixture-only"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn content_addressed_branch_artifacts_verify_bytes_and_pairing_inventory() {
        let (_, branches, _) = fixture_inputs();
        let (root, branches) = write_artifact_fixture(branches);
        let verified = verify_branch_artifacts(&root, &branches, "synthetic").unwrap();
        assert_eq!(verified.status, "synthetic_fixture_content_verified_origin_unattested");
        assert_eq!(verified.snapshot_artifacts_verified, 8);
        assert_eq!(verified.disturbance_tape_artifacts_verified, 16);
        assert_eq!(verified.branch_rows_bound, 32);
        assert_eq!(verified.evidence_class, "synthetic_fixture");
        assert_eq!(verified.snapshot_manifest_sha256.len(), 64);
        assert_eq!(verified.disturbance_tape_manifest_sha256.len(), 64);
        assert_eq!(verified.combined_artifact_manifest_sha256.len(), 64);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn branch_artifact_verifier_rejects_byte_tampering_missing_and_orphan_files() {
        let (_, branches, _) = fixture_inputs();
        let (root, branches) = write_artifact_fixture(branches);
        let first = &branches[0];
        let snapshot_path = root.join("snapshots").join(format!("{}.json", first.state_snapshot_sha256));
        let mut bytes = fs::read(&snapshot_path).unwrap();
        bytes.push(b' ');
        fs::write(&snapshot_path, bytes).unwrap();
        assert!(verify_branch_artifacts(&root, &branches, "synthetic")
            .unwrap_err().contains("SHA-256 does not match"));
        fs::remove_dir_all(&root).unwrap();

        let (_, branches, _) = fixture_inputs();
        let (root, branches) = write_artifact_fixture(branches);
        let first = &branches[0];
        let snapshot_path = root.join("snapshots").join(format!("{}.json", first.state_snapshot_sha256));
        fs::remove_file(snapshot_path).unwrap();
        assert!(verify_branch_artifacts(&root, &branches, "synthetic")
            .unwrap_err().contains("inventory mismatch"));
        fs::remove_dir_all(&root).unwrap();

        let (_, branches, _) = fixture_inputs();
        let (root, branches) = write_artifact_fixture(branches);
        fs::write(root.join("disturbance-tapes").join(format!("{}.json", "f".repeat(64))), b"orphan")
            .unwrap();
        assert!(verify_branch_artifacts(&root, &branches, "synthetic")
            .unwrap_err().contains("inventory mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn branch_artifact_verifier_rejects_hash_valid_but_misbound_snapshot_identity() {
        let (_, branches, _) = fixture_inputs();
        let (root, mut branches) = write_artifact_fixture(branches);
        let old_digest = branches[0].state_snapshot_sha256.clone();
        let old_path = root.join("snapshots").join(format!("{old_digest}.json"));
        let mut artifact: StateSnapshotArtifact =
            serde_json::from_slice(&fs::read(&old_path).unwrap()).unwrap();
        artifact.family_id = "different-family".into();
        let mut bytes = serde_json::to_vec_pretty(&artifact).unwrap();
        bytes.push(b'\n');
        let new_digest = sha256(&bytes);
        fs::write(root.join("snapshots").join(format!("{new_digest}.json")), bytes).unwrap();
        fs::remove_file(old_path).unwrap();
        for row in branches.iter_mut().filter(|row| row.state_snapshot_sha256 == old_digest) {
            row.state_snapshot_sha256 = new_digest.clone();
        }
        assert!(verify_branch_artifacts(&root, &branches, "synthetic")
            .unwrap_err().contains("metadata/identity mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn branch_artifact_verifier_rejects_symlinked_root_and_artifact_file() {
        use std::os::unix::fs::symlink;

        let (_, branches, _) = fixture_inputs();
        let (root, branches) = write_artifact_fixture(branches);
        let link = root.with_extension("symlink");
        symlink(&root, &link).unwrap();
        assert!(verify_branch_artifacts(&link, &branches, "synthetic")
            .unwrap_err().contains("real non-symlink directory"));
        fs::remove_file(&link).unwrap();

        let target = root.join("snapshots").join("target.json");
        fs::write(&target, b"not-an-artifact").unwrap();
        let digest = &branches[0].state_snapshot_sha256;
        let artifact = root.join("snapshots").join(format!("{digest}.json"));
        fs::remove_file(&artifact).unwrap();
        symlink(&target, &artifact).unwrap();
        assert!(verify_branch_artifacts(&root, &branches, "synthetic")
            .unwrap_err().contains("regular non-symlink files"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wilson_lower_bound_rejects_tiny_sample_claims() {
        assert!(wilson_lower_one_sided(4, 4) > 0.5);
        assert!(wilson_lower_one_sided(3, 4) < 0.5);
        assert_eq!(wilson_lower_one_sided(0, 4), 0.0);
        assert_eq!(wilson_lower_one_sided(0, 0), 0.0);
    }

    #[test]
    fn failed_or_late_tracking_branch_is_not_recoverable() {
        let branch = BranchRow {
            family_id: "family-a".into(), flight_id: "flight-a".into(), rollout_start_id: "start-a".into(),
            regime: "nominal".into(), checkpoint_time_s: 1.0, remaining_budget_s: 3.0, branch_elapsed_s: 1.0, training_seed: 7,
            intervention_id: "continue".into(), branch_seed: 1,
            state_snapshot_sha256: "a".repeat(64), exogenous_tape_sha256: "b".repeat(64),
            restore_verified: true, status: "ok".into(), mission_completed: true,
            boundary_violation_count: 0, tracking_rmse_m: Some(0.9), energy_wh: Some(1.0), deadline_misses: 0,
        };
        assert!(!is_success(&branch, 0.5));
        let mut failed = branch.clone();
        failed.status = "diverged".into();
        failed.mission_completed = false;
        failed.tracking_rmse_m = None;
        assert!(!is_success(&failed, 0.5));
    }

    #[test]
    fn complete_paired_branch_grid_is_accepted() {
        let args = test_args();
        let (checkpoints, branches, train) = fixture_inputs();
        let validated = validate_fixture(&args, checkpoints, branches, train).unwrap();
        assert_eq!(validated.branches.len(), 32);
        assert_eq!(validated.families.len(), 2);
        assert_eq!(validated.training_seeds.len(), 2);
    }

    #[test]
    fn missing_branch_is_rejected() {
        let args = test_args();
        let (checkpoints, mut branches, train) = fixture_inputs();
        branches.pop();
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("incomplete branch grid"));
    }

    #[test]
    fn intervention_with_different_snapshot_is_rejected() {
        let args = test_args();
        let (checkpoints, mut branches, train) = fixture_inputs();
        let row = branches.iter_mut().find(|b| b.intervention_id == "robust_fallback" && b.training_seed == 101 && b.checkpoint_time_s == 1.0 && b.branch_seed == 11).unwrap();
        row.state_snapshot_sha256 = "9".repeat(64);
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("same pre-action state snapshot"));
    }

    #[test]
    fn intervention_with_different_disturbance_tape_is_rejected() {
        let args = test_args();
        let (checkpoints, mut branches, train) = fixture_inputs();
        let row = branches.iter_mut().find(|b| b.intervention_id == "robust_fallback" && b.training_seed == 101 && b.checkpoint_time_s == 1.0 && b.branch_seed == 11).unwrap();
        row.exogenous_tape_sha256 = "9".repeat(64);
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("same registered exogenous disturbance tape"));
    }

    #[test]
    fn evaluation_family_in_training_manifest_is_rejected() {
        let args = test_args();
        let (checkpoints, branches, mut train) = fixture_inputs();
        train.insert("family-a".into());
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("also appears in training-family manifest"));
    }

    #[test]
    fn distinct_branch_seeds_cannot_reuse_the_same_tape_digest() {
        let args = test_args();
        let (checkpoints, mut branches, train) = fixture_inputs();
        let first_tape = branches.iter().find(|b| b.family_id == "family-a" && b.checkpoint_time_s == 1.0 && b.training_seed == 101 && b.intervention_id == "continue" && b.branch_seed == 11).unwrap().exogenous_tape_sha256.clone();
        for row in branches.iter_mut().filter(|b| b.family_id == "family-a" && b.checkpoint_time_s == 1.0 && b.training_seed == 101 && b.branch_seed == 12) {
            row.exogenous_tape_sha256 = first_tape.clone();
        }
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("distinct branch seeds reuse"));
    }

    #[test]
    fn inconsistent_remaining_budget_across_checkpoints_is_rejected() {
        let args = test_args();
        let (mut checkpoints, branches, train) = fixture_inputs();
        checkpoints.iter_mut().find(|c| c.flight_id == "flight-a" && c.checkpoint_time_s == 2.0).unwrap().remaining_budget_s = 1.0;
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("fixed terminal deadline"));
    }

    #[test]
    fn branch_elapsed_beyond_remaining_budget_is_rejected() {
        let args = test_args();
        let (checkpoints, mut branches, train) = fixture_inputs();
        branches[0].branch_elapsed_s = 4.0;
        assert!(validate_fixture(&args, checkpoints, branches, train).unwrap_err().contains("exceeds the registered remaining task budget"));
    }

    #[test]
    fn splitmix_resampling_is_deterministic() {
        let mut a = SplitMix64::new(77);
        let mut b = SplitMix64::new(77);
        let left = (0..100).map(|_| a.next()).collect::<Vec<_>>();
        let right = (0..100).map(|_| b.next()).collect::<Vec<_>>();
        assert_eq!(left, right);
    }

    #[test]
    fn quantile_interpolates_and_handles_empty_samples() {
        assert_eq!(quantile(&[], 0.5), None);
        assert_eq!(quantile(&[1.0, 3.0], 0.5), Some(2.0));
        assert_eq!(quantile(&[1.0, 2.0, 3.0], 0.95), Some(2.9));
    }
}
