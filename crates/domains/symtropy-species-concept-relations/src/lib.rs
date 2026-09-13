// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Evidence-bearing semantic relations between species-concept model families.
//!
//! This crate does not decide which species concept is correct and does not establish
//! cross-model independence. It records a qualified semantic relation between exact
//! conceptual identities while keeping nested/overlapping/unknown structure explicit.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId,
};
use symtropy_species_concept::OpenSpeciesConceptIdentity;

pub const SPECIES_CONCEPT_RELATION_DESIGN_VERSION: u32 = 1;
pub const SPECIES_CONCEPT_RELATION_EVIDENCE_VERSION: u32 = 1;

const DESIGN_DOMAIN: &[u8] = b"symtropy:species-concept:relation-design:v1\0";
const EVIDENCE_DOMAIN: &[u8] = b"symtropy:species-concept:relation-evidence:v1\0";
const MAPPING_DOMAIN: &[u8] = b"symtropy:species-concept:relation-mapping:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:relation-rule:v1\0";
const RULE_SPEC: &[u8] = b"species concept semantic relation v1: exact conceptual identity endpoints; outcome-free pair/scope/protocol design precedes assessment; symmetric relations use canonical endpoint order; directional relations preserve canonical-order orientation; nested/refining/criterion relations never imply semantic independence; overlap remains dependence-visible; orthogonal-evidence-framework and competing-ontology relations are only potentially non-nested and still require downstream evidence/qualification fault-domain independence; disputed, unknown, or unavailable relations never upgrade robustness; no species status, speciation event, nomenclature, or universal taxonomy claim";

macro_rules! id_type {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, RelationError> {
                let value = value.into();
                validate_id($field, &value)?;
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

id_type!(SpeciesConceptRelationDesignId, "SpeciesConceptRelationDesignId");
id_type!(SpeciesConceptRelationScopeId, "SpeciesConceptRelationScopeId");

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name([u8; 32]);

        impl $name {
            pub fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                fmt_hex(&self.0, f)?;
                write!(f, ")")
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_hex(&self.0, f)
            }
        }
    };
}

digest_type!(SpeciesConceptRelationScopeDigest);
digest_type!(SpeciesConceptSemanticMappingDigest);
digest_type!(SpeciesConceptRelationDesignDigest);
digest_type!(SpeciesConceptRelationEvidenceDigest);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptRelationScopeRef {
    pub scope_id: SpeciesConceptRelationScopeId,
    pub revision: u32,
    pub content_digest: SpeciesConceptRelationScopeDigest,
}

impl SpeciesConceptRelationScopeRef {
    pub fn new(
        scope_id: SpeciesConceptRelationScopeId,
        revision: u32,
        content_digest: SpeciesConceptRelationScopeDigest,
    ) -> Result<Self, RelationError> {
        if revision == 0 {
            return Err(RelationError::ZeroRevision("relation_scope_revision"));
        }
        Ok(Self {
            scope_id,
            revision,
            content_digest,
        })
    }

