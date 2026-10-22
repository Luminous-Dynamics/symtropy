// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-021: retention-value calibration under hidden future demand.
//!
//! A paired synthetic diagnostic asking whether an online eviction signal
//! preserves records that improve later decisions. The historical write/query
//! stream is visible to the policy. The future query distribution and the
//! utility scorer are used only after the memory snapshot is frozen.
//!
//! This is a mechanism study, not native robotics evidence or an SOTA claim.
//!
//! rustc --edition=2024 --test tools/vision-memory-retention-value-drift.rs -o /tmp/vis-mem-021-tests
//! /tmp/vis-mem-021-tests
//! rustc --edition=2024 -O tools/vision-memory-retention-value-drift.rs -o /tmp/vis-mem-021
//! /tmp/vis-mem-021 > vis-mem-021.tsv

const SEEDS: usize = 24;
const ONLINE_STEPS: usize = 240;
const FUTURE_QUERIES: usize = 128;
const BUDGETS: [usize; 3] = [3, 6, 9];
const KEY_COUNT: usize = 12;
const SLOT_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Workload {
    Stationary,
    DemandShift,
    RareCritical,
    DistractorBurst,
}

impl Workload {
    const ALL: [Self; 4] = [
        Self::Stationary,
        Self::DemandShift,
        Self::RareCritical,
        Self::DistractorBurst,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Stationary => "stationary",
            Self::DemandShift => "demand_shift",
            Self::RareCritical => "rare_critical",
            Self::DistractorBurst => "distractor_burst",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    Fifo,
    Lru,
    Lfu,
    Rarity,
    RiskOnly,
    RiskAwareAccess,
}

impl Policy {
    const ALL: [Self; 6] = [
        Self::Fifo,
        Self::Lru,
        Self::Lfu,
        Self::Rarity,
        Self::RiskOnly,
        Self::RiskAwareAccess,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::Lru => "lru",
            Self::Lfu => "lfu",
            Self::Rarity => "rarity_by_write_frequency",
            Self::RiskOnly => "source_risk_only",
            Self::RiskAwareAccess => "source_risk_plus_access",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    key: u8,
    revision: u16,
    write_step: u32,
    last_access_step: u32,
    access_count: u32,
    writes_while_resident: u32,
    risk_band: u8,
}

impl Entry {
    fn new(key: u8, revision: u16, step: u32) -> Self {
        Self {
            key,
            revision,
            write_step: step,
            last_access_step: 0,
            access_count: 0,
            writes_while_resident: 1,
            risk_band: source_risk_band(key),
        }
    }
}

// Source-side risk metadata is observable when an item is written. It describes
// consequence severity, not which key the evaluator will query in the future.
fn source_risk_band(key: u8) -> u8 {
    match key {
        2 => 3,
        11 => 5,
        _ => 1,
    }
}

// Evaluator-side action utility intentionally has a different interface from
// the selector. A hit on a higher-consequence fact matters more to the score.
fn evaluator_impact(key: u8) -> i64 {
    match key {
        2 => 3,
        11 => 5,
        _ => 1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Write { key: u8, revision: u16 },
    Query { key: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FutureQuery {
    key: u8,
    truth_revision: u16,
}

#[derive(Clone, Copy, Debug)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // xorshift64* requires a non-zero state.
        Self(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next() % upper as u64) as usize
    }
}

fn query_weights(workload: Workload, future: bool) -> [u32; KEY_COUNT] {
    let mut weights = [1_u32; KEY_COUNT];
    match (workload, future) {
        (Workload::Stationary, _) => {
            for (idx, weight) in weights.iter_mut().enumerate() {
                *weight = match idx {
                    0..=3 => 8,
                    4..=7 => 4,
                    _ => 1,
                };
            }
        }
        (Workload::DemandShift, false) => {
            for (idx, weight) in weights.iter_mut().enumerate() {
                *weight = match idx {
                    0..=3 => 9,
                    4..=7 => 3,
                    _ => 1,
                };
            }
        }
        (Workload::DemandShift, true) => {
            for (idx, weight) in weights.iter_mut().enumerate() {
                *weight = match idx {
                    0..=3 => 1,
                    4..=7 => 3,
                    _ => 9,
                };
            }
        }
        (Workload::RareCritical, false) => {
            for (idx, weight) in weights.iter_mut().enumerate() {
                *weight = match idx {
                    0..=3 => 7,
                    4..=7 => 4,
                    _ => 1,
                };
            }
        }
        (Workload::RareCritical, true) => {
            for (idx, weight) in weights.iter_mut().enumerate() {
                *weight = match idx {
                    0..=3 => 5,
                    4..=7 => 4,
                    8..=10 => 1,
                    11 => 8,
                    _ => unreachable!(),
                };
            }
        }
        (Workload::DistractorBurst, _) => {
            for (idx, weight) in weights.iter_mut().enumerate() {
                *weight = match idx {
                    0..=3 => 8,
                    4..=7 => 4,
                    _ => 1,
                };
            }
        }
    }
    weights
}

fn sample_key(rng: &mut Rng, weights: &[u32; KEY_COUNT]) -> u8 {
    let total: u32 = weights.iter().sum();
    let mut draw = (rng.next() % u64::from(total)) as u32;
    for (idx, weight) in weights.iter().enumerate() {
        if draw < *weight {
            return idx as u8;
        }
        draw -= *weight;
    }
    unreachable!("weighted sample must select a key")
}

fn build_online_stream(workload: Workload, seed: u64) -> (Vec<Event>, [u16; KEY_COUNT], Rng) {
    let mut rng = Rng::new(seed);
    let historical_weights = query_weights(workload, false);
    let mut current_revision = [0_u16; KEY_COUNT];
    let mut events = Vec::new();

    for key in 0..KEY_COUNT {
        current_revision[key] = 1;
        events.push(Event::Write {
            key: key as u8,
            revision: 1,
        });
    }

    for offset in 0..ONLINE_STEPS {
        let is_burst_write = workload == Workload::DistractorBurst
            && offset >= ONLINE_STEPS - 48
            && rng.below(100) < 82;

        if is_burst_write || rng.below(100) >= 63 {
            let key = if is_burst_write {
                (8 + rng.below(4)) as u8
            } else {
                rng.below(KEY_COUNT) as u8
            };
            current_revision[key as usize] = current_revision[key as usize].saturating_add(1);
            events.push(Event::Write {
                key,
                revision: current_revision[key as usize],
            });
        } else {
            let key = sample_key(&mut rng, &historical_weights);
            events.push(Event::Query {
                key,
            });
        }
    }

    // Return the PRNG state, not the future query sequence. Callers replay the
    // online stream and freeze every policy snapshot before future sampling.
    (events, current_revision, rng)
}

fn build_future_queries(
    workload: Workload,
    latest_revisions: &[u16; KEY_COUNT],
    rng: &mut Rng,
) -> Vec<FutureQuery> {
    let future_weights = query_weights(workload, true);
    let mut future = Vec::with_capacity(FUTURE_QUERIES);
    for _ in 0..FUTURE_QUERIES {
        let key = sample_key(rng, &future_weights);
        future.push(FutureQuery {
            key,
            truth_revision: latest_revisions[key as usize],
        });
    }
    future
}

fn entry_score(entry: &Entry, policy: Policy, now: u32) -> i64 {
    match policy {
        Policy::Fifo => i64::from(entry.write_step),
        Policy::Lru => i64::from(entry.last_access_step),
        Policy::Lfu => i64::from(entry.access_count) * 1_000
            + i64::from(entry.last_access_step),
        Policy::Rarity => {
            // Lower scores are evicted. Repeated writes to one resident key
            // reduce its retention priority; no future request is consulted.
            1_000_000 / i64::from(entry.writes_while_resident.max(1))
                + i64::from(entry.write_step.min(now))
        }
        Policy::RiskOnly => i64::from(entry.risk_band) * 10_000,
        Policy::RiskAwareAccess => i64::from(entry.risk_band) * 10_000
            + i64::from(entry.access_count.min(1_000)) * 1_000
            + i64::from(entry.last_access_step.min(999)),
    }
}

// Policy boundary: only current resident entries, policy, and online time enter
// eviction selection. No future query list, future query count, or evaluator
// utility object exists in this function's signature.
fn select_victim(entries: &[Entry], policy: Policy, now: u32) -> usize {
    assert!(!entries.is_empty());
    let mut victim = 0;
    let mut victim_score = entry_score(&entries[0], policy, now);
    for (idx, entry) in entries.iter().enumerate().skip(1) {
        let score = entry_score(entry, policy, now);
        if score < victim_score
            || (score == victim_score && entry.key < entries[victim].key)
        {
            victim = idx;
            victim_score = score;
        }
    }
    victim
}

#[derive(Clone, Debug)]
struct Replay {
    entries: Vec<Entry>,
    comparisons: u64,
    writes: u64,
    evictions: u64,
}

fn replay(events: &[Event], policy: Policy, budget: usize) -> Replay {
    assert!(budget > 0);
    let mut entries: Vec<Entry> = Vec::with_capacity(budget);
    let mut comparisons = 0_u64;
    let mut writes = 0_u64;
    let mut evictions = 0_u64;

    for (step, event) in events.iter().enumerate() {
        let now = step as u32 + 1;
        match *event {
            Event::Write { key, revision } => {
                writes += 1;
                if let Some(idx) = entries.iter().position(|entry| entry.key == key) {
                    let old = entries[idx];
                    let mut updated = Entry::new(key, revision, now);
                    // Access experience persists when a retained key is revised.
                    // No evaluator labels are carried in online events.
                    updated.write_step = old.write_step; // keyed update does not refresh FIFO admission order
                    updated.last_access_step = old.last_access_step;
                    updated.access_count = old.access_count;
                    updated.writes_while_resident =
                        old.writes_while_resident.saturating_add(1);
                    entries[idx] = updated;
                } else {
                    if entries.len() == budget {
                        comparisons += entries.len() as u64;
                        let victim = select_victim(&entries, policy, now);
                        entries.remove(victim);
                        evictions += 1;
                    }
                    entries.push(Entry::new(key, revision, now));
                }
            }
            Event::Query { key } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.key == key) {
                    entry.last_access_step = now;
                    entry.access_count = entry.access_count.saturating_add(1);
                }
            }
        }
    }

