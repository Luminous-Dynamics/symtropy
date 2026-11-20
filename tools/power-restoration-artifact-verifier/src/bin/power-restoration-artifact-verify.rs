use power_restoration_artifact_verifier::{
    verify_attestation_set, verify_bundle_manifest, AttestationEnvelope,
    BundleManifest, ExpectedAttestationBinding, TrustRoot, VerifiedAttestation,
    BundleVerificationReport,
};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CliReport {
    result_status: String,
    bundle: BundleVerificationReport,
    verified_attestations: Vec<VerifiedAttestation>,
    claim_boundary: String,
}

fn read_json<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {label} {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot decode {label} {}: {error}", path.display()))
}

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} <bundle-root> <manifest.json> <trust-root.json> \
         <pinned-trust-root-sha256> <now-unix-ms> <requirements.json> <attestations.json>\n\
         \n\
         requirements.json is a JSON array of exact attestation obligations derived \
         from the run receipt and observation/service evidence records. The trust-root \
         digest must be supplied through an out-of-band trusted configuration channel."
    )
}

fn run() -> Result<CliReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if values.len() != 7 {
        return Err(usage(&program.to_string_lossy()));
    }

    let bundle_root = PathBuf::from(&values[0]);
    let manifest_path = PathBuf::from(&values[1]);
    let trust_root_path = PathBuf::from(&values[2]);
    let pinned_root = values[3]
        .to_str()
        .ok_or_else(|| "pinned trust-root digest must be UTF-8".to_string())?;
    let now_unix_ms = values[4]
        .to_str()
        .ok_or_else(|| "verification timestamp must be UTF-8".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid verification timestamp: {error}"))?;
    let requirements_path = PathBuf::from(&values[5]);
    let attestations_path = PathBuf::from(&values[6]);

    let manifest: BundleManifest = read_json(&manifest_path, "bundle manifest")?;
    let trust_root: TrustRoot = read_json(&trust_root_path, "trust root")?;
    let requirements: Vec<ExpectedAttestationBinding> =
        read_json(&requirements_path, "attestation requirements")?;
    let attestations: Vec<AttestationEnvelope> =
        read_json(&attestations_path, "attestation envelopes")?;

    let verified_bundle = verify_bundle_manifest(&bundle_root, &manifest, 64 * 1024 * 1024)
        .map_err(|errors| format!("bundle verification failed: {}", errors.join("; ")))?;
    let verified_attestations = verify_attestation_set(
        &verified_bundle,
        &trust_root,
        pinned_root,
        now_unix_ms,
        &requirements,
        &attestations,
    )
    .map_err(|errors| format!("attestation verification failed: {}", errors.join("; ")))?;

    Ok(CliReport {
        result_status: "bundle_and_required_attestations_verified".to_string(),
        bundle: verified_bundle.report().clone(),
        verified_attestations,
        claim_boundary: "Offline artifact byte identity and caller-pinned Ed25519 claim authentication only; underlying physical truth, electrical feasibility, service delivery and restoration success remain domain-qualified claims".to_string(),
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("verification report serialization failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
