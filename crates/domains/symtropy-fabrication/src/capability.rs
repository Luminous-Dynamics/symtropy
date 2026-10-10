// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Multidimensional, evidence-bound fabrication capabilities.
//!
//! Capability is not a character level or a universal tool score. An envelope
//! states the operating region a concrete provider can currently support under
//! explicit modes and conditions. Process admission may project a satisfied
//! envelope into the F4 bootstrap token, but the envelope remains the richer
//! authority record.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{CapabilityEvidence, CapabilityRequirement};

/// Stable identity of one reusable capability need.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityNeedId(StableId);

impl CapabilityNeedId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for CapabilityNeedId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Stable identity of one concrete capability admission decision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityAdmissionId(StableId);

impl CapabilityAdmissionId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for CapabilityAdmissionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Revisioned authority evidence for one capability envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
}

impl CapabilityEvidenceRef {
    pub fn new(
        authority_id: StableId,
        evidence_id: StableId,
        revision: u64,
        digest: impl Into<String>,
    ) -> Result<Self, CapabilityError> {
        let digest = digest.into();
        if digest.is_empty() || digest.len() > 256 {
            return Err(CapabilityError::InvalidEvidenceDigest(digest));
        }
        Ok(Self {
            authority_id,
            evidence_id,
            revision,
            digest,
        })
    }
}

/// One measurable operating axis of a provider capability.
///
/// Units and dimensional meaning are owned by `axis_id`; values use integers
/// so deterministic admission is independent of floating-point behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAxisRange {
    pub axis_id: StableId,
    pub lower: i64,
    pub upper: i64,
    /// Smallest reliably controllable/observable step in axis units.
    pub resolution: u64,
}

impl CapabilityAxisRange {
    pub fn new(
        axis_id: StableId,
        lower: i64,
        upper: i64,
        resolution: u64,
    ) -> Result<Self, CapabilityError> {
        if lower > upper {
            return Err(CapabilityError::InvalidAxisRange {
                axis_id,
                lower,
                upper,
            });
        }
        if resolution == 0 {
            return Err(CapabilityError::ZeroResolution(axis_id));
        }
        Ok(Self {
            axis_id,
            lower,
            upper,
            resolution,
        })
    }
}

/// Required operating interval for one capability axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAxisNeed {
    pub axis_id: StableId,
    pub lower: i64,
    pub upper: i64,
    /// When present, a provider with coarser resolution cannot satisfy the need.
    pub max_resolution: Option<u64>,
}

impl CapabilityAxisNeed {
    pub fn new(
        axis_id: StableId,
        lower: i64,
        upper: i64,
        max_resolution: Option<u64>,
    ) -> Result<Self, CapabilityError> {
        if lower > upper {
            return Err(CapabilityError::InvalidAxisNeed {
                axis_id,
                lower,
                upper,
            });
        }
        if max_resolution == Some(0) {
            return Err(CapabilityError::ZeroMaximumResolution(axis_id));
        }
        Ok(Self {
            axis_id,
            lower,
            upper,
            max_resolution,
        })
    }
}

/// Current operating envelope of a concrete tool, machine, operator, fixture,
/// or cooperating system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEnvelope {
    pub provider_id: StableId,
    pub provider_revision: u64,
    pub capability_id: StableId,
    pub mode_id: StableId,
    pub axes: Vec<CapabilityAxisRange>,
    /// Required environmental/configuration predicates already established by
    /// the supplying authority, such as `condition:shield-gas:argon`.
    pub conditions: Vec<StableId>,
    pub evidence: CapabilityEvidenceRef,
}

impl CapabilityEnvelope {
    pub fn new(
        provider_id: StableId,
        provider_revision: u64,
        capability_id: StableId,
        mode_id: StableId,
        axes: Vec<CapabilityAxisRange>,
        conditions: Vec<StableId>,
        evidence: CapabilityEvidenceRef,
    ) -> Result<Self, CapabilityError> {
        reject_duplicate_axes(&axes)?;
        reject_duplicate_ids(&conditions, CapabilityError::DuplicateCondition)?;
        Ok(Self {
            provider_id,
            provider_revision,
            capability_id,
            mode_id,
            axes,
            conditions,
            evidence,
        })
    }

