// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Broadphase collision detection via LBVH (Linear Bounding Volume Hierarchy).
//!
//! Uses Morton codes (Z-order curves) for spatial sorting, then builds a BVH
//! from the sorted order. The Morton encoding is the only D-dependent code;
//! the hierarchy is dimension-agnostic. Falls back to brute-force for <50 bodies.
//!
//! # Determinism
//! Morton codes use integer quantization, avoiding floating-point heuristics.
//! Pair list sorted by (BodyHandle, BodyHandle) for identical resolution order.

use crate::body::{BodyHandle, BodyType, RigidBody};
use nalgebra::SVector;

use crate::support_map::{TransformedShape, support_aabb};

/// Pair of body handles that may be colliding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BroadphasePair(pub BodyHandle, pub BodyHandle);

/// Axis-aligned bounding box in D dimensions.
#[derive(Clone, Debug)]
pub struct Aabb<const D: usize> {
    pub min: SVector<f64, D>,
    pub max: SVector<f64, D>,
}

impl<const D: usize> Aabb<D> {
    pub fn empty() -> Self {
        Self {
            min: SVector::from_element(f64::MAX),
            max: SVector::from_element(f64::MIN),
        }
    }
    pub fn union(&self, other: &Self) -> Self {
        let mut min = self.min;
        let mut max = self.max;
        for i in 0..D {
            if other.min[i] < min[i] {
                min[i] = other.min[i];
            }
            if other.max[i] > max[i] {
                max[i] = other.max[i];
            }
        }
        Self { min, max }
    }
    pub fn overlaps(&self, other: &Self) -> bool {
        for i in 0..D {
            if self.max[i] < other.min[i] || self.min[i] > other.max[i] {
                return false;
            }
        }
        true
    }
    pub fn from_sphere(center: &SVector<f64, D>, radius: f64) -> Self {
        Self {
            min: center - SVector::from_element(radius),
            max: center + SVector::from_element(radius),
        }
    }
    /// Exact world-axis bounds for a bounded convex rigid body.
    pub fn from_body(body: &RigidBody<D>) -> Self {
        let map = TransformedShape::new(body.collider.as_ref(), &body.transform);
        let (min, max) = support_aabb(&map);
        Self { min, max }
    }
}

// Morton codes
fn quantize(value: f64, world_min: f64, world_max: f64, bits: usize) -> u32 {
    let range = world_max - world_min;
    if range <= 0.0 {
        return 0;
    }
    let max_val = ((1u64 << bits) - 1) as f64;
    let normalized = ((value - world_min) / range).clamp(0.0, 1.0);
    (normalized * max_val) as u32
}

/// Encode a D-dimensional position into a 64-bit Morton code.
pub fn morton_encode<const D: usize>(
    position: &SVector<f64, D>,
    world_min: &SVector<f64, D>,
    world_max: &SVector<f64, D>,
) -> u64 {
    let bits = 64 / D;
    let mut code: u64 = 0;
    for bit in 0..bits {
        for axis in 0..D {
            let q = quantize(position[axis], world_min[axis], world_max[axis], bits);
            code |= (((q >> bit) & 1) as u64) << (bit * D + axis);
        }
    }
    code
}

/// Extract top prefix_bits from a Morton code (for spatial authority zones).
pub fn morton_prefix(code: u64, prefix_bits: usize) -> u64 {
    if prefix_bits >= 64 {
        code
    } else {
        code >> (64 - prefix_bits)
    }
}

// LBVH
const BRUTE_FORCE_THRESHOLD: usize = 50;
const LEAF_BIT: u32 = 0x8000_0000;
fn is_leaf(idx: u32) -> bool {
    idx & LEAF_BIT != 0
}
fn leaf_index(idx: u32) -> usize {
    (idx & !LEAF_BIT) as usize
}

#[derive(Clone, Debug)]
struct BvhNode<const D: usize> {
    aabb: Aabb<D>,
    left: u32,
    right: u32,
}

/// Linear Bounding Volume Hierarchy.
pub struct Lbvh<const D: usize> {
    sorted_indices: Vec<usize>,
    leaf_aabbs: Vec<Aabb<D>>,
    nodes: Vec<BvhNode<D>>,
}

