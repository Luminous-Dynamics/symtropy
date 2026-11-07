// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic plural-panel / judge-calibration fixture.
//! No human-subject, platform or cultural-community data is imported.

use civdyn_evaluator_audit::{
    EvaluationCase, EvaluatorAuditReport, EvaluatorDataset, JudgePrediction, PanelRating, evaluate,
};

const CATEGORIES: [&str; 4] = ["category_a", "category_b", "category_c", "category_d"];
const CSV_COLUMNS: usize = 28;

fn blank_row() -> Vec<String> {
    vec![String::new(); CSV_COLUMNS]
}

fn emit(row: Vec<String>) {
    assert_eq!(
        row.len(),
        CSV_COLUMNS,
        "evaluator audit CSV schema must remain fixed"
    );
    println!("{}", row.join(","));
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn fixture_dataset() -> EvaluatorDataset {
    let mut cases = Vec::new();
    for index in 0..12 {
        let context_id = if index < 6 {
            "context_alpha"
        } else {
            "context_beta"
        };
        let cohort_id = if index % 2 == 0 {
            "community_a"
        } else {
            "community_b"
        };
        let panel_categories = if index < 4 {
            vec![0, 0, 0]
        } else if index < 8 {
            vec![1, 1, 2]
        } else {
            vec![3, 2, 3]
        };
        let mut target = vec![0.0; CATEGORIES.len()];
        for category in &panel_categories {
            target[*category] += 1.0 / panel_categories.len() as f64;
        }
        let selected = target
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1).then_with(|| right.0.cmp(&left.0)))
            .map_or(0, |(category, _)| category);
        let confidence = target[selected];
        let overconfident = (0..CATEGORIES.len())
            .map(|category| if category == selected { 0.97 } else { 0.01 })
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
                    probabilities: target,
                    reported_confidence: Some(confidence),
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
        category_labels: CATEGORIES.iter().map(|label| (*label).into()).collect(),
        cases,
    }
}

fn write_common(row: &mut [String], report: &EvaluatorAuditReport, record_type: &str) {
    row[0] = record_type.into();
    row[1] = report.rubric_id.clone();
    row[2] = report.rubric_version.clone();
}

fn emit_panel_overall(report: &EvaluatorAuditReport) {
    let metrics = &report.overall_panel;
    let mut row = blank_row();
    write_common(&mut row, report, "panel_overall");
    row[7] = metrics.case_count.to_string();
    row[8] = metrics.rating_count.to_string();
    row[9] = metrics.unanimous_case_count.to_string();
    row[10] = fmt(metrics.observed_pairwise_agreement);
    row[11] = metrics
        .krippendorff_alpha_nominal
        .map_or_else(String::new, fmt);
    row[12] = fmt(metrics.mean_disagreement_entropy_bits);
    emit(row);
}

fn emit_panel_slice(report: &EvaluatorAuditReport) {
    for slice in &report.panel_slices {
        let metrics = &slice.metrics;
        let mut row = blank_row();
        write_common(&mut row, report, "panel_slice");
        row[3] = slice.context_id.clone();
        row[4] = slice.cohort_id.clone();
        row[7] = metrics.case_count.to_string();
        row[8] = metrics.rating_count.to_string();
        row[9] = metrics.unanimous_case_count.to_string();
        row[10] = fmt(metrics.observed_pairwise_agreement);
        row[11] = metrics
            .krippendorff_alpha_nominal
            .map_or_else(String::new, fmt);
        row[12] = fmt(metrics.mean_disagreement_entropy_bits);
        emit(row);
    }
}

fn emit_case_panels(report: &EvaluatorAuditReport) {
    for case in &report.case_panels {
        let mut row = blank_row();
        write_common(&mut row, report, "case_panel");
        row[3] = case.context_id.clone();
        row[4] = case.cohort_id.clone();
        row[5] = case.case_id.clone();
        row[7] = "1".into();
        row[8] = case.rater_count.to_string();
        row[9] = if case.disagreement_entropy_bits == 0.0 {
            "1"
        } else {
            "0"
        }
        .into();
        row[12] = fmt(case.disagreement_entropy_bits);
        row[13] = case.judge_count.to_string();
        row[26] = CATEGORIES[case.plurality_category].into();
        row[27] = fmt(case.plurality_support);
        emit(row);
    }
}

