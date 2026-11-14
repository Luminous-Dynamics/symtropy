// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Research-only independent oracle for a bounded energy-accounting identity.
//!
//! This does not call the production ThermodynamicLedger: it qualifies the
//! reference contract before any adapter or production behavior is changed.

use serde_json::{Value, json};
use std::collections::HashSet;

const CORPUS: &str = include_str!("data/thermo-ledger-reference-v1.json");
const ABS_TOL: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy)]
struct Balance {
    residual: f64,
    tolerance: f64,
    balanced: bool,
    inflow_total: f64,
    outflow_total: f64,
}

fn parse_finite(value: &Value, name: &str) -> Result<f64, String> {
    let raw = value
        .as_str()
        .ok_or_else(|| format!("{name}: expected decimal string"))?;
    let parsed = raw
        .parse::<f64>()
        .map_err(|error| format!("{name}: invalid scalar {raw:?}: {error}"))?;
    if !parsed.is_finite() {
        return Err(format!("{name}: scalar must be finite"));
    }
    Ok(parsed)
}

fn parse_nonnegative(value: &Value, name: &str) -> Result<f64, String> {
    let parsed = parse_finite(value, name)?;
    if parsed < 0.0 {
        return Err(format!("{name}: magnitude must be non-negative"));
    }
    Ok(parsed)
}

fn add_finite(total: f64, amount: f64, name: &str) -> Result<f64, String> {
    let next = total + amount;
    if !next.is_finite() {
        return Err(format!("{name}: accumulated sum overflowed"));
    }
    Ok(next)
}

/// Evaluate the declared boundary identity:
/// residual = initial + inflows - outflows - final.
///
/// Internal transfers are validated and deduplicated, but are not boundary
/// flows and therefore do not enter the total.
fn evaluate(case: &Value, allowed_categories: &[String]) -> Result<Balance, String> {
    let initial = parse_nonnegative(&case["initial_stored_energy"], "initial_stored_energy")?;
    let final_energy = parse_nonnegative(&case["final_stored_energy"], "final_stored_energy")?;
    let relative_tolerance = parse_finite(&case["relative_tolerance"], "relative_tolerance")?;
    if !(0.0 < relative_tolerance && relative_tolerance <= 1.0) {
        return Err("relative_tolerance must be in (0, 1]".to_owned());
    }

    let stores = case["tracked_stores"]
        .as_array()
        .ok_or_else(|| "tracked_stores must be an array".to_owned())?;
    let mut tracked = HashSet::new();
    for store in stores {
        let store = store
            .as_str()
            .ok_or_else(|| "tracked store names must be strings".to_owned())?;
        if store.is_empty() || !tracked.insert(store) {
            return Err("tracked store names must be non-empty and unique".to_owned());
        }
    }

    let mut event_ids = HashSet::new();
    let mut inflow_total = 0.0_f64;
    let mut outflow_total = 0.0_f64;

    for (field, is_inflow) in [("boundary_inflows", true), ("boundary_outflows", false)] {
        let flows = case[field]
            .as_array()
            .ok_or_else(|| format!("{field} must be an array"))?;
        for (index, flow) in flows.iter().enumerate() {
            let event_id = flow["event_id"]
                .as_str()
                .ok_or_else(|| format!("{field}[{index}].event_id must be a string"))?;
            if event_id.is_empty() || !event_ids.insert(event_id) {
                return Err(format!(
                    "{field}[{index}] has an empty or duplicate event ID"
                ));
            }

            let category = flow["category"]
                .as_str()
                .ok_or_else(|| format!("{field}[{index}].category must be a string"))?;
            if !allowed_categories.iter().any(|allowed| allowed == category) {
                return Err(format!("{field}[{index}] has an undocumented category"));
            }

            let amount = parse_nonnegative(&flow["amount"], &format!("{field}[{index}].amount"))?;
            if is_inflow {
                inflow_total = add_finite(inflow_total, amount, "boundary inflow sum")?;
            } else {
                outflow_total = add_finite(outflow_total, amount, "boundary outflow sum")?;
            }
        }
    }

    let transfers = case["internal_transfers"]
        .as_array()
        .ok_or_else(|| "internal_transfers must be an array".to_owned())?;
    for (index, transfer) in transfers.iter().enumerate() {
        let event_id = transfer["event_id"]
            .as_str()
            .ok_or_else(|| format!("internal_transfers[{index}].event_id must be a string"))?;
        if event_id.is_empty() || !event_ids.insert(event_id) {
            return Err(format!(
                "internal_transfers[{index}] has an empty or duplicate event ID"
            ));
        }
        let from = transfer["from_store"]
            .as_str()
            .ok_or_else(|| format!("internal_transfers[{index}].from_store must be a string"))?;
        let to = transfer["to_store"]
            .as_str()
            .ok_or_else(|| format!("internal_transfers[{index}].to_store must be a string"))?;
        if from == to || !tracked.contains(from) || !tracked.contains(to) {
            return Err(format!(
                "internal_transfers[{index}] must connect two different tracked stores"
            ));
        }
        let _amount = parse_nonnegative(
            &transfer["amount"],
            &format!("internal_transfers[{index}].amount"),
        )?;
    }

    let initial_plus_inflows = add_finite(initial, inflow_total, "initial energy plus inflows")?;
    let scale = add_finite(initial_plus_inflows, outflow_total, "balance scale")?;
    let scale = add_finite(scale, final_energy, "balance scale")?;
    let residual_before_final = initial_plus_inflows - outflow_total;
    if !residual_before_final.is_finite() {
        return Err("intermediate residual is non-finite".to_owned());
    }
    let residual = residual_before_final - final_energy;
    if !residual.is_finite() {
        return Err("residual is non-finite".to_owned());
    }
    let tolerance = relative_tolerance * scale.max(1.0);
    if !tolerance.is_finite() {
        return Err("effective tolerance is non-finite".to_owned());
    }

    Ok(Balance {
        residual,
        tolerance,
        balanced: residual.abs() <= tolerance,
        inflow_total,
        outflow_total,
    })
}

