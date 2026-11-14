// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Controlled retention experiment for bounded HDC episodic memory.
//!
//! The test intentionally stores four orthogonal bipolar prototypes, then
//! appends redundant copies of only one class. It compares FIFO, diversity-
//! aware admission, and unbounded reference storage under an identical query
//! set. This is a mechanism contract, not a physics/robot benchmark or SOTA
//! claim. Run with:
//!
//! cargo run --locked --package symtropy-hdc-physics --example episode_memory_retention
//! cargo test --locked --package symtropy-hdc-physics --example episode_memory_retention

use std::collections::BTreeSet;
use std::mem::{size_of, size_of_val};

use symtropy_hdc_core::BipolarHV;
use symtropy_hdc_physics::{EpisodeMemory, EpisodeMetadata, PhysicsEpisode, PhysicsHdcError};

const DIMENSION: usize = 16_384;
const ENCODER_FINGERPRINT: u64 = 0x5245_5445_4e54_494f;
const CAPACITY: usize = 4;
const REDUNDANT_EVENTS: usize = 24;
const NOVELTY_MARGIN: f32 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    A,
    B,
    C,
    D,
}

impl Class {
    const ALL: [Self; 4] = [Self::A, Self::B, Self::C, Self::D];

    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

// Four pairwise-orthogonal bipolar vectors; all patterns have DIMENSION
// components and do not expose query labels to the admission decision.
fn class_vector(class: Class) -> BipolarHV {
    let values = (0..DIMENSION)
        .map(|index| match class {
            Class::A => 1,
            Class::B => if index < DIMENSION / 2 { 1 } else { -1 },
            Class::C => if index % 2 == 0 { 1 } else { -1 },
            Class::D => if (index / 4) % 2 == 0 { 1 } else { -1 },
        })
        .collect();
    BipolarHV::new(values).expect("prototype vectors are bipolar")
}

fn make_episode(id: String, class: Class) -> PhysicsEpisode {
    PhysicsEpisode {
        id: id.clone(),
        encoder_fingerprint: ENCODER_FINGERPRINT,
        first_tick: 0,
        last_tick: 0,
        frame_count: 1,
        temporal_stride: 1,
        exact_digests: Vec::new(),
        vector: class_vector(class),
        metadata: EpisodeMetadata {
            label: class.label().to_owned(),
            run_id: "retention-benchmark".to_owned(),
            seed: 0,
            tags: Default::default(),
        },
    }
}

fn stream() -> Vec<PhysicsEpisode> {
    let mut events: Vec<PhysicsEpisode> = Class::ALL
        .into_iter()
        .map(|class| make_episode(format!("anchor-{}", class.label()), class))
        .collect();

    // Adversarially redundant recent history. The correct query distribution
    // remains balanced across A/B/C/D after these A-only writes.
    for index in 0..REDUNDANT_EVENTS {
        events.push(make_episode(format!("repeat-a-{index:02}"), Class::A));
    }
    events
}

#[derive(Debug, Clone, PartialEq)]
struct Hit {
    episode_id: String,
    label: String,
    similarity: f32,
}

fn top_hit(
    episodes: &[PhysicsEpisode],
    query: &BipolarHV,
) -> Result<Option<Hit>, PhysicsHdcError> {
    let mut best: Option<Hit> = None;
    for episode in episodes {
        let similarity = query.similarity(&episode.vector)?;
        let candidate = Hit {
            episode_id: episode.id.clone(),
            label: episode.metadata.label.clone(),
            similarity,
        };
        let replace = match best.as_ref() {
            None => true,
            Some(current) => match candidate.similarity.total_cmp(&current.similarity) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Equal => candidate.episode_id < current.episode_id,
                std::cmp::Ordering::Less => false,
            },
        };
        if replace {
            best = Some(candidate);
        }
    }
    Ok(best)
}

#[derive(Debug, Clone)]
struct DiversityMemory {
    capacity: usize,
    episodes: Vec<PhysicsEpisode>,
    evicted: usize,
    rejected: usize,
    admission_similarity_comparisons: usize,
}

