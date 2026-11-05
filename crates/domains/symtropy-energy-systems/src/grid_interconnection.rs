// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! PCC ride-through and post-disturbance recovery assurance.
//!
//! This module verifies a caller-supplied, site-/operator-specific disturbance
//! envelope from time-stamped PCC voltage, frequency, active/reactive power,
//! breaker state and measurement-quality samples. It does not embed universal
//! grid-code limits, simulate electromagnetic transients, operate protection,
//! or control equipment. Limits must come from the applicable interconnection
//! agreement / operator profile and the relevant jurisdiction.
//!
//! Ramp and oscillation observations are bounded by telemetry resolution;
//! sub-sample transients remain explicitly unverified.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

const MAX_ID_BYTES: usize = 256;
const MAX_RIDE_THROUGH_BANDS: usize = 128;

/// Identity of one frozen grid-disturbance qualification event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GridInterconnectionIdentity {
    event_id: String,
    site_id: String,
    pcc_boundary_id: String,
    interconnection_agreement_id: String,
    envelope_version: String,
    measurement_configuration_id: String,
}

impl GridInterconnectionIdentity {
    pub fn new(
        event_id: impl Into<String>,
        site_id: impl Into<String>,
        pcc_boundary_id: impl Into<String>,
        interconnection_agreement_id: impl Into<String>,
        envelope_version: impl Into<String>,
        measurement_configuration_id: impl Into<String>,
    ) -> Result<Self, GridInterconnectionError> {
        let identity = Self {
            event_id: event_id.into(),
            site_id: site_id.into(),
            pcc_boundary_id: pcc_boundary_id.into(),
            interconnection_agreement_id: interconnection_agreement_id.into(),
            envelope_version: envelope_version.into(),
            measurement_configuration_id: measurement_configuration_id.into(),
        };
        for (name, value) in [
            ("event ID", identity.event_id.as_str()),
            ("site ID", identity.site_id.as_str()),
            ("PCC boundary ID", identity.pcc_boundary_id.as_str()),
            ("interconnection agreement ID", identity.interconnection_agreement_id.as_str()),
            ("envelope version", identity.envelope_version.as_str()),
            ("measurement configuration ID", identity.measurement_configuration_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        Ok(identity)
    }

    pub fn event_id(&self) -> &str { &self.event_id }
    pub fn site_id(&self) -> &str { &self.site_id }
    pub fn pcc_boundary_id(&self) -> &str { &self.pcc_boundary_id }
    pub fn interconnection_agreement_id(&self) -> &str { &self.interconnection_agreement_id }
    pub fn envelope_version(&self) -> &str { &self.envelope_version }
    pub fn measurement_configuration_id(&self) -> &str { &self.measurement_configuration_id }
}

/// Piecewise voltage/frequency ride-through band, relative to disturbance start.
/// Bands must be sorted, contiguous, begin at zero, and use half-open intervals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GridRideThroughBand {
    start_offset_milliseconds: u64,
    end_offset_milliseconds: u64,
    minimum_voltage_pu: f64,
    maximum_voltage_pu: f64,
    minimum_frequency_hz: f64,
    maximum_frequency_hz: f64,
}

impl GridRideThroughBand {
    pub fn new(
        start_offset_milliseconds: u64,
        end_offset_milliseconds: u64,
        minimum_voltage_pu: f64,
        maximum_voltage_pu: f64,
        minimum_frequency_hz: f64,
        maximum_frequency_hz: f64,
    ) -> Result<Self, GridInterconnectionError> {
        if !minimum_voltage_pu.is_finite() || minimum_voltage_pu < 0.0 {
            return Err(GridInterconnectionError::InvalidNonNegativeQuantity {
                name: "minimum voltage pu",
                value: minimum_voltage_pu,
            });
        }
        for (name, value) in [
            ("maximum voltage pu", maximum_voltage_pu),
            ("minimum frequency Hz", minimum_frequency_hz),
            ("maximum frequency Hz", maximum_frequency_hz),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(GridInterconnectionError::InvalidPositiveQuantity { name, value });
            }
        }
        if start_offset_milliseconds >= end_offset_milliseconds
            || minimum_voltage_pu > maximum_voltage_pu
            || minimum_frequency_hz > maximum_frequency_hz
        {
            return Err(GridInterconnectionError::InvalidRideThroughBand);
        }
        Ok(Self {
            start_offset_milliseconds,
            end_offset_milliseconds,
            minimum_voltage_pu,
            maximum_voltage_pu,
            minimum_frequency_hz,
            maximum_frequency_hz,
        })
    }

    pub const fn start_offset_milliseconds(&self) -> u64 { self.start_offset_milliseconds }
    pub const fn end_offset_milliseconds(&self) -> u64 { self.end_offset_milliseconds }
}