impl<const D: usize> Lbvh<D> {
    pub fn build(bodies: &[RigidBody<D>]) -> Self {
        let aabbs = bodies.iter().map(Aabb::from_body).collect::<Vec<_>>();
        let (sorted_indices, leaf_aabbs, nodes) = build_morton_index(&aabbs);
        Self {
            sorted_indices,
            leaf_aabbs,
            nodes,
        }
    }

    fn find_split(codes: &[u64], first: usize, last: usize) -> usize {
        if codes[first] == codes[last] {
            return (first + last) / 2;
        }
        let cp = (codes[first] ^ codes[last]).leading_zeros();
        let mut split = first;
        let mut step = last - first;
        loop {
            step = step.div_ceil(2);
            let ns = split + step;
            if ns < last && (codes[first] ^ codes[ns]).leading_zeros() > cp {
                split = ns;
            }
            if step <= 1 {
                break;
            }
        }
        split
    }

    fn build_nodes(codes: &[u64], aabbs: &[Aabb<D>]) -> Vec<BvhNode<D>> {
        let mut nodes = Vec::with_capacity(codes.len() - 1);
        Self::build_rec(codes, aabbs, 0, codes.len() - 1, &mut nodes);
        nodes
    }

    fn build_rec(
        codes: &[u64],
        aabbs: &[Aabb<D>],
        first: usize,
        last: usize,
        nodes: &mut Vec<BvhNode<D>>,
    ) -> u32 {
        if first == last {
            return LEAF_BIT | (first as u32);
        }
        let split = Self::find_split(codes, first, last);
        let left = Self::build_rec(codes, aabbs, first, split, nodes);
        let right = Self::build_rec(codes, aabbs, split + 1, last, nodes);
        let la = if is_leaf(left) {
            aabbs[leaf_index(left)].clone()
        } else {
            nodes[left as usize].aabb.clone()
        };
        let ra = if is_leaf(right) {
            aabbs[leaf_index(right)].clone()
        } else {
            nodes[right as usize].aabb.clone()
        };
        let idx = nodes.len() as u32;
        nodes.push(BvhNode {
            aabb: la.union(&ra),
            left,
            right,
        });
        idx
    }

    pub fn query_pairs(&self, bodies: &[RigidBody<D>]) -> Vec<BroadphasePair> {
        let n = self.sorted_indices.len();
        if n <= 1 {
            return Vec::new();
        }
        let mut pairs = Vec::new();
        let root = (self.nodes.len() - 1) as u32;
        for li in 0..n {
            let la = &self.leaf_aabbs[li];
            let lt = bodies[self.sorted_indices[li]].body_type;
            let mut stack = arrayvec::ArrayVec::<u32, 64>::new();
            stack.push(root);
            while let Some(ni) = stack.pop() {
                if is_leaf(ni) {
                    let oi = leaf_index(ni);
                    if oi > li {
                        let ot = bodies[self.sorted_indices[oi]].body_type;
                        if lt == BodyType::Static && ot == BodyType::Static {
                            continue;
                        }
                        if !should_collide(
                            &bodies[self.sorted_indices[li]],
                            &bodies[self.sorted_indices[oi]],
                        ) {
                            continue;
                        }
                        if la.overlaps(&self.leaf_aabbs[oi]) {
                            let (ha, hb) = (
                                bodies[self.sorted_indices[li]].handle,
                                bodies[self.sorted_indices[oi]].handle,
                            );
                            if ha < hb {
                                pairs.push(BroadphasePair(ha, hb));
                            } else {
                                pairs.push(BroadphasePair(hb, ha));
                            }
                        }
                    }
                    continue;
                }
                let node = &self.nodes[ni as usize];
                if la.overlaps(&node.aabb) {
                    stack.push(node.left);
                    stack.push(node.right);
                }
            }
        }
        pairs.sort_unstable_by_key(|p| (p.0, p.1));
        pairs.dedup();
        pairs
    }
}