    Replay {
        entries,
        comparisons,
        writes,
        evictions,
    }
}

#[derive(Clone, Debug)]
struct Score {
    hit: usize,
    stale: usize,
    miss: usize,
    critical_misses: usize,
    utility: i64,
}

fn score_future(entries: &[Entry], future: &[FutureQuery]) -> Score {
    let mut result = Score {
        hit: 0,
        stale: 0,
        miss: 0,
        critical_misses: 0,
        utility: 0,
    };
    for query in future {
        let impact = evaluator_impact(query.key);
        if let Some(entry) = entries.iter().find(|entry| entry.key == query.key) {
            if entry.revision == query.truth_revision {
                result.hit += 1;
                result.utility += impact;
            } else {
                result.stale += 1;
                result.utility -= 2 * impact;
                if impact >= 3 {
                    result.critical_misses += 1;
                }
            }
        } else {
            result.miss += 1;
            result.utility -= impact;
            if impact >= 3 {
                result.critical_misses += 1;
            }
        }
    }
    result
}

// Evaluator-only clairvoyant upper bound: it chooses a fixed-budget subset of
// the latest valid key states after seeing the future query trace. This is
// deliberately impossible for online policies and must never guide selection.
fn hindsight_upper_bound(
    latest_revisions: &[u16; KEY_COUNT],
    future: &[FutureQuery],
    budget: usize,
) -> i64 {
    let mut counts = [0_u64; KEY_COUNT];
    for query in future {
        counts[query.key as usize] += 1;
    }
    let mut keys: Vec<usize> = (0..KEY_COUNT).collect();
    keys.sort_by(|a, b| {
        let a_value = counts[*a] as i64 * evaluator_impact(*a as u8);
        let b_value = counts[*b] as i64 * evaluator_impact(*b as u8);
        b_value.cmp(&a_value).then_with(|| a.cmp(b))
    });
    let retained: Vec<usize> = keys.into_iter().take(budget).collect();
    future.iter().fold(0_i64, |sum, query| {
        if retained.contains(&(query.key as usize))
            && latest_revisions[query.key as usize] == query.truth_revision
        {
            sum + evaluator_impact(query.key)
        } else {
            sum - evaluator_impact(query.key)
        }
    })
}

#[derive(Clone, Debug)]
struct Row {
    workload: Workload,
    seed: usize,
    budget: usize,
    policy: Policy,
    score: Score,
    comparisons: u64,
    writes: u64,
    evictions: u64,
    retained: usize,
    logical_bytes: usize,
    oracle_utility: i64,
}

fn run_row(workload: Workload, seed: usize, budget: usize, policy: Policy) -> Row {
    let mixed_seed = 0x71_9d_21_a5_0000_0001_u64
        ^ (seed as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ ((Workload::ALL.iter().position(|w| *w == workload).unwrap() as u64 + 1) << 40);
    let (events, latest_revisions, mut future_rng) = build_online_stream(workload, mixed_seed);
    let replayed = replay(&events, policy, budget);
    // The evaluation trace is generated only after this policy snapshot freezes.
    let future = build_future_queries(workload, &latest_revisions, &mut future_rng);
    let score = score_future(&replayed.entries, &future);
    let oracle_utility = hindsight_upper_bound(&latest_revisions, &future, budget);

    Row {
        workload,
        seed,
        budget,
        policy,
        score,
        comparisons: replayed.comparisons,
        writes: replayed.writes,
        evictions: replayed.evictions,
        retained: replayed.entries.len(),
        logical_bytes: replayed.entries.len() * SLOT_BYTES,
        oracle_utility,
    }
}

fn report() -> String {
    let mut out = String::new();
    out.push_str("META\tformat\tVIS-MEM-021-v0.1\n");
    out.push_str("META\tscope\tfinite synthetic online-retention value diagnostic; not native robot performance\n");
    out.push_str("META\tfuture_query_visibility\tfuture trace is evaluator-only and starts after the frozen memory snapshot\n");
    out.push_str("META\toracle\tclairvoyant fixed-budget upper bound; diagnostic only; never passed to selectors\n");
    out.push_str(&format!("META\tseeds\t{SEEDS}\n"));
    out.push_str(&format!("META\tonline_steps\t{ONLINE_STEPS}\n"));
    out.push_str(&format!("META\tfuture_queries_per_run\t{FUTURE_QUERIES}\n"));
    out.push_str(&format!("META\tlogical_slot_bytes\t{SLOT_BYTES}; illustrative fixed slot accounting, not RSS\n"));

    out.push_str("ROW_HEADER\tworkload\tseed\tbudget\tpolicy\tqueries\thit_rate\tstale_rate\tmiss_rate\tweighted_utility\tcritical_misses\tretained\tlogical_bytes\tselector_comparisons\twrites\tevictions\toracle_utility\tregret\n");

    let mut rows = Vec::new();
    for workload in Workload::ALL {
        for budget in BUDGETS {
            for seed in 0..SEEDS {
                for policy in Policy::ALL {
                    rows.push(run_row(workload, seed, budget, policy));
                }
            }
        }
    }

    for row in &rows {
        let denom = FUTURE_QUERIES as f64;
        let hit_rate = row.score.hit as f64 / denom;
        let stale_rate = row.score.stale as f64 / denom;
        let miss_rate = row.score.miss as f64 / denom;
        let regret = row.oracle_utility - row.score.utility;
        out.push_str(&format!(
            "ROW\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.workload.label(),
            row.seed,
            row.budget,
            row.policy.label(),
            FUTURE_QUERIES,
            hit_rate,
            stale_rate,
            miss_rate,
            row.score.utility,
            row.score.critical_misses,
            row.retained,
            row.logical_bytes,
            row.comparisons,
            row.writes,
            row.evictions,
            row.oracle_utility,
            regret,
        ));
    }

    out.push_str("SUMMARY_HEADER\tworkload\tbudget\tpolicy\truns\tmean_hit_rate\tmean_stale_rate\tmean_miss_rate\tmean_utility_per_query\tmean_critical_misses\tmean_regret_to_oracle\tmean_selector_comparisons\tmean_evictions\n");
    for workload in Workload::ALL {
        for budget in BUDGETS {
            for policy in Policy::ALL {
                let matching: Vec<&Row> = rows
                    .iter()
                    .filter(|row| row.workload == workload && row.budget == budget && row.policy == policy)
                    .collect();
                let runs = matching.len() as f64;
                let mean = |f: fn(&Row) -> f64| matching.iter().map(|row| f(row)).sum::<f64>() / runs;
                out.push_str(&format!(
                    "SUMMARY\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\n",
                    workload.label(),
                    budget,
                    policy.label(),
                    matching.len(),
                    mean(|row| row.score.hit as f64 / FUTURE_QUERIES as f64),
                    mean(|row| row.score.stale as f64 / FUTURE_QUERIES as f64),
                    mean(|row| row.score.miss as f64 / FUTURE_QUERIES as f64),
                    mean(|row| row.score.utility as f64 / FUTURE_QUERIES as f64),
                    mean(|row| row.score.critical_misses as f64),
                    mean(|row| (row.oracle_utility - row.score.utility) as f64),
                    mean(|row| row.comparisons as f64),
                    mean(|row| row.evictions as f64),
                ));
            }
        }
    }
    out
}

fn main() {
    print!("{}", report());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_replays_deterministically() {
        assert_eq!(report(), report());
    }

    #[test]
    fn policy_selector_has_no_future_query_parameter() {
        let entries = [
            Entry::new(1, 1, 1),
            Entry::new(2, 1, 2),
            Entry::new(11, 1, 3),
        ];
        for policy in Policy::ALL {
            let first = select_victim(&entries, policy, 100);
            let second = select_victim(&entries, policy, 100);
            assert_eq!(first, second);
            assert!(first < entries.len());
        }
    }

    #[test]
    fn online_replay_snapshot_is_independent_of_future_trace() {
        let events = [
            Event::Write { key: 0, revision: 1 },
            Event::Write { key: 11, revision: 1 },
            Event::Query { key: 11 },
            Event::Write { key: 2, revision: 1 },
        ];
        let future_a = [FutureQuery { key: 0, truth_revision: 1 }];
        let future_b = [FutureQuery { key: 11, truth_revision: 1 }];
        for policy in Policy::ALL {
            // Retention sees only events. Changing the held-out workload can
            // change the evaluation score, but cannot change the frozen store.
            let snapshot_a = replay(&events, policy, 2).entries;
            let snapshot_b = replay(&events, policy, 2).entries;
            assert_eq!(snapshot_a, snapshot_b);
            assert_ne!(
                score_future(&snapshot_a, &future_a).utility,
                score_future(&snapshot_b, &future_b).utility
            );
        }
    }

    #[test]
    fn capacity_is_never_exceeded() {
        for workload in Workload::ALL {
            let (events, _, _) = build_online_stream(workload, 12345);
            for budget in BUDGETS {
                for policy in Policy::ALL {
                    let replayed = replay(&events, policy, budget);
                    assert!(replayed.entries.len() <= budget);
                    assert_eq!(
                        replayed.entries.iter().map(|entry| entry.key).collect::<std::collections::BTreeSet<_>>().len(),
                        replayed.entries.len()
                    );
                }
            }
        }
    }

    #[test]
    fn hindsight_upper_bound_dominates_all_online_policies() {
        for workload in Workload::ALL {
            for seed in 0..SEEDS {
                for budget in BUDGETS {
                    let mixed_seed = 0x71_9d_21_a5_0000_0001_u64
                        ^ (seed as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                        ^ ((Workload::ALL.iter().position(|w| *w == workload).unwrap() as u64 + 1) << 40);
                    let (events, latest, mut future_rng) = build_online_stream(workload, mixed_seed);
                    let snapshots: Vec<(Policy, Replay)> = Policy::ALL
                        .into_iter()
                        .map(|policy| (policy, replay(&events, policy, budget)))
                        .collect();
                    let future = build_future_queries(workload, &latest, &mut future_rng);
                    let oracle = hindsight_upper_bound(&latest, &future, budget);
                    for (policy, replayed) in snapshots {
                        let score = score_future(&replayed.entries, &future);
                        assert!(
                            oracle >= score.utility,
                            "oracle must dominate: {workload:?} seed={seed} budget={budget} policy={policy:?} oracle={} policy={}",
                            oracle, score.utility
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn fifo_update_does_not_refresh_admission_order() {
        let events = [
            Event::Write { key: 0, revision: 1 },
            Event::Write { key: 1, revision: 1 },
            Event::Write { key: 0, revision: 2 },
            Event::Write { key: 2, revision: 1 },
        ];
        let replayed = replay(&events, Policy::Fifo, 2);
        assert!(!replayed.entries.iter().any(|entry| entry.key == 0));
        assert!(replayed.entries.iter().any(|entry| entry.key == 1));
        assert!(replayed.entries.iter().any(|entry| entry.key == 2));
    }

    #[test]
    fn stale_memory_is_scored_worse_than_a_miss() {
        let stale = Entry::new(3, 1, 1);
        let query = FutureQuery { key: 3, truth_revision: 2 };
        let stale_score = score_future(&[stale], &[query]);
        let miss_score = score_future(&[], &[query]);
        assert!(stale_score.utility < miss_score.utility);
        assert_eq!(stale_score.stale, 1);
        assert_eq!(miss_score.miss, 1);
    }

    #[test]
    fn fixed_slot_bytes_match_retained_records() {
        let (events, _, _) = build_online_stream(Workload::Stationary, 9);
        let replayed = replay(&events, Policy::Lru, 3);
        assert!(replayed.entries.len() <= 3);
        let logical_bytes = replayed.entries.len() * SLOT_BYTES;
        assert_eq!(logical_bytes, 3 * SLOT_BYTES);
    }
}
