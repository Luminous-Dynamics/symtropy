// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Backend-neutral broadphase pair-set evaluation.
//! The independent oracle must enumerate all eligible pairs whose frozen
//! conservative AABBs overlap. This crate validates and scores pair sets; it
//! does not compute overlaps or prove oracle completeness.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ColliderKey {
    body_slot: u32,
    body_generation: u32,
    collider_id: u32,
    subshape_path: Vec<u32>,
}

impl ColliderKey {
    #[must_use]
    pub fn new(body_slot: u32, body_generation: u32, collider_id: u32, subshape_path: Vec<u32>) -> Self {
        Self { body_slot, body_generation, collider_id, subshape_path }
    }

    #[must_use]
    pub fn body_slot(&self) -> u32 { self.body_slot }

    #[must_use]
    pub fn body_generation(&self) -> u32 { self.body_generation }

    #[must_use]
    pub fn collider_id(&self) -> u32 { self.collider_id }

    #[must_use]
    pub fn subshape_path(&self) -> &[u32] { &self.subshape_path }
}

/// Unordered collider pair with canonical endpoint ordering.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PairKey {
    first: ColliderKey,
    second: ColliderKey,
}

impl PairKey {
    pub fn new(left: ColliderKey, right: ColliderKey) -> Result<Self, PairSetError> {
        if left == right { return Err(PairSetError::SelfPair); }
        let (first, second) = if left < right { (left, right) } else { (right, left) };
        Ok(Self { first, second })
    }

    #[must_use]
    pub fn first(&self) -> &ColliderKey { &self.first }

    #[must_use]
    pub fn second(&self) -> &ColliderKey { &self.second }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleCoverage {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameReceipt {
    world_revision: u64,
    step: u64,
    aabb_snapshot_sha256: String,
    collision_filter_sha256: String,
    oracle_id: String,
    oracle_version: String,
    oracle_coverage: OracleCoverage,
}

impl FrameReceipt {
    #[must_use]
    pub fn new(
        world_revision: u64,
        step: u64,
        aabb_snapshot_sha256: impl Into<String>,
        collision_filter_sha256: impl Into<String>,
        oracle_id: impl Into<String>,
        oracle_version: impl Into<String>,
        oracle_coverage: OracleCoverage,
    ) -> Self {
        Self {
            world_revision,
            step,
            aabb_snapshot_sha256: aabb_snapshot_sha256.into(),
            collision_filter_sha256: collision_filter_sha256.into(),
            oracle_id: oracle_id.into(),
            oracle_version: oracle_version.into(),
            oracle_coverage,
        }
    }

    #[must_use]
    pub fn world_revision(&self) -> u64 { self.world_revision }

    #[must_use]
    pub fn step(&self) -> u64 { self.step }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        is_lower_hex(&self.aabb_snapshot_sha256, 64)
            && is_lower_hex(&self.collision_filter_sha256, 64)
            && !self.oracle_id.trim().is_empty()
            && !self.oracle_version.trim().is_empty()
    }

    #[must_use]
    pub fn aabb_snapshot_sha256(&self) -> &str { &self.aabb_snapshot_sha256 }

    #[must_use]
    pub fn collision_filter_sha256(&self) -> &str { &self.collision_filter_sha256 }

    #[must_use]
    pub fn oracle_id(&self) -> &str { &self.oracle_id }

    #[must_use]
    pub fn oracle_version(&self) -> &str { &self.oracle_version }

    #[must_use]
    pub fn oracle_coverage(&self) -> OracleCoverage { self.oracle_coverage }
}

/// Per-frame workload and cost decomposition. Current object classes are
/// disjoint: static + awake dynamic + sleeping dynamic == collider_count.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkloadFrame {
    pub collider_count: u64,
    pub static_colliders: u64,
    pub awake_dynamic_colliders: u64,
    pub sleeping_dynamic_colliders: u64,
    pub moved_colliders: u64,
    pub added_colliders: u64,
    pub removed_colliders: u64,
    pub update_ns: u64,
    pub rebuild_ns: u64,
    pub pair_generation_ns: u64,
    pub full_step_ns: u64,
    pub node_visits: u64,
    pub leaf_visits: u64,
    pub reinserts: u64,
    pub pair_cache_additions: u64,
    pub pair_cache_removals: u64,
    pub peak_memory_bytes: u64,
}

