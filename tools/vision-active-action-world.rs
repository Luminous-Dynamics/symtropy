// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Parameterized causal synthetic world for active visual-action selection.
//!
//! The policy receives only PublicObservation. HiddenWorld and all candidate
//! outcomes remain evaluator-side. Worlds pair on exogenous seed but can differ
//! in hidden target identity; when that target is fully occluded, initial legal
//! observations are identical while a successful executed view can disambiguate.
//!
//! These are deterministic synthetic units and simulated outcomes, not camera,
//! robotics, physical safety, latency, or energy results.
//!
//! Standalone report:
//! rustc --edition=2024 tools/vision-active-action-world.rs -o /tmp/vis-act-world
//! /tmp/vis-act-world
//!
//! Standalone tests (includes the reference scorer's tests):
//! rustc --edition=2024 --test tools/vision-active-action-world.rs -o /tmp/vis-act-world-tests
//! /tmp/vis-act-world-tests

#[path = "vision-active-action-reference.rs"]
mod evaluator;

use evaluator::{
    BudgetStatus, CandidateOutcomeV1, EvaluationError, ResourceAxisProfileV1,
    ResourceProfileV1, RunSummaryV1, TrialV1, VisionAction, summarize_run_v1,
};
use std::collections::BTreeMap;

const ROOT_SEED: u64 = 0x6A09_E667_F3BC_C909;
const EPISODE_COUNT: usize = 36;
const LATENCY_RUN_BUDGET_PER_EPISODE: u64 = 5;
const SENSOR_RUN_BUDGET_DIVISOR: u128 = 3;
const TRIAL_LATENCY_BUDGET: u64 = 10;
const TRIAL_SENSOR_BUDGET: u64 = 1;
const MIN_ACTION_LATENCY_UNITS: u64 = 1;

const ACTIONS: [VisionAction; 6] = [
    VisionAction::KeepCheap,
    VisionAction::EscalateCurrent,
    VisionAction::AcquireNext,
    VisionAction::ChangeView,
    VisionAction::WaitForEvent,
    VisionAction::AbstainOrHandoff,
];

fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn unit_milli(seed: u64, domain: u64) -> u16 {
    (mix64(seed ^ domain) % 1000) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PublicObservation {
    /// Initial sensor's fallible coarse identity cue.
    initial_guess: bool,
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

    fn public_hash(self, policy_seed: u64) -> u64 {
        let mut bits = (self.initial_guess as u64);
        bits |= u64::from(self.confidence_milli) << 1;
        bits |= u64::from(self.surprise_milli) << 12;
        bits |= u64::from(self.occlusion_milli) << 23;
        bits |= u64::from(self.blur_milli) << 34;
        bits |= u64::from(self.novelty_milli) << 45;
        bits ^= (self.compute_available as u64) << 56;
        bits ^= (self.next_sample_available as u64) << 57;
        bits ^= (self.alternate_view_available as u64) << 58;
        bits ^= (self.timed_event_available as u64) << 59;
        mix64(bits ^ policy_seed.rotate_left(19))
    }
}

/// This struct is evaluator-owned. Do not pass it, its seed, target identity,
/// or outcome functions to a policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HiddenWorld {
    episode_id: u64,
    world_seed: u64,
    target_identity: bool,
    occlusion_milli: u16,
    blur_milli: u16,
    distractor_salience_milli: u16,
    novelty_milli: u16,
    sensor_noise_milli: u16,
    compute_available: bool,
    next_sample_available: bool,
    alternate_view_available: bool,
    timed_event_available: bool,
    next_sample_reveals_target: bool,
    alternate_view_reveals_target: bool,
    timed_event_reveals_target: bool,
}

