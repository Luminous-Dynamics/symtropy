// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Held-out estimator calibration against observed global physical error.
//!
//! TSV v0.1 deliberately avoids permissive CSV parsing. Identifiers cannot
//! contain tabs/newlines, the header is exact, and every row is bound to
//! simulator/reference build/configuration digests and a units profile.
//!
//! This tool measures empirical coverage. It does not turn a local estimator
//! into a mathematical error bound.

#![forbid(unsafe_code)]

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

pub const TSV_HEADER_V1: &str = "sample_id\ttrajectory_family_id\tsplit\tsimulator\tsimulator_version\tbuild_sha256\tconfig_sha256\tmethod\tcontact_regime\testimator_kind\ttime_step_s\tstate_dimension\tconstraint_count\treported_local_error\tobserved_global_error\ttarget_tolerance\twall_time_ms\tpeak_memory_bytes\treference_build_sha256\treference_config_sha256\tunits_profile_id";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatasetSplitV1 {
    Calibration,
    Evaluation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleV1 {
    pub sample_id: String,
    pub trajectory_family_id: String,
    pub split: DatasetSplitV1,
    pub simulator: String,
    pub simulator_version: String,
    pub build_sha256: String,
    pub config_sha256: String,
    pub method: String,
    pub contact_regime: String,
    pub estimator_kind: String,
    pub time_step_s: f64,
    pub state_dimension: usize,
    pub constraint_count: usize,
    pub reported_local_error: Option<f64>,
    pub observed_global_error: f64,
    pub target_tolerance: f64,
    pub wall_time_ms: f64,
    pub peak_memory_bytes: u64,
    pub reference_build_sha256: String,
    pub reference_config_sha256: String,
    pub units_profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetErrorV1 {
    MissingHeader,
    HeaderMismatch,
    BlankRow { line: usize },
    WrongFieldCount { line: usize, expected: usize, actual: usize },
    InvalidField { line: usize, field: &'static str },
    InvalidDigest { line: usize, field: &'static str },
    DuplicateSampleId(String),
    TrajectoryFamilySplitLeakage { family_id: String },
    EstimatorValueMismatch { sample_id: String },
    NoCalibrationSamples { group_key: String },
    NoEvaluationSamples { group_key: String },
    ZeroEstimateUnderpredicts { sample_id: String },
    InvalidCoverageQuantile,
}

impl fmt::Display for DatasetErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DatasetErrorV1 {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_finite_nonnegative(
    value: &str,
    line: usize,
    field: &'static str,
) -> Result<f64, DatasetErrorV1> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| DatasetErrorV1::InvalidField { line, field })?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(DatasetErrorV1::InvalidField { line, field });
    }
    Ok(parsed)
}

fn parse_positive(
    value: &str,
    line: usize,
    field: &'static str,
) -> Result<f64, DatasetErrorV1> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| DatasetErrorV1::InvalidField { line, field })?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(DatasetErrorV1::InvalidField { line, field });
    }
    Ok(parsed)
}

fn parse_usize(
    value: &str,
    line: usize,
    field: &'static str,
) -> Result<usize, DatasetErrorV1> {
    value
        .parse::<usize>()
        .map_err(|_| DatasetErrorV1::InvalidField { line, field })
}

fn validate_id(value: &str, line: usize, field: &'static str) -> Result<(), DatasetErrorV1> {
    if nonempty(value) && !value.contains('\t') && !value.contains('\n') && !value.contains('\r') {
        Ok(())
    } else {
        Err(DatasetErrorV1::InvalidField { line, field })
    }
}

