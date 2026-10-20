// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic synthetic pilot for active visual-action selection.
//!
//! This benchmark deliberately has two separate data paths:
//!   1. policy proposal receives only PublicObservation and trial id;
//!   2. evaluator outcomes are assembled after the action is committed.
//!
//! The labels are authored synthetic task losses, not data from a real camera.
//! Resource values are abstract simulator units, not measured latency or energy.
//!
//! Standalone run (no workspace membership or third-party dependencies):
//! rustc --edition=2024 tools/vision-active-action-benchmark.rs -o /tmp/vis-act-bench
//! /tmp/vis-act-bench
//!
//! Standalone tests (also runs tests in the included evaluator reference):
//! rustc --edition=2024 --test tools/vision-active-action-benchmark.rs -o /tmp/vis-act-tests
//! /tmp/vis-act-tests

#[path = "vision-active-action-reference.rs"]
mod evaluator;

use evaluator::{
    CandidateOutcomeV1, EvaluationError, ResourceAxisProfileV1, ResourceProfileV1, RunSummaryV1,
    TrialV1, VisionAction, summarize_run_v1,
};
use std::collections::BTreeMap;

const ACTIONS: [VisionAction; 6] = [
    VisionAction::KeepCheap,
    VisionAction::EscalateCurrent,
    VisionAction::AcquireNext,
    VisionAction::ChangeView,
    VisionAction::WaitForEvent,
    VisionAction::AbstainOrHandoff,
];

const LATENCY_RUN_BUDGET: u128 = 50;
const SENSOR_RUN_BUDGET: u128 = 4;
const TRIAL_LATENCY_BUDGET: u64 = 10;
const TRIAL_SENSOR_BUDGET: u64 = 1;

#[derive(Debug, Clone, Copy)]
struct PublicObservation {
    /// Fixed-point score in thousandths. These values are observable features,
    /// not scenario labels or evaluator-truth encodings.
    confidence_milli: u16,
    surprise_milli: u16,
    occlusion_milli: u16,
    blur_milli: u16,
    novelty_milli: u16,
    compute_available: bool,
    next_sample_available: bool,
    alternate_view_available: bool,
    timed_event_available: bool,
}

impl PublicObservation {
    fn action_available(self, action: VisionAction) -> bool {
        match action {
            VisionAction::KeepCheap | VisionAction::AbstainOrHandoff => true,
            VisionAction::EscalateCurrent => self.compute_available,
            VisionAction::AcquireNext => self.next_sample_available,
            VisionAction::ChangeView => self.alternate_view_available,
            VisionAction::WaitForEvent => self.timed_event_available,
        }
    }
}

/// Candidate losses live on the evaluator side. The policy function cannot
/// access this struct; it receives only the PublicObservation above.
#[derive(Debug, Clone)]
struct SyntheticScenario {
    id: u64,
    observation: PublicObservation,
    evaluator_losses: BTreeMap<VisionAction, f64>,
}

fn costs(action: VisionAction) -> BTreeMap<String, u64> {
    let (latency_units, sensor_units) = match action {
        VisionAction::KeepCheap => (2, 0),
        VisionAction::EscalateCurrent => (8, 0),
        VisionAction::AcquireNext => (4, 1),
        VisionAction::ChangeView => (5, 1),
        VisionAction::WaitForEvent => (6, 1),
        VisionAction::AbstainOrHandoff => (1, 0),
    };
    BTreeMap::from([
        ("latency_unit".to_string(), latency_units),
        ("sensor_action".to_string(), sensor_units),
    ])
}

fn resource_profile() -> ResourceProfileV1 {
    BTreeMap::from([
        (
            "latency_unit".to_string(),
            ResourceAxisProfileV1 {
                unit_id: "synthetic_latency_unit".to_string(),
                trial_budget: Some(TRIAL_LATENCY_BUDGET),
                run_budget: Some(LATENCY_RUN_BUDGET),
            },
        ),
        (
            "sensor_action".to_string(),
            ResourceAxisProfileV1 {
                unit_id: "synthetic_sensor_action".to_string(),
                trial_budget: Some(TRIAL_SENSOR_BUDGET),
                run_budget: Some(SENSOR_RUN_BUDGET),
            },
        ),
    ])
}

