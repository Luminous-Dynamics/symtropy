//! Indexed dirty-frontier pair reconciliation.
//!
//! This is a research adapter over the persistent incremental AABB tree.
//! It uses cached old adjacency for pair deaths and a spatial query for the
//! current frontier. The current AABB is always rechecked before a pair event
//! is committed; fat-tree hits are conservative candidates, not final pairs.

use super::{overlaps, validate_proxy_sequence, PairDeltaFrame, ProxyFrame, TournamentError};
use incremental_aabb_tree_reference::{BalancePolicy, IncrementalAabbTree, ProxyId, ProxyPair};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct IndexedPairDeltaCache<const D: usize> {
    previous_bounds: BTreeMap<ProxyId, super::Aabb<D>>,
    neighbors: BTreeMap<ProxyId, BTreeSet<ProxyId>>,
    active_pair_count: usize,
    tree: IncrementalAabbTree<D>,
    force_dirty: bool,
}

impl<const D: usize> IndexedPairDeltaCache<D> {
    pub(super) fn new(
        fat_margin: f64,
        balance_policy: BalancePolicy,
    ) -> Result<Self, TournamentError> {
        Ok(Self {
            previous_bounds: BTreeMap::new(),
            neighbors: BTreeMap::new(),
            active_pair_count: 0,
            tree: IncrementalAabbTree::new_with_balance_policy(fat_margin, balance_policy)?,
            force_dirty: false,
        })
    }

    pub(super) fn invalidate_all(&mut self) {
        self.force_dirty = true;
    }

    pub(super) fn active_pairs(&self) -> Vec<ProxyPair> {
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
        pairs
    }

