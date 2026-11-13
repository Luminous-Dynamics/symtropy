//! Versioned cross-repository receipt encoding for joint metrology.
//!
//! The payload carries the exact normalized trace identities and upstream
//! evidence alongside the report. SHA-256 is computed over compact,
//! lexicographically-key-sorted JSON payload bytes (RFC 8785 for this
//! restricted integer/string/boolean object model).

use super::*;
use serde_json::{json, Value};

pub const RECEIPT_SCHEMA_ID: &str =
    "urn:luminous-dynamics:loco-joint-metrology-normalized-receipt:0.1.0";
pub const RECEIPT_SCHEMA_VERSION: &str = "0.1.0";
pub const RECEIPT_CANONICALIZATION: &str = "ld-json-integer-canonical-v1";

/// Errors in receipt construction, parsing, hashing or identity binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireReceiptError {
    /// Adapter output does not refer to the supplied trace.
    ReportBindingMismatch(&'static str),
    /// A retrospective evidence report must never authorize actuation.
    ActuationAuthorityMustBeFalse,
    /// Receipt JSON is malformed or lacks its required envelope structure.
    InvalidEnvelope,
    /// Schema identity or canonicalization version is unsupported.
    UnsupportedSchemaVersion,
    /// SHA-256 does not match the canonical payload bytes.
    PayloadDigestMismatch,
    /// Repeated bindings inside the receipt are contradictory.
    ContradictoryPayloadBinding(&'static str),
    /// JSON serialization failed.
    SerializationFailure,
}

impl std::fmt::Display for WireReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for WireReceiptError {}

fn timestamp_value(value: &TimestampInterval) -> Value {
    json!({
        "clock_id": value.clock_id,
        "boot_epoch": value.boot_epoch,
        "lower_ns": value.lower_ns,
        "upper_ns": value.upper_ns
    })
}

fn identity_value(value: &CausalIdentity) -> Value {
    json!({
        "mission_id": value.mission_id,
        "robot_id": value.robot_id,
        "boot_epoch": value.boot_epoch,
        "scenario_manifest_digest": value.scenario_manifest_digest,
        "source_observation_id": value.source_observation_id,
        "source_event_sequence": value.source_event_sequence,
        "decision_id": value.decision_id,
        "decision_event_sequence": value.decision_event_sequence,
        "proposal_id": value.proposal_id,
        "proposal_event_sequence": value.proposal_event_sequence,
        "assurance_event_id": value.assurance_event_id,
        "assurance_event_sequence": value.assurance_event_sequence,
        "command_id": value.command_id,
        "command_event_sequence": value.command_event_sequence,
        "actuator_application_event_id": value.actuator_application_event_id,
        "actuator_application_event_sequence": value.actuator_application_event_sequence,
        "actuator_application_receipt_digest": value.actuator_application_receipt_digest,
        "source_sensor_calibration_digest": value.source_sensor_calibration_digest,
        "physical_transition_id": value.physical_transition_id,
        "physical_transition_event_sequence": value.physical_transition_event_sequence,
        "terminal_event_id": value.terminal_event_id,
        "terminal_event_sequence": value.terminal_event_sequence,
        "actuator_id": value.actuator_id
    })
}

fn configuration_value(value: &ConfigurationEvidence) -> Value {
    json!({
        "actuator_configuration_digest": value.actuator_configuration_digest,
        "controller_configuration_digest": value.controller_configuration_digest,
        "policy_digest": value.policy_digest,
        "model_digest": value.model_digest,
        "action_domain_digest": value.action_domain_digest,
        "sensor_calibration_digest": value.sensor_calibration_digest,
        "torque_transducer_calibration_digest": value.torque_transducer_calibration_digest,
        "power_analyzer_calibration_digest": value.power_analyzer_calibration_digest,
        "thermal_sensor_calibration_digest": value.thermal_sensor_calibration_digest,
        "sampling_spec_digest": value.sampling_spec_digest,
        "energy_boundary_manifest_digest": value.energy_boundary_manifest_digest,
        "actuator_application_receipt_digest": value.actuator_application_receipt_digest,
        "executed_prefix_expected_digest": value.executed_prefix_expected_digest,
        "executed_prefix_observed_digest": value.executed_prefix_observed_digest,
        "decision_calibration_receipt_digest": value.decision_calibration_receipt_digest
    })
}

fn clock_relation_value(value: &ClockRelation) -> Value {
    match value {
        ClockRelation::SameMonotonicClockVerified => {
            json!({"kind": "same_monotonic_clock_verified"})
        }
        ClockRelation::BoundedMappingVerified(mapping) => json!({
            "kind": "bounded_mapping_verified",
            "mapping": {
                "source_clock_id": mapping.source_clock_id,
                "target_clock_id": mapping.target_clock_id,
                "boot_epoch": mapping.boot_epoch,
                "offset_lower_ns": mapping.offset_lower_ns,
                "offset_upper_ns": mapping.offset_upper_ns,
                "mapping_digest": mapping.mapping_digest
            }
        }),
        ClockRelation::Unknown => json!({"kind": "unknown"}),
    }
}

fn decision_input_value(value: &DecisionEvidence) -> Value {
    json!({
        "decision_id": value.decision_id,
        "model_digest": value.model_digest,
        "action_domain_digest": value.action_domain_digest,
        "calibration_receipt_digest": value.calibration_receipt_digest,
        "regret_upper_units": value.regret_upper_units,
        "epsilon_task_loss_units": value.epsilon_task_loss_units,
        "valid_through_tick": value.valid_through_tick,
        "actuator_application_tick": value.actuator_application_tick
    })
}

fn thermal_input_value(value: &ThermalEvidence) -> Value {
    json!({
        "physical_transition_id": value.physical_transition_id,
        "sensor_calibration_digest": value.sensor_calibration_digest,
        "required_node_ids": value.required_node_ids,
        "observed_node_ids": value.observed_node_ids,
        "expected_predecessor_digest": value.expected_predecessor_digest,
        "observed_predecessor_digest": value.observed_predecessor_digest,
        "result_digest": value.result_digest,
        "sequence_reset": value.sequence_reset
    })
}

fn energy_input_value(value: &EnergyEvidence) -> Value {
    let closure = match &value.closure {
        UpstreamEnergyClosure::Balanced { receipt_digest } => json!({
            "status": "balanced",
            "receipt_digest": receipt_digest
        }),
        UpstreamEnergyClosure::Inconclusive { reason } => json!({
            "status": "inconclusive",
            "reason": reason
        }),
        UpstreamEnergyClosure::Invalid { reason } => json!({
            "status": "invalid",
            "reason": reason
        }),
        UpstreamEnergyClosure::Unknown => json!({"status": "unknown"}),
    };
    json!({
        "boundary_complete": value.boundary_complete,
        "boundary_manifest_digest": value.boundary_manifest_digest,
        "closure": closure
    })
}

fn sampling_input_value(value: &SamplingEvidence) -> Value {
    json!({
        "measured_rate_hz": value.measured_rate_hz,
        "required_rate_hz": value.required_rate_hz,
        "specification_digest": value.specification_digest,
        "specification_qualified": value.specification_qualified
    })
}

fn unknown_reason_code(value: &UnknownReason) -> &'static str {
    match value {
        UnknownReason::ClockRelationUnavailable => "clock_relation_unavailable",
        UnknownReason::ClockMappingDoesNotCoverTrace => "clock_mapping_does_not_cover_trace",
        UnknownReason::TimestampOrderOverlaps => "timestamp_order_overlaps",
        UnknownReason::DecisionCalibrationUnavailable => "decision_calibration_unavailable",
        UnknownReason::ThermalNodeCoverageIncomplete => "thermal_node_coverage_incomplete",
        UnknownReason::EnergyReceiptUnavailable => "energy_receipt_unavailable",
        UnknownReason::SamplingSpecificationUnavailable => "sampling_specification_unavailable",
    }
}

