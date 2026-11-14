// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Versioned, renderer-independent semantic intent for embodied agents.
//!
//! This contract is the boundary between cognition/planning and embodiment. It is not a
//! motor command: the receiving runtime must validate it, map it to supported skills, and
//! keep physics and safety controllers authoritative. Confidence values are not calibrated
//! probabilities unless confidence_semantics and calibration_profile_id say so explicitly.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub mod skill;

pub const HUMAN_MOTION_INTENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionIntentKind {
    Idle,
    Locomote,
    Speak,
    Gesture,
    Reach,
    Grasp,
    Sit,
    Stand,
    Yield,
    Turn,
    Carry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionPolicy {
    Preemptible,
    FinishCurrentAction,
    SafetyInterruptOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceSemantics {
    /// A probability with calibration evidence identified by calibration_profile_id.
    CalibratedProbability,
    /// A normalized score in [0, 1], not established to be a probability.
    UncalibratedScore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    Symthaea,
    RuleBased,
    ScriptedBaseline,
    HumanOperator,
    ExternalModel,
    SyntheticTestFixture,
}

/// Coordinate convention shared by target positions and planar velocity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionIntentCoordinateFrame {
    pub length_unit: String,
    pub up_axis: String,
    pub forward_axis: String,
    pub position_space: String,
}

/// Optional geometric or entity target in the declared world coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionIntentTarget {
    /// Opaque in-simulation entity/object reference; never put a person's real name here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_ref: Option<String>,
    /// Target position in world-space metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_world_m: Option<[f64; 3]>,
    /// Unit quaternion in x, y, z, w order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation_xyzw: Option<[f64; 4]>,
}

/// Provenance used to reproduce or audit the policy that emitted an intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentProvenance {
    pub source: IntentSource,
    pub policy_id: String,
    pub policy_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_digest_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_sha256: Option<String>,
}

/// Cognition-to-embodiment contract. The fields are versioned independently from Bevy
/// components, animation clip names, actuator units, and visual assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticMotionIntent {
    pub schema_version: u32,
    /// Stable ID for this intent event, not an actor's identity.
    pub intent_id: String,
    /// Opaque, non-identifying in-simulation actor reference.
    pub actor_ref: String,
    pub intent_kind: MotionIntentKind,
    pub coordinate_frame: MotionIntentCoordinateFrame,
    /// Simulation-clock timestamp in seconds, not wall-clock time.
    pub issued_at_sec: f64,
    /// Optional exclusive expiration time on the same simulation clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_sec: Option<f64>,
    /// Relative priority in [0, 1]; not authority to bypass safety constraints.
    pub urgency: f32,
    /// Value in [0, 1]; interpret it according to confidence_semantics.
    pub confidence: f32,
    pub confidence_semantics: ConfidenceSemantics,
    /// Normalized uncertainty proxy in [0, 1]; its calibration must be measured separately.
    pub uncertainty: f32,
    /// Optional desired velocity on the world X/Z plane in metres per second.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_planar_velocity_mps: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<MotionIntentTarget>,
    pub interruption_policy: InterruptionPolicy,
    /// Required only for confidence_semantics = calibrated_probability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_profile_id: Option<String>,
    pub provenance: IntentProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentValidationError {
    UnsupportedSchemaVersion,
    EmptyIntentId,
    EmptyActorRef,
    InvalidCoordinateFrame,
    InvalidIssuedTime,
    InvalidEvaluationTime,
    InvalidValidityWindow,
    NotYetValid,
    Expired,
    InvalidUrgency,
    InvalidConfidence,
    InvalidUncertainty,
    MissingCalibrationProfile,
    MissingLocomotionVelocity,
    InvalidVelocity,
    MissingTarget,
    InvalidTarget,
    InvalidTargetPosition,
    InvalidTargetOrientation,
    EmptyPolicyId,
    EmptyPolicyVersion,
    InvalidObservationDigest,
    InvalidModelDigest,
    MissingModelDigest,
}