/// Site-/operator-specific recovery and measurement contract.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridInterconnectionContract {
    identity: GridInterconnectionIdentity,
    ride_through_bands: Vec<GridRideThroughBand>,
    baseline_active_power_kw: f64,
    baseline_reactive_power_kvar: f64,
    active_power_recovery_tolerance_kw: f64,
    reactive_power_recovery_tolerance_kvar: f64,
    maximum_active_power_ramp_kw_per_second: f64,
    maximum_reactive_power_ramp_kvar_per_second: f64,
    maximum_recovery_duration_milliseconds: u64,
    settling_window_milliseconds: u64,
    maximum_sample_gap_milliseconds: u64,
    maximum_measurement_uncertainty_milliseconds: u64,
}

impl GridInterconnectionContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: GridInterconnectionIdentity,
        ride_through_bands: Vec<GridRideThroughBand>,
        baseline_active_power_kw: f64,
        baseline_reactive_power_kvar: f64,
        active_power_recovery_tolerance_kw: f64,
        reactive_power_recovery_tolerance_kvar: f64,
        maximum_active_power_ramp_kw_per_second: f64,
        maximum_reactive_power_ramp_kvar_per_second: f64,
        maximum_recovery_duration_milliseconds: u64,
        settling_window_milliseconds: u64,
        maximum_sample_gap_milliseconds: u64,
        maximum_measurement_uncertainty_milliseconds: u64,
    ) -> Result<Self, GridInterconnectionError> {
        if ride_through_bands.is_empty() || ride_through_bands.len() > MAX_RIDE_THROUGH_BANDS {
            return Err(GridInterconnectionError::InvalidRideThroughBandCount(
                ride_through_bands.len(),
            ));
        }
        if ride_through_bands[0].start_offset_milliseconds != 0 {
            return Err(GridInterconnectionError::RideThroughBandsMustStartAtZero);
        }
        for (index, pair) in ride_through_bands.windows(2).enumerate() {
            if pair[0].end_offset_milliseconds != pair[1].start_offset_milliseconds {
                return Err(GridInterconnectionError::RideThroughBandGapOrOverlap {
                    preceding_index: index,
                });
            }
        }
        for (name, value) in [
            ("baseline active power kW", baseline_active_power_kw),
            ("baseline reactive power kvar", baseline_reactive_power_kvar),
        ] {
            if !value.is_finite() {
                return Err(GridInterconnectionError::InvalidFiniteQuantity { name, value });
            }
        }
        for (name, value) in [
            ("active-power recovery tolerance kW", active_power_recovery_tolerance_kw),
            ("reactive-power recovery tolerance kvar", reactive_power_recovery_tolerance_kvar),
            ("maximum active-power ramp kW/s", maximum_active_power_ramp_kw_per_second),
            ("maximum reactive-power ramp kvar/s", maximum_reactive_power_ramp_kvar_per_second),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(GridInterconnectionError::InvalidNonNegativeQuantity { name, value });
            }
        }
        if maximum_active_power_ramp_kw_per_second == 0.0
            || maximum_reactive_power_ramp_kvar_per_second == 0.0
            || maximum_recovery_duration_milliseconds == 0
            || settling_window_milliseconds == 0
            || maximum_sample_gap_milliseconds == 0
            || settling_window_milliseconds > maximum_recovery_duration_milliseconds
            || maximum_sample_gap_milliseconds > settling_window_milliseconds
            || maximum_sample_gap_milliseconds
                > ride_through_bands.iter()
                    .map(|band| band.end_offset_milliseconds - band.start_offset_milliseconds)
                    .min()
                    .unwrap_or(0)
        {
            return Err(GridInterconnectionError::InvalidSamplingOrRecoveryContract);
        }
        let event_end = ride_through_bands
            .last()
            .map(|band| band.end_offset_milliseconds)
            .ok_or(GridInterconnectionError::InvalidRideThroughBandCount(0))?;
        if event_end
            .checked_add(maximum_recovery_duration_milliseconds)
            .and_then(|deadline| deadline.checked_add(settling_window_milliseconds))
            .is_none()
        {
            return Err(GridInterconnectionError::TimestampOverflow);
        }
        Ok(Self {
            identity,
            ride_through_bands,
            baseline_active_power_kw,
            baseline_reactive_power_kvar,
            active_power_recovery_tolerance_kw,
            reactive_power_recovery_tolerance_kvar,
            maximum_active_power_ramp_kw_per_second,
            maximum_reactive_power_ramp_kvar_per_second,
            maximum_recovery_duration_milliseconds,
            settling_window_milliseconds,
            maximum_sample_gap_milliseconds,
            maximum_measurement_uncertainty_milliseconds,
        })
    }

    pub fn identity(&self) -> &GridInterconnectionIdentity { &self.identity }
    pub fn ride_through_bands(&self) -> &[GridRideThroughBand] { &self.ride_through_bands }
    pub const fn baseline_active_power_kw(&self) -> f64 { self.baseline_active_power_kw }
    pub const fn baseline_reactive_power_kvar(&self) -> f64 { self.baseline_reactive_power_kvar }
    pub const fn maximum_recovery_duration_milliseconds(&self) -> u64 {
        self.maximum_recovery_duration_milliseconds
    }
    pub const fn settling_window_milliseconds(&self) -> u64 {
        self.settling_window_milliseconds
    }
    pub const fn maximum_sample_gap_milliseconds(&self) -> u64 {
        self.maximum_sample_gap_milliseconds
    }
}

