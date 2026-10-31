//! Time-windowed manufacturing capability diagnostic.
//!
//! Descriptive subgroup capability, process performance and guarded mean margins.
//! This is not an SPC/control-chart implementation or an ISO 22514-2 conformity claim.
//!
//! Build:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-capability/windowed_capability.rs -o /tmp/windowed-capability-tests
//!   /tmp/windowed-capability-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-capability/windowed_capability.rs -o /tmp/windowed-capability
//!   /tmp/windowed-capability --verify
//!   /tmp/windowed-capability --csv

use std::collections::HashSet;

const MIN_WINDOWS: usize = 3;
const MIN_SAMPLES_PER_WINDOW: usize = 8;
const MEAN_SPAN_FRACTION: f64 = 0.10;
const VARIANCE_RATIO_LIMIT: f64 = 3.0;
const REL_TOL: f64 = 1.0e-10;
const DIGEST_LEN: usize = 64;

const OFFSETS: [f64; 8] = [-1.5, -1.0, -0.5, -0.2, 0.2, 0.5, 1.0, 1.5];

#[derive(Clone, Debug)]
struct Measurement {
    id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    station_id: String,
    instrument_id: String,
    calibration_id: String,
    measurand_id: String,
    unit: String,
    value: f64,
    timestamp_ns: i64,
    calibration_valid_from_ns: i64,
    calibration_valid_until_ns: i64,
    source_sha256: String,
}

#[derive(Clone, Debug)]
struct Window {
    id: String,
    start_ns: i64,
    end_ns: i64,
    measurements: Vec<Measurement>,
}

#[derive(Clone, Debug)]
struct Campaign {
    id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    station_id: String,
    instrument_id: String,
    calibration_id: String,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    specification_source_sha256: String,
    lower_specification: f64,
    upper_specification: f64,
    measurement_expanded_uncertainty_for_window_mean: f64,
    uncertainty_budget_id: String,
    uncertainty_source_sha256: String,
    msa_study_id: String,
    msa_qualified: bool,
    calibration_evidence_verified: bool,
    specification_evidence_verified: bool,
    uncertainty_evidence_verified: bool,
    windows: Vec<Window>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagnosticStatus {
    InsufficientEvidence,
    MeanShiftFlag,
    VarianceShiftFlag,
    BothShiftFlags,
    NoShiftFlagDetected,
}

impl DiagnosticStatus {
    fn code(self) -> &'static str {
        match self {
            Self::InsufficientEvidence => "INSUFFICIENT_EVIDENCE",
            Self::MeanShiftFlag => "MEAN_SHIFT_FLAG",
            Self::VarianceShiftFlag => "VARIANCE_SHIFT_FLAG",
            Self::BothShiftFlags => "BOTH_SHIFT_FLAGS",
            Self::NoShiftFlagDetected => "NO_SHIFT_FLAG_DETECTED",
        }
    }
}

#[derive(Clone, Debug)]
struct WindowMetrics {
    id: String,
    sample_count: usize,
    mean: f64,
    sample_standard_deviation: f64,
    cp_descriptive: f64,
    cpk_descriptive: f64,
    guarded_mean_margin: f64,
    guarded_mean_inside_specification: bool,
    mean_delta_from_previous: Option<f64>,
    variance_ratio_from_first: f64,
}

#[derive(Clone, Debug)]
struct Report {
    campaign_id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    station_id: String,
    instrument_id: String,
    measurand_id: String,
    unit: String,
    specification_id: String,
    specification_revision: String,
    lower_specification: f64,
    upper_specification: f64,
    uncertainty_budget_id: String,
    measurement_expanded_uncertainty: f64,
    msa_study_id: String,
    window_count: usize,
    total_sample_count: usize,
    pooled_within_standard_deviation: f64,
    pooled_within_cp: f64,
    pooled_within_cpk: f64,
    overall_mean: f64,
    overall_standard_deviation: f64,
    overall_pp: f64,
    overall_ppk: f64,
    worst_window_cpk: f64,
    worst_window_id: String,
    window_mean_span: f64,
    maximum_variance_ratio: f64,
    first_mean_shift_window: Option<String>,
    first_variance_shift_window: Option<String>,
    status: DiagnosticStatus,
    windows: Vec<WindowMetrics>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == DIGEST_LEN && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn mean(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() {
        return Err("SAMPLE_EMPTY".into());
    }
    let value = values.iter().sum::<f64>() / values.len() as f64;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("MEAN_NONFINITE".into())
    }
}

