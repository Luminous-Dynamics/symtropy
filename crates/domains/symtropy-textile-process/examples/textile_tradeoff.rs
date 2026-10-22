// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Synthetic demonstration: hard gates are evaluated before Pareto ranking.
//!
//! Run with:
//! cargo run -p symtropy-textile-process --example textile_tradeoff

use std::error::Error;
use symtropy_textile_process::{
    CandidateEvaluation, Direction, GateCheck, GateOutcome, ObjectiveSpec, ObjectiveValue,
    compare_candidates,
};

fn objectives() -> Vec<ObjectiveSpec> {
    vec![
        ObjectiveSpec {
            id: "energy".into(),
            unit: "kWh/functional-unit".into(),
            scale: 1_000,
            direction: Direction::Minimize,
        },
        ObjectiveSpec {
            id: "throughput".into(),
            unit: "functional-units/hour".into(),
            scale: 100,
            direction: Direction::Maximize,
        },
        ObjectiveSpec {
            id: "water".into(),
            unit: "L/functional-unit".into(),
            scale: 1_000,
            direction: Direction::Minimize,
        },
    ]
}

fn candidate(
    id: &str,
    energy: i64,
    throughput: i64,
    water: i64,
    effluent: GateOutcome,
) -> CandidateEvaluation {
    let objective_specs = objectives();
    CandidateEvaluation {
        id: id.into(),
        gates: vec![
            GateCheck {
                id: "product_quality".into(),
                outcome: GateOutcome::Pass {
                    evidence_ref: format!("synthetic:{id}:quality"),
                },
            },
            GateCheck {
                id: "effluent_admission".into(),
                outcome: effluent,
            },
        ],
        values: objective_specs
            .iter()
            .zip([energy, throughput, water])
            .map(|(spec, value_ticks)| ObjectiveValue {
                objective_id: spec.id.clone(),
                value_ticks,
                unit: spec.unit.clone(),
                scale: spec.scale,
                evidence_ref: format!("synthetic:{id}:{}", spec.id),
            })
            .collect(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let report = compare_candidates(
        "synthetic-textile-scheduling-v0.1",
        &objectives(),
        &[
            candidate(
                "balanced",
                100_000,
                12_000,
                120_000,
                GateOutcome::Pass {
                    evidence_ref: "synthetic:balanced:effluent-pass".into(),
                },
            ),
            candidate(
                "fast",
                130_000,
                15_000,
                125_000,
                GateOutcome::Pass {
                    evidence_ref: "synthetic:fast:effluent-pass".into(),
                },
            ),
            candidate(
                "blocked-low-energy",
                50_000,
                20_000,
                60_000,
                GateOutcome::Fail {
                    evidence_ref: "synthetic:blocked:effluent-failure".into(),
                },
            ),
        ],
    )?;

    println!("comparison_id={}", report.comparison_id);
    println!("Pareto frontier (synthetic only):");
    for candidate_id in &report.frontier {
        println!("  {candidate_id}");
    }
    println!("Dominance witnesses:");
    for witness in &report.dominated {
        println!(
            "  {} dominates {}",
            witness.dominator_id, witness.dominated_id
        );
    }
    println!("Admission dispositions:");
    for disposition in &report.dispositions {
        println!("  {}: {:?}", disposition.candidate_id, disposition.disposition);
    }
    println!("No factory performance or environmental result is inferred from this fixture.");
    Ok(())
}
