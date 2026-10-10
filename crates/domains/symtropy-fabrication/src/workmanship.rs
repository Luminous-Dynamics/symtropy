// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workmanship observations for fabrication processes.
//!
//! Workmanship is evidence about how work occurred, not a universal quality
//! score and not a declaration of engineering fitness. Functional evaluators
//! may later consume these observations alongside physical matter evidence.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{ProcessExecutionId, ProcessSpecId};

/// Stable identity of one observation captured during fabrication.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkmanshipObservationId(StableId);

impl WorkmanshipObservationId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for WorkmanshipObservationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Exact authority evidence supporting one workmanship observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkmanshipEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
}

impl WorkmanshipEvidenceRef {
    pub fn new(
        authority_id: StableId,
        evidence_id: StableId,
        revision: u64,
        digest: impl Into<String>,
    ) -> Result<Self, WorkmanshipError> {
        let digest = digest.into();
        if digest.is_empty() || digest.len() > 256 {
            return Err(WorkmanshipError::InvalidEvidenceDigest(digest));
        }
        Ok(Self {
            authority_id,
            evidence_id,
            revision,
            digest,
        })
    }
}

/// Bounded measurement. Units and sign convention are owned by the stable
/// dimension identity; intervals preserve uncertainty instead of hiding it in
/// an arbitrary confidence or quality score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementInterval {
    pub lower: i64,
    pub upper: i64,
    pub resolution: u64,
}

impl MeasurementInterval {
    pub fn new(lower: i64, upper: i64, resolution: u64) -> Result<Self, WorkmanshipError> {
        if lower > upper {
            return Err(WorkmanshipError::InvalidMeasurementInterval { lower, upper });
        }
        if resolution == 0 {
            return Err(WorkmanshipError::ZeroMeasurementResolution);
        }
        Ok(Self {
            lower,
            upper,
            resolution,
        })
    }
}

/// Evidence value for one named workmanship dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value_kind", rename_all = "snake_case")]
pub enum WorkmanshipValue {
    /// Quantitative interval such as alignment error or heat exposure.
    Measurement { interval: MeasurementInterval },
    /// Stable categorical observation such as surface condition or bead form.
    Category { value_id: StableId },
    /// Three-valued predicate for directly observed conditions.
    Predicate { state: ObservationState },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationState {
    Present,
    Absent,
    Unknown,
}

/// One provenance-bearing observation made during a process execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkmanshipObservation {
    pub id: WorkmanshipObservationId,
    /// Stable semantic dimension such as `workmanship:alignment-error`.
    pub dimension_id: StableId,
    pub value: WorkmanshipValue,
    pub evidence: WorkmanshipEvidenceRef,
}

/// Append-only evidence vector for one exact process specification/execution.
///
/// Multiple observations may legitimately address the same dimension when they
/// come from different instruments or stages. Observation identity, not
/// dimension identity, is therefore unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkmanshipVector {
    pub execution_id: ProcessExecutionId,
    pub spec_id: ProcessSpecId,
    pub spec_revision: u64,
    observations: Vec<WorkmanshipObservation>,
}

impl WorkmanshipVector {
    pub fn new(
        execution_id: ProcessExecutionId,
        spec_id: ProcessSpecId,
        spec_revision: u64,
    ) -> Self {
        Self {
            execution_id,
            spec_id,
            spec_revision,
            observations: Vec::new(),
        }
    }

    pub fn observations(&self) -> &[WorkmanshipObservation] {
        &self.observations
    }

    pub fn observations_for(&self, dimension_id: &StableId) -> Vec<&WorkmanshipObservation> {
        self.observations
            .iter()
            .filter(|observation| &observation.dimension_id == dimension_id)
            .collect()
    }

    /// Adds evidence without overwriting prior observations.
    pub fn record(&mut self, observation: WorkmanshipObservation) -> Result<(), WorkmanshipError> {
        if self
            .observations
            .iter()
            .any(|existing| existing.id == observation.id)
        {
            return Err(WorkmanshipError::DuplicateObservation(observation.id));
        }
        self.observations.push(observation);
        Ok(())
    }
}