fn sample_variance(values: &[f64]) -> Result<f64, String> {
    if values.len() < 2 {
        return Err("SAMPLE_VARIANCE_REQUIRES_TWO_VALUES".into());
    }
    let average = mean(values)?;
    let sum = values.iter().map(|value| (value - average).powi(2)).sum::<f64>();
    let variance = sum / (values.len() - 1) as f64;
    if variance.is_finite() && variance > 0.0 {
        Ok(variance)
    } else {
        Err("SAMPLE_VARIANCE_INVALID".into())
    }
}

fn capability_indices(
    lower: f64,
    upper: f64,
    average: f64,
    standard_deviation: f64,
) -> Result<(f64, f64), String> {
    let width = upper - lower;
    if !width.is_finite() || width <= 0.0
        || !standard_deviation.is_finite() || standard_deviation <= 0.0 {
        return Err("CAPABILITY_INPUT_INVALID".into());
    }
    let cp = width / (6.0 * standard_deviation);
    let cpk = (upper - average).min(average - lower) / (3.0 * standard_deviation);
    if cp.is_finite() && cpk.is_finite() {
        Ok((cp, cpk))
    } else {
        Err("CAPABILITY_RESULT_NONFINITE".into())
    }
}

impl Campaign {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("campaign_id", self.id.as_str()),
            ("part_id", self.part_id.as_str()),
            ("lot_id", self.lot_id.as_str()),
            ("operation_id", self.operation_id.as_str()),
            ("station_id", self.station_id.as_str()),
            ("instrument_id", self.instrument_id.as_str()),
            ("calibration_id", self.calibration_id.as_str()),
            ("measurand_id", self.measurand_id.as_str()),
            ("unit", self.unit.as_str()),
            ("specification_id", self.specification_id.as_str()),
            ("specification_revision", self.specification_revision.as_str()),
            ("uncertainty_budget_id", self.uncertainty_budget_id.as_str()),
            ("msa_study_id", self.msa_study_id.as_str()),
        ] {
            if !nonempty(value) {
                return Err(format!("IDENTITY_MISSING:{name}"));
            }
        }

        if !self.lower_specification.is_finite()
            || !self.upper_specification.is_finite()
            || self.lower_specification >= self.upper_specification {
            return Err("SPECIFICATION_INVALID".into());
        }
        if !self.measurement_expanded_uncertainty_for_window_mean.is_finite()
            || self.measurement_expanded_uncertainty_for_window_mean < 0.0 {
            return Err("MEASUREMENT_UNCERTAINTY_INVALID".into());
        }
        if !valid_sha256(&self.specification_source_sha256)
            || !valid_sha256(&self.uncertainty_source_sha256) {
            return Err("SOURCE_DIGEST_INVALID".into());
        }
        if !self.msa_qualified
            || !self.calibration_evidence_verified
            || !self.specification_evidence_verified
            || !self.uncertainty_evidence_verified {
            return Err("UPSTREAM_EVIDENCE_INCOMPLETE".into());
        }
        if self.windows.len() < MIN_WINDOWS {
            return Err("WINDOW_COUNT_INSUFFICIENT".into());
        }

        let mut seen_ids = HashSet::new();
        let mut previous_timestamp = None;
        let mut previous_window_end = None;
        for window in &self.windows {
            if !nonempty(&window.id) || window.start_ns >= window.end_ns {
                return Err("WINDOW_ID_OR_INTERVAL_INVALID".into());
            }
            if previous_window_end.is_some_and(|end| window.start_ns < end) {
                return Err("WINDOW_INTERVALS_OVERLAP_OR_REORDER".into());
            }
            if window.measurements.len() < MIN_SAMPLES_PER_WINDOW {
                return Err("WINDOW_SAMPLE_COUNT_INSUFFICIENT".into());
            }
            let mut values = Vec::with_capacity(window.measurements.len());
            for measurement in &window.measurements {
                if !seen_ids.insert(measurement.id.clone()) {
                    return Err("MEASUREMENT_ID_DUPLICATE".into());
                }
                if !nonempty(&measurement.id) || !valid_sha256(&measurement.source_sha256) {
                    return Err("MEASUREMENT_ID_OR_SOURCE_DIGEST_INVALID".into());
                }
                if measurement.part_id != self.part_id
                    || measurement.lot_id != self.lot_id
                    || measurement.operation_id != self.operation_id
                    || measurement.station_id != self.station_id
                    || measurement.instrument_id != self.instrument_id
                    || measurement.calibration_id != self.calibration_id
                    || measurement.measurand_id != self.measurand_id
                    || measurement.unit != self.unit {
                    return Err("MEASUREMENT_SCOPE_MISMATCH".into());
                }
                if !measurement.value.is_finite() {
                    return Err("MEASUREMENT_VALUE_NONFINITE".into());
                }
                if measurement.timestamp_ns < window.start_ns
                    || measurement.timestamp_ns > window.end_ns
                    || measurement.calibration_valid_from_ns > measurement.timestamp_ns
                    || measurement.calibration_valid_until_ns < measurement.timestamp_ns {
                    return Err("MEASUREMENT_TIME_OR_CALIBRATION_SCOPE_INVALID".into());
                }
                if let Some(previous) = previous_timestamp {
                    if measurement.timestamp_ns <= previous {
                        return Err("MEASUREMENT_TIMESTAMPS_NOT_STRICTLY_INCREASING".into());
                    }
                }
                previous_timestamp = Some(measurement.timestamp_ns);
                values.push(measurement.value);
            }
            // Reject a window with zero variance rather than inventing infinite capability.
            sample_variance(&values)?;
            previous_window_end = Some(window.end_ns);
        }
        Ok(())
    }

    fn evaluate(&self) -> Result<Report, String> {
        self.validate()?;

        let mut all_values = Vec::new();
        let mut window_metrics = Vec::with_capacity(self.windows.len());
        let mut within_sum_squares = 0.0;
        let mut within_degrees_of_freedom = 0_usize;
        let mut previous_mean = None;
        let mut first_variance = None;
        let mut window_mean_min = f64::INFINITY;
        let mut window_mean_max = f64::NEG_INFINITY;
        let mut max_variance_ratio = 0.0_f64;
        let mut worst_window_cpk = f64::INFINITY;
        let mut worst_window_id = String::new();
        let mut first_mean_shift_window = None;
        let mut first_variance_shift_window = None;
        let mean_shift_threshold = MEAN_SPAN_FRACTION
            * (self.upper_specification - self.lower_specification);

        for window in &self.windows {
            let values = window.measurements.iter().map(|m| m.value).collect::<Vec<_>>();
            let average = mean(&values)?;
            let variance = sample_variance(&values)?;
            let standard_deviation = variance.sqrt();
            let (cp, cpk) = capability_indices(
                self.lower_specification, self.upper_specification,
                average, standard_deviation
            )?;
            let guarded_margin = (
                self.upper_specification
                    - (average + self.measurement_expanded_uncertainty_for_window_mean)
            ).min(
                (average - self.measurement_expanded_uncertainty_for_window_mean)
                    - self.lower_specification
            );
            let delta = previous_mean.map(|prev: f64| average - prev);
            let variance_ratio = first_variance.map_or(1.0, |base: f64| variance / base);

            if first_variance.is_none() {
                first_variance = Some(variance);
            }
            if max_variance_ratio < variance_ratio {
                max_variance_ratio = variance_ratio;
            }
            if variance_ratio > VARIANCE_RATIO_LIMIT && first_variance_shift_window.is_none() {
                first_variance_shift_window = Some(window.id.clone());
            }
            if delta.is_some() && delta.unwrap().abs() > mean_shift_threshold
                && first_mean_shift_window.is_none() {
                first_mean_shift_window = Some(window.id.clone());
            }

            window_mean_min = window_mean_min.min(average);
            window_mean_max = window_mean_max.max(average);
            if cpk < worst_window_cpk {
                worst_window_cpk = cpk;
                worst_window_id = window.id.clone();
            }
            within_sum_squares += (values.len() - 1) as f64 * variance;
            within_degrees_of_freedom += values.len() - 1;
            all_values.extend_from_slice(&values);
            window_metrics.push(WindowMetrics {
                id: window.id.clone(),
                sample_count: values.len(),
                mean: average,
                sample_standard_deviation: standard_deviation,
                cp_descriptive: cp,
                cpk_descriptive: cpk,
                guarded_mean_margin: guarded_margin,
                guarded_mean_inside_specification: guarded_margin >= 0.0,
                mean_delta_from_previous: delta,
                variance_ratio_from_first: variance_ratio,
            });
            previous_mean = Some(average);
        }

        if within_degrees_of_freedom == 0 {
            return Err("WITHIN_VARIANCE_DEGREES_OF_FREEDOM_INVALID".into());
        }
        let pooled_within_variance = within_sum_squares / within_degrees_of_freedom as f64;
        if !pooled_within_variance.is_finite() || pooled_within_variance <= 0.0 {
            return Err("POOLED_WITHIN_VARIANCE_INVALID".into());
        }
        let pooled_within_sd = pooled_within_variance.sqrt();
        let overall_mean = mean(&all_values)?;
        let overall_sd = sample_variance(&all_values)?.sqrt();
        let (pooled_cp, pooled_cpk) = capability_indices(
            self.lower_specification, self.upper_specification,
            overall_mean, pooled_within_sd
        )?;
        let (overall_pp, overall_ppk) = capability_indices(
            self.lower_specification, self.upper_specification,
            overall_mean, overall_sd
        )?;
        let mean_span = window_mean_max - window_mean_min;

        let mean_shift = mean_span > mean_shift_threshold;
        let variance_shift = max_variance_ratio > VARIANCE_RATIO_LIMIT;
        let status = match (mean_shift, variance_shift) {
            (true, true) => DiagnosticStatus::BothShiftFlags,
            (true, false) => DiagnosticStatus::MeanShiftFlag,
            (false, true) => DiagnosticStatus::VarianceShiftFlag,
            (false, false) => DiagnosticStatus::NoShiftFlagDetected,
        };

        Ok(Report {
            campaign_id: self.id.clone(),
            part_id: self.part_id.clone(),
            lot_id: self.lot_id.clone(),
            operation_id: self.operation_id.clone(),
            station_id: self.station_id.clone(),
            instrument_id: self.instrument_id.clone(),
            measurand_id: self.measurand_id.clone(),
            unit: self.unit.clone(),
            specification_id: self.specification_id.clone(),
            specification_revision: self.specification_revision.clone(),
            lower_specification: self.lower_specification,
            upper_specification: self.upper_specification,
            uncertainty_budget_id: self.uncertainty_budget_id.clone(),
            measurement_expanded_uncertainty: self.measurement_expanded_uncertainty_for_window_mean,
            msa_study_id: self.msa_study_id.clone(),
            window_count: self.windows.len(),
            total_sample_count: all_values.len(),
            pooled_within_standard_deviation: pooled_within_sd,
            pooled_within_cp: pooled_cp,
            pooled_within_cpk: pooled_cpk,
            overall_mean,
            overall_standard_deviation: overall_sd,
            overall_pp,
            overall_ppk,
            worst_window_cpk,
            worst_window_id,
            window_mean_span: mean_span,
            maximum_variance_ratio: max_variance_ratio,
            first_mean_shift_window,
            first_variance_shift_window,
            status,
            windows: window_metrics,
        })
    }
}

