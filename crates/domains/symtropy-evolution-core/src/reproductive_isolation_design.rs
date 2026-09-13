use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, error::validate_text,
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, EvolutionError,
    ReproductiveContactContextPolicy, ReproductiveContactStudyDesignDigest,
    ValidatedReproductiveContactStudyDesign,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const REPRODUCTIVE_ISOLATION_DESIGN_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:reproductive-isolation-design:v1\0";
const BARRIER_RULE_DOMAIN: &[u8] =
    b"symtropy:evolution:complete-reproductive-isolation-barrier-rule:v1\0";
const BARRIER_RULE_SPEC: &[u8] = b"complete-barrier evidence v1: no-contact never supports isolation; observed contact terminating before pairing, mating, conception, viable offspring, or fertility may support the corresponding barrier component; viable infertile offspring supports a postzygotic fertility barrier; viable fertile offspring or realized hereditary gene flow contradicts complete isolation; viable offspring with unknown fertility or unavailable reproductive/gene-flow evidence is insufficient; support requires preregistered minimum observed-contact opportunities and minimum independent barrier-supporting studies; no species or speciation claim";

pub fn complete_reproductive_isolation_barrier_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(BARRIER_RULE_DOMAIN);
    put_u64(&mut digest, BARRIER_RULE_SPEC.len() as u64);
    digest.update(BARRIER_RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("complete-reproductive-isolation-barrier-v1")
            .expect("static isolation rule method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

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

local_id!(ReproductiveIsolationDesignId, "ReproductiveIsolationDesignId");
local_id!(IsolationStudyUnitId, "IsolationStudyUnitId");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationContextCompatibility {
    RequireSharedContextPolicy,
    AllowDeclaredVariation { authority: AnalysisAuthorityRef },
}

impl IsolationContextCompatibility {
    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::RequireSharedContextPolicy => digest.update([0]),
            Self::AllowDeclaredVariation { authority } => {
                digest.update([1]);
                put_authority(digest, authority);
            }
        }
    }
}

