//! E6 provider-schema lifecycle and adapter-promotion gate.
//!
//! The registry separates a standard's lifecycle from the scope actually
//! validated by this repository. Draft profiles remain shadow-only. Final
//! profiles with partial schema bundles remain selected-profile-only. Full
//! admission requires an exact schema digest, a complete dependency bundle,
//! a declared full validation scope and an exact-head PASS receipt.
//!
//! This is a dated snapshot/policy oracle, not an automatic upstream crawler.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::process;

const REGISTRY: &str = include_str!("../../fixtures/schema-lifecycle-registry-v0.1.json");
const REGISTRY_SCHEMA: &str =
    include_str!("../../schemas/schema-lifecycle-registry-v0.1.schema.json");
const DATACITE_XSD: &[u8] =
    include_bytes!("../../schemas/datacite-4.7/metadata.xsd");
const OGC_RECORD_SCHEMA: &[u8] =
    include_bytes!("../../schemas/ogc-records-1.0/recordGeoJSON.yaml");
const OPENLINEAGE_SCHEMA: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/schemas/openlineage-2.0.2.schema.json"
);

const DATACITE_PROFILE: &str = "datacite:kernel-4:4.7";
const DATACITE_BLOB: &str = "b7cad8149301676bc39f17b6a77d776ab6556f57";
const DATACITE_DIGEST: &str =
    "be878f9afddeb677ec4cd274233de0c328246748305d9f58ee78352714412522";
const OGC_PROFILE: &str = "ogcapi-records:part1-core:1.0.0";
const OGC_BLOB: &str = "e9e4fcb700e2c5e19d26dbf0eb7f9f2be938768f";
const OGC_DIGEST: &str =
    "cc9c94abd30adf17dfba3fc7ff344afcd36376dd43b3519f535233994edf13b3";
const OPENLINEAGE_PROFILE: &str = "openlineage:run-event:2.0.2";
const OPENLINEAGE_BLOB: &str = "58167f71407add7734810cda9943b39b83c89aa2";
const OPENLINEAGE_DIGEST: &str =
    "69f68bee00b9beac88a87059c0102410e7bb05f3f43c46d02a0409831eceb0d2";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum AdmissionState {
    DraftShadowOnly,
    FinalSelectedProfileOnly,
    FinalConformancePending,
    FullyAdmitted,
    HistoricalReplayOnly,
    AbstainUnverified,
}

#[derive(Debug, Serialize)]
struct ProfileAssessment {
    profile_id: String,
    lifecycle: String,
    admission_state: AdmissionState,
    schema_digest_verified: bool,
    bundle_completeness: String,
    validation_scope: String,
    validator_receipt: String,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct EvolutionReport {
    schema_version: &'static str,
    snapshot_date: String,
    automatic_upgrade: bool,
    version_increment_implies_compatibility: bool,
    profile_count: usize,
    final_selected_profile_only: usize,
    draft_shadow_only: usize,
    final_conformance_pending: usize,
    fully_admitted: usize,
    historical_replay_only: usize,
    abstain_unverified: usize,
    local_schema_snapshots_verified: usize,
    compatibility_observation_count: usize,
    unknown_compatibility_observations: usize,
    profile_assessments: Vec<ProfileAssessment>,
    disposition: &'static str,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn required_text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn parse_json(text: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|error| format!("{label} JSON invalid: {error}"))
}

fn validate_registry_schema(registry: &Value, schema: &Value) -> Result<(), String> {
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("E6 registry JSON Schema is invalid: {error}"))?;
    validator
        .validate(registry)
        .map_err(|error| format!("E6 registry schema validation failed: {error}"))
}

