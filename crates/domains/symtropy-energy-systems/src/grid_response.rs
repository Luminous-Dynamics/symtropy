// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Time-resolved dispatch-to-delivery response verification.
//!
//! The existing flexibility-event evaluator owns whole-horizon work, facility
//! energy, QoS, thermal and rebound accounting. This module owns the distinct
//! question: did a frozen grid-service instruction produce the requested PCC
//! import reduction quickly enough, sustain it, and respect tracking limits?
//!
//! Baseline and response samples are caller-supplied with separate evidence
//! references. The baseline is not authenticated or made causal here.
//! Ramp is measured only at the declared telemetry resolution; sub-sample
//! transients cannot be inferred from coarse samples.
//!
//! This module does not dispatch equipment, communicate with an aggregator,
//! certify a market product, or replace local electrical protection.

use core::fmt;

use serde::Serialize;

/// Immutable identity of one dispatch-response qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GridResponseIdentity {
    event_id: String,
    service_product_id: String,
    measurement_boundary_id: String,
    baseline_method_id: String,
    response_configuration_digest: String,
}

impl GridResponseIdentity {
    pub fn new(
        event_id: impl Into<String>,
        service_product_id: impl Into<String>,
        measurement_boundary_id: impl Into<String>,
        baseline_method_id: impl Into<String>,
        response_configuration_digest: impl Into<String>,
    ) -> Result<Self, GridResponseError> {
        let identity = Self {
            event_id: event_id.into(),
            service_product_id: service_product_id.into(),
            measurement_boundary_id: measurement_boundary_id.into(),
            baseline_method_id: baseline_method_id.into(),
            response_configuration_digest: response_configuration_digest.into(),
        };
        for (name, value) in [
            ("event ID", identity.event_id.as_str()),
            ("service product ID", identity.service_product_id.as_str()),
            ("measurement boundary ID", identity.measurement_boundary_id.as_str()),
            ("baseline method ID", identity.baseline_method_id.as_str()),
            ("response configuration digest", identity.response_configuration_digest.as_str()),
        ] {
            require_id(name, value)?;
        }
        Ok(identity)
    }

    pub fn event_id(&self) -> &str { &self.event_id }
    pub fn service_product_id(&self) -> &str { &self.service_product_id }
    pub fn measurement_boundary_id(&self) -> &str { &self.measurement_boundary_id }
    pub fn baseline_method_id(&self) -> &str { &self.baseline_method_id }
    pub fn response_configuration_digest(&self) -> &str {
        &self.response_configuration_digest
    }
}

/// Product-specific response envelope and sampling contract.
///
/// The response window begins when the instruction is issued. Samples are
/// interval-end readings at instruction_issued_at_ns + (index + 1) * interval.
/// A product whose allowed response latency is shorter than the telemetry
/// interval cannot be qualified at this sampling resolution and is rejected.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridResponseContract {
    identity: GridResponseIdentity,
    instruction_issued_at_ns: u64,
    service_end_ns: u64,
    sample_interval_ns: u64,
    target_reduction_kw: f64,
    minimum_delivery_fraction: f64,
    maximum_response_latency_ns: u64,
    minimum_compliant_sample_fraction: f64,
    maximum_underdelivery_energy_fraction: f64,
    maximum_overshoot_energy_fraction: f64,
    maximum_ramp_kw_per_second: f64,
}

