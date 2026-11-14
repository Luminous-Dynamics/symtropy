// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const REPORT_SCHEMA: &str = "luminous.habitat_manifest_verifier.report.v0.1";
pub const CLAIM_CEILING: &str = "structural_conformance_only_not_semantic_or_physical_validation";
const WHOLE_PROPERTY_SHA256: &str =
    "68dd4c244b29e840319fb898706fa1a4aca411a7a99655ebe3bb8c646801eb94";
const TERRAIN_NATIVE_SHA256: &str =
    "2e5eb734bd14264812c27c6d23cd10ad9b49e8327f3cde8df300124cb77f578e";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestKind {
    WholeProperty,
    TerrainNative,
}

impl ManifestKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "whole-property" => Ok(Self::WholeProperty),
            "terrain-native" => Ok(Self::TerrainNative),
            other => Err(format!(
                "unknown manifest kind {other:?}; expected whole-property or terrain-native"
            )),
        }
    }

    fn expected_version(self) -> &'static str {
        match self {
            Self::WholeProperty => "whole-property-habitat-scenarios-v0.1",
            Self::TerrainNative => "terrain-native-habitat-scenarios-v0.1",
        }
    }

    fn id_prefix(self) -> &'static str {
        match self {
            Self::WholeProperty => "WP-",
            Self::TerrainNative => "TNH-",
        }
    }

    fn expected_count(self) -> usize {
        match self {
            Self::WholeProperty => 19,
            Self::TerrainNative => 14,
        }
    }

    fn expected_negative_controls(self) -> usize {
        match self {
            Self::WholeProperty => 17,
            Self::TerrainNative => 12,
        }
    }

    fn pinned_sha256(self) -> &'static str {
        match self {
            Self::WholeProperty => WHOLE_PROPERTY_SHA256,
            Self::TerrainNative => TERRAIN_NATIVE_SHA256,
        }
    }

    fn allowed_dispositions(self) -> &'static [&'static str] {
        match self {
            Self::WholeProperty => &[
                "needs_evidence",
                "ineligible",
                "comparison_only",
                "ineligible_or_redesign",
                "service_floor_not_met",
                "hard_gate_failure",
                "maintenance_requirement_unmet",
                "revalidation_required",
                "stale_evidence",
                "needs_authorized_decision",
                "needs_qualified_review",
                "out_of_scope",
                "indeterminate_or_bounded",
                "oracle_match",
                "dependent_results_stale",
            ],
            Self::TerrainNative => &[
                "ineligible_under_hard_constraint",
                "needs_evidence",
                "needs_qualified_review",
                "resilience_requirement_not_met",
                "stale_needs_reassessment",
                "decision_blocker_and_measurement_dependency_reported",
                "deterministic_pareto_comparison_within_scope",
            ],
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest<T> {
    manifest_version: String,
    status: String,
    synthetic_only: bool,
    intended_use: String,
    scenario_count: usize,
    scenarios: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WholePropertyScenario {
    id: String,
    profile: String,
    domain: String,
    setup: String,
    expected_gate_state: String,
    negative_control: bool,
    evidence_class: String,
    required_invariants: Vec<String>,
    claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerrainNativeScenario {
    id: String,
    profile: String,
    setup: String,
    expected: String,
    negative_control: bool,
    invariants: Vec<String>,
    evidence_class: String,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ScenarioSummary {
    pub id: String,
    pub expected_disposition: String,
    pub negative_control: bool,
    pub invariant_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub schema_version: String,
    pub manifest_version: String,
    pub input_sha256: String,
    pub verification_status: String,
    pub claim_ceiling: String,
    pub scenario_count: usize,
    pub negative_control_count: usize,
    pub scenario_ids: Vec<String>,
    pub scenarios: Vec<ScenarioSummary>,
}

#[derive(Debug)]
struct CaseSummary {
    id: String,
    expected_disposition: String,
    negative_control: bool,
    evidence_class: String,
    claim_ceiling: String,
    text_fields: Vec<(&'static str, String)>,
    invariants: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Verify the exact frozen v0.1 corpus bytes and then validate their strict structure.
/// This tool does not independently verify whether a scenario's authored disposition is
/// scientifically correct; the report therefore cannot be interpreted as a semantic pass.
pub fn verify_manifest_bytes(
    kind: ManifestKind,
    bytes: &[u8],
) -> Result<VerificationReport, Vec<String>> {
    let actual = sha256_hex(bytes);
    let expected = kind.pinned_sha256();
    if actual != expected {
        return Err(vec![format!(
            "frozen manifest digest mismatch: expected {expected}, got {actual}"
        )]);
    }
    validate_manifest_structure(kind, bytes)
}

fn validate_manifest_structure(
    kind: ManifestKind,
    bytes: &[u8],
) -> Result<VerificationReport, Vec<String>> {
    let (manifest_version, status, synthetic_only, intended_use, declared_count, cases) =
        match kind {
            ManifestKind::WholeProperty => {
                let manifest: Manifest<WholePropertyScenario> =
                    serde_json::from_slice(bytes).map_err(|error| {
                        vec![format!("whole-property manifest rejected: {error}")]
                    })?;
                let cases: Vec<CaseSummary> = manifest
                    .scenarios
                    .into_iter()
                    .map(|case| CaseSummary {
                        id: case.id,
                        expected_disposition: case.expected_gate_state,
                        negative_control: case.negative_control,
                        evidence_class: case.evidence_class,
                        claim_ceiling: case.claim_ceiling,
                        text_fields: vec![
                            ("profile", case.profile),
                            ("domain", case.domain),
                            ("setup", case.setup),
                        ],
                        invariants: case.required_invariants,
                    })
                    .collect();
                (
                    manifest.manifest_version,
                    manifest.status,
                    manifest.synthetic_only,
                    manifest.intended_use,
                    manifest.scenario_count,
                    cases,
                )
            }
            ManifestKind::TerrainNative => {
                let manifest: Manifest<TerrainNativeScenario> =
                    serde_json::from_slice(bytes).map_err(|error| {
                        vec![format!("terrain-native manifest rejected: {error}")]
                    })?;
                let cases: Vec<CaseSummary> = manifest
                    .scenarios
                    .into_iter()
                    .map(|case| CaseSummary {
                        id: case.id,
                        expected_disposition: case.expected,
                        negative_control: case.negative_control,
                        evidence_class: case.evidence_class,
                        claim_ceiling: case.claim_ceiling,
                        text_fields: vec![("profile", case.profile), ("setup", case.setup)],
                        invariants: case.invariants,
                    })
                    .collect();
                (
                    manifest.manifest_version,
                    manifest.status,
                    manifest.synthetic_only,
                    manifest.intended_use,
                    manifest.scenario_count,
                    cases,
                )
            }
        };

    let mut errors = Vec::new();
    if manifest_version != kind.expected_version() {
        errors.push(format!(
            "manifest_version must be {:?}, got {:?}",
            kind.expected_version(),
            manifest_version
        ));
    }
    if status != "proposed_synthetic_conformance_corpus" {
        errors.push(format!("unsupported manifest status: {status:?}"));
    }
    if !synthetic_only {
        errors.push("synthetic_only must be true".to_owned());
    }
    if intended_use.trim().is_empty() {
        errors.push("intended_use must not be blank".to_owned());
    }
    if declared_count != cases.len() {
        errors.push(format!(
            "scenario_count declares {declared_count}, but {} scenario records exist",
            cases.len()
        ));
    }
    if cases.len() != kind.expected_count() {
        errors.push(format!(
            "frozen v0.1 corpus requires {} scenarios, got {}",
            kind.expected_count(),
            cases.len()
        ));
    }

    let mut seen = BTreeSet::new();
    let mut summaries = Vec::with_capacity(cases.len());
    let mut negative_control_count = 0usize;
    for (index, case) in cases.into_iter().enumerate() {
        let expected_id = format!("{}{number:03}", kind.id_prefix(), number = index + 1);
        if !seen.insert(case.id.clone()) {
            errors.push(format!("duplicate scenario ID: {:?}", case.id));
        }
        if case.id != expected_id {
            errors.push(format!(
                "scenario at index {index} must have ID {expected_id:?}, got {:?}",
                case.id
            ));
        }
        if case.expected_disposition.trim().is_empty()
            || !kind.allowed_dispositions().contains(&case.expected_disposition.as_str())
        {
            errors.push(format!(
                "{} has unsupported expected disposition {:?}",
                case.id, case.expected_disposition
            ));
        }
        for (field, value) in &case.text_fields {
            if value.trim().is_empty() {
                errors.push(format!("{} field {field} must not be blank", case.id));
            }
        }
        if case.invariants.is_empty() {
            errors.push(format!("{} must declare at least one invariant", case.id));
        }
        for (inv_index, invariant) in case.invariants.iter().enumerate() {
            if invariant.trim().is_empty() {
                errors.push(format!(
                    "{} invariant at index {inv_index} must not be blank",
                    case.id
                ));
            }
        }
        if case.evidence_class != "synthetic" {
            errors.push(format!(
                "{} evidence_class must remain synthetic, got {:?}",
                case.id, case.evidence_class
            ));
        }
        if case.claim_ceiling != "synthetic_conformance_only" {
            errors.push(format!(
                "{} has unsupported claim ceiling {:?}",
                case.id, case.claim_ceiling
            ));
        }
        if case.negative_control {
            negative_control_count += 1;
        }
        summaries.push(ScenarioSummary {
            id: case.id,
            expected_disposition: case.expected_disposition,
            negative_control: case.negative_control,
            invariant_count: case.invariants.len(),
        });
    }
    if negative_control_count != kind.expected_negative_controls() {
        errors.push(format!(
            "frozen v0.1 corpus requires {} negative controls, got {negative_control_count}",
            kind.expected_negative_controls()
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let input_sha256 = sha256_hex(bytes);
    let scenario_ids = summaries.iter().map(|case| case.id.clone()).collect();
    Ok(VerificationReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        manifest_version,
        input_sha256,
        verification_status: "STRUCTURAL_PASS_NOT_SEMANTIC_PASS".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        scenario_count: summaries.len(),
        negative_control_count,
        scenario_ids,
        scenarios: summaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn synthetic_manifest(kind: ManifestKind) -> Value {
        let (version, prefix, count, negatives) = match kind {
            ManifestKind::WholeProperty => (
                "whole-property-habitat-scenarios-v0.1",
                "WP-",
                19,
                17,
            ),
            ManifestKind::TerrainNative => (
                "terrain-native-habitat-scenarios-v0.1",
                "TNH-",
                14,
                12,
            ),
        };
        let scenarios: Vec<Value> = (1..=count)
            .map(|index| {
                let id = format!("{prefix}{index:03}");
                let negative_control = index <= negatives;
                match kind {
                    ManifestKind::WholeProperty => json!({
                        "id": id,
                        "profile": "HOME-BASE",
                        "domain": "site_evidence",
                        "setup": "Synthetic test setup.",
                        "expected_gate_state": "needs_evidence",
                        "negative_control": negative_control,
                        "evidence_class": "synthetic",
                        "required_invariants": ["Unknown remains unknown."],
                        "claim_ceiling": "synthetic_conformance_only"
                    }),
                    ManifestKind::TerrainNative => json!({
                        "id": id,
                        "profile": "RAVINE_CANYON",
                        "setup": "Synthetic test setup.",
                        "expected": "needs_evidence",
                        "negative_control": negative_control,
                        "invariants": ["Unknown remains unknown."],
                        "evidence_class": "synthetic",
                        "claim_ceiling": "synthetic_conformance_only"
                    }),
                }
            })
            .collect();
        json!({
            "manifest_version": version,
            "status": "proposed_synthetic_conformance_corpus",
            "synthetic_only": true,
            "intended_use": "Synthetic structural verifier unit test.",
            "scenario_count": count,
            "scenarios": scenarios
        })
    }

    fn validate_test_value(kind: ManifestKind, value: &Value) -> Result<VerificationReport, Vec<String>> {
        validate_manifest_structure(kind, serde_json::to_string(value).unwrap().as_bytes())
    }

    #[test]
    fn verifies_pinned_whole_property_fixture_bytes() {
        let bytes = include_bytes!(
            "../../../docs/research/fixtures/WHOLE_PROPERTY_HABITAT_SCENARIOS_V0_1.json"
        );
        let report = verify_manifest_bytes(ManifestKind::WholeProperty, bytes).unwrap();
        assert_eq!(report.input_sha256, WHOLE_PROPERTY_SHA256);
        assert_eq!(report.scenario_count, 19);
        assert_eq!(report.negative_control_count, 17);
        assert_eq!(report.verification_status, "STRUCTURAL_PASS_NOT_SEMANTIC_PASS");
    }

    #[test]
    fn verifies_pinned_terrain_native_fixture_bytes() {
        let bytes = include_bytes!(
            "../../../docs/research/fixtures/TERRAIN_NATIVE_HABITAT_SCENARIOS_V0_1.json"
        );
        let report = verify_manifest_bytes(ManifestKind::TerrainNative, bytes).unwrap();
        assert_eq!(report.input_sha256, TERRAIN_NATIVE_SHA256);
        assert_eq!(report.scenario_count, 14);
        assert_eq!(report.negative_control_count, 12);
        assert_eq!(report.verification_status, "STRUCTURAL_PASS_NOT_SEMANTIC_PASS");
    }

    #[test]
    fn sha256_known_answer_matches_standard_abc_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn accepts_complete_whole_property_structure() {
        let report = validate_test_value(
            ManifestKind::WholeProperty,
            &synthetic_manifest(ManifestKind::WholeProperty),
        )
        .unwrap();
        assert_eq!(report.scenario_count, 19);
        assert_eq!(report.negative_control_count, 17);
        assert_eq!(report.verification_status, "STRUCTURAL_PASS_NOT_SEMANTIC_PASS");
    }

    #[test]
    fn accepts_complete_terrain_native_structure() {
        let report = validate_test_value(
            ManifestKind::TerrainNative,
            &synthetic_manifest(ManifestKind::TerrainNative),
        )
        .unwrap();
        assert_eq!(report.scenario_count, 14);
        assert_eq!(report.negative_control_count, 12);
    }

    #[test]
    fn rejects_duplicate_or_out_of_order_ids() {
        let mut value = synthetic_manifest(ManifestKind::TerrainNative);
        value["scenarios"][1]["id"] = json!("TNH-001");
        let errors = validate_test_value(ManifestKind::TerrainNative, &value).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("duplicate scenario ID")));
        assert!(errors.iter().any(|error| error.contains("must have ID")));
    }

    #[test]
    fn rejects_wrong_census_or_negative_control_count() {
        let mut value = synthetic_manifest(ManifestKind::WholeProperty);
        value["scenario_count"] = json!(17);
        value["scenarios"][17]["negative_control"] = json!(true);
        let errors = validate_test_value(ManifestKind::WholeProperty, &value).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("scenario_count declares")));
        assert!(errors.iter().any(|error| error.contains("negative controls")));
    }

    #[test]
    fn rejects_unknown_fields_at_top_level_and_case_level() {
        let mut top_level = synthetic_manifest(ManifestKind::WholeProperty);
        top_level["surprise"] = json!(true);
        let errors = validate_test_value(ManifestKind::WholeProperty, &top_level).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("unknown field")));

        let mut case_level = synthetic_manifest(ManifestKind::TerrainNative);
        case_level["scenarios"][0]["surprise"] = json!("unreviewed field");
        let errors = validate_test_value(ManifestKind::TerrainNative, &case_level).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("unknown field")));
    }

    #[test]
    fn rejects_non_synthetic_claims_and_evidence() {
        let mut value = synthetic_manifest(ManifestKind::WholeProperty);
        value["synthetic_only"] = json!(false);
        value["scenarios"][2]["claim_ceiling"] = json!("physically_safe");
        value["scenarios"][3]["evidence_class"] = json!("measured");
        let errors = validate_test_value(ManifestKind::WholeProperty, &value).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("synthetic_only must be true")));
        assert!(errors.iter().any(|error| error.contains("claim ceiling")));
        assert!(errors.iter().any(|error| error.contains("evidence_class must remain synthetic")));
    }

    #[test]
    fn rejects_blank_requirements_or_invariants() {
        let mut value = synthetic_manifest(ManifestKind::TerrainNative);
        value["scenarios"][0]["profile"] = json!("  ");
        value["scenarios"][1]["invariants"] = json!([]);
        value["scenarios"][2]["invariants"][0] = json!("");
        let errors = validate_test_value(ManifestKind::TerrainNative, &value).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("field profile must not be blank")));
        assert!(errors.iter().any(|error| error.contains("must declare at least one invariant")));
        assert!(errors.iter().any(|error| error.contains("invariant at index")));
    }

    #[test]
    fn rejects_unrecognized_expected_disposition() {
        let mut value = synthetic_manifest(ManifestKind::TerrainNative);
        value["scenarios"][0]["expected"] = json!("safe");
        let errors = validate_test_value(ManifestKind::TerrainNative, &value).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("unsupported expected disposition")));
    }

    #[test]
    fn rejects_unpinned_manifest_bytes_before_parsing() {
        let valid_shape = serde_json::to_vec(&synthetic_manifest(ManifestKind::WholeProperty)).unwrap();
        let error = verify_manifest_bytes(ManifestKind::WholeProperty, &valid_shape).unwrap_err();
        assert!(error[0].contains("frozen manifest digest mismatch"));
    }

    #[test]
    fn rejects_malformed_json() {
        let errors = validate_manifest_structure(ManifestKind::TerrainNative, b"{not-json").unwrap_err();
        assert!(errors[0].contains("manifest rejected"));
    }

    #[test]
    fn rejects_wrong_kind_for_other_frozen_corpus() {
        let value = synthetic_manifest(ManifestKind::TerrainNative);
        let bytes = serde_json::to_vec(&value).unwrap();
        let errors = validate_manifest_structure(ManifestKind::WholeProperty, &bytes).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("whole-property manifest rejected")));
    }
}
