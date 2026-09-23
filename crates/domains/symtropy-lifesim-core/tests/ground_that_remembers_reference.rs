// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test-only reference semantics for LENV-CELL-00 `Ground That Remembers`.
//!
//! This fixture is deliberately synthetic. It proves observation/currentness,
//! exactly-once effect settlement, bounded history sensitivity, and in-memory
//! checkpoint continuation without claiming production Terrain, vegetation,
//! Hydrology, persistence, spatial embedding, or terramechanics authority.

use std::collections::BTreeMap;

use symtropy_lifesim_core::composite_information::{
    AuthorityScope, AuthoritySnapshotToken, CapabilitySourceRevision,
};

const INITIAL_VEGETATION_COVER: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PatchId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct EffectId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroundPatchState {
    patch: PatchId,
    revision: CapabilitySourceRevision,
    disturbance_units: u32,
    vegetation_cover_units: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroundObservation {
    patch: PatchId,
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    source_revision: CapabilitySourceRevision,
    disturbance_units: u32,
    vegetation_cover_units: u32,
}

impl GroundObservation {
    fn state(self) -> GroundPatchState {
        GroundPatchState {
            patch: self.patch,
            revision: self.source_revision,
            disturbance_units: self.disturbance_units,
            vegetation_cover_units: self.vegetation_cover_units,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContactEffectProposal {
    effect_id: EffectId,
    patch: PatchId,
    scope: AuthorityScope,
    observed_snapshot: AuthoritySnapshotToken,
    expected_revision: CapabilitySourceRevision,
    disturbance_delta: u32,
    vegetation_damage: u32,
}

impl ContactEffectProposal {
    fn from_observation(
        effect_id: EffectId,
        observation: GroundObservation,
        disturbance_delta: u32,
        vegetation_damage: u32,
    ) -> Self {
        Self {
            effect_id,
            patch: observation.patch,
            scope: observation.scope,
            observed_snapshot: observation.snapshot,
            expected_revision: observation.source_revision,
            disturbance_delta,
            vegetation_damage,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroundSettlementReceipt {
    proposal: ContactEffectProposal,
    before: GroundPatchState,
    after: GroundPatchState,
    committed_snapshot: AuthoritySnapshotToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceGroundCheckpoint {
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    patches: BTreeMap<PatchId, GroundPatchState>,
    receipts: BTreeMap<EffectId, GroundSettlementReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceGroundOwner {
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    patches: BTreeMap<PatchId, GroundPatchState>,
    receipts: BTreeMap<EffectId, GroundSettlementReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundSettlementError {
    DuplicatePatch(PatchId),
    MissingPatch(PatchId),
    ScopeMismatch {
        expected: AuthorityScope,
        actual: AuthorityScope,
    },
    StaleSnapshot {
        expected: AuthoritySnapshotToken,
        actual: AuthoritySnapshotToken,
    },
    StaleRevision {
        patch: PatchId,
        expected: CapabilitySourceRevision,
        actual: CapabilitySourceRevision,
    },
    EffectIdConflict(EffectId),
    DisturbanceOverflow(PatchId),
    VegetationUnderflow(PatchId),
    RevisionOverflow(PatchId),
    SnapshotOverflow,
}

impl ReferenceGroundOwner {
    fn new(
        scope: AuthorityScope,
        patches: impl IntoIterator<Item = PatchId>,
    ) -> Result<Self, GroundSettlementError> {
        let mut states = BTreeMap::new();
        for patch in patches {
            if states
                .insert(
                    patch,
                    GroundPatchState {
                        patch,
                        revision: CapabilitySourceRevision(0),
                        disturbance_units: 0,
                        vegetation_cover_units: INITIAL_VEGETATION_COVER,
                    },
                )
                .is_some()
            {
                return Err(GroundSettlementError::DuplicatePatch(patch));
            }
        }

        Ok(Self {
            scope,
            snapshot: AuthoritySnapshotToken(1),
            patches: states,
            receipts: BTreeMap::new(),
        })
    }

    fn observe(&self, patch: PatchId) -> Result<GroundObservation, GroundSettlementError> {
        let state = self
            .patches
            .get(&patch)
            .copied()
            .ok_or(GroundSettlementError::MissingPatch(patch))?;
        Ok(GroundObservation {
            patch,
            scope: self.scope,
            snapshot: self.snapshot,
            source_revision: state.revision,
            disturbance_units: state.disturbance_units,
            vegetation_cover_units: state.vegetation_cover_units,
        })
    }

    fn state(&self, patch: PatchId) -> Result<GroundPatchState, GroundSettlementError> {
        self.patches
            .get(&patch)
            .copied()
            .ok_or(GroundSettlementError::MissingPatch(patch))
    }

    fn settle(
        &mut self,
        proposal: ContactEffectProposal,
    ) -> Result<GroundSettlementReceipt, GroundSettlementError> {
        if let Some(existing) = self.receipts.get(&proposal.effect_id).copied() {
            return if existing.proposal == proposal {
                Ok(existing)
            } else {
                Err(GroundSettlementError::EffectIdConflict(proposal.effect_id))
            };
        }

        if proposal.scope != self.scope {
            return Err(GroundSettlementError::ScopeMismatch {
                expected: self.scope,
                actual: proposal.scope,
            });
        }
        if proposal.observed_snapshot != self.snapshot {
            return Err(GroundSettlementError::StaleSnapshot {
                expected: proposal.observed_snapshot,
                actual: self.snapshot,
            });
        }

        let before = self
            .patches
            .get(&proposal.patch)
            .copied()
            .ok_or(GroundSettlementError::MissingPatch(proposal.patch))?;
        if proposal.expected_revision != before.revision {
            return Err(GroundSettlementError::StaleRevision {
                patch: proposal.patch,
                expected: proposal.expected_revision,
                actual: before.revision,
            });
        }

        // Compute the complete successor before mutating anything. Every error
        // below therefore leaves both canonical state and receipt history intact.
        let disturbance_units = before
            .disturbance_units
            .checked_add(proposal.disturbance_delta)
            .ok_or(GroundSettlementError::DisturbanceOverflow(proposal.patch))?;
        let vegetation_cover_units = before
            .vegetation_cover_units
            .checked_sub(proposal.vegetation_damage)
            .ok_or(GroundSettlementError::VegetationUnderflow(proposal.patch))?;
        let revision = before
            .revision
            .0
            .checked_add(1)
            .map(CapabilitySourceRevision)
            .ok_or(GroundSettlementError::RevisionOverflow(proposal.patch))?;
        let committed_snapshot = self
            .snapshot
            .0
            .checked_add(1)
            .map(AuthoritySnapshotToken)
            .ok_or(GroundSettlementError::SnapshotOverflow)?;

        let after = GroundPatchState {
            patch: proposal.patch,
            revision,
            disturbance_units,
            vegetation_cover_units,
        };
        let receipt = GroundSettlementReceipt {
            proposal,
            before,
            after,
            committed_snapshot,
        };

        self.patches.insert(proposal.patch, after);
        self.receipts.insert(proposal.effect_id, receipt);
        self.snapshot = committed_snapshot;
        Ok(receipt)
    }

    fn checkpoint(&self) -> ReferenceGroundCheckpoint {
        ReferenceGroundCheckpoint {
            scope: self.scope,
            snapshot: self.snapshot,
            patches: self.patches.clone(),
            receipts: self.receipts.clone(),
        }
    }

    fn restore(checkpoint: ReferenceGroundCheckpoint) -> Self {
        Self {
            scope: checkpoint.scope,
            snapshot: checkpoint.snapshot,
            patches: checkpoint.patches,
            receipts: checkpoint.receipts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactProfile {
    LightFoot,
    HeavyHoof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReferenceInteractionEstimate {
    support_cost_units: u64,
    cover_loss_units: u32,
}

fn estimate_interaction(
    observation: GroundObservation,
    profile: ContactProfile,
) -> ReferenceInteractionEstimate {
    let load_scale = match profile {
        ContactProfile::LightFoot => 1_u64,
        ContactProfile::HeavyHoof => 3_u64,
    };
    let cover_loss_units = INITIAL_VEGETATION_COVER - observation.vegetation_cover_units;
    ReferenceInteractionEstimate {
        support_cost_units: u64::from(observation.disturbance_units) * load_scale
            + u64::from(cover_loss_units),
        cover_loss_units,
    }
}

const USED_PATCH: PatchId = PatchId(1);
const CONTROL_PATCH: PatchId = PatchId(2);
const SCOPE: AuthorityScope = AuthorityScope(0x6c656e765f67726f756e645f7631);

fn owner() -> ReferenceGroundOwner {
    ReferenceGroundOwner::new(SCOPE, [USED_PATCH, CONTROL_PATCH]).unwrap()
}

fn apply_contact(
    owner: &mut ReferenceGroundOwner,
    effect: EffectId,
    patch: PatchId,
    disturbance_delta: u32,
    vegetation_damage: u32,
) -> GroundSettlementReceipt {
    let observation = owner.observe(patch).unwrap();
    owner
        .settle(ContactEffectProposal::from_observation(
            effect,
            observation,
            disturbance_delta,
            vegetation_damage,
        ))
        .unwrap()
}

fn run_history(pass_count: u128) -> ReferenceGroundOwner {
    let mut owner = owner();
    for index in 0..pass_count {
        apply_contact(&mut owner, EffectId(index + 1), USED_PATCH, 10, 15);
    }
    owner
}

#[test]
fn identical_scenario_replays_exactly() {
    assert_eq!(run_history(3), run_history(3));
}

#[test]
fn controlled_histories_diverge_while_control_patch_remains_unchanged() {
    let h0 = run_history(0);
    let h1 = run_history(1);
    let h2 = run_history(3);

    let h0_used = h0.state(USED_PATCH).unwrap();
    let h1_used = h1.state(USED_PATCH).unwrap();
    let h2_used = h2.state(USED_PATCH).unwrap();
    assert!(h0_used.disturbance_units < h1_used.disturbance_units);
    assert!(h1_used.disturbance_units < h2_used.disturbance_units);
    assert!(h0_used.vegetation_cover_units > h1_used.vegetation_cover_units);
    assert!(h1_used.vegetation_cover_units > h2_used.vegetation_cover_units);

    assert_eq!(h0.state(CONTROL_PATCH), h1.state(CONTROL_PATCH));
    assert_eq!(h1.state(CONTROL_PATCH), h2.state(CONTROL_PATCH));
}

#[test]
fn duplicate_identical_effect_is_idempotent() {
    let mut owner = owner();
    let observation = owner.observe(USED_PATCH).unwrap();
    let proposal = ContactEffectProposal::from_observation(EffectId(7), observation, 10, 15);
    let first = owner.settle(proposal).unwrap();
    let state_after_first = owner.clone();
    let retry = owner.settle(proposal).unwrap();

    assert_eq!(first, retry);
    assert_eq!(owner, state_after_first);
}

#[test]
fn reused_effect_id_with_different_payload_fails_without_mutation() {
    let mut owner = owner();
    let observation = owner.observe(USED_PATCH).unwrap();
    let first = ContactEffectProposal::from_observation(EffectId(8), observation, 10, 15);
    owner.settle(first).unwrap();
    let before_conflict = owner.clone();

    let conflicting = ContactEffectProposal {
        disturbance_delta: 11,
        ..first
    };
    assert_eq!(
        owner.settle(conflicting),
        Err(GroundSettlementError::EffectIdConflict(EffectId(8)))
    );
    assert_eq!(owner, before_conflict);
}

#[test]
fn stale_revision_fails_atomically_even_with_current_snapshot() {
    let mut owner = owner();
    let old = owner.observe(USED_PATCH).unwrap();
    apply_contact(&mut owner, EffectId(1), USED_PATCH, 10, 15);
    let before_reject = owner.clone();
    let current = owner.observe(USED_PATCH).unwrap();
    let stale = ContactEffectProposal {
        effect_id: EffectId(2),
        patch: USED_PATCH,
        scope: SCOPE,
        observed_snapshot: current.snapshot,
        expected_revision: old.source_revision,
        disturbance_delta: 10,
        vegetation_damage: 15,
    };

    assert!(matches!(
        owner.settle(stale),
        Err(GroundSettlementError::StaleRevision { .. })
    ));
    assert_eq!(owner, before_reject);
}

#[test]
fn stale_snapshot_fails_atomically_even_with_current_revision() {
    let mut owner = owner();
    let old = owner.observe(USED_PATCH).unwrap();
    apply_contact(&mut owner, EffectId(1), USED_PATCH, 10, 15);
    let before_reject = owner.clone();
    let current = owner.observe(USED_PATCH).unwrap();
    let stale = ContactEffectProposal {
        effect_id: EffectId(2),
        patch: USED_PATCH,
        scope: SCOPE,
        observed_snapshot: old.snapshot,
        expected_revision: current.source_revision,
        disturbance_delta: 10,
        vegetation_damage: 15,
    };

    assert!(matches!(
        owner.settle(stale),
        Err(GroundSettlementError::StaleSnapshot { .. })
    ));
    assert_eq!(owner, before_reject);
}

#[test]
fn wrong_scope_and_unknown_patch_fail_without_mutation() {
    let mut owner = owner();
    let observation = owner.observe(USED_PATCH).unwrap();

    let wrong_scope = ContactEffectProposal {
        scope: AuthorityScope(SCOPE.0 + 1),
        ..ContactEffectProposal::from_observation(EffectId(1), observation, 10, 15)
    };
    let before_scope = owner.clone();
    assert!(matches!(
        owner.settle(wrong_scope),
        Err(GroundSettlementError::ScopeMismatch { .. })
    ));
    assert_eq!(owner, before_scope);

    let unknown_patch = ContactEffectProposal {
        effect_id: EffectId(2),
        patch: PatchId(999),
        ..ContactEffectProposal::from_observation(EffectId(2), observation, 10, 15)
    };
    let before_patch = owner.clone();
    assert_eq!(
        owner.settle(unknown_patch),
        Err(GroundSettlementError::MissingPatch(PatchId(999)))
    );
    assert_eq!(owner, before_patch);
}

#[test]
fn arithmetic_failures_are_atomic() {
    let mut owner = owner();
    owner
        .patches
        .get_mut(&USED_PATCH)
        .unwrap()
        .disturbance_units = u32::MAX;
    let observation = owner.observe(USED_PATCH).unwrap();
    let before_overflow = owner.clone();
    let overflow = ContactEffectProposal::from_observation(EffectId(1), observation, 1, 0);
    assert_eq!(
        owner.settle(overflow),
        Err(GroundSettlementError::DisturbanceOverflow(USED_PATCH))
    );
    assert_eq!(owner, before_overflow);

    let observation = owner.observe(USED_PATCH).unwrap();
    let before_underflow = owner.clone();
    let underflow = ContactEffectProposal::from_observation(
        EffectId(2),
        observation,
        0,
        INITIAL_VEGETATION_COVER + 1,
    );
    assert_eq!(
        owner.settle(underflow),
        Err(GroundSettlementError::VegetationUnderflow(USED_PATCH))
    );
    assert_eq!(owner, before_underflow);
}

#[test]
fn checkpoint_restore_continuation_matches_uninterrupted_run() {
    let uninterrupted = run_history(4);

    let mut interrupted = owner();
    apply_contact(&mut interrupted, EffectId(1), USED_PATCH, 10, 15);
    apply_contact(&mut interrupted, EffectId(2), USED_PATCH, 10, 15);
    let checkpoint = interrupted.checkpoint();
    let mut resumed = ReferenceGroundOwner::restore(checkpoint);
    apply_contact(&mut resumed, EffectId(3), USED_PATCH, 10, 15);
    apply_contact(&mut resumed, EffectId(4), USED_PATCH, 10, 15);

    assert_eq!(uninterrupted, resumed);
}

#[test]
fn old_observation_cannot_authorize_later_effect() {
    let mut owner = owner();
    let old = owner.observe(USED_PATCH).unwrap();
    apply_contact(&mut owner, EffectId(1), USED_PATCH, 10, 15);
    let before_reject = owner.clone();

    let proposal = ContactEffectProposal::from_observation(EffectId(2), old, 10, 15);
    assert!(matches!(
        owner.settle(proposal),
        Err(GroundSettlementError::StaleSnapshot { .. })
    ));
    assert_eq!(owner, before_reject);
}

#[test]
fn downstream_interaction_changes_only_after_world_state_changes() {
    let mut owner = owner();
    let before = owner.observe(USED_PATCH).unwrap();
    let light_before = estimate_interaction(before, ContactProfile::LightFoot);
    let heavy_before = estimate_interaction(before, ContactProfile::HeavyHoof);
    assert_eq!(light_before.cover_loss_units, heavy_before.cover_loss_units);
    assert_eq!(
        light_before.support_cost_units,
        heavy_before.support_cost_units
    );

    apply_contact(&mut owner, EffectId(1), USED_PATCH, 10, 15);
    let after = owner.observe(USED_PATCH).unwrap();
    let light_after = estimate_interaction(after, ContactProfile::LightFoot);
    let heavy_after = estimate_interaction(after, ContactProfile::HeavyHoof);

    assert!(light_after.support_cost_units > light_before.support_cost_units);
    assert!(heavy_after.support_cost_units > light_after.support_cost_units);
    assert_eq!(light_after.cover_loss_units, heavy_after.cover_loss_units);

    // The estimator has no biography input: same current world observation and
    // same contact profile always produce the same physical reference evidence.
    assert_eq!(
        estimate_interaction(after, ContactProfile::HeavyHoof),
        heavy_after
    );
}

#[test]
fn independent_patch_effect_order_preserves_final_per_patch_state() {
    fn run(order: [PatchId; 2]) -> ReferenceGroundOwner {
        let mut owner = owner();
        for (index, patch) in order.into_iter().enumerate() {
            apply_contact(&mut owner, EffectId(index as u128 + 1), patch, 10, 15);
        }
        owner
    }

    let a = run([USED_PATCH, CONTROL_PATCH]);
    let b = run([CONTROL_PATCH, USED_PATCH]);
    assert_eq!(a.state(USED_PATCH), b.state(USED_PATCH));
    assert_eq!(a.state(CONTROL_PATCH), b.state(CONTROL_PATCH));
}

#[test]
fn observation_projects_exact_reference_state() {
    let owner = run_history(2);
    let observation = owner.observe(USED_PATCH).unwrap();
    assert_eq!(observation.state(), owner.state(USED_PATCH).unwrap());
}