/// Quality classification supplied by the independent measurement pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GridInterconnectionSampleQuality {
    Valid,
    Stale,
    TimeUncertain,
    Invalid,
}

/// One simultaneous PCC voltage/frequency/P/Q/connection sample.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridInterconnectionSample {
    elapsed_milliseconds: u64,
    voltage_pu: f64,
    frequency_hz: f64,
    active_power_kw: f64,
    reactive_power_kvar: f64,
    breaker_closed: bool,
    measurement_uncertainty_milliseconds: u64,
    quality: GridInterconnectionSampleQuality,
    evidence_id: String,
}

impl GridInterconnectionSample {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        elapsed_milliseconds: u64,
        voltage_pu: f64,
        frequency_hz: f64,
        active_power_kw: f64,
        reactive_power_kvar: f64,
        breaker_closed: bool,
        measurement_uncertainty_milliseconds: u64,
        quality: GridInterconnectionSampleQuality,
        evidence_id: impl Into<String>,
    ) -> Result<Self, GridInterconnectionError> {
        for (name, value) in [
            ("voltage pu", voltage_pu),
            ("frequency Hz", frequency_hz),
            ("active power kW", active_power_kw),
            ("reactive power kvar", reactive_power_kvar),
        ] {
            if !value.is_finite() {
                return Err(GridInterconnectionError::InvalidFiniteQuantity { name, value });
            }
        }
        if voltage_pu < 0.0 || frequency_hz <= 0.0 {
            return Err(GridInterconnectionError::InvalidElectricalSample {
                voltage_pu,
                frequency_hz,
            });
        }
        let evidence_id = evidence_id.into();
        require_id("sample evidence ID", &evidence_id)?;
        Ok(Self {
            elapsed_milliseconds,
            voltage_pu,
            frequency_hz,
            active_power_kw,
            reactive_power_kvar,
            breaker_closed,
            measurement_uncertainty_milliseconds,
            quality,
            evidence_id,
        })
    }

    pub const fn elapsed_milliseconds(&self) -> u64 { self.elapsed_milliseconds }
    pub const fn voltage_pu(&self) -> f64 { self.voltage_pu }
    pub const fn frequency_hz(&self) -> f64 { self.frequency_hz }
    pub const fn active_power_kw(&self) -> f64 { self.active_power_kw }
    pub const fn reactive_power_kvar(&self) -> f64 { self.reactive_power_kvar }
    pub const fn breaker_closed(&self) -> bool { self.breaker_closed }
    pub const fn quality(&self) -> GridInterconnectionSampleQuality { self.quality }
    pub fn evidence_id(&self) -> &str { &self.evidence_id }
}

/// Machine-readable event-envelope violation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum GridInterconnectionViolation {
    BreakerOpened { sample_index: usize },
    VoltageOutsideRideThroughBand {
        sample_index: usize,
        observed_pu: f64,
        minimum_pu: f64,
        maximum_pu: f64,
    },
    FrequencyOutsideRideThroughBand {
        sample_index: usize,
        observed_hz: f64,
        minimum_hz: f64,
        maximum_hz: f64,
    },
    SampleQualityNotValid {
        sample_index: usize,
        quality: GridInterconnectionSampleQuality,
    },
    MeasurementUncertaintyExceeded {
        sample_index: usize,
        observed_milliseconds: u64,
        maximum_milliseconds: u64,
    },
    ActivePowerRecoveryRampExceeded {
        sample_index: usize,
        observed_kw_per_second: f64,
        maximum_kw_per_second: f64,
    },
    ReactivePowerRecoveryRampExceeded {
        sample_index: usize,
        observed_kvar_per_second: f64,
        maximum_kvar_per_second: f64,
    },
    RecoveryNotCompletedWithinDeadline {
        recovery_deadline_milliseconds: u64,
    },
}

