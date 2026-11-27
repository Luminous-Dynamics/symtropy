//! Revision-bound contact freshness decisions for the broadphase research crate.
//!
//! A broadphase pair can stay active while its cached manifold becomes stale.
//! Conversely, a body can wake without changing the geometry of an existing
//! contact. This module keeps three decisions separate:
//!
//! - geometry/manifold reuse;
//! - solver warm-start reuse;
//! - solver-island reactivation.
//!
//! The stamp is written by the contact-producing side after successful
//! narrowphase work. Broadphase code must not manufacture or self-authorize
//! these stamps. The caller remains responsible for proving that its supplied
//! endpoint/global revisions came from the authoritative world mutation log.

use super::PairDeltaFrame;
use incremental_aabb_tree_reference::{ProxyId, ProxyPair};
use std::collections::{BTreeMap, BTreeSet};

/// Revision domains attached to one generation-stable collider identity.
///
/// Revisions must be monotonic for one proxy generation. A new generation
/// starts a new identity lifetime and invalidates all prior contact state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EndpointRevisions {
    pub proxy: ProxyId,
    pub generation: u64,
    pub transform: u64,
    pub shape: u64,
    pub eligibility: u64,
    pub motion_class: u64,
    pub material: u64,
    pub wake_epoch: u64,
}

/// Global inputs that can change contact interpretation for every pair.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlobalContactRevisions {
    pub world: u64,
    pub collision_policy: u64,
    /// SHA-256 digest of the narrowphase algorithm/configuration contract.
    pub narrowphase_config_sha256: [u8; 32],
    /// SHA-256 digest of the solver algorithm/configuration contract.
    pub solver_config_sha256: [u8; 32],
}

