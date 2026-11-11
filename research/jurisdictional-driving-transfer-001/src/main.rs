//! CLI for jurisdiction profile and left/right convention fixture auditing.
//!
//! Usage:
//!   jurisdictional-driving-transfer audit-profile <profile.json> <report.json>
//!   jurisdictional-driving-transfer audit-metamorphic-pair <pair.json> <report.json>
//!
//! Exit codes: 0 structurally valid, 1 rejected, 2 invocation/input error.
//! A valid draft profile is not policy eligible; the report says so explicitly.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use jurisdictional_driving_transfer::{
    audit_metamorphic_fixture, audit_profile, ConventionMetamorphicFixture, JurisdictionProfile,
};
use serde::Serialize;
use serde_json::json;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

fn read_typed<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("input must be a regular file no larger than 16 MiB".into());
    }
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage:\n  jurisdictional-driving-transfer audit-profile <profile.json> <report.json>\n  jurisdictional-driving-transfer audit-metamorphic-pair <pair.json> <report.json>"
                .into(),
        );
    }
    let mode = args[0].as_str();
    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    match mode {
        "audit-profile" => {
            let profile: JurisdictionProfile = read_typed(input_path)?;
            let report = audit_profile(&profile);
            write_json(output_path, &report)?;
            println!(
                "jurisdiction profile {}: {}; policy_eligible={}; findings={}; report={}",
                report.profile_id,
                report.disposition,
                report.policy_eligible,
                report.findings.len(),
                output_path.display()
            );
            Ok(if report.disposition != "rejected" { 0 } else { 1 })
        }
        "audit-metamorphic-pair" => {
            let fixture: ConventionMetamorphicFixture = read_typed(input_path)?;
            let report = audit_metamorphic_fixture(&fixture);
            write_json(output_path, &report)?;
            println!(
                "convention fixture {}: {}; geometry_mirrored={}; findings={}; report={}",
                report.fixture_id,
                report.disposition,
                report.geometry_mirrored,
                report.findings.len(),
                output_path.display()
            );
            Ok(if report.disposition == "valid" { 0 } else { 1 })
        }
        _ => Err(format!("unsupported mode {mode:?}").into()),
    }
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("jurisdictional-driving-transfer: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn za_profile_json() -> Value {
        json!({
            "schema_version": 1,
            "profile_id": "ZA-source-skeleton-v0-1",
            "jurisdiction_code": "ZA",
            "profile_status": "draft",
            "traffic_side": "left",
            "steering_wheel_side": "unknown",
            "roundabout_circulation": "unknown",
            "sign_standard": "SADC/SA Road Traffic Signs Manual — exact applicable edition pending review",
            "effective_from": null,
            "effective_to": null,
            "rule_bundle_sha256": null,
            "sources": [
                {
                    "source_id": "za-government-left-traffic-guidance",
                    "authority": "South African Department of Tourism",
                    "title": "Safety and Travel Tips — Exploring Mzansi by Road",
                    "url": "https://www.tourism.gov.za/Pages/Safety--Travel-Tips-Info.aspx",
                    "source_kind": "government_guidance",
                    "edition_or_revision": null,
                    "published_date": null,
                    "effective_from": null,
                    "effective_to": null,
                    "content_sha256": null,
                    "verification_status": "not_reviewed",
                    "supported_rule_ids": ["traffic-side-left"]
                }
            ],
            "notes": ["draft source skeleton; not policy eligible"]
        })
    }

    #[test]
    fn profile_cli_accepts_draft_but_reports_not_policy_eligible() {
        let profile: JurisdictionProfile =
            serde_json::from_value(za_profile_json()).unwrap();
        let report = audit_profile(&profile);
        assert_eq!(report.disposition, "valid_draft");
        assert!(!report.policy_eligible);
    }

    #[test]
    fn checked_in_za_source_profile_is_valid_draft_not_policy_eligible() {
        let profile: JurisdictionProfile =
            serde_json::from_str(include_str!("../fixtures/za-profile-draft-v0-1.json")).unwrap();
        let report = audit_profile(&profile);
        assert_eq!(report.disposition, "valid_draft");
        assert!(!report.policy_eligible);
        assert_eq!(report.traffic_side, jurisdictional_driving_transfer::TrafficSide::Left);
        assert_eq!(report.legal_reviewed_source_count, 0);
    }

    #[test]
    fn checked_in_left_right_metamorphic_pair_is_valid() {
        let fixture: ConventionMetamorphicFixture =
            serde_json::from_str(include_str!("../fixtures/za-us-metamorphic-lht-rht-v0-1.json")).unwrap();
        let report = audit_metamorphic_fixture(&fixture);
        assert_eq!(report.disposition, "valid");
        assert!(report.same_task_intent);
        assert!(report.traffic_side_changed);
        assert!(report.geometry_mirrored);
    }

    #[test]
    fn both_versioned_schemas_are_parseable_and_close_to_the_fixture_types() {
        let profile_schema: Value =
            serde_json::from_str(include_str!("../schemas/jurisdiction-profile-v1.schema.json")).unwrap();
        let pair_schema: Value =
            serde_json::from_str(include_str!("../schemas/convention-metamorphic-fixture-v1.schema.json")).unwrap();
        assert_eq!(profile_schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
        assert_eq!(pair_schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
        assert!(profile_schema["required"].as_array().unwrap().contains(&json!("traffic_side")));
        assert!(profile_schema["required"].as_array().unwrap().contains(&json!("steering_wheel_side")));
        assert!(pair_schema["required"].as_array().unwrap().contains(&json!("base_geometry")));
        assert!(pair_schema["required"].as_array().unwrap().contains(&json!("alternate_geometry")));
    }

    #[test]
    fn profile_cli_rejects_unknown_fields() {
        let mut value = za_profile_json();
        value["untracked_rule_override"] = serde_json::json!("trust-me");
        assert!(serde_json::from_value::<JurisdictionProfile>(value).is_err());
    }

    #[test]
    fn report_json_preserves_policy_eligibility_separately_from_validity() {
        let profile: JurisdictionProfile =
            serde_json::from_value(za_profile_json()).unwrap();
        let report = audit_profile(&profile);
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["disposition"], "valid_draft");
        assert_eq!(value["policy_eligible"], false);
        assert!(value.get("claim_ceiling").is_some());
    }
}
