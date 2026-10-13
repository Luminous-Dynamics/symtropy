// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core types and traits for robotic agents in Symtropy.

pub mod agent;
pub mod haptic;
pub mod motor;
pub mod platform;
pub mod safety;
pub mod sensor_model;
pub mod world_contract;
pub use sensor_model::{
    SensorModelError, SensorModelV1, SensorReadingV1, SensorSampleDispositionV1,
    SensorSampleV1, SensorTruthChannelV1,
};

pub use agent::{RoboticAgent, spawn_robot_body};
pub use haptic::{HapticAttestation, HapticOracle};
pub use motor::{MotorPlanner, UniformGainPlanner};
pub use platform::PlatformType;
pub use safety::JointSafetyAuthority;