impl WorkloadFrame {
    pub fn validate(self) -> Result<(), PairSetError> {
        let classified = self.static_colliders
            .checked_add(self.awake_dynamic_colliders)
            .and_then(|sum| sum.checked_add(self.sleeping_dynamic_colliders))
            .ok_or(PairSetError::WorkloadCountOverflow)?;
        if classified != self.collider_count {
            return Err(PairSetError::InvalidWorkloadClassification);
        }
        if self.moved_colliders > self.awake_dynamic_colliders {
            return Err(PairSetError::MovedCountExceedsAwakeDynamics);
        }
        if self.update_ns.checked_add(self.rebuild_ns)
            .and_then(|sum| sum.checked_add(self.pair_generation_ns)).is_none()
        {
            return Err(PairSetError::WorkloadCostOverflow);
        }
        Ok(())
    }

    #[must_use]
    pub fn broadphase_cost_ns(self) -> Option<u64> {
        self.update_ns.checked_add(self.rebuild_ns)?.checked_add(self.pair_generation_ns)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkloadQuantiles {
    pub frame_count: u64,
    pub p50_broadphase_ns: Option<u64>,
    pub p95_broadphase_ns: Option<u64>,
    pub p99_broadphase_ns: Option<u64>,
    pub p50_full_step_ns: Option<u64>,
    pub p95_full_step_ns: Option<u64>,
    pub p99_full_step_ns: Option<u64>,
    pub total_node_visits: u64,
    pub total_leaf_visits: u64,
    pub total_reinserts: u64,
    pub total_pair_cache_additions: u64,
    pub total_pair_cache_removals: u64,
    pub peak_memory_bytes: u64,
}

/// Summarize a frozen workload trace using nearest-rank p50/p95/p99.
/// The environment/timer-resolution receipt is held by the calling run; this
/// function preserves the raw nanosecond samples, including zero observations.
pub fn summarize_workload_frames(
    frames: &[WorkloadFrame],
) -> Result<WorkloadQuantiles, PairSetError> {
    if frames.is_empty() {
        return Err(PairSetError::EmptyWorkloadSeries);
    }

    let mut broadphase_times = Vec::with_capacity(frames.len());
    let mut full_step_times = Vec::with_capacity(frames.len());
    let mut summary = WorkloadQuantiles {
        frame_count: frames.len() as u64,
        ..WorkloadQuantiles::default()
    };
    for frame in frames {
        frame.validate()?;
        broadphase_times.push(
            frame
                .broadphase_cost_ns()
                .ok_or(PairSetError::WorkloadCostOverflow)?,
        );
        full_step_times.push(frame.full_step_ns);
        summary.total_node_visits = summary
            .total_node_visits
            .checked_add(frame.node_visits)
            .ok_or(PairSetError::WorkloadCountOverflow)?;
        summary.total_leaf_visits = summary
            .total_leaf_visits
            .checked_add(frame.leaf_visits)
            .ok_or(PairSetError::WorkloadCountOverflow)?;
        summary.total_reinserts = summary
            .total_reinserts
            .checked_add(frame.reinserts)
            .ok_or(PairSetError::WorkloadCountOverflow)?;
        summary.total_pair_cache_additions = summary
            .total_pair_cache_additions
            .checked_add(frame.pair_cache_additions)
            .ok_or(PairSetError::WorkloadCountOverflow)?;
        summary.total_pair_cache_removals = summary
            .total_pair_cache_removals
            .checked_add(frame.pair_cache_removals)
            .ok_or(PairSetError::WorkloadCountOverflow)?;
        summary.peak_memory_bytes = summary.peak_memory_bytes.max(frame.peak_memory_bytes);
    }

    summary.p50_broadphase_ns = nearest_rank(&mut broadphase_times, 50);
    summary.p95_broadphase_ns = nearest_rank(&mut broadphase_times, 95);
    summary.p99_broadphase_ns = nearest_rank(&mut broadphase_times, 99);
    summary.p50_full_step_ns = nearest_rank(&mut full_step_times, 50);
    summary.p95_full_step_ns = nearest_rank(&mut full_step_times, 95);
    summary.p99_full_step_ns = nearest_rank(&mut full_step_times, 99);
    Ok(summary)
}

fn nearest_rank(values: &mut [u64], percentile: u8) -> Option<u64> {
    if values.is_empty() || !(1..=100).contains(&percentile) {
        return None;
    }
    values.sort_unstable();
    let rank = (values.len() * usize::from(percentile)).div_ceil(100).max(1);
    values.get(rank - 1).copied()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairSetError {
    InvalidFrameReceipt,
    EmptyWorkloadSeries,
    OracleCoverageIncomplete,
    WorkloadPopulationMismatch,
    DuplicateColliderIdentity,
    SelfPair,
    PairReferencesUnknownCollider,
    DuplicateOraclePair,
    DuplicateCandidatePair,
    EligiblePairCountExceedsPopulation,
    OracleCountExceedsEligiblePopulation,
    CandidateCountExceedsEligiblePopulation,
    InvalidWorkloadClassification,
    MovedCountExceedsAwakeDynamics,
    WorkloadCountOverflow,
    WorkloadCostOverflow,
}

impl fmt::Display for PairSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl Error for PairSetError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PairSetCounts {
    pub collider_count: u64,
    pub possible_unfiltered_pairs: u128,
    pub eligible_pairs_after_filter: u64,
    pub oracle_overlapping_pairs: u64,
    pub emitted_candidate_pairs: u64,
    pub true_positive_pairs: u64,
    pub false_positive_pairs: u64,
    pub false_negative_pairs: u64,
}

impl PairSetCounts {
    #[must_use]
    pub fn candidate_recall_basis_points(self) -> Option<u64> {
        rate_basis_points(self.true_positive_pairs, self.oracle_overlapping_pairs)
    }

    #[must_use]
    pub fn candidate_precision_basis_points(self) -> Option<u64> {
        rate_basis_points(self.true_positive_pairs, self.emitted_candidate_pairs)
    }

    #[must_use]
    pub fn candidate_false_positive_rate_basis_points(self) -> Option<u64> {
        rate_basis_points(
            self.false_positive_pairs,
            self.eligible_pairs_after_filter.checked_sub(self.oracle_overlapping_pairs)?,
        )
    }

    #[must_use]
    pub fn selectivity_basis_points(self) -> Option<u64> {
        rate_basis_points(self.emitted_candidate_pairs, self.eligible_pairs_after_filter)
    }

    #[must_use]
    pub fn reduction_basis_points(self) -> Option<u64> {
        rate_basis_points(
            self.eligible_pairs_after_filter.checked_sub(self.emitted_candidate_pairs)?,
            self.eligible_pairs_after_filter,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairSetScore {
    pub frame: FrameReceipt,
    pub workload: WorkloadFrame,
    pub counts: PairSetCounts,
}

impl PairSetScore {
    /// Empty-oracle frames are not benchmark-qualified successes.
    #[must_use]
    pub fn benchmark_qualified(&self) -> bool {
        self.frame.is_valid()
            && self.workload.validate().is_ok()
            && self.counts.oracle_overlapping_pairs > 0
            && self.counts.false_negative_pairs == 0
    }
}

/// Score output pairs against an independent brute-force overlap oracle for the
/// same frozen collider universe, AABB snapshot, and collision filter.
///
/// eligible_pair_count is the number of pairs allowed by the frozen filter
/// before AABB overlap is applied. Both implementations must use the same
/// collision-filter SHA-256 in the frame receipt.
pub fn evaluate_pair_sets(
    frame: FrameReceipt,
    workload: WorkloadFrame,
    colliders: &[ColliderKey],
    eligible_pair_count: u64,
    oracle_pairs: &[PairKey],
    candidate_pairs: &[PairKey],
) -> Result<PairSetScore, Vec<PairSetError>> {
    let mut errors = Vec::new();
    if !frame.is_valid() { errors.push(PairSetError::InvalidFrameReceipt); }
    if let Err(error) = workload.validate() { errors.push(error); }

    let mut collider_set = HashSet::with_capacity(colliders.len());
    for collider in colliders {
        if !collider_set.insert(collider.clone()) {
            errors.push(PairSetError::DuplicateColliderIdentity);
        }
    }

    let collider_count = colliders.len() as u64;
    if workload.collider_count != collider_count {
        errors.push(PairSetError::WorkloadPopulationMismatch);
    }
    if frame.oracle_coverage != OracleCoverage::Complete {
        errors.push(PairSetError::OracleCoverageIncomplete);
    }
    let possible_unfiltered_pairs =
        u128::from(collider_count) * u128::from(collider_count.saturating_sub(1)) / 2;
    if u128::from(eligible_pair_count) > possible_unfiltered_pairs {
        errors.push(PairSetError::EligiblePairCountExceedsPopulation);
    }
    if oracle_pairs.len() as u64 > eligible_pair_count {
        errors.push(PairSetError::OracleCountExceedsEligiblePopulation);
    }
    if candidate_pairs.len() as u64 > eligible_pair_count {
        errors.push(PairSetError::CandidateCountExceedsEligiblePopulation);
    }

    let oracle_set = validate_pairs(
        oracle_pairs, &collider_set, PairSetError::DuplicateOraclePair, &mut errors,
    );
    let candidate_set = validate_pairs(
        candidate_pairs, &collider_set, PairSetError::DuplicateCandidatePair, &mut errors,
    );
    if !errors.is_empty() { return Err(errors); }

    let true_positive = candidate_set.intersection(&oracle_set).count() as u64;
    let false_positive = candidate_set.difference(&oracle_set).count() as u64;
    let false_negative = oracle_set.difference(&candidate_set).count() as u64;
    Ok(PairSetScore {
        frame,
        workload,
        counts: PairSetCounts {
            collider_count,
            possible_unfiltered_pairs,
            eligible_pairs_after_filter: eligible_pair_count,
            oracle_overlapping_pairs: oracle_set.len() as u64,
            emitted_candidate_pairs: candidate_set.len() as u64,
            true_positive_pairs: true_positive,
            false_positive_pairs: false_positive,
            false_negative_pairs: false_negative,
        },
    })
}

fn validate_pairs(
    pairs: &[PairKey],
    colliders: &HashSet<ColliderKey>,
    duplicate_error: PairSetError,
    errors: &mut Vec<PairSetError>,
) -> HashSet<PairKey> {
    let mut result = HashSet::with_capacity(pairs.len());
    for pair in pairs {
        if !colliders.contains(pair.first()) || !colliders.contains(pair.second()) {
            errors.push(PairSetError::PairReferencesUnknownCollider);
        }
        if !result.insert(pair.clone()) { errors.push(duplicate_error); }
    }
    result
}

fn rate_basis_points(numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 { return None; }
    Some((u128::from(numerator) * 10_000 / u128::from(denominator)) as u64)
}

fn is_lower_hex(value: &str, expected_length: usize) -> bool {
    value.len() == expected_length
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collider(slot: u32, generation: u32, id: u32) -> ColliderKey {
        ColliderKey::new(slot, generation, id, Vec::new())
    }
    fn pair(a: &ColliderKey, b: &ColliderKey) -> PairKey {
        PairKey::new(a.clone(), b.clone()).unwrap()
    }
    fn frame() -> FrameReceipt {
        FrameReceipt::new(
            42, 7,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "brute-force-aabb-overlap", "v1", OracleCoverage::Complete,
        )
    }
    fn workload(n: u64) -> WorkloadFrame {
        WorkloadFrame {
            collider_count: n,
            static_colliders: n / 2,
            awake_dynamic_colliders: n - n / 2,
            moved_colliders: n - n / 2,
            update_ns: 100,
            rebuild_ns: 50,
            pair_generation_ns: 250,
            full_step_ns: 1_000,
            ..WorkloadFrame::default()
        }
    }

    #[test]
    fn canonical_pair_identity_ignores_endpoint_order() {
        let a = collider(1, 4, 8);
        let b = collider(2, 1, 3);
        assert_eq!(pair(&a, &b), pair(&b, &a));
        assert_eq!(PairKey::new(a.clone(), a), Err(PairSetError::SelfPair));
    }

    #[test]
    fn body_generation_prevents_slot_reuse_aliasing() {
        let old = collider(7, 1, 3);
        let new = collider(7, 2, 3);
        let other = collider(8, 1, 3);
        assert_ne!(pair(&old, &other), pair(&new, &other));
    }

    #[test]
    fn computes_pair_recall_precision_and_false_positive_rate() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0), collider(2, 1, 0), collider(3, 1, 0)];
        let oracle = vec![pair(&cs[0], &cs[1]), pair(&cs[2], &cs[3])];
        let candidates = vec![pair(&cs[0], &cs[1]), pair(&cs[1], &cs[2]), pair(&cs[2], &cs[3])];
        let score = evaluate_pair_sets(frame(), workload(4), &cs, 6, &oracle, &candidates).unwrap();
        assert_eq!(score.counts.possible_unfiltered_pairs, 6);
        assert_eq!(score.counts.oracle_overlapping_pairs, 2);
        assert_eq!(score.counts.emitted_candidate_pairs, 3);
        assert_eq!(score.counts.true_positive_pairs, 2);
        assert_eq!(score.counts.false_positive_pairs, 1);
        assert_eq!(score.counts.false_negative_pairs, 0);
        assert_eq!(score.counts.candidate_recall_basis_points(), Some(10_000));
        assert_eq!(score.counts.candidate_precision_basis_points(), Some(6_666));
        assert_eq!(score.counts.candidate_false_positive_rate_basis_points(), Some(2_500));
        assert!(score.benchmark_qualified());
    }

    #[test]
    fn missed_oracle_pair_invalidates_benchmark_qualification() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0), collider(2, 1, 0)];
        let oracle = vec![pair(&cs[0], &cs[1]), pair(&cs[1], &cs[2])];
        let candidates = vec![pair(&cs[0], &cs[1])];
        let score = evaluate_pair_sets(frame(), workload(3), &cs, 3, &oracle, &candidates).unwrap();
        assert_eq!(score.counts.false_negative_pairs, 1);
        assert_eq!(score.counts.candidate_recall_basis_points(), Some(5_000));
        assert!(!score.benchmark_qualified());
    }

