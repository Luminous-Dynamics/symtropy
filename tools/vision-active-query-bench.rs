// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-ACT-004: active evidence acquisition under a fixed sensing budget.
//!
//! Standalone reference:
//!   rustc --edition=2024 --test tools/vision-active-query-bench.rs -o /tmp/active-query-tests
//!   /tmp/active-query-tests
//!   rustc --edition=2024 -O tools/vision-active-query-bench.rs -o /tmp/active-query-bench
//!   /tmp/active-query-bench
//!
//! This is a synthetic policy-allocation experiment, not RGB perception or a
//! real-robot benchmark. Every policy receives a matched pre-generated world,
//! but only sees the observation returned for the action it chose.

const TOTAL_CLAIMS: usize = 8;
const QUERY_KEYS: [usize; 3] = [0, 1, 2];
const QUERY_COUNT: usize = QUERY_KEYS.len();
const STRATA_COUNT: usize = 7;
const STRATA_NAMES: [&str; STRATA_COUNT] = [
    "world_query_claim_changed_last_8_ticks",
    "query_evidence_arrived_out_of_order",
    "latest_query_evidence_low_confidence",
    "query_observation_delay_gt_2",
    "profile_high_claim_flip_rate",
    "profile_high_corruption_rate",
    "profile_high_distractor_rate",
];
const HORIZON: usize = 48;
const REPLICATES: usize = 12;
const PAIRS: usize = 32;
const BOOTSTRAPS: usize = 2_000;
// Independent seed domain for the frozen VOI replication (the policy itself is unchanged).
const ROOT_SEED: u64 = 0x9B05_688C_2B3E_6C1F;
const PRIOR_SENSOR_BASE_PER_THOUSAND: f64 = 32.5;

fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix64(self.0)
    }
    fn range(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next() % upper as u64) as usize
    }
    fn chance(&mut self, per_thousand: u64) -> bool {
        self.next() % 1_000 < per_thousand
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family { Development, HeldOut }

impl Family {
    fn name(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::HeldOut => "held_out",
        }
    }
    fn sample_profile(self, rng: &mut Rng) -> Profile {
        let (distractors, flips, delays, corruptions) = match self {
            Self::Development => (
                [150_u64, 250, 350].as_slice(),
                [10_u64, 20, 30].as_slice(),
                [0_usize, 1, 2].as_slice(),
                [0_u64, 10, 20].as_slice(),
            ),
            Self::HeldOut => (
                [350_u64, 500, 650].as_slice(),
                [30_u64, 50, 70].as_slice(),
                [2_usize, 3, 4, 5].as_slice(),
                [40_u64, 55, 70].as_slice(),
            ),
        };
        Profile {
            distractor_rate: distractors[rng.range(distractors.len())],
            flip_rate: flips[rng.range(flips.len())],
            max_delay: delays[rng.range(delays.len())],
            corruption_rate: corruptions[rng.range(corruptions.len())],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Profile {
    distractor_rate: u64,
    flip_rate: u64,
    max_delay: usize,
    corruption_rate: u64,
}

fn sample_confidence(rng: &mut Rng) -> u16 {
    match rng.range(1_000) {
        0..=749 => 950,
        750..=949 => 850,
        _ => 650,
    }
}

fn conditional_corruption_rate(base_rate: u64, confidence: u16) -> u64 {
    let multiplier = match confidence {
        950 => 20_u64,
        850 => 160_u64,
        650 => 320_u64,
        _ => 160_u64,
    };
    (base_rate.saturating_mul(multiplier).saturating_add(31) / 63).min(999)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Evidence {
    key: u8,
    value: bool,
    source_tick: u8,
    arrival_tick: u8,
    confidence: u16,
}

impl Evidence {
    fn key(self) -> usize { usize::from(self.key) }
    fn rank(self) -> (u8, u8) { (self.source_tick, self.arrival_tick) }
    fn newer_than(self, old: Self) -> bool { self.rank() > old.rank() }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveEpisode {
    // Evaluator-only hidden truth. The policy receives observations, never this vector.
    truth: Vec<[bool; TOTAL_CLAIMS]>,
    profile: Profile,
    // observations[tick][requested query key] is a pre-generated potential
    // result. A policy is given only the cell corresponding to its chosen action.
    observations: Vec<[Evidence; QUERY_COUNT]>,
}

impl ActiveEpisode {
    fn generate(seed: u64, family: Family) -> Self {
        let mut profile_rng = Rng::new(mix64(seed ^ 0x5052_4F46_494C_45));
        let profile = family.sample_profile(&mut profile_rng);

        // Keep the public query fixed across policies and seeds. This makes
        // matched action choices semantically comparable while the five
        // remaining claims provide the distractor pool.
        let query_keys = QUERY_KEYS;
        let hot_distractor = 3_usize;

        let mut world_rng = Rng::new(mix64(seed ^ 0x574F_524C_445F_5255));
        let mut truth = vec![[false; TOTAL_CLAIMS]; HORIZON];
        for key in 0..TOTAL_CLAIMS {
            truth[0][key] = world_rng.chance(500);
        }
        for tick in 1..HORIZON {
            truth[tick] = truth[tick - 1];
            for key in 0..TOTAL_CLAIMS {
                if world_rng.chance(profile.flip_rate) {
                    truth[tick][key] = !truth[tick][key];
                }
            }
        }

        let mut sensor_rng = Rng::new(mix64(seed ^ 0x5345_4E53_4F52_5F52));
        let mut observations = Vec::with_capacity(HORIZON);
        for tick in 0..HORIZON {
            let candidates: [Evidence; QUERY_COUNT] = std::array::from_fn(|request_index| {
                let delay = sensor_rng.range(profile.max_delay + 1);
                let source_tick = tick.saturating_sub(delay);
                let observed_key = if sensor_rng.chance(profile.distractor_rate) {
                    hot_distractor
                } else {
                    query_keys[request_index]
                };
                let confidence = sample_confidence(&mut sensor_rng);
                let corrupted = sensor_rng.chance(conditional_corruption_rate(profile.corruption_rate, confidence));
                let mut value = truth[source_tick][observed_key];
                if corrupted { value = !value; }
                Evidence {
                    key: observed_key as u8,
                    value,
                    source_tick: source_tick as u8,
                    arrival_tick: tick as u8,
                    confidence,
                }
            });
            observations.push(candidates);
        }

        Self { truth, profile, observations }
    }

    fn target_answer(&self) -> bool {
        QUERY_KEYS.iter().filter(|key| self.truth[HORIZON - 1][**key]).count() >= 2
    }

    /// Policy-independent evaluator strata are computed from hidden truth,
    /// the episode profile and the entire pre-generated potential-observation
    /// tensor. They are emitted only after all policy actions; no stratum label
    /// or unchosen observation is passed to a PolicyState.
    fn evaluator_strata(&self, family: Family) -> [bool; STRATA_COUNT] {
        let mut latest: [Option<Evidence>; TOTAL_CLAIMS] = [None; TOTAL_CLAIMS];
        let mut max_source: [Option<u8>; TOTAL_CLAIMS] = [None; TOTAL_CLAIMS];
        let mut out_of_order = false;
        let mut long_delay = false;

        for evidence in self.observations.iter().flatten().copied() {
            let key = evidence.key();
            if !QUERY_KEYS.contains(&key) { continue; }
            if max_source[key].is_some_and(|previous| evidence.source_tick < previous) {
                out_of_order = true;
            }
            max_source[key] = Some(max_source[key]
                .map_or(evidence.source_tick, |previous| previous.max(evidence.source_tick)));
            long_delay |= usize::from(evidence.arrival_tick)
                .saturating_sub(usize::from(evidence.source_tick)) > 2;
            if latest[key].is_none_or(|previous| evidence.newer_than(previous)) {
                latest[key] = Some(evidence);
            }
        }

        // A world-defined recent-change label is independent of which
        // observation action a policy selected or successfully received.
        let mut recent_query_change = false;
        let first_tick = HORIZON.saturating_sub(8);
        for tick in first_tick..HORIZON {
            for key in QUERY_KEYS {
                if self.truth[tick][key] != self.truth[tick.saturating_sub(1)][key] {
                    recent_query_change = true;
                }
            }
        }
        let mut latest_low_confidence = false;
        for key in QUERY_KEYS {
            latest_low_confidence |= latest[key].is_none_or(|evidence| evidence.confidence < 900);
        }

        let (high_flip, high_corruption, high_distractor) = match family {
            Family::Development => (
                self.profile.flip_rate >= 20,
                self.profile.corruption_rate >= 10,
                self.profile.distractor_rate >= 250,
            ),
            Family::HeldOut => (
                self.profile.flip_rate >= 50,
                self.profile.corruption_rate >= 55,
                self.profile.distractor_rate >= 500,
            ),
        };
        [
            recent_query_change,
            out_of_order,
            latest_low_confidence,
            long_delay,
            high_flip,
            high_corruption,
            high_distractor,
        ]
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct HazardStats {
    last_reliable: Option<Evidence>,
    transition_count: u16,
    elapsed_ticks: u16,
}

impl HazardStats {
    fn observe(&mut self, evidence: Evidence) {
        if evidence.confidence < 900 { return; }
        match self.last_reliable {
            None => self.last_reliable = Some(evidence),
            Some(previous) if evidence.source_tick > previous.source_tick => {
                self.elapsed_ticks = self.elapsed_ticks.saturating_add(
                    evidence.source_tick.saturating_sub(previous.source_tick) as u16
                );
                if evidence.value != previous.value {
                    self.transition_count = self.transition_count.saturating_add(1);
                }
                self.last_reliable = Some(evidence);
            }
            // A late stale report never rewinds the per-claim history.
            Some(_) => {}
        }
    }

    fn rate(self) -> f64 {
        ((f64::from(self.transition_count) + 1.0)
            / (f64::from(self.elapsed_ticks) + 20.0))
            .clamp(0.0, 0.49)
    }
}

fn majority_decision(probabilities: [f64; QUERY_COUNT]) -> Option<bool> {
    let [p0, p1, p2] = probabilities;
    let p_majority_true =
        p0 * p1 * (1.0 - p2)
        + p0 * (1.0 - p1) * p2
        + (1.0 - p0) * p1 * p2
        + p0 * p1 * p2;
    if p_majority_true > 0.75 {
        Some(true)
    } else if p_majority_true < 0.25 {
        Some(false)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolicyKind {
    RoundRobin,
    UniformRandom,
    LeastObserved,
    StalestFirst,
    MaxUncertainty,
    ValueOfInformation,
}

impl PolicyKind {
    const ALL: [Self; 6] = [
        Self::RoundRobin,
        Self::UniformRandom,
        Self::LeastObserved,
        Self::StalestFirst,
        Self::MaxUncertainty,
        Self::ValueOfInformation,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::UniformRandom => "uniform_random",
            Self::LeastObserved => "least_observed",
            Self::StalestFirst => "stalest_first",
            Self::MaxUncertainty => "max_uncertainty",
            Self::ValueOfInformation => "value_of_information",
        }
    }
}

#[derive(Clone, Debug)]
struct PolicyState {
    kind: PolicyKind,
    requested_counts: [usize; QUERY_COUNT],
    relevant_counts: [usize; QUERY_COUNT],
    last: [Option<Evidence>; QUERY_COUNT],
    hazard: [HazardStats; QUERY_COUNT],
    action_rng: Rng,
}

impl PolicyState {
    fn new(kind: PolicyKind, seed: u64) -> Self {
        Self {
            kind,
            requested_counts: [0; QUERY_COUNT],
            relevant_counts: [0; QUERY_COUNT],
            last: [None; QUERY_COUNT],
            hazard: [HazardStats::default(); QUERY_COUNT],
            action_rng: Rng::new(seed),
        }
    }

    fn probabilities(&self, tick: usize) -> [f64; QUERY_COUNT] {
        std::array::from_fn(|index| {
            let Some(evidence) = self.last[index] else { return 0.5; };
            let age = tick.saturating_sub(usize::from(evidence.source_tick));
            let hazard = self.hazard[index].rate();
            let multiplier = match evidence.confidence {
                950 => 20.0,
                850 => 160.0,
                650 => 320.0,
                _ => 160.0,
            };
            let corruption = PRIOR_SENSOR_BASE_PER_THOUSAND * multiplier / 63.0 / 1_000.0;
            let reliability = (1.0 - corruption).clamp(0.5, 0.999);
            let persistence = (1.0 - 2.0 * hazard).powi(age as i32);
            let probability_same = 0.5 + (reliability - 0.5) * persistence;
            if evidence.value { probability_same } else { 1.0 - probability_same }
        })
    }

    fn tie_break(scores: [f64; QUERY_COUNT], tick: usize) -> usize {
        let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        for offset in 0..QUERY_COUNT {
            let index = (tick + offset) % QUERY_COUNT;
            if scores[index] >= maximum - 1e-12 {
                return index;
            }
        }
        0
    }

    fn choose_action(&mut self, tick: usize) -> usize {
        match self.kind {
            PolicyKind::RoundRobin => tick % QUERY_COUNT,
            PolicyKind::UniformRandom => self.action_rng.range(QUERY_COUNT),
            PolicyKind::LeastObserved => {
                let scores = std::array::from_fn(|index| -(self.requested_counts[index] as f64));
                Self::tie_break(scores, tick)
            }
            PolicyKind::StalestFirst => {
                let scores = std::array::from_fn(|index| match self.last[index] {
                    None => (HORIZON + tick + 1) as f64,
                    Some(evidence) => tick.saturating_sub(usize::from(evidence.source_tick)) as f64,
                });
                Self::tie_break(scores, tick)
            }
            PolicyKind::MaxUncertainty => {
                let probabilities = self.probabilities(tick);
                let scores = std::array::from_fn(|index| probabilities[index] * (1.0 - probabilities[index]));
                Self::tie_break(scores, tick)
            }
            PolicyKind::ValueOfInformation => {
                let probabilities = self.probabilities(tick);
                let scores = std::array::from_fn(|index| {
                    let others: Vec<f64> = (0..QUERY_COUNT).filter(|other| *other != index)
                        .map(|other| probabilities[other]).collect();
                    let pivotality = others[0] + others[1] - 2.0 * others[0] * others[1];
                    4.0 * probabilities[index] * (1.0 - probabilities[index]) * pivotality
                });
                Self::tie_break(scores, tick)
            }
        }
    }

    fn observe(&mut self, action: usize, evidence: Evidence) {
        self.requested_counts[action] += 1;
        let key = evidence.key();
        if let Some(index) = QUERY_KEYS.iter().position(|query_key| *query_key == key) {
            self.relevant_counts[index] += 1;
            if self.last[index].is_none_or(|previous| evidence.source_tick >= previous.source_tick) {
                self.last[index] = Some(evidence);
            }
            self.hazard[index].observe(evidence);
        }
    }

    fn final_decision(&self) -> Option<bool> {
        majority_decision(self.probabilities(HORIZON - 1))
    }

    fn logical_state_bytes(&self) -> usize {
        std::mem::size_of::<[Option<Evidence>; QUERY_COUNT]>()
            + std::mem::size_of::<[HazardStats; QUERY_COUNT]>()
            + std::mem::size_of::<[usize; QUERY_COUNT]>()
            + std::mem::size_of::<[usize; QUERY_COUNT]>()
    }
}

#[derive(Clone, Copy, Debug)]
struct Outcome {
    loss: f64,
    answered: bool,
    correct: bool,
    requested_counts: [usize; QUERY_COUNT],
    relevant_counts: [usize; QUERY_COUNT],
    wasted_fraction: f64,
    allocation_imbalance: usize,
    logical_state_bytes: usize,
}

#[derive(Clone, Debug)]
struct RunResult {
    outcome: Outcome,
    actions: Vec<usize>,
}

fn run_policy(episode: &ActiveEpisode, kind: PolicyKind, seed: u64) -> RunResult {
    let mut policy = PolicyState::new(kind, seed);
    let mut actions = Vec::with_capacity(HORIZON);

    for tick in 0..HORIZON {
        let action = policy.choose_action(tick);
        actions.push(action);
        // Only the requested candidate observation is revealed to the policy.
        let released = episode.observations[tick][action];
        policy.observe(action, released);
    }

    let decision = policy.final_decision();
    let correct = decision.is_some_and(|answer| answer == episode.target_answer());
    let loss = match decision {
        None => 0.25,
        Some(_) if correct => 0.0,
        Some(_) => 1.0,
    };
    let relevant = policy.relevant_counts.iter().sum::<usize>();
    let max_count = policy.requested_counts.iter().copied().max().unwrap_or(0);
    let min_count = policy.requested_counts.iter().copied().min().unwrap_or(0);

    RunResult {
        outcome: Outcome {
            loss,
            answered: decision.is_some(),
            correct,
            requested_counts: policy.requested_counts,
            relevant_counts: policy.relevant_counts,
            wasted_fraction: (HORIZON - relevant) as f64 / HORIZON as f64,
            allocation_imbalance: max_count - min_count,
            logical_state_bytes: policy.logical_state_bytes(),
        },
        actions,
    }
}

#[derive(Clone, Debug, Default)]
struct Accumulator {
    episodes: u64,
    loss_sum: f64,
    answered: u64,
    correct: u64,
    requested_counts_sum: [u64; QUERY_COUNT],
    relevant_counts_sum: [u64; QUERY_COUNT],
    wasted_sum: f64,
    imbalance_sum: u64,
    state_bytes_sum: u64,
    paired_deltas_vs_rr: Vec<f64>,
}

impl Accumulator {
    fn record(&mut self, outcome: Outcome, delta: Option<f64>) {
        self.episodes += 1;
        self.loss_sum += outcome.loss;
        self.answered += u64::from(outcome.answered);
        self.correct += u64::from(outcome.correct);
        for index in 0..QUERY_COUNT {
            self.requested_counts_sum[index] += outcome.requested_counts[index] as u64;
            self.relevant_counts_sum[index] += outcome.relevant_counts[index] as u64;
        }
        self.wasted_sum += outcome.wasted_fraction;
        self.imbalance_sum += outcome.allocation_imbalance as u64;
        self.state_bytes_sum += outcome.logical_state_bytes as u64;
        if let Some(delta) = delta { self.paired_deltas_vs_rr.push(delta); }
    }

    fn mean_loss(&self) -> f64 { self.loss_sum / self.episodes.max(1) as f64 }
    fn answer_rate(&self) -> f64 { self.answered as f64 / self.episodes.max(1) as f64 }
    fn accuracy(&self) -> String {
        if self.answered == 0 { "NA".to_string() }
        else { format!("{:.4}", self.correct as f64 / self.answered as f64) }
    }
    fn mean_requested_per_claim(&self) -> f64 {
        self.requested_counts_sum.iter().sum::<u64>() as f64
            / (self.episodes.max(1) as f64 * QUERY_COUNT as f64)
    }
    fn mean_relevant_per_claim(&self) -> f64 {
        self.relevant_counts_sum.iter().sum::<u64>() as f64
            / (self.episodes.max(1) as f64 * QUERY_COUNT as f64)
    }
    fn mean_wasted_fraction(&self) -> f64 { self.wasted_sum / self.episodes.max(1) as f64 }
    fn mean_imbalance(&self) -> f64 { self.imbalance_sum as f64 / self.episodes.max(1) as f64 }
    fn mean_state_bytes(&self) -> f64 { self.state_bytes_sum as f64 / self.episodes.max(1) as f64 }
}

fn bootstrap_ci(values: &[f64], seed: u64) -> (f64, f64) {
    if values.is_empty() { return (f64::NAN, f64::NAN); }
    let mut rng = Rng::new(seed);
    let mut means = Vec::with_capacity(BOOTSTRAPS);
    for _ in 0..BOOTSTRAPS {
        let mut sum = 0.0;
        for _ in 0..values.len() {
            sum += values[rng.range(values.len())];
        }
        means.push(sum / values.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    (means[BOOTSTRAPS * 25 / 1_000], means[BOOTSTRAPS * 975 / 1_000])
}

fn seed_for(rep: usize, pair: usize, family: Family) -> u64 {
    let domain = match family {
        Family::Development => 0x4445_565F_4143_5155,
        Family::HeldOut => 0x4845_4C44_4143_5155,
    };
    mix64(ROOT_SEED
        ^ domain
        ^ (rep as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (pair as u64).wrapping_mul(0xA24B_AED4_963E_E407))
}

fn report() {
    println!("VIS-ACT-004 active evidence acquisition benchmark");
    println!("Fixed observation budget: {HORIZON} requests per episode; 3 public query claims; 5 distractor claims");
    println!("Potential observations are pre-generated by (episode,tick,requested-key). Each policy sees only the result for its chosen action.");
    println!("Policies: round_robin, uniform_random, least_observed, stalest_first, max_uncertainty, value_of_information");
    println!("Confidence is sampled independently; only the evaluator has the latent truth and true nuisance profile.");
    println!("Loss: correct=0, abstain=0.25, incorrect=1. Deltas are paired episode-loss differences vs round-robin.");
    println!("family\tpolicy\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tmean_requested_per_claim\tmean_relevant_per_claim\twasted_observation_rate\tallocation_imbalance\tlogical_state_bytes\tdelta_loss_vs_round_robin\tbootstrap95_lo\tbootstrap95_hi");

    let mut cells = vec![Accumulator::default(); Family::ALL.len() * PolicyKind::ALL.len()];
    let mut strata_cells = vec![
        Accumulator::default();
        Family::ALL.len() * STRATA_COUNT * 2 * PolicyKind::ALL.len()
    ];

    for family in Family::ALL {
        for replicate in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(replicate, pair, family);
                let episode = ActiveEpisode::generate(seed, family);
                // Shared evaluator labels: every policy in this pair is placed
                // in the same strata, independent of its chosen observations.
                let episode_strata = episode.evaluator_strata(family);
                let mut round_robin_loss = 0.0;

                for (policy_index, policy) in PolicyKind::ALL.iter().copied().enumerate() {
                    let actor_seed = mix64(seed
                        ^ (policy_index as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03));
                    let run = run_policy(&episode, policy, actor_seed);
                    let index = (family as usize) * PolicyKind::ALL.len() + policy_index;
                    if policy_index == 0 {
                        round_robin_loss = run.outcome.loss;
                        cells[index].record(run.outcome, None);
                    } else {
                        cells[index].record(run.outcome, Some(run.outcome.loss - round_robin_loss));
                    }

                    for (stratum_index, active) in episode_strata.iter().copied().enumerate() {
                        let level = usize::from(active);
                        let index = (((family as usize * STRATA_COUNT + stratum_index) * 2 + level)
                            * PolicyKind::ALL.len()) + policy_index;
                        if policy_index == 0 {
                            strata_cells[index].record(run.outcome, None);
                        } else {
                            strata_cells[index].record(run.outcome, Some(run.outcome.loss - round_robin_loss));
                        }
                    }
                }
            }
        }
    }

    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        for (policy_index, policy) in PolicyKind::ALL.iter().copied().enumerate() {
            let cell = &cells[family_index * PolicyKind::ALL.len() + policy_index];
            let (delta, lo, hi) = if policy_index == 0 {
                (0.0, 0.0, 0.0)
            } else {
                let delta = cell.paired_deltas_vs_rr.iter().sum::<f64>()
                    / cell.paired_deltas_vs_rr.len().max(1) as f64;
                let seed = mix64(ROOT_SEED
                    ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                    ^ (policy_index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
                let (lo, hi) = bootstrap_ci(&cell.paired_deltas_vs_rr, seed);
                (delta, lo, hi)
            };
            println!("{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.3}\t{:.3}\t{:.4}\t{:.3}\t{:.1}\t{:.4}\t{:.4}\t{:.4}",
                family.name(), policy.name(), cell.episodes, cell.mean_loss(), cell.answer_rate(),
                cell.accuracy(), cell.mean_requested_per_claim(), cell.mean_relevant_per_claim(),
                cell.mean_wasted_fraction(), cell.mean_imbalance(), cell.mean_state_bytes(),
                delta, lo, hi);
        }
    }

    println!("STRATUM\tfamily\tstratum\tlevel\tpolicy\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tdelta_loss_vs_round_robin\tbootstrap95_lo\tbootstrap95_hi");
    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        for (stratum_index, stratum_name) in STRATA_NAMES.iter().copied().enumerate() {
            for level in 0..=1 {
                for (policy_index, policy) in PolicyKind::ALL.iter().copied().enumerate() {
                    let index = (((family_index * STRATA_COUNT + stratum_index) * 2 + level)
                        * PolicyKind::ALL.len()) + policy_index;
                    let cell = &strata_cells[index];
                    if cell.episodes == 0 { continue; }
                    let (delta, lo, hi) = if policy_index == 0 {
                        (0.0, 0.0, 0.0)
                    } else {
                        let delta = cell.paired_deltas_vs_rr.iter().sum::<f64>()
                            / cell.paired_deltas_vs_rr.len().max(1) as f64;
                        let seed = mix64(ROOT_SEED
                            ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                            ^ (stratum_index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
                            ^ (level as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
                            ^ (policy_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                        let (lo, hi) = bootstrap_ci(&cell.paired_deltas_vs_rr, seed);
                        (delta, lo, hi)
                    };
                    println!("STRATUM\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.4}\t{:.4}\t{:.4}",
                        family.name(), stratum_name, level == 1, policy.name(), cell.episodes,
                        cell.mean_loss(), cell.answer_rate(), cell.accuracy(), delta, lo, hi);
                }
            }
        }
    }
}

impl Family {
    const ALL: [Self; 2] = [Self::Development, Self::HeldOut];
}

fn main() { report(); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_replays_active_world_and_policy_actions() {
        let a = ActiveEpisode::generate(0xCAFE_BABE, Family::HeldOut);
        let b = ActiveEpisode::generate(0xCAFE_BABE, Family::HeldOut);
        assert_eq!(a, b);
        for policy in PolicyKind::ALL {
            let first = run_policy(&a, policy, 1234);
            let second = run_policy(&b, policy, 1234);
            assert_eq!(first.actions, second.actions);
            assert_eq!(first.outcome.loss, second.outcome.loss);
        }
    }

    #[test]
    fn every_policy_respects_the_same_observation_budget() {
        let episode = ActiveEpisode::generate(0xB0D6_E700, Family::HeldOut);
        for policy in PolicyKind::ALL {
            let result = run_policy(&episode, policy, 99);
            assert_eq!(result.actions.len(), HORIZON);
            assert_eq!(result.outcome.requested_counts.iter().sum::<usize>(), HORIZON);
            assert_eq!(result.outcome.relevant_counts.iter().sum::<usize>() + 
                (result.outcome.wasted_fraction * HORIZON as f64).round() as usize, HORIZON);
        }
    }

    #[test]
    fn hidden_truth_relabel_does_not_change_policy_actions() {
        let episode = ActiveEpisode::generate(0xABCD_1234, Family::HeldOut);
        let mut relabeled = episode.clone();
        for tick in 0..HORIZON {
            relabeled.truth[tick][QUERY_KEYS[0]] = !relabeled.truth[tick][QUERY_KEYS[0]];
        }
        for policy in PolicyKind::ALL {
            let original = run_policy(&episode, policy, 0x1234);
            let altered = run_policy(&relabeled, policy, 0x1234);
            assert_eq!(original.actions, altered.actions);
        }
    }

    #[test]
    fn evaluator_profile_relabel_does_not_change_actions_or_released_data() {
        let episode = ActiveEpisode::generate(0xA11C_E001, Family::HeldOut);
        let mut relabeled = episode.clone();
        relabeled.profile.flip_rate = 0;
        relabeled.profile.corruption_rate = 0;
        relabeled.profile.distractor_rate = 0;
        for policy in PolicyKind::ALL {
            let original = run_policy(&episode, policy, 77);
            let altered = run_policy(&relabeled, policy, 77);
            assert_eq!(original.actions, altered.actions);
            assert_eq!(original.outcome.loss, altered.outcome.loss);
        }
    }

    #[test]
    fn evaluator_strata_detect_claim_changes_delay_and_profile_stress() {
        let mut episode = ActiveEpisode::generate(0xA11C_E003, Family::HeldOut);
        // Force evaluator-only profile labels high to cover those stratum branches.
        // The observation tensor is deliberately left unchanged, as in a leakage test.
        episode.profile = Profile {
            distractor_rate: 650,
            flip_rate: 70,
            max_delay: 5,
            corruption_rate: 70,
        };
        // Controlled recent world change, independent of selected observation actions.
        for tick in HORIZON - 4..HORIZON {
            episode.truth[tick][QUERY_KEYS[0]] = !episode.truth[tick][QUERY_KEYS[0]];
        }
        let released = vec![
            Evidence { key: QUERY_KEYS[0] as u8, value: false, source_tick: 5, arrival_tick: 5, confidence: 650 },
            Evidence { key: QUERY_KEYS[0] as u8, value: true, source_tick: 2, arrival_tick: 12, confidence: 650 },
            Evidence { key: QUERY_KEYS[1] as u8, value: true, source_tick: 3, arrival_tick: 12, confidence: 850 },
        ];
        let strata = episode.evaluator_strata(Family::HeldOut);
        assert!(strata[0]); // truth changed after latest source evidence
        assert!(strata[1]); // later-arriving old source timestamp
        assert!(strata[2]); // latest source-time confidence < 900
        assert!(strata[3]); // at least one delay > 2
        assert!(strata[4]); // high held-out claim-flip profile
        assert!(strata[5]); // high held-out corruption profile
        assert!(strata[6]); // high held-out distractor profile
    }

    #[test]
    fn round_robin_has_balanced_requested_action_counts() {
        let episode = ActiveEpisode::generate(0xBABA_0404, Family::Development);
        let result = run_policy(&episode, PolicyKind::RoundRobin, 100);
        let max_count = result.outcome.requested_counts.iter().copied().max().unwrap();
        let min_count = result.outcome.requested_counts.iter().copied().min().unwrap();
        assert!(max_count - min_count <= 1);
    }

    #[test]
    fn stale_source_observation_does_not_rewind_claim_memory() {
        let mut state = PolicyState::new(PolicyKind::StalestFirst, 1);
        state.observe(0, Evidence { key: 0, value: true, source_tick: 8, arrival_tick: 8, confidence: 950 });
        state.observe(0, Evidence { key: 0, value: false, source_tick: 3, arrival_tick: 9, confidence: 950 });
        assert_eq!(state.last[0].unwrap().source_tick, 8);
    }

    #[test]
    fn value_of_information_score_is_bounded_and_deterministic() {
        let episode = ActiveEpisode::generate(0x0F10_1234, Family::Development);
        let mut state = PolicyState::new(PolicyKind::ValueOfInformation, 77);
        let a = state.choose_action(0);
        let b = state.choose_action(1);
        assert!(a < QUERY_COUNT && b < QUERY_COUNT);
        assert_ne!(a, b);
        assert_eq!(episode.observations.len(), HORIZON);
    }

    #[test]
    fn bootstrap_intervals_replay_deterministically() {
        let values = [-0.2, -0.1, 0.0, 0.1, 0.05, -0.15];
        assert_eq!(bootstrap_ci(&values, 42), bootstrap_ci(&values, 42));
        assert!(bootstrap_ci(&values, 42).0 <= bootstrap_ci(&values, 42).1);
    }
}
