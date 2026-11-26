//! Tree-backed qualification adapter for replayable swept motion envelopes.
//!
//! The adapter builds an incremental-AABB-tree reference from conservative
//! motion envelopes, queries the tree for supported-supported pairs, and adds
//! all fail-closed fallback pairs from the O(n²) swept-pair oracle.
//!
//! This first version intentionally rebuilds a reference tree per call. Its
//! purpose is to cross-check tree output and preserve a reproducible receipt;
//! it is not yet an incremental runtime broadphase or a performance claim.
//!
//! The tree implementation is pinned to the exact #7524 revision so the
//! balancing policy and its behavior are part of the qualification contract.

use directional_motion_envelopes::{Aabb as MotionAabb, ColliderIdentity};
use incremental_aabb_tree_reference::{
    Aabb as TreeAabb, BalancePolicy, IncrementalAabbTree, PairQueryStats, ProxyId, TreeError,
    TreeStats,
};
use std::collections::BTreeMap;
use swept_pair_oracle::{
    MotionEvidence, PairKey, PairSetError, PairSetReceipt, Proxy, build_swept_pair_set,
};

pub const TREE_REFERENCE_REVISION: &str = "fed9ba265485d495d9f6191f280ff826d255e648";
pub const TREE_FAT_MARGIN: f64 = 0.0;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IdentityKey {
    body_slot: u32,
    body_generation: u32,
    collider_id: u64,
    subshape_path: Vec<u32>,
}

impl IdentityKey {
    fn from_identity(identity: &ColliderIdentity) -> Self {
        Self {
            body_slot: identity.body_slot,
            body_generation: identity.body_generation,
            collider_id: identity.collider_id,
            subshape_path: identity.subshape_path.clone(),
        }
    }

    fn to_identity(&self) -> ColliderIdentity {
        ColliderIdentity {
            body_slot: self.body_slot,
            body_generation: self.body_generation,
            collider_id: self.collider_id,
            subshape_path: self.subshape_path.clone(),
        }
    }

}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyIdAssignment {
    pub identity: ColliderIdentity,
    pub proxy_id: ProxyId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyIdRegistrySnapshot {
    pub next_proxy_id: u64,
    /// Sorted by full stable collider identity. Retired identities remain
    /// present so their proxy IDs are never reused for a new generation.
    pub assignments: Vec<ProxyIdAssignment>,
}

/// Collision-free stable identity registry. It uses an ordered map and a
/// monotonic ID allocator rather than compressing identities into a hash.
/// Keep one registry across frames to preserve IDs when proxies enter/leave.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProxyIdRegistry {
    next_proxy_id: u64,
    by_identity: BTreeMap<IdentityKey, ProxyId>,
    by_proxy_id: BTreeMap<ProxyId, IdentityKey>,
}

