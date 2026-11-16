// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-gated bacterial movement across hypha-associated liquid films.
//!
//! This module represents a multiplex network: liquid patches, hyphal support,
//! and individual bacterial trajectories are distinct entities. A route is
//! qualified only when every observed path hop has a source-linked active
//! filament segment and a source-linked connected liquid film.
//!
//! It intentionally distinguishes true fungal hyphae from oomycete hyphae.
//! It does not infer fungal cytoplasmic streaming, solute movement, nutrient
//! delivery to plants, or biological causation from topology alone.

use std::collections::BTreeMap;
use std::fmt;

use crate::hyphal_graph::{HyphalGraph, HyphalSegmentState, SegmentId};
use crate::observation_evidence::{
    EvidenceGraphTarget, EvidenceReceiptError, ObservationEvidenceId, ObservationEvidenceReceipt,
    ObservationSubject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiquidPatchId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HyphalHighwayLinkId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BacterialTrackId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BacterialTransitId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilamentDomain {
    TrueFungus,
    Oomycete,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidFilmState {
    Connected,
    Disconnected,
    Unreported,
}

/// One optically resolved liquid patch. Geometry is descriptive, not a fluid
/// volume estimate unless the imaging calibration and segmentation support it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquidPatch {
    pub id: LiquidPatchId,
    pub centroid_x_um: f64,
    pub centroid_y_um: f64,
    pub segmented_area_um2: Option<f64>,
    /// Receipt for the observation that established this patch's identity.
    pub observation: Option<ObservationEvidenceId>,
}

impl LiquidPatch {
    pub fn validate(&self) -> Result<(), HyphalHighwayError> {
        if !self.centroid_x_um.is_finite() || !self.centroid_y_um.is_finite() {
            return Err(HyphalHighwayError::InvalidPatchGeometry(self.id));
        }
        if self
            .segmented_area_um2
            .is_some_and(|area| !area.is_finite() || area <= 0.0)
        {
            return Err(HyphalHighwayError::InvalidPatchArea(self.id));
        }
        Ok(())
    }
}

/// A candidate bridge between two liquid patches, supported by one hyphal
/// segment and two independently sourced observations.
///
/// The liquid state must be measured; the module never assumes that a
/// topological hyphal link implies a continuous aqueous film.
#[derive(Debug, Clone, PartialEq)]
pub struct HyphalHighwayLink {
    pub id: HyphalHighwayLinkId,
    pub from_patch: LiquidPatchId,
    pub to_patch: LiquidPatchId,
    pub filament_domain: FilamentDomain,
    pub hyphal_segment: SegmentId,
    pub liquid_state: LiquidFilmState,
    /// Optional measured film thickness; no default or biological threshold.
    pub film_thickness_um: Option<f64>,
    pub hyphal_observation: Option<ObservationEvidenceId>,
    pub liquid_observation: Option<ObservationEvidenceId>,
}

impl HyphalHighwayLink {
    pub fn validate(&self) -> Result<(), HyphalHighwayError> {
        if self.from_patch == self.to_patch {
            return Err(HyphalHighwayError::SelfLink(self.from_patch));
        }
        if self
            .film_thickness_um
            .is_some_and(|thickness| !thickness.is_finite() || thickness <= 0.0)
        {
            return Err(HyphalHighwayError::InvalidFilmThickness(self.id));
        }
        Ok(())
    }

    fn connects(&self, first: LiquidPatchId, second: LiquidPatchId) -> bool {
        (self.from_patch == first && self.to_patch == second)
            || (self.from_patch == second && self.to_patch == first)
    }
}

/// Directly observed bacterial trajectory through a sequence of liquid patches.
#[derive(Debug, Clone, PartialEq)]
pub struct BacterialTransitObservation {
    pub id: BacterialTransitId,
    pub track: BacterialTrackId,
    pub source_patch: LiquidPatchId,
    pub target_patch: LiquidPatchId,
    /// Ordered patch sequence from source to target. It may traverse a link
    /// against its stored from-to orientation.
    pub path: Vec<LiquidPatchId>,
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub trajectory_observation: ObservationEvidenceId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighwayUnresolvedReason {
    MissingPatchInPath(LiquidPatchId),
    MissingPatchObservation(LiquidPatchId),
    MissingPatchReceipt(ObservationEvidenceId),
    WrongPatchReceiptSubject(ObservationEvidenceId),
    PatchReceiptReusedAcrossDistinctPatches {
        receipt: ObservationEvidenceId,
        first_patch: LiquidPatchId,
        second_patch: LiquidPatchId,
    },
    MissingLiquidPatchLink {
        from: LiquidPatchId,
        to: LiquidPatchId,
    },
    LiquidFilmDisconnected {
        from: LiquidPatchId,
        to: LiquidPatchId,
    },
    LiquidFilmStateUnreported {
        from: LiquidPatchId,
        to: LiquidPatchId,
    },
    MissingActiveHyphalSupport {
        from: LiquidPatchId,
        to: LiquidPatchId,
    },
    InactiveHyphalSupport(SegmentId),
    MissingHyphalObservation(SegmentId),
    MissingLiquidObservation(HyphalHighwayLinkId),
    MissingHyphalReceipt(ObservationEvidenceId),
    MissingLiquidReceipt(ObservationEvidenceId),
    WrongHyphalReceiptSubject(ObservationEvidenceId),
    WrongLiquidBridgeReceiptSubject(ObservationEvidenceId),
    HyphalReceiptTargetMismatch(ObservationEvidenceId),
    UnresolvedFilamentDomain(HyphalHighwayLinkId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyphalHighwayOutcome {
    QualifiedTrueFungus,
    QualifiedOomycete,
    QualifiedMixedFilamentDomains,
    Unresolved(HighwayUnresolvedReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyphalHighwayAssessment {
    pub outcome: HyphalHighwayOutcome,
    /// One admitted evidence link per adjacent pair in the bacterial path.
    pub supporting_links: Vec<HyphalHighwayLinkId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyphalHighwayError {
    InvalidPatchGeometry(LiquidPatchId),
    InvalidPatchArea(LiquidPatchId),
    InvalidFilmThickness(HyphalHighwayLinkId),
    SelfLink(LiquidPatchId),
    DuplicatePatch(LiquidPatchId),
    DuplicateLink(HyphalHighwayLinkId),
    MissingLinkPatch(LiquidPatchId),
    InvalidTransitPath,
    InvalidTransitTime,
    DuplicateObservationReceipt(ObservationEvidenceId),
    MissingBacterialTrajectoryReceipt(ObservationEvidenceId),
    InvalidObservationReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    WrongBacterialTrajectorySubject(ObservationEvidenceId),
}

impl fmt::Display for HyphalHighwayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPatchGeometry(id) => {
                write!(f, "liquid patch {:?} has non-finite coordinates", id)
            }
            Self::InvalidPatchArea(id) => {
                write!(f, "liquid patch {:?} area must be finite and positive", id)
            }
            Self::InvalidFilmThickness(id) => {
                write!(f, "hyphal highway link {:?} has invalid film thickness", id)
            }
            Self::SelfLink(id) => write!(f, "liquid patch {:?} cannot link to itself", id),
            Self::DuplicatePatch(id) => write!(f, "duplicate liquid patch {:?}", id),
            Self::DuplicateLink(id) => write!(f, "duplicate hyphal highway link {:?}", id),
            Self::MissingLinkPatch(id) => {
                write!(f, "hyphal highway link references missing patch {:?}", id)
            }
            Self::InvalidTransitPath => {
                write!(
                    f,
                    "bacterial transit path must contain matching source and target patches"
                )
            }
            Self::InvalidTransitTime => {
                write!(
                    f,
                    "bacterial transit times must be finite, non-negative and ordered"
                )
            }
            Self::DuplicateObservationReceipt(id) => {
                write!(f, "duplicate observation receipt {:?}", id)
            }
            Self::MissingBacterialTrajectoryReceipt(id) => {
                write!(f, "missing bacterial trajectory receipt {:?}", id)
            }
            Self::InvalidObservationReceipt { id, source } => {
                write!(f, "invalid observation receipt {:?}: {:?}", id, source)
            }
            Self::WrongBacterialTrajectorySubject(id) => {
                write!(f, "receipt {:?} is not a bacterial trajectory", id)
            }
        }
    }
}

impl std::error::Error for HyphalHighwayError {}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HyphalHighwayNetwork {
    patches: BTreeMap<LiquidPatchId, LiquidPatch>,
    links: BTreeMap<HyphalHighwayLinkId, HyphalHighwayLink>,
}

impl HyphalHighwayNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_patch(&mut self, patch: LiquidPatch) -> Result<(), HyphalHighwayError> {
        patch.validate()?;
        if self.patches.contains_key(&patch.id) {
            return Err(HyphalHighwayError::DuplicatePatch(patch.id));
        }
        self.patches.insert(patch.id, patch);
        Ok(())
    }

    pub fn add_link(&mut self, link: HyphalHighwayLink) -> Result<(), HyphalHighwayError> {
        link.validate()?;
        if self.links.contains_key(&link.id) {
            return Err(HyphalHighwayError::DuplicateLink(link.id));
        }
        for patch in [link.from_patch, link.to_patch] {
            if !self.patches.contains_key(&patch) {
                return Err(HyphalHighwayError::MissingLinkPatch(patch));
            }
        }
        self.links.insert(link.id, link);
        Ok(())
    }

    pub fn patch(&self, id: LiquidPatchId) -> Option<&LiquidPatch> {
        self.patches.get(&id)
    }

    pub fn links(&self) -> impl Iterator<Item = &HyphalHighwayLink> {
        self.links.values()
    }

    /// Assess one observed bacterial transit against a separately typed graph
    /// of hyphal support and liquid-film connectivity.
    ///
    /// The route is qualified only if every path hop has:
    /// - a measured connected liquid-film state;
    /// - an active segment in the canonical hyphal graph;
    /// - a source receipt typed as HyphalSegmentTrace and bound to that segment;
    /// - a source receipt typed as LiquidFilmPatch;
    /// - a direct BacterialTrajectory receipt for the observed track.
    ///
    /// Missing or negative evidence returns Unresolved, not a guessed route.
    pub fn assess_transit(
        &self,
        graph: &HyphalGraph,
        transit: &BacterialTransitObservation,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<HyphalHighwayAssessment, HyphalHighwayError> {
        if transit.path.len() < 2
            || transit.path.first().copied() != Some(transit.source_patch)
            || transit.path.last().copied() != Some(transit.target_patch)
            || transit.source_patch == transit.target_patch
        {
            return Err(HyphalHighwayError::InvalidTransitPath);
        }
        if !transit.start_time_s.is_finite()
            || !transit.end_time_s.is_finite()
            || transit.start_time_s < 0.0
            || transit.end_time_s < transit.start_time_s
        {
            return Err(HyphalHighwayError::InvalidTransitTime);
        }

        let receipt_map = receipt_index(receipts)?;
        let Some(trajectory_receipt) = receipt_map.get(&transit.trajectory_observation) else {
            return Err(HyphalHighwayError::MissingBacterialTrajectoryReceipt(
                transit.trajectory_observation,
            ));
        };
        trajectory_receipt.validate().map_err(|source| {
            HyphalHighwayError::InvalidObservationReceipt {
                id: transit.trajectory_observation,
                source,
            }
        })?;
        if trajectory_receipt.subject != ObservationSubject::BacterialTrajectory {
            return Err(HyphalHighwayError::WrongBacterialTrajectorySubject(
                transit.trajectory_observation,
            ));
        }

        for patch in &transit.path {
            if !self.patches.contains_key(patch) {
                return Ok(HyphalHighwayAssessment {
                    outcome: HyphalHighwayOutcome::Unresolved(
                        HighwayUnresolvedReason::MissingPatchInPath(*patch),
                    ),
                    supporting_links: Vec::new(),
                });
            }
        }

        let mut supporting_links = Vec::with_capacity(transit.path.len() - 1);
        let mut saw_fungus = false;
        let mut saw_oomycete = false;
        let mut patch_receipt_owners = BTreeMap::<ObservationEvidenceId, LiquidPatchId>::new();

        for hop in transit.path.windows(2) {
            let from = hop[0];
            let to = hop[1];

            // Each endpoint patch is independently source-linked. A bridge
            // observation cannot silently stand in for both patch identities.
            for patch_id in [from, to] {
                let patch = self
                    .patches
                    .get(&patch_id)
                    .expect("every path patch was checked above");
                let Some(observation_id) = patch.observation else {
                    return Ok(unresolved(
                        HighwayUnresolvedReason::MissingPatchObservation(patch_id),
                    ));
                };
                if let Some(first_patch) = patch_receipt_owners.insert(observation_id, patch_id)
                    && first_patch != patch_id
                {
                    return Ok(unresolved(
                        HighwayUnresolvedReason::PatchReceiptReusedAcrossDistinctPatches {
                            receipt: observation_id,
                            first_patch,
                            second_patch: patch_id,
                        },
                    ));
                }
                let Some(patch_receipt) = receipt_map.get(&observation_id) else {
                    return Ok(unresolved(HighwayUnresolvedReason::MissingPatchReceipt(
                        observation_id,
                    )));
                };
                patch_receipt.validate().map_err(|source| {
                    HyphalHighwayError::InvalidObservationReceipt {
                        id: observation_id,
                        source,
                    }
                })?;
                if patch_receipt.subject != ObservationSubject::LiquidFilmPatch {
                    return Ok(unresolved(
                        HighwayUnresolvedReason::WrongPatchReceiptSubject(observation_id),
                    ));
                }
            }

            let candidates: Vec<_> = self
                .links
                .values()
                .filter(|link| link.connects(from, to))
                .collect();

            if candidates.is_empty() {
                return Ok(unresolved(
                    HighwayUnresolvedReason::MissingLiquidPatchLink { from, to },
                ));
            }

            let mut best_reason = HighwayUnresolvedReason::LiquidFilmStateUnreported { from, to };
            let mut qualified = None;
            let mut saw_disconnected = false;
            let mut saw_unreported = false;
            let mut saw_connected = false;

            for link in candidates {
                match link.liquid_state {
                    LiquidFilmState::Disconnected => {
                        saw_disconnected = true;
                        continue;
                    }
                    LiquidFilmState::Unreported => {
                        saw_unreported = true;
                        continue;
                    }
                    LiquidFilmState::Connected => saw_connected = true,
                }

                let Some(segment) = graph.segment(link.hyphal_segment) else {
                    best_reason = HighwayUnresolvedReason::MissingActiveHyphalSupport { from, to };
                    continue;
                };
                if segment.state != HyphalSegmentState::Active {
                    best_reason =
                        HighwayUnresolvedReason::InactiveHyphalSupport(link.hyphal_segment);
                    continue;
                }
                if link.filament_domain == FilamentDomain::Unresolved {
                    best_reason = HighwayUnresolvedReason::UnresolvedFilamentDomain(link.id);
                    continue;
                }
                let Some(hyphal_id) = link.hyphal_observation else {
                    best_reason =
                        HighwayUnresolvedReason::MissingHyphalObservation(link.hyphal_segment);
                    continue;
                };
                let Some(liquid_id) = link.liquid_observation else {
                    best_reason = HighwayUnresolvedReason::MissingLiquidObservation(link.id);
                    continue;
                };

                let Some(hyphal_receipt) = receipt_map.get(&hyphal_id) else {
                    best_reason = HighwayUnresolvedReason::MissingHyphalReceipt(hyphal_id);
                    continue;
                };
                hyphal_receipt.validate().map_err(|source| {
                    HyphalHighwayError::InvalidObservationReceipt {
                        id: hyphal_id,
                        source,
                    }
                })?;
                if hyphal_receipt.subject != ObservationSubject::HyphalSegmentTrace {
                    best_reason = HighwayUnresolvedReason::WrongHyphalReceiptSubject(hyphal_id);
                    continue;
                }
                if hyphal_receipt.graph_target
                    != Some(EvidenceGraphTarget::Segment(link.hyphal_segment))
                {
                    best_reason = HighwayUnresolvedReason::HyphalReceiptTargetMismatch(hyphal_id);
                    continue;
                }

                let Some(liquid_receipt) = receipt_map.get(&liquid_id) else {
                    best_reason = HighwayUnresolvedReason::MissingLiquidReceipt(liquid_id);
                    continue;
                };
                liquid_receipt.validate().map_err(|source| {
                    HyphalHighwayError::InvalidObservationReceipt {
                        id: liquid_id,
                        source,
                    }
                })?;
                if liquid_receipt.subject != ObservationSubject::LiquidFilmBridge {
                    best_reason =
                        HighwayUnresolvedReason::WrongLiquidBridgeReceiptSubject(liquid_id);
                    continue;
                }

                qualified = Some((link.id, link.filament_domain));
                break;
            }

            let Some((link_id, filament_domain)) = qualified else {
                if !saw_connected && saw_disconnected {
                    return Ok(unresolved(
                        HighwayUnresolvedReason::LiquidFilmDisconnected { from, to },
                    ));
                }
                if !saw_connected && saw_unreported {
                    return Ok(unresolved(
                        HighwayUnresolvedReason::LiquidFilmStateUnreported { from, to },
                    ));
                }
                return Ok(unresolved(best_reason));
            };

            supporting_links.push(link_id);
            match filament_domain {
                FilamentDomain::TrueFungus => saw_fungus = true,
                FilamentDomain::Oomycete => saw_oomycete = true,
                FilamentDomain::Unresolved => unreachable!("unresolved domain is rejected above"),
            }
        }

        let outcome = match (saw_fungus, saw_oomycete) {
            (true, true) => HyphalHighwayOutcome::QualifiedMixedFilamentDomains,
            (true, false) => HyphalHighwayOutcome::QualifiedTrueFungus,
            (false, true) => HyphalHighwayOutcome::QualifiedOomycete,
            (false, false) => {
                return Ok(unresolved(
                    HighwayUnresolvedReason::UnresolvedFilamentDomain(
                        *supporting_links.first().unwrap_or(&HyphalHighwayLinkId(0)),
                    ),
                ));
            }
        };

        Ok(HyphalHighwayAssessment {
            outcome,
            supporting_links,
        })
    }
}

fn unresolved(reason: HighwayUnresolvedReason) -> HyphalHighwayAssessment {
    HyphalHighwayAssessment {
        outcome: HyphalHighwayOutcome::Unresolved(reason),
        supporting_links: Vec::new(),
    }
}

fn receipt_index(
    receipts: &[ObservationEvidenceReceipt],
) -> Result<BTreeMap<ObservationEvidenceId, &ObservationEvidenceReceipt>, HyphalHighwayError> {
    let mut index = BTreeMap::new();
    for receipt in receipts {
        if index.insert(receipt.id, receipt).is_some() {
            return Err(HyphalHighwayError::DuplicateObservationReceipt(receipt.id));
        }
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyphal_graph::{HyphalNode, HyphalNodeKind, HyphalSegment, NodeId, Position3Mm};
    use crate::observation_evidence::{
        EvidenceDisposition, ImageFrameIndex, ImageFrameReference, ImageRoiBoundsPx,
        ObservationRoiId, PhysicalSamplingCalibration, Sha256Digest,
    };

    fn receipt(
        id: u64,
        subject: ObservationSubject,
        graph_target: Option<EvidenceGraphTarget>,
    ) -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(id),
            source_file_sha256: Sha256Digest([1; 32]),
            frame: ImageFrameReference {
                frame: ImageFrameIndex(1),
                slice: None,
                timestamp_s: 10.0,
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
            processor_name: "hyphal-highway-test".to_owned(),
            processor_revision: "v0.1.0".to_owned(),
            processor_parameters_sha256: Sha256Digest([2; 32]),
            subject,
            graph_target,
            graph_event: None,
            uncertainty: None,
            disposition: EvidenceDisposition::MachineGenerated,
        }
    }

    fn graph_with_segment_state(state: HyphalSegmentState) -> HyphalGraph {
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
                id: SegmentId(7),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 1.0,
                diameter_mm: Some(0.02),
                state,
            })
            .unwrap();
        graph
    }

    fn graph() -> HyphalGraph {
        graph_with_segment_state(HyphalSegmentState::Active)
    }

    fn fixture(
        domain: FilamentDomain,
        state: LiquidFilmState,
    ) -> (
        HyphalHighwayNetwork,
        BacterialTransitObservation,
        Vec<ObservationEvidenceReceipt>,
    ) {
        let mut network = HyphalHighwayNetwork::new();
        network
            .add_patch(LiquidPatch {
                id: LiquidPatchId(1),
                centroid_x_um: 0.0,
                centroid_y_um: 0.0,
                segmented_area_um2: Some(100.0),
                observation: Some(ObservationEvidenceId(20)),
            })
            .unwrap();
        network
            .add_patch(LiquidPatch {
                id: LiquidPatchId(2),
                centroid_x_um: 100.0,
                centroid_y_um: 0.0,
                segmented_area_um2: Some(100.0),
                observation: Some(ObservationEvidenceId(21)),
            })
            .unwrap();
        network
            .add_link(HyphalHighwayLink {
                id: HyphalHighwayLinkId(50),
                from_patch: LiquidPatchId(1),
                to_patch: LiquidPatchId(2),
                filament_domain: domain,
                hyphal_segment: SegmentId(7),
                liquid_state: state,
                film_thickness_um: Some(3.0),
                hyphal_observation: Some(ObservationEvidenceId(10)),
                liquid_observation: Some(ObservationEvidenceId(11)),
            })
            .unwrap();

        let transit = BacterialTransitObservation {
            id: BacterialTransitId(80),
            track: BacterialTrackId(90),
            source_patch: LiquidPatchId(1),
            target_patch: LiquidPatchId(2),
            path: vec![LiquidPatchId(1), LiquidPatchId(2)],
            start_time_s: 1.0,
            end_time_s: 4.0,
            trajectory_observation: ObservationEvidenceId(12),
        };
        let receipts = vec![
            receipt(
                10,
                ObservationSubject::HyphalSegmentTrace,
                Some(EvidenceGraphTarget::Segment(SegmentId(7))),
            ),
            receipt(11, ObservationSubject::LiquidFilmBridge, None),
            receipt(12, ObservationSubject::BacterialTrajectory, None),
            receipt(20, ObservationSubject::LiquidFilmPatch, None),
            receipt(21, ObservationSubject::LiquidFilmPatch, None),
        ];
        (network, transit, receipts)
    }

    #[test]
    fn source_linked_fungal_hyphal_liquid_route_qualifies() {
        let (network, transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        let assessment = network
            .assess_transit(&graph(), &transit, &receipts)
            .unwrap();
        assert_eq!(
            assessment.outcome,
            HyphalHighwayOutcome::QualifiedTrueFungus
        );
        assert_eq!(assessment.supporting_links, vec![HyphalHighwayLinkId(50)]);
    }

    #[test]
    fn oomycete_route_is_not_misclassified_as_true_fungus() {
        let (network, transit, receipts) =
            fixture(FilamentDomain::Oomycete, LiquidFilmState::Connected);
        let assessment = network
            .assess_transit(&graph(), &transit, &receipts)
            .unwrap();
        assert_eq!(assessment.outcome, HyphalHighwayOutcome::QualifiedOomycete);
    }

    #[test]
    fn mixed_fungal_and_oomycete_route_has_explicit_mixed_class() {
        let mut network = HyphalHighwayNetwork::new();
        for id in 1..=3 {
            network
                .add_patch(LiquidPatch {
                    id: LiquidPatchId(id),
                    centroid_x_um: id as f64,
                    centroid_y_um: 0.0,
                    segmented_area_um2: Some(10.0),
                    observation: Some(ObservationEvidenceId(200 + id)),
                })
                .unwrap();
        }
        for (id, from, to, domain, segment_id, hyphal_id, liquid_id) in [
            (1, 1, 2, FilamentDomain::TrueFungus, 7, 10, 11),
            (2, 2, 3, FilamentDomain::Oomycete, 8, 13, 14),
        ] {
            network
                .add_link(HyphalHighwayLink {
                    id: HyphalHighwayLinkId(id),
                    from_patch: LiquidPatchId(from),
                    to_patch: LiquidPatchId(to),
                    filament_domain: domain,
                    hyphal_segment: SegmentId(segment_id),
                    liquid_state: LiquidFilmState::Connected,
                    film_thickness_um: None,
                    hyphal_observation: Some(ObservationEvidenceId(hyphal_id)),
                    liquid_observation: Some(ObservationEvidenceId(liquid_id)),
                })
                .unwrap();
        }
        let mut graph = graph();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(8),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 1.0,
                diameter_mm: Some(0.02),
                state: HyphalSegmentState::Active,
            })
            .unwrap();
        let transit = BacterialTransitObservation {
            id: BacterialTransitId(81),
            track: BacterialTrackId(91),
            source_patch: LiquidPatchId(1),
            target_patch: LiquidPatchId(3),
            path: vec![LiquidPatchId(1), LiquidPatchId(2), LiquidPatchId(3)],
            start_time_s: 0.0,
            end_time_s: 3.0,
            trajectory_observation: ObservationEvidenceId(12),
        };
        let receipts = vec![
            receipt(
                10,
                ObservationSubject::HyphalSegmentTrace,
                Some(EvidenceGraphTarget::Segment(SegmentId(7))),
            ),
            receipt(11, ObservationSubject::LiquidFilmBridge, None),
            receipt(
                13,
                ObservationSubject::HyphalSegmentTrace,
                Some(EvidenceGraphTarget::Segment(SegmentId(8))),
            ),
            receipt(14, ObservationSubject::LiquidFilmBridge, None),
            receipt(12, ObservationSubject::BacterialTrajectory, None),
            receipt(201, ObservationSubject::LiquidFilmPatch, None),
            receipt(202, ObservationSubject::LiquidFilmPatch, None),
            receipt(203, ObservationSubject::LiquidFilmPatch, None),
        ];
        let result = network.assess_transit(&graph, &transit, &receipts).unwrap();
        assert_eq!(
            result.outcome,
            HyphalHighwayOutcome::QualifiedMixedFilamentDomains
        );
        assert_eq!(
            result.supporting_links,
            vec![HyphalHighwayLinkId(1), HyphalHighwayLinkId(2)]
        );
    }

    #[test]
    fn disconnected_liquid_film_never_qualifies_a_highway() {
        let (network, transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Disconnected);
        let assessment = network
            .assess_transit(&graph(), &transit, &receipts)
            .unwrap();
        assert_eq!(
            assessment.outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::LiquidFilmDisconnected {
                from: LiquidPatchId(1),
                to: LiquidPatchId(2),
            })
        );
        assert!(assessment.supporting_links.is_empty());
    }

    #[test]
    fn missing_source_receipt_for_endpoint_patch_prevents_qualification() {
        let (mut network, transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        network
            .patches
            .get_mut(&LiquidPatchId(1))
            .unwrap()
            .observation = None;
        assert_eq!(
            network
                .assess_transit(&graph(), &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::MissingPatchObservation(
                LiquidPatchId(1)
            ))
        );
    }

    #[test]
    fn one_receipt_cannot_stand_for_two_distinct_liquid_patches() {
        let (mut network, transit, mut receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        network
            .patches
            .get_mut(&LiquidPatchId(2))
            .unwrap()
            .observation = Some(ObservationEvidenceId(20));
        receipts.retain(|item| item.id != ObservationEvidenceId(21));
        assert_eq!(
            network
                .assess_transit(&graph(), &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(
                HighwayUnresolvedReason::PatchReceiptReusedAcrossDistinctPatches {
                    receipt: ObservationEvidenceId(20),
                    first_patch: LiquidPatchId(1),
                    second_patch: LiquidPatchId(2),
                }
            )
        );
    }

    #[test]
    fn wrong_endpoint_patch_receipt_subject_prevents_qualification() {
        let (network, transit, mut receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        receipts
            .iter_mut()
            .find(|item| item.id == ObservationEvidenceId(20))
            .unwrap()
            .subject = ObservationSubject::FluorescenceRoi;
        assert_eq!(
            network
                .assess_transit(&graph(), &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::WrongPatchReceiptSubject(
                ObservationEvidenceId(20)
            ))
        );
    }

    #[test]
    fn unreported_liquid_state_never_qualifies_a_highway() {
        let (network, transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Unreported);
        assert_eq!(
            network
                .assess_transit(&graph(), &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::LiquidFilmStateUnreported {
                from: LiquidPatchId(1),
                to: LiquidPatchId(2),
            })
        );
    }

    #[test]
    fn absent_liquid_receipt_prevents_qualification() {
        let (mut network, transit, mut receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        let link = network.links.get_mut(&HyphalHighwayLinkId(50)).unwrap();
        link.liquid_observation = None;
        receipts.retain(|item| item.id != ObservationEvidenceId(11));
        assert_eq!(
            network
                .assess_transit(&graph(), &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::MissingLiquidObservation(
                HyphalHighwayLinkId(50)
            ))
        );
    }

    #[test]
    fn inactive_or_absent_hyphal_segment_prevents_qualification() {
        let (network, transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        let inactive_graph = graph_with_segment_state(HyphalSegmentState::Inactive);
        assert_eq!(
            network
                .assess_transit(&inactive_graph, &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::InactiveHyphalSupport(
                SegmentId(7)
            ))
        );
    }

    #[test]
    fn reverse_direction_uses_the_same_observed_liquid_link() {
        let (network, mut transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        transit.source_patch = LiquidPatchId(2);
        transit.target_patch = LiquidPatchId(1);
        transit.path = vec![LiquidPatchId(2), LiquidPatchId(1)];
        assert_eq!(
            network
                .assess_transit(&graph(), &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::QualifiedTrueFungus
        );
    }

    #[test]
    fn wrong_or_missing_bacterial_trajectory_receipt_is_rejected() {
        let (network, transit, mut receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        receipts.retain(|item| item.id != ObservationEvidenceId(12));
        assert_eq!(
            network.assess_transit(&graph(), &transit, &receipts),
            Err(HyphalHighwayError::MissingBacterialTrajectoryReceipt(
                ObservationEvidenceId(12)
            ))
        );

        receipts.push(receipt(12, ObservationSubject::FluorescenceRoi, None));
        assert_eq!(
            network.assess_transit(&graph(), &transit, &receipts),
            Err(HyphalHighwayError::WrongBacterialTrajectorySubject(
                ObservationEvidenceId(12)
            ))
        );
    }

    #[test]
    fn bacterial_route_cannot_use_visual_overlap_without_a_hyphal_graph_segment() {
        let (network, transit, receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        let empty_graph = HyphalGraph::new();
        assert_eq!(
            network
                .assess_transit(&empty_graph, &transit, &receipts)
                .unwrap()
                .outcome,
            HyphalHighwayOutcome::Unresolved(HighwayUnresolvedReason::MissingActiveHyphalSupport {
                from: LiquidPatchId(1),
                to: LiquidPatchId(2),
            })
        );
    }

    #[test]
    fn duplicate_receipts_and_invalid_timestamps_are_rejected() {
        let (network, transit, mut receipts) =
            fixture(FilamentDomain::TrueFungus, LiquidFilmState::Connected);
        receipts.push(receipt(12, ObservationSubject::BacterialTrajectory, None));
        assert_eq!(
            network.assess_transit(&graph(), &transit, &receipts),
            Err(HyphalHighwayError::DuplicateObservationReceipt(
                ObservationEvidenceId(12)
            ))
        );

        let mut invalid = transit;
        invalid.end_time_s = f64::NAN;
        assert_eq!(
            network.assess_transit(&graph(), &invalid, &receipts[..3]),
            Err(HyphalHighwayError::InvalidTransitTime)
        );
    }

    #[test]
    fn patch_and_link_insertions_reject_invalid_or_missing_identity() {
        let mut network = HyphalHighwayNetwork::new();
        assert_eq!(
            network.add_patch(LiquidPatch {
                id: LiquidPatchId(1),
                centroid_x_um: f64::NAN,
                centroid_y_um: 0.0,
                segmented_area_um2: None,
                observation: None,
            }),
            Err(HyphalHighwayError::InvalidPatchGeometry(LiquidPatchId(1)))
        );
        network
            .add_patch(LiquidPatch {
                id: LiquidPatchId(1),
                centroid_x_um: 0.0,
                centroid_y_um: 0.0,
                segmented_area_um2: None,
                observation: Some(ObservationEvidenceId(22)),
            })
            .unwrap();
        assert_eq!(
            network.add_link(HyphalHighwayLink {
                id: HyphalHighwayLinkId(1),
                from_patch: LiquidPatchId(1),
                to_patch: LiquidPatchId(2),
                filament_domain: FilamentDomain::TrueFungus,
                hyphal_segment: SegmentId(7),
                liquid_state: LiquidFilmState::Connected,
                film_thickness_um: Some(0.0),
                hyphal_observation: None,
                liquid_observation: None,
            }),
            Err(HyphalHighwayError::InvalidFilmThickness(
                HyphalHighwayLinkId(1)
            ))
        );
        assert_eq!(
            network.add_link(HyphalHighwayLink {
                id: HyphalHighwayLinkId(1),
                from_patch: LiquidPatchId(1),
                to_patch: LiquidPatchId(2),
                filament_domain: FilamentDomain::TrueFungus,
                hyphal_segment: SegmentId(7),
                liquid_state: LiquidFilmState::Connected,
                film_thickness_um: None,
                hyphal_observation: None,
                liquid_observation: None,
            }),
            Err(HyphalHighwayError::MissingLinkPatch(LiquidPatchId(2)))
        );
    }
}
