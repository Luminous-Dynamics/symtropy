// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Structural manifest verifier; does not import the repair-recovery evaluator.

use std::collections::BTreeSet;
use std::process::ExitCode;

const MANIFEST: &str = include_str!("../../fixtures/repair_recovery_cases.v1.tsv");
const TEST_SOURCE: &str = include_str!("../../tests/repair_recovery_independent_oracle.rs");
const EXPECTED_CASE_COUNT: usize = 28;
const HEADER: &str = "case_id\texpected_result\trationale";
const DISPOSITIONS: [&str; 2] = ["CALCULATED", "BLOCKED"];

fn verify() -> Result<(), String> {
    let records: Vec<&str> = MANIFEST
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    if records.first().copied() != Some(HEADER) {
        return Err("repair-recovery manifest header missing or drifted".into());
    }
    let rows = &records[1..];
    if rows.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} repair-recovery cases, found {}",
            rows.len()
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let columns: Vec<&str> = row.split('\t').collect();
        if columns.len() != 3 {
            return Err(format!("row {} must have exactly three columns", index + 2));
        }
        let id = columns[0];
        if id.is_empty() || !ids.insert(id) {
            return Err(format!("row {} has empty or duplicate case ID", index + 2));
        }
        if !DISPOSITIONS.contains(&columns[1]) {
            return Err(format!("row {} has unknown expected result", index + 2));
        }
        if columns[2].trim().is_empty() {
            return Err(format!("row {} has no rationale", index + 2));
        }
        if !TEST_SOURCE.contains(&format!("fn {id}(")) {
            return Err(format!(
                "manifest case {id} has no matching independent test function"
            ));
        }
    }
    println!("STRUCTURE_VALID_ONLY cases={}", rows.len());
    println!("This verifies inventory structure, not tests or observed fleet reliability.");
    Ok(())
}
fn main() -> ExitCode {
    match verify() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("repair-recovery manifest rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
