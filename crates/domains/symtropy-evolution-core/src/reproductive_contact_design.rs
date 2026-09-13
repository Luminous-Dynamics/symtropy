use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, error::validate_text,
    AnalysisAuthorityRef, EvolutionError, EvolutionIndividualId, EvolutionaryContextRefDigest,
    PopulationGeneration,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const REPRODUCTIVE_CONTACT_STUDY_DESIGN_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:reproductive-contact-study-design:v1\0";

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
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
            }
        }
    };
}
local_id!(ReproductiveContactStudyId, "ReproductiveContactStudyId");
local_id!(ReproductiveOpportunityId, "ReproductiveOpportunityId");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveContactContextPolicy {
    ExactContext { context_digest: EvolutionaryContextRefDigest },
    DeclaredContextTrajectory { authority: AnalysisAuthorityRef },
}
impl ReproductiveContactContextPolicy {
    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::ExactContext { context_digest } => {
                digest.update([0]);
                digest.update(context_digest.as_bytes());
            }
            Self::DeclaredContextTrajectory { authority } => {
                digest.update([1]);
                put_authority(digest, authority);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveOpportunityDeclaration {
    pub opportunity_id: ReproductiveOpportunityId,
    pub generation: PopulationGeneration,
    pub parent_a: EvolutionIndividualId,
    pub parent_b: EvolutionIndividualId,
    pub context_digest: EvolutionaryContextRefDigest,
    pub contact_zone_evidence: AnalysisAuthorityRef,
}
impl ReproductiveOpportunityDeclaration {
    fn validate(&self) -> Result<(), ReproductiveContactDesignError> {
        if self.parent_a == self.parent_b {
            return Err(ReproductiveContactDesignError::SameIndividualPair);
        }
        Ok(())
    }
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.opportunity_id.as_str());
        put_u64(digest, self.generation.0);
        put_text(digest, self.parent_a.as_str());
        put_text(digest, self.parent_b.as_str());
        digest.update(self.context_digest.as_bytes());
        put_authority(digest, &self.contact_zone_evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveContactStudyDesign {
    design_version: u32,
    pub study_id: ReproductiveContactStudyId,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub context_policy: ReproductiveContactContextPolicy,
    pub opportunities: Vec<ReproductiveOpportunityDeclaration>,
    pub opportunity_definition_authority: AnalysisAuthorityRef,
    pub contact_protocol: AnalysisAuthorityRef,
    pub pairing_protocol: AnalysisAuthorityRef,
    pub mating_protocol: AnalysisAuthorityRef,
    pub conception_protocol: AnalysisAuthorityRef,
    pub offspring_viability_protocol: AnalysisAuthorityRef,
    pub offspring_fertility_protocol: AnalysisAuthorityRef,
    pub parentage_authority: AnalysisAuthorityRef,
    pub gene_flow_materialization_authority: AnalysisAuthorityRef,
    pub demography_accounting_authority: AnalysisAuthorityRef,
    pub missing_data_authority: AnalysisAuthorityRef,
}

impl ReproductiveContactStudyDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        study_id: ReproductiveContactStudyId,
        lineage_a: AnalysisAuthorityRef,
        lineage_b: AnalysisAuthorityRef,
        start_generation: PopulationGeneration,
        end_generation: PopulationGeneration,
        context_policy: ReproductiveContactContextPolicy,
        opportunities: impl IntoIterator<Item = ReproductiveOpportunityDeclaration>,
        opportunity_definition_authority: AnalysisAuthorityRef,
        contact_protocol: AnalysisAuthorityRef,
        pairing_protocol: AnalysisAuthorityRef,
        mating_protocol: AnalysisAuthorityRef,
        conception_protocol: AnalysisAuthorityRef,
        offspring_viability_protocol: AnalysisAuthorityRef,
        offspring_fertility_protocol: AnalysisAuthorityRef,
        parentage_authority: AnalysisAuthorityRef,
        gene_flow_materialization_authority: AnalysisAuthorityRef,
        demography_accounting_authority: AnalysisAuthorityRef,
        missing_data_authority: AnalysisAuthorityRef,
    ) -> Result<Self, ReproductiveContactDesignError> {
        let mut by_id = BTreeMap::new();
        for opportunity in opportunities {
            let id = opportunity.opportunity_id.clone();
            if by_id.insert(id.clone(), opportunity).is_some() {
                return Err(ReproductiveContactDesignError::DuplicateOpportunity(id));
            }
        }
        let design = Self {
            design_version: REPRODUCTIVE_CONTACT_STUDY_DESIGN_VERSION,
            study_id, lineage_a, lineage_b, start_generation, end_generation, context_policy,
            opportunities: by_id.into_values().collect(), opportunity_definition_authority,
            contact_protocol, pairing_protocol, mating_protocol, conception_protocol,
            offspring_viability_protocol, offspring_fertility_protocol, parentage_authority,
            gene_flow_materialization_authority, demography_accounting_authority,
            missing_data_authority,
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(&self) -> Result<ReproductiveContactStudyDesignDigest, ReproductiveContactDesignError> {
        self.validate_local()?;
        let mut d = Sha256::new();
        d.update(DOMAIN); put_u32(&mut d, self.design_version); put_text(&mut d, self.study_id.as_str());
        put_authority(&mut d, &self.lineage_a); put_authority(&mut d, &self.lineage_b);
        put_u64(&mut d, self.start_generation.0); put_u64(&mut d, self.end_generation.0);
        self.context_policy.put(&mut d); put_u64(&mut d, self.opportunities.len() as u64);
        for opportunity in &self.opportunities { opportunity.put(&mut d); }
        for authority in [
            &self.opportunity_definition_authority, &self.contact_protocol, &self.pairing_protocol,
            &self.mating_protocol, &self.conception_protocol, &self.offspring_viability_protocol,
            &self.offspring_fertility_protocol, &self.parentage_authority,
            &self.gene_flow_materialization_authority, &self.demography_accounting_authority,
            &self.missing_data_authority,
        ] { put_authority(&mut d, authority); }
        Ok(ReproductiveContactStudyDesignDigest(d.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ReproductiveContactDesignError> {
        if self.design_version != REPRODUCTIVE_CONTACT_STUDY_DESIGN_VERSION {
            return Err(ReproductiveContactDesignError::UnsupportedVersion(self.design_version));
        }
        if self.lineage_a == self.lineage_b { return Err(ReproductiveContactDesignError::SameLineageIdentity); }
        if self.end_generation.0 < self.start_generation.0 { return Err(ReproductiveContactDesignError::InvalidGenerationInterval); }
        if self.opportunities.is_empty() { return Err(ReproductiveContactDesignError::EmptyOpportunityCensus); }
        if self.opportunities.windows(2).any(|pair| pair[0].opportunity_id >= pair[1].opportunity_id) {
            return Err(ReproductiveContactDesignError::NonCanonicalOpportunityOrder);
        }
        for opportunity in &self.opportunities {
            opportunity.validate()?;
            if opportunity.generation.0 < self.start_generation.0 || opportunity.generation.0 > self.end_generation.0 {
                return Err(ReproductiveContactDesignError::OpportunityOutsideInterval(opportunity.opportunity_id.clone()));
            }
            if let ReproductiveContactContextPolicy::ExactContext { context_digest } = &self.context_policy {
                if opportunity.context_digest != *context_digest {
                    return Err(ReproductiveContactDesignError::UndeclaredContextDrift(opportunity.opportunity_id.clone()));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductiveContactStudyDesignDigest([u8; 32]);
impl ReproductiveContactStudyDesignDigest { pub fn as_bytes(&self) -> &[u8; 32] { &self.0 } }
impl fmt::Debug for ReproductiveContactStudyDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "ReproductiveContactStudyDesignDigest(")?; fmt_hex(&self.0, f)?; write!(f, ")") }
}
impl fmt::Display for ReproductiveContactStudyDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_hex(&self.0, f) }
}

#[derive(Debug)]
#[must_use = "validated reproductive-contact design should gate outcome capture"]
pub struct ValidatedReproductiveContactStudyDesign<'a> {
    design: &'a ReproductiveContactStudyDesign,
    design_digest: ReproductiveContactStudyDesignDigest,
}
impl<'a> ValidatedReproductiveContactStudyDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a ReproductiveContactStudyDesign,
        lineage_a: AnalysisAuthorityRef, lineage_b: AnalysisAuthorityRef,
        context_policy: ReproductiveContactContextPolicy,
        opportunities: impl IntoIterator<Item = ReproductiveOpportunityDeclaration>,
        opportunity_definition_authority: AnalysisAuthorityRef, contact_protocol: AnalysisAuthorityRef,
        pairing_protocol: AnalysisAuthorityRef, mating_protocol: AnalysisAuthorityRef,
        conception_protocol: AnalysisAuthorityRef, offspring_viability_protocol: AnalysisAuthorityRef,
        offspring_fertility_protocol: AnalysisAuthorityRef, parentage_authority: AnalysisAuthorityRef,
        gene_flow_materialization_authority: AnalysisAuthorityRef,
        demography_accounting_authority: AnalysisAuthorityRef, missing_data_authority: AnalysisAuthorityRef,
    ) -> Result<Self, ReproductiveContactDesignError> {
        design.validate_local()?;
        let recomputed = ReproductiveContactStudyDesign::declare(
            design.study_id.clone(), lineage_a, lineage_b, design.start_generation, design.end_generation,
            context_policy, opportunities, opportunity_definition_authority, contact_protocol,
            pairing_protocol, mating_protocol, conception_protocol, offspring_viability_protocol,
            offspring_fertility_protocol, parentage_authority, gene_flow_materialization_authority,
            demography_accounting_authority, missing_data_authority,
        )?;
        if recomputed != *design { return Err(ReproductiveContactDesignError::ReplayMismatch); }
        Ok(Self { design, design_digest: design.canonical_digest()? })
    }
    pub fn design(&self) -> &'a ReproductiveContactStudyDesign { self.design }
    pub fn design_digest(&self) -> ReproductiveContactStudyDesignDigest { self.design_digest }
}

fn put_authority(d: &mut Sha256, a: &AnalysisAuthorityRef) {
    put_text(d, a.method_id.as_str()); put_u64(d, a.revision); d.update(a.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum ReproductiveContactDesignError {
    UnsupportedVersion(u32), SameLineageIdentity, SameIndividualPair, InvalidGenerationInterval,
    EmptyOpportunityCensus, DuplicateOpportunity(ReproductiveOpportunityId),
    OpportunityOutsideInterval(ReproductiveOpportunityId), UndeclaredContextDrift(ReproductiveOpportunityId),
    NonCanonicalOpportunityOrder, ReplayMismatch,
}
impl fmt::Display for ReproductiveContactDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(v) => write!(f, "unsupported reproductive-contact design version {v}"),
            Self::SameLineageIdentity => write!(f, "contact study requires two distinct lineage identities"),
            Self::SameIndividualPair => write!(f, "cross-lineage opportunity cannot pair an individual with itself"),
            Self::InvalidGenerationInterval => write!(f, "reproductive-contact generation interval is invalid"),
            Self::EmptyOpportunityCensus => write!(f, "reproductive-contact design requires at least one preregistered opportunity"),
            Self::DuplicateOpportunity(id) => write!(f, "opportunity {} is declared more than once", id.as_str()),
            Self::OpportunityOutsideInterval(id) => write!(f, "opportunity {} lies outside the preregistered interval", id.as_str()),
            Self::UndeclaredContextDrift(id) => write!(f, "opportunity {} changes context under ExactContext", id.as_str()),
            Self::NonCanonicalOpportunityOrder => write!(f, "opportunity declarations are not in canonical ID order"),
            Self::ReplayMismatch => write!(f, "persisted reproductive-contact design does not replay against current authorities"),
        }
    }
}
impl Error for ReproductiveContactDesignError {}