/// Build a deterministic Morton-sorted AABB hierarchy for an arbitrary
/// leaf population. The returned sorted indices refer to the caller's AABB
/// slice; the tree's leaf AABBs are stored in Morton order.
fn build_morton_index<const D: usize>(
    aabbs: &[Aabb<D>],
) -> (Vec<usize>, Vec<Aabb<D>>, Vec<BvhNode<D>>) {
    let n = aabbs.len();
    let mut sorted_indices = (0..n).collect::<Vec<_>>();
    if n <= 1 {
        return (
            sorted_indices,
            aabbs.to_vec(),
            Vec::new(),
        );
    }

    let mut world_min = SVector::<f64, D>::from_element(f64::MAX);
    let mut world_max = SVector::<f64, D>::from_element(f64::MIN);
    for aabb in aabbs {
        for axis in 0..D {
            world_min[axis] = world_min[axis].min(aabb.min[axis]);
            world_max[axis] = world_max[axis].max(aabb.max[axis]);
        }
    }
    for axis in 0..D {
        if world_max[axis] - world_min[axis] < 1e-6 {
            world_min[axis] -= 1.0;
            world_max[axis] += 1.0;
        }
    }

    let morton_codes = aabbs
        .iter()
        .map(|aabb| {
            let center = (aabb.min + aabb.max) * 0.5;
            morton_encode::<D>(&center, &world_min, &world_max)
        })
        .collect::<Vec<_>>();
    sorted_indices.sort_by_key(|&index| morton_codes[index]);
    let sorted_aabbs = sorted_indices
        .iter()
        .map(|&index| aabbs[index].clone())
        .collect::<Vec<_>>();
    let sorted_codes = sorted_indices
        .iter()
        .map(|&index| morton_codes[index])
        .collect::<Vec<_>>();
    let nodes = Lbvh::<D>::build_nodes(&sorted_codes, &sorted_aabbs);
    (sorted_indices, sorted_aabbs, nodes)
}

/// Check if two bodies should collide based on collision group/mask filters.
/// Both bodies must accept each other: `(a.group & b.mask != 0) && (b.group & a.mask != 0)`.
#[inline]
fn should_collide<const D: usize>(a: &RigidBody<D>, b: &RigidBody<D>) -> bool {
    (a.collision_group & b.collision_mask != 0) && (b.collision_group & a.collision_mask != 0)
}

/// Find all potentially colliding pairs. Uses LBVH for ≥50 bodies, brute-force otherwise.
pub fn find_pairs<const D: usize>(bodies: &[RigidBody<D>]) -> Vec<BroadphasePair> {
    if bodies.len() < BRUTE_FORCE_THRESHOLD {
        find_pairs_brute(bodies)
    } else {
        Lbvh::build(bodies).query_pairs(bodies)
    }
}

// ---------------------------------------------------------------------------
// Incremental broadphase: cache static/kinematic AABBs across frames
// ---------------------------------------------------------------------------

/// Pre-computed broadphase descriptor for a single body (AABB + sphere + filter).
///
/// Decoupled from `RigidBody<D>` so it can be cached without holding references.
/// Stores a tight orientation-aware AABB for pair rejection and a bounding sphere
/// for diagnostics and future coarse hierarchy construction.
#[derive(Clone, Debug)]
pub struct BpEntry<const D: usize> {
    pub handle: BodyHandle,
    pub aabb: Aabb<D>,
    /// World-space bounding sphere center.
    pub sphere_center: SVector<f64, D>,
    /// Bounding sphere radius.
    pub sphere_radius: f64,
    pub body_type: BodyType,
    pub collision_group: u32,
    pub collision_mask: u32,
}

impl<const D: usize> BpEntry<D> {
    pub fn from_body(body: &RigidBody<D>) -> Self {
        let (c_local, r) = body.collider.bounding_sphere();
        let c_world = body.transform.transform_point(&c_local).0;
        BpEntry {
            handle: body.handle,
            aabb: Aabb::from_body(body),
            sphere_center: c_world,
            sphere_radius: r,
            body_type: body.body_type,
            collision_group: body.collision_group,
            collision_mask: body.collision_mask,
        }
    }
}

/// Cached broadphase state for Static and Kinematic bodies.
///
/// Rebuilt only when a static/kinematic body is added or removed (`dirty` flag).
/// This avoids including immovable geometry in the per-frame LBVH rebuild,
/// which is O(n log n) in the total body count.
///
/// For scenes with many static bodies (terrain, walls) and few dynamic bodies,
/// this typically halves or better the broadphase cost.
pub struct StaticBroadphase<const D: usize> {
    entries: Vec<BpEntry<D>>,
    sorted_indices: Vec<usize>,
    leaf_aabbs: Vec<Aabb<D>>,
    nodes: Vec<BvhNode<D>>,
}