    pub(super) fn derive_dirty_proxy_ids(
        &self,
        frame: &[ProxyFrame<D>],
    ) -> Result<Vec<ProxyId>, TournamentError> {
        validate_proxy_sequence(frame)?;
        let mut dirty = BTreeSet::new();
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

    /// Require complete geometric coverage while allowing additional known
    /// IDs for wake-only or explicit lifecycle invalidations. Snapshot
    /// comparison cannot prove the provenance of these extra wake reasons.
    pub(super) fn verify_dirty_proxy_ids(
        &self,
        frame: &[ProxyFrame<D>],
        supplied: &[ProxyId],
    ) -> Result<bool, TournamentError> {
        let expected = self.derive_dirty_proxy_ids(frame)?;
        let mut previous = None;
        for id in supplied {
            if previous.is_some_and(|prior| prior >= *id) {
                return Ok(false);
            }
            let existed_before = self.previous_bounds.contains_key(id);
            let exists_now = frame.binary_search_by_key(id, |proxy| proxy.id).is_ok();
            if !existed_before && !exists_now {
                return Ok(false);
            }
            previous = Some(*id);
        }
        Ok(expected
            .iter()
            .all(|id| supplied.binary_search(id).is_ok()))
    }

    pub(super) fn step(
        &mut self,
        frame: &[ProxyFrame<D>],
    ) -> Result<PairDeltaFrame, TournamentError> {
        let dirty = self.derive_dirty_proxy_ids(frame)?;
        let mut snapshot_entries_visited = u64::try_from(self.previous_bounds.len())
            .ok()
            .and_then(|previous| {
                u64::try_from(frame.len())
                    .ok()
                    .and_then(|current| {
                        current
                            .checked_mul(2)
                            .and_then(|current| previous.checked_add(current))
                    })
            })
            .ok_or(TournamentError::CounterOverflow)?;
        if self.force_dirty {
            let invalidation_entries = u64::try_from(self.previous_bounds.len())
                .ok()
                .and_then(|previous| {
                    u64::try_from(frame.len())
                        .ok()
                        .and_then(|current| previous.checked_add(current))
                })
                .ok_or(TournamentError::CounterOverflow)?;
            snapshot_entries_visited = snapshot_entries_visited
                .checked_add(invalidation_entries)
                .ok_or(TournamentError::CounterOverflow)?;
        }
        self.apply_dirty_ids(frame, &dirty, true, true, snapshot_entries_visited, 0)
    }

    pub(super) fn step_with_dirty_ids(
        &mut self,
        frame: &[ProxyFrame<D>],
        dirty_ids: &[ProxyId],
    ) -> Result<PairDeltaFrame, TournamentError> {
        let frame_validation_entries_visited =
            u64::try_from(frame.len()).map_err(|_| TournamentError::CounterOverflow)?;
        self.apply_dirty_ids(
            frame,
            dirty_ids,
            false,
            false,
            0,
            frame_validation_entries_visited,
        )
    }

    fn apply_full_relation_reconciliation(
        &mut self,
        frame: &[ProxyFrame<D>],
        dirty: &BTreeSet<ProxyId>,
        dirty_snapshot_entries_visited: u64,
        frame_validation_entries_visited: u64,
    ) -> Result<PairDeltaFrame, TournamentError> {
        // Build and qualify the replacement index off to the side. An error
        // cannot partially mutate the currently committed tree or adjacency.
        let mut next_tree = IncrementalAabbTree::new_with_balance_policy(
            self.tree.fat_margin(),
            self.tree.stats().balance_policy,
        )?;
        for proxy in frame {
            next_tree.insert(proxy.id, proxy.bounds)?;
        }

        let query = next_tree.query_pairs();
        let candidate_count = u64::try_from(query.pairs.len())
            .map_err(|_| TournamentError::CounterOverflow)?;
        let node_visits = u64::try_from(query.stats.node_visits)
            .map_err(|_| TournamentError::CounterOverflow)?;
        let leaf_tests = u64::try_from(query.stats.leaf_tests)
            .map_err(|_| TournamentError::CounterOverflow)?;
        let aabb_tests = u64::try_from(query.stats.aabb_tests)
            .map_err(|_| TournamentError::CounterOverflow)?;

        let mut current_pairs = BTreeSet::<ProxyPair>::new();
        let mut overlap_tests = 0_u64;
        for pair in query.pairs {
            let left = frame
                .binary_search_by_key(&pair.0, |proxy| proxy.id)
                .ok()
                .map(|index| &frame[index].bounds)
                .ok_or(TournamentError::PairCacheInvariant)?;
            let right = frame
                .binary_search_by_key(&pair.1, |proxy| proxy.id)
                .ok()
                .map(|index| &frame[index].bounds)
                .ok_or(TournamentError::PairCacheInvariant)?;
            overlap_tests = overlap_tests
                .checked_add(1)
                .ok_or(TournamentError::CounterOverflow)?;
            if overlaps(left, right) {
                current_pairs.insert(pair);
            }
        }

        let previous_pairs = self.active_pairs().into_iter().collect::<BTreeSet<_>>();
        let added = current_pairs
            .difference(&previous_pairs)
            .copied()
            .collect::<Vec<_>>();
        let removed = previous_pairs
            .difference(&current_pairs)
            .copied()
            .collect::<Vec<_>>();
        let revalidated = current_pairs
            .intersection(&previous_pairs)
            .filter(|pair| dirty.contains(&pair.0) || dirty.contains(&pair.1))
            .copied()
            .collect::<Vec<_>>();

        let next_pair_count = current_pairs.len();
        let mut next_neighbors = BTreeMap::<ProxyId, BTreeSet<ProxyId>>::new();
        for pair in &current_pairs {
            next_neighbors.entry(pair.0).or_default().insert(pair.1);
            next_neighbors.entry(pair.1).or_default().insert(pair.0);
        }
        let next_bounds = frame
            .iter()
            .map(|proxy| (proxy.id, proxy.bounds))
            .collect::<BTreeMap<_, _>>();
        let index_mutations =
            u64::try_from(frame.len()).map_err(|_| TournamentError::CounterOverflow)?;
        let dirty_proxies = dirty.len();

        // Commit the replacement tree and relation only after oracle candidates,
        // exact pair refinement, event sets and all accounting conversions pass.
        self.tree = next_tree;
        self.previous_bounds = next_bounds;
        self.neighbors = next_neighbors;
        self.active_pair_count = next_pair_count;
        self.force_dirty = false;

        Ok(PairDeltaFrame {
            added,
            removed,
            revalidated,
            dirty_proxies,
            overlap_tests,
            active_pairs: self.active_pair_count,
            dirty_snapshot_entries_visited,
            frame_validation_entries_visited,
            frontier_proxy_visits: 0,
            frontier_pair_candidates: candidate_count,
            frontier_unique_pairs: candidate_count,
            frontier_node_visits: node_visits,
            frontier_leaf_tests: leaf_tests,
            frontier_aabb_tests: aabb_tests,
            frontier_index_mutations: index_mutations,
            frontier_index_reinsertions: 0,
            full_relation_reconciliation: true,
        })
    }

    fn apply_dirty_ids(
        &mut self,
        frame: &[ProxyFrame<D>],
        dirty_ids: &[ProxyId],
        allow_full_invalidation: bool,
        frame_prevalidated: bool,
        dirty_snapshot_entries_visited: u64,
        frame_validation_entries_visited: u64,
    ) -> Result<PairDeltaFrame, TournamentError> {
        if !frame_prevalidated {
            validate_proxy_sequence(frame)?;
        }
        if self.force_dirty && !allow_full_invalidation {
            return Err(TournamentError::DirtyJournalCannotApplyFullInvalidation);
        }

        let mut previous_dirty = None;
        for dirty_id in dirty_ids {
            if previous_dirty.is_some_and(|previous| previous >= *dirty_id) {
                return Err(TournamentError::InvalidDirtyJournal);
            }
            let existed_before = self.previous_bounds.contains_key(dirty_id);
            let exists_now = frame.binary_search_by_key(dirty_id, |proxy| proxy.id).is_ok();
            if !existed_before && !exists_now {
                return Err(TournamentError::InvalidDirtyJournal);
            }
            previous_dirty = Some(*dirty_id);
        }
        let dirty = dirty_ids.iter().copied().collect::<BTreeSet<_>>();

        // Avoid an O(k^2) dirty/dirty enumeration when a large fraction of
        // the world changes. In that regime, building a temporary current-frame
        // tree and querying its unique pair relation is the cheaper, simpler
        // reconciliation path. The threshold is a work estimate, not a timing
        // claim; the selected mode is exported in the frame receipt.
        let population = self.previous_bounds.len().max(frame.len()).max(1);
        let log_factor = (usize::BITS - population.leading_zeros()) as usize;
        let dirty_pair_upper_bound =
            dirty.len().saturating_mul(dirty.len().saturating_sub(1)) / 2;
        if dirty_pair_upper_bound > population.saturating_mul(log_factor) {
            return self.apply_full_relation_reconciliation(
                frame,
                &dirty,
                dirty_snapshot_entries_visited,
                frame_validation_entries_visited,
            );
        }

        let mut affected_pairs = BTreeSet::<ProxyPair>::new();
        let mut frontier_proxy_visits = 0_u64;
        let mut frontier_pair_candidates = 0_u64;
        let mut frontier_node_visits = 0_u64;
        let mut frontier_leaf_tests = 0_u64;
        let mut frontier_aabb_tests = 0_u64;
        let mut frontier_index_mutations = 0_u64;
        let mut frontier_index_reinsertions = 0_u64;

        // Preserve old neighbours before changing the index. They are needed
        // to emit removals when a proxy moves away or is removed entirely.
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
        }

        // Query the old conservative index before mutating it. Every
        // unchanged proxy's old fat AABB still contains its current tight AABB,
        // so a current overlap with an unchanged proxy must be returned here.
        // Changed/changed relationships are handled explicitly below.
        for dirty_id in &dirty {
            let Some(index) = frame.binary_search_by_key(dirty_id, |proxy| proxy.id).ok()
            else {
                continue;
            };
            let query = self.tree.query(frame[index].bounds)?;
            frontier_node_visits = frontier_node_visits
                .checked_add(
                    u64::try_from(query.stats.node_visits)
                        .map_err(|_| TournamentError::CounterOverflow)?,
                )
                .ok_or(TournamentError::CounterOverflow)?;
            frontier_leaf_tests = frontier_leaf_tests
                .checked_add(
                    u64::try_from(query.stats.leaf_tests)
                        .map_err(|_| TournamentError::CounterOverflow)?,
                )
                .ok_or(TournamentError::CounterOverflow)?;
            frontier_aabb_tests = frontier_aabb_tests
                .checked_add(
                    u64::try_from(query.stats.aabb_tests)
                        .map_err(|_| TournamentError::CounterOverflow)?,
                )
                .ok_or(TournamentError::CounterOverflow)?;
            for candidate in query.proxies {
                if candidate == *dirty_id || dirty.contains(&candidate) {
                    continue;
                }
                frontier_pair_candidates = frontier_pair_candidates
                    .checked_add(1)
                    .ok_or(TournamentError::CounterOverflow)?;
                if let Some(pair) = ProxyPair::new(*dirty_id, candidate) {
                    affected_pairs.insert(pair);
                }
            }
        }

        // Two changed endpoints may have moved together so their old tree
        // leaves do not overlap. Enumerate dirty/dirty pairs explicitly.
        let current_dirty = dirty
            .iter()
            .filter(|id| frame.binary_search_by_key(id, |proxy| proxy.id).is_ok())
            .copied()
            .collect::<Vec<_>>();
        for left_index in 0..current_dirty.len() {
            for right_index in (left_index + 1)..current_dirty.len() {
                frontier_pair_candidates = frontier_pair_candidates
                    .checked_add(1)
                    .ok_or(TournamentError::CounterOverflow)?;
                if let Some(pair) =
                    ProxyPair::new(current_dirty[left_index], current_dirty[right_index])
                {
                    affected_pairs.insert(pair);
                }
            }
        }

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut revalidated = Vec::new();
        let mut overlap_tests = 0_u64;
        for pair in &affected_pairs {
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
            match (left_active, is_active) {
                (false, true) => added.push(*pair),
                (true, false) => removed.push(*pair),
                (true, true) => revalidated.push(*pair),
                (false, false) => {}
            }
        }

        let next_pair_count = self
            .active_pair_count
            .checked_sub(removed.len())
            .and_then(|count| count.checked_add(added.len()))
            .ok_or(TournamentError::CounterOverflow)?;

        // The old-bounds map is the cache's expected leaf-membership
        // ledger. Check the global count as well as dirty leaves, so a stale
        // non-dirty tree leaf cannot silently survive a commit.
        if self.tree.len() != self.previous_bounds.len() {
            return Err(TournamentError::PairCacheInvariant);
        }

        // Preflight every index operation before mutation. Also validate every
        // new fat AABB so user-data errors cannot arise halfway through commit.
        for dirty_id in &dirty {
            let existed_before = self.previous_bounds.contains_key(dirty_id);
            if self.tree.contains_proxy(*dirty_id) != existed_before {
                return Err(TournamentError::PairCacheInvariant);
            }
            if let Some(index) = frame.binary_search_by_key(dirty_id, |proxy| proxy.id).ok() {
                let next = frame[index].bounds;
                if self
                    .previous_bounds
                    .get(dirty_id)
                    .is_none_or(|previous| *previous != next)
                {
                    next.fatten(self.tree.fat_margin())?;
                }
            }
        }

        // Apply the staged spatial-index mutations only after all pair
        // transitions, pair-count arithmetic, membership checks and AABB
        // fattening checks have been validated.
        for dirty_id in &dirty {
            let current = frame
                .binary_search_by_key(dirty_id, |proxy| proxy.id)
                .ok()
                .map(|index| frame[index].bounds);
            match (self.previous_bounds.get(dirty_id), current) {
                (Some(previous), Some(next)) if *previous != next => {
                    if self.tree.update(*dirty_id, next)? {
                        frontier_index_reinsertions = frontier_index_reinsertions
                            .checked_add(1)
                            .ok_or(TournamentError::CounterOverflow)?;
                    }
                    frontier_index_mutations = frontier_index_mutations
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                }
                (None, Some(next)) => {
                    self.tree.insert(*dirty_id, next)?;
                    frontier_index_mutations = frontier_index_mutations
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                }
                (Some(_), None) => {
                    self.tree.remove(*dirty_id)?;
                    frontier_index_mutations = frontier_index_mutations
                        .checked_add(1)
                        .ok_or(TournamentError::CounterOverflow)?;
                }
                (Some(_), Some(_)) | (None, None) => {}
            }
        }

        // Commit symmetric adjacency only after the transition is validated.
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
            dirty_snapshot_entries_visited,
            frame_validation_entries_visited,
            frontier_proxy_visits,
            frontier_pair_candidates,
            frontier_unique_pairs: u64::try_from(affected_pairs.len())
                .map_err(|_| TournamentError::CounterOverflow)?,
            frontier_node_visits,
            frontier_leaf_tests,
            frontier_aabb_tests,
            frontier_index_mutations,
            frontier_index_reinsertions,
            full_relation_reconciliation: false,
        })
    }
}
