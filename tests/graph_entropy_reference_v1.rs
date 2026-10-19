// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Executable arithmetic oracle for the research-only graph entropy corpus.
//!
//! This is intentionally separate from production physics and does not claim
//! that Symtropy implements the graph heat-flow system. It validates the
//! frozen fixture contract and the sign/permutation properties it specifies.

use serde_json::Value;

const CORPUS: &str = include_str!("../docs/research/data/graph-entropy-reference-v1.json");
const REL_TOL: f64 = 1.0e-12;

fn corpus() -> Value {
    serde_json::from_str(CORPUS).expect("graph entropy reference corpus must be valid JSON")
}

fn parse_scalar(value: &Value, context: &str) -> Result<f64, String> {
    let raw = value
        .as_str()
        .ok_or_else(|| format!("{context}: expected decimal string"))?;
    let parsed = raw
        .parse::<f64>()
        .map_err(|error| format!("{context}: invalid number {raw:?}: {error}"))?;
    if !parsed.is_finite() {
        return Err(format!("{context}: non-finite number"));
    }
    Ok(parsed)
}

/// Compute D(h) = sum_e c_e * (h_src-h_dst) * (ln(h_src)-ln(h_dst)).
/// Input-domain checks happen before evaluating any logarithm.
fn evaluate_dissipation(case: &Value) -> Result<f64, String> {
    let nodes = case["nodes"]
        .as_u64()
        .ok_or_else(|| "nodes must be a non-negative integer".to_owned())? as usize;
    let state = case["state"]
        .as_array()
        .ok_or_else(|| "state must be an array".to_owned())?;
    if state.len() != nodes {
        return Err("state length must equal nodes".to_owned());
    }

    let mut h = Vec::with_capacity(state.len());
    for (index, value) in state.iter().enumerate() {
        let value = parse_scalar(value, &format!("state[{index}]"))?;
        if value <= 0.0 {
            return Err(format!("state[{index}] must be strictly positive"));
        }
        h.push(value);
    }

    let edges = case["edges"]
        .as_array()
        .ok_or_else(|| "edges must be an array".to_owned())?;
    let mut total = 0.0_f64;
    for (index, edge) in edges.iter().enumerate() {
        let src = edge["src"]
            .as_u64()
            .ok_or_else(|| format!("edge[{index}].src must be a non-negative integer"))?
            as usize;
        let dst = edge["dst"]
            .as_u64()
            .ok_or_else(|| format!("edge[{index}].dst must be a non-negative integer"))?
            as usize;
        if src >= nodes || dst >= nodes {
            return Err(format!("edge[{index}] endpoint outside node range"));
        }

        let conductance =
            parse_scalar(&edge["conductance"], &format!("edge[{index}].conductance"))?;
        if conductance < 0.0 {
            return Err(format!("edge[{index}] conductance must be non-negative"));
        }

        let hs = h[src];
        let hd = h[dst];
        let contribution = conductance * (hs - hd) * (hs.ln() - hd.ln());
        if !contribution.is_finite() {
            return Err(format!("edge[{index}] produced a non-finite contribution"));
        }
        total += contribution;
        if !total.is_finite() {
            return Err("dissipation sum is non-finite".to_owned());
        }
    }
    Ok(total)
}

/// Evaluate only the deliberately small symbolic vocabulary frozen in v1.
/// Adding a new symbolic expression requires an explicit reviewed case.
fn expected_expression(expression: &str) -> f64 {
    match expression {
        "0" => 0.0,
        "ln(2)" => 2.0_f64.ln(),
        "4*ln(2)" => 4.0 * 2.0_f64.ln(),
        "ln(2)+2*ln(3)" => 2.0_f64.ln() + 2.0 * 3.0_f64.ln(),
        other => panic!("unreviewed symbolic expression in reference corpus: {other}"),
    }
}

fn close_enough(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= REL_TOL * expected.abs().max(1.0)
}

#[test]
fn corpus_cases_match_symbolic_expectations_and_domains() {
    let corpus = corpus();
    let cases = corpus["cases"]
        .as_array()
        .expect("corpus cases must be an array");
    assert_eq!(cases.len(), 11, "unexpected reference-corpus case count");

    for case in cases {
        let id = case["id"].as_str().expect("each case needs a stable ID");
        let valid = case["expected"]["valid"]
            .as_bool()
            .expect("each expectation needs a valid flag");
        let result = evaluate_dissipation(case);

        if valid {
            let actual =
                result.unwrap_or_else(|error| panic!("{id}: valid case rejected: {error}"));
            let expression = case["expected"]["dissipation_expression"]
                .as_str()
                .unwrap_or_else(|| panic!("{id}: valid case lacks symbolic expectation"));
            let expected = expected_expression(expression);
            assert!(actual.is_finite(), "{id}: result is not finite");
            assert!(
                actual >= -REL_TOL,
                "{id}: dissipation must be non-negative, got {actual}"
            );
            assert!(
                close_enough(actual, expected),
                "{id}: actual {actual:.17e} differs from {expression} ({expected:.17e})"
            );
            match case["expected"]["relation_to_zero"].as_str() {
                Some("strictly_positive") => {
                    assert!(actual > 0.0, "{id}: expected strict positivity")
                }
                Some("equal") => assert_eq!(actual, 0.0, "{id}: expected exact zero"),
                other => panic!("{id}: unknown relation-to-zero expectation {other:?}"),
            }
        } else {
            assert!(result.is_err(), "{id}: invalid-domain case was accepted");
        }
    }
}

