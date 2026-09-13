use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, EvolutionError,
    PopulationGeneration,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const LINEAGE_DIVERGENCE_HISTORY_DESIGN_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:lineage-divergence-history-design:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:evolution:lineage-divergence-history-rule:v1\0";
const RULE_SPEC: &[u8] = b"lineage-divergence history v1: exact inclusive generation coverage; lineage membership evidence binds exact lineage authority plus trajectory-point identity; ancestry, recontact, gene-flow, and fusion evidence bind the ordered lineage pair; qualified lineage fusion has highest classification precedence; explicit qualified loss of either lineage persistence yields not-persistent; otherwise unavailable required generation/evidence is insufficient or fail-closed under the frozen missing-data policy; recontact or realized gene flow without fusion yields divergence-with-recontact; otherwise complete qualified persistent lineage tracks yield persistent-divergence-observed; no species or speciation claim";

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

local_id!(LineageDivergenceHistoryId, "LineageDivergenceHistoryId");
local_id!(LineageHistoryEpisodeId, "LineageHistoryEpisodeId");

pub fn lineage_divergence_history_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("lineage-divergence-history-v1")
            .expect("static lineage history rule method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageHistoryContextPolicy {
    ExactAcrossInterval,
    DeclaredTrajectory { authority: AnalysisAuthorityRef },
}

