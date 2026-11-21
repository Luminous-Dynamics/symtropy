use power_restoration_artifact_verifier::{
    AttestationEnvelope, BundleManifest, ClaimProducerProfile, RestorationClaimVerificationReport,
    TrustRoot, verify_restoration_claim_evidence,
};
use power_restoration_benchmark_case::BenchmarkCase;
use power_restoration_observation_evidence::EvidenceBatch;
use power_restoration_run_receipt::RunReceipt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ClaimPackage {
    case: BenchmarkCase,
    run_receipt: RunReceipt,
    observation_batches: Vec<EvidenceBatch>,
    manifest: BundleManifest,
    attestations: Vec<AttestationEnvelope>,
    run_receipt_artifact_uri: String,
}

fn read_json<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {label} {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot decode {label} {}: {error}", path.display()))
}

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} <bundle-root> <claim-package.json> <trusted-roots.json> <pinned-trust-root-sha256> <producer-profiles.json> <verified-at-unix-ms>\n\nTrust-root digest and producer profiles are supplied outside the evidence bundle. The claim package contains the frozen case, run receipt, observation batches, artifact manifest, attestations, and run receipt artifact URI."
    )
}

fn run() -> Result<RestorationClaimVerificationReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if values.len() != 6 {
        return Err(usage(&program.to_string_lossy()));
    }

    let bundle_root = PathBuf::from(&values[0]);
    let claim_package_path = PathBuf::from(&values[1]);
    let trust_root_path = PathBuf::from(&values[2]);
    let pinned_root = values[3]
        .to_str()
        .ok_or_else(|| "pinned trust-root digest must be UTF-8".to_string())?;
    let producer_profiles_path = PathBuf::from(&values[4]);
    let verified_at_unix_ms = values[5]
        .to_str()
        .ok_or_else(|| "verification timestamp must be UTF-8".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid verification timestamp: {error}"))?;

    let package: ClaimPackage = read_json(&claim_package_path, "restoration claim package")?;
    let trust_root: TrustRoot = read_json(&trust_root_path, "trusted root")?;
    let producer_profiles: Vec<ClaimProducerProfile> =
        read_json(&producer_profiles_path, "trusted producer profiles")?;

    verify_restoration_claim_evidence(
        &bundle_root,
        &package.case,
        &package.run_receipt,
        &package.observation_batches,
        &package.manifest,
        &trust_root,
        pinned_root,
        &package.attestations,
        &producer_profiles,
        &package.run_receipt_artifact_uri,
        verified_at_unix_ms,
        64 * 1024 * 1024,
    )
    .map_err(|errors| errors.join("\n"))
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("claim report serialization failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("restoration claim is not qualified:\n{error}");
            ExitCode::from(1)
        }
    }
}