impl DiversityMemory {
    fn new(capacity: usize) -> Result<Self, PhysicsHdcError> {
        if capacity == 0 {
            return Err(PhysicsHdcError::InvalidConfig(
                "diversity memory capacity must be greater than zero".to_owned(),
            ));
        }
        Ok(Self {
            capacity,
            episodes: Vec::new(),
            evicted: 0,
            rejected: 0,
            admission_similarity_comparisons: 0,
        })
    }

    // Only ID and hypervector participate in admission. Labels, true query
    // class, and future query vectors never enter this method.
    fn insert(&mut self, candidate: PhysicsEpisode) -> Result<bool, PhysicsHdcError> {
        if self.episodes.iter().any(|stored| stored.id == candidate.id) {
            return Err(PhysicsHdcError::DuplicateEpisodeId(candidate.id));
        }
        if candidate.encoder_fingerprint != ENCODER_FINGERPRINT {
            return Err(PhysicsHdcError::EncoderMismatch {
                expected: ENCODER_FINGERPRINT,
                actual: candidate.encoder_fingerprint,
            });
        }
        if candidate.vector.len() != DIMENSION {
            return Err(PhysicsHdcError::VectorDimensionMismatch {
                expected: DIMENSION,
                actual: candidate.vector.len(),
            });
        }
        if self.episodes.len() < self.capacity {
            self.episodes.push(candidate);
            return Ok(true);
        }

        // A one-slot cache has no peer set against which to estimate
        // diversity; use deterministic FIFO rather than inventing an oracle.
        if self.capacity == 1 {
            self.episodes.remove(0);
            self.episodes.push(candidate);
            self.evicted += 1;
            return Ok(true);
        }

        let mut candidate_max_similarity = 0.0_f32;
        for stored in &self.episodes {
            candidate_max_similarity = candidate_max_similarity
                .max(candidate.vector.similarity(&stored.vector)?.max(0.0));
            self.admission_similarity_comparisons += 1;
        }
        let candidate_novelty = 1.0 - candidate_max_similarity;

        // Estimate each retained record's redundancy by its nearest peer.
        // The record whose nearest-peer distance is smallest is the most
        // redundant and is the only possible eviction candidate.
        let mut victim: Option<(usize, f32)> = None;
        for index in 0..self.episodes.len() {
            let mut max_peer_similarity = 0.0_f32;
            for other_index in 0..self.episodes.len() {
                if index == other_index {
                    continue;
                }
                max_peer_similarity = max_peer_similarity.max(
                    self.episodes[index]
                        .vector
                        .similarity(&self.episodes[other_index].vector)?
                        .max(0.0),
                );
                self.admission_similarity_comparisons += 1;
            }
            let uniqueness = 1.0 - max_peer_similarity;
            if victim.is_none_or(|(_, best_uniqueness)| uniqueness < best_uniqueness) {
                victim = Some((index, uniqueness));
            }
        }
        let (victim_index, victim_uniqueness) = victim.expect("full memory has a victim candidate");
        if candidate_novelty > victim_uniqueness + NOVELTY_MARGIN {
            self.episodes.remove(victim_index);
            self.episodes.push(candidate);
            self.evicted += 1;
            Ok(true)
        } else {
            self.rejected += 1;
            Ok(false)
        }
    }

    fn episodes(&self) -> &[PhysicsEpisode] {
        &self.episodes
    }

    fn query(&self, vector: &BipolarHV) -> Result<Option<Hit>, PhysicsHdcError> {
        top_hit(&self.episodes, vector)
    }
}

enum Store {
    Fifo(EpisodeMemory),
    Unbounded(EpisodeMemory),
    Diversity(DiversityMemory),
}

impl Store {
    fn insert(&mut self, episode: PhysicsEpisode) -> Result<bool, PhysicsHdcError> {
        match self {
            Self::Fifo(memory) | Self::Unbounded(memory) => {
                memory.insert(episode)?;
                Ok(true)
            }
            Self::Diversity(memory) => memory.insert(episode),
        }
    }

    fn episodes(&self) -> &[PhysicsEpisode] {
        match self {
            Self::Fifo(memory) | Self::Unbounded(memory) => memory.episodes(),
            Self::Diversity(memory) => memory.episodes(),
        }
    }