#[test]
fn reversed_edge_and_vertex_permutation_preserve_the_declared_values() {
    let corpus = corpus();
    let cases = corpus["cases"].as_array().expect("cases array");
    let by_id = |id: &str| {
        cases
            .iter()
            .find(|case| case["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("missing case {id}"))
    };

    let forward =
        evaluate_dissipation(by_id("GE-001-two-node-strict")).expect("forward case valid");
    let reverse = evaluate_dissipation(by_id("GE-007-reversed-edge-enumeration"))
        .expect("reverse case valid");
    let path = evaluate_dissipation(by_id("GE-005-three-node-path")).expect("path case valid");
    let permuted =
        evaluate_dissipation(by_id("GE-008-permuted-path")).expect("permuted path valid");

    assert!(
        close_enough(forward, reverse),
        "reversing both edge differences must preserve D(h)"
    );
    assert!(
        close_enough(path, permuted),
        "vertex relabeling must preserve D(h)"
    );
}

#[test]
fn negative_control_mutations_are_detected() {
    let corpus = corpus();
    let cases = corpus["cases"].as_array().expect("cases array");
    let witness = cases
        .iter()
        .find(|case| case["id"].as_str() == Some("GE-001-two-node-strict"))
        .expect("positive witness case exists");
    let edge = &witness["edges"][0];
    let src = witness["state"][edge["src"].as_u64().expect("src") as usize]
        .as_str()
        .expect("source state")
        .parse::<f64>()
        .expect("source decimal");
    let dst = witness["state"][edge["dst"].as_u64().expect("dst") as usize]
        .as_str()
        .expect("destination state")
        .parse::<f64>()
        .expect("destination decimal");
    let conductance =
        parse_scalar(&edge["conductance"], "witness conductance").expect("conductance");
    let wrong_sign = conductance * (src - dst) * (dst.ln() - src.ln());
    assert!(
        wrong_sign < 0.0,
        "reversing only the logarithmic difference must violate non-negativity"
    );

    let controls = corpus["mutation_controls"]
        .as_array()
        .expect("mutation controls must be an array");
    assert_eq!(controls.len(), 3, "unexpected mutation-control count");

    for invalid_id in [
        "GE-NEG-001-negative-conductance",
        "GE-NEG-002-zero-state",
        "GE-NEG-003-negative-state",
    ] {
        let invalid = cases
            .iter()
            .find(|case| case["id"].as_str() == Some(invalid_id))
            .unwrap_or_else(|| panic!("missing invalid case {invalid_id}"));
        assert!(
            evaluate_dissipation(invalid).is_err(),
            "{invalid_id}: invalid input must fail closed"
        );
    }
}

#[test]
fn deterministic_edge_grid_preserves_dissipation_sign() {
    // A deterministic property-style sweep broadens the hand-authored corpus
    // without adding a random-number dependency or introducing flaky tests.
    let states = [1.0e-6_f64, 0.01, 0.1, 0.5, 1.0, 2.0, 10.0, 1.0e3, 1.0e6];
    let conductances = [0.0_f64, 0.125, 0.5, 1.0, 10.0];
    let mut checked = 0_usize;

    for &a in &states {
        for &b in &states {
            for &conductance in &conductances {
                let contribution = conductance * (a - b) * (a.ln() - b.ln());
                assert!(
                    contribution.is_finite(),
                    "finite domain produced non-finite result"
                );
                assert!(
                    contribution >= -REL_TOL,
                    "negative dissipation for a={a}, b={b}, c={conductance}: {contribution}"
                );
                if conductance > 0.0 && a != b {
                    assert!(
                        contribution > 0.0,
                        "strict monotonicity expected for unequal positive states: a={a}, b={b}, c={conductance}"
                    );
                }
                if conductance == 0.0 || a == b {
                    assert_eq!(
                        contribution, 0.0,
                        "zero-conductance or equal-state edge must dissipate zero"
                    );
                }
                checked += 1;
            }
        }
    }

    assert_eq!(checked, 405, "unexpected deterministic property-grid size");
}

#[test]
fn hostile_edge_and_non_finite_mutations_fail_closed() {
    let corpus = corpus();
    let cases = corpus["cases"].as_array().expect("cases array");
    let valid = cases
        .iter()
        .find(|case| case["id"].as_str() == Some("GE-001-two-node-strict"))
        .expect("positive witness case exists");

    let mut nan_conductance = valid.clone();
    nan_conductance["edges"][0]["conductance"] = Value::String("NaN".to_owned());
    assert!(
        evaluate_dissipation(&nan_conductance).is_err(),
        "NaN conductance must fail closed"
    );

    let mut infinite_conductance = valid.clone();
    infinite_conductance["edges"][0]["conductance"] = Value::String("Infinity".to_owned());
    assert!(
        evaluate_dissipation(&infinite_conductance).is_err(),
        "infinite conductance must fail closed"
    );

    let mut invalid_endpoint = valid.clone();
    invalid_endpoint["edges"][0]["src"] = Value::from(2_u64);
    assert!(
        evaluate_dissipation(&invalid_endpoint).is_err(),
        "out-of-range edge endpoint must fail closed"
    );

    let mut nan_state = valid.clone();
    nan_state["state"][0] = Value::String("NaN".to_owned());
    assert!(
        evaluate_dissipation(&nan_state).is_err(),
        "NaN state must fail closed before logarithm evaluation"
    );
}
