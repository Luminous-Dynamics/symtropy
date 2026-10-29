// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Differential-drive adapter with explicit controller-epoch provenance.
//!
//! This adapter deliberately does not implement the untagged ControllerSink
//! trait. Every submission must provide an epoch, and that epoch plus the
//! request deadline is forwarded to the epoch-aware native-controller seam.
//!
//! The native implementation must atomically reject stale epochs and expired
//! leases at its own command-acceptance boundary. The returned acceptance time
//! is evidence of native admission, not proof that a physical motor command was
//! applied. Hardware write/application evidence belongs to a lower-level adapter.

use crate::control::{ControlRequestError, EmbodiedIntent, Pose3};
use crate::differential_drive::{
    DifferentialDriveClock, DifferentialDriveCommand, DifferentialDriveConfigError,
};
use crate::dispatch::{ControllerDispatchContext, ControllerReceipt, ControllerSubmitError};
use crate::epoch_fence::ControllerEpoch;

/// Provenance the native controller must check and preserve at command admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochFencedCommandProvenance {
    pub controller_epoch: ControllerEpoch,
    pub session_id: u64,
    pub sequence: u64,
    pub request_id: u64,
    /// Monotonic deadline in the declared native clock domain.
    pub expires_at_ns: u64,
}

/// Receipt reported only after native command admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochFencedNativeReceipt {
    pub receipt: ControllerReceipt,
    /// Generation the native controller actually checked when admitting the goal.
    pub accepted_epoch: ControllerEpoch,
    /// Native monotonic timestamp at the admission boundary, not motor application.
    pub accepted_at_ns: u64,
}

/// Platform-specific epoch-aware admission contract for a differential-drive controller.
///
/// Implementations must compare controller_epoch against the authoritative
/// generation atomically with command admission, reject now >= expires_at_ns
/// using the same monotonic clock domain, and return the epoch/time observed at
/// that boundary. A delayed check followed by an untagged enqueue does not meet
/// this contract. Independent collision monitoring, watchdogs, actuator limits,
/// and emergency-stop mechanisms must remain active.
pub trait EpochFencedDifferentialDriveLocalController {
    fn submit_goal_at_epoch(
        &mut self,
        command: DifferentialDriveCommand,
        provenance: EpochFencedCommandProvenance,
    ) -> Result<EpochFencedNativeReceipt, ControllerSubmitError>;
}

/// The adapter-level failures that must not be collapsed into task success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochDifferentialDriveError {
    EpochDidNotAdvance {
        current_generation: u64,
        requested_generation: u64,
    },
    StaleAdapterEpoch {
        current_generation: u64,
        supplied_generation: u64,
    },
    Controller(ControllerSubmitError),
    /// Native accepted a goal under an epoch other than the requested epoch.
    NativeEpochMismatch {
        requested_generation: u64,
        accepted_generation: u64,
    },
    /// Native reported acceptance at or after the command deadline.
    NativeAcceptedExpiredCommand {
        request_id: u64,
        deadline_ns: u64,
        accepted_at_ns: u64,
    },
    EmptyNativeTicket,
}

/// Audit receipt for a command admitted by the epoch-aware native controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochFencedDifferentialDriveReceipt {
    pub epoch: ControllerEpoch,
    pub request_id: u64,
    pub deadline_ns: u64,
    pub native_accepted_at_ns: u64,
    pub receipt: ControllerReceipt,
}

/// Epoch-aware semantic adapter for a differential-drive/mobile-base controller.
///
/// This is intentionally a separate type from DifferentialDriveAdapter so a
/// caller cannot silently get an epoch-zero or guessed epoch through a default.
/// It also deliberately does not implement ControllerSink: callers must name
/// the generation on every submit_at_epoch call.
pub struct EpochFencedDifferentialDriveAdapter<C, K> {
    current_epoch: ControllerEpoch,
    frame_id: String,
    max_speed_mps: f64,
    default_speed_mps: f64,
    clock: K,
    controller: C,
}