fn make_scenario(
    id: u64,
    observation: PublicObservation,
    losses: [f64; 6],
) -> SyntheticScenario {
    let evaluator_losses = ACTIONS
        .into_iter()
        .zip(losses)
        .collect::<BTreeMap<_, _>>();
    SyntheticScenario {
        id,
        observation,
        evaluator_losses,
    }
}

/// Hand-authored fixtures. Public features and evaluator losses are authored
/// separately; do not derive labels from confidence, surprise, or scenario id.
fn scenarios() -> Vec<SyntheticScenario> {
    vec![
        make_scenario(
            1,
            PublicObservation {
                confidence_milli: 960,
                surprise_milli: 120,
                occlusion_milli: 40,
                blur_milli: 20,
                novelty_milli: 70,
                compute_available: true,
                next_sample_available: false,
                alternate_view_available: true,
                timed_event_available: false,
            },
            [1.0, 1.4, 1.4, 1.8, 1.5, 3.0],
        ),
        make_scenario(
            2,
            PublicObservation {
                confidence_milli: 550,
                surprise_milli: 300,
                occlusion_milli: 880,
                blur_milli: 100,
                novelty_milli: 200,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: true,
                timed_event_available: true,
            },
            [8.0, 6.5, 4.0, 1.0, 3.0, 3.0],
        ),
        make_scenario(
            3,
            PublicObservation {
                confidence_milli: 470,
                surprise_milli: 300,
                occlusion_milli: 100,
                blur_milli: 850,
                novelty_milli: 150,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: false,
                timed_event_available: true,
            },
            [7.0, 5.0, 1.0, 4.0, 2.0, 3.0],
        ),
        make_scenario(
            4,
            PublicObservation {
                confidence_milli: 890,
                surprise_milli: 970,
                occlusion_milli: 80,
                blur_milli: 100,
                novelty_milli: 100,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: true,
                timed_event_available: true,
            },
            [1.0, 1.3, 1.2, 6.0, 1.5, 3.0],
        ),
        make_scenario(
            5,
            PublicObservation {
                confidence_milli: 920,
                surprise_milli: 250,
                occlusion_milli: 200,
                blur_milli: 100,
                novelty_milli: 940,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: true,
                timed_event_available: true,
            },
            [8.0, 7.0, 6.0, 1.0, 5.0, 4.0],
        ),
        make_scenario(
            6,
            PublicObservation {
                confidence_milli: 320,
                surprise_milli: 500,
                occlusion_milli: 700,
                blur_milli: 120,
                novelty_milli: 200,
                compute_available: false,
                next_sample_available: false,
                alternate_view_available: false,
                timed_event_available: false,
            },
            [7.0, 8.0, 7.0, 6.0, 5.0, 2.0],
        ),
        make_scenario(
            7,
            PublicObservation {
                confidence_milli: 800,
                surprise_milli: 870,
                occlusion_milli: 100,
                blur_milli: 100,
                novelty_milli: 80,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: true,
                timed_event_available: true,
            },
            [2.0, 2.2, 2.3, 5.0, 2.1, 3.0],
        ),
        make_scenario(
            8,
            PublicObservation {
                confidence_milli: 420,
                surprise_milli: 400,
                occlusion_milli: 870,
                blur_milli: 300,
                novelty_milli: 250,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: false,
                timed_event_available: true,
            },
            [7.0, 5.0, 1.0, 6.0, 2.0, 3.0],
        ),
        make_scenario(
            9,
            PublicObservation {
                confidence_milli: 700,
                surprise_milli: 350,
                occlusion_milli: 450,
                blur_milli: 80,
                novelty_milli: 720,
                compute_available: true,
                next_sample_available: true,
                alternate_view_available: true,
                timed_event_available: true,
            },
            [6.0, 4.0, 5.0, 1.0, 3.0, 3.5],
        ),
        make_scenario(
            10,
            PublicObservation {
                confidence_milli: 360,
                surprise_milli: 200,
                occlusion_milli: 100,
                blur_milli: 100,
                novelty_milli: 100,
                compute_available: true,
                next_sample_available: false,
                alternate_view_available: false,
                timed_event_available: false,
            },
            [6.0, 2.0, 6.0, 8.0, 5.0, 3.0],
        ),
        make_scenario(
            11,
            PublicObservation {
                confidence_milli: 400,
                surprise_milli: 250,
                occlusion_milli: 400,
                blur_milli: 100,
                novelty_milli: 100,
                compute_available: true,
                next_sample_available: false,
                alternate_view_available: false,
                timed_event_available: true,
            },
            [7.0, 5.0, 7.0, 7.0, 1.0, 3.0],
        ),
    ]
}

