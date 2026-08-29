// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bind ARTIST-EYE-v1A spatial evidence to one fail-closed four-ghost episode.
//!
//! This layer keeps perception evidence separate from artistic choice. It
//! verifies that each multi-scale observation came from the exact GPU capture
//! assigned to that candidate and computes proposal-minus-baseline spatial
//! consequences without producing an aggregate preference score.

use std::collections::{BTreeMap, BTreeSet};

use crate::art_eye::{
    ArtistEyeConsequenceEvidence, ArtistEyeError, ArtistEyeObservation,
};
use crate::art_ghost_loop::{FourGhostError, FourGhostRenderSet, GhostCandidateKind};
use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, PartialEq)]
pub struct GhostArtistEyeEvidence {
    pub candidate_id: String,
    pub observation: ArtistEyeObservation,
    /// `None` for abstention baseline; proposals carry candidate - baseline
    /// multi-scale spatial consequence evidence.
    pub consequence: Option<ArtistEyeConsequenceEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourGhostArtistEyeEvidenceSet {
    pub base_revision: String,
    pub frame: StudioFrame,
    pub evidence: Vec<GhostArtistEyeEvidence>,
}

impl FourGhostArtistEyeEvidenceSet {
    pub fn build(
        renders: &FourGhostRenderSet,
        observations: Vec<(String, ArtistEyeObservation)>,
    ) -> Result<Self, FourGhostArtistEyeError> {
        renders
            .validate()
            .map_err(FourGhostArtistEyeError::Ghost)?;
        if observations.len() != 4 {
            return Err(FourGhostArtistEyeError::RequiresExactlyFourObservations);
        }

        let mut by_id = BTreeMap::new();
        for (candidate_id, observation) in observations {
            if by_id.insert(candidate_id.clone(), observation).is_some() {
                return Err(FourGhostArtistEyeError::DuplicateObservation(candidate_id));
            }
        }

        let expected_ids: BTreeSet<&str> = renders
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect();
        let observed_ids: BTreeSet<&str> = by_id.keys().map(String::as_str).collect();
        if expected_ids != observed_ids {
            return Err(FourGhostArtistEyeError::CandidateCoverageMismatch);
        }

        for candidate in &renders.candidates {
            let observation = by_id
                .get(&candidate.candidate_id)
                .ok_or_else(|| {
                    FourGhostArtistEyeError::MissingObservation(candidate.candidate_id.clone())
                })?;
            let receipt = &candidate.capture.receipt;
            if observation.capture_id != receipt.request.capture_id
                || observation.revision_id != renders.base_revision
                || observation.frame != renders.frame
                || observation.rendered_scene_hash != receipt.observed_scene_hash
            {
                return Err(FourGhostArtistEyeError::ObservationMisalignment(
                    candidate.candidate_id.clone(),
                ));
            }
        }

        let baseline_candidate = renders
            .baseline()
            .ok_or(FourGhostArtistEyeError::MissingBaseline)?;
        let baseline = by_id
            .get(&baseline_candidate.candidate_id)
            .cloned()
            .ok_or(FourGhostArtistEyeError::MissingBaseline)?;

        let mut evidence = Vec::with_capacity(4);
        for candidate in &renders.candidates {
            let observation = by_id
                .remove(&candidate.candidate_id)
                .expect("candidate coverage validated above");
            let consequence = match &candidate.kind {
                GhostCandidateKind::AbstentionBaseline => None,
                GhostCandidateKind::Proposal { .. } => Some(
                    ArtistEyeConsequenceEvidence::between(&baseline, &observation)
                        .map_err(FourGhostArtistEyeError::Eye)?,
                ),
            };
            evidence.push(GhostArtistEyeEvidence {
                candidate_id: candidate.candidate_id.clone(),
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
    ) -> Result<(), FourGhostArtistEyeError> {
        renders
            .validate()
            .map_err(FourGhostArtistEyeError::Ghost)?;
        if self.base_revision != renders.base_revision || self.frame != renders.frame {
            return Err(FourGhostArtistEyeError::SetMisalignment);
        }
        if self.evidence.len() != 4 {
            return Err(FourGhostArtistEyeError::RequiresExactlyFourObservations);
        }

        let mut seen = BTreeSet::new();
        for item in &self.evidence {
            if !seen.insert(item.candidate_id.as_str()) {
                return Err(FourGhostArtistEyeError::DuplicateObservation(
                    item.candidate_id.clone(),
                ));
            }
            let candidate = renders
                .candidate(&item.candidate_id)
                .ok_or_else(|| {
                    FourGhostArtistEyeError::MissingObservation(item.candidate_id.clone())
                })?;
            let receipt = &candidate.capture.receipt;
            if item.observation.capture_id != receipt.request.capture_id
                || item.observation.revision_id != renders.base_revision
                || item.observation.frame != renders.frame
                || item.observation.rendered_scene_hash != receipt.observed_scene_hash
            {
                return Err(FourGhostArtistEyeError::ObservationMisalignment(
                    item.candidate_id.clone(),
                ));
            }
            match &candidate.kind {
                GhostCandidateKind::AbstentionBaseline => {
                    if item.consequence.is_some() {
                        return Err(FourGhostArtistEyeError::BaselineHasConsequence);
                    }
                }
                GhostCandidateKind::Proposal { .. } => {
                    if item.consequence.is_none() {
                        return Err(FourGhostArtistEyeError::ProposalMissingConsequence(
                            item.candidate_id.clone(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn baseline(&self, renders: &FourGhostRenderSet) -> Option<&GhostArtistEyeEvidence> {
        let baseline_id = renders.baseline()?.candidate_id.as_str();
        self.evidence
            .iter()
            .find(|item| item.candidate_id == baseline_id)
    }

    pub fn candidate(&self, candidate_id: &str) -> Option<&GhostArtistEyeEvidence> {
        self.evidence
            .iter()
            .find(|item| item.candidate_id == candidate_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FourGhostArtistEyeError {
    Ghost(FourGhostError),
    Eye(ArtistEyeError),
    RequiresExactlyFourObservations,
    DuplicateObservation(String),
    MissingObservation(String),
    CandidateCoverageMismatch,
    ObservationMisalignment(String),
    MissingBaseline,
    BaselineHasConsequence,
    ProposalMissingConsequence(String),
    SetMisalignment,
}

impl std::fmt::Display for FourGhostArtistEyeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ghost(error) => write!(f, "four-ghost error: {error}"),
            Self::Eye(error) => write!(f, "artist-eye error: {error}"),
            Self::RequiresExactlyFourObservations => {
                write!(f, "artist-eye four-ghost set requires exactly four observations")
            }
            Self::DuplicateObservation(id) => write!(f, "duplicate artist-eye observation {id}"),
            Self::MissingObservation(id) => write!(f, "missing artist-eye observation {id}"),
            Self::CandidateCoverageMismatch => {
                write!(f, "artist-eye candidate coverage does not match render set")
            }
            Self::ObservationMisalignment(id) => {
                write!(f, "artist-eye observation {id} is not bound to its render receipt")
            }
            Self::MissingBaseline => write!(f, "artist-eye evidence set is missing baseline"),
            Self::BaselineHasConsequence => {
                write!(f, "artist-eye abstention baseline cannot carry a consequence delta")
            }
            Self::ProposalMissingConsequence(id) => {
                write!(f, "artist-eye proposal {id} is missing consequence evidence")
            }
            Self::SetMisalignment => write!(f, "artist-eye evidence set is revision/frame misaligned"),
        }
    }
}

impl std::error::Error for FourGhostArtistEyeError {}
