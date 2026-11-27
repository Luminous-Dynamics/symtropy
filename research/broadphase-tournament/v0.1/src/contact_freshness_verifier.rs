//! Independent reference verifier for contact-lifecycle plans.
//!
//! This verifier consumes immutable broadphase events, prior contact stamps,
//! current revision snapshots, and a producer-supplied plan. It reconstructs
//! the expected actions using a separate audit implementation rather than
//! calling the producer's planner or its freshness assessment method.
//!
//! The verifier is independent at the algorithm/function boundary, not a
//! cryptographic trust anchor. Production qualification still requires a
//! separately built verifier process and an authoritative world snapshot.

use super::contact_freshness::{
    ContactFreshnessError, ContactFreshnessStamp, ContactLifecycleAction,
    ContactLifecycleActionKind, ContactLifecycleError, ContactLifecyclePlan, ContactPayloadRevisions,
    ContactReuseDecision, EndpointRevisions, FreshnessReason, GlobalContactRevisions,
    ReuseDisposition, RevisionDomain,
};
use super::PairDeltaFrame;
use incremental_aabb_tree_reference::{ProxyId, ProxyPair};
use std::collections::{BTreeMap, BTreeSet};

/// Result of verifying a producer-supplied contact lifecycle plan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ContactLifecycleVerificationReport {
    pub added_pairs: usize,
    pub removed_pairs: usize,
    pub revalidated_pairs: usize,
    pub geometry_reuses: usize,
    pub warm_start_reuses: usize,
    pub solver_activations: usize,
    pub full_contact_builds: usize,
}

/// Failure from independent contact-lifecycle replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLifecycleVerificationError {
    Input(ContactLifecycleError),
    ActionCountMismatch { expected: usize, actual: usize },
    ActionMismatch { index: usize },
}

impl From<ContactLifecycleError> for ContactLifecycleVerificationError {
    fn from(value: ContactLifecycleError) -> Self {
        Self::Input(value)
    }
}

/// Independently reconstruct the expected contact work and compare it with
/// the producer's plan. No mutation is performed.
///
/// The audit checks canonical/disjoint pair streams, event-to-action
/// correspondence, endpoint identity, proxy generation, monotonic revision
/// domains, world-revision advancement, and every reuse/refresh decision.
pub fn verify_contact_lifecycle_plan(
    delta: &PairDeltaFrame,
    previous_stamps: &BTreeMap<ProxyPair, ContactFreshnessStamp>,
    current_global: GlobalContactRevisions,
    current_endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
    supplied: &ContactLifecyclePlan,
) -> Result<ContactLifecycleVerificationReport, ContactLifecycleVerificationError> {
    let events = [&delta.added, &delta.removed, &delta.revalidated];
    let mut seen = BTreeSet::new();

    for stream in events {
        let mut previous = None;
        for pair in stream.iter() {
            if pair.0 >= pair.1 || previous.is_some_and(|last| last >= *pair) {
                return Err(ContactLifecycleError::NonCanonicalEventList.into());
            }
            if !seen.insert(*pair) {
                return Err(ContactLifecycleError::ConflictingPairEvents.into());
            }
        }
    }

    let mut expected = Vec::with_capacity(seen.len());

    for pair in &delta.added {
        if previous_stamps.contains_key(pair) {
            return Err(ContactLifecycleError::UnexpectedStampForNewPair(*pair).into());
        }
        require_current_endpoint(*pair, pair.0, current_endpoints)?;
        require_current_endpoint(*pair, pair.1, current_endpoints)?;

        expected.push(ContactLifecycleAction {
            pair: *pair,
            kind: ContactLifecycleActionKind::Added,
            decision: Some(ContactReuseDecision {
                geometry: ReuseDisposition::RefreshRequired,
                warm_start: ReuseDisposition::RefreshRequired,
                // The AABB tournament does not carry a proof that collision
                // filtering already admitted this pair. Recheck eligibility.
                pair_eligibility_recheck_required: true,
                solver_activation_required: true,
                reasons: vec![FreshnessReason::NewPair],
            }),
        });
    }

    for pair in &delta.removed {
        expected.push(ContactLifecycleAction {
            pair: *pair,
            kind: ContactLifecycleActionKind::Removed,
            decision: None,
        });
    }

    for pair in &delta.revalidated {
        require_current_endpoint(*pair, pair.0, current_endpoints)?;
        require_current_endpoint(*pair, pair.1, current_endpoints)?;

        let decision = match previous_stamps.get(pair) {
            Some(stamp) => {
                let audited = independently_assess(stamp, current_global, current_endpoints)?;
                if audited.reasons.is_empty() {
                    return Err(ContactLifecycleError::UnexplainedRevalidation(*pair).into());
                }
                audited
            }
            None => ContactReuseDecision {
                geometry: ReuseDisposition::RefreshRequired,
                warm_start: ReuseDisposition::RefreshRequired,
                pair_eligibility_recheck_required: true,
                solver_activation_required: false,
                reasons: vec![FreshnessReason::MissingContactStamp],
            },
        };

        expected.push(ContactLifecycleAction {
            pair: *pair,
            kind: ContactLifecycleActionKind::Revalidated,
            decision: Some(decision),
        });
    }

    if supplied.actions.len() != expected.len() {
        return Err(ContactLifecycleVerificationError::ActionCountMismatch {
            expected: expected.len(),
            actual: supplied.actions.len(),
        });
    }

    for (index, (expected_action, actual_action)) in
        expected.iter().zip(&supplied.actions).enumerate()
    {
        if expected_action != actual_action {
            return Err(ContactLifecycleVerificationError::ActionMismatch { index });
        }
    }

    let mut report = ContactLifecycleVerificationReport {
        added_pairs: delta.added.len(),
        removed_pairs: delta.removed.len(),
        revalidated_pairs: delta.revalidated.len(),
        ..ContactLifecycleVerificationReport::default()
    };

    for action in &expected {
        if let Some(decision) = &action.decision {
            if decision.geometry == ReuseDisposition::Reusable {
                report.geometry_reuses += 1;
            } else {
                report.full_contact_builds += 1;
            }
            if decision.warm_start == ReuseDisposition::Reusable {
                report.warm_start_reuses += 1;
            }
            if decision.solver_activation_required {
                report.solver_activations += 1;
            }
        }
    }

    Ok(report)
}

