//! Reproducible temporal broadphase architecture tournament.
//!
//! The independent brute-force oracle is prepared before timed backend runs.
//! Timed regions cover each backend's frame update/query/candidate production;
//! candidate comparison is performed after the timer stops.
//!
//! v0.1 is an AABB-level CPU architecture comparison. It does not measure
//! narrowphase, CCD, solver work, GPU execution, or external engine adapters.

use incremental_aabb_tree_reference::{
    Aabb, BalancePolicy, IncrementalAabbTree, MaintenanceStats, ProxyId, ProxyPair, TreeError,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

mod indexed_pair_cache;
use indexed_pair_cache::IndexedPairDeltaCache;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneFamily {
    SparseCoherent,
    DenseClustered,
    StaticHeavy,
    HighChurn,
    Anisotropic,
    TeleportChurn,
    SleepingWakeups,
}

impl SceneFamily {
    pub const ALL: [Self; 7] = [
        Self::SparseCoherent,
        Self::DenseClustered,
        Self::StaticHeavy,
        Self::HighChurn,
        Self::Anisotropic,
        Self::TeleportChurn,
        Self::SleepingWakeups,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SparseCoherent => "sparse_coherent",
            Self::DenseClustered => "dense_clustered",
            Self::StaticHeavy => "static_heavy",
            Self::HighChurn => "high_churn",
            Self::Anisotropic => "anisotropic",
            Self::TeleportChurn => "teleport_churn",
            Self::SleepingWakeups => "sleeping_wakeups",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProxyFrame<const D: usize> {
    pub id: ProxyId,
    pub bounds: Aabb<D>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalScene<const D: usize> {
    pub family: SceneFamily,
    pub seed: u64,
    pub declared_proxy_count: usize,
    pub frames: Vec<Vec<ProxyFrame<D>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendKind {
    BruteForce,
    SweepAndPrune,
    TreePerLeafUnbalanced,
    TreeSelfPairUnbalanced,
    TreePerLeafBalanced,
    TreeSelfPairBalanced,
    PairDeltaCache,
    IndexedPairDeltaCache,
    MortonLbvhAabbAdapter,
}

impl BackendKind {
    pub const ALL: [Self; 9] = [
        Self::BruteForce,
        Self::SweepAndPrune,
        Self::TreePerLeafUnbalanced,
        Self::TreeSelfPairUnbalanced,
        Self::TreePerLeafBalanced,
        Self::TreeSelfPairBalanced,
        Self::PairDeltaCache,
        Self::IndexedPairDeltaCache,
        Self::MortonLbvhAabbAdapter,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BruteForce => "brute_force",
            Self::SweepAndPrune => "sweep_and_prune",
            Self::TreePerLeafUnbalanced => "tree_per_leaf_unbalanced",
            Self::TreeSelfPairUnbalanced => "tree_self_pair_unbalanced",
            Self::TreePerLeafBalanced => "tree_per_leaf_balanced",
            Self::TreeSelfPairBalanced => "tree_self_pair_balanced",
            Self::PairDeltaCache => "pair_delta_cache",
            Self::IndexedPairDeltaCache => "indexed_pair_delta_cache",
            Self::MortonLbvhAabbAdapter => "symtropy_morton_lbvh_aabb_adapter",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TournamentConfig {
    pub repetitions: u32,
}

impl TournamentConfig {
    #[must_use]
    pub const fn new(repetitions: u32) -> Self {
        Self { repetitions }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkCounters {
    pub pair_tests: u64,
    pub raw_tree_pairs: u64,
    pub stale_leaf_hits_filtered: u64,
    pub node_visits: u64,
    pub leaf_tests: u64,
    pub aabb_tests: u64,
    pub inserts: u64,
    pub removes: u64,
    pub updates: u64,
    pub fat_contained_updates: u64,
    pub proxy_reinsertions: u64,
    pub full_rebuilds: u64,
    pub tree_rotations: u64,
    pub pair_cache_dirty_proxies: u64,
    pub pair_cache_overlap_tests: u64,
    pub pair_events_added: u64,
    pub pair_events_removed: u64,
    pub pair_revalidations: u64,
    pub pair_cache_peak_pairs: usize,
    pub frontier_proxy_visits: u64,
    pub frontier_pair_candidates: u64,
    pub frontier_unique_pairs: u64,
    pub frontier_node_visits: u64,
    pub frontier_leaf_tests: u64,
    pub frontier_aabb_tests: u64,
    pub frontier_index_mutations: u64,
    pub frontier_index_reinsertions: u64,
    pub lbvh_builds: u64,
    pub lbvh_internal_nodes: u64,
    pub lbvh_bruteforce_fallback_frames: u64,
    pub max_tree_height: usize,
    pub max_tree_nodes: usize,
    pub max_active_proxies: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TournamentRow {
    pub family: &'static str,
    pub dimension: usize,
    pub seed: u64,
    pub declared_proxy_count: usize,
    pub scene_frames: usize,
    pub backend: &'static str,
    /// Full-set backends time complete candidate production; the pair cache
    /// times only pair-delta production and reports full-set materialization separately.
    pub output_contract: &'static str,
    pub repetitions: u32,
    pub frame_samples: usize,
    pub proxy_observations: u64,
    pub oracle_pairs: u64,
    pub candidate_pairs: u64,
    pub false_positives: u64,
    pub false_negatives: u64,
    pub mismatch_frames: u64,
    pub event_mismatch_frames: u64,
    pub p50_frame_ns: u128,
    pub p95_frame_ns: u128,
    pub p99_frame_ns: u128,
    pub p50_pair_materialize_ns: u128,
    pub p95_pair_materialize_ns: u128,
    pub p99_pair_materialize_ns: u128,
    pub sum_frame_ns: u128,
    pub campaign_wall_ns: u128,
    pub counters: WorkCounters,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TournamentError {
    EmptyScene,
    EmptyFrame,
    InvalidRepetitions,
    DuplicateProxyId,
    InvalidBounds,
    Tree(TreeError),
    CounterOverflow,
    PairCacheInvariant,
    InvalidDirtyJournal,
    DirtyJournalCannotApplyFullInvalidation,
}

impl From<TreeError> for TournamentError {
    fn from(value: TreeError) -> Self {
        Self::Tree(value)
    }
}

fn next_unit(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 11) as f64) / ((1_u64 << 53) as f64)
}

fn next_between(state: &mut u64, min: f64, max: f64) -> f64 {
    min + (max - min) * next_unit(state)
}

fn integer_grid_coordinate(slot: usize, axis: usize) -> f64 {
    let mut divisor = 1_usize;
    for _ in 0..axis {
        divisor = divisor.saturating_mul(16);
    }
    ((slot / divisor) % 16) as f64 * 5.0
}

/// Generate a deterministic sequence. The same family, count, frame count and
/// seed always produce the same proxy population and AABBs.
pub fn generate_scene<const D: usize>(
    family: SceneFamily,
    proxy_count: usize,
    frame_count: usize,
    seed: u64,
) -> Result<TemporalScene<D>, TournamentError> {
    if D == 0 || proxy_count == 0 || frame_count == 0 {
        return Err(TournamentError::EmptyScene);
    }

    let mut state = seed;
    let mut initial = Vec::with_capacity(proxy_count);
    let mut velocities = Vec::with_capacity(proxy_count);
    let mut extents = Vec::with_capacity(proxy_count);

    for slot in 0..proxy_count {
        let mut center = [0.0; D];
        let mut velocity = [0.0; D];
        let mut extent = [0.0; D];

        for axis in 0..D {
            center[axis] = match family {
                SceneFamily::SparseCoherent | SceneFamily::StaticHeavy => {
                    integer_grid_coordinate(slot, axis) + next_between(&mut state, -0.1, 0.1)
                }
                SceneFamily::DenseClustered => next_between(&mut state, -4.0, 4.0),
                SceneFamily::HighChurn => next_between(&mut state, -30.0, 30.0),
                SceneFamily::Anisotropic => next_between(&mut state, -24.0, 24.0),
                SceneFamily::TeleportChurn => next_between(&mut state, -30.0, 30.0),
                SceneFamily::SleepingWakeups => {
                    let mut divisor = 1_usize;
                    for _ in 0..axis {
                        divisor = divisor.saturating_mul(4);
                    }
                    ((slot / divisor) % 4) as f64 * 0.85 + next_between(&mut state, -0.01, 0.01)
                }
            };

            extent[axis] = match family {
                SceneFamily::SparseCoherent => next_between(&mut state, 0.2, 0.45),
                SceneFamily::DenseClustered => next_between(&mut state, 0.55, 0.95),
                SceneFamily::StaticHeavy => next_between(&mut state, 0.2, 0.4),
                SceneFamily::HighChurn => next_between(&mut state, 0.3, 0.8),
                SceneFamily::Anisotropic => {
                    if axis == 0 {
                        0.025
                    } else if axis == 1 {
                        2.0
                    } else {
                        0.15
                    }
                }
                SceneFamily::TeleportChurn => next_between(&mut state, 0.25, 0.65),
                SceneFamily::SleepingWakeups => next_between(&mut state, 0.45, 0.55),
            };

            velocity[axis] = match family {
                SceneFamily::SparseCoherent => 0.01 * (axis as f64 + 1.0),
                SceneFamily::DenseClustered => next_between(&mut state, -0.05, 0.05),
                SceneFamily::StaticHeavy => {
                    if slot % 10 == 0 {
                        next_between(&mut state, -0.04, 0.04)
                    } else {
                        0.0
                    }
                }
                SceneFamily::HighChurn => next_between(&mut state, -0.4, 0.4),
                SceneFamily::Anisotropic => {
                    if axis == 0 {
                        0.1
                    } else {
                        0.002 * (axis as f64)
                    }
                }
                SceneFamily::TeleportChurn => next_between(&mut state, -0.02, 0.02),
                SceneFamily::SleepingWakeups => {
                    if slot % 64 == 0 {
                        0.025 * (axis as f64 + 1.0)
                    } else {
                        0.0
                    }
                }
            };
        }

        initial.push(center);
        velocities.push(velocity);
        extents.push(extent);
    }

    let mut frames = Vec::with_capacity(frame_count);
    for frame_index in 0..frame_count {
        let mut frame = Vec::with_capacity(proxy_count);
        for slot in 0..proxy_count {
            if family == SceneFamily::HighChurn
                && slot != 0
                && ((slot + frame_index.saturating_mul(7)) % 13 < 2)
            {
                continue;
            }

            let mut center = initial[slot];
            for axis in 0..D {
                center[axis] += velocities[slot][axis] * frame_index as f64;
                if family == SceneFamily::TeleportChurn
                    && slot % 17 == 0
                    && frame_index > 0
                    && frame_index % 11 == 0
                {
                    center[axis] += if axis % 2 == 0 { 60.0 } else { -45.0 };
                }
            }

            let min = std::array::from_fn(|axis| center[axis] - extents[slot][axis]);
            let max = std::array::from_fn(|axis| center[axis] + extents[slot][axis]);
            let bounds = Aabb::new(min, max).map_err(|_| TournamentError::InvalidBounds)?;
            frame.push(ProxyFrame {
                id: ProxyId(slot as u64 + 1),
                bounds,
            });
        }
        frame.sort_unstable_by_key(|proxy| proxy.id);
        frames.push(frame);
    }

    Ok(TemporalScene {
        family,
        seed,
        declared_proxy_count: proxy_count,
        frames,
    })
}

fn changed_proxy_ids<const D: usize>(
    previous: &[ProxyFrame<D>],
    current: &[ProxyFrame<D>],
) -> BTreeSet<ProxyId> {
    let previous_bounds = previous
        .iter()
        .map(|proxy| (proxy.id, proxy.bounds))
        .collect::<BTreeMap<_, _>>();
    let current_bounds = current
        .iter()
        .map(|proxy| (proxy.id, proxy.bounds))
        .collect::<BTreeMap<_, _>>();
    let mut dirty = BTreeSet::new();
    for (id, bounds) in &previous_bounds {
        if current_bounds.get(id) != Some(bounds) {
            dirty.insert(*id);
        }
    }
    for id in current_bounds.keys() {
        if !previous_bounds.contains_key(id) {
            dirty.insert(*id);
        }
    }
    dirty
}

fn validate_proxy_sequence<const D: usize>(frame: &[ProxyFrame<D>]) -> Result<(), TournamentError> {
    let mut previous = None;
    for proxy in frame {
        if !proxy.bounds.is_valid() {
            return Err(TournamentError::InvalidBounds);
        }
        if previous.is_some_and(|id| id >= proxy.id) {
            return Err(TournamentError::DuplicateProxyId);
        }
        previous = Some(proxy.id);
    }
    Ok(())
}

fn validate_frame<const D: usize>(frame: &[ProxyFrame<D>]) -> Result<(), TournamentError> {
    if frame.is_empty() {
        return Err(TournamentError::EmptyFrame);
    }
    validate_proxy_sequence(frame)
}

fn overlaps<const D: usize>(left: &Aabb<D>, right: &Aabb<D>) -> bool {
    (0..D)
        .all(|axis| left.max()[axis] >= right.min()[axis] && right.max()[axis] >= left.min()[axis])
}

/// Independent all-pairs reference, computed outside every timed backend.
fn oracle_pairs<const D: usize>(
    frame: &[ProxyFrame<D>],
) -> Result<BTreeSet<ProxyPair>, TournamentError> {
    validate_frame(frame)?;
    let mut pairs = BTreeSet::new();
    for left_index in 0..frame.len() {
        for right_index in (left_index + 1)..frame.len() {
            if overlaps(&frame[left_index].bounds, &frame[right_index].bounds) {
                let pair = ProxyPair::new(frame[left_index].id, frame[right_index].id)
                    .ok_or(TournamentError::DuplicateProxyId)?;
                pairs.insert(pair);
            }
        }
    }
    Ok(pairs)
}

fn brute_force_candidates<const D: usize>(frame: &[ProxyFrame<D>]) -> Vec<ProxyPair> {
    let mut pairs = Vec::new();
    for left_index in 0..frame.len() {
        for right_index in (left_index + 1)..frame.len() {
            if overlaps(&frame[left_index].bounds, &frame[right_index].bounds) {
                pairs.push(
                    ProxyPair::new(frame[left_index].id, frame[right_index].id)
                        .expect("distinct frame IDs are validated"),
                );
            }
        }
    }
    pairs
}

/// Pair-relation and contact-refresh changes emitted by the cache for one frame.
///
/// The active pair relation persists in the cache. Additions and removals
/// change membership; revalidated pairs remain active but have a dirty endpoint
/// and must refresh narrowphase/contact state. Consumers do not need a
/// regenerated full pair list on every frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PairDeltaFrame {
    pub added: Vec<ProxyPair>,
    pub removed: Vec<ProxyPair>,
    pub revalidated: Vec<ProxyPair>,
    pub dirty_proxies: usize,
    pub overlap_tests: u64,
    pub active_pairs: usize,
    pub frontier_proxy_visits: u64,
    pub frontier_pair_candidates: u64,
    pub frontier_unique_pairs: u64,
    pub frontier_node_visits: u64,
    pub frontier_leaf_tests: u64,
    pub frontier_aabb_tests: u64,
    pub frontier_index_mutations: u64,
    pub frontier_index_reinsertions: u64,
}

/// Persistent symmetric adjacency cache over the exact current-frame AABB
/// overlap relation. Only pairs touching inserted, removed or changed proxies
/// are reconsidered. Unchanged pairs remain resident and generate no event.
#[derive(Clone, Debug)]
pub struct PairDeltaCache<const D: usize> {
    previous_bounds: BTreeMap<ProxyId, Aabb<D>>,
    neighbors: BTreeMap<ProxyId, BTreeSet<ProxyId>>,
    active_pair_count: usize,
    force_dirty: bool,
}

impl<const D: usize> Default for PairDeltaCache<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const D: usize> PairDeltaCache<D> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            previous_bounds: BTreeMap::new(),
            neighbors: BTreeMap::new(),
            active_pair_count: 0,
            force_dirty: false,
        }
    }

    /// Invalidate all pair relations on the next step. Call this when
    /// collision filters, layer rules or other non-geometric eligibility
    /// inputs change; this AABB-only benchmark has no filter mutation.
    pub fn invalidate_all(&mut self) {
        self.force_dirty = true;
    }

    #[must_use]
    pub fn active_pair_count(&self) -> usize {
        self.active_pair_count
    }

    /// Materialize the complete current pair relation. This is intentionally
    /// separate from step: production consumers should normally consume the
    /// pair event stream and persistent cache, not rebuild this vector.
    #[must_use]
    pub fn active_pairs(&self) -> Vec<ProxyPair> {
        let mut pairs = Vec::with_capacity(self.active_pair_count);
        for (left, neighbors) in &self.neighbors {
            for right in neighbors {
                if left < right {
                    if let Some(pair) = ProxyPair::new(*left, *right) {
                        pairs.push(pair);
                    }
                }
            }
        }
        pairs.sort_unstable();
        pairs
    }

    /// Derive the complete dirty identity set by diffing the prior AABB
    /// snapshot against the canonical current frame. This is the safe default
    /// when no authoritative physics-world change journal is available.
    pub fn derive_dirty_proxy_ids(
        &self,
        frame: &[ProxyFrame<D>],
    ) -> Result<Vec<ProxyId>, TournamentError> {
        validate_proxy_sequence(frame)?;
        let mut dirty = BTreeSet::<ProxyId>::new();
        let mut old_iter = self.previous_bounds.iter().peekable();
        let mut new_iter = frame.iter().peekable();
        loop {
            match (old_iter.peek(), new_iter.peek()) {
                (Some((old_id, old_bounds)), Some(new_proxy)) => {
                    match (**old_id).cmp(&new_proxy.id) {
                        std::cmp::Ordering::Less => {
                            dirty.insert(**old_id);
                            old_iter.next();
                        }
                        std::cmp::Ordering::Greater => {
                            dirty.insert(new_proxy.id);
                            new_iter.next();
                        }
                        std::cmp::Ordering::Equal => {
                            if **old_bounds != new_proxy.bounds {
                                dirty.insert(**old_id);
                            }
                            old_iter.next();
                            new_iter.next();
                        }
                    }
                }
                (Some((old_id, _)), None) => {
                    dirty.insert(**old_id);
                    old_iter.next();
                }
                (None, Some(new_proxy)) => {
                    dirty.insert(new_proxy.id);
                    new_iter.next();
                }
                (None, None) => break,
            }
        }
        if self.force_dirty {
            dirty.extend(self.previous_bounds.keys().copied());
            dirty.extend(frame.iter().map(|proxy| proxy.id));
        }
        Ok(dirty.into_iter().collect())
    }

    /// Independently verify that a caller-supplied change journal contains
    /// exactly the inserted, removed and changed AABB identities.
    ///
    /// This performs the full snapshot diff and belongs in the audit path,
    /// not the timed fast path.
    pub fn verify_dirty_proxy_ids(
        &self,
        frame: &[ProxyFrame<D>],
        supplied: &[ProxyId],
    ) -> Result<bool, TournamentError> {
        let expected = self.derive_dirty_proxy_ids(frame)?;
        Ok(expected == supplied)
    }

    /// Safe default: detect changed identities from the complete snapshots.
    pub fn step(&mut self, frame: &[ProxyFrame<D>]) -> Result<PairDeltaFrame, TournamentError> {
        let dirty_ids = self.derive_dirty_proxy_ids(frame)?;
        self.apply_dirty_ids(frame, &dirty_ids, true)
    }

    /// Apply a frame using a caller-provided, strictly increasing dirty-ID
    /// journal. This is the fast path for a physics world that already tracks
    /// inserted/removed/moved/woken colliders. Completeness is a caller
    /// precondition; use verify_dirty_proxy_ids in the independent audit.
    ///
    /// A full invalidation (e.g. collision filter/layer policy change) must use
    /// step, which derives all relevant identities from the snapshot.
    pub fn step_with_dirty_ids(
        &mut self,
        frame: &[ProxyFrame<D>],
        dirty_ids: &[ProxyId],
    ) -> Result<PairDeltaFrame, TournamentError> {
        self.apply_dirty_ids(frame, dirty_ids, false)
    }

    fn apply_dirty_ids(
        &mut self,
        frame: &[ProxyFrame<D>],
        dirty_ids: &[ProxyId],
        allow_full_invalidation: bool,
    ) -> Result<PairDeltaFrame, TournamentError> {
        validate_proxy_sequence(frame)?;
        if self.force_dirty && !allow_full_invalidation {
            return Err(TournamentError::DirtyJournalCannotApplyFullInvalidation);
        }
        let mut previous_dirty = None;
        for dirty_id in dirty_ids {
            if previous_dirty.is_some_and(|previous| previous >= *dirty_id) {
                return Err(TournamentError::InvalidDirtyJournal);
            }
            let exists_before = self.previous_bounds.contains_key(dirty_id);
            let exists_now = frame
                .binary_search_by_key(dirty_id, |proxy| proxy.id)
                .is_ok();
            if !exists_before && !exists_now {
                return Err(TournamentError::InvalidDirtyJournal);
            }
            previous_dirty = Some(*dirty_id);
        }
        let dirty = dirty_ids.iter().copied().collect::<BTreeSet<_>>();

        let mut affected_pairs = BTreeSet::<ProxyPair>::new();
        let mut frontier_proxy_visits = 0_u64;
        let mut frontier_pair_candidates = 0_u64;
        for dirty_id in &dirty {
            if let Some(old_neighbors) = self.neighbors.get(dirty_id) {
                for neighbor in old_neighbors {
                    frontier_pair_candidates = frontier_pair_candidates
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                    if let Some(pair) = ProxyPair::new(*dirty_id, *neighbor) {
                        affected_pairs.insert(pair);
                    }
                }
            }
            if frame
                .binary_search_by_key(dirty_id, |proxy| proxy.id)
                .is_ok()
            {
                for other in frame {
                    frontier_proxy_visits = frontier_proxy_visits
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                    if other.id != *dirty_id {
                        frontier_pair_candidates = frontier_pair_candidates
                            .checked_add(1)
                            .ok_or(TournamentError::CounterOverflow)?;
                    }
                    if let Some(pair) = ProxyPair::new(*dirty_id, other.id) {
                        affected_pairs.insert(pair);
                    }
                }
            }
        }

        // Build and validate the complete transition set before mutating the
        // cache. This prevents a late inconsistency or overflow from leaving a
        // partially applied frame.
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut revalidated = Vec::new();
        let mut overlap_tests = 0_u64;

        for pair in affected_pairs {
            let left_active = self
                .neighbors
                .get(&pair.0)
                .is_some_and(|neighbors| neighbors.contains(&pair.1));
            let right_active = self
                .neighbors
                .get(&pair.1)
                .is_some_and(|neighbors| neighbors.contains(&pair.0));
            if left_active != right_active {
                return Err(TournamentError::PairCacheInvariant);
            }
            let was_active = left_active;

            let left = frame
                .binary_search_by_key(&pair.0, |proxy| proxy.id)
                .ok()
                .map(|index| &frame[index].bounds);
            let right = frame
                .binary_search_by_key(&pair.1, |proxy| proxy.id)
                .ok()
                .map(|index| &frame[index].bounds);
            let is_active = match (left, right) {
                (Some(left), Some(right)) => {
                    overlap_tests = overlap_tests
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                    overlaps(left, right)
                }
                _ => false,
            };

            match (was_active, is_active) {
                (false, true) => added.push(pair),
                (true, false) => removed.push(pair),
                (true, true) => revalidated.push(pair),
                (false, false) => {}
            }
        }

        let next_pair_count = self
            .active_pair_count
            .checked_sub(removed.len())
            .and_then(|count| count.checked_add(added.len()))
            .ok_or(TournamentError::CounterOverflow)?;

        // All possible validation and arithmetic failures are above this
        // point. Apply the transitions symmetrically.
        for pair in &removed {
            self.neighbors
                .get_mut(&pair.0)
                .expect("validated pair endpoint exists")
                .remove(&pair.1);
            self.neighbors
                .get_mut(&pair.1)
                .expect("validated pair endpoint exists")
                .remove(&pair.0);
        }
        for pair in &added {
            self.neighbors.entry(pair.0).or_default().insert(pair.1);
            self.neighbors.entry(pair.1).or_default().insert(pair.0);
        }

        // Retain tombstone-free active adjacency, then update bounds only for
        // dirty identities. Unchanged identities incur no ordered-map writes.
        for dirty_id in &dirty {
            match frame.binary_search_by_key(dirty_id, |proxy| proxy.id) {
                Ok(index) => {
                    self.previous_bounds.insert(*dirty_id, frame[index].bounds);
                }
                Err(_) => {
                    self.previous_bounds.remove(dirty_id);
                    self.neighbors.remove(dirty_id);
                }
            }
        }

        self.active_pair_count = next_pair_count;
        self.force_dirty = false;

        Ok(PairDeltaFrame {
            added,
            removed,
            revalidated,
            dirty_proxies: dirty.len(),
            overlap_tests,
            active_pairs: self.active_pair_count,
            frontier_proxy_visits,
            frontier_pair_candidates,
            frontier_unique_pairs: u64::try_from(affected_pairs.len())
                .map_err(|_| TournamentError::CounterOverflow)?,
            frontier_node_visits: 0,
            frontier_leaf_tests: 0,
            frontier_aabb_tests: 0,
            frontier_index_mutations: 0,
            frontier_index_reinsertions: 0,
        })
    }
}

fn choose_sweep_axis<const D: usize>(frame: &[ProxyFrame<D>]) -> usize {
    let mut best_axis = 0;
    let mut best_score = f64::NEG_INFINITY;

    for axis in 0..D {
        let mut min_center = f64::INFINITY;
        let mut max_center = f64::NEG_INFINITY;
        let mut mean_extent = 0.0;
        for proxy in frame {
            let min = proxy.bounds.min()[axis];
            let max = proxy.bounds.max()[axis];
            let center = min + (max - min) * 0.5;
            min_center = min_center.min(center);
            max_center = max_center.max(center);
            mean_extent += max - min;
        }
        mean_extent /= frame.len() as f64;
        let score = (max_center - min_center) / mean_extent.max(1e-12);
        if score > best_score {
            best_axis = axis;
            best_score = score;
        }
    }
    best_axis
}

/// Sweep-and-prune with a current-frame-only axis heuristic. No future frame
/// or held-out sequence information participates in axis selection.
fn sweep_and_prune<const D: usize>(frame: &[ProxyFrame<D>]) -> (Vec<ProxyPair>, usize, u64) {
    let axis = choose_sweep_axis(frame);
    let mut ordered: Vec<_> = frame.iter().collect();
    ordered.sort_by(|left, right| {
        left.bounds.min()[axis]
            .total_cmp(&right.bounds.min()[axis])
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut pairs = Vec::new();
    let mut pair_tests = 0_u64;
    for left_index in 0..ordered.len() {
        for right_index in (left_index + 1)..ordered.len() {
            if ordered[right_index].bounds.min()[axis] > ordered[left_index].bounds.max()[axis] {
                break;
            }
            pair_tests = pair_tests.saturating_add(1);
            if overlaps(&ordered[left_index].bounds, &ordered[right_index].bounds) {
                pairs.push(
                    ProxyPair::new(ordered[left_index].id, ordered[right_index].id)
                        .expect("distinct frame IDs are validated"),
                );
            }
        }
    }
    pairs.sort_unstable();
    (pairs, axis, pair_tests)
}

const MORTON_LBVH_BRUTE_FORCE_THRESHOLD: usize = 50;
const MORTON_LBVH_LEAF_BIT: u32 = 0x8000_0000;

#[derive(Clone, Copy, Debug)]
struct MortonLbvhNode<const D: usize> {
    bounds: Aabb<D>,
    left: u32,
    right: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct MortonLbvhWork {
    pair_tests: u64,
    internal_nodes: usize,
    node_visits: u64,
    leaf_tests: u64,
    aabb_tests: u64,
    brute_force_fallback: bool,
}

fn morton_quantize(value: f64, world_min: f64, world_max: f64, bits: usize) -> u32 {
    let range = world_max - world_min;
    if range <= 0.0 {
        return 0;
    }
    let max_value = if bits >= 64 {
        u64::MAX as f64
    } else {
        ((1_u64 << bits) - 1) as f64
    };
    let normalized = ((value - world_min) / range).clamp(0.0, 1.0);
    (normalized * max_value) as u32
}

fn morton_encode<const D: usize>(
    position: &[f64; D],
    world_min: &[f64; D],
    world_max: &[f64; D],
) -> u64 {
    let bits = 64 / D;
    let mut code = 0_u64;
    for bit in 0..bits {
        for axis in 0..D {
            let quantized = morton_quantize(position[axis], world_min[axis], world_max[axis], bits);
            code |= (u64::from((quantized >> bit) & 1)) << (bit * D + axis);
        }
    }
    code
}

fn morton_find_split(codes: &[u64], first: usize, last: usize) -> usize {
    if codes[first] == codes[last] {
        return (first + last) / 2;
    }
    let common_prefix = (codes[first] ^ codes[last]).leading_zeros();
    let mut split = first;
    let mut step = last - first;
    loop {
        step = step.div_ceil(2);
        let next_split = split + step;
        if next_split < last && (codes[first] ^ codes[next_split]).leading_zeros() > common_prefix {
            split = next_split;
        }
        if step <= 1 {
            break;
        }
    }
    split
}

fn morton_is_leaf(index: u32) -> bool {
    index & MORTON_LBVH_LEAF_BIT != 0
}

fn morton_leaf_index(index: u32) -> usize {
    (index & !MORTON_LBVH_LEAF_BIT) as usize
}

fn morton_build_recursive<const D: usize>(
    codes: &[u64],
    sorted_bounds: &[Aabb<D>],
    first: usize,
    last: usize,
    nodes: &mut Vec<MortonLbvhNode<D>>,
) -> u32 {
    if first == last {
        return MORTON_LBVH_LEAF_BIT | (first as u32);
    }
    let split = morton_find_split(codes, first, last);
    let left = morton_build_recursive(codes, sorted_bounds, first, split, nodes);
    let right = morton_build_recursive(codes, sorted_bounds, split + 1, last, nodes);

    let left_bounds = if morton_is_leaf(left) {
        sorted_bounds[morton_leaf_index(left)]
    } else {
        nodes[left as usize].bounds
    };
    let right_bounds = if morton_is_leaf(right) {
        sorted_bounds[morton_leaf_index(right)]
    } else {
        nodes[right as usize].bounds
    };
    let index = nodes.len() as u32;
    nodes.push(MortonLbvhNode {
        bounds: left_bounds.union(&right_bounds),
        left,
        right,
    });
    index
}

/// AABB adaptation of Symtropy's live Morton/LBVH broadphase algorithm.
///
/// It follows the production Morton quantization, sorted-index layout,
/// common-prefix split construction, per-leaf traversal, final sort/dedup and
/// the existing <50-body brute-force fallback. Body-type and collision-filter
/// checks are omitted because the frozen tournament AABBs are already the
/// eligible proxy population. This is therefore an algorithmic AABB adapter,
/// not an end-to-end RigidBody/shape/static-cache integration.
fn symtropy_morton_lbvh_aabb<const D: usize>(
    frame: &[ProxyFrame<D>],
) -> (Vec<ProxyPair>, MortonLbvhWork) {
    if frame.len() < MORTON_LBVH_BRUTE_FORCE_THRESHOLD {
        let mut pairs = Vec::new();
        let mut work = MortonLbvhWork {
            brute_force_fallback: true,
            ..MortonLbvhWork::default()
        };
        for left_index in 0..frame.len() {
            for right_index in (left_index + 1)..frame.len() {
                work.pair_tests = work.pair_tests.saturating_add(1);
                if overlaps(&frame[left_index].bounds, &frame[right_index].bounds) {
                    pairs.push(
                        ProxyPair::new(frame[left_index].id, frame[right_index].id)
                            .expect("validated frame IDs are distinct"),
                    );
                }
            }
        }
        return (pairs, work);
    }

    let mut world_min = [f64::INFINITY; D];
    let mut world_max = [f64::NEG_INFINITY; D];
    for proxy in frame {
        for axis in 0..D {
            world_min[axis] = world_min[axis].min(proxy.bounds.min()[axis]);
            world_max[axis] = world_max[axis].max(proxy.bounds.max()[axis]);
        }
    }
    for axis in 0..D {
        if world_max[axis] - world_min[axis] < 1e-6 {
            world_min[axis] -= 1.0;
            world_max[axis] += 1.0;
        }
    }

    let bits = 64 / D;
    let morton_codes = frame
        .iter()
        .map(|proxy| {
            let center = std::array::from_fn(|axis| {
                (proxy.bounds.min()[axis] + proxy.bounds.max()[axis]) * 0.5
            });
            morton_encode(&center, &world_min, &world_max)
        })
        .collect::<Vec<_>>();

    // Stable index sort mirrors the production LBVH layout, including equal
    // Morton-code degeneracies.
    let mut sorted_indices = (0..frame.len()).collect::<Vec<_>>();
    sorted_indices.sort_by_key(|index| morton_codes[*index]);
    let sorted_codes = sorted_indices
        .iter()
        .map(|index| morton_codes[*index])
        .collect::<Vec<_>>();
    let sorted_bounds = sorted_indices
        .iter()
        .map(|index| frame[*index].bounds)
        .collect::<Vec<_>>();
    let mut nodes = Vec::<MortonLbvhNode<D>>::with_capacity(frame.len() - 1);
    let root = morton_build_recursive(
        &sorted_codes,
        &sorted_bounds,
        0,
        sorted_codes.len() - 1,
        &mut nodes,
    );

    let mut pairs = Vec::new();
    let mut work = MortonLbvhWork {
        internal_nodes: nodes.len(),
        ..MortonLbvhWork::default()
    };
    for leaf_index in 0..sorted_bounds.len() {
        let leaf_bounds = &sorted_bounds[leaf_index];
        let mut stack = vec![root];
        while let Some(node_index) = stack.pop() {
            work.node_visits = work.node_visits.saturating_add(1);
            if morton_is_leaf(node_index) {
                let other_index = morton_leaf_index(node_index);
                if other_index > leaf_index {
                    work.leaf_tests = work.leaf_tests.saturating_add(1);
                    work.aabb_tests = work.aabb_tests.saturating_add(1);
                    if leaf_bounds.overlaps(&sorted_bounds[other_index]) {
                        work.pair_tests = work.pair_tests.saturating_add(1);
                        let left = frame[sorted_indices[leaf_index]].id;
                        let right = frame[sorted_indices[other_index]].id;
                        pairs.push(
                            ProxyPair::new(left, right).expect("validated frame IDs are distinct"),
                        );
                    }
                }
                continue;
            }

            let node = &nodes[node_index as usize];
            work.aabb_tests = work.aabb_tests.saturating_add(1);
            if leaf_bounds.overlaps(&node.bounds) {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    let _bits_per_axis = bits;
    (pairs, work)
}

fn compare_candidates(actual: &[ProxyPair], expected: &BTreeSet<ProxyPair>) -> (u64, u64) {
    let actual_set: BTreeSet<_> = actual.iter().copied().collect();
    let false_positives = actual_set.difference(expected).count() as u64;
    let false_negatives = expected.difference(&actual_set).count() as u64;
    (false_positives, false_negatives)
}

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (percentile * sorted.len()).div_ceil(100).max(1);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn checked_add(target: &mut u64, value: usize) -> Result<(), TournamentError> {
    let value = u64::try_from(value).map_err(|_| TournamentError::CounterOverflow)?;
    *target = target
        .checked_add(value)
        .ok_or(TournamentError::CounterOverflow)?;
    Ok(())
}

fn maintenance_delta(previous: MaintenanceStats, current: MaintenanceStats) -> MaintenanceStats {
    MaintenanceStats {
        insert_operations: current
            .insert_operations
            .saturating_sub(previous.insert_operations),
        remove_operations: current
            .remove_operations
            .saturating_sub(previous.remove_operations),
        update_operations: current
            .update_operations
            .saturating_sub(previous.update_operations),
        fat_contained_updates: current
            .fat_contained_updates
            .saturating_sub(previous.fat_contained_updates),
        proxy_reinsertions: current
            .proxy_reinsertions
            .saturating_sub(previous.proxy_reinsertions),
        full_rebuilds: current.full_rebuilds.saturating_sub(previous.full_rebuilds),
        tree_rotations: current
            .tree_rotations
            .saturating_sub(previous.tree_rotations),
    }
}

fn add_maintenance(counters: &mut WorkCounters, delta: MaintenanceStats) {
    counters.inserts = counters.inserts.saturating_add(delta.insert_operations);
    counters.removes = counters.removes.saturating_add(delta.remove_operations);
    counters.updates = counters.updates.saturating_add(delta.update_operations);
    counters.fat_contained_updates = counters
        .fat_contained_updates
        .saturating_add(delta.fat_contained_updates);
    counters.proxy_reinsertions = counters
        .proxy_reinsertions
        .saturating_add(delta.proxy_reinsertions);
    counters.full_rebuilds = counters.full_rebuilds.saturating_add(delta.full_rebuilds);
    counters.tree_rotations = counters.tree_rotations.saturating_add(delta.tree_rotations);
}

fn run_backend<const D: usize>(
    scene: &TemporalScene<D>,
    oracle: &[BTreeSet<ProxyPair>],
    backend: BackendKind,
    repetitions: u32,
) -> Result<TournamentRow, TournamentError> {
    let campaign_start = Instant::now();
    let mut frame_samples = Vec::<u128>::new();
    let mut counters = WorkCounters::default();
    let mut proxy_observations = 0_u64;
    let mut oracle_pair_count = 0_u64;
    let mut candidate_pair_count = 0_u64;
    let mut false_positives = 0_u64;
    let mut false_negatives = 0_u64;
    let mut mismatch_frames = 0_u64;
    let mut event_mismatch_frames = 0_u64;
    let mut pair_materialize_samples = Vec::<u128>::new();

    for _ in 0..repetitions {
        let mut tree = match backend {
            BackendKind::TreePerLeafUnbalanced | BackendKind::TreeSelfPairUnbalanced => Some(
                IncrementalAabbTree::<D>::new_with_balance_policy(0.0, BalancePolicy::Disabled)?,
            ),
            BackendKind::TreePerLeafBalanced | BackendKind::TreeSelfPairBalanced => {
                Some(IncrementalAabbTree::<D>::new_with_balance_policy(
                    0.0,
                    BalancePolicy::HeightRotations,
                )?)
            }
            BackendKind::BruteForce
            | BackendKind::SweepAndPrune
            | BackendKind::PairDeltaCache
            | BackendKind::IndexedPairDeltaCache
            | BackendKind::MortonLbvhAabbAdapter => None,
        };
        let mut pair_cache =
            (backend == BackendKind::PairDeltaCache).then(PairDeltaCache::<D>::new);
        let mut indexed_pair_cache = if backend == BackendKind::IndexedPairDeltaCache {
            Some(IndexedPairDeltaCache::<D>::new(0.5, BalancePolicy::HeightRotations)?)
        } else {
            None
        };
        let mut previous_bounds = BTreeMap::<ProxyId, Aabb<D>>::new();
        let mut previous_maintenance = MaintenanceStats::default();

        for (frame_index, frame) in scene.frames.iter().enumerate() {
            let start = Instant::now();
            let mut actual = Vec::<ProxyPair>::new();
            let mut pair_delta = None;

            match backend {
                BackendKind::BruteForce => {
                    counters.pair_tests = counters.pair_tests.saturating_add(
                        (frame.len().saturating_mul(frame.len().saturating_sub(1)) / 2) as u64,
                    );
                    actual = brute_force_candidates(frame);
                }
                BackendKind::SweepAndPrune => {
                    let (pairs, _axis, pair_tests) = sweep_and_prune(frame);
                    counters.pair_tests = counters.pair_tests.saturating_add(pair_tests);
                    actual = pairs;
                }
                BackendKind::TreePerLeafUnbalanced
                | BackendKind::TreeSelfPairUnbalanced
                | BackendKind::TreePerLeafBalanced
                | BackendKind::TreeSelfPairBalanced => {
                    let tree = tree.as_mut().expect("tree backends initialize a tree");
                    let current_bounds = frame
                        .iter()
                        .map(|proxy| (proxy.id, proxy.bounds))
                        .collect::<BTreeMap<_, _>>();

                    let removals = previous_bounds
                        .keys()
                        .filter(|id| !current_bounds.contains_key(id))
                        .copied()
                        .collect::<Vec<_>>();
                    for id in removals {
                        tree.remove(id)?;
                        previous_bounds.remove(&id);
                    }

                    for proxy in frame {
                        match previous_bounds.get(&proxy.id) {
                            None => {
                                tree.insert(proxy.id, proxy.bounds)?;
                            }
                            Some(previous) if *previous == proxy.bounds => {}
                            Some(_) => {
                                tree.update(proxy.id, proxy.bounds)?;
                            }
                        }
                        previous_bounds.insert(proxy.id, proxy.bounds);
                    }

                    let query = match backend {
                        BackendKind::TreePerLeafUnbalanced | BackendKind::TreePerLeafBalanced => {
                            tree.query_pairs_per_leaf_reference()
                        }
                        BackendKind::TreeSelfPairUnbalanced | BackendKind::TreeSelfPairBalanced => {
                            tree.query_pairs()
                        }
                        BackendKind::BruteForce
                        | BackendKind::SweepAndPrune
                        | BackendKind::PairDeltaCache
                        | BackendKind::IndexedPairDeltaCache
                        | BackendKind::MortonLbvhAabbAdapter => {
                            unreachable!("non-tree backend entered tree query arm")
                        }
                    };
                    checked_add(&mut counters.raw_tree_pairs, query.pairs.len())?;
                    checked_add(&mut counters.node_visits, query.stats.node_visits)?;
                    checked_add(&mut counters.leaf_tests, query.stats.leaf_tests)?;
                    checked_add(&mut counters.aabb_tests, query.stats.aabb_tests)?;

                    for pair in query.pairs {
                        let left = current_bounds
                            .get(&pair.0)
                            .ok_or(TournamentError::DuplicateProxyId)?;
                        let right = current_bounds
                            .get(&pair.1)
                            .ok_or(TournamentError::DuplicateProxyId)?;
                        // Tree leaves may remain larger after a fat-contained
                        // update. Filter those conservative stale hits against
                        // the current frame's authoritative AABBs.
                        if overlaps(left, right) {
                            actual.push(pair);
                        } else {
                            counters.stale_leaf_hits_filtered =
                                counters.stale_leaf_hits_filtered.saturating_add(1);
                        }
                    }
                    actual.sort_unstable();

                    let stats = tree.stats();
                    add_maintenance(
                        &mut counters,
                        maintenance_delta(previous_maintenance, stats.maintenance),
                    );
                    previous_maintenance = stats.maintenance;
                    counters.max_tree_height = counters.max_tree_height.max(stats.height);
                    counters.max_tree_nodes = counters.max_tree_nodes.max(stats.node_count);
                }
                BackendKind::PairDeltaCache => {
                    let cache = pair_cache
                        .as_mut()
                        .expect("pair-delta backend initializes a pair cache");
                    pair_delta = Some(cache.step(frame)?);
                }
                BackendKind::IndexedPairDeltaCache => {
                    let cache = indexed_pair_cache
                        .as_mut()
                        .expect("indexed pair-delta backend initializes a cache");
                    pair_delta = Some(cache.step(frame)?);
                }
                BackendKind::MortonLbvhAabbAdapter => {
                    let (pairs, work) = symtropy_morton_lbvh_aabb(frame);
                    actual = pairs;
                    counters.pair_tests = counters
                        .pair_tests
                        .checked_add(work.pair_tests)
                        .ok_or(TournamentError::CounterOverflow)?;
                    counters.node_visits = counters
                        .node_visits
                        .checked_add(work.node_visits)
                        .ok_or(TournamentError::CounterOverflow)?;
                    counters.leaf_tests = counters
                        .leaf_tests
                        .checked_add(work.leaf_tests)
                        .ok_or(TournamentError::CounterOverflow)?;
                    counters.aabb_tests = counters
                        .aabb_tests
                        .checked_add(work.aabb_tests)
                        .ok_or(TournamentError::CounterOverflow)?;
                    if work.brute_force_fallback {
                        counters.lbvh_bruteforce_fallback_frames = counters
                            .lbvh_bruteforce_fallback_frames
                            .checked_add(1)
                            .ok_or(TournamentError::CounterOverflow)?;
                    } else {
                        counters.lbvh_builds = counters
                            .lbvh_builds
                            .checked_add(1)
                            .ok_or(TournamentError::CounterOverflow)?;
                    }
                    counters.lbvh_internal_nodes = counters
                        .lbvh_internal_nodes
                        .checked_add(
                            u64::try_from(work.internal_nodes)
                                .map_err(|_| TournamentError::CounterOverflow)?,
                        )
                        .ok_or(TournamentError::CounterOverflow)?;
                    counters.max_tree_nodes = counters.max_tree_nodes.max(work.internal_nodes);
                }
            }

            let elapsed = start.elapsed().as_nanos();
            frame_samples.push(elapsed);

            // Pair-cache materialization and independent checks are outside
            // its timed pair-delta update interval.
            if let Some(delta) = pair_delta {
                checked_add(&mut counters.pair_cache_dirty_proxies, delta.dirty_proxies)?;
                counters.pair_cache_overlap_tests = counters
                    .pair_cache_overlap_tests
                    .checked_add(delta.overlap_tests)
                    .ok_or(TournamentError::CounterOverflow)?;
                checked_add(&mut counters.pair_events_added, delta.added.len())?;
                checked_add(&mut counters.pair_events_removed, delta.removed.len())?;
                checked_add(&mut counters.pair_revalidations, delta.revalidated.len())?;
                counters.frontier_proxy_visits = counters.frontier_proxy_visits
                    .checked_add(delta.frontier_proxy_visits)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_pair_candidates = counters.frontier_pair_candidates
                    .checked_add(delta.frontier_pair_candidates)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_unique_pairs = counters.frontier_unique_pairs
                    .checked_add(delta.frontier_unique_pairs)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_node_visits = counters.frontier_node_visits
                    .checked_add(delta.frontier_node_visits)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_leaf_tests = counters.frontier_leaf_tests
                    .checked_add(delta.frontier_leaf_tests)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_aabb_tests = counters.frontier_aabb_tests
                    .checked_add(delta.frontier_aabb_tests)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_index_mutations = counters.frontier_index_mutations
                    .checked_add(delta.frontier_index_mutations)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.frontier_index_reinsertions = counters.frontier_index_reinsertions
                    .checked_add(delta.frontier_index_reinsertions)
                    .ok_or(TournamentError::CounterOverflow)?;
                counters.pair_cache_peak_pairs =
                    counters.pair_cache_peak_pairs.max(delta.active_pairs);

                let expected_current = &oracle[frame_index];
                let empty_previous = BTreeSet::new();
                let expected_previous = if frame_index == 0 {
                    &empty_previous
                } else {
                    &oracle[frame_index - 1]
                };
                let expected_added: BTreeSet<_> = expected_current
                    .difference(expected_previous)
                    .copied()
                    .collect();
                let expected_removed: BTreeSet<_> = expected_previous
                    .difference(expected_current)
                    .copied()
                    .collect();
                let previous_frame = if frame_index == 0 {
                    &[][..]
                } else {
                    scene.frames[frame_index - 1].as_slice()
                };
                let dirty_ids = changed_proxy_ids(previous_frame, frame);
                let expected_revalidated: BTreeSet<_> = expected_current
                    .intersection(expected_previous)
                    .filter(|pair| dirty_ids.contains(&pair.0) || dirty_ids.contains(&pair.1))
                    .copied()
                    .collect();
                let actual_added: BTreeSet<_> = delta.added.iter().copied().collect();
                let actual_removed: BTreeSet<_> = delta.removed.iter().copied().collect();
                let actual_revalidated: BTreeSet<_> =
                    delta.revalidated.iter().copied().collect();
                if actual_added != expected_added
                    || actual_removed != expected_removed
                    || actual_revalidated != expected_revalidated
                    || actual_added.len() != delta.added.len()
                    || actual_removed.len() != delta.removed.len()
                    || actual_revalidated.len() != delta.revalidated.len()
                {
                    event_mismatch_frames = event_mismatch_frames.saturating_add(1);
                }

                let materialize_start = Instant::now();
                actual = if let Some(cache) = pair_cache.as_ref() {
                    cache.active_pairs()
                } else {
                    indexed_pair_cache
                        .as_ref()
                        .expect("indexed pair cache remains initialized")
                        .active_pairs()
                };
                pair_materialize_samples.push(materialize_start.elapsed().as_nanos());
            }

            // Correctness comparison is deliberately outside the timed region.
            let expected = &oracle[frame_index];
            let (frame_fp, frame_fn) = compare_candidates(&actual, expected);
            false_positives = false_positives.saturating_add(frame_fp);
            false_negatives = false_negatives.saturating_add(frame_fn);
            if frame_fp > 0 || frame_fn > 0 {
                mismatch_frames = mismatch_frames.saturating_add(1);
            }

            checked_add(&mut proxy_observations, frame.len())?;
            checked_add(&mut oracle_pair_count, expected.len())?;
            checked_add(&mut candidate_pair_count, actual.len())?;
            counters.max_active_proxies = counters.max_active_proxies.max(frame.len());
        }
    }

    frame_samples.sort_unstable();
    let sum_frame_ns = frame_samples.iter().copied().sum();
    Ok(TournamentRow {
        family: scene.family.name(),
        dimension: D,
        seed: scene.seed,
        declared_proxy_count: scene.declared_proxy_count,
        scene_frames: scene.frames.len(),
        backend: backend.name(),
        output_contract: if matches!(
            backend,
            BackendKind::PairDeltaCache | BackendKind::IndexedPairDeltaCache
        ) {
            "pair_delta_events"
        } else {
            "full_candidate_set"
        },
        repetitions,
        frame_samples: frame_samples.len(),
        proxy_observations,
        oracle_pairs: oracle_pair_count,
        candidate_pairs: candidate_pair_count,
        false_positives,
        false_negatives,
        mismatch_frames,
        event_mismatch_frames,
        p50_frame_ns: percentile(&frame_samples, 50),
        p95_frame_ns: percentile(&frame_samples, 95),
        p99_frame_ns: percentile(&frame_samples, 99),
        p50_pair_materialize_ns: percentile(&pair_materialize_samples, 50),
        p95_pair_materialize_ns: percentile(&pair_materialize_samples, 95),
        p99_pair_materialize_ns: percentile(&pair_materialize_samples, 99),
        sum_frame_ns,
        campaign_wall_ns: campaign_start.elapsed().as_nanos(),
        counters,
    })
}

/// Run all four backend families on exactly the same frozen frame sequence.
/// The brute-force oracle is prepared once outside the timed backend runs.
pub fn run_tournament<const D: usize>(
    scene: &TemporalScene<D>,
    config: TournamentConfig,
) -> Result<Vec<TournamentRow>, TournamentError> {
    if config.repetitions == 0 {
        return Err(TournamentError::InvalidRepetitions);
    }
    if scene.frames.is_empty() {
        return Err(TournamentError::EmptyScene);
    }

    let oracle = scene
        .frames
        .iter()
        .map(|frame| oracle_pairs(frame))
        .collect::<Result<Vec<_>, _>>()?;

    BackendKind::ALL
        .into_iter()
        .map(|backend| run_backend(scene, &oracle, backend, config.repetitions))
        .collect()
}

impl TournamentRow {
    #[must_use]
    pub fn csv_header() -> &'static str {
        "family,dimension,seed,declared_proxy_count,scene_frames,backend,output_contract,repetitions,frame_samples,proxy_observations,oracle_pairs,candidate_pairs,false_positives,false_negatives,mismatch_frames,event_mismatch_frames,p50_frame_ns,p95_frame_ns,p99_frame_ns,p50_pair_materialize_ns,p95_pair_materialize_ns,p99_pair_materialize_ns,sum_frame_ns,campaign_wall_ns,pair_tests,raw_tree_pairs,stale_leaf_hits_filtered,node_visits,leaf_tests,aabb_tests,inserts,removes,updates,fat_contained_updates,proxy_reinsertions,full_rebuilds,tree_rotations,pair_cache_dirty_proxies,pair_cache_overlap_tests,pair_events_added,pair_events_removed,pair_cache_peak_pairs,lbvh_builds,lbvh_internal_nodes,lbvh_bruteforce_fallback_frames,max_tree_height,max_tree_nodes,max_active_proxies,pair_revalidations,frontier_proxy_visits,frontier_pair_candidates,frontier_unique_pairs,frontier_node_visits,frontier_leaf_tests,frontier_aabb_tests,frontier_index_mutations,frontier_index_reinsertions"
    }

    #[must_use]
    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}{},{},{},{},{},{},{},{},"
            self.family,
            self.dimension,
            self.seed,
            self.declared_proxy_count,
            self.scene_frames,
            self.backend,
            self.output_contract,
            self.repetitions,
            self.frame_samples,
            self.proxy_observations,
            self.oracle_pairs,
            self.candidate_pairs,
            self.false_positives,
            self.false_negatives,
            self.mismatch_frames,
            self.event_mismatch_frames,
            self.p50_frame_ns,
            self.p95_frame_ns,
            self.p99_frame_ns,
            self.p50_pair_materialize_ns,
            self.p95_pair_materialize_ns,
            self.p99_pair_materialize_ns,
            self.sum_frame_ns,
            self.campaign_wall_ns,
            self.counters.pair_tests,
            self.counters.raw_tree_pairs,
            self.counters.stale_leaf_hits_filtered,
            self.counters.node_visits,
            self.counters.leaf_tests,
            self.counters.aabb_tests,
            self.counters.inserts,
            self.counters.removes,
            self.counters.updates,
            self.counters.fat_contained_updates,
            self.counters.proxy_reinsertions,
            self.counters.full_rebuilds,
            self.counters.tree_rotations,
            self.counters.pair_cache_dirty_proxies,
            self.counters.pair_cache_overlap_tests,
            self.counters.pair_events_added,
            self.counters.pair_events_removed,
            self.counters.pair_cache_peak_pairs,
            self.counters.lbvh_builds,
            self.counters.lbvh_internal_nodes,
            self.counters.lbvh_bruteforce_fallback_frames,
            self.counters.max_tree_height,
            self.counters.max_tree_nodes,
            self.counters.max_active_proxies,
            self.counters.pair_revalidations,
            self.counters.frontier_proxy_visits,
            self.counters.frontier_pair_candidates,
            self.counters.frontier_unique_pairs,
            self.counters.frontier_node_visits,
            self.counters.frontier_leaf_tests,
            self.counters.frontier_aabb_tests,
            self.counters.frontier_index_mutations,
            self.counters.frontier_index_reinsertions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_all_backends_match<const D: usize>(family: SceneFamily) {
        let scene = generate_scene::<D>(family, 48, 18, 0xace0_55ed_1122_3344).unwrap();
        let rows = run_tournament(&scene, TournamentConfig::new(1)).unwrap();
        assert_eq!(rows.len(), BackendKind::ALL.len());
        for row in rows {
            assert_eq!(row.false_positives, 0, "{family:?} backend {}", row.backend);
            assert_eq!(row.false_negatives, 0, "{family:?} backend {}", row.backend);
            assert_eq!(row.mismatch_frames, 0, "{family:?} backend {}", row.backend);
            assert_eq!(
                row.event_mismatch_frames, 0,
                "{family:?} backend {}",
                row.backend
            );
            assert_eq!(row.oracle_pairs, row.candidate_pairs);
            assert!(row.frame_samples > 0);
        }
    }

    fn test_proxy(id: u64, min: [f64; 2], max: [f64; 2]) -> ProxyFrame<2> {
        ProxyFrame {
            id: ProxyId(id),
            bounds: Aabb::new(min, max).unwrap(),
        }
    }

    #[test]
    fn indexed_pair_delta_cache_matches_membership_and_revalidation_oracles() {
        assert_indexed_pair_delta_matches_oracle::<2>();
        assert_indexed_pair_delta_matches_oracle::<3>();
        assert_indexed_pair_delta_matches_oracle::<4>();
    }

    fn assert_indexed_pair_delta_matches_oracle<const D: usize>() {
        let scene =
            generate_scene::<D>(SceneFamily::SleepingWakeups, 128, 32, 0x1ade_2026).unwrap();
        let oracle = scene
            .frames
            .iter()
            .map(|frame| oracle_pairs(frame).unwrap())
            .collect::<Vec<_>>();
        let mut cache =
            IndexedPairDeltaCache::<D>::new(0.5, BalancePolicy::HeightRotations).unwrap();

        for (frame_index, frame) in scene.frames.iter().enumerate() {
            let delta = cache.step(frame).unwrap();
            let previous = if frame_index == 0 {
                BTreeSet::new()
            } else {
                oracle[frame_index - 1].clone()
            };
            let expected_current = &oracle[frame_index];
            let expected_added: BTreeSet<_> =
                expected_current.difference(&previous).copied().collect();
            let expected_removed: BTreeSet<_> =
                previous.difference(expected_current).copied().collect();
            let previous_frame = if frame_index == 0 {
                &[][..]
            } else {
                scene.frames[frame_index - 1].as_slice()
            };
            let dirty_ids = changed_proxy_ids(previous_frame, frame);
            let expected_revalidated: BTreeSet<_> = expected_current
                .intersection(&previous)
                .filter(|pair| dirty_ids.contains(&pair.0) || dirty_ids.contains(&pair.1))
                .copied()
                .collect();

            assert_eq!(
                delta.added.iter().copied().collect::<BTreeSet<_>>(),
                expected_added
            );
            assert_eq!(
                delta.removed.iter().copied().collect::<BTreeSet<_>>(),
                expected_removed
            );
            assert_eq!(
                delta.revalidated.iter().copied().collect::<BTreeSet<_>>(),
                expected_revalidated
            );
            assert_eq!(
                cache.active_pairs().into_iter().collect::<BTreeSet<_>>(),
                *expected_current
            );
            assert_eq!(delta.frontier_proxy_visits, 0);
            if frame_index > 0 {
                assert!(delta.frontier_node_visits > 0 || delta.frontier_leaf_tests > 0);
            }
        }
    }

    #[test]
    fn indexed_pair_delta_cache_audits_dirty_journal_and_rejects_full_invalidation_fast_path() {
        let mut cache =
            IndexedPairDeltaCache::<2>::new(0.25, BalancePolicy::HeightRotations).unwrap();
        let first = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [0.5, 0.5], [1.5, 1.5]),
            test_proxy(3, [5.0, 5.0], [6.0, 6.0]),
        ];
        cache.step(&first).unwrap();

        let next = [
            test_proxy(1, [2.0, 0.0], [3.0, 1.0]),
            test_proxy(2, [2.5, 0.5], [3.5, 1.5]),
            test_proxy(3, [5.0, 5.0], [6.0, 6.0]),
        ];
        let dirty = cache.derive_dirty_proxy_ids(&next).unwrap();
        assert_eq!(dirty, vec![ProxyId(1), ProxyId(2)]);
        assert!(cache.verify_dirty_proxy_ids(&next, &dirty).unwrap());
        assert!(!cache.verify_dirty_proxy_ids(&next, &[ProxyId(1)]).unwrap());
        let moved = cache.step_with_dirty_ids(&next, &dirty).unwrap();
        assert_eq!(moved.revalidated.len(), 1);
        assert_eq!(moved.frontier_proxy_visits, 0);

        cache.invalidate_all();
        let full_dirty = cache.derive_dirty_proxy_ids(&next).unwrap();
        assert_eq!(
            cache.step_with_dirty_ids(&next, &full_dirty),
            Err(TournamentError::DirtyJournalCannotApplyFullInvalidation)
        );
        let rebuilt = cache.step(&next).unwrap();
        assert_eq!(rebuilt.dirty_proxies, 3);
        assert_eq!(rebuilt.active_pairs, 1);
    }

    #[test]
    fn pair_delta_cache_emits_only_pair_births_and_deaths() {
        let mut cache = PairDeltaCache::<2>::new();
        let first = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [0.5, 0.5], [1.5, 1.5]),
            test_proxy(3, [5.0, 5.0], [6.0, 6.0]),
        ];
        let initial = cache.step(&first).unwrap();
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).unwrap();
        assert_eq!(initial.added, vec![pair]);
        assert!(initial.removed.is_empty());
        assert_eq!(initial.active_pairs, 1);
        assert_eq!(cache.active_pairs(), vec![pair]);

        let unchanged = cache.step(&first).unwrap();
        assert!(unchanged.added.is_empty());
        assert!(unchanged.removed.is_empty());
        assert!(unchanged.revalidated.is_empty());
        assert_eq!(unchanged.dirty_proxies, 0);
        assert_eq!(unchanged.overlap_tests, 0);
        assert_eq!(unchanged.active_pairs, 1);

        let separated = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [3.0, 3.0], [4.0, 4.0]),
            test_proxy(3, [5.0, 5.0], [6.0, 6.0]),
        ];
        let removal = cache.step(&separated).unwrap();
        assert!(removal.added.is_empty());
        assert_eq!(removal.removed, vec![pair]);
        assert!(removal.revalidated.is_empty());
        assert_eq!(removal.active_pairs, 0);
        assert!(cache.active_pairs().is_empty());
    }

    #[test]
    fn pair_delta_cache_removes_last_proxy_and_accepts_empty_frames() {
        let mut cache = PairDeltaCache::<2>::new();
        let initial = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [0.5, 0.5], [1.5, 1.5]),
        ];
        assert_eq!(cache.step(&initial).unwrap().added.len(), 1);

        let dirty = cache.derive_dirty_proxy_ids(&[]).unwrap();
        assert_eq!(dirty, vec![ProxyId(1), ProxyId(2)]);
        assert!(cache.verify_dirty_proxy_ids(&[], &dirty).unwrap());
        let removed = cache.step(&[]).unwrap();
        assert_eq!(removed.removed.len(), 1);
        assert_eq!(removed.active_pairs, 0);
        assert!(cache.active_pairs().is_empty());

        let reinitialized = [test_proxy(3, [10.0, 10.0], [11.0, 11.0])];
        let fresh = cache.step(&reinitialized).unwrap();
        assert!(fresh.added.is_empty());
        assert!(fresh.removed.is_empty());
        assert_eq!(fresh.dirty_proxies, 1);
        assert_eq!(fresh.active_pairs, 0);
    }

    #[test]
    fn pair_delta_cache_preserves_pairs_when_both_endpoints_move_together() {
        let mut cache = PairDeltaCache::<2>::new();
        let first = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [0.5, 0.0], [1.5, 1.0]),
        ];
        let initial = cache.step(&first).unwrap();
        assert_eq!(initial.added.len(), 1);

        let moved_together = [
            test_proxy(1, [5.0, 0.0], [6.0, 1.0]),
            test_proxy(2, [5.5, 0.0], [6.5, 1.0]),
        ];
        let moved = cache.step(&moved_together).unwrap();
        assert_eq!(moved.dirty_proxies, 2);
        assert_eq!(moved.overlap_tests, 1);
        assert!(moved.added.is_empty());
        assert!(moved.removed.is_empty());
        assert_eq!(moved.revalidated, vec![ProxyPair::new(ProxyId(1), ProxyId(2)).unwrap()]);
        assert_eq!(moved.active_pairs, 1);
    }

    #[test]
    fn pair_delta_cache_handles_insert_remove_and_explicit_invalidation() {
        let mut cache = PairDeltaCache::<2>::new();
        let first = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [0.5, 0.5], [1.5, 1.5]),
        ];
        let initial = cache.step(&first).unwrap();
        assert_eq!(initial.added.len(), 1);

        let inserted = [
            first[0],
            first[1],
            test_proxy(3, [0.75, 0.75], [1.25, 1.25]),
        ];
        let add = cache.step(&inserted).unwrap();
        assert_eq!(add.added.len(), 2);
        assert!(add.removed.is_empty());
        assert!(add.revalidated.is_empty());
        assert_eq!(cache.active_pair_count(), 3);

        let removed = [inserted[0], inserted[2]];
        let remove = cache.step(&removed).unwrap();
        assert_eq!(remove.removed.len(), 2);
        assert!(remove.revalidated.is_empty());
        assert_eq!(
            cache.active_pairs(),
            vec![ProxyPair::new(ProxyId(1), ProxyId(3)).unwrap()]
        );

        cache.invalidate_all();
        let revalidate = cache.step(&removed).unwrap();
        assert_eq!(revalidate.dirty_proxies, 2);
        assert_eq!(revalidate.overlap_tests, 1);
        assert!(revalidate.added.is_empty());
        assert!(revalidate.removed.is_empty());
        assert_eq!(
            revalidate.revalidated,
            vec![ProxyPair::new(ProxyId(1), ProxyId(2)).unwrap()]
        );
    }

    #[test]
    fn pair_delta_cache_accepts_verified_dirty_journals_and_rejects_invalid_ordering() {
        let mut cache = PairDeltaCache::<2>::new();
        let first = [
            test_proxy(1, [0.0, 0.0], [1.0, 1.0]),
            test_proxy(2, [0.5, 0.5], [1.5, 1.5]),
            test_proxy(3, [5.0, 5.0], [6.0, 6.0]),
        ];
        cache.step(&first).unwrap();

        let next = [
            test_proxy(2, [0.5, 0.5], [1.5, 1.5]),
            test_proxy(3, [0.8, 0.8], [1.8, 1.8]),
        ];
        let dirty = cache.derive_dirty_proxy_ids(&next).unwrap();
        assert_eq!(dirty, vec![ProxyId(1), ProxyId(3)]);
        assert!(cache.verify_dirty_proxy_ids(&next, &dirty).unwrap());
        assert!(!cache.verify_dirty_proxy_ids(&next, &[ProxyId(1)]).unwrap());

        assert_eq!(
            cache.step_with_dirty_ids(&next, &[ProxyId(3), ProxyId(1)]),
            Err(TournamentError::InvalidDirtyJournal)
        );
        assert_eq!(
            cache.step_with_dirty_ids(&next, &[ProxyId(1), ProxyId(1)]),
            Err(TournamentError::InvalidDirtyJournal)
        );
        assert_eq!(cache.active_pair_count(), 1);

        let delta = cache.step_with_dirty_ids(&next, &dirty).unwrap();
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.added.len(), 1);
        assert_eq!(
            cache.active_pairs(),
            vec![ProxyPair::new(ProxyId(2), ProxyId(3)).unwrap()]
        );

        cache.invalidate_all();
        let full_dirty = cache.derive_dirty_proxy_ids(&next).unwrap();
        assert_eq!(
            cache.step_with_dirty_ids(&next, &full_dirty),
            Err(TournamentError::DirtyJournalCannotApplyFullInvalidation)
        );
        assert_eq!(cache.step(&next).unwrap().overlap_tests, 1);
    }

    #[test]
    fn pair_delta_cache_matches_oracle_transitions_for_sleeping_wakeup_sequences() {
        for dimension in [2_usize, 3, 4] {
            match dimension {
                2 => assert_pair_delta_matches_oracle::<2>(),
                3 => assert_pair_delta_matches_oracle::<3>(),
                4 => assert_pair_delta_matches_oracle::<4>(),
                _ => unreachable!(),
            }
        }
    }

    fn assert_pair_delta_matches_oracle<const D: usize>() {
        let scene =
            generate_scene::<D>(SceneFamily::SleepingWakeups, 128, 32, 0x5eed_1126).unwrap();
        let oracle = scene
            .frames
            .iter()
            .map(|frame| oracle_pairs(frame).unwrap())
            .collect::<Vec<_>>();
        let mut cache = PairDeltaCache::<D>::new();

        for (frame_index, frame) in scene.frames.iter().enumerate() {
            let delta = cache.step(frame).unwrap();
            let previous = if frame_index == 0 {
                BTreeSet::new()
            } else {
                oracle[frame_index - 1].clone()
            };
            let expected_added: BTreeSet<_> =
                oracle[frame_index].difference(&previous).copied().collect();
            let expected_removed: BTreeSet<_> =
                previous.difference(&oracle[frame_index]).copied().collect();
            let previous_frame = if frame_index == 0 {
                &[][..]
            } else {
                scene.frames[frame_index - 1].as_slice()
            };
            let dirty_ids = changed_proxy_ids(previous_frame, frame);
            let expected_revalidated: BTreeSet<_> = oracle[frame_index]
                .intersection(&previous)
                .filter(|pair| dirty_ids.contains(&pair.0) || dirty_ids.contains(&pair.1))
                .copied()
                .collect();
            assert_eq!(
                delta.added.iter().copied().collect::<BTreeSet<_>>(),
                expected_added
            );
            assert_eq!(
                delta.removed.iter().copied().collect::<BTreeSet<_>>(),
                expected_removed
            );
            assert_eq!(
                delta.revalidated.iter().copied().collect::<BTreeSet<_>>(),
                expected_revalidated
            );
            assert_eq!(
                cache.active_pairs().into_iter().collect::<BTreeSet<_>>(),
                oracle[frame_index]
            );
            if frame_index > 0 {
                assert!(delta.dirty_proxies <= 2);
                assert!(delta.overlap_tests <= 2 * 128);
            }
        }
    }

    fn assert_morton_lbvh_hierarchy_matches<const D: usize>(family: SceneFamily) {
        let scene = generate_scene::<D>(family, 64, 12, 0x6c62_7668_2026).unwrap();
        let rows = run_tournament(&scene, TournamentConfig::new(1)).unwrap();
        let row = rows
            .iter()
            .find(|row| row.backend == BackendKind::MortonLbvhAabbAdapter.name())
            .unwrap();
        assert_eq!(row.false_positives, 0);
        assert_eq!(row.false_negatives, 0);
        assert_eq!(row.mismatch_frames, 0);
        assert_eq!(row.counters.lbvh_builds, 12);
        assert_eq!(row.counters.lbvh_bruteforce_fallback_frames, 0);
        assert_eq!(row.counters.lbvh_internal_nodes, 12 * (64 - 1));
    }

    #[test]
    fn symtropy_morton_lbvh_hierarchy_matches_oracle_in_2d_3d_4d() {
        assert_morton_lbvh_hierarchy::<2>(SceneFamily::SparseCoherent);
        assert_morton_lbvh_hierarchy::<3>(SceneFamily::DenseClustered);
        assert_morton_lbvh_hierarchy::<4>(SceneFamily::Anisotropic);
    }

    #[test]
    fn symtropy_morton_lbvh_handles_identical_morton_codes() {
        let bounds = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]).unwrap();
        let frame = (0..64)
            .map(|index| ProxyFrame {
                id: ProxyId(index + 1),
                bounds,
            })
            .collect::<Vec<_>>();
        let expected = oracle_pairs(&frame).unwrap();
        let (actual, work) = symtropy_morton_lbvh_aabb(&frame);
        let actual_set = actual.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(actual_set, expected);
        assert_eq!(work.internal_nodes, 63);
        assert!(!work.brute_force_fallback);
    }

    #[test]
    fn symtropy_morton_lbvh_uses_live_small_population_fallback_threshold() {
        let scene = generate_scene::<3>(SceneFamily::SparseCoherent, 48, 9, 0x50).unwrap();
        let rows = run_tournament(&scene, TournamentConfig::new(1)).unwrap();
        let row = rows
            .iter()
            .find(|row| row.backend == BackendKind::MortonLbvhAabbAdapter.name())
            .unwrap();
        assert_eq!(row.false_positives, 0);
        assert_eq!(row.false_negatives, 0);
        assert_eq!(row.counters.lbvh_builds, 0);
        assert_eq!(row.counters.lbvh_bruteforce_fallback_frames, 9);
        assert_eq!(row.counters.pair_tests, 9 * 48 * 47 / 2);
    }

    #[test]
    fn csv_header_and_rows_have_identical_width_and_retain_scene_identity() {
        let scene = generate_scene::<3>(SceneFamily::HighChurn, 32, 7, 0x1234_5678).unwrap();
        let row = run_tournament(&scene, TournamentConfig::new(1))
            .unwrap()
            .remove(0);
        assert_eq!(
            TournamentRow::csv_header().split(',').count(),
            row.to_csv().split(',').count()
        );
        assert_eq!(row.seed, scene.seed);
        assert_eq!(row.declared_proxy_count, scene.declared_proxy_count);
        assert_eq!(row.scene_frames, scene.frames.len());
    }

    #[test]
    fn deterministic_scene_generation_replays_identically() {
        let first = generate_scene::<3>(SceneFamily::HighChurn, 64, 20, 7).unwrap();
        let second = generate_scene::<3>(SceneFamily::HighChurn, 64, 20, 7).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn every_backend_matches_oracle_for_sparse_dense_and_churn_scenes() {
        assert_all_backends_match::<2>(SceneFamily::SparseCoherent);
        assert_all_backends_match::<3>(SceneFamily::DenseClustered);
        assert_all_backends_match::<4>(SceneFamily::HighChurn);
    }

    #[test]
    fn every_backend_matches_anisotropic_and_teleport_sequences() {
        assert_all_backends_match::<3>(SceneFamily::Anisotropic);
        assert_all_backends_match::<4>(SceneFamily::TeleportChurn);
    }

    #[test]
    fn static_heavy_sequence_preserves_candidate_population() {
        assert_all_backends_match::<3>(SceneFamily::StaticHeavy);
        assert_all_backends_match::<2>(SceneFamily::SleepingWakeups);
        assert_all_backends_match::<3>(SceneFamily::SleepingWakeups);
        assert_all_backends_match::<4>(SceneFamily::SleepingWakeups);
    }

    #[test]
    fn sap_axis_selection_uses_only_current_frame_geometry() {
        let first = generate_scene::<3>(SceneFamily::SparseCoherent, 64, 8, 42).unwrap();
        let scene2 = generate_scene::<3>(SceneFamily::SparseCoherent, 64, 8, 43).unwrap();
        let axis_a = choose_sweep_axis(&first.frames[0]);
        let axis_b = choose_sweep_axis(&scene2.frames[0]);
        assert!(axis_a < 3);
        assert!(axis_b < 3);
    }
}
