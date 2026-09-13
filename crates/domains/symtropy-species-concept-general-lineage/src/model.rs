use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId,
};
use symtropy_species_concept::{
    open_species_concept_descriptor_rule_v1, DescriptorError, OpenSpeciesConceptContentDigest,
    OpenSpeciesConceptFamilyId, OpenSpeciesConceptIdentity, OpenSpeciesConceptModelId,
    SpeciesConceptCapabilityContentDigest, SpeciesConceptCapabilityId,
    SpeciesConceptCapabilityRef, SpeciesConceptFamilyDescriptor,
    SpeciesConceptFamilyDescriptorDigest, SpeciesConceptSchema, SpeciesConceptSchemaId,
    SpeciesConceptSchemaTerm, SpeciesConceptSchemaTermDigest, SpeciesConceptSchemaTermId,
    SpeciesConceptSourceAuthorityDigest, SpeciesConceptSourceAuthorityRef,
    SpeciesConceptSourceKindId, SpeciesConceptSourceValidityDomainDigest,
};

pub const GENERAL_LINEAGE_SPECIES_MODEL_VERSION: u32 = 1;
pub const GENERAL_LINEAGE_SPECIES_FAMILY_VERSION: u32 = 1;
const MODEL_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:model:v1\0";
const CONTENT_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:content:v1\0";
const VALIDITY_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:validity-domain:v1\0";
const MODEL_RULE_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:model-rule:v1\0";
const DESCRIPTOR_ADAPTER_DOMAIN: &[u8] =
    b"symtropy:species-concept:general-lineage:descriptor-adapter:v1\0";
const CAPABILITY_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:capability:v1\0";
const SCHEMA_TERM_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:schema-term:v1\0";

const MODEL_SPEC: &[u8] = b"general-lineage species model v1: species are separately evolving metapopulation lineage segments; current-status authority only; separately evolving lineage is the conceptual target; operational properties are evidence rather than universal defining properties; reproductive isolation is optional corroboration; monophyly is not universally required; population subdivision alone is insufficient; no universal divergence-distance threshold; evidence channels and dependency groups are preregistered before outcomes; missing evidence is not negative evidence; contradictory evidence remains explicit; sexual and asexual targets may be in-domain as separately qualified; no historical speciation-time claim; no scalar species score; no universal taxonomy truth";
const DESCRIPTOR_ADAPTER_SPEC: &[u8] = b"general-lineage descriptor adapter v1: preserve exact general-lineage conceptual content and current model authority; expose current-species-status capability only; publish open evidence/domain schemas without target outcomes; source model qualification remains provenance; persisted descriptor is not current authority";

