// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Revision-isolated counterfactual preview registry for Bevy art worlds.
//!
//! This registry records preview lineage only. It owns no Bevy scene mutation
//! handle, so creating/observing/discarding a preview cannot advance committed
//! scene identity by construction.

use bevy::prelude::*;
use std::collections::BTreeMap;

use crate::art_timeline::StudioFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum PreviewBranchState {
    Proposed,
    Rendering,
    Ready,
    Disposed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewBranch {
    pub branch_id: String,
    pub base_revision: String,
    pub frame: StudioFrame,
    pub proposal_ids: Vec<String>,
    pub capture_ids: Vec<String>,
    pub state: PreviewBranchState,
}

#[derive(Resource, Debug, Clone)]
pub struct CounterfactualRegistry {
    committed_revision: String,
    branches: BTreeMap<String, PreviewBranch>,
}

impl CounterfactualRegistry {
    pub fn new(committed_revision: impl Into<String>) -> Result<Self, CounterfactualError> {
        let committed_revision = committed_revision.into();
        if committed_revision.trim().is_empty() {
            return Err(CounterfactualError::EmptyCommittedRevision);
        }
        Ok(Self {
            committed_revision,
            branches: BTreeMap::new(),
        })
    }

    pub fn committed_revision(&self) -> &str {
        &self.committed_revision
    }

    pub fn branch(&self, branch_id: &str) -> Option<&PreviewBranch> {
        self.branches.get(branch_id)
    }

    pub fn create_branch(
        &mut self,
        branch_id: impl Into<String>,
        base_revision: impl Into<String>,
        frame: StudioFrame,
        proposal_ids: Vec<String>,
    ) -> Result<(), CounterfactualError> {
        let branch_id = branch_id.into();
        let base_revision = base_revision.into();
        if branch_id.trim().is_empty() {
            return Err(CounterfactualError::EmptyBranchId);
        }
        if base_revision != self.committed_revision {
            return Err(CounterfactualError::StaleBaseRevision {
                base: base_revision,
                committed: self.committed_revision.clone(),
            });
        }
        if self.branches.contains_key(&branch_id) {
            return Err(CounterfactualError::DuplicateBranch(branch_id));
        }
        self.branches.insert(
            branch_id.clone(),
            PreviewBranch {
                branch_id,
                base_revision,
                frame,
                proposal_ids,
                capture_ids: Vec::new(),
                state: PreviewBranchState::Proposed,
            },
        );
        Ok(())
    }

    pub fn mark_rendering(&mut self, branch_id: &str) -> Result<(), CounterfactualError> {
        let branch = self.branch_mut(branch_id)?;
        if branch.state != PreviewBranchState::Proposed {
            return Err(CounterfactualError::InvalidTransition);
        }
        branch.state = PreviewBranchState::Rendering;
        Ok(())
    }

    pub fn attach_capture(
        &mut self,
        branch_id: &str,
        capture_id: impl Into<String>,
    ) -> Result<(), CounterfactualError> {
        let capture_id = capture_id.into();
        let branch = self.branch_mut(branch_id)?;
        if !matches!(
            branch.state,
            PreviewBranchState::Rendering | PreviewBranchState::Ready
        ) {
            return Err(CounterfactualError::InvalidTransition);
        }
        if branch.capture_ids.iter().any(|existing| existing == &capture_id) {
            return Err(CounterfactualError::DuplicateCapture(capture_id));
        }
        branch.capture_ids.push(capture_id);
        Ok(())
    }

    pub fn mark_ready(&mut self, branch_id: &str) -> Result<(), CounterfactualError> {
        let branch = self.branch_mut(branch_id)?;
        if branch.state != PreviewBranchState::Rendering {
            return Err(CounterfactualError::InvalidTransition);
        }
        branch.state = PreviewBranchState::Ready;
        Ok(())
    }

    pub fn dispose(&mut self, branch_id: &str) -> Result<(), CounterfactualError> {
        let branch = self.branch_mut(branch_id)?;
        if branch.state == PreviewBranchState::Disposed {
            return Err(CounterfactualError::InvalidTransition);
        }
        branch.state = PreviewBranchState::Disposed;
        Ok(())
    }

    /// Host adapters call this *after* a separately authorized committed scene
    /// mutation. This only updates registry identity and disposes stale previews;
    /// it cannot perform the scene mutation itself.
    pub fn observe_committed_revision(
        &mut self,
        revision: impl Into<String>,
    ) -> Result<(), CounterfactualError> {
        let revision = revision.into();
        if revision.trim().is_empty() {
            return Err(CounterfactualError::EmptyCommittedRevision);
        }
        self.committed_revision = revision;
        for branch in self.branches.values_mut() {
            if branch.base_revision != self.committed_revision {
                branch.state = PreviewBranchState::Disposed;
            }
        }
        Ok(())
    }

    pub fn active_branches(&self) -> impl Iterator<Item = &PreviewBranch> {
        self.branches
            .values()
            .filter(|branch| branch.state != PreviewBranchState::Disposed)
    }

    fn branch_mut(&mut self, branch_id: &str) -> Result<&mut PreviewBranch, CounterfactualError> {
        self.branches
            .get_mut(branch_id)
            .ok_or_else(|| CounterfactualError::UnknownBranch(branch_id.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterfactualError {
    EmptyCommittedRevision,
    EmptyBranchId,
    DuplicateBranch(String),
    UnknownBranch(String),
    StaleBaseRevision { base: String, committed: String },
    InvalidTransition,
    DuplicateCapture(String),
}

impl std::fmt::Display for CounterfactualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCommittedRevision => write!(f, "committed revision may not be empty"),
            Self::EmptyBranchId => write!(f, "preview branch id may not be empty"),
            Self::DuplicateBranch(id) => write!(f, "duplicate preview branch id: {id}"),
            Self::UnknownBranch(id) => write!(f, "unknown preview branch id: {id}"),
            Self::StaleBaseRevision { base, committed } => write!(
                f,
                "preview base revision {base} does not match committed revision {committed}"
            ),
            Self::InvalidTransition => write!(f, "invalid preview branch state transition"),
            Self::DuplicateCapture(id) => write!(f, "duplicate preview capture id: {id}"),
        }
    }
}

impl std::error::Error for CounterfactualError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_preview_never_advances_committed_revision() {
        let mut registry = CounterfactualRegistry::new("r1").unwrap();
        registry
            .create_branch("b1", "r1", StudioFrame(10), vec!["p1".into()])
            .unwrap();
        registry.mark_rendering("b1").unwrap();
        registry.attach_capture("b1", "capture-1").unwrap();
        registry.mark_ready("b1").unwrap();
        assert_eq!(registry.committed_revision(), "r1");
    }

    #[test]
    fn stale_branch_creation_fails_closed() {
        let mut registry = CounterfactualRegistry::new("r2").unwrap();
        assert!(matches!(
            registry.create_branch("b1", "r1", StudioFrame(1), vec![]),
            Err(CounterfactualError::StaleBaseRevision { .. })
        ));
    }

    #[test]
    fn real_commit_observation_disposes_old_previews() {
        let mut registry = CounterfactualRegistry::new("r1").unwrap();
        registry
            .create_branch("b1", "r1", StudioFrame(1), vec![])
            .unwrap();
        registry.observe_committed_revision("r2").unwrap();
        assert_eq!(registry.active_branches().count(), 0);
        assert_eq!(registry.committed_revision(), "r2");
    }
}
