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
pub mod world_contract;
pub use sensor_model::{
    SensorModelError, SensorModelV1, SensorReadingV1, SensorSampleDispositionV1,
    SensorSampleV1, SensorTruthChannelV1,
};

pub use excavation_contract::{
    ExcavationContractErrorV1, ExcavationDispositionV1, ExcavationResponseV1,
    ExcavationSpatialDeltaV1, ExcavationStateDeltaFormatV1, ExcavationToolProfileV1,
    MaterialAwareExcavationRequestV1, MaterialAwareExcavationResultV1,
    MATERIAL_AWARE_EXCAVATION_SCHEMA_VERSION,
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
