use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "luminous.vector_habitat_cases.v0.1";
pub const STATUS: &str = "synthetic_only";
pub const CLAIM_CEILING: &str = "semantic_and_model_interface_conformance_only";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub status: String,
    pub claim_ceiling: String,
    pub units_policy: String,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub name: String,
    pub class: String,
    pub expected_state: String,
    pub assertions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub report_schema: &'static str,
    pub status: &'static str,
    pub source_sha256: String,
    pub case_count: usize,
    pub errors: Vec<String>,
    pub claim_ceiling: &'static str,
}

const REQUIRED: [(&str, &str, &[&str]); 11] = [
    ("VH-001", "candidate_for_comparison", &["opening_inventory_complete", "screen_integrity_known", "species_context_declared"]),
    ("VH-002", "protection_not_established", &["damage_not_silently_ignored", "maintenance_state_preserved"]),
    ("VH-003", "infeasible_or_requires_review", &["ventilation_floor_not_traded_away", "no_aggregate_score_compensation"]),
    ("VH-004", "comparable_only_under_matched_boundary_conditions", &["same_weather_trace", "same_occupancy_profile", "same_model_revision"]),
    ("VH-005", "risk_observation_requires_local_review", &["water_observation_source_bound", "no_unqualified_disease_prediction"]),
    ("VH-006", "unknown", &["stale_is_not_zero", "observation_window_preserved"]),
    ("VH-007", "out_of_domain", &["species_identity_preserved", "no_effect_size_transfer"]),
    ("VH-008", "rejected", &["spatial_resolution_preserved", "no_false_household_precision"]),
    ("VH-009", "rejected_or_quarantined", &["crs_validated", "transformation_lineage_retained"]),
    ("VH-010", "vector_endpoint_only", &["no_clinical_efficacy_claim", "measurement_tier_preserved"]),
    ("VH-011", "blocked_pending_professional_review", &["egress_is_hard_floor", "accessibility_is_hard_floor"]),
];

pub fn verify(bytes: &[u8]) -> Report {
    let source_sha256 = format!("{:x}", Sha256::digest(bytes));
    let mut errors = Vec::new();
    let mut case_count = 0;

    match serde_json::from_slice::<Manifest>(bytes) {
        Err(error) => errors.push(format!("strict_json_decode: {error}")),
        Ok(manifest) => {
            case_count = manifest.cases.len();
            if manifest.schema != SCHEMA { errors.push("schema_mismatch".into()); }
            if manifest.status != STATUS { errors.push("status_mismatch".into()); }
            if manifest.claim_ceiling != CLAIM_CEILING { errors.push("claim_ceiling_mismatch".into()); }
            if manifest.units_policy.trim().is_empty() { errors.push("units_policy_missing".into()); }
            if manifest.cases.len() != REQUIRED.len() {
                errors.push(format!("case_count_mismatch: expected {}, got {}", REQUIRED.len(), manifest.cases.len()));
            }

            let expected: BTreeMap<&str, (&str, &[&str])> = REQUIRED.iter()
                .map(|(id, state, assertions)| (*id, (*state, *assertions))).collect();
            let mut seen = BTreeSet::new();
            for case in &manifest.cases {
                if case.id.trim().is_empty() || !seen.insert(case.id.as_str()) {
                    errors.push(format!("empty_or_duplicate_id: {}", case.id));
                }
                if case.name.trim().is_empty() || case.class.trim().is_empty() || case.assertions.is_empty() {
                    errors.push(format!("incomplete_case: {}", case.id));
                }
                let actual: BTreeSet<&str> = case.assertions.iter().map(String::as_str).collect();
                if actual.len() != case.assertions.len() {
                    errors.push(format!("duplicate_assertion: {}", case.id));
                }
                match expected.get(case.id.as_str()) {
                    None => errors.push(format!("unexpected_case: {}", case.id)),
                    Some((state, assertions)) => {
                        if case.expected_state != *state {
                            errors.push(format!("state_mismatch: {} expected {}, got {}", case.id, state, case.expected_state));
                        }
                        for assertion in *assertions {
                            if !actual.contains(assertion) {
                                errors.push(format!("required_assertion_missing: {} -> {}", case.id, assertion));
                            }
                        }
                    }
                }
            }
            for (id, _, _) in REQUIRED {
                if !seen.contains(id) { errors.push(format!("missing_case: {id}")); }
            }
        }
    }

    errors.sort();
    Report {
        report_schema: "luminous.vector_habitat_verification.v0.1",
        status: if errors.is_empty() { "CONFORMS" } else { "REJECTED" },
        source_sha256,
        case_count,
        errors,
        claim_ceiling: "synthetic_fixture_semantic_conformance_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../docs/research/vector-habitat/VECTOR_HABITAT_CASES_V0_1.json");

    #[test]
    fn frozen_fixture_conforms() {
        let report = verify(FIXTURE.as_bytes());
        assert_eq!(report.status, "CONFORMS", "{:?}", report.errors);
        assert_eq!(report.case_count, REQUIRED.len());
        assert_eq!(report.source_sha256.len(), 64);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let changed = FIXTURE.replace("\"id\": \"VH-001\"", "\"id\": \"VH-001\", \"extra\": true");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("strict_json_decode:")));
    }

    #[test]
    fn clinical_claim_promotion_is_rejected() {
        let changed = FIXTURE.replace("\"expected_state\": \"vector_endpoint_only\"", "\"expected_state\": \"malaria_prevented\"");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("state_mismatch: VH-010")));
    }

    #[test]
    fn species_transfer_is_rejected() {
        let changed = FIXTURE.replace("\"expected_state\": \"out_of_domain\"", "\"expected_state\": \"candidate_for_comparison\"");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("state_mismatch: VH-007")));
    }

    #[test]
    fn duplicate_identity_is_rejected() {
        let changed = FIXTURE.replace("\"id\": \"VH-011\"", "\"id\": \"VH-010\"");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("empty_or_duplicate_id:")));
        assert!(report.errors.iter().any(|e| e == "missing_case: VH-011"));
    }
}
