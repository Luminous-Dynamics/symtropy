//! Evidence-preserving adapter for instrumented single-joint traces.
//!
//! This crate normalizes measured bench evidence into a causal replay fragment.
//! It deliberately does not implement actuator, thermal, first-law, entropy,
//! viability, calibration, controller or actuation authority models.
//!
//! Unknown clocks remain Unknown; incomplete energy boundaries remain
//! Inconclusive; physical-state resets and identity contradictions are rejected.
//! The adapter can never authorize actuator commands.

#![forbid(unsafe_code)]

pub mod wire;

use std::collections::BTreeSet;

/// Closed interval for an event timestamp in one declared monotonic clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampInterval {
    pub clock_id: String,
    pub boot_epoch: u64,
    pub lower_ns: u64,
    pub upper_ns: u64,
}

impl TimestampInterval {
    fn validate(&self, field: &'static str) -> Result<(), TraceValidationError> {
        if self.clock_id.trim().is_empty() {
            return Err(TraceValidationError::EmptyField(field));
        }
        if self.lower_ns > self.upper_ns {
            return Err(TraceValidationError::InvalidTimestampInterval {
                field,
                lower_ns: self.lower_ns,
                upper_ns: self.upper_ns,
            });
        }
        Ok(())
    }
}

/// Qualified mapping from the source-observation clock into the actuator clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockMapping {
    pub source_clock_id: String,
    pub target_clock_id: String,
    pub boot_epoch: u64,
    /// Target-clock time minus source-clock time, with bounded uncertainty.
    pub offset_lower_ns: i64,
    pub offset_upper_ns: i64,
    pub mapping_digest: String,
}

/// Clock relation for source capture to actuator application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockRelation {
    SameMonotonicClockVerified,
    BoundedMappingVerified(ClockMapping),
    Unknown,
}

/// Stable IDs and strictly ordered event sequence numbers from one mission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalIdentity {
    pub mission_id: String,
    pub robot_id: String,
    pub boot_epoch: u64,
    pub scenario_manifest_digest: String,
    pub source_observation_id: String,
    pub source_event_sequence: u64,
    pub decision_id: String,
    pub decision_event_sequence: u64,
    pub proposal_id: String,
    pub proposal_event_sequence: u64,
    pub assurance_event_id: String,
    pub assurance_event_sequence: u64,
    pub command_id: String,
    pub command_event_sequence: u64,
    pub actuator_application_event_id: String,
    pub actuator_application_event_sequence: u64,
    pub actuator_application_receipt_digest: String,
    pub source_sensor_calibration_digest: String,
    pub physical_transition_id: String,
    pub physical_transition_event_sequence: u64,
    pub terminal_event_id: String,
    pub terminal_event_sequence: u64,
    pub actuator_id: String,
}

/// Configuration and calibration identities fixed for one comparison stratum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationEvidence {
    pub actuator_configuration_digest: String,
    pub controller_configuration_digest: String,
    pub policy_digest: String,
    pub model_digest: String,
    pub action_domain_digest: String,
    pub sensor_calibration_digest: String,
    pub torque_transducer_calibration_digest: String,
    pub power_analyzer_calibration_digest: String,
    pub thermal_sensor_calibration_digest: String,
    pub sampling_spec_digest: String,
    pub energy_boundary_manifest_digest: String,
    pub actuator_application_receipt_digest: String,
    pub executed_prefix_expected_digest: String,
    pub executed_prefix_observed_digest: String,
    pub decision_calibration_receipt_digest: String,
}

/// Upstream decision-sufficiency result bound to the actuator application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionEvidence {
    pub decision_id: String,
    pub model_digest: String,
    pub action_domain_digest: String,
    pub calibration_receipt_digest: String,
    pub regret_upper_units: u64,
    pub epsilon_task_loss_units: u64,
    pub valid_through_tick: u64,
    pub actuator_application_tick: u64,
}

/// Per-node transition evidence from the upstream qualified thermal model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalEvidence {
    pub physical_transition_id: String,
    pub sensor_calibration_digest: String,
    pub required_node_ids: Vec<String>,
    pub observed_node_ids: Vec<String>,
    pub expected_predecessor_digest: String,
    pub observed_predecessor_digest: String,
    pub result_digest: String,
    pub sequence_reset: bool,
}

/// Output of the upstream energy-boundary / first-law owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpstreamEnergyClosure {
    Balanced { receipt_digest: String },
    Inconclusive { reason: String },
    Invalid { reason: String },
    Unknown,
}

/// Complete declared boundary plus the independent first-law receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyEvidence {
    pub boundary_complete: bool,
    pub boundary_manifest_digest: String,
    pub closure: UpstreamEnergyClosure,
}

/// Sampling qualification is supplied by the instrument specification owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingEvidence {
    pub measured_rate_hz: u64,
    pub required_rate_hz: u64,
    pub specification_digest: String,
    pub specification_qualified: bool,
}

/// One normalized joint trace. Raw artifacts remain external and digest-bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointTrace {
    pub identity: CausalIdentity,
    pub configuration: ConfigurationEvidence,
    pub clock_relation: ClockRelation,
    pub source_capture: TimestampInterval,
    pub actuator_application: TimestampInterval,
    pub motion_onset: TimestampInterval,
    pub decision: DecisionEvidence,
    pub thermal: ThermalEvidence,
    pub energy: EnergyEvidence,
    pub sampling: SamplingEvidence,
    pub terminal_safe_stop_qualified: bool,
    /// Terminal physical reserve-state digest bound to the safe-stop receipt.
    pub terminal_reserve_state_digest: String,
}

