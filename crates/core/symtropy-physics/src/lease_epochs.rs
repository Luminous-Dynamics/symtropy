// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Dependency-scoped revision tokens for replayable contact-lease invalidation.
//!
//! This module does not certify geometry, motion, CCD, or physical semantics.
//! It answers a narrower question: are the dependencies captured by an
//! existing lease still at the same revisions? Geometric and scene-discovery
//! receipts remain separate proof obligations.
//!
//! The intended composition is:
//! - this module detects stale dependency snapshots;
//! - geometric lease validation checks the current pair configuration;
//! - a horizon validator checks bounded future motion and event surfaces;
//! - a scene-discovery receipt establishes that newly approaching pairs were
//!   not omitted from the candidate set.
//!
//! Unknown or untracked mutations must not be represented by an unchanged
//! epoch. Callers must fail closed when they cannot advance or observe a
//! dependency revision.

use core::fmt;

/// Monotonic revision number for one dependency domain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(u64);

impl Epoch {
    /// Initial revision for a newly created dependency owner.
    pub const ZERO: Self = Self(0);

    /// Construct an epoch from a persisted or independently supplied value.
    ///
    /// Callers must not use this to reset an existing counter or reuse an
    /// epoch within one owner generation.
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Return the numeric revision.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next epoch, or an error rather than wrapping.
    pub const fn checked_next(self) -> Result<Self, EpochOverflow> {
        match self.0.checked_add(1) {
            Some(next) => Ok(Self(next)),
            None => Err(EpochOverflow),
        }
    }
}

/// A revision counter which becomes permanently unusable after overflow.
///
/// Once exhausted, current() returns None and further advances fail. The
/// caller must replace the owning generation or disable dependent reuse; it
/// must never wrap the counter and make an old receipt appear current again.
#[derive(Debug, PartialEq, Eq)]
pub struct RevisionCounter {
    current: Epoch,
    valid: bool,
}

impl Default for RevisionCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl RevisionCounter {
    /// Create a new counter for a newly established dependency owner.
    pub const fn new() -> Self {
        Self {
            current: Epoch::ZERO,
            valid: true,
        }
    }

    /// Restore a counter from its authoritative persisted revision.
    ///
    /// Only use this when the persisted owner generation and revision are
    /// authenticated by the surrounding authority. Otherwise, start a new
    /// world generation and do not reuse existing receipts.
    pub const fn from_persisted(initial: Epoch) -> Self {
        Self {
            current: initial,
            valid: true,
        }
    }

    /// Current revision, or None after exhaustion.
    pub const fn current(&self) -> Option<Epoch> {
        if self.valid { Some(self.current) } else { None }
    }

    /// Advance exactly once without wrapping.
    pub fn advance(&mut self) -> Result<Epoch, EpochOverflow> {
        if !self.valid {
            return Err(EpochOverflow);
        }

        match self.current.checked_next() {
            Ok(next) => {
                self.current = next;
                Ok(next)
            }
            Err(error) => {
                self.valid = false;
                Err(error)
            }
        }
    }
}

/// A revision epoch exhausted its representable range.
///
/// This is a fail-closed condition, not a request to reset the counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochOverflow;

impl fmt::Display for EpochOverflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("revision epoch overflowed; dependent evidence is invalid")
    }
}

impl std::error::Error for EpochOverflow {}

/// Stable identity for a body participating in a cached contact.
///
/// stable_id is a world-assigned identity, not an array index. generation
/// changes whenever that identity is retired and reissued. A runtime body
/// handle may be stored separately by the caller, but must not replace this
/// generation-bound identity in replay evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BodyIdentity {
    pub stable_id: u64,
    pub generation: u64,
}

impl BodyIdentity {
    pub const fn new(stable_id: u64, generation: u64) -> Self {
        Self {
            stable_id,
            generation,
        }
    }
}

