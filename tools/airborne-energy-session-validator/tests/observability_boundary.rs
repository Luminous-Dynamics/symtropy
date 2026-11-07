// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Integration seam between the aerial session lifecycle and the upstream
//! receiver-pose observability evaluator. These fixtures are synthetic only.

use airborne_energy_observability_validator::validate_json as validate_observability;
use airborne_energy_session_validator::validate_json as validate_session;

const UNIQUE_POSE_MAP: &str = include_str!(
    "../../../research/airborne-energy-observability-v0/observability.unique.synthetic.json"
);
const AMBIGUOUS_POSE_MAP: &str = include_str!(
    "../../../research/airborne-energy-observability-v0/observability.ambiguous.synthetic.json"
);
const NOT_RUN_POSE_MAP: &str = include_str!(
    "../../../research/airborne-energy-observability-v0/observability.NOT_RUN.template.json"
);
const SESSION_REACQUIRE: &str = include_str!(
    "../../../research/airborne-energy-session-v0/session.synthetic-reacquire.example.json"
);

#[test]
fn unique_pose_map_is_a_separate_synthetic_input_to_session_admission() {
    let observability = validate_observability(UNIQUE_POSE_MAP).unwrap();
    assert_eq!(
        observability.disposition,
        "UNIQUE_WITHIN_DECLARED_SAMPLE_SET"
    );
    assert_eq!(observability.unique_sample_numerator(), 3);
    assert_eq!(observability.unique_sample_denominator(), 3);

    let session = validate_session(SESSION_REACQUIRE).unwrap();
    assert_eq!(session.final_state, "COMPLETE");
    assert_eq!(session.power_active_intervals, 2);
}

#[test]
fn ambiguous_high_power_pose_map_is_not_a_unique_observability_result() {
    let report = validate_observability(AMBIGUOUS_POSE_MAP).unwrap();
    assert_eq!(report.disposition, "AMBIGUOUS");
    assert_eq!(report.ambiguous_pose_samples, 3);
    assert_eq!(report.alias_pairs.len(), 3);
}

#[test]
fn out_of_domain_samples_do_not_expand_the_admitted_observability_set() {
    let report = validate_observability(UNIQUE_POSE_MAP).unwrap();
    assert_eq!(report.total_pose_samples, 4);
    assert_eq!(report.in_domain_pose_samples, 3);
    assert_eq!(report.out_of_domain_pose_samples, 1);
}

#[test]
fn not_run_observability_template_is_not_a_physical_receipt_or_pass() {
    let report = validate_observability(NOT_RUN_POSE_MAP).unwrap();
    assert_eq!(report.disposition, "NOT_RUN");
    assert_eq!(report.in_domain_pose_samples, 0);
    assert!(report.alias_pairs.is_empty());
}
