// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core types and traits for robotic agents in Symtropy.

pub mod agent;
pub mod haptic;
pub mod motor;
pub mod observation;
pub mod platform;
pub mod safety;

pub use agent::{RoboticAgent, spawn_robot_body};
pub use haptic::{HapticAttestation, HapticOracle};
pub use motor::{MotorPlanner, UniformGainPlanner};
pub use observation::{
    BaseKinematicObservationError, BaseKinematicObservationV0,
    BASE_KINEMATIC_OBSERVATION_DIM, BASE_KINEMATIC_OBSERVATION_FIELD_IDS,
};
pub use platform::PlatformType;
pub use safety::JointSafetyAuthority;
