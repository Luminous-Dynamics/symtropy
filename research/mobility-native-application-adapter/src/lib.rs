// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proof-carrying composition of the existing mobility action owners.
//!
//! This crate does not implement a scheduler, authority system, controller,
//! clock converter or durable event journal. It composes M2 action applicability,
//! the authority-linearized one-shot mutation boundary, and the robotics
//! bridge's epoch/lease queue.
//!
//! The live-context provider must hold its authoritative snapshot stable for
//! the entire callback. A wrapper cannot lock external authority, sensors,
//! firmware or a second process.

use mobility_action_evidence as m2;
use mobility_mutation_boundary as boundary;
use symtropy_robotics_bridge_core::epoch_command_queue::{
    EpochCommandApplication, EpochCommandQueue, EpochCommandQueueError,
};
use symtropy_robotics_bridge_core::epoch_fence::ControllerEpoch;

/// Identity of the clock used only by the robotics queue's nanosecond lease.
/// This clock is deliberately distinct from the qualified M2 action clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueClockIdentity {
    pub clock_id: String,
    pub boot_epoch_id: String,
}

/// One reading from the queue's qualified monotonic clock.
/// Its unit is explicit and is never compared to M2 ticks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueInstant {
    pub identity: QueueClockIdentity,
    pub monotonic_nanoseconds: u64,
}

/// Current owner-provided facts held stable during native application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveApplicationSnapshot {
    pub m2_context: m2::LiveContext,
    pub m2_now: boundary::QualifiedMonotonicInstant,
    pub controller_epoch: ControllerEpoch,
    /// Independently computed commitment to the complete current M1/M2/platform
    /// binding. It must not be copied from the proposed ticket.
    pub live_binding_commitment: boundary::BindingCommitment,
}

/// Provider contract: implementations must hold authoritative state stable
/// until callback returns. A cloned, unlocked snapshot is insufficient.
pub trait LiveContextProvider {
    type Error;

    fn with_live_snapshot<R>(
        &self,
        callback: impl FnOnce(&LiveApplicationSnapshot) -> R,
    ) -> Result<R, Self::Error>;
}

