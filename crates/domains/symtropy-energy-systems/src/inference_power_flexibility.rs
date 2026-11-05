// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paired inference power-flexibility and service-quality envelope.
//!
//! This module evaluates measured control sweeps. It is not a GPU controller,
//! workload scheduler, facility simulator, or grid dispatch interface.
//!
//! The primary boundary power is measured explicitly. GPU, CPU and DRAM power
//! are retained as disaggregated diagnostics and MUST NOT be summed into the
//! declared boundary reading unless the meter definition says they are
//! disjoint. Tokens/J and controllable kW are different outcomes.

use core::fmt;

use serde::Serialize;

/// Power boundary at which the paired counterfactual is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InferenceMeasurementBoundary {
    AcceleratorNode,
    ServerInlet,
    RackPdu,
    FacilityMeter,
    PointOfCommonCoupling,
}

/// Frozen identity for the matched baseline/candidate inference experiment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InferenceFlexibilityIdentity {
    study_id: String,
    baseline_run_id: String,
    candidate_run_id: String,
    hardware_id: String,
    workload_id: String,
    model_digest: String,
    runtime_digest: String,
    request_manifest_digest: String,
    condition_stratum_id: String,
    measurement_boundary_id: String,
    meter_calibration_id: String,
    boundary: InferenceMeasurementBoundary,
}

impl InferenceFlexibilityIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        study_id: impl Into<String>,
        baseline_run_id: impl Into<String>,
        candidate_run_id: impl Into<String>,
        hardware_id: impl Into<String>,
        workload_id: impl Into<String>,
        model_digest: impl Into<String>,
        runtime_digest: impl Into<String>,
        request_manifest_digest: impl Into<String>,
        condition_stratum_id: impl Into<String>,
        measurement_boundary_id: impl Into<String>,
        meter_calibration_id: impl Into<String>,
        boundary: InferenceMeasurementBoundary,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        let value = Self {
            study_id: study_id.into(),
            baseline_run_id: baseline_run_id.into(),
            candidate_run_id: candidate_run_id.into(),
            hardware_id: hardware_id.into(),
            workload_id: workload_id.into(),
            model_digest: model_digest.into(),
            runtime_digest: runtime_digest.into(),
            request_manifest_digest: request_manifest_digest.into(),
            condition_stratum_id: condition_stratum_id.into(),
            measurement_boundary_id: measurement_boundary_id.into(),
            meter_calibration_id: meter_calibration_id.into(),
            boundary,
        };
        for (name, id) in [
            ("study ID", value.study_id.as_str()),
            ("baseline run ID", value.baseline_run_id.as_str()),
            ("candidate run ID", value.candidate_run_id.as_str()),
            ("hardware ID", value.hardware_id.as_str()),
            ("workload ID", value.workload_id.as_str()),
            ("model digest", value.model_digest.as_str()),
            ("runtime digest", value.runtime_digest.as_str()),
            ("request manifest digest", value.request_manifest_digest.as_str()),
            ("condition stratum ID", value.condition_stratum_id.as_str()),
            ("measurement boundary ID", value.measurement_boundary_id.as_str()),
            ("meter calibration ID", value.meter_calibration_id.as_str()),
        ] {
            require_id(name, id)?;
        }
        if value.baseline_run_id == value.candidate_run_id {
            return Err(InferencePowerFlexibilityError::RunIdsMustDiffer);
        }
        Ok(value)
    }

    pub fn study_id(&self) -> &str { &self.study_id }
    pub fn baseline_run_id(&self) -> &str { &self.baseline_run_id }
    pub fn candidate_run_id(&self) -> &str { &self.candidate_run_id }
    pub fn hardware_id(&self) -> &str { &self.hardware_id }
    pub fn workload_id(&self) -> &str { &self.workload_id }
    pub fn model_digest(&self) -> &str { &self.model_digest }
    pub fn runtime_digest(&self) -> &str { &self.runtime_digest }
    pub fn request_manifest_digest(&self) -> &str { &self.request_manifest_digest }
    pub fn condition_stratum_id(&self) -> &str { &self.condition_stratum_id }
    pub fn measurement_boundary_id(&self) -> &str { &self.measurement_boundary_id }
    pub fn meter_calibration_id(&self) -> &str { &self.meter_calibration_id }
    pub const fn boundary(&self) -> InferenceMeasurementBoundary { self.boundary }
}

/// Control configuration recorded for one measured run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceControlAction {
    action_id: String,
    gpu_clock_limit_mhz: Option<f64>,
    gpu_power_cap_w: Option<f64>,
    cpu_power_cap_w: Option<f64>,
    maximum_batch_size: Option<u32>,
    maximum_concurrency: Option<u32>,
}

impl InferenceControlAction {
    pub fn new(
        action_id: impl Into<String>,
        gpu_clock_limit_mhz: Option<f64>,
        gpu_power_cap_w: Option<f64>,
        cpu_power_cap_w: Option<f64>,
        maximum_batch_size: Option<u32>,
        maximum_concurrency: Option<u32>,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        let action = Self {
            action_id: action_id.into(),
            gpu_clock_limit_mhz,
            gpu_power_cap_w,
            cpu_power_cap_w,
            maximum_batch_size,
            maximum_concurrency,
        };
        require_id("control action ID", &action.action_id)?;
        if [
            gpu_clock_limit_mhz,
            gpu_power_cap_w,
            cpu_power_cap_w,
        ].iter().flatten().any(|value| !value.is_finite() || *value <= 0.0)
            || maximum_batch_size == Some(0)
            || maximum_concurrency == Some(0)
            || (gpu_clock_limit_mhz.is_none()
                && gpu_power_cap_w.is_none()
                && cpu_power_cap_w.is_none()
                && maximum_batch_size.is_none()
                && maximum_concurrency.is_none())
        {
            return Err(InferencePowerFlexibilityError::InvalidControlAction);
        }
        Ok(action)
    }

    pub fn action_id(&self) -> &str { &self.action_id }
}

/// One request in the frozen request manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InferenceRequestObligation {
    request_id: String,
    expected_output_tokens: u64,
}

impl InferenceRequestObligation {
    pub fn new(
        request_id: impl Into<String>,
        expected_output_tokens: u64,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        let value = Self { request_id: request_id.into(), expected_output_tokens };
        require_id("request ID", &value.request_id)?;
        if expected_output_tokens == 0 {
            return Err(InferencePowerFlexibilityError::InvalidRequestObligation);
        }
        Ok(value)
    }

    pub fn request_id(&self) -> &str { &self.request_id }
    pub const fn expected_output_tokens(&self) -> u64 { self.expected_output_tokens }
}

/// Measured result for one request. Timing values are milliseconds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceRequestResult {
    request_id: String,
    succeeded: bool,
    output_tokens: u64,
    time_to_first_token_ms: Option<f64>,
    completion_latency_ms: Option<f64>,
}

