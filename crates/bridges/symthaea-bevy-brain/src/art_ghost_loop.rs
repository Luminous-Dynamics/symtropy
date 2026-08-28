// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Four-ghost artistic observation: baseline/abstention plus three proposal previews.
//!
//! Proposal ghosts are allowed to have different rendered scene hashes from the
//! committed baseline. Their evidence is instead bound to a shared *base* scene
//! hash, revision, frame, camera, and render fidelity. This distinction matters:
//! a useful counterfactual must actually depict a changed scene while remaining
//! causally attributable to one unchanged committed parent.

use std::collections::{BTreeMap, BTreeSet};

use crate::art_capture::ArtCapturePurpose;
use crate::art_observation::{FidelityTaggedCapture, RenderFidelity};
use crate::art_timeline::StudioFrame;
use crate::art_visual::{VisualConsequenceVector, VisualObservation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhostCandidateKind {
    AbstentionBaseline,
    Proposal {
        proposal_id: String,
        branch_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostRenderObservation {
    pub candidate_id: String,
    pub kind: GhostCandidateKind,
    /// Hash of the committed parent scene shared by all four candidates.
    pub base_scene_hash: String,
    /// Capture of either the committed baseline or a rendered preview scene.
    pub capture: FidelityTaggedCapture,
}

impl GhostRenderObservation {
    pub fn rendered_scene_hash(&self) -> &str {
        &self.capture.receipt.observed_scene_hash
    }

    fn validate_individual(&self) -> Result<(), FourGhostError> {
        if self.candidate_id.trim().is_empty() || self.base_scene_hash.trim().is_empty() {
            return Err(FourGhostError::MissingIdentity);
        }
        self.capture
            .validate()
            .map_err(|error| FourGhostError::Observation(error.to_string()))?;

        match &self.kind {
            GhostCandidateKind::AbstentionBaseline => {
                if self.capture.receipt.request.purpose != ArtCapturePurpose::CommittedObservation {
                    return Err(FourGhostError::BaselinePurposeMismatch);
                }
                if self.capture.receipt.observed_scene_hash != self.base_scene_hash {
                    return Err(FourGhostError::BaselineSceneMismatch);
                }
            }
            GhostCandidateKind::Proposal {
                proposal_id,
                branch_id,
            } => {
                if proposal_id.trim().is_empty() || branch_id.trim().is_empty() {
                    return Err(FourGhostError::MissingIdentity);
                }
                if self.capture.receipt.request.purpose != ArtCapturePurpose::CounterfactualPreview
                {
                    return Err(FourGhostError::ProposalPurposeMismatch);
                }
            }
        }
        Ok(())
    }
}

/// One committed baseline and exactly three rendered proposal ghosts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FourGhostRenderSet {
    pub base_revision: String,
    pub frame: StudioFrame,
    pub camera_stable_id: String,
    pub fidelity: RenderFidelity,
    pub candidates: Vec<GhostRenderObservation>,
}

impl FourGhostRenderSet {
    pub fn validate(&self) -> Result<(), FourGhostError> {
        if self.base_revision.trim().is_empty() || self.camera_stable_id.trim().is_empty() {
            return Err(FourGhostError::MissingIdentity);
        }
        self.fidelity
            .validate()
            .map_err(|error| FourGhostError::Observation(error.to_string()))?;
        if self.candidates.len() != 4 {
            return Err(FourGhostError::RequiresExactlyFourCandidates);
        }

        let mut candidate_ids = BTreeSet::new();
        let mut proposal_ids = BTreeSet::new();
        let mut branch_ids = BTreeSet::new();
        let mut base_hash: Option<&str> = None;
        let mut baselines = 0usize;

        for candidate in &self.candidates {
            candidate.validate_individual()?;
            if !candidate_ids.insert(candidate.candidate_id.as_str()) {
                return Err(FourGhostError::DuplicateCandidate(candidate.candidate_id.clone()));
            }
            if base_hash.is_some_and(|hash| hash != candidate.base_scene_hash) {
                return Err(FourGhostError::CrossBaseScene);
            }
            base_hash = Some(candidate.base_scene_hash.as_str());

            let receipt = &candidate.capture.receipt;
            if receipt.observed_revision_id != self.base_revision
                || receipt.observed_frame != self.frame
                || candidate.capture.fidelity != self.fidelity
                || receipt.request.camera_stable_id.as_deref()
                    != Some(self.camera_stable_id.as_str())
            {
                return Err(FourGhostError::CrossObservationPlane);
            }

            match &candidate.kind {
                GhostCandidateKind::AbstentionBaseline => baselines += 1,
                GhostCandidateKind::Proposal {
                    proposal_id,
                    branch_id,
                } => {
                    if !proposal_ids.insert(proposal_id.as_str()) {
                        return Err(FourGhostError::DuplicateProposal(proposal_id.clone()));
                    }
                    if !branch_ids.insert(branch_id.as_str()) {
                        return Err(FourGhostError::DuplicateBranch(branch_id.clone()));
                    }
                }
            }
        }

        if baselines != 1 || proposal_ids.len() != 3 || branch_ids.len() != 3 {
            return Err(FourGhostError::RequiresOneBaselineThreeProposals);
        }
        Ok(())
    }

    pub fn baseline(&self) -> Option<&GhostRenderObservation> {
        self.candidates.iter().find(|candidate| {
            matches!(candidate.kind, GhostCandidateKind::AbstentionBaseline)
        })
    }

    pub fn candidate(&self, id: &str) -> Option<&GhostRenderObservation> {
        self.candidates.iter().find(|candidate| candidate.candidate_id == id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GhostVisualEvidence {
    pub candidate_id: String,
    pub observation: VisualObservation,
    /// `None` for the baseline; proposal candidates carry candidate - baseline.
    pub consequence: Option<VisualConsequenceVector>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourGhostVisualEvidenceSet {
    pub base_revision: String,
    pub frame: StudioFrame,
    pub evidence: Vec<GhostVisualEvidence>,
}

impl FourGhostVisualEvidenceSet {
    pub fn build(
        renders: &FourGhostRenderSet,
        observations: Vec<(String, VisualObservation)>,
    ) -> Result<Self, FourGhostError> {
        renders.validate()?;
        if observations.len() != 4 {
            return Err(FourGhostError::RequiresExactlyFourVisualObservations);
        }

        let mut by_id = BTreeMap::new();
        for (candidate_id, observation) in observations {
            if by_id.insert(candidate_id.clone(), observation).is_some() {
                return Err(FourGhostError::DuplicateVisualObservation(candidate_id));
            }
        }

        for candidate in &renders.candidates {
            let observation = by_id
                .get(&candidate.candidate_id)
                .ok_or_else(|| FourGhostError::MissingVisualObservation(
                    candidate.candidate_id.clone(),
                ))?;
            if observation.capture_id != candidate.capture.receipt.request.capture_id
                || observation.revision_id != renders.base_revision
                || observation.frame != renders.frame
                || observation.rendered_scene_hash != candidate.rendered_scene_hash()
            {
                return Err(FourGhostError::VisualCaptureMisalignment(
                    candidate.candidate_id.clone(),
                ));
            }
        }

        let baseline_candidate = renders.baseline().ok_or(FourGhostError::MissingBaseline)?;
        let baseline = by_id
            .get(&baseline_candidate.candidate_id)
            .ok_or(FourGhostError::MissingBaseline)?;

        let mut evidence = Vec::with_capacity(4);
        for candidate in &renders.candidates {
            let observation = by_id
                .remove(&candidate.candidate_id)
                .expect("validated visual observation coverage");
            let consequence = match candidate.kind {
                GhostCandidateKind::AbstentionBaseline => None,
                GhostCandidateKind::Proposal { .. } => {
                    Some(VisualConsequenceVector::between(baseline, &observation))
                }
            };
            evidence.push(GhostVisualEvidence {
                candidate_id: candidate.candidate_id.clone(),
                observation,
                consequence,
            });
        }

        Ok(Self {
            base_revision: renders.base_revision.clone(),
            frame: renders.frame,
            evidence,
        })
    }

    pub fn evidence_for(&self, candidate_id: &str) -> Option<&GhostVisualEvidence> {
        self.evidence
            .iter()
            .find(|evidence| evidence.candidate_id == candidate_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhostDecisionKind {
    SelectProposal {
        candidate_id: String,
        proposal_id: String,
    },
    Abstain { candidate_id: String },
    Revise { considered_candidate_ids: Vec<String> },
    Inconclusive,
}

/// Artistic decision receipt. Selection is still not mutation authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostDecisionReceipt {
    pub base_revision: String,
    pub frame: StudioFrame,
    pub decision: GhostDecisionKind,
    pub rationale: Option<String>,
    pub evidence_refs: Vec<String>,
}

impl GhostDecisionReceipt {
    pub fn validate_against(&self, renders: &FourGhostRenderSet) -> Result<(), FourGhostError> {
        renders.validate()?;
        if self.base_revision != renders.base_revision || self.frame != renders.frame {
            return Err(FourGhostError::DecisionObservationMisalignment);
        }
        match &self.decision {
            GhostDecisionKind::SelectProposal {
                candidate_id,
                proposal_id,
            } => {
                let candidate = renders
                    .candidate(candidate_id)
                    .ok_or_else(|| FourGhostError::UnknownCandidate(candidate_id.clone()))?;
                match &candidate.kind {
                    GhostCandidateKind::Proposal {
                        proposal_id: observed,
                        ..
                    } if observed == proposal_id => {}
                    _ => return Err(FourGhostError::DecisionProposalMismatch),
                }
            }
            GhostDecisionKind::Abstain { candidate_id } => {
                let candidate = renders
                    .candidate(candidate_id)
                    .ok_or_else(|| FourGhostError::UnknownCandidate(candidate_id.clone()))?;
                if !matches!(candidate.kind, GhostCandidateKind::AbstentionBaseline) {
                    return Err(FourGhostError::DecisionAbstentionMismatch);
                }
            }
            GhostDecisionKind::Revise {
                considered_candidate_ids,
            } => {
                if considered_candidate_ids.is_empty() {
                    return Err(FourGhostError::EmptyRevisionSet);
                }
                let mut seen = BTreeSet::new();
                for id in considered_candidate_ids {
                    if !seen.insert(id.as_str()) {
                        return Err(FourGhostError::DuplicateRevisionCandidate(id.clone()));
                    }
                    if renders.candidate(id).is_none() {
                        return Err(FourGhostError::UnknownCandidate(id.clone()));
                    }
                }
            }
            GhostDecisionKind::Inconclusive => {}
        }
        Ok(())
    }

    pub fn selected_proposal_id(&self) -> Option<&str> {
        match &self.decision {
            GhostDecisionKind::SelectProposal { proposal_id, .. } => Some(proposal_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FourGhostError {
    MissingIdentity,
    RequiresExactlyFourCandidates,
    RequiresOneBaselineThreeProposals,
    DuplicateCandidate(String),
    DuplicateProposal(String),
    DuplicateBranch(String),
    CrossBaseScene,
    CrossObservationPlane,
    BaselinePurposeMismatch,
    ProposalPurposeMismatch,
    BaselineSceneMismatch,
    Observation(String),
    RequiresExactlyFourVisualObservations,
    DuplicateVisualObservation(String),
    MissingVisualObservation(String),
    VisualCaptureMisalignment(String),
    MissingBaseline,
    DecisionObservationMisalignment,
    UnknownCandidate(String),
    DecisionProposalMismatch,
    DecisionAbstentionMismatch,
    EmptyRevisionSet,
    DuplicateRevisionCandidate(String),
}

impl std::fmt::Display for FourGhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FourGhostError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_capture::{ArtCaptureReceipt, ArtCaptureRequest, ArtRenderChannel};
    use crate::art_observation::RenderFidelityClass;
    use crate::art_visual::ImagePlaneFeatures;

    fn fidelity() -> RenderFidelity {
        RenderFidelity {
            class: RenderFidelityClass::CognitiveObservation,
            width: 320,
            height: 180,
            samples_per_pixel: None,
            profile: "ghost-v1".into(),
        }
    }

    fn capture(id: &str, purpose: ArtCapturePurpose, scene_hash: &str) -> FidelityTaggedCapture {
        FidelityTaggedCapture {
            receipt: ArtCaptureReceipt {
                request: ArtCaptureRequest {
                    capture_id: id.into(),
                    revision_id: "r1".into(),
                    frame: StudioFrame(7),
                    scene_hash: scene_hash.into(),
                    camera_stable_id: Some("camera".into()),
                    width: 320,
                    height: 180,
                    purpose,
                    channels: vec![ArtRenderChannel::Color],
                },
                observed_revision_id: "r1".into(),
                observed_frame: StudioFrame(7),
                observed_scene_hash: scene_hash.into(),
                artifact_locator: format!("memory://{id}"),
                artifact_digest: Some(format!("digest-{id}")),
            },
            fidelity: fidelity(),
        }
    }

    fn renders() -> FourGhostRenderSet {
        FourGhostRenderSet {
            base_revision: "r1".into(),
            frame: StudioFrame(7),
            camera_stable_id: "camera".into(),
            fidelity: fidelity(),
            candidates: vec![
                GhostRenderObservation {
                    candidate_id: "baseline".into(),
                    kind: GhostCandidateKind::AbstentionBaseline,
                    base_scene_hash: "base".into(),
                    capture: capture("base", ArtCapturePurpose::CommittedObservation, "base"),
                },
                GhostRenderObservation {
                    candidate_id: "a".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "p1".into(),
                        branch_id: "b1".into(),
                    },
                    base_scene_hash: "base".into(),
                    capture: capture("a", ArtCapturePurpose::CounterfactualPreview, "preview-a"),
                },
                GhostRenderObservation {
                    candidate_id: "b".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "p2".into(),
                        branch_id: "b2".into(),
                    },
                    base_scene_hash: "base".into(),
                    capture: capture("b", ArtCapturePurpose::CounterfactualPreview, "preview-b"),
                },
                GhostRenderObservation {
                    candidate_id: "c".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "p3".into(),
                        branch_id: "b3".into(),
                    },
                    base_scene_hash: "base".into(),
                    capture: capture("c", ArtCapturePurpose::CounterfactualPreview, "preview-c"),
                },
            ],
        }
    }

    fn visual(id: &str, scene: &str, mean: f64) -> (String, VisualObservation) {
        (
            id.into(),
            VisualObservation {
                capture_id: id.into(),
                revision_id: "r1".into(),
                frame: StudioFrame(7),
                rendered_scene_hash: scene.into(),
                features: ImagePlaneFeatures {
                    mean_luminance: mean,
                    rms_contrast: 0.1,
                    edge_energy: 0.2,
                    mean_saturation: 0.3,
                    horizontal_balance: 0.0,
                    vertical_balance: 0.0,
                    center_emphasis: 0.0,
                },
            },
        )
    }

    #[test]
    fn proposal_ghosts_may_have_distinct_scene_hashes() {
        renders().validate().unwrap();
    }

    #[test]
    fn visual_consequence_is_candidate_minus_baseline() {
        let evidence = FourGhostVisualEvidenceSet::build(
            &renders(),
            vec![
                visual("base", "base", 0.4),
                visual("a", "preview-a", 0.6),
                visual("b", "preview-b", 0.3),
                visual("c", "preview-c", 0.4),
            ],
        )
        .unwrap();
        let a = evidence.evidence_for("a").unwrap();
        assert!((a.consequence.as_ref().unwrap().mean_luminance_delta - 0.2).abs() < 1e-12);
    }

    #[test]
    fn artistic_selection_is_not_commit_authority() {
        let decision = GhostDecisionReceipt {
            base_revision: "r1".into(),
            frame: StudioFrame(7),
            decision: GhostDecisionKind::SelectProposal {
                candidate_id: "a".into(),
                proposal_id: "p1".into(),
            },
            rationale: Some("opens negative space without losing silhouette".into()),
            evidence_refs: vec!["capture:a".into()],
        };
        decision.validate_against(&renders()).unwrap();
        assert_eq!(decision.selected_proposal_id(), Some("p1"));
    }
}
