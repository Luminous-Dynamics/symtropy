// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic configuration-surface decision sensitivity fixture.
//! No human, hiring, lending or demographic data is imported.

use civdyn_config_sensitivity::{
    ConfigurationBenchmark, ConfigurationFacets, ConfigurationIdentity, ConfigurationRun,
    CohortMetrics, ConfigurationMetrics, DecisionRecord, SingleFacetContrast, evaluate,
};

const CSV_COLUMNS: usize = 21;

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn facets(order: &str) -> ConfigurationFacets {
    ConfigurationFacets {
        persona_id: "balanced".into(),
        interview_protocol_id: "structured".into(),
        facilitator_id: "chair_a".into(),
        aggregation_rule_id: "majority".into(),
        order_profile_id: order.into(),
        prompt_template_id: "prompt_v1".into(),
        tool_policy_id: "no_external_tools".into(),
        evidence_snapshot_id: "snapshot_v1".into(),
        decoding_policy_id: "deterministic".into(),
    }
}

fn decisions(selected: &[usize]) -> Vec<DecisionRecord> {
    (0..12)
        .map(|index| DecisionRecord {
            case_id: format!("case_{index:02}"),
            cohort_id: if index < 6 { "cohort_a" } else { "cohort_b" }.into(),
            selected: selected.contains(&index),
        })
        .collect()
}

fn run(id: &str, digest_char: char, order: &str, selected: &[usize]) -> ConfigurationRun {
    ConfigurationRun {
        configuration: ConfigurationIdentity {
            configuration_id: id.into(),
            configuration_digest: digest(digest_char),
            model_id: "synthetic_reference_model".into(),
            model_version: "v1".into(),
            facets: facets(order),
        },
        decisions: decisions(selected),
    }
}

fn fixture() -> ConfigurationBenchmark {
    ConfigurationBenchmark {
        benchmark_id: "synthetic_configuration_surface".into(),
        baseline_configuration_id: "baseline".into(),
        reference_cohort_id: "cohort_a".into(),
        runs: vec![
            run("baseline", 'a', "natural_order", &[0, 1, 2, 3, 6, 7]),
            run(
                "semantic_permutation",
                'b',
                "permuted_stable_ids",
                &[0, 1, 2, 3, 6, 7],
            ),
            run(
                "position_biased_order",
                'c',
                "position_biased",
                &[0, 1, 6, 7, 8, 9],
            ),
        ],
    }
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn empty_row(record_type: &str, benchmark_id: &str) -> Vec<String> {
    let mut row = vec![String::new(); CSV_COLUMNS];
    row[0] = record_type.into();
    row[1] = benchmark_id.into();
    row
}

fn emit(row: Vec<String>) {
    assert_eq!(row.len(), CSV_COLUMNS, "configuration CSV schema must remain fixed");
    println!("{}", row.join(","));
}

fn summary_row(benchmark_id: &str, metrics: &ConfigurationMetrics) {
    let mut row = empty_row("configuration_summary", benchmark_id);
    row[2] = metrics.configuration.configuration_id.clone();
    row[3] = metrics.configuration.configuration_digest.clone();
    row[4] = metrics.configuration.model_version.clone();
    row[8] = metrics.overall_rate.selected.to_string();
    row[9] = metrics.overall_rate.total.to_string();
    row[10] = fmt(metrics.overall_rate.value);
    row[13] = metrics.flipped_cases_vs_baseline.to_string();
    row[14] = fmt(metrics.flip_rate_vs_baseline);
    emit(row);

    for cohort in &metrics.cohorts {
        cohort_row(benchmark_id, &metrics.configuration, cohort);
    }
}

fn cohort_row(
    benchmark_id: &str,
    configuration: &ConfigurationIdentity,
    cohort: &CohortMetrics,
) {
    let mut row = empty_row("cohort_summary", benchmark_id);
    row[2] = configuration.configuration_id.clone();
    row[3] = configuration.configuration_digest.clone();
    row[4] = configuration.model_version.clone();
    row[7] = cohort.cohort_id.clone();
    row[8] = cohort.rate.selected.to_string();
    row[9] = cohort.rate.total.to_string();
    row[10] = fmt(cohort.rate.value);
    row[11] = cohort.rate_ratio_to_reference.map_or_else(String::new, fmt);
    row[12] = fmt(cohort.rate_gap_to_reference);
    emit(row);
}

fn contrast_row(benchmark_id: &str, contrast: &SingleFacetContrast) {
    let mut row = empty_row("single_facet_contrast", benchmark_id);
    row[5] = contrast.facet.clone();
    row[6] = format!("{} -> {}", contrast.value_a, contrast.value_b);
    row[15] = contrast.configuration_a.clone();
    row[16] = contrast.configuration_b.clone();
    row[17] = contrast.flipped_cases.to_string();
    row[18] = contrast.case_count.to_string();
    row[19] = fmt(contrast.flip_rate);
    row[20] = fmt(contrast.overall_rate_delta_b_minus_a);
    emit(row);
}

fn surface_row(benchmark_id: &str, report: &civdyn_config_sensitivity::ConfigurationReport) {
    let mut row = empty_row("configuration_surface", benchmark_id);
    row[8] = report.surface.configuration_count.to_string();
    row[9] = report.surface.case_count_per_configuration.to_string();
    row[10] = fmt(report.surface.selection_rate_range);
    row[11] = fmt(report.surface.minimum_selection_rate);
    row[12] = fmt(report.surface.maximum_selection_rate);
    emit(row);
}

fn emit_report(report: civdyn_config_sensitivity::ConfigurationReport) {
    for metrics in &report.configuration_metrics {
        summary_row(&report.benchmark_id, metrics);
    }
    for contrast in &report.single_facet_contrasts {
        contrast_row(&report.benchmark_id, contrast);
    }
    surface_row(&report.benchmark_id, &report);
}

fn main() {
    println!(
        "record_type,benchmark_id,configuration_id,configuration_digest,model_version,facet,facet_value,cohort_id,selected,total,rate,ratio_to_reference,rate_gap_to_reference,flipped_vs_baseline,flip_rate_vs_baseline,configuration_a,configuration_b,pair_flipped_cases,pair_case_count,pair_flip_rate,overall_rate_delta_b_minus_a"
    );
    let report = evaluate(&fixture()).expect("synthetic configuration surface must validate");
    emit_report(report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_fixture_contains_three_complete_configuration_runs() {
        let report = evaluate(&fixture()).unwrap();
        assert_eq!(report.surface.configuration_count, 3);
        assert_eq!(report.surface.case_count_per_configuration, 12);
    }

    #[test]
    fn semantic_reorder_is_invariant_but_position_bias_is_detected() {
        let report = evaluate(&fixture()).unwrap();
        let semantic = report
            .configuration_metrics
            .iter()
            .find(|metrics| metrics.configuration.configuration_id == "semantic_permutation")
            .unwrap();
        let biased = report
            .configuration_metrics
            .iter()
            .find(|metrics| metrics.configuration.configuration_id == "position_biased_order")
            .unwrap();
        assert_eq!(semantic.flipped_cases_vs_baseline, 0);
        assert_eq!(biased.flipped_cases_vs_baseline, 4);
        assert!((biased.flip_rate_vs_baseline - (1.0 / 3.0)).abs() < 1e-12);
    }
}