fn inconclusive_value(value: &InconclusiveReason) -> (&'static str, Option<String>) {
    match value {
        InconclusiveReason::EnergyBoundaryIncomplete => ("energy_boundary_incomplete", None),
        InconclusiveReason::TerminalSafeStopNotQualified => {
            ("terminal_safe_stop_not_qualified", None)
        }
        InconclusiveReason::UpstreamEnergyClosureInconclusive(reason) => {
            ("upstream_energy_closure_inconclusive", Some(reason.clone()))
        }
    }
}

fn rejected_value(value: &RejectReason) -> (&'static str, Value) {
    match value {
        RejectReason::TimestampOrderContradiction => {
            ("timestamp_order_contradiction", Value::Null)
        }
        RejectReason::ThermalSequenceReset => ("thermal_sequence_reset", Value::Null),
        RejectReason::ThermalPredecessorMismatch => {
            ("thermal_predecessor_mismatch", Value::Null)
        }
        RejectReason::EnergyBoundaryInvalid(reason) => {
            ("energy_boundary_invalid", json!({"reason": reason}))
        }
        RejectReason::DecisionHorizonExpired {
            valid_through_tick,
            application_tick,
        } => (
            "decision_horizon_expired",
            json!({
                "valid_through_tick": valid_through_tick,
                "application_tick": application_tick
            }),
        ),
        RejectReason::DecisionRegretExceeded {
            regret_upper_units,
            epsilon_task_loss_units,
        } => (
            "decision_regret_exceeded",
            json!({
                "regret_upper_units": regret_upper_units,
                "epsilon_task_loss_units": epsilon_task_loss_units
            }),
        ),
        RejectReason::ExecutedPrefixMismatch => {
            ("executed_prefix_mismatch", Value::Null)
        }
        RejectReason::ReceiptBindingMismatch(field) => {
            ("receipt_binding_mismatch", json!({"field": field}))
        }
        RejectReason::SamplingRateBelowSpecification {
            measured_rate_hz,
            required_rate_hz,
        } => (
            "sampling_rate_below_specification",
            json!({
                "measured_rate_hz": measured_rate_hz,
                "required_rate_hz": required_rate_hz
            }),
        ),
        RejectReason::UpstreamEnergyClosureInvalid(reason) => {
            ("upstream_energy_closure_invalid", json!({"reason": reason}))
        }
    }
}

