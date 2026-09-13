use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AnalysisAuthorityRef, BiologicalSpeciesModel, BiologicalSpeciesModelDigest,
    CurrentSpeciesStatusEvidence, CurrentSpeciesStatusEvidenceDigest, LineageDivergenceHistory,
    LineageDivergenceHistoryDigest, LineageHistoryEpisodeKind, LineageHistoryGenerationRecord,
    LineageObservationState, PopulationGeneration, RealizedGeneFlowObservation,
    ReproductiveIsolationEvidence, ReproductiveIsolationEvidenceDigest,
    ReproductiveOpportunityOutcome, SpeciationTransitionDesign, SpeciationTransitionDesignDigest,
    SpeciationTransitionMissingPolicy, ValidatedBiologicalSpeciesModel,
    ValidatedCurrentSpeciesStatus, ValidatedLineageDivergenceHistory,
    ValidatedReproductiveIsolationEvidence, ValidatedSpeciationTransitionDesign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const SPECIATION_TRANSITION_EVIDENCE_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:speciation-transition-evidence:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SpeciationTemporalCriterion {
    PreTransitionCommonSource,
    DivergenceTiming,
    ReproductiveBarrierTiming,
    DemographicHistory,
    IntervalCompleteness,
    ModelApplicability,
    LaterCounterHistory,
}

impl SpeciationTemporalCriterion {
    fn tag(self) -> u8 {
        match self {
            Self::PreTransitionCommonSource => 0,
            Self::DivergenceTiming => 1,
            Self::ReproductiveBarrierTiming => 2,
            Self::DemographicHistory => 3,
            Self::IntervalCompleteness => 4,
            Self::ModelApplicability => 5,
            Self::LaterCounterHistory => 6,
        }
    }

