//! Synthetic known-truth manufacturing twin drift diagnosis.
//!
//! Diagnostic classes distinguish sensor, process-context, model and interface
//! signatures. The evaluator abstains when evidence is insufficient or multiple
//! fault signatures coexist. It never adapts a model or controls equipment.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/drift_diagnosis.rs -o /tmp/mfg-drift-tests
//!   /tmp/mfg-drift-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/drift_diagnosis.rs -o /tmp/mfg-drift
//!   /tmp/mfg-drift

#[derive(Clone, Copy, Debug)]
struct Sample {
    primary_sensor: Option<f64>,
    reference_sensor: Option<f64>,
    twin_prediction: Option<f64>,
    process_input: Option<f64>,
    interface_source_value: Option<f64>,
    interface_scale: Option<f64>,
    interface_offset: Option<f64>,
    interface_mapped_value: Option<f64>,
    source_time_ms: Option<i64>,
    expected_transport_latency_ms: Option<i64>,
    receiver_time_ms: Option<i64>,
    calibration_qualified: bool,
    stale: bool,
    replayed: bool,
}

#[derive(Clone, Copy, Debug)]
struct Thresholds {
    minimum_samples: usize,
    sensor_bias_limit: f64,
    model_residual_limit: f64,
    process_input_min: f64,
    process_input_max: f64,
    interface_value_tolerance: f64,
    clock_residual_limit_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WindowMetrics {
    mean_sensor_reference_residual: f64,
    mean_reference_prediction_residual: f64,
    max_interface_mapping_residual: f64,
    max_clock_residual_ms: u64,
    process_input_min: f64,
    process_input_max: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct DiagnosticReport {
    classification: String,
    sample_count: usize,
    metrics: Option<WindowMetrics>,
    claim_ceiling: &'static str,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            minimum_samples: 3,
            sensor_bias_limit: 0.75,
            model_residual_limit: 1.0,
            process_input_min: 0.0,
            process_input_max: 10.0,
            interface_value_tolerance: 0.01,
            clock_residual_limit_ms: 100,
        }
    }
}

fn empty_report(classification: &str, sample_count: usize) -> DiagnosticReport {
    DiagnosticReport {
        classification: classification.to_string(),
        sample_count,
        metrics: None,
        claim_ceiling: "synthetic_diagnostic_only",
    }
}

fn validate_thresholds(thresholds: Thresholds) -> Result<(), String> {
    if thresholds.minimum_samples == 0
        || !thresholds.sensor_bias_limit.is_finite()
        || thresholds.sensor_bias_limit < 0.0
        || !thresholds.model_residual_limit.is_finite()
        || thresholds.model_residual_limit < 0.0
        || !thresholds.process_input_min.is_finite()
        || !thresholds.process_input_max.is_finite()
        || thresholds.process_input_min >= thresholds.process_input_max
        || !thresholds.interface_value_tolerance.is_finite()
        || thresholds.interface_value_tolerance < 0.0
    {
        return Err("invalid diagnostic thresholds".to_string());
    }
    Ok(())
}

fn required_finite(value: Option<f64>) -> Option<f64> {
    value.filter(|number| number.is_finite())
}

fn mean(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("mean requires at least one value".to_string());
    }
    let result = values.iter().sum::<f64>() / values.len() as f64;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("non-finite diagnostic mean".to_string())
    }
}

