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
pub enum EvidenceDisposition {
    MachineGenerated,
    HumanReviewed(HumanReviewReceipt),
    IndependentlyAdjudicated(HumanReviewReceipt),
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
            Self::InvalidDigest => write!(f, "source and processor digests must not be placeholder zero values"),
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
                write!(f, "evidence references graph event {:?} that has not been applied", id)
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
            EvidenceDisposition::HumanReviewed(review)
            | EvidenceDisposition::IndependentlyAdjudicated(review) => {
                if review.reviewer_key.trim().is_empty() {
                    return Err(EvidenceReceiptError::EmptyReviewerKey);
                }
                if review.reason_code.trim().is_empty() {
                    return Err(EvidenceReceiptError::EmptyReviewReason);
                }
            }
        }
        Ok(())
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