    fn query(&self, vector: &BipolarHV) -> Result<Option<Hit>, PhysicsHdcError> {
        match self {
            Self::Fifo(memory) | Self::Unbounded(memory) => Ok(memory
                .query(vector, 1)?
                .first()
                .map(|hit| Hit {
                    episode_id: hit.episode_id.clone(),
                    label: hit.label.clone(),
                    similarity: hit.similarity,
                })),
            Self::Diversity(memory) => memory.query(vector),
        }
    }

    fn policy(&self) -> &'static str {
        match self {
            Self::Fifo(_) => "fifo_k4",
            Self::Unbounded(_) => "unbounded_reference",
            Self::Diversity(_) => "diversity_novelty_k4",
        }
    }

    fn evicted(&self) -> usize {
        match self {
            Self::Fifo(memory) => memory.evicted_count() as usize,
            Self::Unbounded(_) => 0,
            Self::Diversity(memory) => memory.evicted,
        }
    }

    fn rejected(&self) -> usize {
        match self {
            Self::Diversity(memory) => memory.rejected,
            _ => 0,
        }
    }

    fn admission_comparisons(&self) -> usize {
        match self {
            Self::Diversity(memory) => memory.admission_similarity_comparisons,
            _ => 0,
        }
    }
}

fn payload_bytes(episodes: &[PhysicsEpisode]) -> (usize, usize) {
    let vector_bytes = episodes
        .iter()
        .map(|episode| episode.vector.values.len() * size_of::<i8>())
        .sum();
    let digest_bytes = episodes
        .iter()
        .map(|episode| size_of_val(episode.exact_digests.as_slice()))
        .sum();
    (vector_bytes, digest_bytes)
}

