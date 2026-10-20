// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Accuracy-constrained, multi-objective benchmark comparison.
//!
//! These helpers compare completed run measurements; they do not execute a
//! solver, select scientifically valid tolerances, or confer empirical skill.
//! Runtime, memory, optional energy and each declared error remain separate
//! dimensions. No weighted composite score hides trade-offs.

use std::collections::{BTreeMap, BTreeSet};

/// Measurements for one run of a frozen benchmark case.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationRun {
    pub run_id: String,
    /// SHA-256 of frozen scenario, geometry, forcing and observation support.
    /// Solver-specific settings belong in configuration_digest_sha256 instead.
    pub case_digest_sha256: String,
    /// Stable identity of the solver/profile variant used for this run.
    pub solver_variant_id: String,
    /// SHA-256 of the exact solver, numerical parameters and runtime configuration.
    /// This is expected to differ between candidate variants.
    pub configuration_digest_sha256: String,
    /// Stable hardware/backend identity required for fair runtime comparison.
    pub hardware_fingerprint: String,
    pub wall_time_seconds: f64,
    pub peak_memory_bytes: u64,
    /// Joules at a declared meter boundary. None means not measured.
    pub energy_joules: Option<f64>,
    /// Named lower-is-better errors, each measured against the same declared target.
    pub quality_errors: BTreeMap<String, f64>,
}

/// Predeclared maximum allowed error for every quality metric in the case.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityBudget {
    pub max_error_by_metric: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityViolation {
    pub metric: String,
    pub observed_error: f64,
    pub maximum_allowed_error: f64,
}