/// Revisions owned by one body and relevant to contact reuse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyRevisions {
    /// Collider geometry, scale, topology, or provider identity changed.
    pub geometry: Epoch,
    /// A discontinuous pose change occurred (e.g. teleport or pose reset).
    ///
    /// Ordinary continuous motion does not increment this field; it is
    /// checked by the geometric certificate and motion-horizon model.
    pub pose_discontinuity: Epoch,
    /// Collision group/mask or pair-filter inputs changed.
    pub filter: Epoch,
    /// Friction, restitution, surface velocity, or related material inputs
    /// changed. Current-step values must still be read every step. If a
    /// material mutation changes the admitted motion envelope, the caller
    /// must also advance motion_bound.
    pub material: Epoch,
    /// Callback registration or callback-policy identity changed.
    pub callback: Epoch,
    /// The admitted force/impulse/constraint-motion envelope changed.
    pub motion_bound: Epoch,
}

/// Identity and dependency revisions for one body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyRevisionSnapshot {
    pub identity: BodyIdentity,
    pub revisions: BodyRevisions,
}

impl BodyRevisionSnapshot {
    pub const fn new(identity: BodyIdentity, revisions: BodyRevisions) -> Self {
        Self {
            identity,
            revisions,
        }
    }
}

/// Revisions owned by the scene/world rather than an individual contact pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneRevisions {
    /// World generation; changes on reset/reconstruction/adoption.
    pub world_generation: Epoch,
    /// Body membership and stable-identity allocation changed.
    pub membership: Epoch,
    /// Scene-level geometry/provider/topology changed.
    pub topology: Epoch,
    /// Scene-wide pair-filter policy changed.
    pub filter: Epoch,
    /// Scene-wide motion-envelope assumptions changed.
    pub motion_bound: Epoch,
}

/// Revisions for algorithms whose semantics are part of the proof profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProfileRevisions {
    /// Geometric-certificate algorithm, tolerance, or arithmetic profile.
    pub geometry_certificate: Epoch,
    /// Integrator/force/constraint motion model used by swept bounds.
    pub motion_model: Epoch,
    /// Constraint solver profile governing cached impulse interpretation.
    pub solver: Epoch,
}


/// A supported mutation of one body's lease dependencies.
///
/// This enum is the deterministic mapping input for the pure revision
/// transition below. Runtime world APIs must call the matching transition
/// whenever they own the mutation. Public/unobservable mutation paths must
/// return Untracked rather than pretending that no dependency changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyMutation {
    ColliderGeometryChanged,
    PoseDiscontinuity,
    CollisionFilterChanged,
    MaterialChanged,
    MaterialAndMotionBoundsChanged,
    CallbackPolicyChanged,
    CallbackAndMotionBoundsChanged,
    MotionBoundsChanged,
    Untracked,
}

/// A supported mutation of scene-level discovery dependencies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneMutation {
    BodyMembershipChanged,
    TopologyOrProviderChanged,
    PairFilterPolicyChanged,
    MotionBoundsChanged,
    WorldGenerationChanged,
    Untracked,
}

/// A supported change to a proof or solver profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileMutation {
    GeometryCertificateChanged,
    MotionModelChanged,
    SolverProfileChanged,
    Untracked,
}

/// Why a pure revision transition could not produce a current snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionMutationError {
    UntrackedMutation,
    EpochOverflow,
}

impl From<EpochOverflow> for RevisionMutationError {
    fn from(_: EpochOverflow) -> Self {
        Self::EpochOverflow
    }
}

fn next_epoch(epoch: Epoch) -> Result<Epoch, RevisionMutationError> {
    Ok(epoch.checked_next()?)
}

