use crate::{
    GeneralLineageClassificationDesign, GeneralLineageClassificationDesignDigest,
    GeneralLineageEvidenceChannelDeclaration, GeneralLineageEvidenceChannelId,
    ValidatedGeneralLineageClassificationDesign,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, PopulationGeneration,
};

pub const GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION: u32 = 1;
const DESIGN_DOMAIN: &[u8] =
    b"symtropy:species-concept:general-lineage:historical-evidence-design:v1\0";
const RULE_DOMAIN: &[u8] =
    b"symtropy:species-concept:general-lineage:historical-evidence-design-rule:v1\0";
const RULE_SPEC: &[u8] = b"general-lineage historical evidence design v1: bind one exact current general-lineage classification design and its exact SEL-10A lineage-history subject; preregister a genuine multi-generation candidate transition interval with at least one lineage-history generation before and after; historical channels inherit current channel ID, kind, role, dependency group, dependency-group qualification, protocol and applicability semantics exactly and add only a temporal-projection protocol; every current channel has exactly one historical projection; minimum temporal support-group threshold is preregistered and at least two; common-source, historical-context, and later-counter-history completeness protocols are frozen before outcomes; no transition verdict, no exact event instant, and no universal taxonomy claim";

macro_rules! id_type {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, GeneralLineageHistoricalDesignError> {
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

id_type!(
    GeneralLineageHistoricalEvidenceDesignId,
    "GeneralLineageHistoricalEvidenceDesignId"
);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceDesignDigest([u8; 32]);

impl GeneralLineageHistoricalEvidenceDesignDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for GeneralLineageHistoricalEvidenceDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GeneralLineageHistoricalEvidenceDesignDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for GeneralLineageHistoricalEvidenceDesignDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageHistoricalMissingPolicy {
    FailClosed,
    ReportIncompleteEvidence,
}

impl GeneralLineageHistoricalMissingPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::ReportIncompleteEvidence => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalChannelProjectionInput {
    pub channel_id: GeneralLineageEvidenceChannelId,
    pub temporal_projection_protocol: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalChannelDeclaration {
    pub current_channel: GeneralLineageEvidenceChannelDeclaration,
    pub temporal_projection_protocol: AnalysisAuthorityRef,
}

impl GeneralLineageHistoricalChannelDeclaration {
    fn validate_local(&self) -> Result<(), GeneralLineageHistoricalDesignError> {
        validate_authority(
            &self.temporal_projection_protocol,
            "temporal_projection_protocol_revision",
        )?;
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.current_channel.channel_id.as_str());
        digest.update([channel_kind_tag(self.current_channel.kind)]);
        digest.update([channel_role_tag(self.current_channel.role)]);
        put_text(digest, self.current_channel.dependency_group_id.as_str());
        put_authority(
            digest,
            &self.current_channel.dependency_group_qualification_authority,
        );
        put_authority(digest, &self.current_channel.protocol_authority);
        put_authority(digest, &self.current_channel.applicability_authority);
        put_authority(digest, &self.temporal_projection_protocol);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceDesign {
    design_version: u32,
    pub historical_design_id: GeneralLineageHistoricalEvidenceDesignId,
    pub current_classification_design: GeneralLineageClassificationDesign,
    pub current_classification_design_digest: GeneralLineageClassificationDesignDigest,
    pub history_start_generation: PopulationGeneration,
    pub history_end_generation: PopulationGeneration,
    pub candidate_interval_start: PopulationGeneration,
    pub candidate_interval_end: PopulationGeneration,
    pub candidate_interval_generation_count: u64,
    pub channels: Vec<GeneralLineageHistoricalChannelDeclaration>,
    pub minimum_independent_temporal_support_groups: u32,
    pub common_source_protocol: AnalysisAuthorityRef,
    pub historical_context_protocol: AnalysisAuthorityRef,
    pub counter_history_completeness_protocol: AnalysisAuthorityRef,
    pub missing_policy: GeneralLineageHistoricalMissingPolicy,
    pub design_rule_authority: AnalysisAuthorityRef,
}

impl GeneralLineageHistoricalEvidenceDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        historical_design_id: GeneralLineageHistoricalEvidenceDesignId,
        classification_design: &ValidatedGeneralLineageClassificationDesign<'_>,
        candidate_interval_start: PopulationGeneration,
        candidate_interval_end: PopulationGeneration,
        projections: impl IntoIterator<Item = HistoricalChannelProjectionInput>,
        minimum_independent_temporal_support_groups: u32,
        common_source_protocol: AnalysisAuthorityRef,
        historical_context_protocol: AnalysisAuthorityRef,
        counter_history_completeness_protocol: AnalysisAuthorityRef,
        missing_policy: GeneralLineageHistoricalMissingPolicy,
    ) -> Result<Self, GeneralLineageHistoricalDesignError> {
        validate_authority(&common_source_protocol, "common_source_protocol_revision")?;
        validate_authority(
            &historical_context_protocol,
            "historical_context_protocol_revision",
        )?;
        validate_authority(
            &counter_history_completeness_protocol,
            "counter_history_completeness_protocol_revision",
        )?;
        if minimum_independent_temporal_support_groups < 2 {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdTooLow);
        }

        let current = classification_design.design();
        let history = &current.lineage_history_design;
        validate_candidate_interval(
            history.start_generation,
            history.end_generation,
            candidate_interval_start,
            candidate_interval_end,
        )?;

        let mut by_channel = BTreeMap::new();
        for projection in projections {
            validate_authority(
                &projection.temporal_projection_protocol,
                "temporal_projection_protocol_revision",
            )?;
            if by_channel
                .insert(projection.channel_id.clone(), projection)
                .is_some()
            {
                return Err(GeneralLineageHistoricalDesignError::DuplicateChannelProjection);
            }
        }

        let mut channels = Vec::with_capacity(current.channels.len());
        for current_channel in &current.channels {
            let projection = by_channel
                .remove(&current_channel.channel_id)
                .ok_or(GeneralLineageHistoricalDesignError::MissingChannelProjection)?;
            channels.push(GeneralLineageHistoricalChannelDeclaration {
                current_channel: current_channel.clone(),
                temporal_projection_protocol: projection.temporal_projection_protocol,
            });
        }
        if !by_channel.is_empty() {
            return Err(GeneralLineageHistoricalDesignError::UnexpectedChannelProjection);
        }

        let distinct_groups = channels
            .iter()
            .map(|channel| channel.current_channel.dependency_group_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if usize::try_from(minimum_independent_temporal_support_groups)
            .map_err(|_| GeneralLineageHistoricalDesignError::ArithmeticOverflow)?
            > distinct_groups
        {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdExceedsGroups);
        }

        let design = Self {
            design_version: GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION,
            historical_design_id,
            current_classification_design: current.clone(),
            current_classification_design_digest: classification_design.design_digest(),
            history_start_generation: history.start_generation,
            history_end_generation: history.end_generation,
            candidate_interval_start,
            candidate_interval_end,
            candidate_interval_generation_count: interval_count(
                candidate_interval_start,
                candidate_interval_end,
            )?,
            channels,
            minimum_independent_temporal_support_groups,
            common_source_protocol,
            historical_context_protocol,
            counter_history_completeness_protocol,
            missing_policy,
            design_rule_authority: general_lineage_historical_evidence_design_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageHistoricalEvidenceDesignDigest, GeneralLineageHistoricalDesignError>
    {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.historical_design_id.as_str());
        digest.update(self.current_classification_design_digest.as_bytes());
        put_u64(&mut digest, self.history_start_generation.0);
        put_u64(&mut digest, self.history_end_generation.0);
        put_u64(&mut digest, self.candidate_interval_start.0);
        put_u64(&mut digest, self.candidate_interval_end.0);
        put_u64(&mut digest, self.candidate_interval_generation_count);
        put_u64(&mut digest, self.channels.len() as u64);
        for channel in &self.channels {
            channel.put(&mut digest);
        }
        put_u32(
            &mut digest,
            self.minimum_independent_temporal_support_groups,
        );
        put_authority(&mut digest, &self.common_source_protocol);
        put_authority(&mut digest, &self.historical_context_protocol);
        put_authority(&mut digest, &self.counter_history_completeness_protocol);
        digest.update([self.missing_policy.tag()]);
        put_authority(&mut digest, &self.design_rule_authority);
        Ok(GeneralLineageHistoricalEvidenceDesignDigest(
            digest.finalize().into(),
        ))
    }

    pub fn channel(
        &self,
        channel_id: &GeneralLineageEvidenceChannelId,
    ) -> Option<&GeneralLineageHistoricalChannelDeclaration> {
        self.channels
            .iter()
            .find(|channel| &channel.current_channel.channel_id == channel_id)
    }

    fn validate_local(&self) -> Result<(), GeneralLineageHistoricalDesignError> {
        if self.design_version != GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION {
            return Err(GeneralLineageHistoricalDesignError::UnsupportedDesignVersion(
                self.design_version,
            ));
        }
        if self.current_classification_design.canonical_digest()?
            != self.current_classification_design_digest
        {
            return Err(GeneralLineageHistoricalDesignError::ClassificationDesignDigestMismatch);
        }
        let history = &self.current_classification_design.lineage_history_design;
        if self.history_start_generation != history.start_generation
            || self.history_end_generation != history.end_generation
        {
            return Err(GeneralLineageHistoricalDesignError::HistoryBoundsMismatch);
        }
        validate_candidate_interval(
            self.history_start_generation,
            self.history_end_generation,
            self.candidate_interval_start,
            self.candidate_interval_end,
        )?;
        if interval_count(
            self.candidate_interval_start,
            self.candidate_interval_end,
        )? != self.candidate_interval_generation_count
        {
            return Err(GeneralLineageHistoricalDesignError::CandidateIntervalCountMismatch);
        }
        if self.channels.len() != self.current_classification_design.channels.len() {
            return Err(GeneralLineageHistoricalDesignError::IncompleteChannelProjection);
        }
        for (historical, current) in self
            .channels
            .iter()
            .zip(&self.current_classification_design.channels)
        {
            historical.validate_local()?;
            if historical.current_channel != *current {
                return Err(GeneralLineageHistoricalDesignError::CurrentChannelBindingMismatch);
            }
        }
        for pair in self.channels.windows(2) {
            if pair[0].current_channel.channel_id >= pair[1].current_channel.channel_id {
                return Err(GeneralLineageHistoricalDesignError::NonCanonicalChannelOrder);
            }
        }
        if self.minimum_independent_temporal_support_groups < 2 {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdTooLow);
        }
        let distinct_groups = self
            .channels
            .iter()
            .map(|channel| channel.current_channel.dependency_group_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if usize::try_from(self.minimum_independent_temporal_support_groups)
            .map_err(|_| GeneralLineageHistoricalDesignError::ArithmeticOverflow)?
            > distinct_groups
        {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdExceedsGroups);
        }
        validate_authority(&self.common_source_protocol, "common_source_protocol_revision")?;
        validate_authority(
            &self.historical_context_protocol,
            "historical_context_protocol_revision",
        )?;
        validate_authority(
            &self.counter_history_completeness_protocol,
            "counter_history_completeness_protocol_revision",
        )?;
        if self.design_rule_authority != general_lineage_historical_evidence_design_rule_v1() {
            return Err(GeneralLineageHistoricalDesignError::DesignRuleMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated historical design should gate F1A temporal evidence materialization"]
pub struct ValidatedGeneralLineageHistoricalEvidenceDesign<'a> {
    design: &'a GeneralLineageHistoricalEvidenceDesign,
    design_digest: GeneralLineageHistoricalEvidenceDesignDigest,
}

impl<'a> ValidatedGeneralLineageHistoricalEvidenceDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a GeneralLineageHistoricalEvidenceDesign,
        classification_design: &ValidatedGeneralLineageClassificationDesign<'_>,
        candidate_interval_start: PopulationGeneration,
        candidate_interval_end: PopulationGeneration,
        projections: impl IntoIterator<Item = HistoricalChannelProjectionInput>,
        minimum_independent_temporal_support_groups: u32,
        common_source_protocol: AnalysisAuthorityRef,
        historical_context_protocol: AnalysisAuthorityRef,
        counter_history_completeness_protocol: AnalysisAuthorityRef,
        missing_policy: GeneralLineageHistoricalMissingPolicy,
    ) -> Result<Self, GeneralLineageHistoricalDesignError> {
        design.validate_local()?;
        let recomputed = GeneralLineageHistoricalEvidenceDesign::declare(
            design.historical_design_id.clone(),
            classification_design,
            candidate_interval_start,
            candidate_interval_end,
            projections,
            minimum_independent_temporal_support_groups,
            common_source_protocol,
            historical_context_protocol,
            counter_history_completeness_protocol,
            missing_policy,
        )?;
        if recomputed != *design {
            return Err(GeneralLineageHistoricalDesignError::DesignReplayMismatch);
        }
        Ok(Self {
            design,
            design_digest: design.canonical_digest()?,
        })
    }

    pub fn design(&self) -> &'a GeneralLineageHistoricalEvidenceDesign {
        self.design
    }

    pub fn design_digest(&self) -> GeneralLineageHistoricalEvidenceDesignDigest {
        self.design_digest
    }
}

pub fn general_lineage_historical_evidence_design_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    put_u64(&mut digest, RULE_SPEC.len() as u64);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("general-lineage-historical-evidence-design-v1")
            .expect("static historical design rule method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

fn validate_candidate_interval(
    history_start: PopulationGeneration,
    history_end: PopulationGeneration,
    interval_start: PopulationGeneration,
    interval_end: PopulationGeneration,
) -> Result<(), GeneralLineageHistoricalDesignError> {
    let count = interval_count(interval_start, interval_end)?;
    if count < 2 {
        return Err(GeneralLineageHistoricalDesignError::ExactEventIntervalForbidden);
    }
    if interval_start.0 <= history_start.0 || interval_end.0 >= history_end.0 {
        return Err(GeneralLineageHistoricalDesignError::MissingBeforeOrAfterCoverage);
    }
    Ok(())
}

fn interval_count(
    start: PopulationGeneration,
    end: PopulationGeneration,
) -> Result<u64, GeneralLineageHistoricalDesignError> {
    end.0
        .checked_sub(start.0)
        .and_then(|delta| delta.checked_add(1))
        .ok_or(GeneralLineageHistoricalDesignError::InvalidCandidateInterval)
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), GeneralLineageHistoricalDesignError> {
    if authority.revision == 0 {
        return Err(GeneralLineageHistoricalDesignError::ZeroAuthorityRevision(field));
    }
    Ok(())
}

fn validate_id(
    field: &'static str,
    value: &str,
) -> Result<(), GeneralLineageHistoricalDesignError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value || value.len() > 160 {
        return Err(GeneralLineageHistoricalDesignError::InvalidIdentifier(field));
    }
    Ok(())
}

fn channel_kind_tag(kind: crate::GeneralLineageEvidenceChannelKind) -> u8 {
    use crate::GeneralLineageEvidenceChannelKind::*;
    match kind {
        LongitudinalLineageSeparation => 0,
        DemographicIndependence => 1,
        PopulationConnectivity => 2,
        GeneticDiagnosability => 3,
        PhenotypicDiagnosability => 4,
        EcologicalDifferentiation => 5,
        ReproductiveIsolation => 6,
        GenealogicalConcordance => 7,
    }
}

fn channel_role_tag(role: crate::GeneralLineageEvidenceChannelRole) -> u8 {
    use crate::GeneralLineageEvidenceChannelRole::*;
    match role {
        CoreRequired => 0,
        Required => 1,
        Corroborating => 2,
        Optional => 3,
    }
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
    digest.update(value.to_le_bytes());
}

fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_le_bytes());
}

