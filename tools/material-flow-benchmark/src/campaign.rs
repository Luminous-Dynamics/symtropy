// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Executable protocol-level fault campaign for the MFB handoff state machine.
//!
//! These cases inject typed events into the pure transaction protocol. They do
//! not inject faults into a simulator, sensor, actuator, physical carrier or
//! deployed Holochain conductor. The report therefore qualifies protocol
//! transitions only, not physical fault tolerance.

use serde::Serialize;

use crate::transaction::{
    DispatchGate, EventDisposition, PhysicalTransaction, PhysicalTransactionState,
    PreflightHoldReason, PreflightRejectionReason, TransactionContext, TransactionEvent,
    TransactionEventKind,
};
use crate::{
    ActorRole, AuthorizationAction, ExpectedTerminalState, FaultCampaignManifest, FaultEpisode,
    FaultKind, ScenarioManifest, UseLimit,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CampaignTraceStep {
    pub action: String,
    pub disposition: String,
    pub transaction_state: PhysicalTransactionState,
    pub dispatch_issued: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaultEpisodeResult {
    pub episode_id: String,
    pub fault_id: String,
    pub fault_kind: FaultKind,
    pub expected_terminal_state: ExpectedTerminalState,
    pub observed_transaction_state: PhysicalTransactionState,
    pub dispatch_issued: bool,
    pub duplicate_command_suppressed: bool,
    pub duplicate_physical_observation_reconciled: bool,
    pub blind_retry_blocked: bool,
    pub accepted: bool,
    pub trace: Vec<CampaignTraceStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaultCampaignResult {
    pub schema_version: u16,
    pub campaign_id: String,
    pub base_scenario_id: String,
    pub base_scenario_source_commit: String,
    pub seed: u64,
    pub execution_kind: String,
    pub episode_count: usize,
    pub accepted_episode_count: usize,
    pub all_expected_outcomes_observed: bool,
    pub pre_dispatch_faults_never_dispatched: bool,
    pub post_dispatch_ambiguities_blocked_retry: bool,
    pub no_episode_false_completed: bool,
    pub episodes: Vec<FaultEpisodeResult>,
    pub limitations: Vec<String>,
}

fn disposition_name(value: EventDisposition) -> &'static str {
    match value {
        EventDisposition::Applied => "applied",
        EventDisposition::DuplicateIgnored => "duplicate_ignored",
        EventDisposition::Rejected => "rejected",
        EventDisposition::SafeHold => "safe_hold",
        EventDisposition::ReconciliationRequired => "reconciliation_required",
        EventDisposition::Completed => "completed",
        EventDisposition::Reconciled => "reconciled",
    }
}

fn gate_name(value: DispatchGate) -> &'static str {
    match value {
        DispatchGate::DispatchOnce => "dispatch_once",
        DispatchGate::DuplicateSuppressed => "duplicate_suppressed",
        DispatchGate::BlockedForReconciliation => "blocked_for_reconciliation",
        DispatchGate::BlockedByUnexpectedCommand => "blocked_by_unexpected_command",
        DispatchGate::BlockedByState(_) => "blocked_by_state",
    }
}

fn emit(
    transaction: &mut PhysicalTransaction,
    trace: &mut Vec<CampaignTraceStep>,
    sequence: u64,
    event_id: String,
    actor_id: &str,
    action: &str,
    kind: TransactionEventKind,
) -> Result<EventDisposition, String> {
    let disposition = transaction.record_event(TransactionEvent {
        transaction_id: transaction.context.transaction_id.clone(),
        event_id,
        sequence,
        actor_id: actor_id.to_string(),
        kind,
    })?;
    trace.push(CampaignTraceStep {
        action: action.to_string(),
        disposition: disposition_name(disposition).to_string(),
        transaction_state: transaction.state,
        dispatch_issued: transaction.dispatch_issued,
    });
    Ok(disposition)
}

fn make_transaction(
    scenario: &ScenarioManifest,
    episode: &FaultEpisode,
) -> Result<PhysicalTransaction, String> {
    let authorization = &scenario.authorization;
    let reconciler = scenario
        .actors
        .iter()
        .find(|actor| {
            actor.role == ActorRole::Custodian
                && actor.actor_id != authorization.principal_actor_id
                && actor.actor_id != authorization.audience_actor_id
                && actor.actor_id != authorization.receiver_device_actor_id
        })
        .ok_or_else(|| {
            "protocol fault campaign requires a declared independent Custodian actor".to_string()
        })?;

    Ok(PhysicalTransaction::new(TransactionContext {
        transaction_id: format!("tx-{}-{}", scenario.scenario_id, episode.episode_id),
        expected_command_id: format!("cmd-{}", episode.episode_id),
        package_id: scenario.package.package_id.clone(),
        principal_actor_id: authorization.principal_actor_id.clone(),
        source_actor_id: authorization.audience_actor_id.clone(),
        receiver_actor_id: authorization.receiver_device_actor_id.clone(),
        receiver_endpoint_id: authorization.receiver_endpoint_node_id.clone(),
        reconciler_actor_id: reconciler.actor_id.clone(),
    }))
}

fn authorize(
    scenario: &ScenarioManifest,
    transaction: &mut PhysicalTransaction,
    trace: &mut Vec<CampaignTraceStep>,
    episode_id: &str,
    expired: bool,
) -> Result<(), String> {
    let authorization = &scenario.authorization;
    let scope_matches = authorization
        .actions
        .contains(&AuthorizationAction::ReceiveExternalPackage)
        && authorization
            .actions
            .contains(&AuthorizationAction::TransferToTote);
    let use_remaining = match authorization.use_limit {
        UseLimit::OneShot => true,
        UseLimit::Bounded { maximum_uses } => maximum_uses > 0,
    };
    let checked_at_unix_ms = if expired {
        authorization.expires_at_unix_ms
    } else {
        scenario.started_at_unix_ms
    };
    let disposition = emit(
        transaction,
        trace,
        1,
        format!("event-{episode_id}-authorization"),
        &authorization.principal_actor_id,
        if expired {
            "inject_expired_authorization"
        } else {
            "authorize_transaction"
        },
        TransactionEventKind::AuthorizationGranted {
            not_before_unix_ms: authorization.not_before_unix_ms,
            expires_at_unix_ms: authorization.expires_at_unix_ms,
            checked_at_unix_ms,
            audience_matches: true,
            endpoint_matches: true,
            scope_matches,
            use_remaining,
        },
    )?;
    if expired {
        if disposition != EventDisposition::Rejected {
            return Err("expired-grant negative control did not reject the grant".into());
        }
    } else if disposition != EventDisposition::Applied {
        return Err(format!(
            "valid campaign authorization was not applied: {}",
            disposition_name(disposition)
        ));
    }
    Ok(())
}

fn issue_release(
    transaction: &mut PhysicalTransaction,
    trace: &mut Vec<CampaignTraceStep>,
    episode_id: &str,
) -> Result<bool, String> {
    let command_id = transaction.context.expected_command_id.clone();
    let first = transaction.request_release_command(&command_id)?;
    trace.push(CampaignTraceStep {
        action: "request_first_release_command".into(),
        disposition: gate_name(first).into(),
        transaction_state: transaction.state,
        dispatch_issued: transaction.dispatch_issued,
    });
    if first != DispatchGate::DispatchOnce {
        return Err(format!(
            "first release command was not issued once: {first:?}"
        ));
    }

    let second = transaction.request_release_command(&command_id)?;
    trace.push(CampaignTraceStep {
        action: "replay_same_release_command".into(),
        disposition: gate_name(second).into(),
        transaction_state: transaction.state,
        dispatch_issued: transaction.dispatch_issued,
    });
    if second != DispatchGate::DuplicateSuppressed {
        return Err(format!(
            "same command replay was not suppressed for {episode_id}: {second:?}"
        ));
    }
    Ok(true)
}

fn record_release(
    scenario: &ScenarioManifest,
    transaction: &mut PhysicalTransaction,
    trace: &mut Vec<CampaignTraceStep>,
    episode_id: &str,
) -> Result<(), String> {
    let authorization = &scenario.authorization;
    let disposition = emit(
        transaction,
        trace,
        2,
        format!("event-{episode_id}-package-released"),
        &authorization.audience_actor_id,
        "observe_source_release",
        TransactionEventKind::PackageReleased {
            command_id: transaction.context.expected_command_id.clone(),
            package_id: scenario.package.package_id.clone(),
        },
    )?;
    if disposition != EventDisposition::Applied {
        return Err(format!(
            "source release observation did not apply: {}",
            disposition_name(disposition)
        ));
    }
    Ok(())
}

fn run_episode(
    scenario: &ScenarioManifest,
    episode: &FaultEpisode,
) -> Result<FaultEpisodeResult, String> {
    let authorization = &scenario.authorization;
    let mut transaction = make_transaction(scenario, episode)?;
    let mut trace = Vec::new();
    let mut duplicate_command_suppressed = false;
    let mut duplicate_physical_observation_reconciled = false;
    let mut blind_retry_blocked = false;

    match episode.injection.kind {
        FaultKind::WrongTote => {
            let disposition = emit(
                &mut transaction,
                &mut trace,
                1,
                format!("event-{}-wrong-tote", episode.episode_id),
                &authorization.receiver_device_actor_id,
                "reject_wrong_container_before_dispatch",
                TransactionEventKind::PreflightRejected {
                    reason: PreflightRejectionReason::WrongContainer,
                },
            )?;
            if disposition != EventDisposition::Rejected {
                return Err("wrong-tote preflight check did not reject".into());
            }
        }
        FaultKind::OverMass => {
            let disposition = emit(
                &mut transaction,
                &mut trace,
                1,
                format!("event-{}-over-mass", episode.episode_id),
                &authorization.receiver_device_actor_id,
                "reject_payload_mass_before_dispatch",
                TransactionEventKind::PreflightRejected {
                    reason: PreflightRejectionReason::PayloadMassExceeded,
                },
            )?;
            if disposition != EventDisposition::Rejected {
                return Err("over-mass preflight check did not reject".into());
            }
        }
        FaultKind::ExpiredGrant => {
            authorize(
                scenario,
                &mut transaction,
                &mut trace,
                &episode.episode_id,
                true,
            )?;
        }
        FaultKind::StaleTemperature => {
            let disposition = emit(
                &mut transaction,
                &mut trace,
                1,
                format!("event-{}-stale-temperature", episode.episode_id),
                &authorization.receiver_device_actor_id,
                "safe_hold_on_stale_temperature_observation",
                TransactionEventKind::PreflightSafeHold {
                    reason: PreflightHoldReason::TemperatureObservationStale,
                },
            )?;
            if disposition != EventDisposition::SafeHold {
                return Err("stale-temperature preflight check did not enter safe hold".into());
            }
        }
        FaultKind::BlockedTransferGate => {
            let disposition = emit(
                &mut transaction,
                &mut trace,
                1,
                format!("event-{}-blocked-gate", episode.episode_id),
                &authorization.receiver_device_actor_id,
                "safe_hold_on_interlocked_transfer_gate",
                TransactionEventKind::PreflightSafeHold {
                    reason: PreflightHoldReason::TransferGateInterlocked,
                },
            )?;
            if disposition != EventDisposition::SafeHold {
                return Err("blocked gate did not enter safe hold".into());
            }
        }
        FaultKind::DamagedSeal => {
            let disposition = emit(
                &mut transaction,
                &mut trace,
                1,
                format!("event-{}-damaged-seal", episode.episode_id),
                &authorization.receiver_device_actor_id,
                "safe_hold_on_damaged_or_unknown_seal",
                TransactionEventKind::PreflightSafeHold {
                    reason: PreflightHoldReason::SealNotIntact,
                },
            )?;
            if disposition != EventDisposition::SafeHold {
                return Err("damaged seal did not enter safe hold".into());
            }
        }
        FaultKind::DuplicateHandoff => {
            authorize(
                scenario,
                &mut transaction,
                &mut trace,
                &episode.episode_id,
                false,
            )?;
            duplicate_command_suppressed =
                issue_release(&mut transaction, &mut trace, &episode.episode_id)?;
            record_release(scenario, &mut transaction, &mut trace, &episode.episode_id)?;
            let duplicate_command_id = transaction.context.expected_command_id.clone();
            let disposition = emit(
                &mut transaction,
                &mut trace,
                3,
                format!("event-{}-duplicate-release-observation", episode.episode_id),
                &authorization.audience_actor_id,
                "inject_second_physical_release_observation",
                TransactionEventKind::PackageReleased {
                    command_id: duplicate_command_id,
                    package_id: scenario.package.package_id.clone(),
                },
            )?;
            duplicate_physical_observation_reconciled =
                disposition == EventDisposition::ReconciliationRequired;
            let retry = transaction
                .request_release_command(&transaction.context.expected_command_id.clone())?;
            blind_retry_blocked = matches!(
                retry,
                DispatchGate::BlockedByState(PhysicalTransactionState::ReconciliationRequired)
            );
            trace.push(CampaignTraceStep {
                action: "attempt_release_retry_after_duplicate_observation".into(),
                disposition: gate_name(retry).into(),
                transaction_state: transaction.state,
                dispatch_issued: transaction.dispatch_issued,
            });
        }
        FaultKind::MissingReceiverAcknowledgement
        | FaultKind::PowerInterruption
        | FaultKind::NetworkLoss => {
            authorize(
                scenario,
                &mut transaction,
                &mut trace,
                &episode.episode_id,
                false,
            )?;
            duplicate_command_suppressed =
                issue_release(&mut transaction, &mut trace, &episode.episode_id)?;
            record_release(scenario, &mut transaction, &mut trace, &episode.episode_id)?;
            let (action, kind) = match episode.injection.kind {
                FaultKind::MissingReceiverAcknowledgement => (
                    "timeout_waiting_for_receiver_acknowledgement",
                    TransactionEventKind::ActuationTimeout,
                ),
                FaultKind::PowerInterruption => (
                    "inject_power_interruption_after_release",
                    TransactionEventKind::PowerInterruption,
                ),
                FaultKind::NetworkLoss => (
                    "inject_network_partition_after_release",
                    TransactionEventKind::NetworkPartition,
                ),
                _ => unreachable!("matched post-dispatch fault family"),
            };
            let disposition = emit(
                &mut transaction,
                &mut trace,
                3,
                format!("event-{}-post-dispatch-fault", episode.episode_id),
                &authorization.receiver_device_actor_id,
                action,
                kind,
            )?;
            if disposition != EventDisposition::ReconciliationRequired {
                return Err(format!(
                    "post-dispatch ambiguity did not require reconciliation: {}",
                    disposition_name(disposition)
                ));
            }
            let retry = transaction
                .request_release_command(&transaction.context.expected_command_id.clone())?;
            blind_retry_blocked = matches!(
                retry,
                DispatchGate::BlockedByState(PhysicalTransactionState::ReconciliationRequired)
            );
            trace.push(CampaignTraceStep {
                action: "attempt_release_retry_after_ambiguous_outcome".into(),
                disposition: gate_name(retry).into(),
                transaction_state: transaction.state,
                dispatch_issued: transaction.dispatch_issued,
            });
        }
    }

    let expected = episode.expected_terminal_state;
    let observed = transaction.state;
    let terminal_matches = match expected {
        ExpectedTerminalState::Rejected => observed == PhysicalTransactionState::Rejected,
        ExpectedTerminalState::SafeHold => observed == PhysicalTransactionState::SafeHold,
        ExpectedTerminalState::ReconciliationRequired => {
            observed == PhysicalTransactionState::ReconciliationRequired
        }
    };
    let pre_dispatch = matches!(
        episode.injection.kind,
        FaultKind::WrongTote
            | FaultKind::OverMass
            | FaultKind::StaleTemperature
            | FaultKind::BlockedTransferGate
            | FaultKind::DamagedSeal
            | FaultKind::ExpiredGrant
    );
    let post_dispatch = matches!(
        episode.injection.kind,
        FaultKind::DuplicateHandoff
            | FaultKind::MissingReceiverAcknowledgement
            | FaultKind::PowerInterruption
            | FaultKind::NetworkLoss
    );
    let safety_control = if pre_dispatch {
        !transaction.dispatch_issued
    } else {
        transaction.dispatch_issued && blind_retry_blocked
    };
    let duplicate_control = episode.injection.kind != FaultKind::DuplicateHandoff
        || (duplicate_command_suppressed && duplicate_physical_observation_reconciled);
    let accepted = terminal_matches
        && safety_control
        && duplicate_control
        && (!post_dispatch || blind_retry_blocked)
        && observed != PhysicalTransactionState::Completed;

    Ok(FaultEpisodeResult {
        episode_id: episode.episode_id.clone(),
        fault_id: episode.injection.fault_id.clone(),
        fault_kind: episode.injection.kind,
        expected_terminal_state: expected,
        observed_transaction_state: observed,
        dispatch_issued: transaction.dispatch_issued,
        duplicate_command_suppressed,
        duplicate_physical_observation_reconciled,
        blind_retry_blocked,
        accepted,
        trace,
    })
}

/// Validate the scenario/campaign pairing, then execute each protocol fault
/// independently against a fresh physical-transaction state machine.
pub fn run_fault_campaign(
    scenario: &ScenarioManifest,
    campaign: &FaultCampaignManifest,
) -> Result<FaultCampaignResult, String> {
    scenario
        .validate()
        .map_err(|errors| format!("invalid base scenario: {}", errors.join("; ")))?;
    campaign
        .validate_against(scenario)
        .map_err(|errors| format!("invalid campaign: {}", errors.join("; ")))?;

    let mut episodes = Vec::with_capacity(campaign.episodes.len());
    for episode in &campaign.episodes {
        episodes.push(run_episode(scenario, episode)?);
    }

    let all_expected_outcomes_observed = episodes.iter().all(|episode| episode.accepted);
    let pre_dispatch_faults_never_dispatched = episodes
        .iter()
        .filter(|episode| {
            matches!(
                episode.fault_kind,
                FaultKind::WrongTote
                    | FaultKind::OverMass
                    | FaultKind::StaleTemperature
                    | FaultKind::BlockedTransferGate
                    | FaultKind::DamagedSeal
                    | FaultKind::ExpiredGrant
            )
        })
        .all(|episode| !episode.dispatch_issued);
    let post_dispatch_ambiguities_blocked_retry = episodes
        .iter()
        .filter(|episode| {
            matches!(
                episode.fault_kind,
                FaultKind::DuplicateHandoff
                    | FaultKind::MissingReceiverAcknowledgement
                    | FaultKind::PowerInterruption
                    | FaultKind::NetworkLoss
            )
        })
        .all(|episode| episode.dispatch_issued && episode.blind_retry_blocked);
    let no_episode_false_completed = episodes
        .iter()
        .all(|episode| episode.observed_transaction_state != PhysicalTransactionState::Completed);
    let accepted_episode_count = episodes.iter().filter(|episode| episode.accepted).count();

    Ok(FaultCampaignResult {
        schema_version: campaign.schema_version,
        campaign_id: campaign.campaign_id.clone(),
        base_scenario_id: scenario.scenario_id.clone(),
        base_scenario_source_commit: scenario.digests.source_commit.clone(),
        seed: scenario.seed,
        execution_kind: "deterministic_protocol_event_injection".into(),
        episode_count: episodes.len(),
        accepted_episode_count,
        all_expected_outcomes_observed,
        pre_dispatch_faults_never_dispatched,
        post_dispatch_ambiguities_blocked_retry,
        no_episode_false_completed,
        episodes,
        limitations: vec![
            "Injects typed events into a pure Rust handoff state machine; it does not inject physical sensor, actuator, power or network faults".into(),
            "Preflight rejection/hold observations are synthetic and do not authenticate real sensor or policy evidence".into(),
            "No physical robot, receiver, carrier, Holochain conductor, HIL rig or field system is exercised".into(),
            "A passed campaign qualifies only these protocol transitions and negative controls".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO: &str = include_str!("../fixtures/nominal-stationary-carrier.json");
    const CAMPAIGN: &str = include_str!("../fixtures/ten-fault-campaign.json");

    #[test]
    fn all_ten_protocol_fault_episodes_reach_their_expected_terminal_state() {
        let scenario: ScenarioManifest = serde_json::from_str(SCENARIO).expect("scenario fixture");
        let campaign: FaultCampaignManifest =
            serde_json::from_str(CAMPAIGN).expect("campaign fixture");
        let result =
            run_fault_campaign(&scenario, &campaign).expect("protocol campaign should execute");
        assert_eq!(result.episode_count, 10);
        assert_eq!(result.accepted_episode_count, 10);
        assert!(result.all_expected_outcomes_observed, "{result:?}");
        assert!(result.pre_dispatch_faults_never_dispatched, "{result:?}");
        assert!(result.post_dispatch_ambiguities_blocked_retry, "{result:?}");
        assert!(result.no_episode_false_completed, "{result:?}");
    }

    #[test]
    fn duplicate_handoff_suppresses_command_replay_but_reconciles_second_release() {
        let scenario: ScenarioManifest = serde_json::from_str(SCENARIO).expect("scenario fixture");
        let campaign: FaultCampaignManifest =
            serde_json::from_str(CAMPAIGN).expect("campaign fixture");
        let result =
            run_fault_campaign(&scenario, &campaign).expect("protocol campaign should execute");
        let episode = result
            .episodes
            .iter()
            .find(|episode| episode.fault_kind == FaultKind::DuplicateHandoff)
            .expect("duplicate-handoff case");
        assert!(episode.duplicate_command_suppressed);
        assert!(episode.duplicate_physical_observation_reconciled);
        assert!(episode.blind_retry_blocked);
        assert_eq!(
            episode.observed_transaction_state,
            PhysicalTransactionState::ReconciliationRequired
        );
    }

    #[test]
    fn power_and_network_faults_after_release_never_reissue_command() {
        let scenario: ScenarioManifest = serde_json::from_str(SCENARIO).expect("scenario fixture");
        let campaign: FaultCampaignManifest =
            serde_json::from_str(CAMPAIGN).expect("campaign fixture");
        let result =
            run_fault_campaign(&scenario, &campaign).expect("protocol campaign should execute");
        for episode in result.episodes.iter().filter(|episode| {
            matches!(
                episode.fault_kind,
                FaultKind::PowerInterruption | FaultKind::NetworkLoss
            )
        }) {
            assert!(episode.dispatch_issued);
            assert!(episode.blind_retry_blocked);
            assert_eq!(
                episode.observed_transaction_state,
                PhysicalTransactionState::ReconciliationRequired
            );
        }
    }
}