impl InferenceRequestResult {
    pub fn new(
        request_id: impl Into<String>,
        succeeded: bool,
        output_tokens: u64,
        time_to_first_token_ms: Option<f64>,
        completion_latency_ms: Option<f64>,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        let value = Self {
            request_id: request_id.into(),
            succeeded,
            output_tokens,
            time_to_first_token_ms,
            completion_latency_ms,
        };
        require_id("request result ID", &value.request_id)?;
        for (name, timing) in [
            ("time to first token ms", time_to_first_token_ms),
            ("completion latency ms", completion_latency_ms),
        ] {
            if timing.is_some_and(|v| !v.is_finite() || v < 0.0) {
                return Err(InferencePowerFlexibilityError::InvalidRequestTiming { name });
            }
        }
        if succeeded
            && (output_tokens == 0
                || time_to_first_token_ms.is_none()
                || completion_latency_ms.is_none())
        {
            return Err(InferencePowerFlexibilityError::InvalidSuccessfulRequest);
        }
        if let (Some(ttft), Some(latency)) = (time_to_first_token_ms, completion_latency_ms) {
            if ttft > latency {
                return Err(InferencePowerFlexibilityError::InvalidRequestTiming {
                    name: "TTFT exceeds completion latency",
                });
            }
        }
        Ok(value)
    }

    pub fn request_id(&self) -> &str { &self.request_id }
}

/// One aligned telemetry interval. All power values are interval-average W.
/// boundary_power_w is the authoritative primary meter at the identity's
/// declared boundary; disaggregated GPU/CPU/DRAM channels are not added to it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferencePowerInterval {
    start_elapsed_ms: u64,
    duration_ms: u64,
    boundary_power_w: f64,
    gpu_power_w: f64,
    cpu_power_w: f64,
    dram_power_w: f64,
    gpu_clock_mhz: f64,
    gpu_utilization_fraction: f64,
    cpu_utilization_fraction: f64,
    gpu_temperature_c: f64,
    dram_temperature_c: f64,
}

impl InferencePowerInterval {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start_elapsed_ms: u64,
        duration_ms: u64,
        boundary_power_w: f64,
        gpu_power_w: f64,
        cpu_power_w: f64,
        dram_power_w: f64,
        gpu_clock_mhz: f64,
        gpu_utilization_fraction: f64,
        cpu_utilization_fraction: f64,
        gpu_temperature_c: f64,
        dram_temperature_c: f64,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        let value = Self {
            start_elapsed_ms,
            duration_ms,
            boundary_power_w,
            gpu_power_w,
            cpu_power_w,
            dram_power_w,
            gpu_clock_mhz,
            gpu_utilization_fraction,
            cpu_utilization_fraction,
            gpu_temperature_c,
            dram_temperature_c,
        };
        if duration_ms == 0 {
            return Err(InferencePowerFlexibilityError::InvalidPowerInterval);
        }
        for (name, power) in [
            ("boundary power W", boundary_power_w),
            ("GPU power W", gpu_power_w),
            ("CPU power W", cpu_power_w),
            ("DRAM power W", dram_power_w),
            ("GPU clock MHz", gpu_clock_mhz),
        ] {
            if !power.is_finite() || power < 0.0 {
                return Err(InferencePowerFlexibilityError::InvalidPowerQuantity { name, value: power });
            }
        }
        for (name, fraction) in [
            ("GPU utilization fraction", gpu_utilization_fraction),
            ("CPU utilization fraction", cpu_utilization_fraction),
        ] {
            if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
                return Err(InferencePowerFlexibilityError::InvalidUtilization { name, value: fraction });
            }
        }
        for (name, temperature) in [
            ("GPU temperature C", gpu_temperature_c),
            ("DRAM temperature C", dram_temperature_c),
        ] {
            if !temperature.is_finite() {
                return Err(InferencePowerFlexibilityError::InvalidPowerQuantity { name, value: temperature });
            }
        }
        start_elapsed_ms.checked_add(duration_ms)
            .ok_or(InferencePowerFlexibilityError::NumericalOverflow("power interval end time"))?;
        Ok(value)
    }
}

/// A complete matched inference run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceRun {
    run_id: String,
    control_action: InferenceControlAction,
    request_results: Vec<InferenceRequestResult>,
    power_intervals: Vec<InferencePowerInterval>,
}

impl InferenceRun {
    pub fn new(
        run_id: impl Into<String>,
        control_action: InferenceControlAction,
        request_results: Vec<InferenceRequestResult>,
        power_intervals: Vec<InferencePowerInterval>,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        let value = Self {
            run_id: run_id.into(),
            control_action,
            request_results,
            power_intervals,
        };
        require_id("run ID", &value.run_id)?;
        if value.request_results.is_empty() || value.power_intervals.is_empty() {
            return Err(InferencePowerFlexibilityError::EmptyRun);
        }
        Ok(value)
    }

    pub fn run_id(&self) -> &str { &self.run_id }
}

/// Hard service thresholds. There is no energy-vs-quality blended score.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceServiceContract {
    minimum_success_fraction: f64,
    minimum_useful_work_fraction: f64,
    minimum_useful_tokens_per_second: f64,
    minimum_throughput_retention_fraction: f64,
    maximum_p95_ttft_ms: f64,
    maximum_p99_ttft_ms: f64,
    maximum_p95_completion_latency_ms: f64,
    maximum_p99_completion_latency_ms: f64,
    maximum_p99_ttft_degradation_fraction: f64,
    maximum_p99_latency_degradation_fraction: f64,
}

impl InferenceServiceContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        minimum_success_fraction: f64,
        minimum_useful_work_fraction: f64,
        minimum_useful_tokens_per_second: f64,
        minimum_throughput_retention_fraction: f64,
        maximum_p95_ttft_ms: f64,
        maximum_p99_ttft_ms: f64,
        maximum_p95_completion_latency_ms: f64,
        maximum_p99_completion_latency_ms: f64,
        maximum_p99_ttft_degradation_fraction: f64,
        maximum_p99_latency_degradation_fraction: f64,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        for (name, value) in [
            ("minimum success fraction", minimum_success_fraction),
            ("minimum useful-work fraction", minimum_useful_work_fraction),
            ("minimum throughput retention fraction", minimum_throughput_retention_fraction),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(InferencePowerFlexibilityError::InvalidServiceFraction { name, value });
            }
        }
        for (name, value) in [
            ("minimum tokens per second", minimum_useful_tokens_per_second),
            ("maximum p95 TTFT ms", maximum_p95_ttft_ms),
            ("maximum p99 TTFT ms", maximum_p99_ttft_ms),
            ("maximum p95 latency ms", maximum_p95_completion_latency_ms),
            ("maximum p99 latency ms", maximum_p99_completion_latency_ms),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(InferencePowerFlexibilityError::InvalidServiceThreshold { name, value });
            }
        }
        for (name, value) in [
            ("maximum p99 TTFT degradation fraction", maximum_p99_ttft_degradation_fraction),
            ("maximum p99 latency degradation fraction", maximum_p99_latency_degradation_fraction),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(InferencePowerFlexibilityError::InvalidServiceThreshold { name, value });
            }
        }
        Ok(Self {
            minimum_success_fraction,
            minimum_useful_work_fraction,
            minimum_useful_tokens_per_second,
            minimum_throughput_retention_fraction,
            maximum_p95_ttft_ms,
            maximum_p99_ttft_ms,
            maximum_p95_completion_latency_ms,
            maximum_p99_completion_latency_ms,
            maximum_p99_ttft_degradation_fraction,
            maximum_p99_latency_degradation_fraction,
        })
    }
}