fn emit_judge_overall(report: &EvaluatorAuditReport) {
    for metrics in &report.overall_judges {
        let mut row = blank_row();
        write_common(&mut row, report, "judge_overall");
        row[6] = metrics.judge_id.clone();
        row[13] = metrics.case_count.to_string();
        row[14] = fmt(metrics.mean_soft_panel_brier_score);
        row[15] = fmt(metrics.mean_soft_panel_cross_entropy_nats);
        row[16] = fmt(metrics.mean_plurality_match_rate);
        row[17] = fmt(metrics.mean_panel_support_for_selected);
        row[18] = metrics.confidence_case_count.to_string();
        row[19] = metrics
            .mean_reported_confidence
            .map_or_else(String::new, fmt);
        row[20] = metrics
            .mean_absolute_confidence_support_error
            .map_or_else(String::new, fmt);
        row[21] = metrics.confidence_ece.map_or_else(String::new, fmt);
        emit(row);
    }
}

fn emit_judge_slices(report: &EvaluatorAuditReport) {
    for slice in &report.judge_slices {
        let metrics = &slice.metrics;
        let mut row = blank_row();
        write_common(&mut row, report, "judge_slice");
        row[3] = slice.context_id.clone();
        row[4] = slice.cohort_id.clone();
        row[6] = metrics.judge_id.clone();
        row[13] = metrics.case_count.to_string();
        row[14] = fmt(metrics.mean_soft_panel_brier_score);
        row[15] = fmt(metrics.mean_soft_panel_cross_entropy_nats);
        row[16] = fmt(metrics.mean_plurality_match_rate);
        row[17] = fmt(metrics.mean_panel_support_for_selected);
        row[18] = metrics.confidence_case_count.to_string();
        row[19] = metrics.mean_reported_confidence.map_or_else(String::new, fmt);
        row[20] = metrics.mean_absolute_confidence_support_error.map_or_else(String::new, fmt);
        row[21] = metrics.confidence_ece.map_or_else(String::new, fmt);
        emit(row);
    }
}

fn emit_calibration_bins(report: &EvaluatorAuditReport) {
    for metrics in &report.overall_judges {
        for bin in &metrics.calibration_bins {
            let mut row = blank_row();
            write_common(&mut row, report, "calibration_bin");
            row[6] = metrics.judge_id.clone();
            row[22] = bin.bin_index.to_string();
            row[23] = bin.case_count.to_string();
            row[24] = fmt(bin.mean_reported_confidence);
            row[25] = fmt(bin.mean_panel_support);
            emit(row);
        }
    }
}

fn main() {
    println!(
        "record_type,rubric_id,rubric_version,context_id,cohort_id,case_id,judge_id,case_count,rating_count,unanimous_case_count,observed_pairwise_agreement,krippendorff_alpha_nominal,mean_disagreement_entropy_bits,judge_case_count,mean_soft_panel_brier_score,mean_soft_panel_cross_entropy_nats,mean_plurality_match_rate,mean_panel_support_for_selected,confidence_case_count,mean_reported_confidence,mean_absolute_confidence_support_error,confidence_ece,bin_index,bin_case_count,bin_mean_reported_confidence,bin_mean_panel_support,plurality_category,plurality_support"
    );

    let report =
        evaluate(&fixture_dataset()).expect("synthetic evaluator audit fixture must validate");
    emit_panel_overall(&report);
    emit_judge_overall(&report);
    emit_panel_slice(&report);
    emit_case_panels(&report);
    emit_judge_slices(&report);
    emit_calibration_bins(&report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_has_twelve_cases_and_two_judges() {
        let report = evaluate(&fixture_dataset()).unwrap();
        assert_eq!(report.overall_panel.case_count, 12);
        assert_eq!(report.overall_judges.len(), 2);
        assert_eq!(report.case_panels.len(), 12);
    }

    #[test]
    fn distribution_aware_judge_matches_the_soft_panel_target() {
        let report = evaluate(&fixture_dataset()).unwrap();
        let aware = report
            .overall_judges
            .iter()
            .find(|metrics| metrics.judge_id == "distribution_aware")
            .unwrap();
        assert!(aware.mean_soft_panel_brier_score < 1e-12);
        assert!(aware.confidence_ece.unwrap() < 1e-12);
    }

    #[test]
    fn overconfident_majority_keeps_plurality_accuracy_but_loses_calibration() {
        let report = evaluate(&fixture_dataset()).unwrap();
        let overconfident = report
            .overall_judges
            .iter()
            .find(|metrics| metrics.judge_id == "overconfident_majority")
            .unwrap();
        assert_eq!(overconfident.mean_plurality_match_rate, 1.0);
        assert!(overconfident.mean_soft_panel_brier_score > 0.1);
        assert!(overconfident.confidence_ece.unwrap() > 0.1);
    }

    #[test]
    fn evidence_rows_have_stable_column_width() {
        let row = blank_row();
        assert_eq!(row.len(), CSV_COLUMNS);
    }
}