impl<C, K> EpochFencedDifferentialDriveAdapter<C, K>
where
    C: EpochFencedDifferentialDriveLocalController,
    K: DifferentialDriveClock,
{
    pub fn new(
        initial_epoch: ControllerEpoch,
        frame_id: impl Into<String>,
        max_speed_mps: f64,
        default_speed_mps: f64,
        clock: K,
        controller: C,
    ) -> Result<Self, DifferentialDriveConfigError> {
        let frame_id = frame_id.into();
        if frame_id.trim().is_empty() {
            return Err(DifferentialDriveConfigError::EmptyFrameId);
        }
        if !max_speed_mps.is_finite() || max_speed_mps <= 0.0 {
            return Err(DifferentialDriveConfigError::InvalidMaximumSpeed);
        }
        if !default_speed_mps.is_finite()
            || default_speed_mps <= 0.0
            || default_speed_mps > max_speed_mps
        {
            return Err(DifferentialDriveConfigError::InvalidDefaultSpeed);
        }

        Ok(Self {
            current_epoch: initial_epoch,
            frame_id,
            max_speed_mps,
            default_speed_mps,
            clock,
            controller,
        })
    }

    pub const fn current_epoch(&self) -> ControllerEpoch {
        self.current_epoch
    }

    /// Advance the adapter's in-process view of the authoritative generation.
    ///
    /// The host must obtain next_epoch from ControllerEpochAuthority and
    /// coordinate native-controller activation/safe-state before using it.
    /// This method does not persist authority or mutate the native controller.
    pub fn advance_epoch(
        &mut self,
        next_epoch: ControllerEpoch,
    ) -> Result<(), EpochDifferentialDriveError> {
        if next_epoch <= self.current_epoch {
            return Err(EpochDifferentialDriveError::EpochDidNotAdvance {
                current_generation: self.current_epoch.generation(),
                requested_generation: next_epoch.generation(),
            });
        }
        self.current_epoch = next_epoch;
        Ok(())
    }

    /// Validate a request and submit it with explicit epoch and expiry provenance.
    ///
    /// The local clock check is an early rejection only. Native admission must
    /// repeat epoch and deadline checks atomically at the boundary it owns.
    pub fn submit_at_epoch(
        &mut self,
        epoch: ControllerEpoch,
        dispatch: &ControllerDispatchContext<'_>,
    ) -> Result<EpochFencedDifferentialDriveReceipt, EpochDifferentialDriveError> {
        if epoch != self.current_epoch {
            return Err(EpochDifferentialDriveError::StaleAdapterEpoch {
                current_generation: self.current_epoch.generation(),
                supplied_generation: epoch.generation(),
            });
        }

        let now_s = self.clock.now_s();
        dispatch.request.validate_at(now_s).map_err(|error| {
            EpochDifferentialDriveError::Controller(match error {
                ControlRequestError::Expired => ControllerSubmitError::DeadlineMissed,
                _ => ControllerSubmitError::InvalidRequest,
            })
        })?;

        let command = self.command_for(&dispatch.request.intent)?;
        let deadline_ns = seconds_to_monotonic_ns(dispatch.request.expires_at_s).ok_or(
            EpochDifferentialDriveError::Controller(ControllerSubmitError::InvalidRequest),
        )?;
        let provenance = EpochFencedCommandProvenance {
            controller_epoch: epoch,
            session_id: dispatch.session_id,
            sequence: dispatch.sequence,
            request_id: dispatch.request.request_id,
            expires_at_ns: deadline_ns,
        };

        let native = self
            .controller
            .submit_goal_at_epoch(command, provenance)
            .map_err(EpochDifferentialDriveError::Controller)?;

        if native.accepted_epoch != epoch {
            return Err(EpochDifferentialDriveError::NativeEpochMismatch {
                requested_generation: epoch.generation(),
                accepted_generation: native.accepted_epoch.generation(),
            });
        }
        if native.accepted_at_ns >= deadline_ns {
            return Err(EpochDifferentialDriveError::NativeAcceptedExpiredCommand {
                request_id: dispatch.request.request_id,
                deadline_ns,
                accepted_at_ns: native.accepted_at_ns,
            });
        }
        if native.receipt.ticket.trim().is_empty() {
            return Err(EpochDifferentialDriveError::EmptyNativeTicket);
        }

        Ok(EpochFencedDifferentialDriveReceipt {
            epoch,
            request_id: dispatch.request.request_id,
            deadline_ns,
            native_accepted_at_ns: native.accepted_at_ns,
            receipt: native.receipt,
        })
    }

    fn command_for(
        &self,
        intent: &EmbodiedIntent,
    ) -> Result<DifferentialDriveCommand, EpochDifferentialDriveError> {
        match intent {
            EmbodiedIntent::NavigateTo {
                target,
                max_speed_mps,
            } => {
                if target.frame_id != self.frame_id {
                    return Err(EpochDifferentialDriveError::Controller(
                        ControllerSubmitError::CapabilityMismatch,
                    ));
                }
                let speed = max_speed_mps.unwrap_or(self.default_speed_mps);
                if !speed.is_finite() || speed <= 0.0 || speed > self.max_speed_mps {
                    return Err(EpochDifferentialDriveError::Controller(
                        ControllerSubmitError::CapabilityMismatch,
                    ));
                }
                Ok(DifferentialDriveCommand::NavigateTo {
                    target: target.clone(),
                    max_speed_mps: speed,
                })
            }
            EmbodiedIntent::HoldPosition => Ok(DifferentialDriveCommand::HoldPosition),
            EmbodiedIntent::Stabilize => Ok(DifferentialDriveCommand::Stabilize),
            EmbodiedIntent::Stop { reason } => Ok(DifferentialDriveCommand::Stop {
                reason: reason.clone(),
            }),
            EmbodiedIntent::RequestAssistance { reason } => {
                Ok(DifferentialDriveCommand::RequestAssistance {
                    reason: reason.clone(),
                })
            }
            EmbodiedIntent::Inspect { .. }
            | EmbodiedIntent::Reach { .. }
            | EmbodiedIntent::Manipulate { .. } => Err(EpochDifferentialDriveError::Controller(
                ControllerSubmitError::UnsupportedIntent,
            )),
        }
    }

    pub fn into_inner(self) -> C {
        self.controller
    }
}

