// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test-only reference semantics for admitting physical contact evidence into
//! LENV environmental-effect proposals.
//!
//! This fixture deliberately does not mutate Terrain, vegetation, Hydrology,
//! physics, or any product owner. It proves only the causal boundary:
//!
//! ```text
//! raw contact samples
//! -> stable participant/spatial binding
//! -> admitted contact-episode evidence
//! -> versioned effect evaluation
//! -> deterministic owner proposal
//! ```
//!
//! It does not claim that current `symtropy-physics::CollisionEvent` is already
//! canonical environmental authority.

use symtropy_lifesim_core::composite_information::{
    AuthorityScope, AuthoritySnapshotToken, CapabilitySourceRevision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RuntimeBodyHandle(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct StableParticipantId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct StableSurfaceId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PatchId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ContactEvidenceId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ContactEpisodeId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ContactEffectProfileId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NumericPolicyId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BindingGeneration(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SpatialBindingGeneration(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StableParticipantBinding {
    runtime_handle: RuntimeBodyHandle,
    stable_id: StableParticipantId,
    generation: BindingGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StableSurfaceBinding {
    runtime_handle: RuntimeBodyHandle,
    stable_id: StableSurfaceId,
    patch: PatchId,
    generation: BindingGeneration,
    spatial_generation: SpatialBindingGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContactSample {
    step: u64,
    actor_handle: RuntimeBodyHandle,
    surface_handle: RuntimeBodyHandle,
    normal_impulse_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContactEpisodeInput {
    evidence_id: ContactEvidenceId,
    episode_id: ContactEpisodeId,
    canonical_tick: u64,
    actor_binding: StableParticipantBinding,
    surface_binding: StableSurfaceBinding,
    numeric_policy: NumericPolicyId,
    samples: Vec<ContactSample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalContactEvidence {
    evidence_id: ContactEvidenceId,
    episode_id: ContactEpisodeId,
    canonical_tick: u64,
    actor_id: StableParticipantId,
    actor_binding_generation: BindingGeneration,
    surface_id: StableSurfaceId,
    surface_binding_generation: BindingGeneration,
    patch: PatchId,
    spatial_binding_generation: SpatialBindingGeneration,
    numeric_policy: NumericPolicyId,
    integrated_normal_impulse_units: u64,
    first_source_step: u64,
    last_source_step: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContactAdmissionContext {
    actor_binding: StableParticipantBinding,
    surface_binding: StableSurfaceBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnvironmentObservation {
    patch: PatchId,
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    revision: CapabilitySourceRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactEffectMode {
    RigidNoEffect,
    LinearDose {
        minimum_impulse_units: u64,
        disturbance_divisor: u64,
        vegetation_divisor: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContactEffectProfile {
    id: ContactEffectProfileId,
    numeric_policy: NumericPolicyId,
    mode: ContactEffectMode,
}

impl ContactEffectProfile {
    fn rigid(id: ContactEffectProfileId, numeric_policy: NumericPolicyId) -> Self {
        Self {
            id,
            numeric_policy,
            mode: ContactEffectMode::RigidNoEffect,
        }
    }

    fn linear(
        id: ContactEffectProfileId,
        numeric_policy: NumericPolicyId,
        minimum_impulse_units: u64,
        disturbance_divisor: u64,
        vegetation_divisor: u64,
    ) -> Result<Self, ContactEvidenceError> {
        if disturbance_divisor == 0 || vegetation_divisor == 0 {
            return Err(ContactEvidenceError::InvalidProfile);
        }
        Ok(Self {
            id,
            numeric_policy,
            mode: ContactEffectMode::LinearDose {
                minimum_impulse_units,
                disturbance_divisor,
                vegetation_divisor,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SemanticEffectId {
    evidence_id: ContactEvidenceId,
    profile_id: ContactEffectProfileId,
    patch: PatchId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContactEffectProposal {
    effect_id: SemanticEffectId,
    evidence_id: ContactEvidenceId,
    episode_id: ContactEpisodeId,
    canonical_tick: u64,
    profile_id: ContactEffectProfileId,
    patch: PatchId,
    scope: AuthorityScope,
    observed_snapshot: AuthoritySnapshotToken,
    expected_revision: CapabilitySourceRevision,
    disturbance_delta: u32,
    vegetation_damage: u32,
}

impl ContactEffectProposal {
    fn validate_environment_currentness(
        self,
        current: EnvironmentObservation,
    ) -> Result<(), ContactEvidenceError> {
        if current.patch != self.patch {
            return Err(ContactEvidenceError::SpatialSubjectMismatch);
        }
        if current.scope != self.scope
            || current.snapshot != self.observed_snapshot
            || current.revision != self.expected_revision
        {
            return Err(ContactEvidenceError::StaleEnvironmentObservation);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoEffectReason {
    RigidProfile,
    BelowThreshold,
    BelowResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectEvaluation {
    NoEffect {
        evidence_id: ContactEvidenceId,
        profile_id: ContactEffectProfileId,
        reason: NoEffectReason,
    },
    Proposal(ContactEffectProposal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactEvidenceError {
    MissingEvidence,
    EmptyEpisode,
    DuplicateSourceStep(u64),
    ActorHandleMismatch,
    SurfaceHandleMismatch,
    ImpulseOverflow,
    ActorBindingStale,
    SurfaceBindingStale,
    SpatialBindingStale,
    SpatialSubjectMismatch,
    NumericPolicyMismatch,
    InvalidProfile,
    DoseOverflow,
    StaleEnvironmentObservation,
}

fn aggregate_contact_episode(
    mut input: ContactEpisodeInput,
) -> Result<PhysicalContactEvidence, ContactEvidenceError> {
    if input.samples.is_empty() {
        return Err(ContactEvidenceError::EmptyEpisode);
    }

    input.samples.sort_by_key(|sample| sample.step);
    for pair in input.samples.windows(2) {
        if pair[0].step == pair[1].step {
            return Err(ContactEvidenceError::DuplicateSourceStep(pair[0].step));
        }
    }

    let mut integrated_normal_impulse_units = 0_u64;
    for sample in &input.samples {
        if sample.actor_handle != input.actor_binding.runtime_handle {
            return Err(ContactEvidenceError::ActorHandleMismatch);
        }
        if sample.surface_handle != input.surface_binding.runtime_handle {
            return Err(ContactEvidenceError::SurfaceHandleMismatch);
        }
        integrated_normal_impulse_units = integrated_normal_impulse_units
            .checked_add(sample.normal_impulse_units)
            .ok_or(ContactEvidenceError::ImpulseOverflow)?;
    }

    let first_source_step = input.samples[0].step;
    let last_source_step = input.samples[input.samples.len() - 1].step;

    Ok(PhysicalContactEvidence {
        evidence_id: input.evidence_id,
        episode_id: input.episode_id,
        canonical_tick: input.canonical_tick,
        actor_id: input.actor_binding.stable_id,
        actor_binding_generation: input.actor_binding.generation,
        surface_id: input.surface_binding.stable_id,
        surface_binding_generation: input.surface_binding.generation,
        patch: input.surface_binding.patch,
        spatial_binding_generation: input.surface_binding.spatial_generation,
        numeric_policy: input.numeric_policy,
        integrated_normal_impulse_units,
        first_source_step,
        last_source_step,
    })
}

fn validate_contact_currentness(
    evidence: PhysicalContactEvidence,
    current: ContactAdmissionContext,
) -> Result<(), ContactEvidenceError> {
    if evidence.actor_id != current.actor_binding.stable_id
        || evidence.actor_binding_generation != current.actor_binding.generation
    {
        return Err(ContactEvidenceError::ActorBindingStale);
    }
    if evidence.surface_id != current.surface_binding.stable_id
        || evidence.surface_binding_generation != current.surface_binding.generation
    {
        return Err(ContactEvidenceError::SurfaceBindingStale);
    }
    if evidence.patch != current.surface_binding.patch {
        return Err(ContactEvidenceError::SpatialSubjectMismatch);
    }
    if evidence.spatial_binding_generation != current.surface_binding.spatial_generation {
        return Err(ContactEvidenceError::SpatialBindingStale);
    }
    Ok(())
}

fn evaluate_contact_effect(
    evidence: Option<PhysicalContactEvidence>,
    current_bindings: ContactAdmissionContext,
    observation: EnvironmentObservation,
    profile: ContactEffectProfile,
) -> Result<EffectEvaluation, ContactEvidenceError> {
    let evidence = evidence.ok_or(ContactEvidenceError::MissingEvidence)?;
    validate_contact_currentness(evidence, current_bindings)?;

    if evidence.patch != observation.patch {
        return Err(ContactEvidenceError::SpatialSubjectMismatch);
    }
    if evidence.numeric_policy != profile.numeric_policy {
        return Err(ContactEvidenceError::NumericPolicyMismatch);
    }

    match profile.mode {
        ContactEffectMode::RigidNoEffect => Ok(EffectEvaluation::NoEffect {
            evidence_id: evidence.evidence_id,
            profile_id: profile.id,
            reason: NoEffectReason::RigidProfile,
        }),
        ContactEffectMode::LinearDose {
            minimum_impulse_units,
            disturbance_divisor,
            vegetation_divisor,
        } => {
            if evidence.integrated_normal_impulse_units < minimum_impulse_units {
                return Ok(EffectEvaluation::NoEffect {
                    evidence_id: evidence.evidence_id,
                    profile_id: profile.id,
                    reason: NoEffectReason::BelowThreshold,
                });
            }

            let disturbance = evidence.integrated_normal_impulse_units / disturbance_divisor;
            let vegetation = evidence.integrated_normal_impulse_units / vegetation_divisor;
            if disturbance == 0 && vegetation == 0 {
                return Ok(EffectEvaluation::NoEffect {
                    evidence_id: evidence.evidence_id,
                    profile_id: profile.id,
                    reason: NoEffectReason::BelowResolution,
                });
            }

            let disturbance_delta =
                u32::try_from(disturbance).map_err(|_| ContactEvidenceError::DoseOverflow)?;
            let vegetation_damage =
                u32::try_from(vegetation).map_err(|_| ContactEvidenceError::DoseOverflow)?;

            Ok(EffectEvaluation::Proposal(ContactEffectProposal {
                effect_id: SemanticEffectId {
                    evidence_id: evidence.evidence_id,
                    profile_id: profile.id,
                    patch: evidence.patch,
                },
                evidence_id: evidence.evidence_id,
                episode_id: evidence.episode_id,
                canonical_tick: evidence.canonical_tick,
                profile_id: profile.id,
                patch: evidence.patch,
                scope: observation.scope,
                observed_snapshot: observation.snapshot,
                expected_revision: observation.revision,
                disturbance_delta,
                vegetation_damage,
            }))
        }
    }
}

const ACTOR_HANDLE: RuntimeBodyHandle = RuntimeBodyHandle(11);
const SURFACE_HANDLE: RuntimeBodyHandle = RuntimeBodyHandle(22);
const ACTOR_ID: StableParticipantId = StableParticipantId(101);
const SURFACE_ID: StableSurfaceId = StableSurfaceId(202);
const PATCH: PatchId = PatchId(303);
const SCOPE: AuthorityScope = AuthorityScope(0x6c656e765f636f6e746163745f7630);
const NUMERIC_POLICY: NumericPolicyId = NumericPolicyId(404);
const PROFILE_A: ContactEffectProfileId = ContactEffectProfileId(501);
const PROFILE_B: ContactEffectProfileId = ContactEffectProfileId(502);

fn actor_binding() -> StableParticipantBinding {
    StableParticipantBinding {
        runtime_handle: ACTOR_HANDLE,
        stable_id: ACTOR_ID,
        generation: BindingGeneration(7),
    }
}

fn surface_binding() -> StableSurfaceBinding {
    StableSurfaceBinding {
        runtime_handle: SURFACE_HANDLE,
        stable_id: SURFACE_ID,
        patch: PATCH,
        generation: BindingGeneration(9),
        spatial_generation: SpatialBindingGeneration(12),
    }
}

fn current_bindings() -> ContactAdmissionContext {
    ContactAdmissionContext {
        actor_binding: actor_binding(),
        surface_binding: surface_binding(),
    }
}

fn observation() -> EnvironmentObservation {
    EnvironmentObservation {
        patch: PATCH,
        scope: SCOPE,
        snapshot: AuthoritySnapshotToken(41),
        revision: CapabilitySourceRevision(5),
    }
}

fn linear_profile(id: ContactEffectProfileId) -> ContactEffectProfile {
    ContactEffectProfile::linear(id, NUMERIC_POLICY, 10, 5, 10).unwrap()
}

fn episode(samples: Vec<ContactSample>) -> ContactEpisodeInput {
    ContactEpisodeInput {
        evidence_id: ContactEvidenceId(601),
        episode_id: ContactEpisodeId(701),
        canonical_tick: 800,
        actor_binding: actor_binding(),
        surface_binding: surface_binding(),
        numeric_policy: NUMERIC_POLICY,
        samples,
    }
}

fn sample(step: u64, impulse: u64) -> ContactSample {
    ContactSample {
        step,
        actor_handle: ACTOR_HANDLE,
        surface_handle: SURFACE_HANDLE,
        normal_impulse_units: impulse,
    }
}

fn admitted_reference_evidence() -> PhysicalContactEvidence {
    aggregate_contact_episode(episode(vec![sample(1, 4), sample(2, 6)])).unwrap()
}

fn proposal_from(evidence: PhysicalContactEvidence, profile: ContactEffectProfile) -> ContactEffectProposal {
    match evaluate_contact_effect(Some(evidence), current_bindings(), observation(), profile).unwrap() {
        EffectEvaluation::Proposal(proposal) => proposal,
        EffectEvaluation::NoEffect { .. } => panic!("reference contact should produce a proposal"),
    }
}

#[test]
fn no_contact_evidence_cannot_create_contact_derived_effect() {
    assert_eq!(
        evaluate_contact_effect(None, current_bindings(), observation(), linear_profile(PROFILE_A)),
        Err(ContactEvidenceError::MissingEvidence)
    );
}

#[test]
fn equivalent_episode_partitioning_produces_the_same_effect() {
    let split = aggregate_contact_episode(episode(vec![sample(1, 4), sample(2, 6)])).unwrap();
    let combined = aggregate_contact_episode(episode(vec![sample(1, 10)])).unwrap();

    assert_eq!(split.integrated_normal_impulse_units, 10);
    assert_eq!(combined.integrated_normal_impulse_units, 10);
    assert_ne!(split.last_source_step, combined.last_source_step);

    assert_eq!(
        proposal_from(split, linear_profile(PROFILE_A)),
        proposal_from(combined, linear_profile(PROFILE_A))
    );
}

#[test]
fn source_sample_enumeration_order_is_not_authority() {
    let ordered = aggregate_contact_episode(episode(vec![sample(1, 4), sample(2, 6)])).unwrap();
    let reversed = aggregate_contact_episode(episode(vec![sample(2, 6), sample(1, 4)])).unwrap();
    assert_eq!(ordered, reversed);
}

#[test]
fn duplicate_source_step_and_wrong_runtime_handles_fail_closed() {
    assert_eq!(
        aggregate_contact_episode(episode(vec![sample(1, 4), sample(1, 6)])),
        Err(ContactEvidenceError::DuplicateSourceStep(1))
    );

    let mut wrong_actor = sample(1, 10);
    wrong_actor.actor_handle = RuntimeBodyHandle(999);
    assert_eq!(
        aggregate_contact_episode(episode(vec![wrong_actor])),
        Err(ContactEvidenceError::ActorHandleMismatch)
    );

    let mut wrong_surface = sample(1, 10);
    wrong_surface.surface_handle = RuntimeBodyHandle(999);
    assert_eq!(
        aggregate_contact_episode(episode(vec![wrong_surface])),
        Err(ContactEvidenceError::SurfaceHandleMismatch)
    );
}

#[test]
fn episode_impulse_overflow_fails_closed() {
    assert_eq!(
        aggregate_contact_episode(episode(vec![sample(1, u64::MAX), sample(2, 1)])),
        Err(ContactEvidenceError::ImpulseOverflow)
    );
}

#[test]
fn stale_identity_or_spatial_binding_cannot_author_environment_effects() {
    let evidence = admitted_reference_evidence();

    let mut stale_actor = current_bindings();
    stale_actor.actor_binding.generation = BindingGeneration(8);
    assert_eq!(
        evaluate_contact_effect(Some(evidence), stale_actor, observation(), linear_profile(PROFILE_A)),
        Err(ContactEvidenceError::ActorBindingStale)
    );

    let mut stale_surface = current_bindings();
    stale_surface.surface_binding.generation = BindingGeneration(10);
    assert_eq!(
        evaluate_contact_effect(Some(evidence), stale_surface, observation(), linear_profile(PROFILE_A)),
        Err(ContactEvidenceError::SurfaceBindingStale)
    );

    let mut stale_spatial = current_bindings();
    stale_spatial.surface_binding.spatial_generation = SpatialBindingGeneration(13);
    assert_eq!(
        evaluate_contact_effect(Some(evidence), stale_spatial, observation(), linear_profile(PROFILE_A)),
        Err(ContactEvidenceError::SpatialBindingStale)
    );
}

#[test]
fn wrong_environment_subject_and_numeric_policy_reject() {
    let evidence = admitted_reference_evidence();

    let mut wrong_patch = observation();
    wrong_patch.patch = PatchId(999);
    assert_eq!(
        evaluate_contact_effect(Some(evidence), current_bindings(), wrong_patch, linear_profile(PROFILE_A)),
        Err(ContactEvidenceError::SpatialSubjectMismatch)
    );

    let wrong_numeric = ContactEffectProfile::linear(PROFILE_A, NumericPolicyId(999), 10, 5, 10).unwrap();
    assert_eq!(
        evaluate_contact_effect(Some(evidence), current_bindings(), observation(), wrong_numeric),
        Err(ContactEvidenceError::NumericPolicyMismatch)
    );
}

#[test]
fn rigid_and_below_threshold_profiles_legitimately_produce_no_effect() {
    let evidence = admitted_reference_evidence();
    let rigid = ContactEffectProfile::rigid(PROFILE_A, NUMERIC_POLICY);
    assert_eq!(
        evaluate_contact_effect(Some(evidence), current_bindings(), observation(), rigid),
        Ok(EffectEvaluation::NoEffect {
            evidence_id: ContactEvidenceId(601),
            profile_id: PROFILE_A,
            reason: NoEffectReason::RigidProfile,
        })
    );

    let threshold = ContactEffectProfile::linear(PROFILE_A, NUMERIC_POLICY, 11, 5, 10).unwrap();
    assert_eq!(
        evaluate_contact_effect(Some(evidence), current_bindings(), observation(), threshold),
        Ok(EffectEvaluation::NoEffect {
            evidence_id: ContactEvidenceId(601),
            profile_id: PROFILE_A,
            reason: NoEffectReason::BelowThreshold,
        })
    );
}

#[test]
fn caller_does_not_supply_effect_identity_or_dose() {
    let evidence = admitted_reference_evidence();
    let proposal = proposal_from(evidence, linear_profile(PROFILE_A));

    assert_eq!(
        proposal.effect_id,
        SemanticEffectId {
            evidence_id: ContactEvidenceId(601),
            profile_id: PROFILE_A,
            patch: PATCH,
        }
    );
    assert_eq!(proposal.disturbance_delta, 2);
    assert_eq!(proposal.vegetation_damage, 1);
}

#[test]
fn same_evidence_and_profile_replay_to_the_same_semantic_proposal() {
    let evidence = admitted_reference_evidence();
    let first = proposal_from(evidence, linear_profile(PROFILE_A));
    let second = proposal_from(evidence, linear_profile(PROFILE_A));
    assert_eq!(first, second);
}

#[test]
fn profile_identity_is_part_of_semantic_effect_identity() {
    let evidence = admitted_reference_evidence();
    let a = proposal_from(evidence, linear_profile(PROFILE_A));
    let b = proposal_from(evidence, linear_profile(PROFILE_B));

    assert_ne!(a.effect_id, b.effect_id);
    assert_eq!(a.evidence_id, b.evidence_id);
}

#[test]
fn prepared_proposal_binds_environment_currentness() {
    let proposal = proposal_from(admitted_reference_evidence(), linear_profile(PROFILE_A));
    assert_eq!(proposal.validate_environment_currentness(observation()), Ok(()));

    let mut advanced = observation();
    advanced.revision = CapabilitySourceRevision(6);
    assert_eq!(
        proposal.validate_environment_currentness(advanced),
        Err(ContactEvidenceError::StaleEnvironmentObservation)
    );
}
