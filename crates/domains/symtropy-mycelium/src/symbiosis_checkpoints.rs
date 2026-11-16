// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-gated model of overlapping plant–fungus symbiosis checkpoints.
//!
//! The checkpoint framework is not a linear lifecycle automaton. Invitation,
//! admission, accommodation, investment, maintenance and termination/renewal
//! can overlap across interfaces, root cells and developmental times. This
//! module records source-linked observations without forcing a single global
//! stage or inferring intent, punishment, partner choice or causality.
//!
//! A separate performance-evaluation evidence chain checks whether measured
//! resource contribution precedes a host response and a later interface fate.
//! Even a complete, time-consistent chain remains observational evidence, not
//! proof of selective sanction or causal partner evaluation.

use std::collections::BTreeSet;
use std::fmt;

use crate::observation_evidence::{
    EvidenceReceiptError, ObservationEvidenceId, ObservationEvidenceReceipt, ObservationSubject,
};
use crate::temporal_alignment::{
    AlignedExperimentTimestamp, ExperimentClockId, ExperimentTimeAligner, TemporalAlignmentError,
    TemporalOrdering,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbiosisInterfaceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlantUnitId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FungalIndividualId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckpointEventId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbiosisCheckpoint {
    Invitation,
    Admission,
    Accommodation,
    Investment,
    Maintenance,
    TerminationAndRenewal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegulatoryActor {
    Plant,
    Fungus,
    Joint,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointEvidenceKind {
    PlantRootExudateSignal,
    SymbioticSignalResponse,
    ArbusculeInterfaceMorphology,
    HostCarbonInvestment,
    FungalNutrientDelivery,
    ArbusculeTurnover,
    FungalRegulatorySignal,
    IsotopeTracerTransfer,
    HostNutrientResponse,
    MatchedControlCondition,
}

impl CheckpointEvidenceKind {
    pub const fn required_subject(self) -> ObservationSubject {
        match self {
            Self::PlantRootExudateSignal => ObservationSubject::PlantRootExudateSignal,
            Self::SymbioticSignalResponse => ObservationSubject::SymbioticSignalResponse,
            Self::ArbusculeInterfaceMorphology => ObservationSubject::ArbusculeInterfaceMorphology,
            Self::HostCarbonInvestment => ObservationSubject::HostCarbonInvestment,
            Self::FungalNutrientDelivery => ObservationSubject::FungalNutrientDelivery,
            Self::ArbusculeTurnover => ObservationSubject::ArbusculeTurnover,
            Self::FungalRegulatorySignal => ObservationSubject::FungalRegulatorySignal,
            Self::IsotopeTracerTransfer => ObservationSubject::IsotopeTracerTransfer,
            Self::HostNutrientResponse => ObservationSubject::HostNutrientResponse,
            Self::MatchedControlCondition => ObservationSubject::MatchedControlCondition,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointOutcome {
    Observed,
    NotObserved,
    Ambiguous,
    Unreported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbiosisInterface {
    pub id: SymbiosisInterfaceId,
    pub plant: PlantUnitId,
    pub fungus: FungalIndividualId,
    /// Receipt defining the source-linked cross-modal interface registration.
    pub registration_observation: ObservationEvidenceId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointObservation {
    pub id: CheckpointEventId,
    pub interface: SymbiosisInterfaceId,
    pub checkpoint: SymbiosisCheckpoint,
    pub actor: RegulatoryActor,
    pub evidence_kind: CheckpointEvidenceKind,
    pub outcome: CheckpointOutcome,
    pub observation: Option<ObservationEvidenceId>,
    pub time: AlignedExperimentTimestamp,
}

impl CheckpointObservation {
    pub fn validate_shape(&self) -> Result<(), SymbiosisCheckpointError> {
        let allowed = match self.checkpoint {
            SymbiosisCheckpoint::Invitation => matches!(
                self.evidence_kind,
                CheckpointEvidenceKind::PlantRootExudateSignal
                    | CheckpointEvidenceKind::FungalRegulatorySignal
            ),
            SymbiosisCheckpoint::Admission => matches!(
                self.evidence_kind,
                CheckpointEvidenceKind::SymbioticSignalResponse
                    | CheckpointEvidenceKind::FungalRegulatorySignal
            ),
            SymbiosisCheckpoint::Accommodation => matches!(
                self.evidence_kind,
                CheckpointEvidenceKind::ArbusculeInterfaceMorphology
                    | CheckpointEvidenceKind::FungalRegulatorySignal
            ),
            SymbiosisCheckpoint::Investment => matches!(
                self.evidence_kind,
                CheckpointEvidenceKind::HostCarbonInvestment
                    | CheckpointEvidenceKind::IsotopeTracerTransfer
                    | CheckpointEvidenceKind::FungalRegulatorySignal
            ),
            SymbiosisCheckpoint::Maintenance => matches!(
                self.evidence_kind,
                CheckpointEvidenceKind::FungalNutrientDelivery
                    | CheckpointEvidenceKind::HostNutrientResponse
                    | CheckpointEvidenceKind::FungalRegulatorySignal
            ),
            SymbiosisCheckpoint::TerminationAndRenewal => matches!(
                self.evidence_kind,
                CheckpointEvidenceKind::ArbusculeTurnover
                    | CheckpointEvidenceKind::FungalRegulatorySignal
            ),
        };
        if !allowed {
            return Err(
                SymbiosisCheckpointError::EvidenceKindNotValidForCheckpoint {
                    checkpoint: self.checkpoint,
                    evidence_kind: self.evidence_kind,
                },
            );
        }

        if self.outcome == CheckpointOutcome::Unreported && self.observation.is_some() {
            return Err(SymbiosisCheckpointError::UnexpectedObservationReceipt(
                self.id,
            ));
        }
        if self.outcome != CheckpointOutcome::Unreported && self.observation.is_none() {
            return Err(SymbiosisCheckpointError::MissingObservationReceiptForEvent(
                self.id,
            ));
        }
        if !self.time.reference_seconds.is_finite()
            || !self.time.error_bound_s.is_finite()
            || self.time.error_bound_s < 0.0
        {
            return Err(SymbiosisCheckpointError::InvalidAlignedTimestamp(self.id));
        }
        Ok(())
    }
}

/// Observations for one registered host–fungus interface.
///
/// No total order between checkpoint types is imposed. A timeline may contain
/// concurrent/overlapping observations, repeated maintenance observations or
/// termination of one interface while another is being invited.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbiosisCheckpointTimeline {
    pub interface: SymbiosisInterface,
    pub reference_clock: ExperimentClockId,
    pub events: Vec<CheckpointObservation>,
}

impl SymbiosisCheckpointTimeline {
    pub fn validate_against_receipts(
        &self,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), SymbiosisCheckpointError> {
        let mut receipt_ids = BTreeSet::new();
        for receipt in receipts {
            if !receipt_ids.insert(receipt.id) {
                return Err(SymbiosisCheckpointError::DuplicateReceipt(receipt.id));
            }
        }

        let registration = find_receipt(receipts, self.interface.registration_observation)?;
        validate_receipt_subject(
            registration,
            ObservationSubject::SymbiosisInterfaceRegistration,
        )?;

        let mut event_ids = BTreeSet::new();
        let mut observation_ids = BTreeSet::new();
        if !observation_ids.insert(self.interface.registration_observation) {
            return Err(SymbiosisCheckpointError::DuplicateReceipt(
                self.interface.registration_observation,
            ));
        }

        for event in &self.events {
            if event.interface != self.interface.id {
                return Err(SymbiosisCheckpointError::InterfaceMismatch {
                    expected: self.interface.id,
                    actual: event.interface,
                });
            }
            if !event_ids.insert(event.id) {
                return Err(SymbiosisCheckpointError::DuplicateEvent(event.id));
            }
            event.validate_shape()?;
            if event.time.reference_clock != self.reference_clock {
                return Err(SymbiosisCheckpointError::ReferenceClockMismatch(event.id));
            }
            if let Some(observation_id) = event.observation {
                if !observation_ids.insert(observation_id) {
                    return Err(SymbiosisCheckpointError::DuplicateReceipt(observation_id));
                }
                let receipt = find_receipt(receipts, observation_id)?;
                validate_receipt_subject(receipt, event.evidence_kind.required_subject())?;
            }
        }
        Ok(())
    }

    /// Stable chronological view. Tied center times use event identity as a
    /// deterministic tie-breaker; this does not establish temporal precedence.
    pub fn chronological_events(&self) -> Vec<&CheckpointObservation> {
        let mut events: Vec<_> = self.events.iter().collect();
        events.sort_by(|left, right| {
            left.time
                .reference_seconds
                .total_cmp(&right.time.reference_seconds)
                .then_with(|| left.id.cmp(&right.id))
        });
        events
    }

    pub fn has_checkpoint(&self, checkpoint: SymbiosisCheckpoint) -> bool {
        self.events.iter().any(|event| {
            event.checkpoint == checkpoint && event.outcome != CheckpointOutcome::Unreported
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerformanceEvidenceLink {
    pub interface: SymbiosisInterfaceId,
    pub observation: ObservationEvidenceId,
    pub time: AlignedExperimentTimestamp,
}

/// Minimal evidence chain for testing a performance-sensitive host response.
///
/// All four links must refer to the same registered interface: isotope/tracer
/// resource contribution, host nutrient response, subsequent interface fate,
/// and a matched control condition. A sequence-consistent result is not a
/// causal conclusion and does not establish sanction or partner selection.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceEvaluationEvidence {
    pub interface: SymbiosisInterfaceId,
    pub registration_observation: ObservationEvidenceId,
    pub resource_contribution: PerformanceEvidenceLink,
    pub host_response: PerformanceEvidenceLink,
    pub interface_fate: PerformanceEvidenceLink,
    pub matched_control: PerformanceEvidenceLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceEvaluationStatus {
    Incomplete,
    TemporalOrderIndeterminate,
    SequenceConsistentButCausalityNotEstablished,
    SequenceContradictsProposedDirection,
}

impl PerformanceEvaluationEvidence {
    pub fn validate_and_assess(
        &self,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<PerformanceEvaluationStatus, SymbiosisCheckpointError> {
        let mut seen = BTreeSet::new();
        for receipt in receipts {
            if !seen.insert(receipt.id) {
                return Err(SymbiosisCheckpointError::DuplicateReceipt(receipt.id));
            }
        }

        let registration = find_receipt(receipts, self.registration_observation)?;
        validate_receipt_subject(
            registration,
            ObservationSubject::SymbiosisInterfaceRegistration,
        )?;

        let links = [
            (
                self.resource_contribution,
                ObservationSubject::IsotopeTracerTransfer,
            ),
            (self.host_response, ObservationSubject::HostNutrientResponse),
            (self.interface_fate, ObservationSubject::ArbusculeTurnover),
            (
                self.matched_control,
                ObservationSubject::MatchedControlCondition,
            ),
        ];

        let mut link_ids = BTreeSet::from([self.registration_observation]);
        for (link, required_subject) in links {
            if link.interface != self.interface {
                return Err(SymbiosisCheckpointError::PerformanceInterfaceMismatch {
                    expected: self.interface,
                    actual: link.interface,
                });
            }
            if !link_ids.insert(link.observation) {
                return Err(SymbiosisCheckpointError::DuplicateReceipt(link.observation));
            }
            if link.time.reference_clock != self.resource_contribution.time.reference_clock {
                return Err(SymbiosisCheckpointError::PerformanceReferenceClockMismatch);
            }
            let receipt = find_receipt(receipts, link.observation)?;
            validate_receipt_subject(receipt, required_subject)?;
        }

        let contribution_before_response = ExperimentTimeAligner::compare(
            self.resource_contribution.time,
            self.host_response.time,
        )?;
        let response_before_fate =
            ExperimentTimeAligner::compare(self.host_response.time, self.interface_fate.time)?;

        Ok(match (contribution_before_response, response_before_fate) {
            (TemporalOrdering::FirstBeforeSecond, TemporalOrdering::FirstBeforeSecond) => {
                PerformanceEvaluationStatus::SequenceConsistentButCausalityNotEstablished
            }
            (TemporalOrdering::IndeterminateOverlap, _)
            | (_, TemporalOrdering::IndeterminateOverlap) => {
                PerformanceEvaluationStatus::TemporalOrderIndeterminate
            }
            _ => PerformanceEvaluationStatus::SequenceContradictsProposedDirection,
        })
    }
}

fn find_receipt(
    receipts: &[ObservationEvidenceReceipt],
    id: ObservationEvidenceId,
) -> Result<&ObservationEvidenceReceipt, SymbiosisCheckpointError> {
    receipts
        .iter()
        .find(|receipt| receipt.id == id)
        .ok_or(SymbiosisCheckpointError::MissingObservationReceipt(id))
}

fn validate_receipt_subject(
    receipt: &ObservationEvidenceReceipt,
    required: ObservationSubject,
) -> Result<(), SymbiosisCheckpointError> {
    receipt.validate().map_err(
        |source| SymbiosisCheckpointError::InvalidObservationReceipt {
            id: receipt.id,
            source,
        },
    )?;
    if receipt.subject != required {
        return Err(SymbiosisCheckpointError::WrongObservationSubject {
            id: receipt.id,
            expected: required,
            actual: receipt.subject,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbiosisCheckpointError {
    DuplicateReceipt(ObservationEvidenceId),
    DuplicateEvent(CheckpointEventId),
    MissingObservationReceipt(ObservationEvidenceId),
    InvalidObservationReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    WrongObservationSubject {
        id: ObservationEvidenceId,
        expected: ObservationSubject,
        actual: ObservationSubject,
    },
    EvidenceKindNotValidForCheckpoint {
        checkpoint: SymbiosisCheckpoint,
        evidence_kind: CheckpointEvidenceKind,
    },
    MissingObservationReceiptForEvent(CheckpointEventId),
    UnexpectedObservationReceipt(CheckpointEventId),
    InvalidAlignedTimestamp(CheckpointEventId),
    InterfaceMismatch {
        expected: SymbiosisInterfaceId,
        actual: SymbiosisInterfaceId,
    },
    ReferenceClockMismatch(CheckpointEventId),
    PerformanceInterfaceMismatch {
        expected: SymbiosisInterfaceId,
        actual: SymbiosisInterfaceId,
    },
    PerformanceReferenceClockMismatch,
    TemporalAlignment(TemporalAlignmentError),
}

impl From<TemporalAlignmentError> for SymbiosisCheckpointError {
    fn from(value: TemporalAlignmentError) -> Self {
        Self::TemporalAlignment(value)
    }
}

impl fmt::Display for SymbiosisCheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateReceipt(id) => write!(f, "duplicate observation receipt {:?}", id),
            Self::DuplicateEvent(id) => write!(f, "duplicate checkpoint event {:?}", id),
            Self::MissingObservationReceipt(id) => {
                write!(f, "missing checkpoint observation receipt {:?}", id)
            }
            Self::InvalidObservationReceipt { id, source } => {
                write!(f, "invalid checkpoint receipt {:?}: {}", id, source)
            }
            Self::WrongObservationSubject {
                id,
                expected,
                actual,
            } => write!(
                f,
                "receipt {:?} has subject {:?}; expected {:?}",
                id, actual, expected
            ),
            Self::EvidenceKindNotValidForCheckpoint {
                checkpoint,
                evidence_kind,
            } => write!(
                f,
                "evidence kind {:?} is not valid for checkpoint {:?}",
                evidence_kind, checkpoint
            ),
            Self::MissingObservationReceiptForEvent(id) => {
                write!(
                    f,
                    "checkpoint event {:?} requires an observation receipt",
                    id
                )
            }
            Self::UnexpectedObservationReceipt(id) => {
                write!(
                    f,
                    "unreported checkpoint event {:?} must not carry a receipt",
                    id
                )
            }
            Self::InvalidAlignedTimestamp(id) => {
                write!(f, "checkpoint event {:?} has invalid aligned time", id)
            }
            Self::InterfaceMismatch { expected, actual } => write!(
                f,
                "checkpoint event interface {:?} differs from timeline interface {:?}",
                actual, expected
            ),
            Self::ReferenceClockMismatch(id) => {
                write!(f, "checkpoint event {:?} uses another reference clock", id)
            }
            Self::PerformanceInterfaceMismatch { expected, actual } => write!(
                f,
                "performance evidence interface {:?} differs from claimed interface {:?}",
                actual, expected
            ),
            Self::PerformanceReferenceClockMismatch => {
                write!(
                    f,
                    "performance evidence links use different reference clocks"
                )
            }
            Self::TemporalAlignment(source) => write!(f, "temporal alignment failed: {}", source),
        }
    }
}

impl std::error::Error for SymbiosisCheckpointError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation_evidence::{
        EvidenceDisposition, ImageFrameIndex, ImageFrameReference, ImageRoiBoundsPx,
        ObservationRoiId, PhysicalSamplingCalibration, Sha256Digest,
    };
    use crate::temporal_alignment::{ClockCalibrationId, ExperimentClockId};

    fn receipt(id: u64, subject: ObservationSubject) -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(id),
            source_file_sha256: Sha256Digest([1; 32]),
            frame: ImageFrameReference {
                frame: ImageFrameIndex(id as u32),
                slice: None,
                timestamp_s: id as f64,
            },
            roi: ImageRoiBoundsPx {
                id: ObservationRoiId(id),
                x_min: 0,
                y_min: 0,
                width_px: 10,
                height_px: 10,
            },
            calibration: PhysicalSamplingCalibration {
                pixel_width_um: 1.0,
                pixel_height_um: 1.0,
                voxel_depth_um: Some(1.0),
                frame_interval_s: Some(1.0),
            },
            processor_name: "synthetic_checkpoint_fixture".to_owned(),
            processor_revision: "v0.1".to_owned(),
            processor_parameters_sha256: Sha256Digest([2; 32]),
            subject,
            graph_target: None,
            graph_event: None,
            uncertainty: None,
            disposition: EvidenceDisposition::MachineGenerated,
        }
    }

    fn time(seconds: f64, error: f64) -> AlignedExperimentTimestamp {
        AlignedExperimentTimestamp {
            source_clock: ExperimentClockId(1),
            reference_clock: ExperimentClockId(1),
            reference_seconds: seconds,
            error_bound_s: error,
            calibration: Some(ClockCalibrationId(1)),
            calibration_evidence: Some(ObservationEvidenceId(99)),
        }
    }

    fn interface() -> SymbiosisInterface {
        SymbiosisInterface {
            id: SymbiosisInterfaceId(7),
            plant: PlantUnitId(11),
            fungus: FungalIndividualId(21),
            registration_observation: ObservationEvidenceId(1),
        }
    }

    #[test]
    fn overlapping_checkpoint_events_validate_without_linear_stage_order() {
        let interface = interface();
        let timeline = SymbiosisCheckpointTimeline {
            interface: interface.clone(),
            reference_clock: ExperimentClockId(1),
            events: vec![
                CheckpointObservation {
                    id: CheckpointEventId(2),
                    interface: interface.id,
                    checkpoint: SymbiosisCheckpoint::Maintenance,
                    actor: RegulatoryActor::Joint,
                    evidence_kind: CheckpointEvidenceKind::FungalNutrientDelivery,
                    outcome: CheckpointOutcome::Observed,
                    observation: Some(ObservationEvidenceId(3)),
                    time: time(10.0, 1.0),
                },
                CheckpointObservation {
                    id: CheckpointEventId(1),
                    interface: interface.id,
                    checkpoint: SymbiosisCheckpoint::Investment,
                    actor: RegulatoryActor::Plant,
                    evidence_kind: CheckpointEvidenceKind::HostCarbonInvestment,
                    outcome: CheckpointOutcome::Observed,
                    observation: Some(ObservationEvidenceId(2)),
                    time: time(10.5, 1.0),
                },
            ],
        };
        let receipts = vec![
            receipt(1, ObservationSubject::SymbiosisInterfaceRegistration),
            receipt(2, ObservationSubject::HostCarbonInvestment),
            receipt(3, ObservationSubject::FungalNutrientDelivery),
        ];
        assert_eq!(timeline.validate_against_receipts(&receipts), Ok(()));
        // Center-time ordering is a deterministic display order only. These
        // uncertainty intervals overlap, so the temporal comparator must not
        // claim that either checkpoint preceded the other.
        assert_eq!(timeline.chronological_events()[0].id, CheckpointEventId(2));
        assert_eq!(
            ExperimentTimeAligner::compare(
                timeline.chronological_events()[0].time,
                timeline.chronological_events()[1].time,
            ),
            Ok(TemporalOrdering::IndeterminateOverlap)
        );
        assert!(timeline.has_checkpoint(SymbiosisCheckpoint::Maintenance));
    }

    #[test]
    fn event_from_another_interface_is_rejected() {
        let interface = interface();
        let timeline = SymbiosisCheckpointTimeline {
            interface: interface.clone(),
            reference_clock: ExperimentClockId(1),
            events: vec![CheckpointObservation {
                id: CheckpointEventId(1),
                interface: SymbiosisInterfaceId(999),
                checkpoint: SymbiosisCheckpoint::Invitation,
                actor: RegulatoryActor::Plant,
                evidence_kind: CheckpointEvidenceKind::PlantRootExudateSignal,
                outcome: CheckpointOutcome::Observed,
                observation: Some(ObservationEvidenceId(2)),
                time: time(1.0, 0.0),
            }],
        };
        let receipts = vec![
            receipt(1, ObservationSubject::SymbiosisInterfaceRegistration),
            receipt(2, ObservationSubject::PlantRootExudateSignal),
        ];
        assert!(matches!(
            timeline.validate_against_receipts(&receipts),
            Err(SymbiosisCheckpointError::InterfaceMismatch { .. })
        ));
    }

    #[test]
    fn stage_and_receipt_subject_must_match() {
        let interface = interface();
        let timeline = SymbiosisCheckpointTimeline {
            interface: interface.clone(),
            reference_clock: ExperimentClockId(1),
            events: vec![CheckpointObservation {
                id: CheckpointEventId(1),
                interface: interface.id,
                checkpoint: SymbiosisCheckpoint::Invitation,
                actor: RegulatoryActor::Plant,
                evidence_kind: CheckpointEvidenceKind::PlantRootExudateSignal,
                outcome: CheckpointOutcome::Observed,
                observation: Some(ObservationEvidenceId(2)),
                time: time(1.0, 0.0),
            }],
        };
        let receipts = vec![
            receipt(1, ObservationSubject::SymbiosisInterfaceRegistration),
            receipt(2, ObservationSubject::HostCarbonInvestment),
        ];
        assert!(matches!(
            timeline.validate_against_receipts(&receipts),
            Err(SymbiosisCheckpointError::WrongObservationSubject { .. })
        ));
    }

    #[test]
    fn unreported_checkpoint_has_no_receipt() {
        let interface = interface();
        let timeline = SymbiosisCheckpointTimeline {
            interface: interface.clone(),
            reference_clock: ExperimentClockId(1),
            events: vec![CheckpointObservation {
                id: CheckpointEventId(1),
                interface: interface.id,
                checkpoint: SymbiosisCheckpoint::Admission,
                actor: RegulatoryActor::Unresolved,
                evidence_kind: CheckpointEvidenceKind::SymbioticSignalResponse,
                outcome: CheckpointOutcome::Unreported,
                observation: Some(ObservationEvidenceId(2)),
                time: time(1.0, 0.0),
            }],
        };
        let receipts = vec![
            receipt(1, ObservationSubject::SymbiosisInterfaceRegistration),
            receipt(2, ObservationSubject::SymbioticSignalResponse),
        ];
        assert_eq!(
            timeline.validate_against_receipts(&receipts),
            Err(SymbiosisCheckpointError::UnexpectedObservationReceipt(
                CheckpointEventId(1)
            ))
        );
    }

    #[test]
    fn overlapping_checkpoints_are_not_reported_as_causal_order() {
        let first = time(10.0, 1.0);
        let second = time(10.5, 1.0);
        assert_eq!(
            ExperimentTimeAligner::compare(first, second).unwrap(),
            TemporalOrdering::IndeterminateOverlap
        );
    }

    fn performance_evidence(
        contribution_time: AlignedExperimentTimestamp,
        response_time: AlignedExperimentTimestamp,
        fate_time: AlignedExperimentTimestamp,
    ) -> PerformanceEvaluationEvidence {
        PerformanceEvaluationEvidence {
            interface: SymbiosisInterfaceId(7),
            registration_observation: ObservationEvidenceId(1),
            resource_contribution: PerformanceEvidenceLink {
                interface: SymbiosisInterfaceId(7),
                observation: ObservationEvidenceId(2),
                time: contribution_time,
            },
            host_response: PerformanceEvidenceLink {
                interface: SymbiosisInterfaceId(7),
                observation: ObservationEvidenceId(3),
                time: response_time,
            },
            interface_fate: PerformanceEvidenceLink {
                interface: SymbiosisInterfaceId(7),
                observation: ObservationEvidenceId(4),
                time: fate_time,
            },
            matched_control: PerformanceEvidenceLink {
                interface: SymbiosisInterfaceId(7),
                observation: ObservationEvidenceId(5),
                time: response_time,
            },
        }
    }

    fn performance_receipts() -> Vec<ObservationEvidenceReceipt> {
        vec![
            receipt(1, ObservationSubject::SymbiosisInterfaceRegistration),
            receipt(2, ObservationSubject::IsotopeTracerTransfer),
            receipt(3, ObservationSubject::HostNutrientResponse),
            receipt(4, ObservationSubject::ArbusculeTurnover),
            receipt(5, ObservationSubject::MatchedControlCondition),
        ]
    }

    #[test]
    fn complete_time_consistent_performance_chain_does_not_claim_causality() {
        let item = performance_evidence(time(1.0, 0.1), time(3.0, 0.1), time(5.0, 0.1));
        assert_eq!(
            item.validate_and_assess(&performance_receipts()),
            Ok(PerformanceEvaluationStatus::SequenceConsistentButCausalityNotEstablished)
        );
    }

    #[test]
    fn performance_chain_with_overlapping_intervals_is_indeterminate() {
        let item = performance_evidence(time(1.0, 0.5), time(1.5, 0.5), time(5.0, 0.1));
        assert_eq!(
            item.validate_and_assess(&performance_receipts()),
            Ok(PerformanceEvaluationStatus::TemporalOrderIndeterminate)
        );
    }

    #[test]
    fn performance_chain_with_reversed_order_is_not_supported() {
        let item = performance_evidence(time(4.0, 0.1), time(2.0, 0.1), time(5.0, 0.1));
        assert_eq!(
            item.validate_and_assess(&performance_receipts()),
            Ok(PerformanceEvaluationStatus::SequenceContradictsProposedDirection)
        );
    }

    #[test]
    fn performance_evidence_must_refer_to_the_same_interface() {
        let mut item = performance_evidence(time(1.0, 0.1), time(3.0, 0.1), time(5.0, 0.1));
        item.host_response.interface = SymbiosisInterfaceId(999);
        assert!(matches!(
            item.validate_and_assess(&performance_receipts()),
            Err(SymbiosisCheckpointError::PerformanceInterfaceMismatch { .. })
        ));
    }

    #[test]
    fn performance_chain_requires_all_source_linked_roles() {
        let item = performance_evidence(time(1.0, 0.1), time(3.0, 0.1), time(5.0, 0.1));
        let mut receipts = performance_receipts();
        receipts.pop();
        assert_eq!(
            item.validate_and_assess(&receipts),
            Err(SymbiosisCheckpointError::MissingObservationReceipt(
                ObservationEvidenceId(5)
            ))
        );
    }
}
