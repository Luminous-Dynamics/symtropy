use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MANIFEST_VERSION: &str = "ev-bms-generated-reference/v0.1";
const SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/transport/ev-bms-generated-reference/0.1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedReferenceManifest {
    manifest_version: String,
    dataset_id: String,
    title: String,
    paper_url: String,
    source_repository_url: String,
    release_url: String,
    release_tag: String,
    release_published_on: String,
    metadata_retrieved_on: String,
    license_spdx: String,
    permitted_use: String,
    redistribution_allowed: bool,
    commercial_use_allowed: bool,
    evidence_class: String,
    generation_method: String,
    raw_fault_labelled_field_telemetry_released: bool,
    generated_pack_instance_count: u64,
    chemistry_counts: Vec<ChemistryCount>,
    charge_segments_per_pack: u32,
    discharge_segments_per_pack: u32,
    sample_interval_seconds: u32,
    segment_duration_minutes_min: u32,
    segment_duration_minutes_max: u32,
    pack_topologies_cell_count: Vec<u32>,
    ambient_temperature_c_min: i32,
    ambient_temperature_c_max: i32,
    fault_classes: Vec<String>,
    severity_levels: Vec<String>,
    fault_counts_by_chemistry: Vec<ChemistryFaultCounts>,
    assets: Vec<ReleaseAsset>,
    admission_state: String,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChemistryCount {
    chemistry: String,
    generated_pack_instance_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChemistryFaultCounts {
    chemistry: String,
    normal: u64,
    high_resistance: u64,
    low_capacity: u64,
    self_discharge: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseAsset {
    name: String,
    size_bytes: u64,
    publisher_sha256: String,
    local_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GeneratedReferenceAdmissionReport {
    pub report_version: String,
    pub dataset_id: String,
    pub release_tag: String,
    pub release_published_on: String,
    pub license_spdx: String,
    pub permitted_use: String,
    pub commercial_use_allowed: bool,
    pub redistribution_allowed: bool,
    pub evidence_class: String,
    pub generation_method: String,
    pub raw_fault_labelled_field_telemetry_released: bool,
    pub generated_pack_instance_count: u64,
    pub generated_segment_count: u64,
    pub chemistry_count: usize,
    pub sample_interval_seconds: u32,
    pub pack_topologies_cell_count: Vec<u32>,
    pub fault_class_count: usize,
    pub severity_level_count: usize,
    pub asset_count: usize,
    pub total_release_bytes: u64,
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
    for field in ["assets", "chemistry_counts", "fault_counts_by_chemistry"] {
        if let Some(items) = value.get_mut(field).and_then(Value::as_array_mut) {
            items.sort_by(|left, right| {
                let key = if field == "assets" {
                    "name"
                } else {
                    "chemistry"
                };
                left.get(key)
                    .and_then(Value::as_str)
                    .cmp(&right.get(key).and_then(Value::as_str))
            });
        }
    }
    if let Some(topologies) = value
        .get_mut("pack_topologies_cell_count")
        .and_then(Value::as_array_mut)
    {
        topologies.sort_by_key(Value::as_u64);
    }
    if let Some(classes) = value.get_mut("fault_classes").and_then(Value::as_array_mut) {
        classes.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
    }
    if let Some(levels) = value.get_mut("severity_levels").and_then(Value::as_array_mut) {
        levels.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
    }
}

fn validate_manifest(manifest: &GeneratedReferenceManifest) -> Result<(), String> {
    if manifest.manifest_version != MANIFEST_VERSION {
        return Err("unsupported generated-reference manifest version".to_owned());
    }
    for (name, value) in [
        ("dataset_id", manifest.dataset_id.as_str()),
        ("title", manifest.title.as_str()),
        ("paper_url", manifest.paper_url.as_str()),
        ("source_repository_url", manifest.source_repository_url.as_str()),
        ("release_url", manifest.release_url.as_str()),
        ("release_tag", manifest.release_tag.as_str()),
        ("release_published_on", manifest.release_published_on.as_str()),
        ("metadata_retrieved_on", manifest.metadata_retrieved_on.as_str()),
        ("claim_ceiling", manifest.claim_ceiling.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{name} must not be empty"));
        }
    }
    if manifest.dataset_id != "ch-batterygen-iclr-2026-v1"
        || manifest.paper_url
            != "https://proceedings.iclr.cc/paper_files/paper/2026/file/0d781fa5f639bf2caf728a68e9678362-Paper-Conference.pdf"
        || manifest.source_repository_url != "https://github.com/CH-BatteryGen/dataset-warehouse"
        || manifest.release_url
            != "https://github.com/CH-BatteryGen/dataset-warehouse/releases/tag/v1.0.0"
        || manifest.release_tag != "v1.0.0"
        || manifest.release_published_on != "2025-09-15"
    {
        return Err(
            "CH-BatteryGen source identity does not match the pinned ICLR/GitHub release".to_owned(),
        );
    }
    if manifest.license_spdx != "CC-BY-NC-SA-4.0"
        || manifest.permitted_use != "academic_research_non_commercial_only"
        || manifest.commercial_use_allowed
        || manifest.redistribution_allowed
    {
        return Err(
            "CH-BatteryGen v1.0.0 terms must remain CC-BY-NC-SA-4.0, academic/non-commercial and non-redistributable".to_owned(),
        );
    }
    if manifest.evidence_class != "mechanism_constrained_generated"
        || manifest.generation_method != "mechanism_constrained_generated_from_real_fleet_distribution"
        || manifest.raw_fault_labelled_field_telemetry_released
    {
        return Err(
            "generated CH-BatteryGen output must not be promoted to raw field telemetry".to_owned(),
        );
    }
    if manifest.generated_pack_instance_count != 1000
        || manifest.chemistry_counts.len() != 2
        || manifest.charge_segments_per_pack != 10
        || manifest.discharge_segments_per_pack != 10
        || manifest.sample_interval_seconds != 10
        || manifest.segment_duration_minutes_min != 30
        || manifest.segment_duration_minutes_max != 60
        || manifest.ambient_temperature_c_min != -10
        || manifest.ambient_temperature_c_max != 45
    {
        return Err("CH-BatteryGen size, sampling or operating-range metadata mismatch".to_owned());
    }
    let mut chemistries = BTreeSet::new();
    let mut chemistry_total = 0_u64;
    for chemistry in &manifest.chemistry_counts {
        if !["LFP", "NCM"].contains(&chemistry.chemistry.as_str())
            || !chemistries.insert(chemistry.chemistry.as_str())
            || chemistry.generated_pack_instance_count != 500
        {
            return Err(
                "CH-BatteryGen must contain 500 generated LFP and 500 generated NCM packs"
                    .to_owned(),
            );
        }
        chemistry_total += chemistry.generated_pack_instance_count;
    }
    if chemistry_total != manifest.generated_pack_instance_count
        || chemistries.len() != 2
    {
        return Err("chemistry counts do not reconcile to generated pack total".to_owned());
    }
    let expected_classes = [
        "normal",
        "high_resistance",
        "low_capacity",
        "self_discharge",
    ];
    let classes = manifest
        .fault_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if classes.len() != 4
        || expected_classes.iter().any(|class| !classes.contains(class))
        || manifest.severity_levels.iter().map(String::as_str).collect::<BTreeSet<_>>()
            != ["mild", "moderate", "severe"].into_iter().collect()
        || manifest.severity_levels.len() != 3
    {
        return Err("fault taxonomy or severity levels do not match the paper".to_owned());
    }
    if manifest.pack_topologies_cell_count.iter().copied().collect::<BTreeSet<_>>()
        != [28, 92, 96, 124].into_iter().collect()
        || manifest.pack_topologies_cell_count.len() != 4
    {
        return Err("CH-BatteryGen pack topologies must be 28, 92, 96 and 124 cells".to_owned());
    }
    if manifest.fault_counts_by_chemistry.len() != 2 {
        return Err("fault counts for both chemistries are required".to_owned());
    }
    for counts in &manifest.fault_counts_by_chemistry {
        if !["LFP", "NCM"].contains(&counts.chemistry.as_str())
            || counts.normal != 400
            || counts.high_resistance != 30
            || counts.low_capacity != 30
            || counts.self_discharge != 40
            || counts.normal + counts.high_resistance + counts.low_capacity + counts.self_discharge
                != 500
        {
            return Err("per-chemistry class counts must be 400/30/30/40".to_owned());
        }
    }
    if manifest.assets.len() != 2 {
        return Err("CH-BatteryGen release must contain exactly two declared assets".to_owned());
    }
    let expected_assets = [
        (
            "V1.0.7z",
            386_457_534_u64,
            "2bce2a26720519833f8a0b73e7af95bb6ab8ac997cc62afc3d320c67fd02be21",
        ),
        (
            "faults.details.-.LFP.batteries.xlsx",
            13_437_u64,
            "583411788a3c5b90d43d27de87493d36dfff93da01493e69f503e6c0a84e3e6b",
        ),
    ];
    let mut assets = BTreeSet::new();
    for asset in &manifest.assets {
        if !assets.insert(asset.name.as_str())
            || !is_sha256(&asset.publisher_sha256)
        {
            return Err("asset names must be unique and publisher hashes valid".to_owned());
        }
        if asset
            .local_sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256(digest))
        {
            return Err(format!("{} has a malformed local SHA-256", asset.name));
        }
    }
    for (name, size, digest) in expected_assets {
        if !manifest.assets.iter().any(|asset| {
            asset.name == name
                && asset.size_bytes == size
                && asset.publisher_sha256 == digest
        }) {
            return Err(format!("CH-BatteryGen release asset metadata mismatch for {name}"));
        }
    }
    let total_bytes = manifest.assets.iter().map(|asset| asset.size_bytes).sum::<u64>();
    if manifest.admission_state == "metadata_only" {
        if manifest.assets.iter().any(|asset| asset.local_sha256.is_some()) {
            return Err(
                "metadata_only manifest cannot claim locally verified release files".to_owned(),
            );
        }
    } else if manifest.admission_state == "snapshot_admitted" {
        if manifest.assets.iter().any(|asset| asset.local_sha256.is_none()) {
            return Err(
                "snapshot_admitted requires local SHA-256 for every release asset".to_owned(),
            );
        }
    } else {
        return Err("admission_state must be metadata_only or snapshot_admitted".to_owned());
    }
    if total_bytes != 386_470_971 {
        return Err("CH-BatteryGen release asset size total mismatch".to_owned());
    }
    Ok(())
}

pub fn audit_generated_reference(
    schema_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<GeneratedReferenceAdmissionReport, String> {
    let schema: Value =
        serde_json::from_slice(schema_bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("generated-reference schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("generated-reference schema identity does not match v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err(
            "generated-reference schema fails Draft 2020-12 meta-schema validation".to_owned(),
        );
    }
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("generated-reference schema compilation: {error}"))?;

    let mut value: Value =
        serde_json::from_slice(manifest_bytes).map_err(|error| format!("manifest JSON: {error}"))?;
    canonicalize(&mut value);
    if let Err(error) = validator.validate(&value) {
        return Err(format!("generated-reference schema validation: {error}"));
    }
    let canonical_bytes =
        serde_json::to_vec(&value).map_err(|error| format!("canonical manifest: {error}"))?;
    let manifest: GeneratedReferenceManifest = serde_json::from_slice(&canonical_bytes)
        .map_err(|error| format!("typed manifest: {error}"))?;
    validate_manifest(&manifest)?;

    let report = GeneratedReferenceAdmissionReport {
        report_version: "ev-bms-generated-reference-report/v0.1".to_owned(),
        dataset_id: manifest.dataset_id,
        release_tag: manifest.release_tag,
        release_published_on: manifest.release_published_on,
        license_spdx: manifest.license_spdx,
        permitted_use: manifest.permitted_use,
        commercial_use_allowed: manifest.commercial_use_allowed,
        redistribution_allowed: manifest.redistribution_allowed,
        evidence_class: manifest.evidence_class,
        generation_method: manifest.generation_method,
        raw_fault_labelled_field_telemetry_released: manifest
            .raw_fault_labelled_field_telemetry_released,
        generated_pack_instance_count: manifest.generated_pack_instance_count,
        generated_segment_count: manifest.generated_pack_instance_count
            * u64::from(manifest.charge_segments_per_pack + manifest.discharge_segments_per_pack),
        chemistry_count: manifest.chemistry_counts.len(),
        sample_interval_seconds: manifest.sample_interval_seconds,
        pack_topologies_cell_count: manifest.pack_topologies_cell_count,
        fault_class_count: manifest.fault_classes.len(),
        severity_level_count: manifest.severity_levels.len(),
        asset_count: manifest.assets.len(),
        total_release_bytes: manifest.assets.iter().map(|asset| asset.size_bytes).sum(),
        admission_state: manifest.admission_state,
        schema_sha256: sha256(schema_bytes),
        canonical_manifest_sha256: sha256(&canonical_bytes),
        claim_ceiling: manifest.claim_ceiling,
        report_sha256: String::new(),
    };
    let report_bytes =
        serde_json::to_vec(&report).map_err(|error| format!("report serialization: {error}"))?;
    let mut final_report = report;
    final_report.report_sha256 = sha256(&report_bytes);
    Ok(final_report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] =
        include_bytes!("../../../schemas/transport/ev_bms_generated_reference_v0_1.schema.json");
    const FIXTURE: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/ch_batteryg_en_generated_manifest_v0_1.json");

    #[test]
    fn admits_metadata_only_mechanism_constrained_generated_source() {
        let report = audit_generated_reference(SCHEMA, FIXTURE).expect("CH-BatteryGen metadata");
        assert_eq!(report.generated_pack_instance_count, 1000);
        assert_eq!(report.generated_segment_count, 20_000);
        assert_eq!(report.chemistry_count, 2);
        assert_eq!(report.asset_count, 2);
        assert_eq!(report.total_release_bytes, 386_470_971);
        assert_eq!(report.evidence_class, "mechanism_constrained_generated");
        assert_eq!(report.license_spdx, "CC-BY-NC-SA-4.0");
        assert!(!report.commercial_use_allowed);
        assert_eq!(report.admission_state, "metadata_only");
    }

    #[test]
    fn rejects_claim_that_generated_output_is_raw_field_telemetry() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["raw_fault_labelled_field_telemetry_released"] = Value::Bool(true);
        let bytes = serde_json::to_vec(&value).expect("mutated manifest");
        assert!(audit_generated_reference(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_commercial_permission_or_license_upgrade() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["commercial_use_allowed"] = Value::Bool(true);
        let bytes = serde_json::to_vec(&value).expect("mutated manifest");
        assert!(audit_generated_reference(SCHEMA, &bytes).is_err());

        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["license_spdx"] = Value::String("CC-BY-4.0".to_owned());
        let bytes = serde_json::to_vec(&value).expect("mutated manifest");
        assert!(audit_generated_reference(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_release_asset_hash_size_or_inventory_mutation() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["assets"][0]["publisher_sha256"] = Value::String("00".repeat(32));
        let bytes = serde_json::to_vec(&value).expect("mutated manifest");
        assert!(audit_generated_reference(SCHEMA, &bytes).is_err());

        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["assets"][0]["size_bytes"] = Value::from(1);
        let bytes = serde_json::to_vec(&value).expect("mutated manifest");
        assert!(audit_generated_reference(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn rejects_snapshot_admission_without_local_hashes() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["admission_state"] = Value::String("snapshot_admitted".to_owned());
        let bytes = serde_json::to_vec(&value).expect("mutated manifest");
        assert!(audit_generated_reference(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn canonical_manifest_digest_is_invariant_to_asset_order() {
        let first = audit_generated_reference(SCHEMA, FIXTURE).expect("first manifest");
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["assets"].as_array_mut().unwrap().reverse();
        let bytes = serde_json::to_vec(&value).expect("permuted manifest");
        let second = audit_generated_reference(SCHEMA, &bytes).expect("permuted manifest");
        assert_eq!(first.canonical_manifest_sha256, second.canonical_manifest_sha256);
        assert_eq!(first.report_sha256, second.report_sha256);
    }
}
