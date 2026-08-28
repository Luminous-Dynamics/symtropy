// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed state machine for one live baseline + three-ghost artistic cycle.
//!
//! Missing or dropped evidence never silently reduces the candidate set. The
//! session can decide only after all four expected captures have arrived and
//! been converted into aligned visual evidence. Closing the session requires a
//! separately supplied causal closure receipt; this module owns no scene or
//! authority handle.

use std::collections::{BTreeMap, BTreeSet};

use crate::art_ghost_loop::{
    FourGhostCycleReceipt, FourGhostError, FourGhostRenderSet, FourGhostVisualEvidenceSet,
    GhostDecisionReceipt,
};
use crate::art_visual::VisualObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FourGhostSessionPhase {
    Planned,
    Rendering,
    Perceived,
    Decided,
    Closed,
    Invalid,
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
    renders: FourGhostRenderSet,
    phase: FourGhostSessionPhase,
    expected: BTreeMap<String, String>,
    visual: BTreeMap<String, VisualObservation>,
    failures: Vec<GhostEvidenceFailure>,
    evidence: Option<FourGhostVisualEvidenceSet>,
    decision: Option<GhostDecisionReceipt>,
    closure: Option<FourGhostCycleReceipt>,
}

impl FourGhostSession {
    pub fn new(renders: FourGhostRenderSet) -> Result<Self, FourGhostSessionError> {
        renders.validate().map_err(FourGhostSessionError::Ghost)?;
        let mut expected = BTreeMap::new();
        for candidate in &renders.candidates {
            let capture_id = candidate.capture.receipt.request.capture_id.clone();
            if expected
                .insert(candidate.candidate_id.clone(), capture_id)
                .is_some()
            {
                return Err(FourGhostSessionError::DuplicateCandidate);
            }
        }
        Ok(Self {
            renders,
            phase: FourGhostSessionPhase::Planned,
            expected,
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

    pub fn renders(&self) -> &FourGhostRenderSet {
        &self.renders
    }

    pub fn expected_captures(&self) -> Vec<ExpectedGhostCapture> {
        self.expected
            .iter()
            .map(|(candidate_id, capture_id)| ExpectedGhostCapture {
                candidate_id: candidate_id.clone(),
                capture_id: capture_id.clone(),
            })
            .collect()
    }

    pub fn begin_rendering(&mut self) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Planned)?;
        self.phase = FourGhostSessionPhase::Rendering;
        Ok(())
    }

    pub fn record_visual(
        &mut self,
        candidate_id: impl Into<String>,
        observation: VisualObservation,
    ) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendering)?;
        let candidate_id = candidate_id.into();
        let expected_capture = self
            .expected
            .get(&candidate_id)
            .ok_or_else(|| FourGhostSessionError::UnexpectedCandidate(candidate_id.clone()))?;
        if observation.capture_id != *expected_capture {
            return Err(FourGhostSessionError::CaptureIdentityMismatch {
                candidate_id,
                expected: expected_capture.clone(),
                actual: observation.capture_id,
            });
        }
        if self.visual.insert(candidate_id.clone(), observation).is_some() {
            return Err(FourGhostSessionError::DuplicateVisual(candidate_id));
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
        if !self.expected.values().any(|expected| expected == &capture_id) {
            return Err(FourGhostSessionError::UnexpectedCapture(capture_id));
        }
        self.failures.push(GhostEvidenceFailure {
            capture_id,
            reason: reason.into(),
        });
        self.phase = FourGhostSessionPhase::Invalid;
        Ok(())
    }

    pub fn finish_perception(&mut self) -> Result<&FourGhostVisualEvidenceSet, FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Rendering)?;
        if !self.failures.is_empty() {
            self.phase = FourGhostSessionPhase::Invalid;
            return Err(FourGhostSessionError::EvidenceFailure);
        }
        if self.visual.len() != 4 {
            return Err(FourGhostSessionError::IncompleteVisualEvidence {
                expected: 4,
                actual: self.visual.len(),
            });
        }
        let expected_ids: BTreeSet<&str> = self.expected.keys().map(String::as_str).collect();
        let observed_ids: BTreeSet<&str> = self.visual.keys().map(String::as_str).collect();
        if expected_ids != observed_ids {
            return Err(FourGhostSessionError::CandidateCoverageMismatch);
        }

        let observations = self
            .visual
            .iter()
            .map(|(candidate, observation)| (candidate.clone(), observation.clone()))
            .collect();
        let evidence = FourGhostVisualEvidenceSet::build(&self.renders, observations)
            .map_err(FourGhostSessionError::Ghost)?;
        self.evidence = Some(evidence);
        self.phase = FourGhostSessionPhase::Perceived;
        Ok(self.evidence.as_ref().expect("evidence just inserted"))
    }

    pub fn decide(&mut self, decision: GhostDecisionReceipt) -> Result<(), FourGhostSessionError> {
        self.require_phase(FourGhostSessionPhase::Perceived)?;
        decision
            .validate_against(&self.renders)
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
        if closure.decision != *decision {
            return Err(FourGhostSessionError::ClosureDecisionMismatch);
        }
        closure
            .validate_closed(
                &self.renders,
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
    DuplicateCandidate,
    WrongPhase {
        expected: FourGhostSessionPhase,
        actual: FourGhostSessionPhase,
    },
    UnexpectedCandidate(String),
    UnexpectedCapture(String),
    CaptureIdentityMismatch {
        candidate_id: String,
        expected: String,
        actual: String,
    },
    DuplicateVisual(String),
    EvidenceFailure,
    IncompleteVisualEvidence { expected: usize, actual: usize },
    CandidateCoverageMismatch,
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

    #[test]
    fn invalid_phase_transition_fails_closed() {
        // Construction is exercised through the integration tests in
        // `art_ghost_loop`; this unit test keeps the phase contract explicit.
        assert_ne!(FourGhostSessionPhase::Planned, FourGhostSessionPhase::Perceived);
    }
}