fn evidence_value<T>(evidence: &Evidence<T>, encode: impl FnOnce(&T) -> Value) -> Value {
    match evidence {
        Evidence::Qualified(value) => json!({
            "status": "qualified",
            "value": encode(value)
        }),
        Evidence::Unknown(reason) => json!({
            "status": "unknown",
            "reason_code": unknown_reason_code(reason)
        }),
        Evidence::Inconclusive(reason) => {
            let (code, detail) = inconclusive_value(reason);
            let mut result = json!({
                "status": "inconclusive",
                "reason_code": code
            });
            if let Some(detail) = detail {
                result["reason_detail"] = json!(detail);
            }
            result
        }
        Evidence::Rejected(reason) => {
            let (code, detail) = rejected_value(reason);
            let mut result = json!({
                "status": "rejected",
                "reason_code": code
            });
            if !detail.is_null() {
                result["details"] = detail;
            }
            result
        }
    }
}

fn duration_value(value: &DurationBounds) -> Value {
    json!({
        "lower_ns": value.lower_ns,
        "upper_ns": value.upper_ns
    })
}

fn decision_value(value: &DecisionDisposition) -> Value {
    match value {
        DecisionDisposition::Sufficient { regret_upper_units } => json!({
            "disposition": "sufficient",
            "regret_upper_units": regret_upper_units
        }),
        DecisionDisposition::Insufficient {
            regret_upper_units,
            epsilon_task_loss_units,
        } => json!({
            "disposition": "insufficient",
            "regret_upper_units": regret_upper_units,
            "epsilon_task_loss_units": epsilon_task_loss_units
        }),
        DecisionDisposition::Expired {
            valid_through_tick,
            application_tick,
        } => json!({
            "disposition": "expired",
            "valid_through_tick": valid_through_tick,
            "application_tick": application_tick
        }),
    }
}

fn thermal_value(value: &ThermalDisposition) -> Value {
    match value {
        ThermalDisposition::Continuous {
            node_count,
            result_digest,
        } => json!({
            "disposition": "continuous",
            "node_count": node_count,
            "result_digest": result_digest
        }),
    }
}

