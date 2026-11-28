//! Evidence-gated comparison of AM thermo-electrical winding/cooling stacks.
//!
//! Synthetic tests validate evaluator semantics only. This crate does not
//! authenticate issuers, certify standards, release processes, or establish
//! physical route superiority.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    ConventionalWindingCooling,
    ConventionalHighPerformanceCooling,
    ConventionalDirectOilCooling,
    ConventionalMultiNozzleOilJetCooling,
    ConventionalHairpinOilSprayCooling,
    ConventionalSlotEmbeddedCooling,
    ConventionalHollowConductorCooling,
    AmIntegratedWindingCooling,
    HybridThermalSubassembly,
    ReferenceControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Candidate,
    RequiredBaseline,
    SecondaryControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricClass {
    MandatoryFunction,
    StackBenefit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    Simulation,
    Literature,
    Synthetic,
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
    Rejected,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    NotComparable,
    Fail,
    Indeterminate,
    InferiorToBaseline,
    NonInferiorNoStackBenefit,
    StackAdvantage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub application_id: String,
    /// Digest of canonical JSON of all other Scope fields, excluding this field.
    pub comparison_scope_digest: String,
    pub duty_digest: String,
    pub waveform_digest: String,
    pub coolant_digest: String,
    pub thermal_boundary_digest: String,
    pub insulation_policy_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
    pub objective_digest: String,
    pub constraint_digest: String,
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
pub struct BaselineApplicability {
    pub family: Family,
    pub applicable: bool,
    pub rationale: String,
    pub supporting_evidence_ids: Vec<String>,
    pub decision_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkSpec {
    pub benchmark_id: String,
    pub scope: Scope,
    pub metrics: Vec<MetricSpec>,
    pub baseline_applicability: Vec<BaselineApplicability>,
    pub required_baseline_families: Vec<Family>,
    pub minimum_baseline_evaluations: u32,
    pub minimum_independent_restarts: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentAcceptance {
    pub component_id: String,
    pub specimen_id: String,
    pub build_id: String,
    pub functional_test_evidence_id: String,
    pub functional_test_digest: String,
    pub linked_evidence_ids: Vec<String>,
    pub maturity: Maturity,
    pub disposition: Disposition,
    pub acceptance_policy_digest: String,
    pub comparison_scope_digest: String,
    pub receipt_digest: String,
    pub physical_evidence_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptimizationReceipt {
    pub receipt_digest: String,
    pub search_space_digest: String,
    pub objective_digest: String,
    pub constraint_digest: String,
    pub evaluations: u32,
    pub independent_restarts: u32,
    pub frozen_before_comparison: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub metric_id: String,
    pub comparison_scope_digest: String,
    pub component_id: String,
    pub specimen_id: String,
    pub build_id: String,
    pub material_lot_id: String,
    pub unit: String,
    pub value: f64,
    pub uncertainty_radius: f64,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub evidence_class: EvidenceClass,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trial {
    pub architecture_id: String,
    pub component_id: String,
    pub family: Family,
    pub role: Role,
    pub scope: Scope,
    pub acceptance: ComponentAcceptance,
    pub optimization: Option<OptimizationReceipt>,
    pub observations: Vec<Observation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    InvalidBenchmarkSpec,
    DuplicateArchitectureId,
    DuplicateComponentId,
    DuplicateEvidenceIdentity,
    MissingCandidate,
    MissingRequiredBaseline,
    MissingConventionalBaseline,
    ScopeMismatch,
    WrongCandidateFamily,
    InvalidAcceptance,
    BrokenSpecimenBinding,
    DuplicateObservation,
    MissingObservation,
    InvalidObservation,
    InadmissibleEvidenceClass,
    MissingMeasurementProvenance,
    InvalidOptimizationReceipt,
    BaselineNotAdmissible,
    BaselineFailsHardLimit,
    DefiniteHardLimitViolation,
    UnresolvedHardLimit,
    NonInferiorityViolation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BaselineComparison {
    pub baseline_id: String,
    pub all_mandatory_metrics_non_inferior: bool,
    pub robust_benefits: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub architecture_id: String,
    pub decision: Decision,
    pub blockers: Vec<Blocker>,
    pub baseline_comparisons: Vec<BaselineComparison>,
    pub robust_stack_benefits: Vec<String>,
}

pub fn evaluate(spec: &BenchmarkSpec, trials: &[Trial]) -> Vec<Evaluation> {
    let mut global_blockers = validate_spec(spec);
    let mut ids = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();

    for trial in trials {
        if !ids.insert(trial.architecture_id.as_str()) {
            global_blockers.push(Blocker::DuplicateArchitectureId);
        }
        if !component_ids.insert(trial.component_id.as_str()) {
            global_blockers.push(Blocker::DuplicateComponentId);
        }
        if !evidence_ids.insert(trial.acceptance.functional_test_evidence_id.as_str()) {
            global_blockers.push(Blocker::DuplicateEvidenceIdentity);
        }
        for observation in &trial.observations {
            if !evidence_ids.insert(observation.evidence_id.as_str()) {
                global_blockers.push(Blocker::DuplicateEvidenceIdentity);
            }
        }
    }

    let candidates: Vec<&Trial> = trials
        .iter()
        .filter(|trial| trial.role == Role::Candidate)
        .collect();

    if candidates.is_empty() {
        global_blockers.push(Blocker::MissingCandidate);
    }

    let mut results = Vec::new();
    for candidate in candidates {
        let mut blockers = global_blockers.clone();

        if candidate.scope != spec.scope
            || candidate.acceptance.acceptance_policy_digest != spec.scope.acceptance_policy_digest
        {
            blockers.push(Blocker::ScopeMismatch);
        }
        if !matches!(
            candidate.family,
            Family::AmIntegratedWindingCooling | Family::HybridThermalSubassembly
        ) {
            blockers.push(Blocker::WrongCandidateFamily);
        }

        validate_acceptance(candidate, spec, &mut blockers);
        validate_optimization(candidate, spec, &mut blockers);
        let candidate_intervals = validate_observations(candidate, spec, &mut blockers);
        let candidate_hard = hard_limit_status(&candidate_intervals, &spec.metrics);
        blockers.extend(candidate_hard.blockers.clone());

        let mut baseline_comparisons = Vec::new();
        let mut any_baseline_invalid = false;

        for family in &spec.required_baseline_families {
            let matching: Vec<&Trial> = trials
                .iter()
                .filter(|trial| trial.role == Role::RequiredBaseline && trial.family == *family)
                .collect();

            if matching.is_empty() {
                blockers.push(Blocker::MissingRequiredBaseline);
                any_baseline_invalid = true;
                continue;
            }

            for baseline in matching {
                if baseline.scope != spec.scope
                    || baseline.acceptance.acceptance_policy_digest
                        != spec.scope.acceptance_policy_digest
                {
                    blockers.push(Blocker::ScopeMismatch);
                    any_baseline_invalid = true;
                    continue;
                }

                let mut baseline_blockers = Vec::new();
                validate_optimization(baseline, spec, &mut baseline_blockers);
                validate_acceptance(baseline, spec, &mut baseline_blockers);
                let intervals = validate_observations(baseline, spec, &mut baseline_blockers);
                let hard = hard_limit_status(&intervals, &spec.metrics);
                baseline_blockers.extend(hard.blockers);

                if !baseline_blockers.is_empty() {
                    any_baseline_invalid = true;
                    if baseline_blockers.contains(&Blocker::DefiniteHardLimitViolation) {
                        blockers.push(Blocker::BaselineFailsHardLimit);
                    } else {
                        blockers.push(Blocker::BaselineNotAdmissible);
                    }
                    continue;
                }

                let comparison =
                    compare_metrics(spec, &candidate_intervals, &intervals, &mut blockers);
                baseline_comparisons.push(BaselineComparison {
                    baseline_id: baseline.architecture_id.clone(),
                    all_mandatory_metrics_non_inferior: comparison.non_inferior,
                    robust_benefits: comparison.robust_benefits,
                });
            }
        }

        if !spec
            .required_baseline_families
            .contains(&Family::ConventionalWindingCooling)
        {
            blockers.push(Blocker::MissingConventionalBaseline);
            any_baseline_invalid = true;
        }

        let all_comparators_present =
            baseline_comparisons.len() >= spec.required_baseline_families.len();
        let any_noninferiority_violation = blockers.contains(&Blocker::NonInferiorityViolation);
        let benefits = common_robust_benefits(&baseline_comparisons);

        let not_comparable = blockers.iter().any(|blocker| {
            matches!(
                blocker,
                Blocker::InvalidBenchmarkSpec
                    | Blocker::DuplicateArchitectureId
                    | Blocker::DuplicateComponentId
                    | Blocker::MissingCandidate
                    | Blocker::MissingRequiredBaseline
                    | Blocker::MissingConventionalBaseline
                    | Blocker::ScopeMismatch
                    | Blocker::WrongCandidateFamily
            )
        });
        let unresolved_blockers = blockers
            .iter()
            .any(|blocker| *blocker != Blocker::NonInferiorityViolation);
        let decision = if not_comparable {
            Decision::NotComparable
        } else if blockers.contains(&Blocker::DefiniteHardLimitViolation) {
            Decision::Fail
        } else if unresolved_blockers
            || any_baseline_invalid
            || !all_comparators_present
            || candidate_hard.unresolved
        {
            Decision::Indeterminate
        } else if any_noninferiority_violation
            || baseline_comparisons
                .iter()
                .any(|comparison| !comparison.all_mandatory_metrics_non_inferior)
        {
            Decision::InferiorToBaseline
        } else if benefits.is_empty() {
            Decision::NonInferiorNoStackBenefit
        } else {
            Decision::StackAdvantage
        };

        results.push(Evaluation {
            architecture_id: candidate.architecture_id.clone(),
            decision,
            blockers: dedupe(blockers),
            baseline_comparisons,
            robust_stack_benefits: benefits,
        });
    }
    results
}

fn validate_spec(spec: &BenchmarkSpec) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    let mut ids = BTreeSet::new();
    let mut mandatory = 0usize;
    let mut benefits = 0usize;

    if spec.benchmark_id.trim().is_empty()
        || spec.scope.application_id.trim().is_empty()
        || !valid_sha256(&spec.scope.comparison_scope_digest)
        || [
            &spec.scope.duty_digest,
            &spec.scope.waveform_digest,
            &spec.scope.coolant_digest,
            &spec.scope.thermal_boundary_digest,
            &spec.scope.insulation_policy_digest,
            &spec.scope.measurement_setup_digest,
            &spec.scope.acceptance_policy_digest,
            &spec.scope.objective_digest,
            &spec.scope.constraint_digest,
        ]
        .iter()
        .any(|digest| !valid_sha256(digest))
    {
        blockers.push(Blocker::InvalidBenchmarkSpec);
    }

    for metric in &spec.metrics {
        if metric.metric_id.trim().is_empty()
            || metric.unit.trim().is_empty()
            || !ids.insert(metric.metric_id.as_str())
            || metric.hard_min.is_some_and(|v| !v.is_finite())
            || metric.hard_max.is_some_and(|v| !v.is_finite())
            || metric
                .hard_min
                .zip(metric.hard_max)
                .is_some_and(|(min, max)| min > max)
        {
            blockers.push(Blocker::InvalidBenchmarkSpec);
        }

        match metric.class {
            MetricClass::MandatoryFunction => {
                mandatory += 1;
                if metric
                    .non_inferiority_margin
                    .is_none_or(|v| !v.is_finite() || v < 0.0)
                    || (metric.hard_min.is_none() && metric.hard_max.is_none())
                {
                    blockers.push(Blocker::InvalidBenchmarkSpec);
                }
            }
            MetricClass::StackBenefit => {
                benefits += 1;
                if metric.hard_min.is_some() || metric.hard_max.is_some() {
                    blockers.push(Blocker::InvalidBenchmarkSpec);
                }
            }
        }
    }

    if mandatory == 0
        || benefits == 0
        || spec.required_baseline_families.is_empty()
        || !spec
            .required_baseline_families
            .contains(&Family::ConventionalWindingCooling)
        || spec.minimum_baseline_evaluations == 0
        || spec.minimum_independent_restarts == 0
    {
        blockers.push(Blocker::InvalidBenchmarkSpec);
    }

    const REQUIRED_FUNCTION_METRICS: [(&str, Direction, bool); 7] = [
        ("winding_hotspot_temperature", Direction::Minimize, true),
        ("ac_winding_loss", Direction::Minimize, true),
        ("dielectric_margin", Direction::Maximize, false),
        ("coolant_pressure_drop", Direction::Minimize, true),
        ("pump_input_power", Direction::Minimize, true),
        ("leak_rate", Direction::Minimize, true),
        ("thermal_cycle_survival", Direction::Maximize, false),
    ];
    for (metric_id, direction, requires_hard_max) in REQUIRED_FUNCTION_METRICS {
        let valid = spec.metrics.iter().any(|metric| {
            metric.metric_id == metric_id
                && metric.class == MetricClass::MandatoryFunction
                && metric.direction == direction
                && if requires_hard_max {
                    metric.hard_max.is_some() && metric.hard_min.is_none()
                } else {
                    metric.hard_min.is_some() && metric.hard_max.is_none()
                }
        });
        if !valid {
            blockers.push(Blocker::InvalidBenchmarkSpec);
        }
    }

    const CONVENTIONAL_FAMILIES: [Family; 7] = [
        Family::ConventionalWindingCooling,
        Family::ConventionalHighPerformanceCooling,
        Family::ConventionalDirectOilCooling,
        Family::ConventionalMultiNozzleOilJetCooling,
        Family::ConventionalHairpinOilSprayCooling,
        Family::ConventionalSlotEmbeddedCooling,
        Family::ConventionalHollowConductorCooling,
    ];
    let mut applicability_by_family = BTreeMap::new();
    for record in &spec.baseline_applicability {
        if !CONVENTIONAL_FAMILIES.contains(&record.family)
            || record.rationale.trim().is_empty()
            || record.supporting_evidence_ids.is_empty()
            || record
                .supporting_evidence_ids
                .iter()
                .any(|id| id.trim().is_empty())
            || !valid_sha256(&record.decision_digest)
            || applicability_by_family
                .insert(record.family, record)
                .is_some()
        {
            blockers.push(Blocker::InvalidBenchmarkSpec);
        }
    }
    for family in CONVENTIONAL_FAMILIES {
        let Some(record) = applicability_by_family.get(&family) else {
            blockers.push(Blocker::InvalidBenchmarkSpec);
            continue;
        };
        if record.applicable != spec.required_baseline_families.contains(&family) {
            blockers.push(Blocker::InvalidBenchmarkSpec);
        }
    }
    let mut baseline_families = BTreeSet::new();
    for family in &spec.required_baseline_families {
        if !CONVENTIONAL_FAMILIES.contains(family) || !baseline_families.insert(*family) {
            blockers.push(Blocker::InvalidBenchmarkSpec);
        }
    }

    blockers
}

fn validate_acceptance(trial: &Trial, spec: &BenchmarkSpec, blockers: &mut Vec<Blocker>) {
    let acceptance = &trial.acceptance;
    if acceptance.component_id != trial.component_id
        || acceptance.specimen_id.trim().is_empty()
        || acceptance.build_id.trim().is_empty()
        || acceptance.functional_test_evidence_id.trim().is_empty()
        || acceptance.linked_evidence_ids.is_empty()
        || !valid_sha256(&acceptance.functional_test_digest)
        || acceptance.maturity < Maturity::M4
        || acceptance.disposition != Disposition::Qualified
        || !valid_sha256(&acceptance.acceptance_policy_digest)
        || acceptance.comparison_scope_digest != spec.scope.comparison_scope_digest
        || !valid_sha256(&acceptance.comparison_scope_digest)
        || !valid_sha256(&acceptance.receipt_digest)
        || !valid_sha256(&acceptance.physical_evidence_digest)
    {
        blockers.push(Blocker::InvalidAcceptance);
    }
}

fn validate_optimization(baseline: &Trial, spec: &BenchmarkSpec, blockers: &mut Vec<Blocker>) {
    let Some(receipt) = &baseline.optimization else {
        blockers.push(Blocker::InvalidOptimizationReceipt);
        return;
    };

    if !valid_sha256(&receipt.receipt_digest)
        || !valid_sha256(&receipt.search_space_digest)
        || receipt.objective_digest != spec.scope.objective_digest
        || receipt.constraint_digest != spec.scope.constraint_digest
        || receipt.evaluations < spec.minimum_baseline_evaluations
        || receipt.independent_restarts < spec.minimum_independent_restarts
        || !receipt.frozen_before_comparison
    {
        blockers.push(Blocker::InvalidOptimizationReceipt);
    }
}

fn validate_observations(
    trial: &Trial,
    spec: &BenchmarkSpec,
    blockers: &mut Vec<Blocker>,
) -> BTreeMap<String, Interval> {
    let mut by_id = BTreeMap::new();
    let mut evidence_ids = BTreeSet::new();
    let mut metric_ids = BTreeSet::new();

    for observation in &trial.observations {
        let mut valid = true;
        if !metric_ids.insert(observation.metric_id.as_str()) {
            blockers.push(Blocker::DuplicateObservation);
            valid = false;
        }
        if observation.evidence_id.trim().is_empty()
            || !evidence_ids.insert(observation.evidence_id.as_str())
        {
            blockers.push(Blocker::DuplicateObservation);
            valid = false;
        }
        if observation.comparison_scope_digest != spec.scope.comparison_scope_digest
            || observation.component_id != trial.component_id
            || observation.specimen_id != trial.acceptance.specimen_id
            || observation.build_id != trial.acceptance.build_id
            || observation.material_lot_id.trim().is_empty()
        {
            blockers.push(Blocker::BrokenSpecimenBinding);
            valid = false;
        }
        if observation.evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::InadmissibleEvidenceClass);
            valid = false;
        }
        if observation.method_id.trim().is_empty()
            || !valid_sha256(&observation.evidence_digest)
            || !valid_sha256(&observation.calibration_digest)
            || !valid_sha256(&observation.uncertainty_digest)
        {
            blockers.push(Blocker::MissingMeasurementProvenance);
            valid = false;
        }
        if !observation.value.is_finite()
            || !observation.uncertainty_radius.is_finite()
            || observation.uncertainty_radius < 0.0
        {
            blockers.push(Blocker::InvalidObservation);
            valid = false;
        }
        if let Some(metric) = spec
            .metrics
            .iter()
            .find(|m| m.metric_id == observation.metric_id)
        {
            if metric.unit != observation.unit {
                blockers.push(Blocker::InvalidObservation);
                valid = false;
            }
        } else {
            blockers.push(Blocker::InvalidObservation);
            valid = false;
        }
        if valid {
            by_id.insert(observation.metric_id.as_str(), observation);
        }
    }

    let linked_ids: BTreeSet<&str> = trial
        .acceptance
        .linked_evidence_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut expected_linked_ids = evidence_ids.clone();
    expected_linked_ids.insert(trial.acceptance.functional_test_evidence_id.as_str());
    if linked_ids.len() != trial.acceptance.linked_evidence_ids.len()
        || linked_ids != expected_linked_ids
    {
        blockers.push(Blocker::BrokenSpecimenBinding);
    }

    let mut intervals = BTreeMap::new();
    for metric in &spec.metrics {
        let Some(observation) = by_id.get(metric.metric_id.as_str()) else {
            blockers.push(Blocker::MissingObservation);
            continue;
        };
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

struct HardStatus {
    blockers: Vec<Blocker>,
    unresolved: bool,
}

fn hard_limit_status(intervals: &BTreeMap<String, Interval>, metrics: &[MetricSpec]) -> HardStatus {
    let mut blockers = Vec::new();
    let mut unresolved = false;

    for metric in metrics
        .iter()
        .filter(|m| m.class == MetricClass::MandatoryFunction)
    {
        let Some(interval) = intervals.get(&metric.metric_id) else {
            unresolved = true;
            continue;
        };

        if metric.hard_min.is_some_and(|limit| interval.upper < limit)
            || metric.hard_max.is_some_and(|limit| interval.lower > limit)
        {
            blockers.push(Blocker::DefiniteHardLimitViolation);
        } else if metric.hard_min.is_some_and(|limit| interval.lower < limit)
            || metric.hard_max.is_some_and(|limit| interval.upper > limit)
        {
            blockers.push(Blocker::UnresolvedHardLimit);
            unresolved = true;
        }
    }

    HardStatus {
        blockers: dedupe(blockers),
        unresolved,
    }
}

struct Comparison {
    non_inferior: bool,
    robust_benefits: Vec<String>,
}

fn compare_metrics(
    spec: &BenchmarkSpec,
    candidate: &BTreeMap<String, Interval>,
    baseline: &BTreeMap<String, Interval>,
    blockers: &mut Vec<Blocker>,
) -> Comparison {
    let mut non_inferior = true;
    let mut robust_benefits = Vec::new();

    for metric in &spec.metrics {
        let (Some(c), Some(b)) = (
            candidate.get(&metric.metric_id),
            baseline.get(&metric.metric_id),
        ) else {
            blockers.push(Blocker::MissingObservation);
            non_inferior = false;
            continue;
        };

        match metric.class {
            MetricClass::MandatoryFunction => {
                let margin = metric.non_inferiority_margin.unwrap_or(0.0);
                let safe = match metric.direction {
                    Direction::Maximize => c.lower >= b.upper - margin,
                    Direction::Minimize => c.upper <= b.lower + margin,
                };
                if !safe {
                    blockers.push(Blocker::NonInferiorityViolation);
                    non_inferior = false;
                }
            }
            MetricClass::StackBenefit => {
                let improved = match metric.direction {
                    Direction::Maximize => c.lower > b.upper,
                    Direction::Minimize => c.upper < b.lower,
                };
                if improved {
                    robust_benefits.push(metric.metric_id.clone());
                }
            }
        }
    }

    Comparison {
        non_inferior,
        robust_benefits,
    }
}

fn common_robust_benefits(comparisons: &[BaselineComparison]) -> Vec<String> {
    let Some(first) = comparisons.first() else {
        return Vec::new();
    };
    first
        .robust_benefits
        .iter()
        .filter(|metric| {
            comparisons
                .iter()
                .all(|comparison| comparison.robust_benefits.contains(metric))
        })
        .cloned()
        .collect()
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

    fn scope() -> Scope {
        Scope {
            application_id: "MOTOR-50KW-001".to_string(),
            comparison_scope_digest: digest('0'),
            duty_digest: digest('a'),
            waveform_digest: digest('b'),
            coolant_digest: digest('c'),
            thermal_boundary_digest: digest('d'),
            insulation_policy_digest: digest('e'),
            measurement_setup_digest: digest('f'),
            acceptance_policy_digest: digest('1'),
            objective_digest: digest('2'),
            constraint_digest: digest('3'),
        }
    }

    fn spec() -> BenchmarkSpec {
        BenchmarkSpec {
            benchmark_id: "THERMO-STACK-001".to_string(),
            scope: scope(),
            baseline_applicability: vec![
                BaselineApplicability {
                    family: Family::ConventionalWindingCooling,
                    applicable: true,
                    rationale: "Synthetic conventional comparator for known-answer semantics"
                        .to_string(),
                    supporting_evidence_ids: vec!["BASELINE-SOURCE-001".to_string()],
                    decision_digest: digest('a'),
                },
                BaselineApplicability {
                    family: Family::ConventionalHighPerformanceCooling,
                    applicable: false,
                    rationale: "Synthetic exclusion for known-answer semantics".to_string(),
                    supporting_evidence_ids: vec!["EXCLUSION-SOURCE-001".to_string()],
                    decision_digest: digest('b'),
                },
                BaselineApplicability {
                    family: Family::ConventionalDirectOilCooling,
                    applicable: false,
                    rationale: "Synthetic exclusion for known-answer semantics".to_string(),
                    supporting_evidence_ids: vec!["EXCLUSION-SOURCE-002".to_string()],
                    decision_digest: digest('c'),
                },
                BaselineApplicability {
                    family: Family::ConventionalMultiNozzleOilJetCooling,
                    applicable: false,
                    rationale: "Synthetic exclusion for known-answer semantics".to_string(),
                    supporting_evidence_ids: vec!["EXCLUSION-SOURCE-003".to_string()],
                    decision_digest: digest('d'),
                },
                BaselineApplicability {
                    family: Family::ConventionalHairpinOilSprayCooling,
                    applicable: false,
                    rationale: "Synthetic exclusion for known-answer semantics".to_string(),
                    supporting_evidence_ids: vec!["EXCLUSION-SOURCE-004".to_string()],
                    decision_digest: digest('e'),
                },
                BaselineApplicability {
                    family: Family::ConventionalSlotEmbeddedCooling,
                    applicable: false,
                    rationale: "Synthetic exclusion for known-answer semantics".to_string(),
                    supporting_evidence_ids: vec!["EXCLUSION-SOURCE-005".to_string()],
                    decision_digest: digest('f'),
                },
                BaselineApplicability {
                    family: Family::ConventionalHollowConductorCooling,
                    applicable: false,
                    rationale: "Synthetic exclusion for known-answer semantics".to_string(),
                    supporting_evidence_ids: vec!["EXCLUSION-SOURCE-006".to_string()],
                    decision_digest: digest('1'),
                },
            ],
            metrics: vec![
                MetricSpec {
                    metric_id: "winding_hotspot_temperature".to_string(),
                    unit: "degC".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(160.0),
                    non_inferiority_margin: Some(5.0),
                },
                MetricSpec {
                    metric_id: "ac_winding_loss".to_string(),
                    unit: "W".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(1800.0),
                    non_inferiority_margin: Some(50.0),
                },
                MetricSpec {
                    metric_id: "dielectric_margin".to_string(),
                    unit: "ratio".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Maximize,
                    hard_min: Some(1.5),
                    hard_max: None,
                    non_inferiority_margin: Some(0.1),
                },
                MetricSpec {
                    metric_id: "coolant_pressure_drop".to_string(),
                    unit: "kPa".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(45.0),
                    non_inferiority_margin: Some(2.0),
                },
                MetricSpec {
                    metric_id: "pump_input_power".to_string(),
                    unit: "W".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(180.0),
                    non_inferiority_margin: Some(10.0),
                },
                MetricSpec {
                    metric_id: "leak_rate".to_string(),
                    unit: "mL/h".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(0.01),
                    non_inferiority_margin: Some(0.002),
                },
                MetricSpec {
                    metric_id: "thermal_cycle_survival".to_string(),
                    unit: "cycles".to_string(),
                    class: MetricClass::MandatoryFunction,
                    direction: Direction::Maximize,
                    hard_min: Some(1000.0),
                    hard_max: None,
                    non_inferiority_margin: Some(100.0),
                },
                MetricSpec {
                    metric_id: "effective_thermal_resistance".to_string(),
                    unit: "K/W".to_string(),
                    class: MetricClass::StackBenefit,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "system_mass".to_string(),
                    unit: "kg".to_string(),
                    class: MetricClass::StackBenefit,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
            ],
            required_baseline_families: vec![Family::ConventionalWindingCooling],
            minimum_baseline_evaluations: 100,
            minimum_independent_restarts: 3,
        }
    }

    fn acceptance(id: &str) -> ComponentAcceptance {
        ComponentAcceptance {
            component_id: id.to_string(),
            specimen_id: format!("SPEC-{id}"),
            build_id: format!("BUILD-{id}"),
            functional_test_evidence_id: String::new(),
            functional_test_digest: digest('5'),
            linked_evidence_ids: Vec::new(),
            maturity: Maturity::M4,
            disposition: Disposition::Qualified,
            acceptance_policy_digest: digest('1'),
            comparison_scope_digest: digest('0'),
            receipt_digest: digest('4'),
            physical_evidence_digest: digest('5'),
        }
    }

    fn optimization() -> OptimizationReceipt {
        OptimizationReceipt {
            receipt_digest: digest('6'),
            search_space_digest: digest('7'),
            objective_digest: digest('2'),
            constraint_digest: digest('3'),
            evaluations: 500,
            independent_restarts: 5,
            frozen_before_comparison: true,
        }
    }

    fn trial(id: &str, family: Family, role: Role, values: [f64; 9]) -> Trial {
        let component_id = format!("COMP-{id}");
        let specimen_id = format!("SPEC-{component_id}");
        let build_id = format!("BUILD-{component_id}");
        let metric_values = [
            ("winding_hotspot_temperature", "degC", values[0], 1.0, 'a'),
            ("ac_winding_loss", "W", values[1], 5.0, 'b'),
            ("dielectric_margin", "ratio", values[2], 0.02, 'c'),
            ("coolant_pressure_drop", "kPa", values[3], 0.5, 'd'),
            ("pump_input_power", "W", values[4], 2.0, 'e'),
            ("leak_rate", "mL/h", values[5], 0.001, 'f'),
            ("thermal_cycle_survival", "cycles", values[6], 10.0, '1'),
            ("effective_thermal_resistance", "K/W", values[7], 0.01, '2'),
            ("system_mass", "kg", values[8], 0.1, '3'),
        ];
        let observations: Vec<Observation> = metric_values
            .into_iter()
            .map(
                |(metric_id, unit, value, uncertainty_radius, suffix)| Observation {
                    metric_id: metric_id.to_string(),
                    comparison_scope_digest: digest('0'),
                    component_id: component_id.clone(),
                    specimen_id: specimen_id.clone(),
                    build_id: build_id.clone(),
                    material_lot_id: format!("LOT-{metric_id}"),
                    unit: unit.to_string(),
                    value,
                    uncertainty_radius,
                    evidence_id: format!("OBS-{id}-{metric_id}"),
                    evidence_digest: digest(suffix),
                    evidence_class: EvidenceClass::PhysicalMeasurement,
                    method_id: format!("METHOD-{metric_id}"),
                    calibration_digest: digest('8'),
                    uncertainty_digest: digest('9'),
                },
            )
            .collect();

        Trial {
            architecture_id: id.to_string(),
            component_id: component_id.clone(),
            family,
            role,
            scope: scope(),
            acceptance: {
                let mut accepted = acceptance(&component_id);
                accepted.specimen_id = specimen_id;
                accepted.build_id = build_id;
                accepted.functional_test_evidence_id = format!("FUNCTIONAL-{id}");
                accepted.linked_evidence_ids = observations
                    .iter()
                    .map(|observation| observation.evidence_id.clone())
                    .chain(std::iter::once(
                        accepted.functional_test_evidence_id.clone(),
                    ))
                    .collect();
                accepted
            },
            optimization: Some(optimization()),
            observations,
        }
    }

    fn known_trials() -> Vec<Trial> {
        vec![
            trial(
                "AM-STACK",
                Family::AmIntegratedWindingCooling,
                Role::Candidate,
                [130.0, 1500.0, 1.8, 34.0, 130.0, 0.0, 1300.0, 0.08, 8.0],
            ),
            trial(
                "CONVENTIONAL",
                Family::ConventionalWindingCooling,
                Role::RequiredBaseline,
                [145.0, 1600.0, 1.7, 38.0, 150.0, 0.0, 1250.0, 0.11, 9.0],
            ),
        ]
    }

    #[test]
    fn qualified_stack_is_advantage_when_same_benefit_beats_all_baselines() {
        let results = evaluate(&spec(), &known_trials());
        assert_eq!(results[0].decision, Decision::StackAdvantage);
        assert!(
            results[0]
                .robust_stack_benefits
                .contains(&"effective_thermal_resistance".to_string())
        );
    }

    #[test]
    fn benchmark_spec_requires_all_seven_mandatory_function_metrics() {
        let mut benchmark = spec();
        benchmark
            .metrics
            .retain(|metric| metric.metric_id != "ac_winding_loss");
        let result = evaluate(&benchmark, &known_trials());
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::InvalidBenchmarkSpec));
    }

    #[test]
    fn evidence_ids_must_be_unique_across_candidate_and_baseline() {
        let mut trials = known_trials();
        trials[1].observations[0].evidence_id = trials[0].observations[0].evidence_id.clone();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            result[0]
                .blockers
                .contains(&Blocker::DuplicateEvidenceIdentity)
        );
    }

    #[test]
    fn synthetic_measurements_cannot_establish_stack_advantage() {
        let mut trials = known_trials();
        trials[0].observations[0].evidence_class = EvidenceClass::Synthetic;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            result[0]
                .blockers
                .contains(&Blocker::InadmissibleEvidenceClass)
        );
    }

    #[test]
    fn thermal_gain_cannot_compensate_for_dielectric_failure() {
        let mut trials = known_trials();
        trials[0].observations[2].value = 1.2;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn pressure_drop_is_a_non_compensatory_requirement() {
        let mut trials = known_trials();
        trials[0].observations[3].value = 50.0;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn uncertainty_touching_hard_limit_is_indeterminate() {
        let mut trials = known_trials();
        trials[0].observations[2].value = 1.51;
        trials[0].observations[2].uncertainty_radius = 0.02;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
    }

    #[test]
    fn leak_rate_is_a_non_compensatory_requirement() {
        let mut trials = known_trials();
        trials[0].observations[5].value = 0.03;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn missing_dielectric_observation_is_not_defaulted() {
        let mut trials = known_trials();
        trials[0]
            .observations
            .retain(|item| item.metric_id != "dielectric_margin");
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::MissingObservation));
    }

    #[test]
    fn wrong_waveform_makes_comparison_not_comparable() {
        let mut trials = known_trials();
        trials[0].scope.waveform_digest = digest('a');
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn coolant_mismatch_makes_comparison_not_comparable() {
        let mut trials = known_trials();
        trials[0].scope.coolant_digest = digest('a');
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
    }

    #[test]
    fn missing_conventional_baseline_is_not_comparable() {
        let mut trials = known_trials();
        trials.remove(1);
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(
            result[0]
                .blockers
                .contains(&Blocker::MissingRequiredBaseline)
        );
    }

    #[test]
    fn unoptimized_baseline_is_not_a_valid_comparator() {
        let mut trials = known_trials();
        trials[1].optimization.as_mut().unwrap().evaluations = 1;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            result[0]
                .blockers
                .contains(&Blocker::InvalidOptimizationReceipt)
        );
    }

    #[test]
    fn noninferiority_failure_is_inferior_to_baseline() {
        let mut trials = known_trials();
        trials[0].observations[1].value = 1750.0;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::InferiorToBaseline);
    }

    #[test]
    fn overlapping_benefit_intervals_are_not_an_advantage() {
        let mut trials = known_trials();
        trials[0].observations[7].value = 0.105;
        trials[0].observations[7].uncertainty_radius = 0.02;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NonInferiorNoStackBenefit);
    }

    #[test]
    fn pump_power_penalty_cannot_hide_inside_thermal_improvement() {
        let mut trials = known_trials();
        trials[0].observations[4].value = 190.0;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn duplicate_metric_observations_invalidate_evidence() {
        let mut trials = known_trials();
        let duplicate = trials[0].observations[0].clone();
        trials[0].observations.push(duplicate);
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::DuplicateObservation));
    }

    #[test]
    fn candidate_must_have_a_fair_optimization_receipt() {
        let mut trials = known_trials();
        trials[0].optimization.as_mut().unwrap().evaluations = 1;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            result[0]
                .blockers
                .contains(&Blocker::InvalidOptimizationReceipt)
        );
    }

    #[test]
    fn acceptance_scope_mismatch_cannot_create_a_false_hard_fail() {
        let mut trials = known_trials();
        trials[0].observations[0].value = 500.0;
        trials[0].acceptance.comparison_scope_digest = digest('f');
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            !result[0]
                .blockers
                .contains(&Blocker::DefiniteHardLimitViolation)
        );
        assert!(result[0].blockers.contains(&Blocker::InvalidAcceptance));
    }

    #[test]
    fn wrong_measurement_scope_cannot_create_a_false_hard_fail() {
        let mut trials = known_trials();
        trials[0].observations[0].value = 500.0;
        trials[0].observations[0].comparison_scope_digest = digest('f');
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            !result[0]
                .blockers
                .contains(&Blocker::DefiniteHardLimitViolation)
        );
        assert!(result[0].blockers.contains(&Blocker::BrokenSpecimenBinding));
    }

    #[test]
    fn invalid_specimen_binding_cannot_create_a_false_hard_fail() {
        let mut trials = known_trials();
        trials[0].observations[0].value = 500.0;
        trials[0].observations[0].specimen_id = "SPEC-OTHER".to_string();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(
            !result[0]
                .blockers
                .contains(&Blocker::DefiniteHardLimitViolation)
        );
        assert!(result[0].blockers.contains(&Blocker::BrokenSpecimenBinding));
    }

    #[test]
    fn candidate_acceptance_must_be_m4_and_scope_bound() {
        let mut trials = known_trials();
        trials[0].acceptance.maturity = Maturity::M3;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::InvalidAcceptance));
    }

    #[test]
    fn acceptance_must_link_every_measurement_and_functional_test_receipt() {
        let mut trials = known_trials();
        trials[0].acceptance.linked_evidence_ids.pop();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::BrokenSpecimenBinding));
    }

    #[test]
    fn thermal_cycle_survival_is_not_decorative() {
        let mut trials = known_trials();
        trials[0].observations[6].value = 700.0;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn another_comparator_must_also_be_beaten() {
        let mut benchmark = spec();
        benchmark
            .required_baseline_families
            .push(Family::ConventionalDirectOilCooling);
        let oil_record = benchmark
            .baseline_applicability
            .iter_mut()
            .find(|record| record.family == Family::ConventionalDirectOilCooling)
            .expect("direct-oil applicability decision");
        oil_record.applicable = true;
        oil_record.rationale = "Synthetic direct-oil comparator is applicable".to_string();
        let mut trials = known_trials();
        trials.push(trial(
            "CONVENTIONAL-HIGH",
            Family::ConventionalDirectOilCooling,
            Role::RequiredBaseline,
            [132.0, 1470.0, 1.8, 33.0, 128.0, 0.0, 1280.0, 0.075, 7.8],
        ));
        let result = evaluate(&benchmark, &trials);
        assert_eq!(result[0].decision, Decision::NonInferiorNoStackBenefit);
    }

    #[test]
    fn every_conventional_family_requires_an_applicability_decision() {
        let mut benchmark = spec();
        benchmark
            .baseline_applicability
            .retain(|record| record.family != Family::ConventionalDirectOilCooling);
        let result = evaluate(&benchmark, &known_trials());
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::InvalidBenchmarkSpec));
    }

    #[test]
    fn applicable_direct_oil_baseline_cannot_be_omitted() {
        let mut benchmark = spec();
        benchmark
            .baseline_applicability
            .iter_mut()
            .find(|record| record.family == Family::ConventionalDirectOilCooling)
            .expect("direct-oil applicability decision")
            .applicable = true;
        let result = evaluate(&benchmark, &known_trials());
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::InvalidBenchmarkSpec));
    }

    #[test]
    fn non_applicable_baseline_requires_rationale_and_evidence() {
        let mut benchmark = spec();
        let record = benchmark
            .baseline_applicability
            .iter_mut()
            .find(|record| record.family == Family::ConventionalDirectOilCooling)
            .expect("direct-oil applicability decision");
        record.rationale.clear();
        record.supporting_evidence_ids.clear();
        let result = evaluate(&benchmark, &known_trials());
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::InvalidBenchmarkSpec));
    }

    #[test]
    fn secondary_control_cannot_replace_conventional_reference() {
        let mut trials = known_trials();
        trials[1].role = Role::SecondaryControl;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
    }
}
