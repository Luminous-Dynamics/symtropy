use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisAuthorityRef,
    AnalysisContentDigest, CalibrationAuthorityRef, ChannelSupportPolicy,
    DeathBeforeEndpointPolicy, EvidenceContentDigest, EvidenceProtocolContentDigest,
    EvidenceProtocolId, EvolutionIndividualId, EvolutionaryContextRefDigest,
    ExplicitConsequenceLedgerDigest, ExplicitLinkedPopulationCensusDigest,
    ExplicitSelectionEvidenceLedgerDigest, ExposureEvidenceSourceId, ExposureEvidenceStatus,
    LinkedIndividualManifestDigest, ObservationSupportDigest, PhenotypeEvidenceSourceId,
    PhenotypeEvidenceStatus, PopulationId, PredictorDefinitionRef, PredictorSourceKind,
    ProtocolComparabilityPolicy, SelectionComparisonDesign, SelectionComparisonDesignDigest,
    SelectionDesignClass, SelectionDesignError, SelectionEstimand, UncertaintyPlan,
    ValidatedConsequenceLedger, ValidatedSelectionComparisonDesign,
    ValidatedSelectionEvidenceLedger, ViabilityConsequence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const EXPLICIT_SELECTION_ANALYSIS_FRAME_VERSION: u32 = 1;
pub const SELECTION_ANALYSIS_FRAME_ROW_VERSION: u32 = 1;

const FRAME_DOMAIN: &[u8] = b"symtropy:evolution:explicit-selection-analysis-frame:v1\0";
const PREDICTOR_DEFINITION_BINDING_DOMAIN: &[u8] =
    b"symtropy:evolution:predictor-definition-binding:v1\0";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredictorDefinitionBindingDigest([u8; 32]);

impl PredictorDefinitionBindingDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PredictorDefinitionBindingDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PredictorDefinitionBindingDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PredictorDefinitionBindingDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