/// Small static populations use a direct scan to avoid tree traversal overhead.
/// Larger populations use the cached Morton hierarchy.
const STATIC_LINEAR_THRESHOLD: usize = 16;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct StaticQueryStats {
    node_visits: usize,
    leaf_tests: usize,
    linear_queries: usize,
}

impl<const D: usize> StaticBroadphase<D> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            sorted_indices: Vec::new(),
            leaf_aabbs: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// Rebuild cached static/kinematic bounds and their Morton hierarchy.
    ///
    /// Call this when `static_tree_dirty` is set on `PhysicsWorld`.
    pub fn rebuild(&mut self, bodies: &[RigidBody<D>]) {
        self.entries = bodies
            .iter()
            .filter(|body| {
                body.body_type == BodyType::Static || body.body_type == BodyType::Kinematic
            })
            .map(BpEntry::from_body)
            .collect();
        let aabbs = self
            .entries
            .iter()
            .map(|entry| entry.aabb.clone())
            .collect::<Vec<_>>();
        let (sorted_indices, leaf_aabbs, nodes) = build_morton_index(&aabbs);
        self.sorted_indices = sorted_indices;
        self.leaf_aabbs = leaf_aabbs;
        self.nodes = nodes;
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Query the cached static population against one dynamic AABB.
    ///
    /// Returns canonical pairs and narrow instrumentation counters. The
    /// counters are intentionally query-local so benchmark adapters can
    /// measure tree pruning without relying on wall-clock timing alone.
    fn query_against(
        &self,
        dyn_entry: &BpEntry<D>,
    ) -> (Vec<BroadphasePair>, StaticQueryStats) {
        let mut pairs = Vec::new();
        let mut stats = StaticQueryStats::default();

        if self.entries.len() <= STATIC_LINEAR_THRESHOLD || self.nodes.is_empty() {
            stats.linear_queries = 1;
            for entry in &self.entries {
                stats.leaf_tests += 1;
                append_static_pair(dyn_entry, entry, &mut pairs);
            }
            return (pairs, stats);
        }

        let root = (self.nodes.len() - 1) as u32;
        let mut stack = Vec::with_capacity(64);
        stack.push(root);
        while let Some(node_index) = stack.pop() {
            if is_leaf(node_index) {
                let leaf_index = leaf_index(node_index);
                stats.leaf_tests += 1;
                let entry = &self.entries[self.sorted_indices[leaf_index]];
                if dyn_entry.aabb.overlaps(&self.leaf_aabbs[leaf_index]) {
                    append_static_pair(dyn_entry, entry, &mut pairs);
                }
                continue;
            }

            stats.node_visits += 1;
            let node = &self.nodes[node_index as usize];
            if dyn_entry.aabb.overlaps(&node.aabb) {
                stack.push(node.right);
                stack.push(node.left);
            }
        }
        (pairs, stats)
    }
}

fn append_static_pair<const D: usize>(
    dynamic: &BpEntry<D>,
    static_entry: &BpEntry<D>,
    pairs: &mut Vec<BroadphasePair>,
) {
    let group_ok = (dynamic.collision_group & static_entry.collision_mask != 0)
        && (static_entry.collision_group & dynamic.collision_mask != 0);
    if group_ok && dynamic.aabb.overlaps(&static_entry.aabb) {
        let (first, second) = (dynamic.handle, static_entry.handle);
        pairs.push(if first < second {
            BroadphasePair(first, second)
        } else {
            BroadphasePair(second, first)
        });
    }
}

impl<const D: usize> Default for StaticBroadphase<D> {
    fn default() -> Self {
        Self::new()
    }
}

/// Instrumentation for the incremental broadphase. These structural counts
/// allow deterministic algorithm comparisons without depending on timer noise.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IncrementalBroadphaseStats {
    pub dynamic_entries: usize,
    pub static_entries: usize,
    pub dynamic_pair_tests: usize,
    pub static_bvh_node_visits: usize,
    pub static_leaf_tests: usize,
    pub static_linear_queries: usize,
    pub candidate_pairs: usize,
    pub used_full_lbvh_fallback: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncrementalBroadphaseResult {
    pub pairs: Vec<BroadphasePair>,
    pub stats: IncrementalBroadphaseStats,
}