impl GridResponseContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: GridResponseIdentity,
        instruction_issued_at_ns: u64,
        service_end_ns: u64,
        sample_interval_ns: u64,
        target_reduction_kw: f64,
        minimum_delivery_fraction: f64,
        maximum_response_latency_ns: u64,
        minimum_compliant_sample_fraction: f64,
        maximum_underdelivery_energy_fraction: f64,
        maximum_overshoot_energy_fraction: f64,
        maximum_ramp_kw_per_second: f64,
    ) -> Result<Self, GridResponseError> {
        if instruction_issued_at_ns >= service_end_ns {
            return Err(GridResponseError::InvalidWindow {
                instruction_issued_at_ns,
                service_end_ns,
            });
        }
        if sample_interval_ns == 0
            || (service_end_ns - instruction_issued_at_ns) % sample_interval_ns != 0
        {
            return Err(GridResponseError::InvalidSampleInterval { sample_interval_ns });
        }
        if maximum_response_latency_ns < sample_interval_ns {
            return Err(GridResponseError::TelemetryResolutionInsufficient {
                maximum_response_latency_ns,
                sample_interval_ns,
            });
        }
        if maximum_response_latency_ns > service_end_ns - instruction_issued_at_ns {
            return Err(GridResponseError::ResponseDeadlineOutsideWindow {
                maximum_response_latency_ns,
                service_duration_ns: service_end_ns - instruction_issued_at_ns,
            });
        }
        if !target_reduction_kw.is_finite() || target_reduction_kw <= 0.0 {
            return Err(GridResponseError::InvalidQuantity {
                name: "target reduction",
                value: target_reduction_kw,
            });
        }
        for (name, value) in [
            ("minimum delivery fraction", minimum_delivery_fraction),
            ("minimum compliant sample fraction", minimum_compliant_sample_fraction),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) || value == 0.0 {
                return Err(GridResponseError::InvalidFraction { name, value });
            }
        }
        for (name, value) in [
            ("maximum under-delivery energy fraction", maximum_underdelivery_energy_fraction),
            ("maximum overshoot energy fraction", maximum_overshoot_energy_fraction),
            ("maximum ramp", maximum_ramp_kw_per_second),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(GridResponseError::InvalidQuantity { name, value });
            }
        }
        if maximum_ramp_kw_per_second == 0.0 {
            return Err(GridResponseError::InvalidQuantity {
                name: "maximum ramp",
                value: maximum_ramp_kw_per_second,
            });
        }

        Ok(Self {
            identity,
            instruction_issued_at_ns,
            service_end_ns,
            sample_interval_ns,
            target_reduction_kw,
            minimum_delivery_fraction,
            maximum_response_latency_ns,
            minimum_compliant_sample_fraction,
            maximum_underdelivery_energy_fraction,
            maximum_overshoot_energy_fraction,
            maximum_ramp_kw_per_second,
        })
    }

    pub fn identity(&self) -> &GridResponseIdentity { &self.identity }
    pub const fn instruction_issued_at_ns(&self) -> u64 { self.instruction_issued_at_ns }
    pub const fn service_end_ns(&self) -> u64 { self.service_end_ns }
    pub const fn sample_interval_ns(&self) -> u64 { self.sample_interval_ns }
    pub const fn target_reduction_kw(&self) -> f64 { self.target_reduction_kw }
}

/// One interval-end PCC reading against its declared counterfactual baseline.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridResponseSample {
    elapsed_ns: u64,
    baseline_pcc_import_kw: f64,
    observed_response_pcc_import_kw: f64,
    baseline_evidence_id: String,
    response_evidence_id: String,
}

impl GridResponseSample {
    pub fn new(
        elapsed_ns: u64,
        baseline_pcc_import_kw: f64,
        observed_response_pcc_import_kw: f64,
        baseline_evidence_id: impl Into<String>,
        response_evidence_id: impl Into<String>,
    ) -> Result<Self, GridResponseError> {
        let sample = Self {
            elapsed_ns,
            baseline_pcc_import_kw,
            observed_response_pcc_import_kw,
            baseline_evidence_id: baseline_evidence_id.into(),
            response_evidence_id: response_evidence_id.into(),
        };
        require_id("baseline evidence ID", &sample.baseline_evidence_id)?;
        require_id("response evidence ID", &sample.response_evidence_id)?;
        if sample.baseline_evidence_id == sample.response_evidence_id {
            return Err(GridResponseError::EvidenceReferenceCollision { elapsed_ns });
        }
        for (name, value) in [
            ("baseline PCC import", sample.baseline_pcc_import_kw),
            ("observed response PCC import", sample.observed_response_pcc_import_kw),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(GridResponseError::InvalidQuantity { name, value });
            }
        }
        Ok(sample)
    }

    pub const fn elapsed_ns(&self) -> u64 { self.elapsed_ns }
    pub fn baseline_evidence_id(&self) -> &str { &self.baseline_evidence_id }
    pub fn response_evidence_id(&self) -> &str { &self.response_evidence_id }
}