/// The exact revision tuple under which a contact manifold was last produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContactFreshnessStamp {
    pub pair: ProxyPair,
    pub global: GlobalContactRevisions,
    pub left: EndpointRevisions,
    pub right: EndpointRevisions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RevisionDomain {
    World,
    CollisionPolicy,
    Generation,
    Transform,
    Shape,
    Eligibility,
    MotionClass,
    Material,
    WakeEpoch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactFreshnessError {
    PairIdentityMismatch,
    WorldRevisionRegressed,
    WorldRevisionDidNotAdvanceForMutation,
    CollisionPolicyRevisionRegressed,
    EndpointRevisionRegressed {
        proxy: ProxyId,
        domain: RevisionDomain,
    },
    GeometryRefreshNotCommitted,
    WarmStartResetNotCommitted,
    SolverActivationNotCommitted,
    PairEligibilityRecheckNotCommitted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FreshnessReason {
    ProxyGenerationChanged,
    TransformChanged,
    ShapeChanged,
    CollisionEligibilityChanged,
    MotionClassChanged,
    CollisionPolicyChanged,
    NarrowphaseConfigurationChanged,
    ContactMaterialChanged,
    SolverConfigurationChanged,
    WakeEpochAdvanced,
    NewPair,
    MissingContactStamp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReuseDisposition {
    Reusable,
    RefreshRequired,
}

/// Independent decisions for manifold geometry, warm-start impulses and wake.
///
/// Wake-only changes deliberately do not invalidate geometry or warm-start
/// state. The solver island must be reactivated, but the existing manifold may
/// remain useful. A geometry or solver-input revision change is conservative:
/// the relevant cached data is not reused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactReuseDecision {
    pub geometry: ReuseDisposition,
    pub warm_start: ReuseDisposition,
    pub pair_eligibility_recheck_required: bool,
    pub solver_activation_required: bool,
    pub reasons: Vec<FreshnessReason>,
}

impl ContactFreshnessStamp {
    /// Capture a stamp after a successful narrowphase/contact update.
    pub fn new(
        pair: ProxyPair,
        global: GlobalContactRevisions,
        left: EndpointRevisions,
        right: EndpointRevisions,
    ) -> Result<Self, ContactFreshnessError> {
        if pair.0 != left.proxy || pair.1 != right.proxy {
            return Err(ContactFreshnessError::PairIdentityMismatch);
        }
        Ok(Self {
            pair,
            global,
            left,
            right,
        })
    }

    /// Decide which cached contact data may be reused for the supplied world
    /// state. Any revision regression is an error and must fail closed.
    ///
    /// A world revision may advance by more than one while this pair remains
    /// untouched. Reuse therefore depends on exact endpoint/global revision
    /// equality and a trustworthy complete mutation journal, not on requiring
    /// every pair stamp to be rewritten every frame.
    pub fn assess(
        &self,
        global: GlobalContactRevisions,
        left: EndpointRevisions,
        right: EndpointRevisions,
    ) -> Result<ContactReuseDecision, ContactFreshnessError> {
        if self.pair.0 != left.proxy || self.pair.1 != right.proxy {
            return Err(ContactFreshnessError::PairIdentityMismatch);
        }
        if global.world < self.global.world {
            return Err(ContactFreshnessError::WorldRevisionRegressed);
        }
        if global.collision_policy < self.global.collision_policy {
            return Err(ContactFreshnessError::CollisionPolicyRevisionRegressed);
        }

        let mut reasons = BTreeSet::new();
        let mut geometry_reusable = true;
        let mut warm_start_reusable = true;
        let mut pair_eligibility_recheck_required = false;
        let mut solver_activation_required = false;

        for (previous, current) in [(self.left, left), (self.right, right)] {
            if current.generation < previous.generation {
                return Err(ContactFreshnessError::EndpointRevisionRegressed {
                    proxy: current.proxy,
                    domain: RevisionDomain::Generation,
                });
            }

            if current.generation != previous.generation {
                reasons.insert(FreshnessReason::ProxyGenerationChanged);
                geometry_reusable = false;
                warm_start_reusable = false;
                pair_eligibility_recheck_required = true;
                // Counters for a recycled slot belong to a new identity
                // lifetime and must not be compared with the old generation.
                continue;
            }

            check_revision(
                previous.transform,
                current.transform,
                current.proxy,
                RevisionDomain::Transform,
            )?;
            if current.transform != previous.transform {
                reasons.insert(FreshnessReason::TransformChanged);
                geometry_reusable = false;
            }

            check_revision(
                previous.shape,
                current.shape,
                current.proxy,
                RevisionDomain::Shape,
            )?;
            if current.shape != previous.shape {
                reasons.insert(FreshnessReason::ShapeChanged);
                geometry_reusable = false;
            }

            check_revision(
                previous.eligibility,
                current.eligibility,
                current.proxy,
                RevisionDomain::Eligibility,
            )?;
            if current.eligibility != previous.eligibility {
                reasons.insert(FreshnessReason::CollisionEligibilityChanged);
                geometry_reusable = false;
                pair_eligibility_recheck_required = true;
            }

            check_revision(
                previous.motion_class,
                current.motion_class,
                current.proxy,
                RevisionDomain::MotionClass,
            )?;
            if current.motion_class != previous.motion_class {
                reasons.insert(FreshnessReason::MotionClassChanged);
                geometry_reusable = false;
                pair_eligibility_recheck_required = true;
            }

            check_revision(
                previous.material,
                current.material,
                current.proxy,
                RevisionDomain::Material,
            )?;
            if current.material != previous.material {
                reasons.insert(FreshnessReason::ContactMaterialChanged);
                warm_start_reusable = false;
            }

            check_revision(
                previous.wake_epoch,
                current.wake_epoch,
                current.proxy,
                RevisionDomain::WakeEpoch,
            )?;
            if current.wake_epoch != previous.wake_epoch {
                reasons.insert(FreshnessReason::WakeEpochAdvanced);
                solver_activation_required = true;
            }
        }

        if global.collision_policy != self.global.collision_policy {
            reasons.insert(FreshnessReason::CollisionPolicyChanged);
            geometry_reusable = false;
            pair_eligibility_recheck_required = true;
        }
        if global.narrowphase_config_sha256 != self.global.narrowphase_config_sha256 {
            reasons.insert(FreshnessReason::NarrowphaseConfigurationChanged);
            geometry_reusable = false;
        }
        if global.solver_config_sha256 != self.global.solver_config_sha256 {
            reasons.insert(FreshnessReason::SolverConfigurationChanged);
            warm_start_reusable = false;
        }

        if !reasons.is_empty() && global.world == self.global.world {
            return Err(ContactFreshnessError::WorldRevisionDidNotAdvanceForMutation);
        }

        if !geometry_reusable {
            warm_start_reusable = false;
        }

        Ok(ContactReuseDecision {
            geometry: if geometry_reusable {
                ReuseDisposition::Reusable
            } else {
                ReuseDisposition::RefreshRequired
            },
            warm_start: if warm_start_reusable {
                ReuseDisposition::Reusable
            } else {
                ReuseDisposition::RefreshRequired
            },
            pair_eligibility_recheck_required,
            solver_activation_required,
            reasons: reasons.into_iter().collect(),
        })
    }

    /// Commit a new freshness stamp only after all obligations in the current
    /// assessment have actually completed. The caller must not set a flag
    /// merely because work was scheduled: geometry, warm-start reset,
    /// eligibility recheck and solver activation must each have completed.
    ///
    /// This single stamp records the now-current state after all required
    /// actions have been performed. It avoids repeatedly invalidating a
    /// retained manifold after a material/solver-only warm-start reset, while
    /// preventing premature stamp advancement.
    pub fn commit_transition(
        &self,
        global: GlobalContactRevisions,
        left: EndpointRevisions,
        right: EndpointRevisions,
        geometry_refreshed: bool,
        warm_start_reset: bool,
        pair_eligibility_rechecked: bool,
        solver_activated: bool,
    ) -> Result<Self, ContactFreshnessError> {
        let decision = self.assess(global, left, right)?;
        if decision.geometry == ReuseDisposition::RefreshRequired && !geometry_refreshed {
            return Err(ContactFreshnessError::GeometryRefreshNotCommitted);
        }
        if decision.warm_start == ReuseDisposition::RefreshRequired && !warm_start_reset {
            return Err(ContactFreshnessError::WarmStartResetNotCommitted);
        }
        if decision.pair_eligibility_recheck_required && !pair_eligibility_rechecked {
            return Err(ContactFreshnessError::PairEligibilityRecheckNotCommitted);
        }
        if decision.solver_activation_required && !solver_activated {
            return Err(ContactFreshnessError::SolverActivationNotCommitted);
        }
        Self::new(self.pair, global, left, right)
    }
}

/// Meaning of one contact-manager action produced from a pair-delta frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLifecycleActionKind {
    Added,
    Removed,
    Revalidated,
}

/// A pair event translated into a contact-manager obligation.
///
/// Added pairs require a new narrowphase/contact build. Removed pairs retire
/// existing contact state. Revalidated pairs carry a revision-based reuse
/// decision, or require a full contact build if no prior stamp exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactLifecycleAction {
    pub pair: ProxyPair,
    pub kind: ContactLifecycleActionKind,
    pub decision: Option<ContactReuseDecision>,
}

