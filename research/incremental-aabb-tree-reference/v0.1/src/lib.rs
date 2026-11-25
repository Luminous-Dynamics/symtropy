// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Deterministic incremental AABB tree reference for broadphase tournaments.
//!
//! This is a benchmark-only reference implementation. It prioritizes stable
//! identities, conservative fat bounds, inspectable work counters and a simple
//! mutation contract over production-specific lock-free or SIMD optimizations.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProxyId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb<const D: usize> {
    min: [f64; D],
    max: [f64; D],
}

impl<const D: usize> Aabb<D> {
    pub fn new(min: [f64; D], max: [f64; D]) -> Result<Self, TreeError> {
        let value = Self { min, max };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(TreeError::InvalidBounds)
        }
    }

    #[must_use]
    pub fn min(&self) -> &[f64; D] {
        &self.min
    }

    #[must_use]
    pub fn max(&self) -> &[f64; D] {
        &self.max
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        (0..D).all(|axis| {
            self.min[axis].is_finite()
                && self.max[axis].is_finite()
                && self.min[axis] <= self.max[axis]
        })
    }

    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        (0..D).all(|axis| {
            self.max[axis] >= other.min[axis] && other.max[axis] >= self.min[axis]
        })
    }

    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        (0..D).all(|axis| {
            self.min[axis] <= other.min[axis] && self.max[axis] >= other.max[axis]
        })
    }

    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: std::array::from_fn(|axis| self.min[axis].min(other.min[axis])),
            max: std::array::from_fn(|axis| self.max[axis].max(other.max[axis])),
        }
    }

    pub fn fatten(&self, margin: f64) -> Result<Self, TreeError> {
        if !self.is_valid() {
            return Err(TreeError::InvalidBounds);
        }
        if !margin.is_finite() || margin < 0.0 {
            return Err(TreeError::InvalidMargin);
        }
        let result = Self {
            min: std::array::from_fn(|axis| self.min[axis] - margin),
            max: std::array::from_fn(|axis| self.max[axis] + margin),
        };
        if result.is_valid() {
            Ok(result)
        } else {
            Err(TreeError::InvalidBounds)
        }
    }

    /// Generalized AABB boundary content. This is perimeter in 2D and surface
    /// area in 3D. It is used only as a deterministic insertion heuristic,
    /// not as a cross-dimensional quality score.
    #[must_use]
    pub fn boundary_content(&self) -> f64 {
        let widths = std::array::from_fn::<_, D, _>(|axis| self.max[axis] - self.min[axis]);
        let mut sum = 0.0;
        for excluded_axis in 0..D {
            let mut product = 1.0;
            for axis in 0..D {
                if axis != excluded_axis {
                    product *= widths[axis];
                }
            }
            sum += 2.0 * product;
        }
        sum
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreeError {
    InvalidBounds,
    InvalidMargin,
    DuplicateProxy,
    MissingProxy,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for TreeError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueryStats {
    /// Internal nodes whose AABBs were tested.
    pub node_visits: usize,
    /// Leaf proxies whose AABBs were tested.
    pub leaf_tests: usize,
    /// Total node and leaf AABB tests.
    pub aabb_tests: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryResult {
    pub proxies: Vec<ProxyId>,
    pub stats: QueryStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ProxyPair(pub ProxyId, pub ProxyId);

impl ProxyPair {
    #[must_use]
    pub fn new(first: ProxyId, second: ProxyId) -> Option<Self> {
        if first == second {
            None
        } else if first < second {
            Some(Self(first, second))
        } else {
            Some(Self(second, first))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PairQueryStats {
    pub queries: usize,
    pub node_visits: usize,
    pub leaf_tests: usize,
    pub aabb_tests: usize,
    pub emitted_pairs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairQueryResult {
    pub pairs: Vec<ProxyPair>,
    pub stats: PairQueryStats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MaintenanceStats {
    pub insert_operations: u64,
    pub remove_operations: u64,
    pub update_operations: u64,
    pub fat_contained_updates: u64,
    pub proxy_reinsertions: u64,
    pub full_rebuilds: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TreeStats {
    pub proxy_count: usize,
    pub node_count: usize,
    pub height: usize,
    pub free_node_count: usize,
    pub maintenance: MaintenanceStats,
}

#[derive(Clone, Debug)]
struct Node<const D: usize> {
    aabb: Aabb<D>,
    parent: Option<usize>,
    left: Option<usize>,
    right: Option<usize>,
    proxy: Option<ProxyId>,
    height: usize,
    min_proxy: ProxyId,
}

impl<const D: usize> Node<D> {
    fn leaf(aabb: Aabb<D>, proxy: ProxyId) -> Self {
        Self {
            aabb,
            parent: None,
            left: None,
            right: None,
            proxy: Some(proxy),
            height: 0,
            min_proxy: proxy,
        }
    }

    fn branch(aabb: Aabb<D>, left: usize, right: usize) -> Self {
        Self {
            aabb,
            parent: None,
            left: Some(left),
            right: Some(right),
            proxy: None,
            height: 1,
            min_proxy: ProxyId(u64::MAX),
        }
    }

    fn is_leaf(&self) -> bool {
        self.proxy.is_some()
    }
}

/// A benchmark-only dynamic AABB tree with stable proxy IDs and fat bounds.
///
/// Insert/remove/update operations are deterministic for a fixed operation
/// sequence. Updating a tight AABB contained by the existing fat AABB avoids
/// reinsertion. Otherwise the proxy is detached and reinserted using a
/// generalized perimeter-cost heuristic. This baseline does not implement
/// concurrent updates or automatic rotations; height and traversal counters
/// are exposed so those costs and limitations remain visible.
pub struct IncrementalAabbTree<const D: usize> {
    root: Option<usize>,
    nodes: Vec<Option<Node<D>>>,
    free_nodes: Vec<usize>,
    leaves: BTreeMap<ProxyId, usize>,
    fat_margin: f64,
    maintenance: MaintenanceStats,
}

impl<const D: usize> IncrementalAabbTree<D> {
    pub fn new(fat_margin: f64) -> Result<Self, TreeError> {
        if !fat_margin.is_finite() || fat_margin < 0.0 {
            return Err(TreeError::InvalidMargin);
        }
        Ok(Self {
            root: None,
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            leaves: BTreeMap::new(),
            fat_margin,
            maintenance: MaintenanceStats::default(),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    #[must_use]
    pub fn fat_margin(&self) -> f64 {
        self.fat_margin
    }

    #[must_use]
    pub fn stats(&self) -> TreeStats {
        TreeStats {
            proxy_count: self.leaves.len(),
            node_count: self.nodes.iter().filter(|node| node.is_some()).count(),
            height: self.root.map_or(0, |root| self.node(root).height),
            free_node_count: self.free_nodes.len(),
            maintenance: self.maintenance,
        }
    }

    pub fn insert(&mut self, proxy: ProxyId, tight_aabb: Aabb<D>) -> Result<(), TreeError> {
        if self.leaves.contains_key(&proxy) {
            return Err(TreeError::DuplicateProxy);
        }
        let fat_aabb = tight_aabb.fatten(self.fat_margin)?;
        let leaf = self.allocate_node(Node::leaf(fat_aabb, proxy));
        self.leaves.insert(proxy, leaf);
        self.insert_leaf(leaf);
        self.maintenance.insert_operations =
            self.maintenance.insert_operations.saturating_add(1);
        Ok(())
    }

    /// Remove one proxy. Returns the removed fat AABB for audit/debug use.
    pub fn remove(&mut self, proxy: ProxyId) -> Result<Aabb<D>, TreeError> {
        let leaf = self
            .leaves
            .remove(&proxy)
            .ok_or(TreeError::MissingProxy)?;
        let old_aabb = self.node(leaf).aabb;
        self.detach_leaf(leaf);
        self.free_node(leaf);
        self.maintenance.remove_operations =
            self.maintenance.remove_operations.saturating_add(1);
        Ok(old_aabb)
    }

    /// Update a proxy's tight AABB. Returns true only when the fat proxy had
    /// to be reinserted. If the new tight bounds remain contained, the old fat
    /// AABB is still a conservative enclosure and no tree mutation occurs.
    pub fn update(
        &mut self,
        proxy: ProxyId,
        tight_aabb: Aabb<D>,
    ) -> Result<bool, TreeError> {
        let leaf = *self.leaves.get(&proxy).ok_or(TreeError::MissingProxy)?;
        let new_fat_aabb = tight_aabb.fatten(self.fat_margin)?;
        self.maintenance.update_operations =
            self.maintenance.update_operations.saturating_add(1);
        if self.node(leaf).aabb.contains(&tight_aabb) {
            self.maintenance.fat_contained_updates =
                self.maintenance.fat_contained_updates.saturating_add(1);
            return Ok(false);
        }

        self.detach_leaf(leaf);
        self.node_mut(leaf).aabb = new_fat_aabb;
        self.insert_leaf(leaf);
        self.maintenance.proxy_reinsertions =
            self.maintenance.proxy_reinsertions.saturating_add(1);
        Ok(true)
    }

    pub fn query(&self, query_aabb: Aabb<D>) -> Result<QueryResult, TreeError> {
        if !query_aabb.is_valid() {
            return Err(TreeError::InvalidBounds);
        }
        let mut result = QueryResult {
            proxies: Vec::new(),
            stats: QueryStats::default(),
        };
        let Some(root) = self.root else {
            return Ok(result);
        };

        let mut stack = Vec::with_capacity(64);
        stack.push(root);
        while let Some(index) = stack.pop() {
            let node = self.node(index);
            result.stats.aabb_tests += 1;
            if node.is_leaf() {
                result.stats.leaf_tests += 1;
                if query_aabb.overlaps(&node.aabb) {
                    result.proxies.push(node.proxy.expect("leaf proxy"));
                }
            } else {
                result.stats.node_visits += 1;
                if query_aabb.overlaps(&node.aabb) {
                    let left = node.left.expect("branch left child");
                    let right = node.right.expect("branch right child");
                    let left_key = self.node(left).min_proxy;
                    let right_key = self.node(right).min_proxy;
                    // Push larger key first so the smaller key is visited first.
                    if left_key <= right_key {
                        stack.push(right);
                        stack.push(left);
                    } else {
                        stack.push(left);
                        stack.push(right);
                    }
                }
            }
        }
        result.proxies.sort_unstable();
        Ok(result)
    }

    /// Emit each overlapping fat-proxy pair exactly once, in canonical order.
    pub fn query_pairs(&self) -> PairQueryResult {
        let mut result = PairQueryResult {
            pairs: Vec::new(),
            stats: PairQueryStats::default(),
        };
        for (&proxy, &leaf) in &self.leaves {
            let query = self.query(self.node(leaf).aabb).expect("stored bounds are valid");
            result.stats.queries += 1;
            result.stats.node_visits += query.stats.node_visits;
            result.stats.leaf_tests += query.stats.leaf_tests;
            result.stats.aabb_tests += query.stats.aabb_tests;
            for candidate in query.proxies {
                if candidate > proxy {
                    result
                        .pairs
                        .push(ProxyPair::new(proxy, candidate).expect("distinct proxy IDs"));
                }
            }
        }
        result.pairs.sort_unstable();
        result.pairs.dedup();
        result.stats.emitted_pairs = result.pairs.len();
        result
    }

    /// Rebuild the hierarchy in stable proxy-ID order while preserving fat
    /// bounds. This is a benchmark control for measuring rebuild/recovery cost.
    pub fn rebuild(&mut self) {
        self.maintenance.full_rebuilds =
            self.maintenance.full_rebuilds.saturating_add(1);
        let mut entries = self
            .leaves
            .iter()
            .map(|(&proxy, &leaf)| (proxy, self.node(leaf).aabb))
            .collect::<Vec<_>>();
        entries.sort_unstable_by_key(|(proxy, _)| *proxy);
        self.root = None;
        self.nodes.clear();
        self.free_nodes.clear();
        self.leaves.clear();
        for (proxy, fat_aabb) in entries {
            let leaf = self.allocate_node(Node::leaf(fat_aabb, proxy));
            self.leaves.insert(proxy, leaf);
            self.insert_leaf(leaf);
        }
    }

    fn allocate_node(&mut self, node: Node<D>) -> usize {
        if let Some(index) = self.free_nodes.pop() {
            self.nodes[index] = Some(node);
            index
        } else {
            let index = self.nodes.len();
            self.nodes.push(Some(node));
            index
        }
    }

    fn free_node(&mut self, index: usize) {
        self.nodes[index] = None;
        self.free_nodes.push(index);
    }

    fn node(&self, index: usize) -> &Node<D> {
        self.nodes[index].as_ref().expect("live tree node")
    }

    fn node_mut(&mut self, index: usize) -> &mut Node<D> {
        self.nodes[index].as_mut().expect("live tree node")
    }

    fn insert_leaf(&mut self, leaf: usize) {
        let Some(root) = self.root else {
            self.root = Some(leaf);
            return;
        };

        let leaf_aabb = self.node(leaf).aabb;
        let mut sibling = root;
        while !self.node(sibling).is_leaf() {
            let node = self.node(sibling);
            let left = node.left.expect("branch left child");
            let right = node.right.expect("branch right child");
            let combined = node.aabb.union(&leaf_aabb);
            let combined_cost = 2.0 * combined.boundary_content();
            let inheritance_cost = 2.0 * nonnegative_cost_delta(
                combined.boundary_content(),
                node.aabb.boundary_content(),
            );
            let left_cost = self.descend_cost(left, &leaf_aabb, inheritance_cost);
            let right_cost = self.descend_cost(right, &leaf_aabb, inheritance_cost);

            if combined_cost <= left_cost && combined_cost <= right_cost {
                break;
            }
            sibling = if left_cost < right_cost
                || (left_cost == right_cost
                    && self.node(left).min_proxy <= self.node(right).min_proxy)
            {
                left
            } else {
                right
            };
        }

        let old_parent = self.node(sibling).parent;
        let new_parent_aabb = self.node(sibling).aabb.union(&leaf_aabb);
        let new_parent_height = 1 + self.node(sibling).height.max(self.node(leaf).height);
        let new_parent_min_proxy = self.node(sibling).min_proxy.min(self.node(leaf).min_proxy);
        let new_parent = self.allocate_node(Node::branch(new_parent_aabb, sibling, leaf));
        {
            let parent = self.node_mut(new_parent);
            parent.parent = old_parent;
            parent.height = new_parent_height;
            parent.min_proxy = new_parent_min_proxy;
        }
        self.node_mut(sibling).parent = Some(new_parent);
        self.node_mut(leaf).parent = Some(new_parent);

        if let Some(old_parent) = old_parent {
            let old = self.node_mut(old_parent);
            if old.left == Some(sibling) {
                old.left = Some(new_parent);
            } else {
                old.right = Some(new_parent);
            }
            self.fix_upwards(old_parent);
        } else {
            self.root = Some(new_parent);
        }
    }

    fn descend_cost(&self, child: usize, leaf_aabb: &Aabb<D>, inheritance: f64) -> f64 {
        let node = self.node(child);
        let combined = node.aabb.union(leaf_aabb);
        let combined_content = combined.boundary_content();
        if node.is_leaf() {
            combined_content + inheritance
        } else {
            nonnegative_cost_delta(combined_content, node.aabb.boundary_content()) + inheritance
        }
    }

    fn detach_leaf(&mut self, leaf: usize) {
        let Some(parent) = self.node(leaf).parent else {
            self.root = None;
            return;
        };
        let grandparent = self.node(parent).parent;
        let sibling = if self.node(parent).left == Some(leaf) {
            self.node(parent).right.expect("parent right sibling")
        } else {
            self.node(parent).left.expect("parent left sibling")
        };

        if let Some(grandparent) = grandparent {
            if self.node(grandparent).left == Some(parent) {
                self.node_mut(grandparent).left = Some(sibling);
            } else {
                self.node_mut(grandparent).right = Some(sibling);
            }
            self.node_mut(sibling).parent = Some(grandparent);
            self.node_mut(leaf).parent = None;
            self.free_node(parent);
            self.fix_upwards(grandparent);
        } else {
            self.root = Some(sibling);
            self.node_mut(sibling).parent = None;
            self.node_mut(leaf).parent = None;
            self.free_node(parent);
        }
    }

    fn fix_upwards(&mut self, mut index: usize) {
        loop {
            let (left, right) = {
                let node = self.node(index);
                (
                    node.left.expect("branch left child"),
                    node.right.expect("branch right child"),
                )
            };
            let left_node = self.node(left);
            let right_node = self.node(right);
            let aabb = left_node.aabb.union(&right_node.aabb);
            let height = 1 + left_node.height.max(right_node.height);
            let min_proxy = left_node.min_proxy.min(right_node.min_proxy);
            {
                let node = self.node_mut(index);
                node.aabb = aabb;
                node.height = height;
                node.min_proxy = min_proxy;
            }
            let Some(parent) = self.node(index).parent else {
                break;
            };
            index = parent;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkloadFamily {
    Uniform,
    Clustered,
    Corridor,
    HighAspectRatio,
    HighOverlap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkloadSpec {
    pub proxy_count: usize,
    pub seed: u64,
    pub family: WorkloadFamily,
    /// Dynamic proxy fraction in basis points: 0 = all static, 10_000 = all moving.
    pub moving_basis_points: u16,
}

impl WorkloadSpec {
    pub fn validate(self) -> Result<Self, WorkloadError> {
        if self.moving_basis_points > 10_000 {
            Err(WorkloadError::InvalidMovingFraction)
        } else {
            Ok(self)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkloadProxy<const D: usize> {
    pub id: ProxyId,
    pub tight_aabb: Aabb<D>,
    pub moving: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedWorkload<const D: usize> {
    pub spec: WorkloadSpec,
    pub proxies: Vec<WorkloadProxy<D>>,
}

impl<const D: usize> GeneratedWorkload<D> {
    /// Non-cryptographic FNV-1a diagnostic hash over the generated proxy
    /// sequence. Use a cryptographic manifest digest for published receipts.
    #[must_use]
    pub fn diagnostic_hash(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut mix = |word: u64| {
            for byte in word.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        mix(self.spec.seed);
        mix(self.spec.proxy_count as u64);
        mix(self.spec.moving_basis_points as u64);
        mix(self.spec.family as u64);
        mix(D as u64);
        for proxy in &self.proxies {
            mix(proxy.id.0);
            mix(if proxy.moving { 1 } else { 0 });
            for axis in 0..D {
                mix(proxy.tight_aabb.min[axis].to_bits());
                mix(proxy.tight_aabb.max[axis].to_bits());
            }
        }
        hash
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkloadError {
    InvalidDimension,
    InvalidMovingFraction,
    InvalidGeneratedBounds,
}

impl fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for WorkloadError {}

/// Generate a stable workload from a seed and explicit distribution family.
/// All generated coordinates are finite binary64 values; the spec and
/// diagnostic hash identify the exact generated input for local comparisons.
pub fn generate_workload<const D: usize>(
    spec: WorkloadSpec,
) -> Result<GeneratedWorkload<D>, WorkloadError> {
    if D < 2 {
        return Err(WorkloadError::InvalidDimension);
    }
    let spec = spec.validate()?;
    let mut random = SplitMix64::new(spec.seed);
    let mut proxies = Vec::with_capacity(spec.proxy_count);

    for index in 0..spec.proxy_count {
        let dominant_axis = index % D.max(1);
        let mut min = [0.0; D];
        let mut max = [0.0; D];
        for axis in 0..D {
            let (center, half_extent) = match spec.family {
                WorkloadFamily::Uniform => {
                    (random.range(0.0, 1_000.0), random.range(0.25, 2.0))
                }
                WorkloadFamily::Clustered => {
                    let cluster = (random.next_u64() % 8) as f64 * 120.0;
                    (
                        cluster + random.range(-12.0, 12.0),
                        random.range(0.5, 3.0),
                    )
                }
                WorkloadFamily::Corridor => {
                    if axis == 0 {
                        (random.range(0.0, 1_000.0), random.range(3.0, 20.0))
                    } else {
                        (random.range(-3.0, 3.0), random.range(0.1, 1.0))
                    }
                }
                WorkloadFamily::HighAspectRatio => {
                    let half = if axis == dominant_axis {
                        random.range(10.0, 40.0)
                    } else {
                        random.range(0.05, 0.5)
                    };
                    (random.range(0.0, 1_000.0), half)
                }
                WorkloadFamily::HighOverlap => {
                    (50.0 + random.range(-5.0, 5.0), random.range(8.0, 12.0))
                }
            };
            min[axis] = center - half_extent;
            max[axis] = center + half_extent;
        }
        let tight_aabb = Aabb::new(min, max).map_err(|_| WorkloadError::InvalidGeneratedBounds)?;
        let moving = random.next_u64() % 10_000 < u64::from(spec.moving_basis_points);
        proxies.push(WorkloadProxy {
            id: ProxyId(index as u64),
            tight_aabb,
            moving,
        });
    }

    Ok(GeneratedWorkload { spec, proxies })
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
    }

    fn range(&mut self, minimum: f64, maximum: f64) -> f64 {
        minimum + (maximum - minimum) * self.unit()
    }
}

fn nonnegative_cost_delta(new_cost: f64, old_cost: f64) -> f64 {
    if new_cost.is_finite() && old_cost.is_finite() {
        (new_cost - old_cost).max(0.0)
    } else if new_cost == old_cost {
        0.0
    } else {
        f64::MAX
    }
}

impl<const D: usize> Default for IncrementalAabbTree<D> {
    fn default() -> Self {
        Self::new(0.1).expect("static default margin is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb<const D: usize>(min: [f64; D], max: [f64; D]) -> Aabb<D> {
        Aabb::new(min, max).unwrap()
    }

    #[test]
    fn validates_bounds_and_margin() {
        assert_eq!(
            Aabb::<2>::new([f64::NAN, 0.0], [1.0, 1.0]),
            Err(TreeError::InvalidBounds)
        );
        assert_eq!(
            Aabb::<2>::new([2.0, 0.0], [1.0, 1.0]),
            Err(TreeError::InvalidBounds)
        );
        assert_eq!(
            IncrementalAabbTree::<2>::new(-0.1).err(),
            Some(TreeError::InvalidMargin)
        );
        assert_eq!(
            IncrementalAabbTree::<2>::new(f64::INFINITY).err(),
            Some(TreeError::InvalidMargin)
        );
    }

    #[test]
    fn overlap_is_inclusive_and_touching_boxes_are_candidates() {
        let mut tree = IncrementalAabbTree::<2>::new(0.0).unwrap();
        tree.insert(ProxyId(2), aabb([0.0, 0.0], [1.0, 1.0])).unwrap();
        tree.insert(ProxyId(1), aabb([1.0, 0.0], [2.0, 1.0])).unwrap();
        let result = tree.query(aabb([1.0, 0.0], [1.0, 1.0])).unwrap();
        assert_eq!(result.proxies, vec![ProxyId(1), ProxyId(2)]);
        assert_eq!(tree.query_pairs().pairs, vec![ProxyPair(ProxyId(1), ProxyId(2))]);
    }

    #[test]
    fn rejects_duplicate_and_missing_proxy_mutations() {
        let mut tree = IncrementalAabbTree::<2>::new(0.25).unwrap();
        let bounds = aabb([0.0, 0.0], [1.0, 1.0]);
        tree.insert(ProxyId(7), bounds).unwrap();
        assert_eq!(tree.insert(ProxyId(7), bounds), Err(TreeError::DuplicateProxy));
        assert_eq!(tree.remove(ProxyId(9)), Err(TreeError::MissingProxy));
        assert_eq!(tree.update(ProxyId(9), bounds), Err(TreeError::MissingProxy));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn contained_motion_skips_reinsert_but_escape_reinserts() {
        let mut tree = IncrementalAabbTree::<2>::new(0.5).unwrap();
        tree.insert(ProxyId(1), aabb([0.0, 0.0], [1.0, 1.0])).unwrap();

        assert!(!tree
            .update(ProxyId(1), aabb([0.25, 0.25], [0.75, 0.75]))
            .unwrap());
        assert!(tree
            .update(ProxyId(1), aabb([2.0, 0.0], [3.0, 1.0]))
            .unwrap());
        let result = tree.query(aabb([2.0, 0.0], [3.0, 1.0])).unwrap();
        assert_eq!(result.proxies, vec![ProxyId(1)]);
    }

    #[test]
    fn pair_queries_match_independent_brute_force_oracle() {
        let mut tree = IncrementalAabbTree::<3>::new(0.25).unwrap();
        let mut tight = Vec::new();
        for index in 0..256_u64 {
            let x = ((index * 37) % 113) as f64 * 0.75;
            let y = ((index * 19) % 71) as f64 * 0.5;
            let z = ((index * 11) % 43) as f64 * 0.25;
            let radius = if index % 7 == 0 { 1.5 } else { 0.35 };
            let bounds = aabb(
                [x - radius, y - radius, z - radius],
                [x + radius, y + radius, z + radius],
            );
            tight.push((ProxyId(index), bounds));
            tree.insert(ProxyId(index), bounds).unwrap();
        }

        let mut oracle = Vec::new();
        for first in 0..tight.len() {
            for second in (first + 1)..tight.len() {
                let (first_id, first_aabb) = tight[first];
                let (second_id, second_aabb) = tight[second];
                if first_aabb
                    .fatten(0.25)
                    .unwrap()
                    .overlaps(&second_aabb.fatten(0.25).unwrap())
                {
                    oracle.push(ProxyPair(first_id, second_id));
                }
            }
        }
        oracle.sort_unstable();
        let result = tree.query_pairs();
        assert_eq!(result.pairs, oracle);
        assert_eq!(result.stats.emitted_pairs, oracle.len());
        assert_eq!(result.stats.queries, 256);
        assert!(result.stats.aabb_tests > 0);
        assert!(tree.stats().height > 0);
    }

    #[test]
    fn pair_set_is_invariant_under_translation_scale_axis_permutation_and_reflection() {
        fn pair_set(bounds: &[(ProxyId, Aabb<3>)]) -> Vec<ProxyPair> {
            let mut tree = IncrementalAabbTree::<3>::new(0.125).unwrap();
            for (proxy, aabb) in bounds {
                tree.insert(*proxy, *aabb).unwrap();
            }
            tree.query_pairs().pairs
        }

        let mut original = Vec::new();
        for index in 0..96_u64 {
            let x = ((index * 37) % 127) as f64 * 0.3;
            let y = ((index * 19) % 83) as f64 * 0.4;
            let z = ((index * 11) % 59) as f64 * 0.5;
            let radius = if index % 9 == 0 { 1.25 } else { 0.2 };
            original.push((
                ProxyId(index),
                aabb(
                    [x - radius, y - radius, z - radius],
                    [x + radius, y + radius, z + radius],
                ),
            ));
        }
        let expected = pair_set(&original);

        let translated = original
            .iter()
            .map(|(proxy, bounds)| {
                (
                    *proxy,
                    aabb(
                        [
                            bounds.min[0] + 17.0,
                            bounds.min[1] - 9.0,
                            bounds.min[2] + 3.0,
                        ],
                        [
                            bounds.max[0] + 17.0,
                            bounds.max[1] - 9.0,
                            bounds.max[2] + 3.0,
                        ],
                    ),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(pair_set(&translated), expected);

        let scaled = original
            .iter()
            .map(|(proxy, bounds)| {
                (
                    *proxy,
                    aabb(
                        [
                            bounds.min[0] * 3.5,
                            bounds.min[1] * 3.5,
                            bounds.min[2] * 3.5,
                        ],
                        [
                            bounds.max[0] * 3.5,
                            bounds.max[1] * 3.5,
                            bounds.max[2] * 3.5,
                        ],
                    ),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(pair_set(&scaled), expected);

        let permuted = original
            .iter()
            .map(|(proxy, bounds)| {
                (
                    *proxy,
                    aabb(
                        [bounds.min[2], bounds.min[0], bounds.min[1]],
                        [bounds.max[2], bounds.max[0], bounds.max[1]],
                    ),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(pair_set(&permuted), expected);

        let reflected = original
            .iter()
            .map(|(proxy, bounds)| {
                (
                    *proxy,
                    aabb(
                        [-bounds.max[0], bounds.min[1], bounds.min[2]],
                        [-bounds.min[0], bounds.max[1], bounds.max[2]],
                    ),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(pair_set(&reflected), expected);
    }

    #[test]
    fn removing_proxies_preserves_tree_and_pair_completeness() {
        let mut tree = IncrementalAabbTree::<2>::new(0.0).unwrap();
        for index in 0..40_u64 {
            let start = index as f64;
            tree.insert(
                ProxyId(index),
                aabb([start, 0.0], [start + 1.0, 1.0]),
            )
            .unwrap();
        }
        for index in (0..40_u64).step_by(2) {
            tree.remove(ProxyId(index)).unwrap();
        }
        assert_eq!(tree.len(), 20);
        assert_eq!(tree.query_pairs().pairs.len(), 0);
        assert_eq!(tree.stats().node_count, 39);
    }

    #[test]
    fn rebuild_preserves_pair_set_and_stable_query_order() {
        let mut tree = IncrementalAabbTree::<2>::new(0.2).unwrap();
        for index in (0..64_u64).rev() {
            let start = (index % 8) as f64;
            tree.insert(
                ProxyId(index),
                aabb([start, 0.0], [start + 0.75, 0.75]),
            )
            .unwrap();
        }
        let before = tree.query_pairs().pairs;
        tree.rebuild();
        let after = tree.query_pairs().pairs;
        assert_eq!(before, after);
        assert!(after.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn workload_generation_is_deterministic_and_matches_brute_force() {
        let families = [
            WorkloadFamily::Uniform,
            WorkloadFamily::Clustered,
            WorkloadFamily::Corridor,
            WorkloadFamily::HighAspectRatio,
            WorkloadFamily::HighOverlap,
        ];
        for family in families {
            let spec = WorkloadSpec {
                proxy_count: 96,
                seed: 0x5eed_1234,
                family,
                moving_basis_points: 3_500,
            };
            let first = generate_workload::<3>(spec).unwrap();
            let second = generate_workload::<3>(spec).unwrap();
            assert_eq!(first, second);
            assert_eq!(first.diagnostic_hash(), second.diagnostic_hash());

            let mut tree = IncrementalAabbTree::<3>::new(0.25).unwrap();
            for proxy in &first.proxies {
                tree.insert(proxy.id, proxy.tight_aabb).unwrap();
            }
            let result = tree.query_pairs();

            let mut oracle = Vec::new();
            for left in 0..first.proxies.len() {
                for right in (left + 1)..first.proxies.len() {
                    let a = first.proxies[left].tight_aabb.fatten(0.25).unwrap();
                    let b = first.proxies[right].tight_aabb.fatten(0.25).unwrap();
                    if a.overlaps(&b) {
                        oracle.push(ProxyPair(
                            first.proxies[left].id,
                            first.proxies[right].id,
                        ));
                    }
                }
            }
            oracle.sort_unstable();
            assert_eq!(result.pairs, oracle, "family {family:?}");
            assert_eq!(first.proxies.iter().filter(|proxy| proxy.moving).count() > 0, true);
        }
    }

    #[test]
    fn workload_generator_covers_moving_fraction_and_rejects_invalid_profiles() {
        let static_only = generate_workload::<2>(WorkloadSpec {
            proxy_count: 32,
            seed: 42,
            family: WorkloadFamily::Uniform,
            moving_basis_points: 0,
        })
        .unwrap();
        assert!(static_only.proxies.iter().all(|proxy| !proxy.moving));

        let all_moving = generate_workload::<4>(WorkloadSpec {
            proxy_count: 32,
            seed: 42,
            family: WorkloadFamily::Corridor,
            moving_basis_points: 10_000,
        })
        .unwrap();
        assert!(all_moving.proxies.iter().all(|proxy| proxy.moving));

        assert_eq!(
            WorkloadSpec {
                proxy_count: 1,
                seed: 1,
                family: WorkloadFamily::Uniform,
                moving_basis_points: 10_001,
            }
            .validate(),
            Err(WorkloadError::InvalidMovingFraction)
        );
        assert_eq!(
            generate_workload::<1>(WorkloadSpec {
                proxy_count: 1,
                seed: 1,
                family: WorkloadFamily::Uniform,
                moving_basis_points: 0,
            }),
            Err(WorkloadError::InvalidDimension)
        );
    }

    #[test]
    fn maintenance_counters_separate_contained_updates_reinserts_and_rebuilds() {
        let mut tree = IncrementalAabbTree::<2>::new(0.5).unwrap();
        tree.insert(ProxyId(1), aabb([0.0, 0.0], [1.0, 1.0])).unwrap();
        tree.update(ProxyId(1), aabb([0.25, 0.25], [0.75, 0.75]))
            .unwrap();
        tree.update(ProxyId(1), aabb([3.0, 0.0], [4.0, 1.0]))
            .unwrap();
        tree.remove(ProxyId(1)).unwrap();
        tree.rebuild();

        let maintenance = tree.stats().maintenance;
        assert_eq!(maintenance.insert_operations, 1);
        assert_eq!(maintenance.update_operations, 2);
        assert_eq!(maintenance.fat_contained_updates, 1);
        assert_eq!(maintenance.proxy_reinsertions, 1);
        assert_eq!(maintenance.remove_operations, 1);
        assert_eq!(maintenance.full_rebuilds, 1);
    }

    #[test]
    fn failed_invalid_update_does_not_change_existing_tree() {
        let mut tree = IncrementalAabbTree::<2>::new(0.0).unwrap();
        tree.insert(ProxyId(1), aabb([0.0, 0.0], [1.0, 1.0])).unwrap();
        assert_eq!(
            tree.update(ProxyId(1), Aabb::new([f64::NAN, 0.0], [1.0, 1.0]).unwrap_or(
                Aabb { min: [f64::NAN, 0.0], max: [1.0, 1.0] }
            )),
            Err(TreeError::InvalidBounds)
        );
        assert_eq!(tree.query_pairs().pairs, Vec::<ProxyPair>::new());
    }
}
