// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Outcome-free preregistration for general-lineage historical evidence.

use crate::{
    GeneralLineageClassificationDesign, GeneralLineageClassificationDesignDigest,
    GeneralLineageDesignError, GeneralLineageEvidenceChannelDeclaration,
    GeneralLineageEvidenceChannelId, GeneralLineageEvidenceChannelKind,
    ValidatedGeneralLineageClassificationDesign,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, PopulationGeneration,
};

pub const GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION: u32 = 1;
const DESIGN_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:historical-evidence-design:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:historical-evidence-rule:v1\0";
const RULE_SPEC: &[u8] = b"general-lineage historical evidence design v1: exact current general-lineage classification design; exact SEL-10A ordered lineage/history design; genuine before/candidate/after temporal windows; candidate interval spans at least two generations and never represents an exact event instant; exactly one temporal projection per preregistered current evidence channel; preserve current channel roles, dependency groups and grouping qualifications; minimum temporal support-group threshold preregistered before historical outcomes; explicit missing-data, common-source and later-counter-history completeness protocols; no outcome-derived channel/window selection; no historical transition verdict, exact speciation instant, cross-model robustness, nomenclature, or universal taxonomy claim";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct GeneralLineageHistoricalEvidenceDesignId(String);

impl GeneralLineageHistoricalEvidenceDesignId {
    pub fn new(value: impl Into<String>) -> Result<Self, GeneralLineageHistoricalDesignError> {
        let value = value.into();
        validate_id("GeneralLineageHistoricalEvidenceDesignId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GeneralLineageHistoricalEvidenceDesignId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceDesignDigest([u8; 32]);

impl GeneralLineageHistoricalEvidenceDesignDigest {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HistoricalEvidenceWindow {
    PreInterval,
    CandidateInterval,
    PostInterval,
}

impl HistoricalEvidenceWindow {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::PreInterval => 0,
            Self::CandidateInterval => 1,
            Self::PostInterval => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalEvidenceWindowBounds {
    pub window: HistoricalEvidenceWindow,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub generation_count: u64,
}

impl HistoricalEvidenceWindowBounds {
    fn new(
        window: HistoricalEvidenceWindow,
        start_generation: PopulationGeneration,
        end_generation: PopulationGeneration,
    ) -> Result<Self, GeneralLineageHistoricalDesignError> {
        let generation_count = end_generation
            .0
            .checked_sub(start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(GeneralLineageHistoricalDesignError::InvalidTemporalWindow)?;
        if generation_count == 0 {
            return Err(GeneralLineageHistoricalDesignError::InvalidTemporalWindow);
        }
        Ok(Self {
            window,
            start_generation,
            end_generation,
            generation_count,
        })
    }

    pub(crate) fn put(&self, digest: &mut Sha256) {
        digest.update([self.window.tag()]);
        put_u64(digest, self.start_generation.0);
        put_u64(digest, self.end_generation.0);
        put_u64(digest, self.generation_count);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageHistoricalMissingPolicy {
    FailClosed,
    RetainUnavailable,
}

impl GeneralLineageHistoricalMissingPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::FailClosed => 0,
            Self::RetainUnavailable => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalChannelProjectionInput {
    pub channel_id: GeneralLineageEvidenceChannelId,
    pub temporal_projection_protocol: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalChannelProjection {
    pub declaration: GeneralLineageEvidenceChannelDeclaration,
    pub temporal_projection_protocol: AnalysisAuthorityRef,
}

impl HistoricalChannelProjection {
    pub(crate) fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.declaration.channel_id.as_str());
        put_text(digest, channel_kind_id(self.declaration.kind));
        put_text(digest, self.declaration.dependency_group.as_str());
        put_authority(digest, &self.declaration.dependency_group_qualification);
        put_authority(digest, &self.declaration.evidence_protocol);
        put_authority(digest, &self.declaration.applicability_authority);
        put_authority(digest, &self.temporal_projection_protocol);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceDesign {
    design_version: u32,
    pub design_id: GeneralLineageHistoricalEvidenceDesignId,
    pub classification_design: GeneralLineageClassificationDesign,
    pub classification_design_digest: GeneralLineageClassificationDesignDigest,
    pub history_start_generation: PopulationGeneration,
    pub history_end_generation: PopulationGeneration,
    pub candidate_start_generation: PopulationGeneration,
    pub candidate_end_generation: PopulationGeneration,
    pub pre_interval: HistoricalEvidenceWindowBounds,
    pub candidate_interval: HistoricalEvidenceWindowBounds,
    pub post_interval: HistoricalEvidenceWindowBounds,
    pub channel_projections: Vec<HistoricalChannelProjection>,
    pub minimum_temporal_support_dependency_groups: u32,
    pub missing_policy: GeneralLineageHistoricalMissingPolicy,
    pub common_source_protocol_authority: AnalysisAuthorityRef,
    pub post_interval_completeness_protocol_authority: AnalysisAuthorityRef,
    pub rule_authority: AnalysisAuthorityRef,
}

impl GeneralLineageHistoricalEvidenceDesign {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        design_id: GeneralLineageHistoricalEvidenceDesignId,
        classification_design: &ValidatedGeneralLineageClassificationDesign<'_>,
        candidate_start_generation: PopulationGeneration,
        candidate_end_generation: PopulationGeneration,
        channel_projection_inputs: impl IntoIterator<Item = HistoricalChannelProjectionInput>,
        minimum_temporal_support_dependency_groups: u32,
        missing_policy: GeneralLineageHistoricalMissingPolicy,
        common_source_protocol_authority: AnalysisAuthorityRef,
        post_interval_completeness_protocol_authority: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageHistoricalDesignError> {
        validate_authority(
            &common_source_protocol_authority,
            "common_source_protocol_revision",
        )?;
        validate_authority(
            &post_interval_completeness_protocol_authority,
            "post_interval_completeness_protocol_revision",
        )?;
        let classification = classification_design.design();
        let history = &classification.lineage_history_design;
        let history_start_generation = history.start_generation;
        let history_end_generation = history.end_generation;
        if candidate_end_generation.0 <= candidate_start_generation.0 {
            return Err(GeneralLineageHistoricalDesignError::ExactOrInvalidCandidateInterval);
        }
        if candidate_start_generation.0 <= history_start_generation.0
            || candidate_end_generation.0 >= history_end_generation.0
        {
            return Err(GeneralLineageHistoricalDesignError::MissingBeforeOrAfterWindow);
        }

        let pre_end = candidate_start_generation
            .0
            .checked_sub(1)
            .ok_or(GeneralLineageHistoricalDesignError::InvalidTemporalWindow)?;
        let post_start = candidate_end_generation
            .0
            .checked_add(1)
            .ok_or(GeneralLineageHistoricalDesignError::InvalidTemporalWindow)?;
        let pre_interval = HistoricalEvidenceWindowBounds::new(
            HistoricalEvidenceWindow::PreInterval,
            history_start_generation,
            PopulationGeneration(pre_end),
        )?;
        let candidate_interval = HistoricalEvidenceWindowBounds::new(
            HistoricalEvidenceWindow::CandidateInterval,
            candidate_start_generation,
            candidate_end_generation,
        )?;
        if candidate_interval.generation_count < 2 {
            return Err(GeneralLineageHistoricalDesignError::ExactOrInvalidCandidateInterval);
        }
        let post_interval = HistoricalEvidenceWindowBounds::new(
            HistoricalEvidenceWindow::PostInterval,
            PopulationGeneration(post_start),
            history_end_generation,
        )?;

        let mut inputs = BTreeMap::new();
        for input in channel_projection_inputs {
            validate_authority(
                &input.temporal_projection_protocol,
                "temporal_projection_protocol_revision",
            )?;
            if inputs
                .insert(input.channel_id.clone(), input.temporal_projection_protocol)
                .is_some()
            {
                return Err(GeneralLineageHistoricalDesignError::DuplicateChannelProjection);
            }
        }
        let mut channel_projections = Vec::with_capacity(classification.channels.len());
        for declaration in &classification.channels {
            let temporal_projection_protocol = inputs
                .remove(&declaration.channel_id)
                .ok_or(GeneralLineageHistoricalDesignError::IncompleteChannelProjectionCoverage)?;
            channel_projections.push(HistoricalChannelProjection {
                declaration: declaration.clone(),
                temporal_projection_protocol,
            });
        }
        if !inputs.is_empty() {
            return Err(GeneralLineageHistoricalDesignError::UnexpectedChannelProjection);
        }

        let distinct_groups = classification
            .channels
            .iter()
            .map(|channel| channel.dependency_group.clone())
            .collect::<BTreeSet<_>>()
            .len();
        if minimum_temporal_support_dependency_groups < 2 {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdTooLow);
        }
        if minimum_temporal_support_dependency_groups as usize > distinct_groups {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdExceedsGroups);
        }

        let design = Self {
            design_version: GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION,
            design_id,
            classification_design: classification.clone(),
            classification_design_digest: classification_design.design_digest(),
            history_start_generation,
            history_end_generation,
            candidate_start_generation,
            candidate_end_generation,
            pre_interval,
            candidate_interval,
            post_interval,
            channel_projections,
            minimum_temporal_support_dependency_groups,
            missing_policy,
            common_source_protocol_authority,
            post_interval_completeness_protocol_authority,
            rule_authority: general_lineage_historical_evidence_rule_v1(),
        };
        design.validate_local()?;
        Ok(design)
    }

    pub fn window_bounds(&self, window: HistoricalEvidenceWindow) -> HistoricalEvidenceWindowBounds {
        match window {
            HistoricalEvidenceWindow::PreInterval => self.pre_interval,
            HistoricalEvidenceWindow::CandidateInterval => self.candidate_interval,
            HistoricalEvidenceWindow::PostInterval => self.post_interval,
        }
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageHistoricalEvidenceDesignDigest, GeneralLineageHistoricalDesignError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DESIGN_DOMAIN);
        put_u32(&mut digest, self.design_version);
        put_text(&mut digest, self.design_id.as_str());
        digest.update(self.classification_design_digest.as_bytes());
        put_u64(&mut digest, self.history_start_generation.0);
        put_u64(&mut digest, self.history_end_generation.0);
        put_u64(&mut digest, self.candidate_start_generation.0);
        put_u64(&mut digest, self.candidate_end_generation.0);
        self.pre_interval.put(&mut digest);
        self.candidate_interval.put(&mut digest);
        self.post_interval.put(&mut digest);
        put_u64(&mut digest, self.channel_projections.len() as u64);
        for projection in &self.channel_projections {
            projection.put(&mut digest);
        }
        put_u32(&mut digest, self.minimum_temporal_support_dependency_groups);
        digest.update([self.missing_policy.tag()]);
        put_authority(&mut digest, &self.common_source_protocol_authority);
        put_authority(
            &mut digest,
            &self.post_interval_completeness_protocol_authority,
        );
        put_authority(&mut digest, &self.rule_authority);
        Ok(GeneralLineageHistoricalEvidenceDesignDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), GeneralLineageHistoricalDesignError> {
        if self.design_version != GENERAL_LINEAGE_HISTORICAL_EVIDENCE_DESIGN_VERSION {
            return Err(GeneralLineageHistoricalDesignError::UnsupportedVersion(
                self.design_version,
            ));
        }
        if self.classification_design.canonical_digest()? != self.classification_design_digest {
            return Err(GeneralLineageHistoricalDesignError::ClassificationDesignDigestMismatch);
        }
        let history = &self.classification_design.lineage_history_design;
        if self.history_start_generation != history.start_generation
            || self.history_end_generation != history.end_generation
        {
            return Err(GeneralLineageHistoricalDesignError::HistoryBoundsMismatch);
        }
        if self.candidate_end_generation.0 <= self.candidate_start_generation.0
            || self.candidate_start_generation.0 <= self.history_start_generation.0
            || self.candidate_end_generation.0 >= self.history_end_generation.0
        {
            return Err(GeneralLineageHistoricalDesignError::MissingBeforeOrAfterWindow);
        }
        let expected_pre = HistoricalEvidenceWindowBounds::new(
            HistoricalEvidenceWindow::PreInterval,
            self.history_start_generation,
            PopulationGeneration(self.candidate_start_generation.0 - 1),
        )?;
        let expected_candidate = HistoricalEvidenceWindowBounds::new(
            HistoricalEvidenceWindow::CandidateInterval,
            self.candidate_start_generation,
            self.candidate_end_generation,
        )?;
        let expected_post = HistoricalEvidenceWindowBounds::new(
            HistoricalEvidenceWindow::PostInterval,
            PopulationGeneration(self.candidate_end_generation.0 + 1),
            self.history_end_generation,
        )?;
        if expected_candidate.generation_count < 2 {
            return Err(GeneralLineageHistoricalDesignError::ExactOrInvalidCandidateInterval);
        }
        if self.pre_interval != expected_pre
            || self.candidate_interval != expected_candidate
            || self.post_interval != expected_post
        {
            return Err(GeneralLineageHistoricalDesignError::WindowInvariant);
        }
        if self.channel_projections.len() != self.classification_design.channels.len() {
            return Err(GeneralLineageHistoricalDesignError::IncompleteChannelProjectionCoverage);
        }
        for (projection, declaration) in self
            .channel_projections
            .iter()
            .zip(&self.classification_design.channels)
        {
            if &projection.declaration != declaration {
                return Err(GeneralLineageHistoricalDesignError::ChannelProjectionBindingMismatch);
            }
            validate_authority(
                &projection.temporal_projection_protocol,
                "temporal_projection_protocol_revision",
            )?;
        }
        let groups = self
            .classification_design
            .channels
            .iter()
            .map(|channel| channel.dependency_group.clone())
            .collect::<BTreeSet<_>>();
        if self.minimum_temporal_support_dependency_groups < 2 {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdTooLow);
        }
        if self.minimum_temporal_support_dependency_groups as usize > groups.len() {
            return Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdExceedsGroups);
        }
        validate_authority(
            &self.common_source_protocol_authority,
            "common_source_protocol_revision",
        )?;
        validate_authority(
            &self.post_interval_completeness_protocol_authority,
            "post_interval_completeness_protocol_revision",
        )?;
        if self.rule_authority != general_lineage_historical_evidence_rule_v1() {
            return Err(GeneralLineageHistoricalDesignError::RuleAuthorityMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated historical evidence design should gate historical materialization"]
pub struct ValidatedGeneralLineageHistoricalEvidenceDesign<'a> {
    design: &'a GeneralLineageHistoricalEvidenceDesign,
    design_digest: GeneralLineageHistoricalEvidenceDesignDigest,
}

impl<'a> ValidatedGeneralLineageHistoricalEvidenceDesign<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        design: &'a GeneralLineageHistoricalEvidenceDesign,
        classification_design: &ValidatedGeneralLineageClassificationDesign<'_>,
        candidate_start_generation: PopulationGeneration,
        candidate_end_generation: PopulationGeneration,
        channel_projection_inputs: impl IntoIterator<Item = HistoricalChannelProjectionInput>,
        minimum_temporal_support_dependency_groups: u32,
        missing_policy: GeneralLineageHistoricalMissingPolicy,
        common_source_protocol_authority: AnalysisAuthorityRef,
        post_interval_completeness_protocol_authority: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageHistoricalDesignError> {
        design.validate_local()?;
        let recomputed = GeneralLineageHistoricalEvidenceDesign::declare(
            design.design_id.clone(),
            classification_design,
            candidate_start_generation,
            candidate_end_generation,
            channel_projection_inputs,
            minimum_temporal_support_dependency_groups,
            missing_policy,
            common_source_protocol_authority,
            post_interval_completeness_protocol_authority,
        )?;
        if recomputed != *design {
            return Err(GeneralLineageHistoricalDesignError::ReplayMismatch);
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

pub fn general_lineage_historical_evidence_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("general-lineage-historical-evidence-v1")
            .expect("static historical evidence rule ID must be valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

fn channel_kind_id(kind: GeneralLineageEvidenceChannelKind) -> &'static str {
    match kind {
        GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation => "longitudinal-lineage-separation",
        GeneralLineageEvidenceChannelKind::ReproductiveIsolation => "reproductive-isolation",
        GeneralLineageEvidenceChannelKind::GeneticDiagnosability => "genetic-diagnosability",
        GeneralLineageEvidenceChannelKind::EcologicalDifferentiation => "ecological-differentiation",
        GeneralLineageEvidenceChannelKind::MorphologicalDifferentiation => "morphological-differentiation",
        GeneralLineageEvidenceChannelKind::PhylogeneticPattern => "phylogenetic-pattern",
        GeneralLineageEvidenceChannelKind::Other => "other",
    }
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

fn validate_id(field: &'static str, value: &str) -> Result<(), GeneralLineageHistoricalDesignError> {
    if value.is_empty() || value.len() > 96 {
        return Err(GeneralLineageHistoricalDesignError::InvalidId(field));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(GeneralLineageHistoricalDesignError::InvalidId(field));
    }
    Ok(())
}

pub(crate) fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

pub(crate) fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

pub(crate) fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

pub(crate) fn put_u64(digest: &mut Sha256, value: u64) {
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
    GeneralLineage(GeneralLineageDesignError),
    UnsupportedVersion(u32),
    InvalidId(&'static str),
    ZeroAuthorityRevision(&'static str),
    ExactOrInvalidCandidateInterval,
    MissingBeforeOrAfterWindow,
    InvalidTemporalWindow,
    DuplicateChannelProjection,
    IncompleteChannelProjectionCoverage,
    UnexpectedChannelProjection,
    TemporalSupportThresholdTooLow,
    TemporalSupportThresholdExceedsGroups,
    ClassificationDesignDigestMismatch,
    HistoryBoundsMismatch,
    WindowInvariant,
    ChannelProjectionBindingMismatch,
    RuleAuthorityMismatch,
    ReplayMismatch,
}

impl From<GeneralLineageDesignError> for GeneralLineageHistoricalDesignError {
    fn from(value: GeneralLineageDesignError) -> Self {
        Self::GeneralLineage(value)
    }
}

impl fmt::Display for GeneralLineageHistoricalDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GeneralLineage(error) => write!(f, "general-lineage design error: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported historical evidence design version {version}"),
            Self::InvalidId(field) => write!(f, "invalid {field}"),
            Self::ZeroAuthorityRevision(field) => write!(f, "{field} must be nonzero"),
            Self::ExactOrInvalidCandidateInterval => write!(f, "candidate historical interval must span at least two generations"),
            Self::MissingBeforeOrAfterWindow => write!(f, "historical design requires at least one generation before and after the candidate interval"),
            Self::InvalidTemporalWindow => write!(f, "invalid historical evidence window"),
            Self::DuplicateChannelProjection => write!(f, "duplicate historical projection for one classification channel"),
            Self::IncompleteChannelProjectionCoverage => write!(f, "historical design must project every preregistered classification channel exactly once"),
            Self::UnexpectedChannelProjection => write!(f, "historical projection names a channel not present in the classification design"),
            Self::TemporalSupportThresholdTooLow => write!(f, "historical support threshold must require at least two dependency groups"),
            Self::TemporalSupportThresholdExceedsGroups => write!(f, "historical support threshold exceeds the preregistered dependency-group surface"),
            Self::ClassificationDesignDigestMismatch => write!(f, "persisted classification design snapshot/digest mismatch"),
            Self::HistoryBoundsMismatch => write!(f, "historical design bounds differ from the frozen SEL-10A history design"),
            Self::WindowInvariant => write!(f, "persisted before/candidate/after windows do not recompute from frozen bounds"),
            Self::ChannelProjectionBindingMismatch => write!(f, "historical channel projection does not bind the exact preregistered classification channel"),
            Self::RuleAuthorityMismatch => write!(f, "historical evidence design does not bind the built-in V1 rule"),
            Self::ReplayMismatch => write!(f, "persisted historical evidence design does not replay against current authorities"),
        }
    }
}

impl Error for GeneralLineageHistoricalDesignError {}