impl HiddenWorld {
    fn error_rate_milli(self, action: VisionAction) -> u16 {
        let raw = match action {
            VisionAction::KeepCheap => {
                u32::from(self.occlusion_milli) * 420 / 1000
                    + u32::from(self.blur_milli) * 300 / 1000
                    + u32::from(self.distractor_salience_milli) * 250 / 1000
                    + u32::from(self.sensor_noise_milli)
            }
            VisionAction::EscalateCurrent => {
                // Compute can suppress blur/distractor errors but cannot see
                // through a genuinely occluded current frame.
                if self.occlusion_milli >= 650 {
                    1000
                } else {
                    u32::from(self.occlusion_milli) * 300 / 1000
                        + u32::from(self.blur_milli) * 100 / 1000
                        + u32::from(self.distractor_salience_milli) * 100 / 1000
                        + u32::from(self.sensor_noise_milli) / 2
                }
            }
            VisionAction::AcquireNext => {
                u32::from(self.blur_milli) * 180 / 1000
                    + u32::from(self.distractor_salience_milli) * 100 / 1000
                    + u32::from(self.sensor_noise_milli)
            }
            VisionAction::ChangeView => {
                u32::from(self.distractor_salience_milli) * 80 / 1000
                    + u32::from(self.sensor_noise_milli) / 2
            }
            VisionAction::WaitForEvent => {
                u32::from(self.sensor_noise_milli) / 2
            }
            VisionAction::AbstainOrHandoff => 0,
        };
        raw.min(1000) as u16
    }

    fn initial_observation(self) -> PublicObservation {
        let hidden = self.occlusion_milli >= 700;
        let initial_guess = if hidden {
            // Domain-separated prior draw deliberately ignores the hidden
            // identity, yielding identical visible history for paired worlds.
            unit_milli(self.world_seed, 0x494E_4954_4849_4444) >= 500
        } else {
            self.sensor_guess(
                VisionAction::KeepCheap,
                self.error_rate_milli(VisionAction::KeepCheap),
            )
        };

        let confidence_penalty = u32::from(self.occlusion_milli) * 500 / 1000
            + u32::from(self.blur_milli) * 350 / 1000
            + u32::from(self.distractor_salience_milli) * 150 / 1000;
        let confidence_milli = (1000_u32.saturating_sub(confidence_penalty)) as u16;
        let surprise_milli = self.distractor_salience_milli.max(self.novelty_milli);
        PublicObservation {
            initial_guess,
            confidence_milli,
            surprise_milli,
            occlusion_milli: self.occlusion_milli,
            blur_milli: self.blur_milli,
            novelty_milli: self.novelty_milli,
            compute_available: self.compute_available,
            next_sample_available: self.next_sample_available,
            alternate_view_available: self.alternate_view_available,
            timed_event_available: self.timed_event_available,
        }
    }

    fn sensor_guess(self, action: VisionAction, error_rate_milli: u16) -> bool {
        let domain = match action {
            VisionAction::KeepCheap => 0x5345_4E53_4F52_0001,
            VisionAction::EscalateCurrent => 0x5345_4E53_4F52_0002,
            VisionAction::AcquireNext => 0x5345_4E53_4F52_0003,
            VisionAction::ChangeView => 0x5345_4E53_4F52_0004,
            VisionAction::WaitForEvent => 0x5345_4E53_4F52_0005,
            VisionAction::AbstainOrHandoff => 0x5345_4E53_4F52_0006,
        };
        let draw = unit_milli(self.world_seed, domain);
        if draw < error_rate_milli {
            !self.target_identity
        } else {
            self.target_identity
        }
    }

    /// Action-conditioned post-action evidence and task decision. This function
    /// is evaluator-only; the policy sees only initial_observation().
    fn task_loss_after(self, action: VisionAction) -> f64 {
        if action == VisionAction::AbstainOrHandoff {
            return 0.35;
        }

        let observation_reveals_target = match action {
            VisionAction::KeepCheap => false,
            VisionAction::EscalateCurrent => self.occlusion_milli < 650,
            VisionAction::AcquireNext => self.next_sample_reveals_target,
            VisionAction::ChangeView => self.alternate_view_reveals_target,
            VisionAction::WaitForEvent => self.timed_event_reveals_target,
            VisionAction::AbstainOrHandoff => false,
        };

        let estimate = if action == VisionAction::KeepCheap {
            self.initial_observation().initial_guess
        } else if observation_reveals_target {
            self.sensor_guess(action, self.error_rate_milli(action))
        } else {
            self.initial_observation().initial_guess
        };

        if estimate == self.target_identity { 0.0 } else { 1.0 }
    }
}

