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
const ROOT_SEED: u64 = 0x6A09_E667_F3BC_C909;
const CAPS: [usize; 5] = [1, 2, 3, 4, 8];

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
    fn distractor_rate(self) -> u64 {
        match self { Self::Development => 350, Self::HeldOut => 600 }
    }
    fn flip_rate(self) -> u64 {
        match self { Self::Development => 25, Self::HeldOut => 65 }
    }
    fn max_delay(self) -> usize {
        match self { Self::Development => 2, Self::HeldOut => 5 }
    }
    fn corruption_rate(self) -> u64 {
        match self { Self::Development => 20, Self::HeldOut => 70 }
    }
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
    fn rank(self) -> (u8, u16, u8) { (self.source_tick, self.confidence, self.arrival_tick) }
    fn newer_than(self, old: Self) -> bool { self.rank() > old.rank() }
    fn fresh_at(self, tick: usize) -> bool {
        tick.saturating_sub(usize::from(self.source_tick)) <= FRESHNESS_LIMIT
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Episode {
    query: Query,
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

        let mut truth = vec![[false; CLAIMS]; HORIZON + 1];
        for k in 0..CLAIMS { truth[0][k] = rng.chance(500); }
        for tick in 1..=HORIZON {
            truth[tick] = truth[tick - 1];
            for k in 0..CLAIMS {
                if rng.chance(family.flip_rate()) { truth[tick][k] = !truth[tick][k]; }
            }
        }

        let mut stream = Vec::with_capacity(HORIZON + 1);
        for arrival in 0..=HORIZON {
            let (key, source) = if arrival < QUERY_WIDTH {
                (query.keys[arrival], 0)
            } else {
                let key = if rng.chance(family.distractor_rate()) {
                    hot_distractor
                } else {
                    query.keys[rng.range(QUERY_WIDTH)]
                };
                let delay = rng.range(family.max_delay() + 1);
                (key, arrival.saturating_sub(delay))
            };
            let corrupted = rng.chance(family.corruption_rate());
            let mut value = truth[source][key];
            if corrupted { value = !value; }
            stream.push(Evidence {
                key: key as u8,
                value,
                source_tick: source as u8,
                arrival_tick: arrival as u8,
                confidence: if corrupted { 350 } else { 950 },
            });
        }
        Self { query, truth, stream }
    }

    fn target_answer(&self) -> bool {
        self.query.keys.iter().filter(|key| self.truth[HORIZON][**key]).count() >= 2
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind { Fifo, Reservoir, QueryKeyed, QueryKeyedSelective }

impl Kind {
    const ALL: [Self; 4] = [
        Self::Fifo,
        Self::Reservoir,
        Self::QueryKeyed,
        Self::QueryKeyedSelective,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::Reservoir => "uniform_reservoir",
            Self::QueryKeyed => "query_keyed_latest",
            Self::QueryKeyedSelective => "query_keyed_selective",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecisionThreshold {
    min_confidence: u16,
    max_age: usize,
}

#[derive(Clone, Debug)]
struct Memory {
    kind: Kind,
    cap: usize,
    query: Query,
    entries: Vec<Evidence>,
    seen: u64,
    actor_seed: u64,
}

impl Memory {
    fn new(kind: Kind, cap: usize, query: Query, actor_seed: u64) -> Self {
        Self { kind, cap, query, entries: Vec::with_capacity(cap), seen: 0, actor_seed }
    }

    fn observe(&mut self, e: Evidence) {
        if self.cap == 0 || e.key() >= CLAIMS { return; }
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
            Kind::QueryKeyed | Kind::QueryKeyedSelective => {
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
}

#[derive(Clone, Copy, Debug)]
struct Outcome { loss: f64, answered: bool, correct: bool, fresh_claims: usize, entries: usize }

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
    let decision = if kind == Kind::QueryKeyedSelective {
        memory.decide_with_threshold(HORIZON, threshold)
    } else {
        memory.decide(HORIZON)
    };
    let correct = decision.is_some_and(|answer| answer == ep.target_answer());
    Outcome {
        loss: match decision { None => 0.25, Some(_) if correct => 0.0, Some(_) => 1.0 },
        answered: decision.is_some(),
        correct,
        fresh_claims: memory.fresh_claims(HORIZON),
        entries: memory.entries.len(),
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
        if let Some(d) = delta { self.delta_vs_fifo.push(d); }
    }
    fn mean_loss(&self) -> f64 { self.loss_sum / self.n.max(1) as f64 }
    fn answer_rate(&self) -> f64 { self.answered as f64 / self.n.max(1) as f64 }
    fn accuracy(&self) -> String {
        if self.answered == 0 { "NA".to_string() }
        else { format!("{:.4}", self.correct as f64 / self.answered as f64) }
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
    let confidence_grid = [350_u16, 900_u16];
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

fn report() {
    let selective_threshold = calibrate_threshold();
    println!("VIS-MEM-002 causal multi-claim evidence-memory benchmark");
    println!("selective threshold calibrated on {CALIBRATION_EPISODES} separate development episodes: min_confidence={}, max_age={}",
        selective_threshold.min_confidence, selective_threshold.max_age);
    println!("8 claims; public majority query over 3; 48 revealed events; source time can differ from arrival time");
    println!("paired policies, correlated distractor bursts, changing claims, delayed and occasionally corrupted evidence");
    println!("loss: correct=0, abstain=0.25, incorrect=1; intervals are paired episode bootstrap vs FIFO");
    println!("family\tpolicy\tcapacity\tepisodes\tmean_loss\tanswer_rate\taccuracy_when_answered\tfresh_query_claim_coverage\tmean_memory_entries\tdelta_loss_vs_fifo\tbootstrap95_lo_vs_fifo\tbootstrap95_hi_vs_fifo");

    for family in [Family::Development, Family::HeldOut] {
        let mut cells = vec![Acc::default(); Kind::ALL.len() * CAPS.len()];
        for rep in 0..REPLICATES {
            for pair in 0..PAIRS {
                let seed = seed_for(rep, pair, family);
                let ep = Episode::generate(seed, family);
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
                println!("{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{:.4}\t{:.3}\t{:.4}\t{:.4}\t{:.4}",
                    family.name(), kind.name(), cap, cell.n, cell.mean_loss(), cell.answer_rate(),
                    cell.accuracy(), coverage, mean_entries, delta, lo, hi);
            }
        }
    }
}

fn main() { report(); }

#[cfg(test)]
mod tests {
    use super::*;

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
        memory.observe(Evidence { key: 2, value: true, source_tick: HORIZON as u8, arrival_tick: HORIZON as u8, confidence: 350 });
        memory.observe(Evidence { key: 3, value: false, source_tick: HORIZON as u8, arrival_tick: HORIZON as u8, confidence: 950 });
        assert_eq!(
            memory.decide_with_threshold(HORIZON, DecisionThreshold { min_confidence: 900, max_age: 7 }),
            None
        );
        assert_eq!(
            memory.decide_with_threshold(HORIZON, DecisionThreshold { min_confidence: 350, max_age: 7 }),
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
    fn development_calibration_is_deterministic_and_uses_declared_grid() {
        let a = calibrate_threshold();
        let b = calibrate_threshold();
        assert_eq!(a, b);
        assert!([350, 900].contains(&a.min_confidence));
        assert!([2, 4, FRESHNESS_LIMIT].contains(&a.max_age));
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
    fn held_out_profile_changes_every_declared_nuisance() {
        assert!(Family::HeldOut.distractor_rate() > Family::Development.distractor_rate());
        assert!(Family::HeldOut.flip_rate() > Family::Development.flip_rate());
        assert!(Family::HeldOut.max_delay() > Family::Development.max_delay());
        assert!(Family::HeldOut.corruption_rate() > Family::Development.corruption_rate());
    }

    #[test]
    fn bootstrap_is_deterministic_and_has_ordered_bounds() {
        let vals = [-0.2, -0.1, 0.0, 0.1, 0.05, -0.15];
        assert_eq!(bootstrap_ci(&vals, 42), bootstrap_ci(&vals, 42));
        assert!(bootstrap_ci(&vals, 42).0 <= bootstrap_ci(&vals, 42).1);
    }
}
