//! Evidence-gated comparison of AM/hybrid motor architectures against
//! optimized conventional permanent-magnet baselines.
//!
//! This crate evaluates supplied component and optimization receipts. It does
//! not solve electromagnetic, thermal, mechanical or acoustic physics.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArchitectureFamily {
    ConventionalSinteredPm,
    ConventionalBondedPm,
    PmAssistedSynrm,
    ReluctanceOnlyControl,
    AmShapedPmInsert,
    AmHybridArchitecture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchitectureRole {
    Candidate,
    RequiredBaseline,
    SecondaryControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricClass {
    MandatoryFunction,
    ArchitectureBenefit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    ProductionRecord,
    Simulation,
    PublishedLiterature,
    SyntheticControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Maturity {
    M1,
    M2,
    M3,
    M4,
    M5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Qualified,
    Fail,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    NotComparable,
    Fail,
    Indeterminate,
    InferiorToBaseline,
    NonInferiorNoArchitectureBenefit,
    ArchitectureAdvantage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemScope {
    pub application_id: String,
    pub target_function_digest: String,
    pub stator_winding_digest: String,
    pub inverter_control_digest: String,
    pub cooling_boundary_digest: String,
    pub mechanical_boundary_digest: String,
    pub duty_cycle_digest: String,
    pub thermal_envelope_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
    pub objective_set_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricSpec {
    pub metric_id: String,
    pub unit: String,
    pub class: MetricClass,
    pub direction: Direction,
    pub hard_min: Option<f64>,
    pub hard_max: Option<f64>,
    pub non_inferiority_margin: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkSpec {
    pub benchmark_id: String,
    pub scope: SystemScope,
    pub metrics: Vec<MetricSpec>,
    pub required_baseline_families: Vec<ArchitectureFamily>,
    pub minimum_baseline_optimization_evaluations: u32,
    pub minimum_independent_restarts: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentAcceptance {
    pub component_id: String,
    pub specimen_id: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub scope_digest: String,
    pub maturity: Maturity,
    pub disposition: Disposition,
    pub physical_evidence_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptimizationReceipt {
    pub receipt_id: String,
    pub digest: String,
    pub optimizer_id: String,
    pub search_space_digest: String,
    pub objective_digest: String,
    pub constraint_digest: String,
    pub protocol_digest: String,
    pub evaluations: u32,
    pub independent_restarts: u32,
    pub frozen_before_comparison: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricObservation {
    pub metric_id: String,
    pub unit: String,
    pub value: f64,
    pub uncertainty_radius: f64,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub evidence_class: EvidenceClass,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureTrial {
    pub architecture_id: String,
    pub family: ArchitectureFamily,
    pub role: ArchitectureRole,
    pub scope: SystemScope,
    pub acceptance: ComponentAcceptance,
    pub optimization: Option<OptimizationReceipt>,
    pub observations: Vec<MetricObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    MissingIdentity,
    InvalidDigest,
    InvalidMetricSet,
    DuplicateMetricSpec,
    InvalidHardBounds,
    MissingNonInferiorityMargin,
    MissingRequiredBaselineFamilies,
    NoConventionalPmBaseline,
    MissingRequiredBaseline,
    NoCandidateArchitecture,
    ReluctanceOnlyCannotBeSoleBaseline,
    DuplicateArchitectureId,
    DuplicateComponentId,
    WrongArchitectureRole,
    ScopeMismatch,
    MissingM4Acceptance,
    NonQualifiedAcceptance,
    NonPhysicalComponentEvidence,
    MissingOptimizationReceipt,
    InvalidOptimizationReceipt,
    InsufficientBaselineOptimizationBudget,
    UnfrozenBaselineOptimization,
    MissingMetricObservation,
    DuplicateMetricObservation,
    UnknownMetric,
    WrongMetricUnit,
    InadmissibleEvidenceClass,
    MissingMeasurementMethod,
    MissingCalibration,
    MissingUncertaintyRecord,
    NonFiniteMetric,
    NegativeUncertainty,
    DefiniteHardLimitViolation,
    UnresolvedHardLimit,
    NonInferiorMarginViolation,
    BaselineFailsHardLimit,
    BaselineHardLimitUnresolved,
    MissingConventionalBaseline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricComparison {
    pub metric_id: String,
    pub candidate_interval: Interval,
    pub baseline_interval: Interval,
    pub non_inferior: Option<bool>,
    pub robust_benefit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BaselineComparison {
    pub baseline_id: String,
    pub baseline_family: ArchitectureFamily,
    pub mandatory_metrics_non_inferior: bool,
    pub robustly_improved_benefit_metrics: Vec<String>,
    pub metrics: Vec<MetricComparison>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureEvaluation {
    pub architecture_id: String,
    pub decision: Decision,
    pub blockers: Vec<Blocker>,
    pub baseline_comparisons: Vec<BaselineComparison>,
    pub robust_benefit_metrics: Vec<String>,
    pub claim_ceiling: String,
}

pub fn evaluate_architectures(
    spec: &BenchmarkSpec,
    trials: &[ArchitectureTrial],
) -> Vec<ArchitectureEvaluation> {
    let global_blockers = validate_spec(spec, trials);
    trials
        .iter()
        .filter(|trial| trial.role == ArchitectureRole::Candidate)
        .map(|trial| evaluate_candidate(spec, trials, trial, &global_blockers))
        .collect()
}

fn validate_spec(spec: &BenchmarkSpec, trials: &[ArchitectureTrial]) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if spec.benchmark_id.trim().is_empty() || spec.metrics.is_empty() {
        blockers.push(Blocker::MissingIdentity);
    }
    for digest in [
        &spec.scope.target_function_digest,
        &spec.scope.stator_winding_digest,
        &spec.scope.inverter_control_digest,
        &spec.scope.cooling_boundary_digest,
        &spec.scope.mechanical_boundary_digest,
        &spec.scope.duty_cycle_digest,
        &spec.scope.thermal_envelope_digest,
        &spec.scope.measurement_setup_digest,
        &spec.scope.acceptance_policy_digest,
        &spec.scope.objective_set_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }
    let mut metric_ids = BTreeSet::new();
    let mut mandatory_count = 0usize;
    let mut benefit_count = 0usize;
    for metric in &spec.metrics {
        if metric.metric_id.trim().is_empty() || metric.unit.trim().is_empty() {
            blockers.push(Blocker::InvalidMetricSet);
        }
        if !metric_ids.insert(metric.metric_id.as_str()) {
            blockers.push(Blocker::DuplicateMetricSpec);
        }
        match metric.class {
            MetricClass::MandatoryFunction => {
                mandatory_count += 1;
                if metric
                    .non_inferiority_margin
                    .is_none_or(|margin| !margin.is_finite() || margin < 0.0)
                {
                    blockers.push(Blocker::MissingNonInferiorityMargin);
                }
            }
            MetricClass::ArchitectureBenefit => benefit_count += 1,
        }
        if metric.hard_min.is_some_and(|value| !value.is_finite())
            || metric.hard_max.is_some_and(|value| !value.is_finite())
            || metric
                .hard_min
                .zip(metric.hard_max)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            blockers.push(Blocker::InvalidHardBounds);
        }
    }
    if mandatory_count == 0 || benefit_count == 0 {
        blockers.push(Blocker::InvalidMetricSet);
    }
    if spec.required_baseline_families.is_empty() {
        blockers.push(Blocker::MissingRequiredBaselineFamilies);
    }
    if !spec.required_baseline_families.iter().any(|family| {
        matches!(
            family,
            ArchitectureFamily::ConventionalSinteredPm
                | ArchitectureFamily::ConventionalBondedPm
                | ArchitectureFamily::PmAssistedSynrm
        )
    }) {
        blockers.push(Blocker::NoConventionalPmBaseline);
    }
    if spec.minimum_baseline_optimization_evaluations == 0 || spec.minimum_independent_restarts == 0
    {
        blockers.push(Blocker::InvalidMetricSet);
    }

    let mut architecture_ids = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    if !trials
        .iter()
        .any(|trial| trial.role == ArchitectureRole::Candidate)
    {
        blockers.push(Blocker::NoCandidateArchitecture);
    }
    for trial in trials {
        if !architecture_ids.insert(trial.architecture_id.as_str()) {
            blockers.push(Blocker::DuplicateArchitectureId);
        }
        if !component_ids.insert(trial.acceptance.component_id.as_str()) {
            blockers.push(Blocker::DuplicateComponentId);
        }
    }

    let baselines = trials
        .iter()
        .filter(|trial| trial.role == ArchitectureRole::RequiredBaseline)
        .collect::<Vec<_>>();
    for family in &spec.required_baseline_families {
        if !baselines.iter().any(|trial| trial.family == *family) {
            blockers.push(Blocker::MissingRequiredBaseline);
        }
    }
    if !baselines.is_empty()
        && baselines
            .iter()
            .all(|trial| trial.family == ArchitectureFamily::ReluctanceOnlyControl)
    {
        blockers.push(Blocker::ReluctanceOnlyCannotBeSoleBaseline);
    }
    blockers
}

fn evaluate_candidate(
    spec: &BenchmarkSpec,
    trials: &[ArchitectureTrial],
    candidate: &ArchitectureTrial,
    global_blockers: &[Blocker],
) -> ArchitectureEvaluation {
    let mut blockers = global_blockers.to_vec();
    if candidate.architecture_id.trim().is_empty()
        || candidate.acceptance.component_id.trim().is_empty()
        || candidate.acceptance.specimen_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    if candidate.role != ArchitectureRole::Candidate
        || !matches!(
            candidate.family,
            ArchitectureFamily::AmShapedPmInsert | ArchitectureFamily::AmHybridArchitecture
        )
    {
        blockers.push(Blocker::WrongArchitectureRole);
    }
    if candidate.scope != spec.scope {
        blockers.push(Blocker::ScopeMismatch);
    }
    validate_acceptance(candidate, spec, &mut blockers);
    let candidate_values = validate_observations(candidate, spec, &mut blockers);
    let mut hard_failure = false;
    let mut hard_unresolved = false;
    let mut missing_metrics = false;
    for metric in &spec.metrics {
        let Some(interval) = candidate_values.get(metric.metric_id.as_str()) else {
            missing_metrics = true;
            continue;
        };
        if let Some(minimum) = metric.hard_min {
            if interval.upper < minimum {
                hard_failure = true;
                blockers.push(Blocker::DefiniteHardLimitViolation);
            } else if interval.lower < minimum {
                hard_unresolved = true;
                blockers.push(Blocker::UnresolvedHardLimit);
            }
        }
        if let Some(maximum) = metric.hard_max {
            if interval.lower > maximum {
                hard_failure = true;
                blockers.push(Blocker::DefiniteHardLimitViolation);
            } else if interval.upper > maximum {
                hard_unresolved = true;
                blockers.push(Blocker::UnresolvedHardLimit);
            }
        }
    }

    let baselines = trials
        .iter()
        .filter(|trial| {
            trial.role == ArchitectureRole::RequiredBaseline
                && spec.required_baseline_families.contains(&trial.family)
        })
        .collect::<Vec<_>>();
    let mut comparisons = Vec::new();
    let mut all_baselines_non_inferior = !baselines.is_empty();
    let mut all_baselines_have_admissible_evidence = true;
    let mut any_robust_benefit = false;
    let mut all_candidate_benefits = BTreeSet::new();

    for baseline in baselines {
        let mut baseline_blockers = Vec::new();
        if baseline.scope != spec.scope {
            baseline_blockers.push(Blocker::ScopeMismatch);
        }
        validate_acceptance(baseline, spec, &mut baseline_blockers);
        validate_baseline_optimization(baseline, spec, &mut baseline_blockers);
        let baseline_values = validate_observations(baseline, spec, &mut baseline_blockers);
        for metric in &spec.metrics {
            if let Some(interval) = baseline_values.get(metric.metric_id.as_str()) {
                if let Some(minimum) = metric.hard_min {
                    if interval.upper < minimum {
                        baseline_blockers.push(Blocker::BaselineFailsHardLimit);
                    } else if interval.lower < minimum {
                        baseline_blockers.push(Blocker::BaselineHardLimitUnresolved);
                    }
                }
                if let Some(maximum) = metric.hard_max {
                    if interval.lower > maximum {
                        baseline_blockers.push(Blocker::BaselineFailsHardLimit);
                    } else if interval.upper > maximum {
                        baseline_blockers.push(Blocker::BaselineHardLimitUnresolved);
                    }
                }
            }
        }
        if !baseline_blockers.is_empty() {
            all_baselines_have_admissible_evidence = false;
            blockers.extend(baseline_blockers);
        }

        let mut baseline_non_inferior = !baseline_values.is_empty();
        let mut robust_benefits = Vec::new();
        let mut metric_comparisons = Vec::new();
        for metric in &spec.metrics {
            let (Some(candidate_interval), Some(baseline_interval)) = (
                candidate_values.get(metric.metric_id.as_str()).copied(),
                baseline_values.get(metric.metric_id.as_str()).copied(),
            ) else {
                missing_metrics = true;
                baseline_non_inferior = false;
                continue;
            };
            let non_inferior = match metric.class {
                MetricClass::MandatoryFunction => {
                    let margin = metric.non_inferiority_margin.unwrap_or(0.0);
                    let safe = match metric.direction {
                        Direction::Maximize => {
                            candidate_interval.lower >= baseline_interval.upper - margin
                        }
                        Direction::Minimize => {
                            candidate_interval.upper <= baseline_interval.lower + margin
                        }
                    };
                    if !safe {
                        baseline_non_inferior = false;
                        blockers.push(Blocker::NonInferiorMarginViolation);
                    }
                    Some(safe)
                }
                MetricClass::ArchitectureBenefit => None,
            };
            let robust_benefit = metric.class == MetricClass::ArchitectureBenefit
                && match metric.direction {
                    Direction::Maximize => candidate_interval.lower > baseline_interval.upper,
                    Direction::Minimize => candidate_interval.upper < baseline_interval.lower,
                };
            if robust_benefit {
                any_robust_benefit = true;
                robust_benefits.push(metric.metric_id.clone());
                all_candidate_benefits.insert(metric.metric_id.clone());
            }
            metric_comparisons.push(MetricComparison {
                metric_id: metric.metric_id.clone(),
                candidate_interval,
                baseline_interval,
                non_inferior,
                robust_benefit,
            });
        }
        all_baselines_non_inferior &= baseline_non_inferior;
        comparisons.push(BaselineComparison {
            baseline_id: baseline.architecture_id.clone(),
            baseline_family: baseline.family,
            mandatory_metrics_non_inferior: baseline_non_inferior,
            robustly_improved_benefit_metrics: robust_benefits,
            metrics: metric_comparisons,
        });
    }

    let has_not_comparable = blockers.iter().any(|blocker| {
        matches!(
            blocker,
            Blocker::MissingIdentity
                | Blocker::ScopeMismatch
                | Blocker::WrongArchitectureRole
                | Blocker::MissingRequiredBaseline
                | Blocker::MissingRequiredBaselineFamilies
                | Blocker::NoConventionalPmBaseline
                | Blocker::NoCandidateArchitecture
                | Blocker::InvalidMetricSet
                | Blocker::DuplicateMetricSpec
                | Blocker::InvalidHardBounds
                | Blocker::MissingNonInferiorityMargin
                | Blocker::DuplicateArchitectureId
                | Blocker::DuplicateComponentId
                | Blocker::ReluctanceOnlyCannotBeSoleBaseline
        )
    });
    let decision = if has_not_comparable {
        Decision::NotComparable
    } else if hard_failure {
        Decision::Fail
    } else if hard_unresolved
        || missing_metrics
        || !all_baselines_have_admissible_evidence
        || blockers
            .iter()
            .any(|blocker| *blocker != Blocker::NonInferiorMarginViolation)
    {
        Decision::Indeterminate
    } else if !all_baselines_non_inferior {
        Decision::InferiorToBaseline
    } else if any_robust_benefit {
        Decision::ArchitectureAdvantage
    } else {
        Decision::NonInferiorNoArchitectureBenefit
    };

    ArchitectureEvaluation {
        architecture_id: candidate.architecture_id.clone(),
        decision,
        blockers: dedupe(blockers),
        baseline_comparisons: comparisons,
        robust_benefit_metrics: all_candidate_benefits.into_iter().collect(),
        claim_ceiling: format!(
            "{:?}: system architecture decision only; no solver validation, certification or product release",
            decision
        ),
    }
}

fn validate_acceptance(
    trial: &ArchitectureTrial,
    spec: &BenchmarkSpec,
    blockers: &mut Vec<Blocker>,
) {
    if trial.acceptance.component_id.trim().is_empty()
        || trial.acceptance.specimen_id.trim().is_empty()
        || trial.acceptance.receipt_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    for digest in [
        &trial.acceptance.receipt_digest,
        &trial.acceptance.scope_digest,
        &trial.acceptance.physical_evidence_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }
    if trial.acceptance.scope_digest != spec.scope.acceptance_policy_digest {
        blockers.push(Blocker::ScopeMismatch);
    }
    if trial.acceptance.maturity < Maturity::M4 {
        blockers.push(Blocker::MissingM4Acceptance);
    }
    if trial.acceptance.disposition != Disposition::Qualified {
        blockers.push(Blocker::NonQualifiedAcceptance);
    }
}

fn validate_baseline_optimization(
    trial: &ArchitectureTrial,
    spec: &BenchmarkSpec,
    blockers: &mut Vec<Blocker>,
) {
    let Some(receipt) = &trial.optimization else {
        blockers.push(Blocker::MissingOptimizationReceipt);
        return;
    };
    if receipt.receipt_id.trim().is_empty()
        || receipt.optimizer_id.trim().is_empty()
        || !valid_sha256(&receipt.digest)
        || !valid_sha256(&receipt.search_space_digest)
        || !valid_sha256(&receipt.objective_digest)
        || !valid_sha256(&receipt.constraint_digest)
        || !valid_sha256(&receipt.protocol_digest)
        || receipt.objective_digest != spec.scope.objective_set_digest
        || receipt.constraint_digest != spec.scope.acceptance_policy_digest
    {
        blockers.push(Blocker::InvalidOptimizationReceipt);
    }
    if receipt.evaluations < spec.minimum_baseline_optimization_evaluations
        || receipt.independent_restarts < spec.minimum_independent_restarts
    {
        blockers.push(Blocker::InsufficientBaselineOptimizationBudget);
    }
    if !receipt.frozen_before_comparison {
        blockers.push(Blocker::UnfrozenBaselineOptimization);
    }
}

fn validate_observations(
    trial: &ArchitectureTrial,
    spec: &BenchmarkSpec,
    blockers: &mut Vec<Blocker>,
) -> BTreeMap<String, Interval> {
    let mut by_id = BTreeMap::new();
    for observation in &trial.observations {
        if by_id
            .insert(observation.metric_id.as_str(), observation)
            .is_some()
        {
            blockers.push(Blocker::DuplicateMetricObservation);
        }
        if observation.metric_id.trim().is_empty() || observation.evidence_id.trim().is_empty() {
            blockers.push(Blocker::MissingIdentity);
        }
        if !valid_sha256(&observation.evidence_digest)
            || !valid_sha256(&observation.calibration_digest)
            || !valid_sha256(&observation.uncertainty_record_digest)
        {
            blockers.push(Blocker::InvalidDigest);
        }
        if observation.evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::InadmissibleEvidenceClass);
        }
        if observation.method_id.trim().is_empty() {
            blockers.push(Blocker::MissingMeasurementMethod);
        }
        if !observation.value.is_finite() || !observation.uncertainty_radius.is_finite() {
            blockers.push(Blocker::NonFiniteMetric);
        }
        if observation.uncertainty_radius < 0.0 {
            blockers.push(Blocker::NegativeUncertainty);
        }
        let Some(metric) = spec
            .metrics
            .iter()
            .find(|metric| metric.metric_id == observation.metric_id)
        else {
            blockers.push(Blocker::UnknownMetric);
            continue;
        };
        if metric.unit != observation.unit {
            blockers.push(Blocker::WrongMetricUnit);
        }
    }

    let mut intervals = BTreeMap::new();
    for metric in &spec.metrics {
        let Some(observation) = by_id.get(metric.metric_id.as_str()) else {
            blockers.push(Blocker::MissingMetricObservation);
            continue;
        };
        if !observation.value.is_finite()
            || !observation.uncertainty_radius.is_finite()
            || observation.uncertainty_radius < 0.0
        {
            continue;
        }
        intervals.insert(
            metric.metric_id.clone(),
            Interval {
                lower: observation.value - observation.uncertainty_radius,
                upper: observation.value + observation.uncertainty_radius,
            },
        );
    }
    intervals
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn dedupe(mut blockers: Vec<Blocker>) -> Vec<Blocker> {
    blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    blockers.dedup();
    blockers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn scope() -> SystemScope {
        SystemScope {
            application_id: "MOTOR-50KW-001".to_string(),
            target_function_digest: digest('a'),
            stator_winding_digest: digest('b'),
            inverter_control_digest: digest('c'),
            cooling_boundary_digest: digest('d'),
            mechanical_boundary_digest: digest('e'),
            duty_cycle_digest: digest('f'),
            thermal_envelope_digest: digest('1'),
            measurement_setup_digest: digest('2'),
            acceptance_policy_digest: digest('3'),
            objective_set_digest: digest('4'),
        }
    }

    fn spec() -> BenchmarkSpec {
        BenchmarkSpec {
            benchmark_id: "ARCH-BENCH-001".to_string(),
            scope: scope(),
            metrics: vec![
                MetricSpec {
                    metric_id: "torque_density".to_string(),
                    unit: "Nm/L".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Maximize,
                    hard_min: Some(12.0),
                    hard_max: None,
                    non_inferiority_margin: Some(0.5),
                },
                MetricSpec {
                    metric_id: "efficiency".to_string(),
                    unit: "fraction".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Maximize,
                    hard_min: Some(0.90),
                    hard_max: None,
                    non_inferiority_margin: Some(0.005),
                },
                MetricSpec {
                    metric_id: "maximum_magnet_temperature".to_string(),
                    unit: "degC".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(120.0),
                    non_inferiority_margin: Some(5.0),
                },
                MetricSpec {
                    metric_id: "demagnetization_margin".to_string(),
                    unit: "ratio".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Maximize,
                    hard_min: Some(1.3),
                    hard_max: None,
                    non_inferiority_margin: Some(0.1),
                },
                MetricSpec {
                    metric_id: "retention_margin".to_string(),
                    unit: "ratio".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Maximize,
                    hard_min: Some(2.0),
                    hard_max: None,
                    non_inferiority_margin: Some(0.1),
                },
                MetricSpec {
                    metric_id: "pm_mass".to_string(),
                    unit: "kg".to_string(),
                    class: MetricClass::ArchitectureBenefit,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "rotor_inertia".to_string(),
                    unit: "kg*m2".to_string(),
                    class: MetricClass::ArchitectureBenefit,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "cost_per_torque".to_string(),
                    unit: "USD/Nm".to_string(),
                    class: MetricClass::ArchitectureBenefit,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
            ],
            required_baseline_families: vec![
                ArchitectureFamily::ConventionalSinteredPm,
                ArchitectureFamily::PmAssistedSynrm,
            ],
            minimum_baseline_optimization_evaluations: 100,
            minimum_independent_restarts: 3,
        }
    }

    fn acceptance(id: &str) -> ComponentAcceptance {
        ComponentAcceptance {
            component_id: id.to_string(),
            specimen_id: format!("SPEC-{id}"),
            receipt_id: format!("ACCEPT-{id}"),
            receipt_digest: digest('5'),
            scope_digest: digest('3'),
            maturity: Maturity::M4,
            disposition: Disposition::Qualified,
            physical_evidence_digest: digest('6'),
        }
    }

    fn optimization(id: &str) -> OptimizationReceipt {
        OptimizationReceipt {
            receipt_id: format!("OPT-{id}"),
            digest: digest('7'),
            optimizer_id: format!("OPTIMIZER-{id}"),
            search_space_digest: digest('8'),
            objective_digest: digest('4'),
            constraint_digest: digest('3'),
            protocol_digest: digest('9'),
            evaluations: 500,
            independent_restarts: 5,
            frozen_before_comparison: true,
        }
    }

    fn observation(
        metric_id: &str,
        unit: &str,
        value: f64,
        uncertainty: f64,
        suffix: char,
    ) -> MetricObservation {
        MetricObservation {
            metric_id: metric_id.to_string(),
            unit: unit.to_string(),
            value,
            uncertainty_radius: uncertainty,
            evidence_id: format!("OBS-{metric_id}-{suffix}"),
            evidence_digest: digest(suffix),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            method_id: format!("METHOD-{metric_id}"),
            calibration_digest: digest('a'),
            uncertainty_record_digest: digest('b'),
        }
    }

    fn trial(
        id: &str,
        family: ArchitectureFamily,
        role: ArchitectureRole,
        values: [f64; 8],
    ) -> ArchitectureTrial {
        let observations = vec![
            observation("torque_density", "Nm/L", values[0], 0.1, 'a'),
            observation("efficiency", "fraction", values[1], 0.001, 'b'),
            observation("maximum_magnet_temperature", "degC", values[2], 1.0, 'c'),
            observation("demagnetization_margin", "ratio", values[3], 0.05, 'd'),
            observation("retention_margin", "ratio", values[4], 0.05, 'e'),
            observation("pm_mass", "kg", values[5], 0.01, 'f'),
            observation("rotor_inertia", "kg*m2", values[6], 0.005, '1'),
            observation("cost_per_torque", "USD/Nm", values[7], 20.0, '2'),
        ];
        ArchitectureTrial {
            architecture_id: id.to_string(),
            family,
            role,
            scope: scope(),
            acceptance: acceptance(id),
            optimization: (role == ArchitectureRole::RequiredBaseline).then(|| optimization(id)),
            observations,
        }
    }

    fn known_answer_trials() -> Vec<ArchitectureTrial> {
        vec![
            trial(
                "AM-HYBRID-001",
                ArchitectureFamily::AmHybridArchitecture,
                ArchitectureRole::Candidate,
                [14.7, 0.946, 92.0, 1.7, 2.6, 0.45, 0.15, 930.0],
            ),
            trial(
                "SINTERED-PM-001",
                ArchitectureFamily::ConventionalSinteredPm,
                ArchitectureRole::RequiredBaseline,
                [15.0, 0.94, 95.0, 1.6, 2.5, 1.0, 0.20, 1000.0],
            ),
            trial(
                "PMASS-SYNRM-001",
                ArchitectureFamily::PmAssistedSynrm,
                ArchitectureRole::RequiredBaseline,
                [14.5, 0.95, 90.0, 1.8, 2.7, 0.6, 0.18, 1050.0],
            ),
            trial(
                "RELUCTANCE-CONTROL-001",
                ArchitectureFamily::ReluctanceOnlyControl,
                ArchitectureRole::SecondaryControl,
                [13.0, 0.93, 96.0, 10.0, 2.8, 0.0, 0.16, 900.0],
            ),
        ]
    }

    #[test]
    fn am_hybrid_is_architecture_advantage_with_two_optimized_pm_baselines() {
        let evaluations = evaluate_architectures(&spec(), &known_answer_trials());
        let candidate = evaluations
            .iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::ArchitectureAdvantage);
        assert_eq!(candidate.baseline_comparisons.len(), 2);
        assert!(candidate
            .baseline_comparisons
            .iter()
            .all(|comparison| comparison.mandatory_metrics_non_inferior));
        assert!(candidate
            .robust_benefit_metrics
            .contains(&"pm_mass".to_string()));
    }

    #[test]
    fn reluctance_only_control_cannot_be_the_only_baseline() {
        let mut spec = spec();
        spec.required_baseline_families = vec![ArchitectureFamily::ReluctanceOnlyControl];
        let mut trials = known_answer_trials();
        trials.retain(|trial| trial.role != ArchitectureRole::RequiredBaseline);
        trials.push(trial(
            "RELUCTANCE-BASELINE",
            ArchitectureFamily::ReluctanceOnlyControl,
            ArchitectureRole::RequiredBaseline,
            [13.0, 0.93, 96.0, 10.0, 2.8, 0.0, 0.16, 900.0],
        ));
        let candidate = evaluate_architectures(&spec, &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::NotComparable);
        assert!(candidate
            .blockers
            .contains(&Blocker::NoConventionalPmBaseline));
    }

    #[test]
    fn missing_required_conventional_baseline_makes_comparison_not_comparable() {
        let mut trials = known_answer_trials();
        trials.retain(|trial| trial.family != ArchitectureFamily::PmAssistedSynrm);
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::NotComparable);
        assert!(candidate.blockers.contains(&Blocker::MissingRequiredBaseline));
    }

    #[test]
    fn scope_mismatch_is_not_repaired_by_normalizing_metrics() {
        let mut trials = known_answer_trials();
        trials[0].scope.duty_cycle_digest = digest('f');
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::NotComparable);
        assert!(candidate.blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn hard_demagnetization_failure_cannot_be_compensated_by_pm_mass() {
        let mut trials = known_answer_trials();
        trials[0].observations[3].value = 1.1;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Fail);
        assert!(candidate.blockers.contains(&Blocker::DefiniteHardLimitViolation));
    }

    #[test]
    fn uncertainty_overlapping_hard_limit_is_indeterminate() {
        let mut trials = known_answer_trials();
        trials[0].observations[3].value = 1.32;
        trials[0].observations[3].uncertainty_radius = 0.05;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate.blockers.contains(&Blocker::UnresolvedHardLimit));
    }

    #[test]
    fn noninferiority_failure_is_distinguished_from_unresolved_evidence() {
        let mut trials = known_answer_trials();
        trials[0].observations[1].value = 0.92;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::InferiorToBaseline);
    }

    #[test]
    fn baseline_outside_hard_application_limits_is_not_an_admissible_reference() {
        let mut trials = known_answer_trials();
        trials[1].observations[3].value = 1.0;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate.blockers.contains(&Blocker::BaselineFailsHardLimit));
    }

    #[test]
    fn noninferiority_failure_against_any_required_baseline_blocks_advantage() {
        let mut trials = known_answer_trials();
        trials[0].observations[1].value = 0.93;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_ne!(candidate.decision, Decision::ArchitectureAdvantage);
        assert!(candidate.blockers.contains(&Blocker::NonInferiorMarginViolation));
    }

    #[test]
    fn benefit_interval_overlap_is_not_counted_as_architecture_improvement() {
        let mut trials = known_answer_trials();
        trials[0].observations[5].value = 0.80;
        trials[0].observations[5].uncertainty_radius = 0.30;
        trials[0].observations[6].value = 0.18;
        trials[0].observations[6].uncertainty_radius = 0.02;
        trials[0].observations[7].value = 1020.0;
        trials[0].observations[7].uncertainty_radius = 100.0;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(
            candidate.decision,
            Decision::NonInferiorNoArchitectureBenefit
        );
        assert!(candidate.robust_benefit_metrics.is_empty());
    }

    #[test]
    fn simulation_only_component_metrics_do_not_support_architecture_advantage() {
        let mut trials = known_answer_trials();
        trials[0].observations[0].evidence_class = EvidenceClass::Simulation;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate.blockers.contains(&Blocker::InadmissibleEvidenceClass));
    }

    #[test]
    fn baseline_optimization_budget_must_be_frozen_and_adequate() {
        let mut trials = known_answer_trials();
        trials[1].optimization.as_mut().unwrap().evaluations = 10;
        trials[2].optimization.as_mut().unwrap().frozen_before_comparison = false;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate
            .blockers
            .contains(&Blocker::InsufficientBaselineOptimizationBudget));
        assert!(candidate
            .blockers
            .contains(&Blocker::UnfrozenBaselineOptimization));
    }

    #[test]
    fn nonqualified_component_acceptance_blocks_architecture_advantage() {
        let mut trials = known_answer_trials();
        trials[0].acceptance.disposition = Disposition::Indeterminate;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate.blockers.contains(&Blocker::NonQualifiedAcceptance));
    }

    #[test]
    fn duplicate_observation_ids_are_not_silently_overwritten() {
        let mut trials = known_answer_trials();
        trials[0].observations.push(trials[0].observations[0].clone());
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate.blockers.contains(&Blocker::DuplicateMetricObservation));
    }

    #[test]
    fn overlapping_mandatory_metric_intervals_may_be_noninferior_with_margin() {
        let mut trials = known_answer_trials();
        trials[0].observations[0].value = 14.8;
        trials[0].observations[0].uncertainty_radius = 0.2;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert!(candidate
            .baseline_comparisons
            .iter()
            .all(|comparison| comparison.mandatory_metrics_non_inferior));
    }

    #[test]
    fn non_am_candidate_cannot_claim_an_am_architecture_advantage() {
        let mut trials = known_answer_trials();
        trials[0].family = ArchitectureFamily::ConventionalBondedPm;
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::NotComparable);
        assert!(candidate.blockers.contains(&Blocker::WrongArchitectureRole));
    }

    #[test]
    fn no_candidate_trial_returns_no_architecture_decisions() {
        let mut trials = known_answer_trials();
        trials.retain(|trial| trial.role != ArchitectureRole::Candidate);
        assert!(evaluate_architectures(&spec(), &trials).is_empty());
    }

    #[test]
    fn missing_metric_observation_cannot_default_to_zero() {
        let mut trials = known_answer_trials();
        trials[0].observations.retain(|item| item.metric_id != "demagnetization_margin");
        let candidate = evaluate_architectures(&spec(), &trials)
            .into_iter()
            .find(|evaluation| evaluation.architecture_id == "AM-HYBRID-001")
            .expect("candidate");
        assert_eq!(candidate.decision, Decision::Indeterminate);
        assert!(candidate.blockers.contains(&Blocker::MissingMetricObservation));
    }
}
