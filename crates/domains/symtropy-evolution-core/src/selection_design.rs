use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisMethodId, CalibrationAuthorityId,
    ComparisonAuthorityId, EvidenceProtocolContentDigest, EvidenceProtocolId, EvolutionIndividualId,
    EvolutionaryContextRefDigest, ExplicitLinkedPopulationCensusDigest,
    ExplicitSelectionEvidenceLedgerDigest, ExposureEvidenceStatus, ExclusionReasonId,
    PhenotypeEvidenceStatus, PopulationId, PredictorDefinitionId, SelectionComparisonDesignId,
    SelectionEvidenceRecord, ValidatedSelectionEvidenceLedger,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};

pub const SELECTION_COMPARISON_DESIGN_VERSION: u32 = 1;

const SELECTION_COMPARISON_DESIGN_DOMAIN: &[u8] =
    b"symtropy:evolution:selection-comparison-design:v1\0";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnalysisContentDigest([u8; 32]);

impl AnalysisContentDigest {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AnalysisContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnalysisContentDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for AnalysisContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisAuthorityRef {
    pub method_id: AnalysisMethodId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
}

impl AnalysisAuthorityRef {
    pub fn new(
        method_id: AnalysisMethodId,
        revision: u64,
        content_digest: AnalysisContentDigest,
    ) -> Self {
        Self {
            method_id,
            revision,
            content_digest,
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.method_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonAuthorityRef {
    pub authority_id: ComparisonAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
}

impl ComparisonAuthorityRef {
    pub fn new(
        authority_id: ComparisonAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationAuthorityRef {
    pub authority_id: CalibrationAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
}

impl CalibrationAuthorityRef {
    pub fn new(
        authority_id: CalibrationAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictorSourceKind {
    Phenotype,
    Exposure,
    GenotypeOrLineage,
    ExternalOpaque,
}

impl PredictorSourceKind {
    fn tag(self) -> u8 {
        match self {
            Self::Phenotype => 0,
            Self::Exposure => 1,
            Self::GenotypeOrLineage => 2,
            Self::ExternalOpaque => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictorDefinitionRef {
    pub predictor_id: PredictorDefinitionId,
    pub source_kind: PredictorSourceKind,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
}

impl PredictorDefinitionRef {
    pub fn new(
        predictor_id: PredictorDefinitionId,
        source_kind: PredictorSourceKind,
        revision: u64,
        content_digest: AnalysisContentDigest,
    ) -> Self {
        Self {
            predictor_id,
            source_kind,
            revision,
            content_digest,
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.predictor_id.as_str());
        digest.update([self.source_kind.tag()]);
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionEstimand {
    ViabilityWindowRiskContrast,
    ReproductiveEventOpportunityConditionedContrast,
    DescendantProductionContrast,
    DescendantRecruitmentContrast,
}

impl SelectionEstimand {
    fn tag(self) -> u8 {
        match self {
            Self::ViabilityWindowRiskContrast => 0,
            Self::ReproductiveEventOpportunityConditionedContrast => 1,
            Self::DescendantProductionContrast => 2,
            Self::DescendantRecruitmentContrast => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionDesignClass {
    DescriptiveAssociation,
    RandomizedInterventional,
    ControlledSimulationIntervention,
    MatchedStratifiedObservational,
    WithinFamilyOrLineageControlled,
    CommonEnvironment,
    ReciprocalContext,
    DeclaredNeutralNullComparison,
}

impl SelectionDesignClass {
    fn tag(self) -> u8 {
        match self {
            Self::DescriptiveAssociation => 0,
            Self::RandomizedInterventional => 1,
            Self::ControlledSimulationIntervention => 2,
            Self::MatchedStratifiedObservational => 3,
            Self::WithinFamilyOrLineageControlled => 4,
            Self::CommonEnvironment => 5,
            Self::ReciprocalContext => 6,
            Self::DeclaredNeutralNullComparison => 7,
        }
    }

    fn requires_comparison_authority(self) -> bool {
        self != Self::DescriptiveAssociation
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelSupportPolicy {
    NotRequired,
    CompleteOnly,
    PartialAllowed { method: AnalysisAuthorityRef },
    UnavailableAllowed { method: AnalysisAuthorityRef },
}

impl ChannelSupportPolicy {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::NotRequired => digest.update([0]),
            Self::CompleteOnly => digest.update([1]),
            Self::PartialAllowed { method } => {
                digest.update([2]);
                method.update_digest(digest);
            }
            Self::UnavailableAllowed { method } => {
                digest.update([3]);
                method.update_digest(digest);
            }
        }
    }

    fn accepts_phenotype(&self, status: &PhenotypeEvidenceStatus) -> bool {
        match self {
            Self::NotRequired | Self::UnavailableAllowed { .. } => true,
            Self::CompleteOnly => matches!(status, PhenotypeEvidenceStatus::CompleteWindow(_)),
            Self::PartialAllowed { .. } => !matches!(status, PhenotypeEvidenceStatus::Unavailable),
        }
    }

    fn accepts_exposure(&self, status: &ExposureEvidenceStatus) -> bool {
        match self {
            Self::NotRequired | Self::UnavailableAllowed { .. } => true,
            Self::CompleteOnly => matches!(status, ExposureEvidenceStatus::CompleteWindow(_)),
            Self::PartialAllowed { .. } => !matches!(status, ExposureEvidenceStatus::Unavailable),
        }
    }

    fn is_required(&self) -> bool {
        !matches!(self, Self::NotRequired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolComparabilityPolicy {
    ExactIdentityOnly,
    Calibrated { authority: CalibrationAuthorityRef },
}

impl ProtocolComparabilityPolicy {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::ExactIdentityOnly => digest.update([0]),
            Self::Calibrated { authority } => {
                digest.update([1]);
                authority.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusionDeclaration {
    pub individual_id: EvolutionIndividualId,
    pub reason_id: ExclusionReasonId,
}

impl ExclusionDeclaration {
    pub fn new(individual_id: EvolutionIndividualId, reason_id: ExclusionReasonId) -> Self {
        Self {
            individual_id,
            reason_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfoundingPolicy {
    DescriptiveNoCausalClaim,
    DesignBasedNoAdjustmentDeclared,
    DeclaredAdjustment { authority: AnalysisAuthorityRef },
}

impl ConfoundingPolicy {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::DescriptiveNoCausalClaim => digest.update([0]),
            Self::DesignBasedNoAdjustmentDeclared => digest.update([1]),
            Self::DeclaredAdjustment { authority } => {
                digest.update([2]);
                authority.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathBeforeEndpointPolicy {
    NotApplicable,
    EndpointFailure,
    Censored,
    CompetingRisk,
}

impl DeathBeforeEndpointPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::NotApplicable => 0,
            Self::EndpointFailure => 1,
            Self::Censored => 2,
            Self::CompetingRisk => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompetingRiskPolicy {
    NoneDeclared,
    Declared { authority: AnalysisAuthorityRef },
}

impl CompetingRiskPolicy {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::NoneDeclared => digest.update([0]),
            Self::Declared { authority } => {
                digest.update([1]);
                authority.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsufficientSupportPolicy {
    FailClosed,
    ReportInsufficientSupport,
    DescriptiveOnlyFallback,
}

impl InsufficientSupportPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::ReportInsufficientSupport => 1,
            Self::DescriptiveOnlyFallback => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UncertaintyPlan {
    pub method: AnalysisAuthorityRef,
    pub replicate_count: Option<u64>,
    pub seed_lineage_digest: Option<AnalysisContentDigest>,
    pub multiplicity: Option<AnalysisAuthorityRef>,
    pub minimum_information: AnalysisAuthorityRef,
    pub insufficient_support: InsufficientSupportPolicy,
}

impl UncertaintyPlan {
    pub fn validate(&self) -> Result<(), SelectionDesignError> {
        if self.replicate_count == Some(0) {
            return Err(SelectionDesignError::ZeroReplicateCount);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        self.method.update_digest(digest);
        match self.replicate_count {
            None => digest.update([0]),
            Some(value) => {
                digest.update([1]);
                put_u64(digest, value);
            }
        }
        match self.seed_lineage_digest {
            None => digest.update([0]),
            Some(value) => {
                digest.update([1]);
                digest.update(value.as_bytes());
            }
        }
        match &self.multiplicity {
            None => digest.update([0]),
            Some(value) => {
                digest.update([1]);
                value.update_digest(digest);
            }
        }
        self.minimum_information.update_digest(digest);
        digest.update([self.insufficient_support.tag()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionComparisonDesign {
    design_version: u32,
    pub design_id: SelectionComparisonDesignId,
    selection_evidence_ledger_digest: ExplicitSelectionEvidenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
    pub predictor: PredictorDefinitionRef,
    pub estimand: SelectionEstimand,
    pub design_class: SelectionDesignClass,
    pub comparison_authority: Option<ComparisonAuthorityRef>,
    pub included_individuals: Vec<EvolutionIndividualId>,
    pub exclusions: Vec<ExclusionDeclaration>,
    pub eligibility_rule: AnalysisAuthorityRef,
    pub phenotype_support: ChannelSupportPolicy,
    pub exposure_support: ChannelSupportPolicy,
    pub phenotype_protocols: ProtocolComparabilityPolicy,
    pub exposure_protocols: ProtocolComparabilityPolicy,
    pub confounding: ConfoundingPolicy,
    pub death_before_endpoint: DeathBeforeEndpointPolicy,
    pub competing_risk: CompetingRiskPolicy,
    pub uncertainty: UncertaintyPlan,
}

impl SelectionComparisonDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        design_id: SelectionComparisonDesignId,
        validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
        predictor: PredictorDefinitionRef,
        estimand: SelectionEstimand,
        design_class: SelectionDesignClass,
        comparison_authority: Option<ComparisonAuthorityRef>,
        included_individuals: impl IntoIterator<Item = EvolutionIndividualId>,
        exclusions: impl IntoIterator<Item = ExclusionDeclaration>,
        eligibility_rule: AnalysisAuthorityRef,
        phenotype_support: ChannelSupportPolicy,
        exposure_support: ChannelSupportPolicy,
        phenotype_protocols: ProtocolComparabilityPolicy,
        exposure_protocols: ProtocolComparabilityPolicy,
        confounding: ConfoundingPolicy,
        death_before_endpoint: DeathBeforeEndpointPolicy,
        competing_risk: CompetingRiskPolicy,
        uncertainty: UncertaintyPlan,
    ) -> Result<Self, SelectionDesignError> {
        uncertainty.validate()?;
        if design_class.requires_comparison_authority() && comparison_authority.is_none() {
            return Err(SelectionDesignError::MissingComparisonAuthority);
        }
        if design_class == SelectionDesignClass::DescriptiveAssociation
            && confounding != ConfoundingPolicy::DescriptiveNoCausalClaim
        {
            return Err(SelectionDesignError::DescriptiveDesignClaimsCausalAdjustment);
        }
        validate_endpoint_policies(estimand, death_before_endpoint, &competing_risk)?;

        let records = &validated_evidence.ledger().records;
        let record_ids: BTreeSet<_> = records
            .iter()
            .map(|record| record.individual_id.clone())
            .collect();

        let mut included: BTreeSet<EvolutionIndividualId> = BTreeSet::new();
        for id in included_individuals {
            if !record_ids.contains(&id) {
                return Err(SelectionDesignError::UnknownIndividual(id));
            }
            if !included.insert(id.clone()) {
                return Err(SelectionDesignError::DuplicateIncludedIndividual(id));
            }
        }
        if included.is_empty() {
            return Err(SelectionDesignError::EmptyEstimandPopulation);
        }

        let mut excluded: BTreeMap<EvolutionIndividualId, ExclusionDeclaration> = BTreeMap::new();
        for declaration in exclusions {
            if !record_ids.contains(&declaration.individual_id) {
                return Err(SelectionDesignError::UnknownIndividual(
                    declaration.individual_id,
                ));
            }
            if included.contains(&declaration.individual_id) {
                return Err(SelectionDesignError::IncludedAndExcluded(
                    declaration.individual_id,
                ));
            }
            let id = declaration.individual_id.clone();
            if excluded.insert(id.clone(), declaration).is_some() {
                return Err(SelectionDesignError::DuplicateExclusion(id));
            }
        }

        if included.len() + excluded.len() != record_ids.len()
            || record_ids
                .iter()
                .any(|id| !included.contains(id) && !excluded.contains_key(id))
        {
            return Err(SelectionDesignError::IncompleteDenominatorAccounting);
        }

        let records_by_id: BTreeMap<_, _> = records
            .iter()
            .map(|record| (record.individual_id.clone(), record))
            .collect();
        for id in &included {
            let record = records_by_id
                .get(id)
                .ok_or_else(|| SelectionDesignError::UnknownIndividual(id.clone()))?;
            validate_record_support(record, &phenotype_support, &exposure_support)?;
        }

        validate_protocol_policy(
            &included,
            &records_by_id,
            true,
            &phenotype_support,
            &phenotype_protocols,
        )?;
        validate_protocol_policy(
            &included,
            &records_by_id,
            false,
            &exposure_support,
            &exposure_protocols,
        )?;

        let design = Self {
            design_version: SELECTION_COMPARISON_DESIGN_VERSION,
            design_id,
            selection_evidence_ledger_digest: validated_evidence.ledger_digest(),
            population_id: validated_evidence.population_id().clone(),
            census_digest: validated_evidence.census_digest(),
            context_digest: validated_evidence.context_digest(),
            predictor,
            estimand,
            design_class,
            comparison_authority,
            included_individuals: included.into_iter().collect(),
            exclusions: excluded.into_values().collect(),
            eligibility_rule,
            phenotype_support,
            exposure_support,
            phenotype_protocols,
            exposure_protocols,
            confounding,
            death_before_endpoint,
            competing_risk,
            uncertainty,
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn validate_current(
        &self,
        validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
    ) -> Result<(), SelectionDesignError> {
        self.validate_local()?;
        let recomputed = Self::declare(
            self.design_id.clone(),
            validated_evidence,
            self.predictor.clone(),
            self.estimand,
            self.design_class,
            self.comparison_authority.clone(),
            self.included_individuals.clone(),
            self.exclusions.clone(),
            self.eligibility_rule.clone(),
            self.phenotype_support.clone(),
            self.exposure_support.clone(),
            self.phenotype_protocols.clone(),
            self.exposure_protocols.clone(),
            self.confounding.clone(),
            self.death_before_endpoint,
            self.competing_risk.clone(),
            self.uncertainty.clone(),
        )?;
        if recomputed != *self {
            return Err(SelectionDesignError::DesignReplayMismatch);
        }
        Ok(())
    }

    pub fn original_denominator(&self) -> u64 {
        (self.included_individuals.len() + self.exclusions.len()) as u64
    }

    pub fn estimand_denominator(&self) -> u64 {
        self.included_individuals.len() as u64
    }

    pub fn selection_evidence_ledger_digest(&self) -> ExplicitSelectionEvidenceLedgerDigest {
        self.selection_evidence_ledger_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<SelectionComparisonDesignDigest, SelectionDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(SELECTION_COMPARISON_DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.design_id.as_str());
        digest.update(self.selection_evidence_ledger_digest.as_bytes());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.census_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
        self.predictor.update_digest(&mut digest);
        digest.update([self.estimand.tag(), self.design_class.tag()]);
        match &self.comparison_authority {
            None => digest.update([0]),
            Some(authority) => {
                digest.update([1]);
                authority.update_digest(&mut digest);
            }
        }
        put_u64(&mut digest, self.included_individuals.len() as u64);
        for id in &self.included_individuals {
            put_text(&mut digest, id.as_str());
        }
        put_u64(&mut digest, self.exclusions.len() as u64);
        for exclusion in &self.exclusions {
            put_text(&mut digest, exclusion.individual_id.as_str());
            put_text(&mut digest, exclusion.reason_id.as_str());
        }
        self.eligibility_rule.update_digest(&mut digest);
        self.phenotype_support.update_digest(&mut digest);
        self.exposure_support.update_digest(&mut digest);
        self.phenotype_protocols.update_digest(&mut digest);
        self.exposure_protocols.update_digest(&mut digest);
        self.confounding.update_digest(&mut digest);
        digest.update([self.death_before_endpoint.tag()]);
        self.competing_risk.update_digest(&mut digest);
        self.uncertainty.update_digest(&mut digest);
        Ok(SelectionComparisonDesignDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), SelectionDesignError> {
        if self.design_version != SELECTION_COMPARISON_DESIGN_VERSION {
            return Err(SelectionDesignError::UnsupportedDesignVersion(
                self.design_version,
            ));
        }
        if self.included_individuals.is_empty() {
            return Err(SelectionDesignError::EmptyEstimandPopulation);
        }
        if self
            .included_individuals
            .windows(2)
            .any(|window| window[0] >= window[1])
        {
            return Err(SelectionDesignError::NonCanonicalIncludedOrder);
        }
        if self
            .exclusions
            .windows(2)
            .any(|window| window[0].individual_id >= window[1].individual_id)
        {
            return Err(SelectionDesignError::NonCanonicalExclusionOrder);
        }
        let included: BTreeSet<_> = self.included_individuals.iter().cloned().collect();
        if self
            .exclusions
            .iter()
            .any(|exclusion| included.contains(&exclusion.individual_id))
        {
            return Err(SelectionDesignError::LocalDenominatorOverlap);
        }
        if self.design_class.requires_comparison_authority()
            && self.comparison_authority.is_none()
        {
            return Err(SelectionDesignError::MissingComparisonAuthority);
        }
        if self.design_class == SelectionDesignClass::DescriptiveAssociation
            && self.confounding != ConfoundingPolicy::DescriptiveNoCausalClaim
        {
            return Err(SelectionDesignError::DescriptiveDesignClaimsCausalAdjustment);
        }
        validate_endpoint_policies(
            self.estimand,
            self.death_before_endpoint,
            &self.competing_risk,
        )?;
        self.uncertainty.validate()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SelectionComparisonDesignDigest([u8; 32]);

impl SelectionComparisonDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SelectionComparisonDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SelectionComparisonDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SelectionComparisonDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated selection design authority should be consumed by analysis execution APIs"]
pub struct ValidatedSelectionComparisonDesign<'a> {
    design: &'a SelectionComparisonDesign,
    design_digest: SelectionComparisonDesignDigest,
    selection_evidence_ledger_digest: ExplicitSelectionEvidenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
}

impl<'a> ValidatedSelectionComparisonDesign<'a> {
    pub fn validate_current(
        design: &'a SelectionComparisonDesign,
        validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
    ) -> Result<Self, SelectionDesignError> {
        design.validate_current(validated_evidence)?;
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
            selection_evidence_ledger_digest: validated_evidence.ledger_digest(),
            population_id: validated_evidence.population_id().clone(),
            census_digest: validated_evidence.census_digest(),
            context_digest: validated_evidence.context_digest(),
        })
    }

    pub fn design(&self) -> &'a SelectionComparisonDesign {
        self.design
    }

    pub fn design_digest(&self) -> SelectionComparisonDesignDigest {
        self.design_digest
    }

    pub fn selection_evidence_ledger_digest(&self) -> ExplicitSelectionEvidenceLedgerDigest {
        self.selection_evidence_ledger_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn census_digest(&self) -> ExplicitLinkedPopulationCensusDigest {
        self.census_digest
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }
}

fn validate_record_support(
    record: &SelectionEvidenceRecord,
    phenotype_support: &ChannelSupportPolicy,
    exposure_support: &ChannelSupportPolicy,
) -> Result<(), SelectionDesignError> {
    if !phenotype_support.accepts_phenotype(&record.phenotype) {
        return Err(SelectionDesignError::PhenotypeSupportInsufficient(
            record.individual_id.clone(),
        ));
    }
    if !exposure_support.accepts_exposure(&record.exposure) {
        return Err(SelectionDesignError::ExposureSupportInsufficient(
            record.individual_id.clone(),
        ));
    }
    Ok(())
}

fn phenotype_protocol(
    status: &PhenotypeEvidenceStatus,
) -> Option<(&EvidenceProtocolId, &EvidenceProtocolContentDigest)> {
    match status {
        PhenotypeEvidenceStatus::Unavailable => None,
        PhenotypeEvidenceStatus::PartialWindow { evidence, .. }
        | PhenotypeEvidenceStatus::CompleteWindow(evidence) => {
            Some((&evidence.protocol_id, &evidence.protocol_content_digest))
        }
    }
}

fn exposure_protocol(
    status: &ExposureEvidenceStatus,
) -> Option<(&EvidenceProtocolId, &EvidenceProtocolContentDigest)> {
    match status {
        ExposureEvidenceStatus::Unavailable => None,
        ExposureEvidenceStatus::PartialWindow { evidence, .. }
        | ExposureEvidenceStatus::CompleteWindow(evidence) => {
            Some((&evidence.protocol_id, &evidence.protocol_content_digest))
        }
    }
}

fn validate_protocol_policy(
    included: &BTreeSet<EvolutionIndividualId>,
    records: &BTreeMap<EvolutionIndividualId, &SelectionEvidenceRecord>,
    phenotype: bool,
    support_policy: &ChannelSupportPolicy,
    protocol_policy: &ProtocolComparabilityPolicy,
) -> Result<(), SelectionDesignError> {
    if !support_policy.is_required()
        || matches!(protocol_policy, ProtocolComparabilityPolicy::Calibrated { .. })
    {
        return Ok(());
    }

    let mut expected: Option<(EvidenceProtocolId, EvidenceProtocolContentDigest)> = None;
    for id in included {
        let record = records
            .get(id)
            .ok_or_else(|| SelectionDesignError::UnknownIndividual(id.clone()))?;
        let protocol = if phenotype {
            phenotype_protocol(&record.phenotype)
        } else {
            exposure_protocol(&record.exposure)
        };
        let Some((protocol_id, protocol_digest)) = protocol else {
            continue;
        };
        match &expected {
            None => expected = Some((protocol_id.clone(), *protocol_digest)),
            Some((expected_id, expected_digest))
                if expected_id == protocol_id && expected_digest == protocol_digest => {}
            Some(_) => {
                return Err(if phenotype {
                    SelectionDesignError::PhenotypeProtocolMismatch
                } else {
                    SelectionDesignError::ExposureProtocolMismatch
                });
            }
        }
    }
    Ok(())
}

fn validate_endpoint_policies(
    estimand: SelectionEstimand,
    death_policy: DeathBeforeEndpointPolicy,
    competing_risk: &CompetingRiskPolicy,
) -> Result<(), SelectionDesignError> {
    if estimand == SelectionEstimand::ViabilityWindowRiskContrast {
        if death_policy != DeathBeforeEndpointPolicy::NotApplicable {
            return Err(SelectionDesignError::InvalidDeathPolicyForViability);
        }
    } else if death_policy == DeathBeforeEndpointPolicy::NotApplicable {
        return Err(SelectionDesignError::MissingDeathBeforeEndpointPolicy);
    }

    match (death_policy, competing_risk) {
        (DeathBeforeEndpointPolicy::CompetingRisk, CompetingRiskPolicy::Declared { .. }) => Ok(()),
        (DeathBeforeEndpointPolicy::CompetingRisk, CompetingRiskPolicy::NoneDeclared) => {
            Err(SelectionDesignError::MissingCompetingRiskAuthority)
        }
        (_, CompetingRiskPolicy::Declared { .. }) => {
            Err(SelectionDesignError::UnexpectedCompetingRiskAuthority)
        }
        (_, CompetingRiskPolicy::NoneDeclared) => Ok(()),
    }
}

#[derive(Debug)]
pub enum SelectionDesignError {
    UnsupportedDesignVersion(u32),
    MissingComparisonAuthority,
    DescriptiveDesignClaimsCausalAdjustment,
    UnknownIndividual(EvolutionIndividualId),
    DuplicateIncludedIndividual(EvolutionIndividualId),
    DuplicateExclusion(EvolutionIndividualId),
    IncludedAndExcluded(EvolutionIndividualId),
    EmptyEstimandPopulation,
    IncompleteDenominatorAccounting,
    PhenotypeSupportInsufficient(EvolutionIndividualId),
    ExposureSupportInsufficient(EvolutionIndividualId),
    PhenotypeProtocolMismatch,
    ExposureProtocolMismatch,
    InvalidDeathPolicyForViability,
    MissingDeathBeforeEndpointPolicy,
    MissingCompetingRiskAuthority,
    UnexpectedCompetingRiskAuthority,
    ZeroReplicateCount,
    NonCanonicalIncludedOrder,
    NonCanonicalExclusionOrder,
    LocalDenominatorOverlap,
    DesignReplayMismatch,
}

impl fmt::Display for SelectionDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDesignVersion(version) => {
                write!(f, "unsupported selection comparison design version {version}")
            }
            Self::MissingComparisonAuthority => {
                write!(f, "non-descriptive design requires explicit comparison authority")
            }
            Self::DescriptiveDesignClaimsCausalAdjustment => write!(
                f,
                "descriptive association design must explicitly remain non-causal"
            ),
            Self::UnknownIndividual(id) => {
                write!(f, "selection design references unknown individual {}", id.as_str())
            }
            Self::DuplicateIncludedIndividual(id) => write!(
                f,
                "selection design includes individual {} more than once",
                id.as_str()
            ),
            Self::DuplicateExclusion(id) => write!(
                f,
                "selection design excludes individual {} more than once",
                id.as_str()
            ),
            Self::IncludedAndExcluded(id) => write!(
                f,
                "selection design both includes and excludes individual {}",
                id.as_str()
            ),
            Self::EmptyEstimandPopulation => {
                write!(f, "selection design estimand population may not be empty")
            }
            Self::IncompleteDenominatorAccounting => write!(
                f,
                "selection design does not account for every original evidence-ledger individual"
            ),
            Self::PhenotypeSupportInsufficient(id) => write!(
                f,
                "phenotype evidence support is insufficient for included individual {}",
                id.as_str()
            ),
            Self::ExposureSupportInsufficient(id) => write!(
                f,
                "exposure evidence support is insufficient for included individual {}",
                id.as_str()
            ),
            Self::PhenotypeProtocolMismatch => write!(
                f,
                "included phenotype evidence uses incompatible exact protocol identities"
            ),
            Self::ExposureProtocolMismatch => write!(
                f,
                "included exposure evidence uses incompatible exact protocol identities"
            ),
            Self::InvalidDeathPolicyForViability => write!(
                f,
                "viability estimand must declare death-before-endpoint policy as not applicable"
            ),
            Self::MissingDeathBeforeEndpointPolicy => write!(
                f,
                "post-viability endpoint requires explicit death-before-endpoint handling"
            ),
            Self::MissingCompetingRiskAuthority => write!(
                f,
                "competing-risk death handling requires explicit method authority"
            ),
            Self::UnexpectedCompetingRiskAuthority => write!(
                f,
                "competing-risk authority supplied when death handling is not competing risk"
            ),
            Self::ZeroReplicateCount => {
                write!(f, "uncertainty replicate count may not be zero")
            }
            Self::NonCanonicalIncludedOrder => {
                write!(f, "selection design included-individual order is noncanonical")
            }
            Self::NonCanonicalExclusionOrder => {
                write!(f, "selection design exclusion order is noncanonical")
            }
            Self::LocalDenominatorOverlap => write!(
                f,
                "selection design locally overlaps included and excluded individuals"
            ),
            Self::DesignReplayMismatch => {
                write!(f, "selection comparison design replay mismatch")
            }
        }
    }
}

impl Error for SelectionDesignError {}