fn corpus() -> Value {
    serde_json::from_str(CORPUS).expect("frozen ledger corpus must be valid JSON")
}

fn cases(corpus: &Value) -> &[Value] {
    corpus["cases"]
        .as_array()
        .expect("frozen ledger cases must be an array")
}

fn case_by_id<'a>(cases: &'a [Value], id: &str) -> &'a Value {
    cases
        .iter()
        .find(|case| case["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("missing frozen ledger case {id}"))
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= ABS_TOL * expected.abs().max(1.0)
}

#[test]
fn frozen_corpus_matches_the_balance_identity_and_rejects_invalid_domains() {
    let corpus = corpus();
    assert_eq!(
        corpus["schema"].as_str(),
        Some("symtropy-thermo-ledger-reference-v1")
    );
    let corpus_cases = cases(&corpus);
    assert_eq!(corpus_cases.len(), 24, "unexpected frozen case count");
    assert_eq!(corpus["case_count"].as_u64(), Some(24));

    let categories = corpus["allowed_categories"]
        .as_array()
        .expect("allowed_categories must be an array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("allowed category must be a string")
                .to_owned()
        })
        .collect::<Vec<_>>();
    let controls = corpus["mutation_controls"]
        .as_array()
        .expect("mutation_controls must be an array");
    assert_eq!(controls.len(), 5, "unexpected mutation-control count");

    let mut ids = HashSet::new();
    let mut valid_count = 0;
    let mut invalid_count = 0;
    for case in corpus_cases {
        let id = case["id"].as_str().expect("each case must have an ID");
        assert!(ids.insert(id), "duplicate case ID {id}");
        let expected_valid = case["expected"]["valid"]
            .as_bool()
            .expect("expected.valid must be boolean");
        let result = evaluate(case, &categories);
        if expected_valid {
            valid_count += 1;
            let result =
                result.unwrap_or_else(|error| panic!("{id}: valid case rejected: {error}"));
            let expected_balanced = case["expected"]["balanced"]
                .as_bool()
                .expect("valid case needs expected.balanced");
            let expected_residual =
                parse_finite(&case["expected"]["residual"], "expected.residual")
                    .expect("valid case needs a finite expected residual");
            assert_eq!(
                result.balanced, expected_balanced,
                "{id}: unexpected balance decision (residual {})",
                result.residual
            );
            assert!(
                close(result.residual, expected_residual),
                "{id}: residual {} differs from expected {}",
                result.residual,
                expected_residual
            );
            assert_eq!(
                result.balanced,
                result.residual.abs() <= result.tolerance,
                "{id}: balance flag does not match its tolerance"
            );
        } else {
            invalid_count += 1;
            assert!(result.is_err(), "{id}: invalid case was accepted");
        }
    }
    assert_eq!(valid_count, 12, "unexpected valid-case count");
    assert_eq!(invalid_count, 12, "unexpected invalid-case count");
}

