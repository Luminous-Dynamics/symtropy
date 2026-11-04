// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic micro/population/trajectory fidelity demonstration.
//! No human or platform traces are imported by this executable.

use civdyn_human_fidelity::{
    CoveragePoint, EvaluationResult, HumanFidelityDataset, ObservedTrial, SimulatedTrial,
    evaluate, population_coverage_curve,
};

const OPTIONS: [&str; 4] = ["option_a", "option_b", "option_c", "option_d"];
const COVERAGE_SIZES: [usize; 5] = [10, 20, 30, 50, 60];
const CSV_COLUMNS: usize = 30;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "human-fidelity CSV schema must stay fixed");
    println!("{}", fields.join(","));
}

fn synthetic_dataset(mode_collapse: bool) -> HumanFidelityDataset {
    let mut observed = Vec::new();
    let mut simulated = Vec::new();

    for participant in 0..60 {
        let participant_id = format!("participant_{participant:03}");
        let cohort_id = format!("declared_cohort_{}", participant % 3);
        for step_index in 0..3_u32 {
            let context_draw = ((participant * 7 + step_index as usize * 11) % 100) as u32;
            let choice = match participant % 3 {
                0 => {
                    if context_draw < 6 { 2 }
                    else if context_draw < 10 { 3 }
                    else if context_draw < 30 { 1 }
                    else { 0 }
                }
                1 => {
                    if context_draw < 6 { 2 }
                    else if context_draw < 10 { 3 }
                    else if context_draw < 30 { 0 }
                    else { 1 }
                }
                _ => {
                    if context_draw < 6 { 2 }
                    else if context_draw < 10 { 1 }
                    else if context_draw < 30 { 0 }
                    else { 3 }
                }
            };
            let context_id = format!("scenario_{step_index}");
            observed.push(ObservedTrial {
                participant_id: participant_id.clone(),
                context_id: context_id.clone(),
                cohort_id: cohort_id.clone(),
                step_index,
                choice,
            });

            let mut probabilities = if mode_collapse {
                vec![0.82, 0.10, 0.06, 0.02]
            } else {
                vec![0.1; OPTIONS.len()]
            };
            if !mode_collapse {
                probabilities[choice] = 0.7;
            }
            simulated.push(SimulatedTrial {
                participant_id: participant_id.clone(),
                context_id,
                cohort_id: cohort_id.clone(),
                step_index,
                choice_probabilities: probabilities,
                generated_choice: if mode_collapse { 0 } else { choice },
            });
        }
    }

    HumanFidelityDataset {
        option_labels: OPTIONS.iter().map(|label| (*label).into()).collect(),
        observed,
        simulated,
    }
}

fn metric_row(
    record_type: &str,
    policy: &str,
    scope: &str,
    metrics: &civdyn_human_fidelity::SliceMetrics,
    trajectory_tvd: Option<f64>,
    transition_count: usize,
) {
    let mut row = vec![
        record_type.into(),
        policy.into(),
        scope.into(),
        metrics.participant_count.to_string(),
        metrics.trial_count.to_string(),
        fmt(metrics.mean_brier_score),
        fmt(metrics.mean_log_loss),
        fmt(metrics.top_choice_accuracy),
    ];
    row.extend(metrics.observed_choice_frequency.iter().map(|value| fmt(*value)));
    row.extend(metrics.generated_choice_frequency.iter().map(|value| fmt(*value)));
    row.extend(metrics.mean_predicted_probability.iter().map(|value| fmt(*value)));
    row.extend([
        fmt(metrics.total_variation),
        fmt(metrics.jensen_shannon_bits),
        fmt(metrics.observed_entropy_bits),
        fmt(metrics.generated_entropy_bits),
        fmt(metrics.entropy_delta_bits),
        metrics.observed_tail_option_count.to_string(),
        metrics.generated_tail_option_count.to_string(),
        metrics.tail_option_coverage.map_or_else(String::new, fmt),
        trajectory_tvd.map_or_else(String::new, fmt),
        transition_count.to_string(),
    ]);
    emit(row);
}

fn coverage_row(policy: &str, point: CoveragePoint) {
    emit(vec![
        "coverage_curve".into(),
        policy.into(),
        "deterministic_prefix".into(),
        point.participant_count.to_string(),
        point.trial_count.to_string(),
        fmt(point.mean_brier_score),
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
        fmt(point.total_variation),
        fmt(point.jensen_shannon_bits),
        String::new(),
        String::new(),
        fmt(point.entropy_delta_bits),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]);
}

fn emit_evaluation(policy: &str, result: EvaluationResult) {
    metric_row(
        "overall",
        policy,
        "all_contexts_all_cohorts",
        &result.overall,
        result.trajectory_transition_tvd,
        result.transition_count,
    );
    for slice in &result.slices {
        metric_row(
            "slice",
            policy,
            &format!("{}/{}", slice.context_id, slice.cohort_id),
            slice,
            None,
            0,
        );
    }
}

fn main() {
    println!(
        "record_type,policy,scope,participant_count,trial_count,mean_brier_score,mean_log_loss,top_choice_accuracy,observed_frequency_0,observed_frequency_1,observed_frequency_2,observed_frequency_3,generated_frequency_0,generated_frequency_1,generated_frequency_2,generated_frequency_3,mean_predicted_probability_0,mean_predicted_probability_1,mean_predicted_probability_2,mean_predicted_probability_3,total_variation,jensen_shannon_bits,observed_entropy_bits,generated_entropy_bits,entropy_delta_bits,observed_tail_option_count,generated_tail_option_count,tail_option_coverage,trajectory_transition_tvd,transition_count"
    );

    for (policy, mode_collapse) in [
        ("direct_prompt_mode_collapse", true),
        ("structured_context_reference", false),
    ] {
        let dataset = synthetic_dataset(mode_collapse);
        let result = evaluate(&dataset).expect("synthetic human-fidelity task must validate");
        let coverage = population_coverage_curve(&dataset, &COVERAGE_SIZES)
            .expect("synthetic coverage curve must validate");
        for point in coverage {
            coverage_row(policy, point);
        }
        emit_evaluation(policy, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_fixture_contains_sixty_opaque_participants() {
        let dataset = synthetic_dataset(false);
        let result = evaluate(&dataset).unwrap();
        assert_eq!(result.overall.participant_count, 60);
        assert_eq!(result.overall.trial_count, 180);
        assert_eq!(result.slices.len(), 9);
    }

    #[test]
    fn structured_context_control_outperforms_mode_collapsed_control_on_both_layers() {
        let collapsed = evaluate(&synthetic_dataset(true)).unwrap();
        let structured = evaluate(&synthetic_dataset(false)).unwrap();
        assert!(structured.overall.mean_brier_score < collapsed.overall.mean_brier_score);
        assert!(structured.overall.total_variation < collapsed.overall.total_variation);
        assert!(structured.overall.jensen_shannon_bits < collapsed.overall.jensen_shannon_bits);
        assert!(structured.trajectory_transition_tvd.unwrap() < collapsed.trajectory_transition_tvd.unwrap());
    }

    #[test]
    fn coverage_curve_samples_the_declared_population_sizes() {
        let points = population_coverage_curve(&synthetic_dataset(true), &COVERAGE_SIZES).unwrap();
        assert_eq!(
            points.iter().map(|point| point.participant_count).collect::<Vec<_>>(),
            COVERAGE_SIZES
        );
    }
}
