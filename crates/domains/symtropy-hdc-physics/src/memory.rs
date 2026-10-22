// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Temporal physics episodes and a deterministic brute-force reference memory.
//!
//! The reference index is intentionally simple and auditable. Production
//! deployments may replace it with an ANN index, but research baselines should
//! retain this implementation so retrieval quality is not conflated with index
//! approximation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use symtropy_hdc_core::{BipolarBundle, BipolarHV};

use crate::{EncodedPhysicsFrame, ExactStateDigest, PhysicsFrameEncoder, PhysicsHdcError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EpisodeMetadata {
    pub label: String,
    pub run_id: String,
    pub seed: u64,
    pub tags: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicsEpisode {
    pub id: String,
    pub encoder_fingerprint: u64,
    pub first_tick: u64,
    pub last_tick: u64,
    pub frame_count: usize,
    pub temporal_stride: i64,
    pub exact_digests: Vec<ExactStateDigest>,
    pub vector: BipolarHV,
    pub metadata: EpisodeMetadata,
}

#[derive(Debug, Clone)]
pub struct EpisodeBuilder {
    id: String,
    encoder_fingerprint: u64,
    dimension: usize,
    temporal_stride: i64,
    tie_breaker: BipolarHV,
    bundle: BipolarBundle,
    exact_digests: Vec<ExactStateDigest>,
    first_tick: Option<u64>,
    last_tick: Option<u64>,
    frame_count: usize,
    metadata: EpisodeMetadata,
}

impl EpisodeBuilder {
    pub(crate) fn new(
        id: impl Into<String>,
        encoder_fingerprint: u64,
        dimension: usize,
        temporal_stride: i64,
        tie_breaker: BipolarHV,
        metadata: EpisodeMetadata,
    ) -> Self {
        Self {
            id: id.into(),
            encoder_fingerprint,
            dimension,
            temporal_stride,
            tie_breaker,
            bundle: BipolarBundle::new(dimension),
            exact_digests: Vec::new(),
            first_tick: None,
            last_tick: None,
            frame_count: 0,
            metadata,
        }
    }

    /// Add a frame in temporal order. Its vector is permuted by the frame's
    /// relative index before bundling, so reversed episodes remain distinct.
    pub fn push<const D: usize>(
        &mut self,
        frame: &EncodedPhysicsFrame<D>,
    ) -> Result<(), PhysicsHdcError> {
        if frame.encoder_fingerprint != self.encoder_fingerprint {
            return Err(PhysicsHdcError::EncoderMismatch {
                expected: self.encoder_fingerprint,
                actual: frame.encoder_fingerprint,
            });
        }
        if frame.vector.len() != self.dimension {
            return Err(PhysicsHdcError::VectorDimensionMismatch {
                expected: self.dimension,
                actual: frame.vector.len(),
            });
        }
        let offset = self.temporal_stride.saturating_mul(self.frame_count as i64);
        self.bundle.add(&frame.vector.permute(offset))?;
        self.exact_digests.push(frame.exact_digest);
        self.first_tick.get_or_insert(frame.tick);
        self.last_tick = Some(frame.tick);
        self.frame_count += 1;
        Ok(())
    }

    pub fn finish(self) -> Result<PhysicsEpisode, PhysicsHdcError> {
        if self.frame_count == 0 {
            return Err(PhysicsHdcError::EmptyEpisode);
        }
        Ok(PhysicsEpisode {
            id: self.id,
            encoder_fingerprint: self.encoder_fingerprint,
            first_tick: self.first_tick.expect("non-empty episode has a first tick"),
            last_tick: self.last_tick.expect("non-empty episode has a last tick"),
            frame_count: self.frame_count,
            temporal_stride: self.temporal_stride,
            exact_digests: self.exact_digests,
            vector: self.bundle.finish(&self.tie_breaker)?,
            metadata: self.metadata,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalHit {
    pub index: usize,
    pub episode_id: String,
    pub label: String,
    pub similarity: f32,
}

/// Exact linear-scan associative memory. Results are ordered by descending
/// similarity and then episode id, making ties deterministic.
#[derive(Debug, Clone)]
pub struct EpisodeMemory {
    encoder_fingerprint: u64,
    dimension: usize,
    episodes: Vec<PhysicsEpisode>,
    max_episodes: Option<usize>,
    evicted_count: u64,
}

impl EpisodeMemory {
    pub fn new(encoder_fingerprint: u64, dimension: usize) -> Self {
        Self {
            encoder_fingerprint,
            dimension,
            episodes: Vec::new(),
            max_episodes: None,
            evicted_count: 0,
        }
    }

    /// Construct a FIFO-bounded reference memory. When full, insertion of a
    /// valid new episode evicts the oldest retained episode. A zero capacity
    /// is rejected so a caller cannot accidentally create a memory that
    /// reports successful inserts while retaining nothing.
    pub fn with_capacity(
        encoder_fingerprint: u64,
        dimension: usize,
        max_episodes: usize,
    ) -> Result<Self, PhysicsHdcError> {
        if max_episodes == 0 {
            return Err(PhysicsHdcError::InvalidConfig(
                "episode memory capacity must be greater than zero".to_owned(),
            ));
        }
        Ok(Self {
            encoder_fingerprint,
            dimension,
            episodes: Vec::new(),
            max_episodes: Some(max_episodes),
            evicted_count: 0,
        })
    }

    pub fn from_encoder(encoder: &PhysicsFrameEncoder) -> Self {
        Self::new(
            encoder.config().fingerprint(),
            encoder.config().hdc.dimension,
        )
    }

    pub fn from_encoder_with_capacity(
        encoder: &PhysicsFrameEncoder,
        max_episodes: usize,
    ) -> Result<Self, PhysicsHdcError> {
        Self::with_capacity(
            encoder.config().fingerprint(),
            encoder.config().hdc.dimension,
            max_episodes,
        )
    }

    /// Configured maximum retained episode count; None preserves legacy
    /// unbounded behavior of new and from_encoder.
    pub fn capacity(&self) -> Option<usize> {
        self.max_episodes
    }

    /// Monotonic count of payload episodes evicted by the FIFO capacity policy.
    pub fn evicted_count(&self) -> u64 {
        self.evicted_count
    }

    pub fn len(&self) -> usize {
        self.episodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }

    pub fn episodes(&self) -> &[PhysicsEpisode] {
        &self.episodes
    }

    pub fn insert(&mut self, episode: PhysicsEpisode) -> Result<usize, PhysicsHdcError> {
        if episode.encoder_fingerprint != self.encoder_fingerprint {
            return Err(PhysicsHdcError::EncoderMismatch {
                expected: self.encoder_fingerprint,
                actual: episode.encoder_fingerprint,
            });
        }
        if episode.vector.len() != self.dimension {
            return Err(PhysicsHdcError::VectorDimensionMismatch {
                expected: self.dimension,
                actual: episode.vector.len(),
            });
        }
        if self
            .episodes
            .iter()
            .any(|existing| existing.id == episode.id)
        {
            return Err(PhysicsHdcError::DuplicateEpisodeId(episode.id));
        }
        if let Some(capacity) = self.max_episodes
            && self.episodes.len() >= capacity
        {
            // All identity/encoder validations happen before eviction. A
            // rejected write must never displace a valid retained episode.
            self.episodes.remove(0);
            self.evicted_count = self.evicted_count.saturating_add(1);
        }
        let index = self.episodes.len();
        self.episodes.push(episode);
        Ok(index)
    }

    pub fn query(
        &self,
        vector: &BipolarHV,
        k: usize,
    ) -> Result<Vec<RetrievalHit>, PhysicsHdcError> {
        if vector.len() != self.dimension {
            return Err(PhysicsHdcError::VectorDimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }
        let mut hits = self
            .episodes
            .iter()
            .enumerate()
            .map(|(index, episode)| {
                Ok(RetrievalHit {
                    index,
                    episode_id: episode.id.clone(),
                    label: episode.metadata.label.clone(),
                    similarity: vector.similarity(&episode.vector)?,
                })
            })
            .collect::<Result<Vec<_>, PhysicsHdcError>>()?;
        hits.sort_by(|left, right| {
            right
                .similarity
                .total_cmp(&left.similarity)
                .then_with(|| left.episode_id.cmp(&right.episode_id))
        });
        hits.truncate(k.min(hits.len()));
        Ok(hits)
    }

    pub fn query_episode(
        &self,
        episode: &PhysicsEpisode,
        k: usize,
        exclude_same_id: bool,
    ) -> Result<Vec<RetrievalHit>, PhysicsHdcError> {
        if episode.encoder_fingerprint != self.encoder_fingerprint {
            return Err(PhysicsHdcError::EncoderMismatch {
                expected: self.encoder_fingerprint,
                actual: episode.encoder_fingerprint,
            });
        }
        let mut hits = self.query(&episode.vector, self.episodes.len())?;
        if exclude_same_id {
            hits.retain(|hit| hit.episode_id != episode.id);
        }
        hits.truncate(k.min(hits.len()));
        Ok(hits)
    }

    /// `0` means an identical stored pattern; values near `1` indicate little
    /// positive similarity. Negative similarity is clamped to maximum novelty.
    pub fn novelty(&self, vector: &BipolarHV) -> Result<f32, PhysicsHdcError> {
        let nearest = self.query(vector, 1)?;
        Ok(nearest
            .first()
            .map(|hit| 1.0 - hit.similarity.max(0.0))
            .unwrap_or(1.0))
    }
}

impl PhysicsFrameEncoder {
    pub fn episode_builder(
        &self,
        id: impl Into<String>,
        metadata: EpisodeMetadata,
        temporal_stride: i64,
    ) -> EpisodeBuilder {
        EpisodeBuilder::new(
            id,
            self.config().fingerprint(),
            self.config().hdc.dimension,
            temporal_stride,
            self.memory.tie_breaker("physics-episode"),
            metadata,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IdentityPolicy, PhysicsEncoderConfig, ReferenceFramePolicy};
    use symtropy_math::Point;
    use symtropy_physics::PhysicsWorld;

    fn encoder() -> PhysicsFrameEncoder {
        let mut config = PhysicsEncoderConfig::default();
        config.hdc.dimension = 4_096;
        config.hdc.scalar_levels = 129;
        config.identity_policy = IdentityPolicy::None;
        config.reference_frame = ReferenceFramePolicy::World;
        // The default `position_extent`/`velocity_extent` (1_000.0/250.0) are
        // tuned for real-world-scale simulations. These tests use a toy
        // scene (positions under 1 unit, velocities around +/-2) that is
        // entirely swallowed by one quantization bucket at the default
        // resolution (bucket width = 2*extent/(scalar_levels-1) ~= 15.6 for
        // position, ~3.9 for velocity) -- every tick/direction collapses to
        // the same encoded level, so temporal-order and motion-direction
        // tests can't distinguish anything regardless of how correct the
        // permutation/bundling/retrieval logic is. Scale the extents down to
        // match this test scene's actual magnitude.
        config.position_extent = 2.0;
        config.velocity_extent = 5.0;
        PhysicsFrameEncoder::new(config).unwrap()
    }

    fn moving_frames(direction: f64) -> Vec<EncodedPhysicsFrame<2>> {
        let encoder = encoder();
        let mut world = PhysicsWorld::<2>::default();
        let body = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(body).unwrap().linear_velocity[0] = direction;
        let mut frames = Vec::new();
        for tick in 0..8 {
            frames.push(encoder.encode_world(tick, &world).unwrap());
            world.step(1.0 / 30.0);
        }
        frames
    }

    fn make_episode(
        encoder: &PhysicsFrameEncoder,
        id: &str,
        direction: f64,
    ) -> PhysicsEpisode {
        let mut world = PhysicsWorld::<2>::default();
        let body = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(body).unwrap().linear_velocity[0] = direction;
        let mut builder = encoder.episode_builder(
            id,
            EpisodeMetadata {
                label: id.to_owned(),
                ..EpisodeMetadata::default()
            },
            53,
        );
        for tick in 0..8 {
            let frame = encoder.encode_world(tick, &world).unwrap();
            builder.push(&frame).unwrap();
            world.step(1.0 / 30.0);
        }
        builder.finish().unwrap()
    }

    #[test]
    fn temporal_order_changes_episode_vector() {
        let encoder = encoder();
        let frames = moving_frames(2.0);
        let mut forward = encoder.episode_builder("forward", EpisodeMetadata::default(), 97);
        for frame in &frames {
            forward.push(frame).unwrap();
        }
        let mut reverse = encoder.episode_builder("reverse", EpisodeMetadata::default(), 97);
        for frame in frames.iter().rev() {
            reverse.push(frame).unwrap();
        }
        assert_ne!(
            forward.finish().unwrap().vector,
            reverse.finish().unwrap().vector
        );
    }

    #[test]
    fn episode_retains_exact_frame_links() {
        let encoder = encoder();
        let frames = moving_frames(1.0);
        let expected: Vec<_> = frames.iter().map(|frame| frame.exact_digest).collect();
        let mut builder = encoder.episode_builder("run", EpisodeMetadata::default(), 31);
        for frame in &frames {
            builder.push(frame).unwrap();
        }
        assert_eq!(builder.finish().unwrap().exact_digests, expected);
    }

    #[test]
    fn nearest_episode_retrieval_separates_motion_direction() {
        let encoder = encoder();
        let build = |id: &str, label: &str, frames: Vec<EncodedPhysicsFrame<2>>| {
            let mut builder = encoder.episode_builder(
                id,
                EpisodeMetadata {
                    label: label.to_owned(),
                    ..EpisodeMetadata::default()
                },
                53,
            );
            for frame in &frames {
                builder.push(frame).unwrap();
            }
            builder.finish().unwrap()
        };
        let right = build("right-train", "right", moving_frames(2.0));
        let left = build("left-train", "left", moving_frames(-2.0));
        let query = build("right-query", "right", moving_frames(1.8));
        let mut memory = EpisodeMemory::from_encoder(&encoder);
        memory.insert(right).unwrap();
        memory.insert(left).unwrap();
        let hits = memory.query_episode(&query, 1, false).unwrap();
        assert_eq!(hits[0].label, "right");
    }

    #[test]
    fn zero_capacity_is_rejected() {
        let encoder = encoder();
        assert!(matches!(
            EpisodeMemory::from_encoder_with_capacity(&encoder, 0),
            Err(PhysicsHdcError::InvalidConfig(_))
        ));
    }

    #[test]
    fn bounded_memory_evicts_oldest_episode_and_reports_count() {
        let encoder = encoder();
        let mut memory = EpisodeMemory::from_encoder_with_capacity(&encoder, 2).unwrap();
        assert_eq!(memory.capacity(), Some(2));
        assert_eq!(memory.evicted_count(), 0);
        memory.insert(make_episode(&encoder, "episode-0", 1.0)).unwrap();
        memory.insert(make_episode(&encoder, "episode-1", 1.5)).unwrap();
        memory.insert(make_episode(&encoder, "episode-2", 2.0)).unwrap();

        assert_eq!(memory.len(), 2);
        assert_eq!(memory.evicted_count(), 1);
        assert_eq!(
            memory.episodes().iter().map(|episode| episode.id.as_str()).collect::<Vec<_>>(),
            vec!["episode-1", "episode-2"]
        );
    }

    #[test]
    fn invalid_writes_do_not_evict_valid_retained_episodes() {
        let encoder = encoder();
        let other_encoder = PhysicsFrameEncoder::new(PhysicsEncoderConfig::default()).unwrap();
        let mut memory = EpisodeMemory::from_encoder_with_capacity(&encoder, 2).unwrap();
        memory.insert(make_episode(&encoder, "keep-0", 1.0)).unwrap();
        memory.insert(make_episode(&encoder, "keep-1", -1.0)).unwrap();

        let duplicate = make_episode(&encoder, "keep-0", 1.0);
        assert!(matches!(
            memory.insert(duplicate),
            Err(PhysicsHdcError::DuplicateEpisodeId(_))
        ));
        let mismatched = make_episode(&other_encoder, "wrong-encoder", 1.0);
        assert!(matches!(
            memory.insert(mismatched),
            Err(PhysicsHdcError::EncoderMismatch { .. })
        ));

        assert_eq!(memory.evicted_count(), 0);
        assert_eq!(
            memory.episodes().iter().map(|episode| episode.id.as_str()).collect::<Vec<_>>(),
            vec!["keep-0", "keep-1"]
        );
    }

    #[test]
    fn bounded_retrieval_matches_reference_over_the_same_retained_set() {
        let encoder = encoder();
        let episode_0 = make_episode(&encoder, "episode-0", 1.0);
        let episode_1 = make_episode(&encoder, "episode-1", -1.0);
        let episode_2 = make_episode(&encoder, "episode-2", 1.8);
        let query = make_episode(&encoder, "query", 1.7);

        let mut bounded = EpisodeMemory::from_encoder_with_capacity(&encoder, 2).unwrap();
        bounded.insert(episode_0).unwrap();
        bounded.insert(episode_1.clone()).unwrap();
        bounded.insert(episode_2.clone()).unwrap();

        let mut retained_reference = EpisodeMemory::from_encoder(&encoder);
        retained_reference.insert(episode_1).unwrap();
        retained_reference.insert(episode_2).unwrap();

        let bounded_hit = bounded.query_episode(&query, 1, true).unwrap();
        let reference_hit = retained_reference.query_episode(&query, 1, true).unwrap();
        assert_eq!(bounded_hit, reference_hit);
    }

    #[test]
    fn legacy_constructor_remains_unbounded() {
        let encoder = encoder();
        let mut memory = EpisodeMemory::from_encoder(&encoder);
        for idx in 0..3 {
            memory.insert(make_episode(&encoder, &format!("episode-{idx}"), idx as f64 + 1.0)).unwrap();
        }
        assert_eq!(memory.capacity(), None);
        assert_eq!(memory.len(), 3);
        assert_eq!(memory.evicted_count(), 0);
    }

    #[test]
    fn duplicate_episode_ids_are_rejected() {
        let encoder = encoder();
        let mut builder = encoder.episode_builder("same", EpisodeMetadata::default(), 1);
        builder.push(&moving_frames(1.0)[0]).unwrap();
        let episode = builder.finish().unwrap();
        let mut memory = EpisodeMemory::from_encoder(&encoder);
        memory.insert(episode.clone()).unwrap();
        assert!(matches!(
            memory.insert(episode),
            Err(PhysicsHdcError::DuplicateEpisodeId(_))
        ));
    }
}
