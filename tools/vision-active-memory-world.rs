// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-ACT-003 multi-round causal active-vision and bounded-memory pilot.
//!
//! This is a deterministic synthetic diagnostic, not a real-camera or robotics
//! result. The policy interface receives only an Observation plus actor-owned
//! bounded memory; it never receives HiddenWorld, terminal truth, future cues,
//! branch labels, world seeds, or evaluator outcomes.
//!
//! Run and test without workspace dependencies:
//!   rustc --edition=2024 tools/vision-active-memory-world.rs -o /tmp/vis-act-memory
//!   /tmp/vis-act-memory
//!   rustc --edition=2024 --test tools/vision-active-memory-world.rs -o /tmp/vis-act-memory-tests
//!   /tmp/vis-act-memory-tests
//!
//! Episodes begin with a confirmed visible entity, then share an occluded
//! observation. In the hidden branch, the entity is either still present or is
//! removed while occluded. A legal alternate view is the only action that
//! reveals the branch. The same exogenous seed pairs the branches. Explicit
//! negative evidence is deliberately weaker than the first positive cue to
//! exercise confidence-gated overwrite failure versus provenance-aware revision.

use std::collections::VecDeque;

const ROOT_SEED: u64 = 0xBB67_AE85_84CA_A73B;
const WORLD_REPLICATES: usize = 12;
const PAIRS_PER_REPLICATE: usize = 16;
const LATENCY_BUDGET: u64 = 12;
const SENSOR_ACTION_BUDGET: u64 = 1;

fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn unit_milli(seed: u64, domain: u64) -> u16 {
    (mix64(seed ^ domain) % 1000) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cue {
    VisiblePresent,
    VisibleAbsent,
    Occluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    FrontView,
    AlternateView,
    OccludedFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Observation {
    tick: u8,
    cue: Cue,
    confidence_milli: u16,
    source: Source,
    alternate_view_available: bool,
}

impl Observation {
    fn is_conclusive(self) -> bool {
        matches!(self.cue, Cue::VisiblePresent | Cue::VisibleAbsent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HiddenWorld {
    // Evaluator-owned fields. Never passed to policy or memory methods.
    exogenous_seed: u64,
    terminal_present: bool,
    alternate_view_available: bool,
}

impl HiddenWorld {
    fn generated(exogenous_seed: u64, terminal_present: bool) -> Self {
        Self {
            exogenous_seed,
            terminal_present,
            alternate_view_available: unit_milli(exogenous_seed, 0x5649_4557_4156_4149) >= 250,
        }
    }

    fn with_view(exogenous_seed: u64, terminal_present: bool, available: bool) -> Self {
        Self {
            exogenous_seed,
            terminal_present,
            alternate_view_available: available,
        }
    }

    fn observation(self, tick: u8, alternate_view_active: bool) -> Observation {
        if tick == 0 {
            // Both hidden branches begin with the same positive cue: the entity
            // is present before the occlusion interval.
            return Observation {
                tick,
                cue: Cue::VisiblePresent,
                confidence_milli: 900,
                source: Source::FrontView,
                alternate_view_available: false,
            };
        }

        if alternate_view_active {
            Observation {
                tick,
                cue: if self.terminal_present {
                    Cue::VisiblePresent
                } else {
                    Cue::VisibleAbsent
                },
                // The fresh explicit absence is lower confidence than the old
                // positive cue. Provenance/freshness should beat confidence-only
                // overwrite, not because the absent cue is numerically stronger.
                confidence_milli: 720,
                source: Source::AlternateView,
                alternate_view_available: false,
            }
        } else {
            Observation {
                tick,
                cue: Cue::Occluded,
                confidence_milli: 0,
                source: Source::OccludedFrame,
                alternate_view_available: self.alternate_view_available,
            }
        }
    }

    /// The environment advances only after an action is committed.
    /// A view acquires branch-specific evidence only when that legal action is
    /// executed and the world exposes an alternate view.
    fn transition(self, next_tick: u8, action: Action) -> Observation {
        // A viewpoint move produces one fresh observation, not a permanently
        // clear stream. The scene is occluded again on the following step
        // unless another legal sensing action is executed.
        let view_active = action == Action::ChangeView && self.alternate_view_available;
        self.observation(next_tick, view_active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Wait,
    ChangeView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Present,
    Absent,
    Abstain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyKind {
    NoMemory,
    LastObservation,
    BoundedFifo,
    ConfidenceGated,
    ProvenanceMemoryPassive,
    ActiveRevision,
}

impl PolicyKind {
    fn name(self) -> &'static str {
        match self {
            Self::NoMemory => "no_memory",
            Self::LastObservation => "last_observation",
            Self::BoundedFifo => "bounded_fifo",
            Self::ConfidenceGated => "confidence_gated",
            Self::ProvenanceMemoryPassive => "provenance_memory_passive",
            Self::ActiveRevision => "active_revision",
        }
    }

    fn max_memory_entries(self) -> usize {
        match self {
            Self::NoMemory => 0,
            Self::LastObservation => 1,
            Self::BoundedFifo => 2,
            Self::ConfidenceGated => 1,
            Self::ProvenanceMemoryPassive | Self::ActiveRevision => 1,
        }
    }
}

#[derive(Debug, Clone)]
enum MemoryState {
    None,
    Last(Option<Observation>),
    Fifo(VecDeque<Observation>),
    ConfidenceBest(Option<Observation>),
    ProvenanceLatest(Option<Observation>),
}

impl MemoryState {
    fn new(policy: PolicyKind) -> Self {
        match policy {
            PolicyKind::NoMemory => Self::None,
            PolicyKind::LastObservation => Self::Last(None),
            PolicyKind::BoundedFifo => Self::Fifo(VecDeque::with_capacity(2)),
            PolicyKind::ConfidenceGated => Self::ConfidenceBest(None),
            PolicyKind::ProvenanceMemoryPassive | PolicyKind::ActiveRevision => {
                Self::ProvenanceLatest(None)
            }
        }
    }

    fn observe(&mut self, observation: Observation) {
        match self {
            Self::None => {}
            Self::Last(last) => *last = Some(observation),
            Self::Fifo(entries) => {
                entries.push_back(observation);
                while entries.len() > 2 {
                    entries.pop_front();
                }
            }
            Self::ConfidenceBest(best) => {
                if observation.is_conclusive()
                    && best.as_ref().map_or(true, |previous| {
                        observation.confidence_milli >= previous.confidence_milli
                    })
                {
                    *best = Some(observation);
                }
            }
            Self::ProvenanceLatest(latest) => {
                if observation.is_conclusive() {
                    // New direct evidence supersedes old evidence even if its
                    // confidence score is lower; source and time are retained.
                    *latest = Some(observation);
                }
            }
        }
    }

    fn decide(&self, policy: PolicyKind, terminal: Observation) -> Decision {
        match policy {
            PolicyKind::NoMemory => decision_from_observation(terminal).unwrap_or(Decision::Abstain),
            PolicyKind::LastObservation => match self {
                Self::Last(last) => last.and_then(decision_from_observation).unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            PolicyKind::BoundedFifo => match self {
                Self::Fifo(entries) => entries
                    .iter()
                    .rev()
                    .find_map(|entry| decision_from_observation(*entry))
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            PolicyKind::ConfidenceGated => match self {
                Self::ConfidenceBest(best) => best
                    .and_then(decision_from_observation)
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            PolicyKind::ProvenanceMemoryPassive | PolicyKind::ActiveRevision => match self {
                Self::ProvenanceLatest(latest) => latest
                    .filter(|evidence| {
                        // Retained evidence becomes stale after more than one
                        // step without direct confirmation. Do not assert that
                        // an entity persisted through an unobserved interval.
                        terminal.tick.saturating_sub(evidence.tick) <= 1
                            && evidence.source != Source::OccludedFrame
                    })
                    .and_then(decision_from_observation)
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
        }
    }
}

fn decision_from_observation(observation: Observation) -> Option<Decision> {
    match observation.cue {
        Cue::VisiblePresent => Some(Decision::Present),
        Cue::VisibleAbsent => Some(Decision::Absent),
        Cue::Occluded => None,
    }
}

/// Policy action API: no HiddenWorld, truth, seed, future observation, episode
/// id, or evaluator outcomes are accepted here.
fn propose_action(
    policy: PolicyKind,
    observation: Observation,
    sensor_action_used: bool,
) -> Action {
    if policy == PolicyKind::ProvenanceMemoryPassive {
        return Action::Wait;
    }
    if observation.cue == Cue::Occluded
        && observation.alternate_view_available
        && !sensor_action_used
    {
        Action::ChangeView
    } else {
        Action::Wait
    }
}

#[derive(Debug, Clone, Copy)]
struct EpisodeResult {
    truth_present: bool,
    decision: Decision,
    loss: f64,
    saw_explicit_absence: bool,
    view_actions: u64,
    latency_units: u64,
    budget_violations: u64,
}

fn run_episode(world: HiddenWorld, policy: PolicyKind) -> EpisodeResult {
    let mut memory = MemoryState::new(policy);
    let mut observation = world.observation(0, false);
    let mut sensor_action_used = false;
    let mut view_actions = 0_u64;
    let mut latency_units = 0_u64;
    let mut budget_violations = 0_u64;
    let mut saw_explicit_absence = false;

    for tick in 0..=3_u8 {
        memory.observe(observation);
        saw_explicit_absence |= observation.cue == Cue::VisibleAbsent;

        if tick == 3 {
            break;
        }

        let proposed = propose_action(policy, observation, sensor_action_used);
        // The evaluator independently validates the proposal and budget. No
        // unavailable/over-budget view is silently credited as new evidence.
        let action = match proposed {
            Action::ChangeView
                if observation.alternate_view_available
                    && !sensor_action_used
                    && latency_units.saturating_add(3) <= LATENCY_BUDGET
                    && view_actions < SENSOR_ACTION_BUDGET =>
            {
                sensor_action_used = true;
                view_actions += 1;
                latency_units += 3;
                Action::ChangeView
            }
            Action::ChangeView => {
                budget_violations += 1;
                latency_units = latency_units.saturating_add(1);
                Action::Wait
            }
            Action::Wait => {
                latency_units = latency_units.saturating_add(1);
                Action::Wait
            }
        };

        observation = world.transition(tick + 1, action);
    }

    let decision = memory.decide(policy, observation);
    let loss = match decision {
        Decision::Present if world.terminal_present => 0.0,
        Decision::Absent if !world.terminal_present => 0.0,
        Decision::Abstain => 0.25,
        _ => 1.0,
    };

    if latency_units > LATENCY_BUDGET || view_actions > SENSOR_ACTION_BUDGET {
        budget_violations += 1;
    }

    EpisodeResult {
        truth_present: world.terminal_present,
        decision,
        loss,
        saw_explicit_absence,
        view_actions,
        latency_units,
        budget_violations,
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    episodes: u64,
    answered: u64,
    correct_answered: u64,
    abstentions: u64,
    loss_sum: f64,
    removed_episodes: u64,
    false_persistence: u64,
    persistent_episodes: u64,
    persistent_correct: u64,
    removed_with_explicit_absence: u64,
    removed_revised_correctly: u64,
    view_actions: u64,
    latency_units: u64,
    budget_violations: u64,
}

impl Counts {
    fn record(&mut self, result: EpisodeResult) {
        self.episodes += 1;
        self.loss_sum += result.loss;
        self.view_actions += result.view_actions;
        self.latency_units += result.latency_units;
        self.budget_violations += result.budget_violations;

        match result.decision {
            Decision::Abstain => self.abstentions += 1,
            decision => {
                self.answered += 1;
                let correct = matches!(
                    (decision, result.truth_present),
                    (Decision::Present, true) | (Decision::Absent, false)
                );
                if correct {
                    self.correct_answered += 1;
                }
            }
        }

        if result.truth_present {
            self.persistent_episodes += 1;
            if result.decision == Decision::Present {
                self.persistent_correct += 1;
            }
        } else {
            self.removed_episodes += 1;
            if result.decision == Decision::Present {
                self.false_persistence += 1;
            }
            if result.saw_explicit_absence {
                self.removed_with_explicit_absence += 1;
                if result.decision == Decision::Absent {
                    self.removed_revised_correctly += 1;
                }
            }
        }
    }

    fn mean_loss(self) -> f64 {
        self.loss_sum / self.episodes.max(1) as f64
    }

    fn rate(numerator: u64, denominator: u64) -> String {
        if denominator == 0 {
            "NA".to_string()
        } else {
            format!("{:.4}", numerator as f64 / denominator as f64)
        }
    }
}

fn seed_for(replicate: usize, pair: usize) -> u64 {
    mix64(
        ROOT_SEED
            ^ (replicate as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (pair as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
    )
}

fn run_report() {
    let policies = [
        PolicyKind::NoMemory,
        PolicyKind::LastObservation,
        PolicyKind::BoundedFifo,
        PolicyKind::ConfidenceGated,
        PolicyKind::ProvenanceMemoryPassive,
        PolicyKind::ActiveRevision,
    ];

    println!("VIS-ACT-003 multi-round active-vision and memory diagnostic");
    println!("paired persistent/removed worlds; 4 observations per episode; view is the only branch-revealing action");
    println!("synthetic costs only; abstention loss=0.25; 12 world-seed replicates x 16 paired exogenous seeds");
    println!("policy\tworlds\tmean_loss\tmin_replicate_loss\tmax_replicate_loss\tanswered_accuracy\tabstention_rate\tpersistent_correct_rate\tremoved_false_persistence_rate\tremoved_revision_rate_when_absence_seen\tremoved_explicit_absence_cases\tview_actions_per_episode\tlatency_units_per_episode\tmax_memory_entries\tbudget_violations");

    for policy in policies {
        let mut total = Counts::default();
        let mut replicate_losses = Vec::with_capacity(WORLD_REPLICATES);

        for replicate in 0..WORLD_REPLICATES {
            let mut per_replicate = Counts::default();

            for pair in 0..PAIRS_PER_REPLICATE {
                let seed = seed_for(replicate, pair);
                // Paired counterfactual worlds share all exogenous properties;
                // only terminal persistence/removal truth differs.
                let persistent = HiddenWorld::generated(seed, true);
                let removed = HiddenWorld::generated(seed, false);

                per_replicate.record(run_episode(persistent, policy));
                per_replicate.record(run_episode(removed, policy));
            }

            replicate_losses.push(per_replicate.mean_loss());
            total.episodes += per_replicate.episodes;
            total.answered += per_replicate.answered;
            total.correct_answered += per_replicate.correct_answered;
            total.abstentions += per_replicate.abstentions;
            total.loss_sum += per_replicate.loss_sum;
            total.removed_episodes += per_replicate.removed_episodes;
            total.false_persistence += per_replicate.false_persistence;
            total.persistent_episodes += per_replicate.persistent_episodes;
            total.persistent_correct += per_replicate.persistent_correct;
            total.removed_with_explicit_absence += per_replicate.removed_with_explicit_absence;
            total.removed_revised_correctly += per_replicate.removed_revised_correctly;
            total.view_actions += per_replicate.view_actions;
            total.latency_units += per_replicate.latency_units;
            total.budget_violations += per_replicate.budget_violations;
        }

        let min_replicate_loss = replicate_losses.iter().copied().fold(f64::INFINITY, f64::min);
        let max_replicate_loss = replicate_losses.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        println!(
            "{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}",
            policy.name(),
            total.episodes,
            total.mean_loss(),
            min_replicate_loss,
            max_replicate_loss,
            Counts::rate(total.correct_answered, total.answered),
            Counts::rate(total.abstentions, total.episodes),
            Counts::rate(total.persistent_correct, total.persistent_episodes),
            Counts::rate(total.false_persistence, total.removed_episodes),
            Counts::rate(total.removed_revised_correctly, total.removed_with_explicit_absence),
            total.removed_with_explicit_absence,
            total.view_actions as f64 / total.episodes.max(1) as f64,
            total.latency_units as f64 / total.episodes.max(1) as f64,
            policy.max_memory_entries(),
            total.budget_violations,
        );
    }
}

fn main() {
    run_report();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_worlds_have_identical_observations_until_a_legal_view_reveals_truth() {
        let persistent = HiddenWorld::with_view(0xCAFE_BABE, true, true);
        let removed = HiddenWorld::with_view(0xCAFE_BABE, false, true);
        assert_eq!(persistent.observation(0, false), removed.observation(0, false));
        let p1 = persistent.observation(1, false);
        let r1 = removed.observation(1, false);
        assert_eq!(p1, r1);
        assert_eq!(p1.cue, Cue::Occluded);

        let p2 = persistent.transition(2, Action::ChangeView);
        let r2 = removed.transition(2, Action::ChangeView);
        assert_ne!(p2, r2);
        assert_eq!(p2.cue, Cue::VisiblePresent);
        assert_eq!(r2.cue, Cue::VisibleAbsent);
    }

    #[test]
    fn one_view_is_ephemeral_and_memory_must_preserve_its_evidence() {
        let world = HiddenWorld::with_view(0xF00D, false, true);
        let initial = world.observation(0, false);
        let occluded = world.observation(1, false);
        let fresh_absence = world.transition(2, Action::ChangeView);
        assert_eq!(fresh_absence.cue, Cue::VisibleAbsent);

        let terminal_occlusion = world.transition(3, Action::Wait);
        assert_eq!(terminal_occlusion.cue, Cue::Occluded);

        let mut memory = MemoryState::new(PolicyKind::ActiveRevision);
        for observation in [initial, occluded, fresh_absence, terminal_occlusion] {
            memory.observe(observation);
        }
        assert_eq!(
            memory.decide(PolicyKind::ActiveRevision, terminal_occlusion),
            Decision::Absent
        );
        assert_eq!(
            decision_from_observation(terminal_occlusion),
            None,
            "terminal pixels alone must not expose the hidden intervention"
        );

        let mut no_memory = MemoryState::new(PolicyKind::NoMemory);
        for observation in [initial, occluded, fresh_absence, terminal_occlusion] {
            no_memory.observe(observation);
        }
        assert_eq!(
            no_memory.decide(PolicyKind::NoMemory, terminal_occlusion),
            Decision::Abstain
        );
    }

    #[test]
    fn no_executed_view_means_no_branch_specific_observation() {
        let persistent = HiddenWorld::with_view(123, true, true);
        let removed = HiddenWorld::with_view(123, false, true);
        for tick in 0..=3 {
            assert_eq!(
                persistent.observation(tick, false),
                removed.observation(tick, false)
            );
        }
    }

    #[test]
    fn confidence_gate_negative_control_retains_stale_positive_against_newer_absence() {
        let mut memory = MemoryState::new(PolicyKind::ConfidenceGated);
        memory.observe(HiddenWorld::with_view(1, true, true).observation(0, false));
        let absence = HiddenWorld::with_view(1, false, true).observation(2, true);
        assert_eq!(absence.cue, Cue::VisibleAbsent);
        memory.observe(absence);
        assert_eq!(memory.decide(PolicyKind::ConfidenceGated, absence), Decision::Present);
    }

    #[test]
    fn provenance_memory_revises_to_newer_lower_confidence_absence() {
        let mut memory = MemoryState::new(PolicyKind::ActiveRevision);
        memory.observe(HiddenWorld::with_view(2, true, true).observation(0, false));
        let absence = HiddenWorld::with_view(2, false, true).observation(2, true);
        memory.observe(absence);
        assert_eq!(memory.decide(PolicyKind::ActiveRevision, absence), Decision::Absent);
    }

    #[test]
    fn provenance_memory_abstains_when_all_evidence_is_stale() {
        let mut memory = MemoryState::new(PolicyKind::ActiveRevision);
        memory.observe(HiddenWorld::with_view(3, true, false).observation(0, false));
        let terminal = HiddenWorld::with_view(3, false, false).observation(3, false);
        memory.observe(terminal);
        assert_eq!(memory.decide(PolicyKind::ActiveRevision, terminal), Decision::Abstain);
    }

    #[test]
    fn all_policies_obey_sensor_and_latency_budgets() {
        for policy in [
            PolicyKind::NoMemory,
            PolicyKind::LastObservation,
            PolicyKind::BoundedFifo,
            PolicyKind::ConfidenceGated,
            PolicyKind::ProvenanceMemoryPassive,
            PolicyKind::ActiveRevision,
        ] {
            let result = run_episode(HiddenWorld::with_view(9, false, true), policy);
            assert!(result.view_actions <= SENSOR_ACTION_BUDGET);
            assert!(result.latency_units <= LATENCY_BUDGET);
            assert_eq!(result.budget_violations, 0);
        }
    }

    #[test]
    fn undefined_rate_denominators_are_reported_as_na() {
        assert_eq!(Counts::rate(0, 0), "NA");
        assert_eq!(Counts::rate(1, 4), "0.2500");
    }

    #[test]
    fn exact_seed_is_deterministic_and_paired_branches_share_exogenous_availability() {
        let seed = seed_for(3, 7);
        let a = HiddenWorld::generated(seed, true);
        let b = HiddenWorld::generated(seed, false);
        assert_eq!(a, HiddenWorld::generated(seed, true));
        assert_eq!(a.alternate_view_available, b.alternate_view_available);
        assert_eq!(a.observation(1, false), b.observation(1, false));
    }

    #[test]
    fn policy_decision_interface_has_no_world_argument_or_truth_parameter() {
        let observation = HiddenWorld::with_view(4, false, true).observation(1, false);
        assert_eq!(propose_action(PolicyKind::ActiveRevision, observation, false), Action::ChangeView);
        assert_eq!(propose_action(PolicyKind::ProvenanceMemoryPassive, observation, false), Action::Wait);
    }
}