macro_rules! id_type {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, GeneralLineageModelError> {
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

id_type!(GeneralLineageSpeciesModelId, "GeneralLineageSpeciesModelId");

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

digest_type!(GeneralLineageModelContentDigest);
digest_type!(GeneralLineageValidityDomainDigest);
digest_type!(GeneralLineageSpeciesModelDigest);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GeneralLineageReproductiveModePolicy {
    SexualOnly,
    AsexualOnly,
    SexualOrAsexual,
}

impl GeneralLineageReproductiveModePolicy {
    fn tag(self) -> u8 {
        match self {
            Self::SexualOnly => 0,
            Self::AsexualOnly => 1,
            Self::SexualOrAsexual => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageValidityDomainRef {
    pub authority: AnalysisAuthorityRef,
    pub reproductive_mode: GeneralLineageReproductiveModePolicy,
}

impl GeneralLineageValidityDomainRef {
    pub fn new(
        authority: AnalysisAuthorityRef,
        reproductive_mode: GeneralLineageReproductiveModePolicy,
    ) -> Result<Self, GeneralLineageModelError> {
        validate_authority(&authority, "validity_domain_revision")?;
        Ok(Self {
            authority,
            reproductive_mode,
        })
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageValidityDomainDigest, GeneralLineageModelError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(VALIDITY_DOMAIN);
        put_authority(&mut digest, &self.authority);
        digest.update([self.reproductive_mode.tag()]);
        Ok(GeneralLineageValidityDomainDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), GeneralLineageModelError> {
        validate_authority(&self.authority, "validity_domain_revision")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageSpeciesModel {
    model_version: u32,
    pub model_id: GeneralLineageSpeciesModelId,
    pub model_content_digest: GeneralLineageModelContentDigest,
    pub validity_domain: GeneralLineageValidityDomainRef,
    pub qualification_authority: AnalysisAuthorityRef,
    pub model_rule_authority: AnalysisAuthorityRef,
}

impl GeneralLineageSpeciesModel {
    pub fn declare(
        model_id: GeneralLineageSpeciesModelId,
        validity_domain: GeneralLineageValidityDomainRef,
        qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageModelError> {
        validity_domain.validate_local()?;
        validate_authority(&qualification_authority, "model_qualification_revision")?;
        let model = Self {
            model_version: GENERAL_LINEAGE_SPECIES_MODEL_VERSION,
            model_id,
            model_content_digest: general_lineage_species_model_content_digest_v1(),
            validity_domain,
            qualification_authority,
            model_rule_authority: general_lineage_species_model_rule_v1(),
        };
        model.validate_local()?;
        Ok(model)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageSpeciesModelDigest, GeneralLineageModelError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(MODEL_DOMAIN);
        put_u32(&mut digest, self.model_version);
        put_text(&mut digest, self.model_id.as_str());
        digest.update(self.model_content_digest.as_bytes());
        digest.update(self.validity_domain.canonical_digest()?.as_bytes());
        put_authority(&mut digest, &self.qualification_authority);
        put_authority(&mut digest, &self.model_rule_authority);
        Ok(GeneralLineageSpeciesModelDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), GeneralLineageModelError> {
        if self.model_version != GENERAL_LINEAGE_SPECIES_MODEL_VERSION {
            return Err(GeneralLineageModelError::UnsupportedModelVersion(
                self.model_version,
            ));
        }
        self.validity_domain.validate_local()?;
        validate_authority(&self.qualification_authority, "model_qualification_revision")?;
        if self.model_content_digest != general_lineage_species_model_content_digest_v1() {
            return Err(GeneralLineageModelError::ModelContentMismatch);
        }
        if self.model_rule_authority != general_lineage_species_model_rule_v1() {
            return Err(GeneralLineageModelError::ModelRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated general-lineage model should gate target classification design"]
pub struct ValidatedGeneralLineageSpeciesModel<'a> {
    model: &'a GeneralLineageSpeciesModel,
    model_digest: GeneralLineageSpeciesModelDigest,
}

impl<'a> ValidatedGeneralLineageSpeciesModel<'a> {
    pub fn validate_current(
        model: &'a GeneralLineageSpeciesModel,
        validity_domain: GeneralLineageValidityDomainRef,
        qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageModelError> {
        model.validate_local()?;
        let recomputed = GeneralLineageSpeciesModel::declare(
            model.model_id.clone(),
            validity_domain,
            qualification_authority,
        )?;
        if recomputed != *model {
            return Err(GeneralLineageModelError::ModelReplayMismatch);
        }
        Ok(Self {
            model,
            model_digest: model.canonical_digest()?,
        })
    }

    pub fn model(&self) -> &'a GeneralLineageSpeciesModel {
        self.model
    }

    pub fn model_digest(&self) -> GeneralLineageSpeciesModelDigest {
        self.model_digest
    }
}

#[derive(Debug)]
#[must_use = "validated family descriptor should accompany current general-lineage model authority"]
pub struct ValidatedGeneralLineageFamilyDescriptor<'a> {
    descriptor: &'a SpeciesConceptFamilyDescriptor,
    descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
    model_digest: GeneralLineageSpeciesModelDigest,
}

impl<'a> ValidatedGeneralLineageFamilyDescriptor<'a> {
    pub fn validate_current(
        descriptor: &'a SpeciesConceptFamilyDescriptor,
        model: &ValidatedGeneralLineageSpeciesModel<'_>,
    ) -> Result<Self, GeneralLineageModelError> {
        let recomputed = general_lineage_family_descriptor(model)?;
        if recomputed != *descriptor {
            return Err(GeneralLineageModelError::DescriptorReplayMismatch);
        }
        Ok(Self {
            descriptor,
            descriptor_digest: descriptor.canonical_digest()?,
            model_digest: model.model_digest(),
        })
    }

    pub fn descriptor(&self) -> &'a SpeciesConceptFamilyDescriptor {
        self.descriptor
    }

    pub fn descriptor_digest(&self) -> SpeciesConceptFamilyDescriptorDigest {
        self.descriptor_digest
    }

    pub fn model_digest(&self) -> GeneralLineageSpeciesModelDigest {
        self.model_digest
    }

    pub fn conceptual_identity(&self) -> &OpenSpeciesConceptIdentity {
        &self.descriptor.conceptual_identity
    }
}

pub fn general_lineage_species_model_content_digest_v1() -> GeneralLineageModelContentDigest {
    let mut digest = Sha256::new();
    digest.update(CONTENT_DOMAIN);
    put_u64(&mut digest, MODEL_SPEC.len() as u64);
    digest.update(MODEL_SPEC);
    GeneralLineageModelContentDigest::new(digest.finalize().into())
}

pub fn general_lineage_species_model_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(MODEL_RULE_DOMAIN);
    put_u64(&mut digest, MODEL_SPEC.len() as u64);
    digest.update(MODEL_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("general-lineage-species-model-v1")
            .expect("static general-lineage model rule ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

pub fn general_lineage_descriptor_adapter_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(DESCRIPTOR_ADAPTER_DOMAIN);
    put_u64(&mut digest, DESCRIPTOR_ADAPTER_SPEC.len() as u64);
    digest.update(DESCRIPTOR_ADAPTER_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("general-lineage-species-descriptor-adapter-v1")
            .expect("static descriptor adapter rule ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

pub fn general_lineage_family_descriptor(
    model: &ValidatedGeneralLineageSpeciesModel<'_>,
) -> Result<SpeciesConceptFamilyDescriptor, GeneralLineageModelError> {
    let source = model.model();
    let identity = OpenSpeciesConceptIdentity::new(
        OpenSpeciesConceptFamilyId::new("general-lineage-species")?,
        GENERAL_LINEAGE_SPECIES_FAMILY_VERSION,
        OpenSpeciesConceptContentDigest::new(*source.model_content_digest.as_bytes()),
    )?;
    let model_id = OpenSpeciesConceptModelId::new(source.model_id.as_str())?;
    let source_authority = SpeciesConceptSourceAuthorityRef::new(
        SpeciesConceptSourceKindId::new("sel10e1b-general-lineage-model")?,
        1,
        SpeciesConceptSourceAuthorityDigest::new(*model.model_digest().as_bytes()),
        source.qualification_authority.clone(),
        SpeciesConceptSourceValidityDomainDigest::new(
            *source.validity_domain.canonical_digest()?.as_bytes(),
        ),
        general_lineage_descriptor_adapter_rule_v1(),
    )?;

    let capabilities = vec![open_capability(
        &identity,
        "current-species-status",
        b"general-lineage current species-status capability v1",
    )?];
    let evidence_schema = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("general-lineage-evidence-schema")?,
        1,
        [
            open_schema_term(
                &identity,
                "longitudinal-lineage-separation-core",
                b"longitudinal lineage separation/persistence is a core evidence channel, not automatic species truth",
            )?,
            open_schema_term(
                &identity,
                "preregistered-multichannel-evidence",
                b"eligible evidence channels and roles are frozen before outcomes",
            )?,
            open_schema_term(
                &identity,
                "independent-dependency-group-corroboration",
                b"support thresholds count distinct preregistered evidence-dependency groups rather than raw metrics",
            )?,
            open_schema_term(
                &identity,
                "explicit-conflict-and-missingness",
                b"contradiction, non-support, unavailable, and outside-channel-domain states remain distinct",
            )?,
        ],
    )?;
    let domain_schema = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("general-lineage-domain-schema")?,
        1,
        [
            open_schema_term(
                &identity,
                "qualified-target-validity-domain",
                b"target applicability requires explicit qualified validity-domain evidence",
            )?,
            open_schema_term(
                &identity,
                "sexual-or-asexual-as-declared",
                b"sexual and asexual targets may be in-domain only as explicitly declared",
            )?,
            open_schema_term(
                &identity,
                "reproductive-isolation-not-universal",
                b"reproductive isolation may corroborate lineage separation but is not universally required",
            )?,
            open_schema_term(
                &identity,
                "monophyly-not-universal",
                b"monophyly is not a universal defining requirement",
            )?,
        ],
    )?;

    Ok(SpeciesConceptFamilyDescriptor::declare(
        identity,
        model_id,
        source_authority,
        capabilities,
        evidence_schema,
        domain_schema,
        open_species_concept_descriptor_rule_v1(),
    )?)
}

fn open_capability(
    identity: &OpenSpeciesConceptIdentity,
    id: &str,
    semantic_bytes: &[u8],
) -> Result<SpeciesConceptCapabilityRef, GeneralLineageModelError> {
    let mut digest = Sha256::new();
    digest.update(CAPABILITY_DOMAIN);
    put_open_identity(&mut digest, identity);
    put_text(&mut digest, id);
    put_u64(&mut digest, semantic_bytes.len() as u64);
    digest.update(semantic_bytes);
    Ok(SpeciesConceptCapabilityRef::new(
        SpeciesConceptCapabilityId::new(id)?,
        1,
        SpeciesConceptCapabilityContentDigest::new(digest.finalize().into()),
    )?)
}

fn open_schema_term(
    identity: &OpenSpeciesConceptIdentity,
    id: &str,
    semantic_bytes: &[u8],
) -> Result<SpeciesConceptSchemaTerm, GeneralLineageModelError> {
    let mut digest = Sha256::new();
    digest.update(SCHEMA_TERM_DOMAIN);
    put_open_identity(&mut digest, identity);
    put_text(&mut digest, id);
    put_u64(&mut digest, semantic_bytes.len() as u64);
    digest.update(semantic_bytes);
    Ok(SpeciesConceptSchemaTerm::new(
        SpeciesConceptSchemaTermId::new(id)?,
        1,
        SpeciesConceptSchemaTermDigest::new(digest.finalize().into()),
    )?)
}

fn put_open_identity(digest: &mut Sha256, identity: &OpenSpeciesConceptIdentity) {
    put_text(digest, identity.family_id.as_str());
    put_u32(digest, identity.family_version);
    digest.update(identity.content_digest.as_bytes());
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), GeneralLineageModelError> {
    if authority.revision == 0 {
        return Err(GeneralLineageModelError::ZeroRevision(field));
    }
    Ok(())
}

fn validate_id(field: &'static str, value: &str) -> Result<(), GeneralLineageModelError> {
    if value.is_empty()
        || value.len() > 160
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(GeneralLineageModelError::InvalidIdentifier {
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
pub enum GeneralLineageModelError {
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    ZeroRevision(&'static str),
    UnsupportedModelVersion(u32),
    ModelContentMismatch,
    ModelRuleMismatch,
    ModelReplayMismatch,
    DescriptorReplayMismatch,
    Descriptor(DescriptorError),
}

impl From<DescriptorError> for GeneralLineageModelError {
    fn from(value: DescriptorError) -> Self {
        Self::Descriptor(value)
    }
}

impl fmt::Display for GeneralLineageModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field, value } => {
                write!(f, "invalid {field} value {value:?}")
            }
            Self::ZeroRevision(field) => write!(f, "{field} must be nonzero"),
            Self::UnsupportedModelVersion(version) => {
                write!(f, "unsupported general-lineage model version {version}")
            }
            Self::ModelContentMismatch => {
                write!(f, "general-lineage model content does not match the V1 family semantics")
            }
            Self::ModelRuleMismatch => {
                write!(f, "general-lineage model does not bind the built-in V1 rule")
            }
            Self::ModelReplayMismatch => {
                write!(f, "persisted general-lineage model does not replay from current authorities")
            }
            Self::DescriptorReplayMismatch => write!(
                f,
                "persisted open descriptor does not replay from the current general-lineage model"
            ),
            Self::Descriptor(error) => write!(f, "open descriptor error: {error}"),
        }
    }
}

impl Error for GeneralLineageModelError {}
