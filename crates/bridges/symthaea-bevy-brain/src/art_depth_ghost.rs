// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bind ARTIST-EYE-v1B depth evidence to one four-ghost episode.
//!
//! Depth may be produced by a distinct render pass, so it has its own capture
//! receipt. This module requires that receipt to match the candidate's exact
//! revision, frame, rendered semantic scene hash, camera and resolution before
//! any proposal-minus-baseline depth consequence is computed.

use std::collections::{BTreeMap, BTreeSet};

use crate::art_capture::{ArtCaptureReceipt, ArtRenderChannel};
use crate::art_depth::{ArtistDepthConsequenceEvidence, ArtistDepthObservation};
use crate::art_ghost_loop::{FourGhostError, FourGhostRenderSet, GhostCandidateKind};
use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, PartialEq)]
pub struct GhostArtistDepthEvidence {
    pub candidate_id: String,
    pub depth_receipt: ArtCaptureReceipt,
    pub observation: ArtistDepthObservation,
    /// `None` for the abstention baseline; proposals carry candidate - baseline.
    pub consequence: Option<ArtistDepthConsequenceEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourGhostArtistDepthEvidenceSet {
    pub base_revision: String,
    pub frame: StudioFrame,
    pub evidence: Vec<GhostArtistDepthEvidence>,
}

impl FourGhostArtistDepthEvidenceSet {
    pub fn build(
        renders: &FourGhostRenderSet,
        observations: Vec<(String, ArtCaptureReceipt, ArtistDepthObservation)>,
    ) -> Result<Self, FourGhostArtistDepthError> {
        renders.validate().map_err(FourGhostArtistDepthError::Ghost)?;
        if observations.len() != 4 {
            return Err(FourGhostArtistDepthError::RequiresExactlyFourObservations);
        }

        let mut by_id = BTreeMap::new();
        for (candidate_id, receipt, observation) in observations {
            if by_id
                .insert(candidate_id.clone(), (receipt, observation))
                .is_some()
            {
                return Err(FourGhostArtistDepthError::DuplicateObservation(candidate_id));
            }
        }

        let expected_ids: BTreeSet<&str> = renders
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect();
        let observed_ids: BTreeSet<&str> = by_id.keys().map(String::as_str).collect();
        if expected_ids != observed_ids {
            return Err(FourGhostArtistDepthError::CandidateCoverageMismatch);
        }

        for candidate in &renders.candidates {
            let (depth_receipt, observation) = by_id
                .get(&candidate.candidate_id)
                .ok_or_else(|| {
                    FourGhostArtistDepthError::MissingObservation(candidate.candidate_id.clone())
                })?;
            validate_binding(renders, candidate, depth_receipt, observation)?;
        }

        let baseline_candidate = renders
            .baseline()
            .ok_or(FourGhostArtistDepthError::MissingBaseline)?;
        let baseline = by_id
            .get(&baseline_candidate.candidate_id)
            .map(|(_, observation)| observation.clone())
            .ok_or(FourGhostArtistDepthError::MissingBaseline)?;

        let mut evidence = Vec::with_capacity(4);
        for candidate in &renders.candidates {
            let (depth_receipt, observation) = by_id
                .remove(&candidate.candidate_id)
                .expect("candidate coverage validated above");
            let consequence = match &candidate.kind {
                GhostCandidateKind::AbstentionBaseline => None,
                GhostCandidateKind::Proposal { .. } => {
                    Some(ArtistDepthConsequenceEvidence::between(&baseline, &observation))
                }
            };
            evidence.push(GhostArtistDepthEvidence {
                candidate_id: candidate.candidate_id.clone(),
                depth_receipt,
                observation,
                consequence,
            });
        }

        let set = Self {
            base_revision: renders.base_revision.clone(),
            frame: renders.frame,
            evidence,
        };
        set.validate_against(renders)?;
        Ok(set)
    }

