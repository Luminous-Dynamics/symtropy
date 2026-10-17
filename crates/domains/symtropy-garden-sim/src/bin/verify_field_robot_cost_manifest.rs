// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Independent structural verifier for the frozen field-robot cost test inventories.
//! This binary reads source as text; it does not import or execute either cost evaluator.

use std::collections::BTreeSet;
use std::process::ExitCode;

const COST_MANIFEST: &str = include_str!("../../fixtures/field_robot_cost_cases.v1.tsv");
const SENSITIVITY_MANIFEST: &str =
    include_str!("../../fixtures/field_robot_cost_sensitivity_cases.v1.tsv");
const COST_TEST_SOURCE: &str = include_str!("../field_robot_cost.rs");
const SENSITIVITY_TEST_SOURCE: &str = include_str!("../field_robot_cost_sensitivity.rs");

const COST_EXPECTED_FNV1A64: u64 = 0x990b_499f_f888_627e;
const SENSITIVITY_EXPECTED_FNV1A64: u64 = 0xb9ae_2c78_717a_7705;

const COST_EXPECTED: &[(&str, &str)] = &[
    ("baseline_cost_components_match_hand_calculated_oracle", "CALCULATED_PROVISIONAL"),
    ("complete_source_references_change_evidence_disposition_not_math", "CALCULATED_SOURCE_REFERENCED"),
    ("estimated_route_data_makes_result_provisional", "CALCULATED_PROVISIONAL"),
    ("missing_cost_evidence_blocks_calculation", "BLOCKED"),
    ("stale_evidence_blocks_even_when_numeric_values_look_favorable", "BLOCKED"),
    ("conflicting_repair_evidence_blocks_ranking_inputs", "BLOCKED"),
    ("duplicate_evidence_domains_block", "BLOCKED"),
    ("missing_evidence_domain_blocks", "BLOCKED"),
    ("malformed_source_digest_blocks", "BLOCKED"),
    ("invalid_currency_code_blocks", "BLOCKED"),
    ("unsupported_schema_blocks", "BLOCKED"),
    ("zero_output_blocks_per_unit_cost", "BLOCKED"),
    ("zero_completed_area_blocks_hectare_cost", "BLOCKED"),
    ("zero_useful_life_blocks_capital_annualization", "BLOCKED"),
    ("salvage_value_above_acquisition_cost_blocks", "BLOCKED"),
    ("supervision_is_subset_of_labor_not_double_counted", "BLOCKED"),
    ("aggregate_operator_labor_can_exceed_one_machine_calendar_year", "CALCULATED_PROVISIONAL"),
    ("zero_route_missions_have_zero_route_and_mobilization_cost", "CALCULATED_PROVISIONAL"),
    ("route_distance_increases_logistics_cost", "CALCULATED_PROVISIONAL"),
    ("no_robot_baseline_is_modelled_without_fake_capital_cost", "CALCULATED_PROVISIONAL"),
    ("exact_ceiling_arithmetic_never_rounds_cost_down", "CALCULATED_PROVISIONAL"),
    ("route_distance_overflow_blocks_without_wrapping", "BLOCKED"),
    ("annual_time_over_budget_blocks", "BLOCKED"),
    ("total_cost_overflow_blocks_without_partial_breakdown", "BLOCKED"),
    ("exact_integer_cost_values_round_trip_as_decimal_strings", "ROUNDTRIP_VALID"),
];

const SENSITIVITY_EXPECTED: &[(&str, &str)] = &[
    ("hand_calculated_route_sweep_produces_exact_observed_ranges", "COMPLETE_PROVISIONAL"),
    ("stale_case_yields_partial_not_a_complete_bound", "PARTIAL"),
    ("all_blocked_scenarios_return_no_numeric_range", "BLOCKED"),
    ("input_order_permutation_produces_identical_report", "COMPLETE_PROVISIONAL"),
    ("duplicate_scenario_ids_block_the_whole_request", "BLOCKED"),
    ("scope_period_currency_and_unit_mismatches_block_the_whole_request", "BLOCKED"),
    ("area_denominator_mode_mismatch_blocks_hectare_range", "BLOCKED"),
    ("zero_scenarios_blocks_the_request", "BLOCKED"),
    ("sensitivity_report_round_trips_exact_integer_ranges", "ROUNDTRIP_VALID"),
];

const COST_HEADER: &str = "case_id\texpected_result\trationale";
const SENSITIVITY_HEADER: &str = "case_id\texpected_disposition\trationale";

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn verify_manifest(
    name: &str,
    manifest: &str,
    header: &str,
    expected: &[(&str, &str)],
    test_source: &str,
    expected_fingerprint: u64,
) -> Result<usize, String> {
    let fingerprint = fnv1a64(manifest.as_bytes());
    if fingerprint != expected_fingerprint {
        return Err(format!(
            "{name} fingerprint changed: expected {expected_fingerprint:016x}, got {fingerprint:016x}; review the corpus and update the pin intentionally"
        ));
    }

    let records: Vec<&str> = manifest
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    if records.first().copied() != Some(header) {
        return Err(format!("{name} manifest header has drifted"));
    }

    let rows = &records[1..];
    if rows.len() != expected.len() {
        return Err(format!(
            "{name} case count mismatch: expected {}, found {}",
            expected.len(),
            rows.len()
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, (row, expected_case)) in rows.iter().zip(expected.iter()).enumerate() {
        let columns: Vec<&str> = row.split('\t').collect();
        if columns.len() != 3 {
            return Err(format!("{name} row {} must have exactly three TSV columns", index + 2));
        }

        let id = columns[0];
        let disposition = columns[1];
        let rationale = columns[2];
        if id != expected_case.0 || disposition != expected_case.1 {
            return Err(format!(
                "{name} row {} mismatch: expected ({:?}, {:?}), found ({:?}, {:?})",
                index + 2,
                expected_case.0,
                expected_case.1,
                id,
                disposition
            ));
        }
        if id.trim().is_empty() || rationale.trim().is_empty() {
            return Err(format!("{name} row {} has a blank ID or rationale", index + 2));
        }
        if !ids.insert(id) {
            return Err(format!("{name} has duplicate case ID {id}"));
        }

        let expected_test_signature = format!("fn {id}(");
        if !test_source.contains(&expected_test_signature) {
            return Err(format!("{name} case {id} has no corresponding Rust test function"));
        }
    }

    println!("{name}_cases={}", rows.len());
    println!("{name}_fnv1a64={fingerprint:016x} (non-cryptographic fingerprint)");
    println!("{name}_test_functions_matched={}", rows.len());
    Ok(rows.len())
}

fn verify() -> Result<(), String> {
    let cost_count = verify_manifest(
        "cost",
        COST_MANIFEST,
        COST_HEADER,
        COST_EXPECTED,
        COST_TEST_SOURCE,
        COST_EXPECTED_FNV1A64,
    )?;
    let sensitivity_count = verify_manifest(
        "sensitivity",
        SENSITIVITY_MANIFEST,
        SENSITIVITY_HEADER,
        SENSITIVITY_EXPECTED,
        SENSITIVITY_TEST_SOURCE,
        SENSITIVITY_EXPECTED_FNV1A64,
    )?;

    println!("total_case_count={}", cost_count + sensitivity_count);
    println!("result=STRUCTURE_VALID_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    match verify() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("field-robot cost/sensitivity manifest verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
