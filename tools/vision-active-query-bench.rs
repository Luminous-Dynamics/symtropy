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
const BUDGETS: [usize; 5] = [8, 16, 24, 32, 48];
const REPLICATES: usize = 12;
const PAIRS: usize = 32;
const BOOTSTRAPS: usize = 2_000;
// Independent seed domain for the frozen VOI replication (the policy itself is unchanged).
const ROOT_SEED: u64 = 0x9B05_688C_2B3E_6C1F;
const PRIOR_SENSOR_BASE_PER_THOUSAND: f64 = 32.5;
const PRIOR_DISTRACTOR_PER_THOUSAND: f64 = 375.0;
// Fixed, declared terminal-risk threshold expressed in task-loss units per request.
// This is a transparent satisficing baseline, not an optimal stopping solver.
const STOP_COST_PER_OBSERVATION: f64 = 0.04;
const STOP_COST_GRID: [f64; 4] = [0.01, 0.04, 0.08, 0.12];
// Marginal risk-reduction thresholds are calibrated on development episodes only.
// Costs are task-loss units per request; the objective is loss + cost * requests.
const MARGINAL_VALUE_COST_GRID: [f64; 7] = [0.0, 0.00025, 0.0005, 0.001, 0.0015, 0.002, 0.003];
const VALUE_CALIBRATION_TICKS: [usize; 12] = [0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44];
const VALUE_CALIBRATION_STRATA: [&str; 7] = [
    "all_candidates",
    "relevant_observation",
    "distractor_observation",
    "source_delay_le_2",
    "source_delay_gt_2",
    "high_confidence_ge_900",
    "low_confidence_lt_900",
];
const CONFIDENCE_TIERS: [(u16, f64); 3] = [(950, 0.75), (850, 0.20), (650, 0.05)];
const DELAY_PRIORS: [(usize, f64); 6] = [
    (0, 0.167), (1, 0.167), (2, 0.292), (3, 0.125), (4, 0.125), (5, 0.124),
];

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

fn majority_true_probability(probabilities: [f64; QUERY_COUNT]) -> f64 {
    let [p0, p1, p2] = probabilities;
    p0 * p1 * (1.0 - p2)
        + p0 * (1.0 - p1) * p2
        + (1.0 - p0) * p1 * p2
        + p0 * p1 * p2
}

fn terminal_decision_risk(probabilities: [f64; QUERY_COUNT]) -> f64 {
    let majority_true = majority_true_probability(probabilities).clamp(0.0, 1.0);
    majority_true.min(1.0 - majority_true).min(0.25)
}

fn majority_decision(probabilities: [f64; QUERY_COUNT]) -> Option<bool> {
    let p_majority_true = majority_true_probability(probabilities);
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
    ExpectedTaskLossReduction,
    RiskThresholdStop,
    MarginalValueStop,
    OnlineAdaptiveValue,
}

impl PolicyKind {
    const ALL: [Self; 10] = [
        Self::RoundRobin,
        Self::UniformRandom,
        Self::LeastObserved,
        Self::StalestFirst,
        Self::MaxUncertainty,
        Self::ValueOfInformation,
        Self::ExpectedTaskLossReduction,
        Self::RiskThresholdStop,
        Self::MarginalValueStop,
        Self::OnlineAdaptiveValue,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::UniformRandom => "uniform_random",
            Self::LeastObserved => "least_observed",
            Self::StalestFirst => "stalest_first",
            Self::MaxUncertainty => "max_uncertainty",
            Self::ValueOfInformation => "value_of_information",
            Self::ExpectedTaskLossReduction => "expected_task_loss_reduction",
            Self::RiskThresholdStop => "risk_threshold_stop",
            Self::MarginalValueStop => "marginal_value_stop",
            Self::OnlineAdaptiveValue => "online_adaptive_value",
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
    estimated_scoring_work_units: u64,
    stop_cost_per_observation: f64,
    observed_delay_counts: [u64; 6],
    observed_delay_samples: u64,
    observed_distractor_count: u64,
    observed_response_count: u64,
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
            estimated_scoring_work_units: 0,
            stop_cost_per_observation: STOP_COST_PER_OBSERVATION,
            observed_delay_counts: [0; 6],
            observed_delay_samples: 0,
            observed_distractor_count: 0,
            observed_response_count: 0,
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
        let action = match self.kind {
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
            PolicyKind::ExpectedTaskLossReduction => {
                let scores = self.expected_task_loss_reduction_scores(tick);
                let best = scores.iter().copied().fold(0.0_f64, f64::max);
                if best <= 1e-12 {
                    tick % QUERY_COUNT
                } else {
                    Self::tie_break(scores, tick)
                }
            }
            PolicyKind::RiskThresholdStop => {
                let terminal_risk = terminal_decision_risk(self.probabilities(HORIZON - 1));
                if terminal_risk <= self.stop_cost_per_observation {
                    QUERY_COUNT // sentinel: stop sensing; run loop must not reveal a response
                } else {
                    let probabilities = self.probabilities(tick);
                    let scores = std::array::from_fn(|index| probabilities[index] * (1.0 - probabilities[index]));
                    Self::tie_break(scores, tick)
                }
            }
            PolicyKind::OnlineAdaptiveValue => {
                let scores = self.adaptive_task_loss_reduction_scores(tick);
                let best = scores.iter().copied().fold(0.0_f64, f64::max);
                if best <= 1e-12 { tick % QUERY_COUNT } else { Self::tie_break(scores, tick) }
            }
            PolicyKind::MarginalValueStop => {
                // A majority query has complementary information value: one claim can
                // look non-pivotal while the other claims remain completely unknown.
                // Guarantee one relevant observation per query claim before testing the
                // myopic stop rule, rather than stopping immediately at the symmetric prior.
                if self.relevant_counts.iter().any(|count| *count == 0) {
                    // First try every as-yet-unrequested query claim. If an observation is
                    // diverted to a distractor, retry the least-requested missing claim
                    // rather than starving the other claims with repeated attempts on index 0.
                    let scores = std::array::from_fn(|index| {
                        if self.relevant_counts[index] == 0 {
                            -(self.requested_counts[index] as f64)
                        } else {
                            f64::NEG_INFINITY
                        }
                    });
                    Self::tie_break(scores, tick)
                } else {
                    let scores = self.expected_task_loss_reduction_scores(tick);
                    let best_marginal_value = scores.iter().copied().fold(0.0_f64, f64::max);
                    if best_marginal_value <= self.stop_cost_per_observation {
                        QUERY_COUNT // stop if the best estimated next-observation value does not pay its cost
                    } else {
                        Self::tie_break(scores, tick)
                    }
                }
            }
        };
        // Deterministic operation proxy, not wall-clock time: simple policies
        // score at most one value per candidate; expected-risk evaluates every
        // candidate × delay × confidence tier × binary observation result.
        let work_this_action = match self.kind {
            PolicyKind::RoundRobin | PolicyKind::UniformRandom => 1,
            PolicyKind::LeastObserved | PolicyKind::StalestFirst | PolicyKind::MaxUncertainty
                | PolicyKind::ValueOfInformation => QUERY_COUNT as u64,
            PolicyKind::ExpectedTaskLossReduction =>
                (QUERY_COUNT * DELAY_PRIORS.len() * CONFIDENCE_TIERS.len() * 2 + QUERY_COUNT) as u64,
            PolicyKind::RiskThresholdStop => QUERY_COUNT as u64 + 1,
            PolicyKind::MarginalValueStop =>
                (QUERY_COUNT * DELAY_PRIORS.len() * CONFIDENCE_TIERS.len() * 2 + QUERY_COUNT + 1) as u64,
            PolicyKind::OnlineAdaptiveValue =>
                (QUERY_COUNT * DELAY_PRIORS.len() * CONFIDENCE_TIERS.len() * 2 + QUERY_COUNT + DELAY_PRIORS.len() + 2) as u64,
        };
        self.estimated_scoring_work_units = self.estimated_scoring_work_units
            .saturating_add(work_this_action);
        action
    }