fn require_current_endpoint(
    pair: ProxyPair,
    id: ProxyId,
    endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
) -> Result<EndpointRevisions, ContactLifecycleVerificationError> {
    let endpoint = endpoints
        .get(&id)
        .copied()
        .ok_or(ContactLifecycleError::MissingEndpointRevision(id))?;
    if endpoint.proxy != id || (id != pair.0 && id != pair.1) {
        return Err(ContactFreshnessError::PairIdentityMismatch.into());
    }
    Ok(endpoint)
}

/// Separate audit implementation of the revision-to-reuse policy.
///
/// Intentionally does not call ContactFreshnessStamp::assess or the producer
/// planner. Keep this logic small and exhaustively cross-check it against
/// mutation-class fixtures.
fn independently_assess(
    stamp: &ContactFreshnessStamp,
    global: GlobalContactRevisions,
    endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
) -> Result<ContactReuseDecision, ContactLifecycleVerificationError> {
    let left = require_current_endpoint(stamp.pair, stamp.pair.0, endpoints)?;
    let right = require_current_endpoint(stamp.pair, stamp.pair.1, endpoints)?;

    if stamp.pair.0 != stamp.left.proxy
        || stamp.pair.1 != stamp.right.proxy
        || stamp.pair.0 != left.proxy
        || stamp.pair.1 != right.proxy
    {
        return Err(ContactFreshnessError::PairIdentityMismatch.into());
    }
    if global.world < stamp.global.world {
        return Err(ContactFreshnessError::WorldRevisionRegressed.into());
    }
    if global.collision_policy < stamp.global.collision_policy {
        return Err(ContactFreshnessError::CollisionPolicyRevisionRegressed.into());
    }

    let mut reasons = BTreeSet::new();
    let mut geometry = ReuseDisposition::Reusable;
    let mut warm_start = ReuseDisposition::Reusable;
    let mut eligibility_recheck = false;
    let mut activate_solver = false;

    audit_endpoint(
        stamp.left,
        left,
        &mut reasons,
        &mut geometry,
        &mut warm_start,
        &mut eligibility_recheck,
        &mut activate_solver,
    )?;
    audit_endpoint(
        stamp.right,
        right,
        &mut reasons,
        &mut geometry,
        &mut warm_start,
        &mut eligibility_recheck,
        &mut activate_solver,
    )?;

    if global.collision_policy != stamp.global.collision_policy {
        reasons.insert(FreshnessReason::CollisionPolicyChanged);
        geometry = ReuseDisposition::RefreshRequired;
        eligibility_recheck = true;
    }
    if global.narrowphase_config_sha256 != stamp.global.narrowphase_config_sha256 {
        reasons.insert(FreshnessReason::NarrowphaseConfigurationChanged);
        geometry = ReuseDisposition::RefreshRequired;
    }
    if global.solver_config_sha256 != stamp.global.solver_config_sha256 {
        reasons.insert(FreshnessReason::SolverConfigurationChanged);
        warm_start = ReuseDisposition::RefreshRequired;
    }

    if !reasons.is_empty() && global.world == stamp.global.world {
        return Err(ContactFreshnessError::WorldRevisionDidNotAdvanceForMutation.into());
    }
    if geometry == ReuseDisposition::RefreshRequired {
        warm_start = ReuseDisposition::RefreshRequired;
    }

    Ok(ContactReuseDecision {
        geometry,
        warm_start,
        pair_eligibility_recheck_required: eligibility_recheck,
        solver_activation_required: activate_solver,
        reasons: reasons.into_iter().collect(),
    })
}

#[allow(clippy::too_many_arguments)]
fn audit_endpoint(
    previous: EndpointRevisions,
    current: EndpointRevisions,
    reasons: &mut BTreeSet<FreshnessReason>,
    geometry: &mut ReuseDisposition,
    warm_start: &mut ReuseDisposition,
    eligibility_recheck: &mut bool,
    activate_solver: &mut bool,
) -> Result<(), ContactLifecycleVerificationError> {
    if current.proxy != previous.proxy {
        return Err(ContactFreshnessError::PairIdentityMismatch.into());
    }
    audit_monotonic(
        previous.generation,
        current.generation,
        current.proxy,
        RevisionDomain::Generation,
    )?;

    if current.generation != previous.generation {
        reasons.insert(FreshnessReason::ProxyGenerationChanged);
        *geometry = ReuseDisposition::RefreshRequired;
        *warm_start = ReuseDisposition::RefreshRequired;
        *eligibility_recheck = true;
        return Ok(());
    }

    audit_domain(
        previous.transform,
        current.transform,
        current.proxy,
        RevisionDomain::Transform,
    )?;
    if current.transform != previous.transform {
        reasons.insert(FreshnessReason::TransformChanged);
        *geometry = ReuseDisposition::RefreshRequired;
    }

    audit_domain(
        previous.shape,
        current.shape,
        current.proxy,
        RevisionDomain::Shape,
    )?;
    if current.shape != previous.shape {
        reasons.insert(FreshnessReason::ShapeChanged);
        *geometry = ReuseDisposition::RefreshRequired;
    }

    audit_domain(
        previous.eligibility,
        current.eligibility,
        current.proxy,
        RevisionDomain::Eligibility,
    )?;
    if current.eligibility != previous.eligibility {
        reasons.insert(FreshnessReason::CollisionEligibilityChanged);
        *geometry = ReuseDisposition::RefreshRequired;
        *eligibility_recheck = true;
    }

    audit_domain(
        previous.motion_class,
        current.motion_class,
        current.proxy,
        RevisionDomain::MotionClass,
    )?;
    if current.motion_class != previous.motion_class {
        reasons.insert(FreshnessReason::MotionClassChanged);
        *geometry = ReuseDisposition::RefreshRequired;
        *eligibility_recheck = true;
    }

    audit_domain(
        previous.material,
        current.material,
        current.proxy,
        RevisionDomain::Material,
    )?;
    if current.material != previous.material {
        reasons.insert(FreshnessReason::ContactMaterialChanged);
        *warm_start = ReuseDisposition::RefreshRequired;
    }

    audit_domain(
        previous.wake_epoch,
        current.wake_epoch,
        current.proxy,
        RevisionDomain::WakeEpoch,
    )?;
    if current.wake_epoch != previous.wake_epoch {
        reasons.insert(FreshnessReason::WakeEpochAdvanced);
        *activate_solver = true;
    }

    Ok(())
}

