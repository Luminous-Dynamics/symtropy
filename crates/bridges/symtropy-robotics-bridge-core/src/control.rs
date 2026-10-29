// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed request and outcome contracts between cognitive policy and embodied control.
//!
//! This module deliberately contains no Symthaea dependency and no actuator driver.
//! A request expresses a semantic intent; a platform adapter must validate its
//! capabilities and translate it into a platform-specific controller request.
//! This contract is not itself a safety controller or a stability certificate.

use serde::{Deserialize, Serialize};

const TIME_EPSILON_S: f64 = 1.0e-9;
const QUATERNION_NORM_SQUARED_TOLERANCE: f64 = 1.0e-3;

/// A pose expressed in an explicitly named reference frame.
///
/// Orientation is quaternion [x, y, z, w] and must be finite and normalized.
/// Frame conversion belongs to the embodiment adapter; it must not be guessed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pose3 {
    pub frame_id: String,
    pub position_m: [f64; 3],
    pub orientation_xyzw: [f64; 4],
}

impl Pose3 {
    pub fn validate(&self) -> Result<(), IntentValidationError> {
        if self.frame_id.trim().is_empty() {
            return Err(IntentValidationError::EmptyFrameId);
        }
        if !self.position_m.iter().all(|v| v.is_finite()) {
            return Err(IntentValidationError::NonFinitePosition);
        }
        if !self.orientation_xyzw.iter().all(|v| v.is_finite()) {
            return Err(IntentValidationError::NonFiniteOrientation);
        }
        let norm_squared = self.orientation_xyzw.iter().map(|v| v * v).sum::<f64>();
        if !norm_squared.is_finite()
            || norm_squared <= f64::EPSILON
            || (norm_squared - 1.0).abs() > QUATERNION_NORM_SQUARED_TOLERANCE
        {
            return Err(IntentValidationError::OrientationNotNormalized);
        }
        Ok(())
    }
}

/// Semantic manipulation operation. It is not a gripper/torque command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManipulationOperation {
    Grasp,
    Release,
    Push,
    Pull,
}

/// Cross-platform task intent. Each embodiment adapter decides whether an
/// intent is supported and maps it to a suitable local skill/controller.
///
/// Unsupported intents must be rejected explicitly, not silently converted to
/// an arbitrary actuator vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmbodiedIntent {
    /// Move toward a pose using a platform-specific planner.
    NavigateTo {
        target: Pose3,
        max_speed_mps: Option<f64>,
    },
    /// Hold the platform's locally defined safe/hold state.
    HoldPosition,
    /// Orient or move sensors toward a target pose to gather information.
    Inspect { target: Pose3 },
    /// Move an end effector or platform-specific tool toward a pose.
    Reach { target: Pose3 },
    /// Execute a platform-defined manipulation skill at a target pose.
    Manipulate {
        target: Pose3,
        operation: ManipulationOperation,
    },
    /// Request the platform's stabilization skill (not a direct torque command).
    Stabilize,
    /// Request a platform-defined controlled stop or safe-stop procedure.
    Stop { reason: String },
    /// Ask a supervisor/operator/peer for assistance.
    RequestAssistance { reason: String },
}