/// Apply the exact dependency revisions associated with one body mutation.
///
/// The input is copied and the updated snapshot is returned, so a failed
/// transition cannot partially mutate caller-owned state. The caller must
/// replace its authoritative snapshot only on success. Untracked mutation
/// explicitly fails and must disable dependent reuse.
pub fn advance_body_revisions(
    mut revisions: BodyRevisions,
    mutation: BodyMutation,
) -> Result<BodyRevisions, RevisionMutationError> {
    match mutation {
        BodyMutation::ColliderGeometryChanged => {
            revisions.geometry = next_epoch(revisions.geometry)?;
        }
        BodyMutation::PoseDiscontinuity => {
            revisions.pose_discontinuity = next_epoch(revisions.pose_discontinuity)?;
        }
        BodyMutation::CollisionFilterChanged => {
            revisions.filter = next_epoch(revisions.filter)?;
        }
        BodyMutation::MaterialChanged => {
            revisions.material = next_epoch(revisions.material)?;
        }
        BodyMutation::MaterialAndMotionBoundsChanged => {
            let material = next_epoch(revisions.material)?;
            let motion_bound = next_epoch(revisions.motion_bound)?;
            revisions.material = material;
            revisions.motion_bound = motion_bound;
        }
        BodyMutation::CallbackPolicyChanged => {
            revisions.callback = next_epoch(revisions.callback)?;
        }
        BodyMutation::CallbackAndMotionBoundsChanged => {
            let callback = next_epoch(revisions.callback)?;
            let motion_bound = next_epoch(revisions.motion_bound)?;
            revisions.callback = callback;
            revisions.motion_bound = motion_bound;
        }
        BodyMutation::MotionBoundsChanged => {
            revisions.motion_bound = next_epoch(revisions.motion_bound)?;
        }
        BodyMutation::Untracked => return Err(RevisionMutationError::UntrackedMutation),
    }

    Ok(revisions)
}

/// Apply the exact revision transition associated with a scene mutation.
pub fn advance_scene_revisions(
    mut revisions: SceneRevisions,
    mutation: SceneMutation,
) -> Result<SceneRevisions, RevisionMutationError> {
    match mutation {
        SceneMutation::BodyMembershipChanged => {
            revisions.membership = next_epoch(revisions.membership)?;
        }
        SceneMutation::TopologyOrProviderChanged => {
            revisions.topology = next_epoch(revisions.topology)?;
        }
        SceneMutation::PairFilterPolicyChanged => {
            revisions.filter = next_epoch(revisions.filter)?;
        }
        SceneMutation::MotionBoundsChanged => {
            revisions.motion_bound = next_epoch(revisions.motion_bound)?;
        }
        SceneMutation::WorldGenerationChanged => {
            revisions.world_generation = next_epoch(revisions.world_generation)?;
        }
        SceneMutation::Untracked => return Err(RevisionMutationError::UntrackedMutation),
    }

    Ok(revisions)
}

/// Apply the exact revision transition associated with a proof/profile change.
pub fn advance_profile_revisions(
    mut revisions: ProfileRevisions,
    mutation: ProfileMutation,
) -> Result<ProfileRevisions, RevisionMutationError> {
    match mutation {
        ProfileMutation::GeometryCertificateChanged => {
            revisions.geometry_certificate = next_epoch(revisions.geometry_certificate)?;
        }
        ProfileMutation::MotionModelChanged => {
            revisions.motion_model = next_epoch(revisions.motion_model)?;
        }
        ProfileMutation::SolverProfileChanged => {
            revisions.solver = next_epoch(revisions.solver)?;
        }
        ProfileMutation::Untracked => return Err(RevisionMutationError::UntrackedMutation),
    }

    Ok(revisions)
}

/// Complete revision snapshot needed to evaluate one two-body contact lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaseRevisionSnapshot {
    pub body_a: BodyRevisionSnapshot,
    pub body_b: BodyRevisionSnapshot,
    pub scene: SceneRevisions,
    pub profiles: ProfileRevisions,
}

