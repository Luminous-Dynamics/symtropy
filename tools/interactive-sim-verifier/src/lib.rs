use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "luminous.interactive_sim_cases.v0.1";
pub const STATUS: &str = "synthetic_non_operational";
pub const COMPARISON_POLICY: &str = "matched_workload_or_explicitly_non_comparable";
pub const EXPECTED_FIXTURE_SHA256: &str = "686cb2246cd0b5dd2f1307c5b620d798d653c21451a06970f3af337eaae00534";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub status: String,
    pub comparison_policy: String,
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

const REQUIRED: [(&str, &str, &[&str]); 8] = [
    ("IS-001", "exact_replay_hash_match", &["inputs_frozen", "entity_insertion_order_frozen", "timestep_frozen", "build_digest_pinned"]),
    ("IS-002", "bounded_error_reported", &["tolerance_preregistered", "no_unjustified_bit_identity_claim"]),
    ("IS-003", "fail", &["state_transition_id_shared", "mismatch_detected"]),
    ("IS-004", "compare_p95_to_16_7ms_target", &["raw_frame_time_trace_retained", "p99_and_hitches_reported"]),
    ("IS-005", "correction_and_divergence_reported", &["server_authority_declared", "resimulation_cost_measured", "player_visible_delay_measured"]),
    ("IS-006", "non_comparable_feature_disclosed", &["no_synthetic_equivalence", "exact_engine_versions_pinned"]),
    ("IS-007", "rejected", &["asset_digest_checked", "run_receipt_binds_exact_assets"]),
    ("IS-008", "all_frozen_state_transitions_match", &["fictional_only", "input_animation_audio_and_effect_state_coherent"]),
];

pub fn verify(bytes: &[u8]) -> Report {
    let source_sha256 = format!("{:x}", Sha256::digest(bytes));
    let mut errors = Vec::new();
    if source_sha256 != EXPECTED_FIXTURE_SHA256 {
        errors.push(format!(
            "fixture_digest_mismatch: expected {}, got {}",
            EXPECTED_FIXTURE_SHA256,
            source_sha256
        ));
    }
    let mut case_count = 0;

    match serde_json::from_slice::<Manifest>(bytes) {
        Err(error) => errors.push(format!("strict_json_decode: {error}")),
        Ok(manifest) => {
            case_count = manifest.cases.len();
            if manifest.schema != SCHEMA { errors.push("schema_mismatch".into()); }
            if manifest.status != STATUS { errors.push("status_mismatch".into()); }
            if manifest.comparison_policy != COMPARISON_POLICY { errors.push("comparison_policy_mismatch".into()); }
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
        report_schema: "luminous.interactive_sim_verification.v0.1",
        status: if errors.is_empty() { "CONFORMS" } else { "REJECTED" },
        source_sha256,
        case_count,
        errors,
        claim_ceiling: "synthetic_interactive_sim_semantics_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str =
        include_str!("../../../docs/research/interactive-sim/INTERACTIVE_SIM_CASES_V0_1.json");

    #[test]
    fn frozen_fixture_conforms() {
        let report = verify(FIXTURE.as_bytes());
        assert_eq!(report.status, "CONFORMS", "{:?}", report.errors);
        assert_eq!(report.case_count, REQUIRED.len());
        assert_eq!(report.source_sha256, EXPECTED_FIXTURE_SHA256);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let changed = FIXTURE.replace("\"id\": \"IS-001\"", "\"id\": \"IS-001\", \"extra\": true");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("strict_json_decode:")));
    }

    #[test]
    fn cross_platform_determinism_claim_must_remain_bounded() {
        let changed = FIXTURE.replace(
            "\"expected_state\": \"bounded_error_reported\"",
            "\"expected_state\": \"universal_bitwise_identity\"",
        );
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("state_mismatch: IS-002")));
    }

    #[test]
    fn mismatched_audio_animation_transition_must_fail() {
        let changed = FIXTURE.replace(
            "\"expected_state\": \"fail\"",
            "\"expected_state\": \"pass\"",
        );
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("state_mismatch: IS-003")));
    }

    #[test]
    fn fictional_only_boundary_is_mandatory() {
        let changed = FIXTURE.replace("\"fictional_only\"", "\"real_weapon_calibration\"");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("required_assertion_missing: IS-008")));
    }
    #[test]
    fn fixture_byte_change_is_rejected_by_digest() {
        let changed = FIXTURE.replace("same_build_replay", "same_build_replay ");
        assert_ne!(changed, FIXTURE);
        let report = verify(changed.as_bytes());
        assert_eq!(report.status, "REJECTED");
        assert!(report.errors.iter().any(|e| e.starts_with("fixture_digest_mismatch:")));
    }

}