fn energy_value(value: &EnergyDisposition) -> Value {
    match value {
        EnergyDisposition::Balanced {
            first_law_receipt_digest,
        } => json!({
            "disposition": "balanced",
            "first_law_receipt_digest": first_law_receipt_digest
        }),
    }
}

fn sampling_value(value: &SamplingDisposition) -> Value {
    match value {
        SamplingDisposition::MeetsDeclaredSpecification { measured_rate_hz } => json!({
            "disposition": "meets_declared_specification",
            "measured_rate_hz": measured_rate_hz
        }),
    }
}

fn closure_value(value: &MissionClosure) -> Value {
    match value {
        MissionClosure::Closed => json!({
            "status": "closed",
            "unqualified_domains": []
        }),
        MissionClosure::Open { unqualified_domains } => json!({
            "status": "open",
            "unqualified_domains": unqualified_domains
        }),
    }
}

fn report_value(report: &AdapterReport) -> Value {
    json!({
        "mission_id": report.mission_id,
        "robot_id": report.robot_id,
        "actuator_id": report.actuator_id,
        "source_to_actuator_aoi": evidence_value(&report.source_to_actuator_aoi, duration_value),
        "command_to_motion": evidence_value(&report.command_to_motion, duration_value),
        "decision": evidence_value(&report.decision, decision_value),
        "thermal": evidence_value(&report.thermal, thermal_value),
        "energy": evidence_value(&report.energy, energy_value),
        "sampling": evidence_value(&report.sampling, sampling_value),
        "terminal_safe_stop": evidence_value(&report.terminal_safe_stop, |_| json!(true)),
        "causal_edges": report.causal_edges.iter().map(|edge| json!({
            "from_event_id": edge.from_event_id,
            "to_event_id": edge.to_event_id,
            "from_sequence": edge.from_sequence,
            "to_sequence": edge.to_sequence
        })).collect::<Vec<_>>(),
        "causal_chain_complete": report.causal_chain_complete,
        "mission_closure": closure_value(&report.mission_closure),
        "actuation_authorized": report.actuation_authorized
    })
}

fn expected_edges(identity: &CausalIdentity) -> Vec<CausalEdge> {
    vec![
        CausalEdge {
            from_event_id: identity.source_observation_id.clone(),
            to_event_id: identity.decision_id.clone(),
            from_sequence: identity.source_event_sequence,
            to_sequence: identity.decision_event_sequence,
        },
        CausalEdge {
            from_event_id: identity.decision_id.clone(),
            to_event_id: identity.proposal_id.clone(),
            from_sequence: identity.decision_event_sequence,
            to_sequence: identity.proposal_event_sequence,
        },
        CausalEdge {
            from_event_id: identity.proposal_id.clone(),
            to_event_id: identity.assurance_event_id.clone(),
            from_sequence: identity.proposal_event_sequence,
            to_sequence: identity.assurance_event_sequence,
        },
        CausalEdge {
            from_event_id: identity.assurance_event_id.clone(),
            to_event_id: identity.command_id.clone(),
            from_sequence: identity.assurance_event_sequence,
            to_sequence: identity.command_event_sequence,
        },
        CausalEdge {
            from_event_id: identity.command_id.clone(),
            to_event_id: identity.actuator_application_event_id.clone(),
            from_sequence: identity.command_event_sequence,
            to_sequence: identity.actuator_application_event_sequence,
        },
        CausalEdge {
            from_event_id: identity.actuator_application_event_id.clone(),
            to_event_id: identity.physical_transition_id.clone(),
            from_sequence: identity.actuator_application_event_sequence,
            to_sequence: identity.physical_transition_event_sequence,
        },
        CausalEdge {
            from_event_id: identity.physical_transition_id.clone(),
            to_event_id: identity.terminal_event_id.clone(),
            from_sequence: identity.physical_transition_event_sequence,
            to_sequence: identity.terminal_event_sequence,
        },
    ]
}

