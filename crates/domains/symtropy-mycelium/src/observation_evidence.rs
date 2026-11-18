// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-linked evidence receipts for observed fungal morphology and dynamics.
//!
//! The receipt binds an observation to an immutable source-file digest,
//! acquisition frame/ROI, physical calibration, processing revision and
//! parameter digest. It does not imply that an observation is correct or that
//! a linked graph event is biologically validated.

use std::fmt;

use crate::hyphal_graph::{EventId, HyphalGraph, NodeId, SegmentId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationEvidenceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageFrameIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageSliceIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationRoiId(pub u64);

/// A caller-computed SHA-256 digest of an immutable source or parameter file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest(pub [u8; 32]);

impl Sha256Digest {
    pub fn is_zero(self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalSamplingCalibration {
    pub pixel_width_um: f64,
    pub pixel_height_um: f64,
    /// None for a truly 2D acquisition or when Z spacing is unknown.
    pub voxel_depth_um: Option<f64>,
    /// None when timestamps are not evenly sampled / frame cadence is unknown.
    pub frame_interval_s: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageFrameReference {
    pub frame: ImageFrameIndex,
    pub slice: Option<ImageSliceIndex>,
    /// Timestamp relative to the acquisition origin.
    pub timestamp_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRoiBoundsPx {
    pub id: ObservationRoiId,
    pub x_min: u32,
    pub y_min: u32,
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationSubject {
    HyphalSegmentTrace,
    BranchPointCandidate,
    JunctionCandidate,
    CrossingCandidate,
    DepthEstimate,
    GrowingTip,
    CytoplasmicFlowStream,
    FluorescenceRoi,
    /// A measured liquid patch in a hyphal-highway assay.
    LiquidFilmPatch,
    /// Evidence that an aqueous film connects two distinct liquid patches.
    LiquidFilmBridge,
    /// An individually tracked bacterial trajectory in a microhabitat network.
    BacterialTrajectory,
    /// A spontaneous or isotope-probing Raman spectrum acquired from a hyphal ROI.
    RamanSpectrum,
    /// An SRS chemical map acquired from a hyphal ROI.
    RamanChemicalMap,
    /// An independently sourced spectral calibration / reference observation.
    RamanCalibrationReference,
    /// A synchronization pulse/trigger used to align separate acquisition clocks.
    ClockSynchronization,
    /// A source-linked mapping of one plant/fungus interface across modalities.
    SymbiosisInterfaceRegistration,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceGraphTarget {
    Node(NodeId),
    Segment(SegmentId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanDecision {
    Confirmed,
    Rejected,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanReviewReceipt {
    /// Pseudonymous reviewer key; do not store personal identifiers here.
    pub reviewer_key: String,
    pub decision: HumanDecision,
    /// Stable code, e.g. "depth_not_resolved" or "continuity_confirmed".
    pub reason_code: String,
    pub reviewed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndependentAdjudicationReceipt {
    /// Two or more distinct pseudonymous reviewer keys.
    pub reviewer_keys: Vec<String>,
    pub decision: HumanDecision,
    pub reason_code: String,
    pub reviewed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceDisposition {
    MachineGenerated,
    HumanReviewed(HumanReviewReceipt),
    IndependentlyAdjudicated(IndependentAdjudicationReceipt),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialUncertaintyUm {
    pub x_std_dev_um: Option<f64>,
    pub y_std_dev_um: Option<f64>,
    pub z_std_dev_um: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationEvidenceReceipt {
    pub id: ObservationEvidenceId,
    pub source_file_sha256: Sha256Digest,
    pub frame: ImageFrameReference,
    pub roi: ImageRoiBoundsPx,
    pub calibration: PhysicalSamplingCalibration,
    pub processor_name: String,
    pub processor_revision: String,
    pub processor_parameters_sha256: Sha256Digest,
    pub subject: ObservationSubject,
    pub graph_target: Option<EvidenceGraphTarget>,
    /// Event that consumed/accepted this evidence, if a topology mutation occurred.
    pub graph_event: Option<EventId>,
    pub uncertainty: Option<SpatialUncertaintyUm>,
    pub disposition: EvidenceDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceReceiptError {
    InvalidDigest,
    InvalidTimestamp,
    InvalidPixelCalibration,
    InvalidVoxelCalibration,
    InvalidFrameInterval,
    InvalidRoi,
    EmptyProcessorName,
    EmptyProcessorRevision,
    EmptyReviewerKey,
    EmptyReviewReason,
    InsufficientIndependentReviewers,
    DuplicateReviewerKey,
    StringTooLong,
    InvalidSpatialUncertainty,
    DepthCalibrationRequired,
    ThreeDimensionalUncertaintyRequiresDepthCalibration,
    MissingGraphNode(NodeId),
    MissingGraphSegment(SegmentId),
    MissingGraphEvent(EventId),
}

impl fmt::Display for EvidenceReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDigest => write!(
                f,
                "source and processor digests must not be placeholder zero values"
            ),
            Self::InvalidTimestamp => write!(f, "image timestamp must be finite and non-negative"),
            Self::InvalidPixelCalibration => {
                write!(
                    f,
                    "pixel width/height calibration must be finite and positive"
                )
            }
            Self::InvalidVoxelCalibration => {
                write!(
                    f,
                    "voxel depth calibration must be finite and positive when present"
                )
            }
            Self::InvalidFrameInterval => {
                write!(f, "frame interval must be finite and positive when present")
            }
            Self::InvalidRoi => write!(f, "ROI width/height and pixel bounds must be valid"),
            Self::EmptyProcessorName => write!(f, "processor name cannot be empty"),
            Self::EmptyProcessorRevision => write!(f, "processor revision cannot be empty"),
            Self::EmptyReviewerKey => write!(f, "human reviewer key cannot be empty"),
            Self::EmptyReviewReason => write!(f, "human review reason code cannot be empty"),
            Self::InsufficientIndependentReviewers => {
                write!(
                    f,
                    "independent adjudication requires at least two reviewers"
                )
            }
            Self::DuplicateReviewerKey => {
                write!(f, "independent adjudication reviewer keys must be distinct")
            }
            Self::StringTooLong => write!(f, "canonical receipt string exceeds u32 length limit"),
            Self::InvalidSpatialUncertainty => {
                write!(f, "spatial uncertainty must be finite and non-negative")
            }
            Self::DepthCalibrationRequired => {
                write!(f, "a depth estimate requires explicit Z calibration")
            }
            Self::ThreeDimensionalUncertaintyRequiresDepthCalibration => {
                write!(f, "Z uncertainty requires explicit Z calibration")
            }
            Self::MissingGraphNode(id) => {
                write!(
                    f,
                    "evidence references graph node {:?} that does not exist",
                    id
                )
            }
            Self::MissingGraphSegment(id) => {
                write!(
                    f,
                    "evidence references graph segment {:?} that does not exist",
                    id
                )
            }
            Self::MissingGraphEvent(id) => {
                write!(
                    f,
                    "evidence references graph event {:?} that has not been applied",
                    id
                )
            }
        }
    }
}

impl std::error::Error for EvidenceReceiptError {}

impl PhysicalSamplingCalibration {
    pub fn validate(self) -> Result<(), EvidenceReceiptError> {
        if !self.pixel_width_um.is_finite()
            || self.pixel_width_um <= 0.0
            || !self.pixel_height_um.is_finite()
            || self.pixel_height_um <= 0.0
        {
            return Err(EvidenceReceiptError::InvalidPixelCalibration);
        }
        if self
            .voxel_depth_um
            .is_some_and(|depth| !depth.is_finite() || depth <= 0.0)
        {
            return Err(EvidenceReceiptError::InvalidVoxelCalibration);
        }
        if self
            .frame_interval_s
            .is_some_and(|interval| !interval.is_finite() || interval <= 0.0)
        {
            return Err(EvidenceReceiptError::InvalidFrameInterval);
        }
        Ok(())
    }
}

impl ImageRoiBoundsPx {
    pub fn validate(self) -> Result<(), EvidenceReceiptError> {
        if self.width_px == 0
            || self.height_px == 0
            || self.x_min.checked_add(self.width_px).is_none()
            || self.y_min.checked_add(self.height_px).is_none()
        {
            return Err(EvidenceReceiptError::InvalidRoi);
        }
        Ok(())
    }
}

impl ObservationEvidenceReceipt {
    pub fn validate(&self) -> Result<(), EvidenceReceiptError> {
        if self.source_file_sha256.is_zero() || self.processor_parameters_sha256.is_zero() {
            return Err(EvidenceReceiptError::InvalidDigest);
        }
        if !self.frame.timestamp_s.is_finite() || self.frame.timestamp_s < 0.0 {
            return Err(EvidenceReceiptError::InvalidTimestamp);
        }
        self.calibration.validate()?;
        self.roi.validate()?;

        if self.processor_name.trim().is_empty() {
            return Err(EvidenceReceiptError::EmptyProcessorName);
        }
        if self.processor_revision.trim().is_empty() {
            return Err(EvidenceReceiptError::EmptyProcessorRevision);
        }
        if self.subject == ObservationSubject::DepthEstimate
            && self.calibration.voxel_depth_um.is_none()
        {
            return Err(EvidenceReceiptError::DepthCalibrationRequired);
        }

        if let Some(uncertainty) = self.uncertainty {
            for value in [
                uncertainty.x_std_dev_um,
                uncertainty.y_std_dev_um,
                uncertainty.z_std_dev_um,
            ]
            .into_iter()
            .flatten()
            {
                if !value.is_finite() || value < 0.0 {
                    return Err(EvidenceReceiptError::InvalidSpatialUncertainty);
                }
            }
            if uncertainty.z_std_dev_um.is_some() && self.calibration.voxel_depth_um.is_none() {
                return Err(
                    EvidenceReceiptError::ThreeDimensionalUncertaintyRequiresDepthCalibration,
                );
            }
        }

        match &self.disposition {
            EvidenceDisposition::MachineGenerated => {}
            EvidenceDisposition::HumanReviewed(review) => {
                if review.reviewer_key.trim().is_empty() {
                    return Err(EvidenceReceiptError::EmptyReviewerKey);
                }
                if review.reason_code.trim().is_empty() {
                    return Err(EvidenceReceiptError::EmptyReviewReason);
                }
            }
            EvidenceDisposition::IndependentlyAdjudicated(review) => {
                if review.reviewer_keys.len() < 2 {
                    return Err(EvidenceReceiptError::InsufficientIndependentReviewers);
                }
                let mut reviewer_keys = std::collections::BTreeSet::new();
                for key in &review.reviewer_keys {
                    if key.trim().is_empty() {
                        return Err(EvidenceReceiptError::EmptyReviewerKey);
                    }
                    if !reviewer_keys.insert(key) {
                        return Err(EvidenceReceiptError::DuplicateReviewerKey);
                    }
                }
                if review.reason_code.trim().is_empty() {
                    return Err(EvidenceReceiptError::EmptyReviewReason);
                }
            }
        }
        Ok(())
    }

    /// Return deterministic, versioned bytes for external hashing or archival.
    ///
    /// Strings use UTF-8 with a little-endian u32 byte length. Integers and
    /// IEEE-754 f64 bit patterns use little-endian encoding. Optional fields
    /// have explicit 0/1 tags. Negative zero is normalized to positive zero.
    /// Reviewer keys in independent adjudication are serialized in sorted
    /// order because the panel is semantically an unordered set.
    ///
    /// This method does not compute SHA-256; it defines canonical bytes only.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EvidenceReceiptError> {
        self.validate()?;
        let mut out = Vec::with_capacity(256);
        out.extend_from_slice(b"MYCO-EVIDENCE-RECEIPT\0");
        push_u16(&mut out, 1);

        push_u64(&mut out, self.id.0);
        out.extend_from_slice(&self.source_file_sha256.0);
        push_u32(&mut out, self.frame.frame.0);
        match self.frame.slice {
            Some(slice) => {
                push_u8(&mut out, 1);
                push_u32(&mut out, slice.0);
            }
            None => push_u8(&mut out, 0),
        }
        push_f64(&mut out, self.frame.timestamp_s);

        push_u64(&mut out, self.roi.id.0);
        push_u32(&mut out, self.roi.x_min);
        push_u32(&mut out, self.roi.y_min);
        push_u32(&mut out, self.roi.width_px);
        push_u32(&mut out, self.roi.height_px);

        push_f64(&mut out, self.calibration.pixel_width_um);
        push_f64(&mut out, self.calibration.pixel_height_um);
        push_optional_f64(&mut out, self.calibration.voxel_depth_um);
        push_optional_f64(&mut out, self.calibration.frame_interval_s);

        push_string(&mut out, &self.processor_name)?;
        push_string(&mut out, &self.processor_revision)?;
        out.extend_from_slice(&self.processor_parameters_sha256.0);
        push_u8(&mut out, subject_tag(self.subject));

        match self.graph_target {
            Some(EvidenceGraphTarget::Node(id)) => {
                push_u8(&mut out, 1);
                push_u64(&mut out, id.0);
            }
            Some(EvidenceGraphTarget::Segment(id)) => {
                push_u8(&mut out, 2);
                push_u64(&mut out, id.0);
            }
            None => push_u8(&mut out, 0),
        }
        match self.graph_event {
            Some(event_id) => {
                push_u8(&mut out, 1);
                push_u64(&mut out, event_id.0);
            }
            None => push_u8(&mut out, 0),
        }

        match self.uncertainty {
            Some(uncertainty) => {
                push_u8(&mut out, 1);
                push_optional_f64(&mut out, uncertainty.x_std_dev_um);
                push_optional_f64(&mut out, uncertainty.y_std_dev_um);
                push_optional_f64(&mut out, uncertainty.z_std_dev_um);
            }
            None => push_u8(&mut out, 0),
        }

        match &self.disposition {
            EvidenceDisposition::MachineGenerated => push_u8(&mut out, 0),
            EvidenceDisposition::HumanReviewed(review) => {
                push_u8(&mut out, 1);
                push_string(&mut out, &review.reviewer_key)?;
                push_u8(&mut out, decision_tag(review.decision));
                push_string(&mut out, &review.reason_code)?;
                push_u64(&mut out, review.reviewed_at_unix_ms);
            }
            EvidenceDisposition::IndependentlyAdjudicated(review) => {
                push_u8(&mut out, 2);
                let mut reviewers: Vec<&str> =
                    review.reviewer_keys.iter().map(String::as_str).collect();
                reviewers.sort_unstable();
                push_u32(
                    &mut out,
                    u32::try_from(reviewers.len())
                        .map_err(|_| EvidenceReceiptError::StringTooLong)?,
                );
                for reviewer in reviewers {
                    push_string(&mut out, reviewer)?;
                }
                push_u8(&mut out, decision_tag(review.decision));
                push_string(&mut out, &review.reason_code)?;
                push_u64(&mut out, review.reviewed_at_unix_ms);
            }
        }
        Ok(out)
    }

    pub fn validate_graph_target(&self, graph: &HyphalGraph) -> Result<(), EvidenceReceiptError> {
        self.validate()?;
        match self.graph_target {
            Some(EvidenceGraphTarget::Node(id)) if graph.node(id).is_none() => {
                Err(EvidenceReceiptError::MissingGraphNode(id))
            }
            Some(EvidenceGraphTarget::Segment(id)) if graph.segment(id).is_none() => {
                Err(EvidenceReceiptError::MissingGraphSegment(id))
            }
            _ => {
                if let Some(event_id) = self.graph_event
                    && !graph.contains_event(event_id)
                {
                    return Err(EvidenceReceiptError::MissingGraphEvent(event_id));
                }
                Ok(())
            }
        }
    }
}

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f64(out: &mut Vec<u8>, value: f64) {
    let bits = if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    };
    push_u64(out, bits);
}

fn push_optional_f64(out: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            push_u8(out, 1);
            push_f64(out, value);
        }
        None => push_u8(out, 0),
    }
}

fn push_string(out: &mut Vec<u8>, value: &str) -> Result<(), EvidenceReceiptError> {
    let length = u32::try_from(value.len()).map_err(|_| EvidenceReceiptError::StringTooLong)?;
    push_u32(out, length);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

const fn decision_tag(decision: HumanDecision) -> u8 {
    match decision {
        HumanDecision::Confirmed => 1,
        HumanDecision::Rejected => 2,
        HumanDecision::Ambiguous => 3,
    }
}

const fn subject_tag(subject: ObservationSubject) -> u8 {
    match subject {
        ObservationSubject::HyphalSegmentTrace => 1,
        ObservationSubject::BranchPointCandidate => 2,
        ObservationSubject::JunctionCandidate => 3,
        ObservationSubject::CrossingCandidate => 4,
        ObservationSubject::DepthEstimate => 5,
        ObservationSubject::GrowingTip => 6,
        ObservationSubject::CytoplasmicFlowStream => 7,
        ObservationSubject::FluorescenceRoi => 8,
        ObservationSubject::LiquidFilmPatch => 9,
        ObservationSubject::BacterialTrajectory => 10,
        ObservationSubject::LiquidFilmBridge => 11,
        ObservationSubject::RamanSpectrum => 12,
        ObservationSubject::RamanChemicalMap => 13,
        ObservationSubject::RamanCalibrationReference => 14,
        ObservationSubject::ClockSynchronization => 15,
        ObservationSubject::SymbiosisInterfaceRegistration => 16,
        ObservationSubject::PlantRootExudateSignal => 17,
        ObservationSubject::SymbioticSignalResponse => 18,
        ObservationSubject::ArbusculeInterfaceMorphology => 19,
        ObservationSubject::HostCarbonInvestment => 20,
        ObservationSubject::FungalNutrientDelivery => 21,
        ObservationSubject::ArbusculeTurnover => 22,
        ObservationSubject::FungalRegulatorySignal => 23,
        ObservationSubject::IsotopeTracerTransfer => 24,
        ObservationSubject::HostNutrientResponse => 25,
        ObservationSubject::MatchedControlCondition => 26,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyphal_graph::{
        HyphalNode, HyphalNodeKind, HyphalSegment, HyphalSegmentState, Position3Mm,
    };

    fn receipt() -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(1),
            source_file_sha256: Sha256Digest([1; 32]),
            frame: ImageFrameReference {
                frame: ImageFrameIndex(12),
                slice: Some(ImageSliceIndex(4)),
                timestamp_s: 2.5,
            },
            roi: ImageRoiBoundsPx {
                id: ObservationRoiId(3),
                x_min: 10,
                y_min: 20,
                width_px: 100,
                height_px: 80,
            },
            calibration: PhysicalSamplingCalibration {
                pixel_width_um: 0.25,
                pixel_height_um: 0.25,
                voxel_depth_um: Some(1.0),
                frame_interval_s: Some(0.5),
            },
            processor_name: "synthetic_truth_generator".to_owned(),
            processor_revision: "v0.1.0".to_owned(),
            processor_parameters_sha256: Sha256Digest([2; 32]),
            subject: ObservationSubject::CrossingCandidate,
            graph_target: None,
            graph_event: None,
            uncertainty: Some(SpatialUncertaintyUm {
                x_std_dev_um: Some(0.1),
                y_std_dev_um: Some(0.1),
                z_std_dev_um: Some(0.5),
            }),
            disposition: EvidenceDisposition::MachineGenerated,
        }
    }

    fn graph() -> HyphalGraph {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(HyphalNode {
                id: NodeId(1),
                kind: HyphalNodeKind::Anchor,
                position: Position3Mm::new(0.0, 0.0, 0.0),
            })
            .unwrap();
        graph
            .add_node(HyphalNode {
                id: NodeId(2),
                kind: HyphalNodeKind::ActiveTip,
                position: Position3Mm::new(1.0, 0.0, 0.0),
            })
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(4),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 1.0,
                diameter_mm: None,
                state: HyphalSegmentState::Active,
            })
            .unwrap();
        graph
    }

    #[test]
    fn complete_source_linked_receipt_validates() {
        assert_eq!(receipt().validate(), Ok(()));
    }

    #[test]
    fn placeholder_zero_digests_are_rejected() {
        let mut item = receipt();
        item.source_file_sha256 = Sha256Digest([0; 32]);
        assert_eq!(item.validate(), Err(EvidenceReceiptError::InvalidDigest));

        item = receipt();
        item.processor_parameters_sha256 = Sha256Digest([0; 32]);
        assert_eq!(item.validate(), Err(EvidenceReceiptError::InvalidDigest));
    }

    #[test]
    fn two_dimensional_receipts_may_omit_z_spacing() {
        let mut item = receipt();
        item.calibration.voxel_depth_um = None;
        item.uncertainty.as_mut().unwrap().z_std_dev_um = None;
        assert_eq!(item.validate(), Ok(()));
    }

    #[test]
    fn depth_estimate_requires_z_calibration() {
        let mut item = receipt();
        item.subject = ObservationSubject::DepthEstimate;
        item.calibration.voxel_depth_um = None;
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::DepthCalibrationRequired)
        );
    }

    #[test]
    fn invalid_timestamp_roi_and_calibration_are_rejected() {
        let mut item = receipt();
        item.frame.timestamp_s = f64::NAN;
        assert_eq!(item.validate(), Err(EvidenceReceiptError::InvalidTimestamp));

        item = receipt();
        item.roi.width_px = 0;
        assert_eq!(item.validate(), Err(EvidenceReceiptError::InvalidRoi));

        item = receipt();
        item.calibration.pixel_width_um = 0.0;
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::InvalidPixelCalibration)
        );
    }

    #[test]
    fn z_uncertainty_requires_z_calibration() {
        let mut item = receipt();
        item.calibration.voxel_depth_um = None;
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::ThreeDimensionalUncertaintyRequiresDepthCalibration)
        );
    }

    #[test]
    fn human_review_receipt_requires_pseudonymous_reviewer_and_reason() {
        let mut item = receipt();
        item.disposition = EvidenceDisposition::HumanReviewed(HumanReviewReceipt {
            reviewer_key: " ".to_owned(),
            decision: HumanDecision::Ambiguous,
            reason_code: "depth_unresolved".to_owned(),
            reviewed_at_unix_ms: 1,
        });
        assert_eq!(item.validate(), Err(EvidenceReceiptError::EmptyReviewerKey));
    }

    #[test]
    fn graph_targets_are_checked_against_canonical_graph() {
        let mut item = receipt();
        item.graph_target = Some(EvidenceGraphTarget::Segment(SegmentId(4)));
        assert_eq!(item.validate_graph_target(&graph()), Ok(()));

        item.graph_target = Some(EvidenceGraphTarget::Segment(SegmentId(99)));
        assert_eq!(
            item.validate_graph_target(&graph()),
            Err(EvidenceReceiptError::MissingGraphSegment(SegmentId(99)))
        );
    }

    #[test]
    fn graph_event_reference_requires_an_applied_event() {
        let graph = graph();
        let mut item = receipt();
        item.graph_event = Some(EventId(99));
        assert_eq!(
            item.validate_graph_target(&graph),
            Err(EvidenceReceiptError::MissingGraphEvent(EventId(99)))
        );
    }

    #[test]
    fn graph_event_reference_accepts_applied_topology_event() {
        use crate::hyphal_graph::BranchEvent;

        let mut graph = graph();
        graph
            .apply_branch(BranchEvent {
                id: EventId(9),
                parent_segment: SegmentId(4),
                attachment_fraction: 0.5,
                branch_node: NodeId(3),
                branch_tip: NodeId(4),
                proximal_segment: SegmentId(5),
                distal_segment: SegmentId(6),
                branch_segment: SegmentId(7),
                branch_tip_position: Position3Mm::new(0.5, 1.0, 0.0),
            })
            .unwrap();

        let mut item = receipt();
        item.graph_event = Some(EventId(9));
        item.graph_target = Some(EvidenceGraphTarget::Node(NodeId(3)));
        assert_eq!(item.validate_graph_target(&graph), Ok(()));
    }

    #[test]
    fn new_hyphal_highway_subjects_have_stable_distinct_tags() {
        assert_eq!(subject_tag(ObservationSubject::LiquidFilmPatch), 9);
        assert_eq!(subject_tag(ObservationSubject::BacterialTrajectory), 10);
        assert_eq!(subject_tag(ObservationSubject::LiquidFilmBridge), 11);
        assert_eq!(subject_tag(ObservationSubject::RamanSpectrum), 12);
        assert_eq!(subject_tag(ObservationSubject::RamanChemicalMap), 13);
        assert_eq!(
            subject_tag(ObservationSubject::RamanCalibrationReference),
            14
        );
        assert_eq!(subject_tag(ObservationSubject::ClockSynchronization), 15);
        assert_eq!(
            subject_tag(ObservationSubject::SymbiosisInterfaceRegistration),
            16
        );
        assert_eq!(subject_tag(ObservationSubject::PlantRootExudateSignal), 17);
        assert_eq!(subject_tag(ObservationSubject::SymbioticSignalResponse), 18);
        assert_eq!(
            subject_tag(ObservationSubject::ArbusculeInterfaceMorphology),
            19
        );
        assert_eq!(subject_tag(ObservationSubject::HostCarbonInvestment), 20);
        assert_eq!(subject_tag(ObservationSubject::FungalNutrientDelivery), 21);
        assert_eq!(subject_tag(ObservationSubject::ArbusculeTurnover), 22);
        assert_eq!(subject_tag(ObservationSubject::FungalRegulatorySignal), 23);
        assert_eq!(subject_tag(ObservationSubject::IsotopeTracerTransfer), 24);
        assert_eq!(subject_tag(ObservationSubject::HostNutrientResponse), 25);
        assert_eq!(subject_tag(ObservationSubject::MatchedControlCondition), 26);
        assert_ne!(
            subject_tag(ObservationSubject::LiquidFilmPatch),
            subject_tag(ObservationSubject::BacterialTrajectory)
        );
        assert_ne!(
            subject_tag(ObservationSubject::ClockSynchronization),
            subject_tag(ObservationSubject::RamanCalibrationReference)
        );
    }

    #[test]
    fn canonical_bytes_are_deterministic_and_versioned() {
        let item = receipt();
        let first = item.canonical_bytes().unwrap();
        let second = item.canonical_bytes().unwrap();
        assert_eq!(first, second);
        assert_eq!(&first[..22], b"MYCO-EVIDENCE-RECEIPT\0");
        assert_eq!(&first[24..32], &1_u64.to_le_bytes());
        assert_eq!(&first[32..64], &[1_u8; 32]);
        assert_eq!(&first[22..24], &1_u16.to_le_bytes());
    }

    #[test]
    fn canonical_bytes_change_when_source_or_roi_changes() {
        let base = receipt().canonical_bytes().unwrap();

        let mut changed_source = receipt();
        changed_source.source_file_sha256 = Sha256Digest([3; 32]);
        assert_ne!(base, changed_source.canonical_bytes().unwrap());

        let mut changed_roi = receipt();
        changed_roi.roi.x_min += 1;
        assert_ne!(base, changed_roi.canonical_bytes().unwrap());
    }

    #[test]
    fn canonical_bytes_normalize_negative_zero() {
        let mut positive_zero = receipt();
        positive_zero.frame.timestamp_s = 0.0;
        let mut negative_zero = receipt();
        negative_zero.frame.timestamp_s = -0.0;
        assert_eq!(
            positive_zero.canonical_bytes().unwrap(),
            negative_zero.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn independent_adjudication_requires_two_distinct_reviewers() {
        let mut item = receipt();
        item.disposition =
            EvidenceDisposition::IndependentlyAdjudicated(IndependentAdjudicationReceipt {
                reviewer_keys: vec!["reviewer-a".to_owned()],
                decision: HumanDecision::Confirmed,
                reason_code: "continuity_confirmed".to_owned(),
                reviewed_at_unix_ms: 2,
            });
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::InsufficientIndependentReviewers)
        );

        item.disposition =
            EvidenceDisposition::IndependentlyAdjudicated(IndependentAdjudicationReceipt {
                reviewer_keys: vec!["reviewer-a".to_owned(), "reviewer-a".to_owned()],
                decision: HumanDecision::Confirmed,
                reason_code: "continuity_confirmed".to_owned(),
                reviewed_at_unix_ms: 2,
            });
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::DuplicateReviewerKey)
        );
    }

    #[test]
    fn independent_adjudication_serialization_is_panel_order_independent() {
        let mut first = receipt();
        first.disposition =
            EvidenceDisposition::IndependentlyAdjudicated(IndependentAdjudicationReceipt {
                reviewer_keys: vec!["reviewer-a".to_owned(), "reviewer-b".to_owned()],
                decision: HumanDecision::Confirmed,
                reason_code: "continuity_confirmed".to_owned(),
                reviewed_at_unix_ms: 2,
            });
        let mut second = first.clone();
        second.disposition =
            EvidenceDisposition::IndependentlyAdjudicated(IndependentAdjudicationReceipt {
                reviewer_keys: vec!["reviewer-b".to_owned(), "reviewer-a".to_owned()],
                decision: HumanDecision::Confirmed,
                reason_code: "continuity_confirmed".to_owned(),
                reviewed_at_unix_ms: 2,
            });
        assert_eq!(
            first.canonical_bytes().unwrap(),
            second.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn spatial_uncertainty_rejects_negative_and_nan_values() {
        let mut item = receipt();
        item.uncertainty.as_mut().unwrap().z_std_dev_um = Some(-0.1);
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::InvalidSpatialUncertainty)
        );

        item = receipt();
        item.uncertainty.as_mut().unwrap().x_std_dev_um = Some(f64::NAN);
        assert_eq!(
            item.validate(),
            Err(EvidenceReceiptError::InvalidSpatialUncertainty)
        );
    }
}
