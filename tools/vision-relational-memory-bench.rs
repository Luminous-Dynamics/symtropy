// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-002: deterministic causal multi-claim evidence-memory benchmark.
//! Policies receive only a public query and released observations. Hidden
//! truth, seed and future observations remain evaluator-side.
//!
//! rustc --edition=2024 --test tools/vision-relational-memory-bench.rs -o /tmp/vis-mem-tests
//! /tmp/vis-mem-tests
//! rustc --edition=2024 -O tools/vision-relational-memory-bench.rs -o /tmp/vis-mem-bench
//! /tmp/vis-mem-bench
//!
//! This symbolic benchmark isolates memory/decision behavior. It is not an RGB,
//! VLA, robot, physical-safety, runtime or energy result.

const CLAIMS: usize = 8;
const QUERY_WIDTH: usize = 3;
const HORIZON: usize = 47;
const FRESHNESS_LIMIT: usize = 7;
const REPLICATES: usize = 12;
const PAIRS: usize = 32;
const BOOTSTRAPS: usize = 2_000;
// Fresh exploration seed domain for the joint transition/sensor-noise candidate.
// Model development has inspected prior outputs; this is not a preregistered confirmation.
const ROOT_SEED: u64 = 0x1F83_D9AB_FB41_BD6B;
const CAPS: [usize; 5] = [1, 2, 3, 4, 8];
const HAZARD_RATES: [f64; 7] = [0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.4];
const HAZARD_LOG_PRIOR: [i32; 7] = [-1234, -1550, -1755, -2016, -2588, -3067, -3590];
const SENSOR_BASE_RATES: [u64; 6] = [0, 10, 20, 40, 55, 70];
const JOINT_HYPOTHESES: usize = HAZARD_RATES.len() * SENSOR_BASE_RATES.len();
const SENSOR_LOG_PRIOR: i32 = -1835; // log(1/6) on the 1024 fixed-point scale
const LOG_SCORE_SCALE: f64 = 1024.0;
const STRATA_COUNT: usize = 8;
const STRATA_NAMES: [&str; STRATA_COUNT] = [
    "claim_changed_after_latest_source",
    "query_evidence_arrived_out_of_order",
    "latest_query_evidence_low_confidence",
    "query_observation_delay_gt_2",
    "profile_high_distractor_rate",
    "profile_high_claim_flip_rate",
    "profile_high_max_delay",
    "profile_high_corruption_rate",
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
    fn chance(&mut self, per_thousand: u64) -> bool { self.next() % 1_000 < per_thousand }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family { Development, HeldOut }

impl Family {
    fn name(self) -> &'static str {
        match self { Self::Development => "development", Self::HeldOut => "held_out" }
    }

    fn sample_profile(self, rng: &mut Rng) -> NuisanceProfile {
        // Draw each nuisance independently. Development and held-out ranges
        // overlap only at their declared boundary, but vary within each family.
        let (distractor_rate, flip_rate, max_delay, corruption_rate) = match self {
            Self::Development => (
                [150_u64, 250, 350][rng.range(3)],
                [10_u64, 20, 30][rng.range(3)],
                [0_usize, 1, 2][rng.range(3)],
                [0_u64, 10, 20][rng.range(3)],
            ),
            Self::HeldOut => (
                [350_u64, 500, 650][rng.range(3)],
                [30_u64, 50, 70][rng.range(3)],
                [2_usize, 3, 4, 5][rng.range(4)],
                [40_u64, 55, 70][rng.range(3)],
            ),
        };

        let (high_distractor_rate, high_flip_rate, high_max_delay, high_corruption_rate) =
            match self {
                Self::Development => (
                    distractor_rate >= 250,
                    flip_rate >= 20,
                    max_delay >= 1,
                    corruption_rate >= 10,
                ),
                Self::HeldOut => (
                    distractor_rate >= 500,
                    flip_rate >= 50,
                    max_delay >= 4,
                    corruption_rate >= 55,
                ),
            };

        NuisanceProfile {
            distractor_rate,
            flip_rate,
            max_delay,
            corruption_rate,
            high_distractor_rate,
            high_flip_rate,
            high_max_delay,
            high_corruption_rate,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NuisanceProfile {
    distractor_rate: u64,
    flip_rate: u64,
    max_delay: usize,
    corruption_rate: u64,
    high_distractor_rate: bool,
    high_flip_rate: bool,
    high_max_delay: bool,
    high_corruption_rate: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Query { keys: [usize; QUERY_WIDTH] }

impl Query {
    fn contains(self, key: usize) -> bool { self.keys.contains(&key) }
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
    // Source time determines freshness; for equal source times use arrival
    // order as the deterministic tie-break. Confidence is not chronology.
    fn rank(self) -> (u8, u8) { (self.source_tick, self.arrival_tick) }
    fn newer_than(self, old: Self) -> bool { self.rank() > old.rank() }
    fn fresh_at(self, tick: usize) -> bool {
        tick.saturating_sub(usize::from(self.source_tick)) <= FRESHNESS_LIMIT
    }
}

fn sample_confidence(rng: &mut Rng) -> u16 {
    // Draw reported confidence independently of whether the value is
    // corrupted. The tier changes the observation's error probability below.
    match rng.range(1_000) {
        0..=749 => 950,
        750..=949 => 850,
        _ => 650,
    }
}

fn conditional_corruption_rate(base_rate: u64, confidence: u16) -> u64 {
    // Normalize per-tier relative risks so the weighted population corruption
    // probability is approximately base_rate / 1000 for tier weights
    // (0.75, 0.20, 0.05). Relative risk multipliers are (20, 160, 320) / 63.
    let multiplier = match confidence {
        950 => 20_u64,
        850 => 160_u64,
        650 => 320_u64,
        _ => 160_u64,
    };
    (base_rate.saturating_mul(multiplier).saturating_add(31) / 63).min(999)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Episode {
    query: Query,
    // Evaluator-only domain profile; never passed to a memory policy.
    profile: NuisanceProfile,
    // Evaluator-only state: never passed to a memory policy.
    truth: Vec<[bool; CLAIMS]>,
    // Only these events are revealed to the policy, in arrival order.
    stream: Vec<Evidence>,
}

impl Episode {
    fn generate(seed: u64, family: Family) -> Self {
        let mut rng = Rng::new(seed);
        let mut order = [0, 1, 2, 3, 4, 5, 6, 7];
        for i in (1..CLAIMS).rev() {
            let j = rng.range(i + 1);
            order.swap(i, j);
        }
        let query = Query { keys: [order[0], order[1], order[2]] };
        let hot_distractor = order[3];
        let profile = family.sample_profile(&mut rng);

        let mut truth = vec![[false; CLAIMS]; HORIZON + 1];
        for k in 0..CLAIMS { truth[0][k] = rng.chance(500); }
        for tick in 1..=HORIZON {
            truth[tick] = truth[tick - 1];
            for k in 0..CLAIMS {
                if rng.chance(profile.flip_rate) { truth[tick][k] = !truth[tick][k]; }
            }
        }

        let mut stream = Vec::with_capacity(HORIZON + 1);
        for arrival in 0..=HORIZON {
            let (key, source) = if arrival < QUERY_WIDTH {
                (query.keys[arrival], 0)
            } else {
                let key = if rng.chance(profile.distractor_rate) {
                    hot_distractor
                } else {
                    query.keys[rng.range(QUERY_WIDTH)]
                };
                let delay = rng.range(profile.max_delay + 1);
                (key, arrival.saturating_sub(delay))
            };
            let confidence = sample_confidence(&mut rng);
            let corruption_probability = conditional_corruption_rate(profile.corruption_rate, confidence);
            let corrupted = rng.chance(corruption_probability);
            let mut value = truth[source][key];
            if corrupted { value = !value; }
            stream.push(Evidence {
                key: key as u8,
                value,
                source_tick: source as u8,
                arrival_tick: arrival as u8,
                confidence,
            });
        }
        Self { query, profile, truth, stream }
    }

    fn target_answer(&self) -> bool {
        self.query.keys.iter().filter(|key| self.truth[HORIZON][**key]).count() >= 2
    }

    /// Evaluator-only diagnostic labels. This method never feeds the memory
    /// policy; it inspects hidden truth strictly for stratified evaluation.
    fn evaluator_strata(&self) -> [bool; STRATA_COUNT] {
        let mut latest_source: [Option<Evidence>; CLAIMS] = [None; CLAIMS];
        let mut max_source_seen: [Option<u8>; CLAIMS] = [None; CLAIMS];
        let mut out_of_order = false;
        let mut long_delay = false;

        for evidence in self.stream.iter().copied() {
            let key = evidence.key();
            if !self.query.contains(key) { continue; }

            if let Some(maximum) = max_source_seen[key] {
                if evidence.source_tick < maximum {
                    out_of_order = true;
                }
            }
            max_source_seen[key] = Some(max_source_seen[key]
                .map_or(evidence.source_tick, |old| old.max(evidence.source_tick)));

            if usize::from(evidence.arrival_tick)
                .saturating_sub(usize::from(evidence.source_tick)) > 2
            {
                long_delay = true;
            }

            if latest_source[key].is_none_or(|old| evidence.newer_than(old)) {
                latest_source[key] = Some(evidence);
            }
        }

        let mut truth_changed_after_latest = false;
        let mut low_confidence_latest = false;
        for key in self.query.keys {
            if let Some(evidence) = latest_source[key] {
                low_confidence_latest |= evidence.confidence < 900;
                for tick in usize::from(evidence.source_tick) + 1..=HORIZON {
                    if self.truth[tick][key] != self.truth[tick - 1][key] {
                        truth_changed_after_latest = true;
                    }
                }
            } else {
                truth_changed_after_latest = true;
            }
        }

        [
            truth_changed_after_latest,
            out_of_order,
            low_confidence_latest,
            long_delay,
            self.profile.high_distractor_rate,
            self.profile.high_flip_rate,
            self.profile.high_max_delay,
            self.profile.high_corruption_rate,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind { Fifo, Reservoir, QueryKeyed, QueryKeyedSelective, QueryKeyedHazardAware, QueryKeyedGapAware, QueryKeyedJointNoise }

impl Kind {
    const ALL: [Self; 7] = [
        Self::Fifo,
        Self::Reservoir,
        Self::QueryKeyed,
        Self::QueryKeyedSelective,
        Self::QueryKeyedHazardAware,
        Self::QueryKeyedGapAware,
        Self::QueryKeyedJointNoise,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::Reservoir => "uniform_reservoir",
            Self::QueryKeyed => "query_keyed_latest",
            Self::QueryKeyedSelective => "query_keyed_selective",
            Self::QueryKeyedHazardAware => "query_keyed_hazard_aware",
            Self::QueryKeyedGapAware => "query_keyed_gap_aware",
            Self::QueryKeyedJointNoise => "query_keyed_joint_noise",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecisionThreshold {
    min_confidence: u16,
    max_age: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct HazardStats {
    last_reliable: Option<Evidence>,
    transition_count: u16,
    elapsed_ticks: u16,
}

impl HazardStats {
    fn observe(&mut self, evidence: Evidence) {
        // Use only released evidence marked high-confidence. This is not an
        // evaluator corruption flag; no hidden truth crosses this boundary.
        if evidence.confidence < 900 { return; }
        match self.last_reliable {
            None => self.last_reliable = Some(evidence),
            Some(previous) if evidence.source_tick > previous.source_tick => {
                let gap = evidence.source_tick.saturating_sub(previous.source_tick) as u16;
                self.elapsed_ticks = self.elapsed_ticks.saturating_add(gap);
                if evidence.value != previous.value {
                    self.transition_count = self.transition_count.saturating_add(1);
                }
                self.last_reliable = Some(evidence);
            }
            // Same-source duplicates and late older observations do not
            // invent a state transition or rewind the estimator.
            Some(_) => {}
        }
    }

    fn estimated_flip_hazard(self) -> f64 {
        // Regularized per-source-tick estimate with a 5% prior mean. Clamp
        // below 0.5 to keep the symmetric two-state persistence model valid.
        ((f64::from(self.transition_count) + 1.0)
            / (f64::from(self.elapsed_ticks) + 20.0))
            .clamp(0.0, 0.49)
    }
}

fn majority_decision(probabilities: [f64; QUERY_WIDTH]) -> Option<bool> {
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

fn probability_different_after_gap(hazard: f64, gap: u8) -> f64 {
    assert!(gap > 0, "source-time gap must be positive");
    let persistence = (1.0 - 2.0 * hazard.clamp(0.0, 0.5 - 1e-9)).powi(i32::from(gap));
    ((1.0 - persistence) / 2.0).clamp(1e-9, 0.5 - 1e-9)
}

#[derive(Clone, Copy, Debug)]
struct GapAwareStats {
    last_reliable: Option<Evidence>,
    log_scores: [i32; HAZARD_RATES.len()],
}

impl Default for GapAwareStats {
    fn default() -> Self {
        Self {
            last_reliable: None,
            log_scores: HAZARD_LOG_PRIOR,
        }
    }
}

impl GapAwareStats {
    fn observe(&mut self, evidence: Evidence) {
        // Use only the released high-confidence tier. Confidence is not a
        // corruption label, and evaluator truth never reaches this policy.
        if evidence.confidence < 900 { return; }
        match self.last_reliable {
            None => self.last_reliable = Some(evidence),
            Some(previous) if evidence.source_tick > previous.source_tick => {
                let gap = evidence.source_tick - previous.source_tick;
                let changed = evidence.value != previous.value;
                for (index, hazard) in HAZARD_RATES.iter().copied().enumerate() {
                    // Symmetric two-state Markov likelihood for an irregular
                    // observation interval of length gap.
                    let probability_changed = probability_different_after_gap(hazard, gap);
                    let likelihood = if changed { probability_changed } else { 1.0 - probability_changed };
                    let increment = (likelihood.ln() * LOG_SCORE_SCALE).round() as i32;
                    self.log_scores[index] = self.log_scores[index].saturating_add(increment);
                }
                self.last_reliable = Some(evidence);
            }
            // Same-source duplicates and delayed older reports cannot rewind
            // source-time likelihood statistics.
            Some(_) => {}
        }
    }

    fn estimated_flip_hazard(self) -> f64 {
        let max_score = *self.log_scores.iter().max().unwrap_or(&0);
        let weights: [f64; HAZARD_RATES.len()] = std::array::from_fn(|index| {
            (f64::from(self.log_scores[index] - max_score) / LOG_SCORE_SCALE).exp()
        });
        let denominator = weights.iter().sum::<f64>().max(f64::MIN_POSITIVE);
        HAZARD_RATES.iter().zip(weights.iter())
            .map(|(rate, weight)| rate * weight)
            .sum::<f64>() / denominator
    }
}

#[derive(Clone, Copy, Debug)]
struct JointNoiseStats {
    last_source_tick: Option<u8>,
    // Posterior P(latent=true) for each (hazard, sensor-base-rate) hypothesis,
    // stored as fixed-point permille-ish u16 probabilities.
    alpha_true: [u16; JOINT_HYPOTHESES],
    log_scores: [i32; JOINT_HYPOTHESES],
}

impl Default for JointNoiseStats {
    fn default() -> Self {
        Self {
            last_source_tick: None,
            alpha_true: [32_768; JOINT_HYPOTHESES],
            log_scores: std::array::from_fn(|index| {
                HAZARD_LOG_PRIOR[index / SENSOR_BASE_RATES.len()] + SENSOR_LOG_PRIOR
            }),
        }
    }
}

impl JointNoiseStats {
    fn observe(&mut self, evidence: Evidence) {
        let previous_tick = self.last_source_tick;
        // Out-of-order observations cannot rewind the latent-state filter.
        if previous_tick.is_some_and(|previous| evidence.source_tick < previous) {
            return;
        }

        for hypothesis in 0..JOINT_HYPOTHESES {
            let hazard_index = hypothesis / SENSOR_BASE_RATES.len();
            let sensor_index = hypothesis % SENSOR_BASE_RATES.len();
            let hazard = HAZARD_RATES[hazard_index];
            let sensor_base_rate = SENSOR_BASE_RATES[sensor_index];
            let observation_error =
                conditional_corruption_rate(sensor_base_rate, evidence.confidence) as f64 / 1_000.0;

            let prior_true = if let Some(previous_source_tick) = previous_tick {
                let previous_true = f64::from(self.alpha_true[hypothesis]) / 65_535.0;
                let gap = evidence.source_tick - previous_source_tick;
                let p_change = if gap == 0 {
                    0.0
                } else {
                    probability_different_after_gap(hazard, gap)
                };
                previous_true * (1.0 - p_change) + (1.0 - previous_true) * p_change
            } else {
                0.5
            };

            let observation_likelihood = if evidence.value {
                prior_true * (1.0 - observation_error) + (1.0 - prior_true) * observation_error
            } else {
                prior_true * observation_error + (1.0 - prior_true) * (1.0 - observation_error)
            }.clamp(1e-9, 1.0);

            let posterior_true = if evidence.value {
                prior_true * (1.0 - observation_error) / observation_likelihood
            } else {
                prior_true * observation_error / observation_likelihood
            }.clamp(0.0, 1.0);

            let increment = (observation_likelihood.ln() * LOG_SCORE_SCALE).round() as i32;
            self.log_scores[hypothesis] = self.log_scores[hypothesis].saturating_add(increment);
            self.alpha_true[hypothesis] = (posterior_true * 65_535.0).round() as u16;
        }
        self.last_source_tick = Some(evidence.source_tick);
    }

    fn posterior_weights(self) -> [f64; JOINT_HYPOTHESES] {
        let max_score = *self.log_scores.iter().max().unwrap_or(&0);
        std::array::from_fn(|index| {
            (f64::from(self.log_scores[index] - max_score) / LOG_SCORE_SCALE).exp()
        })
    }

    fn estimated_flip_hazard(self) -> f64 {
        let weights = self.posterior_weights();
        let denominator = weights.iter().sum::<f64>().max(f64::MIN_POSITIVE);
        (0..JOINT_HYPOTHESES)
            .map(|index| HAZARD_RATES[index / SENSOR_BASE_RATES.len()] * weights[index])
            .sum::<f64>() / denominator
    }

    fn estimated_sensor_base_rate(self) -> f64 {
        let weights = self.posterior_weights();
        let denominator = weights.iter().sum::<f64>().max(f64::MIN_POSITIVE);
        (0..JOINT_HYPOTHESES)
            .map(|index| {
                (SENSOR_BASE_RATES[index % SENSOR_BASE_RATES.len()] as f64 / 1_000.0)
                    * weights[index]
            })
            .sum::<f64>() / denominator
    }

    fn probability_true_at(self, tick: usize) -> f64 {
        let Some(last_source_tick) = self.last_source_tick else { return 0.5; };
        let age = tick.saturating_sub(usize::from(last_source_tick));
        let weights = self.posterior_weights();
        let denominator = weights.iter().sum::<f64>().max(f64::MIN_POSITIVE);
        (0..JOINT_HYPOTHESES)
            .map(|index| {
                let hazard = HAZARD_RATES[index / SENSOR_BASE_RATES.len()];
                let previous_true = f64::from(self.alpha_true[index]) / 65_535.0;
                let persistence = (1.0 - 2.0 * hazard).powi(age as i32);
                let p_change = (1.0 - persistence) / 2.0;
                let p_true = previous_true * (1.0 - p_change)
                    + (1.0 - previous_true) * p_change;
                p_true * weights[index]
            })
            .sum::<f64>() / denominator
    }
}

#[derive(Clone, Debug)]
struct Memory {
    kind: Kind,
    cap: usize,
    query: Query,
    entries: Vec<Evidence>,
    seen: u64,
    actor_seed: u64,
    // Per-query sufficient statistics are included in logical policy cost
    // only for the policy that consumes them.
    hazard_stats: [HazardStats; QUERY_WIDTH],
    gap_aware_stats: [GapAwareStats; QUERY_WIDTH],
    joint_noise_stats: [JointNoiseStats; QUERY_WIDTH],
}

impl Memory {
    fn new(kind: Kind, cap: usize, query: Query, actor_seed: u64) -> Self {
        Self {
            kind,
            cap,
            query,
            entries: Vec::with_capacity(cap),
            seen: 0,
            actor_seed,
            hazard_stats: [HazardStats::default(); QUERY_WIDTH],
            gap_aware_stats: [GapAwareStats::default(); QUERY_WIDTH],
            joint_noise_stats: [JointNoiseStats::default(); QUERY_WIDTH],
        }
    }

    fn observe(&mut self, e: Evidence) {
        if self.cap == 0 || e.key() >= CLAIMS { return; }
        if self.query.contains(e.key()) {
            if let Some(index) = self.query.keys.iter().position(|key| *key == e.key()) {
                if self.kind == Kind::QueryKeyedHazardAware {
                    self.hazard_stats[index].observe(e);
                } else if self.kind == Kind::QueryKeyedGapAware {
                    self.gap_aware_stats[index].observe(e);
                } else if self.kind == Kind::QueryKeyedJointNoise {
                    self.joint_noise_stats[index].observe(e);
                }
            }
        }
        match self.kind {
            Kind::Fifo => {
                self.entries.push(e);
                if self.entries.len() > self.cap { self.entries.remove(0); }
            }
            Kind::Reservoir => {
                self.seen = self.seen.saturating_add(1);
                if self.entries.len() < self.cap {
                    self.entries.push(e);
                } else {
                    let draw = mix64(
                        self.actor_seed ^ self.seen.wrapping_mul(0xD1B5_4A32_D192_ED03)
                            ^ 0x554E_4946_4F52_4D52,
                    ) % self.seen;
                    if draw < self.cap as u64 { self.entries[draw as usize] = e; }
                }
            }
            Kind::QueryKeyed | Kind::QueryKeyedSelective | Kind::QueryKeyedHazardAware | Kind::QueryKeyedGapAware | Kind::QueryKeyedJointNoise => {
                // Query keys are public. The policy receives no hidden truth,
                // future observation, seed or evaluator annotation.
                if !self.query.contains(e.key()) { return; }
                if let Some(i) = self.entries.iter().position(|old| old.key() == e.key()) {
                    if e.newer_than(self.entries[i]) { self.entries[i] = e; }
                } else if self.entries.len() < self.cap {
                    self.entries.push(e);
                } else if let Some((i, old)) = self.entries.iter().copied()
                    .enumerate().min_by_key(|(_, item)| item.rank())
                {
                    if e.newer_than(old) { self.entries[i] = e; }
                }
            }
        }
    }

    fn freshest(&self, key: usize) -> Option<Evidence> {
        self.entries.iter().copied().filter(|e| e.key() == key).max_by_key(|e| e.rank())
    }

    fn fresh_claims(&self, tick: usize) -> usize {
        self.query.keys.iter().filter(|key| self.freshest(**key).is_some_and(|e| e.fresh_at(tick))).count()
    }

    fn decide(&self, tick: usize) -> Option<bool> {
        self.decide_with_threshold(tick, DecisionThreshold {
            min_confidence: 0,
            max_age: FRESHNESS_LIMIT,
        })
    }

    fn decide_with_threshold(&self, tick: usize, threshold: DecisionThreshold) -> Option<bool> {
        let mut positive = 0;
        for key in self.query.keys {
            let e = self.freshest(key)?;
            let age = tick.saturating_sub(usize::from(e.source_tick));
            if age > threshold.max_age || e.confidence < threshold.min_confidence {
                return None;
            }
            positive += usize::from(e.value);
        }
        Some(positive >= 2)
    }

    fn decide_joint_noise(&self, tick: usize) -> Option<bool> {
        let probabilities: [f64; QUERY_WIDTH] = std::array::from_fn(|index| {
            self.joint_noise_stats[index].probability_true_at(tick)
        });
        majority_decision(probabilities)
    }

    fn decide_gap_aware(&self, tick: usize) -> Option<bool> {
        let mut probabilities = [0.5_f64; QUERY_WIDTH];
        for (index, key) in self.query.keys.iter().copied().enumerate() {
            let evidence = self.freshest(key)?;
            let age = tick.saturating_sub(usize::from(evidence.source_tick));
            let reliability = (f64::from(evidence.confidence) / 1_000.0).clamp(0.5, 0.99);
            let hazard = self.gap_aware_stats[index].estimated_flip_hazard();
            let parity_persistence = (1.0 - 2.0 * hazard).powi(age as i32);
            let probability_same = 0.5 + (reliability - 0.5) * parity_persistence;
            probabilities[index] = if evidence.value { probability_same } else { 1.0 - probability_same };
        }

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

    fn decide_hazard_aware(&self, tick: usize) -> Option<bool> {
        let mut probabilities = [0.5_f64; QUERY_WIDTH];
        for (index, key) in self.query.keys.iter().copied().enumerate() {
            let evidence = self.freshest(key)?;
            let age = tick.saturating_sub(usize::from(evidence.source_tick));
            let reliability = (f64::from(evidence.confidence) / 1_000.0).clamp(0.5, 0.99);
            let hazard = self.hazard_stats[index].estimated_flip_hazard();
            let parity_persistence = (1.0 - 2.0 * hazard).powi(age as i32);
            let probability_same = 0.5 + (reliability - 0.5) * parity_persistence;
            probabilities[index] = if evidence.value { probability_same } else { 1.0 - probability_same };
        }

        // Exact majority probability for the benchmark's independent binary
        // claims; answer only when expected 0/1 loss beats 0.25 abstention loss.
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
}

#[derive(Clone, Copy, Debug)]
struct Outcome {
    loss: f64,
    answered: bool,
    correct: bool,
    fresh_claims: usize,
    entries: usize,
    logical_state_bytes: usize,
    // Evaluator-only errors; never visible through Memory or a policy callback.
    hazard_error_mae: Option<f64>,
    hazard_error_mse: Option<f64>,
    // Evaluator-only corruption-rate errors for the joint-noise policy.
    sensor_error_mae: Option<f64>,
    sensor_error_mse: Option<f64>,
    // Uniform-grid prior baseline evaluated only by the evaluator.
    sensor_prior_error_mae: f64,
    sensor_prior_error_mse: f64,
}

fn evaluate(ep: &Episode, kind: Kind, cap: usize, actor_seed: u64) -> Outcome {
    evaluate_with_threshold(ep, kind, cap, actor_seed, DecisionThreshold {
        min_confidence: 0,
        max_age: FRESHNESS_LIMIT,
    })
}

fn evaluate_with_threshold(
    ep: &Episode,
    kind: Kind,
    cap: usize,
    actor_seed: u64,
    threshold: DecisionThreshold,
) -> Outcome {
    let mut memory = Memory::new(kind, cap, ep.query, actor_seed);
    for e in ep.stream.iter().copied() { memory.observe(e); }
    let decision = match kind {
        Kind::QueryKeyedSelective => memory.decide_with_threshold(HORIZON, threshold),
        Kind::QueryKeyedHazardAware => memory.decide_hazard_aware(HORIZON),
        Kind::QueryKeyedGapAware => memory.decide_gap_aware(HORIZON),
        Kind::QueryKeyedJointNoise => memory.decide_joint_noise(HORIZON),
        _ => memory.decide(HORIZON),
    };
    let correct = decision.is_some_and(|answer| answer == ep.target_answer());
    // Logical serialized-state estimate: retained evidence plus hazard
    // sufficient statistics. Allocator overhead and executable/runtime memory
    // are excluded and must not be presented as measured RAM.
    let logical_state_bytes = memory.entries.len() * std::mem::size_of::<Evidence>()
        + match kind {
            Kind::QueryKeyedHazardAware => std::mem::size_of::<[HazardStats; QUERY_WIDTH]>(),
            Kind::QueryKeyedGapAware => std::mem::size_of::<[GapAwareStats; QUERY_WIDTH]>(),
            Kind::QueryKeyedJointNoise => std::mem::size_of::<[JointNoiseStats; QUERY_WIDTH]>(),
            _ => 0,
        };

    // This is evaluator-only: the hidden profile rate is read only after the
    // policy has decided. It never enters Memory, evidence, or decision code.
    let hazard_errors = match kind {
        Kind::QueryKeyedHazardAware => Some(memory.hazard_stats.map(|stats| stats.estimated_flip_hazard())),
        Kind::QueryKeyedGapAware => Some(memory.gap_aware_stats.map(|stats| stats.estimated_flip_hazard())),
        Kind::QueryKeyedJointNoise => Some(memory.joint_noise_stats.map(|stats| stats.estimated_flip_hazard())),
        _ => None,
    }.map(|estimates| {
        let true_rate = ep.profile.flip_rate as f64 / 1_000.0;
        let absolute_sum = estimates.iter().map(|estimate| (estimate - true_rate).abs()).sum::<f64>();
        let squared_sum = estimates.iter().map(|estimate| (estimate - true_rate).powi(2)).sum::<f64>();
        (
            absolute_sum / QUERY_WIDTH as f64,
            squared_sum / QUERY_WIDTH as f64,
        )
    });
    // The true episode corruption rate is evaluator-only and read strictly
    // after the policy action. It never enters JointNoiseStats or its inputs.
    let true_sensor_rate = ep.profile.corruption_rate as f64 / 1_000.0;
    let prior_sensor_rate = SENSOR_BASE_RATES.iter().map(|rate| *rate as f64).sum::<f64>()
        / SENSOR_BASE_RATES.len() as f64 / 1_000.0;
    let sensor_prior_error_mae = (prior_sensor_rate - true_sensor_rate).abs();
    let sensor_prior_error_mse = (prior_sensor_rate - true_sensor_rate).powi(2);
    let sensor_errors = if kind == Kind::QueryKeyedJointNoise {
        let estimates = memory.joint_noise_stats.map(|stats| stats.estimated_sensor_base_rate());
        let absolute_sum = estimates.iter().map(|estimate| (estimate - true_sensor_rate).abs()).sum::<f64>();
        let squared_sum = estimates.iter().map(|estimate| (estimate - true_sensor_rate).powi(2)).sum::<f64>();
        Some((absolute_sum / QUERY_WIDTH as f64, squared_sum / QUERY_WIDTH as f64))
    } else {
        None
    };
    Outcome {
        loss: match decision { None => 0.25, Some(_) if correct => 0.0, Some(_) => 1.0 },
        answered: decision.is_some(),
        correct,
        fresh_claims: memory.fresh_claims(HORIZON),
        entries: memory.entries.len(),
        logical_state_bytes,
        hazard_error_mae: hazard_errors.map(|errors| errors.0),
        hazard_error_mse: hazard_errors.map(|errors| errors.1),
        sensor_error_mae: sensor_errors.map(|errors| errors.0),
        sensor_error_mse: sensor_errors.map(|errors| errors.1),
        sensor_prior_error_mae,
        sensor_prior_error_mse,
    }
}

#[derive(Clone, Debug, Default)]
struct Acc {
    n: u64,
    loss_sum: f64,
    answered: u64,
    correct: u64,
    fresh_claims: u64,
    entries: u64,
    logical_state_bytes_sum: u64,
    hazard_error_episodes: u64,
    hazard_abs_error_sum: f64,
    hazard_squared_error_sum: f64,
    sensor_error_episodes: u64,
    sensor_abs_error_sum: f64,
    sensor_squared_error_sum: f64,
    sensor_prior_error_episodes: u64,
    sensor_prior_abs_error_sum: f64,
    sensor_prior_squared_error_sum: f64,
    sensor_mae_delta_vs_prior: Vec<f64>,
    delta_vs_fifo: Vec<f64>,
}

impl Acc {
    fn record(&mut self, o: Outcome, delta: Option<f64>) {
        self.n += 1;
        self.loss_sum += o.loss;
        self.answered += u64::from(o.answered);
        self.correct += u64::from(o.correct);
        self.fresh_claims += o.fresh_claims as u64;
        self.entries += o.entries as u64;
        self.logical_state_bytes_sum += o.logical_state_bytes as u64;
        if let (Some(mae), Some(mse)) = (o.hazard_error_mae, o.hazard_error_mse) {
            self.hazard_error_episodes += 1;
            self.hazard_abs_error_sum += mae;
            self.hazard_squared_error_sum += mse;
        }
        self.sensor_prior_error_episodes += 1;
        self.sensor_prior_abs_error_sum += o.sensor_prior_error_mae;
        self.sensor_prior_squared_error_sum += o.sensor_prior_error_mse;
        if let (Some(mae), Some(mse)) = (o.sensor_error_mae, o.sensor_error_mse) {
            self.sensor_error_episodes += 1;
            self.sensor_abs_error_sum += mae;
            self.sensor_squared_error_sum += mse;
            self.sensor_mae_delta_vs_prior.push(mae - o.sensor_prior_error_mae);
        }
        if let Some(d) = delta { self.delta_vs_fifo.push(d); }
    }
    fn mean_loss(&self) -> f64 { self.loss_sum / self.n.max(1) as f64 }
    fn answer_rate(&self) -> f64 { self.answered as f64 / self.n.max(1) as f64 }
    fn accuracy(&self) -> String {
        if self.answered == 0 { "NA".to_string() }
        else { format!("{:.4}", self.correct as f64 / self.answered as f64) }
    }
    fn mean_logical_state_bytes(&self) -> f64 {
        self.logical_state_bytes_sum as f64 / self.n.max(1) as f64
    }
    fn hazard_mae(&self) -> String {
        if self.hazard_error_episodes == 0 { "NA".to_string() }
        else { format!("{:.4}", self.hazard_abs_error_sum / self.hazard_error_episodes as f64) }
    }
    fn hazard_rmse(&self) -> String {
        if self.hazard_error_episodes == 0 { "NA".to_string() }
        else { format!("{:.4}", (self.hazard_squared_error_sum / self.hazard_error_episodes as f64).sqrt()) }
    }
    fn sensor_mae(&self) -> String {
        if self.sensor_error_episodes == 0 { "NA".to_string() }
        else { format!("{:.4}", self.sensor_abs_error_sum / self.sensor_error_episodes as f64) }
    }
    fn sensor_rmse(&self) -> String {
        if self.sensor_error_episodes == 0 { "NA".to_string() }
        else { format!("{:.4}", (self.sensor_squared_error_sum / self.sensor_error_episodes as f64).sqrt()) }
    }
    fn sensor_prior_mae(&self) -> String {
        if self.sensor_prior_error_episodes == 0 { "NA".to_string() }
        else { format!("{:.4}", self.sensor_prior_abs_error_sum / self.sensor_prior_error_episodes as f64) }
    }
    fn sensor_prior_rmse(&self) -> String {
        if self.sensor_prior_error_episodes == 0 { "NA".to_string() }
        else { format!("{:.4}", (self.sensor_prior_squared_error_sum / self.sensor_prior_error_episodes as f64).sqrt()) }
    }
}

fn bootstrap_ci(values: &[f64], seed: u64) -> (f64, f64) {
    if values.is_empty() { return (f64::NAN, f64::NAN); }
    let mut rng = Rng::new(seed);
    let mut means = Vec::with_capacity(BOOTSTRAPS);
    for _ in 0..BOOTSTRAPS {
        let mut sum = 0.0;
        for _ in 0..values.len() { sum += values[rng.range(values.len())]; }
        means.push(sum / values.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    (means[BOOTSTRAPS * 25 / 1_000], means[BOOTSTRAPS * 975 / 1_000])
}

fn seed_for(rep: usize, pair: usize, family: Family) -> u64 {
    let domain = match family {
        Family::Development => 0x4445_565F_524D_454D,
        Family::HeldOut => 0x4845_4C44_524D_454D,
    };
    mix64(ROOT_SEED ^ domain
        ^ (rep as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (pair as u64).wrapping_mul(0xA24B_AED4_963E_E407))
}

const CALIBRATION_EPISODES: usize = 256;

fn calibration_seed(index: usize) -> u64 {
    mix64(ROOT_SEED
        ^ 0x4341_4C49_4252_4154
        ^ (index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03))
}

fn calibrate_threshold() -> DecisionThreshold {
    // A completely separate development-seed set selects the threshold.
    // Held-out report seeds never participate in this selection.
    let confidence_grid = [650_u16, 900_u16];
    let age_grid = [2_usize, 4_usize, FRESHNESS_LIMIT];
    let mut best: Option<(f64, DecisionThreshold)> = None;

    for min_confidence in confidence_grid {
        for max_age in age_grid {
            let threshold = DecisionThreshold { min_confidence, max_age };
            let mut loss_sum = 0.0;
            for index in 0..CALIBRATION_EPISODES {
                let seed = calibration_seed(index);
                let ep = Episode::generate(seed, Family::Development);
                let actor_seed = mix64(seed ^ 0x5345_4C45_4354_4956);
                loss_sum += evaluate_with_threshold(
                    &ep,
                    Kind::QueryKeyedSelective,
                    3,
                    actor_seed,
                    threshold,
                ).loss;
            }
            let mean_loss = loss_sum / CALIBRATION_EPISODES as f64;
            let replace = match best {
                None => true,
                Some((old_loss, old_threshold)) => {
                    mean_loss < old_loss - 1e-12
                        || ((mean_loss - old_loss).abs() <= 1e-12
                            && (threshold.min_confidence > old_threshold.min_confidence
                                || (threshold.min_confidence == old_threshold.min_confidence
                                    && threshold.max_age < old_threshold.max_age)))
                }
            };
            if replace { best = Some((mean_loss, threshold)); }
        }
    }
    best.expect("threshold grid is non-empty").1
}

const IDENTIFIABILITY_REPLICATES: usize = 24;
const IDENTIFIABILITY_LENGTHS: [usize; 5] = [8, 16, 32, 64, 128];

fn joint_noise_identifiability_report() {
    println!("Joint transition/noise identifiability diagnostic (controlled on-grid synthetic trajectories)");
    println!("Only the evaluator knows the simulated hazard and corruption rate; the estimator sees confidence, observed binary value and source tick");
    println!("CALIBRATION\tobservations_per_claim\ttrue_hazard\ttrue_sensor_base_per_thousand\treplicates\thazard_mae\thazard_rmse\tsensor_base_mae\tsensor_base_rmse\tuniform_prior_sensor_mae");

    let prior_sensor_rate = SENSOR_BASE_RATES.iter().map(|rate| *rate as f64).sum::<f64>()
        / SENSOR_BASE_RATES.len() as f64 / 1_000.0;

    for (length_index, observations_per_claim) in IDENTIFIABILITY_LENGTHS.iter().copied().enumerate() {
        for (hazard_index, true_hazard) in HAZARD_RATES.iter().copied().enumerate() {
            for (sensor_index, true_sensor_rate) in SENSOR_BASE_RATES.iter().copied().enumerate() {
                let mut hazard_abs_sum = 0.0;
                let mut hazard_squared_sum = 0.0;
                let mut sensor_abs_sum = 0.0;
                let mut sensor_squared_sum = 0.0;
                let mut prior_sensor_abs_sum = 0.0;

                for replicate in 0..IDENTIFIABILITY_REPLICATES {
                    let seed = mix64(
                        ROOT_SEED
                            ^ 0x4944_454E_5449_4659
                            ^ (length_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                            ^ (hazard_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
                            ^ (sensor_index as u64).wrapping_mul(0xA24B_AED4_963E_E407)
                            ^ (replicate as u64).wrapping_mul(0x94D0_49BB_1331_11EB),
                    );
                    let mut rng = Rng::new(seed);
                    let mut latent_true = rng.chance(500);
                    let mut stats = JointNoiseStats::default();

                    for tick in 0..observations_per_claim {
                        if tick > 0 && rng.chance((true_hazard * 1_000.0).round() as u64) {
                            latent_true = !latent_true;
                        }
                        let confidence = sample_confidence(&mut rng);
                        let p_corrupt = conditional_corruption_rate(true_sensor_rate, confidence);
                        let corrupted = rng.chance(p_corrupt);
                        let observed = if corrupted { !latent_true } else { latent_true };
                        let source_tick = u8::try_from(tick).expect("calibration horizon fits source timestamp");

                        // The policy sees only this release. Hidden hazard and
                        // sensor rate remain evaluator-local for error scoring.
                        stats.observe(Evidence {
                            key: 0,
                            value: observed,
                            source_tick,
                            arrival_tick: source_tick,
                            confidence,
                        });
                    }

                    let hazard_error = stats.estimated_flip_hazard() - true_hazard;
                    let sensor_estimate = stats.estimated_sensor_base_rate();
                    let true_sensor_rate_f64 = true_sensor_rate as f64 / 1_000.0;
                    let sensor_error = sensor_estimate - true_sensor_rate_f64;
                    hazard_abs_sum += hazard_error.abs();
                    hazard_squared_sum += hazard_error.powi(2);
                    sensor_abs_sum += sensor_error.abs();
                    sensor_squared_sum += sensor_error.powi(2);
                    prior_sensor_abs_sum += (prior_sensor_rate - true_sensor_rate_f64).abs();
                }

                let denominator = IDENTIFIABILITY_REPLICATES as f64;
                println!(
                    "CALIBRATION\t{}\t{:.4}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}",
                    observations_per_claim,
                    true_hazard,
                    true_sensor_rate,
                    IDENTIFIABILITY_REPLICATES,
                    hazard_abs_sum / denominator,
                    (hazard_squared_sum / denominator).sqrt(),
                    sensor_abs_sum / denominator,
                    (sensor_squared_sum / denominator).sqrt(),
                    prior_sensor_abs_sum / denominator,
                );
            }
        }
    }
}

fn report() {
    let selective_threshold = calibrate_threshold();
    println!("VIS-MEM-002 causal multi-claim evidence-memory benchmark");
    println!("selective threshold calibrated on {CALIBRATION_EPISODES} separate development episodes: min_confidence={}, max_age={}",
        selective_threshold.min_confidence, selective_threshold.max_age);
    println!("8 claims; public majority query over 3; 48 revealed events; source time can differ from arrival time");
    println!("paired policies, correlated distractor bursts, changing claims, delayed and occasionally corrupted evidence");
    println!("confidence is sampled independently; P(confidence 950,850,650)=(0.75,0.20,0.05)");
    println!("conditional corruption p per tier = base_rate * (20,160,320)/63 per thousand; aggregate rate approximately preserved");
    println!("gap-aware policy: candidate per-tick hazards={HAZARD_RATES:?}; Bayesian fixed-point log-likelihood grid, irregular source-time gaps");
    println!("joint-noise policy: joint transition-hazard x sensor-base-rate grid; online binary-state filter, confidence-conditioned emissions");
    println!("joint-noise grid hazards={HAZARD_RATES:?}; sensor base corruption rates per thousand={SENSOR_BASE_RATES:?}; posterior state entries are fixed-size");
    println!("loss: correct=0, abstain=0.25, incorrect=1; intervals are paired episode bootstrap vs FIFO");
    println!("family\tpolicy\tcapacity\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tfresh_query_claim_coverage\tmean_memory_entries\tmean_logical_state_bytes\tmean_hazard_mae\troot_mean_hazard_mse\tmean_sensor_base_mae\troot_mean_sensor_base_mse\tprior_sensor_base_mae\troot_mean_prior_sensor_base_mse\tdelta_loss_vs_fifo\tbootstrap95_lo_vs_fifo\tbootstrap95_hi_vs_fifo");

    for family in [Family::Development, Family::HeldOut] {
        let mut cells = vec![Acc::default(); Kind::ALL.len() * CAPS.len()];
        let mut strata_cells = vec![Acc::default(); STRATA_COUNT * 2 * Kind::ALL.len()];
        for rep in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(rep, pair, family);
                let ep = Episode::generate(seed, family);
                let evaluator_strata = ep.evaluator_strata();
                for (ci, cap) in CAPS.iter().copied().enumerate() {
                    let mut fifo_loss = 0.0;
                    for (pi, kind) in Kind::ALL.iter().copied().enumerate() {
                        let actor_seed = mix64(seed
                            ^ (pi as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03)
                            ^ (cap as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                        let outcome = evaluate_with_threshold(&ep, kind, cap, actor_seed, selective_threshold);
                        let cell = pi * CAPS.len() + ci;
                        if pi == 0 {
                            fifo_loss = outcome.loss;
                            cells[cell].record(outcome, None);
                        } else {
                            cells[cell].record(outcome, Some(outcome.loss - fifo_loss));
                        }

                        // The four evaluator labels are recorded only for the
                        // preregistered primary capacity; they never reach policy code.
                        if cap == 3 {
                            for (stratum_index, active) in evaluator_strata.iter().copied().enumerate() {
                                let level = usize::from(active);
                                let strata_index = (stratum_index * 2 + level) * Kind::ALL.len() + pi;
                                if pi == 0 {
                                    strata_cells[strata_index].record(outcome, None);
                                } else {
                                    strata_cells[strata_index]
                                        .record(outcome, Some(outcome.loss - fifo_loss));
                                }
                            }
                        }
                    }
                }
            }
        }

        for (pi, kind) in Kind::ALL.iter().copied().enumerate() {
            for (ci, cap) in CAPS.iter().copied().enumerate() {
                let cell = &cells[pi * CAPS.len() + ci];
                let (delta, lo, hi) = if pi == 0 {
                    (0.0, 0.0, 0.0)
                } else {
                    let delta = cell.delta_vs_fifo.iter().sum::<f64>() / cell.delta_vs_fifo.len().max(1) as f64;
                    let seed = mix64(ROOT_SEED
                        ^ (cap as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                        ^ (pi as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
                    let (lo, hi) = bootstrap_ci(&cell.delta_vs_fifo, seed);
                    (delta, lo, hi)
                };
                let coverage = cell.fresh_claims as f64 / (cell.n.max(1) * QUERY_WIDTH as u64) as f64;
                let mean_entries = cell.entries as f64 / cell.n.max(1) as f64;
                println!("{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.4}\t{:.3}\t{:.1}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}",
                    family.name(), kind.name(), cap, cell.n, cell.mean_loss(), cell.answer_rate(),
                    cell.accuracy(), coverage, mean_entries, cell.mean_logical_state_bytes(),
                    cell.hazard_mae(), cell.hazard_rmse(), cell.sensor_mae(), cell.sensor_rmse(),
                    cell.sensor_prior_mae(), cell.sensor_prior_rmse(), delta, lo, hi);
            }
        }

        // Direct paired contrasts use the same per-episode streams. Subtracting
        // FIFO-relative deltas cancels FIFO and yields policy A minus policy B.
        let primary_capacity_index = CAPS.iter().position(|cap| *cap == 3).expect("capacity 3 is declared");
        let pairwise = [
            ("gap_aware_minus_hazard_aware", Kind::QueryKeyedGapAware, Kind::QueryKeyedHazardAware),
            ("joint_noise_minus_hazard_aware", Kind::QueryKeyedJointNoise, Kind::QueryKeyedHazardAware),
            ("joint_noise_minus_gap_aware", Kind::QueryKeyedJointNoise, Kind::QueryKeyedGapAware),
        ];
        println!("PAIRWISE\tfamily\tcontrast\tepisodes\tdelta_loss_a_minus_b\tbootstrap95_lo\tbootstrap95_hi");
        for (pair_index, (name, policy_a, policy_b)) in pairwise.iter().copied().enumerate() {
            let index_a = Kind::ALL.iter().position(|kind| *kind == policy_a).expect("policy A is declared");
            let index_b = Kind::ALL.iter().position(|kind| *kind == policy_b).expect("policy B is declared");
            let cell_a = &cells[index_a * CAPS.len() + primary_capacity_index];
            let cell_b = &cells[index_b * CAPS.len() + primary_capacity_index];
            let paired_deltas: Vec<f64> = cell_a.delta_vs_fifo.iter()
                .zip(cell_b.delta_vs_fifo.iter())
                .map(|(delta_a, delta_b)| delta_a - delta_b)
                .collect();
            let paired_delta = paired_deltas.iter().sum::<f64>() / paired_deltas.len().max(1) as f64;
            let family_domain = match family {
                Family::Development => 0x4445_56,
                Family::HeldOut => 0x4845_4C44,
            };
            let paired_seed = mix64(ROOT_SEED ^ 0x5041_4952_5749_5345
                ^ family_domain
                ^ (pair_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
            let (paired_lo, paired_hi) = bootstrap_ci(&paired_deltas, paired_seed);
            println!("PAIRWISE\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}",
                family.name(), name, paired_deltas.len(), paired_delta, paired_lo, paired_hi);
        }

        let joint_index = Kind::ALL.iter().position(|kind| *kind == Kind::QueryKeyedJointNoise).expect("joint-noise policy is declared");
        let joint_cell = &cells[joint_index * CAPS.len() + primary_capacity_index];
        let q_mae_delta = &joint_cell.sensor_mae_delta_vs_prior;
        let q_mae_delta_mean = q_mae_delta.iter().sum::<f64>() / q_mae_delta.len().max(1) as f64;
        let q_mae_seed = mix64(ROOT_SEED ^ 0x515F_4D41_455F_4349
            ^ match family { Family::Development => 0x4445_56, Family::HeldOut => 0x4845_4C44 });
        let (q_mae_delta_lo, q_mae_delta_hi) = bootstrap_ci(q_mae_delta, q_mae_seed);
        println!("PAIRSENSOR\tfamily\tcontrast\tepisodes\tdelta_mae_joint_minus_prior\tbootstrap95_lo\tbootstrap95_hi");
        println!("PAIRSENSOR\t{}\tjoint_minus_uniform_grid_prior\t{}\t{:.4}\t{:.4}\t{:.4}",
            family.name(), q_mae_delta.len(), q_mae_delta_mean, q_mae_delta_lo, q_mae_delta_hi);

        println!("VIS-MEM-002 evaluator-only failure-mode strata (capacity=3; groups may overlap)");
        println!("STRATUM\tfamily\tmode\tlevel\tpolicy\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tdelta_loss_vs_fifo\tbootstrap95_lo_vs_fifo\tbootstrap95_hi_vs_fifo");
        for (stratum_index, stratum_name) in STRATA_NAMES.iter().copied().enumerate() {
            for level in 0..=1 {
                for (policy_index, kind) in Kind::ALL.iter().copied().enumerate() {
                    let index = (stratum_index * 2 + level) * Kind::ALL.len() + policy_index;
                    let cell = &strata_cells[index];
                    if cell.n == 0 { continue; }
                    let (delta, lo, hi) = if policy_index == 0 {
                        (0.0, 0.0, 0.0)
                    } else {
                        let delta = cell.delta_vs_fifo.iter().sum::<f64>()
                            / cell.delta_vs_fifo.len().max(1) as f64;
                        let seed = mix64(ROOT_SEED
                            ^ (stratum_index as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
                            ^ (level as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
                            ^ (policy_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
                        let (lo, hi) = bootstrap_ci(&cell.delta_vs_fifo, seed);
                        (delta, lo, hi)
                    };
                    println!("STRATUM\t{}\t{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.4}\t{:.4}\t{:.4}",
                        family.name(), stratum_name, level == 1, kind.name(), cell.n,
                        cell.mean_loss(), cell.answer_rate(), cell.accuracy(), delta, lo, hi);
                }
            }
        }
    }
    joint_noise_identifiability_report();
}

fn main() { report(); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_is_informative_but_not_a_corruption_label() {
        let confidence_bucket = |value: u16| match value {
            950 => 0,
            850 => 1,
            650 => 2,
            _ => panic!("unexpected confidence tier"),
        };
        let n = 100_000_u64;
        let mut rng = Rng::new(0xC0FF_EE12);
        let mut tier_counts = [0_u64; 3];
        let mut corrupt_counts = [0_u64; 3];

        for _ in 0..n {
            let confidence = sample_confidence(&mut rng);
            let tier = confidence_bucket(confidence);
            tier_counts[tier] += 1;
            if rng.chance(conditional_corruption_rate(60, confidence)) {
                corrupt_counts[tier] += 1;
            }
        }

        assert!(tier_counts.iter().all(|count| *count > 0));
        let rates: [u64; 3] = std::array::from_fn(|i| corrupt_counts[i] * 1_000 / tier_counts[i]);
        assert!(rates[0] < rates[1], "high-confidence observations should be less error-prone");
        assert!(rates[1] < rates[2], "medium-confidence observations should be less error-prone than low-confidence");
        let overall_rate = corrupt_counts.iter().sum::<u64>() * 1_000 / n;
        assert!((45..=75).contains(&overall_rate), "expected aggregate corruption near base rate 60/1000, got {overall_rate}");
    }

    #[test]
    fn seeded_worlds_and_randomness_replay_exactly() {
        assert_eq!(Episode::generate(1234, Family::HeldOut), Episode::generate(1234, Family::HeldOut));
        let mut a = Rng::new(55);
        let mut b = Rng::new(55);
        for _ in 0..64 { assert_eq!(a.next(), b.next()); }
    }

    #[test]
    fn queries_use_three_distinct_valid_claims() {
        for seed in 0..256 {
            let mut keys = Episode::generate(seed, Family::Development).query.keys;
            keys.sort_unstable();
            assert_ne!(keys[0], keys[1]);
            assert_ne!(keys[1], keys[2]);
            assert!(keys.iter().all(|key| *key < CLAIMS));
        }
    }

    #[test]
    fn late_arrival_of_older_source_evidence_does_not_overwrite_newer_claim() {
        let q = Query { keys: [1, 2, 3] };
        let mut m = Memory::new(Kind::QueryKeyed, 3, q, 99);
        m.observe(Evidence { key: 1, value: true, source_tick: 8, arrival_tick: 8, confidence: 950 });
        m.observe(Evidence { key: 1, value: false, source_tick: 3, arrival_tick: 9, confidence: 950 });
        let retained = m.freshest(1).unwrap();
        assert!(retained.value);
        assert_eq!(retained.source_tick, 8);
    }

    #[test]
    fn query_keyed_memory_filters_irrelevant_distractor_claims() {
        let mut m = Memory::new(Kind::QueryKeyed, 3, Query { keys: [1, 2, 3] }, 7);
        m.observe(Evidence { key: 7, value: true, source_tick: 5, arrival_tick: 5, confidence: 950 });
        assert!(m.entries.is_empty());
    }

    #[test]
    fn query_keyed_memory_tracks_three_current_claims_independently() {
        let mut m = Memory::new(Kind::QueryKeyed, 3, Query { keys: [1, 2, 3] }, 7);
        for (key, value) in [(1, true), (2, true), (3, false)] {
            m.observe(Evidence { key, value, source_tick: HORIZON as u8, arrival_tick: HORIZON as u8, confidence: 950 });
        }
        assert_eq!(m.decide(HORIZON), Some(true));
    }

    #[test]
    fn selective_policy_abstains_on_low_confidence_and_stale_claims() {
        let query = Query { keys: [1, 2, 3] };
        let mut memory = Memory::new(Kind::QueryKeyedSelective, 3, query, 7);
        memory.observe(Evidence { key: 1, value: true, source_tick: HORIZON as u8, arrival_tick: HORIZON as u8, confidence: 950 });
        memory.observe(Evidence { key: 2, value: true, source_tick: HORIZON as u8, arrival_tick: HORIZON as u8, confidence: 650 });
        memory.observe(Evidence { key: 3, value: false, source_tick: HORIZON as u8, arrival_tick: HORIZON as u8, confidence: 950 });
        assert_eq!(
            memory.decide_with_threshold(HORIZON, DecisionThreshold { min_confidence: 900, max_age: 7 }),
            None
        );
        assert_eq!(
            memory.decide_with_threshold(HORIZON, DecisionThreshold { min_confidence: 650, max_age: 7 }),
            Some(true)
        );

        let mut stale = Memory::new(Kind::QueryKeyedSelective, 3, query, 7);
        for (key, value) in [(1, true), (2, true), (3, false)] {
            stale.observe(Evidence { key, value, source_tick: 30, arrival_tick: 30, confidence: 950 });
        }
        assert_eq!(
            stale.decide_with_threshold(HORIZON, DecisionThreshold { min_confidence: 350, max_age: 4 }),
            None
        );
    }

    #[test]
    fn hazard_aware_belief_downweights_old_evidence_after_repeated_changes() {
        let query = Query { keys: [1, 2, 3] };
        let mut hazard = Memory::new(Kind::QueryKeyedHazardAware, 3, query, 99);
        let mut ordinary = Memory::new(Kind::QueryKeyed, 3, query, 99);

        // Repeated high-confidence source-time changes estimate elevated
        // transition risk without providing any hidden truth to the policy.
        for tick in (0_u8..=40).step_by(2) {
            let e = Evidence {
                key: 1,
                value: (tick / 2) % 2 == 0,
                source_tick: tick,
                arrival_tick: tick,
                confidence: 950,
            };
            hazard.observe(e);
            ordinary.observe(e);
        }
        for e in [
            Evidence { key: 2, value: true, source_tick: 40, arrival_tick: 40, confidence: 950 },
            Evidence { key: 3, value: false, source_tick: 40, arrival_tick: 40, confidence: 950 },
        ] {
            hazard.observe(e);
            ordinary.observe(e);
        }

        assert!(hazard.hazard_stats[0].transition_count > 0);
        assert_eq!(ordinary.decide(HORIZON), Some(true));
        assert_eq!(hazard.decide_hazard_aware(HORIZON), None);
    }

    #[test]
    fn hazard_policy_reports_its_additional_logical_state_cost() {
        let ep = Episode::generate(0xA2A_D123, Family::HeldOut);
        let simple = evaluate(&ep, Kind::QueryKeyed, 3, 0x1234);
        let hazard = evaluate(&ep, Kind::QueryKeyedHazardAware, 3, 0x1234);
        assert_eq!(simple.entries, hazard.entries);
        assert!(hazard.logical_state_bytes > simple.logical_state_bytes);
    }

    #[test]
    fn hazard_statistics_ignore_late_or_low_confidence_updates() {
        let mut stats = HazardStats::default();
        stats.observe(Evidence { key: 1, value: true, source_tick: 5, arrival_tick: 5, confidence: 950 });
        stats.observe(Evidence { key: 1, value: false, source_tick: 3, arrival_tick: 7, confidence: 950 });
        stats.observe(Evidence { key: 1, value: false, source_tick: 6, arrival_tick: 8, confidence: 350 });
        stats.observe(Evidence { key: 1, value: true, source_tick: 7, arrival_tick: 9, confidence: 950 });
        assert_eq!(stats.transition_count, 0);
        assert_eq!(stats.elapsed_ticks, 2);
        assert_eq!(stats.last_reliable.unwrap().source_tick, 7);
    }

    #[test]
    fn gap_transition_likelihood_normalizes_and_is_monotone() {
        for gap in 1..=8 {
            let mut previous = 0.0;
            for hazard in [0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.4] {
                let p_change = probability_different_after_gap(hazard, gap);
                assert!((0.0..=0.5).contains(&p_change));
                assert!((p_change + (1.0 - p_change) - 1.0).abs() < 1e-12);
                assert!(p_change > previous);
                previous = p_change;
            }
        }
        for hazard in HAZARD_RATES {
            assert!((probability_different_after_gap(hazard, 1) - hazard).abs() < 1e-8);
        }
    }

    #[test]
    fn gap_aware_posterior_accounts_for_irregular_observation_intervals() {
        let mut one_tick = GapAwareStats::default();
        one_tick.observe(Evidence { key: 1, value: true, source_tick: 0, arrival_tick: 0, confidence: 950 });
        one_tick.observe(Evidence { key: 1, value: false, source_tick: 1, arrival_tick: 1, confidence: 950 });

        let mut five_ticks = GapAwareStats::default();
        five_ticks.observe(Evidence { key: 1, value: true, source_tick: 0, arrival_tick: 0, confidence: 950 });
        five_ticks.observe(Evidence { key: 1, value: false, source_tick: 5, arrival_tick: 5, confidence: 950 });

        assert!(one_tick.estimated_flip_hazard() > five_ticks.estimated_flip_hazard());
    }

    #[test]
    fn gap_aware_statistics_ignore_delayed_old_source_updates() {
        let mut stats = GapAwareStats::default();
        stats.observe(Evidence { key: 1, value: true, source_tick: 4, arrival_tick: 4, confidence: 950 });
        stats.observe(Evidence { key: 1, value: false, source_tick: 8, arrival_tick: 8, confidence: 950 });
        let scores_before = stats.log_scores;
        stats.observe(Evidence { key: 1, value: false, source_tick: 6, arrival_tick: 10, confidence: 950 });
        assert_eq!(stats.log_scores, scores_before);
        assert_eq!(stats.last_reliable.unwrap().source_tick, 8);
    }

    #[test]
    fn evaluator_hazard_metrics_are_post_decision_and_policy_scoped() {
        let episode = Episode::generate(0xA11C_E001, Family::HeldOut);
        let fifo = evaluate(&episode, Kind::Fifo, 3, 7);
        let naive = evaluate(&episode, Kind::QueryKeyedHazardAware, 3, 7);
        let gap = evaluate(&episode, Kind::QueryKeyedGapAware, 3, 7);
        assert_eq!(fifo.hazard_error_mae, None);
        assert_eq!(fifo.hazard_error_mse, None);
        assert!(naive.hazard_error_mae.is_some_and(f64::is_finite));
        assert!(naive.hazard_error_mse.is_some_and(f64::is_finite));
        let joint = evaluate(&episode, Kind::QueryKeyedJointNoise, 3, 7);
        assert!(gap.hazard_error_mae.is_some_and(f64::is_finite));
        assert!(gap.hazard_error_mse.is_some_and(f64::is_finite));
        assert!(joint.hazard_error_mae.is_some_and(f64::is_finite));
        assert!(joint.hazard_error_mse.is_some_and(f64::is_finite));
        assert!(naive.hazard_error_mse.unwrap() >= 0.0);
        assert!(gap.hazard_error_mse.unwrap() >= 0.0);
        assert!(joint.hazard_error_mse.unwrap() >= 0.0);
        assert!(joint.sensor_error_mae.is_some_and(f64::is_finite));
        assert!(joint.sensor_error_mse.is_some_and(f64::is_finite));
        assert!(joint.sensor_error_mae.unwrap() >= 0.0);
        assert!(joint.sensor_error_mse.unwrap() >= 0.0);
    }

    #[test]
    fn hidden_hazard_label_changes_evaluator_error_not_policy_decision() {
        let episode = Episode::generate(0xA11C_E002, Family::HeldOut);
        let mut relabeled = episode.clone();
        relabeled.profile.flip_rate = 0;
        relabeled.profile.corruption_rate = 0;

        for kind in [Kind::QueryKeyedGapAware, Kind::QueryKeyedJointNoise] {
            let original = evaluate(&episode, kind, 3, 0xBEEF);
            let changed_flip_label = evaluate(&relabeled, kind, 3, 0xBEEF);
            assert_eq!(original.loss, changed_flip_label.loss);
            assert_eq!(original.answered, changed_flip_label.answered);
            assert_eq!(original.correct, changed_flip_label.correct);
            assert_ne!(original.hazard_error_mae, changed_flip_label.hazard_error_mae);

            if kind == Kind::QueryKeyedJointNoise {
                let mut only_corruption_relabel = episode.clone();
                only_corruption_relabel.profile.corruption_rate = 0;
                let changed_q_label = evaluate(&only_corruption_relabel, kind, 3, 0xBEEF);
                assert_eq!(original.loss, changed_q_label.loss);
                assert_eq!(original.answered, changed_q_label.answered);
                assert_eq!(original.correct, changed_q_label.correct);
                assert_ne!(original.sensor_error_mae, changed_q_label.sensor_error_mae);
            }
        }
    }

    #[test]
    fn joint_noise_filter_is_bounded_and_emits_valid_probabilities() {
        let query = Query { keys: [1, 2, 3] };
        let mut memory = Memory::new(Kind::QueryKeyedJointNoise, 3, query, 7);
        for evidence in [
            Evidence { key: 1, value: true, source_tick: 2, arrival_tick: 2, confidence: 950 },
            Evidence { key: 2, value: false, source_tick: 3, arrival_tick: 3, confidence: 850 },
            Evidence { key: 3, value: true, source_tick: 4, arrival_tick: 4, confidence: 650 },
        ] {
            memory.observe(evidence);
        }

        for stats in memory.joint_noise_stats {
            assert!(stats.estimated_flip_hazard().is_finite());
            assert!((0.0..=0.5).contains(&stats.estimated_flip_hazard()));
            for tick in [0, 4, HORIZON] {
                let p = stats.probability_true_at(tick);
                assert!(p.is_finite());
                assert!((0.0..=1.0).contains(&p));
            }
        }
        let probabilities: [f64; QUERY_WIDTH] = std::array::from_fn(|index| {
            memory.joint_noise_stats[index].probability_true_at(HORIZON)
        });
        assert_eq!(memory.decide_joint_noise(HORIZON), majority_decision(probabilities));
    }

    #[test]
    fn joint_noise_filter_ignores_out_of_order_evidence_without_rewinding() {
        let mut stats = JointNoiseStats::default();
        stats.observe(Evidence { key: 1, value: true, source_tick: 4, arrival_tick: 4, confidence: 950 });
        stats.observe(Evidence { key: 1, value: false, source_tick: 8, arrival_tick: 8, confidence: 850 });
        let previous_tick = stats.last_source_tick;
        let scores_before = stats.log_scores;
        let state_before = stats.alpha_true;
        stats.observe(Evidence { key: 1, value: false, source_tick: 6, arrival_tick: 10, confidence: 950 });
        assert_eq!(stats.last_source_tick, previous_tick);
        assert_eq!(stats.log_scores, scores_before);
        assert_eq!(stats.alpha_true, state_before);
    }

    #[test]
    fn joint_noise_policy_reports_larger_logical_state_cost() {
        let episode = Episode::generate(0xCAFE_BA22, Family::HeldOut);
        let hazard = evaluate(&episode, Kind::QueryKeyedHazardAware, 3, 0x1234);
        let joint = evaluate(&episode, Kind::QueryKeyedJointNoise, 3, 0x1234);
        assert_eq!(hazard.entries, joint.entries);
        assert!(joint.logical_state_bytes > hazard.logical_state_bytes);
        assert!(joint.hazard_error_mae.is_some_and(f64::is_finite));
        assert!(joint.hazard_error_mse.is_some_and(f64::is_finite));
    }

    #[test]
    fn gap_aware_policy_reports_larger_logical_state_cost() {
        let episode = Episode::generate(0xCAFE_BABE, Family::HeldOut);
        let simple = evaluate(&episode, Kind::QueryKeyedHazardAware, 3, 0x1234);
        let gap_aware = evaluate(&episode, Kind::QueryKeyedGapAware, 3, 0x1234);
        assert_eq!(simple.entries, gap_aware.entries);
        assert!(gap_aware.logical_state_bytes > simple.logical_state_bytes);
    }

    #[test]
    fn development_calibration_is_deterministic_and_uses_declared_grid() {
        let a = calibrate_threshold();
        let b = calibrate_threshold();
        assert_eq!(a, b);
        assert!([650, 900].contains(&a.min_confidence));
        assert!([2, 4, FRESHNESS_LIMIT].contains(&a.max_age));
    }

    #[test]
    fn evaluator_strata_flag_drift_out_of_order_confidence_and_delay() {
        let query = Query { keys: [1, 2, 3] };
        let mut truth = vec![[false; CLAIMS]; HORIZON + 1];
        for tick in 6..=HORIZON { truth[tick][1] = true; }
        let stream = vec![
            Evidence { key: 1, value: false, source_tick: 5, arrival_tick: 5, confidence: 950 },
            Evidence { key: 1, value: false, source_tick: 2, arrival_tick: 6, confidence: 950 },
            Evidence { key: 2, value: false, source_tick: 3, arrival_tick: 6, confidence: 350 },
            Evidence { key: 3, value: false, source_tick: 40, arrival_tick: 47, confidence: 950 },
        ];
        let profile = NuisanceProfile {
            distractor_rate: 350,
            flip_rate: 25,
            max_delay: 2,
            corruption_rate: 20,
            high_distractor_rate: true,
            high_flip_rate: true,
            high_max_delay: true,
            high_corruption_rate: true,
        };
        let episode = Episode { query, profile, truth, stream };
        assert_eq!(episode.evaluator_strata(), [true; STRATA_COUNT]);
    }

    #[test]
    fn every_policy_respects_capacity_and_deterministic_replay() {
        let ep = Episode::generate(0xCAFE_BABE, Family::HeldOut);
        for kind in Kind::ALL {
            for cap in CAPS {
                let a = evaluate(&ep, kind, cap, 0x1234);
                let b = evaluate(&ep, kind, cap, 0x1234);
                assert_eq!(a.loss, b.loss);
                assert!(a.entries <= cap);
            }
        }
    }

    #[test]
    fn per_episode_nuisances_vary_independently_within_each_family() {
        for family in [Family::Development, Family::HeldOut] {
            let profiles: Vec<NuisanceProfile> = (0..256)
                .map(|seed| {
                    let mut rng = Rng::new(seed);
                    family.sample_profile(&mut rng)
                })
                .collect();
            assert!(profiles.iter().map(|p| p.distractor_rate).collect::<std::collections::BTreeSet<_>>().len() > 1);
            assert!(profiles.iter().map(|p| p.flip_rate).collect::<std::collections::BTreeSet<_>>().len() > 1);
            assert!(profiles.iter().map(|p| p.max_delay).collect::<std::collections::BTreeSet<_>>().len() > 1);
            assert!(profiles.iter().map(|p| p.corruption_rate).collect::<std::collections::BTreeSet<_>>().len() > 1);
        }
        let mut dev_rng = Rng::new(123);
        let dev = Family::Development.sample_profile(&mut dev_rng);
        let mut held_rng = Rng::new(123);
        let held = Family::HeldOut.sample_profile(&mut held_rng);
        assert!(held.distractor_rate >= dev.distractor_rate);
        assert!(held.flip_rate >= dev.flip_rate);
        assert!(held.max_delay >= dev.max_delay);
        assert!(held.corruption_rate >= dev.corruption_rate);
    }

    #[test]
    fn bootstrap_is_deterministic_and_has_ordered_bounds() {
        let vals = [-0.2, -0.1, 0.0, 0.1, 0.05, -0.15];
        assert_eq!(bootstrap_ci(&vals, 42), bootstrap_ci(&vals, 42));
        assert!(bootstrap_ci(&vals, 42).0 <= bootstrap_ci(&vals, 42).1);
    }
}