    fn protocol<'a>(self, design: &'a SpeciationTransitionDesign) -> &'a AnalysisAuthorityRef {
        match self {
            Self::PreTransitionCommonSource => &design.protocols.pre_transition_common_source,
            Self::DivergenceTiming => &design.protocols.divergence_timing,
            Self::ReproductiveBarrierTiming => &design.protocols.reproductive_barrier_timing,
            Self::DemographicHistory => &design.protocols.demographic_history,
            Self::IntervalCompleteness => &design.protocols.interval_completeness,
            Self::ModelApplicability => &design.protocols.model_applicability,
            Self::LaterCounterHistory => &design.protocols.later_counter_history,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalEvidenceDisposition {
    Supports,
    DoesNotSupport,
    Contradicts,
    Unavailable,
    OutsideValidityDomain,
}

impl TemporalEvidenceDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::Supports => 0,
            Self::DoesNotSupport => 1,
            Self::Contradicts => 2,
            Self::Unavailable => 3,
            Self::OutsideValidityDomain => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpeciationTemporalEvidenceInput {
    pub criterion: SpeciationTemporalCriterion,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub disposition: TemporalEvidenceDisposition,
    pub evidence: AnalysisAuthorityRef,
    pub qualification: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciationTemporalEvidenceRecord {
    pub criterion: SpeciationTemporalCriterion,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub protocol: AnalysisAuthorityRef,
    pub disposition: TemporalEvidenceDisposition,
    pub evidence: AnalysisAuthorityRef,
    pub qualification_protocol: AnalysisAuthorityRef,
    pub qualification: AnalysisAuthorityRef,
    pub transition_design_digest: SpeciationTransitionDesignDigest,
    pub lineage_history_digest: LineageDivergenceHistoryDigest,
    pub reproductive_isolation_digest: ReproductiveIsolationEvidenceDigest,
    pub current_species_status_digest: CurrentSpeciesStatusEvidenceDigest,
    pub species_model_digest: BiologicalSpeciesModelDigest,
}

impl SpeciationTemporalEvidenceRecord {
    fn put(&self, digest: &mut Sha256) {
        digest.update([self.criterion.tag()]);
        put_u64(digest, self.start_generation.0);
        put_u64(digest, self.end_generation.0);
        put_authority(digest, &self.protocol);
        digest.update([self.disposition.tag()]);
        put_authority(digest, &self.evidence);
        put_authority(digest, &self.qualification_protocol);
        put_authority(digest, &self.qualification);
        digest.update(self.transition_design_digest.as_bytes());
        digest.update(self.lineage_history_digest.as_bytes());
        digest.update(self.reproductive_isolation_digest.as_bytes());
        digest.update(self.current_species_status_digest.as_bytes());
        digest.update(self.species_model_digest.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalCounterHistoryKind {
    Recontact,
    RealizedGeneFlow,
    AdmixtureOrIntrogression,
    LineageFusionOrRemerger,
    LineageLossOrExtinction,
    ViableFertileHybrid,
}

impl HistoricalCounterHistoryKind {
    fn tag(self) -> u8 {
        match self {
            Self::Recontact => 0,
            Self::RealizedGeneFlow => 1,
            Self::AdmixtureOrIntrogression => 2,
            Self::LineageFusionOrRemerger => 3,
            Self::LineageLossOrExtinction => 4,
            Self::ViableFertileHybrid => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalCounterHistorySource {
    LineageHistoryState,
    LineageHistoryEpisode,
    ReproductiveContactStudy,
}

impl HistoricalCounterHistorySource {
    fn tag(self) -> u8 {
        match self {
            Self::LineageHistoryState => 0,
            Self::LineageHistoryEpisode => 1,
            Self::ReproductiveContactStudy => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalCounterHistoryObservation {
    pub generation: PopulationGeneration,
    pub kind: HistoricalCounterHistoryKind,
    pub source: HistoricalCounterHistorySource,
    pub evidence: AnalysisAuthorityRef,
}

impl HistoricalCounterHistoryObservation {
    fn put(&self, digest: &mut Sha256) {
        put_u64(digest, self.generation.0);
        digest.update([self.kind.tag(), self.source.tag()]);
        put_authority(digest, &self.evidence);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciationTransitionStatus {
    TransitionSupportedUnderModel,
    TransitionNotSupportedUnderModel,
    TransitionContradictedUnderModel,
    InsufficientTemporalEvidence,
    TransitionIntervalOnly,
    OutsideModelValidityDomain,
}

impl SpeciationTransitionStatus {
    fn tag(self) -> u8 {
        match self {
            Self::TransitionSupportedUnderModel => 0,
            Self::TransitionNotSupportedUnderModel => 1,
            Self::TransitionContradictedUnderModel => 2,
            Self::InsufficientTemporalEvidence => 3,
            Self::TransitionIntervalOnly => 4,
            Self::OutsideModelValidityDomain => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciationTransitionEvidence {
    evidence_version: u32,
    design: SpeciationTransitionDesign,
    design_digest: SpeciationTransitionDesignDigest,
    lineage_history: LineageDivergenceHistory,
    lineage_history_digest: LineageDivergenceHistoryDigest,
    reproductive_isolation: ReproductiveIsolationEvidence,
    reproductive_isolation_digest: ReproductiveIsolationEvidenceDigest,
    current_species_status: CurrentSpeciesStatusEvidence,
    current_species_status_digest: CurrentSpeciesStatusEvidenceDigest,
    species_model: BiologicalSpeciesModel,
    species_model_digest: BiologicalSpeciesModelDigest,
    pub temporal_evidence: Vec<SpeciationTemporalEvidenceRecord>,
    pub later_counter_history: Vec<HistoricalCounterHistoryObservation>,
    pub status: SpeciationTransitionStatus,
}

impl SpeciationTransitionEvidence {
    pub fn evaluate(
        design: &ValidatedSpeciationTransitionDesign<'_>,
        lineage_history: &ValidatedLineageDivergenceHistory<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationEvidence<'_>,
        current_species_status: &ValidatedCurrentSpeciesStatus<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        temporal_inputs: impl IntoIterator<Item = SpeciationTemporalEvidenceInput>,
    ) -> Result<Self, SpeciationTransitionEvidenceError> {
        let frozen = design.design();
        if lineage_history.design_digest() != frozen.lineage_history_design_digest {
            return Err(SpeciationTransitionEvidenceError::LineageHistoryDesignMismatch);
        }
        if reproductive_isolation.design_digest() != frozen.reproductive_isolation_design_digest {
            return Err(SpeciationTransitionEvidenceError::IsolationDesignMismatch);
        }
        if current_species_status.design_digest() != frozen.current_species_design_digest {
            return Err(SpeciationTransitionEvidenceError::CurrentSpeciesDesignMismatch);
        }
        if species_model.model_digest() != frozen.species_model_digest {
            return Err(SpeciationTransitionEvidenceError::SpeciesModelMismatch);
        }

        let history_digest = lineage_history.history_digest();
        let isolation_digest = reproductive_isolation.evidence_digest();
        let current_digest = current_species_status.evidence_digest();
        let model_digest = species_model.model_digest();
        let temporal_evidence = materialize_temporal_evidence(
            frozen,
            design.design_digest(),
            history_digest,
            isolation_digest,
            current_digest,
            model_digest,
            temporal_inputs,
        )?;
        let later_counter_history = derive_later_counter_history(
            frozen,
            lineage_history.history(),
            reproductive_isolation.evidence(),
        );
        let status = derive_transition_status(&temporal_evidence);

        let evidence = Self {
            evidence_version: SPECIATION_TRANSITION_EVIDENCE_VERSION,
            design: frozen.clone(),
            design_digest: design.design_digest(),
            lineage_history: lineage_history.history().clone(),
            lineage_history_digest: history_digest,
            reproductive_isolation: reproductive_isolation.evidence().clone(),
            reproductive_isolation_digest: isolation_digest,
            current_species_status: current_species_status.evidence().clone(),
            current_species_status_digest: current_digest,
            species_model: species_model.model().clone(),
            species_model_digest: model_digest,
            temporal_evidence,
            later_counter_history,
            status,
        };
        evidence.validate_local()?;
        Ok(evidence)
    }

    pub fn design(&self) -> &SpeciationTransitionDesign {
        &self.design
    }

    pub fn design_digest(&self) -> SpeciationTransitionDesignDigest {
        self.design_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<SpeciationTransitionEvidenceDigest, SpeciationTransitionEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.evidence_version);
        digest.update(self.design_digest.as_bytes());
        digest.update(self.lineage_history_digest.as_bytes());
        digest.update(self.reproductive_isolation_digest.as_bytes());
        digest.update(self.current_species_status_digest.as_bytes());
        digest.update(self.species_model_digest.as_bytes());
        put_u64(&mut digest, self.temporal_evidence.len() as u64);
        for record in &self.temporal_evidence {
            record.put(&mut digest);
        }
        put_u64(&mut digest, self.later_counter_history.len() as u64);
        for observation in &self.later_counter_history {
            observation.put(&mut digest);
        }
        digest.update([self.status.tag()]);
        Ok(SpeciationTransitionEvidenceDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), SpeciationTransitionEvidenceError> {
        if self.evidence_version != SPECIATION_TRANSITION_EVIDENCE_VERSION {
            return Err(SpeciationTransitionEvidenceError::UnsupportedVersion(
                self.evidence_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(SpeciationTransitionEvidenceError::DesignBindingMismatch);
        }
        if self.lineage_history.canonical_digest()? != self.lineage_history_digest
            || self.lineage_history.design_digest() != self.design.lineage_history_design_digest
        {
            return Err(SpeciationTransitionEvidenceError::LineageHistoryBindingMismatch);
        }
        if self.reproductive_isolation.canonical_digest()? != self.reproductive_isolation_digest
            || self.reproductive_isolation.design_digest()
                != self.design.reproductive_isolation_design_digest
        {
            return Err(SpeciationTransitionEvidenceError::IsolationBindingMismatch);
        }
        if self.current_species_status.canonical_digest()? != self.current_species_status_digest
            || self.current_species_status.design_digest() != self.design.current_species_design_digest
        {
            return Err(SpeciationTransitionEvidenceError::CurrentSpeciesBindingMismatch);
        }
        if self.species_model.canonical_digest()? != self.species_model_digest
            || self.species_model_digest != self.design.species_model_digest
            || self.species_model.model_content_digest != self.design.species_model_content_digest
            || self.species_model.validity_domain.canonical_digest()
                != self.design.validity_domain_digest
        {
            return Err(SpeciationTransitionEvidenceError::SpeciesModelBindingMismatch);
        }
        if self.lineage_history.design().lineage_a != self.design.lineage_a
            || self.lineage_history.design().lineage_b != self.design.lineage_b
            || self.reproductive_isolation.design().lineage_a != self.design.lineage_a
            || self.reproductive_isolation.design().lineage_b != self.design.lineage_b
            || self.current_species_status.design().lineage_a != self.design.lineage_a
            || self.current_species_status.design().lineage_b != self.design.lineage_b
        {
            return Err(SpeciationTransitionEvidenceError::TargetLineageBindingMismatch);
        }
        validate_temporal_records(
            &self.design,
            self.design_digest,
            self.lineage_history_digest,
            self.reproductive_isolation_digest,
            self.current_species_status_digest,
            self.species_model_digest,
            &self.temporal_evidence,
        )?;
        let expected_counter = derive_later_counter_history(
            &self.design,
            &self.lineage_history,
            &self.reproductive_isolation,
        );
        if expected_counter != self.later_counter_history {
            return Err(SpeciationTransitionEvidenceError::CounterHistoryInvariant);
        }
        let expected_status = derive_transition_status(&self.temporal_evidence);
        if expected_status != self.status {
            return Err(SpeciationTransitionEvidenceError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciationTransitionEvidenceDigest([u8; 32]);

impl SpeciationTransitionEvidenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciationTransitionEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciationTransitionEvidenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciationTransitionEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated historical transition evidence is interval-bound and model-bound, never an exact event time"]
pub struct ValidatedSpeciationTransitionEvidence<'a> {
    evidence: &'a SpeciationTransitionEvidence,
    evidence_digest: SpeciationTransitionEvidenceDigest,
    design_digest: SpeciationTransitionDesignDigest,
}

impl<'a> ValidatedSpeciationTransitionEvidence<'a> {
    pub fn validate_current(
        evidence: &'a SpeciationTransitionEvidence,
        design: &ValidatedSpeciationTransitionDesign<'_>,
        lineage_history: &ValidatedLineageDivergenceHistory<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationEvidence<'_>,
        current_species_status: &ValidatedCurrentSpeciesStatus<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        temporal_inputs: impl IntoIterator<Item = SpeciationTemporalEvidenceInput>,
    ) -> Result<Self, SpeciationTransitionEvidenceError> {
        evidence.validate_local()?;
        let recomputed = SpeciationTransitionEvidence::evaluate(
            design,
            lineage_history,
            reproductive_isolation,
            current_species_status,
            species_model,
            temporal_inputs,
        )?;
        if recomputed != *evidence {
            return Err(SpeciationTransitionEvidenceError::ReplayMismatch);
        }
        Ok(Self {
            evidence,
            evidence_digest: evidence.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn evidence(&self) -> &'a SpeciationTransitionEvidence {
        self.evidence
    }

    pub fn evidence_digest(&self) -> SpeciationTransitionEvidenceDigest {
        self.evidence_digest
    }

    pub fn design_digest(&self) -> SpeciationTransitionDesignDigest {
        self.design_digest
    }
}

fn materialize_temporal_evidence(
    design: &SpeciationTransitionDesign,
    design_digest: SpeciationTransitionDesignDigest,
    history_digest: LineageDivergenceHistoryDigest,
    isolation_digest: ReproductiveIsolationEvidenceDigest,
    current_digest: CurrentSpeciesStatusEvidenceDigest,
    model_digest: BiologicalSpeciesModelDigest,
    inputs: impl IntoIterator<Item = SpeciationTemporalEvidenceInput>,
) -> Result<Vec<SpeciationTemporalEvidenceRecord>, SpeciationTransitionEvidenceError> {
    let mut by_criterion = BTreeMap::new();
    for input in inputs {
        let criterion = input.criterion;
        if by_criterion.insert(criterion, input).is_some() {
            return Err(SpeciationTransitionEvidenceError::DuplicateTemporalCriterion(
                criterion,
            ));
        }
    }
    let required = required_criteria();
    if by_criterion.len() != required.len() {
        return Err(SpeciationTransitionEvidenceError::IncompleteTemporalCriteria);
    }

    let mut records = Vec::with_capacity(required.len());
    for criterion in required {
        let input = by_criterion
            .remove(&criterion)
            .ok_or(SpeciationTransitionEvidenceError::MissingTemporalCriterion(
                criterion,
            ))?;
        validate_temporal_window(design, criterion, input.start_generation, input.end_generation)?;
        validate_disposition(criterion, input.disposition)?;
        if input.disposition == TemporalEvidenceDisposition::Unavailable
            && design.missing_policy == SpeciationTransitionMissingPolicy::FailClosed
        {
            return Err(SpeciationTransitionEvidenceError::UnavailableTemporalEvidenceForbidden(
                criterion,
            ));
        }
        records.push(SpeciationTemporalEvidenceRecord {
            criterion,
            start_generation: input.start_generation,
            end_generation: input.end_generation,
            protocol: criterion.protocol(design).clone(),
            disposition: input.disposition,
            evidence: input.evidence,
            qualification_protocol: design.protocols.qualification.clone(),
            qualification: input.qualification,
            transition_design_digest: design_digest,
            lineage_history_digest: history_digest,
            reproductive_isolation_digest: isolation_digest,
            current_species_status_digest: current_digest,
            species_model_digest: model_digest,
        });
    }
    Ok(records)
}

fn validate_temporal_records(
    design: &SpeciationTransitionDesign,
    design_digest: SpeciationTransitionDesignDigest,
    history_digest: LineageDivergenceHistoryDigest,
    isolation_digest: ReproductiveIsolationEvidenceDigest,
    current_digest: CurrentSpeciesStatusEvidenceDigest,
    model_digest: BiologicalSpeciesModelDigest,
    records: &[SpeciationTemporalEvidenceRecord],
) -> Result<(), SpeciationTransitionEvidenceError> {
    let required = required_criteria();
    if records.len() != required.len() {
        return Err(SpeciationTransitionEvidenceError::IncompleteTemporalCriteria);
    }
    for (record, criterion) in records.iter().zip(required) {
        if record.criterion != criterion {
            return Err(SpeciationTransitionEvidenceError::NonCanonicalTemporalCriterionOrder);
        }
        validate_temporal_window(
            design,
            record.criterion,
            record.start_generation,
            record.end_generation,
        )?;
        validate_disposition(record.criterion, record.disposition)?;
        if &record.protocol != record.criterion.protocol(design)
            || &record.qualification_protocol != &design.protocols.qualification
        {
            return Err(SpeciationTransitionEvidenceError::TemporalProtocolMismatch(
                record.criterion,
            ));
        }
        if record.transition_design_digest != design_digest
            || record.lineage_history_digest != history_digest
            || record.reproductive_isolation_digest != isolation_digest
            || record.current_species_status_digest != current_digest
            || record.species_model_digest != model_digest
        {
            return Err(SpeciationTransitionEvidenceError::TemporalSubjectBindingMismatch(
                record.criterion,
            ));
        }
        if record.disposition == TemporalEvidenceDisposition::Unavailable
            && design.missing_policy == SpeciationTransitionMissingPolicy::FailClosed
        {
            return Err(SpeciationTransitionEvidenceError::UnavailableTemporalEvidenceForbidden(
                record.criterion,
            ));
        }
    }
    Ok(())
}

fn required_criteria() -> [SpeciationTemporalCriterion; 7] {
    [
        SpeciationTemporalCriterion::PreTransitionCommonSource,
        SpeciationTemporalCriterion::DivergenceTiming,
        SpeciationTemporalCriterion::ReproductiveBarrierTiming,
        SpeciationTemporalCriterion::DemographicHistory,
        SpeciationTemporalCriterion::IntervalCompleteness,
        SpeciationTemporalCriterion::ModelApplicability,
        SpeciationTemporalCriterion::LaterCounterHistory,
    ]
}

fn validate_disposition(
    criterion: SpeciationTemporalCriterion,
    disposition: TemporalEvidenceDisposition,
) -> Result<(), SpeciationTransitionEvidenceError> {
    if disposition == TemporalEvidenceDisposition::OutsideValidityDomain
        && criterion != SpeciationTemporalCriterion::ModelApplicability
    {
        return Err(
            SpeciationTransitionEvidenceError::OutsideValidityDomainDispositionForbidden(
                criterion,
            ),
        );
    }
    Ok(())
}

fn validate_temporal_window(
    design: &SpeciationTransitionDesign,
    criterion: SpeciationTemporalCriterion,
    start: PopulationGeneration,
    end: PopulationGeneration,
) -> Result<(), SpeciationTransitionEvidenceError> {
    if start.0 > end.0
        || start.0 < design.history_start_generation.0
        || end.0 > design.history_end_generation.0
    {
        return Err(SpeciationTransitionEvidenceError::TemporalWindowMismatch(
            criterion,
        ));
    }
    let expected = match criterion {
        SpeciationTemporalCriterion::PreTransitionCommonSource => (
            design.history_start_generation,
            design.pre_transition_generation,
        ),
        SpeciationTemporalCriterion::DivergenceTiming => (
            design.candidate_start_generation,
            design.candidate_end_generation,
        ),
        SpeciationTemporalCriterion::ReproductiveBarrierTiming => (
            design.candidate_start_generation,
            design.post_transition_generation,
        ),
        SpeciationTemporalCriterion::DemographicHistory => (
            design.pre_transition_generation,
            design.post_transition_generation,
        ),
        SpeciationTemporalCriterion::IntervalCompleteness => (
            design.candidate_start_generation,
            design.candidate_end_generation,
        ),
        SpeciationTemporalCriterion::ModelApplicability => (
            design.candidate_start_generation,
            design.candidate_end_generation,
        ),
        SpeciationTemporalCriterion::LaterCounterHistory => (
            design.post_transition_generation,
            design.history_end_generation,
        ),
    };
    if (start, end) != expected {
        return Err(SpeciationTransitionEvidenceError::TemporalWindowMismatch(
            criterion,
        ));
    }
    Ok(())
}

fn derive_transition_status(
    records: &[SpeciationTemporalEvidenceRecord],
) -> SpeciationTransitionStatus {
    let disposition = |criterion| {
        records
            .iter()
            .find(|record| record.criterion == criterion)
            .map(|record| record.disposition)
            .expect("validated temporal evidence contains every criterion")
    };

    if disposition(SpeciationTemporalCriterion::ModelApplicability)
        == TemporalEvidenceDisposition::OutsideValidityDomain
    {
        return SpeciationTransitionStatus::OutsideModelValidityDomain;
    }
    if records
        .iter()
        .any(|record| record.disposition == TemporalEvidenceDisposition::Contradicts)
    {
        return SpeciationTransitionStatus::TransitionContradictedUnderModel;
    }
    if records
        .iter()
        .any(|record| record.disposition == TemporalEvidenceDisposition::DoesNotSupport)
    {
        return SpeciationTransitionStatus::TransitionNotSupportedUnderModel;
    }

    let bracket_supported = [
        SpeciationTemporalCriterion::PreTransitionCommonSource,
        SpeciationTemporalCriterion::DivergenceTiming,
        SpeciationTemporalCriterion::IntervalCompleteness,
        SpeciationTemporalCriterion::ModelApplicability,
    ]
    .into_iter()
    .all(|criterion| disposition(criterion) == TemporalEvidenceDisposition::Supports);

    let any_unavailable = records
        .iter()
        .any(|record| record.disposition == TemporalEvidenceDisposition::Unavailable);
    if any_unavailable {
        if bracket_supported {
            return SpeciationTransitionStatus::TransitionIntervalOnly;
        }
        return SpeciationTransitionStatus::InsufficientTemporalEvidence;
    }
    SpeciationTransitionStatus::TransitionSupportedUnderModel
}

fn derive_later_counter_history(
    design: &SpeciationTransitionDesign,
    history: &LineageDivergenceHistory,
    isolation: &ReproductiveIsolationEvidence,
) -> Vec<HistoricalCounterHistoryObservation> {
    let mut observations = Vec::new();
    for generation in &history.generations {
        let LineageHistoryGenerationRecord::Observed(record) = generation else {
            continue;
        };
        if record.generation.0 < design.post_transition_generation.0 {
            continue;
        }
        if let LineageObservationState::Observed { evidence } = &record.recontact.state {
            observations.push(HistoricalCounterHistoryObservation {
                generation: record.generation,
                kind: HistoricalCounterHistoryKind::Recontact,
                source: HistoricalCounterHistorySource::LineageHistoryState,
                evidence: evidence.evidence.clone(),
            });
        }
        if let LineageObservationState::Observed { evidence } = &record.gene_flow.state {
            observations.push(HistoricalCounterHistoryObservation {
                generation: record.generation,
                kind: HistoricalCounterHistoryKind::RealizedGeneFlow,
                source: HistoricalCounterHistorySource::LineageHistoryState,
                evidence: evidence.evidence.clone(),
            });
        }
        if let LineageObservationState::Observed { evidence } = &record.fusion.state {
            observations.push(HistoricalCounterHistoryObservation {
                generation: record.generation,
                kind: HistoricalCounterHistoryKind::LineageFusionOrRemerger,
                source: HistoricalCounterHistorySource::LineageHistoryState,
                evidence: evidence.evidence.clone(),
            });
        }
        for episode in &record.episodes {
            let kind = match episode.kind {
                LineageHistoryEpisodeKind::AdmixtureOrIntrogression => {
                    Some(HistoricalCounterHistoryKind::AdmixtureOrIntrogression)
                }
                LineageHistoryEpisodeKind::Recontact => {
                    Some(HistoricalCounterHistoryKind::Recontact)
                }
                LineageHistoryEpisodeKind::LineageFusionOrRemerger => {
                    Some(HistoricalCounterHistoryKind::LineageFusionOrRemerger)
                }
                LineageHistoryEpisodeKind::Extinction => {
                    Some(HistoricalCounterHistoryKind::LineageLossOrExtinction)
                }
                _ => None,
            };
            if let Some(kind) = kind {
                observations.push(HistoricalCounterHistoryObservation {
                    generation: episode.generation,
                    kind,
                    source: HistoricalCounterHistorySource::LineageHistoryEpisode,
                    evidence: episode.evidence.evidence.clone(),
                });
            }
        }
    }

    for study in &isolation.studies {
        for (opportunity, record) in study
            .contact_study
            .design()
            .opportunities
            .iter()
            .zip(&study.contact_study.records)
        {
            if opportunity.generation.0 < design.post_transition_generation.0 {
                continue;
            }
            if let ReproductiveOpportunityOutcome::ViableFertileOffspring { fertility, .. } =
                &record.outcome
            {
                observations.push(HistoricalCounterHistoryObservation {
                    generation: opportunity.generation,
                    kind: HistoricalCounterHistoryKind::ViableFertileHybrid,
                    source: HistoricalCounterHistorySource::ReproductiveContactStudy,
                    evidence: fertility.evidence.clone(),
                });
            }
            if let RealizedGeneFlowObservation::Realized {
                ancestry_evidence, ..
            } = &record.realized_gene_flow
            {
                observations.push(HistoricalCounterHistoryObservation {
                    generation: opportunity.generation,
                    kind: HistoricalCounterHistoryKind::RealizedGeneFlow,
                    source: HistoricalCounterHistorySource::ReproductiveContactStudy,
                    evidence: ancestry_evidence.clone(),
                });
            }
        }
    }

    observations.sort_by(|a, b| {
        a.generation
            .0
            .cmp(&b.generation.0)
            .then_with(|| a.kind.tag().cmp(&b.kind.tag()))
            .then_with(|| a.source.tag().cmp(&b.source.tag()))
            .then_with(|| a.evidence.method_id.as_str().cmp(b.evidence.method_id.as_str()))
            .then_with(|| a.evidence.revision.cmp(&b.evidence.revision))
            .then_with(|| {
                a.evidence
                    .content_digest
                    .as_bytes()
                    .cmp(b.evidence.content_digest.as_bytes())
            })
    });
    observations
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum SpeciationTransitionEvidenceError {
    Design(crate::SpeciationTransitionDesignError),
    LineageHistory(crate::LineageDivergenceHistoryError),
    Isolation(crate::ReproductiveIsolationEvidenceError),
    CurrentSpecies(crate::CurrentSpeciesStatusError),
    SpeciesModel(crate::SpeciesModelError),
    UnsupportedVersion(u32),
    LineageHistoryDesignMismatch,
    IsolationDesignMismatch,
    CurrentSpeciesDesignMismatch,
    SpeciesModelMismatch,
    DesignBindingMismatch,
    LineageHistoryBindingMismatch,
    IsolationBindingMismatch,
    CurrentSpeciesBindingMismatch,
    SpeciesModelBindingMismatch,
    TargetLineageBindingMismatch,
    DuplicateTemporalCriterion(SpeciationTemporalCriterion),
    MissingTemporalCriterion(SpeciationTemporalCriterion),
    IncompleteTemporalCriteria,
    NonCanonicalTemporalCriterionOrder,
    TemporalWindowMismatch(SpeciationTemporalCriterion),
    TemporalProtocolMismatch(SpeciationTemporalCriterion),
    TemporalSubjectBindingMismatch(SpeciationTemporalCriterion),
    UnavailableTemporalEvidenceForbidden(SpeciationTemporalCriterion),
    OutsideValidityDomainDispositionForbidden(SpeciationTemporalCriterion),
    CounterHistoryInvariant,
    StatusInvariant,
    ReplayMismatch,
}

impl From<crate::SpeciationTransitionDesignError> for SpeciationTransitionEvidenceError {
    fn from(value: crate::SpeciationTransitionDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<crate::LineageDivergenceHistoryError> for SpeciationTransitionEvidenceError {
    fn from(value: crate::LineageDivergenceHistoryError) -> Self {
        Self::LineageHistory(value)
    }
}

impl From<crate::ReproductiveIsolationEvidenceError> for SpeciationTransitionEvidenceError {
    fn from(value: crate::ReproductiveIsolationEvidenceError) -> Self {
        Self::Isolation(value)
    }
}

impl From<crate::CurrentSpeciesStatusError> for SpeciationTransitionEvidenceError {
    fn from(value: crate::CurrentSpeciesStatusError) -> Self {
        Self::CurrentSpecies(value)
    }
}

impl From<crate::SpeciesModelError> for SpeciationTransitionEvidenceError {
    fn from(value: crate::SpeciesModelError) -> Self {
        Self::SpeciesModel(value)
    }
}

impl fmt::Display for SpeciationTransitionEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "speciation-transition design error: {error}"),
            Self::LineageHistory(error) => write!(f, "lineage-history error: {error}"),
            Self::Isolation(error) => write!(f, "reproductive-isolation error: {error}"),
            Self::CurrentSpecies(error) => write!(f, "current-species error: {error}"),
            Self::SpeciesModel(error) => write!(f, "species-model error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported speciation-transition evidence version {version}")
            }
            Self::LineageHistoryDesignMismatch => write!(
                f,
                "current SEL-10A history does not match the preregistered transition design"
            ),
            Self::IsolationDesignMismatch => write!(
                f,
                "current SEL-09B isolation evidence does not match the preregistered transition design"
            ),
            Self::CurrentSpeciesDesignMismatch => write!(
                f,
                "current SEL-10C status does not match the preregistered transition design"
            ),
            Self::SpeciesModelMismatch => write!(
                f,
                "current SEL-10B model does not match the preregistered transition design"
            ),
            Self::DesignBindingMismatch => {
                write!(f, "persisted transition evidence binds a different design")
            }
            Self::LineageHistoryBindingMismatch => write!(
                f,
                "persisted SEL-10A history snapshot/digest is inconsistent"
            ),
            Self::IsolationBindingMismatch => write!(
                f,
                "persisted SEL-09B isolation snapshot/digest is inconsistent"
            ),
            Self::CurrentSpeciesBindingMismatch => write!(
                f,
                "persisted SEL-10C current-status snapshot/digest is inconsistent"
            ),
            Self::SpeciesModelBindingMismatch => write!(
                f,
                "persisted SEL-10B model snapshot/digest is inconsistent"
            ),
            Self::TargetLineageBindingMismatch => write!(
                f,
                "upstream evidence does not bind the preregistered ordered lineage pair"
            ),
            Self::DuplicateTemporalCriterion(criterion) => {
                write!(f, "duplicate temporal criterion {criterion:?}")
            }
            Self::MissingTemporalCriterion(criterion) => {
                write!(f, "missing temporal criterion {criterion:?}")
            }
            Self::IncompleteTemporalCriteria => {
                write!(f, "temporal evidence does not contain the exact seven V1 criteria")
            }
            Self::NonCanonicalTemporalCriterionOrder => {
                write!(f, "temporal criteria are not in canonical V1 order")
            }
            Self::TemporalWindowMismatch(criterion) => write!(
                f,
                "temporal criterion {criterion:?} binds the wrong generation window"
            ),
            Self::TemporalProtocolMismatch(criterion) => write!(
                f,
                "temporal criterion {criterion:?} binds the wrong evidence/qualification protocol"
            ),
            Self::TemporalSubjectBindingMismatch(criterion) => write!(
                f,
                "temporal criterion {criterion:?} binds a different transition/history/isolation/current-status/model subject"
            ),
            Self::UnavailableTemporalEvidenceForbidden(criterion) => write!(
                f,
                "temporal criterion {criterion:?} is unavailable under fail-closed policy"
            ),
            Self::OutsideValidityDomainDispositionForbidden(criterion) => write!(
                f,
                "only the ModelApplicability criterion may use OutsideValidityDomain, not {criterion:?}"
            ),
            Self::CounterHistoryInvariant => write!(
                f,
                "persisted later counter-history does not recompute from SEL-10A/09B evidence"
            ),
            Self::StatusInvariant => write!(
                f,
                "persisted transition status does not recompute from qualified temporal evidence"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted transition evidence does not replay against current authorities"
            ),
        }
    }
}

impl Error for SpeciationTransitionEvidenceError {}