/// Per-sample delivery, tracking-error and ramp receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridResponseSampleReceipt {
    pub elapsed_ns: u64,
    pub baseline_pcc_import_kw: f64,
    pub observed_response_pcc_import_kw: f64,
    /// Baseline minus measured response. Negative values are preserved.
    pub delivered_reduction_kw: f64,
    pub delivered_fraction: f64,
    pub interval_compliant: bool,
    pub underdelivery_kw: f64,
    pub overshoot_kw: f64,
    pub measured_ramp_kw_per_second: f64,
    pub baseline_evidence_id: String,
    pub response_evidence_id: String,
}

/// Full response-envelope report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridResponseReport {
    pub identity: GridResponseIdentity,
    pub instruction_issued_at_ns: u64,
    pub service_end_ns: u64,
    pub sample_interval_ns: u64,
    pub sample_count: usize,
    pub target_reduction_kw: f64,
    pub requested_reduction_energy_kwh: f64,
    /// Signed integral; negative values mean net import increased versus baseline.
    pub delivered_reduction_energy_kwh: f64,
    pub underdelivery_energy_kwh: f64,
    pub overshoot_energy_kwh: f64,
    pub delivered_energy_fraction: f64,
    pub underdelivery_energy_fraction: f64,
    pub overshoot_energy_fraction: f64,
    pub response_latency_ns: Option<u64>,
    pub maximum_response_latency_ns: u64,
    pub compliant_sample_count: usize,
    pub compliant_sample_fraction: f64,
    pub maximum_underdelivery_kw: f64,
    pub maximum_overshoot_kw: f64,
    pub maximum_measured_ramp_kw_per_second: f64,
    pub response_latency_pass: bool,
    pub sustained_delivery_pass: bool,
    pub underdelivery_energy_pass: bool,
    pub overshoot_energy_pass: bool,
    pub ramp_limit_pass: bool,
    pub passed: bool,
    /// Always true: ramp is only qualified at the declared sample interval.
    pub sub_sample_transients_unverified: bool,
    pub sample_receipts: Vec<GridResponseSampleReceipt>,
}

/// Invalid dispatch instructions, telemetry or response arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum GridResponseError {
    MissingIdentity { name: &'static str },
    InvalidWindow { instruction_issued_at_ns: u64, service_end_ns: u64 },
    InvalidSampleInterval { sample_interval_ns: u64 },
    TelemetryResolutionInsufficient {
        maximum_response_latency_ns: u64,
        sample_interval_ns: u64,
    },
    ResponseDeadlineOutsideWindow {
        maximum_response_latency_ns: u64,
        service_duration_ns: u64,
    },
    InvalidQuantity { name: &'static str, value: f64 },
    InvalidFraction { name: &'static str, value: f64 },
    EvidenceReferenceCollision { elapsed_ns: u64 },
    EmptySamples,
    SampleCountMismatch { expected: usize, observed: usize },
    SampleTimestampOverflow,
    SampleTimestampMismatch {
        index: usize,
        expected_elapsed_ns: u64,
        observed_elapsed_ns: u64,
    },
    NumericalOverflow(&'static str),
}

impl fmt::Display for GridResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::InvalidWindow { instruction_issued_at_ns, service_end_ns } => write!(
                f, "invalid dispatch window: instruction {instruction_issued_at_ns}, service end {service_end_ns}"
            ),
            Self::InvalidSampleInterval { sample_interval_ns } => write!(
                f, "sample interval {sample_interval_ns} ns is zero or does not divide the response window"
            ),
            Self::TelemetryResolutionInsufficient { maximum_response_latency_ns, sample_interval_ns } => write!(
                f, "telemetry interval {sample_interval_ns} ns cannot verify maximum response latency {maximum_response_latency_ns} ns"
            ),
            Self::ResponseDeadlineOutsideWindow { maximum_response_latency_ns, service_duration_ns } => write!(
                f, "response latency limit {maximum_response_latency_ns} ns exceeds service duration {service_duration_ns} ns"
            ),
            Self::InvalidQuantity { name, value } => write!(
                f, "{name} must be finite and within its non-negative range; got {value}"
            ),
            Self::InvalidFraction { name, value } => write!(f, "{name} must be in (0, 1]; got {value}"),
            Self::EvidenceReferenceCollision { elapsed_ns } => write!(
                f, "baseline and response evidence IDs collide at sample {elapsed_ns}"
            ),
            Self::EmptySamples => write!(f, "grid response evaluation requires telemetry samples"),
            Self::SampleCountMismatch { expected, observed } => write!(
                f, "grid response sample count mismatch: expected {expected}, observed {observed}"
            ),
            Self::SampleTimestampOverflow => write!(f, "grid response timestamp arithmetic overflow"),
            Self::SampleTimestampMismatch { index, expected_elapsed_ns, observed_elapsed_ns } => write!(
                f, "grid response sample {index} timestamp mismatch: expected {expected_elapsed_ns}, observed {observed_elapsed_ns}"
            ),
            Self::NumericalOverflow(operation) => write!(
                f, "non-finite or overflowing arithmetic during {operation}"
            ),
        }
    }
}

