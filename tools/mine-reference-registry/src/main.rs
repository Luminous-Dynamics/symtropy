use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const EXPECTED_ENTRIES: usize = 6;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RecordKind {
    DatasetCatalogue,
    FormatSpecification,
    ResearchArticle,
    Dataset,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LicenseState {
    VerifiedOpen,
    RestrictedTerms,
    Unverified,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AccessState {
    MetadataOnly,
    Downloaded,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_id: String,
    schema_version: String,
    registry_date: String,
    evidence_class: String,
    claim_ceiling: String,
    entries: Vec<SourceEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceEntry {
    source_id: String,
    record_kind: RecordKind,
    title: String,
    canonical_url: String,
    citation: String,
    upstream_version: String,
    license_state: LicenseState,
    #[serde(default)]
    license_name: Option<String>,
    redistribution_allowed: bool,
    access_state: AccessState,
    #[serde(default)]
    source_sha256: Option<String>,
    review_note: String,
}

#[derive(Debug, Clone, Serialize)]
struct EntryReceipt {
    source_id: String,
    record_kind: RecordKind,
    license_state: LicenseState,
    redistribution_allowed: bool,
    access_state: AccessState,
    source_hash_present: bool,
    review_required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    result: String,
    schema_id: String,
    schema_version: String,
    validator_version: String,
    source_commit: String,
    registry_sha256: String,
    entry_count: usize,
    expected_entries_matched: usize,
    metadata_only_count: usize,
    redistribution_allowed_count: usize,
    license_review_required_count: usize,
    source_data_hashes_present: usize,
    entries: Vec<EntryReceipt>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }

    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && day >= 1 && day <= days_in_month
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value == value.to_ascii_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(char::is_whitespace)
        && value.len() > 12
}

fn validate_registry(registry: &Registry) -> Check<()> {
    if registry.schema_id != "MINE-REFERENCE-001"
        || registry.schema_version != VERSION
        || registry.evidence_class != "source_metadata_only"
        || registry.entries.len() != EXPECTED_ENTRIES
        || !is_iso_date(&registry.registry_date)
        || !nonempty(&registry.claim_ceiling)
    {
        return Err(
            "invalid registry identity, version, date, evidence class, or entry count".to_owned(),
        );
    }

    let mut ids = BTreeSet::new();
    for entry in &registry.entries {
        if !nonempty(&entry.source_id)
            || !ids.insert(entry.source_id.clone())
            || !nonempty(&entry.title)
            || !validate_url(&entry.canonical_url)
            || !nonempty(&entry.citation)
            || !nonempty(&entry.upstream_version)
            || !nonempty(&entry.review_note)
        {
            return Err(
                "source identity, URL, citation, version, note, or uniqueness check failed"
                    .to_owned(),
            );
        }

        match entry.license_state {
            LicenseState::VerifiedOpen => {
                if !entry
                    .license_name
                    .as_ref()
                    .is_some_and(|name| nonempty(name))
                {
                    return Err(format!(
                        "{}: verified-open record requires a specific license name",
                        entry.source_id
                    ));
                }
            }
            LicenseState::RestrictedTerms | LicenseState::Unverified => {
                if entry.redistribution_allowed {
                    return Err(format!(
                        "{}: unknown/restricted rights cannot permit redistribution",
                        entry.source_id
                    ));
                }
            }
        }

        match entry.access_state {
            AccessState::MetadataOnly => {
                if entry.source_sha256.is_some() {
                    return Err(format!(
                        "{}: source hash requires recorded downloaded bytes",
                        entry.source_id
                    ));
                }
            }
            AccessState::Downloaded => {
                let hash = entry.source_sha256.as_deref().ok_or_else(|| {
                    format!(
                        "{}: downloaded record must bind a source hash",
                        entry.source_id
                    )
                })?;
                if !is_lower_hex(hash, 64) {
                    return Err(format!(
                        "{}: source SHA-256 must be 64 lowercase hexadecimal characters",
                        entry.source_id
                    ));
                }
                if !entry.redistribution_allowed
                    || entry.license_state != LicenseState::VerifiedOpen
                {
                    return Err(format!(
                        "{}: downloaded source cannot be admitted to the repository without verified redistribution rights",
                        entry.source_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn source_commit_from_args(args: &[String]) -> Check<String> {
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: mine-reference-registry <registry.json> --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    let commit = args[3].to_ascii_lowercase();
    if !is_lower_hex(&commit, 40) {
        return Err("source commit must be a 40-character hexadecimal SHA".to_owned());
    }
    Ok(commit)
}

fn evaluate(registry: &Registry, source_commit: String, registry_bytes: &[u8]) -> Receipt {
    let entries: Vec<EntryReceipt> = registry
        .entries
        .iter()
        .map(|entry| EntryReceipt {
            source_id: entry.source_id.clone(),
            record_kind: entry.record_kind,
            license_state: entry.license_state,
            redistribution_allowed: entry.redistribution_allowed,
            access_state: entry.access_state,
            source_hash_present: entry.source_sha256.is_some(),
            review_required: entry.license_state != LicenseState::VerifiedOpen,
        })
        .collect();

    Receipt {
        result: "validated".to_owned(),
        schema_id: registry.schema_id.clone(),
        schema_version: registry.schema_version.clone(),
        validator_version: VERSION.to_owned(),
        source_commit,
        registry_sha256: format!("{:x}", Sha256::digest(registry_bytes)),
        entry_count: entries.len(),
        expected_entries_matched: entries.len(),
        metadata_only_count: entries
            .iter()
            .filter(|entry| entry.access_state == AccessState::MetadataOnly)
            .count(),
        redistribution_allowed_count: entries
            .iter()
            .filter(|entry| entry.redistribution_allowed)
            .count(),
        license_review_required_count: entries.iter().filter(|entry| entry.review_required).count(),
        source_data_hashes_present: entries
            .iter()
            .filter(|entry| entry.source_hash_present)
            .count(),
        entries,
    }
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().collect();
    let source_commit = source_commit_from_args(&args)?;
    let bytes = fs::read(&args[1]).map_err(|error| format!("cannot read registry: {error}"))?;
    let registry: Registry = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid registry JSON: {error}"))?;
    validate_registry(&registry)?;
    let receipt = evaluate(&registry, source_commit, &bytes);
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize receipt: {error}"))?
    );
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        serde_json::from_str(include_str!(
            "../../../docs/tech/mine-reference-registry-v0.1.json"
        ))
        .expect("frozen reference registry should deserialize")
    }

    #[test]
    fn frozen_registry_matches_entry_count() {
        let registry = registry();
        validate_registry(&registry).expect("reference registry must validate");
        assert_eq!(EXPECTED_ENTRIES, registry.entries.len());
        assert!(registry
            .entries
            .iter()
            .all(|entry| entry.access_state == AccessState::MetadataOnly));
        assert!(registry
            .entries
            .iter()
            .all(|entry| entry.source_sha256.is_none()));
    }

    #[test]
    fn date_validation_rejects_impossible_calendar_dates() {
        assert!(is_iso_date("2024-02-29"));
        assert!(!is_iso_date("2026-02-29"));
        assert!(!is_iso_date("2026-13-01"));
        assert!(!is_iso_date("2026-04-31"));
    }

    #[test]
    fn unverified_or_restricted_sources_cannot_be_redistributable() {
        let mut registry = registry();
        let entry = registry
            .entries
            .iter_mut()
            .find(|entry| entry.source_id == "lotsu-2026-zenodo-artifacts")
            .expect("MineLib entry exists");
        entry.redistribution_allowed = true;
        assert!(validate_registry(&registry).is_err());
    }

    #[test]
    fn downloaded_bytes_require_hash_and_verified_rights() {
        let mut registry = registry();
        let entry = registry
            .entries
            .iter_mut()
            .find(|entry| entry.source_id == "lotsu-2026-zenodo-artifacts")
            .expect("artifact entry exists");
        entry.access_state = AccessState::Downloaded;
        entry.source_sha256 = Some("a".repeat(64));
        assert!(validate_registry(&registry).is_err());
    }

    #[test]
    fn downloaded_hash_requires_sha256_shape() {
        let mut registry = registry();
        let entry = registry
            .entries
            .iter_mut()
            .find(|entry| entry.source_id == "lotsu-2026-production-scheduling-article")
            .expect("article entry exists");
        entry.access_state = AccessState::Downloaded;
        entry.source_sha256 = Some("bad-hash".to_owned());
        assert!(validate_registry(&registry).is_err());
    }

    #[test]
    fn duplicate_source_identity_fails_closed() {
        let mut registry = registry();
        let repeated_id = registry.entries[0].source_id.clone();
        registry.entries[1].source_id = repeated_id;
        assert!(validate_registry(&registry).is_err());
    }

    #[test]
    fn unknown_json_fields_fail_closed() {
        let raw = include_str!("../../../docs/tech/mine-reference-registry-v0.1.json").replacen(
            "\"schema_id\":",
            "\"unreviewed_authority\":true,\"schema_id\":",
            1,
        );
        assert!(serde_json::from_str::<Registry>(&raw).is_err());
    }
}
