// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Parameter-ensemble and Morris sensitivity sweep over synthetic resilience scenarios.

use civdyn_uncertainty_resilience::{
    DEFAULT_ENSEMBLE_SIZE, DEFAULT_MORRIS_TRAJECTORIES, DESIGN_SEED, evaluate_morris,
    latin_hypercube, morris_design, reference_scenarios, run_latin_hypercube_ensemble,
    summarize_ensemble,
};

const CSV_COLUMNS: usize = 17;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn empty_row(record_type: &str, design_id: &str, seed: u64) -> Vec<String> {
    vec![
        record_type.into(),
        design_id.into(),
        seed.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "uncertainty CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "record_type,design_id,seed,shock,policy,metric,factor,sample_count,p05,p50,p95,mean,worst10_mean,service_floor_breach_fraction,recovery_success_fraction,morris_mu_star,morris_sigma"
    );

    let scenarios = reference_scenarios();
    let lhs = latin_hypercube(DEFAULT_ENSEMBLE_SIZE, DESIGN_SEED)
        .expect("declared Latin-hypercube design must validate");
    let lhs_outcomes = run_latin_hypercube_ensemble(&scenarios, DEFAULT_ENSEMBLE_SIZE, DESIGN_SEED)
        .expect("Latin-hypercube resilience portfolio must execute");
    let ensemble = summarize_ensemble(&lhs_outcomes)
        .expect("each shock/policy combination must have ensemble outcomes");

    for summary in ensemble {
        let mut row = empty_row("parameter_ensemble", "lhs_independent_uniform_v0", lhs.seed);
        row[3] = summary.shock.name().into();
        row[4] = summary.policy.name().into();
        row[5] = summary.metric.name().into();
        row[7] = summary.sample_count.to_string();
        row[8] = fmt(summary.p05);
        row[9] = fmt(summary.p50);
        row[10] = fmt(summary.p95);
        row[11] = fmt(summary.mean);
        row[12] = fmt(summary.worst10_mean);
        row[13] = fmt(summary.service_floor_breach_fraction);
        row[14] = fmt(summary.recovery_success_fraction);
        emit(row);
    }

    let trajectories = morris_design(DEFAULT_MORRIS_TRAJECTORIES, DESIGN_SEED)
        .expect("frozen Morris design must validate");
    let effects = evaluate_morris(&scenarios, &trajectories)
        .expect("Morris trajectories must execute over the shock/policy portfolio");

    for summary in effects {
        let mut row = empty_row("morris_effect", "morris_4level_delta_2of3_v0", DESIGN_SEED);
        row[3] = summary.shock.name().into();
        row[4] = summary.policy.name().into();
        row[5] = summary.metric.name().into();
        row[6] = summary.factor.name().into();
        row[7] = summary.trajectory_count.to_string();
        row[15] = fmt(summary.mu_star);
        row[16] = fmt(summary.sigma);
        emit(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_portfolio_has_frozen_ensemble_and_morris_row_counts() {
        let scenarios = reference_scenarios();
        let lhs = latin_hypercube(DEFAULT_ENSEMBLE_SIZE, DESIGN_SEED).unwrap();
        assert_eq!(lhs.points.len(), 30);
        let outcomes =
            run_latin_hypercube_ensemble(&scenarios, DEFAULT_ENSEMBLE_SIZE, DESIGN_SEED).unwrap();
        assert_eq!(outcomes.len(), 30 * 3 * 4);
        assert_eq!(summarize_ensemble(&outcomes).unwrap().len(), 3 * 4 * 4);

        let trajectories = morris_design(DEFAULT_MORRIS_TRAJECTORIES, DESIGN_SEED).unwrap();
        let effects = evaluate_morris(&scenarios, &trajectories).unwrap();
        assert_eq!(effects.len(), 3 * 4 * 7 * 4);
        assert!(effects.iter().all(|row| row.trajectory_count == 30));
    }

    #[test]
    fn csv_schema_is_fixed() {
        assert_eq!(CSV_COLUMNS, 17);
    }
}
