use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisContentDigest,
    BinaryViabilityGroupSupport, CausalSelectionTarget, CausalViabilityRiskEffectDigest,
    EvolutionaryContextRefDigest, GenerationMappingAuthorityId, HeritableClassMappingAuthorityId,
    ModelSpecificSelectionEstimateId, PopulationId, PredictorDefinitionBindingDigest,
    PredictorRepresentation, PredictorSourceKind, SelectionEstimand, SelectionTranslationModelId,
    SelectionTranslationQualificationAuthorityId, ValidatedCausalSelectionEffect,
    ValidatedSelectionAnalysisFrame,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const VIABILITY_SELECTION_TRANSLATION_MODEL_VERSION: u32 = 1;
pub const MODEL_SPECIFIC_SELECTION_ESTIMATE_VERSION: u32 = 1;

const MODEL_DOMAIN: &[u8] = b"symtropy:evolution:viability-selection-translation-model:v1\0";
const RESULT_DOMAIN: &[u8] = b"symtropy:evolution:model-specific-selection-estimate:v1\0";
const MODEL_SPEC_DOMAIN: &[u8] = b"symtropy:evolution:viability-selection-model-spec:v1\0";
const MODEL_SPEC: &[u8] = b"binary genotype-or-lineage Reference/Comparison classes; viability consequence-window stage; convert death risk to survival; relative viability ratio comparison_survival/reference_survival; model-specific selection quantity ratio_minus_one; Reference baseline; other fitness components held equal or excluded by qualified model; frequency dependence excluded; density dependence excluded; migration contribution excluded; exact generation mapping and heritable-class mapping required; zero Reference survival produces typed infinity/undefined boundary; no generic lifetime fitness, universal selection coefficient, adaptation, or speciation claim";

