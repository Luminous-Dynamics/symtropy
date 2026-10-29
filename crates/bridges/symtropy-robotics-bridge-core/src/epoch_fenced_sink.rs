// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! In-process command-admission fence that requires an explicit controller epoch.
//!
//! This wrapper serializes the epoch comparison and wrapped sink submission
//! through one exclusive mutable borrow. When shared across threads, callers
//! must hold one mutex over the entire wrapper method. This protects one
//! in-process admission boundary only; a separate host/process can still advance
//! a persistent authority unless the actual native command sink enforces the
//! same fencing token atomically.
//!
//! Advancing the epoch does not cancel a command already accepted by the inner
//! controller. Restart integration must separately drain/cancel old work or put
//! the platform into its defined safe state before enabling the new generation.

use crate::dispatch::{
    ControllerDispatchContext, ControllerReceipt, ControllerSink, ControllerSubmitError,
};
use crate::epoch_fence::ControllerEpoch;

/// Receipt binding a controller ticket to the generation that admitted it.
///
/// Retain both the epoch and ticket when registering the submitted request in
/// the outcome lifecycle ledger. The epoch is part of the effective controller
/// request identity, even if a restarted native controller reuses a ticket string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochFencedControllerReceipt {
    pub epoch: ControllerEpoch,
    pub receipt: ControllerReceipt,
}

/// Fail-closed results from epoch-fenced controller admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochFencedSubmitError {
    EpochDidNotAdvance {
        current_generation: u64,
        requested_generation: u64,
    },
    StaleEpoch {
        current_generation: u64,
        supplied_generation: u64,
    },
    Controller(ControllerSubmitError),
}

/// A single-owner wrapper that requires a generation on every controller submission.
///
/// This wrapper deliberately does not implement `ControllerSink`: doing so
/// would allow callers to submit without naming the generation. Use
/// `submit_at_epoch` and retain the epoch-tagged receipt.
pub struct EpochFencedControllerSink<S> {
    current_epoch: ControllerEpoch,
    inner: S,
}

impl<S: ControllerSink> EpochFencedControllerSink<S> {
    pub fn new(initial_epoch: ControllerEpoch, inner: S) -> Self {
        Self {
            current_epoch: initial_epoch,
            inner,
        }
    }

    pub const fn current_epoch(&self) -> ControllerEpoch {
        self.current_epoch
    }

    /// Activate a strictly newer generation at this in-process admission boundary.
    ///
    /// The caller must obtain `next_epoch` from the authoritative epoch
    /// authority and coordinate this transition with the native controller's
    /// restart/safe-state procedure. This method does not itself consult or
    /// persist the authority and cannot prove that a supplied generation is real.
    pub fn advance_epoch(
        &mut self,
        next_epoch: ControllerEpoch,
    ) -> Result<(), EpochFencedSubmitError> {
        if next_epoch <= self.current_epoch {
            return Err(EpochFencedSubmitError::EpochDidNotAdvance {
                current_generation: self.current_epoch.generation(),
                requested_generation: next_epoch.generation(),
            });
        }
        self.current_epoch = next_epoch;
        Ok(())
    }

    /// Compare the supplied generation and submit while retaining exclusive access.
    ///
    /// On a stale epoch, the wrapped controller sink is not called. A successful
    /// receipt binds the native controller ticket to the generation checked here.
    /// Production hardware/IPC controllers must still verify the same token at
    /// their actual enqueue/application boundary to prevent cross-process TOCTOU.
    pub fn submit_at_epoch(
        &mut self,
        epoch: ControllerEpoch,
        dispatch: &ControllerDispatchContext<'_>,
    ) -> Result<EpochFencedControllerReceipt, EpochFencedSubmitError> {
        if epoch != self.current_epoch {
            return Err(EpochFencedSubmitError::StaleEpoch {
                current_generation: self.current_epoch.generation(),
                supplied_generation: epoch.generation(),
            });
        }

        let receipt = self
            .inner
            .submit(dispatch)
            .map_err(EpochFencedSubmitError::Controller)?;
        Ok(EpochFencedControllerReceipt { epoch, receipt })
    }

