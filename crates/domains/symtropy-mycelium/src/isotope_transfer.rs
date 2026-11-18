// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-gated stable-isotope transfer assessment for plant–fungus experiments.
//!
//! This module composes existing source receipts, experiment-clock alignment
//! and fungal-network evidence. It does not create a second provenance,
//! temporal-alignment or network-classification system.
//!
//! The strongest result is deliberately named
//! `RouteConsistentEvidenceCandidate`, never "causal transfer proved".
//! It applies conservative absolute error bounds to matched intervention
//! contrasts; it does not perform statistical inference or replace a
//! preregistered causal analysis.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::network_evidence::{
    CytoplasmicFlowEvidence, EvidenceOutcome, FungalNetworkIdentity, NetworkClaimLevel,
    NetworkCriterion, NetworkEvidenceAssessment, NetworkEvidenceError,
};
use crate::observation_evidence::{
    EvidenceReceiptError, ObservationEvidenceId, ObservationEvidenceReceipt, ObservationSubject,
};
use crate::temporal_alignment::{
    AlignedExperimentTimestamp, ExperimentTimeAligner, RawExperimentTimestamp,
    TemporalAlignmentError, TemporalOrdering,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TracerExperimentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TracerBlockId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TracerReplicateId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StableIsotope {
    Carbon13,
    Nitrogen15,
    Deuterium2,
    Oxygen18,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TracerIntervention {
    IntactHyphalConnection,
    HyphaeSevered,
    DiffusionOnlyBarrier,
    NoFungus,
    MicrobialOnly,
    NoDonorLabel,
}

impl TracerIntervention {
    pub const REQUIRED_CONTROLS: [Self; 5] = [
        Self::HyphaeSevered,
        Self::DiffusionOnlyBarrier,
        Self::NoFungus,
        Self::MicrobialOnly,
        Self::NoDonorLabel,
    ];

    pub const ALL: [Self; 6] = [
        Self::IntactHyphalConnection,
        Self::HyphaeSevered,
        Self::DiffusionOnlyBarrier,
        Self::NoFungus,
        Self::MicrobialOnly,
        Self::NoDonorLabel,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TracerCompartment {
    DonorPlant,
    FungalBridge,
    ReceiverPlant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TracerPhase {
    PreLabelBaseline,
    PostLabel,
}

/// Absolute atom-fraction uncertainty bound, not a standard deviation unless
/// the source explicitly defines it that way. The inference uses the bound
/// conservatively and never assumes independent Gaussian errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsotopeTracerObservation {
    pub evidence: ObservationEvidenceId,
    pub experiment: TracerExperimentId,
    pub block: TracerBlockId,
    pub replicate: TracerReplicateId,
    pub intervention: TracerIntervention,
    pub compartment: TracerCompartment,
    pub phase: TracerPhase,
    pub isotope: StableIsotope,
    /// Isotope atom fraction in [0, 1], not atom-percent.
    pub atom_fraction: f64,
    /// Conservative absolute error bound in atom-fraction units.
    pub atom_fraction_error_bound: f64,
    /// Optional total elemental pool in the sampled compartment, mg.
    pub total_element_mass_mg: Option<f64>,
    /// Required whenever total_element_mass_mg is present.
    pub total_element_mass_error_bound_mg: Option<f64>,
    /// Raw timestamp remains in the instrument's own clock domain.
    pub raw_timestamp: RawExperimentTimestamp,
}

impl IsotopeTracerObservation {
    pub fn validate(&self) -> Result<(), IsotopeTransferError> {
        if !self.atom_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.atom_fraction)
            || !self.atom_fraction_error_bound.is_finite()
            || self.atom_fraction_error_bound < 0.0
            || self.atom_fraction_error_bound > 1.0
        {
            return Err(IsotopeTransferError::InvalidAtomFraction(self.evidence));
        }

        match (
            self.total_element_mass_mg,
            self.total_element_mass_error_bound_mg,
        ) {
            (None, None) => {}
            (Some(mass), Some(error)) => {
                if !mass.is_finite() || mass <= 0.0 || !error.is_finite() || error < 0.0 {
                    return Err(IsotopeTransferError::InvalidElementPool(self.evidence));
                }
            }
            _ => return Err(IsotopeTransferError::IncompleteElementPool(self.evidence)),
        }

        self.raw_timestamp
            .validate()
            .map_err(IsotopeTransferError::TemporalAlignment)?;
        match (self.compartment, self.phase) {
            (
                TracerCompartment::DonorPlant | TracerCompartment::ReceiverPlant,
                TracerPhase::PreLabelBaseline,
            ) => {}
            (_, TracerPhase::PostLabel) => {}
            (_, TracerPhase::PreLabelBaseline) => {
                return Err(IsotopeTransferError::InvalidSampleRole(self.evidence));
            }
        }
        if self.compartment == TracerCompartment::FungalBridge
            && self.intervention != TracerIntervention::IntactHyphalConnection
        {
            return Err(IsotopeTransferError::UnexpectedFungalBridgeSample(
                self.evidence,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TracerArmEvidence {
    pub intervention: TracerIntervention,
    pub evidence: ObservationEvidenceId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExcessIsotopeInventoryEstimate {
    /// Excess isotope inventory in the receiver sample, mg. This is not a
    /// transferred amount: it is a receiver-pool inventory estimate.
    pub estimate_mg: f64,
    /// Conservative bound from atom-fraction and element-pool uncertainties.
    pub error_bound_mg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TracerControlContrast {
    pub intervention: TracerIntervention,
    pub control_receiver_excess_upper_bound: f64,
    /// Intact receiver lower bound minus this control's upper bound.
    pub intact_minus_control_lower_bound: f64,
    pub baseline_donor_receiver_order_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TracerBlockReport {
    pub block: TracerBlockId,
    pub intact_donor_excess_lower_bound: f64,
    pub intact_bridge_excess_lower_bound: f64,
    pub intact_receiver_excess_lower_bound: f64,
    pub intact_receiver_excess_upper_bound: f64,
    pub receiver_excess_inventory: Option<ExcessIsotopeInventoryEstimate>,
    pub donor_bridge_receiver_order_confirmed: bool,
    pub control_contrasts: Vec<TracerControlContrast>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsotopeTransferStatus {
    NetworkNotQualified,
    InsufficientMatchedBlocks,
    DonorLabelNotResolved,
    FungalBridgeEnrichmentNotResolved,
    ReceiverEnrichmentNotResolved,
    TemporalOrderIndeterminate,
    RouteNotDistinguishedFromControls,
    RouteConsistentEvidenceCandidate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsotopeTransferReport {
    pub experiment: TracerExperimentId,
    pub isotope: StableIsotope,
    pub network_claim_level: NetworkClaimLevel,
    pub network_route_evidence_supported: bool,
    pub matched_block_count: usize,
    pub minimum_matched_blocks: usize,
    pub blocks: Vec<TracerBlockReport>,
    pub status: IsotopeTransferStatus,
    /// Always false: this deterministic layer does not prove causality.
    pub causal_claim_permitted: bool,
    /// Always false: this module does not calculate p-values or confidence intervals.
    pub statistical_inference_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsotopeTransferError {
    EmptyObservations,
    InvalidMinimumMatchedBlocks,
    MissingRequiredControlArm(TracerIntervention),
    DuplicateRequiredControlArm(TracerIntervention),
    MissingArmEvidence(TracerIntervention),
    DuplicateArmEvidence(TracerIntervention),
    DuplicateObservation(ObservationEvidenceId),
    DuplicateReceipt(ObservationEvidenceId),
    MissingReceipt(ObservationEvidenceId),
    InvalidReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    WrongSampleReceiptSubject(ObservationEvidenceId),
    WrongArmReceiptSubject(ObservationEvidenceId),
    ReceiptTimestampMismatch(ObservationEvidenceId),
    InvalidAtomFraction(ObservationEvidenceId),
    InvalidElementPool(ObservationEvidenceId),
    IncompleteElementPool(ObservationEvidenceId),
    InvalidSampleRole(ObservationEvidenceId),
    UnexpectedFungalBridgeSample(ObservationEvidenceId),
    MixedExperiment(ObservationEvidenceId),
    MixedIsotope(ObservationEvidenceId),
    MissingBlock(TracerBlockId),
    MissingArm {
        block: TracerBlockId,
        intervention: TracerIntervention,
    },
    DuplicateSampleSlot(ObservationEvidenceId),
    InconsistentReplicate {
        block: TracerBlockId,
        intervention: TracerIntervention,
    },
    MissingSample {
        block: TracerBlockId,
        intervention: TracerIntervention,
        compartment: TracerCompartment,
        phase: TracerPhase,
    },
    UnexpectedSampleRole {
        block: TracerBlockId,
        intervention: TracerIntervention,
    },
    TemporalAlignment(TemporalAlignmentError),
    NetworkEvidence(NetworkEvidenceError),
}

impl fmt::Display for IsotopeTransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyObservations => write!(f, "at least one isotope observation is required"),
            Self::InvalidMinimumMatchedBlocks => {
                write!(f, "minimum matched blocks must be at least three")
            }
            Self::MissingRequiredControlArm(arm) => {
                write!(f, "required control arm {:?} is absent", arm)
            }
            Self::DuplicateRequiredControlArm(arm) => {
                write!(f, "required control arm {:?} is duplicated", arm)
            }
            Self::MissingArmEvidence(arm) => {
                write!(
                    f,
                    "intervention {:?} has no source-linked design receipt",
                    arm
                )
            }
            Self::DuplicateArmEvidence(arm) => {
                write!(f, "intervention {:?} has duplicate design receipts", arm)
            }
            Self::DuplicateObservation(id) => {
                write!(f, "isotope observation {:?} is duplicated", id)
            }
            Self::DuplicateReceipt(id) => write!(f, "receipt {:?} is duplicated", id),
            Self::MissingReceipt(id) => write!(f, "receipt {:?} is missing", id),
            Self::InvalidReceipt { id, source } => {
                write!(f, "receipt {:?} is invalid: {}", id, source)
            }
            Self::WrongSampleReceiptSubject(id) => {
                write!(
                    f,
                    "sample receipt {:?} is not an isotope-tracer observation",
                    id
                )
            }
            Self::WrongArmReceiptSubject(id) => {
                write!(
                    f,
                    "intervention receipt {:?} is not a matched-condition record",
                    id
                )
            }
            Self::ReceiptTimestampMismatch(id) => {
                write!(f, "receipt {:?} and raw instrument timestamp disagree", id)
            }
            Self::InvalidAtomFraction(id) => {
                write!(
                    f,
                    "atom fraction or its error bound is invalid for {:?}",
                    id
                )
            }
            Self::InvalidElementPool(id) => {
                write!(
                    f,
                    "element pool mass or error bound is invalid for {:?}",
                    id
                )
            }
            Self::IncompleteElementPool(id) => {
                write!(
                    f,
                    "element pool mass and error bound must be both present or both absent for {:?}",
                    id
                )
            }
            Self::InvalidSampleRole(id) => {
                write!(
                    f,
                    "baseline sample {:?} must be from the receiver plant",
                    id
                )
            }
            Self::UnexpectedFungalBridgeSample(id) => {
                write!(
                    f,
                    "fungal bridge sample {:?} is only valid in the intact arm",
                    id
                )
            }
            Self::MixedExperiment(id) => {
                write!(f, "observation {:?} belongs to another experiment", id)
            }
            Self::MixedIsotope(id) => {
                write!(f, "observation {:?} uses a different isotope", id)
            }
            Self::MissingBlock(id) => write!(f, "matched block {:?} has no observations", id),
            Self::MissingArm {
                block,
                intervention,
            } => {
                write!(f, "block {:?} is missing arm {:?}", block, intervention)
            }
            Self::DuplicateSampleSlot(id) => {
                write!(f, "duplicate compartment/phase sample slot at {:?}", id)
            }
            Self::InconsistentReplicate {
                block,
                intervention,
            } => {
                write!(
                    f,
                    "block {:?} arm {:?} mixes replicate identities",
                    block, intervention
                )
            }
            Self::MissingSample {
                block,
                intervention,
                compartment,
                phase,
            } => write!(
                f,
                "block {:?} arm {:?} lacks {:?} sample in phase {:?}",
                block, intervention, compartment, phase
            ),
            Self::UnexpectedSampleRole {
                block,
                intervention,
            } => {
                write!(
                    f,
                    "block {:?} arm {:?} contains an unsupported sample role",
                    block, intervention
                )
            }
            Self::TemporalAlignment(error) => write!(f, "temporal alignment failed: {}", error),
            Self::NetworkEvidence(error) => write!(f, "network evidence is invalid: {}", error),
        }
    }
}

impl std::error::Error for IsotopeTransferError {}

#[derive(Debug, Clone, PartialEq)]
pub struct IsotopeTransferExperiment {
    pub id: TracerExperimentId,
    pub isotope: StableIsotope,
    pub required_control_arms: Vec<TracerIntervention>,
    pub arm_evidence: Vec<TracerArmEvidence>,
    pub observations: Vec<IsotopeTracerObservation>,
    pub network_evidence: NetworkEvidenceAssessment,
    pub minimum_matched_blocks: usize,
}

#[derive(Debug, Clone, Copy)]
struct ArmSamples<'a> {
    baseline: &'a IsotopeTracerObservation,
    donor_baseline: &'a IsotopeTracerObservation,
    donor: &'a IsotopeTracerObservation,
    bridge: Option<&'a IsotopeTracerObservation>,
    receiver: &'a IsotopeTracerObservation,
    baseline_time: AlignedExperimentTimestamp,
    donor_baseline_time: AlignedExperimentTimestamp,
    donor_time: AlignedExperimentTimestamp,
    bridge_time: Option<AlignedExperimentTimestamp>,
    receiver_time: AlignedExperimentTimestamp,
}

#[derive(Debug, Clone, Copy)]
struct ExcessBounds {
    lower: f64,
    upper: f64,
}

fn excess_bounds(
    post: &IsotopeTracerObservation,
    baseline: &IsotopeTracerObservation,
) -> ExcessBounds {
    ExcessBounds {
        lower: (post.atom_fraction - post.atom_fraction_error_bound)
            - (baseline.atom_fraction + baseline.atom_fraction_error_bound),
        upper: (post.atom_fraction + post.atom_fraction_error_bound)
            - (baseline.atom_fraction - baseline.atom_fraction_error_bound),
    }
}

fn receiver_inventory_estimate(
    post: &IsotopeTracerObservation,
    baseline: &IsotopeTracerObservation,
) -> Option<ExcessIsotopeInventoryEstimate> {
    let mass = post.total_element_mass_mg?;
    let mass_error = post.total_element_mass_error_bound_mg?;
    let atom_excess = post.atom_fraction - baseline.atom_fraction;
    let atom_error = post.atom_fraction_error_bound + baseline.atom_fraction_error_bound;
    let estimate = atom_excess * mass;
    let error_bound = atom_error * (mass + mass_error) + atom_excess.abs() * mass_error;
    (estimate.is_finite() && error_bound.is_finite()).then_some(ExcessIsotopeInventoryEstimate {
        estimate_mg: estimate,
        error_bound_mg: error_bound,
    })
}

impl IsotopeTransferExperiment {
    pub fn assess(
        &self,
        receipts: &[ObservationEvidenceReceipt],
        aligner: &ExperimentTimeAligner,
    ) -> Result<IsotopeTransferReport, IsotopeTransferError> {
        self.validate_design()?;
        self.network_evidence
            .validate_against_receipts(receipts)
            .map_err(IsotopeTransferError::NetworkEvidence)?;

        let mut receipt_by_id = BTreeMap::new();
        for receipt in receipts {
            if receipt_by_id.insert(receipt.id, receipt).is_some() {
                return Err(IsotopeTransferError::DuplicateReceipt(receipt.id));
            }
        }

        for arm in &self.arm_evidence {
            let receipt = receipt_by_id
                .get(&arm.evidence)
                .ok_or(IsotopeTransferError::MissingReceipt(arm.evidence))?;
            receipt
                .validate()
                .map_err(|source| IsotopeTransferError::InvalidReceipt {
                    id: arm.evidence,
                    source,
                })?;
            if receipt.subject != ObservationSubject::MatchedControlCondition {
                return Err(IsotopeTransferError::WrongArmReceiptSubject(arm.evidence));
            }
        }

        let mut seen_observations = BTreeSet::new();
        let mut aligned_by_id = BTreeMap::new();
        let mut grouped = BTreeMap::<
            TracerBlockId,
            BTreeMap<
                TracerIntervention,
                BTreeMap<(TracerCompartment, TracerPhase), &IsotopeTracerObservation>,
            >,
        >::new();

        for observation in &self.observations {
            observation.validate()?;
            if observation.experiment != self.id {
                return Err(IsotopeTransferError::MixedExperiment(observation.evidence));
            }
            if observation.isotope != self.isotope {
                return Err(IsotopeTransferError::MixedIsotope(observation.evidence));
            }
            if !seen_observations.insert(observation.evidence) {
                return Err(IsotopeTransferError::DuplicateObservation(
                    observation.evidence,
                ));
            }

            let receipt = receipt_by_id
                .get(&observation.evidence)
                .ok_or(IsotopeTransferError::MissingReceipt(observation.evidence))?;
            receipt
                .validate()
                .map_err(|source| IsotopeTransferError::InvalidReceipt {
                    id: observation.evidence,
                    source,
                })?;
            if receipt.subject != ObservationSubject::IsotopeTracerTransfer {
                return Err(IsotopeTransferError::WrongSampleReceiptSubject(
                    observation.evidence,
                ));
            }
            if receipt.frame.timestamp_s.to_bits() != observation.raw_timestamp.seconds.to_bits() {
                return Err(IsotopeTransferError::ReceiptTimestampMismatch(
                    observation.evidence,
                ));
            }

            let aligned = aligner
                .align(observation.raw_timestamp)
                .map_err(IsotopeTransferError::TemporalAlignment)?;
            if aligned.reference_clock != aligner.reference_clock() {
                return Err(IsotopeTransferError::TemporalAlignment(
                    TemporalAlignmentError::NonFiniteAlignedTimestamp,
                ));
            }
            aligned_by_id.insert(observation.evidence, aligned);

            let arm = grouped
                .entry(observation.block)
                .or_default()
                .entry(observation.intervention)
                .or_default();
            let slot = (observation.compartment, observation.phase);
            if arm.insert(slot, observation).is_some() {
                return Err(IsotopeTransferError::DuplicateSampleSlot(
                    observation.evidence,
                ));
            }
        }

        if grouped.is_empty() {
            return Err(IsotopeTransferError::EmptyObservations);
        }

        let mut block_reports = Vec::with_capacity(grouped.len());
        for (block_id, arms) in &grouped {
            for intervention in TracerIntervention::ALL {
                if !arms.contains_key(&intervention) {
                    return Err(IsotopeTransferError::MissingArm {
                        block: *block_id,
                        intervention,
                    });
                }
            }

            let intact = Self::arm_samples(
                *block_id,
                TracerIntervention::IntactHyphalConnection,
                arms,
                &aligned_by_id,
                true,
            )?;
            let intact_donor_excess = excess_bounds(intact.donor, intact.donor_baseline);
            let intact_bridge = intact
                .bridge
                .expect("intact arm validation requires a fungal bridge sample");
            let intact_bridge_excess = excess_bounds(intact_bridge, intact.baseline);
            let intact_receiver_excess = excess_bounds(intact.receiver, intact.baseline);
            let inventory = receiver_inventory_estimate(intact.receiver, intact.baseline);

            let mut control_contrasts = Vec::with_capacity(self.required_control_arms.len());
            for control in &self.required_control_arms {
                let samples = Self::arm_samples(*block_id, *control, arms, &aligned_by_id, false)?;
                let control_excess = excess_bounds(samples.receiver, samples.baseline);
                let control_order =
                    Self::baselines_precede_donor(
                        samples.donor_baseline_time,
                        samples.baseline_time,
                        samples.donor_time,
                    )? && Self::order_is_confirmed_pair(samples.donor_time, samples.receiver_time)?;
                control_contrasts.push(TracerControlContrast {
                    intervention: *control,
                    control_receiver_excess_upper_bound: control_excess.upper,
                    intact_minus_control_lower_bound: intact_receiver_excess.lower
                        - control_excess.upper,
                    baseline_donor_receiver_order_confirmed: control_order,
                });
            }

            let baseline_donor_order = Self::baselines_precede_donor(
                intact.donor_baseline_time,
                intact.baseline_time,
                intact.donor_time,
            )?;
            let donor_bridge_receiver_order_confirmed = baseline_donor_order
                && Self::order_is_confirmed(
                    intact.donor_time,
                    intact
                        .bridge_time
                        .expect("intact bridge timestamp is present"),
                    intact.receiver_time,
                )?;

            block_reports.push(TracerBlockReport {
                block: *block_id,
                intact_donor_excess_lower_bound: intact_donor_excess.lower,
                intact_bridge_excess_lower_bound: intact_bridge_excess.lower,
                intact_receiver_excess_lower_bound: intact_receiver_excess.lower,
                intact_receiver_excess_upper_bound: intact_receiver_excess.upper,
                receiver_excess_inventory: inventory,
                donor_bridge_receiver_order_confirmed,
                control_contrasts,
            });
        }

        let network_classification = self
            .network_evidence
            .classify()
            .map_err(IsotopeTransferError::NetworkEvidence)?;
        let network_route_evidence_supported = Self::network_route_supported(
            &self.network_evidence,
            network_classification.claim_level,
        );

        let enough_blocks = block_reports.len() >= self.minimum_matched_blocks;
        let donor_label_resolved = block_reports
            .iter()
            .all(|block| block.intact_donor_excess_lower_bound > 0.0);
        let bridge_enrichment_resolved = block_reports
            .iter()
            .all(|block| block.intact_bridge_excess_lower_bound > 0.0);
        let receiver_enrichment_resolved = block_reports
            .iter()
            .all(|block| block.intact_receiver_excess_lower_bound > 0.0);
        let temporal_order_resolved = block_reports.iter().all(|block| {
            block.donor_bridge_receiver_order_confirmed
                && block
                    .control_contrasts
                    .iter()
                    .all(|control| control.baseline_donor_receiver_order_confirmed)
        });
        let controls_distinguished = block_reports.iter().all(|block| {
            block
                .control_contrasts
                .iter()
                .all(|contrast| contrast.intact_minus_control_lower_bound > 0.0)
        });

        let status = if !network_route_evidence_supported {
            IsotopeTransferStatus::NetworkNotQualified
        } else if !enough_blocks {
            IsotopeTransferStatus::InsufficientMatchedBlocks
        } else if !donor_label_resolved {
            IsotopeTransferStatus::DonorLabelNotResolved
        } else if !bridge_enrichment_resolved {
            IsotopeTransferStatus::FungalBridgeEnrichmentNotResolved
        } else if !receiver_enrichment_resolved {
            IsotopeTransferStatus::ReceiverEnrichmentNotResolved
        } else if !temporal_order_resolved {
            IsotopeTransferStatus::TemporalOrderIndeterminate
        } else if !controls_distinguished {
            IsotopeTransferStatus::RouteNotDistinguishedFromControls
        } else {
            IsotopeTransferStatus::RouteConsistentEvidenceCandidate
        };

        Ok(IsotopeTransferReport {
            experiment: self.id,
            isotope: self.isotope,
            network_claim_level: network_classification.claim_level,
            network_route_evidence_supported,
            matched_block_count: block_reports.len(),
            minimum_matched_blocks: self.minimum_matched_blocks,
            blocks: block_reports,
            status,
            causal_claim_permitted: false,
            statistical_inference_performed: false,
        })
    }

    fn validate_design(&self) -> Result<(), IsotopeTransferError> {
        if self.observations.is_empty() {
            return Err(IsotopeTransferError::EmptyObservations);
        }
        if self.minimum_matched_blocks < 3 {
            return Err(IsotopeTransferError::InvalidMinimumMatchedBlocks);
        }
        let mut controls = BTreeSet::new();
        for control in &self.required_control_arms {
            if *control == TracerIntervention::IntactHyphalConnection {
                return Err(IsotopeTransferError::DuplicateRequiredControlArm(*control));
            }
            if !controls.insert(*control) {
                return Err(IsotopeTransferError::DuplicateRequiredControlArm(*control));
            }
        }
        for required in TracerIntervention::REQUIRED_CONTROLS {
            if !controls.contains(&required) {
                return Err(IsotopeTransferError::MissingRequiredControlArm(required));
            }
        }

        let mut arm_receipts = BTreeMap::new();
        let mut arm_receipt_ids = BTreeSet::new();
        for arm in &self.arm_evidence {
            if !arm_receipt_ids.insert(arm.evidence) {
                return Err(IsotopeTransferError::DuplicateReceipt(arm.evidence));
            }
            if arm_receipts
                .insert(arm.intervention, arm.evidence)
                .is_some()
            {
                return Err(IsotopeTransferError::DuplicateArmEvidence(arm.intervention));
            }
        }
        for intervention in TracerIntervention::ALL {
            if !arm_receipts.contains_key(&intervention) {
                return Err(IsotopeTransferError::MissingArmEvidence(intervention));
            }
        }
        Ok(())
    }

    fn arm_samples<'a>(
        block: TracerBlockId,
        intervention: TracerIntervention,
        arms: &'a BTreeMap<
            TracerIntervention,
            BTreeMap<(TracerCompartment, TracerPhase), &'a IsotopeTracerObservation>,
        >,
        aligned: &BTreeMap<ObservationEvidenceId, AlignedExperimentTimestamp>,
        require_bridge: bool,
    ) -> Result<ArmSamples<'a>, IsotopeTransferError> {
        let samples = arms
            .get(&intervention)
            .ok_or(IsotopeTransferError::MissingArm {
                block,
                intervention,
            })?;

        let get = |compartment, phase| {
            samples
                .get(&(compartment, phase))
                .copied()
                .ok_or(IsotopeTransferError::MissingSample {
                    block,
                    intervention,
                    compartment,
                    phase,
                })
        };

        let baseline = get(
            TracerCompartment::ReceiverPlant,
            TracerPhase::PreLabelBaseline,
        )?;
        let donor_baseline = get(TracerCompartment::DonorPlant, TracerPhase::PreLabelBaseline)?;
        let donor = get(TracerCompartment::DonorPlant, TracerPhase::PostLabel)?;
        let receiver = get(TracerCompartment::ReceiverPlant, TracerPhase::PostLabel)?;
        let bridge = if require_bridge {
            Some(get(
                TracerCompartment::FungalBridge,
                TracerPhase::PostLabel,
            )?)
        } else {
            samples
                .get(&(TracerCompartment::FungalBridge, TracerPhase::PostLabel))
                .copied()
        };

        let expected_replicate = baseline.replicate;
        if [
            Some(baseline),
            Some(donor_baseline),
            Some(donor),
            bridge,
            Some(receiver),
        ]
        .into_iter()
        .flatten()
        .any(|sample| {
            sample.block != block
                || sample.intervention != intervention
                || sample.replicate != expected_replicate
        }) {
            return Err(IsotopeTransferError::InconsistentReplicate {
                block,
                intervention,
            });
        }
        if !require_bridge && bridge.is_some() {
            return Err(IsotopeTransferError::UnexpectedSampleRole {
                block,
                intervention,
            });
        }

        let time = |sample: &IsotopeTracerObservation| {
            aligned
                .get(&sample.evidence)
                .copied()
                .ok_or(IsotopeTransferError::MissingReceipt(sample.evidence))
        };

        Ok(ArmSamples {
            baseline,
            donor_baseline,
            donor,
            bridge,
            receiver,
            baseline_time: time(baseline)?,
            donor_baseline_time: time(donor_baseline)?,
            donor_time: time(donor)?,
            bridge_time: match bridge {
                Some(sample) => Some(time(sample)?),
                None => None,
            },
            receiver_time: time(receiver)?,
        })
    }

    fn baselines_precede_donor(
        donor_baseline: AlignedExperimentTimestamp,
        receiver_baseline: AlignedExperimentTimestamp,
        donor: AlignedExperimentTimestamp,
    ) -> Result<bool, IsotopeTransferError> {
        let donor_baseline_before_label = ExperimentTimeAligner::compare(donor_baseline, donor)
            .map_err(IsotopeTransferError::TemporalAlignment)?;
        let receiver_baseline_before_label =
            ExperimentTimeAligner::compare(receiver_baseline, donor)
                .map_err(IsotopeTransferError::TemporalAlignment)?;
        Ok(
            donor_baseline_before_label == TemporalOrdering::FirstBeforeSecond
                && receiver_baseline_before_label == TemporalOrdering::FirstBeforeSecond,
        )
    }

    fn order_is_confirmed_pair(
        donor: AlignedExperimentTimestamp,
        receiver: AlignedExperimentTimestamp,
    ) -> Result<bool, IsotopeTransferError> {
        let donor_before_receiver = ExperimentTimeAligner::compare(donor, receiver)
            .map_err(IsotopeTransferError::TemporalAlignment)?;
        Ok(donor_before_receiver == TemporalOrdering::FirstBeforeSecond)
    }

    fn order_is_confirmed(
        donor: AlignedExperimentTimestamp,
        bridge: AlignedExperimentTimestamp,
        receiver: AlignedExperimentTimestamp,
    ) -> Result<bool, IsotopeTransferError> {
        let donor_before_bridge = ExperimentTimeAligner::compare(donor, bridge)
            .map_err(IsotopeTransferError::TemporalAlignment)?;
        let bridge_before_receiver = ExperimentTimeAligner::compare(bridge, receiver)
            .map_err(IsotopeTransferError::TemporalAlignment)?;
        Ok(donor_before_bridge == TemporalOrdering::FirstBeforeSecond
            && bridge_before_receiver == TemporalOrdering::FirstBeforeSecond)
    }

    fn network_route_supported(
        evidence: &NetworkEvidenceAssessment,
        claim_level: NetworkClaimLevel,
    ) -> bool {
        if evidence.cytoplasmic_flow.outcome
            != CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved
        {
            return false;
        }
        let outcome = |criterion| {
            evidence
                .criteria
                .iter()
                .find(|item| item.criterion == criterion)
                .map(|item| item.outcome)
                .unwrap_or(EvidenceOutcome::Unreported)
        };
        let common_path = evidence.shared_fungal_link.outcome == EvidenceOutcome::Pass
            && outcome(NetworkCriterion::AtLeastTwoPlantUnits) == EvidenceOutcome::Pass
            && outcome(NetworkCriterion::RootSystemsSpatiallySeparated) == EvidenceOutcome::Pass
            && outcome(NetworkCriterion::HyphalLinkingTreatment) == EvidenceOutcome::Pass
            && outcome(NetworkCriterion::HyphalContinuityTested) == EvidenceOutcome::Pass;
        if !common_path {
            return false;
        }
        match evidence.fungal_identity {
            FungalNetworkIdentity::ArbuscularMycorrhizal => {
                claim_level == NetworkClaimLevel::CommonMycorrhizalNetworkHyphalContinuitySupported
                    && outcome(NetworkCriterion::AllLinkedPlantsAmColonized)
                        == EvidenceOutcome::Pass
            }
            FungalNetworkIdentity::OtherFilamentousFungus => {
                claim_level == NetworkClaimLevel::CommonFungalNetworkSupported
                    && outcome(NetworkCriterion::AllLinkedPlantsAmColonized)
                        == EvidenceOutcome::NotApplicable
            }
            FungalNetworkIdentity::MixedOrUnresolved => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation_evidence::{
        EvidenceDisposition, ImageFrameIndex, ImageFrameReference, ImageRoiBoundsPx,
        ObservationRoiId, PhysicalSamplingCalibration, Sha256Digest,
    };
    use crate::temporal_alignment::ExperimentClockId;

    fn receipt(
        id: u64,
        subject: ObservationSubject,
        timestamp_s: f64,
    ) -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(id),
            source_file_sha256: Sha256Digest([1; 32]),
            frame: ImageFrameReference {
                frame: ImageFrameIndex(id as u32),
                slice: None,
                timestamp_s,
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
                voxel_depth_um: None,
                frame_interval_s: None,
            },
            processor_name: "isotope_tracer_fixture".to_owned(),
            processor_revision: "v1".to_owned(),
            processor_parameters_sha256: Sha256Digest([2; 32]),
            subject,
            graph_target: None,
            graph_event: None,
            uncertainty: None,
            disposition: EvidenceDisposition::MachineGenerated,
        }
    }

    fn network_evidence() -> NetworkEvidenceAssessment {
        use crate::network_evidence::{
            CriterionEvidence, CytoplasmicFlowReceipt, NetworkCriterion, SharedFungalLinkEvidence,
        };
        let criteria = [
            NetworkCriterion::AtLeastTwoPlantUnits,
            NetworkCriterion::AllLinkedPlantsAmColonized,
            NetworkCriterion::RootSystemsSpatiallySeparated,
            NetworkCriterion::HyphalLinkingTreatment,
            NetworkCriterion::HyphalContinuityTested,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, criterion)| CriterionEvidence {
            criterion,
            outcome: EvidenceOutcome::Pass,
            observation: Some(ObservationEvidenceId(800 + index as u64)),
            reason_code: "source_linked_network_criterion".to_owned(),
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("five criteria");

        NetworkEvidenceAssessment {
            fungal_identity: FungalNetworkIdentity::ArbuscularMycorrhizal,
            shared_fungal_link: SharedFungalLinkEvidence {
                outcome: EvidenceOutcome::Pass,
                observation: Some(ObservationEvidenceId(899)),
                reason_code: "shared_fungal_link_source_receipt".to_owned(),
            },
            criteria,
            cytoplasmic_flow: CytoplasmicFlowReceipt {
                outcome: CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved,
                observation: Some(ObservationEvidenceId(900)),
                reason_code: "flow_observed_on_continuous_hyphal_link".to_owned(),
            },
        }
    }

    fn build_experiment() -> (
        IsotopeTransferExperiment,
        Vec<ObservationEvidenceReceipt>,
        ExperimentTimeAligner,
    ) {
        let id = TracerExperimentId(7);
        let mut receipts = vec![receipt(
            899,
            ObservationSubject::SymbiosisInterfaceRegistration,
            0.0,
        )];
        for index in 0..5 {
            receipts.push(receipt(
                800 + index,
                ObservationSubject::MatchedControlCondition,
                0.0,
            ));
        }
        receipts.push(receipt(900, ObservationSubject::CytoplasmicFlowStream, 2.0));

        let all_arms = TracerIntervention::ALL;
        let arm_evidence = all_arms
            .iter()
            .enumerate()
            .map(|(index, intervention)| {
                let evidence = ObservationEvidenceId(700 + index as u64);
                receipts.push(receipt(
                    evidence.0,
                    ObservationSubject::MatchedControlCondition,
                    0.0,
                ));
                TracerArmEvidence {
                    intervention: *intervention,
                    evidence,
                }
            })
            .collect::<Vec<_>>();

        let mut observations = Vec::new();
        let mut next_id = 1_000_u64;
        for block in 1..=3_u64 {
            for intervention in TracerIntervention::ALL {
                let replicate = TracerReplicateId(block * 10 + intervention as u64);
                let (donor_atom, bridge_atom, receiver_atom) = match intervention {
                    TracerIntervention::IntactHyphalConnection => (0.05, Some(0.03), 0.02),
                    TracerIntervention::HyphaeSevered => (0.05, None, 0.011),
                    TracerIntervention::DiffusionOnlyBarrier => (0.05, None, 0.0115),
                    TracerIntervention::NoFungus => (0.05, None, 0.0104),
                    TracerIntervention::MicrobialOnly => (0.05, None, 0.011),
                    TracerIntervention::NoDonorLabel => (0.010, None, 0.0103),
                };
                let mut push = |compartment, phase, atom_fraction, timestamp_s| {
                    let evidence = ObservationEvidenceId(next_id);
                    next_id += 1;
                    observations.push(IsotopeTracerObservation {
                        evidence,
                        experiment: id,
                        block: TracerBlockId(block),
                        replicate,
                        intervention,
                        compartment,
                        phase,
                        isotope: StableIsotope::Carbon13,
                        atom_fraction,
                        atom_fraction_error_bound: 0.0001,
                        total_element_mass_mg: None,
                        total_element_mass_error_bound_mg: None,
                        raw_timestamp: RawExperimentTimestamp {
                            clock: ExperimentClockId(1),
                            seconds: timestamp_s,
                            error_bound_s: 0.0001,
                        },
                    });
                    receipts.push(receipt(
                        evidence.0,
                        ObservationSubject::IsotopeTracerTransfer,
                        timestamp_s,
                    ));
                };

                push(
                    TracerCompartment::ReceiverPlant,
                    TracerPhase::PreLabelBaseline,
                    0.010,
                    0.0,
                );
                push(
                    TracerCompartment::DonorPlant,
                    TracerPhase::PreLabelBaseline,
                    0.010,
                    0.25,
                );
                push(
                    TracerCompartment::DonorPlant,
                    TracerPhase::PostLabel,
                    donor_atom,
                    1.0,
                );
                if let Some(bridge_atom) = bridge_atom {
                    push(
                        TracerCompartment::FungalBridge,
                        TracerPhase::PostLabel,
                        bridge_atom,
                        2.0,
                    );
                }
                push(
                    TracerCompartment::ReceiverPlant,
                    TracerPhase::PostLabel,
                    receiver_atom,
                    3.0,
                );
            }
        }

        let experiment = IsotopeTransferExperiment {
            id,
            isotope: StableIsotope::Carbon13,
            required_control_arms: TracerIntervention::REQUIRED_CONTROLS.to_vec(),
            arm_evidence,
            observations,
            network_evidence: network_evidence(),
            minimum_matched_blocks: 3,
        };
        (
            experiment,
            receipts,
            ExperimentTimeAligner::new(ExperimentClockId(1)),
        )
    }

    #[test]
    fn intact_route_requires_network_tracer_and_all_matched_controls() {
        let (experiment, receipts, aligner) = build_experiment();
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(
            report.status,
            IsotopeTransferStatus::RouteConsistentEvidenceCandidate
        );
        assert_eq!(report.matched_block_count, 3);
        assert!(report.network_route_evidence_supported);
        assert!(!report.causal_claim_permitted);
        assert!(!report.statistical_inference_performed);
        assert!(report.blocks.iter().all(|block| {
            block
                .control_contrasts
                .iter()
                .all(|contrast| contrast.intact_minus_control_lower_bound > 0.0)
        }));
    }

    #[test]
    fn donorless_arm_is_not_misclassified_as_a_labelled_route() {
        let (mut experiment, receipts, aligner) = build_experiment();
        for observation in &mut experiment.observations {
            if observation.intervention == TracerIntervention::IntactHyphalConnection
                && observation.compartment == TracerCompartment::DonorPlant
            {
                observation.atom_fraction = 0.0101;
            }
        }
        for observation in &mut experiment.observations {
            let receipt = receipts
                .iter()
                .find(|receipt| receipt.id == observation.evidence)
                .unwrap();
            assert_eq!(
                receipt.frame.timestamp_s.to_bits(),
                observation.raw_timestamp.seconds.to_bits()
            );
        }
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(report.status, IsotopeTransferStatus::DonorLabelNotResolved);
    }

    #[test]
    fn receiver_signal_not_above_baseline_remains_unresolved() {
        let (mut experiment, receipts, aligner) = build_experiment();
        for observation in &mut experiment.observations {
            if observation.intervention == TracerIntervention::IntactHyphalConnection
                && observation.compartment == TracerCompartment::ReceiverPlant
                && observation.phase == TracerPhase::PostLabel
            {
                observation.atom_fraction = 0.01005;
            }
        }
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(
            report.status,
            IsotopeTransferStatus::ReceiverEnrichmentNotResolved
        );
    }

    #[test]
    fn diffusion_or_microbial_control_matching_intact_blocks_route_claim() {
        let (mut experiment, receipts, aligner) = build_experiment();
        for observation in &mut experiment.observations {
            if observation.intervention == TracerIntervention::DiffusionOnlyBarrier
                && observation.compartment == TracerCompartment::ReceiverPlant
                && observation.phase == TracerPhase::PostLabel
            {
                observation.atom_fraction = 0.0199;
            }
        }
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(
            report.status,
            IsotopeTransferStatus::RouteNotDistinguishedFromControls
        );
    }

    #[test]
    fn missing_required_control_fails_closed() {
        let (mut experiment, receipts, aligner) = build_experiment();
        experiment
            .required_control_arms
            .retain(|arm| *arm != TracerIntervention::MicrobialOnly);
        assert_eq!(
            experiment.assess(&receipts, &aligner),
            Err(IsotopeTransferError::MissingRequiredControlArm(
                TracerIntervention::MicrobialOnly
            ))
        );
    }

    #[test]
    fn missing_sample_receipt_fails_closed() {
        let (experiment, mut receipts, aligner) = build_experiment();
        let id = experiment.observations[0].evidence;
        receipts.retain(|receipt| receipt.id != id);
        assert_eq!(
            experiment.assess(&receipts, &aligner),
            Err(IsotopeTransferError::MissingReceipt(id))
        );
    }

    #[test]
    fn sample_receipt_subject_must_be_isotope_tracer_transfer() {
        let (experiment, mut receipts, aligner) = build_experiment();
        let id = experiment.observations[0].evidence;
        let receipt = receipts
            .iter_mut()
            .find(|receipt| receipt.id == id)
            .unwrap();
        receipt.subject = ObservationSubject::FluorescenceRoi;
        assert_eq!(
            experiment.assess(&receipts, &aligner),
            Err(IsotopeTransferError::WrongSampleReceiptSubject(id))
        );
    }

    #[test]
    fn raw_sample_time_must_match_source_receipt_time() {
        let (mut experiment, receipts, aligner) = build_experiment();
        experiment.observations[0].raw_timestamp.seconds = 0.5;
        assert_eq!(
            experiment.assess(&receipts, &aligner),
            Err(IsotopeTransferError::ReceiptTimestampMismatch(
                experiment.observations[0].evidence
            ))
        );
    }

    #[test]
    fn overlapping_donor_bridge_time_intervals_are_indeterminate() {
        let (mut experiment, receipts, aligner) = build_experiment();
        let bridge = experiment
            .observations
            .iter_mut()
            .find(|observation| {
                observation.intervention == TracerIntervention::IntactHyphalConnection
                    && observation.block == TracerBlockId(1)
                    && observation.compartment == TracerCompartment::FungalBridge
            })
            .unwrap();
        bridge.raw_timestamp.seconds = 1.0;
        bridge.raw_timestamp.error_bound_s = 0.001;
        let bridge_id = bridge.evidence;
        let receipt = receipts
            .iter()
            .find(|receipt| receipt.id == bridge_id)
            .unwrap();
        assert_ne!(
            receipt.frame.timestamp_s.to_bits(),
            bridge.raw_timestamp.seconds.to_bits()
        );
        // Source and sample timestamps must first agree; a true overlap case is
        // represented by equal, source-matched values and non-zero time bounds.
        let mut receipts = receipts;
        receipts
            .iter_mut()
            .find(|receipt| receipt.id == bridge_id)
            .unwrap()
            .frame
            .timestamp_s = 1.0;
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(
            report.status,
            IsotopeTransferStatus::TemporalOrderIndeterminate
        );
    }

    #[test]
    fn fewer_than_three_matched_blocks_cannot_receive_route_candidate() {
        let (mut experiment, receipts, aligner) = build_experiment();
        experiment
            .observations
            .retain(|observation| observation.block != TracerBlockId(3));
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(
            report.status,
            IsotopeTransferStatus::InsufficientMatchedBlocks
        );
        assert_eq!(report.matched_block_count, 2);
    }

    #[test]
    fn non_am_fungal_identity_remains_cfn_not_cmn() {
        let (mut experiment, receipts, aligner) = build_experiment();
        experiment.network_evidence.fungal_identity = FungalNetworkIdentity::OtherFilamentousFungus;
        experiment.network_evidence.criteria[1].outcome = EvidenceOutcome::NotApplicable;
        experiment.network_evidence.criteria[1].observation = None;
        let mut receipts = receipts;
        receipts.retain(|receipt| receipt.id != ObservationEvidenceId(801));
        let report = experiment.assess(&receipts, &aligner).unwrap();
        assert_eq!(
            report.network_claim_level,
            NetworkClaimLevel::CommonFungalNetworkSupported
        );
        assert_eq!(
            report.status,
            IsotopeTransferStatus::RouteConsistentEvidenceCandidate
        );
    }

    #[test]
    fn duplicate_receipt_id_is_rejected_even_when_sample_id_resolves() {
        let (experiment, mut receipts, aligner) = build_experiment();
        receipts.push(receipts[0].clone());
        assert!(matches!(
            experiment.assess(&receipts, &aligner),
            Err(IsotopeTransferError::DuplicateReceipt(_))
                | Err(IsotopeTransferError::NetworkEvidence(
                    NetworkEvidenceError::DuplicateObservationReceipt(_)
                ))
        ));
    }

    #[test]
    fn optional_element_pool_produces_inventory_not_transfer_claim() {
        let (mut experiment, receipts, aligner) = build_experiment();
        for observation in &mut experiment.observations {
            if observation.intervention == TracerIntervention::IntactHyphalConnection
                && observation.block == TracerBlockId(1)
                && observation.compartment == TracerCompartment::ReceiverPlant
                && observation.phase == TracerPhase::PostLabel
            {
                observation.total_element_mass_mg = Some(10.0);
                observation.total_element_mass_error_bound_mg = Some(0.2);
            }
        }
        let report = experiment.assess(&receipts, &aligner).unwrap();
        let inventory = report.blocks[0].receiver_excess_inventory.unwrap();
        assert!(inventory.estimate_mg > 0.0);
        assert!(inventory.error_bound_mg > 0.0);
        assert!(!report.causal_claim_permitted);
    }
}
