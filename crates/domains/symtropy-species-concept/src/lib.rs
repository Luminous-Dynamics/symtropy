// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Open model-family descriptor waist for species-concept reasoning.
//!
//! This crate does not decide species status. It provides a canonical,
//! family-neutral representation that concrete, independently qualified species
//! concept authorities can project into without forcing their evidence semantics
//! into one global enum.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, SpeciesConceptCapability,
    SpeciesConceptDomainConstraint, SpeciesConceptEvidenceRequirement,
    ValidatedSpeciesConceptAuthority,
};

pub const SPECIES_CONCEPT_FAMILY_DESCRIPTOR_VERSION: u32 = 1;
const DESCRIPTOR_DOMAIN: &[u8] = b"symtropy:species-concept:family-descriptor:v1\0";
const SCHEMA_DOMAIN: &[u8] = b"symtropy:species-concept:schema:v1\0";
const TERM_DOMAIN: &[u8] = b"symtropy:species-concept:schema-term:v1\0";
const CAPABILITY_DOMAIN: &[u8] = b"symtropy:species-concept:capability:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:descriptor-rule:v1\0";
const RULE_SPEC: &[u8] = b"open species-concept descriptor v1: family-neutral identity/provenance only; source authority remains family-specific; schemas use canonical open IDs and digest-bound terms; capability membership is explicit; persisted descriptor is not current scientific authority; strict-BSC projection preserves exact E1 conceptual and authority identities; no species outcome, no historical transition outcome, no cross-model robustness, no universal taxonomy claim";

