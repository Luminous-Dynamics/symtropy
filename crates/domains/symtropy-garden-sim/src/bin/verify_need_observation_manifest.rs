// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Independent structural guard for the frozen AGRI-ROBOT-SERVICE-009 need-observation oracle.
//! This validates inventory/test wiring only; the Rust tests must run separately.

use std::collections::BTreeSet;
use std::process::ExitCode;

const MANIFEST: &str = include_str!("../../fixtures/need_observation_cases.v1.tsv");
const TEST_SOURCE: &str = include_str!("../../tests/need_observation_independent_oracle.rs");
const EXPECTED_CASE_COUNT: usize = 22;
const HEADER: &str = "case_id\texpected_result\trationale";
const EXPECTED: [(&str, &str); EXPECTED_CASE_COUNT] = [
    ("valid_mixed_need_and_request_denominators_are_separate", "CALCULATED"),
    ("prospective_retrospective_and_relay_basis_remain_distinct", "CALCULATED"),
    ("retrospective_basis_requires_retrospective_need_timing", "BLOCKED"),
    ("a_farmer_confirmed_no_request_has_no_invented_request_reference", "CALCULATED"),
    ("farmer_reported_off_platform_service_never_becomes_canonical_attainment", "CALCULATED"),
    ("authorized_relayed_other_channel_attainment_stays_noncanonical", "CALCULATED"),
    ("absent_optional_barrier_answer_is_not_imputed", "CALCULATED"),
    ("open_deferred_and_missing_outcomes_stay_distinct", "CALCULATED"),
    ("request_presence_link_requires_real_opaque_request_ref", "BLOCKED"),
    ("no_request_state_cannot_carry_fabricated_request_ref", "BLOCKED"),
    ("telemetry_non_use_cannot_create_a_need_episode", "BLOCKED"),
    ("unverified_intermediary_relay_cannot_count_as_confirmed_need", "BLOCKED"),
    ("authorized_relay_requires_an_opaque_authority_reference", "BLOCKED"),
    ("duplicate_need_references_block_the_report", "BLOCKED"),
    ("episode_must_match_preregistered_task_scope_period_and_window", "BLOCKED"),
    ("screen_counts_and_exclusion_classes_must_reconcile", "BLOCKED"),
    ("no_request_reason_is_not_allowed_on_linked_request", "BLOCKED"),
    ("canonical_service_attainment_requires_a_linked_canonical_request", "BLOCKED"),
    ("unknown_outcome_cannot_claim_positive_service_evidence", "BLOCKED"),
    ("empty_eligible_need_cohort_is_not_assessable", "NOT_ASSESSABLE"),
    ("unsafe_personal_or_url_reference_blocks_public_aggregate", "BLOCKED"),
    ("unknown_fields_are_rejected", "REJECTED"),
];

fn verify() -> Result<(), String> {
    let records: Vec<&str> = MANIFEST
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    if records.first().copied() != Some(HEADER) {
        return Err("need-observation manifest header missing or drifted".into());
    }
    let rows = &records[1..];
    if rows.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} need-observation cases, found {}",
            rows.len()
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, (row, (expected_id, expected_result))) in
        rows.iter().zip(EXPECTED.iter()).enumerate()
    {
        let columns: Vec<&str> = row.split('\t').collect();
        if columns.len() != 3 {
            return Err(format!("row {} must have exactly three columns", index + 2));
        }
        let [id, result, rationale] = [columns[0], columns[1], columns[2]];
        if id != *expected_id || result != *expected_result {
            return Err(format!(
                "row {} drifted: expected {expected_id} / {expected_result}, found {id} / {result}",
                index + 2
            ));
        }
        if !ids.insert(id) {
            return Err(format!("row {} has a duplicate case ID", index + 2));
        }
        if rationale.trim().is_empty() {
            return Err(format!("row {} has no rationale", index + 2));
        }
        if !TEST_SOURCE.contains(&format!("fn {id}(")) {
            return Err(format!("manifest case {id} has no matching independent test"));
        }
    }

    println!("STRUCTURE_VALID_ONLY cases={}", rows.len());
    println!("This checks case inventory structure and expected-result labels, not Rust test outcomes or field claims.");
    Ok(())
}

fn main() -> ExitCode {
    match verify() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("need-observation case manifest rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