fn validate_trace_report_binding(
    trace: &JointTrace,
    report: &AdapterReport,
) -> Result<(), WireReceiptError> {
    if report.mission_id != trace.identity.mission_id {
        return Err(WireReceiptError::ReportBindingMismatch("mission_id"));
    }
    if report.robot_id != trace.identity.robot_id {
        return Err(WireReceiptError::ReportBindingMismatch("robot_id"));
    }
    if report.actuator_id != trace.identity.actuator_id {
        return Err(WireReceiptError::ReportBindingMismatch("actuator_id"));
    }
    if report.actuation_authorized {
        return Err(WireReceiptError::ActuationAuthorityMustBeFalse);
    }
    if report.causal_edges != expected_edges(&trace.identity) {
        return Err(WireReceiptError::ReportBindingMismatch("causal_edges"));
    }
    if report.causal_chain_complete != (report.causal_edges.len() == 7) {
        return Err(WireReceiptError::ReportBindingMismatch("causal_chain_complete"));
    }
    if trace.identity.actuator_application_receipt_digest
        != trace.configuration.actuator_application_receipt_digest
    {
        return Err(WireReceiptError::ReportBindingMismatch(
            "actuator_application_receipt_digest",
        ));
    }
    if trace.identity.source_sensor_calibration_digest
        != trace.configuration.sensor_calibration_digest
    {
        return Err(WireReceiptError::ReportBindingMismatch(
            "source_sensor_calibration_digest",
        ));
    }
    if trace.decision.decision_id != trace.identity.decision_id
        || trace.decision.model_digest != trace.configuration.model_digest
        || trace.decision.action_domain_digest != trace.configuration.action_domain_digest
        || trace.decision.calibration_receipt_digest
            != trace.configuration.decision_calibration_receipt_digest
    {
        return Err(WireReceiptError::ReportBindingMismatch("decision_input"));
    }
    if trace.thermal.physical_transition_id != trace.identity.physical_transition_id
        || trace.thermal.sensor_calibration_digest
            != trace.configuration.thermal_sensor_calibration_digest
    {
        return Err(WireReceiptError::ReportBindingMismatch("thermal_input"));
    }
    if trace.energy.boundary_manifest_digest
        != trace.configuration.energy_boundary_manifest_digest
    {
        return Err(WireReceiptError::ReportBindingMismatch("energy_input"));
    }
    if trace.sampling.specification_digest != trace.configuration.sampling_spec_digest {
        return Err(WireReceiptError::ReportBindingMismatch("sampling_input"));
    }
    Ok(())
}

/// Build a versioned normalized receipt from the exact input trace and its report.
///
/// The receipt digest is SHA-256 over the versioned Luminous integer-canonical
/// payload bytes: serde_json 1.0.150 Value maps sort keys lexicographically;
/// the payload uses only exact integers, strings, booleans, arrays and objects.

pub fn build_normalized_receipt(
    trace: &JointTrace,
    report: &AdapterReport,
) -> Result<String, WireReceiptError> {
    validate_trace_report_binding(trace, report)?;
    let payload = json!({
        "identity": identity_value(&trace.identity),
        "configuration": configuration_value(&trace.configuration),
        "clock_relation": clock_relation_value(&trace.clock_relation),
        "timestamp_intervals": {
            "source_capture": timestamp_value(&trace.source_capture),
            "actuator_application": timestamp_value(&trace.actuator_application),
            "motion_onset": timestamp_value(&trace.motion_onset)
        },
        "decision_input": decision_input_value(&trace.decision),
        "thermal_input": thermal_input_value(&trace.thermal),
        "energy_input": energy_input_value(&trace.energy),
        "sampling_input": sampling_input_value(&trace.sampling),
        "terminal_safe_stop_qualified": trace.terminal_safe_stop_qualified,
        "report": report_value(report)
    });
    let canonical_payload =
        serde_json::to_vec(&payload).map_err(|_| WireReceiptError::SerializationFailure)?;
    let payload_sha256 = hex_digest(&sha256(&canonical_payload));
    let envelope = json!({
        "schema_id": RECEIPT_SCHEMA_ID,
        "schema_version": RECEIPT_SCHEMA_VERSION,
        "canonicalization": RECEIPT_CANONICALIZATION,
        "producer": {
            "repository": "Luminous-Dynamics/symtropy",
            "package": "loco-joint-trace-adapter",
            "version": env!("CARGO_PKG_VERSION")
        },
        "payload_sha256": payload_sha256,
        "payload": payload
    });
    serde_json::to_string(&envelope).map_err(|_| WireReceiptError::SerializationFailure)
}