impl fmt::Display for IntentValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedSchemaVersion => "unsupported semantic motion intent schema version",
            Self::EmptyIntentId => "intent_id must not be empty",
            Self::EmptyActorRef => "actor_ref must not be empty",
            Self::InvalidCoordinateFrame => concat!(
                "coordinate frame must be metres, +Y up, ",
                "+Z or -Z forward, in world space"
            ),
            Self::InvalidIssuedTime => "issued_at_sec must be finite and non-negative",
            Self::InvalidEvaluationTime => "evaluation time must be finite and non-negative",
            Self::InvalidValidityWindow => {
                "valid_until_sec must be finite and greater than issued_at_sec"
            }
            Self::NotYetValid => "intent was issued after the supplied simulation time",
            Self::Expired => "intent has expired",
            Self::InvalidUrgency => "urgency must be finite and in [0, 1]",
            Self::InvalidConfidence => "confidence must be finite and in [0, 1]",
            Self::InvalidUncertainty => "uncertainty must be finite and in [0, 1]",
            Self::MissingCalibrationProfile => {
                "calibrated probability requires calibration_profile_id"
            }
            Self::MissingLocomotionVelocity => {
                "locomote intent requires desired_planar_velocity_mps"
            }
            Self::InvalidVelocity => "desired planar velocity contains a non-finite value",
            Self::MissingTarget => "reach/grasp intent requires an entity or positional target",
            Self::InvalidTarget => {
                "intent target contains no valid entity, position, or orientation"
            }
            Self::InvalidTargetPosition => {
                "target position must contain finite world-space coordinates"
            }
            Self::InvalidTargetOrientation => concat!(
                "target orientation must be a finite near-unit quaternion ",
                "in x,y,z,w order"
            ),
            Self::EmptyPolicyId => "provenance.policy_id must not be empty",
            Self::EmptyPolicyVersion => "provenance.policy_version must not be empty",
            Self::InvalidObservationDigest => {
                "observation_digest_sha256 must be 64 hexadecimal characters"
            }
            Self::InvalidModelDigest => "model_sha256 must be 64 hexadecimal characters",
            Self::MissingModelDigest => "external_model source requires model_sha256",
        };
        f.write_str(message)
    }
}

impl Error for IntentValidationError {}

impl SemanticMotionIntent {
    /// Validate all structural and semantic constraints independent of the current time.
    pub fn validate(&self) -> Result<(), IntentValidationError> {
        if self.schema_version != HUMAN_MOTION_INTENT_SCHEMA_VERSION {
            return Err(IntentValidationError::UnsupportedSchemaVersion);
        }
        if self.intent_id.trim().is_empty() {
            return Err(IntentValidationError::EmptyIntentId);
        }
        if self.actor_ref.trim().is_empty() {
            return Err(IntentValidationError::EmptyActorRef);
        }
        let frame = &self.coordinate_frame;
        if frame.length_unit != "metre"
            || frame.up_axis != "+Y"
            || !matches!(frame.forward_axis.as_str(), "+Z" | "-Z")
            || frame.position_space != "world"
        {
            return Err(IntentValidationError::InvalidCoordinateFrame);
        }
        if !self.issued_at_sec.is_finite() || self.issued_at_sec < 0.0 {
            return Err(IntentValidationError::InvalidIssuedTime);
        }
        if self
            .valid_until_sec
            .is_some_and(|end| !end.is_finite() || end <= self.issued_at_sec)
        {
            return Err(IntentValidationError::InvalidValidityWindow);
        }
        if !in_unit_interval(self.urgency) {
            return Err(IntentValidationError::InvalidUrgency);
        }
        if !in_unit_interval(self.confidence) {
            return Err(IntentValidationError::InvalidConfidence);
        }
        if !in_unit_interval(self.uncertainty) {
            return Err(IntentValidationError::InvalidUncertainty);
        }
        if self.confidence_semantics == ConfidenceSemantics::CalibratedProbability
            && self
                .calibration_profile_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(IntentValidationError::MissingCalibrationProfile);
        }
        if self
            .desired_planar_velocity_mps
            .is_some_and(|velocity| velocity.iter().any(|value| !value.is_finite()))
        {
            return Err(IntentValidationError::InvalidVelocity);
        }
        if self.intent_kind == MotionIntentKind::Locomote
            && self.desired_planar_velocity_mps.is_none()
        {
            return Err(IntentValidationError::MissingLocomotionVelocity);
        }

        if self
            .target
            .as_ref()
            .and_then(|target| target.entity_ref.as_deref())
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(IntentValidationError::InvalidTarget);
        }
        if self
            .target
            .as_ref()
            .and_then(|target| target.position_world_m)
            .is_some_and(|position| position.iter().any(|value| !value.is_finite()))
        {
            return Err(IntentValidationError::InvalidTargetPosition);
        }
        if self
            .target
            .as_ref()
            .and_then(|target| target.orientation_xyzw)
            .is_some_and(|orientation| !is_valid_orientation(orientation))
        {
            return Err(IntentValidationError::InvalidTargetOrientation);
        }
        let has_target = self.target.as_ref().is_some_and(|target| {
            target
                .entity_ref
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                || target.position_world_m.is_some()
                || target.orientation_xyzw.is_some()
        });
        if self.target.is_some() && !has_target {
            return Err(IntentValidationError::InvalidTarget);
        }
        if matches!(
            self.intent_kind,
            MotionIntentKind::Reach | MotionIntentKind::Grasp
        ) && !self.target.as_ref().is_some_and(|target| {
            target
                .entity_ref
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                || target.position_world_m.is_some()
        }) {
            return Err(IntentValidationError::MissingTarget);
        }

        if self.provenance.policy_id.trim().is_empty() {
            return Err(IntentValidationError::EmptyPolicyId);
        }
        if self.provenance.policy_version.trim().is_empty() {
            return Err(IntentValidationError::EmptyPolicyVersion);
        }
        if self
            .provenance
            .observation_digest_sha256
            .as_deref()
            .is_some_and(|hash| !is_hex_digest(hash, 64))
        {
            return Err(IntentValidationError::InvalidObservationDigest);
        }
        if self
            .provenance
            .model_sha256
            .as_deref()
            .is_some_and(|hash| !is_hex_digest(hash, 64))
        {
            return Err(IntentValidationError::InvalidModelDigest);
        }
        if self.provenance.source == IntentSource::ExternalModel
            && !self
                .provenance
                .model_sha256
                .as_deref()
                .is_some_and(|hash| is_hex_digest(hash, 64))
        {
            return Err(IntentValidationError::MissingModelDigest);
        }

        Ok(())
    }

    /// Validate that the intent is valid at a particular simulation-clock time.
    pub fn validate_at(&self, now_sec: f64) -> Result<(), IntentValidationError> {
        self.validate()?;
        if !now_sec.is_finite() || now_sec < 0.0 {
            return Err(IntentValidationError::InvalidEvaluationTime);
        }
        if now_sec < self.issued_at_sec {
            return Err(IntentValidationError::NotYetValid);
        }
        if self.valid_until_sec.is_some_and(|end| now_sec >= end) {
            return Err(IntentValidationError::Expired);
        }
        Ok(())
    }
}

