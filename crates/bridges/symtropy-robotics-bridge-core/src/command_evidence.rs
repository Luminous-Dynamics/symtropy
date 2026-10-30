// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent consistency auditor for command lifecycle evidence.
//!
//! The auditor reconstructs each request's lifecycle from a trace and an
//! independently supplied expected identity. It does not trust a success
//! receipt as proof that the command was applied, nor does it authenticate the
//! trace source or make a physical safety guarantee. Deployments must retain
//! authenticated/durable event capture and native watchdog/e-stop mechanisms.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Expected identity fixed by the campaign/supervisor before a command is run.
///
/// `clock_domain_id` names the monotonic clock domain used for all timestamps
/// and `deadline_ns`; wall-clock timestamps must not be substituted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedCommandIdentity {
    pub request_id: u64,
    pub session_id: u64,
    pub sequence: u64,
    pub controller_id: String,
    pub controller_epoch: u64,
    /// Digest of the canonical semantic/native command payload.
    pub command_payload_digest: String,
    /// Digest of the controller and safety-relevant configuration.
    pub controller_config_digest: String,
    pub deadline_ns: u64,
    pub clock_domain_id: String,
}

/// Evidence emitted by a boundary while processing one command.
///
/// Every event repeats the complete identity so an auditor can detect a
/// request/epoch/deadline change between lifecycle stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEvidenceEvent {
    pub identity: ExpectedCommandIdentity,
    pub observed_at_ns: u64,
    pub kind: CommandEvidenceKind,
}

/// Lifecycle observation. `NativeAccepted` is not proof of physical application;
/// `Applied` must be emitted by the platform boundary that owns that evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum CommandEvidenceKind {
    Proposed,
    RuntimeAdmitted,
    NativeAccepted { accepted_epoch: u64 },
    Applied { applied_epoch: u64 },
    Rejected { stage: CommandRejectionStage },
    Cancelled,
    OutcomeRecorded { outcome: CommandTerminalOutcome },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRejectionStage {
    RuntimeAdmission,
    NativeAdmission,
    Application,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandTerminalOutcome {
    Succeeded,
    Failed,
    Rejected,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEvidencePhase {
    AwaitingProposal,
    Proposed,
    RuntimeAdmitted,
    NativeAccepted,
    Applied,
    RejectedAwaitingOutcome,
    CancelledAwaitingOutcome,
    Complete,
}

/// Counts derived from a fully consistent, complete trace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEvidenceAudit {
    pub command_count: usize,
    pub runtime_admissions: usize,
    pub native_acceptances: usize,
    pub applications: usize,
    pub rejected_commands: usize,
    pub cancellations: usize,
    pub successful_outcomes: usize,
    pub failed_outcomes: usize,
    pub timed_out_outcomes: usize,
}

/// Fail-closed errors. Incomplete evidence is not treated as a successful zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvidenceAuditError {
    EmptyExpectedSet,
    InvalidExpectedIdentity { request_id: u64 },
    DuplicateExpectedRequestId { request_id: u64 },
    UnexpectedRequest { request_id: u64 },
    MissingTrace { request_id: u64 },
    IdentityMismatch { request_id: u64 },
    TimestampRegression {
        request_id: u64,
        previous_ns: u64,
        observed_ns: u64,
    },
    ExpiredRuntimeAdmission {
        request_id: u64,
        deadline_ns: u64,
        observed_at_ns: u64,
    },
    StaleNativeEpochAccepted {
        request_id: u64,
        expected_epoch: u64,
        accepted_epoch: u64,
    },
    ExpiredNativeAcceptance {
        request_id: u64,
        deadline_ns: u64,
        accepted_at_ns: u64,
    },
    StaleApplicationEpochAccepted {
        request_id: u64,
        expected_epoch: u64,
        applied_epoch: u64,
    },
    ExpiredApplication {
        request_id: u64,
        deadline_ns: u64,
        applied_at_ns: u64,
    },
    IllegalTransition {
        request_id: u64,
        phase: CommandEvidencePhase,
        event: CommandEvidenceKind,
    },
    OutcomeDoesNotMatchDisposition {
        request_id: u64,
        phase: CommandEvidencePhase,
        outcome: CommandTerminalOutcome,
    },
    EventAfterTerminal { request_id: u64 },
    MissingTerminalOutcome {
        request_id: u64,
        phase: CommandEvidencePhase,
    },
}