/// Synchronous sink callback. It must perform the actual native acceptance
/// operation and must not forward into a second, unfenced queue.
pub trait NativeApplicationSink<P> {
    fn apply(
        &mut self,
        context: &boundary::NativeApplicationContext<'_>,
        request_id: u64,
        ticket: &m2::ActionTicket,
        payload: &P,
    ) -> boundary::NativeApplicationResult;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueuedAction<P> {
    ticket: m2::ActionTicket,
    payload: P,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelledQueuedAction<P> {
    pub epoch: ControllerEpoch,
    pub request_id: u64,
    pub ticket: m2::ActionTicket,
    pub payload: P,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterError<E> {
    Queue(EpochCommandQueueError),
    QueueClockMismatch,
    EmptyIdentity,
    ZeroTicketCommitment,
    BindingCommitmentMismatch,
    ZeroLiveBindingCommitment,
    InvalidTicketValidity,
    PendingMutation,
    NoPendingMutation,
    MissingResolutionContext,
    UnexpectedResolutionContext,
    ControllerEpochMismatch,
    M2ClockBindingMismatch,
    M2Admission(m2::AdmissionError),
    Boundary(boundary::Rejection),
    ContextProvider(E),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionReport<P> {
    pub disposition: boundary::ResolutionDisposition,
    pub cancelled_queued_actions: Vec<CancelledQueuedAction<P>>,
}

pub type QueueApplicationResult<E> =
    Option<EpochCommandApplication<Result<boundary::ApplicationReceipt, AdapterError<E>>>>;
pub type ApplyNextResult<E> = Result<QueueApplicationResult<E>, AdapterError<E>>;

/// Single-flight application gate. It configures the existing epoch queue
/// with capacity one, preserving the current serial mutation rule.
pub struct NativeApplicationGate<P> {
    queue: EpochCommandQueue<QueuedAction<P>>,
    queue_clock: QueueClockIdentity,
    boundary: Option<boundary::ApplicationBoundary>,
    pending_attempt_key: Option<String>,
}

impl<P> NativeApplicationGate<P> {
    pub fn new(
        initial_epoch: ControllerEpoch,
        queue_clock: QueueClockIdentity,
    ) -> Result<Self, AdapterError<()>> {
        if queue_clock.clock_id.trim().is_empty() || queue_clock.boot_epoch_id.trim().is_empty() {
            return Err(AdapterError::QueueClockMismatch);
        }
        Ok(Self {
            queue: EpochCommandQueue::new(initial_epoch, 1).map_err(AdapterError::Queue)?,
            queue_clock,
            boundary: None,
            pending_attempt_key: None,
        })
    }

    pub const fn controller_epoch(&self) -> ControllerEpoch {
        self.queue.current_epoch()
    }

    pub fn is_mutation_pending(&self) -> bool {
        self.pending_attempt_key.is_some()
    }

    pub fn pending_attempt_key(&self) -> Option<&str> {
        self.pending_attempt_key.as_deref()
    }

    /// Queue one M2 ticket. This is not admission or actuator authority:
    /// all applicability fields are rechecked inside the application callback.
    pub fn enqueue(
        &mut self,
        request_id: u64,
        queue_now: &QueueInstant,
        deadline_nanoseconds: u64,
        ticket: m2::ActionTicket,
        payload: P,
    ) -> Result<(), AdapterError<()>> {
        if self.pending_attempt_key.is_some() {
            return Err(AdapterError::PendingMutation);
        }
        if queue_now.identity != self.queue_clock {
            return Err(AdapterError::QueueClockMismatch);
        }
        if request_id == 0
            || ticket.mission_id.trim().is_empty()
            || ticket.subgoal_id.trim().is_empty()
            || ticket.action_id.trim().is_empty()
        {
            return Err(AdapterError::EmptyIdentity);
        }
        if !nonzero(&ticket.ticket_commitment.0) {
            return Err(AdapterError::ZeroTicketCommitment);
        }
        if ticket.issued_at_ticks >= ticket.expires_at_ticks {
            return Err(AdapterError::InvalidTicketValidity);
        }
        self.queue
            .enqueue_at(
                self.queue.current_epoch(),
                request_id,
                deadline_nanoseconds,
                queue_now.monotonic_nanoseconds,
                QueuedAction { ticket, payload },
            )
            .map_err(AdapterError::Queue)
    }

    /// Recheck queue clock identity before the queue compares nanosecond values.
    /// Within its synchronous callback this method obtains a stable owner
    /// snapshot, revalidates M2, checks the independent live binding commitment,
    /// invokes the one-shot authority boundary, then calls the native sink.
    pub fn apply_next_at<PR, SN>(
        &mut self,
        queue_now: &QueueInstant,
        provider: &PR,
        sink: &mut SN,
    ) -> ApplyNextResult<PR::Error>
    where
        PR: LiveContextProvider,
        SN: NativeApplicationSink<P>,
    {
        if queue_now.identity != self.queue_clock {
            return Err(AdapterError::QueueClockMismatch);
        }
        if self.pending_attempt_key.is_some() {
            return Err(AdapterError::PendingMutation);
        }

        let boundary_slot = &mut self.boundary;
        let pending_slot = &mut self.pending_attempt_key;
        let expected_queue_clock = &self.queue_clock;

        Ok(self.queue.apply_next_at(
            queue_now.monotonic_nanoseconds,
            |epoch, request_id, _deadline, queued| {
                if expected_queue_clock.clock_id.trim().is_empty()
                    || expected_queue_clock.boot_epoch_id.trim().is_empty()
                {
                    return Err(AdapterError::QueueClockMismatch);
                }

                provider
                    .with_live_snapshot(|snapshot| {
                        if snapshot.controller_epoch != epoch {
                            return Err(AdapterError::ControllerEpochMismatch);
                        }
                        if !nonzero(&snapshot.live_binding_commitment) {
                            return Err(AdapterError::ZeroLiveBindingCommitment);
                        }
                        validate_m2_clock(snapshot)?;
                        m2::validate_ticket_applicability(&queued.ticket, &snapshot.m2_context)
                            .map_err(AdapterError::M2Admission)?;

                        let attempt_key = attempt_key_from_context(&snapshot.m2_context);
                        let ticket_id = digest_hex(&queued.ticket.ticket_commitment.0);
                        let ticket_binding = boundary::AuthorityBinding {
                            commitment: queued.ticket.ticket_commitment.0,
                            controller_epoch: epoch.generation(),
                            clock_domain_id: queued.ticket.clock.clock_id.clone(),
                            boot_epoch_id: queued.ticket.clock.boot_epoch_id.clone(),
                        };
                        let live_binding = boundary::AuthorityBinding {
                            commitment: snapshot.live_binding_commitment,
                            controller_epoch: epoch.generation(),
                            clock_domain_id: snapshot.m2_context.clock.clock_id.clone(),
                            boot_epoch_id: snapshot.m2_context.clock.boot_epoch_id.clone(),
                        };
                        let live_authority = boundary::LiveAuthoritySnapshot {
                            active_attempt_key: attempt_key.clone(),
                            binding: live_binding,
                        };
                        match boundary_slot {
                            Some(existing) => existing
                                .replace_live_snapshot(live_authority)
                                .map_err(AdapterError::Boundary)?,
                            None => {
                                *boundary_slot =
                                    Some(boundary::ApplicationBoundary::new(live_authority))
                            }
                        }

                        let boundary_ticket = boundary::ActionTicket {
                            ticket_id,
                            attempt_key: attempt_key.clone(),
                            binding: ticket_binding,
                            issued_at_ticks: queued.ticket.issued_at_ticks,
                            expires_at_ticks: queued.ticket.expires_at_ticks,
                        };

                        let application = boundary_slot
                            .as_ref()
                            .expect("boundary was initialized above")
                            .apply(&boundary_ticket, &snapshot.m2_now, |native_context| {
                                sink.apply(
                                    native_context,
                                    request_id,
                                    &queued.ticket,
                                    &queued.payload,
                                )
                            })
                            .map_err(AdapterError::Boundary)?;

                        *pending_slot = Some(attempt_key);
                        Ok(application)
                    })
                    .map_err(AdapterError::ContextProvider)?
            },
        ))
    }

    /// Advance the existing controller epoch and return every old-epoch queued
    /// item for audit. Epoch advance never clears an unresolved mutation.
    pub fn advance_epoch(
        &mut self,
        next_epoch: ControllerEpoch,
    ) -> Result<Vec<CancelledQueuedAction<P>>, AdapterError<()>> {
        if self.pending_attempt_key.is_some() {
            return Err(AdapterError::PendingMutation);
        }
        self.queue
            .advance_epoch(next_epoch)
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| CancelledQueuedAction {
                        epoch: item.epoch,
                        request_id: item.request_id,
                        ticket: item.command.ticket,
                        payload: item.command.payload,
                    })
                    .collect()
            })
            .map_err(AdapterError::Queue)
    }

    /// Resolve the held mutation only after the upstream M1/independent observer
    /// owner has qualified it. Unknown retains the hold. Other outcomes require
    /// a fresh M2-valid next-action ticket and a strictly newer controller epoch.
    ///
    /// Queue epoch advancement occurs before clearing the boundary hold. If the
    /// boundary rejects the resolution, the advanced queue remains blocked by
    /// pending_attempt_key and the caller must reconcile; no action is released.
    pub fn resolve_pending_action<PR>(
        &mut self,
        attempt_key: &str,
        resolution: boundary::QualifiedOutcomeResolution,
        next_ticket: Option<&m2::ActionTicket>,
        next_epoch: Option<ControllerEpoch>,
        recovery_permit: Option<&boundary::RecoveryPermitReference>,
        provider: &PR,
    ) -> Result<ResolutionReport<P>, AdapterError<PR::Error>>
    where
        PR: LiveContextProvider,
    {
        if self.pending_attempt_key.as_deref() != Some(attempt_key) {
            return Err(AdapterError::NoPendingMutation);
        }

        let boundary_owner = self
            .boundary
            .as_mut()
            .ok_or(AdapterError::NoPendingMutation)?;
        if resolution == boundary::QualifiedOutcomeResolution::Unknown {
            if next_ticket.is_some() || next_epoch.is_some() || recovery_permit.is_some() {
                return Err(AdapterError::UnexpectedResolutionContext);
            }
            let disposition = boundary_owner
                .resolve_pending_action(attempt_key, resolution, None, None)
                .map_err(AdapterError::Boundary)?;
            return Ok(ResolutionReport {
                disposition,
                cancelled_queued_actions: Vec::new(),
            });
        }

        let ticket = next_ticket.ok_or(AdapterError::MissingResolutionContext)?;
        let epoch = next_epoch.ok_or(AdapterError::MissingResolutionContext)?;
        let queue = &mut self.queue;
        let pending_slot = &mut self.pending_attempt_key;

        provider
            .with_live_snapshot(|snapshot| {
                if snapshot.controller_epoch != epoch {
                    return Err(AdapterError::ControllerEpochMismatch);
                }
                if !nonzero(&snapshot.live_binding_commitment) {
                    return Err(AdapterError::ZeroLiveBindingCommitment);
                }
                validate_m2_clock(snapshot)?;
                m2::validate_ticket_applicability(ticket, &snapshot.m2_context)
                    .map_err(AdapterError::M2Admission)?;
                if snapshot.live_binding_commitment != ticket.ticket_commitment.0 {
                    return Err(AdapterError::BindingCommitmentMismatch);
                }

                let prior_epoch = queue.current_epoch();
                if epoch < prior_epoch {
                    return Err(AdapterError::ControllerEpochMismatch);
                }
                let cancelled = if epoch > prior_epoch {
                    queue.advance_epoch(epoch).map_err(AdapterError::Queue)?
                } else {
                    Vec::new()
                };

                let next_key = attempt_key_from_context(&snapshot.m2_context);
                let next_snapshot = boundary::LiveAuthoritySnapshot {
                    active_attempt_key: next_key,
                    binding: boundary::AuthorityBinding {
                        commitment: snapshot.live_binding_commitment,
                        controller_epoch: epoch.generation(),
                        clock_domain_id: snapshot.m2_context.clock.clock_id.clone(),
                        boot_epoch_id: snapshot.m2_context.clock.boot_epoch_id.clone(),
                    },
                };
                let disposition = boundary_owner
                    .resolve_pending_action(
                        attempt_key,
                        resolution,
                        Some(next_snapshot),
                        recovery_permit,
                    )
                    .map_err(AdapterError::Boundary)?;

                if disposition == boundary::ResolutionDisposition::ClearedForNewAuthoritySnapshot {
                    *pending_slot = None;
                }

                Ok(ResolutionReport {
                    disposition,
                    cancelled_queued_actions: cancelled
                        .into_iter()
                        .map(|item| CancelledQueuedAction {
                            epoch: item.epoch,
                            request_id: item.request_id,
                            ticket: item.command.ticket,
                            payload: item.command.payload,
                        })
                        .collect(),
                })
            })
            .map_err(AdapterError::ContextProvider)?
    }
}

fn validate_m2_clock<E>(snapshot: &LiveApplicationSnapshot) -> Result<(), AdapterError<E>> {
    if snapshot.m2_now.clock_domain_id != snapshot.m2_context.clock.clock_id
        || snapshot.m2_now.boot_epoch_id != snapshot.m2_context.clock.boot_epoch_id
        || snapshot.m2_now.ticks != snapshot.m2_context.now_ticks
    {
        return Err(AdapterError::M2ClockBindingMismatch);
    }
    Ok(())
}

fn nonzero(value: &[u8; 32]) -> bool {
    value.iter().any(|byte| *byte != 0)
}

fn digest_hex(value: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in value {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

/// Length-prefix components to avoid delimiter ambiguity. This is an identity
/// tuple, not a digest or authentication mechanism.
fn attempt_key_from_context(context: &m2::LiveContext) -> String {
    fn part(value: &str) -> String {
        format!("{}:{value}", value.len())
    }
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        part(&context.mission_id),
        context.mission_generation,
        part(&context.active_subgoal_id),
        part(&context.active_action_id),
        context.active_attempt,
        part(&context.active_effect_predicate_id),
        part(&context.clock.boot_epoch_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct StaticProvider {
        snapshot: Mutex<LiveApplicationSnapshot>,
    }

    impl StaticProvider {
        fn new(snapshot: LiveApplicationSnapshot) -> Self {
            Self {
                snapshot: Mutex::new(snapshot),
            }
        }
    }

    impl LiveContextProvider for StaticProvider {
        type Error = ();

        fn with_live_snapshot<R>(
            &self,
            callback: impl FnOnce(&LiveApplicationSnapshot) -> R,
        ) -> Result<R, Self::Error> {
            let snapshot = self.snapshot.lock().map_err(|_| ())?;
            Ok(callback(&snapshot))
        }
    }

    struct RecordingSink {
        calls: usize,
        result: boundary::NativeApplicationResult,
    }

    impl NativeApplicationSink<String> for RecordingSink {
        fn apply(
            &mut self,
            _context: &boundary::NativeApplicationContext<'_>,
            _request_id: u64,
            _ticket: &m2::ActionTicket,
            _payload: &String,
        ) -> boundary::NativeApplicationResult {
            self.calls += 1;
            self.result
        }
    }

    fn digest(byte: u8) -> m2::Digest {
        m2::Digest([byte; 32])
    }

    fn ticket() -> m2::ActionTicket {
        let clock = m2::ClockBinding {
            clock_id: "m2-monotonic".into(),
            boot_epoch_id: "m2-boot-7".into(),
        };
        m2::ActionTicket {
            ticket_commitment: digest(7),
            mission_id: "mission-1".into(),
            mission_generation: 3,
            subgoal_id: "pick".into(),
            action_id: "pick-attempt-action".into(),
            attempt: 1,
            action_digest: digest(8),
            expected_effect_predicate_id: "object-in-gripper".into(),
            platform: m2::PlatformBinding {
                platform_id: "robot-1".into(),
                configuration_digest: digest(9),
            },
            operation: m2::OperationBinding {
                odd_id: "lab".into(),
                odd_revision: digest(10),
                mode_id: "manipulation".into(),
                mode_revision: digest(11),
            },
            authority: m2::AuthorityBinding {
                authority_id: "mission-authority".into(),
                epoch: 5,
                required_scope: "pick".into(),
            },
            clock: clock.clone(),
            issued_at_ticks: 900,
            expires_at_ticks: 1200,
            navigation: m2::NavigationRequirement {
                profile_digest: digest(12),
                required_service: m2::NavigationService::Qualified,
                spatial_frame_id: "world".into(),
                source_id: "localization-1".into(),
                minimum_source_sequence: 7,
                maximum_age_ticks: 50,
            },
            expected_observation_frame_id: "wrist-camera".into(),
            expected_calibration_id: "calibration-5".into(),
            expected_effect_digest: digest(13),
            expected_observer_failure_domains: vec!["wrist-camera".into()],
            observer_independence_policy_digest: digest(14),
        }
    }

    fn next_ticket() -> m2::ActionTicket {
        let mut next = ticket();
        next.ticket_commitment = digest(17);
        next.subgoal_id = "place".into();
        next.action_id = "place-attempt-action".into();
        next.attempt = 2;
        next.expected_effect_predicate_id = "object-on-target".into();
        next.authority.epoch = 6;
        next.authority.required_scope = "place".into();
        next.issued_at_ticks = 1_150;
        next.expires_at_ticks = 1_400;
        next
    }

    fn snapshot(ticket: &m2::ActionTicket, epoch: u64, now_ticks: u64) -> LiveApplicationSnapshot {
        let clock = ticket.clock.clone();
        LiveApplicationSnapshot {
            m2_context: m2::LiveContext {
                mission_id: ticket.mission_id.clone(),
                mission_generation: ticket.mission_generation,
                active_subgoal_id: ticket.subgoal_id.clone(),
                active_action_id: ticket.action_id.clone(),
                active_attempt: ticket.attempt,
                active_effect_predicate_id: ticket.expected_effect_predicate_id.clone(),
                platform: ticket.platform.clone(),
                operation: ticket.operation.clone(),
                authority_id: ticket.authority.authority_id.clone(),
                authority_epoch: ticket.authority.epoch,
                authority_revoked: false,
                authority_scopes: vec![ticket.authority.required_scope.clone()],
                clock: clock.clone(),
                now_ticks,
                navigation_profile_digest: ticket.navigation.profile_digest,
                navigation_service: m2::NavigationService::Qualified,
                navigation_frame_id: ticket.navigation.spatial_frame_id.clone(),
                navigation_source_id: ticket.navigation.source_id.clone(),
                navigation_source_sequence: 9,
                navigation_time: m2::NavigationTimeEvidence::Qualified {
                    source_clock: clock.clone(),
                    comparison_clock: clock.clone(),
                    normalized_observed_at_ticks: Some(now_ticks - 5),
                    normalization_commitment: Some(digest(15)),
                },
            },
            m2_now: boundary::QualifiedMonotonicInstant {
                clock_domain_id: clock.clock_id,
                boot_epoch_id: clock.boot_epoch_id,
                ticks: now_ticks,
            },
            controller_epoch: ControllerEpoch::new(epoch).expect("valid controller epoch"),
            live_binding_commitment: ticket.ticket_commitment.0,
        }
    }

    fn queue_clock() -> QueueClockIdentity {
        QueueClockIdentity {
            clock_id: "bridge-monotonic-ns".into(),
            boot_epoch_id: "bridge-boot-2".into(),
        }
    }

    fn queue_now(ns: u64) -> QueueInstant {
        QueueInstant {
            identity: queue_clock(),
            monotonic_nanoseconds: ns,
        }
    }

    fn gate() -> NativeApplicationGate<String> {
        NativeApplicationGate::new(ControllerEpoch::new(5).unwrap(), queue_clock()).unwrap()
    }

    fn assert_rejected_at_application(
        action: m2::ActionTicket,
        live: LiveApplicationSnapshot,
        expected: m2::AdmissionError,
    ) {
        let provider = StaticProvider::new(live);
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "fixture".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Err(AdapterError::M2Admission(actual)),
                ..
            }) if actual == expected
        ));
        assert_eq!(sink.calls, 0);
        assert!(!gate.is_mutation_pending());
    }

