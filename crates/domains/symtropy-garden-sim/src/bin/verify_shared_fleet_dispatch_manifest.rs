// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Structural verifier for the frozen shared-fleet dispatch benchmark inventory.
//! This program does not invoke the dispatch scheduler or evaluate expected outcomes.

use std::collections::BTreeSet;
use std::process::ExitCode;

const MANIFEST: &str = include_str!("../../fixtures/shared_fleet_dispatch_cases.v1.tsv");
const TEST_SOURCE: &str = include_str!("../../tests/shared_fleet_dispatch_independent_oracle.rs");
const EXPECTED_CASE_COUNT: usize = 14;
const HEADER: &str = "case_id\texpected_result\trationale";
const DISPOSITIONS: [&str; 2] = ["CALCULATED", "BLOCKED"];

fn verify() -> Result<(), String> {
    let records: Vec<&str> = MANIFEST
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    if records.first().copied() != Some(HEADER) {
        return Err("shared-fleet manifest header missing or drifted".into());
    }

    let rows = &records[1..];
    if rows.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} shared-fleet cases, found {}",
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
            return Err(format!(
                "row {} has an empty or duplicate case ID",
                index + 2
            ));
        }
        if !DISPOSITIONS.contains(&columns[1]) {
            return Err(format!("row {} has an unknown expected result", index + 2));
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
    println!(
        "This validates manifest/test-name structure, not Rust test outcomes or field claims."
    );
    Ok(())
}

fn main() -> ExitCode {
    match verify() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("shared-fleet manifest rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
