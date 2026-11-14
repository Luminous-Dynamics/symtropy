// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-gated causal transfers between distinct hazard events.
//!
//! This module describes when a pre-qualified transfer is admissible. It does
//! not infer causality from correlation or implement physical propagation.

use super::{EvidenceClass, EvidenceRecord, EvidenceStatus, HazardKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One explicitly identified simulated event with digest-bound outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardEventEvidence {
    pub event_id: String,
    pub hazard: HazardKind,
    /// Event onset on the scenario's declared time axis, in seconds.
    pub onset_s: f64,
    /// Output field name -> digest of the serialized output artifact.
    pub output_digests_sha256: BTreeMap<String, String>,
}

impl HazardEventEvidence {
    pub fn validate(&self) -> Result<(), EventValidationError> {
        if !nonblank(&self.event_id) {
            return Err(EventValidationError::EmptyEventId);
        }
        if !self.onset_s.is_finite() || self.onset_s < 0.0 {
            return Err(EventValidationError::InvalidOnset);
        }
        for (field, digest) in &self.output_digests_sha256 {
            if !nonblank(field) {
                return Err(EventValidationError::EmptyOutputField);
            }
            if !is_sha256(digest) {
                return Err(EventValidationError::InvalidOutputDigest(field.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventValidationError {
    EmptyEventId,
    InvalidOnset,
    EmptyOutputField,
    InvalidOutputDigest(String),
}

/// A declared field transfer between two hazard-specific models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalEdge {
    pub edge_id: String,
    pub rule_id: String,
    pub model_revision: String,
    pub source_hazard: HazardKind,
    pub target_hazard: HazardKind,
    pub source_output_field: String,
    pub target_input_field: String,
    pub source_units: String,
    pub target_units: String,
    /// Required when source and target unit labels differ.
    pub unit_conversion_model: Option<String>,
    /// Additional target inputs that must be present before propagation.
    pub required_target_inputs: BTreeSet<String>,
    /// Permitted lag from source event onset to the dependent target event.
    pub min_delay_s: f64,
    pub max_delay_s: f64,
    /// Evidence for this causal mechanism, not just for either model in isolation.
    pub evidence: EvidenceRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CausalEdgeValidationError {
    EmptyIdentifier,
    EmptyFieldOrUnit,
    InvalidDelayEnvelope,
    MissingUnitConversionModel,
    EmptyRequiredInput,
    EmptyEvidenceSuite,
    EvidenceSourceNotHttps,
}

/// Why the dependent event cannot be activated by this edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferRefusal {
    InvalidEdge(CausalEdgeValidationError),
    InvalidSourceEvent(EventValidationError),
    InvalidTargetTime,
    SourceHazardMismatch,
    MissingSourceOutput(String),
    MissingPrerequisite(String),
    TimeOutsideDelayWindow,
    EvidenceFailed,
    EvidenceUnavailable,
    EvidenceNotApplicable,
    CausalMechanismNotEstablished,
}

/// An admissible, digest-bound transfer. This records permission to pass an
/// output to a qualified downstream model; it is not a prediction of the
/// downstream event by itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdmittedTransfer {
    pub edge_id: String,
    pub rule_id: String,
    pub source_event_id: String,
    pub source_hazard: HazardKind,
    pub target_hazard: HazardKind,
    pub source_output_field: String,
    pub source_output_digest_sha256: String,
    pub target_input_field: String,
    pub source_units: String,
    pub target_units: String,
    pub unit_conversion_model: Option<String>,
    pub source_onset_s: f64,
    pub target_event_time_s: f64,
    pub elapsed_delay_s: f64,
    pub evidence_suite_id: String,
}

/// Evidence classes that can substantiate a physical/causal transition.
/// Structural invariants and presentation quality are not enough by themselves.
fn establishes_mechanism(class: EvidenceClass) -> bool {
    matches!(
        class,
        EvidenceClass::AnalyticalOrManufactured
            | EvidenceClass::PublishedBenchmark
            | EvidenceClass::ReferenceDifferential
            | EvidenceClass::HeldOutObservation
    )
}

impl CausalEdge {
    pub fn validate(&self) -> Result<(), CausalEdgeValidationError> {
        if !nonblank(&self.edge_id) || !nonblank(&self.rule_id) || !nonblank(&self.model_revision) {
            return Err(CausalEdgeValidationError::EmptyIdentifier);
        }
        if !nonblank(&self.source_output_field)
            || !nonblank(&self.target_input_field)
            || !nonblank(&self.source_units)
            || !nonblank(&self.target_units)
        {
            return Err(CausalEdgeValidationError::EmptyFieldOrUnit);
        }
        if !self.min_delay_s.is_finite()
            || !self.max_delay_s.is_finite()
            || self.min_delay_s < 0.0
            || self.max_delay_s < self.min_delay_s
        {
            return Err(CausalEdgeValidationError::InvalidDelayEnvelope);
        }
        if self.source_units != self.target_units
            && self
                .unit_conversion_model
                .as_deref()
                .is_none_or(|model| !nonblank(model))
        {
            return Err(CausalEdgeValidationError::MissingUnitConversionModel);
        }
        if self
            .required_target_inputs
            .iter()
            .any(|input| !nonblank(input))
        {
            return Err(CausalEdgeValidationError::EmptyRequiredInput);
        }
        if !nonblank(&self.evidence.suite_id) {
            return Err(CausalEdgeValidationError::EmptyEvidenceSuite);
        }
        if self
            .evidence
            .source_uri
            .as_deref()
            .is_some_and(|uri| !uri.starts_with("https://"))
        {
            return Err(CausalEdgeValidationError::EvidenceSourceNotHttps);
        }
        Ok(())
    }

    /// Admit a cascade only when the declared source output, causal evidence,
    /// temporal window, units and target prerequisites are all valid.
    pub fn admit(
        &self,
        source: &HazardEventEvidence,
        target_event_time_s: f64,
        available_target_inputs: &BTreeSet<String>,
    ) -> Result<AdmittedTransfer, TransferRefusal> {
        self.validate().map_err(TransferRefusal::InvalidEdge)?;
        source
            .validate()
            .map_err(TransferRefusal::InvalidSourceEvent)?;

        if !target_event_time_s.is_finite() || target_event_time_s < 0.0 {
            return Err(TransferRefusal::InvalidTargetTime);
        }
        if source.hazard != self.source_hazard {
            return Err(TransferRefusal::SourceHazardMismatch);
        }

        let source_digest = source
            .output_digests_sha256
            .get(&self.source_output_field)
            .ok_or_else(|| {
                TransferRefusal::MissingSourceOutput(self.source_output_field.clone())
            })?;

        match self.evidence.status {
            EvidenceStatus::Fail => return Err(TransferRefusal::EvidenceFailed),
            EvidenceStatus::Unavailable => return Err(TransferRefusal::EvidenceUnavailable),
            EvidenceStatus::NotApplicable => return Err(TransferRefusal::EvidenceNotApplicable),
            EvidenceStatus::Pass => {}
        }
        if !establishes_mechanism(self.evidence.class) {
            return Err(TransferRefusal::CausalMechanismNotEstablished);
        }

        let elapsed_delay_s = target_event_time_s - source.onset_s;
        if elapsed_delay_s < self.min_delay_s || elapsed_delay_s > self.max_delay_s {
            return Err(TransferRefusal::TimeOutsideDelayWindow);
        }

        if let Some(missing) = self
            .required_target_inputs
            .iter()
            .find(|input| !available_target_inputs.contains(*input))
        {
            return Err(TransferRefusal::MissingPrerequisite(missing.clone()));
        }

        Ok(AdmittedTransfer {
            edge_id: self.edge_id.clone(),
            rule_id: self.rule_id.clone(),
            source_event_id: source.event_id.clone(),
            source_hazard: source.hazard,
            target_hazard: self.target_hazard,
            source_output_field: self.source_output_field.clone(),
            source_output_digest_sha256: source_digest.clone(),
            target_input_field: self.target_input_field.clone(),
            source_units: self.source_units.clone(),
            target_units: self.target_units.clone(),
            unit_conversion_model: self.unit_conversion_model.clone(),
            source_onset_s: source.onset_s,
            target_event_time_s,
            elapsed_delay_s,
            evidence_suite_id: self.evidence.suite_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge() -> CausalEdge {
        CausalEdge {
            edge_id: "wildfire-to-postfire-debris-flow".into(),
            rule_id: "regional-postfire-runoff-v1".into(),
            model_revision: "debris-model-rev-01".into(),
            source_hazard: HazardKind::Wildfire,
            target_hazard: HazardKind::DebrisFlow,
            source_output_field: "burn_severity_index".into(),
            target_input_field: "burn_severity_index".into(),
            source_units: "1".into(),
            target_units: "1".into(),
            unit_conversion_model: None,
            required_target_inputs: [
                "rainfall_30min_mm".into(),
                "slope_grid".into(),
                "soil_moisture".into(),
            ]
            .into_iter()
            .collect(),
            min_delay_s: 3_600.0,
            max_delay_s: 259_200.0,
            evidence: EvidenceRecord {
                class: EvidenceClass::PublishedBenchmark,
                status: EvidenceStatus::Pass,
                suite_id: "postfire-debris-flow-reference-v1".into(),
                source_uri: Some("https://www.usgs.gov/landslide-hazards".into()),
                detail: "Evidence profile for this synthetic contract fixture".into(),
            },
        }
    }

    fn source() -> HazardEventEvidence {
        HazardEventEvidence {
            event_id: "wildfire-event-001".into(),
            hazard: HazardKind::Wildfire,
            onset_s: 0.0,
            output_digests_sha256: [("burn_severity_index".into(), "a".repeat(64))]
                .into_iter()
                .collect(),
        }
    }

    fn prerequisites() -> BTreeSet<String> {
        [
            "rainfall_30min_mm".into(),
            "slope_grid".into(),
            "soil_moisture".into(),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn qualified_transfer_preserves_source_provenance_and_delay() {
        let admitted = edge().admit(&source(), 7_200.0, &prerequisites()).unwrap();
        assert_eq!(admitted.source_event_id, "wildfire-event-001");
        assert_eq!(admitted.target_hazard, HazardKind::DebrisFlow);
        assert_eq!(admitted.source_output_digest_sha256, "a".repeat(64));
        assert_eq!(admitted.elapsed_delay_s, 7_200.0);
        assert_eq!(
            admitted.evidence_suite_id,
            "postfire-debris-flow-reference-v1"
        );
    }

    #[test]
    fn secondary_hazard_is_blocked_without_rainfall_or_other_prerequisites() {
        let mut available = prerequisites();
        available.remove("rainfall_30min_mm");
        assert_eq!(
            edge().admit(&source(), 7_200.0, &available),
            Err(TransferRefusal::MissingPrerequisite(
                "rainfall_30min_mm".into()
            ))
        );
    }

    #[test]
    fn correlation_or_visual_quality_cannot_establish_a_causal_edge() {
        let mut candidate = edge();
        candidate.evidence.class = EvidenceClass::PresentationQuality;
        assert_eq!(
            candidate.admit(&source(), 7_200.0, &prerequisites()),
            Err(TransferRefusal::CausalMechanismNotEstablished)
        );

        candidate.evidence.class = EvidenceClass::StructuralInvariant;
        assert_eq!(
            candidate.admit(&source(), 7_200.0, &prerequisites()),
            Err(TransferRefusal::CausalMechanismNotEstablished)
        );
    }

    #[test]
    fn failed_or_unavailable_mechanism_evidence_never_activates_edge() {
        let mut candidate = edge();
        candidate.evidence.status = EvidenceStatus::Fail;
        assert_eq!(
            candidate.admit(&source(), 7_200.0, &prerequisites()),
            Err(TransferRefusal::EvidenceFailed)
        );
        candidate.evidence.status = EvidenceStatus::Unavailable;
        assert_eq!(
            candidate.admit(&source(), 7_200.0, &prerequisites()),
            Err(TransferRefusal::EvidenceUnavailable)
        );
    }

    #[test]
    fn target_event_must_fall_inside_declared_causal_delay_window() {
        assert_eq!(
            edge().admit(&source(), 300.0, &prerequisites()),
            Err(TransferRefusal::TimeOutsideDelayWindow)
        );
        assert_eq!(
            edge().admit(&source(), 300_000.0, &prerequisites()),
            Err(TransferRefusal::TimeOutsideDelayWindow)
        );
    }

    #[test]
    fn source_hazard_and_source_field_must_match_declared_edge() {
        let mut wrong_hazard = source();
        wrong_hazard.hazard = HazardKind::Flood;
        assert_eq!(
            edge().admit(&wrong_hazard, 7_200.0, &prerequisites()),
            Err(TransferRefusal::SourceHazardMismatch)
        );

        let mut missing_output = source();
        missing_output.output_digests_sha256.clear();
        assert_eq!(
            edge().admit(&missing_output, 7_200.0, &prerequisites()),
            Err(TransferRefusal::MissingSourceOutput(
                "burn_severity_index".into()
            ))
        );
    }

    #[test]
    fn incompatible_units_require_an_explicit_conversion_model() {
        let mut candidate = edge();
        candidate.source_units = "mm".into();
        candidate.target_units = "m".into();
        assert_eq!(
            candidate.validate(),
            Err(CausalEdgeValidationError::MissingUnitConversionModel)
        );
        candidate.unit_conversion_model = Some("mm-to-m-v1".into());
        assert_eq!(candidate.validate(), Ok(()));
    }

    #[test]
    fn malformed_source_digest_and_negative_onset_fail_closed() {
        let mut invalid_digest = source();
        invalid_digest
            .output_digests_sha256
            .insert("burn_severity_index".into(), "not-a-digest".into());
        assert_eq!(
            edge().admit(&invalid_digest, 7_200.0, &prerequisites()),
            Err(TransferRefusal::InvalidSourceEvent(
                EventValidationError::InvalidOutputDigest("burn_severity_index".into())
            ))
        );

        let mut invalid_time = source();
        invalid_time.onset_s = -1.0;
        assert_eq!(
            edge().admit(&invalid_time, 7_200.0, &prerequisites()),
            Err(TransferRefusal::InvalidSourceEvent(
                EventValidationError::InvalidOnset
            ))
        );
    }
}
