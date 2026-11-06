// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Trace-based conformance evaluation for operator-defined large-load
//! voltage/frequency ride-through and post-disturbance active-power recovery.
//! This evaluates observable PCC RMS traces; it is not a dynamic simulator,
//! EMT/PDT model, protection study, harmonic assessment or interconnection
//! certification. Requirements are caller-supplied; no universal curve is
//! hardcoded. A disturbance-related load drop is never demand-response credit.

use core::fmt;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConnectionRequirement {
    MustRemainConnected,
    MomentaryCessationAllowed {
        maximum_disconnected_duration_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RideThroughSegment {
    start_elapsed_ns: u64,
    end_elapsed_ns: u64,
    minimum_voltage_pu: f64,
    maximum_voltage_pu: f64,
    minimum_frequency_hz: f64,
    maximum_frequency_hz: f64,
    maximum_absolute_rocof_hz_per_s: Option<f64>,
    connection_requirement: ConnectionRequirement,
}

impl RideThroughSegment {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start_elapsed_ns: u64,
        end_elapsed_ns: u64,
        minimum_voltage_pu: f64,
        maximum_voltage_pu: f64,
        minimum_frequency_hz: f64,
        maximum_frequency_hz: f64,
        maximum_absolute_rocof_hz_per_s: Option<f64>,
        connection_requirement: ConnectionRequirement,
    ) -> Result<Self, DisturbanceError> {
        if start_elapsed_ns >= end_elapsed_ns {
            return Err(DisturbanceError::InvalidRequirementWindow);
        }
        for (field, value) in [
            ("minimum voltage", minimum_voltage_pu),
            ("maximum voltage", maximum_voltage_pu),
            ("minimum frequency", minimum_frequency_hz),
            ("maximum frequency", maximum_frequency_hz),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(DisturbanceError::InvalidQuantity { field, value });
            }
        }
        if minimum_voltage_pu > maximum_voltage_pu || minimum_frequency_hz > maximum_frequency_hz {
            return Err(DisturbanceError::InvertedEnvelope);
        }
        if let Some(value) = maximum_absolute_rocof_hz_per_s
            && (!value.is_finite() || value <= 0.0)
        {
            return Err(DisturbanceError::InvalidQuantity {
                field: "maximum absolute RoCoF",
                value,
            });
        }
        if let ConnectionRequirement::MomentaryCessationAllowed {
            maximum_disconnected_duration_ns,
        } = connection_requirement
            && maximum_disconnected_duration_ns == 0
        {
            return Err(DisturbanceError::InvalidDisconnectionAllowance);
        }
        Ok(Self {
            start_elapsed_ns,
            end_elapsed_ns,
            minimum_voltage_pu,
            maximum_voltage_pu,
            minimum_frequency_hz,
            maximum_frequency_hz,
            maximum_absolute_rocof_hz_per_s,
            connection_requirement,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActivePowerRecoveryRequirement {
    minimum_recovered_voltage_pu: f64,
    minimum_recovered_frequency_hz: f64,
    maximum_recovered_frequency_hz: f64,
    stable_dwell_ns: u64,
    recovery_deadline_ns: u64,
    minimum_recovered_power_fraction: f64,
    maximum_overshoot_fraction: f64,
    maximum_absolute_recovery_ramp_mw_per_s: f64,
}

impl ActivePowerRecoveryRequirement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        minimum_recovered_voltage_pu: f64,
        minimum_recovered_frequency_hz: f64,
        maximum_recovered_frequency_hz: f64,
        stable_dwell_ns: u64,
        recovery_deadline_ns: u64,
        minimum_recovered_power_fraction: f64,
        maximum_overshoot_fraction: f64,
        maximum_absolute_recovery_ramp_mw_per_s: f64,
    ) -> Result<Self, DisturbanceError> {
        for (field, value) in [
            ("minimum recovered voltage", minimum_recovered_voltage_pu),
            (
                "minimum recovered frequency",
                minimum_recovered_frequency_hz,
            ),
            (
                "maximum recovered frequency",
                maximum_recovered_frequency_hz,
            ),
            (
                "minimum recovered power fraction",
                minimum_recovered_power_fraction,
            ),
            (
                "maximum recovery overshoot fraction",
                maximum_overshoot_fraction,
            ),
            (
                "maximum absolute recovery ramp",
                maximum_absolute_recovery_ramp_mw_per_s,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(DisturbanceError::InvalidQuantity { field, value });
            }
        }
        if minimum_recovered_voltage_pu <= 0.0
            || minimum_recovered_frequency_hz <= 0.0
            || maximum_recovered_frequency_hz < minimum_recovered_frequency_hz
            || !(0.0 < minimum_recovered_power_fraction && minimum_recovered_power_fraction <= 1.0)
            || maximum_absolute_recovery_ramp_mw_per_s <= 0.0
            || stable_dwell_ns == 0
            || recovery_deadline_ns == 0
        {
            return Err(DisturbanceError::InvalidRecoveryRequirement);
        }
        Ok(Self {
            minimum_recovered_voltage_pu,
            minimum_recovered_frequency_hz,
            maximum_recovered_frequency_hz,
            stable_dwell_ns,
            recovery_deadline_ns,
            minimum_recovered_power_fraction,
            maximum_overshoot_fraction,
            maximum_absolute_recovery_ramp_mw_per_s,
        })
    }
}

/// Operator-defined PCC reactive-power and current operating envelope.
/// Measurements are mandatory for every sample whenever this requirement is
/// attached to a requirement set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReactiveCurrentRequirement {
    minimum_reactive_power_mvar: f64,
    maximum_reactive_power_mvar: f64,
    maximum_current_pu: f64,
}

impl ReactiveCurrentRequirement {
    pub fn new(
        minimum_reactive_power_mvar: f64,
        maximum_reactive_power_mvar: f64,
        maximum_current_pu: f64,
    ) -> Result<Self, DisturbanceError> {
        for (field, value) in [
            ("minimum reactive power", minimum_reactive_power_mvar),
            ("maximum reactive power", maximum_reactive_power_mvar),
            ("maximum current", maximum_current_pu),
        ] {
            if !value.is_finite() {
                return Err(DisturbanceError::InvalidQuantity { field, value });
            }
        }
        if minimum_reactive_power_mvar > maximum_reactive_power_mvar || maximum_current_pu <= 0.0 {
            return Err(DisturbanceError::InvalidReactiveCurrentRequirement);
        }
        Ok(Self {
            minimum_reactive_power_mvar,
            maximum_reactive_power_mvar,
            maximum_current_pu,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisturbanceRequirementSet {
    requirement_id: String,
    requirement_version: String,
    jurisdiction: String,
    source_digest: String,
    pcc_id: String,
    nominal_frequency_hz: f64,
    required_observation_horizon_ns: u64,
    segments: Vec<RideThroughSegment>,
    recovery: ActivePowerRecoveryRequirement,
    reactive_current: Option<ReactiveCurrentRequirement>,
}

impl DisturbanceRequirementSet {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        requirement_id: impl Into<String>,
        requirement_version: impl Into<String>,
        jurisdiction: impl Into<String>,
        source_digest: impl Into<String>,
        pcc_id: impl Into<String>,
        nominal_frequency_hz: f64,
        required_observation_horizon_ns: u64,
        segments: Vec<RideThroughSegment>,
        recovery: ActivePowerRecoveryRequirement,
    ) -> Result<Self, DisturbanceError> {
        let set = Self {
            requirement_id: requirement_id.into(),
            requirement_version: requirement_version.into(),
            jurisdiction: jurisdiction.into(),
            source_digest: source_digest.into(),
            pcc_id: pcc_id.into(),
            nominal_frequency_hz,
            required_observation_horizon_ns,
            segments,
            recovery,
            reactive_current: None,
        };
        for (field, value) in [
            ("requirement ID", set.requirement_id.as_str()),
            ("requirement version", set.requirement_version.as_str()),
            ("jurisdiction", set.jurisdiction.as_str()),
            ("requirement source digest", set.source_digest.as_str()),
            ("PCC ID", set.pcc_id.as_str()),
        ] {
            require_id(field, value)?;
        }
        if !nominal_frequency_hz.is_finite() || nominal_frequency_hz <= 0.0 {
            return Err(DisturbanceError::InvalidQuantity {
                field: "nominal frequency",
                value: nominal_frequency_hz,
            });
        }
        if required_observation_horizon_ns == 0 || set.segments.is_empty() {
            return Err(DisturbanceError::EmptyRequirementEnvelope);
        }
        let mut expected_start_ns = 0;
        for (index, segment) in set.segments.iter().enumerate() {
            if segment.start_elapsed_ns != expected_start_ns {
                return Err(DisturbanceError::NonContiguousRequirementEnvelope {
                    index,
                    expected_start_ns,
                    observed_start_ns: segment.start_elapsed_ns,
                });
            }
            expected_start_ns = segment.end_elapsed_ns;
        }
        if expected_start_ns != required_observation_horizon_ns {
            return Err(DisturbanceError::RequirementHorizonMismatch);
        }
        Ok(set)
    }

    pub fn with_reactive_current_requirement(
        mut self,
        requirement: ReactiveCurrentRequirement,
    ) -> Self {
        self.reactive_current = Some(requirement);
        self
    }

    pub fn requirement_id(&self) -> &str {
        &self.requirement_id
    }
    pub fn requirement_version(&self) -> &str {
        &self.requirement_version
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    pub fn pcc_id(&self) -> &str {
        &self.pcc_id
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisturbanceTraceContract {
    maximum_clock_uncertainty_ns: u64,
    maximum_sample_gap_ns: u64,
    maximum_rms_window_ns: u64,
    minimum_measurement_bandwidth_hz: f64,
    minimum_sample_count: usize,
}

impl DisturbanceTraceContract {
    pub fn new(
        maximum_clock_uncertainty_ns: u64,
        maximum_sample_gap_ns: u64,
        maximum_rms_window_ns: u64,
        minimum_measurement_bandwidth_hz: f64,
        minimum_sample_count: usize,
    ) -> Result<Self, DisturbanceError> {
        if maximum_sample_gap_ns == 0 || maximum_rms_window_ns == 0 || minimum_sample_count < 2 {
            return Err(DisturbanceError::InvalidTraceContract);
        }
        if !minimum_measurement_bandwidth_hz.is_finite() || minimum_measurement_bandwidth_hz <= 0.0
        {
            return Err(DisturbanceError::InvalidQuantity {
                field: "minimum measurement bandwidth",
                value: minimum_measurement_bandwidth_hz,
            });
        }
        // The declared sample cadence must satisfy Nyquist for the required
        // observable bandwidth. Do not claim dynamics the trace cannot resolve.
        let nyquist_limit_hz = 500_000_000.0 / maximum_sample_gap_ns as f64;
        if minimum_measurement_bandwidth_hz > nyquist_limit_hz {
            return Err(DisturbanceError::SamplingCadenceCannotResolveBandwidth {
                maximum_sample_gap_ns,
                required_bandwidth_hz: minimum_measurement_bandwidth_hz,
                nyquist_limit_hz,
            });
        }
        Ok(Self {
            maximum_clock_uncertainty_ns,
            maximum_sample_gap_ns,
            maximum_rms_window_ns,
            minimum_measurement_bandwidth_hz,
            minimum_sample_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisturbanceEventIdentity {
    event_id: String,
    requirement_id: String,
    requirement_version: String,
    requirement_source_digest: String,
    pcc_id: String,
    waveform_digest: String,
    measurement_chain_digest: String,
    clock_model_id: String,
    pre_event_reference_digest: String,
    event_start_ns: u64,
    pre_event_active_power_mw: f64,
    clock_uncertainty_ns: u64,
    rms_window_ns: u64,
    measurement_bandwidth_hz: f64,
}

impl DisturbanceEventIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: impl Into<String>,
        requirement_id: impl Into<String>,
        requirement_version: impl Into<String>,
        requirement_source_digest: impl Into<String>,
        pcc_id: impl Into<String>,
        waveform_digest: impl Into<String>,
        measurement_chain_digest: impl Into<String>,
        clock_model_id: impl Into<String>,
        pre_event_reference_digest: impl Into<String>,
        event_start_ns: u64,
        pre_event_active_power_mw: f64,
        clock_uncertainty_ns: u64,
        rms_window_ns: u64,
        measurement_bandwidth_hz: f64,
    ) -> Result<Self, DisturbanceError> {
        let event = Self {
            event_id: event_id.into(),
            requirement_id: requirement_id.into(),
            requirement_version: requirement_version.into(),
            requirement_source_digest: requirement_source_digest.into(),
            pcc_id: pcc_id.into(),
            waveform_digest: waveform_digest.into(),
            measurement_chain_digest: measurement_chain_digest.into(),
            clock_model_id: clock_model_id.into(),
            pre_event_reference_digest: pre_event_reference_digest.into(),
            event_start_ns,
            pre_event_active_power_mw,
            clock_uncertainty_ns,
            rms_window_ns,
            measurement_bandwidth_hz,
        };
        for (field, value) in [
            ("event ID", event.event_id.as_str()),
            ("requirement ID", event.requirement_id.as_str()),
            ("requirement version", event.requirement_version.as_str()),
            (
                "requirement source digest",
                event.requirement_source_digest.as_str(),
            ),
            ("PCC ID", event.pcc_id.as_str()),
            ("waveform digest", event.waveform_digest.as_str()),
            (
                "measurement-chain digest",
                event.measurement_chain_digest.as_str(),
            ),
            ("clock model ID", event.clock_model_id.as_str()),
            (
                "pre-event reference digest",
                event.pre_event_reference_digest.as_str(),
            ),
        ] {
            require_id(field, value)?;
        }
        if !pre_event_active_power_mw.is_finite() || pre_event_active_power_mw <= 0.0 {
            return Err(DisturbanceError::InvalidQuantity {
                field: "pre-event active power reference",
                value: pre_event_active_power_mw,
            });
        }
        if rms_window_ns == 0
            || !measurement_bandwidth_hz.is_finite()
            || measurement_bandwidth_hz <= 0.0
        {
            return Err(DisturbanceError::InvalidTraceContract);
        }
        Ok(event)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PccDisturbanceSample {
    pub timestamp_ns: u64,
    pub voltage_pu: f64,
    pub frequency_hz: f64,
    pub active_power_mw: f64,
    pub reactive_power_mvar: Option<f64>,
    pub current_pu: Option<f64>,
    pub grid_connected: bool,
}

impl PccDisturbanceSample {
    pub fn new(
        timestamp_ns: u64,
        voltage_pu: f64,
        frequency_hz: f64,
        active_power_mw: f64,
        reactive_power_mvar: Option<f64>,
        current_pu: Option<f64>,
        grid_connected: bool,
    ) -> Result<Self, DisturbanceError> {
        for (field, value) in [
            ("voltage", voltage_pu),
            ("frequency", frequency_hz),
            ("active power", active_power_mw),
        ] {
            if !value.is_finite() || (field != "active power" && value <= 0.0) {
                return Err(DisturbanceError::InvalidQuantity { field, value });
            }
        }
        if let Some(value) = reactive_power_mvar
            && !value.is_finite()
        {
            return Err(DisturbanceError::InvalidQuantity {
                field: "reactive power",
                value,
            });
        }
        if let Some(value) = current_pu
            && (!value.is_finite() || value < 0.0)
        {
            return Err(DisturbanceError::InvalidQuantity {
                field: "current",
                value,
            });
        }
        Ok(Self {
            timestamp_ns,
            voltage_pu,
            frequency_hz,
            active_power_mw,
            reactive_power_mvar,
            current_pu,
            grid_connected,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RideThroughZoneReceipt {
    pub start_elapsed_ns: u64,
    pub end_elapsed_ns: u64,
    pub sample_count: usize,
    pub voltage_frequency_within_envelope: bool,
    pub rocof_within_envelope: bool,
    pub connection_requirement_met: bool,
    pub maximum_absolute_rocof_hz_per_s: Option<f64>,
    pub disconnected_duration_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisturbanceConformanceReport {
    pub event_id: String,
    pub requirement_id: String,
    pub requirement_version: String,
    pub jurisdiction: String,
    pub requirement_source_digest: String,
    pub nominal_frequency_hz: f64,
    pub pcc_id: String,
    pub waveform_digest: String,
    pub measurement_chain_digest: String,
    pub clock_model_id: String,
    pub sample_count: usize,
    pub observed_horizon_end_ns: u64,
    pub minimum_voltage_pu: f64,
    pub maximum_voltage_pu: f64,
    pub minimum_frequency_hz: f64,
    pub maximum_frequency_hz: f64,
    pub maximum_absolute_rocof_hz_per_s: f64,
    pub reactive_current_assessed: bool,
    pub minimum_reactive_power_mvar: Option<f64>,
    pub maximum_reactive_power_mvar: Option<f64>,
    pub maximum_current_pu: Option<f64>,
    pub reactive_current_compliant: bool,
    pub total_disconnected_duration_ns: u64,
    pub unexpected_trip_detected: bool,
    pub ride_through_compliant: bool,
    pub connection_continuity_compliant: bool,
    pub recovery_condition_observed: bool,
    pub recovery_deadline_ns: Option<u64>,
    pub recovery_achieved_at_ns: Option<u64>,
    pub recovery_power_fraction_at_deadline: Option<f64>,
    pub maximum_absolute_recovery_ramp_mw_per_s: Option<f64>,
    pub observed_recovery_overshoot_fraction: Option<f64>,
    pub recovery_compliant: bool,
    pub qualified: bool,
    pub intentional_demand_response_credit: bool,
    pub zones: Vec<RideThroughZoneReceipt>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisturbanceError {
    MissingIdentity {
        field: &'static str,
    },
    InvalidQuantity {
        field: &'static str,
        value: f64,
    },
    InvalidRequirementWindow,
    InvertedEnvelope,
    InvalidDisconnectionAllowance,
    InvalidRecoveryRequirement,
    InvalidReactiveCurrentRequirement,
    MissingReactivePowerSample {
        timestamp_ns: u64,
    },
    MissingCurrentSample {
        timestamp_ns: u64,
    },
    EmptyRequirementEnvelope,
    NonContiguousRequirementEnvelope {
        index: usize,
        expected_start_ns: u64,
        observed_start_ns: u64,
    },
    RequirementHorizonMismatch,
    InvalidTraceContract,
    SamplingCadenceCannotResolveBandwidth {
        maximum_sample_gap_ns: u64,
        required_bandwidth_hz: f64,
        nyquist_limit_hz: f64,
    },
    RequirementIdentityMismatch,
    PccIdentityMismatch,
    ClockUncertaintyExceedsContract {
        observed_ns: u64,
        maximum_ns: u64,
    },
    RmsWindowExceedsContract {
        observed_ns: u64,
        maximum_ns: u64,
    },
    MeasurementBandwidthBelowContract {
        observed_hz: f64,
        minimum_hz: f64,
    },
    EmptyTrace,
    InsufficientSamples {
        observed: usize,
        minimum: usize,
    },
    SampleBeforeEventStart,
    InitialSampleGapExceedsContract {
        gap_ns: u64,
        maximum_gap_ns: u64,
    },
    DuplicateOrReversedTimestamp {
        index: usize,
        previous_ns: u64,
        observed_ns: u64,
    },
    SampleGapExceedsContract {
        index: usize,
        gap_ns: u64,
        maximum_gap_ns: u64,
    },
    TimestampOverflow,
    SampleOutsideRequirementHorizon,
    NoSamplesInRequirementZone {
        index: usize,
    },
    ObservationHorizonIncomplete {
        required_end_ns: u64,
        observed_end_ns: u64,
    },
    RecoveryDeadlineOverflow,
    NumericalOverflow(&'static str),
}

impl fmt::Display for DisturbanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => {
                write!(f, "{field} must be finite and in range; got {value}")
            }
            Self::InvalidRequirementWindow => {
                write!(f, "ride-through requirement window is empty or reversed")
            }
            Self::InvertedEnvelope => {
                write!(f, "operator voltage or frequency envelope is inverted")
            }
            Self::InvalidDisconnectionAllowance => {
                write!(f, "momentary cessation allowance must be positive")
            }
            Self::InvalidRecoveryRequirement => {
                write!(f, "active-power recovery requirement is invalid")
            }
            Self::InvalidReactiveCurrentRequirement => {
                write!(f, "reactive-power/current requirement envelope is invalid")
            }
            Self::MissingReactivePowerSample { timestamp_ns } => write!(
                f,
                "reactive-power measurement is missing at {timestamp_ns} while the operator requires it"
            ),
            Self::MissingCurrentSample { timestamp_ns } => write!(
                f,
                "current measurement is missing at {timestamp_ns} while the operator requires it"
            ),
            Self::EmptyRequirementEnvelope => {
                write!(f, "operator requirement envelope must not be empty")
            }
            Self::NonContiguousRequirementEnvelope {
                index,
                expected_start_ns,
                observed_start_ns,
            } => write!(
                f,
                "requirement segment {index} starts at {observed_start_ns}; expected {expected_start_ns}"
            ),
            Self::RequirementHorizonMismatch => write!(
                f,
                "requirement segments do not cover the declared observation horizon"
            ),
            Self::InvalidTraceContract => write!(
                f,
                "trace sampling, RMS window, bandwidth or sample-count contract is invalid"
            ),
            Self::RequirementIdentityMismatch => write!(
                f,
                "event requirement identity differs from the supplied pinned requirement set"
            ),
            Self::PccIdentityMismatch => write!(
                f,
                "event PCC differs from the requirement point of applicability"
            ),
            Self::ClockUncertaintyExceedsContract {
                observed_ns,
                maximum_ns,
            } => write!(
                f,
                "clock uncertainty {observed_ns} ns exceeds {maximum_ns} ns"
            ),
            Self::RmsWindowExceedsContract {
                observed_ns,
                maximum_ns,
            } => write!(f, "RMS window {observed_ns} ns exceeds {maximum_ns} ns"),
            Self::MeasurementBandwidthBelowContract {
                observed_hz,
                minimum_hz,
            } => write!(
                f,
                "measurement bandwidth {observed_hz} Hz is below {minimum_hz} Hz"
            ),
            Self::EmptyTrace => write!(f, "disturbance waveform contains no samples"),
            Self::InsufficientSamples { observed, minimum } => write!(
                f,
                "waveform contains {observed} samples; requires {minimum}"
            ),
            Self::SampleBeforeEventStart => {
                write!(f, "waveform sample precedes the pinned event start")
            }
            Self::DuplicateOrReversedTimestamp {
                index,
                previous_ns,
                observed_ns,
            } => write!(
                f,
                "sample {index} timestamp {observed_ns} does not follow {previous_ns}"
            ),
            Self::SampleGapExceedsContract {
                index,
                gap_ns,
                maximum_gap_ns,
            } => write!(
                f,
                "sample gap {gap_ns} ns at {index} exceeds {maximum_gap_ns} ns"
            ),
            Self::TimestampOverflow => write!(f, "event timestamp arithmetic overflow"),
            Self::SampleOutsideRequirementHorizon => write!(
                f,
                "waveform sample lies outside the pinned requirement horizon"
            ),
            Self::NoSamplesInRequirementZone { index } => {
                write!(f, "requirement zone {index} has no waveform samples")
            }
            Self::ObservationHorizonIncomplete {
                required_end_ns,
                observed_end_ns,
            } => write!(
                f,
                "trace ends at {observed_end_ns}; required observation ends at {required_end_ns}"
            ),
            Self::RecoveryDeadlineOverflow => write!(f, "recovery deadline timestamp overflow"),
            Self::NumericalOverflow(field) => write!(f, "numerical overflow computing {field}"),
        }
    }
}
impl std::error::Error for DisturbanceError {}

pub fn evaluate_disturbance_conformance(
    event: &DisturbanceEventIdentity,
    requirement: &DisturbanceRequirementSet,
    trace_contract: &DisturbanceTraceContract,
    samples: &[PccDisturbanceSample],
) -> Result<DisturbanceConformanceReport, DisturbanceError> {
    if event.requirement_id != requirement.requirement_id
        || event.requirement_version != requirement.requirement_version
        || event.requirement_source_digest != requirement.source_digest
    {
        return Err(DisturbanceError::RequirementIdentityMismatch);
    }
    if event.pcc_id != requirement.pcc_id {
        return Err(DisturbanceError::PccIdentityMismatch);
    }
    if event.clock_uncertainty_ns > trace_contract.maximum_clock_uncertainty_ns {
        return Err(DisturbanceError::ClockUncertaintyExceedsContract {
            observed_ns: event.clock_uncertainty_ns,
            maximum_ns: trace_contract.maximum_clock_uncertainty_ns,
        });
    }
    if event.rms_window_ns > trace_contract.maximum_rms_window_ns {
        return Err(DisturbanceError::RmsWindowExceedsContract {
            observed_ns: event.rms_window_ns,
            maximum_ns: trace_contract.maximum_rms_window_ns,
        });
    }
    if event.measurement_bandwidth_hz < trace_contract.minimum_measurement_bandwidth_hz {
        return Err(DisturbanceError::MeasurementBandwidthBelowContract {
            observed_hz: event.measurement_bandwidth_hz,
            minimum_hz: trace_contract.minimum_measurement_bandwidth_hz,
        });
    }
    if samples.is_empty() {
        return Err(DisturbanceError::EmptyTrace);
    }
    if samples.len() < trace_contract.minimum_sample_count {
        return Err(DisturbanceError::InsufficientSamples {
            observed: samples.len(),
            minimum: trace_contract.minimum_sample_count,
        });
    }
    let required_end_ns = event
        .event_start_ns
        .checked_add(requirement.required_observation_horizon_ns)
        .ok_or(DisturbanceError::TimestampOverflow)?;
    let initial_gap_ns = samples[0]
        .timestamp_ns
        .checked_sub(event.event_start_ns)
        .ok_or(DisturbanceError::SampleBeforeEventStart)?;
    if initial_gap_ns > trace_contract.maximum_sample_gap_ns {
        return Err(DisturbanceError::InitialSampleGapExceedsContract {
            gap_ns: initial_gap_ns,
            maximum_gap_ns: trace_contract.maximum_sample_gap_ns,
        });
    }

    let n = requirement.segments.len();
    let mut zone_voltage_frequency = vec![true; n];
    let mut zone_rocof = vec![true; n];
    let mut zone_connection = vec![true; n];
    let mut zone_samples = vec![0usize; n];
    let mut zone_disconnected = vec![0u64; n];
    let mut zone_max_rocof = vec![0.0f64; n];
    let mut min_voltage = f64::INFINITY;
    let mut max_voltage = f64::NEG_INFINITY;
    let mut min_frequency = f64::INFINITY;
    let mut max_frequency = f64::NEG_INFINITY;
    let mut maximum_rocof = 0.0f64;
    let mut minimum_reactive_power = f64::INFINITY;
    let mut maximum_reactive_power = f64::NEG_INFINITY;
    let mut maximum_current = 0.0f64;
    let mut reactive_current_compliant = true;
    let mut total_disconnected = 0u64;
    let mut unexpected_trip = false;

    for (index, sample) in samples.iter().enumerate() {
        if sample.timestamp_ns < event.event_start_ns {
            return Err(DisturbanceError::SampleBeforeEventStart);
        }
        let elapsed = sample.timestamp_ns - event.event_start_ns;
        if elapsed >= requirement.required_observation_horizon_ns {
            return Err(DisturbanceError::SampleOutsideRequirementHorizon);
        }
        let zone_index = find_zone(&requirement.segments, elapsed)
            .ok_or(DisturbanceError::NoSamplesInRequirementZone { index })?;
        zone_samples[zone_index] += 1;
        let zone = &requirement.segments[zone_index];

        if let Some(reactive_requirement) = &requirement.reactive_current {
            let reactive_power =
                sample
                    .reactive_power_mvar
                    .ok_or(DisturbanceError::MissingReactivePowerSample {
                        timestamp_ns: sample.timestamp_ns,
                    })?;
            let current = sample
                .current_pu
                .ok_or(DisturbanceError::MissingCurrentSample {
                    timestamp_ns: sample.timestamp_ns,
                })?;
            minimum_reactive_power = minimum_reactive_power.min(reactive_power);
            maximum_reactive_power = maximum_reactive_power.max(reactive_power);
            maximum_current = maximum_current.max(current);
            if reactive_power < reactive_requirement.minimum_reactive_power_mvar
                || reactive_power > reactive_requirement.maximum_reactive_power_mvar
                || current > reactive_requirement.maximum_current_pu
            {
                reactive_current_compliant = false;
            }
        }

        if sample.voltage_pu < zone.minimum_voltage_pu
            || sample.voltage_pu > zone.maximum_voltage_pu
            || sample.frequency_hz < zone.minimum_frequency_hz
            || sample.frequency_hz > zone.maximum_frequency_hz
        {
            zone_voltage_frequency[zone_index] = false;
        }
        min_voltage = min_voltage.min(sample.voltage_pu);
        max_voltage = max_voltage.max(sample.voltage_pu);
        min_frequency = min_frequency.min(sample.frequency_hz);
        max_frequency = max_frequency.max(sample.frequency_hz);

        if index > 0 {
            let previous = &samples[index - 1];
            if sample.timestamp_ns <= previous.timestamp_ns {
                return Err(DisturbanceError::DuplicateOrReversedTimestamp {
                    index,
                    previous_ns: previous.timestamp_ns,
                    observed_ns: sample.timestamp_ns,
                });
            }
            let gap = sample.timestamp_ns - previous.timestamp_ns;
            if gap > trace_contract.maximum_sample_gap_ns {
                return Err(DisturbanceError::SampleGapExceedsContract {
                    index,
                    gap_ns: gap,
                    maximum_gap_ns: trace_contract.maximum_sample_gap_ns,
                });
            }
            let dt = gap as f64 / 1_000_000_000.0;
            let rocof = ((sample.frequency_hz - previous.frequency_hz) / dt).abs();
            if !rocof.is_finite() {
                return Err(DisturbanceError::NumericalOverflow("RoCoF"));
            }
            maximum_rocof = maximum_rocof.max(rocof);
            zone_max_rocof[zone_index] = zone_max_rocof[zone_index].max(rocof);
            let previous_elapsed = previous.timestamp_ns - event.event_start_ns;
            let previous_zone_index = find_zone(&requirement.segments, previous_elapsed)
                .ok_or(DisturbanceError::NoSamplesInRequirementZone { index: index - 1 })?;
            let limit = match (
                requirement.segments[zone_index].maximum_absolute_rocof_hz_per_s,
                requirement.segments[previous_zone_index].maximum_absolute_rocof_hz_per_s,
            ) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            };
            if limit.is_some_and(|maximum| rocof > maximum) {
                zone_rocof[zone_index] = false;
                zone_rocof[previous_zone_index] = false;
            }
            // Treat the prior connection state as held over the sample interval.
            if !previous.grid_connected {
                let interval_start = previous_elapsed;
                let interval_end = elapsed;
                for (zi, segment) in requirement.segments.iter().enumerate() {
                    let overlap_start = interval_start.max(segment.start_elapsed_ns);
                    let overlap_end = interval_end.min(segment.end_elapsed_ns);
                    if overlap_end > overlap_start {
                        let duration = overlap_end - overlap_start;
                        zone_disconnected[zi] = zone_disconnected[zi].saturating_add(duration);
                        total_disconnected = total_disconnected.saturating_add(duration);
                    }
                }
            }
        }
        if !sample.grid_connected {
            match zone.connection_requirement {
                ConnectionRequirement::MustRemainConnected => {
                    zone_connection[zone_index] = false;
                    unexpected_trip = true;
                }
                ConnectionRequirement::MomentaryCessationAllowed { .. } => {}
            }
        }
    }

    let observed_end_ns = samples
        .last()
        .ok_or(DisturbanceError::EmptyTrace)?
        .timestamp_ns;
    if observed_end_ns.saturating_add(trace_contract.maximum_sample_gap_ns) < required_end_ns {
        return Err(DisturbanceError::ObservationHorizonIncomplete {
            required_end_ns,
            observed_end_ns,
        });
    }

    // Hold the final observed connection state through the declared horizon.
    if let Some(last) = samples.last()
        && !last.grid_connected
    {
        let elapsed = last.timestamp_ns - event.event_start_ns;
        for (zi, segment) in requirement.segments.iter().enumerate() {
            let overlap_start = elapsed.max(segment.start_elapsed_ns);
            let overlap_end = requirement
                .required_observation_horizon_ns
                .min(segment.end_elapsed_ns);
            if overlap_end > overlap_start {
                let duration = overlap_end - overlap_start;
                zone_disconnected[zi] = zone_disconnected[zi].saturating_add(duration);
                total_disconnected = total_disconnected.saturating_add(duration);
            }
        }
    }

    for (index, segment) in requirement.segments.iter().enumerate() {
        if zone_samples[index] == 0 {
            return Err(DisturbanceError::NoSamplesInRequirementZone { index });
        }
        if let ConnectionRequirement::MustRemainConnected = segment.connection_requirement {
            if zone_disconnected[index] > 0 {
                zone_connection[index] = false;
                unexpected_trip = true;
            }
        }
        if let ConnectionRequirement::MomentaryCessationAllowed {
            maximum_disconnected_duration_ns,
        } = segment.connection_requirement
            && zone_disconnected[index] > maximum_disconnected_duration_ns
        {
            zone_connection[index] = false;
            unexpected_trip = true;
        }
    }

    let (
        recovery_condition_observed,
        recovery_deadline_ns,
        recovery_achieved_at_ns,
        recovery_power_fraction_at_deadline,
        maximum_recovery_ramp,
        overshoot,
        recovery_compliant,
    ) = evaluate_recovery(event, &requirement.recovery, samples)?;

    let zones: Vec<RideThroughZoneReceipt> = requirement
        .segments
        .iter()
        .enumerate()
        .map(|(i, segment)| RideThroughZoneReceipt {
            start_elapsed_ns: segment.start_elapsed_ns,
            end_elapsed_ns: segment.end_elapsed_ns,
            sample_count: zone_samples[i],
            voltage_frequency_within_envelope: zone_voltage_frequency[i],
            rocof_within_envelope: zone_rocof[i],
            connection_requirement_met: zone_connection[i],
            maximum_absolute_rocof_hz_per_s: (zone_samples[i] > 1).then_some(zone_max_rocof[i]),
            disconnected_duration_ns: zone_disconnected[i],
        })
        .collect();
    let ride_through_compliant = zones
        .iter()
        .all(|z| z.voltage_frequency_within_envelope && z.rocof_within_envelope);
    let connection_continuity_compliant = zones.iter().all(|z| z.connection_requirement_met);
    let reactive_current_assessed = requirement.reactive_current.is_some();
    let qualified = ride_through_compliant
        && connection_continuity_compliant
        && recovery_compliant
        && reactive_current_compliant;

    Ok(DisturbanceConformanceReport {
        event_id: event.event_id.clone(),
        requirement_id: event.requirement_id.clone(),
        requirement_version: event.requirement_version.clone(),
        jurisdiction: requirement.jurisdiction.clone(),
        requirement_source_digest: event.requirement_source_digest.clone(),
        nominal_frequency_hz: requirement.nominal_frequency_hz,
        pcc_id: event.pcc_id.clone(),
        waveform_digest: event.waveform_digest.clone(),
        measurement_chain_digest: event.measurement_chain_digest.clone(),
        clock_model_id: event.clock_model_id.clone(),
        sample_count: samples.len(),
        observed_horizon_end_ns: observed_end_ns,
        minimum_voltage_pu: min_voltage,
        maximum_voltage_pu: max_voltage,
        minimum_frequency_hz: min_frequency,
        maximum_frequency_hz: max_frequency,
        maximum_absolute_rocof_hz_per_s: maximum_rocof,
        reactive_current_assessed,
        minimum_reactive_power_mvar: reactive_current_assessed.then_some(minimum_reactive_power),
        maximum_reactive_power_mvar: reactive_current_assessed.then_some(maximum_reactive_power),
        maximum_current_pu: reactive_current_assessed.then_some(maximum_current),
        reactive_current_compliant,
        total_disconnected_duration_ns: total_disconnected,
        unexpected_trip_detected: unexpected_trip,
        ride_through_compliant,
        connection_continuity_compliant,
        recovery_condition_observed,
        recovery_deadline_ns,
        recovery_achieved_at_ns,
        recovery_power_fraction_at_deadline,
        maximum_absolute_recovery_ramp_mw_per_s: maximum_recovery_ramp,
        observed_recovery_overshoot_fraction: overshoot,
        recovery_compliant,
        qualified,
        intentional_demand_response_credit: false,
        zones,
        claim_boundary: "PCC trace conformance to a pinned operator envelope only; not internal converter stability, AC network stability, EMT/PDT, protection, harmonics, grid-code certification, market accreditation or ELCC".to_owned(),
    })
}

fn evaluate_recovery(
    event: &DisturbanceEventIdentity,
    requirement: &ActivePowerRecoveryRequirement,
    samples: &[PccDisturbanceSample],
) -> Result<
    (
        bool,
        Option<u64>,
        Option<u64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        bool,
    ),
    DisturbanceError,
> {
    let mut stable_index = None;
    for (i, first) in samples.iter().enumerate() {
        if first.voltage_pu < requirement.minimum_recovered_voltage_pu
            || first.frequency_hz < requirement.minimum_recovered_frequency_hz
            || first.frequency_hz > requirement.maximum_recovered_frequency_hz
        {
            continue;
        }
        let stable_end = first
            .timestamp_ns
            .checked_add(requirement.stable_dwell_ns)
            .ok_or(DisturbanceError::RecoveryDeadlineOverflow)?;
        let mut last_stable = first.timestamp_ns;
        let mut valid = true;
        for sample in samples.iter().skip(i + 1) {
            if sample.voltage_pu < requirement.minimum_recovered_voltage_pu
                || sample.frequency_hz < requirement.minimum_recovered_frequency_hz
                || sample.frequency_hz > requirement.maximum_recovered_frequency_hz
            {
                valid = false;
                break;
            }
            last_stable = sample.timestamp_ns;
            if sample.timestamp_ns >= stable_end {
                break;
            }
        }
        if valid && last_stable >= stable_end {
            stable_index = Some(i);
            break;
        }
    }
    let Some(index) = stable_index else {
        return Ok((false, None, None, None, None, None, false));
    };
    let recovery_trigger_ns = samples[index]
        .timestamp_ns
        .checked_add(requirement.stable_dwell_ns)
        .ok_or(DisturbanceError::RecoveryDeadlineOverflow)?;
    let deadline_ns = recovery_trigger_ns
        .checked_add(requirement.recovery_deadline_ns)
        .ok_or(DisturbanceError::RecoveryDeadlineOverflow)?;
    let observed_end = samples.last().map(|s| s.timestamp_ns).unwrap_or(0);
    if observed_end < deadline_ns {
        return Err(DisturbanceError::ObservationHorizonIncomplete {
            required_end_ns: deadline_ns,
            observed_end_ns: observed_end,
        });
    }
    let target_power =
        event.pre_event_active_power_mw * requirement.minimum_recovered_power_fraction;
    let achieved_at = samples
        .iter()
        .filter(|s| s.timestamp_ns >= recovery_trigger_ns && s.timestamp_ns <= deadline_ns)
        .find(|s| s.active_power_mw >= target_power)
        .map(|s| s.timestamp_ns);
    let achieved = achieved_at.is_some();

    let mut maximum_ramp = 0.0f64;
    let mut maximum_power = event.pre_event_active_power_mw;
    for pair in samples
        .windows(2)
        .filter(|p| p[1].timestamp_ns >= recovery_trigger_ns)
    {
        let dt = (pair[1].timestamp_ns - pair[0].timestamp_ns) as f64 / 1e9;
        let ramp = (pair[1].active_power_mw - pair[0].active_power_mw).abs() / dt;
        if !ramp.is_finite() {
            return Err(DisturbanceError::NumericalOverflow(
                "active-power recovery ramp",
            ));
        }
        maximum_ramp = maximum_ramp.max(ramp);
        maximum_power = maximum_power.max(pair[1].active_power_mw);
    }
    let overshoot = (maximum_power - event.pre_event_active_power_mw).max(0.0)
        / event.pre_event_active_power_mw;
    let power_fraction_at_deadline = samples
        .iter()
        .take_while(|s| s.timestamp_ns <= deadline_ns)
        .last()
        .map(|s| s.active_power_mw / event.pre_event_active_power_mw);
    let compliant = achieved
        && maximum_ramp <= requirement.maximum_absolute_recovery_ramp_mw_per_s
        && overshoot <= requirement.maximum_overshoot_fraction;
    Ok((
        true,
        Some(deadline_ns),
        achieved_at,
        power_fraction_at_deadline,
        Some(maximum_ramp),
        Some(overshoot),
        compliant,
    ))
}

fn find_zone(segments: &[RideThroughSegment], elapsed_ns: u64) -> Option<usize> {
    segments
        .iter()
        .position(|s| elapsed_ns >= s.start_elapsed_ns && elapsed_ns < s.end_elapsed_ns)
}

fn require_id(field: &'static str, value: &str) -> Result<(), DisturbanceError> {
    if value.trim().is_empty() {
        Err(DisturbanceError::MissingIdentity { field })
    } else {
        Ok(())
    }
}