#[derive(Debug)]
pub struct IsolationStudyDesignInput<'a, 'b> {
    pub unit_id: IsolationStudyUnitId,
    pub contact_design: &'a ValidatedReproductiveContactStudyDesign<'b>,
    pub independence_evidence: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationStudyDeclaration {
    pub unit_id: IsolationStudyUnitId,
    pub contact_design_digest: ReproductiveContactStudyDesignDigest,
    pub start_generation: crate::PopulationGeneration,
    pub end_generation: crate::PopulationGeneration,
    pub generation_count: u64,
    pub opportunity_count: u64,
    pub context_policy: ReproductiveContactContextPolicy,
    pub independence_evidence: AnalysisAuthorityRef,
}

impl IsolationStudyDeclaration {
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.unit_id.as_str());
        digest.update(self.contact_design_digest.as_bytes());
        put_u64(digest, self.start_generation.0);
        put_u64(digest, self.end_generation.0);
        put_u64(digest, self.generation_count);
        put_u64(digest, self.opportunity_count);
        put_context_policy(digest, &self.context_policy);
        put_authority(digest, &self.independence_evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveIsolationDesign {
    design_version: u32,
    pub design_id: ReproductiveIsolationDesignId,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub studies: Vec<IsolationStudyDeclaration>,
    pub minimum_barrier_supporting_studies: u64,
    pub minimum_observed_contact_opportunities: u64,
    pub minimum_generation_count_per_study: u64,
    pub context_compatibility: IsolationContextCompatibility,
    pub independence_rule_authority: AnalysisAuthorityRef,
    pub barrier_rule_authority: AnalysisAuthorityRef,
}

impl ReproductiveIsolationDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare<'a, 'b>(
        design_id: ReproductiveIsolationDesignId,
        studies: impl IntoIterator<Item = IsolationStudyDesignInput<'a, 'b>>,
        minimum_barrier_supporting_studies: u64,
        minimum_observed_contact_opportunities: u64,
        minimum_generation_count_per_study: u64,
        context_compatibility: IsolationContextCompatibility,
        independence_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, ReproductiveIsolationDesignError> {
        if minimum_generation_count_per_study < 2 {
            return Err(ReproductiveIsolationDesignError::MinimumGenerationCountTooSmall);
        }

        let mut by_id = BTreeMap::new();
        let mut seen_designs = Vec::new();
        let mut lineage_pair: Option<(AnalysisAuthorityRef, AnalysisAuthorityRef)> = None;
        let mut shared_context_policy: Option<ReproductiveContactContextPolicy> = None;

        for input in studies {
            let contact = input.contact_design.design();
            let digest = input.contact_design.design_digest();
            if by_id.contains_key(&input.unit_id) {
                return Err(ReproductiveIsolationDesignError::DuplicateStudyUnit(
                    input.unit_id,
                ));
            }
            if seen_designs.iter().any(|seen| *seen == digest) {
                return Err(ReproductiveIsolationDesignError::DuplicateContactDesignDigest);
            }
            seen_designs.push(digest);

            match &lineage_pair {
                None => lineage_pair = Some((contact.lineage_a.clone(), contact.lineage_b.clone())),
                Some((lineage_a, lineage_b))
                    if lineage_a == &contact.lineage_a && lineage_b == &contact.lineage_b => {}
                Some(_) => return Err(ReproductiveIsolationDesignError::LineagePairMismatch),
            }

            if matches!(
                &context_compatibility,
                IsolationContextCompatibility::RequireSharedContextPolicy
            ) {
                match &shared_context_policy {
                    None => shared_context_policy = Some(contact.context_policy.clone()),
                    Some(existing) if existing == &contact.context_policy => {}
                    Some(_) => {
                        return Err(ReproductiveIsolationDesignError::ContextPolicyMismatch)
                    }
                }
            }

            let generation_count = contact
                .end_generation
                .0
                .checked_sub(contact.start_generation.0)
                .and_then(|delta| delta.checked_add(1))
                .ok_or(ReproductiveIsolationDesignError::ArithmeticOverflow)?;
            if generation_count < minimum_generation_count_per_study {
                return Err(ReproductiveIsolationDesignError::GenerationSpanTooShort {
                    unit_id: input.unit_id,
                    observed: generation_count,
                    minimum: minimum_generation_count_per_study,
                });
            }
            let opportunity_count = u64::try_from(contact.opportunities.len())
                .map_err(|_| ReproductiveIsolationDesignError::ArithmeticOverflow)?;

            by_id.insert(
                input.unit_id.clone(),
                IsolationStudyDeclaration {
                    unit_id: input.unit_id,
                    contact_design_digest: digest,
                    start_generation: contact.start_generation,
                    end_generation: contact.end_generation,
                    generation_count,
                    opportunity_count,
                    context_policy: contact.context_policy.clone(),
                    independence_evidence: input.independence_evidence,
                },
            );
        }

        let studies: Vec<_> = by_id.into_values().collect();
        if studies.len() < 2 {
            return Err(ReproductiveIsolationDesignError::InsufficientDeclaredStudies);
        }
        let study_count = u64::try_from(studies.len())
            .map_err(|_| ReproductiveIsolationDesignError::ArithmeticOverflow)?;
        if minimum_barrier_supporting_studies < 2
            || minimum_barrier_supporting_studies > study_count
        {
            return Err(ReproductiveIsolationDesignError::InvalidBarrierStudyThreshold);
        }
        if minimum_observed_contact_opportunities < minimum_barrier_supporting_studies {
            return Err(ReproductiveIsolationDesignError::InvalidContactOpportunityThreshold);
        }

        let (lineage_a, lineage_b) =
            lineage_pair.ok_or(ReproductiveIsolationDesignError::InsufficientDeclaredStudies)?;
        let design = Self {
            design_version: REPRODUCTIVE_ISOLATION_DESIGN_VERSION,
            design_id,
            lineage_a,
            lineage_b,
            studies,
            minimum_barrier_supporting_studies,
            minimum_observed_contact_opportunities,
            minimum_generation_count_per_study,
            context_compatibility,
            independence_rule_authority,
            barrier_rule_authority: complete_reproductive_isolation_barrier_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ReproductiveIsolationDesignDigest, ReproductiveIsolationDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.design_id.as_str());
        put_authority(&mut digest, &self.lineage_a);
        put_authority(&mut digest, &self.lineage_b);
        put_u64(&mut digest, self.studies.len() as u64);
        for study in &self.studies {
            study.put(&mut digest);
        }
        put_u64(&mut digest, self.minimum_barrier_supporting_studies);
        put_u64(&mut digest, self.minimum_observed_contact_opportunities);
        put_u64(&mut digest, self.minimum_generation_count_per_study);
        self.context_compatibility.put(&mut digest);
        put_authority(&mut digest, &self.independence_rule_authority);
        put_authority(&mut digest, &self.barrier_rule_authority);
        Ok(ReproductiveIsolationDesignDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ReproductiveIsolationDesignError> {
        if self.design_version != REPRODUCTIVE_ISOLATION_DESIGN_VERSION {
            return Err(ReproductiveIsolationDesignError::UnsupportedVersion(
                self.design_version,
            ));
        }
        if self.barrier_rule_authority != complete_reproductive_isolation_barrier_rule_v1() {
            return Err(ReproductiveIsolationDesignError::BarrierRuleMismatch);
        }
        if self.lineage_a == self.lineage_b {
            return Err(ReproductiveIsolationDesignError::LineagePairMismatch);
        }
        if self.studies.len() < 2 {
            return Err(ReproductiveIsolationDesignError::InsufficientDeclaredStudies);
        }
        if self.studies.windows(2).any(|pair| pair[0].unit_id >= pair[1].unit_id) {
            return Err(ReproductiveIsolationDesignError::NonCanonicalStudyOrder);
        }
        if self.minimum_generation_count_per_study < 2 {
            return Err(ReproductiveIsolationDesignError::MinimumGenerationCountTooSmall);
        }
        let study_count = u64::try_from(self.studies.len())
            .map_err(|_| ReproductiveIsolationDesignError::ArithmeticOverflow)?;
        if self.minimum_barrier_supporting_studies < 2
            || self.minimum_barrier_supporting_studies > study_count
        {
            return Err(ReproductiveIsolationDesignError::InvalidBarrierStudyThreshold);
        }
        if self.minimum_observed_contact_opportunities < self.minimum_barrier_supporting_studies {
            return Err(ReproductiveIsolationDesignError::InvalidContactOpportunityThreshold);
        }

        let mut seen_designs = Vec::new();
        for study in &self.studies {
            if seen_designs
                .iter()
                .any(|seen| *seen == study.contact_design_digest)
            {
                return Err(ReproductiveIsolationDesignError::DuplicateContactDesignDigest);
            }
            seen_designs.push(study.contact_design_digest);
            if study.generation_count < self.minimum_generation_count_per_study {
                return Err(ReproductiveIsolationDesignError::GenerationSpanTooShort {
                    unit_id: study.unit_id.clone(),
                    observed: study.generation_count,
                    minimum: self.minimum_generation_count_per_study,
                });
            }
            if study.opportunity_count == 0 {
                return Err(ReproductiveIsolationDesignError::EmptyContactOpportunityCensus);
            }
            let expected_generation_count = study
                .end_generation
                .0
                .checked_sub(study.start_generation.0)
                .and_then(|delta| delta.checked_add(1))
                .ok_or(ReproductiveIsolationDesignError::ArithmeticOverflow)?;
            if expected_generation_count != study.generation_count {
                return Err(ReproductiveIsolationDesignError::GenerationCountInvariant);
            }
        }

        if matches!(
            &self.context_compatibility,
            IsolationContextCompatibility::RequireSharedContextPolicy
        ) {
            let first = &self.studies[0].context_policy;
            if self
                .studies
                .iter()
                .any(|study| &study.context_policy != first)
            {
                return Err(ReproductiveIsolationDesignError::ContextPolicyMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductiveIsolationDesignDigest([u8; 32]);

impl ReproductiveIsolationDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

impl fmt::Debug for ReproductiveIsolationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReproductiveIsolationDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ReproductiveIsolationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_hex(&self.0, f) }
}

#[derive(Debug)]
#[must_use = "validated isolation design should gate reproductive-isolation evidence capture"]
pub struct ValidatedReproductiveIsolationDesign<'a> {
    design: &'a ReproductiveIsolationDesign,
    design_digest: ReproductiveIsolationDesignDigest,
}

impl<'a> ValidatedReproductiveIsolationDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current<'b, 'c>(
        design: &'a ReproductiveIsolationDesign,
        studies: impl IntoIterator<Item = IsolationStudyDesignInput<'b, 'c>>,
        minimum_barrier_supporting_studies: u64,
        minimum_observed_contact_opportunities: u64,
        minimum_generation_count_per_study: u64,
        context_compatibility: IsolationContextCompatibility,
        independence_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, ReproductiveIsolationDesignError> {
        design.validate_local()?;
        let recomputed = ReproductiveIsolationDesign::declare(
            design.design_id.clone(),
            studies,
            minimum_barrier_supporting_studies,
            minimum_observed_contact_opportunities,
            minimum_generation_count_per_study,
            context_compatibility,
            independence_rule_authority,
        )?;
        if recomputed != *design {
            return Err(ReproductiveIsolationDesignError::ReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a ReproductiveIsolationDesign { self.design }
    pub fn design_digest(&self) -> ReproductiveIsolationDesignDigest { self.design_digest }
}

fn put_context_policy(digest: &mut Sha256, policy: &ReproductiveContactContextPolicy) {
    match policy {
        ReproductiveContactContextPolicy::ExactContext { context_digest } => {
            digest.update([0]);
            digest.update(context_digest.as_bytes());
        }
        ReproductiveContactContextPolicy::DeclaredContextTrajectory { authority } => {
            digest.update([1]);
            put_authority(digest, authority);
        }
    }
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum ReproductiveIsolationDesignError {
    ContactDesign(crate::ReproductiveContactDesignError),
    UnsupportedVersion(u32),
    DuplicateStudyUnit(IsolationStudyUnitId),
    DuplicateContactDesignDigest,
    InsufficientDeclaredStudies,
    LineagePairMismatch,
    ContextPolicyMismatch,
    InvalidBarrierStudyThreshold,
    InvalidContactOpportunityThreshold,
    MinimumGenerationCountTooSmall,
    GenerationSpanTooShort {
        unit_id: IsolationStudyUnitId,
        observed: u64,
        minimum: u64,
    },
    EmptyContactOpportunityCensus,
    GenerationCountInvariant,
    NonCanonicalStudyOrder,
    BarrierRuleMismatch,
    ReplayMismatch,
    ArithmeticOverflow,
}

impl From<crate::ReproductiveContactDesignError> for ReproductiveIsolationDesignError {
    fn from(value: crate::ReproductiveContactDesignError) -> Self { Self::ContactDesign(value) }
}

impl fmt::Display for ReproductiveIsolationDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContactDesign(error) => write!(f, "reproductive-contact design error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported reproductive-isolation design version {version}")
            }
            Self::DuplicateStudyUnit(id) => {
                write!(f, "isolation study unit {} is declared more than once", id.as_str())
            }
            Self::DuplicateContactDesignDigest => write!(
                f,
                "one SEL-09A contact design cannot masquerade as multiple independent isolation studies"
            ),
            Self::InsufficientDeclaredStudies => {
                write!(f, "reproductive-isolation design requires at least two studies")
            }
            Self::LineagePairMismatch => write!(
                f,
                "all V1 isolation studies must target the same ordered lineage-authority pair"
            ),
            Self::ContextPolicyMismatch => write!(
                f,
                "contact-study context policies differ under shared-context replication"
            ),
            Self::InvalidBarrierStudyThreshold => write!(
                f,
                "minimum barrier-supporting studies must be between two and the declared study count"
            ),
            Self::InvalidContactOpportunityThreshold => write!(
                f,
                "minimum observed-contact opportunities must be at least the barrier-study threshold"
            ),
            Self::MinimumGenerationCountTooSmall => write!(
                f,
                "sustained reproductive-isolation V1 requires at least two generation coordinates per study"
            ),
            Self::GenerationSpanTooShort { unit_id, observed, minimum } => write!(
                f,
                "study {} has {observed} generation coordinates; minimum is {minimum}",
                unit_id.as_str()
            ),
            Self::EmptyContactOpportunityCensus => write!(
                f,
                "a reproductive-isolation study cannot use an empty contact-opportunity census"
            ),
            Self::GenerationCountInvariant => {
                write!(f, "persisted isolation-study generation count is inconsistent")
            }
            Self::NonCanonicalStudyOrder => {
                write!(f, "reproductive-isolation study declarations are not canonical")
            }
            Self::BarrierRuleMismatch => write!(
                f,
                "persisted isolation design does not bind the built-in V1 complete-barrier rule"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted reproductive-isolation design does not replay against current 09A designs"
            ),
            Self::ArithmeticOverflow => write!(f, "reproductive-isolation design arithmetic overflowed"),
        }
    }
}

impl Error for ReproductiveIsolationDesignError {}