#[derive(Debug)]
pub enum WorkmanshipError {
    InvalidEvidenceDigest(String),
    InvalidMeasurementInterval { lower: i64, upper: i64 },
    ZeroMeasurementResolution,
    DuplicateObservation(WorkmanshipObservationId),
}

impl fmt::Display for WorkmanshipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "workmanship evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::InvalidMeasurementInterval { lower, upper } => write!(
                formatter,
                "workmanship measurement interval is invalid: {lower}..{upper}"
            ),
            Self::ZeroMeasurementResolution => {
                write!(formatter, "workmanship measurement resolution must be non-zero")
            }
            Self::DuplicateObservation(id) => {
                write!(formatter, "workmanship observation {id} already exists")
            }
        }
    }
}

impl Error for WorkmanshipError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn evidence(name: &str, revision: u64) -> WorkmanshipEvidenceRef {
        WorkmanshipEvidenceRef::new(
            id("authority:process-instrumentation"),
            id(name),
            revision,
            format!("digest:{name}:{revision}"),
        )
        .unwrap()
    }

    fn vector() -> WorkmanshipVector {
        WorkmanshipVector::new(
            ProcessExecutionId::new(id("process-execution:patch:1")),
            ProcessSpecId::new(id("process-spec:seal")),
            3,
        )
    }

    #[test]
    fn preserves_measurement_uncertainty_as_interval() {
        let mut vector = vector();
        vector
            .record(WorkmanshipObservation {
                id: WorkmanshipObservationId::new(id("observation:alignment:1")),
                dimension_id: id("workmanship:alignment-error-um"),
                value: WorkmanshipValue::Measurement {
                    interval: MeasurementInterval::new(-18, 24, 2).unwrap(),
                },
                evidence: evidence("evidence:alignment:1", 1),
            })
            .unwrap();

        let observations = vector.observations_for(&id("workmanship:alignment-error-um"));
        assert_eq!(observations.len(), 1);
        assert!(matches!(
            &observations[0].value,
            WorkmanshipValue::Measurement { interval }
                if interval.lower == -18 && interval.upper == 24
        ));
    }

    #[test]
    fn two_instruments_can_observe_same_dimension_without_overwrite() {
        let mut vector = vector();
        for ordinal in 1..=2 {
            vector
                .record(WorkmanshipObservation {
                    id: WorkmanshipObservationId::new(id(&format!("observation:heat:{ordinal}"))),
                    dimension_id: id("workmanship:heat-input"),
                    value: WorkmanshipValue::Measurement {
                        interval: MeasurementInterval::new(790, 810, 5).unwrap(),
                    },
                    evidence: evidence(&format!("evidence:heat:{ordinal}"), ordinal),
                })
                .unwrap();
        }
        assert_eq!(vector.observations_for(&id("workmanship:heat-input")).len(), 2);
    }

    #[test]
    fn unknown_is_distinct_from_absent() {
        assert_ne!(ObservationState::Unknown, ObservationState::Absent);
    }

    #[test]
    fn duplicate_observation_identity_is_rejected() {
        let mut vector = vector();
        let observation = WorkmanshipObservation {
            id: WorkmanshipObservationId::new(id("observation:surface:1")),
            dimension_id: id("workmanship:surface-clean"),
            value: WorkmanshipValue::Predicate {
                state: ObservationState::Present,
            },
            evidence: evidence("evidence:surface:1", 1),
        };
        vector.record(observation.clone()).unwrap();
        assert!(matches!(
            vector.record(observation),
            Err(WorkmanshipError::DuplicateObservation(_))
        ));
    }

    #[test]
    fn serialization_contains_no_universal_quality_score() {
        let value = serde_json::to_value(vector()).unwrap();
        assert!(value.get("quality").is_none());
        assert!(value.get("score").is_none());
        assert!(value.get("grade").is_none());
    }
}
