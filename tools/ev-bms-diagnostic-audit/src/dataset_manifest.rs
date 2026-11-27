use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MANIFEST_VERSION: &str = "ev-bms-dataset-manifest/v0.1";
const SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/transport/ev-bms-dataset-manifest/0.1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetManifest {
    manifest_version: String,
    dataset_id: String,
    title: String,
    doi: String,
    source_record_url: String,
    source_repository: String,
    source_version: String,
    count_basis_asset: String,
    source_published_on: String,
    metadata_retrieved_on: String,
    license_spdx: String,
    evidence_class: String,
    source_domain: String,
    measurement_boundary: String,
    sample_interval_seconds: u64,
    counts: DatasetCounts,
    label_contract: LabelContract,
    asset_group_keys: Vec<String>,
    assets: Vec<AssetFile>,
    admission_state: String,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetCounts {
    sample_count: u64,
    sample_count_is_approximate: bool,
    vehicle_count: u64,
    session_count: Option<u64>,
    station_count: Option<u64>,
    manufacturer_group_count: Option<u64>,
    data_owner_group_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelContract {
    task: String,
    granularity: String,
    classes: Vec<String>,
    label_sources: Vec<String>,
    limitations: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetFile {
    name: String,
    source_md5: Option<String>,
    local_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DatasetAdmissionReport {
    pub report_version: String,
    pub dataset_id: String,
    pub doi: String,
    pub license_spdx: String,
    pub evidence_class: String,
    pub source_domain: String,
    pub measurement_boundary: String,
    pub sample_interval_seconds: u64,
    pub count_basis_asset: String,
    pub sample_count: u64,
    pub sample_count_is_approximate: bool,
    pub vehicle_count: u64,
    pub session_count: Option<u64>,
    pub station_count: Option<u64>,
    pub asset_count: usize,
    pub admission_state: String,
    pub schema_sha256: String,
    pub canonical_manifest_sha256: String,
    pub claim_ceiling: String,
    pub report_sha256: String,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_hex_digest(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn canonicalize_manifest(value: &mut Value) {
    if let Some(assets) = value.get_mut("assets").and_then(Value::as_array_mut) {
        assets.sort_by(|left, right| {
            left.get("name")
                .and_then(Value::as_str)
                .cmp(&right.get("name").and_then(Value::as_str))
        });
    }
}

fn validate_common(manifest: &DatasetManifest) -> Result<(), String> {
    if manifest.manifest_version != MANIFEST_VERSION {
        return Err("unsupported dataset-manifest version".to_owned());
    }
    for (field, value) in [
        ("dataset_id", manifest.dataset_id.as_str()),
        ("title", manifest.title.as_str()),
        ("doi", manifest.doi.as_str()),
        ("source_record_url", manifest.source_record_url.as_str()),
        ("source_repository", manifest.source_repository.as_str()),
        ("source_version", manifest.source_version.as_str()),
        ("count_basis_asset", manifest.count_basis_asset.as_str()),
        ("source_published_on", manifest.source_published_on.as_str()),
        ("metadata_retrieved_on", manifest.metadata_retrieved_on.as_str()),
        ("license_spdx", manifest.license_spdx.as_str()),
        ("source_domain", manifest.source_domain.as_str()),
        ("measurement_boundary", manifest.measurement_boundary.as_str()),
        ("claim_ceiling", manifest.claim_ceiling.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{field} must not be empty"));
        }
    }
    if manifest.evidence_class != "operator_collected_field_observation" {
        return Err(
            "dataset evidence class must remain operator-collected field observation".to_owned(),
        );
    }
    if manifest.license_spdx != "CC-BY-4.0" {
        return Err("v0.1 admission requires the source record's CC-BY-4.0 licence".to_owned());
    }
    if manifest.sample_interval_seconds == 0
        || manifest.counts.sample_count == 0
        || manifest.counts.vehicle_count == 0
    {
        return Err("sample interval, sample count and vehicle count must be positive".to_owned());
    }
    if manifest.assets.is_empty() {
        return Err("asset inventory must not be empty".to_owned());
    }
    if manifest.asset_group_keys.is_empty() {
        return Err("at least one physical grouping key is required".to_owned());
    }
    let mut groups = BTreeSet::new();
    for key in &manifest.asset_group_keys {
        if !nonempty(key) || !groups.insert(key) {
            return Err("asset grouping keys must be non-empty and unique".to_owned());
        }
    }

    let mut names = BTreeSet::new();
    for asset in &manifest.assets {
        if !nonempty(&asset.name) || !names.insert(&asset.name) {
            return Err("asset file names must be non-empty and unique".to_owned());
        }
        if asset
            .source_md5
            .as_deref()
            .is_some_and(|digest| !is_hex_digest(digest, 16))
        {
            return Err(format!("{} has a malformed publisher MD5", asset.name));
        }
        if asset
            .local_sha256
            .as_deref()
            .is_some_and(|digest| !is_hex_digest(digest, 32))
        {
            return Err(format!("{} has a malformed local SHA-256", asset.name));
        }
    }

    match manifest.admission_state.as_str() {
        "metadata_only" => {
            if manifest.assets.iter().any(|asset| asset.local_sha256.is_some()) {
                return Err(
                    "metadata_only manifests cannot claim locally verified file hashes".to_owned(),
                );
            }
        }
        "snapshot_admitted" => {
            if manifest.assets.iter().any(|asset| asset.local_sha256.is_none()) {
                return Err("snapshot_admitted requires a local SHA-256 for every asset".to_owned());
            }
        }
        _ => return Err("admission_state must be metadata_only or snapshot_admitted".to_owned()),
    }

    if manifest.label_contract.classes.is_empty()
        || manifest.label_contract.label_sources.is_empty()
        || manifest.label_contract.limitations.is_empty()
        || !nonempty(&manifest.label_contract.task)
        || !nonempty(&manifest.label_contract.granularity)
    {
        return Err(
            "label contract must describe task, granularity, classes, sources and limitations"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_cao_bms(manifest: &DatasetManifest) -> Result<(), String> {
    let counts = &manifest.counts;
    let expected_faults = [
        "thermal_runaway",
        "electrolyte_leakage",
        "internal_short_circuit",
        "excessive_aging",
    ];
    if manifest.doi != "10.5281/zenodo.10656500"
        || manifest.source_record_url != "https://zenodo.org/records/10656500"
        || manifest.source_repository != "Zenodo"
        || manifest.source_version != "v1"
        || manifest.source_published_on != "2024-02-14"
        || manifest.count_basis_asset != "all_seven_archive_parts"
        || manifest.source_domain != "onboard_bms"
        || manifest.measurement_boundary != "vehicle_bms"
        || manifest.sample_interval_seconds != 30
        || counts.vehicle_count != 515
        || counts.manufacturer_group_count != Some(3)
        || counts.session_count.is_some()
        || counts.station_count.is_some()
        || counts.data_owner_group_count.is_some()
        || counts.sample_count != 18_200_000
        || !counts.sample_count_is_approximate
        || manifest.assets.len() != 7
    {
        return Err("Cao onboard-BMS metadata does not match the pinned Zenodo record".to_owned());
    }
    if manifest.label_contract.task != "multi_class_fault_warning"
        || manifest.label_contract.granularity != "sample_with_vehicle_group"
        || expected_faults.iter().any(|fault| {
            !manifest
                .label_contract
                .classes
                .iter()
                .any(|item| item == fault)
        })
        || !manifest.asset_group_keys.iter().any(|key| key == "vehicle_id")
        || !manifest.asset_group_keys.iter().any(|key| key == "manufacturer_group")
    {
        return Err("Cao fault classes or physical grouping contract is incomplete".to_owned());
    }
    let expected = [
        ("DTI.zip", "400135e26d8bac65cc171e63f71fd5dd"),
        ("GIS.zip", "2b4d5f9d0d045bbe5bb0318ea84f7e7e"),
        ("QAS_1.zip", "a3d3a182c7c82df26073a53f698148c8"),
        ("QAS_2.zip", "9183ff01f6cf3e187716ca3a2d7f6b53"),
        ("QAS_3.zip", "7cb6410cac2c44890eaccd9d594887bf"),
        ("QAS_4.zip", "d138ef98e2fb77c12afc88a6043e3be8"),
        ("QAS_5.zip", "f420b38bab7aca514ab834cbfa0010b8"),
    ];
    for (name, md5) in expected {
        if !manifest.assets.iter().any(|asset| {
            asset.name == name && asset.source_md5.as_deref() == Some(md5)
        }) {
            return Err(format!("Cao publisher file inventory or MD5 mismatch for {name}"));
        }
    }
    Ok(())
}

fn validate_shenzhen_charging(manifest: &DatasetManifest) -> Result<(), String> {
    let counts = &manifest.counts;
    if manifest.doi != "10.17632/c7gg94tmvz.3"
        || manifest.source_record_url != "https://data.mendeley.com/datasets/c7gg94tmvz/3"
        || manifest.source_repository != "Mendeley Data"
        || manifest.source_version != "3"
        || manifest.source_published_on != "2024-10-08"
        || manifest.count_basis_asset != "processed_data_longer_than_30.xlsx"
        || manifest.source_domain != "charger_side"
        || manifest.measurement_boundary != "charging_station_metering"
        || manifest.sample_interval_seconds != 60
        || counts.vehicle_count != 10_154
        || counts.session_count != Some(21_175)
        || counts.station_count != Some(30)
        || counts.data_owner_group_count != Some(8)
        || counts.manufacturer_group_count.is_some()
        || counts.sample_count != 1_547_432
        || counts.sample_count_is_approximate
        || manifest.assets.len() != 2
    {
        return Err("Shenzhen charger-side metadata does not match Mendeley Data v3".to_owned());
    }
    if manifest.label_contract.task != "binary_fault_warning"
        || manifest.label_contract.granularity != "charging_session_sample"
        || !manifest.label_contract.classes.iter().any(|item| item == "normal")
        || !manifest.label_contract.classes.iter().any(|item| item == "fault")
        || !manifest.asset_group_keys.iter().any(|key| key == "vehicle_id")
        || !manifest.asset_group_keys.iter().any(|key| key == "station_id")
        || !manifest.asset_group_keys.iter().any(|key| key == "data_owner_group")
        || !manifest.asset_group_keys.iter().any(|key| key == "transaction_id")
    {
        return Err("Shenzhen label or group-disjoint split contract is incomplete".to_owned());
    }
    for name in ["processed_data.xlsx", "processed_data_longer_than_30.xlsx"] {
        if !manifest.assets.iter().any(|asset| asset.name == name) {
            return Err(format!("Shenzhen source file inventory is missing {name}"));
        }
    }
    Ok(())
}

pub fn audit_dataset_manifest(
    schema_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<DatasetAdmissionReport, String> {
    let schema: Value =
        serde_json::from_slice(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("dataset schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("dataset manifest schema identity does not match v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("dataset schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("dataset schema compilation: {error}"))?;

    let mut value: Value =
        serde_json::from_slice(manifest_bytes).map_err(|error| format!("manifest JSON: {error}"))?;
    canonicalize_manifest(&mut value);
    if let Err(error) = validator.validate(&value) {
        return Err(format!("manifest schema validation: {error}"));
    }
    let canonical_bytes =
        serde_json::to_vec(&value).map_err(|error| format!("canonical manifest: {error}"))?;
    let manifest: DatasetManifest = serde_json::from_slice(&canonical_bytes)
        .map_err(|error| format!("typed manifest: {error}"))?;

    validate_common(&manifest)?;
    match manifest.dataset_id.as_str() {
        "cao-onboard-bms-zenodo-10656500" => validate_cao_bms(&manifest)?,
        "shenzhen-charger-side-mendeley-c7gg94tmvz-v3" => {
            validate_shenzhen_charging(&manifest)?
        }
        _ => return Err("dataset_id is not an admitted v0.1 benchmark source".to_owned()),
    }

    let mut payload = DatasetAdmissionReport {
        report_version: "ev-bms-dataset-admission-report/v0.1".to_owned(),
        dataset_id: manifest.dataset_id,
        doi: manifest.doi,
        license_spdx: manifest.license_spdx,
        evidence_class: manifest.evidence_class,
        source_domain: manifest.source_domain,
        measurement_boundary: manifest.measurement_boundary,
        sample_interval_seconds: manifest.sample_interval_seconds,
        count_basis_asset: manifest.count_basis_asset,
        sample_count: manifest.counts.sample_count,
        sample_count_is_approximate: manifest.counts.sample_count_is_approximate,
        vehicle_count: manifest.counts.vehicle_count,
        session_count: manifest.counts.session_count,
        station_count: manifest.counts.station_count,
        asset_count: manifest.assets.len(),
        admission_state: manifest.admission_state,
        schema_sha256: sha256(schema_bytes),
        canonical_manifest_sha256: sha256(&canonical_bytes),
        claim_ceiling: manifest.claim_ceiling,
        report_sha256: String::new(),
    };
    let report_bytes = serde_json::to_vec(&payload)
        .map_err(|error| format!("dataset report serialization: {error}"))?;
    payload.report_sha256 = sha256(&report_bytes);
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] =
        include_bytes!("../../../schemas/transport/ev_bms_dataset_manifest_v0_1.schema.json");
    const CAO: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/cao_onboard_bms_manifest_v0_1.json");
    const SHENZHEN: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/shenzhen_charger_side_manifest_v0_1.json");

    #[test]
    fn admits_metadata_only_cao_onboard_bms_source_contract() {
        let report = audit_dataset_manifest(SCHEMA, CAO).expect("Cao metadata manifest");
        assert_eq!(report.sample_count, 18_200_000);
        assert!(report.sample_count_is_approximate);
        assert_eq!(report.vehicle_count, 515);
        assert_eq!(report.asset_count, 7);
        assert_eq!(report.admission_state, "metadata_only");
        assert_eq!(report.measurement_boundary, "vehicle_bms");
    }

    #[test]
    fn admits_metadata_only_shenzhen_charger_source_contract() {
        let report = audit_dataset_manifest(SCHEMA, SHENZHEN).expect("Shenzhen metadata manifest");
        assert_eq!(report.sample_count, 1_547_432);
        assert!(!report.sample_count_is_approximate);
        assert_eq!(report.vehicle_count, 10_154);
        assert_eq!(report.session_count, Some(21_175));
        assert_eq!(report.station_count, Some(30));
        assert_eq!(report.asset_count, 2);
        assert_eq!(report.measurement_boundary, "charging_station_metering");
    }

    #[test]
    fn rejects_snapshot_admission_without_local_file_hashes() {
        let mut value: Value = serde_json::from_slice(CAO).expect("Cao manifest JSON");
        value["admission_state"] = Value::String("snapshot_admitted".to_owned());
        let bytes = serde_json::to_vec(&value).expect("mutated manifest JSON");
        assert!(audit_dataset_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_asset_inventory_mutation_against_publisher_md5() {
        let mut value: Value = serde_json::from_slice(CAO).expect("Cao manifest JSON");
        value["assets"][0]["source_md5"] = Value::String("00".repeat(16));
        let bytes = serde_json::to_vec(&value).expect("mutated manifest JSON");
        assert!(audit_dataset_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_sample_interval_and_measurement_boundary_confusion() {
        let mut value: Value = serde_json::from_slice(SHENZHEN).expect("Shenzhen manifest JSON");
        value["sample_interval_seconds"] = Value::from(30);
        let bytes = serde_json::to_vec(&value).expect("mutated manifest JSON");
        assert!(audit_dataset_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_source_record_identity_mutation() {
        let mut value: Value = serde_json::from_slice(CAO).expect("Cao manifest JSON");
        value["source_record_url"] = Value::String("https://example.org/fake-record".to_owned());
        let bytes = serde_json::to_vec(&value).expect("mutated manifest JSON");
        assert!(audit_dataset_manifest(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_dataset_schema_identity_mutation() {
        let mut value: Value = serde_json::from_slice(SCHEMA).expect("schema JSON");
        value["$id"] = Value::String("https://example.org/not-this-schema".to_owned());
        let bytes = serde_json::to_vec(&value).expect("mutated schema JSON");
        assert!(audit_dataset_manifest(&bytes, CAO).is_err());
    }

    #[test]
    fn canonical_manifest_digest_is_invariant_to_asset_order() {
        let first = audit_dataset_manifest(SCHEMA, SHENZHEN).expect("Shenzhen manifest");
        let mut value: Value = serde_json::from_slice(SHENZHEN).expect("Shenzhen manifest JSON");
        value["assets"].as_array_mut().unwrap().reverse();
        let bytes = serde_json::to_vec(&value).expect("permuted manifest JSON");
        let second = audit_dataset_manifest(SCHEMA, &bytes).expect("permuted Shenzhen manifest");
        assert_eq!(first.canonical_manifest_sha256, second.canonical_manifest_sha256);
        assert_eq!(first.report_sha256, second.report_sha256);
    }
}