impl LeaseRevisionSnapshot {
    pub const fn new(
        body_a: BodyRevisionSnapshot,
        body_b: BodyRevisionSnapshot,
        scene: SceneRevisions,
        profiles: ProfileRevisions,
    ) -> Self {
        Self {
            body_a,
            body_b,
            scene,
            profiles,
        }
    }

    fn canonicalized(mut self) -> Self {
        if self.body_b.identity < self.body_a.identity {
            core::mem::swap(&mut self.body_a, &mut self.body_b);
        }
        self
    }
}

/// Immutable dependency read-set captured when a lease is issued or renewed.
///
/// This snapshot is not itself a geometric proof. It is the dependency
/// baseline against which current revisions are checked before the geometric
/// and temporal validators are allowed to consider reuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaseReadSet {
    captured: LeaseRevisionSnapshot,
}

impl LeaseReadSet {
    /// Capture a canonical, body-order-independent dependency read-set.
    pub fn capture(snapshot: LeaseRevisionSnapshot) -> Self {
        Self {
            captured: snapshot.canonicalized(),
        }
    }

    /// Revalidate captured dependencies against the current world snapshot.
    ///
    /// The result separates four decisions that must not be conflated:
    /// - geometry lease eligibility (still requires a geometric predicate);
    /// - future-horizon eligibility (also requires motion and scene evidence);
    /// - scene-discovery receipt currentness;
    /// - solver warm-start eligibility.
    ///
    /// current_step_semantics_changed is only a change indicator. It does
    /// not authorize reuse of old material or callback outputs: current
    /// semantics must be evaluated on every applicable step regardless.
    pub fn revalidate(&self, current: LeaseRevisionSnapshot) -> LeaseRevalidation {
        let captured = self.captured;
        let current = current.canonicalized();
        let mut reasons = RevocationReasons::default();

        let world_generation_current =
            captured.scene.world_generation == current.scene.world_generation;
        if !world_generation_current {
            reasons.insert(RevocationReason::WorldGenerationChanged);
        }

        let identities_current = captured.body_a.identity == current.body_a.identity
            && captured.body_b.identity == current.body_b.identity;

        if !identities_current {
            reasons.insert(RevocationReason::BodyIdentityChanged);
        }

        let geometry_revisions_current = captured.body_a.revisions.geometry
            == current.body_a.revisions.geometry
            && captured.body_b.revisions.geometry == current.body_b.revisions.geometry;

        if !geometry_revisions_current {
            reasons.insert(RevocationReason::ShapeOrTopologyChanged);
        }

        let discontinuities_current = captured.body_a.revisions.pose_discontinuity
            == current.body_a.revisions.pose_discontinuity
            && captured.body_b.revisions.pose_discontinuity
                == current.body_b.revisions.pose_discontinuity;

        if !discontinuities_current {
            reasons.insert(RevocationReason::TransformDiscontinuity);
        }

        let pair_filters_current = captured.body_a.revisions.filter
            == current.body_a.revisions.filter
            && captured.body_b.revisions.filter == current.body_b.revisions.filter
            && captured.scene.filter == current.scene.filter;

        if !pair_filters_current {
            reasons.insert(RevocationReason::FilterChanged);
        }

        let material_current = captured.body_a.revisions.material
            == current.body_a.revisions.material
            && captured.body_b.revisions.material == current.body_b.revisions.material;

        if !material_current {
            reasons.insert(RevocationReason::MaterialSemanticsChanged);
        }

        let callback_current = captured.body_a.revisions.callback
            == current.body_a.revisions.callback
            && captured.body_b.revisions.callback == current.body_b.revisions.callback;

        if !callback_current {
            reasons.insert(RevocationReason::CallbackPolicyChanged);
        }

        let motion_bounds_current = captured.body_a.revisions.motion_bound
            == current.body_a.revisions.motion_bound
            && captured.body_b.revisions.motion_bound == current.body_b.revisions.motion_bound
            && captured.scene.motion_bound == current.scene.motion_bound
            && captured.profiles.motion_model == current.profiles.motion_model;

        if !motion_bounds_current {
            reasons.insert(RevocationReason::MotionBoundChanged);
        }

        let certificate_profile_current =
            captured.profiles.geometry_certificate == current.profiles.geometry_certificate;

        if !certificate_profile_current {
            reasons.insert(RevocationReason::CertificateProfileChanged);
        }

        let solver_profile_current = captured.profiles.solver == current.profiles.solver;

        if !solver_profile_current {
            reasons.insert(RevocationReason::SolverProfileChanged);
        }

        let scene_structure_current = world_generation_current
            && captured.scene.membership == current.scene.membership
            && captured.scene.topology == current.scene.topology;

        let scene_discovery_current = identities_current
            && scene_structure_current
            && geometry_revisions_current
            && discontinuities_current
            && pair_filters_current
            && callback_current
            && motion_bounds_current
            && solver_profile_current;

        if !scene_discovery_current {
            reasons.insert(RevocationReason::SceneReceiptStale);
        }

        let geometry_lease_eligible = identities_current
            && world_generation_current
            && geometry_revisions_current
            && discontinuities_current
            && certificate_profile_current;

        let horizon_eligible =
            geometry_lease_eligible && scene_discovery_current && motion_bounds_current;

        let current_step_semantics_changed =
            !material_current || !callback_current || !solver_profile_current;

        let solver_state_reusable = geometry_lease_eligible
            && solver_profile_current
            && material_current
            && callback_current
            && pair_filters_current;

        LeaseRevalidation {
            geometry_lease_eligible,
            horizon_eligible,
            scene_discovery_current,
            pair_filters_current,
            current_step_semantics_changed,
            solver_state_reusable,
            reasons,
        }
    }
}