fn seconds_to_monotonic_ns(seconds: f64) -> Option<u64> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let nanos = seconds * 1_000_000_000.0;
    if !nanos.is_finite() || nanos >= u64::MAX as f64 {
        return None;
    }
    Some(nanos.round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{ControlRequest, EmbodiedIntent, Pose3};
    use crate::differential_drive::DifferentialDriveClock;
    use std::cell::Cell;

    struct FakeClock(Cell<f64>);

    impl FakeClock {
        fn at(seconds: f64) -> Self {
            Self(Cell::new(seconds))
        }
    }

    impl DifferentialDriveClock for FakeClock {
        fn now_s(&self) -> f64 {
            self.0.get()
        }
    }

    #[derive(Default)]
    struct NativeController {
        active_epoch: Option<ControllerEpoch>,
        now_ns: u64,
        calls: Vec<EpochFencedCommandProvenance>,
        report_wrong_epoch: bool,
        report_empty_ticket: bool,
        accept_expired_for_test: bool,
    }

    impl EpochFencedDifferentialDriveLocalController for NativeController {
        fn submit_goal_at_epoch(
            &mut self,
            _command: DifferentialDriveCommand,
            provenance: EpochFencedCommandProvenance,
        ) -> Result<EpochFencedNativeReceipt, ControllerSubmitError> {
            self.calls.push(provenance);
            if self.active_epoch != Some(provenance.controller_epoch) {
                return Err(ControllerSubmitError::RejectedByController);
            }
            if self.now_ns >= provenance.expires_at_ns && !self.accept_expired_for_test {
                return Err(ControllerSubmitError::DeadlineMissed);
            }
            let accepted_epoch = if self.report_wrong_epoch {
                ControllerEpoch::new(provenance.controller_epoch.generation() + 1).unwrap()
            } else {
                provenance.controller_epoch
            };
            Ok(EpochFencedNativeReceipt {
                receipt: ControllerReceipt {
                    ticket: if self.report_empty_ticket {
                        String::new()
                    } else {
                        {
                            let generation = provenance.controller_epoch.generation();
                            let request_id = provenance.request_id;
                            format!("epoch-{generation}-request-{request_id}")
                        }
                    },
                },
                accepted_epoch,
                accepted_at_ns: self.now_ns,
            })
        }
    }

    fn epoch(generation: u64) -> ControllerEpoch {
        ControllerEpoch::new(generation).unwrap()
    }

    fn request(request_id: u64, expires_at_s: f64) -> ControlRequest {
        ControlRequest {
            request_id,
            issued_at_s: 10.0,
            expires_at_s,
            belief_confidence: 0.9,
            intent: EmbodiedIntent::NavigateTo {
                target: Pose3 {
                    frame_id: "map".into(),
                    position_m: [1.0, 2.0, 0.0],
                    orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                },
                max_speed_mps: Some(0.5),
            },
        }
    }

    fn adapter(
        initial_epoch: ControllerEpoch,
        native_epoch: ControllerEpoch,
        now_s: f64,
        native_now_ns: u64,
    ) -> EpochFencedDifferentialDriveAdapter<NativeController, FakeClock> {
        EpochFencedDifferentialDriveAdapter::new(
            initial_epoch,
            "map",
            1.2,
            0.6,
            FakeClock::at(now_s),
            NativeController {
                active_epoch: Some(native_epoch),
                now_ns: native_now_ns,
                ..NativeController::default()
            },
        )
        .unwrap()
    }

    fn context<'a>(request: &'a ControlRequest) -> ControllerDispatchContext<'a> {
        ControllerDispatchContext {
            session_id: 41,
            sequence: request.request_id,
            request,
        }
    }

    #[test]
    fn current_epoch_is_carried_to_native_controller_and_receipt() {
        let mut adapter = adapter(epoch(4), epoch(4), 10.5, 10_500_000_000);
        let request = request(11, 11.0);
        let receipt = adapter
            .submit_at_epoch(epoch(4), &context(&request))
            .unwrap();

        assert_eq!(receipt.epoch, epoch(4));
        assert_eq!(receipt.request_id, 11);
        assert_eq!(receipt.deadline_ns, 11_000_000_000);
        assert_eq!(receipt.native_accepted_at_ns, 10_500_000_000);
        assert_eq!(receipt.receipt.ticket, "epoch-4-request-11");
        let native = adapter.into_inner();
        assert_eq!(native.calls.len(), 1);
        assert_eq!(native.calls[0].controller_epoch, epoch(4));
        assert_eq!(native.calls[0].session_id, 41);
    }

    #[test]
    fn stale_adapter_epoch_is_rejected_before_native_invocation() {
        let mut adapter = adapter(epoch(5), epoch(5), 10.5, 10_500_000_000);
        let request = request(12, 11.0);
        assert_eq!(
            adapter.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochDifferentialDriveError::StaleAdapterEpoch {
                current_generation: 5,
                supplied_generation: 4,
            })
        );
        assert!(adapter.into_inner().calls.is_empty());
    }

    #[test]
    fn native_controller_rejects_old_epoch_even_if_adapter_view_is_stale() {
        let mut adapter = adapter(epoch(4), epoch(5), 10.5, 10_500_000_000);
        let request = request(13, 11.0);
        assert_eq!(
            adapter.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochDifferentialDriveError::Controller(
                ControllerSubmitError::RejectedByController
            ))
        );
        assert_eq!(adapter.into_inner().calls.len(), 1);
    }

    #[test]
    fn expired_request_is_rejected_by_native_controller_at_admission() {
        let mut adapter = adapter(epoch(4), epoch(4), 10.5, 11_000_000_000);
        let request = request(14, 11.0);
        assert_eq!(
            adapter.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochDifferentialDriveError::Controller(
                ControllerSubmitError::DeadlineMissed
            ))
        );
        assert_eq!(adapter.into_inner().calls.len(), 1);
    }

    #[test]
    fn native_epoch_mismatch_is_reported_as_a_non_compensatory_violation() {
        let mut adapter = adapter(epoch(4), epoch(4), 10.5, 10_500_000_000);
        adapter.controller.report_wrong_epoch = true;
        let request = request(15, 11.0);
        assert_eq!(
            adapter.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochDifferentialDriveError::NativeEpochMismatch {
                requested_generation: 4,
                accepted_generation: 5,
            })
        );
    }

    #[test]
    fn native_acceptance_after_deadline_is_reported_separately_from_safe_rejection() {
        let mut adapter = adapter(epoch(4), epoch(4), 10.5, 11_000_000_000);
        // Native fixture violates its contract and reports acceptance at expiry.
        adapter.controller.now_ns = 11_000_000_000;
        adapter.controller.report_empty_ticket = false;
        // Configure a violating native implementation by setting its contract
        // mode through a test-only flag in the struct.
        adapter.controller.report_wrong_epoch = false;
        // This fixture uses a native that accepts the request despite expiry.
        // The flag bypasses only the fake's rejection, never production code.
        adapter.controller.accept_expired_for_test = true;
        let request = request(16, 11.0);
        assert_eq!(
            adapter.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochDifferentialDriveError::NativeAcceptedExpiredCommand {
                request_id: 16,
                deadline_ns: 11_000_000_000,
                accepted_at_ns: 11_000_000_000,
            })
        );
    }

    #[test]
    fn non_monotonic_epoch_advance_fails_closed() {
        let mut adapter = adapter(epoch(4), epoch(4), 10.5, 10_500_000_000);
        assert_eq!(
            adapter.advance_epoch(epoch(4)),
            Err(EpochDifferentialDriveError::EpochDidNotAdvance {
                current_generation: 4,
                requested_generation: 4,
            })
        );
        assert_eq!(
            adapter.advance_epoch(epoch(3)),
            Err(EpochDifferentialDriveError::EpochDidNotAdvance {
                current_generation: 4,
                requested_generation: 3,
            })
        );
        assert_eq!(adapter.current_epoch(), epoch(4));
    }

    #[test]
    fn empty_native_ticket_is_rejected() {
        let mut adapter = adapter(epoch(4), epoch(4), 10.5, 10_500_000_000);
        adapter.controller.report_empty_ticket = true;
        let request = request(17, 11.0);
        assert_eq!(
            adapter.submit_at_epoch(epoch(4), &context(&request)),
            Err(EpochDifferentialDriveError::EmptyNativeTicket)
        );
    }
}
