// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-008: deterministic evidence-memory lifecycle diagnostic.
//!
//! This is a synthetic state-machine benchmark. It does not load MEMOBench
//! assets, expose evaluator truth to a policy, or claim robotic task success.
//!
//! rustc --edition=2024 --test tools/vision-memory-lifecycle-bench.rs -o /tmp/vis-mem-tests
//! /tmp/vis-mem-tests
//! rustc --edition=2024 -O tools/vision-memory-lifecycle-bench.rs -o /tmp/vis-mem-bench
//! /tmp/vis-mem-bench > lifecycle-metrics.tsv
//! /tmp/vis-mem-bench --receipts > lifecycle-receipts.tsv

use std::env;

const CLAIMS: usize = 8;
const QUERY_WIDTH: usize = 3;
const HORIZON: usize = 24;
const EPISODES: usize = 64;
const BOOTSTRAPS: usize = 2_000;
const ROOT_SEED: u64 = 0x6A09_E667_F3BC_C909;
const CAPACITIES: [usize; 4] = [2, 3, 4, 6];

// Fixed-width logical accounting, not an estimate of allocator/RAM overhead.
// The 24-byte record layout reserves padding so results do not depend on ABI.
const OBSERVATION_BYTES: u64 = 24;
const REVISION_LINK_BYTES: u64 = 8;
const RECEIPT_LOGICAL_BYTES: u64 = 64;

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

    fn chance(&mut self, per_thousand: u64) -> bool {
        self.next() % 1_000 < per_thousand
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Observation {
    key: u8,
    value: bool,
    source_tick: u16,
    arrival_tick: u16,
    session: u8,
    confidence: u16,
    id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    observation: Observation,
    prior_id: Option<u64>,
}

#[derive(Clone, Debug)]
struct Episode {
    query_keys: [u8; QUERY_WIDTH],
    truth: Vec<[bool; CLAIMS]>,
    events: Vec<Observation>,
}

impl Episode {
    fn generate(seed: u64, episode_index: usize) -> Self {
        let mut rng = Rng::new(seed);
        let mut truth = vec![[false; CLAIMS]; HORIZON];

        for value in &mut truth[0] {
            *value = rng.chance(500);
        }

        let flip_rate = if episode_index % 3 == 0 { 180 } else { 90 };
        for tick in 1..HORIZON {
            truth[tick] = truth[tick - 1];
            for value in &mut truth[tick] {
                if rng.chance(flip_rate) {
                    *value = !*value;
                }
            }
        }

        let mut query_keys = [0_u8; QUERY_WIDTH];
        let mut query_len = 0;
        while query_len < QUERY_WIDTH {
            let candidate = rng.range(CLAIMS) as u8;
            if !query_keys[..query_len].contains(&candidate) {
                query_keys[query_len] = candidate;
                query_len += 1;
            }
        }

        let mut events = Vec::new();
        let mut ordinal = 0_u64;
        for source_tick in 0..HORIZON {
            for key in 0..CLAIMS {
                let regular_query_probe =
                    query_keys.contains(&(key as u8)) && source_tick % 5 == 0;
                if !regular_query_probe && !rng.chance(430) {
                    continue;
                }

                let corruption_rate = if episode_index % 5 == 0 { 170 } else { 80 };
                let value = truth[source_tick][key] ^ rng.chance(corruption_rate);
                let confidence_tiers = [650_u16, 850, 950];
                // Confidence is sampled independently of corruption.
                let confidence = confidence_tiers[rng.range(confidence_tiers.len())];
                let max_delay = if episode_index % 4 == 0 { 7 } else { 4 };
                let delay = rng.range(max_delay);
                let arrival_tick = (source_tick + delay).min(HORIZON - 1);
                let id = mix64(
                    seed ^ ((source_tick as u64) << 40)
                        ^ ((key as u64) << 32)
                        ^ ordinal,
                );
                ordinal += 1;

                events.push(Observation {
                    key: key as u8,
                    value,
                    source_tick: source_tick as u16,
                    arrival_tick: arrival_tick as u16,
                    session: rng.range(8) as u8,
                    confidence,
                    id,
                });
            }
        }

        events.sort_by_key(|o| (o.arrival_tick, o.id));
        Self {
            query_keys,
            truth,
            events,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    ArrivalOverwrite,
    SourceTimeLatest,
    EventLedger,
    RecencyProjection,
    QueryPreserving,
}

impl Policy {
    const ALL: [Self; 5] = [
        Self::ArrivalOverwrite,
        Self::SourceTimeLatest,
        Self::EventLedger,
        Self::RecencyProjection,
        Self::QueryPreserving,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::ArrivalOverwrite => "arrival_overwrite",
            Self::SourceTimeLatest => "source_time_latest",
            Self::EventLedger => "event_ledger",
            Self::RecencyProjection => "recency_projection",
            Self::QueryPreserving => "query_preserving",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Receipt {
    episode: usize,
    policy: &'static str,
    capacity: usize,
    tick: u16,
    operation: &'static str,
    key: u8,
    source_id: u64,
    prior_id: Option<u64>,
    evicted_id: Option<u64>,
    before_digest: u64,
    after_digest: u64,
    bytes_before: u64,
    bytes_after: u64,
    result: &'static str,
}

impl Receipt {
    fn query(
        episode: usize,
        policy: Policy,
        capacity: usize,
        tick: u16,
        key: u8,
        entry: Option<Entry>,
        memory: &Memory,
    ) -> Self {
        let digest = memory.digest();
        let bytes = memory.logical_bytes();
        Self {
            episode,
            policy: policy.name(),
            capacity,
            tick,
            operation: "query",
            key,
            source_id: entry.map(|e| e.observation.id).unwrap_or(0),
            prior_id: entry.and_then(|e| e.prior_id),
            evicted_id: None,
            before_digest: digest,
            after_digest: digest,
            bytes_before: bytes,
            bytes_after: bytes,
            result: if entry.is_some() { "cited" } else { "missing" },
        }
    }
}

struct ApplyResult {
    primary: Receipt,
    compression: Option<Receipt>,
    evicted: Option<Entry>,
}

#[derive(Clone, Debug, Default)]
struct Memory {
    items: Vec<Entry>,
}

impl Memory {
    fn digest(&self) -> u64 {
        let mut h = 0xCBF2_9CE4_8422_2325_u64;
        for e in &self.items {
            let o = e.observation;
            for word in [
                o.id,
                o.key as u64,
                o.value as u64,
                o.source_tick as u64,
                o.arrival_tick as u64,
                o.session as u64,
                o.confidence as u64,
                e.prior_id.unwrap_or(0),
                e.prior_id.is_some() as u64,
            ] {
                h ^= word;
                h = h.wrapping_mul(0x0000_0100_0000_01B3);
            }
        }
        h
    }

    fn logical_bytes(&self) -> u64 {
        self.items
            .iter()
            .map(|e| OBSERVATION_BYTES + if e.prior_id.is_some() { REVISION_LINK_BYTES } else { 0 })
            .sum()
    }

    fn best_entry(&self, key: u8) -> Option<Entry> {
        self.items
            .iter()
            .copied()
            .filter(|e| e.observation.key == key)
            .max_by_key(|e| (e.observation.source_tick, e.observation.id))
    }

    fn best_entries(&self, query: &[u8; QUERY_WIDTH]) -> [Option<Entry>; QUERY_WIDTH] {
        std::array::from_fn(|i| self.best_entry(query[i]))
    }

    fn eviction_index(
        &self,
        prefer_non_query: bool,
        query: &[u8; QUERY_WIDTH],
    ) -> Option<usize> {
        if prefer_non_query {
            if let Some((i, _)) = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, e)| !query.contains(&e.observation.key))
                .min_by_key(|(_, e)| (e.observation.arrival_tick, e.observation.id))
            {
                return Some(i);
            }
        }

        self.items
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| (e.observation.arrival_tick, e.observation.id))
            .map(|(i, _)| i)
    }

    fn insert_bounded(
        &mut self,
        entry: Entry,
        capacity: usize,
        prefer_non_query: bool,
        query: &[u8; QUERY_WIDTH],
    ) -> Option<Entry> {
        if capacity == 0 {
            return Some(entry);
        }
        let evicted = if self.items.len() >= capacity {
            self.eviction_index(prefer_non_query, query)
                .map(|i| self.items.remove(i))
        } else {
            None
        };
        if self.items.len() < capacity {
            self.items.push(entry);
        }
        evicted
    }

    fn apply(
        &mut self,
        observation: Observation,
        policy: Policy,
        capacity: usize,
        query: &[u8; QUERY_WIDTH],
        episode: usize,
    ) -> ApplyResult {
        let before_digest = self.digest();
        let bytes_before = self.logical_bytes();
        let before_best = self.best_entry(observation.key);
        let prior_id = before_best.map(|e| e.observation.id);
        let incoming_order = (observation.source_tick, observation.id);
        let is_newer = before_best
            .map(|e| incoming_order > (e.observation.source_tick, e.observation.id))
            .unwrap_or(true);
        let index = self.items.iter().position(|e| e.observation.key == observation.key);
        let mut evicted: Option<Entry> = None;
        let mut accepted = false;
        let mut result = "ignored_stale";
        let mut mutation_kind = "ignore";

        match policy {
            Policy::ArrivalOverwrite => {
                if let Some(i) = index {
                    self.items.remove(i);
                    self.items.push(Entry { observation, prior_id: None });
                    accepted = true;
                    result = "overwrote_by_arrival";
                    mutation_kind = "update";
                } else {
                    evicted = self.insert_bounded(
                        Entry { observation, prior_id: None },
                        capacity,
                        false,
                        query,
                    );
                    accepted = self.items.iter().any(|e| e.observation.id == observation.id);
                    result = if accepted { "stored_arrival_order" } else { "capacity_rejected" };
                    mutation_kind = if accepted { "storage" } else { "ignore" };
                }
            }
            Policy::SourceTimeLatest => {
                if let Some(i) = index {
                    let current = self.items[i];
                    if incoming_order > (current.observation.source_tick, current.observation.id) {
                        self.items.remove(i);
                        self.items.push(Entry { observation, prior_id: Some(current.observation.id) });
                        accepted = true;
                        result = "accepted_new_source_revision";
                        mutation_kind = "update";
                    } else {
                        result = "rejected_old_source_revision";
                    }
                } else {
                    evicted = self.insert_bounded(
                        Entry { observation, prior_id: None },
                        capacity,
                        false,
                        query,
                    );
                    accepted = self.items.iter().any(|e| e.observation.id == observation.id);
                    result = if accepted { "stored_new_key" } else { "capacity_rejected" };
                    mutation_kind = if accepted { "storage" } else { "ignore" };
                }
            }
            Policy::EventLedger => {
                if self.items.iter().any(|e| e.observation.id == observation.id) {
                    result = "duplicate_source_id";
                } else {
                    evicted = self.insert_bounded(
                        Entry { observation, prior_id },
                        capacity,
                        false,
                        query,
                    );
                    accepted = self.items.iter().any(|e| e.observation.id == observation.id);
                    result = if accepted { "appended_event" } else { "capacity_rejected" };
                    mutation_kind = if prior_id.is_some() { "update" } else { "storage" };
                }
            }
            Policy::RecencyProjection => {
                if let Some(i) = index {
                    let old = self.items.remove(i);
                    self.items.push(Entry { observation, prior_id: Some(old.observation.id) });
                    accepted = true;
                    result = "replaced_by_recent_arrival_with_link";
                    mutation_kind = "update";
                } else {
                    evicted = self.insert_bounded(
                        Entry { observation, prior_id: None },
                        capacity,
                        false,
                        query,
                    );
                    accepted = self.items.iter().any(|e| e.observation.id == observation.id);
                    result = if accepted { "stored_recent_projection" } else { "capacity_rejected" };
                    mutation_kind = if accepted { "storage" } else { "ignore" };
                }
            }
            Policy::QueryPreserving => {
                if !query.contains(&observation.key) {
                    result = "ignored_non_query_evidence";
                } else if let Some(i) = index {
                    let old = self.items[i];
                    if incoming_order > (old.observation.source_tick, old.observation.id) {
                        self.items.remove(i);
                        self.items.push(Entry { observation, prior_id: Some(old.observation.id) });
                        accepted = true;
                        result = "accepted_query_revision";
                        mutation_kind = "update";
                    } else {
                        result = "ignored_old_query_revision";
                    }
                } else {
                    evicted = self.insert_bounded(
                        Entry { observation, prior_id: None },
                        capacity,
                        true,
                        query,
                    );
                    accepted = self.items.iter().any(|e| e.observation.id == observation.id);
                    result = if accepted { "stored_query_evidence" } else { "capacity_rejected" };
                    mutation_kind = if accepted { "storage" } else { "ignore" };
                }
            }
        }

        // A rejected stale event never rewinds source-time-aware projections.
        if !accepted && evicted.is_some() {
            result = "capacity_eviction_without_admission";
        }
        let after_digest = self.digest();
        let bytes_after = self.logical_bytes();
        let primary = Receipt {
            episode,
            policy: policy.name(),
            capacity,
            tick: observation.arrival_tick,
            operation: mutation_kind,
            key: observation.key,
            source_id: observation.id,
            prior_id,
            evicted_id: evicted.map(|e| e.observation.id),
            before_digest,
            after_digest,
            bytes_before,
            bytes_after,
            result,
        };
        // Compression/eviction is separately visible in the same transition.
        let compression = evicted.map(|old| Receipt {
            episode,
            policy: policy.name(),
            capacity,
            tick: observation.arrival_tick,
            operation: "compression",
            key: old.observation.key,
            source_id: observation.id,
            prior_id,
            evicted_id: Some(old.observation.id),
            before_digest,
            after_digest,
            bytes_before,
            bytes_after,
            result: "capacity_eviction",
        });
        ApplyResult { primary, compression, evicted }
    }
}

#[derive(Clone, Debug)]
struct RunSummary {
    episode: usize,
    policy: Policy,
    capacity: usize,
    task_loss: f64,
    answered: bool,
    storage_ok: u64,
    storage_n: u64,
    update_ok: u64,
    update_n: u64,
    compression_ok: u64,
    compression_n: u64,
    projection_errors: u64,
    projection_checks: u64,
    link_ok: u64,
    link_n: u64,
    citation_ok: u64,
    citation_n: u64,
    final_bytes: u64,
    peak_bytes: u64,
    receipt_count: u64,
    final_digest: u64,
    receipts: Vec<Receipt>,
}

fn newer(a: Observation, b: Observation) -> bool {
    (a.source_tick, a.id) > (b.source_tick, b.id)
}

fn run_episode(
    episode: &Episode,
    episode_index: usize,
    policy: Policy,
    capacity: usize,
    capture_receipts: bool,
) -> RunSummary {
    let mut memory = Memory::default();
    let mut latest_seen: [Option<Observation>; CLAIMS] = [None; CLAIMS];
    let mut storage_ok = 0;
    let mut storage_n = 0;
    let mut update_ok = 0;
    let mut update_n = 0;
    let mut compression_ok = 0;
    let mut compression_n = 0;
    let mut projection_errors = 0;
    let mut projection_checks = 0;
    let mut link_ok = 0;
    let mut link_n = 0;
    let mut peak_bytes = 0;
    let mut receipt_count = 0;
    let mut receipts = Vec::new();

    for tick in 0..HORIZON {
        for observation in episode.events.iter().filter(|o| o.arrival_tick as usize == tick) {
            let previous_seen = latest_seen[observation.key as usize];
            let is_newest_observation = previous_seen
                .map(|old| newer(*observation, old))
                .unwrap_or(true);

            let update_target = episode.query_keys.contains(&observation.key)
                && previous_seen.is_some_and(|old| {
                    newer(*observation, old) && observation.value != old.value
                });

            if is_newest_observation {
                latest_seen[observation.key as usize] = Some(*observation);
            }

            let applied = memory.apply(
                *observation,
                policy,
                capacity,
                &episode.query_keys,
                episode_index,
            );
            receipt_count += 1 + u64::from(applied.compression.is_some());
            if capture_receipts {
                receipts.push(applied.primary.clone());
                if let Some(ref receipt) = applied.compression {
                    receipts.push(receipt.clone());
                }
            }

            if episode.query_keys.contains(&observation.key) {
                storage_n += 1;
                let expected = latest_seen[observation.key as usize].map(|o| o.id);
                let actual = memory.best_entry(observation.key).map(|e| e.observation.id);
                if expected.is_some() && actual == expected {
                    storage_ok += 1;
                }
            }

            if update_target {
                update_n += 1;
                let expected = latest_seen[observation.key as usize].map(|o| o.id);
                let actual_entry = memory.best_entry(observation.key);
                if expected.is_some() && actual_entry.map(|e| e.observation.id) == expected {
                    update_ok += 1;
                }
                link_n += 1;
                if actual_entry.and_then(|e| e.prior_id) == previous_seen.map(|o| o.id) {
                    link_ok += 1;
                }
            }

            if applied.evicted.is_some() {
                for key in episode.query_keys {
                    if let Some(expected) = latest_seen[key as usize] {
                        compression_n += 1;
                        if memory.best_entry(key).map(|e| e.observation.id) == Some(expected.id) {
                            compression_ok += 1;
                        }
                    }
                }
            }

            for key in episode.query_keys {
                if let Some(expected) = latest_seen[key as usize] {
                    projection_checks += 1;
                    if memory.best_entry(key).map(|e| e.observation.id) != Some(expected.id) {
                        projection_errors += 1;
                    }
                }
            }
            peak_bytes = peak_bytes.max(memory.logical_bytes());
        }
    }

    let cited = memory.best_entries(&episode.query_keys);
    let final_truth = episode.truth[HORIZON - 1];
    let truth_positive = episode.query_keys.iter().filter(|&&k| final_truth[k as usize]).count();
    let target = truth_positive >= 2;
    let answered = cited.iter().all(Option::is_some);
    let task_loss = if !answered {
        0.25
    } else {
        let predicted_positive = cited
            .iter()
            .filter(|entry| entry.is_some_and(|e| e.observation.value))
            .count();
        if (predicted_positive >= 2) == target { 0.0 } else { 1.0 }
    };

    let mut citation_ok = 0;
    let mut citation_n = 0;
    for (i, key) in episode.query_keys.iter().copied().enumerate() {
        if let Some(expected) = latest_seen[key as usize] {
            citation_n += 1;
            if cited[i].map(|e| e.observation.id) == Some(expected.id) {
                citation_ok += 1;
            }
        }
        if capture_receipts {
            receipts.push(Receipt::query(
                episode_index,
                policy,
                capacity,
                HORIZON as u16,
                key,
                cited[i],
                &memory,
            ));
        }
        receipt_count += 1;
    }

    RunSummary {
        episode: episode_index,
        policy,
        capacity,
        task_loss,
        answered,
        storage_ok,
        storage_n,
        update_ok,
        update_n,
        compression_ok,
        compression_n,
        projection_errors,
        projection_checks,
        link_ok,
        link_n,
        citation_ok,
        citation_n,
        final_bytes: memory.logical_bytes(),
        peak_bytes,
        receipt_count,
        final_digest: memory.digest(),
        receipts,
    }
}

fn rate(ok: u64, total: u64) -> String {
    if total == 0 {
        "NA".to_owned()
    } else {
        format!("{:.6}", ok as f64 / total as f64)
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn bootstrap_ci(values: &[f64], seed: u64) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
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
    let lo = ((BOOTSTRAPS - 1) as f64 * 0.025).floor() as usize;
    let hi = ((BOOTSTRAPS - 1) as f64 * 0.975).floor() as usize;
    (means[lo], means[hi])
}

fn run_report() {
    let mut rows = Vec::new();
    for episode_index in 0..EPISODES {
        let seed = mix64(ROOT_SEED ^ episode_index as u64);
        let episode = Episode::generate(seed, episode_index);
        for policy in Policy::ALL {
            for capacity in CAPACITIES {
                rows.push(run_episode(&episode, episode_index, policy, capacity, false));
            }
        }
    }

    println!("META\tformat\tVIS-MEM-008-v0.1");
    println!("META\troot_seed\t{ROOT_SEED:016x}");
    println!("META\tepisodes\t{EPISODES}");
    println!("META\tcapacities\t2,3,4,6");
    println!("META\tconfidence\tsampled independently of corruption");
    println!("META\ttruth_access\tevaluator only");
    println!("METRIC_HEADER\tpolicy\tcapacity\tepisodes\tmean_task_loss\tanswer_rate\tstorage_rate\tupdate_rate\tcompression_retention\tstale_projection_error_rate\trevision_link_integrity\tquery_citation_rate\tmean_final_state_bytes\tmean_peak_state_bytes\tmean_logical_receipt_bytes");
    for policy in Policy::ALL {
        for capacity in CAPACITIES {
            let group: Vec<&RunSummary> = rows
                .iter()
                .filter(|r| r.policy == policy && r.capacity == capacity)
                .collect();
            let losses: Vec<f64> = group.iter().map(|r| r.task_loss).collect();
            let answered = group.iter().filter(|r| r.answered).count();
            let storage_ok: u64 = group.iter().map(|r| r.storage_ok).sum();
            let storage_n: u64 = group.iter().map(|r| r.storage_n).sum();
            let update_ok: u64 = group.iter().map(|r| r.update_ok).sum();
            let update_n: u64 = group.iter().map(|r| r.update_n).sum();
            let compression_ok: u64 = group.iter().map(|r| r.compression_ok).sum();
            let compression_n: u64 = group.iter().map(|r| r.compression_n).sum();
            let projection_errors: u64 = group.iter().map(|r| r.projection_errors).sum();
            let projection_checks: u64 = group.iter().map(|r| r.projection_checks).sum();
            let link_ok: u64 = group.iter().map(|r| r.link_ok).sum();
            let link_n: u64 = group.iter().map(|r| r.link_n).sum();
            let citation_ok: u64 = group.iter().map(|r| r.citation_ok).sum();
            let citation_n: u64 = group.iter().map(|r| r.citation_n).sum();
            let final_bytes = mean(&group.iter().map(|r| r.final_bytes as f64).collect::<Vec<_>>());
            let peak_bytes = mean(&group.iter().map(|r| r.peak_bytes as f64).collect::<Vec<_>>());
            let receipt_bytes = mean(&group.iter().map(|r| (r.receipt_count * RECEIPT_LOGICAL_BYTES) as f64).collect::<Vec<_>>());

            println!(
                "METRIC\t{}\t{}\t{}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}",
                policy.name(),
                capacity,
                group.len(),
                mean(&losses),
                answered as f64 / group.len() as f64,
                rate(storage_ok, storage_n),
                rate(update_ok, update_n),
                rate(compression_ok, compression_n),
                rate(projection_errors, projection_checks),
                rate(link_ok, link_n),
                rate(citation_ok, citation_n),
                final_bytes,
                peak_bytes,
                receipt_bytes
            );
        }
    }

    println!("PAIR_HEADER\tpolicy\tcapacity\tcontrast_vs_source_time_latest\tmean_paired_loss_delta\tci95_low\tci95_high");
    for policy in Policy::ALL {
        for capacity in CAPACITIES {
            let candidate: Vec<&RunSummary> = rows
                .iter()
                .filter(|r| r.policy == policy && r.capacity == capacity)
                .collect();
            let baseline: Vec<&RunSummary> = rows
                .iter()
                .filter(|r| r.policy == Policy::SourceTimeLatest && r.capacity == capacity)
                .collect();
            let mut deltas: Vec<f64> = Vec::with_capacity(EPISODES);
            for i in 0..candidate.len() {
                let base = baseline.iter().find(|r| r.episode == candidate[i].episode).unwrap();
                deltas.push(candidate[i].task_loss - base.task_loss);
            }
            let (lo, hi) = bootstrap_ci(&deltas, ROOT_SEED ^ ((capacity as u64) << 8) ^ policy.name().len() as u64);
            println!(
                "PAIR\t{}\t{}\tvs_source_time_latest\t{:.6}\t{:.6}\t{:.6}",
                policy.name(),
                capacity,
                mean(&deltas),
                lo,
                hi
            );
        }
    }
}

fn run_receipts() {
    let seed = mix64(ROOT_SEED);
    let episode = Episode::generate(seed, 0);
    println!("META\tformat\tVIS-MEM-008-receipts-v0.1");
    println!("META\tepisode\t0");
    println!("META\tcapacity\t3");
    println!("META\tstate_digest\tFNV-1a-64 (non-cryptographic; SHA-256 is applied to artifact by workflow)");
    println!("RECEIPT_HEADER\tepisode\tpolicy\tcapacity\ttick\toperation\tkey\tsource_id\tprior_id\tevicted_id\tbefore_digest\tafter_digest\tbytes_before\tbytes_after\tresult");
    for policy in Policy::ALL {
        let summary = run_episode(&episode, 0, policy, 3, true);
        for r in summary.receipts {
            println!(
                "RECEIPT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}\t{:016x}\t{}\t{}\t{}",
                r.episode,
                r.policy,
                r.capacity,
                r.tick,
                r.operation,
                r.key,
                format_args!("{:016x}", r.source_id),
                r.prior_id.map(|id| format!("{id:016x}")).unwrap_or_else(|| "NA".to_owned()),
                r.evicted_id.map(|id| format!("{id:016x}")).unwrap_or_else(|| "NA".to_owned()),
                r.before_digest,
                r.after_digest,
                r.bytes_before,
                r.bytes_after,
                r.result
            );
        }
    }
}

fn main() {
    if env::args().any(|arg| arg == "--receipts") {
        run_receipts();
    } else {
        run_report();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(key: u8, value: bool, source_tick: u16, arrival_tick: u16, id: u64) -> Observation {
        Observation {
            key,
            value,
            source_tick,
            arrival_tick,
            session: 0,
            confidence: 850,
            id,
        }
    }

    #[test]
    fn source_time_latest_rejects_late_stale_observation() {
        let query = [0, 1, 2];
        let mut memory = Memory::default();
        memory.apply(obs(0, true, 8, 8, 80), Policy::SourceTimeLatest, 3, &query, 0);
        memory.apply(obs(0, false, 3, 9, 30), Policy::SourceTimeLatest, 3, &query, 0);
        assert_eq!(memory.best_entry(0).unwrap().observation.id, 80);
        assert_eq!(memory.best_entry(0).unwrap().observation.value, true);
    }

    #[test]
    fn arrival_overwrite_is_a_negative_control_for_stale_rewind() {
        let query = [0, 1, 2];
        let mut memory = Memory::default();
        memory.apply(obs(0, true, 8, 8, 80), Policy::ArrivalOverwrite, 3, &query, 0);
        memory.apply(obs(0, false, 3, 9, 30), Policy::ArrivalOverwrite, 3, &query, 0);
        assert_eq!(memory.best_entry(0).unwrap().observation.id, 30);
        assert_eq!(memory.best_entry(0).unwrap().observation.value, false);
    }

    #[test]
    fn source_time_revision_preserves_parent_provenance() {
        let query = [0, 1, 2];
        let mut memory = Memory::default();
        memory.apply(obs(0, false, 2, 2, 20), Policy::SourceTimeLatest, 3, &query, 0);
        memory.apply(obs(0, true, 4, 4, 40), Policy::SourceTimeLatest, 3, &query, 0);
        let current = memory.best_entry(0).unwrap();
        assert_eq!(current.observation.id, 40);
        assert_eq!(current.prior_id, Some(20));
    }

    #[test]
    fn query_preserving_admission_rejects_non_query_distractors() {
        let query = [0, 1, 2];
        let mut memory = Memory::default();
        memory.apply(obs(7, true, 1, 1, 71), Policy::QueryPreserving, 3, &query, 0);
        assert!(memory.best_entry(7).is_none());
        for key in query {
            memory.apply(obs(key, key % 2 == 0, 2, 2, 100 + key as u64), Policy::QueryPreserving, 3, &query, 0);
        }
        assert_eq!(memory.items.len(), 3);
        assert!(query.into_iter().all(|key| memory.best_entry(key).is_some()));
    }

    #[test]
    fn every_policy_respects_entry_capacity() {
        let query = [0, 1, 2];
        for policy in Policy::ALL {
            for capacity in CAPACITIES {
                let mut memory = Memory::default();
                for i in 0..100 {
                    let key = (i % CLAIMS) as u8;
                    memory.apply(
                        obs(key, i % 2 == 0, i as u16, (i + 3) as u16, i as u64 + 1),
                        policy,
                        capacity,
                        &query,
                        0,
                    );
                    assert!(memory.items.len() <= capacity);
                }
            }
        }
    }

    #[test]
    fn relabeling_hidden_truth_does_not_change_policy_memory_or_receipts() {
        let episode = Episode::generate(mix64(ROOT_SEED), 0);
        let mut relabeled = episode.clone();
        for row in &mut relabeled.truth {
            for value in row {
                *value = !*value;
            }
        }
        for policy in Policy::ALL {
            let original = run_episode(&episode, 0, policy, 3, true);
            let changed = run_episode(&relabeled, 0, policy, 3, true);
            assert_eq!(original.final_digest, changed.final_digest);
            assert_eq!(original.receipts, changed.receipts);
        }
    }

    #[test]
    fn deterministic_generator_and_digest() {
        let a = Episode::generate(12345, 7);
        let b = Episode::generate(12345, 7);
        assert_eq!(a.query_keys, b.query_keys);
        assert_eq!(a.events, b.events);
        let mut m1 = Memory::default();
        let mut m2 = Memory::default();
        for o in a.events.iter().take(10) {
            m1.apply(*o, Policy::EventLedger, 4, &a.query_keys, 7);
            m2.apply(*o, Policy::EventLedger, 4, &a.query_keys, 7);
        }
        assert_eq!(m1.digest(), m2.digest());
    }

    #[test]
    fn explicit_byte_accounting_includes_revision_link() {
        let query = [0, 1, 2];
        let mut memory = Memory::default();
        memory.apply(obs(0, false, 2, 2, 20), Policy::SourceTimeLatest, 3, &query, 0);
        assert_eq!(memory.logical_bytes(), OBSERVATION_BYTES);
        memory.apply(obs(0, true, 4, 4, 40), Policy::SourceTimeLatest, 3, &query, 0);
        assert_eq!(memory.logical_bytes(), OBSERVATION_BYTES + REVISION_LINK_BYTES);
    }
}