/// Deterministic contact-manager work plan for one broadphase delta frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContactLifecyclePlan {
    /// Actions are grouped in stable order: added, removed, then revalidated.
    pub actions: Vec<ContactLifecycleAction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactLifecycleError {
    NonCanonicalEventList,
    ConflictingPairEvents,
    UnexpectedStampForNewPair(ProxyPair),
    MissingEndpointRevision(ProxyId),
    UnexplainedRevalidation(ProxyPair),
    Freshness(ContactFreshnessError),
}

impl From<ContactFreshnessError> for ContactLifecycleError {
    fn from(value: ContactFreshnessError) -> Self {
        Self::Freshness(value)
    }
}

/// Translate broadphase membership/revalidation events into contact work.
///
/// The caller supplies the *previously committed* contact stamps and the
/// current authoritative endpoint revision map. This function does not mutate
/// the contact cache and does not certify journal provenance. It validates
/// that event streams are canonical and disjoint, then produces explicit
/// contact obligations for a downstream contact manager.
pub fn plan_contact_lifecycle(
    delta: &PairDeltaFrame,
    previous_stamps: &BTreeMap<ProxyPair, ContactFreshnessStamp>,
    current_global: GlobalContactRevisions,
    current_endpoints: &BTreeMap<ProxyId, EndpointRevisions>,
) -> Result<ContactLifecyclePlan, ContactLifecycleError> {
    let mut seen = BTreeSet::new();
    for stream in [&delta.added, &delta.removed, &delta.revalidated] {
        let mut previous = None;
        for pair in stream {
            if pair.0 >= pair.1 || previous.is_some_and(|last| last >= *pair) {
                return Err(ContactLifecycleError::NonCanonicalEventList);
            }
            if !seen.insert(*pair) {
                return Err(ContactLifecycleError::ConflictingPairEvents);
            }
            previous = Some(*pair);
        }
    }

    let mut plan = ContactLifecyclePlan::default();

    for pair in &delta.added {
        if previous_stamps.contains_key(pair) {
            return Err(ContactLifecycleError::UnexpectedStampForNewPair(*pair));
        }
        let left = current_endpoints
            .get(&pair.0)
            .copied()
            .ok_or(ContactLifecycleError::MissingEndpointRevision(pair.0))?;
        let right = current_endpoints
            .get(&pair.1)
            .copied()
            .ok_or(ContactLifecycleError::MissingEndpointRevision(pair.1))?;
        if left.proxy != pair.0 || right.proxy != pair.1 {
            return Err(ContactLifecycleError::Freshness(
                ContactFreshnessError::PairIdentityMismatch,
            ));
        }
        plan.actions.push(ContactLifecycleAction {
            pair: *pair,
            kind: ContactLifecycleActionKind::Added,
            decision: Some(ContactReuseDecision {
                geometry: ReuseDisposition::RefreshRequired,
                warm_start: ReuseDisposition::RefreshRequired,
                pair_eligibility_recheck_required: true,
                solver_activation_required: true,
                reasons: vec![FreshnessReason::NewPair],
            }),
        });
    }

    for pair in &delta.removed {
        plan.actions.push(ContactLifecycleAction {
            pair: *pair,
            kind: ContactLifecycleActionKind::Removed,
            decision: None,
        });
    }

    for pair in &delta.revalidated {
        let left = current_endpoints
            .get(&pair.0)
            .copied()
            .ok_or(ContactLifecycleError::MissingEndpointRevision(pair.0))?;
        let right = current_endpoints
            .get(&pair.1)
            .copied()
            .ok_or(ContactLifecycleError::MissingEndpointRevision(pair.1))?;
        if left.proxy != pair.0 {
            return Err(ContactLifecycleError::Freshness(
                ContactFreshnessError::PairIdentityMismatch,
            ));
        }
        if right.proxy != pair.1 {
            return Err(ContactLifecycleError::Freshness(
                ContactFreshnessError::PairIdentityMismatch,
            ));
        }

        let decision = match previous_stamps.get(pair) {
            Some(stamp) => {
                let decision = stamp.assess(current_global, left, right)?;
                if decision.reasons.is_empty() {
                    return Err(ContactLifecycleError::UnexplainedRevalidation(*pair));
                }
                decision
            }
            None => ContactReuseDecision {
                geometry: ReuseDisposition::RefreshRequired,
                warm_start: ReuseDisposition::RefreshRequired,
                pair_eligibility_recheck_required: true,
                solver_activation_required: false,
                reasons: vec![FreshnessReason::MissingContactStamp],
            },
        };
        plan.actions.push(ContactLifecycleAction {
            pair: *pair,
            kind: ContactLifecycleActionKind::Revalidated,
            decision: Some(decision),
        });
    }

    Ok(plan)
}

