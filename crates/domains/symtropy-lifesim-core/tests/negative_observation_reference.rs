// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PatchId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservationProfileId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpatialBinding {
    patch: PatchId,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepresentationState {
    Authoritative,
    CoarseAbsenceCapable,
    Unavailable,
}

impl RepresentationState {
    const fn can_prove_absence(self) -> bool {
        matches!(self, Self::Authoritative | Self::CoarseAbsenceCapable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservationProfile {
    id: ObservationProfileId,
    detection_threshold: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NegativeSubjectObservation {
    patch: PatchId,
    membership_generation: u64,
    spatial_generation: u64,
    profile: ObservationProfileId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VegetationObservation {
    Present {
        patch: PatchId,
        membership_generation: u64,
    },
    NoSubject(NegativeSubjectObservation),
    BelowThreshold {
        patch: PatchId,
        strength: u16,
        threshold: u16,
    },
    Unknown {
        patch: PatchId,
        reason: UnknownReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnknownReason {
    RepresentationUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObserveError {
    UnknownPatch,
    WrongSpatialSubject,
    StaleSpatialBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NegativeRevalidationError {
    UnknownPatch,
    WrongProfile,
    StaleSpatialBinding,
    StaleMembership,
    RepresentationUnavailable,
    SubjectAppeared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PatchRecord {
    vegetation_present: bool,
    vegetation_strength: u16,
    membership_generation: u64,
    spatial_generation: u64,
    representation: RepresentationState,
}

impl PatchRecord {
    const fn absent() -> Self {
        Self {
            vegetation_present: false,
            vegetation_strength: 0,
            membership_generation: 1,
            spatial_generation: 1,
            representation: RepresentationState::Authoritative,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceVegetationAuthority {
    patches: BTreeMap<PatchId, PatchRecord>,
}

impl ReferenceVegetationAuthority {
    fn new(patches: impl IntoIterator<Item = PatchId>) -> Self {
        let patches = patches
            .into_iter()
            .map(|patch| (patch, PatchRecord::absent()))
            .collect();
        Self { patches }
    }

    fn spatial_binding(&self, patch: PatchId) -> Result<SpatialBinding, ObserveError> {
        let record = self.patches.get(&patch).ok_or(ObserveError::UnknownPatch)?;
        Ok(SpatialBinding {
            patch,
            generation: record.spatial_generation,
        })
    }

    fn observe(
        &self,
        patch: PatchId,
        binding: SpatialBinding,
        profile: ObservationProfile,
    ) -> Result<VegetationObservation, ObserveError> {
        if binding.patch != patch {
            return Err(ObserveError::WrongSpatialSubject);
        }

        let record = self.patches.get(&patch).ok_or(ObserveError::UnknownPatch)?;
        if binding.generation != record.spatial_generation {
            return Err(ObserveError::StaleSpatialBinding);
        }

        if matches!(record.representation, RepresentationState::Unavailable) {
            return Ok(VegetationObservation::Unknown {
                patch,
                reason: UnknownReason::RepresentationUnavailable,
            });
        }

        if record.vegetation_present {
            if record.vegetation_strength < profile.detection_threshold {
                return Ok(VegetationObservation::BelowThreshold {
                    patch,
                    strength: record.vegetation_strength,
                    threshold: profile.detection_threshold,
                });
            }
            return Ok(VegetationObservation::Present {
                patch,
                membership_generation: record.membership_generation,
            });
        }

        if record.representation.can_prove_absence() {
            return Ok(VegetationObservation::NoSubject(
                NegativeSubjectObservation {
                    patch,
                    membership_generation: record.membership_generation,
                    spatial_generation: record.spatial_generation,
                    profile: profile.id,
                },
            ));
        }

        Ok(VegetationObservation::Unknown {
            patch,
            reason: UnknownReason::RepresentationUnavailable,
        })
    }

    fn revalidate_negative(
        &self,
        negative: NegativeSubjectObservation,
        profile: ObservationProfile,
    ) -> Result<(), NegativeRevalidationError> {
        if negative.profile != profile.id {
            return Err(NegativeRevalidationError::WrongProfile);
        }

        let record = self
            .patches
            .get(&negative.patch)
            .ok_or(NegativeRevalidationError::UnknownPatch)?;

        if negative.spatial_generation != record.spatial_generation {
            return Err(NegativeRevalidationError::StaleSpatialBinding);
        }
        if negative.membership_generation != record.membership_generation {
            return Err(NegativeRevalidationError::StaleMembership);
        }
        if !record.representation.can_prove_absence() {
            return Err(NegativeRevalidationError::RepresentationUnavailable);
        }
        if record.vegetation_present {
            return Err(NegativeRevalidationError::SubjectAppeared);
        }
        Ok(())
    }

    fn set_vegetation(
        &mut self,
        patch: PatchId,
        present: bool,
        strength: u16,
    ) -> Result<(), ObserveError> {
        let record = self
            .patches
            .get_mut(&patch)
            .ok_or(ObserveError::UnknownPatch)?;
        record.vegetation_present = present;
        record.vegetation_strength = if present { strength } else { 0 };
        record.membership_generation = record
            .membership_generation
            .checked_add(1)
            .expect("reference membership generation remains bounded");
        Ok(())
    }

    fn bump_membership_generation(&mut self, patch: PatchId) -> Result<(), ObserveError> {
        let record = self
            .patches
            .get_mut(&patch)
            .ok_or(ObserveError::UnknownPatch)?;
        record.membership_generation = record
            .membership_generation
            .checked_add(1)
            .expect("reference membership generation remains bounded");
        Ok(())
    }

    fn set_representation(
        &mut self,
        patch: PatchId,
        representation: RepresentationState,
    ) -> Result<(), ObserveError> {
        let record = self
            .patches
            .get_mut(&patch)
            .ok_or(ObserveError::UnknownPatch)?;
        record.representation = representation;
        Ok(())
    }

    fn rebind_spatial_subject(&mut self, patch: PatchId) -> Result<(), ObserveError> {
        let record = self
            .patches
            .get_mut(&patch)
            .ok_or(ObserveError::UnknownPatch)?;
        record.spatial_generation = record
            .spatial_generation
            .checked_add(1)
            .expect("reference spatial generation remains bounded");
        Ok(())
    }
}

fn require_negative(
    observation: VegetationObservation,
) -> Result<NegativeSubjectObservation, VegetationObservation> {
    match observation {
        VegetationObservation::NoSubject(negative) => Ok(negative),
        other => Err(other),
    }
}

const PROFILE: ObservationProfile = ObservationProfile {
    id: ObservationProfileId(7),
    detection_threshold: 5,
};

#[test]
fn stable_absence_is_explicit_and_revalidates() {
    let patch = PatchId(1);
    let authority = ReferenceVegetationAuthority::new([patch]);
    let binding = authority.spatial_binding(patch).expect("binding");
    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");

    assert_eq!(negative.patch, patch);
    assert_eq!(authority.revalidate_negative(negative, PROFILE), Ok(()));
}

#[test]
fn subject_appearance_invalidates_prior_absence() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    let binding = authority.spatial_binding(patch).expect("binding");
    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");

    authority
        .set_vegetation(patch, true, 10)
        .expect("add vegetation");

    assert_eq!(
        authority.revalidate_negative(negative, PROFILE),
        Err(NegativeRevalidationError::StaleMembership)
    );
}

#[test]
fn unrelated_partition_mutation_does_not_stale_local_absence() {
    let target = PatchId(1);
    let unrelated = PatchId(2);
    let mut authority = ReferenceVegetationAuthority::new([target, unrelated]);
    let binding = authority.spatial_binding(target).expect("binding");
    let negative = require_negative(authority.observe(target, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");

    authority
        .set_vegetation(unrelated, true, 20)
        .expect("mutate unrelated patch");

    assert_eq!(authority.revalidate_negative(negative, PROFILE), Ok(()));
}

#[test]
fn unavailable_representation_is_unknown_not_absent() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    authority
        .set_representation(patch, RepresentationState::Unavailable)
        .expect("unload reference representation");
    let binding = authority.spatial_binding(patch).expect("binding");

    assert_eq!(
        authority.observe(patch, binding, PROFILE),
        Ok(VegetationObservation::Unknown {
            patch,
            reason: UnknownReason::RepresentationUnavailable,
        })
    );
}

#[test]
fn previously_valid_absence_cannot_survive_loss_of_absence_capability() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    let binding = authority.spatial_binding(patch).expect("binding");
    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");

    authority
        .set_representation(patch, RepresentationState::Unavailable)
        .expect("make representation unavailable");

    assert_eq!(
        authority.revalidate_negative(negative, PROFILE),
        Err(NegativeRevalidationError::RepresentationUnavailable)
    );
}

#[test]
fn coarse_representation_may_prove_absence_only_when_declared_capable() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    authority
        .set_representation(patch, RepresentationState::CoarseAbsenceCapable)
        .expect("set coarse representation");
    let binding = authority.spatial_binding(patch).expect("binding");

    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("declared coarse representation proves absence");
    assert_eq!(authority.revalidate_negative(negative, PROFILE), Ok(()));
}

#[test]
fn membership_generation_change_stales_absence_even_when_current_state_is_still_empty() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    let binding = authority.spatial_binding(patch).expect("binding");
    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");

    authority
        .bump_membership_generation(patch)
        .expect("rebuild membership authority");

    assert_eq!(
        authority.revalidate_negative(negative, PROFILE),
        Err(NegativeRevalidationError::StaleMembership)
    );
}

#[test]
fn spatial_rebinding_stales_negative_observation() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    let binding = authority.spatial_binding(patch).expect("binding");
    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");

    authority
        .rebind_spatial_subject(patch)
        .expect("rebind spatial subject");

    assert_eq!(
        authority.revalidate_negative(negative, PROFILE),
        Err(NegativeRevalidationError::StaleSpatialBinding)
    );
    assert_eq!(
        authority.observe(patch, binding, PROFILE),
        Err(ObserveError::StaleSpatialBinding)
    );
}

#[test]
fn wrong_spatial_subject_cannot_prove_absence() {
    let a = PatchId(1);
    let b = PatchId(2);
    let authority = ReferenceVegetationAuthority::new([a, b]);
    let binding_a = authority.spatial_binding(a).expect("binding");

    assert_eq!(
        authority.observe(b, binding_a, PROFILE),
        Err(ObserveError::WrongSpatialSubject)
    );
}

#[test]
fn below_threshold_presence_is_not_no_subject() {
    let patch = PatchId(1);
    let mut authority = ReferenceVegetationAuthority::new([patch]);
    authority
        .set_vegetation(patch, true, 3)
        .expect("add weak vegetation signal");
    let binding = authority.spatial_binding(patch).expect("binding");

    assert_eq!(
        authority.observe(patch, binding, PROFILE),
        Ok(VegetationObservation::BelowThreshold {
            patch,
            strength: 3,
            threshold: 5,
        })
    );
}

#[test]
fn observation_profile_identity_is_part_of_negative_evidence() {
    let patch = PatchId(1);
    let authority = ReferenceVegetationAuthority::new([patch]);
    let binding = authority.spatial_binding(patch).expect("binding");
    let negative = require_negative(authority.observe(patch, binding, PROFILE).expect("observe"))
        .expect("authority proves absence");
    let other_profile = ObservationProfile {
        id: ObservationProfileId(8),
        detection_threshold: PROFILE.detection_threshold,
    };

    assert_eq!(
        authority.revalidate_negative(negative, other_profile),
        Err(NegativeRevalidationError::WrongProfile)
    );
}
