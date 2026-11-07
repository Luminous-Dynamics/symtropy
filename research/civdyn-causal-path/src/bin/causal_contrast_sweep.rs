// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic paired-contrast controls for causal-path qualification.
//! These fixtures are not human data or real-world effect estimates.

use civdyn_causal_path::paired_effects::{
    ContrastContract, ContrastDataset, ExpectedResponse, PairingDesign, RunObservation,
    evaluate_paired_contrast,
};
use civdyn_causal_path::temporal_graph::{TemporalEdge, TemporalGraph, TemporalVariable};
use std::collections::BTreeMap;

const CSV_COLUMNS: usize = 13;

fn graph(has_path: bool) -> TemporalGraph {
    let variables = vec![
        TemporalVariable {
            id: "intervention".into(),
            unit: "flag".into(),
        },
        TemporalVariable {
            id: "outcome".into(),
            unit: "fraction".into(),
        },
    ];
    let edges = if has_path {
        vec![TemporalEdge {
            id: "intervention_to_outcome".into(),
            from: "intervention".into(),
            to: "outcome".into(),
            lag_steps: 0,
            mechanism_id: "declared_service_mechanism".into(),
            unit_transform_id: Some("flag_to_fraction_response".into()),
        }]
    } else {
        Vec::new()
    };
    TemporalGraph { variables, edges }
}

fn dataset(design: PairingDesign, difference: impl Fn(usize) -> f64) -> ContrastDataset {
    let mut control = Vec::new();
    let mut treatment = Vec::new();
    for seed in 0..30_u64 {
        let index = seed as usize;
        let baseline = 0.25 + index as f64 * 0.001;
        control.push(RunObservation {
            seed,
            values: BTreeMap::from([("service".into(), baseline)]),
        });
        treatment.push(RunObservation {
            seed,
            values: BTreeMap::from([("service".into(), baseline + difference(index))]),
        });
    }
    ContrastDataset {
        design,
        control,
        treatment,
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "paired contrast CSV width changed"
    );
    println!("{}", fields.join(","));
}

fn run_case(
    scenario: &str,
    design: PairingDesign,
    difference: impl Fn(usize) -> f64,
    has_path: bool,
    expected: ExpectedResponse,
    margin: f64,
) {
    let graph = graph(has_path);
    let path = graph
        .has_path("intervention", "outcome")
        .expect("graph must validate");
    assert_eq!(path, has_path);

    let result = evaluate_paired_contrast(
        &dataset(design, difference),
        &ContrastContract {
            outcome_variable_id: "service".into(),
            expected_response: expected,
            equivalence_margin: margin,
        },
        path,
    )
    .expect("paired synthetic data must validate");

    emit(vec![
        "paired_contrast".into(),
        scenario.into(),
        match design {
            PairingDesign::CommonRandomNumbers => "common_random_numbers".into(),
            PairingDesign::MatchedObservational => "matched_observational".into(),
        },
        result.pair_count.to_string(),
        result.has_declared_path.to_string(),
        result.expected_response_name().into(),
        format!("{:.6}", result.mean_paired_difference),
        format!("{:.6}", result.paired_difference_sd),
        format!("{:.6}", result.ci95_lower),
        format!("{:.6}", result.ci95_upper),
        result.status.name().into(),
        result.confirmatory_eligible.to_string(),
        result.control_gate_passed.to_string(),
    ]);
}

trait ExpectedResponseName {
    fn expected_response_name(&self) -> &'static str;
}

impl ExpectedResponseName for civdyn_causal_path::paired_effects::PairedEffectResult {
    fn expected_response_name(&self) -> &'static str {
        match self.expected_response {
            ExpectedResponse::Positive => "positive",
            ExpectedResponse::Negative => "negative",
            ExpectedResponse::Equivalent => "equivalent",
            ExpectedResponse::Exploratory => "exploratory",
        }
    }
}

fn main() {
    println!(
        "record_type,scenario,pairing_design,pair_count,has_declared_path,expected_response,mean_paired_difference,paired_difference_sd,ci95_lower,ci95_upper,status,confirmatory_eligible,control_gate_passed"
    );

    run_case(
        "positive_control",
        PairingDesign::CommonRandomNumbers,
        |index| 0.5 + ((index % 3) as f64 - 1.0) * 0.01,
        true,
        ExpectedResponse::Positive,
        0.05,
    );
    run_case(
        "equivalence_negative_control",
        PairingDesign::CommonRandomNumbers,
        |index| if index % 2 == 0 { 0.002 } else { -0.002 },
        true,
        ExpectedResponse::Equivalent,
        0.01,
    );
    run_case(
        "disconnected_outcome_movement",
        PairingDesign::CommonRandomNumbers,
        |_| 0.4,
        false,
        ExpectedResponse::Positive,
        0.05,
    );
    run_case(
        "matched_observational_only",
        PairingDesign::MatchedObservational,
        |_| 0.4,
        true,
        ExpectedResponse::Positive,
        0.05,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_causal_path::paired_effects::EffectStatus;

    #[test]
    fn positive_control_is_directionally_supported() {
        let r = evaluate_paired_contrast(
            &dataset(PairingDesign::CommonRandomNumbers, |i| {
                0.5 + (i % 2) as f64 * 0.01
            }),
            &ContrastContract {
                outcome_variable_id: "service".into(),
                expected_response: ExpectedResponse::Positive,
                equivalence_margin: 0.05,
            },
            true,
        )
        .unwrap();
        assert_eq!(
            r.status,
            EffectStatus::DirectionallyConsistentWithDeclaredPath
        );
        assert!(r.control_gate_passed);
    }

    #[test]
    fn matched_observational_design_never_passes_confirmatory_gate() {
        let r = evaluate_paired_contrast(
            &dataset(PairingDesign::MatchedObservational, |_| 0.5),
            &ContrastContract {
                outcome_variable_id: "service".into(),
                expected_response: ExpectedResponse::Positive,
                equivalence_margin: 0.05,
            },
            true,
        )
        .unwrap();
        assert_eq!(r.status, EffectStatus::DescriptiveOnly);
        assert!(!r.control_gate_passed);
    }
}