fn verify_local_schema_snapshots(registry: &Value) -> Result<usize, String> {
    let profiles = registry
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "E6 registry.profiles must be an array".to_owned())?;
    let assets = [
        (
            DATACITE_PROFILE,
            DATACITE_XSD,
            DATACITE_BLOB,
            DATACITE_DIGEST,
            "schemas/datacite-4.7/metadata.xsd",
        ),
        (
            OGC_PROFILE,
            OGC_RECORD_SCHEMA,
            OGC_BLOB,
            OGC_DIGEST,
            "schemas/ogc-records-1.0/recordGeoJSON.yaml",
        ),
        (
            OPENLINEAGE_PROFILE,
            OPENLINEAGE_SCHEMA,
            OPENLINEAGE_BLOB,
            OPENLINEAGE_DIGEST,
            "../fixtures/protective-evidence-multiprovider-e3-v0.1/schemas/openlineage-2.0.2.schema.json",
        ),
    ];
    for (profile_id, bytes, blob, digest, expected_path) in assets {
        let profile = profiles
            .iter()
            .find(|item| item.get("profile_id").and_then(Value::as_str) == Some(profile_id))
            .ok_or_else(|| format!("E6 pinned schema profile is missing: {profile_id}"))?;
        if required_text(profile, "source_blob_sha1", profile_id)? != blob
            || required_text(profile, "schema_sha256", profile_id)? != digest
            || required_text(profile, "local_snapshot_path", profile_id)? != expected_path
            || profile.get("byte_length").and_then(Value::as_u64) != Some(bytes.len() as u64)
            || sha256(bytes) != digest
        {
            return Err(format!(
                "E6 local schema snapshot identity/size/SHA-256 mismatch: {profile_id}"
            ));
        }
    }
    let datacite_text = std::str::from_utf8(DATACITE_XSD)
        .map_err(|error| format!("DataCite XSD snapshot is not UTF-8: {error}"))?;
    if !datacite_text.contains("2026-03-03 v4.7")
        || !datacite_text.contains("relationTypeInformation")
    {
        return Err("DataCite XSD snapshot does not identify its 4.7 revision".to_owned());
    }
    let ogc_text = std::str::from_utf8(OGC_RECORD_SCHEMA)
        .map_err(|error| format!("OGC schema snapshot is not UTF-8: {error}"))?;
    if !ogc_text.contains("recordGeoJSON") && !ogc_text.contains("geometry") {
        return Err("OGC record schema snapshot does not match the GeoJSON record profile".to_owned());
    }
    let openlineage: Value = serde_json::from_slice(OPENLINEAGE_SCHEMA)
        .map_err(|error| format!("OpenLineage schema snapshot invalid: {error}"))?;
    if required_text(&openlineage, "$id", "OpenLineage schema")?
        != "https://openlineage.io/spec/2-0-2/OpenLineage.json"
    {
        return Err("OpenLineage schema snapshot ID differs from the pinned 2.0.2 profile".to_owned());
    }
    Ok(assets.len())
}

fn evaluate_profile(profile: &Value) -> ProfileAssessment {
    let profile_id = profile
        .get("profile_id")
        .and_then(Value::as_str)
        .unwrap_or("<missing-profile-id>")
        .to_owned();
    let lifecycle = profile
        .get("lifecycle")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .to_owned();
    let bundle = profile
        .get("bundle_completeness")
        .and_then(Value::as_str)
        .unwrap_or("UNVERIFIED")
        .to_owned();
    let scope = profile
        .get("validation_scope")
        .and_then(Value::as_str)
        .unwrap_or("NOT_VALIDATED")
        .to_owned();
    let receipt = profile
        .get("validator_receipt")
        .and_then(Value::as_str)
        .unwrap_or("NONE")
        .to_owned();
    let digest = profile
        .get("schema_sha256")
        .and_then(Value::as_str)
        .filter(|value| is_lower_hex(value, 64));
    let schema_digest_verified = digest.is_some();
    let validator_identity_present = profile
        .get("validator_id")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    let mapping_manifest_present = profile
        .get("mapping_manifest_version")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());

    let (admission_state, reason) = match lifecycle.as_str() {
        "DRAFT" => (
            AdmissionState::DraftShadowOnly,
            "Draft/preview profiles are shadow-only regardless of parse success or version number.",
        ),
        "DEPRECATED" | "RETIRED" => (
            AdmissionState::HistoricalReplayOnly,
            "Deprecated/retired profiles may replay historical evidence but cannot admit new records.",
        ),
        "FINAL" if !schema_digest_verified => (
            AdmissionState::AbstainUnverified,
            "Final lifecycle status without a valid pinned schema SHA-256 is unverified.",
        ),
        "FINAL"
            if bundle == "PARTIAL_EXTERNAL_REFERENCES"
                || bundle == "NOT_BUNDLED"
                || scope == "SELECTED_PROFILE_RULES"
                || scope == "NOT_VALIDATED" =>
        {
            (
                AdmissionState::FinalSelectedProfileOnly,
                "Final standard lifecycle does not imply full adapter conformance; only the declared selected profile scope is available.",
            )
        }
        "FINAL"
            if bundle == "COMPLETE_LOCAL_SCHEMA"
                && (scope == "FULL_RUN_EVENT_JSON_SCHEMA"
                    || scope == "FULL_PROFILE_CONFORMANCE")
                && receipt == "PENDING_EXACT_HEAD_CI"
                && validator_identity_present
                && mapping_manifest_present =>
        {
            (
                AdmissionState::FinalConformancePending,
                "Schema bundle and scope are declared, but exact-head validator receipt is still pending.",
            )
        }
        "FINAL"
            if bundle == "COMPLETE_LOCAL_SCHEMA"
                && (scope == "FULL_RUN_EVENT_JSON_SCHEMA"
                    || scope == "FULL_PROFILE_CONFORMANCE")
                && receipt == "PASS_EXACT_HEAD"
                && validator_identity_present
                && mapping_manifest_present =>
        {
            (
                AdmissionState::FullyAdmitted,
                "Final profile, complete pinned schema scope and exact-head validator receipt are present.",
            )
        }
        "FINAL" => (
            AdmissionState::AbstainUnverified,
            "Profile lacks a complete supported validation path, validator identity, mapping manifest or admissible validator receipt.",
        ),
        _ => (
            AdmissionState::AbstainUnverified,
            "Unknown lifecycle or malformed profile state is not admissible.",
        ),
    };

    ProfileAssessment {
        profile_id,
        lifecycle,
        admission_state,
        schema_digest_verified,
        bundle_completeness: bundle,
        validation_scope: scope,
        validator_receipt: receipt,
        reason,
    }
}