    #[test]
    fn valid_ticket_reaches_native_sink_once_and_holds_mutation_slot() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 5, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied { result: Ok(_), .. })
        ));
        assert_eq!(sink.calls, 1);
        assert!(gate.is_mutation_pending());
        assert!(matches!(
            gate.enqueue(2, &queue_now(160), 220, ticket(), "duplicate".into()),
            Err(AdapterError::PendingMutation)
        ));
    }

    #[test]
    fn changed_configuration_at_application_yields_zero_native_calls() {
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_context.platform.configuration_digest = digest(99);
        let provider = StaticProvider::new(live);
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Err(AdapterError::M2Admission(
                    m2::AdmissionError::ConfigurationMismatch
                )),
                ..
            })
        ));
        assert_eq!(sink.calls, 0);
        assert!(!gate.is_mutation_pending());
    }

    #[test]
    fn stale_live_binding_commitment_is_rejected_by_authority_boundary() {
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.live_binding_commitment = [88; 32];
        let provider = StaticProvider::new(live);
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Err(AdapterError::Boundary(boundary::Rejection::BindingMismatch)),
                ..
            })
        ));
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn controller_epoch_mismatch_rejects_before_native_sink() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 6, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Err(AdapterError::ControllerEpochMismatch),
                ..
            })
        ));
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn m2_clock_domain_or_boot_mismatch_rejects_before_native_sink() {
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_now.boot_epoch_id = "restarted-boot".into();
        let provider = StaticProvider::new(live);
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Err(AdapterError::M2ClockBindingMismatch),
                ..
            })
        ));
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn queue_clock_basis_mismatch_does_not_pop_or_apply_queued_work() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 5, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let wrong_clock = QueueInstant {
            identity: QueueClockIdentity {
                clock_id: "some-other-clock".into(),
                boot_epoch_id: "bridge-boot-2".into(),
            },
            monotonic_nanoseconds: 150,
        };
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        assert_eq!(
            gate.apply_next_at(&wrong_clock, &provider, &mut sink),
            Err(AdapterError::QueueClockMismatch)
        );
        assert_eq!(sink.calls, 0);
        assert_eq!(gate.queue.len(), 1);
    }

    #[test]
    fn queue_lease_expiry_at_application_never_invokes_callback() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 5, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let result = gate
            .apply_next_at(&queue_now(200), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::RejectedExpired { .. })
        ));
        assert_eq!(sink.calls, 0);
    }

    #[test]
    fn unknown_native_result_still_consumes_and_holds_attempt() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 5, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::UnknownAfterPossibleApplication,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Ok(receipt),
                ..
            }) if receipt.disposition == boundary::ReceiptDisposition::ReconciliationRequired
        ));
        assert_eq!(sink.calls, 1);
        assert!(gate.is_mutation_pending());
    }

    #[test]
    fn native_definitive_no_effect_still_requires_existing_recovery_permit() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 5, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::RejectedNoEffect,
        };
        let result = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        assert!(matches!(
            result,
            Some(EpochCommandApplication::Applied {
                result: Ok(receipt),
                ..
            }) if receipt.disposition == boundary::ReceiptDisposition::RejectedNoEffect
        ));
        assert_eq!(sink.calls, 1);
        assert!(gate.is_mutation_pending());
    }
    #[test]
    fn satisfied_effect_advances_epoch_and_admits_only_new_ticket() {
        let first = ticket();
        let first_provider = StaticProvider::new(snapshot(&first, 5, 1_050));
        let next = next_ticket();
        let next_provider = StaticProvider::new(snapshot(&next, 6, 1_170));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, first, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let applied = gate
            .apply_next_at(&queue_now(150), &first_provider, &mut sink)
            .unwrap();
        let receipt = match applied {
            Some(EpochCommandApplication::Applied {
                result: Ok(receipt),
                ..
            }) => receipt,
            other => panic!("expected successful native application, got {other:?}"),
        };
        let report = gate
            .resolve_pending_action(
                &receipt.attempt_key,
                boundary::QualifiedOutcomeResolution::Satisfied,
                Some(&next),
                Some(ControllerEpoch::new(6).unwrap()),
                None,
                &next_provider,
            )
            .unwrap();
        assert_eq!(
            report.disposition,
            boundary::ResolutionDisposition::ClearedForNewAuthoritySnapshot
        );
        assert!(!gate.is_mutation_pending());
        assert_eq!(gate.controller_epoch(), ControllerEpoch::new(6).unwrap());

        gate.enqueue(2, &queue_now(220), 300, next, "place".into())
            .unwrap();
        let applied_next = gate
            .apply_next_at(&queue_now(250), &next_provider, &mut sink)
            .unwrap();
        assert!(matches!(
            applied_next,
            Some(EpochCommandApplication::Applied { result: Ok(_), .. })
        ));
        assert_eq!(sink.calls, 2);
        assert!(gate.is_mutation_pending());
    }

    #[test]
    fn no_effect_without_recovery_permit_keeps_hold_even_after_epoch_advance() {
        let first = ticket();
        let first_provider = StaticProvider::new(snapshot(&first, 5, 1_050));
        let next = next_ticket();
        let next_provider = StaticProvider::new(snapshot(&next, 6, 1_170));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, first, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::RejectedNoEffect,
        };
        let applied = gate
            .apply_next_at(&queue_now(150), &first_provider, &mut sink)
            .unwrap();
        let receipt = match applied {
            Some(EpochCommandApplication::Applied {
                result: Ok(receipt),
                ..
            }) => receipt,
            other => panic!("expected native no-effect receipt, got {other:?}"),
        };

        let missing_permit = gate.resolve_pending_action(
            &receipt.attempt_key,
            boundary::QualifiedOutcomeResolution::NoEffect,
            Some(&next),
            Some(ControllerEpoch::new(6).unwrap()),
            None,
            &next_provider,
        );
        assert_eq!(
            missing_permit,
            Err(AdapterError::Boundary(
                boundary::Rejection::RecoveryPermitMissing
            ))
        );
        assert!(gate.is_mutation_pending());
        assert_eq!(gate.controller_epoch(), ControllerEpoch::new(6).unwrap());

        let permit = boundary::RecoveryPermitReference {
            permit_id: "recovery-1".into(),
            failed_attempt_key: receipt.attempt_key,
            plan_digest: next.ticket_commitment.0,
            authority_epoch: 5,
        };
        let pending_key = gate.pending_attempt_key().unwrap().to_owned();
        let recovered = gate
            .resolve_pending_action(
                &pending_key,
                boundary::QualifiedOutcomeResolution::NoEffect,
                Some(&next),
                Some(ControllerEpoch::new(6).unwrap()),
                Some(&permit),
                &next_provider,
            )
            .unwrap();
        assert_eq!(
            recovered.disposition,
            boundary::ResolutionDisposition::ClearedForNewAuthoritySnapshot
        );
        assert!(!gate.is_mutation_pending());
        assert_eq!(sink.calls, 1);
    }

    #[test]
    fn unknown_resolution_retains_hold_and_rejects_new_action() {
        let action = ticket();
        let provider = StaticProvider::new(snapshot(&action, 5, 1_050));
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, action, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::UnknownAfterPossibleApplication,
        };
        let applied = gate
            .apply_next_at(&queue_now(150), &provider, &mut sink)
            .unwrap();
        let receipt = match applied {
            Some(EpochCommandApplication::Applied {
                result: Ok(receipt),
                ..
            }) => receipt,
            other => panic!("expected unknown application receipt, got {other:?}"),
        };
        let report = gate
            .resolve_pending_action(
                &receipt.attempt_key,
                boundary::QualifiedOutcomeResolution::Unknown,
                None,
                None,
                None,
                &provider,
            )
            .unwrap();
        assert_eq!(
            report.disposition,
            boundary::ResolutionDisposition::HoldRetained
        );
        assert!(gate.is_mutation_pending());
        assert_eq!(
            gate.enqueue(2, &queue_now(160), 220, ticket(), "replay".into()),
            Err(AdapterError::PendingMutation)
        );
    }

    #[test]
    fn new_action_resolution_requires_current_ticket_commitment() {
        let first = ticket();
        let first_provider = StaticProvider::new(snapshot(&first, 5, 1_050));
        let next = next_ticket();
        let mut invalid_next = snapshot(&next, 6, 1_170);
        invalid_next.live_binding_commitment = digest(99).0;
        let next_provider = StaticProvider::new(invalid_next);
        let mut gate = gate();
        gate.enqueue(1, &queue_now(100), 200, first, "grasp".into())
            .unwrap();
        let mut sink = RecordingSink {
            calls: 0,
            result: boundary::NativeApplicationResult::Accepted,
        };
        let applied = gate
            .apply_next_at(&queue_now(150), &first_provider, &mut sink)
            .unwrap();
        let receipt = match applied {
            Some(EpochCommandApplication::Applied {
                result: Ok(receipt),
                ..
            }) => receipt,
            other => panic!("expected native receipt, got {other:?}"),
        };
        assert_eq!(
            gate.resolve_pending_action(
                &receipt.attempt_key,
                boundary::QualifiedOutcomeResolution::Satisfied,
                Some(&next),
                Some(ControllerEpoch::new(6).unwrap()),
                None,
                &next_provider,
            ),
            Err(AdapterError::BindingCommitmentMismatch)
        );
        assert!(gate.is_mutation_pending());
        assert_eq!(sink.calls, 1);
    }
    #[test]
    fn distinct_m2_invalidations_all_block_the_native_sink() {
        // Operational design domain revision.
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_context.operation.odd_revision = digest(91);
        assert_rejected_at_application(action, live, m2::AdmissionError::OddRevisionMismatch);

        // Mode revision.
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_context.operation.mode_revision = digest(92);
        assert_rejected_at_application(action, live, m2::AdmissionError::ModeRevisionMismatch);

        // Revoked authority and authority epoch drift.
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_context.authority_revoked = true;
        assert_rejected_at_application(action, live, m2::AdmissionError::AuthorityRevoked);

        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_context.authority_epoch += 1;
        assert_rejected_at_application(action, live, m2::AdmissionError::AuthorityEpochMismatch);

        // Navigation sequence and qualified time-basis drift.
        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        live.m2_context.navigation_source_sequence = 6;
        assert_rejected_at_application(action, live, m2::AdmissionError::NavigationSequenceStale);

        let action = ticket();
        let mut live = snapshot(&action, 5, 1_050);
        if let m2::NavigationTimeEvidence::Qualified {
            comparison_clock, ..
        } = &mut live.m2_context.navigation_time
        {
            comparison_clock.boot_epoch_id = "different-boot".into();
        } else {
            panic!("fixture requires qualified navigation time");
        }
        assert_rejected_at_application(
            action,
            live,
            m2::AdmissionError::NavigationTimeComparisonClockMismatch,
        );

        // Exclusive M2 ticket expiry boundary (distinct from the queue lease).
        let mut action = ticket();
        action.expires_at_ticks = 1_050;
        let live = snapshot(&action, 5, 1_050);
        assert_rejected_at_application(action, live, m2::AdmissionError::TicketExpired);
    }
    #[test]
    fn invalid_queue_clock_identity_is_not_misreported_as_queue_capacity() {
        let invalid = QueueClockIdentity {
            clock_id: String::new(),
            boot_epoch_id: "bridge-boot-2".into(),
        };
        assert!(matches!(
            NativeApplicationGate::<String>::new(ControllerEpoch::new(1).unwrap(), invalid),
            Err(AdapterError::QueueClockMismatch)
        ));
    }


}
