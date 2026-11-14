//! Claim-scoped construction reference-class admission.
//!
//! This crate validates whether a completed construction comparison may be retained
//! as local descriptive evidence, a matched reference candidate, a conditional
//! quasi-experimental claim, a randomized-design claim, or an independently
//! replicated reference. It does not estimate causal effects, fit predictive models,
//! authenticate source receipts, or update a planner.
//!
//! Promotion is deliberately conservative. Matching observed covariates does not
//! eliminate unmeasured confounding. Quasi-experimental eligibility remains conditional
//! on declared identification assumptions and diagnostics. Randomization labels require
//! allocation records and cluster-aware analysis evidence.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionalUnitBinding {
    pub contract_id: String,
    pub contract_digest: String,
    pub unit_class_id: String,
    pub unit_symbol: String,
    pub acceptance_contract_digest: String,
    pub currency_code: String,
    pub assessment_horizon_months: u64,
}

impl FunctionalUnitBinding {
    fn validate(&self) -> Result<(), LearningError> {
        require_non_empty("contract_id", &self.contract_id)?;
        require_non_empty("contract_digest", &self.contract_digest)?;
        require_non_empty("unit_class_id", &self.unit_class_id)?;
        require_non_empty("unit_symbol", &self.unit_symbol)?;
        require_non_empty(
            "acceptance_contract_digest",
            &self.acceptance_contract_digest,
        )?;
        validate_currency_code(&self.currency_code)?;
        if self.assessment_horizon_months == 0 {
            return Err(LearningError::ZeroAssessmentHorizon);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectContext {
    pub project_id: String,
    pub site_id: String,
    pub floor_or_zone_id: String,
    pub work_package_id: String,
    pub task_family_id: String,
    pub design_revision_digest: String,
    pub method_profile_digest: String,
    pub fleet_profile_digest: String,
    pub site_epoch: String,
    pub target_frame_id: String,
    /// Declared, non-personal context fields such as shift class, ODD, material family,
    /// geometry stratum, access regime or weather category.
    pub attributes: BTreeMap<String, String>,
}

impl ProjectContext {
    fn validate(&self) -> Result<(), LearningError> {
        require_non_empty("project_id", &self.project_id)?;
        require_non_empty("site_id", &self.site_id)?;
        require_non_empty("floor_or_zone_id", &self.floor_or_zone_id)?;
        require_non_empty("work_package_id", &self.work_package_id)?;
        require_non_empty("task_family_id", &self.task_family_id)?;
        require_non_empty("design_revision_digest", &self.design_revision_digest)?;
        require_non_empty("method_profile_digest", &self.method_profile_digest)?;
        require_non_empty("fleet_profile_digest", &self.fleet_profile_digest)?;
        require_non_empty("site_epoch", &self.site_epoch)?;
        require_non_empty("target_frame_id", &self.target_frame_id)?;
        for (key, value) in &self.attributes {
            require_non_empty("context_attribute_key", key)?;
            require_non_empty("context_attribute_value", value)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeAccounting {
    pub planned_units: u64,
    pub attempted_units: u64,
    pub first_pass_inspected_units: u64,
    pub first_pass_accepted_units: u64,
    pub finally_accepted_units: u64,
    pub finally_rejected_units: u64,
    pub unresolved_units: u64,
    pub rework_attempts: u64,
    pub total_workflow_crew_minutes: u64,
    pub elapsed_minutes: u64,
    pub direct_cost_minor_units: u64,
    pub shared_cost_minor_units: u64,
    pub energy_wh: u64,
    pub carbon_gco2e: u64,
}

impl OutcomeAccounting {
    fn validate(&self) -> Result<(), LearningError> {
        if self.attempted_units > self.planned_units {
            return Err(LearningError::AttemptedExceedsPlanned);
        }
        if self.first_pass_inspected_units > self.attempted_units
            || self.first_pass_accepted_units > self.first_pass_inspected_units
        {
            return Err(LearningError::InvalidFirstPassDenominator);
        }
        if self.finally_accepted_units > self.attempted_units
            || self.finally_rejected_units > self.attempted_units
            || self.unresolved_units > self.attempted_units
        {
            return Err(LearningError::InvalidTerminalDisposition);
        }
        let reconciled = self
            .finally_accepted_units
            .checked_add(self.finally_rejected_units)
            .and_then(|value| value.checked_add(self.unresolved_units))
            .ok_or(LearningError::ArithmeticOverflow)?;
        if reconciled != self.attempted_units {
            return Err(LearningError::OutcomeDenominatorMismatch {
                attempted: self.attempted_units,
                reconciled,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseRole {
    Descriptive,
    Treatment,
    Control,
    PreIntervention,
    PostIntervention,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEvidenceStatus {
    AdmissibleForDeclaredClaim,
    Unknown,
    Invalidated,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEvidence {
    pub evidence_id: String,
    pub artifact_ref: String,
    pub content_digest: String,
    pub supported_claim_ref: String,
    pub status: SourceEvidenceStatus,
}

impl SourceEvidence {
    fn validate(&self) -> Result<(), LearningError> {
        require_non_empty("evidence_id", &self.evidence_id)?;
        require_non_empty("artifact_ref", &self.artifact_ref)?;
        require_non_empty("content_digest", &self.content_digest)?;
        require_non_empty("supported_claim_ref", &self.supported_claim_ref)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseRecord {
    pub case_id: String,
    pub case_digest: String,
    pub role: CaseRole,
    pub functional_unit: FunctionalUnitBinding,
    pub context: ProjectContext,
    pub outcome: OutcomeAccounting,
    pub source_evidence: Vec<SourceEvidence>,
}

impl CaseRecord {
    fn validate(&self, expected: &FunctionalUnitBinding) -> Result<(), LearningError> {
        require_non_empty("case_id", &self.case_id)?;
        require_non_empty("case_digest", &self.case_digest)?;
        self.functional_unit.validate()?;
        self.context.validate()?;
        self.outcome.validate()?;
        if self.functional_unit != *expected {
            return Err(LearningError::FunctionalUnitMismatch(self.case_id.clone()));
        }
        if self.source_evidence.is_empty() {
            return Err(LearningError::MissingSourceEvidence(self.case_id.clone()));
        }
        let mut evidence_ids = BTreeSet::new();
        for evidence in &self.source_evidence {
            evidence.validate()?;
            if !evidence_ids.insert(evidence.evidence_id.as_str()) {
                return Err(LearningError::DuplicateEvidenceId(
                    evidence.evidence_id.clone(),
                ));
            }
        }
        Ok(())
    }

    fn has_uncertain_source(&self) -> bool {
        self.source_evidence.iter().any(|evidence| {
            evidence.status != SourceEvidenceStatus::AdmissibleForDeclaredClaim
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicabilityEnvelope {
    pub envelope_id: String,
    pub dimensions: BTreeMap<String, BTreeSet<String>>,
    pub excluded_values: BTreeMap<String, BTreeSet<String>>,
}

impl ApplicabilityEnvelope {
    fn validate(&self) -> Result<(), LearningError> {
        require_non_empty("applicability_envelope_id", &self.envelope_id)?;
        if self.dimensions.is_empty() {
            return Err(LearningError::EmptyApplicabilityEnvelope);
        }
        for (dimension, accepted) in &self.dimensions {
            require_non_empty("applicability_dimension", dimension)?;
            if accepted.is_empty() {
                return Err(LearningError::EmptyApplicabilityValues(dimension.clone()));
            }
            for value in accepted {
                require_non_empty("applicability_value", value)?;
            }
        }
        for (dimension, excluded) in &self.excluded_values {
            require_non_empty("excluded_dimension", dimension)?;
            for value in excluded {
                require_non_empty("excluded_value", value)?;
            }
        }
        Ok(())
    }

    pub fn evaluate(&self, context: &ProjectContext) -> ApplicabilityDisposition {
        let mut missing = Vec::new();
        let mut outside = Vec::new();

        for (dimension, accepted) in &self.dimensions {
            match context.attributes.get(dimension) {
                None => missing.push(dimension.clone()),
                Some(value) => {
                    if self
                        .excluded_values
                        .get(dimension)
                        .is_some_and(|values| values.contains(value))
                        || !accepted.contains(value)
                    {
                        outside.push(dimension.clone());
                    }
                }
            }
        }

        if !outside.is_empty() {
            outside.sort();
            ApplicabilityDisposition::OutOfDomain { dimensions: outside }
        } else if !missing.is_empty() {
            missing.sort();
            ApplicabilityDisposition::InsufficientContext { dimensions: missing }
        } else {
            ApplicabilityDisposition::WithinDeclaredEnvelope
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataPartition {
    Training,
    Validation,
    Test,
    BlindChallenge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartitionAssignment {
    pub case_id: String,
    pub partition: DataPartition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    PreInterventionTrend,
    PlaceboOutcome,
    ShuffledTreatment,
    SensitivityAnalysis,
    CommonCauseReview,
    RandomizationIntegrity,
    ContaminationAssessment,
    IntentToTreatAnalysis,
    ClusterCorrectedAnalysis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDisposition {
    Passed,
    Failed,
    NotEvaluated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FalsificationDiagnostic {
    pub kind: DiagnosticKind,
    pub disposition: DiagnosticDisposition,
    pub evidence_ref: String,
}

impl FalsificationDiagnostic {
    fn validate(&self) -> Result<(), LearningError> {
        require_non_empty("diagnostic_evidence_ref", &self.evidence_ref)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignmentUnit {
    Task,
    WorkPackage,
    Shift,
    Floor,
    Site,
    Project,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuasiExperimentalMethod {
    DifferenceInDifferences,
    InterruptedTimeSeries,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudyDesign {
    Descriptive {
        rationale_ref: String,
    },
    MatchedObservational {
        treatment_case_ids: Vec<String>,
        control_case_ids: Vec<String>,
        matching_variable_ids: Vec<String>,
        balance_report_ref: String,
        balance_disposition: DiagnosticDisposition,
    },
    QuasiExperimental {
        method: QuasiExperimentalMethod,
        pre_case_ids: Vec<String>,
        post_case_ids: Vec<String>,
        assignment_unit: AssignmentUnit,
        preregistration_ref: String,
        intervention_ref: String,
        identification_assumptions: Vec<String>,
        diagnostics: Vec<FalsificationDiagnostic>,
    },
    Randomized {
        treatment_case_ids: Vec<String>,
        control_case_ids: Vec<String>,
        assignment_unit: AssignmentUnit,
        preregistration_ref: String,
        allocation_protocol_ref: String,
        allocation_record_ref: String,
        intent_to_treat_ref: String,
        diagnostics: Vec<FalsificationDiagnostic>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectEstimate {
    pub functional_contract_digest: String,
    /// Exact case set summarized by this effect report. This must equal the
    /// declared treatment/control or pre/post study population for v0.1.
    pub comparison_case_ids: Vec<String>,
    pub metric_id: String,
    pub unit_symbol: String,
    /// Fixed-point divisor: actual value = stored integer / scale.
    pub scale: u64,
    /// Treatment-minus-control or other predeclared contrast. This crate does not
    /// calculate or independently verify the estimate.
    pub estimate: i64,
    pub lower_bound: i64,
    pub upper_bound: i64,
    pub estimate_method_ref: String,
    pub interval_method_ref: String,
    pub source_report_ref: String,
    pub observation_count: u64,
    pub independent_cluster_count: u64,
}

impl EffectEstimate {
    fn validate(
        &self,
        functional_contract_digest: &str,
        expected_case_ids: &BTreeSet<String>,
    ) -> Result<(), LearningError> {
        require_non_empty("effect_functional_contract_digest", &self.functional_contract_digest)?;
        if self.functional_contract_digest != functional_contract_digest {
            return Err(LearningError::EffectContractMismatch(self.metric_id.clone()));
        }
        if self.comparison_case_ids.is_empty() {
            return Err(LearningError::EmptyEffectCaseSet(self.metric_id.clone()));
        }
        let mut comparison_cases = BTreeSet::new();
        for case_id in &self.comparison_case_ids {
            require_non_empty("effect_comparison_case_id", case_id)?;
            if !comparison_cases.insert(case_id.clone()) {
                return Err(LearningError::DuplicateEffectCase(
                    self.metric_id.clone(),
                    case_id.clone(),
                ));
            }
        }
        if &comparison_cases != expected_case_ids {
            return Err(LearningError::EffectCaseSetMismatch(self.metric_id.clone()));
        }
        require_non_empty("effect_metric_id", &self.metric_id)?;
        require_non_empty("effect_unit_symbol", &self.unit_symbol)?;
        require_non_empty("estimate_method_ref", &self.estimate_method_ref)?;
        require_non_empty("interval_method_ref", &self.interval_method_ref)?;
        require_non_empty("source_report_ref", &self.source_report_ref)?;
        if self.scale == 0 {
            return Err(LearningError::ZeroEffectScale(self.metric_id.clone()));
        }
        if self.lower_bound > self.estimate || self.estimate > self.upper_bound {
            return Err(LearningError::InvalidEffectInterval(self.metric_id.clone()));
        }
        if self.observation_count == 0 || self.independent_cluster_count == 0 {
            return Err(LearningError::MissingEffectDenominator(self.metric_id.clone()));
        }
        if self.independent_cluster_count > self.observation_count {
            return Err(LearningError::InvalidClusterCount(self.metric_id.clone()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationEvidence {
    pub replication_protocol_ref: String,
    pub included_case_ids: Vec<String>,
    pub independent_assessor_refs: Vec<String>,
}

impl ReplicationEvidence {
    fn validate(&self, cases: &BTreeMap<&str, &CaseRecord>) -> Result<(), LearningError> {
        require_non_empty("replication_protocol_ref", &self.replication_protocol_ref)?;
        if self.included_case_ids.is_empty() {
            return Err(LearningError::EmptyReplicationSet);
        }
        let mut site_ids = BTreeSet::new();
        let mut project_ids = BTreeSet::new();
        let mut case_ids = BTreeSet::new();
        for case_id in &self.included_case_ids {
            require_non_empty("replication_case_id", case_id)?;
            if !case_ids.insert(case_id.as_str()) {
                return Err(LearningError::DuplicateReplicationCase(case_id.clone()));
            }
            let case = cases
                .get(case_id.as_str())
                .ok_or_else(|| LearningError::UnknownReplicationCase(case_id.clone()))?;
            site_ids.insert(case.context.site_id.as_str());
            project_ids.insert(case.context.project_id.as_str());
        }
        if site_ids.len() < 2 || project_ids.len() < 2 {
            return Err(LearningError::InsufficientIndependentSites);
        }
        let mut assessors = BTreeSet::new();
        for assessor in &self.independent_assessor_refs {
            require_non_empty("independent_assessor_ref", assessor)?;
            if !assessors.insert(assessor.as_str()) {
                return Err(LearningError::DuplicateIndependentAssessor(assessor.clone()));
            }
        }
        if assessors.len() < 2 {
            return Err(LearningError::InsufficientIndependentAssessors);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonManifest {
    pub study_id: String,
    pub study_digest: String,
    pub functional_unit: FunctionalUnitBinding,
    pub applicability: ApplicabilityEnvelope,
    pub cases: Vec<CaseRecord>,
    pub study_design: StudyDesign,
    pub effects: Vec<EffectEstimate>,
    pub partition_assignments: Vec<PartitionAssignment>,
    pub replication: Option<ReplicationEvidence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Descriptive,
    MatchedObservational,
    QuasiExperimental,
    Randomized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalClaimDisposition {
    NotSupported,
    ConditionalOnQuasiExperimentalAssumptions,
    RandomizedDesignEligible,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicationDisposition {
    NotClaimed,
    InsufficientEvidence,
    IndependentCrossSiteReplication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicabilityDisposition {
    WithinDeclaredEnvelope,
    OutOfDomain { dimensions: Vec<String> },
    InsufficientContext { dimensions: Vec<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionDisposition {
    LocalDescriptiveOnly,
    MatchedReferenceCandidate,
    QuasiExperimentalClaimEligible,
    RandomizedClaimEligible,
    IndependentlyReplicatedDescriptiveReference,
    IndependentlyReplicatedQuasiExperimentalReference,
    IndependentlyReplicatedRandomizedReference,
    Quarantined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearningValidationReport {
    pub study_id: String,
    pub evidence_class: EvidenceClass,
    pub causal_claim: CausalClaimDisposition,
    pub replication: ReplicationDisposition,
    pub promotion: PromotionDisposition,
    pub applicability: ApplicabilityDisposition,
    pub case_count: usize,
    pub attempted_units: u64,
    pub finally_accepted_units: u64,
    pub finally_rejected_units: u64,
    pub unresolved_units: u64,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LearningError {
    EmptyField(&'static str),
    InvalidCurrencyCode(String),
    ZeroAssessmentHorizon,
    AttemptedExceedsPlanned,
    InvalidFirstPassDenominator,
    InvalidTerminalDisposition,
    OutcomeDenominatorMismatch { attempted: u64, reconciled: u64 },
    FunctionalUnitMismatch(String),
    MissingSourceEvidence(String),
    DuplicateEvidenceId(String),
    DuplicateCaseId(String),
    EmptyApplicabilityEnvelope,
    EmptyApplicabilityValues(String),
    DuplicatePartitionAssignment(String),
    MissingPartitionAssignment(String),
    PartitionCaseUnknown(String),
    ProjectLeakage(String),
    SiteLeakage(String),
    EmptyStudy,
    UnknownDesignCase(String),
    DuplicateDesignCase(String),
    InvalidRoleForDesign(String),
    MissingDesignEvidence(&'static str),
    EmptyMatchingVariables,
    MatchedBalanceNotEstablished,
    EmptyIdentificationAssumptions,
    MissingRequiredDiagnostic(DiagnosticKind),
    FailedRequiredDiagnostic(DiagnosticKind),
    UnevaluatedRequiredDiagnostic(DiagnosticKind),
    ZeroEffectScale(String),
    InvalidEffectInterval(String),
    MissingEffectDenominator(String),
    InvalidClusterCount(String),
    DuplicateEffectMetric(String),
    DuplicateEffectCase(String, String),
    EmptyEffectCaseSet(String),
    EffectCaseSetMismatch(String),
    EffectContractMismatch(String),
    DuplicateDiagnostic(DiagnosticKind),
    DuplicateMatchingVariable(String),
    EffectContextMismatch(String),
    EmptyReplicationSet,
    DuplicateReplicationCase(String),
    UnknownReplicationCase(String),
    InsufficientIndependentSites,
    DuplicateIndependentAssessor(String),
    InsufficientIndependentAssessors,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DesignAssessment {
    evidence_class: EvidenceClass,
    causal_claim: CausalClaimDisposition,
    promotion: PromotionDisposition,
    quarantined: bool,
}

pub fn evaluate_comparison(
    manifest: &ComparisonManifest,
) -> Result<LearningValidationReport, LearningError> {
    require_non_empty("study_id", &manifest.study_id)?;
    require_non_empty("study_digest", &manifest.study_digest)?;
    manifest.functional_unit.validate()?;
    manifest.applicability.validate()?;
    if manifest.cases.is_empty() {
        return Err(LearningError::EmptyStudy);
    }

    let mut cases = BTreeMap::new();
    for case in &manifest.cases {
        case.validate(&manifest.functional_unit)?;
        if cases.insert(case.case_id.as_str(), case).is_some() {
            return Err(LearningError::DuplicateCaseId(case.case_id.clone()));
        }
        if !matches!(
            manifest.applicability.evaluate(&case.context),
            ApplicabilityDisposition::WithinDeclaredEnvelope
        ) {
            return Err(LearningError::EffectContextMismatch(case.case_id.clone()));
        }
    }

    let partition = validate_partitions(&manifest.cases, &manifest.partition_assignments)?;
    let _ = partition;

    let mut uncertain_sources = false;
    let mut attempted_units = 0_u64;
    let mut finally_accepted_units = 0_u64;
    let mut finally_rejected_units = 0_u64;
    let mut unresolved_units = 0_u64;

    for case in &manifest.cases {
        uncertain_sources |= case.has_uncertain_source();
        attempted_units = checked_add(attempted_units, case.outcome.attempted_units)?;
        finally_accepted_units =
            checked_add(finally_accepted_units, case.outcome.finally_accepted_units)?;
        finally_rejected_units =
            checked_add(finally_rejected_units, case.outcome.finally_rejected_units)?;
        unresolved_units = checked_add(unresolved_units, case.outcome.unresolved_units)?;
    }

    let design = assess_study_design(&manifest.study_design, &cases)?;
    let expected_effect_case_ids = declared_effect_case_ids(&manifest.study_design, &cases)?;
    let mut metric_ids = BTreeSet::new();
    for effect in &manifest.effects {
        effect.validate(
            &manifest.functional_unit.contract_digest,
            &expected_effect_case_ids,
        )?;
        if !metric_ids.insert(effect.metric_id.as_str()) {
            return Err(LearningError::DuplicateEffectMetric(effect.metric_id.clone()));
        }
    }

    let mut reasons = Vec::new();
    let mut promotion = design.promotion;
    let mut causal_claim = design.causal_claim;
    let mut quarantined = design.quarantined;

    if uncertain_sources {
        quarantined = true;
        reasons.push("one or more source artifacts are unknown, invalidated or revoked".to_owned());
    }
    if unresolved_units > 0 {
        reasons.push("unresolved work remains in the source outcome denominator".to_owned());
        if promotion != PromotionDisposition::LocalDescriptiveOnly {
            promotion = PromotionDisposition::LocalDescriptiveOnly;
        }
        causal_claim = CausalClaimDisposition::Rejected;
    }

    let applicability = ApplicabilityDisposition::WithinDeclaredEnvelope;
    let replication = if let Some(replication) = &manifest.replication {
        match replication.validate(&cases) {
            Ok(()) => ReplicationDisposition::IndependentCrossSiteReplication,
            Err(error) => {
                reasons.push(format!("replication not established: {error:?}"));
                ReplicationDisposition::InsufficientEvidence
            }
        }
    } else {
        ReplicationDisposition::NotClaimed
    };

    if quarantined {
        promotion = PromotionDisposition::Quarantined;
        causal_claim = CausalClaimDisposition::Rejected;
    } else if unresolved_units == 0
        && replication == ReplicationDisposition::IndependentCrossSiteReplication
    {
        promotion = match design.evidence_class {
            EvidenceClass::Descriptive => {
                PromotionDisposition::IndependentlyReplicatedDescriptiveReference
            }
            EvidenceClass::MatchedObservational => PromotionDisposition::IndependentlyReplicatedDescriptiveReference,
            EvidenceClass::QuasiExperimental => {
                PromotionDisposition::IndependentlyReplicatedQuasiExperimentalReference
            }
            EvidenceClass::Randomized => {
                PromotionDisposition::IndependentlyReplicatedRandomizedReference
            }
        };
    }

    if manifest.effects.is_empty() {
        reasons.push("no independently reported effect estimate is attached".to_owned());
    }
    if partition == PartitionValidation::NoBlindHoldout {
        reasons.push("no blind challenge partition is present; holdout generalization is not evaluated".to_owned());
    }
    if promotion == PromotionDisposition::LocalDescriptiveOnly
        && !reasons.iter().any(|reason| reason.contains("local"))
    {
        reasons.push("comparison remains local; it is not promoted as a causal or transferable claim".to_owned());
    }

    Ok(LearningValidationReport {
        study_id: manifest.study_id.clone(),
        evidence_class: design.evidence_class,
        causal_claim,
        replication,
        promotion,
        applicability,
        case_count: manifest.cases.len(),
        attempted_units,
        finally_accepted_units,
        finally_rejected_units,
        unresolved_units,
        reasons,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PartitionValidation {
    Complete,
    NoBlindHoldout,
}

fn validate_partitions(
    cases: &[CaseRecord],
    assignments: &[PartitionAssignment],
) -> Result<PartitionValidation, LearningError> {
    let mut case_by_id = BTreeMap::new();
    for case in cases {
        case_by_id.insert(case.case_id.as_str(), case);
    }

    let mut assigned_cases = BTreeSet::new();
    let mut project_partition = BTreeMap::new();
    let mut site_partition = BTreeMap::new();
    let mut has_blind = false;

    for assignment in assignments {
        require_non_empty("partition_case_id", &assignment.case_id)?;
        let case = case_by_id
            .get(assignment.case_id.as_str())
            .ok_or_else(|| LearningError::PartitionCaseUnknown(assignment.case_id.clone()))?;
        if !assigned_cases.insert(assignment.case_id.as_str()) {
            return Err(LearningError::DuplicatePartitionAssignment(
                assignment.case_id.clone(),
            ));
        }
        has_blind |= assignment.partition == DataPartition::BlindChallenge;

        if let Some(previous) =
            project_partition.insert(case.context.project_id.as_str(), assignment.partition)
            && previous != assignment.partition
        {
            return Err(LearningError::ProjectLeakage(
                case.context.project_id.clone(),
            ));
        }
        if let Some(previous) =
            site_partition.insert(case.context.site_id.as_str(), assignment.partition)
            && previous != assignment.partition
        {
            return Err(LearningError::SiteLeakage(case.context.site_id.clone()));
        }
    }

    if assigned_cases.len() != cases.len() {
        for case in cases {
            if !assigned_cases.contains(case.case_id.as_str()) {
                return Err(LearningError::MissingPartitionAssignment(
                    case.case_id.clone(),
                ));
            }
        }
    }

    Ok(if has_blind {
        PartitionValidation::Complete
    } else {
        PartitionValidation::NoBlindHoldout
    })
}

fn declared_effect_case_ids(
    design: &StudyDesign,
    cases: &BTreeMap<&str, &CaseRecord>,
) -> Result<BTreeSet<String>, LearningError> {
    let mut ids = BTreeSet::new();
    let groups: Vec<&[String]> = match design {
        StudyDesign::Descriptive { .. } => {
            return Ok(cases.keys().map(|case_id| (*case_id).to_owned()).collect());
        }
        StudyDesign::MatchedObservational {
            treatment_case_ids,
            control_case_ids,
            ..
        } => vec![treatment_case_ids, control_case_ids],
        StudyDesign::QuasiExperimental {
            pre_case_ids,
            post_case_ids,
            ..
        } => vec![pre_case_ids, post_case_ids],
        StudyDesign::Randomized {
            treatment_case_ids,
            control_case_ids,
            ..
        } => vec![treatment_case_ids, control_case_ids],
    };
    for group in groups {
        for case_id in group {
            if !cases.contains_key(case_id.as_str()) {
                return Err(LearningError::UnknownDesignCase(case_id.clone()));
            }
            ids.insert(case_id.clone());
        }
    }
    Ok(ids)
}

fn assess_study_design(
    design: &StudyDesign,
    cases: &BTreeMap<&str, &CaseRecord>,
) -> Result<DesignAssessment, LearningError> {
    match design {
        StudyDesign::Descriptive { rationale_ref } => {
            require_non_empty("descriptive_rationale_ref", rationale_ref)?;
            ensure_roles(cases, CaseRole::Descriptive)?;
            Ok(DesignAssessment {
                evidence_class: EvidenceClass::Descriptive,
                causal_claim: CausalClaimDisposition::NotSupported,
                promotion: PromotionDisposition::LocalDescriptiveOnly,
                quarantined: false,
            })
        }
        StudyDesign::MatchedObservational {
            treatment_case_ids,
            control_case_ids,
            matching_variable_ids,
            balance_report_ref,
            balance_disposition,
        } => {
            ensure_non_empty_group(treatment_case_ids, "treatment_case_ids")?;
            ensure_non_empty_group(control_case_ids, "control_case_ids")?;
            require_non_empty("balance_report_ref", balance_report_ref)?;
            if matching_variable_ids.is_empty() {
                return Err(LearningError::EmptyMatchingVariables);
            }
            let mut matching = BTreeSet::new();
            for variable in matching_variable_ids {
                require_non_empty("matching_variable_id", variable)?;
                if !matching.insert(variable.as_str()) {
                    return Err(LearningError::DuplicateMatchingVariable(variable.clone()));
                }
            }
            validate_case_group(treatment_case_ids, cases, CaseRole::Treatment)?;
            validate_case_group(control_case_ids, cases, CaseRole::Control)?;
            ensure_disjoint_groups(treatment_case_ids, control_case_ids)?;
            let quarantined = *balance_disposition == DiagnosticDisposition::Failed;
            let promotion = match balance_disposition {
                DiagnosticDisposition::Passed => {
                    PromotionDisposition::MatchedReferenceCandidate
                }
                DiagnosticDisposition::Failed => PromotionDisposition::Quarantined,
                DiagnosticDisposition::NotEvaluated => {
                    PromotionDisposition::LocalDescriptiveOnly
                }
            };
            Ok(DesignAssessment {
                evidence_class: EvidenceClass::MatchedObservational,
                causal_claim: CausalClaimDisposition::NotSupported,
                promotion,
                quarantined,
            })
        }
        StudyDesign::QuasiExperimental {
            method: _,
            pre_case_ids,
            post_case_ids,
            assignment_unit: _,
            preregistration_ref,
            intervention_ref,
            identification_assumptions,
            diagnostics,
        } => {
            ensure_non_empty_group(pre_case_ids, "pre_case_ids")?;
            ensure_non_empty_group(post_case_ids, "post_case_ids")?;
            validate_case_group(pre_case_ids, cases, CaseRole::PreIntervention)?;
            validate_case_group(post_case_ids, cases, CaseRole::PostIntervention)?;
            ensure_disjoint_groups(pre_case_ids, post_case_ids)?;
            require_non_empty("preregistration_ref", preregistration_ref)?;
            require_non_empty("intervention_ref", intervention_ref)?;
            if identification_assumptions.is_empty() {
                return Err(LearningError::EmptyIdentificationAssumptions);
            }
            for assumption in identification_assumptions {
                require_non_empty("identification_assumption", assumption)?;
            }
            let (quarantined, complete) = validate_required_diagnostics(
                diagnostics,
                &[
                    DiagnosticKind::PreInterventionTrend,
                    DiagnosticKind::PlaceboOutcome,
                    DiagnosticKind::ShuffledTreatment,
                    DiagnosticKind::SensitivityAnalysis,
                    DiagnosticKind::CommonCauseReview,
                ],
            )?;
            Ok(DesignAssessment {
                evidence_class: EvidenceClass::QuasiExperimental,
                causal_claim: if quarantined {
                    CausalClaimDisposition::Rejected
                } else if complete {
                    CausalClaimDisposition::ConditionalOnQuasiExperimentalAssumptions
                } else {
                    CausalClaimDisposition::Rejected
                },
                promotion: if quarantined {
                    PromotionDisposition::Quarantined
                } else if complete {
                    PromotionDisposition::QuasiExperimentalClaimEligible
                } else {
                    PromotionDisposition::LocalDescriptiveOnly
                },
                quarantined,
            })
        }
        StudyDesign::Randomized {
            treatment_case_ids,
            control_case_ids,
            assignment_unit: _,
            preregistration_ref,
            allocation_protocol_ref,
            allocation_record_ref,
            intent_to_treat_ref,
            diagnostics,
        } => {
            ensure_non_empty_group(treatment_case_ids, "treatment_case_ids")?;
            ensure_non_empty_group(control_case_ids, "control_case_ids")?;
            validate_case_group(treatment_case_ids, cases, CaseRole::Treatment)?;
            validate_case_group(control_case_ids, cases, CaseRole::Control)?;
            ensure_disjoint_groups(treatment_case_ids, control_case_ids)?;
            require_non_empty("preregistration_ref", preregistration_ref)?;
            require_non_empty("allocation_protocol_ref", allocation_protocol_ref)?;
            require_non_empty("allocation_record_ref", allocation_record_ref)?;
            require_non_empty("intent_to_treat_ref", intent_to_treat_ref)?;
            let (quarantined, complete) = validate_required_diagnostics(
                diagnostics,
                &[
                    DiagnosticKind::RandomizationIntegrity,
                    DiagnosticKind::ContaminationAssessment,
                    DiagnosticKind::IntentToTreatAnalysis,
                    DiagnosticKind::ClusterCorrectedAnalysis,
                    DiagnosticKind::CommonCauseReview,
                ],
            )?;
            Ok(DesignAssessment {
                evidence_class: EvidenceClass::Randomized,
                causal_claim: if quarantined {
                    CausalClaimDisposition::Rejected
                } else if complete {
                    CausalClaimDisposition::RandomizedDesignEligible
                } else {
                    CausalClaimDisposition::Rejected
                },
                promotion: if quarantined {
                    PromotionDisposition::Quarantined
                } else if complete {
                    PromotionDisposition::RandomizedClaimEligible
                } else {
                    PromotionDisposition::LocalDescriptiveOnly
                },
                quarantined,
            })
        }
    }
}

fn validate_required_diagnostics(
    diagnostics: &[FalsificationDiagnostic],
    required: &[DiagnosticKind],
) -> Result<(bool, bool), LearningError> {
    let mut by_kind = BTreeMap::new();
    for diagnostic in diagnostics {
        diagnostic.validate()?;
        if by_kind.insert(diagnostic.kind, diagnostic.disposition).is_some() {
            return Err(LearningError::DuplicateDiagnostic(diagnostic.kind));
        }
    }

    let mut failed = false;
    let mut complete = true;
    for required_kind in required {
        match by_kind.get(required_kind) {
            None => {
                complete = false;
            }
            Some(DiagnosticDisposition::Passed) => {}
            Some(DiagnosticDisposition::Failed) => {
                failed = true;
            }
            Some(DiagnosticDisposition::NotEvaluated) => {
                complete = false;
            }
        }
    }
    Ok((failed, complete))
}

fn validate_case_group(
    case_ids: &[String],
    cases: &BTreeMap<&str, &CaseRecord>,
    expected_role: CaseRole,
) -> Result<(), LearningError> {
    let mut seen = BTreeSet::new();
    for case_id in case_ids {
        require_non_empty("design_case_id", case_id)?;
        if !seen.insert(case_id.as_str()) {
            return Err(LearningError::DuplicateDesignCase(case_id.clone()));
        }
        let case = cases
            .get(case_id.as_str())
            .ok_or_else(|| LearningError::UnknownDesignCase(case_id.clone()))?;
        if case.role != expected_role {
            return Err(LearningError::InvalidRoleForDesign(case_id.clone()));
        }
    }
    Ok(())
}

fn ensure_roles(
    cases: &BTreeMap<&str, &CaseRecord>,
    expected_role: CaseRole,
) -> Result<(), LearningError> {
    if cases.values().all(|case| case.role == expected_role) {
        Ok(())
    } else {
        Err(LearningError::InvalidRoleForDesign(
            "descriptive-study-case-role".to_owned(),
        ))
    }
}

fn ensure_disjoint_groups(left: &[String], right: &[String]) -> Result<(), LearningError> {
    let left_ids: BTreeSet<&str> = left.iter().map(String::as_str).collect();
    if let Some(duplicate) = right
        .iter()
        .map(String::as_str)
        .find(|case_id| left_ids.contains(case_id))
    {
        return Err(LearningError::DuplicateDesignCase(duplicate.to_owned()));
    }
    Ok(())
}

fn ensure_non_empty_group(values: &[String], name: &'static str) -> Result<(), LearningError> {
    if values.is_empty() {
        return Err(LearningError::MissingDesignEvidence(name));
    }
    Ok(())
}

fn checked_add(left: u64, right: u64) -> Result<u64, LearningError> {
    left.checked_add(right).ok_or(LearningError::ArithmeticOverflow)
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), LearningError> {
    if value.trim().is_empty() {
        Err(LearningError::EmptyField(name))
    } else {
        Ok(())
    }
}

fn validate_currency_code(value: &str) -> Result<(), LearningError> {
    if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(LearningError::InvalidCurrencyCode(value.to_owned()));
    }
    Ok(())
}

impl std::fmt::Display for LearningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> FunctionalUnitBinding {
        FunctionalUnitBinding {
            contract_id: "accepted-panel-m2-v1".to_owned(),
            contract_digest: "git-blob-sha1:accepted-panel-contract".to_owned(),
            unit_class_id: "non-loadbearing-panel-area".to_owned(),
            unit_symbol: "m2".to_owned(),
            acceptance_contract_digest: "git-blob-sha1:panel-acceptance".to_owned(),
            currency_code: "USD".to_owned(),
            assessment_horizon_months: 240,
        }
    }

    fn context(project: &str, site: &str) -> ProjectContext {
        ProjectContext {
            project_id: project.to_owned(),
            site_id: site.to_owned(),
            floor_or_zone_id: "floor-01".to_owned(),
            work_package_id: "panel-installation".to_owned(),
            task_family_id: "non-loadbearing-panel-install".to_owned(),
            design_revision_digest: "git-blob-sha1:design-v1".to_owned(),
            method_profile_digest: "git-blob-sha1:method-v1".to_owned(),
            fleet_profile_digest: "git-blob-sha1:fleet-v1".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
            attributes: BTreeMap::from([
                ("material_family".to_owned(), "panel-type-a".to_owned()),
                ("access_regime".to_owned(), "ground-level-guarded".to_owned()),
            ]),
        }
    }

    fn case(id: &str, project: &str, site: &str, role: CaseRole) -> CaseRecord {
        CaseRecord {
            case_id: id.to_owned(),
            case_digest: format!("git-blob-sha1:case-{id}"),
            role,
            functional_unit: unit(),
            context: context(project, site),
            outcome: OutcomeAccounting {
                planned_units: 10,
                attempted_units: 10,
                first_pass_inspected_units: 10,
                first_pass_accepted_units: 9,
                finally_accepted_units: 9,
                finally_rejected_units: 1,
                unresolved_units: 0,
                rework_attempts: 1,
                total_workflow_crew_minutes: 500,
                elapsed_minutes: 600,
                direct_cost_minor_units: 10_000,
                shared_cost_minor_units: 1_000,
                energy_wh: 50_000,
                carbon_gco2e: 20_000,
            },
            source_evidence: vec![SourceEvidence {
                evidence_id: format!("evidence-{id}"),
                artifact_ref: format!("trial-receipt:{id}"),
                content_digest: format!("git-blob-sha1:receipt-{id}"),
                supported_claim_ref: "claim:accepted-panel-output".to_owned(),
                status: SourceEvidenceStatus::AdmissibleForDeclaredClaim,
            }],
        }
    }

    fn envelope() -> ApplicabilityEnvelope {
        ApplicabilityEnvelope {
            envelope_id: "panel-ground-level-v1".to_owned(),
            dimensions: BTreeMap::from([
                (
                    "material_family".to_owned(),
                    BTreeSet::from(["panel-type-a".to_owned()]),
                ),
                (
                    "access_regime".to_owned(),
                    BTreeSet::from(["ground-level-guarded".to_owned()]),
                ),
            ]),
            excluded_values: BTreeMap::from([(
                "access_regime".to_owned(),
                BTreeSet::from(["occupied-high-rise-façade".to_owned()]),
            )]),
        }
    }

    fn assignments(cases: &[CaseRecord]) -> Vec<PartitionAssignment> {
        cases
            .iter()
            .map(|case| PartitionAssignment {
                case_id: case.case_id.clone(),
                partition: DataPartition::Training,
            })
            .collect()
    }

    fn matched_manifest() -> ComparisonManifest {
        let cases = vec![
            case("treated-1", "project-a", "site-a", CaseRole::Treatment),
            case("control-1", "project-b", "site-b", CaseRole::Control),
        ];
        ComparisonManifest {
            study_id: "matched-study-001".to_owned(),
            study_digest: "git-blob-sha1:study-001".to_owned(),
            functional_unit: unit(),
            applicability: envelope(),
            partition_assignments: assignments(&cases),
            cases,
            study_design: StudyDesign::MatchedObservational {
                treatment_case_ids: vec!["treated-1".to_owned()],
                control_case_ids: vec!["control-1".to_owned()],
                matching_variable_ids: vec![
                    "panel-geometry-class".to_owned(),
                    "access-regime".to_owned(),
                ],
                balance_report_ref: "balance-report:matched-001".to_owned(),
                balance_disposition: DiagnosticDisposition::Passed,
            },
            effects: vec![EffectEstimate {
                functional_contract_digest: unit().contract_digest,
                comparison_case_ids: vec!["treated-1".to_owned(), "control-1".to_owned()],
                metric_id: "total-workflow-crew-minutes-per-accepted-m2".to_owned(),
                unit_symbol: "crew-min/m2".to_owned(),
                scale: 1,
                estimate: -25,
                lower_bound: -40,
                upper_bound: -10,
                estimate_method_ref: "estimator:matched-difference-v1".to_owned(),
                interval_method_ref: "interval:cluster-bootstrap-v1".to_owned(),
                source_report_ref: "report:matched-study-001".to_owned(),
                observation_count: 20,
                independent_cluster_count: 4,
            }],
            replication: None,
        }
    }

    fn diagnostics(kinds: &[DiagnosticKind]) -> Vec<FalsificationDiagnostic> {
        kinds
            .iter()
            .map(|kind| FalsificationDiagnostic {
                kind: *kind,
                disposition: DiagnosticDisposition::Passed,
                evidence_ref: format!("diagnostic:{kind:?}"),
            })
            .collect()
    }

    #[test]
    fn matched_observational_comparison_is_not_promoted_to_causal_claim() {
        let report = evaluate_comparison(&matched_manifest()).expect("valid matched study");
        assert_eq!(report.evidence_class, EvidenceClass::MatchedObservational);
        assert_eq!(report.causal_claim, CausalClaimDisposition::NotSupported);
        assert_eq!(report.promotion, PromotionDisposition::MatchedReferenceCandidate);
    }

    #[test]
    fn matched_balance_failure_quarantines_reference_candidate() {
        let mut manifest = matched_manifest();
        let StudyDesign::MatchedObservational {
            balance_disposition, ..
        } = &mut manifest.study_design
        else {
            panic!("matched design");
        };
        *balance_disposition = DiagnosticDisposition::Failed;
        let report = evaluate_comparison(&manifest).expect("failed balance is reported");
        assert_eq!(report.promotion, PromotionDisposition::Quarantined);
        assert_eq!(report.causal_claim, CausalClaimDisposition::Rejected);
    }

    #[test]
    fn outcome_denominator_must_reconcile_every_attempted_unit() {
        let mut manifest = matched_manifest();
        manifest.cases[0].outcome.finally_accepted_units = 8;
        assert_eq!(
            evaluate_comparison(&manifest),
            Err(LearningError::OutcomeDenominatorMismatch {
                attempted: 10,
                reconciled: 9,
            })
        );
    }

    #[test]
    fn matched_cases_must_use_the_same_functional_unit_contract() {
        let mut manifest = matched_manifest();
        manifest.cases[1].functional_unit.unit_class_id = "different-output".to_owned();
        assert_eq!(
            evaluate_comparison(&manifest),
            Err(LearningError::FunctionalUnitMismatch("control-1".to_owned()))
        );
    }

    #[test]
    fn project_or_site_leakage_across_holdout_partitions_is_rejected() {
        let mut manifest = matched_manifest();
        manifest.partition_assignments[0].partition = DataPartition::Training;
        manifest.partition_assignments[1].partition = DataPartition::BlindChallenge;
        manifest.cases[1].context.project_id = manifest.cases[0].context.project_id.clone();
        assert_eq!(
            evaluate_comparison(&manifest),
            Err(LearningError::ProjectLeakage("project-a".to_owned()))
        );
    }

    #[test]
    fn quasi_experimental_claim_requires_all_falsification_diagnostics() {
        let cases = vec![
            case("pre-1", "project-a", "site-a", CaseRole::PreIntervention),
            case("post-1", "project-b", "site-b", CaseRole::PostIntervention),
        ];
        let manifest = ComparisonManifest {
            study_id: "did-study-001".to_owned(),
            study_digest: "git-blob-sha1:did-study".to_owned(),
            functional_unit: unit(),
            applicability: envelope(),
            partition_assignments: assignments(&cases),
            cases,
            study_design: StudyDesign::QuasiExperimental {
                method: QuasiExperimentalMethod::DifferenceInDifferences,
                pre_case_ids: vec!["pre-1".to_owned()],
                post_case_ids: vec!["post-1".to_owned()],
                assignment_unit: AssignmentUnit::Site,
                preregistration_ref: "prereg:did-001".to_owned(),
                intervention_ref: "intervention:early-survey-vs-deferred".to_owned(),
                identification_assumptions: vec![
                    "parallel pre-intervention trends".to_owned(),
                    "no unmeasured time-varying confounding".to_owned(),
                ],
                diagnostics: diagnostics(&[
                    DiagnosticKind::PreInterventionTrend,
                    DiagnosticKind::PlaceboOutcome,
                    DiagnosticKind::ShuffledTreatment,
                    DiagnosticKind::SensitivityAnalysis,
                    DiagnosticKind::CommonCauseReview,
                ]),
            },
            effects: Vec::new(),
            replication: None,
        };
        let report = evaluate_comparison(&manifest).expect("valid DID design record");
        assert_eq!(
            report.evidence_class,
            EvidenceClass::QuasiExperimental
        );
        assert_eq!(
            report.causal_claim,
            CausalClaimDisposition::ConditionalOnQuasiExperimentalAssumptions
        );
        assert_eq!(
            report.promotion,
            PromotionDisposition::QuasiExperimentalClaimEligible
        );
    }

    #[test]
    fn failed_pretrend_or_placebo_diagnostic_quarantines_quasi_experimental_claim() {
        let mut manifest = quasi_manifest();
        let StudyDesign::QuasiExperimental { diagnostics, .. } =
            &mut manifest.study_design
        else {
            panic!("quasi-experimental design");
        };
        diagnostics[0].disposition = DiagnosticDisposition::Failed;
        let report = evaluate_comparison(&manifest).expect("failed diagnostic is a report");
        assert_eq!(report.promotion, PromotionDisposition::Quarantined);
        assert_eq!(report.causal_claim, CausalClaimDisposition::Rejected);
    }

    fn quasi_manifest() -> ComparisonManifest {
        let cases = vec![
            case("pre-1", "project-a", "site-a", CaseRole::PreIntervention),
            case("post-1", "project-b", "site-b", CaseRole::PostIntervention),
        ];
        ComparisonManifest {
            study_id: "did-study-002".to_owned(),
            study_digest: "git-blob-sha1:did-study-002".to_owned(),
            functional_unit: unit(),
            applicability: envelope(),
            partition_assignments: assignments(&cases),
            cases,
            study_design: StudyDesign::QuasiExperimental {
                method: QuasiExperimentalMethod::DifferenceInDifferences,
                pre_case_ids: vec!["pre-1".to_owned()],
                post_case_ids: vec!["post-1".to_owned()],
                assignment_unit: AssignmentUnit::Site,
                preregistration_ref: "prereg:did-002".to_owned(),
                intervention_ref: "intervention:method-switch".to_owned(),
                identification_assumptions: vec!["parallel pre-trends".to_owned()],
                diagnostics: diagnostics(&[
                    DiagnosticKind::PreInterventionTrend,
                    DiagnosticKind::PlaceboOutcome,
                    DiagnosticKind::ShuffledTreatment,
                    DiagnosticKind::SensitivityAnalysis,
                    DiagnosticKind::CommonCauseReview,
                ]),
            },
            effects: Vec::new(),
            replication: None,
        }
    }

    #[test]
    fn out_of_envelope_context_is_not_admitted_for_reference_class() {
        let mut manifest = matched_manifest();
        manifest.cases[0]
            .context
            .attributes
            .insert(
                "access_regime".to_owned(),
                "occupied-high-rise-façade".to_owned(),
            );
        assert_eq!(
            evaluate_comparison(&manifest),
            Err(LearningError::EffectContextMismatch("treated-1".to_owned()))
        );
    }

    #[test]
    fn unknown_source_evidence_stays_quarantined() {
        let mut manifest = matched_manifest();
        manifest.cases[0].source_evidence[0].status = SourceEvidenceStatus::Unknown;
        let report = evaluate_comparison(&manifest).expect("unknown source is quarantined");
        assert_eq!(report.promotion, PromotionDisposition::Quarantined);
        assert_eq!(report.causal_claim, CausalClaimDisposition::Rejected);
    }

    #[test]
    fn independent_replication_requires_two_sites_projects_and_assessors() {
        let mut manifest = matched_manifest();
        manifest.replication = Some(ReplicationEvidence {
            replication_protocol_ref: "replication-protocol:v1".to_owned(),
            included_case_ids: vec!["treated-1".to_owned(), "control-1".to_owned()],
            independent_assessor_refs: vec![
                "assessor:independent-a".to_owned(),
                "assessor:independent-b".to_owned(),
            ],
        });
        let report = evaluate_comparison(&manifest).expect("independent replication");
        assert_eq!(
            report.replication,
            ReplicationDisposition::IndependentCrossSiteReplication
        );
        assert_eq!(
            report.promotion,
            PromotionDisposition::IndependentlyReplicatedDescriptiveReference
        );
        assert_eq!(
            report.causal_claim,
            CausalClaimDisposition::NotSupported
        );
    }

    #[test]
    fn effect_interval_and_cluster_counts_are_validated() {
        let mut manifest = matched_manifest();
        manifest.effects[0].lower_bound = -5;
        assert_eq!(
            evaluate_comparison(&manifest),
            Err(LearningError::InvalidEffectInterval(
                "total-workflow-crew-minutes-per-accepted-m2".to_owned()
            ))
        );

        let mut second = matched_manifest();
        second.effects[0].independent_cluster_count = 30;
        assert_eq!(
            evaluate_comparison(&second),
            Err(LearningError::InvalidClusterCount(
                "total-workflow-crew-minutes-per-accepted-m2".to_owned()
            ))
        );
    }

    #[test]
    fn future_context_outside_reference_class_returns_out_of_domain() {
        let mut manifest = matched_manifest();
        manifest.applicability.excluded_values.clear();
        let _ = evaluate_comparison(&manifest).expect("baseline comparison");
        let mut future = context("future-project", "future-site");
        future
            .attributes
            .insert("access_regime".to_owned(), "occupied-high-rise-façade".to_owned());
        assert_eq!(
            manifest.applicability.evaluate(&future),
            ApplicabilityDisposition::OutOfDomain {
                dimensions: vec!["access_regime".to_owned()]
            }
        );
    }
}