fn build_report(registry: &Value) -> Result<EvolutionReport, String> {
    let snapshot_date = required_text(registry, "snapshot_date", "E6 registry")?.to_owned();
    let policy = registry
        .get("policy")
        .ok_or_else(|| "E6 registry.policy is required".to_owned())?;
    if policy.get("automatic_upgrade").and_then(Value::as_bool) != Some(false)
        || policy
            .get("version_increment_implies_compatibility")
            .and_then(Value::as_bool)
            != Some(false)
        || policy.get("drafts_are_shadow_only").and_then(Value::as_bool) != Some(true)
        || policy.get("unknown_lifecycle_abstains").and_then(Value::as_bool) != Some(true)
    {
        return Err("E6 safety policy must prohibit auto-upgrade and version-based compatibility inference".to_owned());
    }
    if required_text(
        policy,
        "full_admission_requires_validator_receipt",
        "E6 registry.policy",
    )? != "PASS_EXACT_HEAD"
    {
        return Err("E6 full admission must require PASS_EXACT_HEAD".to_owned());
    }

    let profiles = registry
        .get("profiles")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "E6 registry.profiles must be a non-empty array".to_owned())?;
    let mut seen = BTreeSet::new();
    let mut assessments = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let id = required_text(profile, "profile_id", "E6 profile")?;
        if !seen.insert(id.to_owned()) {
            return Err(format!("E6 registry contains duplicate profile_id: {id}"));
        }
        assessments.push(evaluate_profile(profile));
    }

    let mut compatibility_unknown = 0usize;
    let observations = registry
        .get("compatibility_observations")
        .and_then(Value::as_array)
        .ok_or_else(|| "E6 compatibility_observations must be an array".to_owned())?;
    for observation in observations {
        let from = required_text(observation, "from_profile", "E6 compatibility observation")?;
        let to = required_text(observation, "to_profile", "E6 compatibility observation")?;
        if !seen.contains(from) || !seen.contains(to) {
            return Err("E6 compatibility observation references an unknown profile".to_owned());
        }
        let class = required_text(observation, "compatibility", "E6 compatibility observation")?;
        if class == "UNKNOWN" {
            compatibility_unknown += 1;
        }
        if class == "UNKNOWN"
            && required_text(observation, "decision", "E6 compatibility observation")? != "ABSTAIN"
            && required_text(observation, "decision", "E6 compatibility observation")? != "BLOCKED_DRAFT"
            && required_text(observation, "decision", "E6 compatibility observation")? != "REVIEW_REQUIRED"
        {
            return Err("E6 unknown compatibility cannot be accepted or auto-promoted".to_owned());
        }
    }

    let count = |state| {
        assessments
            .iter()
            .filter(|item| item.admission_state == state)
            .count()
    };
    let local_schema_snapshots_verified = verify_local_schema_snapshots(registry)?;

    Ok(EvolutionReport {
        schema_version: "protect-schema-evolution-e6/0.1",
        snapshot_date,
        automatic_upgrade: false,
        version_increment_implies_compatibility: false,
        profile_count: assessments.len(),
        final_selected_profile_only: count(AdmissionState::FinalSelectedProfileOnly),
        draft_shadow_only: count(AdmissionState::DraftShadowOnly),
        final_conformance_pending: count(AdmissionState::FinalConformancePending),
        fully_admitted: count(AdmissionState::FullyAdmitted),
        historical_replay_only: count(AdmissionState::HistoricalReplayOnly),
        abstain_unverified: count(AdmissionState::AbstainUnverified),
        local_schema_snapshots_verified,
        compatibility_observation_count: observations.len(),
        unknown_compatibility_observations: compatibility_unknown,
        profile_assessments: assessments,
        disposition: "ABSTAIN_UNTIL_PROFILE_ADMISSION_EVIDENCE_IS_COMPLETE",
    })
}