/// Stable reason codes for a dependency mismatch.
///
/// Numeric values are intentionally stable so an enclosing evidence format
/// can persist them. Do not reorder or reuse assigned values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RevocationReason {
    BodyIdentityChanged = 0,
    ShapeOrTopologyChanged = 1,
    TransformDiscontinuity = 2,
    FilterChanged = 3,
    MaterialSemanticsChanged = 4,
    CallbackPolicyChanged = 5,
    MotionBoundChanged = 6,
    SceneReceiptStale = 7,
    SolverProfileChanged = 8,
    CertificateProfileChanged = 9,
    WorldGenerationChanged = 10,
}

/// Compact, allocation-free set of stable revocation reasons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RevocationReasons(u32);

impl RevocationReasons {
    const ALL: [RevocationReason; 11] = [
        RevocationReason::BodyIdentityChanged,
        RevocationReason::ShapeOrTopologyChanged,
        RevocationReason::TransformDiscontinuity,
        RevocationReason::FilterChanged,
        RevocationReason::MaterialSemanticsChanged,
        RevocationReason::CallbackPolicyChanged,
        RevocationReason::MotionBoundChanged,
        RevocationReason::SceneReceiptStale,
        RevocationReason::SolverProfileChanged,
        RevocationReason::CertificateProfileChanged,
        RevocationReason::WorldGenerationChanged,
    ];

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, reason: RevocationReason) -> bool {
        self.0 & (1u32 << (reason as u8)) != 0
    }

    fn insert(&mut self, reason: RevocationReason) {
        self.0 |= 1u32 << (reason as u8);
    }

    /// Iterate in stable numeric-code order without allocating.
    pub fn iter(self) -> impl Iterator<Item = RevocationReason> {
        Self::ALL
            .into_iter()
            .filter(move |reason| self.contains(*reason))
    }
}

