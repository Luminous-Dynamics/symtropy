// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! A pure state ledger for feeder actuator proposals and reconciliation.
//!
//! This module does not communicate with or command hardware. It models the
//! authority boundary a future transport must satisfy.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeederSetpoints {
    pub gamma: f64,
    pub feeder_speed_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissioningEnvelope {
    pub profile_id: String,
    pub profile_revision: String,
    pub gamma_min: f64,
    pub gamma_max: f64,
    pub feeder_speed_min_fraction: f64,
    pub feeder_speed_max_fraction: f64,
    pub gamma_tracking_tolerance_fraction: f64,
    pub feeder_tracking_tolerance_fraction: f64,
}

impl CommissioningEnvelope {
    pub fn validate(&self) -> Result<(), String> {
        if self.profile_id.trim().is_empty() || self.profile_revision.trim().is_empty() {
            return Err("commissioning profile ID and revision are required".to_owned());
        }

        if !self.gamma_min.is_finite() || self.gamma_min < 0.0 {
            return Err("gamma_min must be finite and non-negative".to_owned());
        }
        if !self.gamma_max.is_finite() || self.gamma_max <= 0.0 {
            return Err("gamma_max must be finite and positive".to_owned());
        }
        if self.gamma_min > self.gamma_max {
            return Err("gamma_min must not exceed gamma_max".to_owned());
        }

        for (value, name) in [
            (self.feeder_speed_min_fraction, "feeder_speed_min_fraction"),
            (self.feeder_speed_max_fraction, "feeder_speed_max_fraction"),
            (
                self.gamma_tracking_tolerance_fraction,
                "gamma_tracking_tolerance_fraction",
            ),
            (
                self.feeder_tracking_tolerance_fraction,
                "feeder_tracking_tolerance_fraction",
            ),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!("{name} must be finite and in [0, 1]"));
            }
        }
        if self.feeder_speed_min_fraction > self.feeder_speed_max_fraction {
            return Err("feeder speed minimum must not exceed maximum".to_owned());
        }
        Ok(())
    }

    fn validate_setpoints(&self, setpoints: &FeederSetpoints) -> Result<(), String> {
        if !setpoints.gamma.is_finite()
            || setpoints.gamma < self.gamma_min
            || setpoints.gamma > self.gamma_max
        {
            return Err("Gamma setpoint is outside the commissioned envelope".to_owned());
        }
        if !setpoints.feeder_speed_fraction.is_finite()
            || setpoints.feeder_speed_fraction < self.feeder_speed_min_fraction
            || setpoints.feeder_speed_fraction > self.feeder_speed_max_fraction
        {
            return Err("feeder speed setpoint is outside the commissioned envelope".to_owned());
        }
        Ok(())
    }

    fn tracks(&self, expected: &FeederSetpoints, measured: &FeederSetpoints) -> bool {
        let gamma_error =
            (measured.gamma - expected.gamma).abs() / expected.gamma.abs().max(1.0e-9);
        let feeder_error = (measured.feeder_speed_fraction - expected.feeder_speed_fraction).abs()
            / expected.feeder_speed_fraction.abs().max(1.0e-9);
        gamma_error <= self.gamma_tracking_tolerance_fraction
            && feeder_error <= self.feeder_tracking_tolerance_fraction
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndependentActuatorReadback {
    pub setpoints: FeederSetpoints,
    pub device_id: String,
    pub calibration_record_id: String,
    pub observed_at_utc: String,
    pub readback_artifact_sha256: String,
    pub fresh: bool,
    pub calibration_current: bool,
    pub independent: bool,
}

impl IndependentActuatorReadback {
    fn validate(&self, envelope: &CommissioningEnvelope) -> Result<(), String> {
        if !self.fresh || !self.calibration_current || !self.independent {
            return Err(
                "readback must be fresh, independently observed and currently calibrated"
                    .to_owned(),
            );
        }
        if self.device_id.trim().is_empty()
            || self.calibration_record_id.trim().is_empty()
            || self.observed_at_utc.trim().is_empty()
        {
            return Err("readback device, calibration and timestamp identities are required".to_owned());
        }
        if self.readback_artifact_sha256.len() != 64
            || !self
                .readback_artifact_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err("readback artifact must have a lowercase SHA-256 digest".to_owned());
        }
        envelope.validate_setpoints(&self.setpoints)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActuatorCommand {
    pub sequence: u64,
    pub command_id: String,
    pub commissioning_profile_id: String,
    pub commissioning_profile_revision: String,
    pub previous_acknowledged_sequence: u64,
    pub previous_acknowledged: FeederSetpoints,
    pub requested: FeederSetpoints,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorResponseStatus {
    Pending,
    UnknownOutcome,
    Applied,
    RejectedNoEffect,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorResponse {
    pub sequence: u64,
    pub command_id: String,
    pub status: ActuatorResponseStatus,
    pub readback: Option<IndependentActuatorReadback>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcknowledgedResolution {
    InitialVerifiedState,
    AppliedWithIndependentReadback,
    RejectedWithNoEffect,
    ReconciledObservedState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcknowledgedState {
    pub sequence: u64,
    pub setpoints: FeederSetpoints,
    pub resolution: AcknowledgedResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LedgerState {
    Acknowledged(AcknowledgedState),
    CommandPending {
        command: ActuatorCommand,
    },
    UnknownOutcome {
        command: ActuatorCommand,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LedgerResolution {
    AwaitingResponse,
    CommandApplied,
    CommandRejectedNoEffect,
    OutcomeUnknown,
    ReconciledObservedState,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandLedger {
    envelope: CommissioningEnvelope,
    state: LedgerState,
    next_sequence: u64,
}

impl CommandLedger {
    pub fn new(
        envelope: CommissioningEnvelope,
        initial_readback: IndependentActuatorReadback,
    ) -> Result<Self, String> {
        envelope.validate()?;
        initial_readback.validate(&envelope)?;
        Ok(Self {
            envelope,
            state: LedgerState::Acknowledged(AcknowledgedState {
                sequence: 0,
                setpoints: initial_readback.setpoints,
                resolution: AcknowledgedResolution::InitialVerifiedState,
            }),
            next_sequence: 1,
        })
    }

    pub fn propose(&mut self, requested: FeederSetpoints) -> Result<ActuatorCommand, String> {
        self.envelope.validate_setpoints(&requested)?;
        let acknowledged = match &self.state {
            LedgerState::Acknowledged(state) => state,
            LedgerState::CommandPending { .. } => {
                return Err("a command is already pending; no new mutation is allowed".to_owned());
            }
            LedgerState::UnknownOutcome { .. } => {
                return Err(
                    "command outcome is unknown; reconcile actuator state before any new mutation"
                        .to_owned(),
                );
            }
        };

        if acknowledged.setpoints == requested {
            return Err("requested setpoints equal the acknowledged actuator state".to_owned());
        }

        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| "actuator command sequence exhausted".to_owned())?;

        let command = ActuatorCommand {
            sequence,
            command_id: format!("feeder-command-{sequence:016}"),
            commissioning_profile_id: self.envelope.profile_id.clone(),
            commissioning_profile_revision: self.envelope.profile_revision.clone(),
            previous_acknowledged_sequence: acknowledged.sequence,
            previous_acknowledged: acknowledged.setpoints.clone(),
            requested,
        };
        self.state = LedgerState::CommandPending {
            command: command.clone(),
        };
        Ok(command)
    }

    pub fn apply_response(
        &mut self,
        response: ActuatorResponse,
    ) -> Result<LedgerResolution, String> {
        let (command, previous_acknowledged_sequence, previous_acknowledged, was_unknown) = match &self.state {
            LedgerState::CommandPending { command } => (
                command.clone(),
                command.previous_acknowledged_sequence,
                command.previous_acknowledged.clone(),
                false,
            ),
            LedgerState::UnknownOutcome { command, .. } => (
                command.clone(),
                command.previous_acknowledged_sequence,
                command.previous_acknowledged.clone(),
                true,
            ),
            LedgerState::Acknowledged(_) => {
                return Err("there is no outstanding actuator command to resolve".to_owned());
            }
        };

        if response.sequence != command.sequence || response.command_id != command.command_id {
            return Err("response sequence or command ID does not match the outstanding command".to_owned());
        }

        match response.status {
            ActuatorResponseStatus::Pending => {
                if was_unknown {
                    Ok(LedgerResolution::OutcomeUnknown)
                } else {
                    Ok(LedgerResolution::AwaitingResponse)
                }
            }
            ActuatorResponseStatus::UnknownOutcome => {
                self.state = LedgerState::UnknownOutcome {
                    command,
                    reason: response.reason.unwrap_or_else(|| {
                        "transport or actuator outcome is unknown".to_owned()
                    }),
                };
                Ok(LedgerResolution::OutcomeUnknown)
            }
            ActuatorResponseStatus::Partial => {
                self.state = LedgerState::UnknownOutcome {
                    command,
                    reason: response.reason.unwrap_or_else(|| {
                        "partial actuator application requires independent reconciliation"
                            .to_owned()
                    }),
                };
                Ok(LedgerResolution::OutcomeUnknown)
            }
            ActuatorResponseStatus::Applied => {
                let Some(readback) = response.readback else {
                    self.state = LedgerState::UnknownOutcome {
                        command,
                        reason: "applied response lacked independent actuator readback".to_owned(),
                    };
                    return Ok(LedgerResolution::OutcomeUnknown);
                };

                if let Err(error) = readback.validate(&self.envelope) {
                    self.state = LedgerState::UnknownOutcome {
                        command,
                        reason: format!("applied response readback is not admissible: {error}"),
                    };
                    return Ok(LedgerResolution::OutcomeUnknown);
                }
                if !self.envelope.tracks(&command.requested, &readback.setpoints) {
                    self.state = LedgerState::UnknownOutcome {
                        command,
                        reason: "applied response readback differs from the requested setpoints"
                            .to_owned(),
                    };
                    return Ok(LedgerResolution::OutcomeUnknown);
                }

                self.state = LedgerState::Acknowledged(AcknowledgedState {
                    sequence: command.sequence,
                    setpoints: readback.setpoints,
                    resolution: AcknowledgedResolution::AppliedWithIndependentReadback,
                });
                Ok(LedgerResolution::CommandApplied)
            }
            ActuatorResponseStatus::RejectedNoEffect => {
                let Some(readback) = response.readback else {
                    self.state = LedgerState::UnknownOutcome {
                        command,
                        reason: "rejection did not include readback proving no effect".to_owned(),
                    };
                    return Ok(LedgerResolution::OutcomeUnknown);
                };

                if let Err(error) = readback.validate(&self.envelope) {
                    self.state = LedgerState::UnknownOutcome {
                        command,
                        reason: format!("rejection readback is not admissible: {error}"),
                    };
                    return Ok(LedgerResolution::OutcomeUnknown);
                }
                if !self.envelope.tracks(&previous_acknowledged, &readback.setpoints) {
                    self.state = LedgerState::UnknownOutcome {
                        command,
                        reason: "rejection readback does not match the previous acknowledged state"
                            .to_owned(),
                    };
                    return Ok(LedgerResolution::OutcomeUnknown);
                }

                self.state = LedgerState::Acknowledged(AcknowledgedState {
                    sequence: previous_acknowledged_sequence,
                    setpoints: readback.setpoints,
                    resolution: AcknowledgedResolution::RejectedWithNoEffect,
                });
                Ok(LedgerResolution::CommandRejectedNoEffect)
            }
        }
    }

    pub fn mark_unknown(&mut self, sequence: u64, command_id: &str, reason: &str) -> Result<(), String> {
        let command = match &self.state {
            LedgerState::CommandPending { command } => command.clone(),
            LedgerState::UnknownOutcome { command, .. } => {
                if command.sequence == sequence && command.command_id == command_id {
                    return Ok(());
                }
                return Err("unknown-outcome identity mismatch".to_owned());
            }
            LedgerState::Acknowledged(_) => {
                return Err("no pending command exists to mark unknown".to_owned());
            }
        };

        if command.sequence != sequence || command.command_id != command_id {
            return Err("timeout identity does not match the pending command".to_owned());
        }
        self.state = LedgerState::UnknownOutcome {
            command,
            reason: if reason.trim().is_empty() {
                "command timed out; physical outcome is unknown".to_owned()
            } else {
                reason.to_owned()
            },
        };
        Ok(())
    }

    pub fn reconcile(
        &mut self,
        readback: IndependentActuatorReadback,
    ) -> Result<LedgerResolution, String> {
        let command = match &self.state {
            LedgerState::UnknownOutcome { command, .. } => command.clone(),
            LedgerState::CommandPending { .. } => {
                return Err("mark the command outcome unknown before reconciliation".to_owned());
            }
            LedgerState::Acknowledged(_) => {
                return Err("there is no unresolved command to reconcile".to_owned());
            }
        };
        readback.validate(&self.envelope)?;
        self.state = LedgerState::Acknowledged(AcknowledgedState {
            sequence: command.sequence,
            setpoints: readback.setpoints,
            resolution: AcknowledgedResolution::ReconciledObservedState,
        });
        Ok(LedgerResolution::ReconciledObservedState)
    }

    pub fn state(&self) -> &LedgerState {
        &self.state
    }

    pub fn envelope(&self) -> &CommissioningEnvelope {
        &self.envelope
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> CommissioningEnvelope {
        CommissioningEnvelope {
            profile_id: "SYNTHETIC-HOPPER-FEEDER".to_owned(),
            profile_revision: "fixture-v1".to_owned(),
            gamma_min: 3.0,
            gamma_max: 10.0,
            feeder_speed_min_fraction: 0.1,
            feeder_speed_max_fraction: 1.0,
            gamma_tracking_tolerance_fraction: 0.05,
            feeder_tracking_tolerance_fraction: 0.05,
        }
    }

    fn points(gamma: f64, speed: f64) -> FeederSetpoints {
        FeederSetpoints {
            gamma,
            feeder_speed_fraction: speed,
        }
    }

    fn readback(gamma: f64, speed: f64) -> IndependentActuatorReadback {
        IndependentActuatorReadback {
            setpoints: points(gamma, speed),
            device_id: "SYNTHETIC-ACTUATOR".to_owned(),
            calibration_record_id: "SYNTHETIC-CALIBRATION".to_owned(),
            observed_at_utc: "2026-11-18T10:00:00Z".to_owned(),
            readback_artifact_sha256: "a".repeat(64),
            fresh: true,
            calibration_current: true,
            independent: true,
        }
    }

    fn ledger() -> CommandLedger {
        CommandLedger::new(envelope(), readback(4.0, 0.5)).unwrap()
    }

    fn response(command: &ActuatorCommand, status: ActuatorResponseStatus, readback: Option<IndependentActuatorReadback>) -> ActuatorResponse {
        ActuatorResponse {
            sequence: command.sequence,
            command_id: command.command_id.clone(),
            status,
            readback,
            reason: None,
        }
    }

    #[test]
    fn only_one_mutation_can_be_outstanding() {
        let mut ledger = ledger();
        ledger.propose(points(5.0, 0.5)).unwrap();
        assert!(ledger.propose(points(6.0, 0.6)).is_err());
    }

    #[test]
    fn pending_response_keeps_mutations_blocked() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.5)).unwrap();
        assert_eq!(
            ledger.apply_response(response(&cmd, ActuatorResponseStatus::Pending, None)).unwrap(),
            LedgerResolution::AwaitingResponse
        );
        assert!(ledger.propose(points(6.0, 0.6)).is_err());
    }

    #[test]
    fn unknown_outcome_blocks_new_mutations_until_readback_reconciliation() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.5)).unwrap();
        ledger
            .mark_unknown(cmd.sequence, &cmd.command_id, "transport timeout")
            .unwrap();
        assert!(ledger.propose(points(6.0, 0.6)).is_err());

        assert_eq!(
            ledger.reconcile(readback(5.0, 0.5)).unwrap(),
            LedgerResolution::ReconciledObservedState
        );
        let next = ledger.propose(points(6.0, 0.5)).unwrap();
        assert_eq!(next.sequence, cmd.sequence + 1);
    }

    #[test]
    fn applied_requires_matching_independent_readback() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.6)).unwrap();
        assert_eq!(
            ledger.apply_response(response(&cmd, ActuatorResponseStatus::Applied, Some(readback(5.0, 0.6)))).unwrap(),
            LedgerResolution::CommandApplied
        );
        assert!(matches!(
            ledger.state,
            LedgerState::Acknowledged(AcknowledgedState {
                sequence: 1,
                resolution: AcknowledgedResolution::AppliedWithIndependentReadback,
                ..
            })
        ));
    }

    #[test]
    fn readback_mismatch_turns_applied_response_into_unknown() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.6)).unwrap();
        assert_eq!(
            ledger.apply_response(response(&cmd, ActuatorResponseStatus::Applied, Some(readback(5.0, 0.5)))).unwrap(),
            LedgerResolution::OutcomeUnknown
        );
        assert!(matches!(ledger.state, LedgerState::UnknownOutcome { .. }));
    }

    #[test]
    fn stale_or_uncalibrated_readback_cannot_reconcile() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.5)).unwrap();
        ledger.mark_unknown(cmd.sequence, &cmd.command_id, "lost response").unwrap();
        let mut stale = readback(5.0, 0.5);
        stale.fresh = false;
        assert!(ledger.reconcile(stale).is_err());
        assert!(matches!(ledger.state, LedgerState::UnknownOutcome { .. }));
    }

    #[test]
    fn rejection_requires_readback_proving_no_effect() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.5)).unwrap();
        assert_eq!(
            ledger.apply_response(response(&cmd, ActuatorResponseStatus::RejectedNoEffect, Some(readback(4.0, 0.5)))).unwrap(),
            LedgerResolution::CommandRejectedNoEffect
        );
        let next = ledger.propose(points(6.0, 0.5)).unwrap();
        assert_eq!(next.sequence, cmd.sequence + 1);
    }

    #[test]
    fn repeated_no_effect_rejections_preserve_the_last_acknowledged_sequence() {
        let mut ledger = ledger();
        let first = ledger.propose(points(5.0, 0.5)).unwrap();
        ledger
            .apply_response(response(
                &first,
                ActuatorResponseStatus::RejectedNoEffect,
                Some(readback(4.0, 0.5)),
            ))
            .unwrap();

        let second = ledger.propose(points(6.0, 0.5)).unwrap();
        assert_eq!(second.sequence, 2);
        assert_eq!(second.previous_acknowledged_sequence, 0);
        ledger
            .apply_response(response(
                &second,
                ActuatorResponseStatus::RejectedNoEffect,
                Some(readback(4.0, 0.5)),
            ))
            .unwrap();

        assert!(matches!(
            &ledger.state,
            LedgerState::Acknowledged(AcknowledgedState { sequence: 0, .. })
        ));
        assert_eq!(ledger.next_sequence(), 3);
    }

    #[test]
    fn rejection_without_no_effect_readback_remains_unknown() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.5)).unwrap();
        assert_eq!(
            ledger.apply_response(response(&cmd, ActuatorResponseStatus::RejectedNoEffect, None)).unwrap(),
            LedgerResolution::OutcomeUnknown
        );
        assert!(ledger.propose(points(6.0, 0.5)).is_err());
    }

    #[test]
    fn wrong_or_duplicate_response_identity_is_rejected() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.5)).unwrap();
        let mut wrong = response(&cmd, ActuatorResponseStatus::Applied, Some(readback(5.0, 0.5)));
        wrong.command_id = "different-id".to_owned();
        assert!(ledger.apply_response(wrong).is_err());
        ledger.apply_response(response(&cmd, ActuatorResponseStatus::Applied, Some(readback(5.0, 0.5)))).unwrap();
        assert!(ledger.apply_response(response(&cmd, ActuatorResponseStatus::Applied, Some(readback(5.0, 0.5)))).is_err());
    }

    #[test]
    fn partial_response_requires_reconciliation() {
        let mut ledger = ledger();
        let cmd = ledger.propose(points(5.0, 0.6)).unwrap();
        assert_eq!(
            ledger.apply_response(response(&cmd, ActuatorResponseStatus::Partial, None)).unwrap(),
            LedgerResolution::OutcomeUnknown
        );
        assert!(ledger.propose(points(6.0, 0.6)).is_err());
        ledger.reconcile(readback(5.0, 0.5)).unwrap();
    }
}
