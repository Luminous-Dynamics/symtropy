// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Complete temporal evidence materialization for general-lineage historical reasoning.
//!
//! This module records facts over the preregistered before/candidate/after surface.
//! It deliberately emits no historical-transition verdict.

use crate::historical_design::{put_authority, put_text, put_u32, put_u64};
use crate::{
    GeneralLineageEvidenceChannelDeclaration, GeneralLineageEvidenceChannelId,
    GeneralLineageEvidenceChannelKind, GeneralLineageEvidenceChannelRole,
    GeneralLineageHistoricalEvidenceDesign, GeneralLineageHistoricalEvidenceDesignDigest,
    GeneralLineageHistoricalMissingPolicy, HistoricalChannelProjection,
    HistoricalEvidenceWindow, HistoricalEvidenceWindowBounds,
    ValidatedGeneralLineageHistoricalEvidenceDesign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, LineageDivergenceHistory, LineageDivergenceHistoryDigest,
    LineageHistoryEpisodeKind, LineageHistoryEpisodeObservation, LineageHistoryGenerationRecord,
    LineageObservationState, LineagePersistenceState, ReproductiveIsolationEvidence,
    ReproductiveIsolationEvidenceDigest, ValidatedLineageDivergenceHistory,
    ValidatedReproductiveIsolationEvidence,
};

pub const GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION: u32 = 1;
const LEDGER_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:historical-evidence-ledger:v1\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceLedgerDigest([u8; 32]);

