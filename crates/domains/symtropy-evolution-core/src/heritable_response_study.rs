use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisAuthorityRef,
    EvolutionaryContextRefDigest, ExplicitLinkedPopulationCensusDigest,
    ExpectedHeritableResponseDirection, HeritableResponseContextPolicy,
    HeritableResponseStudyDesign, HeritableResponseStudyDesignDigest,
    ModelSpecificSelectionEstimateDigest, PopulationGeneration, PopulationId,
    PopulationTrajectoryPointDigest, ValidatedHeritableResponseStudyDesign,
    ValidatedModelSpecificSelectionEstimate,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{cmp::Ordering, collections::BTreeMap, error::Error, fmt};

pub const GENERATION_RESPONSE_RECORD_VERSION: u32 = 1;
pub const HERITABLE_RESPONSE_STUDY_VERSION: u32 = 1;
const RECORD_DOMAIN: &[u8] = b"symtropy:evolution:generation-response-record:v1\0";
const STUDY_DOMAIN: &[u8] = b"symtropy:evolution:heritable-response-study:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationTraitDirection {
    ComparisonFavored,
    Neutral,
    ReferenceFavored,
}

impl GenerationTraitDirection {
    fn tag(self) -> u8 {
        match self {
            Self::ComparisonFavored => 0,
            Self::Neutral => 1,
            Self::ReferenceFavored => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationEvidenceDisposition {
    Included,
    Excluded { reason: AnalysisAuthorityRef },
    Unavailable { reason: AnalysisAuthorityRef },
}

impl GenerationEvidenceDisposition {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Included => digest.update([0]),
            Self::Excluded { reason } => {
                digest.update([1]);
                update_authority_digest(digest, reason);
            }
            Self::Unavailable { reason } => {
                digest.update([2]);
                update_authority_digest(digest, reason);
            }
        }
    }

    fn is_included(&self) -> bool {
        matches!(self, Self::Included)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransmissionEvidenceStatus {
    NotApplicableAtStudyStart,
    Present { authority: AnalysisAuthorityRef },
    Unavailable { reason: AnalysisAuthorityRef },
}

impl TransmissionEvidenceStatus {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::NotApplicableAtStudyStart => digest.update([0]),
            Self::Present { authority } => {
                digest.update([1]);
                update_authority_digest(digest, authority);
            }
            Self::Unavailable { reason } => {
                digest.update([2]);
                update_authority_digest(digest, reason);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationResponseEvidenceInput {
    pub generation: PopulationGeneration,
    pub population_id: PopulationId,
    pub trajectory_point_digest: PopulationTrajectoryPointDigest,
    pub census_digest: ExplicitLinkedPopulationCensusDigest,
    pub context_digest: EvolutionaryContextRefDigest,
    pub focal_class_count: u64,
    pub census_size: u64,
    pub trait_direction: GenerationTraitDirection,
    pub hereditary_frequency_evidence: AnalysisAuthorityRef,
    pub trait_response_evidence: AnalysisAuthorityRef,
    pub transmission: TransmissionEvidenceStatus,
    pub demography_evidence: AnalysisAuthorityRef,
    pub disposition: GenerationEvidenceDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationResponseRecord {
    record_version: u32,
    pub generation: PopulationGeneration,
    pub population_id: PopulationId,
    pub trajectory_point_digest: PopulationTrajectoryPointDigest,
    pub census_digest: ExplicitLinkedPopulationCensusDigest,
    pub context_digest: EvolutionaryContextRefDigest,
    pub focal_class_count: u64,
    pub census_size: u64,
    pub trait_direction: GenerationTraitDirection,
    pub hereditary_frequency_evidence: AnalysisAuthorityRef,
    pub trait_response_evidence: AnalysisAuthorityRef,
    pub transmission: TransmissionEvidenceStatus,
    pub demography_evidence: AnalysisAuthorityRef,
    pub disposition: GenerationEvidenceDisposition,
}

impl GenerationResponseRecord {
    fn from_input(input: GenerationResponseEvidenceInput) -> Result<Self, HeritableResponseError> {
        let record = Self {
            record_version: GENERATION_RESPONSE_RECORD_VERSION,
            generation: input.generation,
            population_id: input.population_id,
            trajectory_point_digest: input.trajectory_point_digest,
            census_digest: input.census_digest,
            context_digest: input.context_digest,
            focal_class_count: input.focal_class_count,
            census_size: input.census_size,
            trait_direction: input.trait_direction,
            hereditary_frequency_evidence: input.hereditary_frequency_evidence,
            trait_response_evidence: input.trait_response_evidence,
            transmission: input.transmission,
            demography_evidence: input.demography_evidence,
            disposition: input.disposition,
        };
        record.validate_local()?;
        Ok(record)
    }

    pub fn canonical_digest(&self) -> Result<GenerationResponseRecordDigest, HeritableResponseError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(RECORD_DOMAIN);
        put_u32(&mut digest, self.record_version);
        put_u64(&mut digest, self.generation.0);
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.trajectory_point_digest.as_bytes());
        digest.update(self.census_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
        put_u64(&mut digest, self.focal_class_count);
        put_u64(&mut digest, self.census_size);
        digest.update([self.trait_direction.tag()]);
        update_authority_digest(&mut digest, &self.hereditary_frequency_evidence);
        update_authority_digest(&mut digest, &self.trait_response_evidence);
        self.transmission.update_digest(&mut digest);
        update_authority_digest(&mut digest, &self.demography_evidence);
        self.disposition.update_digest(&mut digest);
        Ok(GenerationResponseRecordDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), HeritableResponseError> {
        if self.record_version != GENERATION_RESPONSE_RECORD_VERSION {
            return Err(HeritableResponseError::UnsupportedRecordVersion(self.record_version));
        }
        if self.census_size == 0 || self.focal_class_count > self.census_size {
            return Err(HeritableResponseError::FrequencyInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenerationResponseRecordDigest([u8; 32]);

impl GenerationResponseRecordDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for GenerationResponseRecordDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GenerationResponseRecordDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for GenerationResponseRecordDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeritableResponseStudyStatus {
    DirectionalResponseObserved,
    NullResponse,
    ReversedResponse,
    HereditaryOnlyMismatch,
    TraitOnlyMismatch,
    InsufficientEvidence,
}

impl HeritableResponseStudyStatus {
    fn tag(self) -> u8 {
        match self {
            Self::DirectionalResponseObserved => 0,
            Self::NullResponse => 1,
            Self::ReversedResponse => 2,
            Self::HereditaryOnlyMismatch => 3,
            Self::TraitOnlyMismatch => 4,
            Self::InsufficientEvidence => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeritableResponseStudy {
    study_version: u32,
    design: HeritableResponseStudyDesign,
    design_digest: HeritableResponseStudyDesignDigest,
    selection_estimate_digest: ModelSpecificSelectionEstimateDigest,
    pub records: Vec<GenerationResponseRecord>,
    pub status: HeritableResponseStudyStatus,
}

impl HeritableResponseStudy {
    pub fn capture(
        design: &ValidatedHeritableResponseStudyDesign<'_>,
        selection: &ValidatedModelSpecificSelectionEstimate<'_>,
        inputs: impl IntoIterator<Item = GenerationResponseEvidenceInput>,
    ) -> Result<Self, HeritableResponseError> {
        validate_selection_chain(design, selection)?;
        let mut by_generation = BTreeMap::new();
        for input in inputs {
            let generation = input.generation;
            if by_generation.insert(generation, input).is_some() {
                return Err(HeritableResponseError::DuplicateGeneration(generation));
            }
        }
        let expected_count = design.design().generation_count()?;
        if by_generation.len() as u64 != expected_count {
            return Err(HeritableResponseError::IncompleteGenerationCoverage);
        }
        let mut records = Vec::with_capacity(by_generation.len());
        for offset in 0..expected_count {
            let generation = PopulationGeneration(
                design
                    .design()
                    .start_generation
                    .0
                    .checked_add(offset)
                    .ok_or(HeritableResponseError::ArithmeticOverflow)?,
            );
            let input = by_generation
                .remove(&generation)
                .ok_or(HeritableResponseError::MissingGeneration(generation))?;
            let record = GenerationResponseRecord::from_input(input)?;
            validate_record_against_design(design.design(), &record)?;
            records.push(record);
        }
        let status = derive_status(design.design(), &records)?;
        let study = Self {
            study_version: HERITABLE_RESPONSE_STUDY_VERSION,
            design: design.design().clone(),
            design_digest: design.design_digest(),
            selection_estimate_digest: selection.estimate_digest(),
            records,
            status,
        };
        study.validate_local()?;
        Ok(study)
    }

    pub fn design(&self) -> &HeritableResponseStudyDesign {
        &self.design
    }

    pub fn design_digest(&self) -> HeritableResponseStudyDesignDigest {
        self.design_digest
    }

    pub fn selection_estimate_digest(&self) -> ModelSpecificSelectionEstimateDigest {
        self.selection_estimate_digest
    }

    pub fn canonical_digest(&self) -> Result<HeritableResponseStudyDigest, HeritableResponseError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(STUDY_DOMAIN);
        put_u32(&mut digest, self.study_version);
        digest.update(self.design_digest.as_bytes());
        digest.update(self.selection_estimate_digest.as_bytes());
        put_u64(&mut digest, self.records.len() as u64);
        for record in &self.records {
            digest.update(record.canonical_digest()?.as_bytes());
        }
        digest.update([self.status.tag()]);
        Ok(HeritableResponseStudyDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), HeritableResponseError> {
        if self.study_version != HERITABLE_RESPONSE_STUDY_VERSION {
            return Err(HeritableResponseError::UnsupportedStudyVersion(self.study_version));
        }
        if self.design.canonical_digest()? != self.design_digest
            || self.selection_estimate_digest != self.design.selection_estimate_digest()
        {
            return Err(HeritableResponseError::StudyBindingMismatch);
        }
        let expected_count = self.design.generation_count()?;
        if self.records.len() as u64 != expected_count {
            return Err(HeritableResponseError::IncompleteGenerationCoverage);
        }
        for (offset, record) in self.records.iter().enumerate() {
            record.validate_local()?;
            let expected_generation = self
                .design
                .start_generation
                .0
                .checked_add(offset as u64)
                .ok_or(HeritableResponseError::ArithmeticOverflow)?;
            if record.generation.0 != expected_generation {
                return Err(HeritableResponseError::NonCanonicalGenerationOrder);
            }
            validate_record_against_design(&self.design, record)?;
        }
        if derive_status(&self.design, &self.records)? != self.status {
            return Err(HeritableResponseError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeritableResponseStudyDigest([u8; 32]);

impl HeritableResponseStudyDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for HeritableResponseStudyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HeritableResponseStudyDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for HeritableResponseStudyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated heritable-response study should gate replication/adaptation evidence"]
pub struct ValidatedHeritableResponseStudy<'a> {
    study: &'a HeritableResponseStudy,
    study_digest: HeritableResponseStudyDigest,
    design_digest: HeritableResponseStudyDesignDigest,
    selection_estimate_digest: ModelSpecificSelectionEstimateDigest,
}

impl<'a> ValidatedHeritableResponseStudy<'a> {
    pub fn validate_current(
        study: &'a HeritableResponseStudy,
        design: &ValidatedHeritableResponseStudyDesign<'_>,
        selection: &ValidatedModelSpecificSelectionEstimate<'_>,
        inputs: impl IntoIterator<Item = GenerationResponseEvidenceInput>,
    ) -> Result<Self, HeritableResponseError> {
        study.validate_local()?;
        let recomputed = HeritableResponseStudy::capture(design, selection, inputs)?;
        if recomputed != *study {
            return Err(HeritableResponseError::StudyReplayMismatch);
        }
        Ok(Self {
            study,
            study_digest: study.canonical_digest()?,
            design_digest: design.design_digest(),
            selection_estimate_digest: selection.estimate_digest(),
        })
    }

    pub fn study(&self) -> &'a HeritableResponseStudy {
        self.study
    }

    pub fn study_digest(&self) -> HeritableResponseStudyDigest {
        self.study_digest
    }

    pub fn design_digest(&self) -> HeritableResponseStudyDesignDigest {
        self.design_digest
    }

    pub fn selection_estimate_digest(&self) -> ModelSpecificSelectionEstimateDigest {
        self.selection_estimate_digest
    }
}

fn validate_selection_chain(
    design: &ValidatedHeritableResponseStudyDesign<'_>,
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
) -> Result<(), HeritableResponseError> {
    if design.selection_estimate_digest() != selection.estimate_digest()
        || design.design().selection_model_digest() != selection.model_digest()
        || design.design().population_id() != selection.estimate().population_id()
        || design.design().selection_context_digest() != selection.estimate().context_digest()
    {
        return Err(HeritableResponseError::SelectionLineageMismatch);
    }
    Ok(())
}

fn validate_record_against_design(
    design: &HeritableResponseStudyDesign,
    record: &GenerationResponseRecord,
) -> Result<(), HeritableResponseError> {
    if &record.population_id != design.population_id() {
        return Err(HeritableResponseError::PopulationMismatch);
    }
    if record.generation.0 < design.start_generation.0
        || record.generation.0 > design.end_generation.0
    {
        return Err(HeritableResponseError::GenerationOutsideDesign(record.generation));
    }
    if matches!(
        &design.context_policy,
        HeritableResponseContextPolicy::ExactSelectionContext
    ) && record.context_digest != design.selection_context_digest()
    {
        return Err(HeritableResponseError::UndeclaredContextDrift(record.generation));
    }
    if &record.hereditary_frequency_evidence != &design.hereditary_frequency_authority
        || &record.trait_response_evidence != &design.trait_response_authority
        || &record.demography_evidence != &design.demography_accounting_authority
    {
        return Err(HeritableResponseError::EvidenceAuthorityMismatch(record.generation));
    }
    match (&record.transmission, record.generation == design.start_generation) {
        (TransmissionEvidenceStatus::NotApplicableAtStudyStart, true) => {}
        (TransmissionEvidenceStatus::Present { authority }, false)
            if authority == &design.transmission_authority => {}
        (TransmissionEvidenceStatus::Unavailable { .. }, false) => {}
        _ => return Err(HeritableResponseError::TransmissionSemanticMismatch(record.generation)),
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectionClass {
    Expected,
    Neutral,
    Opposite,
    Mixed,
}

fn derive_status(
    design: &HeritableResponseStudyDesign,
    records: &[GenerationResponseRecord],
) -> Result<HeritableResponseStudyStatus, HeritableResponseError> {
    if records.len() < 2 {
        return Err(HeritableResponseError::IncompleteGenerationCoverage);
    }
    for record in records {
        if !record.disposition.is_included()
            || matches!(
                &record.transmission,
                TransmissionEvidenceStatus::Unavailable { .. }
            )
        {
            return Ok(HeritableResponseStudyStatus::InsufficientEvidence);
        }
    }
    let hereditary = classify_hereditary_trajectory(records, design.expected_direction);
    let trait_direction = classify_trait_direction(design, &records[1..]);
    Ok(match (hereditary, trait_direction) {
        (DirectionClass::Expected, DirectionClass::Expected) => {
            HeritableResponseStudyStatus::DirectionalResponseObserved
        }
        (DirectionClass::Neutral, DirectionClass::Neutral) => {
            HeritableResponseStudyStatus::NullResponse
        }
        (DirectionClass::Opposite, DirectionClass::Opposite) => {
            HeritableResponseStudyStatus::ReversedResponse
        }
        (_, DirectionClass::Mixed) | (DirectionClass::Mixed, _) => {
            HeritableResponseStudyStatus::InsufficientEvidence
        }
        (DirectionClass::Expected, _) | (DirectionClass::Opposite, DirectionClass::Neutral) => {
            HeritableResponseStudyStatus::HereditaryOnlyMismatch
        }
        (_, DirectionClass::Expected) | (DirectionClass::Neutral, DirectionClass::Opposite) => {
            HeritableResponseStudyStatus::TraitOnlyMismatch
        }
    })
}

fn classify_hereditary_trajectory(
    records: &[GenerationResponseRecord],
    expected: ExpectedHeritableResponseDirection,
) -> DirectionClass {
    classify_frequency_trajectory(
        records
            .iter()
            .map(|record| (record.focal_class_count, record.census_size)),
        expected,
    )
}

fn classify_frequency_trajectory(
    frequencies: impl IntoIterator<Item = (u64, u64)>,
    expected: ExpectedHeritableResponseDirection,
) -> DirectionClass {
    let frequencies = frequencies.into_iter().collect::<Vec<_>>();
    let mut saw_expected = false;
    let mut saw_opposite = false;
    for pair in frequencies.windows(2) {
        match pair_frequency_direction(pair[0], pair[1], expected) {
            DirectionClass::Expected => saw_expected = true,
            DirectionClass::Opposite => saw_opposite = true,
            DirectionClass::Neutral => {}
            DirectionClass::Mixed => unreachable!("one transition cannot be mixed"),
        }
        if saw_expected && saw_opposite {
            return DirectionClass::Mixed;
        }
    }
    if saw_expected {
        DirectionClass::Expected
    } else if saw_opposite {
        DirectionClass::Opposite
    } else {
        DirectionClass::Neutral
    }
}

fn pair_frequency_direction(
    first: (u64, u64),
    second: (u64, u64),
    expected: ExpectedHeritableResponseDirection,
) -> DirectionClass {
    let left = u128::from(second.0) * u128::from(first.1);
    let right = u128::from(first.0) * u128::from(second.1);
    let observed = left.cmp(&right);
    let expected_ordering = match expected {
        ExpectedHeritableResponseDirection::ComparisonFrequencyIncrease => Ordering::Greater,
        ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease => Ordering::Less,
    };
    if observed == Ordering::Equal {
        DirectionClass::Neutral
    } else if observed == expected_ordering {
        DirectionClass::Expected
    } else {
        DirectionClass::Opposite
    }
}

fn classify_trait_direction(
    design: &HeritableResponseStudyDesign,
    records: &[GenerationResponseRecord],
) -> DirectionClass {
    let expected = match design.expected_direction {
        ExpectedHeritableResponseDirection::ComparisonFrequencyIncrease => {
            GenerationTraitDirection::ComparisonFavored
        }
        ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease => {
            GenerationTraitDirection::ReferenceFavored
        }
    };
    let opposite = match expected {
        GenerationTraitDirection::ComparisonFavored => GenerationTraitDirection::ReferenceFavored,
        GenerationTraitDirection::ReferenceFavored => GenerationTraitDirection::ComparisonFavored,
        GenerationTraitDirection::Neutral => GenerationTraitDirection::Neutral,
    };
    if records.iter().all(|record| record.trait_direction == expected) {
        DirectionClass::Expected
    } else if records
        .iter()
        .all(|record| record.trait_direction == GenerationTraitDirection::Neutral)
    {
        DirectionClass::Neutral
    } else if records.iter().all(|record| record.trait_direction == opposite) {
        DirectionClass::Opposite
    } else {
        DirectionClass::Mixed
    }
}

fn update_authority_digest(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum HeritableResponseError {
    Design(crate::HeritableResponseDesignError),
    UnsupportedRecordVersion(u32),
    UnsupportedStudyVersion(u32),
    SelectionLineageMismatch,
    DuplicateGeneration(PopulationGeneration),
    MissingGeneration(PopulationGeneration),
    IncompleteGenerationCoverage,
    NonCanonicalGenerationOrder,
    GenerationOutsideDesign(PopulationGeneration),
    PopulationMismatch,
    UndeclaredContextDrift(PopulationGeneration),
    EvidenceAuthorityMismatch(PopulationGeneration),
    TransmissionSemanticMismatch(PopulationGeneration),
    FrequencyInvariant,
    StudyBindingMismatch,
    StatusInvariant,
    StudyReplayMismatch,
    ArithmeticOverflow,
}

impl From<crate::HeritableResponseDesignError> for HeritableResponseError {
    fn from(value: crate::HeritableResponseDesignError) -> Self {
        Self::Design(value)
    }
}

impl fmt::Display for HeritableResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "heritable-response design error: {error}"),
            Self::UnsupportedRecordVersion(version) => {
                write!(f, "unsupported generation-response record version {version}")
            }
            Self::UnsupportedStudyVersion(version) => {
                write!(f, "unsupported heritable-response study version {version}")
            }
            Self::SelectionLineageMismatch => {
                write!(f, "study binds a different current SEL-08A selection lineage")
            }
            Self::DuplicateGeneration(generation) => {
                write!(f, "generation {} appears more than once", generation.0)
            }
            Self::MissingGeneration(generation) => {
                write!(f, "generation {} is missing from the declared interval", generation.0)
            }
            Self::IncompleteGenerationCoverage => write!(
                f,
                "ledger must contain exactly one record for every declared generation"
            ),
            Self::NonCanonicalGenerationOrder => {
                write!(f, "generation ledger is not canonical and contiguous")
            }
            Self::GenerationOutsideDesign(generation) => write!(
                f,
                "generation {} lies outside the preregistered interval",
                generation.0
            ),
            Self::PopulationMismatch => write!(f, "generation record binds another population"),
            Self::UndeclaredContextDrift(generation) => write!(
                f,
                "generation {} changes context under ExactSelectionContext",
                generation.0
            ),
            Self::EvidenceAuthorityMismatch(generation) => write!(
                f,
                "generation {} changes a preregistered evidence authority",
                generation.0
            ),
            Self::TransmissionSemanticMismatch(generation) => write!(
                f,
                "generation {} has invalid transmission-evidence semantics",
                generation.0
            ),
            Self::FrequencyInvariant => write!(
                f,
                "focal hereditary-class count must be within a non-zero census"
            ),
            Self::StudyBindingMismatch => write!(f, "study bindings are internally inconsistent"),
            Self::StatusInvariant => write!(
                f,
                "persisted study status does not recompute from the complete ledger"
            ),
            Self::StudyReplayMismatch => write!(
                f,
                "persisted heritable-response study does not replay against current authorities"
            ),
            Self::ArithmeticOverflow => write!(f, "heritable-response arithmetic overflowed"),
        }
    }
}

impl Error for HeritableResponseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorable_endpoint_cannot_hide_internal_frequency_reversal() {
        let class = classify_frequency_trajectory(
            [(3, 4), (1, 4), (2, 4)],
            ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease,
        );
        assert_eq!(class, DirectionClass::Mixed);
    }

    #[test]
    fn plateaus_do_not_erase_consistent_directional_response() {
        let class = classify_frequency_trajectory(
            [(3, 4), (3, 4), (2, 4), (2, 4), (1, 4)],
            ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease,
        );
        assert_eq!(class, DirectionClass::Expected);
    }
}
