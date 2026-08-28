// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cinematic shot/sequence planning substrate for Bevy-hosted Symthaea.
//!
//! Planning is revision-bound and non-mutating. This module stores temporal
//! intention, camera paths, candidate evidence, and explicit selections while
//! leaving actual camera/scene mutation to an authorized host adapter.

use bevy::prelude::*;
use std::collections::BTreeSet;

use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, PartialEq)]
pub struct ArtCameraPose {
    pub translation: [f32; 3],
    pub rotation_xyzw: [f32; 4],
    pub vertical_fov_radians: f32,
}

impl ArtCameraPose {
    pub fn validate(&self) -> Result<(), CinematicPlanError> {
        let finite = self
            .translation
            .iter()
            .chain(self.rotation_xyzw.iter())
            .chain(std::iter::once(&self.vertical_fov_radians))
            .all(|value| value.is_finite());
        if !finite {
            return Err(CinematicPlanError::NonFiniteCameraPose);
        }
        if self.vertical_fov_radians <= 0.0 || self.vertical_fov_radians >= std::f32::consts::PI {
            return Err(CinematicPlanError::InvalidFieldOfView);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtCameraKeyframe {
    pub frame: StudioFrame,
    pub pose: ArtCameraPose,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CinematicEvidence {
    pub dimension: String,
    pub predicted_delta: Option<f64>,
    pub observed_delta: Option<f64>,
    pub uncertainty: Option<f64>,
    pub evidence_refs: Vec<String>,
}

/// One shot plan. It describes camera/time/intention but cannot mutate Bevy.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtShotPlan {
    pub shot_id: String,
    pub base_revision: String,
    pub start_frame: StudioFrame,
    pub end_frame_exclusive: StudioFrame,
    pub camera_stable_id: Option<String>,
    pub keyframes: Vec<ArtCameraKeyframe>,
    pub artistic_intent_id: Option<String>,
    pub scheduled_proposal_ids: Vec<(StudioFrame, String)>,
    pub notes: Vec<String>,
}

impl ArtShotPlan {
    pub fn validate(&self) -> Result<(), CinematicPlanError> {
        if self.shot_id.trim().is_empty() {
            return Err(CinematicPlanError::EmptyShotId);
        }
        if self.base_revision.trim().is_empty() {
            return Err(CinematicPlanError::EmptyRevision);
        }
        if self.end_frame_exclusive <= self.start_frame {
            return Err(CinematicPlanError::InvalidShotSpan);
        }

        let mut previous = None;
        for keyframe in &self.keyframes {
            if keyframe.frame < self.start_frame || keyframe.frame >= self.end_frame_exclusive {
                return Err(CinematicPlanError::KeyframeOutsideShot(keyframe.frame));
            }
            keyframe.pose.validate()?;
            if previous.is_some_and(|frame| keyframe.frame <= frame) {
                return Err(CinematicPlanError::NonMonotonicKeyframes);
            }
            previous = Some(keyframe.frame);
        }

        for (frame, _) in &self.scheduled_proposal_ids {
            if *frame < self.start_frame || *frame >= self.end_frame_exclusive {
                return Err(CinematicPlanError::ProposalOutsideShot(*frame));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShotCandidate {
    pub candidate_id: String,
    pub plan: ArtShotPlan,
    pub evidence: Vec<CinematicEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotSelectionRecord {
    pub selected_candidate_id: Option<String>,
    /// `None` means abstain/preserve current direction.
    pub actor: String,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
}

/// Ordered sequence of shots. There is intentionally no aggregate cinematic
/// score; candidates may carry many independent consequence dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtSequencePlan {
    pub sequence_id: String,
    pub base_revision: String,
    pub shots: Vec<ArtShotPlan>,
    pub recurring_motif_refs: Vec<String>,
    pub unresolved_question_refs: Vec<String>,
}

impl ArtSequencePlan {
    pub fn validate(&self) -> Result<(), CinematicPlanError> {
        if self.sequence_id.trim().is_empty() {
            return Err(CinematicPlanError::EmptySequenceId);
        }
        if self.base_revision.trim().is_empty() {
            return Err(CinematicPlanError::EmptyRevision);
        }

        let mut ids = BTreeSet::new();
        let mut previous_end = None;
        for shot in &self.shots {
            shot.validate()?;
            if shot.base_revision != self.base_revision {
                return Err(CinematicPlanError::CrossRevisionSequence);
            }
            if !ids.insert(shot.shot_id.as_str()) {
                return Err(CinematicPlanError::DuplicateShotId(shot.shot_id.clone()));
            }
            if previous_end.is_some_and(|end| shot.start_frame < end) {
                return Err(CinematicPlanError::OverlappingShots);
            }
            previous_end = Some(shot.end_frame_exclusive);
        }
        Ok(())
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct CinematicHistory {
    records: Vec<ExecutedShotRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutedShotRecord {
    pub sequence_id: String,
    pub shot_id: String,
    pub revision_id: String,
    pub start_frame: StudioFrame,
    pub end_frame_exclusive: StudioFrame,
    pub capture_ids: Vec<String>,
    pub selected_candidate_id: Option<String>,
    pub evidence_refs: Vec<String>,
}

impl CinematicHistory {
    pub fn record(&mut self, record: ExecutedShotRecord) -> Result<(), CinematicPlanError> {
        if self.records.iter().any(|existing| {
            existing.sequence_id == record.sequence_id && existing.shot_id == record.shot_id
        }) {
            return Err(CinematicPlanError::DuplicateExecutedShot {
                sequence_id: record.sequence_id,
                shot_id: record.shot_id,
            });
        }
        self.records.push(record);
        Ok(())
    }

    pub fn records(&self) -> impl Iterator<Item = &ExecutedShotRecord> {
        self.records.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CinematicPlanError {
    EmptyShotId,
    EmptySequenceId,
    EmptyRevision,
    InvalidShotSpan,
    NonFiniteCameraPose,
    InvalidFieldOfView,
    KeyframeOutsideShot(StudioFrame),
    NonMonotonicKeyframes,
    ProposalOutsideShot(StudioFrame),
    CrossRevisionSequence,
    DuplicateShotId(String),
    OverlappingShots,
    DuplicateExecutedShot { sequence_id: String, shot_id: String },
}

impl std::fmt::Display for CinematicPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyShotId => write!(f, "shot id may not be empty"),
            Self::EmptySequenceId => write!(f, "sequence id may not be empty"),
            Self::EmptyRevision => write!(f, "cinematic plan must be revision-bound"),
            Self::InvalidShotSpan => write!(f, "shot end must be greater than shot start"),
            Self::NonFiniteCameraPose => write!(f, "camera pose contains non-finite values"),
            Self::InvalidFieldOfView => write!(f, "camera field of view must be inside (0, pi)"),
            Self::KeyframeOutsideShot(frame) => {
                write!(f, "camera keyframe {:?} lies outside shot", frame)
            }
            Self::NonMonotonicKeyframes => write!(f, "camera keyframes must be strictly ordered"),
            Self::ProposalOutsideShot(frame) => {
                write!(f, "scheduled proposal {:?} lies outside shot", frame)
            }
            Self::CrossRevisionSequence => write!(f, "sequence mixes shots from different revisions"),
            Self::DuplicateShotId(id) => write!(f, "duplicate shot id: {id}"),
            Self::OverlappingShots => write!(f, "shots overlap in the sequence timeline"),
            Self::DuplicateExecutedShot { sequence_id, shot_id } => write!(
                f,
                "shot {shot_id} in sequence {sequence_id} has already been recorded"
            ),
        }
    }
}

impl std::error::Error for CinematicPlanError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(id: &str, start: u64, end: u64) -> ArtShotPlan {
        ArtShotPlan {
            shot_id: id.into(),
            base_revision: "r1".into(),
            start_frame: StudioFrame(start),
            end_frame_exclusive: StudioFrame(end),
            camera_stable_id: Some("camera-main".into()),
            keyframes: Vec::new(),
            artistic_intent_id: None,
            scheduled_proposal_ids: Vec::new(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn sequence_rejects_overlapping_shots() {
        let sequence = ArtSequencePlan {
            sequence_id: "seq".into(),
            base_revision: "r1".into(),
            shots: vec![shot("a", 0, 10), shot("b", 9, 20)],
            recurring_motif_refs: Vec::new(),
            unresolved_question_refs: Vec::new(),
        };
        assert_eq!(sequence.validate(), Err(CinematicPlanError::OverlappingShots));
    }

    #[test]
    fn candidate_evidence_has_no_scalar_score_field() {
        let candidate = ShotCandidate {
            candidate_id: "c1".into(),
            plan: shot("a", 0, 10),
            evidence: vec![CinematicEvidence {
                dimension: "negative-space-balance".into(),
                predicted_delta: Some(0.2),
                observed_delta: None,
                uncertainty: Some(0.1),
                evidence_refs: vec!["eye:42".into()],
            }],
        };
        assert_eq!(candidate.evidence.len(), 1);
    }

    #[test]
    fn cinematic_history_preserves_abstention_selection() {
        let mut history = CinematicHistory::default();
        history
            .record(ExecutedShotRecord {
                sequence_id: "seq".into(),
                shot_id: "shot".into(),
                revision_id: "r1".into(),
                start_frame: StudioFrame(0),
                end_frame_exclusive: StudioFrame(10),
                capture_ids: vec!["capture-1".into()],
                selected_candidate_id: None,
                evidence_refs: vec!["baseline-preserved".into()],
            })
            .unwrap();
        assert_eq!(history.records().count(), 1);
    }
}