pub fn viability_selection_model_content_digest_v1() -> AnalysisContentDigest {
    let mut digest = Sha256::new();
    digest.update(MODEL_SPEC_DOMAIN);
    put_u64(&mut digest, MODEL_SPEC.len() as u64);
    digest.update(MODEL_SPEC);
    AnalysisContentDigest::new(digest.finalize().into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionQuantityKind {
    RelativeViabilityRatioMinusOne,
}

impl SelectionQuantityKind {
    fn tag(self) -> u8 {
        match self {
            Self::RelativeViabilityRatioMinusOne => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionBaselineConvention {
    ReferenceSurvival,
}

impl SelectionBaselineConvention {
    fn tag(self) -> u8 {
        match self {
            Self::ReferenceSurvival => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViabilityLifeCycleStagePolicy {
    BoundConsequenceWindow,
}

impl ViabilityLifeCycleStagePolicy {
    fn tag(self) -> u8 {
        match self {
            Self::BoundConsequenceWindow => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OtherFitnessComponentsPolicy {
    HeldEqualOrExcludedByQualifiedModel,
}

impl OtherFitnessComponentsPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::HeldEqualOrExcludedByQualifiedModel => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrequencyDependencePolicy {
    ExcludedByQualifiedModel,
}

impl FrequencyDependencePolicy {
    fn tag(self) -> u8 {
        match self {
            Self::ExcludedByQualifiedModel => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityDependencePolicy {
    ExcludedByQualifiedModel,
}

impl DensityDependencePolicy {
    fn tag(self) -> u8 {
        match self {
            Self::ExcludedByQualifiedModel => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationContributionPolicy {
    ExcludedByQualifiedModel,
}

impl MigrationContributionPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::ExcludedByQualifiedModel => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationMappingRef {
    pub authority_id: GenerationMappingAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
    pub population_id: PopulationId,
    pub context_digest: EvolutionaryContextRefDigest,
}

impl GenerationMappingRef {
    pub fn new(
        authority_id: GenerationMappingAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
        population_id: PopulationId,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
            population_id,
            context_digest,
        }
    }

    fn validate_subject(
        &self,
        population_id: &PopulationId,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionTranslationError> {
        if &self.population_id != population_id || self.context_digest != context_digest {
            return Err(SelectionTranslationError::GenerationMappingSubjectMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        put_text(digest, self.population_id.as_str());
        digest.update(self.context_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeritableClassMappingRef {
    pub authority_id: HeritableClassMappingAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
    pub predictor_definition_digest: PredictorDefinitionBindingDigest,
    pub context_digest: EvolutionaryContextRefDigest,
}

impl HeritableClassMappingRef {
    pub fn new(
        authority_id: HeritableClassMappingAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
        predictor_definition_digest: PredictorDefinitionBindingDigest,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
            predictor_definition_digest,
            context_digest,
        }
    }

    fn validate_subject(
        &self,
        predictor_definition_digest: PredictorDefinitionBindingDigest,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionTranslationError> {
        if self.predictor_definition_digest != predictor_definition_digest
            || self.context_digest != context_digest
        {
            return Err(SelectionTranslationError::HeritableClassMappingSubjectMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        digest.update(self.predictor_definition_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionTranslationQualificationRef {
    pub authority_id: SelectionTranslationQualificationAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
    pub population_id: PopulationId,
    pub predictor_definition_digest: PredictorDefinitionBindingDigest,
    pub context_digest: EvolutionaryContextRefDigest,
}

impl SelectionTranslationQualificationRef {
    pub fn new(
        authority_id: SelectionTranslationQualificationAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
        population_id: PopulationId,
        predictor_definition_digest: PredictorDefinitionBindingDigest,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
            population_id,
            predictor_definition_digest,
            context_digest,
        }
    }

    fn validate_subject(
        &self,
        population_id: &PopulationId,
        predictor_definition_digest: PredictorDefinitionBindingDigest,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionTranslationError> {
        if &self.population_id != population_id
            || self.predictor_definition_digest != predictor_definition_digest
            || self.context_digest != context_digest
        {
            return Err(SelectionTranslationError::QualificationSubjectMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        put_text(digest, self.population_id.as_str());
        digest.update(self.predictor_definition_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViabilitySelectionTranslationModel {
    model_version: u32,
    pub model_id: SelectionTranslationModelId,
    pub model_revision: u64,
    pub model_content_digest: AnalysisContentDigest,
    pub target: CausalSelectionTarget,
    pub quantity_kind: SelectionQuantityKind,
    pub baseline: SelectionBaselineConvention,
    pub life_cycle_stage: ViabilityLifeCycleStagePolicy,
    pub other_fitness_components: OtherFitnessComponentsPolicy,
    pub frequency_dependence: FrequencyDependencePolicy,
    pub density_dependence: DensityDependencePolicy,
    pub migration_contribution: MigrationContributionPolicy,
    population_id: PopulationId,
    context_digest: EvolutionaryContextRefDigest,
    predictor_definition_digest: PredictorDefinitionBindingDigest,
    pub generation_mapping: GenerationMappingRef,
    pub heritable_class_mapping: HeritableClassMappingRef,
    pub qualification: SelectionTranslationQualificationRef,
}

impl ViabilitySelectionTranslationModel {
    pub fn declare(
        model_id: SelectionTranslationModelId,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        generation_mapping: GenerationMappingRef,
        heritable_class_mapping: HeritableClassMappingRef,
        qualification: SelectionTranslationQualificationRef,
    ) -> Result<Self, SelectionTranslationError> {
        validate_frame_for_model(frame)?;
        let predictor_definition_digest = frame
            .frame()
            .rows
            .first()
            .ok_or(SelectionTranslationError::EmptyFrame)?
            .predictor_definition_digest;
        let population_id = frame.population_id().clone();
        let context_digest = frame.context_digest();
        generation_mapping.validate_subject(&population_id, context_digest)?;
        heritable_class_mapping
            .validate_subject(predictor_definition_digest, context_digest)?;
        qualification.validate_subject(
            &population_id,
            predictor_definition_digest,
            context_digest,
        )?;

        let model = Self {
            model_version: VIABILITY_SELECTION_TRANSLATION_MODEL_VERSION,
            model_id,
            model_revision: 1,
            model_content_digest: viability_selection_model_content_digest_v1(),
            target: CausalSelectionTarget::BinaryViabilityRiskDifference,
            quantity_kind: SelectionQuantityKind::RelativeViabilityRatioMinusOne,
            baseline: SelectionBaselineConvention::ReferenceSurvival,
            life_cycle_stage: ViabilityLifeCycleStagePolicy::BoundConsequenceWindow,
            other_fitness_components:
                OtherFitnessComponentsPolicy::HeldEqualOrExcludedByQualifiedModel,
            frequency_dependence: FrequencyDependencePolicy::ExcludedByQualifiedModel,
            density_dependence: DensityDependencePolicy::ExcludedByQualifiedModel,
            migration_contribution: MigrationContributionPolicy::ExcludedByQualifiedModel,
            population_id,
            context_digest,
            predictor_definition_digest,
            generation_mapping,
            heritable_class_mapping,
            qualification,
        };
        model.validate_local()?;
        Ok(model)
    }

    pub fn validate_current(
        &self,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        generation_mapping: GenerationMappingRef,
        heritable_class_mapping: HeritableClassMappingRef,
        qualification: SelectionTranslationQualificationRef,
    ) -> Result<(), SelectionTranslationError> {
        self.validate_local()?;
        let recomputed = Self::declare(
            self.model_id.clone(),
            frame,
            generation_mapping,
            heritable_class_mapping,
            qualification,
        )?;
        if recomputed != *self {
            return Err(SelectionTranslationError::ModelReplayMismatch);
        }
        Ok(())
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }

    pub fn predictor_definition_digest(&self) -> PredictorDefinitionBindingDigest {
        self.predictor_definition_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ViabilitySelectionTranslationModelDigest, SelectionTranslationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(MODEL_DOMAIN);
        put_u32(&mut digest, self.model_version);
        put_text(&mut digest, self.model_id.as_str());
        put_u64(&mut digest, self.model_revision);
        digest.update(self.model_content_digest.as_bytes());
        digest.update([
            target_tag(self.target),
            self.quantity_kind.tag(),
            self.baseline.tag(),
            self.life_cycle_stage.tag(),
            self.other_fitness_components.tag(),
            self.frequency_dependence.tag(),
            self.density_dependence.tag(),
            self.migration_contribution.tag(),
        ]);
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.context_digest.as_bytes());
        digest.update(self.predictor_definition_digest.as_bytes());
        self.generation_mapping.update_digest(&mut digest);
        self.heritable_class_mapping.update_digest(&mut digest);
        self.qualification.update_digest(&mut digest);
        Ok(ViabilitySelectionTranslationModelDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), SelectionTranslationError> {
        if self.model_version != VIABILITY_SELECTION_TRANSLATION_MODEL_VERSION {
            return Err(SelectionTranslationError::UnsupportedModelVersion(
                self.model_version,
            ));
        }
        if self.model_revision != 1
            || self.model_content_digest != viability_selection_model_content_digest_v1()
        {
            return Err(SelectionTranslationError::ModelSpecMismatch);
        }
        if self.target != CausalSelectionTarget::BinaryViabilityRiskDifference
            || self.quantity_kind != SelectionQuantityKind::RelativeViabilityRatioMinusOne
            || self.baseline != SelectionBaselineConvention::ReferenceSurvival
            || self.life_cycle_stage != ViabilityLifeCycleStagePolicy::BoundConsequenceWindow
            || self.other_fitness_components
                != OtherFitnessComponentsPolicy::HeldEqualOrExcludedByQualifiedModel
            || self.frequency_dependence != FrequencyDependencePolicy::ExcludedByQualifiedModel
            || self.density_dependence != DensityDependencePolicy::ExcludedByQualifiedModel
            || self.migration_contribution != MigrationContributionPolicy::ExcludedByQualifiedModel
        {
            return Err(SelectionTranslationError::ModelSemanticMismatch);
        }
        self.generation_mapping
            .validate_subject(&self.population_id, self.context_digest)?;
        self.heritable_class_mapping
            .validate_subject(self.predictor_definition_digest, self.context_digest)?;
        self.qualification.validate_subject(
            &self.population_id,
            self.predictor_definition_digest,
            self.context_digest,
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViabilitySelectionTranslationModelDigest([u8; 32]);

impl ViabilitySelectionTranslationModelDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ViabilitySelectionTranslationModelDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ViabilitySelectionTranslationModelDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ViabilitySelectionTranslationModelDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated selection-translation model should gate model-specific selection execution"]
pub struct ValidatedSelectionTranslationModel<'a> {
    model: &'a ViabilitySelectionTranslationModel,
    model_digest: ViabilitySelectionTranslationModelDigest,
    population_id: PopulationId,
    context_digest: EvolutionaryContextRefDigest,
    predictor_definition_digest: PredictorDefinitionBindingDigest,
}

impl<'a> ValidatedSelectionTranslationModel<'a> {
    pub fn validate_current(
        model: &'a ViabilitySelectionTranslationModel,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        generation_mapping: GenerationMappingRef,
        heritable_class_mapping: HeritableClassMappingRef,
        qualification: SelectionTranslationQualificationRef,
    ) -> Result<Self, SelectionTranslationError> {
        model.validate_current(
            frame,
            generation_mapping,
            heritable_class_mapping,
            qualification,
        )?;
        Ok(Self {
            model,
            model_digest: model.canonical_digest()?,
            population_id: frame.population_id().clone(),
            context_digest: frame.context_digest(),
            predictor_definition_digest: model.predictor_definition_digest,
        })
    }

    pub fn model(&self) -> &'a ViabilitySelectionTranslationModel {
        self.model
    }

    pub fn model_digest(&self) -> ViabilitySelectionTranslationModelDigest {
        self.model_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }

    pub fn predictor_definition_digest(&self) -> PredictorDefinitionBindingDigest {
        self.predictor_definition_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactSelectionUnsignedFraction {
    pub numerator: u64,
    pub denominator: u64,
}

impl ExactSelectionUnsignedFraction {
    fn new(numerator: u64, denominator: u64) -> Result<Self, SelectionTranslationError> {
        if denominator == 0 {
            return Err(SelectionTranslationError::ArithmeticInvariant);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn validate_canonical(self) -> Result<(), SelectionTranslationError> {
        if self.denominator == 0 || gcd(self.numerator, self.denominator) != 1 {
            return Err(SelectionTranslationError::ArithmeticInvariant);
        }
        Ok(())
    }

    fn update_digest(self, digest: &mut Sha256) {
        put_u64(digest, self.numerator);
        put_u64(digest, self.denominator);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactSelectionSignedFraction {
    pub negative: bool,
    pub numerator: u64,
    pub denominator: u64,
}

impl ExactSelectionSignedFraction {
    fn new(numerator: i64, denominator: u64) -> Result<Self, SelectionTranslationError> {
        if denominator == 0 {
            return Err(SelectionTranslationError::ArithmeticInvariant);
        }
        let magnitude = numerator.unsigned_abs();
        let divisor = gcd(magnitude, denominator);
        Ok(Self {
            negative: numerator < 0 && magnitude != 0,
            numerator: magnitude / divisor,
            denominator: denominator / divisor,
        })
    }

    fn validate_canonical(self) -> Result<(), SelectionTranslationError> {
        if self.denominator == 0
            || (self.numerator == 0 && self.negative)
            || gcd(self.numerator, self.denominator) != 1
        {
            return Err(SelectionTranslationError::ArithmeticInvariant);
        }
        Ok(())
    }

    fn update_digest(self, digest: &mut Sha256) {
        digest.update([u8::from(self.negative)]);
        put_u64(digest, self.numerator);
        put_u64(digest, self.denominator);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelativeViabilityRatio {
    Finite(ExactSelectionUnsignedFraction),
    PositiveInfinity,
    UndefinedBothSurvivalsZero,
}

impl RelativeViabilityRatio {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Finite(value) => {
                digest.update([0]);
                value.update_digest(digest);
            }
            Self::PositiveInfinity => digest.update([1]),
            Self::UndefinedBothSurvivalsZero => digest.update([2]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelativeViabilitySelectionQuantity {
    Finite(ExactSelectionSignedFraction),
    PositiveInfinity,
    UndefinedBothSurvivalsZero,
}

impl RelativeViabilitySelectionQuantity {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Finite(value) => {
                digest.update([0]);
                value.update_digest(digest);
            }
            Self::PositiveInfinity => digest.update([1]),
            Self::UndefinedBothSurvivalsZero => digest.update([2]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionSignConvention {
    PositiveMeansHigherComparisonRelativeViability,
}

impl SelectionSignConvention {
    fn tag(self) -> u8 {
        match self {
            Self::PositiveMeansHigherComparisonRelativeViability => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSpecificSelectionEstimate {
    estimate_version: u32,
    pub estimate_id: ModelSpecificSelectionEstimateId,
    causal_effect_digest: CausalViabilityRiskEffectDigest,
    model_digest: ViabilitySelectionTranslationModelDigest,
    frame_digest: crate::ExplicitSelectionAnalysisFrameDigest,
    population_id: PopulationId,
    context_digest: EvolutionaryContextRefDigest,
    predictor_definition_digest: PredictorDefinitionBindingDigest,
    pub target: CausalSelectionTarget,
    pub quantity_kind: SelectionQuantityKind,
    pub sign_convention: SelectionSignConvention,
    pub reference_support: BinaryViabilityGroupSupport,
    pub comparison_support: BinaryViabilityGroupSupport,
    pub reference_survival: ExactSelectionUnsignedFraction,
    pub comparison_survival: ExactSelectionUnsignedFraction,
    pub relative_viability_ratio: RelativeViabilityRatio,
    pub selection_quantity: RelativeViabilitySelectionQuantity,
}

impl ModelSpecificSelectionEstimate {
    pub fn causal_effect_digest(&self) -> CausalViabilityRiskEffectDigest {
        self.causal_effect_digest
    }

    pub fn model_digest(&self) -> ViabilitySelectionTranslationModelDigest {
        self.model_digest
    }

    pub fn frame_digest(&self) -> crate::ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ModelSpecificSelectionEstimateDigest, SelectionTranslationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(RESULT_DOMAIN);
        put_u32(&mut digest, self.estimate_version);
        put_text(&mut digest, self.estimate_id.as_str());
        digest.update(self.causal_effect_digest.as_bytes());
        digest.update(self.model_digest.as_bytes());
        digest.update(self.frame_digest.as_bytes());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.context_digest.as_bytes());
        digest.update(self.predictor_definition_digest.as_bytes());
        digest.update([
            target_tag(self.target),
            self.quantity_kind.tag(),
            self.sign_convention.tag(),
        ]);
        update_group_support_digest(&mut digest, self.reference_support);
        update_group_support_digest(&mut digest, self.comparison_support);
        self.reference_survival.update_digest(&mut digest);
        self.comparison_survival.update_digest(&mut digest);
        self.relative_viability_ratio.update_digest(&mut digest);
        self.selection_quantity.update_digest(&mut digest);
        Ok(ModelSpecificSelectionEstimateDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), SelectionTranslationError> {
        if self.estimate_version != MODEL_SPECIFIC_SELECTION_ESTIMATE_VERSION {
            return Err(SelectionTranslationError::UnsupportedEstimateVersion(
                self.estimate_version,
            ));
        }
        if self.target != CausalSelectionTarget::BinaryViabilityRiskDifference
            || self.quantity_kind != SelectionQuantityKind::RelativeViabilityRatioMinusOne
            || self.sign_convention
                != SelectionSignConvention::PositiveMeansHigherComparisonRelativeViability
        {
            return Err(SelectionTranslationError::ResultSemanticMismatch);
        }
        validate_group_support(self.reference_support)?;
        validate_group_support(self.comparison_support)?;
        let derived = derive_selection(self.reference_support, self.comparison_support)?;
        if self.reference_survival != derived.reference_survival
            || self.comparison_survival != derived.comparison_survival
            || self.relative_viability_ratio != derived.relative_viability_ratio
            || self.selection_quantity != derived.selection_quantity
        {
            return Err(SelectionTranslationError::ResultArithmeticMismatch);
        }
        self.reference_survival.validate_canonical()?;
        self.comparison_survival.validate_canonical()?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelSpecificSelectionEstimateDigest([u8; 32]);

impl ModelSpecificSelectionEstimateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ModelSpecificSelectionEstimateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModelSpecificSelectionEstimateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ModelSpecificSelectionEstimateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated model-specific selection evidence should remain bound to current causal effect and translation model"]
pub struct ValidatedModelSpecificSelectionEstimate<'a> {
    estimate: &'a ModelSpecificSelectionEstimate,
    estimate_digest: ModelSpecificSelectionEstimateDigest,
    effect_digest: CausalViabilityRiskEffectDigest,
    model_digest: ViabilitySelectionTranslationModelDigest,
    frame_digest: crate::ExplicitSelectionAnalysisFrameDigest,
}

impl<'a> ValidatedModelSpecificSelectionEstimate<'a> {
    pub fn validate_current(
        estimate: &'a ModelSpecificSelectionEstimate,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        effect: &ValidatedCausalSelectionEffect<'_>,
        model: &ValidatedSelectionTranslationModel<'_>,
    ) -> Result<Self, SelectionTranslationError> {
        estimate.validate_local()?;
        let recomputed = translate_model_specific_viability_selection(
            estimate.estimate_id.clone(),
            frame,
            effect,
            model,
        )?;
        if recomputed != *estimate {
            return Err(SelectionTranslationError::EstimateReplayMismatch);
        }
        Ok(Self {
            estimate,
            estimate_digest: estimate.canonical_digest()?,
            effect_digest: effect.effect_digest(),
            model_digest: model.model_digest(),
            frame_digest: frame.frame_digest(),
        })
    }

    pub fn estimate(&self) -> &'a ModelSpecificSelectionEstimate {
        self.estimate
    }

    pub fn estimate_digest(&self) -> ModelSpecificSelectionEstimateDigest {
        self.estimate_digest
    }

    pub fn effect_digest(&self) -> CausalViabilityRiskEffectDigest {
        self.effect_digest
    }

    pub fn model_digest(&self) -> ViabilitySelectionTranslationModelDigest {
        self.model_digest
    }

    pub fn frame_digest(&self) -> crate::ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }
}

pub fn translate_model_specific_viability_selection(
    estimate_id: ModelSpecificSelectionEstimateId,
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    effect: &ValidatedCausalSelectionEffect<'_>,
    model: &ValidatedSelectionTranslationModel<'_>,
) -> Result<ModelSpecificSelectionEstimate, SelectionTranslationError> {
    validate_translation_chain(frame, effect, model)?;
    let derived = derive_selection(
        effect.effect().reference_support,
        effect.effect().comparison_support,
    )?;
    let estimate = ModelSpecificSelectionEstimate {
        estimate_version: MODEL_SPECIFIC_SELECTION_ESTIMATE_VERSION,
        estimate_id,
        causal_effect_digest: effect.effect_digest(),
        model_digest: model.model_digest(),
        frame_digest: frame.frame_digest(),
        population_id: frame.population_id().clone(),
        context_digest: frame.context_digest(),
        predictor_definition_digest: model.predictor_definition_digest(),
        target: effect.effect().target,
        quantity_kind: model.model().quantity_kind,
        sign_convention: SelectionSignConvention::PositiveMeansHigherComparisonRelativeViability,
        reference_support: effect.effect().reference_support,
        comparison_support: effect.effect().comparison_support,
        reference_survival: derived.reference_survival,
        comparison_survival: derived.comparison_survival,
        relative_viability_ratio: derived.relative_viability_ratio,
        selection_quantity: derived.selection_quantity,
    };
    estimate.validate_local()?;
    Ok(estimate)
}

fn validate_frame_for_model(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
) -> Result<(), SelectionTranslationError> {
    if frame.estimand() != SelectionEstimand::ViabilityWindowRiskContrast {
        return Err(SelectionTranslationError::UnsupportedEstimand(frame.estimand()));
    }
    if frame.frame().predictor_representation != PredictorRepresentation::BinaryComparisonGroup {
        return Err(SelectionTranslationError::UnsupportedPredictorRepresentation);
    }
    if frame.frame().design.predictor.source_kind != PredictorSourceKind::GenotypeOrLineage {
        return Err(SelectionTranslationError::NonHeritablePredictorSource(
            frame.frame().design.predictor.source_kind,
        ));
    }
    if frame.frame().rows.is_empty() {
        return Err(SelectionTranslationError::EmptyFrame);
    }
    Ok(())
}

fn validate_translation_chain(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    effect: &ValidatedCausalSelectionEffect<'_>,
    model: &ValidatedSelectionTranslationModel<'_>,
) -> Result<(), SelectionTranslationError> {
    validate_frame_for_model(frame)?;
    if effect.frame_digest() != frame.frame_digest() {
        return Err(SelectionTranslationError::EffectFrameMismatch);
    }
    if effect.effect().target != CausalSelectionTarget::BinaryViabilityRiskDifference
        || model.model().target != CausalSelectionTarget::BinaryViabilityRiskDifference
    {
        return Err(SelectionTranslationError::UnsupportedTarget);
    }
    let predictor_definition_digest = frame.frame().rows[0].predictor_definition_digest;
    if model.population_id() != frame.population_id()
        || model.context_digest() != frame.context_digest()
        || model.predictor_definition_digest() != predictor_definition_digest
    {
        return Err(SelectionTranslationError::ModelFrameMismatch);
    }
    if effect.effect().original_denominator() != frame.frame().original_denominator()
        || effect.effect().estimand_denominator() != frame.frame().estimand_denominator()
    {
        return Err(SelectionTranslationError::DenominatorInvariant);
    }
    Ok(())
}

struct DerivedSelection {
    reference_survival: ExactSelectionUnsignedFraction,
    comparison_survival: ExactSelectionUnsignedFraction,
    relative_viability_ratio: RelativeViabilityRatio,
    selection_quantity: RelativeViabilitySelectionQuantity,
}

fn derive_selection(
    reference: BinaryViabilityGroupSupport,
    comparison: BinaryViabilityGroupSupport,
) -> Result<DerivedSelection, SelectionTranslationError> {
    validate_group_support(reference)?;
    validate_group_support(comparison)?;
    let reference_survival =
        ExactSelectionUnsignedFraction::new(reference.survived_window, reference.total)?;
    let comparison_survival =
        ExactSelectionUnsignedFraction::new(comparison.survived_window, comparison.total)?;

    let (relative_viability_ratio, selection_quantity) = if reference.survived_window == 0 {
        if comparison.survived_window == 0 {
            (
                RelativeViabilityRatio::UndefinedBothSurvivalsZero,
                RelativeViabilitySelectionQuantity::UndefinedBothSurvivalsZero,
            )
        } else {
            (
                RelativeViabilityRatio::PositiveInfinity,
                RelativeViabilitySelectionQuantity::PositiveInfinity,
            )
        }
    } else {
        let ratio_numerator = comparison
            .survived_window
            .checked_mul(reference.total)
            .ok_or(SelectionTranslationError::ArithmeticOverflow)?;
        let ratio_denominator = reference
            .survived_window
            .checked_mul(comparison.total)
            .ok_or(SelectionTranslationError::ArithmeticOverflow)?;
        let ratio = ExactSelectionUnsignedFraction::new(ratio_numerator, ratio_denominator)?;

        let comparison_cross = i64::try_from(ratio_numerator)
            .map_err(|_| SelectionTranslationError::ArithmeticOverflow)?;
        let reference_cross = i64::try_from(ratio_denominator)
            .map_err(|_| SelectionTranslationError::ArithmeticOverflow)?;
        let quantity_numerator = comparison_cross
            .checked_sub(reference_cross)
            .ok_or(SelectionTranslationError::ArithmeticOverflow)?;
        let quantity = ExactSelectionSignedFraction::new(quantity_numerator, ratio_denominator)?;
        (
            RelativeViabilityRatio::Finite(ratio),
            RelativeViabilitySelectionQuantity::Finite(quantity),
        )
    };

    Ok(DerivedSelection {
        reference_survival,
        comparison_survival,
        relative_viability_ratio,
        selection_quantity,
    })
}

fn validate_group_support(
    support: BinaryViabilityGroupSupport,
) -> Result<(), SelectionTranslationError> {
    if support.total == 0 {
        return Err(SelectionTranslationError::DenominatorInvariant);
    }
    let observed = support
        .died_during_window
        .checked_add(support.survived_window)
        .ok_or(SelectionTranslationError::ArithmeticOverflow)?;
    if observed != support.total {
        return Err(SelectionTranslationError::DenominatorInvariant);
    }
    Ok(())
}

fn update_group_support_digest(digest: &mut Sha256, support: BinaryViabilityGroupSupport) {
    put_u64(digest, support.total);
    put_u64(digest, support.died_during_window);
    put_u64(digest, support.survived_window);
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn target_tag(value: CausalSelectionTarget) -> u8 {
    match value {
        CausalSelectionTarget::BinaryViabilityRiskDifference => 0,
    }
}

#[derive(Debug)]
pub enum SelectionTranslationError {
    UnsupportedModelVersion(u32),
    UnsupportedEstimateVersion(u32),
    ModelSpecMismatch,
    ModelSemanticMismatch,
    ResultSemanticMismatch,
    UnsupportedTarget,
    UnsupportedEstimand(SelectionEstimand),
    UnsupportedPredictorRepresentation,
    NonHeritablePredictorSource(PredictorSourceKind),
    EmptyFrame,
    GenerationMappingSubjectMismatch,
    HeritableClassMappingSubjectMismatch,
    QualificationSubjectMismatch,
    ModelReplayMismatch,
    EffectFrameMismatch,
    ModelFrameMismatch,
    DenominatorInvariant,
    ArithmeticOverflow,
    ArithmeticInvariant,
    ResultArithmeticMismatch,
    EstimateReplayMismatch,
}

impl fmt::Display for SelectionTranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedModelVersion(version) => {
                write!(f, "unsupported viability-selection translation model version {version}")
            }
            Self::UnsupportedEstimateVersion(version) => {
                write!(f, "unsupported model-specific selection estimate version {version}")
            }
            Self::ModelSpecMismatch => write!(f, "selection-translation model spec identity mismatch"),
            Self::ModelSemanticMismatch => write!(f, "selection-translation model semantics do not match V1"),
            Self::ResultSemanticMismatch => write!(f, "model-specific selection result semantics do not match V1"),
            Self::UnsupportedTarget => write!(f, "SEL-08A V1 supports only binary viability-risk causal effects"),
            Self::UnsupportedEstimand(estimand) => write!(f, "SEL-08A V1 requires viability-window estimand, got {estimand:?}"),
            Self::UnsupportedPredictorRepresentation => write!(f, "SEL-08A V1 requires binary Reference/Comparison predictor representation"),
            Self::NonHeritablePredictorSource(source) => write!(f, "SEL-08A V1 requires genotype/lineage predictor source, got {source:?}"),
            Self::EmptyFrame => write!(f, "SEL-08A cannot bind an empty analysis frame"),
            Self::GenerationMappingSubjectMismatch => write!(f, "generation/life-cycle mapping authority is bound to another population/context"),
            Self::HeritableClassMappingSubjectMismatch => write!(f, "heritable-class mapping authority is bound to another predictor/context"),
            Self::QualificationSubjectMismatch => write!(f, "selection-model qualification authority is bound to another population/predictor/context"),
            Self::ModelReplayMismatch => write!(f, "persisted selection-translation model does not replay against current authorities"),
            Self::EffectFrameMismatch => write!(f, "causal viability effect is bound to another frame"),
            Self::ModelFrameMismatch => write!(f, "selection-translation model is bound to another frame context or heritable predictor"),
            Self::DenominatorInvariant => write!(f, "model-specific viability denominator invariant failed"),
            Self::ArithmeticOverflow => write!(f, "exact model-specific selection arithmetic overflowed"),
            Self::ArithmeticInvariant => write!(f, "exact model-specific selection arithmetic invariant failed"),
            Self::ResultArithmeticMismatch => write!(f, "persisted model-specific selection quantities do not recompute from stored viability support"),
            Self::EstimateReplayMismatch => write!(f, "persisted model-specific selection estimate does not replay against current authorities"),
        }
    }
}

impl Error for SelectionTranslationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_zero_survival_is_typed_undefined_not_divided_or_corrected() {
        let derived = derive_selection(
            BinaryViabilityGroupSupport {
                total: 2,
                died_during_window: 2,
                survived_window: 0,
            },
            BinaryViabilityGroupSupport {
                total: 3,
                died_during_window: 3,
                survived_window: 0,
            },
        )
        .unwrap();
        assert_eq!(
            derived.relative_viability_ratio,
            RelativeViabilityRatio::UndefinedBothSurvivalsZero
        );
        assert_eq!(
            derived.selection_quantity,
            RelativeViabilitySelectionQuantity::UndefinedBothSurvivalsZero
        );
    }

    #[test]
    fn zero_reference_survival_with_positive_comparison_is_typed_infinity() {
        let derived = derive_selection(
            BinaryViabilityGroupSupport {
                total: 2,
                died_during_window: 2,
                survived_window: 0,
            },
            BinaryViabilityGroupSupport {
                total: 2,
                died_during_window: 0,
                survived_window: 2,
            },
        )
        .unwrap();
        assert_eq!(
            derived.relative_viability_ratio,
            RelativeViabilityRatio::PositiveInfinity
        );
        assert_eq!(
            derived.selection_quantity,
            RelativeViabilitySelectionQuantity::PositiveInfinity
        );
    }
}