    fn validate_local(&self) -> Result<(), RelationError> {
        if self.revision == 0 {
            return Err(RelationError::ZeroRevision("relation_scope_revision"));
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.scope_id.as_str());
        put_u32(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RelationMissingPolicy {
    FailClosed,
    ReportUnknownOrUnavailable,
}

impl RelationMissingPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::ReportUnknownOrUnavailable => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptRelationDesign {
    design_version: u32,
    pub design_id: SpeciesConceptRelationDesignId,
    /// Canonical lexicographic endpoint order. Relation direction is represented separately.
    pub left: OpenSpeciesConceptIdentity,
    pub right: OpenSpeciesConceptIdentity,
    pub scope: SpeciesConceptRelationScopeRef,
    pub protocol_authority: AnalysisAuthorityRef,
    pub missing_policy: RelationMissingPolicy,
    pub relation_rule_authority: AnalysisAuthorityRef,
}

impl SpeciesConceptRelationDesign {
    pub fn declare(
        design_id: SpeciesConceptRelationDesignId,
        first: OpenSpeciesConceptIdentity,
        second: OpenSpeciesConceptIdentity,
        scope: SpeciesConceptRelationScopeRef,
        protocol_authority: AnalysisAuthorityRef,
        missing_policy: RelationMissingPolicy,
    ) -> Result<Self, RelationError> {
        validate_identity(&first)?;
        validate_identity(&second)?;
        if first == second {
            return Err(RelationError::SameConceptEndpoint);
        }
        let (left, right) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        let design = Self {
            design_version: SPECIES_CONCEPT_RELATION_DESIGN_VERSION,
            design_id,
            left,
            right,
            scope,
            protocol_authority,
            missing_policy,
            relation_rule_authority: species_concept_relation_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(&self) -> Result<SpeciesConceptRelationDesignDigest, RelationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.design_id.as_str());
        put_identity(&mut digest, &self.left);
        put_identity(&mut digest, &self.right);
        self.scope.put(&mut digest);
        put_authority(&mut digest, &self.protocol_authority);
        digest.update([self.missing_policy.tag()]);
        put_authority(&mut digest, &self.relation_rule_authority);
        Ok(SpeciesConceptRelationDesignDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), RelationError> {
        if self.design_version != SPECIES_CONCEPT_RELATION_DESIGN_VERSION {
            return Err(RelationError::UnsupportedDesignVersion(self.design_version));
        }
        validate_identity(&self.left)?;
        validate_identity(&self.right)?;
        if self.left >= self.right {
            return Err(RelationError::NonCanonicalEndpoints);
        }
        self.scope.validate_local()?;
        validate_authority_revision(&self.protocol_authority, "relation_protocol_revision")?;
        if self.relation_rule_authority != species_concept_relation_rule_v1() {
            return Err(RelationError::RelationRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated relation design should gate semantic-relation assessment"]
pub struct ValidatedSpeciesConceptRelationDesign<'a> {
    design: &'a SpeciesConceptRelationDesign,
    design_digest: SpeciesConceptRelationDesignDigest,
}

impl<'a> ValidatedSpeciesConceptRelationDesign<'a> {
    pub fn validate_current(
        design: &'a SpeciesConceptRelationDesign,
        first: OpenSpeciesConceptIdentity,
        second: OpenSpeciesConceptIdentity,
        scope: SpeciesConceptRelationScopeRef,
        protocol_authority: AnalysisAuthorityRef,
        missing_policy: RelationMissingPolicy,
    ) -> Result<Self, RelationError> {
        design.validate_local()?;
        let recomputed = SpeciesConceptRelationDesign::declare(
            design.design_id.clone(),
            first,
            second,
            scope,
            protocol_authority,
            missing_policy,
        )?;
        if recomputed != *design {
            return Err(RelationError::DesignReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a SpeciesConceptRelationDesign {
        self.design
    }

    pub fn design_digest(&self) -> SpeciesConceptRelationDesignDigest {
        self.design_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeciesConceptRelationKind {
    EquivalentSemanticTarget,
    Generalizes,
    Refines,
    OperationalCriterionWithin,
    PartiallyOverlaps,
    OrthogonalEvidenceFramework,
    CompetingOntology,
}

impl SpeciesConceptRelationKind {
    fn tag(self) -> u8 {
        match self {
            Self::EquivalentSemanticTarget => 0,
            Self::Generalizes => 1,
            Self::Refines => 2,
            Self::OperationalCriterionWithin => 3,
            Self::PartiallyOverlaps => 4,
            Self::OrthogonalEvidenceFramework => 5,
            Self::CompetingOntology => 6,
        }
    }

    fn requires_direction(self) -> bool {
        matches!(
            self,
            Self::Generalizes | Self::Refines | Self::OperationalCriterionWithin
        )
    }

    /// This is a semantic-dependency class only. It is never an independence proof.
    pub fn dependency_class(self) -> SemanticDependencyClass {
        match self {
            Self::EquivalentSemanticTarget
            | Self::Generalizes
            | Self::Refines
            | Self::OperationalCriterionWithin => {
                SemanticDependencyClass::NestedOrCriterionDependent
            }
            Self::PartiallyOverlaps => SemanticDependencyClass::Overlapping,
            Self::OrthogonalEvidenceFramework | Self::CompetingOntology => {
                SemanticDependencyClass::PotentiallyNonNested
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeciesConceptRelationDirection {
    Symmetric,
    LeftToRight,
    RightToLeft,
}

impl SpeciesConceptRelationDirection {
    fn tag(self) -> u8 {
        match self {
            Self::Symmetric => 0,
            Self::LeftToRight => 1,
            Self::RightToLeft => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SemanticDependencyClass {
    NestedOrCriterionDependent,
    Overlapping,
    PotentiallyNonNested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptRelationAssertion {
    pub kind: SpeciesConceptRelationKind,
    pub direction: SpeciesConceptRelationDirection,
}

impl SpeciesConceptRelationAssertion {
    pub fn new(
        kind: SpeciesConceptRelationKind,
        direction: SpeciesConceptRelationDirection,
    ) -> Result<Self, RelationError> {
        let assertion = Self { kind, direction };
        assertion.validate_local()?;
        Ok(assertion)
    }

    pub fn dependency_class(&self) -> SemanticDependencyClass {
        self.kind.dependency_class()
    }

    fn validate_local(&self) -> Result<(), RelationError> {
        if self.kind.requires_direction() {
            if self.direction == SpeciesConceptRelationDirection::Symmetric {
                return Err(RelationError::DirectionalRelationNeedsDirection(self.kind));
            }
        } else if self.direction != SpeciesConceptRelationDirection::Symmetric {
            return Err(RelationError::SymmetricRelationCannotBeDirected(self.kind));
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        digest.update([self.kind.tag(), self.direction.tag()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciesConceptRelationAssessment {
    Qualified {
        assertion: SpeciesConceptRelationAssertion,
        semantic_mapping_digest: SpeciesConceptSemanticMappingDigest,
        scientific_reference_authority: AnalysisAuthorityRef,
        qualification_authority: AnalysisAuthorityRef,
    },
    Disputed {
        candidate: SpeciesConceptRelationAssertion,
        semantic_mapping_digest: SpeciesConceptSemanticMappingDigest,
        scientific_reference_authority: AnalysisAuthorityRef,
        qualification_authority: AnalysisAuthorityRef,
        dispute_authority: AnalysisAuthorityRef,
    },
    UnknownOrUnqualified {
        evidence_authority: AnalysisAuthorityRef,
    },
    Unavailable {
        evidence_authority: AnalysisAuthorityRef,
    },
}

impl SpeciesConceptRelationAssessment {
    fn validate_local(&self) -> Result<(), RelationError> {
        match self {
            Self::Qualified {
                assertion,
                scientific_reference_authority,
                qualification_authority,
                ..
            } => {
                assertion.validate_local()?;
                validate_authority_revision(
                    scientific_reference_authority,
                    "scientific_reference_revision",
                )?;
                validate_authority_revision(
                    qualification_authority,
                    "relation_qualification_revision",
                )?;
            }
            Self::Disputed {
                candidate,
                scientific_reference_authority,
                qualification_authority,
                dispute_authority,
                ..
            } => {
                candidate.validate_local()?;
                validate_authority_revision(
                    scientific_reference_authority,
                    "scientific_reference_revision",
                )?;
                validate_authority_revision(
                    qualification_authority,
                    "relation_qualification_revision",
                )?;
                validate_authority_revision(dispute_authority, "relation_dispute_revision")?;
            }
            Self::UnknownOrUnqualified { evidence_authority }
            | Self::Unavailable { evidence_authority } => {
                validate_authority_revision(evidence_authority, "relation_evidence_revision")?;
            }
        }
        Ok(())
    }

    fn tag(&self) -> u8 {
        match self {
            Self::Qualified { .. } => 0,
            Self::Disputed { .. } => 1,
            Self::UnknownOrUnqualified { .. } => 2,
            Self::Unavailable { .. } => 3,
        }
    }

    fn put(&self, digest: &mut Sha256) {
        digest.update([self.tag()]);
        match self {
            Self::Qualified {
                assertion,
                semantic_mapping_digest,
                scientific_reference_authority,
                qualification_authority,
            } => {
                assertion.put(digest);
                digest.update(semantic_mapping_digest.as_bytes());
                put_authority(digest, scientific_reference_authority);
                put_authority(digest, qualification_authority);
            }
            Self::Disputed {
                candidate,
                semantic_mapping_digest,
                scientific_reference_authority,
                qualification_authority,
                dispute_authority,
            } => {
                candidate.put(digest);
                digest.update(semantic_mapping_digest.as_bytes());
                put_authority(digest, scientific_reference_authority);
                put_authority(digest, qualification_authority);
                put_authority(digest, dispute_authority);
            }
            Self::UnknownOrUnqualified { evidence_authority }
            | Self::Unavailable { evidence_authority } => {
                put_authority(digest, evidence_authority);
            }
        }
    }

    fn is_missing_or_unknown(&self) -> bool {
        matches!(
            self,
            Self::UnknownOrUnqualified { .. } | Self::Unavailable { .. }
        )
    }

    pub fn dependency_class_if_qualified(&self) -> Option<SemanticDependencyClass> {
        match self {
            Self::Qualified { assertion, .. } => Some(assertion.dependency_class()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptRelationEvidence {
    evidence_version: u32,
    pub design: SpeciesConceptRelationDesign,
    pub design_digest: SpeciesConceptRelationDesignDigest,
    pub assessment: SpeciesConceptRelationAssessment,
}

impl SpeciesConceptRelationEvidence {
    pub fn evaluate(
        design: &ValidatedSpeciesConceptRelationDesign<'_>,
        assessment: SpeciesConceptRelationAssessment,
    ) -> Result<Self, RelationError> {
        assessment.validate_local()?;
        if assessment.is_missing_or_unknown()
            && design.design().missing_policy == RelationMissingPolicy::FailClosed
        {
            return Err(RelationError::MissingRelationEvidenceFailClosed);
        }
        let evidence = Self {
            evidence_version: SPECIES_CONCEPT_RELATION_EVIDENCE_VERSION,
            design: design.design().clone(),
            design_digest: design.design_digest(),
            assessment,
        };
        evidence.validate_local()?;
        Ok(evidence)
    }

    pub fn canonical_digest(&self) -> Result<SpeciesConceptRelationEvidenceDigest, RelationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(EVIDENCE_DOMAIN);
        put_u32(&mut digest, self.evidence_version);
        digest.update(self.design_digest.as_bytes());
        self.assessment.put(&mut digest);
        Ok(SpeciesConceptRelationEvidenceDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), RelationError> {
        if self.evidence_version != SPECIES_CONCEPT_RELATION_EVIDENCE_VERSION {
            return Err(RelationError::UnsupportedEvidenceVersion(
                self.evidence_version,
            ));
        }
        let actual_design_digest = self.design.canonical_digest()?;
        if actual_design_digest != self.design_digest {
            return Err(RelationError::DesignDigestMismatch);
        }
        self.assessment.validate_local()?;
        if self.assessment.is_missing_or_unknown()
            && self.design.missing_policy == RelationMissingPolicy::FailClosed
        {
            return Err(RelationError::MissingRelationEvidenceFailClosed);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated semantic-relation evidence should gate downstream robustness reasoning"]
pub struct ValidatedSpeciesConceptRelationEvidence<'a> {
    evidence: &'a SpeciesConceptRelationEvidence,
    evidence_digest: SpeciesConceptRelationEvidenceDigest,
}

impl<'a> ValidatedSpeciesConceptRelationEvidence<'a> {
    pub fn validate_current(
        evidence: &'a SpeciesConceptRelationEvidence,
        design: &ValidatedSpeciesConceptRelationDesign<'_>,
        current_assessment: SpeciesConceptRelationAssessment,
    ) -> Result<Self, RelationError> {
        evidence.validate_local()?;
        let recomputed = SpeciesConceptRelationEvidence::evaluate(design, current_assessment)?;
        if recomputed != *evidence {
            return Err(RelationError::EvidenceReplayMismatch);
        }
        Ok(Self {
            evidence,
            evidence_digest: evidence.canonical_digest()?,
        })
    }

    pub fn evidence(&self) -> &'a SpeciesConceptRelationEvidence {
        self.evidence
    }

    pub fn evidence_digest(&self) -> SpeciesConceptRelationEvidenceDigest {
        self.evidence_digest
    }

    pub fn dependency_class_if_qualified(&self) -> Option<SemanticDependencyClass> {
        self.evidence.assessment.dependency_class_if_qualified()
    }
}

pub fn species_concept_relation_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("species-concept-semantic-relation-v1")
            .expect("static relation method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

pub fn semantic_mapping_digest_v1(
    design: &SpeciesConceptRelationDesign,
    assertion: &SpeciesConceptRelationAssertion,
    mapping_bytes: &[u8],
) -> Result<SpeciesConceptSemanticMappingDigest, RelationError> {
    design.validate_local()?;
    assertion.validate_local()?;
    let mut digest = Sha256::new();
    digest.update(MAPPING_DOMAIN);
    digest.update(design.canonical_digest()?.as_bytes());
    assertion.put(&mut digest);
    put_u64(&mut digest, mapping_bytes.len() as u64);
    digest.update(mapping_bytes);
    Ok(SpeciesConceptSemanticMappingDigest::new(
        digest.finalize().into(),
    ))
}

fn validate_identity(identity: &OpenSpeciesConceptIdentity) -> Result<(), RelationError> {
    if identity.family_version == 0 {
        return Err(RelationError::ZeroConceptFamilyVersion);
    }
    Ok(())
}

fn validate_authority_revision(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), RelationError> {
    if authority.revision == 0 {
        return Err(RelationError::ZeroRevision(field));
    }
    Ok(())
}

fn validate_id(field: &'static str, value: &str) -> Result<(), RelationError> {
    if value.is_empty()
        || value.len() > 160
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(RelationError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
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

fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_be_bytes());
}

fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_be_bytes());
}

fn fmt_hex(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum RelationError {
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    ZeroRevision(&'static str),
    ZeroConceptFamilyVersion,
    UnsupportedDesignVersion(u32),
    UnsupportedEvidenceVersion(u32),
    SameConceptEndpoint,
    NonCanonicalEndpoints,
    RelationRuleMismatch,
    DesignReplayMismatch,
    DirectionalRelationNeedsDirection(SpeciesConceptRelationKind),
    SymmetricRelationCannotBeDirected(SpeciesConceptRelationKind),
    MissingRelationEvidenceFailClosed,
    DesignDigestMismatch,
    EvidenceReplayMismatch,
}

impl fmt::Display for RelationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field, value } => {
                write!(f, "invalid {field} value {value:?}")
            }
            Self::ZeroRevision(field) => write!(f, "{field} must be nonzero"),
            Self::ZeroConceptFamilyVersion => {
                write!(f, "concept endpoint family_version must be nonzero")
            }
            Self::UnsupportedDesignVersion(version) => {
                write!(f, "unsupported species-concept relation design version {version}")
            }
            Self::UnsupportedEvidenceVersion(version) => {
                write!(f, "unsupported species-concept relation evidence version {version}")
            }
            Self::SameConceptEndpoint => {
                write!(f, "semantic relation endpoints must be distinct conceptual identities")
            }
            Self::NonCanonicalEndpoints => {
                write!(f, "semantic relation endpoints are not in canonical identity order")
            }
            Self::RelationRuleMismatch => {
                write!(f, "relation design does not bind the built-in V1 semantic rule")
            }
            Self::DesignReplayMismatch => {
                write!(f, "persisted relation design does not replay from current inputs")
            }
            Self::DirectionalRelationNeedsDirection(kind) => {
                write!(f, "directional relation {kind:?} requires an explicit direction")
            }
            Self::SymmetricRelationCannotBeDirected(kind) => {
                write!(f, "symmetric relation {kind:?} cannot carry a direction")
            }
            Self::MissingRelationEvidenceFailClosed => {
                write!(f, "relation evidence is missing/unknown under fail-closed policy")
            }
            Self::DesignDigestMismatch => {
                write!(f, "embedded relation design does not match its persisted digest")
            }
            Self::EvidenceReplayMismatch => {
                write!(f, "persisted relation evidence does not replay from current evidence")
            }
        }
    }
}

impl Error for RelationError {}