/// Find all potentially colliding pairs using the incremental static cache.
///
/// Dynamic-dynamic pairs remain brute force while the dynamic population is
/// below `BRUTE_FORCE_THRESHOLD`; larger dynamic populations use the full
/// LBVH. Dynamic-static queries use a cached Morton BVH above
/// `STATIC_LINEAR_THRESHOLD`, otherwise a direct scan for small caches.
pub fn find_pairs_incremental<const D: usize>(
    bodies: &[RigidBody<D>],
    static_bp: &StaticBroadphase<D>,
) -> Vec<BroadphasePair> {
    find_pairs_incremental_with_stats(bodies, static_bp).pairs
}

/// Instrumented variant of `find_pairs_incremental`.
pub fn find_pairs_incremental_with_stats<const D: usize>(
    bodies: &[RigidBody<D>],
    static_bp: &StaticBroadphase<D>,
) -> IncrementalBroadphaseResult {
    let mut stats = IncrementalBroadphaseStats {
        static_entries: static_bp.len(),
        ..IncrementalBroadphaseStats::default()
    };
    if bodies.is_empty() {
        return IncrementalBroadphaseResult {
            pairs: Vec::new(),
            stats,
        };
    }
    if static_bp.is_empty() {
        let pairs = find_pairs(bodies);
        stats.candidate_pairs = pairs.len();
        stats.used_full_lbvh_fallback = bodies.len() >= BRUTE_FORCE_THRESHOLD;
        return IncrementalBroadphaseResult { pairs, stats };
    }

    let dynamic = bodies
        .iter()
        .filter(|body| body.body_type == BodyType::Dynamic)
        .map(BpEntry::from_body)
        .collect::<Vec<_>>();
    stats.dynamic_entries = dynamic.len();

    if dynamic.len() >= BRUTE_FORCE_THRESHOLD {
        let pairs = find_pairs(bodies);
        stats.candidate_pairs = pairs.len();
        stats.used_full_lbvh_fallback = true;
        return IncrementalBroadphaseResult { pairs, stats };
    }

    let mut pairs = Vec::new();
    for first in 0..dynamic.len() {
        for second in (first + 1)..dynamic.len() {
            stats.dynamic_pair_tests += 1;
            let a = &dynamic[first];
            let b = &dynamic[second];
            let group_ok = (a.collision_group & b.collision_mask != 0)
                && (b.collision_group & a.collision_mask != 0);
            if group_ok && a.aabb.overlaps(&b.aabb) {
                let (left, right) = (a.handle, b.handle);
                pairs.push(if left < right {
                    BroadphasePair(left, right)
                } else {
                    BroadphasePair(right, left)
                });
            }
        }
    }

    for dynamic_entry in &dynamic {
        let (static_pairs, query_stats) = static_bp.query_against(dynamic_entry);
        stats.static_bvh_node_visits += query_stats.node_visits;
        stats.static_leaf_tests += query_stats.leaf_tests;
        stats.static_linear_queries += query_stats.linear_queries;
        pairs.extend(static_pairs);
    }

    pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
    pairs.dedup();
    stats.candidate_pairs = pairs.len();
    IncrementalBroadphaseResult { pairs, stats }
}