#[test]
fn equal_boundary_flows_do_not_hide_stored_energy_drift() {
    let corpus = corpus();
    let categories = corpus["allowed_categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .map(|value| value.as_str().expect("category string").to_owned())
        .collect::<Vec<_>>();
    let case = case_by_id(cases(&corpus), "TL-005-equal-flows-hide-storage-drift");
    let result = evaluate(case, &categories).expect("case is in the valid numeric domain");

    assert_eq!(result.inflow_total, result.outflow_total);
    assert!(!result.balanced);
    assert!(close(result.residual, 5.0));

    // The legacy-style flow-only proxy incorrectly declares zero residual here.
    let flow_only_proxy = result.inflow_total - result.outflow_total;
    assert_eq!(flow_only_proxy, 0.0);
    assert!(flow_only_proxy.abs() <= result.tolerance);
}

#[test]
fn internal_transfers_are_not_external_boundary_flows() {
    let corpus = corpus();
    let categories = corpus["allowed_categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .map(|value| value.as_str().expect("category string").to_owned())
        .collect::<Vec<_>>();
    let original = case_by_id(cases(&corpus), "TL-004-internal-transfer-not-boundary-flow");
    let correct = evaluate(original, &categories).expect("original case is valid");
    assert!(correct.balanced);

    let mut mutated = original.clone();
    mutated["boundary_inflows"] = json!([{
        "event_id": "mislabeled-internal-move",
        "category": "external_work",
        "amount": "25"
    }]);
    let wrong = evaluate(&mutated, &categories).expect("mutation remains in numeric domain");
    assert!(!wrong.balanced);
    assert!(close(wrong.residual, 25.0));
}

#[test]
fn deterministic_boundary_grid_balances_and_perturbations_fail() {
    let corpus = corpus();
    let categories = corpus["allowed_categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .map(|value| value.as_str().expect("category string").to_owned())
        .collect::<Vec<_>>();
    let energies = [0.0_f64, 1.0e-6, 1.0, 100.0, 1.0e6];
    let inflows = [0.0_f64, 0.1, 1.0, 1000.0];
    let outflows = [0.0_f64, 0.1, 1.0, 100.0];
    let mut checked = 0_usize;

    for initial in energies {
        for inflow in inflows {
            for outflow in outflows {
                let before_output = initial + inflow;
                if outflow > before_output {
                    continue;
                }
                let final_energy = before_output - outflow;
                let case = json!({
                    "initial_stored_energy": initial.to_string(),
                    "boundary_inflows": [{
                        "event_id": "grid-in",
                        "category": "external_work",
                        "amount": inflow.to_string()
                    }],
                    "boundary_outflows": [{
                        "event_id": "grid-out",
                        "category": "heat_rejected",
                        "amount": outflow.to_string()
                    }],
                    "tracked_stores": [],
                    "internal_transfers": [],
                    "relative_tolerance": "1e-12",
                    "final_stored_energy": final_energy.to_string()
                });
                let balanced =
                    evaluate(&case, &categories).expect("constructed grid case is valid");
                assert!(
                    balanced.balanced,
                    "constructed balance failed for initial={initial}, in={inflow}, out={outflow}: residual={}",
                    balanced.residual
                );

                let mut perturbed = case.clone();
                let disturbance = (initial + inflow + outflow + final_energy).max(1.0) * 1.0e-6;
                perturbed["final_stored_energy"] = json!((final_energy + disturbance).to_string());
                let unbalanced =
                    evaluate(&perturbed, &categories).expect("perturbed case remains valid");
                assert!(
                    !unbalanced.balanced,
                    "large final-storage perturbation went undetected"
                );
                checked += 1;
            }
        }
    }

    assert!(
        checked >= 50,
        "unexpectedly small deterministic grid: {checked}"
    );
}

#[test]
fn declared_semantic_mutations_have_frozen_counterexamples() {
    let corpus = corpus();
    let corpus_cases = cases(&corpus);
    let categories = corpus["allowed_categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .map(|value| value.as_str().expect("category string").to_owned())
        .collect::<Vec<_>>();

    let negative_outflow = case_by_id(
        corpus_cases,
        "TL-102-negative-outflow-regeneration-mutation",
    );
    assert!(evaluate(negative_outflow, &categories).is_err());

    let duplicate = case_by_id(corpus_cases, "TL-108-duplicate-boundary-event-id");
    assert!(evaluate(duplicate, &categories).is_err());

    let overflow = case_by_id(corpus_cases, "TL-105-inflow-sum-overflow");
    assert!(evaluate(overflow, &categories).is_err());

    let equal_flows = case_by_id(corpus_cases, "TL-005-equal-flows-hide-storage-drift");
    let checked = evaluate(equal_flows, &categories).expect("counterexample is valid");
    assert_eq!(checked.inflow_total - checked.outflow_total, 0.0);
    assert!(!checked.balanced);
}
