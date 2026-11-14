//! Independent structural check for the frozen fixture corpus.
//!
//! This executable does not call the library normalizer. A PASS here means
//! only that the declared corpus inventory is complete and well-formed.

use std::collections::BTreeSet;
use std::process::ExitCode;

const CORPUS: &str = include_str!("../../fixtures/equilibrium-cases-v1.tsv");

const REQUIRED_CASES: [&str; 17] = [
    "valid-wb-per-radian",
    "explicit-weber-2pi",
    "explicit-mm-and-ms-units",
    "unknown-psi-unit",
    "missing-cocos-flags",
    "missing-quantity-signs",
    "cocos-sign-mismatch",
    "rz-transposed",
    "nonmonotonic-r",
    "nonmonotonic-z",
    "shape-mismatch",
    "nonfinite-psi",
    "static-two-frames",
    "evolving-one-frame",
    "nonmonotonic-time",
    "stale-geometry-receipt",
    "fec-unknown-psi-convention",
];

fn main() -> ExitCode {
    match audit() {
        Ok(()) => {
            println!(
                "PASS: {} frozen rows; required identities, expected dispositions, and mutation classes present",
                REQUIRED_CASES.len()
            );
            println!("SCOPE: structural corpus audit only; library behavior and physical correctness are not evaluated");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn audit() -> Result<(), String> {
    let mut lines = CORPUS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));

    let header = lines.next().ok_or("missing header")?;
    if header != "case_id|expected|mutation" {
        return Err(format!("unexpected header: {header}"));
    }

    let mut ids = BTreeSet::new();
    let mut rows = 0usize;
    let mut mutations = BTreeSet::new();

    for line in lines {
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 3 {
            return Err(format!("row must have three pipe-delimited fields: {line}"));
        }
        let [case_id, expected, mutation] = [fields[0], fields[1], fields[2]];
        if case_id.is_empty() || !ids.insert(case_id) {
            return Err(format!("empty or duplicate case_id: {case_id}"));
        }
        if expected != "ACCEPT" && expected != "REJECT" {
            return Err(format!("invalid expected disposition for {case_id}: {expected}"));
        }
        if mutation.is_empty() {
            return Err(format!("missing mutation class for {case_id}"));
        }
        mutations.insert(mutation);
        rows += 1;
    }

    if rows != REQUIRED_CASES.len() {
        return Err(format!(
            "expected {} rows, found {rows}",
            REQUIRED_CASES.len()
        ));
    }

    for required in REQUIRED_CASES {
        if !ids.contains(required) {
            return Err(format!("required case missing: {required}"));
        }
    }

    for required_mutation in [
        "none",
        "unknown_flux_unit",
        "missing_cocos_flags",
        "missing_quantity_signs",
        "convention_mismatch",
        "unsupported_frame_layout",
        "nonmonotonic_r",
        "nonmonotonic_z",
        "shape_mismatch",
        "nonfinite_psi",
        "static_frame_count",
        "evolving_frame_count",
        "nonmonotonic_time",
        "stale_geometry",
    ] {
        if !mutations.contains(required_mutation) {
            return Err(format!("required mutation class missing: {required_mutation}"));
        }
    }

    Ok(())
}
