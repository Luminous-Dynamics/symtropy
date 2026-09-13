use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, BiologicalSpeciesModelDigest,
    CurrentSpeciesClassificationDesignDigest, EvolutionError, LineageDivergenceHistoryDesignDigest,
    PopulationGeneration, ReproductiveIsolationDesignDigest, SpeciesModelContentDigest,
    SpeciesModelValidityDomainDigest, ValidatedBiologicalSpeciesModel,
    ValidatedCurrentSpeciesClassificationDesign, ValidatedLineageDivergenceHistoryDesign,
    ValidatedReproductiveIsolationDesign,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const SPECIATION_TRANSITION_DESIGN_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:speciation-transition-design:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:evolution:historical-speciation-transition-rule:v1\0";
const RULE_SPEC: &[u8] = b"historical speciation transition v1: preregister an interval spanning at least two generations with at least one lineage-history generation before and after; bind exact SEL-10A, SEL-09B, SEL-10B, and SEL-10C design/model identities; require separately qualified temporal evidence for pre-transition common source, divergence timing, reproductive-barrier timing, demographic history, interval completeness, model applicability, and later counter-history; later recontact, introgression, barrier collapse, or fusion remain explicit counter-history and do not erase an earlier supported transition; current species status alone never establishes a historical transition; no exact transition generation is representable";

macro_rules! local_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
                let value = value.into();
                validate_text($field, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
            }
        }
    };
}

local_id!(SpeciationTransitionDesignId, "SpeciationTransitionDesignId");

pub fn historical_speciation_transition_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("historical-speciation-transition-v1")
            .expect("static transition rule method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciationTransitionEvidenceProtocols {
    pub pre_transition_common_source: AnalysisAuthorityRef,
    pub divergence_timing: AnalysisAuthorityRef,
    pub reproductive_barrier_timing: AnalysisAuthorityRef,
    pub demographic_history: AnalysisAuthorityRef,
    pub interval_completeness: AnalysisAuthorityRef,
    pub model_applicability: AnalysisAuthorityRef,
    pub later_counter_history: AnalysisAuthorityRef,
    pub qualification: AnalysisAuthorityRef,
}