/// An elapsed-time interval whose bounds include timestamp uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurationBounds {
    pub lower_ns: u64,
    pub upper_ns: u64,
}

/// Qualification status. The variants deliberately do not collapse into bool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence<T> {
    Qualified(T),
    Unknown(UnknownReason),
    Inconclusive(InconclusiveReason),
    Rejected(RejectReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownReason {
    ClockRelationUnavailable,
    ClockMappingDoesNotCoverTrace,
    TimestampOrderOverlaps,
    DecisionCalibrationUnavailable,
    ThermalNodeCoverageIncomplete,
    EnergyReceiptUnavailable,
    SamplingSpecificationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InconclusiveReason {
    EnergyBoundaryIncomplete,
    TerminalSafeStopNotQualified,
    UpstreamEnergyClosureInconclusive(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    TimestampOrderContradiction,
    ThermalSequenceReset,
    ThermalPredecessorMismatch,
    EnergyBoundaryInvalid(String),
    DecisionHorizonExpired {
        valid_through_tick: u64,
        application_tick: u64,
    },
    DecisionRegretExceeded {
        regret_upper_units: u64,
        epsilon_task_loss_units: u64,
    },
    ExecutedPrefixMismatch,
    ReceiptBindingMismatch(&'static str),
    SamplingRateBelowSpecification {
        measured_rate_hz: u64,
        required_rate_hz: u64,
    },
    UpstreamEnergyClosureInvalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionDisposition {
    Sufficient {
        regret_upper_units: u64,
    },
    Insufficient {
        regret_upper_units: u64,
        epsilon_task_loss_units: u64,
    },
    Expired {
        valid_through_tick: u64,
        application_tick: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThermalDisposition {
    Continuous {
        node_count: usize,
        result_digest: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyDisposition {
    Balanced { first_law_receipt_digest: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SamplingDisposition {
    MeetsDeclaredSpecification { measured_rate_hz: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalEdge {
    pub from_event_id: String,
    pub to_event_id: String,
    pub from_sequence: u64,
    pub to_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionClosure {
    Closed,
    Open {
        unqualified_domains: Vec<&'static str>,
    },
}

/// Output for Symthaea's offline causal mission replay adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReport {
    pub mission_id: String,
    pub robot_id: String,
    pub actuator_id: String,
    pub source_to_actuator_aoi: Evidence<DurationBounds>,
    pub command_to_motion: Evidence<DurationBounds>,
    pub decision: Evidence<DecisionDisposition>,
    pub thermal: Evidence<ThermalDisposition>,
    pub energy: Evidence<EnergyDisposition>,
    pub sampling: Evidence<SamplingDisposition>,
    pub terminal_safe_stop: Evidence<()>,
    pub causal_edges: Vec<CausalEdge>,
    pub causal_chain_complete: bool,
    pub mission_closure: MissionClosure,
    /// This is a retrospective evidence adapter, never a runtime safety authority.
    pub actuation_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceValidationError {
    EmptyField(&'static str),
    DuplicateEventIdentity(String),
    EventSequenceNotIncreasing {
        preceding: &'static str,
        following: &'static str,
        preceding_sequence: u64,
        following_sequence: u64,
    },
    InvalidTimestampInterval {
        field: &'static str,
        lower_ns: u64,
        upper_ns: u64,
    },
    TimestampBootEpochMismatch {
        field: &'static str,
        expected: u64,
        observed: u64,
    },
    ReceiptBindingMismatch(&'static str),
}

impl std::fmt::Display for TraceValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TraceValidationError {}

fn require_nonempty(fields: &[(&'static str, &str)]) -> Result<(), TraceValidationError> {
    for (name, value) in fields {
        if value.trim().is_empty() {
            return Err(TraceValidationError::EmptyField(name));
        }
    }
    Ok(())
}

fn validate_identity(
    identity: &CausalIdentity,
    configuration: &ConfigurationEvidence,
) -> Result<Vec<CausalEdge>, TraceValidationError> {
    require_nonempty(&[
        ("mission_id", &identity.mission_id),
        ("robot_id", &identity.robot_id),
        (
            "scenario_manifest_digest",
            &identity.scenario_manifest_digest,
        ),
        ("source_observation_id", &identity.source_observation_id),
        ("decision_id", &identity.decision_id),
        ("proposal_id", &identity.proposal_id),
        ("assurance_event_id", &identity.assurance_event_id),
        ("command_id", &identity.command_id),
        (
            "actuator_application_event_id",
            &identity.actuator_application_event_id,
        ),
        (
            "actuator_application_receipt_digest",
            &identity.actuator_application_receipt_digest,
        ),
        (
            "source_sensor_calibration_digest",
            &identity.source_sensor_calibration_digest,
        ),
        ("physical_transition_id", &identity.physical_transition_id),
        ("terminal_event_id", &identity.terminal_event_id),
        ("actuator_id", &identity.actuator_id),
    ])?;
    require_nonempty(&[
        (
            "actuator_configuration_digest",
            &configuration.actuator_configuration_digest,
        ),
        (
            "controller_configuration_digest",
            &configuration.controller_configuration_digest,
        ),
        ("policy_digest", &configuration.policy_digest),
        ("model_digest", &configuration.model_digest),
        ("action_domain_digest", &configuration.action_domain_digest),
        (
            "sensor_calibration_digest",
            &configuration.sensor_calibration_digest,
        ),
        (
            "torque_transducer_calibration_digest",
            &configuration.torque_transducer_calibration_digest,
        ),
        (
            "power_analyzer_calibration_digest",
            &configuration.power_analyzer_calibration_digest,
        ),
        (
            "thermal_sensor_calibration_digest",
            &configuration.thermal_sensor_calibration_digest,
        ),
        ("sampling_spec_digest", &configuration.sampling_spec_digest),
        (
            "energy_boundary_manifest_digest",
            &configuration.energy_boundary_manifest_digest,
        ),
        (
            "actuator_application_receipt_digest",
            &configuration.actuator_application_receipt_digest,
        ),
        (
            "executed_prefix_expected_digest",
            &configuration.executed_prefix_expected_digest,
        ),
        (
            "executed_prefix_observed_digest",
            &configuration.executed_prefix_observed_digest,
        ),
    ])?;
    if identity.actuator_application_receipt_digest
        != configuration.actuator_application_receipt_digest
    {
        return Err(TraceValidationError::ReceiptBindingMismatch(
            "actuator-application-receipt",
        ));
    }
    if identity.source_sensor_calibration_digest != configuration.sensor_calibration_digest {
        return Err(TraceValidationError::ReceiptBindingMismatch(
            "source-sensor-calibration",
        ));
    }

    let events = [
        (
            "source",
            identity.source_observation_id.as_str(),
            identity.source_event_sequence,
        ),
        (
            "decision",
            identity.decision_id.as_str(),
            identity.decision_event_sequence,
        ),
        (
            "proposal",
            identity.proposal_id.as_str(),
            identity.proposal_event_sequence,
        ),
        (
            "assurance",
            identity.assurance_event_id.as_str(),
            identity.assurance_event_sequence,
        ),
        (
            "command",
            identity.command_id.as_str(),
            identity.command_event_sequence,
        ),
        (
            "application",
            identity.actuator_application_event_id.as_str(),
            identity.actuator_application_event_sequence,
        ),
        (
            "physical",
            identity.physical_transition_id.as_str(),
            identity.physical_transition_event_sequence,
        ),
        (
            "terminal",
            identity.terminal_event_id.as_str(),
            identity.terminal_event_sequence,
        ),
    ];
    let mut unique_ids = BTreeSet::new();
    for (_, event_id, _) in events {
        if !unique_ids.insert(event_id) {
            return Err(TraceValidationError::DuplicateEventIdentity(
                event_id.to_owned(),
            ));
        }
    }
    for pair in events.windows(2) {
        let (from_name, from_id, from_sequence) = pair[0];
        let (to_name, to_id, to_sequence) = pair[1];
        if from_sequence >= to_sequence {
            return Err(TraceValidationError::EventSequenceNotIncreasing {
                preceding: from_name,
                following: to_name,
                preceding_sequence: from_sequence,
                following_sequence: to_sequence,
            });
        }
        if from_id == to_id {
            return Err(TraceValidationError::DuplicateEventIdentity(
                from_id.to_owned(),
            ));
        }
    }
    Ok(events
        .windows(2)
        .map(|pair| CausalEdge {
            from_event_id: pair[0].1.to_owned(),
            to_event_id: pair[1].1.to_owned(),
            from_sequence: pair[0].2,
            to_sequence: pair[1].2,
        })
        .collect())
}

fn validate_timestamp(
    field: &'static str,
    timestamp: &TimestampInterval,
    expected_boot_epoch: u64,
) -> Result<(), TraceValidationError> {
    timestamp.validate(field)?;
    if timestamp.boot_epoch != expected_boot_epoch {
        return Err(TraceValidationError::TimestampBootEpochMismatch {
            field,
            expected: expected_boot_epoch,
            observed: timestamp.boot_epoch,
        });
    }
    Ok(())
}

fn apply_mapping(
    source: &TimestampInterval,
    target_clock_id: &str,
    mapping: &ClockMapping,
) -> Option<TimestampInterval> {
    if mapping.source_clock_id != source.clock_id
        || mapping.target_clock_id != target_clock_id
        || mapping.boot_epoch != source.boot_epoch
        || mapping.mapping_digest.trim().is_empty()
        || mapping.offset_lower_ns > mapping.offset_upper_ns
    {
        return None;
    }
    let lower = i128::from(source.lower_ns) + i128::from(mapping.offset_lower_ns);
    let upper = i128::from(source.upper_ns) + i128::from(mapping.offset_upper_ns);
    if lower < 0 || upper < lower || upper > i128::from(u64::MAX) {
        return None;
    }
    Some(TimestampInterval {
        clock_id: target_clock_id.to_owned(),
        boot_epoch: source.boot_epoch,
        lower_ns: lower as u64,
        upper_ns: upper as u64,
    })
}

fn elapsed_in_one_clock(
    start: &TimestampInterval,
    end: &TimestampInterval,
) -> Evidence<DurationBounds> {
    if start.boot_epoch != end.boot_epoch || start.clock_id != end.clock_id {
        return Evidence::Unknown(UnknownReason::ClockMappingDoesNotCoverTrace);
    }
    if end.upper_ns < start.lower_ns {
        return Evidence::Rejected(RejectReason::TimestampOrderContradiction);
    }
    if end.lower_ns < start.upper_ns {
        return Evidence::Unknown(UnknownReason::TimestampOrderOverlaps);
    }
    Evidence::Qualified(DurationBounds {
        lower_ns: end.lower_ns - start.upper_ns,
        upper_ns: end.upper_ns - start.lower_ns,
    })
}

fn source_to_application_aoi(trace: &JointTrace) -> Evidence<DurationBounds> {
    match &trace.clock_relation {
        ClockRelation::Unknown => Evidence::Unknown(UnknownReason::ClockRelationUnavailable),
        ClockRelation::SameMonotonicClockVerified => {
            elapsed_in_one_clock(&trace.source_capture, &trace.actuator_application)
        }
        ClockRelation::BoundedMappingVerified(mapping) => {
            let Some(mapped_source) = apply_mapping(
                &trace.source_capture,
                &trace.actuator_application.clock_id,
                mapping,
            ) else {
                return Evidence::Unknown(UnknownReason::ClockMappingDoesNotCoverTrace);
            };
            elapsed_in_one_clock(&mapped_source, &trace.actuator_application)
        }
    }
}

fn command_to_motion(trace: &JointTrace) -> Evidence<DurationBounds> {
    elapsed_in_one_clock(&trace.actuator_application, &trace.motion_onset)
}

fn decision_status(trace: &JointTrace) -> Evidence<DecisionDisposition> {
    if trace
        .configuration
        .decision_calibration_receipt_digest
        .trim()
        .is_empty()
        || trace.decision.calibration_receipt_digest.trim().is_empty()
    {
        return Evidence::Unknown(UnknownReason::DecisionCalibrationUnavailable);
    }
    if trace.decision.decision_id != trace.identity.decision_id
        || trace.decision.model_digest != trace.configuration.model_digest
        || trace.decision.action_domain_digest != trace.configuration.action_domain_digest
        || trace.decision.calibration_receipt_digest
            != trace.configuration.decision_calibration_receipt_digest
    {
        return Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
            "decision-calibration-or-scope",
        ));
    }
    if trace.configuration.executed_prefix_expected_digest
        != trace.configuration.executed_prefix_observed_digest
    {
        return Evidence::Rejected(RejectReason::ExecutedPrefixMismatch);
    }
    if trace.decision.actuator_application_tick > trace.decision.valid_through_tick {
        return Evidence::Rejected(RejectReason::DecisionHorizonExpired {
            valid_through_tick: trace.decision.valid_through_tick,
            application_tick: trace.decision.actuator_application_tick,
        });
    }
    if trace.decision.regret_upper_units > trace.decision.epsilon_task_loss_units {
        return Evidence::Qualified(DecisionDisposition::Insufficient {
            regret_upper_units: trace.decision.regret_upper_units,
            epsilon_task_loss_units: trace.decision.epsilon_task_loss_units,
        });
    }
    Evidence::Qualified(DecisionDisposition::Sufficient {
        regret_upper_units: trace.decision.regret_upper_units,
    })
}

fn thermal_status(trace: &JointTrace) -> Evidence<ThermalDisposition> {
    let thermal = &trace.thermal;
    if thermal.sequence_reset {
        return Evidence::Rejected(RejectReason::ThermalSequenceReset);
    }
    if thermal.expected_predecessor_digest.trim().is_empty()
        || thermal.observed_predecessor_digest.trim().is_empty()
        || thermal.result_digest.trim().is_empty()
        || thermal.physical_transition_id.trim().is_empty()
        || thermal.sensor_calibration_digest.trim().is_empty()
    {
        return Evidence::Unknown(UnknownReason::ThermalNodeCoverageIncomplete);
    }
    if thermal.expected_predecessor_digest != thermal.observed_predecessor_digest {
        return Evidence::Rejected(RejectReason::ThermalPredecessorMismatch);
    }
    if thermal.physical_transition_id != trace.identity.physical_transition_id
        || thermal.sensor_calibration_digest
            != trace.configuration.thermal_sensor_calibration_digest
    {
        return Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
            "thermal-transition-or-calibration",
        ));
    }
    if thermal.required_node_ids.is_empty() {
        return Evidence::Unknown(UnknownReason::ThermalNodeCoverageIncomplete);
    }
    let required: BTreeSet<&str> = thermal
        .required_node_ids
        .iter()
        .map(String::as_str)
        .collect();
    let observed: BTreeSet<&str> = thermal
        .observed_node_ids
        .iter()
        .map(String::as_str)
        .collect();
    if required.len() != thermal.required_node_ids.len()
        || observed.len() != thermal.observed_node_ids.len()
        || required != observed
    {
        return Evidence::Unknown(UnknownReason::ThermalNodeCoverageIncomplete);
    }
    Evidence::Qualified(ThermalDisposition::Continuous {
        node_count: required.len(),
        result_digest: thermal.result_digest.clone(),
    })
}

fn energy_status(trace: &JointTrace) -> Evidence<EnergyDisposition> {
    let energy = &trace.energy;
    if !energy.boundary_complete {
        return Evidence::Inconclusive(InconclusiveReason::EnergyBoundaryIncomplete);
    }
    if energy.boundary_manifest_digest.trim().is_empty() {
        return Evidence::Unknown(UnknownReason::EnergyReceiptUnavailable);
    }
    if energy.boundary_manifest_digest != trace.configuration.energy_boundary_manifest_digest {
        return Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
            "energy-boundary-manifest",
        ));
    }
    match &energy.closure {
        UpstreamEnergyClosure::Balanced { receipt_digest } => {
            if receipt_digest.trim().is_empty() {
                Evidence::Unknown(UnknownReason::EnergyReceiptUnavailable)
            } else {
                Evidence::Qualified(EnergyDisposition::Balanced {
                    first_law_receipt_digest: receipt_digest.clone(),
                })
            }
        }
        UpstreamEnergyClosure::Inconclusive { reason } => Evidence::Inconclusive(
            InconclusiveReason::UpstreamEnergyClosureInconclusive(reason.clone()),
        ),
        UpstreamEnergyClosure::Invalid { reason } => {
            Evidence::Rejected(RejectReason::UpstreamEnergyClosureInvalid(reason.clone()))
        }
        UpstreamEnergyClosure::Unknown => {
            Evidence::Unknown(UnknownReason::EnergyReceiptUnavailable)
        }
    }
}

fn sampling_status(trace: &JointTrace) -> Evidence<SamplingDisposition> {
    let sampling = &trace.sampling;
    if !sampling.specification_qualified || sampling.specification_digest.trim().is_empty() {
        return Evidence::Unknown(UnknownReason::SamplingSpecificationUnavailable);
    }
    if sampling.required_rate_hz == 0 || sampling.measured_rate_hz == 0 {
        return Evidence::Unknown(UnknownReason::SamplingSpecificationUnavailable);
    }
    if sampling.specification_digest != trace.configuration.sampling_spec_digest {
        return Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
            "sampling-specification",
        ));
    }
    if sampling.measured_rate_hz < sampling.required_rate_hz {
        return Evidence::Rejected(RejectReason::SamplingRateBelowSpecification {
            measured_rate_hz: sampling.measured_rate_hz,
            required_rate_hz: sampling.required_rate_hz,
        });
    }
    Evidence::Qualified(SamplingDisposition::MeetsDeclaredSpecification {
        measured_rate_hz: sampling.measured_rate_hz,
    })
}

/// Convert one normalized physical trace into the causal replay evidence fragment.
///
/// Structural identity contradictions return an error. Incomplete measurements
/// remain Unknown or Inconclusive. This function never grants actuation authority.
pub fn adapt_joint_trace(trace: &JointTrace) -> Result<AdapterReport, TraceValidationError> {
    let causal_edges = validate_identity(&trace.identity, &trace.configuration)?;
    validate_timestamp(
        "source_capture",
        &trace.source_capture,
        trace.identity.boot_epoch,
    )?;
    validate_timestamp(
        "actuator_application",
        &trace.actuator_application,
        trace.identity.boot_epoch,
    )?;
    validate_timestamp(
        "motion_onset",
        &trace.motion_onset,
        trace.identity.boot_epoch,
    )?;

    let aoi = source_to_application_aoi(trace);
    let response = command_to_motion(trace);
    let decision = decision_status(trace);
    let thermal = thermal_status(trace);
    let energy = energy_status(trace);
    let sampling = sampling_status(trace);
    let terminal = if trace.terminal_safe_stop_qualified {
        Evidence::Qualified(())
    } else {
        Evidence::Inconclusive(InconclusiveReason::TerminalSafeStopNotQualified)
    };

    let domains = [
        (
            "source-to-actuator AoI",
            matches!(aoi, Evidence::Qualified(_)),
        ),
        (
            "command-to-motion response",
            matches!(response, Evidence::Qualified(_)),
        ),
        (
            "decision sufficiency",
            matches!(
                decision,
                Evidence::Qualified(DecisionDisposition::Sufficient { .. })
            ),
        ),
        (
            "thermal continuity",
            matches!(thermal, Evidence::Qualified(_)),
        ),
        ("energy closure", matches!(energy, Evidence::Qualified(_))),
        ("sampling", matches!(sampling, Evidence::Qualified(_))),
        (
            "terminal safe-stop",
            matches!(terminal, Evidence::Qualified(_)),
        ),
    ];
    let unqualified_domains: Vec<&'static str> = domains
        .into_iter()
        .filter_map(|(name, qualified)| (!qualified).then_some(name))
        .collect();
    let mission_closure = if unqualified_domains.is_empty() && causal_edges.len() == 7 {
        MissionClosure::Closed
    } else {
        MissionClosure::Open {
            unqualified_domains,
        }
    };

    Ok(AdapterReport {
        mission_id: trace.identity.mission_id.clone(),
        robot_id: trace.identity.robot_id.clone(),
        actuator_id: trace.identity.actuator_id.clone(),
        source_to_actuator_aoi: aoi,
        command_to_motion: response,
        decision,
        thermal,
        energy,
        sampling,
        terminal_safe_stop: terminal,
        causal_chain_complete: causal_edges.len() == 7,
        causal_edges,
        mission_closure,
        actuation_authorized: false,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn reference_trace() -> JointTrace {
        JointTrace {
            identity: CausalIdentity {
                mission_id: "mission-01".into(),
                robot_id: "joint-bench-01".into(),
                boot_epoch: 7,
                scenario_manifest_digest: "scenario-sha256".into(),
                source_observation_id: "observation-01".into(),
                source_event_sequence: 1,
                decision_id: "decision-01".into(),
                decision_event_sequence: 2,
                proposal_id: "proposal-01".into(),
                proposal_event_sequence: 3,
                assurance_event_id: "assurance-01".into(),
                assurance_event_sequence: 4,
                command_id: "command-01".into(),
                command_event_sequence: 5,
                actuator_application_event_id: "application-01".into(),
                actuator_application_event_sequence: 6,
                actuator_application_receipt_digest: "application-receipt".into(),
                source_sensor_calibration_digest: "sensor-cal".into(),
                physical_transition_id: "physical-01".into(),
                physical_transition_event_sequence: 7,
                terminal_event_id: "terminal-01".into(),
                terminal_event_sequence: 8,
                actuator_id: "actuator-01".into(),
            },
            configuration: ConfigurationEvidence {
                actuator_configuration_digest: "actuator-config".into(),
                controller_configuration_digest: "controller-config".into(),
                policy_digest: "policy".into(),
                model_digest: "model".into(),
                action_domain_digest: "action-domain".into(),
                sensor_calibration_digest: "sensor-cal".into(),
                torque_transducer_calibration_digest: "torque-cal".into(),
                power_analyzer_calibration_digest: "power-cal".into(),
                thermal_sensor_calibration_digest: "thermal-cal".into(),
                sampling_spec_digest: "sampling-spec".into(),
                energy_boundary_manifest_digest: "energy-boundary".into(),
                actuator_application_receipt_digest: "application-receipt".into(),
                executed_prefix_expected_digest: "prefix-expected".into(),
                executed_prefix_observed_digest: "prefix-expected".into(),
                decision_calibration_receipt_digest: "decision-calibration".into(),
            },
            clock_relation: ClockRelation::SameMonotonicClockVerified,
            source_capture: TimestampInterval {
                clock_id: "joint-clock".into(),
                boot_epoch: 7,
                lower_ns: 100_000_000,
                upper_ns: 100_000_000,
            },
            actuator_application: TimestampInterval {
                clock_id: "joint-clock".into(),
                boot_epoch: 7,
                lower_ns: 110_000_000,
                upper_ns: 110_000_000,
            },
            motion_onset: TimestampInterval {
                clock_id: "joint-clock".into(),
                boot_epoch: 7,
                lower_ns: 125_000_000,
                upper_ns: 125_000_000,
            },
            decision: DecisionEvidence {
                decision_id: "decision-01".into(),
                model_digest: "model".into(),
                action_domain_digest: "action-domain".into(),
                calibration_receipt_digest: "decision-calibration".into(),
                regret_upper_units: 1,
                epsilon_task_loss_units: 2,
                valid_through_tick: 10,
                actuator_application_tick: 3,
            },
            thermal: ThermalEvidence {
                physical_transition_id: "physical-01".into(),
                sensor_calibration_digest: "thermal-cal".into(),
                required_node_ids: vec!["winding".into(), "housing".into(), "drive".into()],
                observed_node_ids: vec!["winding".into(), "housing".into(), "drive".into()],
                expected_predecessor_digest: "thermal-0".into(),
                observed_predecessor_digest: "thermal-0".into(),
                result_digest: "thermal-1".into(),
                sequence_reset: false,
            },
            energy: EnergyEvidence {
                boundary_complete: true,
                boundary_manifest_digest: "energy-boundary".into(),
                closure: UpstreamEnergyClosure::Balanced {
                    receipt_digest: "first-law-receipt".into(),
                },
            },
            sampling: SamplingEvidence {
                measured_rate_hz: 20_000,
                required_rate_hz: 20_000,
                specification_digest: "sampling-spec".into(),
                specification_qualified: true,
            },
            terminal_safe_stop_qualified: true,
            terminal_reserve_state_digest: "thermal-1".into(),
        }
    }

    #[test]
    fn complete_reference_trace_closes_without_authorizing_actuation() {
        let report = adapt_joint_trace(&reference_trace()).unwrap();
        assert_eq!(
            report.source_to_actuator_aoi,
            Evidence::Qualified(DurationBounds {
                lower_ns: 10_000_000,
                upper_ns: 10_000_000,
            })
        );
        assert_eq!(
            report.command_to_motion,
            Evidence::Qualified(DurationBounds {
                lower_ns: 15_000_000,
                upper_ns: 15_000_000,
            })
        );
        assert_eq!(report.causal_edges.len(), 7);
        assert_eq!(report.mission_closure, MissionClosure::Closed);
        assert!(!report.actuation_authorized);
    }

    #[test]
    fn queue_dwell_ages_information_without_changing_command_response() {
        let mut trace = reference_trace();
        trace.actuator_application.lower_ns = 180_000_000;
        trace.actuator_application.upper_ns = 180_000_000;
        trace.motion_onset.lower_ns = 195_000_000;
        trace.motion_onset.upper_ns = 195_000_000;
        let report = adapt_joint_trace(&trace).unwrap();
        assert_eq!(
            report.source_to_actuator_aoi,
            Evidence::Qualified(DurationBounds {
                lower_ns: 80_000_000,
                upper_ns: 80_000_000,
            })
        );
        assert_eq!(
            report.command_to_motion,
            Evidence::Qualified(DurationBounds {
                lower_ns: 15_000_000,
                upper_ns: 15_000_000,
            })
        );
    }

    #[test]
    fn command_response_can_degrade_without_changing_aoi() {
        let mut trace = reference_trace();
        trace.motion_onset.lower_ns = 155_000_000;
        trace.motion_onset.upper_ns = 155_000_000;
        let report = adapt_joint_trace(&trace).unwrap();
        assert_eq!(
            report.source_to_actuator_aoi,
            Evidence::Qualified(DurationBounds {
                lower_ns: 10_000_000,
                upper_ns: 10_000_000,
            })
        );
        assert_eq!(
            report.command_to_motion,
            Evidence::Qualified(DurationBounds {
                lower_ns: 45_000_000,
                upper_ns: 45_000_000,
            })
        );
    }

    #[test]
    fn unknown_clock_relation_blocks_cross_clock_aoi() {
        let mut trace = reference_trace();
        trace.clock_relation = ClockRelation::Unknown;
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().source_to_actuator_aoi,
            Evidence::Unknown(UnknownReason::ClockRelationUnavailable)
        );
    }

    #[test]
    fn bounded_clock_mapping_preserves_interval_uncertainty() {
        let mut trace = reference_trace();
        trace.source_capture.clock_id = "daq-clock".into();
        trace.source_capture.lower_ns = 100;
        trace.source_capture.upper_ns = 102;
        trace.actuator_application.lower_ns = 120;
        trace.actuator_application.upper_ns = 120;
        trace.motion_onset.lower_ns = 130;
        trace.motion_onset.upper_ns = 130;
        trace.clock_relation = ClockRelation::BoundedMappingVerified(ClockMapping {
            source_clock_id: "daq-clock".into(),
            target_clock_id: "joint-clock".into(),
            boot_epoch: 7,
            offset_lower_ns: 8,
            offset_upper_ns: 10,
            mapping_digest: "clock-map".into(),
        });
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().source_to_actuator_aoi,
            Evidence::Qualified(DurationBounds {
                lower_ns: 8,
                upper_ns: 12,
            })
        );
    }

    #[test]
    fn overlapping_timestamp_intervals_are_unknown_not_zero_latency() {
        let mut trace = reference_trace();
        trace.source_capture.lower_ns = 100;
        trace.source_capture.upper_ns = 120;
        trace.actuator_application.lower_ns = 110;
        trace.actuator_application.upper_ns = 130;
        let report = adapt_joint_trace(&trace).unwrap();
        assert_eq!(
            report.source_to_actuator_aoi,
            Evidence::Unknown(UnknownReason::TimestampOrderOverlaps)
        );
    }

    #[test]
    fn definite_timestamp_inversion_is_rejected() {
        let mut trace = reference_trace();
        trace.actuator_application.lower_ns = 80;
        trace.actuator_application.upper_ns = 90;
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().source_to_actuator_aoi,
            Evidence::Rejected(RejectReason::TimestampOrderContradiction)
        );
    }

    #[test]
    fn unknown_energy_boundary_cannot_become_balanced_by_arithmetic() {
        let mut trace = reference_trace();
        trace.energy.boundary_complete = false;
        let report = adapt_joint_trace(&trace).unwrap();
        assert_eq!(
            report.energy,
            Evidence::Inconclusive(InconclusiveReason::EnergyBoundaryIncomplete)
        );
        assert_ne!(report.mission_closure, MissionClosure::Closed);
    }

    #[test]
    fn incomplete_thermal_node_manifest_remains_unknown() {
        let mut trace = reference_trace();
        trace.thermal.observed_node_ids.pop();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().thermal,
            Evidence::Unknown(UnknownReason::ThermalNodeCoverageIncomplete)
        );
    }

    #[test]
    fn predecessor_mismatch_is_rejected() {
        let mut trace = reference_trace();
        trace.thermal.observed_predecessor_digest = "wrong-predecessor".into();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().thermal,
            Evidence::Rejected(RejectReason::ThermalPredecessorMismatch)
        );
    }

    #[test]
    fn thermal_sequence_reset_is_rejected() {
        let mut trace = reference_trace();
        trace.thermal.sequence_reset = true;
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().thermal,
            Evidence::Rejected(RejectReason::ThermalSequenceReset)
        );
    }

    #[test]
    fn decision_regret_expiry_and_prefix_mismatch_are_separate() {
        let mut regret = reference_trace();
        regret.decision.regret_upper_units = 3;
        assert_eq!(
            adapt_joint_trace(&regret).unwrap().decision,
            Evidence::Qualified(DecisionDisposition::Insufficient {
                regret_upper_units: 3,
                epsilon_task_loss_units: 2,
            })
        );

        let mut expiry = reference_trace();
        expiry.decision.valid_through_tick = 2;
        assert_eq!(
            adapt_joint_trace(&expiry).unwrap().decision,
            Evidence::Rejected(RejectReason::DecisionHorizonExpired {
                valid_through_tick: 2,
                application_tick: 3,
            })
        );

        let mut prefix = reference_trace();
        prefix.configuration.executed_prefix_observed_digest = "different-prefix".into();
        assert_eq!(
            adapt_joint_trace(&prefix).unwrap().decision,
            Evidence::Rejected(RejectReason::ExecutedPrefixMismatch)
        );
    }

    #[test]
    fn incomplete_decision_calibration_is_unknown() {
        let mut trace = reference_trace();
        trace
            .configuration
            .decision_calibration_receipt_digest
            .clear();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().decision,
            Evidence::Unknown(UnknownReason::DecisionCalibrationUnavailable)
        );
    }

    #[test]
    fn sampling_rate_must_meet_the_qualified_instrument_specification() {
        let mut trace = reference_trace();
        trace.sampling.measured_rate_hz = 10_000;
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().sampling,
            Evidence::Rejected(RejectReason::SamplingRateBelowSpecification {
                measured_rate_hz: 10_000,
                required_rate_hz: 20_000,
            })
        );
    }

    #[test]
    fn unqualified_sampling_specification_is_unknown() {
        let mut trace = reference_trace();
        trace.sampling.specification_qualified = false;
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().sampling,
            Evidence::Unknown(UnknownReason::SamplingSpecificationUnavailable)
        );
    }

    #[test]
    fn no_qualified_terminal_safe_stop_keeps_mission_open() {
        let mut trace = reference_trace();
        trace.terminal_safe_stop_qualified = false;
        let report = adapt_joint_trace(&trace).unwrap();
        assert_eq!(
            report.terminal_safe_stop,
            Evidence::Inconclusive(InconclusiveReason::TerminalSafeStopNotQualified)
        );
        assert_ne!(report.mission_closure, MissionClosure::Closed);
    }

    #[test]
    fn non_monotonic_causal_sequence_is_a_structural_error() {
        let mut trace = reference_trace();
        trace.identity.assurance_event_sequence = 3;
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::EventSequenceNotIncreasing { .. })
        ));
    }

    #[test]
    fn command_event_must_precede_actuator_application() {
        let mut trace = reference_trace();
        trace.identity.command_event_sequence = trace.identity.actuator_application_event_sequence;
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::EventSequenceNotIncreasing {
                preceding: "command",
                following: "application",
                ..
            })
        ));
    }

    #[test]
    fn command_event_identity_must_be_unique() {
        let mut trace = reference_trace();
        trace.identity.command_id = trace.identity.assurance_event_id.clone();
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::DuplicateEventIdentity(_))
        ));
    }

    #[test]
    fn timestamps_must_belong_to_the_declared_boot_epoch() {
        let mut trace = reference_trace();
        trace.motion_onset.boot_epoch = 8;
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::TimestampBootEpochMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let mut trace = reference_trace();
        trace.identity.proposal_id = trace.identity.decision_id.clone();
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::DuplicateEventIdentity(_))
        ));
    }

    #[test]
    fn malformed_timestamp_interval_is_rejected() {
        let mut trace = reference_trace();
        trace.source_capture.lower_ns = 200;
        trace.source_capture.upper_ns = 100;
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::InvalidTimestampInterval { .. })
        ));
    }

    #[test]
    fn insufficient_decision_cannot_close_the_mission() {
        let mut trace = reference_trace();
        trace.decision.regret_upper_units = 3;
        let report = adapt_joint_trace(&trace).unwrap();
        assert_eq!(
            report.decision,
            Evidence::Qualified(DecisionDisposition::Insufficient {
                regret_upper_units: 3,
                epsilon_task_loss_units: 2,
            })
        );
        assert_ne!(report.mission_closure, MissionClosure::Closed);
    }

    #[test]
    fn decision_receipt_must_bind_to_the_exact_decision_model_and_action_domain() {
        let mut trace = reference_trace();
        trace.decision.action_domain_digest = "different-action-domain".into();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().decision,
            Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
                "decision-calibration-or-scope",
            ))
        );
    }

    #[test]
    fn thermal_receipt_must_bind_to_configuration_and_physical_transition() {
        let mut trace = reference_trace();
        trace.thermal.sensor_calibration_digest = "other-thermal-calibration".into();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().thermal,
            Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
                "thermal-transition-or-calibration",
            ))
        );
    }

    #[test]
    fn energy_receipt_must_bind_to_the_declared_boundary_manifest() {
        let mut trace = reference_trace();
        trace.energy.boundary_manifest_digest = "other-energy-boundary".into();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().energy,
            Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
                "energy-boundary-manifest",
            ))
        );
    }

    #[test]
    fn sampling_receipt_must_bind_to_the_declared_specification() {
        let mut trace = reference_trace();
        trace.sampling.specification_digest = "other-sampling-spec".into();
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().sampling,
            Evidence::Rejected(RejectReason::ReceiptBindingMismatch(
                "sampling-specification",
            ))
        );
    }

    #[test]
    fn actuator_receipt_and_source_sensor_calibration_must_match_configuration() {
        let mut trace = reference_trace();
        trace.identity.actuator_application_receipt_digest = "wrong-receipt".into();
        assert!(matches!(
            adapt_joint_trace(&trace),
            Err(TraceValidationError::ReceiptBindingMismatch(
                "actuator-application-receipt"
            ))
        ));

        let mut sensor = reference_trace();
        sensor.identity.source_sensor_calibration_digest = "wrong-calibration".into();
        assert!(matches!(
            adapt_joint_trace(&sensor),
            Err(TraceValidationError::ReceiptBindingMismatch(
                "source-sensor-calibration"
            ))
        ));
    }

    #[test]
    fn invalid_upstream_energy_receipt_is_not_reinterpreted() {
        let mut trace = reference_trace();
        trace.energy.closure = UpstreamEnergyClosure::Invalid {
            reason: "negative residual outside uncertainty".into(),
        };
        assert_eq!(
            adapt_joint_trace(&trace).unwrap().energy,
            Evidence::Rejected(RejectReason::UpstreamEnergyClosureInvalid(
                "negative residual outside uncertainty".into()
            ))
        );
    }
}
