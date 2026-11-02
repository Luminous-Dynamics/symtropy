// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-free admission contracts for embodied neural sensorimotor adapters.
//!
//! This crate validates the boundary between a neural model and an embodied
//! agent. It does not implement neural dynamics, read world state, commit
//! physics, or render animation. Digest strings are syntax-checked references;
//! this crate does not hash or authenticate artifacts.

#![forbid(unsafe_code)]

pub const MODEL_DESCRIPTOR_SCHEMA_VERSION: &str = "NEURAL-ADAPTER-MODEL-V1";

const MAX_ACTIVATION_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    SyntheticFixture,
    Phenomenological,
    ConnectomeConstrained,
    PerturbationGrounded,
    BehaviourallyValidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unit {
    Normalized,
    MicromolesPerLitre,
    Radians,
    RadiansPerSecond,
    Newtons,
    MetresPerSecondSquared,
    DegreesCelsius,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelKind {
    OdorConcentration,
    VisualMotion,
    AcousticEnergy,
    SurfaceContactPressure,
    JointAngle,
    JointVelocity,
    MuscleStretch,
    BodyLinearAcceleration,
    EnergyPressure,
    HydrationPressure,
    FatiguePressure,
    ThermalPressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OriginClass {
    ExternalPercept,
    BodySensor,
    HomeostaticModulator,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChannelKey {
    pub kind: ChannelKind,
    /// Stable receptor/body-channel identifier, e.g. nose.left or leg.front.left.
    pub instance_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelSpec {
    pub key: ChannelKey,
    pub origin: OriginClass,
    pub unit: Unit,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectorSpec {
    /// Stable muscle or motor-unit identifier. This is not a joint transform.
    pub effector_id: String,
    pub maximum_activation_ppm: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelDescriptor {
    pub schema_version: String,
    pub model_id: String,
    pub evidence_tier: EvidenceTier,
    pub model_sha256: String,
    pub input_mapping_sha256: String,
    pub output_mapping_sha256: String,
    pub source_connectome_sha256: Option<String>,
    pub perturbation_dataset_sha256: Option<String>,
    pub behaviour_reference_sha256: Option<String>,
    pub solver_id: String,
    pub timestep_ns: u64,
    pub maximum_observation_age_ticks: u64,
    pub maximum_proposal_horizon_ticks: u64,
    pub channels: Vec<ChannelSpec>,
    pub effectors: Vec<EffectorSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationOrigin {
    ExternalPercept {
        emission_id: String,
        emission_sha256: String,
        emitted_tick: u64,
        expires_tick: u64,
    },
    BodySensor {
        body_state_sha256: String,
        sample_tick: u64,
    },
    HomeostaticModulator {
        physiology_state_sha256: String,
        sample_tick: u64,
    },
}

impl ObservationOrigin {
    fn class(&self) -> OriginClass {
        match self {
            Self::ExternalPercept { .. } => OriginClass::ExternalPercept,
            Self::BodySensor { .. } => OriginClass::BodySensor,
            Self::HomeostaticModulator { .. } => OriginClass::HomeostaticModulator,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorObservation {
    pub key: ChannelKey,
    pub unit: Unit,
    pub value: f64,
    pub origin: ObservationOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationFrame {
    pub model_id: String,
    pub tick: u64,
    /// Digest of the authoritative body snapshot from which body sensors were read.
    pub body_state_sha256: String,
    pub observations: Vec<SensorObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotorActivation {
    pub effector_id: String,
    /// Unsigned parts-per-million activation, constrained to [0, 1_000_000].
    pub activation_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotorProposal {
    pub model_id: String,
    pub tick: u64,
    pub valid_until_tick: u64,
    pub activations: Vec<MotorActivation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyEffectorSet {
    pub supported_effector_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasonCode {
    UnsupportedSchemaVersion,
    EmptyModelId,
    InvalidModelId,
    MalformedModelDigest,
    MalformedInputMappingDigest,
    MalformedOutputMappingDigest,
    MalformedConnectomeDigest,
    MalformedPerturbationDigest,
    MalformedBehaviourReferenceDigest,
    MissingConnectomeForEvidenceTier,
    MissingPerturbationDataForEvidenceTier,
    MissingBehaviourReferenceForEvidenceTier,
    EmptySolverId,
    InvalidSolverId,
    ZeroTimestep,
    ZeroObservationAgeLimit,
    ZeroProposalHorizon,
    MissingInputChannelSpecs,
    MissingEffectorSpecs,
    MissingBodyEffectors,
    EmptyChannelInstanceId,
    InvalidChannelInstanceId,
    InvalidChannelRange,
    DuplicateChannelSpec,
    EmptyEffectorId,
    InvalidEffectorId,
    InvalidEffectorActivationLimit,
    DuplicateEffectorSpec,
    InvalidFrameModelId,
    FrameModelMismatch,
    MalformedBodyStateDigest,
    UnknownObservationChannel,
    DuplicateObservation,
    ObservationUnitMismatch,
    ObservationOriginMismatch,
    NonFiniteObservation,
    ObservationOutOfRange,
    EmptyEmissionId,
    InvalidEmissionId,
    MalformedEmissionDigest,
    FutureEmission,
    InvalidEmissionWindow,
    ExpiredPercept,
    StalePercept,
    MalformedObservedBodyDigest,
    BodySnapshotMismatch,
    FutureBodySample,
    StaleBodySample,
    MalformedPhysiologyDigest,
    FuturePhysiologySample,
    StalePhysiologySample,
    ProposalModelMismatch,
    ProposalTickMismatch,
    EmptyMotorProposal,
    ExpiredMotorProposal,
    ProposalHorizonExceeded,
    ProposalHorizonOverflow,
    DuplicateBodyEffector,
    EmptyBodyEffectorId,
    InvalidBodyEffectorId,
    DuplicateMotorActivation,
    EmptyMotorEffectorId,
    InvalidMotorEffectorId,
    UnknownModelEffector,
    EffectorUnsupportedByBody,
    ActivationOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    reasons: Vec<ReasonCode>,
}

impl ValidationFailure {
    fn from_reasons(mut reasons: Vec<ReasonCode>) -> Self {
        reasons.sort_unstable();
        reasons.dedup();
        Self { reasons }
    }

    /// Returns the canonically sorted, duplicate-free reason codes.
    pub fn reasons(&self) -> &[ReasonCode] {
        &self.reasons
    }

    pub fn contains(&self, reason: ReasonCode) -> bool {
        self.reasons.binary_search(&reason).is_ok()
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_whitespace)
}

fn valid_sha256_reference(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_optional_digest(
    value: &Option<String>,
    reason: ReasonCode,
    reasons: &mut Vec<ReasonCode>,
) {
    if let Some(digest) = value {
        if !valid_sha256_reference(digest) {
            reasons.push(reason);
        }
    }
}

fn channel_spec<'a>(descriptor: &'a ModelDescriptor, key: &ChannelKey) -> Option<&'a ChannelSpec> {
    descriptor.channels.iter().find(|spec| spec.key == *key)
}

fn effector_spec<'a>(
    descriptor: &'a ModelDescriptor,
    effector_id: &str,
) -> Option<&'a EffectorSpec> {
    descriptor
        .effectors
        .iter()
        .find(|spec| spec.effector_id == effector_id)
}

fn observation_age_is_valid(sample_tick: u64, frame_tick: u64, maximum_age: u64) -> bool {
    sample_tick <= frame_tick && frame_tick - sample_tick <= maximum_age
}

/// Validates model identity, declared evidence tier, sensor/effector mappings and time bounds.
///
/// This checks digest-reference syntax only. It does not fetch, hash or authenticate source
/// artifacts; a qualified provenance authority must do that independently.
pub fn validate_model_descriptor(
    descriptor: &ModelDescriptor,
) -> Result<(), ValidationFailure> {
    let mut reasons = Vec::new();

    if descriptor.schema_version != MODEL_DESCRIPTOR_SCHEMA_VERSION {
        reasons.push(ReasonCode::UnsupportedSchemaVersion);
    }
    if descriptor.model_id.trim().is_empty() {
        reasons.push(ReasonCode::EmptyModelId);
    } else if !valid_identifier(&descriptor.model_id) {
        reasons.push(ReasonCode::InvalidModelId);
    }
    if !valid_sha256_reference(&descriptor.model_sha256) {
        reasons.push(ReasonCode::MalformedModelDigest);
    }
    if !valid_sha256_reference(&descriptor.input_mapping_sha256) {
        reasons.push(ReasonCode::MalformedInputMappingDigest);
    }
    if !valid_sha256_reference(&descriptor.output_mapping_sha256) {
        reasons.push(ReasonCode::MalformedOutputMappingDigest);
    }

    push_optional_digest(
        &descriptor.source_connectome_sha256,
        ReasonCode::MalformedConnectomeDigest,
        &mut reasons,
    );
    push_optional_digest(
        &descriptor.perturbation_dataset_sha256,
        ReasonCode::MalformedPerturbationDigest,
        &mut reasons,
    );
    push_optional_digest(
        &descriptor.behaviour_reference_sha256,
        ReasonCode::MalformedBehaviourReferenceDigest,
        &mut reasons,
    );

    match descriptor.evidence_tier {
        EvidenceTier::SyntheticFixture | EvidenceTier::Phenomenological => {}
        EvidenceTier::ConnectomeConstrained => {
            if descriptor.source_connectome_sha256.is_none() {
                reasons.push(ReasonCode::MissingConnectomeForEvidenceTier);
            }
        }
        EvidenceTier::PerturbationGrounded => {
            if descriptor.source_connectome_sha256.is_none() {
                reasons.push(ReasonCode::MissingConnectomeForEvidenceTier);
            }
            if descriptor.perturbation_dataset_sha256.is_none() {
                reasons.push(ReasonCode::MissingPerturbationDataForEvidenceTier);
            }
        }
        EvidenceTier::BehaviourallyValidated => {
            if descriptor.behaviour_reference_sha256.is_none() {
                reasons.push(ReasonCode::MissingBehaviourReferenceForEvidenceTier);
            }
        }
    }

    if descriptor.solver_id.trim().is_empty() {
        reasons.push(ReasonCode::EmptySolverId);
    } else if !valid_identifier(&descriptor.solver_id) {
        reasons.push(ReasonCode::InvalidSolverId);
    }
    if descriptor.timestep_ns == 0 {
        reasons.push(ReasonCode::ZeroTimestep);
    }
    if descriptor.maximum_observation_age_ticks == 0 {
        reasons.push(ReasonCode::ZeroObservationAgeLimit);
    }
    if descriptor.maximum_proposal_horizon_ticks == 0 {
        reasons.push(ReasonCode::ZeroProposalHorizon);
    }
    if descriptor.channels.is_empty() {
        reasons.push(ReasonCode::MissingInputChannelSpecs);
    }
    if descriptor.effectors.is_empty() {
        reasons.push(ReasonCode::MissingEffectorSpecs);
    }

    for (index, spec) in descriptor.channels.iter().enumerate() {
        if spec.key.instance_id.trim().is_empty() {
            reasons.push(ReasonCode::EmptyChannelInstanceId);
        } else if !valid_identifier(&spec.key.instance_id) {
            reasons.push(ReasonCode::InvalidChannelInstanceId);
        }
        if !spec.minimum.is_finite()
            || !spec.maximum.is_finite()
            || spec.minimum >= spec.maximum
        {
            reasons.push(ReasonCode::InvalidChannelRange);
        }
        if descriptor.channels[..index]
            .iter()
            .any(|previous| previous.key == spec.key)
        {
            reasons.push(ReasonCode::DuplicateChannelSpec);
        }
    }

    for (index, spec) in descriptor.effectors.iter().enumerate() {
        if spec.effector_id.trim().is_empty() {
            reasons.push(ReasonCode::EmptyEffectorId);
        } else if !valid_identifier(&spec.effector_id) {
            reasons.push(ReasonCode::InvalidEffectorId);
        }
        if spec.maximum_activation_ppm == 0
            || spec.maximum_activation_ppm > MAX_ACTIVATION_PPM
        {
            reasons.push(ReasonCode::InvalidEffectorActivationLimit);
        }
        if descriptor.effectors[..index]
            .iter()
            .any(|previous| previous.effector_id == spec.effector_id)
        {
            reasons.push(ReasonCode::DuplicateEffectorSpec);
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(ValidationFailure::from_reasons(reasons))
    }
}

/// Validates that every neural input is a declared, fresh, unit-correct observation.
///
/// The closed ChannelKind and ObservationOrigin enums intentionally provide no
/// representation for an omniscient world snapshot or renderer/camera state.
pub fn validate_observation_frame(
    descriptor: &ModelDescriptor,
    frame: &ObservationFrame,
) -> Result<(), ValidationFailure> {
    let mut reasons = Vec::new();

    if let Err(failure) = validate_model_descriptor(descriptor) {
        reasons.extend(failure.reasons);
    }
    if !valid_identifier(&frame.model_id) {
        reasons.push(ReasonCode::InvalidFrameModelId);
    }
    if frame.model_id != descriptor.model_id {
        reasons.push(ReasonCode::FrameModelMismatch);
    }
    if !valid_sha256_reference(&frame.body_state_sha256) {
        reasons.push(ReasonCode::MalformedBodyStateDigest);
    }

    for (index, observation) in frame.observations.iter().enumerate() {
        if frame.observations[..index]
            .iter()
            .any(|previous| previous.key == observation.key)
        {
            reasons.push(ReasonCode::DuplicateObservation);
        }

        let Some(spec) = channel_spec(descriptor, &observation.key) else {
            reasons.push(ReasonCode::UnknownObservationChannel);
            continue;
        };

        if observation.unit != spec.unit {
            reasons.push(ReasonCode::ObservationUnitMismatch);
        }
        if observation.origin.class() != spec.origin {
            reasons.push(ReasonCode::ObservationOriginMismatch);
        }
        if !observation.value.is_finite() {
            reasons.push(ReasonCode::NonFiniteObservation);
        } else if observation.value < spec.minimum || observation.value > spec.maximum {
            reasons.push(ReasonCode::ObservationOutOfRange);
        }

        match &observation.origin {
            ObservationOrigin::ExternalPercept {
                emission_id,
                emission_sha256,
                emitted_tick,
                expires_tick,
            } => {
                if emission_id.trim().is_empty() {
                    reasons.push(ReasonCode::EmptyEmissionId);
                } else if !valid_identifier(emission_id) {
                    reasons.push(ReasonCode::InvalidEmissionId);
                }
                if !valid_sha256_reference(emission_sha256) {
                    reasons.push(ReasonCode::MalformedEmissionDigest);
                }
                if *emitted_tick > frame.tick {
                    reasons.push(ReasonCode::FutureEmission);
                }
                if emitted_tick > expires_tick {
                    reasons.push(ReasonCode::InvalidEmissionWindow);
                }
                if *expires_tick < frame.tick {
                    reasons.push(ReasonCode::ExpiredPercept);
                }
                if !observation_age_is_valid(
                    *emitted_tick,
                    frame.tick,
                    descriptor.maximum_observation_age_ticks,
                ) {
                    reasons.push(ReasonCode::StalePercept);
                }
            }
            ObservationOrigin::BodySensor {
                body_state_sha256,
                sample_tick,
            } => {
                if !valid_sha256_reference(body_state_sha256) {
                    reasons.push(ReasonCode::MalformedObservedBodyDigest);
                } else if body_state_sha256 != &frame.body_state_sha256 {
                    reasons.push(ReasonCode::BodySnapshotMismatch);
                }
                if *sample_tick > frame.tick {
                    reasons.push(ReasonCode::FutureBodySample);
                } else if !observation_age_is_valid(
                    *sample_tick,
                    frame.tick,
                    descriptor.maximum_observation_age_ticks,
                ) {
                    reasons.push(ReasonCode::StaleBodySample);
                }
            }
            ObservationOrigin::HomeostaticModulator {
                physiology_state_sha256,
                sample_tick,
            } => {
                if !valid_sha256_reference(physiology_state_sha256) {
                    reasons.push(ReasonCode::MalformedPhysiologyDigest);
                }
                if *sample_tick > frame.tick {
                    reasons.push(ReasonCode::FuturePhysiologySample);
                } else if !observation_age_is_valid(
                    *sample_tick,
                    frame.tick,
                    descriptor.maximum_observation_age_ticks,
                ) {
                    reasons.push(ReasonCode::StalePhysiologySample);
                }
            }
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(ValidationFailure::from_reasons(reasons))
    }
}

/// Validates a bounded muscle/motor-unit activation proposal against model and body mappings.
///
/// A successful result only admits the proposal for downstream action arbitration. It does not
/// commit movement, apply forces, resolve contacts or grant renderer/physics authority.
pub fn validate_motor_proposal(
    descriptor: &ModelDescriptor,
    frame: &ObservationFrame,
    proposal: &MotorProposal,
    body_effectors: &BodyEffectorSet,
) -> Result<(), ValidationFailure> {
    let mut reasons = Vec::new();

    if let Err(failure) = validate_observation_frame(descriptor, frame) {
        reasons.extend(failure.reasons);
    }
    if proposal.model_id != descriptor.model_id {
        reasons.push(ReasonCode::ProposalModelMismatch);
    }
    if proposal.tick != frame.tick {
        reasons.push(ReasonCode::ProposalTickMismatch);
    }
    if proposal.activations.is_empty() {
        reasons.push(ReasonCode::EmptyMotorProposal);
    }
    if proposal.valid_until_tick <= proposal.tick {
        reasons.push(ReasonCode::ExpiredMotorProposal);
    }
    let maximum_valid_until = proposal
        .tick
        .checked_add(descriptor.maximum_proposal_horizon_ticks);
    match maximum_valid_until {
        Some(maximum) if proposal.valid_until_tick > maximum => {
            reasons.push(ReasonCode::ProposalHorizonExceeded);
        }
        None => reasons.push(ReasonCode::ProposalHorizonOverflow),
        _ => {}
    }

    if body_effectors.supported_effector_ids.is_empty() {
        reasons.push(ReasonCode::MissingBodyEffectors);
    }

    for (index, effector_id) in body_effectors.supported_effector_ids.iter().enumerate() {
        if effector_id.trim().is_empty() {
            reasons.push(ReasonCode::EmptyBodyEffectorId);
        } else if !valid_identifier(effector_id) {
            reasons.push(ReasonCode::InvalidBodyEffectorId);
        }
        if body_effectors.supported_effector_ids[..index]
            .iter()
            .any(|previous| previous == effector_id)
        {
            reasons.push(ReasonCode::DuplicateBodyEffector);
        }
    }

    for (index, activation) in proposal.activations.iter().enumerate() {
        if activation.effector_id.trim().is_empty() {
            reasons.push(ReasonCode::EmptyMotorEffectorId);
        } else if !valid_identifier(&activation.effector_id) {
            reasons.push(ReasonCode::InvalidMotorEffectorId);
        }
        if proposal.activations[..index]
            .iter()
            .any(|previous| previous.effector_id == activation.effector_id)
        {
            reasons.push(ReasonCode::DuplicateMotorActivation);
        }

        match effector_spec(descriptor, &activation.effector_id) {
            None => reasons.push(ReasonCode::UnknownModelEffector),
            Some(spec) if activation.activation_ppm > spec.maximum_activation_ppm => {
                reasons.push(ReasonCode::ActivationOutOfRange);
            }
            Some(_) => {}
        }

        if !body_effectors
            .supported_effector_ids
            .iter()
            .any(|supported| supported == &activation.effector_id)
        {
            reasons.push(ReasonCode::EffectorUnsupportedByBody);
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(ValidationFailure::from_reasons(reasons))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn descriptor() -> ModelDescriptor {
        ModelDescriptor {
            schema_version: MODEL_DESCRIPTOR_SCHEMA_VERSION.to_string(),
            model_id: "synthetic-worm-cpg".to_string(),
            evidence_tier: EvidenceTier::SyntheticFixture,
            model_sha256: digest('a'),
            input_mapping_sha256: digest('b'),
            output_mapping_sha256: digest('c'),
            source_connectome_sha256: None,
            perturbation_dataset_sha256: None,
            behaviour_reference_sha256: None,
            solver_id: "fixed-euler-v1".to_string(),
            timestep_ns: 10_000_000,
            maximum_observation_age_ticks: 4,
            maximum_proposal_horizon_ticks: 2,
            channels: vec![
                ChannelSpec {
                    key: ChannelKey {
                        kind: ChannelKind::OdorConcentration,
                        instance_id: "nose.left".to_string(),
                    },
                    origin: OriginClass::ExternalPercept,
                    unit: Unit::MicromolesPerLitre,
                    minimum: 0.0,
                    maximum: 10.0,
                },
                ChannelSpec {
                    key: ChannelKey {
                        kind: ChannelKind::SurfaceContactPressure,
                        instance_id: "foot.front.left".to_string(),
                    },
                    origin: OriginClass::BodySensor,
                    unit: Unit::Newtons,
                    minimum: 0.0,
                    maximum: 100.0,
                },
                ChannelSpec {
                    key: ChannelKey {
                        kind: ChannelKind::EnergyPressure,
                        instance_id: "metabolic".to_string(),
                    },
                    origin: OriginClass::HomeostaticModulator,
                    unit: Unit::Normalized,
                    minimum: 0.0,
                    maximum: 1.0,
                },
            ],
            effectors: vec![
                EffectorSpec {
                    effector_id: "muscle.left".to_string(),
                    maximum_activation_ppm: 900_000,
                },
                EffectorSpec {
                    effector_id: "muscle.right".to_string(),
                    maximum_activation_ppm: 900_000,
                },
            ],
        }
    }

    fn frame() -> ObservationFrame {
        ObservationFrame {
            model_id: "synthetic-worm-cpg".to_string(),
            tick: 10,
            body_state_sha256: digest('d'),
            observations: vec![
                SensorObservation {
                    key: ChannelKey {
                        kind: ChannelKind::OdorConcentration,
                        instance_id: "nose.left".to_string(),
                    },
                    unit: Unit::MicromolesPerLitre,
                    value: 2.0,
                    origin: ObservationOrigin::ExternalPercept {
                        emission_id: "basin-emission-17".to_string(),
                        emission_sha256: digest('e'),
                        emitted_tick: 8,
                        expires_tick: 12,
                    },
                },
                SensorObservation {
                    key: ChannelKey {
                        kind: ChannelKind::SurfaceContactPressure,
                        instance_id: "foot.front.left".to_string(),
                    },
                    unit: Unit::Newtons,
                    value: 12.5,
                    origin: ObservationOrigin::BodySensor {
                        body_state_sha256: digest('d'),
                        sample_tick: 10,
                    },
                },
                SensorObservation {
                    key: ChannelKey {
                        kind: ChannelKind::EnergyPressure,
                        instance_id: "metabolic".to_string(),
                    },
                    unit: Unit::Normalized,
                    value: 0.7,
                    origin: ObservationOrigin::HomeostaticModulator {
                        physiology_state_sha256: digest('f'),
                        sample_tick: 9,
                    },
                },
            ],
        }
    }

    fn body_effectors() -> BodyEffectorSet {
        BodyEffectorSet {
            supported_effector_ids: vec![
                "muscle.left".to_string(),
                "muscle.right".to_string(),
            ],
        }
    }

    fn proposal() -> MotorProposal {
        MotorProposal {
            model_id: "synthetic-worm-cpg".to_string(),
            tick: 10,
            valid_until_tick: 11,
            activations: vec![
                MotorActivation {
                    effector_id: "muscle.left".to_string(),
                    activation_ppm: 400_000,
                },
                MotorActivation {
                    effector_id: "muscle.right".to_string(),
                    activation_ppm: 450_000,
                },
            ],
        }
    }

    #[test]
    fn synthetic_fixture_is_admitted_without_biological_sources() {
        assert!(validate_model_descriptor(&descriptor()).is_ok());
    }

    #[test]
    fn model_descriptor_requires_declared_input_channels_and_effectors() {
        let mut model = descriptor();
        model.channels.clear();
        model.effectors.clear();
        let error = validate_model_descriptor(&model)
            .expect_err("model without channels or effectors must reject");
        assert!(error.contains(ReasonCode::MissingInputChannelSpecs));
        assert!(error.contains(ReasonCode::MissingEffectorSpecs));
    }

    #[test]
    fn connectome_claim_requires_source_digest() {
        let mut model = descriptor();
        model.evidence_tier = EvidenceTier::ConnectomeConstrained;
        let error = validate_model_descriptor(&model).expect_err("must reject unsupported claim");
        assert!(error.contains(ReasonCode::MissingConnectomeForEvidenceTier));
    }

    #[test]
    fn perturbation_grounded_claim_requires_connectome_and_perturbation_source() {
        let mut model = descriptor();
        model.evidence_tier = EvidenceTier::PerturbationGrounded;
        let error = validate_model_descriptor(&model).expect_err("must reject missing sources");
        assert!(error.contains(ReasonCode::MissingConnectomeForEvidenceTier));
        assert!(error.contains(ReasonCode::MissingPerturbationDataForEvidenceTier));
    }

    #[test]
    fn digest_references_require_canonical_lowercase_sha256_syntax() {
        let mut model = descriptor();
        model.model_sha256 = "A".repeat(64);
        let error = validate_model_descriptor(&model).expect_err("uppercase digest must reject");
        assert!(error.contains(ReasonCode::MalformedModelDigest));
    }

    #[test]
    fn well_formed_observation_frame_is_admitted() {
        assert!(validate_observation_frame(&descriptor(), &frame()).is_ok());
    }

    #[test]
    fn external_percept_requires_emission_identity() {
        let mut input = frame();
        if let ObservationOrigin::ExternalPercept { emission_id, .. } =
            &mut input.observations[0].origin
        {
            emission_id.clear();
        }
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("emission without identity must reject");
        assert!(error.contains(ReasonCode::EmptyEmissionId));
    }

    #[test]
    fn expired_external_percept_is_rejected() {
        let mut input = frame();
        if let ObservationOrigin::ExternalPercept { expires_tick, .. } =
            &mut input.observations[0].origin
        {
            *expires_tick = 9;
        }
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("expired percept must reject");
        assert!(error.contains(ReasonCode::ExpiredPercept));
    }

    #[test]
    fn future_external_percept_is_rejected() {
        let mut input = frame();
        if let ObservationOrigin::ExternalPercept { emitted_tick, .. } =
            &mut input.observations[0].origin
        {
            *emitted_tick = 11;
        }
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("future emission must reject");
        assert!(error.contains(ReasonCode::FutureEmission));
    }

    #[test]
    fn stale_body_sensor_is_rejected() {
        let mut input = frame();
        if let ObservationOrigin::BodySensor { sample_tick, .. } =
            &mut input.observations[1].origin
        {
            *sample_tick = 1;
        }
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("stale body sample must reject");
        assert!(error.contains(ReasonCode::StaleBodySample));
    }

    #[test]
    fn body_sensor_must_bind_to_current_body_snapshot() {
        let mut input = frame();
        if let ObservationOrigin::BodySensor {
            body_state_sha256, ..
        } = &mut input.observations[1].origin
        {
            *body_state_sha256 = digest('9');
        }
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("mismatched body snapshot must reject");
        assert!(error.contains(ReasonCode::BodySnapshotMismatch));
    }

    #[test]
    fn duplicate_observation_channel_is_rejected() {
        let mut input = frame();
        input.observations.push(input.observations[0].clone());
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("duplicate channel must reject");
        assert!(error.contains(ReasonCode::DuplicateObservation));
    }

    #[test]
    fn non_finite_and_out_of_range_values_are_rejected() {
        let mut input = frame();
        input.observations[0].value = f64::NAN;
        input.observations[1].value = 101.0;
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("invalid values must reject");
        assert!(error.contains(ReasonCode::NonFiniteObservation));
        assert!(error.contains(ReasonCode::ObservationOutOfRange));
    }

    #[test]
    fn unknown_channel_is_rejected() {
        let mut input = frame();
        input.observations[0].key.instance_id = "nose.hidden-world".to_string();
        let error = validate_observation_frame(&descriptor(), &input)
            .expect_err("undeclared channel must reject");
        assert!(error.contains(ReasonCode::UnknownObservationChannel));
    }

    #[test]
    fn valid_activation_proposal_is_admitted_for_body_arbitration() {
        assert!(validate_motor_proposal(&descriptor(), &frame(), &proposal(), &body_effectors())
            .is_ok());
    }

    #[test]
    fn motor_proposal_cannot_outlive_its_declared_horizon() {
        let mut output = proposal();
        output.valid_until_tick = 13;
        let error = validate_motor_proposal(&descriptor(), &frame(), &output, &body_effectors())
            .expect_err("proposal beyond horizon must reject");
        assert!(error.contains(ReasonCode::ProposalHorizonExceeded));
    }

    #[test]
    fn body_must_support_every_proposed_effector() {
        let mut body = body_effectors();
        body.supported_effector_ids.pop();
        let error = validate_motor_proposal(&descriptor(), &frame(), &proposal(), &body)
            .expect_err("unsupported body effector must reject");
        assert!(error.contains(ReasonCode::EffectorUnsupportedByBody));
    }

    #[test]
    fn empty_body_effector_set_is_rejected() {
        let body = BodyEffectorSet {
            supported_effector_ids: Vec::new(),
        };
        let error = validate_motor_proposal(&descriptor(), &frame(), &proposal(), &body)
            .expect_err("body without effectors must reject");
        assert!(error.contains(ReasonCode::MissingBodyEffectors));
    }

    #[test]
    fn activation_cannot_exceed_model_or_global_bound() {
        let mut output = proposal();
        output.activations[0].activation_ppm = 950_000;
        let error = validate_motor_proposal(&descriptor(), &frame(), &output, &body_effectors())
            .expect_err("activation above declared cap must reject");
        assert!(error.contains(ReasonCode::ActivationOutOfRange));
    }

    #[test]
    fn repeated_effector_activation_is_rejected() {
        let mut output = proposal();
        output.activations.push(output.activations[0].clone());
        let error = validate_motor_proposal(&descriptor(), &frame(), &output, &body_effectors())
            .expect_err("duplicate activation must reject");
        assert!(error.contains(ReasonCode::DuplicateMotorActivation));
    }

    #[test]
    fn proposal_from_another_model_is_rejected() {
        let mut output = proposal();
        output.model_id = "other-model".to_string();
        let error = validate_motor_proposal(&descriptor(), &frame(), &output, &body_effectors())
            .expect_err("wrong model identity must reject");
        assert!(error.contains(ReasonCode::ProposalModelMismatch));
    }

    #[test]
    fn failure_reason_order_is_canonical() {
        let mut model = descriptor();
        model.model_sha256.clear();
        model.input_mapping_sha256.clear();
        let error = validate_model_descriptor(&model).expect_err("invalid digests must reject");
        let mut sorted = error.reasons.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(error.reasons, sorted);
    }
}
