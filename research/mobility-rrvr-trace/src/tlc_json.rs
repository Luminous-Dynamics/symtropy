// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Importer for TLC's native JSON counterexample format.
//!
//! TLC state values are preserved as opaque JSON values. The importer validates
//! the state/action chain, maps exact action names through the pinned RRVR
//! catalogue, and binds every imported event to the exact source JSON, model,
//! config and tool artifacts. It never infers physical semantics from a state
//! value or action source location.

use super::{
    Commitment, FailureSignature, TLA_ACTION_EVENT_MAPPING, TraceArtifact, TraceBundle, TraceEvent,
    TraceEventKind, TraceHeader, TraceValidation, validate_trace_bundle,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const TLC_JSON_FORMAT_VERSION: &str = "0.1";
pub const TLC_RAW_ARTIFACT_ID: &str = "tlc-raw-json";
pub const TLC_MODEL_ARTIFACT_ID: &str = "tla-model-source";
pub const TLC_CONFIG_ARTIFACT_ID: &str = "tla-model-config";
pub const TLC_TOOL_ARTIFACT_ID: &str = "tla2tools-jar";

/// Exact owner/context values required to turn a TLC trace into an RRVR bundle.
/// Source bytes are passed rather than precomputed digests so the importer
/// computes their SHA-256 commitments itself.
pub struct TlcImportContext<'a> {
    pub bundle_id: &'a str,
    pub system_id: &'a str,
    pub implementation_revision: &'a str,
    pub replay_seed: u64,
    pub property_id: &'a str,
    pub violation_class: &'a str,
    pub actor_id: &'a str,
    pub model_source_bytes: &'a [u8],
    pub model_config_bytes: &'a [u8],
    pub tla_tool_jar_bytes: &'a [u8],
    pub model_revision_label: &'a str,
    pub fault_profile_revision_label: &'a str,
    pub oracle_revision_label: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TlcImportError {
    InvalidContext,
    InvalidJson,
    MissingCounterexample,
    MissingField(&'static str),
    InvalidField(&'static str),
    MissingStateVariable,
    TooFewStates,
    TooManyStates,
    StateFrameShape(usize),
    StateIndexMismatch(usize),
    StateValueMustBeObject(usize),
    StateVariableMissingFromFrame(usize),
    ActionCountMismatch,
    ActionFrameShape(usize),
    ActionMetadataMissingName(usize),
    UnknownAction(String),
    DisconnectedTransition(usize),
    BundleValidation,
}

fn sha256(bytes: &[u8]) -> Commitment {
    let digest = Sha256::digest(bytes);
    let mut result = [0_u8; 32];
    result.copy_from_slice(&digest);
    result
}

fn hex(commitment: &Commitment) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in commitment {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, TlcImportError> {
    serde_json::to_vec(value).map_err(|_| TlcImportError::InvalidJson)
}

fn required<'a>(value: &'a Value, key: &'static str) -> Result<&'a Value, TlcImportError> {
    value.get(key).ok_or(TlcImportError::MissingField(key))
}

fn text_field<'a>(value: &'a Value, key: &'static str) -> Result<&'a str, TlcImportError> {
    required(value, key)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or(TlcImportError::InvalidField(key))
}

fn state_frame(value: &Value, expected_index: usize) -> Result<(), TlcImportError> {
    let pair = value
        .as_array()
        .filter(|items| items.len() == 2)
        .ok_or(TlcImportError::StateFrameShape(expected_index))?;
    let state_index = pair[0]
        .as_u64()
        .ok_or(TlcImportError::StateFrameShape(expected_index))?;
    if state_index != expected_index as u64 {
        return Err(TlcImportError::StateIndexMismatch(expected_index));
    }
    let state_value = pair[1]
        .as_object()
        .ok_or(TlcImportError::StateValueMustBeObject(expected_index))?;
    if !state_value.contains_key("s") {
        return Err(TlcImportError::StateVariableMissingFromFrame(
            expected_index,
        ));
    }
    Ok(())
}

fn action_event_kind(action_name: &str) -> Option<TraceEventKind> {
    // TLC exports the outer disjunction name Next for this model rather
    // than the selected inner action (for example, NaiveRetry). Preserve that
    // fact explicitly; do not guess the disjunct from the source location.
    if action_name == "Next" {
        return Some(TraceEventKind::TlaNextTransition);
    }
    TLA_ACTION_EVENT_MAPPING
        .iter()
        .find(|mapping| mapping.action_name == action_name)
        .map(|mapping| mapping.event_kind)
}

fn length_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// Import a TLC `-dumpTrace json` counterexample into a validated RRVR TraceBundle.
///
/// Supported TLC shape:
/// - `vars`: array of state-variable names;
/// - `counterexample.state`: `[[state_number, state_value], ...]`;
/// - `counterexample.action`: `[[before_state, action_metadata, after_state], ...]`.
///
/// State values and action metadata are kept as arbitrary JSON values when
/// hashing. The complete original JSON is retained by a raw-artifact commitment.
/// Sequential TLC transitions become an explicit linear predecessor chain; no
/// ordering is inferred from source locations or timestamps.
pub fn import_tlc_json_trace(
    raw_json: &[u8],
    context: &TlcImportContext<'_>,
) -> Result<TraceBundle, TlcImportError> {
    if [
        context.bundle_id,
        context.system_id,
        context.implementation_revision,
        context.property_id,
        context.violation_class,
        context.actor_id,
        context.model_revision_label,
        context.fault_profile_revision_label,
        context.oracle_revision_label,
    ]
    .iter()
    .any(|value| value.trim().is_empty())
        || context.model_source_bytes.is_empty()
        || context.model_config_bytes.is_empty()
        || context.tla_tool_jar_bytes.is_empty()
        || raw_json.is_empty()
    {
        return Err(TlcImportError::InvalidContext);
    }

    let root: Value = serde_json::from_slice(raw_json).map_err(|_| TlcImportError::InvalidJson)?;
    let vars = required(&root, "vars")?
        .as_array()
        .ok_or(TlcImportError::InvalidField("vars"))?;
    if !vars.iter().any(|value| value.as_str() == Some("s")) {
        return Err(TlcImportError::MissingStateVariable);
    }

    let counterexample =
        required(&root, "counterexample").map_err(|_| TlcImportError::MissingCounterexample)?;
    let state_values = required(counterexample, "state")?
        .as_array()
        .ok_or(TlcImportError::InvalidField("counterexample.state"))?;
    let action_values = required(counterexample, "action")?
        .as_array()
        .ok_or(TlcImportError::InvalidField("counterexample.action"))?;

    if state_values.len() < 2 {
        return Err(TlcImportError::TooFewStates);
    }
    if action_values.is_empty() {
        return Err(TlcImportError::TooFewStates);
    }
    if state_values.len() > super::MAX_TRACE_EVENTS + 1 {
        return Err(TlcImportError::TooManyStates);
    }
    if action_values.len() != state_values.len() - 1 {
        return Err(TlcImportError::ActionCountMismatch);
    }

    for (index, state) in state_values.iter().enumerate() {
        state_frame(state, index + 1)?;
    }

    let model_digest = sha256(context.model_source_bytes);
    let config_digest = sha256(context.model_config_bytes);
    let tool_digest = sha256(context.tla_tool_jar_bytes);
    let raw_digest = sha256(raw_json);

    let mut config_hasher = Sha256::new();
    length_prefixed(&mut config_hasher, &model_digest);
    length_prefixed(&mut config_hasher, &config_digest);
    length_prefixed(&mut config_hasher, &tool_digest);
    length_prefixed(&mut config_hasher, context.model_revision_label.as_bytes());
    length_prefixed(
        &mut config_hasher,
        context.fault_profile_revision_label.as_bytes(),
    );
    length_prefixed(&mut config_hasher, context.oracle_revision_label.as_bytes());
    let configuration_commitment: Commitment = config_hasher.finalize().into();

    let initial_state_commitment = sha256(&canonical_json(&state_values[0])?);
    let witness_state_commitment = sha256(&canonical_json(
        state_values.last().ok_or(TlcImportError::TooFewStates)?,
    )?);

    let artifact_ids = vec![
        TLC_RAW_ARTIFACT_ID.to_owned(),
        TLC_MODEL_ARTIFACT_ID.to_owned(),
        TLC_CONFIG_ARTIFACT_ID.to_owned(),
        TLC_TOOL_ARTIFACT_ID.to_owned(),
    ];

    let mut events = Vec::with_capacity(action_values.len());
    for (index, action_value) in action_values.iter().enumerate() {
        let action_frame = action_value
            .as_array()
            .filter(|items| items.len() == 3)
            .ok_or(TlcImportError::ActionFrameShape(index + 1))?;
        if action_frame[0] != state_values[index] || action_frame[2] != state_values[index + 1] {
            return Err(TlcImportError::DisconnectedTransition(index + 1));
        }

        let action_name = text_field(&action_frame[1], "name")
            .map_err(|_| TlcImportError::ActionMetadataMissingName(index + 1))?;
        let kind = action_event_kind(action_name)
            .ok_or_else(|| TlcImportError::UnknownAction(action_name.to_owned()))?;
        let sequence = index as u64 + 1;
        let event_id = format!("tlc-action-{:03}-{action_name}", index + 1);
        let predecessor_event_ids = if index == 0 {
            Vec::new()
        } else {
            vec![events[index - 1].event_id.clone()]
        };
        let input_commitment = sha256(&canonical_json(action_value)?);

        events.push(TraceEvent {
            event_id,
            logical_sequence: sequence,
            kind,
            actor_id: context.actor_id.to_owned(),
            predecessor_event_ids,
            clock_observation: None,
            raw_artifact_ids: artifact_ids.clone(),
            input_commitment,
            configuration_commitment,
        });
    }

    let witness_event_id = events
        .last()
        .ok_or(TlcImportError::TooFewStates)?
        .event_id
        .clone();

    let bundle = TraceBundle {
        header: TraceHeader {
            bundle_id: context.bundle_id.to_owned(),
            parent_bundle_id: None,
            format_version: super::CONTRACT_VERSION.to_owned(),
            system_id: context.system_id.to_owned(),
            implementation_revision: context.implementation_revision.to_owned(),
            model_revision: context.model_revision_label.to_owned(),
            fault_profile_revision: context.fault_profile_revision_label.to_owned(),
            oracle_revision: context.oracle_revision_label.to_owned(),
            replay_seed: context.replay_seed,
            initial_state_commitment,
            target_failure: FailureSignature {
                property_id: context.property_id.to_owned(),
                violation_class: context.violation_class.to_owned(),
                witness_event_id,
                witness_state_commitment,
            },
        },
        events,
        artifacts: vec![
            TraceArtifact {
                artifact_id: TLC_RAW_ARTIFACT_ID.to_owned(),
                media_type: "application/json".to_owned(),
                raw_artifact_commitment: raw_digest,
            },
            TraceArtifact {
                artifact_id: TLC_MODEL_ARTIFACT_ID.to_owned(),
                media_type: "text/plain".to_owned(),
                raw_artifact_commitment: model_digest,
            },
            TraceArtifact {
                artifact_id: TLC_CONFIG_ARTIFACT_ID.to_owned(),
                media_type: "text/plain".to_owned(),
                raw_artifact_commitment: config_digest,
            },
            TraceArtifact {
                artifact_id: TLC_TOOL_ARTIFACT_ID.to_owned(),
                media_type: "application/java-archive".to_owned(),
                raw_artifact_commitment: tool_digest,
            },
        ],
    };

    if validate_trace_bundle(&bundle) != TraceValidation::Valid {
        return Err(TlcImportError::BundleValidation);
    }
    Ok(bundle)
}

fn event_kind_name(kind: TraceEventKind) -> &'static str {
    match kind {
        TraceEventKind::ProcessStarted => "process_started",
        TraceEventKind::AttemptReserved => "attempt_reserved",
        TraceEventKind::DispatchStarted => "dispatch_started",
        TraceEventKind::NativeAccepted => "native_accepted",
        TraceEventKind::CommandQueued => "command_queued",
        TraceEventKind::ProcessCrashed => "process_crashed",
        TraceEventKind::ProcessRestarted => "process_restarted",
        TraceEventKind::ControllerQueueFenced => "controller_queue_fenced",
        TraceEventKind::DownstreamGraphFenced => "downstream_graph_fenced",
        TraceEventKind::OldCommandApplied => "old_command_applied",
        TraceEventKind::OldCommandNoEffect => "old_command_no_effect",
        TraceEventKind::OldCommandPartial => "old_command_partial",
        TraceEventKind::PersistenceCommitStarted => "persistence_commit_started",
        TraceEventKind::PersistenceCommitObserved => "persistence_commit_observed",
        TraceEventKind::PersistenceCommitConfirmed => "persistence_commit_confirmed",
        TraceEventKind::PersistenceNoCommitConfirmed => "persistence_no_commit_confirmed",
        TraceEventKind::PersistenceCommitAborted => "persistence_commit_aborted",
        TraceEventKind::PersistenceResolved => "persistence_resolved",
        TraceEventKind::PhysicalEffectObserved => "physical_effect_observed",
        TraceEventKind::PhysicalNoEffectObserved => "physical_no_effect_observed",
        TraceEventKind::PhysicalPartialObserved => "physical_partial_observed",
        TraceEventKind::PhysicalOutcomeUnknown => "physical_outcome_unknown",
        TraceEventKind::PhysicalOutcomeDisputed => "physical_outcome_disputed",
        TraceEventKind::MissionResolutionRecorded => "mission_resolution_recorded",
        TraceEventKind::RecoveryPermitIssued => "recovery_permit_issued",
        TraceEventKind::FreshTicketCreated => "fresh_ticket_created",
        TraceEventKind::NativeRecoveryAccepted => "native_recovery_accepted",
        TraceEventKind::RecoveryCommandApplied => "recovery_command_applied",
        TraceEventKind::RecoveryCommandNoEffect => "recovery_command_no_effect",
        TraceEventKind::RecoveryCommandPartial => "recovery_command_partial",
        TraceEventKind::M1AuthorityRevoked => "m1_authority_revoked",
        TraceEventKind::M2ApplicabilityRevoked => "m2_applicability_revoked",
        TraceEventKind::LiveBindingInvalidated => "live_binding_invalidated",
        TraceEventKind::NaiveRetryAttempted => "naive_retry_attempted",
        TraceEventKind::ObserverCommonCauseInjected => "observer_common_cause_injected",
        TraceEventKind::ObserverSampleCaptured => "observer_sample_captured",
        TraceEventKind::TelemetryCaptured => "telemetry_captured",
        TraceEventKind::TraceFlushed => "trace_flushed",
        TraceEventKind::TlaNextTransition => "tla_next_transition",
        TraceEventKind::Other => "other",
    }
}

pub fn trace_bundle_to_json_value(bundle: &TraceBundle) -> Value {
    let commitment_hex = |value: &Commitment| hex(value);
    json!({
        "header": {
            "bundle_id": bundle.header.bundle_id,
            "parent_bundle_id": bundle.header.parent_bundle_id,
            "format_version": bundle.header.format_version,
            "system_id": bundle.header.system_id,
            "implementation_revision": bundle.header.implementation_revision,
            "model_revision": bundle.header.model_revision,
            "fault_profile_revision": bundle.header.fault_profile_revision,
            "oracle_revision": bundle.header.oracle_revision,
            "replay_seed": bundle.header.replay_seed,
            "initial_state_commitment": commitment_hex(&bundle.header.initial_state_commitment),
            "target_failure": {
                "property_id": bundle.header.target_failure.property_id,
                "violation_class": bundle.header.target_failure.violation_class,
                "witness_event_id": bundle.header.target_failure.witness_event_id,
                "witness_state_commitment": commitment_hex(&bundle.header.target_failure.witness_state_commitment)
            }
        },
        "events": bundle.events.iter().map(|event| {
            let clock = event.clock_observation.as_ref().map(|value| json!({
                "clock_id": value.clock_id,
                "boot_epoch_id": value.boot_epoch_id,
                "unit": value.unit,
                "time_value": value.time_value,
                "resolution": value.resolution,
                "uncertainty": value.uncertainty
            }));
            json!({
                "event_id": event.event_id,
                "logical_sequence": event.logical_sequence,
                "kind": event_kind_name(event.kind),
                "actor_id": event.actor_id,
                "predecessor_event_ids": event.predecessor_event_ids,
                "clock_observation": clock,
                "raw_artifact_ids": event.raw_artifact_ids,
                "input_commitment": commitment_hex(&event.input_commitment),
                "configuration_commitment": commitment_hex(&event.configuration_commitment)
            })
        }).collect::<Vec<_>>(),
        "artifacts": bundle.artifacts.iter().map(|artifact| json!({
            "artifact_id": artifact.artifact_id,
            "media_type": artifact.media_type,
            "raw_artifact_commitment": commitment_hex(&artifact.raw_artifact_commitment)
        })).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(model: &'a [u8], config: &'a [u8], tool: &'a [u8]) -> TlcImportContext<'a> {
        TlcImportContext {
            bundle_id: "tlc-import-test-001",
            system_id: "synthetic-rrvr-system",
            implementation_revision: "symtropy-test-head",
            replay_seed: 42,
            property_id: "NoDuplicatePhysicalEffects",
            violation_class: "naive_retry_duplicate_effect",
            actor_id: "TLC",
            model_source_bytes: model,
            model_config_bytes: config,
            tla_tool_jar_bytes: tool,
            model_revision_label: "sha256:model-fixture",
            fault_profile_revision_label: "sha256:config-fixture",
            oracle_revision_label: "TLC-2ef2af8+sha256:tool-fixture",
        }
    }

    fn state(index: u64, stage: &str) -> Value {
        json!([
            index,
            {
                "s": {
                    "stage": stage,
                    "nested": {
                        "set_like": [1, 3, 5],
                        "tuple_like": ["old", false, {"x": 7}]
                    }
                },
                "other_state_var": {
                    "preserve": true,
                    "numbers": [0, 2, 4]
                }
            }
        ])
    }

    fn tlc_trace(action_names: &[&str]) -> Vec<u8> {
        let states: Vec<Value> = (0..=action_names.len())
            .map(|index| state(index as u64 + 1, &format!("state-{index}")))
            .collect();
        let actions: Vec<Value> = action_names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                json!([
                    states[index].clone(),
                    {
                        "name": name,
                        "location": {
                            "module": "RestartSafeEffectContinuity",
                            "line": index + 10
                        }
                    },
                    states[index + 1].clone()
                ])
            })
            .collect();
        serde_json::to_vec(&json!({
            "vars": ["s", "other_state_var"],
            "counterexample": {
                "state": states,
                "action": actions
            }
        }))
        .expect("serialize TLC fixture")
    }

    #[test]
    fn imports_a_valid_two_state_one_action_counterexample() {
        let bytes = tlc_trace(&["ReserveAttempt"]);
        let bundle = import_tlc_json_trace(
            &bytes,
            &context(b"model source", b"model config", b"TLC jar bytes"),
        )
        .expect("TLC import");

        assert_eq!(bundle.events.len(), 1);
        assert_eq!(bundle.events[0].kind, TraceEventKind::AttemptReserved);
        assert_eq!(bundle.events[0].predecessor_event_ids, Vec::<String>::new());
        assert_eq!(
            bundle.header.target_failure.witness_event_id,
            "tlc-action-001-ReserveAttempt"
        );
        assert_eq!(bundle.artifacts[0].raw_artifact_commitment, sha256(&bytes));
        assert_eq!(bundle.artifacts.len(), 4);
        assert_eq!(validate_trace_bundle(&bundle), TraceValidation::Valid);
    }

    #[test]
    fn imports_outer_tla_next_wrapper_without_guessing_inner_disjunct() {
        let bytes = tlc_trace(&["Next"]);
        let bundle = import_tlc_json_trace(
            &bytes,
            &context(b"model source", b"model config", b"TLC jar bytes"),
        )
        .expect("TLC Next wrapper import");

        assert_eq!(bundle.events.len(), 1);
        assert_eq!(bundle.events[0].kind, TraceEventKind::TlaNextTransition);
        assert_eq!(bundle.events[0].event_id, "tlc-action-001-Next");
        assert_eq!(
            bundle.header.target_failure.witness_event_id,
            "tlc-action-001-Next"
        );
    }

    #[test]
    fn imports_the_complete_thirty_action_rrvr_action_catalogue() {
        let names: Vec<&str> = TLA_ACTION_EVENT_MAPPING
            .iter()
            .map(|mapping| mapping.action_name)
            .collect();
        let bytes = tlc_trace(&names);
        let bundle = import_tlc_json_trace(
            &bytes,
            &context(b"model source", b"model config", b"TLC jar bytes"),
        )
        .expect("full TLC import");

        assert_eq!(bundle.events.len(), 30);
        assert_eq!(
            bundle.events.last().expect("last event").kind,
            TraceEventKind::NaiveRetryAttempted
        );
        assert_eq!(
            bundle
                .events
                .last()
                .expect("last event")
                .predecessor_event_ids,
            vec![bundle.events[28].event_id.clone()]
        );
        assert_eq!(
            bundle.header.target_failure.witness_event_id,
            bundle.events[29].event_id
        );
    }

    #[test]
    fn rejects_mismatched_state_action_counts() {
        let mut value: Value = serde_json::from_slice(&tlc_trace(&["ReserveAttempt"])).unwrap();
        value["counterexample"]["action"] = json!([]);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            import_tlc_json_trace(
                &bytes,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::ActionCountMismatch)
        );
    }

    #[test]
    fn rejects_a_disconnected_action_before_after_frame() {
        let mut value: Value = serde_json::from_slice(&tlc_trace(&["ReserveAttempt"])).unwrap();
        value["counterexample"]["action"][0][0] = state(99, "different-before-state");
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            import_tlc_json_trace(
                &bytes,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::DisconnectedTransition(1))
        );
    }

    #[test]
    fn rejects_unknown_action_names_and_missing_state_variable() {
        let unknown = tlc_trace(&["NotADeclaredAction"]);
        assert_eq!(
            import_tlc_json_trace(
                &unknown,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::UnknownAction(
                "NotADeclaredAction".to_owned()
            ))
        );

        let mut value: Value = serde_json::from_slice(&tlc_trace(&["ReserveAttempt"])).unwrap();
        value["vars"] = json!(["some_other_variable"]);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            import_tlc_json_trace(
                &bytes,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::MissingStateVariable)
        );
    }

    #[test]
    fn rejects_invalid_state_frame_indices_and_missing_s_values() {
        let mut index_mismatch: Value =
            serde_json::from_slice(&tlc_trace(&["ReserveAttempt"])).unwrap();
        index_mismatch["counterexample"]["state"][1][0] = json!(9);
        let bytes = serde_json::to_vec(&index_mismatch).unwrap();
        assert_eq!(
            import_tlc_json_trace(
                &bytes,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::StateIndexMismatch(2))
        );

        let mut missing_s: Value = serde_json::from_slice(&tlc_trace(&["ReserveAttempt"])).unwrap();
        missing_s["counterexample"]["state"][0][1] = json!({"other": true});
        let bytes = serde_json::to_vec(&missing_s).unwrap();
        assert_eq!(
            import_tlc_json_trace(
                &bytes,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::StateVariableMissingFromFrame(1))
        );
    }

    #[test]
    fn nested_tla_values_are_committed_without_coercion() {
        let bytes = tlc_trace(&["ReserveAttempt"]);
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let expected_action = &value["counterexample"]["action"][0];
        let bundle = import_tlc_json_trace(
            &bytes,
            &context(b"model source", b"model config", b"TLC jar bytes"),
        )
        .expect("TLC import");
        assert_eq!(
            bundle.events[0].input_commitment,
            sha256(&canonical_json(expected_action).unwrap())
        );
        assert_eq!(
            bundle.header.initial_state_commitment,
            sha256(&canonical_json(&value["counterexample"]["state"][0]).unwrap())
        );
        assert_eq!(
            bundle.header.target_failure.witness_state_commitment,
            sha256(&canonical_json(&value["counterexample"]["state"][1]).unwrap())
        );
    }

    #[test]
    fn imported_trace_is_deterministic_and_round_trips_to_schema_json() {
        let bytes = tlc_trace(&["ReserveAttempt", "MarkDispatchStarted", "NativeAcceptOld"]);
        let first = import_tlc_json_trace(
            &bytes,
            &context(b"model source", b"model config", b"TLC jar bytes"),
        )
        .unwrap();
        let second = import_tlc_json_trace(
            &bytes,
            &context(b"model source", b"model config", b"TLC jar bytes"),
        )
        .unwrap();
        assert_eq!(first, second);

        let json_bundle = trace_bundle_to_json_value(&first);
        assert_eq!(
            json_bundle["header"]["target_failure"]["property_id"],
            "NoDuplicatePhysicalEffects"
        );
        assert_eq!(json_bundle["events"].as_array().unwrap().len(), 3);
        assert_eq!(json_bundle["artifacts"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn rejects_missing_counterexample_and_invalid_context() {
        let bytes = br#"{"vars":["s"],"states":[]}"#;
        assert_eq!(
            import_tlc_json_trace(
                bytes,
                &context(b"model source", b"model config", b"TLC jar bytes")
            ),
            Err(TlcImportError::MissingCounterexample)
        );

        let empty_context = TlcImportContext {
            bundle_id: "",
            ..context(b"model source", b"model config", b"TLC jar bytes")
        };
        assert_eq!(
            import_tlc_json_trace(&tlc_trace(&["ReserveAttempt"]), &empty_context),
            Err(TlcImportError::InvalidContext)
        );
    }
}
