// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-009: causal memory-reliance and invariance diagnostic.
//!
//! Dependency-free synthetic mechanism test. The acting policy receives only
//! a public query/current observation and observation history. Evaluator truth
//! is used after acting and is never part of the policy input.
//!
//! rustc --edition=2024 --test tools/vision-memory-intervention-bench.rs -o /tmp/vis-mem-intervention-tests
//! /tmp/vis-mem-intervention-tests
//! rustc --edition=2024 -O tools/vision-memory-intervention-bench.rs -o /tmp/vis-mem-intervention-bench
//! /tmp/vis-mem-intervention-bench > vis-mem-interventions.tsv
//!
//! This diagnostic is not learned-VLA evidence or robotic task success. Its
//! purpose is to test that intervention plumbing can detect useful dependence
//! on relevant history and invariance to irrelevant/stale history.

use std::env;

const KEYS: usize = 8;
const DISTRACTORS: usize = 8;
const EPISODES: usize = 128;
const BOOTSTRAPS: usize = 2_000;
const ROOT_SEED: u64 = 0x510E_527F_ADE6_82D1;

fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix64(self.0)
    }

    fn range(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next() % upper as u64) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Observation {
    id: u64,
    key: u8,
    value: bool,
    source_tick: u16,
    arrival_tick: u16,
    session: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CurrentObservation {
    query_key: u8,
    cue_visible: bool,
    cue_value: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Episode {
    query_key: u8,
    // Evaluator-only answer. The policy_decision function cannot receive it.
    truth: bool,
    critical_id: u64,
    history: Vec<Observation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    NoMemory,
    GlobalLast,
    LatestByArrival,
    LatestBySourceTime,
}

impl Policy {
    const ALL: [Self; 4] = [
        Self::NoMemory,
        Self::GlobalLast,
        Self::LatestByArrival,
        Self::LatestBySourceTime,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::NoMemory => "no_memory",
            Self::GlobalLast => "global_last",
            Self::LatestByArrival => "keyed_latest_arrival",
            Self::LatestBySourceTime => "keyed_latest_source_time",
        }
    }
}

fn make_episode(seed: u64) -> Episode {
    let mut rng = Rng::new(seed);
    let query_key = rng.range(KEYS) as u8;
    let truth = rng.next() & 1 != 0;
    let critical_id = mix64(seed ^ 0xA5A5_5A5A_0123_4567);
    let old_id = mix64(seed ^ 0x5A5A_A5A5_7654_3210);
    let mut history = Vec::with_capacity(DISTRACTORS + 2);

    // Old, incorrect value is observed first; the critical record supersedes it.
    history.push(Observation {
        id: old_id,
        key: query_key,
        value: !truth,
        source_tick: 1,
        arrival_tick: 2,
        session: 0,
    });
    history.push(Observation {
        id: critical_id,
        key: query_key,
        value: truth,
        source_tick: 8,
        arrival_tick: 8,
        session: 1,
    });

    // Unrelated sessions/keys arrive after the relevant record. Their values
    // carry no answer to the public query and should not alter keyed retrieval.
    for i in 0..DISTRACTORS {
        let offset = 1 + rng.range(KEYS - 1);
        let key = ((query_key as usize + offset) % KEYS) as u8;
        let tick = 9 + i as u16;
        history.push(Observation {
            id: mix64(seed ^ ((i as u64 + 1) << 32) ^ 0x1357_9BDF),
            key,
            value: rng.next() & 1 != 0,
            source_tick: tick,
            arrival_tick: tick,
            session: (i % 7 + 2) as u8,
        });
    }

    Episode { query_key, truth, critical_id, history }
}

fn relevant_history_swap(episode: &Episode) -> Vec<Observation> {
    let mut history = episode.history.clone();
    let critical = history
        .iter_mut()
        .find(|o| o.id == episode.critical_id)
        .expect("synthetic episode must contain critical record");
    critical.value = !critical.value;
    history
}

fn insert_irrelevant_session(episode: &Episode) -> Vec<Observation> {
    let mut history = episode.history.clone();
    let last = *history.last().expect("synthetic history must be nonempty");
    let offset = 1 + (episode.query_key as usize % (KEYS - 1));
    let key = ((episode.query_key as usize + offset) % KEYS) as u8;
    history.push(Observation {
        id: mix64(last.id ^ 0xD1B5_4A32_D192_ED03),
        key,
        value: !last.value,
        source_tick: last.source_tick + 10,
        arrival_tick: last.arrival_tick + 10,
        session: 250,
    });
    history
}

fn insert_stale_late_arrival(episode: &Episode) -> Vec<Observation> {
    let mut history = episode.history.clone();
    let last_arrival = history.iter().map(|o| o.arrival_tick).max().unwrap_or(0);
    history.push(Observation {
        id: mix64(episode.critical_id ^ 0xC0AC_29B7_C97C_50DD),
        key: episode.query_key,
        value: !episode.truth,
        source_tick: 7, // older than the valid source revision at tick 8
        arrival_tick: last_arrival + 10, // but it arrives last
        session: 251,
    });
    history
}

fn policy_decision(policy: Policy, current: CurrentObservation, history: &[Observation]) -> bool {
    // An actually visible cue must override memory. This is the reactive negative control.
    if current.cue_visible {
        return current.cue_value;
    }

    match policy {
        Policy::NoMemory => false,
        Policy::GlobalLast => history.last().map(|o| o.value).unwrap_or(false),
        Policy::LatestByArrival => history
            .iter()
            .filter(|o| o.key == current.query_key)
            .max_by_key(|o| (o.arrival_tick, o.id))
            .map(|o| o.value)
            .unwrap_or(false),
        Policy::LatestBySourceTime => history
            .iter()
            .filter(|o| o.key == current.query_key)
            .max_by_key(|o| (o.source_tick, o.id))
            .map(|o| o.value)
            .unwrap_or(false),
    }
}

#[derive(Clone, Debug)]
struct EpisodeResult {
    policy: Policy,
    baseline_correct: f64,
    paired_benefit_vs_knockout: f64,
    relevant_sensitive: f64,
    relevant_counterfactual_correct: f64,
    irrelevant_invariant: f64,
    stale_rejected: f64,
    knockout_correct: f64,
    reactive_correct: f64,
    reactive_history_invariant: f64,
}

fn evaluate_episode(episode: &Episode, policy: Policy) -> EpisodeResult {
    let hidden_current = CurrentObservation {
        query_key: episode.query_key,
        cue_visible: false,
        cue_value: false,
    };
    let baseline_action = policy_decision(policy, hidden_current, &episode.history);
    let baseline_correct = baseline_action == episode.truth;

    let swapped_history = relevant_history_swap(episode);
    let swapped_action = policy_decision(policy, hidden_current, &swapped_history);
    let relevant_sensitive = swapped_action != baseline_action;
    // Counterfactual truth also flips because the single critical prior fact is
    // the task-defining cue in this synthetic episode.
    let relevant_counterfactual_correct = swapped_action == !episode.truth;

    let irrelevant_history = insert_irrelevant_session(episode);
    let irrelevant_action = policy_decision(policy, hidden_current, &irrelevant_history);
    let irrelevant_invariant = irrelevant_action == baseline_action;

    let stale_history = insert_stale_late_arrival(episode);
    let stale_action = policy_decision(policy, hidden_current, &stale_history);
    let stale_rejected = stale_action == baseline_action;

    let knockout_action = policy_decision(policy, hidden_current, &[]);
    let knockout_correct = knockout_action == episode.truth;

    let visible_current = CurrentObservation {
        query_key: episode.query_key,
        cue_visible: true,
        cue_value: episode.truth,
    };
    let reactive_action = policy_decision(policy, visible_current, &episode.history);
    let reactive_correct = reactive_action == episode.truth;
    let reactive_intervened = policy_decision(policy, visible_current, &stale_history);
    let reactive_history_invariant = reactive_intervened == reactive_action;

    EpisodeResult {
        policy,
        baseline_correct: if baseline_correct { 1.0 } else { 0.0 },
        paired_benefit_vs_knockout: (if baseline_correct { 1.0 } else { 0.0 })
            - (if knockout_correct { 1.0 } else { 0.0 }),
        relevant_sensitive: if relevant_sensitive { 1.0 } else { 0.0 },
        relevant_counterfactual_correct: if relevant_counterfactual_correct { 1.0 } else { 0.0 },
        irrelevant_invariant: if irrelevant_invariant { 1.0 } else { 0.0 },
        stale_rejected: if stale_rejected { 1.0 } else { 0.0 },
        knockout_correct: if knockout_correct { 1.0 } else { 0.0 },
        reactive_correct: if reactive_correct { 1.0 } else { 0.0 },
        reactive_history_invariant: if reactive_history_invariant { 1.0 } else { 0.0 },
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.iter().sum::<f64>() / values.len() as f64
}

fn bootstrap_ci(values: &[f64], seed: u64) -> (f64, f64) {
    if values.is_empty() { return (0.0, 0.0); }
    let mut rng = Rng::new(seed);
    let mut draws = Vec::with_capacity(BOOTSTRAPS);
    for _ in 0..BOOTSTRAPS {
        let mut sum = 0.0;
        for _ in 0..values.len() {
            sum += values[rng.range(values.len())];
        }
        draws.push(sum / values.len() as f64);
    }
    draws.sort_by(f64::total_cmp);
    let lo = ((BOOTSTRAPS - 1) as f64 * 0.025).floor() as usize;
    let hi = ((BOOTSTRAPS - 1) as f64 * 0.975).floor() as usize;
    (draws[lo], draws[hi])
}

fn run_report() {
    let episodes: Vec<Episode> = (0..EPISODES)
        .map(|i| make_episode(mix64(ROOT_SEED ^ i as u64)))
        .collect();
    println!("META\tformat\tVIS-MEM-009-v0.1");
    println!("META\troot_seed\t{ROOT_SEED:016x}");
    println!("META\tepisodes\t{EPISODES}");
    println!("META\tpolicy_inputs\tpublic query, current observation, released history only");
    println!("META\tevaluator_truth\tnever passed to policy");
    println!("META\tscope\tsynthetic causal mechanism diagnostic, not VLA success");
    println!("METRIC_HEADER\tpolicy\tepisodes\ttask_success\tpaired_benefit_vs_knockout\tci95_low\tci95_high\trelevant_sensitivity\trelevant_counterfactual_accuracy\tirrelevant_invariance\tstale_rejection\tknockout_accuracy\treactive_success\treactive_history_invariance");

    for policy in Policy::ALL {
        let rows: Vec<EpisodeResult> = episodes.iter().map(|e| evaluate_episode(e, policy)).collect();
        let baseline = rows.iter().map(|r| r.baseline_correct).collect::<Vec<_>>();
        let benefit = rows.iter().map(|r| r.paired_benefit_vs_knockout).collect::<Vec<_>>();
        let sensitivity = rows.iter().map(|r| r.relevant_sensitive).collect::<Vec<_>>();
        let counterfactual = rows.iter().map(|r| r.relevant_counterfactual_correct).collect::<Vec<_>>();
        let irrelevant = rows.iter().map(|r| r.irrelevant_invariant).collect::<Vec<_>>();
        let stale = rows.iter().map(|r| r.stale_rejected).collect::<Vec<_>>();
        let knockout = rows.iter().map(|r| r.knockout_correct).collect::<Vec<_>>();
        let reactive = rows.iter().map(|r| r.reactive_correct).collect::<Vec<_>>();
        let reactive_invariant = rows.iter().map(|r| r.reactive_history_invariant).collect::<Vec<_>>();
        let (lo, hi) = bootstrap_ci(&benefit, ROOT_SEED ^ policy.name().len() as u64);

        println!(
            "METRIC\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
            policy.name(),
            EPISODES,
            mean(&baseline),
            mean(&benefit),
            lo,
            hi,
            mean(&sensitivity),
            mean(&counterfactual),
            mean(&irrelevant),
            mean(&stale),
            mean(&knockout),
            mean(&reactive),
            mean(&reactive_invariant)
        );
    }
}

fn main() {
    if env::args().any(|arg| arg == "--help") {
        println!("VIS-MEM-009 deterministic causal memory intervention diagnostic");
        println!("Usage: vis-mem-intervention-bench [--help]");
        return;
    }
    run_report();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Episode {
        make_episode(0x1234_5678_9ABC_DEF0)
    }

    #[test]
    fn source_time_policy_uses_relevant_historical_fact() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        assert_eq!(policy_decision(Policy::LatestBySourceTime, current, &e.history), e.truth);
    }

    #[test]
    fn relevant_fact_intervention_changes_the_source_time_action() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        let before = policy_decision(Policy::LatestBySourceTime, current, &e.history);
        let after = policy_decision(Policy::LatestBySourceTime, current, &relevant_history_swap(&e));
        assert_eq!(after, !before);
    }

    #[test]
    fn keyed_source_policy_is_invariant_to_irrelevant_session() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        assert_eq!(
            policy_decision(Policy::LatestBySourceTime, current, &e.history),
            policy_decision(Policy::LatestBySourceTime, current, &insert_irrelevant_session(&e))
        );
    }

    #[test]
    fn global_last_is_detectably_sensitive_to_irrelevant_history() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        let before = policy_decision(Policy::GlobalLast, current, &e.history);
        let after = policy_decision(Policy::GlobalLast, current, &insert_irrelevant_session(&e));
        assert_eq!(after, !before);
    }

    #[test]
    fn source_time_policy_rejects_late_arriving_stale_revision() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        assert_eq!(
            policy_decision(Policy::LatestBySourceTime, current, &e.history),
            policy_decision(Policy::LatestBySourceTime, current, &insert_stale_late_arrival(&e))
        );
    }

    #[test]
    fn arrival_order_policy_is_detectably_vulnerable_to_late_stale_revision() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        let before = policy_decision(Policy::LatestByArrival, current, &e.history);
        let after = policy_decision(Policy::LatestByArrival, current, &insert_stale_late_arrival(&e));
        assert_eq!(after, !before);
    }

    #[test]
    fn visible_reactive_cue_dominates_all_history_interventions() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: true, cue_value: e.truth };
        for policy in Policy::ALL {
            assert_eq!(policy_decision(policy, current, &e.history), e.truth);
            assert_eq!(policy_decision(policy, current, &relevant_history_swap(&e)), e.truth);
            assert_eq!(policy_decision(policy, current, &insert_irrelevant_session(&e)), e.truth);
            assert_eq!(policy_decision(policy, current, &insert_stale_late_arrival(&e)), e.truth);
        }
    }


    #[test]
    fn source_time_metrics_show_relevant_use_and_irrelevant_invariance() {
        let result = evaluate_episode(&sample(), Policy::LatestBySourceTime);
        assert_eq!(result.baseline_correct, 1.0);
        assert_eq!(result.relevant_sensitive, 1.0);
        assert_eq!(result.relevant_counterfactual_correct, 1.0);
        assert_eq!(result.irrelevant_invariant, 1.0);
        assert_eq!(result.stale_rejected, 1.0);
        assert_eq!(result.reactive_correct, 1.0);
        assert_eq!(result.reactive_history_invariant, 1.0);
    }

    #[test]
    fn no_memory_metric_does_not_fake_relevant_sensitivity() {
        let result = evaluate_episode(&sample(), Policy::NoMemory);
        assert_eq!(result.relevant_sensitive, 0.0);
        assert_eq!(result.irrelevant_invariant, 1.0);
        assert_eq!(result.stale_rejected, 1.0);
        assert_eq!(result.reactive_correct, 1.0);
    }

    #[test]
    fn arrival_order_metric_fails_stale_rejection() {
        let result = evaluate_episode(&sample(), Policy::LatestByArrival);
        assert_eq!(result.baseline_correct, 1.0);
        assert_eq!(result.stale_rejected, 0.0);
    }

    #[test]
    fn global_last_metric_fails_irrelevant_invariance() {
        let result = evaluate_episode(&sample(), Policy::GlobalLast);
        assert_eq!(result.relevant_sensitive, 0.0);
        assert_eq!(result.irrelevant_invariant, 0.0);
    }

    #[test]
    fn relabeling_evaluator_truth_does_not_change_policy_output() {
        let e = sample();
        let current = CurrentObservation { query_key: e.query_key, cue_visible: false, cue_value: false };
        let relabeled = Episode { truth: !e.truth, ..e.clone() };
        for policy in Policy::ALL {
            assert_eq!(
                policy_decision(policy, current, &e.history),
                policy_decision(policy, current, &relabeled.history)
            );
        }
    }

    #[test]
    fn episode_generation_and_metrics_are_deterministic() {
        let a = make_episode(998877);
        let b = make_episode(998877);
        assert_eq!(a, b);
        let ra = evaluate_episode(&a, Policy::LatestBySourceTime);
        let rb = evaluate_episode(&b, Policy::LatestBySourceTime);
        assert_eq!(ra.policy, rb.policy);
        assert_eq!(ra.paired_benefit_vs_knockout, rb.paired_benefit_vs_knockout);
        assert_eq!(ra.relevant_counterfactual_correct, rb.relevant_counterfactual_correct);
    }
}
