// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic plural-panel and judge-calibration fixture.
//! No human, video, audio, demographic or cultural-community data is imported.

use civdyn_evaluator_audit::{
    CalibrationBin, EvaluationCase, EvaluatorAuditReport, EvaluatorDataset, JudgeMetrics,
    JudgePrediction, PanelMetrics, PanelRating, evaluate,
};

const CSV_COLUMNS: usize = 28;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn fmt_option(value: Option<f64>) -> String {
    value.map_or_else(String::new, fmt)
}

fn empty_row(record_type: &str, scope: &str) -> Vec<String> {
    let mut row = vec![String::new(); CSV_COLUMNS];
    row[0] = record_type.into();
    row[1] = scope.into();
    row
}

fn emit(row: Vec<String>) {
    assert_eq!(
        row.len(),
        CSV_COLUMNS,
        "evaluator-audit CSV schema must remain fixed"
    );
    println!("{}", row.join(","));
}

fn panel_row(record_type: &str, scope: &str, metrics: &PanelMetrics) {
    let mut row = empty_row(record_type, scope);
    row[6] = metrics.case_count.to_string();
    row[7] = metrics.rating_count.to_string();
    row[8] = metrics.pair_count.to_string();
    row[9] = fmt(metrics.mean_raters_per_case);
    row[10] = metrics.minimum_raters_per_case.to_string();
    row[11] = metrics.maximum_raters_per_case.to_string();
    row[12] = fmt(metrics.observed_pairwise_agreement);
    row[13] = fmt_option(metrics.krippendorff_alpha_nominal);
    row[14] = fmt(metrics.mean_disagreement_entropy_bits);
    row[15] = metrics.unanimous_case_count.to_string();
    emit(row);
}

fn judge_row(record_type: &str, scope: &str, metrics: &JudgeMetrics) {
    let mut row = empty_row(record_type, scope);
    row[3] = metrics.judge_id.clone();
    row[6] = metrics.case_count.to_string();
    row[16] = fmt(metrics.mean_soft_panel_brier_score);
    row[17] = fmt(metrics.mean_soft_panel_cross_entropy_nats);
    row[18] = fmt(metrics.mean_plurality_match_rate);
    row[19] = fmt(metrics.mean_panel_support_for_selected);
    row[20] = metrics.confidence_case_count.to_string();
    row[21] = fmt_option(metrics.mean_reported_confidence);
    row[22] = fmt_option(metrics.mean_absolute_confidence_support_error);
    row[23] = fmt_option(metrics.confidence_ece);
    emit(row);
}

fn calibration_row(judge_id: &str, scope: &str, bin: &CalibrationBin) {
    let mut row = empty_row("calibration_bin", scope);
    row[3] = judge_id.into();
    row[24] = bin.bin_index.to_string();
    row[25] = bin.case_count.to_string();
    row[26] = fmt(bin.mean_reported_confidence);
    row[27] = fmt(bin.mean_panel_support);
    emit(row);
}

fn fixture_dataset() -> EvaluatorDataset {
    let mut cases = Vec::new();
    for index in 0..12 {
        let context_id = if index < 6 { "context_alpha" } else { "context_beta" };
        let cohort_id = if index % 2 == 0 { "community_a" } else { "community_b" };
        let panel_categories = if index < 4 {
            vec![0, 0, 0]
        } else if index < 8 {
            vec![1, 1, 2]
        } else {
            vec![3, 2, 3]
        };

        let mut counts = vec![0.0_f64; 4];
        for category in &panel_categories {
            counts[*category] += 1.0 / panel_categories.len() as f64;
        }
        let selected = counts
            .iter()
            .enumerate()
            .max_by(|left, right| {
                left.1.total_cmp(right.1).then_with(|| right.0.cmp(&left.0))
            })
            .map_or(0, |(category, _)| category);
        let support = counts[selected];
        let overconfident = (0..4)
            .map(|category| {
                if category == selected {
                    0.97
                } else {
                    0.01
                }
            })
            .collect::<Vec<_>>();

        cases.push(EvaluationCase {
            case_id: format!("case_{index:02}"),
            context_id: context_id.into(),
            cohort_id: cohort_id.into(),
            ratings: panel_categories
                .iter()
                .enumerate()
                .map(|(rater_index, category_index)| PanelRating {
                    rater_id: format!("panel_rater_{rater_index}"),
                    category_index: *category_index,
                    reported_confidence: None,
                })
                .collect(),
            judge_predictions: vec![
                JudgePrediction {
                    judge_id: "distribution_aware".into(),
                    selected_category: selected,
                    probabilities: counts,
                    reported_confidence: Some(support),
                },
                JudgePrediction {
                    judge_id: "overconfident_majority".into(),
                    selected_category: selected,
                    probabilities: overconfident,
                    reported_confidence: Some(0.99),
                },
            ],
        });
    }

    EvaluatorDataset {
        rubric_id: "synthetic_norm_rubric".into(),
        rubric_version: "0.1.0".into(),
        category_labels: vec![
            "category_a".into(),
            "category_b".into(),
            "category_c".into(),
            "category_d".into(),
        ],
        cases,
    }
}

