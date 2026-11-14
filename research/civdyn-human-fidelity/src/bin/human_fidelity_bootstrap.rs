// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic participant-cluster bootstrap coverage curve.
//! No human or platform traces are imported.

use civdyn_human_fidelity::{
    BootstrapConfig, BootstrapMetric, HumanFidelityDataset, ObservedTrial, SimulatedTrial,
    bootstrap_population_coverage_curve,
};

const OPTIONS: [&str; 4] = ["option_a", "option_b", "option_c", "option_d"];
const COVERAGE_SIZES: [usize; 5] = [10, 20, 30, 50, 60];
const REPLICATES: usize = 200;
const SEED: u64 = 0x4855_4D53_494D_3032;
const CSV_COLUMNS: usize = 13;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(String::new, fmt)
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "bootstrap CSV schema must remain fixed"
    );
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
                    if context_draw < 6 {
                        2
                    } else if context_draw < 10 {
                        3
                    } else if context_draw < 30 {
                        1
                    } else {
                        0
                    }
                }
                1 => {
                    if context_draw < 6 {
                        2
                    } else if context_draw < 10 {
                        3
                    } else if context_draw < 30 {
                        0
                    } else {
                        1
                    }
                }
                _ => {
                    if context_draw < 6 {
                        2
                    } else if context_draw < 10 {
                        1
                    } else if context_draw < 30 {
                        0
                    } else {
                        3
                    }
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

fn main() {
    println!(
        "record_type,policy,scope,participant_count,replicate_count,mean_trial_count_per_replicate,metric,valid_replicates,undefined_replicates,mean,p2_5,p50,p97_5"
    );

    for (policy, mode_collapse) in [
        ("direct_prompt_mode_collapse", true),
        ("structured_context_reference", false),
    ] {
        let dataset = synthetic_dataset(mode_collapse);
        let points = bootstrap_population_coverage_curve(
            &dataset,
            &COVERAGE_SIZES,
            BootstrapConfig {
                seed: SEED,
                replicate_count: REPLICATES,
            },
        )
        .expect("synthetic participant-cluster bootstrap must validate");

        for point in points {
            for metric in point.metrics {
                emit(vec![
                    "bootstrap_summary".into(),
                    policy.into(),
                    "participant_cluster".into(),
                    point.participant_count.to_string(),
                    point.replicate_count.to_string(),
                    fmt(point.mean_trial_count_per_replicate),
                    metric.metric.name().into(),
                    metric.valid_replicates.to_string(),
                    metric.undefined_replicates.to_string(),
                    optional(metric.mean),
                    optional(metric.p2_5),
                    optional(metric.p50),
                    optional(metric.p97_5),
                ]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_synthetic_behaviour_policies_are_distinct_and_well_formed() {
        let collapsed = synthetic_dataset(true);
        let structured = synthetic_dataset(false);
        assert_eq!(collapsed.observed.len(), 180);
        assert_eq!(structured.simulated.len(), 180);
        assert_ne!(collapsed.simulated, structured.simulated);
    }

    #[test]
    fn bootstrap_coverage_design_has_five_sizes_and_eight_metrics() {
        assert_eq!(COVERAGE_SIZES.len(), 5);
        assert_eq!(BootstrapMetric::ALL.len(), 8);
        assert_eq!(COVERAGE_SIZES[4], 60);
    }
}