/// Parse and verify the version, canonical payload digest, duplicate bindings
/// and no-actuation invariant of a serialized normalized receipt.
pub fn verify_normalized_receipt(serialized: &str) -> Result<Value, WireReceiptError> {
    let envelope: Value =
        serde_json::from_str(serialized).map_err(|_| WireReceiptError::InvalidEnvelope)?;
    if !has_exact_keys(
        &envelope,
        &[
            "schema_id",
            "schema_version",
            "canonicalization",
            "producer",
            "payload_sha256",
            "payload",
        ],
    ) {
        return Err(WireReceiptError::InvalidEnvelope);
    }
    if envelope["schema_id"].as_str() != Some(RECEIPT_SCHEMA_ID)
        || envelope["schema_version"].as_str() != Some(RECEIPT_SCHEMA_VERSION)
        || envelope["canonicalization"].as_str() != Some(RECEIPT_CANONICALIZATION)
        || !has_exact_keys(&envelope["producer"], &["repository", "package", "version"])
    {
        return Err(WireReceiptError::UnsupportedSchemaVersion);
    }
    let payload = &envelope["payload"];
    if !has_exact_keys(
        payload,
        &[
            "identity",
            "configuration",
            "clock_relation",
            "timestamp_intervals",
            "decision_input",
            "thermal_input",
            "energy_input",
            "sampling_input",
            "terminal_safe_stop_qualified",
            "report",
        ],
    ) {
        return Err(WireReceiptError::InvalidEnvelope);
    }
    let canonical = serde_json::to_vec(payload).map_err(|_| WireReceiptError::SerializationFailure)?;
    let expected = envelope["payload_sha256"]
        .as_str()
        .ok_or(WireReceiptError::InvalidEnvelope)?;
    if hex_digest(&sha256(&canonical)) != expected {
        return Err(WireReceiptError::PayloadDigestMismatch);
    }
    let identity = &payload["identity"];
    let configuration = &payload["configuration"];
    let decision = &payload["decision_input"];
    let thermal = &payload["thermal_input"];
    let energy = &payload["energy_input"];
    let sampling = &payload["sampling_input"];
    let report = &payload["report"];
    if report["actuation_authorized"] != Value::Bool(false) {
        return Err(WireReceiptError::ActuationAuthorityMustBeFalse);
    }
    for (field, left, right) in [
        ("mission_id", identity["mission_id"].as_str(), report["mission_id"].as_str()),
        ("robot_id", identity["robot_id"].as_str(), report["robot_id"].as_str()),
        ("actuator_id", identity["actuator_id"].as_str(), report["actuator_id"].as_str()),
        (
            "actuator_application_receipt_digest",
            identity["actuator_application_receipt_digest"].as_str(),
            configuration["actuator_application_receipt_digest"].as_str(),
        ),
        (
            "source_sensor_calibration_digest",
            identity["source_sensor_calibration_digest"].as_str(),
            configuration["sensor_calibration_digest"].as_str(),
        ),
        ("decision_id", identity["decision_id"].as_str(), decision["decision_id"].as_str()),
        ("model_digest", configuration["model_digest"].as_str(), decision["model_digest"].as_str()),
        (
            "action_domain_digest",
            configuration["action_domain_digest"].as_str(),
            decision["action_domain_digest"].as_str(),
        ),
        (
            "decision_calibration_receipt_digest",
            configuration["decision_calibration_receipt_digest"].as_str(),
            decision["calibration_receipt_digest"].as_str(),
        ),
        (
            "physical_transition_id",
            identity["physical_transition_id"].as_str(),
            thermal["physical_transition_id"].as_str(),
        ),
        (
            "thermal_sensor_calibration_digest",
            configuration["thermal_sensor_calibration_digest"].as_str(),
            thermal["sensor_calibration_digest"].as_str(),
        ),
        (
            "energy_boundary_manifest_digest",
            configuration["energy_boundary_manifest_digest"].as_str(),
            energy["boundary_manifest_digest"].as_str(),
        ),
        (
            "sampling_spec_digest",
            configuration["sampling_spec_digest"].as_str(),
            sampling["specification_digest"].as_str(),
        ),
    ] {
        if left.is_none() || right.is_none() || left != right {
            return Err(WireReceiptError::ContradictoryPayloadBinding(field));
        }
    }
    Ok(envelope)
}

