use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, EvolutionError, EvolutionaryContextRefDigest,
    ModelSpecificSelectionEstimateDigest, PopulationGeneration, PopulationId,
    RelativeViabilitySelectionQuantity, ValidatedModelSpecificSelectionEstimate,
    ViabilitySelectionTranslationModelDigest,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const HERITABLE_RESPONSE_STUDY_DESIGN_VERSION: u32 = 1;
const DESIGN_DOMAIN: &[u8] = b"symtropy:evolution:heritable-response-study-design:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct HeritableResponseStudyId(String);

impl HeritableResponseStudyId {
    pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
        let value = value.into();
        validate_text("HeritableResponseStudyId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for HeritableResponseStudyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedHeritableResponseDirection {
    ComparisonFrequencyIncrease,
    ComparisonFrequencyDecrease,
}

impl ExpectedHeritableResponseDirection {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::ComparisonFrequencyIncrease => 0,
            Self::ComparisonFrequencyDecrease => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeritableResponseContextPolicy {
    ExactSelectionContext,
    DeclaredContextTrajectory { authority: AnalysisAuthorityRef },
}

impl HeritableResponseContextPolicy {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::ExactSelectionContext => digest.update([0]),
            Self::DeclaredContextTrajectory { authority } => {
                digest.update([1]);
                update_authority_digest(digest, authority);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeritableResponseStudyDesign {
    design_version: u32,
    pub study_id: HeritableResponseStudyId,
    selection_estimate_digest: ModelSpecificSelectionEstimateDigest,
    selection_model_digest: ViabilitySelectionTranslationModelDigest,
    population_id: PopulationId,
    selection_context_digest: EvolutionaryContextRefDigest,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub expected_direction: ExpectedHeritableResponseDirection,
    pub context_policy: HeritableResponseContextPolicy,
    pub generation_axis_authority: AnalysisAuthorityRef,
    pub hereditary_frequency_authority: AnalysisAuthorityRef,
    pub trait_response_authority: AnalysisAuthorityRef,
    pub transmission_authority: AnalysisAuthorityRef,
    pub demography_accounting_authority: AnalysisAuthorityRef,
    pub response_rule_authority: AnalysisAuthorityRef,
}

impl HeritableResponseStudyDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        study_id: HeritableResponseStudyId,
        selection: &ValidatedModelSpecificSelectionEstimate<'_>,
        start_generation: PopulationGeneration,
        end_generation: PopulationGeneration,
        context_policy: HeritableResponseContextPolicy,
        generation_axis_authority: AnalysisAuthorityRef,
        hereditary_frequency_authority: AnalysisAuthorityRef,
        trait_response_authority: AnalysisAuthorityRef,
        transmission_authority: AnalysisAuthorityRef,
        demography_accounting_authority: AnalysisAuthorityRef,
        response_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, HeritableResponseDesignError> {
        if end_generation.0 <= start_generation.0 {
            return Err(HeritableResponseDesignError::GenerationIntervalTooShort);
        }
        let expected_direction = expected_direction_from_selection(selection)?;
        let design = Self {
            design_version: HERITABLE_RESPONSE_STUDY_DESIGN_VERSION,
            study_id,
            selection_estimate_digest: selection.estimate_digest(),
            selection_model_digest: selection.model_digest(),
            population_id: selection.estimate().population_id().clone(),
            selection_context_digest: selection.estimate().context_digest(),
            start_generation,
            end_generation,
            expected_direction,
            context_policy,
            generation_axis_authority,
            hereditary_frequency_authority,
            trait_response_authority,
            transmission_authority,
            demography_accounting_authority,
            response_rule_authority,
        };
        design.validate_local()?;
        Ok(design)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        selection: &ValidatedModelSpecificSelectionEstimate<'_>,
        context_policy: HeritableResponseContextPolicy,
        generation_axis_authority: AnalysisAuthorityRef,
        hereditary_frequency_authority: AnalysisAuthorityRef,
        trait_response_authority: AnalysisAuthorityRef,
        transmission_authority: AnalysisAuthorityRef,
        demography_accounting_authority: AnalysisAuthorityRef,
        response_rule_authority: AnalysisAuthorityRef,
    ) -> Result<(), HeritableResponseDesignError> {
        self.validate_local()?;
        let recomputed = Self::declare(
            self.study_id.clone(),
            selection,
            self.start_generation,
            self.end_generation,
            context_policy,
            generation_axis_authority,
            hereditary_frequency_authority,
            trait_response_authority,
            transmission_authority,
            demography_accounting_authority,
            response_rule_authority,
        )?;
        if recomputed != *self {
            return Err(HeritableResponseDesignError::ReplayMismatch);
        }
        Ok(())
    }

    pub fn selection_estimate_digest(&self) -> ModelSpecificSelectionEstimateDigest {
        self.selection_estimate_digest
    }

    pub fn selection_model_digest(&self) -> ViabilitySelectionTranslationModelDigest {
        self.selection_model_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn selection_context_digest(&self) -> EvolutionaryContextRefDigest {
        self.selection_context_digest
    }

    pub fn generation_count(&self) -> Result<u64, HeritableResponseDesignError> {
        self.end_generation
            .0
            .checked_sub(self.start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(HeritableResponseDesignError::ArithmeticOverflow)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<HeritableResponseStudyDesignDigest, HeritableResponseDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.study_id.as_str());
        digest.update(self.selection_estimate_digest.as_bytes());
        digest.update(self.selection_model_digest.as_bytes());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.selection_context_digest.as_bytes());
        put_u64(&mut digest, self.start_generation.0);
        put_u64(&mut digest, self.end_generation.0);
        digest.update([self.expected_direction.tag()]);
        self.context_policy.update_digest(&mut digest);
        update_authority_digest(&mut digest, &self.generation_axis_authority);
        update_authority_digest(&mut digest, &self.hereditary_frequency_authority);
        update_authority_digest(&mut digest, &self.trait_response_authority);
        update_authority_digest(&mut digest, &self.transmission_authority);
        update_authority_digest(&mut digest, &self.demography_accounting_authority);
        update_authority_digest(&mut digest, &self.response_rule_authority);
        Ok(HeritableResponseStudyDesignDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), HeritableResponseDesignError> {
        if self.design_version != HERITABLE_RESPONSE_STUDY_DESIGN_VERSION {
            return Err(HeritableResponseDesignError::UnsupportedVersion(self.design_version));
        }
        if self.end_generation.0 <= self.start_generation.0 {
            return Err(HeritableResponseDesignError::GenerationIntervalTooShort);
        }
        self.generation_count()?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeritableResponseStudyDesignDigest([u8; 32]);

impl HeritableResponseStudyDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for HeritableResponseStudyDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HeritableResponseStudyDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for HeritableResponseStudyDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated study design should gate generation-ledger capture"]
pub struct ValidatedHeritableResponseStudyDesign<'a> {
    design: &'a HeritableResponseStudyDesign,
    design_digest: HeritableResponseStudyDesignDigest,
    selection_estimate_digest: ModelSpecificSelectionEstimateDigest,
}

impl<'a> ValidatedHeritableResponseStudyDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a HeritableResponseStudyDesign,
        selection: &ValidatedModelSpecificSelectionEstimate<'_>,
        context_policy: HeritableResponseContextPolicy,
        generation_axis_authority: AnalysisAuthorityRef,
        hereditary_frequency_authority: AnalysisAuthorityRef,
        trait_response_authority: AnalysisAuthorityRef,
        transmission_authority: AnalysisAuthorityRef,
        demography_accounting_authority: AnalysisAuthorityRef,
        response_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, HeritableResponseDesignError> {
        design.validate_current(
            selection,
            context_policy,
            generation_axis_authority,
            hereditary_frequency_authority,
            trait_response_authority,
            transmission_authority,
            demography_accounting_authority,
            response_rule_authority,
        )?;
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
            selection_estimate_digest: selection.estimate_digest(),
        })
    }

