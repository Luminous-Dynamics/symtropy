// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core types and traits for robotic agents in Symtropy.

pub mod agent;
pub mod assurance;
pub mod audit_receipt;
pub mod command_evidence;
pub mod control;
pub mod differential_drive;
pub mod dispatch;
pub mod epoch_authority;
pub mod epoch_command_queue;
pub mod epoch_differential_drive;
pub mod epoch_fence;
pub mod epoch_fenced_sink;
pub mod haptic;
pub mod motor;
pub mod outcome_ledger;
pub mod platform;
pub mod recovery_benchmark;
pub mod safety;
pub mod sequence_guard;

pub use agent::{RoboticAgent, spawn_robot_body};
pub use assurance::{
    AxisAlignedWorkspace, EmbodimentEnvelope, GateDecision, IntentKind, RuntimeAssuranceGate,
    RuntimeAssurancePolicy, RuntimeSafetyState, VetoReason,
};
pub use audit_receipt::{
    COMMAND_EVIDENCE_RECEIPT_SCHEMA_VERSION, CommandEvidenceReceipt, CommandEvidenceReceiptContext,
    CommandEvidenceReceiptError, create_command_evidence_receipt, verify_command_evidence_receipt,
};
pub use command_evidence::{
    CommandEvidenceAudit, CommandEvidenceAuditError, CommandEvidenceEvent, CommandEvidenceKind,
    CommandEvidencePhase, CommandRejectionStage, CommandTerminalOutcome, ExpectedCommandIdentity,
    audit_command_evidence,
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
pub use epoch_authority::{
    ControllerEpochAuthority, ControllerEpochStore, EpochAuthorityError, EpochAuthorityStoreError,
};
pub use epoch_command_queue::{
    EpochCommandApplication, EpochCommandQueue, EpochCommandQueueError, EpochCommandQueueStats,
    EpochQueuedCommand,
};
pub use epoch_differential_drive::{
    EpochDifferentialDriveError, EpochFencedCommandProvenance, EpochFencedDifferentialDriveAdapter,
    EpochFencedDifferentialDriveLocalController, EpochFencedDifferentialDriveReceipt,
    EpochFencedNativeReceipt,
};
pub use epoch_fence::{ControllerEpoch, EpochFenceError, EpochFencedOutcomeLedger};
pub use epoch_fenced_sink::{
    EpochFencedControllerReceipt, EpochFencedControllerSink, EpochFencedSubmitError,
};
pub use haptic::{HapticAttestation, HapticOracle};
pub use motor::{MotorPlanner, UniformGainPlanner};
pub use outcome_ledger::{ControlOutcomeLedger, OutcomeIngestReceipt, OutcomeLedgerError};
pub use platform::PlatformType;
pub use recovery_benchmark::{
    ExpectedRecoveryScenario, PairedRecoveryComparison, RecoveryArmSummary, RecoveryBenchmarkArm,
    RecoveryBenchmarkError, RecoveryBenchmarkSummary, RecoveryScenarioClass, RecoveryTrialRecord,
    score_recovery_campaign,
};
pub use safety::JointSafetyAuthority;
pub use sequence_guard::{RequestSequenceGuard, SequenceGuardError, SequencedRuntimeAssurance};