    pub fn validate_against(
        &self,
        renders: &FourGhostRenderSet,
    ) -> Result<(), FourGhostArtistDepthError> {
        renders.validate().map_err(FourGhostArtistDepthError::Ghost)?;
        if self.base_revision != renders.base_revision || self.frame != renders.frame {
            return Err(FourGhostArtistDepthError::SetMisalignment);
        }
        if self.evidence.len() != 4 {
            return Err(FourGhostArtistDepthError::RequiresExactlyFourObservations);
        }

        let mut seen = BTreeSet::new();
        for item in &self.evidence {
            if !seen.insert(item.candidate_id.as_str()) {
                return Err(FourGhostArtistDepthError::DuplicateObservation(
                    item.candidate_id.clone(),
                ));
            }
            let candidate = renders
                .candidate(&item.candidate_id)
                .ok_or_else(|| {
                    FourGhostArtistDepthError::MissingObservation(item.candidate_id.clone())
                })?;
            validate_binding(renders, candidate, &item.depth_receipt, &item.observation)?;
            match &candidate.kind {
                GhostCandidateKind::AbstentionBaseline => {
                    if item.consequence.is_some() {
                        return Err(FourGhostArtistDepthError::BaselineHasConsequence);
                    }
                }
                GhostCandidateKind::Proposal { .. } => {
                    if item.consequence.is_none() {
                        return Err(FourGhostArtistDepthError::ProposalMissingConsequence(
                            item.candidate_id.clone(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_binding(
    renders: &FourGhostRenderSet,
    candidate: &crate::art_ghost_loop::GhostRenderObservation,
    depth_receipt: &ArtCaptureReceipt,
    observation: &ArtistDepthObservation,
) -> Result<(), FourGhostArtistDepthError> {
    depth_receipt
        .validate_alignment()
        .map_err(|error| FourGhostArtistDepthError::DepthCapture(error.to_string()))?;
    if !depth_receipt.request.channels.contains(&ArtRenderChannel::Depth) {
        return Err(FourGhostArtistDepthError::DepthChannelMissing(
            candidate.candidate_id.clone(),
        ));
    }

    let color_receipt = &candidate.capture.receipt;
    if depth_receipt.observed_revision_id != renders.base_revision
        || depth_receipt.observed_frame != renders.frame
        || depth_receipt.observed_scene_hash != color_receipt.observed_scene_hash
        || depth_receipt.request.camera_stable_id.as_deref()
            != Some(renders.camera_stable_id.as_str())
        || depth_receipt.request.width != color_receipt.request.width
        || depth_receipt.request.height != color_receipt.request.height
    {
        return Err(FourGhostArtistDepthError::DepthCaptureMisalignment(
            candidate.candidate_id.clone(),
        ));
    }

    if observation.capture_id != depth_receipt.request.capture_id
        || observation.revision_id != depth_receipt.observed_revision_id
        || observation.frame != depth_receipt.observed_frame
        || observation.rendered_scene_hash != depth_receipt.observed_scene_hash
    {
        return Err(FourGhostArtistDepthError::ObservationMisalignment(
            candidate.candidate_id.clone(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FourGhostArtistDepthError {
    Ghost(FourGhostError),
    DepthCapture(String),
    RequiresExactlyFourObservations,
    DuplicateObservation(String),
    MissingObservation(String),
    CandidateCoverageMismatch,
    DepthChannelMissing(String),
    DepthCaptureMisalignment(String),
    ObservationMisalignment(String),
    MissingBaseline,
    BaselineHasConsequence,
    ProposalMissingConsequence(String),
    SetMisalignment,
}

impl std::fmt::Display for FourGhostArtistDepthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ghost(error) => write!(f, "four-ghost error: {error}"),
            Self::DepthCapture(error) => write!(f, "depth capture error: {error}"),
            Self::RequiresExactlyFourObservations => {
                write!(f, "depth four-ghost set requires exactly four observations")
            }
            Self::DuplicateObservation(id) => write!(f, "duplicate depth observation {id}"),
            Self::MissingObservation(id) => write!(f, "missing depth observation {id}"),
            Self::CandidateCoverageMismatch => {
                write!(f, "depth candidate coverage does not match render set")
            }
            Self::DepthChannelMissing(id) => write!(f, "depth candidate {id} has no depth channel"),
            Self::DepthCaptureMisalignment(id) => write!(
                f,
                "depth capture {id} is not aligned to the candidate render plane"
            ),
            Self::ObservationMisalignment(id) => {
                write!(f, "depth observation {id} does not match its depth receipt")
            }
            Self::MissingBaseline => write!(f, "depth evidence set is missing baseline"),
            Self::BaselineHasConsequence => {
                write!(f, "depth abstention baseline cannot carry a consequence delta")
            }
            Self::ProposalMissingConsequence(id) => {
                write!(f, "depth proposal {id} is missing consequence evidence")
            }
            Self::SetMisalignment => write!(f, "depth evidence set is revision/frame misaligned"),
        }
    }
}

impl std::error::Error for FourGhostArtistDepthError {}