fn check_revision(
    previous: u64,
    current: u64,
    proxy: ProxyId,
    domain: RevisionDomain,
) -> Result<(), ContactFreshnessError> {
    if current < previous {
        return Err(ContactFreshnessError::EndpointRevisionRegressed { proxy, domain });
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
        .expect("canonical pair identity")
    }

    #[test]
    fn unchanged_revisions_reuse_geometry_and_warm_start() {
        let stamp = stamp();
        let decision = stamp
            .assess(global(), endpoint(1), endpoint(2))
            .expect("monotonic unchanged revisions");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::Reusable);
        assert!(!decision.solver_activation_required);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn wake_only_change_reactivates_solver_without_discarding_manifold() {
        let stamp = stamp();
        let mut awake = endpoint(1);
        awake.wake_epoch += 1;
        let decision = stamp
            .assess(next_global(), awake, endpoint(2))
            .expect("wake epoch advances monotonically");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::Reusable);
        assert!(decision.solver_activation_required);
        assert_eq!(decision.reasons, vec![FreshnessReason::WakeEpochAdvanced]);
    }

    #[test]
    fn transform_change_refreshes_manifold_and_warm_start() {
        let stamp = stamp();
        let mut moved = endpoint(1);
        moved.transform += 1;
        let decision = stamp
            .assess(next_global(), moved, endpoint(2))
            .expect("transform advances monotonically");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.reasons, vec![FreshnessReason::TransformChanged]);
    }

    #[test]
    fn material_change_preserves_geometry_but_rejects_warm_start() {
        let stamp = stamp();
        let mut material = endpoint(2);
        material.material += 1;
        let decision = stamp
            .assess(next_global(), endpoint(1), material)
            .expect("material advances monotonically");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(
            decision.reasons,
            vec![FreshnessReason::ContactMaterialChanged]
        );
    }

    #[test]
    fn filter_eligibility_change_requires_pair_policy_recheck() {
        let stamp = stamp();
        let mut changed = endpoint(1);
        changed.eligibility += 1;
        let decision = stamp
            .assess(next_global(), changed, endpoint(2))
            .expect("eligibility advances monotonically");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert!(decision.pair_eligibility_recheck_required);
        assert_eq!(
            decision.reasons,
            vec![FreshnessReason::CollisionEligibilityChanged]
        );
    }

    #[test]
    fn shape_change_invalidates_manifold_even_when_transform_is_stable() {
        let stamp = stamp();
        let mut changed = endpoint(2);
        changed.shape += 1;
        let decision = stamp
            .assess(next_global(), endpoint(1), changed)
            .expect("shape advances monotonically");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.reasons, vec![FreshnessReason::ShapeChanged]);
    }

    #[test]
    fn generation_change_never_aliases_old_contact_state() {
        let stamp = stamp();
        let mut recycled = endpoint(1);
        recycled.generation += 1;
        recycled.transform = 0;
        let decision = stamp
            .assess(next_global(), recycled, endpoint(2))
            .expect("new generation is a new identity lifetime");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(
            decision.reasons,
            vec![FreshnessReason::ProxyGenerationChanged]
        );
    }

    #[test]
    fn world_revision_regression_fails_closed() {
        let stamp = stamp();
        let mut older = global();
        older.world -= 1;
        assert_eq!(
            stamp.assess(older, endpoint(1), endpoint(2)),
            Err(ContactFreshnessError::WorldRevisionRegressed)
        );
    }

    #[test]
    fn endpoint_revision_regression_fails_closed() {
        let stamp = stamp();
        let mut older = endpoint(2);
        older.shape -= 1;
        assert_eq!(
            stamp.assess(next_global(), endpoint(1), older),
            Err(ContactFreshnessError::EndpointRevisionRegressed {
                proxy: ProxyId(2),
                domain: RevisionDomain::Shape,
            })
        );
    }

    #[test]
    fn narrowphase_and_solver_configuration_changes_are_separate() {
        let stamp = stamp();
        let mut changed = global();
        changed.world += 1;
        changed.narrowphase_config_sha256 = [13; 32];
        let decision = stamp
            .assess(changed, endpoint(1), endpoint(2))
            .expect("configuration digest change");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(
            decision.reasons,
            vec![FreshnessReason::NarrowphaseConfigurationChanged]
        );

        let mut solver_changed = global();
        solver_changed.world += 1;
        solver_changed.solver_config_sha256 = [14; 32];
        let decision = stamp
            .assess(solver_changed, endpoint(1), endpoint(2))
            .expect("solver digest change");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(
            decision.reasons,
            vec![FreshnessReason::SolverConfigurationChanged]
        );
    }

    #[test]
    fn motion_class_and_global_filter_changes_request_eligibility_recheck() {
        let stamp = stamp();
        let mut changed_endpoint = endpoint(1);
        changed_endpoint.motion_class += 1;
        let decision = stamp
            .assess(next_global(), changed_endpoint, endpoint(2))
            .expect("motion class advances");
        assert!(decision.pair_eligibility_recheck_required);
        assert_eq!(decision.reasons, vec![FreshnessReason::MotionClassChanged]);

        let mut changed_global = global();
        changed_global.world += 1;
        changed_global.collision_policy += 1;
        let decision = stamp
            .assess(changed_global, endpoint(1), endpoint(2))
            .expect("global policy advances");
        assert!(decision.pair_eligibility_recheck_required);
        assert_eq!(
            decision.reasons,
            vec![FreshnessReason::CollisionPolicyChanged]
        );
    }

    #[test]
    fn pair_delta_wake_only_plan_preserves_contact_and_warm_start() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);

        let mut stamps = BTreeMap::new();
        stamps.insert(pair, stamp());
        let mut endpoints = BTreeMap::new();
        let mut awake = endpoint(1);
        awake.wake_epoch += 1;
        endpoints.insert(ProxyId(1), awake);
        endpoints.insert(ProxyId(2), endpoint(2));

        let plan = plan_contact_lifecycle(&delta, &stamps, next_global(), &endpoints)
            .expect("complete pair revisions");
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(
            plan.actions[0].kind,
            ContactLifecycleActionKind::Revalidated
        );
        let decision = plan.actions[0].decision.as_ref().expect("decision");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::Reusable);
        assert!(decision.solver_activation_required);
    }

    #[test]
    fn new_pair_requires_geometry_and_solver_activation() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.added.push(pair);
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        let plan = plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &endpoints)
            .expect("new pair has current endpoint revisions");
        let decision = plan.actions[0]
            .decision
            .as_ref()
            .expect("new pair decision");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert!(decision.pair_eligibility_recheck_required);
        assert!(decision.solver_activation_required);
        assert_eq!(decision.reasons, vec![FreshnessReason::NewPair]);
    }

    #[test]
    fn removed_pair_retires_contact_state_without_current_endpoint_lookup() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.removed.push(pair);
        let plan = plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &BTreeMap::new())
            .expect("removed endpoints need not exist in current world");
        assert_eq!(
            plan.actions,
            vec![ContactLifecycleAction {
                pair,
                kind: ContactLifecycleActionKind::Removed,
                decision: None,
            }]
        );
    }

    #[test]
    fn revalidated_pair_without_stamp_requires_full_contact_build() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        let plan = plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &endpoints)
            .expect("endpoint revisions are complete");
        let decision = plan.actions[0]
            .decision
            .as_ref()
            .expect("missing stamp decision");
        assert_eq!(decision.geometry, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.warm_start, ReuseDisposition::RefreshRequired);
        assert_eq!(decision.reasons, vec![FreshnessReason::MissingContactStamp]);
    }

    #[test]
    fn overlapping_pair_event_classes_are_rejected() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.added.push(pair);
        delta.removed.push(pair);
        assert_eq!(
            plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &BTreeMap::new()),
            Err(ContactLifecycleError::ConflictingPairEvents)
        );
    }

    #[test]
    fn noncanonical_pair_event_order_is_rejected() {
        let mut delta = PairDeltaFrame::default();
        delta.added.push(ProxyPair(ProxyId(2), ProxyId(1)));
        assert_eq!(
            plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &BTreeMap::new()),
            Err(ContactLifecycleError::NonCanonicalEventList)
        );
    }

    #[test]
    fn revalidated_pair_requires_both_endpoint_revision_records() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1))]);
        assert_eq!(
            plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &endpoints),
            Err(ContactLifecycleError::MissingEndpointRevision(ProxyId(2)))
        );
    }

    #[test]
    fn added_pair_rejects_an_orphaned_prior_contact_stamp() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.added.push(pair);
        let mut stamps = BTreeMap::new();
        stamps.insert(pair, stamp());
        assert_eq!(
            plan_contact_lifecycle(&delta, &stamps, global(), &BTreeMap::new()),
            Err(ContactLifecycleError::UnexpectedStampForNewPair(pair))
        );
    }

    #[test]
    fn endpoint_mutation_without_world_revision_advance_fails_closed() {
        let stamp = stamp();
        let mut moved = endpoint(1);
        moved.transform += 1;
        assert_eq!(
            stamp.assess(global(), moved, endpoint(2)),
            Err(ContactFreshnessError::WorldRevisionDidNotAdvanceForMutation)
        );
    }

    #[test]
    fn added_pair_requires_current_endpoint_revision_records() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.added.push(pair);
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1))]);
        assert_eq!(
            plan_contact_lifecycle(&delta, &BTreeMap::new(), global(), &endpoints),
            Err(ContactLifecycleError::MissingEndpointRevision(ProxyId(2)))
        );
    }

    #[test]
    fn revalidation_without_revision_reason_fails_closed() {
        let pair = ProxyPair::new(ProxyId(1), ProxyId(2)).expect("distinct IDs");
        let mut delta = PairDeltaFrame::default();
        delta.revalidated.push(pair);
        let mut stamps = BTreeMap::new();
        stamps.insert(pair, stamp());
        let endpoints = BTreeMap::from([(ProxyId(1), endpoint(1)), (ProxyId(2), endpoint(2))]);
        assert_eq!(
            plan_contact_lifecycle(&delta, &stamps, next_global(), &endpoints),
            Err(ContactLifecycleError::UnexplainedRevalidation(pair))
        );
    }

    #[test]
    fn material_only_transition_commits_after_warm_start_reset_without_geometry_rebuild() {
        let stamp = stamp();
        let mut material = endpoint(2);
        material.material += 1;
        let next = next_global();
        assert_eq!(
            stamp.commit_transition(
                next,
                endpoint(1),
                material,
                false,
                false,
                false,
                false,
            ),
            Err(ContactFreshnessError::WarmStartResetNotCommitted)
        );
        let committed = stamp
            .commit_transition(
                next,
                endpoint(1),
                material,
                false,
                true,
                false,
                false,
            )
            .expect("warm-start reset completed");
        let next_step = GlobalContactRevisions {
            world: next.world + 1,
            ..next
        };
        let decision = committed
            .assess(next_step, endpoint(1), material)
            .expect("committed material revision is current");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::Reusable);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn wake_only_transition_commits_only_after_solver_activation() {
        let stamp = stamp();
        let mut awake = endpoint(1);
        awake.wake_epoch += 1;
        let next = next_global();
        assert_eq!(
            stamp.commit_transition(
                next,
                awake,
                endpoint(2),
                false,
                false,
                false,
                false,
            ),
            Err(ContactFreshnessError::SolverActivationNotCommitted)
        );
        let committed = stamp
            .commit_transition(
                next,
                awake,
                endpoint(2),
                false,
                false,
                false,
                true,
            )
            .expect("solver island has been reactivated");
        let next_step = GlobalContactRevisions {
            world: next.world + 1,
            ..next
        };
        let decision = committed
            .assess(next_step, awake, endpoint(2))
            .expect("wake revision was committed");
        assert_eq!(decision.geometry, ReuseDisposition::Reusable);
        assert_eq!(decision.warm_start, ReuseDisposition::Reusable);
        assert!(!decision.solver_activation_required);
    }

    #[test]
    fn geometry_and_warm_start_must_both_commit_after_transform_change() {
        let stamp = stamp();
        let mut moved = endpoint(1);
        moved.transform += 1;
        assert_eq!(
            stamp.commit_transition(
                next_global(),
                moved,
                endpoint(2),
                true,
                false,
                false,
                false,
            ),
            Err(ContactFreshnessError::WarmStartResetNotCommitted)
        );
    }

    #[test]
    fn filter_change_requires_eligibility_recheck_before_stamp_advance() {
        let stamp = stamp();
        let mut filtered = endpoint(1);
        filtered.eligibility += 1;
        let mut next = next_global();
        next.collision_policy += 1;
        assert_eq!(
            stamp.commit_transition(
                next,
                filtered,
                endpoint(2),
                true,
                true,
                false,
                false,
            ),
            Err(ContactFreshnessError::PairEligibilityRecheckNotCommitted)
        );
        assert!(stamp
            .commit_transition(
                next,
                filtered,
                endpoint(2),
                true,
                true,
                true,
                false,
            )
            .is_ok());
    }

}
