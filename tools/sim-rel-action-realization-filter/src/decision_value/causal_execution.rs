//! Causal execution receipts for command-to-evidence attribution.
//!
//! This module verifies logical identity, event ordering, clock-uncertainty
//! separation and explicit links from command intent through actuator readback,
//! physical transition, sensor acquisition, inference and decision. It does
//! not authenticate device signatures or prove that a sensor is calibrated.

use std::collections::BTreeSet;

const MAX_ID_BYTES: usize = 256;
const EVENT_COUNT: u64 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UncertainTimestamp {
    pub timestamp_ns: u64,
    pub uncertainty_ns: u64,
}

impl UncertainTimestamp {
    fn earliest(self) -> u64 {
        self.timestamp_ns.saturating_sub(self.uncertainty_ns)
    }

    fn latest(self) -> Result<u64, String> {
        self.timestamp_ns
            .checked_add(self.uncertainty_ns)
            .ok_or_else(|| "timestamp uncertainty interval overflows u64".into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchState {
    Accepted,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcknowledgementState {
    Accepted,
    Rejected,
    Absent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RealizationAssessment {
    WithinTolerance,
    OutsideTolerance,
    MeasurementAmbiguous,
    Unattributed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionEvidenceStatus {
    RiskBudgetSatisfied,
    Abstained,
    RiskBudgetViolation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CausalExecutionStage {
    Command,
    Dispatch,
    Acknowledgement,
    ActuatorReadback,
    PhysicalTransition,
    SensorAcquisition,
    Inference,
    Decision,
    FinalConsistency,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalExecutionError {
    pub stage: CausalExecutionStage,
    pub sequence: Option<u64>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandIntent {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub device_id: String,
    pub requested_value: i64,
    pub unit: String,
    pub tolerance: u64,
    pub operating_minimum: i64,
    pub operating_maximum: i64,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchReceipt {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub device_id: String,
    pub transport_id: String,
    pub payload_digest: String,
    pub state: DispatchState,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceAcknowledgement {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub device_id: String,
    pub firmware_identity: String,
    pub state: AcknowledgementState,
    pub echoed_value: Option<i64>,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActuatorReadback {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub device_id: String,
    pub measured_value: i64,
    pub measurement_uncertainty: u64,
    pub calibration_receipt_id: String,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalTransitionEvidence {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub parent_actuator_event_id: String,
    pub pre_state_digest: String,
    pub post_state_digest: String,
    pub model_version: String,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SensorAcquisition {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub parent_transition_event_id: String,
    pub instrument_id: String,
    pub raw_sample_digest: String,
    pub calibration_receipt_id: String,
    pub quality_flags: Vec<String>,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InferenceEvidence {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub sensor_event_ids: Vec<String>,
    pub inference_version: String,
    pub conclusion_digest: String,
    pub uncertainty_digest: String,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionEvidence {
    pub event_id: String,
    pub sequence: u64,
    pub command_id: String,
    pub inference_event_id: String,
    pub policy_identity: String,
    pub decision_digest: String,
    pub abstained: bool,
    pub risk_budget_satisfied: bool,
    pub clock: UncertainTimestamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalExecutionBundle {
    pub command: CommandIntent,
    pub dispatch: DispatchReceipt,
    pub acknowledgement: DeviceAcknowledgement,
    pub actuator_readback: ActuatorReadback,
    pub physical_transition: PhysicalTransitionEvidence,
    pub sensor_acquisition: SensorAcquisition,
    pub inference: InferenceEvidence,
    pub decision: DecisionEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalExecutionAuditReceipt {
    pub command_id: String,
    pub event_ids: Vec<String>,
    pub realization_assessment: RealizationAssessment,
    pub resolved_delivery: AcknowledgementState,
    pub decision_status: DecisionEvidenceStatus,
    pub sequence_complete: bool,
    pub timestamp_intervals_strictly_ordered: bool,
    pub causal_references_consistent: bool,
    pub device_identity_consistent: bool,
    pub calibration_receipt_ids: Vec<String>,
}

/// Checks that the full uncertainty interval for an earlier event is strictly
/// before the full uncertainty interval for a later event. Overlap is treated
/// as ambiguous, not silently ordered by nominal timestamp.
fn require_strict_temporal_precedence(
    earlier: UncertainTimestamp,
    later: UncertainTimestamp,
    stage: CausalExecutionStage,
    sequence: u64,
) -> Result<(), CausalExecutionError> {
    let latest_earlier = earlier.latest().map_err(|reason| CausalExecutionError {
        stage,
        sequence: Some(sequence),
        reason,
    })?;
    if latest_earlier >= later.earliest() {
        return Err(CausalExecutionError {
            stage,
            sequence: Some(sequence),
            reason: "timestamp uncertainty intervals overlap or are not strictly ordered".into(),
        });
    }
    let _ = later.latest().map_err(|reason| CausalExecutionError {
        stage,
        sequence: Some(sequence),
        reason,
    })?;
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn validate_event_identity(
    event_id: &str,
    command_id: &str,
    expected_command_id: &str,
    sequence: u64,
    expected_sequence: u64,
    stage: CausalExecutionStage,
) -> Result<(), CausalExecutionError> {
    if !valid_id(event_id) || !valid_id(command_id) {
        return Err(CausalExecutionError {
            stage,
            sequence: Some(sequence),
            reason: "event or command identity is empty or overlong".into(),
        });
    }
    if command_id != expected_command_id {
        return Err(CausalExecutionError {
            stage,
            sequence: Some(sequence),
            reason: "event is not bound to the bundle command identity".into(),
        });
    }
    if sequence != expected_sequence {
        return Err(CausalExecutionError {
            stage,
            sequence: Some(sequence),
            reason: format!("expected sequence {expected_sequence}, found {sequence}"),
        });
    }
    Ok(())
}

fn assess_realization(command: &CommandIntent, readback: &ActuatorReadback) -> Result<RealizationAssessment, String> {
    let tolerance = i64::try_from(command.tolerance)
        .map_err(|_| "command tolerance exceeds signed measurement range".to_string())?;
    let uncertainty = i64::try_from(readback.measurement_uncertainty)
        .map_err(|_| "actuator measurement uncertainty exceeds signed range".to_string())?;
    let target_lower = command
        .requested_value
        .checked_sub(tolerance)
        .ok_or_else(|| "requested-value tolerance lower bound overflows".to_string())?;
    let target_upper = command
        .requested_value
        .checked_add(tolerance)
        .ok_or_else(|| "requested-value tolerance upper bound overflows".to_string())?;
    let measured_lower = readback
        .measured_value
        .checked_sub(uncertainty)
        .ok_or_else(|| "measured-value uncertainty lower bound overflows".to_string())?;
    let measured_upper = readback
        .measured_value
        .checked_add(uncertainty)
        .ok_or_else(|| "measured-value uncertainty upper bound overflows".to_string())?;

    if measured_lower >= target_lower && measured_upper <= target_upper {
        Ok(RealizationAssessment::WithinTolerance)
    } else if measured_upper < target_lower || measured_lower > target_upper {
        Ok(RealizationAssessment::OutsideTolerance)
    } else {
        Ok(RealizationAssessment::MeasurementAmbiguous)
    }
}

/// Audit the complete command-to-decision evidence bundle.
///
/// A successful receipt means only that the supplied records have consistent
/// identities, strict clock-interval order and references. It does not
/// cryptographically authenticate the records or independently establish
/// physical truth. An acknowledgement is never treated as actuator readback.
pub fn audit_causal_execution_bundle(
    bundle: &CausalExecutionBundle,
) -> Result<CausalExecutionAuditReceipt, CausalExecutionError> {
    let command = &bundle.command;
    let command_id = command.command_id.as_str();
    validate_event_identity(
        &command.event_id,
        command_id,
        command_id,
        command.sequence,
        0,
        CausalExecutionStage::Command,
    )?;
    for (value, label) in [
        (&command.device_id, "command device ID"),
        (&command.unit, "command unit"),
    ] {
        if !valid_id(value) {
            return Err(CausalExecutionError {
                stage: CausalExecutionStage::Command,
                sequence: Some(command.sequence),
                reason: format!("{label} is empty or overlong"),
            });
        }
    }
    if command.operating_minimum > command.operating_maximum
        || command.requested_value < command.operating_minimum
        || command.requested_value > command.operating_maximum
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::Command,
            sequence: Some(command.sequence),
            reason: "commanded value is outside the declared operating envelope".into(),
        });
    }

    let events = [
        (&command.event_id, command.sequence, command.clock),
        (
            &bundle.dispatch.event_id,
            bundle.dispatch.sequence,
            bundle.dispatch.clock,
        ),
        (
            &bundle.acknowledgement.event_id,
            bundle.acknowledgement.sequence,
            bundle.acknowledgement.clock,
        ),
        (
            &bundle.actuator_readback.event_id,
            bundle.actuator_readback.sequence,
            bundle.actuator_readback.clock,
        ),
        (
            &bundle.physical_transition.event_id,
            bundle.physical_transition.sequence,
            bundle.physical_transition.clock,
        ),
        (
            &bundle.sensor_acquisition.event_id,
            bundle.sensor_acquisition.sequence,
            bundle.sensor_acquisition.clock,
        ),
        (
            &bundle.inference.event_id,
            bundle.inference.sequence,
            bundle.inference.clock,
        ),
        (
            &bundle.decision.event_id,
            bundle.decision.sequence,
            bundle.decision.clock,
        ),
    ];
    let mut event_ids = BTreeSet::new();
    for (index, (event_id, sequence, _)) in events.iter().enumerate() {
        if !event_ids.insert((*event_id).clone()) {
            return Err(CausalExecutionError {
                stage: CausalExecutionStage::EventSequence,
                sequence: Some(*sequence),
                reason: "event identity is duplicated".into(),
            });
        }
        if *sequence != index as u64 {
            return Err(CausalExecutionError {
                stage: CausalExecutionStage::EventSequence,
                sequence: Some(*sequence),
                reason: format!("expected contiguous event sequence {index}"),
            });
        }
    }

    validate_event_identity(
        &bundle.dispatch.event_id,
        &bundle.dispatch.command_id,
        command_id,
        bundle.dispatch.sequence,
        1,
        CausalExecutionStage::Dispatch,
    )?;
    validate_event_identity(
        &bundle.acknowledgement.event_id,
        &bundle.acknowledgement.command_id,
        command_id,
        bundle.acknowledgement.sequence,
        2,
        CausalExecutionStage::Acknowledgement,
    )?;
    validate_event_identity(
        &bundle.actuator_readback.event_id,
        &bundle.actuator_readback.command_id,
        command_id,
        bundle.actuator_readback.sequence,
        3,
        CausalExecutionStage::ActuatorReadback,
    )?;
    validate_event_identity(
        &bundle.physical_transition.event_id,
        &bundle.physical_transition.command_id,
        command_id,
        bundle.physical_transition.sequence,
        4,
        CausalExecutionStage::PhysicalTransition,
    )?;
    validate_event_identity(
        &bundle.sensor_acquisition.event_id,
        &bundle.sensor_acquisition.command_id,
        command_id,
        bundle.sensor_acquisition.sequence,
        5,
        CausalExecutionStage::SensorAcquisition,
    )?;
    validate_event_identity(
        &bundle.inference.event_id,
        &bundle.inference.command_id,
        command_id,
        bundle.inference.sequence,
        6,
        CausalExecutionStage::Inference,
    )?;
    validate_event_identity(
        &bundle.decision.event_id,
        &bundle.decision.command_id,
        command_id,
        bundle.decision.sequence,
        7,
        CausalExecutionStage::Decision,
    )?;

    for (device_id, stage, sequence) in [
        (
            bundle.dispatch.device_id.as_str(),
            CausalExecutionStage::Dispatch,
            1,
        ),
        (
            bundle.acknowledgement.device_id.as_str(),
            CausalExecutionStage::Acknowledgement,
            2,
        ),
        (
            bundle.actuator_readback.device_id.as_str(),
            CausalExecutionStage::ActuatorReadback,
            3,
        ),
    ] {
        if device_id != command.device_id {
            return Err(CausalExecutionError {
                stage,
                sequence: Some(sequence),
                reason: "device identity differs from the command target".into(),
            });
        }
    }

    for (value, label, stage, sequence) in [
        (
            &bundle.dispatch.transport_id,
            "transport ID",
            CausalExecutionStage::Dispatch,
            1,
        ),
        (
            &bundle.dispatch.payload_digest,
            "dispatch payload digest",
            CausalExecutionStage::Dispatch,
            1,
        ),
        (
            &bundle.acknowledgement.firmware_identity,
            "firmware identity",
            CausalExecutionStage::Acknowledgement,
            2,
        ),
        (
            &bundle.actuator_readback.calibration_receipt_id,
            "actuator calibration receipt",
            CausalExecutionStage::ActuatorReadback,
            3,
        ),
        (
            &bundle.physical_transition.parent_actuator_event_id,
            "physical-transition parent ID",
            CausalExecutionStage::PhysicalTransition,
            4,
        ),
        (
            &bundle.physical_transition.pre_state_digest,
            "pre-transition state digest",
            CausalExecutionStage::PhysicalTransition,
            4,
        ),
        (
            &bundle.physical_transition.post_state_digest,
            "post-transition state digest",
            CausalExecutionStage::PhysicalTransition,
            4,
        ),
        (
            &bundle.physical_transition.model_version,
            "physical model version",
            CausalExecutionStage::PhysicalTransition,
            4,
        ),
        (
            &bundle.sensor_acquisition.parent_transition_event_id,
            "sensor transition parent ID",
            CausalExecutionStage::SensorAcquisition,
            5,
        ),
        (
            &bundle.sensor_acquisition.instrument_id,
            "instrument ID",
            CausalExecutionStage::SensorAcquisition,
            5,
        ),
        (
            &bundle.sensor_acquisition.raw_sample_digest,
            "raw sample digest",
            CausalExecutionStage::SensorAcquisition,
            5,
        ),
        (
            &bundle.sensor_acquisition.calibration_receipt_id,
            "sensor calibration receipt",
            CausalExecutionStage::SensorAcquisition,
            5,
        ),
        (
            &bundle.inference.inference_version,
            "inference version",
            CausalExecutionStage::Inference,
            6,
        ),
        (
            &bundle.inference.conclusion_digest,
            "inference conclusion digest",
            CausalExecutionStage::Inference,
            6,
        ),
        (
            &bundle.inference.uncertainty_digest,
            "inference uncertainty digest",
            CausalExecutionStage::Inference,
            6,
        ),
        (
            &bundle.decision.inference_event_id,
            "decision inference reference",
            CausalExecutionStage::Decision,
            7,
        ),
        (
            &bundle.decision.policy_identity,
            "policy identity",
            CausalExecutionStage::Decision,
            7,
        ),
        (
            &bundle.decision.decision_digest,
            "decision digest",
            CausalExecutionStage::Decision,
            7,
        ),
    ] {
        if !valid_id(value) {
            return Err(CausalExecutionError {
                stage,
                sequence: Some(sequence),
                reason: format!("{label} is empty or overlong"),
            });
        }
    }

    if bundle.physical_transition.parent_actuator_event_id != bundle.actuator_readback.event_id
        || bundle.sensor_acquisition.parent_transition_event_id != bundle.physical_transition.event_id
        || bundle.inference.sensor_event_ids != vec![bundle.sensor_acquisition.event_id.clone()]
        || bundle.decision.inference_event_id != bundle.inference.event_id
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::FinalConsistency,
            sequence: None,
            reason: "causal parent references do not form one consistent command-to-decision chain".into(),
        });
    }
    if bundle.dispatch.payload_digest.trim().is_empty()
        || bundle.physical_transition.pre_state_digest == bundle.physical_transition.post_state_digest
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::FinalConsistency,
            sequence: None,
            reason: "dispatch payload or physical transition evidence is invalid".into(),
        });
    }

    for index in 1..events.len() {
        require_strict_temporal_precedence(
            events[index - 1].2,
            events[index].2,
            match index {
                1 => CausalExecutionStage::Dispatch,
                2 => CausalExecutionStage::Acknowledgement,
                3 => CausalExecutionStage::ActuatorReadback,
                4 => CausalExecutionStage::PhysicalTransition,
                5 => CausalExecutionStage::SensorAcquisition,
                6 => CausalExecutionStage::Inference,
                _ => CausalExecutionStage::Decision,
            },
            events[index].1,
        )?;
    }

    let acknowledgement = bundle.acknowledgement.state;
    if bundle.dispatch.state == DispatchState::Rejected
        && acknowledgement == AcknowledgementState::Accepted
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::Acknowledgement,
            sequence: Some(2),
            reason: "transport rejection conflicts with device acceptance".into(),
        });
    }
    if bundle.dispatch.state == DispatchState::Accepted
        && acknowledgement == AcknowledgementState::Rejected
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::Acknowledgement,
            sequence: Some(2),
            reason: "transport acceptance conflicts with device rejection".into(),
        });
    }
    if acknowledgement == AcknowledgementState::Accepted
        && bundle.acknowledgement.echoed_value != Some(command.requested_value)
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::Acknowledgement,
            sequence: Some(2),
            reason: "device acknowledgement does not echo the intended setpoint".into(),
        });
    }
    if acknowledgement != AcknowledgementState::Accepted
        && bundle.acknowledgement.echoed_value.is_some()
    {
        return Err(CausalExecutionError {
            stage: CausalExecutionStage::Acknowledgement,
            sequence: Some(2),
            reason: "non-accepted acknowledgement contains an accepted setpoint echo".into(),
        });
    }

    let realization_assessment = if acknowledgement == AcknowledgementState::Accepted {
        assess_realization(command, &bundle.actuator_readback).map_err(|reason| {
            CausalExecutionError {
                stage: CausalExecutionStage::ActuatorReadback,
                sequence: Some(3),
                reason,
            }
        })?
    } else {
        // A plausible actuator reading is not attributable to this command
        // unless the device acknowledged the exact intended setpoint.
        RealizationAssessment::Unattributed
    };
    let decision_status = if bundle.decision.abstained {
        DecisionEvidenceStatus::Abstained
    } else if bundle.decision.risk_budget_satisfied {
        DecisionEvidenceStatus::RiskBudgetSatisfied
    } else {
        DecisionEvidenceStatus::RiskBudgetViolation
    };

    let calibration_receipt_ids = vec![
        bundle.actuator_readback.calibration_receipt_id.clone(),
        bundle.sensor_acquisition.calibration_receipt_id.clone(),
    ];
    Ok(CausalExecutionAuditReceipt {
        command_id: command_id.into(),
        event_ids: events.iter().map(|(event_id, _, _)| (*event_id).clone()).collect(),
        realization_assessment,
        resolved_delivery: acknowledgement,
        decision_status,
        sequence_complete: events.len() as u64 == EVENT_COUNT,
        timestamp_intervals_strictly_ordered: true,
        causal_references_consistent: true,
        device_identity_consistent: true,
        calibration_receipt_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AcknowledgementState, ActuatorReadback, CausalExecutionBundle, CommandIntent,
        DecisionEvidence, DecisionEvidenceStatus, DispatchReceipt, DispatchState,
        DeviceAcknowledgement, PhysicalTransitionEvidence, RealizationAssessment,
        SensorAcquisition, UncertainTimestamp, InferenceEvidence,
        audit_causal_execution_bundle,
    };

    fn clock(index: u64) -> UncertainTimestamp {
        UncertainTimestamp {
            timestamp_ns: index * 1_000,
            uncertainty_ns: 10,
        }
    }

    fn bundle() -> CausalExecutionBundle {
        CausalExecutionBundle {
            command: CommandIntent {
                event_id: "event-command".into(),
                sequence: 0,
                command_id: "command-1".into(),
                device_id: "heater-1".into(),
                requested_value: 100,
                unit: "watt".into(),
                tolerance: 5,
                operating_minimum: 0,
                operating_maximum: 200,
                clock: clock(1),
            },
            dispatch: DispatchReceipt {
                event_id: "event-dispatch".into(),
                sequence: 1,
                command_id: "command-1".into(),
                device_id: "heater-1".into(),
                transport_id: "transport-1".into(),
                payload_digest: "sha256-payload".into(),
                state: DispatchState::Accepted,
                clock: clock(2),
            },
            acknowledgement: DeviceAcknowledgement {
                event_id: "event-ack".into(),
                sequence: 2,
                command_id: "command-1".into(),
                device_id: "heater-1".into(),
                firmware_identity: "heater-fw-1".into(),
                state: AcknowledgementState::Accepted,
                echoed_value: Some(100),
                clock: clock(3),
            },
            actuator_readback: ActuatorReadback {
                event_id: "event-readback".into(),
                sequence: 3,
                command_id: "command-1".into(),
                device_id: "heater-1".into(),
                measured_value: 102,
                measurement_uncertainty: 1,
                calibration_receipt_id: "actuator-cal-1".into(),
                clock: clock(4),
            },
            physical_transition: PhysicalTransitionEvidence {
                event_id: "event-transition".into(),
                sequence: 4,
                command_id: "command-1".into(),
                parent_actuator_event_id: "event-readback".into(),
                pre_state_digest: "state-before".into(),
                post_state_digest: "state-after".into(),
                model_version: "thermal-model-v1".into(),
                clock: clock(5),
            },
            sensor_acquisition: SensorAcquisition {
                event_id: "event-sensor".into(),
                sequence: 5,
                command_id: "command-1".into(),
                parent_transition_event_id: "event-transition".into(),
                instrument_id: "thermometer-1".into(),
                raw_sample_digest: "raw-sample-sha256".into(),
                calibration_receipt_id: "sensor-cal-1".into(),
                quality_flags: Vec::new(),
                clock: clock(6),
            },
            inference: InferenceEvidence {
                event_id: "event-inference".into(),
                sequence: 6,
                command_id: "command-1".into(),
                sensor_event_ids: vec!["event-sensor".into()],
                inference_version: "inference-v1".into(),
                conclusion_digest: "conclusion-sha256".into(),
                uncertainty_digest: "uncertainty-sha256".into(),
                clock: clock(7),
            },
            decision: DecisionEvidence {
                event_id: "event-decision".into(),
                sequence: 7,
                command_id: "command-1".into(),
                inference_event_id: "event-inference".into(),
                policy_identity: "policy-v1".into(),
                decision_digest: "decision-sha256".into(),
                abstained: false,
                risk_budget_satisfied: true,
                clock: clock(8),
            },
        }
    }

    #[test]
    fn accepts_complete_causal_chain_and_separates_command_ack_from_realization() {
        let receipt = audit_causal_execution_bundle(&bundle()).unwrap();
        assert_eq!(receipt.realization_assessment, RealizationAssessment::WithinTolerance);
        assert_eq!(receipt.resolved_delivery, AcknowledgementState::Accepted);
        assert_eq!(receipt.decision_status, DecisionEvidenceStatus::RiskBudgetSatisfied);
        assert!(receipt.sequence_complete);
        assert!(receipt.timestamp_intervals_strictly_ordered);
        assert!(receipt.causal_references_consistent);
        assert_eq!(receipt.calibration_receipt_ids.len(), 2);
    }

    #[test]
    fn a_matching_actuator_value_without_device_acceptance_is_unattributed() {
        let mut evidence = bundle();
        evidence.dispatch.state = DispatchState::Unknown;
        evidence.acknowledgement.state = AcknowledgementState::Absent;
        evidence.acknowledgement.echoed_value = None;
        let receipt = audit_causal_execution_bundle(&evidence).unwrap();
        assert_eq!(receipt.realization_assessment, RealizationAssessment::Unattributed);
        assert_eq!(receipt.resolved_delivery, AcknowledgementState::Absent);
    }

    #[test]
    fn distinguishes_outside_tolerance_from_measurement_ambiguity() {
        let mut evidence = bundle();
        evidence.actuator_readback.measured_value = 80;
        evidence.actuator_readback.measurement_uncertainty = 1;
        assert_eq!(
            audit_causal_execution_bundle(&evidence).unwrap().realization_assessment,
            RealizationAssessment::OutsideTolerance
        );

        evidence.actuator_readback.measured_value = 95;
        evidence.actuator_readback.measurement_uncertainty = 1;
        assert_eq!(
            audit_causal_execution_bundle(&evidence).unwrap().realization_assessment,
            RealizationAssessment::MeasurementAmbiguous
        );
    }

    #[test]
    fn rejects_acknowledgement_echo_mismatch_and_device_identity_substitution() {
        let mut evidence = bundle();
        evidence.acknowledgement.echoed_value = Some(99);
        assert!(audit_causal_execution_bundle(&evidence).is_err());

        evidence = bundle();
        evidence.actuator_readback.device_id = "other-heater".into();
        assert!(audit_causal_execution_bundle(&evidence).is_err());
    }

    #[test]
    fn rejects_overlapping_clock_uncertainty_and_stale_sensor_causality() {
        let mut evidence = bundle();
        evidence.dispatch.clock.uncertainty_ns = 1_100;
        assert!(audit_causal_execution_bundle(&evidence).is_err());

        evidence = bundle();
        evidence.sensor_acquisition.clock = clock(4);
        assert!(audit_causal_execution_bundle(&evidence).is_err());
    }

    #[test]
    fn rejects_duplicate_event_ids_and_operating_envelope_violations() {
        let mut evidence = bundle();
        evidence.decision.event_id = evidence.command.event_id.clone();
        assert!(audit_causal_execution_bundle(&evidence).is_err());

        evidence = bundle();
        evidence.command.requested_value = 250;
        assert!(audit_causal_execution_bundle(&evidence).is_err());
    }

    #[test]
    fn preserves_abstention_and_risk_budget_violation_as_distinct_decisions() {
        let mut evidence = bundle();
        evidence.decision.abstained = true;
        evidence.decision.risk_budget_satisfied = false;
        assert_eq!(
            audit_causal_execution_bundle(&evidence).unwrap().decision_status,
            DecisionEvidenceStatus::Abstained
        );

        evidence.decision.abstained = false;
        assert_eq!(
            audit_causal_execution_bundle(&evidence).unwrap().decision_status,
            DecisionEvidenceStatus::RiskBudgetViolation
        );
    }
}