fn audit_domain(
    previous: u64,
    current: u64,
    proxy: ProxyId,
    domain: RevisionDomain,
) -> Result<(), ContactLifecycleVerificationError> {
    audit_monotonic(previous, current, proxy, domain)
}

fn audit_monotonic(
    previous: u64,
    current: u64,
    proxy: ProxyId,
    domain: RevisionDomain,
) -> Result<(), ContactLifecycleVerificationError> {
    if current < previous {
        return Err(ContactFreshnessError::EndpointRevisionRegressed { proxy, domain }.into());
    }
    Ok(())
}

/// Complete immutable transition input for one world step.
///
/// Pair relations are supplied as pre/post snapshots and must already have
/// passed the broadphase's independent geometric oracle. This verifier audits
/// lifecycle deltas and contact freshness; it does not independently derive
/// geometric overlap from shapes or AABBs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactLifecycleReceipt {
    pub previous_global: GlobalContactRevisions,
    pub current_global: GlobalContactRevisions,
    pub previous_pair_relation: Vec<ProxyPair>,
    pub current_pair_relation: Vec<ProxyPair>,
    pub previous_endpoints: BTreeMap<ProxyId, EndpointRevisions>,
    pub current_endpoints: BTreeMap<ProxyId, EndpointRevisions>,
    pub previous_contact_stamps: BTreeMap<ProxyPair, ContactFreshnessStamp>,
    /// Stamps committed after all required contact actions completed. A pair
    /// without a stamp may be a conservative broadphase candidate with no
    /// exact shape contact.
    pub post_contact_stamps: BTreeMap<ProxyPair, ContactFreshnessStamp>,
    pub producer_plan: ContactLifecyclePlan,
}

/// Additional receipt-level failures, beyond plan/action mismatches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLifecycleReceiptError {
    WorldRevisionDidNotAdvance,
    CollisionPolicyRevisionRegressed,
    NonCanonicalPairRelation,
    PairEndpointMissing(ProxyId),
    EndpointMapKeyMismatch(ProxyId),
    UnexpectedPriorContactStamp(ProxyPair),
    PriorContactStampMismatch(ProxyPair),
    PriorContactStampGlobalMismatch(ProxyPair),
    EndpointRevisionRegressed {
        proxy: ProxyId,
        domain: RevisionDomain,
    },
    UnexpectedPostContactStamp(ProxyPair),
    PostContactStampMismatch(ProxyPair),
    MissingCommittedContactStamp(ProxyPair),
    PriorContactPayloadDigestMissing(ProxyPair),
    PostContactPayloadDigestMissing(ProxyPair),
    ManifoldPayloadRevisionDidNotAdvance(ProxyPair),
    ManifoldPayloadChangedWithoutRefresh(ProxyPair),
    WarmStartPayloadRevisionDidNotAdvance(ProxyPair),
    WarmStartPayloadChangedWithoutReset(ProxyPair),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLifecycleReceiptVerificationError {
    Receipt(ContactLifecycleReceiptError),
    Plan(ContactLifecycleVerificationError),
}

impl From<ContactLifecycleReceiptError> for ContactLifecycleReceiptVerificationError {
    fn from(value: ContactLifecycleReceiptError) -> Self {
        Self::Receipt(value)
    }
}

impl From<ContactLifecycleVerificationError> for ContactLifecycleReceiptVerificationError {
    fn from(value: ContactLifecycleVerificationError) -> Self {
        Self::Plan(value)
    }
}

impl From<ContactLifecycleError> for ContactLifecycleReceiptVerificationError {
    fn from(value: ContactLifecycleError) -> Self {
        Self::Plan(ContactLifecycleVerificationError::Input(value))
    }
}

