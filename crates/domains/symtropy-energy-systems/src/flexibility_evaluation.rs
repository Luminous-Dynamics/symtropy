// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Matched-service verification for grid-interactive data-centre flexibility.
//!
//! A power-reduction event is not counted as a success merely because PCC
//! import fell. This evaluator also checks completed work, energy per completed
//! work, latency/deadlines, thermal limits and post-event rebound over one
//! frozen comparison horizon.
//!
//! This is an accounting/evaluation primitive. It does not schedule workloads,
//! control UPS/inverters, certify grid services or replace local protection.

use core::fmt;

use serde::Serialize;

/// Frozen event, rebound and service constraints for one flexibility test.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlexibilityEventContract {
    event_start_ns: u64,
    event_end_ns: u64,
    rebound_end_ns: u64,
    sample_interval_ns: u64,
    minimum_power_reduction_fraction: f64,
    maximum_work_loss_fraction: f64,
    maximum_energy_per_work_ratio: f64,
    maximum_rebound_peak_ratio: f64,
    maximum_p99_latency_ms: f64,
    maximum_deadline_misses: u64,
    maximum_temperature_c: f64,
}

impl FlexibilityEventContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_start_ns: u64,
        event_end_ns: u64,
        rebound_end_ns: u64,
        sample_interval_ns: u64,
        minimum_power_reduction_fraction: f64,
        maximum_work_loss_fraction: f64,
        maximum_energy_per_work_ratio: f64,
        maximum_rebound_peak_ratio: f64,
        maximum_p99_latency_ms: f64,
        maximum_deadline_misses: u64,
        maximum_temperature_c: f64,
    ) -> Result<Self, FlexibilityEvaluationError> {
        if event_start_ns >= event_end_ns || event_end_ns >= rebound_end_ns {
            return Err(FlexibilityEvaluationError::InvalidWindow {
                event_start_ns,
                event_end_ns,
                rebound_end_ns,
            });
        }
        if sample_interval_ns == 0
            || (event_end_ns - event_start_ns) % sample_interval_ns != 0
            || (rebound_end_ns - event_end_ns) % sample_interval_ns != 0
        {
            return Err(FlexibilityEvaluationError::InvalidSampleInterval {
                sample_interval_ns,
            });
        }

        for (name, value) in [
            (
                "minimum power reduction fraction",
                minimum_power_reduction_fraction,
            ),
            ("maximum work loss fraction", maximum_work_loss_fraction),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(FlexibilityEvaluationError::InvalidConstraint {
                    name,
                    value,
                });
            }
        }
        for (name, value) in [
            ("maximum energy-per-work ratio", maximum_energy_per_work_ratio),
            ("maximum rebound peak ratio", maximum_rebound_peak_ratio),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(FlexibilityEvaluationError::InvalidConstraint {
                    name,
                    value,
                });
            }
        }
        for (name, value) in [
            ("maximum p99 latency", maximum_p99_latency_ms),
            ("maximum temperature", maximum_temperature_c),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(FlexibilityEvaluationError::InvalidConstraint {
                    name,
                    value,
                });
            }
        }

        Ok(Self {
            event_start_ns,
            event_end_ns,
            rebound_end_ns,
            sample_interval_ns,
            minimum_power_reduction_fraction,
            maximum_work_loss_fraction,
            maximum_energy_per_work_ratio,
            maximum_rebound_peak_ratio,
            maximum_p99_latency_ms,
            maximum_deadline_misses,
            maximum_temperature_c,
        })
    }

    pub const fn event_start_ns(&self) -> u64 {
        self.event_start_ns
    }

    pub const fn event_end_ns(&self) -> u64 {
        self.event_end_ns
    }

    pub const fn rebound_end_ns(&self) -> u64 {
        self.rebound_end_ns
    }

    pub const fn sample_interval_ns(&self) -> u64 {
        self.sample_interval_ns
    }
}

/// One aligned baseline/response interval from the same site and workload contract.
///
/// Power values are facility-PCC import in kW. Work is an explicitly chosen
/// application-specific completed-work unit; its specification must be frozen
/// outside this primitive and identical for baseline and response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlexibilityEventSample {
    elapsed_ns: u64,
    baseline_pcc_import_kw: f64,
    response_pcc_import_kw: f64,
    baseline_completed_work_units: f64,
    response_completed_work_units: f64,
    response_p99_latency_ms: f64,
    response_deadline_misses: u64,
    response_peak_temperature_c: f64,
}