fn report() -> Result<String, Box<dyn std::error::Error>> {
    let events = stream();
    let mut stores = vec![
        Store::Fifo(EpisodeMemory::with_capacity(
            ENCODER_FINGERPRINT,
            DIMENSION,
            CAPACITY,
        )?),
        Store::Diversity(DiversityMemory::new(CAPACITY)?),
        Store::Unbounded(EpisodeMemory::new(ENCODER_FINGERPRINT, DIMENSION)),
    ];
    let mut accepted = [0_usize; 3];

    for episode in events.iter().cloned() {
        for (index, store) in stores.iter_mut().enumerate() {
            if store.insert(episode.clone())? {
                accepted[index] += 1;
            }
        }
    }

    let mut out = String::new();
    out.push_str("META\tformat\tEPISODE-RETENTION-001-v0.1\n");
    out.push_str("META\tscope\torthogonal HDC prototypes with redundant distractor writes; not physics or robot performance\n");
    out.push_str("META\tdimension\t16384\n");
    out.push_str("META\tcapacity\t4\n");
    out.push_str("META\tstream\t4 balanced anchors followed by 24 duplicate-A episodes\n");
    out.push_str("CASE_HEADER\tpolicy\tquery_label\tpredicted_label\tsimilarity\tcorrect\tretained_episodes\tvector_payload_bytes\tdigest_payload_bytes\n");

    let mut query_correct = [0_usize; 3];
    let queries = Class::ALL;
    for class in queries {
        let query = class_vector(class);
        for (index, store) in stores.iter().enumerate() {
            let hit = store.query(&query)?;
            let predicted = hit.as_ref().map(|h| h.label.as_str()).unwrap_or("none");
            let correct = predicted == class.label();
            query_correct[index] += usize::from(correct);
            let (vector_bytes, digest_bytes) = payload_bytes(store.episodes());
            let similarity = hit.map(|h| format!("{:.6}", h.similarity)).unwrap_or_else(|| "NA".to_owned());
            out.push_str(&format!(
                "CASE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                store.policy(),
                class.label(),
                predicted,
                similarity,
                u8::from(correct),
                store.episodes().len(),
                vector_bytes,
                digest_bytes
            ));
        }
    }

    out.push_str("SUMMARY_HEADER\tpolicy\tqueries\tcorrect\taccuracy\tretained_episodes\tclass_coverage\tvector_payload_bytes\tdigest_payload_bytes\twrites_accepted\tevicted\trejected\tsimilarity_comparisons\n");
    for (index, store) in stores.iter().enumerate() {
        let labels: BTreeSet<&str> = store.episodes().iter()
            .map(|episode| episode.metadata.label.as_str())
            .collect();
        let (vector_bytes, digest_bytes) = payload_bytes(store.episodes());
        let retrieval_comparisons = store.episodes().len() * queries.len();
        let total_comparisons = retrieval_comparisons + store.admission_comparisons();
        out.push_str(&format!(
            "SUMMARY\t{}\t{}\t{}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            store.policy(),
            queries.len(),
            query_correct[index],
            query_correct[index] as f64 / queries.len() as f64,
            store.episodes().len(),
            labels.len(),
            vector_bytes,
            digest_bytes,
            accepted[index],
            store.evicted(),
            store.rejected(),
            total_comparisons
        ));
    }
    Ok(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}", report()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diversity_admission_preserves_rare_orthogonal_classes() {
        let events = stream();
        let mut fifo = EpisodeMemory::with_capacity(ENCODER_FINGERPRINT, DIMENSION, CAPACITY).unwrap();
        let mut diverse = DiversityMemory::new(CAPACITY).unwrap();
        let mut unbounded = EpisodeMemory::new(ENCODER_FINGERPRINT, DIMENSION);
        for episode in events {
            fifo.insert(episode.clone()).unwrap();
            diverse.insert(episode.clone()).unwrap();
            unbounded.insert(episode).unwrap();
        }

        let fifo_coverage: BTreeSet<&str> = fifo.episodes().iter()
            .map(|episode| episode.metadata.label.as_str()).collect();
        let diverse_coverage: BTreeSet<&str> = diverse.episodes().iter()
            .map(|episode| episode.metadata.label.as_str()).collect();
        assert_eq!(fifo.len(), CAPACITY);
        assert_eq!(fifo_coverage.len(), 1);
        assert_eq!(diverse.episodes().len(), CAPACITY);
        assert_eq!(diverse_coverage.len(), 4);
        assert_eq!(diverse.rejected, REDUNDANT_EVENTS);
        assert_eq!(diverse.evicted, 0);

        let mut fifo_correct = 0;
        let mut diverse_correct = 0;
        let mut unbounded_correct = 0;
        for class in Class::ALL {
            let query = class_vector(class);
            fifo_correct += usize::from(fifo.query(&query, 1).unwrap()[0].label == class.label());
            diverse_correct += usize::from(diverse.query(&query).unwrap().unwrap().label == class.label());
            unbounded_correct += usize::from(unbounded.query(&query, 1).unwrap()[0].label == class.label());
        }
        assert_eq!(fifo_correct, 1);
        assert_eq!(diverse_correct, 4);
        assert_eq!(unbounded_correct, 4);
    }

    #[test]
    fn admission_decision_does_not_read_query_or_label_truth() {
        let source = include_str!("episode_memory_retention.rs");
        let start = source.find("fn insert(&mut self, candidate: PhysicsEpisode)").unwrap();
        let end = source[start..].find("\n    fn episodes(").unwrap() + start;
        let selector = &source[start..end];
        assert!(!selector.contains("metadata.label"));
        assert!(!selector.contains("query_label"));
        assert!(!selector.contains("true_label"));
        assert!(!selector.contains("Class::ALL"));
    }

    #[test]
    fn report_is_deterministic_and_has_fixed_schema() {
        let first = report().unwrap();
        assert_eq!(first, report().unwrap());
        assert_eq!(first.lines().filter(|line| line.starts_with("CASE_HEADER\t")).count(), 1);
        assert_eq!(first.lines().filter(|line| line.starts_with("CASE\t")).count(), 12);
        assert_eq!(first.lines().filter(|line| line.starts_with("SUMMARY_HEADER\t")).count(), 1);
        assert_eq!(first.lines().filter(|line| line.starts_with("SUMMARY\t")).count(), 3);
        for line in first.lines() {
            if line.starts_with("CASE_HEADER\t") || line.starts_with("CASE\t") {
                assert_eq!(line.split('\t').count(), 9, "case schema: {line}");
            }
            if line.starts_with("SUMMARY_HEADER\t") || line.starts_with("SUMMARY\t") {
                assert_eq!(line.split('\t').count(), 13, "summary schema: {line}");
            }
        }
    }
}