fn analyze_window(samples: &[Sample], thresholds: Thresholds) -> Result<DiagnosticReport, String> {
    validate_thresholds(thresholds)?;

    if samples.len() < thresholds.minimum_samples {
        return Ok(empty_report("INSUFFICIENT_SAMPLE_COUNT", samples.len()));
    }

    // A missing or non-finite primary/reference/prediction/context/interface
    // channel makes root-cause attribution incomplete. Never impute it here.
    for sample in samples {
        if required_finite(sample.primary_sensor).is_none()
            || required_finite(sample.reference_sensor).is_none()
            || required_finite(sample.twin_prediction).is_none()
            || required_finite(sample.process_input).is_none()
            || required_finite(sample.interface_source_value).is_none()
            || required_finite(sample.interface_scale).is_none()
            || required_finite(sample.interface_offset).is_none()
            || required_finite(sample.interface_mapped_value).is_none()
            || sample.source_time_ms.is_none()
            || sample.expected_transport_latency_ms.is_none()
            || sample.receiver_time_ms.is_none()
        {
            return Ok(empty_report("INSUFFICIENT_EVIDENCE", samples.len()));
        }
    }

    if samples.iter().any(|sample| !sample.calibration_qualified) {
        return Ok(empty_report("MEASUREMENT_SYSTEM_UNQUALIFIED", samples.len()));
    }
    if samples.iter().any(|sample| sample.stale || sample.replayed) {
        return Ok(empty_report("DATA_INTEGRITY_FAULT", samples.len()));
    }

    let mut sensor_differences = Vec::with_capacity(samples.len());
    let mut model_differences = Vec::with_capacity(samples.len());
    let mut mapping_residuals = Vec::with_capacity(samples.len());
    let mut clock_residuals = Vec::with_capacity(samples.len());
    let mut process_inputs = Vec::with_capacity(samples.len());

    for sample in samples {
        let primary = sample.primary_sensor.expect("finite channel was checked");
        let reference = sample.reference_sensor.expect("finite channel was checked");
        let prediction = sample.twin_prediction.expect("finite channel was checked");
        let process_input = sample.process_input.expect("finite channel was checked");
        let source_value = sample.interface_source_value.expect("finite channel was checked");
        let scale = sample.interface_scale.expect("finite channel was checked");
        let offset = sample.interface_offset.expect("finite channel was checked");
        let mapped = sample.interface_mapped_value.expect("finite channel was checked");
        let expected = source_value * scale + offset;
        if !expected.is_finite() {
            return Ok(empty_report("INSUFFICIENT_EVIDENCE", samples.len()));
        }

        sensor_differences.push((primary - reference).abs());
        model_differences.push((reference - prediction).abs());
        mapping_residuals.push((mapped - expected).abs());
        let expected_receiver_time = sample
            .source_time_ms
            .expect("source time channel was checked")
            .checked_add(sample.expected_transport_latency_ms.expect("latency was checked"))
            .ok_or_else(|| "expected receiver timestamp overflow".to_string())?;
        clock_residuals.push(
            sample
                .receiver_time_ms
                .expect("time channel was checked")
                .abs_diff(expected_receiver_time),
        );
        process_inputs.push(process_input);
    }

    let metrics = WindowMetrics {
        mean_sensor_reference_residual: mean(&sensor_differences)?,
        mean_reference_prediction_residual: mean(&model_differences)?,
        max_interface_mapping_residual: mapping_residuals
            .iter()
            .copied()
            .fold(0.0_f64, f64::max),
        max_clock_residual_ms: clock_residuals.iter().copied().max().unwrap_or(0),
        process_input_min: process_inputs.iter().copied().fold(f64::INFINITY, f64::min),
        process_input_max: process_inputs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    };

    let sensor_bias = metrics.mean_sensor_reference_residual > thresholds.sensor_bias_limit;
    let interface_mapping = metrics.max_interface_mapping_residual > thresholds.interface_value_tolerance;
    let clock_skew = metrics.max_clock_residual_ms > thresholds.clock_residual_limit_ms;
    let out_of_domain = metrics.process_input_min < thresholds.process_input_min
        || metrics.process_input_max > thresholds.process_input_max;
    // A large residual outside the declared process envelope is not attributed
    // to model discrepancy: the model is not qualified for that regime.
    let model_discrepancy = !out_of_domain
        && metrics.mean_reference_prediction_residual > thresholds.model_residual_limit;

    let fault_count = [
        sensor_bias,
        interface_mapping,
        clock_skew,
        out_of_domain,
        model_discrepancy,
    ]
    .iter()
    .filter(|flag| **flag)
    .count();

    let classification = if fault_count > 1 {
        "UNRESOLVED_MULTIPLE_FAULTS"
    } else if sensor_bias {
        "PRIMARY_SENSOR_BIAS"
    } else if interface_mapping {
        "INTERFACE_MAPPING_FAULT"
    } else if clock_skew {
        "CLOCK_SKEW"
    } else if out_of_domain {
        "OUT_OF_VALIDITY_DOMAIN"
    } else if model_discrepancy {
        "MODEL_DISCREPANCY"
    } else {
        "IN_DOMAIN_NO_DETECTED_DRIFT"
    };

    Ok(DiagnosticReport {
        classification: classification.to_string(),
        sample_count: samples.len(),
        metrics: Some(metrics),
        claim_ceiling: "synthetic_diagnostic_only",
    })
}

fn sample(
    primary: f64,
    reference: f64,
    prediction: f64,
    process_input: f64,
) -> Sample {
    Sample {
        primary_sensor: Some(primary),
        reference_sensor: Some(reference),
        twin_prediction: Some(prediction),
        process_input: Some(process_input),
        interface_source_value: Some(5.0),
        interface_scale: Some(1.0),
        interface_offset: Some(273.15),
        interface_mapped_value: Some(278.15),
        source_time_ms: Some(1_000),
        expected_transport_latency_ms: Some(50),
        receiver_time_ms: Some(1_050),
        calibration_qualified: true,
        stale: false,
        replayed: false,
    }
}

fn repeat_sample(value: Sample, count: usize) -> Vec<Sample> {
    vec![value; count]
}

fn baseline_scenario() -> Vec<Sample> {
    vec![
        sample(5.10, 5.00, 5.00, 5.0),
        sample(4.95, 4.98, 5.00, 5.1),
        sample(5.03, 5.02, 5.00, 4.9),
        sample(4.98, 5.01, 5.00, 5.0),
    ]
}

fn sensor_bias_scenario() -> Vec<Sample> {
    repeat_sample(sample(6.5, 5.0, 5.0, 5.0), 4)
}

fn process_context_shift_scenario() -> Vec<Sample> {
    repeat_sample(sample(15.0, 15.0, 5.0, 12.0), 4)
}