impl std::error::Error for GridResponseError {}

/// Verify dispatch response against one frozen product-specific envelope.
pub fn evaluate_grid_response(
    contract: &GridResponseContract,
    samples: &[GridResponseSample],
) -> Result<GridResponseReport, GridResponseError> {
    if samples.is_empty() {
        return Err(GridResponseError::EmptySamples);
    }
    let duration_ns = contract.service_end_ns - contract.instruction_issued_at_ns;
    let expected_samples = usize::try_from(duration_ns / contract.sample_interval_ns)
        .map_err(|_| GridResponseError::NumericalOverflow("expected sample count"))?;
    if samples.len() != expected_samples {
        return Err(GridResponseError::SampleCountMismatch {
            expected: expected_samples,
            observed: samples.len(),
        });
    }

    let interval_seconds = contract.sample_interval_ns as f64 / 1_000_000_000.0;
    let interval_hours = contract.sample_interval_ns as f64 / 3_600_000_000_000.0;
    if !interval_seconds.is_finite() || interval_seconds <= 0.0
        || !interval_hours.is_finite() || interval_hours <= 0.0
    {
        return Err(GridResponseError::NumericalOverflow("telemetry interval conversion"));
    }

    let requested_reduction_energy_kwh =
        contract.target_reduction_kw * interval_hours * expected_samples as f64;
    if !requested_reduction_energy_kwh.is_finite() || requested_reduction_energy_kwh <= 0.0 {
        return Err(GridResponseError::NumericalOverflow("requested reduction energy"));
    }

    let minimum_delivered_kw =
        contract.target_reduction_kw * contract.minimum_delivery_fraction;
    let mut delivered_reduction_energy_kwh = 0.0_f64;
    let mut underdelivery_energy_kwh = 0.0_f64;
    let mut overshoot_energy_kwh = 0.0_f64;
    let mut response_latency_ns = None;
    let mut compliant_sample_count = 0_usize;
    let mut maximum_underdelivery_kw = 0.0_f64;
    let mut maximum_overshoot_kw = 0.0_f64;
    let mut maximum_measured_ramp_kw_per_second = 0.0_f64;
    let mut previous_delivered_reduction_kw = 0.0_f64;
    let mut sample_receipts = Vec::with_capacity(samples.len());

    for (index, sample) in samples.iter().enumerate() {
        let sample_number = u64::try_from(index + 1)
            .map_err(|_| GridResponseError::SampleTimestampOverflow)?;
        let interval_offset = sample_number
            .checked_mul(contract.sample_interval_ns)
            .ok_or(GridResponseError::SampleTimestampOverflow)?;
        let expected_elapsed_ns = contract.instruction_issued_at_ns
            .checked_add(interval_offset)
            .ok_or(GridResponseError::SampleTimestampOverflow)?;
        if sample.elapsed_ns != expected_elapsed_ns {
            return Err(GridResponseError::SampleTimestampMismatch {
                index,
                expected_elapsed_ns,
                observed_elapsed_ns: sample.elapsed_ns,
            });
        }

        let delivered_reduction_kw =
            sample.baseline_pcc_import_kw - sample.observed_response_pcc_import_kw;
        let delivered_fraction = delivered_reduction_kw / contract.target_reduction_kw;
        let underdelivery_kw =
            (contract.target_reduction_kw - delivered_reduction_kw).max(0.0);
        let overshoot_kw =
            (delivered_reduction_kw - contract.target_reduction_kw).max(0.0);
        let measured_ramp_kw_per_second =
            (delivered_reduction_kw - previous_delivered_reduction_kw).abs() / interval_seconds;
        for (name, value) in [
            ("delivered reduction", delivered_reduction_kw),
            ("delivered fraction", delivered_fraction),
            ("underdelivery", underdelivery_kw),
            ("overshoot", overshoot_kw),
            ("measured ramp", measured_ramp_kw_per_second),
        ] {
            if !value.is_finite() {
                return Err(GridResponseError::NumericalOverflow(name));
            }
        }

        let interval_compliant = delivered_reduction_kw >= minimum_delivered_kw;
        if interval_compliant {
            compliant_sample_count += 1;
        }
        if response_latency_ns.is_none() && interval_compliant {
            response_latency_ns = Some(sample.elapsed_ns - contract.instruction_issued_at_ns);
        }

        delivered_reduction_energy_kwh += delivered_reduction_kw * interval_hours;
        underdelivery_energy_kwh += underdelivery_kw * interval_hours;
        overshoot_energy_kwh += overshoot_kw * interval_hours;
        maximum_underdelivery_kw = maximum_underdelivery_kw.max(underdelivery_kw);
        maximum_overshoot_kw = maximum_overshoot_kw.max(overshoot_kw);
        maximum_measured_ramp_kw_per_second =
            maximum_measured_ramp_kw_per_second.max(measured_ramp_kw_per_second);
        for (name, value) in [
            ("delivered reduction energy", delivered_reduction_energy_kwh),
            ("underdelivery energy", underdelivery_energy_kwh),
            ("overshoot energy", overshoot_energy_kwh),
        ] {
            if !value.is_finite() {
                return Err(GridResponseError::NumericalOverflow(name));
            }
        }

        sample_receipts.push(GridResponseSampleReceipt {
            elapsed_ns: sample.elapsed_ns,
            baseline_pcc_import_kw: sample.baseline_pcc_import_kw,
            observed_response_pcc_import_kw: sample.observed_response_pcc_import_kw,
            delivered_reduction_kw,
            delivered_fraction,
            interval_compliant,
            underdelivery_kw,
            overshoot_kw,
            measured_ramp_kw_per_second,
            baseline_evidence_id: sample.baseline_evidence_id.clone(),
            response_evidence_id: sample.response_evidence_id.clone(),
        });
        previous_delivered_reduction_kw = delivered_reduction_kw;
    }

    let sample_count = samples.len();
    let compliant_sample_fraction = compliant_sample_count as f64 / sample_count as f64;
    let delivered_energy_fraction =
        delivered_reduction_energy_kwh / requested_reduction_energy_kwh;
    let underdelivery_energy_fraction =
        underdelivery_energy_kwh / requested_reduction_energy_kwh;
    let overshoot_energy_fraction =
        overshoot_energy_kwh / requested_reduction_energy_kwh;
    for (name, value) in [
        ("compliant sample fraction", compliant_sample_fraction),
        ("delivered energy fraction", delivered_energy_fraction),
        ("underdelivery energy fraction", underdelivery_energy_fraction),
        ("overshoot energy fraction", overshoot_energy_fraction),
    ] {
        if !value.is_finite() {
            return Err(GridResponseError::NumericalOverflow(name));
        }
    }

    let response_latency_pass = response_latency_ns
        .is_some_and(|latency| latency <= contract.maximum_response_latency_ns);
    let sustained_delivery_pass =
        compliant_sample_fraction >= contract.minimum_compliant_sample_fraction;
    let underdelivery_energy_pass =
        underdelivery_energy_fraction <= contract.maximum_underdelivery_energy_fraction;
    let overshoot_energy_pass =
        overshoot_energy_fraction <= contract.maximum_overshoot_energy_fraction;
    let ramp_limit_pass =
        maximum_measured_ramp_kw_per_second <= contract.maximum_ramp_kw_per_second;
    let passed = response_latency_pass && sustained_delivery_pass
        && underdelivery_energy_pass && overshoot_energy_pass && ramp_limit_pass;

    Ok(GridResponseReport {
        identity: contract.identity.clone(),
        instruction_issued_at_ns: contract.instruction_issued_at_ns,
        service_end_ns: contract.service_end_ns,
        sample_interval_ns: contract.sample_interval_ns,
        sample_count,
        target_reduction_kw: contract.target_reduction_kw,
        requested_reduction_energy_kwh,
        delivered_reduction_energy_kwh,
        underdelivery_energy_kwh,
        overshoot_energy_kwh,
        delivered_energy_fraction,
        underdelivery_energy_fraction,
        overshoot_energy_fraction,
        response_latency_ns,
        maximum_response_latency_ns: contract.maximum_response_latency_ns,
        compliant_sample_count,
        compliant_sample_fraction,
        maximum_underdelivery_kw,
        maximum_overshoot_kw,
        maximum_measured_ramp_kw_per_second,
        response_latency_pass,
        sustained_delivery_pass,
        underdelivery_energy_pass,
        overshoot_energy_pass,
        ramp_limit_pass,
        passed,
        sub_sample_transients_unverified: true,
        sample_receipts,
    })
}

