// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Internal monotonic sequencing state for physics-authority temporal provenance.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TemporalCounterError {
    MutationEpochExhausted,
    StepIndexExhausted,
    InterruptedStepTainted,
    InterruptedMutationTainted,
}

/// Private sequencing state owned by one `PhysicsAuthorityWorld`.
///
/// A mutation epoch changes whenever state can be altered outside the qualified
/// authority-owned step path. `step_index` is meaningful only within one exact
/// mutation epoch. It is ordering metadata, not elapsed physical time.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthorityTemporalState {
    mutation_epoch: u64,
    step_index: u64,
    has_authorized_step: bool,
    step_tainted: bool,
    mutation_commit_tainted: bool,
}

impl AuthorityTemporalState {
    pub(crate) const fn new() -> Self {
        Self {
            mutation_epoch: 0,
            step_index: 0,
            has_authorized_step: false,
            step_tainted: false,
            mutation_commit_tainted: false,
        }
    }

    pub(crate) const fn mutation_epoch(self) -> u64 {
        self.mutation_epoch
    }

    pub(crate) const fn step_index(self) -> u64 {
        self.step_index
    }

    pub(crate) const fn has_authorized_step(self) -> bool {
        self.has_authorized_step && !self.step_tainted && !self.mutation_commit_tainted
    }

    /// Preflight a non-step mutation without changing state.
    ///
    /// An interrupted typed mutation is intentionally not recoverable by merely
    /// advancing another epoch: its world may be structurally uncertain. The
    /// wrapper must instead be consumed and recovered through a separately
    /// qualified reconstruction/adoption path.
    pub(crate) fn next_mutation_epoch(self) -> Result<u64, TemporalCounterError> {
        if self.mutation_commit_tainted {
            return Err(TemporalCounterError::InterruptedMutationTainted);
        }
        self.mutation_epoch
            .checked_add(1)
            .ok_or(TemporalCounterError::MutationEpochExhausted)
    }

    /// Commit an already-preflighted non-step mutation boundary.
    ///
    /// This is used by the raw mutable-world escape hatch and as the first half
    /// of [`Self::begin_mutation_commit`]. It may recover an interrupted physics
    /// step, but it must never clear an interrupted typed-mutation taint.
    pub(crate) fn commit_mutation_epoch(&mut self, next_epoch: u64) {
        debug_assert!(!self.mutation_commit_tainted);
        debug_assert_eq!(self.mutation_epoch.checked_add(1), Some(next_epoch));
        self.mutation_epoch = next_epoch;
        self.step_index = 0;
        self.has_authorized_step = false;
        self.step_tainted = false;
    }

    /// Begin a prepared typed non-step mutation.
    ///
    /// The epoch is committed and the wrapper becomes mutation-tainted before
    /// the first mutation-capable instruction executes. If the commit panics and
    /// downstream catches the unwind, the taint remains set and no future step
    /// or raw mutation boundary can make the wrapper evidence-bearing again.
    pub(crate) fn begin_mutation_commit(&mut self, next_epoch: u64) {
        self.commit_mutation_epoch(next_epoch);
        self.mutation_commit_tainted = true;
    }

    /// Finish one normally completed prepared typed mutation.
    pub(crate) fn finish_mutation_commit(&mut self) {
        debug_assert!(self.mutation_commit_tainted);
        self.mutation_commit_tainted = false;
    }

    /// Begin one authority-owned step.
    ///
    /// The state becomes tainted before the underlying physics step executes.
    /// If execution panics and the unwind is caught by downstream code, no
    /// authorized-step stamp remains valid and another authorized step is
    /// rejected until an explicit mutation-epoch break occurs.
    pub(crate) fn begin_step(&mut self) -> Result<u64, TemporalCounterError> {
        if self.mutation_commit_tainted {
            return Err(TemporalCounterError::InterruptedMutationTainted);
        }
        if self.step_tainted {
            return Err(TemporalCounterError::InterruptedStepTainted);
        }
        let next_step = self
            .step_index
            .checked_add(1)
            .ok_or(TemporalCounterError::StepIndexExhausted)?;
        self.step_tainted = true;
        self.has_authorized_step = false;
        Ok(next_step)
    }

    /// Commit one normally completed authority-owned step.
    pub(crate) fn commit_step(&mut self, next_step: u64) {
        debug_assert!(!self.mutation_commit_tainted);
        debug_assert!(self.step_tainted);
        debug_assert_eq!(self.step_index.checked_add(1), Some(next_step));
        self.step_index = next_step;
        self.has_authorized_step = true;
        self.step_tainted = false;
    }
}

impl Default for AuthorityTemporalState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_advance_resets_step_and_clears_step_taint() {
        let mut state = AuthorityTemporalState::new();
        let next = state.begin_step().unwrap();
        state.commit_step(next);
        assert!(state.has_authorized_step());
        assert_eq!(state.step_index(), 1);

        let next_epoch = state.next_mutation_epoch().unwrap();
        state.commit_mutation_epoch(next_epoch);
        assert_eq!(state.mutation_epoch(), 1);
        assert_eq!(state.step_index(), 0);
        assert!(!state.has_authorized_step());
    }

    #[test]
    fn interrupted_step_taints_until_epoch_break() {
        let mut state = AuthorityTemporalState::new();
        let _uncommitted = state.begin_step().unwrap();
        assert!(!state.has_authorized_step());
        assert_eq!(
            state.begin_step(),
            Err(TemporalCounterError::InterruptedStepTainted)
        );

        let next_epoch = state.next_mutation_epoch().unwrap();
        state.commit_mutation_epoch(next_epoch);
        assert_eq!(state.begin_step(), Ok(1));
    }

    #[test]
    fn interrupted_mutation_commit_cannot_resume_or_advance_epoch() {
        let mut state = AuthorityTemporalState::new();
        let next = state.begin_step().unwrap();
        state.commit_step(next);

        let next_epoch = state.next_mutation_epoch().unwrap();
        state.begin_mutation_commit(next_epoch);

        assert_eq!(state.mutation_epoch(), 1);
        assert_eq!(state.step_index(), 0);
        assert!(!state.has_authorized_step());
        assert_eq!(
            state.begin_step(),
            Err(TemporalCounterError::InterruptedMutationTainted)
        );
        assert_eq!(
            state.next_mutation_epoch(),
            Err(TemporalCounterError::InterruptedMutationTainted)
        );
    }

    #[test]
    fn completed_mutation_commit_allows_future_steps() {
        let mut state = AuthorityTemporalState::new();
        let next_epoch = state.next_mutation_epoch().unwrap();
        state.begin_mutation_commit(next_epoch);
        state.finish_mutation_commit();

        assert_eq!(state.mutation_epoch(), 1);
        assert_eq!(state.begin_step(), Ok(1));
    }

    #[test]
    fn mutation_epoch_never_wraps() {
        let state = AuthorityTemporalState {
            mutation_epoch: u64::MAX,
            step_index: 0,
            has_authorized_step: false,
            step_tainted: false,
            mutation_commit_tainted: false,
        };
        assert_eq!(
            state.next_mutation_epoch(),
            Err(TemporalCounterError::MutationEpochExhausted)
        );
    }

    #[test]
    fn step_index_never_wraps() {
        let mut state = AuthorityTemporalState {
            mutation_epoch: 7,
            step_index: u64::MAX,
            has_authorized_step: true,
            step_tainted: false,
            mutation_commit_tainted: false,
        };
        assert_eq!(
            state.begin_step(),
            Err(TemporalCounterError::StepIndexExhausted)
        );
        assert!(state.has_authorized_step());
    }
}