/// Audit complete expected identities against the raw ordered event stream.
///
/// Event order is the trace's recorded order. Timestamps must be nondecreasing
/// within each request and must use the identity's declared monotonic domain.
/// Any stale/expired acceptance is an error, even if later task utility is high.
pub fn audit_command_evidence(
    expected: &[ExpectedCommandIdentity],
    events: &[CommandEvidenceEvent],
) -> Result<CommandEvidenceAudit, CommandEvidenceAuditError> {
    if expected.is_empty() {
        return Err(CommandEvidenceAuditError::EmptyExpectedSet);
    }

    let mut expected_by_id = BTreeMap::new();
    for identity in expected {
        if identity.request_id == 0
            || identity.session_id == 0
            || identity.sequence == 0
            || identity.controller_epoch == 0
            || identity.controller_id.trim().is_empty()
            || identity.command_payload_digest.trim().is_empty()
            || identity.controller_config_digest.trim().is_empty()
            || identity.clock_domain_id.trim().is_empty()
        {
            return Err(CommandEvidenceAuditError::InvalidExpectedIdentity {
                request_id: identity.request_id,
            });
        }
        if expected_by_id
            .insert(identity.request_id, identity)
            .is_some()
        {
            return Err(CommandEvidenceAuditError::DuplicateExpectedRequestId {
                request_id: identity.request_id,
            });
        }
    }

    let expected_ids: BTreeSet<u64> = expected_by_id.keys().copied().collect();
    let mut grouped: BTreeMap<u64, Vec<&CommandEvidenceEvent>> = BTreeMap::new();
    for event in events {
        if !expected_ids.contains(&event.identity.request_id) {
            return Err(CommandEvidenceAuditError::UnexpectedRequest {
                request_id: event.identity.request_id,
            });
        }
        grouped
            .entry(event.identity.request_id)
            .or_default()
            .push(event);
    }

    let mut report = CommandEvidenceAudit {
        command_count: expected.len(),
        ..CommandEvidenceAudit::default()
    };

    for identity in expected {
        let request_id = identity.request_id;
        let request_events = grouped
            .remove(&request_id)
            .ok_or(CommandEvidenceAuditError::MissingTrace { request_id })?;
        let mut phase = CommandEvidencePhase::AwaitingProposal;
        let mut previous_ns = None;

        for event in request_events {
            if event.identity != *identity {
                return Err(CommandEvidenceAuditError::IdentityMismatch { request_id });
            }
            if let Some(previous) = previous_ns
                && event.observed_at_ns < previous
            {
                return Err(CommandEvidenceAuditError::TimestampRegression {
                    request_id,
                    previous_ns: previous,
                    observed_ns: event.observed_at_ns,
                });
            }
            previous_ns = Some(event.observed_at_ns);

            if phase == CommandEvidencePhase::Complete {
                return Err(CommandEvidenceAuditError::EventAfterTerminal { request_id });
            }

            match &event.kind {
                CommandEvidenceKind::Proposed
                    if phase == CommandEvidencePhase::AwaitingProposal =>
                {
                    phase = CommandEvidencePhase::Proposed;
                }
                CommandEvidenceKind::RuntimeAdmitted
                    if phase == CommandEvidencePhase::Proposed =>
                {
                    if event.observed_at_ns >= identity.deadline_ns {
                        return Err(CommandEvidenceAuditError::ExpiredRuntimeAdmission {
                            request_id,
                            deadline_ns: identity.deadline_ns,
                            observed_at_ns: event.observed_at_ns,
                        });
                    }
                    phase = CommandEvidencePhase::RuntimeAdmitted;
                    report.runtime_admissions += 1;
                }
                CommandEvidenceKind::NativeAccepted { accepted_epoch }
                    if phase == CommandEvidencePhase::RuntimeAdmitted =>
                {
                    if *accepted_epoch != identity.controller_epoch {
                        return Err(CommandEvidenceAuditError::StaleNativeEpochAccepted {
                            request_id,
                            expected_epoch: identity.controller_epoch,
                            accepted_epoch: *accepted_epoch,
                        });
                    }
                    if event.observed_at_ns >= identity.deadline_ns {
                        return Err(CommandEvidenceAuditError::ExpiredNativeAcceptance {
                            request_id,
                            deadline_ns: identity.deadline_ns,
                            accepted_at_ns: event.observed_at_ns,
                        });
                    }
                    phase = CommandEvidencePhase::NativeAccepted;
                    report.native_acceptances += 1;
                }
                CommandEvidenceKind::Applied { applied_epoch }
                    if phase == CommandEvidencePhase::NativeAccepted =>
                {
                    if *applied_epoch != identity.controller_epoch {
                        return Err(CommandEvidenceAuditError::StaleApplicationEpochAccepted {
                            request_id,
                            expected_epoch: identity.controller_epoch,
                            applied_epoch: *applied_epoch,
                        });
                    }
                    if event.observed_at_ns >= identity.deadline_ns {
                        return Err(CommandEvidenceAuditError::ExpiredApplication {
                            request_id,
                            deadline_ns: identity.deadline_ns,
                            applied_at_ns: event.observed_at_ns,
                        });
                    }
                    phase = CommandEvidencePhase::Applied;
                    report.applications += 1;
                }
                CommandEvidenceKind::Rejected { stage }
                    if (*stage == CommandRejectionStage::RuntimeAdmission
                        && phase == CommandEvidencePhase::Proposed)
                        || (*stage == CommandRejectionStage::NativeAdmission
                            && phase == CommandEvidencePhase::RuntimeAdmitted)
                        || (*stage == CommandRejectionStage::Application
                            && phase == CommandEvidencePhase::NativeAccepted) =>
                {
                    phase = CommandEvidencePhase::RejectedAwaitingOutcome;
                    report.rejected_commands += 1;
                }
                CommandEvidenceKind::Cancelled
                    if matches!(
                        phase,
                        CommandEvidencePhase::Proposed
                            | CommandEvidencePhase::RuntimeAdmitted
                            | CommandEvidencePhase::NativeAccepted
                    ) =>
                {
                    phase = CommandEvidencePhase::CancelledAwaitingOutcome;
                    report.cancellations += 1;
                }
                CommandEvidenceKind::OutcomeRecorded { outcome } => {
                    let allowed = match phase {
                        CommandEvidencePhase::Applied => matches!(
                            outcome,
                            CommandTerminalOutcome::Succeeded
                                | CommandTerminalOutcome::Failed
                                | CommandTerminalOutcome::TimedOut
                        ),
                        CommandEvidencePhase::RejectedAwaitingOutcome => matches!(
                            outcome,
                            CommandTerminalOutcome::Rejected | CommandTerminalOutcome::TimedOut
                        ),
                        CommandEvidencePhase::CancelledAwaitingOutcome => matches!(
                            outcome,
                            CommandTerminalOutcome::Cancelled | CommandTerminalOutcome::TimedOut
                        ),
                        _ => false,
                    };
                    if !allowed {
                        return Err(CommandEvidenceAuditError::OutcomeDoesNotMatchDisposition {
                            request_id,
                            phase,
                            outcome: *outcome,
                        });
                    }
                    match outcome {
                        CommandTerminalOutcome::Succeeded => report.successful_outcomes += 1,
                        CommandTerminalOutcome::Failed => report.failed_outcomes += 1,
                        CommandTerminalOutcome::TimedOut => report.timed_out_outcomes += 1,
                        CommandTerminalOutcome::Rejected | CommandTerminalOutcome::Cancelled => {}
                    }
                    phase = CommandEvidencePhase::Complete;
                }
                _ => {
                    return Err(CommandEvidenceAuditError::IllegalTransition {
                        request_id,
                        phase,
                        event: event.kind.clone(),
                    });
                }
            }
        }

        if phase != CommandEvidencePhase::Complete {
            return Err(CommandEvidenceAuditError::MissingTerminalOutcome {
                request_id,
                phase,
            });
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ExpectedCommandIdentity {
        ExpectedCommandIdentity {
            request_id: 11,
            session_id: 7,
            sequence: 3,
            controller_id: "mobile-base-01".into(),
            controller_epoch: 4,
            command_payload_digest: "sha256:command-fixture".into(),
            controller_config_digest: "sha256:controller-config-fixture".into(),
            deadline_ns: 100,
            clock_domain_id: "robot-monotonic-0".into(),
        }
    }

    fn event(
        identity: &ExpectedCommandIdentity,
        observed_at_ns: u64,
        kind: CommandEvidenceKind,
    ) -> CommandEvidenceEvent {
        CommandEvidenceEvent {
            identity: identity.clone(),
            observed_at_ns,
            kind,
        }
    }

    fn successful_trace(identity: &ExpectedCommandIdentity) -> Vec<CommandEvidenceEvent> {
        vec![
            event(identity, 10, CommandEvidenceKind::Proposed),
            event(identity, 11, CommandEvidenceKind::RuntimeAdmitted),
            event(
                identity,
                12,
                CommandEvidenceKind::NativeAccepted {
                    accepted_epoch: identity.controller_epoch,
                },
            ),
            event(
                identity,
                13,
                CommandEvidenceKind::Applied {
                    applied_epoch: identity.controller_epoch,
                },
            ),
            event(
                identity,
                14,
                CommandEvidenceKind::OutcomeRecorded {
                    outcome: CommandTerminalOutcome::Succeeded,
                },
            ),
        ]
    }

    #[test]
    fn empty_expected_set_is_not_reported_as_a_zero_violation_audit() {
        assert_eq!(
            audit_command_evidence(&[], &[]),
            Err(CommandEvidenceAuditError::EmptyExpectedSet)
        );
    }

    #[test]
    fn complete_successful_trace_reports_separate_lifecycle_counts() {
        let identity = identity();
        let report = audit_command_evidence(&[identity.clone()], &successful_trace(&identity))
            .unwrap();

        assert_eq!(report.command_count, 1);
        assert_eq!(report.runtime_admissions, 1);
        assert_eq!(report.native_acceptances, 1);
        assert_eq!(report.applications, 1);
        assert_eq!(report.successful_outcomes, 1);
    }

    #[test]
    fn safe_rejection_after_deadline_is_not_misreported_as_application() {
        let identity = identity();
        let events = vec![
            event(&identity, 90, CommandEvidenceKind::Proposed),
            event(
                &identity,
                100,
                CommandEvidenceKind::Rejected {
                    stage: CommandRejectionStage::RuntimeAdmission,
                },
            ),
            event(
                &identity,
                101,
                CommandEvidenceKind::OutcomeRecorded {
                    outcome: CommandTerminalOutcome::Rejected,
                },
            ),
        ];

        let report = audit_command_evidence(&[identity], &events).unwrap();
        assert_eq!(report.rejected_commands, 1);
        assert_eq!(report.applications, 0);
        assert_eq!(report.native_acceptances, 0);
    }

    #[test]
    fn expired_runtime_admission_fails_closed() {
        let identity = identity();
        let events = vec![
            event(&identity, 90, CommandEvidenceKind::Proposed),
            event(&identity, 100, CommandEvidenceKind::RuntimeAdmitted),
        ];

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::ExpiredRuntimeAdmission {
                request_id: 11,
                deadline_ns: 100,
                observed_at_ns: 100,
            })
        );
    }

    #[test]
    fn stale_native_epoch_acceptance_is_a_hard_failure() {
        let identity = identity();
        let events = vec![
            event(&identity, 10, CommandEvidenceKind::Proposed),
            event(&identity, 11, CommandEvidenceKind::RuntimeAdmitted),
            event(
                &identity,
                12,
                CommandEvidenceKind::NativeAccepted { accepted_epoch: 3 },
            ),
        ];

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::StaleNativeEpochAccepted {
                request_id: 11,
                expected_epoch: 4,
                accepted_epoch: 3,
            })
        );
    }

    #[test]
    fn expired_native_acceptance_is_a_hard_failure() {
        let identity = identity();
        let events = vec![
            event(&identity, 90, CommandEvidenceKind::Proposed),
            event(&identity, 91, CommandEvidenceKind::RuntimeAdmitted),
            event(
                &identity,
                100,
                CommandEvidenceKind::NativeAccepted { accepted_epoch: 4 },
            ),
        ];

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::ExpiredNativeAcceptance {
                request_id: 11,
                deadline_ns: 100,
                accepted_at_ns: 100,
            })
        );
    }

    #[test]
    fn stale_application_epoch_is_a_hard_failure() {
        let identity = identity();
        let events = vec![
            event(&identity, 10, CommandEvidenceKind::Proposed),
            event(&identity, 11, CommandEvidenceKind::RuntimeAdmitted),
            event(
                &identity,
                12,
                CommandEvidenceKind::NativeAccepted { accepted_epoch: 4 },
            ),
            event(
                &identity,
                13,
                CommandEvidenceKind::Applied { applied_epoch: 3 },
            ),
        ];

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::StaleApplicationEpochAccepted {
                request_id: 11,
                expected_epoch: 4,
                applied_epoch: 3,
            })
        );
    }

    #[test]
    fn expired_application_is_a_hard_failure() {
        let identity = identity();
        let events = vec![
            event(&identity, 90, CommandEvidenceKind::Proposed),
            event(&identity, 91, CommandEvidenceKind::RuntimeAdmitted),
            event(
                &identity,
                92,
                CommandEvidenceKind::NativeAccepted { accepted_epoch: 4 },
            ),
            event(
                &identity,
                100,
                CommandEvidenceKind::Applied { applied_epoch: 4 },
            ),
        ];

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::ExpiredApplication {
                request_id: 11,
                deadline_ns: 100,
                applied_at_ns: 100,
            })
        );
    }

    #[test]
    fn timestamp_regression_fails_closed() {
        let identity = identity();
        let events = vec![
            event(&identity, 20, CommandEvidenceKind::Proposed),
            event(&identity, 19, CommandEvidenceKind::RuntimeAdmitted),
        ];

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::TimestampRegression {
                request_id: 11,
                previous_ns: 20,
                observed_ns: 19,
            })
        );
    }

    #[test]
    fn changed_identity_or_clock_domain_fails_closed() {
        let identity = identity();
        let mut events = successful_trace(&identity);
        events[2].identity.clock_domain_id = "wall-clock".into();

        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::IdentityMismatch { request_id: 11 })
        );
    }

    #[test]
    fn command_or_controller_configuration_digest_drift_fails_closed() {
        let identity = identity();
        let mut events = successful_trace(&identity);
        events[3].identity.command_payload_digest = "sha256:mutated-command".into();

        assert_eq!(
            audit_command_evidence(&[identity.clone()], &events),
            Err(CommandEvidenceAuditError::IdentityMismatch { request_id: 11 })
        );

        let mut events = successful_trace(&identity);
        events[3].identity.controller_config_digest = "sha256:mutated-config".into();
        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::IdentityMismatch { request_id: 11 })
        );
    }

    #[test]
    fn missing_trace_and_missing_terminal_outcome_are_not_zero_failures() {
        let identity = identity();
        assert_eq!(
            audit_command_evidence(std::slice::from_ref(&identity), &[]),
            Err(CommandEvidenceAuditError::MissingTrace { request_id: 11 })
        );

        let mut events = successful_trace(&identity);
        events.pop();
        assert_eq!(
            audit_command_evidence(&[identity], &events),
            Err(CommandEvidenceAuditError::MissingTerminalOutcome {
                request_id: 11,
                phase: CommandEvidencePhase::Applied,
            })
        );
    }

    #[test]
    fn unexpected_request_and_duplicate_expected_identity_are_rejected() {
        let identity = identity();
        let mut unexpected = identity.clone();
        unexpected.request_id = 99;
        let events = vec![event(&unexpected, 10, CommandEvidenceKind::Proposed)];

        assert_eq!(
            audit_command_evidence(&[identity.clone()], &events),
            Err(CommandEvidenceAuditError::UnexpectedRequest { request_id: 99 })
        );
        assert_eq!(
            audit_command_evidence(&[identity.clone(), identity.clone()], &[]),
            Err(CommandEvidenceAuditError::DuplicateExpectedRequestId { request_id: 11 })
        );
    }

    #[test]
    fn event_after_terminal_and_outcome_mismatch_fail_closed() {
        let identity = identity();
        let mut events = successful_trace(&identity);
        events.push(event(&identity, 15, CommandEvidenceKind::Cancelled));
        assert_eq!(
            audit_command_evidence(&[identity.clone()], &events),
            Err(CommandEvidenceAuditError::EventAfterTerminal { request_id: 11 })
        );

        let rejected = vec![
            event(&identity, 10, CommandEvidenceKind::Proposed),
            event(
                &identity,
                11,
                CommandEvidenceKind::Rejected {
                    stage: CommandRejectionStage::RuntimeAdmission,
                },
            ),
            event(
                &identity,
                12,
                CommandEvidenceKind::OutcomeRecorded {
                    outcome: CommandTerminalOutcome::Succeeded,
                },
            ),
        ];
        assert_eq!(
            audit_command_evidence(&[identity], &rejected),
            Err(CommandEvidenceAuditError::OutcomeDoesNotMatchDisposition {
                request_id: 11,
                phase: CommandEvidencePhase::RejectedAwaitingOutcome,
                outcome: CommandTerminalOutcome::Succeeded,
            })
        );
    }
}
