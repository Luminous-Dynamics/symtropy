use crate::{
    GeneralLineageEvidenceChannelId, GeneralLineageEvidenceChannelKind,
    GeneralLineageHistoricalChannelDeclaration, GeneralLineageHistoricalDesignError,
    GeneralLineageHistoricalEvidenceDesign, GeneralLineageHistoricalEvidenceDesignDigest,
    GeneralLineageHistoricalMissingPolicy, ValidatedGeneralLineageHistoricalEvidenceDesign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, IsolationStudyUnitId, LineageDivergenceHistoryDigest,
    LineageHistoryGenerationRecord, PopulationGeneration, RealizedGeneFlowObservation,
    ReproductiveContactRecord, ReproductiveIsolationEvidenceDigest,
    ReproductiveOpportunityOutcome, ValidatedLineageDivergenceHistory,
    ValidatedReproductiveIsolationEvidence,
};

pub const GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION: u32 = 1;
const LEDGER_DOMAIN: &[u8] =
    b"symtropy:species-concept:general-lineage:historical-evidence-ledger:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HistoricalChannelWindow {
    BeforeCandidateInterval,
    CandidateInterval,
    AfterCandidateInterval,
}

impl HistoricalChannelWindow {
    fn tag(self) -> u8 {
        match self {
            Self::BeforeCandidateInterval => 0,
            Self::CandidateInterval => 1,
            Self::AfterCandidateInterval => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalWindowBounds {
    pub window: HistoricalChannelWindow,
    pub start_generation: PopulationGeneration,
    pub end_generation: PopulationGeneration,
    pub generation_count: u64,
}

impl HistoricalWindowBounds {
    fn from_design(
        design: &GeneralLineageHistoricalEvidenceDesign,
        window: HistoricalChannelWindow,
    ) -> Result<Self, GeneralLineageHistoricalEvidenceError> {
        let (start_generation, end_generation) = match window {
            HistoricalChannelWindow::BeforeCandidateInterval => (
                design.history_start_generation,
                PopulationGeneration(
                    design
                        .candidate_interval_start
                        .0
                        .checked_sub(1)
                        .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?,
                ),
            ),
            HistoricalChannelWindow::CandidateInterval => (
                design.candidate_interval_start,
                design.candidate_interval_end,
            ),
            HistoricalChannelWindow::AfterCandidateInterval => (
                PopulationGeneration(
                    design
                        .candidate_interval_end
                        .0
                        .checked_add(1)
                        .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?,
                ),
                design.history_end_generation,
            ),
        };
        let generation_count = end_generation
            .0
            .checked_sub(start_generation.0)
            .and_then(|delta| delta.checked_add(1))
            .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?;
        Ok(Self {
            window,
            start_generation,
            end_generation,
            generation_count,
        })
    }

    fn contains(&self, generation: PopulationGeneration) -> bool {
        generation.0 >= self.start_generation.0 && generation.0 <= self.end_generation.0
    }

    fn put(&self, digest: &mut Sha256) {
        digest.update([self.window.tag()]);
        put_u64(digest, self.start_generation.0);
        put_u64(digest, self.end_generation.0);
        put_u64(digest, self.generation_count);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedHistoricalEvidence {
    pub protocol: AnalysisAuthorityRef,
    pub evidence: AnalysisAuthorityRef,
}

impl QualifiedHistoricalEvidence {
    fn new(protocol: &AnalysisAuthorityRef, evidence: AnalysisAuthorityRef) -> Self {
        Self {
            protocol: protocol.clone(),
            evidence,
        }
    }

    fn validate(&self, expected_protocol: &AnalysisAuthorityRef) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        validate_authority(&self.protocol, "qualified_historical_protocol_revision")?;
        validate_authority(&self.evidence, "qualified_historical_evidence_revision")?;
        if &self.protocol != expected_protocol {
            return Err(GeneralLineageHistoricalEvidenceError::ProtocolMismatch);
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.protocol);
        put_authority(digest, &self.evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeLineageHistoryWindow {
    pub bounds: HistoricalWindowBounds,
    pub generations: Vec<LineageHistoryGenerationRecord>,
}

impl NativeLineageHistoryWindow {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        self.bounds.put(digest);
        put_u64(digest, self.generations.len() as u64);
        for generation in &self.generations {
            let encoded = serde_json::to_vec(generation)
                .map_err(|_| GeneralLineageHistoricalEvidenceError::CanonicalEncodingFailure)?;
            put_bytes(digest, &encoded);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeLineageHistoryTemporalEvidence {
    pub history_digest: LineageDivergenceHistoryDigest,
    pub windows: Vec<NativeLineageHistoryWindow>,
}

impl NativeLineageHistoryTemporalEvidence {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        digest.update(self.history_digest.as_bytes());
        put_u64(digest, self.windows.len() as u64);
        for window in &self.windows {
            window.put(digest)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeReproductiveIsolationOpportunityRecord {
    pub unit_id: IsolationStudyUnitId,
    pub generation: PopulationGeneration,
    pub contact_study_digest: symtropy_evolution_core::ReproductiveContactStudyDigest,
    pub record: ReproductiveContactRecord,
}

impl NativeReproductiveIsolationOpportunityRecord {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        put_text(digest, self.unit_id.as_str());
        put_u64(digest, self.generation.0);
        digest.update(self.contact_study_digest.as_bytes());
        let encoded = serde_json::to_vec(&self.record)
            .map_err(|_| GeneralLineageHistoricalEvidenceError::CanonicalEncodingFailure)?;
        put_bytes(digest, &encoded);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeReproductiveIsolationWindow {
    pub bounds: HistoricalWindowBounds,
    pub opportunities: Vec<NativeReproductiveIsolationOpportunityRecord>,
}

impl NativeReproductiveIsolationWindow {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        self.bounds.put(digest);
        put_u64(digest, self.opportunities.len() as u64);
        for opportunity in &self.opportunities {
            opportunity.put(digest)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeReproductiveIsolationTemporalEvidence {
    pub evidence_digest: ReproductiveIsolationEvidenceDigest,
    pub outside_history_opportunity_count: u64,
    pub windows: Vec<NativeReproductiveIsolationWindow>,
}

impl NativeReproductiveIsolationTemporalEvidence {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        digest.update(self.evidence_digest.as_bytes());
        put_u64(digest, self.outside_history_opportunity_count);
        put_u64(digest, self.windows.len() as u64);
        for window in &self.windows {
            window.put(digest)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalChannelDisposition {
    SupportsLineageSeparation,
    DoesNotSupportLineageSeparation,
    ContradictsLineageSeparation,
    Unavailable,
    OutsideChannelDomain,
}

impl HistoricalChannelDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::SupportsLineageSeparation => 0,
            Self::DoesNotSupportLineageSeparation => 1,
            Self::ContradictsLineageSeparation => 2,
            Self::Unavailable => 3,
            Self::OutsideChannelDomain => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalExternalWindowEvidenceInput {
    pub window: HistoricalChannelWindow,
    pub disposition: HistoricalChannelDisposition,
    pub observation_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalExternalChannelEvidenceInput {
    pub channel_id: GeneralLineageEvidenceChannelId,
    pub windows: Vec<HistoricalExternalWindowEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalExternalWindowEvidence {
    pub bounds: HistoricalWindowBounds,
    pub disposition: HistoricalChannelDisposition,
    pub projection_protocol: AnalysisAuthorityRef,
    pub observation_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

impl HistoricalExternalWindowEvidence {
    fn put(&self, digest: &mut Sha256) {
        self.bounds.put(digest);
        digest.update([self.disposition.tag()]);
        put_authority(digest, &self.projection_protocol);
        put_authority(digest, &self.observation_authority);
        put_authority(digest, &self.qualification_authority);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalHistoricalChannelEvidence {
    pub windows: Vec<HistoricalExternalWindowEvidence>,
}

impl ExternalHistoricalChannelEvidence {
    fn put(&self, digest: &mut Sha256) {
        put_u64(digest, self.windows.len() as u64);
        for window in &self.windows {
            window.put(digest);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalChannelEvidenceSource {
    NativeLineageHistory(NativeLineageHistoryTemporalEvidence),
    NativeReproductiveIsolation(NativeReproductiveIsolationTemporalEvidence),
    External(ExternalHistoricalChannelEvidence),
}

impl HistoricalChannelEvidenceSource {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        match self {
            Self::NativeLineageHistory(source) => {
                digest.update([0]);
                source.put(digest)?;
            }
            Self::NativeReproductiveIsolation(source) => {
                digest.update([1]);
                source.put(digest)?;
            }
            Self::External(source) => {
                digest.update([2]);
                source.put(digest);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalChannelEvidenceRecord {
    pub declaration: GeneralLineageHistoricalChannelDeclaration,
    pub source: HistoricalChannelEvidenceSource,
}

impl HistoricalChannelEvidenceRecord {
    fn put(&self, digest: &mut Sha256) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        put_text(digest, self.declaration.current_channel.channel_id.as_str());
        put_authority(digest, &self.declaration.temporal_projection_protocol);
        self.source.put(digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceLedger {
    ledger_version: u32,
    pub design: GeneralLineageHistoricalEvidenceDesign,
    pub design_digest: GeneralLineageHistoricalEvidenceDesignDigest,
    pub common_source_evidence: QualifiedHistoricalEvidence,
    pub historical_context_evidence: QualifiedHistoricalEvidence,
    pub counter_history_completeness_evidence: QualifiedHistoricalEvidence,
    pub channels: Vec<HistoricalChannelEvidenceRecord>,
}

impl GeneralLineageHistoricalEvidenceLedger {
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        design: &ValidatedGeneralLineageHistoricalEvidenceDesign<'_>,
        lineage_history: &ValidatedLineageDivergenceHistory<'_>,
        reproductive_isolation: Option<&ValidatedReproductiveIsolationEvidence<'_>>,
        external_channels: impl IntoIterator<Item = HistoricalExternalChannelEvidenceInput>,
        common_source_evidence: AnalysisAuthorityRef,
        historical_context_evidence: AnalysisAuthorityRef,
        counter_history_completeness_evidence: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageHistoricalEvidenceError> {
        let raw_design = design.design();
        if lineage_history.design_digest()
            != raw_design
                .current_classification_design
                .lineage_history_design_digest
        {
            return Err(GeneralLineageHistoricalEvidenceError::LineageHistoryDesignMismatch);
        }

        let mut external_by_channel = BTreeMap::new();
        for input in external_channels {
            if external_by_channel
                .insert(input.channel_id.clone(), input)
                .is_some()
            {
                return Err(GeneralLineageHistoricalEvidenceError::DuplicateExternalChannel);
            }
        }

        let mut channels = Vec::with_capacity(raw_design.channels.len());
        let mut reproductive_isolation_consumed = false;
        for declaration in &raw_design.channels {
            let source = match declaration.current_channel.kind {
                GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation => {
                    if declaration.current_channel
                        != *raw_design.current_classification_design.core_channel()
                    {
                        return Err(GeneralLineageHistoricalEvidenceError::NativeCoreChannelMismatch);
                    }
                    if external_by_channel
                        .remove(&declaration.current_channel.channel_id)
                        .is_some()
                    {
                        return Err(GeneralLineageHistoricalEvidenceError::NativeChannelExternalOverride);
                    }
                    HistoricalChannelEvidenceSource::NativeLineageHistory(
                        materialize_lineage_history(raw_design, lineage_history)?,
                    )
                }
                GeneralLineageEvidenceChannelKind::ReproductiveIsolation => {
                    if external_by_channel
                        .remove(&declaration.current_channel.channel_id)
                        .is_some()
                    {
                        return Err(GeneralLineageHistoricalEvidenceError::NativeChannelExternalOverride);
                    }
                    let evidence = reproductive_isolation.ok_or(
                        GeneralLineageHistoricalEvidenceError::MissingNativeReproductiveIsolation,
                    )?;
                    reproductive_isolation_consumed = true;
                    HistoricalChannelEvidenceSource::NativeReproductiveIsolation(
                        materialize_reproductive_isolation(raw_design, evidence)?,
                    )
                }
                _ => {
                    let input = external_by_channel
                        .remove(&declaration.current_channel.channel_id)
                        .ok_or(GeneralLineageHistoricalEvidenceError::MissingExternalChannel)?;
                    HistoricalChannelEvidenceSource::External(materialize_external_channel(
                        raw_design,
                        declaration,
                        input,
                    )?)
                }
            };
            channels.push(HistoricalChannelEvidenceRecord {
                declaration: declaration.clone(),
                source,
            });
        }
        if !external_by_channel.is_empty() {
            return Err(GeneralLineageHistoricalEvidenceError::UnexpectedExternalChannel);
        }
        if reproductive_isolation.is_some() && !reproductive_isolation_consumed {
            return Err(GeneralLineageHistoricalEvidenceError::UnexpectedNativeReproductiveIsolation);
        }

        let ledger = Self {
            ledger_version: GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION,
            design: raw_design.clone(),
            design_digest: design.design_digest(),
            common_source_evidence: QualifiedHistoricalEvidence::new(
                &raw_design.common_source_protocol,
                common_source_evidence,
            ),
            historical_context_evidence: QualifiedHistoricalEvidence::new(
                &raw_design.historical_context_protocol,
                historical_context_evidence,
            ),
            counter_history_completeness_evidence: QualifiedHistoricalEvidence::new(
                &raw_design.counter_history_completeness_protocol,
                counter_history_completeness_evidence,
            ),
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
        self.historical_context_evidence.put(&mut digest);
        self.counter_history_completeness_evidence.put(&mut digest);
        put_u64(&mut digest, self.channels.len() as u64);
        for channel in &self.channels {
            channel.put(&mut digest)?;
        }
        Ok(GeneralLineageHistoricalEvidenceLedgerDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), GeneralLineageHistoricalEvidenceError> {
        if self.ledger_version != GENERAL_LINEAGE_HISTORICAL_EVIDENCE_LEDGER_VERSION {
            return Err(GeneralLineageHistoricalEvidenceError::UnsupportedLedgerVersion(
                self.ledger_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(GeneralLineageHistoricalEvidenceError::DesignDigestMismatch);
        }
        self.common_source_evidence
            .validate(&self.design.common_source_protocol)?;
        self.historical_context_evidence
            .validate(&self.design.historical_context_protocol)?;
        self.counter_history_completeness_evidence
            .validate(&self.design.counter_history_completeness_protocol)?;
        if self.channels.len() != self.design.channels.len() {
            return Err(GeneralLineageHistoricalEvidenceError::IncompleteChannelCoverage);
        }
        for (record, declaration) in self.channels.iter().zip(&self.design.channels) {
            if record.declaration != *declaration {
                return Err(GeneralLineageHistoricalEvidenceError::ChannelBindingMismatch);
            }
            validate_channel_source(&self.design, record)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneralLineageHistoricalEvidenceLedgerDigest([u8; 32]);

impl GeneralLineageHistoricalEvidenceLedgerDigest {
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

#[derive(Debug)]
#[must_use = "validated F1A historical evidence should gate any later general-lineage transition classifier"]
pub struct ValidatedGeneralLineageHistoricalEvidence<'a> {
    ledger: &'a GeneralLineageHistoricalEvidenceLedger,
    ledger_digest: GeneralLineageHistoricalEvidenceLedgerDigest,
    design_digest: GeneralLineageHistoricalEvidenceDesignDigest,
}

impl<'a> ValidatedGeneralLineageHistoricalEvidence<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        ledger: &'a GeneralLineageHistoricalEvidenceLedger,
        design: &ValidatedGeneralLineageHistoricalEvidenceDesign<'_>,
        lineage_history: &ValidatedLineageDivergenceHistory<'_>,
        reproductive_isolation: Option<&ValidatedReproductiveIsolationEvidence<'_>>,
        external_channels: impl IntoIterator<Item = HistoricalExternalChannelEvidenceInput>,
        common_source_evidence: AnalysisAuthorityRef,
        historical_context_evidence: AnalysisAuthorityRef,
        counter_history_completeness_evidence: AnalysisAuthorityRef,
    ) -> Result<Self, GeneralLineageHistoricalEvidenceError> {
        ledger.validate_local()?;
        let recomputed = GeneralLineageHistoricalEvidenceLedger::capture(
            design,
            lineage_history,
            reproductive_isolation,
            external_channels,
            common_source_evidence,
            historical_context_evidence,
            counter_history_completeness_evidence,
        )?;
        if recomputed != *ledger {
            return Err(GeneralLineageHistoricalEvidenceError::LedgerReplayMismatch);
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

fn materialize_lineage_history(
    design: &GeneralLineageHistoricalEvidenceDesign,
    history: &ValidatedLineageDivergenceHistory<'_>,
) -> Result<NativeLineageHistoryTemporalEvidence, GeneralLineageHistoricalEvidenceError> {
    let mut windows = Vec::with_capacity(3);
    for window in canonical_windows() {
        let bounds = HistoricalWindowBounds::from_design(design, window)?;
        let generations = history
            .history()
            .generations
            .iter()
            .filter(|record| bounds.contains(lineage_record_generation(record)))
            .cloned()
            .collect::<Vec<_>>();
        if generations.len() != bounds.generation_count as usize {
            return Err(GeneralLineageHistoricalEvidenceError::IncompleteNativeHistoryWindow);
        }
        for (offset, record) in generations.iter().enumerate() {
            let expected = PopulationGeneration(
                bounds
                    .start_generation
                    .0
                    .checked_add(offset as u64)
                    .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?,
            );
            if lineage_record_generation(record) != expected {
                return Err(GeneralLineageHistoricalEvidenceError::NonCanonicalNativeHistoryWindow);
            }
        }
        windows.push(NativeLineageHistoryWindow { bounds, generations });
    }
    Ok(NativeLineageHistoryTemporalEvidence {
        history_digest: history.history_digest(),
        windows,
    })
}

fn materialize_reproductive_isolation(
    design: &GeneralLineageHistoricalEvidenceDesign,
    isolation: &ValidatedReproductiveIsolationEvidence<'_>,
) -> Result<NativeReproductiveIsolationTemporalEvidence, GeneralLineageHistoricalEvidenceError> {
    let raw = isolation.evidence();
    let history_design = &design.current_classification_design.lineage_history_design;
    if raw.design().lineage_a != history_design.lineage_a
        || raw.design().lineage_b != history_design.lineage_b
    {
        return Err(GeneralLineageHistoricalEvidenceError::ReproductiveIsolationLineageMismatch);
    }

    let mut windows = canonical_windows()
        .into_iter()
        .map(|window| {
            Ok(NativeReproductiveIsolationWindow {
                bounds: HistoricalWindowBounds::from_design(design, window)?,
                opportunities: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, GeneralLineageHistoricalEvidenceError>>()?;
    let mut outside_history_opportunity_count = 0_u64;

    for study in &raw.studies {
        let contact = &study.contact_study;
        for (declaration, record) in contact.design().opportunities.iter().zip(&contact.records) {
            let generation = declaration.generation;
            if generation.0 < design.history_start_generation.0
                || generation.0 > design.history_end_generation.0
            {
                outside_history_opportunity_count = outside_history_opportunity_count
                    .checked_add(1)
                    .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?;
                continue;
            }
            let window = windows
                .iter_mut()
                .find(|window| window.bounds.contains(generation))
                .ok_or(GeneralLineageHistoricalEvidenceError::HistoricalWindowCoverageGap)?;
            window
                .opportunities
                .push(NativeReproductiveIsolationOpportunityRecord {
                    unit_id: study.unit_id.clone(),
                    generation,
                    contact_study_digest: study.contact_study_digest,
                    record: record.clone(),
                });
        }
    }
    for window in &mut windows {
        window.opportunities.sort_by(|left, right| {
            left.generation
                .cmp(&right.generation)
                .then_with(|| left.unit_id.cmp(&right.unit_id))
                .then_with(|| left.record.opportunity_id.cmp(&right.record.opportunity_id))
        });
    }
    Ok(NativeReproductiveIsolationTemporalEvidence {
        evidence_digest: isolation.evidence_digest(),
        outside_history_opportunity_count,
        windows,
    })
}

fn materialize_external_channel(
    design: &GeneralLineageHistoricalEvidenceDesign,
    declaration: &GeneralLineageHistoricalChannelDeclaration,
    input: HistoricalExternalChannelEvidenceInput,
) -> Result<ExternalHistoricalChannelEvidence, GeneralLineageHistoricalEvidenceError> {
    if input.channel_id != declaration.current_channel.channel_id {
        return Err(GeneralLineageHistoricalEvidenceError::ChannelBindingMismatch);
    }
    let mut by_window = BTreeMap::new();
    for window in input.windows {
        validate_authority(&window.observation_authority, "external_observation_revision")?;
        validate_authority(&window.qualification_authority, "external_qualification_revision")?;
        if by_window.insert(window.window, window).is_some() {
            return Err(GeneralLineageHistoricalEvidenceError::DuplicateExternalWindow);
        }
    }
    let mut windows = Vec::with_capacity(3);
    for window_kind in canonical_windows() {
        let input = by_window
            .remove(&window_kind)
            .ok_or(GeneralLineageHistoricalEvidenceError::MissingExternalWindow)?;
        if design.missing_policy == GeneralLineageHistoricalMissingPolicy::FailClosed
            && input.disposition == HistoricalChannelDisposition::Unavailable
        {
            return Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden);
        }
        windows.push(HistoricalExternalWindowEvidence {
            bounds: HistoricalWindowBounds::from_design(design, window_kind)?,
            disposition: input.disposition,
            projection_protocol: declaration.temporal_projection_protocol.clone(),
            observation_authority: input.observation_authority,
            qualification_authority: input.qualification_authority,
        });
    }
    if !by_window.is_empty() {
        return Err(GeneralLineageHistoricalEvidenceError::UnexpectedExternalWindow);
    }
    Ok(ExternalHistoricalChannelEvidence { windows })
}

fn validate_channel_source(
    design: &GeneralLineageHistoricalEvidenceDesign,
    record: &HistoricalChannelEvidenceRecord,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    match (&record.declaration.current_channel.kind, &record.source) {
        (
            GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
            HistoricalChannelEvidenceSource::NativeLineageHistory(source),
        ) => validate_native_lineage_history(design, source),
        (
            GeneralLineageEvidenceChannelKind::ReproductiveIsolation,
            HistoricalChannelEvidenceSource::NativeReproductiveIsolation(source),
        ) => validate_native_reproductive_isolation(design, source),
        (
            GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation
            | GeneralLineageEvidenceChannelKind::ReproductiveIsolation,
            _,
        ) => Err(GeneralLineageHistoricalEvidenceError::NativeChannelSourceMismatch),
        (_, HistoricalChannelEvidenceSource::External(source)) => {
            validate_external_source(design, &record.declaration, source)
        }
        (_, _) => Err(GeneralLineageHistoricalEvidenceError::ExternalChannelSourceMismatch),
    }
}

fn validate_native_lineage_history(
    design: &GeneralLineageHistoricalEvidenceDesign,
    source: &NativeLineageHistoryTemporalEvidence,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    validate_three_windows(
        design,
        source.windows.iter().map(|window| &window.bounds),
    )?;
    for window in &source.windows {
        if window.generations.len() != window.bounds.generation_count as usize {
            return Err(GeneralLineageHistoricalEvidenceError::IncompleteNativeHistoryWindow);
        }
        for (offset, record) in window.generations.iter().enumerate() {
            let expected = PopulationGeneration(
                window
                    .bounds
                    .start_generation
                    .0
                    .checked_add(offset as u64)
                    .ok_or(GeneralLineageHistoricalEvidenceError::ArithmeticOverflow)?,
            );
            if lineage_record_generation(record) != expected {
                return Err(GeneralLineageHistoricalEvidenceError::NonCanonicalNativeHistoryWindow);
            }
        }
    }
    Ok(())
}

fn validate_native_reproductive_isolation(
    design: &GeneralLineageHistoricalEvidenceDesign,
    source: &NativeReproductiveIsolationTemporalEvidence,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    validate_three_windows(
        design,
        source.windows.iter().map(|window| &window.bounds),
    )?;
    for window in &source.windows {
        for opportunity in &window.opportunities {
            if !window.bounds.contains(opportunity.generation) {
                return Err(GeneralLineageHistoricalEvidenceError::ReproductiveIsolationWindowMismatch);
            }
        }
        for pair in window.opportunities.windows(2) {
            let left = (
                pair[0].generation,
                &pair[0].unit_id,
                &pair[0].record.opportunity_id,
            );
            let right = (
                pair[1].generation,
                &pair[1].unit_id,
                &pair[1].record.opportunity_id,
            );
            if left >= right {
                return Err(GeneralLineageHistoricalEvidenceError::NonCanonicalReproductiveIsolationOrder);
            }
        }
    }
    Ok(())
}

fn validate_external_source(
    design: &GeneralLineageHistoricalEvidenceDesign,
    declaration: &GeneralLineageHistoricalChannelDeclaration,
    source: &ExternalHistoricalChannelEvidence,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    validate_three_windows(design, source.windows.iter().map(|window| &window.bounds))?;
    for window in &source.windows {
        if window.projection_protocol != declaration.temporal_projection_protocol {
            return Err(GeneralLineageHistoricalEvidenceError::ProtocolMismatch);
        }
        validate_authority(&window.observation_authority, "external_observation_revision")?;
        validate_authority(&window.qualification_authority, "external_qualification_revision")?;
        if design.missing_policy == GeneralLineageHistoricalMissingPolicy::FailClosed
            && window.disposition == HistoricalChannelDisposition::Unavailable
        {
            return Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden);
        }
    }
    Ok(())
}

fn validate_three_windows<'a>(
    design: &GeneralLineageHistoricalEvidenceDesign,
    windows: impl IntoIterator<Item = &'a HistoricalWindowBounds>,
) -> Result<(), GeneralLineageHistoricalEvidenceError> {
    let actual = windows.into_iter().collect::<Vec<_>>();
    if actual.len() != 3 {
        return Err(GeneralLineageHistoricalEvidenceError::IncompleteWindowCoverage);
    }
    for (actual, window) in actual.iter().zip(canonical_windows()) {
        let expected = HistoricalWindowBounds::from_design(design, window)?;
        if **actual != expected {
            return Err(GeneralLineageHistoricalEvidenceError::WindowBindingMismatch);
        }
    }
    Ok(())
}

fn lineage_record_generation(record: &LineageHistoryGenerationRecord) -> PopulationGeneration {
    match record {
        LineageHistoryGenerationRecord::Observed(observed) => observed.generation,
        LineageHistoryGenerationRecord::Unavailable { generation, .. } => *generation,
    }
}

fn canonical_windows() -> [HistoricalChannelWindow; 3] {
    [
        HistoricalChannelWindow::BeforeCandidateInterval,
        HistoricalChannelWindow::CandidateInterval,
        HistoricalChannelWindow::AfterCandidateInterval,
    ]
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

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

fn put_bytes(digest: &mut Sha256, value: &[u8]) {
    put_u64(digest, value.len() as u64);
    digest.update(value);
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
pub enum GeneralLineageHistoricalEvidenceError {
    UnsupportedLedgerVersion(u32),
    ZeroAuthorityRevision(&'static str),
    DesignDigestMismatch,
    ProtocolMismatch,
    LineageHistoryDesignMismatch,
    ReproductiveIsolationLineageMismatch,
    DuplicateExternalChannel,
    MissingExternalChannel,
    UnexpectedExternalChannel,
    NativeCoreChannelMismatch,
    NativeChannelExternalOverride,
    MissingNativeReproductiveIsolation,
    UnexpectedNativeReproductiveIsolation,
    NativeChannelSourceMismatch,
    ExternalChannelSourceMismatch,
    ChannelBindingMismatch,
    IncompleteChannelCoverage,
    IncompleteNativeHistoryWindow,
    NonCanonicalNativeHistoryWindow,
    HistoricalWindowCoverageGap,
    ReproductiveIsolationWindowMismatch,
    NonCanonicalReproductiveIsolationOrder,
    DuplicateExternalWindow,
    MissingExternalWindow,
    UnexpectedExternalWindow,
    IncompleteWindowCoverage,
    WindowBindingMismatch,
    UnavailableEvidenceForbidden,
    LedgerReplayMismatch,
    CanonicalEncodingFailure,
    ArithmeticOverflow,
    HistoricalDesign(GeneralLineageHistoricalDesignError),
    LineageHistory(symtropy_evolution_core::LineageDivergenceHistoryError),
    ReproductiveIsolation(symtropy_evolution_core::ReproductiveIsolationEvidenceError),
}

impl From<GeneralLineageHistoricalDesignError> for GeneralLineageHistoricalEvidenceError {
    fn from(value: GeneralLineageHistoricalDesignError) -> Self {
        Self::HistoricalDesign(value)
    }
}

impl From<symtropy_evolution_core::LineageDivergenceHistoryError>
    for GeneralLineageHistoricalEvidenceError
{
    fn from(value: symtropy_evolution_core::LineageDivergenceHistoryError) -> Self {
        Self::LineageHistory(value)
    }
}

impl From<symtropy_evolution_core::ReproductiveIsolationEvidenceError>
    for GeneralLineageHistoricalEvidenceError
{
    fn from(value: symtropy_evolution_core::ReproductiveIsolationEvidenceError) -> Self {
        Self::ReproductiveIsolation(value)
    }
}

impl fmt::Display for GeneralLineageHistoricalEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLedgerVersion(version) => write!(f, "unsupported F1A ledger version {version}"),
            Self::ZeroAuthorityRevision(field) => write!(f, "{field} must be nonzero"),
            Self::DesignDigestMismatch => write!(f, "embedded F1A design does not match its digest"),
            Self::ProtocolMismatch => write!(f, "historical evidence protocol does not match preregistered protocol"),
            Self::LineageHistoryDesignMismatch => write!(f, "current SEL-10A history does not bind the F1A history design"),
            Self::ReproductiveIsolationLineageMismatch => write!(f, "SEL-09B reproductive-isolation evidence binds a different lineage pair"),
            Self::DuplicateExternalChannel => write!(f, "duplicate external historical channel input"),
            Self::MissingExternalChannel => write!(f, "missing external historical channel input"),
            Self::UnexpectedExternalChannel => write!(f, "unexpected external historical channel input"),
            Self::NativeCoreChannelMismatch => write!(f, "longitudinal historical source does not bind the exact current core channel"),
            Self::NativeChannelExternalOverride => write!(f, "native historical channel cannot be replaced by an opaque external assertion"),
            Self::MissingNativeReproductiveIsolation => write!(f, "declared reproductive-isolation channel requires current SEL-09B evidence"),
            Self::UnexpectedNativeReproductiveIsolation => write!(f, "SEL-09B evidence supplied but reproductive isolation was not preregistered"),
            Self::NativeChannelSourceMismatch => write!(f, "native channel persisted with the wrong source type"),
            Self::ExternalChannelSourceMismatch => write!(f, "external channel persisted with a native source type"),
            Self::ChannelBindingMismatch => write!(f, "historical channel record does not bind the exact preregistered channel"),
            Self::IncompleteChannelCoverage => write!(f, "F1A ledger must retain exactly one record per preregistered channel"),
            Self::IncompleteNativeHistoryWindow => write!(f, "native SEL-10A historical window is not generation-complete"),
            Self::NonCanonicalNativeHistoryWindow => write!(f, "native SEL-10A historical window is not in exact generation order"),
            Self::HistoricalWindowCoverageGap => write!(f, "historical evidence generation is not covered by before/interval/after windows"),
            Self::ReproductiveIsolationWindowMismatch => write!(f, "SEL-09B opportunity is stored in the wrong historical window"),
            Self::NonCanonicalReproductiveIsolationOrder => write!(f, "SEL-09B temporal opportunities are not canonically ordered"),
            Self::DuplicateExternalWindow => write!(f, "duplicate external historical window"),
            Self::MissingExternalWindow => write!(f, "external historical channel is missing a required window"),
            Self::UnexpectedExternalWindow => write!(f, "external historical channel has an unexpected window"),
            Self::IncompleteWindowCoverage => write!(f, "historical channel must retain before, candidate, and after windows"),
            Self::WindowBindingMismatch => write!(f, "historical window bounds do not match the preregistered design"),
            Self::UnavailableEvidenceForbidden => write!(f, "unavailable external temporal evidence is forbidden by the F1A missing policy"),
            Self::LedgerReplayMismatch => write!(f, "persisted F1A ledger does not replay from fresh current authorities"),
            Self::CanonicalEncodingFailure => write!(f, "failed to encode an embedded native evidence record canonically"),
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow while materializing F1A evidence"),
            Self::HistoricalDesign(error) => write!(f, "F1A historical design error: {error}"),
            Self::LineageHistory(error) => write!(f, "SEL-10A history error: {error}"),
            Self::ReproductiveIsolation(error) => write!(f, "SEL-09B evidence error: {error}"),
        }
    }
}

impl Error for GeneralLineageHistoricalEvidenceError {}

#[allow(dead_code)]
fn _preserve_native_semantic_types(
    outcome: &ReproductiveOpportunityOutcome,
    gene_flow: &RealizedGeneFlowObservation,
) {
    let _ = (outcome, gene_flow);
}
