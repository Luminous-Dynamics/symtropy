use crate::design::{
    GeneralLineageClassificationDesign, GeneralLineageClassificationDesignDigest,
    GeneralLineageDesignError, GeneralLineageEvidenceChannelDeclaration,
    GeneralLineageEvidenceChannelId, GeneralLineageEvidenceChannelKind,
    GeneralLineageEvidenceChannelRole, GeneralLineageEvidenceDependencyGroupId,
    GeneralLineageMissingEvidencePolicy, ValidatedGeneralLineageClassificationDesign,
};
use crate::model::{
    GeneralLineageModelError, GeneralLineageSpeciesModelDigest,
    ValidatedGeneralLineageSpeciesModel,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};
use symtropy_evolution_core::{
    AnalysisAuthorityRef, LineageDivergenceHistory, LineageDivergenceHistoryDigest,
    LineageDivergenceHistoryStatus, ReproductiveIsolationEvidence,
    ReproductiveIsolationEvidenceDigest, ReproductiveIsolationStatus,
    ValidatedLineageDivergenceHistory, ValidatedReproductiveIsolationEvidence,
};

pub const GENERAL_LINEAGE_SPECIES_EVIDENCE_VERSION: u32 = 1;
const EVIDENCE_DOMAIN: &[u8] = b"symtropy:species-concept:general-lineage:current-evidence:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageModelApplicabilityDisposition {
    InDomain,
    OutsideModelValidityDomain,
    Unavailable,
}

impl GeneralLineageModelApplicabilityDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::InDomain => 0,
            Self::OutsideModelValidityDomain => 1,
            Self::Unavailable => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneralLineageModelApplicabilityInput {
    pub disposition: GeneralLineageModelApplicabilityDisposition,
    pub evidence_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageModelApplicabilityEvidence {
    pub disposition: GeneralLineageModelApplicabilityDisposition,
    pub protocol_authority: AnalysisAuthorityRef,
    pub evidence_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
    pub model_digest: GeneralLineageSpeciesModelDigest,
    pub lineage_history_design_digest:
        symtropy_evolution_core::LineageDivergenceHistoryDesignDigest,
}

impl GeneralLineageModelApplicabilityEvidence {
    fn materialize(
        design: &GeneralLineageClassificationDesign,
        input: GeneralLineageModelApplicabilityInput,
    ) -> Result<Self, GeneralLineageEvidenceError> {
        validate_authority(&input.evidence_authority, "model_applicability_evidence_revision")?;
        validate_authority(
            &input.qualification_authority,
            "model_applicability_qualification_revision",
        )?;
        Ok(Self {
            disposition: input.disposition,
            protocol_authority: design.model_applicability_protocol.clone(),
            evidence_authority: input.evidence_authority,
            qualification_authority: input.qualification_authority,
            model_digest: design.model_digest,
            lineage_history_design_digest: design.lineage_history_design_digest,
        })
    }

    fn validate_local(
        &self,
        design: &GeneralLineageClassificationDesign,
    ) -> Result<(), GeneralLineageEvidenceError> {
        if self.protocol_authority != design.model_applicability_protocol
            || self.model_digest != design.model_digest
            || self.lineage_history_design_digest != design.lineage_history_design_digest
        {
            return Err(GeneralLineageEvidenceError::ModelApplicabilityBindingMismatch);
        }
        validate_authority(&self.evidence_authority, "model_applicability_evidence_revision")?;
        validate_authority(
            &self.qualification_authority,
            "model_applicability_qualification_revision",
        )?;
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        digest.update([self.disposition.tag()]);
        put_authority(digest, &self.protocol_authority);
        put_authority(digest, &self.evidence_authority);
        put_authority(digest, &self.qualification_authority);
        digest.update(self.model_digest.as_bytes());
        digest.update(self.lineage_history_design_digest.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageChannelDisposition {
    SupportsSeparation,
    DoesNotSupportSeparation,
    ContradictsSeparation,
    Unavailable,
    OutsideChannelDomain,
}

impl GeneralLineageChannelDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::SupportsSeparation => 0,
            Self::DoesNotSupportSeparation => 1,
            Self::ContradictsSeparation => 2,
            Self::Unavailable => 3,
            Self::OutsideChannelDomain => 4,
        }
    }
}

#[derive(Debug)]
pub enum GeneralLineageChannelEvidenceInput<'a, 'b> {
    ReproductiveIsolation {
        channel_id: GeneralLineageEvidenceChannelId,
        evidence: &'a ValidatedReproductiveIsolationEvidence<'b>,
    },
    External {
        channel_id: GeneralLineageEvidenceChannelId,
        disposition: GeneralLineageChannelDisposition,
        evidence_authority: AnalysisAuthorityRef,
        qualification_authority: AnalysisAuthorityRef,
    },
}

impl GeneralLineageChannelEvidenceInput<'_, '_> {
    fn channel_id(&self) -> &GeneralLineageEvidenceChannelId {
        match self {
            Self::ReproductiveIsolation { channel_id, .. }
            | Self::External { channel_id, .. } => channel_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageChannelEvidenceSource {
    LineageHistory {
        history: LineageDivergenceHistory,
        history_digest: LineageDivergenceHistoryDigest,
    },
    ReproductiveIsolation {
        evidence: ReproductiveIsolationEvidence,
        evidence_digest: ReproductiveIsolationEvidenceDigest,
    },
    External {
        evidence_authority: AnalysisAuthorityRef,
        qualification_authority: AnalysisAuthorityRef,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageChannelEvidenceRecord {
    pub declaration: GeneralLineageEvidenceChannelDeclaration,
    pub disposition: GeneralLineageChannelDisposition,
    pub source: GeneralLineageChannelEvidenceSource,
}

impl GeneralLineageChannelEvidenceRecord {
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.declaration.channel_id.as_str());
        digest.update([self.disposition.tag()]);
        match &self.source {
            GeneralLineageChannelEvidenceSource::LineageHistory {
                history_digest, ..
            } => {
                digest.update([0]);
                digest.update(history_digest.as_bytes());
            }
            GeneralLineageChannelEvidenceSource::ReproductiveIsolation {
                evidence_digest, ..
            } => {
                digest.update([1]);
                digest.update(evidence_digest.as_bytes());
            }
            GeneralLineageChannelEvidenceSource::External {
                evidence_authority,
                qualification_authority,
            } => {
                digest.update([2]);
                put_authority(digest, evidence_authority);
                put_authority(digest, qualification_authority);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralLineageSpeciesStatus {
    SupportedUnderGeneralLineageModel,
    NotSupportedUnderGeneralLineageModel,
    ContradictedUnderGeneralLineageModel,
    InsufficientIndependentEvidence,
    OutsideModelValidityDomain,
}

impl GeneralLineageSpeciesStatus {
    fn tag(self) -> u8 {
        match self {
            Self::SupportedUnderGeneralLineageModel => 0,
            Self::NotSupportedUnderGeneralLineageModel => 1,
            Self::ContradictedUnderGeneralLineageModel => 2,
            Self::InsufficientIndependentEvidence => 3,
            Self::OutsideModelValidityDomain => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralLineageSpeciesEvidence {
    evidence_version: u32,
    pub design: GeneralLineageClassificationDesign,
    pub design_digest: GeneralLineageClassificationDesignDigest,
    pub applicability: GeneralLineageModelApplicabilityEvidence,
    pub channels: Vec<GeneralLineageChannelEvidenceRecord>,
    pub supporting_dependency_groups: Vec<GeneralLineageEvidenceDependencyGroupId>,
    pub unavailable_potential_dependency_groups: Vec<GeneralLineageEvidenceDependencyGroupId>,
    pub status: GeneralLineageSpeciesStatus,
}

impl GeneralLineageSpeciesEvidence {
    pub fn evaluate<'a, 'b, 'c>(
        design: &ValidatedGeneralLineageClassificationDesign<'_>,
        history: &ValidatedLineageDivergenceHistory<'a>,
        model: &ValidatedGeneralLineageSpeciesModel<'_>,
        applicability_input: GeneralLineageModelApplicabilityInput,
        channel_inputs: impl IntoIterator<Item = GeneralLineageChannelEvidenceInput<'b, 'c>>,
    ) -> Result<Self, GeneralLineageEvidenceError> {
        let raw = design.design();
        if history.design_digest() != raw.lineage_history_design_digest {
            return Err(GeneralLineageEvidenceError::LineageHistoryDesignMismatch);
        }
        if model.model_digest() != raw.model_digest {
            return Err(GeneralLineageEvidenceError::ModelDigestMismatch);
        }

        let applicability = GeneralLineageModelApplicabilityEvidence::materialize(
            raw,
            applicability_input,
        )?;
        if applicability.disposition == GeneralLineageModelApplicabilityDisposition::Unavailable
            && raw.missing_policy == GeneralLineageMissingEvidencePolicy::FailClosed
        {
            return Err(GeneralLineageEvidenceError::MissingEvidenceFailClosed);
        }

        let mut by_id = BTreeMap::new();
        for input in channel_inputs {
            let id = input.channel_id().clone();
            if by_id.insert(id.clone(), input).is_some() {
                return Err(GeneralLineageEvidenceError::DuplicateChannelInput(id));
            }
        }

        let mut channels = Vec::with_capacity(raw.channels.len());
        for declaration in &raw.channels {
            if declaration.role == GeneralLineageEvidenceChannelRole::CoreRequired {
                channels.push(materialize_core_history_channel(declaration, history)?);
                if by_id.remove(&declaration.channel_id).is_some() {
                    return Err(GeneralLineageEvidenceError::CoreChannelInputForbidden);
                }
                continue;
            }
            let input = by_id
                .remove(&declaration.channel_id)
                .ok_or_else(|| {
                    GeneralLineageEvidenceError::MissingChannelInput(
                        declaration.channel_id.clone(),
                    )
                })?;
            channels.push(materialize_noncore_channel(raw, declaration, input)?);
        }
        if let Some((unexpected, _)) = by_id.into_iter().next() {
            return Err(GeneralLineageEvidenceError::UnexpectedChannelInput(unexpected));
        }

        let (supporting_dependency_groups, unavailable_potential_dependency_groups, status) =
            derive_status(raw, &applicability, &channels)?;
        let evidence = Self {
            evidence_version: GENERAL_LINEAGE_SPECIES_EVIDENCE_VERSION,
            design: raw.clone(),
            design_digest: design.design_digest(),
            applicability,
            channels,
            supporting_dependency_groups,
            unavailable_potential_dependency_groups,
            status,
        };
        evidence.validate_local()?;
        Ok(evidence)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<GeneralLineageSpeciesEvidenceDigest, GeneralLineageEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(EVIDENCE_DOMAIN);
        put_u32(&mut digest, self.evidence_version);
        digest.update(self.design_digest.as_bytes());
        self.applicability.put(&mut digest);
        put_u64(&mut digest, self.channels.len() as u64);
        for channel in &self.channels {
            channel.put(&mut digest);
        }
        put_u64(&mut digest, self.supporting_dependency_groups.len() as u64);
        for group in &self.supporting_dependency_groups {
            put_text(&mut digest, group.as_str());
        }
        put_u64(
            &mut digest,
            self.unavailable_potential_dependency_groups.len() as u64,
        );
        for group in &self.unavailable_potential_dependency_groups {
            put_text(&mut digest, group.as_str());
        }
        digest.update([self.status.tag()]);
        Ok(GeneralLineageSpeciesEvidenceDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), GeneralLineageEvidenceError> {
        if self.evidence_version != GENERAL_LINEAGE_SPECIES_EVIDENCE_VERSION {
            return Err(GeneralLineageEvidenceError::UnsupportedEvidenceVersion(
                self.evidence_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(GeneralLineageEvidenceError::DesignDigestMismatch);
        }
        self.applicability.validate_local(&self.design)?;
        if self.channels.len() != self.design.channels.len() {
            return Err(GeneralLineageEvidenceError::IncompleteChannelCoverage);
        }
        for (declaration, record) in self.design.channels.iter().zip(&self.channels) {
            if declaration != &record.declaration {
                return Err(GeneralLineageEvidenceError::ChannelDeclarationMismatch);
            }
            validate_record_local(&self.design, record)?;
        }
        let (supporting, unavailable, status) =
            derive_status(&self.design, &self.applicability, &self.channels)?;
        if supporting != self.supporting_dependency_groups
            || unavailable != self.unavailable_potential_dependency_groups
        {
            return Err(GeneralLineageEvidenceError::DependencyGroupInvariant);
        }
        if status != self.status {
            return Err(GeneralLineageEvidenceError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneralLineageSpeciesEvidenceDigest([u8; 32]);

impl GeneralLineageSpeciesEvidenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for GeneralLineageSpeciesEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GeneralLineageSpeciesEvidenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for GeneralLineageSpeciesEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated general-lineage species evidence should gate any downstream model comparison"]
pub struct ValidatedGeneralLineageSpeciesEvidence<'a> {
    evidence: &'a GeneralLineageSpeciesEvidence,
    evidence_digest: GeneralLineageSpeciesEvidenceDigest,
}

impl<'a> ValidatedGeneralLineageSpeciesEvidence<'a> {
    pub fn validate_current<'b, 'c, 'd>(
        evidence: &'a GeneralLineageSpeciesEvidence,
        design: &ValidatedGeneralLineageClassificationDesign<'_>,
        history: &ValidatedLineageDivergenceHistory<'b>,
        model: &ValidatedGeneralLineageSpeciesModel<'_>,
        applicability_input: GeneralLineageModelApplicabilityInput,
        channel_inputs: impl IntoIterator<Item = GeneralLineageChannelEvidenceInput<'c, 'd>>,
    ) -> Result<Self, GeneralLineageEvidenceError> {
        evidence.validate_local()?;
        let recomputed = GeneralLineageSpeciesEvidence::evaluate(
            design,
            history,
            model,
            applicability_input,
            channel_inputs,
        )?;
        if recomputed != *evidence {
            return Err(GeneralLineageEvidenceError::ReplayMismatch);
        }
        Ok(Self {
            evidence,
            evidence_digest: evidence.canonical_digest()?,
        })
    }

    pub fn evidence(&self) -> &'a GeneralLineageSpeciesEvidence {
        self.evidence
    }

    pub fn evidence_digest(&self) -> GeneralLineageSpeciesEvidenceDigest {
        self.evidence_digest
    }
}

fn materialize_core_history_channel(
    declaration: &GeneralLineageEvidenceChannelDeclaration,
    history: &ValidatedLineageDivergenceHistory<'_>,
) -> Result<GeneralLineageChannelEvidenceRecord, GeneralLineageEvidenceError> {
    if declaration.kind != GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation
        || declaration.role != GeneralLineageEvidenceChannelRole::CoreRequired
    {
        return Err(GeneralLineageEvidenceError::InvalidCoreChannel);
    }
    let disposition = lineage_history_disposition(history.history().status);
    Ok(GeneralLineageChannelEvidenceRecord {
        declaration: declaration.clone(),
        disposition,
        source: GeneralLineageChannelEvidenceSource::LineageHistory {
            history: history.history().clone(),
            history_digest: history.history_digest(),
        },
    })
}

fn materialize_noncore_channel(
    design: &GeneralLineageClassificationDesign,
    declaration: &GeneralLineageEvidenceChannelDeclaration,
    input: GeneralLineageChannelEvidenceInput<'_, '_>,
) -> Result<GeneralLineageChannelEvidenceRecord, GeneralLineageEvidenceError> {
    match input {
        GeneralLineageChannelEvidenceInput::ReproductiveIsolation {
            channel_id,
            evidence,
        } => {
            if channel_id != declaration.channel_id {
                return Err(GeneralLineageEvidenceError::ChannelInputIdMismatch);
            }
            if declaration.kind != GeneralLineageEvidenceChannelKind::ReproductiveIsolation {
                return Err(GeneralLineageEvidenceError::ChannelSourceKindMismatch);
            }
            let isolation = evidence.evidence();
            if isolation.design().lineage_a != design.lineage_history_design.lineage_a
                || isolation.design().lineage_b != design.lineage_history_design.lineage_b
            {
                return Err(GeneralLineageEvidenceError::ReproductiveIsolationLineageMismatch);
            }
            Ok(GeneralLineageChannelEvidenceRecord {
                declaration: declaration.clone(),
                disposition: reproductive_isolation_disposition(isolation.status),
                source: GeneralLineageChannelEvidenceSource::ReproductiveIsolation {
                    evidence: isolation.clone(),
                    evidence_digest: evidence.evidence_digest(),
                },
            })
        }
        GeneralLineageChannelEvidenceInput::External {
            channel_id,
            disposition,
            evidence_authority,
            qualification_authority,
        } => {
            if channel_id != declaration.channel_id {
                return Err(GeneralLineageEvidenceError::ChannelInputIdMismatch);
            }
            if declaration.kind == GeneralLineageEvidenceChannelKind::ReproductiveIsolation {
                return Err(GeneralLineageEvidenceError::NativeIsolationAuthorityRequired);
            }
            validate_authority(&evidence_authority, "channel_evidence_revision")?;
            validate_authority(&qualification_authority, "channel_qualification_revision")?;
            if disposition == GeneralLineageChannelDisposition::Unavailable
                && design.missing_policy == GeneralLineageMissingEvidencePolicy::FailClosed
            {
                return Err(GeneralLineageEvidenceError::MissingEvidenceFailClosed);
            }
            Ok(GeneralLineageChannelEvidenceRecord {
                declaration: declaration.clone(),
                disposition,
                source: GeneralLineageChannelEvidenceSource::External {
                    evidence_authority,
                    qualification_authority,
                },
            })
        }
    }
}

fn validate_record_local(
    design: &GeneralLineageClassificationDesign,
    record: &GeneralLineageChannelEvidenceRecord,
) -> Result<(), GeneralLineageEvidenceError> {
    match &record.source {
        GeneralLineageChannelEvidenceSource::LineageHistory {
            history,
            history_digest,
        } => {
            if record.declaration.role != GeneralLineageEvidenceChannelRole::CoreRequired
                || record.declaration.kind
                    != GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation
            {
                return Err(GeneralLineageEvidenceError::InvalidCoreChannel);
            }
            if history.design_digest() != design.lineage_history_design_digest
                || history.canonical_digest()? != *history_digest
            {
                return Err(GeneralLineageEvidenceError::LineageHistoryBindingMismatch);
            }
            if record.disposition != lineage_history_disposition(history.status) {
                return Err(GeneralLineageEvidenceError::ChannelDispositionInvariant);
            }
        }
        GeneralLineageChannelEvidenceSource::ReproductiveIsolation {
            evidence,
            evidence_digest,
        } => {
            if record.declaration.kind != GeneralLineageEvidenceChannelKind::ReproductiveIsolation
            {
                return Err(GeneralLineageEvidenceError::ChannelSourceKindMismatch);
            }
            if evidence.design().lineage_a != design.lineage_history_design.lineage_a
                || evidence.design().lineage_b != design.lineage_history_design.lineage_b
                || evidence.canonical_digest()? != *evidence_digest
            {
                return Err(GeneralLineageEvidenceError::ReproductiveIsolationLineageMismatch);
            }
            if record.disposition != reproductive_isolation_disposition(evidence.status) {
                return Err(GeneralLineageEvidenceError::ChannelDispositionInvariant);
            }
        }
        GeneralLineageChannelEvidenceSource::External {
            evidence_authority,
            qualification_authority,
        } => {
            if record.declaration.role == GeneralLineageEvidenceChannelRole::CoreRequired {
                return Err(GeneralLineageEvidenceError::InvalidCoreChannel);
            }
            if record.declaration.kind == GeneralLineageEvidenceChannelKind::ReproductiveIsolation {
                return Err(GeneralLineageEvidenceError::NativeIsolationAuthorityRequired);
            }
            validate_authority(evidence_authority, "channel_evidence_revision")?;
            validate_authority(qualification_authority, "channel_qualification_revision")?;
            if record.disposition == GeneralLineageChannelDisposition::Unavailable
                && design.missing_policy == GeneralLineageMissingEvidencePolicy::FailClosed
            {
                return Err(GeneralLineageEvidenceError::MissingEvidenceFailClosed);
            }
        }
    }
    Ok(())
}

fn derive_status(
    design: &GeneralLineageClassificationDesign,
    applicability: &GeneralLineageModelApplicabilityEvidence,
    channels: &[GeneralLineageChannelEvidenceRecord],
) -> Result<(
    Vec<GeneralLineageEvidenceDependencyGroupId>,
    Vec<GeneralLineageEvidenceDependencyGroupId>,
    GeneralLineageSpeciesStatus,
), GeneralLineageEvidenceError> {
    match applicability.disposition {
        GeneralLineageModelApplicabilityDisposition::OutsideModelValidityDomain => {
            return Ok((Vec::new(), Vec::new(), GeneralLineageSpeciesStatus::OutsideModelValidityDomain));
        }
        GeneralLineageModelApplicabilityDisposition::Unavailable => {
            if design.missing_policy == GeneralLineageMissingEvidencePolicy::FailClosed {
                return Err(GeneralLineageEvidenceError::MissingEvidenceFailClosed);
            }
            return Ok((Vec::new(), Vec::new(), GeneralLineageSpeciesStatus::InsufficientIndependentEvidence));
        }
        GeneralLineageModelApplicabilityDisposition::InDomain => {}
    }

    if channels
        .iter()
        .any(|record| record.disposition == GeneralLineageChannelDisposition::ContradictsSeparation)
    {
        return Ok((Vec::new(), Vec::new(), GeneralLineageSpeciesStatus::ContradictedUnderGeneralLineageModel));
    }

    let core = channels
        .iter()
        .find(|record| record.declaration.role == GeneralLineageEvidenceChannelRole::CoreRequired)
        .ok_or(GeneralLineageEvidenceError::InvalidCoreChannel)?;
    match core.disposition {
        GeneralLineageChannelDisposition::SupportsSeparation => {}
        GeneralLineageChannelDisposition::Unavailable
        | GeneralLineageChannelDisposition::OutsideChannelDomain => {
            if design.missing_policy == GeneralLineageMissingEvidencePolicy::FailClosed
                && core.disposition == GeneralLineageChannelDisposition::Unavailable
            {
                return Err(GeneralLineageEvidenceError::MissingEvidenceFailClosed);
            }
            return Ok((Vec::new(), vec![core.declaration.dependency_group_id.clone()], GeneralLineageSpeciesStatus::InsufficientIndependentEvidence));
        }
        GeneralLineageChannelDisposition::DoesNotSupportSeparation => {
            return Ok((Vec::new(), Vec::new(), GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel));
        }
        GeneralLineageChannelDisposition::ContradictsSeparation => unreachable!("contradiction handled above"),
    }

    let mut support = BTreeSet::new();
    let mut unavailable = BTreeSet::new();
    let mut required_not_supported = false;
    let mut required_missing = false;

    for record in channels {
        match record.disposition {
            GeneralLineageChannelDisposition::SupportsSeparation => {
                support.insert(record.declaration.dependency_group_id.clone());
            }
            GeneralLineageChannelDisposition::DoesNotSupportSeparation => {
                if matches!(
                    record.declaration.role,
                    GeneralLineageEvidenceChannelRole::CoreRequired
                        | GeneralLineageEvidenceChannelRole::Required
                ) {
                    required_not_supported = true;
                }
            }
            GeneralLineageChannelDisposition::Unavailable => {
                if design.missing_policy == GeneralLineageMissingEvidencePolicy::FailClosed {
                    return Err(GeneralLineageEvidenceError::MissingEvidenceFailClosed);
                }
                unavailable.insert(record.declaration.dependency_group_id.clone());
                if matches!(
                    record.declaration.role,
                    GeneralLineageEvidenceChannelRole::CoreRequired
                        | GeneralLineageEvidenceChannelRole::Required
                ) {
                    required_missing = true;
                }
            }
            GeneralLineageChannelDisposition::OutsideChannelDomain => {
                if matches!(
                    record.declaration.role,
                    GeneralLineageEvidenceChannelRole::CoreRequired
                        | GeneralLineageEvidenceChannelRole::Required
                ) {
                    required_missing = true;
                }
            }
            GeneralLineageChannelDisposition::ContradictsSeparation => unreachable!(),
        }
    }

    let supporting: Vec<_> = support.iter().cloned().collect();
    let unavailable_potential: Vec<_> = unavailable
        .iter()
        .filter(|group| !support.contains(*group))
        .cloned()
        .collect();

    if required_missing {
        return Ok((supporting, unavailable_potential, GeneralLineageSpeciesStatus::InsufficientIndependentEvidence));
    }
    if required_not_supported {
        return Ok((supporting, unavailable_potential, GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel));
    }
    if support.len() >= design.minimum_independent_support_groups as usize {
        return Ok((supporting, unavailable_potential, GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel));
    }

    let potential_count = support
        .union(&unavailable)
        .count();
    if potential_count >= design.minimum_independent_support_groups as usize {
        Ok((supporting, unavailable_potential, GeneralLineageSpeciesStatus::InsufficientIndependentEvidence))
    } else {
        Ok((supporting, unavailable_potential, GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel))
    }
}

fn lineage_history_disposition(
    status: LineageDivergenceHistoryStatus,
) -> GeneralLineageChannelDisposition {
    match status {
        LineageDivergenceHistoryStatus::PersistentDivergenceObserved
        | LineageDivergenceHistoryStatus::DivergenceWithRecontact => {
            GeneralLineageChannelDisposition::SupportsSeparation
        }
        LineageDivergenceHistoryStatus::LineageFusionObserved
        | LineageDivergenceHistoryStatus::NotPersistent => {
            GeneralLineageChannelDisposition::ContradictsSeparation
        }
        LineageDivergenceHistoryStatus::InsufficientEvidence => {
            GeneralLineageChannelDisposition::Unavailable
        }
    }
}

fn reproductive_isolation_disposition(
    status: ReproductiveIsolationStatus,
) -> GeneralLineageChannelDisposition {
    match status {
        ReproductiveIsolationStatus::Supported => {
            GeneralLineageChannelDisposition::SupportsSeparation
        }
        ReproductiveIsolationStatus::NotSupported => {
            GeneralLineageChannelDisposition::DoesNotSupportSeparation
        }
        ReproductiveIsolationStatus::Contradicted => {
            GeneralLineageChannelDisposition::ContradictsSeparation
        }
        ReproductiveIsolationStatus::InsufficientEvidence => {
            GeneralLineageChannelDisposition::Unavailable
        }
    }
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), GeneralLineageEvidenceError> {
    if authority.revision == 0 {
        return Err(GeneralLineageEvidenceError::ZeroRevision(field));
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
pub enum GeneralLineageEvidenceError {
    ZeroRevision(&'static str),
    UnsupportedEvidenceVersion(u32),
    LineageHistoryDesignMismatch,
    ModelDigestMismatch,
    ModelApplicabilityBindingMismatch,
    DuplicateChannelInput(GeneralLineageEvidenceChannelId),
    MissingChannelInput(GeneralLineageEvidenceChannelId),
    UnexpectedChannelInput(GeneralLineageEvidenceChannelId),
    CoreChannelInputForbidden,
    ChannelInputIdMismatch,
    ChannelSourceKindMismatch,
    NativeIsolationAuthorityRequired,
    ReproductiveIsolationLineageMismatch,
    InvalidCoreChannel,
    LineageHistoryBindingMismatch,
    ChannelDispositionInvariant,
    MissingEvidenceFailClosed,
    DesignDigestMismatch,
    IncompleteChannelCoverage,
    ChannelDeclarationMismatch,
    DependencyGroupInvariant,
    StatusInvariant,
    ReplayMismatch,
    Design(GeneralLineageDesignError),
    Model(GeneralLineageModelError),
    LineageHistory(symtropy_evolution_core::LineageDivergenceHistoryError),
    ReproductiveIsolation(symtropy_evolution_core::ReproductiveIsolationEvidenceError),
}

impl From<GeneralLineageDesignError> for GeneralLineageEvidenceError {
    fn from(value: GeneralLineageDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<GeneralLineageModelError> for GeneralLineageEvidenceError {
    fn from(value: GeneralLineageModelError) -> Self {
        Self::Model(value)
    }
}

impl From<symtropy_evolution_core::LineageDivergenceHistoryError>
    for GeneralLineageEvidenceError
{
    fn from(value: symtropy_evolution_core::LineageDivergenceHistoryError) -> Self {
        Self::LineageHistory(value)
    }
}

impl From<symtropy_evolution_core::ReproductiveIsolationEvidenceError>
    for GeneralLineageEvidenceError
{
    fn from(value: symtropy_evolution_core::ReproductiveIsolationEvidenceError) -> Self {
        Self::ReproductiveIsolation(value)
    }
}

impl fmt::Display for GeneralLineageEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroRevision(field) => write!(f, "{field} must be nonzero"),
            Self::UnsupportedEvidenceVersion(version) => {
                write!(f, "unsupported general-lineage evidence version {version}")
            }
            Self::LineageHistoryDesignMismatch => write!(
                f,
                "current SEL-10A history does not match the preregistered target design"
            ),
            Self::ModelDigestMismatch => write!(
                f,
                "current general-lineage model does not match the preregistered model"
            ),
            Self::ModelApplicabilityBindingMismatch => write!(
                f,
                "model-applicability evidence does not bind the exact target/model/protocol"
            ),
            Self::DuplicateChannelInput(id) => {
                write!(f, "duplicate channel input {}", id.as_str())
            }
            Self::MissingChannelInput(id) => {
                write!(f, "missing preregistered channel input {}", id.as_str())
            }
            Self::UnexpectedChannelInput(id) => {
                write!(f, "unexpected channel input {}", id.as_str())
            }
            Self::CoreChannelInputForbidden => write!(
                f,
                "the core longitudinal channel is mechanically derived from current SEL-10A history"
            ),
            Self::ChannelInputIdMismatch => {
                write!(f, "channel input ID does not match its preregistered declaration")
            }
            Self::ChannelSourceKindMismatch => write!(
                f,
                "channel evidence source does not match the preregistered channel kind"
            ),
            Self::NativeIsolationAuthorityRequired => write!(
                f,
                "reproductive-isolation channels must consume current SEL-09B authority"
            ),
            Self::ReproductiveIsolationLineageMismatch => write!(
                f,
                "SEL-09B reproductive-isolation evidence targets a different lineage pair"
            ),
            Self::InvalidCoreChannel => write!(
                f,
                "general-lineage core channel is absent or not the reserved longitudinal channel"
            ),
            Self::LineageHistoryBindingMismatch => write!(
                f,
                "persisted core channel does not match its SEL-10A history snapshot/digest"
            ),
            Self::ChannelDispositionInvariant => write!(
                f,
                "persisted typed channel disposition does not match its native source authority"
            ),
            Self::MissingEvidenceFailClosed => write!(
                f,
                "unavailable evidence is forbidden by the preregistered fail-closed policy"
            ),
            Self::DesignDigestMismatch => write!(
                f,
                "embedded classification design does not match its persisted digest"
            ),
            Self::IncompleteChannelCoverage => write!(
                f,
                "persisted evidence does not cover every preregistered channel exactly once"
            ),
            Self::ChannelDeclarationMismatch => write!(
                f,
                "persisted channel record does not match its preregistered declaration"
            ),
            Self::DependencyGroupInvariant => write!(
                f,
                "persisted supporting/unavailable dependency groups do not recompute"
            ),
            Self::StatusInvariant => write!(
                f,
                "persisted general-lineage species status does not recompute"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted general-lineage evidence does not replay from current authorities"
            ),
            Self::Design(error) => write!(f, "general-lineage design error: {error}"),
            Self::Model(error) => write!(f, "general-lineage model error: {error}"),
            Self::LineageHistory(error) => write!(f, "SEL-10A history error: {error}"),
            Self::ReproductiveIsolation(error) => write!(f, "SEL-09B isolation error: {error}"),
        }
    }
}

impl Error for GeneralLineageEvidenceError {}