#[derive(Debug, Clone, Copy)]
enum Policy {
    AlwaysCheap,
    AlwaysEscalate,
    ConfidenceThreshold,
    SurpriseOnly,
    DeterministicRandom,
    SequentialEvidence,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Self::AlwaysCheap => "always_cheap",
            Self::AlwaysEscalate => "always_escalate",
            Self::ConfidenceThreshold => "confidence_threshold",
            Self::SurpriseOnly => "surprise_only",
            Self::DeterministicRandom => "deterministic_random",
            Self::SequentialEvidence => "sequential_evidence",
        }
    }

    /// This function has no access to evaluator labels, hidden world state,
    /// counterfactual outcomes, or future frames.
    fn propose(self, trial_id: u64, observation: PublicObservation) -> VisionAction {
        match self {
            Self::AlwaysCheap => VisionAction::KeepCheap,
            Self::AlwaysEscalate => VisionAction::EscalateCurrent,
            Self::ConfidenceThreshold => {
                if observation.confidence_milli < 550 {
                    VisionAction::EscalateCurrent
                } else {
                    VisionAction::KeepCheap
                }
            }
            Self::SurpriseOnly => {
                if observation.surprise_milli >= 700 {
                    if observation.alternate_view_available {
                        VisionAction::ChangeView
                    } else if observation.next_sample_available {
                        VisionAction::AcquireNext
                    } else {
                        VisionAction::KeepCheap
                    }
                } else {
                    VisionAction::KeepCheap
                }
            }
            Self::DeterministicRandom => {
                let available: Vec<VisionAction> = ACTIONS
                    .into_iter()
                    .filter(|action| observation.action_available(*action))
                    .collect();
                let mixed = trial_id
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                available[(mixed % available.len() as u64) as usize]
            }
            Self::SequentialEvidence => {
                if observation.occlusion_milli >= 600 {
                    if observation.alternate_view_available {
                        return VisionAction::ChangeView;
                    }
                    if observation.next_sample_available {
                        return VisionAction::AcquireNext;
                    }
                    if observation.timed_event_available {
                        return VisionAction::WaitForEvent;
                    }
                }
                if observation.blur_milli >= 600 {
                    if observation.next_sample_available {
                        return VisionAction::AcquireNext;
                    }
                    if observation.alternate_view_available {
                        return VisionAction::ChangeView;
                    }
                }
                if observation.novelty_milli >= 700 {
                    if observation.alternate_view_available {
                        return VisionAction::ChangeView;
                    }
                    if observation.next_sample_available {
                        return VisionAction::AcquireNext;
                    }
                }
                if observation.confidence_milli < 450 {
                    if observation.next_sample_available {
                        return VisionAction::AcquireNext;
                    }
                    if observation.timed_event_available {
                        return VisionAction::WaitForEvent;
                    }
                    if observation.compute_available {
                        return VisionAction::EscalateCurrent;
                    }
                    return VisionAction::AbstainOrHandoff;
                }
                if observation.confidence_milli < 550 && observation.compute_available {
                    return VisionAction::EscalateCurrent;
                }
                VisionAction::KeepCheap
            }
        }
    }
}