impl FlexibilityEventSample {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        elapsed_ns: u64,
        baseline_pcc_import_kw: f64,
        response_pcc_import_kw: f64,
        baseline_completed_work_units: f64,
        response_completed_work_units: f64,
        response_p99_latency_ms: f64,
        response_deadline_misses: u64,
        response_peak_temperature_c: f64,
    ) -> Result<Self, FlexibilityEvaluationError> {
        for (name, value) in [
            ("baseline PCC import", baseline_pcc_import_kw),
            ("response PCC import", response_pcc_import_kw),
            ("baseline completed work", baseline_completed_work_units),
            ("response completed work", response_completed_work_units),
            ("response p99 latency", response_p99_latency_ms),
            ("response peak temperature", response_peak_temperature_c),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(FlexibilityEvaluationError::InvalidSampleValue {
                    name,
                    value,
                });
            }
        }

        Ok(Self {
            elapsed_ns,
            baseline_pcc_import_kw,
            response_pcc_import_kw,
            baseline_completed_work_units,
            response_completed_work_units,
            response_p99_latency_ms,
            response_deadline_misses,
            response_peak_temperature_c,
        })
    }

    pub const fn elapsed_ns(&self) -> u64 {
        self.elapsed_ns
    }
}

/// Per-gate evaluation and the complete matched-horizon report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlexibilityEvaluationReport {
    sample_count: usize,
    event_duration_ns: u64,
    rebound_duration_ns: u64,
    baseline_event_energy_kwh: f64,
    response_event_energy_kwh: f64,
    baseline_total_energy_kwh: f64,
    response_total_energy_kwh: f64,
    event_power_reduction_fraction: f64,
    baseline_completed_work_units: f64,
    response_completed_work_units: f64,
    work_loss_fraction: f64,
    energy_per_work_ratio: f64,
    baseline_rebound_peak_kw: f64,
    response_rebound_peak_kw: f64,
    rebound_peak_ratio: f64,
    maximum_response_p99_latency_ms: f64,
    response_deadline_misses: u64,
    maximum_response_temperature_c: f64,
    power_reduction_pass: bool,
    work_completion_pass: bool,
    energy_per_work_pass: bool,
    rebound_pass: bool,
    latency_and_deadline_pass: bool,
    thermal_pass: bool,
    passed: bool,
}

impl FlexibilityEvaluationReport {
    pub const fn sample_count(&self) -> usize {
        self.sample_count
    }

    pub const fn event_duration_ns(&self) -> u64 {
        self.event_duration_ns
    }

    pub const fn rebound_duration_ns(&self) -> u64 {
        self.rebound_duration_ns
    }

    pub const fn baseline_event_energy_kwh(&self) -> f64 {
        self.baseline_event_energy_kwh
    }

    pub const fn response_event_energy_kwh(&self) -> f64 {
        self.response_event_energy_kwh
    }

    pub const fn baseline_total_energy_kwh(&self) -> f64 {
        self.baseline_total_energy_kwh
    }

    pub const fn response_total_energy_kwh(&self) -> f64 {
        self.response_total_energy_kwh
    }

    pub const fn event_power_reduction_fraction(&self) -> f64 {
        self.event_power_reduction_fraction
    }

    pub const fn baseline_completed_work_units(&self) -> f64 {
        self.baseline_completed_work_units
    }

    pub const fn response_completed_work_units(&self) -> f64 {
        self.response_completed_work_units
    }

    pub const fn work_loss_fraction(&self) -> f64 {
        self.work_loss_fraction
    }

    pub const fn energy_per_work_ratio(&self) -> f64 {
        self.energy_per_work_ratio
    }

    pub const fn baseline_rebound_peak_kw(&self) -> f64 {
        self.baseline_rebound_peak_kw
    }

    pub const fn response_rebound_peak_kw(&self) -> f64 {
        self.response_rebound_peak_kw
    }

    pub const fn rebound_peak_ratio(&self) -> f64 {
        self.rebound_peak_ratio
    }

    pub const fn maximum_response_p99_latency_ms(&self) -> f64 {
        self.maximum_response_p99_latency_ms
    }

    pub const fn response_deadline_misses(&self) -> u64 {
        self.response_deadline_misses
    }

    pub const fn maximum_response_temperature_c(&self) -> f64 {
        self.maximum_response_temperature_c
    }

    pub const fn power_reduction_pass(&self) -> bool {
        self.power_reduction_pass
    }

