// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Reference-only LENV semantics for owner-specific contact responses.
//!
//! This fixture proves a deliberately narrow theorem:
//!
//! ```text
//! admitted physical contact evidence
//!          /               \
//!         /                 \
//! ground observation      vegetation observation
//! ground profile          vegetation profile
//!         |                  |
//!         v                  v
//! ground proposal       vegetation proposal
//! ```
//!
//! The common contact is provenance, not a universal damage scalar. Neither
//! evaluator mutates an owner. Production identity, spatial binding, currentness,
//! and coupled publication remain separate qualification subjects.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContactEvidence {
    id: u64,
    impulse_units: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroundMaterial {
    Rock,
    Soil,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GroundObservation {
    subject_id: u64,
    revision: u64,
    material: GroundMaterial,
    moisture_permille: u16,
    disturbance_units: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VegetationObservation {
    subject_id: u64,
    revision: u64,
    cover_units: u32,
    resilience_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GroundResponseProfile {
    id: u64,
    dry_disturbance_per_impulse: u32,
    wet_threshold_permille: u16,
    wet_multiplier: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VegetationResponseProfile {
    id: u64,
    damage_per_impulse: u32,
    resilience_floor_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GroundProposalIdentity {
    contact_id: u64,
    subject_id: u64,
    response_profile_id: u64,
    source_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VegetationProposalIdentity {
    contact_id: u64,
    subject_id: u64,
    response_profile_id: u64,
    source_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GroundProposal {
    identity: GroundProposalIdentity,
    disturbance_delta: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VegetationProposal {
    identity: VegetationProposalIdentity,
    damage_delta: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroundEvaluation {
    NoEffect {
        identity: GroundProposalIdentity,
    },
    Proposal(GroundProposal),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VegetationEvaluation {
    NoSubject,
    NoEffect {
        identity: VegetationProposalIdentity,
    },
    Proposal(VegetationProposal),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvaluationError {
    MissingContactEvidence,
    ArithmeticOverflow,
    InvalidMoisture,
    InvalidResilience,
}

fn evaluate_ground(
    contact: Option<ContactEvidence>,
    observation: GroundObservation,
    profile: GroundResponseProfile,
) -> Result<GroundEvaluation, EvaluationError> {
    let contact = contact.ok_or(EvaluationError::MissingContactEvidence)?;
    if observation.moisture_permille > 1_000 {
        return Err(EvaluationError::InvalidMoisture);
    }

    let identity = GroundProposalIdentity {
        contact_id: contact.id,
        subject_id: observation.subject_id,
        response_profile_id: profile.id,
        source_revision: observation.revision,
    };

    if matches!(observation.material, GroundMaterial::Rock) {
        return Ok(GroundEvaluation::NoEffect { identity });
    }

    let base = contact
        .impulse_units
        .checked_mul(profile.dry_disturbance_per_impulse)
        .ok_or(EvaluationError::ArithmeticOverflow)?;
    let delta = if observation.moisture_permille >= profile.wet_threshold_permille {
        base.checked_mul(profile.wet_multiplier)
            .ok_or(EvaluationError::ArithmeticOverflow)?
    } else {
        base
    };

    if delta == 0 {
        Ok(GroundEvaluation::NoEffect { identity })
    } else {
        Ok(GroundEvaluation::Proposal(GroundProposal {
            identity,
            disturbance_delta: delta,
        }))
    }
}

fn evaluate_vegetation(
    contact: Option<ContactEvidence>,
    observation: Option<VegetationObservation>,
    profile: VegetationResponseProfile,
) -> Result<VegetationEvaluation, EvaluationError> {
    let contact = contact.ok_or(EvaluationError::MissingContactEvidence)?;
    let Some(observation) = observation else {
        return Ok(VegetationEvaluation::NoSubject);
    };
    if observation.resilience_permille > 1_000 {
        return Err(EvaluationError::InvalidResilience);
    }

    let identity = VegetationProposalIdentity {
        contact_id: contact.id,
        subject_id: observation.subject_id,
        response_profile_id: profile.id,
        source_revision: observation.revision,
    };

    if observation.cover_units == 0
        || observation.resilience_permille >= profile.resilience_floor_permille
    {
        return Ok(VegetationEvaluation::NoEffect { identity });
    }

    let raw_damage = contact
        .impulse_units
        .checked_mul(profile.damage_per_impulse)
        .ok_or(EvaluationError::ArithmeticOverflow)?;
    let vulnerability = u32::from(1_000 - observation.resilience_permille);
    let scaled = raw_damage
        .checked_mul(vulnerability)
        .ok_or(EvaluationError::ArithmeticOverflow)?
        / 1_000;
    let damage = scaled.min(observation.cover_units);

    if damage == 0 {
        Ok(VegetationEvaluation::NoEffect { identity })
    } else {
        Ok(VegetationEvaluation::Proposal(VegetationProposal {
            identity,
            damage_delta: damage,
        }))
    }
}

fn contact() -> ContactEvidence {
    ContactEvidence {
        id: 41,
        impulse_units: 10,
    }
}

fn dry_soil() -> GroundObservation {
    GroundObservation {
        subject_id: 100,
        revision: 7,
        material: GroundMaterial::Soil,
        moisture_permille: 250,
        disturbance_units: 12,
    }
}

fn wet_soil() -> GroundObservation {
    GroundObservation {
        moisture_permille: 800,
        ..dry_soil()
    }
}

fn vegetation() -> VegetationObservation {
    VegetationObservation {
        subject_id: 200,
        revision: 9,
        cover_units: 80,
        resilience_permille: 200,
    }
}

fn ground_profile(id: u64) -> GroundResponseProfile {
    GroundResponseProfile {
        id,
        dry_disturbance_per_impulse: 2,
        wet_threshold_permille: 700,
        wet_multiplier: 3,
    }
}

fn vegetation_profile(id: u64) -> VegetationResponseProfile {
    VegetationResponseProfile {
        id,
        damage_per_impulse: 2,
        resilience_floor_permille: 900,
    }
}

#[test]
fn same_contact_can_produce_independent_owner_proposals() {
    let ground = evaluate_ground(Some(contact()), dry_soil(), ground_profile(11)).unwrap();
    let plants = evaluate_vegetation(
        Some(contact()),
        Some(vegetation()),
        vegetation_profile(21),
    )
    .unwrap();

    assert!(matches!(ground, GroundEvaluation::Proposal(_)));
    assert!(matches!(plants, VegetationEvaluation::Proposal(_)));
}

#[test]
fn changing_ground_profile_does_not_change_vegetation_child() {
    let first_ground = evaluate_ground(Some(contact()), dry_soil(), ground_profile(11)).unwrap();
    let mut stronger = ground_profile(12);
    stronger.dry_disturbance_per_impulse = 5;
    let second_ground = evaluate_ground(Some(contact()), dry_soil(), stronger).unwrap();

    let first_plants = evaluate_vegetation(
        Some(contact()),
        Some(vegetation()),
        vegetation_profile(21),
    )
    .unwrap();
    let second_plants = evaluate_vegetation(
        Some(contact()),
        Some(vegetation()),
        vegetation_profile(21),
    )
    .unwrap();

    assert_ne!(first_ground, second_ground);
    assert_eq!(first_plants, second_plants);
}

#[test]
fn changing_vegetation_profile_does_not_change_ground_child() {
    let first_ground = evaluate_ground(Some(contact()), dry_soil(), ground_profile(11)).unwrap();
    let second_ground = evaluate_ground(Some(contact()), dry_soil(), ground_profile(11)).unwrap();

    let first_plants = evaluate_vegetation(
        Some(contact()),
        Some(vegetation()),
        vegetation_profile(21),
    )
    .unwrap();
    let mut tougher = vegetation_profile(22);
    tougher.damage_per_impulse = 1;
    let second_plants =
        evaluate_vegetation(Some(contact()), Some(vegetation()), tougher).unwrap();

    assert_eq!(first_ground, second_ground);
    assert_ne!(first_plants, second_plants);
}

#[test]
fn same_contact_is_state_sensitive_without_changing_contact_identity() {
    let dry = evaluate_ground(Some(contact()), dry_soil(), ground_profile(11)).unwrap();
    let wet = evaluate_ground(Some(contact()), wet_soil(), ground_profile(11)).unwrap();

    let GroundEvaluation::Proposal(dry) = dry else {
        panic!("dry soil should produce a proposal")
    };
    let GroundEvaluation::Proposal(wet) = wet else {
        panic!("wet soil should produce a proposal")
    };

    assert_eq!(dry.identity.contact_id, wet.identity.contact_id);
    assert_eq!(dry.identity.subject_id, wet.identity.subject_id);
    assert!(wet.disturbance_delta > dry.disturbance_delta);
}

#[test]
fn rigid_bare_rock_can_legitimately_produce_no_effect() {
    let rock = GroundObservation {
        material: GroundMaterial::Rock,
        ..dry_soil()
    };
    let result = evaluate_ground(Some(contact()), rock, ground_profile(11)).unwrap();

    assert!(matches!(result, GroundEvaluation::NoEffect { .. }));
}

#[test]
fn absent_vegetation_is_no_subject_not_fabricated_damage() {
    let result = evaluate_vegetation(Some(contact()), None, vegetation_profile(21)).unwrap();

    assert_eq!(result, VegetationEvaluation::NoSubject);
}

#[test]
fn resilient_or_empty_vegetation_can_legitimately_produce_no_effect() {
    let resilient = VegetationObservation {
        resilience_permille: 950,
        ..vegetation()
    };
    let empty = VegetationObservation {
        cover_units: 0,
        ..vegetation()
    };

    assert!(matches!(
        evaluate_vegetation(Some(contact()), Some(resilient), vegetation_profile(21)).unwrap(),
        VegetationEvaluation::NoEffect { .. }
    ));
    assert!(matches!(
        evaluate_vegetation(Some(contact()), Some(empty), vegetation_profile(21)).unwrap(),
        VegetationEvaluation::NoEffect { .. }
    ));
}

#[test]
fn missing_contact_evidence_fails_both_evaluators_closed() {
    assert_eq!(
        evaluate_ground(None, dry_soil(), ground_profile(11)),
        Err(EvaluationError::MissingContactEvidence)
    );
    assert_eq!(
        evaluate_vegetation(None, Some(vegetation()), vegetation_profile(21)),
        Err(EvaluationError::MissingContactEvidence)
    );
}

#[test]
fn proposal_identity_is_owner_and_profile_specific() {
    let GroundEvaluation::Proposal(ground) =
        evaluate_ground(Some(contact()), dry_soil(), ground_profile(11)).unwrap()
    else {
        panic!("ground proposal expected")
    };
    let VegetationEvaluation::Proposal(plants) = evaluate_vegetation(
        Some(contact()),
        Some(vegetation()),
        vegetation_profile(21),
    )
    .unwrap()
    else {
        panic!("vegetation proposal expected")
    };

    assert_eq!(ground.identity.contact_id, plants.identity.contact_id);
    assert_ne!(ground.identity.subject_id, plants.identity.subject_id);
    assert_ne!(
        ground.identity.response_profile_id,
        plants.identity.response_profile_id
    );
}

#[test]
fn invalid_normalized_observations_fail_closed() {
    let invalid_ground = GroundObservation {
        moisture_permille: 1_001,
        ..dry_soil()
    };
    let invalid_plants = VegetationObservation {
        resilience_permille: 1_001,
        ..vegetation()
    };

    assert_eq!(
        evaluate_ground(Some(contact()), invalid_ground, ground_profile(11)),
        Err(EvaluationError::InvalidMoisture)
    );
    assert_eq!(
        evaluate_vegetation(
            Some(contact()),
            Some(invalid_plants),
            vegetation_profile(21)
        ),
        Err(EvaluationError::InvalidResilience)
    );
}

#[test]
fn arithmetic_overflow_fails_before_a_proposal_exists() {
    let huge = ContactEvidence {
        id: 99,
        impulse_units: u32::MAX,
    };
    let mut profile = ground_profile(11);
    profile.dry_disturbance_per_impulse = 2;

    assert_eq!(
        evaluate_ground(Some(huge), dry_soil(), profile),
        Err(EvaluationError::ArithmeticOverflow)
    );
}

#[test]
fn existing_owner_state_is_observed_but_not_mutated_by_evaluation() {
    let ground = dry_soil();
    let plants = vegetation();

    let _ = evaluate_ground(Some(contact()), ground, ground_profile(11)).unwrap();
    let _ = evaluate_vegetation(Some(contact()), Some(plants), vegetation_profile(21)).unwrap();

    assert_eq!(ground.disturbance_units, 12);
    assert_eq!(plants.cover_units, 80);
}