fn generate_world(episode_id: u64, world_seed: u64) -> HiddenWorld {
    // A crossed nuisance grid guarantees a mix of easy, occluded, blurred,
    // distractor-heavy and novel scenes; hidden target identity and sensor noise
    // come from separate seed domains.
    const OCCLUSION_LEVELS: [u16; 4] = [0, 350, 700, 900];
    const BLUR_LEVELS: [u16; 3] = [0, 450, 800];
    const DISTRACTOR_LEVELS: [u16; 3] = [50, 500, 950];
    const NOVELTY_LEVELS: [u16; 3] = [50, 400, 900];

    let target_identity = unit_milli(world_seed, 0x5441_5247_4554_0001) >= 500;
    let occlusion_milli = OCCLUSION_LEVELS[(episode_id as usize) % OCCLUSION_LEVELS.len()];
    let blur_milli = BLUR_LEVELS[((episode_id as usize) / 4) % BLUR_LEVELS.len()];
    let distractor_salience_milli =
        DISTRACTOR_LEVELS[((episode_id as usize) / 3) % DISTRACTOR_LEVELS.len()];
    let novelty_milli = NOVELTY_LEVELS[((episode_id as usize) / 7) % NOVELTY_LEVELS.len()];
    let sensor_noise_milli = (unit_milli(world_seed, 0x4E4F_4953_4500_0001) / 8).min(120);

    let compute_available = unit_milli(world_seed, 0x4143_5449_4F4E_0001) >= 80;
    let next_sample_available = unit_milli(world_seed, 0x4143_5449_4F4E_0002) >= 140;
    let alternate_view_available = unit_milli(world_seed, 0x4143_5449_4F4E_0003) >= 160;
    let timed_event_available = unit_milli(world_seed, 0x4143_5449_4F4E_0004) >= 420;

    let next_sample_reveals_target = next_sample_available
        && (blur_milli >= 400
            || unit_milli(world_seed, 0x4F4253_4552_5645_01) >= 560);
    let alternate_view_reveals_target = alternate_view_available
        && (occlusion_milli >= 300
            || distractor_salience_milli >= 500
            || unit_milli(world_seed, 0x4F4253_4552_5645_02) >= 760);
    let timed_event_reveals_target = timed_event_available
        && (novelty_milli >= 400
            || unit_milli(world_seed, 0x4F4253_4552_5645_03) >= 800);

    HiddenWorld {
        episode_id,
        world_seed,
        target_identity,
        occlusion_milli,
        blur_milli,
        distractor_salience_milli,
        novelty_milli,
        sensor_noise_milli,
        compute_available,
        next_sample_available,
        alternate_view_available,
        timed_event_available,
        next_sample_reveals_target,
        alternate_view_reveals_target,
        timed_event_reveals_target,
    }
}

