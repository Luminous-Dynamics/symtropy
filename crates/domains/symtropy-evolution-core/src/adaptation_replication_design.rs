use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, EvolutionError, EvolutionaryContextRefDigest,
    ExpectedHeritableResponseDirection, HeritableResponseStudyDesignDigest, PopulationId,
    ValidatedHeritableResponseStudyDesign, ViabilitySelectionTranslationModelDigest,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const ADAPTATION_REPLICATION_DESIGN_VERSION: u32 = 1;
const DESIGN_DOMAIN: &[u8] = b"symtropy:evolution:adaptation-replication-design:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AdaptationReplicationDesignId(String);

impl AdaptationReplicationDesignId {
    pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
        let value = value.into();
        validate_text("AdaptationReplicationDesignId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for AdaptationReplicationDesignId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ReplicationUnitId(String);

impl ReplicationUnitId {
    pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
        let value = value.into();
        validate_text("ReplicationUnitId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ReplicationUnitId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationContextCompatibility {
    RequireSharedSelectionContext,
    AllowDeclaredVariation { authority: AnalysisAuthorityRef },
}

impl ReplicationContextCompatibility {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::RequireSharedSelectionContext => digest.update([0]),
            Self::AllowDeclaredVariation { authority } => {
                digest.update([1]);
                update_authority_digest(digest, authority);
            }
        }
    }
}

#[derive(Debug)]
pub struct ReplicationDesignUnitInput<'a, 'b> {
    pub unit_id: ReplicationUnitId,
    pub study_design: &'a ValidatedHeritableResponseStudyDesign<'b>,
    pub independence_evidence: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationUnitDeclaration {
    pub unit_id: ReplicationUnitId,
    pub study_design_digest: HeritableResponseStudyDesignDigest,
    pub population_id: PopulationId,
    pub selection_context_digest: EvolutionaryContextRefDigest,
    pub generation_count: u64,
    pub independence_evidence: AnalysisAuthorityRef,
}

impl ReplicationUnitDeclaration {
    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.unit_id.as_str());
        digest.update(self.study_design_digest.as_bytes());
        put_text(digest, self.population_id.as_str());
        digest.update(self.selection_context_digest.as_bytes());
        put_u64(digest, self.generation_count);
        update_authority_digest(digest, &self.independence_evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptationReplicationDesign {
    design_version: u32,
    pub design_id: AdaptationReplicationDesignId,
    selection_model_digest: ViabilitySelectionTranslationModelDigest,
    expected_direction: ExpectedHeritableResponseDirection,
    pub units: Vec<ReplicationUnitDeclaration>,
    pub minimum_supported_replicates: u64,
    pub minimum_generation_count: u64,
    pub context_compatibility: ReplicationContextCompatibility,
    pub independence_rule_authority: AnalysisAuthorityRef,
    pub decision_rule_authority: AnalysisAuthorityRef,
}

impl AdaptationReplicationDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare<'a, 'b>(
        design_id: AdaptationReplicationDesignId,
        units: impl IntoIterator<Item = ReplicationDesignUnitInput<'a, 'b>>,
        minimum_supported_replicates: u64,
        minimum_generation_count: u64,
        context_compatibility: ReplicationContextCompatibility,
        independence_rule_authority: AnalysisAuthorityRef,
        decision_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, AdaptationReplicationDesignError> {
        let mut by_id = BTreeMap::new();
        let mut seen_designs = Vec::new();
        let mut common_model = None;
        let mut common_direction = None;
        let mut first_context = None;

        for input in units {
            let b1 = input.study_design.design();
            let digest = input.study_design.design_digest();
            if by_id.contains_key(&input.unit_id) {
                return Err(AdaptationReplicationDesignError::DuplicateReplicationUnit(
                    input.unit_id,
                ));
            }
            if seen_designs.iter().any(|seen| *seen == digest) {
                return Err(AdaptationReplicationDesignError::DuplicateStudyDesignDigest);
            }
            seen_designs.push(digest);

            let model = b1.selection_model_digest();
            match common_model {
                None => common_model = Some(model),
                Some(existing) if existing == model => {}
                Some(_) => return Err(AdaptationReplicationDesignError::SelectionModelMismatch),
            }

            let direction = b1.expected_direction;
            match common_direction {
                None => common_direction = Some(direction),
                Some(existing) if existing == direction => {}
                Some(_) => return Err(AdaptationReplicationDesignError::ExpectedDirectionMismatch),
            }

            let context = b1.selection_context_digest();
            match (&context_compatibility, first_context) {
                (ReplicationContextCompatibility::RequireSharedSelectionContext, None) => {
                    first_context = Some(context);
                }
                (ReplicationContextCompatibility::RequireSharedSelectionContext, Some(existing))
                    if existing == context => {}
                (ReplicationContextCompatibility::RequireSharedSelectionContext, Some(_)) => {
                    return Err(AdaptationReplicationDesignError::SelectionContextMismatch);
                }
                (ReplicationContextCompatibility::AllowDeclaredVariation { .. }, None) => {
                    first_context = Some(context);
                }
                (ReplicationContextCompatibility::AllowDeclaredVariation { .. }, Some(_)) => {}
            }

            let generation_count = b1.generation_count()?;
            if generation_count < minimum_generation_count {
                return Err(AdaptationReplicationDesignError::GenerationSpanTooShort {
                    unit_id: input.unit_id,
                    observed: generation_count,
                    minimum: minimum_generation_count,
                });
            }

            by_id.insert(
                input.unit_id.clone(),
                ReplicationUnitDeclaration {
                    unit_id: input.unit_id,
                    study_design_digest: digest,
                    population_id: b1.population_id().clone(),
                    selection_context_digest: context,
                    generation_count,
                    independence_evidence: input.independence_evidence,
                },
            );
        }

        let units: Vec<_> = by_id.into_values().collect();
        if units.len() < 2 {
            return Err(AdaptationReplicationDesignError::InsufficientDeclaredReplications);
        }
        if minimum_generation_count < 2 {
            return Err(AdaptationReplicationDesignError::MinimumGenerationCountTooSmall);
        }
        let unit_count = u64::try_from(units.len())
            .map_err(|_| AdaptationReplicationDesignError::ArithmeticOverflow)?;
        if minimum_supported_replicates < 2 || minimum_supported_replicates > unit_count {
            return Err(AdaptationReplicationDesignError::InvalidSupportedReplicationThreshold);
        }

        let design = Self {
            design_version: ADAPTATION_REPLICATION_DESIGN_VERSION,
            design_id,
            selection_model_digest: common_model
                .ok_or(AdaptationReplicationDesignError::InsufficientDeclaredReplications)?,
            expected_direction: common_direction
                .ok_or(AdaptationReplicationDesignError::InsufficientDeclaredReplications)?,
            units,
            minimum_supported_replicates,
            minimum_generation_count,
            context_compatibility,
            independence_rule_authority,
            decision_rule_authority,
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn selection_model_digest(&self) -> ViabilitySelectionTranslationModelDigest {
        self.selection_model_digest
    }

    pub fn expected_direction(&self) -> ExpectedHeritableResponseDirection {
        self.expected_direction
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<AdaptationReplicationDesignDigest, AdaptationReplicationDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.design_id.as_str());
        digest.update(self.selection_model_digest.as_bytes());
        digest.update([direction_tag(self.expected_direction)]);
        put_u64(&mut digest, self.units.len() as u64);
        for unit in &self.units {
            unit.update_digest(&mut digest);
        }
        put_u64(&mut digest, self.minimum_supported_replicates);
        put_u64(&mut digest, self.minimum_generation_count);
        self.context_compatibility.update_digest(&mut digest);
        update_authority_digest(&mut digest, &self.independence_rule_authority);
        update_authority_digest(&mut digest, &self.decision_rule_authority);
        Ok(AdaptationReplicationDesignDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), AdaptationReplicationDesignError> {
        if self.design_version != ADAPTATION_REPLICATION_DESIGN_VERSION {
            return Err(AdaptationReplicationDesignError::UnsupportedVersion(
                self.design_version,
            ));
        }
        if self.units.len() < 2 {
            return Err(AdaptationReplicationDesignError::InsufficientDeclaredReplications);
        }
        if self.minimum_generation_count < 2 {
            return Err(AdaptationReplicationDesignError::MinimumGenerationCountTooSmall);
        }
        let unit_count = u64::try_from(self.units.len())
            .map_err(|_| AdaptationReplicationDesignError::ArithmeticOverflow)?;
        if self.minimum_supported_replicates < 2
            || self.minimum_supported_replicates > unit_count
        {
            return Err(AdaptationReplicationDesignError::InvalidSupportedReplicationThreshold);
        }
        if self.units.windows(2).any(|pair| pair[0].unit_id >= pair[1].unit_id) {
            return Err(AdaptationReplicationDesignError::NonCanonicalUnitOrder);
        }
        let mut seen_designs = Vec::new();
        for unit in &self.units {
            if unit.generation_count < self.minimum_generation_count {
                return Err(AdaptationReplicationDesignError::GenerationSpanTooShort {
                    unit_id: unit.unit_id.clone(),
                    observed: unit.generation_count,
                    minimum: self.minimum_generation_count,
                });
            }
            if seen_designs
                .iter()
                .any(|seen| *seen == unit.study_design_digest)
            {
                return Err(AdaptationReplicationDesignError::DuplicateStudyDesignDigest);
            }
            seen_designs.push(unit.study_design_digest);
        }
        if matches!(
            &self.context_compatibility,
            ReplicationContextCompatibility::RequireSharedSelectionContext
        ) {
            let first = self.units[0].selection_context_digest;
            if self
                .units
                .iter()
                .any(|unit| unit.selection_context_digest != first)
            {
                return Err(AdaptationReplicationDesignError::SelectionContextMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AdaptationReplicationDesignDigest([u8; 32]);

impl AdaptationReplicationDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AdaptationReplicationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AdaptationReplicationDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for AdaptationReplicationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated replication design should gate adaptation-evidence capture"]
pub struct ValidatedAdaptationReplicationDesign<'a> {
    design: &'a AdaptationReplicationDesign,
    design_digest: AdaptationReplicationDesignDigest,
}

impl<'a> ValidatedAdaptationReplicationDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current<'b, 'c>(
        design: &'a AdaptationReplicationDesign,
        units: impl IntoIterator<Item = ReplicationDesignUnitInput<'b, 'c>>,
        minimum_supported_replicates: u64,
        minimum_generation_count: u64,
        context_compatibility: ReplicationContextCompatibility,
        independence_rule_authority: AnalysisAuthorityRef,
        decision_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, AdaptationReplicationDesignError> {
        design.validate_local()?;
        let recomputed = AdaptationReplicationDesign::declare(
            design.design_id.clone(),
            units,
            minimum_supported_replicates,
            minimum_generation_count,
            context_compatibility,
            independence_rule_authority,
            decision_rule_authority,
        )?;
        if recomputed != *design {
            return Err(AdaptationReplicationDesignError::ReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a AdaptationReplicationDesign {
        self.design
    }

    pub fn design_digest(&self) -> AdaptationReplicationDesignDigest {
        self.design_digest
    }
}

fn direction_tag(direction: ExpectedHeritableResponseDirection) -> u8 {
    match direction {
        ExpectedHeritableResponseDirection::ComparisonFrequencyIncrease => 0,
        ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease => 1,
    }
}

fn update_authority_digest(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum AdaptationReplicationDesignError {
    B1Design(crate::HeritableResponseDesignError),
    UnsupportedVersion(u32),
    DuplicateReplicationUnit(ReplicationUnitId),
    DuplicateStudyDesignDigest,
    InsufficientDeclaredReplications,
    InvalidSupportedReplicationThreshold,
    MinimumGenerationCountTooSmall,
    SelectionModelMismatch,
    ExpectedDirectionMismatch,
    SelectionContextMismatch,
    GenerationSpanTooShort {
        unit_id: ReplicationUnitId,
        observed: u64,
        minimum: u64,
    },
    NonCanonicalUnitOrder,
    ReplayMismatch,
    ArithmeticOverflow,
}

impl From<crate::HeritableResponseDesignError> for AdaptationReplicationDesignError {
    fn from(value: crate::HeritableResponseDesignError) -> Self {
        Self::B1Design(value)
    }
}

impl fmt::Display for AdaptationReplicationDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::B1Design(error) => write!(f, "B1 study-design error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported adaptation-replication design version {version}")
            }
            Self::DuplicateReplicationUnit(id) => {
                write!(f, "replication unit {} appears more than once", id.as_str())
            }
            Self::DuplicateStudyDesignDigest => write!(
                f,
                "two replication units may not reuse the same B1 study-design digest"
            ),
            Self::InsufficientDeclaredReplications => {
                write!(f, "adaptation replication design requires at least two units")
            }
            Self::InvalidSupportedReplicationThreshold => write!(
                f,
                "supported-replication threshold must be between two and declared unit count"
            ),
            Self::MinimumGenerationCountTooSmall => write!(
                f,
                "minimum generation count must preserve a multi-generation study"
            ),
            Self::SelectionModelMismatch => write!(
                f,
                "replication units must share the exact SEL-08A translation-model digest"
            ),
            Self::ExpectedDirectionMismatch => write!(
                f,
                "replication units must preregister the same expected response direction"
            ),
            Self::SelectionContextMismatch => write!(
                f,
                "replication contexts differ under RequireSharedSelectionContext"
            ),
            Self::GenerationSpanTooShort {
                unit_id,
                observed,
                minimum,
            } => write!(
                f,
                "replication unit {} has {observed} generations; minimum is {minimum}",
                unit_id.as_str()
            ),
            Self::NonCanonicalUnitOrder => {
                write!(f, "replication units are not in canonical unit-id order")
            }
            Self::ReplayMismatch => write!(
                f,
                "persisted replication design does not replay against current B1 designs"
            ),
            Self::ArithmeticOverflow => write!(f, "adaptation-replication arithmetic overflowed"),
        }
    }
}

impl Error for AdaptationReplicationDesignError {}
