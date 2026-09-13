use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AnalysisAuthorityRef, DemographicInterventionProofBundleDigest, HereditarySchema,
    LineageDivergenceHistoryDesign, LineageDivergenceHistoryDesignDigest,
    LineageHistoryContextPolicy, LineageHistoryEpisodeId, LineageHistoryMissingPolicy,
    PopulationGeneration, PopulationGeneticState, PopulationTrajectoryPoint,
    PopulationTrajectoryPointDigest, ValidatedLineageDivergenceHistoryDesign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const LINEAGE_DIVERGENCE_HISTORY_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:lineage-divergence-history:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedLineageHistoryEvidence {
    pub protocol: AnalysisAuthorityRef,
    pub evidence: AnalysisAuthorityRef,
}

impl QualifiedLineageHistoryEvidence {
    fn new(protocol: &AnalysisAuthorityRef, evidence: AnalysisAuthorityRef) -> Self {
        Self {
            protocol: protocol.clone(),
            evidence,
        }
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.protocol);
        put_authority(digest, &self.evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineagePointMembershipEvidence {
    pub lineage: AnalysisAuthorityRef,
    pub point_digest: PopulationTrajectoryPointDigest,
    pub evidence: QualifiedLineageHistoryEvidence,
}

impl LineagePointMembershipEvidence {
    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.lineage);
        digest.update(self.point_digest.as_bytes());
        self.evidence.put(digest);
    }
}

#[derive(Debug, Clone)]
pub enum LineagePersistenceInput {
    Persistent { evidence: AnalysisAuthorityRef },
    NotPersistent { evidence: AnalysisAuthorityRef },
    Unavailable { reason: AnalysisAuthorityRef },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineagePersistenceState {
    Persistent {
        evidence: QualifiedLineageHistoryEvidence,
    },
    NotPersistent {
        evidence: QualifiedLineageHistoryEvidence,
    },
    Unavailable {
        reason: QualifiedLineageHistoryEvidence,
    },
}

impl LineagePersistenceState {
    fn from_input(input: LineagePersistenceInput, protocol: &AnalysisAuthorityRef) -> Self {
        match input {
            LineagePersistenceInput::Persistent { evidence } => Self::Persistent {
                evidence: QualifiedLineageHistoryEvidence::new(protocol, evidence),
            },
            LineagePersistenceInput::NotPersistent { evidence } => Self::NotPersistent {
                evidence: QualifiedLineageHistoryEvidence::new(protocol, evidence),
            },
            LineagePersistenceInput::Unavailable { reason } => Self::Unavailable {
                reason: QualifiedLineageHistoryEvidence::new(protocol, reason),
            },
        }
    }

    fn protocol(&self) -> &AnalysisAuthorityRef {
        match self {
            Self::Persistent { evidence } | Self::NotPersistent { evidence } => &evidence.protocol,
            Self::Unavailable { reason } => &reason.protocol,
        }
    }

    fn not_persistent(&self) -> bool {
        matches!(self, Self::NotPersistent { .. })
    }

    fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }

    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::Persistent { evidence } => {
                digest.update([0]);
                evidence.put(digest);
            }
            Self::NotPersistent { evidence } => {
                digest.update([1]);
                evidence.put(digest);
            }
            Self::Unavailable { reason } => {
                digest.update([2]);
                reason.put(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineagePersistenceObservation {
    pub lineage: AnalysisAuthorityRef,
    pub state: LineagePersistenceState,
}

impl LineagePersistenceObservation {
    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.lineage);
        self.state.put(digest);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualifiedLineagePairEvidence {
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub evidence: QualifiedLineageHistoryEvidence,
}

impl QualifiedLineagePairEvidence {
    fn new(
        design: &LineageDivergenceHistoryDesign,
        protocol: &AnalysisAuthorityRef,
        evidence: AnalysisAuthorityRef,
    ) -> Self {
        Self {
            lineage_a: design.lineage_a.clone(),
            lineage_b: design.lineage_b.clone(),
            evidence: QualifiedLineageHistoryEvidence::new(protocol, evidence),
        }
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.lineage_a);
        put_authority(digest, &self.lineage_b);
        self.evidence.put(digest);
    }
}

#[derive(Debug, Clone)]
pub enum LineageObservationInput {
    NoneObserved { evidence: AnalysisAuthorityRef },
    Observed { evidence: AnalysisAuthorityRef },
    Unavailable { reason: AnalysisAuthorityRef },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageObservationState {
    NoneObserved {
        evidence: QualifiedLineageHistoryEvidence,
    },
    Observed {
        evidence: QualifiedLineageHistoryEvidence,
    },
    Unavailable {
        reason: QualifiedLineageHistoryEvidence,
    },
}

impl LineageObservationState {
    fn from_input(input: LineageObservationInput, protocol: &AnalysisAuthorityRef) -> Self {
        match input {
            LineageObservationInput::NoneObserved { evidence } => Self::NoneObserved {
                evidence: QualifiedLineageHistoryEvidence::new(protocol, evidence),
            },
            LineageObservationInput::Observed { evidence } => Self::Observed {
                evidence: QualifiedLineageHistoryEvidence::new(protocol, evidence),
            },
            LineageObservationInput::Unavailable { reason } => Self::Unavailable {
                reason: QualifiedLineageHistoryEvidence::new(protocol, reason),
            },
        }
    }

    fn protocol(&self) -> &AnalysisAuthorityRef {
        match self {
            Self::NoneObserved { evidence } | Self::Observed { evidence } => &evidence.protocol,
            Self::Unavailable { reason } => &reason.protocol,
        }
    }

    fn is_observed(&self) -> bool {
        matches!(self, Self::Observed { .. })
    }

    fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }

    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::NoneObserved { evidence } => {
                digest.update([0]);
                evidence.put(digest);
            }
            Self::Observed { evidence } => {
                digest.update([1]);
                evidence.put(digest);
            }
            Self::Unavailable { reason } => {
                digest.update([2]);
                reason.put(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineagePairObservationState {
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub state: LineageObservationState,
}

impl LineagePairObservationState {
    fn new(
        design: &LineageDivergenceHistoryDesign,
        input: LineageObservationInput,
        protocol: &AnalysisAuthorityRef,
    ) -> Self {
        Self {
            lineage_a: design.lineage_a.clone(),
            lineage_b: design.lineage_b.clone(),
            state: LineageObservationState::from_input(input, protocol),
        }
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.lineage_a);
        put_authority(digest, &self.lineage_b);
        self.state.put(digest);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageHistoryEpisodeKind {
    PopulationSplit,
    FounderOrRecolonization,
    AdmixtureOrIntrogression,
    Recontact,
    LineageFusionOrRemerger,
    Extinction,
    Other,
}

impl LineageHistoryEpisodeKind {
    fn tag(self) -> u8 {
        match self {
            Self::PopulationSplit => 0,
            Self::FounderOrRecolonization => 1,
            Self::AdmixtureOrIntrogression => 2,
            Self::Recontact => 3,
            Self::LineageFusionOrRemerger => 4,
            Self::Extinction => 5,
            Self::Other => 6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineageHistoryEpisodeInput {
    pub episode_id: LineageHistoryEpisodeId,
    pub kind: LineageHistoryEpisodeKind,
    pub proof_bundle_digest: Option<DemographicInterventionProofBundleDigest>,
    pub evidence: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageHistoryEpisodeObservation {
    pub episode_id: LineageHistoryEpisodeId,
    pub generation: PopulationGeneration,
    pub kind: LineageHistoryEpisodeKind,
    pub proof_bundle_digest: Option<DemographicInterventionProofBundleDigest>,
    pub evidence: QualifiedLineageHistoryEvidence,
}

impl LineageHistoryEpisodeObservation {
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.episode_id.as_str());
        put_u64(digest, self.generation.0);
        digest.update([self.kind.tag()]);
        match self.proof_bundle_digest {
            Some(bundle) => {
                digest.update([1]);
                digest.update(bundle.as_bytes());
            }
            None => digest.update([0]),
        }
        self.evidence.put(digest);
    }
}

#[derive(Debug)]
pub struct ObservedLineageGenerationInput<'a> {
    pub generation: PopulationGeneration,
    pub lineage_a_point: &'a PopulationTrajectoryPoint,
    pub lineage_a_schema: &'a HereditarySchema,
    pub lineage_a_population: &'a PopulationGeneticState,
    pub lineage_b_point: &'a PopulationTrajectoryPoint,
    pub lineage_b_schema: &'a HereditarySchema,
    pub lineage_b_population: &'a PopulationGeneticState,
    pub lineage_a_membership_evidence: AnalysisAuthorityRef,
    pub lineage_b_membership_evidence: AnalysisAuthorityRef,
    pub lineage_a_persistence: LineagePersistenceInput,
    pub lineage_b_persistence: LineagePersistenceInput,
    pub ancestry_relation_evidence: AnalysisAuthorityRef,
    pub context_evidence: AnalysisAuthorityRef,
    pub population_structure_evidence: AnalysisAuthorityRef,
    pub demographic_episode_census_evidence: AnalysisAuthorityRef,
    pub recontact: LineageObservationInput,
    pub gene_flow: LineageObservationInput,
    pub fusion: LineageObservationInput,
    pub episodes: Vec<LineageHistoryEpisodeInput>,
}

#[derive(Debug)]
pub enum LineageHistoryGenerationInput<'a> {
    Observed(ObservedLineageGenerationInput<'a>),
    Unavailable {
        generation: PopulationGeneration,
        reason: AnalysisAuthorityRef,
    },
}

impl<'a> LineageHistoryGenerationInput<'a> {
    fn generation(&self) -> PopulationGeneration {
        match self {
            Self::Observed(input) => input.generation,
            Self::Unavailable { generation, .. } => *generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedLineageGenerationRecord {
    pub generation: PopulationGeneration,
    pub lineage_a_point: PopulationTrajectoryPoint,
    pub lineage_b_point: PopulationTrajectoryPoint,
    pub lineage_a_membership: LineagePointMembershipEvidence,
    pub lineage_b_membership: LineagePointMembershipEvidence,
    pub lineage_a_persistence: LineagePersistenceObservation,
    pub lineage_b_persistence: LineagePersistenceObservation,
    pub ancestry_relation: QualifiedLineagePairEvidence,
    pub context: QualifiedLineageHistoryEvidence,
    pub population_structure: QualifiedLineageHistoryEvidence,
    pub demographic_episode_census: QualifiedLineageHistoryEvidence,
    pub recontact: LineagePairObservationState,
    pub gene_flow: LineagePairObservationState,
    pub fusion: LineagePairObservationState,
    pub episodes: Vec<LineageHistoryEpisodeObservation>,
}

impl ObservedLineageGenerationRecord {
    fn put(&self, digest: &mut Sha256) {
        put_u64(digest, self.generation.0);
        digest.update(self.lineage_a_point.canonical_digest().as_bytes());
        digest.update(self.lineage_b_point.canonical_digest().as_bytes());
        self.lineage_a_membership.put(digest);
        self.lineage_b_membership.put(digest);
        self.lineage_a_persistence.put(digest);
        self.lineage_b_persistence.put(digest);
        self.ancestry_relation.put(digest);
        self.context.put(digest);
        self.population_structure.put(digest);
        self.demographic_episode_census.put(digest);
        self.recontact.put(digest);
        self.gene_flow.put(digest);
        self.fusion.put(digest);
        put_u64(digest, self.episodes.len() as u64);
        for episode in &self.episodes {
            episode.put(digest);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageHistoryGenerationRecord {
    Observed(ObservedLineageGenerationRecord),
    Unavailable {
        generation: PopulationGeneration,
        reason: QualifiedLineageHistoryEvidence,
    },
}

impl LineageHistoryGenerationRecord {
    fn generation(&self) -> PopulationGeneration {
        match self {
            Self::Observed(record) => record.generation,
            Self::Unavailable { generation, .. } => *generation,
        }
    }

    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::Observed(record) => {
                digest.update([0]);
                record.put(digest);
            }
            Self::Unavailable { generation, reason } => {
                digest.update([1]);
                put_u64(digest, generation.0);
                reason.put(digest);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageDivergenceHistoryStatus {
    PersistentDivergenceObserved,
    DivergenceWithRecontact,
    LineageFusionObserved,
    NotPersistent,
    InsufficientEvidence,
}

impl LineageDivergenceHistoryStatus {
    fn tag(self) -> u8 {
        match self {
            Self::PersistentDivergenceObserved => 0,
            Self::DivergenceWithRecontact => 1,
            Self::LineageFusionObserved => 2,
            Self::NotPersistent => 3,
            Self::InsufficientEvidence => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageDivergenceHistory {
    history_version: u32,
    design: LineageDivergenceHistoryDesign,
    design_digest: LineageDivergenceHistoryDesignDigest,
    pub generations: Vec<LineageHistoryGenerationRecord>,
    pub status: LineageDivergenceHistoryStatus,
}

impl LineageDivergenceHistory {
    pub fn capture<'a>(
        design: &ValidatedLineageDivergenceHistoryDesign<'_>,
        inputs: impl IntoIterator<Item = LineageHistoryGenerationInput<'a>>,
    ) -> Result<Self, LineageDivergenceHistoryError> {
        let mut by_generation = BTreeMap::new();
        for input in inputs {
            let generation = input.generation();
            if by_generation.insert(generation, input).is_some() {
                return Err(LineageDivergenceHistoryError::DuplicateGeneration(
                    generation,
                ));
            }
        }
        if by_generation.len() != design.design().generation_count as usize {
            return Err(LineageDivergenceHistoryError::IncompleteGenerationCoverage);
        }

        let mut generations = Vec::with_capacity(by_generation.len());
        for raw in design.design().start_generation.0..=design.design().end_generation.0 {
            let generation = PopulationGeneration(raw);
            let input = by_generation
                .remove(&generation)
                .ok_or(LineageDivergenceHistoryError::MissingGeneration(generation))?;
            generations.push(materialize_generation(design.design(), input)?);
        }
        if !by_generation.is_empty() {
            return Err(LineageDivergenceHistoryError::UnexpectedGeneration);
        }

        let status = derive_status(&generations);
        let history = Self {
            history_version: LINEAGE_DIVERGENCE_HISTORY_VERSION,
            design: design.design().clone(),
            design_digest: design.design_digest(),
            generations,
            status,
        };
        history.validate_local()?;
        Ok(history)
    }

    pub fn design(&self) -> &LineageDivergenceHistoryDesign {
        &self.design
    }

    pub fn design_digest(&self) -> LineageDivergenceHistoryDesignDigest {
        self.design_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<LineageDivergenceHistoryDigest, LineageDivergenceHistoryError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.history_version);
        digest.update(self.design_digest.as_bytes());
        put_u64(&mut digest, self.generations.len() as u64);
        for generation in &self.generations {
            generation.put(&mut digest);
        }
        digest.update([self.status.tag()]);
        Ok(LineageDivergenceHistoryDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), LineageDivergenceHistoryError> {
        if self.history_version != LINEAGE_DIVERGENCE_HISTORY_VERSION {
            return Err(LineageDivergenceHistoryError::UnsupportedVersion(
                self.history_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(LineageDivergenceHistoryError::DesignBindingMismatch);
        }
        if self.generations.len() != self.design.generation_count as usize {
            return Err(LineageDivergenceHistoryError::IncompleteGenerationCoverage);
        }

        let mut exact_context: Option<&AnalysisAuthorityRef> = None;
        for (offset, record) in self.generations.iter().enumerate() {
            let expected = PopulationGeneration(
                self.design
                    .start_generation
                    .0
                    .checked_add(offset as u64)
                    .ok_or(LineageDivergenceHistoryError::ArithmeticOverflow)?,
            );
            if record.generation() != expected {
                return Err(LineageDivergenceHistoryError::NonCanonicalGenerationOrder);
            }
            match record {
                LineageHistoryGenerationRecord::Unavailable { reason, .. } => {
                    if self.design.missing_policy == LineageHistoryMissingPolicy::FailClosed {
                        return Err(LineageDivergenceHistoryError::UnavailableEvidenceForbidden);
                    }
                    if reason.protocol != self.design.completeness_authority {
                        return Err(LineageDivergenceHistoryError::ProtocolMismatch);
                    }
                }
                LineageHistoryGenerationRecord::Observed(observed) => {
                    validate_observed_local(&self.design, observed)?;
                    if matches!(
                        &self.design.context_policy,
                        LineageHistoryContextPolicy::ExactAcrossInterval
                    ) {
                        match exact_context {
                            None => exact_context = Some(&observed.context.evidence),
                            Some(existing) if existing == &observed.context.evidence => {}
                            Some(_) => {
                                return Err(LineageDivergenceHistoryError::ContextMismatch)
                            }
                        }
                    }
                }
            }
        }
        if derive_status(&self.generations) != self.status {
            return Err(LineageDivergenceHistoryError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageDivergenceHistoryDigest([u8; 32]);

impl LineageDivergenceHistoryDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LineageDivergenceHistoryDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LineageDivergenceHistoryDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LineageDivergenceHistoryDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated lineage-divergence history should gate any later species-model evaluation"]
pub struct ValidatedLineageDivergenceHistory<'a> {
    history: &'a LineageDivergenceHistory,
    history_digest: LineageDivergenceHistoryDigest,
    design_digest: LineageDivergenceHistoryDesignDigest,
}

impl<'a> ValidatedLineageDivergenceHistory<'a> {
    pub fn validate_current<'b>(
        history: &'a LineageDivergenceHistory,
        design: &ValidatedLineageDivergenceHistoryDesign<'_>,
        inputs: impl IntoIterator<Item = LineageHistoryGenerationInput<'b>>,
    ) -> Result<Self, LineageDivergenceHistoryError> {
        history.validate_local()?;
        let recomputed = LineageDivergenceHistory::capture(design, inputs)?;
        if recomputed != *history {
            return Err(LineageDivergenceHistoryError::ReplayMismatch);
        }
        Ok(Self {
            history,
            history_digest: history.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn history(&self) -> &'a LineageDivergenceHistory {
        self.history
    }

    pub fn history_digest(&self) -> LineageDivergenceHistoryDigest {
        self.history_digest
    }

    pub fn design_digest(&self) -> LineageDivergenceHistoryDesignDigest {
        self.design_digest
    }
}

fn materialize_generation(
    design: &LineageDivergenceHistoryDesign,
    input: LineageHistoryGenerationInput<'_>,
) -> Result<LineageHistoryGenerationRecord, LineageDivergenceHistoryError> {
    match input {
        LineageHistoryGenerationInput::Unavailable { generation, reason } => {
            if design.missing_policy == LineageHistoryMissingPolicy::FailClosed {
                return Err(LineageDivergenceHistoryError::UnavailableEvidenceForbidden);
            }
            Ok(LineageHistoryGenerationRecord::Unavailable {
                generation,
                reason: QualifiedLineageHistoryEvidence::new(
                    &design.completeness_authority,
                    reason,
                ),
            })
        }
        LineageHistoryGenerationInput::Observed(input) => {
            input
                .lineage_a_point
                .validate_current(input.lineage_a_schema, input.lineage_a_population)?;
            input
                .lineage_b_point
                .validate_current(input.lineage_b_schema, input.lineage_b_population)?;
            if input.lineage_a_point.generation() != input.generation
                || input.lineage_b_point.generation() != input.generation
            {
                return Err(LineageDivergenceHistoryError::TrajectoryGenerationMismatch);
            }

            let mut episodes_by_id = BTreeMap::new();
            for episode in input.episodes {
                if episodes_by_id
                    .insert(episode.episode_id.clone(), episode)
                    .is_some()
                {
                    return Err(LineageDivergenceHistoryError::DuplicateEpisodeId);
                }
            }
            let episodes = episodes_by_id
                .into_values()
                .map(|episode| LineageHistoryEpisodeObservation {
                    episode_id: episode.episode_id,
                    generation: input.generation,
                    kind: episode.kind,
                    proof_bundle_digest: episode.proof_bundle_digest,
                    evidence: QualifiedLineageHistoryEvidence::new(
                        &design.protocols.demographic_episode,
                        episode.evidence,
                    ),
                })
                .collect();

            let lineage_a_point = (*input.lineage_a_point).clone();
            let lineage_b_point = (*input.lineage_b_point).clone();
            let record = ObservedLineageGenerationRecord {
                generation: input.generation,
                lineage_a_membership: LineagePointMembershipEvidence {
                    lineage: design.lineage_a.clone(),
                    point_digest: lineage_a_point.canonical_digest(),
                    evidence: QualifiedLineageHistoryEvidence::new(
                        &design.protocols.lineage_membership,
                        input.lineage_a_membership_evidence,
                    ),
                },
                lineage_b_membership: LineagePointMembershipEvidence {
                    lineage: design.lineage_b.clone(),
                    point_digest: lineage_b_point.canonical_digest(),
                    evidence: QualifiedLineageHistoryEvidence::new(
                        &design.protocols.lineage_membership,
                        input.lineage_b_membership_evidence,
                    ),
                },
                lineage_a_persistence: LineagePersistenceObservation {
                    lineage: design.lineage_a.clone(),
                    state: LineagePersistenceState::from_input(
                        input.lineage_a_persistence,
                        &design.protocols.lineage_persistence,
                    ),
                },
                lineage_b_persistence: LineagePersistenceObservation {
                    lineage: design.lineage_b.clone(),
                    state: LineagePersistenceState::from_input(
                        input.lineage_b_persistence,
                        &design.protocols.lineage_persistence,
                    ),
                },
                ancestry_relation: QualifiedLineagePairEvidence::new(
                    design,
                    &design.protocols.ancestry_relation,
                    input.ancestry_relation_evidence,
                ),
                context: QualifiedLineageHistoryEvidence::new(
                    &design.protocols.context,
                    input.context_evidence,
                ),
                population_structure: QualifiedLineageHistoryEvidence::new(
                    &design.protocols.population_structure,
                    input.population_structure_evidence,
                ),
                demographic_episode_census: QualifiedLineageHistoryEvidence::new(
                    &design.protocols.demographic_episode_census,
                    input.demographic_episode_census_evidence,
                ),
                recontact: LineagePairObservationState::new(
                    design,
                    input.recontact,
                    &design.protocols.recontact,
                ),
                gene_flow: LineagePairObservationState::new(
                    design,
                    input.gene_flow,
                    &design.protocols.gene_flow,
                ),
                fusion: LineagePairObservationState::new(
                    design,
                    input.fusion,
                    &design.protocols.lineage_fusion,
                ),
                episodes,
                lineage_a_point,
                lineage_b_point,
            };
            validate_observed_local(design, &record)?;
            Ok(LineageHistoryGenerationRecord::Observed(record))
        }
    }
}

fn validate_observed_local(
    design: &LineageDivergenceHistoryDesign,
    record: &ObservedLineageGenerationRecord,
) -> Result<(), LineageDivergenceHistoryError> {
    if record.lineage_a_point.generation() != record.generation
        || record.lineage_b_point.generation() != record.generation
    {
        return Err(LineageDivergenceHistoryError::TrajectoryGenerationMismatch);
    }
    if record.lineage_a_membership.lineage != design.lineage_a
        || record.lineage_b_membership.lineage != design.lineage_b
        || record.lineage_a_membership.point_digest != record.lineage_a_point.canonical_digest()
        || record.lineage_b_membership.point_digest != record.lineage_b_point.canonical_digest()
        || record.lineage_a_persistence.lineage != design.lineage_a
        || record.lineage_b_persistence.lineage != design.lineage_b
    {
        return Err(LineageDivergenceHistoryError::LineageSubjectBindingMismatch);
    }
    if record.ancestry_relation.lineage_a != design.lineage_a
        || record.ancestry_relation.lineage_b != design.lineage_b
    {
        return Err(LineageDivergenceHistoryError::LineagePairBindingMismatch);
    }
    for pair_state in [&record.recontact, &record.gene_flow, &record.fusion] {
        if pair_state.lineage_a != design.lineage_a || pair_state.lineage_b != design.lineage_b {
            return Err(LineageDivergenceHistoryError::LineagePairBindingMismatch);
        }
    }

    for (actual, expected) in [
        (
            &record.lineage_a_membership.evidence.protocol,
            &design.protocols.lineage_membership,
        ),
        (
            &record.lineage_b_membership.evidence.protocol,
            &design.protocols.lineage_membership,
        ),
        (
            record.lineage_a_persistence.state.protocol(),
            &design.protocols.lineage_persistence,
        ),
        (
            record.lineage_b_persistence.state.protocol(),
            &design.protocols.lineage_persistence,
        ),
        (
            &record.ancestry_relation.evidence.protocol,
            &design.protocols.ancestry_relation,
        ),
        (&record.context.protocol, &design.protocols.context),
        (
            &record.population_structure.protocol,
            &design.protocols.population_structure,
        ),
        (
            &record.demographic_episode_census.protocol,
            &design.protocols.demographic_episode_census,
        ),
        (record.recontact.state.protocol(), &design.protocols.recontact),
        (record.gene_flow.state.protocol(), &design.protocols.gene_flow),
        (record.fusion.state.protocol(), &design.protocols.lineage_fusion),
    ] {
        if actual != expected {
            return Err(LineageDivergenceHistoryError::ProtocolMismatch);
        }
    }
    if design.missing_policy == LineageHistoryMissingPolicy::FailClosed
        && (record.lineage_a_persistence.state.is_unavailable()
            || record.lineage_b_persistence.state.is_unavailable()
            || record.recontact.state.is_unavailable()
            || record.gene_flow.state.is_unavailable()
            || record.fusion.state.is_unavailable())
    {
        return Err(LineageDivergenceHistoryError::UnavailableEvidenceForbidden);
    }
    for pair in record.episodes.windows(2) {
        if pair[0].episode_id >= pair[1].episode_id {
            return Err(LineageDivergenceHistoryError::NonCanonicalEpisodeOrder);
        }
    }
    for episode in &record.episodes {
        if episode.generation != record.generation
            || episode.evidence.protocol != design.protocols.demographic_episode
        {
            return Err(LineageDivergenceHistoryError::EpisodeBindingMismatch);
        }
    }
    Ok(())
}

fn derive_status(generations: &[LineageHistoryGenerationRecord]) -> LineageDivergenceHistoryStatus {
    let mut unavailable = false;
    let mut not_persistent = false;
    let mut recontact = false;
    let mut fusion = false;

    for generation in generations {
        match generation {
            LineageHistoryGenerationRecord::Unavailable { .. } => unavailable = true,
            LineageHistoryGenerationRecord::Observed(record) => {
                if record.fusion.state.is_observed() {
                    fusion = true;
                }
                if record.lineage_a_persistence.state.not_persistent()
                    || record.lineage_b_persistence.state.not_persistent()
                {
                    not_persistent = true;
                }
                if record.lineage_a_persistence.state.is_unavailable()
                    || record.lineage_b_persistence.state.is_unavailable()
                    || record.recontact.state.is_unavailable()
                    || record.gene_flow.state.is_unavailable()
                    || record.fusion.state.is_unavailable()
                {
                    unavailable = true;
                }
                if record.recontact.state.is_observed() || record.gene_flow.state.is_observed() {
                    recontact = true;
                }
            }
        }
    }

    if fusion {
        LineageDivergenceHistoryStatus::LineageFusionObserved
    } else if not_persistent {
        LineageDivergenceHistoryStatus::NotPersistent
    } else if unavailable {
        LineageDivergenceHistoryStatus::InsufficientEvidence
    } else if recontact {
        LineageDivergenceHistoryStatus::DivergenceWithRecontact
    } else {
        LineageDivergenceHistoryStatus::PersistentDivergenceObserved
    }
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum LineageDivergenceHistoryError {
    Design(crate::LineageDivergenceDesignError),
    Population(crate::EvolutionError),
    UnsupportedVersion(u32),
    DuplicateGeneration(PopulationGeneration),
    MissingGeneration(PopulationGeneration),
    UnexpectedGeneration,
    IncompleteGenerationCoverage,
    NonCanonicalGenerationOrder,
    TrajectoryGenerationMismatch,
    DuplicateEpisodeId,
    NonCanonicalEpisodeOrder,
    EpisodeBindingMismatch,
    ProtocolMismatch,
    ContextMismatch,
    LineageSubjectBindingMismatch,
    LineagePairBindingMismatch,
    UnavailableEvidenceForbidden,
    DesignBindingMismatch,
    StatusInvariant,
    ReplayMismatch,
    ArithmeticOverflow,
}

impl From<crate::LineageDivergenceDesignError> for LineageDivergenceHistoryError {
    fn from(value: crate::LineageDivergenceDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<crate::EvolutionError> for LineageDivergenceHistoryError {
    fn from(value: crate::EvolutionError) -> Self {
        Self::Population(value)
    }
}

impl fmt::Display for LineageDivergenceHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "lineage-divergence design error: {error}"),
            Self::Population(error) => {
                write!(f, "population trajectory authority error: {error}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported lineage-divergence history version {version}")
            }
            Self::DuplicateGeneration(generation) => {
                write!(f, "generation {} was supplied more than once", generation.0)
            }
            Self::MissingGeneration(generation) => {
                write!(f, "required generation {} is missing", generation.0)
            }
            Self::UnexpectedGeneration => write!(f, "an undeclared generation was supplied"),
            Self::IncompleteGenerationCoverage => {
                write!(f, "lineage history requires exact complete generation coverage")
            }
            Self::NonCanonicalGenerationOrder => write!(
                f,
                "lineage-history generations are not canonical and contiguous"
            ),
            Self::TrajectoryGenerationMismatch => write!(
                f,
                "population trajectory point does not match the history generation"
            ),
            Self::DuplicateEpisodeId => write!(
                f,
                "one demographic episode ID appears more than once in a generation"
            ),
            Self::NonCanonicalEpisodeOrder => {
                write!(f, "demographic episodes are not in canonical semantic-ID order")
            }
            Self::EpisodeBindingMismatch => {
                write!(f, "demographic episode does not bind its generation/protocol")
            }
            Self::ProtocolMismatch => {
                write!(f, "history evidence does not bind its preregistered protocol")
            }
            Self::ContextMismatch => {
                write!(f, "context evidence changed under ExactAcrossInterval policy")
            }
            Self::LineageSubjectBindingMismatch => write!(
                f,
                "lineage membership/persistence evidence binds the wrong lineage or trajectory point"
            ),
            Self::LineagePairBindingMismatch => {
                write!(f, "pair evidence binds a different ordered lineage pair")
            }
            Self::UnavailableEvidenceForbidden => write!(
                f,
                "unavailable required evidence is forbidden by the frozen missing-data policy"
            ),
            Self::DesignBindingMismatch => {
                write!(f, "persisted history binds a different lineage-history design")
            }
            Self::StatusInvariant => write!(
                f,
                "persisted lineage-history status does not recompute from the complete ledger"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted lineage-divergence history does not replay against current inputs"
            ),
            Self::ArithmeticOverflow => {
                write!(f, "lineage-divergence history arithmetic overflowed")
            }
        }
    }
}

impl Error for LineageDivergenceHistoryError {}
