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
pub mod survey_adjustment;
pub mod world_contract;
pub use sensor_model::{
    SensorModelError, SensorModelV1, SensorReadingV1, SensorSampleDispositionV1, SensorSampleV1,
    SensorTruthChannelV1,
};

pub use agent::{RoboticAgent, spawn_robot_body};
pub use excavation_contract::{
    CELL_REMOVAL_MASK_SCHEMA_VERSION, CellRemovalMaskPayloadV1, CellRemovalMaskV1,
    EXCAVATION_STATE_DELTA_SCHEMA_VERSION, ExcavationContractErrorV1, ExcavationDispositionV1,
    ExcavationResponseV1, ExcavationSpatialDeltaV1, ExcavationStateDeltaFormatV1,
    ExcavationToolProfileV1, MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION,
    MAX_CELL_REMOVAL_MASK_PAYLOAD_BYTES, MaterialAwareExcavationRequestV1,
    MaterialAwareExcavationResultV1,
};
pub use haptic::{HapticAttestation, HapticOracle};
pub use material_profile::{
    MaterialEvidenceClassV1, MaterialPropertyV1, MaterialQuantityV1, RockLithologyV1,
    RockMaterialConditionV1, RockMaterialProfileV1,
};
pub use motor::{MotorPlanner, UniformGainPlanner};
pub use platform::PlatformType;
pub use rock_cutting_metrics::{
    CuttingMetricError, CuttingPowerBasisV1, CuttingPowerInputV1,
    MechanicalSpecificEnergyUncertaintyV1, MechanicalSpecificEnergyV1, MseUncertaintyInputV1,
    RockCuttingObservationV1,
};
pub use safety::JointSafetyAuthority;
pub use survey_adjustment::{
    FixedSurveyParameterV1, LinearizedSurveyAdjustmentV1, LinearizedSurveyObservationV1,
    SurveyAdjustmentErrorV1, SurveyAdjustmentObservationRoleV1, SurveyAdjustmentProvenanceV1,
    SurveyAdjustmentResidualV1, SurveyAdjustmentResultV1, SurveyParameterEstimateV1,
    MAX_ADJUSTMENT_ID_LENGTH, MAX_ADJUSTMENT_OBSERVATIONS, MAX_ADJUSTMENT_PARAMETERS,
    MAX_ADJUSTMENT_PAYLOAD_BYTES, MAX_ADJUSTMENT_SOURCE_ARTIFACTS,
    SURVEY_ADJUSTMENT_SCHEMA_VERSION,
};
pub use survey_contract::{
    MAX_SURVEY_ARTIFACT_REFS, MAX_SURVEY_EVIDENCE_BYTES, MAX_SURVEY_FRAMES,
    MAX_SURVEY_OBSERVATIONS, MAX_SURVEY_RECEIPTS, MAX_SURVEY_SESSIONS, MAX_SURVEY_TRANSFORMS,
    SURVEY_EVIDENCE_SCHEMA_VERSION, SurveyAccuracyMetricsV1, SurveyAdmissionPolicyV1,
    SurveyAxisDirectionV1, SurveyChangeCandidateV1, SurveyContractErrorV1, SurveyCoordinateFrameV1,
    SurveyCoordinateUnitV1, SurveyEvidenceEnvelopeV1, SurveyHandednessV1, SurveyHeightTypeV1,
    SurveyModalityV1, SurveyObservationRoleV1, SurveyObservationV1, SurveyProcessingDispositionV1,
    SurveyProcessingReceiptV1, SurveyProcessingStageV1, SurveyQualityFlagV1, SurveySessionV1,
    SurveyTransformV1, SurveyUncertaintyV1, VerifiedSurveyEvidenceEnvelopeV1,
};