fn fits_remaining(action: VisionAction, spent_latency: u128, spent_sensor: u128) -> bool {
    let cost = costs(action);
    spent_latency + u128::from(cost["latency_unit"]) <= LATENCY_RUN_BUDGET
        && spent_sensor + u128::from(cost["sensor_action"]) <= SENSOR_RUN_BUDGET
}

fn guarded_action(
    requested: VisionAction,
    observation: PublicObservation,
    spent_latency: u128,
    spent_sensor: u128,
) -> Result<VisionAction, EvaluationError> {
    if observation.action_available(requested)
        && fits_remaining(requested, spent_latency, spent_sensor)
    {
        return Ok(requested);
    }
    for fallback in [VisionAction::KeepCheap, VisionAction::AbstainOrHandoff] {
        if observation.action_available(fallback)
            && fits_remaining(fallback, spent_latency, spent_sensor)
        {
            return Ok(fallback);
        }
    }
    Err(EvaluationError::NoRunBudgetFeasiblePlan)
}

fn evaluate_policy(policy: Policy, cases: &[SyntheticScenario]) -> Result<RunSummaryV1, EvaluationError> {
    let mut trials = Vec::with_capacity(cases.len());
    let mut spent_latency = 0_u128;
    let mut spent_sensor = 0_u128;

    for scenario in cases {
        let requested = policy.propose(scenario.id, scenario.observation);
        let chosen = guarded_action(
            requested,
            scenario.observation,
            spent_latency,
            spent_sensor,
        )?;
        let chosen_cost = costs(chosen);
        spent_latency += u128::from(chosen_cost["latency_unit"]);
        spent_sensor += u128::from(chosen_cost["sensor_action"]);

        // Counterfactual labels are assembled only after proposal/guard/commit.
        // Keep this block evaluator-side; never pass it to Policy::propose.
        let candidates = ACTIONS
            .into_iter()
            .map(|action| CandidateOutcomeV1 {
                action,
                feasible: scenario.observation.action_available(action),
                counterfactual_task_loss: scenario.evaluator_losses[&action],
                declared_resource_units: costs(action),
            })
            .collect();
        let actual_resource_units = BTreeMap::from([
            ("latency_unit".to_string(), Some(chosen_cost["latency_unit"])),
            ("sensor_action".to_string(), Some(chosen_cost["sensor_action"])),
        ]);

        trials.push(TrialV1 {
            trial_id: scenario.id,
            loss_profile_id: "synthetic-active-vision-task-loss-v1".to_string(),
            resource_profile: resource_profile(),
            candidates,
            chosen_action: chosen,
            action_executed: true,
            fresh_observation_received: chosen.can_acquire_observation()
                && scenario.observation.action_available(chosen),
            realized_task_loss: scenario.evaluator_losses[&chosen],
            actual_resource_units,
        });
    }

    summarize_run_v1(&trials)
}

fn mean_action_value(summary: &RunSummaryV1) -> f64 {
    summary
        .scores
        .iter()
        .map(|score| score.counterfactual_action_value_vs_cheap)
        .sum::<f64>()
        / summary.scores.len() as f64
}