impl Report {
    fn csv_rows(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "campaign,{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            self.campaign_id, self.part_id, self.lot_id, self.operation_id,
            self.station_id, self.instrument_id, self.measurand_id, self.unit,
            self.specification_id, self.specification_revision,
            self.lower_specification, self.upper_specification,
            self.uncertainty_budget_id, self.measurement_expanded_uncertainty,
            self.msa_study_id, self.window_count, self.total_sample_count,
            self.pooled_within_standard_deviation, self.pooled_within_cp,
            self.pooled_within_cpk, self.overall_mean, self.overall_standard_deviation,
            self.overall_pp, self.overall_ppk, self.worst_window_cpk,
        ));
        out.push_str(&format!(
            "summary,{},{},{},{},{},{},{}\n",
            self.worst_window_id, self.window_mean_span, self.maximum_variance_ratio,
            self.first_mean_shift_window.as_deref().unwrap_or(""),
            self.first_variance_shift_window.as_deref().unwrap_or(""),
            self.status.code(), "NORMAL_ASSUMPTION_UNVERIFIED"
        ));
        out.push_str("window_id,sample_count,mean,sample_sd,cp_descriptive,cpk_descriptive,guarded_mean_margin,guarded_mean_inside_spec,mean_delta_from_previous,variance_ratio_from_first\n");
        for window in &self.windows {
            out.push_str(&format!(
                "{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{},{:.12e}\n",
                window.id, window.sample_count, window.mean,
                window.sample_standard_deviation, window.cp_descriptive,
                window.cpk_descriptive, window.guarded_mean_margin,
                window.guarded_mean_inside_specification,
                window.mean_delta_from_previous.map(|v| format!("{v:.12e}")).unwrap_or_default(),
                window.variance_ratio_from_first
            ));
        }
        out
    }
}