macro_rules! open_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, DescriptorError> {
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

open_id!(OpenSpeciesConceptFamilyId, "OpenSpeciesConceptFamilyId");
open_id!(OpenSpeciesConceptModelId, "OpenSpeciesConceptModelId");
open_id!(SpeciesConceptSourceKindId, "SpeciesConceptSourceKindId");
open_id!(SpeciesConceptCapabilityId, "SpeciesConceptCapabilityId");
open_id!(SpeciesConceptSchemaId, "SpeciesConceptSchemaId");
open_id!(SpeciesConceptSchemaTermId, "SpeciesConceptSchemaTermId");

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

digest_type!(OpenSpeciesConceptContentDigest);
digest_type!(SpeciesConceptSourceAuthorityDigest);
digest_type!(SpeciesConceptSourceValidityDomainDigest);
digest_type!(SpeciesConceptCapabilityContentDigest);
digest_type!(SpeciesConceptSchemaTermDigest);
digest_type!(SpeciesConceptSchemaDigest);
digest_type!(SpeciesConceptFamilyDescriptorDigest);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpenSpeciesConceptIdentity {
    pub family_id: OpenSpeciesConceptFamilyId,
    pub family_version: u32,
    pub content_digest: OpenSpeciesConceptContentDigest,
}

impl OpenSpeciesConceptIdentity {
    pub fn new(
        family_id: OpenSpeciesConceptFamilyId,
        family_version: u32,
        content_digest: OpenSpeciesConceptContentDigest,
    ) -> Result<Self, DescriptorError> {
        require_nonzero(family_version, "family_version")?;
        Ok(Self {
            family_id,
            family_version,
            content_digest,
        })
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.family_id.as_str());
        put_u32(digest, self.family_version);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesConceptCapabilityRef {
    pub capability_id: SpeciesConceptCapabilityId,
    pub revision: u32,
    pub content_digest: SpeciesConceptCapabilityContentDigest,
}

impl SpeciesConceptCapabilityRef {
    pub fn new(
        capability_id: SpeciesConceptCapabilityId,
        revision: u32,
        content_digest: SpeciesConceptCapabilityContentDigest,
    ) -> Result<Self, DescriptorError> {
        require_nonzero(revision, "capability_revision")?;
        Ok(Self {
            capability_id,
            revision,
            content_digest,
        })
    }

    fn validate_local(&self) -> Result<(), DescriptorError> {
        require_nonzero(self.revision, "capability_revision")
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.capability_id.as_str());
        put_u32(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesConceptSchemaTerm {
    pub term_id: SpeciesConceptSchemaTermId,
    pub revision: u32,
    pub content_digest: SpeciesConceptSchemaTermDigest,
}

impl SpeciesConceptSchemaTerm {
    pub fn new(
        term_id: SpeciesConceptSchemaTermId,
        revision: u32,
        content_digest: SpeciesConceptSchemaTermDigest,
    ) -> Result<Self, DescriptorError> {
        require_nonzero(revision, "schema_term_revision")?;
        Ok(Self {
            term_id,
            revision,
            content_digest,
        })
    }

    fn validate_local(&self) -> Result<(), DescriptorError> {
        require_nonzero(self.revision, "schema_term_revision")
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.term_id.as_str());
        put_u32(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptSchema {
    pub schema_id: SpeciesConceptSchemaId,
    pub revision: u32,
    pub terms: Vec<SpeciesConceptSchemaTerm>,
    pub content_digest: SpeciesConceptSchemaDigest,
}

impl SpeciesConceptSchema {
    pub fn declare(
        schema_id: SpeciesConceptSchemaId,
        revision: u32,
        terms: impl IntoIterator<Item = SpeciesConceptSchemaTerm>,
    ) -> Result<Self, DescriptorError> {
        require_nonzero(revision, "schema_revision")?;
        let mut terms: Vec<_> = terms.into_iter().collect();
        for term in &terms {
            term.validate_local()?;
        }
        terms.sort();
        validate_unique_terms(&terms)?;
        let content_digest = derive_schema_digest(&schema_id, revision, &terms);
        Ok(Self {
            schema_id,
            revision,
            terms,
            content_digest,
        })
    }

    fn validate_local(&self) -> Result<(), DescriptorError> {
        require_nonzero(self.revision, "schema_revision")?;
        for term in &self.terms {
            term.validate_local()?;
        }
        let mut canonical = self.terms.clone();
        canonical.sort();
        validate_unique_terms(&canonical)?;
        if canonical != self.terms {
            return Err(DescriptorError::NonCanonicalSchemaOrder(
                self.schema_id.clone(),
            ));
        }
        if self.content_digest != derive_schema_digest(&self.schema_id, self.revision, &self.terms)
        {
            return Err(DescriptorError::SchemaDigestMismatch(
                self.schema_id.clone(),
            ));
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.schema_id.as_str());
        put_u32(digest, self.revision);
        put_u64(digest, self.terms.len() as u64);
        for term in &self.terms {
            term.put(digest);
        }
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptSourceAuthorityRef {
    pub source_kind_id: SpeciesConceptSourceKindId,
    pub source_kind_version: u32,
    pub authority_digest: SpeciesConceptSourceAuthorityDigest,
    pub qualification_authority: AnalysisAuthorityRef,
    pub validity_domain_digest: SpeciesConceptSourceValidityDomainDigest,
    pub source_adapter_rule_authority: AnalysisAuthorityRef,
}

impl SpeciesConceptSourceAuthorityRef {
    pub fn new(
        source_kind_id: SpeciesConceptSourceKindId,
        source_kind_version: u32,
        authority_digest: SpeciesConceptSourceAuthorityDigest,
        qualification_authority: AnalysisAuthorityRef,
        validity_domain_digest: SpeciesConceptSourceValidityDomainDigest,
        source_adapter_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, DescriptorError> {
        require_nonzero(source_kind_version, "source_kind_version")?;
        Ok(Self {
            source_kind_id,
            source_kind_version,
            authority_digest,
            qualification_authority,
            validity_domain_digest,
            source_adapter_rule_authority,
        })
    }

    fn validate_local(&self) -> Result<(), DescriptorError> {
        require_nonzero(self.source_kind_version, "source_kind_version")
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.source_kind_id.as_str());
        put_u32(digest, self.source_kind_version);
        digest.update(self.authority_digest.as_bytes());
        put_authority(digest, &self.qualification_authority);
        digest.update(self.validity_domain_digest.as_bytes());
        put_authority(digest, &self.source_adapter_rule_authority);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptFamilyDescriptor {
    descriptor_version: u32,
    pub conceptual_identity: OpenSpeciesConceptIdentity,
    pub model_id: OpenSpeciesConceptModelId,
    pub source_authority: SpeciesConceptSourceAuthorityRef,
    pub capabilities: Vec<SpeciesConceptCapabilityRef>,
    pub evidence_schema: SpeciesConceptSchema,
    pub domain_schema: SpeciesConceptSchema,
    pub descriptor_rule_authority: AnalysisAuthorityRef,
}

impl SpeciesConceptFamilyDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        conceptual_identity: OpenSpeciesConceptIdentity,
        model_id: OpenSpeciesConceptModelId,
        source_authority: SpeciesConceptSourceAuthorityRef,
        capabilities: impl IntoIterator<Item = SpeciesConceptCapabilityRef>,
        evidence_schema: SpeciesConceptSchema,
        domain_schema: SpeciesConceptSchema,
        descriptor_rule_authority: AnalysisAuthorityRef,
    ) -> Result<Self, DescriptorError> {
        let mut capabilities: Vec<_> = capabilities.into_iter().collect();
        for capability in &capabilities {
            capability.validate_local()?;
        }
        capabilities.sort();
        validate_unique_capabilities(&capabilities)?;
        let descriptor = Self {
            descriptor_version: SPECIES_CONCEPT_FAMILY_DESCRIPTOR_VERSION,
            conceptual_identity,
            model_id,
            source_authority,
            capabilities,
            evidence_schema,
            domain_schema,
            descriptor_rule_authority,
        };
        descriptor.validate_local()?;
        Ok(descriptor)
    }

    pub fn from_strict_biological(
        source: &ValidatedSpeciesConceptAuthority<'_>,
    ) -> Result<Self, DescriptorError> {
        let authority = source.authority();
        let source_identity = source.conceptual_identity();
        let conceptual_identity = OpenSpeciesConceptIdentity::new(
            OpenSpeciesConceptFamilyId::new(source_identity.family_id.as_str())?,
            source_identity.family_version,
            OpenSpeciesConceptContentDigest::new(*source_identity.content_digest.as_bytes()),
        )?;
        let model_id = OpenSpeciesConceptModelId::new(authority.model_id.as_str())?;
        let source_authority = SpeciesConceptSourceAuthorityRef::new(
            SpeciesConceptSourceKindId::new("sel10e1-strict-bsc-authority")?,
            1,
            SpeciesConceptSourceAuthorityDigest::new(*source.authority_digest().as_bytes()),
            authority.qualification.authority.clone(),
            SpeciesConceptSourceValidityDomainDigest::new(*authority.validity_domain_digest.as_bytes()),
            authority.adapter_rule_authority.clone(),
        )?;
        let capabilities = authority
            .capabilities
            .iter()
            .copied()
            .map(|capability| strict_capability_ref(capability, &conceptual_identity))
            .collect::<Result<Vec<_>, _>>()?;
        let evidence_schema = strict_evidence_schema(authority, &conceptual_identity)?;
        let domain_schema = strict_domain_schema(authority, &conceptual_identity)?;
        Self::declare(
            conceptual_identity,
            model_id,
            source_authority,
            capabilities,
            evidence_schema,
            domain_schema,
            open_species_concept_descriptor_rule_v1(),
        )
    }

    pub fn canonical_digest(&self) -> Result<SpeciesConceptFamilyDescriptorDigest, DescriptorError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESCRIPTOR_DOMAIN);
        put_u32(&mut digest, self.descriptor_version);
        self.conceptual_identity.put(&mut digest);
        put_text(&mut digest, self.model_id.as_str());
        self.source_authority.put(&mut digest);
        put_u64(&mut digest, self.capabilities.len() as u64);
        for capability in &self.capabilities {
            capability.put(&mut digest);
        }
        self.evidence_schema.put(&mut digest);
        self.domain_schema.put(&mut digest);
        put_authority(&mut digest, &self.descriptor_rule_authority);
        Ok(SpeciesConceptFamilyDescriptorDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), DescriptorError> {
        if self.descriptor_version != SPECIES_CONCEPT_FAMILY_DESCRIPTOR_VERSION {
            return Err(DescriptorError::UnsupportedVersion(self.descriptor_version));
        }
        require_nonzero(self.conceptual_identity.family_version, "family_version")?;
        self.source_authority.validate_local()?;
        if self.capabilities.is_empty() {
            return Err(DescriptorError::EmptyCapabilitySurface);
        }
        for capability in &self.capabilities {
            capability.validate_local()?;
        }
        let mut canonical_capabilities = self.capabilities.clone();
        canonical_capabilities.sort();
        validate_unique_capabilities(&canonical_capabilities)?;
        if canonical_capabilities != self.capabilities {
            return Err(DescriptorError::NonCanonicalCapabilityOrder);
        }
        self.evidence_schema.validate_local()?;
        self.domain_schema.validate_local()?;
        if self.evidence_schema.terms.is_empty() {
            return Err(DescriptorError::EmptyEvidenceSchema);
        }
        if self.evidence_schema.schema_id == self.domain_schema.schema_id {
            return Err(DescriptorError::SchemaRoleCollision);
        }
        if self.descriptor_rule_authority != open_species_concept_descriptor_rule_v1() {
            return Err(DescriptorError::DescriptorRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated descriptor currentness should be consumed instead of a restored descriptor"]
pub struct ValidatedSpeciesConceptFamilyDescriptor<'a> {
    descriptor: &'a SpeciesConceptFamilyDescriptor,
    descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
}

impl<'a> ValidatedSpeciesConceptFamilyDescriptor<'a> {
    pub fn validate_current_strict_biological(
        descriptor: &'a SpeciesConceptFamilyDescriptor,
        source: &ValidatedSpeciesConceptAuthority<'_>,
    ) -> Result<Self, DescriptorError> {
        descriptor.validate_local()?;
        let recomputed = SpeciesConceptFamilyDescriptor::from_strict_biological(source)?;
        if recomputed != *descriptor {
            return Err(DescriptorError::ReplayMismatch);
        }
        Ok(Self {
            descriptor,
            descriptor_digest: descriptor.canonical_digest()?,
        })
    }

    pub fn descriptor(&self) -> &'a SpeciesConceptFamilyDescriptor {
        self.descriptor
    }

    pub fn descriptor_digest(&self) -> SpeciesConceptFamilyDescriptorDigest {
        self.descriptor_digest
    }

    pub fn conceptual_identity(&self) -> &OpenSpeciesConceptIdentity {
        &self.descriptor.conceptual_identity
    }
}

pub fn open_species_concept_descriptor_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("open-species-concept-family-descriptor-v1")
            .expect("static descriptor rule method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

fn strict_capability_ref(
    capability: SpeciesConceptCapability,
    identity: &OpenSpeciesConceptIdentity,
) -> Result<SpeciesConceptCapabilityRef, DescriptorError> {
    let id = match capability {
        SpeciesConceptCapability::CurrentSpeciesStatus => "current-species-status",
        SpeciesConceptCapability::HistoricalTransitionInterval => "historical-transition-interval",
    };
    let mut digest = Sha256::new();
    digest.update(CAPABILITY_DOMAIN);
    identity.put(&mut digest);
    put_text(&mut digest, id);
    SpeciesConceptCapabilityRef::new(
        SpeciesConceptCapabilityId::new(id)?,
        1,
        SpeciesConceptCapabilityContentDigest::new(digest.finalize().into()),
    )
}

fn strict_evidence_schema(
    source: &symtropy_evolution_core::SpeciesConceptAuthority,
    identity: &OpenSpeciesConceptIdentity,
) -> Result<SpeciesConceptSchema, DescriptorError> {
    let terms = source
        .evidence_requirements
        .iter()
        .copied()
        .map(|requirement| {
            let id = match requirement {
                SpeciesConceptEvidenceRequirement::ExplicitQualifiedModelApplicability => {
                    "explicit-qualified-model-applicability"
                }
                SpeciesConceptEvidenceRequirement::CurrentCompleteReproductiveBarrier => {
                    "current-complete-reproductive-barrier"
                }
                SpeciesConceptEvidenceRequirement::PersistentOrRecontactLineageHistory => {
                    "persistent-or-recontact-lineage-history"
                }
                SpeciesConceptEvidenceRequirement::HistoricalTransitionTemporalEvidence => {
                    "historical-transition-temporal-evidence"
                }
            };
            strict_schema_term(id, identity, b"evidence")
        })
        .collect::<Result<Vec<_>, _>>()?;
    SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("strict-bsc-evidence-schema")?,
        1,
        terms,
    )
}

fn strict_domain_schema(
    source: &symtropy_evolution_core::SpeciesConceptAuthority,
    identity: &OpenSpeciesConceptIdentity,
) -> Result<SpeciesConceptSchema, DescriptorError> {
    let terms = source
        .domain_constraints
        .iter()
        .copied()
        .map(|constraint| {
            let id = match constraint {
                SpeciesConceptDomainConstraint::ReproductiveIsolationMustBeBiologicallyMeaningful => {
                    "reproductive-isolation-biologically-meaningful"
                }
            };
            strict_schema_term(id, identity, b"domain")
        })
        .collect::<Result<Vec<_>, _>>()?;
    SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("strict-bsc-domain-schema")?,
        1,
        terms,
    )
}

fn strict_schema_term(
    id: &str,
    identity: &OpenSpeciesConceptIdentity,
    role: &[u8],
) -> Result<SpeciesConceptSchemaTerm, DescriptorError> {
    let mut digest = Sha256::new();
    digest.update(TERM_DOMAIN);
    put_u64(&mut digest, role.len() as u64);
    digest.update(role);
    identity.put(&mut digest);
    put_text(&mut digest, id);
    SpeciesConceptSchemaTerm::new(
        SpeciesConceptSchemaTermId::new(id)?,
        1,
        SpeciesConceptSchemaTermDigest::new(digest.finalize().into()),
    )
}

fn derive_schema_digest(
    schema_id: &SpeciesConceptSchemaId,
    revision: u32,
    terms: &[SpeciesConceptSchemaTerm],
) -> SpeciesConceptSchemaDigest {
    let mut digest = Sha256::new();
    digest.update(SCHEMA_DOMAIN);
    put_text(&mut digest, schema_id.as_str());
    put_u32(&mut digest, revision);
    put_u64(&mut digest, terms.len() as u64);
    for term in terms {
        term.put(&mut digest);
    }
    SpeciesConceptSchemaDigest::new(digest.finalize().into())
}

fn validate_unique_terms(terms: &[SpeciesConceptSchemaTerm]) -> Result<(), DescriptorError> {
    for pair in terms.windows(2) {
        if pair[0].term_id == pair[1].term_id {
            return Err(DescriptorError::DuplicateSchemaTerm(
                pair[0].term_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_unique_capabilities(
    capabilities: &[SpeciesConceptCapabilityRef],
) -> Result<(), DescriptorError> {
    for pair in capabilities.windows(2) {
        if pair[0].capability_id == pair[1].capability_id {
            return Err(DescriptorError::DuplicateCapability(
                pair[0].capability_id.clone(),
            ));
        }
    }
    Ok(())
}

fn require_nonzero(value: u32, field: &'static str) -> Result<(), DescriptorError> {
    if value == 0 {
        Err(DescriptorError::ZeroVersion(field))
    } else {
        Ok(())
    }
}

fn validate_id(field: &'static str, value: &str) -> Result<(), DescriptorError> {
    if value.is_empty()
        || value.len() > 160
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(DescriptorError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
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
pub enum DescriptorError {
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    ZeroVersion(&'static str),
    UnsupportedVersion(u32),
    EmptyCapabilitySurface,
    EmptyEvidenceSchema,
    DuplicateCapability(SpeciesConceptCapabilityId),
    DuplicateSchemaTerm(SpeciesConceptSchemaTermId),
    NonCanonicalCapabilityOrder,
    NonCanonicalSchemaOrder(SpeciesConceptSchemaId),
    SchemaDigestMismatch(SpeciesConceptSchemaId),
    SchemaRoleCollision,
    DescriptorRuleMismatch,
    ReplayMismatch,
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field, value } => {
                write!(f, "invalid {field} value {value:?}")
            }
            Self::ZeroVersion(field) => write!(f, "{field} must be nonzero"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported species-concept descriptor version {version}")
            }
            Self::EmptyCapabilitySurface => {
                write!(f, "species-concept descriptor must expose at least one capability")
            }
            Self::EmptyEvidenceSchema => {
                write!(f, "species-concept descriptor must expose a nonempty evidence schema")
            }
            Self::DuplicateCapability(id) => {
                write!(f, "duplicate species-concept capability {}", id.as_str())
            }
            Self::DuplicateSchemaTerm(id) => {
                write!(f, "duplicate species-concept schema term {}", id.as_str())
            }
            Self::NonCanonicalCapabilityOrder => {
                write!(f, "species-concept capabilities are not in canonical order")
            }
            Self::NonCanonicalSchemaOrder(id) => write!(
                f,
                "species-concept schema {} is not in canonical term order",
                id.as_str()
            ),
            Self::SchemaDigestMismatch(id) => write!(
                f,
                "species-concept schema {} does not match its content digest",
                id.as_str()
            ),
            Self::SchemaRoleCollision => write!(
                f,
                "evidence and domain schemas must use distinct schema identities"
            ),
            Self::DescriptorRuleMismatch => write!(
                f,
                "descriptor does not bind the built-in open-waist V1 rule"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted descriptor does not replay from the current concrete family authority"
            ),
        }
    }
}

impl Error for DescriptorError {}