    pub fn design(&self) -> &'a HeritableResponseStudyDesign {
        self.design
    }

    pub fn design_digest(&self) -> HeritableResponseStudyDesignDigest {
        self.design_digest
    }

    pub fn selection_estimate_digest(&self) -> ModelSpecificSelectionEstimateDigest {
        self.selection_estimate_digest
    }
}

fn expected_direction_from_selection(
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
) -> Result<ExpectedHeritableResponseDirection, HeritableResponseDesignError> {
    match &selection.estimate().selection_quantity {
        RelativeViabilitySelectionQuantity::Finite(value) if value.numerator == 0 => {
            Err(HeritableResponseDesignError::NonDirectionalSelectionQuantity)
        }
        RelativeViabilitySelectionQuantity::Finite(value) if value.negative => {
            Ok(ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease)
        }
        RelativeViabilitySelectionQuantity::Finite(_) => {
            Ok(ExpectedHeritableResponseDirection::ComparisonFrequencyIncrease)
        }
        RelativeViabilitySelectionQuantity::PositiveInfinity => {
            Ok(ExpectedHeritableResponseDirection::ComparisonFrequencyIncrease)
        }
        RelativeViabilitySelectionQuantity::UndefinedBothSurvivalsZero => {
            Err(HeritableResponseDesignError::UndefinedSelectionQuantity)
        }
    }
}

fn update_authority_digest(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum HeritableResponseDesignError {
    UnsupportedVersion(u32),
    GenerationIntervalTooShort,
    NonDirectionalSelectionQuantity,
    UndefinedSelectionQuantity,
    ReplayMismatch,
    ArithmeticOverflow,
}

impl fmt::Display for HeritableResponseDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported heritable-response study-design version {version}")
            }
            Self::GenerationIntervalTooShort => write!(
                f,
                "multi-generation heritable-response study requires end generation after start"
            ),
            Self::NonDirectionalSelectionQuantity => write!(
                f,
                "SEL-08B1 V1 requires a non-zero directional SEL-08A selection quantity"
            ),
            Self::UndefinedSelectionQuantity => write!(
                f,
                "undefined SEL-08A relative viability cannot preregister a directional study"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted heritable-response design does not replay against current authorities"
            ),
            Self::ArithmeticOverflow => write!(f, "heritable-response design arithmetic overflowed"),
        }
    }
}

impl Error for HeritableResponseDesignError {}