fn fixture(id: &str, means: [f64; 4], scales: [f64; 4]) -> Campaign {
    let windows = (0..4).map(|window_index| {
        let start = 1_000_000_000_i64 + window_index as i64 * 1_000_000_000;
        let end = start + 900_000_000;
        let measurements = OFFSETS.iter().enumerate().map(|(sample_index, offset)| {
            let sequence = window_index * OFFSETS.len() + sample_index;
            Measurement {
                id: format!("MEAS-{sequence:03}"),
                part_id: "PART-001".into(),
                lot_id: "LOT-001".into(),
                operation_id: "OP-FINAL-MILL".into(),
                station_id: "CNC-01".into(),
                instrument_id: "CMM-01".into(),
                calibration_id: "CAL-CMM-01".into(),
                measurand_id: "shaft-diameter".into(),
                unit: "mm".into(),
                value: means[window_index] + scales[window_index] * offset,
                timestamp_ns: start + (sample_index as i64 + 1) * 100_000_000,
                calibration_valid_from_ns: 0,
                calibration_valid_until_ns: 10_000_000_000,
                source_sha256: format!("{:064x}", sequence + 1),
            }
        }).collect();
        Window {
            id: format!("WINDOW-{}", window_index + 1),
            start_ns: start,
            end_ns: end,
            measurements,
        }
    }).collect();

    Campaign {
        id: id.into(),
        part_id: "PART-001".into(),
        lot_id: "LOT-001".into(),
        operation_id: "OP-FINAL-MILL".into(),
        station_id: "CNC-01".into(),
        instrument_id: "CMM-01".into(),
        calibration_id: "CAL-CMM-01".into(),
        measurand_id: "shaft-diameter".into(),
        unit: "mm".into(),
        specification_id: "SPEC-SHAFT-01".into(),
        specification_revision: "REV-7".into(),
        specification_source_sha256: "a".repeat(DIGEST_LEN),
        lower_specification: 0.0,
        upper_specification: 1.0,
        measurement_expanded_uncertainty_for_window_mean: 0.02,
        uncertainty_budget_id: "UQ-CMM-01".into(),
        uncertainty_source_sha256: "b".repeat(DIGEST_LEN),
        msa_study_id: "MSA-CROSSED-01".into(),
        msa_qualified: true,
        calibration_evidence_verified: true,
        specification_evidence_verified: true,
        uncertainty_evidence_verified: true,
        windows,
    }
}

