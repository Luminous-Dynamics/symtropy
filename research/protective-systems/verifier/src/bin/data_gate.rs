//! Fail-closed integrity gate for the registered NIST V50 archive.
//!
//! This binary does not download, extract, parse, or analyse the dataset. It
//! verifies the local archive bytes against the checksum published by NIST
//! after validating the rights-aware registry entry.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    process,
    sync::atomic::{AtomicU64, Ordering},
};

const REGISTRY_SCHEMA: &str = "protective-data-registry/0.1";
const NIST_RECORD_ID: &str = "NIST-M32149-V50";
const NIST_LICENSE_URI: &str = "https://www.nist.gov/open/license";

const EMBEDDED_REGISTRY: &str =
    include_str!("../../../fixtures/body-armor-public-datasets-v0.1.json");

#[derive(Debug, Serialize)]
struct GateReport {
    dataset_id: &'static str,
    archive_path: String,
    archive_bytes: u64,
    expected_sha256: String,
    actual_sha256: String,
    checksum_matches: bool,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nist_archive_metadata(registry_text: &str) -> Result<(String, String), String> {
    let root: Value = serde_json::from_str(registry_text)
        .map_err(|error| format!("invalid registry JSON: {error}"))?;

    if root.get("schema_version").and_then(Value::as_str) != Some(REGISTRY_SCHEMA) {
        return Err(format!("registry schema_version must equal {REGISTRY_SCHEMA}"));
    }

    let records = root
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry records must be an array".to_owned())?;

    let matching: Vec<&Value> = records
        .iter()
        .filter(|record| {
            record.get("record_id").and_then(Value::as_str) == Some(NIST_RECORD_ID)
        })
        .collect();

    if matching.len() != 1 {
        return Err(format!(
            "registry must contain exactly one {NIST_RECORD_ID} record"
        ));
    }

    let record = matching[0];
    if record.get("access_level").and_then(Value::as_str) != Some("public") {
        return Err("NIST record is not marked public in the registry".to_owned());
    }
    if record.get("reuse_status").and_then(Value::as_str) != Some("NIST_OPEN_LICENSE") {
        return Err("NIST record is not marked with its declared NIST reuse status".to_owned());
    }
    if record.get("license_uri").and_then(Value::as_str) != Some(NIST_LICENSE_URI) {
        return Err("NIST license URI is missing or changed".to_owned());
    }

    let archive_name = record
        .get("archive_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "NIST archive_name is missing".to_owned())?
        .to_owned();

    let expected_sha256 = record
        .get("archive_sha256_published_by_repository")
        .and_then(Value::as_str)
        .filter(|value| is_sha256(value))
        .ok_or_else(|| "NIST published archive SHA-256 is missing or malformed".to_owned())?
        .to_ascii_lowercase();

    Ok((archive_name, expected_sha256))
}

fn sha256_reader<R: Read>(mut reader: R) -> io::Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut byte_count = 0_u64;

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        byte_count += read as u64;
    }

    Ok((format!("{:x}", hasher.finalize()), byte_count))
}

fn run(archive_path: &str) -> Result<GateReport, String> {
    // Compile-time inclusion binds the integrity check to the reviewed registry
    // that was present in this exact source revision.
    let (registered_archive_name, expected_sha256) =
        nist_archive_metadata(EMBEDDED_REGISTRY)?;

    let supplied_archive_name = std::path::Path::new(archive_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "archive path must end in a UTF-8 file name".to_owned())?;

    if supplied_archive_name != registered_archive_name {
        return Err(format!(
            "expected archive file name {registered_archive_name}, received {supplied_archive_name}"
        ));
    }

    let file = File::open(archive_path)
        .map_err(|error| format!("cannot open archive {archive_path}: {error}"))?;
    let (actual_sha256, archive_bytes) = sha256_reader(file)
        .map_err(|error| format!("cannot hash archive {archive_path}: {error}"))?;

    Ok(GateReport {
        dataset_id: NIST_RECORD_ID,
        archive_path: archive_path.to_owned(),
        archive_bytes,
        checksum_matches: actual_sha256 == expected_sha256,
        expected_sha256,
        actual_sha256,
    })
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: protective-data-gate <V50.zip>");
        process::exit(2);
    }

    let report = match run(&args[0]) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("gate report serialization")
    );

    if !report.checksum_matches {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn sha256_known_answer_is_correct() {
        let (digest, bytes) = sha256_reader(Cursor::new(b"abc")).expect("hash bytes");
        assert_eq!(bytes, 3);
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn embedded_registry_has_one_eligible_nist_record() {
        let (archive_name, digest) =
            nist_archive_metadata(EMBEDDED_REGISTRY).expect("valid NIST registry record");
        assert_eq!(archive_name, "V50.zip");
        assert_eq!(
            digest,
            "ba8ee6071849d5883af434d1bda46727f695ef64d446fdbf8f26a0a653f4d66b"
        );
    }

    #[test]
    fn malformed_published_digest_fails_closed() {
        let mut registry: Value =
            serde_json::from_str(EMBEDDED_REGISTRY).expect("embedded registry JSON");
        let record = registry["records"]
            .as_array_mut()
            .expect("records array")
            .iter_mut()
            .find(|record| record["record_id"] == NIST_RECORD_ID)
            .expect("NIST record");
        record["archive_sha256_published_by_repository"] = Value::String("not-a-digest".into());

        assert!(nist_archive_metadata(&registry.to_string()).is_err());
    }

    #[test]
    fn wrong_archive_filename_is_rejected_before_file_access() {
        let error = run("not-the-registered-file.zip").expect_err("wrong filename must fail");
        assert!(error.contains("expected archive file name V50.zip"));
    }

    #[test]
    fn mismatched_archive_digest_is_reported_without_promotion() {
        static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
        let temp_dir = env::temp_dir().join(format!(
            "protective-data-gate-{}-{}",
            process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&temp_dir).expect("create temporary test directory");
        let archive = temp_dir.join("V50.zip");
        fs::write(&archive, b"synthetic negative control; not the NIST V50 archive")
            .expect("write synthetic negative-control bytes");

        let report = run(archive.to_str().expect("UTF-8 temporary path"))
            .expect("synthetic archive can be hashed");
        assert!(!report.checksum_matches);
        assert_ne!(report.actual_sha256, report.expected_sha256);

        fs::remove_dir_all(&temp_dir).expect("remove temporary test directory");
    }

    #[test]
    fn duplicate_nist_registry_entries_are_rejected() {
        let mut registry: Value =
            serde_json::from_str(EMBEDDED_REGISTRY).expect("embedded registry JSON");
        let first = registry["records"]
            .as_array()
            .expect("records array")[0]
            .clone();
        registry["records"]
            .as_array_mut()
            .expect("records array")
            .push(first);
        assert!(nist_archive_metadata(&registry.to_string()).is_err());
    }

    #[test]
    fn unlicensed_zenodo_candidate_is_not_accepted_as_nist_input() {
        let registry: Value =
            serde_json::from_str(EMBEDDED_REGISTRY).expect("embedded registry JSON");
        let candidate = registry["records"]
            .as_array()
            .expect("records array")
            .iter()
            .find(|record| record["record_id"] == "ZENODO-23313293-CFRP")
            .expect("Zenodo candidate");

        assert_eq!(
            candidate["benchmark_disposition"],
            "EXCLUDED_PENDING_RIGHTS_AND_PERMITTED_USE_REVIEW"
        );
        assert_eq!(candidate["license_status_on_record"], "to be confirmed");
    }
}