fn generate_worlds(count: usize, root_seed: u64) -> Vec<HiddenWorld> {
    let mut worlds: Vec<HiddenWorld> = (0..count)
        .map(|index| {
            let seed = mix64(root_seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            generate_world(index as u64, seed)
        })
        .collect();

    // Shuffle by an independent order domain so episode index is not coupled
    // to the policy-visible presentation order or target identity.
    worlds.sort_by_key(|world| mix64(world.world_seed ^ 0x4F52_4445_525F_5345));
    worlds
}

fn cost(action: VisionAction) -> BTreeMap<String, u64> {
    let (latency, sensor_action) = match action {
        VisionAction::KeepCheap => (2, 0),
        VisionAction::EscalateCurrent => (8, 0),
        VisionAction::AcquireNext => (4, 1),
        VisionAction::ChangeView => (5, 1),
        VisionAction::WaitForEvent => (6, 1),
        VisionAction::AbstainOrHandoff => (1, 0),
    };
    BTreeMap::from([
        ("latency_unit".to_string(), latency),
        ("sensor_action".to_string(), sensor_action),
    ])
}

fn resource_profile(episode_count: usize) -> ResourceProfileV1 {
    BTreeMap::from([
        (
            "latency_unit".to_string(),
            ResourceAxisProfileV1 {
                unit_id: "synthetic_latency_unit".to_string(),
                trial_budget: Some(TRIAL_LATENCY_BUDGET),
                run_budget: Some(
                    u128::from(LATENCY_RUN_BUDGET_PER_EPISODE) * episode_count as u128,
                ),
            },
        ),
        (
            "sensor_action".to_string(),
            ResourceAxisProfileV1 {
                unit_id: "synthetic_sensor_action".to_string(),
                trial_budget: Some(TRIAL_SENSOR_BUDGET),
                run_budget: Some((episode_count as u128) / SENSOR_RUN_BUDGET_DIVISOR),
            },
        ),
    ])
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

    /// Deliberately has no HiddenWorld reference, target label, outcome table,
    /// world seed, future observations, or oracle value.
    fn propose(self, observation: PublicObservation, policy_seed: u64) -> VisionAction {
        match self {
            Self::AlwaysCheap => VisionAction::KeepCheap,
            Self::AlwaysEscalate => {
                if observation.compute_available {
                    VisionAction::EscalateCurrent
                } else {
                    VisionAction::KeepCheap
                }
            }
            Self::ConfidenceThreshold => {
                if observation.confidence_milli < 520 && observation.compute_available {
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
                let index = (observation.public_hash(policy_seed) % available.len() as u64) as usize;
                available[index]
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

fn action_fits(action: VisionAction, observation: PublicObservation) -> bool {
    observation.action_available(action)
        && cost(action)["latency_unit"] <= TRIAL_LATENCY_BUDGET
        && cost(action)["sensor_action"] <= TRIAL_SENSOR_BUDGET
}

fn guard_budget(
    requested: VisionAction,
    observation: PublicObservation,
    spent_latency: u128,
    spent_sensor: u128,
    episodes_after: usize,
    run_latency_budget: u128,
    run_sensor_budget: u128,
) -> Result<VisionAction, EvaluationError> {
    let fits = |action: VisionAction| {
        let units = cost(action);
        spent_latency
            + u128::from(units["latency_unit"])
            + episodes_after as u128 * u128::from(MIN_ACTION_LATENCY_UNITS)
            <= run_latency_budget
            && spent_sensor + u128::from(units["sensor_action"]) <= run_sensor_budget
    };
    if action_fits(requested, observation) && fits(requested) {
        return Ok(requested);
    }
    for fallback in [VisionAction::KeepCheap, VisionAction::AbstainOrHandoff] {
        if action_fits(fallback, observation) && fits(fallback) {
            return Ok(fallback);
        }
    }
    Err(EvaluationError::NoRunBudgetFeasiblePlan)
}

fn evaluate_world_action(world: HiddenWorld, action: VisionAction) -> f64 {
    world.task_loss_after(action)
}

fn evaluate_policy(
    policy: Policy,
    worlds: &[HiddenWorld],
    policy_seed: u64,
) -> Result<RunSummaryV1, EvaluationError> {
    let episode_count = worlds.len();
    let run_latency_budget = u128::from(LATENCY_RUN_BUDGET_PER_EPISODE) * episode_count as u128;
    let run_sensor_budget = (episode_count as u128) / SENSOR_RUN_BUDGET_DIVISOR;
    let profile = resource_profile(episode_count);
    let mut trials = Vec::with_capacity(episode_count);
    let mut spent_latency = 0_u128;
    let mut spent_sensor = 0_u128;

    for (index, world) in worlds.iter().enumerate() {
        let public_observation = world.initial_observation();
        let requested = policy.propose(public_observation, policy_seed);
        let episodes_after = episode_count - index - 1;
        let chosen = guard_budget(
            requested,
            public_observation,
            spent_latency,
            spent_sensor,
            episodes_after,
            run_latency_budget,
            run_sensor_budget,
        )?;

        let chosen_cost = cost(chosen);
        spent_latency += u128::from(chosen_cost["latency_unit"]);
        spent_sensor += u128::from(chosen_cost["sensor_action"]);

        // Crucial boundary: all hidden outcomes are read only after the action
        // has been chosen/guarded. Nothing from this block flows back to policy.
        let candidates = ACTIONS
            .into_iter()
            .map(|action| CandidateOutcomeV1 {
                action,
                feasible: public_observation.action_available(action),
                counterfactual_task_loss: evaluate_world_action(*world, action),
                declared_resource_units: cost(action),
            })
            .collect();

        let actual_resource_units = BTreeMap::from([
            ("latency_unit".to_string(), Some(chosen_cost["latency_unit"])),
            ("sensor_action".to_string(), Some(chosen_cost["sensor_action"])),
        ]);

        trials.push(TrialV1 {
            trial_id: world.episode_id,
            loss_profile_id: "causal-synthetic-active-vision-v1".to_string(),
            resource_profile: profile.clone(),
            candidates,
            chosen_action: chosen,
            action_executed: true,
            fresh_observation_received: chosen.can_acquire_observation()
                && public_observation.action_available(chosen),
            realized_task_loss: evaluate_world_action(*world, chosen),
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

fn mean_resource(summary: &RunSummaryV1, key: &str) -> Option<f64> {
    let value = summary.total_actual_resource_units.get(key)?.as_ref()?;
    Some(*value as f64 / summary.scores.len() as f64)
}

fn run_report() -> Result<(), EvaluationError> {
    let worlds = generate_worlds(EPISODE_COUNT, ROOT_SEED);
    let policies = [
        Policy::AlwaysCheap,
        Policy::AlwaysEscalate,
        Policy::ConfidenceThreshold,
        Policy::SurpriseOnly,
        Policy::DeterministicRandom,
        Policy::SequentialEvidence,
    ];
    let profile = resource_profile(worlds.len());

    println!("VIS-ACT-002 causal synthetic active-vision pilot");
    println!("Causal generated outcomes; not real sensors, not a powered study, abstract resource units only.");
    println!("episodes={} latency_budget={} sensor_action_budget={}", worlds.len(), profile["latency_unit"].run_budget.unwrap(), profile["sensor_action"].run_budget.unwrap());
    println!("policy_seed_replicates=16 on the same world set; these are not independent world samples.");
    println!("policy\tseed_replicates\tmean_loss\tmin_loss\tmax_loss\tmean_run_oracle_regret\tmean_action_value_vs_cheap\tlatency_units_per_episode\tsensor_actions_per_episode");

    for policy in policies {
        let repetitions = if matches!(policy, Policy::DeterministicRandom) { 16 } else { 1 };
        let mut summaries = Vec::with_capacity(repetitions);
        for policy_seed in 0..repetitions {
            summaries.push(evaluate_policy(policy, &worlds, policy_seed as u64)?);
        }
        let mean_loss = summaries.iter().map(|summary| summary.mean_realized_loss).sum::<f64>()
            / summaries.len() as f64;
        let min_loss = summaries.iter().map(|summary| summary.mean_realized_loss).fold(f64::INFINITY, f64::min);
        let max_loss = summaries.iter().map(|summary| summary.mean_realized_loss).fold(f64::NEG_INFINITY, f64::max);
        let mean_regret = summaries.iter().map(|summary| summary.mean_regret_vs_run_budget_oracle).sum::<f64>()
            / summaries.len() as f64;
        let mean_action_value = summaries.iter().map(mean_action_value).sum::<f64>() / summaries.len() as f64;
        let latency_per_episode = summaries.iter().filter_map(|summary| mean_resource(summary, "latency_unit")).sum::<f64>()
            / summaries.len() as f64;
        let sensor_per_episode = summaries.iter().filter_map(|summary| mean_resource(summary, "sensor_action")).sum::<f64>()
            / summaries.len() as f64;

        println!(
            "{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.3}\t{:.3}",
            policy.name(), repetitions, mean_loss, min_loss, max_loss,
            mean_regret, mean_action_value, latency_per_episode, sensor_per_episode
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run_report() {
        eprintln!("causal synthetic benchmark failed closed: {error:?}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_identity_is_not_in_initial_observation_for_paired_fully_occluded_worlds() {
        let shared_seed = 0x1234_9876_ABCDEFFF;
        let make = |target_identity| HiddenWorld {
            episode_id: 7,
            world_seed: shared_seed,
            target_identity,
            occlusion_milli: 900,
            blur_milli: 300,
            distractor_salience_milli: 0,
            novelty_milli: 400,
            sensor_noise_milli: 0,
            compute_available: true,
            next_sample_available: true,
            alternate_view_available: true,
            timed_event_available: true,
            next_sample_reveals_target: true,
            alternate_view_reveals_target: true,
            timed_event_reveals_target: true,
        };
        let a = make(false);
        let b = make(true);
        assert_eq!(a.initial_observation(), b.initial_observation());
        assert_eq!(a.task_loss_after(VisionAction::ChangeView), 0.0);
        assert_eq!(b.task_loss_after(VisionAction::ChangeView), 0.0);
        assert_ne!(a.sensor_guess(VisionAction::ChangeView, 0), b.sensor_guess(VisionAction::ChangeView, 0));
    }

    #[test]
    fn high_occlusion_cannot_be_repaired_by_more_compute_on_same_frame() {
        let world = HiddenWorld {
            episode_id: 1,
            world_seed: 5,
            target_identity: true,
            occlusion_milli: 900,
            blur_milli: 0,
            distractor_salience_milli: 0,
            novelty_milli: 50,
            sensor_noise_milli: 0,
            compute_available: true,
            next_sample_available: true,
            alternate_view_available: true,
            timed_event_available: false,
            next_sample_reveals_target: false,
            alternate_view_reveals_target: true,
            timed_event_reveals_target: false,
        };
        assert_eq!(world.task_loss_after(VisionAction::EscalateCurrent), world.task_loss_after(VisionAction::KeepCheap));
        assert_eq!(world.task_loss_after(VisionAction::ChangeView), 0.0);
    }

    #[test]
    fn causal_world_generation_is_deterministic_for_exact_seed() {
        assert_eq!(generate_worlds(EPISODE_COUNT, ROOT_SEED), generate_worlds(EPISODE_COUNT, ROOT_SEED));
    }

    #[test]
    fn generator_covers_required_nuisance_strata() {
        let worlds = generate_worlds(EPISODE_COUNT, ROOT_SEED);
        assert!(worlds.iter().any(|world| world.occlusion_milli >= 700));
        assert!(worlds.iter().any(|world| world.blur_milli >= 700));
        assert!(worlds.iter().any(|world| world.distractor_salience_milli >= 900));
        assert!(worlds.iter().any(|world| world.novelty_milli >= 900));
        assert!(worlds.iter().any(|world| !world.alternate_view_available));
        assert!(worlds.iter().any(|world| !world.compute_available));
    }

    #[test]
    fn actor_policy_api_accepts_only_public_observation_and_policy_seed() {
        let world = generate_world(3, 77);
        let observation = world.initial_observation();
        let action = Policy::SequentialEvidence.propose(observation, 0);
        assert!(observation.action_available(action));
    }

    #[test]
    fn all_policies_stay_within_shared_budgets_and_summary_is_repeatable() {
        let worlds = generate_worlds(EPISODE_COUNT, ROOT_SEED);
        for policy in [
            Policy::AlwaysCheap,
            Policy::AlwaysEscalate,
            Policy::ConfidenceThreshold,
            Policy::SurpriseOnly,
            Policy::DeterministicRandom,
            Policy::SequentialEvidence,
        ] {
            let first = evaluate_policy(policy, &worlds, 0).unwrap();
            let second = evaluate_policy(policy, &worlds, 0).unwrap();
            assert_eq!(first, second);
            assert_eq!(first.run_budget_status["latency_unit"], BudgetStatus::WithinBudget);
            assert_eq!(first.run_budget_status["sensor_action"], BudgetStatus::WithinBudget);
        }
    }

    #[test]
    fn paired_hidden_worlds_are_same_observation_but_post_view_truth_is_distinguishable() {
        let base = HiddenWorld {
            episode_id: 1,
            world_seed: 0xCAFEBABE,
            target_identity: false,
            occlusion_milli: 900,
            blur_milli: 100,
            distractor_salience_milli: 0,
            novelty_milli: 200,
            sensor_noise_milli: 0,
            compute_available: true,
            next_sample_available: true,
            alternate_view_available: true,
            timed_event_available: true,
            next_sample_reveals_target: true,
            alternate_view_reveals_target: true,
            timed_event_reveals_target: true,
        };
        let counterfactual = HiddenWorld { target_identity: true, ..base };
        assert_eq!(base.initial_observation(), counterfactual.initial_observation());
        assert_ne!(
            base.sensor_guess(VisionAction::ChangeView, 0),
            counterfactual.sensor_guess(VisionAction::ChangeView, 0)
        );
    }
}
