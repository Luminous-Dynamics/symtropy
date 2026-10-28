//! Verifies the actual bytes of a pinned external CityLearn dataset snapshot.
//!
//! The source CSV bytes are read from an upstream checkout at an exact Git
//! commit. The report stores SHA-256 receipts only; it does not copy source data.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, path::Path, process::Command};

const MANIFEST_BYTES: &[u8] =
    include_bytes!("../../fixtures/citylearn-challenge-2022-plus-evs-v3.0.2-source-manifest-v0.1.json");
const MANIFEST_SHA256: &str =
    "bd5086f66e47b400a1e37b4f6a6b5fffeabe02bf2c3a7a7c001c108813efe657";
const SOURCE_COMMIT: &str = "834575c1a0194c8ae9d648ae858376a94dfceb78";
const DATASET_ID: &str = "citylearn_challenge_2022_phase_all_plus_evs";
const DATASET_ROOT: &str = "data/datasets/citylearn_challenge_2022_phase_all_plus_evs/";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha1(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    schema_version: String,
    dataset_id: String,
    source_repository: String,
    source_release_tag: String,
    source_commit: String,
    source_commit_is_pinned: bool,
    schema_path: String,
    schema_git_blob_sha1: String,
    schema_size_bytes: usize,
    start_date: String,
    simulation_start_step: usize,
    simulation_end_step: usize,
    time_steps: usize,
    seconds_per_time_step: u32,
    buildings: usize,
    dataset_license_status: String,
    forecast_issue_time_status: String,
    external_raw_bytes_committed: bool,
    referenced_csv_count: usize,
    files: Vec<ManifestFile>,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestFile {
    path: String,
    git_blob_sha1: String,
    size_bytes: usize,
    data_rows: usize,
    immutable_source_url: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct FileReceipt {
    path: String,
    git_blob_sha1: String,
    size_bytes: usize,
    data_rows: usize,
    sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Report {
    schema_version: &'static str,
    status: &'static str,
    dataset_id: String,
    source_commit: String,
    source_manifest_sha256: String,
    schema_path: String,
    schema_git_blob_sha1: String,
    time_steps: usize,
    seconds_per_time_step: u32,
    buildings: usize,
    file_count: usize,
    total_source_bytes_verified: usize,
    source_commit_verified: bool,
    schema_git_blob_identity_verified: bool,
    source_file_git_blob_identities_verified: bool,
    raw_sha256_receipts_generated: bool,
    dataset_terms_cleared: bool,
    forecast_issue_times_verified: bool,
    source_bytes_committed_to_symtropy: bool,
    files: Vec<FileReceipt>,
    claim_ceiling: &'static str,
}

/// Count logical CSV records while respecting quoted fields and escaped quotes.
fn csv_record_count(bytes: &[u8]) -> usize {
    let mut records = 0usize;
    let mut in_quotes = false;
    let mut i = 0usize;
    let mut record_start = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'"' if in_quotes && i + 1 < bytes.len() && bytes[i + 1] == b'"' => {
                i += 2;
            }
            b'"' => {
                in_quotes = !in_quotes;
                i += 1;
            }
            b'\n' if !in_quotes => {
                records += 1;
                record_start = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }

    if record_start < bytes.len()
        && bytes[record_start..].iter().any(|byte| !byte.is_ascii_whitespace())
    {
        records += 1;
    }
    records
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "citylearn-source-tree-manifest-v0.1"
        || manifest.dataset_id != DATASET_ID
        || manifest.source_repository != "https://github.com/citylearn-project/CityLearn"
        || manifest.source_release_tag != "v3.0.2"
        || manifest.source_commit != SOURCE_COMMIT
        || !manifest.source_commit_is_pinned
        || manifest.schema_path != format!("{DATASET_ROOT}schema.json")
        || !valid_sha1(&manifest.schema_git_blob_sha1)
        || manifest.schema_size_bytes != 27644
        || manifest.start_date != "2024-08-01"
        || manifest.simulation_start_step != 0
        || manifest.simulation_end_step != 8759
        || manifest.time_steps != 8760
        || manifest.seconds_per_time_step != 3600
        || manifest.buildings != 17
        || manifest.referenced_csv_count != 30
        || manifest.files.len() != manifest.referenced_csv_count
        || manifest.dataset_license_status.contains("rights_reviewed_and_cleared")
        || manifest.forecast_issue_time_status != "not_verified_from_schema_and_column_names"
        || manifest.external_raw_bytes_committed
    {
        return Err("pinned CityLearn manifest changed or overstates qualification".to_owned());
    }

    let mut paths = BTreeSet::new();
    for entry in &manifest.files {
        if !entry.path.starts_with(DATASET_ROOT)
            || entry.path.contains("..")
            || !paths.insert(entry.path.as_str())
            || !valid_sha1(&entry.git_blob_sha1)
            || entry.size_bytes == 0
            || entry.data_rows == 0
        {
            return Err(format!("invalid or duplicate file entry {}", entry.path));
        }
        let expected_url = format!(
            "{}/blob/{}/{}",
            manifest.source_repository, manifest.source_commit, entry.path
        );
        if entry.immutable_source_url != expected_url {
            return Err(format!("file URL does not use pinned commit for {}", entry.path));
        }
    }
    Ok(())
}

fn git_text(root: &Path, args: &[String]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute git for {}: {error}", root.display()))?;
    if !output.status.success() {
        return Err(format!(
            "git {:?} failed in {}: {}",
            args,
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn verify_snapshot(root: &Path) -> Result<Report, String> {
    let actual_manifest_sha = sha256_hex(MANIFEST_BYTES);
    if actual_manifest_sha != MANIFEST_SHA256 {
        return Err(format!(
            "embedded manifest SHA-256 mismatch: expected {MANIFEST_SHA256}, got {actual_manifest_sha}"
        ));
    }
    let manifest: Manifest = serde_json::from_slice(MANIFEST_BYTES)
        .map_err(|error| format!("manifest parse failed: {error}"))?;
    validate_manifest(&manifest)?;

    let actual_commit = git_text(root, &["rev-parse".to_owned(), "HEAD".to_owned()])?;
    if actual_commit != manifest.source_commit {
        return Err(format!(
            "source checkout commit mismatch: expected {}, got {actual_commit}",
            manifest.source_commit
        ));
    }
    let schema_blob = git_text(root, &[
        "rev-parse".to_owned(),
        format!("HEAD:{}", manifest.schema_path),
    ])?;
    if schema_blob != manifest.schema_git_blob_sha1 {
        return Err(format!(
            "schema Git blob identity mismatch: expected {}, got {schema_blob}",
            manifest.schema_git_blob_sha1
        ));
    }

    let schema_path = root.join(&manifest.schema_path);
    let schema_bytes = fs::read(&schema_path).map_err(|error| {
        format!("cannot read pinned schema {}: {error}", schema_path.display())
    })?;
    if schema_bytes.len() != manifest.schema_size_bytes {
        return Err(format!("schema byte size mismatch: expected {}, got {}", manifest.schema_size_bytes, schema_bytes.len()));
    }
    let schema_json: serde_json::Value = serde_json::from_slice(&schema_bytes)
        .map_err(|error| format!("source schema is not valid JSON: {error}"))?;
    if schema_json.get("start_date").and_then(|v| v.as_str()) != Some(manifest.start_date.as_str())
        || schema_json.get("simulation_start_time_step").and_then(|v| v.as_u64()) != Some(manifest.simulation_start_step as u64)
        || schema_json.get("simulation_end_time_step").and_then(|v| v.as_u64()) != Some(manifest.simulation_end_step as u64)
        || schema_json.get("seconds_per_time_step").and_then(|v| v.as_u64()) != Some(manifest.seconds_per_time_step as u64)
        || schema_json.get("buildings").and_then(|v| v.as_object()).map(|v| v.len()) != Some(manifest.buildings)
    {
        return Err("actual upstream schema disagrees with pinned manifest metadata".to_owned());
    }

    let mut receipts = Vec::with_capacity(manifest.files.len());
    let mut total_bytes = 0usize;
    for entry in &manifest.files {
        let actual_blob = git_text(root, &[
            "rev-parse".to_owned(),
            format!("HEAD:{}", entry.path),
        ])?;
        if actual_blob != entry.git_blob_sha1 {
            return Err(format!(
                "Git blob identity mismatch for {}: expected {}, got {actual_blob}",
                entry.path, entry.git_blob_sha1
            ));
        }

        let path = root.join(&entry.path);
        let bytes = fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if bytes.len() != entry.size_bytes {
            return Err(format!("size mismatch for {}: expected {}, got {}", entry.path, entry.size_bytes, bytes.len()));
        }
        let record_count = csv_record_count(&bytes);
        let data_rows = record_count.checked_sub(1)
            .ok_or_else(|| format!("CSV {} has no header record", entry.path))?;
        if data_rows != entry.data_rows {
            return Err(format!("row count mismatch for {}: expected {}, got {}", entry.path, entry.data_rows, data_rows));
        }
        total_bytes = total_bytes.checked_add(bytes.len())
            .ok_or_else(|| "source byte total overflow".to_owned())?;
        receipts.push(FileReceipt {
            path: entry.path.clone(),
            git_blob_sha1: entry.git_blob_sha1.clone(),
            size_bytes: bytes.len(),
            data_rows,
            sha256: sha256_hex(&bytes),
        });
    }

    Ok(Report {
        schema_version: "citylearn-source-snapshot-audit-report-v0.1",
        status: "PASS_PINNED_SOURCE_BYTES_ONLY",
        dataset_id: manifest.dataset_id,
        source_commit: manifest.source_commit,
        source_manifest_sha256: actual_manifest_sha,
        schema_path: manifest.schema_path,
        schema_git_blob_sha1: manifest.schema_git_blob_sha1,
        time_steps: manifest.time_steps,
        seconds_per_time_step: manifest.seconds_per_time_step,
        buildings: manifest.buildings,
        file_count: receipts.len(),
        total_source_bytes_verified: total_bytes,
        source_commit_verified: actual_commit == SOURCE_COMMIT,
        schema_git_blob_identity_verified: schema_blob == manifest.schema_git_blob_sha1,
        source_file_git_blob_identities_verified: receipts.len() == manifest.referenced_csv_count,
        raw_sha256_receipts_generated: receipts.len() == 30 && receipts.iter().all(|receipt| valid_sha1(&receipt.git_blob_sha1) && receipt.sha256.len() == 64),
        dataset_terms_cleared: false,
        forecast_issue_times_verified: false,
        source_bytes_committed_to_symtropy: false,
        files: receipts,
        claim_ceiling: "Pinned-source byte integrity only; no dataset-rights clearance, causal forecast qualification, simulator control result, field result or SOTA claim.",
    })
}

fn main() {
    let mut args = env::args().skip(1);
    let root = args.next();
    let output_path = args.next();
    if root.is_none() || output_path.is_none() || args.next().is_some() {
        eprintln!("usage: energy_citylearn_source_snapshot_audit <citylearn-checkout-root> <output-report.json>");
        std::process::exit(2);
    }
    let root = root.unwrap();
    let output_path = output_path.unwrap();
    match verify_snapshot(Path::new(&root)) {
        Ok(report) => {
            let serialized = match serde_json::to_vec_pretty(&report) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("FAIL report serialization: {error}");
                    std::process::exit(2);
                }
            };
            if let Err(error) = fs::write(&output_path, &serialized) {
                eprintln!("FAIL writing report {}: {}", output_path, error);
                std::process::exit(2);
            }
            println!("PASS pinned CityLearn source byte audit");
            println!("source commit: {}", report.source_commit);
            println!("files verified: {}", report.file_count);
            println!("source bytes verified: {}", report.total_source_bytes_verified);
            println!("source commit verified: {}", report.source_commit_verified);
            println!("schema and file Git blobs verified: {}", report.schema_git_blob_identity_verified && report.source_file_git_blob_identities_verified);
            println!("raw SHA-256 receipts generated: {}", report.raw_sha256_receipts_generated);
            println!("dataset terms cleared: {}", report.dataset_terms_cleared);
            println!("forecast issue times verified: {}", report.forecast_issue_times_verified);
            println!("source data committed to Symtropy: {}", report.source_bytes_committed_to_symtropy);
        }
        Err(error) => {
            eprintln!("FAIL pinned CityLearn source byte audit: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_csv_records_with_quoted_newlines_and_escaped_quotes() {
        let csv = b"a,b\r\n1,\"hello\nworld\"\r\n2,\"say \"\"hi\"\"\"\r\n";
        assert_eq!(csv_record_count(csv), 3);
    }

    #[test]
    fn counts_csv_records_without_final_newline() {
        let csv = b"a,b\n1,2\n3,4";
        assert_eq!(csv_record_count(csv), 3);
    }

    #[test]
    fn rejects_mismatched_commit_or_claimed_dataset_qualification() {
        let mut manifest: Manifest = serde_json::from_slice(MANIFEST_BYTES).unwrap();
        manifest.source_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
        assert!(validate_manifest(&manifest).unwrap_err().contains("manifest changed"));

        let mut manifest: Manifest = serde_json::from_slice(MANIFEST_BYTES).unwrap();
        manifest.dataset_license_status = "rights_reviewed_and_cleared".to_owned();
        assert!(validate_manifest(&manifest).is_err());
    }
}
