// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed state machine for one live baseline + three-ghost artistic cycle.
//!
//! The session begins from a prospective four-candidate capture plan, before any
//! render exists. Missing/dropped evidence therefore cannot silently disappear
//! from a post-hoc candidate set. A decision is legal only after exactly four
//! render receipts and exactly four aligned visual observations have completed.
//! Closing requires a separate causal closure receipt; this module owns no scene
//! or authority handle.

use std::collections::{BTreeMap, BTreeSet};

use crate::art_ghost_loop::{
    FourGhostCycleReceipt, FourGhostError, FourGhostRenderSet, FourGhostVisualEvidenceSet,
    GhostCandidateKind, GhostDecisionReceipt, GhostRenderObservation,
};
use crate::art_observation::RenderFidelity;
use crate::art_timeline::StudioFrame;
use crate::art_visual::VisualObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FourGhostSessionPhase {
    Planned,
    Rendering,
    Rendered,
    Perceived,
    Decided,
    Closed,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FourGhostCandidatePlan {
    pub candidate_id: String,
    pub capture_id: String,
    pub kind: GhostCandidateKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FourGhostPlan {
    pub base_revision: String,
    pub base_scene_hash: String,
    pub frame: StudioFrame,
    pub camera_stable_id: String,
    pub fidelity: RenderFidelity,
    pub candidates: Vec<FourGhostCandidatePlan>,
}

impl FourGhostPlan {
    pub fn validate(&self) -> Result<(), FourGhostSessionError> {
        if self.base_revision.trim().is_empty()
            || self.base_scene_hash.trim().is_empty()
            || self.camera_stable_id.trim().is_empty()
        {
            return Err(FourGhostSessionError::MissingIdentity);
        }
        self.fidelity
            .validate()
            .map_err(|error| FourGhostSessionError::Observation(error.to_string()))?;
        if self.candidates.len() != 4 {
            return Err(FourGhostSessionError::RequiresExactlyFourCandidates);
        }

        let mut candidate_ids = BTreeSet::new();
        let mut capture_ids = BTreeSet::new();
        let mut proposal_ids = BTreeSet::new();
        let mut branch_ids = BTreeSet::new();
        let mut baselines = 0usize;
        for candidate in &self.candidates {
            if candidate.candidate_id.trim().is_empty() || candidate.capture_id.trim().is_empty() {
                return Err(FourGhostSessionError::MissingIdentity);
            }
            if !candidate_ids.insert(candidate.candidate_id.as_str()) {
                return Err(FourGhostSessionError::DuplicateCandidate(
                    candidate.candidate_id.clone(),
                ));
            }
            if !capture_ids.insert(candidate.capture_id.as_str()) {
                return Err(FourGhostSessionError::DuplicateCapture(
                    candidate.capture_id.clone(),
                ));
            }
            match &candidate.kind {
                GhostCandidateKind::AbstentionBaseline => baselines += 1,
                GhostCandidateKind::Proposal {
                    proposal_id,
                    branch_id,
                } => {
                    if proposal_id.trim().is_empty() || branch_id.trim().is_empty() {
                        return Err(FourGhostSessionError::MissingIdentity);
                    }
                    if !proposal_ids.insert(proposal_id.as_str()) {
                        return Err(FourGhostSessionError::DuplicateProposal(proposal_id.clone()));
                    }
                    if !branch_ids.insert(branch_id.as_str()) {
                        return Err(FourGhostSessionError::DuplicateBranch(branch_id.clone()));
                    }
                }
            }
        }
        if baselines != 1 || proposal_ids.len() != 3 || branch_ids.len() != 3 {
            return Err(FourGhostSessionError::RequiresOneBaselineThreeProposals);
        }
        Ok(())
    }

    pub fn candidate(&self, candidate_id: &str) -> Option<&FourGhostCandidatePlan> {
        self.candidates
            .iter()
            .find(|candidate| candidate.candidate_id == candidate_id)
    }

    pub fn capture_ids(&self) -> BTreeSet<&str> {
        self.candidates
            .iter()
            .map(|candidate| candidate.capture_id.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedGhostCapture {
    pub candidate_id: String,
    pub capture_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostEvidenceFailure {
    pub capture_id: String,
    pub reason: String,
}

#[derive(Debug)]
pub struct FourGhostSession {
    plan: FourGhostPlan,
    phase: FourGhostSessionPhase,
    renders: BTreeMap<String, GhostRenderObservation>,
    render_set: Option<FourGhostRenderSet>,
    visual: BTreeMap<String, VisualObservation>,
    failures: Vec<GhostEvidenceFailure>,
    evidence: Option<FourGhostVisualEvidenceSet>,
    decision: Option<GhostDecisionReceipt>,
    closure: Option<FourGhostCycleReceipt>,
}

impl FourGhostSession {
    pub fn new(plan: FourGhostPlan) -> Result<Self, FourGhostSessionError> {
        plan.validate()?;
        Ok(Self {
            plan,
            phase: FourGhostSessionPhase::Planned,
            renders: BTreeMap::new(),
            render_set: None,
            visual: BTreeMap::new(),
            failures: Vec::new(),
            evidence: None,
            decision: None,
            closure: None,
        })
    }

    pub fn phase(&self) -> FourGhostSessionPhase {
        self.phase
    }

    pub fn plan(&self) -> &FourGhostPlan {
        &self.plan
    }

    pub fn expected_captures(&self) -> Vec<ExpectedGhostCapture> {
        self.plan
            .candidates
            .iter()
            .map(|candidate| ExpectedGhostCapture {
                candidate_id: candidate.candidate_id.clone(),
                capture_id: candidate.capture_id.clone(),
            })
            .collect()
    }

    pub fn begin_rendering(&mut self) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Planned)?;
        self.phase = FourGhostSessionPhase::Rendering;
        Ok(())
    }

    pub fn record_render(
        &mut self,
        candidate_id: impl Into<String>,
        render: GhostRenderObservation,
    ) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendering)?;
        let candidate_id = candidate_id.into();
        let planned = self
            .plan
            .candidate(&candidate_id)
            .ok_or_else(|| FourGhostSessionError::UnexpectedCandidate(candidate_id.clone()))?;
        if render.candidate_id != candidate_id {
            return Err(FourGhostSessionError::RenderCandidateMismatch);
        }
        if render.kind != planned.kind {
            return Err(FourGhostSessionError::RenderKindMismatch(candidate_id));
        }
        if render.base_scene_hash != self.plan.base_scene_hash {
            return Err(FourGhostSessionError::BaseSceneMismatch);
        }
        let receipt = &render.capture.receipt;
        if receipt.request.capture_id != planned.capture_id {
            return Err(FourGhostSessionError::CaptureIdentityMismatch {
                candidate_id,
                expected: planned.capture_id.clone(),
                actual: receipt.request.capture_id.clone(),
            });
        }
        if receipt.observed_revision_id != self.plan.base_revision
            || receipt.observed_frame != self.plan.frame
            || receipt.request.camera_stable_id.as_deref()
                != Some(self.plan.camera_stable_id.as_str())
            || render.capture.fidelity != self.plan.fidelity
        {
            return Err(FourGhostSessionError::RenderObservationPlaneMismatch);
        }
        if self.renders.insert(render.candidate_id.clone(), render).is_some() {
            return Err(FourGhostSessionError::DuplicateRender(candidate_id));
        }
        Ok(())
    }

    /// Record a capture/readback/backpressure failure. A confirmatory four-ghost
    /// cycle is invalid from this point; it cannot continue with three options.
    pub fn record_failure(
        &mut self,
        capture_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendering)?;
        let capture_id = capture_id.into();
        if !self.plan.capture_ids().contains(capture_id.as_str()) {
            return Err(FourGhostSessionError::UnexpectedCapture(capture_id));
        }
        self.failures.push(GhostEvidenceFailure {
            capture_id,
            reason: reason.into(),
        });
        self.phase = FourGhostSessionPhase::Invalid;
        Ok(())
    }

    pub fn finish_rendering(&mut self) -> Result<&FourGhostRenderSet, FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendering)?;
        if !self.failures.is_empty() {
            self.phase = FourGhostSessionPhase::Invalid;
            return Err(FourGhostSessionError::EvidenceFailure);
        }
        if self.renders.len() != 4 {
            return Err(FourGhostSessionError::IncompleteRenderEvidence {
                expected: 4,
                actual: self.renders.len(),
            });
        }
        let planned: BTreeSet<&str> = self
            .plan
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect();
        let observed: BTreeSet<&str> = self.renders.keys().map(String::as_str).collect();
        if planned != observed {
            return Err(FourGhostSessionError::CandidateCoverageMismatch);
        }

        let render_set = FourGhostRenderSet {
            base_revision: self.plan.base_revision.clone(),
            frame: self.plan.frame,
            camera_stable_id: self.plan.camera_stable_id.clone(),
            fidelity: self.plan.fidelity.clone(),
            candidates: self.renders.values().cloned().collect(),
        };
        render_set.validate().map_err(FourGhostSessionError::Ghost)?;
        self.render_set = Some(render_set);
        self.phase = FourGhostSessionPhase::Rendered;
        Ok(self.render_set.as_ref().expect("render set just inserted"))
    }

    pub fn record_visual(
        &mut self,
        candidate_id: impl Into<String>,
        observation: VisualObservation,
    ) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendered)?;
        let candidate_id = candidate_id.into();
        let render = self
            .render_set
            .as_ref()
            .and_then(|renders| renders.candidate(&candidate_id))
            .ok_or_else(|| FourGhostSessionError::UnexpectedCandidate(candidate_id.clone()))?;
        if observation.capture_id != render.capture.receipt.request.capture_id {
            return Err(FourGhostSessionError::CaptureIdentityMismatch {
                candidate_id,
                expected: render.capture.receipt.request.capture_id.clone(),
                actual: observation.capture_id,
            });
        }
        if self.visual.insert(render.candidate_id.clone(), observation).is_some() {
            return Err(FourGhostSessionError::DuplicateVisual(
                render.candidate_id.clone(),
            ));
        }
        Ok(())
    }

    pub fn finish_perception(&mut self) -> Result<&FourGhostVisualEvidenceSet, FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendered)?;
        if self.visual.len() != 4 {
            return Err(FourGhostSessionError::IncompleteVisualEvidence {
                expected: 4,
                actual: self.visual.len(),
            });
        }
        let renders = self
            .render_set
            .as_ref()
            .ok_or(FourGhostSessionError::MissingRenders)?;
        let observations = self
            .visual
            .iter()
            .map(|(candidate, observation)| (candidate.clone(), observation.clone()))
            .collect();
        let evidence = FourGhostVisualEvidenceSet::build(renders, observations)
            .map_err(FourGhostSessionError::Ghost)?;
        self.evidence = Some(evidence);
        self.phase = FourGhostSessionPhase::Perceived;
        Ok(self.evidence.as_ref().expect("evidence just inserted"))
    }

    pub fn decide(&mut self, decision: GhostDecisionReceipt) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Perceived)?;
        decision
            .validate_against(
                self.render_set
                    .as_ref()
                    .ok_or(FourGhostSessionError::MissingRenders)?,
            )
            .map_err(FourGhostSessionError::Ghost)?;
        if self.evidence.is_none() {
            return Err(FourGhostSessionError::MissingPerception);
        }
        self.decision = Some(decision);
        self.phase = FourGhostSessionPhase::Decided;
        Ok(())
    }

    pub fn close(
        &mut self,
        closure: FourGhostCycleReceipt,
    ) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Decided)?;
        let decision = self
            .decision
            .as_ref()
            .ok_or(FourGhostSessionError::MissingDecision)?;
        if &closure.decision != decision {
            return Err(FourGhostSessionError::ClosureDecisionMismatch);
        }
        closure
            .validate_closed(
                self.render_set
                    .as_ref()
                    .ok_or(FourGhostSessionError::MissingRenders)?,
                self.evidence
                    .as_ref()
                    .ok_or(FourGhostSessionError::MissingPerception)?,
            )
            .map_err(FourGhostSessionError::Ghost)?;
        self.closure = Some(closure);
        self.phase = FourGhostSessionPhase::Closed;
        Ok(())
    }

    pub fn failures(&self) -> &[GhostEvidenceFailure] {
        &self.failures
    }

    pub fn render_set(&self) -> Option<&FourGhostRenderSet> {
        self.render_set.as_ref()
    }

    pub fn evidence(&self) -> Option<&FourGhostVisualEvidenceSet> {
        self.evidence.as_ref()
    }

    pub fn decision(&self) -> Option<&GhostDecisionReceipt> {
        self.decision.as_ref()
    }

    pub fn closure(&self) -> Option<&FourGhostCycleReceipt> {
        self.closure.as_ref()
    }

    fn require_phase(&self, expected: FourGhostSessionPhase) -> Result<(), FourGhostSessionError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(FourGhostSessionError::WrongPhase {
                expected,
                actual: self.phase,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FourGhostSessionError {
    Ghost(FourGhostError),
    Observation(String),
    MissingIdentity,
    RequiresExactlyFourCandidates,
    RequiresOneBaselineThreeProposals,
    DuplicateCandidate(String),
    DuplicateCapture(String),
    DuplicateProposal(String),
    DuplicateBranch(String),
    WrongPhase {
        expected: FourGhostSessionPhase,
        actual: FourGhostSessionPhase,
    },
    UnexpectedCandidate(String),
    UnexpectedCapture(String),
    RenderCandidateMismatch,
    RenderKindMismatch(String),
    BaseSceneMismatch,
    CaptureIdentityMismatch {
        candidate_id: String,
        expected: String,
        actual: String,
    },
    RenderObservationPlaneMismatch,
    DuplicateRender(String),
    EvidenceFailure,
    IncompleteRenderEvidence { expected: usize, actual: usize },
    CandidateCoverageMismatch,
    DuplicateVisual(String),
    IncompleteVisualEvidence { expected: usize, actual: usize },
    MissingRenders,
    MissingPerception,
    MissingDecision,
    ClosureDecisionMismatch,
}

impl std::fmt::Display for FourGhostSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FourGhostSessionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_observation::RenderFidelityClass;

    fn plan() -> FourGhostPlan {
        FourGhostPlan {
            base_revision: "r1".into(),
            base_scene_hash: "base".into(),
            frame: StudioFrame(7),
            camera_stable_id: "camera".into(),
            fidelity: RenderFidelity {
                class: RenderFidelityClass::CognitiveObservation,
                width: 320,
                height: 180,
                samples_per_pixel: None,
                profile: "ghost-v1".into(),
            },
            candidates: vec![
                FourGhostCandidatePlan {
                    candidate_id: "baseline".into(),
                    capture_id: "capture-base".into(),
                    kind: GhostCandidateKind::AbstentionBaseline,
                },
                FourGhostCandidatePlan {
                    candidate_id: "a".into(),
                    capture_id: "capture-a".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "p1".into(),
                        branch_id: "b1".into(),
                    },
                },
                FourGhostCandidatePlan {
                    candidate_id: "b".into(),
                    capture_id: "capture-b".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "p2".into(),
                        branch_id: "b2".into(),
                    },
                },
                FourGhostCandidatePlan {
                    candidate_id: "c".into(),
                    capture_id: "capture-c".into(),
                    kind: GhostCandidateKind::Proposal {
                        proposal_id: "p3".into(),
                        branch_id: "b3".into(),
                    },
                },
            ],
        }
    }

    #[test]
    fn prospective_plan_requires_exact_four_way_structure() {
        plan().validate().unwrap();
        let mut bad = plan();
        bad.candidates.pop();
        assert_eq!(
            bad.validate(),
            Err(FourGhostSessionError::RequiresExactlyFourCandidates)
        );
    }

    #[test]
    fn a_known_capture_failure_invalidates_the_session() {
        let mut session = FourGhostSession::new(plan()).unwrap();
        session.begin_rendering().unwrap();
        session
            .record_failure("capture-b", "gpu readback dropped")
            .unwrap();
        assert_eq!(session.phase(), FourGhostSessionPhase::Invalid);
        assert_eq!(session.failures().len(), 1);
    }
}
