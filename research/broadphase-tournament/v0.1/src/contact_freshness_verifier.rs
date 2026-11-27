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
    ContactLifecycleActionKind, ContactLifecycleError, ContactLifecyclePlan,
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
    ActionCountMismatch {
        expected: usize,
        actual: usize,
    },
    ActionMismatch {
        index: usize,
    },
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
                    return Err(
                        ContactLifecycleError::UnexplainedRevalidation(*pair).into()
                    );
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

    audit_endpoint(stamp.left, left, &mut reasons, &mut geometry,
        &mut warm_start, &mut eligibility_recheck, &mut activate_solver)?;
    audit_endpoint(stamp.right, right, &mut reasons, &mut geometry,
        &mut warm_start, &mut eligibility_recheck, &mut activate_solver)?;

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
        return Err(
            ContactFreshnessError::WorldRevisionDidNotAdvanceForMutation.into()
        );
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
        return Err(ContactFreshnessError::EndpointRevisionRegressed {
            proxy,
            domain,
        }
        .into());
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

    fn stamp() -> ContactFreshnessStamp {
        ContactFreshnessStamp::new(
            ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs"),
            global(),
            endpoint(1),
            endpoint(2),
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

        let report = verify_contact_lifecycle_plan(
            &delta, &stamps, next_global(), &endpoints, &plan,
        )
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
        plan.actions[0].decision.as_mut().expect("decision").geometry =
            ReuseDisposition::RefreshRequired;

        assert_eq!(
            verify_contact_lifecycle_plan(
                &delta, &stamps, next_global(), &endpoints, &plan,
            ),
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
                &delta, &stamps, next_global(), &endpoints, &plan,
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
                &delta, &BTreeMap::new(), global(), &endpoints, &plan,
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
        plan.actions[0].decision.as_mut().expect("decision").geometry = ReuseDisposition::Reusable;

        assert_eq!(
            verify_contact_lifecycle_plan(
                &delta, &BTreeMap::new(), global(), &endpoints, &plan,
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
                &delta, &stamps, older, &endpoints, &ContactLifecyclePlan::default(),
            ),
            Err(ContactLifecycleVerificationError::Input(
                ContactLifecycleError::Freshness(ContactFreshnessError::WorldRevisionRegressed)
            ))
        );
    }
}
