// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Outcome-free preregistration for cross-model current-species robustness.
//!
//! This crate never inspects model-bound species outcomes. It freezes the exact
//! model set, common biological subject, family-specific outcome-free designs,
//! semantic-dependency evidence, and a qualified fault-domain policy before E2B.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId,
    CurrentSpeciesClassificationDesignDigest, LineageDivergenceHistoryDesignDigest,
    ReproductiveIsolationDesignDigest, ValidatedCurrentSpeciesClassificationDesign,
};
use symtropy_species_concept::{
    OpenSpeciesConceptIdentity, SpeciesConceptFamilyDescriptor,
    SpeciesConceptFamilyDescriptorDigest, ValidatedSpeciesConceptFamilyDescriptor,
};
use symtropy_species_concept_general_lineage::{
    GeneralLineageClassificationDesignDigest, ValidatedGeneralLineageClassificationDesign,
    ValidatedGeneralLineageFamilyDescriptor,
};
use symtropy_species_concept_relations::{
    SemanticDependencyClass, SpeciesConceptRelationEvidence,
    SpeciesConceptRelationEvidenceDigest, ValidatedSpeciesConceptRelationEvidence,
};

pub const CURRENT_CROSS_MODEL_ROBUSTNESS_DESIGN_VERSION: u32 = 1;
const DESIGN_DOMAIN: &[u8] = b"symtropy:species-concept:current-robustness-design:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:current-robustness-design-rule:v1\0";
const PROFILE_DOMAIN: &[u8] = b"symtropy:species-concept:model-fault-domain-profile:v1\0";
const FAULT_RULE_DOMAIN: &[u8] = b"symtropy:species-concept:model-fault-domain-rule:v1\0";
const POLICY_DOMAIN: &[u8] = b"symtropy:species-concept:fault-domain-independence-policy:v1\0";
const POLICY_RULE_DOMAIN: &[u8] = b"symtropy:species-concept:fault-domain-independence-policy-rule:v1\0";

const RULE_SPEC: &[u8] = b"cross-model current species robustness design v1: outcome-free preregistration; one entry per conceptual identity; every model exposes current-species-status; exact ordered lineage pair and exact SEL-10A history-design identity match across models; family-specific outcome-free classification surfaces remain explicit; evidence-universe authority qualifies the common biological evidence universe; complete semantic-relation and pairwise fault-domain surfaces are required; nested/criterion-dependent or overlapping relations never establish conceptual independence; potentially-non-nested remains only semantically eligible and separately requires qualified fault-domain diversity under the exact preregistered policy; no model outcome, majority vote, historical-transition robustness, nomenclature, or universal taxonomy claim";
const FAULT_RULE_SPEC: &[u8] = b"model fault-domain profile v1: conceptual identity differs from qualification identity; each model profile binds qualification organization/process, evidence-source lineage, implementation/toolchain lineage, upstream evidence-authority lineage, and semantic-mapping qualification lineage; IDs are auditable classifications rather than self-proving independence claims";
const POLICY_RULE_SPEC: &[u8] = b"fault-domain independence policy v1: the required distinct dimensions are outcome-free, canonical, externally qualified, contain at least two dimensions, and include at least one evidence-related dimension; pairwise sufficiently-distinct status requires every policy-required dimension to differ plus an external authority qualifying the exact profile pair and policy digest; changing policy changes design lineage and invalidates stale pair assessments; different IDs alone never prove independence";

macro_rules! id_type {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, RobustnessDesignError> {
                let value = value.into();
                validate_id($field, &value)?;
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

id_type!(CurrentRobustnessDesignId, "CurrentRobustnessDesignId");
id_type!(ModelFaultDomainProfileId, "ModelFaultDomainProfileId");
id_type!(FaultDomainId, "FaultDomainId");

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name([u8; 32]);
        impl $name {
            pub fn new(bytes: [u8; 32]) -> Self { Self(bytes) }
            pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                fmt_hex(&self.0, f)?;
                write!(f, ")")
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_hex(&self.0, f) }
        }
    };
}

digest_type!(ModelFaultDomainProfileDigest);
digest_type!(FaultDomainIndependencePolicyDigest);
digest_type!(CurrentCrossModelRobustnessDesignDigest);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedEvidenceSubject {
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
    pub evidence_universe_authority: AnalysisAuthorityRef,
}

impl SharedEvidenceSubject {
    pub fn new(
        lineage_a: AnalysisAuthorityRef,
        lineage_b: AnalysisAuthorityRef,
        lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
        evidence_universe_authority: AnalysisAuthorityRef,
    ) -> Result<Self, RobustnessDesignError> {
        let subject = Self { lineage_a, lineage_b, lineage_history_design_digest, evidence_universe_authority };
        subject.validate_local()?;
        Ok(subject)
    }
    fn validate_local(&self) -> Result<(), RobustnessDesignError> {
        if self.lineage_a == self.lineage_b { return Err(RobustnessDesignError::LineagePairMismatch); }
        validate_authority(&self.evidence_universe_authority, "evidence_universe_revision")
    }
    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.lineage_a);
        put_authority(digest, &self.lineage_b);
        digest.update(self.lineage_history_design_digest.as_bytes());
        put_authority(digest, &self.evidence_universe_authority);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelSpecificClassificationDesignRef {
    StrictBiological {
        design_digest: CurrentSpeciesClassificationDesignDigest,
        reproductive_isolation_design_digest: ReproductiveIsolationDesignDigest,
    },
    GeneralLineage { design_digest: GeneralLineageClassificationDesignDigest },
}