fn require_id(name: &'static str, value: &str) -> Result<(), GridResponseError> {
    if value.trim().is_empty() {
        return Err(GridResponseError::MissingIdentity { name });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND_NS: u64 = 1_000_000_000;

    fn identity() -> GridResponseIdentity {
        GridResponseIdentity::new(
            "dispatch-event-001",
            "product-fast-response-v1",
            "site-pcc-001",
            "baseline-method-digest",
            "response-configuration-digest",
        ).unwrap()
    }

    fn contract() -> GridResponseContract {
        GridResponseContract::new(
            identity(), 0, 4 * SECOND_NS, SECOND_NS, 100.0, 0.90,
            2 * SECOND_NS, 0.75, 0.10, 0.05, 150.0,
        ).unwrap()
    }

    fn samples(reductions_kw: [f64; 4]) -> Vec<GridResponseSample> {
        reductions_kw.into_iter().enumerate().map(|(index, reduction)| {
            GridResponseSample::new(
                (index as u64 + 1) * SECOND_NS,
                1_000.0,
                1_000.0 - reduction,
                format!("baseline-evidence-{index}"),
                format!("response-evidence-{index}"),
            ).unwrap()
        }).collect()
    }

    #[test]
    fn fast_sustained_response_passes_with_exact_known_truth_metrics() {
        let report =
            evaluate_grid_response(&contract(), &samples([100.0, 100.0, 100.0, 100.0])).unwrap();
        assert!(report.passed);
        assert_eq!(report.response_latency_ns, Some(SECOND_NS));
        assert_eq!(report.compliant_sample_count, 4);
        assert!((report.compliant_sample_fraction - 1.0).abs() < 1e-12);
        assert!((report.requested_reduction_energy_kwh - 100.0 * 4.0 / 3_600.0).abs() < 1e-12);
        assert!((report.delivered_reduction_energy_kwh
            - report.requested_reduction_energy_kwh).abs() < 1e-12);
        assert!(report.sub_sample_transients_unverified);
    }

    #[test]
    fn event_average_reduction_does_not_hide_late_initial_response() {
        let report =
            evaluate_grid_response(&contract(), &samples([0.0, 100.0, 100.0, 100.0])).unwrap();
        assert!((report.delivered_energy_fraction - 0.75).abs() < 1e-12);
        assert_eq!(report.response_latency_ns, Some(2 * SECOND_NS));
        assert!(report.response_latency_pass);
        assert!(!report.underdelivery_energy_pass);
        assert!(!report.passed);
    }

    #[test]
    fn rejects_slow_response_even_when_later_delivery_is_perfect() {
        let mut slow_contract = contract();
        slow_contract.maximum_response_latency_ns = SECOND_NS;
        let report =
            evaluate_grid_response(&slow_contract, &samples([0.0, 100.0, 100.0, 100.0])).unwrap();
        assert!(!report.response_latency_pass);
        assert!(!report.passed);
    }

    #[test]
    fn fast_initial_response_followed_by_under_delivery_fails_sustained_gate() {
        let report =
            evaluate_grid_response(&contract(), &samples([100.0, 40.0, 40.0, 100.0])).unwrap();
        assert_eq!(report.response_latency_ns, Some(SECOND_NS));
        assert!(!report.sustained_delivery_pass);
        assert!(report.underdelivery_energy_fraction > 0.0);
        assert!(!report.passed);
    }

    #[test]
    fn excessive_overshoot_is_reported_and_gated() {
        let report =
            evaluate_grid_response(&contract(), &samples([120.0, 120.0, 120.0, 120.0])).unwrap();
        assert!(report.overshoot_energy_fraction > 0.0);
        assert!(!report.overshoot_energy_pass);
        assert!(!report.passed);
    }

    #[test]
    fn excessive_ramp_is_detected_even_if_average_delivery_passes() {
        let mut ramp_contract = contract();
        ramp_contract.maximum_ramp_kw_per_second = 50.0;
        let report =
            evaluate_grid_response(&ramp_contract, &samples([100.0, 100.0, 100.0, 100.0])).unwrap();
        assert_eq!(report.compliant_sample_fraction, 1.0);
        assert!(!report.ramp_limit_pass);
        assert!(!report.passed);
    }

    #[test]
    fn increased_grid_import_is_preserved_as_negative_delivery() {
        let report =
            evaluate_grid_response(&contract(), &samples([-20.0, -20.0, -20.0, -20.0])).unwrap();
        assert!(report.delivered_reduction_energy_kwh < 0.0);
        assert!(report.underdelivery_energy_fraction > 1.0);
        assert_eq!(report.response_latency_ns, None);
        assert!(!report.response_latency_pass);
    }

    #[test]
    fn rejects_missing_or_misaligned_sample_chronology() {
        let mut input = samples([100.0, 100.0, 100.0, 100.0]);
        input[1].elapsed_ns += 1;
        assert!(matches!(
            evaluate_grid_response(&contract(), &input),
            Err(GridResponseError::SampleTimestampMismatch { .. })
        ));
        assert!(matches!(
            evaluate_grid_response(&contract(), &input[..3]),
            Err(GridResponseError::SampleCountMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_evidence_and_evidence_reference_collision() {
        assert!(matches!(
            GridResponseSample::new(SECOND_NS, 1_000.0, 900.0, "", "response"),
            Err(GridResponseError::MissingIdentity { .. })
        ));
        assert!(matches!(
            GridResponseSample::new(
                SECOND_NS, 1_000.0, 900.0, "same-evidence", "same-evidence",
            ),
            Err(GridResponseError::EvidenceReferenceCollision { .. })
        ));
    }

    #[test]
    fn rejects_telemetry_resolution_that_cannot_prove_product_latency() {
        assert!(matches!(
            GridResponseContract::new(
                identity(), 0, 4 * SECOND_NS, 2 * SECOND_NS, 100.0, 0.90,
                SECOND_NS, 0.75, 0.10, 0.05, 150.0,
            ),
            Err(GridResponseError::TelemetryResolutionInsufficient { .. })
        ));
    }

    #[test]
    fn rejects_invalid_contracts_and_non_finite_samples() {
        assert!(GridResponseIdentity::new("", "product", "pcc", "baseline", "config").is_err());
        assert!(GridResponseSample::new(
            SECOND_NS, f64::NAN, 900.0, "baseline", "response",
        ).is_err());
        assert!(GridResponseContract::new(
            identity(), 0, 4 * SECOND_NS, SECOND_NS, 0.0, 0.9,
            SECOND_NS, 0.75, 0.1, 0.05, 150.0,
        ).is_err());
        assert!(GridResponseContract::new(
            identity(), 0, 4 * SECOND_NS, SECOND_NS, 100.0, 1.1,
            SECOND_NS, 0.75, 0.1, 0.05, 150.0,
        ).is_err());
        assert!(GridResponseContract::new(
            identity(), 0, 4 * SECOND_NS, SECOND_NS, 100.0, 0.0,
            SECOND_NS, 0.75, 0.1, 0.05, 150.0,
        ).is_err());
        assert!(GridResponseContract::new(
            identity(), 0, 4 * SECOND_NS, SECOND_NS, 100.0, 0.9,
            SECOND_NS, 0.0, 0.1, 0.05, 150.0,
        ).is_err());
    }

    #[test]
    fn deterministic_replay_preserves_all_sample_receipts() {
        let input = samples([100.0, 95.0, 105.0, 100.0]);
        let first = evaluate_grid_response(&contract(), &input).unwrap();
        let second = evaluate_grid_response(&contract(), &input).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.sample_receipts.len(), 4);
        assert_eq!(first.sample_receipts[0].baseline_evidence_id, "baseline-evidence-0");
    }
}