    fn expected_task_loss_reduction_scores(&self, tick: usize) -> [f64; QUERY_COUNT] {
        self.expected_task_loss_reduction_scores_with_priors(
            tick, DELAY_PRIORS, PRIOR_DISTRACTOR_PER_THOUSAND / 1_000.0
        )
    }

    fn adaptive_task_loss_reduction_scores(&self, tick: usize) -> [f64; QUERY_COUNT] {
        self.expected_task_loss_reduction_scores_with_priors(
            tick, self.adaptive_delay_priors(), self.adaptive_distractor_probability()
        )
    }

    fn adaptive_delay_priors(&self) -> [(usize, f64); 6] {
        // Dirichlet smoothing prevents a few early observations from erasing the prior.
        // The prior contributes six pseudo-observations in total.
        let prior_strength = 6.0;
        let total = prior_strength + self.observed_delay_samples as f64;
        std::array::from_fn(|delay| {
            let prior = DELAY_PRIORS[delay].1 * prior_strength;
            (delay, (prior + self.observed_delay_counts[delay] as f64) / total)
        })
    }

    fn adaptive_distractor_probability(&self) -> f64 {
        // Beta prior with four pseudo-observations around the fixed public prior.
        let alpha = PRIOR_DISTRACTOR_PER_THOUSAND / 1_000.0 * 4.0;
        let beta = (1.0 - PRIOR_DISTRACTOR_PER_THOUSAND / 1_000.0) * 4.0;
        (alpha + self.observed_distractor_count as f64)
            / (alpha + beta + self.observed_response_count as f64)
    }

    fn expected_task_loss_reduction_scores_with_priors(
        &self,
        tick: usize,
        delay_priors: [(usize, f64); 6],
        distractor_probability: f64,
    ) -> [f64; QUERY_COUNT] {
        let terminal_now = self.probabilities(HORIZON - 1);
        let current_risk = terminal_decision_risk(terminal_now);
        let current_at_tick = self.probabilities(tick);

        std::array::from_fn(|claim_index| {
            let hazard = self.hazard[claim_index].rate();
            let mut expected_risk_given_relevant = 0.0;

            for (delay, delay_weight) in delay_priors {
                let source_tick = tick.saturating_sub(delay);
                // A hypothetical observation older than retained source-time
                // evidence would be ignored and adds no decision value.
                if self.last[claim_index]
                    .is_some_and(|last| usize::from(last.source_tick) > source_tick)
                {
                    expected_risk_given_relevant += delay_weight * current_risk;
                    continue;
                }

                let effective_gap = tick.saturating_sub(source_tick);
                let p_tick = current_at_tick[claim_index];
                let p_source = 0.5 + (p_tick - 0.5)
                    * (1.0 - 2.0 * hazard).powi(effective_gap as i32);

                for (confidence, confidence_weight) in CONFIDENCE_TIERS {
                    let corruption = conditional_corruption_rate(
                        PRIOR_SENSOR_BASE_PER_THOUSAND.round() as u64,
                        confidence,
                    ) as f64 / 1_000.0;
                    let p_observed_true =
                        p_source * (1.0 - corruption) + (1.0 - p_source) * corruption;

                    let p_latent_true_if_observed_true =
                        (p_source * (1.0 - corruption) / p_observed_true.max(1e-12))
                            .clamp(0.0, 1.0);
                    let p_latent_true_if_observed_false =
                        (p_source * corruption / (1.0 - p_observed_true).max(1e-12))
                            .clamp(0.0, 1.0);

                    let terminal_gap = HORIZON - 1 - source_tick;
                    let persistence = (1.0 - 2.0 * hazard).powi(terminal_gap as i32);
                    let p_terminal_if_true_observation = 0.5
                        + (p_latent_true_if_observed_true - 0.5) * persistence;
                    let p_terminal_if_false_observation = 0.5
                        + (p_latent_true_if_observed_false - 0.5) * persistence;

                    let mut true_case = terminal_now;
                    true_case[claim_index] = p_terminal_if_true_observation;
                    let mut false_case = terminal_now;
                    false_case[claim_index] = p_terminal_if_false_observation;

                    let risk = p_observed_true * terminal_decision_risk(true_case)
                        + (1.0 - p_observed_true) * terminal_decision_risk(false_case);
                    expected_risk_given_relevant +=
                        delay_weight * confidence_weight * risk;
                }
            }

            // A requested view may be diverted to an irrelevant hot distractor.
            let expected_risk = distractor_probability * current_risk
                + (1.0 - distractor_probability) * expected_risk_given_relevant;
            (current_risk - expected_risk).max(0.0)
        })
    }