impl ModelSpecificClassificationDesignRef {
    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::StrictBiological { design_digest, reproductive_isolation_design_digest } => {
                digest.update([0]);
                digest.update(design_digest.as_bytes());
                digest.update(reproductive_isolation_design_digest.as_bytes());
            }
            Self::GeneralLineage { design_digest } => {
                digest.update([1]);
                digest.update(design_digest.as_bytes());
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FaultDomainDimension {
    QualificationOrganization,
    QualificationProcess,
    EvidenceSourceLineage,
    ImplementationToolchainLineage,
    UpstreamEvidenceAuthorityLineage,
    SemanticMappingQualificationLineage,
}

impl FaultDomainDimension {
    fn tag(self) -> u8 {
        match self {
            Self::QualificationOrganization => 0,
            Self::QualificationProcess => 1,
            Self::EvidenceSourceLineage => 2,
            Self::ImplementationToolchainLineage => 3,
            Self::UpstreamEvidenceAuthorityLineage => 4,
            Self::SemanticMappingQualificationLineage => 5,
        }
    }
    fn is_evidence_related(self) -> bool {
        matches!(self, Self::EvidenceSourceLineage | Self::UpstreamEvidenceAuthorityLineage)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelFaultDomainProfile {
    pub profile_id: ModelFaultDomainProfileId,
    pub conceptual_identity: OpenSpeciesConceptIdentity,
    pub descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
    pub qualification_organization: FaultDomainId,
    pub qualification_process: FaultDomainId,
    pub evidence_source_lineage: FaultDomainId,
    pub implementation_toolchain_lineage: FaultDomainId,
    pub upstream_evidence_authority_lineage: FaultDomainId,
    pub semantic_mapping_qualification_lineage: FaultDomainId,
    pub profile_qualification_authority: AnalysisAuthorityRef,
}

impl ModelFaultDomainProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        profile_id: ModelFaultDomainProfileId,
        conceptual_identity: OpenSpeciesConceptIdentity,
        descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
        qualification_organization: FaultDomainId,
        qualification_process: FaultDomainId,
        evidence_source_lineage: FaultDomainId,
        implementation_toolchain_lineage: FaultDomainId,
        upstream_evidence_authority_lineage: FaultDomainId,
        semantic_mapping_qualification_lineage: FaultDomainId,
        profile_qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, RobustnessDesignError> {
        let profile = Self {
            profile_id, conceptual_identity, descriptor_digest, qualification_organization,
            qualification_process, evidence_source_lineage, implementation_toolchain_lineage,
            upstream_evidence_authority_lineage, semantic_mapping_qualification_lineage,
            profile_qualification_authority,
        };
        profile.validate_local()?;
        Ok(profile)
    }

    pub fn canonical_digest(&self) -> Result<ModelFaultDomainProfileDigest, RobustnessDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(PROFILE_DOMAIN);
        put_text(&mut digest, self.profile_id.as_str());
        put_identity(&mut digest, &self.conceptual_identity);
        digest.update(self.descriptor_digest.as_bytes());
        for dimension in ALL_FAULT_DIMENSIONS {
            digest.update([dimension.tag()]);
            put_text(&mut digest, self.domain_id(dimension).as_str());
        }
        put_authority(&mut digest, &self.profile_qualification_authority);
        Ok(ModelFaultDomainProfileDigest::new(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), RobustnessDesignError> {
        validate_identity(&self.conceptual_identity)?;
        validate_authority(&self.profile_qualification_authority, "fault_profile_qualification_revision")
    }

    fn domain_id(&self, dimension: FaultDomainDimension) -> &FaultDomainId {
        match dimension {
            FaultDomainDimension::QualificationOrganization => &self.qualification_organization,
            FaultDomainDimension::QualificationProcess => &self.qualification_process,
            FaultDomainDimension::EvidenceSourceLineage => &self.evidence_source_lineage,
            FaultDomainDimension::ImplementationToolchainLineage => &self.implementation_toolchain_lineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage => &self.upstream_evidence_authority_lineage,
            FaultDomainDimension::SemanticMappingQualificationLineage => &self.semantic_mapping_qualification_lineage,
        }
    }
}

const ALL_FAULT_DIMENSIONS: [FaultDomainDimension; 6] = [
    FaultDomainDimension::QualificationOrganization,
    FaultDomainDimension::QualificationProcess,
    FaultDomainDimension::EvidenceSourceLineage,
    FaultDomainDimension::ImplementationToolchainLineage,
    FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
    FaultDomainDimension::SemanticMappingQualificationLineage,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaultDomainIndependencePolicy {
    pub required_distinct_dimensions: Vec<FaultDomainDimension>,
    pub qualification_authority: AnalysisAuthorityRef,
    pub rule_authority: AnalysisAuthorityRef,
}

impl FaultDomainIndependencePolicy {
    pub fn declare(
        dimensions: impl IntoIterator<Item = FaultDomainDimension>,
        qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, RobustnessDesignError> {
        let mut required_distinct_dimensions: Vec<_> = dimensions.into_iter().collect();
        required_distinct_dimensions.sort();
        if required_distinct_dimensions.windows(2).any(|w| w[0] == w[1]) {
            return Err(RobustnessDesignError::DuplicateFaultDomainDimension);
        }
        let policy = Self {
            required_distinct_dimensions,
            qualification_authority,
            rule_authority: fault_domain_independence_policy_rule_v1(),
        };
        policy.validate_local()?;
        Ok(policy)
    }

    pub fn canonical_digest(&self) -> Result<FaultDomainIndependencePolicyDigest, RobustnessDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(POLICY_DOMAIN);
        put_u64(&mut digest, self.required_distinct_dimensions.len() as u64);
        for dimension in &self.required_distinct_dimensions { digest.update([dimension.tag()]); }
        put_authority(&mut digest, &self.qualification_authority);
        put_authority(&mut digest, &self.rule_authority);
        Ok(FaultDomainIndependencePolicyDigest::new(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), RobustnessDesignError> {
        if self.required_distinct_dimensions.len() < 2 {
            return Err(RobustnessDesignError::FaultPolicyTooFewDimensions);
        }
        if self.required_distinct_dimensions.windows(2).any(|w| w[0] >= w[1]) {
            return Err(RobustnessDesignError::NonCanonicalFaultDomainDimensions);
        }
        if !self.required_distinct_dimensions.iter().copied().any(FaultDomainDimension::is_evidence_related) {
            return Err(RobustnessDesignError::FaultPolicyMissingEvidenceDimension);
        }
        validate_authority(&self.qualification_authority, "fault_policy_qualification_revision")?;
        if self.rule_authority != fault_domain_independence_policy_rule_v1() {
            return Err(RobustnessDesignError::FaultPolicyRuleMismatch);
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) -> Result<(), RobustnessDesignError> {
        digest.update(self.canonical_digest()?.as_bytes());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesModelDesignRecord {
    pub descriptor: SpeciesConceptFamilyDescriptor,
    pub descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
    pub classification_design: ModelSpecificClassificationDesignRef,
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
    pub fault_profile: ModelFaultDomainProfile,
    pub fault_profile_digest: ModelFaultDomainProfileDigest,
}

impl SpeciesModelDesignRecord {
    pub fn strict_biological(
        descriptor: &ValidatedSpeciesConceptFamilyDescriptor<'_>,
        design: &ValidatedCurrentSpeciesClassificationDesign<'_>,
        fault_profile: ModelFaultDomainProfile,
    ) -> Result<Self, RobustnessDesignError> {
        let raw_descriptor = descriptor.descriptor();
        require_current_status_capability(raw_descriptor)?;
        validate_profile_for_descriptor(&fault_profile, raw_descriptor, descriptor.descriptor_digest())?;
        let raw_design = design.design();
        let record = Self {
            descriptor: raw_descriptor.clone(),
            descriptor_digest: descriptor.descriptor_digest(),
            classification_design: ModelSpecificClassificationDesignRef::StrictBiological {
                design_digest: design.design_digest(),
                reproductive_isolation_design_digest: raw_design.reproductive_isolation_design_digest,
            },
            lineage_a: raw_design.lineage_a.clone(),
            lineage_b: raw_design.lineage_b.clone(),
            lineage_history_design_digest: raw_design.lineage_history_design_digest,
            fault_profile_digest: fault_profile.canonical_digest()?,
            fault_profile,
        };
        record.validate_local()?;
        Ok(record)
    }

    pub fn general_lineage(
        descriptor: &ValidatedGeneralLineageFamilyDescriptor<'_>,
        design: &ValidatedGeneralLineageClassificationDesign<'_>,
        fault_profile: ModelFaultDomainProfile,
    ) -> Result<Self, RobustnessDesignError> {
        let raw_descriptor = descriptor.descriptor();
        require_current_status_capability(raw_descriptor)?;
        validate_profile_for_descriptor(&fault_profile, raw_descriptor, descriptor.descriptor_digest())?;
        let raw_design = design.design();
        let history_design = &raw_design.lineage_history_design;
        let record = Self {
            descriptor: raw_descriptor.clone(),
            descriptor_digest: descriptor.descriptor_digest(),
            classification_design: ModelSpecificClassificationDesignRef::GeneralLineage {
                design_digest: design.design_digest(),
            },
            lineage_a: history_design.lineage_a.clone(),
            lineage_b: history_design.lineage_b.clone(),
            lineage_history_design_digest: raw_design.lineage_history_design_digest,
            fault_profile_digest: fault_profile.canonical_digest()?,
            fault_profile,
        };
        record.validate_local()?;
        Ok(record)
    }

    pub fn conceptual_identity(&self) -> &OpenSpeciesConceptIdentity { &self.descriptor.conceptual_identity }

    fn validate_local(&self) -> Result<(), RobustnessDesignError> {
        if self.descriptor.canonical_digest()? != self.descriptor_digest {
            return Err(RobustnessDesignError::DescriptorDigestMismatch);
        }
        require_current_status_capability(&self.descriptor)?;
        validate_profile_for_descriptor(&self.fault_profile, &self.descriptor, self.descriptor_digest)?;
        if self.fault_profile.canonical_digest()? != self.fault_profile_digest {
            return Err(RobustnessDesignError::FaultProfileDigestMismatch);
        }
        if self.lineage_a == self.lineage_b { return Err(RobustnessDesignError::LineagePairMismatch); }
        match (&self.classification_design, self.descriptor.source_authority.source_kind_id.as_str()) {
            (ModelSpecificClassificationDesignRef::StrictBiological { .. }, "sel10e1-strict-bsc-authority") => {}
            (ModelSpecificClassificationDesignRef::GeneralLineage { .. }, "sel10e1b-general-lineage-model") => {}
            _ => return Err(RobustnessDesignError::ClassificationSourceKindMismatch),
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) -> Result<(), RobustnessDesignError> {
        self.validate_local()?;
        put_identity(digest, self.conceptual_identity());
        digest.update(self.descriptor_digest.as_bytes());
        self.classification_design.put(digest);
        put_authority(digest, &self.lineage_a);
        put_authority(digest, &self.lineage_b);
        digest.update(self.lineage_history_design_digest.as_bytes());
        digest.update(self.fault_profile_digest.as_bytes());
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRelationRecord {
    pub left: OpenSpeciesConceptIdentity,
    pub right: OpenSpeciesConceptIdentity,
    pub evidence: SpeciesConceptRelationEvidence,
    pub evidence_digest: SpeciesConceptRelationEvidenceDigest,
    pub dependency_class: Option<SemanticDependencyClass>,
}

impl SemanticRelationRecord {
    pub fn from_current(relation: &ValidatedSpeciesConceptRelationEvidence<'_>) -> Result<Self, RobustnessDesignError> {
        let evidence = relation.evidence();
        let record = Self {
            left: evidence.design.left.clone(),
            right: evidence.design.right.clone(),
            evidence: evidence.clone(),
            evidence_digest: relation.evidence_digest(),
            dependency_class: relation.dependency_class_if_qualified(),
        };
        record.validate_local()?;
        Ok(record)
    }

    fn validate_local(&self) -> Result<(), RobustnessDesignError> {
        validate_identity(&self.left)?;
        validate_identity(&self.right)?;
        if self.left >= self.right { return Err(RobustnessDesignError::NonCanonicalPair); }
        if self.evidence.design.left != self.left || self.evidence.design.right != self.right {
            return Err(RobustnessDesignError::RelationEndpointMismatch);
        }
        if self.evidence.canonical_digest()? != self.evidence_digest {
            return Err(RobustnessDesignError::RelationDigestMismatch);
        }
        if self.evidence.assessment.dependency_class_if_qualified() != self.dependency_class {
            return Err(RobustnessDesignError::RelationDependencyInvariant);
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) -> Result<(), RobustnessDesignError> {
        self.validate_local()?;
        put_identity(digest, &self.left);
        put_identity(digest, &self.right);
        digest.update(self.evidence_digest.as_bytes());
        digest.update([match self.dependency_class {
            None => 0,
            Some(SemanticDependencyClass::NestedOrCriterionDependent) => 1,
            Some(SemanticDependencyClass::Overlapping) => 2,
            Some(SemanticDependencyClass::PotentiallyNonNested) => 3,
        }]);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairwiseFaultDomainDisposition {
    QualifiedSufficientlyDistinct,
    KnownDependent,
    UnknownOrUnqualified,
}
impl PairwiseFaultDomainDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::QualifiedSufficientlyDistinct => 0,
            Self::KnownDependent => 1,
            Self::UnknownOrUnqualified => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairwiseFaultDomainAssessment {
    pub left: OpenSpeciesConceptIdentity,
    pub right: OpenSpeciesConceptIdentity,
    pub left_profile_digest: ModelFaultDomainProfileDigest,
    pub right_profile_digest: ModelFaultDomainProfileDigest,
    pub policy_digest: FaultDomainIndependencePolicyDigest,
    pub disposition: PairwiseFaultDomainDisposition,
    pub qualification_authority: AnalysisAuthorityRef,
}

impl PairwiseFaultDomainAssessment {
    pub fn new(
        policy: &FaultDomainIndependencePolicy,
        first: &ModelFaultDomainProfile,
        second: &ModelFaultDomainProfile,
        disposition: PairwiseFaultDomainDisposition,
        qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, RobustnessDesignError> {
        policy.validate_local()?;
        validate_authority(&qualification_authority, "pair_fault_qualification_revision")?;
        let (left, right) = if first.conceptual_identity < second.conceptual_identity {
            (first, second)
        } else if second.conceptual_identity < first.conceptual_identity {
            (second, first)
        } else {
            return Err(RobustnessDesignError::DuplicateConceptualIdentity);
        };
        validate_pair_disposition(policy, left, right, disposition)?;
        Ok(Self {
            left: left.conceptual_identity.clone(),
            right: right.conceptual_identity.clone(),
            left_profile_digest: left.canonical_digest()?,
            right_profile_digest: right.canonical_digest()?,
            policy_digest: policy.canonical_digest()?,
            disposition,
            qualification_authority,
        })
    }

    fn validate_against_profiles(
        &self,
        policy: &FaultDomainIndependencePolicy,
        left: &ModelFaultDomainProfile,
        right: &ModelFaultDomainProfile,
    ) -> Result<(), RobustnessDesignError> {
        if self.left != left.conceptual_identity || self.right != right.conceptual_identity
            || self.left_profile_digest != left.canonical_digest()?
            || self.right_profile_digest != right.canonical_digest()?
        {
            return Err(RobustnessDesignError::FaultAssessmentProfileMismatch);
        }
        if self.policy_digest != policy.canonical_digest()? {
            return Err(RobustnessDesignError::FaultAssessmentPolicyMismatch);
        }
        validate_authority(&self.qualification_authority, "pair_fault_qualification_revision")?;
        validate_pair_disposition(policy, left, right, self.disposition)
    }

    fn put(&self, digest: &mut Sha256) {
        put_identity(digest, &self.left);
        put_identity(digest, &self.right);
        digest.update(self.left_profile_digest.as_bytes());
        digest.update(self.right_profile_digest.as_bytes());
        digest.update(self.policy_digest.as_bytes());
        digest.update([self.disposition.tag()]);
        put_authority(digest, &self.qualification_authority);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingModelPolicy { FailClosed, ReportInsufficientCoverage }
impl MissingModelPolicy {
    fn tag(self) -> u8 { match self { Self::FailClosed => 0, Self::ReportInsufficientCoverage => 1 } }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentCrossModelRobustnessDesign {
    design_version: u32,
    pub design_id: CurrentRobustnessDesignId,
    pub subject: SharedEvidenceSubject,
    pub models: Vec<SpeciesModelDesignRecord>,
    pub semantic_relations: Vec<SemanticRelationRecord>,
    pub fault_domain_policy: FaultDomainIndependencePolicy,
    pub fault_domain_policy_digest: FaultDomainIndependencePolicyDigest,
    pub fault_assessments: Vec<PairwiseFaultDomainAssessment>,
    pub minimum_conceptual_family_count: u32,
    pub minimum_independent_model_coverage: u32,
    pub missing_model_policy: MissingModelPolicy,
    pub fault_domain_rule_authority: AnalysisAuthorityRef,
    pub design_rule_authority: AnalysisAuthorityRef,
}

impl CurrentCrossModelRobustnessDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        design_id: CurrentRobustnessDesignId,
        subject: SharedEvidenceSubject,
        models: impl IntoIterator<Item = SpeciesModelDesignRecord>,
        semantic_relations: impl IntoIterator<Item = SemanticRelationRecord>,
        fault_domain_policy: FaultDomainIndependencePolicy,
        fault_assessments: impl IntoIterator<Item = PairwiseFaultDomainAssessment>,
        minimum_conceptual_family_count: u32,
        minimum_independent_model_coverage: u32,
        missing_model_policy: MissingModelPolicy,
    ) -> Result<Self, RobustnessDesignError> {
        if minimum_conceptual_family_count < 2 { return Err(RobustnessDesignError::ConceptualCoverageThresholdTooLow); }
        if minimum_independent_model_coverage < 2 { return Err(RobustnessDesignError::IndependentCoverageThresholdTooLow); }
        subject.validate_local()?;
        fault_domain_policy.validate_local()?;
        let mut models: Vec<_> = models.into_iter().collect();
        models.sort_by(|a, b| a.conceptual_identity().cmp(b.conceptual_identity()));
        validate_models(&subject, &models, minimum_conceptual_family_count)?;
        if minimum_independent_model_coverage as usize > models.len() {
            return Err(RobustnessDesignError::IndependentCoverageThresholdExceedsModelSet);
        }
        let mut semantic_relations: Vec<_> = semantic_relations.into_iter().collect();
        semantic_relations.sort_by(|a, b| (&a.left, &a.right).cmp(&(&b.left, &b.right)));
        validate_relations(&models, &semantic_relations)?;
        let mut fault_assessments: Vec<_> = fault_assessments.into_iter().collect();
        fault_assessments.sort_by(|a, b| (&a.left, &a.right).cmp(&(&b.left, &b.right)));
        validate_fault_assessments(&models, &fault_domain_policy, &fault_assessments)?;
        let design = Self {
            design_version: CURRENT_CROSS_MODEL_ROBUSTNESS_DESIGN_VERSION,
            design_id, subject, models, semantic_relations,
            fault_domain_policy_digest: fault_domain_policy.canonical_digest()?,
            fault_domain_policy, fault_assessments,
            minimum_conceptual_family_count, minimum_independent_model_coverage,
            missing_model_policy,
            fault_domain_rule_authority: model_fault_domain_rule_v1(),
            design_rule_authority: current_cross_model_robustness_design_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(&self) -> Result<CurrentCrossModelRobustnessDesignDigest, RobustnessDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.design_id.as_str());
        self.subject.put(&mut digest);
        put_u64(&mut digest, self.models.len() as u64);
        for model in &self.models { model.put(&mut digest)?; }
        put_u64(&mut digest, self.semantic_relations.len() as u64);
        for relation in &self.semantic_relations { relation.put(&mut digest)?; }
        self.fault_domain_policy.put(&mut digest)?;
        digest.update(self.fault_domain_policy_digest.as_bytes());
        put_u64(&mut digest, self.fault_assessments.len() as u64);
        for assessment in &self.fault_assessments { assessment.put(&mut digest); }
        put_u32(&mut digest, self.minimum_conceptual_family_count);
        put_u32(&mut digest, self.minimum_independent_model_coverage);
        digest.update([self.missing_model_policy.tag()]);
        put_authority(&mut digest, &self.fault_domain_rule_authority);
        put_authority(&mut digest, &self.design_rule_authority);
        Ok(CurrentCrossModelRobustnessDesignDigest::new(digest.finalize().into()))
    }

    /// Pair eligibility only. E2B must still evaluate the complete preregistered matrix.
    pub fn pair_is_eligible_for_independent_coverage(
        &self,
        first: &OpenSpeciesConceptIdentity,
        second: &OpenSpeciesConceptIdentity,
    ) -> bool {
        if first == second { return false; }
        let (left, right) = if first < second { (first, second) } else { (second, first) };
        let relation = self.semantic_relations.iter().find(|r| &r.left == left && &r.right == right);
        let fault = self.fault_assessments.iter().find(|a| &a.left == left && &a.right == right);
        matches!(relation.and_then(|r| r.dependency_class), Some(SemanticDependencyClass::PotentiallyNonNested))
            && matches!(fault.map(|a| a.disposition), Some(PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct))
    }

    fn validate_local(&self) -> Result<(), RobustnessDesignError> {
        if self.design_version != CURRENT_CROSS_MODEL_ROBUSTNESS_DESIGN_VERSION {
            return Err(RobustnessDesignError::UnsupportedDesignVersion(self.design_version));
        }
        self.subject.validate_local()?;
        if self.minimum_conceptual_family_count < 2 { return Err(RobustnessDesignError::ConceptualCoverageThresholdTooLow); }
        if self.minimum_independent_model_coverage < 2 { return Err(RobustnessDesignError::IndependentCoverageThresholdTooLow); }
        validate_models(&self.subject, &self.models, self.minimum_conceptual_family_count)?;
        if self.minimum_independent_model_coverage as usize > self.models.len() {
            return Err(RobustnessDesignError::IndependentCoverageThresholdExceedsModelSet);
        }
        self.fault_domain_policy.validate_local()?;
        if self.fault_domain_policy.canonical_digest()? != self.fault_domain_policy_digest {
            return Err(RobustnessDesignError::FaultPolicyDigestMismatch);
        }
        validate_relations(&self.models, &self.semantic_relations)?;
        validate_fault_assessments(&self.models, &self.fault_domain_policy, &self.fault_assessments)?;
        if self.fault_domain_rule_authority != model_fault_domain_rule_v1() { return Err(RobustnessDesignError::FaultDomainRuleMismatch); }
        if self.design_rule_authority != current_cross_model_robustness_design_rule_v1() { return Err(RobustnessDesignError::DesignRuleMismatch); }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated robustness design should gate cross-model status execution"]
pub struct ValidatedCurrentCrossModelRobustnessDesign<'a> {
    design: &'a CurrentCrossModelRobustnessDesign,
    design_digest: CurrentCrossModelRobustnessDesignDigest,
}

impl<'a> ValidatedCurrentCrossModelRobustnessDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a CurrentCrossModelRobustnessDesign,
        subject: SharedEvidenceSubject,
        models: impl IntoIterator<Item = SpeciesModelDesignRecord>,
        semantic_relations: impl IntoIterator<Item = SemanticRelationRecord>,
        fault_domain_policy: FaultDomainIndependencePolicy,
        fault_assessments: impl IntoIterator<Item = PairwiseFaultDomainAssessment>,
        minimum_conceptual_family_count: u32,
        minimum_independent_model_coverage: u32,
        missing_model_policy: MissingModelPolicy,
    ) -> Result<Self, RobustnessDesignError> {
        design.validate_local()?;
        let recomputed = CurrentCrossModelRobustnessDesign::declare(
            design.design_id.clone(), subject, models, semantic_relations, fault_domain_policy,
            fault_assessments, minimum_conceptual_family_count,
            minimum_independent_model_coverage, missing_model_policy,
        )?;
        if recomputed != *design { return Err(RobustnessDesignError::DesignReplayMismatch); }
        Ok(Self { design, design_digest: design.canonical_digest()? })
    }
    pub fn design(&self) -> &'a CurrentCrossModelRobustnessDesign { self.design }
    pub fn design_digest(&self) -> CurrentCrossModelRobustnessDesignDigest { self.design_digest }
}

pub fn current_cross_model_robustness_design_rule_v1() -> AnalysisAuthorityRef {
    digest_rule("current-cross-model-robustness-design-v1", RULE_DOMAIN, RULE_SPEC)
}
pub fn model_fault_domain_rule_v1() -> AnalysisAuthorityRef {
    digest_rule("species-model-fault-domain-v1", FAULT_RULE_DOMAIN, FAULT_RULE_SPEC)
}
pub fn fault_domain_independence_policy_rule_v1() -> AnalysisAuthorityRef {
    digest_rule("species-model-fault-domain-independence-policy-v1", POLICY_RULE_DOMAIN, POLICY_RULE_SPEC)
}

fn validate_models(
    subject: &SharedEvidenceSubject,
    models: &[SpeciesModelDesignRecord],
    minimum_conceptual_family_count: u32,
) -> Result<(), RobustnessDesignError> {
    if models.len() < minimum_conceptual_family_count as usize {
        return Err(RobustnessDesignError::InsufficientConceptualFamilyCoverage);
    }
    let mut previous: Option<&OpenSpeciesConceptIdentity> = None;
    for model in models {
        model.validate_local()?;
        let identity = model.conceptual_identity();
        if let Some(prev) = previous {
            if prev >= identity {
                if prev == identity { return Err(RobustnessDesignError::DuplicateConceptualIdentity); }
                return Err(RobustnessDesignError::NonCanonicalModelOrder);
            }
        }
        previous = Some(identity);
        if model.lineage_a != subject.lineage_a
            || model.lineage_b != subject.lineage_b
            || model.lineage_history_design_digest != subject.lineage_history_design_digest
        {
            return Err(RobustnessDesignError::EvidenceSubjectMismatch);
        }
    }
    Ok(())
}

fn validate_relations(
    models: &[SpeciesModelDesignRecord],
    relations: &[SemanticRelationRecord],
) -> Result<(), RobustnessDesignError> {
    let expected = expected_pairs(models);
    if relations.len() != expected.len() { return Err(RobustnessDesignError::IncompleteRelationCoverage); }
    for (expected_pair, record) in expected.iter().zip(relations) {
        record.validate_local()?;
        if (&record.left, &record.right) != *expected_pair {
            return Err(RobustnessDesignError::RelationEndpointMismatch);
        }
    }
    Ok(())
}

fn validate_fault_assessments(
    models: &[SpeciesModelDesignRecord],
    policy: &FaultDomainIndependencePolicy,
    assessments: &[PairwiseFaultDomainAssessment],
) -> Result<(), RobustnessDesignError> {
    let expected = expected_pairs(models);
    if assessments.len() != expected.len() { return Err(RobustnessDesignError::IncompleteFaultAssessmentCoverage); }
    let by_identity: BTreeMap<_, _> = models.iter().map(|m| (m.conceptual_identity().clone(), &m.fault_profile)).collect();
    for (expected_pair, assessment) in expected.iter().zip(assessments) {
        if (&assessment.left, &assessment.right) != *expected_pair {
            return Err(RobustnessDesignError::FaultAssessmentEndpointMismatch);
        }
        let left = by_identity.get(&assessment.left).ok_or(RobustnessDesignError::FaultAssessmentProfileMismatch)?;
        let right = by_identity.get(&assessment.right).ok_or(RobustnessDesignError::FaultAssessmentProfileMismatch)?;
        assessment.validate_against_profiles(policy, left, right)?;
    }
    Ok(())
}

fn validate_pair_disposition(
    policy: &FaultDomainIndependencePolicy,
    left: &ModelFaultDomainProfile,
    right: &ModelFaultDomainProfile,
    disposition: PairwiseFaultDomainDisposition,
) -> Result<(), RobustnessDesignError> {
    if disposition != PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct { return Ok(()); }
    for dimension in &policy.required_distinct_dimensions {
        if left.domain_id(*dimension) == right.domain_id(*dimension) {
            return Err(RobustnessDesignError::QualifiedIndependenceSharesRequiredDimension(*dimension));
        }
    }
    Ok(())
}

fn expected_pairs(models: &[SpeciesModelDesignRecord]) -> Vec<(&OpenSpeciesConceptIdentity, &OpenSpeciesConceptIdentity)> {
    let mut pairs = Vec::new();
    for left in 0..models.len() {
        for right in (left + 1)..models.len() {
            pairs.push((models[left].conceptual_identity(), models[right].conceptual_identity()));
        }
    }
    pairs
}

fn require_current_status_capability(descriptor: &SpeciesConceptFamilyDescriptor) -> Result<(), RobustnessDesignError> {
    if descriptor.capabilities.iter().any(|c| c.capability_id.as_str() == "current-species-status") {
        Ok(())
    } else {
        Err(RobustnessDesignError::MissingCurrentSpeciesStatusCapability)
    }
}

fn validate_profile_for_descriptor(
    profile: &ModelFaultDomainProfile,
    descriptor: &SpeciesConceptFamilyDescriptor,
    descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
) -> Result<(), RobustnessDesignError> {
    profile.validate_local()?;
    if profile.conceptual_identity != descriptor.conceptual_identity || profile.descriptor_digest != descriptor_digest {
        return Err(RobustnessDesignError::FaultProfileDescriptorMismatch);
    }
    Ok(())
}

fn validate_identity(identity: &OpenSpeciesConceptIdentity) -> Result<(), RobustnessDesignError> {
    if identity.family_version == 0 { return Err(RobustnessDesignError::ZeroFamilyVersion); }
    Ok(())
}
fn validate_authority(authority: &AnalysisAuthorityRef, field: &'static str) -> Result<(), RobustnessDesignError> {
    if authority.revision == 0 { return Err(RobustnessDesignError::ZeroRevision(field)); }
    Ok(())
}
fn validate_id(field: &'static str, value: &str) -> Result<(), RobustnessDesignError> {
    if value.is_empty() || value.len() > 180 || value.trim() != value || value.chars().any(char::is_control) {
        return Err(RobustnessDesignError::InvalidIdentifier { field, value: value.to_owned() });
    }
    Ok(())
}
fn digest_rule(id: &str, domain: &[u8], spec: &[u8]) -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(domain);
    put_u64(&mut digest, spec.len() as u64);
    digest.update(spec);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(id).expect("static robustness method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}
fn put_identity(digest: &mut Sha256, identity: &OpenSpeciesConceptIdentity) {
    put_text(digest, identity.family_id.as_str());
    put_u32(digest, identity.family_version);
    digest.update(identity.content_digest.as_bytes());
}
fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}
fn put_text(digest: &mut Sha256, value: &str) { put_u64(digest, value.len() as u64); digest.update(value.as_bytes()); }
fn put_u32(digest: &mut Sha256, value: u32) { digest.update(value.to_be_bytes()); }
fn put_u64(digest: &mut Sha256, value: u64) { digest.update(value.to_be_bytes()); }
fn fmt_hex(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes { write!(f, "{byte:02x}")?; }
    Ok(())
}

#[derive(Debug)]
pub enum RobustnessDesignError {
    InvalidIdentifier { field: &'static str, value: String },
    ZeroRevision(&'static str),
    ZeroFamilyVersion,
    UnsupportedDesignVersion(u32),
    LineagePairMismatch,
    EvidenceSubjectMismatch,
    MissingCurrentSpeciesStatusCapability,
    DescriptorDigestMismatch,
    ClassificationSourceKindMismatch,
    FaultProfileDescriptorMismatch,
    FaultProfileDigestMismatch,
    DuplicateConceptualIdentity,
    NonCanonicalModelOrder,
    ConceptualCoverageThresholdTooLow,
    IndependentCoverageThresholdTooLow,
    IndependentCoverageThresholdExceedsModelSet,
    InsufficientConceptualFamilyCoverage,
    RelationEndpointMismatch,
    RelationDigestMismatch,
    RelationDependencyInvariant,
    NonCanonicalPair,
    IncompleteRelationCoverage,
    FaultPolicyTooFewDimensions,
    DuplicateFaultDomainDimension,
    NonCanonicalFaultDomainDimensions,
    FaultPolicyMissingEvidenceDimension,
    FaultPolicyRuleMismatch,
    FaultPolicyDigestMismatch,
    FaultAssessmentEndpointMismatch,
    FaultAssessmentProfileMismatch,
    FaultAssessmentPolicyMismatch,
    IncompleteFaultAssessmentCoverage,
    QualifiedIndependenceSharesRequiredDimension(FaultDomainDimension),
    FaultDomainRuleMismatch,
    DesignRuleMismatch,
    DesignReplayMismatch,
    Descriptor(symtropy_species_concept::DescriptorError),
    Relation(symtropy_species_concept_relations::RelationError),
}

impl From<symtropy_species_concept::DescriptorError> for RobustnessDesignError {
    fn from(value: symtropy_species_concept::DescriptorError) -> Self { Self::Descriptor(value) }
}
impl From<symtropy_species_concept_relations::RelationError> for RobustnessDesignError {
    fn from(value: symtropy_species_concept_relations::RelationError) -> Self { Self::Relation(value) }
}

impl fmt::Display for RobustnessDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field, value } => write!(f, "invalid {field} value {value:?}"),
            Self::ZeroRevision(field) => write!(f, "{field} must be nonzero"),
            Self::ZeroFamilyVersion => write!(f, "species-concept family version must be nonzero"),
            Self::UnsupportedDesignVersion(v) => write!(f, "unsupported current robustness design version {v}"),
            Self::LineagePairMismatch => write!(f, "shared evidence subject must bind two distinct ordered lineage authorities"),
            Self::EvidenceSubjectMismatch => write!(f, "model design does not bind the exact shared lineage pair and SEL-10A history design"),
            Self::MissingCurrentSpeciesStatusCapability => write!(f, "model descriptor lacks current-species-status capability"),
            Self::DescriptorDigestMismatch => write!(f, "persisted descriptor does not match its digest"),
            Self::ClassificationSourceKindMismatch => write!(f, "family-specific classification-design kind does not match descriptor source kind"),
            Self::FaultProfileDescriptorMismatch => write!(f, "fault-domain profile does not bind the exact model descriptor identity"),
            Self::FaultProfileDigestMismatch => write!(f, "persisted fault-domain profile does not match its digest"),
            Self::DuplicateConceptualIdentity => write!(f, "one E2A design may contain only one authority instance per conceptual identity"),
            Self::NonCanonicalModelOrder => write!(f, "model set is not canonically ordered"),
            Self::ConceptualCoverageThresholdTooLow => write!(f, "minimum conceptual family count must be at least two"),
            Self::IndependentCoverageThresholdTooLow => write!(f, "minimum independent model coverage must be at least two"),
            Self::IndependentCoverageThresholdExceedsModelSet => write!(f, "minimum independent model coverage cannot exceed the preregistered model set"),
            Self::InsufficientConceptualFamilyCoverage => write!(f, "declared conceptual family set does not meet its preregistered minimum"),
            Self::RelationEndpointMismatch => write!(f, "semantic relation does not bind the expected model pair"),
            Self::RelationDigestMismatch => write!(f, "semantic relation snapshot does not match its digest"),
            Self::RelationDependencyInvariant => write!(f, "semantic relation dependency class does not recompute"),
            Self::NonCanonicalPair => write!(f, "model pair is not canonically ordered"),
            Self::IncompleteRelationCoverage => write!(f, "semantic relation matrix must cover every preregistered model pair exactly once"),
            Self::FaultPolicyTooFewDimensions => write!(f, "fault-domain independence policy must require at least two distinct dimensions"),
            Self::DuplicateFaultDomainDimension => write!(f, "fault-domain independence policy contains a duplicate dimension"),
            Self::NonCanonicalFaultDomainDimensions => write!(f, "fault-domain independence policy dimensions are not canonically ordered"),
            Self::FaultPolicyMissingEvidenceDimension => write!(f, "fault-domain independence policy must require at least one evidence-related dimension"),
            Self::FaultPolicyRuleMismatch => write!(f, "fault-domain independence policy does not bind the built-in V1 rule"),
            Self::FaultPolicyDigestMismatch => write!(f, "persisted fault-domain policy does not match its digest"),
            Self::FaultAssessmentEndpointMismatch => write!(f, "fault-domain assessment does not bind the expected model pair"),
            Self::FaultAssessmentProfileMismatch => write!(f, "fault-domain assessment does not bind the current exact pair of model profiles"),
            Self::FaultAssessmentPolicyMismatch => write!(f, "fault-domain assessment was qualified under a different independence policy"),
            Self::IncompleteFaultAssessmentCoverage => write!(f, "fault-domain assessments must cover every preregistered model pair exactly once"),
            Self::QualifiedIndependenceSharesRequiredDimension(dimension) => write!(f, "pair cannot be qualified sufficiently distinct because required fault dimension {dimension:?} is shared"),
            Self::FaultDomainRuleMismatch => write!(f, "design does not bind the built-in model fault-domain rule"),
            Self::DesignRuleMismatch => write!(f, "design does not bind the built-in E2A design rule"),
            Self::DesignReplayMismatch => write!(f, "persisted E2A design does not replay from current preregistered inputs"),
            Self::Descriptor(error) => write!(f, "descriptor error: {error}"),
            Self::Relation(error) => write!(f, "semantic-relation error: {error}"),
        }
    }
}
impl Error for RobustnessDesignError {}