/// Replay the pair/contact transition from immutable before/after snapshots.
///
/// The verifier derives additions/removals from pair-relation set differences
/// and derives retained-pair revalidation from endpoint/global revision
/// changes. A producer cannot omit a revalidation simply by omitting its event.
/// The pre/post pair relations must have been independently checked against
/// the geometric AABB/shape oracle before they are admitted to this receipt.
pub fn verify_contact_lifecycle_receipt(
    receipt: &ContactLifecycleReceipt,
) -> Result<ContactLifecycleVerificationReport, ContactLifecycleReceiptVerificationError> {
    if receipt.current_global.world <= receipt.previous_global.world {
        return Err(ContactLifecycleReceiptError::WorldRevisionDidNotAdvance.into());
    }
    if receipt.current_global.collision_policy < receipt.previous_global.collision_policy {
        return Err(ContactLifecycleReceiptError::CollisionPolicyRevisionRegressed.into());
    }

    validate_pair_relation(&receipt.previous_pair_relation)?;
    validate_pair_relation(&receipt.current_pair_relation)?;
    validate_endpoint_map(&receipt.previous_endpoints)?;
    validate_endpoint_map(&receipt.current_endpoints)?;

    let previous = receipt
        .previous_pair_relation
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let current = receipt
        .current_pair_relation
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    for pair in &previous {
        require_relation_endpoints(*pair, &receipt.previous_endpoints)?;
    }
    for pair in &current {
        require_relation_endpoints(*pair, &receipt.current_endpoints)?;
    }

    for (key, stamp) in &receipt.previous_contact_stamps {
        if !previous.contains(key) {
            return Err(ContactLifecycleReceiptError::UnexpectedPriorContactStamp(*key).into());
        }
        if stamp.pair != *key {
            return Err(ContactLifecycleReceiptError::PriorContactStampMismatch(*key).into());
        }
        if !stamp.payload.has_digests() {
            return Err(ContactLifecycleReceiptError::PriorContactPayloadDigestMissing(*key).into());
        }
        let left = receipt.previous_endpoints.get(&key.0).copied();
        let right = receipt.previous_endpoints.get(&key.1).copied();
        if left != Some(stamp.left) || right != Some(stamp.right) {
            return Err(ContactLifecycleReceiptError::PriorContactStampMismatch(*key).into());
        }
        if stamp.global.collision_policy != receipt.previous_global.collision_policy
            || stamp.global.narrowphase_config_sha256
                != receipt.previous_global.narrowphase_config_sha256
            || stamp.global.solver_config_sha256 != receipt.previous_global.solver_config_sha256
            || stamp.global.world > receipt.previous_global.world
        {
            return Err(ContactLifecycleReceiptError::PriorContactStampGlobalMismatch(*key).into());
        }
    }

    let mut dirty_endpoints = BTreeSet::new();
    for (id, previous_endpoint) in &receipt.previous_endpoints {
        match receipt.current_endpoints.get(id) {
            Some(current_endpoint) => {
                audit_endpoint_revision_monotonicity(*previous_endpoint, *current_endpoint)?;
                if previous_endpoint != current_endpoint {
                    dirty_endpoints.insert(*id);
                }
            }
            None => {
                dirty_endpoints.insert(*id);
            }
        }
    }
    for id in receipt.current_endpoints.keys() {
        if !receipt.previous_endpoints.contains_key(id) {
            dirty_endpoints.insert(*id);
        }
    }

    let global_contact_policy_changed = receipt.current_global.collision_policy
        != receipt.previous_global.collision_policy
        || receipt.current_global.narrowphase_config_sha256
            != receipt.previous_global.narrowphase_config_sha256
        || receipt.current_global.solver_config_sha256
            != receipt.previous_global.solver_config_sha256;

    let mut delta = PairDeltaFrame::default();
    delta.added = current.difference(&previous).copied().collect();
    delta.removed = previous.difference(&current).copied().collect();
    delta.revalidated = current
        .intersection(&previous)
        .filter(|pair| {
            global_contact_policy_changed
                || dirty_endpoints.contains(&pair.0)
                || dirty_endpoints.contains(&pair.1)
                || !receipt.previous_contact_stamps.contains_key(pair)
        })
        .copied()
        .collect();

    for (pair, stamp) in &receipt.post_contact_stamps {
        if !current.contains(pair) {
            return Err(ContactLifecycleReceiptError::UnexpectedPostContactStamp(*pair).into());
        }
        if !stamp.payload.has_digests() {
            return Err(ContactLifecycleReceiptError::PostContactPayloadDigestMissing(*pair).into());
        }
        let left = receipt.current_endpoints.get(&pair.0).copied();
        let right = receipt.current_endpoints.get(&pair.1).copied();
        if stamp.pair != *pair
            || left != Some(stamp.left)
            || right != Some(stamp.right)
            || stamp.global.collision_policy != receipt.current_global.collision_policy
            || stamp.global.narrowphase_config_sha256
                != receipt.current_global.narrowphase_config_sha256
            || stamp.global.solver_config_sha256 != receipt.current_global.solver_config_sha256
            || stamp.global.world > receipt.current_global.world
        {
            return Err(ContactLifecycleReceiptError::PostContactStampMismatch(*pair).into());
        }
    }

    // If old contact geometry and eligibility remain valid, the contact must
    // still have a committed stamp after the transition. A wake or warm-start
    // reset alone is not a reason to silently drop existing contact state.
    for pair in current.intersection(&previous) {
        let Some(stamp) = receipt.previous_contact_stamps.get(pair) else {
            continue;
        };
        let left = receipt
            .current_endpoints
            .get(&pair.0)
            .copied()
            .ok_or(ContactLifecycleError::MissingEndpointRevision(pair.0))?;
        let right = receipt
            .current_endpoints
            .get(&pair.1)
            .copied()
            .ok_or(ContactLifecycleError::MissingEndpointRevision(pair.1))?;
        let decision = independently_assess(
            stamp,
            receipt.current_global,
            &BTreeMap::from([(pair.0, left), (pair.1, right)]),
        )?;
        if decision.geometry == ReuseDisposition::Reusable
            && !decision.pair_eligibility_recheck_required
            && !receipt.post_contact_stamps.contains_key(pair)
        {
            return Err(ContactLifecycleReceiptError::MissingCommittedContactStamp(*pair).into());
        }
        if let Some(post) = receipt.post_contact_stamps.get(pair) {
            let generation_changed = left.generation != stamp.left.generation
                || right.generation != stamp.right.generation;
            if !generation_changed {
                if decision.geometry == ReuseDisposition::RefreshRequired {
                    if post.payload.manifold_revision <= stamp.payload.manifold_revision {
                        return Err(
                            ContactLifecycleReceiptError::ManifoldPayloadRevisionDidNotAdvance(
                                *pair,
                            )
                            .into(),
                        );
                    }
                } else if post.payload.manifold_revision != stamp.payload.manifold_revision
                    || post.payload.manifold_digest_sha256
                        != stamp.payload.manifold_digest_sha256
                {
                    return Err(
                        ContactLifecycleReceiptError::ManifoldPayloadChangedWithoutRefresh(*pair)
                            .into(),
                    );
                }
                if decision.warm_start == ReuseDisposition::RefreshRequired {
                    if post.payload.warm_start_revision <= stamp.payload.warm_start_revision {
                        return Err(
                            ContactLifecycleReceiptError::WarmStartPayloadRevisionDidNotAdvance(
                                *pair,
                            )
                            .into(),
                        );
                    }
                } else if post.payload.warm_start_revision != stamp.payload.warm_start_revision
                    || post.payload.warm_start_digest_sha256
                        != stamp.payload.warm_start_digest_sha256
                {
                    return Err(
                        ContactLifecycleReceiptError::WarmStartPayloadChangedWithoutReset(*pair)
                            .into(),
                    );
                }
            }
        }
    }

    let report = verify_contact_lifecycle_plan(
        &delta,
        &receipt.previous_contact_stamps,
        receipt.current_global,
        &receipt.current_endpoints,
        &receipt.producer_plan,
    )?;
    Ok(report)
}

