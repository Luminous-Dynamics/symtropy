// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Independent structural verifier for the frozen field-robot cost test inventory.
//! It does not import or execute the cost assessment implementation.

use std::collections::BTreeSet;
use std::process::ExitCode;

const MANIFEST: &str = include_str!("../../fixtures/field_robot_cost_cases.v1.tsv");
const TEST_SOURCE: &str = include_str!("../field_robot_cost.rs");
const EXPECTED_CASE_COUNT: usize = 25;
const HEADER: &str = "case_id\texpected_result\trationale";

fn verify() -> Result<(), String> {
    let records: Vec<&str> = MANIFEST
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    if records.first().copied() != Some(HEADER) {
        return Err("manifest header is missing or has drifted".into());
    }

    let rows = &records[1..];
    if rows.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} cases, found {}",
            rows.len()
        ));
    }

    let mut ids = BTreeSet::new();
    let mut dispositions = [0_usize; 4];
    for (index, row) in rows.iter().enumerate() {
        let columns: Vec<&str> = row.split('\t').collect();
        if columns.len() != 3 {
            return Err(format!(
                "row {} must contain exactly three TSV columns",
                index + 2
            ));
        }

        let id = columns[0];
        let disposition = columns[1];
        let rationale = columns[2];
        if id.trim().is_empty() || rationale.trim().is_empty() {
            return Err(format!("row {} has a blank ID or rationale", index + 2));
        }
        if !ids.insert(id) {
            return Err(format!("duplicate case ID: {id}"));
        }

        let disposition_index = match disposition {
            "CALCULATED_PROVISIONAL" => 0,
            "CALCULATED_SOURCE_REFERENCED" => 1,
            "BLOCKED" => 2,
            "ROUNDTRIP_VALID" => 3,
            _ => return Err(format!("unknown disposition {disposition:?} for {id}")),
        };
        dispositions[disposition_index] += 1;

        let expected_test_signature = format!("fn {id}(");
        if !TEST_SOURCE.contains(&expected_test_signature) {
            return Err(format!(
                "manifest case {id} has no corresponding Rust test function"
            ));
        }
    }

    println!("contract=FIELD-ROBOT-COST-BENCH-V1");
    println!("manifest_cases={}", rows.len());
    println!("calculated_provisional={}", dispositions[0]);
    println!("calculated_source_referenced={}", dispositions[1]);
    println!("blocked={}", dispositions[2]);
    println!("roundtrip_valid={}", dispositions[3]);
    println!("test_functions_matched={}", rows.len());
    println!("result=STRUCTURE_VALID_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    match verify() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("field-robot cost manifest verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