/// Layer-specific result of checking a captured dependency read-set.
///
/// geometry_lease_eligible says only that captured identities, geometry
/// revisions, pose-discontinuity revisions and certificate profile remain
/// current. The geometric certificate must still evaluate actual
/// transform/feature/separation predicates.
///
/// horizon_eligible additionally requires current scene-discovery and
/// motion-model dependencies. It does not replace a swept-pair completeness
/// proof; scene_discovery_current only checks whether the captured receipt's
/// declared dependencies have changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaseRevalidation {
    pub geometry_lease_eligible: bool,
    pub horizon_eligible: bool,
    pub scene_discovery_current: bool,
    pub pair_filters_current: bool,
    pub current_step_semantics_changed: bool,
    pub solver_state_reusable: bool,
    pub reasons: RevocationReasons,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> LeaseRevisionSnapshot {
        let body_a = BodyRevisionSnapshot::new(BodyIdentity::new(10, 1), BodyRevisions::default());
        let body_b = BodyRevisionSnapshot::new(BodyIdentity::new(20, 4), BodyRevisions::default());

        LeaseRevisionSnapshot::new(
            body_a,
            body_b,
            SceneRevisions::default(),
            ProfileRevisions::default(),
        )
    }

    #[test]
    fn identical_revision_snapshot_preserves_all_dependency_layers() {
        let captured = LeaseReadSet::capture(snapshot());
        let result = captured.revalidate(snapshot());

        assert!(result.geometry_lease_eligible);
        assert!(result.horizon_eligible);
        assert!(result.scene_discovery_current);
        assert!(result.pair_filters_current);
        assert!(!result.current_step_semantics_changed);
        assert!(result.solver_state_reusable);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn body_order_permutation_does_not_change_revalidation() {
        let original = snapshot();
        let captured = LeaseReadSet::capture(original);
        let permuted = LeaseRevisionSnapshot::new(
            original.body_b,
            original.body_a,
            original.scene,
            original.profiles,
        );
        let result = captured.revalidate(permuted);

        assert!(result.geometry_lease_eligible);
        assert!(result.horizon_eligible);
        assert!(result.scene_discovery_current);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn material_change_preserves_geometry_but_invalidates_solver_state() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.body_a.revisions.material = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(result.geometry_lease_eligible);
        assert!(result.horizon_eligible);
        assert!(result.scene_discovery_current);
        assert!(result.pair_filters_current);
        assert!(result.current_step_semantics_changed);
        assert!(!result.solver_state_reusable);
        assert!(
            result
                .reasons
                .contains(RevocationReason::MaterialSemanticsChanged)
        );
    }

    #[test]
    fn callback_change_preserves_pair_geometry_but_revokes_horizon_and_solver_state() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.body_b.revisions.callback = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(!result.scene_discovery_current);
        assert!(result.current_step_semantics_changed);
        assert!(!result.solver_state_reusable);
        assert!(
            result
                .reasons
                .contains(RevocationReason::CallbackPolicyChanged)
        );
        assert!(result.reasons.contains(RevocationReason::SceneReceiptStale));
    }

    #[test]
    fn filter_change_requires_pair_filter_and_scene_rediscovery() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.body_a.revisions.filter = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(result.geometry_lease_eligible);
        assert!(!result.pair_filters_current);
        assert!(!result.scene_discovery_current);
        assert!(!result.solver_state_reusable);
        assert!(!result.horizon_eligible);
        assert!(result.reasons.contains(RevocationReason::FilterChanged));
    }

    #[test]
    fn geometry_change_revokes_pair_lease_and_scene_receipt() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.body_b.revisions.geometry = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(!result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(!result.scene_discovery_current);
        assert!(!result.solver_state_reusable);
        assert!(
            result
                .reasons
                .contains(RevocationReason::ShapeOrTopologyChanged)
        );
        assert!(result.reasons.contains(RevocationReason::SceneReceiptStale));
    }

    #[test]
    fn pose_discontinuity_revokes_pair_lease() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.body_a.revisions.pose_discontinuity = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(!result.geometry_lease_eligible);
        assert!(!result.scene_discovery_current);
        assert!(
            result
                .reasons
                .contains(RevocationReason::TransformDiscontinuity)
        );
    }

    #[test]
    fn solver_profile_change_preserves_geometry_but_revokes_horizon_and_solver_state() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.profiles.solver = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(!result.scene_discovery_current);
        assert!(!result.solver_state_reusable);
        assert!(result.current_step_semantics_changed);
        assert!(
            result
                .reasons
                .contains(RevocationReason::SolverProfileChanged)
        );
        assert!(result.reasons.contains(RevocationReason::SceneReceiptStale));
    }

    #[test]
    fn scene_membership_change_does_not_claim_pair_geometry_is_invalid() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.scene.membership = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(!result.scene_discovery_current);
        assert!(result.solver_state_reusable);
        assert!(result.reasons.contains(RevocationReason::SceneReceiptStale));
    }

    #[test]
    fn motion_model_change_revokes_horizon_not_current_pair_geometry() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.profiles.motion_model = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(!result.scene_discovery_current);
        assert!(
            result
                .reasons
                .contains(RevocationReason::MotionBoundChanged)
        );
    }

    #[test]
    fn certificate_profile_change_revokes_geometry_lease() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.profiles.geometry_certificate = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(!result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(
            result
                .reasons
                .contains(RevocationReason::CertificateProfileChanged)
        );
    }

    #[test]
    fn body_generation_change_cannot_alias_old_lease() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.body_a.identity.generation += 1;

        let result = captured.revalidate(changed);

        assert!(!result.geometry_lease_eligible);
        assert!(!result.scene_discovery_current);
        assert!(
            result
                .reasons
                .contains(RevocationReason::BodyIdentityChanged)
        );
    }

    #[test]
    fn world_generation_change_revokes_all_old_pair_evidence() {
        let base = snapshot();
        let captured = LeaseReadSet::capture(base);
        let mut changed = base;
        changed.scene.world_generation = Epoch::from_raw(1);

        let result = captured.revalidate(changed);

        assert!(!result.geometry_lease_eligible);
        assert!(!result.horizon_eligible);
        assert!(!result.scene_discovery_current);
        assert!(!result.solver_state_reusable);
        assert!(
            result
                .reasons
                .contains(RevocationReason::WorldGenerationChanged)
        );
    }

    #[test]
    fn material_only_mutation_advances_no_geometry_or_motion_bound_revision() {
        let before = BodyRevisions::default();
        let after = advance_body_revisions(before, BodyMutation::MaterialChanged).unwrap();

        assert_eq!(after.geometry, before.geometry);
        assert_eq!(after.pose_discontinuity, before.pose_discontinuity);
        assert_eq!(after.filter, before.filter);
        assert_eq!(after.material.get(), before.material.get() + 1);
        assert_eq!(after.callback, before.callback);
        assert_eq!(after.motion_bound, before.motion_bound);
    }

    #[test]
    fn material_that_changes_motion_assumptions_advances_both_dependencies() {
        let before = BodyRevisions::default();
        let after =
            advance_body_revisions(before, BodyMutation::MaterialAndMotionBoundsChanged).unwrap();

        assert_eq!(after.material.get(), before.material.get() + 1);
        assert_eq!(after.motion_bound.get(), before.motion_bound.get() + 1);
        assert_eq!(after.geometry, before.geometry);
    }

    #[test]
    fn callback_policy_mutations_declare_motion_bound_effect_separately() {
        let before = BodyRevisions::default();
        let policy_only =
            advance_body_revisions(before, BodyMutation::CallbackPolicyChanged).unwrap();
        assert_eq!(policy_only.callback.get(), 1);
        assert_eq!(policy_only.motion_bound, before.motion_bound);

        let force_affecting =
            advance_body_revisions(before, BodyMutation::CallbackAndMotionBoundsChanged).unwrap();
        assert_eq!(force_affecting.callback.get(), 1);
        assert_eq!(force_affecting.motion_bound.get(), 1);
    }

    #[test]
    fn each_scene_mutation_advances_only_its_owned_revision() {
        let before = SceneRevisions::default();

        let membership =
            advance_scene_revisions(before, SceneMutation::BodyMembershipChanged).unwrap();
        assert_eq!(membership.membership.get(), 1);
        assert_eq!(membership.topology, before.topology);
        assert_eq!(membership.filter, before.filter);

        let topology =
            advance_scene_revisions(before, SceneMutation::TopologyOrProviderChanged).unwrap();
        assert_eq!(topology.topology.get(), 1);
        assert_eq!(topology.membership, before.membership);

        let filters =
            advance_scene_revisions(before, SceneMutation::PairFilterPolicyChanged).unwrap();
        assert_eq!(filters.filter.get(), 1);
        assert_eq!(filters.motion_bound, before.motion_bound);

        let reset =
            advance_scene_revisions(before, SceneMutation::WorldGenerationChanged).unwrap();
        assert_eq!(reset.world_generation.get(), 1);
        assert_eq!(reset.membership, before.membership);
    }

    #[test]
    fn profile_mutations_advance_only_the_matching_profile_revision() {
        let before = ProfileRevisions::default();

        let geometry =
            advance_profile_revisions(before, ProfileMutation::GeometryCertificateChanged)
                .unwrap();
        assert_eq!(geometry.geometry_certificate.get(), 1);
        assert_eq!(geometry.motion_model, before.motion_model);
        assert_eq!(geometry.solver, before.solver);

        let solver =
            advance_profile_revisions(before, ProfileMutation::SolverProfileChanged).unwrap();
        assert_eq!(solver.solver.get(), 1);
        assert_eq!(solver.geometry_certificate, before.geometry_certificate);
    }

    #[test]
    fn untracked_mutations_fail_instead_of_preserving_stale_revisions() {
        assert_eq!(
            advance_body_revisions(BodyRevisions::default(), BodyMutation::Untracked),
            Err(RevisionMutationError::UntrackedMutation)
        );
        assert_eq!(
            advance_scene_revisions(SceneRevisions::default(), SceneMutation::Untracked),
            Err(RevisionMutationError::UntrackedMutation)
        );
        assert_eq!(
            advance_profile_revisions(ProfileRevisions::default(), ProfileMutation::Untracked),
            Err(RevisionMutationError::UntrackedMutation)
        );
    }

    #[test]
    fn multi_dependency_overflow_returns_no_partially_advanced_snapshot() {
        let before = BodyRevisions {
            material: Epoch::from_raw(u64::MAX),
            ..BodyRevisions::default()
        };
        let result =
            advance_body_revisions(before, BodyMutation::MaterialAndMotionBoundsChanged);
        assert_eq!(result, Err(RevisionMutationError::EpochOverflow));
        assert_eq!(before.motion_bound, Epoch::ZERO);
    }

    #[test]
    fn revision_counter_fails_closed_on_overflow() {
        let mut counter = RevisionCounter::from_persisted(Epoch::from_raw(u64::MAX));

        assert_eq!(counter.advance(), Err(EpochOverflow));
        assert_eq!(counter.current(), None);
        assert_eq!(counter.advance(), Err(EpochOverflow));
    }

    #[test]
    fn reason_codes_are_stable_and_iterate_in_numeric_order() {
        let mut reasons = RevocationReasons::default();
        reasons.insert(RevocationReason::SolverProfileChanged);
        reasons.insert(RevocationReason::BodyIdentityChanged);
        reasons.insert(RevocationReason::FilterChanged);

        assert_eq!(
            reasons.iter().collect::<Vec<_>>(),
            vec![
                RevocationReason::BodyIdentityChanged,
                RevocationReason::FilterChanged,
                RevocationReason::SolverProfileChanged,
            ]
        );
        assert_eq!(RevocationReason::BodyIdentityChanged as u8, 0);
        assert_eq!(RevocationReason::SolverProfileChanged as u8, 8);
    }
}
