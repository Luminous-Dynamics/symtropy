//! Deterministic synthetic waveform perturbations for diagnostic-robustness studies.
//!
//! This module transforms explicitly synthetic input arrays. It does not model
//! instrument electronics, reconstruct missing measurements, or establish
//! physical failure distributions. Every output is permanently labelled
//! synthetic and cannot be promoted to measured evidence.

use super::{ReplayError, digest_serialized, non_empty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const WAVEFORM_SCHEMA_VERSION: &str = "luminous.plasma-synthetic-waveform.v0.1";
pub const WAVEFORM_EVIDENCE_CLASS: &str = "SYNTHETIC_WAVEFORM_FAULT_REPLAY";
pub const WAVEFORM_CLAIM_CEILING: &str =
    "DETERMINISTIC_SYNTHETIC_TRANSFORM_ONLY_NO_PHYSICAL_DIAGNOSTIC_OR_FAILURE_RATE_CLAIM";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveformRequest {
    pub schema_version: String,
    pub scenario_id: String,
    pub channel_id: String,
    pub units: String,
    pub time_origin_ns: u64,
    pub sample_interval_ns: u64,
    pub input_values: Vec<f64>,
    pub faults: Vec<WaveformFault>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveformFault {
    pub fault_id: String,
    /// Half-open sample interval [start_sample, end_sample).
    pub start_sample: usize,
    pub end_sample: usize,
    pub mode: WaveformFaultMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum WaveformFaultMode {
    Dropout,
    IntegrityFailure,
    Delay { lag_samples: usize },
    Saturation { lower: f64, upper: f64 },
    Bias { offset: f64 },
    GainDrift { start_gain: f64, end_gain: f64 },
    StaleCalibration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveformSampleState {
    SyntheticNominal,
    Unavailable,
    Invalid,
    Delayed,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveformSample {
    pub sample_index: usize,
    pub time_ns: u64,
    pub state: WaveformSampleState,
    /// None means that no numeric value is available/valid for this sample.
    pub value: Option<f64>,
    /// For delayed samples, identifies the earlier input index used.
    pub source_sample_index: Option<usize>,
    pub active_fault_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveformReceiptBody {
    pub schema_version: String,
    pub scenario_id: String,
    pub channel_id: String,
    pub units: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub claim_ceiling: String,
    pub input_sha256: String,
    pub fault_schedule_sha256: String,
    pub time_origin_ns: u64,
    pub sample_interval_ns: u64,
    pub sample_count: usize,
    pub samples: Vec<WaveformSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveformReceipt {
    pub body: WaveformReceiptBody,
    pub receipt_sha256: String,
}

fn waveform_error(message: impl Into<String>) -> ReplayError {
    ReplayError::new(message)
}

fn validate_request(request: &WaveformRequest) -> Result<(), ReplayError> {
    if request.schema_version != WAVEFORM_SCHEMA_VERSION {
        return Err(waveform_error("unsupported waveform schema version"));
    }
    non_empty(&request.scenario_id, "scenario_id")?;
    non_empty(&request.channel_id, "channel_id")?;
    non_empty(&request.units, "units")?;
    if request.sample_interval_ns == 0 {
        return Err(waveform_error("sample_interval_ns must be positive"));
    }
    if request.input_values.is_empty() {
        return Err(waveform_error("input_values must not be empty"));
    }
    if request.input_values.iter().any(|value| !value.is_finite()) {
        return Err(waveform_error("input waveform contains a non-finite value"));
    }
    let last_index = u64::try_from(request.input_values.len() - 1)
        .map_err(|_| waveform_error("sample count exceeds the supported timebase"))?;
    let last_offset = last_index
        .checked_mul(request.sample_interval_ns)
        .ok_or_else(|| waveform_error("sample timebase multiplication overflow"))?;
    request
        .time_origin_ns
        .checked_add(last_offset)
        .ok_or_else(|| waveform_error("sample timebase addition overflow"))?;

    let mut fault_ids = BTreeSet::new();
    for fault in &request.faults {
        non_empty(&fault.fault_id, "fault_id")?;
        if !fault_ids.insert(fault.fault_id.as_str()) {
            return Err(waveform_error(format!(
                "duplicate waveform fault_id: {}",
                fault.fault_id
            )));
        }
        if fault.start_sample >= fault.end_sample {
            return Err(waveform_error(format!(
                "fault {} has an empty or reversed sample interval",
                fault.fault_id
            )));
        }
        if fault.end_sample > request.input_values.len() {
            return Err(waveform_error(format!(
                "fault {} extends beyond the input waveform",
                fault.fault_id
            )));
        }
        match &fault.mode {
            WaveformFaultMode::Dropout
            | WaveformFaultMode::IntegrityFailure
            | WaveformFaultMode::StaleCalibration => {}
            WaveformFaultMode::Delay { lag_samples } => {
                if *lag_samples == 0 {
                    return Err(waveform_error("delay lag_samples must be positive"));
                }
            }
            WaveformFaultMode::Saturation { lower, upper } => {
                if !lower.is_finite() || !upper.is_finite() || lower >= upper {
                    return Err(waveform_error(
                        "saturation bounds must be finite and lower must be below upper",
                    ));
                }
            }
            WaveformFaultMode::Bias { offset } => {
                if !offset.is_finite() {
                    return Err(waveform_error("bias offset must be finite"));
                }
            }
            WaveformFaultMode::GainDrift {
                start_gain,
                end_gain,
            } => {
                if !start_gain.is_finite()
                    || !end_gain.is_finite()
                    || *start_gain <= 0.0
                    || *end_gain <= 0.0
                {
                    return Err(waveform_error(
                        "gain drift factors must be finite and strictly positive",
                    ));
                }
            }
        }
    }

    let mut ordered_faults: Vec<&WaveformFault> = request.faults.iter().collect();
    ordered_faults.sort_by_key(|fault| (fault.start_sample, fault.end_sample));
    for pair in ordered_faults.windows(2) {
        if pair[0].end_sample > pair[1].start_sample {
            return Err(waveform_error(format!(
                "overlapping waveform faults {} and {} affect the same channel",
                pair[0].fault_id, pair[1].fault_id
            )));
        }
    }
    Ok(())
}

/// Applies a deterministic fault schedule to a synthetic one-channel waveform.
///
/// Fault windows are half-open sample intervals. Multiple faults may affect a
/// channel when their sample intervals do not overlap. The returned receipt
/// binds the input and schedule digests and is permanently marked synthetic.
pub fn run_waveform_replay(request: &WaveformRequest) -> Result<WaveformReceipt, ReplayError> {
    validate_request(request)?;

    let input_sha256 = digest_serialized(&request.input_values)?;
    let mut canonical_faults = request.faults.clone();
    canonical_faults
        .sort_by_key(|fault| (fault.start_sample, fault.end_sample, fault.fault_id.clone()));
    let fault_schedule_sha256 = digest_serialized(&canonical_faults)?;
    let ordered_faults: Vec<&WaveformFault> = canonical_faults.iter().collect();

    let mut samples = Vec::with_capacity(request.input_values.len());
    for (sample_index, input_value) in request.input_values.iter().copied().enumerate() {
        let sample_offset = u64::try_from(sample_index)
            .map_err(|_| waveform_error("sample index exceeds the supported timebase"))?
            .checked_mul(request.sample_interval_ns)
            .ok_or_else(|| waveform_error("sample timebase multiplication overflow"))?;
        let time_ns = request
            .time_origin_ns
            .checked_add(sample_offset)
            .ok_or_else(|| waveform_error("sample timebase addition overflow"))?;

        let active = ordered_faults
            .iter()
            .copied()
            .find(|fault| fault.start_sample <= sample_index && sample_index < fault.end_sample);

        let (state, value, source_sample_index, active_fault_id) = match active {
            None => (
                WaveformSampleState::SyntheticNominal,
                Some(input_value),
                Some(sample_index),
                None,
            ),
            Some(fault) => {
                let fault_id = Some(fault.fault_id.clone());
                match &fault.mode {
                    WaveformFaultMode::Dropout => {
                        (WaveformSampleState::Unavailable, None, None, fault_id)
                    }
                    WaveformFaultMode::IntegrityFailure | WaveformFaultMode::StaleCalibration => {
                        (WaveformSampleState::Invalid, None, None, fault_id)
                    }
                    WaveformFaultMode::Delay { lag_samples } => {
                        let source = sample_index.checked_sub(*lag_samples);
                        let delayed_value = source.map(|index| request.input_values[index]);
                        (
                            WaveformSampleState::Delayed,
                            delayed_value,
                            source,
                            fault_id,
                        )
                    }
                    WaveformFaultMode::Saturation { lower, upper } => (
                        WaveformSampleState::Degraded,
                        Some(input_value.clamp(*lower, *upper)),
                        Some(sample_index),
                        fault_id,
                    ),
                    WaveformFaultMode::Bias { offset } => {
                        let value = input_value + offset;
                        if !value.is_finite() {
                            return Err(waveform_error(format!(
                                "bias fault {} produced a non-finite sample",
                                fault.fault_id
                            )));
                        }
                        (
                            WaveformSampleState::Degraded,
                            Some(value),
                            Some(sample_index),
                            fault_id,
                        )
                    }
                    WaveformFaultMode::GainDrift {
                        start_gain,
                        end_gain,
                    } => {
                        let window_len = fault.end_sample - fault.start_sample;
                        let fraction = if window_len <= 1 {
                            0.0
                        } else {
                            (sample_index - fault.start_sample) as f64 / (window_len - 1) as f64
                        };
                        let gain = start_gain + (end_gain - start_gain) * fraction;
                        let value = input_value * gain;
                        if !value.is_finite() {
                            return Err(waveform_error(format!(
                                "gain drift fault {} produced a non-finite sample",
                                fault.fault_id
                            )));
                        }
                        (
                            WaveformSampleState::Degraded,
                            Some(value),
                            Some(sample_index),
                            fault_id,
                        )
                    }
                }
            }
        };

        samples.push(WaveformSample {
            sample_index,
            time_ns,
            state,
            value,
            source_sample_index,
            active_fault_id,
        });
    }

    let body = WaveformReceiptBody {
        schema_version: WAVEFORM_SCHEMA_VERSION.to_owned(),
        scenario_id: request.scenario_id.clone(),
        channel_id: request.channel_id.clone(),
        units: request.units.clone(),
        evidence_class: WAVEFORM_EVIDENCE_CLASS.to_owned(),
        physical_measurements: false,
        claim_ceiling: WAVEFORM_CLAIM_CEILING.to_owned(),
        input_sha256,
        fault_schedule_sha256,
        time_origin_ns: request.time_origin_ns,
        sample_interval_ns: request.sample_interval_ns,
        sample_count: samples.len(),
        samples,
    };
    let receipt_sha256 = digest_serialized(&body)?;
    Ok(WaveformReceipt {
        body,
        receipt_sha256,
    })
}

/// Verifies the immutable synthetic claim ceiling and receipt content digest.
pub fn verify_waveform_receipt(receipt: &WaveformReceipt) -> Result<(), ReplayError> {
    let body = &receipt.body;
    if body.schema_version != WAVEFORM_SCHEMA_VERSION {
        return Err(waveform_error("waveform receipt schema is unsupported"));
    }
    if body.evidence_class != WAVEFORM_EVIDENCE_CLASS
        || body.physical_measurements
        || body.claim_ceiling != WAVEFORM_CLAIM_CEILING
    {
        return Err(waveform_error(
            "waveform receipt evidence class or claim ceiling was changed",
        ));
    }
    non_empty(&body.scenario_id, "scenario_id")?;
    non_empty(&body.channel_id, "channel_id")?;
    non_empty(&body.units, "units")?;
    for (digest, field) in [
        (body.input_sha256.as_str(), "input_sha256"),
        (body.fault_schedule_sha256.as_str(), "fault_schedule_sha256"),
    ] {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(waveform_error(format!(
                "waveform receipt {field} must be 64 lowercase hexadecimal characters",
            )));
        }
    }
    if body.sample_count == 0 || body.samples.len() != body.sample_count {
        return Err(waveform_error("waveform receipt sample count mismatch"));
    }
    if body.sample_interval_ns == 0 {
        return Err(waveform_error(
            "waveform receipt sample interval is invalid",
        ));
    }
    let mut previous_time = None;
    for (index, sample) in body.samples.iter().enumerate() {
        if sample.sample_index != index {
            return Err(waveform_error(
                "waveform receipt sample indices are not contiguous",
            ));
        }
        if !sample.value.is_none_or(f64::is_finite) {
            return Err(waveform_error(
                "waveform receipt contains a non-finite value",
            ));
        }
        let offset = u64::try_from(index)
            .map_err(|_| waveform_error("waveform receipt sample index overflow"))?
            .checked_mul(body.sample_interval_ns)
            .ok_or_else(|| waveform_error("waveform receipt timebase overflow"))?;
        let expected_time = body
            .time_origin_ns
            .checked_add(offset)
            .ok_or_else(|| waveform_error("waveform receipt timebase overflow"))?;
        if sample.time_ns != expected_time {
            return Err(waveform_error(
                "waveform receipt timebase does not match its header",
            ));
        }
        if let Some(previous) = previous_time
            && sample.time_ns <= previous
        {
            return Err(waveform_error(
                "waveform receipt time is not strictly increasing",
            ));
        }
        previous_time = Some(sample.time_ns);
        match sample.state {
            WaveformSampleState::SyntheticNominal => {
                if sample.value.is_none()
                    || sample.source_sample_index != Some(index)
                    || sample.active_fault_id.is_some()
                {
                    return Err(waveform_error(
                        "nominal sample must retain its own value and have no active fault",
                    ));
                }
            }
            WaveformSampleState::Degraded => {
                if sample.value.is_none()
                    || sample.source_sample_index != Some(index)
                    || sample.active_fault_id.as_deref().is_none_or(str::is_empty)
                {
                    return Err(waveform_error(
                        "degraded sample must retain its own value and non-empty fault identity",
                    ));
                }
            }
            WaveformSampleState::Delayed => {
                if sample.value.is_some() != sample.source_sample_index.is_some()
                    || sample.active_fault_id.as_deref().is_none_or(str::is_empty)
                    || sample
                        .source_sample_index
                        .is_some_and(|source| source >= index)
                {
                    return Err(waveform_error(
                        "delayed sample must bind an earlier source and non-empty fault identity",
                    ));
                }
            }
            WaveformSampleState::Unavailable | WaveformSampleState::Invalid => {
                if sample.value.is_some()
                    || sample.source_sample_index.is_some()
                    || sample.active_fault_id.as_deref().is_none_or(str::is_empty)
                {
                    return Err(waveform_error(
                        "unavailable/invalid sample must not expose a value and must identify its fault",
                    ));
                }
            }
        }
    }
    if receipt.receipt_sha256 != digest_serialized(body)? {
        return Err(waveform_error("waveform receipt digest mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn request() -> WaveformRequest {
        WaveformRequest {
            schema_version: WAVEFORM_SCHEMA_VERSION.to_owned(),
            scenario_id: "synthetic-waveform-001".to_owned(),
            channel_id: "density".to_owned(),
            units: "synthetic-unit".to_owned(),
            time_origin_ns: 1_000,
            sample_interval_ns: 10,
            input_values: vec![-2.0, 0.0, 1.0, 2.0, 3.0, 4.0],
            faults: Vec::new(),
        }
    }

    #[test]
    fn deterministic_saturation_bias_and_gain_drift_are_sample_level() {
        let mut request = request();
        request.faults = vec![
            WaveformFault {
                fault_id: "saturation".to_owned(),
                start_sample: 0,
                end_sample: 1,
                mode: WaveformFaultMode::Saturation {
                    lower: -1.0,
                    upper: 1.0,
                },
            },
            WaveformFault {
                fault_id: "bias".to_owned(),
                start_sample: 1,
                end_sample: 2,
                mode: WaveformFaultMode::Bias { offset: 0.5 },
            },
            WaveformFault {
                fault_id: "gain-drift".to_owned(),
                start_sample: 2,
                end_sample: 5,
                mode: WaveformFaultMode::GainDrift {
                    start_gain: 1.0,
                    end_gain: 2.0,
                },
            },
        ];
        let first = run_waveform_replay(&request).expect("valid synthetic replay");
        let second = run_waveform_replay(&request).expect("replay should be deterministic");
        assert_eq!(first, second);
        assert_eq!(first.body.samples[0].value, Some(-1.0));
        assert_eq!(first.body.samples[1].value, Some(0.5));
        assert_eq!(first.body.samples[2].value, Some(1.0));
        assert_eq!(first.body.samples[3].value, Some(3.0));
        assert_eq!(first.body.samples[4].value, Some(6.0));
        assert_eq!(first.body.samples[5].value, Some(4.0));
        assert_eq!(first.body.samples[0].state, WaveformSampleState::Degraded);
        assert_eq!(
            first.body.samples[5].state,
            WaveformSampleState::SyntheticNominal
        );
        verify_waveform_receipt(&first).expect("receipt should verify");
    }

    #[test]
    fn fault_schedule_digest_is_independent_of_non_overlapping_input_order() {
        let mut first = request();
        first.faults = vec![
            WaveformFault {
                fault_id: "late-bias".to_owned(),
                start_sample: 3,
                end_sample: 5,
                mode: WaveformFaultMode::Bias { offset: 0.2 },
            },
            WaveformFault {
                fault_id: "early-clamp".to_owned(),
                start_sample: 0,
                end_sample: 2,
                mode: WaveformFaultMode::Saturation {
                    lower: -1.0,
                    upper: 1.0,
                },
            },
        ];
        let mut second = first.clone();
        second.faults.reverse();
        let receipt_a = run_waveform_replay(&first).expect("first ordering should replay");
        let receipt_b = run_waveform_replay(&second).expect("second ordering should replay");
        assert_eq!(
            receipt_a.body.fault_schedule_sha256,
            receipt_b.body.fault_schedule_sha256
        );
        assert_eq!(receipt_a.body.samples, receipt_b.body.samples);
    }

    #[test]
    fn dropout_integrity_and_stale_calibration_never_emit_values() {
        let mut request = request();
        request.faults = vec![
            WaveformFault {
                fault_id: "dropout".to_owned(),
                start_sample: 0,
                end_sample: 1,
                mode: WaveformFaultMode::Dropout,
            },
            WaveformFault {
                fault_id: "integrity".to_owned(),
                start_sample: 1,
                end_sample: 2,
                mode: WaveformFaultMode::IntegrityFailure,
            },
            WaveformFault {
                fault_id: "stale-cal".to_owned(),
                start_sample: 2,
                end_sample: 3,
                mode: WaveformFaultMode::StaleCalibration,
            },
        ];
        let receipt = run_waveform_replay(&request).expect("valid fault schedule");
        assert_eq!(
            receipt.body.samples[0].state,
            WaveformSampleState::Unavailable
        );
        assert_eq!(receipt.body.samples[1].state, WaveformSampleState::Invalid);
        assert_eq!(receipt.body.samples[2].state, WaveformSampleState::Invalid);
        assert!(
            receipt.body.samples[..3]
                .iter()
                .all(|sample| sample.value.is_none())
        );
        verify_waveform_receipt(&receipt).expect("receipt should verify");
    }

    #[test]
    fn delayed_samples_reference_the_original_input_index() {
        let mut request = request();
        request.faults = vec![WaveformFault {
            fault_id: "one-sample-delay".to_owned(),
            start_sample: 0,
            end_sample: 4,
            mode: WaveformFaultMode::Delay { lag_samples: 1 },
        }];
        let receipt = run_waveform_replay(&request).expect("valid delay");
        assert_eq!(receipt.body.samples[0].value, None);
        assert_eq!(receipt.body.samples[0].source_sample_index, None);
        assert_eq!(receipt.body.samples[1].value, Some(-2.0));
        assert_eq!(receipt.body.samples[1].source_sample_index, Some(0));
        assert_eq!(receipt.body.samples[3].value, Some(1.0));
        assert_eq!(receipt.body.samples[3].source_sample_index, Some(2));
        verify_waveform_receipt(&receipt).expect("receipt should verify");
    }

    #[test]
    fn overlapping_sample_faults_are_refused() {
        let mut request = request();
        request.faults = vec![
            WaveformFault {
                fault_id: "dropout".to_owned(),
                start_sample: 0,
                end_sample: 3,
                mode: WaveformFaultMode::Dropout,
            },
            WaveformFault {
                fault_id: "bias".to_owned(),
                start_sample: 2,
                end_sample: 4,
                mode: WaveformFaultMode::Bias { offset: 0.1 },
            },
        ];
        assert!(run_waveform_replay(&request).is_err());
    }

    #[test]
    fn invalid_parameters_and_non_finite_inputs_are_refused() {
        let mut invalid_parameters = request();
        invalid_parameters.faults.push(WaveformFault {
            fault_id: "zero-delay".to_owned(),
            start_sample: 0,
            end_sample: 1,
            mode: WaveformFaultMode::Delay { lag_samples: 0 },
        });
        assert!(run_waveform_replay(&invalid_parameters).is_err());

        let mut non_finite = request();
        non_finite.faults.clear();
        non_finite.input_values[0] = f64::NAN;
        assert!(run_waveform_replay(&non_finite).is_err());
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct WaveformBenchmarkManifest {
        schema_version: String,
        benchmark_id: String,
        evidence_class: String,
        physical_measurements: bool,
        claim_ceiling: String,
        cases: Vec<WaveformBenchmarkCase>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct WaveformBenchmarkCase {
        case_id: String,
        expected_disposition: String,
        request: WaveformRequest,
        expected_states: Vec<WaveformSampleState>,
        expected_values: Vec<Option<f64>>,
        expected_source_sample_indices: Vec<Option<usize>>,
        expected_active_fault_ids: Vec<Option<String>>,
    }

    #[test]
    fn repository_waveform_fixture_matches_every_sample_and_claim_boundary() {
        let source = include_str!("../../../../benchmarks/plasma/waveform-fault-replay-v0.json");
        let manifest: WaveformBenchmarkManifest =
            serde_json::from_str(source).expect("waveform fixture should deserialize");
        assert_eq!(
            manifest.schema_version,
            "luminous.plasma-synthetic-waveform-benchmark.v0.1"
        );
        assert_eq!(manifest.benchmark_id, "PLASMA-WAVEFORM-FAULT-001");
        assert_eq!(manifest.evidence_class, WAVEFORM_EVIDENCE_CLASS);
        assert!(!manifest.physical_measurements);
        assert_eq!(manifest.claim_ceiling, WAVEFORM_CLAIM_CEILING);
        assert_eq!(manifest.cases.len(), 8);

        let mut case_ids = BTreeSet::new();
        for case in manifest.cases {
            assert!(!case.case_id.trim().is_empty());
            assert!(case_ids.insert(case.case_id.clone()), "duplicate case ID");
            assert_eq!(case.expected_disposition, "COMPLETED");

            let receipt =
                run_waveform_replay(&case.request).expect("fixture replay should complete");
            verify_waveform_receipt(&receipt).expect("fixture receipt should verify");
            assert_eq!(receipt.body.evidence_class, WAVEFORM_EVIDENCE_CLASS);
            assert!(!receipt.body.physical_measurements);
            assert_eq!(receipt.body.claim_ceiling, WAVEFORM_CLAIM_CEILING);

            let sample_count = case.request.input_values.len();
            assert_eq!(receipt.body.samples.len(), sample_count);
            assert_eq!(case.expected_states.len(), sample_count);
            assert_eq!(case.expected_values.len(), sample_count);
            assert_eq!(case.expected_source_sample_indices.len(), sample_count);
            assert_eq!(case.expected_active_fault_ids.len(), sample_count);

            for index in 0..sample_count {
                let actual = &receipt.body.samples[index];
                assert_eq!(actual.state, case.expected_states[index], "{}", case.case_id);
                assert_eq!(
                    actual.source_sample_index,
                    case.expected_source_sample_indices[index],
                    "{} sample {index}",
                    case.case_id
                );
                assert_eq!(
                    actual.active_fault_id, case.expected_active_fault_ids[index],
                    "{} sample {index}",
                    case.case_id
                );
                match (actual.value, case.expected_values[index]) {
                    (Some(actual), Some(expected)) => assert!(
                        (actual - expected).abs() <= 1e-12,
                        "{} sample {index}: expected {expected}, got {actual}",
                        case.case_id
                    ),
                    (None, None) => {}
                    (actual, expected) => panic!(
                        "{} sample {index}: value disposition mismatch: expected {expected:?}, got {actual:?}",
                        case.case_id
                    ),
                }
            }
        }
    }

    #[test]
    fn receipt_tampering_is_detected_and_claim_ceiling_is_fixed() {
        let mut receipt = run_waveform_replay(&request()).expect("valid replay");
        receipt.body.samples[0].value = Some(999.0);
        assert!(verify_waveform_receipt(&receipt).is_err());

        let mut promoted = run_waveform_replay(&request()).expect("valid replay");
        promoted.body.physical_measurements = true;
        promoted.receipt_sha256 = digest_serialized(&promoted.body).expect("digest");
        assert!(verify_waveform_receipt(&promoted).is_err());
    }
}