/// Ratios and independent gates; no dimensionless weighted score is produced.
#[derive(Debug, Clone, PartialEq)]
pub struct PairwiseAssessment {
    pub candidate_within_quality_budget: bool,
    pub quality_violations: Vec<QualityViolation>,
    /// Reference wall time divided by candidate wall time; above 1 means faster.
    pub wall_time_speedup: f64,
    /// Candidate peak memory divided by reference peak memory; below 1 uses less.
    pub peak_memory_ratio: f64,
    /// Candidate energy divided by reference energy, only when both were measured.
    pub energy_ratio: Option<f64>,
    /// True only when no quality/resource dimension regresses and at least one improves.
    pub candidate_strictly_dominates_reference: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EfficiencyError {
    EmptyRunId,
    InvalidCaseDigest,
    EmptySolverVariantId,
    InvalidConfigurationDigest,
    EmptyHardwareFingerprint,
    InvalidWallTime,
    InvalidPeakMemory,
    InvalidEnergy,
    EmptyQualityMetrics,
    EmptyQualityMetricName,
    InvalidQualityError(String),
    EmptyQualityBudget,
    InvalidQualityTolerance(String),
    DuplicateRunId(String),
    DifferentCase,
    DifferentHardware,
    MetricSetMismatch,
    BudgetMetricSetMismatch,
    EnergyCoverageMismatch,
    NoRuns,
    NonFiniteDerivedRatio,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl SimulationRun {
    /// Check measurement identity and validity, not physical model correctness.
    pub fn validate(&self) -> Result<(), EfficiencyError> {
        if !nonblank(&self.run_id) {
            return Err(EfficiencyError::EmptyRunId);
        }
        if !is_sha256(&self.case_digest_sha256) {
            return Err(EfficiencyError::InvalidCaseDigest);
        }
        if !nonblank(&self.solver_variant_id) {
            return Err(EfficiencyError::EmptySolverVariantId);
        }
        if !is_sha256(&self.configuration_digest_sha256) {
            return Err(EfficiencyError::InvalidConfigurationDigest);
        }
        if !nonblank(&self.hardware_fingerprint) {
            return Err(EfficiencyError::EmptyHardwareFingerprint);
        }
        if !self.wall_time_seconds.is_finite() || self.wall_time_seconds <= 0.0 {
            return Err(EfficiencyError::InvalidWallTime);
        }
        if self.peak_memory_bytes == 0 {
            return Err(EfficiencyError::InvalidPeakMemory);
        }
        if self
            .energy_joules
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(EfficiencyError::InvalidEnergy);
        }
        if self.quality_errors.is_empty() {
            return Err(EfficiencyError::EmptyQualityMetrics);
        }
        for (metric, value) in &self.quality_errors {
            if !nonblank(metric) {
                return Err(EfficiencyError::EmptyQualityMetricName);
            }
            if !value.is_finite() || *value < 0.0 {
                return Err(EfficiencyError::InvalidQualityError(metric.clone()));
            }
        }
        Ok(())
    }
}

impl QualityBudget {
    pub fn validate(&self) -> Result<(), EfficiencyError> {
        if self.max_error_by_metric.is_empty() {
            return Err(EfficiencyError::EmptyQualityBudget);
        }
        for (metric, tolerance) in &self.max_error_by_metric {
            if !nonblank(metric) {
                return Err(EfficiencyError::EmptyQualityMetricName);
            }
            if !tolerance.is_finite() || *tolerance < 0.0 {
                return Err(EfficiencyError::InvalidQualityTolerance(metric.clone()));
            }
        }
        Ok(())
    }
}

/// Return per-metric violations against a predeclared maximum-error budget.
pub fn evaluate_quality_budget(
    run: &SimulationRun,
    budget: &QualityBudget,
) -> Result<Vec<QualityViolation>, EfficiencyError> {
    run.validate()?;
    budget.validate()?;
    if run.quality_errors.keys().collect::<BTreeSet<_>>()
        != budget.max_error_by_metric.keys().collect::<BTreeSet<_>>()
    {
        return Err(EfficiencyError::BudgetMetricSetMismatch);
    }

    Ok(run
        .quality_errors
        .iter()
        .filter_map(|(metric, error)| {
            let tolerance = budget.max_error_by_metric[metric];
            (*error > tolerance).then(|| QualityViolation {
                metric: metric.clone(),
                observed_error: *error,
                maximum_allowed_error: tolerance,
            })
        })
        .collect())
}

/// Compare candidates only for the same frozen scenario and same hardware identity.
/// Solver variant and configuration digests are recorded but are expected to differ.
///
/// The error fields must already have been evaluated against the same declared
/// target. The reference run is not assumed to be ground truth.
pub fn compare_runs(
    reference: &SimulationRun,
    candidate: &SimulationRun,
    budget: &QualityBudget,
) -> Result<PairwiseAssessment, EfficiencyError> {
    reference.validate()?;
    candidate.validate()?;
    ensure_comparable(reference, candidate)?;
    if reference.run_id == candidate.run_id {
        return Err(EfficiencyError::DuplicateRunId(candidate.run_id.clone()));
    }
    let quality_violations = evaluate_quality_budget(candidate, budget)?;
    let wall_time_speedup = reference.wall_time_seconds / candidate.wall_time_seconds;
    let peak_memory_ratio = candidate.peak_memory_bytes as f64 / reference.peak_memory_bytes as f64;
    let energy_ratio = match (reference.energy_joules, candidate.energy_joules) {
        (Some(reference_energy), Some(candidate_energy)) => {
            Some(candidate_energy / reference_energy)
        }
        (None, None) => None,
        _ => return Err(EfficiencyError::EnergyCoverageMismatch),
    };
    if !wall_time_speedup.is_finite()
        || !peak_memory_ratio.is_finite()
        || energy_ratio.is_some_and(|ratio| !ratio.is_finite())
    {
        return Err(EfficiencyError::NonFiniteDerivedRatio);
    }

    Ok(PairwiseAssessment {
        candidate_within_quality_budget: quality_violations.is_empty(),
        quality_violations,
        wall_time_speedup,
        peak_memory_ratio,
        energy_ratio,
        candidate_strictly_dominates_reference: dominates(candidate, reference),
    })
}

/// Return nondominated run IDs in input order (the quality/resource Pareto frontier).
///
/// Every run must use the same case, hardware identity, error metrics and energy
/// measurement coverage. Frontier membership is descriptive, not qualification.
pub fn pareto_frontier(runs: &[SimulationRun]) -> Result<Vec<String>, EfficiencyError> {
    if runs.is_empty() {
        return Err(EfficiencyError::NoRuns);
    }
    for run in runs {
        run.validate()?;
    }

    let mut ids = BTreeSet::new();
    for run in runs {
        if !ids.insert(run.run_id.as_str()) {
            return Err(EfficiencyError::DuplicateRunId(run.run_id.clone()));
        }
    }
    for run in runs.iter().skip(1) {
        ensure_comparable(&runs[0], run)?;
    }

    Ok(runs
        .iter()
        .enumerate()
        .filter(|(index, run)| {
            !runs
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != *index && dominates(other, run))
        })
        .map(|(_, run)| run.run_id.clone())
        .collect())
}