fn validate_pair_relation(
    pairs: &[ProxyPair],
) -> Result<(), ContactLifecycleReceiptVerificationError> {
    let mut previous = None;
    for pair in pairs {
        if pair.0 >= pair.1 || previous.is_some_and(|last| last >= *pair) {
            return Err(ContactLifecycleReceiptError::NonCanonicalPairRelation.into());
        }
        previous = Some(*pair);
    }
    Ok(())
}

fn validate_endpoint_map(
    endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
) -> Result<(), ContactLifecycleReceiptVerificationError> {
    for (id, endpoint) in endpoints {
        if *id != endpoint.proxy {
            return Err(ContactLifecycleReceiptError::EndpointMapKeyMismatch(*id).into());
        }
    }
    Ok(())
}

fn require_relation_endpoints(
    pair: ProxyPair,
    endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
) -> Result<(), ContactLifecycleReceiptVerificationError> {
    for id in [pair.0, pair.1] {
        if !endpoints.contains_key(&id) {
            return Err(ContactLifecycleReceiptError::PairEndpointMissing(id).into());
        }
    }
    Ok(())
}

fn audit_endpoint_revision_monotonicity(
    previous: EndpointRevisions,
    current: EndpointRevisions,
) -> Result<(), ContactLifecycleReceiptVerificationError> {
    if previous.proxy != current.proxy {
        return Err(ContactLifecycleReceiptError::EndpointMapKeyMismatch(current.proxy).into());
    }
    for (old, new, domain) in [
        (
            previous.generation,
            current.generation,
            RevisionDomain::Generation,
        ),
        (
            previous.transform,
            current.transform,
            RevisionDomain::Transform,
        ),
        (previous.shape, current.shape, RevisionDomain::Shape),
        (
            previous.eligibility,
            current.eligibility,
            RevisionDomain::Eligibility,
        ),
        (
            previous.motion_class,
            current.motion_class,
            RevisionDomain::MotionClass,
        ),
        (
            previous.material,
            current.material,
            RevisionDomain::Material,
        ),
        (
            previous.wake_epoch,
            current.wake_epoch,
            RevisionDomain::WakeEpoch,
        ),
    ] {
        // Within a generation, all revision domains are monotonic. Once the
        // generation changes, the old counter values are a different identity
        // lifetime and are intentionally not compared.
        if domain != RevisionDomain::Generation && previous.generation != current.generation {
            continue;
        }
        if new < old {
            return Err(ContactLifecycleReceiptError::EndpointRevisionRegressed {
                proxy: current.proxy,
                domain,
            }
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(id: u64) -> EndpointRevisions {
        EndpointRevisions {
            proxy: ProxyId(id),
            generation: 1,
            transform: 4,
            shape: 2,
            eligibility: 3,
            motion_class: 1,
            material: 7,
            wake_epoch: 2,
        }
    }

    fn global() -> GlobalContactRevisions {
        GlobalContactRevisions {
            world: 10,
            collision_policy: 5,
            narrowphase_config_sha256: [11; 32],
            solver_config_sha256: [12; 32],
        }
    }

    fn next_global() -> GlobalContactRevisions {
        let mut current = global();
        current.world += 1;
        current
    }

    fn payload() -> ContactPayloadRevisions {
        ContactPayloadRevisions {
            manifold_revision: 1,
            manifold_digest_sha256: [21; 32],
            warm_start_revision: 1,
            warm_start_digest_sha256: [22; 32],
        }
    }

    fn stamp() -> ContactFreshnessStamp {
        ContactFreshnessStamp::new(
            ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs"),
            global(),
            endpoint(1),
            endpoint(2),
            payload(),
        )
        .expect("valid contact freshness stamp")
    }

    fn make_plan(
        delta: &PairDeltaFrame,
        stamps: &BTreeMap<ProxyPair, ContactFreshnessStamp>,
        current_global: GlobalContactRevisions,
        endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
    ) -> ContactLifecyclePlan {
        super::super::contact_freshness::plan_contact_lifecycle(
            delta,
            stamps,
            current_global,
            endpoints,
        )
        .expect("producer plan")
    }

    #[test]
    fn verifier_accepts_exact_wake_only_plan() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let stamps = BTreeMap::from([(pair, stamp())]);
        let mut awake = endpoint(1);
        awake.wake_epoch += 1;
        let endpoints = BTreeMap::from([(ProxyId(1), awake), (ProxyId(2), endpoint(2))]);
        let plan = make_plan(&delta, &stamps, next_global(), &endpoints);

        let report =
            verify_contact_lifecycle_plan(&delta, &stamps, next_global(), &endpoints, &plan)
                .expect("independent verifier accepts valid wake-only transition");

        assert_eq!(report.revalidated_pairs, 1);
        assert_eq!(report.geometry_reuses, 1);
        assert_eq!(report.warm_start_reuses, 1);
        assert_eq!(report.solver_activations, 1);
    }

    #[test]
    fn verifier_rejects_wake_only_manifold_discard() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let stamps = BTreeMap::from([(pair, stamp())]);
        let mut awake = endpoint(1);
        awake.wake_epoch += 1;
        let endpoints = BTreeMap::from([(ProxyId(1), awake), (ProxyId(2), endpoint(2))]);
        let mut plan = make_plan(&delta, &stamps, next_global(), &endpoints);
        plan.actions[0]
            .decision
            .as_mut()
            .expect("decision")
            .geometry = ReuseDisposition::RefreshRequired;

        assert_eq!(
            verify_contact_lifecycle_plan(&delta, &stamps, next_global(), &endpoints, &plan,),
            Err(ContactLifecycleVerificationError::ActionMismatch { index: 0 })
        );
    }

    #[test]
    fn verifier_rejects_suppressed_wake_activation() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let stamps = BTreeMap::from([(pair, stamp())]);
        let mut awake = endpoint(1);
        awake.wake_epoch += 1;
        let endpoints = BTreeMap::from([(ProxyId(1), awake), (ProxyId(2), endpoint(2))]);
        let mut plan = make_plan(&delta, &stamps, next_global(), &endpoints);
        plan.actions[0]
            .decision
            .as_mut()
            .expect("decision")
            .solver_activation_required = false;

        assert_eq!(
            verify_contact_lifecycle_plan(
                &delta,
                &stamps,
                next_global(),
                &endpoints,
                &plan,
            ),
            Err(ContactLifecycleVerificationError::ActionMismatch { index: 0 })
        );
    }

    #[test]
    fn verifier_rejects_missing_event_action() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut delta = PairDeltaFrame::default();
        delta.added.push(pair);
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        let plan = ContactLifecyclePlan::default();

        assert_eq!(
            verify_contact_lifecycle_plan(
                &delta,
                &BTreeMap::new(),
                global(),
                &endpoints,
                &plan,
            ),
            Err(ContactLifecycleVerificationError::ActionCountMismatch {
                expected: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn verifier_rejects_a_reusable_new_pair_decision() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut delta = PairDeltaFrame::default();
        delta.added.push(pair);
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        let mut plan = make_plan(&delta, &BTreeMap::new(), global(), &endpoints);
        plan.actions[0]
            .decision
            .as_mut()
            .expect("decision")
            .geometry = ReuseDisposition::Reusable;

        assert_eq!(
            verify_contact_lifecycle_plan(
                &delta,
                &BTreeMap::new(),
                global(),
                &endpoints,
                &plan,
            ),
            Err(ContactLifecycleVerificationError::ActionMismatch { index: 0 })
        );
    }

    #[test]
    fn verifier_rejects_world_revision_regression() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let stamps = BTreeMap::from([(pair, stamp())]);
        let mut older = global();
        older.world -= 1;
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);

        assert_eq!(
            verify_contact_lifecycle_plan(
                &delta,
                &stamps,
                older,
                &endpoints,
                &ContactLifecyclePlan::default(),
            ),
            Err(ContactLifecycleVerificationError::Input(
                ContactLifecycleError::Freshness(ContactFreshnessError::WorldRevisionRegressed)
            ))
        );
    }

    fn receipt_fixture(
        wake_only: bool,
    ) -> (
        ContactLifecycleReceipt,
        PairDeltaFrame,
        BTreeMap<ProxyPair, ContactFreshnessStamp>,
    ) {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let previous_global = global();
        let mut current_global = global();
        current_global.world += 1;
        let previous_endpoints =
            BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        let mut current_endpoints = previous_endpoints.clone();
        if wake_only {
            current_endpoints
                .get_mut(&ProxyId(1))
                .expect("left")
                .wake_epoch += 1;
        }
        let stamps = BTreeMap::from([(pair, stamp())]);
        let mut delta = PairDeltaFrame::default();
        if wake_only {
            delta.revalidated.push(pair);
        }
        let plan = super::super::contact_freshness::plan_contact_lifecycle(
            &delta,
            &stamps,
            current_global,
            &current_endpoints,
        )
        .expect("producer plan");
        (
            ContactLifecycleReceipt {
                previous_global,
                current_global,
                previous_pair_relation: vec![pair],
                current_pair_relation: vec![pair],
                previous_endpoints,
                current_endpoints,
                previous_contact_stamps: stamps.clone(),
                post_contact_stamps: if wake_only {
                    let awake = current_endpoints.get(&ProxyId(1)).copied().expect("awake");
                    let committed = stamps[&pair]
                        .commit_transition(
                            current_global,
                            awake,
                            endpoint(2),
                            false,
                            false,
                            false,
                            true,
                            stamps[&pair].payload,
                        )
                        .expect("wake transition committed");
                    BTreeMap::from([(pair, committed)])
                } else {
                    stamps.clone()
                },
                producer_plan: plan,
            },
            delta,
            stamps,
        )
    }

    #[test]
    fn receipt_replay_derives_wake_only_revalidation() {
        let (receipt, _, _) = receipt_fixture(true);
        let report =
            verify_contact_lifecycle_receipt(&receipt).expect("replay verifies wake-only pair");
        assert_eq!(report.revalidated_pairs, 1);
        assert_eq!(report.geometry_reuses, 1);
        assert_eq!(report.warm_start_reuses, 1);
        assert_eq!(report.solver_activations, 1);
    }

    #[test]
    fn receipt_replay_rejects_omitted_wake_revalidation() {
        let (mut receipt, _, _) = receipt_fixture(true);
        receipt.producer_plan.actions.clear();
        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Plan(
                ContactLifecycleVerificationError::ActionCountMismatch {
                    expected: 1,
                    actual: 0,
                }
            ))
        );
    }

    #[test]
    fn receipt_replay_rejects_nonadvancing_world_revision() {
        let (mut receipt, _, _) = receipt_fixture(false);
        receipt.current_global.world = receipt.previous_global.world;
        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Receipt(
                ContactLifecycleReceiptError::WorldRevisionDidNotAdvance
            ))
        );
    }

    #[test]
    fn receipt_replay_rejects_prior_stamp_that_does_not_match_snapshot() {
        let (mut receipt, _, _) = receipt_fixture(false);
        receipt
            .previous_endpoints
            .get_mut(&ProxyId(1))
            .expect("left")
            .transform += 1;
        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Receipt(
                ContactLifecycleReceiptError::PriorContactStampMismatch(
                    ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair")
                )
            ))
        );
    }

    #[test]
    fn receipt_replay_rejects_pair_relation_without_endpoint() {
        let (mut receipt, _, _) = receipt_fixture(false);
        receipt.current_endpoints.remove(&ProxyId(2));
        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Receipt(
                ContactLifecycleReceiptError::PairEndpointMissing(ProxyId(2))
            ))
        );
    }

    #[test]
    fn receipt_replay_derives_new_and_removed_pair_events() {
        let old_pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("old pair");
        let new_pair = ProxyPair::new(ProxyId(2), ProxyId(3)).expect("new pair");
        let previous_global = global();
        let mut current_global = global();
        current_global.world += 1;
        let previous_endpoints =
            BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        let current_endpoints =
            BTreeMap::from([(ProxyId(2), endpoint(2)), (ProxyId(3), endpoint(3))]);
        let stamps = BTreeMap::from([(
            old_pair,
            ContactFreshnessStamp::new(
                old_pair,
                previous_global,
                endpoint(1),
                endpoint(2),
                payload(),
            )
            .expect("old stamp"),
        )]);
        let mut delta = PairDeltaFrame::default();
        delta.added.push(new_pair);
        delta.removed.push(old_pair);
        let plan = super::super::contact_freshness::plan_contact_lifecycle(
            &delta,
            &stamps,
            current_global,
            &current_endpoints,
        )
        .expect("producer plan");
        let receipt = ContactLifecycleReceipt {
            previous_global,
            current_global,
            previous_pair_relation: vec![old_pair],
            current_pair_relation: vec![new_pair],
            previous_endpoints,
            current_endpoints,
            previous_contact_stamps: stamps,
            post_contact_stamps: BTreeMap::new(),
            producer_plan: plan,
        };
        let report =
            verify_contact_lifecycle_receipt(&receipt).expect("replay exact set differences");
        assert_eq!(report.added_pairs, 1);
        assert_eq!(report.removed_pairs, 1);
        assert_eq!(report.full_contact_builds, 1);
    }

    #[test]
    fn receipt_replay_rejects_uncommitted_wake_contact_stamp {
        let (mut receipt, _, _) = receipt_fixture(true);
        receipt.post_contact_stamps.clear();
        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Receipt(
                ContactLifecycleReceiptError::MissingCommittedContactStamp(
                    ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair")
                )
            ))
        );
    }

    #[test]
    fn receipt_replay_rejects_post_stamp_with_stale_endpoint_revision() {
        let (mut receipt, _, _) = receipt_fixture(true);
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut stale = receipt.post_contact_stamps[&pair];
        stale.left.wake_epoch -= 1;
        receipt.post_contact_stamps.insert(pair, stale);
        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Receipt(
                ContactLifecycleReceiptError::PostContactStampMismatch(pair)
            ))
        );
    }

    #[test]
    fn receipt_replay_requires_revalidation_for_global_solver_change() {
        let (mut receipt, _, _) = receipt_fixture(false);
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        receipt.current_global.solver_config_sha256 = [42; 32];
        let old_stamp = receipt.previous_contact_stamps[&pair].clone();
        let committed = old_stamp
            .commit_transition(
                receipt.current_global,
                endpoint(1),
                endpoint(2),
                false,
                true,
                false,
                false,
                ContactPayloadRevisions {
                    manifold_revision: 1,
                    manifold_digest_sha256: [21; 32],
                    warm_start_revision: 2,
                    warm_start_digest_sha256: [23; 32],
                },
            )
            .expect("warm-start reset committed under new solver config");
        receipt.post_contact_stamps = BTreeMap::from([(pair, committed)]);
        receipt.producer_plan.actions.clear();

        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Plan(
                ContactLifecycleVerificationError::ActionCountMismatch {
                    expected: 1,
                    actual: 0,
                }
            ))
        );
    }

    #[test]
    fn receipt_replay_commits_shape_refresh_and_warm_start_reset() {
        let (mut receipt, _, _) = receipt_fixture(false);
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut changed_left = endpoint(1);
        changed_left.shape += 1;
        receipt.current_endpoints.insert(ProxyId(1), changed_left);

        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        receipt.producer_plan = super::super::contact_freshness::plan_contact_lifecycle(
            &delta,
            &receipt.previous_contact_stamps,
            receipt.current_global,
            &receipt.current_endpoints,
        )
        .expect("shape change produces a typed revalidation");
        let prior = receipt.previous_contact_stamps[&pair];
        let committed = prior
            .commit_transition(
                receipt.current_global,
                changed_left,
                endpoint(2),
                true,
                true,
                false,
                false,
                ContactPayloadRevisions {
                    manifold_revision: 2,
                    manifold_digest_sha256: [24; 32],
                    warm_start_revision: 2,
                    warm_start_digest_sha256: [23; 32],
                },
            )
            .expect("geometry and warm-start work completed");
        receipt.post_contact_stamps = BTreeMap::from([(pair, committed)]);

        let report = verify_contact_lifecycle_receipt(&receipt)
            .expect("shape update receipt replay succeeds");
        assert_eq!(report.revalidated_pairs, 1);
        assert_eq!(report.geometry_reuses, 0);
        assert_eq!(report.warm_start_reuses, 0);
        assert_eq!(report.full_contact_builds, 1);
    }

    #[test]
    fn receipt_replay_rejects_omitted_shape_revalidation() {
        let (mut receipt, _, _) = receipt_fixture(false);
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut changed_left = endpoint(1);
        changed_left.shape += 1;
        receipt.current_endpoints.insert(ProxyId(1), changed_left);
        let prior = receipt.previous_contact_stamps[&pair];
        let committed = prior
            .commit_transition(
                receipt.current_global,
                changed_left,
                endpoint(2),
                true,
                true,
                false,
                false,
                ContactPayloadRevisions {
                    manifold_revision: 2,
                    manifold_digest_sha256: [24; 32],
                    warm_start_revision: 2,
                    warm_start_digest_sha256: [23; 32],
                },
            )
            .expect("producer completed required work");
        receipt.post_contact_stamps = BTreeMap::from([(pair, committed)]);
        receipt.producer_plan.actions.clear();

        assert_eq!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Plan(
                ContactLifecycleVerificationError::ActionCountMismatch {
                    expected: 1,
                    actual: 0,
                }
            ))
        );
    }

    #[test]
    fn receipt_replay_requires_filter_recheck_and_post_stamp() {
        let (mut receipt, _, _) = receipt_fixture(false);
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        receipt.current_global.collision_policy += 1;
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        receipt.producer_plan = super::super::contact_freshness::plan_contact_lifecycle(
            &delta,
            &receipt.previous_contact_stamps,
            receipt.current_global,
            &receipt.current_endpoints,
        )
        .expect("global filter revision requires revalidation");
        let prior = receipt.previous_contact_stamps[&pair];
        let committed = prior
            .commit_transition(
                receipt.current_global,
                endpoint(1),
                endpoint(2),
                true,
                true,
                true,
                false,
                ContactPayloadRevisions {
                    manifold_revision: 2,
                    manifold_digest_sha256: [24; 32],
                    warm_start_revision: 2,
                    warm_start_digest_sha256: [23; 32],
                },
            )
            .expect("geometry, warm-start and eligibility work completed");
        receipt.post_contact_stamps = BTreeMap::from([(pair, committed)]);

        let report = verify_contact_lifecycle_receipt(&receipt)
            .expect("filter revision receipt replay succeeds");
        assert_eq!(report.revalidated_pairs, 1);
        assert_eq!(report.geometry_reuses, 0);
    }

    #[test]
    fn receipt_replay_accepts_generation_advance_with_counter_reset() {
        let (mut receipt, _, _) = receipt_fixture(false);
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("pair");
        let mut recycled = endpoint(1);
        recycled.generation += 1;
        recycled.transform = 0;
        recycled.shape = 0;
        recycled.eligibility = 0;
        recycled.motion_class = 0;
        recycled.material = 0;
        recycled.wake_epoch = 0;
        receipt.current_endpoints.insert(ProxyId(1), recycled);

        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        receipt.producer_plan = super::super::contact_freshness::plan_contact_lifecycle(
            &delta,
            &receipt.previous_contact_stamps,
            receipt.current_global,
            &receipt.current_endpoints,
        )
        .expect("new proxy generation is a new residency");
        let prior = receipt.previous_contact_stamps[&pair];
        let committed = prior
            .commit_transition(
                receipt.current_global,
                recycled,
                endpoint(2),
                true,
                true,
                true,
                false,
                ContactPayloadRevisions {
                    manifold_revision: 1,
                    manifold_digest_sha256: [30; 32],
                    warm_start_revision: 1,
                    warm_start_digest_sha256: [31; 32],
                },
            )
            .expect("new generation gets a new contact stamp");
        receipt.post_contact_stamps = BTreeMap::from([(pair, committed)]);

        let report = verify_contact_lifecycle_receipt(&receipt)
            .expect("generation change receipt replay succeeds");
        assert_eq!(report.revalidated_pairs, 1);
        assert_eq!(report.full_contact_builds, 1);
    }

    #[test]
    fn receipt_replay_rejects_generation_regression() {
        let (mut receipt, _, _) = receipt_fixture(false);
        let mut older = endpoint(1);
        older.generation -= 1;
        receipt.current_endpoints.insert(ProxyId(1), older);
        assert!(matches!(
            verify_contact_lifecycle_receipt(&receipt),
            Err(ContactLifecycleReceiptVerificationError::Receipt(
                ContactLifecycleReceiptError::EndpointRevisionRegressed {
                    proxy: ProxyId(1),
                    domain: RevisionDomain::Generation,
                }
            ))
        ));
    }

}