impl ProxyIdRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_proxy_id: 1,
            by_identity: BTreeMap::new(),
            by_proxy_id: BTreeMap::new(),
        }
    }

    fn ensure(&mut self, key: IdentityKey) -> Result<ProxyId, TreePairSetError> {
        if let Some(proxy_id) = self.by_identity.get(&key) {
            return Ok(*proxy_id);
        }
        // u64::MAX is reserved as the tree's internal min-proxy sentinel.
        if self.next_proxy_id == 0 || self.next_proxy_id == u64::MAX {
            return Err(TreePairSetError::ProxyIdExhausted);
        }
        let proxy_id = ProxyId(self.next_proxy_id);
        self.next_proxy_id = self
            .next_proxy_id
            .checked_add(1)
            .ok_or(TreePairSetError::ProxyIdExhausted)?;
        self.by_identity.insert(key.clone(), proxy_id);
        self.by_proxy_id.insert(proxy_id, key);
        Ok(proxy_id)
    }

    fn assign_batch<'a>(
        &mut self,
        identities: impl IntoIterator<Item = &'a ColliderIdentity>,
    ) -> Result<(), TreePairSetError> {
        let mut keys = identities
            .into_iter()
            .map(IdentityKey::from_identity)
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        for key in keys {
            self.ensure(key)?;
        }
        Ok(())
    }

    fn proxy_id(&self, identity: &ColliderIdentity) -> Result<ProxyId, TreePairSetError> {
        self.by_identity
            .get(&IdentityKey::from_identity(identity))
            .copied()
            .ok_or(TreePairSetError::MissingProxyIdentity)
    }

    #[must_use]
    pub fn snapshot(&self) -> ProxyIdRegistrySnapshot {
        ProxyIdRegistrySnapshot {
            next_proxy_id: self.next_proxy_id,
            assignments: self
                .by_identity
                .iter()
                .map(|(key, proxy_id)| ProxyIdAssignment {
                    identity: key.to_identity(),
                    proxy_id: *proxy_id,
                })
                .collect(),
        }
    }

    pub fn from_snapshot(snapshot: &ProxyIdRegistrySnapshot) -> Result<Self, TreePairSetError> {
        if snapshot.next_proxy_id == 0 || snapshot.next_proxy_id == u64::MAX {
            return Err(TreePairSetError::InvalidRegistrySnapshot);
        }

        let mut registry = Self::new();
        registry.next_proxy_id = snapshot.next_proxy_id;
        let mut previous_key: Option<IdentityKey> = None;
        let mut maximum_id = 0_u64;

        for assignment in &snapshot.assignments {
            assignment
                .identity
                .validate()
                .map_err(|_| TreePairSetError::InvalidRegistrySnapshot)?;
            let key = IdentityKey::from_identity(&assignment.identity);
            if previous_key
                .as_ref()
                .is_some_and(|previous| previous >= &key)
                || assignment.proxy_id.0 == 0
                || assignment.proxy_id.0 == u64::MAX
                || registry.by_proxy_id.contains_key(&assignment.proxy_id)
            {
                return Err(TreePairSetError::InvalidRegistrySnapshot);
            }
            maximum_id = maximum_id.max(assignment.proxy_id.0);
            registry
                .by_identity
                .insert(key.clone(), assignment.proxy_id);
            registry
                .by_proxy_id
                .insert(assignment.proxy_id, key.clone());
            previous_key = Some(key);
        }

        if registry.next_proxy_id <= maximum_id {
            return Err(TreePairSetError::InvalidRegistrySnapshot);
        }
        Ok(registry)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreeBuildConfig {
    pub balance_policy: BalancePolicy,
}

impl Default for TreeBuildConfig {
    fn default() -> Self {
        Self {
            balance_policy: BalancePolicy::HeightRotations,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TreePairSetStats {
    pub input_proxies: usize,
    pub supported_proxies: usize,
    pub unsupported_proxies: usize,
    pub raw_tree_pairs: usize,
    pub filtered_tree_pairs: usize,
    pub fallback_pairs: usize,
    pub final_candidate_pairs: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreePairSetReceipt<const D: usize> {
    pub config: TreeBuildConfig,
    pub tree_revision: String,
    pub registry: ProxyIdRegistrySnapshot,
    /// The independent O(n²) envelope-pair reference remains embedded so a
    /// verifier can compare tree output against the full candidate oracle.
    pub reference: PairSetReceipt<D>,
    pub candidate_pairs: Vec<PairKey>,
    pub fallback_pairs: Vec<PairKey>,
    pub tree_stats: TreeStats,
    pub pair_query_stats: PairQueryStats,
    pub stats: TreePairSetStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreePairSetError {
    PairOracle(PairSetError),
    Tree(TreeError),
    InvalidIdentity,
    InvalidRegistrySnapshot,
    ProxyIdExhausted,
    MissingProxyIdentity,
    CandidateMismatch,
}

impl From<PairSetError> for TreePairSetError {
    fn from(value: PairSetError) -> Self {
        Self::PairOracle(value)
    }
}

impl From<TreeError> for TreePairSetError {
    fn from(value: TreeError) -> Self {
        Self::Tree(value)
    }
}

fn envelope<const D: usize>(proxy: &Proxy<D>) -> Option<MotionAabb<D>> {
    match &proxy.motion {
        MotionEvidence::Envelope(receipt) => Some(receipt.envelope),
        MotionEvidence::SemiImplicitEuler(receipt) => Some(receipt.base.envelope),
        MotionEvidence::Unsupported(_) => None,
    }
}

fn tree_aabb<const D: usize>(bounds: &MotionAabb<D>) -> Result<TreeAabb<D>, TreePairSetError> {
    TreeAabb::new(*bounds.min(), *bounds.max()).map_err(TreePairSetError::Tree)
}

fn same_body(left: &ColliderIdentity, right: &ColliderIdentity) -> bool {
    left.body_slot == right.body_slot && left.body_generation == right.body_generation
}

fn identity_key(identity: &ColliderIdentity) -> IdentityKey {
    IdentityKey::from_identity(identity)
}

fn canonical_pair(left: &ColliderIdentity, right: &ColliderIdentity) -> PairKey {
    if identity_key(left) <= identity_key(right) {
        PairKey {
            left: left.clone(),
            right: right.clone(),
        }
    } else {
        PairKey {
            left: right.clone(),
            right: left.clone(),
        }
    }
}

fn pair_cmp(left: &PairKey, right: &PairKey) -> std::cmp::Ordering {
    identity_key(&left.left)
        .cmp(&identity_key(&right.left))
        .then_with(|| identity_key(&left.right).cmp(&identity_key(&right.right)))
}

/// Build a tree-backed pair population and compare it with the independent
/// O(n²) swept-envelope pair oracle. A nonzero tree margin is deliberately not
/// exposed: certified motion envelopes already include their own guard, and
/// arbitrary extra fattening would distort candidate inflation.
pub fn build_tree_swept_pair_set<const D: usize>(
    proxies: &[Proxy<D>],
    registry: &mut ProxyIdRegistry,
    config: TreeBuildConfig,
) -> Result<TreePairSetReceipt<D>, TreePairSetError> {
    let reference = build_swept_pair_set(proxies)?;
    let mut working_registry = registry.clone();
    working_registry.assign_batch(reference.proxies.iter().map(|proxy| &proxy.identity))?;

    let mut tree =
        IncrementalAabbTree::<D>::new_with_balance_policy(TREE_FAT_MARGIN, config.balance_policy)?;

    let mut metadata = BTreeMap::<ProxyId, (ColliderIdentity, u32, u32)>::new();
    let mut supported = Vec::<(ProxyId, MotionAabb<D>)>::new();
    let mut unsupported_proxies = 0_usize;

    for proxy in &reference.proxies {
        let proxy_id = working_registry.proxy_id(&proxy.identity)?;
        metadata.insert(
            proxy_id,
            (
                proxy.identity.clone(),
                proxy.collision_group,
                proxy.collision_mask,
            ),
        );
        if let Some(bounds) = envelope(proxy) {
            supported.push((proxy_id, bounds));
        } else {
            unsupported_proxies = unsupported_proxies.saturating_add(1);
        }
    }

    // Preserve deterministic tree topology even if a long-lived registry has
    // allocated IDs in an order different from the current identity order.
    supported.sort_by_key(|(proxy_id, _)| *proxy_id);
    for (proxy_id, bounds) in &supported {
        tree.insert(*proxy_id, tree_aabb(bounds)?)?;
    }

    let pair_query = tree.query_pairs();
    let mut tree_candidates = Vec::with_capacity(pair_query.pairs.len());
    for pair in &pair_query.pairs {
        let (left, left_group, left_mask) = metadata
            .get(&pair.0)
            .ok_or(TreePairSetError::MissingProxyIdentity)?;
        let (right, right_group, right_mask) = metadata
            .get(&pair.1)
            .ok_or(TreePairSetError::MissingProxyIdentity)?;

        if same_body(left, right) {
            continue;
        }
        if (left_group & right_mask == 0) || (right_group & left_mask == 0) {
            continue;
        }
        tree_candidates.push(canonical_pair(left, right));
    }

    tree_candidates.sort_by(pair_cmp);
    tree_candidates.dedup();

    let mut fallback_pairs = reference.fallback_pairs.clone();
    fallback_pairs.sort_by(pair_cmp);
    fallback_pairs.dedup();

    let mut candidate_pairs = tree_candidates;
    candidate_pairs.extend(fallback_pairs.iter().cloned());
    candidate_pairs.sort_by(pair_cmp);
    candidate_pairs.dedup();

    if candidate_pairs != reference.candidate_pairs {
        return Err(TreePairSetError::CandidateMismatch);
    }

    let tree_stats = tree.stats();
    let stats = TreePairSetStats {
        input_proxies: reference.proxies.len(),
        supported_proxies: supported.len(),
        unsupported_proxies,
        raw_tree_pairs: pair_query.pairs.len(),
        filtered_tree_pairs: candidate_pairs.len().saturating_sub(fallback_pairs.len()),
        fallback_pairs: fallback_pairs.len(),
        final_candidate_pairs: candidate_pairs.len(),
    };

    let receipt = TreePairSetReceipt {
        config,
        tree_revision: TREE_REFERENCE_REVISION.to_owned(),
        registry: working_registry.snapshot(),
        reference,
        candidate_pairs,
        fallback_pairs,
        tree_stats,
        pair_query_stats: pair_query.stats,
        stats,
    };

    // Commit registry allocations only after the entire tree/reference
    // comparison succeeds.
    *registry = working_registry;
    Ok(receipt)
}

/// Reconstruct the registry and tree from the exact input population and
/// captured registry snapshot. All candidate pairs and work counters must
/// reproduce exactly; otherwise the receipt is rejected.
pub fn verify_tree_swept_pair_set<const D: usize>(
    proxies: &[Proxy<D>],
    receipt: &TreePairSetReceipt<D>,
) -> Result<(), TreePairSetError> {
    if receipt.tree_revision != TREE_REFERENCE_REVISION {
        return Err(TreePairSetError::CandidateMismatch);
    }
    let mut registry = ProxyIdRegistry::from_snapshot(&receipt.registry)?;
    let replay = build_tree_swept_pair_set(proxies, &mut registry, receipt.config)?;
    if replay == *receipt {
        Ok(())
    } else {
        Err(TreePairSetError::CandidateMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use directional_motion_envelopes::{Digest, DisplacementInterval, MotionKind, build_envelope};
    use incremental_aabb_tree_reference::BalancePolicy;
    use swept_pair_oracle::{MotionEvidence, UnsupportedReason};

    fn identity(slot: u32) -> ColliderIdentity {
        ColliderIdentity {
            body_slot: slot,
            body_generation: 1,
            collider_id: u64::from(slot) + 100,
            subshape_path: Vec::new(),
        }
    }

    fn context() -> directional_motion_envelopes::EnvelopeContext {
        directional_motion_envelopes::EnvelopeContext {
            world_revision: 7,
            frame_index: 11,
            horizon_steps: 1,
            integrator: Digest::from_bytes([1; 32]),
            motion_bound_producer: Digest::from_bytes([2; 32]),
            arithmetic_profile: Digest::from_bytes([3; 32]),
        }
    }

    fn proxy<const D: usize>(
        slot: u32,
        min: [f64; D],
        max: [f64; D],
        displacement: [f64; D],
    ) -> Proxy<D> {
        let motion_min = std::array::from_fn(|axis| displacement[axis].min(0.0));
        let motion_max = std::array::from_fn(|axis| displacement[axis].max(0.0));
        let receipt = build_envelope(
            identity(slot),
            context(),
            MotionKind::FixedOrientationTranslation,
            MotionAabb::new(min, max).unwrap(),
            DisplacementInterval {
                min: motion_min,
                max: motion_max,
            },
            0.0,
        )
        .unwrap()
        .receipt
        .unwrap();
        Proxy {
            identity: identity(slot),
            motion: MotionEvidence::Envelope(receipt),
            collision_group: 1,
            collision_mask: u32::MAX,
        }
    }

    #[test]
    fn tree_candidates_match_pair_oracle_and_replay() {
        let proxies = [
            proxy(1, [0.0, 0.0], [1.0, 1.0], [2.0, 0.0]),
            proxy(2, [2.0, 0.0], [3.0, 1.0], [-1.0, 0.0]),
            proxy(3, [10.0, 10.0], [11.0, 11.0], [0.0, 0.0]),
        ];
        let mut registry = ProxyIdRegistry::new();
        let receipt =
            build_tree_swept_pair_set(&proxies, &mut registry, TreeBuildConfig::default()).unwrap();

        assert_eq!(receipt.candidate_pairs, receipt.reference.candidate_pairs);
        assert_eq!(receipt.stats.supported_proxies, 3);
        assert_eq!(receipt.stats.unsupported_proxies, 0);
        assert_eq!(receipt.tree_stats.proxy_count, 3);
        assert_eq!(receipt.tree_stats.maintenance.insert_operations, 3);
        assert!(verify_tree_swept_pair_set(&proxies, &receipt).is_ok());
    }

    #[test]
    fn unsupported_motion_is_added_as_fail_closed_fallback() {
        let mut proxies = [
            proxy(1, [0.0, 0.0], [1.0, 1.0], [0.0, 0.0]),
            proxy(2, [100.0, 100.0], [101.0, 101.0], [0.0, 0.0]),
        ];
        proxies[1].motion = MotionEvidence::Unsupported(UnsupportedReason::Rotation);
        let mut registry = ProxyIdRegistry::new();
        let receipt =
            build_tree_swept_pair_set(&proxies, &mut registry, TreeBuildConfig::default()).unwrap();

        assert_eq!(receipt.stats.supported_proxies, 1);
        assert_eq!(receipt.stats.unsupported_proxies, 1);
        assert_eq!(receipt.stats.fallback_pairs, 1);
        assert_eq!(receipt.candidate_pairs, receipt.reference.candidate_pairs);
        assert!(verify_tree_swept_pair_set(&proxies, &receipt).is_ok());
    }

    #[test]
    fn balanced_and_unbalanced_tree_policies_preserve_pair_population() {
        let proxies = [
            proxy(1, [0.0, 0.0], [1.0, 1.0], [0.0, 0.0]),
            proxy(2, [0.5, 0.5], [1.5, 1.5], [0.0, 0.0]),
            proxy(3, [5.0, 5.0], [6.0, 6.0], [0.0, 0.0]),
            proxy(4, [5.5, 5.5], [6.5, 6.5], [0.0, 0.0]),
        ];
        let mut unbalanced_registry = ProxyIdRegistry::new();
        let unbalanced = build_tree_swept_pair_set(
            &proxies,
            &mut unbalanced_registry,
            TreeBuildConfig {
                balance_policy: BalancePolicy::Disabled,
            },
        )
        .unwrap();
        let mut balanced_registry = ProxyIdRegistry::new();
        let balanced = build_tree_swept_pair_set(
            &proxies,
            &mut balanced_registry,
            TreeBuildConfig {
                balance_policy: BalancePolicy::HeightRotations,
            },
        )
        .unwrap();

        assert_eq!(unbalanced.candidate_pairs, balanced.candidate_pairs);
        assert_eq!(
            unbalanced.candidate_pairs,
            unbalanced.reference.candidate_pairs
        );
        assert_eq!(balanced.candidate_pairs, balanced.reference.candidate_pairs);
        assert_eq!(
            unbalanced.tree_stats.balance_policy,
            BalancePolicy::Disabled
        );
        assert_eq!(
            balanced.tree_stats.balance_policy,
            BalancePolicy::HeightRotations
        );
    }

    #[test]
    fn proxy_ids_remain_stable_when_a_new_lower_identity_arrives() {
        let mut registry = ProxyIdRegistry::new();
        let first = [
            proxy(10, [0.0], [1.0], [0.0]),
            proxy(20, [2.0], [3.0], [0.0]),
        ];
        build_tree_swept_pair_set(&first, &mut registry, TreeBuildConfig::default()).unwrap();
        let old_id = registry.proxy_id(&identity(20)).unwrap();

        let later = [
            proxy(1, [-3.0], [-2.0], [0.0]),
            proxy(10, [0.0], [1.0], [0.0]),
            proxy(20, [2.0], [3.0], [0.0]),
        ];
        let mut shuffled = later.clone();
        shuffled.reverse();
        let receipt =
            build_tree_swept_pair_set(&shuffled, &mut registry, TreeBuildConfig::default())
                .unwrap();

        assert_eq!(registry.proxy_id(&identity(20)).unwrap(), old_id);
        assert!(verify_tree_swept_pair_set(&shuffled, &receipt).is_ok());
    }

    #[test]
    fn tampered_registry_and_tree_policy_invalidate_receipt() {
        let proxies = [proxy(1, [0.0], [1.0], [0.0]), proxy(2, [3.0], [4.0], [0.0])];
        let mut registry = ProxyIdRegistry::new();
        let receipt =
            build_tree_swept_pair_set(&proxies, &mut registry, TreeBuildConfig::default()).unwrap();

        let mut bad_registry = receipt.clone();
        bad_registry.registry.next_proxy_id = 1;
        assert!(verify_tree_swept_pair_set(&proxies, &bad_registry).is_err());

        let mut bad_revision = receipt.clone();
        bad_revision.tree_revision = "changed".to_owned();
        assert!(verify_tree_swept_pair_set(&proxies, &bad_revision).is_err());
    }

    fn generated<const D: usize>(count: u32) -> Vec<Proxy<D>> {
        let mut state = 0x9137_4f21_8a2d_6c5b_u64;
        (0..count)
            .map(|slot| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let x = ((state >> 11) as f64 / ((1_u64 << 53) as f64)) * 30.0 - 15.0;
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let extent = 0.2 + ((state >> 11) as f64 / ((1_u64 << 53) as f64)) * 1.5;
                let min = std::array::from_fn(|axis| x + axis as f64 * 2.0);
                let max = std::array::from_fn(|axis| min[axis] + extent);
                let displacement = std::array::from_fn(|axis| {
                    if axis % 2 == 0 {
                        extent * 0.5
                    } else {
                        -extent * 0.25
                    }
                });
                proxy(slot + 1, min, max, displacement)
            })
            .collect()
    }

    fn assert_generated_tree_recall<const D: usize>() {
        let proxies = generated::<D>(64);
        let mut registry = ProxyIdRegistry::new();
        let receipt =
            build_tree_swept_pair_set(&proxies, &mut registry, TreeBuildConfig::default()).unwrap();
        assert_eq!(receipt.candidate_pairs, receipt.reference.candidate_pairs);
        assert!(verify_tree_swept_pair_set(&proxies, &receipt).is_ok());
    }

    #[test]
    fn generated_2d_tree_matches_pair_oracle() {
        assert_generated_tree_recall::<2>();
    }

    #[test]
    fn generated_3d_tree_matches_pair_oracle() {
        assert_generated_tree_recall::<3>();
    }

    #[test]
    fn generated_4d_tree_matches_pair_oracle() {
        assert_generated_tree_recall::<4>();
    }
}