/// Power-response qualification policy applied to the paired measured trace.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferencePowerResponseContract {
    minimum_sustained_reduction_w: f64,
    required_consecutive_samples: usize,
    minimum_sustained_duration_ms: u64,
    maximum_rebound_power_w: f64,
    maximum_rebound_energy_wh: f64,
}

impl InferencePowerResponseContract {
    pub fn new(
        minimum_sustained_reduction_w: f64,
        required_consecutive_samples: usize,
        minimum_sustained_duration_ms: u64,
        maximum_rebound_power_w: f64,
        maximum_rebound_energy_wh: f64,
    ) -> Result<Self, InferencePowerFlexibilityError> {
        if !minimum_sustained_reduction_w.is_finite() || minimum_sustained_reduction_w <= 0.0
            || !(1..=10_000).contains(&required_consecutive_samples)
            || minimum_sustained_duration_ms == 0
            || !maximum_rebound_power_w.is_finite()
            || maximum_rebound_power_w < 0.0
            || !maximum_rebound_energy_wh.is_finite()
            || maximum_rebound_energy_wh < 0.0
        {
            return Err(InferencePowerFlexibilityError::InvalidPowerResponseContract);
        }
        Ok(Self {
            minimum_sustained_reduction_w,
            required_consecutive_samples,
            minimum_sustained_duration_ms,
            maximum_rebound_power_w,
            maximum_rebound_energy_wh,
        })
    }
}

/// Combined paired power/service evaluation contract.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferencePowerFlexibilityContract {
    service: InferenceServiceContract,
    response: InferencePowerResponseContract,
}

impl InferencePowerFlexibilityContract {
    pub const fn new(
        service: InferenceServiceContract,
        response: InferencePowerResponseContract,
    ) -> Self {
        Self { service, response }
    }
}

/// Per-request and aggregate service quality metrics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferenceServiceMetrics {
    pub request_count: usize,
    pub successful_request_count: usize,
    pub success_fraction: f64,
    pub useful_work_fraction: f64,
    pub completed_useful_tokens: u64,
    pub total_output_tokens: u64,
    pub useful_tokens_per_second: f64,
    pub tokens_per_second: f64,
    pub p95_ttft_ms: Option<f64>,
    pub p99_ttft_ms: Option<f64>,
    pub p95_completion_latency_ms: Option<f64>,
    pub p99_completion_latency_ms: Option<f64>,
}

/// Slot-level power evidence at the declared primary measurement boundary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferencePowerSlotReceipt {
    pub start_elapsed_ms: u64,
    pub duration_ms: u64,
    pub baseline_boundary_power_w: f64,
    pub candidate_boundary_power_w: f64,
    /// Positive means reduced import; negative means increased import/rebound.
    pub signed_power_reduction_w: f64,
    pub signed_energy_reduction_wh: f64,
    pub baseline_gpu_power_w: f64,
    pub candidate_gpu_power_w: f64,
    pub baseline_cpu_power_w: f64,
    pub candidate_cpu_power_w: f64,
    pub baseline_dram_power_w: f64,
    pub candidate_dram_power_w: f64,
}

/// Reasons a candidate fails the declared service envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
/// Recovery/rebound envelope failures are distinct from service failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InferencePowerResponseGateViolation {
    MaximumReboundPowerExceeded,
    MaximumReboundEnergyExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InferenceServiceGateViolation {
    SuccessFractionBelowMinimum,
    UsefulWorkFractionBelowMinimum,
    TokensPerSecondBelowMinimum,
    ThroughputRetentionBelowMinimum,
    P95TtftUnavailable,
    P99TtftUnavailable,
    P95LatencyUnavailable,
    P99LatencyUnavailable,
    P95TtftAboveMaximum,
    P99TtftAboveMaximum,
    P95LatencyAboveMaximum,
    P99LatencyAboveMaximum,
    P99TtftRegressionExceeded,
    P99LatencyRegressionExceeded,
}