fn model_discrepancy_scenario() -> Vec<Sample> {
    repeat_sample(sample(7.0, 7.0, 5.0, 5.0), 4)
}

fn interface_mapping_scenario() -> Vec<Sample> {
    let mut value = sample(5.0, 5.0, 5.0, 5.0);
    value.interface_mapped_value = Some(5.0);
    repeat_sample(value, 4)
}

fn clock_skew_scenario() -> Vec<Sample> {
    let mut value = sample(5.0, 5.0, 5.0, 5.0);
    value.receiver_time_ms = Some(1_500);
    repeat_sample(value, 4)
}

fn multiple_fault_scenario() -> Vec<Sample> {
    repeat_sample(sample(8.0, 7.0, 5.0, 5.0), 4)
}

fn run_report() -> Result<String, String> {
    let thresholds = Thresholds::default();
    let cases = [
        ("baseline", baseline_scenario()),
        ("sensor_bias", sensor_bias_scenario()),
        ("process_context_shift", process_context_shift_scenario()),
        ("model_discrepancy", model_discrepancy_scenario()),
        ("interface_mapping", interface_mapping_scenario()),
        ("clock_skew", clock_skew_scenario()),
        ("multiple_faults", multiple_fault_scenario()),
    ];
    let mut output = String::from("MFG_DRIFT_SYNTHETIC_DIAGNOSTIC version=1\n");
    for (name, samples) in cases {
        let report = analyze_window(&samples, thresholds)?;
        output.push_str(&format!(
            "DIAGNOSTIC case={} classification={} samples={} claim_ceiling={}\n",
            name, report.classification, report.sample_count, report.claim_ceiling
        ));
    }
    output.push_str(
        "CLAIM_CEILING synthetic_known_truth=true real_factory_data=false drift_proven=false model_recalibrated=false machine_actuation=false\n",
    );
    Ok(output)
}

#[cfg(not(test))]
fn main() {
    match run_report() {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-drift-diagnosis: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_fault_control_stays_in_domain() {
        let result = analyze_window(&baseline_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "IN_DOMAIN_NO_DETECTED_DRIFT");
    }

    #[test]
    fn independent_reference_identifies_primary_sensor_bias() {
        let result = analyze_window(&sensor_bias_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "PRIMARY_SENSOR_BIAS");
    }

    #[test]
    fn context_shift_is_not_mislabeled_as_model_discrepancy() {
        let result = analyze_window(&process_context_shift_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "OUT_OF_VALIDITY_DOMAIN");
    }

    #[test]
    fn in_domain_residual_with_agreeing_sensors_is_model_discrepancy() {
        let result = analyze_window(&model_discrepancy_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "MODEL_DISCREPANCY");
    }

    #[test]
    fn mapping_error_is_separated_from_process_and_model_residuals() {
        let result = analyze_window(&interface_mapping_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "INTERFACE_MAPPING_FAULT");
    }

    #[test]
    fn clock_skew_is_separated_from_model_residuals() {
        let result = analyze_window(&clock_skew_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "CLOCK_SKEW");
    }

    #[test]
    fn multiple_simultaneous_signatures_force_abstention() {
        let result = analyze_window(&multiple_fault_scenario(), Thresholds::default()).unwrap();
        assert_eq!(result.classification, "UNRESOLVED_MULTIPLE_FAULTS");
    }

    #[test]
    fn missing_reference_channel_forces_insufficient_evidence() {
        let mut samples = baseline_scenario();
        samples[1].reference_sensor = None;
        let result = analyze_window(&samples, Thresholds::default()).unwrap();
        assert_eq!(result.classification, "INSUFFICIENT_EVIDENCE");
    }

    #[test]
    fn unqualified_calibration_blocks_root_cause_attribution() {
        let mut samples = baseline_scenario();
        samples[0].calibration_qualified = false;
        let result = analyze_window(&samples, Thresholds::default()).unwrap();
        assert_eq!(result.classification, "MEASUREMENT_SYSTEM_UNQUALIFIED");
    }

    #[test]
    fn stale_or_replayed_data_is_not_used_for_drift_attribution() {
        let mut samples = baseline_scenario();
        samples[0].replayed = true;
        let result = analyze_window(&samples, Thresholds::default()).unwrap();
        assert_eq!(result.classification, "DATA_INTEGRITY_FAULT");
    }

    #[test]
    fn insufficient_window_size_forces_abstention() {
        let samples = repeat_sample(sample(5.0, 5.0, 5.0, 5.0), 2);
        let result = analyze_window(&samples, Thresholds::default()).unwrap();
        assert_eq!(result.classification, "INSUFFICIENT_SAMPLE_COUNT");
    }

    #[test]
    fn diagnostic_report_replays_deterministically() {
        assert_eq!(run_report().unwrap(), run_report().unwrap());
    }

    #[test]
    fn invalid_thresholds_are_rejected() {
        let thresholds = Thresholds { process_input_min: 10.0, process_input_max: 0.0, ..Thresholds::default() };
        assert!(analyze_window(&baseline_scenario(), thresholds).is_err());
    }
}
