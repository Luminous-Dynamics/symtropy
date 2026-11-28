//! Evidence-gated comparison of serviceable and circular PM rotor designs.
//!
//! Synthetic tests validate decision semantics only. This evaluator does not
//! authenticate issuers, certify standards, qualify a recycler or establish an
//! audited life-cycle assessment.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    ConventionalBondedAdhesivePmRotor,
    ConventionalMechanicallyRetainedSegmentedPmRotor,
    ConventionalServiceablePmRotor,
    AmModularRecoverableRotor,
    MagnetFreeReferenceControl,
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
    CircularityBenefit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricSource {
    Observation,
    DisassemblyElapsedMinutes,
    DisassemblyEnergyWh,
    DirectReuseFraction,
    RemanufacturedAcceptedFraction,
    RecyclingFeedstockFraction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    AuditedCostRecord,
    Simulation,
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
    NonInferiorNoCircularityBenefit,
    CircularityAdvantage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub application_id: String,
    pub duty_digest: String,
    pub lifetime_profile_digest: String,
    pub thermal_environment_digest: String,
    pub mechanical_envelope_digest: String,
    pub safety_policy_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
    pub lifecycle_boundary_digest: String,
    pub objective_digest: String,
    pub constraint_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricSpec {
    pub metric_id: String,
    pub unit: String,
    pub class: MetricClass,
    pub source: MetricSource,
    pub direction: Direction,
    pub hard_min: Option<f64>,
    pub hard_max: Option<f64>,
    pub non_inferiority_margin: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkSpec {
    pub benchmark_id: String,
    pub scope: Scope,
    pub metrics: Vec<MetricSpec>,
    pub required_baseline_families: Vec<Family>,
    pub minimum_baseline_evaluations: u32,
    pub minimum_independent_restarts: u32,
    pub max_mass_balance_error_kg: f64,
    pub max_mass_balance_error_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentAcceptance {
    pub component_id: String,
    pub specimen_id: String,
    pub build_id: String,
    pub material_lot_id: String,
    pub maturity: Maturity,
    pub disposition: Disposition,
    pub scope_digest: String,
    pub receipt_digest: String,
    pub functional_test_evidence_id: String,
    pub functional_test_digest: String,
    pub linked_evidence_ids: Vec<String>,
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
pub struct DisassemblyEvidence {
    pub component_id: String,
    pub specimen_id: String,
    pub build_id: String,
    pub material_lot_id: String,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_digest: String,
    pub elapsed_minutes: f64,
    pub elapsed_uncertainty_minutes: f64,
    pub energy_wh: f64,
    pub energy_uncertainty_wh: f64,
    pub safe_state_confirmed: bool,
    pub worker_safety_confirmed: bool,
    pub component_integrity_pass: bool,
    pub critical_damage_events: u32,
    pub recovery_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryPathReceipt {
    pub evidence_id: String,
    pub evidence_digest: String,
    pub output_lot_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryReceipt {
    pub component_id: String,
    pub specimen_id: String,
    pub build_id: String,
    pub material_lot_id: String,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_digest: String,
    pub input_magnet_mass_kg: f64,
    pub input_uncertainty_kg: f64,
    pub reuse_grade_mass_kg: f64,
    pub reuse_grade_uncertainty_kg: f64,
    pub remanufactured_accepted_mass_kg: f64,
    pub remanufactured_uncertainty_kg: f64,
    pub recycling_feedstock_mass_kg: f64,
    pub recycling_uncertainty_kg: f64,
    pub reject_waste_mass_kg: f64,
    pub reject_uncertainty_kg: f64,
    pub reuse_acceptance_receipt: Option<RecoveryPathReceipt>,
    pub remanufacture_acceptance_receipt: Option<RecoveryPathReceipt>,
    pub recycling_intake_receipt: Option<RecoveryPathReceipt>,
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
    pub disassembly: DisassemblyEvidence,
    pub recovery: RecoveryReceipt,
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
    MissingConventionalPmBaseline,
    ScopeMismatch,
    WrongCandidateFamily,
    InvalidAcceptance,
    BrokenSpecimenBinding,
    MissingObservation,
    DuplicateObservation,
    InvalidObservation,
    InadmissibleEvidenceClass,
    MissingMeasurementProvenance,
    InvalidOptimizationReceipt,
    InvalidDisassemblyEvidence,
    UnsafeDisassembly,
    InvalidRecoveryReceipt,
    UnclosedMassBalance,
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
    pub robust_circularity_benefits: Vec<String>,
}

pub fn evaluate(spec: &BenchmarkSpec, trials: &[Trial]) -> Vec<Evaluation> {
    let mut global_blockers = validate_spec(spec);
    let mut architecture_ids = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();

    for trial in trials {
        if !architecture_ids.insert(trial.architecture_id.as_str()) {
            global_blockers.push(Blocker::DuplicateArchitectureId);
        }
        if !component_ids.insert(trial.component_id.as_str()) {
            global_blockers.push(Blocker::DuplicateComponentId);
        }
        for evidence_id in trial_evidence_ids(trial) {
            if evidence_id.trim().is_empty() || !evidence_ids.insert(evidence_id) {
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
            || candidate.acceptance.scope_digest != spec.scope.acceptance_policy_digest
        {
            blockers.push(Blocker::ScopeMismatch);
        }
        if candidate.family != Family::AmModularRecoverableRotor {
            blockers.push(Blocker::WrongCandidateFamily);
        }

        validate_acceptance(candidate, &mut blockers);
        validate_optimization(candidate, spec, &mut blockers);
        let candidate_intervals = trial_intervals(candidate, spec, &mut blockers);
        let candidate_hard = hard_limit_status(&candidate_intervals, &spec.metrics);
        blockers.extend(candidate_hard.blockers.clone());

        let mut baseline_comparisons = Vec::new();
        let mut any_baseline_invalid = false;
        let mut present_families = BTreeSet::new();

        for family in &spec.required_baseline_families {
            let matching: Vec<&Trial> = trials
                .iter()
                .filter(|trial| {
                    trial.role == Role::RequiredBaseline && trial.family == *family
                })
                .collect();

            if matching.is_empty() {
                blockers.push(Blocker::MissingRequiredBaseline);
                any_baseline_invalid = true;
                continue;
            }

            for baseline in matching {
                present_families.insert(baseline.family);
                if baseline.scope != spec.scope
                    || baseline.acceptance.scope_digest != spec.scope.acceptance_policy_digest
                {
                    blockers.push(Blocker::ScopeMismatch);
                    any_baseline_invalid = true;
                    continue;
                }

                let mut baseline_blockers = Vec::new();
                validate_acceptance(baseline, &mut baseline_blockers);
                validate_optimization(baseline, spec, &mut baseline_blockers);
                let intervals = trial_intervals(baseline, spec, &mut baseline_blockers);
                let hard = hard_limit_status(&intervals, &spec.metrics);
                baseline_blockers.extend(hard.blockers);

                if !baseline_blockers.is_empty() {
                    any_baseline_invalid = true;
                    if baseline_blockers.contains(&Blocker::DefiniteHardLimitViolation)
                        || baseline_blockers.contains(&Blocker::UnsafeDisassembly)
                    {
                        blockers.push(Blocker::BaselineFailsHardLimit);
                    } else {
                        blockers.push(Blocker::BaselineNotAdmissible);
                    }
                    continue;
                }

                let comparison = compare_metrics(
                    spec,
                    &candidate_intervals,
                    &intervals,
                    &mut blockers,
                );
                baseline_comparisons.push(BaselineComparison {
                    baseline_id: baseline.architecture_id.clone(),
                    all_mandatory_metrics_non_inferior: comparison.non_inferior,
                    robust_benefits: comparison.robust_benefits,
                });
            }
        }

        if !spec
            .required_baseline_families
            .iter()
            .any(|family| is_conventional_pm_family(*family))
        {
            blockers.push(Blocker::MissingConventionalPmBaseline);
            any_baseline_invalid = true;
        }
        if spec
            .required_baseline_families
            .iter()
            .any(|family| !present_families.contains(family))
        {
            any_baseline_invalid = true;
        }

        let all_comparators_present = spec
            .required_baseline_families
            .iter()
            .all(|family| present_families.contains(family))
            && !baseline_comparisons.is_empty();
        let noninferiority_failure = blockers.contains(&Blocker::NonInferiorityViolation)
            || baseline_comparisons
                .iter()
                .any(|comparison| !comparison.all_mandatory_metrics_non_inferior);
        let common_benefits = common_robust_benefits(&baseline_comparisons);

        let not_comparable = blockers.iter().any(|blocker| {
            matches!(
                blocker,
                Blocker::InvalidBenchmarkSpec
                    | Blocker::DuplicateArchitectureId
                    | Blocker::DuplicateComponentId
                    | Blocker::DuplicateEvidenceIdentity
                    | Blocker::MissingCandidate
                    | Blocker::MissingRequiredBaseline
                    | Blocker::MissingConventionalPmBaseline
                    | Blocker::ScopeMismatch
                    | Blocker::WrongCandidateFamily
            )
        });
        let unresolved_blockers = blockers
            .iter()
            .any(|blocker| *blocker != Blocker::NonInferiorityViolation);

        let decision = if not_comparable {
            Decision::NotComparable
        } else if blockers.contains(&Blocker::DefiniteHardLimitViolation)
            || blockers.contains(&Blocker::UnsafeDisassembly)
        {
            Decision::Fail
        } else if unresolved_blockers
            || any_baseline_invalid
            || !all_comparators_present
            || candidate_hard.unresolved
        {
            Decision::Indeterminate
        } else if noninferiority_failure {
            Decision::InferiorToBaseline
        } else if common_benefits.is_empty() {
            Decision::NonInferiorNoCircularityBenefit
        } else {
            Decision::CircularityAdvantage
        };

        results.push(Evaluation {
            architecture_id: candidate.architecture_id.clone(),
            decision,
            blockers: dedupe(blockers),
            baseline_comparisons,
            robust_circularity_benefits: common_benefits,
        });
    }
    results
}

fn validate_spec(spec: &BenchmarkSpec) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    let mut metric_ids = BTreeSet::new();
    let mut mandatory_count = 0usize;
    let mut benefit_count = 0usize;
    let digests = [
        &spec.scope.duty_digest,
        &spec.scope.lifetime_profile_digest,
        &spec.scope.thermal_environment_digest,
        &spec.scope.mechanical_envelope_digest,
        &spec.scope.safety_policy_digest,
        &spec.scope.measurement_setup_digest,
        &spec.scope.acceptance_policy_digest,
        &spec.scope.lifecycle_boundary_digest,
        &spec.scope.objective_digest,
        &spec.scope.constraint_digest,
    ];

    if spec.benchmark_id.trim().is_empty()
        || spec.scope.application_id.trim().is_empty()
        || digests.iter().any(|digest| !valid_sha256(digest))
        || spec.metrics.is_empty()
        || !spec
            .required_baseline_families
            .contains(&Family::ConventionalBondedAdhesivePmRotor)
        || spec.minimum_baseline_evaluations == 0
        || spec.minimum_independent_restarts == 0
        || !spec.max_mass_balance_error_kg.is_finite()
        || spec.max_mass_balance_error_kg < 0.0
        || !spec.max_mass_balance_error_fraction.is_finite()
        || spec.max_mass_balance_error_fraction < 0.0
    {
        blockers.push(Blocker::InvalidBenchmarkSpec);
    }

    for metric in &spec.metrics {
        if metric.metric_id.trim().is_empty()
            || metric.unit.trim().is_empty()
            || !metric_ids.insert(metric.metric_id.as_str())
            || metric.hard_min.is_some_and(|value| !value.is_finite())
            || metric.hard_max.is_some_and(|value| !value.is_finite())
            || metric
                .hard_min
                .zip(metric.hard_max)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            blockers.push(Blocker::InvalidBenchmarkSpec);
        }

        match metric.class {
            MetricClass::MandatoryFunction => {
                mandatory_count += 1;
                if metric.source != MetricSource::Observation
                    || metric.non_inferiority_margin.is_none_or(|margin| {
                        !margin.is_finite() || margin < 0.0
                    })
                    || (metric.hard_min.is_none() && metric.hard_max.is_none())
                {
                    blockers.push(Blocker::InvalidBenchmarkSpec);
                }
            }
            MetricClass::CircularityBenefit => {
                benefit_count += 1;
                if metric.hard_min.is_some()
                    || metric.hard_max.is_some()
                    || metric.non_inferiority_margin.is_some()
                {
                    blockers.push(Blocker::InvalidBenchmarkSpec);
                }
            }
        }
    }

    if mandatory_count < 5 || benefit_count < 4 {
        blockers.push(Blocker::InvalidBenchmarkSpec);
    }
    blockers
}

fn validate_acceptance(trial: &Trial, spec: &BenchmarkSpec, blockers: &mut Vec<Blocker>) {
    let acceptance = &trial.acceptance;
    if acceptance.component_id != trial.component_id
        || acceptance.specimen_id.trim().is_empty()
        || acceptance.build_id.trim().is_empty()
        || acceptance.material_lot_id.trim().is_empty()
        || acceptance.maturity < Maturity::M4
        || acceptance.disposition != Disposition::Qualified
        || acceptance.scope_digest != spec.scope.acceptance_policy_digest
        || !valid_sha256(&acceptance.receipt_digest)
        || acceptance.functional_test_evidence_id.trim().is_empty()
        || !valid_sha256(&acceptance.functional_test_digest)
    {
        blockers.push(Blocker::InvalidAcceptance);
    }

    let mut expected: BTreeSet<&str> = trial
        .observations
        .iter()
        .map(|observation| observation.evidence_id.as_str())
        .collect();
    expected.insert(trial.disassembly.evidence_id.as_str());
    expected.insert(trial.recovery.evidence_id.as_str());
    for receipt in [
        &trial.recovery.reuse_acceptance_receipt,
        &trial.recovery.remanufacture_acceptance_receipt,
        &trial.recovery.recycling_intake_receipt,
    ]
    .into_iter()
    .flatten()
    {
        expected.insert(receipt.evidence_id.as_str());
    }
    expected.insert(acceptance.functional_test_evidence_id.as_str());

    let linked: BTreeSet<&str> = acceptance
        .linked_evidence_ids
        .iter()
        .map(String::as_str)
        .collect();
    if linked.len() != acceptance.linked_evidence_ids.len()
        || linked != expected
        || expected.contains("")
    {
        blockers.push(Blocker::BrokenSpecimenBinding);
    }
}

fn validate_optimization(trial: &Trial, spec: &BenchmarkSpec, blockers: &mut Vec<Blocker>) {
    let Some(receipt) = &trial.optimization else {
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
    let mut seen_metrics = BTreeSet::new();
    let mut seen_evidence = BTreeSet::new();
    let mut intervals = BTreeMap::new();

    for observation in &trial.observations {
        let mut valid = true;
        if !seen_metrics.insert(observation.metric_id.as_str()) {
            blockers.push(Blocker::DuplicateObservation);
            valid = false;
        }
        if observation.evidence_id.trim().is_empty()
            || !seen_evidence.insert(observation.evidence_id.as_str())
        {
            blockers.push(Blocker::DuplicateEvidenceIdentity);
            valid = false;
        }
        if observation.component_id != trial.component_id
            || observation.specimen_id != trial.acceptance.specimen_id
            || observation.build_id != trial.acceptance.build_id
            || observation.material_lot_id != trial.acceptance.material_lot_id
        {
            blockers.push(Blocker::BrokenSpecimenBinding);
            valid = false;
        }
        if observation.evidence_class != expected_evidence_class(&observation.metric_id) {
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
            .find(|metric| metric.metric_id == observation.metric_id)
        {
            if metric.source != MetricSource::Observation || metric.unit != observation.unit {
                blockers.push(Blocker::InvalidObservation);
                valid = false;
            }
        } else {
            blockers.push(Blocker::InvalidObservation);
            valid = false;
        }
        if valid {
            intervals.insert(
                observation.metric_id.clone(),
                Interval {
                    lower: observation.value - observation.uncertainty_radius,
                    upper: observation.value + observation.uncertainty_radius,
                },
            );
        }
    }

    for metric in spec
        .metrics
        .iter()
        .filter(|metric| metric.source == MetricSource::Observation)
    {
        if !intervals.contains_key(&metric.metric_id) {
            blockers.push(Blocker::MissingObservation);
        }
    }
    intervals
}

fn expected_evidence_class(metric_id: &str) -> EvidenceClass {
    if metric_id == "lifetime_cost_per_kwh" {
        EvidenceClass::AuditedCostRecord
    } else {
        EvidenceClass::PhysicalMeasurement
    }
}

fn validate_disassembly(
    trial: &Trial,
    blockers: &mut Vec<Blocker>,
) -> BTreeMap<String, Interval> {
    let evidence = &trial.disassembly;
    let identity_valid = evidence.component_id == trial.component_id
        && evidence.specimen_id == trial.acceptance.specimen_id
        && evidence.build_id == trial.acceptance.build_id
        && evidence.material_lot_id == trial.acceptance.material_lot_id;
    let receipt_valid = !evidence.evidence_id.trim().is_empty()
        && !evidence.method_id.trim().is_empty()
        && valid_sha256(&evidence.evidence_digest)
        && valid_sha256(&evidence.calibration_digest)
        && valid_sha256(&evidence.uncertainty_digest)
        && evidence.elapsed_minutes.is_finite()
        && evidence.elapsed_minutes >= 0.0
        && evidence.elapsed_uncertainty_minutes.is_finite()
        && evidence.elapsed_uncertainty_minutes >= 0.0
        && evidence.energy_wh.is_finite()
        && evidence.energy_wh >= 0.0
        && evidence.energy_uncertainty_wh.is_finite()
        && evidence.energy_uncertainty_wh >= 0.0
        && evidence.recovery_receipt_id == trial.recovery.evidence_id;

    if !identity_valid {
        blockers.push(Blocker::BrokenSpecimenBinding);
    }
    if !receipt_valid {
        blockers.push(Blocker::InvalidDisassemblyEvidence);
        return BTreeMap::new();
    }
    if evidence.evidence_id == trial.recovery.evidence_id {
        blockers.push(Blocker::DuplicateEvidenceIdentity);
        return BTreeMap::new();
    }
    if !identity_valid {
        return BTreeMap::new();
    }
    if !evidence.safe_state_confirmed
        || !evidence.worker_safety_confirmed
        || !evidence.component_integrity_pass
        || evidence.critical_damage_events > 0
    {
        blockers.push(Blocker::UnsafeDisassembly);
        return BTreeMap::new();
    }

    BTreeMap::from([
        (
            "disassembly_time_minutes".to_string(),
            Interval {
                lower: (evidence.elapsed_minutes - evidence.elapsed_uncertainty_minutes).max(0.0),
                upper: evidence.elapsed_minutes + evidence.elapsed_uncertainty_minutes,
            },
        ),
        (
            "disassembly_energy_wh".to_string(),
            Interval {
                lower: (evidence.energy_wh - evidence.energy_uncertainty_wh).max(0.0),
                upper: evidence.energy_wh + evidence.energy_uncertainty_wh,
            },
        ),
    ])
}

fn validate_recovery(
    trial: &Trial,
    spec: &BenchmarkSpec,
    blockers: &mut Vec<Blocker>,
) -> BTreeMap<String, Interval> {
    let receipt = &trial.recovery;
    let identity_valid = receipt.component_id == trial.component_id
        && receipt.specimen_id == trial.acceptance.specimen_id
        && receipt.build_id == trial.acceptance.build_id
        && receipt.material_lot_id == trial.acceptance.material_lot_id;
    if !identity_valid {
        blockers.push(Blocker::BrokenSpecimenBinding);
    }

    let masses = [
        receipt.input_magnet_mass_kg,
        receipt.reuse_grade_mass_kg,
        receipt.remanufactured_accepted_mass_kg,
        receipt.recycling_feedstock_mass_kg,
        receipt.reject_waste_mass_kg,
    ];
    let uncertainties = [
        receipt.input_uncertainty_kg,
        receipt.reuse_grade_uncertainty_kg,
        receipt.remanufactured_uncertainty_kg,
        receipt.recycling_uncertainty_kg,
        receipt.reject_uncertainty_kg,
    ];
    let receipt_valid = !receipt.evidence_id.trim().is_empty()
        && !receipt.method_id.trim().is_empty()
        && valid_sha256(&receipt.evidence_digest)
        && valid_sha256(&receipt.calibration_digest)
        && valid_sha256(&receipt.uncertainty_digest)
        && masses.iter().all(|mass| mass.is_finite() && *mass >= 0.0)
        && uncertainties
            .iter()
            .all(|uncertainty| uncertainty.is_finite() && *uncertainty >= 0.0)
        && receipt.input_magnet_mass_kg > receipt.input_uncertainty_kg;
    if !receipt_valid {
        blockers.push(Blocker::InvalidRecoveryReceipt);
        return BTreeMap::new();
    }
    if !identity_valid {
        return BTreeMap::new();
    }

    let path_receipt_valid = |mass: f64, path: &Option<RecoveryPathReceipt>| {
        if mass <= 0.0 {
            return path.is_none();
        }
        path.as_ref().is_some_and(|path| {
            !path.evidence_id.trim().is_empty()
                && !path.output_lot_id.trim().is_empty()
                && valid_sha256(&path.evidence_digest)
        })
    };
    if !path_receipt_valid(
        receipt.reuse_grade_mass_kg,
        &receipt.reuse_acceptance_receipt,
    ) || !path_receipt_valid(
        receipt.remanufactured_accepted_mass_kg,
        &receipt.remanufacture_acceptance_receipt,
    ) || !path_receipt_valid(
        receipt.recycling_feedstock_mass_kg,
        &receipt.recycling_intake_receipt,
    ) {
        blockers.push(Blocker::InvalidRecoveryReceipt);
        return BTreeMap::new();
    }

    let output_mass = receipt.reuse_grade_mass_kg
        + receipt.remanufactured_accepted_mass_kg
        + receipt.recycling_feedstock_mass_kg
        + receipt.reject_waste_mass_kg;
    let input_lower = receipt.input_magnet_mass_kg - receipt.input_uncertainty_kg;
    let input_upper = receipt.input_magnet_mass_kg + receipt.input_uncertainty_kg;
    let output_lower = (receipt.reuse_grade_mass_kg - receipt.reuse_grade_uncertainty_kg).max(0.0)
        + (receipt.remanufactured_accepted_mass_kg - receipt.remanufactured_uncertainty_kg)
            .max(0.0)
        + (receipt.recycling_feedstock_mass_kg - receipt.recycling_uncertainty_kg).max(0.0)
        + (receipt.reject_waste_mass_kg - receipt.reject_uncertainty_kg).max(0.0);
    let output_upper = output_mass
        + receipt.reuse_grade_uncertainty_kg
        + receipt.remanufactured_uncertainty_kg
        + receipt.recycling_uncertainty_kg
        + receipt.reject_uncertainty_kg;
    let difference_lower = output_lower - input_upper;
    let difference_upper = output_upper - input_lower;
    let allowed_error = spec.max_mass_balance_error_kg
        + receipt.input_magnet_mass_kg * spec.max_mass_balance_error_fraction;
    if difference_lower < -allowed_error || difference_upper > allowed_error {
        blockers.push(Blocker::UnclosedMassBalance);
        return BTreeMap::new();
    }

    let direct_reuse_fraction = Interval {
        lower: ((receipt.reuse_grade_mass_kg - receipt.reuse_grade_uncertainty_kg).max(0.0)
            / input_upper)
            .clamp(0.0, 1.0),
        upper: ((receipt.reuse_grade_mass_kg + receipt.reuse_grade_uncertainty_kg) / input_lower)
            .clamp(0.0, 1.0),
    };
    let remanufactured_fraction = Interval {
        lower: ((receipt.remanufactured_accepted_mass_kg
            - receipt.remanufactured_uncertainty_kg)
            .max(0.0)
            / input_upper)
            .clamp(0.0, 1.0),
        upper: ((receipt.remanufactured_accepted_mass_kg
            + receipt.remanufactured_uncertainty_kg)
            / input_lower)
            .clamp(0.0, 1.0),
    };
    let recycle_fraction = Interval {
        lower: ((receipt.recycling_feedstock_mass_kg - receipt.recycling_uncertainty_kg).max(0.0)
            / input_upper)
            .clamp(0.0, 1.0),
        upper: ((receipt.recycling_feedstock_mass_kg + receipt.recycling_uncertainty_kg)
            / input_lower)
            .clamp(0.0, 1.0),
    };

    BTreeMap::from([
        ("direct_reuse_fraction".to_string(), direct_reuse_fraction),
        (
            "accepted_remanufacture_fraction".to_string(),
            remanufactured_fraction,
        ),
        ("recycling_feedstock_fraction".to_string(), recycle_fraction),
    ])
}

fn trial_intervals(
    trial: &Trial,
    spec: &BenchmarkSpec,
    blockers: &mut Vec<Blocker>,
) -> BTreeMap<String, Interval> {
    let mut intervals = validate_observations(trial, spec, blockers);
    intervals.extend(validate_disassembly(trial, blockers));
    intervals.extend(validate_recovery(trial, spec, blockers));

    for metric in &spec.metrics {
        let source_id = match metric.source {
            MetricSource::Observation => metric.metric_id.as_str(),
            MetricSource::DisassemblyElapsedMinutes => "disassembly_time_minutes",
            MetricSource::DisassemblyEnergyWh => "disassembly_energy_wh",
            MetricSource::DirectReuseFraction => "direct_reuse_fraction",
            MetricSource::RemanufacturedAcceptedFraction => "accepted_remanufacture_fraction",
            MetricSource::RecyclingFeedstockFraction => "recycling_feedstock_fraction",
        };
        if !intervals.contains_key(source_id) {
            blockers.push(Blocker::MissingObservation);
        }
    }
    intervals
}

struct HardStatus {
    blockers: Vec<Blocker>,
    unresolved: bool,
}

fn hard_limit_status(
    intervals: &BTreeMap<String, Interval>,
    metrics: &[MetricSpec],
) -> HardStatus {
    let mut blockers = Vec::new();
    let mut unresolved = false;
    for metric in metrics
        .iter()
        .filter(|metric| metric.class == MetricClass::MandatoryFunction)
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
        let (Some(candidate_interval), Some(baseline_interval)) =
            (candidate.get(&metric_id(metric)), baseline.get(&metric_id(metric)))
        else {
            blockers.push(Blocker::MissingObservation);
            non_inferior = false;
            continue;
        };
        match metric.class {
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
                    blockers.push(Blocker::NonInferiorityViolation);
                    non_inferior = false;
                }
            }
            MetricClass::CircularityBenefit => {
                let improved = match metric.direction {
                    Direction::Maximize => candidate_interval.lower > baseline_interval.upper,
                    Direction::Minimize => candidate_interval.upper < baseline_interval.lower,
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

fn metric_id(metric: &MetricSpec) -> String {
    match metric.source {
        MetricSource::Observation => metric.metric_id.clone(),
        MetricSource::DisassemblyElapsedMinutes => "disassembly_time_minutes".to_string(),
        MetricSource::DisassemblyEnergyWh => "disassembly_energy_wh".to_string(),
        MetricSource::DirectReuseFraction => "direct_reuse_fraction".to_string(),
        MetricSource::RemanufacturedAcceptedFraction => {
            "accepted_remanufacture_fraction".to_string()
        }
        MetricSource::RecyclingFeedstockFraction => {
            "recycling_feedstock_fraction".to_string()
        }
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

fn is_conventional_pm_family(family: Family) -> bool {
    matches!(
        family,
        Family::ConventionalBondedAdhesivePmRotor
            | Family::ConventionalMechanicallyRetainedSegmentedPmRotor
            | Family::ConventionalServiceablePmRotor
    )
}

fn trial_evidence_ids(trial: &Trial) -> Vec<&str> {
    let mut ids: Vec<&str> = trial
        .observations
        .iter()
        .map(|observation| observation.evidence_id.as_str())
        .collect();
    ids.push(trial.disassembly.evidence_id.as_str());
    ids.push(trial.recovery.evidence_id.as_str());
    for receipt in [
        &trial.recovery.reuse_acceptance_receipt,
        &trial.recovery.remanufacture_acceptance_receipt,
        &trial.recovery.recycling_intake_receipt,
    ]
    .into_iter()
    .flatten()
    {
        ids.push(receipt.evidence_id.as_str());
    }
    ids.push(trial.acceptance.functional_test_evidence_id.as_str());
    ids
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
            application_id: "TRACTOR-DRIVE-150KW".to_string(),
            duty_digest: digest('a'),
            lifetime_profile_digest: digest('b'),
            thermal_environment_digest: digest('c'),
            mechanical_envelope_digest: digest('d'),
            safety_policy_digest: digest('e'),
            measurement_setup_digest: digest('f'),
            acceptance_policy_digest: digest('1'),
            lifecycle_boundary_digest: digest('2'),
            objective_digest: digest('3'),
            constraint_digest: digest('4'),
        }
    }

    fn spec() -> BenchmarkSpec {
        BenchmarkSpec {
            benchmark_id: "CIRCULAR-ROTOR-001".to_string(),
            scope: scope(),
            metrics: vec![
                MetricSpec {
                    metric_id: "post_service_torque_nm".to_string(),
                    unit: "Nm".to_string(),
                    class: MetricClass::MandatoryFunction,
                    source: MetricSource::Observation,
                    direction: Direction::Maximize,
                    hard_min: Some(180.0),
                    hard_max: None,
                    non_inferiority_margin: Some(2.0),
                },
                MetricSpec {
                    metric_id: "lifetime_energy_weighted_efficiency".to_string(),
                    unit: "fraction".to_string(),
                    class: MetricClass::MandatoryFunction,
                    source: MetricSource::Observation,
                    direction: Direction::Maximize,
                    hard_min: Some(0.93),
                    hard_max: None,
                    non_inferiority_margin: Some(0.003),
                },
                MetricSpec {
                    metric_id: "demagnetization_margin".to_string(),
                    unit: "ratio".to_string(),
                    class: MetricClass::MandatoryFunction,
                    source: MetricSource::Observation,
                    direction: Direction::Maximize,
                    hard_min: Some(1.3),
                    hard_max: None,
                    non_inferiority_margin: Some(0.05),
                },
                MetricSpec {
                    metric_id: "magnet_retention_margin".to_string(),
                    unit: "ratio".to_string(),
                    class: MetricClass::MandatoryFunction,
                    source: MetricSource::Observation,
                    direction: Direction::Maximize,
                    hard_min: Some(2.0),
                    hard_max: None,
                    non_inferiority_margin: Some(0.1),
                },
                MetricSpec {
                    metric_id: "service_cycle_survival".to_string(),
                    unit: "cycles".to_string(),
                    class: MetricClass::MandatoryFunction,
                    source: MetricSource::Observation,
                    direction: Direction::Maximize,
                    hard_min: Some(5.0),
                    hard_max: None,
                    non_inferiority_margin: Some(1.0),
                },
                MetricSpec {
                    metric_id: "disassembly_time_minutes".to_string(),
                    unit: "min".to_string(),
                    class: MetricClass::CircularityBenefit,
                    source: MetricSource::DisassemblyElapsedMinutes,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "disassembly_energy_wh".to_string(),
                    unit: "Wh".to_string(),
                    class: MetricClass::CircularityBenefit,
                    source: MetricSource::DisassemblyEnergyWh,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "direct_reuse_fraction".to_string(),
                    unit: "fraction".to_string(),
                    class: MetricClass::CircularityBenefit,
                    source: MetricSource::DirectReuseFraction,
                    direction: Direction::Maximize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "accepted_remanufacture_fraction".to_string(),
                    unit: "fraction".to_string(),
                    class: MetricClass::CircularityBenefit,
                    source: MetricSource::RemanufacturedAcceptedFraction,
                    direction: Direction::Maximize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "recycling_feedstock_fraction".to_string(),
                    unit: "fraction".to_string(),
                    class: MetricClass::CircularityBenefit,
                    source: MetricSource::RecyclingFeedstockFraction,
                    direction: Direction::Maximize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
                MetricSpec {
                    metric_id: "lifetime_cost_per_kwh".to_string(),
                    unit: "USD/kWh".to_string(),
                    class: MetricClass::CircularityBenefit,
                    source: MetricSource::Observation,
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: None,
                    non_inferiority_margin: None,
                },
            ],
            required_baseline_families: vec![Family::ConventionalBondedAdhesivePmRotor],
            minimum_baseline_evaluations: 100,
            minimum_independent_restarts: 3,
            max_mass_balance_error_kg: 0.02,
            max_mass_balance_error_fraction: 0.015,
        }
    }

    fn optimization() -> OptimizationReceipt {
        OptimizationReceipt {
            receipt_digest: digest('7'),
            search_space_digest: digest('8'),
            objective_digest: digest('3'),
            constraint_digest: digest('4'),
            evaluations: 500,
            independent_restarts: 5,
            frozen_before_comparison: true,
        }
    }

    fn trial(
        id: &str,
        family: Family,
        role: Role,
        values: [f64; 6],
        recovery_masses: [f64; 5],
    ) -> Trial {
        let component_id = format!("COMP-{id}");
        let specimen_id = format!("SPEC-{id}");
        let build_id = format!("BUILD-{id}");
        let lot_id = format!("LOT-{id}");
        let metrics = [
            ("post_service_torque_nm", "Nm", values[0], 0.2, 'a', EvidenceClass::PhysicalMeasurement),
            ("lifetime_energy_weighted_efficiency", "fraction", values[1], 0.001, 'b', EvidenceClass::PhysicalMeasurement),
            ("demagnetization_margin", "ratio", values[2], 0.02, 'c', EvidenceClass::PhysicalMeasurement),
            ("magnet_retention_margin", "ratio", values[3], 0.02, 'd', EvidenceClass::PhysicalMeasurement),
            ("service_cycle_survival", "cycles", values[4], 0.5, 'e', EvidenceClass::PhysicalMeasurement),
            ("lifetime_cost_per_kwh", "USD/kWh", values[5], 0.00005, 'f', EvidenceClass::AuditedCostRecord),
        ];
        let observations: Vec<Observation> = metrics
            .into_iter()
            .map(|(metric_id, unit, value, uncertainty_radius, suffix, evidence_class)| {
                Observation {
                    metric_id: metric_id.to_string(),
                    component_id: component_id.clone(),
                    specimen_id: specimen_id.clone(),
                    build_id: build_id.clone(),
                    material_lot_id: lot_id.clone(),
                    unit: unit.to_string(),
                    value,
                    uncertainty_radius,
                    evidence_id: format!("OBS-{id}-{metric_id}"),
                    evidence_digest: digest(suffix),
                    evidence_class,
                    method_id: format!("METHOD-{metric_id}"),
                    calibration_digest: digest('9'),
                    uncertainty_digest: digest('0'),
                }
            })
            .collect();

        let disassembly = DisassemblyEvidence {
            component_id: component_id.clone(),
            specimen_id: specimen_id.clone(),
            build_id: build_id.clone(),
            material_lot_id: lot_id.clone(),
            evidence_id: format!("TEARDOWN-{id}"),
            evidence_digest: digest('1'),
            method_id: "METHOD-TEARDOWN".to_string(),
            calibration_digest: digest('2'),
            uncertainty_digest: digest('3'),
            elapsed_minutes: if role == Role::Candidate { 18.0 } else { 60.0 },
            elapsed_uncertainty_minutes: 1.0,
            energy_wh: if role == Role::Candidate { 25.0 } else { 70.0 },
            energy_uncertainty_wh: 2.0,
            safe_state_confirmed: true,
            worker_safety_confirmed: true,
            component_integrity_pass: true,
            critical_damage_events: 0,
            recovery_receipt_id: format!("RECOVERY-{id}"),
        };

        let recovery = RecoveryReceipt {
            component_id: component_id.clone(),
            specimen_id: specimen_id.clone(),
            build_id: build_id.clone(),
            material_lot_id: lot_id.clone(),
            evidence_id: format!("RECOVERY-{id}"),
            evidence_digest: digest('4'),
            method_id: "METHOD-MASS-BALANCE".to_string(),
            calibration_digest: digest('5'),
            uncertainty_digest: digest('6'),
            input_magnet_mass_kg: recovery_masses[0],
            input_uncertainty_kg: 0.01,
            reuse_grade_mass_kg: recovery_masses[1],
            reuse_grade_uncertainty_kg: 0.01,
            remanufactured_accepted_mass_kg: recovery_masses[2],
            remanufactured_uncertainty_kg: 0.01,
            recycling_feedstock_mass_kg: recovery_masses[3],
            recycling_uncertainty_kg: 0.01,
            reject_waste_mass_kg: recovery_masses[4],
            reject_uncertainty_kg: 0.01,
            reuse_acceptance_receipt: (recovery_masses[1] > 0.0).then(|| RecoveryPathReceipt {
                evidence_id: format!("REUSE-ACCEPT-{id}"),
                evidence_digest: digest('a'),
                output_lot_id: format!("REUSE-LOT-{id}"),
            }),
            remanufacture_acceptance_receipt: (recovery_masses[2] > 0.0).then(|| RecoveryPathReceipt {
                evidence_id: format!("REMAN-ACCEPT-{id}"),
                evidence_digest: digest('b'),
                output_lot_id: format!("REMAN-LOT-{id}"),
            }),
            recycling_intake_receipt: (recovery_masses[3] > 0.0).then(|| RecoveryPathReceipt {
                evidence_id: format!("RECYCLER-INTAKE-{id}"),
                evidence_digest: digest('c'),
                output_lot_id: format!("FEEDSTOCK-LOT-{id}"),
            }),
        };

        let functional_test_evidence_id = format!("FUNCTIONAL-{id}");
        let linked_evidence_ids = observations
            .iter()
            .map(|observation| observation.evidence_id.clone())
            .chain([
                disassembly.evidence_id.clone(),
                recovery.evidence_id.clone(),
            ])
            .chain(
                recovery
                    .reuse_acceptance_receipt
                    .iter()
                    .map(|receipt| receipt.evidence_id.clone()),
            )
            .chain(
                recovery
                    .remanufacture_acceptance_receipt
                    .iter()
                    .map(|receipt| receipt.evidence_id.clone()),
            )
            .chain(
                recovery
                    .recycling_intake_receipt
                    .iter()
                    .map(|receipt| receipt.evidence_id.clone()),
            )
            .chain([functional_test_evidence_id.clone()])
            .collect();

        let acceptance = ComponentAcceptance {
            component_id: component_id.clone(),
            specimen_id: specimen_id.clone(),
            build_id: build_id.clone(),
            material_lot_id: lot_id,
            maturity: Maturity::M4,
            disposition: Disposition::Qualified,
            scope_digest: digest('1'),
            receipt_digest: digest('5'),
            functional_test_evidence_id,
            functional_test_digest: digest('6'),
            linked_evidence_ids,
        };

        Trial {
            architecture_id: id.to_string(),
            component_id,
            family,
            role,
            scope: scope(),
            acceptance,
            optimization: Some(optimization()),
            observations,
            disassembly,
            recovery,
        }
    }

    fn known_trials() -> Vec<Trial> {
        vec![
            trial(
                "AM-MODULAR",
                Family::AmModularRecoverableRotor,
                Role::Candidate,
                [200.0, 0.95, 1.8, 2.8, 12.0, 0.002],
                [10.0, 5.0, 2.0, 2.0, 1.0],
            ),
            trial(
                "CONVENTIONAL-BONDED",
                Family::ConventionalBondedAdhesivePmRotor,
                Role::RequiredBaseline,
                [198.0, 0.945, 1.7, 2.7, 10.0, 0.003],
                [10.0, 1.0, 1.0, 4.0, 4.0],
            ),
        ]
    }

    #[test]
    fn modular_rotor_is_advantage_when_function_and_recovery_are_proven() {
        let result = evaluate(&spec(), &known_trials());
        assert_eq!(result[0].decision, Decision::CircularityAdvantage);
        assert!(result[0]
            .robust_circularity_benefits
            .contains(&"direct_reuse_fraction".to_string()));
        assert!(result[0]
            .robust_circularity_benefits
            .contains(&"accepted_remanufacture_fraction".to_string()));
    }

    #[test]
    fn positive_direct_reuse_mass_requires_its_own_acceptance_receipt() {
        let mut trials = known_trials();
        trials[0].recovery.reuse_acceptance_receipt = None;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::InvalidRecoveryReceipt));
        assert!(!result[0]
            .robust_circularity_benefits
            .contains(&"direct_reuse_fraction".to_string()));
    }

    #[test]
    fn positive_recycling_feedstock_mass_requires_intake_receipt() {
        let mut trials = known_trials();
        trials[0].recovery.recycling_intake_receipt = None;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::InvalidRecoveryReceipt));
        assert!(!result[0]
            .robust_circularity_benefits
            .contains(&"recycling_feedstock_fraction".to_string()));
    }

    #[test]
    fn accepted_remanufacture_is_separate_from_direct_reuse() {
        let mut trials = known_trials();
        trials[0].recovery.reuse_grade_mass_kg = 1.0;
        trials[0].recovery.remanufactured_accepted_mass_kg = 5.0;
        trials[0].recovery.recycling_feedstock_mass_kg = 2.0;
        trials[0].recovery.reject_waste_mass_kg = 2.0;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::CircularityAdvantage);
        assert!(!result[0]
            .robust_circularity_benefits
            .contains(&"direct_reuse_fraction".to_string()));
        assert!(result[0]
            .robust_circularity_benefits
            .contains(&"accepted_remanufacture_fraction".to_string()));
    }

    #[test]
    fn synthetic_measurements_cannot_establish_circularity_advantage() {
        let mut trials = known_trials();
        trials[0].observations[0].evidence_class = EvidenceClass::Synthetic;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::InadmissibleEvidenceClass));
    }

    #[test]
    fn demagnetization_failure_is_non_compensatory() {
        let mut trials = known_trials();
        trials[0].observations[2].value = 1.0;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn uncertainty_touching_hard_limit_is_indeterminate() {
        let mut trials = known_trials();
        trials[0].observations[2].value = 1.31;
        trials[0].observations[2].uncertainty_radius = 0.03;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
    }

    #[test]
    fn missing_post_service_metric_is_not_defaulted() {
        let mut trials = known_trials();
        trials[0]
            .observations
            .retain(|observation| observation.metric_id != "post_service_torque_nm");
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::MissingObservation));
    }

    #[test]
    fn unclosed_mass_balance_is_indeterminate() {
        let mut trials = known_trials();
        trials[0].recovery.reject_waste_mass_kg = 0.1;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::UnclosedMassBalance));
    }

    #[test]
    fn unsafe_disassembly_is_a_definite_failure() {
        let mut trials = known_trials();
        trials[0].disassembly.worker_safety_confirmed = false;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
        assert!(result[0].blockers.contains(&Blocker::UnsafeDisassembly));
    }

    #[test]
    fn component_damage_during_teardown_is_a_failure() {
        let mut trials = known_trials();
        trials[0].disassembly.component_integrity_pass = false;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Fail);
    }

    #[test]
    fn scope_mismatch_is_not_normalized_away() {
        let mut trials = known_trials();
        trials[0].scope.lifetime_profile_digest = digest('0');
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
    }

    #[test]
    fn missing_conventional_pm_baseline_is_not_comparable() {
        let mut trials = known_trials();
        trials.remove(1);
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::MissingRequiredBaseline));
    }

    #[test]
    fn magnet_free_control_cannot_replace_pm_baseline() {
        let mut trials = known_trials();
        trials[1].family = Family::MagnetFreeReferenceControl;
        trials[1].role = Role::SecondaryControl;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
    }

    #[test]
    fn conservative_noninferiority_failure_is_reported_as_inferior() {
        let mut trials = known_trials();
        trials[0].observations[1].value = 0.91;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::InferiorToBaseline);
    }

    #[test]
    fn overlapping_benefit_intervals_do_not_prove_advantage() {
        let mut trials = known_trials();
        trials[0].disassembly.elapsed_minutes = 60.0;
        trials[0].disassembly.elapsed_uncertainty_minutes = 5.0;
        trials[0].disassembly.energy_wh = 70.0;
        trials[0].disassembly.energy_uncertainty_wh = 5.0;
        trials[0].recovery.reuse_grade_mass_kg = 1.0;
        trials[0].recovery.remanufactured_accepted_mass_kg = 1.0;
        trials[0].recovery.recycling_feedstock_mass_kg = 4.0;
        trials[0].recovery.reject_waste_mass_kg = 4.0;
        trials[0].observations[5].value = 0.003;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NonInferiorNoCircularityBenefit);
    }

    #[test]
    fn every_required_pm_family_must_be_beaten() {
        let mut benchmark = spec();
        benchmark.required_baseline_families.push(
            Family::ConventionalMechanicallyRetainedSegmentedPmRotor,
        );
        let mut trials = known_trials();
        let mut segmented = trial(
            "CONVENTIONAL-SEGMENTED",
            Family::ConventionalMechanicallyRetainedSegmentedPmRotor,
            Role::RequiredBaseline,
            [199.0, 0.949, 1.75, 2.75, 11.0, 0.001],
            [10.0, 8.0, 0.0, 1.0, 1.0],
        );
        segmented.disassembly.elapsed_minutes = 10.0;
        segmented.disassembly.energy_wh = 10.0;
        trials.push(segmented);
        let result = evaluate(&benchmark, &trials);
        assert_eq!(result[0].decision, Decision::NonInferiorNoCircularityBenefit);
    }

    #[test]
    fn wrong_lot_cannot_be_used_to_claim_recovery() {
        let mut trials = known_trials();
        trials[0].recovery.material_lot_id = "LOT-OTHER".to_string();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::BrokenSpecimenBinding));
    }

    #[test]
    fn malformed_observation_cannot_create_a_false_hard_fail() {
        let mut trials = known_trials();
        trials[0].observations[0].value = 10.0;
        trials[0].observations[0].specimen_id = "SPEC-OTHER".to_string();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(!result[0].blockers.contains(&Blocker::DefiniteHardLimitViolation));
    }

    #[test]
    fn candidate_and_baseline_need_frozen_optimization_receipts() {
        let mut trials = known_trials();
        trials[1].optimization.as_mut().unwrap().frozen_before_comparison = false;
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::InvalidOptimizationReceipt));
    }

    #[test]
    fn duplicate_evidence_identity_is_rejected_globally() {
        let mut trials = known_trials();
        trials[1].recovery.evidence_id = trials[0].recovery.evidence_id.clone();
        trials[1].disassembly.recovery_receipt_id = trials[1].recovery.evidence_id.clone();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::NotComparable);
        assert!(result[0].blockers.contains(&Blocker::DuplicateEvidenceIdentity));
    }

    #[test]
    fn acceptance_must_link_every_measurement_and_recovery_receipt() {
        let mut trials = known_trials();
        trials[0].acceptance.linked_evidence_ids.pop();
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::BrokenSpecimenBinding));
    }

    #[test]
    fn recovery_fraction_is_derived_not_accepted_as_observation() {
        let mut trials = known_trials();
        trials[0].observations.push(Observation {
            metric_id: "direct_reuse_fraction".to_string(),
            component_id: "COMP-AM-MODULAR".to_string(),
            specimen_id: "SPEC-AM-MODULAR".to_string(),
            build_id: "BUILD-AM-MODULAR".to_string(),
            material_lot_id: "LOT-AM-MODULAR".to_string(),
            unit: "fraction".to_string(),
            value: 1.0,
            uncertainty_radius: 0.0,
            evidence_id: "OBS-FAKE-REUSE-FRACTION".to_string(),
            evidence_digest: digest('a'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            method_id: "METHOD-FAKE".to_string(),
            calibration_digest: digest('b'),
            uncertainty_digest: digest('c'),
        });
        let result = evaluate(&spec(), &trials);
        assert_eq!(result[0].decision, Decision::Indeterminate);
        assert!(result[0].blockers.contains(&Blocker::InvalidObservation));
    }
}