fn fmt_hex(bytes: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum GeneralLineageHistoricalDesignError {
    InvalidIdentifier(&'static str),
    UnsupportedDesignVersion(u32),
    ZeroAuthorityRevision(&'static str),
    InvalidCandidateInterval,
    ExactEventIntervalForbidden,
    MissingBeforeOrAfterCoverage,
    CandidateIntervalCountMismatch,
    DuplicateChannelProjection,
    MissingChannelProjection,
    UnexpectedChannelProjection,
    IncompleteChannelProjection,
    CurrentChannelBindingMismatch,
    NonCanonicalChannelOrder,
    ClassificationDesignDigestMismatch,
    HistoryBoundsMismatch,
    TemporalSupportThresholdTooLow,
    TemporalSupportThresholdExceedsGroups,
    DesignRuleMismatch,
    DesignReplayMismatch,
    ArithmeticOverflow,
    CurrentClassification(crate::GeneralLineageDesignError),
}

impl From<crate::GeneralLineageDesignError> for GeneralLineageHistoricalDesignError {
    fn from(value: crate::GeneralLineageDesignError) -> Self {
        Self::CurrentClassification(value)
    }
}

impl fmt::Display for GeneralLineageHistoricalDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(f, "invalid historical identifier in {field}"),
            Self::UnsupportedDesignVersion(version) => {
                write!(f, "unsupported general-lineage historical design version {version}")
            }
            Self::ZeroAuthorityRevision(field) => write!(f, "{field} must be nonzero"),
            Self::InvalidCandidateInterval => write!(f, "invalid candidate transition interval"),
            Self::ExactEventIntervalForbidden => write!(
                f,
                "V1 candidate transition interval must span at least two generations"
            ),
            Self::MissingBeforeOrAfterCoverage => write!(
                f,
                "candidate interval must leave at least one SEL-10A generation before and after"
            ),
            Self::CandidateIntervalCountMismatch => {
                write!(f, "candidate interval generation count does not recompute")
            }
            Self::DuplicateChannelProjection => write!(f, "duplicate historical channel projection"),
            Self::MissingChannelProjection => write!(f, "missing historical projection for a current channel"),
            Self::UnexpectedChannelProjection => write!(f, "historical projection names an undeclared current channel"),
            Self::IncompleteChannelProjection => write!(f, "historical design does not retain every current channel exactly once"),
            Self::CurrentChannelBindingMismatch => write!(f, "historical channel semantics do not exactly match the frozen current channel"),
            Self::NonCanonicalChannelOrder => write!(f, "historical channels are not in canonical current-channel order"),
            Self::ClassificationDesignDigestMismatch => write!(f, "embedded current classification design does not match its digest"),
            Self::HistoryBoundsMismatch => write!(f, "historical bounds do not match the frozen SEL-10A design"),
            Self::TemporalSupportThresholdTooLow => write!(f, "historical support-group threshold must be at least two"),
            Self::TemporalSupportThresholdExceedsGroups => write!(f, "historical support-group threshold exceeds preregistered dependency groups"),
            Self::DesignRuleMismatch => write!(f, "historical design rule authority mismatch"),
            Self::DesignReplayMismatch => write!(f, "historical design does not replay from current classification authority and temporal policy"),
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow while validating historical design"),
            Self::CurrentClassification(error) => write!(f, "current general-lineage design error: {error}"),
        }
    }
}

impl Error for GeneralLineageHistoricalDesignError {}