fn emit_report(report: EvaluatorAuditReport) {
    panel_row("panel_overall", "all", &report.overall_panel);
    for slice in &report.panel_slices {
        panel_row(
            "panel_slice",
            &format!("{}/{}", slice.context_id, slice.cohort_id),
            &slice.metrics,
        );
    }

    for case in &report.case_panels {
        let mut row = empty_row(
            "case_panel",
            &format!("{}/{}", case.context_id, case.cohort_id),
        );
        row[2] = case.case_id.clone();
        row[4] = case.context_id.clone();
        row[5] = case.cohort_id.clone();
        row[6] = "1".into();
        row[7] = case.rater_count.to_string();
        row[9] = case.rater_count.to_string();
        row[10] = case.rater_count.to_string();
        row[11] = case.rater_count.to_string();
        row[14] = fmt(case.disagreement_entropy_bits);
        row[15] = case.plurality_category.to_string();
        row[18] = fmt(case.plurality_support);
        row[19] = case.judge_count.to_string();
        emit(row);
    }

    for metrics in &report.overall_judges {
        judge_row("judge_overall", "all", metrics);
        for bin in &metrics.calibration_bins {
            calibration_row(&metrics.judge_id, "all", bin);
        }
    }
    for slice in &report.judge_slices {
        judge_row(
            "judge_slice",
            &format!("{}/{}", slice.context_id, slice.cohort_id),
            &slice.metrics,
        );
    }
}

fn main() {
    println!(
        "record_type,scope,case_id,judge_id,context_id,cohort_id,case_count,rating_count,pair_count,mean_raters_per_case,min_raters_per_case,max_raters_per_case,observed_pairwise_agreement,krippendorff_alpha_nominal,mean_disagreement_entropy_bits,unanimous_case_count,mean_soft_panel_brier_score,mean_soft_panel_cross_entropy_nats,mean_plurality_match_rate,mean_panel_support_for_selected,confidence_case_count,mean_reported_confidence,mean_absolute_confidence_support_error,confidence_ece,calibration_bin,calibration_bin_case_count,bin_mean_reported_confidence,bin_mean_panel_support"
    );
    let report = evaluate(&fixture_dataset()).expect("synthetic evaluator audit must validate");
    emit_report(report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_fixture_has_the_declared_panel_and_context_support() {
        let report = evaluate(&fixture_dataset()).unwrap();
        assert_eq!(report.overall_panel.case_count, 12);
        assert_eq!(report.overall_panel.rating_count, 36);
        assert_eq!(report.overall_panel.unanimous_case_count, 4);
        assert_eq!(report.panel_slices.len(), 4);
        assert_eq!(report.case_panels.len(), 12);
    }

    #[test]
    fn judge_probability_quality_and_reported_confidence_are_distinct() {
        let report = evaluate(&fixture_dataset()).unwrap();
        let aware = report
            .overall_judges
            .iter()
            .find(|metrics| metrics.judge_id == "distribution_aware")
            .unwrap();
        let overconfident = report
            .overall_judges
            .iter()
            .find(|metrics| metrics.judge_id == "overconfident_majority")
            .unwrap();
        assert!(aware.mean_soft_panel_brier_score < 1e-12);
        assert!(aware.confidence_ece.unwrap() < 1e-12);
        assert!(overconfident.mean_soft_panel_brier_score > aware.mean_soft_panel_brier_score);
        assert!(overconfident.confidence_ece.unwrap() > 0.1);
        assert_eq!(aware.mean_plurality_match_rate, 1.0);
        assert_eq!(overconfident.mean_plurality_match_rate, 1.0);
    }
}