fn find_pairs_brute<const D: usize>(bodies: &[RigidBody<D>]) -> Vec<BroadphasePair> {
    let mut pairs = Vec::new();
    for i in 0..bodies.len() {
        let ti = bodies[i].body_type;
        for j in (i + 1)..bodies.len() {
            if ti == BodyType::Static && bodies[j].body_type == BodyType::Static {
                continue;
            }
            if !should_collide(&bodies[i], &bodies[j]) {
                continue;
            }
            let aabb_i = Aabb::from_body(&bodies[i]);
            let aabb_j = Aabb::from_body(&bodies[j]);
            if aabb_i.overlaps(&aabb_j) {
                pairs.push(BroadphasePair(bodies[i].handle, bodies[j].handle));
            }
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::RigidBody;
    use symtropy_math::{Point, Sphere};

    #[test]
    fn overlapping_pair_detected() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
        ];
        assert_eq!(find_pairs(&bodies).len(), 1);
    }

    #[test]
    fn separated_pair_not_detected() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([5.0, 0.0, 0.0]), 1.0, 1.0),
        ];
        assert!(find_pairs(&bodies).is_empty());
    }

    #[test]
    fn static_static_ignored() {
        let bodies = vec![
            RigidBody::<3>::static_body(BodyHandle(0), Point::origin(), Box::new(Sphere::unit())),
            RigidBody::<3>::static_body(
                BodyHandle(1),
                Point::new([0.5, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ),
        ];
        assert!(find_pairs(&bodies).is_empty());
    }

    #[test]
    fn static_dynamic_detected() {
        let bodies = vec![
            RigidBody::<3>::static_body(BodyHandle(0), Point::origin(), Box::new(Sphere::unit())),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
        ];
        assert_eq!(find_pairs(&bodies).len(), 1);
    }

    #[test]
    fn multiple_pairs() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(2), Point::new([10.0, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(3), Point::new([1.0, 1.0, 0.0]), 1.0, 1.0),
        ];
        assert!(find_pairs(&bodies).len() >= 2);
    }

    #[test]
    fn morton_encode_2d_origin() {
        let pos = SVector::<f64, 2>::from([0.0, 0.0]);
        let min = SVector::<f64, 2>::from([0.0, 0.0]);
        let max = SVector::<f64, 2>::from([100.0, 100.0]);
        assert_eq!(morton_encode::<2>(&pos, &min, &max), 0);
    }

    #[test]
    fn morton_locality() {
        let min = SVector::<f64, 3>::from([0.0; 3]);
        let max = SVector::<f64, 3>::from([100.0; 3]);
        let a = morton_encode::<3>(&SVector::from([10.0, 10.0, 10.0]), &min, &max);
        let b = morton_encode::<3>(&SVector::from([11.0, 10.0, 10.0]), &min, &max);
        let far = morton_encode::<3>(&SVector::from([90.0, 90.0, 90.0]), &min, &max);
        assert!((a as i64 - b as i64).unsigned_abs() < (a as i64 - far as i64).unsigned_abs());
    }

    #[test]
    fn lbvh_matches_brute_force() {
        let mut bodies = Vec::new();
        for i in 0..(BRUTE_FORCE_THRESHOLD + 10) {
            let x = (i as f64) * 0.5;
            bodies.push(RigidBody::<2>::dynamic_sphere(
                BodyHandle(i),
                Point::new([x, 0.0]),
                1.0,
                1.0,
            ));
        }
        let mut brute = find_pairs_brute(&bodies);
        brute.sort_by_key(|p| (p.0, p.1));
        let lbvh = find_pairs(&bodies);
        for pair in &brute {
            assert!(lbvh.contains(pair), "LBVH missing pair {:?}", pair);
        }
    }

    #[test]
    fn lbvh_deterministic() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(2), Point::new([0.5, 0.5, 0.5]), 0.8, 1.0),
        ];
        assert_eq!(find_pairs(&bodies), find_pairs(&bodies));
    }

    #[test]
    fn aabb_overlap() {
        let a = Aabb::<2> {
            min: SVector::from([0.0, 0.0]),
            max: SVector::from([2.0, 2.0]),
        };
        let b = Aabb::<2> {
            min: SVector::from([1.0, 1.0]),
            max: SVector::from([3.0, 3.0]),
        };
        let c = Aabb::<2> {
            min: SVector::from([5.0, 5.0]),
            max: SVector::from([6.0, 6.0]),
        };
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    // --- Incremental broadphase tests ---

    #[test]
    fn incremental_dynamic_static_detected() {
        let bodies = vec![
            RigidBody::<3>::static_body(BodyHandle(0), Point::origin(), Box::new(Sphere::unit())),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
        ];
        let mut static_bp = super::StaticBroadphase::new();
        static_bp.rebuild(&bodies);

        let pairs = super::find_pairs_incremental(&bodies, &static_bp);
        assert_eq!(pairs.len(), 1);
        let p = pairs[0];
        assert!(
            (p.0 == BodyHandle(0) && p.1 == BodyHandle(1))
                || (p.0 == BodyHandle(1) && p.1 == BodyHandle(0))
        );
    }

    #[test]
    fn incremental_static_static_not_produced() {
        let bodies = vec![
            RigidBody::<3>::static_body(BodyHandle(0), Point::origin(), Box::new(Sphere::unit())),
            RigidBody::<3>::static_body(
                BodyHandle(1),
                Point::new([0.5, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ),
        ];
        let mut static_bp = super::StaticBroadphase::new();
        static_bp.rebuild(&bodies);

        let pairs = super::find_pairs_incremental(&bodies, &static_bp);
        assert!(pairs.is_empty(), "static–static must never collide");
    }

    #[test]
    fn incremental_matches_standard_find_pairs() {
        let bodies = vec![
            RigidBody::<3>::static_body(
                BodyHandle(0),
                Point::new([0.0, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(2), Point::new([0.0, 1.5, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(3), Point::new([10.0, 0.0, 0.0]), 1.0, 1.0),
        ];
        let mut static_bp = super::StaticBroadphase::new();
        static_bp.rebuild(&bodies);

        let mut incremental = super::find_pairs_incremental(&bodies, &static_bp);
        let mut standard = find_pairs(&bodies);
        incremental.sort_by_key(|p| (p.0, p.1));
        standard.sort_by_key(|p| (p.0, p.1));
        assert_eq!(
            incremental, standard,
            "incremental must match standard broadphase"
        );
    }

    #[test]
    fn incremental_no_static_degrades_to_standard() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 0.0, 0.0]), 1.0, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
        ];
        let empty_bp = super::StaticBroadphase::<3>::new(); // no rebuild
        let pairs = super::find_pairs_incremental(&bodies, &empty_bp);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn static_broadphase_rebuild_updates_cache() {
        let bodies = vec![
            RigidBody::<3>::static_body(BodyHandle(0), Point::origin(), Box::new(Sphere::unit())),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.5, 0.0, 0.0]), 1.0, 1.0),
        ];
        let mut static_bp = super::StaticBroadphase::<3>::new();
        assert_eq!(static_bp.len(), 0);

        static_bp.rebuild(&bodies);
        assert_eq!(static_bp.len(), 1, "should cache the one static body");
    }

    #[test]
    fn static_morton_tree_matches_brute_force_and_prunes_leaf_tests() {
        let mut bodies = Vec::new();
        for index in 0..256 {
            bodies.push(RigidBody::<3>::static_body(
                BodyHandle(index),
                Point::new([index as f64 * 4.0, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ));
        }
        for (offset, x) in [0.0, 200.0, 500.0, 800.0].into_iter().enumerate() {
            bodies.push(RigidBody::<3>::dynamic_sphere(
                BodyHandle(1_000 + offset),
                Point::new([x, 0.0, 0.0]),
                1.0,
                1.0,
            ));
        }

        let mut static_bp = StaticBroadphase::new();
        static_bp.rebuild(&bodies);
        let result = find_pairs_incremental_with_stats(&bodies, &static_bp);
        let oracle = find_pairs_brute(&bodies);

        assert_eq!(result.pairs, oracle);
        assert_eq!(result.stats.dynamic_entries, 4);
        assert_eq!(result.stats.static_entries, 256);
        assert!(result.stats.static_bvh_node_visits > 0);
        assert!(
            result.stats.static_leaf_tests < result.stats.dynamic_entries * result.stats.static_entries,
            "tree should test fewer leaves than the linear scan"
        );
        assert_eq!(result.stats.candidate_pairs, oracle.len());
        assert_eq!(result.stats.static_linear_queries, 0);
    }

    #[test]
    fn static_morton_tree_preserves_bidirectional_collision_filters() {
        let mut static_body = RigidBody::<3>::static_body(
            BodyHandle(1),
            Point::origin(),
            Box::new(Sphere::unit()),
        );
        static_body.collision_group = 0b0001;
        static_body.collision_mask = 0b0010;

        let mut dynamic_body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(2),
            Point::new([1.0, 0.0, 0.0]),
            1.0,
            1.0,
        );
        dynamic_body.collision_group = 0b0010;
        dynamic_body.collision_mask = 0b0001;

        let mut bodies = vec![static_body, dynamic_body];
        for index in 0..32 {
            bodies.push(RigidBody::<3>::static_body(
                BodyHandle(10 + index),
                Point::new([100.0 + index as f64 * 4.0, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ));
        }

        let mut static_bp = StaticBroadphase::new();
        static_bp.rebuild(&bodies);
        let result = find_pairs_incremental_with_stats(&bodies, &static_bp);
        assert_eq!(result.pairs, find_pairs_brute(&bodies));
        assert_eq!(result.pairs, vec![BroadphasePair(BodyHandle(1), BodyHandle(2))]);

        // If either side rejects the other, the overlapping AABBs must not
        // produce a candidate.
        bodies[1].collision_mask = 0;
        static_bp.rebuild(&bodies);
        let filtered = find_pairs_incremental_with_stats(&bodies, &static_bp);
        assert!(filtered.pairs.is_empty());
    }

    #[test]
    fn small_static_population_keeps_the_linear_fast_path() {
        let mut bodies = Vec::new();
        for index in 0..8 {
            bodies.push(RigidBody::<3>::static_body(
                BodyHandle(index),
                Point::new([index as f64 * 4.0, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ));
        }
        bodies.push(RigidBody::<3>::dynamic_sphere(
            BodyHandle(100),
            Point::origin(),
            1.0,
            1.0,
        ));

        let mut static_bp = StaticBroadphase::new();
        static_bp.rebuild(&bodies);
        let result = find_pairs_incremental_with_stats(&bodies, &static_bp);
        assert_eq!(result.pairs, find_pairs_brute(&bodies));
        assert_eq!(result.stats.static_linear_queries, 1);
        assert_eq!(result.stats.static_bvh_node_visits, 0);
        assert_eq!(result.stats.static_leaf_tests, 8);
    }

    #[test]
    fn high_dynamic_population_keeps_the_full_lbvh_fallback() {
        let mut bodies = Vec::new();
        for index in 0..64 {
            bodies.push(RigidBody::<3>::static_body(
                BodyHandle(index),
                Point::new([index as f64 * 10.0, 0.0, 0.0]),
                Box::new(Sphere::unit()),
            ));
        }
        for index in 0..BRUTE_FORCE_THRESHOLD {
            bodies.push(RigidBody::<3>::dynamic_sphere(
                BodyHandle(1_000 + index),
                Point::new([index as f64 * 3.0, 0.0, 0.0]),
                1.0,
                1.0,
            ));
        }

        let mut static_bp = StaticBroadphase::new();
        static_bp.rebuild(&bodies);
        let result = find_pairs_incremental_with_stats(&bodies, &static_bp);
        assert!(result.stats.used_full_lbvh_fallback);
        assert_eq!(result.pairs, find_pairs(&bodies));
    }

    #[test]
    fn rotated_box_aabb_uses_orientation() {
        use std::f64::consts::FRAC_PI_2;
        use symtropy_math::{Bivector, HyperBox, Rotor, Transform};

        let body = RigidBody::new(
            BodyHandle(0),
            BodyType::Dynamic,
            Transform {
                translation: Point::new([1.0, 2.0, 0.0]),
                rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
            },
            Box::new(HyperBox::<3>::new([2.0, 0.5, 0.25])),
            1.0,
            SVector::from_element(1.0),
        );

        let aabb = Aabb::from_body(&body);
        assert!((aabb.min[0] - 0.5).abs() < 1e-10);
        assert!((aabb.max[0] - 1.5).abs() < 1e-10);
        assert!((aabb.min[1] - 0.0).abs() < 1e-10);
        assert!((aabb.max[1] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn rotated_thin_boxes_are_rejected_by_tight_aabbs() {
        use std::f64::consts::FRAC_PI_2;
        use symtropy_math::{Bivector, HyperBox, Rotor, Transform};

        let make = |handle, x| {
            RigidBody::new(
                BodyHandle(handle),
                BodyType::Dynamic,
                Transform {
                    translation: Point::new([x, 0.0, 0.0]),
                    rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
                },
                Box::new(HyperBox::<3>::new([2.0, 0.1, 0.1])),
                1.0,
                SVector::from_element(1.0),
            )
        };

        // Bounding spheres overlap heavily (radius ~= 2), but after rotation
        // each box is only 0.2 units wide on X.
        let bodies = vec![make(0, 0.0), make(1, 1.0)];
        assert!(find_pairs(&bodies).is_empty());
    }
}