    pub fn axis(&self, axis_id: &StableId) -> Option<&CapabilityAxisRange> {
        self.axes.iter().find(|axis| &axis.axis_id == axis_id)
    }
}

/// Reusable process need. It describes a feasible operating region rather than
/// assigning a single difficulty or level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityNeed {
    pub id: CapabilityNeedId,
    pub capability_id: StableId,
    pub required_mode_id: Option<StableId>,
    pub axes: Vec<CapabilityAxisNeed>,
    pub required_conditions: Vec<StableId>,
}

impl CapabilityNeed {
    pub fn new(
        id: CapabilityNeedId,
        capability_id: StableId,
        required_mode_id: Option<StableId>,
        axes: Vec<CapabilityAxisNeed>,
        required_conditions: Vec<StableId>,
    ) -> Result<Self, CapabilityError> {
        reject_duplicate_axis_needs(&axes)?;
        reject_duplicate_ids(
            &required_conditions,
            CapabilityError::DuplicateRequiredCondition,
        )?;
        Ok(Self {
            id,
            capability_id,
            required_mode_id,
            axes,
            required_conditions,
        })
    }

    /// Evaluates containment and categorical predicates without inventing a
    /// weighted score. Every mismatch remains independently visible.
    pub fn evaluate(
        &self,
        admission_id: CapabilityAdmissionId,
        envelope: &CapabilityEnvelope,
    ) -> CapabilityAdmission {
        let mut failures = Vec::new();

        if self.capability_id != envelope.capability_id {
            failures.push(CapabilityMismatch::CapabilityId {
                required: self.capability_id.clone(),
                provided: envelope.capability_id.clone(),
            });
        }

        if let Some(required_mode) = &self.required_mode_id {
            if required_mode != &envelope.mode_id {
                failures.push(CapabilityMismatch::Mode {
                    required: required_mode.clone(),
                    provided: envelope.mode_id.clone(),
                });
            }
        }

        for condition in &self.required_conditions {
            if !envelope.conditions.contains(condition) {
                failures.push(CapabilityMismatch::MissingCondition(condition.clone()));
            }
        }

        for required in &self.axes {
            let Some(provided) = envelope.axis(&required.axis_id) else {
                failures.push(CapabilityMismatch::MissingAxis(required.axis_id.clone()));
                continue;
            };

            if provided.lower > required.lower || provided.upper < required.upper {
                failures.push(CapabilityMismatch::OperatingRange {
                    axis_id: required.axis_id.clone(),
                    required_lower: required.lower,
                    required_upper: required.upper,
                    provided_lower: provided.lower,
                    provided_upper: provided.upper,
                });
            }

            if let Some(max_resolution) = required.max_resolution {
                if provided.resolution > max_resolution {
                    failures.push(CapabilityMismatch::Resolution {
                        axis_id: required.axis_id.clone(),
                        maximum: max_resolution,
                        provided: provided.resolution,
                    });
                }
            }
        }

        let outcome = if failures.is_empty() {
            CapabilityOutcome::Satisfied
        } else {
            CapabilityOutcome::Unsatisfied
        };

        CapabilityAdmission {
            id: admission_id,
            need_id: self.id.clone(),
            provider_id: envelope.provider_id.clone(),
            provider_revision: envelope.provider_revision,
            envelope_evidence: envelope.evidence.clone(),
            outcome,
            failures,
        }
    }

