//! Element-specific virgin-material intensity per accepted useful service.
//!
//! This evaluator consumes measured material-lot composition and accepted
//! component service evidence. It does not calculate material chemistry, motor
//! physics, supply-chain criticality, life-cycle impacts, or service-life models.
//!
//! Recovered and reused inputs are reported separately. No end-of-life credit
//! is subtracted from virgin input in v0.1, avoiding double-counted recycling
//! credits and keeping allocation conventions outside this decision adapter.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementOrigin {
    Virgin,
    ExternalSecondary,
    InternalRecycled,
    DirectReuse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchitectureFamily {
    ConventionalSinteredPm,
    ConventionalBondedPm,
    PmAssistedSynrm,
    WoundFieldSynchronous,
    Induction,
    SynchronousReluctance,
    AmShapedPmInsert,
    AmHybridArchitecture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchitectureRole {
    Candidate,
    RequiredBaseline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
pub enum AcceptanceDisposition {
    Qualified,
    Fail,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceBasis {
    ObservedService,
    DurabilityTestedService,
    ValidatedProjection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    NotComparable,
    Fail,
    Indeterminate,
    InferiorToBaseline,
    NonInferiorNoMaterialAdvantage,
    CriticalMaterialAdvantage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub const fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    pub fn is_valid(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite() && self.lower <= self.upper
    }

    pub fn is_nonnegative(self) -> bool {
        self.is_valid() && self.lower >= 0.0
    }

    pub fn is_positive(self) -> bool {
        self.is_valid() && self.lower > 0.0
    }

    pub fn add(self, rhs: Self) -> Self {
        Self::new(self.lower + rhs.lower, self.upper + rhs.upper)
    }

    pub fn multiply_nonnegative(self, rhs: Self) -> Option<Self> {
        if !self.is_nonnegative() || !rhs.is_nonnegative() {
            return None;
        }
        let result = Self::new(self.lower * rhs.lower, self.upper * rhs.upper);
        result.is_nonnegative().then_some(result)
    }

    /// Conservative interval ratio for a strictly positive denominator.
    /// No statistical-independence or probability assumption is made.
    pub fn divide_positive(self, denominator: Self) -> Option<Self> {
        if !self.is_nonnegative() || !denominator.is_valid() || denominator.lower <= 0.0 {
            return None;
        }
        let result = Self::new(
            self.lower / denominator.upper,
            self.upper / denominator.lower,
        );
        result.is_nonnegative().then_some(result)
    }

    pub fn scale_nonnegative(self, factor: f64) -> Option<Self> {
        if !self.is_nonnegative() || !factor.is_finite() || factor < 0.0 {
            return None;
        }
        let result = Self::new(self.lower * factor, self.upper * factor);
        result.is_nonnegative().then_some(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemScope {
    pub application_id: String,
    pub target_function_digest: String,
    pub duty_profile_digest: String,
    pub system_boundary_digest: String,
    pub inverter_control_digest: String,
    pub cooling_boundary_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
    pub service_horizon_digest: String,
    pub element_inventory_scope_digest: String,
    pub objective_set_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementSpec {
    pub element_id: String,
    pub unit: String,
    /// Maximum allowed increase in candidate virgin g/MWh versus baseline.
    pub no_worse_margin_g_per_mwh: f64,
    /// Minimum robust reduction required to count as an element improvement.
    pub improvement_margin_g_per_mwh: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkSpec {
    pub benchmark_id: String,
    pub scope: SystemScope,
    pub required_elements: Vec<ElementSpec>,
    pub required_baseline_families: Vec<ArchitectureFamily>,
    pub minimum_optimization_evaluations: u32,
    pub minimum_independent_restarts: u32,
    pub optimization_budget_tier: String,
    pub minimum_completed_missions: u32,
    pub service_noninferiority_margin_kwh: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentAcceptance {
    pub component_id: String,
    pub specimen_id: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub scope_digest: String,
    pub maturity: Maturity,
    pub disposition: AcceptanceDisposition,
    pub evidence_class: EvidenceClass,
    pub physical_evidence_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServiceEvidence {
    pub receipt_id: String,
    pub receipt_digest: String,
    pub component_id: String,
    pub duty_profile_digest: String,
    pub system_boundary_digest: String,
    pub service_horizon_digest: String,
    pub evidence_class: EvidenceClass,
    pub service_basis: ServiceBasis,
    /// Useful mechanical shaft energy delivered per completed mission.
    pub useful_shaft_energy_per_mission_kwh: Interval,
    /// Bounded count of completed missions in the declared service horizon.
    pub completed_mission_count: u32,
    pub durability_receipt_id: String,
    pub durability_receipt_digest: String,
    pub durability_evidence_class: EvidenceClass,
    pub durability_method_id: String,
    pub durability_calibration_digest: String,
    pub durability_uncertainty_record_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub instrument_id: String,
}

#[derive(Clone, Debug, PartialEq)]
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
    pub budget_tier: String,
    pub frozen_before_unseal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementFractionAssay {
    pub element_id: String,
    /// Dimensionless mass fraction in [0, 1]. A non-detect must be represented
    /// by a measured interval bounded by a declared detection limit, not omission.
    pub mass_fraction: Interval,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub laboratory_id: String,
    pub laboratory_method_scope_digest: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialLotEvidence {
    pub material_lot_id: String,
    pub component_id: String,
    pub material_family_id: String,
    pub origin: ElementOrigin,
    pub scope_digest: String,
    pub inventory_scope_digest: String,
    pub material_mass_g: Interval,
    pub mass_evidence_id: String,
    pub mass_evidence_digest: String,
    pub mass_method_id: String,
    pub mass_calibration_digest: String,
    pub mass_uncertainty_record_digest: String,
    pub mass_laboratory_id: String,
    pub mass_laboratory_method_scope_digest: String,
    pub mass_evidence_class: EvidenceClass,
    pub element_assays: Vec<ElementFractionAssay>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureTrial {
    pub architecture_id: String,
    pub family: ArchitectureFamily,
    pub role: ArchitectureRole,
    pub scope: SystemScope,
    pub acceptance: ComponentAcceptance,
    pub service: ServiceEvidence,
    pub optimization: OptimizationReceipt,
    pub material_lots: Vec<MaterialLotEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    InvalidSpec,
    InvalidDigest,
    MissingIdentity,
    NoCandidateArchitecture,
    NoRequiredBaselineFamily,
    MissingRequiredBaseline,
    DuplicateArchitectureId,
    WrongArchitectureRole,
    ScopeMismatch,
    MissingM4Acceptance,
    ComponentAcceptanceFailed,
    NonPhysicalAcceptanceEvidence,
    MissingOptimizationReceipt,
    InvalidOptimizationReceipt,
    InsufficientOptimizationBudget,
    OptimizationBudgetTierMismatch,
    MissingServiceEvidence,
    ServiceScopeMismatch,
    InvalidServiceInterval,
    InsufficientCompletedMissions,
    NonPhysicalServiceEvidence,
    ProjectionCannotSupportLifetimeClaim,
    MissingDurabilityReceipt,
    InvalidMaterialInventory,
    DuplicateMaterialLot,
    DuplicateMaterialReceipt,
    MissingElementAssay,
    DuplicateElementAssay,
    ExtraElementAssay,
    InvalidElementFraction,
    NonPhysicalElementEvidence,
    MissingElementMethod,
    MissingLaboratoryIdentity,
    InvalidLaboratoryMethodScope,
    InvalidElementCalibration,
    InvalidElementUncertainty,
    MissingMassMethod,
    MissingMassCalibration,
    MissingMassLaboratoryIdentity,
    InvalidMassLaboratoryMethodScope,
    MissingMassUncertainty,
    NonPhysicalMassEvidence,
    ElementArithmeticOverflow,
    ElementFractionMassClosureFailure,
    ElementFractionMassClosureUnresolved,
    NonPositiveServiceDenominator,
    ServiceNoninferiorityViolation,
    ElementIntensityWorse,
    ElementIntensityUnresolved,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementMassTotals {
    pub virgin_g: Interval,
    pub external_secondary_g: Interval,
    pub internal_recycled_g: Interval,
    pub direct_reuse_g: Interval,
    pub total_embodied_g: Interval,
}

impl Default for ElementMassTotals {
    fn default() -> Self {
        Self {
            virgin_g: Interval::new(0.0, 0.0),
            external_secondary_g: Interval::new(0.0, 0.0),
            internal_recycled_g: Interval::new(0.0, 0.0),
            direct_reuse_g: Interval::new(0.0, 0.0),
            total_embodied_g: Interval::new(0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementIntensitySummary {
    pub element_id: String,
    pub virgin_g: Interval,
    pub external_secondary_g: Interval,
    pub internal_recycled_g: Interval,
    pub direct_reuse_g: Interval,
    pub total_embodied_g: Interval,
    /// Virgin grams per MWh of observed/durability-tested useful shaft service.
    pub virgin_g_per_mwh: Interval,
    pub external_secondary_g_per_mwh: Option<Interval>,
    pub internal_recycled_g_per_mwh: Option<Interval>,
    pub direct_reuse_g_per_mwh: Option<Interval>,
    pub total_embodied_g_per_mwh: Option<Interval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementComparison {
    pub element_id: String,
    pub candidate_virgin_g_per_mwh: Interval,
    pub baseline_virgin_g_per_mwh: Interval,
    pub robustly_no_worse: bool,
    pub robustly_improved: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BaselineComparison {
    pub baseline_id: String,
    pub baseline_family: ArchitectureFamily,
    pub candidate_service_kwh: Interval,
    pub baseline_service_kwh: Interval,
    pub service_noninferior: bool,
    pub element_comparisons: Vec<ElementComparison>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureEvaluation {
    pub architecture_id: String,
    pub decision: Decision,
    pub blockers: Vec<Blocker>,
    pub accepted_service_kwh: Option<Interval>,
    pub element_intensities: Vec<ElementIntensitySummary>,
    pub baseline_comparisons: Vec<BaselineComparison>,
    pub robustly_improved_elements: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq)]
struct TrialSummary {
    service_kwh: Option<Interval>,
    element_intensities: BTreeMap<String, ElementIntensitySummary>,
    blockers: Vec<Blocker>,
}

pub fn evaluate_critical_material_intensity(
    spec: &BenchmarkSpec,
    trials: &[ArchitectureTrial],
) -> Vec<ArchitectureEvaluation> {
    let global_blockers = validate_spec(spec, trials);
    trials
        .iter()
        .filter(|trial| trial.role == ArchitectureRole::Candidate)
        .map(|candidate| evaluate_candidate(spec, trials, candidate, &global_blockers))
        .collect()
}

fn validate_spec(spec: &BenchmarkSpec, trials: &[ArchitectureTrial]) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if spec.benchmark_id.trim().is_empty()
        || spec.scope.application_id.trim().is_empty()
        || spec.required_elements.is_empty()
        || spec.required_baseline_families.is_empty()
        || spec.minimum_optimization_evaluations == 0
        || spec.minimum_independent_restarts == 0
        || spec.optimization_budget_tier.trim().is_empty()
        || !spec.minimum_completed_missions.is_finite()
        || spec.minimum_completed_missions <= 0.0
        || !spec.service_noninferiority_margin_kwh.is_finite()
        || spec.service_noninferiority_margin_kwh < 0.0
    {
        blockers.push(Blocker::InvalidSpec);
    }

    for digest in [
        &spec.scope.target_function_digest,
        &spec.scope.duty_profile_digest,
        &spec.scope.system_boundary_digest,
        &spec.scope.inverter_control_digest,
        &spec.scope.cooling_boundary_digest,
        &spec.scope.measurement_setup_digest,
        &spec.scope.acceptance_policy_digest,
        &spec.scope.service_horizon_digest,
        &spec.scope.objective_set_digest,
    ] {
        if !valid_digest(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }

    let mut element_ids = BTreeSet::new();
    for element in &spec.required_elements {
        if element.element_id.trim().is_empty()
            || element.unit != "g"
            || !element_ids.insert(element.element_id.as_str())
            || !element.no_worse_margin_g_per_mwh.is_finite()
            || element.no_worse_margin_g_per_mwh < 0.0
            || !element.improvement_margin_g_per_mwh.is_finite()
            || element.improvement_margin_g_per_mwh < 0.0
        {
            blockers.push(Blocker::InvalidSpec);
        }
    }

    if !spec.required_baseline_families.iter().any(|family| {
        matches!(
            family,
            ArchitectureFamily::ConventionalSinteredPm
                | ArchitectureFamily::ConventionalBondedPm
                | ArchitectureFamily::PmAssistedSynrm
        )
    }) {
        blockers.push(Blocker::NoRequiredBaselineFamily);
    }

    let mut architecture_ids = BTreeSet::new();
    for trial in trials {
        if trial.architecture_id.trim().is_empty()
            || !architecture_ids.insert(trial.architecture_id.as_str())
        {
            blockers.push(Blocker::DuplicateArchitectureId);
        }
    }
    if !trials
        .iter()
        .any(|trial| trial.role == ArchitectureRole::Candidate)
    {
        blockers.push(Blocker::NoCandidateArchitecture);
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
    blockers
}

fn evaluate_candidate(
    spec: &BenchmarkSpec,
    trials: &[ArchitectureTrial],
    candidate: &ArchitectureTrial,
    global_blockers: &[Blocker],
) -> ArchitectureEvaluation {
    let mut blockers = global_blockers.to_vec();
    let mut not_comparable = blockers.iter().any(|item| {
        matches!(
            item,
            Blocker::InvalidSpec
                | Blocker::NoRequiredBaselineFamily
                | Blocker::NoCandidateArchitecture
                | Blocker::MissingRequiredBaseline
                | Blocker::DuplicateArchitectureId
        )
    });

    if !matches!(
        candidate.family,
        ArchitectureFamily::AmShapedPmInsert | ArchitectureFamily::AmHybridArchitecture
    ) {
        blockers.push(Blocker::WrongArchitectureRole);
        not_comparable = true;
    }
    if candidate.scope != spec.scope {
        blockers.push(Blocker::ScopeMismatch);
        not_comparable = true;
    }

    let candidate_summary = summarize_trial(spec, candidate);
    if candidate_summary.blockers.contains(&Blocker::ScopeMismatch)
        || candidate_summary.blockers.contains(&Blocker::ServiceScopeMismatch)
    {
        not_comparable = true;
    }
    blockers.extend(candidate_summary.blockers.iter().cloned());

    let baselines = trials
        .iter()
        .filter(|trial| {
            trial.role == ArchitectureRole::RequiredBaseline
                && spec.required_baseline_families.contains(&trial.family)
        })
        .collect::<Vec<_>>();

    let mut comparisons = Vec::new();
    let mut all_baselines_noninferior = !baselines.is_empty();
    let mut all_evidence_admissible = candidate_summary.blockers.is_empty();
    let mut all_elements_non_worse = true;
    let mut robust_improved_intersection: Option<BTreeSet<String>> = None;
    let mut definitely_inferior = false;

    for baseline in baselines {
        let baseline_summary = summarize_trial(spec, baseline);
        if baseline.scope != spec.scope
            || baseline_summary.blockers.contains(&Blocker::ScopeMismatch)
            || baseline_summary.blockers.contains(&Blocker::ServiceScopeMismatch)
        {
            blockers.push(Blocker::ScopeMismatch);
            not_comparable = true;
        }
        if !baseline_summary.blockers.is_empty() {
            all_evidence_admissible = false;
            blockers.extend(baseline_summary.blockers.iter().cloned());
        }

        let (Some(candidate_service), Some(baseline_service)) = (
            candidate_summary.service_kwh,
            baseline_summary.service_kwh,
        ) else {
            all_baselines_noninferior = false;
            all_evidence_admissible = false;
            continue;
        };

        let service_noninferior = candidate_service.lower
            >= baseline_service.upper - spec.service_noninferiority_margin_kwh;
        if !service_noninferior {
            all_baselines_noninferior = false;
            if candidate_service.upper
                < baseline_service.lower - spec.service_noninferiority_margin_kwh
            {
                definitely_inferior = true;
                blockers.push(Blocker::ServiceNoninferiorityViolation);
            } else {
                all_evidence_admissible = false;
                blockers.push(Blocker::ServiceNoninferiorityViolation);
            }
        }

        let mut element_comparisons = Vec::new();
        let mut baseline_improvements = BTreeSet::new();
        for element in &spec.required_elements {
            let (Some(candidate_intensity), Some(baseline_intensity)) = (
                candidate_summary
                    .element_intensities
                    .get(&element.element_id),
                baseline_summary
                    .element_intensities
                    .get(&element.element_id),
            ) else {
                all_evidence_admissible = false;
                all_elements_non_worse = false;
                blockers.push(Blocker::MissingElementAssay);
                continue;
            };

            let robustly_no_worse = candidate_intensity.virgin_g_per_mwh.upper
                <= baseline_intensity.virgin_g_per_mwh.lower
                    + element.no_worse_margin_g_per_mwh;
            let robustly_improved = candidate_intensity.virgin_g_per_mwh.upper
                < baseline_intensity.virgin_g_per_mwh.lower
                    - element.improvement_margin_g_per_mwh;
            if !robustly_no_worse {
                all_elements_non_worse = false;
                if candidate_intensity.virgin_g_per_mwh.lower
                    > baseline_intensity.virgin_g_per_mwh.upper
                        + element.no_worse_margin_g_per_mwh
                {
                    definitely_inferior = true;
                    blockers.push(Blocker::ElementIntensityWorse);
                } else {
                    all_evidence_admissible = false;
                    blockers.push(Blocker::ElementIntensityUnresolved);
                }
            }
            if robustly_improved {
                baseline_improvements.insert(element.element_id.clone());
            }
            element_comparisons.push(ElementComparison {
                element_id: element.element_id.clone(),
                candidate_virgin_g_per_mwh: candidate_intensity.virgin_g_per_mwh,
                baseline_virgin_g_per_mwh: baseline_intensity.virgin_g_per_mwh,
                robustly_no_worse,
                robustly_improved,
            });
        }

        robust_improved_intersection = Some(match robust_improved_intersection.take() {
            Some(previous) => previous
                .intersection(&baseline_improvements)
                .cloned()
                .collect(),
            None => baseline_improvements,
        });

        comparisons.push(BaselineComparison {
            baseline_id: baseline.architecture_id.clone(),
            baseline_family: baseline.family,
            candidate_service_kwh,
            baseline_service_kwh,
            service_noninferior,
            element_comparisons,
        });
    }

    let improved_elements = robust_improved_intersection
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<_>>();

    let decision = if not_comparable {
        Decision::NotComparable
    } else if candidate_summary.blockers.contains(&Blocker::ComponentAcceptanceFailed)
        || candidate_summary
            .blockers
            .contains(&Blocker::ElementFractionMassClosureFailure)
    {
        Decision::Fail
    } else if definitely_inferior {
        Decision::InferiorToBaseline
    } else if !all_evidence_admissible
        || !all_baselines_noninferior
        || !all_elements_non_worse
    {
        Decision::Indeterminate
    } else if !improved_elements.is_empty() {
        Decision::CriticalMaterialAdvantage
    } else {
        Decision::NonInferiorNoMaterialAdvantage
    };

    blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    blockers.dedup();

    ArchitectureEvaluation {
        architecture_id: candidate.architecture_id.clone(),
        decision,
        blockers,
        accepted_service_kwh: candidate_summary.service_kwh,
        element_intensities: candidate_summary
            .element_intensities
            .values()
            .cloned()
            .collect(),
        baseline_comparisons: comparisons,
        robustly_improved_elements: improved_elements,
        claim_ceiling: format!(
            "{decision:?}: element-specific virgin material per accepted observed service; no supply-risk score, LCA, product certification or lifetime extrapolation"
        ),
    }
}

fn summarize_trial(spec: &BenchmarkSpec, trial: &ArchitectureTrial) -> TrialSummary {
    let mut blockers = Vec::new();

    if trial.architecture_id.trim().is_empty()
        || trial.acceptance.component_id.trim().is_empty()
        || trial.acceptance.specimen_id.trim().is_empty()
        || trial.acceptance.receipt_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    if trial.scope != spec.scope
        || trial.acceptance.scope_digest != spec.scope.acceptance_policy_digest
    {
        blockers.push(Blocker::ScopeMismatch);
    }
    if !valid_digest(&trial.acceptance.receipt_digest)
        || !valid_digest(&trial.acceptance.scope_digest)
        || !valid_digest(&trial.acceptance.physical_evidence_digest)
    {
        blockers.push(Blocker::InvalidDigest);
    }
    if trial.acceptance.maturity < Maturity::M4 {
        blockers.push(Blocker::MissingM4Acceptance);
    }
    if trial.acceptance.evidence_class != EvidenceClass::PhysicalMeasurement {
        blockers.push(Blocker::NonPhysicalAcceptanceEvidence);
    }
    if trial.acceptance.disposition == AcceptanceDisposition::Fail {
        blockers.push(Blocker::ComponentAcceptanceFailed);
    } else if trial.acceptance.disposition != AcceptanceDisposition::Qualified {
        blockers.push(Blocker::MissingM4Acceptance);
    }
    let optimization = &trial.optimization;
    if optimization.receipt_id.trim().is_empty() || optimization.optimizer_id.trim().is_empty() {
        blockers.push(Blocker::MissingOptimizationReceipt);
    }
    if !valid_digest(&optimization.digest)
        || !valid_digest(&optimization.search_space_digest)
        || !valid_digest(&optimization.objective_digest)
        || !valid_digest(&optimization.constraint_digest)
        || !valid_digest(&optimization.protocol_digest)
        || optimization.objective_digest != spec.scope.objective_set_digest
        || optimization.constraint_digest != spec.scope.acceptance_policy_digest
    {
        blockers.push(Blocker::InvalidOptimizationReceipt);
    }
    if optimization.evaluations < spec.minimum_optimization_evaluations
        || optimization.independent_restarts < spec.minimum_independent_restarts
    {
        blockers.push(Blocker::InsufficientOptimizationBudget);
    }
    if optimization.budget_tier != spec.optimization_budget_tier {
        blockers.push(Blocker::OptimizationBudgetTierMismatch);
    }
    if !optimization.frozen_before_unseal {
        blockers.push(Blocker::InvalidOptimizationReceipt);
    }

    let service = &trial.service;
    if service.component_id != trial.acceptance.component_id
        || service.duty_profile_digest != spec.scope.duty_profile_digest
        || service.system_boundary_digest != spec.scope.system_boundary_digest
        || service.service_horizon_digest != spec.scope.service_horizon_digest
    {
        blockers.push(Blocker::ServiceScopeMismatch);
    }
    if service.receipt_id.trim().is_empty()
        || service.durability_receipt_id.trim().is_empty()
        || service.method_id.trim().is_empty()
        || service.instrument_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingServiceEvidence);
    }
    if service.durability_evidence_class != EvidenceClass::PhysicalMeasurement
        || service.durability_method_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingDurabilityReceipt);
    }
    if !valid_digest(&service.durability_calibration_digest)
        || !valid_digest(&service.durability_uncertainty_record_digest)
    {
        blockers.push(Blocker::InvalidDigest);
    }
    if !valid_digest(&service.receipt_digest)
        || !valid_digest(&service.durability_receipt_digest)
        || !valid_digest(&service.duty_profile_digest)
        || !valid_digest(&service.system_boundary_digest)
        || !valid_digest(&service.service_horizon_digest)
        || !valid_digest(&service.calibration_digest)
        || !valid_digest(&service.uncertainty_record_digest)
    {
        blockers.push(Blocker::InvalidDigest);
    }
    if service.evidence_class != EvidenceClass::PhysicalMeasurement {
        blockers.push(Blocker::NonPhysicalServiceEvidence);
    }
    if service.service_basis == ServiceBasis::ValidatedProjection {
        blockers.push(Blocker::ProjectionCannotSupportLifetimeClaim);
    }
    if !service.useful_shaft_energy_per_mission_kwh.is_positive()
        || service.completed_mission_count == 0
    {
        blockers.push(Blocker::InvalidServiceInterval);
    }
    if service.completed_mission_count < spec.minimum_completed_missions {
        blockers.push(Blocker::InsufficientCompletedMissions);
    }

    let service_kwh = if service.useful_shaft_energy_per_mission_kwh.is_positive()
        && service.completed_mission_count >= spec.minimum_completed_missions
    {
        service
            .useful_shaft_energy_per_mission_kwh
            .scale_nonnegative(f64::from(service.completed_mission_count))
            .filter(|value| value.is_positive())
    } else {
        None
    };
    if service_kwh.is_none() {
        blockers.push(Blocker::NonPositiveServiceDenominator);
    }

    if trial.material_lots.is_empty() {
        blockers.push(Blocker::InvalidMaterialInventory);
    }
    let mut seen_lots = BTreeSet::new();
    let mut seen_receipts = BTreeSet::new();
    let mut seen_element_assay_ids = BTreeSet::new();
    let required_elements = spec
        .required_elements
        .iter()
        .map(|element| element.element_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut totals: BTreeMap<String, ElementMassTotals> = spec
        .required_elements
        .iter()
        .map(|element| (element.element_id.clone(), ElementMassTotals::default()))
        .collect();

    for lot in &trial.material_lots {
        if lot.material_lot_id.trim().is_empty()
            || lot.component_id != trial.acceptance.component_id
            || lot.material_family_id.trim().is_empty()
        {
            blockers.push(Blocker::MissingIdentity);
            continue;
        }
        if !seen_lots.insert(lot.material_lot_id.as_str()) {
            blockers.push(Blocker::DuplicateMaterialLot);
            continue;
        }
        if !seen_receipts.insert(lot.mass_evidence_id.as_str()) {
            blockers.push(Blocker::DuplicateMaterialReceipt);
        }
        if lot.scope_digest != spec.scope.acceptance_policy_digest
            || lot.inventory_scope_digest != spec.scope.element_inventory_scope_digest
        {
            blockers.push(Blocker::ScopeMismatch);
        }
        if !valid_digest(&lot.scope_digest)
            || !valid_digest(&lot.mass_evidence_digest)
            || !valid_digest(&lot.mass_calibration_digest)
            || !valid_digest(&lot.mass_uncertainty_record_digest)
        {
            blockers.push(Blocker::InvalidDigest);
        }
        if !lot.material_mass_g.is_positive() {
            blockers.push(Blocker::InvalidMaterialInventory);
            continue;
        }
        if lot.mass_evidence_id.trim().is_empty() || lot.mass_method_id.trim().is_empty() {
            blockers.push(Blocker::MissingMassMethod);
        }
        if lot.mass_evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::NonPhysicalMassEvidence);
        }
        if lot.mass_calibration_digest.trim().is_empty() {
            blockers.push(Blocker::MissingMassCalibration);
        }
        if lot.mass_laboratory_id.trim().is_empty() {
            blockers.push(Blocker::MissingMassLaboratoryIdentity);
        }
        if !valid_digest(&lot.mass_laboratory_method_scope_digest) {
            blockers.push(Blocker::InvalidMassLaboratoryMethodScope);
        }
        if lot.mass_uncertainty_record_digest.trim().is_empty() {
            blockers.push(Blocker::MissingMassUncertainty);
        }

        let mut seen_assays = BTreeSet::new();
        let mut summed_fraction = Interval::new(0.0, 0.0);
        for assay in &lot.element_assays {
            if !seen_assays.insert(assay.element_id.as_str()) {
                blockers.push(Blocker::DuplicateElementAssay);
                continue;
            }
            if !required_elements.contains(assay.element_id.as_str()) {
                blockers.push(Blocker::ExtraElementAssay);
                continue;
            }
            if assay.evidence_class != EvidenceClass::PhysicalMeasurement {
                blockers.push(Blocker::NonPhysicalElementEvidence);
                continue;
            }
            if assay.evidence_id.trim().is_empty() || assay.method_id.trim().is_empty() {
                blockers.push(Blocker::MissingElementMethod);
                continue;
            }
            if !seen_element_assay_ids.insert(assay.evidence_id.as_str()) {
                blockers.push(Blocker::DuplicateElementAssay);
                continue;
            }
            if assay.laboratory_id.trim().is_empty() {
                blockers.push(Blocker::MissingLaboratoryIdentity);
                continue;
            }
            if !valid_digest(&assay.laboratory_method_scope_digest) {
                blockers.push(Blocker::InvalidLaboratoryMethodScope);
                continue;
            }
            if !valid_digest(&assay.evidence_digest) {
                blockers.push(Blocker::InvalidDigest);
                continue;
            }
            if !valid_digest(&assay.calibration_digest) {
                blockers.push(Blocker::InvalidElementCalibration);
                continue;
            }
            if !valid_digest(&assay.uncertainty_record_digest) {
                blockers.push(Blocker::InvalidElementUncertainty);
                continue;
            }
            if !assay.mass_fraction.is_valid()
                || assay.mass_fraction.lower < 0.0
                || assay.mass_fraction.upper > 1.0
            {
                blockers.push(Blocker::InvalidElementFraction);
                continue;
            }

            summed_fraction = summed_fraction.add(assay.mass_fraction);
            let Some(element_mass) = lot
                .material_mass_g
                .multiply_nonnegative(assay.mass_fraction)
            else {
                blockers.push(Blocker::ElementArithmeticOverflow);
                continue;
            };

            let entry = totals
                .entry(assay.element_id.clone())
                .or_default();
            entry.total_embodied_g = entry.total_embodied_g.add(element_mass);
            match lot.origin {
                ElementOrigin::Virgin => {
                    entry.virgin_g = entry.virgin_g.add(element_mass);
                }
                ElementOrigin::ExternalSecondary => {
                    entry.external_secondary_g =
                        entry.external_secondary_g.add(element_mass);
                }
                ElementOrigin::InternalRecycled => {
                    entry.internal_recycled_g =
                        entry.internal_recycled_g.add(element_mass);
                }
                ElementOrigin::DirectReuse => {
                    entry.direct_reuse_g = entry.direct_reuse_g.add(element_mass);
                }
            }
        }

        if summed_fraction.lower > 1.0 {
            blockers.push(Blocker::ElementFractionMassClosureFailure);
        } else if summed_fraction.upper > 1.0 {
            blockers.push(Blocker::ElementFractionMassClosureUnresolved);
        }

        for element_id in &required_elements {
            if !seen_assays.contains(element_id) {
                blockers.push(Blocker::MissingElementAssay);
            }
        }
    }

    let mut element_intensities = BTreeMap::new();
    for element in &spec.required_elements {
        let Some(total) = totals.get(&element.element_id).copied() else {
            blockers.push(Blocker::MissingElementAssay);
            continue;
        };
        if !total.virgin_g.is_nonnegative()
            || !total.external_secondary_g.is_nonnegative()
            || !total.internal_recycled_g.is_nonnegative()
            || !total.direct_reuse_g.is_nonnegative()
            || !total.total_embodied_g.is_nonnegative()
        {
            blockers.push(Blocker::ElementArithmeticOverflow);
            continue;
        }
        if let Some(service) = service_kwh {
            let Some(virgin_g_per_mwh) = total
                .virgin_g
                .divide_positive(service)
                .and_then(|value| value.scale_nonnegative(1000.0))
            else {
                blockers.push(Blocker::NonPositiveServiceDenominator);
                continue;
            };
            let external_secondary_g_per_mwh = total
                .external_secondary_g
                .divide_positive(service)
                .and_then(|value| value.scale_nonnegative(1000.0));
            let internal_recycled_g_per_mwh = total
                .internal_recycled_g
                .divide_positive(service)
                .and_then(|value| value.scale_nonnegative(1000.0));
            let direct_reuse_g_per_mwh = total
                .direct_reuse_g
                .divide_positive(service)
                .and_then(|value| value.scale_nonnegative(1000.0));
            let total_embodied_g_per_mwh = total
                .total_embodied_g
                .divide_positive(service)
                .and_then(|value| value.scale_nonnegative(1000.0));

            if external_secondary_g_per_mwh.is_none()
                || internal_recycled_g_per_mwh.is_none()
                || direct_reuse_g_per_mwh.is_none()
                || total_embodied_g_per_mwh.is_none()
            {
                blockers.push(Blocker::ElementArithmeticOverflow);
                continue;
            }

            element_intensities.insert(
                element.element_id.clone(),
                ElementIntensitySummary {
                    element_id: element.element_id.clone(),
                    virgin_g: total.virgin_g,
                    external_secondary_g: total.external_secondary_g,
                    internal_recycled_g: total.internal_recycled_g,
                    direct_reuse_g: total.direct_reuse_g,
                    total_embodied_g: total.total_embodied_g,
                    virgin_g_per_mwh,
                    external_secondary_g_per_mwh,
                    internal_recycled_g_per_mwh,
                    direct_reuse_g_per_mwh,
                    total_embodied_g_per_mwh,
                },
            );
        }
    }

    blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    blockers.dedup();

    TrialSummary {
        service_kwh,
        element_intensities,
        blockers,
    }
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn scope() -> SystemScope {
        SystemScope {
            application_id: "TRACTION-50KW-001".to_string(),
            target_function_digest: digest('a'),
            duty_profile_digest: digest('b'),
            system_boundary_digest: digest('c'),
            inverter_control_digest: digest('d'),
            cooling_boundary_digest: digest('e'),
            measurement_setup_digest: digest('f'),
            acceptance_policy_digest: digest('1'),
            service_horizon_digest: digest('2'),
            element_inventory_scope_digest: digest('3'),
            objective_set_digest: digest('4'),
        }
    }

    fn spec() -> BenchmarkSpec {
        BenchmarkSpec {
            benchmark_id: "CRITICAL-MATERIAL-001".to_string(),
            scope: scope(),
            required_elements: vec![
                ElementSpec {
                    element_id: "Nd".to_string(),
                    unit: "g".to_string(),
                    no_worse_margin_g_per_mwh: 1.0,
                    improvement_margin_g_per_mwh: 0.1,
                },
                ElementSpec {
                    element_id: "Dy".to_string(),
                    unit: "g".to_string(),
                    no_worse_margin_g_per_mwh: 40.0,
                    improvement_margin_g_per_mwh: 0.1,
                },
                ElementSpec {
                    element_id: "Cu".to_string(),
                    unit: "g".to_string(),
                    no_worse_margin_g_per_mwh: 10.0,
                    improvement_margin_g_per_mwh: 1.0,
                },
            ],
            required_baseline_families: vec![
                ArchitectureFamily::ConventionalSinteredPm,
                ArchitectureFamily::PmAssistedSynrm,
            ],
            minimum_optimization_evaluations: 100,
            minimum_independent_restarts: 3,
            optimization_budget_tier: "B1".to_string(),
            minimum_completed_missions: 10.0,
            service_noninferiority_margin_kwh: 35.0,
        }
    }

    fn acceptance(component: &str) -> ComponentAcceptance {
        ComponentAcceptance {
            component_id: component.to_string(),
            specimen_id: format!("SPEC-{component}"),
            receipt_id: format!("ACCEPT-{component}"),
            receipt_digest: digest('4'),
            scope_digest: digest('1'),
            maturity: Maturity::M4,
            disposition: AcceptanceDisposition::Qualified,
            evidence_class: EvidenceClass::PhysicalMeasurement,
            physical_evidence_digest: digest('5'),
        }
    }

    fn service(component: &str, energy_per_mission: f64, missions: f64) -> ServiceEvidence {
        ServiceEvidence {
            receipt_id: format!("SERVICE-{component}"),
            receipt_digest: digest('6'),
            component_id: component.to_string(),
            duty_profile_digest: digest('b'),
            system_boundary_digest: digest('c'),
            service_horizon_digest: digest('2'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            service_basis: ServiceBasis::DurabilityTestedService,
            useful_shaft_energy_per_mission_kwh: Interval::new(
                energy_per_mission - 0.1,
                energy_per_mission + 0.1,
            ),
            completed_mission_count: missions,
            durability_receipt_id: format!("DURABILITY-{component}"),
            durability_receipt_digest: digest('7'),
            durability_evidence_class: EvidenceClass::PhysicalMeasurement,
            durability_method_id: "DURABILITY-TEST-METHOD-001".to_string(),
            durability_calibration_digest: digest('a'),
            durability_uncertainty_record_digest: digest('b'),
            method_id: "SHAFT-ENERGY-METHOD-001".to_string(),
            calibration_digest: digest('8'),
            uncertainty_record_digest: digest('9'),
            instrument_id: "DYNAMOMETER-001".to_string(),
        }
    }

    fn optimization(component: &str) -> OptimizationReceipt {
        OptimizationReceipt {
            receipt_id: format!("OPT-{component}"),
            digest: digest('a'),
            optimizer_id: "MOTOR-OPTIMIZER-001".to_string(),
            search_space_digest: digest('b'),
            objective_digest: digest('4'),
            constraint_digest: digest('1'),
            protocol_digest: digest('c'),
            evaluations: 500,
            independent_restarts: 5,
            budget_tier: "B1".to_string(),
            frozen_before_unseal: true,
        }
    }

    fn lot(
        component: &str,
        lot_id: &str,
        origin: ElementOrigin,
        mass_g: f64,
        nd_fraction: f64,
        dy_fraction: f64,
        cu_fraction: f64,
        suffix: char,
    ) -> MaterialLotEvidence {
        MaterialLotEvidence {
            material_lot_id: lot_id.to_string(),
            component_id: component.to_string(),
            material_family_id: "MATERIAL-FAMILY-001".to_string(),
            origin,
            scope_digest: digest('1'),
            inventory_scope_digest: digest('3'),
            material_mass_g: Interval::new(mass_g - 0.01, mass_g + 0.01),
            mass_evidence_id: format!("MASS-{lot_id}"),
            mass_evidence_digest: digest(suffix),
            mass_method_id: "CALIBRATED-SCALE-001".to_string(),
            mass_calibration_digest: digest('d'),
            mass_uncertainty_record_digest: digest('e'),
            mass_laboratory_id: "METROLOGY-LAB-001".to_string(),
            mass_laboratory_method_scope_digest: digest('f'),
            mass_evidence_class: EvidenceClass::PhysicalMeasurement,
            element_assays: vec![
                assay("Nd", nd_fraction, suffix),
                assay("Dy", dy_fraction, '4'),
                assay("Cu", cu_fraction, '5'),
            ],
        }
    }

    fn assay(element: &str, fraction: f64, suffix: char) -> ElementFractionAssay {
        ElementFractionAssay {
            element_id: element.to_string(),
            mass_fraction: if fraction == 0.0 {
                Interval::new(0.0, 0.0)
            } else {
                Interval::new(
                    (fraction - 0.0001).max(0.0),
                    (fraction + 0.0001).min(1.0),
                )
            },
            evidence_id: format!("ASSAY-{element}-{suffix}"),
            evidence_digest: digest(suffix),
            method_id: "ICP-MS-METHOD-001".to_string(),
            calibration_digest: digest('f'),
            uncertainty_record_digest: digest('1'),
            laboratory_id: "CHEMISTRY-LAB-001".to_string(),
            laboratory_method_scope_digest: digest('a'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn trial(
        id: &str,
        family: ArchitectureFamily,
        role: ArchitectureRole,
        magnet_mass_g: f64,
        nd_fraction: f64,
        dy_fraction: f64,
        service_per_mission_kwh: f64,
        mission_count: f64,
    ) -> ArchitectureTrial {
        ArchitectureTrial {
            architecture_id: id.to_string(),
            family,
            role,
            scope: scope(),
            acceptance: acceptance(id),
            service: service(id, service_per_mission_kwh, mission_count),
            optimization: optimization(id),
            material_lots: vec![
                lot(
                    id,
                    &format!("MAG-{id}"),
                    ElementOrigin::Virgin,
                    magnet_mass_g,
                    nd_fraction,
                    dy_fraction,
                    0.0,
                    '2',
                ),
                lot(
                    id,
                    &format!("CU-{id}"),
                    ElementOrigin::Virgin,
                    500.0,
                    0.0,
                    0.0,
                    0.70,
                    '3',
                ),
            ],
        }
    }

    fn baselines() -> Vec<ArchitectureTrial> {
        vec![
            trial(
                "SINTERED-IPM",
                ArchitectureFamily::ConventionalSinteredPm,
                ArchitectureRole::RequiredBaseline,
                1000.0,
                0.28,
                0.02,
                50.0,
                100.0,
            ),
            trial(
                "PMASS-SYNRM",
                ArchitectureFamily::PmAssistedSynrm,
                ArchitectureRole::RequiredBaseline,
                700.0,
                0.28,
                0.02,
                50.0,
                100.0,
            ),
        ]
    }

    fn run(candidate: ArchitectureTrial) -> ArchitectureEvaluation {
        let mut trials = vec![candidate];
        trials.extend(baselines());
        evaluate_critical_material_intensity(&spec(), &trials)
            .into_iter()
            .next()
            .expect("candidate evaluation")
    }

    #[test]
    fn robust_element_reduction_per_service_is_critical_material_advantage() {
        let result = run(trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.0215,
            50.0,
            100.0,
        ));
        assert_eq!(result.decision, Decision::CriticalMaterialAdvantage);
        assert!(result.robustly_improved_elements.contains(&"Nd".to_string()));
        assert!(!result.robustly_improved_elements.contains(&"Dy".to_string()));
    }

    #[test]
    fn recycled_content_is_separate_from_virgin_input() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.material_lots[0].origin = ElementOrigin::ExternalSecondary;
        let result = run(candidate);
        assert_eq!(result.decision, Decision::CriticalMaterialAdvantage);
        let nd = result
            .element_intensities
            .iter()
            .find(|item| item.element_id == "Nd")
            .unwrap();
        assert!(nd.virgin_g.upper < 0.01);
        assert!(nd.external_secondary_g.lower > 40.0);
    }

    #[test]
    fn all_equal_element_intensity_is_noninferior_without_advantage() {
        let result = run(trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            700.0,
            0.28,
            0.02,
            50.0,
            100.0,
        ));
        assert_eq!(result.decision, Decision::NonInferiorNoMaterialAdvantage);
    }

    #[test]
    fn increase_in_dysprosium_blocks_a_nd_only_improvement_claim() {
        let result = run(trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.05,
            50.0,
            100.0,
        ));
        assert_ne!(result.decision, Decision::CriticalMaterialAdvantage);
    }

    #[test]
    fn reduced_service_cannot_be_hidden_by_lower_element_mass() {
        let result = run(trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            40.0,
            100.0,
        ));
        assert_eq!(result.decision, Decision::InferiorToBaseline);
        assert!(result.blockers.contains(&Blocker::ServiceNoninferiorityViolation));
    }

    #[test]
    fn missing_element_assay_is_not_treated_as_zero() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.material_lots[0]
            .element_assays
            .retain(|assay| assay.element_id != "Dy");
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingElementAssay));
    }

    #[test]
    fn duplicate_material_lot_cannot_be_counted_twice() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.material_lots.push(candidate.material_lots[0].clone());
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::DuplicateMaterialLot));
    }

    #[test]
    fn zero_or_nonpositive_service_denominator_is_indeterminate() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.service.useful_shaft_energy_per_mission_kwh = Interval::new(0.0, 0.0);
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonPositiveServiceDenominator));
    }

    #[test]
    fn modeled_service_projection_cannot_claim_observed_lifetime_service() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.service.service_basis = ServiceBasis::ValidatedProjection;
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::ProjectionCannotSupportLifetimeClaim));
    }

    #[test]
    fn wrong_duty_profile_is_not_comparable() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.service.duty_profile_digest = digest('0');
        let result = run(candidate);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ServiceScopeMismatch));
    }

    #[test]
    fn missing_required_baseline_family_is_not_comparable() {
        let candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        let trials = vec![candidate, baselines()[0].clone()];
        let specification = spec();
        let evaluations = evaluate_critical_material_intensity(&specification, &trials);
        assert_eq!(evaluations[0].decision, Decision::NotComparable);
    }

    #[test]
    fn insufficient_optimization_budget_is_indeterminate() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.optimization.evaluations = 10;
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InsufficientOptimizationBudget));
    }

    #[test]
    fn synthetic_material_assays_cannot_claim_a_physical_advantage() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.material_lots[0].mass_evidence_class = EvidenceClass::SyntheticControl;
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonPhysicalMassEvidence));
    }

    #[test]
    fn material_fraction_above_unity_is_rejected() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.material_lots[0].element_assays[0].mass_fraction =
            Interval::new(1.01, 1.02);
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InvalidElementFraction));
    }

    #[test]
    fn direct_reuse_and_internal_recycling_are_not_subtracted_from_virgin_mass() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.material_lots[0].origin = ElementOrigin::DirectReuse;
        candidate.material_lots[1].origin = ElementOrigin::InternalRecycled;
        let result = run(candidate);
        let nd = result
            .element_intensities
            .iter()
            .find(|item| item.element_id == "Nd")
            .unwrap();
        assert_eq!(nd.virgin_g, Interval::new(0.0, 0.0));
        assert!(nd.direct_reuse_g.upper > 0.0);
    }

    #[test]
    fn element_mass_fraction_sum_above_unity_is_a_hard_failure() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.80,
            0.30,
            50.0,
            100.0,
        );
        candidate.material_lots[0].element_assays[0].mass_fraction =
            Interval::new(0.80, 0.81);
        candidate.material_lots[0].element_assays[1].mass_fraction =
            Interval::new(0.30, 0.31);
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Fail);
        assert!(result
            .blockers
            .contains(&Blocker::ElementFractionMassClosureFailure));
    }

    #[test]
    fn element_mass_fraction_interval_crossing_unity_is_indeterminate() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.80,
            0.19,
            50.0,
            100.0,
        );
        candidate.material_lots[0].element_assays[0].mass_fraction =
            Interval::new(0.80, 0.81);
        candidate.material_lots[0].element_assays[1].mass_fraction =
            Interval::new(0.19, 0.21);
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result
            .blockers
            .contains(&Blocker::ElementFractionMassClosureUnresolved));
    }

    #[test]
    fn cooling_boundary_change_makes_element_comparison_not_comparable() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.0215,
            50.0,
            100.0,
        );
        candidate.scope.cooling_boundary_digest = digest('0');
        let result = run(candidate);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn candidate_and_baseline_must_use_the_declared_budget_tier() {
        let mut candidate = trial(
            "AM-HYBRID",
            ArchitectureFamily::AmHybridArchitecture,
            ArchitectureRole::Candidate,
            650.0,
            0.28,
            0.02,
            50.0,
            100.0,
        );
        candidate.optimization.budget_tier = "B0".to_string();
        let result = run(candidate);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::OptimizationBudgetTierMismatch));
    }
}
