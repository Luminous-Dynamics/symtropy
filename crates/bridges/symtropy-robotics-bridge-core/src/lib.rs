// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core types and traits for robotic agents in Symtropy.

pub mod agent;
pub mod excavation_contract;
pub mod haptic;
pub mod material_profile;
pub mod motor;
pub mod platform;
pub mod rock_cutting_metrics;
pub mod safety;
pub mod sensor_model;
pub mod survey_contract;
pub mod world_contract;
pub use sensor_model::{
    SensorModelError, SensorModelV1, SensorReadingV1, SensorSampleDispositionV1,
    SensorSampleV1, SensorTruthChannelV1,
};

pub use excavation_contract::{
    CellRemovalMaskPayloadV1, CellRemovalMaskV1, ExcavationContractErrorV1,
    ExcavationDispositionV1, ExcavationResponseV1, ExcavationSpatialDeltaV1,
    ExcavationStateDeltaFormatV1,
    ExcavationToolProfileV1, MaterialAwareExcavationRequestV1,
    MaterialAwareExcavationResultV1, CELL_REMOVAL_MASK_SCHEMA_VERSION,
    EXCAVATION_STATE_DELTA_SCHEMA_VERSION, MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION,
    MAX_CELL_REMOVAL_MASK_PAYLOAD_BYTES,
};
pub use survey_contract::{
    SurveyAccuracyMetricsV1, SurveyAdmissionPolicyV1, SurveyAxisDirectionV1,
    SurveyChangeCandidateV1, SurveyContractErrorV1, SurveyCoordinateFrameV1,
    SurveyCoordinateUnitV1, SurveyEvidenceEnvelopeV1, SurveyHandednessV1,
    SurveyHeightTypeV1, SurveyModalityV1, SurveyObservationRoleV1, SurveyObservationV1,
    SurveyProcessingDispositionV1, SurveyProcessingReceiptV1, SurveyProcessingStageV1,
    SurveyQualityFlagV1, SurveySessionV1, SurveyTransformV1, SurveyUncertaintyV1,
    VerifiedSurveyEvidenceEnvelopeV1, MAX_SURVEY_ARTIFACT_REFS, MAX_SURVEY_EVIDENCE_BYTES,
    MAX_SURVEY_FRAMES, MAX_SURVEY_OBSERVATIONS, MAX_SURVEY_RECEIPTS, MAX_SURVEY_SESSIONS,
    MAX_SURVEY_TRANSFORMS, SURVEY_EVIDENCE_SCHEMA_VERSION,
};
pub use agent::{RoboticAgent, spawn_robot_body};
pub use haptic::{HapticAttestation, HapticOracle};
pub use material_profile::{
    MaterialEvidenceClassV1, MaterialPropertyV1, MaterialQuantityV1, RockLithologyV1,
    RockMaterialConditionV1, RockMaterialProfileV1,
};
pub use motor::{MotorPlanner, UniformGainPlanner};
pub use rock_cutting_metrics::{
    CuttingMetricError, CuttingPowerBasisV1, CuttingPowerInputV1,
    MechanicalSpecificEnergyUncertaintyV1, MechanicalSpecificEnergyV1, MseUncertaintyInputV1,
    RockCuttingObservationV1,
};
pub use platform::PlatformType;
pub use safety::JointSafetyAuthority;