fn predictor_definition_digest(
    predictor: &PredictorDefinitionRef,
) -> PredictorDefinitionBindingDigest {
    let mut digest = Sha256::new();
    digest.update(PREDICTOR_DEFINITION_BINDING_DOMAIN);
    put_text(&mut digest, predictor.predictor_id.as_str());
    digest.update([match predictor.source_kind {
        PredictorSourceKind::Phenotype => 0,
        PredictorSourceKind::Exposure => 1,
        PredictorSourceKind::GenotypeOrLineage => 2,
        PredictorSourceKind::ExternalOpaque => 3,
    }]);
    put_u64(&mut digest, predictor.revision);
    digest.update(predictor.content_digest.as_bytes());
    PredictorDefinitionBindingDigest(digest.finalize().into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceWindowBinding {
    Partial {
        support_digest: ObservationSupportDigest,
    },
    Complete,
}

impl EvidenceWindowBinding {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Partial { support_digest } => {
                digest.update([0]);
                digest.update(support_digest.as_bytes());
            }
            Self::Complete => digest.update([1]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictorSourceBinding {
    PhenotypeUnavailable,
    Phenotype {
        source_id: PhenotypeEvidenceSourceId,
        revision: u64,
        content_digest: EvidenceContentDigest,
        protocol_id: EvidenceProtocolId,
        protocol_content_digest: EvidenceProtocolContentDigest,
        window: EvidenceWindowBinding,
    },
    ExposureUnavailable,
    Exposure {
        source_id: ExposureEvidenceSourceId,
        revision: u64,
        content_digest: EvidenceContentDigest,
        protocol_id: EvidenceProtocolId,
        protocol_content_digest: EvidenceProtocolContentDigest,
        window: EvidenceWindowBinding,
    },
    HereditaryManifest {
        manifest_digest: LinkedIndividualManifestDigest,
    },
    ExternalOpaque {
        authority: AnalysisAuthorityRef,
    },
}

impl PredictorSourceBinding {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::PhenotypeUnavailable => digest.update([0]),
            Self::Phenotype {
                source_id,
                revision,
                content_digest,
                protocol_id,
                protocol_content_digest,
                window,
            } => {
                digest.update([1]);
                put_text(digest, source_id.as_str());
                put_u64(digest, *revision);
                digest.update(content_digest.as_bytes());
                put_text(digest, protocol_id.as_str());
                digest.update(protocol_content_digest.as_bytes());
                window.update_digest(digest);
            }
            Self::ExposureUnavailable => digest.update([2]),
            Self::Exposure {
                source_id,
                revision,
                content_digest,
                protocol_id,
                protocol_content_digest,
                window,
            } => {
                digest.update([3]);
                put_text(digest, source_id.as_str());
                put_u64(digest, *revision);
                digest.update(content_digest.as_bytes());
                put_text(digest, protocol_id.as_str());
                digest.update(protocol_content_digest.as_bytes());
                window.update_digest(digest);
            }
            Self::HereditaryManifest { manifest_digest } => {
                digest.update([4]);
                digest.update(manifest_digest.as_bytes());
            }
            Self::ExternalOpaque { authority } => {
                digest.update([5]);
                update_analysis_authority_digest(digest, authority);
            }
        }
    }

    fn is_unavailable(&self) -> bool {
        matches!(
            self,
            Self::PhenotypeUnavailable | Self::ExposureUnavailable
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryComparisonGroup {
    Reference,
    Comparison,
}

impl BinaryComparisonGroup {
    fn tag(self) -> u8 {
        match self {
            Self::Reference => 0,
            Self::Comparison => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictorRepresentation {
    BinaryComparisonGroup,
    Categorical,
    SignedFixedPoint {
        scale: u64,
        encoding: AnalysisAuthorityRef,
    },
}

impl PredictorRepresentation {
    fn validate(&self) -> Result<(), SelectionAnalysisFrameError> {
        if matches!(self, Self::SignedFixedPoint { scale: 0, .. }) {
            return Err(SelectionAnalysisFrameError::ZeroFixedPointScale);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::BinaryComparisonGroup => digest.update([0]),
            Self::Categorical => digest.update([1]),
            Self::SignedFixedPoint { scale, encoding } => {
                digest.update([2]);
                put_u64(digest, *scale);
                update_analysis_authority_digest(digest, encoding);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializedPredictorValue {
    Binary(BinaryComparisonGroup),
    Categorical(AnalysisAuthorityRef),
    SignedFixedPoint(i64),
}

impl MaterializedPredictorValue {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Binary(group) => digest.update([0, group.tag()]),
            Self::Categorical(level) => {
                digest.update([1]);
                update_analysis_authority_digest(digest, level);
            }
            Self::SignedFixedPoint(value) => {
                digest.update([2]);
                digest.update(value.to_le_bytes());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictorMaterializationStatus {
    Observed(MaterializedPredictorValue),
    Missing { reason: AnalysisAuthorityRef },
}

impl PredictorMaterializationStatus {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Observed(value) => {
                digest.update([0]);
                value.update_digest(digest);
            }
            Self::Missing { reason } => {
                digest.update([1]);
                update_analysis_authority_digest(digest, reason);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictorCalibrationProvenance {
    pub authority: CalibrationAuthorityRef,
    pub transformed_value_digest: AnalysisContentDigest,
}

impl PredictorCalibrationProvenance {
    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority.authority_id.as_str());
        put_u64(digest, self.authority.revision);
        digest.update(self.authority.content_digest.as_bytes());
        digest.update(self.transformed_value_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictorMaterialization {
    pub source: PredictorSourceBinding,
    pub status: PredictorMaterializationStatus,
    pub calibration: Option<PredictorCalibrationProvenance>,
}

impl PredictorMaterialization {
    fn update_digest(&self, digest: &mut Sha256) {
        self.source.update_digest(digest);
        self.status.update_digest(digest);
        match &self.calibration {
            None => digest.update([0]),
            Some(calibration) => {
                digest.update([1]);
                calibration.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictorMaterializationInput {
    pub individual_id: EvolutionIndividualId,
    pub materialization: PredictorMaterialization,
}

impl PredictorMaterializationInput {
    pub fn new(
        individual_id: EvolutionIndividualId,
        materialization: PredictorMaterialization,
    ) -> Self {
        Self {
            individual_id,
            materialization,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializedOutcome {
    ViabilityObserved(ViabilityConsequence),
    ReproductiveEventsObserved {
        opportunities_observed: u64,
        realized_events: u64,
    },
    DescendantProductionObserved { produced: u64 },
    DescendantRecruitmentObserved { recruited: u64 },
    Unavailable,
    EndpointFailureByObservedDeath,
    CensoredByObservedDeath,
    CompetingRiskDeath,
}

impl MaterializedOutcome {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::ViabilityObserved(ViabilityConsequence::SurvivedWindow) => {
                digest.update([0, 0])
            }
            Self::ViabilityObserved(ViabilityConsequence::DiedDuringWindow) => {
                digest.update([0, 1])
            }
            Self::ReproductiveEventsObserved {
                opportunities_observed,
                realized_events,
            } => {
                digest.update([1]);
                put_u64(digest, *opportunities_observed);
                put_u64(digest, *realized_events);
            }
            Self::DescendantProductionObserved { produced } => {
                digest.update([2]);
                put_u64(digest, *produced);
            }
            Self::DescendantRecruitmentObserved { recruited } => {
                digest.update([3]);
                put_u64(digest, *recruited);
            }
            Self::Unavailable => digest.update([4]),
            Self::EndpointFailureByObservedDeath => digest.update([5]),
            Self::CensoredByObservedDeath => digest.update([6]),
            Self::CompetingRiskDeath => digest.update([7]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionAnalysisFrameRow {
    row_version: u32,
    pub individual_id: EvolutionIndividualId,
    pub predictor_definition_digest: PredictorDefinitionBindingDigest,
    pub individual_manifest_digest: LinkedIndividualManifestDigest,
    pub consequence_observation_digest: crate::IndividualConsequenceObservationDigest,
    pub predictor: PredictorMaterialization,
    pub outcome: MaterializedOutcome,
}

impl SelectionAnalysisFrameRow {
    fn validate_local(
        &self,
        expected_predictor_digest: PredictorDefinitionBindingDigest,
        representation: &PredictorRepresentation,
    ) -> Result<(), SelectionAnalysisFrameError> {
        if self.row_version != SELECTION_ANALYSIS_FRAME_ROW_VERSION {
            return Err(SelectionAnalysisFrameError::UnsupportedRowVersion(
                self.row_version,
            ));
        }
        if self.predictor_definition_digest != expected_predictor_digest {
            return Err(SelectionAnalysisFrameError::PredictorDefinitionMismatch(
                self.individual_id.clone(),
            ));
        }
        validate_predictor_representation(&self.predictor.status, representation)?;
        if self.predictor.source.is_unavailable()
            && matches!(
                &self.predictor.status,
                PredictorMaterializationStatus::Observed(_)
            )
        {
            return Err(SelectionAnalysisFrameError::ObservedValueFromUnavailableSource(
                self.individual_id.clone(),
            ));
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_u32(digest, self.row_version);
        put_text(digest, self.individual_id.as_str());
        digest.update(self.predictor_definition_digest.as_bytes());
        digest.update(self.individual_manifest_digest.as_bytes());
        digest.update(self.consequence_observation_digest.as_bytes());
        self.predictor.update_digest(digest);
        self.outcome.update_digest(digest);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitSelectionAnalysisFrame {
    frame_version: u32,
    pub design: SelectionComparisonDesign,
    selection_evidence_ledger_digest: ExplicitSelectionEvidenceLedgerDigest,
    consequence_ledger_digest: ExplicitConsequenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
    original_denominator: u64,
    estimand_denominator: u64,
    pub materialization_authority: AnalysisAuthorityRef,
    pub predictor_representation: PredictorRepresentation,
    pub rows: Vec<SelectionAnalysisFrameRow>,
}

impl ExplicitSelectionAnalysisFrame {
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        validated_design: &ValidatedSelectionComparisonDesign<'_>,
        validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
        validated_consequences: &ValidatedConsequenceLedger<'_>,
        materialization_authority: AnalysisAuthorityRef,
        predictor_representation: PredictorRepresentation,
        predictor_inputs: impl IntoIterator<Item = PredictorMaterializationInput>,
    ) -> Result<Self, SelectionAnalysisFrameError> {
        predictor_representation.validate()?;
        validated_design
            .design()
            .validate_current(validated_evidence)
            .map_err(SelectionAnalysisFrameError::Design)?;
        validate_capability_chain(validated_design, validated_evidence, validated_consequences)?;

        let design = validated_design.design();
        validate_predictor_channel_requirement(design)?;

        let mut inputs = BTreeMap::new();
        for input in predictor_inputs {
            if !design.included_individuals.contains(&input.individual_id) {
                return Err(SelectionAnalysisFrameError::UnknownOrExcludedInput(
                    input.individual_id,
                ));
            }
            let id = input.individual_id.clone();
            if inputs.insert(id.clone(), input).is_some() {
                return Err(SelectionAnalysisFrameError::DuplicatePredictorInput(id));
            }
        }
        if inputs.len() != design.included_individuals.len() {
            return Err(SelectionAnalysisFrameError::IncompleteIncludedCoverage);
        }

        let evidence_records: BTreeMap<_, _> = validated_evidence
            .ledger()
            .records
            .iter()
            .map(|record| (record.individual_id.clone(), record))
            .collect();
        let consequence_observations: BTreeMap<_, _> = validated_consequences
            .ledger()
            .observations
            .iter()
            .map(|observation| (observation.individual_id.clone(), observation))
            .collect();
        let predictor_digest = predictor_definition_digest(&design.predictor);

        let mut rows = Vec::with_capacity(design.included_individuals.len());
        for individual_id in &design.included_individuals {
            let input = inputs
                .remove(individual_id)
                .ok_or(SelectionAnalysisFrameError::IncompleteIncludedCoverage)?;
            let evidence = evidence_records.get(individual_id).ok_or_else(|| {
                SelectionAnalysisFrameError::MissingEvidenceRecord(individual_id.clone())
            })?;
            let observation = consequence_observations.get(individual_id).ok_or_else(|| {
                SelectionAnalysisFrameError::MissingConsequenceObservation(individual_id.clone())
            })?;

            let observation_digest = observation
                .canonical_digest()
                .map_err(SelectionAnalysisFrameError::Consequence)?;
            if evidence.consequence_observation_digest() != observation_digest {
                return Err(
                    SelectionAnalysisFrameError::ConsequenceObservationBindingMismatch(
                        individual_id.clone(),
                    ),
                );
            }

            validate_predictor_materialization(
                design,
                evidence,
                &input.materialization,
                &predictor_representation,
            )?;
            let outcome = materialize_outcome(
                design.estimand,
                design.death_before_endpoint,
                &observation.consequences,
            )?;

            rows.push(SelectionAnalysisFrameRow {
                row_version: SELECTION_ANALYSIS_FRAME_ROW_VERSION,
                individual_id: individual_id.clone(),
                predictor_definition_digest: predictor_digest,
                individual_manifest_digest: evidence.individual_manifest_digest(),
                consequence_observation_digest: observation_digest,
                predictor: input.materialization,
                outcome,
            });
        }
        if !inputs.is_empty() {
            return Err(SelectionAnalysisFrameError::IncompleteIncludedCoverage);
        }

        let frame = Self {
            frame_version: EXPLICIT_SELECTION_ANALYSIS_FRAME_VERSION,
            design: design.clone(),
            selection_evidence_ledger_digest: validated_evidence.ledger_digest(),
            consequence_ledger_digest: validated_consequences.ledger_digest(),
            population_id: validated_design.population_id().clone(),
            census_digest: validated_design.census_digest(),
            context_digest: validated_design.context_digest(),
            original_denominator: design.original_denominator(),
            estimand_denominator: design.estimand_denominator(),
            materialization_authority,
            predictor_representation,
            rows,
        };
        frame.validate_local()?;
        Ok(frame)
    }

    pub fn validate_current(
        &self,
        validated_design: &ValidatedSelectionComparisonDesign<'_>,
        validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
        validated_consequences: &ValidatedConsequenceLedger<'_>,
        materialization_authority: AnalysisAuthorityRef,
        predictor_representation: PredictorRepresentation,
        predictor_inputs: impl IntoIterator<Item = PredictorMaterializationInput>,
    ) -> Result<(), SelectionAnalysisFrameError> {
        self.validate_local()?;
        let recomputed = Self::capture(
            validated_design,
            validated_evidence,
            validated_consequences,
            materialization_authority,
            predictor_representation,
            predictor_inputs,
        )?;
        if recomputed != *self {
            return Err(SelectionAnalysisFrameError::FrameReplayMismatch);
        }
        Ok(())
    }

    pub fn original_denominator(&self) -> u64 {
        self.original_denominator
    }

    pub fn estimand_denominator(&self) -> u64 {
        self.estimand_denominator
    }

    pub fn selection_evidence_ledger_digest(&self) -> ExplicitSelectionEvidenceLedgerDigest {
        self.selection_evidence_ledger_digest
    }

    pub fn consequence_ledger_digest(&self) -> ExplicitConsequenceLedgerDigest {
        self.consequence_ledger_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ExplicitSelectionAnalysisFrameDigest, SelectionAnalysisFrameError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(FRAME_DOMAIN);
        put_u32(&mut digest, self.frame_version);
        digest.update(
            self.design
                .canonical_digest()
                .map_err(SelectionAnalysisFrameError::Design)?
                .as_bytes(),
        );
        digest.update(self.selection_evidence_ledger_digest.as_bytes());
        digest.update(self.consequence_ledger_digest.as_bytes());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.census_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
        put_u64(&mut digest, self.original_denominator);
        put_u64(&mut digest, self.estimand_denominator);
        update_analysis_authority_digest(&mut digest, &self.materialization_authority);
        self.predictor_representation.update_digest(&mut digest);
        put_u64(&mut digest, self.rows.len() as u64);
        for row in &self.rows {
            row.update_digest(&mut digest);
        }
        Ok(ExplicitSelectionAnalysisFrameDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), SelectionAnalysisFrameError> {
        if self.frame_version != EXPLICIT_SELECTION_ANALYSIS_FRAME_VERSION {
            return Err(SelectionAnalysisFrameError::UnsupportedFrameVersion(
                self.frame_version,
            ));
        }
        self.predictor_representation.validate()?;
        self.design
            .canonical_digest()
            .map_err(SelectionAnalysisFrameError::Design)?;
        let expected_predictor_digest = predictor_definition_digest(&self.design.predictor);
        if self.selection_evidence_ledger_digest != self.design.selection_evidence_ledger_digest() {
            return Err(SelectionAnalysisFrameError::SelectionEvidenceBindingMismatch);
        }
        if self.original_denominator != self.design.original_denominator()
            || self.estimand_denominator != self.design.estimand_denominator()
            || self.rows.len() as u64 != self.estimand_denominator
        {
            return Err(SelectionAnalysisFrameError::DenominatorMismatch);
        }
        if self.rows.is_empty() {
            return Err(SelectionAnalysisFrameError::IncompleteIncludedCoverage);
        }
        if self
            .rows
            .windows(2)
            .any(|window| window[0].individual_id >= window[1].individual_id)
        {
            return Err(SelectionAnalysisFrameError::NonCanonicalRowOrder);
        }
        if self
            .rows
            .iter()
            .map(|row| &row.individual_id)
            .ne(self.design.included_individuals.iter())
        {
            return Err(SelectionAnalysisFrameError::IncludedPopulationMismatch);
        }
        for row in &self.rows {
            row.validate_local(expected_predictor_digest, &self.predictor_representation)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExplicitSelectionAnalysisFrameDigest([u8; 32]);

impl ExplicitSelectionAnalysisFrameDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ExplicitSelectionAnalysisFrameDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExplicitSelectionAnalysisFrameDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ExplicitSelectionAnalysisFrameDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated analysis-frame authority should be consumed by analysis execution APIs"]
pub struct ValidatedSelectionAnalysisFrame<'a> {
    frame: &'a ExplicitSelectionAnalysisFrame,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    design_digest: SelectionComparisonDesignDigest,
    selection_evidence_ledger_digest: ExplicitSelectionEvidenceLedgerDigest,
    consequence_ledger_digest: ExplicitConsequenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
}

impl<'a> ValidatedSelectionAnalysisFrame<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        frame: &'a ExplicitSelectionAnalysisFrame,
        validated_design: &ValidatedSelectionComparisonDesign<'_>,
        validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
        validated_consequences: &ValidatedConsequenceLedger<'_>,
        materialization_authority: AnalysisAuthorityRef,
        predictor_representation: PredictorRepresentation,
        predictor_inputs: impl IntoIterator<Item = PredictorMaterializationInput>,
    ) -> Result<Self, SelectionAnalysisFrameError> {
        frame.validate_current(
            validated_design,
            validated_evidence,
            validated_consequences,
            materialization_authority,
            predictor_representation,
            predictor_inputs,
        )?;
        Ok(Self {
            frame,
            frame_digest: frame.canonical_digest()?,
            design_digest: validated_design.design_digest(),
            selection_evidence_ledger_digest: validated_evidence.ledger_digest(),
            consequence_ledger_digest: validated_consequences.ledger_digest(),
            population_id: validated_design.population_id().clone(),
            census_digest: validated_design.census_digest(),
            context_digest: validated_design.context_digest(),
        })
    }

    pub fn frame(&self) -> &'a ExplicitSelectionAnalysisFrame {
        self.frame
    }

    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn design_digest(&self) -> SelectionComparisonDesignDigest {
        self.design_digest
    }

    pub fn selection_evidence_ledger_digest(&self) -> ExplicitSelectionEvidenceLedgerDigest {
        self.selection_evidence_ledger_digest
    }

    pub fn consequence_ledger_digest(&self) -> ExplicitConsequenceLedgerDigest {
        self.consequence_ledger_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn census_digest(&self) -> ExplicitLinkedPopulationCensusDigest {
        self.census_digest
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }

    pub fn design(&self) -> &SelectionComparisonDesign {
        &self.frame.design
    }

    pub fn estimand(&self) -> SelectionEstimand {
        self.frame.design.estimand
    }

    pub fn design_class(&self) -> SelectionDesignClass {
        self.frame.design.design_class
    }

    pub fn uncertainty(&self) -> &UncertaintyPlan {
        &self.frame.design.uncertainty
    }
}

fn validate_capability_chain(
    validated_design: &ValidatedSelectionComparisonDesign<'_>,
    validated_evidence: &ValidatedSelectionEvidenceLedger<'_>,
    validated_consequences: &ValidatedConsequenceLedger<'_>,
) -> Result<(), SelectionAnalysisFrameError> {
    if validated_design.selection_evidence_ledger_digest() != validated_evidence.ledger_digest() {
        return Err(SelectionAnalysisFrameError::SelectionEvidenceBindingMismatch);
    }
    if validated_evidence.consequence_ledger_digest() != validated_consequences.ledger_digest() {
        return Err(SelectionAnalysisFrameError::ConsequenceLedgerBindingMismatch);
    }
    if validated_design.population_id() != validated_evidence.population_id()
        || validated_design.population_id() != validated_consequences.population_id()
        || validated_design.census_digest() != validated_evidence.census_digest()
        || validated_design.census_digest() != validated_consequences.census_digest()
        || validated_design.context_digest() != validated_evidence.context_digest()
        || validated_design.context_digest() != validated_consequences.context_digest()
    {
        return Err(SelectionAnalysisFrameError::AuthorityContextMismatch);
    }
    Ok(())
}

fn validate_predictor_channel_requirement(
    design: &SelectionComparisonDesign,
) -> Result<(), SelectionAnalysisFrameError> {
    match design.predictor.source_kind {
        PredictorSourceKind::Phenotype
            if matches!(
                &design.phenotype_support,
                ChannelSupportPolicy::NotRequired
            ) =>
        {
            Err(SelectionAnalysisFrameError::PredictorChannelNotRequired)
        }
        PredictorSourceKind::Exposure
            if matches!(&design.exposure_support, ChannelSupportPolicy::NotRequired) =>
        {
            Err(SelectionAnalysisFrameError::PredictorChannelNotRequired)
        }
        _ => Ok(()),
    }
}

fn validate_predictor_materialization(
    design: &SelectionComparisonDesign,
    evidence: &crate::SelectionEvidenceRecord,
    materialization: &PredictorMaterialization,
    representation: &PredictorRepresentation,
) -> Result<(), SelectionAnalysisFrameError> {
    validate_predictor_representation(&materialization.status, representation)?;
    if design.predictor.source_kind == PredictorSourceKind::ExternalOpaque {
        if !matches!(
            &materialization.source,
            PredictorSourceBinding::ExternalOpaque { .. }
        ) {
            return Err(SelectionAnalysisFrameError::PredictorSourceBindingMismatch(
                evidence.individual_id.clone(),
            ));
        }
    } else {
        let expected_source = expected_source_binding(design.predictor.source_kind, evidence);
        if materialization.source != expected_source {
            return Err(SelectionAnalysisFrameError::PredictorSourceBindingMismatch(
                evidence.individual_id.clone(),
            ));
        }
    }
    if materialization.source.is_unavailable()
        && matches!(
            &materialization.status,
            PredictorMaterializationStatus::Observed(_)
        )
    {
        return Err(SelectionAnalysisFrameError::ObservedValueFromUnavailableSource(
            evidence.individual_id.clone(),
        ));
    }
    validate_calibration_provenance(design, materialization, &evidence.individual_id)
}

fn expected_source_binding(
    source_kind: PredictorSourceKind,
    evidence: &crate::SelectionEvidenceRecord,
) -> PredictorSourceBinding {
    match source_kind {
        PredictorSourceKind::Phenotype => match &evidence.phenotype {
            PhenotypeEvidenceStatus::Unavailable => PredictorSourceBinding::PhenotypeUnavailable,
            PhenotypeEvidenceStatus::PartialWindow {
                evidence,
                support_digest,
            } => PredictorSourceBinding::Phenotype {
                source_id: evidence.source_id.clone(),
                revision: evidence.revision,
                content_digest: evidence.content_digest,
                protocol_id: evidence.protocol_id.clone(),
                protocol_content_digest: evidence.protocol_content_digest,
                window: EvidenceWindowBinding::Partial {
                    support_digest: *support_digest,
                },
            },
            PhenotypeEvidenceStatus::CompleteWindow(evidence) => {
                PredictorSourceBinding::Phenotype {
                    source_id: evidence.source_id.clone(),
                    revision: evidence.revision,
                    content_digest: evidence.content_digest,
                    protocol_id: evidence.protocol_id.clone(),
                    protocol_content_digest: evidence.protocol_content_digest,
                    window: EvidenceWindowBinding::Complete,
                }
            }
        },
        PredictorSourceKind::Exposure => match &evidence.exposure {
            ExposureEvidenceStatus::Unavailable => PredictorSourceBinding::ExposureUnavailable,
            ExposureEvidenceStatus::PartialWindow {
                evidence,
                support_digest,
            } => PredictorSourceBinding::Exposure {
                source_id: evidence.source_id.clone(),
                revision: evidence.revision,
                content_digest: evidence.content_digest,
                protocol_id: evidence.protocol_id.clone(),
                protocol_content_digest: evidence.protocol_content_digest,
                window: EvidenceWindowBinding::Partial {
                    support_digest: *support_digest,
                },
            },
            ExposureEvidenceStatus::CompleteWindow(evidence) => PredictorSourceBinding::Exposure {
                source_id: evidence.source_id.clone(),
                revision: evidence.revision,
                content_digest: evidence.content_digest,
                protocol_id: evidence.protocol_id.clone(),
                protocol_content_digest: evidence.protocol_content_digest,
                window: EvidenceWindowBinding::Complete,
            },
        },
        PredictorSourceKind::GenotypeOrLineage => PredictorSourceBinding::HereditaryManifest {
            manifest_digest: evidence.individual_manifest_digest(),
        },
        PredictorSourceKind::ExternalOpaque => {
            unreachable!("external predictor bindings are validated as explicit opaque authorities")
        }
    }
}

fn validate_calibration_provenance(
    design: &SelectionComparisonDesign,
    materialization: &PredictorMaterialization,
    individual_id: &EvolutionIndividualId,
) -> Result<(), SelectionAnalysisFrameError> {
    let policy = match design.predictor.source_kind {
        PredictorSourceKind::Phenotype => Some(&design.phenotype_protocols),
        PredictorSourceKind::Exposure => Some(&design.exposure_protocols),
        PredictorSourceKind::GenotypeOrLineage | PredictorSourceKind::ExternalOpaque => None,
    };

    match (policy, &materialization.status, &materialization.calibration) {
        (
            Some(ProtocolComparabilityPolicy::Calibrated { authority }),
            PredictorMaterializationStatus::Observed(_),
            Some(provenance),
        ) if &provenance.authority == authority => Ok(()),
        (
            Some(ProtocolComparabilityPolicy::Calibrated { .. }),
            PredictorMaterializationStatus::Observed(_),
            _,
        ) => Err(SelectionAnalysisFrameError::CalibrationProvenanceMismatch(
            individual_id.clone(),
        )),
        (Some(ProtocolComparabilityPolicy::ExactIdentityOnly), _, Some(_)) => {
            Err(SelectionAnalysisFrameError::UnexpectedCalibrationProvenance(
                individual_id.clone(),
            ))
        }
        (None, _, Some(_)) => Err(SelectionAnalysisFrameError::UnexpectedCalibrationProvenance(
            individual_id.clone(),
        )),
        (_, PredictorMaterializationStatus::Missing { .. }, Some(_)) => {
            Err(SelectionAnalysisFrameError::UnexpectedCalibrationProvenance(
                individual_id.clone(),
            ))
        }
        (_, PredictorMaterializationStatus::Missing { .. }, None) => Ok(()),
        (Some(ProtocolComparabilityPolicy::ExactIdentityOnly), _, None) | (None, _, None) => Ok(()),
    }
}

fn validate_predictor_representation(
    status: &PredictorMaterializationStatus,
    representation: &PredictorRepresentation,
) -> Result<(), SelectionAnalysisFrameError> {
    representation.validate()?;
    let PredictorMaterializationStatus::Observed(value) = status else {
        return Ok(());
    };
    let matches = matches!(
        (representation, value),
        (
            PredictorRepresentation::BinaryComparisonGroup,
            MaterializedPredictorValue::Binary(_)
        ) | (
            PredictorRepresentation::Categorical,
            MaterializedPredictorValue::Categorical(_)
        ) | (
            PredictorRepresentation::SignedFixedPoint { .. },
            MaterializedPredictorValue::SignedFixedPoint(_)
        )
    );
    if matches {
        Ok(())
    } else {
        Err(SelectionAnalysisFrameError::PredictorRepresentationMismatch)
    }
}

fn materialize_outcome(
    estimand: SelectionEstimand,
    death_policy: DeathBeforeEndpointPolicy,
    consequences: &crate::IndividualConsequences,
) -> Result<MaterializedOutcome, SelectionAnalysisFrameError> {
    match estimand {
        SelectionEstimand::ViabilityWindowRiskContrast => Ok(match consequences.viability {
            Some(value) => MaterializedOutcome::ViabilityObserved(value),
            None => MaterializedOutcome::Unavailable,
        }),
        SelectionEstimand::ReproductiveEventOpportunityConditionedContrast => {
            if let Some(value) = &consequences.reproductive_events {
                Ok(MaterializedOutcome::ReproductiveEventsObserved {
                    opportunities_observed: value.opportunities_observed,
                    realized_events: value.realized_events,
                })
            } else {
                materialize_missing_later_endpoint(death_policy, consequences.viability)
            }
        }
        SelectionEstimand::DescendantProductionContrast => {
            if let Some(value) = &consequences.descendant_production {
                Ok(MaterializedOutcome::DescendantProductionObserved {
                    produced: value.produced,
                })
            } else {
                materialize_missing_later_endpoint(death_policy, consequences.viability)
            }
        }
        SelectionEstimand::DescendantRecruitmentContrast => {
            if let Some(value) = &consequences.descendant_recruitment {
                Ok(MaterializedOutcome::DescendantRecruitmentObserved {
                    recruited: value.recruited,
                })
            } else {
                materialize_missing_later_endpoint(death_policy, consequences.viability)
            }
        }
    }
}

fn materialize_missing_later_endpoint(
    death_policy: DeathBeforeEndpointPolicy,
    viability: Option<ViabilityConsequence>,
) -> Result<MaterializedOutcome, SelectionAnalysisFrameError> {
    if viability != Some(ViabilityConsequence::DiedDuringWindow) {
        return Ok(MaterializedOutcome::Unavailable);
    }
    match death_policy {
        DeathBeforeEndpointPolicy::EndpointFailure => {
            Ok(MaterializedOutcome::EndpointFailureByObservedDeath)
        }
        DeathBeforeEndpointPolicy::Censored => Ok(MaterializedOutcome::CensoredByObservedDeath),
        DeathBeforeEndpointPolicy::CompetingRisk => Ok(MaterializedOutcome::CompetingRiskDeath),
        DeathBeforeEndpointPolicy::NotApplicable => {
            Err(SelectionAnalysisFrameError::InvalidLaterEndpointDeathPolicy)
        }
    }
}

fn update_analysis_authority_digest(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum SelectionAnalysisFrameError {
    Design(SelectionDesignError),
    Consequence(crate::ConsequenceError),
    UnsupportedFrameVersion(u32),
    UnsupportedRowVersion(u32),
    SelectionEvidenceBindingMismatch,
    ConsequenceLedgerBindingMismatch,
    AuthorityContextMismatch,
    PredictorChannelNotRequired,
    UnknownOrExcludedInput(EvolutionIndividualId),
    DuplicatePredictorInput(EvolutionIndividualId),
    IncompleteIncludedCoverage,
    MissingEvidenceRecord(EvolutionIndividualId),
    MissingConsequenceObservation(EvolutionIndividualId),
    ConsequenceObservationBindingMismatch(EvolutionIndividualId),
    PredictorDefinitionMismatch(EvolutionIndividualId),
    PredictorSourceBindingMismatch(EvolutionIndividualId),
    PredictorRepresentationMismatch,
    ZeroFixedPointScale,
    ObservedValueFromUnavailableSource(EvolutionIndividualId),
    CalibrationProvenanceMismatch(EvolutionIndividualId),
    UnexpectedCalibrationProvenance(EvolutionIndividualId),
    InvalidLaterEndpointDeathPolicy,
    DenominatorMismatch,
    NonCanonicalRowOrder,
    IncludedPopulationMismatch,
    FrameReplayMismatch,
}

impl fmt::Display for SelectionAnalysisFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "selection design authority failed: {error}"),
            Self::Consequence(error) => write!(f, "consequence authority failed: {error}"),
            Self::UnsupportedFrameVersion(version) => {
                write!(f, "unsupported selection analysis-frame version {version}")
            }
            Self::UnsupportedRowVersion(version) => {
                write!(f, "unsupported selection analysis-frame row version {version}")
            }
            Self::SelectionEvidenceBindingMismatch => {
                write!(f, "analysis frame selection-evidence authority mismatch")
            }
            Self::ConsequenceLedgerBindingMismatch => {
                write!(f, "analysis frame consequence-ledger authority mismatch")
            }
            Self::AuthorityContextMismatch => {
                write!(f, "analysis-frame population/census/context authority mismatch")
            }
            Self::PredictorChannelNotRequired => write!(
                f,
                "analysis design marks the predictor's phenotype/exposure channel as not required"
            ),
            Self::UnknownOrExcludedInput(id) => write!(
                f,
                "predictor materialization references non-included individual {}",
                id.as_str()
            ),
            Self::DuplicatePredictorInput(id) => write!(
                f,
                "predictor materialization supplied individual {} more than once",
                id.as_str()
            ),
            Self::IncompleteIncludedCoverage => {
                write!(f, "analysis frame does not materialize every included individual")
            }
            Self::MissingEvidenceRecord(id) => write!(
                f,
                "analysis-frame individual {} has no selection-evidence record",
                id.as_str()
            ),
            Self::MissingConsequenceObservation(id) => write!(
                f,
                "analysis-frame individual {} has no consequence observation",
                id.as_str()
            ),
            Self::ConsequenceObservationBindingMismatch(id) => write!(
                f,
                "analysis-frame consequence observation binding drifted for individual {}",
                id.as_str()
            ),
            Self::PredictorDefinitionMismatch(id) => write!(
                f,
                "analysis-frame predictor definition drifted for individual {}",
                id.as_str()
            ),
            Self::PredictorSourceBindingMismatch(id) => write!(
                f,
                "analysis-frame predictor source binding mismatched current evidence for individual {}",
                id.as_str()
            ),
            Self::PredictorRepresentationMismatch => {
                write!(f, "materialized predictor value does not match frame representation")
            }
            Self::ZeroFixedPointScale => {
                write!(f, "signed fixed-point predictor scale may not be zero")
            }
            Self::ObservedValueFromUnavailableSource(id) => write!(
                f,
                "analysis frame reports an observed predictor from unavailable source evidence for individual {}",
                id.as_str()
            ),
            Self::CalibrationProvenanceMismatch(id) => write!(
                f,
                "analysis-frame calibration provenance mismatched the frozen design for individual {}",
                id.as_str()
            ),
            Self::UnexpectedCalibrationProvenance(id) => write!(
                f,
                "analysis frame supplied calibration provenance where the frozen design does not permit it for individual {}",
                id.as_str()
            ),
            Self::InvalidLaterEndpointDeathPolicy => write!(
                f,
                "post-viability outcome materialization cannot use NotApplicable death policy"
            ),
            Self::DenominatorMismatch => {
                write!(f, "analysis-frame denominator does not reconcile with frozen design")
            }
            Self::NonCanonicalRowOrder => {
                write!(f, "analysis-frame row order is noncanonical")
            }
            Self::IncludedPopulationMismatch => write!(
                f,
                "analysis-frame rows do not exactly equal the frozen included population"
            ),
            Self::FrameReplayMismatch => write!(f, "selection analysis-frame replay mismatch"),
        }
    }
}

impl Error for SelectionAnalysisFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Design(error) => Some(error),
            Self::Consequence(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SelectionDesignError> for SelectionAnalysisFrameError {
    fn from(value: SelectionDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<crate::ConsequenceError> for SelectionAnalysisFrameError {
    fn from(value: crate::ConsequenceError) -> Self {
        Self::Consequence(value)
    }
}