/// Auditable service-qualified power-flexibility result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InferencePowerFlexibilityReport {
    pub identity: InferenceFlexibilityIdentity,
    pub baseline_control_action: InferenceControlAction,
    pub candidate_control_action: InferenceControlAction,
    pub baseline_service: InferenceServiceMetrics,
    pub candidate_service: InferenceServiceMetrics,
    pub baseline_boundary_energy_wh: f64,
    pub candidate_boundary_energy_wh: f64,
    pub baseline_gpu_energy_wh: f64,
    pub candidate_gpu_energy_wh: f64,
    pub baseline_cpu_energy_wh: f64,
    pub candidate_cpu_energy_wh: f64,
    pub baseline_dram_energy_wh: f64,
    pub candidate_dram_energy_wh: f64,
    pub baseline_tokens_per_joule: f64,
    pub candidate_tokens_per_joule: f64,
    pub baseline_average_boundary_power_w: f64,
    pub candidate_average_boundary_power_w: f64,
    pub maximum_sampled_reduction_w: f64,
    pub maximum_sampled_rebound_w: f64,
    pub maximum_sustained_reduction_duration_ms: u64,
    pub gross_reduction_energy_wh: f64,
    pub rebound_energy_wh: f64,
    pub net_boundary_energy_reduction_wh: f64,
    pub stabilized_response_elapsed_ms: Option<u64>,
    pub candidate_service_gate_violations: Vec<InferenceServiceGateViolation>,
    pub power_response_gate_violations: Vec<InferencePowerResponseGateViolation>,
    pub service_equivalent: bool,
    pub power_response_qualified: bool,
    pub slot_receipts: Vec<InferencePowerSlotReceipt>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferencePowerFlexibilityError {
    MissingIdentity { name: &'static str },
    IdentityTooLong { name: &'static str },
    RunIdsMustDiffer,
    InvalidControlAction,
    InvalidRequestObligation,
    InvalidRequestTiming { name: &'static str },
    InvalidSuccessfulRequest,
    InvalidPowerInterval,
    InvalidPowerQuantity { name: &'static str, value: f64 },
    InvalidUtilization { name: &'static str, value: f64 },
    InvalidServiceFraction { name: &'static str, value: f64 },
    InvalidServiceThreshold { name: &'static str, value: f64 },
    InvalidPowerResponseContract,
    EmptyRun,
    DuplicateRequestId(String),
    RequestManifestMismatch,
    RunIdentityMismatch,
    EmptyPowerTrace,
    PowerTimeAxisMismatch { interval_index: usize },
    NonContiguousPowerTrace { run_id: String, interval_index: usize },
    BaselineViolatesServiceContract(Vec<InferenceServiceGateViolation>),
    NumericalOverflow(&'static str),
}

impl fmt::Display for InferencePowerFlexibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::IdentityTooLong { name } => write!(f, "{name} exceeds 256 bytes"),
            Self::RunIdsMustDiffer => write!(f, "baseline and candidate run IDs must differ"),
            Self::InvalidControlAction => write!(f, "control action is empty or has invalid limits"),
            Self::InvalidRequestObligation => write!(f, "request obligation requires an ID and positive expected tokens"),
            Self::InvalidRequestTiming { name } => write!(f, "{name} must be finite and non-negative"),
            Self::InvalidSuccessfulRequest => write!(f, "successful request must have output tokens, TTFT and completion latency"),
            Self::InvalidPowerInterval => write!(f, "power interval duration must be positive"),
            Self::InvalidPowerQuantity { name, value } => write!(f, "{name} is invalid: {value}"),
            Self::InvalidUtilization { name, value } => write!(f, "{name} must be in [0, 1]; got {value}"),
            Self::InvalidServiceFraction { name, value } => write!(f, "{name} must be in [0, 1]; got {value}"),
            Self::InvalidServiceThreshold { name, value } => write!(f, "{name} is invalid: {value}"),
            Self::InvalidPowerResponseContract => write!(f, "power response contract is invalid"),
            Self::EmptyRun => write!(f, "inference run requires request results and power intervals"),
            Self::DuplicateRequestId(id) => write!(f, "duplicate inference request ID {id}"),
            Self::RequestManifestMismatch => write!(f, "baseline and candidate must complete the same frozen request manifest"),
            Self::RunIdentityMismatch => write!(f, "run IDs or control action IDs do not match frozen study identity"),
            Self::EmptyPowerTrace => write!(f, "power traces must be non-empty"),
            Self::PowerTimeAxisMismatch { interval_index } => write!(f, "baseline/candidate power time axes differ at interval {interval_index}"),
            Self::NonContiguousPowerTrace { run_id, interval_index } => write!(f, "run {run_id} power trace is not contiguous at interval {interval_index}"),
            Self::BaselineViolatesServiceContract(reasons) => write!(f, "baseline violates service contract: {reasons:?}"),
            Self::NumericalOverflow(operation) => write!(f, "non-finite arithmetic during {operation}"),
        }
    }
}

impl std::error::Error for InferencePowerFlexibilityError {}

/// Compare matched inference runs. Request obligations, model/runtime, hardware,
/// condition stratum, measurement boundary, calibration and time axis are frozen.
pub fn evaluate_inference_power_flexibility(
    identity: InferenceFlexibilityIdentity,
    request_manifest: &[InferenceRequestObligation],
    baseline: &InferenceRun,
    candidate: &InferenceRun,
    contract: &InferencePowerFlexibilityContract,
) -> Result<InferencePowerFlexibilityReport, InferencePowerFlexibilityError> {
    if baseline.run_id != identity.baseline_run_id
        || candidate.run_id != identity.candidate_run_id
        || baseline.control_action.action_id == candidate.control_action.action_id
    {
        return Err(InferencePowerFlexibilityError::RunIdentityMismatch);
    }
    if request_manifest.is_empty() {
        return Err(InferencePowerFlexibilityError::RequestManifestMismatch);
    }
    validate_request_manifest(request_manifest)?;
    validate_run_requests(&baseline.run_id, request_manifest, &baseline.request_results)?;
    validate_run_requests(&candidate.run_id, request_manifest, &candidate.request_results)?;

    validate_power_trace(&baseline.run_id, &baseline.power_intervals)?;
    validate_power_trace(&candidate.run_id, &candidate.power_intervals)?;
    if baseline.power_intervals.len() != candidate.power_intervals.len() {
        return Err(InferencePowerFlexibilityError::PowerTimeAxisMismatch {
            interval_index: baseline.power_intervals.len().min(candidate.power_intervals.len()),
        });
    }
    for (index, (left, right)) in baseline.power_intervals.iter()
        .zip(candidate.power_intervals.iter()).enumerate()
    {
        if left.start_elapsed_ms != right.start_elapsed_ms
            || left.duration_ms != right.duration_ms
        {
            return Err(InferencePowerFlexibilityError::PowerTimeAxisMismatch {
                interval_index: index,
            });
        }
    }

    let duration_ms = baseline.power_intervals.last()
        .and_then(|last| last.start_elapsed_ms.checked_add(last.duration_ms))
        .ok_or(InferencePowerFlexibilityError::NumericalOverflow("total power trace duration"))?;
    if duration_ms == 0 {
        return Err(InferencePowerFlexibilityError::EmptyPowerTrace);
    }

    let baseline_service = calculate_service_metrics(
        request_manifest,
        &baseline.request_results,
        duration_ms,
    )?;
    let candidate_service = calculate_service_metrics(
        request_manifest,
        &candidate.request_results,
        duration_ms,
    )?;
    let baseline_violations = service_violations(&baseline_service, &contract.service, None);
    if !baseline_violations.is_empty() {
        return Err(InferencePowerFlexibilityError::BaselineViolatesServiceContract(
            baseline_violations,
        ));
    }
    let candidate_violations = service_violations(
        &candidate_service,
        &contract.service,
        Some(&baseline_service),
    );
    let service_equivalent = candidate_violations.is_empty();

    let mut baseline_boundary_energy_wh = 0.0;
    let mut candidate_boundary_energy_wh = 0.0;
    let mut baseline_gpu_energy_wh = 0.0;
    let mut candidate_gpu_energy_wh = 0.0;
    let mut baseline_cpu_energy_wh = 0.0;
    let mut candidate_cpu_energy_wh = 0.0;
    let mut baseline_dram_energy_wh = 0.0;
    let mut candidate_dram_energy_wh = 0.0;
    let mut gross_reduction_energy_wh = 0.0;
    let mut rebound_energy_wh = 0.0;
    let mut maximum_sampled_reduction_w = 0.0_f64;
    let mut maximum_sampled_rebound_w = 0.0_f64;
    let mut slot_receipts = Vec::with_capacity(baseline.power_intervals.len());
    let mut consecutive = 0_usize;
    let mut current_sustained_duration_ms = 0_u64;
    let mut current_segment_start_ms = 0_u64;
    let mut maximum_sustained_reduction_duration_ms = 0_u64;
    let mut stabilized_response_elapsed_ms = None;

    for (base, flex) in baseline.power_intervals.iter().zip(candidate.power_intervals.iter()) {
        let duration_hours = base.duration_ms as f64 / 3_600_000.0;
        let reduction_w = base.boundary_power_w - flex.boundary_power_w;
        let reduction_wh = reduction_w * duration_hours;
        baseline_boundary_energy_wh += base.boundary_power_w * duration_hours;
        candidate_boundary_energy_wh += flex.boundary_power_w * duration_hours;
        baseline_gpu_energy_wh += base.gpu_power_w * duration_hours;
        candidate_gpu_energy_wh += flex.gpu_power_w * duration_hours;
        baseline_cpu_energy_wh += base.cpu_power_w * duration_hours;
        candidate_cpu_energy_wh += flex.cpu_power_w * duration_hours;
        baseline_dram_energy_wh += base.dram_power_w * duration_hours;
        candidate_dram_energy_wh += flex.dram_power_w * duration_hours;

        if reduction_w > 0.0 {
            gross_reduction_energy_wh += reduction_wh;
        } else {
            rebound_energy_wh += -reduction_wh;
        }
        maximum_sampled_reduction_w = maximum_sampled_reduction_w.max(reduction_w);
        maximum_sampled_rebound_w = maximum_sampled_rebound_w.max(-reduction_w);

        if reduction_w >= contract.response.minimum_sustained_reduction_w {
            if consecutive == 0 {
                current_segment_start_ms = base.start_elapsed_ms;
                current_sustained_duration_ms = 0;
            }
            consecutive += 1;
            current_sustained_duration_ms = current_sustained_duration_ms
                .checked_add(base.duration_ms)
                .ok_or(InferencePowerFlexibilityError::NumericalOverflow(
                    "sustained response duration",
                ))?;
            maximum_sustained_reduction_duration_ms = maximum_sustained_reduction_duration_ms
                .max(current_sustained_duration_ms);
            if consecutive >= contract.response.required_consecutive_samples
                && current_sustained_duration_ms >= contract.response.minimum_sustained_duration_ms
                && stabilized_response_elapsed_ms.is_none()
            {
                stabilized_response_elapsed_ms = Some(
                    base.start_elapsed_ms.checked_add(base.duration_ms).ok_or(
                        InferencePowerFlexibilityError::NumericalOverflow(
                            "stabilized response timestamp",
                        ),
                    )?
                );
            }
        } else {
            consecutive = 0;
            current_sustained_duration_ms = 0;
        }

        slot_receipts.push(InferencePowerSlotReceipt {
            start_elapsed_ms: base.start_elapsed_ms,
            duration_ms: base.duration_ms,
            baseline_boundary_power_w: base.boundary_power_w,
            candidate_boundary_power_w: flex.boundary_power_w,
            signed_power_reduction_w: reduction_w,
            signed_energy_reduction_wh: reduction_wh,
            baseline_gpu_power_w: base.gpu_power_w,
            candidate_gpu_power_w: flex.gpu_power_w,
            baseline_cpu_power_w: base.cpu_power_w,
            candidate_cpu_power_w: flex.cpu_power_w,
            baseline_dram_power_w: base.dram_power_w,
            candidate_dram_power_w: flex.dram_power_w,
        });
    }

    let net_boundary_energy_reduction_wh =
        baseline_boundary_energy_wh - candidate_boundary_energy_wh;
    let total_duration_hours = duration_ms as f64 / 3_600_000.0;
    let baseline_average_power = baseline_boundary_energy_wh / total_duration_hours;
    let candidate_average_power = candidate_boundary_energy_wh / total_duration_hours;
    let baseline_joules = baseline_boundary_energy_wh * 3_600.0;
    let candidate_joules = candidate_boundary_energy_wh * 3_600.0;
    let baseline_tokens_per_joule =
        baseline_service.completed_useful_tokens as f64 / baseline_joules;
    let candidate_tokens_per_joule =
        candidate_service.completed_useful_tokens as f64 / candidate_joules;

    if ![
        baseline_boundary_energy_wh,
        candidate_boundary_energy_wh,
        baseline_gpu_energy_wh,
        candidate_gpu_energy_wh,
        baseline_cpu_energy_wh,
        candidate_cpu_energy_wh,
        baseline_dram_energy_wh,
        candidate_dram_energy_wh,
        gross_reduction_energy_wh,
        rebound_energy_wh,
        net_boundary_energy_reduction_wh,
        baseline_average_power,
        candidate_average_power,
        baseline_tokens_per_joule,
        candidate_tokens_per_joule,
    ].iter().all(|value| value.is_finite()) {
        return Err(InferencePowerFlexibilityError::NumericalOverflow(
            "paired inference power/energy aggregation",
        ));
    }

    let mut power_response_gate_violations = Vec::new();
    if maximum_sampled_rebound_w > contract.response.maximum_rebound_power_w {
        power_response_gate_violations.push(
            InferencePowerResponseGateViolation::MaximumReboundPowerExceeded,
        );
    }
    if rebound_energy_wh > contract.response.maximum_rebound_energy_wh {
        power_response_gate_violations.push(
            InferencePowerResponseGateViolation::MaximumReboundEnergyExceeded,
        );
    }
    let power_response_qualified = service_equivalent
        && stabilized_response_elapsed_ms.is_some()
        && maximum_sampled_reduction_w >= contract.response.minimum_sustained_reduction_w
        && maximum_sustained_reduction_duration_ms
            >= contract.response.minimum_sustained_duration_ms
        && power_response_gate_violations.is_empty();
    let claim_ceiling = format!(
        "MEASURED_INFERENCE_POWER_QOS_ENVELOPE_AT_{:?}_BOUNDARY_NOT_GRID_DISPATCH_OR_RESOURCE_ACCREDITATION",
        identity.boundary
    );

    Ok(InferencePowerFlexibilityReport {
        identity,
        baseline_control_action: baseline.control_action.clone(),
        candidate_control_action: candidate.control_action.clone(),
        baseline_service,
        candidate_service,
        baseline_boundary_energy_wh,
        candidate_boundary_energy_wh,
        baseline_gpu_energy_wh,
        candidate_gpu_energy_wh,
        baseline_cpu_energy_wh,
        candidate_cpu_energy_wh,
        baseline_dram_energy_wh,
        candidate_dram_energy_wh,
        baseline_tokens_per_joule,
        candidate_tokens_per_joule,
        baseline_average_boundary_power_w: baseline_average_power,
        candidate_average_boundary_power_w: candidate_average_power,
        maximum_sampled_reduction_w,
        maximum_sampled_rebound_w,
        maximum_sustained_reduction_duration_ms,
        gross_reduction_energy_wh,
        rebound_energy_wh,
        net_boundary_energy_reduction_wh,
        stabilized_response_elapsed_ms,
        candidate_service_gate_violations: candidate_violations,
        power_response_gate_violations,
        service_equivalent,
        power_response_qualified,
        slot_receipts,
        claim_ceiling,
    })
}

fn validate_request_manifest(
    manifest: &[InferenceRequestObligation],
) -> Result<(), InferencePowerFlexibilityError> {
    let mut ids = std::collections::BTreeSet::new();
    for request in manifest {
        if !ids.insert(request.request_id.clone()) {
            return Err(InferencePowerFlexibilityError::DuplicateRequestId(
                request.request_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_run_requests(
    run_id: &str,
    manifest: &[InferenceRequestObligation],
    results: &[InferenceRequestResult],
) -> Result<(), InferencePowerFlexibilityError> {
    if manifest.len() != results.len() {
        return Err(InferencePowerFlexibilityError::RequestManifestMismatch);
    }
    let mut result_map = std::collections::BTreeMap::new();
    for result in results {
        if result_map.insert(result.request_id.as_str(), result).is_some() {
            return Err(InferencePowerFlexibilityError::DuplicateRequestId(
                result.request_id.clone(),
            ));
        }
    }
    for obligation in manifest {
        if !result_map.contains_key(obligation.request_id.as_str()) {
            return Err(InferencePowerFlexibilityError::RequestManifestMismatch);
        }
    }
    if result_map.len() != manifest.len() {
        return Err(InferencePowerFlexibilityError::RequestManifestMismatch);
    }
    let _ = run_id;
    Ok(())
}

fn validate_power_trace(
    run_id: &str,
    intervals: &[InferencePowerInterval],
) -> Result<(), InferencePowerFlexibilityError> {
    if intervals.is_empty() {
        return Err(InferencePowerFlexibilityError::EmptyPowerTrace);
    }
    if intervals[0].start_elapsed_ms != 0 {
        return Err(InferencePowerFlexibilityError::NonContiguousPowerTrace {
            run_id: run_id.to_string(),
            interval_index: 0,
        });
    }
    for (index, pair) in intervals.windows(2).enumerate() {
        let expected_start = pair[0].start_elapsed_ms
            .checked_add(pair[0].duration_ms)
            .ok_or(InferencePowerFlexibilityError::NumericalOverflow("power interval chronology"))?;
        if pair[1].start_elapsed_ms != expected_start {
            return Err(InferencePowerFlexibilityError::NonContiguousPowerTrace {
                run_id: run_id.to_string(),
                interval_index: index + 1,
            });
        }
    }
    Ok(())
}

fn calculate_service_metrics(
    manifest: &[InferenceRequestObligation],
    results: &[InferenceRequestResult],
    duration_ms: u64,
) -> Result<InferenceServiceMetrics, InferencePowerFlexibilityError> {
    let mut result_map = std::collections::BTreeMap::new();
    for result in results {
        result_map.insert(result.request_id.as_str(), result);
    }
    let expected_tokens = manifest.iter().try_fold(0_u64, |total, obligation| {
        total.checked_add(obligation.expected_output_tokens)
            .ok_or(InferencePowerFlexibilityError::NumericalOverflow("expected output token count"))
    })?;
    let mut completed_useful_tokens = 0_u64;
    let mut total_output_tokens = 0_u64;
    let mut successful_request_count = 0_usize;
    let mut ttft_values = Vec::new();
    let mut latency_values = Vec::new();
    for obligation in manifest {
        let result = result_map.get(obligation.request_id.as_str())
            .ok_or(InferencePowerFlexibilityError::RequestManifestMismatch)?;
        if result.succeeded {
            successful_request_count += 1;
        }
        if result.succeeded {
            completed_useful_tokens = completed_useful_tokens.checked_add(
                result.output_tokens.min(obligation.expected_output_tokens)
            ).ok_or(InferencePowerFlexibilityError::NumericalOverflow("completed useful token count"))?;
        }
        total_output_tokens = total_output_tokens.checked_add(result.output_tokens)
            .ok_or(InferencePowerFlexibilityError::NumericalOverflow("output token count"))?;
        if result.succeeded {
            ttft_values.push(result.time_to_first_token_ms.ok_or(
                InferencePowerFlexibilityError::InvalidSuccessfulRequest
            )?);
            latency_values.push(result.completion_latency_ms.ok_or(
                InferencePowerFlexibilityError::InvalidSuccessfulRequest
            )?);
        }
    }
    let request_count = manifest.len();
    let success_fraction = successful_request_count as f64 / request_count as f64;
    let useful_work_fraction = completed_useful_tokens as f64 / expected_tokens as f64;
    let duration_seconds = duration_ms as f64 / 1_000.0;
    let useful_tokens_per_second = completed_useful_tokens as f64 / duration_seconds;
    let tokens_per_second = total_output_tokens as f64 / duration_seconds;
    let metrics = InferenceServiceMetrics {
        request_count,
        successful_request_count,
        success_fraction,
        useful_work_fraction,
        completed_useful_tokens,
        total_output_tokens,
        useful_tokens_per_second,
        tokens_per_second,
        p95_ttft_ms: nearest_rank_quantile(&ttft_values, 95),
        p99_ttft_ms: nearest_rank_quantile(&ttft_values, 99),
        p95_completion_latency_ms: nearest_rank_quantile(&latency_values, 95),
        p99_completion_latency_ms: nearest_rank_quantile(&latency_values, 99),
    };
    if ![
        metrics.success_fraction,
        metrics.useful_work_fraction,
        metrics.useful_tokens_per_second,
        metrics.tokens_per_second,
    ].iter().all(|value| value.is_finite()) {
        return Err(InferencePowerFlexibilityError::NumericalOverflow(
            "inference service metrics",
        ));
    }
    Ok(metrics)
}

fn nearest_rank_quantile(values: &[f64], percentile: usize) -> Option<f64> {
    if values.is_empty() || percentile == 0 || percentile > 100 {
        return None;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let rank = (percentile * ordered.len()).div_ceil(100).max(1);
    ordered.get(rank - 1).copied()
}

fn service_violations(
    metrics: &InferenceServiceMetrics,
    contract: &InferenceServiceContract,
    baseline: Option<&InferenceServiceMetrics>,
) -> Vec<InferenceServiceGateViolation> {
    let mut reasons = Vec::new();
    if metrics.success_fraction < contract.minimum_success_fraction {
        reasons.push(InferenceServiceGateViolation::SuccessFractionBelowMinimum);
    }
    if metrics.useful_work_fraction < contract.minimum_useful_work_fraction {
        reasons.push(InferenceServiceGateViolation::UsefulWorkFractionBelowMinimum);
    }
    if metrics.useful_tokens_per_second < contract.minimum_useful_tokens_per_second {
        reasons.push(InferenceServiceGateViolation::TokensPerSecondBelowMinimum);
    }
    if let Some(base) = baseline {
        let required_throughput = base.useful_tokens_per_second
            * contract.minimum_throughput_retention_fraction;
        if metrics.useful_tokens_per_second < required_throughput {
            reasons.push(InferenceServiceGateViolation::ThroughputRetentionBelowMinimum);
        }
    }

    match metrics.p95_ttft_ms {
        Some(value) if value <= contract.maximum_p95_ttft_ms => {}
        Some(_) => reasons.push(InferenceServiceGateViolation::P95TtftAboveMaximum),
        None => reasons.push(InferenceServiceGateViolation::P95TtftUnavailable),
    }
    match metrics.p99_ttft_ms {
        Some(value) if value <= contract.maximum_p99_ttft_ms => {}
        Some(_) => reasons.push(InferenceServiceGateViolation::P99TtftAboveMaximum),
        None => reasons.push(InferenceServiceGateViolation::P99TtftUnavailable),
    }
    match metrics.p95_completion_latency_ms {
        Some(value) if value <= contract.maximum_p95_completion_latency_ms => {}
        Some(_) => reasons.push(InferenceServiceGateViolation::P95LatencyAboveMaximum),
        None => reasons.push(InferenceServiceGateViolation::P95LatencyUnavailable),
    }
    match metrics.p99_completion_latency_ms {
        Some(value) if value <= contract.maximum_p99_completion_latency_ms => {}
        Some(_) => reasons.push(InferenceServiceGateViolation::P99LatencyAboveMaximum),
        None => reasons.push(InferenceServiceGateViolation::P99LatencyUnavailable),
    }

    if let Some(base) = baseline {
        if let (Some(base_p99), Some(candidate_p99)) =
            (base.p99_ttft_ms, metrics.p99_ttft_ms)
        {
            if candidate_p99 > base_p99 * (1.0 + contract.maximum_p99_ttft_degradation_fraction) {
                reasons.push(InferenceServiceGateViolation::P99TtftRegressionExceeded);
            }
        }
        if let (Some(base_p99), Some(candidate_p99)) =
            (base.p99_completion_latency_ms, metrics.p99_completion_latency_ms)
        {
            if candidate_p99 > base_p99
                * (1.0 + contract.maximum_p99_latency_degradation_fraction)
            {
                reasons.push(InferenceServiceGateViolation::P99LatencyRegressionExceeded);
            }
        }
    }
    reasons
}

fn require_id(name: &'static str, value: &str) -> Result<(), InferencePowerFlexibilityError> {
    if value.trim().is_empty() {
        Err(InferencePowerFlexibilityError::MissingIdentity { name })
    } else if value.len() > 256 {
        Err(InferencePowerFlexibilityError::IdentityTooLong { name })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> InferenceFlexibilityIdentity {
        InferenceFlexibilityIdentity::new(
            "study-1", "baseline-run", "candidate-run", "NVIDIA-H100",
            "llm-inference-v1", "model-sha256", "runtime-sha256",
            "request-manifest-sha256", "steady-state-stratum",
            "rack-pdu-1", "meter-cal-1", InferenceMeasurementBoundary::RackPdu,
        ).expect("identity")
    }

    fn service_contract() -> InferenceServiceContract {
        InferenceServiceContract::new(
            1.0, 0.99, 10.0, 0.90, 100.0, 150.0,
            500.0, 750.0, 0.10, 0.10,
        ).expect("service contract")
    }

    fn response_contract() -> InferencePowerResponseContract {
        InferencePowerResponseContract::new(15.0, 3, 300, 10.0, 0.0005).expect("response contract")
    }

    fn action(id: &str, cap: f64) -> InferenceControlAction {
        InferenceControlAction::new(
            id, Some(1_200.0), Some(cap), Some(300.0), Some(8), Some(16),
        ).expect("action")
    }

    fn manifest() -> Vec<InferenceRequestObligation> {
        (0..4).map(|i| InferenceRequestObligation::new(
            format!("req-{i}"), 100,
        ).expect("obligation")).collect()
    }

    fn results(prefix: &str, slow_last: bool) -> Vec<InferenceRequestResult> {
        (0..4).map(|i| {
            let latency = if slow_last && i == 3 { 900.0 } else { 100.0 };
            InferenceRequestResult::new(
                format!("req-{i}"), true, 100, Some(20.0), Some(latency),
            ).expect("result")
        }).collect::<Vec<_>>().into_iter().map(|r| {
            let _ = prefix;
            r
        }).collect()
    }

    fn intervals(boundary_power: &[f64]) -> Vec<InferencePowerInterval> {
        boundary_power.iter().enumerate().map(|(i, p)| {
            InferencePowerInterval::new(
                i as u64 * 100, 100, *p, *p * 0.6, 20.0, 10.0,
                1_200.0, 0.8, 0.5, 65.0, 55.0,
            ).expect("interval")
        }).collect()
    }

    fn run(
        id: &str,
        action: InferenceControlAction,
        result: Vec<InferenceRequestResult>,
        power: Vec<f64>,
    ) -> InferenceRun {
        InferenceRun::new(id, action, result, intervals(&power)).expect("run")
    }

    fn contract() -> InferencePowerFlexibilityContract {
        InferencePowerFlexibilityContract::new(service_contract(), response_contract())
    }

    #[test]
    fn paired_power_reduction_is_qualified_only_when_service_is_preserved() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0, 120.0, 120.0, 120.0, 120.0]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![120.0, 100.0, 100.0, 100.0, 100.0]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("paired measurement");
        assert!(report.service_equivalent);
        assert!(report.power_response_qualified);
        assert_eq!(report.stabilized_response_elapsed_ms, Some(400));
        assert_eq!(report.maximum_sampled_reduction_w, 20.0);
        assert!((report.gross_reduction_energy_wh - 20.0 * 0.4 / 3_600.0).abs() < 1e-12);
        assert_eq!(report.rebound_energy_wh, 0.0);
    }

    #[test]
    fn brief_power_dip_does_not_qualify_as_sustained_response() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![120.0, 100.0, 100.0, 120.0, 120.0]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("paired measurement");
        assert_eq!(report.maximum_sustained_reduction_duration_ms, 200);
        assert_eq!(report.stabilized_response_elapsed_ms, None);
        assert!(!report.power_response_qualified);
        assert!(report.service_equivalent);
    }

    #[test]
    fn response_and_recovery_are_both_retained_in_signed_energy() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![100.0, 100.0, 140.0, 140.0, 120.0]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("paired measurement");
        assert_eq!(report.maximum_sampled_reduction_w, 20.0);
        assert_eq!(report.maximum_sampled_rebound_w, 20.0);
        assert!(report.rebound_energy_wh > 0.0);
        assert!(report.net_boundary_energy_reduction_wh < report.gross_reduction_energy_wh);
    }

    #[test]
    fn sustained_power_response_with_excessive_rebound_fails_response_envelope() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![120.0, 100.0, 100.0, 100.0, 140.0]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("paired measurement");
        assert!(report.service_equivalent);
        assert_eq!(report.stabilized_response_elapsed_ms, Some(400));
        assert!(!report.power_response_qualified);
        assert!(report.power_response_gate_violations.contains(
            &InferencePowerResponseGateViolation::MaximumReboundPowerExceeded
        ));
        assert!(report.power_response_gate_violations.contains(
            &InferencePowerResponseGateViolation::MaximumReboundEnergyExceeded
        ));
    }

    #[test]
    fn failed_request_partial_tokens_do_not_count_as_completed_useful_work() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let mut candidate_results = results("c", false);
        candidate_results[0] = InferenceRequestResult::new(
            "req-0", false, 100, None, None,
        ).expect("failed request may retain partial output tokens");
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            candidate_results, vec![100.0; 5]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("measurement reports service failure");
        assert_eq!(report.candidate_service.completed_useful_tokens, 300);
        assert_eq!(report.candidate_service.total_output_tokens, 400);
        assert_eq!(report.candidate_service.useful_work_fraction, 0.75);
        assert!(!report.service_equivalent);
    }

    #[test]
    fn p99_latency_slo_breach_blocks_power_flexibility_credit() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", true), vec![100.0; 5]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("measurement still reports the breach");
        assert!(!report.service_equivalent);
        assert!(!report.power_response_qualified);
        assert!(report.candidate_service_gate_violations.contains(
            &InferenceServiceGateViolation::P99LatencyAboveMaximum
        ));
    }

    #[test]
    fn useful_token_loss_blocks_service_equivalence_even_when_requests_succeed() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let reduced_work = (0..4).map(|i| InferenceRequestResult::new(
            format!("req-{i}"), true, 50, Some(20.0), Some(100.0),
        ).expect("result")).collect();
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            reduced_work, vec![100.0; 5]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("measurement");
        assert!(!report.service_equivalent);
        assert!(report.candidate_service_gate_violations.contains(
            &InferenceServiceGateViolation::UsefulWorkFractionBelowMinimum
        ));
    }

    #[test]
    fn request_manifest_must_match_exactly() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let mut candidate_results = results("c", false);
        candidate_results[0] = InferenceRequestResult::new(
            "different-request", true, 100, Some(20.0), Some(100.0),
        ).expect("result");
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            candidate_results, vec![100.0; 5]);
        assert_eq!(
            evaluate_inference_power_flexibility(
                identity(), &manifest(), &baseline, &candidate, &contract(),
            ),
            Err(InferencePowerFlexibilityError::RequestManifestMismatch)
        );
    }

    #[test]
    fn baseline_must_satisfy_service_contract_before_comparison() {
        let bad_baseline_results = (0..4).map(|i| InferenceRequestResult::new(
            format!("req-{i}"), true, 100, Some(20.0), Some(900.0),
        ).expect("result")).collect();
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            bad_baseline_results, vec![120.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![100.0; 5]);
        assert!(matches!(
            evaluate_inference_power_flexibility(
                identity(), &manifest(), &baseline, &candidate, &contract(),
            ),
            Err(InferencePowerFlexibilityError::BaselineViolatesServiceContract(_))
        ));
    }

    #[test]
    fn power_time_axes_must_be_identical() {
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let mut shifted = intervals(&[100.0; 5]);
        shifted[2] = InferencePowerInterval::new(
            250, 100, 100.0, 60.0, 20.0, 10.0, 1_200.0, 0.8, 0.5, 65.0, 55.0,
        ).expect("interval");
        let candidate = InferenceRun::new(
            "candidate-run", action("candidate-action", 180.0), results("c", false), shifted,
        ).expect("run");
        assert!(matches!(
            evaluate_inference_power_flexibility(
                identity(), &manifest(), &baseline, &candidate, &contract(),
            ),
            Err(InferencePowerFlexibilityError::NonContiguousPowerTrace { .. })
        ));
    }

    #[test]
    fn time_quantiles_use_nearest_rank_and_boundary_meter_is_authoritative() {
        assert_eq!(nearest_rank_quantile(&[1.0, 2.0, 3.0, 4.0], 95), Some(4.0));
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![100.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![90.0; 5]);
        let report = evaluate_inference_power_flexibility(
            identity(), &manifest(), &baseline, &candidate, &contract(),
        ).expect("measurement");
        assert_eq!(report.baseline_average_boundary_power_w, 100.0);
        assert_eq!(report.candidate_average_boundary_power_w, 90.0);
        assert!((report.baseline_boundary_energy_wh - 60.0 / 3_600.0).abs() < 1e-12);
        assert!((report.candidate_boundary_energy_wh - 52.0 / 3_600.0).abs() < 1e-12);
        assert!((report.baseline_tokens_per_joule - 400.0 / 60.0).abs() < 1e-9);
        assert!((report.candidate_tokens_per_joule - 400.0 / 52.0).abs() < 1e-9);
    }

    #[test]
    fn duplicate_manifest_ids_are_rejected() {
        let duplicate = vec![
            InferenceRequestObligation::new("same", 100).expect("request"),
            InferenceRequestObligation::new("same", 100).expect("request"),
        ];
        let baseline = run("baseline-run", action("baseline-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let candidate = run("candidate-run", action("candidate-action", 180.0),
            results("c", false), vec![100.0; 5]);
        assert!(matches!(
            evaluate_inference_power_flexibility(
                identity(), &duplicate, &baseline, &candidate, &contract(),
            ),
            Err(InferencePowerFlexibilityError::DuplicateRequestId(_))
        ));
    }

    #[test]
    fn non_finite_power_and_out_of_range_utilization_are_rejected() {
        assert!(matches!(
            InferencePowerInterval::new(
                0, 100, f64::NAN, 20.0, 20.0, 5.0, 1_200.0, 0.5, 0.5, 60.0, 50.0,
            ),
            Err(InferencePowerFlexibilityError::InvalidPowerQuantity { .. })
        ));
        assert!(matches!(
            InferencePowerInterval::new(
                0, 100, 100.0, 20.0, 20.0, 5.0, 1_200.0, 1.2, 0.5, 60.0, 50.0,
            ),
            Err(InferencePowerFlexibilityError::InvalidUtilization { .. })
        ));
    }

    #[test]
    fn action_ids_must_differ() {
        let baseline = run("baseline-run", action("same-action", 250.0),
            results("b", false), vec![120.0; 5]);
        let candidate = run("candidate-run", action("same-action", 180.0),
            results("c", false), vec![100.0; 5]);
        assert_eq!(
            evaluate_inference_power_flexibility(
                identity(), &manifest(), &baseline, &candidate, &contract(),
            ),
            Err(InferencePowerFlexibilityError::RunIdentityMismatch)
        );
    }
}