    /// Consume the wrapper during controlled shutdown or adapter replacement.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{ControlRequest, EmbodiedIntent};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct RecordingSink {
        calls: Vec<u64>,
        ticket: String,
        fail: bool,
    }

    impl ControllerSink for RecordingSink {
        fn submit(
            &mut self,
            dispatch: &ControllerDispatchContext<'_>,
        ) -> Result<ControllerReceipt, ControllerSubmitError> {
            self.calls.push(dispatch.request.request_id);
            if self.fail {
                return Err(ControllerSubmitError::ControllerUnavailable);
            }
            Ok(ControllerReceipt {
                ticket: self.ticket.clone(),
            })
        }
    }

    fn epoch(generation: u64) -> ControllerEpoch {
        ControllerEpoch::new(generation).unwrap()
    }

    fn request(request_id: u64) -> ControlRequest {
        ControlRequest {
            request_id,
            issued_at_s: 1.0,
            expires_at_s: 2.0,
            belief_confidence: 0.9,
            intent: EmbodiedIntent::HoldPosition,
        }
    }

    fn context<'a>(request: &'a ControlRequest) -> ControllerDispatchContext<'a> {
        ControllerDispatchContext {
            session_id: 7,
            sequence: request.request_id,
            request,
        }
    }

    #[test]
    fn current_epoch_submission_returns_epoch_bound_ticket() {
        let sink = RecordingSink {
            calls: Vec::new(),
            ticket: "native-ticket-reused".into(),
            fail: false,
        };
        let mut fenced = EpochFencedControllerSink::new(epoch(4), sink);
        let request = request(11);
        let receipt = fenced.submit_at_epoch(epoch(4), &context(&request)).unwrap();

        assert_eq!(receipt.epoch, epoch(4));
        assert_eq!(receipt.receipt.ticket, "native-ticket-reused");
        assert_eq!(fenced.into_inner().calls, vec![11]);
    }

    #[test]
    fn stale_epoch_is_rejected_before_calling_the_controller_sink() {
        let sink = RecordingSink {
            calls: Vec::new(),
            ticket: "ticket".into(),
            fail: false,
        };
        let mut fenced = EpochFencedControllerSink::new(epoch(5), sink);
        let request = request(12);

        assert_eq!(
            fenced.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochFencedSubmitError::StaleEpoch {
                current_generation: 5,
                supplied_generation: 4,
            })
        );
        assert!(fenced.into_inner().calls.is_empty());
    }

    #[test]
    fn advancing_generation_rejects_old_token_even_when_ticket_names_are_reused() {
        let sink = RecordingSink {
            calls: Vec::new(),
            ticket: "same-native-ticket".into(),
            fail: false,
        };
        let mut fenced = EpochFencedControllerSink::new(epoch(1), sink);
        let first_request = request(13);
        let first = fenced
            .submit_at_epoch(epoch(1), &context(&first_request))
            .unwrap();
        assert_eq!(first.receipt.ticket, "same-native-ticket");

        fenced.advance_epoch(epoch(2)).unwrap();
        let old_request = request(14);
        assert_eq!(
            fenced.submit_at_epoch(epoch(1), &context(&old_request)),
            Err(EpochFencedSubmitError::StaleEpoch {
                current_generation: 2,
                supplied_generation: 1,
            })
        );

        let new_request = request(15);
        let second = fenced
            .submit_at_epoch(epoch(2), &context(&new_request))
            .unwrap();
        assert_eq!(second.epoch, epoch(2));
        assert_eq!(second.receipt.ticket, "same-native-ticket");
        assert_eq!(fenced.into_inner().calls, vec![13, 15]);
    }

    #[test]
    fn generation_cannot_stay_equal_or_move_backwards() {
        let sink = RecordingSink::default();
        let mut fenced = EpochFencedControllerSink::new(epoch(3), sink);
        assert_eq!(
            fenced.advance_epoch(epoch(3)),
            Err(EpochFencedSubmitError::EpochDidNotAdvance {
                current_generation: 3,
                requested_generation: 3,
            })
        );
        assert_eq!(
            fenced.advance_epoch(epoch(2)),
            Err(EpochFencedSubmitError::EpochDidNotAdvance {
                current_generation: 3,
                requested_generation: 2,
            })
        );
        assert_eq!(fenced.current_epoch(), epoch(3));
    }

    #[test]
    fn controller_failure_is_not_reclassified_as_completion() {
        let sink = RecordingSink {
            calls: Vec::new(),
            ticket: "unused".into(),
            fail: true,
        };
        let mut fenced = EpochFencedControllerSink::new(epoch(8), sink);
        let request = request(17);
        assert_eq!(
            fenced.submit_at_epoch(epoch(8), &context(&request)),
            Err(EpochFencedSubmitError::Controller(
                ControllerSubmitError::ControllerUnavailable
            ))
        );
        assert_eq!(fenced.into_inner().calls, vec![17]);
    }

    #[test]
    fn mutex_owned_boundary_serializes_generation_change_and_submission() {
        let sink = RecordingSink {
            calls: Vec::new(),
            ticket: "serialized-ticket".into(),
            fail: false,
        };
        let shared = Arc::new(Mutex::new(EpochFencedControllerSink::new(epoch(1), sink)));
        let before_restart = request(21);
        {
            let mut guard = shared.lock().unwrap();
            guard
                .submit_at_epoch(epoch(1), &context(&before_restart))
                .unwrap();
            guard.advance_epoch(epoch(2)).unwrap();
        }

        let after_restart = request(22);
        let mut guard = shared.lock().unwrap();
        assert_eq!(
            guard.submit_at_epoch(epoch(1), &context(&after_restart)),
            Err(EpochFencedSubmitError::StaleEpoch {
                current_generation: 2,
                supplied_generation: 1,
            })
        );
        assert_eq!(guard.into_inner().unwrap().calls, vec![21]);
    }
}
