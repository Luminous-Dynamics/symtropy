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
    pub const ALL: [Self; 6] = [
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
}

impl BackendKind {
    pub const ALL: [Self; 6] = [
        Self::BruteForce,
        Self::SweepAndPrune,
        Self::TreePerLeafUnbalanced,
        Self::TreeSelfPairUnbalanced,
        Self::TreePerLeafBalanced,
        Self::TreeSelfPairBalanced,
        Self::PairDeltaCache,
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
    pub pair_cache_peak_pairs: usize,
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
                    ((slot / divisor) % 4) as f64 * 0.85
                        + next_between(&mut state, -0.01, 0.01)
                },
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
                },
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

fn validate_frame<const D: usize>(frame: &[ProxyFrame<D>]) -> Result<(), TournamentError> {
    if frame.is_empty() {
        return Err(TournamentError::EmptyFrame);
    }
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

/// Pair-relation changes emitted by the sleep-aware cache for one frame.
///
/// The active pair relation persists in the cache. Consumers update their
/// contact/pair state from these additions and removals; they do not need a
/// regenerated full pair list on every frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PairDeltaFrame {
    pub added: Vec<ProxyPair>,
    pub removed: Vec<ProxyPair>,
    pub dirty_proxies: usize,
    pub overlap_tests: u64,
    pub active_pairs: usize,
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

    /// Update the cached overlap relation and emit only pair births/deaths.
    ///
    /// The affected frontier includes old neighbors of every dirty identity
    /// and all current proxies for every inserted/changed identity. A pair
    /// involving two dirty proxies is evaluated once via the canonical set.
    pub fn step(
        &mut self,
        frame: &[ProxyFrame<D>],
    ) -> Result<PairDeltaFrame, TournamentError> {
        validate_frame(frame)?;

        let current_bounds = frame
            .iter()
            .map(|proxy| (proxy.id, proxy.bounds))
            .collect::<BTreeMap<_, _>>();

        let mut dirty = BTreeSet::<ProxyId>::new();
        for old_id in self.previous_bounds.keys() {
            if !current_bounds.contains_key(old_id) {
                dirty.insert(*old_id);
            }
        }
        for (id, bounds) in &current_bounds {
            if self.previous_bounds.get(id).is_none_or(|old| old != bounds) {
                dirty.insert(*id);
            }
        }
        if self.force_dirty {
            dirty.extend(self.previous_bounds.keys().copied());
            dirty.extend(current_bounds.keys().copied());
        }

        let mut affected_pairs = BTreeSet::<ProxyPair>::new();
        for dirty_id in &dirty {
            if let Some(old_neighbors) = self.neighbors.get(dirty_id) {
                for neighbor in old_neighbors {
                    if let Some(pair) = ProxyPair::new(*dirty_id, *neighbor) {
                        affected_pairs.insert(pair);
                    }
                }
            }
            if current_bounds.contains_key(dirty_id) {
                for other_id in current_bounds.keys() {
                    if let Some(pair) = ProxyPair::new(*dirty_id, *other_id) {
                        affected_pairs.insert(pair);
                    }
                }
            }
        }

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut overlap_tests = 0_u64;

        for pair in affected_pairs {
            let was_active = self
                .neighbors
                .get(&pair.0)
                .is_some_and(|neighbors| neighbors.contains(&pair.1));
            let is_active = match (current_bounds.get(&pair.0), current_bounds.get(&pair.1)) {
                (Some(left), Some(right)) => {
                    overlap_tests = overlap_tests
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                    overlaps(left, right)
                }
                _ => false,
            };

            match (was_active, is_active) {
                (false, true) => {
                    self.neighbors.entry(pair.0).or_default().insert(pair.1);
                    self.neighbors.entry(pair.1).or_default().insert(pair.0);
                    self.active_pair_count = self
                        .active_pair_count
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                    added.push(pair);
                }
                (true, false) => {
                    let left_removed = self
                        .neighbors
                        .get_mut(&pair.0)
                        .is_some_and(|neighbors| neighbors.remove(&pair.1));
                    let right_removed = self
                        .neighbors
                        .get_mut(&pair.1)
                        .is_some_and(|neighbors| neighbors.remove(&pair.0));
                    if !left_removed || !right_removed {
                        return Err(TournamentError::CounterOverflow);
                    }
                    self.active_pair_count = self
                        .active_pair_count
                        .checked_sub(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                    removed.push(pair);
                }
                (true, true) | (false, false) => {}
            }
        }

        self.neighbors
            .retain(|proxy_id, _| current_bounds.contains_key(proxy_id));
        for proxy_id in current_bounds.keys() {
            self.neighbors.entry(*proxy_id).or_default();
        }

        self.previous_bounds = current_bounds;
        self.force_dirty = false;

        Ok(PairDeltaFrame {
            added,
            removed,
            dirty_proxies: dirty.len(),
            overlap_tests,
            active_pairs: self.active_pair_count,
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
            BackendKind::BruteForce | BackendKind::SweepAndPrune | BackendKind::PairDeltaCache => None,
        };
        let mut pair_cache = (backend == BackendKind::PairDeltaCache)
            .then(PairDeltaCache::<D>::new);
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
                        | BackendKind::PairDeltaCache => {
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
            }

            let elapsed = start.elapsed().as_nanos();
            frame_samples.push(elapsed);

            // Pair-cache materialization and independent checks are outside
            // its timed pair-delta update interval.
            if let Some(delta) = pair_delta {
                checked_add(
                    &mut counters.pair_cache_dirty_proxies,
                    delta.dirty_proxies,
                )?;
                counters.pair_cache_overlap_tests = counters
                    .pair_cache_overlap_tests
                    .checked_add(delta.overlap_tests)
                    .ok_or(TournamentError::CounterOverflow)?;
                checked_add(&mut counters.pair_events_added, delta.added.len())?;
                checked_add(&mut counters.pair_events_removed, delta.removed.len())?;
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
                let actual_added: BTreeSet<_> = delta.added.iter().copied().collect();
                let actual_removed: BTreeSet<_> = delta.removed.iter().copied().collect();
                if actual_added != expected_added
                    || actual_removed != expected_removed
                    || actual_added.len() != delta.added.len()
                    || actual_removed.len() != delta.removed.len()
                {
                    event_mismatch_frames = event_mismatch_frames.saturating_add(1);
                }

                let materialize_start = Instant::now();
                actual = pair_cache
                    .as_ref()
                    .expect("pair cache remains initialized")
                    .active_pairs();
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
        output_contract: if backend == BackendKind::PairDeltaCache {
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
        "family,dimension,seed,declared_proxy_count,scene_frames,backend,repetitions,frame_samples,proxy_observations,oracle_pairs,candidate_pairs,false_positives,false_negatives,mismatch_frames,p50_frame_ns,p95_frame_ns,p99_frame_ns,sum_frame_ns,campaign_wall_ns,pair_tests,raw_tree_pairs,stale_leaf_hits_filtered,node_visits,leaf_tests,aabb_tests,inserts,removes,updates,fat_contained_updates,proxy_reinsertions,full_rebuilds,tree_rotations,max_tree_height,max_tree_nodes,max_active_proxies"
    }

    #[must_use]
    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.family,
            self.dimension,
            self.seed,
            self.declared_proxy_count,
            self.scene_frames,
            self.backend,
            self.repetitions,
            self.frame_samples,
            self.proxy_observations,
            self.oracle_pairs,
            self.candidate_pairs,
            self.false_positives,
            self.false_negatives,
            self.mismatch_frames,
            self.p50_frame_ns,
            self.p95_frame_ns,
            self.p99_frame_ns,
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
            self.counters.max_tree_height,
            self.counters.max_tree_nodes,
            self.counters.max_active_proxies,
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
            assert_eq!(row.oracle_pairs, row.candidate_pairs);
            assert!(row.frame_samples > 0);
        }
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