/// Auditable ride-through/recovery result for one declared PCC boundary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridInterconnectionReport {
    pub identity: GridInterconnectionIdentity,
    pub claim_ceiling: String,
    pub event_duration_milliseconds: u64,
    pub sample_count: usize,
    pub sampling_quality_passed: bool,
    pub ride_through_passed: bool,
    pub breaker_continuity_passed: bool,
    pub recovery_ramp_passed: bool,
    pub recovery_passed: bool,
    pub overall_passed: bool,
    pub maximum_observed_active_power_ramp_kw_per_second: f64,
    pub maximum_observed_reactive_power_ramp_kvar_per_second: f64,
    pub recovery_time_milliseconds: Option<u64>,
    pub stable_settling_duration_milliseconds: Option<u64>,
    pub sub_sample_transients_unverified: bool,
    pub violations: Vec<GridInterconnectionViolation>,
    pub sample_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GridInterconnectionError {
    MissingIdentity { name: &'static str },
    IdentityTooLong { name: &'static str, maximum_bytes: usize, observed_bytes: usize },
    InvalidPositiveQuantity { name: &'static str, value: f64 },
    InvalidFiniteQuantity { name: &'static str, value: f64 },
    InvalidNonNegativeQuantity { name: &'static str, value: f64 },
    InvalidRideThroughBand,
    InvalidRideThroughBandCount(usize),
    RideThroughBandsMustStartAtZero,
    RideThroughBandGapOrOverlap { preceding_index: usize },
    InvalidSamplingOrRecoveryContract,
    TimestampOverflow,
    InvalidElectricalSample { voltage_pu: f64, frequency_hz: f64 },
    TooFewSamples(usize),
    DuplicateSampleEvidenceId { sample_index: usize, evidence_id: String },
    FirstSampleMustStartAtZero,
    NonIncreasingSampleTime { sample_index: usize },
    SampleGapExceeded {
        sample_index: usize,
        observed_milliseconds: u64,
        maximum_milliseconds: u64,
    },
    SampleOutsideContractHorizon {
        sample_index: usize,
        observed_milliseconds: u64,
        maximum_milliseconds: u64,
    },
    NoSamplesDuringRideThrough,
}

impl fmt::Display for GridInterconnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::IdentityTooLong { name, maximum_bytes, observed_bytes } => {
                write!(f, "{name} exceeds {maximum_bytes} bytes; got {observed_bytes}")
            }
            Self::InvalidPositiveQuantity { name, value } => {
                write!(f, "{name} must be finite and positive; got {value}")
            }
            Self::InvalidFiniteQuantity { name, value } => {
                write!(f, "{name} must be finite; got {value}")
            }
            Self::InvalidNonNegativeQuantity { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::InvalidRideThroughBand => write!(f, "ride-through band bounds or duration are invalid"),
            Self::InvalidRideThroughBandCount(count) => {
                write!(f, "ride-through band count must be in 1..={MAX_RIDE_THROUGH_BANDS}; got {count}")
            }
            Self::RideThroughBandsMustStartAtZero => write!(f, "ride-through bands must start at offset zero"),
            Self::RideThroughBandGapOrOverlap { preceding_index } => {
                write!(f, "ride-through bands contain a gap or overlap after band {preceding_index}")
            }
            Self::InvalidSamplingOrRecoveryContract => write!(f, "sample-gap, ramp, recovery or settling contract is invalid"),
            Self::TimestampOverflow => write!(f, "disturbance plus recovery horizon overflows timestamp range"),
            Self::InvalidElectricalSample { voltage_pu, frequency_hz } => {
                write!(f, "invalid electrical sample voltage={voltage_pu} pu, frequency={frequency_hz} Hz")
            }
            Self::TooFewSamples(count) => write!(f, "at least three samples are required; got {count}"),
            Self::DuplicateSampleEvidenceId { sample_index, evidence_id } => write!(
                f, "sample evidence ID {evidence_id} is duplicated at sample index {sample_index}"
            ),
            Self::FirstSampleMustStartAtZero => write!(f, "first disturbance sample must start at elapsed time zero"),
            Self::NonIncreasingSampleTime { sample_index } => {
                write!(f, "sample time is not strictly increasing at index {sample_index}")
            }
            Self::SampleGapExceeded { sample_index, observed_milliseconds, maximum_milliseconds } => write!(
                f, "sample gap {observed_milliseconds} ms at index {sample_index} exceeds {maximum_milliseconds} ms"
            ),
            Self::SampleOutsideContractHorizon { sample_index, observed_milliseconds, maximum_milliseconds } => write!(
                f, "sample at {observed_milliseconds} ms (index {sample_index}) exceeds contract horizon {maximum_milliseconds} ms"
            ),
            Self::NoSamplesDuringRideThrough => write!(f, "no samples cover the ride-through disturbance interval"),
        }
    }
}

impl std::error::Error for GridInterconnectionError {}

/// Verify one complete disturbance, ride-through and recovery trace.
pub fn evaluate_grid_interconnection_event(
    contract: &GridInterconnectionContract,
    samples: &[GridInterconnectionSample],
) -> Result<GridInterconnectionReport, GridInterconnectionError> {
    if samples.len() < 3 {
        return Err(GridInterconnectionError::TooFewSamples(samples.len()));
    }
    if samples[0].elapsed_milliseconds != 0 {
        return Err(GridInterconnectionError::FirstSampleMustStartAtZero);
    }
    let mut evidence_ids = BTreeSet::new();
    for (sample_index, sample) in samples.iter().enumerate() {
        if !evidence_ids.insert(sample.evidence_id.as_str()) {
            return Err(GridInterconnectionError::DuplicateSampleEvidenceId {
                sample_index,
                evidence_id: sample.evidence_id.clone(),
            });
        }
    }
    let event_end = contract.ride_through_bands.last()
        .map(|band| band.end_offset_milliseconds)
        .ok_or(GridInterconnectionError::InvalidRideThroughBandCount(0))?;
    let horizon_end = event_end
        .checked_add(contract.maximum_recovery_duration_milliseconds)
        .and_then(|deadline| deadline.checked_add(contract.settling_window_milliseconds))
        .ok_or(GridInterconnectionError::TimestampOverflow)?;

    for (index, pair) in samples.windows(2).enumerate() {
        if pair[1].elapsed_milliseconds <= pair[0].elapsed_milliseconds {
            return Err(GridInterconnectionError::NonIncreasingSampleTime {
                sample_index: index + 1,
            });
        }
        let gap = pair[1].elapsed_milliseconds - pair[0].elapsed_milliseconds;
        if gap > contract.maximum_sample_gap_milliseconds {
            return Err(GridInterconnectionError::SampleGapExceeded {
                sample_index: index + 1,
                observed_milliseconds: gap,
                maximum_milliseconds: contract.maximum_sample_gap_milliseconds,
            });
        }
    }
    for (index, sample) in samples.iter().enumerate() {
        if sample.elapsed_milliseconds > horizon_end {
            return Err(GridInterconnectionError::SampleOutsideContractHorizon {
                sample_index: index,
                observed_milliseconds: sample.elapsed_milliseconds,
                maximum_milliseconds: horizon_end,
            });
        }
    }

    let mut violations = Vec::new();
    let mut sampling_quality_passed = true;
    let mut breaker_continuity_passed = true;
    let mut ride_through_passed = true;
    let mut recovery_ramp_passed = true;
    let mut maximum_active_ramp = 0.0_f64;
    let mut maximum_reactive_ramp = 0.0_f64;
    let mut ride_through_sample_count = 0_usize;

    for (index, sample) in samples.iter().enumerate() {
        if sample.quality != GridInterconnectionSampleQuality::Valid {
            sampling_quality_passed = false;
            violations.push(GridInterconnectionViolation::SampleQualityNotValid {
                sample_index: index,
                quality: sample.quality,
            });
        }
        if sample.measurement_uncertainty_milliseconds
            > contract.maximum_measurement_uncertainty_milliseconds
        {
            sampling_quality_passed = false;
            violations.push(GridInterconnectionViolation::MeasurementUncertaintyExceeded {
                sample_index: index,
                observed_milliseconds: sample.measurement_uncertainty_milliseconds,
                maximum_milliseconds: contract.maximum_measurement_uncertainty_milliseconds,
            });
        }
        if sample.elapsed_milliseconds < event_end {
            ride_through_sample_count += 1;
            if !sample.breaker_closed {
                breaker_continuity_passed = false;
                violations.push(GridInterconnectionViolation::BreakerOpened {
                    sample_index: index,
                });
            }
            let band = contract.ride_through_bands.iter().find(|band| {
                band.start_offset_milliseconds <= sample.elapsed_milliseconds
                    && sample.elapsed_milliseconds < band.end_offset_milliseconds
            }).ok_or(GridInterconnectionError::NoSamplesDuringRideThrough)?;
            if sample.voltage_pu < band.minimum_voltage_pu
                || sample.voltage_pu > band.maximum_voltage_pu
            {
                ride_through_passed = false;
                violations.push(GridInterconnectionViolation::VoltageOutsideRideThroughBand {
                    sample_index: index,
                    observed_pu: sample.voltage_pu,
                    minimum_pu: band.minimum_voltage_pu,
                    maximum_pu: band.maximum_voltage_pu,
                });
            }
            if sample.frequency_hz < band.minimum_frequency_hz
                || sample.frequency_hz > band.maximum_frequency_hz
            {
                ride_through_passed = false;
                violations.push(GridInterconnectionViolation::FrequencyOutsideRideThroughBand {
                    sample_index: index,
                    observed_hz: sample.frequency_hz,
                    minimum_hz: band.minimum_frequency_hz,
                    maximum_hz: band.maximum_frequency_hz,
                });
            }
        }
        if !sample.breaker_closed {
            breaker_continuity_passed = false;
        }
    }
    if ride_through_sample_count == 0 {
        return Err(GridInterconnectionError::NoSamplesDuringRideThrough);
    }

    for index in 1..samples.len() {
        let previous = &samples[index - 1];
        let current = &samples[index];
        if current.elapsed_milliseconds < event_end {
            continue;
        }
        let elapsed_seconds = (current.elapsed_milliseconds - previous.elapsed_milliseconds) as f64 / 1_000.0;
        if elapsed_seconds <= 0.0 || !elapsed_seconds.is_finite() {
            return Err(GridInterconnectionError::InvalidSamplingOrRecoveryContract);
        }
        let active_ramp = (current.active_power_kw - previous.active_power_kw).abs() / elapsed_seconds;
        let reactive_ramp = (current.reactive_power_kvar - previous.reactive_power_kvar).abs() / elapsed_seconds;
        if !active_ramp.is_finite() || !reactive_ramp.is_finite() {
            return Err(GridInterconnectionError::InvalidSamplingOrRecoveryContract);
        }
        maximum_active_ramp = maximum_active_ramp.max(active_ramp);
        maximum_reactive_ramp = maximum_reactive_ramp.max(reactive_ramp);
        if active_ramp > contract.maximum_active_power_ramp_kw_per_second {
            recovery_ramp_passed = false;
            violations.push(GridInterconnectionViolation::ActivePowerRecoveryRampExceeded {
                sample_index: index,
                observed_kw_per_second: active_ramp,
                maximum_kw_per_second: contract.maximum_active_power_ramp_kw_per_second,
            });
        }
        if reactive_ramp > contract.maximum_reactive_power_ramp_kvar_per_second {
            recovery_ramp_passed = false;
            violations.push(GridInterconnectionViolation::ReactivePowerRecoveryRampExceeded {
                sample_index: index,
                observed_kvar_per_second: reactive_ramp,
                maximum_kvar_per_second: contract.maximum_reactive_power_ramp_kvar_per_second,
            });
        }
    }

    let recovery_deadline = event_end + contract.maximum_recovery_duration_milliseconds;
    let mut recovery_time = None;
    let mut settled_duration = None;
    for (candidate_index, candidate) in samples.iter().enumerate() {
        if candidate.elapsed_milliseconds < event_end
            || candidate.elapsed_milliseconds > recovery_deadline
        {
            continue;
        }
        if samples[candidate_index..].iter().all(|sample| {
            (sample.active_power_kw - contract.baseline_active_power_kw).abs()
                <= contract.active_power_recovery_tolerance_kw
                && (sample.reactive_power_kvar - contract.baseline_reactive_power_kvar).abs()
                    <= contract.reactive_power_recovery_tolerance_kvar
                && sample.breaker_closed
                && sample.quality == GridInterconnectionSampleQuality::Valid
                && sample.measurement_uncertainty_milliseconds
                    <= contract.maximum_measurement_uncertainty_milliseconds
        }) {
            let observed_settling = samples.last().map(|last| {
                last.elapsed_milliseconds.saturating_sub(candidate.elapsed_milliseconds)
            }).unwrap_or(0);
            if observed_settling >= contract.settling_window_milliseconds {
                recovery_time = Some(candidate.elapsed_milliseconds - event_end);
                settled_duration = Some(observed_settling);
                break;
            }
        }
    }

    let recovery_passed = recovery_time.is_some();
    if !recovery_passed {
        violations.push(GridInterconnectionViolation::RecoveryNotCompletedWithinDeadline {
            recovery_deadline_milliseconds: recovery_deadline,
        });
    }
    let overall_passed = sampling_quality_passed
        && ride_through_passed
        && breaker_continuity_passed
        && recovery_ramp_passed
        && recovery_passed;

    Ok(GridInterconnectionReport {
        identity: contract.identity.clone(),
        claim_ceiling: "PCC_RIDE_THROUGH_AND_POST_DISTURBANCE_RECOVERY_ENVELOPE_FOR_PINNED_SITE_EVENT_AND_OPERATOR_SUPPLIED_LIMITS_NOT_GRID_CODE_CERTIFICATION_OR_SYSTEM_STABILITY".to_string(),
        event_duration_milliseconds: event_end,
        sample_count: samples.len(),
        sampling_quality_passed,
        ride_through_passed,
        breaker_continuity_passed,
        recovery_ramp_passed,
        recovery_passed,
        overall_passed,
        maximum_observed_active_power_ramp_kw_per_second: maximum_active_ramp,
        maximum_observed_reactive_power_ramp_kvar_per_second: maximum_reactive_ramp,
        recovery_time_milliseconds: recovery_time,
        stable_settling_duration_milliseconds: settled_duration,
        sub_sample_transients_unverified: true,
        violations,
        sample_evidence_ids: samples.iter().map(|sample| sample.evidence_id.clone()).collect(),
    })
}

fn require_id(name: &'static str, value: &str) -> Result<(), GridInterconnectionError> {
    if value.trim().is_empty() {
        Err(GridInterconnectionError::MissingIdentity { name })
    } else if value.len() > MAX_ID_BYTES {
        Err(GridInterconnectionError::IdentityTooLong {
            name,
            maximum_bytes: MAX_ID_BYTES,
            observed_bytes: value.len(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> GridInterconnectionIdentity {
        GridInterconnectionIdentity::new(
            "event-1", "site-1", "pcc-1", "agreement-v1", "operator-envelope-v1", "meter-chain-v1",
        ).expect("identity")
    }

    fn contract(
        max_active_ramp: f64,
        recovery_duration: u64,
        settle: u64,
    ) -> GridInterconnectionContract {
        GridInterconnectionContract::new(
            identity(),
            vec![GridRideThroughBand::new(0, 2_000, 0.70, 1.10, 48.0, 52.0).expect("band")],
            100.0,
            10.0,
            2.0,
            1.0,
            max_active_ramp,
            100.0,
            recovery_duration,
            settle,
            1_000,
            5,
        ).expect("contract")
    }

    fn sample(
        time: u64,
        voltage: f64,
        frequency: f64,
        active: f64,
        reactive: f64,
        breaker: bool,
    ) -> GridInterconnectionSample {
        GridInterconnectionSample::new(
            time, voltage, frequency, active, reactive, breaker, 1,
            GridInterconnectionSampleQuality::Valid, format!("sample-{time}"),
        ).expect("sample")
    }

    fn passing_trace() -> Vec<GridInterconnectionSample> {
        vec![
            sample(0, 0.80, 49.0, 100.0, 10.0, true),
            sample(1_000, 0.90, 49.5, 80.0, 12.0, true),
            sample(2_000, 1.00, 50.0, 100.0, 10.0, true),
            sample(3_000, 1.00, 50.0, 100.0, 10.0, true),
            sample(4_000, 1.00, 50.0, 100.0, 10.0, true),
        ]
    }

    #[test]
    fn in_envelope_ride_through_and_recovery_pass() {
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &passing_trace())
            .expect("report");
        assert!(report.overall_passed);
        assert!(report.ride_through_passed);
        assert!(report.breaker_continuity_passed);
        assert!(report.recovery_passed);
        assert_eq!(report.recovery_time_milliseconds, Some(0));
        assert!(report.sub_sample_transients_unverified);
    }

    #[test]
    fn breaker_open_during_disturbance_fails_ride_through() {
        let mut trace = passing_trace();
        trace[1] = sample(1_000, 0.90, 49.5, 80.0, 12.0, false);
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace)
            .expect("report");
        assert!(!report.overall_passed);
        assert!(!report.breaker_continuity_passed);
        assert!(report.violations.iter().any(|violation| {
            matches!(violation, GridInterconnectionViolation::BreakerOpened { sample_index: 1 })
        }));
    }

    #[test]
    fn voltage_and_frequency_outside_the_declared_band_fail() {
        let mut trace = passing_trace();
        trace[1] = sample(1_000, 0.60, 47.0, 80.0, 12.0, true);
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace)
            .expect("report");
        assert!(!report.ride_through_passed);
        assert!(report.violations.iter().any(|v| matches!(
            v, GridInterconnectionViolation::VoltageOutsideRideThroughBand { .. }
        )));
        assert!(report.violations.iter().any(|v| matches!(
            v, GridInterconnectionViolation::FrequencyOutsideRideThroughBand { .. }
        )));
    }

    #[test]
    fn recovery_after_deadline_does_not_pass() {
        let trace = vec![
            sample(0, 0.80, 49.0, 100.0, 10.0, true),
            sample(1_000, 0.90, 49.5, 80.0, 12.0, true),
            sample(2_000, 1.0, 50.0, 80.0, 12.0, true),
            sample(3_000, 1.0, 50.0, 80.0, 12.0, true),
            sample(4_000, 1.0, 50.0, 80.0, 12.0, true),
            sample(5_000, 1.0, 50.0, 80.0, 12.0, true),
            sample(6_000, 1.0, 50.0, 80.0, 12.0, true),
            sample(7_000, 1.0, 50.0, 80.0, 12.0, true),
            sample(8_000, 1.0, 50.0, 100.0, 10.0, true),
            sample(9_000, 1.0, 50.0, 100.0, 10.0, true),
        ];
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace)
            .expect("report");
        assert!(!report.recovery_passed);
        assert!(report.violations.iter().any(|v| matches!(
            v, GridInterconnectionViolation::RecoveryNotCompletedWithinDeadline { .. }
        )));
    }

    #[test]
    fn oscillatory_power_never_completes_the_settling_window() {
        let trace = vec![
            sample(0, 0.80, 49.0, 100.0, 10.0, true),
            sample(1_000, 0.90, 49.5, 80.0, 12.0, true),
            sample(2_000, 1.0, 50.0, 100.0, 10.0, true),
            sample(3_000, 1.0, 50.0, 110.0, 10.0, true),
            sample(4_000, 1.0, 50.0, 100.0, 10.0, true),
            sample(5_000, 1.0, 50.0, 110.0, 10.0, true),
            sample(6_000, 1.0, 50.0, 100.0, 10.0, true),
        ];
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace)
            .expect("report");
        assert!(!report.recovery_passed);
    }

    #[test]
    fn excessive_post_event_ramp_fails_independently() {
        let report = evaluate_grid_interconnection_event(&contract(5.0, 5_000, 2_000), &passing_trace())
            .expect("report");
        assert!(!report.recovery_ramp_passed);
        assert!(report.maximum_observed_active_power_ramp_kw_per_second > 5.0);
    }

    #[test]
    fn stale_or_time_uncertain_sample_fails_sampling_gate() {
        let mut trace = passing_trace();
        trace[1] = GridInterconnectionSample::new(
            1_000, 0.90, 49.5, 80.0, 12.0, true, 1,
            GridInterconnectionSampleQuality::Stale, "stale-sample",
        ).expect("sample");
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace)
            .expect("report");
        assert!(!report.sampling_quality_passed);
        assert!(!report.overall_passed);
    }

    #[test]
    fn inadequate_measurement_uncertainty_fails_sampling_gate() {
        let mut trace = passing_trace();
        trace[1] = GridInterconnectionSample::new(
            1_000, 0.90, 49.5, 80.0, 12.0, true, 6,
            GridInterconnectionSampleQuality::Valid, "uncertain-sample",
        ).expect("sample");
        let report = evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace)
            .expect("report");
        assert!(!report.sampling_quality_passed);
    }

    #[test]
    fn gaps_larger_than_sampling_contract_are_rejected() {
        let trace = vec![
            sample(0, 0.8, 49.0, 100.0, 10.0, true),
            sample(2_000, 1.0, 50.0, 100.0, 10.0, true),
            sample(3_000, 1.0, 50.0, 100.0, 10.0, true),
        ];
        assert!(matches!(
            evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace),
            Err(GridInterconnectionError::SampleGapExceeded { .. })
        ));
    }

    #[test]
    fn non_contiguous_ride_through_bands_are_rejected() {
        let bands = vec![
            GridRideThroughBand::new(0, 1_000, 0.7, 1.1, 48.0, 52.0).expect("band"),
            GridRideThroughBand::new(1_500, 2_000, 0.8, 1.1, 49.0, 51.0).expect("band"),
        ];
        assert!(matches!(
            GridInterconnectionContract::new(
                identity(), bands, 100.0, 10.0, 2.0, 1.0, 100.0, 100.0,
                5_000, 2_000, 500, 5,
            ),
            Err(GridInterconnectionError::RideThroughBandGapOrOverlap { .. })
        ));
    }

    #[test]
    fn duplicate_sample_evidence_ids_are_rejected() {
        let mut trace = passing_trace();
        trace[4] = GridInterconnectionSample::new(
            4_000, 1.0, 50.0, 100.0, 10.0, true, 1,
            GridInterconnectionSampleQuality::Valid, "sample-3000",
        ).expect("sample");
        assert!(matches!(
            evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &trace),
            Err(GridInterconnectionError::DuplicateSampleEvidenceId { .. })
        ));
    }

    #[test]
    fn sample_time_must_be_strictly_increasing_and_begin_at_zero() {
        let duplicate = vec![
            sample(0, 0.8, 49.0, 100.0, 10.0, true),
            sample(0, 0.9, 49.5, 80.0, 12.0, true),
            sample(2_000, 1.0, 50.0, 100.0, 10.0, true),
        ];
        assert!(matches!(
            evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &duplicate),
            Err(GridInterconnectionError::NonIncreasingSampleTime { .. })
        ));
        let not_zero = vec![
            sample(100, 0.8, 49.0, 100.0, 10.0, true),
            sample(1_000, 0.9, 49.5, 80.0, 12.0, true),
            sample(2_000, 1.0, 50.0, 100.0, 10.0, true),
        ];
        assert!(matches!(
            evaluate_grid_interconnection_event(&contract(100.0, 5_000, 2_000), &not_zero),
            Err(GridInterconnectionError::FirstSampleMustStartAtZero)
        ));
    }
}