fn run_report() -> Result<(), EvaluationError> {
    let cases = scenarios();
    let policies = [
        Policy::AlwaysCheap,
        Policy::AlwaysEscalate,
        Policy::ConfidenceThreshold,
        Policy::SurpriseOnly,
        Policy::DeterministicRandom,
        Policy::SequentialEvidence,
    ];

    println!("VIS-ACT-001 deterministic synthetic diagnostic pilot");
    println!("Not a powered study; resource axes are abstract synthetic units, not energy or measured latency.");
    println!("episodes={} latency_run_budget={} sensor_action_budget={}", cases.len(), LATENCY_RUN_BUDGET, SENSOR_RUN_BUDGET);
    println!("policy\tmean_loss\tmean_run_oracle_regret\tmean_action_value_vs_cheap\tlatency_used\tsensor_actions\tlatency_budget\tsensor_budget");

    for policy in policies {
        let summary = evaluate_policy(policy, &cases)?;
        let latency = summary.total_actual_resource_units["latency_unit"];
        let sensors = summary.total_actual_resource_units["sensor_action"];
        println!(
            "{}\t{:.3}\t{:.3}\t{:.3}\t{:?}\t{:?}\t{:?}\t{:?}",
            policy.name(),
            summary.mean_realized_loss,
            summary.mean_regret_vs_run_budget_oracle,
            mean_action_value(&summary),
            latency,
            sensors,
            summary.run_budget_status["latency_unit"],
            summary.run_budget_status["sensor_action"],
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run_report() {
        eprintln!("benchmark failed closed: {error:?}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_and_evaluator_labels_are_separate_fields() {
        let cases = scenarios();
        assert_eq!(cases.len(), 11);
        assert!(cases.iter().all(|case| case.evaluator_losses.len() == ACTIONS.len()));
        assert!(cases.iter().all(|case| case.evaluator_losses.values().all(|loss| loss.is_finite())));
    }

    #[test]
    fn sequential_policy_selects_different_observation_when_occluded() {
        let cases = scenarios();
        let occluded = cases.iter().find(|case| case.id == 2).unwrap();
        assert_eq!(
            Policy::SequentialEvidence.propose(occluded.id, occluded.observation),
            VisionAction::ChangeView
        );
    }

    #[test]
    fn sequential_policy_abstains_when_confidence_is_low_and_no_evidence_action_is_available() {
        let cases = scenarios();
        let blocked = cases.iter().find(|case| case.id == 6).unwrap();
        assert_eq!(
            Policy::SequentialEvidence.propose(blocked.id, blocked.observation),
            VisionAction::AbstainOrHandoff
        );
    }

    #[test]
    fn surprise_only_can_be_worse_on_a_distractor_than_keeping_the_cheap_result() {
        let cases = scenarios();
        let distractor = cases.iter().find(|case| case.id == 4).unwrap();
        assert_eq!(
            Policy::SurpriseOnly.propose(distractor.id, distractor.observation),
            VisionAction::ChangeView
        );
        assert!(
            distractor.evaluator_losses[&VisionAction::ChangeView]
                > distractor.evaluator_losses[&VisionAction::KeepCheap]
        );
    }

    #[test]
    fn every_policy_stays_inside_declared_synthetic_run_budgets() {
        for policy in [
            Policy::AlwaysCheap,
            Policy::AlwaysEscalate,
            Policy::ConfidenceThreshold,
            Policy::SurpriseOnly,
            Policy::DeterministicRandom,
            Policy::SequentialEvidence,
        ] {
            let summary = evaluate_policy(policy, &scenarios()).unwrap();
            assert_eq!(summary.run_budget_status["latency_unit"], evaluator::BudgetStatus::WithinBudget);
            assert_eq!(summary.run_budget_status["sensor_action"], evaluator::BudgetStatus::WithinBudget);
        }
    }

    #[test]
    fn scorer_is_invariant_to_case_order_after_trials_are_committed() {
        let mut original = scenarios();
        let mut reversed = original.clone();
        reversed.reverse();
        let a = evaluate_policy(Policy::SequentialEvidence, &original).unwrap();
        let b = evaluate_policy(Policy::SequentialEvidence, &reversed).unwrap();
        assert_eq!(a.mean_realized_loss, b.mean_realized_loss);
        assert_eq!(
            a.mean_run_budget_feasible_counterfactual_loss,
            b.mean_run_budget_feasible_counterfactual_loss
        );
    }
}