    /// Compatibility shim for F4 process admission. The scalar is deliberately
    /// binary: all multidimensional reasoning happened here first.
    pub fn bootstrap_requirement(&self) -> CapabilityRequirement {
        CapabilityRequirement {
            capability_id: self.id.stable_id().clone(),
            minimum_value: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOutcome {
    Satisfied,
    Unsatisfied,
}

/// Durable result of comparing one exact provider revision to one need.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAdmission {
    pub id: CapabilityAdmissionId,
    pub need_id: CapabilityNeedId,
    pub provider_id: StableId,
    pub provider_revision: u64,
    pub envelope_evidence: CapabilityEvidenceRef,
    pub outcome: CapabilityOutcome,
    pub failures: Vec<CapabilityMismatch>,
}

impl CapabilityAdmission {
    pub const fn is_satisfied(&self) -> bool {
        matches!(self.outcome, CapabilityOutcome::Satisfied)
    }

    /// Projects a successful rich admission into the F4 bootstrap token. The
    /// process stores this admission identity rather than a fabricated level.
    pub fn bootstrap_evidence(&self) -> Result<CapabilityEvidence, CapabilityError> {
        if !self.is_satisfied() {
            return Err(CapabilityError::UnsatisfiedAdmission(self.id.clone()));
        }
        Ok(CapabilityEvidence {
            capability_id: self.need_id.stable_id().clone(),
            available_value: 1,
            evidence_id: self.id.stable_id().clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mismatch", rename_all = "snake_case")]
pub enum CapabilityMismatch {
    CapabilityId {
        required: StableId,
        provided: StableId,
    },
    Mode {
        required: StableId,
        provided: StableId,
    },
    MissingCondition(StableId),
    MissingAxis(StableId),
    OperatingRange {
        axis_id: StableId,
        required_lower: i64,
        required_upper: i64,
        provided_lower: i64,
        provided_upper: i64,
    },
    Resolution {
        axis_id: StableId,
        maximum: u64,
        provided: u64,
    },
}

#[derive(Debug)]
pub enum CapabilityError {
    InvalidEvidenceDigest(String),
    InvalidAxisRange {
        axis_id: StableId,
        lower: i64,
        upper: i64,
    },
    InvalidAxisNeed {
        axis_id: StableId,
        lower: i64,
        upper: i64,
    },
    ZeroResolution(StableId),
    ZeroMaximumResolution(StableId),
    DuplicateAxis(StableId),
    DuplicateAxisNeed(StableId),
    DuplicateCondition(StableId),
    DuplicateRequiredCondition(StableId),
    UnsatisfiedAdmission(CapabilityAdmissionId),
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "capability evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::InvalidAxisRange {
                axis_id,
                lower,
                upper,
            } => write!(formatter, "capability axis {axis_id} has invalid range {lower}..{upper}"),
            Self::InvalidAxisNeed {
                axis_id,
                lower,
                upper,
            } => write!(formatter, "capability need axis {axis_id} has invalid range {lower}..{upper}"),
            Self::ZeroResolution(axis_id) => {
                write!(formatter, "capability axis {axis_id} requires non-zero resolution")
            }
            Self::ZeroMaximumResolution(axis_id) => write!(
                formatter,
                "capability need axis {axis_id} requires non-zero maximum resolution"
            ),
            Self::DuplicateAxis(axis_id) => {
                write!(formatter, "capability envelope repeats axis {axis_id}")
            }
            Self::DuplicateAxisNeed(axis_id) => {
                write!(formatter, "capability need repeats axis {axis_id}")
            }
            Self::DuplicateCondition(condition) => {
                write!(formatter, "capability envelope repeats condition {condition}")
            }
            Self::DuplicateRequiredCondition(condition) => {
                write!(formatter, "capability need repeats condition {condition}")
            }
            Self::UnsatisfiedAdmission(id) => {
                write!(formatter, "capability admission {id} is not satisfied")
            }
        }
    }
}

impl Error for CapabilityError {}

fn reject_duplicate_axes(axes: &[CapabilityAxisRange]) -> Result<(), CapabilityError> {
    for (index, axis) in axes.iter().enumerate() {
        if axes[..index]
            .iter()
            .any(|existing| existing.axis_id == axis.axis_id)
        {
            return Err(CapabilityError::DuplicateAxis(axis.axis_id.clone()));
        }
    }
    Ok(())
}

fn reject_duplicate_axis_needs(axes: &[CapabilityAxisNeed]) -> Result<(), CapabilityError> {
    for (index, axis) in axes.iter().enumerate() {
        if axes[..index]
            .iter()
            .any(|existing| existing.axis_id == axis.axis_id)
        {
            return Err(CapabilityError::DuplicateAxisNeed(axis.axis_id.clone()));
        }
    }
    Ok(())
}

fn reject_duplicate_ids(
    ids: &[StableId],
    error: fn(StableId) -> CapabilityError,
) -> Result<(), CapabilityError> {
    for (index, id) in ids.iter().enumerate() {
        if ids[..index].contains(id) {
            return Err(error(id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn evidence() -> CapabilityEvidenceRef {
        CapabilityEvidenceRef::new(
            id("authority:tool-diagnostics"),
            id("evidence:welder:17"),
            17,
            "digest:welder:17",
        )
        .unwrap()
    }

    fn welding_need() -> CapabilityNeed {
        CapabilityNeed::new(
            CapabilityNeedId::new(id("capability-need:weld:patch-conduit")),
            id("capability:welding"),
            Some(id("mode:welding:controlled-arc")),
            vec![
                CapabilityAxisNeed::new(id("axis:heat-input"), 780, 840, Some(5)).unwrap(),
                CapabilityAxisNeed::new(id("axis:travel-rate"), 90, 110, Some(2)).unwrap(),
            ],
            vec![id("condition:surface-clean"), id("condition:shielding-active")],
        )
        .unwrap()
    }

    fn welder() -> CapabilityEnvelope {
        CapabilityEnvelope::new(
            id("tool:welder:field-3"),
            12,
            id("capability:welding"),
            id("mode:welding:controlled-arc"),
            vec![
                CapabilityAxisRange::new(id("axis:heat-input"), 700, 900, 4).unwrap(),
                CapabilityAxisRange::new(id("axis:travel-rate"), 70, 130, 1).unwrap(),
            ],
            vec![id("condition:surface-clean"), id("condition:shielding-active")],
            evidence(),
        )
        .unwrap()
    }

    #[test]
    fn multidimensional_envelope_satisfies_only_when_every_axis_and_condition_fit() {
        let admission = welding_need().evaluate(
            CapabilityAdmissionId::new(id("capability-admission:weld:1")),
            &welder(),
        );
        assert!(admission.is_satisfied());
        assert!(admission.failures.is_empty());
    }

    #[test]
    fn coarse_tool_cannot_hide_behind_large_operating_range() {
        let mut envelope = welder();
        envelope.axes[0].resolution = 10;
        let admission = welding_need().evaluate(
            CapabilityAdmissionId::new(id("capability-admission:weld:2")),
            &envelope,
        );
        assert!(matches!(
            admission.failures.as_slice(),
            [CapabilityMismatch::Resolution { axis_id, maximum: 5, provided: 10 }]
                if axis_id == &id("axis:heat-input")
        ));
    }

    #[test]
    fn missing_operating_condition_is_explicit_not_absorbed_into_score() {
        let mut envelope = welder();
        envelope.conditions.retain(|condition| condition != &id("condition:shielding-active"));
        let admission = welding_need().evaluate(
            CapabilityAdmissionId::new(id("capability-admission:weld:3")),
            &envelope,
        );
        assert!(admission.failures.contains(&CapabilityMismatch::MissingCondition(
            id("condition:shielding-active")
        )));
    }

    #[test]
    fn successful_admission_projects_only_binary_bootstrap_token() {
        let need = welding_need();
        let admission = need.evaluate(
            CapabilityAdmissionId::new(id("capability-admission:weld:4")),
            &welder(),
        );
        let requirement = need.bootstrap_requirement();
        let token = admission.bootstrap_evidence().unwrap();
        assert_eq!(requirement.minimum_value, 1);
        assert_eq!(token.available_value, 1);
        assert!(requirement.is_satisfied_by(&[token]));
    }

    #[test]
    fn serialized_envelope_has_no_level_quality_or_score_field() {
        let value = serde_json::to_value(welder()).unwrap();
        assert!(value.get("level").is_none());
        assert!(value.get("quality").is_none());
        assert!(value.get("score").is_none());
    }
}
