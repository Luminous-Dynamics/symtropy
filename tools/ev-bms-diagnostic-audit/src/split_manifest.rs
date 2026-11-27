use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MANIFEST_VERSION: &str = "ev-bms-split-manifest/v0.1";
const SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/transport/ev-bms-split-manifest/0.1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitManifest {
    manifest_version: String,
    split_id: String,
    dataset_id: String,
    dataset_domain: String,
    dataset_admission_state: String,
    dataset_manifest_sha256: String,
    dataset_snapshot_sha256: Option<String>,
    evidence_class: String,
    split_strategy: String,
    group_key: String,
    leakage_keys: Vec<String>,
    temporal_exclusion_keys: Vec<String>,
    split_seed: u64,
    generator_revision: String,
    partitions: Vec<SplitPartition>,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitPartition {
    name: String,
    records: Vec<SplitRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitRecord {
    record_id: String,
    record_sha256: String,
    group_values: BTreeMap<String, String>,
    window_start_unix_ms: u64,
    window_end_unix_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct PartitionSummary {
    pub partition: String,
    pub record_count: usize,
    pub primary_group_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SplitAuditReport {
    pub report_version: String,
    pub split_id: String,
    pub dataset_id: String,
    pub dataset_domain: String,
    pub dataset_admission_state: String,
    pub dataset_manifest_sha256: String,
    pub dataset_snapshot_sha256: Option<String>,
    pub evidence_class: String,
    pub split_strategy: String,
    pub group_key: String,
    pub record_count: usize,
    pub primary_group_count: usize,
    pub partition_summaries: Vec<PartitionSummary>,
    pub canonical_split_manifest_sha256: String,
    pub schema_sha256: String,
    pub claim_ceiling: String,
    pub report_sha256: String,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn canonicalize(value: &mut Value) {
    if let Some(partitions) = value.get_mut("partitions").and_then(Value::as_array_mut) {
        for partition in partitions.iter_mut() {
            if let Some(records) = partition.get_mut("records").and_then(Value::as_array_mut) {
                records.sort_by(|left, right| {
                    left.get("record_id")
                        .and_then(Value::as_str)
                        .cmp(&right.get("record_id").and_then(Value::as_str))
                });
            }
        }
        partitions.sort_by(|left, right| {
            left.get("name")
                .and_then(Value::as_str)
                .cmp(&right.get("name").and_then(Value::as_str))
        });
    }
}

fn validate_group_key(domain: &str, key: &str) -> Result<(), String> {
    let allowed = match domain {
        "onboard_bms" => ["vehicle_id", "manufacturer_group"].as_slice(),
        "charger_side" => {
            ["vehicle_id", "station_id", "data_owner_group"].as_slice()
        }
        _ => return Err("unsupported source domain".to_owned()),
    };
    if !allowed.contains(&key) {
        return Err(format!("group key {key} is not valid for source domain {domain}"));
    }
    Ok(())
}

fn validate_manifest(manifest: &SplitManifest) -> Result<SplitAuditReport, String> {
    if manifest.manifest_version != MANIFEST_VERSION {
        return Err("unsupported split-manifest version".to_owned());
    }
    for (field, value) in [
        ("split_id", manifest.split_id.as_str()),
        ("dataset_id", manifest.dataset_id.as_str()),
        ("group_key", manifest.group_key.as_str()),
        ("generator_revision", manifest.generator_revision.as_str()),
        ("claim_ceiling", manifest.claim_ceiling.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{field} must not be empty"));
        }
    }
    if !is_sha256(&manifest.dataset_manifest_sha256) {
        return Err("dataset_manifest_sha256 must be a lowercase SHA-256 digest".to_owned());
    }
    if manifest.split_strategy != "group_disjoint_with_sequence_guards" {
        return Err("unsupported split strategy".to_owned());
    }
    if manifest.dataset_admission_state == "snapshot_admitted" {
        if manifest.evidence_class != "operator_collected_field_observation"
            || manifest
                .dataset_snapshot_sha256
                .as_deref()
                .is_none_or(|digest| !is_sha256(digest))
        {
            return Err(
                "snapshot-admitted splits require field evidence and a snapshot SHA-256"
                    .to_owned(),
            );
        }
    } else if manifest.dataset_admission_state == "metadata_only" {
        if manifest.evidence_class != "synthetic_fixture"
            || manifest.dataset_snapshot_sha256.is_some()
        {
            return Err(
                "metadata-only source manifests may only use synthetic split fixtures"
                    .to_owned(),
            );
        }
    } else {
        return Err("unknown dataset admission state".to_owned());
    }

    validate_group_key(&manifest.dataset_domain, &manifest.group_key)?;
    if manifest.partitions.len() != 3 {
        return Err("split manifest must contain exactly train, calibration and test".to_owned());
    }

    let mut leakage_keys = BTreeSet::new();
    for key in &manifest.leakage_keys {
        if !nonempty(key) || key == &manifest.group_key || !leakage_keys.insert(key) {
            return Err("leakage keys must be non-empty, unique and distinct from group_key".to_owned());
        }
    }
    let mut temporal_keys = BTreeSet::new();
    for key in &manifest.temporal_exclusion_keys {
        if !nonempty(key)
            || !temporal_keys.insert(key)
            || key == &manifest.group_key
        {
            return Err("temporal exclusion keys must be non-empty, unique and distinct from group_key".to_owned());
        }
    }
    if manifest.leakage_keys.is_empty() {
        return Err("at least one leakage key is required".to_owned());
    }

    let expected_partitions = ["train", "calibration", "test"];
    let mut partition_names = BTreeSet::new();
    let mut record_ids = BTreeSet::new();
    let mut record_digests = BTreeSet::new();
    let mut primary_group_owners: BTreeMap<String, String> = BTreeMap::new();
    let mut leakage_owners: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut temporal_windows: BTreeMap<
        (String, String),
        Vec<(u64, u64, String)>,
    > = BTreeMap::new();
    let mut partition_summaries = Vec::new();
    let mut total_records = 0usize;
    let mut all_primary_groups = BTreeSet::new();

    for partition in &manifest.partitions {
        if !expected_partitions.contains(&partition.name.as_str())
            || !partition_names.insert(partition.name.clone())
        {
            return Err("partition names must be exactly train, calibration and test".to_owned());
        }
        if partition.records.is_empty() {
            return Err(format!("partition {} must not be empty", partition.name));
        }

        let mut local_primary_groups = BTreeSet::new();
        for record in &partition.records {
            if !nonempty(&record.record_id)
                || !record_ids.insert(record.record_id.clone())
            {
                return Err("record identifiers must be non-empty and globally unique".to_owned());
            }
            if !is_sha256(&record.record_sha256)
                || !record_digests.insert(record.record_sha256.clone())
            {
                return Err("record digests must be valid and globally unique".to_owned());
            }
            if record.window_start_unix_ms >= record.window_end_unix_ms {
                return Err(format!(
                    "record {} has an invalid half-open time window",
                    record.record_id
                ));
            }

            let primary_value = record
                .group_values
                .get(&manifest.group_key)
                .filter(|value| nonempty(value))
                .ok_or_else(|| {
                    format!(
                        "record {} is missing primary group key {}",
                        record.record_id, manifest.group_key
                    )
                })?;
            local_primary_groups.insert(primary_value.clone());
            all_primary_groups.insert(primary_value.clone());
            if let Some(owner) = primary_group_owners.get(primary_value) {
                if owner != &partition.name {
                    return Err(format!(
                        "primary group {} crosses {} and {}",
                        primary_value, owner, partition.name
                    ));
                }
            } else {
                primary_group_owners.insert(primary_value.clone(), partition.name.clone());
            }

            for key in &manifest.leakage_keys {
                let value = record
                    .group_values
                    .get(key)
                    .filter(|value| nonempty(value))
                    .ok_or_else(|| {
                        format!(
                            "record {} is missing required leakage key {}",
                            record.record_id, key
                        )
                    })?;
                let index = (key.clone(), value.clone());
                if let Some(owner) = leakage_owners.get(&index) {
                    if owner != &partition.name {
                        return Err(format!(
                            "leakage key {}={} crosses {} and {}",
                            key, value, owner, partition.name
                        ));
                    }
                } else {
                    leakage_owners.insert(index, partition.name.clone());
                }
            }

            for key in &manifest.temporal_exclusion_keys {
                let value = record
                    .group_values
                    .get(key)
                    .filter(|value| nonempty(value))
                    .ok_or_else(|| {
                        format!(
                            "record {} is missing temporal exclusion key {}",
                            record.record_id, key
                        )
                    })?;
                let index = (key.clone(), value.clone());
                let windows = temporal_windows.entry(index).or_default();
                for (start, end, owner) in windows.iter() {
                    let overlaps = record.window_start_unix_ms < *end
                        && *start < record.window_end_unix_ms;
                    if overlaps && owner != &partition.name {
                        return Err(format!(
                            "overlapping time windows for {}={} cross {} and {}",
                            key, value, owner, partition.name
                        ));
                    }
                }
                windows.push((
                    record.window_start_unix_ms,
                    record.window_end_unix_ms,
                    partition.name.clone(),
                ));
            }
            total_records += 1;
        }

        partition_summaries.push(PartitionSummary {
            partition: partition.name.clone(),
            record_count: partition.records.len(),
            primary_group_count: local_primary_groups.len(),
        });
    }

    for name in expected_partitions {
        if !partition_names.contains(name) {
            return Err(format!("required partition {name} is missing"));
        }
    }
    partition_summaries.sort_by(|left, right| left.partition.cmp(&right.partition));

    Ok(SplitAuditReport {
        report_version: "ev-bms-split-audit-report/v0.1".to_owned(),
        split_id: manifest.split_id.clone(),
        dataset_id: manifest.dataset_id.clone(),
        dataset_domain: manifest.dataset_domain.clone(),
        dataset_admission_state: manifest.dataset_admission_state.clone(),
        dataset_manifest_sha256: manifest.dataset_manifest_sha256.clone(),
        dataset_snapshot_sha256: manifest.dataset_snapshot_sha256.clone(),
        evidence_class: manifest.evidence_class.clone(),
        split_strategy: manifest.split_strategy.clone(),
        group_key: manifest.group_key.clone(),
        record_count: total_records,
        primary_group_count: all_primary_groups.len(),
        partition_summaries,
        canonical_split_manifest_sha256: String::new(),
        schema_sha256: String::new(),
        claim_ceiling: manifest.claim_ceiling.clone(),
        report_sha256: String::new(),
    })
}

pub fn audit_split_manifest(
    schema_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<SplitAuditReport, String> {
    let schema: Value =
        serde_json::from_slice(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("split schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("split schema identity does not match v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("split schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("split schema compilation: {error}"))?;

    let mut value: Value =
        serde_json::from_slice(manifest_bytes).map_err(|error| format!("manifest JSON: {error}"))?;
    canonicalize(&mut value);
    if let Err(error) = validator.validate(&value) {
        return Err(format!("split manifest schema validation: {error}"));
    }
    let canonical_bytes =
        serde_json::to_vec(&value).map_err(|error| format!("canonical split manifest: {error}"))?;
    let manifest: SplitManifest = serde_json::from_slice(&canonical_bytes)
        .map_err(|error| format!("typed split manifest: {error}"))?;
    let mut report = validate_manifest(&manifest)?;
    report.canonical_split_manifest_sha256 = sha256(&canonical_bytes);
    report.schema_sha256 = sha256(schema_bytes);
    let report_bytes =
        serde_json::to_vec(&report).map_err(|error| format!("split report serialization: {error}"))?;
    report.report_sha256 = sha256(&report_bytes);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] =
        include_bytes!("../../../schemas/transport/ev_bms_split_manifest_v0_1.schema.json");
    const CAO: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/cao_vehicle_split_manifest_v0_1.json");
    const SHENZHEN: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/shenzhen_station_split_manifest_v0_1.json");

    #[test]
    fn accepts_vehicle_disjoint_onboard_bms_split() {
        let report = audit_split_manifest(SCHEMA, CAO).expect("Cao vehicle split");
        assert_eq!(report.record_count, 5);
        assert_eq!(report.primary_group_count, 5);
        assert_eq!(report.group_key, "vehicle_id");
        assert_eq!(report.dataset_admission_state, "metadata_only");
        assert!(report.dataset_snapshot_sha256.is_none());
    }

    #[test]
    fn accepts_station_disjoint_charger_side_split_with_time_guard() {
        let report = audit_split_manifest(SCHEMA, SHENZHEN).expect("Shenzhen station split");
        assert_eq!(report.record_count, 6);
        assert_eq!(report.primary_group_count, 4);
        assert_eq!(report.group_key, "station_id");
        assert_eq!(report.partition_summaries.len(), 3);
    }

    #[test]
    fn rejects_primary_group_leakage_across_partitions() {
        let mut value: Value = serde_json::from_slice(CAO).expect("Cao split JSON");
        let train_group = value["partitions"][0]["records"][0]["group_values"]["vehicle_id"]
            .as_str()
            .unwrap()
            .to_owned();
        value["partitions"][2]["records"][0]["group_values"]["vehicle_id"] =
            Value::String(train_group);
        let bytes = serde_json::to_vec(&value).expect("mutated split JSON");
        assert!(audit_split_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_transaction_leakage_across_stations() {
        let mut value: Value = serde_json::from_slice(SHENZHEN).expect("Shenzhen split JSON");
        let train_transaction = value["partitions"][0]["records"][0]["group_values"]
            ["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        value["partitions"][2]["records"][0]["group_values"]["transaction_id"] =
            Value::String(train_transaction);
        let bytes = serde_json::to_vec(&value).expect("mutated split JSON");
        assert!(audit_split_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_overlapping_windows_for_same_vehicle_across_station_holdout() {
        let mut value: Value = serde_json::from_slice(SHENZHEN).expect("Shenzhen split JSON");
        value["partitions"][2]["records"][0]["window_start_unix_ms"] = Value::from(1500_u64);
        let bytes = serde_json::to_vec(&value).expect("mutated split JSON");
        assert!(audit_split_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_duplicate_records_and_invalid_time_windows() {
        let mut duplicate: Value = serde_json::from_slice(CAO).expect("Cao split JSON");
        let record = duplicate["partitions"][0]["records"][0].clone();
        duplicate["partitions"][1]["records"]
            .as_array_mut()
            .unwrap()
            .push(record);
        let bytes = serde_json::to_vec(&duplicate).expect("duplicate split JSON");
        assert!(audit_split_manifest(SCHEMA, &bytes).is_err());

        let mut invalid: Value = serde_json::from_slice(CAO).expect("Cao split JSON");
        invalid["partitions"][0]["records"][0]["window_end_unix_ms"] =
            invalid["partitions"][0]["records"][0]["window_start_unix_ms"].clone();
        let bytes = serde_json::to_vec(&invalid).expect("invalid split JSON");
        assert!(audit_split_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn canonical_manifest_digest_is_invariant_to_partition_and_record_order() {
        let first = audit_split_manifest(SCHEMA, CAO).expect("Cao split");
        let mut value: Value = serde_json::from_slice(CAO).expect("Cao split JSON");
        value["partitions"].as_array_mut().unwrap().reverse();
        for partition in value["partitions"].as_array_mut().unwrap() {
            partition["records"].as_array_mut().unwrap().reverse();
        }
        let bytes = serde_json::to_vec(&value).expect("permuted split JSON");
        let second = audit_split_manifest(SCHEMA, &bytes).expect("permuted Cao split");
        assert_eq!(
            first.canonical_split_manifest_sha256,
            second.canonical_split_manifest_sha256
        );
        assert_eq!(first.report_sha256, second.report_sha256);
    }
}