fn has_exact_keys(value: &Value, expected: &[&str]) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != expected.len() {
        return false;
    }
    expected.iter().all(|key| object.contains_key(*key))
}

fn hex_digest(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = INITIAL;
    for chunk in message.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, bytes) in chunk.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut output = [0_u8; 32];
    for (index, word) in state.iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::reference_trace;

    #[test]
    fn sha256_matches_known_empty_and_abc_vectors() {
        assert_eq!(
            hex_digest(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex_digest(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn normalized_receipt_round_trips_and_preserves_all_top_level_bindings() {
        let trace = reference_trace();
        let report = adapt_joint_trace(&trace).unwrap();
        let serialized = build_normalized_receipt(&trace, &report).unwrap();
        let envelope = verify_normalized_receipt(&serialized).unwrap();
        assert_eq!(envelope["schema_version"], RECEIPT_SCHEMA_VERSION);
        assert_eq!(envelope["payload"]["identity"]["mission_id"], "mission-01");
        assert_eq!(envelope["payload"]["identity"]["terminal_event_sequence"], 8);
        assert_eq!(
            envelope["payload"]["configuration"]["executed_prefix_expected_digest"],
            "prefix-expected"
        );
        assert_eq!(envelope["payload"]["report"]["actuation_authorized"], false);
    }

    #[test]
    fn frozen_positive_fixture_has_the_declared_payload_digest() {
        let serialized = include_str!("../../fixtures/normalized_joint_receipt_v0_1.json");
        let envelope = verify_normalized_receipt(serialized).unwrap();
        assert_eq!(
            envelope["payload_sha256"],
            "94137cc32ebeb908b2e0ad50ad6f05fb9836f455056d7b88eb54dffc0c290aad"
        );
        let trace = reference_trace();
        let report = adapt_joint_trace(&trace).unwrap();
        let generated = build_normalized_receipt(&trace, &report).unwrap();
        let generated: Value = serde_json::from_str(&generated).unwrap();
        assert_eq!(generated["payload"], envelope["payload"]);
        assert_eq!(generated["payload_sha256"], envelope["payload_sha256"]);
    }

    #[test]
    fn receipt_digest_detects_payload_mutation() {
        let trace = reference_trace();
        let report = adapt_joint_trace(&trace).unwrap();
        let serialized = build_normalized_receipt(&trace, &report).unwrap();
        let mut envelope: Value = serde_json::from_str(&serialized).unwrap();
        envelope["payload"]["identity"]["mission_id"] = json!("other-mission");
        let mutated = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            verify_normalized_receipt(&mutated),
            Err(WireReceiptError::PayloadDigestMismatch)
        );
    }

    #[test]
    fn adapter_report_cannot_authorize_actuation() {
        let trace = reference_trace();
        let mut report = adapt_joint_trace(&trace).unwrap();
        report.actuation_authorized = true;
        assert_eq!(
            build_normalized_receipt(&trace, &report),
            Err(WireReceiptError::ActuationAuthorityMustBeFalse)
        );
    }

    #[test]
    fn duplicated_identity_bindings_must_agree() {
        let mut envelope: Value = serde_json::from_str(
            include_str!("../../fixtures/normalized_joint_receipt_v0_1.json")
        )
        .unwrap();
        envelope["payload"]["report"]["mission_id"] = json!("other-mission");
        // Recompute the payload digest to prove the structural cross-binding
        // check independently catches a self-consistent hash over bad data.
        let digest = hex_digest(&sha256(
            &serde_json::to_vec(&envelope["payload"]).unwrap()
        ));
        envelope["payload_sha256"] = json!(digest);
        let serialized = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            verify_normalized_receipt(&serialized),
            Err(WireReceiptError::ContradictoryPayloadBinding("mission_id"))
        );
    }
}