impl LineageHistoryContextPolicy {
    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::ExactAcrossInterval => digest.update([0]),
            Self::DeclaredTrajectory { authority } => {
                digest.update([1]);
                put_authority(digest, authority);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageHistoryMissingPolicy {
    FailClosed,
    ReportInsufficientEvidence,
}

impl LineageHistoryMissingPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::ReportInsufficientEvidence => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageHistoryEvidenceProtocols {
    pub lineage_membership: AnalysisAuthorityRef,
    pub lineage_persistence: AnalysisAuthorityRef,
    pub ancestry_relation: AnalysisAuthorityRef,
    pub context: AnalysisAuthorityRef,
    pub population_structure: AnalysisAuthorityRef,
    pub demographic_episode_census: AnalysisAuthorityRef,
    pub demographic_episode: AnalysisAuthorityRef,
    pub recontact: AnalysisAuthorityRef,
    pub gene_flow: AnalysisAuthorityRef,
    pub lineage_fusion: AnalysisAuthorityRef,
}

impl LineageHistoryEvidenceProtocols {
    fn put(&self, digest: &mut Sha256) {
        for authority in [
            &self.lineage_membership,
            &self.lineage_persistence,
            &self.ancestry_relation,
            &self.context,
            &self.population_structure,
            &self.demographic_episode_census,
            &self.demographic_episode,
            &self.recontact,
            &self.gene_flow,
            &self.lineage_fusion,
        ] {
            put_authority(digest, authority);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageDivergenceHistoryDesign {
    design_version: u32,
    pub history_id: LineageDivergenceHistoryId,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub generation_count: u64,
    pub protocols: LineageHistoryEvidenceProtocols,
    pub context_policy: LineageHistoryContextPolicy,
    pub missing_policy: LineageHistoryMissingPolicy,
    pub completeness_authority: AnalysisAuthorityRef,
    pub history_rule_authority: AnalysisAuthorityRef,
}

impl LineageDivergenceHistoryDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        history_id: LineageDivergenceHistoryId,
        lineage_a: AnalysisAuthorityRef,
        lineage_b: AnalysisAuthorityRef,
        start_generation: PopulationGeneration,
        end_generation: PopulationGeneration,
        protocols: LineageHistoryEvidenceProtocols,
        context_policy: LineageHistoryContextPolicy,
        missing_policy: LineageHistoryMissingPolicy,
        completeness_authority: AnalysisAuthorityRef,
    ) -> Result<Self, LineageDivergenceDesignError> {
        if lineage_a == lineage_b {
            return Err(LineageDivergenceDesignError::LineageAuthoritiesMustDiffer);
        }
        let generation_count = end_generation
            .0
            .checked_sub(start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(LineageDivergenceDesignError::InvalidGenerationInterval)?;
        if generation_count < 2 {
            return Err(LineageDivergenceDesignError::GenerationIntervalTooShort);
        }
        let design = Self {
            design_version: LINEAGE_DIVERGENCE_HISTORY_DESIGN_VERSION,
            history_id,
            lineage_a,
            lineage_b,
            start_generation,
            end_generation,
            generation_count,
            protocols,
            context_policy,
            missing_policy,
            completeness_authority,
            history_rule_authority: lineage_divergence_history_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<LineageDivergenceHistoryDesignDigest, LineageDivergenceDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.history_id.as_str());
        put_authority(&mut digest, &self.lineage_a);
        put_authority(&mut digest, &self.lineage_b);
        put_u64(&mut digest, self.start_generation.0);
        put_u64(&mut digest, self.end_generation.0);
        put_u64(&mut digest, self.generation_count);
        self.protocols.put(&mut digest);
        self.context_policy.put(&mut digest);
        digest.update([self.missing_policy.tag()]);
        put_authority(&mut digest, &self.completeness_authority);
        put_authority(&mut digest, &self.history_rule_authority);
        Ok(LineageDivergenceHistoryDesignDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), LineageDivergenceDesignError> {
        if self.design_version != LINEAGE_DIVERGENCE_HISTORY_DESIGN_VERSION {
            return Err(LineageDivergenceDesignError::UnsupportedVersion(
                self.design_version,
            ));
        }
        if self.lineage_a == self.lineage_b {
            return Err(LineageDivergenceDesignError::LineageAuthoritiesMustDiffer);
        }
        let expected = self
            .end_generation
            .0
            .checked_sub(self.start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(LineageDivergenceDesignError::InvalidGenerationInterval)?;
        if expected < 2 || expected != self.generation_count {
            return Err(LineageDivergenceDesignError::GenerationCountInvariant);
        }
        if self.history_rule_authority != lineage_divergence_history_rule_v1() {
            return Err(LineageDivergenceDesignError::HistoryRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageDivergenceHistoryDesignDigest([u8; 32]);

impl LineageDivergenceHistoryDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LineageDivergenceHistoryDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LineageDivergenceHistoryDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LineageDivergenceHistoryDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated lineage-history design should gate history materialization"]
pub struct ValidatedLineageDivergenceHistoryDesign<'a> {
    design: &'a LineageDivergenceHistoryDesign,
    design_digest: LineageDivergenceHistoryDesignDigest,
}

impl<'a> ValidatedLineageDivergenceHistoryDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a LineageDivergenceHistoryDesign,
        lineage_a: AnalysisAuthorityRef,
        lineage_b: AnalysisAuthorityRef,
        start_generation: PopulationGeneration,
        end_generation: PopulationGeneration,
        protocols: LineageHistoryEvidenceProtocols,
        context_policy: LineageHistoryContextPolicy,
        missing_policy: LineageHistoryMissingPolicy,
        completeness_authority: AnalysisAuthorityRef,
    ) -> Result<Self, LineageDivergenceDesignError> {
        design.validate_local()?;
        let recomputed = LineageDivergenceHistoryDesign::declare(
            design.history_id.clone(),
            lineage_a,
            lineage_b,
            start_generation,
            end_generation,
            protocols,
            context_policy,
            missing_policy,
            completeness_authority,
        )?;
        if recomputed != *design {
            return Err(LineageDivergenceDesignError::ReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a LineageDivergenceHistoryDesign {
        self.design
    }

    pub fn design_digest(&self) -> LineageDivergenceHistoryDesignDigest {
        self.design_digest
    }
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum LineageDivergenceDesignError {
    UnsupportedVersion(u32),
    LineageAuthoritiesMustDiffer,
    InvalidGenerationInterval,
    GenerationIntervalTooShort,
    GenerationCountInvariant,
    HistoryRuleMismatch,
    ReplayMismatch,
}

impl fmt::Display for LineageDivergenceDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(
                    f,
                    "unsupported lineage-divergence history design version {version}"
                )
            }
            Self::LineageAuthoritiesMustDiffer => {
                write!(f, "lineage A and lineage B authorities must be distinct")
            }
            Self::InvalidGenerationInterval => {
                write!(f, "invalid lineage-history generation interval")
            }
            Self::GenerationIntervalTooShort => write!(
                f,
                "persistent-divergence history requires at least two generation coordinates"
            ),
            Self::GenerationCountInvariant => write!(
                f,
                "persisted lineage-history generation count is inconsistent"
            ),
            Self::HistoryRuleMismatch => write!(
                f,
                "persisted design does not bind the built-in V1 lineage-history rule"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted lineage-history design does not replay against current authorities"
            ),
        }
    }
}

impl Error for LineageDivergenceDesignError {}