    #[test]
    fn empty_oracle_population_has_unavailable_recall() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0)];
        let score = evaluate_pair_sets(frame(), workload(2), &cs, 1, &[], &[]).unwrap();
        assert_eq!(score.counts.candidate_recall_basis_points(), None);
        assert!(!score.benchmark_qualified());
    }

    #[test]
    fn rejects_duplicates_self_pairs_and_unknown_endpoints() {
        let a = collider(0, 1, 0);
        let b = collider(1, 1, 0);
        let duplicate_collider = vec![a.clone(), b.clone(), a.clone()];
        let p = pair(&a, &b);
        assert_eq!(
            evaluate_pair_sets(frame(), workload(3), &duplicate_collider, 3, &[], &[p.clone(), p]),
            Err(vec![PairSetError::DuplicateColliderIdentity, PairSetError::DuplicateCandidatePair])
        );
        let outside = pair(&a, &collider(99, 1, 0));
        assert_eq!(
            evaluate_pair_sets(frame(), workload(2), &[a, b], 1, &[], &[outside]),
            Err(vec![PairSetError::PairReferencesUnknownCollider])
        );
    }

    #[test]
    fn rejects_invalid_receipt_workload_and_pair_denominator() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0)];
        let bad_frame = FrameReceipt::new(
            1, 2, "bad", "bad", "", "", OracleCoverage::Complete,
        );
        assert_eq!(
            evaluate_pair_sets(bad_frame, workload(2), &cs, 1, &[], &[]),
            Err(vec![PairSetError::InvalidFrameReceipt])
        );
        let bad_workload = WorkloadFrame {
            collider_count: 2,
            static_colliders: 2,
            awake_dynamic_colliders: 1,
            ..WorkloadFrame::default()
        };
        assert_eq!(
            evaluate_pair_sets(frame(), bad_workload, &cs, 1, &[], &[]),
            Err(vec![PairSetError::InvalidWorkloadClassification])
        );
        assert_eq!(
            evaluate_pair_sets(frame(), workload(2), &cs, 2, &[], &[]),
            Err(vec![PairSetError::EligiblePairCountExceedsPopulation])
        );
    }

    #[test]
    fn partial_or_unknown_oracle_coverage_cannot_qualify() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0)];
        for coverage in [OracleCoverage::Partial, OracleCoverage::Unknown] {
            let receipt = FrameReceipt::new(
                42,
                7,
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "brute-force-aabb-overlap",
                "v1",
                coverage,
            );
            assert_eq!(
                evaluate_pair_sets(receipt, workload(2), &cs, 1, &[], &[]),
                Err(vec![PairSetError::OracleCoverageIncomplete])
            );
        }
    }

    #[test]
    fn workload_population_must_match_frozen_collider_universe() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0)];
        assert_eq!(
            evaluate_pair_sets(frame(), workload(3), &cs, 1, &[], &[]),
            Err(vec![PairSetError::WorkloadPopulationMismatch])
        );
    }

    #[test]
    fn collision_filter_population_is_not_the_unfiltered_combinatorial_count() {
        let cs = vec![collider(0, 1, 0), collider(1, 1, 0), collider(2, 1, 0), collider(3, 1, 0)];
        let p = pair(&cs[0], &cs[1]);
        let score = evaluate_pair_sets(frame(), workload(4), &cs, 2, &[p.clone()], &[p]).unwrap();
        assert_eq!(score.counts.possible_unfiltered_pairs, 6);
        assert_eq!(score.counts.eligible_pairs_after_filter, 2);
        assert_eq!(score.counts.candidate_false_positive_rate_basis_points(), Some(0));
        assert_eq!(score.counts.reduction_basis_points(), Some(5_000));
    }

    #[test]
    fn summarizes_p50_p95_p99_and_cumulative_workload_counters() {
        let frames = [10_u64, 20, 30, 40].map(|full_step| WorkloadFrame {
            collider_count: 2,
            static_colliders: 1,
            awake_dynamic_colliders: 1,
            moved_colliders: 1,
            update_ns: full_step / 10,
            rebuild_ns: full_step / 10,
            pair_generation_ns: full_step / 2,
            full_step_ns: full_step,
            node_visits: 2,
            leaf_visits: 3,
            reinserts: 1,
            pair_cache_additions: 4,
            pair_cache_removals: 2,
            peak_memory_bytes: full_step * 100,
            ..WorkloadFrame::default()
        });
        let summary = summarize_workload_frames(&frames).unwrap();

        assert_eq!(summary.frame_count, 4);
        assert_eq!(summary.p50_full_step_ns, Some(20));
        assert_eq!(summary.p95_full_step_ns, Some(40));
        assert_eq!(summary.p99_full_step_ns, Some(40));
        assert_eq!(summary.p50_broadphase_ns, Some(14));
        assert_eq!(summary.p95_broadphase_ns, Some(28));
        assert_eq!(summary.p99_broadphase_ns, Some(28));
        assert_eq!(summary.total_node_visits, 8);
        assert_eq!(summary.total_leaf_visits, 12);
        assert_eq!(summary.total_reinserts, 4);
        assert_eq!(summary.total_pair_cache_additions, 16);
        assert_eq!(summary.total_pair_cache_removals, 8);
        assert_eq!(summary.peak_memory_bytes, 4_000);
    }

    #[test]
    fn rejects_empty_or_overflowing_workload_traces() {
        assert_eq!(
            summarize_workload_frames(&[]),
            Err(PairSetError::EmptyWorkloadSeries)
        );
        let overflow = [
            WorkloadFrame {
                collider_count: 0,
                update_ns: u64::MAX,
                rebuild_ns: 1,
                ..WorkloadFrame::default()
            },
        ];
        assert_eq!(
            summarize_workload_frames(&overflow),
            Err(PairSetError::WorkloadCostOverflow)
        );
    }

    #[test]
    fn workload_cost_components_are_checked() {
        let invalid = WorkloadFrame {
            collider_count: 1,
            static_colliders: 1,
            update_ns: u64::MAX,
            rebuild_ns: 1,
            ..WorkloadFrame::default()
        };
        assert_eq!(invalid.broadphase_cost_ns(), None);
        assert_eq!(invalid.validate(), Err(PairSetError::WorkloadCostOverflow));
    }
}