impl SampleV1 {
    fn parse(line_text: &str, line: usize) -> Result<Self, DatasetErrorV1> {
        let fields: Vec<&str> = line_text.split('\t').collect();
        if fields.len() != 21 {
            return Err(DatasetErrorV1::WrongFieldCount {
                line,
                expected: 21,
                actual: fields.len(),
            });
        }
        let names = [
            "sample_id",
            "trajectory_family_id",
            "split",
            "simulator",
            "simulator_version",
            "build_sha256",
            "config_sha256",
            "method",
            "contact_regime",
            "estimator_kind",
            "time_step_s",
            "state_dimension",
            "constraint_count",
            "reported_local_error",
            "observed_global_error",
            "target_tolerance",
            "wall_time_ms",
            "peak_memory_bytes",
            "reference_build_sha256",
            "reference_config_sha256",
            "units_profile_id",
        ];
        for (index, name) in names.iter().enumerate() {
            validate_id(fields[index], line, name)?;
        }

        let split = match fields[2] {
            "calibration" => DatasetSplitV1::Calibration,
            "evaluation" => DatasetSplitV1::Evaluation,
            _ => return Err(DatasetErrorV1::InvalidField { line, field: "split" }),
        };
        for (field, value) in [
            ("build_sha256", fields[5]),
            ("config_sha256", fields[6]),
            ("reference_build_sha256", fields[18]),
            ("reference_config_sha256", fields[19]),
        ] {
            if !valid_sha256(value) {
                return Err(DatasetErrorV1::InvalidDigest { line, field });
            }
        }

        let estimator_kind = fields[9].to_owned();
        let reported_local_error = if estimator_kind == "none" {
            if fields[13] != "NA" {
                return Err(DatasetErrorV1::EstimatorValueMismatch {
                    sample_id: fields[0].to_owned(),
                });
            }
            None
        } else {
            if fields[13] == "NA" {
                return Err(DatasetErrorV1::EstimatorValueMismatch {
                    sample_id: fields[0].to_owned(),
                });
            }
            Some(parse_finite_nonnegative(fields[13], line, "reported_local_error")?)
        };

        let sample = Self {
            sample_id: fields[0].to_owned(),
            trajectory_family_id: fields[1].to_owned(),
            split,
            simulator: fields[3].to_owned(),
            simulator_version: fields[4].to_owned(),
            build_sha256: fields[5].to_owned(),
            config_sha256: fields[6].to_owned(),
            method: fields[7].to_owned(),
            contact_regime: fields[8].to_owned(),
            estimator_kind,
            time_step_s: parse_positive(fields[10], line, "time_step_s")?,
            state_dimension: parse_usize(fields[11], line, "state_dimension")?,
            constraint_count: parse_usize(fields[12], line, "constraint_count")?,
            reported_local_error,
            observed_global_error: parse_finite_nonnegative(
                fields[14],
                line,
                "observed_global_error",
            )?,
            target_tolerance: parse_positive(fields[15], line, "target_tolerance")?,
            wall_time_ms: parse_positive(fields[16], line, "wall_time_ms")?,
            peak_memory_bytes: fields[17]
                .parse::<u64>()
                .map_err(|_| DatasetErrorV1::InvalidField {
                    line,
                    field: "peak_memory_bytes",
                })?,
            reference_build_sha256: fields[18].to_owned(),
            reference_config_sha256: fields[19].to_owned(),
            units_profile_id: fields[20].to_owned(),
        };
        if sample.state_dimension == 0 || sample.peak_memory_bytes == 0 {
            return Err(DatasetErrorV1::InvalidField {
                line,
                field: "state_dimension_or_memory",
            });
        }
        Ok(sample)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatasetV1 {
    pub samples: Vec<SampleV1>,
}

impl DatasetV1 {
    pub fn parse_tsv(input: &str) -> Result<Self, DatasetErrorV1> {
        let mut lines = input.lines();
        let header = lines.next().ok_or(DatasetErrorV1::MissingHeader)?;
        if header.trim_end_matches('\r') != TSV_HEADER_V1 {
            return Err(DatasetErrorV1::HeaderMismatch);
        }

        let mut samples = Vec::new();
        for (offset, line_text) in lines.enumerate() {
            let line = offset + 2;
            let normalized = line_text.trim_end_matches('\r');
            if normalized.is_empty() {
                return Err(DatasetErrorV1::BlankRow { line });
            }
            samples.push(SampleV1::parse(normalized, line)?);
        }

        let mut ids = BTreeSet::new();
        let mut family_splits = BTreeMap::new();
        for sample in &samples {
            if !ids.insert(sample.sample_id.clone()) {
                return Err(DatasetErrorV1::DuplicateSampleId(
                    sample.sample_id.clone(),
                ));
            }
            if let Some(prior) =
                family_splits.insert(sample.trajectory_family_id.clone(), sample.split)
            {
                if prior != sample.split {
                    return Err(DatasetErrorV1::TrajectoryFamilySplitLeakage {
                        family_id: sample.trajectory_family_id.clone(),
                    });
                }
            }
        }

        Ok(Self { samples })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKeyV1 {
    simulator: String,
    simulator_version: String,
    build_sha256: String,
    config_sha256: String,
    method: String,
    contact_regime: String,
    estimator_kind: String,
    time_step_bits: u64,
    state_dimension: usize,
    constraint_count: usize,
    reference_build_sha256: String,
    reference_config_sha256: String,
    units_profile_id: String,
}

impl From<&SampleV1> for GroupKeyV1 {
    fn from(sample: &SampleV1) -> Self {
        Self {
            simulator: sample.simulator.clone(),
            simulator_version: sample.simulator_version.clone(),
            build_sha256: sample.build_sha256.clone(),
            config_sha256: sample.config_sha256.clone(),
            method: sample.method.clone(),
            contact_regime: sample.contact_regime.clone(),
            estimator_kind: sample.estimator_kind.clone(),
            time_step_bits: sample.time_step_s.to_bits(),
            state_dimension: sample.state_dimension,
            constraint_count: sample.constraint_count,
            reference_build_sha256: sample.reference_build_sha256.clone(),
            reference_config_sha256: sample.reference_config_sha256.clone(),
            units_profile_id: sample.units_profile_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WilsonIntervalV1 {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConfusionMatrixV1 {
    pub true_accept: usize,
    pub false_accept: usize,
    pub false_reject: usize,
    pub true_reject: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupReportV1 {
    pub simulator: String,
    pub simulator_version: String,
    pub method: String,
    pub contact_regime: String,
    pub estimator_kind: String,
    pub time_step_s: f64,
    pub state_dimension: usize,
    pub constraint_count: usize,
    pub calibration_sample_count: usize,
    pub evaluation_sample_count: usize,
    pub calibration_multiplier: Option<f64>,
    pub evaluation_coverage: Option<f64>,
    pub evaluation_coverage_wilson_95: Option<WilsonIntervalV1>,
    pub evaluation_covered_count: usize,
    pub evaluation_estimator_count: usize,
    pub evaluation_unavailable_estimator_count: usize,
    pub evaluation_false_accept_fraction: Option<f64>,
    pub evaluation_false_reject_fraction: Option<f64>,
    pub confusion: ConfusionMatrixV1,
    pub evaluation_ratio_p50: Option<f64>,
    pub evaluation_ratio_p95: Option<f64>,
    pub evaluation_global_error_p50: f64,
    pub evaluation_global_error_p95: f64,
    pub evaluation_wall_time_p50_ms: f64,
    pub evaluation_wall_time_p95_ms: f64,
    pub evaluation_peak_memory_p95_bytes: u64,
}

fn nearest_rank(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty() || !quantile.is_finite() || quantile <= 0.0 || quantile > 1.0 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((quantile * sorted.len() as f64).ceil() as usize).max(1);
    sorted.get(rank - 1).copied()
}

pub fn wilson_95(successes: usize, total: usize) -> Option<WilsonIntervalV1> {
    if total == 0 || successes > total {
        return None;
    }
    let z = 1.959_963_984_540_054_f64;
    let n = total as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width =
        z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    Some(WilsonIntervalV1 {
        lower: (center - half_width).max(0.0),
        upper: (center + half_width).min(1.0),
    })
}

fn summarize_group(
    key: &GroupKeyV1,
    samples: &[&SampleV1],
    calibration_quantile: f64,
) -> Result<GroupReportV1, DatasetErrorV1> {
    let calibration: Vec<&SampleV1> = samples
        .iter()
        .copied()
        .filter(|sample| sample.split == DatasetSplitV1::Calibration)
        .collect();
    let evaluation: Vec<&SampleV1> = samples
        .iter()
        .copied()
        .filter(|sample| sample.split == DatasetSplitV1::Evaluation)
        .collect();

    let is_estimating = key.estimator_kind != "none";
    if is_estimating && calibration.is_empty() {
        return Err(DatasetErrorV1::NoCalibrationSamples {
            group_key: key.method.clone(),
        });
    }
    if evaluation.is_empty() {
        return Err(DatasetErrorV1::NoEvaluationSamples {
            group_key: key.method.clone(),
        });
    }

    let calibration_multiplier = if is_estimating {
        let mut ratios = Vec::with_capacity(calibration.len());
        for sample in &calibration {
            let estimate = sample.reported_local_error.expect("parser enforces estimator");
            if estimate == 0.0 && sample.observed_global_error > 0.0 {
                return Err(DatasetErrorV1::ZeroEstimateUnderpredicts {
                    sample_id: sample.sample_id.clone(),
                });
            }
            let ratio = if estimate == 0.0 {
                0.0
            } else {
                sample.observed_global_error / estimate
            };
            if !ratio.is_finite() {
                return Err(DatasetErrorV1::ZeroEstimateUnderpredicts {
                    sample_id: sample.sample_id.clone(),
                });
            }
            ratios.push(ratio);
        }
        Some(
            nearest_rank(&ratios, calibration_quantile)
                .ok_or(DatasetErrorV1::InvalidCoverageQuantile)?,
        )
    } else {
        None
    };

    let mut eval_ratios = Vec::new();
    let mut global_errors = Vec::with_capacity(evaluation.len());
    let mut wall_times = Vec::with_capacity(evaluation.len());
    let mut memory: Vec<u64> = Vec::with_capacity(evaluation.len());
    let mut confusion = ConfusionMatrixV1::default();
    let mut covered_count = 0usize;
    let mut estimator_count = 0usize;
    let mut unavailable_count = 0usize;

    for sample in &evaluation {
        global_errors.push(sample.observed_global_error);
        wall_times.push(sample.wall_time_ms);
        memory.push(sample.peak_memory_bytes);
        match (calibration_multiplier, sample.reported_local_error) {
            (Some(multiplier), Some(estimate)) => {
                estimator_count += 1;
                let ratio = if estimate == 0.0 {
                    if sample.observed_global_error == 0.0 {
                        0.0
                    } else {
                        f64::INFINITY
                    }
                } else {
                    sample.observed_global_error / estimate
                };
                eval_ratios.push(ratio);
                if sample.observed_global_error <= multiplier * estimate {
                    covered_count += 1;
                }

                let predicted_accept = multiplier * estimate <= sample.target_tolerance;
                let actual_accept = sample.observed_global_error <= sample.target_tolerance;
                match (predicted_accept, actual_accept) {
                    (true, true) => confusion.true_accept += 1,
                    (true, false) => confusion.false_accept += 1,
                    (false, true) => confusion.false_reject += 1,
                    (false, false) => confusion.true_reject += 1,
                }
            }
            (None, None) => unavailable_count += 1,
            _ => {
                return Err(DatasetErrorV1::EstimatorValueMismatch {
                    sample_id: sample.sample_id.clone(),
                })
            }
        }
    }

    let evaluation_coverage = (estimator_count > 0)
        .then_some(covered_count as f64 / estimator_count as f64);
    let wilson = wilson_95(covered_count, estimator_count);

    let predicted_accept_count = confusion.true_accept + confusion.false_accept;
    let actual_accept_count = confusion.true_accept + confusion.false_reject;
    let false_accept_fraction = (predicted_accept_count > 0)
        .then_some(confusion.false_accept as f64 / predicted_accept_count as f64);
    let false_reject_fraction = (actual_accept_count > 0)
        .then_some(confusion.false_reject as f64 / actual_accept_count as f64);

    memory.sort_unstable();
    SomeOrError::check_memory(&memory)?;

    Ok(GroupReportV1 {
        simulator: key.simulator.clone(),
        simulator_version: key.simulator_version.clone(),
        method: key.method.clone(),
        contact_regime: key.contact_regime.clone(),
        estimator_kind: key.estimator_kind.clone(),
        time_step_s: f64::from_bits(key.time_step_bits),
        state_dimension: key.state_dimension,
        constraint_count: key.constraint_count,
        calibration_sample_count: calibration.len(),
        evaluation_sample_count: evaluation.len(),
        calibration_multiplier,
        evaluation_coverage,
        evaluation_coverage_wilson_95: wilson,
        evaluation_covered_count: covered_count,
        evaluation_estimator_count: estimator_count,
        evaluation_unavailable_estimator_count: unavailable_count,
        evaluation_false_accept_fraction: false_accept_fraction,
        evaluation_false_reject_fraction: false_reject_fraction,
        confusion,
        evaluation_ratio_p50: nearest_rank(&eval_ratios, 0.50),
        evaluation_ratio_p95: nearest_rank(&eval_ratios, 0.95),
        evaluation_global_error_p50: nearest_rank(&global_errors, 0.50).unwrap_or(0.0),
        evaluation_global_error_p95: nearest_rank(&global_errors, 0.95).unwrap_or(0.0),
        evaluation_wall_time_p50_ms: nearest_rank(&wall_times, 0.50).unwrap_or(0.0),
        evaluation_wall_time_p95_ms: nearest_rank(&wall_times, 0.95).unwrap_or(0.0),
        evaluation_peak_memory_p95_bytes: percentile_u64(&memory, 0.95).unwrap_or(0),
    })
}

struct SomeOrError;

impl SomeOrError {
    fn check_memory(values: &[u64]) -> Result<(), DatasetErrorV1> {
        if values.is_empty() || values.iter().any(|value| *value == 0) {
            return Err(DatasetErrorV1::InvalidField {
                line: 0,
                field: "peak_memory_bytes",
            });
        }
        Ok(())
    }
}

fn percentile_u64(values: &[u64], quantile: f64) -> Option<u64> {
    if values.is_empty() || !quantile.is_finite() || quantile <= 0.0 || quantile > 1.0 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((quantile * sorted.len() as f64).ceil() as usize).max(1);
    sorted.get(rank - 1).copied()
}

pub fn analyze_dataset(
    dataset: &DatasetV1,
    calibration_quantile: f64,
) -> Result<Vec<GroupReportV1>, DatasetErrorV1> {
    if !calibration_quantile.is_finite()
        || calibration_quantile <= 0.0
        || calibration_quantile > 1.0
    {
        return Err(DatasetErrorV1::InvalidCoverageQuantile);
    }
    let mut groups: BTreeMap<GroupKeyV1, Vec<&SampleV1>> = BTreeMap::new();
    for sample in &dataset.samples {
        groups
            .entry(GroupKeyV1::from(sample))
            .or_default()
            .push(sample);
    }
    groups
        .iter()
        .map(|(key, samples)| summarize_group(key, samples, calibration_quantile))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/synthetic-error-samples-v0.1.tsv");

    #[test]
    fn synthetic_fixture_parses_and_reports_cenic_calibration_and_mujoco_unavailable() {
        let dataset = DatasetV1::parse_tsv(FIXTURE).unwrap();
        let reports = analyze_dataset(&dataset, 0.99).unwrap();
        let cenic = reports.iter().find(|report| report.simulator == "drake").unwrap();
        assert_eq!(cenic.calibration_sample_count, 4);
        assert_eq!(cenic.evaluation_sample_count, 5);
        assert!((cenic.calibration_multiplier.unwrap() - 1.7).abs() < 1.0e-12);
        assert_eq!(cenic.evaluation_covered_count, 3);
        assert_eq!(cenic.evaluation_estimator_count, 5);
        assert_eq!(cenic.confusion.true_accept, 3);
        assert_eq!(cenic.confusion.false_accept, 1);
        assert_eq!(cenic.confusion.true_reject, 1);
        assert_eq!(cenic.confusion.false_reject, 0);
        assert!((cenic.evaluation_coverage.unwrap() - 0.6).abs() < 1.0e-12);
        assert!(cenic.evaluation_coverage_wilson_95.unwrap().lower < 0.6);
        assert!(cenic.evaluation_coverage_wilson_95.unwrap().upper > 0.6);

        let mujoco = reports
            .iter()
            .find(|report| report.simulator == "mujoco")
            .unwrap();
        assert_eq!(mujoco.calibration_multiplier, None);
        assert_eq!(mujoco.evaluation_coverage, None);
        assert_eq!(mujoco.evaluation_unavailable_estimator_count, 2);
        assert!(mujoco.evaluation_global_error_p95 > 0.0);
    }

    #[test]
    fn trajectory_family_cannot_leak_across_splits() {
        let input = FIXTURE.replace(
            "cenic-eval-1\ttraj-e\tevaluation",
            "cenic-eval-1\ttraj-a\tevaluation",
        );
        assert!(matches!(
            DatasetV1::parse_tsv(&input),
            Err(DatasetErrorV1::TrajectoryFamilySplitLeakage { .. })
        ));
    }

    #[test]
    fn zero_estimate_with_nonzero_calibration_error_is_uncalibratable() {
        let input = FIXTURE.replace(
            "cenic-cal-1\ttraj-a\tcalibration\tdrake\tcurrent\t1111111111111111111111111111111111111111111111111111111111111111\t2222222222222222222222222222222222222222222222222222222222222222\tCENIC\tcompliant-stiff\thalf-step\t0.02\t6\t3\t0.0001\t0.00012",
            "cenic-cal-1\ttraj-a\tcalibration\tdrake\tcurrent\t1111111111111111111111111111111111111111111111111111111111111111\t2222222222222222222222222222222222222222222222222222222222222222\tCENIC\tcompliant-stiff\thalf-step\t0.02\t6\t3\t0\t0.00012"
        );
        let dataset = DatasetV1::parse_tsv(&input).unwrap();
        assert!(matches!(
            analyze_dataset(&dataset, 0.99),
            Err(DatasetErrorV1::ZeroEstimateUnderpredicts { .. })
        ));
    }

    #[test]
    fn missing_estimator_is_explicit_na_not_a_zero_estimate() {
        let input = FIXTURE.replace(
            "\tdiscrete-Newton\tstiff-flex\tnone\t0.002\t6\t4\tNA\t0.00031",
            "\tdiscrete-Newton\tstiff-flex\tnone\t0.002\t6\t4\t0\t0.00031",
        );
        assert!(matches!(
            DatasetV1::parse_tsv(&input),
            Err(DatasetErrorV1::EstimatorValueMismatch { .. })
        ));
    }

    #[test]
    fn altered_reference_digest_is_rejected_by_grouping_identity() {
        let input = FIXTURE.replace(
            "mujoco-eval-1\ttraj-mj-c\tevaluation\tmujoco\t4.0.1\t5555555555555555555555555555555555555555555555555555555555555555\t6666666666666666666666666666666666666666666666666666666666666666\tdiscrete-Newton\tstiff-flex\tnone\t0.003\t6\t4\tNA\t0.00039\t0.001\t0.40\t15900000\t7777777777777777777777777777777777777777777777777777777777777777",
            "mujoco-eval-1\ttraj-mj-c\tevaluation\tmujoco\t4.0.1\t5555555555555555555555555555555555555555555555555555555555555555\t6666666666666666666666666666666666666666666666666666666666666666\tdiscrete-Newton\tstiff-flex\tnone\t0.003\t6\t4\tNA\t0.00039\t0.001\t0.40\t15900000\t9999999999999999999999999999999999999999999999999999999999999999"
        );
        let dataset = DatasetV1::parse_tsv(&input).unwrap();
        let reports = analyze_dataset(&dataset, 0.99).unwrap();
        assert_eq!(reports.len(), 3, "reference digest change must split the group");
    }

    #[test]
    fn duplicate_sample_id_is_rejected() {
        let mut lines = FIXTURE.lines();
        let header = lines.next().unwrap();
        let first = lines.next().unwrap();
        let mut input = format!("{header}\n{first}\n{first}\n");
        for line in lines {
            input.push_str(line);
            input.push('\n');
        }
        assert!(matches!(
            DatasetV1::parse_tsv(&input),
            Err(DatasetErrorV1::DuplicateSampleId(_))
        ));
    }

    #[test]
    fn wilson_interval_is_defined_and_bounded() {
        let interval = wilson_95(9, 10).unwrap();
        assert!(interval.lower >= 0.0);
        assert!(interval.upper <= 1.0);
        assert!(interval.lower < 0.9);
        assert!(interval.upper > 0.9);
        assert_eq!(wilson_95(0, 0), None);
    }
}