fn ensure_comparable(
    reference: &SimulationRun,
    candidate: &SimulationRun,
) -> Result<(), EfficiencyError> {
    if !reference
        .case_digest_sha256
        .eq_ignore_ascii_case(&candidate.case_digest_sha256)
    {
        return Err(EfficiencyError::DifferentCase);
    }
    if reference.hardware_fingerprint != candidate.hardware_fingerprint {
        return Err(EfficiencyError::DifferentHardware);
    }
    if reference.quality_errors.keys().collect::<BTreeSet<_>>()
        != candidate.quality_errors.keys().collect::<BTreeSet<_>>()
    {
        return Err(EfficiencyError::MetricSetMismatch);
    }
    if reference.energy_joules.is_some() != candidate.energy_joules.is_some() {
        return Err(EfficiencyError::EnergyCoverageMismatch);
    }
    Ok(())
}

fn dominates(a: &SimulationRun, b: &SimulationRun) -> bool {
    let mut strictly_better = false;
    for (metric, a_error) in &a.quality_errors {
        let b_error = b.quality_errors[metric];
        if *a_error > b_error {
            return false;
        }
        strictly_better |= *a_error < b_error;
    }
    if a.wall_time_seconds > b.wall_time_seconds || a.peak_memory_bytes > b.peak_memory_bytes {
        return false;
    }
    strictly_better |= a.wall_time_seconds < b.wall_time_seconds;
    strictly_better |= a.peak_memory_bytes < b.peak_memory_bytes;

    if let (Some(a_energy), Some(b_energy)) = (a.energy_joules, b.energy_joules) {
        if a_energy > b_energy {
            return false;
        }
        strictly_better |= a_energy < b_energy;
    }
    strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(
        id: &str,
        time: f64,
        memory: u64,
        depth_error: f64,
        arrival_error: f64,
    ) -> SimulationRun {
        SimulationRun {
            run_id: id.into(),
            case_digest_sha256: "a".repeat(64),
            solver_variant_id: id.into(),
            configuration_digest_sha256: if id == "uniform-fine" { "c" } else { "d" }.repeat(64),
            hardware_fingerprint: "cpu-x86_64-model-a".into(),
            wall_time_seconds: time,
            peak_memory_bytes: memory,
            energy_joules: Some(time * 12.0),
            quality_errors: [
                ("depth_rmse_m".into(), depth_error),
                ("arrival_time_abs_s".into(), arrival_error),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn budget() -> QualityBudget {
        QualityBudget {
            max_error_by_metric: [
                ("depth_rmse_m".into(), 0.15),
                ("arrival_time_abs_s".into(), 3.0),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn pairwise_comparison_reports_ratios_without_composite_score() {
        let reference = run("uniform-fine", 10.0, 1_000, 0.01, 0.2);
        let candidate = run("adaptive", 4.0, 600, 0.10, 2.0);
        let result = compare_runs(&reference, &candidate, &budget()).unwrap();
        assert!(result.candidate_within_quality_budget);
        assert!(!result.candidate_strictly_dominates_reference);
        close(result.wall_time_speedup, 2.5);
        close(result.peak_memory_ratio, 0.6);
        close(result.energy_ratio.unwrap(), 0.4);
    }

    #[test]
    fn faster_run_fails_qualification_when_one_quality_gate_fails() {
        let reference = run("uniform-fine", 10.0, 1_000, 0.01, 0.2);
        let candidate = run("fast-but-wrong", 1.0, 200, 0.20, 1.0);
        let result = compare_runs(&reference, &candidate, &budget()).unwrap();
        assert!(!result.candidate_within_quality_budget);
        assert_eq!(result.quality_violations.len(), 1);
        assert_eq!(result.quality_violations[0].metric, "depth_rmse_m");
        assert!(result.wall_time_speedup > 1.0);
    }

    #[test]
    fn strict_dominance_requires_no_quality_or_resource_regression() {
        let reference = run("uniform-fine", 10.0, 1_000, 0.10, 2.0);
        let candidate = run("better-everywhere", 5.0, 800, 0.05, 1.0);
        assert!(
            compare_runs(&reference, &candidate, &budget())
                .unwrap()
                .candidate_strictly_dominates_reference
        );
    }

    #[test]
    fn frontier_keeps_tradeoffs_and_removes_a_dominated_run() {
        let accurate = run("slow-accurate", 10.0, 1_000, 0.01, 0.2);
        let fast = run("fast-approx", 3.0, 400, 0.10, 2.5);
        let middle = run("middle", 5.0, 700, 0.05, 1.0);
        let dominated = run("dominated", 12.0, 1_100, 0.10, 2.5);
        assert_eq!(
            pareto_frontier(&[accurate, fast, middle, dominated]).unwrap(),
            vec!["slow-accurate", "fast-approx", "middle"]
        );
    }

    #[test]
    fn changed_solver_configuration_remains_comparable_on_the_same_case() {
        let reference = run("uniform-fine", 10.0, 1_000, 0.01, 0.2);
        let mut candidate = run("adaptive", 4.0, 600, 0.10, 2.0);
        candidate.configuration_digest_sha256 = "e".repeat(64);
        let assessment = compare_runs(&reference, &candidate, &budget()).unwrap();
        assert!(assessment.candidate_within_quality_budget);
        assert_eq!(assessment.wall_time_speedup, 2.5);
        assert_ne!(reference.configuration_digest_sha256, candidate.configuration_digest_sha256);
    }

    #[test]
    fn different_cases_and_hardware_are_not_comparable() {
        let reference = run("reference", 10.0, 1_000, 0.01, 0.2);
        let mut different_case = run("candidate", 5.0, 800, 0.05, 1.0);
        different_case.case_digest_sha256 = "b".repeat(64);
        assert_eq!(
            compare_runs(&reference, &different_case, &budget()),
            Err(EfficiencyError::DifferentCase)
        );

        let mut different_hardware = run("candidate", 5.0, 800, 0.05, 1.0);
        different_hardware.hardware_fingerprint = "gpu-model-b".into();
        assert_eq!(
            compare_runs(&reference, &different_hardware, &budget()),
            Err(EfficiencyError::DifferentHardware)
        );
    }

    #[test]
    fn missing_energy_is_not_interpreted_as_zero() {
        let reference = run("reference", 10.0, 1_000, 0.01, 0.2);
        let mut candidate = run("candidate", 5.0, 800, 0.05, 1.0);
        candidate.energy_joules = None;
        assert_eq!(
            compare_runs(&reference, &candidate, &budget()),
            Err(EfficiencyError::EnergyCoverageMismatch)
        );
        assert_eq!(
            pareto_frontier(&[reference, candidate]),
            Err(EfficiencyError::EnergyCoverageMismatch)
        );
    }

    #[test]
    fn invalid_measurements_and_tolerances_fail_closed() {
        let invalid_time = run("invalid", f64::NAN, 100, 0.1, 1.0);
        assert_eq!(
            invalid_time.validate(),
            Err(EfficiencyError::InvalidWallTime)
        );
        let invalid_error = run("invalid-error", 1.0, 100, -0.1, 1.0);
        assert_eq!(
            invalid_error.validate(),
            Err(EfficiencyError::InvalidQualityError("depth_rmse_m".into()))
        );

        let invalid_budget = QualityBudget {
            max_error_by_metric: [("depth_rmse_m".into(), f64::NAN)].into_iter().collect(),
        };
        assert_eq!(
            compare_runs(
                &run("reference", 2.0, 100, 0.1, 1.0),
                &run("candidate", 1.0, 90, 0.1, 1.0),
                &invalid_budget
            ),
            Err(EfficiencyError::InvalidQualityTolerance(
                "depth_rmse_m".into()
            ))
        );
    }

    #[test]
    fn empty_frontier_and_duplicate_ids_are_rejected() {
        assert_eq!(pareto_frontier(&[]), Err(EfficiencyError::NoRuns));
        assert_eq!(
            pareto_frontier(&[
                run("same", 10.0, 1_000, 0.01, 0.2),
                run("same", 5.0, 800, 0.05, 1.0)
            ]),
            Err(EfficiencyError::DuplicateRunId("same".into()))
        );
    }

    #[test]
    fn exact_metric_dimension_set_is_required() {
        let reference = run("reference", 10.0, 1_000, 0.01, 0.2);
        let mut candidate = run("candidate", 5.0, 800, 0.05, 1.0);
        candidate.quality_errors.remove("arrival_time_abs_s");
        assert_eq!(
            compare_runs(&reference, &candidate, &budget()),
            Err(EfficiencyError::MetricSetMismatch)
        );
    }
}