impl EmbodiedIntent {
    pub fn validate(&self) -> Result<(), IntentValidationError> {
        match self {
            Self::NavigateTo {
                target,
                max_speed_mps,
            } => {
                target.validate()?;
                if max_speed_mps.is_some_and(|v| !v.is_finite() || v <= 0.0) {
                    return Err(IntentValidationError::InvalidSpeedLimit);
                }
            }
            Self::Inspect { target } | Self::Reach { target } => target.validate()?,
            Self::Manipulate { target, .. } => target.validate()?,
            Self::Stop { reason } | Self::RequestAssistance { reason }
                if reason.trim().is_empty() =>
            {
                return Err(IntentValidationError::EmptyReason);
            }
            Self::HoldPosition
            | Self::Stabilize
            | Self::Stop { .. }
            | Self::RequestAssistance { .. } => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentValidationError {
    EmptyFrameId,
    NonFinitePosition,
    NonFiniteOrientation,
    OrientationNotNormalized,
    InvalidSpeedLimit,
    EmptyReason,
}

/// Result of validating the request's generic envelope.
///
/// This validates envelope shape, time validity and generic target integrity.
/// It does not establish that a platform can safely execute the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlRequestError {
    ZeroRequestId,
    InvalidClock,
    InvalidIssueTime,
    InvalidExpiry,
    FutureDated,
    Expired,
    InvalidConfidence,
    InvalidIntent(IntentValidationError),
}

/// Cognitive proposal delivered to a platform-specific embodiment adapter.
///
/// Timestamps use seconds from the same monotonic clock for issued_at_s,
/// expires_at_s, and validate_at(now_s). Never mix wall-clock time and
/// simulator/hardware monotonic time. Request expiry is checked before adapter
/// dispatch; platform safety checks are still mandatory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlRequest {
    pub request_id: u64,
    pub issued_at_s: f64,
    pub expires_at_s: f64,
    /// Confidence in the intent/belief, not a motor-authority multiplier.
    pub belief_confidence: f32,
    pub intent: EmbodiedIntent,
}

impl ControlRequest {
    pub fn validate_at(&self, now_s: f64) -> Result<(), ControlRequestError> {
        if self.request_id == 0 {
            return Err(ControlRequestError::ZeroRequestId);
        }
        if !now_s.is_finite() || now_s < 0.0 {
            return Err(ControlRequestError::InvalidClock);
        }
        if !self.issued_at_s.is_finite() || self.issued_at_s < 0.0 {
            return Err(ControlRequestError::InvalidIssueTime);
        }
        if !self.expires_at_s.is_finite() || self.expires_at_s <= self.issued_at_s {
            return Err(ControlRequestError::InvalidExpiry);
        }
        if self.issued_at_s > now_s + TIME_EPSILON_S {
            return Err(ControlRequestError::FutureDated);
        }
        if now_s > self.expires_at_s + TIME_EPSILON_S {
            return Err(ControlRequestError::Expired);
        }
        if !self.belief_confidence.is_finite() || !(0.0..=1.0).contains(&self.belief_confidence) {
            return Err(ControlRequestError::InvalidConfidence);
        }
        self.intent
            .validate()
            .map_err(ControlRequestError::InvalidIntent)
    }
}

/// Outcome of a request after the platform adapter/controller/safety authority
/// has considered it. Outcomes are explicit feedback to the cognitive policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlOutcomeStatus {
    Accepted,
    Constrained,
    Vetoed,
    Executing,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

impl ControlOutcomeStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Vetoed | Self::Completed | Self::Failed | Self::Cancelled | Self::Expired
        )
    }

    fn may_follow(self, next: Self) -> bool {
        match self {
            Self::Accepted => matches!(
                next,
                Self::Constrained
                    | Self::Vetoed
                    | Self::Executing
                    | Self::Completed
                    | Self::Failed
                    | Self::Cancelled
                    | Self::Expired
            ),
            Self::Constrained => matches!(
                next,
                Self::Constrained
                    | Self::Vetoed
                    | Self::Executing
                    | Self::Completed
                    | Self::Failed
                    | Self::Cancelled
                    | Self::Expired
            ),
            Self::Executing => matches!(
                next,
                Self::Constrained
                    | Self::Vetoed
                    | Self::Executing
                    | Self::Completed
                    | Self::Failed
                    | Self::Cancelled
                    | Self::Expired
            ),
            Self::Vetoed | Self::Completed | Self::Failed | Self::Cancelled | Self::Expired => {
                false
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlOutcome {
    pub request_id: u64,
    pub status: ControlOutcomeStatus,
    pub observed_at_s: f64,
    pub tick: u64,
    /// Fraction of requested local-skill authority applied, in [0, 1].
    /// This is telemetry, not a command to the actuator layer.
    pub applied_fraction: f32,
    /// Stable machine-readable cause identifiers (e.g. stale_request, workspace_limit).
    pub constraint_codes: Vec<String>,
    /// References/digests for sensor, controller or physics evidence supporting the outcome.
    pub telemetry_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlOutcomeError {
    ZeroRequestId,
    InvalidObservationTime,
    InvalidAppliedFraction,
    MissingConstraintCause,
    RequestIdMismatch,
    TimeWentBackwards,
    TickWentBackwards,
    IllegalStatusTransition,
}

impl ControlOutcome {
    pub fn validate(&self) -> Result<(), ControlOutcomeError> {
        if self.request_id == 0 {
            return Err(ControlOutcomeError::ZeroRequestId);
        }
        if !self.observed_at_s.is_finite() || self.observed_at_s < 0.0 {
            return Err(ControlOutcomeError::InvalidObservationTime);
        }
        if !self.applied_fraction.is_finite() || !(0.0..=1.0).contains(&self.applied_fraction) {
            return Err(ControlOutcomeError::InvalidAppliedFraction);
        }
        if matches!(
            self.status,
            ControlOutcomeStatus::Constrained | ControlOutcomeStatus::Vetoed
        ) && self
            .constraint_codes
            .iter()
            .all(|code| code.trim().is_empty())
        {
            return Err(ControlOutcomeError::MissingConstraintCause);
        }
        Ok(())
    }

    /// Validate that this outcome is a legal, monotonic successor of a prior
    /// outcome for the same request. The first outcome is validated separately.
    pub fn validate_successor_of(&self, previous: &Self) -> Result<(), ControlOutcomeError> {
        previous.validate()?;
        self.validate()?;
        if self.request_id != previous.request_id {
            return Err(ControlOutcomeError::RequestIdMismatch);
        }
        if self.observed_at_s + TIME_EPSILON_S < previous.observed_at_s {
            return Err(ControlOutcomeError::TimeWentBackwards);
        }
        if self.tick < previous.tick {
            return Err(ControlOutcomeError::TickWentBackwards);
        }
        if !previous.status.may_follow(self.status) {
            return Err(ControlOutcomeError::IllegalStatusTransition);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose() -> Pose3 {
        Pose3 {
            frame_id: "world".into(),
            position_m: [1.0, 2.0, 3.0],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn request() -> ControlRequest {
        ControlRequest {
            request_id: 7,
            issued_at_s: 10.0,
            expires_at_s: 11.0,
            belief_confidence: 0.8,
            intent: EmbodiedIntent::NavigateTo {
                target: pose(),
                max_speed_mps: Some(1.5),
            },
        }
    }

    fn outcome(status: ControlOutcomeStatus, at: f64, tick: u64) -> ControlOutcome {
        ControlOutcome {
            request_id: 7,
            status,
            observed_at_s: at,
            tick,
            applied_fraction: if status == ControlOutcomeStatus::Vetoed {
                0.0
            } else {
                1.0
            },
            constraint_codes: if matches!(
                status,
                ControlOutcomeStatus::Constrained | ControlOutcomeStatus::Vetoed
            ) {
                vec!["workspace_limit".into()]
            } else {
                Vec::new()
            },
            telemetry_refs: vec!["sha256:fixture".into()],
        }
    }

    #[test]
    fn accepts_fresh_well_formed_intent() {
        assert_eq!(request().validate_at(10.5), Ok(()));
        assert_eq!(request().validate_at(11.0), Ok(()));
    }

    #[test]
    fn rejects_expired_or_future_dated_request() {
        assert_eq!(
            request().validate_at(11.1),
            Err(ControlRequestError::Expired)
        );
        let mut future = request();
        future.issued_at_s = 12.0;
        future.expires_at_s = 13.0;
        assert_eq!(
            future.validate_at(11.0),
            Err(ControlRequestError::FutureDated)
        );
    }

    #[test]
    fn rejects_invalid_confidence_and_request_identity() {
        let mut invalid = request();
        invalid.belief_confidence = f32::NAN;
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::InvalidConfidence)
        );
        invalid = request();
        invalid.request_id = 0;
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::ZeroRequestId)
        );
    }

    #[test]
    fn rejects_non_finite_target_and_non_unit_quaternion() {
        let mut invalid = request();
        if let EmbodiedIntent::NavigateTo { target, .. } = &mut invalid.intent {
            target.position_m[1] = f64::NAN;
        }
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::InvalidIntent(
                IntentValidationError::NonFinitePosition
            ))
        );
        let mut invalid = request();
        if let EmbodiedIntent::NavigateTo { target, .. } = &mut invalid.intent {
            target.orientation_xyzw = [0.0, 0.0, 0.0, 2.0];
        }
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::InvalidIntent(
                IntentValidationError::OrientationNotNormalized
            ))
        );
    }

    #[test]
    fn rejects_invalid_frame_speed_and_empty_reason() {
        let mut invalid = request();
        if let EmbodiedIntent::NavigateTo {
            target,
            max_speed_mps,
        } = &mut invalid.intent
        {
            target.frame_id = "  ".into();
            *max_speed_mps = Some(0.0);
        }
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::InvalidIntent(
                IntentValidationError::EmptyFrameId
            ))
        );
        let mut invalid = request();
        if let EmbodiedIntent::NavigateTo { max_speed_mps, .. } = &mut invalid.intent {
            *max_speed_mps = Some(f64::INFINITY);
        }
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::InvalidIntent(
                IntentValidationError::InvalidSpeedLimit
            ))
        );
        invalid.intent = EmbodiedIntent::Stop { reason: " ".into() };
        assert_eq!(
            invalid.validate_at(10.5),
            Err(ControlRequestError::InvalidIntent(
                IntentValidationError::EmptyReason
            ))
        );
    }

    #[test]
    fn veto_requires_machine_readable_cause() {
        let mut invalid = outcome(ControlOutcomeStatus::Vetoed, 10.0, 1);
        invalid.constraint_codes.clear();
        assert_eq!(
            invalid.validate(),
            Err(ControlOutcomeError::MissingConstraintCause)
        );
    }

    #[test]
    fn outcome_must_reference_the_same_request_and_monotonic_time() {
        let previous = outcome(ControlOutcomeStatus::Accepted, 10.0, 1);
        let mut next = outcome(ControlOutcomeStatus::Executing, 10.1, 2);
        assert_eq!(next.validate_successor_of(&previous), Ok(()));
        next.request_id = 8;
        assert_eq!(
            next.validate_successor_of(&previous),
            Err(ControlOutcomeError::RequestIdMismatch)
        );
        next = outcome(ControlOutcomeStatus::Executing, 9.0, 2);
        assert_eq!(
            next.validate_successor_of(&previous),
            Err(ControlOutcomeError::TimeWentBackwards)
        );
        next = outcome(ControlOutcomeStatus::Executing, 10.1, 0);
        assert_eq!(
            next.validate_successor_of(&previous),
            Err(ControlOutcomeError::TickWentBackwards)
        );
    }

    #[test]
    fn terminal_outcome_cannot_be_reopened() {
        let previous = outcome(ControlOutcomeStatus::Completed, 10.0, 3);
        let next = outcome(ControlOutcomeStatus::Executing, 10.1, 4);
        assert_eq!(
            next.validate_successor_of(&previous),
            Err(ControlOutcomeError::IllegalStatusTransition)
        );
    }

    #[test]
    fn intent_vocabulary_remains_platform_semantic_not_motor_shaped() {
        let intents = [
            EmbodiedIntent::HoldPosition,
            EmbodiedIntent::Stabilize,
            EmbodiedIntent::Inspect { target: pose() },
            EmbodiedIntent::Reach { target: pose() },
            EmbodiedIntent::Manipulate {
                target: pose(),
                operation: ManipulationOperation::Grasp,
            },
            EmbodiedIntent::Stop {
                reason: "operator requested".into(),
            },
            EmbodiedIntent::RequestAssistance {
                reason: "uncertain foothold".into(),
            },
        ];
        assert!(intents.iter().all(|intent| intent.validate().is_ok()));
    }
}
