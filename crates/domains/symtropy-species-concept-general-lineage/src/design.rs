use crate::model::{
    GeneralLineageModelError, GeneralLineageSpeciesModel, GeneralLineageSpeciesModelDigest,
    ValidatedGeneralLineageSpeciesModel,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId,
    LineageDivergenceHistoryDesign, LineageDivergenceHistoryDesignDigest,
    ValidatedLineageDivergenceHistoryDesign,
};

pub const GENERAL_LINEAGE_CLASSIFICATION_DESIGN_VERSION: u32 = 1;
const DESIGN_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:classification-design:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:classification-rule:v1\0";
const RULE_SPEC: &[u8] = b"general-lineage current classification design v1: bind exact current general-lineage model plus exact SEL-10A lineage-history design before outcomes; require exactly one core longitudinal-lineage-separation channel; every channel has stable ID, kind, role, protocol, applicability authority, and evidence-dependency group; positive status requires the core channel plus at least two distinct supporting dependency groups; raw channel count is never a support threshold; reproductive isolation is optional unless explicitly preregistered as required; missing and outside-channel-domain remain distinct from contradiction; no historical transition or universal taxonomy claim";

macro_rules! id_type {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, GeneralLineageDesignError> {
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

id_type!(GeneralLineageClassificationId, "GeneralLineageClassificationId");
id_type!(GeneralLineageEvidenceChannelId, "GeneralLineageEvidenceChannelId");
id_type!(GeneralLineageEvidenceDependencyGroupId, "GeneralLineageEvidenceDependencyGroupId");

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneralLineageClassificationDesignDigest([u8; 32]);

impl GeneralLineageClassificationDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for GeneralLineageClassificationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GeneralLineageClassificationDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for GeneralLineageClassificationDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GeneralLineageEvidenceChannelKind {
    LongitudinalLineageSeparation,
    DemographicIndependence,
    PopulationConnectivity,
    GeneticDiagnosability,
    PhenotypicDiagnosability,
    EcologicalDifferentiation,
    ReproductiveIsolation,
    GenealogicalConcordance,
}

impl GeneralLineageEvidenceChannelKind {
    fn tag(self) -> u8 {
        match self {
            Self::LongitudinalLineageSeparation => 0,
            Self::DemographicIndependence => 1,
            Self::PopulationConnectivity => 2,
            Self::GeneticDiagnosability => 3,
            Self::PhenotypicDiagnosability => 4,
            Self::EcologicalDifferentiation => 5,
            Self::ReproductiveIsolation => 6,
            Self::GenealogicalConcordance => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GeneralLineageEvidenceChannelRole {
    CoreRequired,
    Required,
    Corroborating,
    Optional,
}

impl GeneralLineageEvidenceChannelRole {
    fn tag(self) -> u8 {
        match self {
            Self::CoreRequired => 0,
            Self::Required => 1,
            Self::Corroborating => 2,
            Self::Optional => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageMissingEvidencePolicy {
    FailClosed,
    ReportInsufficientIndependentEvidence,
}

impl GeneralLineageMissingEvidencePolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::ReportInsufficientIndependentEvidence => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageEvidenceChannelDeclaration {
    pub channel_id: GeneralLineageEvidenceChannelId,
    pub kind: GeneralLineageEvidenceChannelKind,
    pub role: GeneralLineageEvidenceChannelRole,
    pub dependency_group_id: GeneralLineageEvidenceDependencyGroupId,
    pub protocol_authority: AnalysisAuthorityRef,
    pub applicability_authority: AnalysisAuthorityRef,
}

impl GeneralLineageEvidenceChannelDeclaration {
    pub fn new(
        channel_id: GeneralLineageEvidenceChannelId,
        kind: GeneralLineageEvidenceChannelKind,
        role: GeneralLineageEvidenceChannelRole,
        dependency_group_id: GeneralLineageEvidenceDependencyGroupId,
        protocol_authority: AnalysisAuthorityRef,
        applicability_authority: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageDesignError> {
        validate_authority(&protocol_authority, "channel_protocol_revision")?;
        validate_authority(&applicability_authority, "channel_applicability_revision")?;
        Ok(Self {
            channel_id,
            kind,
            role,
            dependency_group_id,
            protocol_authority,
            applicability_authority,
        })
    }

    fn validate_local(&self) -> Result<(), GeneralLineageDesignError> {
        validate_authority(&self.protocol_authority, "channel_protocol_revision")?;
        validate_authority(&self.applicability_authority, "channel_applicability_revision")?;
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.channel_id.as_str());
        digest.update([self.kind.tag(), self.role.tag()]);
        put_text(digest, self.dependency_group_id.as_str());
        put_authority(digest, &self.protocol_authority);
        put_authority(digest, &self.applicability_authority);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageClassificationDesign {
    design_version: u32,
    pub classification_id: GeneralLineageClassificationId,
    pub lineage_history_design: LineageDivergenceHistoryDesign,
    pub lineage_history_design_digest: LineageDivergenceHistoryDesignDigest,
    pub model: GeneralLineageSpeciesModel,
    pub model_digest: GeneralLineageSpeciesModelDigest,
    pub model_applicability_protocol: AnalysisAuthorityRef,
    pub channels: Vec<GeneralLineageEvidenceChannelDeclaration>,
    pub minimum_independent_support_groups: u32,
    pub missing_policy: GeneralLineageMissingEvidencePolicy,
    pub classification_rule_authority: AnalysisAuthorityRef,
}

impl GeneralLineageClassificationDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        classification_id: GeneralLineageClassificationId,
        history_design: &ValidatedLineageDivergenceHistoryDesign<'_>,
        model: &ValidatedGeneralLineageSpeciesModel<'_>,
        model_applicability_protocol: AnalysisAuthorityRef,
        channels: impl IntoIterator<Item = GeneralLineageEvidenceChannelDeclaration>,
        minimum_independent_support_groups: u32,
        missing_policy: GeneralLineageMissingEvidencePolicy,
    ) -> Result<Self, GeneralLineageDesignError> {
        validate_authority(
            &model_applicability_protocol,
            "model_applicability_protocol_revision",
        )?;
        if minimum_independent_support_groups < 2 {
            return Err(GeneralLineageDesignError::SupportThresholdTooLow);
        }
        let mut channels: Vec<_> = channels.into_iter().collect();
        channels.sort_by(|left, right| left.channel_id.cmp(&right.channel_id));
        validate_channels(&channels, minimum_independent_support_groups)?;
        let design = Self {
            design_version: GENERAL_LINEAGE_CLASSIFICATION_DESIGN_VERSION,
            classification_id,
            lineage_history_design: history_design.design().clone(),
            lineage_history_design_digest: history_design.design_digest(),
            model: model.model().clone(),
            model_digest: model.model_digest(),
            model_applicability_protocol,
            channels,
            minimum_independent_support_groups,
            missing_policy,
            classification_rule_authority: general_lineage_classification_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageClassificationDesignDigest, GeneralLineageDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.classification_id.as_str());
        digest.update(self.lineage_history_design_digest.as_bytes());
        digest.update(self.model_digest.as_bytes());
        put_authority(&mut digest, &self.model_applicability_protocol);
        put_u64(&mut digest, self.channels.len() as u64);
        for channel in &self.channels {
            channel.put(&mut digest);
        }
        put_u32(&mut digest, self.minimum_independent_support_groups);
        digest.update([self.missing_policy.tag()]);
        put_authority(&mut digest, &self.classification_rule_authority);
        Ok(GeneralLineageClassificationDesignDigest(
            digest.finalize().into(),
        ))
    }

    pub fn core_channel(&self) -> &GeneralLineageEvidenceChannelDeclaration {
        self.channels
            .iter()
            .find(|channel| channel.role == GeneralLineageEvidenceChannelRole::CoreRequired)
            .expect("validated design contains exactly one core channel")
    }

    fn validate_local(&self) -> Result<(), GeneralLineageDesignError> {
        if self.design_version != GENERAL_LINEAGE_CLASSIFICATION_DESIGN_VERSION {
            return Err(GeneralLineageDesignError::UnsupportedDesignVersion(
                self.design_version,
            ));
        }
        if self.lineage_history_design.canonical_digest()? != self.lineage_history_design_digest {
            return Err(GeneralLineageDesignError::LineageDesignDigestMismatch);
        }
        if self.model.canonical_digest()? != self.model_digest {
            return Err(GeneralLineageDesignError::ModelDigestMismatch);
        }
        validate_authority(
            &self.model_applicability_protocol,
            "model_applicability_protocol_revision",
        )?;
        if self.minimum_independent_support_groups < 2 {
            return Err(GeneralLineageDesignError::SupportThresholdTooLow);
        }
        validate_channels(&self.channels, self.minimum_independent_support_groups)?;
        if self.classification_rule_authority != general_lineage_classification_rule_v1() {
            return Err(GeneralLineageDesignError::ClassificationRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated classification design should gate general-lineage status evidence"]
pub struct ValidatedGeneralLineageClassificationDesign<'a> {
    design: &'a GeneralLineageClassificationDesign,
    design_digest: GeneralLineageClassificationDesignDigest,
}

impl<'a> ValidatedGeneralLineageClassificationDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a GeneralLineageClassificationDesign,
        history_design: &ValidatedLineageDivergenceHistoryDesign<'_>,
        model: &ValidatedGeneralLineageSpeciesModel<'_>,
        model_applicability_protocol: AnalysisAuthorityRef,
        channels: impl IntoIterator<Item = GeneralLineageEvidenceChannelDeclaration>,
        minimum_independent_support_groups: u32,
        missing_policy: GeneralLineageMissingEvidencePolicy,
    ) -> Result<Self, GeneralLineageDesignError> {
        design.validate_local()?;
        let recomputed = GeneralLineageClassificationDesign::declare(
            design.classification_id.clone(),
            history_design,
            model,
            model_applicability_protocol,
            channels,
            minimum_independent_support_groups,
            missing_policy,
        )?;
        if recomputed != *design {
            return Err(GeneralLineageDesignError::DesignReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a GeneralLineageClassificationDesign {
        self.design
    }

    pub fn design_digest(&self) -> GeneralLineageClassificationDesignDigest {
        self.design_digest
    }
}

pub fn general_lineage_classification_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("general-lineage-current-species-classification-v1")
            .expect("static general-lineage classification rule ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

fn validate_channels(
    channels: &[GeneralLineageEvidenceChannelDeclaration],
    minimum_independent_support_groups: u32,
) -> Result<(), GeneralLineageDesignError> {
    if channels.len() < 2 {
        return Err(GeneralLineageDesignError::TooFewChannels);
    }
    let mut previous: Option<&GeneralLineageEvidenceChannelId> = None;
    let mut core_count = 0usize;
    let mut groups = BTreeSet::new();
    for channel in channels {
        channel.validate_local()?;
        if let Some(previous) = previous {
            if previous >= &channel.channel_id {
                if previous == &channel.channel_id {
                    return Err(GeneralLineageDesignError::DuplicateChannelId(
                        channel.channel_id.clone(),
                    ));
                }
                return Err(GeneralLineageDesignError::NonCanonicalChannelOrder);
            }
        }
        previous = Some(&channel.channel_id);
        groups.insert(channel.dependency_group_id.clone());
        if channel.role == GeneralLineageEvidenceChannelRole::CoreRequired {
            core_count += 1;
            if channel.kind != GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation {
                return Err(GeneralLineageDesignError::InvalidCoreChannelKind);
            }
        } else if channel.kind == GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation {
            return Err(GeneralLineageDesignError::ReservedCoreChannelKind);
        }
    }
    if core_count != 1 {
        return Err(GeneralLineageDesignError::CoreChannelCount(core_count));
    }
    if groups.len() < minimum_independent_support_groups as usize {
        return Err(GeneralLineageDesignError::InsufficientDeclaredDependencyGroups {
            declared: groups.len(),
            required: minimum_independent_support_groups,
        });
    }
    Ok(())
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), GeneralLineageDesignError> {
    if authority.revision == 0 {
        return Err(GeneralLineageDesignError::ZeroRevision(field));
    }
    Ok(())
}

fn validate_id(field: &'static str, value: &str) -> Result<(), GeneralLineageDesignError> {
    if value.is_empty()
        || value.len() > 160
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(GeneralLineageDesignError::InvalidIdentifier {
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
pub enum GeneralLineageDesignError {
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    ZeroRevision(&'static str),
    UnsupportedDesignVersion(u32),
    SupportThresholdTooLow,
    TooFewChannels,
    DuplicateChannelId(GeneralLineageEvidenceChannelId),
    NonCanonicalChannelOrder,
    InvalidCoreChannelKind,
    ReservedCoreChannelKind,
    CoreChannelCount(usize),
    InsufficientDeclaredDependencyGroups {
        declared: usize,
        required: u32,
    },
    LineageDesignDigestMismatch,
    ModelDigestMismatch,
    ClassificationRuleMismatch,
    DesignReplayMismatch,
    LineageDesign(symtropy_evolution_core::LineageDivergenceDesignError),
    Model(GeneralLineageModelError),
}

impl From<symtropy_evolution_core::LineageDivergenceDesignError> for GeneralLineageDesignError {
    fn from(value: symtropy_evolution_core::LineageDivergenceDesignError) -> Self {
        Self::LineageDesign(value)
    }
}

impl From<GeneralLineageModelError> for GeneralLineageDesignError {
    fn from(value: GeneralLineageModelError) -> Self {
        Self::Model(value)
    }
}

impl fmt::Display for GeneralLineageDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field, value } => {
                write!(f, "invalid {field} value {value:?}")
            }
            Self::ZeroRevision(field) => write!(f, "{field} must be nonzero"),
            Self::UnsupportedDesignVersion(version) => {
                write!(f, "unsupported general-lineage classification design version {version}")
            }
            Self::SupportThresholdTooLow => write!(
                f,
                "minimum independent support groups must be at least two in V1"
            ),
            Self::TooFewChannels => write!(
                f,
                "general-lineage classification requires a core channel plus corroborating surface"
            ),
            Self::DuplicateChannelId(id) => {
                write!(f, "duplicate general-lineage channel {}", id.as_str())
            }
            Self::NonCanonicalChannelOrder => {
                write!(f, "general-lineage channel declarations are not canonically ordered")
            }
            Self::InvalidCoreChannelKind => write!(
                f,
                "CoreRequired channel must be LongitudinalLineageSeparation"
            ),
            Self::ReservedCoreChannelKind => write!(
                f,
                "LongitudinalLineageSeparation is reserved for the single core channel"
            ),
            Self::CoreChannelCount(count) => write!(
                f,
                "general-lineage design requires exactly one core channel, found {count}"
            ),
            Self::InsufficientDeclaredDependencyGroups { declared, required } => write!(
                f,
                "only {declared} dependency groups declared but threshold requires {required}"
            ),
            Self::LineageDesignDigestMismatch => write!(
                f,
                "embedded SEL-10A lineage-history design does not match its persisted digest"
            ),
            Self::ModelDigestMismatch => write!(
                f,
                "embedded general-lineage model does not match its persisted digest"
            ),
            Self::ClassificationRuleMismatch => write!(
                f,
                "classification design does not bind the built-in V1 rule"
            ),
            Self::DesignReplayMismatch => write!(
                f,
                "persisted general-lineage classification design does not replay from current inputs"
            ),
            Self::LineageDesign(error) => write!(f, "SEL-10A design error: {error}"),
            Self::Model(error) => write!(f, "general-lineage model error: {error}"),
        }
    }
}

impl Error for GeneralLineageDesignError {}