fn is_valid_orientation(orientation: [f64; 4]) -> bool {
    if orientation.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let norm_squared: f64 = orientation.iter().map(|value| value * value).sum();
    let norm = norm_squared.sqrt();
    norm.is_finite() && (norm - 1.0).abs() <= 0.05
}

fn in_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn is_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> SemanticMotionIntent {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/research/fixtures/human-motion-intent-synthetic-v0.1.json");
        let text = std::fs::read_to_string(path).expect("intent fixture must exist");
        serde_json::from_str(&text).expect("intent fixture must deserialize")
    }

    #[test]
    fn synthetic_intent_fixture_validates_but_has_no_claim_of_calibration() {
        let intent = fixture();
        assert_eq!(intent.schema_version, HUMAN_MOTION_INTENT_SCHEMA_VERSION);
        assert_eq!(
            intent.confidence_semantics,
            ConfidenceSemantics::UncalibratedScore
        );
        intent
            .validate()
            .expect("fixture should be structurally valid");
        intent
            .validate_at(0.5)
            .expect("fixture should be valid at 0.5 seconds");
    }

    #[test]
    fn locomote_requires_velocity() {
        let mut intent = fixture();
        intent.desired_planar_velocity_mps = None;
        assert_eq!(
            intent.validate(),
            Err(IntentValidationError::MissingLocomotionVelocity)
        );
    }

    #[test]
    fn calibrated_confidence_requires_a_named_calibration_profile() {
        let mut intent = fixture();
        intent.confidence_semantics = ConfidenceSemantics::CalibratedProbability;
        assert_eq!(
            intent.validate(),
            Err(IntentValidationError::MissingCalibrationProfile)
        );
        intent.calibration_profile_id = Some("isotonic-v1-heldout".to_owned());
        intent
            .validate()
            .expect("calibrated confidence profile is present");
    }

    #[test]
    fn reach_and_grasp_require_a_target() {
        let mut intent = fixture();
        intent.intent_kind = MotionIntentKind::Reach;
        intent.desired_planar_velocity_mps = None;
        assert_eq!(intent.validate(), Err(IntentValidationError::MissingTarget));
        intent.target = Some(MotionIntentTarget {
            entity_ref: Some("object:cup-17".to_owned()),
            position_world_m: None,
            orientation_xyzw: None,
        });
        intent
            .validate()
            .expect("entity target is sufficient for reach");
    }

    #[test]
    fn rejects_invalid_numeric_values_and_bad_quaternions() {
        let mut intent = fixture();
        intent.confidence = f32::NAN;
        assert_eq!(
            intent.validate(),
            Err(IntentValidationError::InvalidConfidence)
        );
        let mut intent = fixture();
        intent.target = Some(MotionIntentTarget {
            entity_ref: None,
            position_world_m: None,
            orientation_xyzw: Some([0.0, 0.0, 0.0, 0.0]),
        });
        assert_eq!(
            intent.validate(),
            Err(IntentValidationError::InvalidTargetOrientation)
        );
    }

    #[test]
    fn validates_expiration_on_simulation_time() {
        let intent = fixture();
        assert_eq!(intent.validate_at(1.0), Err(IntentValidationError::Expired));
        assert_eq!(
            intent.validate_at(-1.0),
            Err(IntentValidationError::InvalidEvaluationTime)
        );
        assert_eq!(intent.validate_at(0.0), Ok(()));
    }

    #[test]
    fn external_model_intent_requires_model_hash() {
        let mut intent = fixture();
        intent.provenance.source = IntentSource::ExternalModel;
        assert_eq!(
            intent.validate(),
            Err(IntentValidationError::MissingModelDigest)
        );
        intent.provenance.model_sha256 = Some("a".repeat(64));
        intent
            .validate()
            .expect("valid model hash should pass validation");
    }
}
