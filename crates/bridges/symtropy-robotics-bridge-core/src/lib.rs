// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core types and traits for robotic agents in Symtropy.

pub mod agent;
pub mod assurance;
pub mod control;
pub mod differential_drive;
pub mod dispatch;
pub mod haptic;
pub mod motor;
pub mod platform;
pub mod safety;
pub mod sequence_guard;

pub use agent::{RoboticAgent, spawn_robot_body};
pub use assurance::{
    AxisAlignedWorkspace, EmbodimentEnvelope, GateDecision, IntentKind, RuntimeAssuranceGate,
    RuntimeAssurancePolicy, RuntimeSafetyState, VetoReason,
};
pub use control::{
    ControlOutcome, ControlOutcomeError, ControlOutcomeStatus, ControlRequest, ControlRequestError,
    EmbodiedIntent, IntentValidationError, ManipulationOperation, Pose3,
};
pub use differential_drive::{
    CommandProvenance, DifferentialDriveAdapter, DifferentialDriveClock, DifferentialDriveCommand,
    DifferentialDriveConfigError, DifferentialDriveLocalController,
};
pub use dispatch::{
    AuthenticatedSession, ControllerDispatchContext, ControllerReceipt, ControllerSink,
    ControllerSubmitError, DispatchBoundary, DispatchReceipt, DispatchRejection, DispatchStatus,
    SessionAuthenticationError, SessionAuthenticator,
};
pub use haptic::{HapticAttestation, HapticOracle};
pub use motor::{MotorPlanner, UniformGainPlanner};
pub use platform::PlatformType;
pub use safety::JointSafetyAuthority;
pub use sequence_guard::{RequestSequenceGuard, SequenceGuardError, SequencedRuntimeAssurance};