    pub const fn work_completion_pass(&self) -> bool {
        self.work_completion_pass
    }

    pub const fn energy_per_work_pass(&self) -> bool {
        self.energy_per_work_pass
    }

    pub const fn rebound_pass(&self) -> bool {
        self.rebound_pass
    }

    pub const fn latency_and_deadline_pass(&self) -> bool {
        self.latency_and_deadline_pass
    }

    pub const fn thermal_pass(&self) -> bool {
        self.thermal_pass
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }
}

/// Evaluate a baseline and grid-response event over the exact same time horizon.
///
/// Samples must start at the event start and continue at the frozen interval
/// through the rebound horizon. Work and QoS are assessed across the complete
/// event-plus-rebound horizon; the power-reduction metric applies only to the
/// event window.
pub fn evaluate_flexibility_event(
    contract: &FlexibilityEventContract,
    samples: &[FlexibilityEventSample],
) -> Result<FlexibilityEvaluationReport, FlexibilityEvaluationError> {
    if samples.is_empty() {
        return Err(FlexibilityEvaluationError::EmptySamples);
    }

    let total_duration_ns = contract.rebound_end_ns - contract.event_start_ns;
    let expected_samples = usize::try_from(total_duration_ns / contract.sample_interval_ns)
        .map_err(|_| FlexibilityEvaluationError::SampleCountOverflow)?;

    if samples.len() != expected_samples {
        return Err(FlexibilityEvaluationError::SampleCountMismatch {
            expected: expected_samples,
            actual: samples.len(),
        });
    }

    let interval_hours = contract.sample_interval_ns as f64 / 3_600_000_000_000.0;
    let event_duration_ns = contract.event_end_ns - contract.event_start_ns;
    let rebound_duration_ns = contract.rebound_end_ns - contract.event_end_ns;

    let mut baseline_event_energy_kwh = 0.0;
    let mut response_event_energy_kwh = 0.0;
    let mut baseline_total_energy_kwh = 0.0;
    let mut response_total_energy_kwh = 0.0;
    let mut baseline_completed_work_units = 0.0;
    let mut response_completed_work_units = 0.0;
    let mut baseline_rebound_peak_kw: f64 = 0.0;
    let mut response_rebound_peak_kw: f64 = 0.0;
    let mut maximum_response_p99_latency_ms: f64 = 0.0;
    let mut response_deadline_misses = 0_u64;
    let mut maximum_response_temperature_c: f64 = 0.0;

    for (index, sample) in samples.iter().enumerate() {
        let offset = u64::try_from(index)
            .ok()
            .and_then(|i| i.checked_mul(contract.sample_interval_ns))
            .ok_or(FlexibilityEvaluationError::SampleTimestampOverflow)?;
        let expected_elapsed = contract
            .event_start_ns
            .checked_add(offset)
            .ok_or(FlexibilityEvaluationError::SampleTimestampOverflow)?;

        if sample.elapsed_ns() != expected_elapsed {
            return Err(FlexibilityEvaluationError::SampleTimestampMismatch {
                index,
                expected_elapsed_ns: expected_elapsed,
                observed_elapsed_ns: sample.elapsed_ns(),
            });
        }

        let baseline_energy = sample.baseline_pcc_import_kw * interval_hours;
        let response_energy = sample.response_pcc_import_kw * interval_hours;
        if !baseline_energy.is_finite() || !response_energy.is_finite() {
            return Err(FlexibilityEvaluationError::EnergyOverflow { index });
        }

        baseline_total_energy_kwh += baseline_energy;
        response_total_energy_kwh += response_energy;
        baseline_completed_work_units += sample.baseline_completed_work_units;
        response_completed_work_units += sample.response_completed_work_units;
        maximum_response_p99_latency_ms =
            maximum_response_p99_latency_ms.max(sample.response_p99_latency_ms);
        response_deadline_misses = response_deadline_misses
            .checked_add(sample.response_deadline_misses)
            .ok_or(FlexibilityEvaluationError::DeadlineMissOverflow)?;
        maximum_response_temperature_c =
            maximum_response_temperature_c.max(sample.response_peak_temperature_c);

        if sample.elapsed_ns() < contract.event_end_ns {
            baseline_event_energy_kwh += baseline_energy;
            response_event_energy_kwh += response_energy;
        } else {
            baseline_rebound_peak_kw =
                baseline_rebound_peak_kw.max(sample.baseline_pcc_import_kw);
            response_rebound_peak_kw =
                response_rebound_peak_kw.max(sample.response_pcc_import_kw);
        }
    }

    for (name, value) in [
        ("baseline event energy", baseline_event_energy_kwh),
        ("response event energy", response_event_energy_kwh),
        ("baseline total energy", baseline_total_energy_kwh),
        ("response total energy", response_total_energy_kwh),
        ("baseline completed work", baseline_completed_work_units),
        ("response completed work", response_completed_work_units),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(FlexibilityEvaluationError::InvalidDerivedValue {
                name,
                value,
            });
        }
    }

    if baseline_event_energy_kwh <= 0.0
        || baseline_total_energy_kwh <= 0.0
        || baseline_completed_work_units <= 0.0
        || response_completed_work_units <= 0.0
    {
        return Err(FlexibilityEvaluationError::ZeroBaselineOrWork);
    }
    if baseline_rebound_peak_kw <= 0.0 {
        return Err(FlexibilityEvaluationError::ZeroBaselineReboundPeak);
    }

    let event_power_reduction_fraction =
        1.0 - (response_event_energy_kwh / baseline_event_energy_kwh);
    let work_loss_fraction =
        (1.0 - (response_completed_work_units / baseline_completed_work_units)).max(0.0);
    let baseline_energy_per_work = baseline_total_energy_kwh / baseline_completed_work_units;
    let response_energy_per_work = response_total_energy_kwh / response_completed_work_units;
    let energy_per_work_ratio = response_energy_per_work / baseline_energy_per_work;
    let rebound_peak_ratio = response_rebound_peak_kw / baseline_rebound_peak_kw;

    for (name, value) in [
        ("event power reduction fraction", event_power_reduction_fraction),
        ("work loss fraction", work_loss_fraction),
        ("energy per work ratio", energy_per_work_ratio),
        ("rebound peak ratio", rebound_peak_ratio),
    ] {
        if !value.is_finite() {
            return Err(FlexibilityEvaluationError::InvalidDerivedValue {
                name,
                value,
            });
        }
    }

    let power_reduction_pass =
        event_power_reduction_fraction >= contract.minimum_power_reduction_fraction;
    let work_completion_pass = work_loss_fraction <= contract.maximum_work_loss_fraction;
    let energy_per_work_pass = energy_per_work_ratio <= contract.maximum_energy_per_work_ratio;
    let rebound_pass = rebound_peak_ratio <= contract.maximum_rebound_peak_ratio;
    let latency_and_deadline_pass = maximum_response_p99_latency_ms
        <= contract.maximum_p99_latency_ms
        && response_deadline_misses <= contract.maximum_deadline_misses;
    let thermal_pass = maximum_response_temperature_c <= contract.maximum_temperature_c;
    let passed = power_reduction_pass
        && work_completion_pass
        && energy_per_work_pass
        && rebound_pass
        && latency_and_deadline_pass
        && thermal_pass;

    Ok(FlexibilityEvaluationReport {
        sample_count: samples.len(),
        event_duration_ns,
        rebound_duration_ns,
        baseline_event_energy_kwh,
        response_event_energy_kwh,
        baseline_total_energy_kwh,
        response_total_energy_kwh,
        event_power_reduction_fraction,
        baseline_completed_work_units,
        response_completed_work_units,
        work_loss_fraction,
        energy_per_work_ratio,
        baseline_rebound_peak_kw,
        response_rebound_peak_kw,
        rebound_peak_ratio,
        maximum_response_p99_latency_ms,
        response_deadline_misses,
        maximum_response_temperature_c,
        power_reduction_pass,
        work_completion_pass,
        energy_per_work_pass,
        rebound_pass,
        latency_and_deadline_pass,
        thermal_pass,
        passed,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlexibilityEvaluationError {
    InvalidWindow {
        event_start_ns: u64,
        event_end_ns: u64,
        rebound_end_ns: u64,
    },
    InvalidSampleInterval {
        sample_interval_ns: u64,
    },
    InvalidConstraint {
        name: &'static str,
        value: f64,
    },
    InvalidSampleValue {
        name: &'static str,
        value: f64,
    },
    EmptySamples,
    SampleCountOverflow,
    SampleCountMismatch {
        expected: usize,
        actual: usize,
    },
    SampleTimestampOverflow,
    SampleTimestampMismatch {
        index: usize,
        expected_elapsed_ns: u64,
        observed_elapsed_ns: u64,
    },
    EnergyOverflow {
        index: usize,
    },
    DeadlineMissOverflow,
    InvalidDerivedValue {
        name: &'static str,
        value: f64,
    },
    ZeroBaselineOrWork,
    ZeroBaselineReboundPeak,
}

impl fmt::Display for FlexibilityEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWindow {
                event_start_ns,
                event_end_ns,
                rebound_end_ns,
            } => write!(
                f,
                "invalid flexibility event/rebound windows: event [{event_start_ns}, {event_end_ns}), rebound ends at {rebound_end_ns}"
            ),
            Self::InvalidSampleInterval { sample_interval_ns } => write!(
                f,
                "sample interval {sample_interval_ns} ns is zero or does not divide the event/rebound windows"
            ),
            Self::InvalidConstraint { name, value } => {
                write!(f, "{name} has invalid value {value}")
            }
            Self::InvalidSampleValue { name, value } => {
                write!(f, "sample {name} must be finite and non-negative; got {value}")
            }
            Self::EmptySamples => write!(f, "flexibility evaluation requires samples"),
            Self::SampleCountOverflow => write!(f, "expected flexibility sample count overflowed"),
            Self::SampleCountMismatch { expected, actual } => write!(
                f,
                "flexibility sample count mismatch: expected {expected}, actual {actual}"
            ),
            Self::SampleTimestampOverflow => write!(f, "flexibility sample timestamp overflowed"),
            Self::SampleTimestampMismatch {
                index,
                expected_elapsed_ns,
                observed_elapsed_ns,
            } => write!(
                f,
                "flexibility sample {index} timestamp mismatch: expected {expected_elapsed_ns}, observed {observed_elapsed_ns}"
            ),
            Self::EnergyOverflow { index } => {
                write!(f, "energy calculation overflowed at sample {index}")
            }
            Self::DeadlineMissOverflow => write!(f, "deadline-miss count overflowed"),
            Self::InvalidDerivedValue { name, value } => {
                write!(f, "derived flexibility metric {name} is invalid: {value}")
            }
            Self::ZeroBaselineOrWork => {
                write!(f, "baseline energy and both completed-work totals must be positive")
            }
            Self::ZeroBaselineReboundPeak => {
                write!(f, "baseline rebound peak must be positive")
            }
        }
    }
}