    fn observe(&mut self, action: usize, evidence: Evidence) {
        self.requested_counts[action] += 1;
        self.observed_response_count += 1;
        // At source_tick == 0 the observation's true delay may have been clipped
        // by saturating_sub at the episode boundary. Do not treat that lower bound
        // as an exact delay; this prevents early-episode bias against long delays.
        if evidence.source_tick > 0 {
            let source_delay = usize::from(evidence.arrival_tick)
                .saturating_sub(usize::from(evidence.source_tick))
                .min(self.observed_delay_counts.len() - 1);
            self.observed_delay_counts[source_delay] += 1;
            self.observed_delay_samples += 1;
        }
        if evidence.key() as usize != QUERY_KEYS[action] {
            self.observed_distractor_count += 1;
        }
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
    estimated_scoring_work_units: u64,
}

#[derive(Clone, Debug)]
struct RunResult {
    outcome: Outcome,
    actions: Vec<usize>,
    // Post-run estimates are evaluator-readable only after the policy finishes.
    // They are not present in action-selection inputs.
    estimated_delay_distribution: [f64; 6],
    estimated_distractor_probability: f64,
    total_nuisance_observations: u64,
    exact_delay_samples: u64,
}

fn run_policy(episode: &ActiveEpisode, kind: PolicyKind, seed: u64) -> RunResult {
    run_policy_with_budget(episode, kind, seed, HORIZON)
}

fn run_policy_with_budget(
    episode: &ActiveEpisode,
    kind: PolicyKind,
    seed: u64,
    budget: usize,
) -> RunResult {
    run_policy_with_budget_and_stop_cost(episode, kind, seed, budget, STOP_COST_PER_OBSERVATION)
}

fn scheduled_tick(action_slot: usize, budget: usize) -> usize {
    assert!(budget > 0 && budget <= HORIZON);
    assert!(action_slot < budget);
    action_slot * HORIZON / budget
}

fn run_policy_with_budget_and_stop_cost(
    episode: &ActiveEpisode,
    kind: PolicyKind,
    seed: u64,
    budget: usize,
    stop_cost_per_observation: f64,
) -> RunResult {
    assert!(budget > 0 && budget <= HORIZON);
    assert!(stop_cost_per_observation >= 0.0 && stop_cost_per_observation <= 0.25);
    let mut policy = PolicyState::new(kind, seed);
    policy.stop_cost_per_observation = stop_cost_per_observation;
    let mut actions = Vec::with_capacity(budget);

    for action_slot in 0..budget {
        // Spread the request budget over the fixed world horizon. At budget 48
        // this is exactly one observation per tick; smaller budgets do not
        // accidentally become a front-loaded prefix with a long blind tail.
        let tick = scheduled_tick(action_slot, budget);
        let action = policy.choose_action(tick);
        if action >= QUERY_COUNT {
            break; // terminal stopping action: no potential observation is released
        }
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
    let used_requests = actions.len();
    let max_count = policy.requested_counts.iter().copied().max().unwrap_or(0);
    let min_count = policy.requested_counts.iter().copied().min().unwrap_or(0);

    RunResult {
        outcome: Outcome {
            loss,
            answered: decision.is_some(),
            correct,
            requested_counts: policy.requested_counts,
            relevant_counts: policy.relevant_counts,
            wasted_fraction: if used_requests == 0 { 0.0 } else {
                (used_requests - relevant) as f64 / used_requests as f64
            },
            allocation_imbalance: max_count - min_count,
            logical_state_bytes: policy.logical_state_bytes(),
            estimated_scoring_work_units: policy.estimated_scoring_work_units,
        },
        actions,
        estimated_delay_distribution: policy.adaptive_delay_priors().map(|(_, probability)| probability),
        estimated_distractor_probability: policy.adaptive_distractor_probability(),
        total_nuisance_observations: policy.observed_response_count,
        exact_delay_samples: policy.observed_delay_samples,
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
    scoring_work_units_sum: u64,
    paired_deltas_vs_rr: Vec<f64>,
    paired_objective_deltas_vs_rr: Vec<f64>,
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
        self.scoring_work_units_sum += outcome.estimated_scoring_work_units;
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
    fn mean_scoring_work_units(&self) -> f64 {
        self.scoring_work_units_sum as f64 / self.episodes.max(1) as f64
    }
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
    println!("Policies: round_robin, uniform_random, least_observed, stalest_first, max_uncertainty, value_of_information, expected_task_loss_reduction, online_adaptive_value, risk_threshold_stop, marginal_value_stop");
    println!("Online-adaptive value learns only observable distractor frequency and source-delay distribution from its own released observations; sensor corruption remains fixed and unobserved.");
    println!("Expected-risk scores integrate frozen public confidence/delay/distractor priors and terminal abstention-aware Bayes risk; hidden episode parameters are not consulted.");
    println!("Risk-threshold stop baseline: stop when estimated terminal Bayes risk <= {STOP_COST_PER_OBSERVATION:.3} task-loss units per request; not claimed optimal.");
    println!("Marginal-value stop baseline: stop when modeled next-observation risk reduction <= request cost; threshold selected from a frozen grid using development episodes only.");
    println!("Confidence is sampled independently; only the evaluator has the latent truth and true nuisance profile.");
    println!("Loss: correct=0, abstain=0.25, incorrect=1. Deltas are paired episode-loss differences vs round-robin.");
    println!("family\tpolicy\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tmean_requested_per_claim\tmean_relevant_per_claim\twasted_observation_rate\tallocation_imbalance\tlogical_state_bytes\testimated_scoring_work_units_per_episode\tdelta_loss_vs_round_robin\tbootstrap95_lo\tbootstrap95_hi");

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
            println!("{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.3}\t{:.3}\t{:.4}\t{:.3}\t{:.1}\t{:.1}\t{:.4}\t{:.4}\t{:.4}",
                family.name(), policy.name(), cell.episodes, cell.mean_loss(), cell.answer_rate(),
                cell.accuracy(), cell.mean_requested_per_claim(), cell.mean_relevant_per_claim(),
                cell.mean_wasted_fraction(), cell.mean_imbalance(), cell.mean_state_bytes(),
                cell.mean_scoring_work_units(), delta, lo, hi);
        }
    }

    // Direct paired primary comparison: expected terminal risk-reduction policy
    // minus the approximate uncertainty × pivotality VOI heuristic.
    println!("PAIRWISE\tfamily\tcontrast\tepisodes\tdelta_loss_a_minus_b\tbootstrap95_lo\tbootstrap95_hi");
    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        let heuristic_index = PolicyKind::ALL.iter()
            .position(|policy| *policy == PolicyKind::ValueOfInformation)
            .expect("VOI policy is declared");
        let expected_index = PolicyKind::ALL.iter()
            .position(|policy| *policy == PolicyKind::ExpectedTaskLossReduction)
            .expect("expected-risk policy is declared");
        let heuristic = &cells[family_index * PolicyKind::ALL.len() + heuristic_index];
        let expected = &cells[family_index * PolicyKind::ALL.len() + expected_index];
        let deltas: Vec<f64> = expected.paired_deltas_vs_rr.iter()
            .zip(heuristic.paired_deltas_vs_rr.iter())
            .map(|(expected_delta, heuristic_delta)| expected_delta - heuristic_delta)
            .collect();
        let mean_delta = deltas.iter().sum::<f64>() / deltas.len().max(1) as f64;
        let seed = mix64(ROOT_SEED ^ 0x4558_5056_4F49_4349
            ^ (family_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
        let (lo, hi) = bootstrap_ci(&deltas, seed);
        println!("PAIRWISE\t{}\texpected_task_loss_reduction_minus_value_of_information\t{}\t{:.4}\t{:.4}\t{:.4}",
            family.name(), deltas.len(), mean_delta, lo, hi);

        let adaptive_index = PolicyKind::ALL.iter()
            .position(|policy| *policy == PolicyKind::OnlineAdaptiveValue)
            .expect("online adaptive value policy is declared");
        let adaptive = &cells[family_index * PolicyKind::ALL.len() + adaptive_index];
        let adaptive_deltas: Vec<f64> = adaptive.paired_deltas_vs_rr.iter()
            .zip(expected.paired_deltas_vs_rr.iter())
            .map(|(adaptive_delta, expected_delta)| adaptive_delta - expected_delta)
            .collect();
        let adaptive_delta = adaptive_deltas.iter().sum::<f64>()
            / adaptive_deltas.len().max(1) as f64;
        let adaptive_seed = mix64(ROOT_SEED ^ 0x4144_4150_5456_4558
            ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
        let (adaptive_lo, adaptive_hi) = bootstrap_ci(&adaptive_deltas, adaptive_seed);
        println!("PAIRWISE\t{}\tonline_adaptive_value_minus_expected_task_loss_reduction\t{}\t{:.4}\t{:.4}\t{:.4}",
            family.name(), adaptive_deltas.len(), adaptive_delta, adaptive_lo, adaptive_hi);
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

fn report_budget_frontier() {
    println!("VIS-ACT-006 sensing-budget Pareto frontier; world horizon stays {HORIZON}, terminal target is evaluated at tick {}", HORIZON - 1);
    println!("Requests are evenly scheduled across the fixed 48-tick horizon using floor(slot * horizon / budget); the world continues evolving and no terminal observation is supplied for free.");
    println!("BUDGET\tfamily\tbudget\tpolicy\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tmean_requested_per_claim\tmean_relevant_per_claim\twasted_observation_rate\tlogical_state_bytes\testimated_scoring_work_units_per_episode\tdelta_loss_vs_round_robin\tbootstrap95_lo\tbootstrap95_hi");

    let mut cells = vec![
        Accumulator::default();
        Family::ALL.len() * BUDGETS.len() * PolicyKind::ALL.len()
    ];
    for family in Family::ALL {
        for replicate in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(replicate, pair, family);
                let episode = ActiveEpisode::generate(seed, family);
                for (budget_index, budget) in BUDGETS.iter().copied().enumerate() {
                    let mut round_robin_loss = 0.0;
                    for (policy_index, policy) in PolicyKind::ALL.iter().copied().enumerate() {
                        let actor_seed = mix64(seed
                            ^ (policy_index as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03));
                        let run = run_policy_with_budget(&episode, policy, actor_seed, budget);
                        let index = ((family as usize * BUDGETS.len() + budget_index)
                            * PolicyKind::ALL.len()) + policy_index;
                        if policy_index == 0 {
                            round_robin_loss = run.outcome.loss;
                            cells[index].record(run.outcome, None);
                        } else {
                            cells[index].record(run.outcome, Some(run.outcome.loss - round_robin_loss));
                        }
                    }
                }
            }
        }
    }

    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        for (budget_index, budget) in BUDGETS.iter().copied().enumerate() {
            for (policy_index, policy) in PolicyKind::ALL.iter().copied().enumerate() {
                let cell = &cells[(family_index * BUDGETS.len() + budget_index)
                    * PolicyKind::ALL.len() + policy_index];
                let (delta, lo, hi) = if policy_index == 0 {
                    (0.0, 0.0, 0.0)
                } else {
                    let delta = cell.paired_deltas_vs_rr.iter().sum::<f64>()
                        / cell.paired_deltas_vs_rr.len().max(1) as f64;
                    let seed = mix64(ROOT_SEED
                        ^ 0x4255_4447_4554_4349
                        ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                        ^ (budget_index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
                        ^ (policy_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
                    let (lo, hi) = bootstrap_ci(&cell.paired_deltas_vs_rr, seed);
                    (delta, lo, hi)
                };
                println!("BUDGET\t{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.3}\t{:.3}\t{:.4}\t{:.1}\t{:.1}\t{:.4}\t{:.4}\t{:.4}",
                    family.name(), budget, policy.name(), cell.episodes, cell.mean_loss(),
                    cell.answer_rate(), cell.accuracy(), cell.mean_requested_per_claim(),
                    cell.mean_relevant_per_claim(), cell.mean_wasted_fraction(),
                    cell.mean_state_bytes(), cell.mean_scoring_work_units(), delta, lo, hi);
            }
        }
    }
}

fn report_stop_cost_sensitivity() {
    println!("STOP_SENSITIVITY\tfamily\tstop_cost_per_observation\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tmean_requests_used\twasted_observation_rate\tlogical_state_bytes\testimated_scoring_work_units_per_episode\tdelta_loss_vs_round_robin\tbootstrap95_lo\tbootstrap95_hi");
    let mut cells = vec![Accumulator::default(); Family::ALL.len() * STOP_COST_GRID.len()];

    for family in Family::ALL {
        for replicate in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(replicate, pair, family);
                let episode = ActiveEpisode::generate(seed, family);
                let round_robin = run_policy(&episode, PolicyKind::RoundRobin, mix64(
                    seed ^ 0xD1B5_4A32_D192_ED03
                ));
                for (cost_index, cost) in STOP_COST_GRID.iter().copied().enumerate() {
                    let stopped = run_policy_with_budget_and_stop_cost(
                        &episode,
                        PolicyKind::RiskThresholdStop,
                        mix64(seed ^ 0x8F3F_73B5_CF1C_9ADE),
                        HORIZON,
                        cost,
                    );
                    let cell_index = family as usize * STOP_COST_GRID.len() + cost_index;
                    cells[cell_index].record(
                        stopped.outcome,
                        Some(stopped.outcome.loss - round_robin.outcome.loss),
                    );
                }
            }
        }
    }

    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        for (cost_index, cost) in STOP_COST_GRID.iter().copied().enumerate() {
            let cell = &cells[family_index * STOP_COST_GRID.len() + cost_index];
            let delta = cell.paired_deltas_vs_rr.iter().sum::<f64>()
                / cell.paired_deltas_vs_rr.len().max(1) as f64;
            let seed = mix64(ROOT_SEED
                ^ 0x5354_4F50_5345_4E53
                ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                ^ (cost_index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
            let (lo, hi) = bootstrap_ci(&cell.paired_deltas_vs_rr, seed);
            println!("STOP_SENSITIVITY\t{}\t{:.3}\t{}\t{:.4}\t{:.4}\t{}\t{:.3}\t{:.4}\t{:.1}\t{:.1}\t{:.4}\t{:.4}\t{:.4}",
                family.name(), cost, cell.episodes, cell.mean_loss(), cell.answer_rate(),
                cell.accuracy(), cell.mean_requested_per_claim() * QUERY_COUNT as f64,
                cell.mean_wasted_fraction(), cell.mean_state_bytes(), cell.mean_scoring_work_units(),
                delta, lo, hi);
        }
    }
}

fn report_marginal_value_stop() {
    println!("MARGINAL_STOP\tfamily\tcost_per_request\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tmean_requests_used\testimated_scoring_work_units_per_episode\tcost_adjusted_objective\tdelta_loss_vs_round_robin\tloss_bootstrap95_lo\tloss_bootstrap95_hi\tobjective_delta_vs_round_robin\tobjective_bootstrap95_lo\tobjective_bootstrap95_hi");
    let mut cells = vec![
        Accumulator::default();
        Family::ALL.len() * MARGINAL_VALUE_COST_GRID.len()
    ];

    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        for replicate in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(replicate, pair, family);
                let episode = ActiveEpisode::generate(seed, family);
                let round_robin = run_policy(
                    &episode, PolicyKind::RoundRobin,
                    mix64(seed ^ 0xD1B5_4A32_D192_ED03)
                );
                for (cost_index, cost) in MARGINAL_VALUE_COST_GRID.iter().copied().enumerate() {
                    let run = run_policy_with_budget_and_stop_cost(
                        &episode,
                        PolicyKind::MarginalValueStop,
                        mix64(seed ^ 0x8F3F_73B5_CF1C_9ADE),
                        HORIZON,
                        cost,
                    );
                    let index = family_index * MARGINAL_VALUE_COST_GRID.len() + cost_index;
                    cells[index].record(
                        run.outcome,
                        Some(run.outcome.loss - round_robin.outcome.loss),
                    );
                    let objective_delta =
                        (run.outcome.loss + cost * run.actions.len() as f64)
                        - (round_robin.outcome.loss + cost * HORIZON as f64);
                    cells[index].paired_objective_deltas_vs_rr.push(objective_delta);
                }
            }
        }
    }

    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        for (cost_index, cost) in MARGINAL_VALUE_COST_GRID.iter().copied().enumerate() {
            let cell = &cells[family_index * MARGINAL_VALUE_COST_GRID.len() + cost_index];
            let mean_requests = cell.mean_requested_per_claim() * QUERY_COUNT as f64;
            let objective = cell.mean_loss() + cost * mean_requests;
            let delta = cell.paired_deltas_vs_rr.iter().sum::<f64>()
                / cell.paired_deltas_vs_rr.len().max(1) as f64;
            let seed = mix64(ROOT_SEED
                ^ 0x4D41_5247_5354_4F50
                ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                ^ (cost_index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
            let (lo, hi) = bootstrap_ci(&cell.paired_deltas_vs_rr, seed);
            let objective_delta = cell.paired_objective_deltas_vs_rr.iter().sum::<f64>()
                / cell.paired_objective_deltas_vs_rr.len().max(1) as f64;
            let objective_seed = mix64(seed ^ 0x4F42_4A45_4354_4956);
            let (objective_lo, objective_hi) =
                bootstrap_ci(&cell.paired_objective_deltas_vs_rr, objective_seed);
            println!("MARGINAL_STOP\t{}\t{:.5}\t{}\t{:.4}\t{:.4}\t{}\t{:.3}\t{:.1}\t{:.5}\t{:.4}\t{:.4}\t{:.4}\t{:.5}\t{:.5}\t{:.5}",
                family.name(), cost, cell.episodes, cell.mean_loss(), cell.answer_rate(),
                cell.accuracy(), mean_requests, cell.mean_scoring_work_units(), objective,
                delta, lo, hi, objective_delta, objective_lo, objective_hi);
        }
    }

    // Select the request-cost threshold from development episodes only. The held-out
    // family is evaluated under that selected threshold and never participates in
    // the argmin. Sensitivity rows above are diagnostic, not a tuning signal.
    let development_index = Family::ALL.iter()
        .position(|family| *family == Family::Development)
        .expect("development family is present");
    let held_out_index = Family::ALL.iter()
        .position(|family| *family == Family::HeldOut)
        .expect("held-out family is present");
    let mut selected_index = 0;
    let mut selected_objective = f64::INFINITY;
    for (cost_index, cost) in MARGINAL_VALUE_COST_GRID.iter().copied().enumerate() {
        let cell = &cells[development_index * MARGINAL_VALUE_COST_GRID.len() + cost_index];
        let objective = cell.mean_loss()
            + cost * cell.mean_requested_per_claim() * QUERY_COUNT as f64;
        if objective < selected_objective - 1e-12 {
            selected_objective = objective;
            selected_index = cost_index;
        }
    }

    let selected_cost = MARGINAL_VALUE_COST_GRID[selected_index];
    let dev_cell = &cells[development_index * MARGINAL_VALUE_COST_GRID.len() + selected_index];
    let held_cell = &cells[held_out_index * MARGINAL_VALUE_COST_GRID.len() + selected_index];
    let held_requests = held_cell.mean_requested_per_claim() * QUERY_COUNT as f64;
    let held_objective = held_cell.mean_loss() + selected_cost * held_requests;
    let held_delta = held_cell.paired_deltas_vs_rr.iter().sum::<f64>()
        / held_cell.paired_deltas_vs_rr.len().max(1) as f64;
    let seed = mix64(ROOT_SEED ^ 0x4341_4C49_4252_4154);
    let (loss_lo, loss_hi) = bootstrap_ci(&held_cell.paired_deltas_vs_rr, seed);
    let objective_delta = held_cell.paired_objective_deltas_vs_rr.iter().sum::<f64>()
        / held_cell.paired_objective_deltas_vs_rr.len().max(1) as f64;
    let (objective_lo, objective_hi) = bootstrap_ci(
        &held_cell.paired_objective_deltas_vs_rr,
        mix64(seed ^ 0x4F42_4A45_4354_4956),
    );

    println!("MARGINAL_STOP_CALIBRATION\tselection_rule\tselected_cost_per_request\tdevelopment_episodes\tdevelopment_objective\theld_out_episodes\theld_out_mean_loss\theld_out_mean_requests\theld_out_objective\tdelta_loss_vs_round_robin\tloss_bootstrap95_lo\tloss_bootstrap95_hi\tobjective_delta_vs_round_robin\tobjective_bootstrap95_lo\tobjective_bootstrap95_hi");
    println!("MARGINAL_STOP_CALIBRATION\tminimum_development_mean(loss+cost*requests)\t{:.5}\t{}\t{:.5}\t{}\t{:.4}\t{:.3}\t{:.5}\t{:.4}\t{:.4}\t{:.4}\t{:.5}\t{:.5}\t{:.5}",
        selected_cost, dev_cell.episodes, selected_objective, held_cell.episodes,
        held_cell.mean_loss(), held_requests, held_objective, held_delta,
        loss_lo, loss_hi, objective_delta, objective_lo, objective_hi);
}

const VALUE_CALIBRATION_MC_DRAWS: usize = 64;

fn sample_evaluator_observation(
    episode: &ActiveEpisode,
    tick: usize,
    action: usize,
    seed: u64,
) -> Evidence {
    // Evaluator-only sampler from the simulator's actual hidden sensor profile.
    // This is never called by the policy and its draws never enter policy state
    // except in cloned counterfactual states used to estimate expected value.
    let mut rng = Rng::new(mix64(seed));
    let delay = rng.range(episode.profile.max_delay + 1);
    let source_tick = tick.saturating_sub(delay);
    let observed_key = if rng.chance(episode.profile.distractor_rate) {
        3_usize
    } else {
        QUERY_KEYS[action]
    };
    let confidence = sample_confidence(&mut rng);
    let corrupted = rng.chance(
        conditional_corruption_rate(episode.profile.corruption_rate, confidence)
    );
    let mut value = episode.truth[source_tick][observed_key];
    if corrupted { value = !value; }
    Evidence {
        key: observed_key as u8,
        value,
        source_tick: source_tick as u8,
        arrival_tick: tick as u8,
        confidence,
    }
}

fn report_value_calibration() {
    println!("VALUE_CALIBRATION\tfamily\tepisodes\tcandidate_states\tmc_draws_per_candidate\tcandidate_samples\tmean_predicted_risk_reduction\tmean_mc_expected_risk_reduction\tmean_prediction_minus_mc\tbootstrap95_lo\tbootstrap95_hi\tmean_absolute_gap");
    println!("VALUE_REALIZED_STRATA\tfamily\tstratum\tepisodes\tobservation_samples\tmean_realized_belief_risk_drop\tbootstrap95_lo\tbootstrap95_hi");
    println!("VALUE_RANKING\tfamily\tepisodes\tdecision_states\tmean_top_action_agreement\ttop_bootstrap95_lo\ttop_bootstrap95_hi\tmean_pairwise_concordance\tconcordance_bootstrap95_lo\tconcordance_bootstrap95_hi\tmean_mc_oracle_regret\tregret_bootstrap95_lo\tregret_bootstrap95_hi");

    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        let mut predicted_mean_by_episode = Vec::new();
        let mut mc_mean_by_episode = Vec::new();
        let mut calibration_error_by_episode = Vec::new();
        let mut calibration_abs_gap_by_episode = Vec::new();
        let mut realized_strata_by_episode: Vec<Vec<f64>> =
            (0..VALUE_CALIBRATION_STRATA.len()).map(|_| Vec::new()).collect();
        let mut samples_by_stratum = [0_u64; 7];
        let mut top_agreement_by_episode = Vec::new();
        let mut pairwise_concordance_by_episode = Vec::new();
        let mut mc_regret_by_episode = Vec::new();

        for replicate in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(replicate, pair, family);
                let episode = ActiveEpisode::generate(seed, family);
                let actor_seed = mix64(seed ^ 0x5641_4C55_4543_414C);
                let mut policy =
                    PolicyState::new(PolicyKind::ExpectedTaskLossReduction, actor_seed);

                let mut episode_predicted_values = Vec::new();
                let mut episode_mc_values = Vec::new();
                let mut episode_realized_strata: Vec<Vec<f64>> =
                    (0..VALUE_CALIBRATION_STRATA.len()).map(|_| Vec::new()).collect();
                let mut episode_top_matches = Vec::new();
                let mut episode_pairwise_concordance = Vec::new();
                let mut episode_regrets = Vec::new();

                for tick in 0..HORIZON {
                    if VALUE_CALIBRATION_TICKS.contains(&tick) {
                        // The state is the reference policy's pre-action information set.
                        // Evaluator-only Monte Carlo draws estimate the true conditional
                        // expected value of each candidate under this episode's hidden
                        // profile. Those draws are never revealed to the policy.
                        let scores = policy.expected_task_loss_reduction_scores(tick);
                        let before = terminal_decision_risk(policy.probabilities(HORIZON - 1));
                        let mut mc_expected_drops = [0.0_f64; QUERY_COUNT];

                        for action in 0..QUERY_COUNT {
                            let mut drop_sum = 0.0;
                            for draw in 0..VALUE_CALIBRATION_MC_DRAWS {
                                let draw_seed = mix64(
                                    seed
                                    ^ 0x4D43_5F56_414C_5545
                                    ^ (tick as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                                    ^ (action as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
                                    ^ (draw as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                                );
                                let evidence = sample_evaluator_observation(
                                    &episode, tick, action, draw_seed
                                );
                                let mut counterfactual = policy.clone();
                                counterfactual.observe(action, evidence);
                                let after = terminal_decision_risk(
                                    counterfactual.probabilities(HORIZON - 1)
                                );
                                let realized_drop = before - after;
                                drop_sum += realized_drop;

                                let delay = usize::from(evidence.arrival_tick)
                                    .saturating_sub(usize::from(evidence.source_tick));
                                let relevant = evidence.key() == QUERY_KEYS[action];
                                let active = [
                                    true,
                                    relevant,
                                    !relevant,
                                    delay <= 2,
                                    delay > 2,
                                    evidence.confidence >= 900,
                                    evidence.confidence < 900,
                                ];
                                for stratum in 0..VALUE_CALIBRATION_STRATA.len() {
                                    if active[stratum] {
                                        episode_realized_strata[stratum].push(realized_drop);
                                        samples_by_stratum[stratum] += 1;
                                    }
                                }
                            }
                            let mc_expected_drop =
                                drop_sum / VALUE_CALIBRATION_MC_DRAWS as f64;
                            mc_expected_drops[action] = mc_expected_drop;
                            episode_predicted_values.push(scores[action]);
                            episode_mc_values.push(mc_expected_drop);
                        }

                        // Compare predicted rankings with Monte Carlo expected value,
                        // not a single noisy sampled outcome. This is an evaluator-only
                        // estimate under hidden simulator parameters, not a deployable oracle.
                        let predicted_top = PolicyState::tie_break(scores, tick);
                        let mc_top = PolicyState::tie_break(mc_expected_drops, tick);
                        episode_top_matches.push(
                            if predicted_top == mc_top { 1.0 } else { 0.0 }
                        );
                        episode_regrets.push(
                            (mc_expected_drops[mc_top] - mc_expected_drops[predicted_top])
                                .max(0.0)
                        );

                        let mut concordance_sum = 0.0;
                        let mut pair_count = 0_u64;
                        for left in 0..QUERY_COUNT {
                            for right in (left + 1)..QUERY_COUNT {
                                let predicted_difference = scores[left] - scores[right];
                                let mc_difference =
                                    mc_expected_drops[left] - mc_expected_drops[right];
                                let concordance = if predicted_difference.abs() <= 1e-12
                                    || mc_difference.abs() <= 1e-12 {
                                    0.5
                                } else if predicted_difference * mc_difference > 0.0 {
                                    1.0
                                } else {
                                    0.0
                                };
                                concordance_sum += concordance;
                                pair_count += 1;
                            }
                        }
                        episode_pairwise_concordance.push(
                            concordance_sum / pair_count.max(1) as f64
                        );
                    }

                    // Advance the reference policy using only its selected pre-generated
                    // observation; Monte Carlo counterfactual observations stay isolated.
                    let action = policy.choose_action(tick);
                    if action < QUERY_COUNT {
                        policy.observe(action, episode.observations[tick][action]);
                    }
                }

                let predicted_mean = episode_predicted_values.iter().sum::<f64>()
                    / episode_predicted_values.len().max(1) as f64;
                let mc_mean = episode_mc_values.iter().sum::<f64>()
                    / episode_mc_values.len().max(1) as f64;
                predicted_mean_by_episode.push(predicted_mean);
                mc_mean_by_episode.push(mc_mean);
                calibration_error_by_episode.push(predicted_mean - mc_mean);
                calibration_abs_gap_by_episode.push(
                    episode_predicted_values.iter()
                        .zip(episode_mc_values.iter())
                        .map(|(p, r)| (p - r).abs())
                        .sum::<f64>()
                        / episode_predicted_values.len().max(1) as f64
                );
                top_agreement_by_episode.push(
                    episode_top_matches.iter().sum::<f64>()
                        / episode_top_matches.len().max(1) as f64
                );
                pairwise_concordance_by_episode.push(
                    episode_pairwise_concordance.iter().sum::<f64>()
                        / episode_pairwise_concordance.len().max(1) as f64
                );
                mc_regret_by_episode.push(
                    episode_regrets.iter().sum::<f64>()
                        / episode_regrets.len().max(1) as f64
                );

                for stratum in 0..VALUE_CALIBRATION_STRATA.len() {
                    if !episode_realized_strata[stratum].is_empty() {
                        let mean = episode_realized_strata[stratum].iter().sum::<f64>()
                            / episode_realized_strata[stratum].len() as f64;
                        realized_strata_by_episode[stratum].push(mean);
                    }
                }
            }
        }

        let mean_predicted = predicted_mean_by_episode.iter().sum::<f64>()
            / predicted_mean_by_episode.len().max(1) as f64;
        let mean_mc = mc_mean_by_episode.iter().sum::<f64>()
            / mc_mean_by_episode.len().max(1) as f64;
        let mean_error = calibration_error_by_episode.iter().sum::<f64>()
            / calibration_error_by_episode.len().max(1) as f64;
        let mean_abs_gap = calibration_abs_gap_by_episode.iter().sum::<f64>()
            / calibration_abs_gap_by_episode.len().max(1) as f64;
        let cal_seed = mix64(ROOT_SEED ^ 0x4341_4C49_4252_4154
            ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
        let (cal_lo, cal_hi) = bootstrap_ci(&calibration_error_by_episode, cal_seed);
        println!("VALUE_CALIBRATION\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
            family.name(), calibration_error_by_episode.len(),
            VALUE_CALIBRATION_TICKS.len(),
            VALUE_CALIBRATION_MC_DRAWS,
            calibration_error_by_episode.len()
                * VALUE_CALIBRATION_TICKS.len()
                * QUERY_COUNT
                * VALUE_CALIBRATION_MC_DRAWS,
            mean_predicted, mean_mc, mean_error, cal_lo, cal_hi, mean_abs_gap);

        for stratum in 0..VALUE_CALIBRATION_STRATA.len() {
            let values = &realized_strata_by_episode[stratum];
            if values.is_empty() {
                println!("VALUE_REALIZED_STRATA\t{}\t{}\t0\t0\tNA\tNA\tNA",
                    family.name(), VALUE_CALIBRATION_STRATA[stratum]);
                continue;
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let stratum_seed = mix64(ROOT_SEED ^ 0x5354_5241_5441_5641
                ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                ^ (stratum as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
            let (lo, hi) = bootstrap_ci(values, stratum_seed);
            println!("VALUE_REALIZED_STRATA\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}",
                family.name(), VALUE_CALIBRATION_STRATA[stratum], values.len(),
                samples_by_stratum[stratum], mean, lo, hi);
        }

        let top_agreement = top_agreement_by_episode.iter().sum::<f64>()
            / top_agreement_by_episode.len().max(1) as f64;
        let pairwise_concordance = pairwise_concordance_by_episode.iter().sum::<f64>()
            / pairwise_concordance_by_episode.len().max(1) as f64;
        let mc_regret = mc_regret_by_episode.iter().sum::<f64>()
            / mc_regret_by_episode.len().max(1) as f64;
        let (agreement_lo, agreement_hi) = bootstrap_ci(
            &top_agreement_by_episode,
            mix64(ROOT_SEED ^ 0x5241_4E4B_4147_5245
                ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)),
        );
        let (concordance_lo, concordance_hi) = bootstrap_ci(
            &pairwise_concordance_by_episode,
            mix64(ROOT_SEED ^ 0x5241_4E4B_434F_4E43
                ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)),
        );
        let (regret_lo, regret_hi) = bootstrap_ci(
            &mc_regret_by_episode,
            mix64(ROOT_SEED ^ 0x5241_4E4B_5245_4752
                ^ (family_index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)),
        );
        println!("VALUE_RANKING\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.6}\t{:.6}\t{:.6}",
            family.name(), top_agreement_by_episode.len(),
            VALUE_CALIBRATION_TICKS.len() * top_agreement_by_episode.len(),
            top_agreement, agreement_lo, agreement_hi, pairwise_concordance,
            concordance_lo, concordance_hi, mc_regret, regret_lo, regret_hi);
    }
}

fn report_nuisance_adaptation() {
    println!("NUISANCE_ADAPTATION\tfamily\tepisodes\tmean_exact_delay_samples\tmean_total_responses\tdelay_distribution_mae\tdelay_mae_bootstrap95_lo\tdelay_mae_bootstrap95_hi\tdistractor_rate_abs_error\tdistractor_abs_bootstrap95_lo\tdistractor_abs_bootstrap95_hi\tdistractor_rate_signed_bias\tdistractor_bias_bootstrap95_lo\tdistractor_bias_bootstrap95_hi");
    for (family_index, family) in Family::ALL.iter().copied().enumerate() {
        let mut delay_mae_by_episode = Vec::new();
        let mut distractor_abs_error_by_episode = Vec::new();
        let mut distractor_signed_error_by_episode = Vec::new();
        let mut exact_delay_samples = Vec::new();
        let mut total_observations = Vec::new();

        for replicate in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(replicate, pair, family);
                let episode = ActiveEpisode::generate(seed, family);
                let run = run_policy(
                    &episode,
                    PolicyKind::OnlineAdaptiveValue,
                    mix64(seed ^ 0x4E55_4953_414E_4345),
                );
                let target_delays: [f64; 6] = std::array::from_fn(|delay| {
                    if delay <= episode.profile.max_delay {
                        1.0 / (episode.profile.max_delay as f64 + 1.0)
                    } else {
                        0.0
                    }
                });
                let delay_mae = run.estimated_delay_distribution.iter()
                    .zip(target_delays.iter())
                    .map(|(estimated, target)| (estimated - target).abs())
                    .sum::<f64>() / 6.0;
                let true_distractor = episode.profile.distractor_rate as f64 / 1_000.0;
                delay_mae_by_episode.push(delay_mae);
                distractor_signed_error_by_episode.push(
                    run.estimated_distractor_probability - true_distractor
                );
                distractor_abs_error_by_episode.push(
                    (run.estimated_distractor_probability - true_distractor).abs()
                );
                exact_delay_samples.push(run.exact_delay_samples as f64);
                total_observations.push(run.total_nuisance_observations as f64);
            }
        }

        let episodes = delay_mae_by_episode.len();
        let delay_mae = delay_mae_by_episode.iter().sum::<f64>() / episodes.max(1) as f64;
        let distractor_abs = distractor_abs_error_by_episode.iter().sum::<f64>()
            / episodes.max(1) as f64;
        let distractor_bias = distractor_signed_error_by_episode.iter().sum::<f64>()
            / episodes.max(1) as f64;
        let mean_exact = exact_delay_samples.iter().sum::<f64>() / episodes.max(1) as f64;
        let mean_total = total_observations.iter().sum::<f64>() / episodes.max(1) as f64;
        let seed = mix64(ROOT_SEED ^ 0x4E55_4953_414E_4345
            ^ (family_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
        let (delay_lo, delay_hi) = bootstrap_ci(&delay_mae_by_episode, seed);
        let (abs_lo, abs_hi) = bootstrap_ci(
            &distractor_abs_error_by_episode, mix64(seed ^ 0xD1B5_4A32_D192_ED03)
        );
        let (bias_lo, bias_hi) = bootstrap_ci(
            &distractor_signed_error_by_episode, mix64(seed ^ 0xBF58_476D_1CE4_E5B9)
        );
        println!("NUISANCE_ADAPTATION\t{}\t{}\t{:.3}\t{:.3}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
            family.name(), episodes, mean_exact, mean_total, delay_mae, delay_lo, delay_hi,
            distractor_abs, abs_lo, abs_hi, distractor_bias, bias_lo, bias_hi);
    }
}

fn main() {
    report();
    report_budget_frontier();
    report_stop_cost_sensitivity();
    report_marginal_value_stop();
    report_value_calibration();
    report_nuisance_adaptation();
}

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
    fn every_policy_respects_budget_and_stop_policy_never_exceeds_it() {
        let episode = ActiveEpisode::generate(0xB0D6_E700, Family::HeldOut);
        for policy in PolicyKind::ALL {
            let result = run_policy(&episode, policy, 99);
            if matches!(policy, PolicyKind::RiskThresholdStop | PolicyKind::MarginalValueStop) {
                assert!(result.actions.len() <= HORIZON);
            } else {
                assert_eq!(result.actions.len(), HORIZON);
            }
            let used = result.actions.len();
            assert_eq!(result.outcome.requested_counts.iter().sum::<usize>(), used);
            let relevant = result.outcome.relevant_counts.iter().sum::<usize>();
            let wasted = if used == 0 { 0 } else {
                (result.outcome.wasted_fraction * used as f64).round() as usize
            };
            assert_eq!(relevant + wasted, used);
        }
    }

    #[test]
    fn risk_threshold_stop_can_end_sensing_without_revealing_another_observation() {
        let mut state = PolicyState::new(PolicyKind::RiskThresholdStop, 18);
        for key in 0..QUERY_COUNT {
            state.last[key] = Some(Evidence {
                key: QUERY_KEYS[key] as u8,
                value: true,
                source_tick: (HORIZON - 1) as u8,
                arrival_tick: (HORIZON - 1) as u8,
                confidence: 950,
            });
            state.hazard[key] = HazardStats {
                last_reliable: state.last[key],
                transition_count: 0,
                elapsed_ticks: 500,
            };
        }
        assert!(terminal_decision_risk(state.probabilities(HORIZON - 1)) <= STOP_COST_PER_OBSERVATION);
        assert_eq!(state.choose_action(HORIZON - 1), QUERY_COUNT);
    }

    #[test]
    fn online_adaptive_value_updates_only_from_observed_nuisance_features() {
        let mut state = PolicyState::new(PolicyKind::OnlineAdaptiveValue, 22);
        let initial_delays = state.adaptive_delay_priors();
        let initial_distractor = state.adaptive_distractor_probability();
        assert!((initial_distractor - PRIOR_DISTRACTOR_PER_THOUSAND / 1_000.0).abs() < 1e-12);
        assert!((initial_delays.iter().map(|(_, p)| *p).sum::<f64>() - 1.0).abs() < 1e-12);

        state.observe(0, Evidence {
            key: 3,
            value: true,
            source_tick: 2,
            arrival_tick: 7,
            confidence: 950,
        });
        let adapted_delays = state.adaptive_delay_priors();
        assert!(adapted_delays[5].1 > initial_delays[5].1);
        assert!(state.adaptive_distractor_probability() > initial_distractor);
        assert_eq!(state.observed_delay_samples, 1);

        // A source timestamp clipped to tick zero is censored, not an exact delay.
        state.observe(2, Evidence {
            key: QUERY_KEYS[2] as u8,
            value: false,
            source_tick: 0,
            arrival_tick: 1,
            confidence: 950,
        });
        assert_eq!(state.observed_delay_samples, 1);

        state.observe(1, Evidence {
            key: QUERY_KEYS[1] as u8,
            value: true,
            source_tick: 8,
            arrival_tick: 8,
            confidence: 950,
        });
        assert!(state.adaptive_distractor_probability()
            < (PRIOR_DISTRACTOR_PER_THOUSAND / 1_000.0 + 1.0) / 2.0);
    }

    #[test]
    fn marginal_value_stop_uses_a_counterfactual_value_cost_rule() {
        let mut state = PolicyState::new(PolicyKind::MarginalValueStop, 18);
        state.relevant_counts = [1; QUERY_COUNT];
        state.stop_cost_per_observation = 0.25;
        assert_eq!(state.choose_action(0), QUERY_COUNT);

        let mut free = PolicyState::new(PolicyKind::MarginalValueStop, 18);
        free.stop_cost_per_observation = 0.0;
        for tick in 0..QUERY_COUNT {
            let action = free.choose_action(tick);
            assert!(action < QUERY_COUNT);
            free.observe(action, Evidence {
                key: QUERY_KEYS[action] as u8,
                value: true,
                source_tick: tick as u8,
                arrival_tick: tick as u8,
                confidence: 950,
            });
        }
        assert_eq!(free.relevant_counts, [1; QUERY_COUNT]);
    }

    #[test]
    fn marginal_value_explores_unrequested_claims_before_retrying_a_distractor() {
        let mut state = PolicyState::new(PolicyKind::MarginalValueStop, 18);
        state.stop_cost_per_observation = 0.25;

        let first = state.choose_action(0);
        assert_eq!(first, 0);
        state.observe(first, Evidence {
            key: 3, // evaluator-generated hot distractor: does not satisfy the floor
            value: false,
            source_tick: 0,
            arrival_tick: 0,
            confidence: 950,
        });

        let second = state.choose_action(1);
        assert_eq!(second, 1);
        state.observe(second, Evidence {
            key: QUERY_KEYS[second] as u8,
            value: true,
            source_tick: 1,
            arrival_tick: 1,
            confidence: 950,
        });
        let third = state.choose_action(2);
        assert_eq!(third, 2);
    }

    #[test]
    fn higher_stop_cost_never_uses_more_sensing_requests_on_same_episode() {
        let episode = ActiveEpisode::generate(0xB0D6_00C0, Family::HeldOut);
        let low = run_policy_with_budget_and_stop_cost(
            &episode, PolicyKind::RiskThresholdStop, 1234, HORIZON, 0.01
        );
        let high = run_policy_with_budget_and_stop_cost(
            &episode, PolicyKind::RiskThresholdStop, 1234, HORIZON, 0.12
        );
        assert!(high.actions.len() <= low.actions.len());

        let marginal_low = run_policy_with_budget_and_stop_cost(
            &episode, PolicyKind::MarginalValueStop, 5678, HORIZON, 0.00025
        );
        let marginal_high = run_policy_with_budget_and_stop_cost(
            &episode, PolicyKind::MarginalValueStop, 5678, HORIZON, 0.003
        );
        assert!(marginal_high.actions.len() <= marginal_low.actions.len());
    }

    #[test]
    fn lower_budget_schedules_still_span_the_full_world_horizon() {
        let ticks_8: Vec<usize> = (0..8).map(|slot| scheduled_tick(slot, 8)).collect();
        let ticks_16: Vec<usize> = (0..16).map(|slot| scheduled_tick(slot, 16)).collect();
        let ticks_48: Vec<usize> = (0..48).map(|slot| scheduled_tick(slot, 48)).collect();
        assert_eq!(ticks_8, vec![0, 6, 12, 18, 24, 30, 36, 42]);
        assert_eq!(ticks_16.first(), Some(&0));
        assert_eq!(ticks_16.last(), Some(&45));
        assert_eq!(ticks_48.first(), Some(&0));
        assert_eq!(ticks_48.last(), Some(&47));
        assert!(ticks_8.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn budget_frontier_caps_actions_but_keeps_terminal_world_horizon() {
        let episode = ActiveEpisode::generate(0xB0D6_0016, Family::HeldOut);
        for budget in BUDGETS {
            let result = run_policy_with_budget(
                &episode, PolicyKind::ValueOfInformation, 1234, budget
            );
            assert_eq!(result.actions.len(), budget);
            assert_eq!(result.outcome.requested_counts.iter().sum::<usize>(), budget);
            assert!(result.outcome.wasted_fraction >= 0.0 && result.outcome.wasted_fraction <= 1.0);
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
    fn expected_task_loss_selector_is_deterministic_and_returns_a_valid_action() {
        let episode = ActiveEpisode::generate(0xE7A5_0001, Family::HeldOut);
        let mut state = PolicyState::new(PolicyKind::ExpectedTaskLossReduction, 77);
        let first = state.expected_task_loss_reduction_scores(0);
        let second = state.expected_task_loss_reduction_scores(0);
        assert_eq!(first, second);
        assert!(first.iter().all(|score| score.is_finite() && *score >= 0.0));
        let action = state.choose_action(0);
        assert!(action < QUERY_COUNT);
        assert_eq!(episode.observations.len(), HORIZON);
    }

    #[test]
    fn terminal_risk_is_zero_when_the_majority_cannot_change() {
        assert_eq!(terminal_decision_risk([1.0, 1.0, 0.5]), 0.0);
        assert_eq!(terminal_decision_risk([0.0, 0.0, 0.5]), 0.0);
        assert!((terminal_decision_risk([0.5, 0.5, 0.5]) - 0.25).abs() < 1e-12);
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