impl SpeciationTransitionEvidenceProtocols {
    fn put(&self, digest: &mut Sha256) {
        for authority in [
            &self.pre_transition_common_source,
            &self.divergence_timing,
            &self.reproductive_barrier_timing,
            &self.demographic_history,
            &self.interval_completeness,
            &self.model_applicability,
            &self.later_counter_history,
            &self.qualification,
        ] {
            put_authority(digest, authority);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciationTransitionMissingPolicy {
    FailClosed,
    ReportInsufficientTemporalEvidence,
}

impl SpeciationTransitionMissingPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::ReportInsufficientTemporalEvidence => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciationTransitionDesign {
    design_version: u32,
    pub transition_id: SpeciationTransitionDesignId,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
    pub reproductive_isolation_design_digest: ReproductiveIsolationDesignDigest,
    pub current_species_design_digest: CurrentSpeciesClassificationDesignDigest,
    pub species_model_digest: BiologicalSpeciesModelDigest,
    pub species_model_content_digest: SpeciesModelContentDigest,
    pub validity_domain_digest: SpeciesModelValidityDomainDigest,
    pub history_start_generation: PopulationGeneration,
    pub history_end_generation: PopulationGeneration,
    pub candidate_start_generation: PopulationGeneration,
    pub candidate_end_generation: PopulationGeneration,
    pub candidate_generation_count: u64,
    pub pre_transition_generation: PopulationGeneration,
    pub post_transition_generation: PopulationGeneration,
    pub protocols: SpeciationTransitionEvidenceProtocols,
    pub missing_policy: SpeciationTransitionMissingPolicy,
    pub transition_rule_authority: AnalysisAuthorityRef,
}

impl SpeciationTransitionDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        transition_id: SpeciationTransitionDesignId,
        lineage_history: &ValidatedLineageDivergenceHistoryDesign<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationDesign<'_>,
        current_species: &ValidatedCurrentSpeciesClassificationDesign<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        candidate_start_generation: PopulationGeneration,
        candidate_end_generation: PopulationGeneration,
        protocols: SpeciationTransitionEvidenceProtocols,
        missing_policy: SpeciationTransitionMissingPolicy,
    ) -> Result<Self, SpeciationTransitionDesignError> {
        let history = lineage_history.design();
        let isolation = reproductive_isolation.design();
        let current = current_species.design();
        let model = species_model.model();

        if history.lineage_a != isolation.lineage_a
            || history.lineage_b != isolation.lineage_b
            || history.lineage_a != current.lineage_a
            || history.lineage_b != current.lineage_b
        {
            return Err(SpeciationTransitionDesignError::LineagePairMismatch);
        }
        if current.lineage_history_design_digest != lineage_history.design_digest()
            || current.reproductive_isolation_design_digest != reproductive_isolation.design_digest()
        {
            return Err(SpeciationTransitionDesignError::CurrentSpeciesDesignMismatch);
        }
        if current.species_model_digest != species_model.model_digest()
            || current.species_model_content_digest != model.model_content_digest
            || current.validity_domain_digest != model.validity_domain.canonical_digest()
        {
            return Err(SpeciationTransitionDesignError::SpeciesModelMismatch);
        }

        let candidate_generation_count = candidate_end_generation
            .0
            .checked_sub(candidate_start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(SpeciationTransitionDesignError::InvalidCandidateInterval)?;
        if candidate_generation_count < 2 {
            return Err(SpeciationTransitionDesignError::ExactTransitionGenerationForbidden);
        }
        if candidate_start_generation.0 <= history.start_generation.0
            || candidate_end_generation.0 >= history.end_generation.0
        {
            return Err(SpeciationTransitionDesignError::MissingBeforeAfterCoverage);
        }
        let pre_transition_generation = PopulationGeneration(
            candidate_start_generation
                .0
                .checked_sub(1)
                .ok_or(SpeciationTransitionDesignError::ArithmeticOverflow)?,
        );
        let post_transition_generation = PopulationGeneration(
            candidate_end_generation
                .0
                .checked_add(1)
                .ok_or(SpeciationTransitionDesignError::ArithmeticOverflow)?,
        );

        let design = Self {
            design_version: SPECIATION_TRANSITION_DESIGN_VERSION,
            transition_id,
            lineage_a: history.lineage_a.clone(),
            lineage_b: history.lineage_b.clone(),
            lineage_history_design_digest: lineage_history.design_digest(),
            reproductive_isolation_design_digest: reproductive_isolation.design_digest(),
            current_species_design_digest: current_species.design_digest(),
            species_model_digest: species_model.model_digest(),
            species_model_content_digest: model.model_content_digest,
            validity_domain_digest: model.validity_domain.canonical_digest(),
            history_start_generation: history.start_generation,
            history_end_generation: history.end_generation,
            candidate_start_generation,
            candidate_end_generation,
            candidate_generation_count,
            pre_transition_generation,
            post_transition_generation,
            protocols,
            missing_policy,
            transition_rule_authority: historical_speciation_transition_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<SpeciationTransitionDesignDigest, SpeciationTransitionDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.transition_id.as_str());
        put_authority(&mut digest, &self.lineage_a);
        put_authority(&mut digest, &self.lineage_b);
        digest.update(self.lineage_history_design_digest.as_bytes());
        digest.update(self.reproductive_isolation_design_digest.as_bytes());
        digest.update(self.current_species_design_digest.as_bytes());
        digest.update(self.species_model_digest.as_bytes());
        digest.update(self.species_model_content_digest.as_bytes());
        digest.update(self.validity_domain_digest.as_bytes());
        put_u64(&mut digest, self.history_start_generation.0);
        put_u64(&mut digest, self.history_end_generation.0);
        put_u64(&mut digest, self.candidate_start_generation.0);
        put_u64(&mut digest, self.candidate_end_generation.0);
        put_u64(&mut digest, self.candidate_generation_count);
        put_u64(&mut digest, self.pre_transition_generation.0);
        put_u64(&mut digest, self.post_transition_generation.0);
        self.protocols.put(&mut digest);
        digest.update([self.missing_policy.tag()]);
        put_authority(&mut digest, &self.transition_rule_authority);
        Ok(SpeciationTransitionDesignDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), SpeciationTransitionDesignError> {
        if self.design_version != SPECIATION_TRANSITION_DESIGN_VERSION {
            return Err(SpeciationTransitionDesignError::UnsupportedVersion(
                self.design_version,
            ));
        }
        if self.lineage_a == self.lineage_b {
            return Err(SpeciationTransitionDesignError::LineagePairMismatch);
        }
        if self.history_start_generation.0 >= self.history_end_generation.0 {
            return Err(SpeciationTransitionDesignError::InvalidHistoryInterval);
        }
        let expected_count = self
            .candidate_end_generation
            .0
            .checked_sub(self.candidate_start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(SpeciationTransitionDesignError::InvalidCandidateInterval)?;
        if expected_count < 2 {
            return Err(SpeciationTransitionDesignError::ExactTransitionGenerationForbidden);
        }
        if expected_count != self.candidate_generation_count {
            return Err(SpeciationTransitionDesignError::CandidateGenerationCountInvariant);
        }
        if self.candidate_start_generation.0 <= self.history_start_generation.0
            || self.candidate_end_generation.0 >= self.history_end_generation.0
        {
            return Err(SpeciationTransitionDesignError::MissingBeforeAfterCoverage);
        }
        if self.pre_transition_generation.0.checked_add(1)
            != Some(self.candidate_start_generation.0)
            || self.candidate_end_generation.0.checked_add(1)
                != Some(self.post_transition_generation.0)
        {
            return Err(SpeciationTransitionDesignError::BeforeAfterGenerationInvariant);
        }
        if self.pre_transition_generation.0 < self.history_start_generation.0
            || self.post_transition_generation.0 > self.history_end_generation.0
        {
            return Err(SpeciationTransitionDesignError::BeforeAfterOutsideHistory);
        }
        if self.transition_rule_authority != historical_speciation_transition_rule_v1() {
            return Err(SpeciationTransitionDesignError::TransitionRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciationTransitionDesignDigest([u8; 32]);

impl SpeciationTransitionDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciationTransitionDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciationTransitionDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciationTransitionDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated transition design should gate historical transition evaluation"]
pub struct ValidatedSpeciationTransitionDesign<'a> {
    design: &'a SpeciationTransitionDesign,
    design_digest: SpeciationTransitionDesignDigest,
}

impl<'a> ValidatedSpeciationTransitionDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a SpeciationTransitionDesign,
        lineage_history: &ValidatedLineageDivergenceHistoryDesign<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationDesign<'_>,
        current_species: &ValidatedCurrentSpeciesClassificationDesign<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        protocols: SpeciationTransitionEvidenceProtocols,
        missing_policy: SpeciationTransitionMissingPolicy,
    ) -> Result<Self, SpeciationTransitionDesignError> {
        design.validate_local()?;
        let recomputed = SpeciationTransitionDesign::declare(
            design.transition_id.clone(),
            lineage_history,
            reproductive_isolation,
            current_species,
            species_model,
            design.candidate_start_generation,
            design.candidate_end_generation,
            protocols,
            missing_policy,
        )?;
        if recomputed != *design {
            return Err(SpeciationTransitionDesignError::ReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a SpeciationTransitionDesign {
        self.design
    }

    pub fn design_digest(&self) -> SpeciationTransitionDesignDigest {
        self.design_digest
    }
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum SpeciationTransitionDesignError {
    UnsupportedVersion(u32),
    LineagePairMismatch,
    CurrentSpeciesDesignMismatch,
    SpeciesModelMismatch,
    InvalidHistoryInterval,
    InvalidCandidateInterval,
    ExactTransitionGenerationForbidden,
    MissingBeforeAfterCoverage,
    CandidateGenerationCountInvariant,
    BeforeAfterGenerationInvariant,
    BeforeAfterOutsideHistory,
    TransitionRuleMismatch,
    ArithmeticOverflow,
    ReplayMismatch,
}

impl fmt::Display for SpeciationTransitionDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported speciation-transition design version {version}")
            }
            Self::LineagePairMismatch => write!(
                f,
                "SEL-10A, SEL-09B, and SEL-10C do not bind the same ordered lineage pair"
            ),
            Self::CurrentSpeciesDesignMismatch => write!(
                f,
                "SEL-10C does not bind the supplied SEL-10A/SEL-09B designs"
            ),
            Self::SpeciesModelMismatch => write!(
                f,
                "SEL-10C and SEL-10B species-model identities do not match"
            ),
            Self::InvalidHistoryInterval => write!(f, "invalid stored SEL-10A history interval"),
            Self::InvalidCandidateInterval => write!(f, "invalid candidate transition interval"),
            Self::ExactTransitionGenerationForbidden => write!(
                f,
                "SEL-10D V1 forbids a one-generation exact transition claim; use a genuine interval"
            ),
            Self::MissingBeforeAfterCoverage => write!(
                f,
                "candidate transition interval must leave at least one SEL-10A generation before and after"
            ),
            Self::CandidateGenerationCountInvariant => write!(
                f,
                "stored candidate transition generation count is inconsistent with its bounds"
            ),
            Self::BeforeAfterGenerationInvariant => write!(
                f,
                "stored before/after generations are not immediately adjacent to the candidate interval"
            ),
            Self::BeforeAfterOutsideHistory => write!(
                f,
                "stored before/after generations are outside the frozen SEL-10A history interval"
            ),
            Self::TransitionRuleMismatch => write!(
                f,
                "persisted design does not bind the built-in historical transition V1 rule"
            ),
            Self::ArithmeticOverflow => write!(f, "speciation-transition generation arithmetic overflow"),
            Self::ReplayMismatch => write!(
                f,
                "persisted speciation-transition design does not replay against current upstream designs/model"
            ),
        }
    }
}

impl Error for SpeciationTransitionDesignError {}
