// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic software fixture for multi-run operational pattern comparison.
//! Every value is artificial; this binary does not import empirical data.

use civdyn_operational_validity::{
    MetricSpec, NumericSample, SampleSet, compare_metric,
};

const REFERENCE_SAMPLE_COUNT: usize = 30;

fn samples(prefix: &str, values: &[f64], unit: &str, independent_unit: &str) -> SampleSet {
    SampleSet {
        unit: unit.to_owned(),
        independent_unit: independent_unit.to_owned(),
        samples: values
            .iter()
            .enumerate()
            .map(|(index, value)| NumericSample {
                sample_id: format!("{prefix}_{index:03}"),
                value: *value,
            })
            .collect(),
    }
}

fn fixture_values(base: f64, step: f64) -> Vec<f64> {
    (0..REFERENCE_SAMPLE_COUNT)
        .map(|index| base + (index % 7) as f64 * step)
        .collect()
}

fn emit_row(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        19,
        "each comparison record must have 19 columns"
    );
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "run_kind,metric_id,stratum_id,unit,n_observed,n_simulated,observed_mean,simulated_mean,mean_delta,observed_mean_99_half_width,simulated_mean_99_half_width,observed_median,simulated_median,p10_delta,p50_delta,p90_delta,p99_delta,ks_distance,wasserstein_1"
    );

    let fixtures = [
        ("daily_active_users", "all", "users/day", 30.0, 31.0, 1.0),
        (
            "comments_per_thread",
            "all",
            "comments/thread",
            1.0,
            1.3,
            0.1,
        ),
        ("network_core_share", "all", "fraction", 0.05, 0.19, 0.005),
        (
            "toxicity",
            "root_posts",
            "classifier_score",
            0.20,
            0.33,
            0.01,
        ),
        ("toxicity", "comments", "classifier_score", 0.35, 0.26, 0.01),
    ];

    for (metric_id, stratum_id, unit, observed_base, simulated_base, step) in fixtures {
        let observed_values = fixture_values(observed_base, step);
        let simulated_values = fixture_values(simulated_base, step);
        let comparison = compare_metric(
            MetricSpec {
                metric_id: metric_id.to_owned(),
                unit: unit.to_owned(),
                stratum_id: stratum_id.to_owned(),
                observed_independent_unit: "matched_non_overlapping_empirical_window".into(),
                simulated_independent_unit: "independent_simulation_run".into(),
                minimum_observed_samples: REFERENCE_SAMPLE_COUNT,
                minimum_simulated_samples: REFERENCE_SAMPLE_COUNT,
                dependence_assessment: "Empirical windows do not overlap; each simulation run uses a distinct independent seed.".into(),
                sample_size_rationale: "This synthetic fixture follows the 30-window/30-run replicated forum study design; another dataset must specify its own dependence and power rationale.".into(),
            },
            &samples("observed_window", &observed_values, unit, "matched_non_overlapping_empirical_window"),
            &samples("simulation_run", &simulated_values, unit, "independent_simulation_run"),
        )
        .expect("synthetic sample sets satisfy the reference contract");

        emit_row(vec![
            "synthetic_benchmark_fixture".into(),
            comparison.metric.metric_id,
            comparison.metric.stratum_id,
            comparison.metric.unit,
            comparison.observed.sample_count.to_string(),
            comparison.simulated.sample_count.to_string(),
            format!("{:.6}", comparison.observed.mean),
            format!("{:.6}", comparison.simulated.mean),
            format!("{:.6}", comparison.mean_delta_simulated_minus_observed),
            format!("{:.6}", comparison.observed.mean_99_ci_half_width),
            format!("{:.6}", comparison.simulated.mean_99_ci_half_width),
            format!("{:.6}", comparison.observed.median),
            format!("{:.6}", comparison.simulated.median),
            format!("{:.6}", comparison.p10_delta_simulated_minus_observed),
            format!("{:.6}", comparison.median_delta_simulated_minus_observed),
            format!("{:.6}", comparison.p90_delta_simulated_minus_observed),
            format!("{:.6}", comparison.p99_delta_simulated_minus_observed),
            format!("{:.6}", comparison.ks_distance),
            format!("{:.6}", comparison.wasserstein_1),
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fixture_dimensions_use_thirty_independent_samples() {
        for base in [0.0, 1.0, 10.0] {
            assert_eq!(fixture_values(base, 0.1).len(), 30);
        }
    }

    #[test]
    fn toxicity_is_not_pooled_across_root_and_comment_strata() {
        let root = fixture_values(0.20, 0.01);
        let comments = fixture_values(0.35, 0.01);
        let sim_root = fixture_values(0.33, 0.01);
        let sim_comments = fixture_values(0.26, 0.01);
        let root_result = compare_metric(
            MetricSpec {
                metric_id: "toxicity".into(),
                unit: "classifier_score".into(),
                stratum_id: "root_posts".into(),
                observed_independent_unit: "matched_non_overlapping_empirical_window".into(),
                simulated_independent_unit: "independent_simulation_run".into(),
                minimum_observed_samples: REFERENCE_SAMPLE_COUNT,
                minimum_simulated_samples: REFERENCE_SAMPLE_COUNT,
                dependence_assessment: "Empirical windows do not overlap; each simulation run uses a distinct independent seed.".into(),
                sample_size_rationale: "This synthetic fixture follows the 30-window/30-run replicated forum study design; another dataset must specify its own dependence and power rationale.".into(),
            },
            &samples("root_obs", &root, "classifier_score", "matched_non_overlapping_empirical_window"),
            &samples("root_sim", &sim_root, "classifier_score", "independent_simulation_run"),
        )
        .unwrap();
        let comment_result = compare_metric(
            MetricSpec {
                metric_id: "toxicity".into(),
                unit: "classifier_score".into(),
                stratum_id: "comments".into(),
                observed_independent_unit: "matched_non_overlapping_empirical_window".into(),
                simulated_independent_unit: "independent_simulation_run".into(),
                minimum_observed_samples: REFERENCE_SAMPLE_COUNT,
                minimum_simulated_samples: REFERENCE_SAMPLE_COUNT,
                dependence_assessment: "Empirical windows do not overlap; each simulation run uses a distinct independent seed.".into(),
                sample_size_rationale: "This synthetic fixture follows the 30-window/30-run replicated forum study design; another dataset must specify its own dependence and power rationale.".into(),
            },
            &samples("comment_obs", &comments, "classifier_score", "matched_non_overlapping_empirical_window"),
            &samples("comment_sim", &sim_comments, "classifier_score", "independent_simulation_run"),
        )
        .unwrap();

        assert!(root_result.mean_delta_simulated_minus_observed > 0.0);
        assert!(comment_result.mean_delta_simulated_minus_observed < 0.0);
        assert_ne!(
            root_result.metric.stratum_id,
            comment_result.metric.stratum_id
        );
    }
}
