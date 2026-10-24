// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Renderer-independent motion skill contracts and fail-closed outcome evaluation.
//!
//! A contract describes what a skill requires and how completion is verified; it does not
//! implement a physics solver or controller. Runtime adapters supply reachability, support,
//! constraint-conflict, and postcondition evidence from the authoritative simulation.

use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, error::Error, fmt};

pub const MOTION_SKILL_CONTRACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionConstraintHardness {
    /// Must not be violated or silently traded for a soft preference.
    Hard,
    /// May be traded against other soft constraints according to priority.
    Soft,
}

impl MotionConstraintHardness {
    fn rank(self) -> u8 {
        match self {
            Self::Hard => 0,
            Self::Soft => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionConstraintKind {
    CollisionFree,
    JointLimits,
    SupportStable,
    RootPosition,
    RootOrientation,
    EndEffectorPosition,
    EndEffectorOrientation,
    ContactState,
    DesiredSpeed,
    Style,
    Gaze,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MotionConstraintTarget {
    PositionWorldM([f64; 3]),
    OrientationWorldXyzw([f64; 4]),
    Scalar(f64),
    Boolean(bool),
    Label(String),
}

/// A typed constraint. Hardness dominates the numeric priority; a large style priority
/// can never override a hard physics/safety constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSkillConstraint {
    pub constraint_id: String,
    pub kind: MotionConstraintKind,
    pub hardness: MotionConstraintHardness,
    /// Higher numbers win only among constraints of equal hardness.
    pub priority: u8,
    /// Task-space tolerance in metres/radians or the target's declared scalar unit.
    pub tolerance: f64,
    pub target: MotionConstraintTarget,
}

/// Returns sort order from stronger to weaker, with deterministic ID tie-breaking.
pub fn compare_constraint_precedence(
    left: &MotionSkillConstraint,
    right: &MotionSkillConstraint,
) -> Ordering {
    left.hardness
        .rank()
        .cmp(&right.hardness.rank())
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| left.constraint_id.cmp(&right.constraint_id))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MotionSkillPrecondition {
    CapabilityAvailable { capability_id: String },
    TargetReachable { target_ref: String },
    SupportAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionSkillPostconditionKind {
    AtTarget,
    ContactEstablished,
    StableSupport,
    RootVelocityBelowThreshold,
    ObjectTransferred,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSkillPostcondition {
    pub postcondition_id: String,
    pub kind: MotionSkillPostconditionKind,
    /// Required for postconditions tied to a particular object or target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    /// Task-space tolerance in the units declared by the postcondition kind.
    pub tolerance: f64,
}

/// The contract uses simulation time, not wall-clock time or frame counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSkillContract {
    pub schema_version: u32,
    pub skill_id: String,
    pub intent_id: String,
    pub issued_at_sec: f64,
    /// Exclusive expiration time; once now >= this value, the contract is expired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_sec: Option<f64>,
    pub preconditions: Vec<MotionSkillPrecondition>,
    pub constraints: Vec<MotionSkillConstraint>,
    pub postconditions: Vec<MotionSkillPostcondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetReachabilityEvidence {
    pub target_ref: String,
    pub reachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardConstraintConflict {
    pub left_constraint_id: String,
    pub right_constraint_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSkillStartEvidence {
    pub now_sec: f64,
    pub available_capabilities: Vec<String>,
    pub target_reachability: Vec<TargetReachabilityEvidence>,
    /// None means the runtime has not observed enough information to answer.
    pub support_available: Option<bool>,
    /// Produced by the planner/physics feasibility check, not inferred from style priority.
    pub hard_constraint_conflicts: Vec<HardConstraintConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionSkillExecutionFailure {
    ExecutionNotStarted,
    MissingPreconditionEvidence,
    MissingPostconditionEvidence,
    PostconditionNotVerified,
    PostconditionsFailed,
    ConflictingPostconditionEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MotionSkillOutcome {
    Ready,
    Completed,
    Infeasible {
        reason: String,
    },
    NotYetValid,
    Expired,
    Interrupted {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recovery_skill_id: Option<String>,
    },
    CapabilityMissing {
        capability_id: String,
    },
    ExecutionFailed {
        reason: MotionSkillExecutionFailure,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSkillCompletionEvidence {
    /// Current simulation-clock time used to enforce the same expiry boundary as skill start.
    pub now_sec: f64,
    /// Runtime attestation that this evidence refers to an execution that actually started.
    pub execution_started: bool,
    /// False when one or more required observations were unavailable or stale.
    pub observations_complete: bool,
    /// IDs the runtime verified against measured state and each postcondition's tolerance.
    /// This evaluator checks coverage; it does not itself solve physics or measure joints.
    pub verified_postconditions: Vec<String>,
    /// IDs the runtime measured and found outside their declared tolerance.
    pub failed_postconditions: Vec<String>,
    pub interrupted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_skill_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionSkillValidationError {
    UnsupportedSchemaVersion,
    EmptySkillId,
    EmptyIntentId,
    InvalidIssuedTime,
    InvalidValidityWindow,
    InvalidEvaluationTime,
    EmptyConstraintId,
    DuplicateConstraintId,
    InvalidConstraintTolerance,
    InvalidConstraintTarget,
    InvalidConstraintHardness,
    EmptyPostconditionId,
    DuplicatePostconditionId,
    InvalidPostconditionTolerance,
    InvalidTargetRef,
    InvalidReachabilityEvidence,
    DuplicateReachabilityEvidence,
    InvalidConflictEvidence,
    UnknownPostcondition,
    ConflictingPostconditionEvidence,
    InvalidRecoverySkillId,
}

impl fmt::Display for MotionSkillValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for MotionSkillValidationError {}

impl MotionSkillContract {
    /// Validate the serializable contract without assuming any runtime or physics backend.
    pub fn validate(&self) -> Result<(), MotionSkillValidationError> {
        if self.schema_version != MOTION_SKILL_CONTRACT_SCHEMA_VERSION {
            return Err(MotionSkillValidationError::UnsupportedSchemaVersion);
        }
        if self.skill_id.trim().is_empty() {
            return Err(MotionSkillValidationError::EmptySkillId);
        }
        if self.intent_id.trim().is_empty() {
            return Err(MotionSkillValidationError::EmptyIntentId);
        }
        if !self.issued_at_sec.is_finite() || self.issued_at_sec < 0.0 {
            return Err(MotionSkillValidationError::InvalidIssuedTime);
        }
        if self
            .valid_until_sec
            .is_some_and(|until| !until.is_finite() || until <= self.issued_at_sec)
        {
            return Err(MotionSkillValidationError::InvalidValidityWindow);
        }

        let mut constraint_ids: Vec<String> = Vec::with_capacity(self.constraints.len());
        for constraint in &self.constraints {
            if constraint.constraint_id.trim().is_empty() {
                return Err(MotionSkillValidationError::EmptyConstraintId);
            }
            if constraint_ids
                .iter()
                .any(|seen| seen == &constraint.constraint_id)
            {
                return Err(MotionSkillValidationError::DuplicateConstraintId);
            }
            constraint_ids.push(constraint.constraint_id.clone());

            if !constraint.tolerance.is_finite() || constraint.tolerance < 0.0 {
                return Err(MotionSkillValidationError::InvalidConstraintTolerance);
            }
            if !valid_constraint_target(constraint) {
                return Err(MotionSkillValidationError::InvalidConstraintTarget);
            }
            if !valid_constraint_hardness(constraint) {
                return Err(MotionSkillValidationError::InvalidConstraintHardness);
            }
        }

        if self.postconditions.is_empty() {
            return Err(MotionSkillValidationError::EmptyPostconditionId);
        }
        let mut postcondition_ids: Vec<String> = Vec::with_capacity(self.postconditions.len());
        for postcondition in &self.postconditions {
            if postcondition.postcondition_id.trim().is_empty() {
                return Err(MotionSkillValidationError::EmptyPostconditionId);
            }
            if postcondition_ids
                .iter()
                .any(|seen| seen == &postcondition.postcondition_id)
            {
                return Err(MotionSkillValidationError::DuplicatePostconditionId);
            }
            postcondition_ids.push(postcondition.postcondition_id.clone());
            if !postcondition.tolerance.is_finite() || postcondition.tolerance < 0.0 {
                return Err(MotionSkillValidationError::InvalidPostconditionTolerance);
            }
            if postcondition
                .target_ref
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(MotionSkillValidationError::InvalidTargetRef);
            }
        }
        for precondition in &self.preconditions {
            match precondition {
                MotionSkillPrecondition::CapabilityAvailable { capability_id }
                    if capability_id.trim().is_empty() =>
                {
                    return Err(MotionSkillValidationError::InvalidTargetRef);
                }
                MotionSkillPrecondition::TargetReachable { target_ref }
                    if target_ref.trim().is_empty() =>
                {
                    return Err(MotionSkillValidationError::InvalidTargetRef);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Evaluate whether a skill may start. The runtime supplies physical feasibility evidence;
    /// this function never treats missing evidence as proof of feasibility.
    pub fn evaluate_start(
        &self,
        evidence: &MotionSkillStartEvidence,
    ) -> Result<MotionSkillOutcome, MotionSkillValidationError> {
        self.validate()?;
        if !evidence.now_sec.is_finite() || evidence.now_sec < 0.0 {
            return Err(MotionSkillValidationError::InvalidEvaluationTime);
        }
        if evidence.now_sec < self.issued_at_sec {
            return Ok(MotionSkillOutcome::NotYetValid);
        }
        if self
            .valid_until_sec
            .is_some_and(|until| evidence.now_sec >= until)
        {
            return Ok(MotionSkillOutcome::Expired);
        }
        validate_start_evidence(self, evidence)?;

        if !evidence.hard_constraint_conflicts.is_empty() {
            let details = evidence
                .hard_constraint_conflicts
                .iter()
                .map(|conflict| {
                    format!(
                        "{} conflicts with {}: {}",
                        conflict.left_constraint_id, conflict.right_constraint_id, conflict.reason
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Ok(MotionSkillOutcome::Infeasible { reason: details });
        }

        for precondition in &self.preconditions {
            match precondition {
                MotionSkillPrecondition::CapabilityAvailable { capability_id } => {
                    if !evidence
                        .available_capabilities
                        .iter()
                        .any(|available| available == capability_id)
                    {
                        return Ok(MotionSkillOutcome::CapabilityMissing {
                            capability_id: capability_id.clone(),
                        });
                    }
                }
                MotionSkillPrecondition::TargetReachable { target_ref } => {
                    let Some(reachable) = evidence
                        .target_reachability
                        .iter()
                        .find(|item| item.target_ref == *target_ref)
                    else {
                        return Ok(MotionSkillOutcome::ExecutionFailed {
                            reason: MotionSkillExecutionFailure::MissingPreconditionEvidence,
                        });
                    };
                    if !reachable.reachable {
                        return Ok(MotionSkillOutcome::Infeasible {
                            reason: format!("target {target_ref} is not reachable"),
                        });
                    }
                }
                MotionSkillPrecondition::SupportAvailable => match evidence.support_available {
                    Some(true) => {}
                    Some(false) => {
                        return Ok(MotionSkillOutcome::Infeasible {
                            reason: "required support is unavailable".to_owned(),
                        });
                    }
                    None => {
                        return Ok(MotionSkillOutcome::ExecutionFailed {
                            reason: MotionSkillExecutionFailure::MissingPreconditionEvidence,
                        });
                    }
                },
            }
        }
        Ok(MotionSkillOutcome::Ready)
    }

    /// Only report completion when every declared postcondition is verified by complete
    /// observations. Unknown or missing observations are a non-success outcome.
    pub fn evaluate_completion(
        &self,
        evidence: &MotionSkillCompletionEvidence,
    ) -> Result<MotionSkillOutcome, MotionSkillValidationError> {
        self.validate()?;
        if !evidence.now_sec.is_finite() || evidence.now_sec < 0.0 {
            return Err(MotionSkillValidationError::InvalidEvaluationTime);
        }
        if evidence.now_sec < self.issued_at_sec {
            return Ok(MotionSkillOutcome::NotYetValid);
        }
        if self
            .valid_until_sec
            .is_some_and(|until| evidence.now_sec >= until)
        {
            return Ok(MotionSkillOutcome::Expired);
        }
        if evidence
            .recovery_skill_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(MotionSkillValidationError::InvalidRecoverySkillId);
        }
        if !evidence.execution_started {
            return Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::ExecutionNotStarted,
            });
        }

        let required = self
            .postconditions
            .iter()
            .map(|item| item.postcondition_id.as_str())
            .collect::<Vec<_>>();
        for observed in evidence
            .verified_postconditions
            .iter()
            .chain(evidence.failed_postconditions.iter())
        {
            if !required.contains(&observed.as_str()) {
                return Err(MotionSkillValidationError::UnknownPostcondition);
            }
        }
        if evidence
            .verified_postconditions
            .iter()
            .any(|id| evidence.failed_postconditions.contains(id))
        {
            return Err(MotionSkillValidationError::ConflictingPostconditionEvidence);
        }
        if evidence.interrupted {
            return Ok(MotionSkillOutcome::Interrupted {
                recovery_skill_id: evidence.recovery_skill_id.clone(),
            });
        }
        if !evidence.observations_complete {
            return Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::MissingPostconditionEvidence,
            });
        }
        if !evidence.failed_postconditions.is_empty() {
            return Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::PostconditionsFailed,
            });
        }
        if required.iter().all(|id| {
            evidence
                .verified_postconditions
                .iter()
                .any(|seen| seen == id)
        }) {
            return Ok(MotionSkillOutcome::Completed);
        }
        Ok(MotionSkillOutcome::ExecutionFailed {
            reason: MotionSkillExecutionFailure::PostconditionNotVerified,
        })
    }
}

fn valid_constraint_target(constraint: &MotionSkillConstraint) -> bool {
    match (&constraint.kind, &constraint.target) {
        (
            MotionConstraintKind::RootPosition | MotionConstraintKind::EndEffectorPosition,
            MotionConstraintTarget::PositionWorldM(position),
        ) => position.iter().all(|value| value.is_finite()),
        (
            MotionConstraintKind::RootOrientation | MotionConstraintKind::EndEffectorOrientation,
            MotionConstraintTarget::OrientationWorldXyzw(orientation),
        ) => {
            if orientation.iter().any(|value| !value.is_finite()) {
                return false;
            }
            let norm = orientation
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            norm.is_finite() && (norm - 1.0).abs() <= 0.05
        }
        (MotionConstraintKind::DesiredSpeed, MotionConstraintTarget::Scalar(speed)) => {
            speed.is_finite() && *speed >= 0.0
        }
        (
            MotionConstraintKind::CollisionFree
            | MotionConstraintKind::JointLimits
            | MotionConstraintKind::SupportStable,
            MotionConstraintTarget::Boolean(enabled),
        ) => *enabled,
        (MotionConstraintKind::ContactState, MotionConstraintTarget::Boolean(_)) => true,
        (
            MotionConstraintKind::Style | MotionConstraintKind::Gaze,
            MotionConstraintTarget::Label(label),
        ) => !label.trim().is_empty(),
        _ => false,
    }
}

fn valid_constraint_hardness(constraint: &MotionSkillConstraint) -> bool {
    match constraint.kind {
        MotionConstraintKind::CollisionFree
        | MotionConstraintKind::JointLimits
        | MotionConstraintKind::SupportStable => {
            constraint.hardness == MotionConstraintHardness::Hard
        }
        MotionConstraintKind::Style | MotionConstraintKind::Gaze => {
            constraint.hardness == MotionConstraintHardness::Soft
        }
        _ => true,
    }
}

fn validate_start_evidence(
    contract: &MotionSkillContract,
    evidence: &MotionSkillStartEvidence,
) -> Result<(), MotionSkillValidationError> {
    let mut targets = Vec::with_capacity(evidence.target_reachability.len());
    for item in &evidence.target_reachability {
        if item.target_ref.trim().is_empty() {
            return Err(MotionSkillValidationError::InvalidReachabilityEvidence);
        }
        if targets.iter().any(|seen| seen == &item.target_ref) {
            return Err(MotionSkillValidationError::DuplicateReachabilityEvidence);
        }
        targets.push(item.target_ref.as_str());
    }
    for conflict in &evidence.hard_constraint_conflicts {
        if conflict.left_constraint_id == conflict.right_constraint_id
            || conflict.reason.trim().is_empty()
        {
            return Err(MotionSkillValidationError::InvalidConflictEvidence);
        }
        let left = contract
            .constraints
            .iter()
            .find(|item| item.constraint_id == conflict.left_constraint_id);
        let right = contract
            .constraints
            .iter()
            .find(|item| item.constraint_id == conflict.right_constraint_id);
        if !left.is_some_and(|item| item.hardness == MotionConstraintHardness::Hard)
            || !right.is_some_and(|item| item.hardness == MotionConstraintHardness::Hard)
        {
            return Err(MotionSkillValidationError::InvalidConflictEvidence);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> MotionSkillContract {
        MotionSkillContract {
            schema_version: MOTION_SKILL_CONTRACT_SCHEMA_VERSION,
            skill_id: "approach-and-reach-v1".to_owned(),
            intent_id: "intent-42".to_owned(),
            issued_at_sec: 1.0,
            valid_until_sec: Some(5.0),
            preconditions: vec![
                MotionSkillPrecondition::CapabilityAvailable {
                    capability_id: "locomotion".to_owned(),
                },
                MotionSkillPrecondition::TargetReachable {
                    target_ref: "object:cup".to_owned(),
                },
                MotionSkillPrecondition::SupportAvailable,
            ],
            constraints: vec![
                MotionSkillConstraint {
                    constraint_id: "floor-collision".to_owned(),
                    kind: MotionConstraintKind::CollisionFree,
                    hardness: MotionConstraintHardness::Hard,
                    priority: 1,
                    tolerance: 0.0,
                    target: MotionConstraintTarget::Boolean(true),
                },
                MotionSkillConstraint {
                    constraint_id: "desired-style".to_owned(),
                    kind: MotionConstraintKind::Style,
                    hardness: MotionConstraintHardness::Soft,
                    priority: u8::MAX,
                    tolerance: 0.0,
                    target: MotionConstraintTarget::Label("relaxed".to_owned()),
                },
                MotionSkillConstraint {
                    constraint_id: "hand-target".to_owned(),
                    kind: MotionConstraintKind::EndEffectorPosition,
                    hardness: MotionConstraintHardness::Hard,
                    priority: 5,
                    tolerance: 0.02,
                    target: MotionConstraintTarget::PositionWorldM([0.3, 1.0, 0.2]),
                },
            ],
            postconditions: vec![
                MotionSkillPostcondition {
                    postcondition_id: "at-target".to_owned(),
                    kind: MotionSkillPostconditionKind::AtTarget,
                    target_ref: Some("object:cup".to_owned()),
                    tolerance: 0.02,
                },
                MotionSkillPostcondition {
                    postcondition_id: "stable-support".to_owned(),
                    kind: MotionSkillPostconditionKind::StableSupport,
                    target_ref: None,
                    tolerance: 0.01,
                },
            ],
        }
    }

    #[test]
    fn synthetic_contract_fixture_deserializes_and_matches_the_typed_contract() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/research/fixtures/motion-skill-contract-synthetic-v1.json");
        let text =
            std::fs::read_to_string(path).expect("synthetic skill contract fixture must exist");
        let decoded: MotionSkillContract =
            serde_json::from_str(&text).expect("synthetic skill contract fixture must deserialize");
        assert_eq!(decoded, contract());
        decoded
            .validate()
            .expect("synthetic skill contract fixture must validate");
    }

    fn start_evidence(now_sec: f64) -> MotionSkillStartEvidence {
        MotionSkillStartEvidence {
            now_sec,
            available_capabilities: vec!["locomotion".to_owned()],
            target_reachability: vec![TargetReachabilityEvidence {
                target_ref: "object:cup".to_owned(),
                reachable: true,
            }],
            support_available: Some(true),
            hard_constraint_conflicts: Vec::new(),
        }
    }

    #[test]
    fn start_is_expired_at_exclusive_validity_boundary() {
        let skill = contract();
        assert_eq!(
            skill.evaluate_start(&start_evidence(5.0)),
            Ok(MotionSkillOutcome::Expired)
        );
        assert_eq!(
            skill.evaluate_start(&start_evidence(0.5)),
            Ok(MotionSkillOutcome::NotYetValid)
        );
        assert_eq!(
            skill.evaluate_start(&start_evidence(1.0)),
            Ok(MotionSkillOutcome::Ready)
        );
    }

    #[test]
    fn hard_constraints_outrank_soft_style_even_at_maximum_style_priority() {
        let skill = contract();
        let hard = &skill.constraints[0];
        let style = &skill.constraints[1];
        assert_eq!(compare_constraint_precedence(hard, style), Ordering::Less);
        assert_eq!(
            compare_constraint_precedence(style, hard),
            Ordering::Greater
        );
        for hard_priority in [0, 1, 127, u8::MAX] {
            for soft_priority in [0, 1, 127, u8::MAX] {
                let mut hard_variant = hard.clone();
                hard_variant.priority = hard_priority;
                let mut soft_variant = style.clone();
                soft_variant.priority = soft_priority;
                assert_eq!(
                    compare_constraint_precedence(&hard_variant, &soft_variant),
                    Ordering::Less
                );
            }
        }
    }

    #[test]
    fn conflicting_hard_constraints_return_infeasible() {
        let skill = contract();
        let mut evidence = start_evidence(2.0);
        evidence
            .hard_constraint_conflicts
            .push(HardConstraintConflict {
                left_constraint_id: "floor-collision".to_owned(),
                right_constraint_id: "hand-target".to_owned(),
                reason: "target requires penetration of a blocking surface".to_owned(),
            });
        match skill.evaluate_start(&evidence).expect("valid evidence") {
            MotionSkillOutcome::Infeasible { reason } => {
                assert!(reason.contains("floor-collision"));
                assert!(reason.contains("hand-target"));
            }
            other => panic!("expected explicit infeasible outcome, got {other:?}"),
        }
    }

    #[test]
    fn missing_capability_and_missing_physics_evidence_are_not_success() {
        let skill = contract();
        let mut evidence = start_evidence(2.0);
        evidence.available_capabilities.clear();
        assert_eq!(
            skill.evaluate_start(&evidence),
            Ok(MotionSkillOutcome::CapabilityMissing {
                capability_id: "locomotion".to_owned()
            })
        );

        let mut evidence = start_evidence(2.0);
        evidence.support_available = None;
        assert_eq!(
            skill.evaluate_start(&evidence),
            Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::MissingPreconditionEvidence
            })
        );
    }

    #[test]
    fn unreachable_target_is_infeasible_not_completed() {
        let skill = contract();
        let mut evidence = start_evidence(2.0);
        evidence.target_reachability[0].reachable = false;
        assert!(matches!(
            skill.evaluate_start(&evidence),
            Ok(MotionSkillOutcome::Infeasible { .. })
        ));
    }

    #[test]
    fn completion_requires_complete_evidence_for_every_postcondition() {
        let skill = contract();
        let incomplete = MotionSkillCompletionEvidence {
            now_sec: 2.0,
            execution_started: true,
            observations_complete: false,
            verified_postconditions: vec!["at-target".to_owned()],
            failed_postconditions: Vec::new(),
            interrupted: false,
            recovery_skill_id: None,
        };
        assert_eq!(
            skill.evaluate_completion(&incomplete),
            Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::MissingPostconditionEvidence
            })
        );

        let partial = MotionSkillCompletionEvidence {
            now_sec: 2.0,
            execution_started: true,
            observations_complete: true,
            verified_postconditions: vec!["at-target".to_owned()],
            failed_postconditions: Vec::new(),
            interrupted: false,
            recovery_skill_id: None,
        };
        assert_eq!(
            skill.evaluate_completion(&partial),
            Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::PostconditionNotVerified
            })
        );

        let complete = MotionSkillCompletionEvidence {
            now_sec: 2.0,
            execution_started: true,
            observations_complete: true,
            verified_postconditions: vec!["at-target".to_owned(), "stable-support".to_owned()],
            failed_postconditions: Vec::new(),
            interrupted: false,
            recovery_skill_id: None,
        };
        assert_eq!(
            skill.evaluate_completion(&complete),
            Ok(MotionSkillOutcome::Completed)
        );
    }

    #[test]
    fn completion_rejects_unstarted_and_expired_execution() {
        let skill = contract();
        let mut evidence = MotionSkillCompletionEvidence {
            now_sec: 2.0,
            execution_started: false,
            observations_complete: true,
            verified_postconditions: vec!["at-target".to_owned(), "stable-support".to_owned()],
            failed_postconditions: Vec::new(),
            interrupted: false,
            recovery_skill_id: None,
        };
        assert_eq!(
            skill.evaluate_completion(&evidence),
            Ok(MotionSkillOutcome::ExecutionFailed {
                reason: MotionSkillExecutionFailure::ExecutionNotStarted
            })
        );

        evidence.execution_started = true;
        evidence.now_sec = 0.5;
        assert_eq!(
            skill.evaluate_completion(&evidence),
            Ok(MotionSkillOutcome::NotYetValid)
        );

        evidence.now_sec = 5.0;
        assert_eq!(
            skill.evaluate_completion(&evidence),
            Ok(MotionSkillOutcome::Expired)
        );

        evidence.now_sec = 2.0;
        assert_eq!(
            skill.evaluate_completion(&evidence),
            Ok(MotionSkillOutcome::Completed)
        );
    }

    #[test]
    fn interruption_is_explicit_and_carries_optional_recovery_skill() {
        let skill = contract();
        let evidence = MotionSkillCompletionEvidence {
            now_sec: 2.0,
            execution_started: true,
            observations_complete: false,
            verified_postconditions: Vec::new(),
            failed_postconditions: Vec::new(),
            interrupted: true,
            recovery_skill_id: Some("recover-balance-v1".to_owned()),
        };
        assert_eq!(
            skill.evaluate_completion(&evidence),
            Ok(MotionSkillOutcome::Interrupted {
                recovery_skill_id: Some("recover-balance-v1".to_owned())
            })
        );
    }

    #[test]
    fn rejects_unknown_versions_non_finite_targets_and_invalid_hardness() {
        let mut skill = contract();
        skill.schema_version = 2;
        assert_eq!(
            skill.validate(),
            Err(MotionSkillValidationError::UnsupportedSchemaVersion)
        );

        let mut skill = contract();
        skill.constraints[2].target = MotionConstraintTarget::PositionWorldM([f64::NAN, 0.0, 0.0]);
        assert_eq!(
            skill.validate(),
            Err(MotionSkillValidationError::InvalidConstraintTarget)
        );

        let mut skill = contract();
        skill.constraints[1].hardness = MotionConstraintHardness::Hard;
        assert_eq!(
            skill.validate(),
            Err(MotionSkillValidationError::InvalidConstraintHardness)
        );
    }

    #[test]
    fn mandatory_physics_constraints_cannot_be_disabled() {
        for kind in [
            MotionConstraintKind::CollisionFree,
            MotionConstraintKind::JointLimits,
            MotionConstraintKind::SupportStable,
        ] {
            let mut skill = contract();
            skill.constraints[0].kind = kind;
            skill.constraints[0].target = MotionConstraintTarget::Boolean(false);
            assert_eq!(
                skill.validate(),
                Err(MotionSkillValidationError::InvalidConstraintTarget)
            );
        }
    }

    #[test]
    fn start_conflict_evidence_must_reference_two_declared_hard_constraints() {
        let skill = contract();
        let mut evidence = start_evidence(2.0);
        evidence
            .hard_constraint_conflicts
            .push(HardConstraintConflict {
                left_constraint_id: "desired-style".to_owned(),
                right_constraint_id: "hand-target".to_owned(),
                reason: "test conflict".to_owned(),
            });
        assert_eq!(
            skill.evaluate_start(&evidence),
            Err(MotionSkillValidationError::InvalidConflictEvidence)
        );
    }

    #[test]
    fn serialization_round_trip_preserves_typed_contract() {
        let skill = contract();
        let json = serde_json::to_string(&skill).expect("contract serializes");
        let decoded: MotionSkillContract =
            serde_json::from_str(&json).expect("contract deserializes");
        assert_eq!(decoded, skill);
        decoded.validate().expect("round-trip contract validates");
    }

    #[test]
    fn tagged_contract_values_reject_unknown_fields() {
        let mut value = serde_json::to_value(contract()).expect("contract serializes");
        value["preconditions"][0]["unexpected"] = serde_json::Value::Bool(true);
        assert!(
            serde_json::from_value::<MotionSkillContract>(value).is_err(),
            "tagged preconditions must reject unknown fields"
        );

        let mut value = serde_json::to_value(contract()).expect("contract serializes");
        value["constraints"][0]["target"]["unexpected"] = serde_json::Value::Bool(true);
        assert!(
            serde_json::from_value::<MotionSkillContract>(value).is_err(),
            "tagged constraint targets must reject unknown fields"
        );
    }

    #[test]
    fn duplicate_or_unknown_postcondition_evidence_is_rejected() {
        let mut skill = contract();
        skill.postconditions.push(skill.postconditions[0].clone());
        assert_eq!(
            skill.validate(),
            Err(MotionSkillValidationError::DuplicatePostconditionId)
        );

        let skill = contract();
        let evidence = MotionSkillCompletionEvidence {
            now_sec: 2.0,
            execution_started: true,
            observations_complete: true,
            verified_postconditions: vec!["invented-postcondition".to_owned()],
            failed_postconditions: Vec::new(),
            interrupted: false,
            recovery_skill_id: None,
        };
        assert_eq!(
            skill.evaluate_completion(&evidence),
            Err(MotionSkillValidationError::UnknownPostcondition)
        );
    }
}