fn run() -> Result<EvolutionReport, String> {
    let registry = parse_json(REGISTRY, "E6 lifecycle registry")?;
    let schema = parse_json(REGISTRY_SCHEMA, "E6 lifecycle registry schema")?;
    validate_registry_schema(&registry, &schema)?;
    build_report(&registry)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("E6 report serialization")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registry() -> Value {
        parse_json(REGISTRY, "registry").expect("fixture registry")
    }

    fn schema() -> Value {
        parse_json(REGISTRY_SCHEMA, "schema").expect("registry schema")
    }

    fn profile(value: &Value, id: &str) -> Value {
        value["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|item| item["profile_id"] == id)
            .expect("profile")
            .clone()
    }

    #[test]
    fn snapshot_registry_is_valid_and_conservative() {
        let report = run().expect("E6 lifecycle report");
        assert_eq!(report.profile_count, 6);
        assert_eq!(report.final_selected_profile_only, 2);
        assert_eq!(report.draft_shadow_only, 3);
        assert_eq!(report.final_conformance_pending, 1);
        assert_eq!(report.fully_admitted, 0);
        assert_eq!(report.abstain_unverified, 0);
        assert_eq!(report.local_schema_snapshots_verified, 3);
        assert!(!report.automatic_upgrade);
        assert!(!report.version_increment_implies_compatibility);
    }

    #[test]
    fn datacite_47_is_final_but_only_selected_profile_admitted() {
        let value = registry();
        let result = evaluate_profile(&profile(&value, DATACITE_PROFILE));
        assert_eq!(result.lifecycle, "FINAL");
        assert_eq!(result.admission_state, AdmissionState::FinalSelectedProfileOnly);
        assert!(result.schema_digest_verified);
    }

    #[test]
    fn datacite_48_draft_remains_shadow_only() {
        let value = registry();
        let result = evaluate_profile(&profile(&value, "datacite:kernel-4:4.8-draft"));
        assert_eq!(result.lifecycle, "DRAFT");
        assert_eq!(result.admission_state, AdmissionState::DraftShadowOnly);
        assert!(!result.schema_digest_verified);
    }

    #[test]
    fn ogc_core_10_is_final_but_not_full_profile_conformance() {
        let value = registry();
        let result = evaluate_profile(&profile(&value, OGC_PROFILE));
        assert_eq!(result.admission_state, AdmissionState::FinalSelectedProfileOnly);
        assert_eq!(result.bundle_completeness, "PARTIAL_EXTERNAL_REFERENCES");
    }

    #[test]
    fn ogc_parts_two_and_three_are_shadow_only_drafts() {
        let value = registry();
        for id in [
            "ogcapi-records:part2-facets:draft",
            "ogcapi-records:part3-crud-harvest:draft",
        ] {
            assert_eq!(
                evaluate_profile(&profile(&value, id)).admission_state,
                AdmissionState::DraftShadowOnly
            );
        }
    }

    #[test]
    fn openlineage_202_is_pinned_but_waits_for_exact_head_receipt() {
        let value = registry();
        let result = evaluate_profile(&profile(&value, OPENLINEAGE_PROFILE));
        assert_eq!(result.admission_state, AdmissionState::FinalConformancePending);
        assert!(result.schema_digest_verified);
        assert_eq!(result.bundle_completeness, "COMPLETE_LOCAL_SCHEMA");
    }

    #[test]
    fn final_profile_with_missing_schema_digest_abstains() {
        let value = registry();
        let mut item = profile(&value, OPENLINEAGE_PROFILE);
        item["schema_sha256"] = Value::Null;
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::AbstainUnverified
        );
    }

    #[test]
    fn malformed_schema_digest_abstains() {
        let value = registry();
        let mut item = profile(&value, DATACITE_PROFILE);
        item["schema_sha256"] = Value::String("not-a-sha256".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::AbstainUnverified
        );
    }

    #[test]
    fn draft_cannot_be_promoted_even_with_complete_bundle_and_pass_receipt() {
        let value = registry();
        let mut item = profile(&value, "datacite:kernel-4:4.8-draft");
        item["schema_sha256"] = Value::String("a".repeat(64));
        item["bundle_completeness"] = Value::String("COMPLETE_LOCAL_SCHEMA".into());
        item["validation_scope"] = Value::String("FULL_PROFILE_CONFORMANCE".into());
        item["validator_receipt"] = Value::String("PASS_EXACT_HEAD".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::DraftShadowOnly
        );
    }

    #[test]
    fn final_complete_profile_requires_an_exact_head_receipt() {
        let value = registry();
        let mut item = profile(&value, OPENLINEAGE_PROFILE);
        item["validator_receipt"] = Value::String("PENDING_EXACT_HEAD_CI".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::FinalConformancePending
        );
    }

    #[test]
    fn final_complete_profile_admits_only_with_pass_receipt() {
        let value = registry();
        let mut item = profile(&value, OPENLINEAGE_PROFILE);
        item["validator_receipt"] = Value::String("PASS_EXACT_HEAD".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::FullyAdmitted
        );
    }

    #[test]
    fn final_profile_with_selected_scope_never_becomes_full_admission() {
        let value = registry();
        let mut item = profile(&value, DATACITE_PROFILE);
        item["bundle_completeness"] = Value::String("COMPLETE_LOCAL_SCHEMA".into());
        item["validation_scope"] = Value::String("SELECTED_PROFILE_RULES".into());
        item["validator_receipt"] = Value::String("PASS_EXACT_HEAD".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::FinalSelectedProfileOnly
        );
    }

    #[test]
    fn full_admission_requires_a_validator_implementation_identity() {
        let value = registry();
        let mut item = profile(&value, OPENLINEAGE_PROFILE);
        item["validator_receipt"] = Value::String("PASS_EXACT_HEAD".into());
        item["validator_id"] = Value::Null;
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::AbstainUnverified
        );
    }

    #[test]
    fn full_admission_requires_a_mapping_manifest_version() {
        let value = registry();
        let mut item = profile(&value, OPENLINEAGE_PROFILE);
        item["validator_receipt"] = Value::String("PASS_EXACT_HEAD".into());
        item["mapping_manifest_version"] = Value::Null;
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::AbstainUnverified
        );
    }

    #[test]
    fn deprecated_profiles_are_historical_replay_only() {
        let value = registry();
        let mut item = profile(&value, DATACITE_PROFILE);
        item["lifecycle"] = Value::String("DEPRECATED".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::HistoricalReplayOnly
        );
    }

    #[test]
    fn retired_profiles_are_not_admitted_for_new_records() {
        let value = registry();
        let mut item = profile(&value, OGC_PROFILE);
        item["lifecycle"] = Value::String("RETIRED".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::HistoricalReplayOnly
        );
    }

    #[test]
    fn unknown_lifecycle_abstains() {
        let value = registry();
        let mut item = profile(&value, OPENLINEAGE_PROFILE);
        item["lifecycle"] = Value::String("MAYBE_FINAL".into());
        assert_eq!(
            evaluate_profile(&item).admission_state,
            AdmissionState::AbstainUnverified
        );
    }

    #[test]
    fn duplicate_profile_identity_is_rejected() {
        let mut value = registry();
        let first = value["profiles"][0].clone();
        value["profiles"].as_array_mut().expect("profiles").push(first);
        let error = build_report(&value).expect_err("duplicate profile ID");
        assert!(error.contains("duplicate profile_id"));
    }

    #[test]
    fn compatibility_observation_cannot_reference_unknown_profile() {
        let mut value = registry();
        value["compatibility_observations"][0]["to_profile"] =
            Value::String("missing-profile".into());
        let error = build_report(&value).expect_err("unknown profile target");
        assert!(error.contains("unknown profile"));
    }

    #[test]
    fn unknown_compatibility_requires_review_or_abstention() {
        let mut value = registry();
        value["compatibility_observations"][0]["compatibility"] =
            Value::String("UNKNOWN".into());
        value["compatibility_observations"][0]["decision"] =
            Value::String("NO_MIGRATION".into());
        let error = build_report(&value).expect_err("unknown compatibility cannot pass");
        assert!(error.contains("unknown compatibility cannot be accepted"));
    }

    #[test]
    fn version_increment_does_not_imply_compatibility() {
        let value = registry();
        assert_eq!(
            value["policy"]["version_increment_implies_compatibility"],
            Value::Bool(false)
        );
        assert_eq!(
            value["compatibility_observations"][0]["compatibility"],
            Value::String("UNKNOWN".into())
        );
    }

    #[test]
    fn no_automatic_upgrade_is_permitted() {
        let mut value = registry();
        value["policy"]["automatic_upgrade"] = Value::Bool(true);
        let error = build_report(&value).expect_err("automatic upgrade policy");
        assert!(error.contains("prohibit auto-upgrade"));
    }

    #[test]
    fn validator_receipt_must_be_exact_head_pass() {
        let mut value = registry();
        value["policy"]["full_admission_requires_validator_receipt"] =
            Value::String("PASS_ANY_RUN".into());
        let error = build_report(&value).expect_err("weak receipt policy");
        assert!(error.contains("PASS_EXACT_HEAD"));
    }

    #[test]
    fn data_cite_root_snapshot_matches_pinned_digest_and_revision() {
        let value = registry();
        let verified = verify_local_schema_snapshots(&value).expect("schema snapshots");
        assert_eq!(verified, 3);
        let text = std::str::from_utf8(DATACITE_XSD).expect("XSD");
        assert!(text.contains("2026-03-03 v4.7"));
    }

    #[test]
    fn ogc_record_geojson_root_snapshot_matches_pinned_digest() {
        let value = registry();
        assert!(verify_local_schema_snapshots(&value).is_ok());
        let text = std::str::from_utf8(OGC_RECORD_SCHEMA).expect("OGC schema");
        assert!(text.contains("geometry"));
        assert!(text.contains("conformsTo"));
    }

    #[test]
    fn openlineage_root_snapshot_matches_pinned_schema_id_and_digest() {
        let value = registry();
        assert!(verify_local_schema_snapshots(&value).is_ok());
        let schema: Value = serde_json::from_slice(OPENLINEAGE_SCHEMA).expect("OpenLineage schema");
        assert_eq!(
            schema["$id"],
            "https://openlineage.io/spec/2-0-2/OpenLineage.json"
        );
    }

    #[test]
    fn local_snapshot_tampering_changes_digest_verification() {
        let wrong = sha256(b"tampered DataCite schema");
        assert_ne!(wrong, DATACITE_DIGEST);
        assert_ne!(sha256(DATACITE_XSD), wrong);
    }

    #[test]
    fn draft_and_partial_bundle_profiles_cannot_be_fully_admitted() {
        let value = registry();
        for id in [
            DATACITE_PROFILE,
            OGC_PROFILE,
            "datacite:kernel-4:4.8-draft",
            "ogcapi-records:part2-facets:draft",
            "ogcapi-records:part3-crud-harvest:draft",
        ] {
            assert_ne!(
                evaluate_profile(&profile(&value, id)).admission_state,
                AdmissionState::FullyAdmitted
            );
        }
    }

    #[test]
    fn full_admission_count_remains_zero_while_all_receipts_are_pending() {
        let report = run().expect("registry");
        assert_eq!(report.fully_admitted, 0);
        assert_eq!(report.final_conformance_pending, 1);
        assert_eq!(report.final_selected_profile_only, 2);
    }
}
