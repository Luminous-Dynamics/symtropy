// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-separated evaluation of WPT reduced-order and learned model traces.
//!
//! This crate scores time-aligned candidate traces against a declared reference.
//! It does not establish physical passivity, stability, electromagnetic safety,
//! model provenance, or the validity of the reference itself.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SUITE_SCHEMA_VERSION: &str = "wpt-multifidelity-bench-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SyntheticKnownTruth,
    PublishedDataReplay,
    ExperimentalDataReplay,
    SimulationReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelFamily {
    FullOrder,
    BalancedTruncation,
    DataDrivenPetrovGalerkin,
    PortHamiltonian,
    LearnedSurrogate,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PowerFlowDirection {
    GridToVehicle,
    VehicleToGrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceChannel {
    pub channel_id: String,
    pub unit: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveformTrace {
    pub time_s: Vec<f64>,
    pub channels: Vec<TraceChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCandidate {
    pub candidate_id: String,
    pub family: ModelFamily,
    pub state_order: Option<u32>,
    pub model_digest: String,
    pub training_data_digest: String,
    pub execution_environment_digest: String,
    /// Median end-to-end evaluation latency over the declared repetition protocol.
    pub runtime_median_ms: f64,
    pub runtime_p95_ms: f64,
    pub peak_memory_bytes: u64,
    pub trace: WaveformTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeakLimit {
    pub channel_id: String,
    pub maximum_absolute_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceThresholds {
    pub maximum_channel_nrmse: f64,
    pub maximum_peak_magnitude_relative_error: f64,
    pub maximum_energy_relative_error: f64,
    pub peak_limits: Vec<PeakLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkCase {
    pub case_id: String,
    pub evidence_class: EvidenceClass,
    pub power_flow_direction: PowerFlowDirection,
    pub execution_environment_digest: String,
    pub reference_runtime_median_ms: f64,
    pub reference_runtime_p95_ms: f64,
    pub reference: WaveformTrace,
    /// Active-power channels in W. Each is integrated independently in its
    /// declared positive flow direction; receiver totals are not collapsed.
    pub energy_channel_ids: Vec<String>,
    pub candidates: Vec<ModelCandidate>,
    pub acceptance: AcceptanceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkSuite {
    pub schema_version: String,
    pub suite_id: String,
    pub cases: Vec<BenchmarkCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelMetric {
    pub channel_id: String,
    pub nrmse: f64,
    pub peak_magnitude_relative_error: f64,
    pub reference_peak_absolute: f64,
    pub candidate_peak_absolute: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnergyMetric {
    pub channel_id: String,
    pub reference_energy_wh: f64,
    pub candidate_energy_wh: f64,
    pub absolute_error_wh: f64,
    pub relative_error: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateScore {
    pub case_id: String,
    pub evidence_class: EvidenceClass,
    pub power_flow_direction: PowerFlowDirection,
    pub candidate_id: String,
    pub family: ModelFamily,
    pub model_digest: String,
    pub training_data_digest: String,
    pub execution_environment_digest: String,
    pub runtime_median_ms: f64,
    pub runtime_p95_ms: f64,
    pub peak_memory_bytes: u64,
    pub speedup_vs_reference: f64,
    pub state_order: Option<u32>,
    pub channel_metrics: Vec<ChannelMetric>,
    pub energy_metrics: Vec<EnergyMetric>,
    pub maximum_channel_nrmse: f64,
    pub maximum_peak_magnitude_relative_error: f64,
    pub maximum_energy_relative_error: f64,
    pub maximum_normalized_violation: f64,
    pub feasible: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub suite_id: String,
    pub scores: Vec<CandidateScore>,
    pub ranking: Vec<Vec<String>>,
    pub claim_ceiling: String,
}

impl BenchmarkSuite {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SUITE_SCHEMA_VERSION {
            return Err("unsupported benchmark suite schema version".into());
        }
        if self.suite_id.trim().is_empty() || self.cases.is_empty() {
            return Err("suite_id and at least one case are required".into());
        }

        let mut case_ids = HashSet::with_capacity(self.cases.len());
        for case in &self.cases {
            if case.case_id.trim().is_empty() || !case_ids.insert(&case.case_id) {
                return Err("case IDs must be non-empty and unique".into());
            }
            if !valid_sha256(&case.execution_environment_digest)
                || !case.reference_runtime_median_ms.is_finite()
                || case.reference_runtime_median_ms <= 0.0
                || !case.reference_runtime_p95_ms.is_finite()
                || case.reference_runtime_p95_ms < case.reference_runtime_median_ms
            {
                return Err(
                    "reference runtime or execution-environment metadata is invalid".into(),
                );
            }
            validate_trace(&case.reference)?;
            let reference_channels = channel_index(&case.reference)?;
            if case.candidates.is_empty() {
                return Err(format!("case {} has no model candidates", case.case_id));
            }

            let mut candidate_ids = HashSet::with_capacity(case.candidates.len());
            for candidate in &case.candidates {
                if candidate.candidate_id.trim().is_empty()
                    || !candidate_ids.insert(&candidate.candidate_id)
                {
                    return Err(format!(
                        "candidate IDs must be non-empty and unique in case {}",
                        case.case_id
                    ));
                }
                if candidate.state_order == Some(0) {
                    return Err("state_order must be positive when supplied".into());
                }
                if candidate.family != ModelFamily::LearnedSurrogate
                    && candidate.family != ModelFamily::Other
                    && candidate.state_order.is_none()
                {
                    return Err(
                        "state_order is required for explicit state-space model families".into(),
                    );
                }
                if !valid_sha256(&candidate.model_digest)
                    || !valid_sha256(&candidate.training_data_digest)
                    || !valid_sha256(&candidate.execution_environment_digest)
                {
                    return Err("model, training-data and execution-environment digests must be sha256:<64 lowercase hex>".into());
                }
                if candidate.execution_environment_digest != case.execution_environment_digest {
                    return Err("candidate and reference execution environments differ".into());
                }
                if !candidate.runtime_median_ms.is_finite()
                    || candidate.runtime_median_ms <= 0.0
                    || !candidate.runtime_p95_ms.is_finite()
                    || candidate.runtime_p95_ms < candidate.runtime_median_ms
                    || candidate.peak_memory_bytes == 0
                {
                    return Err("candidate runtime distribution or peak memory is invalid".into());
                }
                if !(case.reference_runtime_median_ms / candidate.runtime_median_ms).is_finite() {
                    return Err("candidate speedup calculation overflowed".into());
                }
                validate_trace(&candidate.trace)?;
                if candidate.trace.time_s != case.reference.time_s {
                    return Err(format!(
                        "candidate {} time grid differs from reference in case {}",
                        candidate.candidate_id, case.case_id
                    ));
                }
                let candidate_channels = channel_index(&candidate.trace)?;
                if reference_channels.len() != candidate_channels.len() {
                    return Err("candidate and reference channel counts differ".into());
                }
                for (channel_id, reference_channel) in &reference_channels {
                    let candidate_channel = candidate_channels
                        .get(channel_id)
                        .ok_or_else(|| format!("candidate is missing channel {channel_id}"))?;
                    if candidate_channel.unit != reference_channel.unit {
                        return Err(format!("unit mismatch for channel {channel_id}"));
                    }
                }
            }

            let mut energy_ids = HashSet::with_capacity(case.energy_channel_ids.len());
            if case.energy_channel_ids.is_empty() {
                return Err("at least one active-power energy channel is required".into());
            }
            for channel_id in &case.energy_channel_ids {
                if !energy_ids.insert(channel_id) {
                    return Err("energy channel IDs must be unique".into());
                }
                let channel = reference_channels
                    .get(channel_id)
                    .ok_or_else(|| format!("unknown energy channel {channel_id}"))?;
                if channel.unit != "W" {
                    return Err(format!("energy channel {channel_id} must use watts (W)"));
                }
            }

            let mut peak_limit_ids = HashSet::with_capacity(case.acceptance.peak_limits.len());
            for limit in &case.acceptance.peak_limits {
                if !peak_limit_ids.insert(&limit.channel_id) {
                    return Err("peak-limit channel IDs must be unique".into());
                }
                if !reference_channels.contains_key(&limit.channel_id) {
                    return Err(format!(
                        "peak limit references unknown channel {}",
                        limit.channel_id
                    ));
                }
                if !limit.maximum_absolute_value.is_finite() || limit.maximum_absolute_value <= 0.0
                {
                    return Err("peak limits must be finite and positive".into());
                }
            }
            for (name, value) in [
                (
                    "maximum_channel_nrmse",
                    case.acceptance.maximum_channel_nrmse,
                ),
                (
                    "maximum_peak_magnitude_relative_error",
                    case.acceptance.maximum_peak_magnitude_relative_error,
                ),
                (
                    "maximum_energy_relative_error",
                    case.acceptance.maximum_energy_relative_error,
                ),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(format!("{name} must be finite and non-negative"));
                }
            }
        }
        Ok(())
    }

    pub fn evaluate(&self) -> Result<BenchmarkReport, String> {
        self.validate()?;
        let mut scores = Vec::new();
        let mut ranking = Vec::new();

        for case in &self.cases {
            let mut case_scores = Vec::with_capacity(case.candidates.len());
            for candidate in &case.candidates {
                case_scores.push(score_candidate(case, candidate)?);
            }
            case_scores.sort_by(compare_scores);
            ranking.push(
                case_scores
                    .iter()
                    .map(|score| score.candidate_id.clone())
                    .collect(),
            );
            scores.extend(case_scores);
        }

        Ok(BenchmarkReport {
            schema_version: SUITE_SCHEMA_VERSION.into(),
            suite_id: self.suite_id.clone(),
            scores,
            ranking,
            claim_ceiling: "Trace-comparison benchmark only. This report does not prove passivity, stability, causality, electromagnetic safety, thermal safety, hardware validity, provenance authenticity, or standards compliance. Compare model families only on matched cases and measurement boundaries.".into(),
        })
    }
}

fn score_candidate(
    case: &BenchmarkCase,
    candidate: &ModelCandidate,
) -> Result<CandidateScore, String> {
    let reference = channel_index(&case.reference)?;
    let predicted = channel_index(&candidate.trace)?;
    let mut channel_metrics = Vec::with_capacity(reference.len());
    let mut max_nrmse = 0.0_f64;
    let mut max_peak_error = 0.0_f64;

    for (channel_id, reference_channel) in &reference {
        let predicted_channel = predicted
            .get(channel_id)
            .ok_or_else(|| format!("candidate missing channel {channel_id}"))?;
        let count = reference_channel.values.len() as f64;
        let mse = reference_channel
            .values
            .iter()
            .zip(&predicted_channel.values)
            .map(|(reference, predicted)| (reference - predicted).powi(2))
            .sum::<f64>()
            / count;
        let reference_rms = (reference_channel
            .values
            .iter()
            .map(|value| value.powi(2))
            .sum::<f64>()
            / count)
            .sqrt();
        if reference_rms <= f64::MIN_POSITIVE {
            return Err(format!(
                "reference channel {channel_id} is all-zero and NRMSE is undefined"
            ));
        }
        let nrmse = mse.sqrt() / reference_rms;
        let reference_peak = peak_absolute(&reference_channel.values);
        let candidate_peak = peak_absolute(&predicted_channel.values);
        let peak_error =
            (candidate_peak - reference_peak).abs() / reference_peak.max(f64::MIN_POSITIVE);
        if !nrmse.is_finite() || !peak_error.is_finite() {
            return Err(format!("non-finite metric for channel {channel_id}"));
        }
        max_nrmse = max_nrmse.max(nrmse);
        max_peak_error = max_peak_error.max(peak_error);
        channel_metrics.push(ChannelMetric {
            channel_id: channel_id.clone(),
            nrmse,
            peak_magnitude_relative_error: peak_error,
            reference_peak_absolute: reference_peak,
            candidate_peak_absolute: candidate_peak,
        });
    }

    let mut energy_metrics = Vec::with_capacity(case.energy_channel_ids.len());
    let mut max_energy_error = 0.0_f64;
    for channel_id in &case.energy_channel_ids {
        let reference_channel = reference.get(channel_id).expect("validated energy channel");
        let predicted_channel = predicted
            .get(channel_id)
            .expect("validated candidate channel");
        let reference_energy = integrate_wh(&case.reference.time_s, &reference_channel.values)?;
        let candidate_energy = integrate_wh(&candidate.trace.time_s, &predicted_channel.values)?;
        let absolute_error = (candidate_energy - reference_energy).abs();
        let relative_error = absolute_error / reference_energy.abs().max(1e-12);
        if !relative_error.is_finite() {
            return Err(format!("non-finite energy error for {channel_id}"));
        }
        max_energy_error = max_energy_error.max(relative_error);
        energy_metrics.push(EnergyMetric {
            channel_id: channel_id.clone(),
            reference_energy_wh: reference_energy,
            candidate_energy_wh: candidate_energy,
            absolute_error_wh: absolute_error,
            relative_error,
        });
    }

    let mut violations = Vec::new();
    let mut max_violation = 0.0_f64;
    record_violation(
        "MAX_CHANNEL_NRMSE",
        max_nrmse,
        case.acceptance.maximum_channel_nrmse,
        &mut max_violation,
        &mut violations,
    );
    record_violation(
        "MAX_PEAK_MAGNITUDE_RELATIVE_ERROR",
        max_peak_error,
        case.acceptance.maximum_peak_magnitude_relative_error,
        &mut max_violation,
        &mut violations,
    );
    record_violation(
        "MAX_ENERGY_RELATIVE_ERROR",
        max_energy_error,
        case.acceptance.maximum_energy_relative_error,
        &mut max_violation,
        &mut violations,
    );

    for limit in &case.acceptance.peak_limits {
        let reference_peak = peak_absolute(
            &reference
                .get(&limit.channel_id)
                .expect("validated peak-limit channel")
                .values,
        );
        if reference_peak > limit.maximum_absolute_value {
            let ratio = reference_peak / limit.maximum_absolute_value;
            let excess = if ratio.is_finite() {
                ratio - 1.0
            } else {
                f64::MAX
            };
            max_violation = max_violation.max(excess);
            violations.push(format!(
                "REFERENCE_EXCEEDS_LIMIT:{}:{reference_peak}:{}",
                limit.channel_id, limit.maximum_absolute_value
            ));
        }

        let peak = channel_metrics
            .iter()
            .find(|metric| metric.channel_id == limit.channel_id)
            .expect("validated peak-limit channel")
            .candidate_peak_absolute;
        if peak > limit.maximum_absolute_value {
            let ratio = peak / limit.maximum_absolute_value;
            let excess = if ratio.is_finite() {
                ratio - 1.0
            } else {
                f64::MAX
            };
            max_violation = max_violation.max(excess);
            violations.push(format!(
                "PEAK_LIMIT_EXCEEDED:{}:{peak}:{}",
                limit.channel_id, limit.maximum_absolute_value
            ));
        }
    }

    Ok(CandidateScore {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        power_flow_direction: case.power_flow_direction.clone(),
        candidate_id: candidate.candidate_id.clone(),
        family: candidate.family.clone(),
        model_digest: candidate.model_digest.clone(),
        training_data_digest: candidate.training_data_digest.clone(),
        execution_environment_digest: candidate.execution_environment_digest.clone(),
        runtime_median_ms: candidate.runtime_median_ms,
        runtime_p95_ms: candidate.runtime_p95_ms,
        peak_memory_bytes: candidate.peak_memory_bytes,
        speedup_vs_reference: case.reference_runtime_median_ms / candidate.runtime_median_ms,
        state_order: candidate.state_order,
        channel_metrics,
        energy_metrics,
        maximum_channel_nrmse: max_nrmse,
        maximum_peak_magnitude_relative_error: max_peak_error,
        maximum_energy_relative_error: max_energy_error,
        maximum_normalized_violation: max_violation,
        feasible: violations.is_empty(),
        violations,
    })
}

fn compare_scores(left: &CandidateScore, right: &CandidateScore) -> std::cmp::Ordering {
    right
        .feasible
        .cmp(&left.feasible)
        .then_with(|| {
            left.maximum_normalized_violation
                .total_cmp(&right.maximum_normalized_violation)
        })
        .then_with(|| {
            left.maximum_energy_relative_error
                .total_cmp(&right.maximum_energy_relative_error)
        })
        .then_with(|| {
            left.maximum_channel_nrmse
                .total_cmp(&right.maximum_channel_nrmse)
        })
        .then_with(|| {
            right
                .speedup_vs_reference
                .total_cmp(&left.speedup_vs_reference)
        })
        .then_with(|| {
            left.state_order
                .unwrap_or(u32::MAX)
                .cmp(&right.state_order.unwrap_or(u32::MAX))
        })
        .then_with(|| left.candidate_id.cmp(&right.candidate_id))
}

fn record_violation(
    name: &str,
    actual: f64,
    maximum: f64,
    maximum_violation: &mut f64,
    violations: &mut Vec<String>,
) {
    if actual > maximum {
        let ratio = if maximum == 0.0 {
            f64::MAX
        } else {
            actual / maximum
        };
        *maximum_violation = (*maximum_violation).max(ratio - 1.0);
        violations.push(format!("{name}:{actual}:{maximum}"));
    }
}

fn validate_trace(trace: &WaveformTrace) -> Result<(), String> {
    if trace.time_s.len() < 2 {
        return Err("trace requires at least two time samples".into());
    }
    if trace.time_s.iter().any(|time| !time.is_finite()) {
        return Err("trace times must be finite".into());
    }
    for pair in trace.time_s.windows(2) {
        if pair[1] <= pair[0] {
            return Err("trace times must be strictly increasing".into());
        }
        if !(pair[1] - pair[0]).is_finite() {
            return Err("trace time interval overflow".into());
        }
    }
    let channels = channel_index(trace)?;
    if channels.is_empty() {
        return Err("trace requires at least one channel".into());
    }
    for channel in channels.values() {
        if channel.values.len() != trace.time_s.len() {
            return Err(format!(
                "channel {} length does not match time grid",
                channel.channel_id
            ));
        }
        if channel.unit.trim().is_empty() || channel.values.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "channel {} has invalid unit or non-finite values",
                channel.channel_id
            ));
        }
    }
    Ok(())
}

fn channel_index(
    trace: &WaveformTrace,
) -> Result<std::collections::BTreeMap<String, &TraceChannel>, String> {
    let mut result = std::collections::BTreeMap::new();
    for channel in &trace.channels {
        if channel.channel_id.trim().is_empty() {
            return Err("channel IDs must be non-empty".into());
        }
        if result.insert(channel.channel_id.clone(), channel).is_some() {
            return Err(format!("duplicate channel ID {}", channel.channel_id));
        }
    }
    Ok(result)
}

fn integrate_wh(time_s: &[f64], power_w: &[f64]) -> Result<f64, String> {
    if time_s.len() != power_w.len() || time_s.len() < 2 {
        return Err("power and time samples must have equal length >= 2".into());
    }
    let mut watt_seconds = 0.0_f64;
    for index in 0..(time_s.len() - 1) {
        let dt = time_s[index + 1] - time_s[index];
        let average_power = 0.5 * power_w[index] + 0.5 * power_w[index + 1];
        let increment = dt * average_power;
        if !increment.is_finite() {
            return Err("energy integration overflow".into());
        }
        watt_seconds += increment;
        if !watt_seconds.is_finite() {
            return Err("energy accumulation overflow".into());
        }
    }
    Ok(watt_seconds / 3600.0)
}

fn peak_absolute(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |peak, value| peak.max(value.abs()))
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn trace(current: Vec<f64>, p1: Vec<f64>, p2: Vec<f64>) -> WaveformTrace {
        WaveformTrace {
            time_s: vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5],
            channels: vec![
                TraceChannel {
                    channel_id: "tx_current_a".into(),
                    unit: "A".into(),
                    values: current,
                },
                TraceChannel {
                    channel_id: "rx1_power_w".into(),
                    unit: "W".into(),
                    values: p1,
                },
                TraceChannel {
                    channel_id: "rx2_power_w".into(),
                    unit: "W".into(),
                    values: p2,
                },
            ],
        }
    }

    fn digest_for_test(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn benchmark_suite() -> BenchmarkSuite {
        let reference = trace(
            vec![0.0, 4.0, 8.0, 6.0, 4.0, 4.0],
            vec![0.0, 100.0, 250.0, 250.0, 250.0, 250.0],
            vec![0.0, 30.0, 80.0, 80.0, 80.0, 80.0],
        );
        let mut near = reference.clone();
        near.channels[0].values[2] = 8.02;
        near.channels[1].values[2] = 249.0;
        near.channels[2].values[2] = 80.3;
        let mut far = reference.clone();
        far.channels[0].values[2] = 8.8;
        far.channels[1].values[2] = 270.0;
        far.channels[2].values[2] = 74.0;

        BenchmarkSuite {
            schema_version: SUITE_SCHEMA_VERSION.into(),
            suite_id: "SYNTHETIC-TRANSIENT-1T2R".into(),
            cases: vec![BenchmarkCase {
                case_id: "LOAD-STEP-001".into(),
                evidence_class: EvidenceClass::SyntheticKnownTruth,
                power_flow_direction: PowerFlowDirection::GridToVehicle,
                execution_environment_digest: digest('9'),
                reference_runtime_median_ms: 100.0,
                reference_runtime_p95_ms: 120.0,
                reference,
                energy_channel_ids: vec!["rx1_power_w".into(), "rx2_power_w".into()],
                candidates: vec![
                    ModelCandidate {
                        candidate_id: "full-order".into(),
                        family: ModelFamily::FullOrder,
                        state_order: Some(16),
                        model_digest: digest('a'),
                        training_data_digest: digest('b'),
                        execution_environment_digest: digest('9'),
                        runtime_median_ms: 100.0,
                        runtime_p95_ms: 120.0,
                        peak_memory_bytes: 64000000,
                        trace: trace(
                            vec![0.0, 4.0, 8.0, 6.0, 4.0, 4.0],
                            vec![0.0, 100.0, 250.0, 250.0, 250.0, 250.0],
                            vec![0.0, 30.0, 80.0, 80.0, 80.0, 80.0],
                        ),
                    },
                    ModelCandidate {
                        candidate_id: "balanced-truncation".into(),
                        family: ModelFamily::BalancedTruncation,
                        state_order: Some(4),
                        model_digest: digest('c'),
                        training_data_digest: digest('d'),
                        execution_environment_digest: digest('9'),
                        runtime_median_ms: 10.0,
                        runtime_p95_ms: 12.0,
                        peak_memory_bytes: 8000000,
                        trace: near,
                    },
                    ModelCandidate {
                        candidate_id: "learned-surrogate".into(),
                        family: ModelFamily::LearnedSurrogate,
                        state_order: None,
                        model_digest: digest('e'),
                        training_data_digest: digest('f'),
                        execution_environment_digest: digest('9'),
                        runtime_median_ms: 3.0,
                        runtime_p95_ms: 4.0,
                        peak_memory_bytes: 12000000,
                        trace: far,
                    },
                ],
                acceptance: AcceptanceThresholds {
                    maximum_channel_nrmse: 0.03,
                    maximum_peak_magnitude_relative_error: 0.03,
                    maximum_energy_relative_error: 0.02,
                    peak_limits: vec![PeakLimit {
                        channel_id: "tx_current_a".into(),
                        maximum_absolute_value: 10.0,
                    }],
                },
            }],
        }
    }

    #[test]
    fn integrates_receiver_energy_in_wh_with_trapezoidal_rule() {
        let trace = trace(
            vec![0.0, 4.0, 8.0, 6.0, 4.0, 4.0],
            vec![0.0, 100.0, 250.0, 250.0, 250.0, 250.0],
            vec![0.0, 30.0, 80.0, 80.0, 80.0, 80.0],
        );
        let energy = integrate_wh(&trace.time_s, &trace.channels[1].values).unwrap();
        assert!((energy - (97.5 / 3600.0)).abs() < 1e-15);
    }

    #[test]
    fn feasibility_first_ranking_keeps_receiver_energy_as_separate_channels() {
        let report = benchmark_suite().evaluate().unwrap();
        assert_eq!(
            report.ranking,
            vec![vec![
                "full-order",
                "balanced-truncation",
                "learned-surrogate"
            ]]
        );
        assert_eq!(report.scores.len(), 3);
        assert!(report.scores[0].feasible);
        assert!(report.scores[1].feasible);
        assert!(!report.scores[2].feasible);
        assert_eq!(report.scores[0].energy_metrics.len(), 2);
        assert_eq!(report.scores[0].energy_metrics[0].channel_id, "rx1_power_w");
        assert_eq!(report.scores[0].energy_metrics[1].channel_id, "rx2_power_w");
    }

    #[test]
    fn marks_learned_model_infeasible_when_receiver_peak_limit_is_exceeded() {
        let mut suite = benchmark_suite();
        suite.cases[0].acceptance.peak_limits[0].maximum_absolute_value = 8.5;
        let report = suite.evaluate().unwrap();
        let learned = report
            .scores
            .iter()
            .find(|score| score.candidate_id == "learned-surrogate")
            .unwrap();
        assert!(!learned.feasible);
        assert!(
            learned
                .violations
                .iter()
                .any(|v| v.starts_with("PEAK_LIMIT_EXCEEDED"))
        );
    }

    #[test]
    fn rejects_time_grid_mismatch_and_unsorted_time() {
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].trace.time_s[2] = 0.21;
        assert!(suite.validate().unwrap_err().contains("time grid differs"));
        let mut suite = benchmark_suite();
        suite.cases[0].reference.time_s.swap(1, 2);
        assert!(
            suite
                .validate()
                .unwrap_err()
                .contains("strictly increasing")
        );
    }

    #[test]
    fn rejects_missing_channels_duplicate_channels_and_unit_mismatch() {
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].trace.channels.pop();
        assert!(
            suite
                .validate()
                .unwrap_err()
                .contains("channel counts differ")
        );
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].trace.channels[0].unit = "mA".into();
        assert!(suite.validate().unwrap_err().contains("unit mismatch"));
        let mut suite = benchmark_suite();
        let repeated_channel = suite.cases[0].reference.channels[0].clone();
        suite.cases[0].reference.channels.push(repeated_channel);
        assert!(suite.validate().unwrap_err().contains("duplicate channel"));
    }

    #[test]
    fn rejects_missing_or_invalid_model_provenance_digests() {
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].model_digest = "unsigned".into();
        assert!(suite.validate().unwrap_err().contains("digests"));
    }

    #[test]
    fn rejects_nonfinite_waveforms_and_nonpositive_state_order() {
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].trace.channels[0].values[0] = f64::NAN;
        assert!(suite.validate().unwrap_err().contains("non-finite"));
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].state_order = Some(0);
        assert!(suite.validate().unwrap_err().contains("state_order"));
    }

    #[test]
    fn rejects_runtime_environment_mismatch_and_invalid_percentile_order() {
        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].execution_environment_digest = digest_for_test('8');
        assert!(
            suite
                .validate()
                .unwrap_err()
                .contains("execution environments differ")
        );

        let mut suite = benchmark_suite();
        suite.cases[0].candidates[0].runtime_p95_ms = 99.0;
        assert!(
            suite
                .validate()
                .unwrap_err()
                .contains("runtime distribution")
        );
    }

    #[test]
    fn reports_runtime_speedup_without_overriding_accuracy_priority() {
        let report = benchmark_suite().evaluate().unwrap();
        let full = report
            .scores
            .iter()
            .find(|score| score.candidate_id == "full-order")
            .unwrap();
        let reduced = report
            .scores
            .iter()
            .find(|score| score.candidate_id == "balanced-truncation")
            .unwrap();
        assert_eq!(full.speedup_vs_reference, 1.0);
        assert_eq!(reduced.speedup_vs_reference, 10.0);
        assert!(full.feasible && reduced.feasible);
        assert_eq!(report.ranking[0][0], "full-order");
    }

    #[test]
    fn rejects_non_watt_energy_channels_and_missing_state_order_for_rom_families() {
        let mut suite = benchmark_suite();
        let energy_channel_id = suite.cases[0].energy_channel_ids[0].clone();
        for candidate in &mut suite.cases[0].candidates {
            let channel = candidate
                .trace
                .channels
                .iter_mut()
                .find(|channel| channel.channel_id == energy_channel_id)
                .expect("energy channel exists in every candidate");
            channel.unit = "kW".into();
        }
        let reference_channel = suite.cases[0]
            .reference
            .channels
            .iter_mut()
            .find(|channel| channel.channel_id == energy_channel_id)
            .expect("energy channel exists in reference");
        reference_channel.unit = "kW".into();
        assert!(suite.validate().unwrap_err().contains("must use watts"));

        let mut suite = benchmark_suite();
        suite.cases[0].candidates[1].state_order = None;
        assert!(
            suite
                .validate()
                .unwrap_err()
                .contains("state_order is required")
        );
    }

    #[test]
    fn reference_peak_limit_violation_marks_every_candidate_infeasible() {
        let mut suite = benchmark_suite();
        suite.cases[0].acceptance.peak_limits[0].maximum_absolute_value = 7.5;
        let report = suite.evaluate().unwrap();
        assert!(report.scores.iter().all(|score| !score.feasible));
        assert!(report.scores.iter().all(|score| {
            score
                .violations
                .iter()
                .any(|violation| violation.starts_with("REFERENCE_EXCEEDS_LIMIT"))
        }));
    }

    #[test]
    fn synthetic_suite_fixture_matches_schema_version_and_evaluates() {
        const FIXTURE: &str = include_str!(
            "../../../benchmarks/wpt-multifidelity/v1/synthetic-transient-comparison.json"
        );
        const SCHEMA: &str = include_str!("../../../benchmarks/wpt-multifidelity/v1/schema.json");
        let suite: BenchmarkSuite =
            serde_json::from_str(FIXTURE).expect("synthetic benchmark fixture");
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("benchmark JSON schema");
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            SUITE_SCHEMA_VERSION
        );
        assert_eq!(schema["additionalProperties"], false);

        fn required_strings(value: &serde_json::Value) -> Vec<&str> {
            let mut names = value["required"]
                .as_array()
                .expect("schema required fields")
                .iter()
                .map(|field| field.as_str().expect("field name"))
                .collect::<Vec<_>>();
            names.sort_unstable();
            names
        }
        let candidate_required = required_strings(&schema["$defs"]["candidate"]);
        assert!(candidate_required.contains(&"execution_environment_digest"));
        assert!(candidate_required.contains(&"runtime_median_ms"));
        assert!(candidate_required.contains(&"runtime_p95_ms"));
        assert!(candidate_required.contains(&"peak_memory_bytes"));
        let case_required = required_strings(&schema["$defs"]["case"]);
        assert!(case_required.contains(&"execution_environment_digest"));
        assert!(case_required.contains(&"reference_runtime_median_ms"));
        assert!(case_required.contains(&"reference_runtime_p95_ms"));

        let report = suite.evaluate().unwrap();
        assert_eq!(
            report.ranking[0],
            vec![
                "full-order",
                "balanced-truncation",
                "port-hamiltonian",
                "learned-surrogate"
            ]
        );
    }

    #[test]
    fn rejects_undefined_nrmse_for_all_zero_reference_channel() {
        let mut suite = benchmark_suite();
        suite.cases[0].reference.channels[0].values.fill(0.0);
        for candidate in &mut suite.cases[0].candidates {
            candidate.trace.channels[0].values.fill(0.0);
        }
        assert!(suite.evaluate().unwrap_err().contains("all-zero"));
    }
}