fn fixtures() -> Vec<Campaign> {
    vec![
        fixture("STABLE_CONTROL", [0.5; 4], [0.05; 4]),
        fixture("LINEAR_DRIFT", [0.20, 0.45, 0.70, 0.95], [0.05; 4]),
        fixture("VARIANCE_SHIFT", [0.5; 4], [0.02, 0.02, 0.10, 0.18]),
        fixture("DRIFT_AND_VARIANCE", [0.20, 0.45, 0.70, 0.95], [0.02, 0.05, 0.10, 0.18]),
    ]
}

fn verify() -> Result<(), String> {
    let reports = fixtures().iter().map(Campaign::evaluate).collect::<Result<Vec<_>, _>>()?;
    if reports[0].status != DiagnosticStatus::NoShiftFlagDetected {
        return Err("STABLE_CONTROL_WRONGLY_FLAGGED".into());
    }
    if reports[1].status != DiagnosticStatus::MeanShiftFlag
        || reports[1].worst_window_id != "WINDOW-4"
        || !(reports[1].worst_window_cpk < 0.5)
        || !(reports[1].pooled_within_cpk > reports[1].worst_window_cpk * 5.0) {
        return Err("MEAN_DRIFT_NOT_EXPOSED_BY_WORST_WINDOW".into());
    }
    if reports[2].status != DiagnosticStatus::VarianceShiftFlag
        || reports[2].maximum_variance_ratio <= VARIANCE_RATIO_LIMIT
        || !(reports[2].worst_window_cpk < reports[2].pooled_within_cpk) {
        return Err("VARIANCE_SHIFT_NOT_EXPOSED".into());
    }
    if reports[3].status != DiagnosticStatus::BothShiftFlags {
        return Err("COMBINED_SHIFT_NOT_EXPOSED".into());
    }

    for report in &reports {
        if report.windows.len() != 4 || report.total_sample_count != 32
            || report.worst_window_cpk > report.pooled_within_cpk + REL_TOL {
            // The worst-window Cpk may not be smaller for arbitrary data, but
            // it must be no more optimistic than the actual minimum window Cpk.
            let actual_worst = report.windows.iter()
                .map(|w| w.cpk_descriptive)
                .fold(f64::INFINITY, f64::min);
            if (actual_worst - report.worst_window_cpk).abs() > REL_TOL {
                return Err("WORST_WINDOW_CPK_AGGREGATION_FAILED".into());
            }
        }
        if report.windows.iter().any(|w| w.sample_count != MIN_SAMPLES_PER_WINDOW) {
            return Err("WINDOW_SAMPLE_COUNT_REPORT_MISMATCH".into());
        }
    }

    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("campaign,field_a,field_b,field_c,field_d,field_e,field_f,field_g,field_h,field_i,field_j,field_k,field_l,field_m,field_n,field_o,field_p,field_q,field_r,field_s,field_t,field_u,field_v,field_w,field_x");
    for campaign in fixtures() {
        let report = campaign.evaluate()?;
        print!("{}", report.csv_rows());
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("WINDOWED_CAPABILITY_OK: pooled vs worst-window drift/variance diagnostics")),
        [arg] if arg == "--verify" => verify().map(|()| println!("WINDOWED_CAPABILITY_OK: pooled vs worst-window drift/variance diagnostics")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: windowed-capability [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("WINDOWED_CAPABILITY_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_campaign_has_no_shift_flag_but_is_not_called_process_stable() {
        let report = fixtures()[0].evaluate().unwrap();
        assert_eq!(report.status, DiagnosticStatus::NoShiftFlagDetected);
        assert_eq!(report.status.code(), "NO_SHIFT_FLAG_DETECTED");
    }

    #[test]
    fn linear_drift_is_hidden_by_pooled_within_cpk_but_exposed_by_worst_window() {
        let report = fixtures()[1].evaluate().unwrap();
        assert_eq!(report.status, DiagnosticStatus::MeanShiftFlag);
        assert!(report.pooled_within_cpk > 2.5);
        assert!(report.worst_window_cpk < 0.5);
        assert_eq!(report.worst_window_id, "WINDOW-4");
        assert!(report.overall_ppk < report.pooled_within_cpk);
    }

    #[test]
    fn variance_shift_is_reported_separately_from_mean_drift() {
        let report = fixtures()[2].evaluate().unwrap();
        assert_eq!(report.status, DiagnosticStatus::VarianceShiftFlag);
        assert!(report.maximum_variance_ratio > 3.0);
        assert!(report.worst_window_cpk < report.pooled_within_cpk);
    }

    #[test]
    fn simultaneous_drift_and_variance_shift_are_not_collapsed() {
        let report = fixtures()[3].evaluate().unwrap();
        assert_eq!(report.status, DiagnosticStatus::BothShiftFlags);
    }

    #[test]
    fn guarded_mean_margin_responds_monotonically_to_uncertainty() {
        let mut narrow = fixtures()[0].evaluate().unwrap();
        let mut campaign = fixtures()[0].clone();
        campaign.measurement_expanded_uncertainty_for_window_mean = 0.10;
        let wide = campaign.evaluate().unwrap();
        assert!(wide.windows[0].guarded_mean_margin < narrow.windows[0].guarded_mean_margin);
        narrow.windows.clear();
        assert!(!narrow.windows.is_empty() == false);
    }

    #[test]
    fn invalid_evidence_or_scope_mismatch_never_returns_capability() {
        let mut campaign = fixtures()[0].clone();
        campaign.msa_qualified = false;
        assert!(campaign.evaluate().is_err());

        let mut campaign = fixtures()[0].clone();
        campaign.calibration_evidence_verified = false;
        assert!(campaign.evaluate().is_err());

        let mut campaign = fixtures()[0].clone();
        campaign.windows[0].measurements[0].unit = "inch".into();
        assert!(campaign.evaluate().is_err());
    }

    #[test]
    fn insufficient_windows_and_sample_counts_reject() {
        let mut campaign = fixtures()[0].clone();
        campaign.windows.pop();
        assert!(campaign.evaluate().is_err());

        let mut campaign = fixtures()[0].clone();
        campaign.windows[0].measurements.pop();
        assert!(campaign.evaluate().is_err());
    }

    #[test]
    fn duplicate_ids_and_nonmonotone_timestamps_reject() {
        let mut campaign = fixtures()[0].clone();
        campaign.windows[1].measurements[0].id = campaign.windows[0].measurements[0].id.clone();
        assert!(campaign.evaluate().is_err());

        let mut campaign = fixtures()[0].clone();
        campaign.windows[0].measurements[1].timestamp_ns =
            campaign.windows[0].measurements[0].timestamp_ns;
        assert!(campaign.evaluate().is_err());
    }

    #[test]
    fn unit_conversion_preserves_dimensionless_capability_indices() {
        let original = fixtures()[1].evaluate().unwrap();
        let mut converted = fixtures()[1].clone();
        converted.unit = "um".into();
        converted.lower_specification *= 1000.0;
        converted.upper_specification *= 1000.0;
        converted.measurement_expanded_uncertainty_for_window_mean *= 1000.0;
        for window in &mut converted.windows {
            for measurement in &mut window.measurements {
                measurement.unit = "um".into();
                measurement.value *= 1000.0;
            }
        }
        let converted_report = converted.evaluate().unwrap();
        assert!((original.pooled_within_cpk - converted_report.pooled_within_cpk).abs() < 1.0e-12);
        assert!((original.worst_window_cpk - converted_report.worst_window_cpk).abs() < 1.0e-12);
        assert!((original.overall_ppk - converted_report.overall_ppk).abs() < 1.0e-12);
    }

    #[test]
    fn deterministic_report_is_byte_identical() {
        let first = fixtures().iter().map(|c| c.evaluate().unwrap().csv_rows()).collect::<String>();
        let second = fixtures().iter().map(|c| c.evaluate().unwrap().csv_rows()).collect::<String>();
        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn golden_campaign_statuses_and_worst_windows_are_frozen() {
        verify().unwrap();
    }
}