impl GeneralLineageHistoricalEvidenceLedgerDigest {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for GeneralLineageHistoricalEvidenceLedgerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GeneralLineageHistoricalEvidenceLedgerDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for GeneralLineageHistoricalEvidenceLedgerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone)]
pub struct QualifiedHistoricalBoundaryEvidenceInput {
    pub evidence_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedHistoricalBoundaryEvidence {
    pub protocol_authority: AnalysisAuthorityRef,
    pub evidence_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

impl QualifiedHistoricalBoundaryEvidence {
    fn materialize(
        protocol_authority: &AnalysisAuthorityRef,
        input: QualifiedHistoricalBoundaryEvidenceInput,
    ) -> Result<Self, GeneralLineageHistoricalEvidenceError> {
        validate_authority(&input.evidence_authority, "boundary_evidence_revision")?;
        validate_authority(
            &input.qualification_authority,
            "boundary_qualification_revision",
        )?;
        Ok(Self {
            protocol_authority: protocol_authority.clone(),
            evidence_authority: input.evidence_authority,
            qualification_authority: input.qualification_authority,
        })
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.protocol_authority);
        put_authority(digest, &self.evidence_authority);
        put_authority(digest, &self.qualification_authority);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalChannelWindowDisposition {
    ObservedSupportsSeparation,
    ObservedDoesNotSupportSeparation,
    ObservedContradictsSeparation,
    Unavailable,
    OutsideChannelDomain,
}

impl HistoricalChannelWindowDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::ObservedSupportsSeparation => 0,
            Self::ObservedDoesNotSupportSeparation => 1,
            Self::ObservedContradictsSeparation => 2,
            Self::Unavailable => 3,
            Self::OutsideChannelDomain => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HistoricalChannelWindowEvidenceInput {
    pub window: HistoricalEvidenceWindow,
    pub disposition: HistoricalChannelWindowDisposition,
    pub evidence_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalChannelWindowEvidenceRecord {
    pub bounds: HistoricalEvidenceWindowBounds,
    pub disposition: HistoricalChannelWindowDisposition,
    pub temporal_projection_protocol: AnalysisAuthorityRef,
    pub evidence_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

impl HistoricalChannelWindowEvidenceRecord {
    fn put(&self, digest: &mut Sha256) {
        self.bounds.put(digest);
        digest.update([self.disposition.tag()]);
        put_authority(digest, &self.temporal_projection_protocol);
        put_authority(digest, &self.evidence_authority);
        put_authority(digest, &self.qualification_authority);
    }
}

#[derive(Debug)]
pub enum GeneralLineageHistoricalChannelInput<'a, 'b> {
    ReproductiveIsolation {
        channel_id: GeneralLineageEvidenceChannelId,
        evidence: &'a ValidatedReproductiveIsolationEvidence<'b>,
        windows: Vec<HistoricalChannelWindowEvidenceInput>,
    },
    External {
        channel_id: GeneralLineageEvidenceChannelId,
        windows: Vec<HistoricalChannelWindowEvidenceInput>,
    },
}

impl GeneralLineageHistoricalChannelInput<'_, '_> {
    fn channel_id(&self) -> &GeneralLineageEvidenceChannelId {
        match self {
            Self::ReproductiveIsolation { channel_id, .. }
            | Self::External { channel_id, .. } => channel_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalLineageWindowSummary {
    pub bounds: HistoricalEvidenceWindowBounds,
    pub unavailable_generation_count: u64,
    pub lineage_a_not_persistent_count: u64,
    pub lineage_b_not_persistent_count: u64,
    pub persistence_unavailable_count: u64,
    pub recontact_observed_count: u64,
    pub recontact_unavailable_count: u64,
    pub gene_flow_observed_count: u64,
    pub gene_flow_unavailable_count: u64,
    pub fusion_observed_count: u64,
    pub fusion_unavailable_count: u64,
    pub demographic_episodes: Vec<LineageHistoryEpisodeObservation>,
}

impl HistoricalLineageWindowSummary {
    fn has_unavailable_required_evidence(&self) -> bool {
        self.unavailable_generation_count > 0
            || self.persistence_unavailable_count > 0
            || self.recontact_unavailable_count > 0
            || self.gene_flow_unavailable_count > 0
            || self.fusion_unavailable_count > 0
    }

    fn put(&self, digest: &mut Sha256) {
        self.bounds.put(digest);
        for value in [
            self.unavailable_generation_count,
            self.lineage_a_not_persistent_count,
            self.lineage_b_not_persistent_count,
            self.persistence_unavailable_count,
            self.recontact_observed_count,
            self.recontact_unavailable_count,
            self.gene_flow_observed_count,
            self.gene_flow_unavailable_count,
            self.fusion_observed_count,
            self.fusion_unavailable_count,
        ] {
            put_u64(digest, value);
        }
        put_u64(digest, self.demographic_episodes.len() as u64);
        for episode in &self.demographic_episodes {
            put_episode(digest, episode);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageHistoricalChannelSource {
    NativeLineageHistory {
        history: LineageDivergenceHistory,
        history_digest: LineageDivergenceHistoryDigest,
        windows: Vec<HistoricalLineageWindowSummary>,
    },
    NativeReproductiveIsolation {
        evidence: ReproductiveIsolationEvidence,
        evidence_digest: ReproductiveIsolationEvidenceDigest,
    },
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalChannelRecord {
    pub declaration: GeneralLineageEvidenceChannelDeclaration,
    pub temporal_projection_protocol: AnalysisAuthorityRef,
    pub source: GeneralLineageHistoricalChannelSource,
    pub windows: Vec<HistoricalChannelWindowEvidenceRecord>,
}

impl GeneralLineageHistoricalChannelRecord {
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.declaration.channel_id.as_str());
        put_authority(digest, &self.temporal_projection_protocol);
        match &self.source {
            GeneralLineageHistoricalChannelSource::NativeLineageHistory {
                history_digest,
                windows,
                ..
            } => {
                digest.update([0]);
                digest.update(history_digest.as_bytes());
                put_u64(digest, windows.len() as u64);
                for window in windows {
                    window.put(digest);
                }
            }
            GeneralLineageHistoricalChannelSource::NativeReproductiveIsolation {
                evidence_digest,
                ..
            } => {
                digest.update([1]);
                digest.update(evidence_digest.as_bytes());
            }
            GeneralLineageHistoricalChannelSource::External => digest.update([2]),
        }
        put_u64(digest, self.windows.len() as u64);
        for window in &self.windows {
            window.put(digest);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceLedger {
    ledger_version: u32,
    pub design: GeneralLineageHistoricalEvidenceDesign,
    pub design_digest: GeneralLineageHistoricalEvidenceDesignDigest,
    pub common_source_evidence: QualifiedHistoricalBoundaryEvidence,
    pub post_interval_completeness_evidence: QualifiedHistoricalBoundaryEvidence,
    pub channels: Vec<GeneralLineageHistoricalChannelRecord>,
}

impl GeneralLineageHistoricalEvidenceLedger {
    #[allow(clippy::too_many_arguments)]
    pub fn capture<'a, 'b, 'c>(
        design: &ValidatedGeneralLineageHistoricalEvidenceDesign<'_>,
        history: &ValidatedLineageDivergenceHistory<'a>,
        common_source_input: QualifiedHistoricalBoundaryEvidenceInput,
        post_interval_completeness_input: QualifiedHistoricalBoundaryEvidenceInput,
        channel_inputs: impl IntoIterator<Item = GeneralLineageHistoricalChannelInput<'b, 'c>>,
    ) -> Result<Self, GeneralLineageHistoricalEvidenceError> {
        let raw = design.design();
        if history.design_digest()
            != raw.classification_design.lineage_history_design_digest
        {
            return Err(GeneralLineageHistoricalEvidenceError::HistoryDesignMismatch);
        }
        let history_snapshot = history.history().clone();
        if history_snapshot.design().start_generation != raw.history_start_generation
            || history_snapshot.design().end_generation != raw.history_end_generation
        {
            return Err(GeneralLineageHistoricalEvidenceError::HistoryBoundsMismatch);
        }
        let common_source_evidence = QualifiedHistoricalBoundaryEvidence::materialize(
            &raw.common_source_protocol_authority,
            common_source_input,
        )?;
        let post_interval_completeness_evidence =
            QualifiedHistoricalBoundaryEvidence::materialize(
                &raw.post_interval_completeness_protocol_authority,
                post_interval_completeness_input,
            )?;

        let mut by_id = BTreeMap::new();
        for input in channel_inputs {
            let id = input.channel_id().clone();
            if by_id.insert(id.clone(), input).is_some() {
                return Err(GeneralLineageHistoricalEvidenceError::DuplicateChannelInput(id));
            }
        }

        let mut channels = Vec::with_capacity(raw.channel_projections.len());
        for projection in &raw.channel_projections {
            if projection.declaration.role == GeneralLineageEvidenceChannelRole::CoreRequired {
                if by_id.remove(&projection.declaration.channel_id).is_some() {
                    return Err(GeneralLineageHistoricalEvidenceError::CoreChannelInputForbidden);
                }
                channels.push(materialize_core_history_channel(
                    raw,
                    projection,
                    history,
                )?);
                continue;
            }
            let input = by_id
                .remove(&projection.declaration.channel_id)
                .ok_or_else(|| {
                    GeneralLineageHistoricalEvidenceError::MissingChannelInput(
                        projection.declaration.channel_id.clone(),
                    )
                })?;
            channels.push(materialize_noncore_channel(raw, projection, input)?);
        }
        if let Some((id, _)) = by_id.into_iter().next() {
            return Err(GeneralLineageHistoricalEvidenceError::UnexpectedChannelInput(id));
        }

        let ledger = Self {
            ledger_version: GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION,
            design: raw.clone(),
            design_digest: design.design_digest(),
            common_source_evidence,
            post_interval_completeness_evidence,
            channels,
        };
        ledger.validate_local()?;
        Ok(ledger)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageHistoricalEvidenceLedgerDigest, GeneralLineageHistoricalEvidenceError>
    {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(LEDGER_DOMAIN);
        put_u32(&mut digest, self.ledger_version);
        digest.update(self.design_digest.as_bytes());
        self.common_source_evidence.put(&mut digest);
        self.post_interval_completeness_evidence.put(&mut digest);
        put_u64(&mut digest, self.channels.len() as u64);
        for channel in &self.channels {
            channel.put(&mut digest);
        }
        Ok(GeneralLineageHistoricalEvidenceLedgerDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        if self.ledger_version != GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION {
            return Err(GeneralLineageHistoricalEvidenceError::UnsupportedVersion(
                self.ledger_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(GeneralLineageHistoricalEvidenceError::DesignDigestMismatch);
        }
        validate_boundary(
            &self.common_source_evidence,
            &self.design.common_source_protocol_authority,
        )?;
        validate_boundary(
            &self.post_interval_completeness_evidence,
            &self.design.post_interval_completeness_protocol_authority,
        )?;
        if self.channels.len() != self.design.channel_projections.len() {
            return Err(GeneralLineageHistoricalEvidenceError::IncompleteChannelCoverage);
        }
        for (projection, record) in self.design.channel_projections.iter().zip(&self.channels) {
            validate_channel_record(&self.design, projection, record)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated general-lineage historical evidence should gate any later historical classifier"]
pub struct ValidatedGeneralLineageHistoricalEvidence<'a> {
    ledger: &'a GeneralLineageHistoricalEvidenceLedger,
    ledger_digest: GeneralLineageHistoricalEvidenceLedgerDigest,
    design_digest: GeneralLineageHistoricalEvidenceDesignDigest,
}

impl<'a> ValidatedGeneralLineageHistoricalEvidence<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current<'b, 'c, 'd>(
        ledger: &'a GeneralLineageHistoricalEvidenceLedger,
        design: &ValidatedGeneralLineageHistoricalEvidenceDesign<'_>,
        history: &ValidatedLineageDivergenceHistory<'b>,
        common_source_input: QualifiedHistoricalBoundaryEvidenceInput,
        post_interval_completeness_input: QualifiedHistoricalBoundaryEvidenceInput,
        channel_inputs: impl IntoIterator<Item = GeneralLineageHistoricalChannelInput<'c, 'd>>,
    ) -> Result<Self, GeneralLineageHistoricalEvidenceError> {
        ledger.validate_local()?;
        let recomputed = GeneralLineageHistoricalEvidenceLedger::capture(
            design,
            history,
            common_source_input,
            post_interval_completeness_input,
            channel_inputs,
        )?;
        if recomputed != *ledger {
            return Err(GeneralLineageHistoricalEvidenceError::ReplayMismatch);
        }
        Ok(Self {
            ledger,
            ledger_digest: ledger.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn ledger(&self) -> &'a GeneralLineageHistoricalEvidenceLedger {
        self.ledger
    }

    pub fn ledger_digest(&self) -> GeneralLineageHistoricalEvidenceLedgerDigest {
        self.ledger_digest
    }

    pub fn design_digest(&self) -> GeneralLineageHistoricalEvidenceDesignDigest {
        self.design_digest
    }
}

fn materialize_core_history_channel(
    design: &GeneralLineageHistoricalEvidenceDesign,
    projection: &HistoricalChannelProjection,
    history: &ValidatedLineageDivergenceHistory<'_>,
) -> Result<GeneralLineageHistoricalChannelRecord, GeneralLineageHistoricalEvidenceError> {
    if projection.declaration.kind
        != GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation
        || projection.declaration.role != GeneralLineageEvidenceChannelRole::CoreRequired
    {
        return Err(GeneralLineageHistoricalEvidenceError::InvalidCoreChannel);
    }
    let history_snapshot = history.history().clone();
    let windows = [
        HistoricalEvidenceWindow::PreInterval,
        HistoricalEvidenceWindow::CandidateInterval,
        HistoricalEvidenceWindow::PostInterval,
    ]
    .into_iter()
    .map(|window| summarize_history_window(design, &history_snapshot, window))
    .collect::<Result<Vec<_>, _>>()?;
    if design.missing_policy == GeneralLineageHistoricalMissingPolicy::FailClosed
        && windows
            .iter()
            .any(HistoricalLineageWindowSummary::has_unavailable_required_evidence)
    {
        return Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden);
    }
    Ok(GeneralLineageHistoricalChannelRecord {
        declaration: projection.declaration.clone(),
        temporal_projection_protocol: projection.temporal_projection_protocol.clone(),
        source: GeneralLineageHistoricalChannelSource::NativeLineageHistory {
            history: history_snapshot,
            history_digest: history.history_digest(),
            windows,
        },
        windows: Vec::new(),
    })
}

fn materialize_noncore_channel(
    design: &GeneralLineageHistoricalEvidenceDesign,
    projection: &HistoricalChannelProjection,
    input: GeneralLineageHistoricalChannelInput<'_, '_>,
) -> Result<GeneralLineageHistoricalChannelRecord, GeneralLineageHistoricalEvidenceError> {
    let (source, windows) = match input {
        GeneralLineageHistoricalChannelInput::ReproductiveIsolation {
            channel_id,
            evidence,
            windows,
        } => {
            if channel_id != projection.declaration.channel_id {
                return Err(GeneralLineageHistoricalEvidenceError::ChannelInputIdMismatch);
            }
            if projection.declaration.kind != GeneralLineageEvidenceChannelKind::ReproductiveIsolation
            {
                return Err(GeneralLineageHistoricalEvidenceError::ChannelSourceKindMismatch);
            }
            let raw = evidence.evidence();
            if raw.design().lineage_a
                != design.classification_design.lineage_history_design.lineage_a
                || raw.design().lineage_b
                    != design.classification_design.lineage_history_design.lineage_b
            {
                return Err(GeneralLineageHistoricalEvidenceError::IsolationLineageMismatch);
            }
            (
                GeneralLineageHistoricalChannelSource::NativeReproductiveIsolation {
                    evidence: raw.clone(),
                    evidence_digest: evidence.evidence_digest(),
                },
                windows,
            )
        }
        GeneralLineageHistoricalChannelInput::External {
            channel_id,
            windows,
        } => {
            if channel_id != projection.declaration.channel_id {
                return Err(GeneralLineageHistoricalEvidenceError::ChannelInputIdMismatch);
            }
            if projection.declaration.kind == GeneralLineageEvidenceChannelKind::ReproductiveIsolation
            {
                return Err(GeneralLineageHistoricalEvidenceError::NativeIsolationAuthorityRequired);
            }
            (GeneralLineageHistoricalChannelSource::External, windows)
        }
    };
    let windows = materialize_window_records(design, projection, windows)?;
    Ok(GeneralLineageHistoricalChannelRecord {
        declaration: projection.declaration.clone(),
        temporal_projection_protocol: projection.temporal_projection_protocol.clone(),
        source,
        windows,
    })
}

fn materialize_window_records(
    design: &GeneralLineageHistoricalEvidenceDesign,
    projection: &HistoricalChannelProjection,
    inputs: Vec<HistoricalChannelWindowEvidenceInput>,
) -> Result<Vec<HistoricalChannelWindowEvidenceRecord>, GeneralLineageHistoricalEvidenceError> {
    let mut by_window = BTreeMap::new();
    for input in inputs {
        validate_authority(&input.evidence_authority, "historical_window_evidence_revision")?;
        validate_authority(
            &input.qualification_authority,
            "historical_window_qualification_revision",
        )?;
        if by_window.insert(input.window, input).is_some() {
            return Err(GeneralLineageHistoricalEvidenceError::DuplicateWindowEvidence);
        }
    }
    let mut records = Vec::with_capacity(3);
    for window in [
        HistoricalEvidenceWindow::PreInterval,
        HistoricalEvidenceWindow::CandidateInterval,
        HistoricalEvidenceWindow::PostInterval,
    ] {
        let input = by_window
            .remove(&window)
            .ok_or(GeneralLineageHistoricalEvidenceError::IncompleteWindowCoverage)?;
        if design.missing_policy == GeneralLineageHistoricalMissingPolicy::FailClosed
            && input.disposition == HistoricalChannelWindowDisposition::Unavailable
        {
            return Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden);
        }
        records.push(HistoricalChannelWindowEvidenceRecord {
            bounds: design.window_bounds(window),
            disposition: input.disposition,
            temporal_projection_protocol: projection.temporal_projection_protocol.clone(),
            evidence_authority: input.evidence_authority,
            qualification_authority: input.qualification_authority,
        });
    }
    if !by_window.is_empty() {
        return Err(GeneralLineageHistoricalEvidenceError::UnexpectedWindowEvidence);
    }
    Ok(records)
}

fn summarize_history_window(
    design: &GeneralLineageHistoricalEvidenceDesign,
    history: &LineageDivergenceHistory,
    window: HistoricalEvidenceWindow,
) -> Result<HistoricalLineageWindowSummary, GeneralLineageHistoricalEvidenceError> {
    let bounds = design.window_bounds(window);
    let mut summary = HistoricalLineageWindowSummary {
        bounds,
        unavailable_generation_count: 0,
        lineage_a_not_persistent_count: 0,
        lineage_b_not_persistent_count: 0,
        persistence_unavailable_count: 0,
        recontact_observed_count: 0,
        recontact_unavailable_count: 0,
        gene_flow_observed_count: 0,
        gene_flow_unavailable_count: 0,
        fusion_observed_count: 0,
        fusion_unavailable_count: 0,
        demographic_episodes: Vec::new(),
    };
    let mut seen = 0u64;
    for generation in &history.generations {
        let coordinate = match generation {
            LineageHistoryGenerationRecord::Observed(record) => record.generation,
            LineageHistoryGenerationRecord::Unavailable { generation, .. } => *generation,
        };
        if coordinate.0 < bounds.start_generation.0 || coordinate.0 > bounds.end_generation.0 {
            continue;
        }
        seen = seen
            .checked_add(1)
            .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?;
        match generation {
            LineageHistoryGenerationRecord::Unavailable { .. } => {
                summary.unavailable_generation_count += 1;
            }
            LineageHistoryGenerationRecord::Observed(record) => {
                if matches!(
                    record.lineage_a_persistence.state,
                    LineagePersistenceState::NotPersistent { .. }
                ) {
                    summary.lineage_a_not_persistent_count += 1;
                }
                if matches!(
                    record.lineage_b_persistence.state,
                    LineagePersistenceState::NotPersistent { .. }
                ) {
                    summary.lineage_b_not_persistent_count += 1;
                }
                for state in [
                    &record.lineage_a_persistence.state,
                    &record.lineage_b_persistence.state,
                ] {
                    if matches!(state, LineagePersistenceState::Unavailable { .. }) {
                        summary.persistence_unavailable_count += 1;
                    }
                }
                count_observation(
                    &record.recontact.state,
                    &mut summary.recontact_observed_count,
                    &mut summary.recontact_unavailable_count,
                );
                count_observation(
                    &record.gene_flow.state,
                    &mut summary.gene_flow_observed_count,
                    &mut summary.gene_flow_unavailable_count,
                );
                count_observation(
                    &record.fusion.state,
                    &mut summary.fusion_observed_count,
                    &mut summary.fusion_unavailable_count,
                );
                summary.demographic_episodes.extend(record.episodes.clone());
            }
        }
    }
    if seen != bounds.generation_count {
        return Err(GeneralLineageHistoricalEvidenceError::IncompleteNativeHistoryWindow);
    }
    Ok(summary)
}

fn count_observation(state: &LineageObservationState, observed: &mut u64, unavailable: &mut u64) {
    match state {
        LineageObservationState::Observed { .. } => *observed += 1,
        LineageObservationState::Unavailable { .. } => *unavailable += 1,
        LineageObservationState::NoneObserved { .. } => {}
    }
}

fn validate_channel_record(
    design: &GeneralLineageHistoricalEvidenceDesign,
    projection: &HistoricalChannelProjection,
    record: &GeneralLineageHistoricalChannelRecord,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    if record.declaration != projection.declaration
        || record.temporal_projection_protocol != projection.temporal_projection_protocol
    {
        return Err(GeneralLineageHistoricalEvidenceError::ChannelProjectionBindingMismatch);
    }
    match &record.source {
        GeneralLineageHistoricalChannelSource::NativeLineageHistory {
            history,
            history_digest,
            windows,
        } => {
            if projection.declaration.kind
                != GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation
                || projection.declaration.role != GeneralLineageEvidenceChannelRole::CoreRequired
                || !record.windows.is_empty()
            {
                return Err(GeneralLineageHistoricalEvidenceError::InvalidCoreChannel);
            }
            if history.canonical_digest()? != *history_digest
                || history.design_digest()
                    != design.classification_design.lineage_history_design_digest
            {
                return Err(GeneralLineageHistoricalEvidenceError::NativeHistoryBindingMismatch);
            }
            let expected = [
                HistoricalEvidenceWindow::PreInterval,
                HistoricalEvidenceWindow::CandidateInterval,
                HistoricalEvidenceWindow::PostInterval,
            ]
            .into_iter()
            .map(|window| summarize_history_window(design, history, window))
            .collect::<Result<Vec<_>, _>>()?;
            if &expected != windows {
                return Err(GeneralLineageHistoricalEvidenceError::NativeHistorySummaryInvariant);
            }
            if design.missing_policy == GeneralLineageHistoricalMissingPolicy::FailClosed
                && windows
                    .iter()
                    .any(HistoricalLineageWindowSummary::has_unavailable_required_evidence)
            {
                return Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden);
            }
        }
        GeneralLineageHistoricalChannelSource::NativeReproductiveIsolation {
            evidence,
            evidence_digest,
        } => {
            if projection.declaration.kind != GeneralLineageEvidenceChannelKind::ReproductiveIsolation
            {
                return Err(GeneralLineageHistoricalEvidenceError::ChannelSourceKindMismatch);
            }
            if evidence.canonical_digest()? != *evidence_digest {
                return Err(GeneralLineageHistoricalEvidenceError::IsolationDigestMismatch);
            }
            if evidence.design().lineage_a
                != design.classification_design.lineage_history_design.lineage_a
                || evidence.design().lineage_b
                    != design.classification_design.lineage_history_design.lineage_b
            {
                return Err(GeneralLineageHistoricalEvidenceError::IsolationLineageMismatch);
            }
            validate_window_records(design, projection, &record.windows)?;
        }
        GeneralLineageHistoricalChannelSource::External => {
            if projection.declaration.kind == GeneralLineageEvidenceChannelKind::ReproductiveIsolation
                || projection.declaration.role == GeneralLineageEvidenceChannelRole::CoreRequired
            {
                return Err(GeneralLineageHistoricalEvidenceError::ChannelSourceKindMismatch);
            }
            validate_window_records(design, projection, &record.windows)?;
        }
    }
    Ok(())
}

fn validate_window_records(
    design: &GeneralLineageHistoricalEvidenceDesign,
    projection: &HistoricalChannelProjection,
    records: &[HistoricalChannelWindowEvidenceRecord],
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    if records.len() != 3 {
        return Err(GeneralLineageHistoricalEvidenceError::IncompleteWindowCoverage);
    }
    for (window, record) in [
        HistoricalEvidenceWindow::PreInterval,
        HistoricalEvidenceWindow::CandidateInterval,
        HistoricalEvidenceWindow::PostInterval,
    ]
    .into_iter()
    .zip(records)
    {
        if record.bounds != design.window_bounds(window)
            || record.temporal_projection_protocol != projection.temporal_projection_protocol
        {
            return Err(GeneralLineageHistoricalEvidenceError::WindowBindingMismatch);
        }
        validate_authority(&record.evidence_authority, "historical_window_evidence_revision")?;
        validate_authority(
            &record.qualification_authority,
            "historical_window_qualification_revision",
        )?;
        if design.missing_policy == GeneralLineageHistoricalMissingPolicy::FailClosed
            && record.disposition == HistoricalChannelWindowDisposition::Unavailable
        {
            return Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden);
        }
    }
    Ok(())
}

fn validate_boundary(
    evidence: &QualifiedHistoricalBoundaryEvidence,
    expected_protocol: &AnalysisAuthorityRef,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    if &evidence.protocol_authority != expected_protocol {
        return Err(GeneralLineageHistoricalEvidenceError::BoundaryProtocolMismatch);
    }
    validate_authority(&evidence.evidence_authority, "boundary_evidence_revision")?;
    validate_authority(
        &evidence.qualification_authority,
        "boundary_qualification_revision",
    )?;
    Ok(())
}

fn put_episode(digest: &mut Sha256, episode: &LineageHistoryEpisodeObservation) {
    put_text(digest, episode.episode_id.as_str());
    put_u64(digest, episode.generation.0);
    digest.update([episode_kind_tag(episode.kind)]);
    match episode.proof_bundle_digest {
        Some(bundle) => {
            digest.update([1]);
            digest.update(bundle.as_bytes());
        }
        None => digest.update([0]),
    }
    put_authority(digest, &episode.evidence.protocol);
    put_authority(digest, &episode.evidence.evidence);
}

fn episode_kind_tag(kind: LineageHistoryEpisodeKind) -> u8 {
    match kind {
        LineageHistoryEpisodeKind::PopulationSplit => 0,
        LineageHistoryEpisodeKind::FounderOrRecolonization => 1,
        LineageHistoryEpisodeKind::AdmixtureOrIntrogression => 2,
        LineageHistoryEpisodeKind::Recontact => 3,
        LineageHistoryEpisodeKind::LineageFusionOrRemerger => 4,
        LineageHistoryEpisodeKind::Extinction => 5,
        LineageHistoryEpisodeKind::Other => 6,
    }
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    if authority.revision == 0 {
        return Err(GeneralLineageHistoricalEvidenceError::ZeroAuthorityRevision(field));
    }
    Ok(())
}

fn fmt_hex(bytes: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum GeneralLineageHistoricalEvidenceError {
    Design(crate::GeneralLineageHistoricalDesignError),
    History(symtropy_evolution_core::LineageDivergenceHistoryError),
    Isolation(symtropy_evolution_core::ReproductiveIsolationEvidenceError),
    UnsupportedVersion(u32),
    ZeroAuthorityRevision(&'static str),
    HistoryDesignMismatch,
    HistoryBoundsMismatch,
    DesignDigestMismatch,
    BoundaryProtocolMismatch,
    DuplicateChannelInput(GeneralLineageEvidenceChannelId),
    MissingChannelInput(GeneralLineageEvidenceChannelId),
    UnexpectedChannelInput(GeneralLineageEvidenceChannelId),
    CoreChannelInputForbidden,
    InvalidCoreChannel,
    ChannelInputIdMismatch,
    ChannelSourceKindMismatch,
    NativeIsolationAuthorityRequired,
    IsolationLineageMismatch,
    DuplicateWindowEvidence,
    IncompleteWindowCoverage,
    UnexpectedWindowEvidence,
    UnavailableEvidenceForbidden,
    ArithmeticOverflow,
    IncompleteNativeHistoryWindow,
    ChannelProjectionBindingMismatch,
    NativeHistoryBindingMismatch,
    NativeHistorySummaryInvariant,
    IsolationDigestMismatch,
    WindowBindingMismatch,
    IncompleteChannelCoverage,
    ReplayMismatch,
}

impl From<crate::GeneralLineageHistoricalDesignError> for GeneralLineageHistoricalEvidenceError {
    fn from(value: crate::GeneralLineageHistoricalDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<symtropy_evolution_core::LineageDivergenceHistoryError>
    for GeneralLineageHistoricalEvidenceError
{
    fn from(value: symtropy_evolution_core::LineageDivergenceHistoryError) -> Self {
        Self::History(value)
    }
}

impl From<symtropy_evolution_core::ReproductiveIsolationEvidenceError>
    for GeneralLineageHistoricalEvidenceError
{
    fn from(value: symtropy_evolution_core::ReproductiveIsolationEvidenceError) -> Self {
        Self::Isolation(value)
    }
}

impl fmt::Display for GeneralLineageHistoricalEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "historical design error: {error}"),
            Self::History(error) => write!(f, "lineage-history error: {error}"),
            Self::Isolation(error) => write!(f, "reproductive-isolation error: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported historical evidence ledger version {version}"),
            Self::ZeroAuthorityRevision(field) => write!(f, "{field} must be nonzero"),
            Self::HistoryDesignMismatch => write!(f, "current SEL-10A history binds a different preregistered history design"),
            Self::HistoryBoundsMismatch => write!(f, "current SEL-10A history has different generation bounds"),
            Self::DesignDigestMismatch => write!(f, "persisted historical design snapshot/digest mismatch"),
            Self::BoundaryProtocolMismatch => write!(f, "historical boundary evidence does not bind the preregistered protocol"),
            Self::DuplicateChannelInput(id) => write!(f, "duplicate historical channel input: {}", id.as_str()),
            Self::MissingChannelInput(id) => write!(f, "missing historical channel input: {}", id.as_str()),
            Self::UnexpectedChannelInput(id) => write!(f, "unexpected historical channel input: {}", id.as_str()),
            Self::CoreChannelInputForbidden => write!(f, "core lineage-history channel is derived only from current SEL-10A history"),
            Self::InvalidCoreChannel => write!(f, "historical core channel is not the required longitudinal-lineage channel"),
            Self::ChannelInputIdMismatch => write!(f, "historical channel input ID does not match the preregistered channel"),
            Self::ChannelSourceKindMismatch => write!(f, "historical channel input source kind does not match the preregistered channel kind"),
            Self::NativeIsolationAuthorityRequired => write!(f, "reproductive-isolation historical channel requires native current SEL-09B authority"),
            Self::IsolationLineageMismatch => write!(f, "reproductive-isolation evidence binds a different ordered lineage pair"),
            Self::DuplicateWindowEvidence => write!(f, "duplicate evidence for one historical window"),
            Self::IncompleteWindowCoverage => write!(f, "non-core historical channel must retain pre/candidate/post window evidence"),
            Self::UnexpectedWindowEvidence => write!(f, "unexpected historical window evidence"),
            Self::UnavailableEvidenceForbidden => write!(f, "frozen historical missing-data policy forbids unavailable required evidence"),
            Self::ArithmeticOverflow => write!(f, "historical summary arithmetic overflow"),
            Self::IncompleteNativeHistoryWindow => write!(f, "native SEL-10A history does not completely cover a preregistered temporal window"),
            Self::ChannelProjectionBindingMismatch => write!(f, "historical channel record does not bind the exact projection/declaration"),
            Self::NativeHistoryBindingMismatch => write!(f, "native lineage-history snapshot/digest does not bind the historical design"),
            Self::NativeHistorySummaryInvariant => write!(f, "persisted native history window summary does not recompute"),
            Self::IsolationDigestMismatch => write!(f, "native reproductive-isolation snapshot/digest mismatch"),
            Self::WindowBindingMismatch => write!(f, "historical window record does not bind the exact preregistered window/protocol"),
            Self::IncompleteChannelCoverage => write!(f, "historical ledger must retain exactly one record per preregistered channel"),
            Self::ReplayMismatch => write!(f, "persisted historical ledger does not replay from current authorities"),
        }
    }
}

impl Error for GeneralLineageHistoricalEvidenceError {}