impl std::error::Error for FlexibilityEvaluationError {}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR_NS: u64 = 3_600_000_000_000;

    fn contract() -> FlexibilityEventContract {
        FlexibilityEventContract::new(
            0,
            3 * HOUR_NS,
            4 * HOUR_NS,
            HOUR_NS,
            0.25,
            0.01,
            1.0,
            1.05,
            100.0,
            0,
            80.0,
        )
        .unwrap()
    }

    fn samples(
        event_response_kw: [f64; 3],
        rebound_response_kw: f64,
        event_response_work: [f64; 3],
        rebound_response_work: f64,
        p99_latency_ms: f64,
        deadline_misses: u64,
        peak_temperature_c: f64,
    ) -> Vec<FlexibilityEventSample> {
        let response_power = [
            event_response_kw[0],
            event_response_kw[1],
            event_response_kw[2],
            rebound_response_kw,
        ];
        let response_work = [
            event_response_work[0],
            event_response_work[1],
            event_response_work[2],
            rebound_response_work,
        ];

        (0..4)
            .map(|index| {
                FlexibilityEventSample::new(
                    index as u64 * HOUR_NS,
                    1_000.0,
                    response_power[index],
                    100.0,
                    response_work[index],
                    p99_latency_ms,
                    if index == 0 { deadline_misses } else { 0 },
                    peak_temperature_c,
                )
                .unwrap()
            })
            .collect()
    }

    #[test]
    fn passes_25_percent_three_hour_reduction_with_work_sla_and_rebound_preserved() {
        let input = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [100.0, 100.0, 100.0],
            100.0,
            50.0,
            0,
            70.0,
        );

        let report = evaluate_flexibility_event(&contract(), &input).unwrap();

        assert!(report.passed());
        assert!((report.event_power_reduction_fraction() - 0.25).abs() < 1e-12);
        assert_eq!(report.event_duration_ns(), 3 * HOUR_NS);
        assert!((report.rebound_peak_ratio() - 1.0).abs() < 1e-12);
        assert!(report.energy_per_work_ratio() < 1.0);
        assert_eq!(report.response_completed_work_units(), 400.0);
    }

    #[test]
    fn rejects_power_reduction_that_loses_completed_work() {
        let input = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [50.0, 50.0, 50.0],
            50.0,
            50.0,
            0,
            70.0,
        );

        let report = evaluate_flexibility_event(&contract(), &input).unwrap();
        assert!(!report.passed());
        assert!(report.power_reduction_pass());
        assert!(!report.work_completion_pass());
    }

    #[test]
    fn rejects_rebound_peak_above_baseline() {
        let input = samples(
            [750.0, 750.0, 750.0],
            1_300.0,
            [100.0, 100.0, 100.0],
            100.0,
            50.0,
            0,
            70.0,
        );

        let report = evaluate_flexibility_event(&contract(), &input).unwrap();
        assert!(report.power_reduction_pass());
        assert!(!report.rebound_pass());
        assert!(!report.passed());
    }

    #[test]
    fn rejects_energy_per_completed_work_regression_even_when_peak_limit_is_relaxed() {
        let relaxed_rebound = FlexibilityEventContract::new(
            0,
            3 * HOUR_NS,
            4 * HOUR_NS,
            HOUR_NS,
            0.25,
            0.01,
            1.0,
            2.0,
            100.0,
            0,
            80.0,
        )
        .unwrap();
        let input = samples(
            [750.0, 750.0, 750.0],
            1_800.0,
            [100.0, 100.0, 100.0],
            100.0,
            50.0,
            0,
            70.0,
        );

        let report = evaluate_flexibility_event(&relaxed_rebound, &input).unwrap();
        assert!(report.power_reduction_pass());
        assert!(report.rebound_pass());
        assert!(!report.energy_per_work_pass());
        assert!(!report.passed());
    }

    #[test]
    fn rejects_latency_deadline_and_thermal_violations() {
        let latency_failure = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [100.0, 100.0, 100.0],
            100.0,
            150.0,
            0,
            70.0,
        );
        let latency_report =
            evaluate_flexibility_event(&contract(), &latency_failure).unwrap();
        assert!(!latency_report.latency_and_deadline_pass());

        let deadline_failure = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [100.0, 100.0, 100.0],
            100.0,
            50.0,
            1,
            70.0,
        );
        let deadline_report =
            evaluate_flexibility_event(&contract(), &deadline_failure).unwrap();
        assert!(!deadline_report.latency_and_deadline_pass());

        let thermal_failure = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [100.0, 100.0, 100.0],
            100.0,
            50.0,
            0,
            85.0,
        );
        let thermal_report =
            evaluate_flexibility_event(&contract(), &thermal_failure).unwrap();
        assert!(!thermal_report.thermal_pass());
    }

    #[test]
    fn rejects_misaligned_or_missing_samples() {
        let mut input = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [100.0, 100.0, 100.0],
            100.0,
            50.0,
            0,
            70.0,
        );
        input[1] = FlexibilityEventSample::new(
            HOUR_NS + 1,
            1_000.0,
            750.0,
            100.0,
            100.0,
            50.0,
            0,
            70.0,
        )
        .unwrap();

        assert!(matches!(
            evaluate_flexibility_event(&contract(), &input),
            Err(FlexibilityEvaluationError::SampleTimestampMismatch { .. })
        ));

        let missing = &input[..3];
        assert!(matches!(
            evaluate_flexibility_event(&contract(), missing),
            Err(FlexibilityEvaluationError::SampleCountMismatch { .. })
        ));
    }

    #[test]
    fn rejects_zero_work_and_invalid_contracts() {
        let zero_work = samples(
            [750.0, 750.0, 750.0],
            1_000.0,
            [0.0, 0.0, 0.0],
            0.0,
            50.0,
            0,
            70.0,
        );
        assert!(matches!(
            evaluate_flexibility_event(&contract(), &zero_work),
            Err(FlexibilityEvaluationError::ZeroBaselineOrWork)
        ));
        assert!(
            FlexibilityEventContract::new(3, 2, 4, 1, 0.25, 0.01, 1.0, 1.05, 100.0, 0, 80.0)
                .is_err()
        );
        assert!(
            FlexibilityEventContract::new(
                0,
                3 * HOUR_NS,
                4 * HOUR_NS,
                0,
                0.25,
                0.01,
                1.0,
                1.05,
                100.0,
                0,
                80.0
            )
            .is_err()
        );
        assert!(
            FlexibilityEventContract::new(
                0,
                3 * HOUR_NS,
                4 * HOUR_NS,
                HOUR_NS,
                0.25,
                0.01,
                0.0,
                1.05,
                100.0,
                0,
                80.0
            )
            .is_err()
        );
    }
}
